#!/bin/bash
# pw1 (decision 0186): the decoupled / phase-staggered output-ring variant (run-ring.mjs). usage: matrix4.sh GROUP...   groups: ring (staggered, lead 1), ringsync (decoupled, all phases 0), ring0 (lead 0 reference)
set -u
H=$(cd "$(dirname "$0")" && pwd)
BLOCKS=${BLOCKS:-650}
run() { # tag serum ott hc [extra...]
  local tag=$1 serum=$2 ott=$3 hc=$4; shift 4
  echo "== $tag serum=[$serum] ott=$ott hc=$hc $* $(date +%T) load=$(cut -d' ' -f1-3 /proc/loadavg)"
  nice -n 15 node $H/run-ring.mjs serum=$serum ott=$ott hc=$hc blocks=$BLOCKS reps=1 tag=$tag "$@" 2>&1 | grep -v '^\['
}
for g in "$@"; do case $g in
  ring) for r in 0 1 2; do for hc in 8 16 4; do
          [ $hc = 4 ] && [ $r = 2 ] && continue
          run ringheavy-hc$hc 0,1,3,4,5,3 12 $hc rep0=$r; run ringlight-hc$hc 0,1,3,4,0s,1s 12 $hc rep0=$r; done; done;;
  ringsync) for hc in 8 16; do run ringsyncheavy-hc$hc 0,1,3,4,5,3 12 $hc stagger=0 rep0=0; done;;
  esac; done
