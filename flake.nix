{
  description = "Chat Actor System - WebAssembly Component";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, rust-overlay, flake-utils, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs { inherit system overlays; };
        
        rustToolchain = pkgs.rust-bin.stable.latest.default.override {
          targets = [ "wasm32-unknown-unknown" ];
          extensions = [ "rust-src" ];
        };
        
        # Basic development shell without the complex cargo-component package
        basicShell = pkgs.mkShell {
          buildInputs = with pkgs; [
            rustToolchain
            openssl
            pkg-config
          ] ++ lib.optionals stdenv.isDarwin [
            darwin.apple_sdk.frameworks.Security
            darwin.apple_sdk.frameworks.SystemConfiguration
          ];
          
          shellHook = ''
            echo "=== Chat Actor Development Environment ==="
            echo ""
            echo "To build the WebAssembly component, first install cargo-component:"
            echo "cargo install cargo-component"
            echo ""
            echo "Then build with:"
            echo "cargo component build --release --target wasm32-unknown-unknown"
          '';
        };
        
        # Very simple build script for the shell
        buildScript = pkgs.writeShellScriptBin "build-component" ''
          #!/usr/bin/env bash
          set -euo pipefail
          
          if ! command -v cargo-component &> /dev/null; then
            echo "cargo-component not found. Please install it first:"
            echo "cargo install cargo-component"
            exit 1
          fi
          
          echo "Building WebAssembly component..."
          cargo component build --release --target wasm32-unknown-unknown
          
          echo "Build completed successfully!"
          echo "Output is in target/wasm32-unknown-unknown/release/chat.wasm"
        '';

      in {
        # For now, just provide a development shell
        devShells.default = pkgs.mkShell {
          buildInputs = with pkgs; [
            rustToolchain
            openssl
            pkg-config
            buildScript
          ] ++ lib.optionals stdenv.isDarwin [
            darwin.apple_sdk.frameworks.Security
            darwin.apple_sdk.frameworks.SystemConfiguration
          ];
          
          shellHook = ''
            echo "=== Chat Actor Development Environment ==="
            echo "Available commands:"
            echo "  build-component - Helper script to build the component"
            echo ""
            echo "First install cargo-component if not already installed:"
            echo "  cargo install cargo-component"
            echo ""
            echo "Then you can build using:"
            echo "  cargo component build --release --target wasm32-unknown-unknown"
            echo "or use the helper script:"
            echo "  build-component"
          '';
        };
      }
    );
}
