#!/bin/bash
# wasm (emcc, node) build of ONE lifted PE + the runtime + the VST3 host harness main_vst.c (decision 0156). Same sources as build_vst_native.sh; the PE must be REBASED below
# GLOBAL_BASE (fe-pe rebase FILE 0x2000000 OUT; the image is mapped at its ImageBase inside the linear memory, harness/image.c). usage:
#   OUT=lift/out-vst2 TAG=ottw FILE=<rebased PE> OPT=O2 BUILTIN=1 JOBS=3 bash lift/harness/vst/build_vst_wasm.sh
# Output: $OUT/wasm/$TAG-$OPT-b$BUILTIN/vst.{js,wasm}; run with run_vst_node.cjs. A built object is skipped.
set -u -o pipefail
export PYTHONDONTWRITEBYTECODE=1
HERE=$(cd "$(dirname "$0")" && pwd)
LIFT=${LIFT:-$(cd "$HERE/../.." && pwd)}
OUT=${OUT:?OUT is not set}; TAG=${TAG:-ottw}; FILE=${FILE:?FILE (the rebased PE) is not set}
OPT=${OPT:-O2}; BUILTIN=${BUILTIN:-1}; JOBS=${JOBS:-3}
RT="$LIFT/runtime"
CFG="$OPT-b$BUILTIN"
D="$OUT/wasm/$TAG-$CFG"
EMCC_VMEM_KB=${EMCC_VMEM_KB:-12582912}
INITMEM=${INITMEM:-$((0x28000000))}
MAXMEM=${MAXMEM:-$((0x80000000))}
GLOBAL_BASE=${GLOBAL_BASE:-268435456}
mkdir -p "$D/obj" "$D/rt" "$D/h" "$D/log" "$D/shadow" "$OUT/tmp" "$OUT/emcache"
export PATH=/opt/emsdk:/opt/emsdk/upstream/emscripten:$PATH
export TMPDIR="$OUT/tmp" EM_CACHE="$OUT/emcache"
BI="-fno-builtin"; [ "$BUILTIN" = 1 ] && BI="-fno-builtin-fprintf -fno-builtin-fputs"
sed 's/ *__attribute__((noreturn))//' "$RT/fle_modules.h" > "$D/shadow/fle_modules.h"
GEN="-std=gnu11 -$OPT -w $BI -ffp-contract=off -fno-fast-math -ferror-limit=0 -I$D/shadow -I$RT -I$HERE"
RMNAME=${RMNAME:-$(basename "$FILE" | sed 's/\.[0-9a-f]*\.vst3$/.vst3/; s/\.[0-9a-f]*\.dll$/.dll/' | tr 'A-Z' 'a-z')}
RMBASE=$(python3 -c "import struct,sys; d=open(sys.argv[1],'rb').read(0x400); l=struct.unpack_from('<I',d,0x3c)[0]; print(hex(struct.unpack_from('<Q',d,l+24+24)[0]))" "$FILE")
compile_one() {   # $1 source, $2 object, $3 flags
    [ "$2" -nt "$1" ] && return 0
    ( ulimit -v "$EMCC_VMEM_KB"; /usr/bin/time -f "%e s %M KB" -o "$2.time" emcc -c $3 "$1" -o "$2" > "$2.log" 2>&1 ) || { echo "FAILED $1 (see $2.log)"; return 1; }
    echo "$(basename "$1") $(tail -1 "$2.time")"
}
export -f compile_one; export EMCC_VMEM_KB
ls "$OUT"/src/$TAG-[0-9][0-9].c "$OUT"/src/common-$TAG.c | xargs -P "$JOBS" -I{} bash -c 'b=$(basename {} .c); compile_one {} '"$D"'/obj/$b.o "'"$GEN"'"' || exit 1
for f in "$RT"/fle_*.c; do b=$(basename "$f" .c); compile_one "$f" "$D/rt/$b.o" "-std=gnu11 -$OPT -ffp-contract=off -fno-fast-math -Wall -Wextra -I$RT" || exit 1; done
compile_one "$LIFT/harness/image.c" "$D/h/image.o" "-std=gnu11 -O1 -Wall -Wextra -I$RT -I$LIFT/harness" || exit 1
for f in "$HERE"/*.c; do b=$(basename "$f" .c); case "$b" in vst_guard) continue;; esac; compile_one "$f" "$D/h/$b.o" "-std=gnu11 -O1 -Wall -Wextra -I$RT -I$LIFT/harness -I$HERE -DVST_RM_NAME=\"\\\"$RMNAME\\\"\" -DVST_RM_BASE=$RMBASE" || exit 1; done
rm -f "$D/vst.js" "$D/vst.wasm"
( ulimit -v "$EMCC_VMEM_KB"; /usr/bin/time -f "link %e s %M KB" -o "$D/log/link.time" emcc -$OPT \
    -sALLOW_MEMORY_GROWTH=1 -sSTACK_SIZE=8MB -sGLOBAL_BASE="$GLOBAL_BASE" -sENVIRONMENT=node -sMODULARIZE=1 -sEXIT_RUNTIME=1 -sNODERAWFS=1 \
    -Wl,--initial-memory="$INITMEM" -Wl,--max-memory="$MAXMEM" \
    -Wl,--wrap=fle_imp_LoadLibraryExW -Wl,--wrap=fle_imp_LoadLibraryW -Wl,--wrap=fle_imp_LoadLibraryA -Wl,--wrap=fle_imp_GetModuleHandleExW -Wl,--wrap=fle_imp_CreateWindowExW \
    -Wl,--wrap=fle_imp_CreateFileW -Wl,--wrap=fle_imp_GetFileType -Wl,--wrap=fle_imp_FormatMessageW -Wl,--wrap=fle_imp_SetFilePointerEx -Wl,--wrap=fle_imp_LockResource \
    "$D"/h/*.o "$D"/obj/*.o "$D"/rt/*.o -lm -o "$D/vst.js" > "$D/log/link.log" 2>&1 )
echo "link exit=$?"; tail -1 "$D/log/link.time"; ls -la "$D"/vst.*
