// Test to verify that MQTT source data can be accessed and updated
use rust_listener::sources::mqtt::MQTTSource;
use rust_listener::graph::types::NodeData;
use std::sync::{Arc, RwLock};

#[tokio::test]
async fn test_mqtt_source_data_access() {
    // Create an MQTT source
    let mqtt_source = MQTTSource {
        host: "localhost".to_string(),
        port: 1883,
        username: None,
        password: None,
        data: Arc::new(RwLock::new(NodeData::Object(serde_json::json!({"test": "data"})))),
    };
    
    // Test getting data
    let data = mqtt_source.get_data().unwrap();
    match data {
        NodeData::Object(value) => {
            assert_eq!(value["test"], "data");
        }
        _ => panic!("Expected Object data"),
    }
    
    // Test updating data
    let new_data = NodeData::Object(serde_json::json!({"updated": "value"}));
    mqtt_source.update_data(new_data.clone()).unwrap();
    
    // Verify the data was updated
    let updated_data = mqtt_source.get_data().unwrap();
    match updated_data {
        NodeData::Object(value) => {
            assert_eq!(value["updated"], "value");
        }
        _ => panic!("Expected Object data"),
    }
}