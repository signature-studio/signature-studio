#!/bin/bash
# N-module native build: the units made by gen_two.sh (FLEngine + the rebased real modules, shared R/FL/X, one merged dispatch
# table) + the current runtime (all fle_*.c, including fle_realmod.c) + a realmod_table.c that lists every real module + the
# harness main_two.c.
# Flags are those of lift/harness/wasm/build_native.sh. The native trace is harness/wasm/trace_native.c (tracked; the *.c.new files
# beside it belong to someone else and are not used).
#
# The modules come from $V/modules.txt (written by gen_two.sh; "label|file|base|..." entries). The units of module L are $V/src/L-NN.c
# and are compiled to $V/native/obj/L-NN.o. realmod_table.c is generated into $V/native/realmod_table_gen.c from that list (name =
# lower-case basename of the file, as the guest's LoadLibraryW spells it after normalisation; base = the entry's base) and compiled
# to $V/native/obj-h/realmod_table.o, so the table is per TAG, not shared. The bases are measurement values of the caller.
#
# usage: bash lift/harness/two/build_two_native.sh
# env:   OUT=lift/out-stage2/two (the OUT of gen_two.sh)   TAG=red (the TAG of gen_two.sh)   JOBS=6   NATIVE_TRACE=<trace c file>
#        LIFT=<tree with runtime/ and harness/>   (default: the lift/ above this script's directory)
#        RMNAMES="label=name;label=name"   the table name of a module when it is not the lower-case file basename
#                                          (the label "dll" defaults to dsp_ippv2_x64.dll, as the one-module script)
#        FRUITY=1   the stage-1 harness of CreateFruityInstance (docs/investigations/create-fruity-instance-harness.md): main_two.c is compiled
#                   with -DFLE_FRUITY=1 into $OUT/native/h-fruity/, lift/harness/fruity.c is compiled and linked, and the link adds
#                   -Wl,--wrap=__cyg_profile_func_enter. The merged common.c of the TAG must hold the stub rows (gen_two.sh FRUITY_ROWS=...;
#                   a4-multi.sh does both for FLE_CALL_FRUITY=1 and gives such a build its own TAG). Unset or 0: every command below is the one
#                   of the version of this script without the variable.
#        IDLE=1     (with FRUITY=1; decision 0129) the stable-idle detector of the idle continuation (lift/harness/idle/idle.h): fruity.c is compiled with -DFLE_IDLE=1 into $V/native/idle/, idle.c with -DFLE_IDLE=1, and the link
#                   adds one -Wl,--wrap=fle_imp_<API> per stub of the runtime (idle/gen_idle_wraps.py; the wrappers only count). It makes $V/native/two-native-idle and does not touch $V/native/two-native. a4-multi.sh does
#                   not use it: lift/out-idle1/tools does. Unset or 0: every command below is the one of the version of this script without the variable.
#        B2D_PROBE=1   the X1 probe of Blend2D (docs/investigations/blend2d-jit-and-drawing-path.md): main_two.c is compiled with -DFLE_B2D_PROBE=1 into
#                   $OUT/native/h-b2d/, lift/harness/b2d_probe.c is compiled and linked, and the link wraps fle_imp_VirtualAlloc, _VirtualFree, _VirtualProtect,
#                   _CreateFileMappingW, _MapViewOfFile and _UnmapViewOfFile (-Wl,--wrap=...) so that the probe sees the memory calls of the guest. a4-multi.sh
#                   does it for FLE_BLEND2D_PROBE=1 and gives such a build its own TAG. Unset or 0: every command below is the one of the version of this script
#                   without the variable. Not combined with FRUITY=1.
#        B2D_SCAN=1    the X2 scan of the Agg2D entries (docs/investigations/blend2d-nojit-slots.md): main_two.c is compiled with -DFLE_B2D_SCAN=1 into
#                   $OUT/native/h-scan/ and lift/harness/b2d_scan.c is compiled and linked (no --wrap, no signal handler: a stop is the runtime's own). a4-multi.sh
#                   does it for FLE_BLEND2D_SCAN=1 and gives such a build its own TAG. Unset or 0: every command below is the one of the version of this script
#                   without the variable. Not combined with FRUITY=1 or B2D_PROBE=1.
# Output: $OUT/v-$TAG/native/two-native. The FLEngine objects ($OUT/native/fle), the runtime objects ($OUT/native/rt) and the
# harness objects ($OUT/native/h) are shared by every TAG; a TAG has its own objects of the real modules and of common.c.
# Nothing is deleted or overwritten: a built object is skipped (move it away to rebuild it, e.g. the runtime objects after a
# runtime edit: the script does not detect a stale object).
set -u -o pipefail
HERE=$(cd "$(dirname "$0")" && pwd)
LIFT=${LIFT:-$(cd "$HERE/../.." && pwd)}
OUT=${OUT:-$LIFT/out-stage2/two}
TAG=${TAG:-red}
V="$OUT/v-$TAG"
RT="$LIFT/runtime"
JOBS=${JOBS:-6}
TR=${NATIVE_TRACE:-$LIFT/harness/wasm/trace_native.c}
RMNAMES=${RMNAMES:-}
FRUITY=${FRUITY:-0}
case "$FRUITY" in 0|1) ;; *) echo "build: FRUITY must be 0 or 1 (got '$FRUITY')"; exit 2 ;; esac
B2D_PROBE=${B2D_PROBE:-0}
case "$B2D_PROBE" in 0|1) ;; *) echo "build: B2D_PROBE must be 0 or 1 (got '$B2D_PROBE')"; exit 2 ;; esac
[ "$FRUITY$B2D_PROBE" != 11 ] || { echo "build: FRUITY=1 and B2D_PROBE=1 together are not supported"; exit 2; }
B2D_SCAN=${B2D_SCAN:-0}
case "$B2D_SCAN" in 0|1) ;; *) echo "build: B2D_SCAN must be 0 or 1 (got '$B2D_SCAN')"; exit 2 ;; esac
[ "$B2D_SCAN" = 0 ] || [ "$FRUITY$B2D_PROBE" = 00 ] || { echo "build: B2D_SCAN=1 is not combined with FRUITY=1 or B2D_PROBE=1"; exit 2; }
IDLE=${IDLE:-0}
case "$IDLE" in 0|1) ;; *) echo "build: IDLE must be 0 or 1 (got '$IDLE')"; exit 2 ;; esac
[ "$IDLE" = 0 ] || [ "$FRUITY" = 1 ] || { echo "build: IDLE=1 needs FRUITY=1"; exit 2; }
[ -r "$V/modules.txt" ] || { echo "build: $V/modules.txt is missing (run gen_two.sh gen first)"; exit 2; }
mkdir -p "$OUT/tmp" "$OUT/native/fle" "$OUT/native/rt" "$OUT/native/h" "$V/native/obj" "$V/native/obj-h"
export TMPDIR="$OUT/tmp"
# decision 0188 (opt-in, a speed measurement): GEN_OPT replaces "-O0 -g" and GEN_INSTR replaces "-finstrument-functions" (empty = none) for the generated C; unset = the flags of every earlier build
GEN="-std=gnu11 ${GEN_OPT:--O0 -g} -w -fno-builtin-fprintf -fno-builtin-fputs -ffp-contract=off -fPIE -I$RT ${GEN_INSTR--finstrument-functions}"

# the realmod table of this TAG, from modules.txt
RT_C="$V/native/realmod_table_gen.c"
if [ ! -e "$RT_C" ]; then
    {
        echo '/* generated by lift/harness/two/build_two_native.sh from modules.txt: the link-time registration of the real modules'
        echo ' * (lift/runtime/fle_realmod.h). Name = the spelling that normalize() of fle_load.c produces; base = the base the file was rebased to. */'
        echo '#include "fle_realmod.h"'
        echo 'const fle_realmod_entry fle_realmod_table[] = {'
        n=0
        while IFS='|' read -r label file base largs sha per; do
            [ -n "$label" ] || continue
            name=""
            IFS=';' read -ra pairs <<< "$RMNAMES"
            for p in "${pairs[@]:-}"; do [ "${p%%=*}" = "$label" ] && name="${p#*=}"; done
            if [ -z "$name" ]; then
                if [ "$label" = dll ]; then name=dsp_ippv2_x64.dll; else name=$(basename "$file" | tr 'A-Z' 'a-z'); fi
            fi
            printf '    { "%s", UINT64_C(%s) },\n' "$name" "$base"
            n=$((n + 1))
        done < "$V/modules.txt"
        echo '};'
        echo "const uint64_t fle_realmod_count = $n;"
    } > "$RT_C"
fi

cd "$OUT/native"
ls "$OUT"/src/fle-[0-9][0-9].c | xargs -P "$JOBS" -I{} bash -c 'b=$(basename {} .c); [ -e fle/$b.o ] || clang '"$GEN"' -c {} -o fle/$b.o' || exit 1
for f in "$RT"/fle_*.c; do b=$(basename "$f" .c); [ -e rt/$b.o ] || clang -std=gnu11 -O0 -g -Wall -Wextra -fPIE -I"$RT" -c "$f" -o rt/$b.o || exit 1; done
for f in image a4; do [ -e h/$f.o ] || clang -std=gnu11 -O1 -Wall -Wextra -fPIE -I"$RT" -I"$LIFT/harness" -c "$LIFT/harness/$f.c" -o h/$f.o || exit 1; done
[ -e h/main_two.o ] || clang -std=gnu11 -O1 -Wall -Wextra -fPIE -I"$RT" -I"$LIFT/harness" -c "$HERE/main_two.c" -o h/main_two.o || exit 1
[ -e h/trace.o ] || clang -std=gnu11 -O1 -Wall -Wextra -fPIE -I"$RT" -I"$LIFT/harness" -c "$TR" -o h/trace.o || exit 1
if [ "$FRUITY" = 1 ]; then
    mkdir -p h-fruity
    [ -e h-fruity/main_two.o ] || clang -std=gnu11 -O1 -Wall -Wextra -fPIE -DFLE_FRUITY=1 -I"$RT" -I"$LIFT/harness" -c "$HERE/main_two.c" -o h-fruity/main_two.o || exit 1
    [ -e h-fruity/fruity.o ] || clang -std=gnu11 -O1 -Wall -Wextra -fPIE -I"$RT" -I"$LIFT/harness" -c "$LIFT/harness/fruity.c" -o h-fruity/fruity.o || exit 1
    # decision 0136: the G3 input script and snapshot (native idle hook)
    [ -e h-fruity/fruity_g3.o ] || clang -std=gnu11 -O1 -Wall -Wextra -fPIE -I"$RT" -I"$LIFT/harness" -c "$LIFT/harness/fruity_g3.c" -o h-fruity/fruity_g3.o || exit 1
fi
if [ "$B2D_PROBE" = 1 ]; then
    mkdir -p h-b2d
    [ -e h-b2d/main_two.o ] || clang -std=gnu11 -O1 -Wall -Wextra -fPIE -DFLE_B2D_PROBE=1 -I"$RT" -I"$LIFT/harness" -c "$HERE/main_two.c" -o h-b2d/main_two.o || exit 1
    [ -e h-b2d/b2d_probe.o ] || clang -std=gnu11 -O1 -Wall -Wextra -fPIE -I"$RT" -I"$LIFT/harness" -c "$LIFT/harness/b2d_probe.c" -o h-b2d/b2d_probe.o || exit 1
fi
if [ "$B2D_SCAN" = 1 ]; then
    mkdir -p h-scan
    [ -e h-scan/main_two.o ] || clang -std=gnu11 -O1 -Wall -Wextra -fPIE -DFLE_B2D_SCAN=1 -I"$RT" -I"$LIFT/harness" -c "$HERE/main_two.c" -o h-scan/main_two.o || exit 1
    [ -e h-scan/b2d_scan.o ] || clang -std=gnu11 -O1 -Wall -Wextra -Wno-unused-function -Wno-unused-parameter -fPIE -I"$RT" -I"$LIFT/harness" -c "$LIFT/harness/b2d_scan.c" -o h-scan/b2d_scan.o || exit 1
fi
cd "$V/native"
[ -e obj-h/realmod_table.o ] || clang -std=gnu11 -O1 -Wall -Wextra -fPIE -I"$RT" -c "$RT_C" -o obj-h/realmod_table.o || exit 1
[ -e obj/common.o ] || clang $GEN -c "$V/src/common.c" -o obj/common.o || exit 1
# the units of every module: any file $V/src/<label>-NN.c (NN two digits); common.c and the *-tables.txt files do not match
ls "$V"/src/*-[0-9][0-9].c | xargs -P "$JOBS" -I{} bash -c 'b=$(basename {} .c); [ -e obj/$b.o ] || clang '"$GEN"' -c {} -o obj/$b.o' || exit 1
if [ "$IDLE" = 1 ]; then
    # The idle binary is $V/native/two-native-idle (decision 0129): the objects of the FRUITY build are shared, fruity.c is compiled with -DFLE_IDLE=1 and linked with idle.c and the counting wrappers of every stub.
    mkdir -p "$V/native/idle"
    nm -g --defined-only "$OUT"/native/rt/*.o | awk '$2 == "T" && $3 ~ /^fle_imp_/ { print $3 }' | LC_ALL=C sort -u > "$V/native/idle/symbols.txt"
    rm -f "$V/native/idle/idle_wraps.c" "$V/native/idle/idle_wrap_flags.txt"
    python3 "$LIFT/harness/idle/gen_idle_wraps.py" "$V/native/idle/symbols.txt" "$V/native/idle" || exit 1
    clang -std=gnu11 -O0 -Wall -Wextra -fPIE -I"$RT" -I"$LIFT/harness" -c "$V/native/idle/idle_wraps.c" -o "$V/native/idle/idle_wraps.o" || exit 1
    clang -std=gnu11 -O1 -Wall -Wextra -fPIE -DFLE_IDLE=1 -I"$RT" -I"$LIFT/harness" -c "$LIFT/harness/fruity.c" -o "$V/native/idle/fruity.o" || exit 1
    clang -std=gnu11 -O1 -Wall -Wextra -fPIE -DFLE_IDLE=1 -I"$RT" -I"$LIFT/harness" -c "$LIFT/harness/idle/idle.c" -o "$V/native/idle/idle.o" || exit 1
    rm -f two-native-idle
    clang -pie -o two-native-idle "$V/native/idle/fruity.o" "$V/native/idle/idle.o" "$V/native/idle/idle_wraps.o" "$OUT"/native/h-fruity/main_two.o "$OUT"/native/h/image.o \
        "$OUT"/native/h/a4.o "$OUT"/native/h/trace.o obj-h/realmod_table.o obj/*.o "$OUT"/native/fle/*.o "$OUT"/native/rt/*.o -Wl,--wrap=fprintf -Wl,--wrap=fputs \
        -Wl,--wrap=__cyg_profile_func_enter $(cat "$V/native/idle/idle_wrap_flags.txt") -lm \
        && echo "built $V/native/two-native-idle (FRUITY=1 IDLE=1: stable-idle detector)"
    exit $?
fi
[ -e two-native ] && { echo "exists: $V/native/two-native (move it away to relink)"; exit 0; }
if [ "$FRUITY" = 1 ]; then
    clang -pie -o two-native "$OUT"/native/h-fruity/main_two.o "$OUT"/native/h-fruity/fruity.o "$OUT"/native/h-fruity/fruity_g3.o "$OUT"/native/h/image.o "$OUT"/native/h/a4.o "$OUT"/native/h/trace.o \
        obj-h/realmod_table.o obj/*.o "$OUT"/native/fle/*.o "$OUT"/native/rt/*.o -Wl,--wrap=fprintf -Wl,--wrap=fputs -Wl,--wrap=__cyg_profile_func_enter -lm \
        && echo "built $V/native/two-native (FRUITY=1: stage-1 harness of CreateFruityInstance)"
elif [ "$B2D_SCAN" = 1 ]; then
    clang -pie -o two-native "$OUT"/native/h-scan/main_two.o "$OUT"/native/h-scan/b2d_scan.o "$OUT"/native/h/image.o "$OUT"/native/h/a4.o "$OUT"/native/h/trace.o \
        obj-h/realmod_table.o obj/*.o "$OUT"/native/fle/*.o "$OUT"/native/rt/*.o -Wl,--wrap=fprintf -Wl,--wrap=fputs -lm \
        && echo "built $V/native/two-native (B2D_SCAN=1: X2 scan of the Agg2D entries)"
elif [ "$B2D_PROBE" = 1 ]; then
    clang -pie -o two-native "$OUT"/native/h-b2d/main_two.o "$OUT"/native/h-b2d/b2d_probe.o "$OUT"/native/h/image.o "$OUT"/native/h/a4.o "$OUT"/native/h/trace.o \
        obj-h/realmod_table.o obj/*.o "$OUT"/native/fle/*.o "$OUT"/native/rt/*.o -Wl,--wrap=fprintf -Wl,--wrap=fputs \
        -Wl,--wrap=fle_imp_VirtualAlloc -Wl,--wrap=fle_imp_VirtualFree -Wl,--wrap=fle_imp_VirtualProtect -Wl,--wrap=fle_imp_CreateFileMappingW \
        -Wl,--wrap=fle_imp_MapViewOfFile -Wl,--wrap=fle_imp_UnmapViewOfFile -lm \
        && echo "built $V/native/two-native (B2D_PROBE=1: X1 probe of Blend2D)"
else
clang -pie -o two-native "$OUT"/native/h/main_two.o "$OUT"/native/h/image.o "$OUT"/native/h/a4.o "$OUT"/native/h/trace.o \
    obj-h/realmod_table.o obj/*.o "$OUT"/native/fle/*.o "$OUT"/native/rt/*.o -Wl,--wrap=fprintf -Wl,--wrap=fputs -lm \
    && echo "built $V/native/two-native"
fi
