#!/bin/bash
# parity gate (decision 0226): CPU seconds per virtual second of the SAME G3 input list on a wasm bundle (node, virtual clock) and, when a native N1 binary is given, on the native -O2 build of the same generated C.
# THE INPUT IS A HARNESS SIMULATION (G3 scripts generated from ops.tsv), NOT A USER OPERATION.
#
#   lift/harness/parity/run.sh --bundle DIR [--native BIN] [--sessions "boot ui-pl ..."] [--out DIR] [--quick] [--chrome ASSETS_NAME]
#     --bundle DIR      a bundle directory (manifest.json + engine glue + wasm + packs), e.g. lift/out-websrv/assets/fl-engine-ship2-F2xH2 or build/bundle/fl-parity1-v1
#     --native BIN      the N1 binary (tools/native-units.sh + tools/native-link.sh of lift/out-parity1; optional: without it only the absolute budgets are checked)
#     --quick           sessions boot + ui-pl only (about 3 minutes of CPU)
#     --thresholds F    another thresholds file (default expected/thresholds.tsv = the hybrid + RT_OPT=O2 + FAST_COPY state, V1H x 2.0; expected/thresholds-nonhybrid.tsv = V1 x 2.0 for a bundle without the hybrid)
#     --chrome NAME     also run chrome-ops.mjs on /assets/NAME/manifest.json (needs app/node_modules, a vite build in $LAT1_DIST, Chrome; about 4 minutes)
#   env: PARITY_REPO (repository root with binaries/ and lift/; default: the checkout this script is in), CPU_LIMIT (seconds of CPU per run before it is cut, default 400)
#   exit code: 0 = every row passes, 1 = a row exceeds a threshold of expected/thresholds.tsv, 2 = a run failed.
# Duration (this machine, 16 cores, load 20-40, ONE run at a time): about 1.2 minutes of wall time per wasm session and 0.5 minute per native session, 10 default sessions (56 rows): about 6-8 minutes at load 10-15, 16 minutes at load 40-65 (--quick: 3). A WARN is printed when the 1-minute load reaches 20 during the run: CPU seconds are then unreliable.
set -u -o pipefail
HERE=$(cd "$(dirname "$0")" && pwd); REPO=${PARITY_REPO:-$(cd "$HERE/../../.." && pwd)}
THR=; BUNDLE=; NATIVE=; SESSIONS="boot ui-pl ui-cr ui-mx ui-mxd ui-br ui-menu ui-f8 ui-pr ui-win"; OUT=$PWD/parity-out; QUICK=0; CHROME=
while [ $# -gt 0 ]; do case "$1" in --bundle) BUNDLE=$2; shift 2;; --native) NATIVE=$2; shift 2;; --sessions) SESSIONS=$2; shift 2;; --out) OUT=$2; shift 2;; --quick) QUICK=1; shift;; --chrome) CHROME=$2; shift 2;; --thresholds) THR=$2; shift 2;; *) echo "unknown option $1"; exit 2;; esac; done
[ -n "$BUNDLE" ] || { echo "usage: run.sh --bundle DIR [--native BIN] ..."; exit 2; }
[ "$QUICK" = 1 ] && SESSIONS="boot ui-pl"
mkdir -p "$OUT/scripts" "$OUT/runs"
python3 "$HERE/mk-scripts.py" "$OUT/scripts" > "$OUT/mk-scripts.txt" || exit 2
export CPU_LIMIT=${CPU_LIMIT:-400}
SEED=$REPO/lift/harness/seeds/vfs-seed-g8.tsv
rc=0
MAXLOAD=0; ld() { l=$(cut -d" " -f1 /proc/loadavg); echo "$l" >> "$OUT/load-samples.txt"; MAXLOAD=$(python3 -c "print(max($MAXLOAD, $l))"); }
: > "$OUT/load-samples.txt"
for s in $SESSIONS; do
  ld
  # wasm (node, JSPI): the driver is tools/node-bundle-run.mjs of lift/out-parity1 (copied next to this script as node-bundle-run.mjs)
  W=$OUT/runs/w-$s; mkdir -p $W
  ( ulimit -t $CPU_LIMIT; PARITY_SEED_TSV=$SEED PARITY_REPO=$REPO nice -n 10 node --experimental-wasm-jspi "$HERE/node-bundle-run.mjs" "$BUNDLE" "$(ls $BUNDLE/seed.*.flpack | head -1)" "$OUT/scripts/$s.wasm.g3" "$W" FLE_VFS_SEED=/data/seed/vfs-seed-idle1.tsv > $W/driver.out 2> $W/driver.err )
  grep -q "^g3-snap: #[0-9]* end" "$W/stdout.txt" 2>/dev/null || { echo "session $s (wasm): did not reach the end"; rc=2; }
  if [ -n "$NATIVE" ]; then
    N=$OUT/runs/n-$s; mkdir -p $N/wintable
    DLLD=$(ls -d "$(dirname "$NATIVE")"/../onat/v-*/dll 2>/dev/null | head -1); [ -n "$DLLD" ] || DLLD=${PARITY_DLLDIR:-}
    REAL=$DLLD/dsp_ippv2_x64.2000000.dll:$DLLD/Blend2D.4700000.dll:$DLLD/quickfontcache_x64.1d00000.dll:$DLLD/freetype_x64.4b00000.dll:$DLLD/ILTools_x64.6000000.dll
    ( ulimit -t $CPU_LIMIT; cd $N; FLE_CALL_FRUITY=1 FLE_VFS_SEED=$SEED FLE_VFS_SEED_ROOT=$REPO FLE_REG_SEED=$REPO/lift/harness/seeds/reg-seed-nsis.tsv:$REPO/lift/harness/seeds/reg-seed-app.tsv \
        FLE_AUTH_CLAIMS=$REPO/lift/harness/seeds/authenticode-claims.tsv FLE_FRUITY_MAX_CALLS=400000000000 FLE_FRUITY_SECONDS=3000 FLE_FRUITY_NO_IDLE_STOP=1 FLE_G3_SCRIPT=$OUT/scripts/$s.native.g3 FLE_G3_PROGRESS=1 \
        FLE_G3_WINTABLE_DIR=$N/wintable FLE_SEH_HANDLERS=0x112c0 nice -n 10 "$NATIVE" "$REPO/binaries/win64/FLEngine_x64.dll" "$REAL" > $N/stdout.txt 2> $N/stderr.txt )
    python3 "$HERE/summarize.py" "$N/stdout.txt" "$W/stdout.txt" --tsv "$OUT/matrix-$s.tsv" --md "$OUT/matrix-$s.md" 2> $OUT/summarize-$s.err || { echo "session $s: summarize failed"; rc=2; }
  else
    python3 "$HERE/wasm-table.py" "$W/stdout.txt" > "$OUT/matrix-$s.tsv" || rc=2
  fi
done
ld
[ "$(python3 -c "print(int($MAXLOAD > 20))")" = 1 ] && echo "WARN: the 1-minute load reached $MAXLOAD (> 20) during this run: CPU seconds and ratios are inflated and unreliable (the same bundle varies 0.59-1.81x between runs); only the pixel-identity result is valid. Re-run on a quieter machine before reading a speed result (decision 0226 section 8)."
python3 "$HERE/check.py" "$OUT" "${THR:-$HERE/expected/thresholds.tsv}" ${NATIVE:+--native} || rc=1
if [ -n "$CHROME" ]; then
  ( LAT1_APP=${LAT1_APP:-$REPO/app} LAT1_DIST=${LAT1_DIST:-$REPO/app/dist} LAT1_ASSETS=${LAT1_ASSETS:-$REPO/lift/out-websrv/assets} nice -n 10 node "$HERE/chrome-ops.mjs" --out "$OUT/chrome" --query "?bundle=/assets/$CHROME/manifest.json" --max-load ${MAX_LOAD:-100} > "$OUT/chrome.txt" 2>&1 ) || rc=2
fi
echo "parity gate: rc=$rc (tables in $OUT)"; exit $rc
