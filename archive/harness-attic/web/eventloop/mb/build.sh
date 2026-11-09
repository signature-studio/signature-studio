#!/bin/bash
# usage: build.sh [OUTDIR]   builds mb.c in 3 modes x 2 optimisation levels (sizes in OUTDIR/sizes.tsv). Needs /opt/emsdk (no network).
# EM_FROZEN_CACHE=1: the emscripten cache under /opt is only read, never written.
HERE="$(cd "$(dirname "$0")" && pwd)"
OUT="${1:-$HERE/../../../../out-web2/mb}"
mkdir -p "$OUT"
export PATH=/opt/emsdk:/opt/emsdk/upstream/emscripten:$PATH PYTHONDONTWRITEBYTECODE=1 EM_FROZEN_CACHE=1
export TMPDIR="${TMPDIR:-$OUT/../tmp}"
mkdir -p "$TMPDIR"
COMMON="-sMODULARIZE=1 -sEXPORT_NAME=MB -sENVIRONMENT=worker -sEXPORTED_RUNTIME_METHODS=ccall,HEAP32,HEAPF64 -sEXPORTED_FUNCTIONS=_loop_echo,_bench_yield,_rec_depth,_main -sALLOW_MEMORY_GROWTH=0 -sINITIAL_MEMORY=16MB -sSTACK_SIZE=4MB"
: > "$OUT/sizes.tsv"
for O in O0 O2; do
  emcc -$O "$HERE/mb.c" -DMODE_SYNC $COMMON -o "$OUT/mb_sync_$O.js" || exit 1
  emcc -$O "$HERE/mb.c" -sASYNCIFY -sASYNCIFY_STACK_SIZE=2097152 $COMMON -o "$OUT/mb_asyncify_$O.js" || exit 1
  emcc -$O "$HERE/mb.c" -sJSPI -sJSPI_EXPORTS=loop_echo,bench_yield,rec_depth $COMMON -o "$OUT/mb_jspi_$O.js" || exit 1
done
for f in "$OUT"/mb_*.wasm; do printf '%s\t%s\n' "$(basename "$f")" "$(stat -c %s "$f")" >> "$OUT/sizes.tsv"; done
cat "$OUT/sizes.tsv"
