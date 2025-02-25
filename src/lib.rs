mod api;
mod bindings;
mod children;
mod handlers;
mod messages;
mod state;

use bindings::exports::ntwk::theater::actor::Guest as ActorGuest;
use bindings::exports::ntwk::theater::http_server::Guest as HttpGuest;
use bindings::exports::ntwk::theater::message_server_client::Guest as MessageServerClientGuest;
use bindings::exports::ntwk::theater::websocket_server::Guest as WebSocketGuest;
use bindings::exports::ntwk::theater::websocket_server::{WebsocketMessage, WebsocketResponse};
use bindings::ntwk::theater::filesystem::read_file;
use bindings::ntwk::theater::http_client::{HttpRequest, HttpResponse};
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
    fn init(data: Option<Vec<u8>>, params: (String,)) -> Result<(Option<Vec<u8>>,), String> {
        log("Initializing chat actor");
        log(format!("{:?}", data).as_str());
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

        // Initialize state
        let initial_state = State::new(
            init_data.store_id,
            api_key,
            init_data.websocket_port,
            init_data.head,
        );

        log("State initialized");
        Ok((Some(serde_json::to_vec(&initial_state).unwrap()),))
    }
}

impl HttpGuest for Component {
    fn handle_request(
        state: Option<Vec<u8>>,
        params: (HttpRequest,),
    ) -> Result<(Option<Vec<u8>>, (HttpResponse,)), String> {
        handlers::http::handle_request(params.0, state.unwrap())
    }
}

impl WebSocketGuest for Component {
    fn handle_message(
        state: Option<Vec<u8>>,
        params: (WebsocketMessage,),
    ) -> Result<(Option<Vec<u8>>, (WebsocketResponse,)), String> {
        handlers::websocket::handle_message(params.0, state.unwrap())
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
        _params: (Vec<u8>,),
    ) -> Result<(Option<Vec<u8>>, (Vec<u8>,)), String> {
        log("Handling message server client request");
        Ok((state, (vec![],)))
    }
}

bindings::export!(Component with_types_in bindings);
