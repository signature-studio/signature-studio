# shellcheck shell=bash
# ci/lib/common.sh - sourced by ci/*.sh. No credential is ever printed: R2 keys are only passed through the environment.
set -u -o pipefail
CI_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
REPO=$(cd "$CI_DIR/.." && pwd)
# shellcheck disable=SC1091
. "$CI_DIR/versions.env"
CI_WORK=${CI_WORK:-$REPO/ci-work}
HOST_ROOT=${HOST_ROOT:-$CI_WORK/host}        # the product-derived inputs (binaries/win64, lift/out-*): the extract stage's output
ENGINE_WORK=${ENGINE_WORK:-$CI_WORK/engine}  # WORK of app/scripts/build-engine-bundle.sh
VARIANT=Oz
PREFIX=${PREFIX:-fl-$VARIANT}
export CI_WORK HOST_ROOT ENGINE_WORK VARIANT
log() { echo "ci: $*" >&2; }
die() { echo "ci: ERROR: $*" >&2; exit 2; }
summary() { [ -n "${GITHUB_STEP_SUMMARY:-}" ] && printf '%s\n' "$*" >> "$GITHUB_STEP_SUMMARY"; return 0; }
set_output() { [ -n "${GITHUB_OUTPUT:-}" ] && printf '%s=%s\n' "$1" "$2" >> "$GITHUB_OUTPUT"; return 0; }
# sha256 of the tracked files under the given paths (git blob ids + paths, so it is cheap and does not depend on the checkout's timestamps); a tree without .git (git archive) is hashed by content
hashpaths() {
  if git -C "$REPO" rev-parse --git-dir >/dev/null 2>&1; then
    ( cd "$REPO" && git ls-files -s -- "$@" | LC_ALL=C sort -k4 | sha256sum | cut -d' ' -f1 )
  else
    ( cd "$REPO" && find "$@" -type f -print0 | LC_ALL=C sort -z | xargs -0 sha256sum | sha256sum | cut -d' ' -f1 )
  fi
}
# the text of one function of app/scripts/build-engine-bundle.sh (a stage's code), so that a change to another stage does not invalidate this one
script_part() {
  local f=$REPO/app/scripts/build-engine-bundle.sh n
  for n in "$@"; do awk -v n="$n" '$0 ~ "^"n"\\(\\) \\{" {p=1} p {print} p && /^}/ {p=0}' "$f"; done
}
script_preamble() { awk '/^(stage_gen|capped|tag_dir|stage_resplit|wasm_env|b2d_adapter|stage_wasm|stage_weblink|stage_wasmopt|stage_seed|stage_bundle)\(\) \{/ {exit} {print}' "$REPO/app/scripts/build-engine-bundle.sh"; }
sha_str() { printf '%s' "$*" | sha256sum | cut -d' ' -f1; }
# shellcheck disable=SC2034   # read by plan.sh (the key) and stage-wasm.sh (the build), which source this file
B2D_PATCH=lift/harness/b2d1/b2d3-compop.patch   # decision 0195: the patch the shipping Blend2D is built with
# shellcheck disable=SC2034
B2D_OPT=-O2
# shellcheck disable=SC2034   # read by plan.sh (it is in the key: a cached library built without atomics must not be restored) and stage-wasm.sh (the build)
B2D_CFLAGS="-matomics -mbulk-memory"   # the shared-memory (real thread) link refuses an object without them
B2D_DIR=${B2D_DIR:-$CI_WORK/b2d}
STAGES_TSV=${STAGES_TSV:-$CI_WORK/stages.tsv}
mkdir -p "$CI_WORK"
# stage_row STAGE KEY STATUS [SECONDS]: one row of the table that the workflow prints at the end
stage_row() { printf '%s\t%s\t%s\t%s\n' "$1" "$2" "$3" "${4:-}" >> "$STAGES_TSV"; }
jobs_default() {   # compile jobs: one per CPU, at most one per 3 GB of RAM (measured 2026-11-04: an emcc -Oz unit peaks at 1.43 GB over 130 units; build-engine-bundle.sh's memory guard drops to 1 below 4 GB available)
  local n m; n=$(nproc); m=$(awk '/MemTotal/ {print int($2/1048576/3)}' /proc/meminfo); [ "$m" -lt 1 ] && m=1; [ "$n" -gt "$m" ] && n=$m; echo "$n"
}
# run_bb STAGE...: app/scripts/build-engine-bundle.sh of THIS checkout on the extracted inputs.
# Its output goes to the step as before and also to $CI_WORK/run_bb-<stage>.log; when it fails, the last 80 lines are printed again as a group and as an annotation, so that the cause is in the step's log and
# its summary (the output of a long stage scrolls past, and the line "ci: ERROR: <stage> failed" that follows says nothing about why).
run_bb() {
  local rlog=$CI_WORK/run_bb-${1:-all}.log rrc
  run_bb_inner "$@" 2>&1 | tee "$rlog"
  rrc=${PIPESTATUS[0]}
  if [ "$rrc" != 0 ]; then
    echo "::group::build-engine-bundle.sh $* failed (rc=$rrc): the last 80 lines of its output"
    tail -80 "$rlog"
    echo "::endgroup::"
    echo "::error title=build-engine-bundle.sh $* failed::rc=$rrc; last line: $(tail -1 "$rlog" | tr -d '\r' | cut -c1-300)"
    summary "### build-engine-bundle.sh $* failed (rc=$rrc)"; summary ""; summary '```'; summary "$(tail -40 "$rlog" | cut -c1-400)"; summary '```'
  fi
  return "$rrc"
}
run_bb_inner() {
  ( export SRCTREE=$REPO HOST_ROOT WORK=$ENGINE_WORK VARIANT PREFIX JOBS=${CI_JOBS:-$(jobs_default)} EMSDK_VERSION
    export B2D_UPSTREAM_LIB=${B2D_UPSTREAM_LIB:-$B2D_DIR/build-wasm/libblend2d.a} B2D_UPSTREAM_SRC=${B2D_UPSTREAM_SRC:-$B2D_DIR/src/src}
    export EM_CACHE=${EM_CACHE:-$ENGINE_WORK/emcache}
    export PATH=$EMSDK_DIR:$EMSDK_DIR/upstream/emscripten:$HOME/.cargo/bin:$PATH
    bash "$REPO/app/scripts/build-engine-bundle.sh" "$@" )
}
