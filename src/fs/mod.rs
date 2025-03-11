// File system client for runtime-content-fs
use std::sync::Arc;
use serde_json::{json, Value};
use crate::bindings::ntwk::theater::runtime::log;

#[derive(Debug, Clone)]
pub struct ContentFS {
    actor_id: String,
}

impl Default for ContentFS {
    fn default() -> Self {
        Self {
            actor_id: "default".to_string()
        }
    }
}

impl ContentFS {
    pub fn new(actor_id: String) -> Arc<ContentFS> {
        Arc::new(Self { actor_id })
    }
    
    pub fn read_file(&self, path: &str) -> Result<Vec<u8>, String> {
        let params = json!({
            "path": path
        });
        
        let response = self.send_request("read-file", params)?;
        
        // Extract content
        let content = response.get("content")
            .and_then(|c| c.as_str())
            .ok_or_else(|| "Invalid content in response".to_string())?;
            
        // Convert content to bytes
        Ok(content.as_bytes().to_vec())
    }
    
    pub fn write_file(&self, path: &str, content: &[u8]) -> Result<(), String> {
        let content_str = String::from_utf8_lossy(content).to_string();
        
        let params = json!({
            "path": path,
            "content": content_str
        });
        
        // Send write request
        self.send_request("write-file", params)?;
        
        Ok(())
    }
    
    pub fn list_directory(&self, path: &str) -> Result<Vec<String>, String> {
        let params = json!({
            "path": path
        });
        
        let response = self.send_request("list-directory", params)?;
        
        // Extract entries
        let entries = response.get("entries")
            .and_then(|e| e.as_array())
            .ok_or_else(|| "Invalid entries in response".to_string())?;
            
        // Extract names
        let mut result = Vec::new();
        for entry in entries {
            if let Some(name) = entry.get("name").and_then(|n| n.as_str()) {
                result.push(name.to_string());
            }
        }
        
        Ok(result)
    }
    
    pub fn create_directory(&self, path: &str) -> Result<(), String> {
        let params = json!({
            "path": path
        });
        
        // Send create-directory request
        self.send_request("create-directory", params)?;
        
        Ok(())
    }
    
    pub fn get_info(&self, path: &str) -> Result<Option<FileInfo>, String> {
        // First try to see if this exists at all
        if !self.exists(path)? {
            return Ok(None);
        }
        
        // Try to list as directory
        match self.list_directory(path) {
            Ok(entries) => {
                return Ok(Some(FileInfo {
                    size: entries.len(),
                    is_directory: true
                }));
            }
            Err(_) => {
                // Not a directory, try to read as file to get its size
                match self.read_file(path) {
                    Ok(content) => {
                        return Ok(Some(FileInfo {
                            size: content.len(),
                            is_directory: false
                        }));
                    }
                    Err(e) => {
                        return Err(format!("Failed to get info for {}: {}", path, e));
                    }
                }
            }
        }
    }
    
    pub fn exists(&self, path: &str) -> Result<bool, String> {
        // Try listing as directory first
        match self.list_directory(path) {
            Ok(_) => return Ok(true),
            Err(_) => {
                // Try reading as file
                match self.read_file(path) {
                    Ok(_) => return Ok(true),
                    Err(_) => return Ok(false)
                }
            }
        }
    }
    
    fn send_request(&self, action: &str, params: Value) -> Result<Value, String> {
        let request = json!({
            "action": action,
            "params": params
        });
        
        let request_bytes = serde_json::to_vec(&request)
            .map_err(|e| format!("Failed to serialize request: {}", e))?;
            
        // Send request to runtime-content-fs actor
        let response_bytes = crate::bindings::ntwk::theater::message_server_host::request(&self.actor_id, &request_bytes)
            .map_err(|e| format!("Request to content-fs failed: {}", e))?;
            
        // Parse the response
        let response: Value = serde_json::from_slice(&response_bytes)
            .map_err(|e| format!("Failed to parse response: {}", e))?;
            
        // Check for errors
        if response.get("status").and_then(|s| s.as_str()) == Some("error") {
            let error_msg = response.get("error")
                .and_then(|e| e.as_str())
                .unwrap_or("Unknown error")
                .to_string();
            return Err(error_msg);
        }
        
        // Return the data field
        Ok(response.get("data")
            .cloned()
            .unwrap_or(Value::Null))
    }
}

/// File/directory info structure
#[derive(Debug, Clone)]
pub struct FileInfo {
    pub size: usize,
    pub is_directory: bool,
}