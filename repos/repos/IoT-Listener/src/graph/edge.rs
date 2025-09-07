use serde::Deserialize;

/// A struct representing an edge in a graph.
/// It also holds string name of the handles of the source and target nodes it connects.
#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Edge {
    #[serde(rename = "source")]
    source_str: String,
    #[serde(rename = "target")]
    target_str: String,
    target_handle: String,
    source_handle: String,
}

impl Edge {
    pub fn source_str(&self) -> String {
        self.source_str.clone()
    }

    pub fn target_str(&self) -> String {
        self.target_str.clone()
    }

    pub fn source_handle(&self) -> &String {
        &self.source_handle
    }

    pub fn target_handle(&self) -> &String {
        &self.target_handle
    }

    pub fn update_target_handle(&mut self, new_target_handle: String) {
        self.target_handle = new_target_handle;
    }
}

impl std::fmt::Debug for Edge {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let target_handle: &str = self.target_handle.as_str();
        write!(f, "{} -> {} ({})", self.source_str, self.target_str, target_handle)
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;

    #[test]
    fn test_edge_debug() {
        let edge = Edge {
            source_str: "source".to_string(),
            target_str: "target".to_string(),
            target_handle: "target_handle".to_string(),
            source_handle: "source_handle".to_string(),
        };
        assert_eq!(format!("{:?}", edge), "source -> target (target_handle)");
    }

    #[test]
    fn test_edge_accessors() {
        let edge = Edge {
            source_str: "source".to_string(),
            target_str: "target".to_string(),
            target_handle: "target_handle".to_string(),
            source_handle: "source_handle".to_string(),
        };
        assert_eq!(edge.source_str(), "source");
        assert_eq!(edge.target_str(), "target");
        assert_eq!(edge.source_handle(), "source_handle");
        assert_eq!(edge.target_handle(), "target_handle");
    }

    #[test]
    fn test_edge_clone() {
        let edge = Edge {
            source_str: "source".to_string(),
            target_str: "target".to_string(),
            target_handle: "target_handle".to_string(),
            source_handle: "source_handle".to_string(),
        };
        let edge_clone = edge.clone();
        assert_eq!(edge.source_str, edge_clone.source_str);
        assert_eq!(edge.target_str, edge_clone.target_str);
        assert_eq!(edge.source_handle, edge_clone.source_handle);
        assert_eq!(edge.target_handle, edge_clone.target_handle);
    }

    #[test]
    fn test_edge_deserialization() {
        let edge_str = r#"{"source": "source", "target": "target", "sourceHandle": "source_handle", "targetHandle": "target_handle"}"#;
        let edge: Edge = serde_json::from_str(edge_str).unwrap();
        assert_eq!(edge.source_str, "source");
        assert_eq!(edge.target_str, "target");
        assert_eq!(edge.source_handle, "source_handle");
        assert_eq!(edge.target_handle, "target_handle");
    }
}