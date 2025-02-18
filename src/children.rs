use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs;

#[derive(Debug, Serialize, Deserialize)]
pub struct ChildInfo {
    pub name: String,
    pub description: String,
    pub manifest_name: String,
}

pub fn scan_available_children() -> Vec<ChildInfo> {
    let mut children = Vec::new();
    let children_dir = "/Users/colinrozzi/work/actors/chat/assets/children";

    if let Ok(entries) = fs::read_dir(children_dir) {
        for entry in entries.flatten() {
            if let Ok(metadata) = entry.metadata() {
                if metadata.is_file() && entry.path().extension().map_or(false, |ext| ext == "toml") {
                    if let Ok(content) = fs::read_to_string(entry.path()) {
                        if let Ok(manifest) = toml::from_str::<Value>(&content) {
                            if let (Some(name), Some(description)) = (
                                manifest.get("name").and_then(Value::as_str),
                                manifest.get("description").and_then(Value::as_str),
                            ) {
                                let manifest_name = entry
                                    .path()
                                    .file_stem()
                                    .unwrap()
                                    .to_string_lossy()
                                    .into_owned();
                                children.push(ChildInfo {
                                    name: name.to_string(),
                                    description: description.to_string(),
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