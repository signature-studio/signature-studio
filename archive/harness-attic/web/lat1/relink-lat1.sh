#!/bin/bash
# lat1: relink ia2's D7 fork-probe binary so that every probe child writes its OWN coverage file (decision 0184's trace_cov_probe_begin/end): ia2's fruity_g3.c (has the `wheel` command) + the three lines HEAD's fruity_g3.c has for the coverage hooks, and HEAD's trace_native.c (has the hooks). Reads ia2's D7 objects, writes only lift/out-lat1/nat/build.
set -eu
R=/home/ubuntu/work/signature-studio-develop; W=$R/lift/out-ia2; NAME=D7
L=$W/src/lift; RT=$L/runtime; HD=$L/harness; HEADHD=$R/lift/harness
ON=$W/$NAME/onat; V=$(ls -d $ON/v-* | head -1); R2=$W/$NAME/rt2; O=$R/lift/out-lat1/nat/build; mkdir -p $O
cp $HD/fruity_g3.c $O/fruity_g3_lat1.c
python3 - <<'PY'
import re
p='/home/ubuntu/work/signature-studio-develop/lift/out-lat1/nat/build/fruity_g3_lat1.c'
s=open(p).read()
s=s.replace('static FILE *follow_f;','extern void trace_cov_probe_begin(const char *name) __attribute__((weak));\nextern void trace_cov_probe_end(void) __attribute__((weak));\nstatic FILE *follow_f;',1)
s=s.replace('        in_probe = 1;\n        follow_f = NULL;','        in_probe = 1;\n        follow_f = NULL;\n        if (trace_cov_probe_begin) trace_cov_probe_begin(name);',1)
open(p,'w').write(s)
PY
grep -n "probe-end" $O/fruity_g3_lat1.c | head -5
python3 - <<'PY'
p='/home/ubuntu/work/signature-studio-develop/lift/out-lat1/nat/build/fruity_g3_lat1.c'
s=open(p).read()
s=s.replace('                g3_snapshot(nm);\n                fflush(stdout);\n                _exit(0);','                g3_snapshot(nm);\n                fflush(stdout);\n                if (trace_cov_probe_end) trace_cov_probe_end();\n                _exit(0);',1)
open(p,'w').write(s)
PY
grep -c "trace_cov_probe_end()" $O/fruity_g3_lat1.c
C="clang -std=gnu11 -O1 -w -fPIE -I$RT -I$HD"
nice -n 15 $C -DFLE_G3_FORKPROBE -c $O/fruity_g3_lat1.c -o $O/fruity_g3.o
nice -n 15 $C -I$HEADHD -c $HEADHD/wasm/trace_native.c -o $O/trace.o
nice -n 15 clang -pie -o $O/two-native-fp $ON/native/h-fruity/main_two.o $W/$NAME/fp2/fruity.o $O/fruity_g3.o $ON/native/h/image.o $ON/native/h/a4.o $O/trace.o \
  $V/native/obj-h/realmod_table.o $V/native/obj/*.o $ON/native/fle/*.o $R2/*.o -Wl,--wrap=fprintf -Wl,--wrap=fputs -Wl,--wrap=__cyg_profile_func_enter -Wl,--wrap=syscall -lm
ls -la $O/two-native-fp | awk '{print $5}'
nm $O/two-native-fp | grep -c "trace_cov_probe"
