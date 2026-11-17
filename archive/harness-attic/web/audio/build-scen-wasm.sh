#!/bin/bash
# usage: build-scen-wasm.sh OUTDIR [RT_OPT=-O0] : builds harness/audio/ds_scenarios.c for the browser (emcc, JSPI, cooperative threads switched by JSPI as in the FRUITY build): the real runtime files at -O0 (as the engine's runtime objects are)
# and the scenario at -O1. Output: OUTDIR/audio_scen.js (+ .wasm), a MODULARIZE factory createScen(). decision 0139 (PROVISIONAL).
set -u -o pipefail
HERE=$(cd "$(dirname "$0")" && pwd)
LIFT=$(cd "$HERE/../../.." && pwd)
RT=$LIFT/runtime
OUTD=${1:?usage: build-scen-wasm.sh OUTDIR}
RTOPT=${RT_OPT:--O0}
export PATH=/opt/emsdk:/opt/emsdk/upstream/emscripten:$PATH
mkdir -p "$OUTD/obj"
FILES="fle_thread.c fle_thread_wasm.c fle_win.c fle_class.c fle_event.c fle_sys.c fle_imports.c fle_proc.c fle_callback.c fle_env.c fle_vfs.c fle_vmap.c fle_vol.c fle_host.c fle_time.c fle_x87ctl.c fle_mxcsr.c fle_x87.c fle_locale.c fle_dsound.c fle_audio_ring.c fle_audio_transport.c"
FL="-std=gnu11 -ffp-contract=off -fno-fast-math -Wall -Wextra -I$RT -sSUPPORT_LONGJMP=wasm"
objs=""
for f in $FILES; do
  o="$OUTD/obj/${f%.c}.o"
  emcc $RTOPT $FL -c "$RT/$f" -o "$o" 2> "$OUTD/obj/${f%.c}.log" || { echo "emcc failed for $f"; tail -5 "$OUTD/obj/${f%.c}.log"; exit 1; }
  objs="$objs $o"
done
emcc -O1 -w $FL -c "$LIFT/harness/audio/ds_scenarios.c" -o "$OUTD/obj/ds_scenarios.o" || exit 1
emcc -O1 -sALLOW_MEMORY_GROWTH=1 -sSTACK_SIZE=8MB -sGLOBAL_BASE=268435456 -Wl,--initial-memory=$((0x12000000)) -Wl,--max-memory=$((0x40000000)) -sENVIRONMENT=web,worker -sMODULARIZE=1 -sEXPORT_NAME=createScen -sEXIT_RUNTIME=1 -sFORCE_FILESYSTEM=1 -sSUPPORT_LONGJMP=wasm -sJSPI \
  --js-library "$LIFT/harness/web/threads/fle_thread_lib.js" -sEXPORTED_FUNCTIONS=_main,_fle_wasm_thread_entry -sEXPORTED_RUNTIME_METHODS=wasmExports,stackRestore \
  $objs "$OUTD/obj/ds_scenarios.o" -lm -o "$OUTD/audio_scen.js" > "$OUTD/link.log" 2>&1
echo "link exit=$?"; tail -3 "$OUTD/link.log"; ls -la "$OUTD"/audio_scen.*
