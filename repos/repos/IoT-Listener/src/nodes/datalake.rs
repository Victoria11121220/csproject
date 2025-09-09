use crate::{
	entities,
	graph::{
		concrete_node::ConcreteNode,
		node::{ NodeError, NodeResult },
		types::{ GraphPayload, GraphPayloadObjects, NodeData },
	},
};
use serde::Deserialize;
use std::{ collections::HashSet, sync::{ Arc, RwLock } };

/// The reading node holds a vector of recent readings from a sensor
/// Thread safe definition is used as the listening threads need to acquire a lock
/// on the updated readings
pub type ReadingStore = (entities::sensor::Model, Vec<entities::reading::Model>);
pub type ThreadSafeReadingStore = Arc<RwLock<Option<ReadingStore>>>;

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct DatalakeNode {
	sensor_id: i32,
	#[serde(skip)]
	reading: ThreadSafeReadingStore,
	// Pre-filled data for use in processor
	#[serde(skip)]
	prefilled_data: Arc<RwLock<Option<NodeData>>>,
}

impl DatalakeNode {
	pub fn sensor_id(&self) -> i32 {
		self.sensor_id
	}

	pub fn get_reading(&self) -> ThreadSafeReadingStore {
		Arc::clone(&self.reading)
	}
	
	/// Set prefilled data for use in processor
    pub fn set_prefilled_data(&self, data: NodeData) -> Result<(), NodeError> {
        let mut prefilled_data = self.prefilled_data.write().map_err(|_| NodeError::GenericComputationError("Failed to write prefilled data".to_string()))?;
        *prefilled_data = Some(data);
        Ok(())
    }
    
    /// Clear prefilled data
    pub fn clear_prefilled_data(&self) -> Result<(), NodeError> {
        let mut prefilled_data = self.prefilled_data.write().map_err(|_| NodeError::GenericComputationError("Failed to write prefilled data".to_string()))?;
        *prefilled_data = None;
        Ok(())
    }
    
    /// Get prefilled data if available
    fn get_prefilled_data(&self) -> Result<Option<NodeData>, NodeError> {
        let prefilled_data = self.prefilled_data.read().map_err(|_| NodeError::GenericComputationError("Failed to read prefilled data".to_string()))?;
        Ok(prefilled_data.clone())
    }
}

impl Default for DatalakeNode {
    fn default() -> Self {
        Self {
            sensor_id: 0,
            reading: Arc::new(RwLock::new(None)),
            prefilled_data: Arc::new(RwLock::new(None)),
        }
    }
}

impl ConcreteNode for DatalakeNode {
	fn set_actual_handles(
		&mut self,
		input_handles: HashSet<String>,
		output_handles: HashSet<String>
	) -> Result<(), NodeError> {
		if !input_handles.is_empty() {
			return Err(
				NodeError::HandleValidationError(
					"ReadingNode must not have input handles".to_string()
				)
			);
		}
		if output_handles.len() > 1 {
			return Err(
				NodeError::HandleValidationError(
					"ReadingNode must not have more than one output handle".to_string()
				)
			);
		}
		Ok(())
	}

	fn generates_reading(&self) -> bool {
		false
	}

	async fn compute_objects(&self, _inputs: &GraphPayloadObjects) -> NodeResult {
		// Check if we have prefilled data (used in processor)
        if let Ok(Some(prefilled_data)) = self.get_prefilled_data() {
            match prefilled_data {
                NodeData::Object(value) => return Ok(GraphPayload::Objects(vec![(self.default_output_handle(), value)].into_iter().collect())),
                NodeData::Collection(values) => return Ok(GraphPayload::Collections(
                    vec![(self.default_output_handle(), values)].into_iter().collect(),
                )),
            }
        }
        
		let reading_lock = self.get_reading();
		// Try to acquire a read lock on the reading store
		let (sensor, readings) = match reading_lock.read() {
			Ok(guard) => {
				// If readings available, return the latest readings
				if let Some(reading_store) = guard.as_ref() {
					reading_store.clone()
				} else {
					// If not available, return a null value to the flow
					return Ok(
						GraphPayload::Objects(
							vec![(self.default_output_handle(), serde_json::json!(null))]
								.into_iter()
								.collect()
						)
					);
				}
			}
			// If the lock is not available, return an error
			Err(_) => {
				return Err(
					NodeError::GenericComputationError("Failed to read reading".to_string())
				);
			}
		};

		// Iterate over each reading and convert to the correct type
		// based on the sensor's value type
		let values = readings
			.iter()
			.map(|reading| {
				match sensor.value_type {
					entities::sea_orm_active_enums::IotFieldType::String => {
						serde_json::json!(reading.raw_value.clone())
					}
					entities::sea_orm_active_enums::IotFieldType::Number => {
						serde_json::json!(reading.value)
					}
					entities::sea_orm_active_enums::IotFieldType::Boolean => {
						serde_json::json!(reading.raw_value == "true")
					}
				}
			})
			.collect::<Vec<_>>();

		match values.len() {
			// If no readings available, return a null value to the flow
			0 =>
				Ok(
					GraphPayload::Objects(
						vec![(self.default_output_handle(), serde_json::json!(null))]
							.into_iter()
							.collect()
					)
				),
			// If only one reading available, return a single value to the flow
			1 =>
				Ok(
					GraphPayload::Objects(
						vec![(self.default_output_handle(), values[0].clone())]
							.into_iter()
							.collect()
					)
				),
			// If multiple readings available, return a collection of values to the flow
			_ =>
				Ok(
					GraphPayload::Collections(
						vec![(self.default_output_handle(), values)].into_iter().collect()
					)
				),
		}
	}
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
	use super::*;
	use crate::graph::types::GraphPayload;
	use std::collections::HashMap;

	#[test]
	fn test_deserialise_reading_node() {
		let json = r#"{"sensorId": 1}"#;
		let reading_node: DatalakeNode = serde_json::from_str(json).unwrap();
		assert_eq!(reading_node.sensor_id(), 1);
	}

	#[test]
	fn test_set_actual_handles() {
		let mut reading_node = DatalakeNode::default();
		let input_handles = HashSet::new();
		let output_handles = HashSet::from([reading_node.default_output_handle()]);
		assert!(reading_node.set_actual_handles(input_handles, output_handles).is_ok());

		let input_handles = HashSet::from(["test".to_string()]);
		let output_handles = HashSet::from([reading_node.default_output_handle()]);
		assert!(reading_node.set_actual_handles(input_handles, output_handles).is_err());

		let input_handles = HashSet::new();
		let output_handles = HashSet::from([
			reading_node.default_output_handle(),
			"another1".to_string(),
		]);
		assert!(reading_node.set_actual_handles(input_handles, output_handles).is_err());
	}

	#[tokio::test]
	async fn test_reading_node() {
		let reading_node = DatalakeNode::default();
		assert_eq!(reading_node.sensor_id(), 0); // Default value
		assert!(reading_node.get_reading().read().unwrap().is_none());
		assert!(!reading_node.generates_reading());
	}

	async fn reading_node_eq(
		reading: entities::reading::Model,
		sensor: entities::sensor::Model,
		reading_node: &DatalakeNode,
		output: GraphPayload
	) {
		let reading_store = Some((sensor, vec![reading]));
		let reading_lock = reading_node.get_reading();
		{
			let mut guard = reading_lock.write().unwrap();
			*guard = reading_store;
		}
		let result = reading_node.compute_objects(&HashMap::new()).await;
		assert!(result.is_ok());
		let payload = result.unwrap();
		assert_eq!(payload, output);
	}

	#[tokio::test]
	async fn test_reading_node_computation() {
		let reading_node = DatalakeNode::default();
		let empty_map: HashMap<String, serde_json::Value> = HashMap::new();
		let result = reading_node.compute_objects(&empty_map).await;
		assert!(result.is_ok());
		let payload = result.unwrap();
		assert_eq!(
			payload,
			GraphPayload::Objects(
				HashMap::from([(reading_node.default_output_handle(), serde_json::json!(null))])
			)
		);

		// Numeric reading
		let reading = entities::reading::Model {
			id: 1,
			sensor_id: 1,
			value: Some(10.0),
			raw_value: "10.0".to_string(),
			timestamp: chrono::Utc::now().naive_utc(),
		};
		let sensor = entities::sensor::Model {
			id: 1,
			flow_id: Some(1),
			identifier: "Sensor 1".to_string(),
			measuring: "Temperature".to_string(),
			unit: entities::sea_orm_active_enums::IotUnit::DegreesCelcius,
			value_type: entities::sea_orm_active_enums::IotFieldType::Number,
		};
		reading_node_eq(
			reading,
			sensor,
			&reading_node,
			GraphPayload::Objects(
				HashMap::from([(reading_node.default_output_handle(), serde_json::json!(10.0))])
			)
		).await;

		// Boolean reading
		let reading = entities::reading::Model {
			id: 2,
			sensor_id: 1,
			value: None,
			raw_value: "true".to_string(),
			timestamp: chrono::Utc::now().naive_utc(),
		};

		let sensor = entities::sensor::Model {
			id: 1,
			flow_id: Some(1),
			identifier: "Sensor 1".to_string(),
			measuring: "Temperature".to_string(),
			unit: entities::sea_orm_active_enums::IotUnit::DegreesCelcius,
			value_type: entities::sea_orm_active_enums::IotFieldType::Boolean,
		};
		reading_node_eq(
			reading,
			sensor,
			&reading_node,
			GraphPayload::Objects(
				HashMap::from([(reading_node.default_output_handle(), serde_json::json!(true))])
			)
		).await;

		// String reading
		let reading = entities::reading::Model {
			id: 3,
			sensor_id: 1,
			value: None,
			raw_value: "test".to_string(),
			timestamp: chrono::Utc::now().naive_utc(),
		};
		let sensor = entities::sensor::Model {
			id: 1,
			flow_id: Some(1),
			identifier: "Sensor 1".to_string(),
			measuring: "Temperature".to_string(),
			unit: entities::sea_orm_active_enums::IotUnit::DegreesCelcius,
			value_type: entities::sea_orm_active_enums::IotFieldType::String,
		};
		reading_node_eq(
			reading,
			sensor,
			&reading_node,
			GraphPayload::Objects(
				HashMap::from([(reading_node.default_output_handle(), serde_json::json!("test"))])
			)
		).await;
	}

	#[tokio::test]
	async fn test_reading_node_prefilled_data() {
		let reading_node = DatalakeNode::default();
		let empty_map: HashMap<String, serde_json::Value> = HashMap::new();
		
		// Test with prefilled data
		let prefilled_data = crate::graph::types::NodeData::Collection(vec![
			serde_json::json!(10.0),
			serde_json::json!(20.0),
			serde_json::json!(30.0)
		]);
		
		assert!(reading_node.set_prefilled_data(prefilled_data.clone()).is_ok());
		
		let result = reading_node.compute_objects(&empty_map).await;
		assert!(result.is_ok());
		let payload = result.unwrap();
		
		match payload {
			GraphPayload::Collections(data_map) => {
				assert!(data_map.contains_key(&reading_node.default_output_handle()));
				let values = data_map.get(&reading_node.default_output_handle()).unwrap();
				assert_eq!(values.len(), 3);
				assert_eq!(values[0], serde_json::json!(10.0));
				assert_eq!(values[1], serde_json::json!(20.0));
				assert_eq!(values[2], serde_json::json!(30.0));
			}
			_ => panic!("Expected GraphPayload::Collections")
		}
		
		// Clear prefilled data and test again
		assert!(reading_node.clear_prefilled_data().is_ok());
		
		let result = reading_node.compute_objects(&empty_map).await;
		assert!(result.is_ok());
		let payload = result.unwrap();
		
		match payload {
			GraphPayload::Objects(data_map) => {
				assert!(data_map.contains_key(&reading_node.default_output_handle()));
				let value = data_map.get(&reading_node.default_output_handle()).unwrap();
				assert_eq!(*value, serde_json::json!(null));
			}
			_ => panic!("Expected GraphPayload::Objects")
		}
	}
}