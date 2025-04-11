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

        # We need cargo-component, which isn't in nixpkgs
        cargoComponent = pkgs.rustPlatform.buildRustPackage rec {
          pname = "cargo-component";
          version = "0.9.0";
          
          src = pkgs.fetchFromGitHub {
            owner = "bytecodealliance";
            repo = "cargo-component";
            rev = "v${version}";
            sha256 = "sha256-HWnM7o8T+T1AlhPQnNeC5LpD0tLbLg3PSWSD0tYjZQI="; # Replace with actual hash
          };
          
          cargoSha256 = "sha256-mPbqV1oBL0bjKGNDyvYQnMGvLZzBmqxlsXRaOgnfdmY="; # Replace with actual hash
          
          nativeBuildInputs = with pkgs; [ 
            pkg-config 
            rustToolchain
          ];
          
          buildInputs = with pkgs; [
            openssl
          ] ++ lib.optionals stdenv.isDarwin [
            darwin.apple_sdk.frameworks.Security
            darwin.apple_sdk.frameworks.SystemConfiguration
          ];
          
          doCheck = false;  # Skip tests for build speed
        };

        # Create a shell script to build the component
        buildScript = pkgs.writeShellScriptBin "build-component" ''
          #!/usr/bin/env bash
          set -euo pipefail
          
          export PATH="${rustToolchain}/bin:${cargoComponent}/bin:$PATH"
          
          echo "Building WebAssembly component..."
          cd "$PWD"
          
          # Disable Nix sandboxing for cargo
          # This is the key part that helps avoid Nix sandboxing issues
          export CARGO_HOME="$PWD/.cargo-home"
          mkdir -p "$CARGO_HOME"
          
          # Build the component
          cargo component build --release --target wasm32-unknown-unknown
          
          # Create output directory structure
          mkdir -p $out/{lib,assets}
          
          # Copy the wasm component
          cp target/wasm32-unknown-unknown/release/chat.wasm $out/lib/
          
          # Copy assets and configuration
          cp -r assets/* $out/assets/ || true
          cp actor.portable.toml $out/
          
          # Make sure init.json exists in assets
          cp init.json $out/assets/ || true
          
          echo "Build completed successfully!"
        '';

        # The main package
        chatActor = pkgs.stdenv.mkDerivation {
          name = "chat-actor";
          version = "0.1.0";
          
          src = ./.;
          
          nativeBuildInputs = [
            rustToolchain
            cargoComponent
            pkgs.pkg-config
          ];
          
          buildInputs = with pkgs; [
            openssl
          ];
          
          # This is where we use IFD (Import From Derivation) to work around Nix sandboxing
          buildPhase = ''
            export PATH="${rustToolchain}/bin:${cargoComponent}/bin:$PATH"
            
            # Create directories
            mkdir -p $out/{lib,assets}
            
            # Disable Nix sandboxing for cargo
            export CARGO_HOME="$PWD/.cargo-home"
            mkdir -p "$CARGO_HOME"
            
            # Build the component
            cargo component build --release --target wasm32-unknown-unknown
            
            # Copy the wasm component
            cp target/wasm32-unknown-unknown/release/chat.wasm $out/lib/
            
            # Copy assets and configuration
            if [ -d "assets" ]; then
              cp -r assets/* $out/assets/ || true
            fi
            
            cp actor.portable.toml $out/
            
            # Make sure init.json exists in assets
            cp init.json $out/assets/ || true
          '';
          
          # Skip install phase as we do everything in buildPhase
          installPhase = "echo 'Install phase skipped'";
          
          # Allow the build to happen locally rather than in a sandbox
          __noChroot = true;
          
          meta = {
            description = "Chat Actor System - WebAssembly Component";
            mainProgram = "chat-actor";
          };
        };

        # A simple wrapper script to run the actor using Theater
        theaterRunner = pkgs.writeShellScriptBin "run-chat-actor" ''
          #!/usr/bin/env bash
          # Assumes theater is installed and in PATH
          ACTOR_PATH="${chatActor}/actor.portable.toml"
          echo "Running chat actor from: $ACTOR_PATH"
          theater run "$ACTOR_PATH" "$@"
        '';

      in {
        packages = {
          default = chatActor;
          chatActor = chatActor;
        };
        
        apps.default = {
          type = "app";
          program = "${theaterRunner}/bin/run-chat-actor";
        };
        
        devShells.default = pkgs.mkShell {
          buildInputs = with pkgs; [
            rustToolchain
            cargoComponent
            buildScript
            openssl
            pkg-config
          ] ++ lib.optionals stdenv.isDarwin [
            darwin.apple_sdk.frameworks.Security
            darwin.apple_sdk.frameworks.SystemConfiguration
          ];
          
          shellHook = ''
            echo "=== Chat Actor Development Environment ==="
            echo "Available commands:"
            echo "  cargo component - Build WebAssembly component"
            echo "  build-component - Build component outside of Nix sandbox"
            echo ""
            echo "Target will be in target/wasm32-unknown-unknown/release/chat.wasm"
          '';
        };
      }
    );
}
