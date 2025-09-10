use petgraph::graph::NodeIndex;
use rdkafka::{
    config::ClientConfig,
    consumer::{Consumer, StreamConsumer},
    producer::{FutureProducer, FutureRecord},
    Message,
};
use rust_listener::{
    create_flow_graphs, entities::sea_orm_active_enums::IotFieldType, graph, nodes::NodeType,
    setup_database_connection, update_flow_graph, EnvFilter, FlowUpdateMessage,
};
use std::{collections::HashMap, sync::Arc, time::Duration};
use tokio::sync::{mpsc::UnboundedReceiver, Mutex};
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

    // 2. Setup database connection
    let db = setup_database_connection().await?;

    // Create a store for multiple flow graphs
    let flow_graphs = create_flow_graphs();

    let brokers = std::env::var("KAFKA_BROKERS").expect("KAFKA_BROKERS must be set");
    let trigger_topic = std::env::var("KAFKA_TOPIC").expect("KAFKA_TOPIC must be set");
    let flow_updates_topic =
        std::env::var("FLOW_UPDATES_TOPIC").unwrap_or_else(|_| "flow-updates".to_string());

    // 3. Create Kafka Producer for triggers
    let trigger_producer: FutureProducer = ClientConfig::new()
        .set("bootstrap.servers", &brokers)
        .set("message.timeout.ms", "5000")
        .create()?;

    // 4. Create Kafka Consumer for flow updates
    let flow_updates_consumer: StreamConsumer = ClientConfig::new()
        .set("bootstrap.servers", &brokers)
        .set("group.id", "collector-flow-updates")
        .set("auto.offset.reset", "latest")
        .create()?;

    flow_updates_consumer.subscribe(&[&flow_updates_topic])?;
    info!("Subscribed to flow updates topic '{}'", flow_updates_topic);

    // 5. Shared state for current graph, data receiver, and cancellation token
    let current_state = Arc::new(Mutex::new((
        None::<(i32, graph::RwLockGraph)>, // (flow_id, graph)
        None::<UnboundedReceiver<Vec<NodeIndex>>>, // data receiver
        None::<CancellationToken>, // cancellation token for current subscriptions
    )));
    
    let cancellation_token = CancellationToken::new();
    let db_arc = Arc::new(db);

    info!("Waiting for flow updates to subscribe to sources...");

    // 6. Spawn a task to listen for flow updates
    let flow_graphs_clone = flow_graphs.clone();
    let db_arc_clone = db_arc.clone();
    let cancellation_token_clone = cancellation_token.clone();
    let current_state_clone = current_state.clone();
    let _flow_updates_task = tokio::spawn(async move {
        loop {
            match flow_updates_consumer.recv().await {
                Ok(msg) => {
                    if let Some(payload) = msg.payload() {
                        match serde_json::from_slice::<graph::flow_updates::FlowUpdateMessage>(
                            payload,
                        ) {
                            Ok(update_msg) => {
                                let FlowUpdateMessage {
                                    flow_id,
                                    nodes,
                                    edges,
                                } = update_msg;

                                info!("Received flow update for flow_id: {flow_id}; node: {nodes}; edges: {edges}");
                                update_flow_graph(&flow_graphs_clone, flow_id, nodes.clone(), edges.clone()).await;
                                info!("Updated flow graph for flow_id: {}", update_msg.flow_id);

                                // Build the graph for this flow
                                match graph::build_graph::from_flow(&nodes, &edges) {
                                    Ok(graph) => {
                                        // Clone the graph for subscription (we need to avoid borrowing it)
                                        let graph_clone = {
                                            let graph_read = graph.graph.read().await;
                                            let _nodes_count = graph_read.node_count();
                                            drop(graph_read);
                                            // We don't actually need to clone the graph, just use it
                                            graph
                                        };
                                        
                                        // Subscribe to sources for this graph
                                                let graph_lock = graph_clone.graph.read().await;
                                        match rust_listener::subscribe_to_sources(
                                            &graph_lock,
                                            &cancellation_token_clone,
                                            db_arc_clone.clone(),
                                        ) {
                                            Ok((join_handles, receiver)) => {
                                                drop(graph_lock);
                                                // Update the current state with new graph, receiver, and cancellation token
                                                let mut state = current_state_clone.lock().await;
                                                // Cancel the previous subscriptions if they exist
                                                if let Some(old_cancellation_token) = &state.2 {
                                                    old_cancellation_token.cancel();
                                                }
                                                *state = (Some((flow_id, graph_clone)), Some(receiver), Some(cancellation_token_clone.clone()));
                                                info!("Subscribed to sources for flow_id: {}", flow_id);
                                            }
                                            Err(e) => {
                                                drop(graph_lock);
                                                error!("Failed to subscribe to sources for flow_id {}: {}", flow_id, e);
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        error!("Failed to build graph for flow_id {}: {}", flow_id, e);
                                    }
                                }
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

    // 7. Main loop to handle data triggers
    loop {
        // Clone what we need and release the lock immediately
        let (graph_info, has_receiver) = {
            let state = current_state.lock().await;
            (state.0.clone(), state.1.is_some())
        };
        
        // Check if we have a data receiver
        if has_receiver {
            // Get the receiver from the state
            let mut receiver = {
                let mut state = current_state.lock().await;
                state.1.take().unwrap()
            };

            // Try to receive data with a timeout
            match tokio::time::timeout(Duration::from_millis(100), receiver.recv()).await {
                Ok(Some(trigger_indices)) => {
                    info!(
                        "Received trigger for nodes: {:?}. Forwarding to Kafka.",
                        trigger_indices
                    );

                    // Process the trigger if we have a graph
                    if let Some((flow_id, ref graph)) = graph_info {
                        // Collect source data for the triggered nodes
                        let graph_read = graph.graph.read().await;
                        let mut source_data_map = HashMap::new();

                        for node_index in &trigger_indices {
                            if let Some(node) = graph_read.node_weight(*node_index) {
                                match node.concrete_node() {
                                    // Check if this is a source node and collect its data
                                    NodeType::Source(source_node) => match source_node.get_source_data() {
                                        Ok(data) => match serde_json::to_value(data) {
                                            Ok(json_data) => {
                                                source_data_map.insert(node.id().to_string(), json_data);
                                            }
                                            Err(e) => {
                                                error!(
                                                    "Failed to serialize source data for node {}: {:?}",
                                                    node.id(),
                                                    e
                                                );
                                            }
                                        },
                                        Err(e) => {
                                            error!("Failed to get source data for node {}: {:?}", node.id(), e);
                                        }
                                    },
                                    // Check if this is a datalake node and collect its data
                                    NodeType::Datalake(datalake_node) => {
                                        let reading_lock = datalake_node.get_reading();
                                        match reading_lock.read() {
                                            Ok(guard) => {
                                                if let Some(reading_store) = guard.as_ref() {
                                                    let (sensor, readings) = reading_store.clone();
                                                    let node_data = crate::graph::types::NodeData::Collection(
                                                        readings
                                                            .into_iter()
                                                            .map(|reading| match sensor.value_type {
                                                                IotFieldType::String => {
                                                                    serde_json::json!(reading.raw_value.clone())
                                                                }
                                                                IotFieldType::Number => {
                                                                    serde_json::json!(reading.value)
                                                                }
                                                                IotFieldType::Boolean => {
                                                                    serde_json::json!(
                                                                        reading.raw_value == "true"
                                                                    )
                                                                }
                                                            })
                                                            .collect(),
                                                    );
                                                    match serde_json::to_value(node_data) {
                                                        Ok(json_data) => {
                                                            source_data_map
                                                                .insert(node.id().to_string(), json_data);
                                                        }
                                                        Err(e) => {
                                                            error!("Failed to serialize datalake data for node {}: {:?}", node.id(), e);
                                                        }
                                                    }
                                                }
                                            }
                                            Err(e) => {
                                                error!(
                                                    "Failed to read datalake data for node {}: {:?}",
                                                    node.id(),
                                                    e
                                                );
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
                            flow_id: flow_id,
                            indices: trigger_indices,
                            source_data: source_data_map,
                        };

                        let payload = serde_json::to_string(&trigger_data)?;
                        let record = FutureRecord::to(&trigger_topic)
                            .payload(&payload)
                            .key("trigger");

                        if let Err(e) = trigger_producer.send(record, Duration::from_secs(0)).await {
                            error!("Failed to send trigger to Kafka: {:?}", e);
                        }
                        
                        // Put the receiver back
                        let mut state = current_state.lock().await;
                        state.1 = Some(receiver);
                    } else {
                        error!("No graph available when trying to send trigger");
                        // Put the receiver back
                        let mut state = current_state.lock().await;
                        state.1 = Some(receiver);
                    }
                }
                Ok(None) => {
                    // Channel closed, don't put the receiver back
                    error!("Data receiver channel closed");
                    break;
                }
                Err(_) => {
                    // Timeout, put the receiver back
                    let mut state = current_state.lock().await;
                    state.1 = Some(receiver);
                    // Continue to next iteration
                }
            }
        } else {
            // No data receiver yet, sleep briefly
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }

    Ok(())
}