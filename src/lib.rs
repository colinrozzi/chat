//! # Chat WebAssembly Component
//!
//! This is a WebAssembly component for a chat application that integrates with Claude AI.
//!
//! ## Building
//!
//! This project must be built with cargo-component:
//! ```bash
//! cargo component build --release --target wasm32-unknown-unknown
//! ```
//! Standard cargo build commands will not work correctly.
//!

mod api;
mod bindings;
mod fs;
mod handlers;
mod mcp_server;
mod messages;
mod resources;
mod state;

use bindings::exports::ntwk::theater::actor::Guest as ActorGuest;
use bindings::exports::ntwk::theater::http_handlers::Guest as HttpHandlersGuest;
use bindings::exports::ntwk::theater::message_server_client::Guest as MessageServerClientGuest;
use bindings::ntwk::theater::http_client::HttpRequest as ClientHttpRequest;
use bindings::ntwk::theater::http_framework::{
    add_route, create_server, enable_websocket, register_handler, start_server, ServerConfig,
};
use bindings::ntwk::theater::http_types::{
    HttpRequest as FrameworkHttpRequest, HttpResponse as FrameworkHttpResponse, MiddlewareResult,
};
use bindings::ntwk::theater::message_server_host::send_on_channel;
use bindings::ntwk::theater::runtime::log;
use bindings::ntwk::theater::store;
use bindings::ntwk::theater::websocket_types::{MessageType, WebsocketMessage};
use mcp_server::McpServerConfig;
use messages::ModelInfo;
use state::State;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
struct InitData {
    store_id: Option<String>,
    openrouter_api_key: String,
    model_configs: Vec<ModelInfo>,
    assets_store_id: Option<String>,
    assets_runtime_content_fs: Option<String>,
    mcp_server_configs: Option<Vec<McpServerConfig>>,
}

struct Component;

fn setup_http_server() -> Result<u64, String> {
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

    log(&format!(
        "Registered handlers - API: {}, WebSocket: {}",
        api_handler_id, ws_handler_id
    ));

    // Add routes
    add_route(server_id, "/", "GET", api_handler_id)?;
    add_route(server_id, "/index.html", "GET", api_handler_id)?;
    add_route(server_id, "/styles.css", "GET", api_handler_id)?;
    add_route(server_id, "/chat.js", "GET", api_handler_id)?;

    // Message API routes
    add_route(server_id, "/api/messages", "GET", api_handler_id)?;

    // Chat API routes
    add_route(server_id, "/api/chats", "GET", api_handler_id)?;
    add_route(server_id, "/api/chats", "POST", api_handler_id)?;
    add_route(server_id, "/api/chats/{id}", "GET", api_handler_id)?;
    add_route(server_id, "/api/chats/{id}", "PUT", api_handler_id)?;
    add_route(server_id, "/api/chats/{id}", "DELETE", api_handler_id)?;

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
        log("Initializing chat actor with runtime store");
        log(format!("{:?}", data).as_str());
        let id = params.0;
        let data = data.unwrap();
        log("Data unwrapped");
        let init_data: InitData = serde_json::from_slice(&data).map_err(|e| {
            log(&format!("Failed to deserialize init data: {}", e));
            "Failed to deserialize init data".to_string()
        })?;
        log("Init data deserialized");
        log(&format!("Init data: {:?}", init_data));

        // Create a new runtime store
        log("Creating runtime store");
        let store_id: String;
        if let Some(init_store_id) = init_data.store_id {
            log(&format!("Using existing store with ID: {}", init_store_id));
            store_id = init_store_id;
        } else {
            log("Creating new store");
            store_id = store::new()?;
            log(&format!("Created runtime store with ID: {}", store_id));
        }

        // Set up the HTTP server
        log("Setting up HTTP server...");
        let server_id = setup_http_server()?;
        log("HTTP server set up successfully");

        // Log the OpenRouter API key (partially redacted for security)
        let openrouter_key_preview = if init_data.openrouter_api_key.len() > 8 {
            &init_data.openrouter_api_key[0..8]
        } else {
            "<empty>"
        };
        log(&format!(
            "OpenRouter API key starts with: {}...",
            openrouter_key_preview
        ));

        // Initialize state with all API keys
        let initial_state = State::new(
            id,
            store_id,
            init_data.openrouter_api_key, // Pass the OpenRouter API key
            server_id,
            init_data.mcp_server_configs,
            init_data.model_configs,
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
        log(&format!(
            "Handling HTTP request with handler ID: {}",
            handler_id
        ));

        // Convert FrameworkHttpRequest to the old HttpRequest format
        let old_request = ClientHttpRequest {
            method: request.method.clone(),
            uri: request.uri.clone(),
            headers: request.headers.clone(),
            body: request.body.clone(),
        };

        // Use the existing HTTP handler
        let (new_state, old_response_tuple) =
            handlers::http::handle_request(old_request, state.unwrap())?;
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
        log(&format!(
            "Handling middleware with handler ID: {}",
            handler_id
        ));

        // For now, just pass all requests through
        Ok((
            state,
            (MiddlewareResult {
                proceed: true,
                request,
            },),
        ))
    }

    fn handle_websocket_connect(
        state: Option<Vec<u8>>,
        params: (u64, u64, String, Option<String>),
    ) -> Result<(Option<Vec<u8>>,), String> {
        let (handler_id, connection_id, path, _query) = params;
        log(&format!(
            "WebSocket connected - Handler: {}, Connection: {}, Path: {}",
            handler_id, connection_id, path
        ));

        // Parse the current state
        let mut current_state: State = serde_json::from_slice(&state.unwrap()).unwrap();

        // Add the client to connected clients
        current_state
            .connected_clients
            .insert(connection_id.to_string(), true);
        log(&format!(
            "Client {} connected, now have {} clients",
            connection_id,
            current_state.connected_clients.len()
        ));

        // Send the current head to the new client
        {
            use bindings::ntwk::theater::http_framework::send_websocket_message;
            use bindings::ntwk::theater::websocket_types::{MessageType, WebsocketMessage};

            // Send head message
            let head_message = WebsocketMessage {
                ty: MessageType::Text,
                text: Some(
                    serde_json::to_string(&serde_json::json!({
                        "type": "head",
                        "head": current_state.head.clone(),
                        "current_chat_id": current_state.current_chat_id.clone()
                    }))
                    .unwrap(),
                ),
                data: None,
            };

            if let Err(e) =
                send_websocket_message(current_state.server_id, connection_id, &head_message)
            {
                log(&format!(
                    "Failed to send initial head update to new client: {}",
                    e
                ));
            }

            // Also send the list of available chats
            let mut chats = Vec::new();
            if let Ok(chat_ids) = current_state.store.list_chat_ids() {
                for chat_id in chat_ids {
                    if let Ok(Some(chat_info)) = current_state.store.get_chat_info(&chat_id) {
                        chats.push(serde_json::json!({
                            "id": chat_info.id,
                            "name": chat_info.name,
                            "icon": chat_info.icon
                        }));
                    }
                }
            }

            let chats_message = WebsocketMessage {
                ty: MessageType::Text,
                text: Some(
                    serde_json::to_string(&serde_json::json!({
                        "type": "chats_update",
                        "chats": chats,
                        "current_chat_id": current_state.current_chat_id.clone()
                    }))
                    .unwrap(),
                ),
                data: None,
            };

            if let Err(e) =
                send_websocket_message(current_state.server_id, connection_id, &chats_message)
            {
                log(&format!(
                    "Failed to send initial chats list to new client: {}",
                    e
                ));
            }
        }

        Ok((Some(serde_json::to_vec(&current_state).unwrap()),))
    }

    fn handle_websocket_message(
        state: Option<Vec<u8>>,
        params: (u64, u64, WebsocketMessage),
    ) -> Result<(Option<Vec<u8>>, (Vec<WebsocketMessage>,)), String> {
        let (handler_id, connection_id, message) = params;
        log(&format!(
            "WebSocket message received - Handler: {}, Connection: {}",
            handler_id, connection_id
        ));

        match message.ty {
            MessageType::Text => {
                if let Some(text) = message.text.clone() {
                    log(&format!("Text message received: {}", text));

                    // Use the existing WebSocket handler with the correct message format
                    let (new_state, old_response_tuple) = handlers::websocket::handle_message(
                        message, // Use the incoming WebsocketMessage directly
                        state.unwrap(),
                    )?;
                    let old_response = old_response_tuple.0;

                    // Convert old responses to new format
                    let mut responses = Vec::new();
                    for old_msg in old_response.messages {
                        responses.push(WebsocketMessage {
                            ty: if old_msg.data.is_some() {
                                MessageType::Binary
                            } else {
                                MessageType::Text
                            },
                            data: old_msg.data,
                            text: old_msg.text,
                        });
                    }

                    return Ok((new_state, (responses,)));
                }
            }
            _ => log(&format!(
                "Non-text message received from connection {}",
                connection_id
            )),
        }

        // Default response
        Ok((state, (vec![],)))
    }

    fn handle_websocket_disconnect(
        state: Option<Vec<u8>>,
        params: (u64, u64),
    ) -> Result<(Option<Vec<u8>>,), String> {
        let (handler_id, connection_id) = params;
        log(&format!(
            "WebSocket disconnected - Handler: {}, Connection: {}",
            handler_id, connection_id
        ));

        // Parse the current state
        let mut current_state: State = serde_json::from_slice(&state.unwrap()).unwrap();

        // Remove the client from connected clients
        current_state
            .connected_clients
            .remove(&connection_id.to_string());
        log(&format!(
            "Client {} disconnected, now have {} clients",
            connection_id,
            current_state.connected_clients.len()
        ));

        Ok((Some(serde_json::to_vec(&current_state).unwrap()),))
    }
}

impl MessageServerClientGuest for Component {
    fn handle_send(
        state: Option<Vec<u8>>,
        _params: (Vec<u8>,),
    ) -> Result<(Option<Vec<u8>>,), String> {
        log("Handling message server client send");

        Ok((state,))
    }

    fn handle_request(
        state: Option<Vec<u8>>,
        _params: (String, Vec<u8>),
    ) -> Result<(Option<Vec<u8>>, (Option<Vec<u8>>,)), String> {
        log("Handling message server client request");
        Ok((state, (Some(vec![]),)))
    }

    fn handle_channel_open(
        state: Option<bindings::exports::ntwk::theater::message_server_client::Json>,
        _params: (bindings::exports::ntwk::theater::message_server_client::Json,),
    ) -> Result<
        (
            Option<bindings::exports::ntwk::theater::message_server_client::Json>,
            (bindings::exports::ntwk::theater::message_server_client::ChannelAccept,),
        ),
        String,
    > {
        Ok((
            state,
            (
                bindings::exports::ntwk::theater::message_server_client::ChannelAccept {
                    accepted: true,
                    message: None,
                },
            ),
        ))
    }

    fn handle_channel_close(
        state: Option<bindings::exports::ntwk::theater::message_server_client::Json>,
        _params: (String,),
    ) -> Result<(Option<bindings::exports::ntwk::theater::message_server_client::Json>,), String>
    {
        Ok((state,))
    }

    fn handle_channel_message(
        state: Option<bindings::exports::ntwk::theater::message_server_client::Json>,
        params: (
            String,
            bindings::exports::ntwk::theater::message_server_client::Json,
        ),
    ) -> Result<(Option<bindings::exports::ntwk::theater::message_server_client::Json>,), String>
    {
        let (channel_id, message) = params;
        log(&format!(
            "Received message on channel {}: {:?}",
            channel_id, message
        ));
        let response = "Hello from the server!";
        let r = send_on_channel(&channel_id, &response.as_bytes().to_vec());
        log(&format!("runtime-content-fs: Sent response: {:?}", r));
        Ok((state,))
    }
}

bindings::export!(Component with_types_in bindings);
