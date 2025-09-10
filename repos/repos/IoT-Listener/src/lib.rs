#![cfg_attr(coverage_nightly, feature(coverage_attribute))]

// Make modules public so they can be used by binary crates
#[cfg_attr(coverage_nightly, coverage(off))]
pub mod entities;
pub mod graph;
pub mod nodes;
pub mod readings;
pub mod sources;
pub mod utils;
#[cfg_attr(coverage_nightly, coverage(off))]
pub mod metrics;

#[macro_use]
extern crate rocket;
extern crate paho_mqtt as mqtt;

// Re-export commonly used items for convenience in binaries
pub use graph::build_graph::{read_graph, from_flow};
pub use graph::flow_updates::{FlowUpdateMessage, FlowGraphs, create_flow_graphs, update_flow_graph, get_flow_graph};
pub use entities::reading::Model as Reading;
pub use sea_orm::{ Database, DatabaseConnection, DbErr };
pub use std::env;
pub use tracing;
pub use tracing_subscriber::{ self, EnvFilter };
pub use utils::subscribe::{subscribe_to_sources, SubscribeError};

/// Get the database connection from the `uri` environment variable.
pub async fn setup_database_connection() -> Result<DatabaseConnection, DbErr> {
	let database_url: String = env::var("uri").expect("uri must be set");
	Database::connect(&database_url).await
}