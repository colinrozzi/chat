use crate::bindings::ntwk::theater::runtime::log;
use crate::bindings::ntwk::theater::types::Json;
use crate::bindings::ntwk::theater::websocket_types::{MessageType, WebsocketMessage};
use crate::children::scan_available_children;
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

                        // Child actor commands
                        Some("get_available_children") => {
                            handle_get_available_children(&current_state)
                        }
                        Some("get_running_children") => handle_get_running_children(&current_state),
                        Some("start_child") => {
                            if let Some(manifest_name) = command["manifest_name"].as_str() {
                                handle_start_child(&mut current_state, manifest_name)
                            } else {
                                default_response(&current_state)
                            }
                        }
                        Some("stop_child") => {
                            if let Some(actor_id) = command["actor_id"].as_str() {
                                handle_stop_child(&mut current_state, actor_id)
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
                            handle_generate_llm_response(&mut current_state)
                        }
                        Some("get_message") => {
                            if let Some(message_id) = command["message_id"].as_str() {
                                handle_get_message(&mut current_state, message_id)
                            } else {
                                default_response(&current_state)
                            }
                        }
                        Some("get_pending_child_messages") => {
                            handle_get_pending_child_messages(&current_state)
                        }
                        Some("toggle_pending_child_message") => {
                            if let (Some(message_id), Some(selected)) = (
                                command["message_id"].as_str(),
                                command["selected"].as_bool(),
                            ) {
                                handle_toggle_pending_child_message(
                                    &mut current_state,
                                    message_id,
                                    selected,
                                )
                            } else {
                                default_response(&current_state)
                            }
                        }
                        Some("remove_pending_child_message") => {
                            if let Some(message_id) = command["message_id"].as_str() {
                                handle_remove_pending_child_message(&mut current_state, message_id)
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

// Child actor handlers
fn handle_get_available_children(
    state: &State,
) -> Result<(Option<Vec<u8>>, (WebsocketResponse,)), String> {
    let available_children = scan_available_children(&state.filesystem)
        .into_iter()
        .map(|child| {
            json!({
                "name": child.name,
                "description": child.description,
                "manifest_name": child.manifest_name
            })
        })
        .collect::<Vec<Value>>();

    Ok((
        Some(serde_json::to_vec(state).unwrap()),
        (WebsocketResponse {
            messages: vec![WebsocketMessage {
                ty: MessageType::Text,
                text: Some(
                    json!({
                        "type": "children_update",
                        "available_children": available_children
                    })
                    .to_string(),
                ),
                data: None,
            }],
        },),
    ))
}

fn handle_get_running_children(
    state: &State,
) -> Result<(Option<Vec<u8>>, (WebsocketResponse,)), String> {
    let running_children = state.list_running_children();

    Ok((
        Some(serde_json::to_vec(state).unwrap()),
        (WebsocketResponse {
            messages: vec![WebsocketMessage {
                ty: MessageType::Text,
                text: Some(
                    json!({
                        "type": "children_update",
                        "running_children": running_children
                    })
                    .to_string(),
                ),
                data: None,
            }],
        },),
    ))
}

fn handle_start_child(
    state: &mut State,
    manifest_name: &str,
) -> Result<(Option<Vec<u8>>, (WebsocketResponse,)), String> {
    match state.start_child(manifest_name) {
        Ok(_actor_id) => {
            // Get the updated running children
            let running_children = state.list_running_children();

            Ok((
                Some(serde_json::to_vec(state).unwrap()),
                (WebsocketResponse {
                    messages: vec![
                        WebsocketMessage {
                            ty: MessageType::Text,
                            text: Some(
                                json!({
                                    "type": "children_update",
                                    "running_children": running_children
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
            log(&format!("Failed to start child: {}", e));
            Ok((
                Some(serde_json::to_vec(state).unwrap()),
                (WebsocketResponse {
                    messages: vec![WebsocketMessage {
                        ty: MessageType::Text,
                        text: Some(
                            json!({
                                "type": "error",
                                "message": format!("Failed to start child: {}", e)
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

fn handle_stop_child(
    state: &mut State,
    actor_id: &str,
) -> Result<(Option<Vec<u8>>, (WebsocketResponse,)), String> {
    match state.stop_child(actor_id) {
        Ok(_) => {
            // Get the updated running children
            let running_children = state.list_running_children();

            Ok((
                Some(serde_json::to_vec(state).unwrap()),
                (WebsocketResponse {
                    messages: vec![WebsocketMessage {
                        ty: MessageType::Text,
                        text: Some(
                            json!({
                                "type": "children_update",
                                "running_children": running_children
                            })
                            .to_string(),
                        ),
                        data: None,
                    }],
                },),
            ))
        }
        Err(e) => {
            log(&format!("Failed to stop child: {}", e));
            Ok((
                Some(serde_json::to_vec(state).unwrap()),
                (WebsocketResponse {
                    messages: vec![WebsocketMessage {
                        ty: MessageType::Text,
                        text: Some(
                            json!({
                                "type": "error",
                                "message": format!("Failed to stop child: {}", e)
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
) -> Result<(Option<Vec<u8>>, (WebsocketResponse,)), String> {
    match state.generate_llm_response() {
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

fn handle_get_pending_child_messages(
    state: &State,
) -> Result<(Option<Vec<u8>>, (WebsocketResponse,)), String> {
    // The notification is already done by state.notify_pending_child_messages_update()
    let _ = state.notify_pending_child_messages_update();

    Ok((
        Some(serde_json::to_vec(state).unwrap()),
        (WebsocketResponse {
            messages: vec![], // Empty messages as we've already sent the notification
        },),
    ))
}

fn handle_toggle_pending_child_message(
    state: &mut State,
    message_id: &str,
    selected: bool,
) -> Result<(Option<Vec<u8>>, (WebsocketResponse,)), String> {
    match state.toggle_pending_child_message(message_id, selected) {
        Ok(_) => {
            // Return empty messages as the notification is already sent
            Ok((
                Some(serde_json::to_vec(state).unwrap()),
                (WebsocketResponse { messages: vec![] },),
            ))
        }
        Err(e) => {
            log(&format!("Failed to toggle pending child message: {}", e));
            Ok((
                Some(serde_json::to_vec(state).unwrap()),
                (WebsocketResponse {
                    messages: vec![WebsocketMessage {
                        ty: MessageType::Text,
                        text: Some(
                            json!({
                                "type": "error",
                                "message": format!("Failed to toggle pending child message: {}", e)
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

fn handle_remove_pending_child_message(
    state: &mut State,
    message_id: &str,
) -> Result<(Option<Vec<u8>>, (WebsocketResponse,)), String> {
    match state.remove_pending_child_message(message_id) {
        Ok(_) => {
            // Return empty messages as the notification is already sent
            Ok((
                Some(serde_json::to_vec(state).unwrap()),
                (WebsocketResponse { messages: vec![] },),
            ))
        }
        Err(e) => {
            log(&format!("Failed to remove pending child message: {}", e));
            Ok((
                Some(serde_json::to_vec(state).unwrap()),
                (WebsocketResponse {
                    messages: vec![WebsocketMessage {
                        ty: MessageType::Text,
                        text: Some(
                            json!({
                                "type": "error",
                                "message": format!("Failed to remove pending child message: {}", e)
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
