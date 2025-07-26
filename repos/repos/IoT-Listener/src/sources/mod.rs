pub mod mas_monitor;
pub mod mqtt;
pub mod traits;

use crate::{graph::types::NodeData, nodes::sources::SourceConfig};
use enum_dispatch::enum_dispatch;
use petgraph::graph::NodeIndex;
use serde::Deserialize;
use std::sync::{Arc, RwLock};
use traits::{
    async_trait::{AsyncSource, SourceResult},
    SubscriptionResult,
};

#[enum_dispatch(AsyncSource)]
#[derive(Deserialize, Debug, Clone)]
#[serde(tag = "type", content = "config", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Source {
    #[serde(rename = "MAS-MONITOR")]
    MASMonitor(mas_monitor::MASMonitorSource),
    Mqtt(mqtt::MQTTSource),
}
