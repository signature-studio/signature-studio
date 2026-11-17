#!/bin/bash
# pw1 (decision 0186): the measurement matrix, one Chrome at a time. usage: matrix.sh GROUP...   (groups: first k1 s6 s4 ref light heavy ott small cold)
# environment: PW1_OUT PW1_DATA PW1_RESULTS (see run-pw1.mjs); REPS (default 3), BLOCKS (default 650)
set -u
H=$(cd "$(dirname "$0")" && pwd)
REPS=${REPS:-3}; BLOCKS=${BLOCKS:-650}
run() { # tag serum ott hc [extra...]
  local tag=$1 serum=$2 ott=$3 hc=$4; shift 4
  echo "== $tag serum=[$serum] ott=$ott hc=$hc $(date +%T) load=$(cut -d' ' -f1-3 /proc/loadavg)"
  nice -n 15 node $H/run-pw1.mjs serum=$serum ott=$ott hc=$hc blocks=$BLOCKS reps=$REPS tag=$tag "$@" 2>&1 | grep -v '^\[' 
}
for g in "$@"; do case $g in
  first) run k1s3-hc16 3 0 16;;
  k1) for s in 0 4 5 3c; do run k1s$s-hc16 $s 0 16; done;;
  s6) for hc in 16 8 4; do run s6-hc$hc 0,1,3,4,5,3 0 $hc; done;;
  s4) for hc in 16 8 4; do run s4-hc$hc 0,1,3,4 0 $hc; done;;
  light) for hc in 4 8 16; do run light-hc$hc 0,1,3,4,0s,1s 12 $hc; done;;
  heavy) for hc in 4 8 16; do run heavy-hc$hc 0,1,3,4,5,3 12 $hc; done;;
  ott) for hc in 4 8 16; do run ott14-hc$hc "" 14 $hc; done;;
  k2) run k2s3-hc16 3,3 0 16;;
  cold) run cold-k1s3-hc16 3 0 16 warm=0 reps=1; run cold-s6-hc4 0,1,3,4,5,3 0 4 warm=0 reps=1;;
  esac; done
