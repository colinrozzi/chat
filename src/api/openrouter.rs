use crate::bindings::ntwk::theater::http_client::{send_http, HttpRequest};
use crate::bindings::ntwk::theater::runtime::log;
use crate::messages::{AssistantMessage, LlmMessage, Message, ModelInfo};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha1::{Digest, Sha1};

// Helper function to check if a model ID is for Llama 4 Maverick free
pub fn is_llama4_maverick_free(model_id: &str) -> bool {
    model_id == "meta-llama/llama-4-maverick:free"
        || model_id == "llama-4-maverick:free"
        || model_id == "llama-4-maverick-free"
}

// Helper function to check if a model ID is for any free model
pub fn is_free_model(model_id: &str) -> bool {
    // Check if it's Llama 4 Maverick free
    if is_llama4_maverick_free(model_id) {
        return true;
    }

    // Check for the :free suffix which indicates a free model
    if model_id.ends_with(":free") {
        return true;
    }

    // List of other free model IDs that don't follow the :free pattern
    let free_models = [
        "deepseek/deepseek-v3-base:free",
        "qwen/qwen2.5-vl-3b-instruct:free",
        "qwen/qwen2.5-vl-32b-instruct:free",
    ];

    free_models.contains(&model_id)
}

// Pricing structure for OpenRouter models (these would typically be fetched from the API)
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ModelPricing {
    pub input_cost_per_million_tokens: Option<f64>,
    pub output_cost_per_million_tokens: Option<f64>,
}

// Helper function to get pricing for a given model
pub fn get_model_pricing(model_id: &str) -> ModelPricing {
    if is_free_model(model_id) {
        return ModelPricing {
            input_cost_per_million_tokens: Some(0.0),  // Free model
            output_cost_per_million_tokens: Some(0.0), // Free model
        };
    }

    // Model-specific pricing
    match model_id {
        "openrouter/quasar-alpha" => ModelPricing {
            input_cost_per_million_tokens: Some(10.0), // Example pricing
            output_cost_per_million_tokens: Some(30.0), // Example pricing
        },
        "openrouter/optimus-alpha" => ModelPricing {
            input_cost_per_million_tokens: Some(10.0), // Example pricing
            output_cost_per_million_tokens: Some(30.0), // Example pricing
        },
        _ => {
            // Default pricing for other models
            ModelPricing {
                input_cost_per_million_tokens: None,
                output_cost_per_million_tokens: None,
            }
        }
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
            app_name,
            url: url.or(Some("https://openrouter.ai/api/v1".to_string())),
        }
    }

    pub fn list_available_models(&self) -> Result<Vec<ModelInfo>, Box<dyn std::error::Error>> {
        // Instead of querying the API for all models, return a hardcoded list of models
        log("[DEBUG] Using hardcoded models instead of querying all OpenRouter models");

        // Create a hardcoded list with models
        let mut models = Vec::new();

        // Add the Llama 4 Maverick free model
        models.push(ModelInfo {
            id: "meta-llama/llama-4-maverick:free".to_string(),
            display_name: "Llama 4 Maverick (free)".to_string(),
            max_tokens: 1000000, // 1 million token context
            provider: Some("openrouter".to_string()),
        });

        // DeepSeek V3 Base (free)
        models.push(ModelInfo {
            id: "deepseek/deepseek-v3-base:free".to_string(),
            display_name: "DeepSeek V3 Base (free)".to_string(),
            max_tokens: 128000, // 128k context window
            provider: Some("openrouter".to_string()),
        });

        // OpenRouter Quasar Alpha
        models.push(ModelInfo {
            id: "openrouter/quasar-alpha".to_string(),
            display_name: "OpenRouter Quasar Alpha".to_string(),
            max_tokens: 128000, // 128k context window
            provider: Some("openrouter".to_string()),
        });

        // OpenRouter Optimus Alpha
        models.push(ModelInfo {
            id: "openrouter/optimus-alpha".to_string(),
            display_name: "OpenRouter Optimus Alpha".to_string(),
            max_tokens: 128000, // 128k context window
            provider: Some("openrouter".to_string()),
        });

        // Qwen 2.5 VL 3B Instruct (free)
        models.push(ModelInfo {
            id: "qwen/qwen2.5-vl-3b-instruct:free".to_string(),
            display_name: "Qwen 2.5 VL 3B Instruct (free)".to_string(),
            max_tokens: 32000, // 32k context window
            provider: Some("openrouter".to_string()),
        });

        // Qwen 2.5 VL 32B Instruct (free)
        models.push(ModelInfo {
            id: "qwen/qwen2.5-vl-32b-instruct:free".to_string(),
            display_name: "Qwen 2.5 VL 32B Instruct (free)".to_string(),
            max_tokens: 32000, // 32k context window
            provider: Some("openrouter".to_string()),
        });

        // Return the models
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

        // Get the model ID or use Llama 4 Maverick free as default
        let model = model_id.unwrap_or_else(|| "meta-llama/llama-4-maverick:free".to_string());

        // Construct the request URL
        let url = format!(
            "{}/chat/completions",
            self.url
                .clone()
                .unwrap_or("https://openrouter.ai/api/v1".to_string())
        );

        // Create request body with model-specific parameters
        let request_body = OpenRouterRequest {
            model: model.clone(),
            messages: openrouter_messages,
        };

        // Prepare the request body - log it for debugging
        let request_body_json = serde_json::to_string(&request_body).unwrap_or_default();
        log(&format!("OpenRouter request body: {}", request_body_json));

        // Set up headers
        let mut headers = vec![
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
        self.usage
            .native_prompt_tokens
            .unwrap_or(self.usage.prompt_tokens)
    }

    fn output_tokens(&self) -> u32 {
        // Use native token count if available, otherwise use normalized count
        self.usage
            .native_completion_tokens
            .unwrap_or(self.usage.completion_tokens)
    }

    fn calculate_cost(&self) -> f64 {
        // If cost is directly provided in the usage, use that
        if let Some(cost) = self.usage.cost {
            return cost;
        }

        // Check if using a free model
        if is_free_model(&self.model) {
            log(&format!(
                "[DEBUG] Using free model {}, cost is 0.0",
                self.model
            ));
            return 0.0;
        }

        // Get model-specific pricing
        let pricing = get_model_pricing(&self.model);

        // Calculate based on token counts and pricing
        let input_cost_per_million = pricing
            .input_cost_per_million_tokens
            .unwrap_or(self.input_cost_per_million_tokens.unwrap_or(5.0));

        let output_cost_per_million = pricing
            .output_cost_per_million_tokens
            .unwrap_or(self.output_cost_per_million_tokens.unwrap_or(15.0));

        // Calculate costs
        let input_cost = input_cost_per_million * (self.input_tokens() as f64) / 1_000_000.0;

        let output_cost = output_cost_per_million * (self.output_tokens() as f64) / 1_000_000.0;

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
