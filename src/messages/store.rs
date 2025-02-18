use crate::bindings::ntwk::theater::message_server_host::request;
use crate::messages::ChainEntry;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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

#[derive(Serialize, Deserialize, Debug)]
struct PutResponse {
    key: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct GetResponse {
    key: String,
    value: Vec<u8>,
}

#[derive(Serialize, Deserialize, Debug)]
struct AllResponse {
    data: Vec<GetResponse>,
}

#[derive(Serialize, Deserialize, Debug)]
enum ResponseData {
    Put(PutResponse),
    Get(GetResponse),
    All(AllResponse),
    Error(String),
}

#[derive(Serialize, Deserialize, Debug)]
struct Response {
    status: String,
    data: ResponseData,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MessageStore {
    pub store_id: String,
    pub cache: HashMap<String, ChainEntry>,
}

impl MessageStore {
    pub fn new(store_id: String) -> Self {
        Self {
            store_id,
            cache: HashMap::new(),
        }
    }

    pub fn save_message(
        &mut self,
        mut entry: ChainEntry,
    ) -> Result<ChainEntry, Box<dyn std::error::Error>> {
        let req = Request {
            _type: "request".to_string(),
            data: Action::Put(serde_json::to_vec(&entry)?),
        };

        let request_bytes = serde_json::to_vec(&req)?;
        let response_bytes = request(&self.store_id, &request_bytes)?;

        let response: Response = serde_json::from_slice(&response_bytes)?;
        if response.status == "ok".to_string() {
            match response.data {
                ResponseData::Put(PutResponse { key }) => {
                    entry.id = Some(key.clone());
                    self.cache.insert(key.clone(), entry.clone());
                    Ok(entry)
                }
                ResponseData::Error(e) => Err(e.into()),
                _ => Err("Unexpected response data".into()),
            }
        } else {
            Err("Failed to save message".into())
        }
    }

    pub fn load_message(&mut self, id: &str) -> Result<ChainEntry, Box<dyn std::error::Error>> {
        if let Some(msg) = self.cache.get(id) {
            return Ok(msg.clone());
        }

        let req = Request {
            _type: "request".to_string(),
            data: Action::Get(id.to_string()),
        };

        let request_bytes = serde_json::to_vec(&req)?;
        let response_bytes = request(&self.store_id, &request_bytes)?;

        let response: Response = serde_json::from_slice(&response_bytes)?;
        if response.status == "ok".to_string() {
            match response.data {
                ResponseData::Get(GetResponse { key, value }) => {
                    let mut msg: ChainEntry = serde_json::from_slice(&value)?;
                    msg.id = Some(key.clone());
                    self.cache.insert(key, msg.clone());
                    Ok(msg)
                }
                ResponseData::Error(e) => Err(e.into()),
                _ => Err("Unexpected response data".into()),
            }
        } else {
            Err("Failed to load message".into())
        }
    }
}
