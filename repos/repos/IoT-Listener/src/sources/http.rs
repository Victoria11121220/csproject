use super::traits::{async_trait::AsyncSource, SubscriptionResult};
use crate::{graph::types::NodeData, metrics::SOURCE_MESSAGE_COUNT, nodes::sources::SourceConfig};
use petgraph::graph::NodeIndex;
use serde::Deserialize;
use std::{
    sync::{Arc, RwLock},
    time::Duration,
};
use tracing::info;
use reqwest::Client;

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct HTTPSource {
    pub endpoint: String,
    pub method: String,
    pub interval: u64,
    #[serde(skip)]
    data: Arc<RwLock<NodeData>>,
}

impl AsyncSource for HTTPSource {
    fn get_lock(&self) -> Arc<RwLock<NodeData>> {
        Arc::clone(&self.data)
    }

    fn subscribe(
        &self,
        node_index: NodeIndex,
        cancellation_token: &tokio_util::sync::CancellationToken,
        transmitter: tokio::sync::mpsc::UnboundedSender<Vec<NodeIndex>>,
        config: &crate::sources::SourceConfig,
    ) -> SubscriptionResult {
        let _http_config = match config {
            SourceConfig::Http(config) => config,
            _ => {
                return Err("Invalid source config".into());
            }
        };

        let endpoint = self.endpoint.clone();
        let method = self.method.clone();
        let interval = self.interval;
        
        let cancellation_token = cancellation_token.clone();
        let data_pointer = self.get_lock();
        let client = Client::new();

        let handle = tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = cancellation_token.cancelled() => {
                        break;
                    },
                    _ = tokio::time::sleep(Duration::from_secs(interval)) => {
                        // Make HTTP request
                        let result = match method.as_str() {
                            "GET" => client.get(&endpoint).send().await,
                            "POST" => client.post(&endpoint).send().await,
                            _ => {
                                tracing::error!("Unsupported HTTP method: {}", method);
                                continue;
                            }
                        };

                        match result {
                            Ok(response) => {
                                if response.status().is_success() {
                                    SOURCE_MESSAGE_COUNT.with_label_values(&[&endpoint]).inc();
                                    match response.text().await {
                                        Ok(text) => {
                                            let value: Result<serde_json::Value, _> = serde_json::from_str(&text);
                                            info!("Received HTTP response: {:?}", value);
                                            if let Ok(json_value) = value {
                                                // Set the data
                                                *data_pointer.write().unwrap() = NodeData::Object(json_value);
                                                if let Err(e) = transmitter.send([node_index].to_vec()) {
                                                    tracing::error!("Failed to send message to transmitter: {:?}", e);
                                                }
                                            }
                                        }
                                        Err(e) => {
                                            tracing::error!("Failed to parse HTTP response as JSON: {:?}", e);
                                        }
                                    }
                                } else {
                                    tracing::error!("HTTP request failed with status: {}", response.status());
                                }
                            }
                            Err(e) => {
                                tracing::error!("HTTP request failed: {:?}", e);
                            }
                        }
                    }
                }
            }
        });

        Ok(handle)
    }
}

impl HTTPSource {
    /// Get the current data stored in the HTTP source
    pub fn get_data(&self) -> Result<NodeData, Box<dyn std::error::Error>> {
        let data = self.data.read().map_err(|_| "Failed to read data")?;
        Ok(data.clone())
    }
    
    /// Update the data stored in the HTTP source
    pub fn update_data(&self, new_data: NodeData) -> Result<(), Box<dyn std::error::Error>> {
        let mut data = self.data.write().map_err(|_| "Failed to write data")?;
        *data = new_data;
        Ok(())
    }
}