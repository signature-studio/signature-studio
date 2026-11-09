#!/bin/bash
# alpha1 (decision 0244): link a native -O2 engine (N1) with the recorder from
#   * the compiled GENERATED C of the production-shaped build (F2xH7) that pick1 (decision 0242) built: copied unchanged (they are not changed by this work; the lifter is not touched),
#   * the runtime (all fle_*.c) compiled here at -O2 from $SRC with the production flags -DFLE_DESKTOP -DFLE_R_FAST_COPY=1 (+ $RT_X),
#   * the FRUITY harness, the upstream Blend2D adapter and library (pick1's), and the recorder tools/alpha1_rec.c + the wrappers tools/gen-wrap.py generates.
# usage: build-n1.sh OUTBIN [SRC (a tree with lift/)]      env: RT_X (extra -D), TAG (object directory suffix), JOBS (default 4)
set -eu -o pipefail
A=/home/ubuntu/work/signature-studio-develop/lift/out-alpha1
HERE=$A/tools
OUTBIN=$(readlink -f "$1"); SRC=${2:-$A/src}; TAG=${TAG:-base}; JOBS=${JOBS:-4}
LIFT=$SRC/lift; RT=$LIFT/runtime; H=$LIFT/harness
P=/home/ubuntu/work/signature-studio-develop/lift/out-pick1
GENOBJ=$A/work/gen-obj
O=$A/work/obj-$TAG; mkdir -p $O/rt $O/h $O/log $GENOBJ/fle $GENOBJ/mod
copy_gen() {
  [ -e $GENOBJ/.copied ] && return 0
  [ "$(ls $P/ci/engine/nat/obj/fle/fle-[0-9][0-9].o 2>/dev/null | wc -l)" = 23 ] && [ "$(ls $P/ci/engine/nat/obj/mod/*.o 2>/dev/null | wc -l)" = 21 ] || { echo "pick1's objects are not complete yet (fle $(ls $P/ci/engine/nat/obj/fle/fle-[0-9][0-9].o | wc -l)/23, mod $(ls $P/ci/engine/nat/obj/mod/*.o | wc -l)/21)"; return 1; }
  cp -u $P/ci/engine/nat/obj/fle/fle-[0-9][0-9].o $GENOBJ/fle/
  cp -u $P/ci/engine/nat/obj/mod/*.o $GENOBJ/mod/
  cp -u $P/ci/engine/onat/v-tb2d2/native/realmod_table_gen.c $GENOBJ/
  cp -u $P/work/b2d/build-native/libblend2d.a $GENOBJ/
  touch $GENOBJ/.copied
}
UP=$GENOBJ/libblend2d.a; UPSRC=$P/work/b2d/src/src
RTD="-DFLE_DESKTOP -DFLE_R_FAST_COPY=1 ${RT_X:-}"
export RT O RTD
ls $RT/fle_*.c | xargs -P $JOBS -I{} bash -c 'b=$(basename {} .c); [ -e $O/rt/$b.o ] || nice -n 15 clang -std=gnu11 -O2 $RTD -ffp-contract=off -fno-fast-math -w -fPIE -I$RT -c {} -o $O/rt/$b.o 2> $O/log/rt-$b.log'
copy_gen
for f in image a4; do clang -std=gnu11 -O1 -fPIE -w -I$RT -I$H -c $H/$f.c -o $O/h/$f.o; done
clang -std=gnu11 -O1 -fPIE -w -I$RT -I$H -c $H/wasm/trace_native.c -o $O/h/trace.o
clang -std=gnu11 -O1 -fPIE -w -DFLE_FRUITY=1 -I$RT -I$H -c $H/two/main_two.c -o $O/h/main_two.o
for f in fruity fruity_g3; do clang -std=gnu11 -O1 -fPIE -w -I$RT -I$H -c $H/$f.c -o $O/h/$f.o; done
clang -std=gnu11 -O1 -fPIE -w -I$RT -c $GENOBJ/realmod_table_gen.c -o $O/h/realmod_table.o
[ -e $A/work/b2d_swap.o ] || clang++ -std=c++17 -O2 -fno-exceptions -fno-rtti -fPIE -DBL_STATIC -DB2D1_LIBM_BRIDGE=1 -I$UPSRC -c $H/b2d1/b2d_swap.cpp -o $A/work/b2d_swap.o
# the recorder
nm --defined-only $O/rt/*.o $O/h/*.o > $O/nm.txt
python3 $HERE/gen-wrap.py $O/nm.txt $O/a1_gen.c $O/wrap.ldflags
clang -std=gnu11 -O2 -w -fPIE -I$RT -I$HERE -c $O/a1_gen.c -o $O/a1_gen.o
clang -std=gnu11 -O2 -w -fPIE -I$RT -I$HERE -c $HERE/alpha1_rec.c -o $O/alpha1_rec.o
clang++ -pie -o $OUTBIN $O/h/main_two.o $O/h/fruity.o $O/h/fruity_g3.o $O/h/image.o $O/h/a4.o $O/h/trace.o $O/h/realmod_table.o \
  $GENOBJ/mod/m[1345]-*.o $GENOBJ/mod/m2-*.o $GENOBJ/mod/common.o $GENOBJ/fle/*.o $O/rt/*.o $A/work/b2d_swap.o $UP $O/a1_gen.o $O/alpha1_rec.o \
  @$O/wrap.ldflags -Wl,--wrap=fle_desk_wrote -Wl,--wrap=fprintf -Wl,--wrap=fputs -Wl,--wrap=__cyg_profile_func_enter -lm
ls -l $OUTBIN
