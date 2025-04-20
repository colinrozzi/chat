pub mod claude;
pub mod openrouter;

use crate::messages::{AssistantMessage, Message, ModelInfo};
use mcp_protocol::types::tool::Tool;

pub trait LlmApi {
    fn new(api_key: String, model_configs: Vec<ModelInfo>) -> Self
    where
        Self: Sized;
    fn list_available_models(&self) -> Result<Vec<ModelInfo>, Box<dyn std::error::Error>>;
    fn tools_enabled(&self, model_id: &str) -> bool;
    fn generate_response(
        &self,
        messages: Vec<Message>,
        model_id: String,
        available_tools: Option<Vec<Tool>>,
    ) -> Result<AssistantMessage, Box<dyn std::error::Error>>;
}
