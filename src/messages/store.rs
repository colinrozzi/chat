use super::{StoredMessage};
use crate::bindings::ntwk::theater::message_server_host::request;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

#[derive(Serialize, Deserialize, Debug)]
pub struct Request {
    pub _type: String,
    pub data: Action,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum Action {
    Get(String),
    Put(Vec<u8>),
    All(()),
}

pub struct MessageStore {
    store_id: String,
}

impl MessageStore {
    pub fn new(store_id: String) -> Self {
        Self { store_id }
    }

    pub fn save_message(&self, msg: &StoredMessage) -> Result<String, Box<dyn std::error::Error>> {
        let req = Request {
            _type: "request".to_string(),
            data: Action::Put(serde_json::to_vec(msg)?),
        };

        let request_bytes = serde_json::to_vec(&req)?;
        let response_bytes = request(&self.store_id, &request_bytes)?;

        let response: Value = serde_json::from_slice(&response_bytes)?;
        if response["status"].as_str() == Some("ok") {
            Ok(response["key"]
                .as_str()
                .map(|s| s.to_string())
                .ok_or("No key in response")?)
        } else {
            Err("Failed to save message".into())
        }
    }

    pub fn load_message(&self, id: &str) -> Result<StoredMessage, Box<dyn std::error::Error>> {
        let req = Request {
            _type: "request".to_string(),
            data: Action::Get(id.to_string()),
        };

        let request_bytes = serde_json::to_vec(&req)?;
        let response_bytes = request(&self.store_id, &request_bytes)?;

        let response: Value = serde_json::from_slice(&response_bytes)?;
        if response["status"].as_str() == Some("ok") {
            if let Some(value) = response.get("value") {
                let bytes = value
                    .as_array()
                    .ok_or("Expected byte array")?
                    .iter()
                    .map(|v| v.as_u64().unwrap_or(0) as u8)
                    .collect::<Vec<u8>>();

                let mut msg: StoredMessage = serde_json::from_slice(&bytes)?;

                // Add ID to the correct variant
                match &mut msg {
                    StoredMessage::Message(m) => m.id = Some(id.to_string()),
                    StoredMessage::Rollup(r) => r.id = Some(id.to_string()),
                }

                Ok(msg)
            } else {
                Err("No value in response".into())
            }
        } else {
            Err("Failed to load message".into())
        }
    }
}