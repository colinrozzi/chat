{
  description = "Chat actor WASM component";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs { inherit system; };
        fetchcargo = pkgs.rustPlatform.fetchCargoTarball;
        buildWasmPackage = import ./build-wasm-package.nix {
          inherit (pkgs) stdenv rustc cargo git cacert;
          inherit fetchcargo;
        };
      in {
        packages.default = buildWasmPackage {
          pname = "chat-actor";
          version = "0.1.0";
          src = ./.;
          cargoSha256 = "0000000000000000000000000000000000000000000000000000"; # Replace after first build

          nativeBuildInputs = with pkgs; [
            esbuild
            cargo-component
            nodejs
          ];

          buildPhase = ''
            export CARGO_HOME=$TMPDIR/cargo-home
            export RUSTUP_HOME=$TMPDIR/rustup-home
            export CARGO_TARGET_DIR=target

            echo "== Bundling frontend with esbuild =="
            mkdir -p assets/dist
            ${pkgs.esbuild}/bin/esbuild \
              --bundle assets/src/index.js \
              --outfile=assets/dist/chat.js

            echo "== Building WASM component =="
            cargo component build --release --target wasm32-unknown-unknown --output target
          '';

          installPhase = ''
            echo "== Installing chat WASM component =="
            mkdir -p $out/lib
            cp target/wasm32-unknown-unknown/release/chat.wasm $out/lib/
            cp -r assets $out/
            cp actor.portable.toml $out/
          '';
        };

        devShells.default = pkgs.mkShell {
          buildInputs = with pkgs; [
            rustc
            cargo
            esbuild
            cargo-component
            nodejs
          ];
          shellHook = ''
            echo "Welcome to the Chat Actor WASM dev shell!"
          '';
        };
      });
}
