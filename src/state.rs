use crate::api::claude::ClaudeClient;
use crate::bindings::ntwk::theater::message_server_host::request;
use crate::bindings::ntwk::theater::runtime::log;
use crate::bindings::ntwk::theater::supervisor::spawn;
use crate::fs::ContentFS;
use crate::messages::store::MessageStore;
use crate::messages::{ChainEntry, ChatInfo, ChildMessage, Message, MessageData};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use std::error::Error;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ChildActor {
    pub actor_id: String,
    pub manifest_name: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PendingChildMessage {
    pub message: ChildMessage,
    pub selected: bool,
    pub timestamp: u64,
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
    pub pending_child_messages: HashMap<String, PendingChildMessage>, // Pending child messages (not committed to chain)
    #[serde(skip)]
    pub filesystem: Arc<ContentFS>, // Content filesystem
}

impl State {
    pub fn new(
        id: String,
        store_id: String,
        api_key: String,
        server_id: u64,
        websocket_port: u16,
        head: Option<String>,
        content_fs_actor_id: String,
    ) -> Self {
        // Create content filesystem
        let filesystem = ContentFS::new(content_fs_actor_id);
        
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
            pending_child_messages: HashMap::new(),
            filesystem,
        };

        // Get the list of chats
        match state.store.list_chat_ids() {
            Ok(chat_ids) if !chat_ids.is_empty() => {
                // Use the first chat by default
                state.current_chat_id = Some(chat_ids[0].clone());
                log(&format!("Using existing chat: {}", chat_ids[0]));
            }
            _ => {
                // Create a default chat if none exists
                log("No existing chats found, attempting to create a default chat");

                // Use a safer approach to handle potential errors
                let create_result = match state.store.create_chat("New Chat".to_string(), None) {
                    Ok(chat_info) => {
                        state.current_chat_id = Some(chat_info.id.clone());
                        log(&format!("Created default chat: {}", chat_info.id));
                        Ok(())
                    }
                    Err(e) => {
                        // Just log the error but don't panic
                        log(&format!(
                            "Failed to create default chat during initialization: {}",
                            e
                        ));
                        log("Will create chat when first message is sent");
                        Err(e)
                    }
                };

                // Continue initialization even if chat creation failed
                if create_result.is_err() {
                    log("Continuing initialization without a default chat");
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

    pub fn add_to_chain(&mut self, data: MessageData, parents: Vec<String>) -> ChainEntry {
        log(&format!("[DEBUG] Adding message to chain with {} parents: {:?}", parents.len(), parents));

        // Ensure we have a current chat
        if self.current_chat_id.is_none() {
            log("[DEBUG] No current chat, creating a new one");
            match self.store.create_chat("New Chat".to_string(), None) {
                Ok(chat_info) => {
                    self.current_chat_id = Some(chat_info.id.clone());
                    log(&format!("[DEBUG] Created new chat: {}", chat_info.id));
                }
                Err(e) => {
                    // Log the error but continue with a fallback chat ID
                    log(&format!("[ERROR] Failed to create a new chat: {}", e));

                    // Create a fallback chat ID
                    let fallback_id = format!(
                        "{}",
                        SystemTime::now()
                            .duration_since(UNIX_EPOCH)
                            .unwrap()
                            .as_secs()
                    );
                    log(&format!("[DEBUG] Using fallback chat ID: {}", fallback_id));
                    self.current_chat_id = Some(fallback_id);
                }
            }
        }

        // Get chat ID
        let chat_id = self.current_chat_id.clone().unwrap();

        let entry = ChainEntry {
            parents,
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
        log("[DEBUG] Adding user message");
        let msg = Message::User {
            content: content.to_string(),
        };

        // Get current head as parent
        let mut parents = Vec::new();
        if let Some(chat_id) = &self.current_chat_id {
            if let Ok(Some(chat_info)) = self.store.get_chat_info(chat_id) {
                if let Some(head) = chat_info.head {
                    log(&format!("[DEBUG] Using head as parent: {}", head));
                    parents.push(head);
                } else {
                    log("[DEBUG] No head found for current chat");
                }
            } else {
                log("[DEBUG] Could not get chat info for current chat");
            }
        } else {
            log("[DEBUG] No current chat ID");
        }

        // Get selected pending child messages as additional parents
        let selected_child_messages: Vec<String> = self.pending_child_messages.iter()
            .filter(|(_, pcm)| pcm.selected)
            .map(|(id, _)| id.clone())
            .collect();

        log(&format!("[DEBUG] Found {} selected pending child messages", selected_child_messages.len()));

        // Add selected child messages as parents if any
        for child_id in &selected_child_messages {
            log(&format!("[DEBUG] Adding pending child message as parent: {}", child_id));
            parents.push(child_id.clone());
        }

        // Add user message to chain with all parents
        let parents_clone = parents.clone();
        log(&format!("[DEBUG] Adding user message to chain with {} parents", parents.len()));
        let user_entry = self.add_to_chain(MessageData::Chat(msg), parents);
        log(&format!("[DEBUG] User message added with ID: {:?}", user_entry.id));
        
        // Add all selected pending child messages to the chain
        for child_id in selected_child_messages {
            if let Some(pcm) = self.pending_child_messages.remove(&child_id) {
                log(&format!("[DEBUG] Committing selected child message: {}", child_id));
                
                // Create a ChainEntry for this child message and save it
                let chat_id = self.current_chat_id.clone().unwrap();
                let entry = ChainEntry {
                    parents: parents_clone.clone(), // Same parents as the user message
                    id: Some(child_id),
                    data: MessageData::ChildMessage(pcm.message.clone()),
                };
                
                log(&format!("[DEBUG] Saving child message with ID: {:?}, parents: {:?}", entry.id, entry.parents));
                if let Err(e) = self.store.save_specific_message(entry, &chat_id) {
                    log(&format!("[ERROR] Error saving child message: {}", e));
                } else {
                    log("[DEBUG] Child message saved successfully");
                }
            }
        }

        // Now that we've committed the selected messages, notify remaining children
        self.notify_children();
    }
    
    pub fn generate_llm_response(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        log("[DEBUG] Getting anthropic messages");
        let messages = self.get_anthropic_messages();
        log(&format!("[DEBUG] Got {} anthropic messages", messages.len()));

        // Get current head as parent
        let mut parents = Vec::new();
        if let Some(chat_id) = &self.current_chat_id {
            if let Ok(Some(chat_info)) = self.store.get_chat_info(chat_id) {
                if let Some(head) = chat_info.head {
                    parents.push(head);
                }
            }
        }

        // Get selected pending child messages as additional parents
        let selected_child_messages: Vec<String> = self.pending_child_messages.iter()
            .filter(|(_, pcm)| pcm.selected)
            .map(|(id, _)| id.clone())
            .collect();

        // Add selected child messages as parents if any
        for child_id in &selected_child_messages {
            parents.push(child_id.clone());
        }

        match self.claude_client.generate_response(messages) {
            Ok(assistant_msg) => {
                log(&format!("Generated completion: {:?}", assistant_msg));
                
                // Add LLM response to chain with all parents
                let parents_clone = parents.clone();
                self.add_to_chain(MessageData::Chat(assistant_msg), parents);
                
                // Add all selected pending child messages to the chain
                for child_id in selected_child_messages {
                    if let Some(pcm) = self.pending_child_messages.remove(&child_id) {
                        log(&format!("Committing selected child message: {}", child_id));
                        
                        // Create a ChainEntry for this child message and save it
                        let chat_id = self.current_chat_id.clone().unwrap();
                        let entry = ChainEntry {
                            parents: parents_clone.clone(), // Same parents as the assistant message
                            id: Some(child_id),
                            data: MessageData::ChildMessage(pcm.message.clone()),
                        };
                        
                        if let Err(e) = self.store.save_specific_message(entry, &chat_id) {
                            log(&format!("Error saving child message: {}", e));
                        }
                    }
                }
                
                self.notify_children();
                Ok(())
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
                Err(e.into())
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
            log(&format!("[DEBUG] Notifying child: {}", actor_id));

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
                    // For Claude integration, we always use the text field even if HTML is present
                    if !child_response.text.is_empty() {
                        // First check if child response has an explicit parent_id field
                        if let Some(parent_id) = child_response.parent_id.as_ref() {
                            log(&format!("[DEBUG] Using explicit parent_id from child message: {}", parent_id));
                            let parents = vec![parent_id.clone()];
                            self.add_to_chain(MessageData::ChildMessage(child_response), parents);
                        }
                        // Fallback to checking data.parent_id for backward compatibility
                        else if let Some(parent_id) = child_response.data.get("parent_id").and_then(|v| v.as_str()) {
                            log(&format!("[DEBUG] Using parent_id from child message data: {}", parent_id));
                            let parents = vec![parent_id.to_string()];
                            self.add_to_chain(MessageData::ChildMessage(child_response), parents);
                        } else {
                            // Fallback to using current head as parent
                            log("[WARN] Child message doesn't contain parent_id, using current head as fallback");
                            if let Some(head_id) = &current_head {
                                let parents = vec![head_id.clone()];
                                self.add_to_chain(MessageData::ChildMessage(child_response), parents);
                            } else {
                                log("[ERROR] No head available for parent reference, using empty parents");
                                // Last resort - use empty parents array, but this may break the DAG structure
                                self.add_to_chain(MessageData::ChildMessage(child_response), vec![]);
                            }
                        }
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

        // Process chain entries (already in chronological order)
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
                        // Always use text field for Claude messages, not HTML
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

        // Add selected pending child messages
        let selected_pending_messages: Vec<_> = self.pending_child_messages.values()
            .filter(|pcm| pcm.selected)
            .collect();
        
        if !selected_pending_messages.is_empty() {
            log(&format!("Adding {} selected pending child messages", selected_pending_messages.len()));
            
            // Sort by timestamp to maintain a consistent order
            let mut sorted_pending = selected_pending_messages.clone();
            sorted_pending.sort_by_key(|pcm| pcm.timestamp);
            
            // Add each selected pending message to the conversation
            for pcm in sorted_pending {
                let child_msg = &pcm.message;
                if !child_msg.text.is_empty() {
                    // Always use text field for Claude messages, not HTML
                    let text = child_msg.text.clone();
                    let actor_msg = format!("\n<actor id={}>{}</actor>", child_msg.child_id, text);

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

        messages
    }

    pub fn get_chain(&mut self) -> Vec<ChainEntry> {
        // Create a set to track processed message IDs
        let mut processed_ids = std::collections::HashSet::new();
        
        // This will store the messages in reverse order (newest first)
        let mut reverse_chain = Vec::new();
        
        // Start with the current head
        let current_id = if let Some(chat_id) = &self.current_chat_id {
            if let Ok(Some(chat_info)) = self.store.get_chat_info(chat_id) {
                chat_info.head
            } else {
                self.head.clone()
            }
        } else {
            self.head.clone()
        };
        
        // Process messages starting from the head
        if let Some(head_id) = current_id {
            self.process_message_chain(&head_id, &mut reverse_chain, &mut processed_ids);
        }
        
        // Reverse to get chronological order (oldest first)
        reverse_chain.reverse();
        return reverse_chain;
    }
    
    // Helper method to recursively process the DAG message chain
    fn process_message_chain(
        &mut self,
        message_id: &str,
        chain: &mut Vec<ChainEntry>,
        processed_ids: &mut std::collections::HashSet<String>
    ) {
        // Skip if already processed
        if processed_ids.contains(message_id) {
            log(&format!("[DEBUG] Message {} already processed, skipping", message_id));
            return;
        }
        
        // Try to load the message
        match self.store.load_message(message_id) {
            Ok(entry) => {
                // Mark as processed
                processed_ids.insert(message_id.to_string());
                log(&format!("[DEBUG] Processing message ID: {}, type: {:?}", 
                    message_id, 
                    if let MessageData::Chat(_) = &entry.data { "Chat" } else { "ChildMessage" }
                ));
                
                // Log parent information
                if !entry.parents.is_empty() {
                    log(&format!("[DEBUG] Message {} has {} parents: {:?}", 
                        message_id, entry.parents.len(), entry.parents));
                } else {
                    log(&format!("[DEBUG] Message {} has no parents", message_id));
                }
                
                // Add to chain
                chain.push(entry.clone());
                
                // Process all parents recursively
                for parent_id in &entry.parents {
                    log(&format!("[DEBUG] Processing parent: {} for message: {}", parent_id, message_id));
                    self.process_message_chain(parent_id, chain, processed_ids);
                }
            }
            Err(e) => {
                log(&format!("[ERROR] Error loading message {}: {}", message_id, e));
            }
        }
    }

    pub fn get_message(&mut self, message_id: &str) -> Result<ChainEntry, Box<dyn Error>> {
        self.store.load_message(message_id)
    }

    pub fn add_pending_child_message(&mut self, child_message: ChildMessage) -> String {
        // Generate a unique ID for this pending message
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        
        let id = format!("pending-{}-{}", child_message.child_id, timestamp);
        
        // Create a pending child message with default selected state (true)
        let pending_msg = PendingChildMessage {
            message: child_message,
            selected: true,  // Default to selected
            timestamp,
        };
        
        // Add to pending messages
        self.pending_child_messages.insert(id.clone(), pending_msg);
        
        // Notify clients about the new pending message
        self.notify_pending_child_messages_update();
        
        id
    }
    
    pub fn toggle_pending_child_message(&mut self, id: &str, selected: bool) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(pcm) = self.pending_child_messages.get_mut(id) {
            pcm.selected = selected;
            self.notify_pending_child_messages_update();
            Ok(())
        } else {
            Err(format!("Pending child message {} not found", id).into())
        }
    }
    
    pub fn remove_pending_child_message(&mut self, id: &str) -> Result<(), Box<dyn std::error::Error>> {
        if self.pending_child_messages.remove(id).is_some() {
            self.notify_pending_child_messages_update();
            Ok(())
        } else {
            Err(format!("Pending child message {} not found", id).into())
        }
    }
    
    pub fn notify_pending_child_messages_update(&self) -> Result<(), String> {
        // Prepare a list of pending child messages for the client
        let pending_messages: Vec<serde_json::Value> = self.pending_child_messages
            .iter()
            .map(|(id, pcm)| {
                json!({
                    "id": id,
                    "child_id": pcm.message.child_id,
                    "text": pcm.message.text,
                    "html": pcm.message.html,
                    "data": pcm.message.data,
                    "selected": pcm.selected,
                    "timestamp": pcm.timestamp
                })
            })
            .collect();
        
        // Send the update to all connected clients
        self.broadcast_websocket_message(
            &serde_json::to_string(&serde_json::json!({
                "type": "pending_child_messages_update",
                "pending_messages": pending_messages
            }))
            .unwrap(),
        )
    }

    pub fn broadcast_websocket_message(&self, message: &str) -> Result<(), String> {
        use crate::bindings::ntwk::theater::http_framework::send_websocket_message;
        use crate::bindings::ntwk::theater::websocket_types::{MessageType, WebsocketMessage};

        log(&format!("[DEBUG] Broadcasting WebSocket message to {} clients", self.connected_clients.len()));

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
                    log(&format!("[ERROR] Failed to send WebSocket message to client {}: {}", connection_id, e));
                } else {
                    log(&format!("[DEBUG] WebSocket message sent to client {}", connection_id));
                }
            } else {
                log(&format!("[WARN] Invalid client ID format: {}", client_id));
            }
        }

        Ok(())
    }

    pub fn notify_head_update(&self) -> Result<(), String> {
        // Format head update notification
        log(&format!("[DEBUG] Notifying clients of head update: Head={:?}, Chat={:?}", self.head, self.current_chat_id));
        let message = serde_json::to_string(&serde_json::json!({
            "type": "messages_updated",
            "head": self.head,
            "current_chat_id": self.current_chat_id
        }))
        .unwrap();

        match self.broadcast_websocket_message(&message) {
            Ok(_) => {
                log("[DEBUG] Head update notification sent successfully");
                Ok(())
            },
            Err(e) => {
                log(&format!("[ERROR] Failed to broadcast head update: {}", e));
                Err(e)
            }
        }
    }

    pub fn notify_chats_update(&self) -> Result<(), String> {
        if let Ok(chat_ids) = self.store.list_chat_ids() {
            let mut chats = Vec::new();

            for chat_id in chat_ids {
                if let Ok(Some(chat_info)) = self.store.get_chat_info(&chat_id) {
                    chats.push(json!({
                        "id": chat_info.id,
                        "name": chat_info.name,
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
        log(&format!("[DEBUG] Starting child actor: {}", manifest_name));
        // Ensure we have a current chat
        if self.current_chat_id.is_none() {
            log("[ERROR] No current chat selected when starting child");
            return Err("No current chat selected".into());
        }

        let chat_id = self.current_chat_id.clone().unwrap();
        log(&format!("[DEBUG] Using chat ID: {}", chat_id));
        // Read the manifest using content-fs
        let manifest_path = format!("children/{}.toml", manifest_name);
        let manifest_content = match self.filesystem.read_file(&manifest_path) {
            Ok(content) => content,
            Err(e) => return Err(format!("Failed to read manifest file: {}", e).into()),
        };
        
        // Create a temporary file for spawning
        use crate::bindings::ntwk::theater::filesystem::write_file;
        let temp_path = format!("/tmp/spawn-manifest-{}.toml", manifest_name);
        write_file(&temp_path, &manifest_content)?;

        log(&format!("[DEBUG] Spawning actor from manifest: {}", temp_path));
        let actor_id = spawn(&temp_path, None)?;
        log(&format!("[DEBUG] Actor spawned with ID: {}", actor_id));

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
        log(&format!("[DEBUG] Current head for child actor introduction: {:?}", current_head));

        log(&format!("[DEBUG] Sending introduction request to child actor: {}", actor_id));
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
            // Get current head for the chat
            let parents = if let Some(ref head) = chat_info.head {
                vec![head.clone()]
            } else {
                vec![]
            };

            // Add the child message directly to the chain
            let child_response: ChildMessage = serde_json::from_slice(&response)?;
            if !child_response.text.is_empty() {
                self.add_to_chain(MessageData::ChildMessage(child_response), parents);
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
        crate::children::scan_available_children(&*self.filesystem)
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
