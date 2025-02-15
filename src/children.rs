use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crate::bindings::ntwk::theater::filesystem::{list_files, read_file};
use serde_json::Value;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ChildInfo {
    pub name: String,
    pub description: String,
    pub manifest_name: String,
}

pub fn scan_available_children() -> Vec<ChildInfo> {
    let mut children = Vec::new();
    
    // List all files in the children directory
    if let Ok(files) = list_files("children") {
        for file in files {
            // Only process .toml files
            if file.ends_with(".toml") {
                if let Ok(content) = read_file(&format!("children/{}", file)) {
                    if let Ok(content_str) = String::from_utf8(content) {
                        if let Ok(toml_value) = toml::from_str::<Value>(&content_str) {
                            // Extract metadata from the TOML
                            if let Some(metadata) = toml_value.get("metadata") {
                                let name = metadata.get("name")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("Unknown Actor")
                                    .to_string();
                                
                                let description = metadata.get("description")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("No description available")
                                    .to_string();
                                
                                // Get manifest name by removing .toml extension
                                let manifest_name = file
                                    .strip_suffix(".toml")
                                    .unwrap_or(&file)
                                    .to_string();
                                
                                children.push(ChildInfo {
                                    name,
                                    description,
                                    manifest_name,
                                });
                            }
                        }
                    }
                }
            }
        }
    }
    
    children
}