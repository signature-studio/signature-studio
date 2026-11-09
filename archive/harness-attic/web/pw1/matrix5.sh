#!/bin/bash
# pw1 (decision 0186): paired runs, ring and lock step taken alternately (the same minutes of the same machine). usage: matrix5.sh HC REPS_FROM REPS_TO [heavy|light]
set -u
H=$(cd "$(dirname "$0")" && pwd)
hc=$1; from=$2; to=$3; cfg=${4:-heavy}; BLOCKS=${BLOCKS:-650}
if [ $cfg = heavy ]; then S=0,1,3,4,5,3; else S=0,1,3,4,0s,1s; fi
for r in $(seq $from $to); do
  echo "== ring$cfg-hc$hc rep $r $(date +%T) load=$(cut -d' ' -f1-3 /proc/loadavg)"
  nice -n 15 node $H/run-ring.mjs serum=$S ott=12 hc=$hc blocks=$BLOCKS reps=1 tag=ring$cfg-hc$hc rep0=$r 2>&1 | grep -v '^\['
  echo "== $cfg-hc$hc rep $r $(date +%T) load=$(cut -d' ' -f1-3 /proc/loadavg)"
  nice -n 15 node $H/run-pw1.mjs serum=$S ott=12 hc=$hc blocks=$BLOCKS reps=1 tag=$cfg-hc$hc rep0=$r warmms=25000 settle=3000 2>&1 | grep -v '^\['
done
