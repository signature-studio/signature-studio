#!/bin/bash
# emscripten pthreads build of smem.c (shared Memory, pool of 4 Workers). EM_FROZEN_CACHE: the shared emsdk cache is only read (the -mt libraries are prebuilt there).
set -e
D=$(cd "$(dirname "$0")" && pwd); OUT=${1:?out dir}
mkdir -p "$OUT"
export PATH=/opt/emsdk:/opt/emsdk/upstream/emscripten:$PATH EM_FROZEN_CACHE=1 TMPDIR=${TMPDIR:-/tmp/opencode/mt1}
mkdir -p "$TMPDIR"
emcc -O2 -pthread -sPTHREAD_POOL_SIZE=4 -sINITIAL_MEMORY=64MB -sENVIRONMENT=web,worker -sEXIT_RUNTIME=0 -sPROXY_TO_PTHREAD=1 "$D/smem.c" -o "$OUT/smem.js"
ls -la "$OUT"/smem.*
