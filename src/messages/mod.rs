use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Message {
    pub role: String,
    pub content: String,
    pub parent: Option<String>,
    pub id: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ChildResponse {
    pub child_id: String,
    pub message_id: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RollupMessage {
    pub original_message_id: String,
    pub child_responses: Vec<ChildResponse>,
    pub parent: Option<String>,
    pub id: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type")]
pub enum StoredMessage {
    Message(Message),
    Rollup(RollupMessage),
}

impl Message {
    pub fn new(role: String, content: String, parent: Option<String>) -> Self {
        Self {
            role,
            content,
            parent,
            id: None,
        }
    }
}

impl StoredMessage {
    pub fn parent(&self) -> Option<String> {
        match self {
            StoredMessage::Message(m) => m.parent.clone(),
            StoredMessage::Rollup(r) => r.parent.clone(),
        }
    }
}

pub mod store;
pub mod history;