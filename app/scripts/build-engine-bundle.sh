#!/bin/bash
# build-engine-bundle.sh: builds the engine BUNDLE (engine.kind "emscripten-fruity-v1") from the tree this script is in. ONE configuration (decision 0243: no preset, no option):
# the lifted FLEngine and the seven modules it loads (dsp_ippv2, Blend2D, QuickFontCache, FreeType, ILTools, the sample decoder oggio and the time-stretch engine Elastique), the runtime, the hybrid (the functions of
# lift/harness/seeds/hyb-U-<module>.txt lowered straight to wasm by fe-wasmgen), upstream Blend2D for the lifted Blend2D context, wasm-opt -Oz, the register file in the thread-local block fle_ctx, one shared memory.
# The procedure is that of decision 0134 with every path derived from this tree; the one thing that is added to the link line is -sEXPORT_ES6=1 (the glue is an ES module that the engine's MODULE worker can import()).
#
#   [WORK=DIR] [HOST_ROOT=DIR] [JOBS=n] [PREFIX=name] [BUCKET=name] [BROTLI_QUALITY=n] bash app/scripts/build-engine-bundle.sh [STAGE...]
#
#   stages (default: all, in this order; every stage skips what its output already has, so a stage can be repeated):
#     gen       fe-pe/fe-lift (cargo --offline), then lift/harness/b2d2/gen-direct.sh: the generated C of FLEngine and of the seven modules (the Blend2D context is replaced by upstream Blend2D, decision 0160; the dsp
#               hot function by a host implementation, decision 0162; the pruned IPP variants, decision 0168).
#     resplit   the generated C cut into 10 MB translation units (harness/two/split_bytes.py), the register file moved to the fixed addresses of the runtime (harness/two/lowregs.py)
#     wasm      build_nmod_wasm.sh rt harness units with the flags of the configuration (the heavy stage); the bodies of the hybrid's functions are taken out of the C units first
#     hybn0     fe-wasmgen with dummy addresses over the lists: which C functions the direct wasm calls -> $WORK/hyb/exports.rsp (the weblink stage exports them)
#     weblink   harness/web/fruity/link-web.sh: flengine6.{js,wasm} for the browser (ES module glue, no file system)
#     wasmopt   wasm-opt -Oz over flengine6.wasm -> flengine6.c.opt.wasm (the C part)
#     hybrid    fe-wasmgen over the lists with the real addresses (from the link map), wasm-opt -Oz of each direct module, wasm-merge into the C part -> flengine6.opt.wasm, and the glue without the stubs of the functions that the direct code defines
#     seed      pack_seed.py: the seed pack (FLSEEDP1, the guest's host files + registry seeds + claims), and the DLL directory (8 files)
#     bundle    scripts/pack-assets.mjs: engine.<sha>.wasm(.br), glue, seed pack, DLL pack, manifest.json, SHA256SUMS, upload-commands.txt  -> $WORK/bundle/$PREFIX
#   parameters (places and numbers; none of them changes what is built): WORK (default $SRCTREE/lift/out-app-engine), HOST_ROOT (the extracted inputs: binaries/win64/FLEngine_x64.dll and lift/out-*), PREFIX (bundle key prefix), BUCKET (the R2 bucket
#             name in the printed upload commands), JOBS, MEMGUARD_MIN_MB, BROTLI_QUALITY (default 11; a lower number only makes the encoding faster), WASM_OPT_BIN, WASM_MERGE_BIN, EM_CACHE,
#             B2D_UPSTREAM_LIB / B2D_UPSTREAM_SRC (upstream Blend2D, built by lift/harness/b2d1/build_upstream.sh with -matomics -mbulk-memory; required).
#   It never uploads anything. The upload commands are written to $WORK/bundle/$PREFIX/upload-commands.txt and printed.
set -u -o pipefail
export PYTHONDONTWRITEBYTECODE=1
HERE=$(cd "$(dirname "$0")" && pwd)
APP=$(cd "$HERE/.." && pwd)
SRCTREE=${SRCTREE:-$(cd "$APP/.." && pwd)}
HOST_ROOT=${HOST_ROOT:-$SRCTREE}
WORK=${WORK:-$SRCTREE/lift/out-app-engine}
JOBS=${JOBS:-3}
MEMGUARD_MIN_MB=${MEMGUARD_MIN_MB:-4096}
BROTLI_QUALITY=${BROTLI_QUALITY:-11}
WASM_OPT_BIN=${WASM_OPT_BIN:-/opt/emsdk/upstream/bin/wasm-opt}
VARIANT=Oz
PREFIX=${PREFIX:-fl-$VARIANT}
die() { echo "build-engine-bundle: $*" >&2; exit 2; }
# ---- the configuration (decision 0243): constants, not variables of the caller. Each line is what the shipped build had: the preset chain F2xH7 -> F2xH5 -> F2xH4 -> F2xH3 -> F2xH2 -> F2x (decisions 0176, 0189, 0208, 0218, 0225, 0230, 0235)
# with the hybrid of decision 0239 (H9opt) and the two modules of decision 0230 (oggio, Elastique).
FRUITY_SECONDS=36000   # the engine harness stops the guest after this many wall-clock seconds (fruity.c: FLE_FRUITY_SECONDS, at most 36000 since decision 0136)
HINT_FIRST_PIXEL_S=53; HINT_STOP_S=59          # the manifest's hints of decision 0137 (Chrome 156, shared machine)
LOWREGS_BASE=0x1000                            # R at 0x1000, FL at +0x100, X at +0x200 (decision 0197)
FLE_SEH_HANDLERS=0x112c0                       # decision 0132: the SEH of FLEngine
EXTRA_IMPORTS=$SRCTREE/lift/harness/seeds/ship6-imports.txt   # decision 0230: the import rows (layers 1-3 of decision 0171, 0178, ... 0229) + decision 0302: dwmapi!DwmGetCompositionTimingInfo (ship6 = ship5 + dwmti)
PRUNE_LIST=$SRCTREE/lift/data/prune-SM.tsv   # decision 0168: the S+M level, for dsp_ippv2 only (gen-direct.sh)
AI_UNITS_FILE=$SRCTREE/lift/harness/seeds/ship-hot-units-F2.tsv      # decision 0151: the start-up-heavy units compiled with -DFLE_ALWAYS_INLINE
AI_UNITS=$(awk -F'\t' '/^#/ {next} {print $1}' "$AI_UNITS_FILE" | tr '\n' ' ')
EXTRA_MODULES="oggio elastique"; NEXTRA=2     # decision 0230: modules 6 and 7 of the lift (the sample decoder and the time-stretch engine)
HYB_MODS="fle dsp b2d qfc ft ilt oggio elastique"   # the modules of the hybrid lists lift/harness/seeds/hyb-U-<m>.txt (decision 0239)
HYB=$WORK/hyb
B2D_UPSTREAM_LIB=${B2D_UPSTREAM_LIB:-}
B2D_UPSTREAM_SRC=${B2D_UPSTREAM_SRC:-}
[ -r "$EXTRA_IMPORTS" ] && [ -r "$AI_UNITS_FILE" ] && [ -r "$PRUNE_LIST" ] || die "a data file of the configuration is missing under $SRCTREE"

L=$SRCTREE/lift
H=$L/harness
HL=$HOST_ROOT/lift
FLE=$HOST_ROOT/binaries/win64/FLEngine_x64.dll
[ -r "$H/b2d2/gen-direct.sh" ] || die "no lift/harness/b2d2/gen-direct.sh under SRCTREE=$SRCTREE"
[ -r "$FLE" ] || die "FLEngine_x64.dll not found at $FLE (HOST_ROOT=$HOST_ROOT: binaries/ is not in git; set HOST_ROOT to the checkout that has it)"
EXPECT_FLE_DLL_SHA=b2c64da40921cf262c15c5b25c2b0422e4d1851e953c87e0bc38ed2c19f9496d
[ "$(sha256sum "$FLE" | cut -d' ' -f1)" = "$EXPECT_FLE_DLL_SHA" ] || die "FLEngine_x64.dll has another SHA-256 than decision 0129's ($EXPECT_FLE_DLL_SHA)"

mkdir -p "$WORK"/{logs,tmp,in}
LOGS=$WORK/logs
ONAT=$WORK/onat                 # gen-direct.sh OUT (generated C, rebased DLLs)
WSRC=$WORK/wsrc                 # the same C cut by size (resplit)
OBJ=$WORK/w-$VARIANT/obj
WEB=$WORK/w-$VARIANT/web
BUNDLE=$WORK/bundle/$PREFIX
export TMPDIR=$WORK/tmp EM_CACHE=${EM_CACHE:-$WORK/emcache}
export PATH=/opt/emsdk:/opt/emsdk/upstream/emscripten:$HOME/.local/bin:$PATH

# ---- run a command in a systemd user scope with a hard memory cap and no swap; the cap, not the command, decides what is killed. Nothing waits for memory: JOBS drops to 1 below the minimum. ----
capped() {   # capped NAME cmd...   (stdout/stderr -> $LOGS/NAME.out/.err; the wall time -> $LOGS/stages.log). Nothing waits for memory: JOBS drops to 1 below the minimum.
  local name=$1; shift
  local avail; avail=$(awk '/MemAvailable/ {print int($2/1024)}' /proc/meminfo)
  echo "memguard: $(date -u +%H:%M:%SZ) stage=$name MemAvailable=${avail}MB JOBS=$JOBS" | tee -a "$LOGS/memguard.log" >&2
  if [ "$avail" -lt "$MEMGUARD_MIN_MB" ] && [ "$JOBS" -gt 1 ]; then
    echo "memguard: MemAvailable ${avail}MB < ${MEMGUARD_MIN_MB}MB: JOBS ${JOBS} -> 1 (not waiting)" | tee -a "$LOGS/memguard.log" >&2
    JOBS=1
  fi
  export JOBS
  local t0=$SECONDS rc
  "$@" > "$LOGS/$name.out" 2> "$LOGS/$name.err"
  rc=$?
  echo "stage-command $name rc=$rc wall=$((SECONDS - t0))s" | tee -a "$LOGS/stages.log" >&2
  return $rc
}

stage_gen() {
  [ -d "$HL/out-ext" ] || die "$HL/out-ext not found (HOST_ROOT=$HOST_ROOT)"
  if [ ! -x "$WORK/cargo-target/release/fe-lift" ] || [ ! -x "$WORK/cargo-target/release/fe-pe" ]; then
    ( cd "$L" && CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_TARGET_DIR="$WORK/cargo-target" cargo build --release --offline -p fe-pe -p fe-lift ) > "$LOGS/cargo-build.txt" 2>&1 || die "cargo build failed (see $LOGS/cargo-build.txt)"
  fi
  # lift/harness/b2d2/gen-direct.sh: gen_two.sh rebase gen split merge over FLEngine and the seven modules, and the realmod table. The lifter applies the dead-flags and the outlining wherever the module allows (decision 0243).
  export FLE FLE_HOOKS=$H/hooks/flengine-hook-points.txt FLE_SEEDS=$H/seeds/flengine-seeds-thunks.txt FLE_SEH_HANDLERS FLE_EXTRA_IMPORTS=$EXTRA_IMPORTS PRUNE_LIST
  export OUT=$ONAT FEBIN=$WORK/cargo-target/release LIFT=$L H HL TAG=tb2d2
  capped gen bash "$H/b2d2/gen-direct.sh" || { tail -n 20 "$LOGS/gen.out" "$LOGS/gen.err"; die "gen failed"; }
  sha256sum "$ONAT/gen/flengine.c" "$ONAT"/v-*/gen/m*.c | tee "$LOGS/gen-summary.txt"
}

tag_dir() { ls -d "$ONAT"/v-* 2>/dev/null | head -1; }

stage_resplit() {
  local TAGDIR TAG
  TAGDIR=$(tag_dir); [ -n "$TAGDIR" ] || die "no $ONAT/v-*: run the gen stage first"
  TAG=$(basename "$TAGDIR" | sed 's/^v-//')
  if [ -e "$WSRC/src" ]; then echo "resplit: $WSRC/src exists, skipped"; return 0; fi
  mkdir -p "$WSRC/src" "$WSRC/v-$TAG/src" "$WSRC/tmp"
  python3 "$H/two/split_bytes.py" "$ONAT/gen/flengine.c" "$WSRC/src/fle" 10000000 || die "split of flengine.c failed"
  local k
  for k in $(seq 1 $((5 + NEXTRA))); do python3 "$H/two/split_bytes.py" "$TAGDIR/gen/m$k.c" "$WSRC/v-$TAG/src/m$k" 10000000 || die "split of m$k.c failed"; done
  cp "$TAGDIR/src/common.c" "$WSRC/v-$TAG/src/common.c"
  cp "$TAGDIR/modules.txt" "$WSRC/v-$TAG/modules.txt"
  echo "TAG=$TAG; $(ls "$WSRC/src" | wc -l) fle units, $(ls "$WSRC/v-$TAG/src" | wc -l) module units" | tee "$LOGS/resplit.txt"
  # decision 0161: R, FL and X of the generated C are defined by the runtime (the thread-local block fle_ctx, decision 0234); the definitions in the units are guarded
  python3 "$H/two/lowregs.py" "$WSRC"/src/fle-*.c "$WSRC/v-$TAG"/src/*.c | tail -2 | tee -a "$LOGS/resplit.txt"
  sha256sum "$ONAT/gen/flengine.c" "$TAGDIR"/gen/m*.c "$TAGDIR/src/common.c" > "$LOGS/generated-c-sha256.txt"
  du -sm "$ONAT/gen" "$TAGDIR/gen" 2>/dev/null | tee -a "$LOGS/resplit.txt"
}

wasm_env() {
  local TAGDIR; TAGDIR=$(tag_dir)
  [ -n "$TAGDIR" ] || die "no $ONAT/v-*: run the gen stage first"
  TAG=$(basename "$TAGDIR" | sed 's/^v-//')
  export OUT=$WSRC TAG
  export REALMOD_C=$ONAT/v-$TAG/native/realmod_table_gen.c
  [ -r "$REALMOD_C" ] || die "no $REALMOD_C (the gen stage makes it)"
  export OBJ AI_UNITS
  # every object: WebAssembly SIMD (fle_sse.h: one wasm instruction per packed x86 instruction, decision 0225) and atomics + bulk memory, so that the link can use a shared memory (decision 0234); the register file location is the runtime's (fle_runtime.h)
  export EMCC_CFLAGS="-msimd128 -matomics -mbulk-memory"
  export RT_O2_FILES="fle_hle fle_raster fle_sse_more fle_desk"   # decisions 0161, 0203, 0223: these runtime files at -O2, the rest of the runtime at RT_OPT
  mkdir -p "$OBJ"
}

b2d_adapter() {   # decision 0167: lift/harness/b2d1/b2d_swap.cpp -> $OBJ/b2d/b2d_swap.o (the lifted Blend2D module calls fle_swap_* of the adapter)
  [ -r "$B2D_UPSTREAM_LIB" ] && [ -d "$B2D_UPSTREAM_SRC" ] || die "B2D_UPSTREAM_LIB=<libblend2d.a of build_upstream.sh TARGET=wasm> and B2D_UPSTREAM_SRC=<its src/src dir> are required"
  mkdir -p "$OBJ/b2d"
  [ -s "$OBJ/b2d/b2d_swap.o" ] && return 0
  emcc -c -O2 -std=c++17 -fno-exceptions -fno-rtti -DBL_STATIC -msimd128 -matomics -mbulk-memory -I"$B2D_UPSTREAM_SRC" "$H/b2d1/b2d_swap.cpp" -o "$OBJ/b2d/b2d_swap.o" > "$LOGS/b2d_swap-compile.txt" 2>&1 || { tail -n 20 "$LOGS/b2d_swap-compile.txt"; die "b2d_swap.cpp failed to compile"; }
}

stage_wasm() {
  wasm_env
  hyb_strip
  local s
  b2d_adapter
  for s in rt harness units; do
    capped "wasm-$VARIANT-$s" bash "$H/two/build_nmod_wasm.sh" "$s" || { tail -n 20 "$LOGS/wasm-$VARIANT-$s.out" "$LOGS/wasm-$VARIANT-$s.err"; die "wasm stage $s failed"; }
  done
  echo "objects: $(ls "$OBJ"/mod "$OBJ"/fle "$OBJ"/rt "$OBJ"/h-fruity 2>/dev/null | grep -c '\.o$')"
}

stage_weblink() {
  wasm_env
  if [ -s "$WEB/flengine6.wasm" ]; then echo "weblink: $WEB/flengine6.wasm exists, skipped"; return 0; fi
  # the C functions that the direct wasm calls are exported, the functions that it defines are left undefined (wasm-merge resolves them), the link map gives the addresses of the data it shares; the adapter and the upstream library of the lifted Blend2D module
  [ -s "$HYB/exports.rsp" ] || die "no $HYB/exports.rsp: run the hybn0 stage first"
  b2d_adapter
  capped "weblink-$VARIANT" bash "$H/web/fruity/link-web.sh" "$WEB" "$OBJ/b2d/b2d_swap.o" "$B2D_UPSTREAM_LIB" "@$HYB/exports.rsp" "-Wl,-Map,$HYB/link.map" || { tail -n 20 "$LOGS/weblink-$VARIANT.out" "$WEB/link.log"; die "weblink failed"; }
  cat "$LOGS/weblink-$VARIANT.out"
  # decision 0238b: the hybrid stage must read the link map of THIS link; the record sits next to the C part (not in hyb/, which hard-linked copies of a work directory share)
  hyb_guard record "$HYB/link.map" "$WEB/link.map.sha256" || die "cannot record the link map"
}

stage_wasmopt() {   # Binaryen -Oz over the linked wasm (docs/investigations/size1/MERGE-NOTES.md; the --enable list is the one the emcc 6.0.12 link needs); this is the C part, the hybrid stage makes flengine6.opt.wasm
  [ -s "$WEB/flengine6.wasm" ] || die "no $WEB/flengine6.wasm: run the weblink stage first"
  local WOPT=$WEB/flengine6.c.opt.wasm
  if [ -s "$WOPT" ]; then echo "wasmopt: $WOPT exists, skipped"; return 0; fi
  [ -x "$WASM_OPT_BIN" ] || die "no wasm-opt at $WASM_OPT_BIN (WASM_OPT_BIN=)"
  rm -f "$WOPT.part"
  capped "wasmopt-$VARIANT" "$WASM_OPT_BIN" -Oz --enable-bulk-memory --enable-exception-handling --enable-sign-ext --enable-mutable-globals \
    --enable-nontrapping-float-to-int --enable-multivalue --enable-reference-types --enable-simd --enable-threads --enable-tail-call --enable-extended-const \
    -o "$WOPT.part" "$WEB/flengine6.wasm" || { tail -n 20 "$LOGS/wasmopt-$VARIANT.err"; die "wasm-opt failed"; }
  mv "$WOPT.part" "$WOPT"
  ls -l "$WEB/flengine6.wasm" "$WOPT" | tee "$LOGS/wasmopt-sizes.txt"
}

# ---- decision 0230: the hybrid. The stages hybn0 (before weblink) and hybrid (after wasmopt) and the strip of the C units in the wasm stage. Derived from docs/investigations/ship1/tools/hyb.sh (decisions 0189, 0208, 0227)
# with the paths of this script; the helper programs are in lift/harness/hyb/ (strip_units.py, chk_lowered.py, strip_glue_stubs.py, wasm_c_imports.py, c-imports-engine.txt, wasm-features.txt), so that the cache key of the wasm stage (lift/harness) covers them.
hyb_lists() {   # $HYB/lowered-<m>.txt = the list lift/harness/seeds/hyb-U-<m>.txt (a function once); lowered-all.txt = all modules (the VAs of the modules do not overlap)
  local m f
  mkdir -p "$HYB"; : > "$HYB/lowered-all.txt"
  for m in $HYB_MODS; do
    f=$H/seeds/hyb-U-$m.txt; [ -r "$f" ] || die "no $f"
    awk 'NF && !/^#/ && !seen[$1]++ {print $1}' "$f" > "$HYB/lowered-$m.txt"
    cat "$HYB/lowered-$m.txt" >> "$HYB/lowered-all.txt"
  done
  [ -s "$HYB/lowered-all.txt" ] || die "the lists of the hybrid are empty"
}

hyb_few() {   # FEW = the fe-wasmgen binary (ci/stage-lift.sh felift builds it with fe-pe and fe-lift; built here when it is absent)
  FEW=$WORK/cargo-target/release/fe-wasmgen
  if [ ! -x "$FEW" ]; then
    ( cd "$L" && CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_TARGET_DIR="$WORK/cargo-target" cargo build --release --offline -p fe-wasmgen ) > "$LOGS/cargo-build-wasmgen.txt" 2>&1 || die "cargo build of fe-wasmgen failed (see $LOGS/cargo-build-wasmgen.txt)"
  fi
}

hyb_modargs() {   # PE, SEEDS, PDATA of module $1: the arguments that fe-lift got in stage_gen / b2d2/gen-direct.sh (Blend2D: the wrapper seeds, no --pdata)
  local d; d=$(tag_dir)/dll
  PDATA="--pdata"
  case $1 in
    fle) PE=$FLE; SEEDS="--seeds $H/seeds/flengine-seeds-thunks.txt" ;;
    dsp) PE=$(ls "$d"/dsp_ippv2_x64.*.dll); SEEDS="--seeds $H/seeds/dsp-ippv2-2000000-seeds-idle1.txt"; PDATA="" ;;
    b2d) PE=$(ls "$d"/Blend2D.*.dll); SEEDS="--seeds $H/b2d1/blend2d-4700000-seeds-wrapper.txt"; PDATA="" ;;
    qfc) PE=$(ls "$d"/quickfontcache_x64.*.dll); SEEDS="" ;;
    ft)  PE=$(ls "$d"/freetype_x64.*.dll); SEEDS="--seeds $H/seeds/ft-seeds-4b00000-c2.txt" ;;
    ilt) PE=$(ls "$d"/ILTools_x64.*.dll); SEEDS="--seeds $H/seeds/iltools-6000000-seeds400.txt" ;;
    oggio) PE=$(ls "$d"/oggio_x64.*.dll); SEEDS="--seeds $H/seeds/dec1-seeds-oggio-c000000.txt" ;;
    elastique) PE=$(ls "$d"/elastique_x64.*.dll); SEEDS="--seeds $H/seeds/elastique-seeds-all3.txt" ;;
  esac
  [ -r "$PE" ] || die "module $1: no $PE"
}

hyb_flags() {   # RB, XB: the register file and the XMM file of the lifted code (LOWREGS_BASE, +0x200); FEW_FLAGS: the fe-wasmgen arguments that every module gets (the flags of the lowering are not arguments: decision 0243)
  RB=$(printf '0x%x' $((LOWREGS_BASE))); XB=$(printf '0x%x' $((LOWREGS_BASE + 0x200)))
  FEW_FLAGS="--c-imports $H/hyb/c-imports-engine.txt --mem-import c.memory"
}

hyb_guard() {   # decision 0238b (load2): checks that the addresses baked into the direct wasm are the ones of the C part (usage in the python text below). In this script, not in a file of lift/harness: a new file there would change the key of the lift stage (ci/plan.sh H_GEN); the function is hashed into SP_WASM (plan.sh lists it)
  python3 - "$@" <<'PY'
"""Decision 0238b (load2): the hybrid stage bakes four data addresses (fle_teb, fle_mxcsr, fle_hyb_buf, fle_progress_count) into the direct wasm; it reads them from the link map of the C part. If the link map is not the one of THIS C part (another build wrote it, a stage was run on part of an old work directory), the direct code reads and writes other variables
than the C part does and the engine misbehaves in ways that look like runtime faults (the first cursor creation gets a 0x0 bitmap and stops at CreateIconIndirect, 6 of 6 boots). These checks make such a build fail instead.
usage:
  hyb_guard record LINKMAP RECORD            RECORD := the sha256 of LINKMAP (the weblink stage writes it next to the C part, not next to the link map)
  hyb_guard map LINKMAP RECORD               exit 1 when LINKMAP is not the one that RECORD says (no RECORD: a note and exit 0; the check of the C part below still holds)
  hyb_guard cpart CPART TEB MXCSR HYBBUF PROGRESS [LIMIT LANDING HOOK]   exit 1 when one of the four (decision 0285: seven) addresses is not used by the C part (an `i32.const <address>` in the code of the C part is how the C part reaches each of them; measured in 9 bundles: 1 to 828 uses each)
  hyb_guard merged CPART MERGED MXCSR HYBBUF         exit 1 when the merged module does not use fle_mxcsr and fle_hyb_buf more often than the C part (the direct code uses them 1,234 and 2,333 times: a direct module made with other addresses does not)
The addresses are numbers (0x... or decimal). Only the bytes are searched: a false find can only hide a fault, not report one that is not there."""
import hashlib
import sys


def sleb(v):
    out = bytearray()
    while True:
        b = v & 0x7F
        v >>= 7
        if (v == 0 and not (b & 0x40)) or (v == -1 and (b & 0x40)):
            out.append(b)
            return bytes(out)
        out.append(b | 0x80)


def uses(data, addr):
    return data.count(b"\x41" + sleb(addr))


def sha(path):
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()


def main(a):
    if len(a) >= 3 and a[0] == "record":
        with open(a[2], "w") as f:
            f.write(sha(a[1]) + "  " + a[1].rsplit("/", 1)[-1] + "\n")
        return 0
    if len(a) >= 3 and a[0] == "map":
        try:
            want = open(a[2]).read().split()[0]
        except (OSError, IndexError):
            print("hyb_guard: no record of the link map at %s: the link map cannot be compared with the link that made the C part (only the check of the C part is made)" % a[2])
            return 0
        got = sha(a[1])
        if got != want:
            print("hyb_guard: FAILED: %s (sha256 %s) is not the link map that the weblink stage wrote for this C part (%s). Another build wrote it, or the work directory is a mix of two builds (hard-linked copies of hyb/ share the file); the direct wasm would get addresses of another layout." % (a[1], got[:16], want[:16]))
            return 1
        print("hyb_guard: link map is the one the weblink stage wrote (sha256 %s)" % got[:16])
        return 0
    if len(a) in (6, 9) and a[0] == "cpart":
        data = open(a[1], "rb").read()
        names = ("fle_teb", "fle_mxcsr", "fle_hyb_buf", "fle_progress_count", "fle_progress_limit", "fle_landing_top", "fle_dispatch_hook")
        rc, row = 0, []
        for n, v in zip(names, a[2:]):
            c = uses(data, int(v, 0))
            row.append("%s %s: %d" % (n, v, c))
            if c == 0:
                rc = 1
        print("hyb_guard: uses in the C part: " + "; ".join(row))
        if rc:
            print("hyb_guard: FAILED: the C part %s does not use an address that the direct wasm would be given. The link map is not the one of this C part (see the `map` check), so the direct code would touch other variables than the C part." % a[1])
        return rc
    if len(a) == 5 and a[0] == "merged":
        cpart, merged = open(a[1], "rb").read(), open(a[2], "rb").read()
        rc, row = 0, []
        for n, v in zip(("fle_mxcsr", "fle_hyb_buf"), a[3:]):
            c0, c1 = uses(cpart, int(v, 0)), uses(merged, int(v, 0))
            row.append("%s %s: C part %d, merged %d" % (n, v, c0, c1))
            if c1 <= c0:
                rc = 1
        print("hyb_guard: " + "; ".join(row))
        if rc:
            print("hyb_guard: FAILED: the direct code of the merged module does not use the addresses that were given: the direct modules are not the ones made with them (another build overwrote hyb/dir/n-*.wasm?).")
        return rc
    print(__doc__)
    return 2


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
PY
}

hyb_strip() {   # takes the bodies of the listed functions out of the C units (the prototypes stay: they become imports of the C part). Idempotent: $HYB/stripped records the list it was done for
  hyb_lists
  local want f out n=0 d=0 k=0 line
  want=$(sha256sum "$HYB/lowered-all.txt" | cut -c1-16)
  if [ "$(cat "$HYB/stripped" 2>/dev/null)" = "$want" ]; then echo "hybrid strip: done for list $want, skipped"; return 0; fi
  [ ! -e "$HYB/stripped" ] || die "$HYB/stripped is for another list: the units are already stripped of it (start from the lift output again)"
  : > "$LOGS/hyb-strip.txt"
  for f in "$WSRC"/src/fle-[0-9][0-9].c "$WSRC/v-$TAG"/src/m[0-9]-[0-9][0-9].c; do
    out=$f.stripped
    line=$(python3 "$H/hyb/strip_units.py" "$f" "$HYB/lowered-all.txt" "$out") || die "strip of $f failed"
    echo "$line" >> "$LOGS/hyb-strip.txt"; n=$((n + 1))
    if [ -e "$out" ]; then mv "$out" "$f"; fi
    d=$((d + $(echo "$line" | awk '{print $3}'))); k=$((k + $(echo "$line" | awk '{print $5}')))
  done
  echo "hybrid strip: $n units, dropped $d functions, kept $k" | tee -a "$LOGS/hyb-strip.txt"
  [ "$d" = "$(wc -l < "$HYB/lowered-all.txt")" ] || die "the strip dropped $d functions but the lists have $(wc -l < "$HYB/lowered-all.txt") (a listed function is not in the C units: the lists belong to another lift)"
  echo "$want" > "$HYB/stripped"
}

stage_hybn0() {   # fe-wasmgen with dummy addresses (and the dummy --tls-ctx 0x0: --hybrid-r needs it, the form of the register page is the thread-local one only): the C functions that the direct code calls (module c of the imports) -> $HYB/exports.rsp; and the check that it emits exactly the listed functions
  hyb_lists; hyb_few; hyb_flags
  local m
  : > "$HYB/c-imports-used.txt"
  for m in $HYB_MODS; do
    [ -s "$HYB/lowered-$m.txt" ] || continue
    hyb_modargs "$m"
    # shellcheck disable=SC2086
    capped "hyb-n0-$m" "$FEW" "$PE" "$HYB/n0-$m.wasm" $PDATA $SEEDS --teb 0x1000 --hybrid-r "$RB" --hybrid-x "$XB" --tls-ctx 0x0 --hybrid-mx 0x3000 0x3100 --hybrid-progress 0x3300 --hybrid-dispatch 0x3308 0x3310 0x3318 $FEW_FLAGS --hybrid-only "$HYB/lowered-$m.txt" --report "$HYB/n0-$m.tsv" || { tail -n 5 "$LOGS/hyb-n0-$m.err"; die "fe-wasmgen failed for $m"; }
    python3 "$H/hyb/wasm_c_imports.py" "$HYB/n0-$m.wasm" c >> "$HYB/c-imports-used.txt" || die "cannot read the imports of $HYB/n0-$m.wasm"
    python3 "$H/hyb/chk_lowered.py" "$HYB/lowered-$m.txt" "$HYB/n0-$m.tsv" | sed "s/^/hybn0 $m: /" || die "fe-wasmgen did not emit the listed functions of $m (see $HYB/n0-$m.tsv.missing)"
  done
  sort -u "$HYB/c-imports-used.txt" > "$HYB/c-imports-used.u.txt"
  sed 's/^/-Wl,--export-if-defined=/' "$HYB/c-imports-used.u.txt" > "$HYB/exports.rsp"
  wc -l "$HYB/exports.rsp"
}

stage_hybrid() {   # fe-wasmgen with the real addresses of the C part (link map), wasm-opt -Oz of each direct module, wasm-merge: direct modules into one, then into the C part (flengine6.c.opt.wasm) -> flengine6.opt.wasm; the glue loses the stubs of the defined functions
  [ -s "$WEB/flengine6.c.opt.wasm" ] || die "no $WEB/flengine6.c.opt.wasm: run the wasmopt stage first"
  [ -s "$HYB/link.map" ] || die "no $HYB/link.map: the weblink stage of a hybrid build writes it"
  if [ -s "$WEB/flengine6.opt.wasm" ]; then echo "hybrid: $WEB/flengine6.opt.wasm exists, skipped"; return 0; fi
  hyb_lists; hyb_few; hyb_flags
  local m T MXA BUFA PRG ins="" FEAT WM=${WASM_MERGE_BIN:-/opt/emsdk/upstream/bin/wasm-merge}
  [ -x "$WM" ] || die "no wasm-merge at $WM (WASM_MERGE_BIN=)"
  FEAT=$(cat "$H/hyb/wasm-features.txt")
  g() { grep -E "$1" "$HYB/link.map" | awk '{print $1}' | head -1; }
  T=$(g "^\s*[0-9a-f]+\s+[0-9a-f]+\s+[0-9a-f]+\s+fle_teb$"); MXA=$(g "^\s*[0-9a-f]+\s+[0-9a-f]+\s+4\s+fle_mxcsr$"); BUFA=$(g "^\s*[0-9a-f]+\s+0\s+40\s+fle_hyb_buf$"); PRG=$(g "^\s*[0-9a-f]+\s+[0-9a-f]+\s+8\s+fle_progress_count$")
  # decision 0285: the data that fle_dispatch reads besides the count (the typed indirect call of the direct wasm does the same checks): the limit (u64), the landing list head and the census hook (32-bit pointers)
  LIM=$(g "^\s*[0-9a-f]+\s+[0-9a-f]+\s+[0-9]+\s+fle_progress_limit$"); LAND=$(g "^\s*[0-9a-f]+\s+[0-9a-f]+\s+[0-9]+\s+fle_landing_top$"); HOOK=$(g "^\s*[0-9a-f]+\s+[0-9a-f]+\s+[0-9]+\s+fle_dispatch_hook$")
  echo "addresses: fle_teb 0x$T fle_mxcsr 0x$MXA fle_hyb_buf 0x$BUFA fle_progress_count 0x$PRG fle_progress_limit 0x$LIM fle_landing_top 0x$LAND fle_dispatch_hook 0x$HOOK" | tee "$LOGS/hyb-addresses.txt"
  [ -n "$T" ] && [ -n "$MXA" ] && [ -n "$BUFA" ] && [ -n "$PRG" ] && [ -n "$LIM" ] && [ -n "$LAND" ] && [ -n "$HOOK" ] || die "an address is missing in $HYB/link.map"
  local TLSARG CTXOFF
  # decision 0234: the register file of the direct wasm is the thread-local block fle_ctx, at this offset of the TLS segment (the link map)
  CTXOFF=$(g "^\s*[0-9a-f]+\s+[0-9a-f]+\s+300\s+fle_ctx$"); [ -n "$CTXOFF" ] || die "no fle_ctx (TLS, size 0x300) in $HYB/link.map"
  TLSARG="--tls-ctx 0x$CTXOFF"; echo "tls: fle_ctx at TLS offset 0x$CTXOFF" | tee -a "$LOGS/hyb-addresses.txt"
  mkdir -p "$HYB/dir"
  # decision 0238b: the addresses are baked into the direct code, so they have to be the ones the C part really uses (a link map of another build gave a bundle that stopped at CreateIconIndirect, 6 of 6 boots; docs/decisions/0238b)
  hyb_guard map "$HYB/link.map" "$WEB/link.map.sha256" || die "hybrid: the link map is not the one of this C part"
  hyb_guard cpart "$WEB/flengine6.c.opt.wasm" "0x$T" "0x$MXA" "0x$BUFA" "0x$PRG" "0x$LIM" "0x$LAND" "0x$HOOK" || die "hybrid: an address of the link map is not used by the C part"
  for m in $HYB_MODS; do
    [ -s "$HYB/lowered-$m.txt" ] || continue
    hyb_modargs "$m"
    # shellcheck disable=SC2086
    capped "hyb-gen-$m" "$FEW" "$PE" "$HYB/dir/n-$m.wasm" $PDATA $SEEDS --teb "0x$T" --hybrid-r "$RB" --hybrid-x "$XB" $TLSARG --hybrid-mx "0x$MXA" "0x$BUFA" --hybrid-progress "0x$PRG" --hybrid-dispatch "0x$LIM" "0x$LAND" "0x$HOOK" $FEW_FLAGS --hybrid-only "$HYB/lowered-$m.txt" --report "$HYB/dir/n-$m.tsv" || { tail -n 5 "$LOGS/hyb-gen-$m.err"; die "fe-wasmgen failed for $m"; }
    python3 "$H/hyb/chk_lowered.py" "$HYB/lowered-$m.txt" "$HYB/dir/n-$m.tsv" | sed "s/^/hybrid $m: /" || die "fe-wasmgen did not emit the listed functions of $m"
    # wasm-opt validates its input as it reads it
    capped "hyb-opt-$m" "$WASM_OPT_BIN" -Oz --enable-bulk-memory --enable-exception-handling --enable-sign-ext --enable-mutable-globals --enable-nontrapping-float-to-int --enable-multivalue \
      --enable-reference-types --enable-simd --enable-threads --enable-tail-call --enable-extended-const -o "$HYB/dir/n-$m.oz.wasm" "$HYB/dir/n-$m.wasm" || { tail -n 5 "$LOGS/hyb-opt-$m.err"; die "wasm-opt failed for the direct module $m"; }
    echo "hybrid $m: $(stat -c %s "$HYB/dir/n-$m.wasm") -> -Oz $(stat -c %s "$HYB/dir/n-$m.oz.wasm") bytes"
    ins="$ins $HYB/dir/n-$m.oz.wasm d$m"
  done
  hyb_guard map "$HYB/link.map" "$WEB/link.map.sha256" || die "hybrid: the link map changed while the direct modules were made"
  # shellcheck disable=SC2086
  capped hyb-merge1 "$WM" $FEAT --rename-export-conflicts $ins -o "$HYB/direct.wasm" || { tail -n 5 "$LOGS/hyb-merge1.err"; die "wasm-merge of the direct modules failed"; }
  capped hyb-merge2 "$WM" $FEAT --rename-export-conflicts "$HYB/direct.wasm" env "$WEB/flengine6.c.opt.wasm" c -o "$HYB/merged.wasm" || { tail -n 5 "$LOGS/hyb-merge2.err"; die "wasm-merge into the C part failed"; }
  hyb_guard merged "$WEB/flengine6.c.opt.wasm" "$HYB/merged.wasm" "0x$MXA" "0x$BUFA" || die "hybrid: the direct code of the merged module does not use the addresses of this link"
  [ "$(python3 "$H/hyb/wasm_c_imports.py" "$HYB/merged.wasm" c | wc -l)" = 0 ] || die "the merged module still imports functions of the module c"
  [ "$(python3 "$H/hyb/wasm_c_imports.py" "$HYB/merged.wasm" env | grep -c '^f_')" = 0 ] || die "the merged module still imports a lifted function (f_<va>) from env: a listed function is not defined by the direct code"
  [ -e "$WEB/flengine6.orig.js" ] || cp "$WEB/flengine6.js" "$WEB/flengine6.orig.js"
  python3 "$H/hyb/strip_glue_stubs.py" "$WEB/flengine6.orig.js" "$WEB/flengine6.js" | tee "$LOGS/hyb-glue.txt"
  [ "$(grep -c "missing function" "$WEB/flengine6.js")" = 0 ] || die "the glue still has the stub of a missing function after the strip (an undefined symbol that the direct code does not define)"
  mv "$HYB/merged.wasm" "$WEB/flengine6.opt.wasm"
  ls -l "$WEB" "$HYB/direct.wasm" | tee "$LOGS/hyb-sizes.txt"
}

stage_seed() {
  local TAGDIR; TAGDIR=$(tag_dir)
  [ -n "$TAGDIR" ] || die "no $ONAT/v-*: run the gen stage first"
  mkdir -p "$WORK/in/dll"
  if [ ! -s "$WORK/in/seedpack.bin" ]; then
    python3 "$H/web/final0129/pack_seed.py" --root "$HOST_ROOT" --tsv "$H/seeds/vfs-seed-idle1.tsv" \
      --extra "$H/seeds/reg-seed-nsis.tsv" --extra "$H/seeds/reg-seed-app.tsv" --extra "$H/seeds/authenticode-claims.tsv" --out "$WORK/in/seedpack.bin" > "$LOGS/pack_seed.txt" || die "pack_seed.py failed"
    cat "$LOGS/pack_seed.txt" | head -12
  fi
  # the guest's modules: the original engine DLL and the seven rebased modules the lifter made (their names carry the base: dsp_ippv2_x64.2000000.dll ...)
  cp -n "$FLE" "$WORK/in/dll/FLEngine_x64.dll"
  local f
  for f in "$TAGDIR"/dll/*.dll; do cp -n "$f" "$WORK/in/dll/$(basename "$f")"; done
  ls -l "$WORK/in/dll" | tee "$LOGS/dll-list.txt"
  [ "$(ls "$WORK/in/dll" | wc -l)" = $((6 + NEXTRA)) ] || die "expected $((6 + NEXTRA)) files in $WORK/in/dll"
}

stage_bundle() {
  [ -s "$WEB/flengine6.wasm" ] || die "no $WEB/flengine6.wasm: run the weblink stage first"
  local ENGINE_WASM=$WEB/flengine6.opt.wasm
  [ -s "$ENGINE_WASM" ] || die "no $ENGINE_WASM: run the hybrid stage"
  [ -s "$WORK/in/seedpack.bin" ] || die "no seed pack: run the seed stage first"
  local dsp blend qfc ft il mods
  dsp=$(cd "$WORK/in/dll" && ls dsp_ippv2_x64.*.dll); blend=$(cd "$WORK/in/dll" && ls Blend2D.*.dll); qfc=$(cd "$WORK/in/dll" && ls quickfontcache_x64.*.dll)
  ft=$(cd "$WORK/in/dll" && ls freetype_x64.*.dll); il=$(cd "$WORK/in/dll" && ls ILTools_x64.*.dll)
  mods="/data/$dsp:/data/$blend:/data/$qfc:/data/$ft:/data/$il"
  for x in $EXTRA_MODULES; do mods="$mods:/data/$(cd "$WORK/in/dll" && ls "${x}"_x64.*.dll)"; done   # modules 6 and 7 in the order of EXTRA_MODULES (= the order of modules.txt)
  local sha; sha=$(sha256sum "$ENGINE_WASM" | cut -c1-12)
  # decision 0141: an engine with SEH (FLE_SEH_HANDLERS) does not stop at wall 16; the stop note below is for the walls known when this was written (decision 0243: the page names no window, and the manifest has no expected stop frame).
  local STOPNOTE_ARGS=(--stop-note "indirect call to 0=>The engine called address 0: a function it asked GetProcAddress for came back 0. Decision 0136 section 7 read this wall (after the welcome wizard is closed) as user32!UpdateLayeredWindow of the layered window TLightweightLayeredControl. Decision 0140 gave the engine a model of it, so a bundle built from a tree with that decision does not stop here. It stopped by design; the desktop shows the last picture of each window.")
  # decision 0197: the stop note that the page shows when the engine stops with the runtime message of fle_pruned() (lift/runtime/fle_prune.h: "fle: pruned function 0x... was executed ...")
  STOPNOTE_ARGS+=(--stop-note "pruned function=>This build leaves out the IPP CPU variants that the fixed CPUID answer should never reach (prune list $(basename "$PRUNE_LIST")). The engine reached one of them, so that leave-out is wrong for this run.")
  # decision 0221: the Emscripten that linked the glue. emcc says so when the toolchain is installed; a CI run that only packs the bundle (everything before it came from the cache) has no emsdk, and the
  # version that linked the cached glue is the one in the link key: EMSDK_VERSION of ci/versions.env (exported by run_bb). Without either the manifest would be invalid, so say so here.
  local EMVER; EMVER=$(emcc --version 2>/dev/null | head -1 | grep -o '[0-9][0-9]*\.[0-9][0-9]*\.[0-9][0-9]*' | head -1)
  [ -n "$EMVER" ] || EMVER=${EMSDK_VERSION:-}
  [ -n "$EMVER" ] || die "cannot tell which Emscripten linked the glue: emcc is not installed and EMSDK_VERSION is not set"
  local HOSTFILES_ARGS=(--host-files "emscripten-$EMVER")
  rm -rf "$BUNDLE"; mkdir -p "$BUNDLE"
  node "$HERE/pack-assets.mjs" --out "$BUNDLE" --prefix "$PREFIX" --name "fl-engine-$VARIANT" --version "$sha" \
    --description "lifted FL Studio engine (decision 0129 final configuration; variant $VARIANT; HEAD $(git -C "$SRCTREE" rev-parse --short HEAD 2>/dev/null || echo archive); one configuration (decision 0243); always-inline-units=$(echo $AI_UNITS | wc -w) prune-list=$(basename "$PRUNE_LIST") modules=8 hybrid=U)" \
    --engine-wasm "$ENGINE_WASM" --glue "$WEB/flengine6.js" --seed-pack "$WORK/in/seedpack.bin" --dll-dir "$WORK/in/dll" \
    --args "/data/FLEngine_x64.dll $mods" \
    --env FLE_VFS_SEED=/data/seed/vfs-seed-idle1.tsv --env FLE_VFS_SEED_ROOT=/data/seed \
    --env FLE_REG_SEED=/data/seed/reg-seed-nsis.tsv:/data/seed/reg-seed-app.tsv --env FLE_AUTH_CLAIMS=/data/seed/authenticode-claims.tsv \
    --env FLE_FRUITY_MAX_CALLS=4000000000 --env "FLE_FRUITY_SECONDS=$FRUITY_SECONDS" \
    --fruity-hints "first=$HINT_FIRST_PIXEL_S,stop=$HINT_STOP_S,variant=$VARIANT" \
    "${STOPNOTE_ARGS[@]}" \
    "${HOSTFILES_ARGS[@]}" --brotli-quality "$BROTLI_QUALITY" --bucket "${BUCKET:-YOUR-BUCKET}" || die "pack-assets.mjs failed"
  echo; echo "== upload commands (NOT executed; $BUNDLE/upload-commands.txt)"; cat "$BUNDLE/upload-commands.txt"
}

DEFAULT_STAGES="gen resplit wasm hybn0 weblink wasmopt hybrid seed bundle"
STAGES=${*:-$DEFAULT_STAGES}
for s in $STAGES; do
  case "$s" in gen|resplit|wasm|hybn0|weblink|wasmopt|hybrid|seed|bundle) ;; *) die "unknown stage '$s'" ;; esac
  echo "== $(date -u +%H:%M:%SZ) stage $s (JOBS=$JOBS)"
  "stage_$s" || die "stage $s failed"
done
echo "== done: $(date -u +%H:%M:%SZ)"
