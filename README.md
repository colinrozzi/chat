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

- 🤖 **AI Model Integration**
  - Automated response generation using Claude 3.7 Sonnet
  - Gemini API integration
  - OpenRouter API integration for multiple model providers
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
- OpenRouter API key (for accessing additional models)

### Installation

1. Clone the repository
2. Set up the API keys in init.json:
   ```json
   {
     "anthropic_api_key": "your-claude-api-key",
     "gemini_api_key": "your-gemini-api-key",
     "openrouter_api_key": "your-openrouter-api-key"
   }
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

2. Install the Rust Component tools:
   ```bash
   cargo install cargo-component
   ```

3. Build the project as a WebAssembly component:
   ```bash
   cargo component build --release --target wasm32-unknown-unknown
   ```

   Note: This project is compiled to a WebAssembly component and run in a custom actor system, so standard Cargo build commands will not work correctly.

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
- `generate_llm_response`: Generate an AI response using specified model (Claude, Gemini, or any OpenRouter model)
  - Optional parameter: `model_id` to specify the model to use
  - Examples: 
    - Claude: `"claude-3-7-sonnet-20250219"`
    - Gemini: `"gemini-2.5-pro-exp-03-25"`
    - OpenRouter: `"anthropic/claude-3-opus-20240229", "openai/gpt-4-turbo", "mistral/mistral-large"`
  - Example usage:
    ```json
    {
      "type": "generate_llm_response",
      "model_id": "openai/gpt-4-turbo"
    }
    ```
- `list_models`: Get a list of all available models from all providers
- `get_message`: Retrieve a specific message
- `get_head`: Get the current head message

## Using Llama 4 Maverick Free with the Chat Actor

This chat actor now includes special support for Meta's Llama 4 Maverick free model via OpenRouter.

### About Llama 4 Maverick

Llama 4 Maverick is a high-capacity multimodal language model from Meta, built on a mixture-of-experts (MoE) architecture with 128 experts and 17 billion active parameters per forward pass (400B total parameters). Notable features include:

- Multimodal capabilities (text and image input)
- 1 million token context window
- Instruction-tuned for assistant-like behavior
- Support for 12 languages

### Using Llama 4 Maverick Free

To use the Llama 4 Maverick free model, set up OpenRouter as described below. The model will be used by default when using the OpenRouter client.

1. Create an account on [OpenRouter](https://openrouter.ai/) if you don't have one already
2. Generate an API key from the OpenRouter dashboard
3. Update the `init.json` file with your API key:
   ```json
   {
     "openrouter_api_key": "your-openrouter-api-key"
   }
   ```

### Sending Messages with Llama 4 Maverick

To explicitly use Llama 4 Maverick, send the WebSocket command:

```json
{
  "type": "generate_llm_response",
  "model_id": "meta-llama/llama-4-maverick:free"
}
```

Alternatively, the model will be used by default when you request an OpenRouter model without specifying a specific one.

## Current Status

### Implemented Features
- Multi-chat functionality with thread management
- Claude AI integration for automated responses
- Gemini API integration
- Llama 4 Maverick integration via OpenRouter
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