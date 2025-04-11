{
  description = "Chat actor for LLM chat application";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs = {
        nixpkgs.follows = "nixpkgs";
        flake-utils.url = "flake-utils";
      };
    };
  };

  outputs = { self, nixpkgs, flake-utils, rust-overlay }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs { inherit system overlays; };

        # Define our Rust toolchain as before
        rustToolchain = pkgs.rust-bin.stable.latest.default.override {
          targets = [ "wasm32-unknown-unknown" ];
          extensions = [ "rust-src" ];
        };
      in {
        packages.default = pkgs.stdenv.mkDerivation rec {
          pname = "chat-actor";
          version = "0.1.0";

          src = ./.;

          # Native tools needed for the build
          nativeBuildInputs = with pkgs; [
            esbuild
            cargo-component
            nodejs
            lld_19
          ];

          # Inject the Rust toolchain into the build environment.
          buildInputs = [ rustToolchain ];

          # Prebuild phase: bundle frontend with esbuild.
          preBuild = ''
            echo "== Bundling frontend with esbuild =="
            mkdir -p assets/dist
            ${pkgs.esbuild}/bin/esbuild \
              --bundle assets/src/index.js \
              --outfile=assets/dist/chat.js
          '';

buildPhase = ''
  echo "== Building WASM component =="
  echo $TMPDIR
  export CARGO_HOME=$TMPDIR/cargo-home
  export RUSTUP_HOME=$TMPDIR/rustup-home
  mkdir -p $CARGO_HOME $RUSTUP_HOME
  echo "cargo home: $CARGO_HOME"
  echo "rustup home: $RUSTUP_HOME"
  echo $(ls -la $TMPDIR)

  # Set local target directory to avoid writing to default
  export CARGO_TARGET_DIR=target

  cargo component build --release --target wasm32-unknown-unknown --output target
'';

          # Install phase: copy the built WASM binary and assets.
          installPhase = ''
            echo "== Installing chat WASM component =="
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
