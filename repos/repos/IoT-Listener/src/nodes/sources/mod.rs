pub mod http;
pub mod mas_monitor;
pub mod mqtt;

use crate::sources::traits::async_trait::AsyncSource;
use crate::{
    graph::{
        concrete_node::ConcreteNode,
        node::{NodeError, NodeResult},
        types::{GraphPayload, GraphPayloadObjects, NodeData},
    },
    sources::Source,
};
use serde::{de, Deserialize, Deserializer};
use std::collections::HashSet;
use std::sync::RwLock;

#[derive(Deserialize, Debug, Clone)]
pub enum SourceConfig {
    Http(http::HTTPSourceConfig),
    Mqtt(mqtt::MQTTSourceConfig),
    MASMonitor(mas_monitor::MASMonitorSourceConfig),
}

#[derive(Debug, Clone)]
pub struct SourceNode {
    source: Source,
    config: SourceConfig,
    // Pre-filled data for use in processor
    prefilled_data: std::sync::Arc<RwLock<Option<NodeData>>>,
}

// Custom deserialization for SourceNode
impl<'de> Deserialize<'de> for SourceNode {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct RawSourceNode {
            source: Source,
            config: serde_json::Value, // Config remains as raw JSON initially
        }

        // Deserialize into a temporary struct
        let raw = RawSourceNode::deserialize(deserializer)?;

        // Match on the source type to deserialize `config` appropriately
        let config = match raw.source {
            Source::MASMonitor(_) => serde_json::from_value(raw.config)
                .map(SourceConfig::MASMonitor)
                .map_err(de::Error::custom)?,
            Source::Mqtt(_) => serde_json::from_value(raw.config).map(SourceConfig::Mqtt).map_err(de::Error::custom)?,
            Source::Http(_) => serde_json::from_value(raw.config).map(SourceConfig::Http).map_err(de::Error::custom)?,
        };

        Ok(SourceNode { 
            source: raw.source, 
            config,
            prefilled_data: std::sync::Arc::new(RwLock::new(None)),
        })
    }
}

impl SourceNode {
    pub fn source(&self) -> &Source {
        &self.source
    }

    pub fn config(&self) -> &SourceConfig {
        &self.config
    }
    
    /// Get the current data stored in the source
    pub fn get_source_data(&self) -> Result<NodeData, Box<dyn std::error::Error>> {
        self.source.get()
    }
    
    /// Update the data stored in the source
    pub fn update_source_data(&self, new_data: NodeData) -> Result<(), Box<dyn std::error::Error>> {
        match &self.source {
            Source::Mqtt(mqtt_source) => mqtt_source.update_data(new_data),
            // Add other source types as needed
            _ => Err("Source type does not support data updates".into()),
        }
    }
    
    /// Set prefilled data for use in processor
    pub fn set_prefilled_data(&self, data: NodeData) -> Result<(), NodeError> {
        let mut prefilled_data = self.prefilled_data.write().map_err(|_| NodeError::GenericComputationError("Failed to write prefilled data".to_string()))?;
        *prefilled_data = Some(data);
        Ok(())
    }
    
    /// Clear prefilled data
    pub fn clear_prefilled_data(&self) -> Result<(), NodeError> {
        let mut prefilled_data = self.prefilled_data.write().map_err(|_| NodeError::GenericComputationError("Failed to write prefilled data".to_string()))?;
        *prefilled_data = None;
        Ok(())
    }
    
    /// Get prefilled data if available
    fn get_prefilled_data(&self) -> Result<Option<NodeData>, NodeError> {
        let prefilled_data = self.prefilled_data.read().map_err(|_| NodeError::GenericComputationError("Failed to read prefilled data".to_string()))?;
        Ok(prefilled_data.clone())
    }
}

impl ConcreteNode for SourceNode {
    fn set_actual_handles(&mut self, input_handles: HashSet<String>, _: HashSet<String>) -> Result<(), NodeError> {
        if !input_handles.is_empty() {
            return Err(NodeError::HandleValidationError(
                "Source node should not have any input handles".to_string(),
            ));
        }

        Ok(())
    }

    fn generates_reading(&self) -> bool {
        false
    }

    async fn compute_objects(&self, _: &GraphPayloadObjects) -> NodeResult {
        // Check if we have prefilled data (used in processor)
        let data = if let Ok(Some(prefilled_data)) = self.get_prefilled_data() {
            prefilled_data
        } else {
            // Read the last value from the source (used in collector)
            self.source().get().map_err(|e| NodeError::SourceError(e.to_string()))?
        };
        
        match self.source {
            Source::Mqtt(_) => {
                match data {
                    /*
                        {
                            "payload": {
                                "temperature": 25.0,
                                "humidity": 50.0
                            },
                            "topic": "sensor/1"
                        }
                    */
                    NodeData::Object(value) => Ok(GraphPayload::Objects(
                        vec![
                            (self.default_output_handle(), value["payload"].clone()),
                            ("topic".to_string(), value["topic"].clone()),
                        ]
                        .into_iter()
                        .collect(),
                    )),

                    /*
                        [
                            {
                                "payload": {
                                    "temperature": 25.0,
                                    "humidity": 50.0
                                },
                                "topic": "sensor/1"
                            },
                            {
                                "payload": {
                                    "temperature": 26.0,
                                    "humidity": 51.0
                                },
                                "topic": "sensor/2"
                            }
                        ]
                        ->
                        {
                            "source": [
                                {
                                    "temperature": 25.0,
                                    "humidity": 50.0
                                },
                                {
                                    "temperature": 26.0,
                                    "humidity": 51.0
                                }
                            ],
                            "topic": [
                                "sensor/1",
                                "sensor/2"
                            ]
                        }
                    */
                    NodeData::Collection(values) => Ok(GraphPayload::Collections(
                        vec![
                            (self.default_output_handle(), values.iter().map(|v| v["payload"].clone()).collect()),
                            ("topic".to_string(), values.iter().map(|v| v["topic"].clone()).collect()),
                        ]
                        .into_iter()
                        .collect(),
                    )),
                }
            }
            _ => match data {
                NodeData::Object(value) => Ok(GraphPayload::Objects(vec![(self.default_output_handle(), value)].into_iter().collect())),
                NodeData::Collection(values) => Ok(GraphPayload::Collections(
                    vec![(self.default_output_handle(), values)].into_iter().collect(),
                )),
            },
        }
    }
}