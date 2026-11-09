#!/bin/bash
# alpha1 (decision 0244): run a G3 script on the native N1 binary with the recorder (adapted from pick1's run-native.sh).
# usage: run-n1.sh BIN SCRIPT OUTDIR [DUMPDIR]    env: A1_PIX="x,y;x,y", A1_CROP="x,y,w,h;..." (see alpha1_rec.c), CPU_LIMIT (seconds, default 1500), SRC (tree with lift/harness/seeds), FLE_* extras
# THE INPUT IS A HARNESS SIMULATION (G3 script), NOT A USER OPERATION.
set -u -o pipefail
BIN=$(readlink -f "$1"); SCRIPT=$(readlink -f "$2"); mkdir -p "$3"; OUT=$(readlink -f "$3"); DUMP=${4:-}; [ -n "$DUMP" ] && { mkdir -p "$DUMP"; DUMP=$(readlink -f "$DUMP"); }
REPO=/home/ubuntu/work/signature-studio-develop
A=$REPO/lift/out-alpha1
SRC=${SRC:-$A/src}
DLLD=$A/work/dll
REAL=$DLLD/dsp_ippv2_x64.2000000.dll:$DLLD/Blend2D.4700000.dll:$DLLD/quickfontcache_x64.1d00000.dll:$DLLD/freetype_x64.4b00000.dll:$DLLD/ILTools_x64.6000000.dll
mkdir -p "$OUT/wintable" "$OUT/crops"
cd "$OUT"
export A1_OUT=$OUT/api.log A1_DIR=$OUT/crops
ulimit -t ${CPU_LIMIT:-1500}
export FLE_CALL_FRUITY=1 FLE_VFS_SEED=$SRC/lift/harness/seeds/vfs-seed-g8.tsv FLE_VFS_SEED_ROOT=$REPO FLE_REG_SEED=$SRC/lift/harness/seeds/reg-seed-nsis.tsv:$SRC/lift/harness/seeds/reg-seed-app.tsv \
  FLE_AUTH_CLAIMS=$SRC/lift/harness/seeds/authenticode-claims.tsv FLE_FRUITY_MAX_CALLS=400000000000 FLE_FRUITY_SECONDS=3000 FLE_FRUITY_NO_IDLE_STOP=1 FLE_G3_SCRIPT=$SCRIPT FLE_G3_PROGRESS=1 \
  FLE_G3_WINTABLE_DIR=$OUT/wintable FLE_SEH_HANDLERS=0x112c0
[ -n "$DUMP" ] && export FLE_G3_DUMP_DIR=$DUMP
nice -n 10 ${RUNNER:-} "$BIN" "$REPO/binaries/win64/FLEngine_x64.dll" "$REAL" > "$OUT/stdout.txt" 2> "$OUT/stderr.txt"
echo "rc=$?" > "$OUT/rc.txt"
