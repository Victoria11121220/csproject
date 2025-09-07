use std::collections::HashSet;
use serde::{self, Deserialize, Deserializer, Serialize};
use crate::graph::{
    concrete_node::ConcreteNode,
    node::{NodeError, NodeResult},
    types::{GraphPayload, GraphPayloadObjects, NodeDataObject},
};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConstantNode {
    constant: NodeDataObject,
}

impl<'de> Deserialize<'de> for ConstantNode {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct RawConstantNode {
            r#type: String,
            constant: NodeDataObject,
        }

        let raw = RawConstantNode::deserialize(deserializer)?;

        let valid = match raw.r#type.as_str() {
            "STRING" => raw.constant.is_string(),
            "NUMBER" => raw.constant.is_number(),
            "BOOLEAN" => raw.constant.is_boolean(),
            "OBJECT" => raw.constant.is_object(),
            _ => false,
        };
        if !valid {
            return Err(serde::de::Error::custom(format!(
                "Invalid constant type. Got type {} with value {}",
                raw.r#type, raw.constant
            )));
        }

        Ok(ConstantNode { constant: raw.constant })
    }
}

impl ConcreteNode for ConstantNode {
    fn set_actual_handles(&mut self, input_handles: HashSet<String>, output_handles: HashSet<String>) -> Result<(), NodeError> {
        if !input_handles.is_empty() {
            return Err(NodeError::HandleValidationError(
                "Constant node should not have any input handles".to_string(),
            ));
        }

        if output_handles.len() != 1 {
            return Err(NodeError::HandleValidationError(
                "Constant node should have exactly one output handle".to_string(),
            ));
        }

        Ok(())
    }

    fn generates_reading(&self) -> bool {
        false
    }

    async fn compute_objects(&self, _: &GraphPayloadObjects) -> NodeResult {
        Ok(GraphPayload::Objects(
            vec![(self.default_output_handle(), self.constant.clone())].into_iter().collect(),
        ))
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod constant_test {
    use super::*;

    #[test]
    fn deserialize_from_string() {
        let valid_node_strings = vec![
            "{\"type\":\"BOOLEAN\",\"constant\":false}",
            "{\"type\":\"NUMBER\",\"constant\":3.14}",
            "{\"type\":\"STRING\",\"constant\":\"Hello, World!\"}",
            "{\"type\":\"OBJECT\",\"constant\":{\"key\":\"value\"}}",
        ];

        for node_string in valid_node_strings {
            let node = serde_json::from_str::<ConstantNode>(node_string);
            if let Err(e) = node {
                panic!("Failed to deserialize ConstantNode from string: {}", e);
            }
        }
    }

    #[test]
    fn derserialise_from_invalid_type() {
        let invalid_node_strings = vec![
            "{\"type\":\"INVALID\",\"constant\":false}",
            "{\"type\":\"INVALID\",\"constant\":3.14}",
            "{\"type\":\"INVALID\",\"constant\":\"Hello, World!\"}",
        ];

        for node_string in invalid_node_strings {
            let node = serde_json::from_str::<ConstantNode>(node_string);
            if node.is_ok() {
                panic!("Deserialized ConstantNode from invalid string");
            }
        }
    }

    #[test]
    fn deserialize_from_invalid_string() {
        let invalid_node_strings = vec![
            "{\"type\":\"BOOLEAN\",\"constant\":\"Hello, World!\"}",
            "{\"type\":\"NUMBER\",\"constant\":true}",
            "{\"type\":\"NUMBER\",\"constant\":\"3.14\"}",
            "{\"type\":\"STRING\",\"constant\":3.14}",
            "{\"type\":\"STRING\",\"constant\":false}",
            "{\"type\":\"STRING\",\"constant\":\"\n\"}",
        ];

        for node_string in invalid_node_strings {
            let node = serde_json::from_str::<ConstantNode>(node_string);
            if node.is_ok() {
                panic!("Deserialized ConstantNode from invalid string");
            }
        }
    }

    #[test]
    fn set_actual_handles_valid() {
        let mut node = ConstantNode {
            constant: NodeDataObject::String("".to_string()),
        };
        let input_handles = HashSet::new();
        let output_handles = vec!["output".to_string()].into_iter().collect();
        let result = node.set_actual_handles(input_handles, output_handles);
        if let Err(e) = result {
            panic!("Failed to set actual handles: {}", e);
        }
    }

    #[test]
    fn set_actual_handles_invalid_handles() {
        let mut node = ConstantNode {
            constant: NodeDataObject::String("".to_string()),
        };
        let input_handles = vec!["input".to_string()].into_iter().collect();
        let output_handles = vec!["output".to_string()].into_iter().collect();
        let result = node.set_actual_handles(input_handles, output_handles);
        assert!(result.is_err());

        let input_handles = vec![].into_iter().collect();
        let output_handles = vec!["output".to_string(), "output2".to_string()].into_iter().collect();
        let result = node.set_actual_handles(input_handles, output_handles);
        assert!(result.is_err());
    }

    #[test]
    fn generates_reading() {
        let node = ConstantNode {
            constant: NodeDataObject::String("".to_string()),
        };
        assert!(!node.generates_reading());
    }
    

    #[tokio::test]
    async fn get_string() {
        let node_strings = vec![
            "{\"type\":\"STRING\",\"constant\":\"\"}",
            "{\"type\":\"STRING\",\"constant\":\"Hello, World!\"}",
        ];

        for node_string in node_strings {
            let node = serde_json::from_str::<ConstantNode>(node_string).unwrap();
            let data = node.compute_objects(&GraphPayloadObjects::new()).await.unwrap();
            match data {
                GraphPayload::Objects(objects) => {
                    let object = objects
                        .get(&node.default_output_handle())
                        .expect("Expected default output handle to be present");
                    assert_eq!(object, &node.constant);
                }
                _ => panic!("Expected GraphPayload::Objects, but got {:?}", data),
            }
        }
    }

    #[tokio::test]
    async fn get_number() {
        let node_strings = vec![
            "{\"type\":\"NUMBER\",\"constant\":0}",
            "{\"type\":\"NUMBER\",\"constant\":-1}",
            "{\"type\":\"NUMBER\",\"constant\":1}",
            "{\"type\":\"NUMBER\",\"constant\":3.14}",
            "{\"type\":\"NUMBER\",\"constant\":1e10}",
            "{\"type\":\"NUMBER\",\"constant\":3.14e5}",
        ];

        for node_string in node_strings {
            let node = serde_json::from_str::<ConstantNode>(node_string).unwrap();
            let data = node.compute_objects(&GraphPayloadObjects::new()).await.unwrap();
            match data {
                GraphPayload::Objects(objects) => {
                    let object = objects
                        .get(&node.default_output_handle())
                        .expect("Expected default output handle to be present");
                    assert_eq!(object, &node.constant);
                }
                _ => panic!("Expected GraphPayload::Objects, but got {:?}", data),
            }
        }
    }

    #[tokio::test]
    async fn get_boolean() {
        let node_strings = vec!["{\"type\":\"BOOLEAN\",\"constant\":false}", "{\"type\":\"BOOLEAN\",\"constant\":true}"];

        for node_string in node_strings {
            let node = serde_json::from_str::<ConstantNode>(node_string).unwrap();
            let data = node.compute_objects(&GraphPayloadObjects::new()).await.unwrap();
            match data {
                GraphPayload::Objects(objects) => {
                    let object = objects
                        .get(&node.default_output_handle())
                        .expect("Expected default output handle to be present");
                    assert_eq!(object, &node.constant);
                }
                _ => panic!("Expected GraphPayload::Objects, but got {:?}", data),
            }
        }
    }
}