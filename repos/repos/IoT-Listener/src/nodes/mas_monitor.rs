use std::collections::{ HashMap, HashSet };
use serde::{ Deserialize, Deserializer, Serialize };
use crate::{
    graph::{ concrete_node::ConcreteNode, node::NodeError, types::{ GraphPayload, GraphPayloadObjects } },
    readings::{ iot::IoTReading, mas_monitor::MASMonitorReading, Readings },
    sources::mas_monitor::MASMonitorSource,
};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MASMonitorNode {
    source: MASMonitorSource,
    authentication_token: String,
    handles: Vec<String>,
    device: MASMonitorDevice,
    event_name: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct MASMonitorDevice {
    name: String,
    #[serde(rename = "ddId")]
    dd_id: String,
    #[serde(rename = "deviceTypeName")]
    device_type_name: String,
    #[serde(rename = "deviceTypeUUId")]
    device_type_id: String,
}

impl<'de> Deserialize<'de> for MASMonitorNode {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error> where D: Deserializer<'de> {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct RawMASMonitorNode {
            source: serde_json::Value,
            device: MASMonitorDevice,
            authentication_token: String,
            handles: Vec<String>,
            event_name: String,
        }

        let raw = RawMASMonitorNode::deserialize(deserializer)?;
        let source = match raw.source.is_object() {
            false => None,
            true => {
                let config = raw.source.get("config").unwrap_or(&serde_json::Value::Null);
                let config = serde_json::from_value::<MASMonitorSource>(config.clone());
                match config {
                    Ok(config) => Some(config),
                    Err(_) => None,
                }
            }
        };

        if source.is_none() {
            return Err(serde::de::Error::custom("Invalid source config"));
        }

        Ok(MASMonitorNode {
            source: source.unwrap(),
            authentication_token: raw.authentication_token,
            device: raw.device,
            handles: raw.handles,
            event_name: raw.event_name,
        })
    }
}

impl ConcreteNode for MASMonitorNode {
    fn set_actual_handles(
        &mut self,
        input_handles: HashSet<String>,
        _output_handles: HashSet<String>
    ) -> Result<(), crate::graph::node::NodeError> {
        if input_handles.len() != self.handles.len() {
            return Err(
                crate::graph::node::NodeError::HandleValidationError(
                    "Invalid number of input handles".to_string()
                )
            );
        }
        Ok(())
    }

    fn generates_reading(&self) -> bool {
        true
    }

    async fn compute_objects(
        &self,
        inputs: &GraphPayloadObjects
    ) -> crate::graph::node::NodeResult {
        // Check if empty
        if inputs.is_empty() {
            return Err(
                crate::graph::node::NodeError::HandleValidationError(
                    "No inputs provided".to_string()
                )
            );
        }

        // Check if input handles match
        if inputs.len() != self.handles.len() {
            return Err(
                crate::graph::node::NodeError::HandleValidationError(
                    "Invalid number of input handles".to_string()
                )
            );
        }

        // Create hashmap of input_handle to reading values
        let input_map: HashMap<String, serde_json::Value> = inputs
            .iter()
            .filter_map(|(handle, value)| {
                match serde_json::from_value::<IoTReading>(value.clone()) {
                    Ok(reading) => reading.get_value_json().map(|value| (handle.clone(), value)),
                    Err(_) => None,
                }
            })
            .collect();

        // Check if all readings are valid
        if input_map.len() != self.handles.len() {
            return Err(
                crate::graph::node::NodeError::HandleValidationError(
                    "Invalid reading input".to_string()
                )
            );
        }

        Ok(
            GraphPayload::Objects(
                vec![(
                    self.default_output_handle(),
                    serde_json::to_value(input_map).unwrap()
                )]
                    .into_iter()
                    .collect()
            )
        )
    }

    fn payload_objects_to_reading(&self,_node_id: &str, objects: &GraphPayloadObjects) -> Result<Readings, NodeError> {
        let value = objects.get(&self.default_output_handle());
        if value.is_none() {
            return Err(NodeError::GenericComputationError("Invalid object".to_string()));
        }
        let value = value.unwrap();
        // Check if an object
        if !value.is_object() {
            return Err(NodeError::GenericComputationError("Invalid object".to_string()));
        }
        // Create MAS Monitor reading
        let mas_monitor_reading = MASMonitorReading::new(
            self.source.get_messaging_url(),
            self.authentication_token.clone(),
            self.device.device_type_name.clone(),
            self.device.name.clone(),
            self.event_name.clone(),
            // Convert hashmap to serde map
            value.as_object().unwrap().clone(),
        );
        Ok(Readings::MASMonitor(mas_monitor_reading))
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod monitor_tests {
    use serde_json::json;

    use super::*;

    fn get_source() -> MASMonitorSource {
        // Deserialise
        let source_json = r#"{ "orgId": "org_id", "baseUrl": "base_url", "userEmail": "user_email", "tenantId": "tenant_id", "apiKey": "api_key", "apiToken": "api_token" }"#;
        serde_json::from_str(source_json).unwrap()
    }

    fn get_node(handles: &[String]) -> MASMonitorNode {
        MASMonitorNode {
            source: get_source(),
            authentication_token: "token".to_string(),
            handles: handles.to_vec(),
            device: MASMonitorDevice {
                name: "device".to_string(),
                dd_id: "dd_id".to_string(),
                device_type_name: "device_type".to_string(),
                device_type_id: "device_type_id".to_string(),
            },
            event_name: "event".to_string(),
        }
    }   

    fn get_reading() -> IoTReading {
        serde_json::from_value(json!({
            "identifier": "id",
            "measuring": "measuring",
            "value": "value",
            "value_type": "STRING",
            "timestamp": "2021-01-01T00:00:00Z",
            "unit": "DEGREES_CELCIUS"
        })).unwrap()
    }

    #[test]
    fn generates_reading() {
        let node = get_node(&["handle1".to_string()]);
        assert!(node.generates_reading());
    }

    #[test]
    fn set_actual_handles() {
        let mut node = get_node(&["handle1".to_string()]);
        let handles = ["handle1".to_string()].iter().cloned().collect();
        assert!(node.set_actual_handles(handles, HashSet::new()).is_ok());

        let mut node = get_node(&[]);
        let handles = ["handle1".to_string()].iter().cloned().collect();
        assert!(node.set_actual_handles(handles, HashSet::new()).is_err());

        let mut node = get_node(&["handle1".to_string()]);
        let handles = ["handle1".to_string(), "handle2".to_string()].iter().cloned().collect();
        assert!(node.set_actual_handles(handles, HashSet::new()).is_err());
    }
}