use crate::bindings::ntwk::theater::runtime::log;
use crate::bindings::ntwk::theater::store::{self, ContentRef};
use crate::messages::{ChainEntry, ChatInfo};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// MessageStore implementation that uses the Theater runtime's built-in content-addressed store
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MessageStore {
    pub store_id: String,
    pub cache: HashMap<String, ChainEntry>,
}

impl MessageStore {
    /// Create a new MessageStore with the given store ID
    pub fn new(store_id: String) -> Self {
        Self {
            store_id,
            cache: HashMap::new(),
        }
    }

    /// Save a message to the store and return the updated entry with its ID
    pub fn save_message(
        &mut self,
        mut entry: ChainEntry,
        chat_id: &str,
    ) -> Result<ChainEntry, Box<dyn std::error::Error>> {
        log("Saving message to runtime store");

        // Serialize the entry to bytes
        let content = serde_json::to_vec(&entry)?;

        // Store the content in the runtime store
        let content_ref = store::store(&self.store_id, &content)?;
        log(&format!("Stored message with hash: {}", content_ref.hash));

        // Set the ID based on the content reference hash
        entry.id = Some(content_ref.hash.clone());

        // Get the current chat info
        let mut chat_info = self
            .get_chat_info(chat_id)?
            .ok_or_else(|| format!("Chat {} not found", chat_id))?;

        // Update the chat head
        chat_info.head = Some(content_ref.hash.clone());

        // Save the updated chat info
        self.update_chat_info(&chat_info)?;

        // Update cache
        self.cache.insert(content_ref.hash.clone(), entry.clone());

        Ok(entry)
    }

    /// Load a message from the store by its ID
    pub fn load_message(&mut self, id: &str) -> Result<ChainEntry, Box<dyn std::error::Error>> {
        // Check cache first
        if let Some(msg) = self.cache.get(id) {
            return Ok(msg.clone());
        }

        log(&format!("Loading message from runtime store: {}", id));

        // Create content reference
        let content_ref = ContentRef {
            hash: id.to_string(),
        };

        // Retrieve content from store
        let content = store::get(&self.store_id, &content_ref)?;

        // Deserialize
        let mut msg: ChainEntry = serde_json::from_slice(&content)?;
        msg.id = Some(id.to_string());

        // Update cache
        self.cache.insert(id.to_string(), msg.clone());
        log("Message loaded and cached");

        Ok(msg)
    }

    /// Get the head (latest) message from a specific chat
    pub fn get_chat_head(
        &mut self,
        chat_id: &str,
    ) -> Result<Option<ChainEntry>, Box<dyn std::error::Error>> {
        log(&format!("Getting head message for chat {}", chat_id));

        // Get the chat info
        let chat_info = self.get_chat_info(chat_id)?;

        if let Some(chat) = chat_info {
            if let Some(head_id) = chat.head {
                log(&format!("Head message found with hash: {}", head_id));
                let result = self.load_message(&head_id)?;
                return Ok(Some(result));
            }
        }

        log("No head message found for this chat");
        Ok(None)
    }

    /// Get the head (latest) message from the chain (legacy method)
    pub fn get_head(&mut self) -> Result<Option<ChainEntry>, Box<dyn std::error::Error>> {
        log("Getting head message from chain (legacy)");

        // Get the head reference from the label
        let head_ref = store::get_by_label(&self.store_id, "chat-head")?;

        if let Some(content_ref) = head_ref {
            log(&format!(
                "Head message found with hash: {}",
                content_ref.hash
            ));
            let result = self.load_message(&content_ref.hash)?;
            return Ok(Some(result));
        }

        log("No head message found");
        Ok(None)
    }

    /// List all chat IDs
    pub fn list_chat_ids(&self) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        log("Listing all chat IDs");

        // Try to get the chats list from the store
        let chats_ref = store::get_by_label(&self.store_id, "chats")?;

        if let Some(content_ref) = chats_ref {
            let content = store::get(&self.store_id, &content_ref)?;
            let chat_ids: Vec<String> = serde_json::from_slice(&content)?;
            return Ok(chat_ids);
        }

        // If no chats list exists yet, return empty vector
        log("No chats list found");
        Ok(Vec::new())
    }

    /// Get chat info by ID
    pub fn get_chat_info(
        &self,
        chat_id: &str,
    ) -> Result<Option<ChatInfo>, Box<dyn std::error::Error>> {
        log(&format!("Getting chat info for {}", chat_id));

        // Try to get the chat info from the store
        let chat_label = format!("chat_{}", chat_id);
        let chat_ref = store::get_by_label(&self.store_id, &chat_label)?;

        if let Some(content_ref) = chat_ref {
            let content = store::get(&self.store_id, &content_ref)?;
            let chat_info: ChatInfo = serde_json::from_slice(&content)?;
            return Ok(Some(chat_info));
        }

        log(&format!("Chat {} not found", chat_id));
        Ok(None)
    }

    /// Create a new chat
    pub fn create_chat(
        &mut self,
        name: String,
        starting_head: Option<String>,
    ) -> Result<ChatInfo, Box<dyn std::error::Error>> {
        log(&format!("Creating new chat: {}", name));

        let num_chats = self.list_chat_ids()?.len();
        let id = format!("{}", num_chats + 1);

        // Create the chat info
        let chat_info = ChatInfo {
            id: id.clone(),
            name,
            head: starting_head,
            icon: None,
            children: HashMap::new(),
        };

        // Try to store the chat info with enhanced error handling
        match self.update_chat_info(&chat_info) {
            Ok(_) => {
                log(&format!("Chat info stored successfully for {}", id));

                // Add the chat ID to the list of chats with safer operations
                match self.list_chat_ids() {
                    Ok(mut chat_ids) => {
                        if !chat_ids.contains(&id) {
                            chat_ids.push(id.clone());

                            // Serialize with error handling
                            match serde_json::to_vec(&chat_ids) {
                                Ok(content) => {
                                    // Store content with error handling
                                    match store::store(&self.store_id, &content) {
                                        Ok(content_ref) => {
                                            // Update chats label with separate try blocks
                                            log("Updating chats label");
                                            let label_result = if let Ok(Some(_)) =
                                                store::get_by_label(&self.store_id, "chats")
                                            {
                                                log("Replacing existing chats label");
                                                store::replace_at_label(
                                                    &self.store_id,
                                                    "chats",
                                                    &content_ref,
                                                )
                                            } else {
                                                log("Creating new chats label");
                                                store::label(&self.store_id, "chats", &content_ref)
                                            };

                                            if let Err(e) = label_result {
                                                log(&format!(
                                                    "Warning: Failed to update chats label: {}",
                                                    e
                                                ));
                                                // Continue anyway since the chat itself was created
                                            }
                                        }
                                        Err(e) => {
                                            log(&format!(
                                                "Warning: Failed to store chat IDs: {}",
                                                e
                                            ));
                                            // Continue anyway since the chat itself was created
                                        }
                                    }
                                }
                                Err(e) => {
                                    log(&format!("Warning: Failed to serialize chat IDs: {}", e));
                                    // Continue anyway since the chat itself was created
                                }
                            }
                        }
                    }
                    Err(e) => {
                        log(&format!("Warning: Failed to list chat IDs: {}", e));
                        // Continue anyway since the chat itself was created
                    }
                }

                log(&format!("Created chat with ID: {}", id));
                Ok(chat_info)
            }
            Err(e) => {
                log(&format!("Failed to store chat info: {}", e));
                Err(e)
            }
        }
    }

    /// Update chat information
    pub fn update_chat_info(&self, chat: &ChatInfo) -> Result<(), Box<dyn std::error::Error>> {
        log(&format!("Updating chat info for {}", chat.id));

        // Serialize the chat info with error checking
        let content = match serde_json::to_vec(chat) {
            Ok(content) => content,
            Err(e) => {
                log(&format!("Failed to serialize chat info: {}", e));
                return Err(Box::new(e));
            }
        };

        // Store in the content-addressed store with error checking
        let content_ref = match store::store(&self.store_id, &content) {
            Ok(ref_id) => ref_id,
            Err(e) => {
                log(&format!("Failed to store chat content: {}", e));
                return Err(e.into());
            }
        };

        // Update the label with safer operations
        let chat_label = format!("chat_{}", chat.id);

        // Try to check if label exists, but continue even if check fails
        let label_exists = match store::get_by_label(&self.store_id, &chat_label) {
            Ok(Some(_)) => true,
            Ok(None) => false,
            Err(e) => {
                log(&format!(
                    "Warning: Failed to check if chat label exists: {}",
                    e
                ));
                false // Assume it doesn't exist and try to create it
            }
        };

        // Create or update label
        let label_result = if label_exists {
            log(&format!("Replacing chat label: {}", chat_label));
            store::replace_at_label(&self.store_id, &chat_label, &content_ref)
        } else {
            log(&format!("Creating chat label: {}", chat_label));
            store::label(&self.store_id, &chat_label, &content_ref)
        };

        // Handle label operation result
        match label_result {
            Ok(_) => {
                log(&format!("Updated chat info for {}", chat.id));
                Ok(())
            }
            Err(e) => {
                log(&format!("Failed to update chat label: {}", e));
                Err(e.into())
            }
        }
    }

    /// Delete a chat
    pub fn delete_chat(&mut self, chat_id: &str) -> Result<(), Box<dyn std::error::Error>> {
        log(&format!("Deleting chat {}", chat_id));

        // Remove from the list of chats
        let mut chat_ids = self.list_chat_ids()?;
        chat_ids.retain(|id| id != chat_id);

        // Update the chats list
        let content = serde_json::to_vec(&chat_ids)?;
        let content_ref = store::store(&self.store_id, &content)?;
        store::replace_at_label(&self.store_id, "chats", &content_ref)?;

        // We don't delete the actual chat data to allow for potential recovery
        // But we could remove the label if desired
        // let chat_label = format!("chat_{}", chat_id);
        // store::remove_label(&self.store_id, &chat_label)?;

        log(&format!("Deleted chat {}", chat_id));
        Ok(())
    }

    /// Migrate legacy chat to the new format
    pub fn migrate_legacy_chat(&mut self) -> Result<Option<String>, Box<dyn std::error::Error>> {
        log("Checking for legacy chat to migrate");

        // Check if we have any chats already
        let chat_ids = self.list_chat_ids()?;
        if !chat_ids.is_empty() {
            log("Chats already exist, no migration needed");
            return Ok(None);
        }

        // Check if we have a legacy head
        let legacy_head = self.get_head()?;
        if legacy_head.is_none() {
            log("No legacy chat found");
            return Ok(None);
        }

        // Create a new chat with the legacy head
        let legacy_head = legacy_head.unwrap();
        let chat_info = self.create_chat("Default Chat".to_string(), legacy_head.id.clone())?;

        log(&format!(
            "Migrated legacy chat to new format with ID: {}",
            chat_info.id
        ));
        Ok(Some(chat_info.id))
    }
}
