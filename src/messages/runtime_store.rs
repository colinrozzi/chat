use crate::bindings::ntwk::theater::runtime::log;
use crate::bindings::ntwk::theater::store::{self, content_ref};
use crate::messages::ChainEntry;
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
    ) -> Result<ChainEntry, Box<dyn std::error::Error>> {
        log("Saving message to runtime store");
        
        // Serialize the entry to bytes
        let content = serde_json::to_vec(&entry)?;
        
        // Store the content in the runtime store
        let content_ref = store::store(self.store_id.clone(), content)?;
        log(&format!("Stored message with hash: {}", content_ref.hash));
        
        // Set the ID based on the content reference hash
        entry.id = Some(content_ref.hash.clone());
        
        // Update the chat-head label to point to the latest message
        store::replace_at_label(
            self.store_id.clone(), 
            "chat-head".to_string(), 
            content_ref.clone()
        )?;
        log("Updated chat-head label");
        
        // If this is a root message, also label it as chat-root
        if entry.parent.is_none() || entry.parent.as_ref().unwrap() == "null" {
            log("Labeling as chat-root");
            store::label(self.store_id.clone(), "chat-root".to_string(), content_ref.clone())?;
        }
        
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
        let content_ref = content_ref { hash: id.to_string() };
        
        // Retrieve content from store
        let content = store::get(self.store_id.clone(), content_ref)?;
        
        // Deserialize
        let mut msg: ChainEntry = serde_json::from_slice(&content)?;
        msg.id = Some(id.to_string());
        
        // Update cache
        self.cache.insert(id.to_string(), msg.clone());
        log("Message loaded and cached");
        
        Ok(msg)
    }
    
    /// Get the head (latest) message from the chain
    pub fn get_head(&mut self) -> Result<Option<ChainEntry>, Box<dyn std::error::Error>> {
        log("Getting head message from chain");
        
        // Get the head reference from the label
        let head_ref = store::get_by_label(self.store_id.clone(), "chat-head".to_string())?;
        
        if let Some(content_ref) = head_ref {
            log(&format!("Head message found with hash: {}", content_ref.hash));
            return self.load_message(&content_ref.hash);
        }
        
        log("No head message found");
        Ok(None)
    }
    
    /// Get the root (first) message from the chain
    pub fn get_root(&mut self) -> Result<Option<ChainEntry>, Box<dyn std::error::Error>> {
        log("Getting root message from chain");
        
        // Get the root reference from the label
        let root_ref = store::get_by_label(self.store_id.clone(), "chat-root".to_string())?;
        
        if let Some(content_ref) = root_ref {
            log(&format!("Root message found with hash: {}", content_ref.hash));
            return self.load_message(&content_ref.hash);
        }
        
        log("No root message found");
        Ok(None)
    }
}
