use serde::{Deserialize, Serialize};
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
    
    // List all files in the children directory in assets
    if let Ok(files) = list_files("assets/children") {
        for file in files {
            // Only process .toml files
            if file.ends_with(".toml") {
                if let Ok(content) = read_file(&format!("assets/children/{}", file)) {
                    if let Ok(content_str) = String::from_utf8(content) {
                        if let Ok(manifest) = toml::from_str::<toml::Value>(&content_str) {
                            // Get base name without .toml extension for manifest_name
                            let manifest_name = file.strip_suffix(".toml")
                                .unwrap_or(&file)
                                .to_string();

                            // Extract name and description directly from top level
                            let name = manifest.get("name")
                                .and_then(|v| v.as_str())
                                .unwrap_or("Unknown Actor")
                                .to_string();

                            let description = manifest.get("description")
                                .and_then(|v| v.as_str())
                                .unwrap_or("No description available")
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
    
    children
}