{
  description = "Chat actor for LLM chat application";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs { inherit system; };
        
        # Define the JavaScript bundle build command
        buildJavaScript = ''
          echo "Bundling JavaScript with esbuild..."
          mkdir -p assets/dist
          ${pkgs.esbuild}/bin/esbuild \
            assets/src/index.js \
            --bundle \
            --minify \
            --outfile=assets/dist/chat.js
        '';
      in {
        packages.default = pkgs.stdenv.mkDerivation {
          name = "chat-actor";
          src = ./.;
          
          nativeBuildInputs = with pkgs; [
            rustc
            cargo-component
            esbuild
            pkg-config
          ];
          
          buildPhase = ''
            # Bundle JavaScript
            ${buildJavaScript}
            
            # Build the Rust component
            cargo component build --release --target wasm32-unknown-unknown
          '';
          
          installPhase = ''
            mkdir -p $out
            cp target/wasm32-unknown-unknown/release/chat.wasm $out/
            cp -r assets $out/
            cp actor.portable.toml $out/
          '';
        };
        
        devShell = pkgs.mkShell {
          buildInputs = with pkgs; [
            rustup
            esbuild
            nodejs
          ];
          
          shellHook = ''
            echo "Chat Actor Development Environment"
            
            # Add a dev script for frontend development
            function dev-frontend() {
              ${pkgs.esbuild}/bin/esbuild \
                --bundle assets/src/index.js \
                --outfile=assets/dist/chat.js \
                --servedir=assets \
                --serve=0.0.0.0:8085 \
                --watch
            }
            
            echo "Run 'dev-frontend' to start the frontend development server"
          '';
        };
      }
    );
}
