// iot-collector reads data from mqtt/http and sends triggers to Kafka
use anyhow::{anyhow, Result};
use rust_listener::{
    get_database_connection,
    graph::build_graph::read_graph,
    utils::subscribe::subscribe_to_sources,
};
use tokio_util::sync::CancellationToken;
use tracing::{error, info};
use tracing_subscriber::EnvFilter;
use std::sync::Arc;
use rdkafka::{
    config::ClientConfig,
    producer::{BaseProducer, BaseRecord},
};

#[tokio::main]
async fn main() -> Result<()> {
    let filter = EnvFilter::from_default_env()
        .add_directive("sqlx::query=off".parse().unwrap())
        .add_directive("info".parse().unwrap());

    tracing_subscriber::fmt().with_env_filter(filter).init();

    dotenv::dotenv().ok();

    let (flow_id, graph) = match read_graph() {
        Ok(res) => res,
        Err(e) => {
            error!("Failed to read flow: {}", e);
            return Err(anyhow!("Failed to read flow"));
        }
    };

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
        subscribe_to_sources(
            &*graph.read().await,
            &global_cancellation_token,
            db.clone(),
        ).map_err(|e| anyhow!("Failed to subscribe to sources: {}", e))?;

    // Set up Kafka producer to send triggers to iot-processor
    let producer = setup_kafka_producer()?;

    // Spawn task to handle incoming data and send triggers to Kafka
    let kafka_handle = tokio::spawn(async move {
        while let Some(trigger) = sources_incoming_data_pipe_receiver.recv().await {
            info!("Received trigger: {:?}", trigger);
            
            // Convert NodeIndex objects to usize for serialization
            let node_indices: Vec<usize> = trigger.into_iter().map(|n| n.index()).collect();
            
            // Serialize the trigger
            let payload = match serde_json::to_string(&node_indices) {
                Ok(p) => p,
                Err(e) => {
                    error!("Failed to serialize trigger: {}", e);
                    continue;
                }
            };
            
            // Send the trigger to Kafka
            let topic = std::env::var("KAFKA_PROCESSOR_TOPIC")
                .unwrap_or_else(|_| "iot-triggers".to_string());
            
            if let Err(e) = producer.send(
                BaseRecord::to(&topic)
                    .key("")
                    .payload(&payload)
            ) {
                error!("Failed to send message to Kafka: {:?}", e);
            }
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
fn setup_kafka_producer() -> Result<BaseProducer> {
    // Get Kafka configuration from environment variables
    let bootstrap_servers = std::env::var("KAFKA_BOOTSTRAP_SERVERS")
        .unwrap_or_else(|_| "localhost:9092".to_string());
    
    info!("Setting up Kafka producer for servers: {}", bootstrap_servers);

    // Create Kafka producer
    let producer: BaseProducer = ClientConfig::new()
        .set("bootstrap.servers", &bootstrap_servers)
        .create()
        .expect("Failed to create Kafka producer");

    Ok(producer)
}