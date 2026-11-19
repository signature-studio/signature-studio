#!/bin/bash
# usage: [OPT=O2] [CC=clang] [EXTRA="-DFLE_MX_STATS"] [RT=<runtime dir>] build_diff.sh OUTDIR : builds mxfp_diff (decision 0143)
set -e
HERE=$(cd "$(dirname "$0")" && pwd)
LIFT=$(cd "$HERE/../.." && pwd)
RT=${RT:-$LIFT/runtime}
OUT=${1:?OUTDIR}; OPT=${OPT:-O2}; CC=${CC:-clang}
mkdir -p $OUT
F="-$OPT -std=gnu11 -ffp-contract=off -Wall -Wextra -Wno-unused-function ${EXTRA:-}"
I="-I$RT -I$HERE -I$HERE/ref -I$LIFT/crates/fe-lift/tests/ref"
$CC $F $I -DFLE_MX_STATS -c $HERE/impl_new.c -o $OUT/impl_new.o &
$CC $F $I -DFLE_MX_NOFAST -c $HERE/impl_nofast.c -o $OUT/impl_nofast.o &
$CC $F $I -c $HERE/impl_old.c -o $OUT/impl_old.o &
HOSTOBJ=""
if [ "$(uname -m)" = x86_64 ] && [ "${NOHOST:-0}" != 1 ]; then
  $CC $F $I -c $HERE/impl_host.c -o $OUT/impl_host.o &
  HOSTOBJ=$OUT/impl_host.o
fi
$CC $F $I -DFLE_MX_STATS -c $HERE/mxfp_diff.c -o $OUT/mxfp_diff.o &
wait
if [ -z "$HOSTOBJ" ]; then
  echo 'const struct MxfpOp {int x;} MXFP_OPS_host[] = {{0}}; const unsigned MXFP_N_host = 0;' > $OUT/nohost.c
  $CC -c $OUT/nohost.c -o $OUT/nohost.o; HOSTOBJ=$OUT/nohost.o
fi
$CC $OUT/mxfp_diff.o $OUT/impl_new.o $OUT/impl_nofast.o $OUT/impl_old.o $HOSTOBJ -o $OUT/mxfp_diff -lm
echo "built $OUT/mxfp_diff"
