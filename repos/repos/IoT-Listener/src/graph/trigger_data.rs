use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use petgraph::graph::NodeIndex;

/// Data structure sent through Kafka containing both trigger indices and source data
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TriggerData {
    /// Flow ID to identify which flow this trigger belongs to
    pub flow_id: i32,
    /// Node indices that triggered the computation
    pub indices: Vec<NodeIndex>,
    /// Source data mapped by node ID
    pub source_data: HashMap<String, serde_json::Value>,
}