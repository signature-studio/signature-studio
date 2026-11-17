#!/bin/bash
# One turn of the wall loop (decision 0156): lift with the current rows/seeds under a new TAG, build -O0, run the harness. usage: iter.sh TAG [harness args]
set -u
HERE=$(cd "$(dirname "$0")" && pwd)
LIFT=$(cd "$HERE/../.." && pwd)
OUT=${OUT:?}; TAG=$1; shift
FILE=${FILE:?}
EXTRA="--extra-imports $HERE/imports-vst.txt"
SEEDS=${SEEDS:+--seeds $SEEDS}
bash "$HERE/mkrows.sh" "$OUT/log/host-rows.txt"
FEBIN=$OUT/target/release OUT=$OUT TAG=$TAG FILE=$FILE LIFT_ARGS="--pdata $SEEDS $EXTRA" ROWS=$OUT/log/host-rows.txt bash "$HERE/gen_vst.sh" | tail -3
OUT=$OUT TAG=$TAG OPT=${OPT:-O0} BUILTIN=1 COUNT=0 JOBS=${JOBS:-4} bash "$HERE/build_vst_native.sh" | tail -1
"$OUT/native/$TAG-${OPT:-O0}-b1-c0/vst-native" "$FILE" "$@"
