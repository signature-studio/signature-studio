#!/bin/bash
# parity1: N1 binary = native-units.sh objects + the runtime (all fle_*.c) at -O2 + the FRUITY harness (main_two.c -DFLE_FRUITY=1, fruity.c, fruity_g3.c) + the upstream Blend2D adapter and library (the swapped configuration of decision 0167).
# Flags as docs/investigations/perf3/tools/build_native_perf3.sh (P2=0) and lift/harness/b2d1/link_b2d1.sh MODE=run. No -finstrument-functions. usage: native-link.sh [RT_OPT] [BIN_NAME]
set -u -o pipefail
W=/home/ubuntu/work/signature-studio-develop/lift/out-parity1
RT_OPT=${1:-O2}; BIN=${2:-two-native-O2}
LIFT=${LIFTD:-$W/src/lift}; RT=$LIFT/runtime; H=$LIFT/harness; OBJ=${OBJD:-$W/build/nat/obj}
UP=$W/up/build-native/libblend2d.a; UPSRC=$W/up/src/src
TAGDIR=$(ls -d $W/build/wsrc/v-* | head -1); RMC=$(ls $W/build/onat/v-*/native/realmod_table_gen.c | head -1)
RTO=${NATD:-$W/build/nat}/rt-$RT_OPT${RT_TAG:+-$RT_TAG}; mkdir -p $RTO $OBJ/h $OBJ/h-fruity $OBJ/log
export RT RTO RT_OPT OBJ
ls $RT/fle_*.c | xargs -P 4 -I{} bash -c 'b=$(basename {} .c); [ -e $RTO/$b.o ] || nice -n 15 clang -std=gnu11 -$RT_OPT -ffp-contract=off -fno-fast-math -Wall -Wextra -fPIE -I$RT -c {} -o $RTO/$b.o 2> $OBJ/log/rt-$RT_OPT-$b.log' || exit 1
for f in image a4; do [ -e $OBJ/h/$f.o ] || clang -std=gnu11 -O1 -fPIE -I$RT -I$H -c $H/$f.c -o $OBJ/h/$f.o 2> $OBJ/log/h-$f.log || exit 1; done
[ -e $OBJ/h/trace.o ] || clang -std=gnu11 -O1 -fPIE -I$RT -I$H -c $H/wasm/trace_native.c -o $OBJ/h/trace.o 2> $OBJ/log/h-trace.log || exit 1
[ -e $OBJ/h-fruity/main_two.o ] || clang -std=gnu11 -O1 -fPIE -DFLE_FRUITY=1 -I$RT -I$H -c $H/two/main_two.c -o $OBJ/h-fruity/main_two.o 2> $OBJ/log/h-main_two.log || exit 1
[ -e $OBJ/h-fruity/fruity.o ] || clang -std=gnu11 -O1 -fPIE -I$RT -I$H -c $H/fruity.c -o $OBJ/h-fruity/fruity.o 2> $OBJ/log/h-fruity.log || exit 1
[ -e $OBJ/h-fruity/fruity_g3.o ] || clang -std=gnu11 -O1 -fPIE -I$RT -I$H -c $H/fruity_g3.c -o $OBJ/h-fruity/fruity_g3.o 2> $OBJ/log/h-fruity_g3.log || exit 1
[ -e $OBJ/h/realmod_table.o ] || clang -std=gnu11 -O1 -fPIE -I$RT -c $RMC -o $OBJ/h/realmod_table.o 2> $OBJ/log/h-realmod.log || exit 1
[ -e $OBJ/b2d_swap.o ] || clang++ -std=c++17 -O2 -fno-exceptions -fno-rtti -fPIE -DBL_STATIC -I$UPSRC -c $H/b2d1/b2d_swap.cpp -o $OBJ/b2d_swap.o 2> $OBJ/log/b2d_swap.log || exit 1
rm -f ${NATD:-$W/build/nat}/$BIN
XOBJ=""; XFLAGS=""; if [ "${WRAPS:-0}" = 1 ]; then clang -std=gnu11 -O2 -fPIE -I$RT -c $H/parity/parity_wrap.c -o $OBJ/parity_wrap.o && clang -std=gnu11 -O2 -fPIE -I$RT -c $H/parity/parity_prof.c -o $OBJ/parity_prof.o && XOBJ="$OBJ/parity_wrap.o $OBJ/parity_prof.o" XFLAGS="-Wl,--wrap=fle_r_blit -Wl,--wrap=fle_r_rop_rect -Wl,--wrap=fle_r_mix_rect -Wl,--wrap=fle_imp_SendMessageW -Wl,--wrap=fle_imp_GetTickCount -Wl,--wrap=fle_imp_PeekMessageW -Wl,--wrap=fle_imp_GetMessageW -Wl,--wrap=fle_imp_WaitMessage -Wl,--wrap=fle_imp_MsgWaitForMultipleObjects -Wl,--wrap=fle_imp_PostMessageW -Wl,--wrap=fle_imp_DispatchMessageW -Wl,--wrap=fle_imp_Sleep -Wl,--wrap=fle_imp_SetTimer -Wl,--wrap=fle_imp_InvalidateRect -Wl,--wrap=fle_imp_BeginPaint -Wl,--wrap=fle_imp_EndPaint -Wl,--wrap=fle_imp_ValidateRect -Wl,--wrap=fle_imp_ValidateRgn -Wl,--wrap=fle_imp_GetUpdateRect -Wl,--wrap=fle_imp_GetUpdateRgn -Wl,--wrap=fle_imp_RedrawWindow -Wl,--wrap=fle_imp_DefWindowProcW -Wl,--wrap=fle_imp_InvalidateRgn -Wl,--wrap=fle_imp_UpdateWindow -Wl,--wrap=fle_imp_CallWindowProcW -Wl,--wrap=fle_imp_GetDC -Wl,--wrap=fle_imp_GetDCEx -Wl,--wrap=fle_imp_ReleaseDC"; fi
if [ "${TRACEALL:-0}" = 1 ]; then clang -std=gnu11 -O1 -fPIE -I$RT -c $H/parity-paint/parity_traceall.c -o $OBJ/parity_traceall.o && XOBJ="$XOBJ $OBJ/parity_traceall.o" XFLAGS="$XFLAGS $(cat $H/parity-paint/traceall-wrap-flags.txt) $(cat $H/parity-paint/customtrace-wrap-flags.txt)"; fi
clang++ -pie -o ${NATD:-$W/build/nat}/$BIN $OBJ/h-fruity/main_two.o $OBJ/h-fruity/fruity.o $OBJ/h-fruity/fruity_g3.o $OBJ/h/image.o $OBJ/h/a4.o $OBJ/h/trace.o $OBJ/h/realmod_table.o \
  $OBJ/mod/m[1345]-*.o $OBJ/mod/m2-*.o $OBJ/mod/common.o $OBJ/fle/*.o $RTO/*.o $OBJ/b2d_swap.o $UP $XOBJ $XFLAGS -Wl,--wrap=fprintf -Wl,--wrap=fputs -Wl,--wrap=__cyg_profile_func_enter -lm 2> $OBJ/log/link-$BIN.log; rc=$?
echo "link rc=$rc"; tail -5 $OBJ/log/link-$BIN.log; ls -l ${NATD:-$W/build/nat}/$BIN
