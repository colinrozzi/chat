# Chat Actor System

A WebAssembly-based actor system that enables dynamic chat interactions with Claude AI. The system allows for real-time message processing, AI-powered responses via Claude API, and comprehensive chat management.

## Overview

The chat actor system serves as a standalone actor that can:
- Manage multiple conversation threads with message history
- Interact with the Claude API for AI responses
- Maintain real-time WebSocket connections for updates
- Serve a web interface for user interaction

## Core Features

- 🎭 **Actor System Architecture**
  - WebAssembly-based implementation
  - State persistence via content-addressable storage
  - Efficient message handling

- 💬 **Chat Functionality**
  - Multiple chat thread management
  - Threaded message history
  - Directed acyclic graph (DAG) message structure
  - Real-time updates via WebSocket
  - Web interface for interaction

- 🤖 **Claude API Integration**
  - Automated response generation using Claude 3.7 Sonnet
  - Context-aware conversations
  - Message history management
  - Token usage tracking

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
│   └── api-key.txt     # Claude API credentials
├── src/
│   ├── lib.rs          # Main actor implementation
│   ├── state.rs        # State management
│   ├── api/            # API clients (Claude)
│   ├── messages/       # Message structure and storage
│   ├── handlers/       # HTTP and WebSocket handlers
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
```

### Communication Channels

The system implements multiple communication channels:
- HTTP Server (Port 8084): Web interface and API endpoints
- WebSocket Server: Real-time updates and commands
- HTTP Client: Claude API interaction

## Setup & Development

### Prerequisites

- Nix with flakes enabled (or alternatively: Rust toolchain with `wasm32-unknown-unknown` target)
- Theater runtime system
- Claude API key

### Installation

1. Clone the repository
2. Set up the Claude API key:
   ```bash
   echo "your-api-key" > assets/api-key.txt
   ```

#### Using Nix Flake (Recommended)

1. Make sure you have Nix with flakes enabled
2. Enter the development environment:
   ```bash
   nix develop
   ```
   Or if you use direnv:
   ```bash
   direnv allow
   ```
3. Build the project:
   ```bash
   nix build
   ```
   This will create a `result` symlink with the built actor.

#### Manual Build

1. Install Rust with wasm32 target:
   ```bash
   rustup target add wasm32-unknown-unknown
   ```
2. Build the project:
   ```bash
   cargo build --target wasm32-unknown-unknown --release
   ```

### Running

#### Using Nix Build

1. Start the Theater runtime with the portable configuration:
   ```bash
   theater run ./result/actor.portable.toml
   ```
   Or use the local configuration:
   ```bash
   theater run actor.toml
   ```

#### Manual Run

1. Start the Theater runtime:
   ```bash
   theater run actor.toml
   ```

2. Access the web interface at `http://localhost:8084`

### Distribution

To distribute your actor:

1. Build with Nix:
   ```bash
   nix build
   ```

2. The resulting package can be shared and contains all dependencies
3. For NixOS users, they can simply add your flake to their inputs and use it directly

## API Endpoints

- `GET /api/messages`: Retrieve full message history
- `GET /api/chats`: List all chats
- `POST /api/chats`: Create a new chat
- `GET /api/chats/{id}`: Get chat info
- `PUT /api/chats/{id}`: Update chat info
- `DELETE /api/chats/{id}`: Delete a chat
- `WS /ws`: WebSocket endpoint for real-time updates

## WebSocket Commands

- `list_chats`: Get list of all available chats
- `create_chat`: Create a new chat
- `switch_chat`: Switch to a different chat thread
- `rename_chat`: Rename an existing chat
- `delete_chat`: Delete a chat
- `send_message`: Send a new user message
- `generate_llm_response`: Generate a Claude AI response
- `get_message`: Retrieve a specific message
- `get_head`: Get the current head message

## Current Status

### Implemented Features
- Multi-chat functionality with thread management
- Claude AI integration for automated responses
- Real-time WebSocket updates
- Message history with DAG structure
- Web interface
- Content-addressable storage

### In Progress
- Enhanced error handling
- Performance optimizations
- UI/UX improvements

## Contributing

Contributions are welcome! Areas for improvement include:
- Enhanced error handling
- UI/UX improvements
- Documentation expansion
- Testing infrastructure