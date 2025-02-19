use crate::api::claude::ClaudeClient;
use crate::bindings::ntwk::theater::message_server_host::request;
use crate::bindings::ntwk::theater::runtime::{log, spawn};
use crate::messages::store::MessageStore;
use crate::messages::{ChainEntry, ChildMessage, Message, MessageData};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use std::error::Error;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ChildActor {
    pub actor_id: String,
    pub manifest_name: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct State {
    pub head: Option<String>,
    pub claude_client: ClaudeClient,
    pub connected_clients: HashMap<String, bool>,
    pub store: MessageStore,
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
            head,
            claude_client: ClaudeClient::new(api_key.clone()),
            connected_clients: HashMap::new(),
            store: MessageStore::new(store_id.clone()),
            websocket_port,
            children: HashMap::new(),
            actor_messages: HashMap::new(),
        }
    }

    pub fn add_to_chain(&mut self, data: MessageData) -> ChainEntry {
        let entry = ChainEntry {
            parent: self.head.clone(),
            id: None,
            data,
        };
        let entry = self.store.save_message(entry).unwrap();
        self.head = Some(entry.id.clone().unwrap());
        log(&format!("Added message to chain: {:?}", entry));
        entry
    }

    pub fn add_user_message(&mut self, content: &str) {
        let msg = Message {
            content: content.to_string(),
            role: "user".to_string(),
        };

        self.add_to_chain(MessageData::Chat(msg));
        self.notify_children();

        log("Getting anthropic messages");
        let messages = self.get_anthropic_messages();
        log(&format!("Anthropic messages: {:?}", messages));

        match self.claude_client.generate_response(messages) {
            Ok(response) => {
                log(&format!("Generated completion: {}", response));
                let anthropic_msg = Message {
                    content: response,
                    role: "assistant".to_string(),
                };
                self.add_to_chain(MessageData::Chat(anthropic_msg));
                self.notify_children();
            }
            Err(e) => {
                log(&format!("Failed to generate completion: {}", e));
            }
        }
    }

    pub fn notify_children(&mut self) {
        let mut child_responses = vec![];
        for (actor_id, _) in self.children.iter() {
            log(&format!("Notifying child: {}", actor_id));

            let response = request(
                actor_id,
                &serde_json::to_vec(&json!({
                    "msg_type": "head-update",
                    "data": {
                        "head": self.head.clone(),
                    }
                }))
                .unwrap(),
            );

            match response {
                Ok(response) => {
                    let child_response: ChildMessage = serde_json::from_slice(&response).unwrap();
                    child_responses.push(child_response);
                }
                Err(e) => {
                    log(&format!("Failed to notify child: {}", e));
                }
            }
        }

        log(&format!("Child responses: {:?}", child_responses));

        self.add_to_chain(MessageData::ChildRollup(child_responses));
        log("Notified children");
    }

    pub fn get_anthropic_messages(&mut self) -> Vec<Message> {
        let mut messages: Vec<Message> = vec![];
        let chain = self.get_chain();
        log(&format!("Chain: {:?}", chain));

        for entry in chain {
            log(&format!("Processing entry: {:?}", entry));
            match entry.data {
                MessageData::Chat(msg) => {
                    log(&format!("Adding message: {:?}", msg));

                    // If the last message is from the user, and the current message is also from
                    // the user, combine them into a single message
                    let last_msg = messages.last();
                    log(&format!("Last message: {:?}", last_msg));
                    if let Some(last_msg) = last_msg {
                        if last_msg.role == "user" && msg.role == "user" {
                            let chat_msg = messages.last_mut().unwrap();
                            chat_msg.content.push_str(&format!("\n{}", msg.content));
                            log(&format!("Updated chat message: {:?}", chat_msg));
                            continue;
                        }
                    }

                    messages.push(msg.clone());
                }
                MessageData::ChildRollup(child_messages) => {
                    log(&format!("Adding child messages: {:?}", child_messages));
                    for child_msg in child_messages {
                        if !child_msg.text.is_empty() {
                            let text = child_msg.text.clone();
                            let actor_msg =
                                format!("\n<actor id={}>{}</actor>", child_msg.child_id, text);

                            if !messages.is_empty() {
                                let last_msg = messages.last().unwrap();
                                if last_msg.role == "assistant" {
                                    let chat_msg = Message {
                                        role: "user".to_string(),
                                        content: actor_msg,
                                    };
                                    messages.push(chat_msg);
                                    continue;
                                } else {
                                    let chat_msg = messages.last_mut().unwrap();
                                    chat_msg.content.push_str(&actor_msg);
                                }
                            } else {
                                let chat_msg = Message {
                                    role: "user".to_string(),
                                    content: actor_msg,
                                };
                                messages.push(chat_msg);
                            }
                        }
                    }
                }
            }
        }

        messages
    }

    pub fn get_chain(&mut self) -> Vec<ChainEntry> {
        let mut chain = vec![];

        let mut current_id = self.head.clone();
        while let Some(id) = current_id {
            let entry = self.store.load_message(&id).unwrap();
            current_id = entry.parent.clone();
            chain.push(entry);
        }

        chain.reverse();
        chain
    }

    pub fn get_message(&mut self, message_id: &str) -> Result<ChainEntry, Box<dyn Error>> {
        self.store.load_message(message_id)
    }

    pub fn start_child(
        &mut self,
        manifest_name: &str,
    ) -> Result<String, Box<dyn std::error::Error>> {
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
                    "child_id": actor_id.clone(),
                    "store_id": self.store.store_id.clone(),
                }
            }))?,
        ) {
            log(&format!("Child response: {:?}", response));
            // create a rollup message with the response and add it to the chain
            let child_response: ChildMessage = serde_json::from_slice(&response)?;
            self.add_to_chain(MessageData::ChildRollup(vec![child_response]));
        }

        Ok(actor_id)
    }
}
