use petgraph::graph::NodeIndex;
use tokio::{sync::mpsc::UnboundedSender, task::JoinHandle};
use tokio_util::sync::CancellationToken;

use crate::{
    nodes::sources::SourceNode,
    sources::{traits::async_trait::AsyncSource as _, Source},
};

use super::subscribe_and_restart;

/// Subscribes to a source and restarts the thread on failure
///
/// # Arguments
/// - `node_index` - The index of the node in the graph
/// - `source_node` - The source node to subscribe to
/// - `cancellation_token` - A cancellation token to cancel the subscription
/// - `tx` - A channel to send the node index to
///
/// # Returns
/// A join handle for the subscription task
pub fn subscribe_and_monitor_source(
    node_index: NodeIndex,
    source_node: &SourceNode,
    cancellation_token: &CancellationToken,
    tx: UnboundedSender<Vec<NodeIndex>>,
) -> JoinHandle<()> {
    let source_node = source_node.clone();
    let cancellation_token = cancellation_token.clone();
    let subscription_cancellation_token = cancellation_token.clone();
    let url = match source_node.source() {
        Source::MASMonitor(mas_monitor) => mas_monitor.get_monitor_url(),
        Source::Mqtt(mqtt) => mqtt.host.clone(),
        Source::Http(http) => http.endpoint.clone(),
    };

    subscribe_and_restart(
        move || {
            let source = source_node.source();
            source.subscribe(
                node_index,
                &cancellation_token,
                tx.clone(),
                source_node.config(),
            )
        },
        &subscription_cancellation_token,
        &url,
    )
}