#!/bin/bash
# Repeats: lift (report only) -> seeds for the rejected MSVC jump tables -> add to the seed file, until no new seed appears. usage: seed_fixpoint.sh FILE START_SEEDS OUT_SEEDS
set -u
HERE=$(cd "$(dirname "$0")" && pwd)
FILE=$1; START=$2; OUT=$3
FE=${FE:-$HERE/../target/release/fe-lift}
cp "$START" "$OUT.cur"
for i in 1 2 3 4 5 6 7 8; do
    $FE lift "$FILE" /dev/null --pdata --seeds "$OUT.cur" ${EXTRA:+--extra-imports $EXTRA} > "$OUT.rep$i" 2>/dev/null
    rm -f "$OUT.new$i"
    python3 "$HERE/rejected_seeds.py" "$FILE" "$OUT.rep$i" "$OUT.new$i" | tail -1
    cat "$OUT.cur" "$OUT.new$i" | sort -u > "$OUT.next"
    if cmp -s "$OUT.cur" "$OUT.next"; then echo "fixpoint after $i rounds"; break; fi
    mv "$OUT.next" "$OUT.cur"
    grep -E "^(functions|bad_nodes|msvc_rejected  )" "$OUT.rep$i" | head -3
done
cp "$OUT.cur" "$OUT"
wc -l "$OUT"
