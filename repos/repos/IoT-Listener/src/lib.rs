pub mod entities;
pub mod graph;
pub mod metrics;
pub mod nodes;
pub mod readings;
pub mod sources;
pub mod utils;
extern crate paho_mqtt as mqtt;
use sea_orm::{Database, DatabaseConnection, DbErr};
use serde::{Deserialize, Serialize};
use std::env;

use crate::graph::types::GraphPayload;

/// Get the database connection
pub async fn get_database_connection() -> Result<DatabaseConnection, DbErr> {
    let database_url: String = env::var("uri").expect("uri must be set");
    Database::connect(&database_url).await
}

#[derive(Debug, Serialize, Deserialize)]
pub struct KafkaTrigger {
    pub index: usize,
    pub payload: GraphPayload,
}