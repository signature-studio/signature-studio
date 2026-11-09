#!/bin/bash
# Native (clang) build of ONE lifted PE (a third-party VST3 plugin; decision 0156) + the runtime + the VST3 host harness main_vst.c.
# The generated C is the one `fe-lift lift FILE gen/ott.c --pdata [...]` writes (not changed); gen_vst.sh splits it and makes common.c.
# usage: OUT=lift/out-vst2 TAG=ott OPT=O2 BUILTIN=1 COUNT=0 JOBS=3 bash lift/harness/vst/build_vst_native.sh
#   OPT=O0|O2   optimisation of the generated units, the runtime and the harness (O0 also gives -g)
#   BUILTIN=1   compiler builtins on (decision 0151 direction); 0 = -fno-builtin (the flag of the production builds at HEAD 0991f92)
#   COUNT=1     -DFLE_COUNT: the opt-in instruction counter (fle_count.h; one add per basic block). Off: the generated C is the one it always was.
# Output: $OUT/native/$CFG/vst-native with CFG=$OPT-b$BUILTIN-c$COUNT. Objects of the units are shared by TAG+CFG; a built object is skipped.
set -u -o pipefail
HERE=$(cd "$(dirname "$0")" && pwd)
LIFT=${LIFT:-$(cd "$HERE/../.." && pwd)}
OUT=${OUT:?OUT is not set}
TAG=${TAG:-ott}
OPT=${OPT:-O2}
BUILTIN=${BUILTIN:-1}
COUNT=${COUNT:-0}
SAN=${SAN:-0}
GUARD=${GUARD:-0}
XCF=${XCF:-}      # extra compiler flags for the units, the runtime and the harness (e.g. -DFLE_MX_STATS=1: fast/slow counts of the MXCSR-exact float operations; the config gets the suffix -x)
export CC=${CC:-clang}
JOBS=${JOBS:-3}
RT="$LIFT/runtime"
CFG="$OPT-b$BUILTIN-c$COUNT"
[ -n "$XCF" ] && CFG="$CFG-x"
GWRAP=""; [ "$GUARD" = 1 ] && { GWRAP="-Wl,--wrap=malloc -Wl,--wrap=calloc -Wl,--wrap=realloc -Wl,--wrap=free -Wl,--wrap=aligned_alloc"; CFG="$CFG-guard"; }
SANF=""; [ "$SAN" = 1 ] && { SANF="-fsanitize=address -fno-omit-frame-pointer"; CFG="$CFG-asan"; }
D="$OUT/native/$TAG-$CFG"
mkdir -p "$D/obj" "$D/rt" "$D/h" "$OUT/tmp"
export TMPDIR="$OUT/tmp"
OPTF="-$OPT"; [ "$OPT" = O0 ] && OPTF="-O0 -g"
BI="-fno-builtin"; [ "$BUILTIN" = 1 ] && BI="-fno-builtin-fprintf -fno-builtin-fputs"
CNT=""; [ "$COUNT" != 0 ] && CNT="-DFLE_COUNT=1"
# The generated units include fle_modules.h (for fle_resp_entry), which declares some stopping answers __attribute__((noreturn)) (GetModuleHandleExW). A wrapper that
# answers (vst_env.c) would then be called as a call that never returns and the compiler would drop the code after it. For the units only, a copy of the header without the
# attribute comes first in the include path; the runtime and the harness see the original.
mkdir -p "$D/shadow"
sed 's/ *__attribute__((noreturn))//' "$RT/fle_modules.h" > "$D/shadow/fle_modules.h"
GEN="-std=gnu11 $OPTF $SANF $XCF -w $BI -ffp-contract=off -fPIE -I$D/shadow -I$RT -I$HERE $CNT"
# COUNT=1: the units come from $OUT/src-count/ (count_instrument.py, basic blocks); COUNT=2: $OUT/src-count-pi/ (--per-insn, for checking COUNT=1 against). common.c is the same file.
SRCD="$OUT/src"
if [ "$COUNT" != 0 ]; then
    SRCD="$OUT/src-count"; PI=""; [ "$COUNT" = 2 ] && { SRCD="$OUT/src-count-pi"; PI="--per-insn"; }
    mkdir -p "$SRCD"
    for f in "$OUT"/src/$TAG-[0-9][0-9].c; do [ -e "$SRCD/$(basename "$f")" ] || python3 "$HERE/count_instrument.py" $PI "$f" "$SRCD/$(basename "$f")" 2> /dev/null || exit 1; done
    [ -e "$SRCD/common-$TAG.c" ] || cp "$OUT/src/common-$TAG.c" "$SRCD/common-$TAG.c"
fi
ls "$SRCD"/$TAG-[0-9][0-9].c "$SRCD"/common-$TAG.c | xargs -P "$JOBS" -I{} bash -c 'b=$(basename {} .c); [ -e '"$D"'/obj/$b.o ] || $CC '"$GEN"' -c {} -o '"$D"'/obj/$b.o' || exit 1
for f in "$RT"/fle_*.c; do b=$(basename "$f" .c); [ "$D/rt/$b.o" -nt "$f" ] || $CC -std=gnu11 $OPTF $SANF $XCF -Wall -Wextra -fPIE -I"$RT" -c "$f" -o "$D/rt/$b.o" || exit 1; done
[ -e "$D/h/image.o" ] || $CC -std=gnu11 -O1 $SANF -Wall -Wextra -fPIE -I"$RT" -I"$LIFT/harness" -c "$LIFT/harness/image.c" -o "$D/h/image.o" || exit 1
# the real-module table: the file name (lower case) and the ImageBase of the PE
FILE=${FILE:?FILE (the PE) is not set}
RMNAME=${RMNAME:-$(basename "$FILE" | tr 'A-Z' 'a-z')}
RMBASE=$(python3 -c "import struct,sys; d=open(sys.argv[1],'rb').read(0x400); l=struct.unpack_from('<I',d,0x3c)[0]; print(hex(struct.unpack_from('<Q',d,l+24+24)[0]))" "$FILE")
for f in "$HERE"/*.c; do b=$(basename "$f" .c); [ "$D/h/$b.o" -nt "$f" ] || $CC -std=gnu11 -O1 $SANF -Wall -Wextra -fPIE -I"$RT" -I"$LIFT/harness" -I"$HERE" $CNT $XCF -DVST_RM_NAME="\"$RMNAME\"" -DVST_RM_BASE="$RMBASE" -c "$f" -o "$D/h/$b.o" || exit 1; done
rm -f "$D/vst-native"
$CC $SANF -pie -o "$D/vst-native" "$D"/h/*.o "$D"/obj/*.o "$D"/rt/*.o -Wl,--wrap=fle_imp_LoadLibraryExW -Wl,--wrap=fle_imp_LoadLibraryW -Wl,--wrap=fle_imp_LoadLibraryA -Wl,--wrap=fle_imp_GetModuleHandleExW -Wl,--wrap=fle_imp_CreateWindowExW -Wl,--wrap=fle_imp_CreateFileW -Wl,--wrap=fle_imp_GetFileType -Wl,--wrap=fle_imp_FormatMessageW -Wl,--wrap=fle_imp_LockResource -Wl,--wrap=fle_imp_SetFilePointerEx $GWRAP -lm -lpthread -ldl && echo "built $D/vst-native"
