#!/bin/bash
# pick1: link the N1 binary of lift/harness/parity/native-build.sh again with the API recorder (apiw_core.c + the wrappers gen-apiwrap.py makes), the raster counters (pix_wrap.c) and the SIGPROF sampler (parity_prof.c).
# usage: link-native.sh WORK OUTBIN        (WORK = the work dir native-build.sh compiled in; the objects are reused, nothing is recompiled except the recorder)
set -eu -o pipefail
HERE=$(cd "$(dirname "$0")" && pwd); SRC=$HERE/../src; RT=$SRC/lift/runtime; H=$SRC/lift/harness
WORK=$(cd "$1" && pwd); OUTBIN=$2; OBJ=$WORK/nat/obj; RTO=$WORK/nat/rt
UP=$WORK/b2d/build-native/libblend2d.a
[ -r "$UP" ] || UP=$(cd "$HERE/.." && pwd)/work/b2d/build-native/libblend2d.a
GEN=$WORK/nat/apiw; mkdir -p $GEN
nm --defined-only $RTO/*.o $OBJ/h/*.o $OBJ/h-fruity/*.o 2>/dev/null > $GEN/nm.txt
python3 $HERE/gen-apiwrap.py $GEN/nm.txt $GEN/apiw_gen.c $GEN/wrap.ldflags
clang -std=gnu11 -O2 -w -fPIE -I$RT -I$HERE -c $GEN/apiw_gen.c -o $GEN/apiw_gen.o
clang -std=gnu11 -O2 -w -fPIE -I$RT -I$HERE -c $HERE/apiw_core.c -o $GEN/apiw_core.o
clang -std=gnu11 -O2 -w -fPIE -I$RT -I$HERE -c $HERE/pix_wrap.c -o $GEN/pix_wrap.o
clang -std=gnu11 -O2 -w -fPIE -I$RT -c $H/parity/parity_prof.c -o $GEN/parity_prof.o
clang++ -pie -o $OUTBIN $OBJ/h-fruity/main_two.o $OBJ/h-fruity/fruity.o $OBJ/h-fruity/fruity_g3.o $OBJ/h/image.o $OBJ/h/a4.o $OBJ/h/trace.o $OBJ/h/realmod_table.o \
  $OBJ/mod/m[1345]-*.o $OBJ/mod/m2-*.o $OBJ/mod/common.o $OBJ/fle/*.o $RTO/*.o $OBJ/b2d_swap.o $UP $GEN/apiw_gen.o $GEN/apiw_core.o $GEN/pix_wrap.o $GEN/parity_prof.o \
  @$GEN/wrap.ldflags -Wl,--wrap=fle_r_blit -Wl,--wrap=fle_r_rop_rect -Wl,--wrap=fle_r_mix_rect -Wl,--wrap=fle_r_stretch -Wl,--wrap=fle_r_maskblt -Wl,--wrap=fle_r_line \
  -Wl,--wrap=fprintf -Wl,--wrap=fputs -Wl,--wrap=__cyg_profile_func_enter -Wl,-Map=$OUTBIN.map -lm
ls -l $OUTBIN
