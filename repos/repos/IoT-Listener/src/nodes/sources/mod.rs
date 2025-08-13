pub mod http;
pub mod mas_monitor;
pub mod mqtt;
pub mod kafka;

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

#[derive(Deserialize, Debug, Clone)]
pub enum SourceConfig {
    Http(http::HTTPSourceConfig),
    Mqtt(mqtt::MQTTSourceConfig),
    MASMonitor(mas_monitor::MASMonitorSourceConfig),
    Kafka(kafka::KafkaSourceConfig),
}

#[derive(Debug, Clone)]
pub struct SourceNode {
    source: Source,
    config: SourceConfig,
}

impl SourceNode {
    pub fn source(&self) -> &Source {
        &self.source
    }

    pub fn config(&self) -> &SourceConfig {
        &self.config
    }
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
            Source::Kafka(_) => serde_json::from_value(raw.config).map(SourceConfig::Kafka).map_err(de::Error::custom)?,
        };

        Ok(SourceNode { source: raw.source, config })
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
        // Read the last value from the source
        let data = self.source().get().map_err(|e| NodeError::SourceError(e.to_string()))?;
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
            Source::Kafka(_) => {
                match data {
                    /*
                        {
                            "payload": {
                                "temperature": 25.0,
                                "humidity": 50.0
                            },
                            "topic": "sensor-data",
                            "partition": 0,
                            "offset": 12345
                        }
                    */
                    NodeData::Object(value) => Ok(GraphPayload::Objects(
                        vec![
                            (self.default_output_handle(), value["payload"].clone()),
                            ("topic".to_string(), value["topic"].clone()),
                            ("partition".to_string(), value["partition"].clone()),
                            ("offset".to_string(), value["offset"].clone()),
                        ]
                        .into_iter()
                        .collect(),
                    )),
                    NodeData::Collection(values) => Ok(GraphPayload::Collections(
                        vec![
                            (self.default_output_handle(), values.iter().map(|v| v["payload"].clone()).collect()),
                            ("topic".to_string(), values.iter().map(|v| v["topic"].clone()).collect()),
                            ("partition".to_string(), values.iter().map(|v| v["partition"].clone()).collect()),
                            ("offset".to_string(), values.iter().map(|v| v["offset"].clone()).collect()),
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