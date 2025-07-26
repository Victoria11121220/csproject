mod constant;
mod debug;
mod index;
mod key;
mod legacy;
mod lorawan;
mod metadata;
mod sensor;
mod mas_monitor;
mod current_timestamp;
mod average;
pub mod datalake;
pub mod sources;

use crate::graph::node::{ NodeError, NodeResult };
use crate::graph::concrete_node::ConcreteNode;
use crate::graph::types::{ GraphPayloadCollections, GraphPayloadMixed, GraphPayloadObjects };
use crate::readings::Readings;
use enum_dispatch::enum_dispatch;
#[cfg(test)]
use mockall::mock;
use serde::{ self, Deserialize };
use std::collections::HashSet;

#[enum_dispatch(ConcreteNode)]
#[derive(Debug, Deserialize)]
#[serde(tag = "type", content = "value")] // Tagged enum for type and value
#[rustfmt::skip]
pub enum NodeType {
    #[serde(rename = "source")] Source(sources::SourceNode),
    #[serde(rename = "key")] Key(key::KeyNode),
    #[serde(rename = "sensor")] Sensor(sensor::SensorNode),
    #[serde(rename = "legacy")] Legacy(legacy::LegacyNode),
    #[serde(rename = "debug")] Debug(debug::DebugNode),
    #[serde(rename = "index")] Index(index::IndexNode),
    #[serde(rename = "constant")] Constant(constant::ConstantNode),
    #[serde(rename = "lorawan")] Lorawan(lorawan::LoRaWANNode),
    #[serde(rename = "metadata")] Metadata(metadata::MetadataNode),
    #[serde(rename = "mas_monitor")] MASMonitor(mas_monitor::MASMonitorNode),
    #[serde(rename = "current_timestamp")] CurrentTimestamp(current_timestamp::CurrentTimestampNode),
    #[serde(rename = "datalake")] Datalake(datalake::DatalakeNode),
    #[serde(rename = "average")] Average(average::AverageNode),
    #[serde(skip)]
    #[cfg(test)]
        Mock(MockInternalNode),
}

#[cfg(test)]
mock! {
    #[derive(Debug)]
    pub InternalNode { }
    
    impl ConcreteNode for InternalNode {
        fn default_output_handle(&self) -> String;
        fn set_actual_handles(&mut self, input_handles: HashSet<String>, output_handles: HashSet<String>) -> Result<(), NodeError>;
        fn generates_reading(&self) -> bool;
        async fn compute_objects(&self, inputs: &GraphPayloadObjects) -> NodeResult;
        fn payload_objects_to_reading(&self, node_id: &str, objects: &GraphPayloadObjects) -> Result<Readings, NodeError>;
        fn payload_collections_to_reading(&self, node_id: &str, collections: &GraphPayloadCollections) -> Result<Vec<Readings>, NodeError>;
        fn payload_mixed_to_reading(&self, node_id: &str, mixed: &GraphPayloadMixed) -> Result<Vec<Readings>, NodeError>;
        async fn compute_collections(&self, inputs: &GraphPayloadCollections) -> NodeResult;
        async fn compute_mixed(&self, inputs: &GraphPayloadMixed) -> NodeResult;
    }

    impl Clone for InternalNode {
        fn clone(&self) -> Self;
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod nodes_tests {
	use super::*;

	#[test]
	fn deserialise() {
		let valid_jsons = [
			r#"{"type": "index", "value": {"index": 0}}"#,
			r#"{"type": "key", "value": {"key": ["key1"]}}"#,
			r#"{"type": "constant", "value": {"type": "STRING", "constant": "value"}}"#,
			r#"{"type": "current_timestamp"}"#,
		];
		for json in valid_jsons.iter() {
			let node = serde_json::from_str::<NodeType>(json);
			assert!(node.is_ok());
		}
	}

	#[test]
	fn deserialise_invalid() {
		let invalid_jsons = [
			r#"{"type": "nonexistent", "value": {"key": ["key1"]}}"#,
			r#"[1,2,3]"#,
		];
		for json in invalid_jsons.iter() {
			let node = serde_json::from_str::<NodeType>(json);
			assert!(node.is_err());
		}
	}
}
