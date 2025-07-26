use crate::graph::{
    concrete_node::ConcreteNode,
    node::{NodeError, NodeResult},
    types::{GraphPayload, GraphPayloadObjects},
};
use serde::Deserialize;
use std::collections::HashSet;

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct IndexNode {
    index: usize,
}

impl ConcreteNode for IndexNode {
    fn set_actual_handles(&mut self, input_handles: HashSet<String>, _: HashSet<String>) -> Result<(), NodeError> {
        if input_handles.len() > 1 {
            return Err(NodeError::HandleValidationError(
                "IndexNode must have at most one input handle".to_string(),
            ));
        }

        Ok(())
    }

    fn generates_reading(&self) -> bool {
        false
    }

    async fn compute_objects(&self, inputs: &GraphPayloadObjects) -> NodeResult {
        match inputs.len() {
            0 => Err(NodeError::InvalidInputError("Expected at least one input".to_string())),
            1 => {
                let json_array = inputs.values().next().unwrap();
                match json_array {
                    serde_json::Value::Array(array) => {
                        if self.index >= array.len() {
                            return Err(NodeError::GenericComputationError("Index out of bounds".to_string()));
                        }
                        Ok(GraphPayload::Objects(
                            vec![(self.default_output_handle(), array[self.index].clone())].into_iter().collect(),
                        ))
                    }
                    _ => Err(NodeError::InvalidInputError("Input must be an array".to_string())),
                }
            }
            _ => Err(NodeError::InvalidInputError("Expected at most one input".to_string())),
        }
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod index_tests {
    use super::*;
    use crate::{graph::types::NodeData, nodes::NodeType};

    fn get_array_payload() -> GraphPayloadObjects {
        GraphPayloadObjects::from([("array".to_string(), serde_json::json!([1, 2, 3, 4, 5]))])
    }

    #[tokio::test]
    async fn no_input() {
        let index_node = IndexNode { index: 0 };
        let inputs = GraphPayloadObjects::new();
        let result = index_node.compute_objects(&inputs).await;
        assert!(matches!(result, Err(NodeError::InvalidInputError(_))));
    }

    #[tokio::test]
    async fn empty_array() {
        let inputs = GraphPayloadObjects::from([("array".to_string(), serde_json::json!([]))]);

        let index_node = IndexNode { index: 0 };
        let result = index_node.compute_objects(&inputs).await;
        assert!(matches!(result, Err(NodeError::GenericComputationError(_))));

        let index_node = IndexNode { index: 5 };
        let result = index_node.compute_objects(&inputs).await;
        assert!(matches!(result, Err(NodeError::GenericComputationError(_))));
    }

    #[tokio::test]
    async fn index_out_of_bounds() {
        let index_node = IndexNode { index: 1 };
        let inputs = GraphPayloadObjects::from([("array".to_string(), serde_json::json!([]))]);
        let result = index_node.compute_objects(&inputs).await;
        assert!(matches!(result, Err(NodeError::GenericComputationError(_))));
    }

    #[tokio::test]
    async fn valid_index() {
        let inputs = get_array_payload();

        let index_node = IndexNode { index: 0 };
        let result = index_node.compute_objects(&inputs).await;
        assert!(matches!(result, Ok(GraphPayload::Objects(_))));
        let result = result.unwrap().get(&index_node.default_output_handle()).unwrap();
        match result {
            NodeData::Object(value) => assert_eq!(value, serde_json::json!(1)),
            _ => panic!("Unexpected NodeData variant"),
        }
    }

    #[tokio::test]
    async fn invalid_input() {
        let index_node = IndexNode { index: 0 };

        let inputs = GraphPayloadObjects::from([("array".to_string(), serde_json::json!(1))]);
        let result = index_node.compute_objects(&inputs).await;
        assert!(matches!(result, Err(NodeError::InvalidInputError(_))));

        let inputs = GraphPayloadObjects::from([("1".to_string(), serde_json::json!(true)), ("2".to_string(), serde_json::json!(false))]);
        let result = index_node.compute_objects(&inputs).await;
        assert!(matches!(result, Err(NodeError::InvalidInputError(_))));
    }

    #[tokio::test]
    async fn generates_reading() {
        let index_node = IndexNode { index: 0 };
        assert!(!index_node.generates_reading());
    }

    #[tokio::test]
    async fn set_actual_handles() {
        let mut index_node = IndexNode { index: 0 };
        let mut input_handles = HashSet::new();
        input_handles.insert("input".to_string());
        let result = index_node.set_actual_handles(input_handles, HashSet::new());
        assert!(matches!(result, Ok(())));

        let mut input_handles = HashSet::new();
        input_handles.insert("input".to_string());
        input_handles.insert("input2".to_string());
        let result = index_node.set_actual_handles(input_handles, HashSet::new());
        assert!(matches!(result, Err(NodeError::HandleValidationError(_))));
    }

    #[tokio::test]
    async fn deserialise_node() {
        let node = serde_json::json!({
            "type": "index",
            "value": {
                "index": 0
            }
        });

        let result = serde_json::from_value(node).unwrap();
        match result {
            NodeType::Index(node) => assert_eq!(node.index, 0),
            _ => panic!("Unexpected NodeType variant"),
        }
    }
}
