pub mod openrouter;
pub mod store;

use openrouter::OpenRouterUsage;
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

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UserMessage {
    pub content: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum AssistantMessage {
    Claude(ClaudeMessage),
    Gemini(GeminiMessage),
    OpenRouter(OpenRouterMessage),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct OpenRouterMessage {
    pub content: String,
    pub id: String,
    pub model: String,
    pub stop_reason: String,
    pub native_finish_reason: Option<String>,
    pub usage: OpenRouterUsage,
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
