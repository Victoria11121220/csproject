use rdkafka::{
    config::ClientConfig,
    consumer::{Consumer, StreamConsumer},
    Message,
};
use rust_listener::{
    create_flow_graphs, get_flow_graph, graph, readings::StoreReading, setup_database_connection,
    update_flow_graph, EnvFilter,
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

    // Create a store for multiple flow graphs
    let flow_graphs = create_flow_graphs();

    let brokers = std::env::var("KAFKA_BROKERS").expect("KAFKA_BROKERS must be set");
    let group_id = std::env::var("KAFKA_GROUP_ID").expect("KAFKA_GROUP_ID must be set");
    let trigger_topic = std::env::var("KAFKA_TOPIC").expect("KAFKA_TOPIC must be set");
    let flow_updates_topic =
        std::env::var("FLOW_UPDATES_TOPIC").unwrap_or_else(|_| "flow-updates".to_string());

    // 4. Create Kafka Consumer for triggers
    let trigger_consumer: StreamConsumer = ClientConfig::new()
        .set("bootstrap.servers", &brokers)
        .set("group.id", &group_id)
        .set("auto.offset.reset", "earliest")
        .create()?;

    trigger_consumer.subscribe(&[&trigger_topic])?;
    info!("Subscribed to Kafka trigger topic '{}'.", trigger_topic);

    // 5. Create Kafka Consumer for flow updates
    let flow_updates_consumer: StreamConsumer = ClientConfig::new()
        .set("bootstrap.servers", &brokers)
        .set("group.id", "processor-flow-updates")
        .set("auto.offset.reset", "latest")
        .create()?;

    flow_updates_consumer.subscribe(&[&flow_updates_topic])?;
    info!("Subscribed to flow updates topic '{}'", flow_updates_topic);

    // 6. Spawn a task to listen for flow updates
    let flow_graphs_clone = flow_graphs.clone();
    let _flow_updates_task = tokio::spawn(async move {
        loop {
            match flow_updates_consumer.recv().await {
                Ok(msg) => {
                    if let Some(payload) = msg.payload() {
                        match serde_json::from_slice::<graph::flow_updates::FlowUpdateMessage>(
                            payload,
                        ) {
                            Ok(update_msg) => {
                                info!("Received flow update for flow_id: {}", update_msg.flow_id);
                                update_flow_graph(
                                    &flow_graphs_clone,
                                    update_msg.flow_id,
                                    update_msg.nodes,
                                    update_msg.edges,
                                )
                                .await;
                                info!("Updated flow graph for flow_id: {}", update_msg.flow_id);
                            }
                            Err(e) => {
                                error!("Failed to deserialize flow update payload: {:?}", e);
                            }
                        }
                    }
                }
                Err(e) => {
                    error!("Error receiving flow update message: {:?}", e);
                }
            }
        }
    });

    // 7. Main processing loop for triggers
    info!("Waiting for triggers and flow updates from Kafka...");
    loop {
        // Use a select! macro to handle both trigger messages and flow updates
        tokio::select! {
            // Handle trigger messages
            result = trigger_consumer.recv() => {
                match result {
                    Ok(msg) => {
                        if let Some(payload) = msg.payload() {
                            match serde_json::from_slice::<graph::trigger_data::TriggerData>(payload) {
                                Ok(trigger_data) => {
                                    info!("Received trigger data for nodes: {:?} with flow_id: {}", trigger_data.indices, trigger_data.flow_id);

                                    // Get the appropriate graph for this flow_id
                                    let flow_graph = get_flow_graph(&flow_graphs, trigger_data.flow_id).await;

                                    if let Some((nodes_str, edges_str)) = flow_graph {
                                        // Clone the strings to move them into the closure
                                        let nodes_str = nodes_str.clone();
                                        let edges_str = edges_str.clone();

                                        // Build the graph for this flow
                                        match graph::build_graph::from_flow(&nodes_str, &edges_str) {
                                            Ok(mut flow_graph) => {
                                                let (processed_readings, _node_errors, _) =
                                                    flow_graph.backpropagate_with_data(trigger_data.indices, trigger_data.source_data).await;
                                                info!("Readings: {:?}", processed_readings);
                                                for r in processed_readings {
                                                    if let Err(e) = r.store(&db, trigger_data.flow_id).await {
                                                        error!("Failed to store reading: {}", e);
                                                    }
                                                }
                                            }
                                            Err(e) => {
                                                error!("Failed to build graph for flow_id {}: {:?}", trigger_data.flow_id, e);
                                            }
                                        }
                                    } else {
                                        error!("No graph found for flow_id: {}", trigger_data.flow_id);
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
    }

    // Wait for the flow updates task (this will never actually be reached in the current implementation)
    // let _ = flow_updates_task.await;

    Ok(())
}