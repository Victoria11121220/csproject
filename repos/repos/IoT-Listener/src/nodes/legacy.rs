use crate::{
    graph::{ concrete_node::ConcreteNode, node::NodeError, types::GraphPayload },
    readings::{ iot::IoTReading, legacy::LegacyReading, Readings },
};
use serde::Deserialize;

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct LegacyNode {
    base_url: String,
    topic: String,
    client_account: String,
    site_id: String,
    location: String,
    sensor_type: String,
    username: String,
    api_key: String,
}

impl ConcreteNode for LegacyNode {
    async fn compute_objects(
        &self,
        inputs: &crate::graph::types::GraphPayloadObjects
    ) -> crate::graph::node::NodeResult {
        // Disconnected node
        if inputs.is_empty() {
            return Err(NodeError::GenericComputationError("No input data".to_string()));
        }

        let value = inputs.iter().next().unwrap().1;
        let reading = serde_json
            ::from_value::<IoTReading>(value.clone())
            .map_err(|e| NodeError::GenericComputationError(e.to_string()))?;

        Ok(GraphPayload::Objects(
            vec![(self.default_output_handle(), serde_json::to_value(reading).unwrap())]
                .into_iter()
                .collect()
        ))
    }

    fn payload_objects_to_reading(&self, _node_id: &str, objects: &crate::graph::types::GraphPayloadObjects) -> Result<Readings,NodeError> {
        // Check if has handle
        let value = objects.get(&self.default_output_handle());
        if value.is_none() {
            return Err(NodeError::GenericComputationError("No reading found".to_string()));
        }
        let reading: IoTReading = serde_json::from_value(value.unwrap().clone()).map_err(|e| NodeError::GenericComputationError(format!("Unable to verify reading: {}", e)))?;

        let legacy_reading = LegacyReading::new(
            reading,
            self.base_url.clone(),
            self.topic.clone(),
            self.client_account.clone(),
            self.site_id.clone(),
            self.location.clone(),
            self.sensor_type.clone(),
            self.username.clone(),
            self.api_key.clone()
        );

        Ok(Readings::Legacy(legacy_reading))
    }

    fn set_actual_handles(
        &mut self,
        input_handles: std::collections::HashSet<String>,
        _output_handles: std::collections::HashSet<String>
    ) -> Result<(), crate::graph::node::NodeError> {
        if input_handles.len() != 1 {
            return Err(
                NodeError::HandleValidationError("Invalid number of input handles".to_string())
            );
        }
        Ok(())
    }

    fn generates_reading(&self) -> bool {
        true
    }
}

// #[cfg(test)]
// mod legacy_tests {
//     use crate::graph::node::ActiveNode;

//     fn create_legacy_node() -> super::LegacyNode {
//         super::LegacyNode {
//             base_url: "http://localhost:8080".to_string(),
//             topic: "test".to_string(),
//             client_account: 1,
//             site_id: "test".to_string(),
//             location: "test".to_string(),
//             sensor_type: "test".to_string(),
//             username: "test".to_string(),
//             api_key: "test".to_string(),
//         }
//     }

//     fn deserialise(value: &str) -> super::LegacyNode {
//         serde_json::from_str(value).unwrap()
//     }

//     #[test]
//     fn deserialise_valid() {
//         let value = r#"{
//             "baseUrl": "http://localhost:8080",
//             "topic": "test",
//             "clientAccount": 1,
//             "siteId": "test",
//             "location": "test",
//             "sensorType": "test",
//             "username": "test",
//             "apiKey": "test"
//         }"#;
//         let legacy_node = deserialise(value);
//         assert_eq!(legacy_node.base_url, "http://localhost:8080");
//         assert_eq!(legacy_node.topic, "test");
//         assert_eq!(legacy_node.client_account, 1);
//         assert_eq!(legacy_node.site_id, "test");
//         assert_eq!(legacy_node.location, "test");
//         assert_eq!(legacy_node.sensor_type, "test");
//         assert_eq!(legacy_node.username, "test");
//         assert_eq!(legacy_node.api_key, "test");
//     }

//     #[test]
//     #[should_panic]
//     fn deserialise_invalid() {
//         let _ = deserialise(r#"{"not_endpoint": "http://localhost:8080"}"#);
//     }

//     #[tokio::test]
//     async fn no_input() {
//         let legacy_node = create_legacy_node();
//         let inputs = vec![];
//         let result = legacy_node.get_data(&inputs, &[], None).await;
//         assert_eq!(result, serde_json::Value::Null);
//     }

//     #[tokio::test]
//     async fn empty_array() {
//         let inputs = vec![serde_json::Value::Array(vec![])];
//         let legacy_node = create_legacy_node();
//         let result = legacy_node.get_data(&inputs, &[], None).await;
//         assert_eq!(result, serde_json::Value::Null);
//     }

//     #[tokio::test]
//     async fn invalid_input() {
//         let inputs = vec![serde_json::Value::String("test".to_string())];
//         let legacy_node = create_legacy_node();
//         let result = legacy_node.get_data(&inputs, &[], None).await;
//         assert_eq!(result, serde_json::Value::Null);
//     }

//     #[tokio::test]
//     async fn valid_reading() {
//         let inputs = vec![serde_json::json!(
//             {
//                 "identifier": "test",
//                 "measuring": "test",
//                 "value": "1",
//                 "value_type": "NUMBER",
//                 "unit": "DEGREES_CELCIUS",
//                 "timestamp": "2021-01-01T00:00:00Z",
//                 "metadata": {}
//             }
//         )];
//         let legacy_node = create_legacy_node();
//         let result = legacy_node.get_data(&inputs, &[], None).await;
//         assert_ne!(result, serde_json::Value::Null);
//     }
// }