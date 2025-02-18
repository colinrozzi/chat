use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crate::messages::{Message, StoredMessage, RollupMessage};
use crate::messages::store::MessageStore;
use crate::messages::history::MessageHistory;
use crate::api::claude::ClaudeClient;
use crate::bindings::ntwk::theater::runtime::{log, spawn};
use crate::bindings::ntwk::theater::message_server_host::request;
use serde_json::json;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ChildActor {
    pub actor_id: String,
    pub manifest_name: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Chat {
    pub head: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct State {
    pub chat: Chat,
    pub api_key: String,
    pub connected_clients: HashMap<String, bool>,
    pub store_id: String,
    pub websocket_port: u16,
    pub children: HashMap<String, ChildActor>,
    pub actor_messages: HashMap<String, Vec<u8>>,
}

impl State {
    pub fn new(
        store_id: String,
        api_key: String,
        websocket_port: u16,
        head: Option<String>,
    ) -> Self {
        Self {
            chat: Chat { head },
            api_key,
            connected_clients: HashMap::new(),
            store_id,
            websocket_port,
            children: HashMap::new(),
            actor_messages: HashMap::new(),
        }
    }

    pub fn handle_send_message(
        &mut self,
        content: &str,
    ) -> Result<Vec<StoredMessage>, Box<dyn std::error::Error>> {
        let store = MessageStore::new(self.store_id.clone());
        let history = MessageHistory::new(store);

        // Process user message and get rollup ID
        let user_rollup_id = self.process_message(content, "user", self.chat.head.clone())?;

        // Generate AI response using message history
        let messages = history.get_message_history(self.chat.head.clone())?;
        let claude = ClaudeClient::new(self.api_key.clone());
        let ai_response = claude.generate_response(messages)?;

        // Process assistant message with user rollup as parent
        let assistant_rollup_id =
            self.process_message(&ai_response, "assistant", Some(user_rollup_id.clone()))?;

        // Return all new messages
        let mut new_messages = Vec::new();
        let store = MessageStore::new(self.store_id.clone());

        // Load user message chain
        if let Ok(user_msg) = store.load_message(&user_rollup_id) {
            new_messages.push(user_msg);
        }

        // Get and add user's child responses
        if let Ok(user_children) = history.get_child_responses(&user_rollup_id) {
            new_messages.extend(user_children);
        }

        // Load assistant message chain
        if let Ok(assistant_msg) = store.load_message(&assistant_rollup_id) {
            new_messages.push(assistant_msg);
        }

        // Get and add assistant's child responses
        if let Ok(assistant_children) = history.get_child_responses(&assistant_rollup_id) {
            new_messages.extend(assistant_children);
        }

        Ok(new_messages)
    }

    fn process_message(
        &mut self,
        content: &str,
        role: &str,
        parent_id: Option<String>,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let store = MessageStore::new(self.store_id.clone());
        
        // Create and save initial message
        let msg = Message::new(role.to_string(), content.to_string(), parent_id);
        let msg_id = store.save_message(&StoredMessage::Message(msg))?;

        // Notify children and collect their responses
        let child_responses = self.notify_children(&msg_id)?;

        // Create and save rollup message if there are any child responses
        if !child_responses.is_empty() {
            let rollup = RollupMessage {
                original_message_id: msg_id.clone(),
                child_responses,
                parent: Some(msg_id.clone()),
                id: None,
            };
            let rollup_id = store.save_message(&StoredMessage::Rollup(rollup))?;
            // Update head to rollup
            self.update_head(rollup_id.clone())?;
            Ok(rollup_id)
        } else {
            // If no child responses, just return the message ID
            self.update_head(msg_id.clone())?;
            Ok(msg_id)
        }
    }

    pub fn notify_children(
        &mut self,
        head_id: &str,
    ) -> Result<Vec<crate::messages::ChildResponse>, Box<dyn std::error::Error>> {
        let mut responses = Vec::new();

        for (actor_id, _child) in &self.children {
            // Notify each child of the new head
            if let Ok(response_bytes) = request(
                actor_id,
                &serde_json::to_vec(&json!({
                    "msg_type": "head-update",
                    "data": {
                        "head_id": head_id
                    }
                }))?,
            ) {
                if let Ok(response) = serde_json::from_slice::<serde_json::Value>(&response_bytes) {
                    if response["status"] == "ok" {
                        if let Some(message_id) = response["message_id"].as_str() {
                            responses.push(crate::messages::ChildResponse {
                                child_id: actor_id.clone(),
                                message_id: message_id.to_string(),
                            });
                        }
                    }
                }
            }
        }

        Ok(responses)
    }

    pub fn start_child(&mut self, manifest_name: &str) -> Result<String, Box<dyn std::error::Error>> {
        let manifest_path = format!(
            "/Users/colinrozzi/work/actors/chat/assets/children/{}.toml",
            manifest_name
        );

        let actor_id = spawn(&manifest_path);

        self.children.insert(
            actor_id.clone(),
            ChildActor {
                actor_id: actor_id.clone(),
                manifest_name: manifest_name.to_string(),
            },
        );

        if let Ok(response) = request(
            &actor_id,
            &serde_json::to_vec(&json!({
                "msg_type": "introduction",
                "data": {
                    "store_id": self.store_id.clone()
                }
            }))?,
        ) {
            self.actor_messages.insert(actor_id.clone(), response);
        }

        Ok(actor_id)
    }

    pub fn update_head(&mut self, message_id: String) -> Result<(), Box<dyn std::error::Error>> {
        self.chat.head = Some(message_id);
        Ok(())
    }
}
