#!/bin/bash
# Generation stage of the single-PE build (decision 0156): lift the PE, split the C into units of ~$PER bytes, make common.c (R/FL/X, dispatch
# table, response table, optional harness rows). Nothing is deleted or overwritten: use a new TAG for a new seed set / row set.
# usage: OUT=lift/out-vst2 TAG=ott FILE=lift/out-vst1/bin/OTT.vst3 FEBIN=<dir with fe-lift> [LIFT_ARGS="--pdata --seeds X"] [ROWS=<rows file>] [PER=4000000] bash lift/harness/vst/gen_vst.sh
set -u -o pipefail
HERE=$(cd "$(dirname "$0")" && pwd)
LIFT=${LIFT:-$(cd "$HERE/../.." && pwd)}
OUT=${OUT:?}; TAG=${TAG:?}; FILE=${FILE:?}; FEBIN=${FEBIN:?}
LIFT_ARGS=${LIFT_ARGS:---pdata}
ROWS=${ROWS:-}
PER=${PER:-4000000}
mkdir -p "$OUT/gen" "$OUT/src" "$OUT/log"
[ -e "$OUT/gen/$TAG.c" ] || "$FEBIN/fe-lift" lift "$FILE" "$OUT/gen/$TAG.c" $LIFT_ARGS > "$OUT/log/gen-$TAG.txt" 2> "$OUT/log/gen-$TAG.err" || exit 1
echo "gen: $OUT/gen/$TAG.c sha256 $(sha256sum "$OUT/gen/$TAG.c" | cut -d' ' -f1)"
[ -e "$OUT/src/$TAG-00.c" ] || python3 "$LIFT/harness/two/split_bytes.py" "$OUT/gen/$TAG.c" "$OUT/src/$TAG" "$PER" || exit 1
if [ ! -e "$OUT/src/common-$TAG.c" ]; then
    python3 "$HERE/mk_common.py" "$OUT/src/common-$TAG.c" "$OUT/gen/$TAG.c" ${ROWS:+--extra-rows "$ROWS"} || exit 1
fi
