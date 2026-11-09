#!/bin/bash
# usage: cps-build.sh F O [OUTDIR]   M-C' on synthetic code: the same call graph as base (wait = C counter, no suspension) and as cps (generator-made
# unwind/rewind). Writes OUTDIR/cps_{base,cps}_<F>-<O>.{js,wasm} (served at /build/) and prints the sizes.
HERE="$(cd "$(dirname "$0")" && pwd)"
F=$1; O=$2; OUT="${3:-$HERE/../../../../out-web2/mb}"
export PATH=/opt/emsdk:/opt/emsdk/upstream/emscripten:$PATH PYTHONDONTWRITEBYTECODE=1 EM_FROZEN_CACHE=1
export TMPDIR="${TMPDIR:-$OUT/../tmp}"; mkdir -p "$TMPDIR" "$OUT"
NODE=${NODE:-/home/ubuntu/.local/bin/node}
W="$OUT/../synth-cps"; mkdir -p "$W"
COMMON="-sMODULARIZE=1 -sEXPORT_NAME=SY -sENVIRONMENT=worker -sEXPORTED_FUNCTIONS=_entry,_cps_mode,_cps_set,_checksum,_waits,_main -sINITIAL_MEMORY=64MB -sSTACK_SIZE=1MB -sALLOW_MEMORY_GROWTH=0"
for v in base:2 cps:3; do
  name=${v%%:*}; mode=${v##*:}
  $NODE "$HERE/synth.mjs" "$W/c_${name}_$F.c" "$F" 1 ${MEANOPS:-60} ${CALLRATE:-0.08} ${DISP:-0.15} $mode 1 > /dev/null || exit 1
  /usr/bin/time -f "compile $name $O %e s %M KB" emcc -$O -w -c "$W/c_${name}_$F.c" -o "$W/c_${name}_$F-$O.o" || exit 1
  /usr/bin/time -f "link $name $O %e s %M KB" emcc -$O "$W/c_${name}_$F-$O.o" $COMMON -o "$OUT/cps_${name}_$F-$O.js" || exit 1
  printf '%s\t%s\n' "cps_${name}_$F-$O.wasm" "$(stat -c %s "$OUT/cps_${name}_$F-$O.wasm")"
  rm -f "$W/c_${name}_$F.c" "$W/c_${name}_$F-$O.o"
done
