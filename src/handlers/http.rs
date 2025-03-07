use crate::bindings::ntwk::theater::filesystem::read_file;
use crate::bindings::ntwk::theater::http_client::{HttpRequest, HttpResponse};
use crate::bindings::ntwk::theater::types::Json;
use crate::state::State;
use serde_json::json;

pub fn handle_request(
    req: HttpRequest,
    state: Json,
) -> Result<(Option<Json>, (HttpResponse,)), String> {
    match (req.method.as_str(), req.uri.as_str()) {
        // Static file serving
        ("GET", "/") | ("GET", "/index.html") => {
            let content = read_file("index.html").unwrap();
            Ok((
                Some(state),
                (HttpResponse {
                    status: 200,
                    headers: vec![("Content-Type".to_string(), "text/html".to_string())],
                    body: Some(content),
                },),
            ))
        }
        ("GET", "/styles.css") => {
            let content = read_file("styles.css").unwrap();
            Ok((
                Some(state),
                (HttpResponse {
                    status: 200,
                    headers: vec![("Content-Type".to_string(), "text/css".to_string())],
                    body: Some(content),
                },),
            ))
        }
        ("GET", "/chat.js") => {
            let raw_content = read_file("chat.js").unwrap();
            let str_content = String::from_utf8(raw_content).unwrap();
            let _current_state: State = serde_json::from_slice(&state).unwrap();
            // Use the same port for WebSocket since we now use the http-framework
            let content = str_content.replace(
                "{{WEBSOCKET_PORT}}",
                "8084", // Use HTTP port for WebSocket with path /ws
            );
            Ok((
                Some(state),
                (HttpResponse {
                    status: 200,
                    headers: vec![(
                        "Content-Type".to_string(),
                        "application/javascript".to_string(),
                    )],
                    body: Some(content.into()),
                },),
            ))
        }

        // API endpoints for messages
        ("GET", "/api/messages") => {
            let mut current_state: State = serde_json::from_slice(&state).unwrap();
            let messages = current_state.get_chain();
            Ok((
                Some(state),
                (HttpResponse {
                    status: 200,
                    headers: vec![("Content-Type".to_string(), "application/json".to_string())],
                    body: Some(
                        serde_json::to_vec(&json!({
                            "status": "success",
                            "messages": messages,
                            "current_chat_id": current_state.current_chat_id
                        }))
                        .unwrap(),
                    ),
                },),
            ))
        }

        // API endpoints for chats
        ("GET", "/api/chats") => {
            let current_state: State = serde_json::from_slice(&state).unwrap();

            // Get list of chats
            let mut chats = Vec::new();
            if let Ok(chat_ids) = current_state.store.list_chat_ids() {
                for chat_id in chat_ids {
                    if let Ok(Some(chat_info)) = current_state.store.get_chat_info(&chat_id) {
                        chats.push(json!({
                            "id": chat_info.id,
                            "name": chat_info.name,
                            "icon": chat_info.icon,
                            "head": chat_info.head,
                            "children_count": chat_info.children.len()
                        }));
                    }
                }
            }

            Ok((
                Some(state),
                (HttpResponse {
                    status: 200,
                    headers: vec![("Content-Type".to_string(), "application/json".to_string())],
                    body: Some(
                        serde_json::to_vec(&json!({
                            "status": "success",
                            "chats": chats,
                            "current_chat_id": current_state.current_chat_id
                        }))
                        .unwrap(),
                    ),
                },),
            ))
        }

        // API endpoint to get a specific chat
        ("GET", uri) if uri.starts_with("/api/chats/") => {
            let current_state: State = serde_json::from_slice(&state).unwrap();

            // Extract the chat ID from the path parameter
            let path_parts: Vec<&str> = uri.split('/').collect();
            if path_parts.len() < 4 {
                return Ok((
                    Some(state),
                    (HttpResponse {
                        status: 400,
                        headers: vec![("Content-Type".to_string(), "application/json".to_string())],
                        body: Some(
                            serde_json::to_vec(&json!({
                                "status": "error",
                                "message": "Invalid chat ID format"
                            }))
                            .unwrap(),
                        ),
                    },),
                ));
            }

            let chat_id = path_parts[3]; // /api/chats/{id} -> id is at index 3

            match current_state.store.get_chat_info(chat_id) {
                Ok(Some(chat_info)) => {
                    let chat_json = json!({
                        "id": chat_info.id,
                        "name": chat_info.name,
                        "icon": chat_info.icon,
                        "head": chat_info.head,
                        "children_count": chat_info.children.len(),
                        "children": chat_info.children.iter().map(|(actor_id, child)| {
                            json!({
                                "actor_id": actor_id,
                                "manifest_name": child.manifest_name
                            })
                        }).collect::<Vec<_>>()
                    });

                    Ok((
                        Some(state),
                        (HttpResponse {
                            status: 200,
                            headers: vec![(
                                "Content-Type".to_string(),
                                "application/json".to_string(),
                            )],
                            body: Some(
                                serde_json::to_vec(&json!({
                                    "status": "success",
                                    "chat": chat_json
                                }))
                                .unwrap(),
                            ),
                        },),
                    ))
                }
                Ok(None) => Ok((
                    Some(state),
                    (HttpResponse {
                        status: 404,
                        headers: vec![("Content-Type".to_string(), "application/json".to_string())],
                        body: Some(
                            serde_json::to_vec(&json!({
                                "status": "error",
                                "message": format!("Chat not found: {}", chat_id)
                            }))
                            .unwrap(),
                        ),
                    },),
                )),
                Err(e) => Ok((
                    Some(state),
                    (HttpResponse {
                        status: 500,
                        headers: vec![("Content-Type".to_string(), "application/json".to_string())],
                        body: Some(
                            serde_json::to_vec(&json!({
                                "status": "error",
                                "message": format!("Failed to get chat: {}", e)
                            }))
                            .unwrap(),
                        ),
                    },),
                )),
            }
        }

        // API endpoint to create a new chat
        ("POST", "/api/chats") => {
            let mut current_state: State = serde_json::from_slice(&state).unwrap();

            // Parse the request body
            if let Some(body) = &req.body {
                if let Ok(data) = serde_json::from_slice::<serde_json::Value>(body) {
                    let name = data["name"].as_str().unwrap_or("New Chat").to_string();
                    let starting_head = data["starting_head"].as_str().map(String::from);

                    match current_state.create_chat(name, starting_head) {
                        Ok(chat_info) => Ok((
                            Some(serde_json::to_vec(&current_state).unwrap()),
                            (HttpResponse {
                                status: 201,
                                headers: vec![(
                                    "Content-Type".to_string(),
                                    "application/json".to_string(),
                                )],
                                body: Some(
                                    serde_json::to_vec(&json!({
                                        "status": "success",
                                        "chat": {
                                            "id": chat_info.id,
                                            "name": chat_info.name,
                                            "head": chat_info.head
                                        }
                                    }))
                                    .unwrap(),
                                ),
                            },),
                        )),
                        Err(e) => Ok((
                            Some(state),
                            (HttpResponse {
                                status: 500,
                                headers: vec![(
                                    "Content-Type".to_string(),
                                    "application/json".to_string(),
                                )],
                                body: Some(
                                    serde_json::to_vec(&json!({
                                        "status": "error",
                                        "message": format!("Failed to create chat: {}", e)
                                    }))
                                    .unwrap(),
                                ),
                            },),
                        )),
                    }
                } else {
                    Ok((
                        Some(state),
                        (HttpResponse {
                            status: 400,
                            headers: vec![(
                                "Content-Type".to_string(),
                                "application/json".to_string(),
                            )],
                            body: Some(
                                serde_json::to_vec(&json!({
                                    "status": "error",
                                    "message": "Invalid JSON data"
                                }))
                                .unwrap(),
                            ),
                        },),
                    ))
                }
            } else {
                // If no body provided, create with default name
                match current_state.create_chat("New Chat".to_string(), None) {
                    Ok(chat_info) => Ok((
                        Some(serde_json::to_vec(&current_state).unwrap()),
                        (HttpResponse {
                            status: 201,
                            headers: vec![(
                                "Content-Type".to_string(),
                                "application/json".to_string(),
                            )],
                            body: Some(
                                serde_json::to_vec(&json!({
                                    "status": "success",
                                    "chat": {
                                        "id": chat_info.id,
                                        "name": chat_info.name,
                                        "head": chat_info.head
                                    }
                                }))
                                .unwrap(),
                            ),
                        },),
                    )),
                    Err(e) => Ok((
                        Some(state),
                        (HttpResponse {
                            status: 500,
                            headers: vec![(
                                "Content-Type".to_string(),
                                "application/json".to_string(),
                            )],
                            body: Some(
                                serde_json::to_vec(&json!({
                                    "status": "error",
                                    "message": format!("Failed to create chat: {}", e)
                                }))
                                .unwrap(),
                            ),
                        },),
                    )),
                }
            }
        }

        // API endpoint to delete a chat
        ("DELETE", uri) if uri.starts_with("/api/chats/") => {
            let mut current_state: State = serde_json::from_slice(&state).unwrap();

            // Extract the chat ID from the path parameter
            let path_parts: Vec<&str> = uri.split('/').collect();
            if path_parts.len() < 4 {
                return Ok((
                    Some(state),
                    (HttpResponse {
                        status: 400,
                        headers: vec![("Content-Type".to_string(), "application/json".to_string())],
                        body: Some(
                            serde_json::to_vec(&json!({
                                "status": "error",
                                "message": "Invalid chat ID format"
                            }))
                            .unwrap(),
                        ),
                    },),
                ));
            }

            let chat_id = path_parts[3]; // /api/chats/{id} -> id is at index 3

            match current_state.delete_chat(chat_id) {
                Ok(_) => Ok((
                    Some(serde_json::to_vec(&current_state).unwrap()),
                    (HttpResponse {
                        status: 200,
                        headers: vec![("Content-Type".to_string(), "application/json".to_string())],
                        body: Some(
                            serde_json::to_vec(&json!({
                                "status": "success",
                                "message": "Chat deleted successfully",
                                "current_chat_id": current_state.current_chat_id
                            }))
                            .unwrap(),
                        ),
                    },),
                )),
                Err(e) => Ok((
                    Some(state),
                    (HttpResponse {
                        status: 500,
                        headers: vec![("Content-Type".to_string(), "application/json".to_string())],
                        body: Some(
                            serde_json::to_vec(&json!({
                                "status": "error",
                                "message": format!("Failed to delete chat: {}", e)
                            }))
                            .unwrap(),
                        ),
                    },),
                )),
            }
        }

        // API endpoint to update a chat
        ("PUT", uri) if uri.starts_with("/api/chats/") => {
            let current_state: State = serde_json::from_slice(&state).unwrap();

            // Extract the chat ID from the path parameter
            let path_parts: Vec<&str> = uri.split('/').collect();
            if path_parts.len() < 4 {
                return Ok((
                    Some(state),
                    (HttpResponse {
                        status: 400,
                        headers: vec![("Content-Type".to_string(), "application/json".to_string())],
                        body: Some(
                            serde_json::to_vec(&json!({
                                "status": "error",
                                "message": "Invalid chat ID format"
                            }))
                            .unwrap(),
                        ),
                    },),
                ));
            }

            let chat_id = path_parts[3]; // /api/chats/{id} -> id is at index 3

            // Parse the request body
            if let Some(body) = &req.body {
                if let Ok(data) = serde_json::from_slice::<serde_json::Value>(body) {
                    // Get the current chat info
                    match current_state.store.get_chat_info(chat_id) {
                        Ok(Some(mut chat_info)) => {
                            // Update the changeable fields
                            if let Some(name) = data["name"].as_str() {
                                chat_info.name = name.to_string();
                            }

                            if let Some(icon) = data["icon"].as_str() {
                                chat_info.icon = Some(icon.to_string());
                            }

                            // Save the updated chat info
                            match current_state.store.update_chat_info(&chat_info) {
                                Ok(_) => Ok((
                                    Some(state),
                                    (HttpResponse {
                                        status: 200,
                                        headers: vec![(
                                            "Content-Type".to_string(),
                                            "application/json".to_string(),
                                        )],
                                        body: Some(
                                            serde_json::to_vec(&json!({
                                                "status": "success",
                                                "chat": {
                                                    "id": chat_info.id,
                                                    "name": chat_info.name,
                                                    "icon": chat_info.icon,
                                                    "head": chat_info.head
                                                }
                                            }))
                                            .unwrap(),
                                        ),
                                    },),
                                )),
                                Err(e) => Ok((
                                    Some(state),
                                    (HttpResponse {
                                        status: 500,
                                        headers: vec![(
                                            "Content-Type".to_string(),
                                            "application/json".to_string(),
                                        )],
                                        body: Some(
                                            serde_json::to_vec(&json!({
                                                "status": "error",
                                                "message": format!("Failed to update chat: {}", e)
                                            }))
                                            .unwrap(),
                                        ),
                                    },),
                                )),
                            }
                        }
                        Ok(None) => Ok((
                            Some(state),
                            (HttpResponse {
                                status: 404,
                                headers: vec![(
                                    "Content-Type".to_string(),
                                    "application/json".to_string(),
                                )],
                                body: Some(
                                    serde_json::to_vec(&json!({
                                        "status": "error",
                                        "message": format!("Chat not found: {}", chat_id)
                                    }))
                                    .unwrap(),
                                ),
                            },),
                        )),
                        Err(e) => Ok((
                            Some(state),
                            (HttpResponse {
                                status: 500,
                                headers: vec![(
                                    "Content-Type".to_string(),
                                    "application/json".to_string(),
                                )],
                                body: Some(
                                    serde_json::to_vec(&json!({
                                        "status": "error",
                                        "message": format!("Failed to get chat: {}", e)
                                    }))
                                    .unwrap(),
                                ),
                            },),
                        )),
                    }
                } else {
                    Ok((
                        Some(state),
                        (HttpResponse {
                            status: 400,
                            headers: vec![(
                                "Content-Type".to_string(),
                                "application/json".to_string(),
                            )],
                            body: Some(
                                serde_json::to_vec(&json!({
                                    "status": "error",
                                    "message": "Invalid JSON data"
                                }))
                                .unwrap(),
                            ),
                        },),
                    ))
                }
            } else {
                Ok((
                    Some(state),
                    (HttpResponse {
                        status: 400,
                        headers: vec![("Content-Type".to_string(), "application/json".to_string())],
                        body: Some(
                            serde_json::to_vec(&json!({
                                "status": "error",
                                "message": "Request body required"
                            }))
                            .unwrap(),
                        ),
                    },),
                ))
            }
        }

        // Default 404 response
        _ => Ok((
            Some(state),
            (HttpResponse {
                status: 404,
                headers: vec![("Content-Type".to_string(), "application/json".to_string())],
                body: Some(
                    serde_json::to_vec(&json!({
                        "status": "error",
                        "message": "Endpoint not found"
                    }))
                    .unwrap(),
                ),
            },),
        )),
    }
}
