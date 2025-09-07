use base64::prelude::*;
use sea_orm::DatabaseConnection;
use serde::{Deserialize, Serialize};
use tracing::info;
use super::StoreReading;

#[derive(Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MASMonitorReading {
    url: String,
    authentication_token: String,
    device_type_id: String,
    device_id: String,
    event_name: String,
    value: serde_json::Map<String, serde_json::Value>,
}

impl MASMonitorReading {
    pub fn new(
        url: String,
        authentication_token: String,
        device_type_id: String,
        device_id: String,
        event_name: String,
        value: serde_json::Map<String, serde_json::Value>,
    ) -> Self {
        Self {
            url,
            authentication_token,
            device_type_id,
            device_id,
            event_name,
            value,
        }
    }
}

impl StoreReading for MASMonitorReading {
    async fn store(&self, _db: &DatabaseConnection, _flow_id: i32) -> Result<(), String> {
        let client = reqwest::Client::builder().timeout(std::time::Duration::from_secs(10))
        .danger_accept_invalid_certs(true).build().map_err(|e| e.to_string())?;

        let url = format!(
            "{}/api/v0002/device/types/{}/devices/{}/events/{}",
            self.url, self.device_type_id, self.device_id, self.event_name
        );
        let body = serde_json::to_string(&self.value).unwrap();

        info!("Body: {}", body);

        let token = BASE64_STANDARD.encode(format!("use-token-auth:{}", self.authentication_token).as_bytes());

        let client = client
            .post(url)
            .body(body)
            .header("Authorization", format!("Basic {}", token))
            .header("Content-Type", "application/json");

        let res = client.send().await.map_err(|e| e.to_string())?;

        if !res.status().is_success() {
            return Err(format!("Failed to store reading: {} {}", res.status(), res.text().await.map_err(|e| e.to_string())?));
        }

        Ok(())
    }
}

// #[cfg(test)]
// mod mas_monitor_reading_tests {
//     use super::*;
//     use serde_json::json;
//     use wiremock::matchers::{method, path};
//     use wiremock::{Mock, MockServer, ResponseTemplate};

//     fn get_reading(server_url: &str) -> MASMonitorReading {
//         MASMonitorReading::new(
//             server_url.to_string(),
//             "test_token".to_string(),
//             "test_device_type".to_string(),
//             "test_device_id".to_string(),
//             "test_event".to_string(),
//             {
//                 let mut map = serde_json::Map::new();
//                 map.insert("key1".to_string(), json!("value1"));
//                 map.insert("key2".to_string(), json!(42));
//                 map.insert("key3".to_string(), json!(true));
//                 map
//             },
//         )
//     }

//     #[test]
//     fn test_serialise_deserialise() {
//         let reading = get_reading("test");
//         let reading_str = serde_json::to_string(&reading).unwrap();
//         let reading_deserialised = serde_json::from_str::<MASMonitorReading>(&reading_str);
//         assert!(reading_deserialised.is_ok());
//     }

//     #[tokio::test]
//     async fn test_store_reading() {
//         let mock_server = MockServer::start().await;
//         let reading = get_reading(&mock_server.uri());

//         let token = BASE64_STANDARD.encode(format!("use-token-auth:{}", reading.authentication_token).as_bytes());

//         let mock_path = format!(
//             "/api/v0002/device/types/{}/devices/{}/events/{}",
//             reading.device_type_id, reading.device_id, reading.event_name
//         );
//         Mock::given(method("POST"))
//             .and(path(&mock_path))
//             .and(wiremock::matchers::body_string(serde_json::to_string(&reading.value).unwrap()))
//             .and(wiremock::matchers::header("Authorization", format!("Basic {}", token)))
//             .respond_with(ResponseTemplate::new(200))
//             .mount(&mock_server)
//             .await;

//         let result = store_reading(&reading).await;
//         assert!(result.is_ok());
//     }

//     #[tokio::test]
//     async fn test_store_reading_error() {
//         let mock_server = MockServer::start().await;
//         let reading = get_reading(&mock_server.uri());

//         let mock_path = format!(
//             "/api/v0002/device/types/{}/devices/{}/events/{}",
//             reading.device_type_id, reading.device_id, reading.event_name
//         );
//         Mock::given(method("POST"))
//             .and(path(&mock_path))
//             .respond_with(ResponseTemplate::new(500))
//             .mount(&mock_server)
//             .await;

//         let result = store_reading(&reading).await;
//         assert!(result.is_err());
//     }
// }