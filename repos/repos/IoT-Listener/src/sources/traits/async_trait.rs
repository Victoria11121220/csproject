use crate::{graph::types::NodeData, nodes::sources::SourceConfig};
use enum_dispatch::enum_dispatch;
use petgraph::graph::NodeIndex;
use std::sync::{Arc, RwLock};

use super::SubscriptionResult;

pub type SourceResult = Result<NodeData, Box<dyn std::error::Error>>;

#[enum_dispatch]
pub trait AsyncSource: Send + Sync {
    // Get the most recently received data from the source
    fn get(&self) -> SourceResult {
        let binding = self.get_lock();
        let data = {
            let read_guard = binding.read();
            if read_guard.is_err() {
                return Err("Failed to read lock".into());
            }
            read_guard.unwrap().clone()
        };
        Ok(data)
    }

    fn get_lock(&self) -> Arc<RwLock<NodeData>>;

    fn subscribe(
        &self,
        node_index: NodeIndex,
        cancellation_token: &tokio_util::sync::CancellationToken,
        transmitter: tokio::sync::mpsc::UnboundedSender<Vec<NodeIndex>>,
        config: &SourceConfig,
    ) -> SubscriptionResult;
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod async_trait_tests {
    use super::*;

    #[derive(Clone)]
    struct TestSource {
        data: Arc<RwLock<NodeData>>,
    }

    impl AsyncSource for TestSource {
        fn get_lock(&self) -> Arc<RwLock<NodeData>> {
            Arc::clone(&self.data)
        }

        fn subscribe(
            &self,
            _node_index: NodeIndex,
            _cancellation_token: &tokio_util::sync::CancellationToken,
            _transmitter: tokio::sync::mpsc::UnboundedSender<Vec<NodeIndex>>,
            _config: &SourceConfig,
        ) -> SubscriptionResult {
            Ok(tokio::task::spawn(async { }))
        }

    }

    #[test]
    fn test_get() {
        let data = Arc::new(RwLock::new(NodeData::Object(serde_json::json!({"test": "data"}))));
        let source = TestSource { data: Arc::clone(&data) };
        let result = source.get();
        assert_eq!(result.unwrap(), NodeData::Object(serde_json::json!({"test": "data"})));
    }

    #[tokio::test]
    async fn test_get_poisoned_lock() {
        let data = Arc::new(RwLock::new(NodeData::Object(serde_json::json!({"test": "data"}))));
        let source = TestSource { data: Arc::clone(&data) };
        // spawn a thread that gets write lock and then panics
        let _ = tokio::task::spawn(async move {
            let mut write_guard = data.write().unwrap();
            *write_guard = NodeData::Object(serde_json::json!({"new": "data"}));
            panic!("Poisoned lock");
        }).await;
        let result = source.get();
        assert!(result.is_err());
    }

}