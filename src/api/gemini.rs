use crate::bindings::ntwk::theater::http_client::{send_http, HttpRequest};
use crate::bindings::ntwk::theater::runtime::log;
use crate::messages::{AssistantMessage, Message, ModelInfo, LlmMessage};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::time::{SystemTime, UNIX_EPOCH};

// Gemini request/response structures
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GeminiPart {
    pub text: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GeminiContent {
    pub role: String,
    pub parts: Vec<GeminiPart>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GeminiRequest {
    pub contents: Vec<GeminiContent>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GeminiResponseContent {
    pub role: String,
    pub parts: Vec<GeminiPart>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GeminiCandidate {
    pub content: GeminiResponseContent,
    pub finish_reason: String,
    pub safety_ratings: Option<Vec<SafetyRating>>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GeminiResponse {
    pub candidates: Vec<GeminiCandidate>,
    pub usage: GeminiUsage,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GeminiUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SafetyRating {
    pub category: String,
    pub probability: String,
}

// Using the GeminiMessage struct from messages module
use crate::messages::GeminiMessage;

// Pricing structure for Gemini models
pub fn get_model_pricing(model_id: &str) -> ModelPricing {
    match model_id {
        "gemini-2.0-flash" => ModelPricing {
            input_cost_per_million_tokens: Some(0.35),
            output_cost_per_million_tokens: Some(1.05),
        },
        "gemini-2.0-pro" => ModelPricing {
            input_cost_per_million_tokens: Some(3.50),
            output_cost_per_million_tokens: Some(10.50),
        },
        // Default case
        _ => ModelPricing {
            input_cost_per_million_tokens: None,
            output_cost_per_million_tokens: None,
        },
    }
}

// Get max tokens for Gemini models
pub fn get_model_max_tokens(model_id: &str) -> u32 {
    match model_id {
        "gemini-2.0-flash" => 32768,
        "gemini-2.0-pro" => 32768,
        _ => 4096, // Default conservative value
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ModelPricing {
    pub input_cost_per_million_tokens: Option<f64>,
    pub output_cost_per_million_tokens: Option<f64>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GeminiClient {
    api_key: String,
}

impl GeminiClient {
    pub fn new(api_key: String) -> Self {
        Self { api_key }
    }

    pub fn generate_response(
        &self,
        messages: Vec<Message>,
        model_id: Option<String>,
    ) -> Result<AssistantMessage, Box<dyn std::error::Error>> {
        // Get the model ID or use default
        let model = model_id.unwrap_or_else(|| "gemini-2.0-flash".to_string());
        
        // Convert our internal message format to Gemini format
        let mut gemini_contents: Vec<GeminiContent> = Vec::new();
        
        for msg in messages {
            match msg {
                Message::User { content } => {
                    gemini_contents.push(GeminiContent {
                        role: "user".to_string(),
                        parts: vec![GeminiPart { text: content }],
                    });
                },
                Message::Assistant(assistant_msg) => {
                    gemini_contents.push(GeminiContent {
                        role: "model".to_string(),
                        parts: vec![GeminiPart { text: assistant_msg.content().to_string() }],
                    });
                },
            }
        }

        // Prepare the HTTP request
        let request = HttpRequest {
            method: "POST".to_string(),
            uri: format!(
                "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
                model, self.api_key
            ),
            headers: vec![
                ("Content-Type".to_string(), "application/json".to_string()),
            ],
            body: Some(serde_json::to_vec(&GeminiRequest { contents: gemini_contents })?),
        };

        // Send the request
        let http_response = send_http(&request)
            .map_err(|e| format!("HTTP request failed: {}", e))?;
        
        // Parse the response
        let body = http_response.body.ok_or("No response body")?;
        let response: GeminiResponse = serde_json::from_slice(&body)?;
        
        // Extract the response content
        if response.candidates.is_empty() {
            return Err("No response candidates".into());
        }
        
        let candidate = &response.candidates[0];
        
        // Extract text from the response
        let parts = &candidate.content.parts;
        if parts.is_empty() {
            return Err("No text parts in response".into());
        }
        
        let content = parts[0].text.clone();
        
        // Create a unique ID for the message using SystemTime
        let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();
        let id = format!("gemini-{}", timestamp);
        
        // Get pricing for this model
        let pricing = get_model_pricing(&model);
        
        // Create our message
        let gemini_message = GeminiMessage {
            content,
            id,
            model,
            finish_reason: candidate.finish_reason.clone(),
            safety_ratings: candidate.safety_ratings.clone(),
            usage: response.usage.clone(),
            input_cost_per_million_tokens: pricing.input_cost_per_million_tokens,
            output_cost_per_million_tokens: pricing.output_cost_per_million_tokens,
        };
        
        // Wrap in the enum
        Ok(AssistantMessage::Gemini(gemini_message))
    }
    
    pub fn list_available_models(&self) -> Result<Vec<ModelInfo>, Box<dyn std::error::Error>> {
        // Return available Gemini models
        Ok(vec![
            ModelInfo {
                id: "gemini-2.0-flash".to_string(),
                display_name: "Gemini 2.0 Flash".to_string(),
                max_tokens: 32768,
                provider: Some("gemini".to_string()),
            },
            ModelInfo {
                id: "gemini-2.0-pro".to_string(),
                display_name: "Gemini 2.0 Pro".to_string(),
                max_tokens: 32768,
                provider: Some("gemini".to_string()),
            },
        ])
    }
}
