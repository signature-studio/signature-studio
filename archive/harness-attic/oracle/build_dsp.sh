#!/bin/sh
# A5 oracle for the rebased, non-reduced dsp_ippv2_x64.dll (docs/investigations/a5-oracle-dsp.md). Same drivers, runtime and clang
# flags as the FLEngine build (a5head-build.sh in a5-oracle-rerun-3.md); only the generated C is the DLL's.
# Input: a generated C file made beforehand by
#   fe-pe rebase dsp_ippv2_x64.dll 0x2000000 REBASED.dll ; fe-lift lift REBASED.dll GEN.c --pdata
# Output (nothing outside $B is written): $B/gen/dsp-part-NN.c, $B/parts, $B/rt, $B/obj, $B/bin/{native_drv2,lift_drv2}.
# Env: B (build directory, required), GEN (generated C, required), CC (clang), JOBS (default 4), PER (functions per part, 800).
set -eu
HERE=$(cd "$(dirname "$0")" && pwd)
LIFT=$(cd "$HERE/../.." && pwd)
RT="$LIFT/runtime"
CC=${CC:-clang}
JOBS=${JOBS:-4}
PER=${PER:-800}
: "${B:?set B}" "${GEN:?set GEN}"
mkdir -p "$B/gen" "$B/parts" "$B/rt" "$B/obj" "$B/bin" "$B/tmp"
TMPDIR="$B/tmp"
export TMPDIR

[ -e "$B/gen/dsp-part-00.c" ] || python3 "$LIFT/harness/wasm/split_c.py" "$GEN" "$B/gen/dsp-part" "$PER"

ls "$B"/gen/dsp-part-*.c | xargs -P "$JOBS" -I{} sh -c '
    b=$(basename {} .c)
    [ -e "'"$B"'/parts/$b.o" ] || '"$CC"' -std=gnu11 -O0 -g -w -fno-builtin-fprintf -fno-builtin-fputs -ffp-contract=off -fPIE -I"'"$RT"'" -finstrument-functions -c {} -o "'"$B"'/parts/$b.o"
'
ls "$RT"/*.c | xargs -P "$JOBS" -I{} sh -c '
    b=$(basename {} .c)
    '"$CC"' -std=gnu11 -O0 -g -Wall -Wextra -fPIE -I"'"$RT"'" -c {} -o "'"$B"'/rt/$b.o"
'
for f in oracle2_common native_drv2 lift_drv2; do
    $CC -std=gnu11 -O1 -g -Wall -Wextra -fPIE -I"$RT" -I"$HERE" -c "$HERE/$f.c" -o "$B/obj/$f.v2.o"
done
$CC -c -fPIE "$HERE/ms_call2.S" -o "$B/obj/ms_call2.v2.o"
$CC -std=gnu11 -O1 -Wall -Wextra -fPIE -I"$LIFT/harness" -c "$LIFT/harness/image.c" -o "$B/obj/image.o"
$CC -pie -o "$B/bin/native_drv2" "$B/obj/native_drv2.v2.o" "$B/obj/oracle2_common.v2.o" "$B/obj/ms_call2.v2.o" "$B/obj/image.o"
# shellcheck disable=SC2086
$CC -pie -o "$B/bin/lift_drv2" "$B/obj/lift_drv2.v2.o" "$B/obj/oracle2_common.v2.o" "$B/obj/image.o" \
    "$B"/parts/dsp-part-*.o "$B"/rt/*.o -Wl,--wrap=fprintf -Wl,--wrap=fputs -lm
echo "built $B/bin/native_drv2 $B/bin/lift_drv2"
