#!/bin/bash
# mt1: run the native -O0 two-native binary of the ui1 build (read-only use) with a G3 script under gdb and list the CreateThread calls. usage: thread-trace.sh OUTDIR SCRIPT.g3
# needs the ui1 build tree (lift/out-ui1/build/onat/v-*) and lift/harness/seeds; observation only.
set -u
R=${REPO:-/home/ubuntu/work/signature-studio-develop}; U=$R/lift/out-ui1; H=$R/lift/harness
OUTD=$1; SCRIPT=$2; mkdir -p $OUTD
TAGDIR=$(ls -d $U/build/onat/v-* | head -1); DLLD=$TAGDIR/dll
REAL=$DLLD/dsp_ippv2_x64.2000000.dll:$DLLD/Blend2D.4700000.dll:$DLLD/quickfontcache_x64.1d00000.dll:$DLLD/freetype_x64.4b00000.dll:$DLLD/ILTools_x64.6000000.dll
export FLE_CALL_FRUITY=1 FLE_VFS_SEED=$H/seeds/vfs-seed-idle1.tsv FLE_VFS_SEED_ROOT=$R
export FLE_REG_SEED=$H/seeds/reg-seed-nsis.tsv:$H/seeds/reg-seed-app.tsv FLE_AUTH_CLAIMS=$H/seeds/authenticode-claims.tsv
export FLE_FRUITY_MAX_CALLS=4000000000 FLE_FRUITY_SECONDS=3000 FLE_FRUITY_NO_IDLE_STOP=1 FLE_G3_SCRIPT=$SCRIPT FLE_G3_PROGRESS=1 FLE_FRUITY_THREAD_TRACE=1
export FLE_G3_WINTABLE_DIR=$OUTD/wintable; mkdir -p $FLE_G3_WINTABLE_DIR
HERE=$(cd "$(dirname "$0")" && pwd)
nice -n 15 gdb -q -batch -x $HERE/thread-trace.gdb --args $TAGDIR/native/two-native $R/binaries/win64/FLEngine_x64.dll "$REAL" > $OUTD/gdb-out.txt 2> $OUTD/gdb-err.txt
grep -aE "^(CT|SNAP) " $OUTD/gdb-out.txt > $OUTD/threads.txt
grep -a "fle-thread:" $OUTD/gdb-err.txt > $OUTD/fle-thread.txt
wc -l $OUTD/threads.txt $OUTD/fle-thread.txt
