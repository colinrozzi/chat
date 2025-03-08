# Child Actor Development Guide

This guide provides instructions and best practices for developing child actors that work with the Chat Actor System. Child actors enable you to extend the functionality of the chat system with custom capabilities while seamlessly integrating with the chat interface.

## Table of Contents

1. [Introduction](#introduction)
2. [Child Actor Protocol](#child-actor-protocol)
3. [Message Format](#message-format)
4. [HTML Content Support](#html-content-support)
5. [Examples](#examples)
6. [Best Practices](#best-practices)
7. [Deployment](#deployment)
8. [Troubleshooting](#troubleshooting)

## Introduction

Child actors in the Chat Actor System are WebAssembly components that can:

- Process messages from the chat
- Generate responses with text and/or HTML content
- Manage their own state
- Interact with external services
- Present rich, interactive content to users

Child actors receive notifications about new messages in the chat and can respond with their own messages that will be displayed in the conversation.

## Child Actor Protocol

Each child actor must implement the Theater Actor interface (`ntwk:theater/actor`) and handle the following message types:

1. **Introduction** - Sent when the actor is first started
2. **Head Update** - Sent when a new message is added to the chat
3. (Optional) Custom message types for specific actor-to-actor communication

### Actor Manifest

Each child actor needs a manifest file in TOML format with the following structure:

```toml
name = "actor-name"
version = "0.1.0"
description = "Description of what your actor does"
component_path = "/path/to/your/actor.wasm"

[interface]
implements = "ntwk:theater/actor"
requires = []

[[handlers]]
type = "runtime"
config = {}

# Additional handlers as needed:
# - filesystem
# - http-client
# - store
# etc.
```

## Message Format

When responding to messages, child actors should return a `ChildMessage` structure in the following format:

```json
{
  "child_id": "your-actor-id",
  "text": "Plain text description or message",
  "html": "<div>Optional HTML content to display</div>",
  "data": {
    "key1": "value1",
    "key2": "value2"
    // Any structured data that might be useful
  }
}
```

### Fields Explained

- **child_id**: The unique identifier of your actor (will be provided during initialization)
- **text**: Plain text content that will be shown if HTML is not available or used for Claude's context
- **html**: (Optional) HTML content that will be rendered in the chat interface
- **data**: (Optional) Any structured data that might be relevant or useful for debugging

## HTML Content Support

The new HTML content support allows child actors to provide rich, formatted content that will be directly injected into the chat UI. This enables visualizations, tables, formatted text, and more.

### HTML Guidelines

1. **Safety First**: All HTML content is sanitized before rendering to prevent XSS attacks. Dangerous elements and attributes are removed.

2. **Allowed HTML Tags**:
   - Structural: `div`, `span`, `p`, `br`, `hr`
   - Headings: `h1` through `h6`
   - Text formatting: `b`, `strong`, `i`, `em`, `u`, `small`, `pre`, `code`
   - Lists: `ul`, `ol`, `li`
   - Tables: `table`, `thead`, `tbody`, `tr`, `th`, `td`
   - Links: `a` (with safe `href` attributes)
   - Images: `img` (inline only, no external sources)

3. **CSS Considerations**:
   - Inline styles are allowed but will be sanitized
   - Use relative sizes (em, rem, %) rather than fixed pixel values
   - Be aware that your content will be displayed within the chat UI's constraints
   - **Use the chat's global CSS variables for consistent styling:**
     ```css
     /* Available CSS Variables */
     --bg-primary: #1A1C23;       /* Primary background */
     --bg-secondary: #242731;     /* Secondary background */
     --bg-tertiary: #2A2D39;      /* Tertiary background */
     --accent-primary: #6366F1;   /* Primary accent color */
     --accent-hover: #818CF8;     /* Hover accent color */
     --accent-muted: rgba(99, 102, 241, 0.1);  /* Muted accent */
     --text-primary: #F3F4F6;     /* Primary text color */
     --text-secondary: #9CA3AF;   /* Secondary text color */
     --text-muted: #6B7280;       /* Muted text color */
     --border-color: rgba(255, 255, 255, 0.1);  /* Border color */
     --radius-sm: 0.375rem;       /* Small border radius */
     --radius-md: 0.5rem;         /* Medium border radius */
     --radius-lg: 0.75rem;        /* Large border radius */
     ```

4. **Responsiveness**:
   - Your HTML content should be responsive and work within the constraints of the chat interface
   - Maximum width is limited to the width of the chat container
   - Use responsive design techniques for tables and other wide content

### Example HTML Content Using Global CSS Variables

```html
<div style="
    background: var(--bg-secondary); 
    border: 1px solid var(--border-color);
    border-radius: var(--radius-md);
    padding: 1rem;
    color: var(--text-primary);
">
  <h3 style="color: var(--accent-primary); margin-bottom: 0.75rem;">Analysis Results</h3>
  <table style="width: 100%; border-collapse: collapse;">
    <thead>
      <tr>
        <th style="
            background: var(--bg-tertiary); 
            color: var(--text-primary);
            padding: 0.5rem;
            text-align: left;
            border: 1px solid var(--border-color);
        ">Metric</th>
        <th style="
            background: var(--bg-tertiary); 
            color: var(--text-primary);
            padding: 0.5rem;
            text-align: left;
            border: 1px solid var(--border-color);
        ">Value</th>
        <th style="
            background: var(--bg-tertiary); 
            color: var(--text-primary);
            padding: 0.5rem;
            text-align: left;
            border: 1px solid var(--border-color);
        ">Status</th>
      </tr>
    </thead>
    <tbody>
      <tr>
        <td style="padding: 0.5rem; border: 1px solid var(--border-color);">Latency</td>
        <td style="padding: 0.5rem; border: 1px solid var(--border-color);">120ms</td>
        <td style="padding: 0.5rem; border: 1px solid var(--border-color); color: #10B981;">Good</td>
      </tr>
      <tr>
        <td style="padding: 0.5rem; border: 1px solid var(--border-color);">Error Rate</td>
        <td style="padding: 0.5rem; border: 1px solid var(--border-color);">2.5%</td>
        <td style="padding: 0.5rem; border: 1px solid var(--border-color); color: #F59E0B;">Warning</td>
      </tr>
    </tbody>
  </table>
  <div style="
    margin-top: 0.75rem; 
    padding: 0.5rem; 
    background: var(--accent-muted);
    color: var(--accent-primary);
    border-radius: var(--radius-sm);
    font-size: 0.875rem;
  ">
    System health is <strong>acceptable</strong> but requires monitoring.
  </div>
</div>
```

This example uses the chat's built-in CSS variables to match the overall aesthetic of the interface.

### Example HTML Content

```html
<div class="result-container">
  <h3>Analysis Results</h3>
  <table>
    <thead>
      <tr>
        <th>Metric</th>
        <th>Value</th>
        <th>Status</th>
      </tr>
    </thead>
    <tbody>
      <tr>
        <td>Latency</td>
        <td>120ms</td>
        <td style="color: green;">Good</td>
      </tr>
      <tr>
        <td>Error Rate</td>
        <td>2.5%</td>
        <td style="color: orange;">Warning</td>
      </tr>
    </tbody>
  </table>
  <p>System health is <strong>acceptable</strong> but requires monitoring.</p>
</div>
```

## Examples

### Basic Child Actor Template (Rust)

```rust
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Serialize, Deserialize, Debug)]
struct ChildMessage {
    child_id: String,
    text: String,
    html: Option<String>,
    data: Value,
}

#[derive(Serialize, Deserialize, Debug)]
struct HeadUpdateMessage {
    head: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
struct IntroductionMessage {
    child_id: String,
    store_id: String,
    chat_id: String,
    head: Option<String>,
}

// Handle messages from the parent actor
fn handle_message(message: Value) -> ChildMessage {
    if let Some(msg_type) = message.get("msg_type").and_then(|v| v.as_str()) {
        match msg_type {
            "introduction" => {
                if let Ok(intro_msg) = serde_json::from_value::<IntroductionMessage>(
                    message.get("data").unwrap_or(&json!({})).clone()
                ) {
                    // Store the actor ID from the introduction
                    let actor_id = intro_msg.child_id.clone();
                    
                    // Return a welcome message with both text and HTML
                    return ChildMessage {
                        child_id: actor_id,
                        text: "Hello! I'm your helper actor.",
                        html: Some("<div><h3>Hello!</h3><p>I'm your <strong>helper</strong> actor.</p></div>".to_string()),
                        data: json!({"status": "initialized"}),
                    };
                }
            },
            "head-update" => {
                if let Ok(head_msg) = serde_json::from_value::<HeadUpdateMessage>(
                    message.get("data").unwrap_or(&json!({})).clone()
                ) {
                    // Process the new message (you would need to fetch it from the store)
                    if let Some(head_id) = head_msg.head {
                        // You would fetch and process the message here
                        
                        // Example response with HTML content
                        return ChildMessage {
                            child_id: "your-actor-id", // Use stored actor ID here
                            text: "I processed your message",
                            html: Some("<div><p>I processed your message</p><table><tr><th>Result</th><td>Success</td></tr></table></div>".to_string()),
                            data: json!({"message_id": head_id}),
                        };
                    }
                }
            },
            _ => {
                // Handle other message types
            }
        }
    }
    
    // Default empty response
    ChildMessage {
        child_id: "your-actor-id".to_string(),
        text: "".to_string(),
        html: None,
        data: json!({}),
    }
}
```

### Rust Code Example Using CSS Variables

Here's how to generate HTML with CSS variables in Rust:

```rust
fn generate_styled_table(data: &[(String, String, String)]) -> String {
    let mut html = String::from(
        "<div style='\
        background: var(--bg-secondary); \
        border: 1px solid var(--border-color);\
        border-radius: var(--radius-md);\
        padding: 1rem;\
        color: var(--text-primary);\
        '>"
    );
    
    html.push_str("<h3 style='color: var(--accent-primary); margin-bottom: 0.75rem;'>Results</h3>");
    html.push_str("<table style='width: 100%; border-collapse: collapse;'>");
    
    // Table header
    html.push_str("<thead><tr>");
    let header_style = "background: var(--bg-tertiary); \
                       color: var(--text-primary); \
                       padding: 0.5rem; \
                       text-align: left; \
                       border: 1px solid var(--border-color);";
                       
    html.push_str(&format!("<th style='{}'>{}</th>", header_style, "Name"));
    html.push_str(&format!("<th style='{}'>{}</th>", header_style, "Value"));
    html.push_str(&format!("<th style='{}'>{}</th>", header_style, "Status"));
    html.push_str("</tr></thead><tbody>");
    
    // Table rows
    let cell_style = "padding: 0.5rem; border: 1px solid var(--border-color);";
    for (name, value, status) in data {
        html.push_str("<tr>");
        html.push_str(&format!("<td style='{}'>{}</td>", cell_style, name));
        html.push_str(&format!("<td style='{}'>{}</td>", cell_style, value));
        
        // Apply appropriate color based on status
        let status_style = match status.as_str() {
            "Good" => format!("{} color: #10B981;", cell_style),  // Success color
            "Warning" => format!("{} color: #F59E0B;", cell_style),  // Warning color
            "Error" => format!("{} color: #EF4444;", cell_style),  // Error color
            _ => cell_style.to_string(),
        };
        
        html.push_str(&format!("<td style='{}'>{}</td>", status_style, status));
        html.push_str("</tr>");
    }
    
    html.push_str("</tbody></table>");
    html.push_str("</div>");
    
    html
}

// Usage example:
let data = vec![
    ("CPU Usage".to_string(), "45%".to_string(), "Good".to_string()),
    ("Memory".to_string(), "86%".to_string(), "Warning".to_string()),
    ("Disk Space".to_string(), "95%".to_string(), "Error".to_string()),
];

let html_content = generate_styled_table(&data);

// Then include in your child message response:
ChildMessage {
    child_id: actor_id,
    text: "System status report: 1 issue detected",
    html: Some(html_content),
    data: json!({"timestamp": "2025-03-07T14:30:00Z"}),
}
```

This code generates a table that uses the chat's CSS variables for consistent styling.

### Data Visualization Example

```rust
fn generate_visualization(data: &[f64]) -> String {
    let max_value = data.iter().cloned().fold(0.0/0.0, f64::max);
    let chart_height = 150;
    let bar_width = 30;
    let spacing = 10;
    let total_width = (bar_width + spacing) * data.len() as i32;
    
    let mut html = format!(
        "<div style='font-family: sans-serif;'>\
         <h3>Data Visualization</h3>\
         <div style='display: flex; align-items: flex-end; height: {}px; width: {}px;'>",
        chart_height, total_width
    );
    
    for (i, &value) in data.iter().enumerate() {
        let height = (value / max_value * chart_height as f64) as i32;
        let percentage = (value / max_value * 100.0) as i32;
        
        html.push_str(&format!(
            "<div style='margin-right: {}px; text-align: center;'>\
             <div style='background-color: #6366F1; width: {}px; height: {}px;'></div>\
             <div style='margin-top: 5px;'>{:.1}</div>\
             </div>",
            spacing, bar_width, height, value
        ));
    }
    
    html.push_str("</div></div>");
    html
}

// Usage:
let data = vec![4.5, 8.2, 2.1, 6.7, 9.3];
let visualization_html = generate_visualization(&data);

// Then include in your response:
ChildMessage {
    child_id: actor_id,
    text: "Data visualization generated",
    html: Some(visualization_html),
    data: json!({"raw_data": data}),
}
```

## Best Practices

1. **Always provide both text and HTML**
   - Text is used for Claude's context and as a fallback
   - HTML provides rich visual representation

2. **Make responses concise and focused**
   - Each response should have a clear purpose
   - Avoid cluttering the chat with unnecessary updates

3. **Handle errors gracefully**
   - If your actor encounters an error, provide a helpful message
   - Include error details in the data field for debugging

4. **Use semantic HTML**
   - Follow accessibility best practices
   - Structure your content logically

5. **Optimize for the chat context**
   - Remember your content will appear in a conversation
   - Design for limited width and contextual relevance
   - **Use the chat's CSS variables for visual consistency:**
     - Leverage `var(--bg-primary)`, `var(--text-primary)`, etc.
     - This ensures your HTML adapts to theme changes and looks native
     - Avoid hardcoded colors that might clash with the interface

6. **Test thoroughly**
   - Test your HTML rendering in the actual chat interface
   - Verify behavior with different message types and states

7. **Include debugging information**
   - Use the data field to include useful debugging information
   - Log important state transitions and events

## Deployment

To deploy your child actor:

1. Build your WebAssembly component
   ```bash
   cargo build --target wasm32-unknown-unknown --release
   ```

2. Create a manifest file in the `/assets/children/` directory
   ```bash
   touch /path/to/chat/assets/children/your-actor.toml
   ```

3. Configure your manifest with the correct paths and capabilities

4. The actor will now appear in the "Available Actors" section of the chat interface

## Troubleshooting

### Common Issues

1. **Actor doesn't appear in the available list**
   - Check that your manifest file is correctly formatted
   - Ensure the component_path in your manifest is correct

2. **Actor starts but doesn't respond**
   - Verify that your actor correctly handles the "introduction" and "head-update" messages
   - Check for errors in the logs

3. **HTML content doesn't render properly**
   - Check for invalid HTML syntax
   - Remember that certain elements and attributes are sanitized

4. **Actor crashes when receiving messages**
   - Implement proper error handling in your message processing
   - Validate all inputs before processing

### Debugging Tips

1. Include detailed `data` fields in your responses for debugging
2. Log important events and state changes
3. Test your actor with simple messages before complex ones
4. Validate your HTML with standard tools before deployment

## Example: A Table Generator Actor

Here's a complete example of a simple actor that generates an HTML table from CSV data:

```rust
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::error::Error;

#[derive(Serialize, Deserialize, Debug)]
struct ChildMessage {
    child_id: String,
    text: String,
    html: Option<String>,
    data: Value,
}

// Message handler function
fn handle_csv_message(actor_id: &str, csv_content: &str) -> ChildMessage {
    // Parse CSV
    let mut html = String::from("<div class='csv-table'>");
    let mut text = String::from("CSV Data:\n");
    let mut rows = Vec::new();
    
    for (i, line) in csv_content.lines().enumerate() {
        let cells: Vec<&str> = line.split(',').collect();
        rows.push(cells);
    }
    
    if !rows.is_empty() {
        // Generate HTML table
        html.push_str("<table border='1' style='border-collapse: collapse; width: 100%;'>");
        
        // Header row
        if rows.len() > 0 {
            html.push_str("<thead><tr>");
            for cell in &rows[0] {
                html.push_str(&format!("<th style='padding: 8px; background-color: #f2f2f2;'>{}</th>", cell));
                text.push_str(&format!("{}\t", cell));
            }
            html.push_str("</tr></thead>");
            text.push('\n');
        }
        
        // Data rows
        html.push_str("<tbody>");
        for row in rows.iter().skip(1) {
            html.push_str("<tr>");
            for cell in row {
                html.push_str(&format!("<td style='padding: 8px;'>{}</td>", cell));
                text.push_str(&format!("{}\t", cell));
            }
            html.push_str("</tr>");
            text.push('\n');
        }
        html.push_str("</tbody></table>");
    } else {
        html.push_str("<p>No data found in CSV</p>");
        text.push_str("No data found in CSV");
    }
    
    html.push_str("</div>");
    
    ChildMessage {
        child_id: actor_id.to_string(),
        text,
        html: Some(html),
        data: json!({
            "row_count": rows.len().saturating_sub(1),
            "column_count": rows.get(0).map_or(0, |r| r.len())
        }),
    }
}

// Usage in your actor's message handler:
// if message contains CSV data, call:
// handle_csv_message(actor_id, csv_content)
```

This example demonstrates:
1. Handling CSV data and converting it to both plain text and HTML
2. Including useful metadata in the `data` field
3. Providing semantic HTML with appropriate styling
4. Ensuring that the content is accessible and displays well in the chat UI

---

For more information or help, please refer to the main project documentation or contact the development team.
