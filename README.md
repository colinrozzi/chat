# Chat Actor System

A WebAssembly-based actor system that enables dynamic chat interactions with child actor capabilities. The system allows for real-time message processing, AI-powered responses via Claude API, and extensible child actor management.

## Overview

The chat actor system serves as a parent actor that can:
- Manage a conversation thread with message history
- Interact with the Claude API for AI responses
- Spawn and manage child actors dynamically
- Process messages through child actors for enhanced functionality
- Maintain real-time WebSocket connections for updates
- Serve a web interface for user interaction

## Core Features

- 🎭 **Actor System Architecture**
  - Parent/child actor relationship model
  - Dynamic actor spawning and management
  - Inter-actor message passing

- 💬 **Chat Functionality**
  - Threaded message history
  - Message rollups with child actor responses
  - Real-time updates via WebSocket
  - Web interface for interaction

- 🤖 **Claude API Integration**
  - Automated response generation
  - Context-aware conversations
  - Message history management

- 👥 **Child Actor Framework**
  - Dynamic child actor discovery
  - Runtime actor management
  - Extensible actor interface
  - Message notification system

## Project Structure

```
chat/
├── Cargo.toml           # Rust project configuration
├── actor.toml           # Main actor manifest
├── assets/             
│   ├── init.json       # Actor initialization data
│   ├── index.html      # Web interface
│   ├── styles.css      # UI styling
│   ├── chat.js         # Frontend JavaScript
│   ├── api-key.txt     # Claude API credentials
│   └── children/       # Child actor manifests
├── src/
│   ├── lib.rs          # Main actor implementation
│   ├── children.rs     # Child actor management
│   └── bindings.rs     # Generated WIT bindings
└── wit/                # WebAssembly interface definitions
```

## Technical Details

### Message Structure

The system uses a sophisticated message structure:
```rust
struct Message {
    role: String,        // "user" or "assistant"
    content: String,     // Message content
    parent: Option<String>, // Parent message ID
    id: Option<String>   // Message identifier
}

enum StoredMessage {
    Message(Message),
    Rollup(RollupMessage)
}

struct RollupMessage {
    original_message_id: String,
    child_responses: Vec<ChildResponse>,
    parent: Option<String>,
    id: Option<String>
}
```

### Child Actor System

Child actors can:
- Process incoming messages
- Generate responses
- Contribute to message rollups
- Maintain independent state
- Be started/stopped dynamically

### Communication Channels

The system implements multiple communication channels:
- HTTP Server (Port 8084): Web interface and API endpoints
- WebSocket Server (Port 8085): Real-time updates and commands
- Message Server: Inter-actor communication
- HTTP Client: Claude API interaction

## Setup & Development

### Prerequisites

- Rust toolchain with `wasm32-unknown-unknown` target
- Theater runtime system
- Claude API key

### Installation

1. Clone the repository
2. Set up the Claude API key:
   ```bash
   echo "your-api-key" > assets/api-key.txt
   ```
3. Build the project:
   ```bash
   cargo build --release
   ```

### Running

1. Start the Theater runtime:
   ```bash
   theater run actor.toml
   ```
2. Access the web interface at `http://localhost:8084`

### Creating Child Actors

1. Create a new manifest in `assets/children/`:
   ```toml
   name = "Example Actor"
   description = "Handles specific message processing"
   version = "0.1.0"
   component_path = "path/to/component.wasm"
   ```
2. Implement the actor interface in your component
3. Build and deploy to the children directory

## API Endpoints

- `GET /api/messages`: Retrieve full message history
- `WS /`: WebSocket endpoint for real-time updates
- Commands:
  - `get_available_children`: List available child actors
  - `get_running_children`: List active child actors
  - `start_child`: Launch a child actor
  - `stop_child`: Terminate a child actor
  - `send_message`: Send a new chat message
  - `get_messages`: Retrieve message updates

## Current Status

### Implemented Features
- Basic chat functionality with AI responses
- Child actor management system
- Real-time WebSocket updates
- Message rollup system
- Web interface
- Dynamic child actor discovery

### In Progress
- Enhanced error handling
- Child actor state persistence
- Extended child actor capabilities
- Performance optimizations

## Contributing

Contributions are welcome! Areas for improvement include:
- Additional child actor implementations
- Enhanced error handling
- UI/UX improvements
- Documentation expansion
- Testing infrastructure

