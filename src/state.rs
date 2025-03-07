use crate::api::claude::ClaudeClient;
use crate::bindings::ntwk::theater::message_server_host::request;
use crate::bindings::ntwk::theater::runtime::log;
use crate::bindings::ntwk::theater::supervisor::spawn;
use crate::messages::store::MessageStore;
use crate::messages::{ChainEntry, ChatInfo, ChildMessage, Message, MessageData};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use std::error::Error;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ChildActor {
    pub actor_id: String,
    pub manifest_name: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct State {
    pub id: String,
    pub head: Option<String>, // Legacy, kept for backward compatibility
    pub current_chat_id: Option<String>,
    pub claude_client: ClaudeClient,
    pub connected_clients: HashMap<String, bool>,
    pub store: MessageStore,
    pub server_id: u64,
    pub websocket_port: u16, // Keep for backward compatibility
    pub children: HashMap<String, ChildActor>, // Global children (legacy)
    pub actor_messages: HashMap<String, Vec<u8>>,
}

impl State {
    pub fn new(
        id: String,
        store_id: String,
        api_key: String,
        server_id: u64,
        websocket_port: u16,
        head: Option<String>,
    ) -> Self {
        let mut state = Self {
            id,
            head,
            current_chat_id: None,
            claude_client: ClaudeClient::new(api_key.clone()),
            connected_clients: HashMap::new(),
            store: MessageStore::new(store_id.clone()),
            server_id,
            websocket_port,
            children: HashMap::new(),
            actor_messages: HashMap::new(),
        };

        // Migrate legacy chat or initialize the first chat
        if let Ok(Some(chat_id)) = state.store.migrate_legacy_chat() {
            log(&format!("Using migrated chat: {}", chat_id));
            state.current_chat_id = Some(chat_id);
        } else {
            // Get the list of chats
            if let Ok(chat_ids) = state.store.list_chat_ids() {
                if !chat_ids.is_empty() {
                    // Use the first chat by default
                    state.current_chat_id = Some(chat_ids[0].clone());
                    log(&format!("Using existing chat: {}", chat_ids[0]));
                } else {
                    // Create a default chat if none exists
                    if let Ok(chat_info) = state.store.create_chat("New Chat".to_string(), None) {
                        state.current_chat_id = Some(chat_info.id.clone());
                        log(&format!("Created default chat: {}", chat_info.id));
                    }
                }
            }
        }

        // For backward compatibility, also set the legacy head
        if state.head.is_none() && state.current_chat_id.is_some() {
            if let Ok(Some(chat_info)) = state
                .store
                .get_chat_info(&state.current_chat_id.clone().unwrap())
            {
                state.head = chat_info.head.clone();
            }
        }

        state
    }

    pub fn get_current_chat(&self) -> Result<Option<ChatInfo>, Box<dyn std::error::Error>> {
        if let Some(chat_id) = &self.current_chat_id {
            self.store.get_chat_info(chat_id)
        } else {
            Ok(None)
        }
    }

    pub fn switch_chat(&mut self, chat_id: &str) -> Result<(), Box<dyn std::error::Error>> {
        // Verify the chat exists
        let chat_info = self
            .store
            .get_chat_info(chat_id)?
            .ok_or_else(|| format!("Chat {} not found", chat_id))?;

        // Update current chat ID
        self.current_chat_id = Some(chat_id.to_string());

        // Update head for backward compatibility
        self.head = chat_info.head.clone();

        // Notify clients about the new head
        self.notify_head_update()?;

        log(&format!("Switched to chat: {}", chat_id));
        Ok(())
    }

    pub fn create_chat(
        &mut self,
        name: String,
        starting_head: Option<String>,
    ) -> Result<ChatInfo, Box<dyn std::error::Error>> {
        // Create the chat in the store
        let chat_info = self.store.create_chat(name, starting_head)?;

        // Switch to the new chat
        self.current_chat_id = Some(chat_info.id.clone());
        self.head = chat_info.head.clone();

        // Notify clients
        self.notify_head_update()?;

        log(&format!("Created new chat: {}", chat_info.id));
        Ok(chat_info)
    }

    pub fn delete_chat(&mut self, chat_id: &str) -> Result<(), Box<dyn std::error::Error>> {
        // Delete the chat
        self.store.delete_chat(chat_id)?;

        // If we deleted the current chat, switch to another one
        if self.current_chat_id.as_deref() == Some(chat_id) {
            let chat_ids = self.store.list_chat_ids()?;
            if !chat_ids.is_empty() {
                // Find a chat that's not the one we're deleting
                for id in chat_ids {
                    if id != chat_id {
                        self.switch_chat(&id)?;
                        break;
                    }
                }
            } else {
                // No chats left, create a new one
                let chat_info = self.store.create_chat("New Chat".to_string(), None)?;
                self.current_chat_id = Some(chat_info.id.clone());
                self.head = None;
            }
        }

        log(&format!("Deleted chat: {}", chat_id));
        Ok(())
    }

    pub fn add_to_chain(&mut self, data: MessageData) -> ChainEntry {
        log("Adding message to chain");

        // Ensure we have a current chat
        if self.current_chat_id.is_none() {
            log("No current chat, creating a new one");
            if let Ok(chat_info) = self.store.create_chat("New Chat".to_string(), None) {
                self.current_chat_id = Some(chat_info.id.clone());
                log(&format!("Created new chat: {}", chat_info.id));
            } else {
                panic!("Failed to create a new chat");
            }
        }

        // Get chat ID
        let chat_id = self.current_chat_id.clone().unwrap();

        // Get the current head for this chat
        let current_head = match self.store.get_chat_info(&chat_id) {
            Ok(Some(chat_info)) => chat_info.head,
            _ => None,
        };

        let entry = ChainEntry {
            parent: current_head,
            id: None,
            data,
        };

        // Save to runtime store
        let entry = match self.store.save_message(entry, &chat_id) {
            Ok(entry) => entry,
            Err(e) => {
                log(&format!("Error saving message: {}", e));
                panic!("Failed to save message: {}", e);
            }
        };

        // Update the legacy head for backward compatibility
        self.head = Some(entry.id.clone().unwrap());
        log(&format!("Added message to chain: {:?}", entry));

        // Notify all clients about head update
        if let Err(e) = self.notify_head_update() {
            log(&format!(
                "Failed to notify clients about head update: {}",
                e
            ));
        }

        entry
    }

    pub fn add_user_message(&mut self, content: &str) {
        let msg = Message::User {
            content: content.to_string(),
        };

        self.add_to_chain(MessageData::Chat(msg));
        self.notify_children();

        log("Getting anthropic messages");
        let messages = self.get_anthropic_messages();
        log(&format!("Anthropic messages: {:?}", messages));

        match self.claude_client.generate_response(messages) {
            Ok(assistant_msg) => {
                log(&format!("Generated completion: {:?}", assistant_msg));
                self.add_to_chain(MessageData::Chat(assistant_msg));
                self.notify_children();
            }
            Err(e) => {
                log(&format!("Failed to generate completion: {}", e));
                // Notify clients about the error
                let error_message = format!("Failed to generate AI response: {}", e);
                let _ = self.broadcast_websocket_message(
                    &serde_json::to_string(&serde_json::json!({
                        "type": "error",
                        "message": error_message
                    }))
                    .unwrap(),
                );
            }
        }
    }

    pub fn notify_children(&mut self) {
        if self.current_chat_id.is_none() {
            log("No current chat, skipping child notification");
            return;
        }

        let chat_id = self.current_chat_id.clone().unwrap();

        // Get current chat's children
        let chat_children = match self.store.get_chat_info(&chat_id) {
            Ok(Some(chat_info)) => chat_info.children,
            _ => {
                log("Could not get chat info, using legacy children");
                // Fallback to legacy global children
                self.children.clone()
            }
        };

        // Get current head for this chat
        let current_head = match self.store.get_chat_info(&chat_id) {
            Ok(Some(chat_info)) => chat_info.head,
            _ => self.head.clone(), // Fallback to legacy head
        };

        for (actor_id, _) in chat_children.iter() {
            log(&format!("Notifying child: {}", actor_id));

            let response = request(
                actor_id,
                &serde_json::to_vec(&json!({
                    "msg_type": "head-update",
                    "data": {
                        "head": current_head.clone(),
                    }
                }))
                .unwrap(),
            );

            match response {
                Ok(response) => {
                    let child_response: ChildMessage = serde_json::from_slice(&response).unwrap();
                    if !child_response.text.is_empty() {
                        self.add_to_chain(MessageData::ChildMessage(child_response));
                    }
                }
                Err(e) => {
                    log(&format!("Failed to notify child: {}", e));
                }
            }
        }
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
                    if let Some(last_msg) = messages.last() {
                        match (last_msg, &msg) {
                            (
                                Message::User {
                                    content: _last_content,
                                },
                                Message::User { content },
                            ) => {
                                if let Some(Message::User {
                                    content: combined_content,
                                }) = messages.last_mut()
                                {
                                    combined_content.push_str(&format!("\n{}", content));
                                    log(&format!("Updated chat message: {:?}", combined_content));
                                    continue;
                                }
                            }
                            _ => {}
                        }
                    }

                    messages.push(msg.clone());
                }
                MessageData::ChildMessage(child_msg) => {
                    log(&format!("Adding child message: {:?}", child_msg));
                    if !child_msg.text.is_empty() {
                        let text = child_msg.text.clone();
                        let actor_msg =
                            format!("\n<actor id={}>{}</actor>", child_msg.child_id, text);

                        if !messages.is_empty() {
                            match messages.last() {
                                Some(Message::Assistant { .. }) => {
                                    messages.push(Message::User { content: actor_msg });
                                }
                                Some(Message::User { content: _ }) => {
                                    if let Some(Message::User { content }) = messages.last_mut() {
                                        content.push_str(&actor_msg);
                                    }
                                }
                                None => {}
                            }
                        } else {
                            messages.push(Message::User { content: actor_msg });
                        }
                    }
                }
            }
        }

        messages
    }

    pub fn get_chain(&mut self) -> Vec<ChainEntry> {
        let mut chain = vec![];

        // Try to get the current chat's head
        let mut current_id = if let Some(chat_id) = &self.current_chat_id {
            if let Ok(Some(chat_info)) = self.store.get_chat_info(chat_id) {
                chat_info.head
            } else {
                self.head.clone()
            }
        } else {
            self.head.clone()
        };

        // If we have a head, follow the parent links
        while let Some(id) = current_id {
            match self.store.load_message(&id) {
                Ok(entry) => {
                    current_id = entry.parent.clone();
                    chain.push(entry);
                }
                Err(e) => {
                    log(&format!("Error loading message {}: {}", id, e));
                    break;
                }
            }
        }

        chain.reverse();
        chain
    }

    pub fn get_message(&mut self, message_id: &str) -> Result<ChainEntry, Box<dyn Error>> {
        self.store.load_message(message_id)
    }

    pub fn add_child_message(&mut self, child_message: ChildMessage) {
        // Only add if the message has content
        if !child_message.text.is_empty() {
            log(&format!(
                "Adding child message from {}: {}",
                child_message.child_id, child_message.text
            ));
            self.add_to_chain(MessageData::ChildMessage(child_message));

            // Log that head has been updated (add_to_chain already handles notification)
            log(&format!("Head has been updated to: {:?}", self.head));
        }
    }

    pub fn broadcast_websocket_message(&self, message: &str) -> Result<(), String> {
        use crate::bindings::ntwk::theater::http_framework::send_websocket_message;
        use crate::bindings::ntwk::theater::websocket_types::{MessageType, WebsocketMessage};

        for client_id in self.connected_clients.keys() {
            if let Ok(connection_id) = client_id.parse::<u64>() {
                let websocket_message = WebsocketMessage {
                    ty: MessageType::Text,
                    text: Some(message.to_string()),
                    data: None,
                };

                // Use the HTTP framework to send the message
                if let Err(e) =
                    send_websocket_message(self.server_id, connection_id, &websocket_message)
                {
                    log(&format!("Failed to send WebSocket message: {}", e));
                }
            }
        }

        Ok(())
    }

    pub fn notify_head_update(&self) -> Result<(), String> {
        // Format head update notification
        let message = serde_json::to_string(&serde_json::json!({
            "type": "messages_updated",
            "head": self.head,
            "current_chat_id": self.current_chat_id
        }))
        .unwrap();

        self.broadcast_websocket_message(&message)
    }

    pub fn notify_chats_update(&self) -> Result<(), String> {
        if let Ok(chat_ids) = self.store.list_chat_ids() {
            let mut chats = Vec::new();

            for chat_id in chat_ids {
                if let Ok(Some(chat_info)) = self.store.get_chat_info(&chat_id) {
                    chats.push(json!({
                        "id": chat_info.id,
                        "name": chat_info.name,
                        "updated_at": chat_info.updated_at,
                        "created_at": chat_info.created_at,
                        "icon": chat_info.icon,
                    }));
                }
            }

            let message = serde_json::to_string(&serde_json::json!({
                "type": "chats_update",
                "chats": chats,
                "current_chat_id": self.current_chat_id
            }))
            .unwrap();

            self.broadcast_websocket_message(&message)
        } else {
            Err("Failed to list chats".to_string())
        }
    }

    pub fn start_child(
        &mut self,
        manifest_name: &str,
    ) -> Result<String, Box<dyn std::error::Error>> {
        // Ensure we have a current chat
        if self.current_chat_id.is_none() {
            return Err("No current chat selected".into());
        }

        let chat_id = self.current_chat_id.clone().unwrap();
        let manifest_path = format!(
            "/Users/colinrozzi/work/actors/chat/assets/children/{}.toml",
            manifest_name
        );

        let actor_id = spawn(&manifest_path, None)?;

        // Create a child actor record
        let child_actor = ChildActor {
            actor_id: actor_id.clone(),
            manifest_name: manifest_name.to_string(),
        };

        // Add the child actor to the current chat's children map
        let mut chat_info = self
            .store
            .get_chat_info(&chat_id)?
            .ok_or_else(|| format!("Chat {} not found", chat_id))?;

        chat_info
            .children
            .insert(actor_id.clone(), child_actor.clone());
        self.store.update_chat_info(&chat_info)?;

        // For backward compatibility, also add to the global children map
        self.children.insert(actor_id.clone(), child_actor);

        // Get current head for the chat
        let current_head = chat_info.head.clone();

        if let Ok(response) = request(
            &actor_id,
            &serde_json::to_vec(&json!({
                "msg_type": "introduction",
                "data": {
                    "child_id": actor_id.clone(),
                    "store_id": self.store.store_id.clone(),
                    "chat_id": chat_id.clone(),
                    "head": current_head,
                }
            }))?,
        ) {
            log(&format!("Child response: {:?}", response));
            // Add the child message directly to the chain
            let child_response: ChildMessage = serde_json::from_slice(&response)?;
            if !child_response.text.is_empty() {
                self.add_to_chain(MessageData::ChildMessage(child_response));
            }
        }

        Ok(actor_id)
    }

    pub fn stop_child(&mut self, actor_id: &str) -> Result<(), Box<dyn std::error::Error>> {
        // Remove from global children (legacy)
        self.children.remove(actor_id);

        // Also remove from the current chat's children
        if let Some(chat_id) = &self.current_chat_id {
            if let Ok(Some(mut chat_info)) = self.store.get_chat_info(chat_id) {
                chat_info.children.remove(actor_id);
                self.store.update_chat_info(&chat_info)?;
            }
        }

        // Currently there's no API to actually stop the actor in the Theater runtime
        // This would need to be added in a future version

        log(&format!("Stopped child actor: {}", actor_id));
        Ok(())
    }

    pub fn list_available_children(&self) -> Vec<crate::children::ChildInfo> {
        crate::children::scan_available_children()
    }

    pub fn list_running_children(&self) -> Vec<serde_json::Value> {
        // If we have a current chat, return its children
        if let Some(chat_id) = &self.current_chat_id {
            if let Ok(Some(chat_info)) = self.store.get_chat_info(chat_id) {
                return chat_info
                    .children
                    .iter()
                    .map(|(actor_id, child)| {
                        json!({
                            "actor_id": actor_id,
                            "manifest_name": child.manifest_name
                        })
                    })
                    .collect();
            }
        }

        // Fallback to global children (legacy)
        self.children
            .iter()
            .map(|(actor_id, child)| {
                json!({
                    "actor_id": actor_id,
                    "manifest_name": child.manifest_name
                })
            })
            .collect()
    }
}
