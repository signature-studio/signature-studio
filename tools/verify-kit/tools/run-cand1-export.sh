#!/bin/bash
# run-cand1-export.sh NAME PORT OUTDIR   (ci1, 2026-11-07; started only on the PM's signal, Chrome 1 at a time; Wine 0 and other Chrome 0 checked)
# NAME = assets-NAME (cand1-orig = the original FLP, cand1-ins1lp = alpha1's ins1-lp.flp under the name serina1-serum2.flp); the server for PORT is already up (serve.sh PORT NAME d0447902).
# One Export (Ctrl+R, scn-export-g9a.json: engine log saved too) with the page of the production E13 (page-d0447902) and the cand1 test bundle; then the WAV size, frames and sha256 are printed and noted in out/steps.log.
set -u
. /home/ubuntu/work/signature-studio-develop/lift/out-ci1/kit/env.sh
N=$1; P=$2; D=$3
note() { echo "$(date +%T) $* load $(cut -d' ' -f1 /proc/loadavg) io $(io)" | tee -a $K/out/steps.log; }
[ -d "$K/assets-$N" ] || { echo "no assets-$N"; exit 2; }
curl -s -o /dev/null -w '%{http_code}' http://127.0.0.1:$P/ | grep -q 200 || { echo "server on $P is not up"; exit 3; }
W=$(ps -eo args | grep -E 'FL64|wineserver' | grep -v grep | wc -l); C=$(tail -1 $K/out/chrome-count.tsv | cut -f2)
[ "$W" = 0 ] || [ -n "${ALLOW_WINE:-}" ] || { echo "stop: wine=$W"; exit 4; }   # the PM allowed the overlap with opt1's Wine reference runs for this deterministic Export (2026-11-07 09:4x)
[ "$C" = 0 ] || [ -n "${ALLOW_OTHER_CHROME:-}" ] || { echo "stop: other-chrome=$C"; exit 4; }   # the PM allowed one overlap with gpu1's untimed GUI look on 2026-11-07 09:2x (the Export is deterministic, not timed)
MA=$(awk '/^MemAvailable:/ {print int($2/1024)}' /proc/meminfo); [ "$MA" -ge 12000 ] || { echo "stop: MemAvailable $MA MB < 12000"; exit 6; }
{ echo "$(date +%T) export $N pre-run note"; free -m | sed -n 2,3p; vmstat 1 3 | tail -2; } | tee -a $K/out/steps.log
Q=$(node $K/tools/wait-quiet.mjs 12 15) || { echo "not quiet: $Q"; exit 5; }
rm -rf "$D"; mkdir -p "$D"
note "export $N start port $P ($Q)"
node $K/tools/sdriveg2-export.mjs --browser chrome --url http://127.0.0.1:$P/ --out $D --app $K/page-${PAGE:-d0447902}/app --scenarios $K/tools/${SCN:-scn-export-g9a.json} --viewport 2260x1116 > $D.log 2>&1; RC=$?
note "export $N end rc=$RC"
ls -la $D/export.wav 2>/dev/null; python3 - "$D" <<'PY'
import sys, json, hashlib, os
d = sys.argv[1]; w = d + '/export.wav'
if os.path.exists(w):
    b = open(w, 'rb').read(); print('wav bytes', len(b), 'sha256', hashlib.sha256(b).hexdigest())
j = d + '/export.json'
print(open(j).read()[:400] if os.path.exists(j) else 'no export.json')
PY
