#!/bin/bash
# run-s3a.sh MODE ...   (S3a sweep, 0312c 5c; ci1 0326). Chrome runs start ONLY on the PM's signal; this script never starts anything by itself: it is called by hand, one slot at a time.
#   plan [ROUNDS]       out/s3a-plan.tsv (tools/s3a-plan.py): 5 conditions x 2 scenarios a round
#   servers             prints the 4 commands that start the servers of the B2 conditions (run each with exec in the background); the control e13 is the production server on 5053
#   check               files and servers only (no Chrome): the assets dirs, FLE_WORKERS in every manifest, the projects, the page, the servers answer, the solo project exists
#   next [N]            the next N todo runs of the plan (default 1; a run is about 7 minutes: 2 runs fit a 15 minute slot). Each run: the gate, then tools/sdriveg2-lat.mjs with scn-s3a-<scenario>.json against the condition's server
#   summary             tools/s3a-summary.py (the table of every out/s3a-r*/)
#   export COND         the Export of the reference project (Chrome, about 20 minutes) against the condition's server: the sha256 of the WAV (expected 381cdb5a...f979)
# The gate of a run (any failure: the run does not start, the line stays todo): no other Chrome (the chrome-count observer's last line, fresh), swap in/out 0 and MemAvailable >= 8 GB (vmstat 1 3 and /proc/meminfo, written to out/steps.log), wait-quiet (load <= 8, I/O pressure 0, swap 0).
. /home/ubuntu/work/signature-studio-develop/lift/out-ci1/kit/env.sh
REF=${REF:-d0447902}; PLAN=$K/out/s3a-plan.tsv; DRY=${DRY:-}
mode=${1:-help}; shift || true
log() { echo "$(date +%T) $* load $(cut -d' ' -f1 /proc/loadavg) io $(io)" | tee -a $K/out/steps.log; }
port_of() { awk -F'\t' -v c="$1" '$3==c{print $5; exit}' $PLAN; }
gate() {   # $1 = port
  local others age si so avail
  if [ -s $K/out/chrome-count.tsv ]; then
    read -r e others mine total < <(tail -n 1 $K/out/chrome-count.tsv); age=$(( $(date +%s%3N) - e ))
    [ "$age" -lt 15000 ] || { echo "GATE: the chrome-count observer is stale (${age} ms): start it (node tools/chrome-count.mjs out/chrome-count.tsv 2000, in the background)"; return 1; }
    [ "$others" = 0 ] || { echo "GATE: $others other Chrome browser process(es) are running: not now"; return 1; }
  else echo "GATE: no out/chrome-count.tsv: start the observer"; return 1; fi
  free -m | sed -n 2,3p >> $K/out/steps.log; vmstat 1 3 | tail -2 >> $K/out/steps.log
  read -r si so < <(vmstat 1 3 | tail -1 | awk '{print $7, $8}'); avail=$(awk '/^MemAvailable:/{print int($2/1024)}' /proc/meminfo)
  [ "$si" = 0 ] && [ "$so" = 0 ] || { echo "GATE: swap in/out is $si/$so: not now"; return 1; }
  [ "$avail" -ge 8192 ] || { echo "GATE: MemAvailable $avail MB < 8192: not now"; return 1; }
  curl -sf -o /dev/null http://127.0.0.1:$1/ || { echo "GATE: the server on port $1 is down: start it (run-s3a.sh servers)"; return 1; }
  q=$(node $K/tools/wait-quiet.mjs ${MAXLOAD:-8} ${MAXWAIT:-15}) || { echo "GATE: not quiet: $q"; return 1; }
  return 0
}
case "$mode" in
plan) python3 $K/tools/s3a-plan.py ${1:-2} ;;
servers)
  for c in w2:b2w2 w4:b2w4 w8:b2w8 w16:b2w16; do n=${c%%:*}; echo "cd $K && exec ./serve.sh $(port_of $n) ${c##*:} $REF      # condition $n"; done ;;
check)
  ok=1
  for n in 2 4 8 16; do a=$K/assets-b2w$n; v=$(python3 -c "import json;print(json.load(open('$a/manifest.json'))['engine']['env'].get('FLE_WORKERS'))" 2>/dev/null); echo "assets-b2w$n: FLE_WORKERS=$v $( [ "$v" = "$n" ] || { echo MISMATCH; ok=0; } )"; [ -e $a/fl-engine-ci-8b2eb5d7/engine.e1e2bf8a072d.wasm ] || { echo "  the B2 wasm is missing under $a"; ok=0; }; done
  [ -d $K/page-$REF/app/dist ] || { echo "page-$REF has no dist"; ok=0; }
  [ -f $K/proj/serina1-serum2.zip ] || { echo "the reference zip is missing"; ok=0; }
  [ -f $K/proj-solo12/serina1-serum2.zip ] && echo "proj-solo12: $(ls -la $K/proj-solo12/serina1-serum2.zip | awk '{print $5}') bytes" || { echo "proj-solo12/serina1-serum2.zip is missing: the solo12 runs cannot start (tools/mk-solo-zip.py ORIG.zip solo12.flp OUT.zip)"; ok=0; }
  for n in e13 w2 w4 w8 w16; do p=$(port_of $n); [ -n "$p" ] || continue; curl -sf -o /dev/null http://127.0.0.1:$p/ && echo "server $n on $p: up" || echo "server $n on $p: DOWN"; done
  [ $ok = 1 ] && echo "check: files OK" || echo "check: PROBLEMS above" ;;
next)
  n=${1:-1}
  for i in $(seq 1 $n); do
    line=$(awk -F'\t' 'NR>1 && $7=="todo"{print; exit}' $PLAN); [ -n "$line" ] || { echo "no todo line left"; exit 0; }
    IFS=$'\t' read -r id round cond scn port assets status <<<"$line"
    d=$K/out/$id; echo "next: $id (condition $cond, scenario $scn, port $port, assets $assets)"
    if [ -n "$DRY" ]; then echo "DRY: gate $port; node $K/tools/sdriveg2-lat.mjs --browser chrome --url http://127.0.0.1:$port/ --out $d --app $K/page-$REF/app --scenarios $K/tools/scn-s3a-$scn.json --viewport 2260x1116"; continue; fi
    [ "$scn" = solo12 ] && [ ! -f $K/proj-solo12/serina1-serum2.zip ] && { echo "proj-solo12 is missing"; exit 5; }
    gate $port || exit 4
    rm -rf $d; mkdir -p $d; { echo "run $id"; echo "start $(date -Iseconds)"; echo "$q"; free -m | sed -n 2,3p; vmstat 1 3 | tail -2; } > $d/gate.txt
    log "s3a $id start ($q)"
    node $K/tools/sdriveg2-lat.mjs --browser chrome --url http://127.0.0.1:$port/ --out $d --app $K/page-$REF/app --scenarios $K/tools/scn-s3a-$scn.json --viewport 2260x1116 > $d.log 2>&1
    rm -rf $d/profile-sdrive $d/netlog-sdrive.json
    if [ -f $d/report.json ] && [ -f $d/audio-p3.json ]; then sed -i "s/^\($id\t.*\)\ttodo\$/\1\tdone/" $PLAN; log "s3a $id end (done)"; else log "s3a $id end (NO report.json / audio-p3.json: see $d.log)"; fi
  done ;;
summary) python3 $K/tools/s3a-summary.py ;;
export)
  c=$1; p=$(port_of $c); [ -n "$p" ] || { echo "unknown condition $c"; exit 2; }; gate $p || exit 4
  d=$K/out/s3a-export-$c; rm -rf $d; mkdir -p $d; log "s3a export $c start ($q)"
  node $K/tools/sdriveg2-export.mjs --browser chrome --url http://127.0.0.1:$p/ --out $d --app $K/page-$REF/app --scenarios $K/tools/scn-export.json --viewport 2260x1116 > $d.log 2>&1
  rm -rf $d/profile-sdrive $d/netlog-sdrive.json; log "s3a export $c end"; ls -la $d | head; sha256sum $d/*.wav 2>/dev/null ;;
*) sed -n 2,13p $0 ;;
esac
