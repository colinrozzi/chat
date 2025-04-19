use crate::bindings::ntwk::theater::message_server_host::request;
use crate::bindings::ntwk::theater::runtime::log;
use crate::bindings::ntwk::theater::supervisor::spawn;
use mcp_protocol::types::tool::Tool;
use serde::{Deserialize, Serialize};
use serde_json::Value;

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

// Actor API request structures
#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type")]
enum McpActorRequest {
    ToolsList {},
    ToolsCall { name: String, args: Value },
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

    pub fn get_tools(&self) -> Vec<Tool> {
        let request_bytes = serde_json::to_vec(&McpActorRequest::ToolsList {}).unwrap();
        let response = request(self.translator_id.as_ref().unwrap(), &request_bytes)
            .expect("Failed to get tools");

        log(&format!(
            "Received response from MCP Actor: {}",
            String::from_utf8_lossy(&response)
        ));

        let response: Value = serde_json::from_slice(&response).unwrap();
        let tools: Vec<Tool> = serde_json::from_value(response["result"]["tools"].clone()).unwrap();

        log(&format!("Parsed tools from MCP Actor: {:?}", tools));

        tools
    }
}
