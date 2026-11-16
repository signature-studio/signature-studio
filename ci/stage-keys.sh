#!/usr/bin/env bash
# shellcheck disable=SC2031   # WORK is only read here (common.sh sets WORK in its own subshell)
# ci/stage-keys.sh STAGE [ARG] - prints key=<name> and paths=<outputs> (for GITHUB_OUTPUT) for one cached stage of app/scripts/build-engine-bundle.sh (decisions 0391, 0395, 0395).
#   wasm      seven object groups (decision 0395): k_<group>=<key> and p_<group>=<outputs> for fle, m1, m2, m3, m4, m5 (modules 5 to 9), rt (runtime, harness, common.c,
#             realmod table, B2D adapter). Env WORK. Each key is content-addressed: the SHA-256 of the bytes of the group's own units (the generated C of the wsrc tree, before the
#             hybrid strip), the git blob ids of lift/runtime and lift/harness, the text of the wasm stage's functions and the tool versions. A changed unit changes its group's key only.
#   hybn0, wasmopt, hybrid   content-addressed: env WORK. The key is the SHA-256 (first 40 digits) of the bytes of the files the stage reads from the earlier stages, the git blob
#             ids of the sources it reads, the text of its functions and the tool versions. Same bytes, same key; a changed byte, source, function or tool gives another key.
#             The key reads the outputs of the stages before it, so the workflow calls it after them.
#   bundle    ARG = the whole input hash of the workflow's prefix (64 hex digits, deploy-engine.yml). Every input of the bundle is in that list.
#   gen       (decision 0391; withdrawn for the workflow by decision 0395) ARG = the extraction layer's key (64 hex digits).
# paths: the stage's output files or directories, relative to WORK (ci/stage-cache.sh packs them; its glob patterns are expanded in WORK).
set -u -o pipefail
# shellcheck source=ci/lib/common.sh
. "$(dirname "$0")/lib/common.sh"
stage=${1:-}; arg=${2:-}
TOOLS="emsdk=$EMSDK_VERSION@$EMSDK_REV rust=$RUST_TOOLCHAIN python=$PYTHON_VERSION node=$NODE_VERSION"

case "$stage" in
  gen)
    [[ $arg =~ ^[0-9a-f]{64}$ ]] || die "the upstream key must be the extraction key (64 hex digits): ci/stage-keys.sh gen UPSTREAM-KEY"
    gpaths=(lift/harness lift/crates lift/Cargo.toml lift/Cargo.lock lift/.cargo lift/data)
    code=$( { script_preamble; script_part capped stage_gen tag_dir; } )
    k=$(sha_str "stage-cache v1|gen|$arg|rust=$RUST_TOOLCHAIN python=$PYTHON_VERSION|$(hashpaths "${gpaths[@]}")|$(sha_str "$code")" | cut -c1-40)
    echo "stage gen: key fl-gen-$k (upstream ${arg:0:12}...)" >&2
    echo "key=fl-gen-$k"
    echo "paths=onat"
    exit 0
    ;;
  bundle)
    [[ $arg =~ ^[0-9a-f]{64}$ ]] || die "the bundle key needs the whole input hash (64 hex digits): ci/stage-keys.sh bundle FULL"
    k=$(sha_str "stage-cache v2|bundle|$arg|$({ script_preamble; script_part capped stage_bundle; } | sha256sum | cut -d' ' -f1)|$TOOLS" | cut -c1-40)
    echo "stage bundle: key fl-bundle-$k (input hash ${arg:0:12}...)" >&2
    echo "key=fl-bundle-$k"
    echo "paths=bundle/fl-engine-ci-${arg:0:8}"
    exit 0
    ;;
  wasm|hybn0|wasmopt|hybrid) ;;
  *) die "unknown stage '$stage' (known: wasm hybn0 wasmopt hybrid bundle gen)" ;;
esac

[ -n "${WORK:-}" ] || die "WORK is not set (the build's WORK directory)"
ONAT=$WORK/onat; WEB=$WORK/w-$VARIANT/web
need() { [ -e "$1" ] || die "$1 is missing: the stage before this one has not run (WORK=$WORK)"; }
tag_of() {
  local d; d=$(find "$ONAT" -maxdepth 1 -name "v-*" -type d 2>/dev/null | LC_ALL=C sort | head -n 1)
  [ -n "$d" ] || die "no $ONAT/v-*: the gen stage has not run"
  basename "$d" | sed 's/^v-//'
}
tree_sha() { need "$1"; ( cd "$1" && find . -type f -print0 | LC_ALL=C sort -z | xargs -0 sha256sum | sha256sum | cut -d' ' -f1 ); }
file_sha() { need "$1"; sha256sum "$1" | cut -d' ' -f1; }
# the SHA-256 of the files that a glob names in a directory (the names and the bytes; no match is an error)
# shellcheck disable=SC2206   # the pattern is a glob on purpose (the names of the units)
glob_sha() { ( export LC_ALL=C; cd "$1" && shopt -s nullglob && f=($2) && [ "${#f[@]}" -gt 0 ] && sha256sum "${f[@]}" | sha256sum | cut -d' ' -f1 ); }
script_sha() { { script_preamble; script_part "$@"; } | sha256sum | cut -d' ' -f1; }
ver() { "$@" --version 2>/dev/null | head -n 1 | tr -d '\r'; }

case "$stage" in
  wasm)   # decision 0395: the object groups of stage_wasm. build_nmod_wasm.sh names the units (src/fle-NN.c, v-TAG/src/mK-NN.c, common.c) and the objects; a group's objects are restored by its key
    tag=$(tag_of)
    SRC=$WORK/wsrc/v-$tag/src
    need "$WORK/wsrc/src"; need "$SRC/common.c"; need "$ONAT/v-$tag/native/realmod_table_gen.c"
    raw=$(hashpaths lift/runtime lift/harness)
    sp=$(script_sha capped tag_dir wasm_env b2d_adapter stage_wasm hyb_strip hyb_lists)
    for g in fle m1 m2 m3 m4 m5 rt; do
      case "$g" in
        fle) up=$(glob_sha "$WORK/wsrc/src" 'fle-[0-9][0-9].c') || die "no fle units in $WORK/wsrc/src" ;;
        m1|m2|m3|m4) up=$(glob_sha "$SRC" "$g-[0-9][0-9].c") || die "no $g units in $SRC" ;;
        m5) up=$(glob_sha "$SRC" 'm[5-9]-[0-9][0-9].c') || die "no m5-m9 units in $SRC" ;;
        rt) up="$(file_sha "$SRC/common.c")|$(file_sha "$ONAT/v-$tag/native/realmod_table_gen.c")" ;;
      esac
      case "$g" in
        fle) p="w-$VARIANT/obj/fle" ;;
        m1|m2|m3|m4) p="w-$VARIANT/obj/mod/$g-*.o" ;;
        m5) p="w-$VARIANT/obj/mod/m[5-9]-*.o" ;;
        rt) p="w-$VARIANT/obj/rt w-$VARIANT/obj/h-fruity w-$VARIANT/obj/mod/common.o w-$VARIANT/obj/b2d" ;;
      esac
      k=$(sha_str "stage-cache v2|wasm|group=$g|$up|$raw|$sp|$TOOLS" | cut -c1-40)
      echo "group $g: key fl-wasm-$k" >&2
      echo "k_$g=fl-wasm-$k"
      echo "p_$g=$p"
    done
    exit 0
    ;;
  hybn0)  # stage_hybn0: fe-wasmgen over the lists, with the module DLLs of the gen stage
    tag=$(tag_of)
    need "$ONAT/v-$tag/dll"
    up=$(tree_sha "$ONAT/v-$tag/dll")
    raw=$(hashpaths lift/harness lift/crates lift/Cargo.toml lift/Cargo.lock lift/.cargo)
    sp=$(script_sha capped tag_dir hyb_lists hyb_few hyb_flags hyb_modargs stage_hybn0)
    paths="hyb/n0-*.wasm hyb/n0-*.tsv hyb/exports.rsp hyb/c-imports-used.txt hyb/c-imports-used.u.txt"
    ;;
  wasmopt)  # stage_wasmopt: wasm-opt -Oz over the linked C part (the weblink output, read as bytes)
    need "$WEB/flengine6.wasm"
    up="$(file_sha "$WEB/flengine6.wasm")|$(ver "${WASM_OPT_BIN:-/opt/emsdk/upstream/bin/wasm-opt}")"
    raw=""
    sp=$(script_sha capped stage_wasmopt)
    paths="w-$VARIANT/web/flengine6.c.opt.wasm"
    ;;
  hybrid)  # stage_hybrid: the direct modules, the merge, and the glue before its strip (flengine6.orig.js, or flengine6.js while no strip has run)
    tag=$(tag_of)
    glue=$WEB/flengine6.orig.js; [ -e "$glue" ] || glue=$WEB/flengine6.js
    need "$WEB/flengine6.c.opt.wasm"; need "$WORK/hyb/link.map"; need "$WEB/link.map.sha256"; need "$glue"; need "$ONAT/v-$tag/dll"
    up="$(file_sha "$WEB/flengine6.c.opt.wasm")|$(file_sha "$WORK/hyb/link.map")|$(file_sha "$WEB/link.map.sha256")|$(file_sha "$glue")|$(tree_sha "$ONAT/v-$tag/dll")|$(ver "${WASM_MERGE_BIN:-/opt/emsdk/upstream/bin/wasm-merge}")|$(ver "${WASM_OPT_BIN:-/opt/emsdk/upstream/bin/wasm-opt}")"
    raw=$(hashpaths lift/harness lift/crates lift/Cargo.toml lift/Cargo.lock lift/.cargo)
    sp=$(script_sha capped tag_dir hyb_lists hyb_few hyb_flags hyb_modargs hyb_guard stage_hybrid)
    paths="w-$VARIANT/web/flengine6.opt.wasm w-$VARIANT/web/flengine6.js"
    ;;
esac
k=$(sha_str "stage-cache v2|$stage|$up|$raw|$sp|$TOOLS" | cut -c1-40)
echo "stage $stage: key fl-$stage-$k" >&2
echo "key=fl-$stage-$k"
echo "paths=$paths"
