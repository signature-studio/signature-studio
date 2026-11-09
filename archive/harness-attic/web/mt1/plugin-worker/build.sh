#!/bin/bash
# mt1 (decision 0173), S1: wasm (emcc) build of the lifted OTT VST3 with the EXTERNAL-INPUT harness main_vst_ext.c. Same recipe as lift/harness/vst/build_vst_wasm.sh (decision 0156); only main_vst.c is replaced.
# The generated C units, the runtime and the harness sources are the ones the existing OTT wasm (decision 0164, lift/out-wasmgen2) was built from, read-only:
#   GEN  = the lifted OTT units    (default lift/out-wasmgen2/src: ottw-NN.c, common-ottw.c)
#   BASE = a lift/ tree whose runtime/ and harness/vst/ match them (default lift/out-wasmgen2/edit/lift; the HEAD tree also works for the harness, the runtime at HEAD has later changes)
#   PE   = the rebased PE           (default lift/out-wasmgen2/rebased/OTT.2000000.vst3)
# usage: OUT=lift/out-mt1/wasm-ext [OPT=O2] [JOBS=2] nice -n 15 bash build.sh      -> $OUT/vst-ext.wasm (the glue vst.js is not used: vst-host.js implements the imports)
set -u -o pipefail
export PYTHONDONTWRITEBYTECODE=1
HERE=$(cd "$(dirname "$0")" && pwd)
REPO=${REPO:-$(cd "$HERE/../../../../.." && pwd)}
OUT=${OUT:?OUT is not set}; OPT=${OPT:-O2}; JOBS=${JOBS:-2}
GEN=${GEN:-$REPO/lift/out-wasmgen2/src}; BASE=${BASE:-$REPO/lift/out-wasmgen2/edit/lift}; PE=${PE:-$REPO/lift/out-wasmgen2/rebased/OTT.2000000.vst3}
RT="$BASE/runtime"; VST="$BASE/harness/vst"
TAG=ottw
D="$OUT/build-$OPT"
EMCC_VMEM_KB=${EMCC_VMEM_KB:-12582912}
INITMEM=${INITMEM:-$((0x28000000))}; MAXMEM=${MAXMEM:-$((0x80000000))}; GLOBAL_BASE=${GLOBAL_BASE:-268435456}
mkdir -p "$D/obj" "$D/rt" "$D/h" "$D/log" "$D/shadow" "$OUT/tmp"
export PATH=/opt/emsdk:/opt/emsdk/upstream/emscripten:$PATH
export TMPDIR="$OUT/tmp" EM_FROZEN_CACHE=1   # the shared emsdk cache is only read (the libraries this link needs are prebuilt there)
BI="-fno-builtin-fprintf -fno-builtin-fputs"
sed 's/ *__attribute__((noreturn))//' "$RT/fle_modules.h" > "$D/shadow/fle_modules.h"
GENF="-std=gnu11 -$OPT -w $BI -ffp-contract=off -fno-fast-math -ferror-limit=0 -I$D/shadow -I$RT -I$VST"
RMNAME=ott.vst3
RMBASE=$(python3 -c "import struct,sys; d=open(sys.argv[1],'rb').read(0x400); l=struct.unpack_from('<I',d,0x3c)[0]; print(hex(struct.unpack_from('<Q',d,l+24+24)[0]))" "$PE")
compile_one() { [ "$2" -nt "$1" ] && return 0; ( ulimit -v "$EMCC_VMEM_KB"; /usr/bin/time -f "%e s %M KB" -o "$2.time" emcc -c $3 "$1" -o "$2" > "$2.log" 2>&1 ) || { echo "FAILED $1 (see $2.log)"; return 1; }; echo "$(basename "$1") $(tail -1 "$2.time")"; }
export -f compile_one; export EMCC_VMEM_KB
ls "$GEN"/$TAG-[0-9][0-9].c "$GEN"/common-$TAG.c | xargs -P "$JOBS" -I{} bash -c 'b=$(basename {} .c); compile_one {} '"$D"'/obj/$b.o "'"$GENF"'"' || exit 1
for f in "$RT"/fle_*.c; do b=$(basename "$f" .c); compile_one "$f" "$D/rt/$b.o" "-std=gnu11 -$OPT -ffp-contract=off -fno-fast-math -Wall -Wextra -I$RT" || exit 1; done
compile_one "$BASE/harness/image.c" "$D/h/image.o" "-std=gnu11 -O1 -Wall -Wextra -I$RT -I$BASE/harness" || exit 1
for f in "$VST"/*.c; do b=$(basename "$f" .c); case "$b" in vst_guard|main_vst) continue;; esac; compile_one "$f" "$D/h/$b.o" "-std=gnu11 -O1 -Wall -Wextra -I$RT -I$BASE/harness -I$VST -DVST_RM_NAME=\"\\\"$RMNAME\\\"\" -DVST_RM_BASE=$RMBASE" || exit 1; done
compile_one "$HERE/main_vst_ext.c" "$D/h/main_vst_ext.o" "-std=gnu11 -O1 -Wall -Wextra -I$RT -I$BASE/harness -I$VST -DVST_RM_NAME=\"\\\"$RMNAME\\\"\" -DVST_RM_BASE=$RMBASE" || exit 1
rm -f "$D/vst.js" "$D/vst.wasm"
( ulimit -v "$EMCC_VMEM_KB"; /usr/bin/time -f "link %e s %M KB" -o "$D/log/link.time" emcc -$OPT \
    -sERROR_ON_UNDEFINED_SYMBOLS=0 -sWARN_ON_UNDEFINED_SYMBOLS=0 -sALLOW_MEMORY_GROWTH=1 -sSTACK_SIZE=8MB -sGLOBAL_BASE="$GLOBAL_BASE" -sENVIRONMENT=node -sMODULARIZE=1 -sEXIT_RUNTIME=1 -sNODERAWFS=1 \
    -Wl,--initial-memory="$INITMEM" -Wl,--max-memory="$MAXMEM" \
    -Wl,--wrap=fle_imp_LoadLibraryExW -Wl,--wrap=fle_imp_LoadLibraryW -Wl,--wrap=fle_imp_LoadLibraryA -Wl,--wrap=fle_imp_GetModuleHandleExW -Wl,--wrap=fle_imp_CreateWindowExW \
    -Wl,--wrap=fle_imp_CreateFileW -Wl,--wrap=fle_imp_GetFileType -Wl,--wrap=fle_imp_FormatMessageW -Wl,--wrap=fle_imp_SetFilePointerEx -Wl,--wrap=fle_imp_LockResource \
    "$D"/h/*.o "$D"/obj/*.o "$D"/rt/*.o -lm -o "$D/vst.js" > "$D/log/link.log" 2>&1 )
echo "link exit=$?"; tail -1 "$D/log/link.time"
cp "$D/vst.wasm" "$OUT/vst-ext-$OPT.wasm" && ls -la "$OUT/vst-ext-$OPT.wasm"
