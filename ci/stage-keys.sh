#!/usr/bin/env bash
# ci/stage-keys.sh STAGE UPSTREAM-KEY - prints key=<name> (for GITHUB_OUTPUT): the cache key of one stage of app/scripts/build-engine-bundle.sh (decision 0391).
#   gen   the generated C (stage_gen). UPSTREAM-KEY = the extraction layer's key (ci/extract-cache.sh key, 64 hex digits): the installer's row and the files the extraction reads.
# The key is the SHA-256 (first 40 digits) of: the cache version, the stage's name, the upstream key, the tool versions the stage runs with (ci/versions.env: rust, python),
# the git blob ids of the files the stage reads (paths below), and the text of the stage's code (the script's preamble = its constants, and the functions the stage calls).
# Same inputs, same key. A change of any of them, or of the upstream key, changes the key. The key method is that of the old ci/plan.sh (fl-<stage>-<hash>); the inputs are the current ones.
#   gen reads: lift/harness (gen-direct.sh, two/gen_two.sh and the hooks, seeds and tables they name; web/ and hyb/ are in it too, so a change there only makes a miss),
#   lift/crates (fe-pe, fe-lift and the realmod tools that gen_two.sh runs), lift/Cargo.toml, lift/Cargo.lock, lift/.cargo (the cargo build of those), lift/data (prune-SM.tsv).
#   Not lift/runtime: no code path of the gen stage reads it (decision 0391 has the grep).
set -u -o pipefail
# shellcheck source=ci/lib/common.sh
. "$(dirname "$0")/lib/common.sh"
stage=${1:-}; up=${2:-}
[[ $up =~ ^[0-9a-f]{64}$ ]] || die "the upstream key must be the extraction key (64 hex digits): ci/stage-keys.sh STAGE UPSTREAM-KEY"
case "$stage" in
  gen)
    paths=(lift/harness lift/crates lift/Cargo.toml lift/Cargo.lock lift/.cargo lift/data)
    code=$( { script_preamble; script_part capped stage_gen tag_dir; } )
    ;;
  *) die "unknown stage '$stage' (known: gen)" ;;
esac
k=$(sha_str "stage-cache v1|$stage|$up|rust=$RUST_TOOLCHAIN python=$PYTHON_VERSION|$(hashpaths "${paths[@]}")|$(sha_str "$code")" | cut -c1-40)
echo "stage $stage: key fl-$stage-$k (upstream ${up:0:12}...)" >&2
echo "key=fl-$stage-$k"
