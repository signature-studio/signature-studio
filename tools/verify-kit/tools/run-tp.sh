#!/bin/bash
# run-tp.sh COND   (ci1, 2026-11-08, PM 05:45: non-timed diagnostic: Chrome trace around File-menu clicks during playback)
# COND = e13 (5053, page-d0447902) | rtf (5122, page-rt1). Gate (PM relaxed for this diagnostic): load <= 12, swap in/out 0, MemAvailable >= 12 GB, other automated Chrome 0; an opt1 Wine overlap is allowed and recorded.
set -u
. /home/ubuntu/work/signature-studio-develop/lift/out-ci1/kit/env.sh
V=$1; RND=${RND:-1}
case "$V" in e13) port=5053; ref=d0447902;; rtf) port=5122; ref=rt1;; rtg-gil) port=5142; ref=rt1;; rth) port=5143; ref=rt1;; *) echo "COND e13|rtf|rtg-gil|rth"; exit 2;; esac
d=$K/out/tp-$RND-$V
read -r e others mine total < <(tail -n 1 $K/out/chrome-count.tsv); age=$(( $(date +%s%3N) - e ))
[ "$age" -lt 15000 ] || { echo "GATE: chrome-count observer stale"; exit 4; }
[ "$others" = 0 ] || { echo "GATE: $others other Chrome"; exit 4; }
curl -sf -o /dev/null http://127.0.0.1:$port/ || { echo "GATE: server $port down"; exit 4; }
W=$(ps -eo args | grep -E 'FL64|wineserver' | grep -v grep | wc -l)
read -r si so < <(vmstat 1 3 | tail -1 | awk '{print $7, $8}'); avail=$(awk '/^MemAvailable:/{print int($2/1024)}' /proc/meminfo); ld=$(cut -d' ' -f1 /proc/loadavg)
{ echo "$(date +%T) tp $V pre-run: load $ld, MemAvailable $avail MB, swap si/so $si/$so, wine procs $W"; free -m | sed -n 2,3p; vmstat 1 3 | tail -2; cat /proc/pressure/io | head -1; } | tee -a $K/out/steps.log
[ "$si" = 0 ] && [ "$so" = 0 ] || { echo "GATE: swap $si/$so"; exit 4; }
[ "$avail" -ge 12000 ] || { echo "GATE: MemAvailable $avail < 12000"; exit 4; }
awk -v l="$ld" 'BEGIN{exit !(l<=12)}' || { echo "GATE: load $ld > 12"; exit 4; }
rm -rf $d; mkdir -p $d; { echo "run tp $V"; echo "start $(date -Iseconds)"; echo "load $ld avail $avail si/so $si/$so wine $W"; } > $d/gate.txt
node $K/tools/sdriveg2-lat-tp.mjs --browser chrome --url http://127.0.0.1:$port/ --out $d --app $K/page-$ref/app --scenarios $K/tools/${SCN:-scn-tp.json} --only tp-play --viewport 2260x1116 > $d.log 2>&1
rm -rf $d/profile-sdrive $d/netlog-sdrive.json
echo "$(date +%T) tp $V end load $(cut -d' ' -f1 /proc/loadavg)" | tee -a $K/out/steps.log
