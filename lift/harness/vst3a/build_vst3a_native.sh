#!/bin/bash
# Native (clang) build of Serum2.vst3 lifted (decision 0165) + the runtime + the vst3a host harness. A modified copy of vst/build_vst_native.sh: the harness is
# vst/*.c without main_vst.c and vst_host.c, plus vst3a/*.c (main_vst3a.c, vst_host3a.c, vst3a_imp.c, ...) plus the generated stub file ($OUT/rows/vst3a_stubs.c).
# usage: OUT=lift/out-vst3a TAG=s2a FILE=<Serum2.vst3> OPT=O0|O2 BUILTIN=1 COUNT=0|1 JOBS=3 bash lift/harness/vst3a/build_vst3a_native.sh
#   COUNT=1: per-function counting units from $OUT/src-countf (count_fn.py) and -DFLE_COUNT
set -u -o pipefail
HERE=$(cd "$(dirname "$0")" && pwd)
LIFT=${LIFT:-$(cd "$HERE/../.." && pwd)}
VST=$LIFT/harness/vst
OUT=${OUT:?OUT is not set}
TAG=${TAG:-s2a}
OPT=${OPT:-O2}
BUILTIN=${BUILTIN:-1}
COUNT=${COUNT:-0}
SAN=${SAN:-0}
XCF=${XCF:-}      # extra compiler flags for the units, the runtime and the harness (e.g. -DFLE_MX_STATS=1: fast/slow counts of the MXCSR-exact float operations; the config gets the suffix -x)
export CC=${CC:-clang}
JOBS=${JOBS:-3}
RT="$LIFT/runtime"
CFG="$OPT-b$BUILTIN-c$COUNT"
[ -n "$XCF" ] && CFG="$CFG-x"
D="$OUT/native/$TAG-$CFG"
mkdir -p "$D/obj" "$D/rt" "$D/h" "$OUT/tmp"
export TMPDIR="$OUT/tmp"
OPTF="-$OPT"; [ "$OPT" = O0 ] && OPTF="-O0 -g"
SANF=""; [ "$SAN" = 1 ] && { SANF="-fsanitize=address -fno-omit-frame-pointer"; CFG="$CFG-asan"; }
BI="-fno-builtin"; [ "$BUILTIN" = 1 ] && BI="-fno-builtin-fprintf -fno-builtin-fputs"
CNT=""; [ "$COUNT" != 0 ] && CNT="-DFLE_COUNT=1"
mkdir -p "$D/shadow"
sed 's/ *__attribute__((noreturn))//' "$RT/fle_modules.h" > "$D/shadow/fle_modules.h"
GEN="-std=gnu11 $OPTF $SANF $XCF -w $BI -ffp-contract=off -fPIE -I$D/shadow -I$RT -I$VST -I$HERE $CNT"
SRCD="$OUT/src"
if [ "$COUNT" != 0 ]; then
    SRCD="$OUT/src-countf"
    if [ ! -e "$SRCD/countf_arrays.c" ]; then python3 "$HERE/count_fn.py" "$SRCD" "$OUT"/src/$TAG-[0-9][0-9].c || exit 1; fi
    [ -e "$SRCD/common-$TAG.c" ] || cp "$OUT/src/common-$TAG.c" "$SRCD/common-$TAG.c"
fi
ls "$SRCD"/$TAG-[0-9][0-9].c "$SRCD"/common-$TAG.c | xargs -P "$JOBS" -I{} bash -c 'b=$(basename {} .c); [ -e '"$D"'/obj/$b.o ] || $CC '"$GEN"' -c {} -o '"$D"'/obj/$b.o' || exit 1
[ "$COUNT" != 0 ] && { [ -e "$D/obj/countf_arrays.o" ] || $CC -std=gnu11 -O1 -fPIE -c "$SRCD/countf_arrays.c" -o "$D/obj/countf_arrays.o" || exit 1; }
for f in "$RT"/fle_*.c; do b=$(basename "$f" .c); [ "$D/rt/$b.o" -nt "$f" ] || $CC -std=gnu11 $OPTF $SANF $XCF -Wall -Wextra -fPIE -I"$RT" -c "$f" -o "$D/rt/$b.o" || exit 1; done
[ -e "$D/h/image.o" ] || $CC -std=gnu11 -O1 $SANF -Wall -Wextra -fPIE -I"$RT" -I"$LIFT/harness" -c "$LIFT/harness/image.c" -o "$D/h/image.o" || exit 1
FILE=${FILE:?FILE (the PE) is not set}
RMNAME=${RMNAME:-$(basename "$FILE" | tr 'A-Z' 'a-z')}
RMBASE=$(python3 -c "import struct,sys; d=open(sys.argv[1],'rb').read(0x400); l=struct.unpack_from('<I',d,0x3c)[0]; print(hex(struct.unpack_from('<Q',d,l+24+24)[0]))" "$FILE")
HF=""
for f in "$VST"/*.c; do b=$(basename "$f"); case $b in main_vst.c|vst_host.c|vst_env.c|hybrid_glue.c) continue;; esac; HF="$HF $f"; done
HF="$HF $HERE/main_vst3a.c $HERE/vst_host3a.c $HERE/vst_env3a.c $HERE/vst3a_imp.c $HERE/vst_stream3a.c $HERE/vst_msg3a.c $HERE/vst_exc3a.c $OUT/rows/vst3a_stubs.c"
for f in $HF; do b=$(basename "$f" .c); [ "$D/h/$b.o" -nt "$f" ] && [ "$D/h/$b.o" -nt "$HERE/vst3a.h" ] || $CC -std=gnu11 -O1 $SANF -Wall -Wextra -Wno-unused-function -Wno-unused-parameter -fPIE -I"$RT" -I"$LIFT/harness" -I"$VST" -I"$HERE" $CNT $XCF -DVST_RM_NAME="\"$RMNAME\"" -DVST_RM_BASE="$RMBASE" -c "$f" -o "$D/h/$b.o" || exit 1; done
rm -f "$D/vst-native"
$CC $SANF -pie -o "$D/vst-native" "$D"/h/*.o "$D"/obj/*.o "$D"/rt/*.o -Wl,--wrap=fle_imp_LoadLibraryExW -Wl,--wrap=fle_imp_LoadLibraryW -Wl,--wrap=fle_imp_LoadLibraryA -Wl,--wrap=fle_imp_GetModuleHandleExW -Wl,--wrap=fle_imp_CreateWindowExW -Wl,--wrap=fle_imp_CreateFileW -Wl,--wrap=fle_imp_GetFileType -Wl,--wrap=fle_imp_FormatMessageW -Wl,--wrap=fle_imp_LockResource -Wl,--wrap=fle_imp_SetFilePointerEx -Wl,--wrap=fle_imp_CreateThread $(cat "$HERE/wraps.txt" 2>/dev/null) -lm -lpthread -ldl && echo "built $D/vst-native"
