use std::{ collections::HashSet, env };
use crate::graph::{
    concrete_node::ConcreteNode,
    node::{ NodeError, NodeResult },
    types::{ GraphPayload, GraphPayloadObjects },
};
use reqwest::{Error, Response};
use serde::Deserialize;

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct LoRaWANNode {
    codec: String,
    format: String,
}

impl LoRaWANNode {
    /// Function for sending data to the lorawan decoder, returning the reqwest response or reqwest error
    /// Builds the url from the DECODER_URL environment variable, or defaults to "http://lorawan-decoder"
    /// Sends a POST request to the /decode/{codec} endpoint with the format and data query parameters
    async fn send_data(&self, data: &serde_json::Value) -> Result<Response, Error> {
        let client = reqwest::Client::builder().timeout(std::time::Duration::from_secs(1)).build()?;
        let url = format!(
            "{}/decode/{}",
            env::var("DECODER_URL").unwrap_or("http://lorawan-decoder".to_string()),
            self.codec
        );

        client
            .post(&url)
            .query(
                &[
                    ("format", self.format.clone()),
                    ("data", match data.is_string() {
                        true => data.as_str().unwrap().to_string(),
                        false => data.to_string()
                    })
                ]
            )
            .send().await
    }

    /// Function for parsing the response from the lorawan decoder
    /// Maps the response to a serde_json::Value or returns a NodeError
    async fn parse_response(response: Response) -> Result<serde_json::Value, NodeError> {
        if response.status().is_success() {
            response
                .json().await
                .map_err(|e| NodeError::RestApiError(format!("Error decoding response: {:?}", e)))
        } else {
            Err(NodeError::RestApiError("Response was not successful".to_string()))
        }
    }
}

impl ConcreteNode for LoRaWANNode {
    fn set_actual_handles(
        &mut self,
        input_handles: HashSet<String>,
        output_handles: HashSet<String>
    ) -> Result<(), NodeError> {
        if input_handles.len() > 1 {
            return Err(
                NodeError::HandleValidationError(
                    "LoRaWANNode must not have more than one input handle".to_string()
                )
            );
        }
        if output_handles.len() > 1 {
            return Err(
                NodeError::HandleValidationError(
                    "LoRaWANNode must not have more than one output handle".to_string()
                )
            );
        }
        Ok(())
    }

    fn generates_reading(&self) -> bool {
        false
    }

    async fn compute_objects(&self, inputs: &GraphPayloadObjects) -> NodeResult {
        if inputs.len() != 1 {
            return Err(
                crate::graph::node::NodeError::HandleValidationError(
                    "LoRaWANNode must have exactly one input handle".to_string()
                )
            );
        }

        let input = inputs.values().next().unwrap();

        let response = self
            .send_data(input).await
            .map_err(|e| NodeError::RestApiError(format!("Error sending request: {:?}", e)))?;
        let response = Self::parse_response(response).await?;

        Ok(
            GraphPayload::Objects(
                vec![(self.default_output_handle(), response.clone())].into_iter().collect()
            )
        )
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod lorawan_tests {
    use wiremock::MockServer;

    use crate::graph::types::NodeData;

    use super::*;

    #[test]
    fn deserialise() {
        let valid_node_strings = vec![
            "{\"codec\":\"cayenne-lpp\",\"format\":\"hex\"}",
            "{\"codec\":\"cayenne-lpp\",\"format\":\"base64\"}",
            "{\"codec\":\"cayenne-lpp\",\"format\":\"json\"}",
        ];

        for node_string in valid_node_strings {
            let node = serde_json::from_str::<LoRaWANNode>(node_string);
            if let Err(e) = node {
                panic!("Failed to deserialize LoRaWANNode from string: {}", e);
            }
        }
    }

    #[test]
    fn deserialise_invalid() {
        let invalid_node_strings = vec![
            "{\"format\":\"hex\"}",
            "{\"codec\":\"cayenne-lpp\"}",
        ];

        for node_string in invalid_node_strings {
            let node = serde_json::from_str::<LoRaWANNode>(node_string);
            if node.is_ok() {
                panic!("Deserialized LoRaWANNode from invalid string");
            }
        }
    }

    #[test]
    fn set_actual_handles() {
        let mut node = LoRaWANNode {
            codec: "cayenne-lpp".to_string(),
            format: "hex".to_string(),
        };

        let mut input_handles = HashSet::new();
        input_handles.insert("input".to_string());
        let mut output_handles = HashSet::new();
        output_handles.insert("output".to_string());

        let result = node.set_actual_handles(input_handles.clone(), output_handles.clone());
        assert!(result.is_ok());

        input_handles.insert("input2".to_string());
        let result = node.set_actual_handles(input_handles.clone(), output_handles.clone());
        assert!(result.is_err());

        input_handles.remove("input2");
        output_handles.insert("output2".to_string());
        let result = node.set_actual_handles(input_handles.clone(), output_handles.clone());
        assert!(result.is_err());
    }

    #[test]
    fn generates_reading() {
        let node = LoRaWANNode {
            codec: "cayenne-lpp".to_string(),
            format: "hex".to_string(),
        };
        assert!(!node.generates_reading());
    }

    async fn server_and_node() -> (MockServer, LoRaWANNode) {
        let server = MockServer::start().await;
        let node = LoRaWANNode {
            codec: "cayenne-lpp".to_string(),
            format: "hex".to_string(),
        };
        env::set_var("DECODER_URL", server.uri());
        (server, node)
    }

    #[tokio::test]
    async fn compute_objects_empty() {
        let node = LoRaWANNode {
            codec: "cayenne-lpp".to_string(),
            format: "hex".to_string(),
        };

        let inputs = GraphPayloadObjects::new();
        let result = node.compute_objects(&inputs).await;
        assert!(matches!(result, Err(NodeError::HandleValidationError(_))));
    }

    //The following tests are all in one function to avoid creating new instances of MockServer
    //This causes tests to fail due to the server being already in use
    #[tokio::test]
    async fn compute_objects() {
        let (server, node) = server_and_node().await;

        let response = serde_json::json!({
            "temperature": 25.0,
            "humidity": 50.0,
        });

        {
            let _mock = wiremock::Mock::given(wiremock::matchers::method("POST"))
            .and(wiremock::matchers::path("/decode/cayenne-lpp"))
            .respond_with(wiremock::ResponseTemplate::new(200).set_body_json(response.clone()))
            .mount_as_scoped(&server)
            .await;

            let inputs = GraphPayloadObjects::from([("input".to_string(), serde_json::json!("0100"))]);
            let result = node.compute_objects(&inputs).await;
            assert!(matches!(result, Ok(GraphPayload::Objects(_))));
            let result = result.unwrap().get(&node.default_output_handle()).unwrap();
            match result {
                NodeData::Object(value) => assert_eq!(value, response),
                _ => panic!("Unexpected NodeData variant"),
            }
        }

        {
            let _mock = wiremock::Mock::given(wiremock::matchers::method("POST"))
            .and(wiremock::matchers::path("/decode/cayenne-lpp"))
            .respond_with(wiremock::ResponseTemplate::new(400))
            .mount_as_scoped(&server)
            .await;
        
            let inputs = GraphPayloadObjects::from([("input".to_string(), serde_json::json!("0100"))]);

            let result = node.compute_objects(&inputs).await;
            assert!(matches!(result, Err(NodeError::RestApiError(_))));
        }

        {
            let _mock = wiremock::Mock::given(wiremock::matchers::method("POST"))
            .and(wiremock::matchers::path("/decode/cayenne-lpp"))
            .respond_with(wiremock::ResponseTemplate::new(200).set_body_json(serde_json::json!({"temperature": 25.0})))
            .mount_as_scoped(&server)
            .await;

            let client = reqwest::Client::new();
            let response = client
                .post(format!("{}/decode/cayenne-lpp", server.uri()))
                .send()
                .await
                .expect("Failed to send request");

            let result = LoRaWANNode::parse_response(response).await;
            assert_eq!(result.unwrap(), serde_json::json!({"temperature": 25.0}));
        }

        {
            let _mock = wiremock::Mock::given(wiremock::matchers::method("POST"))
            .and(wiremock::matchers::path("/decode/cayenne-lpp"))
            .respond_with(wiremock::ResponseTemplate::new(200).set_body_string("not very jsony"))
            .mount_as_scoped(&server)
            .await;

            let client = reqwest::Client::new();
            let response = client
                .post(format!("{}/decode/cayenne-lpp", server.uri()))
                .send()
                .await
                .expect("Failed to send request");

            let result = LoRaWANNode::parse_response(response).await;
            assert!(matches!(result, Err(NodeError::RestApiError(_))));
        }
        
    }

}