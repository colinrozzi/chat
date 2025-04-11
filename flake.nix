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
      in {
        packages.default = pkgs.rustPlatform.buildRustPackage {
          pname = "chat-actor";
          version = "0.1.0";

          src = ./.;

          cargoLock = {
            lockFile = ./Cargo.lock;
          };

          # Optional: remove if not needed
          buildInputs = [ ];

          # If you want to test the binary after building
          doCheck = false;
        };

        devShell = pkgs.mkShell {
          buildInputs = with pkgs; [
            rustup
            cargo
          ];

          shellHook = ''
            echo "Chat Actor Development Environment"
          '';
        };
      }
    );
}
