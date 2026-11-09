#!/bin/bash
# idle4.sh ROUNDS NAME:PORT[:PAGEREF] ...   (ci1, 2026-11-07, PM 11:4x; Chrome only on the PM's signal)
# ROUNDS rounds of single first idles (cold, a new profile each, Chrome 156, one at a time) of the given conditions in a rotated order (round i starts at condition i mod N), like verify-bundle.sh idle but for more than two bundles.
# Each run: wait-quiet (load <= 8, I/O pressure 0, swap traffic 0; a run that did not get a quiet window is marked REFERENCE), entry-check.mjs, beacon.py -> one row of out/idle4.tsv: browser, NAME, round, first idle ms, start load, end load, WebGL renderer, desk.mode.
# The ports are served by serve.sh PORT ASSETS PAGEREF: e13 5053 (E13 page + E13 bundle), rta 5101 (page-rt1 + RT-A), rta-w1 5108 (page-rt1 + RT-A + FLE_WORKERS=1: the cooperative scheduler of the RT build), cand1-nodiag 5109 (page d0447902 + cand1 without FLE_DIAG_*).
set -u
. /home/ubuntu/work/signature-studio-develop/lift/out-ci1/kit/env.sh
R=$1; shift; conds=("$@"); N=${#conds[@]}; f=$K/out/idle4.tsv; : > $f
log() { echo "$(date +%T) $* load $(cut -d' ' -f1 /proc/loadavg) io $(io)" | tee -a $K/out/steps.log; }
for i in $(seq 1 $R); do
  for j in $(seq 0 $((N - 1))); do
    c=${conds[$(( (i - 1 + j) % N ))]}; IFS=: read -r name port ref <<<"$c"; ref=${ref:-rt1}
    d=$K/out/idle4-$name-$i; q=$(node $K/tools/wait-quiet.mjs ${MAXLOAD:-8} ${MAXWAIT:-15}) || q="REFERENCE (start load over): $q"; log "idle4 $name $i ($q)"
    rm -rf $d; mkdir -p $d; echo "$q" > $d/wait.txt; s="$(cut -d' ' -f1 /proc/loadavg) $(io)"; skip=$(cat $K/out/perf-$port.jsonl 2>/dev/null | wc -l)
    node $K/tools/entry-check.mjs --browser chrome --url http://127.0.0.1:$port/ --app $K/page-$ref/app --beacon-wait-s 3 --out $d > $d.log 2>&1
    e="$(cut -d' ' -f1 /proc/loadavg) $(io)"
    python3 $K/tools/beacon.py $K/out/perf-$port.jsonl $skip tsv chrome "$name" "$i" "$s" "$e" $d >> $f
  done
done
log "idle4 x$R of $N conditions"
python3 - $f <<'PY'
import sys, statistics as st, collections
by = collections.defaultdict(list)
for l in open(sys.argv[1]):
    r = l.rstrip('\n').split('\t')
    if len(r) > 3 and r[3] not in ('None', ''): by[r[1]].append(int(r[3]))
for k, v in by.items(): print(k, v, 'median', st.median(v))
PY
