#!/bin/bash
# parity gate (decision 0226): N1 = the generated C of a bundle build compiled natively at -O2 (no instrumentation), the runtime at -O2, the FRUITY harness and the upstream Blend2D adapter. THE SAME generated C as the wasm of that build.
# usage:  native-build.sh WORK OUTBIN [UP_NATIVE_LIB UP_SRC]      env: JOBS (default 5), WRAPS=1 (link the counters of parity_wrap.c / parity_prof.c: pixel volumes, SendMessageW, a SIGPROF sampler)
#   WORK = the work dir of app/scripts/build-engine-bundle.sh after its `gen` and `resplit` stages (WORK/wsrc/src/fle-NN.c, WORK/wsrc/v-TAG/src/mK-NN.c, WORK/onat/v-TAG/native/realmod_table_gen.c).
#   UP_NATIVE_LIB / UP_SRC: upstream Blend2D built natively:  OUTDIR=<dir> TARGET=native OPT=-O2 PATCH=lift/harness/b2d1/b2d3-compop.patch bash lift/harness/b2d1/build_upstream.sh  -> <dir>/build-native/libblend2d.a, <dir>/src/src
# Build time on this machine (16 cores, load 15-40): the 44 units at -O2 with 5 jobs about 20 minutes; the runtime, the harness and the link about 1 minute.
set -u -o pipefail
HERE=$(cd "$(dirname "$0")" && pwd); LIFT=$(cd "$HERE/../.." && pwd); RT=$LIFT/runtime; H=$LIFT/harness
WORK=$(cd "$1" && pwd); OUTBIN=$2; JOBS=${JOBS:-5}
UP=${3:-}; UPSRC=${4:-}
[ -r "$UP" ] && [ -d "$UPSRC" ] || { echo "native-build: UP_NATIVE_LIB (libblend2d.a) and UP_SRC (blend2d src/src) are needed (the swapped Blend2D configuration)"; exit 2; }
TAGDIR=$(ls -d $WORK/wsrc/v-* | head -1); RMC=$(ls $WORK/onat/v-*/native/realmod_table_gen.c | head -1)
OBJ=$WORK/nat/obj; mkdir -p $OBJ/fle $OBJ/mod $OBJ/h $OBJ/h-fruity $OBJ/log
export RT OBJ
cc_one() { [ -e "$2" ] && return 0; b=$(basename $1 .c); nice -n 15 clang -std=gnu11 -O2 -w -fno-builtin-fprintf -fno-builtin-fputs -ffp-contract=off -fno-fast-math -fPIE -I$RT -c $1 -o $2.tmp > $OBJ/log/$b.log 2>&1 && mv $2.tmp $2; }
export -f cc_one
{ echo "$TAGDIR/src/common.c $OBJ/mod/common.o"
  for f in $(ls -S $WORK/wsrc/src/fle-[0-9][0-9].c $TAGDIR/src/m[0-9]*-[0-9][0-9].c); do case $f in */fle-*) echo "$f $OBJ/fle/$(basename $f .c).o" ;; *) echo "$f $OBJ/mod/$(basename $f .c).o" ;; esac; done
} | xargs -P $JOBS -L 1 bash -c 'cc_one "$0" "$1"' || exit 1
RTO=$WORK/nat/rt; mkdir -p $RTO
export RTO
ls $RT/fle_*.c | xargs -P 4 -I{} bash -c 'b=$(basename {} .c); [ -e $RTO/$b.o ] || nice -n 15 clang -std=gnu11 -O2 -ffp-contract=off -fno-fast-math -Wall -Wextra -fPIE -I$RT -c {} -o $RTO/$b.o 2> $OBJ/log/rt-$b.log' || exit 1
for f in image a4; do [ -e $OBJ/h/$f.o ] || clang -std=gnu11 -O1 -fPIE -I$RT -I$H -c $H/$f.c -o $OBJ/h/$f.o || exit 1; done
[ -e $OBJ/h/trace.o ] || clang -std=gnu11 -O1 -fPIE -I$RT -I$H -c $H/wasm/trace_native.c -o $OBJ/h/trace.o || exit 1
[ -e $OBJ/h-fruity/main_two.o ] || clang -std=gnu11 -O1 -fPIE -DFLE_FRUITY=1 -I$RT -I$H -c $H/two/main_two.c -o $OBJ/h-fruity/main_two.o || exit 1
for f in fruity fruity_g3; do [ -e $OBJ/h-fruity/$f.o ] || clang -std=gnu11 -O1 -fPIE -I$RT -I$H -c $H/$f.c -o $OBJ/h-fruity/$f.o || exit 1; done
[ -e $OBJ/h/realmod_table.o ] || clang -std=gnu11 -O1 -fPIE -I$RT -c $RMC -o $OBJ/h/realmod_table.o || exit 1
[ -e $OBJ/b2d_swap.o ] || clang++ -std=c++17 -O2 -fno-exceptions -fno-rtti -fPIE -DBL_STATIC -I$UPSRC -c $H/b2d1/b2d_swap.cpp -o $OBJ/b2d_swap.o || exit 1
XOBJ=""; XF=""
if [ "${WRAPS:-0}" = 1 ]; then
  clang -std=gnu11 -O2 -fPIE -I$RT -c $HERE/parity_wrap.c -o $OBJ/parity_wrap.o && clang -std=gnu11 -O2 -fPIE -I$RT -c $HERE/parity_prof.c -o $OBJ/parity_prof.o || exit 1
  XOBJ="$OBJ/parity_wrap.o $OBJ/parity_prof.o"; XF="-Wl,--wrap=fle_r_blit -Wl,--wrap=fle_r_rop_rect -Wl,--wrap=fle_r_mix_rect -Wl,--wrap=fle_imp_SendMessageW"
fi
clang++ -pie -o $OUTBIN $OBJ/h-fruity/main_two.o $OBJ/h-fruity/fruity.o $OBJ/h-fruity/fruity_g3.o $OBJ/h/image.o $OBJ/h/a4.o $OBJ/h/trace.o $OBJ/h/realmod_table.o \
  $OBJ/mod/m[1345]-*.o $OBJ/mod/m2-*.o $OBJ/mod/common.o $OBJ/fle/*.o $RTO/*.o $OBJ/b2d_swap.o $UP $XOBJ $XF -Wl,--wrap=fprintf -Wl,--wrap=fputs -Wl,--wrap=__cyg_profile_func_enter -lm || exit 1
ls -l $OUTBIN
