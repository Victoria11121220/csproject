// use rust_listener;
// use serde_json::json;

// #[tokio::test]
// async fn test_graph_formation() {
//     let flow = rust_listener::entities::flows::Model {
//         id: 1,
//         name: "flow1".to_string(),
//         nodes: vec![
//             rust_listener::entities::flows::Node {
//                 id: "source".to_string(),
//                 data: json!({
//                     "type": "source",
//                     "name": "source",
//                 })
//             },
//         ],
//         edges: vec![rust_listener::entities::flows::Edge {
//             id: 1,
//             name: "edge1".to_string(),
//             description: "edge1".to_string(),
//             flow_id: 1,
//             from_node_id: 1,
//             to_node_id: 2,
//         }],
//         enabled: true,
//         modified_at: chrono::Utc::now(),
//     };
// }