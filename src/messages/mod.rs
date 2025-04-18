use crate::api::openrouter::OpenRouterLlmMessage;
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
    User { content: String },
    Assistant(OpenRouterLlmMessage),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ModelInfo {
    pub id: String,
    pub display_name: String,
    pub max_tokens: u32,
    pub provider: Option<String>,
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

pub mod store;
