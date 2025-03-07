# Chat Actor System Guidelines

## Build & Run Commands
- **Build with Nix**: `nix build`
- **Build Manually**: `cargo build --target wasm32-unknown-unknown --release`
- **Note**: Running directly from this directory is not currently supported
- **Lint**: `cargo clippy`, `cargo fmt`

## Code Style
- **Naming**: snake_case for variables/functions, PascalCase for types/structs
- **Formatting**: 4-space indentation, consistent braces on new lines
- **Error Handling**: Use `Result<T, String>` with `?` operator, log errors
- **Imports**: Group by module, separate std/external/local imports
- **Pattern Matching**: Prefer `match` expressions for handling `Result`/`Option`
- **Logging**: Use `log()` function for debug messages

## Architecture
- **Actor Model**: Parent/child architecture with message passing
- **WebAssembly**: Components target wasm32-unknown-unknown
- **Interfaces**: WebSocket (Port 8085), HTTP (Port 8084)
- **State**: Store chat messages and child actor information