#!/bin/bash
# The lift stage of one VST3 module (the gen_vst.sh that gen_vst3a.sh calls): fe-lift lift into units of PER bytes, the dispatch table merged with the host rows
# (merge_tables.mjs), and the real-module table of the image (fle_realmod.h: its lower-case name and its base). No Python.
# usage: OUT=<work dir> TAG=<tag> FILE=<rebased PE> FEBIN=<dir with fe-lift> LIFT_ARGS="--pdata --extra-imports <rows>" ROWS=<host rows>
#        RMNAME=<lower-case module name> RMBASE=<0x base> [PER=4000000] bash lift/harness/vst/gen_vst.sh
# Output: $OUT/src/$TAG-NN.c (units), $OUT/src/$TAG-tables.txt, $OUT/src/common-$TAG.c, $OUT/src/realmod-$TAG.c, $OUT/log/lift-$TAG.{txt,err}
set -u -o pipefail
HERE=$(cd "$(dirname "$0")" && pwd)
OUT=${OUT:?OUT is not set}; TAG=${TAG:?TAG is not set}; FILE=${FILE:?FILE is not set}; FEBIN=${FEBIN:?FEBIN is not set}
ROWS=${ROWS:?ROWS is not set}; RMNAME=${RMNAME:?RMNAME is not set}; RMBASE=${RMBASE:?RMBASE is not set}
PER=${PER:-4000000}; LIFT_ARGS=${LIFT_ARGS:-}
mkdir -p "$OUT/src" "$OUT/log"
rm -f "$OUT/src/$TAG"-[0-9][0-9].c "$OUT/src/$TAG-tables.txt"
"$FEBIN/fe-lift" lift "$FILE" "$OUT/src/$TAG" $LIFT_ARGS --split-bytes "$PER" > "$OUT/log/lift-$TAG.txt" 2> "$OUT/log/lift-$TAG.err" || { echo "gen: lift failed (see $OUT/log/lift-$TAG.err)" >&2; exit 1; }
node "$HERE/merge_tables.mjs" "$OUT/src/$TAG-tables.txt" "$ROWS" "$OUT/src/common-$TAG.c" || { echo "gen: merge failed" >&2; exit 1; }
printf '/* the real module of this image (fle_realmod.h) */\n#include "fle_realmod.h"\nconst fle_realmod_entry fle_realmod_table[] = {\n    { "%s", UINT64_C(%s) },\n};\nconst uint64_t fle_realmod_count = 1;\n' "$RMNAME" "$RMBASE" > "$OUT/src/realmod-$TAG.c"
echo "gen: units $(ls "$OUT/src/$TAG"-[0-9][0-9].c 2>/dev/null | wc -l)  $(grep -h '^functions\|^unsupported_total' "$OUT/log/lift-$TAG.txt" | tr -s ' ' | tr '\n' ' ')"
