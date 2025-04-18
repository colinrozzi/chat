use crate::bindings::ntwk::theater::supervisor::spawn;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct McpServerConfig {
    server_path: String,
    args: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct McpServer {
    config: McpServerConfig,
    translator_id: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ToolConfig {}

impl McpServer {
    pub fn new(config: McpServerConfig) -> Self {
        McpServer {
            config,
            translator_id: None,
        }
    }

    pub fn start(&mut self) {
        let mcp_translator = spawn(
            "/Users/colinrozzi/work/actors/mcp-poc/manifest.toml",
            Some(&serde_json::to_vec(&self.config).unwrap()),
        )
        .unwrap();

        self.translator_id = Some(mcp_translator);
    }
}
