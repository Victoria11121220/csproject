use super::traits::{async_trait::AsyncSource, SubscriptionResult};
use crate::{graph::types::NodeData, metrics::SOURCE_MESSAGE_COUNT, nodes::sources::SourceConfig};
use futures::StreamExt;
use petgraph::graph::NodeIndex;
use serde::Deserialize;
use std::{
    sync::{Arc, RwLock},
    time::Duration,
};
use tracing::info;
use uuid::Uuid;

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct MQTTSource {
    pub host: String,
    port: u16,
    username: Option<String>,
    password: Option<String>,
    #[serde(skip)]
    data: Arc<RwLock<NodeData>>,
}

impl MQTTSource {
    /// Get the current data stored in the MQTT source
    pub fn get_data(&self) -> Result<NodeData, Box<dyn std::error::Error>> {
        let data = self.data.read().map_err(|_| "Failed to read data")?;
        Ok(data.clone())
    }
    
    /// Update the data stored in the MQTT source
    pub fn update_data(&self, new_data: NodeData) -> Result<(), Box<dyn std::error::Error>> {
        let mut data = self.data.write().map_err(|_| "Failed to write data")?;
        *data = new_data;
        Ok(())
    }
}

impl AsyncSource for MQTTSource {
    fn get_lock(&self) -> Arc<RwLock<NodeData>> {
        Arc::clone(&self.data)
    }

    fn subscribe(
        &self,
        node_index: NodeIndex,
        cancellation_token: &tokio_util::sync::CancellationToken,
        transmitter: tokio::sync::mpsc::UnboundedSender<Vec<NodeIndex>>,
        config: &crate::sources::SourceConfig,
    ) -> SubscriptionResult {
        let mqtt_config = match config {
            SourceConfig::Mqtt(config) => config,
            _ => {
                return Err("Invalid source config".into());
            }
        };

        let topic = mqtt_config.topic().to_owned();

        let uid = format!("mqtt-{}-{}", topic, Uuid::new_v4());

        let host = self.host.clone();

        let connection_string = format!("tcp://{}:{}", host, self.port);
        let create_opts = mqtt::CreateOptionsBuilder::new().server_uri(connection_string.clone()).client_id(&uid).finalize();

        let mut client = mqtt::AsyncClient::new(create_opts).expect("Failed to create client");

        let username = self.username.clone().unwrap_or_default();
        let password = self.password.clone().unwrap_or_default();

        let connection_opts = mqtt::ConnectOptionsBuilder::new()
            .keep_alive_interval(Duration::from_secs(60))
            .connect_timeout(Duration::from_secs(30))
            .user_name(username)
            .password(password)
            .ssl_options(mqtt::SslOptions::new())
            .finalize();

        let stream: mqtt::AsyncReceiver<Option<mqtt::Message>> = client.get_stream(100);

        let cancellation_token = cancellation_token.clone();
        let data_pointer = self.get_lock();

        let handle = tokio::spawn(async move {
            let err = client.connect(connection_opts).wait();
            if let Err(e) = err {
                tracing::error!("Failed to connect to MQTT broker {}:{}", connection_string, e);
                return;
            }
            let err = client.subscribe(&topic, 1).wait();
            if let Err(e) = err {
                tracing::error!("Failed to subscribe to MQTT broker {}:{}", connection_string, e);
                return;
            }

            info!("Subscribed to MQTT Source {:?} with topic {:?}", host, topic);

            let mut stream = stream;
            loop {
                tokio::select! {
                    _ = cancellation_token.cancelled() => {
                        let _ = client.unsubscribe(&topic).wait();
                        let _ = client.disconnect(None).wait();
                        break;
                    },
                    msg = stream.next() => {
                        if let Some(Some(msg)) = msg {
                            SOURCE_MESSAGE_COUNT.with_label_values(&[host.as_str()]).inc();
                            let payload = msg.payload_str().to_string();
                            let value: Result<serde_json::Value, _> = serde_json::from_str(&payload);
                            info!("Received MQTT message: {:?}", value);
                            if value.is_err() {
                                return;
                            }
                            // Set the data
                            *data_pointer.write().unwrap() = NodeData::Object(serde_json::json!({
                                "payload": value.unwrap(),
                                "topic": topic,
                            }));
                            if let Err(e) = transmitter.send([node_index].to_vec()) {
                                tracing::error!("Failed to send message to transmitter: {:?}", e);
                            }
                        } else {
                            break;
                        }
                    }
                }
            }
        });

        Ok(handle)
    }
}