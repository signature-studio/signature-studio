#!/bin/bash
# run-rta.sh MODE ...   (ci1, 2026-11-07, PM 09:3x: RT-A = the real-thread test bundle fl-engine-test-rt1-91bf78f6; Chrome only on the PM's signal, one at a time)
#   smoke COND            first idle, n (navigator.hardwareConcurrency), the windows, the per-Worker CPU of 5 idle seconds (the number of Workers), the engine log (Filter LoadLibraryW, unsupported). untimed
#   export COND           the Export of the original FLP (scn-export-g9b.json); the sha256 of the WAV against cand1's 94c93666...4090. untimed; MemAvailable >= 12 GB
#   song COND             the 5 minute playback of the reference project (scn-s3a-song.json), timed (the gate of the S3a runs: load <= 8, I/O 0, swap 0, other Chrome 0, MemAvailable >= 8 GB, Wine 0)
#   solo COND             the same with solo12
#   lat COND              the input latency suite (scn-lat-ff.json: File clicks idle, Mixer drag, Space x32, File clicks during playback), timed
#   summary               tools/s3a-summary.py
# COND = e13 (5053, the control: cooperative production) | rta (5101) | rta-w2 (5102) | rta-p2 (5103) | rta-p2w2 (5104); the RT-A conditions use the page of mt3's tree-rt1 (page-rt1: the stack's page changes, REF=rt1); the control e13 must be run with REF=d0447902 (the production page + the E13 bundle)
. /home/ubuntu/work/signature-studio-develop/lift/out-ci1/kit/env.sh
REF=${REF:-rt1}; mode=${1:-help}; cond=${2:-}
case "$cond" in e13) port=5053;; rta) port=5101;; rta-w2) port=5102;; rta-p2) port=5103;; rta-p2w2) port=5104;; rta-n2) port=5106;; rta-n4) port=5107;; rtd) port=5115;; rtd-ins1lp) port=5116;; rta-seed2) port=5105;; rtd3) port=5119;; rtf) port=5122;; rth) port=5143;; rtb) port=5111;; *) port=;; esac
log() { echo "$(date +%T) $* load $(cut -d' ' -f1 /proc/loadavg) io $(io)" | tee -a $K/out/steps.log; }
gate() {   # $1 = port, $2 = min MemAvailable MB, $3 = max load. wait-quiet first (load, I/O pressure and swap traffic over 2 s), then the record of free/vmstat and the swap check right after it; up to 3 tries
  local others age si so avail w try
  read -r e others mine total < <(tail -n 1 $K/out/chrome-count.tsv); age=$(( $(date +%s%3N) - e ))
  [ "$age" -lt 15000 ] || { echo "GATE: the chrome-count observer is stale"; return 1; }
  [ "$others" = 0 ] || { echo "GATE: $others other Chrome: not now"; return 1; }
  w=$(ps -eo args | grep -E 'FL64|wineserver' | grep -v grep | wc -l); [ "$w" = 0 ] || { echo "GATE: Wine is running ($w)"; return 1; }
  curl -sf -o /dev/null http://127.0.0.1:$1/ || { echo "GATE: the server on $1 is down"; return 1; }
  for try in 1 2 3; do
    q=$(node $K/tools/wait-quiet.mjs ${3:-8} ${MAXWAIT:-15}) || { echo "GATE: not quiet: $q"; return 1; }
    { echo "$(date +%T) $mode $cond pre-run note (try $try)"; free -m | sed -n 2,3p; vmstat 1 3 | tail -2; } | tee -a $K/out/steps.log
    read -r si so < <(vmstat 1 3 | tail -1 | awk '{print $7, $8}'); avail=$(awk '/^MemAvailable:/{print int($2/1024)}' /proc/meminfo)
    [ "$avail" -ge ${2:-8192} ] || { echo "GATE: MemAvailable $avail MB < ${2:-8192}"; return 1; }
    [ "$si" = 0 ] && [ "$so" = 0 ] && return 0
    echo "GATE: swap in/out $si/$so (try $try)"
  done
  return 1
}
run_lat() { # scenario file, outdir
  node $K/tools/sdriveg2-lat.mjs --browser chrome --url http://127.0.0.1:$port/ --out $2 --app $K/page-$REF/app --scenarios $K/tools/$1 --viewport 2260x1116 > $2.log 2>&1
  rm -rf $2/profile-sdrive $2/netlog-sdrive.json
}
case "$mode" in
smoke) [ -n "$port" ] || exit 2; gate $port 12000 12 || exit 4; d=$K/out/rta-smoke-$cond; rm -rf $d; mkdir -p $d; log "rta smoke $cond start ($q)"
  node $K/tools/sdriveg2-export.mjs --browser chrome --url http://127.0.0.1:$port/ --out $d --app $K/page-$REF/app --scenarios $K/tools/scn-rta-smoke.json --viewport 2260x1116 > $d.log 2>&1; log "rta smoke $cond end"
  grep -c 'fle: unsupported' $d/engine-log.log; grep 'g9-diag: LoadLibraryW.*Filter\|GetProcAddress(0x7' $d/engine-log.log | cut -c1-200
  python3 - $d <<'PY'
import json, sys
r = json.load(open(sys.argv[1] + '/report.json'))[0]
print(json.dumps(r.get('evals'))[:600]); print('ok', r.get('ok'), 'stopped', r.get('stopped'), 'firstIdleMs', r.get('firstIdleMs'))
for p in (r.get('profs') or []): print('profs', json.dumps(p)[:500])
PY
  ;;
open) [ -n "$port" ] || exit 2; gate $port 12000 12 || exit 4; d=$K/out/rta-open-$cond; rm -rf $d; mkdir -p $d; log "rta open $cond start ($q)"
  node $K/tools/sdriveg2-export.mjs --browser chrome --url http://127.0.0.1:$port/ --out $d --app $K/page-$REF/app --scenarios $K/tools/scn-rta-open.json --viewport 2260x1116 > $d.log 2>&1; log "rta open $cond end"
  python3 - $d <<'PY'
import json, sys
r = json.load(open(sys.argv[1] + '/report.json'))[0]
print('ok', r.get('ok'), 'stopped', r.get('stopped'), 'opErrors', r.get('opErrors')); print(json.dumps(r.get('evals'))[:300])
for dsc in (r.get('describes') or []): print(dsc['name'], [l[:110] for l in dsc['lines']])
PY
  grep -c -i 'MissingPlugins\|plugin(s) could not' $d/engine-log.log 2>/dev/null ;;
export) [ -n "$port" ] || exit 2; ALLOW_OTHER_CHROME= SCN=scn-export-g9b.json bash $K/tools/run-cand1-export.sh $cond $port $K/out/rta-export-$cond ;;
song) [ -n "$port" ] || exit 2; gate $port 8192 8 || exit 4; d=$K/out/s3a-r${RND:-9}-${cond//-/}-song; rm -rf $d; mkdir -p $d; { echo "run $cond"; echo "start $(date -Iseconds)"; echo "$q"; } > $d/gate.txt; log "rta song $cond start ($q)"; run_lat scn-s3a-song.json $d; log "rta song $cond end" ;;
solo) [ -n "$port" ] || exit 2; gate $port 8192 8 || exit 4; d=$K/out/s3a-r${RND:-9}-${cond//-/}-solo12; rm -rf $d; mkdir -p $d; { echo "run $cond"; echo "start $(date -Iseconds)"; echo "$q"; } > $d/gate.txt; log "rta solo $cond start ($q)"; run_lat scn-s3a-solo.json $d; log "rta solo $cond end" ;;
lat) [ -n "$port" ] || exit 2; gate $port 8192 8 || exit 4; d=$K/out/lat-chrome-${LATTAG:-new}-${RND:-1}; rm -rf $d; mkdir -p $d; { echo "run $cond"; echo "start $(date -Iseconds)"; echo "$q"; } > $d/gate.txt; log "rta lat $cond start ($q)"; run_lat scn-lat-ff.json $d; log "rta lat $cond end" ;;
summary) python3 $K/tools/s3a-summary.py $K/out --dirs $(ls -d $K/out/s3a-r*-* 2>/dev/null | tr '\n' ' ') ;;
*) sed -n 2,11p $0 ;;
esac
