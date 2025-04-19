use crate::bindings::ntwk::theater::http_client::{send_http, HttpRequest};
use crate::bindings::ntwk::theater::runtime::log;
use crate::messages::{
    openrouter::{OpenRouterMessage, OpenRouterRequest, OpenRouterResponse},
    AssistantMessage, Message, ModelInfo,
};
use mcp_protocol::types::tool::Tool;
use serde::{Deserialize, Serialize};
use sha1::{Digest, Sha1};

// OpenRouter client implementation
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct OpenRouterClient {
    api_key: String,
    url: String,
    model_configs: Vec<ModelInfo>,
}

impl OpenRouterClient {
    pub fn new(api_key: String, model_configs: Vec<ModelInfo>) -> Self {
        if !api_key.is_empty() {
            // Log the first few characters of the API key for debugging, without exposing the entire key
            let visible_part = if api_key.len() > 8 {
                &api_key[0..8]
            } else {
                &api_key
            };
            log(&format!(
                "Initializing OpenRouter client with API key starting with: {}...",
                visible_part
            ));
        } else {
            log("Warning: Empty OpenRouter API key provided");
        }

        Self {
            api_key,
            url: "https://openrouter.ai/api/v1".to_string(),
            model_configs,
        }
    }

    pub fn list_available_models(&self) -> Result<Vec<ModelInfo>, Box<dyn std::error::Error>> {
        Ok(self.model_configs.clone())
    }

    pub fn tools_enabled(&self, model_id: &str) -> bool {
        self.model_configs
            .iter()
            .any(|model| model.id == model_id && model.tools_enabled)
    }

    pub fn generate_response(
        &self,
        messages: Vec<Message>,
        model_id: String,
        available_tools: Option<Vec<Tool>>,
    ) -> Result<AssistantMessage, Box<dyn std::error::Error>> {
        let model_info = self
            .model_configs
            .iter()
            .find(|m| m.id == model_id)
            .ok_or("Model not found")?;

        // Convert our internal message format to OpenRouter format
        let openrouter_messages: Vec<OpenRouterMessage> = messages
            .iter()
            .map(|msg| match msg {
                Message::User(msg) => OpenRouterMessage {
                    role: "user".to_string(),
                    content: msg.content.clone(),
                    tool_call_id: None,
                },
                Message::Assistant(msg) => OpenRouterMessage {
                    role: "assistant".to_string(),
                    content: msg.content.clone(),
                    tool_call_id: None,
                },
                Message::Tool(msg) => OpenRouterMessage {
                    role: "tool".to_string(),
                    content: msg.content.clone(),
                    tool_call_id: Some(msg.tool_call_id.clone()),
                },
            })
            .collect();

        // Construct the request URL
        let url = format!("{}/chat/completions", self.url.clone());

        let tools = if model_info.tools_enabled {
            available_tools
        } else {
            None
        };

        // Create request body with model-specific parameters
        let request_body = OpenRouterRequest {
            model: model_id.clone(),
            messages: openrouter_messages,
            tools,
        };

        // Prepare the request body - log it for debugging
        let request_body_json = serde_json::to_string(&request_body).unwrap_or_default();
        log(&format!("OpenRouter request body: {}", request_body_json));

        // Set up headers
        let headers = vec![
            (
                "Authorization".to_string(),
                format!("Bearer {}", self.api_key),
            ),
            ("Content-Type".to_string(), "application/json".to_string()),
        ];

        // Create the HTTP request
        let request = HttpRequest {
            method: "POST".to_string(),
            uri: url,
            headers,
            body: Some(serde_json::to_vec(&request_body)?),
        };

        log("Sending OpenRouter request...");
        log(&format!("Request: {:?}", request));

        // Send the request
        let http_response =
            send_http(&request).map_err(|e| format!("HTTP request failed: {}", e))?;

        // Log the response for debugging
        log(&format!(
            "OpenRouter response status: {}",
            http_response.status
        ));

        // Check if the response status is not 2xx (success)
        if http_response.status < 200 || http_response.status >= 300 {
            log(&format!("OpenRouter response: {:?}", http_response));
            return Err(
                format!("OpenRouter API error: HTTP status {}", http_response.status).into(),
            );
        }

        // Check if we have a response body
        let body = http_response.body.ok_or("No response body")?;

        // Log a truncated version of the response body for debugging
        let body_preview =
            String::from_utf8_lossy(&body[..std::cmp::min(body.len(), 500)]).to_string();
        log(&format!(
            "OpenRouter response body preview: {}",
            body_preview
        ));

        // Parse the response
        let response: OpenRouterResponse = serde_json::from_slice(&body)?;

        // Log the parsed response
        log(&format!("Parsed OpenRouter response: {:?}", response));

        // Extract the first choice
        if response.choices.is_empty() {
            return Err("No response choices".into());
        }

        let choice = &response.choices[0];
        let content = choice.message.content.clone();

        // Generate a unique ID for the message
        let mut hasher = Sha1::new();
        hasher.update(content.as_bytes());
        let id = format!("{:x}", hasher.finalize());

        // Create our message
        let openrouter_message = AssistantMessage {
            content,
            id,
            model: response.model.clone(),
            finish_reason: choice.finish_reason.clone(),
            native_finish_reason: response.native_finish_reason.clone(),
            usage: response.usage.clone(),
            input_cost_per_million_tokens: model_info.input_cost_per_million_tokens,
            output_cost_per_million_tokens: model_info.output_cost_per_million_tokens,
        };

        // Wrap in the enum
        Ok(openrouter_message)
    }
}
