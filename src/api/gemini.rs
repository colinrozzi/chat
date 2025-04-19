use crate::bindings::ntwk::theater::http_client::{send_http, HttpRequest};
use crate::bindings::ntwk::theater::runtime::log;
use crate::messages::{AssistantMessage, LlmMessage, Message, ModelInfo};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha1::{Digest, Sha1};

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
    #[serde(rename = "finishReason")]
    pub finish_reason: String,
    pub index: Option<u32>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GeminiResponse {
    pub candidates: Vec<GeminiCandidate>,
    #[serde(rename = "usageMetadata")]
    pub usage: GeminiUsage,
    #[serde(rename = "modelVersion")]
    pub model_version: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GeminiUsage {
    #[serde(rename = "promptTokenCount", default)]
    pub prompt_tokens: u32,
    #[serde(rename = "candidatesTokenCount", default)]
    pub completion_tokens: u32,
    #[serde(rename = "totalTokenCount", default)]
    pub total_tokens: u32,
    #[serde(rename = "promptTokensDetails", default)]
    pub prompt_tokens_details: Option<Vec<TokenDetails>>,
    #[serde(rename = "thoughtsTokenCount", default)]
    pub thoughts_token_count: Option<u32>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TokenDetails {
    pub modality: String,
    #[serde(rename = "tokenCount")]
    pub token_count: u32,
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
        "gemini-2.5-pro-exp-03-25" => ModelPricing {
            input_cost_per_million_tokens: Some(0.35),
            output_cost_per_million_tokens: Some(1.05),
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
        "gemini-2.5-pro-exp-03-25" => 32768,
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
        let model = model_id.unwrap_or_else(|| "gemini-2.5-pro-exp-03-25".to_string());

        // Convert our internal message format to Gemini format
        let mut gemini_contents: Vec<GeminiContent> = Vec::new();

        for msg in messages {
            match msg {
                Message::User { content } => {
                    gemini_contents.push(GeminiContent {
                        role: "user".to_string(),
                        parts: vec![GeminiPart { text: content }],
                    });
                }
                Message::Assistant(assistant_msg) => {
                    gemini_contents.push(GeminiContent {
                        role: "model".to_string(),
                        parts: vec![GeminiPart {
                            text: assistant_msg.content().to_string(),
                        }],
                    });
                }
            }
        }

        // Prepare the HTTP request
        let request = HttpRequest {
            method: "POST".to_string(),
            uri: format!(
                "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
                model, self.api_key
            ),
            headers: vec![("Content-Type".to_string(), "application/json".to_string())],
            body: Some(serde_json::to_vec(&GeminiRequest {
                contents: gemini_contents,
            })?),
        };

        // Send the request
        let http_response =
            send_http(&request).map_err(|e| format!("HTTP request failed: {}", e))?;

        log(format!("Gemini response: {:?}", http_response).as_str());

        // Parse the response
        log("Parsing Gemini response");
        log(format!("Response status: {}", http_response.status).as_str());
        log(format!("Response body: {:?}", http_response.body).as_str());
        let body = http_response.body.ok_or("No response body")?;
        log(format!("Response body: {:?}", String::from_utf8_lossy(&body)).as_str());
        let response: GeminiResponse = serde_json::from_slice(&body)?;
        log(format!("Parsed Gemini response: {:?}", response).as_str());

        // Extract the response content
        if response.candidates.is_empty() {
            return Err("No response candidates".into());
        }

        log(format!("Response candidates: {:?}", response.candidates).as_str());

        let candidate = &response.candidates[0];

        log(format!("Candidate: {:?}", candidate).as_str());

        // Extract text from the response
        let parts = &candidate.content.parts;
        log(format!("Response parts: {:?}", parts).as_str());
        if parts.is_empty() {
            log("Response parts are empty");
            return Err("No text parts in response".into());
        }
        log(format!("Response part: {:?}", parts[0]).as_str());

        let content = parts[0].text.clone();

        // Get pricing for this model
        let pricing = get_model_pricing(&model);

        let mut hasher = sha1::Sha1::new();

        // get the message id from the sha1 hash of the content
        hasher.update(content.as_bytes());
        let id = hasher.finalize();
        let id = format!("{:x}", id);

        // Create our message
        let gemini_message = GeminiMessage {
            content,
            id,
            model,
            finish_reason: candidate.finish_reason.clone(),
            usage: response.usage.clone(),
            input_cost_per_million_tokens: pricing.input_cost_per_million_tokens,
            output_cost_per_million_tokens: pricing.output_cost_per_million_tokens,
        };

        // Wrap in the enum
        Ok(AssistantMessage::Gemini(gemini_message))
    }

    pub fn list_available_models(&self) -> Result<Vec<ModelInfo>, Box<dyn std::error::Error>> {
        // Return available Gemini models
        Ok(vec![ModelInfo {
            id: "gemini-2.5-pro-exp-03-25".to_string(),
            display_name: "Gemini 2.5 Pro Experimental".to_string(),
            max_tokens: 32768,
            provider: Some("gemini".to_string()),
        }])
    }
}
