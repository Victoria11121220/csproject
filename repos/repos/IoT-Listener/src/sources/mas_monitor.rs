use super::traits::{ async_trait::AsyncSource, rest::RestSource, SubscriptionResult, ThreadSafeError };
use petgraph::graph::NodeIndex;
use serde::{ Deserialize, Serialize };
use std::sync::{ Arc, RwLock };
use crate::{ graph::types::NodeData, nodes::sources::SourceConfig };

#[derive(Debug, Deserialize, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MASMonitorSource {
    org_id: String,
    base_url: String,
    user_email: String,
    tenant_id: String,
    api_key: String,
    api_token: String,
    #[serde(skip)]
    data: Arc<RwLock<NodeData>>,
}

impl MASMonitorSource {
    pub fn get_messaging_url(&self) -> String {
        format!("https://{}.messaging.iot.{}", self.org_id, self.base_url)
    }

    pub fn get_monitor_url(&self) -> String {
        format!("https://{}.api.monitor.{}", self.org_id, self.base_url)
    }
}

impl RestSource for MASMonitorSource {
    fn interval(&self, config: &SourceConfig) -> Result<u64, ThreadSafeError> {
        match config {
            SourceConfig::MASMonitor(config) => Ok(config.interval()),
            _ => Err("Invalid source config".into()),
        }
    }

    fn get_request(
        &self,
        config: &SourceConfig
    ) -> Result<reqwest::Request, Box<dyn std::error::Error>> {
        let monitor_config = match config {
            SourceConfig::MASMonitor(config) => config,
            _ => {
                return Err("Invalid source config".into());
            }
        };

        let device_name = monitor_config.device_name().to_string();
        let url = format!(
            "{}/api/v2/datalake/data/deviceTypes/{}/raw",
            self.get_monitor_url(),
            monitor_config.device_type_id()
        );

        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert("X-api-key", self.api_key.parse().unwrap());
        headers.insert("X-api-token", self.api_token.parse().unwrap());
        headers.insert("Content-Type", "application/json".parse().unwrap());
        headers.insert("mam_user_email", self.user_email.parse().unwrap());
        headers.insert("tenantId", self.tenant_id.parse().unwrap());

        let interval = self.interval(config);
        if interval.is_err() {
            return Err("Failed to get interval".into());
        }

        let ts_begin =
            chrono::Utc::now() - chrono::Duration::seconds(interval.unwrap() as i64);
        let ts_end = chrono::Utc::now();

        let request = reqwest::Client
            ::new()
            .get(&url)
            .headers(headers)
            .query(
                &[
                    ("deviceIds", device_name),
                    // yyyy-MM-dd HH:mm:ss.SSS
                    ("tsBegin", ts_begin.to_utc().format("%Y-%m-%d %H:%M:%S").to_string()),
                    ("tsEnd", ts_end.to_utc().format("%Y-%m-%d %H:%M:%S").to_string()),
                ]
            )
            .build()?;

        Ok(request)
    }
}

impl AsyncSource for MASMonitorSource {
    fn get_lock(&self) -> std::sync::Arc<std::sync::RwLock<NodeData>> {
        Arc::clone(&self.data)
    }

    fn subscribe(
        &self,
        node_index: NodeIndex,
        cancellation_token: &tokio_util::sync::CancellationToken,
        transmitter: tokio::sync::mpsc::UnboundedSender<Vec<NodeIndex>>,
        config: &crate::sources::SourceConfig
    ) -> SubscriptionResult {
        RestSource::subscribe(self, node_index, cancellation_token, transmitter, config)
    }
}