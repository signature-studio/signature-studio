#!/bin/bash
# Decision 0167 (provisional): runs a g3 script on the wasm engine in node and writes the g3 lines (g3-snap / g3-surface / g3-wintable / g3-progress) that compare against expected/*.txt with g3cmp.py.
# usage: REPO=<checkout with binaries/win64/FLEngine_x64.dll> DLLDIR=<dir with the five rebased modules: <WORK>/onat/v-*/dll> WASMDIR=<dir with flengine6.js + flengine6.wasm: the NODE link of the objects, see README.md> \
#        [G3=<script>] [OUT=<dir>] bash lift/harness/b2d2/run-g3.sh
# env:   G3 (default docs/investigations/b1/tools/g3/cmp-short.g3: the 16 snapshots of decisions 0151/0161), OUT (default ./g3-run), B2D1_CHECK=1 (the swapped build checks its mirror after every call), PROF=1 (node --cpu-prof into $OUT/prof)
set -u -o pipefail
HERE=$(cd "$(dirname "$0")" && pwd); L=$(cd "$HERE/../.." && pwd)
REPO=${REPO:?}; DLLDIR=${DLLDIR:?}; WASMDIR=${WASMDIR:?}
G3=${G3:-$L/../docs/investigations/b1/tools/g3/cmp-short.g3}
OUT=${OUT:-$PWD/g3-run}; mkdir -p "$OUT"
H=$L/harness
REAL=$DLLDIR/dsp_ippv2_x64.2000000.dll:$DLLDIR/Blend2D.4700000.dll:$DLLDIR/quickfontcache_x64.1d00000.dll:$DLLDIR/freetype_x64.4b00000.dll:$DLLDIR/ILTools_x64.6000000.dll
export FLE=$REPO/binaries/win64/FLEngine_x64.dll FLE_VFS_SEED=$H/seeds/vfs-seed-idle1.tsv FLE_VFS_SEED_ROOT=$REPO
export FLE_REG_SEED=$H/seeds/reg-seed-nsis.tsv:$H/seeds/reg-seed-app.tsv FLE_AUTH_CLAIMS=$H/seeds/authenticode-claims.tsv
export FLE_CALL_FRUITY=1 FLE_FRUITY_MAX_CALLS=400000000000 FLE_FRUITY_SECONDS=3000 FLE_FRUITY_NO_IDLE_STOP=1 FLE_G3_SCRIPT=$G3 FLE_G3_PROGRESS=1
export FLE_G3_WINTABLE_DIR=$OUT/wintable; mkdir -p "$FLE_G3_WINTABLE_DIR"
export FLE_WASM_DIR=$WASMDIR WF1_OUT=$OUT/surfaces WF1_JSON=$OUT/summary.json
PR=""; [ "${PROF:-0}" = 1 ] && PR="--cpu-prof --cpu-prof-dir=$OUT/prof --cpu-prof-interval ${PROF_US:-1000}"
/usr/bin/time -v node --experimental-wasm-jspi $PR --require "$H/b2d2/cpuhook.cjs" "$H/two/run_node_fruity.cjs" "$FLE" "$REAL" > "$OUT/stdout.txt" 2> "$OUT/stderr.txt"
echo "node rc=$? (134 is the normal end of the script)"
grep -a -E "^g3-(snap|surface|wintable|progress)|^fruity: (== stop|runtime message)" "$OUT/stdout.txt" | cut -c1-240 > "$OUT/g3-lines.txt"
wc -l < "$OUT/g3-lines.txt"
