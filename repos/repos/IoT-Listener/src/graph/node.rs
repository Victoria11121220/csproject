use super::{
	concrete_node::ConcreteNode,
	types::{ GraphPayload, GraphPayloadCollections, GraphPayloadMixed, NodeData },
	utils::normalization::normalize_inputs,
};
use crate::nodes::NodeType;
use serde::{ self, Deserialize, Deserializer };
use std::{ collections::HashSet, fmt::{ self, Debug }, ops::{ Deref, DerefMut } };

/// Errors that can occur in the nodes
#[derive(Debug, Clone)]
pub enum NodeError {
	/// Use when the computation is not available for the given input and the graph should attempt to fallback to a different input type
	ComputationUnavailableError(String),
	/// Use when the computation fails due to an error in the node's implementation.
	/// This is a generic error that should be used when no other error fits the situation.
	GenericComputationError(String),
	/// Use when the input is invalid in some way.
	/// For example, missing required inputs, invalid input data format, etc.
	InvalidInputError(String),
	/// Use when the output handle is missing from the output of the node.
	/// Or if the node expects a specific output handle to be present in the output.
	MissingOutputHandleError(String),
	/// Use when the input normalization fails.
	/// This is only really emitted when working with mixed payloads and the normalization fails.
	InputNormalizationError(String),
	/// It only used in the default implementation of the node's [`ConcreteNode::compute_collections`] function to indicate that the result of [`ConcreteNode::payload_objects_to_reading`] is invalid.
	InvalidResultInDefaultCollections(String),
	/// Use when the node fails to validate the actual handles.
	/// For example, if the node expects a specific input handle to be present in the input.
	HandleValidationError(String),
	/// Error type returned by sources when the source fails to read the data from the outside world.
	SourceError(String),
	/// Used when a REST API call fails.
	/// Contains the error message from the REST API call.
	RestApiError(String),
}

#[cfg_attr(coverage_nightly, coverage(off))]
impl fmt::Display for NodeError {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			NodeError::ComputationUnavailableError(e) =>
				write!(f, "ComputationUnavailableError: {e}"),
			NodeError::GenericComputationError(e) => write!(f, "GenericComputationError: {e}"),
			NodeError::InvalidInputError(e) => write!(f, "InvalidInputError: {e}"),
			NodeError::MissingOutputHandleError(e) => write!(f, "MissingOutputHandleError: {e}"),
			NodeError::InputNormalizationError(e) => write!(f, "InputNormalizationError: {e}"),
			NodeError::InvalidResultInDefaultCollections(e) =>
				write!(f, "InvalidResultInDefaultCollections: {e}"),
			NodeError::HandleValidationError(e) => write!(f, "HandleValidationError: {e}"),
			NodeError::SourceError(e) => write!(f, "SourceError: {e}"),
			NodeError::RestApiError(e) => write!(f, "RestApiError: {e}"),
		}
	}
}

pub type NodeResult = Result<GraphPayload, NodeError>;

/// A node in the graph
///
/// This struct is a wrapper around a NodeType that adds an id and internal state to the node.
/// It handles most node actions and defers the computation to the NodeType which implements the [`ConcreteNode`] trait.
pub struct Node {
	id: String,
	concrete_node: NodeType,
	internal_state: Option<NodeResult>,
}

// Custom deserialization for a Node
impl<'de> Deserialize<'de> for Node {
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error> where D: Deserializer<'de> {
		#[derive(Deserialize)]
		struct RawNode {
			id: String,
			data: Option<serde_json::Value>,
			r#type: String,
		}

		// Deserialize into a temporary struct
		let raw = RawNode::deserialize(deserializer)?;

		// Create a new JSON object with the type and value
		let data =
			serde_json::json!({
            "type": raw.r#type,
            "value": raw.data,
        });

		// Deserialize the JSON object into a NodeType
		let concrete_node = NodeType::deserialize(data).map_err(serde::de::Error::custom)?;

		Ok(Node {
			id: raw.id,
			concrete_node,
			internal_state: None,
		})
	}
}

impl Deref for Node {
	type Target = NodeType;

	fn deref(&self) -> &Self::Target {
		&self.concrete_node
	}
}

impl DerefMut for Node {
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.concrete_node
	}
}

#[cfg_attr(coverage_nightly, coverage(off))]
impl Debug for Node {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "Node {{ id: {}, concrete_node: {:?} }}", self.id, self.concrete_node)
	}
}

impl Node {
	pub fn id(&self) -> &str {
		&self.id
	}

	/// Returns the associated concrete node type
	pub fn concrete_node(&self) -> &NodeType {
		&self.concrete_node
	}
	
	/// Returns a mutable reference to the associated concrete node type
	/// This is used to update the node's internal state
	pub fn concrete_node_mut(&mut self) -> &mut NodeType {
		&mut self.concrete_node
	}

	pub fn clear(&mut self) {
		self.internal_state = None;
	}

	pub fn get_data(&self) -> &Option<NodeResult> {
		&self.internal_state
	}

	/// Update the node's internal state by computing the output on the collection payload input.
	///
	/// A check is made to ensure that all collections have the same length before calling the node's computation function.
	async fn internal_compute_collections(
		&self,
		collections: &GraphPayloadCollections
	) -> NodeResult {
		// Ensure all collections have the same length
		let collection_lengths = collections
			.values()
			.map(|c| c.len())
			.collect::<HashSet<_>>();
		match collection_lengths.len() {
			| 0 // No handles, still valid input
			| 1 => self.compute_collections(collections).await, // All collections have the same length
			_ => {
				// Collections have different lengths
				// Attempt to normalize the input
				// As the input is already a collection payload, the normalization should either cast all collections to objects, or to a common size, or fail
				let mixed_payload: GraphPayloadMixed = collections
					.iter()
					.map(|(k, v)| (k.clone(), NodeData::Collection(v.clone())))
					.collect();
				self.mixed_input_compute_fallback(&mixed_payload).await
			}
		}
	}

	/// Update the node's internal state by computing the output on the mixed payload input.
	///
	/// This function is a fallback for when the node's computation function is not implemented for mixed payloads.
	/// It attempts to normalize the input and call the appropriate computation function.
	async fn mixed_input_compute_fallback(&self, mixed: &GraphPayloadMixed) -> NodeResult {
		let normalized_inputs = normalize_inputs(mixed)?;
		match normalized_inputs {
			GraphPayload::Objects(objects) => self.compute_objects(&objects).await,
			GraphPayload::Collections(collections) => self.compute_collections(&collections).await,
			// Should never happen as the normalization should always return either objects or collections
			GraphPayload::Mixed(mixed) =>
				unreachable!("Mixed input normalization failed: {:?}", mixed),
		}
	}

	/// Update the node's internal state by computing the output on the given input.
	///
	/// Based on the type of the input, the appropriate computation function is called with the right callback in case of an error.
	pub async fn update(&mut self, inputs: GraphPayload) {
		self.internal_state = Some(match inputs {
			GraphPayload::Objects(objects) => self.compute_objects(&objects).await,
			GraphPayload::Collections(collections) =>
				self.internal_compute_collections(&collections).await,
			GraphPayload::Mixed(mixed) =>
				match self.compute_mixed(&mixed).await {
					Ok(result) => Ok(result),
					Err(NodeError::ComputationUnavailableError(_)) =>
						self.mixed_input_compute_fallback(&mixed).await,
					Err(e) => Err(e),
				}
		});
	}
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
	use super::*;
	use std::collections::HashMap;
	use mockall::predicate;
	use crate::{ graph::types::GraphPayloadObjects, nodes::MockInternalNode };

	fn debug_node() -> Node {
		let node_json =
			r#"{
            "id": "node_id",
            "type": "debug",
            "data": {}
        }"#;

		serde_json::from_str(node_json).unwrap()
	}

	fn mock_node(mock_internal: MockInternalNode) -> Node {
		Node {
			id: "node_id".to_string(),
			concrete_node: NodeType::Mock(mock_internal),
			internal_state: None,
		}
	}

	#[test]
	fn deserialize_node() {
		let node = debug_node();
		assert_eq!(node.id(), "node_id");
		assert!(matches!(node.concrete_node(), NodeType::Debug(_)));
	}

	#[test]
	fn deref() {
		let node = mock_node(MockInternalNode::default());
		assert!(matches!(*node, NodeType::Mock(_)));

		let mut internal_mock = MockInternalNode::new();
		internal_mock.expect_set_actual_handles().return_const(Ok(()));

		let mut node = mock_node(internal_mock);
		assert!(node.set_actual_handles(HashSet::new(), HashSet::new()).is_ok());
	}

	#[tokio::test]
	async fn update_bypass() {
		let canary_objects: GraphPayloadObjects = vec![(
			"canary".to_string(),
			serde_json::json!(1.0),
		)]
			.into_iter()
			.collect();
		let canary_collections: GraphPayloadCollections = vec![(
			"canary".to_string(),
			vec![serde_json::json!(1.0)],
		)]
			.into_iter()
			.collect();
		let canary_mixed: GraphPayloadMixed = vec![(
			"canary".to_string(),
			NodeData::Object(serde_json::json!(1.0)),
		)]
			.into_iter()
			.collect();

		let mut internal_mock = MockInternalNode::new();
		internal_mock
			.expect_compute_objects()
			.with(predicate::eq(canary_objects.clone()))
			.return_const(
				Ok(
					GraphPayload::Objects(
						vec![("valid".to_string(), serde_json::json!(1.0))]
							.into_iter()
							.collect()
					)
				)
			);
		internal_mock
			.expect_compute_collections()
			.with(predicate::eq(canary_collections.clone()))
			.return_const(
				Ok(
					GraphPayload::Objects(
						vec![("valid".to_string(), serde_json::json!(1.0))]
							.into_iter()
							.collect()
					)
				)
			);
		internal_mock
			.expect_compute_mixed()
			.with(predicate::eq(canary_mixed.clone()))
			.return_const(
				Ok(
					GraphPayload::Objects(
						vec![("valid".to_string(), serde_json::json!(1.0))]
							.into_iter()
							.collect()
					)
				)
			);

		let mut node = mock_node(internal_mock);
		for payload in [
			GraphPayload::Objects(canary_objects),
			GraphPayload::Collections(canary_collections),
			GraphPayload::Mixed(canary_mixed),
		] {
			node.clear();
			node.update(payload).await;
			match node.get_data() {
				Some(Ok(GraphPayload::Objects(objects))) => {
					assert_eq!(objects.len(), 1);
					assert_eq!(objects.get("valid").unwrap(), &serde_json::json!(1.0));
				}
				_ => panic!("Expected GraphPayload::Objects"),
			}
		}
	}

	#[tokio::test]
	async fn clear_get_data() {
		let mut internal_mock = MockInternalNode::new();
		internal_mock
			.expect_compute_objects()
			.return_const(Ok(GraphPayload::Objects(HashMap::new())));

		let mut node = mock_node(internal_mock);

		assert!(node.get_data().is_none());
		node.update(GraphPayload::Objects(HashMap::new())).await;
		assert!(matches!(node.get_data(), Some(Ok(GraphPayload::Objects(_)))));
		node.clear();
		assert!(node.get_data().is_none());
	}

	#[tokio::test]
	async fn internal_collections_empty() {
		let mut internal_mock = MockInternalNode::new();
		internal_mock
			.expect_compute_collections()
			.with(predicate::eq(HashMap::new()))
			.times(1)
			.return_const(Ok(GraphPayload::Objects(HashMap::new())));

		let mut node = mock_node(internal_mock);
		node.update(GraphPayload::Collections(HashMap::new())).await;
	}

	#[tokio::test]
	async fn internal_collections_valid() {
		let canary: GraphPayloadCollections = vec![
			("canary".to_string(), vec![serde_json::json!(1.0), serde_json::json!(2.0)]),
			("test".to_string(), vec![serde_json::json!(2.0), serde_json::json!(3.0)])
		]
			.into_iter()
			.collect();

		let mut internal_mock = MockInternalNode::new();
		internal_mock
			.expect_compute_collections()
			.times(1)
			.with(predicate::eq(canary.clone()))
			.returning(|_| Ok(GraphPayload::Objects(HashMap::new())));

		let mut node = mock_node(internal_mock);
		node.update(GraphPayload::Collections(canary)).await;
	}

	#[tokio::test]
	async fn internal_collections_mixed_can_be_normalized() {
		let canary: GraphPayloadCollections = vec![
			("canary".to_string(), vec![serde_json::json!(1.0)]),
			("test".to_string(), vec![serde_json::json!(2.0), serde_json::json!(3.0)])
		]
			.into_iter()
			.collect();

		let mut internal_mock = MockInternalNode::new();
		internal_mock
			.expect_compute_collections()
			.times(1)
			.returning(|collections| {
				let collection_lengths = collections
					.values()
					.map(|c| c.len())
					.collect::<HashSet<_>>();
				match collection_lengths.len() {
					0 | 1 => Ok(GraphPayload::Objects(HashMap::new())), // All collections have the same length or no collections
					_ =>
						Err(
							NodeError::InvalidInputError(
								"Collections have different lengths".to_string()
							)
						), // Collections have different lengths
				}
			});

		let mut node = mock_node(internal_mock);
		node.update(GraphPayload::Collections(canary)).await;
		assert!(matches!(node.get_data(), Some(Ok(GraphPayload::Objects(_)))));
	}

	#[tokio::test]
	async fn update_mixed_err_prop() {
		let mut internal_mock = MockInternalNode::new();
		internal_mock
			.expect_compute_mixed()
			.return_const(Err(NodeError::GenericComputationError("Test".to_string())));

		let mut node = mock_node(internal_mock);
		node.update(GraphPayload::Mixed(HashMap::new())).await;
		match node.get_data() {
			Some(Err(NodeError::GenericComputationError(msg))) => assert_eq!(msg, "Test"),
			_ => panic!("Expected Some(Err(NodeError::GenericComputationError))"),
		}
	}

	#[tokio::test]
	async fn update_mixed_custom_valid() {
		let mut internal_mock = MockInternalNode::new();
		internal_mock.expect_compute_mixed().return_const(
			Ok(
				GraphPayload::Objects(
					vec![("test".to_string(), serde_json::json!(1.0))]
						.into_iter()
						.collect()
				)
			)
		);

		let mut node = mock_node(internal_mock);
		node.update(GraphPayload::Mixed(HashMap::new())).await;
		match node.get_data() {
			Some(Ok(GraphPayload::Objects(objects))) => {
				assert_eq!(objects.len(), 1);
				assert_eq!(objects.get("test").unwrap(), &serde_json::json!(1.0));
			}
			_ => panic!("Expected Some(Ok(GraphPayload::Objects))"),
		}
	}

	#[tokio::test]
	async fn mixed_fallback_to_objects() {
		let canary: GraphPayloadMixed = vec![
			("canary".to_string(), NodeData::Object(serde_json::json!(1.0))),
			("test".to_string(), NodeData::Collection(vec![serde_json::json!(2.0)]))
		]
			.into_iter()
			.collect();

		let expected_objects: GraphPayloadObjects = vec![
			("canary".to_string(), serde_json::json!(1.0)),
			("test".to_string(), serde_json::json!(2.0))
		]
			.into_iter()
			.collect();

		let mut internal_mock = MockInternalNode::new();
		internal_mock
			.expect_compute_mixed()
			.return_const(
				Err(NodeError::ComputationUnavailableError("Mixed input not supported".to_string()))
			);
		internal_mock
			.expect_compute_objects()
			.with(predicate::eq(expected_objects))
			.returning(|_| Ok(GraphPayload::Objects(HashMap::new())));

		let mut node = mock_node(internal_mock);
		node.update(GraphPayload::Mixed(canary)).await;
		match node.get_data() {
			Some(Ok(GraphPayload::Objects(objects))) => assert!(objects.is_empty()),
			_ => panic!("Expected Some(Ok(GraphPayload::Objects))"),
		}
	}

	#[tokio::test]
	async fn mixed_fallback_norm_error() {
		let canary: GraphPayloadMixed = vec![
			(
				"canary".to_string(),
				NodeData::Collection(
					vec![serde_json::json!(1.0), serde_json::json!(2.0), serde_json::json!(3.0)]
				),
			),
			(
				"test".to_string(),
				NodeData::Collection(vec![serde_json::json!(2.0), serde_json::json!(3.0)]),
			)
		]
			.into_iter()
			.collect();

		let mut internal_mock = MockInternalNode::new();
		internal_mock
			.expect_compute_mixed()
			.return_const(
				Err(NodeError::ComputationUnavailableError("Mixed input not supported".to_string()))
			);

		let mut node = mock_node(internal_mock);
		node.update(GraphPayload::Mixed(canary)).await;
		assert!(matches!(node.get_data(), Some(Err(NodeError::InputNormalizationError(_)))));
	}
}