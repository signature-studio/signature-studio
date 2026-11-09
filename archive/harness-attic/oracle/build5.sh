#!/bin/sh
# A5 oracle, run5 build (decision 0038): same drivers as build2.sh, but the lifted code and the runtime objects
# are the ones built from the current lifter and runtime (MXCSR flags, PREFETCHh), in lift/out-mxcsr/oracle/.
# Nothing under out-oracle/ or out-pdata/ is written. native_drv2 is the unchanged run4 binary (copied).
# Input: out-mxcsr/pd/out/new-part-NN.c (fe-lift lift --pdata, then out-pdata/split_c.py, 3700 functions per part).
# Env: CC (clang), JOBS (16), PARTS_DIR (lift/out-mxcsr/pd/out), OUT5 (lift/out-mxcsr/oracle).
set -eu
HERE=$(cd "$(dirname "$0")" && pwd)
LIFT=$(cd "$HERE/../.." && pwd)
CC=${CC:-clang}
RT="$LIFT/runtime"
OUT5=${OUT5:-$LIFT/out-mxcsr/oracle}
PARTS_DIR=${PARTS_DIR:-$LIFT/out-mxcsr/pd/out}
mkdir -p "$OUT5/bin" "$OUT5/obj" "$OUT5/parts" "$OUT5/rt" "$OUT5/tmp"
TMPDIR="$OUT5/tmp"
export TMPDIR CC RT OUT5

ls "$PARTS_DIR"/new-part-*.c | xargs -P "${JOBS:-16}" -I{} sh -c '
    b=$(basename {} .c)
    $CC -std=gnu11 -O0 -g -w -fno-builtin-fprintf -fno-builtin-fputs -ffp-contract=off -fPIE -I"$RT" -finstrument-functions -c {} -o "$OUT5/parts/$b.o"
'
ls "$RT"/*.c | xargs -P "${JOBS:-16}" -I{} sh -c '
    b=$(basename {} .c)
    $CC -std=gnu11 -O0 -g -Wall -Wextra -fPIE -I"$RT" -c {} -o "$OUT5/rt/$b.o"
'
for f in oracle2_common lift_drv2; do
    $CC -std=gnu11 -O1 -g -Wall -Wextra -fPIE -I"$RT" -I"$HERE" -c "$HERE/$f.c" -o "$OUT5/obj/$f.v2.o"
done
IMG="$LIFT/out-oracle/obj/image.o"
[ -e "$IMG" ] || $CC -std=gnu11 -O1 -Wall -Wextra -fPIE -I"$LIFT/harness" -c "$LIFT/harness/image.c" -o "$OUT5/obj/image.o"
[ -e "$IMG" ] || IMG="$OUT5/obj/image.o"
cp "$LIFT/out-oracle/bin/native_drv2" "$OUT5/bin/native_drv2"
# shellcheck disable=SC2086
$CC -pie -o "$OUT5/bin/lift_drv2" "$OUT5/obj/lift_drv2.v2.o" "$OUT5/obj/oracle2_common.v2.o" "$IMG" \
    "$OUT5/parts"/new-part-*.o "$OUT5/rt"/*.o -Wl,--wrap=fprintf -Wl,--wrap=fputs -lm
echo "built $OUT5/bin/lift_drv2"
