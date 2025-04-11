{
  description = "Chat actor WebAssembly component";

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
        pkgs = import nixpkgs {
          inherit system overlays;
        };
        
        rustToolchain = pkgs.rust-bin.stable.latest.default.override {
          extensions = [ "rust-src" "rust-analyzer" ];
          targets = [ "wasm32-unknown-unknown" ];
        };

      in
      {
        devShells.default = pkgs.mkShell {
          buildInputs = with pkgs; [
            rustToolchain
            pkg-config
            openssl
            # Tools for WebAssembly development
            wasmtime
            binaryen
            wasm-tools
          ];

          RUST_SRC_PATH = "${rustToolchain}/lib/rustlib/src/rust/library";
        };

        packages.default = pkgs.stdenv.mkDerivation {
          pname = "chat-actor";
          version = "0.1.0";
          src = ./.;

          nativeBuildInputs = with pkgs; [ 
            rustToolchain
            pkg-config 
            wasm-tools
            binaryen
            cacert
          ];
          
          buildInputs = with pkgs; [ 
            openssl
            cacert
          ];

          buildPhase = ''
            # Set up cargo home
            export CARGO_HOME=$(mktemp -d)
            
            # Ensure SSL certificates are available
            export SSL_CERT_FILE=${pkgs.cacert}/etc/ssl/certs/ca-bundle.crt
            export NIX_SSL_CERT_FILE=${pkgs.cacert}/etc/ssl/certs/ca-bundle.crt

            echo "installing cargo-component"
            
            # Install cargo-component
            cargo install cargo-component

            echo "building chat actor"
            
            # Build the WebAssembly component
            cargo component build --release --target wasm32-unknown-unknown
            

          '';

          installPhase = ''
            mkdir -p $out/lib
            cp ./target/wasm32-unknown-unknown/release/chat.opt.wasm $out/lib/
            cp ./target/wasm32-unknown-unknown/release/chat.wasm $out/lib/
          '';

          # Allow network access during build
          __noChroot = true;
        };
      });
}
