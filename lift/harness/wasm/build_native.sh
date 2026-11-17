#!/bin/bash
# Native (clang) build of the same generated parts + the current runtime + the native harness, for the comparison with
# the wasm run (docs/investigations/wasm-port-run.md). Reuses $OUT/out/wasm-pdata-part-*.c from build.sh (gen, split).
# Output: $OUT/native/ (nothing deleted or overwritten; a built object is skipped). Same flags as lift/out-loc/build-parts.sh.
set -u
HERE=$(cd "$(dirname "$0")" && pwd)
LIFT=$(cd "$HERE/../.." && pwd)
OUT=${OUT:-$LIFT/out-wasm2}
RT="$LIFT/runtime"
JOBS=${JOBS:-2}
mkdir -p "$OUT/tmp" "$OUT/native/obj" "$OUT/native/rt" "$OUT/native/h"
export TMPDIR="$OUT/tmp"
cd "$OUT/native"
ls "$OUT"/out/wasm-pdata-part-*.c | xargs -P "$JOBS" -I{} bash -c 'b=$(basename {} .c); [ -e obj/$b.o ] || clang -std=gnu11 -O0 -g -w -fno-builtin-fprintf -fno-builtin-fputs -ffp-contract=off -fPIE -I'"$RT"' -finstrument-functions -c {} -o obj/$b.o'
for f in "$RT"/fle_*.c; do b=$(basename "$f" .c); [ -e rt/$b.o ] || clang -std=gnu11 -O0 -g -Wall -Wextra -fPIE -I"$RT" -c "$f" -o rt/$b.o; done
for f in main image a4; do [ -e h/$f.o ] || clang -std=gnu11 -O1 -Wall -Wextra -fPIE -I"$RT" -I"$LIFT/harness" -c "$LIFT/harness/$f.c" -o h/$f.o; done
# the census harness (decision 0097): inert unless FLE_CENSUS is set
[ -e h/census.o ] || clang -std=gnu11 -O1 -Wall -Wextra -fPIE -I"$RT" -I"$LIFT/harness" -I"$LIFT/harness/census" -c "$LIFT/harness/census/census.c" -o h/census.o
# the native trace.c: harness/wasm/trace_native.c = the trace.c of the earlier native runs (out-loc/trace.c = out-pdata/trace.c)
# plus the A4 survey hooks (decision 0060); the report format is the one the wasm trace.c reproduces
TR=${NATIVE_TRACE:-$HERE/trace_native.c}
[ -e h/trace.o ] || clang -std=gnu11 -O1 -Wall -Wextra -fPIE -I"$RT" -I"$LIFT/harness" -c "$TR" -o h/trace.o
clang -pie -o flengine-native h/main.o h/image.o h/a4.o h/census.o h/trace.o obj/*.o rt/*.o -Wl,--wrap=fprintf -Wl,--wrap=fputs -lm && echo "built $OUT/native/flengine-native"
