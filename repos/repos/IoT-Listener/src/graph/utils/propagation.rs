use std::collections::HashSet;
use petgraph::graph::{ DiGraph, NodeIndex };
use crate::graph::{ edge::Edge, node::Node };

/// Finds all nodes that would be affected by new data from the trigger node
pub fn get_affected_nodes(trigger: Vec<NodeIndex>, graph: &DiGraph<Node, Edge>) -> HashSet<NodeIndex> {
    let mut affected_nodes = HashSet::new();
    // This is a simple breadth-first search.
    let mut q = trigger.clone();
    while let Some(current_node) = q.pop() {
        affected_nodes.insert(current_node);
        let children = graph.neighbors_directed(current_node, petgraph::Direction::Outgoing);
        q.extend(children);
    }
    affected_nodes
}
