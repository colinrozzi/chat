use serde::{Deserialize, Serialize};
use crate::api::claude::Usage as ClaudeUsage;
use crate::api::gemini::{GeminiUsage, SafetyRating};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ChainEntry {
    pub parents: Vec<String>,
    pub id: Option<String>,
    pub data: MessageData,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum MessageData {
    Chat(Message),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Message {
    User {
        content: String,
    },
    Assistant(AssistantMessage),
}

// Trait defining common behavior for LLM messages
pub trait LlmMessage {
    fn content(&self) -> &str;
    fn model_id(&self) -> &str;
    fn provider_name(&self) -> &str;
    fn input_tokens(&self) -> u32;
    fn output_tokens(&self) -> u32;
    fn calculate_cost(&self) -> f64;
    fn stop_reason(&self) -> &str;
    fn message_id(&self) -> &str;
    fn provider_data(&self) -> Option<serde_json::Value>;
}

// Claude-specific message implementation
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ClaudeMessage {
    pub content: String,
    pub id: String,
    pub model: String,
    pub stop_reason: String,
    pub stop_sequence: Option<String>,
    pub message_type: String,
    pub usage: ClaudeUsage,
    pub input_cost_per_million_tokens: Option<f64>,
    pub output_cost_per_million_tokens: Option<f64>,
}

// Wrapper enum for different LLM providers
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum AssistantMessage {
    Claude(ClaudeMessage),
    Gemini(GeminiMessage),
}

// Gemini-specific message implementation
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GeminiMessage {
    pub content: String,
    pub id: String,
    pub model: String,
    pub finish_reason: String,
    pub safety_ratings: Option<Vec<SafetyRating>>,
    pub usage: GeminiUsage,
    pub input_cost_per_million_tokens: Option<f64>,
    pub output_cost_per_million_tokens: Option<f64>,
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
        let input_cost = self.input_cost_per_million_tokens.unwrap_or(3.0) * 
            (self.usage.input_tokens as f64) / 1_000_000.0;
            
        let output_cost = self.output_cost_per_million_tokens.unwrap_or(15.0) * 
            (self.usage.output_tokens as f64) / 1_000_000.0;
            
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

// Implementation of LlmMessage for GeminiMessage
impl LlmMessage for GeminiMessage {
    fn content(&self) -> &str {
        &self.content
    }
    
    fn model_id(&self) -> &str {
        &self.model
    }
    
    fn provider_name(&self) -> &str {
        "gemini"
    }
    
    fn input_tokens(&self) -> u32 {
        self.usage.prompt_tokens
    }
    
    fn output_tokens(&self) -> u32 {
        self.usage.completion_tokens
    }
    
    fn calculate_cost(&self) -> f64 {
        let input_cost = self.input_cost_per_million_tokens.unwrap_or(0.35) * 
            (self.usage.prompt_tokens as f64) / 1_000_000.0;
            
        let output_cost = self.output_cost_per_million_tokens.unwrap_or(1.05) * 
            (self.usage.completion_tokens as f64) / 1_000_000.0;
            
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
            "safety_ratings": self.safety_ratings
        }))
    }
}

// Forward trait implementation to the inner message
impl LlmMessage for AssistantMessage {
    fn content(&self) -> &str {
        match self {
            AssistantMessage::Claude(msg) => msg.content(),
            AssistantMessage::Gemini(msg) => msg.content(),
        }
    }
    
    fn model_id(&self) -> &str {
        match self {
            AssistantMessage::Claude(msg) => msg.model_id(),
            AssistantMessage::Gemini(msg) => msg.model_id(),
        }
    }
    
    fn provider_name(&self) -> &str {
        match self {
            AssistantMessage::Claude(msg) => msg.provider_name(),
            AssistantMessage::Gemini(msg) => msg.provider_name(),
        }
    }
    
    fn input_tokens(&self) -> u32 {
        match self {
            AssistantMessage::Claude(msg) => msg.input_tokens(),
            AssistantMessage::Gemini(msg) => msg.input_tokens(),
        }
    }
    
    fn output_tokens(&self) -> u32 {
        match self {
            AssistantMessage::Claude(msg) => msg.output_tokens(),
            AssistantMessage::Gemini(msg) => msg.output_tokens(),
        }
    }
    
    fn calculate_cost(&self) -> f64 {
        match self {
            AssistantMessage::Claude(msg) => msg.calculate_cost(),
            AssistantMessage::Gemini(msg) => msg.calculate_cost(),
        }
    }
    
    fn stop_reason(&self) -> &str {
        match self {
            AssistantMessage::Claude(msg) => msg.stop_reason(),
            AssistantMessage::Gemini(msg) => msg.stop_reason(),
        }
    }
    
    fn message_id(&self) -> &str {
        match self {
            AssistantMessage::Claude(msg) => msg.message_id(),
            AssistantMessage::Gemini(msg) => msg.message_id(),
        }
    }
    
    fn provider_data(&self) -> Option<serde_json::Value> {
        match self {
            AssistantMessage::Claude(msg) => msg.provider_data(),
            AssistantMessage::Gemini(msg) => msg.provider_data(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ModelInfo {
    pub id: String,
    pub display_name: String,
    pub max_tokens: u32,
    pub provider: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ChatInfo {
    pub id: String,           // Unique identifier (same as the label)
    pub name: String,         // Display name
    pub head: Option<String>, // Head message ContentRef
    pub icon: Option<String>, // Optional icon identifier
}

// Implement conversion from ClaudeMessage to AssistantMessage
impl From<ClaudeMessage> for AssistantMessage {
    fn from(msg: ClaudeMessage) -> Self {
        AssistantMessage::Claude(msg)
    }
}

// Implement conversion from GeminiMessage to AssistantMessage
impl From<GeminiMessage> for AssistantMessage {
    fn from(msg: GeminiMessage) -> Self {
        AssistantMessage::Gemini(msg)
    }
}

pub mod store;
