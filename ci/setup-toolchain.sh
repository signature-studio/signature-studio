#!/bin/bash
# ci/setup-toolchain.sh emsdk|rust - the build tools at the versions of ci/versions.env.
#   emsdk: emscripten $EMSDK_VERSION (clang, binaryen's wasm-opt and node come with it) in $EMSDK_DIR (/opt/emsdk: app/scripts/build-engine-bundle.sh has that path in it). The directory is what the "tool-emsdk" cache holds.
#   rust:  rustup toolchain $RUST_TOOLCHAIN (minimal profile), made the default.
set -u -o pipefail
. "$(dirname "$0")/lib/common.sh"
case "${1:-}" in
  emsdk)
    if [ -x "$EMSDK_DIR/upstream/emscripten/emcc" ]; then log "emsdk is in place ($EMSDK_DIR)"; else
      [ -w "$EMSDK_DIR" ] || { sudo mkdir -p "$EMSDK_DIR" && sudo chown "$(id -u):$(id -g)" "$EMSDK_DIR"; }
      git clone --quiet "$EMSDK_REPO" "$EMSDK_DIR/.src" && git -C "$EMSDK_DIR/.src" checkout --quiet "$EMSDK_REV" && cp -a "$EMSDK_DIR/.src/." "$EMSDK_DIR/" && rm -rf "$EMSDK_DIR/.src"
      ( cd "$EMSDK_DIR" && ./emsdk install "$EMSDK_VERSION" && ./emsdk activate "$EMSDK_VERSION" )
    fi
    PATH=$EMSDK_DIR:$EMSDK_DIR/upstream/emscripten:$PATH
    emcc --version | head -1
    "$EMSDK_DIR/upstream/bin/wasm-opt" --version
    emcc --version | grep -q "$EMSDK_VERSION" || die "emcc is not $EMSDK_VERSION" ;;
  rust)
    command -v rustup >/dev/null || { curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal --default-toolchain none; }
    # shellcheck disable=SC1091
    [ -f "$HOME/.cargo/env" ] && . "$HOME/.cargo/env"
    rustup toolchain install "$RUST_TOOLCHAIN" --profile minimal --no-self-update
    # not `rustup default`: the version is selected by RUSTUP_TOOLCHAIN (ci/stage-lift.sh sets it too), so a workstation's own default is left alone
    [ -z "${GITHUB_ENV:-}" ] || echo "RUSTUP_TOOLCHAIN=$RUST_TOOLCHAIN" >> "$GITHUB_ENV"
    RUSTUP_TOOLCHAIN=$RUST_TOOLCHAIN rustc --version ;;
  *) die "usage: setup-toolchain.sh emsdk|rust" ;;
esac
