use rust_listener::{
    graph,
    readings::StoreReading,
    setup_database_connection,
    EnvFilter,
};
use rdkafka::{
    consumer::{Consumer, StreamConsumer},
    Message,
};
use tracing::{error, info};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 1. Initialize logging and environment
    let filter = EnvFilter::from_default_env()
        .add_directive("sqlx::query=off".parse().unwrap())
        .add_directive("info".parse().unwrap());
    tracing_subscriber::fmt().with_env_filter(filter).init();
    dotenvy::dotenv().ok();
    info!("Starting iot-processor...");

    // 2. Connect to the database
    let db = setup_database_connection().await?;
    info!("Database connection successful.");

    // 3. Build the graph
    let (flow_id, mut graph) = match graph::build_graph::read_graph() {
        Ok(res) => res,
        Err(e) => {
            error!("Failed to build graph: {}", e);
            return Err(anyhow::anyhow!("Failed to build graph"));
        }
    };
    info!("Processing graph built successfully for flow_id: {}.", flow_id);

    let brokers = std::env::var("KAFKA_BROKERS").expect("KAFKA_BROKERS must be set");
    let group_id = std::env::var("KAFKA_GROUP_ID").expect("KAFKA_GROUP_ID must be set");

    // 4. Create Kafka Consumer
    let consumer: StreamConsumer = rdkafka::config::ClientConfig::new()
        .set("bootstrap.servers", &brokers)
        .set("group.id", &group_id)
        .set("auto.offset.reset", "earliest")
        .create()?;

    let topic = std::env::var("KAFKA_TOPIC").expect("KAFKA_TOPIC must be set");
    consumer.subscribe(&[&topic])?;
    info!("Subscribed to Kafka topic '{}'.", topic);

    // 5. Main processing loop
    info!("Waiting for triggers from Kafka...");
    loop {
        match consumer.recv().await {
            Ok(msg) => {
                if let Some(payload) = msg.payload() {
                    match serde_json::from_slice::<graph::trigger_data::TriggerData>(payload) {
                        Ok(trigger_data) => {
                            info!("Received trigger data for nodes: {:?}", trigger_data.indices);
                            
                            let (processed_readings, _node_errors, _) =
                                graph.backpropagate_with_data(trigger_data.indices, trigger_data.source_data).await;
                            info!("Readings: {:?}", processed_readings);
                            for r in processed_readings {
                                if let Err(e) = r.store(&db, flow_id).await {
                                    error!("Failed to store reading: {}", e);
                                }
                            }
                        }
                        Err(e) => {
                            error!("Failed to deserialize trigger payload: {:?}", e);
                        }
                    }
                }
            }
            Err(e) => {
                error!("Error receiving message from Kafka: {:?}", e);
            }
        }
    }
}