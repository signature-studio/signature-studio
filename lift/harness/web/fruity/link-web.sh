#!/bin/bash
# usage: OBJ=<build_nmod_wasm.sh OBJ> bash link-web.sh OUTDIR [EXTRA LINK INPUTS...]
# Links the objects of that build for the browser (decision 0128; decision 0243: one configuration): ES module glue (-sEXPORT_ES6=1, an engine MODULE worker import()s it), no file system (-sFILESYSTEM=0: the page answers the engine's
# file reads from the pack bytes, decision 0221), JSPI and the wasm setjmp/longjmp, ONE shared memory with emscripten's pthread libc (Blend2D and the libc need pthread_mutex_* and pthread_rwlock_*; decision 0234), the thread-local
# register block (the C part exports __tls_base for the direct wasm), the file-dialog JS library (decision 0193).
# The extra inputs are objects, libraries and linker arguments that depend on the build (the Blend2D adapter, upstream Blend2D, the export list and the link map of the hybrid).
set -u -o pipefail
HERE=$(cd "$(dirname "$0")" && pwd)
LIFT=$(cd "$HERE/../../.." && pwd)
OBJ=${OBJ:?OBJ is not set}
OUTD=${1:?usage: OBJ=... link-web.sh OUTDIR [EXTRA...]}
shift
export PATH=/opt/emsdk:/opt/emsdk/upstream/emscripten:$PATH
export EM_CACHE=${EM_CACHE:-$OBJ/../emcache} TMPDIR=${TMPDIR:-$OBJ/../tmp}
[ ! -e "$OUTD/flengine6.wasm" ] || { echo "exists: $OUTD/flengine6.wasm"; exit 1; }
mkdir -p "$OUTD"
( shopt -s nullglob; ulimit -v 12582912; /usr/bin/time -f "link %e s %M KB maxrss" -o "$OUTD/link.time" emcc -O0 \
    -sALLOW_MEMORY_GROWTH=1 -sSTACK_SIZE=8MB -sGLOBAL_BASE=268435456 \
    -sENVIRONMENT=web,worker -sMODULARIZE=1 -sEXIT_RUNTIME=1 -sEXPORT_ES6=1 -sFILESYSTEM=0 \
    -sSUPPORT_LONGJMP=wasm -sJSPI \
    --js-library "$LIFT/harness/web/threads/fle_thread_lib.js" -sEXPORTED_FUNCTIONS=_main,_fle_wasm_thread_entry,_fle_host_post_input --js-library "$LIFT/harness/web/fruity/fle_idle_lib.js" -sEXPORTED_RUNTIME_METHODS=ENV,wasmExports,stackRestore \
    --js-library "$LIFT/harness/web/filedlg/fle_filedlg_lib.js" \
    -pthread -sPTHREAD_POOL_SIZE=0 -sINITIAL_MEMORY=671088640 -sMAXIMUM_MEMORY=2147483648 -Wl,--export-memory -Wl,--export=__tls_base \
    -Wl,--allow-undefined -sERROR_ON_UNDEFINED_SYMBOLS=0 -sWARN_ON_UNDEFINED_SYMBOLS=0 \
    -Wl,--wrap=__cyg_profile_func_enter -Wl,--wrap=fle_raster_wrote \
    -Wl,--wrap=fprintf -Wl,--wrap=fputs -Wl,--wrap=abort \
    "$@" \
    "$OBJ"/h-fruity/*.o "$OBJ"/mod/*.o "$OBJ"/fle/*.o "$OBJ"/rt/*.o -lm \
    -o "$OUTD/flengine6.js" > "$OUTD/link.log" 2>&1 )
echo "link exit=$?"; tail -1 "$OUTD/link.time"; ls -l "$OUTD/flengine6.wasm" "$OUTD/flengine6.js"; sha256sum "$OUTD/flengine6.wasm" "$OUTD/flengine6.js"
