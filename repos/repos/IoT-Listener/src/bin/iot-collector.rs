// iot-collector reads data from mqtt/http and sends triggers to Kafka
use anyhow::{anyhow, Result};
use rdkafka::{
    admin::{AdminClient, AdminOptions, NewTopic, TopicReplication}, client::DefaultClientContext, config::ClientConfig, producer::{BaseProducer, BaseRecord}
};
use rust_listener::{
    get_database_connection, graph::build_graph::read_graph,
    utils::subscribe::subscribe_to_sources, KafkaTrigger,
};
use std::{sync::Arc, time::Duration};
use tokio_util::sync::CancellationToken;
use tracing::{error, info, warn};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<()> {
    let filter = EnvFilter::from_default_env()
        .add_directive("sqlx::query=off".parse().unwrap())
        .add_directive("info".parse().unwrap());

    tracing_subscriber::fmt().with_env_filter(filter).init();

    dotenv::dotenv().ok();

    let (flow_id, mut graph) = match read_graph() {
        Ok(res) => res,
        Err(e) => {
            error!("Failed to read flow: {}", e);
            return Err(anyhow!("Failed to read flow"));
        }
    };
    tracing::info!("flow id: {flow_id}");
    // Connect to database
    let conn = match get_database_connection().await {
        Ok(conn) => conn,
        Err(e) => {
            error!("Failed to connect to database: {}", e);
            return Err(anyhow!("Failed to connect to database"));
        }
    };
    info!("Connected to database");
    let db = Arc::new(conn);

    let global_cancellation_token = CancellationToken::new();

    let (mut source_monitors_join_handles, mut sources_incoming_data_pipe_receiver) =
        subscribe_to_sources(&*graph.read().await, &global_cancellation_token, db.clone())
            .map_err(|e| anyhow!("Failed to subscribe to sources: {}", e))?;

    // Set up Kafka producer to send triggers to iot-processor
    let producer = setup_kafka_producer().await?;

    // Spawn task to handle incoming data and send triggers to Kafka
    let kafka_handle = tokio::spawn(async move {
        while let Some(trigger_indices) = sources_incoming_data_pipe_receiver.recv().await {
            info!("Received trigger: {:?}", trigger_indices);

            let graph_guard = graph.read().await;
            let mut kafka_triggers = Vec::new();
            for node_index in trigger_indices {
                if let Some(node_result) = graph_guard[node_index].get_data() {
                    match node_result {
                        Ok(payload) => {
                            let kafka_trigger = KafkaTrigger {
                                index: node_index.index(),
                                payload: payload.clone(),
                            };
                            info!("Trigger: {:?}", kafka_trigger);
                            kafka_triggers.push(kafka_trigger);
                        }
                        Err(e) => {
                            error!(
                                "Node {} has an error state, skipping: {:?}",
                                node_index.index(),
                                e
                            );
                        }
                    }
                } else {
                    warn!(
                        "Node {} triggered but has no data, skipping",
                        node_index.index()
                    );
                }
            }

            if !kafka_triggers.is_empty() {
                let payload_str = match serde_json::to_string(&kafka_triggers) {
                    Ok(p) => p,
                    Err(e) => {
                        error!("Failed to serialize KafkaTriggers: {}", e);
                        continue;
                    }
                };
                let topic = std::env::var("KAFKA_PROCESSOR_TOPIC")
                    .unwrap_or_else(|_| "iot-triggers".to_string());
                if let Err(e) = producer.send(BaseRecord::to(&topic).key("").payload(&payload_str))
                {
                    error!("Failed to send message to Kafka: {:?}", e);
                }
            }

            // Send the trigger to Kafka
        }
    });

    source_monitors_join_handles.push(kafka_handle);

    // Wait for all tasks to complete or cancellation
    let _ = tokio::signal::ctrl_c().await;
    global_cancellation_token.cancel();

    // Wait for all threads to finish
    for handle in source_monitors_join_handles {
        let _ = handle.await;
    }

    Arc::try_unwrap(db)
        .expect("Failed to unwrap Arc")
        .close()
        .await?;
    info!("Database connection closed");

    Ok(())
}

// Set up Kafka producer to send triggers to iot-processor
// async fn setup_kafka_producer() -> Result<BaseProducer> {
//     // Get Kafka configuration from environment variables
//     let bootstrap_servers =
//         std::env::var("KAFKA_BOOTSTRAP_SERVERS").unwrap_or_else(|_| "localhost:9092".to_string());

//     info!(
//         "Setting up Kafka producer for servers: {}",
//         bootstrap_servers
//     );

//     // Create Kafka producer
//     let producer: BaseProducer = ClientConfig::new()
//         .set("bootstrap.servers", &bootstrap_servers)
//         .create()
//         .expect("Failed to create Kafka producer");

//     Ok(producer)
// }

async fn setup_kafka_producer() -> Result<BaseProducer> {
    let bootstrap_servers =
        std::env::var("KAFKA_BOOTSTRAP_SERVERS").unwrap_or_else(|_| "localhost:9092".to_string());
    let topic_name =
        std::env::var("KAFKA_PROCESSOR_TOPIC").unwrap_or_else(|_| "iot-triggers".to_string());

    let client_config = ClientConfig::new()
        .set("bootstrap.servers", &bootstrap_servers)
        .clone();

    let admin_client: AdminClient<DefaultClientContext> = client_config
        .create()
        .map_err(|e| anyhow!("Failed to create Kafka admin client: {}", e))?;

    let num_partitions: i32 = std::env::var("KAFKA_TOPIC_PARTITIONS")
        .unwrap_or_else(|_| "1".to_string())
        .parse()?;
    let replication_factor: i32 = std::env::var("KAFKA_TOPIC_REPLICAS")
        .unwrap_or_else(|_| "1".to_string())
        .parse()?;

    let new_topic = NewTopic::new(
        &topic_name,
        num_partitions,
        TopicReplication::Fixed(replication_factor),
    );

    info!(
        "Attempting to create Kafka topic '{}' if it does not exist...",
        topic_name
    );
    let results = admin_client
        .create_topics(
            &[new_topic],
            &AdminOptions::new().operation_timeout(Some(Duration::from_secs(5))),
        )
        .await?;

    for result in results {
        match result {
            Ok(topic) => info!("Topic '{}' created successfully.", topic),
            Err((topic, error)) => match error {
                rdkafka::error::RDKafkaErrorCode::TopicAlreadyExists => {
                    info!("Topic '{}' already exists.", topic);
                }

                _ => {
                    let err_msg = format!("Failed to create topic '{}': {:?}", topic, error);
                    error!("{}", err_msg);
                    return Err(anyhow!(err_msg));
                }
            },
        }
    }
    let producer: BaseProducer = client_config
        .create()
        .map_err(|e| anyhow!("Failed to create Kafka producer: {}", e))?;
    info!("Kafka producer created for servers: {}", bootstrap_servers);

    Ok(producer)
}