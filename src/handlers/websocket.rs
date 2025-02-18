use crate::bindings::exports::ntwk::theater::websocket_server::{
    MessageType, WebsocketMessage, WebsocketResponse,
};
use crate::bindings::ntwk::theater::types::Json;
use crate::state::State;
use crate::messages::history::MessageHistory;
use crate::messages::store::MessageStore;
use crate::children::scan_available_children;
use serde_json::{json, Value};

pub fn handle_message(msg: WebsocketMessage, state: Json) -> (Json, WebsocketResponse) {
    let mut current_state: State = serde_json::from_slice(&state).unwrap();

    match msg.ty {
        MessageType::Text => {
            if let Some(text) = msg.text {
                if let Ok(command) = serde_json::from_str::<Value>(&text) {
                    match command["type"].as_str() {
                        Some("get_available_children") => {
                            handle_get_available_children(&current_state)
                        }
                        Some("get_running_children") => {
                            handle_get_running_children(&current_state)
                        }
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
                        Some("get_messages") => handle_get_messages(&current_state),
                        Some("get_message_content") => {
                            if let Some(message_id) = command["message_id"].as_str() {
                                handle_get_message_content(&current_state, message_id)
                            } else {
                                default_response(&current_state)
                            }
                        },
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

fn handle_get_available_children(state: &State) -> (Json, WebsocketResponse) {
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

    (
        serde_json::to_vec(state).unwrap(),
        WebsocketResponse {
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
        },
    )
}

fn handle_get_running_children(state: &State) -> (Json, WebsocketResponse) {
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

    (
        serde_json::to_vec(state).unwrap(),
        WebsocketResponse {
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
        },
    )
}

fn handle_start_child(state: &mut State, manifest_name: &str) -> (Json, WebsocketResponse) {
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

        (
            serde_json::to_vec(state).unwrap(),
            WebsocketResponse {
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
            },
        )
    } else {
        default_response(state)
    }
}

fn handle_stop_child(state: &mut State, actor_id: &str) -> (Json, WebsocketResponse) {
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

    (
        serde_json::to_vec(state).unwrap(),
        WebsocketResponse {
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
        },
    )
}

fn handle_send_message(state: &mut State, content: &str) -> (Json, WebsocketResponse) {
    if let Ok(new_messages) = state.handle_send_message(content) {
        (
            serde_json::to_vec(state).unwrap(),
            WebsocketResponse {
                messages: vec![WebsocketMessage {
                    ty: MessageType::Text,
                    text: Some(
                        json!({
                            "type": "message_update",
                            "messages": new_messages
                        })
                        .to_string(),
                    ),
                    data: None,
                }],
            },
        )
    } else {
        default_response(state)
    }
}

fn handle_get_message_content(state: &State, message_id: &str) -> (Json, WebsocketResponse) {
    let store = MessageStore::new(state.store_id.clone());
    let history = MessageHistory::new(store);
    
    if let Ok(responses) = history.get_child_responses(message_id) {
        (
            serde_json::to_vec(state).unwrap(),
            WebsocketResponse {
                messages: vec![WebsocketMessage {
                    ty: MessageType::Text,
                    text: Some(
                        json!({
                            "type": "message_content",
                            "message_id": message_id,
                            "content": responses
                        })
                        .to_string(),
                    ),
                    data: None,
                }],
            },
        )
    } else {
        default_response(state)
    }
}

fn default_response(state: &State) -> (Json, WebsocketResponse) {
    (
        serde_json::to_vec(state).unwrap(),
        WebsocketResponse { messages: vec![] },
    )
}