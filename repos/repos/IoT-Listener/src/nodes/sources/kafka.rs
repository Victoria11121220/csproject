use serde::Deserialize;

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct KafkaSourceConfig {
    bootstrap_servers: String,
    topic: String,
    group_id: String,
    /// Polling interval in seconds
    #[serde(default = "default_poll_interval")]
    poll_interval: u64,
}

impl KafkaSourceConfig {
    pub fn bootstrap_servers(&self) -> &str {
        &self.bootstrap_servers
    }

    pub fn topic(&self) -> &str {
        &self.topic
    }

    pub fn group_id(&self) -> &str {
        &self.group_id
    }

    pub fn poll_interval(&self) -> u64 {
        self.poll_interval
    }
}

fn default_poll_interval() -> u64 {
    5 // Default to 5 seconds
}