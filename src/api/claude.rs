use crate::api::LlmApi;
use crate::bindings::ntwk::theater::http_client::{send_http, HttpRequest};
use crate::bindings::ntwk::theater::runtime::log;
use crate::messages::{
    AssistantMessage, ClaudeMessage, FunctionDefinition, LlmMessage, Message, ModelInfo, Usage,
};
use mcp_protocol::types::tool::Tool;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

// Helper function to get max tokens for a given model
pub fn get_model_max_tokens(model_id: &str) -> u32 {
    match model_id {
        // Claude 3.7 models
        "claude-3-7-sonnet-20250219" => 8192,

        // Claude 3.5 models
        "claude-3-5-sonnet-20241022"
        | "claude-3-5-haiku-20241022"
        | "claude-3-5-sonnet-20240620" => 8192,

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

// Pricing structure for Claude models
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ModelPricing {
    pub input_cost_per_million_tokens: Option<f64>,
    pub output_cost_per_million_tokens: Option<f64>,
}

// Helper function to get pricing for a given model
pub fn get_model_pricing(model_id: &str) -> ModelPricing {
    match model_id {
        // Claude 3.7 models
        "claude-3-7-sonnet-20250219" => ModelPricing {
            input_cost_per_million_tokens: Some(3.00),
            output_cost_per_million_tokens: Some(15.00),
        },

        // Claude 3.5 models
        "claude-3-5-sonnet-20241022" | "claude-3-5-sonnet-20240620" => ModelPricing {
            input_cost_per_million_tokens: Some(3.00),
            output_cost_per_million_tokens: Some(15.00),
        },
        "claude-3-5-haiku-20241022" => ModelPricing {
            input_cost_per_million_tokens: Some(0.80),
            output_cost_per_million_tokens: Some(4.00),
        },

        // Claude 3 models
        "claude-3-opus-20240229" => ModelPricing {
            input_cost_per_million_tokens: Some(15.00),
            output_cost_per_million_tokens: Some(75.00),
        },
        "claude-3-haiku-20240307" => ModelPricing {
            input_cost_per_million_tokens: Some(0.25),
            output_cost_per_million_tokens: Some(1.25),
        },
        "claude-3-sonnet-20240229" => ModelPricing {
            input_cost_per_million_tokens: Some(3.00),
            output_cost_per_million_tokens: Some(15.00),
        },

        // For older or unknown models, return None to indicate unknown pricing
        _ => ModelPricing {
            input_cost_per_million_tokens: None,
            output_cost_per_million_tokens: None,
        },
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum AnthropicMessage {
    ChatMessage {
        role: String,
        content: String,
    },
    ToolResult {
        #[serde(rename = "type")]
        _type: String,
        tool_use_id: String,
        content: String,
    },
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ClaudeClient {
    api_key: String,
    model_configs: Vec<ModelInfo>,
}

impl LlmApi for ClaudeClient {
    fn new(api_key: String, model_configs: Vec<ModelInfo>) -> Self {
        Self {
            api_key,
            model_configs,
        }
    }

    fn tools_enabled(&self, model_id: &str) -> bool {
        self.model_configs
            .iter()
            .any(|model| model.id == model_id && model.tools_enabled)
    }

    fn list_available_models(&self) -> Result<Vec<ModelInfo>, Box<dyn std::error::Error>> {
        Ok(self.model_configs.clone())
    }

    fn generate_response(
        &self,
        messages: Vec<Message>,
        model_id: String,
        available_tools: Option<Vec<Tool>>,
    ) -> Result<AssistantMessage, Box<dyn std::error::Error>> {
        // Get appropriate max_tokens for this model
        let max_tokens = get_model_max_tokens(&model_id);

        let anthropic_messages: Vec<AnthropicMessage> = messages
            .iter()
            .map(|msg| match msg {
                Message::User(msg) => AnthropicMessage::ChatMessage {
                    role: "user".to_string(),
                    content: msg.content.clone(),
                },
                Message::Assistant(msg) => AnthropicMessage::ChatMessage {
                    role: "assistant".to_string(),
                    content: msg.content().clone().to_string(),
                },
                Message::Tool(msg) => AnthropicMessage::ToolResult {
                    _type: "tool_result".to_string(),
                    tool_use_id: msg.tool_call_id.clone(),
                    content: msg.content.clone(),
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
                "model": model_id,
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

        // Get pricing information for this model
        let pricing = get_model_pricing(&model);

        // Create the Claude message
        let claude_message = ClaudeMessage {
            content,
            id,
            model,
            stop_reason,
            stop_sequence,
            message_type,
            usage,
            input_cost_per_million_tokens: pricing.input_cost_per_million_tokens,
            output_cost_per_million_tokens: pricing.output_cost_per_million_tokens,
        };

        // Wrap in the enum
        Ok(AssistantMessage::Claude(claude_message))
    }
}

// Implementation of LlmMessage for ClaudeMessage
impl LlmMessage for ClaudeMessage {
    fn content(&self) -> &str {
        &self.content
    }

    fn model_id(&self) -> &str {
        &self.model
    }

    fn provider_name(&self) -> &str {
        "claude"
    }

    fn input_tokens(&self) -> u32 {
        self.usage.input_tokens
    }

    fn output_tokens(&self) -> u32 {
        self.usage.output_tokens
    }

    fn calculate_cost(&self) -> f64 {
        let input_cost = self.input_cost_per_million_tokens.unwrap_or(3.0)
            * (self.usage.input_tokens as f64)
            / 1_000_000.0;

        let output_cost = self.output_cost_per_million_tokens.unwrap_or(15.0)
            * (self.usage.output_tokens as f64)
            / 1_000_000.0;

        input_cost + output_cost
    }

    fn stop_reason(&self) -> &str {
        &self.stop_reason
    }

    fn message_id(&self) -> &str {
        &self.id
    }

    fn provider_data(&self) -> Option<serde_json::Value> {
        Some(serde_json::json!({
            "stop_sequence": self.stop_sequence,
            "message_type": self.message_type
        }))
    }
}
