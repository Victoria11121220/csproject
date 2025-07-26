use serde::Deserialize;

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct MASMonitorSourceConfig {
    _device_id: String,
    device_name: String,
    device_type_id: String,
    interval: u64,
}

impl MASMonitorSourceConfig {
    pub fn new(device_id: String, device_name: String, device_type_id: String, interval: u64) -> Self {
        MASMonitorSourceConfig {
            _device_id: device_id,
            device_name,
            device_type_id,
            interval,
        }
    }

    pub fn device_type_id(&self) -> &str {
        &self.device_type_id
    }
    pub fn device_name(&self) -> &str {
        &self.device_name
    }
    pub fn interval(&self) -> u64 {
        self.interval
    }
}