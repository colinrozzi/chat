{
  description = "Chat actor for LLM chat application";

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
  };

  outputs = { self, nixpkgs, flake-utils, rust-overlay }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs { inherit system overlays; };
        
        # Define the Rust toolchain
        rustToolchain = pkgs.rust-bin.stable.latest.default.override {
          targets = [ "wasm32-unknown-unknown" ];
          extensions = [ "rust-src" ];
        };
      in {
        packages.default = pkgs.stdenv.mkDerivation {
          name = "chat-actor";
          src = ./.;
          
          nativeBuildInputs = with pkgs; [
            rustToolchain
            pkg-config
            esbuild
            cargo-component
            bash
            which
          ];
          
          dontPatch = true;
          dontConfigure = true;
          
          buildPhase = ''
            # Ensure cargo and cargo-component are in the PATH
            export PATH="${rustToolchain}/bin:${pkgs.cargo-component}/bin:$PATH"
            
            # Print debug information
            echo "== Build Environment =="
            echo "PATH: $PATH"
            echo "$(which cargo) (version: $(cargo --version))"
            echo "$(which cargo-component) (version: $(cargo-component --version))"
            echo "$(which esbuild) (version: $(esbuild --version))"

            # Create necessary directories
            mkdir -p assets/dist

            # Bundle JavaScript assets
            echo "== Bundling JavaScript =="
            esbuild \
              assets/src/index.js \
              --bundle \
              --minify \
              --outfile=assets/dist/chat.js

            # Build Rust component
            echo "== Building Rust Component =="
            export CARGO_HOME=cargo
            export CARGO_TARGET_DIR=target
            mkdir -p $CARGO_HOME $CARGO_TARGET_DIR
            cargo component build --release --target wasm32-unknown-unknown \
              --out-dir $CARGO_TARGET_DIR/wasm32-unknown-unknown/release \
              --target-dir $CARGO_TARGET_DIR
          '';
          
          installPhase = ''
            echo "== Installing to $out =="
            mkdir -p $out/lib
            cp target/wasm32-unknown-unknown/release/chat.wasm $out/lib/
            cp -r assets $out/
            cp actor.portable.toml $out/
            echo "== Build Completed Successfully =="
          '';
        };
        
        devShell = pkgs.mkShell {
          buildInputs = with pkgs; [
            rustToolchain
            cargo-component
            esbuild
            nodejs
          ];
          
          shellHook = ''
            echo "Chat Actor Development Environment"
            
            # Add a dev script for frontend development
            function dev-frontend() {
              mkdir -p assets/dist
              ${pkgs.esbuild}/bin/esbuild \\
                --bundle assets/src/index.js \\
                --outfile=assets/dist/chat.js \\
                --servedir=assets \\
                --serve=0.0.0.0:8085 \\
                --watch
            }
            
            echo "Available commands:"
            echo "  dev-frontend - Start the frontend development server"
          '';
        };
      }
    );
}
