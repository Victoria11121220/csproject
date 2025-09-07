use crate::readings::Readings;
use async_recursion::async_recursion;
use edge::Edge;
use node::{ Node, NodeError, NodeResult };
use concrete_node::ConcreteNode;
use petgraph::{ graph::{ DiGraph, NodeIndex }, visit::EdgeRef };
use std::{ collections::{ HashMap, HashSet }, ops::Deref };
use tokio::sync::RwLock;
use types::GraphPayload;
use utils::{
	payload_utils::{ get_payload_based_on_type, update_result_type, GraphPayloadType },
	propagation::get_affected_nodes,
};
use tracing::info;

pub mod build_graph;
pub mod concrete_node;
pub mod edge;
pub mod node;
pub mod trigger_data;
pub mod types;
mod utils;

pub type PropagationError = (NodeIndex, NodeError);

/// A graph that can be read from and written to concurrently.
///
/// The graph is backed by a [`petgraph::graph::DiGraph`] and contains additional information about the graph structure.
/// It also wraps the graph in a [`tokio::sync::RwLock`] to allow for concurrent reads and writes.
/// require_backprop includes all nodes that need to be recomputed on every backpropagation. This is to ensure nodes such as current timestamp are always recomputed.
#[derive(Debug)]
pub struct RwLockGraph {
	graph: RwLock<DiGraph<Node, Edge>>,
	sinks: HashSet<NodeIndex>,
	output_generators: HashSet<NodeIndex>,
	require_backprop: HashSet<NodeIndex>,
}

impl Deref for RwLockGraph {
	type Target = RwLock<DiGraph<Node, Edge>>;

	fn deref(&self) -> &Self::Target {
		&self.graph
	}
}

impl RwLockGraph {
    /// Propagate a computation through the graph.
    ///
    /// Only the nodes downstream of the trigger node will be recomputed.
    pub async fn backpropagate(
        &mut self,
        trigger: Vec<NodeIndex>
    ) -> (Vec<Readings>, HashMap<String, GraphPayload>, Vec<PropagationError>) {
        self.backpropagate_with_data(trigger, HashMap::new()).await
    }
    
    /// Propagate a computation through the graph with pre-filled source data.
    ///
    /// Only the nodes downstream of the trigger node will be recomputed.
    /// Source data can be pre-filled to avoid calling source.get() during computation.
    pub async fn backpropagate_with_data(
        &mut self,
        trigger: Vec<NodeIndex>,
        source_data: HashMap<String, serde_json::Value>
    ) -> (Vec<Readings>, HashMap<String, GraphPayload>, Vec<PropagationError>) {
        let mut graph = self.write().await;

        // Find all nodes that are affected by this trigger us BFS
        let affected_nodes = get_affected_nodes(trigger, &graph);
        // Find all nodes that need to be cleared from the require_backprop set
        // Done separately from affected_nodes to avoid propagating from sinks not within the affected nodes
        let mut requires_clear = HashSet::new();
        for node in self.require_backprop.iter() {
            let affected = get_affected_nodes([*node].to_vec(), &graph);
            requires_clear.extend(affected);
        }

        // Clear all nodes that would be affected by new data from the trigger node
        affected_nodes.iter().for_each(|node_index| graph[*node_index].clear());
        // Similarly clear all nodes that require backpropagation and the nodes that are affected by them
        requires_clear.iter().for_each(|node_index| graph[*node_index].clear());

        // Pre-populate source nodes with provided data if available
        for node_index in &affected_nodes {
            let node_id = { 
                let node = &graph[*node_index];
                node.id().to_string()
            };
            
            if let Some(node) = graph.node_weight_mut(*node_index) {
                if let crate::nodes::NodeType::Source(source_node) = node.concrete_node_mut() {
                    if let Some(json_data) = source_data.get(&node_id) {
                        match serde_json::from_value(json_data.clone()) {
                            Ok(node_data) => {
                                // Set the prefilled data for use during computation
                                if let Err(e) = source_node.set_prefilled_data(node_data) {
                                    info!("Failed to set prefilled data for node {}: {:?}", node_id, e);
                                }
                            }
                            Err(e) => {
                                info!("Failed to deserialize source data for node {}: {:?}", node_id, e);
                            }
                        }
                    }
                }
            }
        }

        // Backpropagate from all the affected sinks
        // This updates the internal state of all the affected nodes in topological order
        let affected_sinks = self.sinks.intersection(&affected_nodes);
        for sink in affected_sinks {
            let _ = internal_backpropagation(&mut graph, *sink).await;
        }

        // Collect the results from the output generators that were affected by the backpropagation
        let mut readings: Vec<Readings> = Vec::new();
        let mut generic_outputs = HashMap::new();
        let mut errors = Vec::new();

        let affected_output_generators = self.output_generators.intersection(&affected_nodes);
        for output_node_index in affected_output_generators {
            let node = &graph[*output_node_index];
            match node.get_data() {
                None => {
				} // The node did not get computed during back propagation, this can happen if another node in the same sub-tree errored and interrupted propagation
                Some(node_data) =>
                    match node_data {
                        NodeResult::Err(e) => errors.push((*output_node_index, e.clone())),
                        NodeResult::Ok(payload) => {
							info!("Payload: {:?}", payload);
                            if node.generates_reading() {
                                let readings_attempt = match payload {
                                    GraphPayload::Objects(objects) =>
                                        node
                                            .payload_objects_to_reading(node.id(), objects)
                                            .map(|r| vec![r]),
                                    GraphPayload::Collections(collections) =>
                                        node.payload_collections_to_reading(node.id(), collections),
                                    GraphPayload::Mixed(mixed) =>
                                        node.payload_mixed_to_reading(node.id(), mixed),
                                };

                                match readings_attempt {
                                    Ok(rs) => readings.extend(rs),
                                    Err(e) => errors.push((*output_node_index, e)),
                                }
                            } else {
                                generic_outputs.insert(node.id().to_string(), payload.clone());
                            }
                        }
                    }
            }
        }

        // Clear prefilled data after backpropagation to avoid using stale data in future computations
        for node_index in &affected_nodes {
            if let Some(node) = graph.node_weight_mut(*node_index) {
                if let crate::nodes::NodeType::Source(source_node) = node.concrete_node_mut() {
                    let _ = source_node.clear_prefilled_data();
                }
            }
        }

        (readings, generic_outputs, errors)
    }
}

/// Internal function to backpropagate a computation through the graph.
///
/// Because the backpropagation is done recursively, this helper function is needed to handle the recursion.
///
/// For each node it visits, the function first generates the node's input payload by recursively backpropagating from all incoming edges.
/// Building a payload from the output of all connected nodes and mapping output handles to input data and its handles.
#[async_recursion]
async fn internal_backpropagation(
	graph: &mut DiGraph<Node, Edge>,
	node_index: NodeIndex
) -> Result<GraphPayload, PropagationError> {
	match graph[node_index].get_data() {
		Some(data) => data.clone().map_err(|e| (node_index, e)),
		None => {
			// Data for the node does not exist, compute it
			let mut inputs_map = HashMap::new();
			let mut payload_type: Option<GraphPayloadType> = None;

			// Recursively backpropagate from all incoming edges and construct the input payload
			let incoming_edges = graph
				.edges_directed(node_index, petgraph::Direction::Incoming)
				.map(|e| e.id())
				.collect::<Vec<_>>();
			for edge_index in incoming_edges {
				let source_index = graph.edge_endpoints(edge_index).unwrap().0; // The source of the edge, which exists because the edge exists
				let input_payload = internal_backpropagation(graph, source_index).await?; // Recursively backpropagate from the source node
				let output_handle = graph[edge_index].source_handle(); // The handle of the output that this edge is connected to
				let input_data = input_payload.get(output_handle); // The data that the source node produced for this output handle
				match input_data {
					// If the source node did not produce data for this output handle, return an error
					// If this happens something is wrong with the source node, as it should have produced data for this output handle
					None => {
						return NodeResult::Err(
							NodeError::MissingOutputHandleError(
								format!(
									"Missing output handle: {}. Got {:?}",
									output_handle,
									input_payload
								)
							)
						).map_err(|e| (node_index, e));
					}
					Some(data) => {
						let input_handle = graph[edge_index].target_handle(); // The handle of the input that this edge is connected to
						inputs_map.insert(input_handle.clone(), data.clone());
						payload_type = Some(update_result_type(payload_type, &data));
					}
				}
			}

			// Compute the node
			let payload = get_payload_based_on_type(payload_type, inputs_map).map_err(|e| (
				node_index,
				e,
			))?;
			graph[node_index].update(payload).await;
			graph[node_index]
				.get_data()
				.as_ref()
				.unwrap()
				.clone()
				.map_err(|e| (node_index, e))
		}
	}
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
	use serde_json::json;

	use crate::graph::types::NodeData;

	use super::*;

	// Test utility function that returns a source node, using a constant node to provide data
	async fn return_source_with_data(obj: serde_json::Value) -> Node {
		let mut source_node = serde_json
			::from_value::<Node>(
				json!({
            "id": "sourcenode",
            "type": "constant",
            "data": {
                "type": "OBJECT",
                "constant": obj
            }
        })
			)
			.unwrap();
		let input = json!({
            "key1": obj
        });
		source_node.update(
			GraphPayload::Objects(
				vec![(source_node.default_output_handle(), input.clone())].into_iter().collect()
			)
		).await;
		source_node
	}

	#[tokio::test]
	async fn test_source_generation() {
		let obj = json!({
            "test": "test"
        });
		let source_node = return_source_with_data(obj.clone()).await;
		let ret = source_node
			.get_data()
			.as_ref()
			.unwrap()
			.clone()
			.unwrap()
			.get(&source_node.default_output_handle())
			.unwrap();
		match ret {
			NodeData::Object(r) => assert_eq!(r, obj),
			_ => panic!("Source node returned a collection"),
		}
	}

	#[tokio::test]
	async fn test_three_node_propagation() {
		let obj =
			json!({
            "test": {
                "test2": "value"
            }
        });
		let source_node = return_source_with_data(obj.clone()).await;
		let second_key_node = serde_json
			::from_value::<Node>(
				json!({
            "id": "secondnode",
            "type": "key",
            "data": {
                "key": ["test"],
            }
        })
			)
			.unwrap();
		let third_key_node = serde_json
			::from_value::<Node>(
				json!({
            "id": "thirdnode",
            "type": "key",
            "data": {
                "key": ["test2"],
            }
        })
			)
			.unwrap();
		// Add nodes to graph
		let mut graph = DiGraph::new();
		let trigger = graph.add_node(source_node);
		let second_idx = graph.add_node(second_key_node);
		let sink = graph.add_node(third_key_node);

		let edge1 = serde_json::from_value::<Edge>(
			json!({
            "source": "sourcenode",
            "target": "secondnode",
            "sourceHandle": "source",
            "targetHandle": "target"
        })
		);

		let edge2 = serde_json::from_value::<Edge>(
			json!({
            "source": "secondnode",
            "target": "thirdnode",
            "sourceHandle": "source",
            "targetHandle": "target"
        })
		);
		graph.add_edge(trigger, second_idx, edge1.unwrap());
		graph.add_edge(second_idx, sink, edge2.unwrap());
		let mut rwlock_graph = RwLockGraph {
			graph: RwLock::new(graph),
			sinks: HashSet::from([sink]),
			output_generators: HashSet::from([sink]),
			require_backprop: HashSet::new(),
		};
		let (_readings, generic_outputs, _errors) = rwlock_graph.backpropagate(
			[trigger].to_vec()
		).await;
		println!("{:?}", generic_outputs);
		let third_node_result = generic_outputs.get("thirdnode").unwrap();
		match third_node_result {
			GraphPayload::Objects(objects) => {
				let object = objects
					.get(&rwlock_graph.graph.read().await[sink].default_output_handle())
					.unwrap();
				assert_eq!(*object, serde_json::json!("value"));
			}
			_ => panic!("Expected objects"),
		}
	}
}