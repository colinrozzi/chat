use crate::bindings::ntwk::theater::runtime::log;
use crate::bindings::ntwk::theater::types::Json;
use crate::bindings::ntwk::theater::websocket_types::{MessageType, WebsocketMessage};
use crate::state::State;
use serde_json::{json, Value};

// Define a new type for WebsocketResponse to match the old API
pub struct WebsocketResponse {
    pub messages: Vec<WebsocketMessage>,
}

// Helper function to create a messages_updated response
fn create_messages_updated_response(state: &State) -> WebsocketResponse {
    WebsocketResponse {
        messages: vec![WebsocketMessage {
            ty: MessageType::Text,
            text: Some(
                json!({
                    "type": "messages_updated",
                    "head": state.head,
                    "current_chat_id": state.current_chat_id
                })
                .to_string(),
            ),
            data: None,
        }],
    }
}

pub fn handle_message(
    msg: WebsocketMessage,
    state: Json,
) -> Result<(Option<Vec<u8>>, (WebsocketResponse,)), String> {
    log("Handling WebSocket message");
    log(&format!("Message: {:?}", msg));

    let mut current_state: State = serde_json::from_slice(&state).unwrap();

    match msg.ty {
        MessageType::Text => {
            if let Some(ref text) = msg.text {
                if let Ok(command) = serde_json::from_str::<Value>(text) {
                    match command["type"].as_str() {
                        // Chat management commands
                        Some("list_chats") => handle_list_chats(&current_state),
                        Some("create_chat") => {
                            let name = command["name"].as_str().unwrap_or("New Chat");
                            let starting_head = command["starting_head"].as_str().map(String::from);
                            handle_create_chat(&mut current_state, name, starting_head)
                        }
                        Some("switch_chat") => {
                            if let Some(chat_id) = command["chat_id"].as_str() {
                                handle_switch_chat(&mut current_state, chat_id)
                            } else {
                                default_response(&current_state)
                            }
                        }
                        Some("rename_chat") => {
                            if let (Some(chat_id), Some(name)) =
                                (command["chat_id"].as_str(), command["name"].as_str())
                            {
                                handle_rename_chat(&mut current_state, chat_id, name)
                            } else {
                                default_response(&current_state)
                            }
                        }
                        Some("delete_chat") => {
                            if let Some(chat_id) = command["chat_id"].as_str() {
                                handle_delete_chat(&mut current_state, chat_id)
                            } else {
                                default_response(&current_state)
                            }
                        }

                        // Message commands
                        Some("send_message") => {
                            if let Some(content) = command["content"].as_str() {
                                handle_send_message(&mut current_state, content)
                            } else {
                                default_response(&current_state)
                            }
                        }
                        Some("generate_llm_response") => {
                            // Extract optional model ID from the message
                            let model_id = if let Some(model) = command["model_id"].as_str() {
                                Some(model.to_string())
                            } else {
                                None
                            };
                            
                            handle_generate_llm_response(&mut current_state, model_id)
                        }
                        
                        Some("list_models") => {
                            handle_list_models(&current_state)
                        }
                        Some("get_message") => {
                            if let Some(message_id) = command["message_id"].as_str() {
                                handle_get_message(&mut current_state, message_id)
                            } else {
                                default_response(&current_state)
                            }
                        }
                        Some("get_head") => handle_get_head(&current_state),

                        _ => default_response(&current_state),
                    }
                } else {
                    default_response(&current_state)
                }
            } else {
                default_response(&current_state)
            }
        }
        _ => default_response(&current_state),
    }
}

// Chat management handlers
fn handle_list_chats(state: &State) -> Result<(Option<Vec<u8>>, (WebsocketResponse,)), String> {
    // Send a chats update message to the client
    if let Err(e) = state.notify_chats_update() {
        log(&format!("Failed to notify chats update: {}", e));
    }

    // Return a direct response as well (redundant but safe)
    let mut chats = Vec::new();
    if let Ok(chat_ids) = state.store.list_chat_ids() {
        for chat_id in chat_ids {
            if let Ok(Some(chat_info)) = state.store.get_chat_info(&chat_id) {
                chats.push(json!({
                    "id": chat_info.id,
                    "name": chat_info.name,
                    "icon": chat_info.icon,
                }));
            }
        }
    }

    Ok((
        Some(serde_json::to_vec(state).unwrap()),
        (WebsocketResponse {
            messages: vec![WebsocketMessage {
                ty: MessageType::Text,
                text: Some(
                    json!({
                        "type": "chats_update",
                        "chats": chats,
                        "current_chat_id": state.current_chat_id
                    })
                    .to_string(),
                ),
                data: None,
            }],
        },),
    ))
}

fn handle_create_chat(
    state: &mut State,
    name: &str,
    starting_head: Option<String>,
) -> Result<(Option<Vec<u8>>, (WebsocketResponse,)), String> {
    match state.create_chat(name.to_string(), starting_head) {
        Ok(chat_info) => {
            // Notify all clients about chats update
            if let Err(e) = state.notify_chats_update() {
                log(&format!("Failed to notify chats update: {}", e));
            }

            Ok((
                Some(serde_json::to_vec(state).unwrap()),
                (WebsocketResponse {
                    messages: vec![
                        WebsocketMessage {
                            ty: MessageType::Text,
                            text: Some(
                                json!({
                                    "type": "chat_created",
                                    "chat": {
                                        "id": chat_info.id,
                                        "name": chat_info.name,
                                    }
                                })
                                .to_string(),
                            ),
                            data: None,
                        },
                        WebsocketMessage {
                            ty: MessageType::Text,
                            text: Some(
                                json!({
                                    "type": "messages_updated",
                                    "head": state.head,
                                    "current_chat_id": state.current_chat_id
                                })
                                .to_string(),
                            ),
                            data: None,
                        },
                    ],
                },),
            ))
        }
        Err(e) => {
            log(&format!("Failed to create chat: {}", e));
            Ok((
                Some(serde_json::to_vec(state).unwrap()),
                (WebsocketResponse {
                    messages: vec![WebsocketMessage {
                        ty: MessageType::Text,
                        text: Some(
                            json!({
                                "type": "error",
                                "message": format!("Failed to create chat: {}", e)
                            })
                            .to_string(),
                        ),
                        data: None,
                    }],
                },),
            ))
        }
    }
}

fn handle_switch_chat(
    state: &mut State,
    chat_id: &str,
) -> Result<(Option<Vec<u8>>, (WebsocketResponse,)), String> {
    match state.switch_chat(chat_id) {
        Ok(_) => {
            // Head update is already sent by the switch_chat method
            Ok((
                Some(serde_json::to_vec(state).unwrap()),
                (create_messages_updated_response(state),),
            ))
        }
        Err(e) => {
            log(&format!("Failed to switch chat: {}", e));
            Ok((
                Some(serde_json::to_vec(state).unwrap()),
                (WebsocketResponse {
                    messages: vec![WebsocketMessage {
                        ty: MessageType::Text,
                        text: Some(
                            json!({
                                "type": "error",
                                "message": format!("Failed to switch chat: {}", e)
                            })
                            .to_string(),
                        ),
                        data: None,
                    }],
                },),
            ))
        }
    }
}

fn handle_rename_chat(
    state: &mut State,
    chat_id: &str,
    name: &str,
) -> Result<(Option<Vec<u8>>, (WebsocketResponse,)), String> {
    // Get the current chat info
    match state.store.get_chat_info(chat_id) {
        Ok(Some(mut chat_info)) => {
            // Update the name
            chat_info.name = name.to_string();
            // Save the updated chat info
            if let Err(e) = state.store.update_chat_info(&chat_info) {
                log(&format!("Failed to update chat info: {}", e));
                return Ok((
                    Some(serde_json::to_vec(state).unwrap()),
                    (WebsocketResponse {
                        messages: vec![WebsocketMessage {
                            ty: MessageType::Text,
                            text: Some(
                                json!({
                                    "type": "error",
                                    "message": format!("Failed to rename chat: {}", e)
                                })
                                .to_string(),
                            ),
                            data: None,
                        }],
                    },),
                ));
            }

            // Notify all clients about chats update
            if let Err(e) = state.notify_chats_update() {
                log(&format!("Failed to notify chats update: {}", e));
            }

            Ok((
                Some(serde_json::to_vec(state).unwrap()),
                (WebsocketResponse {
                    messages: vec![WebsocketMessage {
                        ty: MessageType::Text,
                        text: Some(
                            json!({
                                "type": "chat_renamed",
                                "chat": {
                                    "id": chat_info.id,
                                    "name": chat_info.name,
                                }
                            })
                            .to_string(),
                        ),
                        data: None,
                    }],
                },),
            ))
        }
        Ok(None) => {
            log(&format!("Chat not found: {}", chat_id));
            Ok((
                Some(serde_json::to_vec(state).unwrap()),
                (WebsocketResponse {
                    messages: vec![WebsocketMessage {
                        ty: MessageType::Text,
                        text: Some(
                            json!({
                                "type": "error",
                                "message": format!("Chat not found: {}", chat_id)
                            })
                            .to_string(),
                        ),
                        data: None,
                    }],
                },),
            ))
        }
        Err(e) => {
            log(&format!("Failed to get chat info: {}", e));
            Ok((
                Some(serde_json::to_vec(state).unwrap()),
                (WebsocketResponse {
                    messages: vec![WebsocketMessage {
                        ty: MessageType::Text,
                        text: Some(
                            json!({
                                "type": "error",
                                "message": format!("Failed to get chat info: {}", e)
                            })
                            .to_string(),
                        ),
                        data: None,
                    }],
                },),
            ))
        }
    }
}

fn handle_delete_chat(
    state: &mut State,
    chat_id: &str,
) -> Result<(Option<Vec<u8>>, (WebsocketResponse,)), String> {
    match state.delete_chat(chat_id) {
        Ok(_) => {
            // Notify all clients about chats update
            if let Err(e) = state.notify_chats_update() {
                log(&format!("Failed to notify chats update: {}", e));
            }

            Ok((
                Some(serde_json::to_vec(state).unwrap()),
                (WebsocketResponse {
                    messages: vec![
                        WebsocketMessage {
                            ty: MessageType::Text,
                            text: Some(
                                json!({
                                    "type": "chat_deleted",
                                    "chat_id": chat_id
                                })
                                .to_string(),
                            ),
                            data: None,
                        },
                        WebsocketMessage {
                            ty: MessageType::Text,
                            text: Some(
                                json!({
                                    "type": "messages_updated",
                                    "head": state.head,
                                    "current_chat_id": state.current_chat_id
                                })
                                .to_string(),
                            ),
                            data: None,
                        },
                    ],
                },),
            ))
        }
        Err(e) => {
            log(&format!("Failed to delete chat: {}", e));
            Ok((
                Some(serde_json::to_vec(state).unwrap()),
                (WebsocketResponse {
                    messages: vec![WebsocketMessage {
                        ty: MessageType::Text,
                        text: Some(
                            json!({
                                "type": "error",
                                "message": format!("Failed to delete chat: {}", e)
                            })
                            .to_string(),
                        ),
                        data: None,
                    }],
                },),
            ))
        }
    }
}

// Message handlers
fn handle_send_message(
    state: &mut State,
    content: &str,
) -> Result<(Option<Vec<u8>>, (WebsocketResponse,)), String> {
    state.add_user_message(content);

    // Use the helper function to create standardized response
    Ok((
        Some(serde_json::to_vec(state).unwrap()),
        (create_messages_updated_response(state),),
    ))
}

fn handle_generate_llm_response(
    state: &mut State,
    model_id: Option<String>,
) -> Result<(Option<Vec<u8>>, (WebsocketResponse,)), String> {
    match state.generate_llm_response(model_id) {
        Ok(_) => {
            // Response success - head will have been updated
            Ok((
                Some(serde_json::to_vec(state).unwrap()),
                (create_messages_updated_response(state),),
            ))
        }
        Err(e) => {
            // Error already logged and notified by the generate_llm_response method
            // Just return the current state
            Ok((
                Some(serde_json::to_vec(state).unwrap()),
                (WebsocketResponse {
                    messages: vec![WebsocketMessage {
                        ty: MessageType::Text,
                        text: Some(
                            json!({
                                "type": "error",
                                "message": format!("Failed to generate LLM response: {}", e)
                            })
                            .to_string(),
                        ),
                        data: None,
                    }],
                },),
            ))
        }
    }
}

fn handle_get_message(
    state: &mut State,
    message_id: &str,
) -> Result<(Option<Vec<u8>>, (WebsocketResponse,)), String> {
    match state.get_message(message_id) {
        Ok(message) => Ok((
            Some(serde_json::to_vec(state).unwrap()),
            (WebsocketResponse {
                messages: vec![WebsocketMessage {
                    ty: MessageType::Text,
                    text: Some(
                        json!({
                            "type": "message",
                            "message": message
                        })
                        .to_string(),
                    ),
                    data: None,
                }],
            },),
        )),
        Err(e) => {
            log(&format!("Failed to get message: {}", e));
            Ok((
                Some(serde_json::to_vec(state).unwrap()),
                (WebsocketResponse {
                    messages: vec![WebsocketMessage {
                        ty: MessageType::Text,
                        text: Some(
                            json!({
                                "type": "error",
                                "message": format!("Failed to get message: {}", e)
                            })
                            .to_string(),
                        ),
                        data: None,
                    }],
                },),
            ))
        }
    }
}

fn handle_get_head(state: &State) -> Result<(Option<Vec<u8>>, (WebsocketResponse,)), String> {
    Ok((
        Some(serde_json::to_vec(state).unwrap()),
        (WebsocketResponse {
            messages: vec![WebsocketMessage {
                ty: MessageType::Text,
                text: Some(
                    json!({
                        "type": "head",
                        "head": state.head,
                        "current_chat_id": state.current_chat_id
                    })
                    .to_string(),
                ),
                data: None,
            }],
        },),
    ))
}

fn handle_list_models(
    state: &State,
) -> Result<(Option<Vec<u8>>, (WebsocketResponse,)), String> {
    match state.claude_client.list_available_models() {
        Ok(models) => {
            Ok((
                Some(serde_json::to_vec(state).unwrap()),
                (WebsocketResponse {
                    messages: vec![WebsocketMessage {
                        ty: MessageType::Text,
                        text: Some(
                            json!({
                                "type": "models_list",
                                "models": models
                            })
                            .to_string(),
                        ),
                        data: None,
                    }],
                },),
            ))
        }
        Err(e) => {
            log(&format!("Failed to list models: {}", e));
            Ok((
                Some(serde_json::to_vec(state).unwrap()),
                (WebsocketResponse {
                    messages: vec![WebsocketMessage {
                        ty: MessageType::Text,
                        text: Some(
                            json!({
                                "type": "error",
                                "message": format!("Failed to list models: {}", e)
                            })
                            .to_string(),
                        ),
                        data: None,
                    }],
                },),
            ))
        }
    }
}

fn default_response(state: &State) -> Result<(Option<Vec<u8>>, (WebsocketResponse,)), String> {
    Ok((
        Some(serde_json::to_vec(state).unwrap()),
        (WebsocketResponse {
            messages: vec![WebsocketMessage {
                ty: MessageType::Text,
                text: Some(
                    json!({
                        "type": "error",
                        "message": "Invalid command"
                    })
                    .to_string(),
                ),
                data: None,
            }],
        },),
    ))
}
