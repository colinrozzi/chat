use crate::state::ChildActor;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ChainEntry {
    pub parents: Vec<String>,
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
    pub html: Option<String>,  // New field for HTML content
    pub parent_id: Option<String>, // New field for parent message reference
    pub data: Value,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ChatInfo {
    pub id: String,                            // Unique identifier (same as the label)
    pub name: String,                          // Display name
    pub head: Option<String>,                  // Head message ContentRef
    pub icon: Option<String>,                  // Optional icon identifier
    pub children: HashMap<String, ChildActor>, // Map of actor_id to ChildActor for this chat
}

pub mod store;
