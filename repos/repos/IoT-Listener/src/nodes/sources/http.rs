use serde::Deserialize;

#[derive(Deserialize, Debug, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct HTTPSourceConfig {
    interval: u64,
}

impl HTTPSourceConfig {
    pub fn interval(&self) -> u64 {
        self.interval
    }
}