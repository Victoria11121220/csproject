use anyhow::{anyhow, Result};
use rust_listener::{
    get_database_connection,
    graph::build_graph::read_graph,
    readings::StoreReading,
    utils::error::upload_errors,
};
use std::sync::Arc;
use tokio::signal;
use tokio_util::sync::CancellationToken;
use tracing::{error, info};
use tracing_subscriber::EnvFilter;
use rdkafka::{
    config::ClientConfig,
    consumer::{BaseConsumer, Consumer},
    message::Message,
};
use petgraph::graph::NodeIndex;
use tokio::sync::mpsc::UnboundedReceiver;

#[tokio::main]
async fn main() -> Result<()> {
    // Set up the logger without logging sqlx::query calls
    let filter = EnvFilter::from_default_env()
        .add_directive("sqlx::query=off".parse().unwrap())
        .add_directive("info".parse().unwrap()); // Ensure info level logs are enabled

    tracing_subscriber::fmt().with_env_filter(filter).init();

    // Load the .env file
    dotenv::dotenv().ok();

    // Build the graph from the env file
    let (flow_id, mut graph) = match read_graph() {
        Ok(res) => res,
        Err(e) => {
            error!("Failed to read flow: {}", e);
            return Err(anyhow!("Failed to read flow"));
        }
    };
    info!("Loaded flow {:?}", graph);

    // Connect to database
    let conn = match get_database_connection().await {
        Ok(conn) => conn,
        Err(e) => {
            error!("Failed to connect to database: {}", e);
            return Err(anyhow!("Failed to connect to database"));
        }
    };
    info!("Connected to database");
    // Wrap the connection in an Arc to share it between threads
    let db = Arc::new(conn);

    let global_cancellation_token = CancellationToken::new();
    
    // Set up Kafka consumer to receive triggers from iot-collector
    let (_join_handle, mut sources_incoming_data_pipe_receiver) = setup_kafka_consumer(&global_cancellation_token)?;

    let receiver_cancel = CancellationToken::clone(&global_cancellation_token);
    let store_db = Arc::clone(&db);
    
    let processor_handle = tokio::spawn(async move {
        loop {
            tokio::select! {
                _ = receiver_cancel.cancelled() => {
                    info!("Cancellation token triggered, shutting down");
                    break;
                },
                Some(trigger) = sources_incoming_data_pipe_receiver.recv() => {
                    info!("Received trigger: {:?}", trigger);

                    let (readings, _, node_errors) = graph.backpropagate(trigger).await;
                    // info!("readings: {:?}", readings);

                    info!("--- DEBUG START ---");
                    info!("Readings: {:#?}", readings);
                    info!("Node Errors: {:#?}", node_errors);
                    info!("--- DEBUG END ---");

                    let mut reading_errors = Vec::new();
                    for reading in readings {
                        reading.store(&store_db, flow_id).await.unwrap_or_else(|e| {
                            error!("Failed to store reading: {}; {:?}", e, reading);
                            reading_errors.push(e);
                        });
                    }
                    let res = upload_errors(&node_errors, &flow_id, &graph, &store_db).await;
                    if let Err(e) = res {
                        error!("Failed to upload errors: {}", e);
                    }
                }
            }
        }
    });

    match signal::ctrl_c().await {
        Ok(_) => info!("Received Ctrl-C, shutting down"),
        Err(e) => {
            error!("Error receiving Ctrl-C: {:?}", e);
            return Err(anyhow!("Error receiving Ctrl-C: {:?}", e));
        }
    }

    global_cancellation_token.cancel();

    // Wait for the processor to finish
    processor_handle.await?;

    Arc::try_unwrap(db)
        .expect("Failed to unwrap Arc")
        .close()
        .await?;
    info!("Database connection closed");

    Ok(())
}

// Set up Kafka consumer to receive triggers from iot-collector
fn setup_kafka_consumer(
    cancellation_token: &CancellationToken,
) -> Result<(tokio::task::JoinHandle<()>, UnboundedReceiver<Vec<NodeIndex>>)> {
    // Get Kafka configuration from environment variables
    let bootstrap_servers = std::env::var("KAFKA_BOOTSTRAP_SERVERS")
        .unwrap_or_else(|_| "localhost:9092".to_string());
    let topic = std::env::var("KAFKA_PROCESSOR_TOPIC")
        .unwrap_or_else(|_| "iot-triggers".to_string());
    let group_id = std::env::var("KAFKA_PROCESSOR_GROUP_ID")
        .unwrap_or_else(|_| "iot-processor-group".to_string());
    
    info!("Setting up Kafka consumer for topic: {} on {}", topic, bootstrap_servers);

    // Create Kafka consumer
    let consumer: BaseConsumer = ClientConfig::new()
        .set("bootstrap.servers", &bootstrap_servers)
        .set("group.id", &group_id)
        .set("enable.auto.commit", "true")
        .set("auto.offset.reset", "latest")
        .create()
        .expect("Failed to create Kafka consumer");

    // Subscribe to topic
    if let Err(e) = consumer.subscribe(&[&topic]) {
        error!("Failed to subscribe to Kafka topic {}: {}", topic, e);
        return Err(anyhow!("Failed to subscribe to Kafka topic {}: {}", topic, e));
    }

    info!("Subscribed to Kafka topic: {}", topic);

    // Create channel for sending triggers
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel::<Vec<NodeIndex>>();
    
    // Clone values for the async task
    let cancellation_token = cancellation_token.clone();
    
    // Spawn task to consume messages from Kafka
    let handle = tokio::spawn(async move {
        loop {
            tokio::select! {
                _ = cancellation_token.cancelled() => {
                    info!("Kafka consumer cancelled for topic: {}", topic);
                    break;
                }
                _ = tokio::time::sleep(tokio::time::Duration::from_millis(100)) => {
                    // Poll for messages
                    match consumer.poll(tokio::time::Duration::from_millis(100)) {
                        Some(Ok(message)) => {
                            // Process the message
                            match message.payload_view::<str>() {
                                Some(Ok(payload)) => {
                                    info!("Received Kafka message from topic {}: {}", topic, payload);
                                    
                                    // Try to parse as JSON array of node indices
                                    let value: Result<Vec<usize>, _> = serde_json::from_str(payload);
                                    match value {
                                        Ok(node_indices) => {
                                            // Convert to NodeIndex objects
                                            let node_indices: Vec<NodeIndex> = node_indices
                                                .into_iter()
                                                .map(NodeIndex::new)
                                                .collect();
                                            
                                            // Send the trigger to the processor
                                            if let Err(e) = tx.send(node_indices) {
                                                error!("Failed to send message to transmitter: {:?}", e);
                                            }
                                        }
                                        Err(e) => {
                                            error!("Failed to parse Kafka message as JSON array of node indices: {}", e);
                                        }
                                    }
                                }
                                Some(Err(e)) => {
                                    error!("Failed to decode Kafka message payload: {}", e);
                                }
                                None => {
                                    error!("Kafka message payload is empty");
                                }
                            }
                        }
                        Some(Err(e)) => {
                            error!("Kafka error: {}", e);
                        }
                        None => {
                            // No message available, continue polling
                            continue;
                        }
                    }
                }
            }
        }

        info!("Kafka consumer stopped for topic: {}", topic);
    });

    Ok((handle, rx))
}