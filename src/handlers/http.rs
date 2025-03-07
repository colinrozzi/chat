use crate::bindings::ntwk::theater::http_client::HttpRequest as ClientHttpRequest;
use crate::bindings::ntwk::theater::http_client::HttpResponse as ClientHttpResponse;
use crate::bindings::ntwk::theater::runtime::log;
use crate::state::State;
use serde_json::{json, Value};

pub fn handle_request(
    req: ClientHttpRequest,
    state_bytes: Vec<u8>,
) -> Result<(Option<Vec<u8>>, (ClientHttpResponse,)), String> {
    let mut state: State = serde_json::from_slice(&state_bytes).map_err(|e| e.to_string())?;
    let uri = req.uri.clone();
    log(&format!("Handling HTTP request: {}", uri));

    // Parse the URI to get the path and query
    let mut path_parts = uri.splitn(2, '?');
    let path = path_parts.next().unwrap_or("/");

    // If requesting root, redirect to index.html
    let path = if path == "/" { "/index.html" } else { path };

    match path {
        "/index.html" => serve_file("index.html", "text/html", &mut state),
        "/styles.css" => serve_file("styles.css", "text/css", &mut state),
        "/chat.js" => serve_chat_js(&mut state),
        "/api/messages" => handle_messages_api(&req, &mut state),
        "/api/chats" => handle_chats_api(&req, &mut state),
        uri if uri.starts_with("/api/chats/") => handle_chat_detail_api(&req, &mut state),
        _ => not_found(),
    }
}

fn serve_file(
    filename: &str,
    content_type: &str,
    state: &mut State,
) -> Result<(Option<Vec<u8>>, (ClientHttpResponse,)), String> {
    use crate::bindings::ntwk::theater::filesystem::read_file;

    match read_file(filename) {
        Ok(content) => {
            let response = ClientHttpResponse {
                status: 200,
                headers: vec![
                    ("Content-Type".to_string(), content_type.to_string()),
                    ("Cache-Control".to_string(), "no-cache".to_string()),
                ],
                body: Some(content),
            };
            Ok((Some(serde_json::to_vec(state).unwrap()), (response,)))
        }
        Err(e) => {
            log(&format!("Error reading file {}: {}", filename, e));
            not_found()
        }
    }
}

fn serve_chat_js(state: &mut State) -> Result<(Option<Vec<u8>>, (ClientHttpResponse,)), String> {
    use crate::bindings::ntwk::theater::filesystem::read_file;

    match read_file("chat.js") {
        Ok(content) => {
            // Convert content to string
            let content_str = String::from_utf8(content).unwrap_or_else(|_| "".to_string());

            // Replace the placeholder with the actual WebSocket port
            let modified_content = content_str.replace("{{WEBSOCKET_PORT}}", "8084");

            let response = ClientHttpResponse {
                status: 200,
                headers: vec![
                    (
                        "Content-Type".to_string(),
                        "application/javascript".to_string(),
                    ),
                    ("Cache-Control".to_string(), "no-cache".to_string()),
                ],
                body: Some(modified_content.into_bytes()),
            };
            Ok((Some(serde_json::to_vec(state).unwrap()), (response,)))
        }
        Err(e) => {
            log(&format!("Error reading chat.js file: {}", e));
            not_found()
        }
    }
}

fn handle_messages_api(
    req: &ClientHttpRequest,
    state: &mut State,
) -> Result<(Option<Vec<u8>>, (ClientHttpResponse,)), String> {
    match req.method.as_str() {
        "GET" => {
            // Get all messages in the chain
            let chain = state.get_chain();
            let chain_json = json!({
                "messages": chain,
                "head": state.head,
            });

            let response = ClientHttpResponse {
                status: 200,
                headers: vec![
                    ("Content-Type".to_string(), "application/json".to_string()),
                    ("Cache-Control".to_string(), "no-cache".to_string()),
                ],
                body: Some(serde_json::to_vec(&chain_json).unwrap()),
            };
            Ok((Some(serde_json::to_vec(state).unwrap()), (response,)))
        }
        _ => {
            let response = ClientHttpResponse {
                status: 405,
                headers: vec![
                    ("Content-Type".to_string(), "application/json".to_string()),
                    ("Allow".to_string(), "GET".to_string()),
                ],
                body: Some(
                    serde_json::to_vec(&json!({
                        "error": "Method not allowed"
                    }))
                    .unwrap(),
                ),
            };
            Ok((Some(serde_json::to_vec(state).unwrap()), (response,)))
        }
    }
}

fn handle_chats_api(
    req: &ClientHttpRequest,
    state: &mut State,
) -> Result<(Option<Vec<u8>>, (ClientHttpResponse,)), String> {
    match req.method.as_str() {
        "GET" => {
            // Get all chats
            let chat_ids = state.store.list_chat_ids().map_err(|e| e.to_string())?;
            let mut chats = Vec::new();

            for chat_id in chat_ids {
                if let Ok(Some(chat_info)) = state.store.get_chat_info(&chat_id) {
                    chats.push(json!({
                        "id": chat_info.id,
                        "name": chat_info.name,
                        "icon": chat_info.icon,
                    }));
                }
            }

            let response = ClientHttpResponse {
                status: 200,
                headers: vec![
                    ("Content-Type".to_string(), "application/json".to_string()),
                    ("Cache-Control".to_string(), "no-cache".to_string()),
                ],
                body: Some(
                    serde_json::to_vec(&json!({
                        "chats": chats,
                        "current_chat_id": state.current_chat_id,
                    }))
                    .unwrap(),
                ),
            };
            Ok((Some(serde_json::to_vec(state).unwrap()), (response,)))
        }
        "POST" => {
            // Create a new chat
            let body = match &req.body {
                Some(body) => String::from_utf8(body.clone())
                    .map_err(|_| "Invalid UTF-8 in request body".to_string())?,
                None => return Err("Missing request body".to_string()),
            };

            let data: Value = serde_json::from_str(&body)
                .map_err(|_| "Invalid JSON in request body".to_string())?;

            let name = data["name"]
                .as_str()
                .ok_or_else(|| "Missing 'name' field".to_string())?
                .to_string();

            let starting_head = data["starting_head"].as_str().map(|s| s.to_string());

            let chat_info = state
                .create_chat(name, starting_head)
                .map_err(|e| e.to_string())?;

            let response = ClientHttpResponse {
                status: 201,
                headers: vec![
                    ("Content-Type".to_string(), "application/json".to_string()),
                    ("Cache-Control".to_string(), "no-cache".to_string()),
                ],
                body: Some(
                    serde_json::to_vec(&json!({
                        "chat": {
                            "id": chat_info.id,
                            "name": chat_info.name,
                            "icon": chat_info.icon,
                        }
                    }))
                    .unwrap(),
                ),
            };
            Ok((Some(serde_json::to_vec(state).unwrap()), (response,)))
        }
        _ => {
            let response = ClientHttpResponse {
                status: 405,
                headers: vec![
                    ("Content-Type".to_string(), "application/json".to_string()),
                    ("Allow".to_string(), "GET, POST".to_string()),
                ],
                body: Some(
                    serde_json::to_vec(&json!({
                        "error": "Method not allowed"
                    }))
                    .unwrap(),
                ),
            };
            Ok((Some(serde_json::to_vec(state).unwrap()), (response,)))
        }
    }
}

fn handle_chat_detail_api(
    req: &ClientHttpRequest,
    state: &mut State,
) -> Result<(Option<Vec<u8>>, (ClientHttpResponse,)), String> {
    // Extract chat ID from the path
    let path_parts: Vec<&str> = req.uri.split('/').collect();
    let chat_id = path_parts
        .get(3)
        .ok_or_else(|| "Invalid chat ID".to_string())?
        .to_string();

    match req.method.as_str() {
        "GET" => {
            // Get chat details
            let chat_info = state
                .store
                .get_chat_info(&chat_id)
                .map_err(|e| e.to_string())?
                .ok_or_else(|| format!("Chat {} not found", chat_id))?;

            let response = ClientHttpResponse {
                status: 200,
                headers: vec![
                    ("Content-Type".to_string(), "application/json".to_string()),
                    ("Cache-Control".to_string(), "no-cache".to_string()),
                ],
                body: Some(
                    serde_json::to_vec(&json!({
                        "chat": {
                            "id": chat_info.id,
                            "name": chat_info.name,
                            "icon": chat_info.icon,
                        }
                    }))
                    .unwrap(),
                ),
            };
            Ok((Some(serde_json::to_vec(state).unwrap()), (response,)))
        }
        "PUT" => {
            // Update chat details
            let body = match &req.body {
                Some(body) => String::from_utf8(body.clone())
                    .map_err(|_| "Invalid UTF-8 in request body".to_string())?,
                None => return Err("Missing request body".to_string()),
            };

            let data: Value = serde_json::from_str(&body)
                .map_err(|_| "Invalid JSON in request body".to_string())?;

            // Get current chat info
            let mut chat_info = state
                .store
                .get_chat_info(&chat_id)
                .map_err(|e| e.to_string())?
                .ok_or_else(|| format!("Chat {} not found", chat_id))?;

            // Update fields
            if let Some(name) = data["name"].as_str() {
                chat_info.name = name.to_string();
            }
            if let Some(icon) = data["icon"].as_str() {
                chat_info.icon = Some(icon.to_string());
            }

            // Save updated chat info
            state
                .store
                .update_chat_info(&chat_info)
                .map_err(|e| e.to_string())?;

            let response = ClientHttpResponse {
                status: 200,
                headers: vec![
                    ("Content-Type".to_string(), "application/json".to_string()),
                    ("Cache-Control".to_string(), "no-cache".to_string()),
                ],
                body: Some(
                    serde_json::to_vec(&json!({
                        "chat": {
                            "id": chat_info.id,
                            "name": chat_info.name,
                            "icon": chat_info.icon,
                        }
                    }))
                    .unwrap(),
                ),
            };
            Ok((Some(serde_json::to_vec(state).unwrap()), (response,)))
        }
        "DELETE" => {
            // Delete chat
            state.delete_chat(&chat_id).map_err(|e| e.to_string())?;

            let response = ClientHttpResponse {
                status: 200,
                headers: vec![
                    ("Content-Type".to_string(), "application/json".to_string()),
                    ("Cache-Control".to_string(), "no-cache".to_string()),
                ],
                body: Some(
                    serde_json::to_vec(&json!({
                        "success": true,
                        "chat_id": chat_id
                    }))
                    .unwrap(),
                ),
            };
            Ok((Some(serde_json::to_vec(state).unwrap()), (response,)))
        }
        _ => {
            let response = ClientHttpResponse {
                status: 405,
                headers: vec![
                    ("Content-Type".to_string(), "application/json".to_string()),
                    ("Allow".to_string(), "GET, PUT, DELETE".to_string()),
                ],
                body: Some(
                    serde_json::to_vec(&json!({
                        "error": "Method not allowed"
                    }))
                    .unwrap(),
                ),
            };
            Ok((Some(serde_json::to_vec(state).unwrap()), (response,)))
        }
    }
}

fn not_found() -> Result<(Option<Vec<u8>>, (ClientHttpResponse,)), String> {
    let response = ClientHttpResponse {
        status: 404,
        headers: vec![("Content-Type".to_string(), "text/plain".to_string())],
        body: Some("Not Found".as_bytes().to_vec()),
    };
    Ok((None, (response,)))
}
