pub mod openrouter;
pub mod store;

use serde::{Deserialize, Serialize};

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
    User(UserMessage),
    Assistant(AssistantMessage),
    Tool(ToolMessage),
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

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UserMessage {
    pub content: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum AssistantMessage {
    Claude(ClaudeMessage),
    OpenRouter(OpenRouterMessage),
}

impl AssistantMessage {
    pub fn content(&self) -> &str {
        match self {
            AssistantMessage::Claude(msg) => &msg.content,
            AssistantMessage::OpenRouter(msg) => &msg.content,
        }
    }

    pub fn model_id(&self) -> &str {
        match self {
            AssistantMessage::Claude(msg) => &msg.model,
            AssistantMessage::OpenRouter(msg) => &msg.model,
        }
    }

    pub fn provider_name(&self) -> &str {
        match self {
            AssistantMessage::Claude(_) => "claude",
            AssistantMessage::OpenRouter(_) => "openrouter",
        }
    }

    pub fn input_tokens(&self) -> u32 {
        match self {
            AssistantMessage::Claude(msg) => msg.usage.input_tokens,
            AssistantMessage::OpenRouter(msg) => msg.usage.input_tokens,
        }
    }
    pub fn output_tokens(&self) -> u32 {
        match self {
            AssistantMessage::Claude(msg) => msg.usage.output_tokens,
            AssistantMessage::OpenRouter(msg) => msg.usage.output_tokens,
        }
    }

    pub fn stop_reason(&self) -> &str {
        match self {
            AssistantMessage::Claude(msg) => &msg.stop_reason,
            AssistantMessage::OpenRouter(msg) => msg.stop_reason.as_str(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Usage {
    pub input_tokens: u32,
    pub output_tokens: u32,
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
    pub usage: Usage,
    pub input_cost_per_million_tokens: Option<f64>,
    pub output_cost_per_million_tokens: Option<f64>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct OpenRouterMessage {
    pub content: String,
    pub id: String,
    pub model: String,
    pub stop_reason: String,
    pub native_finish_reason: Option<String>,
    pub usage: Usage,
    pub input_cost_per_million_tokens: Option<f64>,
    pub output_cost_per_million_tokens: Option<f64>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ToolMessage {
    pub tool_call_id: String,
    pub content: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ModelInfo {
    pub id: String,
    pub display_name: String,
    pub max_tokens: u32,
    pub provider: Option<String>,
    pub tools_enabled: bool,
    pub input_cost_per_million_tokens: Option<f64>,
    pub output_cost_per_million_tokens: Option<f64>,
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
