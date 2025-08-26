use serde::Deserialize;

#[derive(Deserialize, Debug, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct HTTPSourceConfig;