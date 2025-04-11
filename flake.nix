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
        packages.default = pkgs.rustPlatform.buildRustPackage {
          pname = "chat-actor";
          version = "0.1.0";

          src = ./.;

          useFetchCargoVendor = true;
          cargoHash = "sha256-RSsYmToNPKQYBzx5QskWUpdumn7jbyI5Ad3IIf8p1dA=";

          nativeBuildInputs = with pkgs; [
            esbuild
            cargo-component
            nodejs
            lld_19
          ];

          cargoBuildFlags = [
            "--target=wasm32-unknown-unknown"
          ];

          # 👇 This runs before `cargo build`
          preBuild = ''
            echo "== Bundling frontend with esbuild =="

            mkdir -p assets/dist
            ${pkgs.esbuild}/bin/esbuild \
              --bundle assets/src/index.js \
              --outfile=assets/dist/chat.js
          '';

          installPhase = ''
            echo "== Installing chat wasm component =="
            mkdir -p $out/lib
            cp target/wasm32-unknown-unknown/release/chat.wasm $out/lib/
            cp -r assets $out/
            cp actor.portable.toml $out/
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
          '';
        };
      }
    );
}
