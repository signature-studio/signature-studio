#!/bin/bash
# usage: synth-measure.sh F [OPT=O0|O2] [OUTDIR]   Asyncify code expansion / build cost on synthetic lifted-style code (mb/synth.mjs)
# writes one TSV row per variant to OUTDIR/synth-F<F>-<OPT>.tsv : variant, wasm bytes, link seconds, link max RSS KB
HERE="$(cd "$(dirname "$0")" && pwd)"
F=$1; O=${2:-O0}; OUT="${3:-$HERE/../../../../out-web2/synth}"
mkdir -p "$OUT"
export PATH=/opt/emsdk:/opt/emsdk/upstream/emscripten:$PATH PYTHONDONTWRITEBYTECODE=1 EM_FROZEN_CACHE=1
export TMPDIR="${TMPDIR:-$OUT/../tmp}"; mkdir -p "$TMPDIR"
NODE=${NODE:-/home/ubuntu/.local/bin/node}
T=/usr/bin/time
COMMON="-sMODULARIZE=1 -sEXPORT_NAME=SY -sENVIRONMENT=worker -sEXPORTED_FUNCTIONS=_entry,_main -sINITIAL_MEMORY=64MB -sSTACK_SIZE=1MB -sALLOW_MEMORY_GROWTH=0"
$NODE "$HERE/synth.mjs" "$OUT/s_sync_$F.c" "$F" 1 ${MEANOPS:-60} ${CALLRATE:-0.08} ${DISP:-0.15} 0 ${BUDGET:-0} > "$OUT/gen-$F.json" || exit 1
$NODE "$HERE/synth.mjs" "$OUT/s_async_$F.c" "$F" 1 ${MEANOPS:-60} ${CALLRATE:-0.08} ${DISP:-0.15} 1 ${BUDGET:-0} > /dev/null || exit 1
cat "$OUT/gen-$F.json"
R="$OUT/synth-F$F-$O.tsv"
: > "$R"
ts() { /usr/bin/time -f "%e\t%M" -o "$OUT/time.tmp" "$@" || return 1; cat "$OUT/time.tmp"; }
echo "compile $O sync: $( { ts emcc -$O -w -c "$OUT/s_sync_$F.c" -o "$OUT/s_sync_$F-$O.o"; } 2>&1 | tr '\t' ' ')"
echo "compile $O async: $( { ts emcc -$O -w -c "$OUT/s_async_$F.c" -o "$OUT/s_async_$F-$O.o"; } 2>&1 | tr '\t' ' ')"
link() { # name obj flags...
  local name=$1 obj=$2; shift 2
  local tm; tm=$( { ts emcc -$O "$obj" $COMMON "$@" -o "$OUT/l_${name}_$F-$O.js"; } 2>&1 | tail -1)
  printf '%s\t%s\t%s\n' "$name" "$(stat -c %s "$OUT/l_${name}_$F-$O.wasm")" "$tm" >> "$R"
}
link base "$OUT/s_sync_$F-$O.o"
link jspi "$OUT/s_async_$F-$O.o" -sJSPI -sJSPI_EXPORTS=entry
link asyncify "$OUT/s_async_$F-$O.o" -sASYNCIFY -sASYNCIFY_STACK_SIZE=1048576
link asyncify_ignore_indirect "$OUT/s_async_$F-$O.o" -sASYNCIFY -sASYNCIFY_STACK_SIZE=1048576 -sASYNCIFY_IGNORE_INDIRECT
cat "$R"
if [ -n "$KEEP_DIR" ]; then mkdir -p "$KEEP_DIR"; for v in base jspi asyncify asyncify_ignore_indirect; do cp "$OUT/l_${v}_$F-$O.js" "$KEEP_DIR/sy_${v}_$F-$O.js"; cp "$OUT/l_${v}_$F-$O.wasm" "$KEEP_DIR/sy_${v}_$F-$O.wasm"; done; fi
rm -f "$OUT/s_sync_$F.c" "$OUT/s_async_$F.c" "$OUT/s_sync_$F-$O.o" "$OUT/s_async_$F-$O.o" "$OUT"/l_*_"$F-$O".js "$OUT"/l_*_"$F-$O".wasm "$OUT/time.tmp"
