use crate::graph::{
    concrete_node::ConcreteNode,
    node::{NodeError, NodeResult},
    types::{GraphPayload, GraphPayloadObjects},
};
use serde::Deserialize;
use std::collections::HashSet;

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct KeyNode {
    key: Vec<String>,
}

impl ConcreteNode for KeyNode {
    fn set_actual_handles(&mut self, input_handles: HashSet<String>, output_handles: HashSet<String>) -> Result<(), NodeError> {
        if input_handles.len() > 1 {
            return Err(NodeError::HandleValidationError(
                "KeyNode must not have more than one input handle".to_string(),
            ));
        }
        if output_handles.len() > 1 {
            return Err(NodeError::HandleValidationError(
                "KeyNode must not have more than one output handle".to_string(),
            ));
        }
        Ok(())
    }

    fn generates_reading(&self) -> bool {
        false
    }

    async fn compute_objects(&self, inputs: &GraphPayloadObjects) -> NodeResult {
        if inputs.len() != 1 {
            return Err(crate::graph::node::NodeError::HandleValidationError(
                "KeyNode must have exactly one input handle".to_string(),
            ));
        }

        let mut input = inputs.values().next().unwrap();
        for key in self.key.iter() {
            match input {
                serde_json::Value::Object(map) => {
                    if map.contains_key(key) {
                        input = map.get(key).unwrap();
                    } else {
                        return Err(NodeError::GenericComputationError(format!("Key {} not found in object", key)));
                    }
                }
                _ => {
                    return Err(NodeError::GenericComputationError("Cannot get key from a non-object".to_string()));
                }
            }
        }
        Ok(GraphPayload::Objects(
            vec![(self.default_output_handle(), input.clone())].into_iter().collect(),
        ))
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod key_node_tests {
    use super::*;
    use crate::graph::types::NodeData;
    use std::collections::HashMap;

    #[test]
    fn deserialise_key_node() {
        let json = r#"{"key": ["a", "b"]}"#;
        let node: KeyNode = serde_json::from_str(json).unwrap();
        assert_eq!(node.key, vec!["a".to_string(), "b".to_string()]);
    }

    #[test]
    #[should_panic]
    fn deserialise_key_node_missing_key() {
        let json = r#"{}"#;
        let _node: KeyNode = serde_json::from_str(json).unwrap();
    }

    fn get_key_node() -> KeyNode {
        KeyNode {
            key: vec!["a".to_string(), "b".to_string()],
        }
    }

    #[test]
    fn generates_reading() {
        let node = get_key_node();
        assert!(!node.generates_reading());
    }

    #[test]
    fn set_actual_handles() {
        let mut node = get_key_node();
        let mut input_handles = HashSet::new();
        input_handles.insert("input".to_string());
        let mut output_handles = HashSet::new();
        output_handles.insert("output".to_string());
        let result = node.set_actual_handles(input_handles, output_handles);
        assert!(matches!(result, Ok(())));
    }

    #[test]
    fn set_invalid_input_handles() {
        let mut node = get_key_node();
        let mut input_handles = HashSet::new();
        input_handles.insert("input".to_string());
        input_handles.insert("input2".to_string());
        let mut output_handles = HashSet::new();
        output_handles.insert("output".to_string());
        let result = node.set_actual_handles(input_handles, output_handles);
        assert!(matches!(result, Err(NodeError::HandleValidationError(_))));
    }

    #[test]
    fn set_invalid_output_handles() {
        let mut node = get_key_node();
        let mut input_handles = HashSet::new();
        input_handles.insert("input".to_string());
        let mut output_handles = HashSet::new();
        output_handles.insert("output".to_string());
        output_handles.insert("output2".to_string());
        let result = node.set_actual_handles(input_handles, output_handles);
        assert!(matches!(result, Err(NodeError::HandleValidationError(_))));
    }

    #[tokio::test]
    async fn key_node_compute_objects() {
        let node = get_key_node();
        let mut map = HashMap::new();
        map.insert("input".to_string(), serde_json::json!({"a": {"b": 1}}));
        let inputs = GraphPayloadObjects::from(map);
        let result = node.compute_objects(&inputs).await.unwrap();
        let output = result.get(&node.default_output_handle()).unwrap();
        match output {
            NodeData::Object(data) => match data {
                serde_json::Value::Number(num) => {
                    assert_eq!(num.as_i64().unwrap(), 1);
                }
                _ => {
                    panic!("Output is not a number");
                }
            },
            _ => {
                panic!("Output is not an object");
            }
        }
    }

    #[tokio::test]
    async fn no_inputs() {
        let node = get_key_node();
        let inputs = GraphPayloadObjects::new();
        let result = node.compute_objects(&inputs).await;
        assert!(matches!(result, Err(NodeError::HandleValidationError(_))));
    }

    #[tokio::test]
    async fn key_not_found() {
        let node = get_key_node();
        let mut map = HashMap::new();
        map.insert("input".to_string(), serde_json::json!({"a": {"c": 1}}));
        let inputs = GraphPayloadObjects::from(map);
        let result = node.compute_objects(&inputs).await;
        assert!(matches!(result, Err(NodeError::GenericComputationError(_))));
    }

    #[tokio::test]
    async fn key_not_object() {
        let node = get_key_node();
        let mut map = HashMap::new();
        map.insert("input".to_string(), serde_json::json!(1));
        let inputs = GraphPayloadObjects::from(map);
        let result = node.compute_objects(&inputs).await;
        assert!(matches!(result, Err(NodeError::GenericComputationError(_))));
    }
}
