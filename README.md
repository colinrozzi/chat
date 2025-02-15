# Chat Actor System

A WebAssembly-based chat system that can manage child actors. Each child can process the chat's messages and take actions based on them, returning results that feed into future messages.

## Core Concept

The chat actor serves as a parent that can spawn and manage child actors. When a new message becomes the head of the chat, all children are notified and can:
- Process the message
- Take actions
- Return results that become part of the conversation

## Features

- 🌐 Web interface for chat interaction
- 👥 Child actor management panel
  - View available actors
  - Start/stop child actors
  - Monitor running actors
- 💬 Real-time message updates via WebSocket
- 🤖 Integration with Claude API
- 🔗 Linked message structure with parent/child relationships
- 📝 Results from child actors feed into conversation

## Project Structure

```
chat/
├── actor.toml          # Actor manifest
├── assets/            
│   ├── init.json      # Initialization data
│   ├── index.html     # Web interface
│   ├── styles.css     # CSS styles
│   ├── chat.js        # Frontend JavaScript
│   └── api-key.txt    # Claude API key
├── children/           # Child actor manifests
│   └── example-child.toml
└── src/
    ├── lib.rs         # Actor implementation
    └── bindings.rs    # Generated bindings
```

## Quick Start

1. Clone the repository
2. Create an `api-key.txt` file in the assets directory with your Claude API key
3. Build the actor:
```bash
cargo build --release
```
4. Run using the Theater runtime:
```bash
theater run actor.toml
```
5. Open `http://localhost:8084` in your browser

## How It Works

1. **Message Flow**
   - User sends a message
   - Message is saved as new head
   - All child actors are notified
   - Children process message and return results
   - Results are collected for next message
   - Claude generates response
   - Process repeats

2. **Child Management**
   - Child actors are defined by manifests in the `children/` directory
   - UI shows available and running children
   - Children can be started/stopped through the interface
   - Each child maintains its own state

3. **Communication**
   - WebSocket for real-time updates
   - HTTP server for web interface
   - Message-server for parent/child communication
   - Results feed forward into conversation

## Development

The project is built on the Theater actor system and uses:
- Rust for actor implementation
- WebAssembly for actor execution
- Claude API for chat responses
- Web technologies for the interface

### Building

```bash
# Build the actor
cargo build --release

# Run with Theater
theater run actor.toml
```

## Current Status

The system currently supports:
- Basic chat functionality
- Child actor management UI
- Starting/stopping children
- Message notification to children

Next steps:
- Implement directory scanning for available children
- Add error handling for child operations
- Enhance child result processing
- Add example child actors

## Contributing

Contributions are welcome! The project is actively developing and there are many areas for improvement.