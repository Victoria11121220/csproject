use crate::graph::{
	concrete_node::ConcreteNode,
	node::{ NodeError, NodeResult },
	types::{ GraphPayload, GraphPayloadObjects, NodeData },
};
use serde::{ Deserialize, Deserializer, Serialize };
use tracing::info;
use std::collections::HashSet;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AverageNode {}

impl<'de> Deserialize<'de> for AverageNode {
	fn deserialize<D>(_deserializer: D) -> Result<Self, D::Error> where D: Deserializer<'de> {
		Ok(AverageNode {})
	}
}

impl ConcreteNode for AverageNode {
	fn set_actual_handles(
		&mut self,
		_input_handles: HashSet<String>,
		_: HashSet<String>
	) -> Result<(), NodeError> {
		Ok(())
	}

	fn generates_reading(&self) -> bool {
		false
	}

	async fn compute_objects(&self, inputs: &GraphPayloadObjects) -> NodeResult {
		let mut sum = 0.0;
		let mut count = 0;

		for value in inputs.values() {
			// FIX: Instead of converting the whole object, look for a "value" field.
			// if let Some(num) = value.get("value").and_then(|v| v.as_f64()) {
			// 	sum += num;
			// 	count += 1;
			// }
			if let Some(v) = value.as_f64() {
				sum += v;
				count += 1;
			}
		}

		if count == 0 {
			return Err(NodeError::InvalidInputError(
				"No inputs with a valid 'value' field found".to_string(),
			));
		}

		let average = sum / (count as f64);

		Ok(
			GraphPayload::Objects(
				vec![(self.default_output_handle(), average.into())].into_iter().collect()
			)
		)
	}

	/// For mixed types, we need to average over the collections and then return the average
	async fn compute_mixed(&self, inputs: &crate::graph::types::GraphPayloadMixed) -> NodeResult {
		let mut sum = 0.0;
		let mut count = 0;

		for value in inputs.values() {
			match value {
				NodeData::Object(obj) => {
					info!("compute_mixed data: {:?}", obj);
					// FIX: Instead of converting the whole object, look for a "value" field.
					// if let Some(num) = obj.get("value").and_then(|v| v.as_f64()) {
					// 	sum += num;
					// 	count += 1;
					// }
					if let Some(v) = obj.as_f64() {
						sum += v;
						count += 1;
					}
				}
				NodeData::Collection(collection) => {
					let mut collection_sum = 0.0;
					let mut collection_count = 0;
					for item in collection {
						// FIX: Instead of converting the whole object, look for a "value" field.
						if let Some(num) = item.get("value").and_then(|v| v.as_f64()) {
							collection_sum += num;
							collection_count += 1;
						}
					}
					if collection_count > 0 {
						sum += collection_sum / (collection_count as f64);
						count += 1;
					}
				}
			}
		}

		if count == 0 {
			// This can still happen if no inputs have a valid "value" field, which is correct.
			return Err(NodeError::InvalidInputError(
				"No inputs with a valid 'value' field found".to_string(),
			));
		}

		let average = sum / (count as f64);

		Ok(
			GraphPayload::Objects(
				vec![(self.default_output_handle(), average.into())].into_iter().collect()
			)
		)
	}
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod average_tests {
	use serde_json::json;

use super::*;
	use crate::graph::types::{GraphPayload, GraphPayloadMixed};

	#[test]
	fn test_deserialise() {
		let json = r#""#;
		let _: AverageNode = serde_json::from_str(json).unwrap();
	}

	#[test]
	fn test_set_actual_handles() {
		let mut node = AverageNode {};
		let input_handles: HashSet<String> = vec!["input1".to_string(), "input2".to_string()]
			.into_iter()
			.collect();
		let output_handles: HashSet<String> = vec!["output".to_string()].into_iter().collect();

		assert!(node.set_actual_handles(input_handles, output_handles).is_ok());
	}

	#[test]
	fn test_generates_readings() {
		let node = AverageNode {};
		assert!(!node.generates_reading());
	}

	#[tokio::test]
	async fn test_compute_objects() {
		let node = AverageNode {};
		let inputs: GraphPayloadObjects = vec![
				("input1".to_string(), 1.0.into()),
				("input2".to_string(), 2.0.into()),
				("input3".to_string(), 3.0.into()),
			]
			.into_iter()
			.collect();

		let result = node.compute_objects(&inputs).await;
		assert!(result.is_ok());
		if let Ok(GraphPayload::Objects(objects)) = result {
			assert_eq!(objects.len(), 1);
			assert_eq!(objects.get(&node.default_output_handle()), Some(&json!(2.0)));
		}

		let inputs: GraphPayloadObjects = vec![].into_iter().collect();
		let result = node.compute_objects(&inputs).await;
		assert!(result.is_err());
		if let Err(NodeError::InvalidInputError(msg)) = result {
			assert_eq!(msg, "No valid numbers found");
		} else {
			panic!("Expected InvalidInputError");
		}
	}

	#[tokio::test]
	async fn test_compute_mixed() {
		let node = AverageNode {};
		let inputs: GraphPayloadMixed = vec![
				("input1".to_string(), NodeData::Object(1.0.into())),
				("input2".to_string(), NodeData::Object(2.0.into())),
				("input3".to_string(), NodeData::Collection(vec![2.0.into(), 4.0.into()])),
			]
			.into_iter()
			.collect();

		let result = node.compute_mixed(&inputs).await;
		assert!(result.is_ok());
		if let Ok(GraphPayload::Objects(objects)) = result {
			assert_eq!(objects.len(), 1);
			assert_eq!(objects.get(&node.default_output_handle()), Some(&json!(2.0)));
		}

		let inputs: GraphPayloadMixed = vec![].into_iter().collect();
		let result = node.compute_mixed(&inputs).await;
		assert!(result.is_err());
		if let Err(NodeError::InvalidInputError(msg)) = result {
			assert_eq!(msg, "No valid numbers found");
		} else {
			panic!("Expected InvalidInputError");
		}
	}
}