# Change Request: Add Google Gemini Support to Chat Actor System

## Overview

This change request outlines the necessary modifications to add support for Google's Gemini models alongside the existing Claude API integration in the Chat Actor System. The implementation will follow a trait-based approach to maintain clean abstractions and allow for future extensions to additional LLM providers.

## Background

Currently, the Chat Actor System only supports Anthropic's Claude API. By adding support for Google's Gemini models, we can offer users more choice and flexibility in their chat interactions. This will require changes to the message structure, API clients, state management, and UI components.

## Requirements

1. Users should be able to select from both Claude and Gemini models
2. The system should maintain separate API clients for each provider
3. Message history should be preserved regardless of which provider generated responses
4. The UI should display appropriate information for each provider
5. Costs and token usage should be tracked appropriately for both providers

## Technical Design

### 1. Define LlmMessage Trait

Create a new trait that defines common behavior for all LLM messages:

```rust
pub trait LlmMessage {
    fn content(&self) -> &str;
    fn model_id(&self) -> &str;
    fn provider_name(&self) -> &str;
    fn input_tokens(&self) -> u32;
    fn output_tokens(&self) -> u32;
    fn calculate_cost(&self) -> f64;
    fn stop_reason(&self) -> &str;
    fn message_id(&self) -> &str;
    fn provider_data(&self) -> Option<serde_json::Value>;
}
```

### 2. Provider-Specific Message Types

Create concrete message types for each provider:

```rust
// Claude-specific message implementation
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ClaudeMessage {
    pub content: String,
    pub id: String,
    pub model: String,
    pub stop_reason: String,
    pub stop_sequence: Option<String>,
    pub message_type: String,
    pub usage: ClaudeUsage,
    pub input_cost_per_million_tokens: Option<f64>,
    pub output_cost_per_million_tokens: Option<f64>,
}

// Gemini-specific message implementation
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GeminiMessage {
    pub content: String,
    pub id: String,
    pub model: String,
    pub finish_reason: String,
    pub safety_ratings: Option<Vec<SafetyRating>>,
    pub usage: GeminiUsage,
    pub input_cost_per_million_tokens: Option<f64>,
    pub output_cost_per_million_tokens: Option<f64>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GeminiUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SafetyRating {
    pub category: String,
    pub probability: String,
}
```

### 3. Update Message Enum

Update the Message enum to use the new message types:

```rust
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum AssistantMessage {
    Claude(ClaudeMessage),
    Gemini(GeminiMessage),
}

impl LlmMessage for AssistantMessage {
    // Delegate to inner implementations
    fn content(&self) -> &str {
        match self {
            AssistantMessage::Claude(msg) => msg.content(),
            AssistantMessage::Gemini(msg) => msg.content(),
        }
    }
    
    // Implement other methods similarly...
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Message {
    User {
        content: String,
    },
    Assistant(AssistantMessage),
}
```

### 4. Create Gemini API Client

Implement a new client for the Gemini API:

```rust
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GeminiClient {
    api_key: String,
}

impl GeminiClient {
    pub fn new(api_key: String) -> Self {
        Self { api_key }
    }

    pub fn generate_response(
        &self,
        messages: Vec<Message>,
        model_id: Option<String>,
    ) -> Result<AssistantMessage, Box<dyn std::error::Error>> {
        // Get the model ID or use default
        let model = model_id.unwrap_or_else(|| "gemini-2.0-flash".to_string());
        
        // Convert our internal message format to Gemini format
        let gemini_contents: Vec<GeminiContent> = messages
            .iter()
            .map(|msg| {
                match msg {
                    Message::User { content } => GeminiContent {
                        role: "user".to_string(),
                        parts: vec![GeminiPart { text: content.clone() }],
                    },
                    Message::Assistant(assistant_msg) => GeminiContent {
                        role: "model".to_string(),
                        parts: vec![GeminiPart { text: assistant_msg.content().to_string() }],
                    },
                }
            })
            .collect();

        // Prepare the HTTP request
        let request = HttpRequest {
            method: "POST".to_string(),
            uri: format!(
                "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
                model, self.api_key
            ),
            headers: vec![
                ("Content-Type".to_string(), "application/json".to_string()),
            ],
            body: Some(serde_json::to_vec(&GeminiRequest { contents: gemini_contents })?),
        };

        // Send the request
        let http_response = send_http(&request)
            .map_err(|e| format!("HTTP request failed: {}", e))?;
        
        // Parse the response
        let body = http_response.body.ok_or("No response body")?;
        let response: GeminiResponse = serde_json::from_slice(&body)?;
        
        // Extract the response content
        if response.candidates.is_empty() {
            return Err("No response candidates".into());
        }
        
        let candidate = &response.candidates[0];
        
        // Extract text from the response
        let parts = &candidate.content.parts;
        if parts.is_empty() {
            return Err("No text parts in response".into());
        }
        
        let content = parts[0].text.clone();
        
        // Get pricing for this model
        let pricing = get_model_pricing(&model);
        
        // Create our message
        let gemini_message = GeminiMessage {
            content,
            id: uuid::Uuid::new_v4().to_string(),
            model,
            finish_reason: candidate.finish_reason.clone(),
            safety_ratings: candidate.safety_ratings.clone(),
            usage: response.usage.clone(),
            input_cost_per_million_tokens: pricing.input_cost_per_million_tokens,
            output_cost_per_million_tokens: pricing.output_cost_per_million_tokens,
        };
        
        // Wrap in the enum
        Ok(AssistantMessage::Gemini(gemini_message))
    }
    
    pub fn list_available_models(&self) -> Result<Vec<ModelInfo>, Box<dyn std::error::Error>> {
        // Return available Gemini models
        Ok(vec![
            ModelInfo {
                id: "gemini-2.0-flash".to_string(),
                display_name: "Gemini 2.0 Flash".to_string(),
                max_tokens: 32768,
                provider: Some("gemini".to_string()),
            },
            ModelInfo {
                id: "gemini-2.0-pro".to_string(),
                display_name: "Gemini 2.0 Pro".to_string(),
                max_tokens: 32768,
                provider: Some("gemini".to_string()),
            },
        ])
    }
}
```

### 5. Update State Structure

Modify the State struct to include both API clients:

```rust
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct State {
    // Existing fields...
    pub claude_client: ClaudeClient,
    pub gemini_client: GeminiClient,
    // Rest of existing fields...
}

impl State {
    pub fn new(
        id: String,
        store_id: String,
        anthropic_api_key: String,
        gemini_api_key: String,
        server_id: u64,
        websocket_port: u16,
        head: Option<String>,
    ) -> Self {
        let mut state = Self {
            id,
            head,
            current_chat_id: None,
            claude_client: ClaudeClient::new(anthropic_api_key.clone()),
            gemini_client: GeminiClient::new(gemini_api_key.clone()),
            connected_clients: HashMap::new(),
            store: MessageStore::new(store_id.clone()),
            server_id,
            websocket_port,
            children: HashMap::new(),
            actor_messages: HashMap::new(),
        };

        // Rest of initialization
        // ...

        state
    }
    
    // Update generate_llm_response to handle both providers
    pub fn generate_llm_response(&mut self, model_id: Option<String>) -> Result<(), Box<dyn std::error::Error>> {
        log("[DEBUG] Getting messages");
        let messages = self.get_anthropic_messages();
        log(&format!(
            "[DEBUG] Got {} messages",
            messages.len()
        ));

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
        let model = model_id.clone().unwrap_or_else(|| "claude-3-7-sonnet-20250219".to_string());
        let is_gemini = model.starts_with("gemini-");
        
        // Log which model is being used
        if let Some(model) = &model_id {
            log(&format!("[DEBUG] Using specified model: {}", model));
        } else if is_gemini {
            log("[DEBUG] Using default Gemini model (gemini-2.0-flash)");
        } else {
            log("[DEBUG] Using default Claude model (claude-3-7-sonnet-20250219)");
        }

        // Call appropriate client
        let result = if is_gemini {
            self.gemini_client.generate_response(messages, model_id)
        } else {
            self.claude_client.generate_response(messages, model_id)
        };

        match result {
            Ok(assistant_msg) => {
                log(&format!("Generated completion: {:?}", assistant_msg));

                // Add LLM response to chain with all parents
                self.add_to_chain(MessageData::Chat(Message::Assistant(assistant_msg)), parents);

                Ok(())
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
}
```

### 6. Update Initialization

Modify the initialization to include the Gemini API key:

```rust
#[derive(Serialize, Deserialize, Debug)]
struct InitData {
    head: Option<String>,
    websocket_port: u16,
    store_id: Option<String>,
    anthropic_api_key: String,
    gemini_api_key: String,  // Add this field
    assets_store_id: Option<String>,
    assets_runtime_content_fs: Option<String>,
}

impl ActorGuest for Component {
    fn init(data: Option<Vec<u8>>, params: (String,)) -> Result<(Option<Vec<u8>>,), String> {
        // Existing code...
        
        let init_data: InitData = serde_json::from_slice(&data).unwrap();
        
        // Existing code...
        
        // Initialize state with both API keys
        let initial_state = State::new(
            id,
            store_id,
            init_data.anthropic_api_key,
            init_data.gemini_api_key,
            server_id,
            init_data.websocket_port,
            init_data.head,
        );
        
        // Existing code...
        Ok((Some(serde_json::to_vec(&initial_state).unwrap()),))
    }
}
```

### 7. Add Model Information

Add Gemini models to pricing and token functions:

```rust
// Helper function to get max tokens for a given model
fn get_model_max_tokens(model_id: &str) -> u32 {
    match model_id {
        // Claude models...
        
        // Gemini models
        "gemini-2.0-flash" => 32768,
        "gemini-2.0-pro" => 32768,
        
        // Default case
        _ => 4096,
    }
}

// Pricing structure for Gemini models
pub fn get_model_pricing(model_id: &str) -> ModelPricing {
    match model_id {
        // Claude models...
        
        // Gemini models
        "gemini-2.0-flash" => ModelPricing {
            input_cost_per_million_tokens: Some(0.35),
            output_cost_per_million_tokens: Some(1.05),
        },
        "gemini-2.0-pro" => ModelPricing {
            input_cost_per_million_tokens: Some(3.50),
            output_cost_per_million_tokens: Some(10.50),
        },
        
        // Default case
        _ => ModelPricing {
            input_cost_per_million_tokens: None,
            output_cost_per_million_tokens: None,
        },
    }
}
```

### 8. Update WebSocket Handler

Modify the list_models handler to include Gemini models:

```rust
fn handle_list_models(
    state: &State,
) -> Result<(Option<Vec<u8>>, (WebsocketResponse,)), String> {
    // Get Claude models
    let claude_models = match state.claude_client.list_available_models() {
        Ok(models) => models,
        Err(e) => {
            log(&format!("Failed to list Claude models: {}", e));
            vec![] // Return empty list on error
        }
    };
    
    // Add provider field to Claude models
    let claude_models_with_provider: Vec<Value> = claude_models
        .iter()
        .map(|model| {
            json!({
                "id": model.id,
                "display_name": model.display_name,
                "max_tokens": model.max_tokens,
                "provider": "claude"
            })
        })
        .collect();
    
    // Get Gemini models
    let gemini_models = match state.gemini_client.list_available_models() {
        Ok(models) => models,
        Err(e) => {
            log(&format!("Failed to list Gemini models: {}", e));
            vec![] // Return empty list on error
        }
    };
    
    // Add provider field to Gemini models
    let gemini_models_with_provider: Vec<Value> = gemini_models
        .iter()
        .map(|model| {
            json!({
                "id": model.id,
                "display_name": model.display_name,
                "max_tokens": model.max_tokens,
                "provider": "gemini"
            })
        })
        .collect();
    
    // Combine all models
    let all_models: Vec<Value> = [
        claude_models_with_provider,
        gemini_models_with_provider,
    ].concat();
    
    Ok((
        Some(serde_json::to_vec(state).unwrap()),
        (WebsocketResponse {
            messages: vec![WebsocketMessage {
                ty: MessageType::Text,
                text: Some(
                    json!({
                        "type": "models_list",
                        "models": all_models
                    })
                    .to_string(),
                ),
                data: None,
            }],
        },),
    ))
}
```

### 9. Update UI

Modify the JavaScript to group models by provider and display provider-specific information:

```javascript
// Update the populateModelSelector function to group by provider
function populateModelSelector() {
    if (!elements.controlsModelSelector || !models || models.length === 0) return;
    
    // Save the currently selected model if any
    const currentSelection = elements.controlsModelSelector.value;
    
    // Group models by provider
    const claudeModels = models.filter(m => m.provider === 'claude' || !m.provider);
    const geminiModels = models.filter(m => m.provider === 'gemini');
    
    // Clear current options
    elements.controlsModelSelector.innerHTML = '';
    
    // Create Claude group
    const claudeGroup = document.createElement('optgroup');
    claudeGroup.label = 'Claude Models';
    
    // Sort Claude models with the most recent first
    const sortedClaudeModels = [...claudeModels].sort((a, b) => {
        if (a.id === 'claude-3-7-sonnet-20250219') return -1;
        if (b.id === 'claude-3-7-sonnet-20250219') return 1;
        return b.id.localeCompare(a.id);
    });
    
    // Add Claude options
    sortedClaudeModels.forEach(model => {
        const option = document.createElement('option');
        option.value = model.id;
        option.textContent = model.display_name;
        claudeGroup.appendChild(option);
    });
    
    // Create Gemini group
    const geminiGroup = document.createElement('optgroup');
    geminiGroup.label = 'Gemini Models';
    
    // Add Gemini options
    geminiModels.forEach(model => {
        const option = document.createElement('option');
        option.value = model.id;
        option.textContent = model.display_name;
        geminiGroup.appendChild(option);
    });
    
    // Add groups to selector
    elements.controlsModelSelector.appendChild(claudeGroup);
    elements.controlsModelSelector.appendChild(geminiGroup);
    
    // Set selected model (same logic as before)
    // ...
}

// Update renderMessage to show provider in the metadata
function renderMessage(message) {
    // Existing code...
    
    if (msg.Assistant) {
        const assistant = msg.Assistant;
        const provider = assistant.provider || 'claude'; // Fallback for backward compatibility
        
        // Existing code...
        
        // Update display to include provider
        return `
            <div class="message assistant ${smallClass}" data-message-id="${message.id}">
                ${formatMessageContent(assistant.content)}
                <div class="message-actions">
                    <!-- Existing code... -->
                </div>
                <div class="message-metadata">
                    <div class="metadata-item">
                        <span class="metadata-label">Provider:</span> ${provider}
                    </div>
                    <div class="metadata-item">
                        <span class="metadata-label">Model:</span> ${assistant.model}
                    </div>
                    <!-- Rest of metadata... -->
                </div>
            </div>
        `;
    }
}
```

### 10. Add Migration Support

Implement logic to migrate existing messages to the new format:

```rust
// In LlmMessage implementations
impl LlmMessage for ClaudeMessage {
    // Implementation methods...
}

impl LlmMessage for AssistantMessage {
    // Delegate to inner implementations
    fn content(&self) -> &str {
        match self {
            AssistantMessage::Claude(msg) => msg.content(),
            AssistantMessage::Gemini(msg) => msg.content(),
        }
    }
    
    // Other delegations...
}

// Implement migration function
fn migrate_message(old_message: OldAssistantMessage) -> AssistantMessage {
    // Convert old Claude-specific format to new format
    ClaudeMessage {
        content: old_message.content,
        id: old_message.id,
        model: old_message.model,
        stop_reason: old_message.stop_reason,
        stop_sequence: old_message.stop_sequence,
        message_type: old_message.message_type,
        usage: old_message.usage,
        input_cost_per_million_tokens: old_message.input_cost_per_million_tokens,
        output_cost_per_million_tokens: old_message.output_cost_per_million_tokens,
    }.into()
}

// Add a From implementation to make conversion easier
impl From<ClaudeMessage> for AssistantMessage {
    fn from(msg: ClaudeMessage) -> Self {
        AssistantMessage::Claude(msg)
    }
}

impl From<GeminiMessage> for AssistantMessage {
    fn from(msg: GeminiMessage) -> Self {
        AssistantMessage::Gemini(msg)
    }
}
```

## Implementation Plan

1. Create the LlmMessage trait and new message types
2. Implement the trait for both message types
3. Create the GeminiClient
4. Update the State to handle both clients
5. Update initialization and model information
6. Modify WebSocket handlers
7. Update UI components
8. Implement migration logic for existing messages
9. Test thoroughly with both providers

## Acceptance Criteria

1. Users can select from both Claude and Gemini models in the UI
2. Users can send messages and receive responses from both providers
3. Message history is preserved and can be viewed regardless of provider
4. The UI displays appropriate information for each provider
5. Costs and token usage are tracked correctly for both providers
6. Existing chats and messages continue to work after migration

## Risks and Mitigations

1. **Risk**: Incompatible message formats between providers
   **Mitigation**: Use the trait-based approach to abstract provider differences

2. **Risk**: Breaking existing functionality for Claude
   **Mitigation**: Implement thorough tests for both providers and ensure backward compatibility

3. **Risk**: Pricing or token counting inconsistencies
   **Mitigation**: Double-check pricing information and validate token calculations

4. **Risk**: Migration issues for existing messages
   **Mitigation**: Implement robust migration logic with fallbacks

## References

1. [Gemini API Documentation](https://ai.google.dev/docs/gemini_api)
2. [Claude API Documentation](https://docs.anthropic.com/claude/reference)
