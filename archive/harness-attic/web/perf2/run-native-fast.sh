#!/bin/bash
# perf2 (decision 0138): runs a native fast-path binary (build_native_fast.sh) or any two-native built by this work with the run-time environment of a4-fruity.sh on-seh (a4-multi.sh passes it to the harness) on the rebased DLLs of the generated-C directory.
# usage: [NO_IDLE_STOP=1|0 (default 1; 0 = the idle stop of decision 0122 is on)] BIN=<two-native*> DLLDIR=<v-TAG/dll of an OUT> [FLE_FRUITY_SECONDS=900 (max 3600)] [FLE_PERF2_*] run-native-fast.sh LOGPREFIX   -> LOGPREFIX.out (stdout) / LOGPREFIX.err (stderr)
. /home/ubuntu/work/signature-studio-develop/lift/out-perf2/src/docs/investigations/perf2/tools/env.sh
export FLE_CALL_FRUITY=1 FLE_VFS_SEED=${VFS_SEED_OVERRIDE:-$H/seeds/vfs-seed-idle1.tsv} FLE_VFS_SEED_ROOT=$R
export FLE_REG_SEED=$H/seeds/reg-seed-nsis.tsv:$H/seeds/reg-seed-app.tsv FLE_AUTH_CLAIMS=$H/seeds/authenticode-claims.tsv
export FLE_FRUITY_MAX_CALLS=${FLE_FRUITY_MAX_CALLS:-4000000000} FLE_FRUITY_SECONDS=${FLE_FRUITY_SECONDS:-900}
if [ "${NO_IDLE_STOP:-1}" = 1 ]; then export FLE_FRUITY_NO_IDLE_STOP=1; else unset FLE_FRUITY_NO_IDLE_STOP; fi
DLLD=${DLLDIR:?}
REAL=$DLLD/dsp_ippv2_x64.2000000.dll:$DLLD/Blend2D.4700000.dll:$DLLD/quickfontcache_x64.1d00000.dll:$DLLD/freetype_x64.4b00000.dll:$DLLD/ILTools_x64.6000000.dll
LP=${1:?}
date -u +%FT%TZ > $LP.start
/usr/bin/time -v ${BIN:?} $R/binaries/win64/FLEngine_x64.dll "$REAL" > $LP.out 2> $LP.err
echo "rc=$?"; date -u +%FT%TZ > $LP.end
grep -a -E "^fruity: (== stop|runtime message)" $LP.out | cut -c1-300
grep -a -E "Elapsed|Maximum res" $LP.err
