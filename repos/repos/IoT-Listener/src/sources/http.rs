use super::traits::{async_trait::AsyncSource, rest::RestSource, SubscriptionResult, ThreadSafeError};
use crate::{graph::types::NodeData, nodes::sources::SourceConfig};
use petgraph::graph::NodeIndex;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, RwLock};

#[derive(Debug, Deserialize, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HTTPSource {
    endpoint: String,
    method: String,
    headers: Option<std::collections::HashMap<String, String>>,
    body: Option<serde_json::Value>,
    #[serde(skip)]
    data: Arc<RwLock<NodeData>>,
}

impl HTTPSource {
    pub fn new(
        endpoint: String,
        method: String,
        headers: Option<std::collections::HashMap<String, String>>,
        body: Option<serde_json::Value>,
    ) -> Self {
        Self {
            endpoint,
            method,
            headers,
            body,
            data: Arc::new(RwLock::new(NodeData::Object(serde_json::Value::Null))),
        }
    }
    
    pub fn endpoint(&self) -> &str {
        &self.endpoint
    }
}

impl RestSource for HTTPSource {
    fn interval(&self, config: &SourceConfig) -> Result<u64, ThreadSafeError> {
        match config {
            SourceConfig::Http(config) => Ok(config.interval()),
            _ => Err("Invalid source config".into()),
        }
    }

    fn get_request(
        &self,
        _config: &SourceConfig,
    ) -> Result<reqwest::Request, Box<dyn std::error::Error>> {
        let client = reqwest::Client::new();
        
        // Parse the HTTP method
        let method: reqwest::Method = self.method.parse()?;
        
        // Build the request
        let mut request_builder = client.request(method, &self.endpoint);
        
        // Add headers if provided
        if let Some(headers) = &self.headers {
            for (key, value) in headers {
                request_builder = request_builder.header(key, value);
            }
        }
        
        // Add body if provided
        if let Some(body) = &self.body {
            request_builder = request_builder.json(body);
        }
        
        let request = request_builder.build()?;
        Ok(request)
    }
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
        config: &SourceConfig,
    ) -> SubscriptionResult {
        RestSource::subscribe(self, node_index, cancellation_token, transmitter, config)
    }
}