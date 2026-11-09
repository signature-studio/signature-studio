#!/bin/bash
# usage: [OPT=O2] build_diff_wasm.sh OUTDIR : the differential test as WebAssembly (emcc, run with node): new, nofast and old only (no host CPU).
set -e
HERE=$(cd "$(dirname "$0")" && pwd)
LIFT=$(cd "$HERE/../.." && pwd)
RT=${RT:-$LIFT/runtime}
OUT=${1:?OUTDIR}; OPT=${OPT:-O2}
export PATH=/opt/emsdk:/opt/emsdk/upstream/emscripten:$PATH
export EM_CACHE=${EM_CACHE:-$OUT/emcache}
mkdir -p $OUT
F="-$OPT -std=gnu11 -ffp-contract=off -Wall -Wextra -Wno-unused-function ${EXTRA:-}"
I="-I$RT -I$HERE -I$HERE/ref"
emcc $F $I -DFLE_MX_STATS -c $HERE/impl_new.c -o $OUT/impl_new.o &
emcc $F $I -DFLE_MX_NOFAST -c $HERE/impl_nofast.c -o $OUT/impl_nofast.o &
emcc $F $I -c $HERE/impl_old.c -o $OUT/impl_old.o &
emcc $F $I -DFLE_MX_STATS -c $HERE/mxfp_diff.c -o $OUT/mxfp_diff.o &
wait
echo 'const struct MxfpOp {int x;} MXFP_OPS_host[] = {{0}}; const unsigned MXFP_N_host = 0;' > $OUT/nohost.c
emcc -c $OUT/nohost.c -o $OUT/nohost.o
emcc -$OPT $OUT/mxfp_diff.o $OUT/impl_new.o $OUT/impl_nofast.o $OUT/impl_old.o $OUT/nohost.o -o $OUT/mxfp_diff.js -sALLOW_MEMORY_GROWTH=1 -sEXIT_RUNTIME=1
echo "built $OUT/mxfp_diff.js"
