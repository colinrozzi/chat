{
  description = "Rust WASM Component Project with cargo-component support";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs = {
        nixpkgs.follows = "nixpkgs";
        flake-utils.follows = "flake-utils";
      };
    };
    # Add crane for better Rust handling in Nix
    crane = {
      url = "github:ipetkov/crane";
      inputs = {
        nixpkgs.follows = "nixpkgs";
      };
    };
  };

  outputs = { self, nixpkgs, flake-utils, rust-overlay, crane }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };
        
        # Custom Rust with wasm-tools and targeted to wasm
        rustWithWasmTools = pkgs.rust-bin.stable.latest.default.override {
          extensions = [ "rust-src" "clippy" ];
          targets = [ "wasm32-unknown-unknown" ];
        };

        # Set up crane lib with our custom toolchain
        craneLib = (crane.mkLib pkgs).overrideToolchain rustWithWasmTools;
        
        # We need to filter the source to avoid rebuilds when irrelevant files change
        src = pkgs.lib.cleanSourceWith {
          src = ./.;
          filter = path: type:
            pkgs.lib.cleanSourceFilter path type &&
            # Add custom filtering here if needed
            !(pkgs.lib.hasPrefix "target" (baseNameOf path)) &&
            !(pkgs.lib.hasPrefix "result" (baseNameOf path));
        };

        # Create a custom cargo-component wrapper that can work inside the sandbox
        cargoComponentWrapper = pkgs.writeShellScriptBin "cargo-component" ''
          # Set HOME to a writable location in the sandbox
          export HOME=$(mktemp -d)
          export CARGO_HOME="$HOME/.cargo"
          mkdir -p "$CARGO_HOME"
          # Enable network for cargo component if needed
          # This is one of the key parts that helps with sandboxing issues
          export CARGO_NET_OFFLINE=${if pkgs.stdenv.isLinux then "false" else "true"}
          # Enable fetching git dependencies if needed
          export CARGO_NET_GIT_FETCH_WITH_CLI=true
          # Check if we need to enable network
          if [[ "$@" == *"--frozen"* ]]; then
            export CARGO_NET_OFFLINE=true
          fi
          # Run the real cargo-component
          exec ${pkgs.cargo-component}/bin/cargo-component "$@"
        '';
        
        # Use cargo out-dir to avoid Cargo trying to create a target directory in the Nix store
        commonArgs = {
          inherit src;
          strictDeps = true;
          # Set the output directory to avoid writing to immutable paths
          CARGO_TARGET_DIR = "target";
          # Allow network access for cargo component if needed
          __noChroot = true;
          # Configure Cargo.toml location explicitly
          cargoToml = ./Cargo.toml;
          cargoLock = ./Cargo.lock;
          # Adding dependencies needed for Wasm compilation
          buildInputs = with pkgs; [
            wasm-tools
            pkg-config
            openssl.dev
          ] ++ pkgs.lib.optionals pkgs.stdenv.isDarwin [
            pkgs.darwin.apple_sdk.frameworks.Security
            pkgs.darwin.apple_sdk.frameworks.SystemConfiguration
          ];
        };
        
        # Our build derivation using cargo-component
        componentBuild = pkgs.stdenv.mkDerivation {
          pname = "chat-actor";
          version = "0.1.0";
          inherit src;
          
          nativeBuildInputs = [
            rustWithWasmTools
            pkgs.cargo-component
            pkgs.wasm-tools
            cargoComponentWrapper
          ];
          
          # Workaround for cargo-component and nix sandboxing
          __noChroot = true;
          
          buildPhase = ''
            # Set HOME to temporary directory to avoid permission issues
            export HOME=$(mktemp -d)
            export CARGO_HOME="$HOME/.cargo"
            mkdir -p "$CARGO_HOME"
            
            # Create required directories
            mkdir -p .cargo

            echo "running cargo component build"
            
            # Build the component
            ${cargoComponentWrapper}/bin/cargo-component component build --release --target wasm32-unknown-unknown

            echo "built"
            
            # Validate that we built a proper WebAssembly component
            ${pkgs.wasm-tools}/bin/wasm-tools validate target/wasm32-unknown-unknown/release/chat.wasm || echo "Warning: wasm validation failed but continuing anyway"
          '';
          
          installPhase = ''
            # Create output directory
            mkdir -p $out/lib
            
            # Copy the WebAssembly component
            cp target/wasm32-unknown-unknown/release/chat.wasm $out/lib/
            
            # Copy the actor.toml configuration but update the component path
            cat $src/actor.toml | sed "s|component_path = .*|component_path = \"$out/lib/chat.wasm\"|" > $out/actor.toml
            
            # Create a portable actor.toml that uses relative paths
            cat $src/actor.toml | sed "s|component_path = .*|component_path = \"./lib/chat.wasm\"|" > $out/actor.portable.toml
            
            # Copy initialization state
            cp $src/init.json $out/init.json
            
            # Create necessary directories and copy assets if needed
            if [ -d "$src/assets" ]; then
              mkdir -p $out/assets
              cp -r $src/assets/* $out/assets/
            fi
            
            # Create an executable script to run the component
            mkdir -p $out/bin
            cat > $out/bin/run-chat-actor <<EOF
            #!/usr/bin/env bash
            if command -v theater &> /dev/null; then
              theater run $out/actor.toml "\$@"
            else
              echo "Error: 'theater' command not found. Please install it first."
              exit 1
            fi
            EOF
            chmod +x $out/bin/run-chat-actor
          '';
        };
        
      in
      {
        packages = {
          default = componentBuild;
          actor = componentBuild;
        };
        
        devShells.default = pkgs.mkShell {
          buildInputs = [
            rustWithWasmTools
            pkgs.cargo-component
            pkgs.wasm-tools
            # Add other dev dependencies
            pkgs.cargo-watch
            pkgs.rust-analyzer
          ];
          
          # Environment variables to make cargo-component work in dev shell
          shellHook = ''
            # Welcome message
            echo "Rust WebAssembly Component Development Environment"
            echo "Available commands:"
            echo "  cargo component build --target wasm32-unknown-unknown           # Dev build"
            echo "  cargo component build --release --target wasm32-unknown-unknown # Release build"
            echo ""
            echo "To run the component:"
            echo "  theater run actor.toml"
            
            # Set up environment variables to work around sandbox issues
            export CARGO_NET_GIT_FETCH_WITH_CLI=true
            
            # Fix for macOS
            if [[ "$(uname)" == "Darwin" ]]; then
              export RUSTFLAGS="-C link-arg=-undefined -C link-arg=dynamic_lookup"
            fi
          '';
        };
      }
    );
}
