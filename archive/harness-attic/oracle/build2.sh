#!/bin/sh
# Builds the A5 oracle v2 drivers into lift/out-oracle/bin/ (native_drv2, lift_drv2). Same inputs as
# build.sh (out-pdata parts and the runtime snapshot in out-oracle/rt, read only). Objects are
# *.v2.o so the v1 objects are not touched.
set -eu
HERE=$(cd "$(dirname "$0")" && pwd)
LIFT=$(cd "$HERE/../.." && pwd)
OUT="$LIFT/out-oracle"
PD="$LIFT/out-pdata"
NAME=${PARTS_NAME:-after-pdata-b}
CC=${CC:-clang}
RT="$LIFT/runtime"
mkdir -p "$OUT/bin" "$OUT/obj" "$OUT/rt" "$OUT/tmp" "$OUT/results"
TMPDIR="$OUT/tmp"
export TMPDIR

for f in oracle2_common native_drv2 lift_drv2; do
    $CC -std=gnu11 -O1 -g -Wall -Wextra -fPIE -I"$RT" -I"$HERE" -c "$HERE/$f.c" -o "$OUT/obj/$f.v2.o"
done
$CC -c -fPIE "$HERE/ms_call2.S" -o "$OUT/obj/ms_call2.v2.o"
[ -e "$OUT/obj/image.o" ] || $CC -std=gnu11 -O1 -Wall -Wextra -fPIE -I"$LIFT/harness" -c "$LIFT/harness/image.c" -o "$OUT/obj/image.o"

$CC -pie -o "$OUT/bin/native_drv2" "$OUT/obj/native_drv2.v2.o" "$OUT/obj/oracle2_common.v2.o" "$OUT/obj/ms_call2.v2.o" \
    "$OUT/obj/image.o"
for o in "$PD"/fle_*.o; do
    [ -e "$OUT/rt/$(basename "$o")" ] || cp "$o" "$OUT/rt/"
done
# shellcheck disable=SC2086
$CC -pie -o "$OUT/bin/lift_drv2" "$OUT/obj/lift_drv2.v2.o" "$OUT/obj/oracle2_common.v2.o" "$OUT/obj/image.o" \
    "$PD/parts/$NAME"-part-*.o "$OUT"/rt/fle_*.o -Wl,--wrap=fprintf -Wl,--wrap=fputs -lm
echo "built $OUT/bin/native_drv2 $OUT/bin/lift_drv2"
