#!/bin/bash
# draw1: alternating sessions of tools/f8run.mjs (mode cpu), ROUNDS rounds over the given bundle directory names.
# usage: ab.sh OUTDIR ROUNDS REPS BUNDLE...
R=/home/ubuntu/work/signature-studio-develop/lift/out-draw1
export LAT1_ROOT=/home/ubuntu/work/signature-studio-develop LAT1_APP=$R/src/app LAT1_DIST=$R/dist-head LAT1_ASSETS=$R/assets
O=$1; N=$2; REPS=$3; shift 3
for r in $(seq 1 $N); do
  for b in "$@"; do
    d=$O/${b#fl-engine-draw1-}-r$r; mkdir -p $d
    echo "== $b round $r $(date -u +%T) load $(cut -d' ' -f1 /proc/loadavg)"
    nice -n 15 node $R/tools/f8run.mjs --bundle $b --out $d --mode cpu --reps $REPS --max-load 100 --open-ms ${OPEN:-6000} --mid-ms ${MID:-4000} --close-ms ${CLOSE:-3000} 2>&1 | grep -E "rep |idle:|ERROR" | cut -c1-260
  done
done
echo "== done $(date -u +%T)"
