#!/bin/sh
# A5 oracle v2 re-run with the MXCSR status flags and PREFETCHh (decision 0038). Same selection, seed, inputs and
# comparison as run2.sh (run4); only the lifted side is rebuilt from the current lifter and runtime
# (build5.sh -> lift/out-mxcsr/oracle/bin). Output: lift/out-oracle/results/run5 (run4 is not touched).
# Reuses results/pure.json and run4's argptr.json / exclude-stats.json (inputs, unchanged).
# Env: FLENGINE_PE, COUNT (1000), N (16), SEED (1), NEGCTL (200), JOBS (16), OUTDIR (run5), BUILD (1: run build5.sh).
set -eu
HERE=$(cd "$(dirname "$0")" && pwd)
LIFT=$(cd "$HERE/../.." && pwd)
ROOT=$(cd "$LIFT/.." && pwd)
OUT="$LIFT/out-oracle"
PE=${FLENGINE_PE:-$ROOT/binaries/win64/FLEngine_x64.dll}
RES="$OUT/results/${OUTDIR:-run5}"
mkdir -p "$OUT/tmp" "$RES"
TMPDIR="$OUT/tmp"
export TMPDIR
ORACLE_BIN="$LIFT/out-mxcsr/oracle/bin"
export ORACLE_BIN
[ "${BUILD:-1}" = 1 ] && "$HERE/build5.sh"
cp -n "$OUT/results/run4/argptr.json" "$RES/argptr.json"
cp -n "$OUT/results/run4/exclude-stats.json" "$RES/exclude-stats.json"
python3 "$HERE/run_diff2.py" --pe "$PE" --pure "$OUT/results/pure.json" --argptr "$RES/argptr.json" \
    --exclude "$RES/exclude-stats.json" --out "$RES" --count "${COUNT:-1000}" --n "${N:-16}" \
    --seed "${SEED:-1}" --negctl "${NEGCTL:-200}" --jobs "${JOBS:-16}"
