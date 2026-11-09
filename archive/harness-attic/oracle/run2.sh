#!/bin/sh
# A5 oracle v2, one command (decision 0036): build the v2 drivers, take the pure-compute pool of
# run.sh (results/pure.json, created by select_pure.py when missing), detect pointer arguments
# statically (argptr.py), count the exclusion classes (exclude_stats.py), run the native-vs-lift
# differential with pointer-aware inputs, EFLAGS/MXCSR/x87 comparison and the executed-byte
# negative control (run_diff2.py). Output: lift/out-oracle/results/$OUTDIR (default run4).
# Env: FLENGINE_PE, PARTS_NAME (after-pdata-b), COUNT (1000), N (16), SEED (1), NEGCTL (200),
# JOBS (16), OUTDIR (run4).
set -eu
HERE=$(cd "$(dirname "$0")" && pwd)
LIFT=$(cd "$HERE/../.." && pwd)
ROOT=$(cd "$LIFT/.." && pwd)
OUT="$LIFT/out-oracle"
PE=${FLENGINE_PE:-$ROOT/binaries/win64/FLEngine_x64.dll}
NAME=${PARTS_NAME:-after-pdata-b}
RES="$OUT/results/${OUTDIR:-run4}"
mkdir -p "$OUT/tmp" "$RES"
TMPDIR="$OUT/tmp"
export TMPDIR

PARTS_NAME="$NAME" "$HERE/build2.sh"
[ -e "$OUT/results/pure.json" ] || python3 "$HERE/select_pure.py" "$PE" "$LIFT/out-pdata/out/$NAME-part-00.c" "$OUT/results/pure.json"
python3 "$HERE/argptr.py" "$PE" "$OUT/results/pure.json" "$RES/argptr.json"
python3 "$HERE/exclude_stats.py" "$PE" "$LIFT/out-pdata/out/$NAME-part-00.c" "$RES/exclude-stats.json" > "$RES/exclude-stats.txt"
python3 "$HERE/run_diff2.py" --pe "$PE" --pure "$OUT/results/pure.json" --argptr "$RES/argptr.json" \
    --exclude "$RES/exclude-stats.json" --out "$RES" --count "${COUNT:-1000}" --n "${N:-16}" \
    --seed "${SEED:-1}" --negctl "${NEGCTL:-200}" --jobs "${JOBS:-16}"
