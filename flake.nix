{
  description = "Chat WebAssembly Component";

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
        
        # Use a stable Rust toolchain with wasm32 target
        rustToolchain = pkgs.rust-bin.stable.latest.default.override {
          targets = [ "wasm32-unknown-unknown" ];
          extensions = [ "rust-src" ];
        };

        # Build a custom cargo-component package
        cargo-component = pkgs.rustPlatform.buildRustPackage rec {
          pname = "cargo-component";
          version = "0.8.1";
          
          src = pkgs.fetchFromGitHub {
            owner = "bytecodealliance";
            repo = "cargo-component";
            rev = "v${version}";
            sha256 = "sha256-PcJI/uBLRd9VTVL7XbQZVL4hS03/4FBvbhwVu5jA5BE="; 
            # You might need to update this hash if the version changes
          };

          cargoSha256 = "sha256-cQwL5A9oa+hUe9bQE5hiGOqk7uYORUxH12dQXnLAQkg="; 
          # You might need to update this hash if the version changes
          
          nativeBuildInputs = [ pkgs.pkg-config ];
          buildInputs = [ pkgs.openssl ];
        };
        
        # Create the WASM component build script
        buildScript = pkgs.writeShellScriptBin "build-wasm-component" ''
          set -e
          export PATH=${rustToolchain}/bin:${cargo-component}/bin:$PATH
          
          cd $src
          
          # Use a non-sandboxed directory for cargo to work in
          export CARGO_HOME=$(mktemp -d)
          export CARGO_TARGET_DIR=$out/target
          
          # Create the output directories
          mkdir -p $out/lib
          
          # Build the component
          cargo component build --release --target wasm32-unknown-unknown
          
          # Copy the built WASM file to the output
          cp target/wasm32-unknown-unknown/release/chat.wasm $out/lib/
          
          # Copy the actor configuration file
          cp actor.portable.toml $out/
          
          # Create init.json directory in assets if it doesn't exist
          mkdir -p $out/assets
          if [ -f init.json ]; then
            cp init.json $out/assets/
          fi
          
          # Copy the assets directory
          if [ -d assets ]; then
            cp -r assets/* $out/assets/
          fi
        '';

      in {
        packages = {
          # Create the chat-actor derivation
          chat-actor = pkgs.stdenv.mkDerivation {
            pname = "chat-actor";
            version = "0.1.0";
            src = self;
            
            nativeBuildInputs = [
              buildScript
              rustToolchain
              cargo-component
              pkgs.pkg-config
            ];
            
            buildInputs = [ pkgs.openssl ];
            
            # This is required to allow cargo to fetch dependencies
            # during the build despite Nix's sandbox
            SSL_CERT_FILE = "${pkgs.cacert}/etc/ssl/certs/ca-bundle.crt";
            GIT_SSL_CAINFO = "${pkgs.cacert}/etc/ssl/certs/ca-bundle.crt";
            
            # Disable Nix's standard build phases since we're using our own script
            phases = [ "buildPhase" ];
            
            buildPhase = ''
              build-wasm-component
            '';
          };
          
          default = self.packages.${system}.chat-actor;
        };
        
        # Development shell for working on the component
        devShells.default = pkgs.mkShell {
          buildInputs = [
            rustToolchain
            cargo-component
            pkgs.pkg-config
            pkgs.openssl
          ];
          
          # Environment variables for development
          SSL_CERT_FILE = "${pkgs.cacert}/etc/ssl/certs/ca-bundle.crt";
          GIT_SSL_CAINFO = "${pkgs.cacert}/etc/ssl/certs/ca-bundle.crt";
        };
      }
    );
}
