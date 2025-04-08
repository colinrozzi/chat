use crate::bindings::ntwk::theater::http_client::{send_http, HttpRequest};
use crate::bindings::ntwk::theater::runtime::log;
use crate::messages::{Message, Usage};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AnthropicMessage {
    pub role: String,
    pub content: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ClaudeClient {
    api_key: String,
}

impl ClaudeClient {
    pub fn new(api_key: String) -> Self {
        Self { api_key }
    }

    pub fn generate_response(
        &self,
        messages: Vec<Message>,
    ) -> Result<Message, Box<dyn std::error::Error>> {
        let anthropic_messages: Vec<AnthropicMessage> = messages
            .iter()
            .map(|msg| AnthropicMessage {
                role: match msg {
                    Message::User { .. } => "user".to_string(),
                    Message::Assistant { .. } => "assistant".to_string(),
                },
                content: match msg {
                    Message::User { content } => content.clone(),
                    Message::Assistant { content, .. } => content.clone(),
                },
            })
            .collect();

        let request = HttpRequest {
            method: "POST".to_string(),
            uri: "https://api.anthropic.com/v1/messages".to_string(),
            headers: vec![
                ("Content-Type".to_string(), "application/json".to_string()),
                ("x-api-key".to_string(), self.api_key.clone()),
                ("anthropic-version".to_string(), "2023-06-01".to_string()),
            ],
            body: Some(serde_json::to_vec(&json!({
                "model": "claude-3-7-sonnet-20250219",
                "max_tokens": 8192,
                "messages": anthropic_messages,
            }))?),
        };

        let http_response =
            send_http(&request).map_err(|e| format!("HTTP request failed: {}", e))?;
        log(&format!("HTTP response: {:?}", http_response));
        let body = http_response.body.ok_or("No response body")?;
        let response_data: Value = serde_json::from_slice(&body)?;

        // Extract all required fields from the response
        let content = response_data["content"][0]["text"]
            .as_str()
            .ok_or("No content text")?
            .to_string();

        let id = response_data["id"]
            .as_str()
            .ok_or("No message ID")?
            .to_string();

        let model = response_data["model"]
            .as_str()
            .ok_or("No model info")?
            .to_string();

        let stop_reason = response_data["stop_reason"]
            .as_str()
            .ok_or("No stop reason")?
            .to_string();

        let message_type = response_data["type"]
            .as_str()
            .ok_or("No message type")?
            .to_string();

        let stop_sequence = response_data["stop_sequence"].as_str().map(String::from);

        let usage = Usage {
            input_tokens: response_data["usage"]["input_tokens"]
                .as_u64()
                .ok_or("No input tokens")? as u32,
            output_tokens: response_data["usage"]["output_tokens"]
                .as_u64()
                .ok_or("No output tokens")? as u32,
        };

        Ok(Message::Assistant {
            content,
            id,
            model,
            stop_reason,
            stop_sequence,
            message_type,
            usage,
        })
    }
}
