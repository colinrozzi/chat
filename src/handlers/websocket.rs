use crate::bindings::exports::ntwk::theater::websocket_server::{
    MessageType, WebsocketMessage, WebsocketResponse,
};
use crate::bindings::ntwk::theater::runtime::log;
use crate::bindings::ntwk::theater::types::Json;
use crate::children::scan_available_children;
use crate::state::State;
use serde_json::{json, Value};

pub fn handle_message(
    msg: WebsocketMessage,
    state: Json,
) -> Result<(Option<Vec<u8>>, (WebsocketResponse,)), String> {
    log("Handling WebSocket message");

    log(&format!("Message: {:?}", msg));

    let mut current_state: State = serde_json::from_slice(&state).unwrap();

    match msg.ty {
        MessageType::Text => {
            if let Some(text) = msg.text {
                if let Ok(command) = serde_json::from_str::<Value>(&text) {
                    match command["type"].as_str() {
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
                        Some("send_message") => {
                            if let Some(content) = command["content"].as_str() {
                                handle_send_message(&mut current_state, content)
                            } else {
                                default_response(&current_state)
                            }
                        }
                        Some("get_message") => {
                            if let Some(message_id) = command["message_id"].as_str() {
                                let message = current_state.get_message(message_id).unwrap();
                                Ok((
                                    Some(serde_json::to_vec(&current_state).unwrap()),
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
                                ))
                            } else {
                                default_response(&current_state)
                            }
                        },
                        Some("child_message") => {
                            if let (Some(child_id), Some(text)) = (
                                command["child_id"].as_str(),
                                command["text"].as_str(),
                            ) {
                                let data = command["data"].clone();
                                
                                // Create a child message
                                let child_message = crate::messages::ChildMessage {
                                    child_id: child_id.to_string(),
                                    text: text.to_string(),
                                    data,
                                };
                                
                                // Add it to the chain
                                current_state.add_child_message(child_message);
                                
                                Ok((
                                    Some(serde_json::to_vec(&current_state).unwrap()),
                                    (WebsocketResponse {
                                        messages: vec![WebsocketMessage {
                                            ty: MessageType::Text,
                                            text: Some(
                                                json!({
                                                    "type": "messages_updated",
                                                    "head": current_state.head,
                                                })
                                                .to_string(),
                                            ),
                                            data: None,
                                        }],
                                    },),
                                ))
                            } else {
                                default_response(&current_state)
                            }
                        }
                        Some("get_head") => Ok((
                            Some(serde_json::to_vec(&current_state).unwrap()),
                            (WebsocketResponse {
                                messages: vec![WebsocketMessage {
                                    ty: MessageType::Text,
                                    text: Some(
                                        json!({
                                            "type": "head",
                                            "head": current_state.head
                                        })
                                        .to_string(),
                                    ),
                                    data: None,
                                }],
                            },),
                        )),

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

fn handle_get_available_children(
    state: &State,
) -> Result<(Option<Vec<u8>>, (WebsocketResponse,)), String> {
    let available_children = scan_available_children()
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
    let running_children: Vec<Value> = state
        .children
        .iter()
        .map(|(actor_id, child)| {
            json!({
                "actor_id": actor_id,
                "manifest_name": child.manifest_name
            })
        })
        .collect();

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
    if let Ok(_actor_id) = state.start_child(manifest_name) {
        let running_children: Vec<Value> = state
            .children
            .iter()
            .map(|(actor_id, child)| {
                json!({
                    "actor_id": actor_id,
                    "manifest_name": child.manifest_name
                })
            })
            .collect();
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
    } else {
        default_response(state)
    }
}

fn handle_stop_child(
    state: &mut State,
    actor_id: &str,
) -> Result<(Option<Vec<u8>>, (WebsocketResponse,)), String> {
    state.children.remove(actor_id);

    let running_children: Vec<Value> = state
        .children
        .iter()
        .map(|(actor_id, child)| {
            json!({
                "actor_id": actor_id,
                "manifest_name": child.manifest_name
            })
        })
        .collect();

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

fn handle_send_message(
    state: &mut State,
    content: &str,
) -> Result<(Option<Vec<u8>>, (WebsocketResponse,)), String> {
    state.add_user_message(content);
    Ok((
        Some(serde_json::to_vec(state).unwrap()),
        (WebsocketResponse {
            messages: vec![WebsocketMessage {
                ty: MessageType::Text,
                text: Some(
                    json!({
                        "type": "messages_updated",
                        "head": state.head,
                    })
                    .to_string(),
                ),
                data: None,
            }],
        },),
    ))
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
