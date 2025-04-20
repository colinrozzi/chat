use crate::api::openrouter::OpenRouterClient;
use crate::bindings::ntwk::theater::runtime::log;
use crate::mcp_server::{McpServer, McpServerConfig};
use crate::messages::store::MessageStore;
use crate::messages::{
    openrouter::FunctionDefinition, AssistantMessage, ChainEntry, ChatInfo, Message, MessageData,
    ModelInfo, ToolMessage, UserMessage,
};

use mcp_protocol::types::tool::Tool;
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
    pub id: String,
    pub head: Option<String>,
    pub current_chat_id: Option<String>,
    pub openrouter_client: OpenRouterClient,
    pub connected_clients: HashMap<String, bool>,
    pub store: MessageStore,
    pub server_id: u64,
    pub mcp_servers: Vec<McpServer>,
}

impl State {
    pub fn new(
        id: String,
        store_id: String,
        openrouter_api_key: String,
        server_id: u64,
        mcp_server_configs: Option<Vec<McpServerConfig>>,
        model_configs: Vec<ModelInfo>,
    ) -> Self {
        let mut state = Self {
            id,
            head: None,
            current_chat_id: None,
            openrouter_client: OpenRouterClient::new(openrouter_api_key.clone(), model_configs),
            connected_clients: HashMap::new(),
            store: MessageStore::new(store_id.clone()),
            server_id,
            mcp_servers: Vec::new(),
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

        if state.head.is_none() && state.current_chat_id.is_some() {
            if let Ok(Some(chat_info)) = state
                .store
                .get_chat_info(&state.current_chat_id.clone().unwrap())
            {
                state.head = chat_info.head.clone();
            }
        }

        if let Some(mcp_server_configs) = mcp_server_configs {
            log("Starting MCP servers");
            for config in mcp_server_configs {
                let mut mcp_server = McpServer::new(config);
                mcp_server.start();
                state.mcp_servers.push(mcp_server);
            }
        } else {
            log("No MCP server configs provided");
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
        log(&format!(
            "[DEBUG] Adding message to chain with {} parents: {:?}",
            parents.len(),
            parents
        ));

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
                    self.current_chat_id = Some("fallback_chat_id".to_string());
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
        let msg = Message::User(UserMessage {
            content: content.to_string(),
        });

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

        // Add user message to chain with all parents
        log(&format!(
            "[DEBUG] Adding user message to chain with {} parents",
            parents.len()
        ));
        let user_entry = self.add_to_chain(MessageData::Chat(msg), parents);
        log(&format!(
            "[DEBUG] User message added with ID: {:?}",
            user_entry.id
        ));
    }

    pub fn generate_llm_response(
        &mut self,
        model_id: String,
    ) -> Result<(), Box<dyn std::error::Error>> {
        log("[DEBUG] Getting messages for LLM response");
        let messages = self.get_anthropic_messages();
        log(&format!("[DEBUG] Got {} messages", messages.len()));

        // Get current head as parent
        let mut parents = Vec::new();
        if let Some(chat_id) = &self.current_chat_id {
            if let Ok(Some(chat_info)) = self.store.get_chat_info(chat_id) {
                if let Some(head) = chat_info.head {
                    parents.push(head);
                }
            }
        }

        // Determine which provider to use based on model ID
        let tools;
        if self.openrouter_client.tools_enabled(&model_id) {
            tools = self.get_tools();
        } else {
            tools = None;
        }

        // Call appropriate client
        let result = self
            .openrouter_client
            .generate_response(messages, model_id, tools);

        match result {
            Ok(assistant_msg) => {
                log(&format!("Generated completion: {:?}", assistant_msg));

                // Add LLM response to chain with all parents
                self.add_to_chain(
                    MessageData::Chat(Message::Assistant(assistant_msg.clone())),
                    parents,
                );

                match assistant_msg.finish_reason().as_str() {
                    "stop" => Ok(()),
                    "tool_calls" => todo!(),
                    "length" => todo!(),
                    "content_filter" => todo!(),
                    "error" => Err("llm request returned 200 with an error in the body".into()),
                    _ => Err("unknown stop reason".into()),
                }
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
                                Message::User(UserMessage {
                                    content: _last_content,
                                }),
                                Message::User(UserMessage { content }),
                            ) => {
                                if let Some(Message::User(UserMessage {
                                    content: combined_content,
                                })) = messages.last_mut()
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
        processed_ids: &mut std::collections::HashSet<String>,
    ) {
        // Skip if already processed
        if processed_ids.contains(message_id) {
            log(&format!(
                "[DEBUG] Message {} already processed, skipping",
                message_id
            ));
            return;
        }

        // Try to load the message
        match self.store.load_message(message_id) {
            Ok(entry) => {
                // Mark as processed
                processed_ids.insert(message_id.to_string());
                log(&format!(
                    "[DEBUG] Processing message ID: {}, type: {:?}",
                    message_id,
                    "Chat" // All messages are Chat type now
                ));

                // Log parent information
                if !entry.parents.is_empty() {
                    log(&format!(
                        "[DEBUG] Message {} has {} parents: {:?}",
                        message_id,
                        entry.parents.len(),
                        entry.parents
                    ));
                } else {
                    log(&format!("[DEBUG] Message {} has no parents", message_id));
                }

                // Add to chain
                chain.push(entry.clone());

                // Process all parents recursively
                for parent_id in &entry.parents {
                    log(&format!(
                        "[DEBUG] Processing parent: {} for message: {}",
                        parent_id, message_id
                    ));
                    self.process_message_chain(parent_id, chain, processed_ids);
                }
            }
            Err(e) => {
                log(&format!(
                    "[ERROR] Error loading message {}: {}",
                    message_id, e
                ));
            }
        }
    }

    pub fn get_message(&mut self, message_id: &str) -> Result<ChainEntry, Box<dyn Error>> {
        self.store.load_message(message_id)
    }

    pub fn broadcast_websocket_message(&self, message: &str) -> Result<(), String> {
        use crate::bindings::ntwk::theater::http_framework::send_websocket_message;
        use crate::bindings::ntwk::theater::websocket_types::{MessageType, WebsocketMessage};

        log(&format!(
            "[DEBUG] Broadcasting WebSocket message to {} clients",
            self.connected_clients.len()
        ));

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
                    log(&format!(
                        "[ERROR] Failed to send WebSocket message to client {}: {}",
                        connection_id, e
                    ));
                } else {
                    log(&format!(
                        "[DEBUG] WebSocket message sent to client {}",
                        connection_id
                    ));
                }
            } else {
                log(&format!("[WARN] Invalid client ID format: {}", client_id));
            }
        }

        Ok(())
    }

    pub fn notify_head_update(&self) -> Result<(), String> {
        // Format head update notification
        log(&format!(
            "[DEBUG] Notifying clients of head update: Head={:?}, Chat={:?}",
            self.head, self.current_chat_id
        ));
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
            }
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

    fn get_tools(&self) -> Option<Vec<FunctionDefinition>> {
        // Placeholder for tool retrieval logic
        // This should return a Vec<Tool> based on the current state
        log("[DEBUG] Retrieving tools from MCP servers");
        self.mcp_servers
            .iter()
            .flat_map(|server| server.get_tools())
            .collect::<Vec<_>>()
            .into()
    }
}
