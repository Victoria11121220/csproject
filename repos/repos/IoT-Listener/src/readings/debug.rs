use super::StoreReading;
use crate::entities;
use sea_orm::{DatabaseConnection, EntityTrait, Set};

#[derive(Debug, PartialEq)]
pub struct DebugReading {
    pub node_id: String,
    pub value: serde_json::Value,
}

impl DebugReading {
    pub fn new(node_id: String, value: serde_json::Value) -> Self {
        Self { node_id, value }
    }
}

impl StoreReading for DebugReading {
    async fn store(&self, db: &DatabaseConnection, flow_id: i32) -> Result<(), String> {
        let model = entities::iot_flow_debug_message::ActiveModel {
            flow_id: Set(flow_id),
            debug_node_id: Set(self.node_id.clone()),
            message: Set(self.value.to_string()),
            ..Default::default()
        };

        entities::prelude::IotFlowDebugMessage::insert(model)
            .exec(db)
            .await
            .map_err(|e| e.to_string())?;
        Ok(())
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod debug_reading_tests {
    use sea_orm::{MockDatabase, MockExecResult};

    use super::*;

    #[tokio::test]
    async fn test_store() {
        let db = MockDatabase::new(sea_orm::DatabaseBackend::Postgres)
            .append_query_results([[
                entities::iot_flow_debug_message::Model {
                    id: 1,
                    flow_id: 1,
                    debug_node_id: "node_id".to_string(),
                    message: "{\"key\":\"value\"}".to_string(),
                    timestamp: chrono::Utc::now().naive_utc(),
                }
            ]])
            .append_exec_results([MockExecResult {last_insert_id:1, rows_affected: 1 }])
            .into_connection();

        let reading = DebugReading::new("node_id".to_string(), serde_json::json!({"key": "value"}));
        let result = reading.store(&db, 1).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn store_insert_error() {
        let db = MockDatabase::new(sea_orm::DatabaseBackend::Postgres)
            .append_exec_results([MockExecResult {last_insert_id:0, rows_affected: 0 }])
            .into_connection();

        let reading = DebugReading::new("node_id".to_string(), serde_json::json!({"key": "value"}));
        let result = reading.store(&db, 1).await;
        assert!(result.is_err());
    }
}
