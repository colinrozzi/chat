use crate::bindings::ntwk::theater::http_client::{send_http, HttpRequest};
use crate::bindings::ntwk::theater::runtime::log;
use crate::messages::{AssistantMessage, Message, ModelInfo, LlmMessage};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha1::{Digest, Sha1};

// Helper function to get max tokens for a given model
pub fn get_model_max_tokens(model_id: &str) -> u32 {
    match model_id {
        // Claude models via OpenRouter
        m if m.contains("claude") => 100000,
        // Gemini models via OpenRouter
        m if m.contains("gemini") => 32768,
        // GPT models via OpenRouter
        m if m.contains("gpt-4") => 128000,
        m if m.contains("gpt-3.5") => 16384,
        // Anthropic models via OpenRouter
        m if m.contains("anthropic") => 100000,
        // Mistral models via OpenRouter
        m if m.contains("mistral") => 32768,
        // Default case
        _ => 8192, // Conservative default
    }
}

// Pricing structure for OpenRouter models (these would typically be fetched from the API)
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ModelPricing {
    pub input_cost_per_million_tokens: Option<f64>,
    pub output_cost_per_million_tokens: Option<f64>,
}

// Helper function to get pricing for a given model
pub fn get_model_pricing(model_id: &str) -> ModelPricing {
    // Note: These are approximate values, actual pricing should be obtained from OpenRouter's API
    match model_id {
        // Claude models
        m if m.contains("claude-3-opus") => ModelPricing {
            input_cost_per_million_tokens: Some(15.00),
            output_cost_per_million_tokens: Some(75.00),
        },
        m if m.contains("claude-3-sonnet") => ModelPricing {
            input_cost_per_million_tokens: Some(3.00),
            output_cost_per_million_tokens: Some(15.00),
        },
        m if m.contains("claude-3-haiku") => ModelPricing {
            input_cost_per_million_tokens: Some(0.25),
            output_cost_per_million_tokens: Some(1.25),
        },
        
        // OpenAI models
        m if m.contains("gpt-4-turbo") => ModelPricing {
            input_cost_per_million_tokens: Some(10.00),
            output_cost_per_million_tokens: Some(30.00),
        },
        m if m.contains("gpt-4") => ModelPricing {
            input_cost_per_million_tokens: Some(30.00),
            output_cost_per_million_tokens: Some(60.00),
        },
        m if m.contains("gpt-3.5") => ModelPricing {
            input_cost_per_million_tokens: Some(0.50),
            output_cost_per_million_tokens: Some(1.50),
        },
        
        // Anthropic models
        m if m.contains("anthropic") => ModelPricing {
            input_cost_per_million_tokens: Some(3.00),
            output_cost_per_million_tokens: Some(15.00),
        },
        
        // Gemini models
        m if m.contains("gemini") => ModelPricing {
            input_cost_per_million_tokens: Some(0.35),
            output_cost_per_million_tokens: Some(1.05),
        },
        
        // Mistral models
        m if m.contains("mistral") => ModelPricing {
            input_cost_per_million_tokens: Some(2.00),
            output_cost_per_million_tokens: Some(6.00),
        },
        
        // For unknown models, return None to indicate unknown pricing
        _ => ModelPricing {
            input_cost_per_million_tokens: None,
            output_cost_per_million_tokens: None,
        },
    }
}

// OpenRouter message formats
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct OpenRouterMessage {
    pub role: String,
    pub content: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct OpenRouterRequest {
    pub model: String,
    pub messages: Vec<OpenRouterMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<Value>,
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
    app_name: Option<String>,
    url: Option<String>,
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
    pub fn new(api_key: String, app_name: Option<String>, url: Option<String>) -> Self {
        Self { 
            api_key, 
            app_name, 
            url: url.or(Some("https://openrouter.ai/api/v1".to_string())),
        }
    }

    pub fn list_available_models(&self) -> Result<Vec<ModelInfo>, Box<dyn std::error::Error>> {
        // Construct the URL for the models endpoint
        let url = format!("{}/models", self.url.clone().unwrap_or("https://openrouter.ai/api/v1".to_string()));
        
        let mut headers = vec![
            ("Authorization".to_string(), format!("Bearer {}", self.api_key)),
            ("Content-Type".to_string(), "application/json".to_string()),
        ];
        
        // Add optional headers for app discovery on OpenRouter
        if let Some(app_name) = &self.app_name {
            headers.push(("X-Title".to_string(), app_name.clone()));
        }
        
        let request = HttpRequest {
            method: "GET".to_string(),
            uri: url,
            headers,
            body: None,
        };

        let http_response = send_http(&request).map_err(|e| format!("HTTP request failed: {}", e))?;
        
        // Check if we have a response body
        let body = http_response.body.ok_or("No response body")?;
        
        // Parse the response
        let models_response: Value = serde_json::from_slice(&body)?;
        
        // Log the response for debugging
        log(&format!("OpenRouter models response: {:?}", models_response));
        
        // Parse the models from the response
        let mut models = Vec::new();
        
        if let Some(data) = models_response.get("data").and_then(|d| d.as_array()) {
            for model_data in data {
                if let (Some(id), Some(context_length)) = (
                    model_data.get("id").and_then(|v| v.as_str()),
                    model_data.get("context_length").and_then(|v| v.as_u64()),
                ) {
                    // Get the display name or use the ID if not available
                    let display_name = model_data.get("name")
                        .and_then(|v| v.as_str())
                        .unwrap_or(id);
                    
                    // Add the model to the list
                    models.push(ModelInfo {
                        id: id.to_string(),
                        display_name: display_name.to_string(),
                        max_tokens: context_length as u32,
                        provider: Some("openrouter".to_string()),
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
    ) -> Result<AssistantMessage, Box<dyn std::error::Error>> {
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
                    Message::Assistant(assistant_msg) => assistant_msg.content().to_string(),
                },
            })
            .collect();

        // Get the model ID or use a default
        let model = model_id.unwrap_or_else(|| "anthropic/claude-3-sonnet".to_string());
        
        // Construct the request URL
        let url = format!("{}/chat/completions", self.url.clone().unwrap_or("https://openrouter.ai/api/v1".to_string()));
        
        // Calculate appropriate max_tokens for this model
        let max_tokens = get_model_max_tokens(&model);
        
        // Create request body
        let request_body = OpenRouterRequest {
            model: model.clone(),
            messages: openrouter_messages,
            max_tokens: Some(1024), // Use a reasonable default, can be model-specific
            temperature: Some(0.7), // Standard temperature
            provider: None, // Use default provider routing
        };

        // Set up headers
        let mut headers = vec![
            ("Authorization".to_string(), format!("Bearer {}", self.api_key)),
            ("Content-Type".to_string(), "application/json".to_string()),
        ];
        
        // Add optional headers for app discovery on OpenRouter
        if let Some(app_name) = &self.app_name {
            headers.push(("X-Title".to_string(), app_name.clone()));
            // Could also add HTTP-Referer if we had a URL
        }

        // Create the HTTP request
        let request = HttpRequest {
            method: "POST".to_string(),
            uri: url,
            headers,
            body: Some(serde_json::to_vec(&request_body)?),
        };

        // Send the request
        let http_response = send_http(&request).map_err(|e| format!("HTTP request failed: {}", e))?;
        
        // Log the response for debugging
        log(&format!("OpenRouter response: {:?}", http_response));
        
        // Check if we have a response body
        let body = http_response.body.ok_or("No response body")?;
        
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

        // Get pricing for this model
        let pricing = get_model_pricing(&model);

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
            input_cost_per_million_tokens: pricing.input_cost_per_million_tokens,
            output_cost_per_million_tokens: pricing.output_cost_per_million_tokens,
        };

        // Wrap in the enum
        Ok(AssistantMessage::OpenRouter(openrouter_message))
    }
}

// Implementation of LlmMessage for OpenRouterMessage
impl LlmMessage for OpenRouterLlmMessage {
    fn content(&self) -> &str {
        &self.content
    }

    fn model_id(&self) -> &str {
        &self.model
    }

    fn provider_name(&self) -> &str {
        "openrouter"
    }

    fn input_tokens(&self) -> u32 {
        // Use native token count if available, otherwise use normalized count
        self.usage.native_prompt_tokens.unwrap_or(self.usage.prompt_tokens)
    }

    fn output_tokens(&self) -> u32 {
        // Use native token count if available, otherwise use normalized count
        self.usage.native_completion_tokens.unwrap_or(self.usage.completion_tokens)
    }

    fn calculate_cost(&self) -> f64 {
        // If cost is directly provided in the usage, use that
        if let Some(cost) = self.usage.cost {
            return cost;
        }
        
        // Otherwise calculate based on token counts and pricing
        let input_cost = self.input_cost_per_million_tokens.unwrap_or(5.0)
            * (self.input_tokens() as f64)
            / 1_000_000.0;

        let output_cost = self.output_cost_per_million_tokens.unwrap_or(15.0)
            * (self.output_tokens() as f64)
            / 1_000_000.0;

        input_cost + output_cost
    }

    fn stop_reason(&self) -> &str {
        &self.finish_reason
    }

    fn message_id(&self) -> &str {
        &self.id
    }

    fn provider_data(&self) -> Option<serde_json::Value> {
        Some(serde_json::json!({
            "native_finish_reason": self.native_finish_reason,
            "model": self.model
        }))
    }
}
