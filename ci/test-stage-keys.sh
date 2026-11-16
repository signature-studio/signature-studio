#!/usr/bin/env bash
# ci/test-stage-keys.sh - checks ci/stage-keys.sh (decisions 0391, 0395) on throwaway git repositories under $TMPDIR/opencode (not on this one, except read-only runs):
# gen (0391): the same inputs give the same key; each input the gen stage reads changes it; lift/runtime does not; another stage's code does not; the script's constants,
# the tool versions and the upstream key do. wasm, hybn0, wasmopt, hybrid (0395, content-addressed): the same bytes give the same key; one byte of an input the stage reads
# changes it; a change of an upstream output changes the stages after it and not the ones before it; a change of another stage's function does not; a tool version does;
# a missing input is refused. bundle (0395): the whole input hash gives the key; another hash, or a malformed one, gives another key or is refused.
# One PASS or FAIL line per check and a summary; exit 1 when a check fails.
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

make_repo() { # dir: the files the keys read (a copy), in a new git repository (the keys read the index)
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
out=$(bash "$R/ci/stage-keys.sh" nosuch "$UP" 2> /dev/null); rc=$?
check "an unknown stage is refused (exit 2, no key line)" "$([ "$rc" = 2 ] && [ -z "$out" ] && echo yes || echo no)"

A=$(key_in "$root" "$UP"); B=$(key_in "$root" "$UP")
check "the real repository: two runs give the same gen key ($A)" "$(same "$A" "$B")"

# ---- decision 0395 and 0395: the content-addressed keys of the object groups of wasm, and of hybn0, wasmopt, hybrid
synth() { # dir: a WORK with the outputs the stages read (small files; the bytes are what the tests change)
  local w=$1 n
  rm -rf "$w"
  mkdir -p "$w/onat/v-tb2d2/native" "$w/onat/v-tb2d2/dll" "$w/wsrc/src" "$w/wsrc/v-tb2d2/src" "$w/w-Oz/web" "$w/hyb"
  echo "realmod table" > "$w/onat/v-tb2d2/native/realmod_table_gen.c"
  echo "dsp module" > "$w/onat/v-tb2d2/dll/dsp_ippv2_x64.2000000.dll"
  for n in 00 01; do echo "fle unit $n" > "$w/wsrc/src/fle-$n.c"; done
  echo "common" > "$w/wsrc/v-tb2d2/src/common.c"
  echo "modules" > "$w/wsrc/v-tb2d2/modules.txt"
  for n in m1-00 m2-00 m3-00 m4-00 m5-00 m6-00 m9-00; do echo "unit $n" > "$w/wsrc/v-tb2d2/src/$n.c"; done
  echo "linked wasm" > "$w/w-Oz/web/flengine6.wasm"
  echo "C part" > "$w/w-Oz/web/flengine6.c.opt.wasm"
  echo "glue" > "$w/w-Oz/web/flengine6.js"
  echo "map record" > "$w/w-Oz/web/link.map.sha256"
  echo "link map" > "$w/hyb/link.map"
}
keyw() { WORK=$3 bash "$1/ci/stage-keys.sh" "$2" 2> /dev/null | sed -n 's/^key=//p'; } # repo, stage, WORK -> the key of a single-key stage
wgroups() { WORK=$2 bash "$1/ci/stage-keys.sh" wasm 2> /dev/null; } # repo, WORK -> the k_ and p_ lines of the seven object groups
nchanged() { local g n=0; for g in $GRPS; do [ "$(gk "$1" "$g")" != "$(gk "$2" "$g")" ] && n=$((n + 1)); done; echo "$n"; } # output A, output B -> the number of group keys that differ
gk() { printf '%s\n' "$1" | sed -n "s/^k_$2=//p"; } # the output of wgroups, group -> its key
pathsw() { WORK=$3 bash "$1/ci/stage-keys.sh" "$2" 2> /dev/null | sed -n 's/^paths=//p'; }
GRPS="fle m1 m2 m3 m4 m5 rt"

R="$base/c-repo"; make_repo "$R"; W="$base/c-work"; synth "$W"
G0=$(wgroups "$R" "$W")
check "the wasm stage gives seven group keys (k_fle k_m1 k_m2 k_m3 k_m4 k_m5 k_rt)" "$([ "$(printf '%s\n' "$G0" | grep -c '^k_')" = 7 ] && echo yes || echo no)"
check "each group key has the form fl-wasm-<40 hex digits>" "$(printf '%s\n' "$G0" | grep '^k_' | grep -cE '=fl-wasm-[0-9a-f]{40}$' | grep -q '^7$' && echo yes || echo no)"
KF=$(gk "$G0" fle); KM1=$(gk "$G0" m1); KM2=$(gk "$G0" m2); KM3=$(gk "$G0" m3); KM4=$(gk "$G0" m4); KM5=$(gk "$G0" m5); KRT=$(gk "$G0" rt)
check "the seven keys are different from each other" "$(printf '%s\n' "$KF" "$KM1" "$KM2" "$KM3" "$KM4" "$KM5" "$KRT" | sort -u | wc -l | grep -q '^7$' && echo yes || echo no)"
check "the same bytes give the same keys" "$(same "$G0" "$(wgroups "$R" "$W")")"
synth "$base/c-same"; check "a copy of the same bytes in another directory gives the same keys" "$(same "$G0" "$(wgroups "$R" "$base/c-same")")"

# one changed unit changes its group's key and no other (main's requirement, decision 0395)
change_one() { # label, file under W (relative), group that must change
  local label=$1 rel=$2 g=$3 W2="$base/c-$1" G1
  synth "$W2"; printf 'X' | dd of="$W2/$rel" bs=1 seek=0 conv=notrunc status=none
  G1=$(wgroups "$R" "$W2")
  local only="" other
  for other in $GRPS; do [ "$(gk "$G1" "$other")" != "$(gk "$G0" "$other")" ] && only="$only $other"; done
  check "$label changes the key of group $g only (changed:$only)" "$([ "$only" = " $g" ] && echo yes || echo no)"
}
change_one fle-unit wsrc/src/fle-00.c fle
change_one m1-unit wsrc/v-tb2d2/src/m1-00.c m1
change_one m6-unit wsrc/v-tb2d2/src/m6-00.c m5
change_one common-c wsrc/v-tb2d2/src/common.c rt
change_one realmod onat/v-tb2d2/native/realmod_table_gen.c rt

W2="$base/c-dll"; synth "$W2"; printf 'D' | dd of="$W2/onat/v-tb2d2/dll/dsp_ippv2_x64.2000000.dll" bs=1 seek=0 conv=notrunc status=none
check "one byte of a module DLL changes no wasm group key (wasm does not read the DLLs)" "$([ "$(wgroups "$R" "$W2")" = "$G0" ] && echo yes || echo no)"

R2="$base/c-rt"; make_repo "$R2"; echo "more" >> "$R2/lift/runtime/fle_x.c"; git -C "$R2" add -A
G2=$(wgroups "$R2" "$W")
check "a change of lift/runtime changes all seven group keys ($(nchanged "$G0" "$G2") of 7)" "$([ "$(nchanged "$G0" "$G2")" = 7 ] && echo yes || echo no)"
R3="$base/c-harness"; make_repo "$R3"; echo "more" >> "$R3/lift/harness/hooks/flengine-hook-points.txt"; git -C "$R3" add -A
G3=$(wgroups "$R3" "$W")
check "a change of lift/harness changes all seven group keys ($(nchanged "$G0" "$G3") of 7)" "$([ "$(nchanged "$G0" "$G3")" = 7 ] && echo yes || echo no)"
R4="$base/c-fn"; make_repo "$R4"; edit "$R4" app/scripts/build-engine-bundle.sh 's/^  for s in rt harness units; do$/  for s in rt harness units; do # x/'
G4=$(wgroups "$R4" "$W")
check "a change of stage_wasm changes all seven group keys ($(nchanged "$G0" "$G4") of 7)" "$([ "$(nchanged "$G0" "$G4")" = 7 ] && echo yes || echo no)"
R5="$base/c-bundle"; make_repo "$R5"; edit "$R5" app/scripts/build-engine-bundle.sh 's/^stage_bundle() {$/stage_bundle() { # x/'
G5=$(wgroups "$R5" "$W")
check "a change of stage_bundle (another stage's code) changes no wasm group key" "$([ "$(nchanged "$G0" "$G5")" = 0 ] && [ "$(grep -c "^stage_bundle() { # x" "$R5/app/scripts/build-engine-bundle.sh")" = 1 ] && echo yes || echo no)"
R6="$base/c-rust"; make_repo "$R6"; edit "$R6" ci/versions.env 's/^RUST_TOOLCHAIN=1.99.0$/RUST_TOOLCHAIN=1.99.1/'
G6=$(wgroups "$R6" "$W")
check "a change of the rust version changes all seven group keys ($(nchanged "$G0" "$G6") of 7)" "$([ "$(nchanged "$G0" "$G6")" = 7 ] && echo yes || echo no)"

check "the group outputs: fle is its object directory; m1 its objects; m5 modules 5 to 9; rt its four paths" "$([ "$(printf '%s\n' "$G0" | sed -n 's/^p_fle=//p')" = "w-Oz/obj/fle" ] && [ "$(printf '%s\n' "$G0" | sed -n 's/^p_m1=//p')" = "w-Oz/obj/mod/m1-*.o" ] && [ "$(printf '%s\n' "$G0" | sed -n 's/^p_m5=//p')" = "w-Oz/obj/mod/m[5-9]-*.o" ] && [ "$(printf '%s\n' "$G0" | sed -n 's/^p_rt=//p')" = "w-Oz/obj/rt w-Oz/obj/h-fruity w-Oz/obj/mod/common.o w-Oz/obj/b2d" ] && echo yes || echo no)"

rm -rf "$base/c-miss"; mkdir -p "$base/c-miss"
rc=$(WORK="$base/c-miss" bash "$R/ci/stage-keys.sh" wasm > /dev/null 2>&1; echo $?)
out=$(WORK="$base/c-miss" bash "$R/ci/stage-keys.sh" wasm 2> /dev/null)
check "a missing input is refused (exit 2, no key line)" "$([ "$rc" = 2 ] && [ -z "$out" ] && echo yes || echo no)"
rc=$(env -u WORK bash "$R/ci/stage-keys.sh" wasm > /dev/null 2>&1; echo $?)
check "no WORK is refused (exit 2)" "$([ "$rc" = 2 ] && echo yes || echo no)"

# ---- decision 0395: the content-addressed keys of hybn0, wasmopt and hybrid (the chain of the stages after the object groups)
keyc() { keyw "$1" "$2" "$3"; }
KH=$(keyc "$R" hybn0 "$W"); KO=$(keyc "$R" wasmopt "$W"); KY=$(keyc "$R" hybrid "$W")
check "hybn0, wasmopt and hybrid have their forms" "$(printf '%s\n%s\n%s\n' "$KH" "$KO" "$KY" | grep -cE '^fl-(hybn0|wasmopt|hybrid)-[0-9a-f]{40}$' | grep -q '^3$' && echo yes || echo no)"
check "the same bytes give the same hybrid key" "$(same "$KY" "$(keyc "$R" hybrid "$W")")"

W3="$base/c-dll2"; synth "$W3"; printf 'D' | dd of="$W3/onat/v-tb2d2/dll/dsp_ippv2_x64.2000000.dll" bs=1 seek=0 conv=notrunc status=none
check "one byte of a module DLL changes the hybn0 key" "$(differ "$KH" "$(keyc "$R" hybn0 "$W3")")"
check "the module DLL leaves the wasmopt key" "$(same "$KO" "$(keyc "$R" wasmopt "$W3")")"

W5="$base/c-linked"; synth "$W5"; printf 'L' | dd of="$W5/w-Oz/web/flengine6.wasm" bs=1 seek=0 conv=notrunc status=none
check "one byte of the linked wasm changes the wasmopt key (the chain: weblink -> wasmopt)" "$(differ "$KO" "$(keyc "$R" wasmopt "$W5")")"
check "the linked wasm leaves the hybn0 key" "$(same "$KH" "$(keyc "$R" hybn0 "$W5")")"

W6="$base/c-copt"; synth "$W6"; printf 'X' | dd of="$W6/w-Oz/web/flengine6.c.opt.wasm" bs=1 seek=0 conv=notrunc status=none
check "one byte of the C part (wasmopt output) changes the hybrid key" "$(differ "$KY" "$(keyc "$R" hybrid "$W6")")"
check "the C part leaves the wasmopt key (wasmopt is before it)" "$(same "$KO" "$(keyc "$R" wasmopt "$W6")")"

W7="$base/c-map"; synth "$W7"; printf 'M' | dd of="$W7/hyb/link.map" bs=1 seek=0 conv=notrunc status=none
check "one byte of the link map changes the hybrid key" "$(differ "$KY" "$(keyc "$R" hybrid "$W7")")"

W8="$base/c-glue"; synth "$W8"; echo "glue before the strip" > "$W8/w-Oz/web/flengine6.orig.js"
KG1=$(keyc "$R" hybrid "$W8"); echo "glue after weblink, not used" > "$W8/w-Oz/web/flengine6.js"
check "with flengine6.orig.js present, the glue flengine6.js is not in the hybrid key" "$(same "$KG1" "$(keyc "$R" hybrid "$W8")")"
rm -f "$W8/w-Oz/web/flengine6.orig.js"
check "without flengine6.orig.js, the glue flengine6.js is in the hybrid key" "$(differ "$KY" "$(keyc "$R" hybrid "$W8")")"

R7="$base/c-fn2"; make_repo "$R7"; edit "$R7" app/scripts/build-engine-bundle.sh 's/^FRUITY_SECONDS=36000 /FRUITY_SECONDS=36001 /'
check "a change of the script's constants changes the wasmopt key (the preamble is in every stage key)" "$(differ "$KO" "$(keyc "$R7" wasmopt "$W")")"
R8="$base/c-lift"; make_repo "$R8"; echo "more" >> "$R8/lift/harness/hooks/flengine-hook-points.txt"; git -C "$R8" add -A
check "a change in lift/harness changes the hybn0 key" "$(differ "$KH" "$(keyc "$R8" hybn0 "$W")")"
check "a change in lift/harness leaves the wasmopt key (no harness file is read by wasm-opt)" "$(same "$KO" "$(keyc "$R8" wasmopt "$W")")"

# ---- decision 0395: the bundle key is the whole input hash
FULL1=$(printf 'c%.0s' {1..64}); FULL2=$(printf 'd%.0s' {1..64})
B1=$(bash "$R/ci/stage-keys.sh" bundle "$FULL1" 2> /dev/null | sed -n 's/^key=//p')
check "the bundle key has the form fl-bundle-<40 hex digits> ($B1)" "$(printf '%s\n' "$B1" | grep -qE '^fl-bundle-[0-9a-f]{40}$' && echo yes || echo no)"
check "the same input hash gives the same bundle key" "$(same "$B1" "$(bash "$R/ci/stage-keys.sh" bundle "$FULL1" 2> /dev/null | sed -n 's/^key=//p')")"
check "another input hash changes the bundle key" "$(differ "$B1" "$(bash "$R/ci/stage-keys.sh" bundle "$FULL2" 2> /dev/null | sed -n 's/^key=//p')")"
check "the bundle's output is bundle/<prefix of 8 digits>" "$([ "$(bash "$R/ci/stage-keys.sh" bundle "$FULL1" 2> /dev/null | sed -n 's/^paths=//p')" = "bundle/fl-engine-ci-cccccccc" ] && echo yes || echo no)"
rc=$(bash "$R/ci/stage-keys.sh" bundle short > /dev/null 2>&1; echo $?)
check "a malformed bundle input hash is refused (exit 2)" "$([ "$rc" = 2 ] && echo yes || echo no)"

echo "test-stage-keys: $pass passed, $fail failed"
[ "$fail" -eq 0 ]
