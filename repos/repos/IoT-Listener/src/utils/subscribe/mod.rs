use std::{ cmp::min, collections::HashMap, sync::Arc };

use database::watch_database;
use petgraph::graph::{ DiGraph, NodeIndex };
use sea_orm::DatabaseConnection;
use sources::subscribe_and_monitor_source;
use tokio::{ sync::mpsc::UnboundedReceiver, task::JoinHandle };
use tokio_util::sync::CancellationToken;

use crate::{
	graph::{ edge::Edge, node::Node },
	metrics::SOURCE_LAST_ERROR_TIME,
	nodes::{ datalake::ThreadSafeReadingStore, NodeType },
	sources::traits::ThreadSafeError,
};

mod database;
mod sources;

// Define a custom error type that is Send + Sync
#[derive(Debug)]
pub enum SubscribeError {
	Generic(String),
}

impl std::fmt::Display for SubscribeError {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			SubscribeError::Generic(msg) => write!(f, "{}", msg),
		}
	}
}

impl std::error::Error for SubscribeError {}

impl From<Box<dyn std::error::Error>> for SubscribeError {
	fn from(error: Box<dyn std::error::Error>) -> Self {
		SubscribeError::Generic(error.to_string())
	}
}

impl From<String> for SubscribeError {
	fn from(error: String) -> Self {
		SubscribeError::Generic(error)
	}
}

impl From<ThreadSafeError> for SubscribeError {
	fn from(error: ThreadSafeError) -> Self {
		SubscribeError::Generic(error.to_string())
	}
}

type SubscribeThreadResult = Result<JoinHandle<()>, ThreadSafeError>;
type SourcesSubscriptionResult = Result<
	(Vec<JoinHandle<()>>, UnboundedReceiver<Vec<NodeIndex>>),
	SubscribeError
>;

/// Reports an error to prometheus and sleeps for a given time
///
/// # Arguments
/// - `label` - A label to report the error
/// - `backoff_time` - A mutable reference to the backoff time
///
/// # Returns
/// A future that sleeps for the given time
async fn report_and_sleep(label: &str, backoff_time: &mut u64) {
	*backoff_time = min(*backoff_time * 2, 3600); // increment backoff_time
	SOURCE_LAST_ERROR_TIME.with_label_values(&[label]).set(chrono::Utc::now().timestamp() as f64);
	tokio::time::sleep(tokio::time::Duration::from_secs(*backoff_time)).await;
}

/// Subscribes to a thread that runs a function and restarts it on failure
///
/// # Arguments
/// - `function` - A function that returns a JoinHandle
/// - `cancellation_token` - A cancellation token to cancel the subscription
/// - `label` - A label to report any errors and log messages
fn subscribe_and_restart(
	function: impl (Fn() -> SubscribeThreadResult) + Send + Clone + 'static,
	cancellation_token: &CancellationToken,
	label: &str
) -> JoinHandle<()> {
	// Clone cancellation token to move into the async block
	let cancellation_token = cancellation_token.clone();
	let label = label.to_string();
	tokio::spawn(async move {
		let mut backoff_time = 1;
		loop {
			// Get the handle to the function
			let handle = function();
			// If error attempt to recreate the source
			if let Err(e) = handle {
				error!("Failed to subscribe to {}: {}", label, e);
				report_and_sleep(&label, &mut backoff_time).await;
				continue;
			}
			let handle = handle.unwrap();
			// Get abort handle for cancelling the task
			let abort_handle = handle.abort_handle();
			tokio::select! {
                _ = cancellation_token.cancelled() => {
                    info!("Cancelling {:?} monitor and joining source", label);
                    abort_handle.abort();
                    break;
                }
                task_res = handle => {
                    match task_res {
                        // Called when thread terminates successfully
                        Ok(_) => {
                            error!("{:?} thread terminated, retrying in {} seconds.", label, backoff_time);
                            report_and_sleep(&label, &mut backoff_time).await;
                            continue;
                        }
                        // Called when thread terminates with cancelled error
                        Err(e) if e.is_cancelled() => { info!("Source was cancelled"); break; }
                        // Any other error we should retry
                        Err(e) => {
                            error!("{:?} failed, retrying in {} seconds: {}", label, backoff_time, e);
                            report_and_sleep(&label, &mut backoff_time).await;
                            continue;
                        }
                    }
                }
            }
		}
	})
}

/// Subscribe to both the sources and the reading nodes in the graph
///
/// # Arguments
/// - `graph` - A petgraph DiGraph containing the nodes and edges
/// - `cancellation_token` - A cancellation token to cancel the subscription
/// - `db` - A database connection
/// - `flow_id` - The ID of the flow being processed
///
/// # Returns
/// A tuple containing the join handles and receivers for the sources
pub fn subscribe_to_sources(
	graph: &DiGraph<Node, Edge>,
	cancellation_token: &CancellationToken,
	db: Arc<DatabaseConnection>,
	flow_id: i32
) -> SourcesSubscriptionResult {
	let (tx, rx) = tokio::sync::mpsc::unbounded_channel::<Vec<NodeIndex>>();
	let mut join_handles = Vec::new();
	// Mappings from sensor_id to node indices and reading stores
	// This is used to update the reading stores when polling the database
	let mut node_idx_mapping: HashMap<i32, Vec<NodeIndex>> = HashMap::new();
	let mut node_store_mapping: HashMap<i32, Vec<ThreadSafeReadingStore>> = HashMap::new();

	for node_index in graph.node_indices() {
		let concrete_node = graph[node_index].concrete_node();
		if let NodeType::Source(source_node) = concrete_node {
			let monitor_handle = subscribe_and_monitor_source(
				node_index,
				source_node,
				cancellation_token,
				tx.clone()
			);
			join_handles.push(monitor_handle);
		}
		if let NodeType::Datalake(reading_node) = concrete_node {
			let sensor_id = reading_node.sensor_id();
			node_idx_mapping.entry(sensor_id).or_default().push(node_index);
			node_store_mapping.entry(sensor_id).or_default().push(reading_node.get_reading());
		}
	}

	join_handles.push(
		watch_database(
			node_idx_mapping.clone(),
			node_store_mapping.clone(),
			cancellation_token,
			tx.clone(),
			db.clone(),
			flow_id
		)
	);

	Ok((join_handles, rx))
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
	use super::*;

	#[tokio::test]
	async fn test_subscribe_and_restart() {
		let cancellation_token = CancellationToken::new();
		let label = "test";
		let function = || Ok(tokio::task::spawn(async {}));
		let handle = subscribe_and_restart(function, &cancellation_token, label);
		// Briefly monitor the handle to ensure it is running
		assert!(!handle.is_finished());
		tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
		assert!(!handle.is_finished());
		// Cancel the task
		cancellation_token.cancel();
		tokio::time::sleep(tokio::time::Duration::from_millis(3000)).await;
		// Check if the task is finished
		assert!(handle.is_finished());
	}
}