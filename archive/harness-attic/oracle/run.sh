#!/bin/sh
# A5 oracle, one command: build both drivers, select pure-compute functions, run the
# native-vs-lift differential, write lift/out-oracle/results/. Output stays under lift/out-oracle/.
# Env: FLENGINE_PE (default <repo>/binaries/win64/FLEngine_x64.dll), PARTS_NAME (default
# after-pdata-b), COUNT (200), N (12), SEED (1), NEGCTL (24), JOBS (16), OUTDIR (results/run3).
set -eu
HERE=$(cd "$(dirname "$0")" && pwd)
LIFT=$(cd "$HERE/../.." && pwd)
ROOT=$(cd "$LIFT/.." && pwd)
OUT="$LIFT/out-oracle"
PE=${FLENGINE_PE:-$ROOT/binaries/win64/FLEngine_x64.dll}
NAME=${PARTS_NAME:-after-pdata-b}
mkdir -p "$OUT/tmp" "$OUT/results"
TMPDIR="$OUT/tmp"
export TMPDIR

PARTS_NAME="$NAME" "$HERE/build.sh"
python3 "$HERE/select_pure.py" "$PE" "$LIFT/out-pdata/out/$NAME-part-00.c" "$OUT/results/pure.json"
python3 "$HERE/run_diff.py" --pe "$PE" --pure "$OUT/results/pure.json" --out "$OUT/results/${OUTDIR:-run3}" \
    --count "${COUNT:-200}" --n "${N:-12}" --seed "${SEED:-1}" --negctl "${NEGCTL:-24}" --jobs "${JOBS:-16}"
