use std::{ collections::HashMap, sync::Arc };

use chrono::DateTime;
use petgraph::graph::NodeIndex;
use sea_orm::{ ColumnTrait as _, DatabaseConnection, EntityTrait as _, QueryFilter as _ };
use tokio::{ sync::mpsc::UnboundedSender, task::JoinHandle };
use tokio_util::sync::CancellationToken;

use crate::{
	entities,
	nodes::datalake::ThreadSafeReadingStore,
	sources::traits::SubscriptionResult,
};

use super::subscribe_and_restart;

const QUERY_INTERVAL: u64 = 5; // seconds

/// Reads latest readings from a given timestamp and returns them
///
/// # Arguments
/// - `database` - A database connection
/// - `sensor_ids` - A vector of sensor IDs
/// - `timestamp` - A timestamp to read from
///
/// # Returns
/// A vector of readings
async fn read_latest_readings(
	database: &DatabaseConnection,
	sensor_ids: Vec<i32>,
	timestamp: DateTime<chrono::Utc>
) -> Result<
	Vec<(entities::reading::Model, Option<entities::sensor::Model>)>,
	Box<dyn std::error::Error>
> {
	let to_time = timestamp + chrono::Duration::seconds(QUERY_INTERVAL as i64);
	entities::reading::Entity
		::find()
		.find_also_related(entities::sensor::Entity)
		.filter(entities::reading::Column::SensorId.is_in(sensor_ids))
		.filter(entities::reading::Column::Timestamp.gt(timestamp))
		.filter(entities::reading::Column::Timestamp.lt(to_time))
		.all(database).await
		.map_err(|e| {
			error!("Failed to read latest readings: {}", e);
			Box::new(e) as Box<dyn std::error::Error>
		})
}

/// Polls the database for new readings and updates the reading stores
///
/// # Arguments
/// - `node_idx_mapping` - A mapping of sensor IDs to node indices
/// - `node_store_mapping` - A mapping of sensor IDs to reading stores
/// - `cancellation_token` - A cancellation token to cancel the subscription
/// - `tx` - A channel to send the node indices to
/// - `db` - A database connection
/// - `flow_id` - The ID of the flow being processed
///
/// # Returns
/// A subscription result
fn poll_and_update_readings(
	node_idx_mapping: HashMap<i32, Vec<NodeIndex>>,
	node_store_mapping: HashMap<i32, Vec<ThreadSafeReadingStore>>,
	cancellation_token: &CancellationToken,
	tx: UnboundedSender<Vec<NodeIndex>>,
	db: Arc<DatabaseConnection>,
	flow_id: i32
) -> SubscriptionResult {
	let cancellation_token = cancellation_token.clone();
	let sensor_ids: Vec<i32> = node_idx_mapping.keys().cloned().collect();
	Ok(
		tokio::spawn(async move {
			info!("Initialised database polling thread for flow_id: {}", flow_id);
			loop {
				let last_update = chrono::Utc::now();
				tokio::select! {
                _ = cancellation_token.cancelled() => {
                    info!("Cancelling postgres event listener for flow_id: {}", flow_id);
                    return;
                }
                _ = tokio::time::sleep(tokio::time::Duration::from_secs(QUERY_INTERVAL)) => {
                    // Poll the database for new readings
                    let new_readings = read_latest_readings(&db, sensor_ids.clone(), last_update).await;
                    match new_readings {
                        Ok(readings) => {
                            // Filter out readings with no sensor
                            let readings = readings
                                .into_iter()
                                .filter_map(|(reading, sensor)| {
                                    sensor.map(|sensor| (reading, sensor))
                                })
                                .collect::<Vec<_>>();
                            // Create a mapping of sensor Ids to (sensor, Vec<readings)
                            let mut reading_map = HashMap::<i32, (entities::sensor::Model, Vec<entities::reading::Model>)>::new();
                            for (reading, sensor) in readings.clone() {
                                reading_map
                                    .entry(reading.sensor_id)
                                    .and_modify(|(_, readings)| readings.push(reading.clone()))
                                    .or_insert_with(|| (sensor, vec![reading]));
                            }
                            // Update the reading stores with the sensor models and list of readings
                            for (sensor_id, readings) in reading_map.iter() {
                                if let Some(reading_stores) = node_store_mapping.get(sensor_id) {
                                    for reading_store in reading_stores {
                                        let mut store = reading_store.write().unwrap();
                                        *store = Some(readings.clone());
                                    }
                                }
                            }
                            // Collect all node indices to pass to transmitter
                            let node_idxs = node_idx_mapping
                                .iter()
                                .filter_map(|(sensor_id, node_indices)| {
                                    if readings.iter().any(|(r, _)| r.sensor_id == *sensor_id) {
                                        Some(node_indices.clone())
                                    } else {
                                        None
                                    }
                                })
                                .flatten()
                                .collect::<Vec<NodeIndex>>();
                            if !node_idxs.is_empty() {
                                if let Err(e) = tx.send(node_idxs.clone()) {
                                    error!("Failed to send node indices for flow_id {}: {}", flow_id, e);
                                }
                            }
                        }
                        Err(e) => {
                            error!("Failed to read latest readings for flow_id {}: {}", flow_id, e);
                        }
                    }
                }
            }
			}
		})
	)
}

/// Spawns a thread for polling the database,
/// subscribing to new readings, and updating the reading stores.
/// Restarts the thread on failure.
///
/// # Arguments
/// - `node_idx_mapping` - A mapping of sensor IDs to node indices
/// - `node_store_mapping` - A mapping of sensor IDs to reading stores
/// - `cancellation_token` - A cancellation token to cancel the subscription
/// - `tx` - A channel to send the node indices to
/// - `db` - A database connection
/// - `flow_id` - The ID of the flow being processed
///
/// # Returns
/// A join handle for the subscription task
pub fn watch_database(
	node_idx_mapping: HashMap<i32, Vec<NodeIndex>>,
	node_store_mapping: HashMap<i32, Vec<ThreadSafeReadingStore>>,
	cancellation_token: &CancellationToken,
	tx: UnboundedSender<Vec<NodeIndex>>,
	db: Arc<DatabaseConnection>,
	flow_id: i32
) -> JoinHandle<()> {
	let cancellation_token = cancellation_token.clone();
	let subscription_cancellation_token = cancellation_token.clone();
	subscribe_and_restart(
		move || {
			let db = db.clone();
			let node_idx_mapping = node_idx_mapping.clone();
			let node_store_mapping = node_store_mapping.clone();
			let flow_id = flow_id;
			poll_and_update_readings(
				node_idx_mapping,
				node_store_mapping,
				&cancellation_token,
				tx.clone(),
				db,
				flow_id
			)
		},
		&subscription_cancellation_token,
		&format!("Database (flow_id: {})", flow_id)
	)
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
	use sea_orm::MockDatabase;

	use super::*;

	#[tokio::test]
	async fn test_read_latest_readings() {
		let db = MockDatabase::new(sea_orm::DatabaseBackend::Postgres)
			.append_query_results(
				vec![
					vec![(
						entities::reading::Model {
							id: 1,
							sensor_id: 1,
							value: Some(10.0),
							raw_value: "10.0".to_string(),
							timestamp: chrono::Utc::now().naive_utc(),
						},
						Some(entities::sensor::Model {
							id: 1,
							flow_id: Some(1),
							identifier: "Sensor 1".to_string(),
							measuring: "Temperature".to_string(),
							unit: entities::sea_orm_active_enums::IotUnit::DegreesCelcius,
							value_type: entities::sea_orm_active_enums::IotFieldType::Number,
						}),
					)]
				]
			)
			.into_connection();

		let sensor_ids = vec![1];
		let timestamp = chrono::Utc::now() - chrono::Duration::seconds(10);
		let result = read_latest_readings(&db, sensor_ids.clone(), timestamp).await;
		assert!(result.is_ok());
		let readings = result.unwrap();
		assert_eq!(readings.len(), 1);
		assert_eq!(readings[0].0.sensor_id, 1);
		assert_eq!(readings[0].1.as_ref().unwrap().id, 1);

		// Test with error
		let db = MockDatabase::new(sea_orm::DatabaseBackend::Postgres)
			.append_query_errors(
				vec![sea_orm::DbErr::Query(sea_orm::RuntimeErr::Internal("Error".to_string()))]
			)
			.into_connection();
		let result = read_latest_readings(&db, sensor_ids, timestamp).await;
		assert!(result.is_err());
		let error = result.unwrap_err();
		assert_eq!(error.to_string(), "Query Error: Error");
	}

	#[tokio::test]
	async fn test_cancel_watch_database() {
		let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
		let cancellation_token = CancellationToken::new();
		let db = Arc::new(MockDatabase::new(sea_orm::DatabaseBackend::Postgres).into_connection());
		let node_idx_mapping = HashMap::<i32, Vec<NodeIndex>>::new();
		let node_store_mapping = HashMap::<i32, Vec<ThreadSafeReadingStore>>::new();

		let handle = watch_database(
			node_idx_mapping,
			node_store_mapping,
			&cancellation_token,
			tx.clone(),
			db,
			1 // flow_id for testing
		);
		cancellation_token.cancel();
		handle.await.unwrap();
		// Should error if the cancellation was successful
		assert!(rx.try_recv().is_err());
	}
}