use crate::bindings::ntwk::theater::runtime::log;
// Embed static files directly into the binary
// HTML files
pub const INDEX_HTML: &str = include_str!("../../assets/index.html");

// CSS files
pub const STYLES_CSS: &str = include_str!("../../assets/styles.css");

// JavaScript files
pub const CHAT_JS: &str = include_str!("../../assets/chat.js");

// Function to get a resource by path
pub fn get_resource(path: &str) -> Option<(&'static str, &'static str)> {
    log(format!("Request for resource: {}", path).as_str());
    match path {
        "/" | "/index.html" => Some((INDEX_HTML, "text/html")),
        "/styles.css" => Some((STYLES_CSS, "text/css")),
        "/chat.js" => Some((CHAT_JS, "application/javascript")),
        _ => None,
    }
}
