#!/bin/sh
# Builds the A5 oracle drivers into lift/out-oracle/bin/.
#   native_drv : PE image + ms_call.S thunk (no generated code)
#   lift_drv   : same driver linked against the all-pdata split build (lift/out-pdata/parts/*.o,
#                read only) and a snapshot of its runtime objects (copied to out-oracle/rt/)
# Env: PARTS_NAME (default after-pdata-b), CC (default clang).
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

for f in oracle_common native_drv; do
    $CC -std=gnu11 -O1 -g -Wall -Wextra -fPIE -I"$RT" -I"$HERE" -c "$HERE/$f.c" -o "$OUT/obj/$f.o"
done
$CC -std=gnu11 -O1 -g -Wall -Wextra -fPIE -I"$RT" -I"$HERE" -c "$HERE/lift_drv.c" -o "$OUT/obj/lift_drv.o"
$CC -c -fPIE "$HERE/ms_call.S" -o "$OUT/obj/ms_call.o"
$CC -std=gnu11 -O1 -Wall -Wextra -fPIE -I"$LIFT/harness" -c "$LIFT/harness/image.c" -o "$OUT/obj/image.o"

# native_drv has no oracle_last_msg writer: the wrappers live in lift_drv.c only.
$CC -pie -o "$OUT/bin/native_drv" "$OUT/obj/native_drv.o" "$OUT/obj/oracle_common.o" "$OUT/obj/ms_call.o" \
    "$OUT/obj/image.o"

for o in "$PD"/fle_*.o; do
    [ -e "$OUT/rt/$(basename "$o")" ] || cp "$o" "$OUT/rt/"
done
# shellcheck disable=SC2086
$CC -pie -o "$OUT/bin/lift_drv" "$OUT/obj/lift_drv.o" "$OUT/obj/oracle_common.o" "$OUT/obj/image.o" \
    "$PD/parts/$NAME"-part-*.o "$OUT"/rt/fle_*.o -Wl,--wrap=fprintf -Wl,--wrap=fputs -lm
echo "built $OUT/bin/native_drv $OUT/bin/lift_drv"
