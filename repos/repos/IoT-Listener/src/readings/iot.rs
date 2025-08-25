use super::StoreReading;
use crate::{
    entities::{
        self, reading,
        sea_orm_active_enums::{IotFieldType, IotUnit},
    },
    metrics::READINGS_SAVED_TOTAL,
};
use sea_orm::{sea_query::OnConflict, DbErr, EntityTrait, Set, TransactionTrait};
use serde::{Deserialize, Serialize};
use tracing::{warn, error};

#[derive(Debug, Deserialize, Serialize, PartialEq)]
pub struct IoTReading {
    identifier: String,
    measuring: String,
    value: String,
    value_type: IotFieldType,
    unit: IotUnit,
    timestamp: chrono::DateTime<chrono::Utc>,
    metadata: Option<serde_json::Map<String, serde_json::Value>>,
}

impl IoTReading {
    fn cast_value(&self, value: &str) -> Option<f64> {
        match self.value_type {
            IotFieldType::Number => value.parse::<f64>().ok(),
            IotFieldType::Boolean => value.parse::<bool>().map(|b| b as i32 as f64).ok(),
            IotFieldType::String => None,
        }
    }

    pub fn get_value(&self) -> &String {
        &self.value
    }

    pub fn get_value_json(&self) -> Option<serde_json::Value> {
        let ret = match self.value_type {
            IotFieldType::Number | IotFieldType::Boolean => {
                let value = self.value.parse::<serde_json::Value>();
                match value {
                    Ok(value) => Some(value),
                    Err(_) => None,
                }
            }
            IotFieldType::String => Some(serde_json::Value::String(self.value.clone())),
        };
        if ret.is_none() {
            warn!("Failed to parse value as JSON: {}", self.value);
        }
        ret
    }

    pub fn identifier(&self) -> &String {
        &self.identifier
    }

    pub fn timestamp(&self) -> &chrono::DateTime<chrono::Utc> {
        &self.timestamp
    }
}

impl StoreReading for IoTReading {
    async fn store(&self, db: &sea_orm::DatabaseConnection, flow_id: i32) -> Result<(), String> {
        let txn = db.begin().await.map_err(|e| e.to_string())?;

        let sensor = entities::sensor::ActiveModel {
            identifier: Set(self.identifier.clone()),
            measuring: Set(self.measuring.clone()),
            unit: Set(self.unit.clone()),
            value_type: Set(self.value_type.clone()),
            flow_id: Set(Some(flow_id)),
            ..Default::default()
        };

        let sensor = entities::prelude::Sensor::insert(sensor)
            .on_conflict(
                OnConflict::columns(vec![
                    entities::sensor::Column::Identifier,
                    entities::sensor::Column::Measuring,
                ])
                .update_columns(vec![
                    entities::sensor::Column::Unit,
                    entities::sensor::Column::ValueType,
                    entities::sensor::Column::FlowId,
                ])
                .to_owned(),
            )
            .exec_with_returning(&txn)
            .await
            .map_err(|e| e.to_string())?;

        let result = entities::reading::ActiveModel {
            sensor_id: Set(sensor.id),
            value: Set(self.cast_value(&self.value)),
            raw_value: Set(self.value.clone()),
            timestamp: Set(self.timestamp),
            ..Default::default()
        };

        let conflict_action =
            OnConflict::columns([reading::Column::SensorId, reading::Column::Timestamp])
                .update_columns([reading::Column::RawValue, reading::Column::Value]).to_owned();

        let insert_query = entities::prelude::Reading::insert(result)
            .on_conflict(
                conflict_action, 
            )
            .exec(&txn)
            .await;

        match insert_query {
            Ok(_) => {
            }
            Err(DbErr::RecordNotInserted) => {
                error!("err: RecordNotInserted")
            }
            Err(e) => {
                return Err(e.to_string());
            }
        }
        // entities::prelude::Reading::insert(result)
        //     .on_conflict(
        //         // 1. Specify the basis for conflict judgment: must fully match the primary key or unique constraint defined in the table
        //         OnConflict::columns([
        //             entities::reading::Column::SensorId,
        //             entities::reading::Column::Timestamp,
        //         ])
        //         // 2. Specify what to do when a conflict occurs: Update the specified column
        //         .update_columns([
        //             entities::reading::Column::RawValue,
        //             entities::reading::Column::Value,
        //         ]),
        //     )
        //     .exec(&txn)
        //     .await
        //     .map_err(|e| e.to_string())?;

        for (key, value) in self.metadata.clone().unwrap_or_default() {
            let metadata = entities::sensor_metadata::ActiveModel {
                sensor_id: Set(sensor.id),
                key: Set(key),
                value: Set(value.to_string()),
                ..Default::default()
            };
            entities::prelude::SensorMetadata::insert(metadata)
                .on_conflict(
                    OnConflict::columns(vec![
                        entities::sensor_metadata::Column::SensorId,
                        entities::sensor_metadata::Column::Key,
                    ])
                    .update_columns(vec![entities::sensor_metadata::Column::Value])
                    .to_owned(),
                )
                .exec(&txn)
                .await
                .map_err(|e| e.to_string())?;
        }

        txn.commit().await.map_err(|e| e.to_string())?;
        READINGS_SAVED_TOTAL
            .with_label_values(&[flow_id.to_string().as_str()])
            .inc();
        Ok(())
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod iot_reading_tests {
    use sea_orm::{MockDatabase, MockExecResult};

    use super::*;

    #[tokio::test]
    async fn test_store() {
        let db = MockDatabase::new(sea_orm::DatabaseBackend::Postgres)
            .append_query_results([vec![entities::sensor::Model {
                id: 1,
                identifier: "identifier".to_string(),
                measuring: "measuring".to_string(),
                unit: IotUnit::DegreesCelcius,
                value_type: IotFieldType::Number,
                flow_id: Some(1),
            }]])
            .append_query_results([vec![entities::reading::Model {
                id: 1,
                sensor_id: 1,
                value: Some(1.0),
                raw_value: "1.0".to_string(),
                timestamp: chrono::Utc::now(),
            }]])
            .append_exec_results([MockExecResult {
                last_insert_id: 1,
                rows_affected: 1,
            }])
            .into_connection();

        let reading = IoTReading {
            identifier: "identifier".to_string(),
            measuring: "measuring".to_string(),
            value: "1".to_string(),
            value_type: IotFieldType::Number,
            unit: IotUnit::DegreesCelcius,
            timestamp: chrono::Utc::now(),
            metadata: None,
        };
        let result = reading.store(&db, 1).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn store_insert_error() {
        let db = MockDatabase::new(sea_orm::DatabaseBackend::Postgres)
            .append_exec_results([MockExecResult {
                last_insert_id: 0,
                rows_affected: 0,
            }])
            .into_connection();

        let reading = IoTReading {
            identifier: "identifier".to_string(),
            measuring: "measuring".to_string(),
            value: "value".to_string(),
            value_type: IotFieldType::Number,
            unit: IotUnit::DegreesCelcius,
            timestamp: chrono::Utc::now(),
            metadata: None,
        };
        let result = reading.store(&db, 1).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn store_with_metadata() {
        let db = MockDatabase::new(sea_orm::DatabaseBackend::Postgres)
            .append_query_results([vec![entities::sensor::Model {
                id: 1,
                identifier: "identifier".to_string(),
                measuring: "measuring".to_string(),
                unit: IotUnit::DegreesCelcius,
                value_type: IotFieldType::Number,
                flow_id: Some(1),
            }]])
            .append_query_results([vec![entities::reading::Model {
                id: 1,
                sensor_id: 1,
                value: Some(1.0),
                raw_value: "1.0".to_string(),
                timestamp: chrono::Utc::now(),
            }]])
            .append_query_results([vec![entities::sensor_metadata::Model {
                id: 1,
                sensor_id: 1,
                key: "key".to_string(),
                value: "value".to_string(),
            }]])
            .append_exec_results([MockExecResult {
                last_insert_id: 1,
                rows_affected: 1,
            }])
            .append_exec_results([MockExecResult {
                last_insert_id: 1,
                rows_affected: 1,
            }])
            .append_exec_results([MockExecResult {
                last_insert_id: 1,
                rows_affected: 1,
            }])
            .into_connection();

        let mut metadata = serde_json::Map::new();
        metadata.insert(
            "key".to_string(),
            serde_json::Value::String("value".to_string()),
        );

        let reading = IoTReading {
            identifier: "identifier".to_string(),
            measuring: "measuring".to_string(),
            value: "1.0".to_string(),
            value_type: IotFieldType::Number,
            unit: IotUnit::DegreesCelcius,
            timestamp: chrono::Utc::now(),
            metadata: Some(metadata),
        };
        let result = reading.store(&db, 1).await;
        assert!(result.is_ok());
    }

    #[test]
    fn get_value() {
        let reading = IoTReading {
            identifier: "identifier".to_string(),
            measuring: "measuring".to_string(),
            value: "value".to_string(),
            value_type: IotFieldType::Number,
            unit: IotUnit::DegreesCelcius,
            timestamp: chrono::Utc::now(),
            metadata: None,
        };
        assert_eq!(reading.get_value(), "value");
    }

    #[test]
    fn get_value_json() {
        let reading = IoTReading {
            identifier: "identifier".to_string(),
            measuring: "measuring".to_string(),
            value: r#"{"key": "value"}"#.to_string(),
            value_type: IotFieldType::Number,
            unit: IotUnit::DegreesCelcius,
            timestamp: chrono::Utc::now(),
            metadata: None,
        };
        assert_eq!(
            reading.get_value_json().unwrap(),
            serde_json::json!({"key": "value"})
        );
    }

    #[test]
    fn identifier() {
        let reading = IoTReading {
            identifier: "identifier".to_string(),
            measuring: "measuring".to_string(),
            value: "value".to_string(),
            value_type: IotFieldType::Number,
            unit: IotUnit::DegreesCelcius,
            timestamp: chrono::Utc::now(),
            metadata: None,
        };
        assert_eq!(reading.identifier(), "identifier");
    }

    #[test]
    fn timestamp() {
        let timestamp = chrono::Utc::now();
        let reading = IoTReading {
            identifier: "identifier".to_string(),
            measuring: "measuring".to_string(),
            value: "value".to_string(),
            value_type: IotFieldType::Number,
            unit: IotUnit::DegreesCelcius,
            timestamp,
            metadata: None,
        };
        assert_eq!(reading.timestamp(), &timestamp);
    }

    #[test]
    fn cast_value_number() {
        let reading = IoTReading {
            identifier: "identifier".to_string(),
            measuring: "measuring".to_string(),
            value: "1.0".to_string(),
            value_type: IotFieldType::Number,
            unit: IotUnit::DegreesCelcius,
            timestamp: chrono::Utc::now(),
            metadata: None,
        };
        assert_eq!(reading.cast_value("1.0"), Some(1.0));

        let reading = IoTReading {
            identifier: "identifier".to_string(),
            measuring: "measuring".to_string(),
            value: "1".to_string(),
            value_type: IotFieldType::Boolean,
            unit: IotUnit::DegreesCelcius,
            timestamp: chrono::Utc::now(),
            metadata: None,
        };
        assert_eq!(reading.cast_value("1"), None);

        let reading = IoTReading {
            identifier: "identifier".to_string(),
            measuring: "measuring".to_string(),
            value: "1".to_string(),
            value_type: IotFieldType::String,
            unit: IotUnit::DegreesCelcius,
            timestamp: chrono::Utc::now(),
            metadata: None,
        };
        assert_eq!(reading.cast_value("1"), None);

        let reading = IoTReading {
            identifier: "identifier".to_string(),
            measuring: "measuring".to_string(),
            value: "true".to_string(),
            value_type: IotFieldType::Boolean,
            unit: IotUnit::DegreesCelcius,
            timestamp: chrono::Utc::now(),
            metadata: None,
        };
        assert_eq!(reading.cast_value("true"), Some(1.0));
    }
}