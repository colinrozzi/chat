{
  description = "Rust WASM Component Project";

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
        pkgs = import nixpkgs {
          inherit system overlays;
        };
        
        # Custom Rust with wasm-tools and cargo-component
        rustWithWasmTools = pkgs.rust-bin.stable.latest.default.override {
          extensions = [ "rust-src" ];
          targets = [ "wasm32-unknown-unknown" "wasm32-wasi" ];
        };
        
        # Build the WASM component
        buildRustWasmComponent = pkgs.stdenv.mkDerivation {
          pname = "rust-wasm-component";
          version = "0.1.0";
          src = ./.;
          
          nativeBuildInputs = [
            rustWithWasmTools
            pkgs.cargo-component
            pkgs.wasm-tools
          ];
          
          buildPhase = ''
            export HOME=$(mktemp -d)
            cargo component build --release
          '';
          
          installPhase = ''
            mkdir -p $out/lib
            cp target/wasm32-wasi/release/*.wasm $out/lib/
          '';
        };
      in
      {
        packages = {
          default = buildRustWasmComponent;
        };
        
        devShells.default = pkgs.mkShell {
          buildInputs = [
            rustWithWasmTools
            pkgs.cargo-component
            pkgs.wasm-tools
          ];
          
          shellHook = ''
            echo "Rust WASM component development environment"
            echo "Available commands:"
            echo "  cargo component build - Build the WASM component"
            echo "  cargo component build --release - Build in release mode"
          '';
        };
      }
    );
}
