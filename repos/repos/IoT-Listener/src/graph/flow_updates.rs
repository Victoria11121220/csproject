use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Represents a flow update message received from Kafka
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FlowUpdateMessage {
    pub flow_id: i32,
    pub nodes: String,
    pub edges: String,
}

/// A thread-safe store for multiple flow graphs
pub type FlowGraphs = Arc<RwLock<HashMap<i32, (String, String)>>>;

/// Create a new flow graphs store
pub fn create_flow_graphs() -> FlowGraphs {
    Arc::new(RwLock::new(HashMap::new()))
}

/// Update or insert a flow graph in the store
pub async fn update_flow_graph(flow_graphs: &FlowGraphs, flow_id: i32, nodes: String, edges: String) {
    let mut graphs = flow_graphs.write().await;
    graphs.insert(flow_id, (nodes, edges));
}

/// Get a flow graph from the store
pub async fn get_flow_graph(flow_graphs: &FlowGraphs, flow_id: i32) -> Option<(String, String)> {
    let graphs = flow_graphs.read().await;
    graphs.get(&flow_id).cloned()
}