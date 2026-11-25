#!/bin/bash
# The plugin-worker wasm of one external VST3 (app/src/plugins/plugin-worker.ts; #1367): <output>/vst-ext.wasm, the module (<output>/<name>.vst3, rebased to
# ImageBase 0x2000000) and the seed list (<output>/seed.tsv). The input is the installer directory (ci/artifacts.lock names the files); nothing is run.
# usage: bash lift/harness/web/mt1/plugin-worker/build.sh <ott|serum2|vital> <installer dir> <output dir>
# Stages, in order. A failing stage stops the run with its name on stderr (exit 1); a usage error is 64:
#   check    tools/external-inputs/check.mjs: the installers against ci/artifacts.lock
#   extract  tools/external-inputs/extract.mjs: the VST3 binary of the plugin, read out of the installer
#   rebase   fe-pe rebase to ImageBase 0x2000000 (below the wasm GLOBAL_BASE; the layout of decision 0177)
#   lift     mkrows.sh (host rows), mk_stubs.mjs (import rows, stubs), fe-decode reloc-seeds (the addresses that the image's pointers name and that are not
#            function starts), seeds/<name>.txt (the switch-table and lea seeds, lift/harness/vst/seeds_scan.mjs), gen_vst.sh (fe-lift lift into units,
#            merged dispatch table, realmod table)
#   compile  emcc: the units, the runtime (lift/runtime/fle_*.c), image.c, the harness (vst3a C files and main_vst_ext.c)
#   link     emcc -> vst.wasm -> <output>/vst-ext.wasm
# The work directory is .tmp-plugin-<name> in the repository (removed at the start; kept after the run for the next reading of the logs).
set -u -o pipefail
HERE=$(cd "$(dirname "$0")" && pwd)
ROOT=$(cd "$HERE/../../../../.." && pwd)
LIFT=$ROOT/lift
VST=$LIFT/harness/vst
VST3A=$LIFT/harness/vst3a
RT=$LIFT/runtime
FE=$LIFT/target/release
JOBS=4
RMBASE=0x2000000
INITMEM=$((0x28000000))
MAXMEM=$((0xF0000000))
GLOBAL_BASE=268435456

case ${1:-} in
    ott) SLUG=ott; NAME=ott ;;
    serum2) SLUG=serum-2; NAME=serum2 ;;
    vital) SLUG=vital; NAME=vital ;;
    *) echo "usage: build.sh <ott|serum2|vital> <installer dir> <output dir>" >&2; exit 64 ;;
esac
INST=${2:?usage: build.sh <ott|serum2|vital> <installer dir> <output dir>}
OUT=${3:?usage: build.sh <ott|serum2|vital> <installer dir> <output dir>}
RMNAME=$NAME.vst3
WORK=$ROOT/.tmp-plugin-$NAME

fail() { echo "build: stage $1 failed: $2" >&2; exit 1; }
stage() { echo "build: stage $1" >&2; }

rm -rf "$OUT"
mkdir -p "$OUT" "$WORK"
rm -rf "$WORK/ext" "$WORK/check.json" "$WORK/check.err"

stage tools
cargo build --release --locked --manifest-path "$LIFT/Cargo.toml" > "$WORK/cargo.log" 2>&1 || { tail -20 "$WORK/cargo.log" >&2; fail tools "cargo build of fe-lift, fe-pe, fe-decode"; }

stage check
node "$ROOT/tools/external-inputs/check.mjs" "$INST" > "$WORK/check.json" 2> "$WORK/check.err" || { cat "$WORK/check.json" "$WORK/check.err" >&2; fail check "the installers do not match ci/artifacts.lock"; }
echo "build: check $(cat "$WORK/check.json")" >&2

stage extract
node "$ROOT/tools/external-inputs/extract.mjs" "$INST" "$WORK/ext" > "$WORK/extract.txt" 2> "$WORK/extract.err" || { cat "$WORK/extract.err" >&2; fail extract "no VST3 binary for $NAME"; }
MOD=$(ls "$WORK/ext/$SLUG"/*.vst3 2> /dev/null | head -1)
[ -n "$MOD" ] || fail extract "no $SLUG/*.vst3 in the extraction"
echo "build: extract $(wc -c < "$MOD") bytes" >&2

# The work directory is kept between runs while the inputs are the same (the module's bytes, the tools and the sources): the lifted units, and the objects
# that are newer than their sources, are then not made again. Any other input starts from an empty work directory.
KEY=$( { sha256sum "$MOD"; sha256sum "$FE/fe-lift" "$FE/fe-pe" "$VST/gen_vst.sh" "$VST/merge_tables.mjs" \
    "$VST/mk_stubs.mjs" "$VST/mkrows.sh" "$VST/imports-vst.txt" "$VST/ordinals-vst.txt" "$LIFT/crates/fe-lift/src/imports_table.rs" "$RT/fle_modules.h" \
    "$RT"/fle_*.c "$VST3A"/*.c "$VST3A/wraps.txt"; echo "$NAME $RMBASE $JOBS seeds-reloc-v1"; cat "$HERE/seeds/$NAME.txt" 2> /dev/null; } | sha256sum | cut -d' ' -f1)
KEEP=0
if [ "$(cat "$WORK/key" 2> /dev/null)" = "$KEY" ] && [ -s "$WORK/src/common-$NAME.c" ]; then KEEP=1; else
    find "$WORK" -mindepth 1 -maxdepth 1 ! -name ext -exec rm -rf {} +
fi
mkdir -p "$WORK/rows" "$WORK/shadow" "$WORK/src" "$WORK/units" "$WORK/rt" "$WORK/h" "$WORK/link"
echo "$KEY" > "$WORK/key"
rm -rf "$WORK/link"; mkdir -p "$WORK/link"

stage rebase
rm -f "$WORK/$NAME.2000000.vst3"
"$FE/fe-pe" rebase "$MOD" "$RMBASE" "$WORK/$NAME.2000000.vst3" > "$WORK/rebase.txt" 2>&1 || { cat "$WORK/rebase.txt" >&2; fail rebase "fe-pe rebase"; }
PE="$WORK/$NAME.2000000.vst3"

stage lift
if [ "$KEEP" = 1 ]; then echo "build: lift kept (same inputs as the last run)" >&2; else
bash "$VST/mkrows.sh" "$WORK/rows/hm.txt" || fail lift "mkrows.sh"
node "$VST/mk_stubs.mjs" "$PE" "$LIFT/crates/fe-lift/src/imports_table.rs" "$VST/imports-vst.txt" "$RT" \
    "$WORK/rows/stub-rows.txt" "$WORK/rows/vst3a_stubs.c" "$WORK/rows/ord-rows.txt" "$VST/ordinals-vst.txt" || fail lift "mk_stubs.mjs"
{ grep -v '^#' "$VST/imports-vst.txt"; cat "$WORK/rows/stub-rows.txt"; } > "$WORK/rows/imports.txt"
cat "$WORK/rows/hm.txt" "$WORK/rows/ord-rows.txt" > "$WORK/rows/host-rows.txt"
"$FE/fe-decode" reloc-seeds "$PE" --not-function > "$WORK/rows/reloc-seeds.txt" || fail lift "fe-decode reloc-seeds"
grep -hv "^#" "$WORK/rows/reloc-seeds.txt" "$HERE/seeds/$NAME.txt" 2> /dev/null | sort -u > "$WORK/rows/seeds.txt" || true
OUT=$WORK TAG=$NAME FILE=$PE FEBIN=$FE LIFT_ARGS="--pdata --seeds $WORK/rows/seeds.txt --extra-imports $WORK/rows/imports.txt" ROWS=$WORK/rows/host-rows.txt \
    RMNAME=$RMNAME RMBASE=$RMBASE bash "$VST/gen_vst.sh" || fail lift "gen_vst.sh"
fi

stage compile
sed 's/ *__attribute__((noreturn))//' "$RT/fle_modules.h" > "$WORK/shadow/fle_modules.h"
printf '#define VST_RM_NAME "%s"\n#define VST_RM_BASE %s\n' "$RMNAME" "$RMBASE" > "$WORK/vst_rm.h"
export GEN="-std=gnu11 -O2 -ffp-contract=off -fno-fast-math -w -fno-builtin-fprintf -fno-builtin-fputs -ferror-limit=0 -I$WORK/shadow -I$RT -I$VST -I$HERE"
export RTF="-std=gnu11 -O2 -ffp-contract=off -fno-fast-math -w -I$RT"
export HF="-std=gnu11 -O1 -w -I$RT -I$LIFT/harness -I$VST -I$VST3A -I$HERE -include $WORK/vst_rm.h"
export IMGF="-std=gnu11 -O1 -w -I$RT -I$LIFT/harness"
cc_one() {
    local flags
    case $3 in
        unit) flags=$GEN ;;
        rt) flags=$RTF ;;
        img) flags=$IMGF ;;
        *) flags=$HF ;;
    esac
    [ "$2" -nt "$1" ] && return 0
    ( ulimit -v 12582912; emcc -c $flags "$1" -o "$2" > "$2.log" 2>&1 ) || { echo "build: compile $1 failed" >&2; tail -20 "$2.log" >&2; return 1; }
}
export -f cc_one
{
    for f in "$WORK/src/$NAME"-[0-9][0-9].c "$WORK/src/common-$NAME.c" "$WORK/src/realmod-$NAME.c"; do echo "$f $WORK/units/$(basename "$f" .c).o unit"; done
    for f in "$RT"/fle_*.c; do echo "$f $WORK/rt/$(basename "$f" .c).o rt"; done
    echo "$LIFT/harness/image.c $WORK/h/image.o img"
    for f in "$VST3A"/vst_host3a.c "$VST3A"/vst_env3a.c "$VST3A"/vst3a_imp.c "$VST3A"/vst_stream3a.c "$VST3A"/vst_msg3a.c "$VST3A"/vst_exc3a.c "$VST3A"/wasm_cpuclock.c \
             "$HERE/main_vst_ext.c" "$HERE/thread_stub.c" "$WORK/rows/vst3a_stubs.c"; do echo "$f $WORK/h/$(basename "$f" .c).o harness"; done
} > "$WORK/compile.list"
xargs -P "$JOBS" -n 3 bash -c 'cc_one "$0" "$1" "$2"' < "$WORK/compile.list" || fail compile "see the messages above"
for f in "$WORK/units"/*.o "$WORK/rt"/*.o "$WORK/h"/*.o; do [ -s "$f" ] || fail compile "$f is empty"; done

stage link
WRAPS=$(tr '\n' ' ' < "$VST3A/wraps.txt")
( ulimit -v 12582912; emcc -O2 --profiling-funcs -sALLOW_MEMORY_GROWTH=1 -sSTACK_SIZE=8MB -sGLOBAL_BASE="$GLOBAL_BASE" -sENVIRONMENT=node -sMODULARIZE=1 -sEXIT_RUNTIME=1 \
    -Wl,--initial-memory="$INITMEM" -Wl,--max-memory="$MAXMEM" -sMAXIMUM_MEMORY="$MAXMEM" -Wl,--export-memory \
    -sEXPORTED_FUNCTIONS=_main,_fflush -sERROR_ON_UNDEFINED_SYMBOLS=0 -sWARN_ON_UNDEFINED_SYMBOLS=0 -Wl,--allow-undefined \
    -Wl,--wrap=fle_imp_LoadLibraryExW -Wl,--wrap=fle_imp_LoadLibraryW -Wl,--wrap=fle_imp_LoadLibraryA -Wl,--wrap=fle_imp_GetModuleHandleExW -Wl,--wrap=fle_imp_CreateWindowExW \
    -Wl,--wrap=fle_imp_CreateFileW -Wl,--wrap=fle_imp_GetFileType -Wl,--wrap=fle_imp_FormatMessageW -Wl,--wrap=fle_imp_SetFilePointerEx -Wl,--wrap=fle_imp_LockResource -Wl,--wrap=fle_imp_CreateThread -Wl,--wrap=fle_imp_RaiseException \
    $WRAPS -Wl,--wrap=clock_gettime \
    "$WORK"/units/*.o "$WORK"/rt/*.o "$WORK"/h/*.o -lm -o "$WORK/link/vst.js" > "$WORK/link/link.log" 2>&1 ) || { tail -30 "$WORK/link/link.log" >&2; fail link "emcc"; }
cp "$WORK/link/vst.wasm" "$OUT/vst-ext.wasm" || fail link "no vst.wasm"

cp "$PE" "$OUT/$NAME.vst3" || fail output "module"
cp "$VST/vfs-seed-vst.tsv" "$OUT/seed.tsv" || fail output "seed list"
echo "build: done $OUT (vst-ext.wasm $(wc -c < "$OUT/vst-ext.wasm") bytes)" >&2
