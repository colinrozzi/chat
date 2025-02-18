mod bindings;
mod children;
use children::scan_available_children;

use bindings::exports::ntwk::theater::actor::Guest as ActorGuest;
use bindings::exports::ntwk::theater::http_server::Guest as HttpGuest;
use bindings::exports::ntwk::theater::http_server::{
    HttpRequest as ServerHttpRequest, HttpResponse,
};
use bindings::exports::ntwk::theater::message_server_client::Guest as MessageServerClientGuest;
use bindings::exports::ntwk::theater::websocket_server::Guest as WebSocketGuest;
use bindings::exports::ntwk::theater::websocket_server::{
    MessageType, WebsocketMessage, WebsocketResponse,
};
use bindings::ntwk::theater::filesystem::read_file;
use bindings::ntwk::theater::http_client::{send_http, HttpRequest};
use bindings::ntwk::theater::message_server_host::request;
use bindings::ntwk::theater::runtime::log;
use bindings::ntwk::theater::runtime::spawn;
use bindings::ntwk::theater::types::Json;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;

// Message struct changes - making id optional
#[derive(Serialize, Deserialize, Debug, Clone)]
struct Message {
    role: String,
    content: String,
    parent: Option<String>,
    id: Option<String>, // Now optional
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct AnthropicMessage {
    role: String,
    content: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Chat {
    head: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type")]
enum StoredMessage {
    Message(Message),
    Rollup(RollupMessage),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct RollupMessage {
    original_message_id: String,
    child_responses: Vec<ChildResponse>,
    parent: Option<String>,
    id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct ChildActor {
    actor_id: String,
    manifest_name: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct State {
    chat: Chat,
    api_key: String,
    connected_clients: HashMap<String, bool>,
    store_id: String,
    websocket_port: u16,
    children: HashMap<String, ChildActor>,
    actor_messages: HashMap<String, Vec<u8>>,
}

// Import the Request/Action types - we'll need to define these since we can't import from store actor
#[derive(Serialize, Deserialize, Debug)]
struct Request {
    _type: String,
    data: Action,
}

#[derive(Serialize, Deserialize, Debug)]
enum Action {
    Get(String),
    Put(Vec<u8>),
    All(()),
}

// Add new message type for child responses
#[derive(Serialize, Deserialize, Debug, Clone)]
struct ChildResponse {
    child_id: String,
    message_id: String,
}

impl Message {
    fn new(role: String, content: String, parent: Option<String>) -> Self {
        Self {
            role,
            content,
            parent,
            id: None, // No ID until stored
        }
    }
}

impl StoredMessage {
    fn parent(&self) -> Option<String> {
        match self {
            StoredMessage::Message(m) => m.parent.clone(),
            StoredMessage::Rollup(r) => r.parent.clone(),
        }
    }
}

impl State {
    fn start_child(&mut self, manifest_name: &str) -> Result<String, Box<dyn std::error::Error>> {
        // Convert our relative path to full path for the runtime
        let manifest_path = format!(
            "/Users/colinrozzi/work/actors/chat/assets/children/{}.toml",
            manifest_name
        );

        // Spawn the child actor
        let actor_id = spawn(&manifest_path);

        // Add to our children map
        self.children.insert(
            actor_id.clone(),
            ChildActor {
                actor_id: actor_id.clone(),
                manifest_name: manifest_name.to_string(),
            },
        );

        match request(
            &actor_id,
            &serde_json::to_vec(&json!({
                "msg_type": "introduction",
                "data": {
                "store_id": self.store_id.clone()
            }
            }))?,
        ) {
            Ok(response) => {
                log(&format!(
                    "Child {} response: {:?}",
                    actor_id,
                    String::from_utf8(response.clone()).unwrap()
                ));

                self.actor_messages.insert(actor_id.clone(), response);
            }
            Err(e) => {
                log(&format!("Error starting child: {}", e));
            }
        }

        Ok(actor_id)
    }

    fn notify_children(
        &mut self,
        head_id: &str,
    ) -> Result<Vec<ChildResponse>, Box<dyn std::error::Error>> {
        let mut responses = Vec::new();

        for (actor_id, _child) in &self.children {
            // Notify each child of the new head
            if let Ok(response_bytes) = request(
                actor_id,
                &serde_json::to_vec(&json!({
                    "msg_type": "head-update",
                    "data": {
                        "head_id": head_id
                    }
                }))?,
            ) {
                if let Ok(response) = serde_json::from_slice::<Value>(&response_bytes) {
                    if response["status"] == "ok" {
                        if let Some(message_id) = response["message_id"].as_str() {
                            responses.push(ChildResponse {
                                child_id: actor_id.clone(),
                                message_id: message_id.to_string(),
                            });
                        }
                    }
                }
            }
        }

        Ok(responses)
    }

    fn handle_send_message(
        &mut self,
        content: &str,
    ) -> Result<Vec<StoredMessage>, Box<dyn std::error::Error>> {
        // Process user message and get rollup ID
        let user_rollup_id = self.process_message(content, "user", self.chat.head.clone())?;

        // Generate AI response using message history
        let messages = self.get_message_history()?;
        log(&format!("Messages: {:?}", messages));
        let ai_response = self.generate_response(messages)?;

        // Process assistant message with user rollup as parent
        let assistant_rollup_id =
            self.process_message(&ai_response, "assistant", Some(user_rollup_id.clone()))?;

        // Return all new messages
        let mut new_messages = Vec::new();

        // Load user message chain
        if let Ok(user_msg) = self.load_message(&user_rollup_id) {
            new_messages.push(user_msg);
        }

        // Get and add user's child responses
        if let Ok(user_children) = self.get_child_responses(&user_rollup_id) {
            new_messages.extend(user_children);
        }

        // Load assistant message chain
        if let Ok(assistant_msg) = self.load_message(&assistant_rollup_id) {
            new_messages.push(assistant_msg);
        }

        // Get and add assistant's child responses
        if let Ok(assistant_children) = self.get_child_responses(&assistant_rollup_id) {
            new_messages.extend(assistant_children);
        }

        Ok(new_messages)
    }
    // Update WebSocket handler for get_messages
    fn handle_get_messages(&self) -> Result<WebsocketResponse, Box<dyn std::error::Error>> {
        if let Ok(messages) = self.get_full_message_tree() {
            Ok(WebsocketResponse {
                messages: vec![WebsocketMessage {
                    ty: MessageType::Text,
                    text: Some(
                        serde_json::json!({
                            "type": "message_update",
                            "messages": messages
                        })
                        .to_string(),
                    ),
                    data: None,
                }],
            })
        } else {
            Err("Failed to load messages".into())
        }
    }

    fn get_full_message_tree(&self) -> Result<Vec<StoredMessage>, Box<dyn std::error::Error>> {
        let mut messages = Vec::new();
        let mut current_id = self.chat.head.clone();

        while let Some(id) = current_id {
            let stored_msg = self.load_message(&id)?;
            current_id = stored_msg.parent();
            messages.push(stored_msg);
        }

        // Return messages in reverse order (oldest first)
        messages.reverse();
        Ok(messages)
    }

    fn get_child_responses(
        &self,
        message_id: &str,
    ) -> Result<Vec<StoredMessage>, Box<dyn std::error::Error>> {
        if let StoredMessage::Rollup(rollup) = self.load_message(message_id)? {
            let mut responses = Vec::new();
            for child_response in rollup.child_responses {
                if let Ok(msg) = self.load_message(&child_response.message_id) {
                    responses.push(msg);
                }
            }
            Ok(responses)
        } else {
            Ok(vec![])
        }
    }

    fn process_message(
        &mut self,
        content: &str,
        role: &str,
        parent_id: Option<String>,
    ) -> Result<String, Box<dyn std::error::Error>> {
        // Create and save initial message
        let msg = Message::new(role.to_string(), content.to_string(), parent_id);

        // Save message and get its ID
        let msg_id = self.save_message(&StoredMessage::Message(msg))?;

        // Notify children and collect their responses
        let child_responses = self.notify_children(&msg_id)?;

        // Create and save rollup message if there are any child responses
        if !child_responses.is_empty() {
            let rollup = RollupMessage {
                original_message_id: msg_id.clone(),
                child_responses,
                parent: Some(msg_id.clone()),
                id: None,
            };
            let rollup_id = self.save_message(&StoredMessage::Rollup(rollup))?;
            // Update head to rollup
            self.update_head(rollup_id.clone())?;
            Ok(rollup_id)
        } else {
            // If no child responses, just return the message ID
            self.update_head(msg_id.clone())?;
            Ok(msg_id)
        }
    }

    fn save_message(&mut self, msg: &StoredMessage) -> Result<String, Box<dyn std::error::Error>> {
        let req = Request {
            _type: "request".to_string(),
            data: Action::Put(serde_json::to_vec(&msg)?),
        };

        let request_bytes = serde_json::to_vec(&req)?;
        let response_bytes = request(&self.store_id, &request_bytes)?;

        let response: Value = serde_json::from_slice(&response_bytes)?;
        if response["status"].as_str() == Some("ok") {
            Ok(response["key"]
                .as_str()
                .map(|s| s.to_string())
                .ok_or("No key in response")?)
        } else {
            Err("Failed to save message".into())
        }
    }

    fn load_message(&self, id: &str) -> Result<StoredMessage, Box<dyn std::error::Error>> {
        let req = Request {
            _type: "request".to_string(),
            data: Action::Get(id.to_string()),
        };

        let request_bytes = serde_json::to_vec(&req)?;
        let response_bytes = request(&self.store_id, &request_bytes)?;

        let response: Value = serde_json::from_slice(&response_bytes)?;
        if response["status"].as_str() == Some("ok") {
            if let Some(value) = response.get("value") {
                let bytes = value
                    .as_array()
                    .ok_or("Expected byte array")?
                    .iter()
                    .map(|v| v.as_u64().unwrap_or(0) as u8)
                    .collect::<Vec<u8>>();
                log(&format!("Loaded message: {:?}", bytes));
                let msg: Result<StoredMessage, serde_json::Error> = serde_json::from_slice(&bytes);
                log(&format!("Loaded message: {:?}", msg));
                let mut msg = msg?;

                // Add ID to the correct variant
                match &mut msg {
                    StoredMessage::Message(m) => m.id = Some(id.to_string()),
                    StoredMessage::Rollup(r) => r.id = Some(id.to_string()),
                }

                return Ok(msg);
            }
        }
        Err("Failed to load message".into())
    }

    fn get_message_history(&self) -> Result<Vec<Message>, Box<dyn std::error::Error>> {
        log("Getting message history");
        let mut messages = Vec::new();
        let mut stored_messages = Vec::new();
        let mut current_id = self.chat.head.clone();

        // First, collect all messages in the chain
        while let Some(id) = current_id {
            let stored_msg = self.load_message(&id)?;
            current_id = stored_msg.parent();
            stored_messages.push(stored_msg);
        }

        log(&format!("Stored messages: {:?}", stored_messages));

        // Process stored messages in reverse (oldest first)
        for stored_msg in stored_messages.iter().rev() {
            match stored_msg {
                StoredMessage::Message(msg) => {
                    log(&format!("Message: {:?}", msg));
                    messages.push(msg.clone());
                }
                StoredMessage::Rollup(rollup) => {
                    log(&format!("Rollup: {:?}", rollup));
                    // For each child response, load it and append to the previous message
                    if let Some(last_message) = messages.last_mut() {
                        log(&format!("Last message: {:?}", last_message));
                        let mut child_content = String::new();
                        log(&format!(
                            "Rollup child responses: {:?}",
                            rollup.child_responses
                        ));
                        for child_response in &rollup.child_responses {
                            log(&format!("Child response: {:?}", child_response));
                            log(&format!(
                                "Loading child message: {:?}",
                                &child_response.message_id
                            ));
                            let child_msg = self.load_message(&child_response.message_id)?;
                            log(&format!("Child message: {:?}", child_msg));
                            if let Ok(child_msg) = self.load_message(&child_response.message_id) {
                                log(&format!("Child message: {:?}", child_msg));
                                match child_msg {
                                    StoredMessage::Message(msg) => {
                                        child_content.push_str(&format!(
                                            "\nActor {} response:\n{}",
                                            child_response.child_id, msg.content
                                        ));
                                    }
                                    _ => continue,
                                }
                            }
                        }

                        // If we have child responses, append them to the last message
                        if !child_content.is_empty() {
                            last_message.content = format!(
                                "{}\n\nActor Responses:{}",
                                last_message.content, child_content
                            );
                        }
                    } else {
                        log("No previous message to append child responses to");
                    }
                }
            }
        }

        Ok(messages)
    }

    fn update_head(&mut self, message_id: String) -> Result<(), Box<dyn std::error::Error>> {
        self.chat.head = Some(message_id);
        Ok(())
    }

    fn generate_response(
        &self,
        messages: Vec<Message>,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let anthropic_messages: Vec<AnthropicMessage> = messages
            .iter()
            .map(|msg| AnthropicMessage {
                role: msg.role.clone(),
                content: msg.content.clone(),
            })
            .collect();

        let request = HttpRequest {
            method: "POST".to_string(),
            uri: "https://api.anthropic.com/v1/messages".to_string(),
            headers: vec![
                ("Content-Type".to_string(), "application/json".to_string()),
                ("x-api-key".to_string(), self.api_key.clone()),
                ("anthropic-version".to_string(), "2023-06-01".to_string()),
            ],
            body: Some(
                serde_json::to_vec(&json!({
                    "model": "claude-3-5-sonnet-20241022",
                    "max_tokens": 1024,
                    "messages": anthropic_messages,
                }))
                .unwrap(),
            ),
        };

        let http_response = send_http(&request);

        if let Some(body) = http_response.body {
            if let Ok(response_data) = serde_json::from_slice::<Value>(&body) {
                if let Some(text) = response_data["content"][0]["text"].as_str() {
                    return Ok(text.to_string());
                }
            }
        }

        Err("Failed to generate response".into())
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct InitData {
    store_id: String,
    head: Option<String>,
    websocket_port: u16,
}

struct Component;

impl ActorGuest for Component {
    fn init(data: Option<Vec<u8>>) -> Vec<u8> {
        log("Initializing chat actor with children");
        let data = data.unwrap();
        log(&format!("Data: {:?}", data));

        let init_data: InitData = serde_json::from_slice(&data).unwrap();

        log(&format!("Store actor id: {}", init_data.store_id));
        log(&format!("Head: {:?}", init_data.head));
        log(&format!("Websocket port: {}", init_data.websocket_port));

        // Read API key
        log("Reading API key");
        let res = read_file("api-key.txt");
        if res.is_err() {
            log("Failed to read API key");
            return vec![];
        }
        let api_key = res.unwrap();
        log("API key read");
        let api_key = String::from_utf8(api_key).unwrap().trim().to_string();
        log("API key loaded");

        // Load or create chat
        let chat = Chat {
            head: init_data.head,
        };

        log("Chat loaded");

        let initial_state = State {
            chat,
            api_key,
            connected_clients: HashMap::new(),
            store_id: init_data.store_id,
            websocket_port: init_data.websocket_port,
            children: HashMap::new(),
            actor_messages: HashMap::new(),
        };

        log("State initialized");

        serde_json::to_vec(&initial_state).unwrap()
    }
}

impl HttpGuest for Component {
    fn handle_request(req: ServerHttpRequest, state: Json) -> (HttpResponse, Json) {
        log(&format!("Handling HTTP request for: {}", req.uri));

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
                let content = str_content.replace(
                    "{{WEBSOCKET_PORT}}",
                    &format!(
                        "{}",
                        serde_json::from_slice::<State>(&state)
                            .unwrap()
                            .websocket_port
                    ),
                );
                (
                    HttpResponse {
                        status: 200,
                        headers: vec![(
                            "Content-Type".to_string(),
                            "application/javascript".to_string(),
                        )],
                        body: Some(content.into()),
                    },
                    state,
                )
            }

            ("GET", "/api/messages") => {
                let current_state: State = serde_json::from_slice(&state).unwrap();
                // Create a new function to get the full message history including rollups
                match current_state.get_full_message_tree() {
                    Ok(messages) => (
                        HttpResponse {
                            status: 200,
                            headers: vec![(
                                "Content-Type".to_string(),
                                "application/json".to_string(),
                            )],
                            body: Some(
                                serde_json::to_vec(&json!({
                                    "status": "success",
                                    "messages": messages
                                }))
                                .unwrap(),
                            ),
                        },
                        state,
                    ),
                    Err(_) => (
                        HttpResponse {
                            status: 500,
                            headers: vec![],
                            body: Some(b"Failed to load messages".to_vec()),
                        },
                        state,
                    ),
                }
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
}

impl WebSocketGuest for Component {
    fn handle_message(msg: WebsocketMessage, state: Json) -> (Json, WebsocketResponse) {
        let mut current_state: State = serde_json::from_slice(&state).unwrap();

        match msg.ty {
            MessageType::Text => {
                if let Some(text) = msg.text {
                    if let Ok(command) = serde_json::from_str::<Value>(&text) {
                        match command["type"].as_str() {
                            Some("get_available_children") => {
                                // Scan for available child actors
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

                                return (
                                    serde_json::to_vec(&current_state).unwrap(),
                                    WebsocketResponse {
                                        messages: vec![WebsocketMessage {
                                            ty: MessageType::Text,
                                            text: Some(
                                                serde_json::json!({
                                                    "type": "children_update",
                                                    "available_children": available_children
                                                })
                                                .to_string(),
                                            ),
                                            data: None,
                                        }],
                                    },
                                );
                            }
                            Some("get_running_children") => {
                                let running_children: Vec<Value> = current_state
                                    .children
                                    .iter()
                                    .map(|(actor_id, child)| {
                                        json!({
                                            "actor_id": actor_id,
                                            "manifest_name": child.manifest_name
                                        })
                                    })
                                    .collect();

                                return (
                                    serde_json::to_vec(&current_state).unwrap(),
                                    WebsocketResponse {
                                        messages: vec![WebsocketMessage {
                                            ty: MessageType::Text,
                                            text: Some(
                                                serde_json::json!({
                                                    "type": "children_update",
                                                    "running_children": running_children
                                                })
                                                .to_string(),
                                            ),
                                            data: None,
                                        }],
                                    },
                                );
                            }
                            Some("start_child") => {
                                if let Some(manifest_name) = command["manifest_name"].as_str() {
                                    if let Ok(_actor_id) = current_state.start_child(manifest_name)
                                    {
                                        // Send updated running children list
                                        let running_children: Vec<Value> = current_state
                                            .children
                                            .iter()
                                            .map(|(actor_id, child)| {
                                                json!({
                                                    "actor_id": actor_id,
                                                    "manifest_name": child.manifest_name
                                                })
                                            })
                                            .collect();

                                        return (
                                            serde_json::to_vec(&current_state).unwrap(),
                                            WebsocketResponse {
                                                messages: vec![WebsocketMessage {
                                                    ty: MessageType::Text,
                                                    text: Some(
                                                        serde_json::json!({
                                                            "type": "children_update",
                                                            "running_children": running_children
                                                        })
                                                        .to_string(),
                                                    ),
                                                    data: None,
                                                }],
                                            },
                                        );
                                    }
                                }
                            }
                            Some("stop_child") => {
                                if let Some(actor_id) = command["actor_id"].as_str() {
                                    // Remove the child from our state
                                    current_state.children.remove(actor_id);

                                    // Send updated running children list
                                    let running_children: Vec<Value> = current_state
                                        .children
                                        .iter()
                                        .map(|(actor_id, child)| {
                                            json!({
                                                "actor_id": actor_id,
                                                "manifest_name": child.manifest_name
                                            })
                                        })
                                        .collect();

                                    return (
                                        serde_json::to_vec(&current_state).unwrap(),
                                        WebsocketResponse {
                                            messages: vec![WebsocketMessage {
                                                ty: MessageType::Text,
                                                text: Some(
                                                    serde_json::json!({
                                                        "type": "children_update",
                                                        "running_children": running_children
                                                    })
                                                    .to_string(),
                                                ),
                                                data: None,
                                            }],
                                        },
                                    );
                                }
                            }
                            Some("send_message") => {
                                if let Some(content) = command["content"].as_str() {
                                    if let Ok(new_messages) =
                                        current_state.handle_send_message(content)
                                    {
                                        return (
                                            serde_json::to_vec(&current_state).unwrap(),
                                            WebsocketResponse {
                                                messages: vec![WebsocketMessage {
                                                    ty: MessageType::Text,
                                                    text: Some(
                                                        serde_json::json!({
                                                            "type": "message_update",
                                                            "messages": new_messages
                                                        })
                                                        .to_string(),
                                                    ),
                                                    data: None,
                                                }],
                                            },
                                        );
                                    }
                                }
                            }
                            Some("get_messages") => {
                                if let Ok(response) = current_state.handle_get_messages() {
                                    return (serde_json::to_vec(&current_state).unwrap(), response);
                                }
                            }
                            _ => {
                                log("Unknown command type received");
                            }
                        }
                    }
                }
            }
            _ => {}
        }

        (
            serde_json::to_vec(&current_state).unwrap(),
            WebsocketResponse { messages: vec![] },
        )
    }
}

impl MessageServerClientGuest for Component {
    fn handle_send(_msg: Vec<u8>, state: Json) -> Json {
        log("Handling message server client send");
        //let msg_str = String::from_utf8(msg).unwrap();
        //log(&msg_str);
        state
    }

    fn handle_request(msg: Vec<u8>, state: Json) -> (Vec<u8>, Json) {
        log("Handling message server client request");
        let msg_str = String::from_utf8(msg).unwrap();
        log(&msg_str);
        (vec![], state)
    }
}

bindings::export!(Component with_types_in bindings);
