use crate::graph::{
    node::{NodeError, NodeResult},
    types::{GraphPayload, NodeData},
};
use std::collections::HashMap;

/// The type of payload that a node can generate.
pub enum GraphPayloadType {
    Objects,
    Collections,
    Mixed,
}

/// Update the result type based on the current type and the data.
///
/// This is essentially a type upcast to the most restrictive type that can hold both the current type and the new data.
/// 
/// For example, a collection payload of multiple sizes will be treated as a valid collections payload.
/// In this case, the [`ConcreteNode::compute`] is responsible for checking if the sizes are uniform and attempt to reconcile them if necessary.
pub fn update_result_type(current_type: Option<GraphPayloadType>, data: &NodeData) -> GraphPayloadType {
    match current_type {
        None => match data {
            NodeData::Object(_) => GraphPayloadType::Objects,
            NodeData::Collection(_) => GraphPayloadType::Collections,
        },
        Some(GraphPayloadType::Mixed) => GraphPayloadType::Mixed, // Mixed is the least restrictive type possible already
        Some(GraphPayloadType::Objects) => match data {
            NodeData::Object(_) => GraphPayloadType::Objects,
            NodeData::Collection(_) => GraphPayloadType::Mixed,
        },
        Some(GraphPayloadType::Collections) => match data {
            NodeData::Object(_) => GraphPayloadType::Mixed,
            NodeData::Collection(_) => GraphPayloadType::Collections,
        },
    }
}

/// Cast a map of input handles to NodeData to a GraphPayload given the type of payload.
///
/// If the input map is malformed and does not correspond to the payload type, an error is returned.
/// The function does not attempt to reconcile the input map with the payload type.
pub fn get_payload_based_on_type(payload_type: Option<GraphPayloadType>, inputs_map: HashMap<String, NodeData>) -> NodeResult {
    match payload_type {
        None => Ok(GraphPayload::Objects(HashMap::new())),
        Some(GraphPayloadType::Objects) => {
            let mut objects = HashMap::new();
            for (input_handle, object) in inputs_map {
                match object {
                    NodeData::Object(object) => {
                        objects.insert(input_handle, object);
                    }
                    _ => return NodeResult::Err(NodeError::InvalidInputError(format!("Expected NodeData::Object, but got {:?}", object))),
                }
            }
            Ok(GraphPayload::Objects(objects))
        }
        Some(GraphPayloadType::Collections) => {
            let mut collections = HashMap::new();
            for (input_handle, collection) in inputs_map {
                match collection {
                    NodeData::Collection(collection) => {
                        collections.insert(input_handle.clone(), collection);
                    }
                    _ => {
                        return NodeResult::Err(NodeError::InvalidInputError(format!(
                            "Expected NodeData::Collection, but got {:?}",
                            collection
                        )))
                    }
                }
            }
            Ok(GraphPayload::Collections(collections))
        }
        Some(GraphPayloadType::Mixed) => Ok(GraphPayload::Mixed(inputs_map)),
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;
    use crate::graph::types::NodeDataObject;

    #[test]
    fn test_update_type_none() {
        let data = NodeData::Object(NodeDataObject::default());
        assert!(matches!(update_result_type(None, &data), GraphPayloadType::Objects));

        let data = NodeData::Collection(vec![]);
        assert!(matches!(update_result_type(None, &data), GraphPayloadType::Collections));
    }

    #[test]
    fn test_update_type_objects() {
        let data = NodeData::Object(NodeDataObject::default());
        assert!(matches!(update_result_type(Some(GraphPayloadType::Objects), &data), GraphPayloadType::Objects));
        assert!(matches!(update_result_type(Some(GraphPayloadType::Collections), &data), GraphPayloadType::Mixed));
        assert!(matches!(update_result_type(Some(GraphPayloadType::Mixed), &data), GraphPayloadType::Mixed));
    }

    #[test]
    fn test_update_type_collections() {
        let data = NodeData::Collection(vec![]);
        assert!(matches!(update_result_type(Some(GraphPayloadType::Collections), &data), GraphPayloadType::Collections));
        assert!(matches!(update_result_type(Some(GraphPayloadType::Objects), &data), GraphPayloadType::Mixed));
        assert!(matches!(update_result_type(Some(GraphPayloadType::Mixed), &data), GraphPayloadType::Mixed));
    }

    #[test]
    fn test_update_type_mixed() {
        let data = NodeData::Object(NodeDataObject::default());
        assert!(matches!(update_result_type(Some(GraphPayloadType::Collections), &data), GraphPayloadType::Mixed));
        assert!(matches!(update_result_type(Some(GraphPayloadType::Mixed), &data), GraphPayloadType::Mixed));

        let data = NodeData::Collection(vec![]);
        assert!(matches!(update_result_type(Some(GraphPayloadType::Objects), &data), GraphPayloadType::Mixed));
        assert!(matches!(update_result_type(Some(GraphPayloadType::Mixed), &data), GraphPayloadType::Mixed));
    }

    #[test]
    fn get_none_payload() {
        let inputs_map = HashMap::new();
        let payload = get_payload_based_on_type(None, inputs_map);
        assert!(matches!(payload, Ok(GraphPayload::Objects(_))));
        assert!(payload.unwrap().get("input1").is_none());
    }

    #[test]
    fn get_correct_payload_objects() {
        let mut inputs_map = HashMap::new();
        inputs_map.insert("input1".to_string(), NodeData::Object(NodeDataObject::from(1.)));
        inputs_map.insert("input2".to_string(), NodeData::Object(NodeDataObject::from(2.)));

        let payload = get_payload_based_on_type(Some(GraphPayloadType::Objects), inputs_map);
        match payload {
            Ok(GraphPayload::Objects(data)) => {
                assert_eq!(data.get("input1").unwrap(), &NodeDataObject::from(1.));
                assert_eq!(data.get("input2").unwrap(), &NodeDataObject::from(2.));
            }
            _ => panic!("Expected GraphPayload::Objects"),
        }
    }

    #[test]
    fn get_correct_payload_uniform_collections() {
        let mut inputs_map = HashMap::new();
        inputs_map.insert("input1".to_string(), NodeData::Collection(vec![NodeDataObject::from(1.)]));
        inputs_map.insert("input2".to_string(), NodeData::Collection(vec![NodeDataObject::from(2.)]));

        let payload = get_payload_based_on_type(Some(GraphPayloadType::Collections), inputs_map);
        match payload {
            Ok(GraphPayload::Collections(data)) => {
                assert_eq!(data.get("input1").unwrap(), &vec![NodeDataObject::from(1.)]);
                assert_eq!(data.get("input2").unwrap(), &vec![NodeDataObject::from(2.)]);
            }
            _ => panic!("Expected GraphPayload::Collections"),
        }
    }

    #[test]
    fn get_correct_uniform_collections() {
        let mut inputs_map = HashMap::new();
        inputs_map.insert("input1".to_string(), NodeData::Collection([1., 2., 3.].iter().map(|&x| NodeDataObject::from(x)).collect()));
        inputs_map.insert("input2".to_string(), NodeData::Collection(vec![NodeDataObject::from("a")]));

        let payload = get_payload_based_on_type(Some(GraphPayloadType::Collections), inputs_map);
        match payload {
            Ok(GraphPayload::Collections(data)) => {
                assert_eq!(data.get("input1").unwrap(), &vec![NodeDataObject::from(1.), NodeDataObject::from(2.), NodeDataObject::from(3.)]);
                assert_eq!(data.get("input2").unwrap(), &vec![NodeDataObject::from("a")]);
            }
            _ => panic!("Expected GraphPayload::Collections"),
        }
    }

    #[test]
    fn get_correct_mixed_payload() {
        let mut inputs_map = HashMap::new();
        inputs_map.insert("input1".to_string(), NodeData::Object(NodeDataObject::from(1.)));
        inputs_map.insert("input2".to_string(), NodeData::Collection(vec![NodeDataObject::from(2.)]));

        let payload = get_payload_based_on_type(Some(GraphPayloadType::Mixed), inputs_map);
        match payload {
            Ok(GraphPayload::Mixed(data)) => {
                assert!(matches!(data.get("input1").unwrap(), NodeData::Object(_)));
                assert!(matches!(data.get("input2").unwrap(), NodeData::Collection(_)));
            }
            _ => panic!("Expected GraphPayload::Mixed"),
        }
    }

    #[test]
    fn get_wrong_objects() {
        let mut inputs_map = HashMap::new();
        inputs_map.insert("input1".to_string(), NodeData::Object(NodeDataObject::from(1.)));
        inputs_map.insert("input2".to_string(), NodeData::Collection(vec![NodeDataObject::from(2.)]));

        let payload = get_payload_based_on_type(Some(GraphPayloadType::Objects), inputs_map);
        assert!(matches!(payload, Err(NodeError::InvalidInputError(_))));
    }

    #[test]
    fn get_wrong_collections() {
        let mut inputs_map = HashMap::new();
        inputs_map.insert("input1".to_string(), NodeData::Object(NodeDataObject::from(1.)));
        inputs_map.insert("input2".to_string(), NodeData::Collection(vec![NodeDataObject::from(2.)]));

        let payload = get_payload_based_on_type(Some(GraphPayloadType::Collections), inputs_map);
        assert!(matches!(payload, Err(NodeError::InvalidInputError(_))));
    }
}