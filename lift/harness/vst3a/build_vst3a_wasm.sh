#!/bin/bash
# Decision 0177 (srm1): wasm (emcc) build of the lifted Serum2.vst3 (REBASED to 0x2000000, see harness/vst/build_vst_wasm.sh) + the runtime + the vst3a host harness.
# A merge of vst/build_vst_wasm.sh (emcc, link) and vst3a/build_vst3a_native.sh (the harness file list, stubs, wraps). Same sources as the native build; a built object is skipped.
# usage: OUT=<gen dir with src/ and rows/> TAG=s2w FILE=<rebased PE> OPT=O2|Oz JOBS=3 [XCF="-msimd128"] [SUF=...] [RTX=...] [LINKX=...] bash lift/harness/vst3a/build_vst3a_wasm.sh
#   XCF: extra flags for ALL objects (units, runtime, harness); RTX: extra flags for the runtime and harness only (the piano: -DFLE_THREAD_WASM -sSUPPORT_LONGJMP=wasm); LINKX: extra link flags
#   (the piano: -sJSPI -sSUPPORT_LONGJMP=wasm --js-library lift/harness/web/threads/fle_thread_lib.js -sEXPORTED_FUNCTIONS=_main,_fle_wasm_thread_entry -sEXPORTED_RUNTIME_METHODS=wasmExports,stackRestore; a profile: --profiling-funcs).
#   The runtime includes runtime/fle_malloc16.c ( 16-byte aligned malloc, decision 0177). To reuse the unit objects of another build: make $D/obj a symlink to its obj/.
# Output: $OUT/wasm/$TAG-$OPT$SUF/vst.{js,wasm}; run with run_vst3a_node.cjs.
set -u -o pipefail
export PYTHONDONTWRITEBYTECODE=1
HERE=$(cd "$(dirname "$0")" && pwd)
LIFT=${LIFT:-$(cd "$HERE/../.." && pwd)}
VST=$LIFT/harness/vst
OUT=${OUT:?OUT is not set}; TAG=${TAG:-s2w}; FILE=${FILE:?FILE (the rebased PE) is not set}
OPT=${OPT:-O2}; JOBS=${JOBS:-3}; XCF=${XCF:-}; SUF=${SUF:-}; RTX=${RTX:-}   # RTX: extra flags for the runtime and harness objects only (the state-2 build: "-DFLE_THREAD_WASM -sSUPPORT_LONGJMP=wasm")
RT="$LIFT/runtime"
D="$OUT/wasm/$TAG-$OPT$SUF"
EMCC_VMEM_KB=${EMCC_VMEM_KB:-12582912}
INITMEM=${INITMEM:-$((0x28000000))}
MAXMEM=${MAXMEM:-$((0xF0000000))}
GLOBAL_BASE=${GLOBAL_BASE:-268435456}
mkdir -p "$D/obj" "$D/rt" "$D/h" "$D/log" "$D/shadow" "$OUT/tmp" "$OUT/emcache"
export PATH=/opt/emsdk:/opt/emsdk/upstream/emscripten:$PATH
export TMPDIR="$OUT/tmp" EM_CACHE="$OUT/emcache"
BI="-fno-builtin-fprintf -fno-builtin-fputs"
sed 's/ *__attribute__((noreturn))//' "$RT/fle_modules.h" > "$D/shadow/fle_modules.h"
GEN="-std=gnu11 -$OPT $XCF -w $BI -ffp-contract=off -fno-fast-math -ferror-limit=0 -I$D/shadow -I$RT -I$VST -I$HERE"
RMNAME=${RMNAME:-serum2.vst3}
RMBASE=$(python3 -c "import struct,sys; d=open(sys.argv[1],'rb').read(0x400); l=struct.unpack_from('<I',d,0x3c)[0]; print(hex(struct.unpack_from('<Q',d,l+24+24)[0]))" "$FILE")
compile_one() {   # $1 source, $2 object, $3 flags
    [ "$2" -nt "$1" ] && return 0
    ( ulimit -v "$EMCC_VMEM_KB"; /usr/bin/time -f "%e s %M KB" -o "$2.time" emcc -c $3 "$1" -o "$2" > "$2.log" 2>&1 ) || { echo "FAILED $1 (see $2.log)"; return 1; }
    echo "$(basename "$1") $(tail -1 "$2.time")"
}
export -f compile_one; export EMCC_VMEM_KB
ls "$OUT"/src/$TAG-[0-9][0-9].c "$OUT"/src/common-$TAG.c | xargs -P "$JOBS" -I{} bash -c 'b=$(basename {} .c); compile_one {} '"$D"'/obj/$b.o "'"$GEN"'"' || exit 1
for f in "$RT"/fle_*.c; do b=$(basename "$f" .c); compile_one "$f" "$D/rt/$b.o" "-std=gnu11 -$OPT $XCF $RTX -ffp-contract=off -fno-fast-math -Wall -Wextra -I$RT" || exit 1; done
compile_one "$LIFT/harness/image.c" "$D/h/image.o" "-std=gnu11 -O1 $RTX -Wall -Wextra -I$RT -I$LIFT/harness" || exit 1
HF=""
for f in "$VST"/*.c; do b=$(basename "$f"); case $b in main_vst.c|vst_host.c|vst_env.c|hybrid_glue.c|vst_guard.c) continue;; esac; HF="$HF $f"; done
HF="$HF $HERE/main_vst3a.c $HERE/vst_host3a.c $HERE/vst_env3a.c $HERE/vst3a_imp.c $HERE/vst_stream3a.c $HERE/vst_msg3a.c $HERE/vst_exc3a.c $HERE/wasm_cpuclock.c $OUT/rows/vst3a_stubs.c"
for f in $HF; do b=$(basename "$f" .c); compile_one "$f" "$D/h/$b.o" "-std=gnu11 -O1 $XCF $RTX -Wall -Wextra -Wno-unused-function -Wno-unused-parameter -I$RT -I$LIFT/harness -I$VST -I$HERE -DVST_RM_NAME=\"\\\"$RMNAME\\\"\" -DVST_RM_BASE=$RMBASE" || exit 1; done
LINKX=${LINKX:-}
rm -f "$D/vst.js" "$D/vst.wasm"
( ulimit -v "$EMCC_VMEM_KB"; /usr/bin/time -f "link %e s %M KB" -o "$D/log/link.time" emcc -$OPT \
    -sALLOW_MEMORY_GROWTH=1 -sSTACK_SIZE=8MB -sGLOBAL_BASE="$GLOBAL_BASE" -sENVIRONMENT=node -sMODULARIZE=1 -sEXIT_RUNTIME=1 -sNODERAWFS=1 \
    -Wl,--initial-memory="$INITMEM" -Wl,--max-memory="$MAXMEM" -sMAXIMUM_MEMORY="$MAXMEM" \
    -Wl,--wrap=fle_imp_LoadLibraryExW -Wl,--wrap=fle_imp_LoadLibraryW -Wl,--wrap=fle_imp_LoadLibraryA -Wl,--wrap=fle_imp_GetModuleHandleExW -Wl,--wrap=fle_imp_CreateWindowExW \
    -Wl,--wrap=fle_imp_CreateFileW -Wl,--wrap=fle_imp_GetFileType -Wl,--wrap=fle_imp_FormatMessageW -Wl,--wrap=fle_imp_SetFilePointerEx -Wl,--wrap=fle_imp_LockResource -Wl,--wrap=fle_imp_CreateThread \
    $(cat "$HERE/wraps.txt" 2>/dev/null) -Wl,--wrap=clock_gettime $LINKX \
    "$D"/h/*.o "$D"/obj/*.o "$D"/rt/*.o -lm -o "$D/vst.js" > "$D/log/link.log" 2>&1 )
echo "link exit=$?"; tail -1 "$D/log/link.time"; ls -la "$D"/vst.*
