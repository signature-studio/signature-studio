#!/bin/bash
# usage: gdbsurf.sh OUTNAME OUTDIR : measurement only. Runs the (non-idle) two-native of $W/o<OUTNAME> with decision 0129's final environment under gdb (breakpoint only, no stop) and writes the lid-13 surface at every hit of
# describe_view to OUTDIR/hitN.bgrx (first non-background pixel = hit1, the stop = hit2; tools/gdbsurf.py checks the layout).
. /home/ubuntu/work/signature-studio-develop/lift/out-wasm0129/src/docs/investigations/wasm-final0129/tools/env.sh
O=$W/o$1
D=$2
mkdir -p $D
BIN=$(ls $O/v-*/native/two-native | head -1)
DLLD=$(dirname $(dirname $BIN))/dll
export FLE_CALL_FRUITY=1 FLE_VFS_SEED=$H/seeds/vfs-seed-idle1.tsv FLE_VFS_SEED_ROOT=$R
export FLE_REG_SEED=$H/seeds/reg-seed-nsis.tsv:$H/seeds/reg-seed-app.tsv FLE_AUTH_CLAIMS=$H/seeds/authenticode-claims.tsv
export FLE_FRUITY_MAX_CALLS=4000000000 FLE_FRUITY_NO_IDLE_STOP=1 FLE_FRUITY_SECONDS=${FLE_FRUITY_SECONDS:-900}
REAL=$DLLD/dsp_ippv2_x64.2000000.dll:$DLLD/Blend2D.4700000.dll:$DLLD/quickfontcache_x64.1d00000.dll:$DLLD/freetype_x64.4b00000.dll:$DLLD/ILTools_x64.6000000.dll
export GDBSURF_BIN=$BIN GDBSURF_OUT=$D GDBSURF_FLE=$R/binaries/win64/FLEngine_x64.dll GDBSURF_REAL=$REAL GDBSURF_STDOUT=$D/run.out GDBSURF_STDERR=$D/run.err
gdb -q -batch -x $T/gdbsurf.py $BIN > $D/gdb.out 2>&1
cat $D/gdbsurf.txt; grep -c "Python Exception" $D/gdb.out | sed 's/^/python exceptions: /'
grep -E "^fruity: (== stop|CreateFruityInstance at)" $D/run.out | cut -c1-200
