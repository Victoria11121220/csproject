use crate::entities::sea_orm_active_enums::{IotFieldType, IotUnit};
use crate::graph::node::{NodeError, NodeResult};
use crate::graph::concrete_node::ConcreteNode;
use crate::graph::types::{GraphPayload, GraphPayloadObjects};
use crate::readings::iot::IoTReading;
use crate::readings::Readings;
use chrono::{DateTime, NaiveDateTime, Utc};
use serde::Deserialize;
use std::collections::HashSet;

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SensorNode {
    identifier: String,
    measuring: String,
    #[serde(rename = "value")]
    value_type: IotFieldType,
    unit: IotUnit,
    #[serde(rename = "timestamp")]
    timestamp_format: String,
}

impl ConcreteNode for SensorNode {
    fn set_actual_handles(&mut self, input_handles: HashSet<String>, output_handles: HashSet<String>) -> Result<(), NodeError> {
        let required_handles: HashSet<String> = vec!["value".to_string(), "timestamp".to_string()].into_iter().collect();

        if !required_handles.is_subset(&input_handles) {
            let missing_handles = required_handles.difference(&input_handles).cloned().collect::<Vec<String>>();
            return Err(NodeError::HandleValidationError(format!(
                "Missing required input handles: {:?}",
                missing_handles
            )));
        }

        if output_handles.len() > 1 {
            return Err(NodeError::HandleValidationError("Too many output handles".to_string()));
        }

        Ok(())
    }

    fn generates_reading(&self) -> bool {
        true
    }

    async fn compute_objects(&self, inputs: &GraphPayloadObjects) -> NodeResult {
        let mut timestamp: Option<DateTime<Utc>> = None;
        let mut measuring = self.measuring.clone();
        let mut identifier = self.identifier.clone();
        let mut metadata: Option<serde_json::Map<String, serde_json::Value>> = None;
        let mut value: Option<serde_json::Value> = None;

        for (handle, data) in inputs {
            match handle.as_str() {
                "timestamp" => {
                    timestamp = match data {
                        serde_json::Value::String(string) => match NaiveDateTime::parse_from_str(string, &self.timestamp_format) {
                            Err(e) => return Err(NodeError::GenericComputationError(format!("Could not parse timestamp: {}", e))),
                            Ok(formatted_timestamp) => Some(DateTime::<Utc>::from_naive_utc_and_offset(formatted_timestamp, Utc)),
                        },
                        serde_json::Value::Number(number) => match number.as_i64() {
                            None => {
                                return Err(NodeError::GenericComputationError(format!(
                                    "Could not parse timestamp as number: {}",
                                    number
                                )))
                            }
                            Some(timestamp_number) => match DateTime::from_timestamp(timestamp_number, 0) {
                                None => {
                                    return Err(NodeError::GenericComputationError(format!(
                                        "Could not parse timestamp as number: {}",
                                        number
                                    )))
                                }
                                Some(timestamp) => Some(timestamp),
                            },
                        },
                        _ => return Err(NodeError::GenericComputationError(format!("Invalid timestamp format: {:?}", data))),
                    }
                }
                "measuring" => {
                    // Optional
                    measuring = match data.as_str() {
                        None => return Err(NodeError::GenericComputationError(format!("Invalid measuring format: {:?}", data))),
                        Some(measuring) => measuring.to_string(),
                    }
                }
                "identifier" => {
                    // Optional
                    identifier = match data.as_str() {
                        None => return Err(NodeError::GenericComputationError(format!("Invalid identifier format: {:?}", data))),
                        Some(identifier) => identifier.to_string(),
                    }
                }
                "metadata" => {
                    // Optional
                    metadata = match data {
                        serde_json::Value::Object(map) => Some(map.clone()),
                        _ => return Err(NodeError::GenericComputationError(format!("Invalid metadata format: {:?}", data))),
                    }
                }
                "value" => {
                    let cast_attempt: Option<String> = match self.value_type {
                        IotFieldType::Number => data.as_f64().map(|f| f.to_string()),
                        IotFieldType::String => data.as_str().map(|s| s.to_string()),
                        IotFieldType::Boolean => data.as_bool().map(|b| b.to_string()),
                    };
                    match cast_attempt {
                        None => return Err(NodeError::GenericComputationError(format!("Invalid value format: {:?}", data))),
                        Some(cast) => value = Some(serde_json::Value::String(cast)),
                    }
                }
                _ => {
                    return Err(NodeError::HandleValidationError(format!("Invalid handle: {}", handle)));
                }
            }
        }

        if timestamp.is_none() {
            return Err(NodeError::GenericComputationError("Missing timestamp".to_string()));
        }

        if value.is_none() {
            return Err(NodeError::GenericComputationError("Missing value".to_string()));
        }

        let mut sensor_reading = serde_json::json!({
            "identifier": identifier,
            "measuring": measuring,
            "value": value.unwrap(),
            "value_type": serde_json::to_value(self.value_type.clone()).unwrap(),
            "unit": serde_json::to_value(self.unit.clone()).unwrap(),
            "timestamp": serde_json::Value::String(timestamp.unwrap().to_string()),
        });

        if let Some(metadata) = metadata {
            sensor_reading
                .as_object_mut()
                .unwrap()
                .insert("metadata".to_string(), serde_json::Value::Object(metadata));
        }

        Ok(GraphPayload::Objects(
            vec![(self.default_output_handle(), sensor_reading)].into_iter().collect(),
        ))
    }

    fn payload_objects_to_reading(&self, _: &str, objects: &GraphPayloadObjects) -> Result<Readings, NodeError> {
        let reading = objects.get(&self.default_output_handle());
        if reading.is_none() {
            return Err(NodeError::GenericComputationError("Missing reading".to_string()));
        }

        let reading = reading.unwrap();
        serde_json::from_value::<IoTReading>(reading.clone())
            .map_err(|e| NodeError::GenericComputationError(format!("Could not deserialize reading: {}", e)))
            .map(Readings::Iot)
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod sensor_tests {
    use crate::graph::types::NodeData;

    use super::*;

    fn get_sensor_node(value_type: IotFieldType) -> SensorNode {
        SensorNode {
            identifier: "test".to_string(),
            measuring: "test".to_string(),
            value_type,
            unit: IotUnit::DegreesCelcius,
            timestamp_format: "%Y-%m-%dT%H:%M:%S%.3fZ".to_string(),
        }
    }
    
    #[test]
    fn deserialise_valid() {
        let valid = [
            r#"{"identifier":"test","measuring":"test","value":"NUMBER","unit":"DEGREES_CELCIUS","timestamp":"%Y-%m-%dT%H:%M:%S%.3fZ"}"#,
            r#"{"identifier":"sensor123","measuring":"test","value":"STRING","unit":"AMOUNT","timestamp":"%+"}"#,
        ];
        for v in valid {
            let node = serde_json::from_str::<SensorNode>(v);
            assert!(node.is_ok());
        }
    }

    #[test]
    fn deserialise_invalid() {
        let invalid = [
            r#"{"measuring":"test","value":"NUMBER","unit":"DEGREES_CELCIUS","timestamp":"%Y-%m-%dT%H:%M:%S%.3fZ"}"#,
            r#"{"identifier":"test","value":"NUMVER","unit":"DEGREES_CELCIUS","timestamp":"%Y-%m-%dT%H:%M:%S%.3fZ"}"#,
            r#"{"identifier":"test","measuring":"test","unit":"DEGREES_CELCIUS","timestamp":"%Y-%m-%dT%H:%M:%S%.3fZ"}"#,
            r#"{"identifier":"test","measuring":"test","value":"NUMBER","timestamp":"%Y-%m-%dT%H:%M:%S%.3fZ"}"#,
            r#"{"identifier":"test","measuring":"test","value":"NUMBER","unit":"DEGREES_CELCIUS"}"#,
        ];
        for i in invalid {
            let node = serde_json::from_str::<SensorNode>(i);
            assert!(node.is_err());
        }
    }

    #[test]
    fn generates_reading() {
        let node = get_sensor_node(IotFieldType::Number);
        assert!(node.generates_reading());
    }

    #[test]
    fn set_actual_handles() {
        let mut node = get_sensor_node(IotFieldType::Number);
        let mut input_handles = HashSet::new();
        input_handles.insert("timestamp".to_string());
        input_handles.insert("value".to_string());
        let mut output_handles = HashSet::new();
        output_handles.insert("output".to_string());
        let result = node.set_actual_handles(input_handles, output_handles);
        assert!(matches!(result, Ok(())));
    }

    #[test]
    fn set_actual_handles_invalid() {
        let mut node = get_sensor_node(IotFieldType::Number);
        let mut input_handles = HashSet::new();
        input_handles.insert("timestamp".to_string());
        let mut output_handles = HashSet::new();
        output_handles.insert("output".to_string());
        let result = node.set_actual_handles(input_handles.clone(), output_handles.clone());
        assert!(matches!(result, Err(NodeError::HandleValidationError(_))));

        let mut node = get_sensor_node(IotFieldType::Number);
        let mut input_handles = HashSet::new();
        input_handles.insert("timestamp".to_string());
        input_handles.insert("value".to_string());
        let mut output_handles = HashSet::new();
        output_handles.insert("output".to_string());
        output_handles.insert("output2".to_string());
        let result = node.set_actual_handles(input_handles.clone(), output_handles.clone());
        assert!(matches!(result, Err(NodeError::HandleValidationError(_))));
    }

    #[tokio::test]
    async fn compute_objects() {
        let node = get_sensor_node(IotFieldType::Number);
        let inputs = GraphPayloadObjects::from([
            ("timestamp".to_string(), serde_json::json!("2021-01-01T00:00:00.000Z")),
            ("value".to_string(), serde_json::json!(1)),
            ("identifier".to_string(), serde_json::json!("sensor")),
            ("measuring".to_string(), serde_json::json!("temperature")),
            ("metadata".to_string(), serde_json::json!({"key": "value"})),
        ]);
        let result = node.compute_objects(&inputs).await;
        assert!(matches!(result, Ok(GraphPayload::Objects(_))));
        let result = result.unwrap().get(&node.default_output_handle()).unwrap();
        match result {
            NodeData::Object(value) => assert_eq!(value, serde_json::json!({
                "identifier": "sensor",
                "measuring": "temperature",
                "value": "1",
                "value_type": "NUMBER",
                "unit": "DEGREES_CELCIUS",
                "timestamp": "2021-01-01 00:00:00 UTC",
                "metadata": {"key": "value"},
            })),
            _ => panic!("Unexpected NodeData variant"),
        }
    }

    #[tokio::test]
    async fn compute_objects_alternative_reading_types() {
        let node = get_sensor_node(IotFieldType::String);
        let inputs = GraphPayloadObjects::from([
            ("timestamp".to_string(), serde_json::json!("2021-01-01T00:00:00.000Z")),
            ("value".to_string(), serde_json::json!("test")),
        ]);

        let result = node.compute_objects(&inputs).await;
        assert!(matches!(result, Ok(GraphPayload::Objects(_))));
        
        let inputs = GraphPayloadObjects::from([
            ("timestamp".to_string(), serde_json::json!("2021-01-01T00:00:00.000Z")),
            ("value".to_string(), serde_json::json!(true)),
        ]);
        let result = node.compute_objects(&inputs).await;
        assert!(matches!(result, Err(NodeError::GenericComputationError(_))));

        let node = get_sensor_node(IotFieldType::Boolean);
        let inputs = GraphPayloadObjects::from([
            ("timestamp".to_string(), serde_json::json!("2021-01-01T00:00:00.000Z")),
            ("value".to_string(), serde_json::json!(true)),
        ]);
        let result = node.compute_objects(&inputs).await;
        assert!(matches!(result, Ok(GraphPayload::Objects(_))));

        let node = get_sensor_node(IotFieldType::Boolean);
        let inputs = GraphPayloadObjects::from([
            ("timestamp".to_string(), serde_json::json!("2021-01-01T00:00:00.000Z")),
            ("value".to_string(), serde_json::json!(1)),
        ]);
        let result = node.compute_objects(&inputs).await;
        assert!(matches!(result, Err(NodeError::GenericComputationError(_))));
    }

    #[tokio::test]
    async fn compute_objects_null_value() {
        let node = get_sensor_node(IotFieldType::Number);
        let inputs = GraphPayloadObjects::from([
            ("timestamp".to_string(), serde_json::json!("2021-01-01T00:00:00.000Z")),
            ("value".to_string(), serde_json::json!(null)),
        ]);
        let result = node.compute_objects(&inputs).await;
        assert!(matches!(result, Err(NodeError::GenericComputationError(_))));
    }

    #[tokio::test]
    async fn compute_objects_invalid_timestamp() {
        let node = get_sensor_node(IotFieldType::Number);
        let inputs = GraphPayloadObjects::from([
            ("value".to_string(), serde_json::json!(1)),
        ]);
        let result = node.compute_objects(&inputs).await;
        assert!(matches!(result, Err(NodeError::GenericComputationError(_))));

        let inputs = GraphPayloadObjects::from([
            ("timestamp".to_string(), serde_json::json!("mslkfdn")),
            ("value".to_string(), serde_json::json!(1)),
        ]);
        let result = node.compute_objects(&inputs).await;
        assert!(matches!(result, Err(NodeError::GenericComputationError(_))));

        let inputs = GraphPayloadObjects::from([
            ("timestamp".to_string(), serde_json::json!(true)),
            ("value".to_string(), serde_json::json!(1)),
        ]);
        let result = node.compute_objects(&inputs).await;
        assert!(matches!(result, Err(NodeError::GenericComputationError(_))));
    }

    #[tokio::test]
    async fn compute_objects_numeric_timestamp() {
        let node = get_sensor_node(IotFieldType::Number);
        let inputs = GraphPayloadObjects::from([
            ("timestamp".to_string(), serde_json::json!(1609459200)),
            ("value".to_string(), serde_json::json!(1)),
        ]);
        let result = node.compute_objects(&inputs).await.unwrap().get(&node.default_output_handle()).unwrap();
        match result {
            NodeData::Object(value) => assert_eq!(value, serde_json::json!({
                "identifier": "test",
                "measuring": "test",
                "value": "1",
                "value_type": "NUMBER",
                "unit": "DEGREES_CELCIUS",
                "timestamp": "2021-01-01 00:00:00 UTC",
            })),
            _ => panic!("Unexpected NodeData variant"),
        }

        let inputs = GraphPayloadObjects::from([
            ("timestamp".to_string(), serde_json::json!(1609459200.0)),
            ("value".to_string(), serde_json::json!(1)),
        ]);
        let result = node.compute_objects(&inputs).await;
        assert!(matches!(result, Err(NodeError::GenericComputationError(_))));

        let inputs = GraphPayloadObjects::from([
            ("timestamp".to_string(), serde_json::json!(1609459200.0)),
            ("value".to_string(), serde_json::json!(-1)),
        ]);
        let result = node.compute_objects(&inputs).await;
        assert!(matches!(result, Err(NodeError::GenericComputationError(_))));
    }

    #[tokio::test]
    async fn compute_objects_invalid_types() {
        let node = get_sensor_node(IotFieldType::Boolean);
        let inputs = GraphPayloadObjects::from([
            ("timestamp".to_string(), serde_json::json!("2021-01-01T00:00:00.000Z")),
            ("value".to_string(), serde_json::json!(true)),
            ("measuring".to_string(), serde_json::json!(1)),
        ]);
        let result = node.compute_objects(&inputs).await;
        assert!(matches!(result, Err(NodeError::GenericComputationError(_))));

        let inputs = GraphPayloadObjects::from([
            ("timestamp".to_string(), serde_json::json!("2021-01-01T00:00:00.000Z")),
            ("value".to_string(), serde_json::json!(true)),
            ("identifier".to_string(), serde_json::json!(1)),
        ]);
        let result = node.compute_objects(&inputs).await;
        assert!(matches!(result, Err(NodeError::GenericComputationError(_))));

        let inputs = GraphPayloadObjects::from([
            ("timestamp".to_string(), serde_json::json!("2021-01-01T00:00:00.000Z")),
            ("value".to_string(), serde_json::json!(true)),
            ("metadata".to_string(), serde_json::json!(1)),
        ]);
        let result = node.compute_objects(&inputs).await;
        assert!(matches!(result, Err(NodeError::GenericComputationError(_))));
    }

    #[tokio::test]
    async fn compute_objects_invalid_handle() {
        let node = get_sensor_node(IotFieldType::Number);
        let inputs = GraphPayloadObjects::from([
            ("timestamp".to_string(), serde_json::json!("2021-01-01T00:00:00.000Z")),
            ("value".to_string(), serde_json::json!(1)),
            ("invalid".to_string(), serde_json::json!(1)),
        ]);
        let result = node.compute_objects(&inputs).await;
        assert!(matches!(result, Err(NodeError::HandleValidationError(_))));
    }

    #[tokio::test]
    async fn compute_objects_no_value() {
        let node = get_sensor_node(IotFieldType::Number);
        let inputs = GraphPayloadObjects::from([
            ("timestamp".to_string(), serde_json::json!("2021-01-01T00:00:00.000Z")),
        ]);
        let result = node.compute_objects(&inputs).await;
        assert!(matches!(result, Err(NodeError::GenericComputationError(_))));
    }

    #[tokio::test]
    async fn payload_objects_to_readings() {
        let node = get_sensor_node(IotFieldType::Number);
        let objects = GraphPayloadObjects::from([
            ("source".to_string(), serde_json::json!({
                "identifier": "sensor",
                "measuring": "temperature",
                "value": "1",
                "value_type": "NUMBER",
                "unit": "DEGREES_CELCIUS",
                "timestamp": "2021-01-01 00:00:00 UTC",
                "metadata": {"key": "value"},
            })),
        ]);
        let result = node.payload_objects_to_reading("output", &objects);
        println!("{:?}", result);
        assert!(matches!(result, Ok(Readings::Iot(_))));

        let objects = GraphPayloadObjects::new();
        let result = node.payload_objects_to_reading("output", &objects);
        assert!(matches!(result, Err(NodeError::GenericComputationError(_))));

        let objects = GraphPayloadObjects::from([
            ("source".to_string(), serde_json::json!("1212")),
        ]);
        let result = node.payload_objects_to_reading("output", &objects);
        assert!(matches!(result, Err(NodeError::GenericComputationError(_))));
    }
}