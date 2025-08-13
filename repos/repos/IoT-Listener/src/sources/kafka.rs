use super::traits::{async_trait::AsyncSource, SubscriptionResult};
use crate::{graph::types::NodeData, metrics::SOURCE_MESSAGE_COUNT, nodes::sources::SourceConfig};
use petgraph::graph::NodeIndex;
use rdkafka::{
    config::ClientConfig,
    consumer::{BaseConsumer, Consumer},
    message::Message,
};
use serde::Deserialize;
use std::{
    sync::{Arc, RwLock},
    time::Duration,
};
use tracing::{error, info};

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct KafkaSource {
    bootstrap_servers: String,
    #[serde(skip)]
    data: Arc<RwLock<NodeData>>,
}

impl KafkaSource {
    pub fn new(bootstrap_servers: String) -> Self {
        KafkaSource {
            bootstrap_servers,
            data: Arc::new(RwLock::new(NodeData::Object(serde_json::Value::Null))),
        }
    }

    pub fn bootstrap_servers(&self) -> &str {
        &self.bootstrap_servers
    }
}

impl AsyncSource for KafkaSource {
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
        let kafka_config = match config {
            SourceConfig::Kafka(config) => config,
            _ => {
                return Err("Invalid source config".into());
            }
        };

        let bootstrap_servers = self.bootstrap_servers.clone();
        let topic = kafka_config.topic().to_owned();
        let group_id = kafka_config.group_id().to_owned();
        let poll_interval = kafka_config.poll_interval();

        let cancellation_token = cancellation_token.clone();
        let data_pointer = self.get_lock();

        info!(
            "Subscribing to Kafka topic {} on {} with group {}",
            topic, bootstrap_servers, group_id
        );

        let handle = tokio::spawn(async move {
            // Create Kafka consumer
            let consumer: BaseConsumer = ClientConfig::new()
                .set("bootstrap.servers", &bootstrap_servers)
                .set("group.id", &group_id)
                .set("enable.auto.commit", "true")
                .set("auto.offset.reset", "latest")
                .create()
                .expect("Failed to create Kafka consumer");

            // Subscribe to topic
            if let Err(e) = consumer.subscribe(&[&topic]) {
                error!("Failed to subscribe to Kafka topic {}: {}", topic, e);
                return;
            }

            info!("Subscribed to Kafka topic: {}", topic);

            loop {
                tokio::select! {
                    _ = cancellation_token.cancelled() => {
                        info!("Kafka consumer cancelled for topic: {}", topic);
                        break;
                    }
                    _ = tokio::time::sleep(Duration::from_secs(poll_interval)) => {
                        // Poll for messages
                        match consumer.poll(Duration::from_millis(100)) {
                            Some(Ok(message)) => {
                                SOURCE_MESSAGE_COUNT.with_label_values(&[&bootstrap_servers]).inc();
                                
                                // Process the message
                                match message.payload_view::<str>() {
                                    Some(Ok(payload)) => {
                                        info!("Received Kafka message from topic {}: {}", topic, payload);
                                        
                                        // Try to parse as JSON
                                        let value: Result<serde_json::Value, _> = serde_json::from_str(payload);
                                        match value {
                                            Ok(json_value) => {
                                                // Set the data
                                                *data_pointer.write().unwrap() = NodeData::Object(serde_json::json!({
                                                    "payload": json_value,
                                                    "topic": topic,
                                                    "partition": message.partition(),
                                                    "offset": message.offset(),
                                                }));
                                                
                                                // Notify the graph engine
                                                if let Err(e) = transmitter.send(vec![node_index]) {
                                                    error!("Failed to send message to transmitter: {:?}", e);
                                                }
                                            }
                                            Err(e) => {
                                                error!("Failed to parse Kafka message as JSON: {}", e);
                                            }
                                        }
                                    }
                                    Some(Err(e)) => {
                                        error!("Failed to decode Kafka message payload: {}", e);
                                    }
                                    None => {
                                        error!("Kafka message payload is empty");
                                    }
                                }
                            }
                            Some(Err(e)) => {
                                error!("Kafka error: {}", e);
                            }
                            None => {
                                // No message available, continue polling
                                continue;
                            }
                        }
                    }
                }
            }

            info!("Kafka consumer stopped for topic: {}", topic);
        });

        Ok(handle)
    }
}