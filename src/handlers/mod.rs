pub mod http;
pub mod websocket;

// Re-export the main handler functions
pub use http::handle_request as handle_http_request;
pub use websocket::handle_message as handle_websocket_message;