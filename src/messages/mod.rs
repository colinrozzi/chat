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
    User {
        content: String,
    },
    Assistant {
        content: String,
        id: String,
        model: String,
        stop_reason: String,
        stop_sequence: Option<String>,
        message_type: String,
        usage: Usage,
    },
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Usage {
    pub input_tokens: u32,
    pub output_tokens: u32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ChatInfo {
    pub id: String,           // Unique identifier (same as the label)
    pub name: String,         // Display name
    pub head: Option<String>, // Head message ContentRef
    pub icon: Option<String>, // Optional icon identifier
}

pub mod store;
