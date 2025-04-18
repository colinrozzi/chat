use crate::bindings::ntwk::theater::http_client::{send_http, HttpRequest};
use crate::bindings::ntwk::theater::runtime::log;
use crate::messages::{Message, ModelInfo};
use serde::{Deserialize, Serialize};
use sha1::{Digest, Sha1};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct OpenRouterMessage {
    pub role: String,
    pub content: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct OpenRouterRequest {
    pub model: String,
    pub messages: Vec<OpenRouterMessage>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct OpenRouterChoice {
    pub message: OpenRouterChoiceMessage,
    pub finish_reason: String,
    pub index: u32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct OpenRouterChoiceMessage {
    pub role: String,
    pub content: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct OpenRouterUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
    pub native_prompt_tokens: Option<u32>,
    pub native_completion_tokens: Option<u32>,
    pub native_total_tokens: Option<u32>,
    pub cost: Option<f64>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct OpenRouterResponse {
    pub id: String,
    pub model: String,
    pub created: u64,
    pub object: String,
    pub choices: Vec<OpenRouterChoice>,
    pub usage: OpenRouterUsage,
    pub native_finish_reason: Option<String>,
}

// OpenRouter client implementation
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct OpenRouterClient {
    api_key: String,
    url: String,
    model_configs: Vec<ModelInfo>,
}

// Define OpenRouterMessage struct for the messages implementation
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct OpenRouterLlmMessage {
    pub content: String,
    pub id: String,
    pub model: String,
    pub finish_reason: String,
    pub native_finish_reason: Option<String>,
    pub usage: OpenRouterUsage,
    pub input_cost_per_million_tokens: Option<f64>,
    pub output_cost_per_million_tokens: Option<f64>,
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

    pub fn generate_response(
        &self,
        messages: Vec<Message>,
        model_id: String,
    ) -> Result<OpenRouterLlmMessage, Box<dyn std::error::Error>> {
        let model_info = self
            .model_configs
            .iter()
            .find(|m| m.id == model_id)
            .ok_or("Model not found")?;

        // Convert our internal message format to OpenRouter format
        let openrouter_messages: Vec<OpenRouterMessage> = messages
            .iter()
            .map(|msg| OpenRouterMessage {
                role: match msg {
                    Message::User { .. } => "user".to_string(),
                    Message::Assistant(_) => "assistant".to_string(),
                },
                content: match msg {
                    Message::User { content } => content.clone(),
                    Message::Assistant(assistant_msg) => assistant_msg.content.clone(),
                },
            })
            .collect();

        // Construct the request URL
        let url = format!("{}/chat/completions", self.url.clone());

        // Create request body with model-specific parameters
        let request_body = OpenRouterRequest {
            model: model_id.clone(),
            messages: openrouter_messages,
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
        let openrouter_message = OpenRouterLlmMessage {
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
