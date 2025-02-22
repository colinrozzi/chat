use crate::bindings::exports::ntwk::theater::http_server::{HttpRequest, HttpResponse};
use crate::bindings::ntwk::theater::filesystem::read_file;
use crate::bindings::ntwk::theater::types::Json;
use crate::state::State;
use serde_json::json;

pub fn handle_request(req: HttpRequest, state: Json) -> (HttpResponse, Json) {
    match (req.method.as_str(), req.uri.as_str()) {
        ("GET", "/") | ("GET", "/index.html") => {
            let content = read_file("index.html").unwrap();
            (
                HttpResponse {
                    status: 200,
                    headers: vec![("Content-Type".to_string(), "text/html".to_string())],
                    body: Some(content),
                },
                state,
            )
        }
        ("GET", "/styles.css") => {
            let content = read_file("styles.css").unwrap();
            (
                HttpResponse {
                    status: 200,
                    headers: vec![("Content-Type".to_string(), "text/css".to_string())],
                    body: Some(content),
                },
                state,
            )
        }
        ("GET", "/chat.js") => {
            let raw_content = read_file("chat.js").unwrap();
            let str_content = String::from_utf8(raw_content).unwrap();
            let current_state: State = serde_json::from_slice(&state).unwrap();
            let content = str_content.replace(
                "{{WEBSOCKET_PORT}}",
                &current_state.websocket_port.to_string(),
            );
            (
                HttpResponse {
                    status: 200,
                    headers: vec![
                        ("Content-Type".to_string(), "application/javascript".to_string()),
                    ],
                    body: Some(content.into()),
                },
                state,
            )
        }

        ("GET", "/api/messages") => {
            let mut current_state: State = serde_json::from_slice(&state).unwrap();
            let messages = current_state.get_chain();
            (
                HttpResponse {
                    status: 200,
                    headers: vec![
                        ("Content-Type".to_string(), "application/json".to_string()),
                    ],
                    body: Some(
                        serde_json::to_vec(&json!({
                            "status": "success",
                            "messages": messages
                        }))
                        .unwrap(),
                    ),
                },
                state,
            )   
        }
        // Default 404 response
        _ => (
            HttpResponse {
                status: 404,
                headers: vec![],
                body: Some(b"Not Found".to_vec()),
            },
            state,
        ),
    }
}
