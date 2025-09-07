use crate::nodes::NodeType;

use super::edge::Edge;
use super::node::Node;
use super::concrete_node::ConcreteNode;
use super::RwLockGraph;
use petgraph::graph::DiGraph;
use std::collections::{ HashMap, HashSet };
use std::env;
use tokio::sync::RwLock;

/// Generate a graph form a JSON string of nodes and edges.
/// The graph is guaranteed to be a DAG.
/// However, it might contain multiple connected components.
///
/// Errors:
/// - If the graph is not a DAG
/// - If a source or target node for an edge is not found
/// - Edge or node deserialization
/// - Any error from [`Node::set_actual_handles`] for handle validation
fn from_flow(nodes_str: &str, edges_str: &str) -> Result<RwLockGraph, Box<dyn std::error::Error>> {
	let nodes: Vec<Node> = serde_json::from_str(nodes_str)?;
	let mut edges: Vec<Edge> = serde_json::from_str(edges_str)?;
	// For duplicate target handles (i.e. multiple edges to the same target handle),
	// we need to ensure that the target handle is unique in the graph.
	// Done by using a hashmap to store the target handles and the number of occurrences.
	let mut target_handle_map = std::collections::HashMap::new();
	for edge in edges.iter_mut() {
		let target_node = edge.target_str();
		let target_hashmap = target_handle_map
			.entry(target_node.clone())
			.or_insert_with(HashMap::new);
		let target_handle = edge.target_handle();
		let count = target_hashmap.entry(target_handle.clone()).or_insert(0);
		*count += 1;
		if *count > 1 {
			// If the target handle is not unique, we need to update it
			// to ensure that it is unique in the graph.
			// This is done by appending the count to the target handle.
			edge.update_target_handle(format!("{target_handle}{count}"));
		}
	}

	let mut graph = DiGraph::new();

	for node in nodes {
		graph.add_node(node);
	}

	// Create edges and insert into graph
	for edge in edges.iter() {
		// Find source and target nodes for the edge
		let source = graph.node_indices().find(|i| graph[*i].id() == edge.source_str());
		let target = graph.node_indices().find(|i| graph[*i].id() == edge.target_str());

		// If both source and target nodes are found, add the edge to the graph
		if let (Some(source), Some(target)) = (source, target) {
			graph.add_edge(source, target, edge.clone());
		} else {
			return Err(format!("Failed to find source or target node for edge {:?}", edge).into());
		}
	}

	// Check if the graph is a DAG
	if petgraph::algo::is_cyclic_directed(&graph) {
		return Err("Graph is not a DAG".into());
	}

	// Validate all handle connections
	for node_index in graph.node_indices() {
		let input_handles = graph
			.edges_directed(node_index, petgraph::Direction::Incoming)
			.map(|e| e.weight().target_handle().clone())
			.collect::<HashSet<_>>();
		let output_handles = graph
			.edges_directed(node_index, petgraph::Direction::Outgoing)
			.map(|e| e.weight().source_handle().clone())
			.collect::<HashSet<_>>();

		let node = graph.node_weight_mut(node_index).unwrap();
		if let Err(e) = node.set_actual_handles(input_handles, output_handles) {
			return Err(
				format!("Failed to set actual handles for node {:?}: {:?}", node.id(), e).into()
			);
		}
	}

	// All nodes with no incoming edges where graph backpropagation will stop.
	// Ideally all nodes that generate data from external sources will be in this set.
	// However more nodes might be added to this set if they are not connected to the graph.
	// This could be the case for disconnected components or subgraphs.

	// All nodes with no outgoing edges where graph backpropagation will start from.
	// Propagating from sinks ensures that node data is used only after it is generated.
	// It achieves the same effect as topological sorting.
	let sinks = graph.externals(petgraph::Direction::Outgoing).collect::<HashSet<_>>();
	// All nodes that generate data that should escape the graph.
	// This includes all nodes that generate data for reading and all sinks.
	let output_generators = graph
		.node_indices()
		.filter(|i| graph[*i].generates_reading())
		.collect::<HashSet<_>>()
		.union(&sinks)
		.cloned()
		.collect::<HashSet<_>>();

	// All nodes that require backpropagation on every update
	let require_backprop = graph
		.node_indices()
		.filter(|i| matches!(graph[*i].concrete_node(), NodeType::CurrentTimestamp(_)))
		.collect::<HashSet<_>>();

	Ok(RwLockGraph {
		graph: RwLock::new(graph),
		sinks,
		output_generators,
		require_backprop,
	})
}

/// Read the nodes and edges from the environment variables and build a graph.
///
/// Errors:
/// - If the `nodes` environment variable is not set
/// - If the `edges` environment variable is not set
/// - If the `flow_id` environment variable is not set or cannot be parsed to an integer
/// - Any error from [`from_flow`]
///
/// Returns:
/// A tuple of the flow ID and the graph
pub fn read_graph() -> Result<(i32, RwLockGraph), Box<dyn std::error::Error>> {
	// Log all environment variables
	let nodes: String = env::var("nodes").expect("`nodes` must be set");
	let edges: String = env::var("edges").expect("`edges` must be set");
	let flow_id: String = env::var("flow_id").expect("`flow_id` must be set");
	let flow_id = flow_id.trim().parse::<i32>()?;
	let graph = from_flow(&nodes, &edges)?;

	Ok((flow_id, graph))
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
	use super::*;

	#[tokio::test]
	async fn test_from_flow() {
		// struct RawNode {
		//     id: String,
		//     data: Option<serde_json::Value>,
		//     r#type: String,
		// }
		let nodes_str =
			r#"[{"id":"1","data":{ "key": ["test"] },"type":"key"},{"id":"2","data":{ "key": ["test"] },"type":"key"}]"#;
		let edges_str =
			r#"[{"source":"1","target":"2", "sourceHandle":"source","targetHandle":"target"}]"#;
		let graph = from_flow(nodes_str, edges_str);
		println!("{:?}", graph);
		assert!(graph.is_ok());
		let graph = graph.unwrap();
		let graph = graph.graph.read().await;
		assert_eq!(graph.node_count(), 2);
		assert_eq!(graph.edge_count(), 1);
	}

	#[tokio::test]
	async fn test_no_node_for_edge() {
		let nodes_str = r#"[{"id":"1","data":{ "key": ["test"] },"type":"key"}]"#;
		let edges_str =
			r#"[{"source":"1","target":"2", "sourceHandle":"source","targetHandle":"target"}]"#;
		let graph = from_flow(nodes_str, edges_str);
		assert!(graph.is_err());
		assert_eq!(
			graph.unwrap_err().to_string(),
			"Failed to find source or target node for edge 1 -> 2 (target)"
		);
	}

	#[tokio::test]
	async fn test_not_a_dag() {
		let nodes_str =
			r#"[{"id":"1","data":{ "key": ["test"] },"type":"key"},{"id":"2","data":{ "key": ["test"] },"type":"key"}]"#;
		let edges_str =
			r#"[{"source":"1","target":"2", "sourceHandle":"source","targetHandle":"target"},{"source":"2","target":"1", "sourceHandle":"source","targetHandle":"target"}]"#;
		let graph = from_flow(nodes_str, edges_str);
		assert!(graph.is_err());
		assert_eq!(graph.unwrap_err().to_string(), "Graph is not a DAG");
	}

	#[tokio::test]
	async fn test_invalid_handles() {
		// A key node must only have one input handle therefore a test with two input handles should fail
		let nodes_str =
			r#"[{"id":"1","data":{ "key": ["test"] },"type":"key"},{"id":"2","data":{ "key": ["test"] },"type":"key"}]"#;
		let edges_str =
			r#"[{"source":"1","target":"2", "sourceHandle":"source","targetHandle":"target"},{"source":"1","target":"2", "sourceHandle":"source2","targetHandle":"target"}]"#;
		let graph = from_flow(nodes_str, edges_str);
		assert!(graph.is_err());
	}
}