#!/bin/bash
# lat1: alternating runs, one Chrome at a time. usage: series.sh <series-name> "<config order round 1>" "<order round 2>" ... (each order is a comma list)
W=/home/ubuntu/work/signature-studio-develop/lift/out-lat1
NAME="$1"; shift
OUT="$W/runs/$NAME"; mkdir -p "$OUT"
r=0
for order in "$@"; do
  r=$((r+1))
  for cfg in ${order//,/ }; do
    d="$OUT/$cfg-r$r"; mkdir -p "$d"
    echo "== $(date -Is) $NAME $cfg round $r load $(cut -d' ' -f1-3 /proc/loadavg)" >> "$OUT/series.log"
    f8=4; [ "$cfg" = SHIPH ] && f8=12
    (cd "$W" && nice -n 5 node tools/run.mjs --config "$cfg" --round "$r" --reps 12 --f8reps "$f8" --menu-reps 7 --max-wait-min "${MAXWAIT:-4}" --out "$d" --port $((4420 + r)) > "$d/log.txt" 2>&1)
    echo "   exit $? $(date -Is) load $(cut -d' ' -f1-3 /proc/loadavg)" >> "$OUT/series.log"
  done
done
echo "== done $(date -Is)" >> "$OUT/series.log"
