#!/bin/bash
# lat1: run a G3 script on the native -O0 fork-probe binary (a copy of ia2's D7 two-native-fp, read only) with per-probe coverage. usage: run-nat.sh RUNDIR SCRIPT
R=/home/ubuntu/work/signature-studio-develop; W=$R/lift/out-lat1; H=$R/lift/harness
RUNDIR=$1; SCRIPT=$2
DLLD=$W/nat/dll
mkdir -p $RUNDIR/wintable $RUNDIR/probes $RUNDIR/dump
REAL=$DLLD/dsp_ippv2_x64.2000000.dll:$DLLD/Blend2D.4700000.dll:$DLLD/quickfontcache_x64.1d00000.dll:$DLLD/freetype_x64.4b00000.dll:$DLLD/ILTools_x64.6000000.dll
export FLE_CALL_FRUITY=1 FLE_VFS_SEED=$H/seeds/vfs-seed-idle1.tsv FLE_VFS_SEED_ROOT=$R
export FLE_REG_SEED=$H/seeds/reg-seed-nsis.tsv:$H/seeds/reg-seed-app.tsv FLE_AUTH_CLAIMS=$H/seeds/authenticode-claims.tsv
export FLE_FRUITY_MAX_CALLS=400000000000 FLE_FRUITY_SECONDS=3000 FLE_FRUITY_NO_IDLE_STOP=1 FLE_G3_SCRIPT=$SCRIPT FLE_G3_PROGRESS=1 FLE_G3_WINTABLE_DIR=$RUNDIR/wintable
export FLE_G3_PROBE_DIR=$RUNDIR/probes FLE_G8_DUMPDIR=$RUNDIR/dump FLE_G3_PROBE_CPU=1800 FLE_COV_FILE=$RUNDIR/cov.txt
echo "load $(cut -d' ' -f1 /proc/loadavg) start $(date +%T)" > $RUNDIR/load.txt
( cd $RUNDIR && /usr/bin/time -v nice -n 15 ${BIN:-$W/nat/two-native-fp} $R/binaries/win64/FLEngine_x64.dll "$REAL" > $RUNDIR/stdout.txt 2> $RUNDIR/stderr.txt )
echo "rc=$? load-end $(cut -d' ' -f1 /proc/loadavg) end $(date +%T)" >> $RUNDIR/load.txt
grep -a -E "^(g3-(snap|probe))|^fruity: (== stop|runtime message)" $RUNDIR/stdout.txt | cut -c1-260 > $RUNDIR/g3-lines.txt
grep -E "Elapsed|Maximum resident|User time" $RUNDIR/stderr.txt >> $RUNDIR/load.txt
