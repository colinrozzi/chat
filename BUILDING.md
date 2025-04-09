# Building the Chat WebAssembly Component

This document provides detailed instructions for building the chat WebAssembly component.

## Important Note

**This project is compiled to a WebAssembly component and run in a custom actor system.**

Standard Cargo build commands (like `cargo build`) will not work correctly. You must use the `cargo-component` toolchain as described below.

## Prerequisites

1. Rust toolchain installed (https://rustup.rs/)
2. WebAssembly target installed:
   ```bash
   rustup target add wasm32-unknown-unknown
   ```
3. Cargo Component tool installed:
   ```bash
   cargo install cargo-component
   ```

## Build Commands

### Development Build

```bash
cargo component build --target wasm32-unknown-unknown
```

The output will be located at `target/wasm32-unknown-unknown/debug/chat.wasm`.

### Release Build (Optimized)

```bash
cargo component build --release --target wasm32-unknown-unknown
```

The output will be located at `target/wasm32-unknown-unknown/release/chat.wasm`.

### Cleaning the Build

If you need to clean the build artifacts:

```bash
cargo clean
```

## Troubleshooting

### Common Issues

1. **Linking errors with standard cargo build**:
   Error messages like "undefined symbols for architecture" are expected when using standard `cargo build`. Remember to use `cargo component build` instead.

2. **Cargo component command not found**:
   Ensure you have installed `cargo-component` with `cargo install cargo-component`.

3. **Missing WIT files**:
   The project requires WIT (WebAssembly Interface Type) files in the `wit/` directory. If these are missing or have been modified, compilation may fail.

4. **Outdated cargo-component**:
   If you encounter unexpected errors, try updating cargo-component:
   ```bash
   cargo install --force cargo-component
   ```

## Running the Component

After building, the component can be run using the Theater runtime system:

```bash
theater run actor.toml
```

Or with the portable configuration:

```bash
theater run ./result/actor.portable.toml
```

## Additional Resources

- [WebAssembly Component Model](https://github.com/WebAssembly/component-model)
- [Cargo Component Documentation](https://github.com/bytecodealliance/cargo-component)
