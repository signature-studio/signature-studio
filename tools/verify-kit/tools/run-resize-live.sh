#!/bin/bash
# run-resize-live.sh NAME PORT PAGEREF OUTDIR   (ci1, 2026-11-07; PM 09:15 (3-ii); only on the PM's signal; one Chrome)
# The live-resize check with the wizard closed: scn-resize-live.json on the page PAGEREF served on PORT (assets-NAME). Prints the evals (canvas bitmap, compositor's picture, engine state) at the three points.
set -u
. "$(dirname "${BASH_SOURCE[0]}")/../env.sh"
N=$1; P=$2; R=$3; D=$4
note() { echo "$(date +%T) $* load $(cut -d' ' -f1 /proc/loadavg) io $(io)" | tee -a $K/out/steps.log; }
curl -s -o /dev/null -w '%{http_code}' http://127.0.0.1:$P/ | grep -q 200 || { echo "server on $P is not up"; exit 3; }
W=$(ps -eo args | grep -E 'FL64|wineserver' | grep -v grep | wc -l); C=$(tail -1 $K/out/chrome-count.tsv | cut -f2)
[ "$W" = 0 ] && [ "$C" = 0 ] || { echo "stop: wine=$W other-chrome=$C"; exit 4; }
{ echo "$(date +%T) resize-live $N pre-run note"; free -m | sed -n 2,3p; vmstat 1 3 | tail -2; } | tee -a $K/out/steps.log
Q=$(node $K/tools/wait-quiet.mjs 12 15) || { echo "not quiet: $Q"; exit 5; }
rm -rf "$D"; mkdir -p "$D"
note "resize-live $N start port $P page-$R ($Q)"
node $K/tools/sdriveg2-export.mjs --browser chrome --url http://127.0.0.1:$P/ --out $D --app $K/page-$R/app --scenarios $K/tools/scn-resize-live.json --viewport 2260x1116 > $D.log 2>&1; RC=$?
note "resize-live $N end rc=$RC"
python3 - "$D" <<'PY'
import sys, json, os
d = sys.argv[1]; r = d + '/report.json'
if os.path.exists(r):
    j = json.load(open(r))
    def find(o, key, out):
        if isinstance(o, dict):
            if key in o: out.append(o[key])
            for v in o.values(): find(v, key, out)
        elif isinstance(o, list):
            for v in o: find(v, key, out)
    ev = []; find(j, 'evals', ev)
    for lst in ev:
        for e in lst: print(json.dumps(e)[:400])
else: print('no report.json')
PY
grep -c 'callback nesting' $D/engine-log.log 2>/dev/null
