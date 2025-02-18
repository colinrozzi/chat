mod bindings;
mod children;
mod messages;
mod state;
mod handlers;
mod api;

use bindings::exports::ntwk::theater::actor::Guest as ActorGuest;
use bindings::exports::ntwk::theater::http_server::Guest as HttpGuest;
use bindings::exports::ntwk::theater::websocket_server::Guest as WebSocketGuest;
use bindings::exports::ntwk::theater::message_server_client::Guest as MessageServerClientGuest;
use bindings::ntwk::theater::filesystem::read_file;
use bindings::ntwk::theater::runtime::log;
use bindings::ntwk::theater::types::Json;
use serde::{Deserialize, Serialize};
use state::State;

#[derive(Serialize, Deserialize, Debug)]
struct InitData {
    store_id: String,
    head: Option<String>,
    websocket_port: u16,
}

struct Component;

impl ActorGuest for Component {
    fn init(data: Option<Vec<u8>>) -> Vec<u8> {
        log("Initializing chat actor");
        let data = data.unwrap();
        let init_data: InitData = serde_json::from_slice(&data).unwrap();

        // Read API key
        log("Reading API key");
        let api_key = match read_file("api-key.txt") {
            Ok(content) => String::from_utf8(content).unwrap().trim().to_string(),
            Err(_) => {
                log("Failed to read API key");
                return vec![];
            }
        };

        // Initialize state
        let initial_state = State::new(
            init_data.store_id,
            api_key,
            init_data.websocket_port,
            init_data.head,
        );

        log("State initialized");
        serde_json::to_vec(&initial_state).unwrap()
    }
}

impl HttpGuest for Component {
    fn handle_request(
        req: bindings::exports::ntwk::theater::http_server::HttpRequest,
        state: Json,
    ) -> (
        bindings::exports::ntwk::theater::http_server::HttpResponse,
        Json,
    ) {
        handlers::http::handle_request(req, state)
    }
}

impl WebSocketGuest for Component {
    fn handle_message(
        msg: bindings::exports::ntwk::theater::websocket_server::WebsocketMessage,
        state: Json,
    ) -> (
        Json,
        bindings::exports::ntwk::theater::websocket_server::WebsocketResponse,
    ) {
        handlers::websocket::handle_message(msg, state)
    }
}

impl MessageServerClientGuest for Component {
    fn handle_send(_msg: Vec<u8>, state: Json) -> Json {
        log("Handling message server client send");
        state
    }

    fn handle_request(_msg: Vec<u8>, state: Json) -> (Vec<u8>, Json) {
        log("Handling message server client request");
        (vec![], state)
    }
}

bindings::export!(Component with_types_in bindings);