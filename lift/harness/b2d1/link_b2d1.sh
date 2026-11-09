#!/bin/bash
# Decision 0160 (provisional), opt-in: links native harness binaries from the objects of a normal native build (lift/harness/two/build_two_native.sh or
# tools/native-compile.sh + build_native_perf3.sh: $BASE/{fle,mod,rt,h,h-fruity}) and, for the swapped configuration, the objects of the Blend2D module
# lifted with --swap-entries ($SWAPMOD/{m2-*.o,common.o}), lift/harness/b2d1/b2d_swap.cpp and the upstream libblend2d.a (built with lift/harness/b2d1/build_upstream.sh).
# usage: BASE=<obj dir> MODE=run|scan [SWAPMOD=<dir with the swapped m2 and common objects> UPSTREAM_LIB=<libblend2d.a> UPSTREAM_SRC=<src dir>] OUT=<binary> link_b2d1.sh
#   MODE=run   the FRUITY harness (main_two.c -DFLE_FRUITY=1, fruity.c, fruity_g3.c): the 16-snapshot script of FLE_G3_SCRIPT
#   MODE=scan  the Agg2D scan harness (main_two.c -DFLE_B2D_SCAN=1, b2d_scan.c, with -DB2D1_SWAP=1 for the swapped configuration): FLE_B2D_SCAN=<test>
#   DIAG_MISS=1  (diagnostic; --wrap=fle_win_try_thunk) log and skip indirect calls to unknown entries (miss_log.c) instead of stopping
set -eu -o pipefail
HERE=$(cd "$(dirname "$0")" && pwd); LIFT=${LIFT:-$(cd "$HERE/../.." && pwd)}
BASE=${BASE:?}; MODE=${MODE:?}; OUT=${OUT:?}; RT=$LIFT/runtime; WORK=${WORK:-$(dirname "$OUT")/link-$(basename "$OUT")}
mkdir -p "$WORK"
SWAP=0; [ -n "${SWAPMOD:-}" ] && SWAP=1
MODOBJ="$BASE"/mod/*.o
if [ $SWAP = 1 ]; then
    : "${UPSTREAM_LIB:?}" "${UPSTREAM_SRC:?}"
    MODOBJ="$(ls "$BASE"/mod/m[1345]-*.o) $(ls "$SWAPMOD"/m2-*.o) $SWAPMOD/common.o"
    clang++ -std=c++17 -O2 -Wall -Wextra -fno-exceptions -fno-rtti -fPIE -DBL_STATIC -I"$UPSTREAM_SRC" -c "$HERE/b2d_swap.cpp" -o "$WORK/b2d_swap.o"
fi
SCANFLAGS=""; [ $SWAP = 1 ] && SCANFLAGS="-DB2D1_SWAP=1"
LDR=clang; [ $SWAP = 1 ] && LDR=clang++
EXTRA=""; [ $SWAP = 1 ] && EXTRA="$WORK/b2d_swap.o $UPSTREAM_LIB"
DIAG=""
if [ "${DIAG_MISS:-0}" = 1 ]; then clang -std=gnu11 -O1 -fPIE -c "$HERE/miss_log.c" -o "$WORK/miss_log.o"; DIAG="$WORK/miss_log.o -Wl,--wrap=fle_win_try_thunk"; fi
if [ "$MODE" = run ]; then
    $LDR -pie -o "$OUT" "$BASE"/h-fruity/main_two.o "$BASE"/h-fruity/fruity.o "$BASE"/h-fruity/fruity_g3.o "$BASE"/h/image.o "$BASE"/h/a4.o "$BASE"/h/trace.o "$BASE"/h/realmod_table.o \
        $DIAG $MODOBJ "$BASE"/fle/*.o "$BASE"/rt/*.o $EXTRA -Wl,--wrap=fprintf -Wl,--wrap=fputs -Wl,--wrap=__cyg_profile_func_enter -lm
else
    clang -std=gnu11 -O1 -Wall -Wextra -fPIE -DFLE_B2D_SCAN=1 -I"$RT" -I"$LIFT/harness" -c "$LIFT/harness/two/main_two.c" -o "$WORK/main_two.o"
    clang -std=gnu11 -O1 -Wall -Wextra -Wno-unused-function -Wno-unused-parameter -fPIE $SCANFLAGS -I"$RT" -I"$LIFT/harness" -c "$LIFT/harness/b2d_scan.c" -o "$WORK/b2d_scan.o"
    # the dispatch table of the FRUITY configuration names the three stubs of the fake host object (fruity.c); that file brings its own trace wrap
    $LDR -pie -o "$OUT" "$WORK/main_two.o" "$WORK/b2d_scan.o" "$BASE"/h-fruity/fruity.o "$BASE"/h-fruity/fruity_g3.o "$BASE"/h/image.o "$BASE"/h/a4.o "$BASE"/h/trace.o "$BASE"/h/realmod_table.o \
        $DIAG $MODOBJ "$BASE"/fle/*.o "$BASE"/rt/*.o $EXTRA -Wl,--wrap=fprintf -Wl,--wrap=fputs -Wl,--wrap=__cyg_profile_func_enter -lm
fi
echo "built $OUT"
