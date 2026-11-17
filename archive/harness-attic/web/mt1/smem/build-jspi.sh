#!/bin/bash
set -e
D=$(cd "$(dirname "$0")" && pwd); OUT=${1:?out dir}
mkdir -p "$OUT"
export PATH=/opt/emsdk:/opt/emsdk/upstream/emscripten:$PATH EM_FROZEN_CACHE=1 TMPDIR=${TMPDIR:-/tmp/opencode/mt1}
emcc -O1 -pthread -sJSPI -sPTHREAD_POOL_SIZE=3 -sINITIAL_MEMORY=64MB -sENVIRONMENT=web,worker -sEXIT_RUNTIME=0 -sPROXY_TO_PTHREAD=1 "$D/jspi_pthread.c" -o "$OUT/jspi_pthread.js"
ls -la "$OUT"/jspi_pthread.*
