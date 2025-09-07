use super::{iot::IoTReading, StoreReading};
use serde::{Deserialize, Serialize};
use serde_json::Number;

#[derive(Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LegacyReading {
    reading: IoTReading,
    base_url: String,
    topic: String,
    sensor_type: String,
    client_account: String,
    site_id: String,
    location: String,
    username: String,
    api_key: String,
}

impl LegacyReading {
    pub fn new(
        reading: IoTReading,
        base_url: String,
        topic: String,
        client_account: String,
        site_id: String,
        location: String,
        sensor_type: String,
        username: String,
        api_key: String,
    ) -> Self {
        LegacyReading {
            reading,
            base_url,
            topic,
            client_account,
            site_id,
            location,
            sensor_type,
            username,
            api_key,
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
struct LegacyResponse {
    status: Number,
    error: bool,
    message: Option<String>,
}

impl StoreReading for LegacyReading {
    async fn store(&self, _: &sea_orm::DatabaseConnection, _: i32) -> Result<(), String> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            // Disable SSL verification for legacy application
            .danger_accept_invalid_certs(true)
            .build()
            .map_err(|e| e.to_string())?;

        let client_account = self.client_account.parse::<i32>().map_err(|e| e.to_string())?;
        
        let body = serde_json::to_string(&serde_json::json!({
            "topic": self.topic,
            "clientAc": client_account,
            "siteId": self.site_id,
            "location": self.location,
            "sensorType": self.sensor_type,
            "identifier": self.reading.identifier(),
            "value": self.reading.get_value(),
            "timestamp": self.reading.timestamp().to_rfc3339(),
            "_username": self.username,
            "_apiKey": self.api_key,
        }))
        .unwrap();

        let res = client
            .post(format!("{}/api/iv/IoT/iothub.php", self.base_url))
            .body(body)
            .send()
            .await
            .map_err(|e| e.to_string())?;

        if !res.status().is_success() {
            return Err(format!("Failed to store reading: {}", res.status()));
        }

        // Parse response
        let response = res.json::<LegacyResponse>().await.map_err(|e| e.to_string())?;
        if response.status != (200).into() {
            return Err(response.message.unwrap_or("Failed to store reading".into()));
        }

        Ok(())
    }
}