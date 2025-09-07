use crate::graph::{
    concrete_node::ConcreteNode,
    node::{NodeError, NodeResult},
    types::{GraphPayload, GraphPayloadObjects},
};
use serde::Deserialize;
use std::collections::HashSet;

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct MetadataNode {
    items: Vec<String>,
}

impl ConcreteNode for MetadataNode {
    fn set_actual_handles(&mut self, input_handles: HashSet<String>, output_handles: HashSet<String>) -> Result<(), NodeError> {
        // Check only one output handle
        if output_handles.len() > 1 {
            return Err(NodeError::HandleValidationError(
                "MetadataNode must not have more than one output handle".to_string(),
            ));
        }
        // Check that there is a corresponding item for each input handle
        for handle in &input_handles {
            if !self.items.contains(handle) {
                return Err(NodeError::HandleValidationError(format!(
                    "A MetadataNode input handle does not have a corresponding item: {}",
                    handle
                )));
            }
        }
        Ok(())
    }

    fn generates_reading(&self) -> bool {
        false
    }

    async fn compute_objects(&self, inputs: &GraphPayloadObjects) -> NodeResult {
        // Check that all inputs are valid items
        for (handle, _) in inputs.iter() {
            if !self.items.contains(handle) {
                return Err(NodeError::HandleValidationError(format!(
                    "A MetadataNode input handle does not have a corresponding item: {}",
                    handle
                )));
            }
        }
        // Build a json object from the inputs
        // (HashMap<String, Value> -> serde_json::Map<String, Value> -> serde_json::Value::Object)
        let output = serde_json::Value::Object(inputs.clone().into_iter().collect());
        Ok(GraphPayload::Objects(vec![(self.default_output_handle(), output)].into_iter().collect()))
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod metadata_tests {

    use super::*;

    #[test]
    fn deserialise_valid() {
        let json = r#"{
            "items": ["a", "b"]
        }"#;
        let node: MetadataNode = serde_json::from_str(json).unwrap();
        assert_eq!(node.items, vec!["a", "b"]);
    }

    #[test]
    fn deserialise_invalid() {
        let json = r#"{
            "items": "a"
        }"#;
        let node: Result<MetadataNode, _> = serde_json::from_str(json);
        assert!(node.is_err());
    }

    fn return_node(handles: Vec<String>) -> MetadataNode {
        MetadataNode { items: handles }
    }

    #[test]
    fn set_handles_valid() {
        let mut node = return_node(vec!["a".to_string(), "b".to_string()]);
        let input_handles = vec!["a".to_string(), "b".to_string()].into_iter().collect();
        let output_handles = vec!["source".to_string()].into_iter().collect();
        let result = node.set_actual_handles(input_handles, output_handles);
        assert!(result.is_ok());
    }

    #[test]
    fn set_handles_invalid() {
        let mut node = return_node(vec!["a".to_string(), "b".to_string()]);
        let input_handles: HashSet<String> = vec!["a".to_string(), "c".to_string()].into_iter().collect();
        let output_handles = vec!["source".to_string()].into_iter().collect();
        let result = node.set_actual_handles(input_handles.clone(), output_handles);
        assert!(matches!(result, Err(NodeError::HandleValidationError(_))));

        let output_handles = vec!["source".to_string(), "source2".to_string()].into_iter().collect();
        let result = node.set_actual_handles(input_handles, output_handles);
        assert!(matches!(result, Err(NodeError::HandleValidationError(_))));
    }

    #[test]
    fn generates_reading() {
        let node = return_node(vec!["a".to_string(), "b".to_string()]);
        assert!(!node.generates_reading());
    }

    #[tokio::test]
    async fn compute_objects_valid() {
        let node = return_node(vec!["a".to_string(), "b".to_string()]);
        let inputs = vec![
            ("a".to_string(), serde_json::Value::String("value_a".to_string())),
            ("b".to_string(), serde_json::Value::String("value_b".to_string())),
        ]
        .into_iter()
        .collect();
        let result = node.compute_objects(&inputs).await;
        assert!(result.is_ok());
        let output = result.unwrap();
        let expected = serde_json::json!({
            "a": "value_a",
            "b": "value_b",
        });
        match output {
            GraphPayload::Objects(objects) => {
                assert_eq!(objects.len(), 1);
                assert_eq!(objects["source"], expected);
            }
            _ => panic!("Expected GraphPayload::Objects"),
        }
    }

    #[tokio::test]
    async fn compute_objects_empty_input() {
        let node = return_node(vec!["a".to_string(), "b".to_string()]);
        let inputs = GraphPayloadObjects::new();
        let result = node.compute_objects(&inputs).await;
        assert!(result.is_ok());
        let output = result.unwrap();
        match output {
            GraphPayload::Objects(objects) => {
                assert_eq!(objects.len(), 1);
                assert_eq!(objects["source"], serde_json::json!({}));
            }
            _ => panic!("Expected GraphPayload::Objects"),
        }
    }

    #[tokio::test]
    async fn compute_objects_empty_items() {
        let node = return_node(vec![]);
        let inputs = GraphPayloadObjects::new();
        let result = node.compute_objects(&inputs).await;
        assert!(result.is_ok());
        let output = result.unwrap();
        match output {
            GraphPayload::Objects(objects) => {
                assert_eq!(objects.len(), 1);
                assert_eq!(objects["source"], serde_json::json!({}));
            }
            _ => panic!("Expected GraphPayload::Objects"),
        }
    }

    #[tokio::test]
    async fn compute_objects_invalid_handle() {
        let node = return_node(vec!["a".to_string(), "b".to_string()]);
        let inputs = vec![
            ("a".to_string(), serde_json::Value::String("value_a".to_string())),
            ("c".to_string(), serde_json::Value::String("value_c".to_string())),
        ]
        .into_iter()
        .collect();
        let result = node.compute_objects(&inputs).await;
        assert!(matches!(result, Err(NodeError::HandleValidationError(_))));
    }
}