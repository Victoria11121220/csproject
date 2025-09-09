use petgraph::graph::NodeIndex;
use rust_listener::{graph, setup_database_connection, EnvFilter, nodes::NodeType, entities::sea_orm_active_enums::IotFieldType};
use rdkafka::producer::{FutureProducer, FutureRecord};
use std::{sync::Arc, time::Duration, collections::HashMap};
use tokio::sync::mpsc::UnboundedReceiver;
use tokio_util::sync::CancellationToken;
use tracing::{error, info};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 1. Initialize logging and environment
    let filter = EnvFilter::from_default_env()
        .add_directive("sqlx::query=off".parse().unwrap())
        .add_directive("info".parse().unwrap());
    tracing_subscriber::fmt().with_env_filter(filter).init();
    dotenvy::dotenv().ok();
    info!("Starting iot-collector...");

    // 2. Setup database and graph
    let db = setup_database_connection().await?;

    let (_flow_id, graph) = match graph::build_graph::read_graph() {
        Ok(res) => res,
        Err(e) => {
            error!("Failed to build graph: {}", e);
            return Err(anyhow::anyhow!("Failed to build graph"));
        }
    };
    info!("Graph built successfully.");

    let brokers = std::env::var("KAFKA_BROKERS").expect("KAFKA_BROKERS must be set");
    let topic = std::env::var("KAFKA_TOPIC").expect("KAFKA_TOPIC must be set");

    // 3. Create Kafka Producer
    let producer: FutureProducer = rdkafka::config::ClientConfig::new()
        .set("bootstrap.servers", &brokers)
        .set("message.timeout.ms", "5000")
        .create()?;

    // 4. Subscribe to sources
    let cancellation_token = CancellationToken::new();
    let db_arc = Arc::new(db);
    let graph_lock = graph.read().await;

    let (_join_handles, mut data_receiver): (_, UnboundedReceiver<Vec<NodeIndex>>) = 
        match rust_listener::subscribe_to_sources(&graph_lock, &cancellation_token, db_arc) {
            Ok(res) => res,
            Err(e) => {
                error!("Failed to subscribe to sources: {}", e);
                return Err(anyhow::anyhow!("Failed to subscribe to sources"));
            }
        };
    
    // Drop the lock after subscribing
    drop(graph_lock);

    info!("Subscribed to sources. Waiting for data triggers...");

    // 5. Loop to receive from sources and send to Kafka
    loop {
        if let Some(trigger_indices) = data_receiver.recv().await {
            info!("Received trigger for nodes: {:?}. Forwarding to Kafka.", trigger_indices);

            // Collect source data for the triggered nodes
            let graph_read = graph.read().await;
            let mut source_data_map = HashMap::new();
            
            for node_index in &trigger_indices {
                if let Some(node) = graph_read.node_weight(*node_index) {
                    match node.concrete_node() {
                        // Check if this is a source node and collect its data
                        NodeType::Source(source_node) => {
                            match source_node.get_source_data() {
                                Ok(data) => {
                                    match serde_json::to_value(data) {
                                        Ok(json_data) => {
                                            source_data_map.insert(node.id().to_string(), json_data);
                                        }
                                        Err(e) => {
                                            error!("Failed to serialize source data for node {}: {:?}", node.id(), e);
                                        }
                                    }
                                }
                                Err(e) => {
                                    error!("Failed to get source data for node {}: {:?}", node.id(), e);
                                }
                            }
                        }
                        // Check if this is a datalake node and collect its data
                        NodeType::Datalake(datalake_node) => {
                            let reading_lock = datalake_node.get_reading();
                            match reading_lock.read() {
                                Ok(guard) => {
                                    if let Some(reading_store) = guard.as_ref() {
                                        let (sensor, readings) = reading_store.clone();
                                        let node_data = crate::graph::types::NodeData::Collection(
                                            readings.into_iter().map(|reading| {
                                                match sensor.value_type {
                                                    IotFieldType::String => {
                                                        serde_json::json!(reading.raw_value.clone())
                                                    }
                                                    IotFieldType::Number => {
                                                        serde_json::json!(reading.value)
                                                    }
                                                    IotFieldType::Boolean => {
                                                        serde_json::json!(reading.raw_value == "true")
                                                    }
                                                }
                                            }).collect()
                                        );
                                        match serde_json::to_value(node_data) {
                                            Ok(json_data) => {
                                                source_data_map.insert(node.id().to_string(), json_data);
                                            }
                                            Err(e) => {
                                                error!("Failed to serialize datalake data for node {}: {:?}", node.id(), e);
                                            }
                                        }
                                    }
                                }
                                Err(e) => {
                                    error!("Failed to read datalake data for node {}: {:?}", node.id(), e);
                                }
                            };
                        }
                        _ => {}
                    }
                }
            }
            drop(graph_read); // Release the read lock

            // Create TriggerData with both indices and source data
            let trigger_data = graph::trigger_data::TriggerData {
                indices: trigger_indices,
                source_data: source_data_map,
            };

            let payload = serde_json::to_string(&trigger_data)?;
            let record = FutureRecord::to(&topic).payload(&payload).key("trigger");

            if let Err(e) = producer.send(record, Duration::from_secs(0)).await {
                error!("Failed to send trigger to Kafka: {:?}", e);
            }
        }
    }
}