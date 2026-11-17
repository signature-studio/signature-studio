#!/bin/sh
# Native build of the fe-lift harness (clang, x86_64 Linux only).
# Output goes to lift/out-harness/. Inputs: lift/out-dispatch/after.c (read only),
# lift/runtime/*.h (read only), lift/harness/*.c.
set -eu

HERE=$(cd "$(dirname "$0")" && pwd)
LIFT=$(cd "$HERE/.." && pwd)
OUT="$LIFT/out-harness"
SRC="$LIFT/out-dispatch/after.c"
RT="$LIFT/runtime"
CC=${CC:-clang}

mkdir -p "$OUT"

# Generated code: -O0 so that every f_ function keeps its own entry hook.
# -ffp-contract=off as required by fle_sse.h. The hook also sees runtime/ inline
# helpers; trace.c counts only entries listed in fle_dispatch_table.
$CC -std=gnu11 -O0 -g -w -fno-builtin-fprintf -fno-builtin-fputs -ffp-contract=off -fPIE -I"$RT" \
    -finstrument-functions -c "$SRC" -o "$OUT/after.o"

for f in main image trace; do
    $CC -std=gnu11 -O1 -Wall -Wextra -fPIE -I"$RT" -c "$HERE/$f.c" -o "$OUT/$f.o"
done

$CC -pie -o "$OUT/flengine_harness" "$OUT/main.o" "$OUT/image.o" "$OUT/trace.o" "$OUT/after.o" \
    -Wl,--wrap=fprintf -Wl,--wrap=fputs -lm

echo "built $OUT/flengine_harness"
