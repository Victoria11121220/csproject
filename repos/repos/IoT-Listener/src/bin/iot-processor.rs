use anyhow::{anyhow, Result};
use petgraph::graph::NodeIndex;
use rdkafka::{
    config::ClientConfig,
    consumer::{BaseConsumer, Consumer},
    message::Message,
};
use rust_listener::{
    get_database_connection,
    graph::{build_graph::read_graph, types::GraphPayload},
    readings::StoreReading,
    utils::error::upload_errors,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::{
    signal,
    sync::mpsc::{unbounded_channel, UnboundedReceiver},
};
use tokio_util::sync::CancellationToken;
use tracing::{error, info};
use tracing_subscriber::EnvFilter;

#[derive(Serialize, Deserialize, Debug)]
struct KafkaTrigger {
    index: usize,
    payload: GraphPayload,
}

fn setup_kafka_consumer(
    cancellation_token: &CancellationToken,
) -> Result<(
    tokio::task::JoinHandle<()>,
    UnboundedReceiver<Vec<(NodeIndex, GraphPayload)>>,
)> {
    let bootstrap_servers =
        std::env::var("KAFKA_BOOTSTRAP_SERVERS").unwrap_or_else(|_| "localhost:9092".to_string());
    let topic =
        std::env::var("KAFKA_PROCESSOR_TOPIC").unwrap_or_else(|_| "iot-triggers".to_string());
    let group_id = std::env::var("KAFKA_PROCESSOR_GROUP_ID")
        .unwrap_or_else(|_| "iot-processor-group".to_string());

    info!(
        "Setting up Kafka consumer for topic: {} on {}",
        topic, bootstrap_servers
    );

    let consumer: BaseConsumer = ClientConfig::new()
        .set("bootstrap.servers", &bootstrap_servers)
        .set("group.id", &group_id)
        .set("enable.auto.commit", "true")
        .set("auto.offset.reset", "latest")
        .create()
        .expect("Failed to create Kafka consumer");

    consumer.subscribe(&[&topic])?;
    info!("Subscribed to Kafka topic: {}", topic);

    let (tx, rx) = unbounded_channel::<Vec<(NodeIndex, GraphPayload)>>();
    let cancellation_token = cancellation_token.clone();

    let handle = tokio::spawn(async move {
        loop {
            tokio::select! {
                     _ = cancellation_token.cancelled() => {
                         info!("Kafka consumer cancelled for topic: {}", topic);
                         break;
                     }
                     _ = tokio::time::sleep(tokio::time::Duration::from_millis(100)) => {
                         if let Some(Ok(message)) = consumer.poll(tokio::time::Duration::from_millis(100)) {
                             if let Some(Ok(payload)) = message.payload_view::<str>() {
                                 let value: Result<Vec<KafkaTrigger>, _> = serde_json::from_str(payload);
                                 match value {
                                     Ok(kafka_triggers) => {
                                         let triggers: Vec<(NodeIndex, GraphPayload)> = kafka_triggers
                                             .into_iter()
                                             .map(|kt| (NodeIndex::new(kt.index), kt.payload))
                                             .collect();
                                         if let Err(e) = tx.send(triggers) {
                                             error!("Failed to send message to transmitter: {:?}", e);
                                         }
                                     }
                                     Err(e) => {
                                         error!("Failed to parse Kafka message as JSON array of KafkaTrigger: {}",
            e);
                                     }
                                 }
                             }
                         }
                     }
                 }
        }
        info!("Kafka consumer stopped for topic: {}", topic);
    });

    Ok((handle, rx))
}

#[tokio::main]
async fn main() -> Result<()> {
    let filter = EnvFilter::from_default_env()
        .add_directive("sqlx::query=off".parse().unwrap())
        .add_directive("info".parse().unwrap());
    tracing_subscriber::fmt().with_env_filter(filter).init();
    dotenv::dotenv().ok();

    let (flow_id, mut graph) = read_graph().map_err(|e| anyhow!("failed to read graph: {e}"))?;
    let db = Arc::new(get_database_connection().await?);
    let global_cancellation_token = CancellationToken::new();

    let (kafka_join_handle, mut kafka_receiver) = setup_kafka_consumer(&global_cancellation_token)?;

    let receiver_cancel = CancellationToken::clone(&global_cancellation_token);
    let store_db = Arc::clone(&db);

    let processor_handle = tokio::spawn(async move {
        loop {
            tokio::select! {
                _ = receiver_cancel.cancelled() => {
                    info!("Cancellation token triggered, shutting down");
                    break;
                },
                Some(triggers) = kafka_receiver.recv() => {
                    info!("Received {} triggers from Kafka", triggers.len());
                    let (readings, _, node_errors) = graph.backpropagate_with_data(triggers).await;

                    for reading in readings {
                        if let Err(e) = reading.store(&store_db, flow_id).await {
                            error!("Failed to store reading: {}; {:?}", e, reading);
                        }
                    }
                    if let Err(e) = upload_errors(&node_errors, &flow_id, &graph, &store_db).await {
                        error!("Failed to upload errors: {}", e);
                    }
                }
            }
        }
    });

    let handles = vec![kafka_join_handle, processor_handle];
    signal::ctrl_c().await?;
    global_cancellation_token.cancel();
    for handle in handles {
        handle.await?;
    }
    // db.close().await?;
    info!("Database connection closed");

    Ok(())
}