#!/bin/bash
# ci/plan.sh - the cache key of every stage, from the stage's INPUTS only (nothing here needs an output of an earlier stage): file hashes, tool versions, the installers' SHA-256 (ci/artifacts.lock) and the
# build configuration. Same inputs -> same key -> the stage is skipped and its output comes from the cache; this is the Dockerfile layer rule.
#   the build configuration is the text of app/scripts/build-engine-bundle.sh (its constants and the stage functions, hashed below) and the data files it names (lift/harness, docs/investigations/prune1/tables); there is no other input (decision 0243)
#   usage: ci/plan.sh [OUTFILE]      writes NAME=VALUE lines (to OUTFILE, default stdout; the workflow passes $GITHUB_OUTPUT)
set -u -o pipefail
. "$(dirname "$0")/lib/common.sh"
OUT=${1:-/dev/stdout}
# the bar before the stage name is where the cache salt was (decision 0243 removed it): the keys of the stages that are cached stay the same
k() { echo "fl-$1-$(sha_str "|$1|$2" | cut -c1-40)"; }
lock_sha() { awk -F'\t' -v k="$1" '$1==k {print $3}' "$CI_DIR/artifacts.lock"; }
FL_SHA=$(lock_sha installers/flstudio_win64_26.1.7.5653.exe); SERUM_SHA=$(lock_sha installers/Install_Xfer_Serum2_2.1.5.exe); OTT_SHA=$(lock_sha installers/Install_Xfer_OTT_137.exe); VITAL_SHA=$(lock_sha installers/VitalInstaller.exe)
[ -n "$FL_SHA" ] && [ -n "$SERUM_SHA" ] && [ -n "$OTT_SHA" ] && [ -n "$VITAL_SHA" ] || die "ci/artifacts.lock lacks an installer"
# the extraction as the LIFT sees it (the engine DLL and the five bundled DLLs): not the seed list. KEX (below, with the seed list) still decides whether the extract stage runs, because the guest files it writes are the seed's input.
H_EXTRACT_DLL=$(hashpaths ci/lib/extract_fl.py ci/extract-dlls.tsv tools/extract_nsis2.py tools/nsis_script_dump.py lift/harness/seeds/gen_install_tree.py)
H_EXTRACT_TOOLS=$(hashpaths ci/lib/extract_fl.py ci/extract-dlls.tsv tools/extract_nsis2.py tools/nsis_script_dump.py lift/harness/seeds/gen_install_tree.py lift/harness/seeds/vfs-seed-idle1.tsv)
# the files that only the seed stage (app/scripts/build-engine-bundle.sh stage_seed: pack_seed.py + these) and the extractor read: the lift and the C -> wasm stages do not, so a change to them must not be in their keys
SEEDDATA=(':(exclude)lift/harness/seeds/vfs-seed-idle1.tsv' ':(exclude)lift/harness/seeds/reg-seed-nsis.tsv' ':(exclude)lift/harness/seeds/reg-seed-app.tsv' ':(exclude)lift/harness/seeds/authenticode-claims.tsv' ':(exclude)lift/harness/seeds/os-fonts' ':(exclude)lift/harness/web/final0129/pack_seed.py')
H_VST3_TOOLS=$(hashpaths ci/lib/extract_vst3.sh ci/extract-vst3.tsv docs/investigations/vst3-hosting/tools/inno67_carve.py)
H_FELIFT=$(hashpaths lift/Cargo.toml lift/Cargo.lock lift/crates)
H_GEN=$(hashpaths lift/harness ':(exclude)lift/harness/web' "${SEEDDATA[@]}" docs/investigations/wasm-final0129/tools/wrap docs/investigations/prune1/tables)
H_WASM=$(hashpaths lift/runtime lift/harness "${SEEDDATA[@]}" docs/investigations/a4-six-module-wasm/tools docs/investigations/wasm-final0129/tools)
H_SEED=$(hashpaths lift/harness/web/final0129/pack_seed.py lift/harness/seeds)
H_BUNDLE=$(hashpaths app/scripts/pack-assets.mjs)
H_APP=$(hashpaths app ':(exclude)app/scripts/build-engine-bundle.sh' ci/lib/wrangler_config.py)
SP_LIFT=$( { script_preamble; script_part capped stage_gen tag_dir stage_resplit; } | sha256sum | cut -c1-40)
SP_WASM=$( { script_preamble; script_part capped tag_dir wasm_env b2d_adapter stage_wasm stage_weblink stage_wasmopt hyb_lists hyb_few hyb_modargs hyb_flags hyb_strip stage_hybn0 stage_hybrid hyb_guard; } | sha256sum | cut -c1-40)
SP_SEED=$( { script_preamble; script_part tag_dir stage_seed; } | sha256sum | cut -c1-40)
SP_BUNDLE=$( { script_preamble; script_part stage_bundle; } | sha256sum | cut -c1-40)
TOOLS="emsdk=$EMSDK_VERSION@$EMSDK_REV rust=$RUST_TOOLCHAIN python=$PYTHON_VERSION node=$NODE_VERSION os=$(echo "${CI_RUNNER_OS:-ubuntu}" | tr -c 'a-z0-9\n' '-')"
KEX=$(k extract "$FL_SHA|$H_EXTRACT_TOOLS")
KV3=$(k vst3 "$SERUM_SHA|$OTT_SHA|$VITAL_SHA|$H_VST3_TOOLS")
KFE=$(k felift "$H_FELIFT|rust=$RUST_TOOLCHAIN|bins=fe-pe,fe-lift,fe-wasmgen")
KEXD=$(k extract-dll "$FL_SHA|$H_EXTRACT_DLL")
KLIFT=$(k lift "$KEXD|$KFE|$H_GEN|$SP_LIFT")
KEMSDK=$(k tool-emsdk "$EMSDK_VERSION|$EMSDK_REV|${CI_RUNNER_OS:-ubuntu}")
KB2D=$(k b2d "$(hashpaths lift/harness/b2d1/build_upstream.sh lift/harness/b2d1/b2d3-compop.patch lift/harness/b2d1/b2d3-fetch-fma.patch)|emsdk=$EMSDK_VERSION@$EMSDK_REV|opt=$B2D_OPT|cflags=$B2D_CFLAGS|patch=$B2D_PATCH")
KRT=$(k rt "$KLIFT|$H_WASM|$SP_WASM|$TOOLS")
KLINK=$(k link "$KLIFT|$KB2D|$H_WASM|$SP_WASM|$TOOLS")
KSEED=$(k seed "$KEX|$KLIFT|$H_SEED|$SP_SEED")
KBUNDLE=$(k bundle "$KLINK|$KSEED|$H_BUNDLE|$SP_BUNDLE")
KAPP=$(k app "$H_APP|node=$NODE_VERSION")
{
  echo "k_extract=$KEX"; echo "k_vst3=$KV3"; echo "k_felift=$KFE"; echo "k_lift=$KLIFT"; echo "k_b2d=$KB2D"; echo "k_rt=$KRT"
  echo "k_link=$KLINK"; echo "k_seed=$KSEED"; echo "k_bundle=$KBUNDLE"; echo "k_app=$KAPP"
  echo "k_emsdk=$KEMSDK"; echo "prefix=fl-engine-ci-${KBUNDLE: -8}"
  echo "h_gen=$H_GEN"; echo "h_wasm=$H_WASM"; echo "sp_wasm=$SP_WASM"; echo "tools=${TOOLS// /;}"
} > "$OUT"
# the inputs, for the log and the summary
{ echo "--- tools: $TOOLS"; } >&2
