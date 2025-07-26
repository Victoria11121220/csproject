use std::collections::HashSet;
use crate::graph::node::{ NodeError, NodeResult };
use crate::graph::concrete_node::ConcreteNode;
use crate::graph::types::{ GraphPayload, GraphPayloadCollections, GraphPayloadObjects };
use crate::readings::debug::DebugReading;
use crate::readings::{ EmptyReading, Readings };
use serde::{ Deserialize, Deserializer, Serialize };

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DebugNode {}

impl<'de> Deserialize<'de> for DebugNode {
    fn deserialize<D>(_deserializer: D) -> Result<Self, D::Error> where D: Deserializer<'de> {
        Ok(DebugNode {})
    }
}

impl ConcreteNode for DebugNode {
    fn set_actual_handles(
        &mut self,
        input: HashSet<String>,
        output: HashSet<String>
    ) -> Result<(), NodeError> {
        // Check not more than one output handle
        if output.len() > 1 {
            return Err(
                NodeError::HandleValidationError(
                    "DebugNode must not have more than one output handle".to_string()
                )
            );
        }
        // Check not more than one input handle
        if input.len() > 1 {
            return Err(
                NodeError::HandleValidationError(
                    "DebugNode must not have more than one input handle".to_string()
                )
            );
        }
        Ok(())
    }

    fn generates_reading(&self) -> bool {
        true
    }

    async fn compute_objects(&self, inputs: &GraphPayloadObjects) -> NodeResult {
        Ok(GraphPayload::Objects(vec![
            (
                self.default_output_handle(),
                inputs
                    .iter()
                    .next()
                    .map(|(_, v)| v)
                    .unwrap_or(&serde_json::Value::Null)
                    .clone()
            )
        ].into_iter().collect()))
    }

    fn payload_objects_to_reading(
        &self,
        node_id: &str,
        objects: &GraphPayloadObjects
    ) -> Result<Readings, NodeError> {
        // Check that the input has at least one object
        if objects.is_empty() {
            return Ok(Readings::Empty(EmptyReading {}));
        }

        // Get object from first input handle
        let object = objects.iter().next().unwrap().1.clone();

        // Check if null
        if object.is_null() {
            return Ok(Readings::Empty(EmptyReading {}));
        }

        // Create reading
        Ok(Readings::Debug(DebugReading::new(node_id.to_string(), object)))
    }

    fn payload_collections_to_reading(
        &self,
        node_id: &str,
        collections: &GraphPayloadCollections
    ) -> Result<Vec<Readings>, NodeError> {
        // Check that the input has at least one object
        if collections.is_empty() {
            return Ok(vec![Readings::Empty(EmptyReading {})]);
        }

        // Get object from first input handle
        let objects = collections.iter().next().unwrap().1.clone();

        if
            objects
                .iter()
                .filter(|obj| obj.is_null())
                .count() == objects.len()
        {
            return Ok(vec![Readings::Empty(EmptyReading {})]);
        }

        // Create reading
        Ok(
            vec![
                Readings::Debug(
                    DebugReading::new(node_id.to_string(), serde_json::Value::Array(objects))
                )
            ]
        )
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod debug_tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn deserialise() {
        let node = r#""#;
        let node = serde_json::from_str::<DebugNode>(node);
        assert!(node.is_ok());
    }

    #[test]
    fn deserialise_invalid() {
        let node = r#"[1,2,3]"#;
        let node = serde_json::from_str::<DebugNode>(node);
        assert!(node.is_err());
    }

    #[test]
    fn set_actual_handles() {
        let mut node = DebugNode {};
        let mut input = HashSet::new();
        let mut output = HashSet::new();
        input.insert("input".to_string());
        output.insert("output".to_string());
        let result = node.set_actual_handles(input, output);
        assert!(result.is_ok());
    }

    #[test]
    fn set_actual_handles_invalid() {
        let mut node = DebugNode {};
        let mut input = HashSet::new();
        let mut output = HashSet::new();
        input.insert("input".to_string());
        output.insert("output".to_string());
        output.insert("output2".to_string());
        let result = node.set_actual_handles(input, output);
        assert!(result.is_err());

        let mut node = DebugNode {};
        let mut input = HashSet::new();
        let mut output = HashSet::new();
        input.insert("input".to_string());
        input.insert("input2".to_string());
        output.insert("output".to_string());
        let result = node.set_actual_handles(input, output);
        assert!(result.is_err());
    }

    #[test]
    fn generates_reading() {
        let node = DebugNode {};
        assert!(node.generates_reading());
    }

    #[tokio::test]
    async fn compute_objects() {
        let node = DebugNode {};
        let objects = GraphPayloadObjects::new();
        let result = node.compute_objects(&objects).await;
        assert_eq!(result.unwrap(), GraphPayload::Objects(vec![
            (
                node.default_output_handle(),
                serde_json::Value::Null
            )
        ].into_iter().collect()));
    }

    #[test]
    fn payload_objects_to_reading() {
        let node = DebugNode {};
        let node_id = "test";
        let mut objects = GraphPayloadObjects::new();
        objects.insert("input".to_string(), json!(1));
        let result = node.payload_objects_to_reading(node_id, &objects);
        assert_eq!(
            result.unwrap(),
            Readings::Debug(DebugReading::new(node_id.to_string(), json!(1)))
        );
    }

    #[test]
    fn payload_objects_to_reading_empty_or_null() {
        let node = DebugNode {};
        let node_id = "test";
        let objects = GraphPayloadObjects::new();
        let result = node.payload_objects_to_reading(node_id, &objects);
        assert!(result.is_ok());
        match result.unwrap() {
            Readings::Empty(_) => (),
            _ => panic!("Expected Readings::Empty"),
        }

        let mut objects = GraphPayloadObjects::new();
        objects.insert("input".to_string(), serde_json::Value::Null);
        let result = node.payload_objects_to_reading(node_id, &objects);
        assert!(result.is_ok());
        match result.unwrap() {
            Readings::Empty(_) => (),
            _ => panic!("Expected Readings::Empty"),
        }
    }

    #[test]
    fn payload_collections_to_reading() {
        let node = DebugNode {};
        let node_id = "test";
        let mut collections = GraphPayloadCollections::new();
        collections.insert("input".to_string(), vec![json!(1), json!(2)]);
        let result = node.payload_collections_to_reading(node_id, &collections);
        assert_eq!(
            result.unwrap().first().unwrap(),
            &Readings::Debug(DebugReading::new(node_id.to_string(), json!([1, 2])))
        );
    }

    #[test]
    fn payload_collections_to_reading_empty() {
        let node = DebugNode {};
        let node_id = "test";
        let collections = GraphPayloadCollections::new();
        let result = node.payload_collections_to_reading(node_id, &collections);
        assert!(result.is_ok());
        match result.unwrap().first().unwrap() {
            Readings::Empty(_) => (),
            _ => panic!("Expected Readings::Empty"),
        }
    }

    #[test]
    fn payload_collections_to_reading_filter_null() {
        let node = DebugNode {};
        let node_id = "test";
        let mut collections = GraphPayloadCollections::new();
        collections.insert("input".to_string(), vec![serde_json::Value::Null]);
        let result = node.payload_collections_to_reading(node_id, &collections);
        assert!(result.is_ok());
        match result.unwrap().first().unwrap() {
            Readings::Empty(_) => (),
            _ => panic!("Expected Readings::Empty"),
        }
    }
}
