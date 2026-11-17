#!/bin/bash
# pick1: link the N1 binary (no recorder) from the objects native-build.sh compiled, with the runtime objects of a CHANGED runtime tree (only the objects whose .c differs from the compiled ones are rebuilt).
# usage: link-plain.sh WORK RUNTIME_SRC_DIR OUTBIN [RT_OBJ_DIR]     (RT_OBJ_DIR: where the changed runtime objects go; default WORK/nat/rt-fix)
set -eu -o pipefail
HERE=$(cd "$(dirname "$0")" && pwd)
WORK=$(cd "$1" && pwd); RTS=$(cd "$2" && pwd); OUTBIN=$3; OBJ=$WORK/nat/obj; RTO=$WORK/nat/rt; FIX=${4:-$WORK/nat/rt-fix}
UP=$HERE/../work/b2d/build-native/libblend2d.a
mkdir -p $FIX; rm -f $FIX/*.o
for o in $RTO/*.o; do b=$(basename $o .o); cp $o $FIX/$b.o; done
# rebuild any runtime object whose source is newer in the given tree than the one compiled (compare to the pristine tree's file)
ORIG=$HERE/../src-orig/lift/runtime
for c in $RTS/fle_*.c; do b=$(basename $c .c); if ! cmp -s $c $ORIG/$b.c; then echo "rebuild $b"; clang -std=gnu11 -O2 -ffp-contract=off -fno-fast-math -w -fPIE -I$RTS -c $c -o $FIX/$b.o; fi; done
# a changed header would need every dependent object rebuilt: refuse silently-wrong links
for h in $RTS/*.h; do b=$(basename $h); if [ -e $ORIG/$b ] && ! cmp -s $h $ORIG/$b; then echo "header $b changed: rebuilding the objects that include it"; for c in $(grep -l "\"$b\"" $RTS/fle_*.c); do bb=$(basename $c .c); clang -std=gnu11 -O2 -ffp-contract=off -fno-fast-math -w -fPIE -I$RTS -c $c -o $FIX/$bb.o; done; fi; done
clang++ -pie -o $OUTBIN $OBJ/h-fruity/main_two.o $OBJ/h-fruity/fruity.o $OBJ/h-fruity/fruity_g3.o $OBJ/h/image.o $OBJ/h/a4.o $OBJ/h/trace.o $OBJ/h/realmod_table.o \
  $OBJ/mod/m[1345]-*.o $OBJ/mod/m2-*.o $OBJ/mod/common.o $OBJ/fle/*.o $FIX/*.o $OBJ/b2d_swap.o $UP -Wl,--wrap=fprintf -Wl,--wrap=fputs -Wl,--wrap=__cyg_profile_func_enter -lm
ls -l $OUTBIN
