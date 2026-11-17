#!/bin/bash
# perf2 (decision 0138, PROVISIONAL): the NATIVE "fast path" harness: the units already compiled by docs/investigations/perf2/tools/native-compile.sh (generated C at -O2, no -finstrument-functions) + the runtime at -$RT_OPT (default O2) + the harness of build_two_native.sh FRUITY=1
# (same flags, -O1) + perf2_probe.c and the API-counter wrappers (gen_p2_wraps.py). Output: $OBJ/two-native-fast. Nothing is overwritten: an existing object is skipped; the runtime objects are rebuilt when RT_REBUILD=1 (a runtime edit during the wall loop).
# usage: LIFT=<lift/ of the tree> OBJ=<native-compile.sh OBJ> REALMOD_C=<realmod_table_gen.c> [RT_OPT=O2] [P2=1] [JOBS=4] bash build_native_fast.sh
set -u -o pipefail
LIFT=${LIFT:?}; OBJ=${OBJ:?}; REALMOD_C=${REALMOD_C:?}
RT_OPT=${RT_OPT:-O2}; P2=${P2:-1}; JOBS=${JOBS:-4}
RT=$LIFT/runtime; HERE=$LIFT/harness/two; P2D=$LIFT/harness/web/perf2
mkdir -p $OBJ/rt $OBJ/h $OBJ/h-fruity $OBJ/p2 $OBJ/log
export TMPDIR=${TMPDIR:-$OBJ/log}
if [ "${RT_REBUILD:-0}" = 1 ]; then rm -f $OBJ/rt/*.o $OBJ/p2/*.o $OBJ/h/*.o $OBJ/h-fruity/*.o $OBJ/p2/p2_wraps.c $OBJ/p2/p2_wrap_flags.txt; fi
ls $RT/fle_*.c | xargs -P $JOBS -I{} bash -c 'b=$(basename {} .c); [ -e '$OBJ'/rt/$b.o ] || clang -std=gnu11 -'$RT_OPT' -Wall -Wextra -fPIE -I'$RT' -c {} -o '$OBJ'/rt/$b.o 2> '$OBJ'/log/rt-$b.log' || exit 1
for f in image a4; do [ -e $OBJ/h/$f.o ] || clang -std=gnu11 -O1 -Wall -Wextra -fPIE -I$RT -I$LIFT/harness -c $LIFT/harness/$f.c -o $OBJ/h/$f.o 2> $OBJ/log/h-$f.log || exit 1; done
[ -e $OBJ/h/trace.o ] || clang -std=gnu11 -O1 -Wall -Wextra -fPIE -I$RT -I$LIFT/harness -c $LIFT/harness/wasm/trace_native.c -o $OBJ/h/trace.o 2> $OBJ/log/h-trace.log || exit 1
[ -e $OBJ/h-fruity/main_two.o ] || clang -std=gnu11 -O1 -Wall -Wextra -fPIE -DFLE_FRUITY=1 -I$RT -I$LIFT/harness -c $HERE/main_two.c -o $OBJ/h-fruity/main_two.o 2> $OBJ/log/h-main_two.log || exit 1
[ -e $OBJ/h-fruity/fruity.o ] || clang -std=gnu11 -O1 -Wall -Wextra -fPIE -I$RT -I$LIFT/harness -c $LIFT/harness/fruity.c -o $OBJ/h-fruity/fruity.o 2> $OBJ/log/h-fruity.log || exit 1
[ -e $OBJ/h/realmod_table.o ] || clang -std=gnu11 -O1 -Wall -Wextra -fPIE -I$RT -c $REALMOD_C -o $OBJ/h/realmod_table.o 2> $OBJ/log/h-realmod.log || exit 1
EXTRA=""; FLAGS=""
if [ "$P2" = 1 ]; then
  if [ ! -e $OBJ/p2/p2_wraps.c ]; then
    nm -g --defined-only $OBJ/rt/*.o | awk '$2 == "T" && $3 ~ /^fle_imp_/ { print $3 }' | LC_ALL=C sort -u > $OBJ/p2/symbols.txt
    python3 $P2D/gen_p2_wraps.py $OBJ/p2/symbols.txt $OBJ/p2 || exit 1
  fi
  [ -e $OBJ/p2/perf2_probe.o ] || clang -std=gnu11 -O2 -Wall -Wextra -fPIE -I$RT -I$LIFT/harness -c $P2D/perf2_probe.c -o $OBJ/p2/perf2_probe.o 2> $OBJ/log/p2-probe.log || { cat $OBJ/log/p2-probe.log; exit 1; }
  [ -e $OBJ/p2/p2_wraps.o ] || clang -std=gnu11 -O1 -fPIE -c $OBJ/p2/p2_wraps.c -o $OBJ/p2/p2_wraps.o 2> $OBJ/log/p2-wraps.log || exit 1
  EXTRA="$OBJ/p2/perf2_probe.o $OBJ/p2/p2_wraps.o"; FLAGS=$(cat $OBJ/p2/p2_wrap_flags.txt)
fi
rm -f $OBJ/two-native-fast
clang -pie -o $OBJ/two-native-fast $OBJ/h-fruity/main_two.o $OBJ/h-fruity/fruity.o $OBJ/h/image.o $OBJ/h/a4.o $OBJ/h/trace.o $OBJ/h/realmod_table.o $OBJ/mod/*.o $OBJ/fle/*.o $OBJ/rt/*.o $EXTRA \
  -Wl,--wrap=fprintf -Wl,--wrap=fputs -Wl,--wrap=__cyg_profile_func_enter $([ "$P2" = 1 ] && echo -Wl,--wrap=fle_raster_wrote) $FLAGS -lm 2> $OBJ/log/link.log && echo "built $OBJ/two-native-fast" && ls -l $OBJ/two-native-fast
