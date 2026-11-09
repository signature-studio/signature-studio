#!/bin/bash
# pick1: run a G3 script on the native N1-API binary (the generated C of the production-shaped F2xH7 build, clang -O2, runtime -O2, upstream Blend2D, API recorder linked in).
# usage: run-native.sh BIN SCRIPT OUTDIR [DUMPDIR]    env: PICK1_REPO (checkout with binaries/ and lift/out-inventory/; default /home/ubuntu/work/signature-studio-develop), PICK1_DLLD, PROF=1 (SIGPROF sampler), PROF_US
# THE INPUT IS A HARNESS SIMULATION (G3 script), NOT A USER OPERATION.
set -u -o pipefail
BIN=$(readlink -f "$1"); SCRIPT=$(readlink -f "$2"); mkdir -p "$3"; OUT=$(readlink -f "$3"); DUMP=${4:-}; [ -n "$DUMP" ] && { mkdir -p "$DUMP"; DUMP=$(readlink -f "$DUMP"); }
REPO=${PICK1_REPO:-/home/ubuntu/work/signature-studio-develop}
SRC=${PICK1_SRC:-$(cd "$(dirname "$0")/../src" && pwd)}
DLLD=${PICK1_DLLD:-$(cd "$(dirname "$0")/.." && pwd)/work/onat/v-tb2d2/dll}
REAL=$DLLD/dsp_ippv2_x64.2000000.dll:$DLLD/Blend2D.4700000.dll:$DLLD/quickfontcache_x64.1d00000.dll:$DLLD/freetype_x64.4b00000.dll:$DLLD/ILTools_x64.6000000.dll
mkdir -p "$OUT/wintable"
cd "$OUT"
export APIW_OUT=$OUT/api.log
[ "${PROF:-0}" = 1 ] && export PARITY_PROF=$OUT/prof.bin PARITY_PROF_US=${PROF_US:-1000}
export PARITY_BLIT_LOG=$OUT/blit.bin
ulimit -t ${CPU_LIMIT:-900}
export FLE_CALL_FRUITY=1 FLE_VFS_SEED=$SRC/lift/harness/seeds/vfs-seed-g8.tsv FLE_VFS_SEED_ROOT=$REPO FLE_REG_SEED=$SRC/lift/harness/seeds/reg-seed-nsis.tsv:$SRC/lift/harness/seeds/reg-seed-app.tsv \
  FLE_AUTH_CLAIMS=$SRC/lift/harness/seeds/authenticode-claims.tsv FLE_FRUITY_MAX_CALLS=400000000000 FLE_FRUITY_SECONDS=3000 FLE_FRUITY_NO_IDLE_STOP=1 FLE_G3_SCRIPT=$SCRIPT FLE_G3_PROGRESS=1 \
  FLE_G3_WINTABLE_DIR=$OUT/wintable FLE_SEH_HANDLERS=0x112c0
[ -n "$DUMP" ] && export FLE_G3_DUMP_DIR=$DUMP
nice -n 10 ${RUNNER:-} "$BIN" "$REPO/binaries/win64/FLEngine_x64.dll" "$REAL" > "$OUT/stdout.txt" 2> "$OUT/stderr.txt"
echo "rc=$?" > "$OUT/rc.txt"
