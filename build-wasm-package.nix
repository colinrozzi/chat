{ stdenv, fetchcargo, rustc, cargo, git, cacert }:

{ pname
, version
, src
, cargoSha256
, nativeBuildInputs ? []
, buildInputs ? []
, cargoBuildFlags ? []
, buildPhase ? null
, installPhase ? null
, cargoPatches ? []
, logLevel ? ""
}:

let
  cargoDeps = fetchcargo {
    inherit pname src;
    patches = cargoPatches;
    sha256 = cargoSha256;
  };
in
stdenv.mkDerivation {
  inherit pname version src;

  nativeBuildInputs = nativeBuildInputs ++ [ cargo rustc git cacert ];
  buildInputs = buildInputs;

  cargoDeps = cargoDeps;

  postUnpack = ''
    echo "== Setting up Cargo vendor directory =="
    unpackFile "$cargoDeps"
    cargoDepsCopy=$(stripHash $(basename $cargoDeps))
    chmod -R +w "$cargoDepsCopy"

    mkdir -p .cargo
    substitute ${./fetchcargo-default-config.toml} .cargo/config \
      --subst-var-by vendor "$(pwd)/$cargoDepsCopy"

    export RUST_LOG=${logLevel}
  '';

  buildPhase = buildPhase or ''
    echo "== Building WASM component =="
    export CARGO_HOME=$TMPDIR/cargo-home
    export RUSTUP_HOME=$TMPDIR/rustup-home
    export CARGO_TARGET_DIR=target

    cargo component build --release --target wasm32-unknown-unknown --output target \
      ${stdenv.lib.concatStringsSep " " cargoBuildFlags}
  '';

  installPhase = installPhase or ''
    echo "== Installing output files =="
    mkdir -p $out/lib
    cp target/wasm32-unknown-unknown/release/*.wasm $out/lib/
    cp -r assets $out/
    cp actor.portable.toml $out/
  '';

  dontConfigure = true;

  meta = {
    description = "WASM component for ${pname}";
    platforms = [ "x86_64-linux" "aarch64-linux" ];
  };
}
