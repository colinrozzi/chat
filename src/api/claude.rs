use crate::bindings::ntwk::theater::http_client::{send_http, HttpRequest};
use crate::bindings::ntwk::theater::runtime::log;
use crate::messages::{Message, Usage};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

// Helper function to get max tokens for a given model
fn get_model_max_tokens(model_id: &str) -> u32 {
    match model_id {
        // Claude 3.7 models
        "claude-3-7-sonnet-20250219" => 8192,
        
        // Claude 3.5 models
        "claude-3-5-sonnet-20241022" | "claude-3-5-haiku-20241022" | "claude-3-5-sonnet-20240620" => 8192,
        
        // Claude 3 models
        "claude-3-opus-20240229" => 4096,
        "claude-3-sonnet-20240229" => 4096,
        "claude-3-haiku-20240307" => 4096,
        
        // Claude 2 models
        "claude-2.1" | "claude-2.0" => 4096,
        
        // Default case
        _ => 4096, // Conservative default
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AnthropicMessage {
    pub role: String,
    pub content: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ClaudeClient {
    api_key: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ModelInfo {
    pub id: String,
    pub display_name: String,
    pub max_tokens: u32, // Added max_tokens field
}

impl ClaudeClient {
    pub fn new(api_key: String) -> Self {
        Self { api_key }
    }

    pub fn list_available_models(&self) -> Result<Vec<ModelInfo>, Box<dyn std::error::Error>> {
        let request = HttpRequest {
            method: "GET".to_string(),
            uri: "https://api.anthropic.com/v1/models".to_string(),
            headers: vec![
                ("x-api-key".to_string(), self.api_key.clone()),
                ("anthropic-version".to_string(), "2023-06-01".to_string()),
            ],
            body: None,
        };

        let http_response = send_http(&request).map_err(|e| format!("HTTP request failed: {}", e))?;
        let body = http_response.body.ok_or("No response body")?;
        let models_response: Value = serde_json::from_slice(&body)?;
        
        // Parse the models from the response
        let mut models = Vec::new();
        if let Some(data) = models_response.get("data").and_then(|d| d.as_array()) {
            for model_data in data {
                if let (Some(id), Some(display_name)) = (
                    model_data.get("id").and_then(|v| v.as_str()),
                    model_data.get("display_name").and_then(|v| v.as_str()),
                ) {
                    // Get max tokens based on model ID
                    let max_tokens = get_model_max_tokens(id);
                    
                    models.push(ModelInfo {
                        id: id.to_string(),
                        display_name: display_name.to_string(),
                        max_tokens,
                    });
                }
            }
        }
        
        Ok(models)
    }

    pub fn generate_response(
        &self,
        messages: Vec<Message>,
        model_id: Option<String>,
    ) -> Result<Message, Box<dyn std::error::Error>> {
        // Get the model ID
        let model = model_id.unwrap_or_else(|| "claude-3-7-sonnet-20250219".to_string());
        
        // Get appropriate max_tokens for this model
        let max_tokens = get_model_max_tokens(&model);
        
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
                "model": model,
                "max_tokens": max_tokens,
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
