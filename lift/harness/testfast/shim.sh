#!/bin/bash
# Prints the directory that holds the caching `clang` (clangcache.c), building it first if needed. Used by runner.sh and fletest.py.
set -u
here=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
src=$here/clangcache.c
nsrc=$here/nocore.c
cache=${1:-${FLE_TEST_CACHE:-${CARGO_TARGET_DIR:-$here/../../target}/fle-test-cache}}
stamp=$(cat "$src" "$nsrc" | sha256sum | cut -c1-16)
dir=$cache/bin/shim-$stamp
if [ ! -x "$dir/clang" ]; then
    mkdir -p "$cache/bin" || exit 1
    tmp=$cache/bin/.build-$$
    mkdir -p "$tmp"
    cc=$(command -v cc || command -v gcc || command -v clang) || exit 1
    "$cc" -O2 -o "$tmp/clang" "$src" || { rm -rf "$tmp"; exit 1; }
    "$cc" -O2 -shared -fPIC -o "$tmp/libnocore.so" "$nsrc" -ldl || { rm -rf "$tmp"; exit 1; }
    mv "$tmp" "$dir" 2>/dev/null || rm -rf "$tmp"
fi
echo "$dir"
