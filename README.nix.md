# Nix Flake for Chat Actor System

This document explains how to use the Nix flake to build and run the Chat Actor WebAssembly component.

## Prerequisites

- Nix package manager with flakes enabled
- Theater runtime for running the actor

If you don't have Nix with flakes enabled, you can set it up by:

1. Installing Nix:
   ```bash
   curl -L https://nixos.org/nix/install | sh
   ```

2. Enabling flakes (edit `~/.config/nix/nix.conf` or `/etc/nix/nix.conf`):
   ```
   experimental-features = nix-command flakes
   ```

## Using the Flake

### Development Environment

To enter a development shell with all the necessary tools:

```bash
nix develop
```

This provides:
- Rust toolchain with wasm32-unknown-unknown target
- cargo-component command
- build-component helper script
- All required dependencies

### Building the Project

There are two ways to build the project:

#### 1. Using the development shell:

```bash
# Enter the development shell
nix develop

# Build using cargo-component directly
cargo component build --release --target wasm32-unknown-unknown

# Or use the helper script that handles sandboxing issues
build-component
```

#### 2. Using nix build:

```bash
# Build the project
nix build

# The result will be in ./result
```

### Running the Chat Actor

If you have Theater installed:

```bash
# Run the actor from the build output
theater run ./result/actor.portable.toml
```

Or use the bundled runner (if you have Theater in your PATH):

```bash
# Run using the bundled application
nix run
```

## Structure of the Flake

The flake provides:

1. **packages.default**: The built chat actor with all necessary files
2. **apps.default**: A simple wrapper to run the actor using Theater
3. **devShells.default**: A development environment with all tools

## How It Works

The flake addresses the Nix sandboxing issues with cargo in several ways:

1. Uses `__noChroot = true` to build outside of the sandbox
2. Sets up a local CARGO_HOME to avoid permission issues
3. Provides a separate build script that can be run in the development shell

## Customization

If you need to customize the build process:

1. Edit the `flake.nix` file
2. Modify the `buildPhase` in the chatActor derivation
3. Add any additional dependencies to `buildInputs` or `nativeBuildInputs`

## Troubleshooting

### Common Issues

1. **cargo-component not found**:
   If you see errors about cargo-component not being found, ensure you're using the development shell or check that the hash in flake.nix is correct.

2. **Build fails with permission errors**:
   This may indicate sandboxing issues. Try using the development shell and the `build-component` script.

3. **WebAssembly component not found**:
   Check that the build completed successfully and that the paths in actor.portable.toml are correct.

### Checking Build Output

To inspect the contents of the build:

```bash
ls -la ./result/
ls -la ./result/lib/
ls -la ./result/assets/
```

## Advanced Usage

### Using with direnv

If you use direnv, create a `.envrc` file with:

```
use flake
```

Then:

```bash
direnv allow
```

### Updating Dependencies

If you need to update the Rust overlay or nixpkgs:

```bash
nix flake update
```

Or to update specific inputs:

```bash
nix flake lock --update-input rust-overlay
```
