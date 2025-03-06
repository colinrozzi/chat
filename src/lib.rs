mod api;
mod bindings;
mod children;
mod handlers;
mod messages;
mod state;

use bindings::exports::ntwk::theater::actor::Guest as ActorGuest;
use bindings::exports::ntwk::theater::http_handlers::Guest as HttpHandlersGuest;
use bindings::exports::ntwk::theater::message_server_client::Guest as MessageServerClientGuest;
use bindings::ntwk::theater::http_framework::{add_route, create_server, enable_websocket, register_handler, start_server, ServerConfig};
use bindings::ntwk::theater::http_types::{HttpRequest as FrameworkHttpRequest, HttpResponse as FrameworkHttpResponse, MiddlewareResult};
use bindings::ntwk::theater::websocket_types::{MessageType, WebsocketMessage};
use bindings::ntwk::theater::filesystem::read_file;
use bindings::ntwk::theater::http_client::{HttpRequest as ClientHttpRequest};
use bindings::ntwk::theater::runtime::log;
use bindings::ntwk::theater::supervisor::spawn;

use serde::{Deserialize, Serialize};
use state::State;

#[derive(Serialize, Deserialize, Debug)]
struct InitData {
    head: Option<String>,
    websocket_port: u16,
}

struct Component;

fn setup_http_server(_websocket_port: u16) -> Result<u64, String> {
    log("Setting up HTTP server");

    // Create server configuration for main HTTP server
    let config = ServerConfig {
        port: Some(8084), // Use the original HTTP port
        host: Some("0.0.0.0".to_string()),
        tls_config: None,
    };

    // Create a new HTTP server
    let server_id = create_server(&config)?;
    log(&format!("Created server with ID: {}", server_id));

    // Register handlers
    let api_handler_id = register_handler("handle_request")?;
    let ws_handler_id = register_handler("handle_websocket")?;

    log(&format!("Registered handlers - API: {}, WebSocket: {}", api_handler_id, ws_handler_id));

    // Add routes
    add_route(server_id, "/", "GET", api_handler_id)?;
    add_route(server_id, "/index.html", "GET", api_handler_id)?;
    add_route(server_id, "/styles.css", "GET", api_handler_id)?;
    add_route(server_id, "/chat.js", "GET", api_handler_id)?;
    add_route(server_id, "/api/messages", "GET", api_handler_id)?;
    
    // Enable WebSocket
    enable_websocket(
        server_id,
        "/ws",
        Some(ws_handler_id), // Connect handler
        ws_handler_id,       // Message handler
        Some(ws_handler_id), // Disconnect handler
    )?;

    // Start the server
    let port = start_server(server_id)?;
    log(&format!("Server started on port {}", port));

    Ok(server_id)
}

impl ActorGuest for Component {
    fn init(data: Option<Vec<u8>>, params: (String,)) -> Result<(Option<Vec<u8>>,), String> {
        log("Initializing chat actor");
        log(format!("{:?}", data).as_str());
        let id = params.0;
        let data = data.unwrap();
        log("Data unwrapped");
        let init_data: InitData = serde_json::from_slice(&data).unwrap();
        log("Init data deserialized");

        // Read API key
        log("Reading API key");
        let api_key = match read_file("api-key.txt") {
            Ok(content) => String::from_utf8(content).unwrap().trim().to_string(),
            Err(_) => {
                log("Failed to read API key");
                return Err("Failed to read API key".to_string());
            }
        };

        // spawn the store actor
        log("Spawning store actor");
        let store_id = spawn("/Users/colinrozzi/work/actors/store/actor.toml", None)?;

        // Set up the HTTP server
        log("Setting up HTTP server...");
        let server_id = setup_http_server(init_data.websocket_port)?;
        log("HTTP server set up successfully");

        // Initialize state
        let initial_state = State::new(
            id,
            store_id,
            api_key,
            server_id,
            init_data.websocket_port,
            init_data.head,
        );

        log("State initialized");
        Ok((Some(serde_json::to_vec(&initial_state).unwrap()),))
    }
}

impl HttpHandlersGuest for Component {
    fn handle_request(
        state: Option<Vec<u8>>,
        params: (u64, FrameworkHttpRequest),
    ) -> Result<(Option<Vec<u8>>, (FrameworkHttpResponse,)), String> {
        let (handler_id, request) = params;
        log(&format!("Handling HTTP request with handler ID: {}", handler_id));
        
        // Convert FrameworkHttpRequest to the old HttpRequest format
        let old_request = ClientHttpRequest {
            method: request.method.clone(),
            uri: request.uri.clone(),
            headers: request.headers.clone(),
            body: request.body.clone(),
        };
        
        // Use the existing HTTP handler
        let (new_state, old_response_tuple) = handlers::http::handle_request(old_request, state.unwrap())?;
        let old_response = old_response_tuple.0;
        
        // Convert the old HttpResponse to FrameworkHttpResponse
        let framework_response = FrameworkHttpResponse {
            status: old_response.status,
            headers: old_response.headers,
            body: old_response.body,
        };
        
        Ok((new_state, (framework_response,)))
    }
    
    fn handle_middleware(
        state: Option<Vec<u8>>,
        params: (u64, FrameworkHttpRequest),
    ) -> Result<(Option<Vec<u8>>, (MiddlewareResult,)), String> {
        let (handler_id, request) = params;
        log(&format!("Handling middleware with handler ID: {}", handler_id));
        
        // For now, just pass all requests through
        Ok((state, (MiddlewareResult {
            proceed: true,
            request,
        },)))
    }
    
    fn handle_websocket_connect(
        state: Option<Vec<u8>>,
        params: (u64, u64, String, Option<String>),
    ) -> Result<(Option<Vec<u8>>,), String> {
        let (handler_id, connection_id, path, _query) = params;
        log(&format!("WebSocket connected - Handler: {}, Connection: {}, Path: {}", 
            handler_id, connection_id, path));
        
        // Parse the current state
        let mut current_state: State = serde_json::from_slice(&state.unwrap()).unwrap();
        
        // Add the client to connected clients
        current_state.connected_clients.insert(connection_id.to_string(), true);
        log(&format!("Client {} connected, now have {} clients", 
            connection_id, current_state.connected_clients.len()));
        
        Ok((Some(serde_json::to_vec(&current_state).unwrap()),))
    }
    
    fn handle_websocket_message(
        state: Option<Vec<u8>>,
        params: (u64, u64, WebsocketMessage),
    ) -> Result<(Option<Vec<u8>>, (Vec<WebsocketMessage>,)), String> {
        let (handler_id, connection_id, message) = params;
        log(&format!("WebSocket message received - Handler: {}, Connection: {}", 
            handler_id, connection_id));
        
        match message.ty {
            MessageType::Text => {
                if let Some(text) = message.text {
                    log(&format!("Text message received: {}", text));
                    
                    // Create a WebsocketMessage from the old format
                    // Using a custom WebsocketMessage directly
                    let old_websocket_message = crate::handlers::websocket::WebsocketMessage {
                        data: message.data.clone(),
                        text: message.text.clone(),
                    };
                    
                    // Use the existing WebSocket handler
                    let (new_state, old_response_tuple) = handlers::websocket::handle_message(
                        old_websocket_message, 
                        state.unwrap()
                    )?;
                    let old_response = old_response_tuple.0;
                    
                    // Convert old responses to new format
                    let mut responses = Vec::new();
                    for old_msg in old_response.messages {
                        responses.push(WebsocketMessage {
                            ty: if old_msg.data.is_some() { MessageType::Binary } else { MessageType::Text },
                            data: old_msg.data,
                            text: old_msg.text,
                        });
                    }
                    
                    return Ok((new_state, (responses,)));
                }
            }
            _ => log(&format!("Non-text message received from connection {}", connection_id)),
        }
        
        // Default response
        Ok((state, (vec![],)))
    }
    
    fn handle_websocket_disconnect(
        state: Option<Vec<u8>>,
        params: (u64, u64),
    ) -> Result<(Option<Vec<u8>>,), String> {
        let (handler_id, connection_id) = params;
        log(&format!("WebSocket disconnected - Handler: {}, Connection: {}", 
            handler_id, connection_id));
        
        // Parse the current state
        let mut current_state: State = serde_json::from_slice(&state.unwrap()).unwrap();
        
        // Remove the client from connected clients
        current_state.connected_clients.remove(&connection_id.to_string());
        log(&format!("Client {} disconnected, now have {} clients", 
            connection_id, current_state.connected_clients.len()));
        
        Ok((Some(serde_json::to_vec(&current_state).unwrap()),))
    }
}

impl MessageServerClientGuest for Component {
    fn handle_send(
        state: Option<Vec<u8>>,
        params: (Vec<u8>,),
    ) -> Result<(Option<Vec<u8>>,), String> {
        log("Handling message server client send");
        let mut current_state: State = serde_json::from_slice(&state.unwrap()).unwrap();

        // Attempt to parse the incoming message
        match serde_json::from_slice::<serde_json::Value>(&params.0) {
            Ok(message) => {
                log(&format!(
                    "Received message: {}",
                    serde_json::to_string(&message).unwrap()
                ));

                // Check if this is a child message
                if let Some(msg_type) = message.get("msg_type").and_then(|v| v.as_str()) {
                    if msg_type == "child_message" {
                        if let Some(data) = message.get("data") {
                            // Try to parse as ChildMessage
                            match serde_json::from_value::<messages::ChildMessage>(data.clone()) {
                                Ok(child_message) => {
                                    log(&format!("Processing child message: {:?}", child_message));
                                    current_state.add_child_message(child_message);
                                }
                                Err(e) => {
                                    log(&format!("Failed to parse child message: {}", e));
                                }
                            }
                        }
                    }
                }
            }
            Err(e) => {
                log(&format!("Failed to parse message: {}", e));
            }
        }

        Ok((Some(serde_json::to_vec(&current_state).unwrap()),))
    }

    fn handle_request(
        state: Option<Vec<u8>>,
        _params: (Vec<u8>,),
    ) -> Result<(Option<Vec<u8>>, (Vec<u8>,)), String> {
        log("Handling message server client request");
        Ok((state, (vec![],)))
    }
}

bindings::export!(Component with_types_in bindings);
