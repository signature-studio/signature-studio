# shellcheck shell=bash
# Source this file: `source scripts/env.sh`. It only sets variables and runs `mkdir -p`; nothing is installed or changed elsewhere.
# Why (docs/ops/environment.md): emcc and npx are not on PATH, /usr/bin/wasm-opt is version 120 while the build expects 133,
# /opt/emsdk must not be written to, and /tmp is a 3.6 GiB tmpfs.

_ss_emsdk=/opt/emsdk
_ss_cache="$HOME/.cache/signature-studio"

_ss_node_bin=""
for _ss_d in "$_ss_emsdk"/node/*/bin; do
  [ -d "$_ss_d" ] && _ss_node_bin="$_ss_d"
done

_ss_add() {
  case ":$PATH:" in
    *":$1:"*) ;;
    *) PATH="$1:$PATH" ;;
  esac
}
# Added in reverse order of priority: the last one added ends up first. wasm-opt from emsdk (133) must come before /usr/bin (120).
[ -n "$_ss_node_bin" ] && _ss_add "$_ss_node_bin"
[ -d "$_ss_emsdk/upstream/emscripten" ] && _ss_add "$_ss_emsdk/upstream/emscripten"
[ -d "$_ss_emsdk/upstream/bin" ] && _ss_add "$_ss_emsdk/upstream/bin"
export PATH

export EM_CONFIG="$_ss_emsdk/.emscripten"
export EM_CACHE="$_ss_cache/em_cache"
export TMPDIR="$_ss_cache/tmp"
mkdir -p "$EM_CACHE" "$TMPDIR"

unset _ss_emsdk _ss_cache _ss_node_bin _ss_d
unset -f _ss_add
