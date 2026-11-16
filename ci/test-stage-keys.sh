#!/usr/bin/env bash
# ci/test-stage-keys.sh - checks ci/stage-keys.sh (decision 0391) on throwaway git repositories under $TMPDIR/opencode (not on this one, except two read-only runs):
# the same inputs give the same key; each input the gen stage reads changes it; a file the stage does not read (lift/runtime) does not; a change of another
# stage's code (stage_wasm) does not; a change of the gen stage's code, of the script's constants, of a tool version, and of the upstream key (the extraction
# layer's key) does. One PASS or FAIL line per check and a summary; exit 1 when a check fails.
# usage: bash ci/test-stage-keys.sh
set -u -o pipefail
here=$(cd "$(dirname "$0")" && pwd) || exit 2
root=$(cd "$here/.." && pwd) || exit 2
base=${TMPDIR:-/tmp}/opencode/stage-keys-test
rm -rf "$base"; mkdir -p "$base"
pass=0; fail=0
check() { if [ "$2" = yes ]; then pass=$((pass + 1)); echo "PASS  $1"; else fail=$((fail + 1)); echo "FAIL  $1"; fi; }
same() { if [ -n "$1" ] && [ "$1" = "$2" ]; then echo yes; else echo no; fi; }
differ() { if [ -n "$1" ] && [ -n "$2" ] && [ "$1" != "$2" ]; then echo yes; else echo no; fi; }

make_repo() { # dir: the files the gen key reads (a copy), in a new git repository (the key reads the index)
  local t=$1
  rm -rf "$t"
  mkdir -p "$t/ci/lib" "$t/app/scripts" "$t/lift/harness/two" "$t/lift/harness/hooks" "$t/lift/crates/fe-lift/src" "$t/lift/runtime" "$t/lift/data" "$t/lift/.cargo"
  cp "$root/ci/stage-keys.sh" "$root/ci/versions.env" "$t/ci/"
  cp "$root/ci/lib/common.sh" "$t/ci/lib/"
  cp "$root/app/scripts/build-engine-bundle.sh" "$t/app/scripts/"
  echo "gen_two" > "$t/lift/harness/two/gen_two.sh"
  echo "hook" > "$t/lift/harness/hooks/flengine-hook-points.txt"
  echo "fn main() {}" > "$t/lift/crates/fe-lift/src/main.rs"
  echo "runtime" > "$t/lift/runtime/fle_x.c"
  echo "prune" > "$t/lift/data/prune-SM.tsv"
  echo "[package]" > "$t/lift/Cargo.toml"
  echo "lock" > "$t/lift/Cargo.lock"
  echo "[build]" > "$t/lift/.cargo/config.toml"
  git -C "$t" init -q && git -C "$t" add -A
}
key_in() { bash "$1/ci/stage-keys.sh" gen "$2" 2> /dev/null | sed -n 's/^key=//p'; } # repo, upstream key -> the key
edit() { sed -i "$3" "$1/$2" && git -C "$1" add -A; } # repo, file, sed expression: the index is updated
UP=$(printf 'a%.0s' {1..64})
UP2=$(printf 'b%.0s' {1..64})

R="$base/r0"; make_repo "$R"
K0=$(key_in "$R" "$UP")
check "the key has the form fl-gen-<40 hex digits> ($K0)" "$(printf '%s\n' "$K0" | grep -qE '^fl-gen-[0-9a-f]{40}$' && echo yes || echo no)"
check "the same inputs give the same key" "$(same "$K0" "$(key_in "$R" "$UP")")"
check "the upstream key changes the key" "$(differ "$K0" "$(key_in "$R" "$UP2")")"

appended() { # label, file: one line appended to a file the gen stage reads: the key changes
  local R="$base/a-$1"
  make_repo "$R"; echo "x" >> "$R/$2"; git -C "$R" add -A
  check "appending a line to $2 changes the key" "$(differ "$K0" "$(key_in "$R" "$UP")")"
}
appended harness lift/harness/hooks/flengine-hook-points.txt
appended harness-two lift/harness/two/gen_two.sh
appended crates lift/crates/fe-lift/src/main.rs
appended cargolock lift/Cargo.lock
appended cargotoml lift/Cargo.toml
appended cargoconfig lift/.cargo/config.toml
appended data lift/data/prune-SM.tsv

R="$base/rt"; make_repo "$R"; echo "more" >> "$R/lift/runtime/fle_x.c"; git -C "$R" add -A
check "lift/runtime (no path of the gen stage reads it) does not change the key" "$(same "$K0" "$(key_in "$R" "$UP")")"

R="$base/wasm"; make_repo "$R"; edit "$R" app/scripts/build-engine-bundle.sh 's/^  for s in rt harness units; do$/  for s in rt harness units; do # another stage/'
check "a change in another stage (stage_wasm) does not change the gen key" "$(same "$K0" "$(key_in "$R" "$UP")")"

R="$base/gencode"; make_repo "$R"; edit "$R" app/scripts/build-engine-bundle.sh 's/TAG=tb2d2$/TAG=tb2d3/'
check "a change in the gen stage's own code changes the key" "$(differ "$K0" "$(key_in "$R" "$UP")")"

R="$base/preamble"; make_repo "$R"; edit "$R" app/scripts/build-engine-bundle.sh 's/^FRUITY_SECONDS=36000 /FRUITY_SECONDS=36001 /'
check "a change of the script's constants (the preamble) changes the key" "$(differ "$K0" "$(key_in "$R" "$UP")")"

R="$base/rust"; make_repo "$R"; edit "$R" ci/versions.env 's/^RUST_TOOLCHAIN=1.99.0$/RUST_TOOLCHAIN=1.99.1/'
check "a change of the rust version changes the key" "$(differ "$K0" "$(key_in "$R" "$UP")")"

R="$base/py"; make_repo "$R"; edit "$R" ci/versions.env 's/^PYTHON_VERSION=3.14$/PYTHON_VERSION=3.15/'
check "a change of the python version changes the key" "$(differ "$K0" "$(key_in "$R" "$UP")")"

R="$base/refuse"; make_repo "$R"
rc=$(bash "$R/ci/stage-keys.sh" gen short > /dev/null 2>&1; echo $?)
check "an upstream key of the wrong form is refused (exit 2)" "$([ "$rc" = 2 ] && echo yes || echo no)"
out=$(bash "$R/ci/stage-keys.sh" wasm "$UP" 2> /dev/null); rc=$?
check "an unknown stage is refused (exit 2, no key line)" "$([ "$rc" = 2 ] && [ -z "$out" ] && echo yes || echo no)"

A=$(key_in "$root" "$UP"); B=$(key_in "$root" "$UP")
check "the real repository: two runs give the same key ($A)" "$(same "$A" "$B")"

echo "test-stage-keys: $pass passed, $fail failed"
[ "$fail" -eq 0 ]
