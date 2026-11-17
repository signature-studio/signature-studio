#!/bin/bash
# usage: wb.sh OUTNAME VAR stage...    wrapper around the (modified) lift/harness/two/build_nmod_wasm.sh of /tmp/opencode/wasm6/srcw
W=/tmp/opencode/wasm6
OUTNAME=$1; VAR=$2; shift 2
export OUT=$W/$OUTNAME TAG=t2331d00e8ebb TOOLS=$W/tools VAR=$VAR
export REALMOD_C=$W/on/v-t2331d00e8ebb/native/realmod_table_gen.c
export EM_CACHE=$W/emcache JOBS=${JOBS:-2} SERIAL_MODS=${SERIAL_MODS:-m1} DELETE_SRC=${DELETE_SRC:-1}
exec bash $W/srcw/lift/harness/two/build_nmod_wasm.sh "$@"
