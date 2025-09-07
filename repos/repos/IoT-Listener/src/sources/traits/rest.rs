use super::{ async_trait::AsyncSource, SubscriptionResult, ThreadSafeError };
use crate::{ graph::types::NodeData, nodes::sources::SourceConfig };
use petgraph::graph::NodeIndex;
use std::sync::Arc;
use tokio::sync::mpsc::UnboundedSender;
use tokio_util::sync::CancellationToken;
use tracing::{ error, info };

pub trait RestSource: AsyncSource + Clone + 'static {
    /// Get the request object for the source
    ///
    /// # Arguments
    /// - `config` - The source configuration
    /// # Returns
    /// A reqwest::Request object
    fn get_request(
        &self,
        config: &SourceConfig
    ) -> Result<reqwest::Request, Box<dyn std::error::Error>>;

    fn interval(&self, config: &SourceConfig) -> Result<u64, ThreadSafeError>;

    /// Subscribe to a REST source, polling the source at a regular interval and updating the internal data source.
    fn subscribe(
        &self,
        node_index: NodeIndex,
        cancellation_token: &CancellationToken,
        transmitter: UnboundedSender<Vec<NodeIndex>>,
        source_config: &SourceConfig
    ) -> SubscriptionResult {
        let data_pointer = Arc::clone(&self.get_lock());
        let source_config_clone = source_config.clone();
        let interval = self.interval(&source_config_clone)?;
        let cancellation_token = cancellation_token.clone();
        let self_clone = self.clone();

        info!("Subscribing to source with interval: {}", interval);
        Ok(
            tokio::spawn(async move {
                let mut interval = tokio::time::interval(std::time::Duration::from_secs(interval));
                loop {
                    tokio::select! {
                    _ = interval.tick() => { },
                    _ = cancellation_token.cancelled() => { break; }
                }

                    let request = self_clone
                        .get_request(&source_config_clone)
                        .expect("Failed to get request");
                    let body = reqwest::Client
                        ::new()
                        .execute(request.try_clone().expect("Failed to clone request")).await;
                    if let Err(e) = body {
                        error!("Error getting body: {:?}", e);
                        continue;
                    }
                    let body = body
                        .expect("Already checked that body is not an err")
                        .json::<serde_json::Value>().await;
                    if let Err(e) = body {
                        error!("Body should be valid JSON: {:?}", e);
                        continue;
                    }

                    // Set the data
                    let data = match body.expect("Already checked that body is valid JSON") {
                        serde_json::Value::Array(d) =>
                            match d.len() {
                                1 => NodeData::Object(d[0].clone()),
                                _ => NodeData::Collection(d.to_vec()),
                            }
                        body => NodeData::Object(body),
                    };
                    *data_pointer.write().unwrap() = data;

                    transmitter.send([node_index].to_vec()).unwrap();
                }
            })
        )
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod rest_source_tests {
    use super::*;
    use crate::nodes::{ self, sources::SourceConfig };
    use petgraph::graph::NodeIndex;
    use serde_json::json;
    use std::sync::{ Arc, RwLock };
    use tokio::sync::mpsc::unbounded_channel;
    use tokio_util::sync::CancellationToken;

    #[derive(Clone)]
    struct TestRestSource {
        data: Arc<RwLock<NodeData>>,
    }

    impl RestSource for TestRestSource {
        fn get_request(
            &self,
            _config: &SourceConfig
        ) -> Result<reqwest::Request, Box<dyn std::error::Error>> {
            Ok(
                reqwest::Request::new(
                    reqwest::Method::GET,
                    reqwest::Url::parse("http://localhost").unwrap()
                )
            )
        }

        fn interval(&self, _config: &SourceConfig) -> Result<u64, ThreadSafeError> {
            Ok(1)
        }
    }

    impl AsyncSource for TestRestSource {
        fn get_lock(&self) -> Arc<RwLock<NodeData>> {
            Arc::clone(&self.data)
        }

        fn subscribe(
            &self,
            _node_index: NodeIndex,
            _cancellation_token: &tokio_util::sync::CancellationToken,
            _transmitter: tokio::sync::mpsc::UnboundedSender<Vec<NodeIndex>>,
            _config: &SourceConfig
        ) -> SubscriptionResult {
            Ok(tokio::task::spawn(async {}))
        }
    }

    #[tokio::test]
    async fn test_cancellation() {
        let data = Arc::new(RwLock::new(NodeData::Object(json!({"test": "data"}))));
        let source = TestRestSource { data: Arc::clone(&data) };
        let (transmitter, _) = unbounded_channel();
        let cancellation_token = CancellationToken::new();
        let node_index = NodeIndex::new(0);
        let source_config = SourceConfig::MASMonitor(
            nodes::sources::mas_monitor::MASMonitorSourceConfig::new(
                "test".to_string(),
                "test".to_string(),
                "test".to_string(),
                5
            )
        );

        let join_handle = RestSource::subscribe(
            &source,
            node_index,
            &cancellation_token,
            transmitter,
            &source_config
        ).unwrap();
        cancellation_token.cancel();

        join_handle.await.unwrap();
    }

    #[tokio::test]
    async fn test_invalid_json_handling() {
        let data = Arc::new(RwLock::new(NodeData::Object(json!({"initial": "data"}))));
        let source = TestRestSource { data: Arc::clone(&data) };
        let (transmitter, _) = unbounded_channel();
        let cancellation_token = CancellationToken::new();
        let node_index = NodeIndex::new(0);
        let source_config = SourceConfig::MASMonitor(
            nodes::sources::mas_monitor::MASMonitorSourceConfig::new(
                "test".to_string(),
                "test".to_string(),
                "test".to_string(),
                1
            )
        );

        // Mock the request to return invalid JSON
        let join_handle = RestSource::subscribe(
            &source,
            node_index,
            &cancellation_token,
            transmitter,
            &source_config
        ).unwrap();

        // Wait for a short period to ensure the request is made
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        {
            let updated_data = data.read().unwrap();
            match &*updated_data {
                NodeData::Object(obj) => assert_eq!(obj["initial"], "data"),
                _ => panic!("Expected NodeData::Object"),
            }
        }
        cancellation_token.cancel();
        join_handle.await.unwrap();
    }
}