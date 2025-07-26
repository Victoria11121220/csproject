//! Types used in the graph module
//!
//! The problem this typing systems aims to solve is differentiating between JSON lists as units of data and collections of data units.
//!
//! # Problem Example
//! The mas_monitor node can generate either a single reading as a JSON Object or a list of readings as a JSON Array of Objects.
//! The nodes this data is sent to then have to be able to handle both cases.
//! An alternative could be for each node to check whether its input is a list or a single object and handle it accordingly.
//! However, this would make the nodes more complex and harder to maintain and there is no way for nodes to know if what they are handling is to be considered a single object or a collection of objects.
//! For example, imagine the data moving in the graph (payload) is a JSON list of lists (a matrix) and is passed to an index node that extracts the first element from a list.
//! Should the index node treat the payload as a single object (a matrix) and return the first list or should it treat it as a collection of objects (a list of lists) and return a list of the first elements of each list?
//!
//! # NodeData
//! To solve this problem we introduce the notion of NodeData which can be either a single object or a collection of objects.
//! The objects are not to be thought of as JSON objects but as units of data, atoms, that are handled by the nodes in the graph.
//! However, in this specific case, the atoms are represented by JSON objects, but a JSON list can be an atom in the same way a JSON object can.
//! To attempt and reduce confusion we use the terms NodeDataObject and NodeDataCollection to refer to atoms and ordered collections of atoms respectively.
//! NodeData is simply then a generic type that can be either a NodeDataObject or a NodeDataCollection.
//! In other instances this is referred to as "mixed" data.
//!
//! # GraphPayload
//! Because nodes operate on NodeData, the graph needs a way to move around NodeData instances.
//! Especially, it needs to know from which handle a NodeData instance comes from and to which handle it should be sent.
//! GraphPayload allows to store this information by wrapping NodeData instances in a HashMap where the key is the handle.
//! It then naturally follows from the NodeData distinction between objects and collections, that there would be three types of GraphPayloads: Objects, Collections and Mixed.
//! These three types are simply used to provide further guarantees on the data contained in the payload to simplify the implementation of the nodes.
//!
//! If a node receives a payload of type GraphPayload::Objects, it can be sure that all the data in the payload is of type NodeDataObject and can treat all data instances as units, whether they are JSON objects or JSON lists.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A single unit of data handled by nodes in the graph
pub type NodeDataObject = serde_json::Value;
/// A collection of data units handled by nodes in the graph
pub type NodeDataCollection = Vec<serde_json::Value>;
/// A generic data unit that can be either an object or a collection
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum NodeData {
    Object(NodeDataObject),
    Collection(NodeDataCollection),
}

impl Default for NodeData {
    fn default() -> Self {
        NodeData::Object(serde_json::Value::default())
    }
}

pub type GraphPayloadObjects = HashMap<String, NodeDataObject>;
pub type GraphPayloadCollections = HashMap<String, NodeDataCollection>;
pub type GraphPayloadMixed = HashMap<String, NodeData>;
#[derive(Debug, Clone, PartialEq)]
pub enum GraphPayload {
    Objects(GraphPayloadObjects),
    Collections(GraphPayloadCollections),
    Mixed(GraphPayloadMixed),
}

impl GraphPayload {
    /// Get a NodeData instance from the payload by key
    ///
    /// This is just a helper function to work around Rust type system
    /// Since all payloads are maps, it makes sense to be able to access the data by key.
    /// However, doing so on a concrete instance would result in rust complaining about the type that is returned.
    /// This function simply casts the return to a generic NodeData type regardless of the concrete type of the payload.
    pub fn get(&self, key: &str) -> Option<NodeData> {
        match self {
            GraphPayload::Objects(objects) => objects.get(key).map(|v| NodeData::Object(v.clone())),
            GraphPayload::Collections(collections) => collections.get(key).map(|v| NodeData::Collection(v.clone())),
            GraphPayload::Mixed(mixed) => mixed.get(key).cloned(),
        }
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;

    #[test]
    fn test_node_data() {
        let node_data = NodeData::default();
        match node_data {
            NodeData::Object(data) => assert_eq!(data, serde_json::Value::default()),
            _ => panic!("Expected NodeData::Object"),
        }

        let node_data = NodeData::Object(serde_json::json!(1));
        let serialized = serde_json::to_string(&node_data).unwrap();
        let deserialized: NodeData = serde_json::from_str(&serialized).unwrap();
        match deserialized {
            NodeData::Object(data) => assert_eq!(data, serde_json::json!(1)),
            _ => panic!("Expected NodeData::Object"),
        }
    }

    #[test]
    fn test_graph_payload_get() {
        let mut objects = HashMap::new();
        objects.insert("object".to_string(), serde_json::json!(1));
        let payload = GraphPayload::Objects(objects);
        match payload.get("object") {
            Some(NodeData::Object(data)) => assert_eq!(data, serde_json::json!(1)),
            _ => panic!("Expected NodeData::Object"),
        }
        assert!(payload.get("error").is_none());

        let mut collections = HashMap::new();
        collections.insert("collection".to_string(), vec![serde_json::json!(2)]);
        let payload = GraphPayload::Collections(collections);
        match payload.get("collection") {
            Some(NodeData::Collection(data)) => assert_eq!(data, vec![serde_json::json!(2)]),
            _ => panic!("Expected NodeData::Collection"),
        }
        assert!(payload.get("error").is_none());

        let mut mixed = HashMap::new();
        mixed.insert("mixed_obj".to_string(), NodeData::Object(serde_json::json!(1)));
        mixed.insert("mixed_collection".to_string(), NodeData::Collection(vec![serde_json::json!(2)]));
        let payload = GraphPayload::Mixed(mixed);
        match payload.get("mixed_obj") {
            Some(NodeData::Object(data)) => assert_eq!(data, serde_json::json!(1)),
            _ => panic!("Expected NodeData::Object"),
        }
        match payload.get("mixed_collection") {
            Some(NodeData::Collection(data)) => assert_eq!(data, vec![serde_json::json!(2)]),
            _ => panic!("Expected NodeData::Collection"),
        }
        assert!(payload.get("error").is_none());
    }
}
