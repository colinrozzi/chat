use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use crate::state::ChildActor;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ChainEntry {
    pub parent: Option<String>,
    pub id: Option<String>,
    pub data: MessageData,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum MessageData {
    Chat(Message),
    ChildMessage(ChildMessage),
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
pub struct ChildMessage {
    pub child_id: String,
    pub text: String,
    pub data: Value,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ChatInfo {
    pub id: String,         // Unique identifier (same as the label)
    pub name: String,       // Display name
    pub head: Option<String>, // Head message ContentRef
    pub created_at: u64,    // Timestamp
    pub updated_at: u64,    // Timestamp
    pub icon: Option<String>, // Optional icon identifier
    pub children: HashMap<String, ChildActor>, // Map of actor_id to ChildActor for this chat
}

pub mod store;
