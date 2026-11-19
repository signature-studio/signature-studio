#!/bin/bash
# run-o19.sh PROJ COND COMPARE...   (ci1, 2026-11-07, PM 12:4x O19): the owner's conditions (Mixer window shown, mouse moved) for the cooperative E13 and the real-thread RT-A (N=8): scenario o19-PROJ-COND of tools/scn-o19.json (PROJ = song|solo12, COND = hidden|shown|mixmove|plmove), ONE scenario per run (--only), on the page of the condition.
#   run-o19.sh solo12 mixmove e13      run-o19.sh solo12 mixmove rta
# Output out/o19-PROJ-COND-ROUND-VARIANT/ (VARIANT = e13 | rta); the gate of run-rta.sh (other Chrome 0, Wine 0, swap I/O 0, MemAvailable >= 12 GB for this run, load <= 8, I/O pressure 0). Timed: only in the window the PM names.
set -u
. "$(dirname "${BASH_SOURCE[0]}")/../env.sh"
PROJ=$1; COND=$2; V=$3; RND=${RND:-1}
case "$V" in e13) port=5053; ref=d0447902;; rta) port=5101; ref=rt1;; rtb) port=5111; ref=rt1;; rte) port=5121; ref=rt1;; rtf-gil) port=5140; ref=rt1;; rtf-gil2) port=5141; ref=rt1;; *) echo "variant e13|rta|rtb|rte"; exit 2;; esac
d=$K/out/${OUTTAG:-o19-$PROJ}-$COND-$RND-$V
log() { echo "$(date +%T) $* load $(cut -d' ' -f1 /proc/loadavg) io $(io)" | tee -a $K/out/steps.log; }
read -r e others mine total < <(tail -n 1 $K/out/chrome-count.tsv); age=$(( $(date +%s%3N) - e ))
[ "$age" -lt 15000 ] || { echo "GATE: the chrome-count observer is stale"; exit 4; }
[ "$others" = 0 ] || [ -n "${ALLOW_OTHER_CHROME:-}" ] || { echo "GATE: $others other Chrome"; exit 4; }   # ALLOW_OTHER_CHROME=1 only for the untimed check of the coordinates (PM 13:1x): the values of such a run are not timing values
[ "$(ps -eo args | grep -E 'FL64|wineserver' | grep -v grep | wc -l)" = 0 ] || { echo "GATE: Wine"; exit 4; }
curl -sf -o /dev/null http://127.0.0.1:$port/ || { echo "GATE: server $port down"; exit 4; }
for try in 1 2 3; do
  q=$(node $K/tools/wait-quiet.mjs 8 15) || { echo "GATE: not quiet: $q"; exit 4; }
  { echo "$(date +%T) o19 $PROJ $COND $V pre-run note (try $try)"; free -m | sed -n 2,3p; vmstat 1 3 | tail -2; } | tee -a $K/out/steps.log
  read -r si so < <(vmstat 1 3 | tail -1 | awk '{print $7, $8}'); avail=$(awk '/^MemAvailable:/{print int($2/1024)}' /proc/meminfo)
  [ "$avail" -ge 12000 ] || { echo "GATE: MemAvailable $avail < 12000"; exit 4; }
  [ "$si" = 0 ] && [ "$so" = 0 ] && break; echo "GATE: swap $si/$so"; [ $try = 3 ] && exit 4
done
rm -rf $d; mkdir -p $d; { echo "run $PROJ $COND $V"; echo "start $(date -Iseconds)"; echo "$q"; } > $d/gate.txt; log "o19 $PROJ $COND $V start ($q)"
node $K/tools/sdriveg2-lat.mjs --browser chrome --url http://127.0.0.1:$port/ --out $d --app $K/page-$ref/app --scenarios $K/tools/${SCN:-scn-o19b.json} --only ${ONLYNAME:-o19b-$PROJ-$COND} --viewport 2260x1116 > $d.log 2>&1
rm -rf $d/profile-sdrive $d/netlog-sdrive.json; log "o19 $PROJ $COND $V end"
