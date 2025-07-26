use serde::Deserialize;

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct MQTTSourceConfig {
    topic: String,
}

impl MQTTSourceConfig {
    pub fn topic(&self) -> &str {
        &self.topic
    }
}