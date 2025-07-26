use crate::graph::{
    concrete_node::ConcreteNode,
    node::{NodeError, NodeResult},
    types::{GraphPayload, GraphPayloadObjects},
};
use serde::{Deserialize, Deserializer};
use std::collections::HashSet;

#[derive(Debug, Clone)]
pub struct CurrentTimestampNode {}

impl<'de> Deserialize<'de> for CurrentTimestampNode {
    fn deserialize<D>(_deserializer: D) -> Result<Self, D::Error> where D: Deserializer<'de> {
        Ok(CurrentTimestampNode {})
    }
}

impl ConcreteNode for CurrentTimestampNode {
    fn set_actual_handles(&mut self, input_handles: HashSet<String>, output_handles: HashSet<String>) -> Result<(), NodeError> {
        if !input_handles.is_empty() {
            return Err(NodeError::HandleValidationError(
                "CurrentTimestampNode must not have input handles".to_string(),
            ));
        }
        if output_handles.len() > 1 {
            return Err(NodeError::HandleValidationError(
                "CurrentTimestampNode must not have more than one output handle".to_string(),
            ));
        }
        Ok(())
    }

    fn generates_reading(&self) -> bool {
        false
    }

    async fn compute_objects(&self, _inputs: &GraphPayloadObjects) -> NodeResult {
        Ok(GraphPayload::Objects(
            vec![(self.default_output_handle(), serde_json::Value::Number(chrono::Utc::now().timestamp().into()))]
                .into_iter()
                .collect(),
        ))
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod key_node_tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn deserialise() {
        let node = r#""#;
        let node = serde_json::from_str::<CurrentTimestampNode>(node);
        assert!(node.is_ok());
    }

    #[test]
    fn deserialise_invalid() {
        let node = r#"[1,2,3]"#;
        let node = serde_json::from_str::<CurrentTimestampNode>(node);
        assert!(node.is_err());
    }

    #[test]
    fn generates_reading() {
        let node = CurrentTimestampNode {};
        assert!(!node.generates_reading());
    }

    #[test]
    fn set_actual_handles() {
        let mut node = CurrentTimestampNode {};
        let input_handles = HashSet::new();
        let mut output_handles = HashSet::new();
        output_handles.insert("output".to_string());
        let result = node.set_actual_handles(input_handles, output_handles);
        assert!(result.is_ok());

        let mut node = CurrentTimestampNode {};
        let mut input_handles = HashSet::new();
        input_handles.insert("input".to_string());
        let result = node.set_actual_handles(input_handles, HashSet::new());
        assert!(matches!(result, Err(NodeError::HandleValidationError(_))));

        let mut node = CurrentTimestampNode {};
        let mut output_handles = HashSet::new();
        output_handles.insert("output".to_string());
        output_handles.insert("output2".to_string());
        let result = node.set_actual_handles(HashSet::new(), output_handles);
        assert!(matches!(result, Err(NodeError::HandleValidationError(_))));
    }

    #[tokio::test]
    async fn compute_objects() {
        let node = CurrentTimestampNode {};
        let result = node.compute_objects(&HashMap::<String, serde_json::Value>::new()).await;
        assert!(result.is_ok());
        let result = result.unwrap();
        let payload = match result {
            GraphPayload::Objects(objects) => objects,
            _ => panic!("Expected GraphPayload::Objects"),
        };
        assert_eq!(payload.len(), 1);
        let object = payload.get(&node.default_output_handle()).unwrap();
        assert!(object.is_number());

        // sleep
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        
        let result2 = node.compute_objects(&HashMap::<String, serde_json::Value>::new()).await;
        assert!(result2.is_ok());
        let result2 = result2.unwrap();
        let payload2 = match result2 {
            GraphPayload::Objects(objects) => objects,
            _ => panic!("Expected GraphPayload::Objects"),
        };
        assert_eq!(payload2.len(), 1);
        let object2 = payload2.get(&node.default_output_handle()).unwrap();
        assert!(object2.is_number());
        assert_ne!(object, object2);
    }
}
