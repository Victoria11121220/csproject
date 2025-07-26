use crate::graph::{
    node::{NodeError, NodeResult},
    types::{GraphPayload, GraphPayloadMixed, GraphPayloadObjects, NodeData, NodeDataObject},
};
use std::collections::HashSet;

/// Attempt to normalize a mixed input payload to a more specific type.
///
/// # Errors
/// Payloads with collections of different sizes will not be cast to a common size as this would misalign the data, an error will be returned instead.
/// Mixed payloads with objects and empty collections will also return an error instead of using default JSON objects to fill the collections.
///
/// # Returns
/// A normalized input payload
/// Although the output type allows for mixed payloads to be returned, the function will never return a mixed payload.
/// If a collection payload is returned, all collections will have the same size.
/// This function is opinionated and will:
/// - Return an empty object payload if the input is empty and has no handles
/// - Return an empty collection payload with the same input handles if the input has handles, but they are all empty collections
/// - Cast a collection payload with all collections of size 1 to an object payload
pub fn normalize_inputs(inputs: &GraphPayloadMixed) -> NodeResult {
    let inputs_sizes = inputs
        .values()
        .map(|c| match c {
            NodeData::Object(_) => 1,
            NodeData::Collection(c) => c.len(),
        })
        .collect::<HashSet<_>>();

    match inputs_sizes.len() {
        // If there are no sizes the input is empty and has no handles so we return an empty object payload
        0 => Ok(GraphPayload::Objects(GraphPayloadObjects::new())),
        // Non-zero input_sizes length means there are some handles
        // If there is only one size then the input is not truly mixed and can be cast to a more specific type
        // If all the sizes are 0 then the payload is made of empty collections and we return an empty collection payload with the same handles
        1 if inputs_sizes.contains(&0) => {
            let empty_collections_payload = inputs.keys().map(|handle| (handle.clone(), Vec::new())).collect();
            Ok(GraphPayload::Collections(empty_collections_payload))
        }
        // If the only size is 1 then all handle data can be cast to objects
        // This is slightly opinionated, as a size of 1 could also be a collection of 1 element, but realistically it is more likely to be an object
        1 if inputs_sizes.contains(&1) => {
            let mut objects_payload = GraphPayloadObjects::new();
            for (handle, data) in inputs.iter() {
                match data {
                    NodeData::Object(obj) => objects_payload.insert(handle.clone(), obj.clone()),
                    // We access the first element of the collection only because we have already checked that the size of all handle data is 1
                    NodeData::Collection(coll) => objects_payload.insert(handle.clone(), coll[0].clone()),
                };
            }
            Ok(GraphPayload::Objects(objects_payload))
        }
        // If there is only one size and it is > 1 then all handle data is collections of the same length and the payload can be cast to a collection payload
        1 => {
            let collections_payload = inputs
                .iter()
                .map(|(handle, data)| (handle.clone(), verified_node_data_to_collection(data)))
                .collect();
            Ok(GraphPayload::Collections(collections_payload))
        }
        // If there are 2 or more sizes the input is truly a mixed type so we need to check that all sizes can be cast to a common one
        // The cast can only happen if there are only 2 sizes: 1 for objects and X for collections
        // If one of the sizes is 0 then the collections are empty and we cannot possibly reconcile them
        // Admittedly, we could cast all collections to collections of default JSON objects, but we'd rather fail early
        2 if inputs_sizes.contains(&0) => Err(NodeError::InputNormalizationError("Cannot cast empty collections".into())),
        // If one of the sizes is 1 (and the other size is not 0) then we can cast all objects to collections of the same object and same size as the true collections
        // This could also be the case for a payload that contains collections of length 1 and collections of length > 1. In this case, we treat the collections of length 1 as objects
        2 if inputs_sizes.contains(&1) => {
            let common_size = *inputs_sizes.iter().max().unwrap();
            let collections_payload = inputs
                .iter()
                .map(|(handle, data)| {
                    (
                        handle.clone(),
                        match data {
                            NodeData::Object(obj) => vec![obj.clone(); common_size],
                            NodeData::Collection(collection) if collection.len() == 1 => vec![collection[0].clone(); common_size],
                            NodeData::Collection(collection) => collection.clone(),
                        },
                    )
                })
                .collect();
            Ok(GraphPayload::Collections(collections_payload))
        }
        // If the sizes are both > 1 then we cannot possibly cast them to a common size, as we cannot account for different data periods
        // Even casting them to the LCM of the sizes would not be correct as the data would be misaligned
        // This also applies if there are more than 2 sizes
        _ => Err(NodeError::InputNormalizationError("Cannot cast collections of different sizes".into())),
    }
}

/// Helper function to convert a verified collection to a collection
/// Obviously, this function should only be called with inputs that have been verified to be collections
/// This is a private function and should not be called outside of this module
/// We turn off coverage for this function because it contains unreachable code
#[cfg_attr(coverage_nightly, coverage(off))]
fn verified_node_data_to_collection(data: &NodeData) -> Vec<NodeDataObject> {
    match data {
        NodeData::Collection(collection) => collection.clone(),
        NodeData::Object(_) => unreachable!("All handles hold collections"),
    }
}

#[cfg_attr(coverage_nightly, coverage(off))]
#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::types::NodeDataObject;

    #[test]
    fn test_empty_input() {
        let inputs = GraphPayloadMixed::new();
        match normalize_inputs(&inputs) {
            Ok(GraphPayload::Objects(payload)) => assert_eq!(payload.len(), 0),
            _ => panic!("Expected empty object payload"),
        }
    }

    #[test]
    fn test_empty_collections() {
        let mut inputs = GraphPayloadMixed::new();
        inputs.insert("input1".to_string(), NodeData::Collection(Vec::new()));
        inputs.insert("input2".to_string(), NodeData::Collection(Vec::new()));
        match normalize_inputs(&inputs) {
            Ok(GraphPayload::Collections(payload)) => {
                assert!(payload.keys().collect::<Vec<_>>().contains(&&"input1".to_string()));
                assert!(payload.keys().collect::<Vec<_>>().contains(&&"input2".to_string()));
                assert_eq!(payload.get("input1").unwrap().len(), 0);
                assert_eq!(payload.get("input2").unwrap().len(), 0);
            }
            _ => panic!("Expected empty collection payload"),
        }
    }

    #[test]
    fn test_mixed_to_object() {
        let mut inputs = GraphPayloadMixed::new();
        inputs.insert("input1".to_string(), NodeData::Object(NodeDataObject::from(1.)));
        inputs.insert("input2".to_string(), NodeData::Object(NodeDataObject::from(2.)));
        match normalize_inputs(&inputs) {
            Ok(GraphPayload::Objects(payload)) => {
                assert_eq!(payload.len(), 2);
                assert_eq!(payload.get("input1").unwrap(), &NodeDataObject::from(1.));
                assert_eq!(payload.get("input2").unwrap(), &NodeDataObject::from(2.));
            }
            _ => panic!("Expected object payload"),
        }

        let mut inputs = GraphPayloadMixed::new();
        inputs.insert("input1".to_string(), NodeData::Collection(vec![NodeDataObject::from(1.)]));
        inputs.insert("input2".to_string(), NodeData::Collection(vec![NodeDataObject::from(2.)]));
        match normalize_inputs(&inputs) {
            Ok(GraphPayload::Objects(payload)) => {
                assert_eq!(payload.len(), 2);
                assert_eq!(payload.get("input1").unwrap(), &NodeDataObject::from(1.));
                assert_eq!(payload.get("input2").unwrap(), &NodeDataObject::from(2.));
            }
            _ => panic!("Expected object payload"),
        }

        let mut inputs = GraphPayloadMixed::new();
        inputs.insert("input1".to_string(), NodeData::Object(NodeDataObject::from(1.)));
        inputs.insert("input2".to_string(), NodeData::Collection(vec![NodeDataObject::from(2.)]));
        match normalize_inputs(&inputs) {
            Ok(GraphPayload::Objects(payload)) => {
                assert_eq!(payload.len(), 2);
                assert_eq!(payload.get("input1").unwrap(), &NodeDataObject::from(1.));
                assert_eq!(payload.get("input2").unwrap(), &NodeDataObject::from(2.));
            }
            _ => panic!("Expected object payload"),
        }
    }

    #[test]
    fn test_uniform_collections() {
        let mut inputs = GraphPayloadMixed::new();
        inputs.insert(
            "input1".to_string(),
            NodeData::Collection([1., 2., 3.].iter().map(|x| NodeDataObject::from(*x)).collect()),
        );
        inputs.insert(
            "input2".to_string(),
            NodeData::Collection([1., 2., 3.].iter().map(|x| NodeDataObject::from(*x)).collect()),
        );
        match normalize_inputs(&inputs) {
            Ok(GraphPayload::Collections(payload)) => {
                assert_eq!(payload.len(), 2);
                assert_eq!(
                    payload.get("input1").unwrap(),
                    &vec![NodeDataObject::from(1.), NodeDataObject::from(2.), NodeDataObject::from(3.)]
                );
                assert_eq!(
                    payload.get("input2").unwrap(),
                    &vec![NodeDataObject::from(1.), NodeDataObject::from(2.), NodeDataObject::from(3.)]
                );
            }
            _ => panic!("Expected collection payload"),
        }
    }

    #[test]
    fn mixed_with_empty_collections() {
        let mut inputs = GraphPayloadMixed::new();
        inputs.insert("input1".to_string(), NodeData::Collection(Vec::new()));
        inputs.insert("input2".to_string(), NodeData::Object(NodeDataObject::from(1.)));
        match normalize_inputs(&inputs) {
            Err(NodeError::InputNormalizationError(_)) => {}
            _ => panic!("Expected error"),
        }
    }

    #[test]
    fn successful_cast() {
        let mut inputs = GraphPayloadMixed::new();
        inputs.insert(
            "input1".to_string(),
            NodeData::Collection([1., 2., 3.].iter().map(|x| NodeDataObject::from(*x)).collect()),
        );
        inputs.insert("input2".to_string(), NodeData::Object(NodeDataObject::from("a")));
        match normalize_inputs(&inputs) {
            Ok(GraphPayload::Collections(payload)) => {
                assert_eq!(payload.len(), 2);
                assert_eq!(
                    payload.get("input1").unwrap(),
                    &vec![NodeDataObject::from(1.), NodeDataObject::from(2.), NodeDataObject::from(3.)]
                );
                assert_eq!(
                    payload.get("input2").unwrap(),
                    &vec![NodeDataObject::from("a"), NodeDataObject::from("a"), NodeDataObject::from("a")]
                );
            }
            _ => panic!("Expected collection payload"),
        }
    }

    #[test]
    fn multiple_collections_with_2_valid_sizes() {
        let mut inputs = GraphPayloadMixed::new();
        inputs.insert(
            "input1".to_string(),
            NodeData::Collection([1., 2., 3.].iter().map(|x| NodeDataObject::from(*x)).collect()),
        );
        inputs.insert(
            "input2".to_string(),
            NodeData::Collection(["a"].iter().map(|&x| NodeDataObject::from(x)).collect()),
        );
        match normalize_inputs(&inputs) {
            Ok(GraphPayload::Collections(payload)) => {
                assert_eq!(payload.len(), 2);
                assert_eq!(
                    payload.get("input1").unwrap(),
                    &vec![NodeDataObject::from(1.), NodeDataObject::from(2.), NodeDataObject::from(3.)]
                );
                assert_eq!(payload.get("input2").unwrap(), &vec![NodeDataObject::from("a"); 3]);
                assert_eq!(payload.get("input1").unwrap().len(), 3);
                assert_eq!(payload.get("input2").unwrap().len(), 3);
            }
            _ => panic!("Expected collection payload"),
        }
    }

    #[test]
    fn multiple_sizes_unreconcilable() {
        let mut inputs = GraphPayloadMixed::new();
        inputs.insert(
            "input1".to_string(),
            NodeData::Collection([1., 2., 3.].iter().map(|x| NodeDataObject::from(*x)).collect()),
        );
        inputs.insert(
            "input2".to_string(),
            NodeData::Collection([1., 2.].iter().map(|x| NodeDataObject::from(*x)).collect()),
        );
        inputs.insert("input3".to_string(), NodeData::Object(NodeDataObject::from(1.)));
        match normalize_inputs(&inputs) {
            Err(NodeError::InputNormalizationError(_)) => {}
            _ => panic!("Expected error"),
        }
    }
}
