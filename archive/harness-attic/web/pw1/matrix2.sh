#!/bin/bash
# pw1 (decision 0186): further groups of the measurement matrix (matrix.sh keeps the first ones). usage: matrix2.sh GROUP...
# groups: light heavy ott k1 k2 cold smt extra
# environment: PW1_OUT PW1_DATA PW1_RESULTS; REPS (default 3), BLOCKS (default 650)
set -u
H=$(cd "$(dirname "$0")" && pwd)
REPS=${REPS:-3}; BLOCKS=${BLOCKS:-650}
run() { # tag serum ott hc [extra...]
  local tag=$1 serum=$2 ott=$3 hc=$4; shift 4
  echo "== $tag serum=[$serum] ott=$ott hc=$hc $* $(date +%T) load=$(cut -d' ' -f1-3 /proc/loadavg)"
  nice -n 15 node $H/run-pw1.mjs serum=$serum ott=$ott hc=$hc blocks=$BLOCKS reps=$REPS tag=$tag ${WARMARGS-warmms=25000 settle=3000} "$@" 2>&1 | grep -v '^\['
}
for g in "$@"; do case $g in
  light) for hc in 4 8 16; do run light-hc$hc 0,1,3,4,0s,1s 12 $hc; done;;
  heavy) for hc in 4 8 16; do run heavy-hc$hc 0,1,3,4,5,3 12 $hc; done;;
  ott) for hc in 4 8 16; do run ott14-hc$hc "" 14 $hc; done;;
  # interleaved: repetition by repetition over the HC settings (the foreign load of this shared machine changes with time; consecutive repetitions of one condition would take one load each)
  ilight) for r in 0 1 2; do for hc in 4 8 16; do REPS=1 run light-hc$hc 0,1,3,4,0s,1s 12 $hc rep0=$r; done; done;;
  iheavy) for r in 0 1 2; do for hc in 4 8 16; do REPS=1 run heavy-hc$hc 0,1,3,4,5,3 12 $hc rep0=$r; done; done;;
  iott) for r in 0 1 2; do for hc in 4 8 16; do REPS=1 run ott14-hc$hc "" 14 $hc rep0=$r; done; done;;
  is6) for r in 3 4 5; do for hc in 4 8 16; do REPS=1 run s6-hc$hc 0,1,3,4,5,3 0 $hc rep0=$r; done; done;;
  is4) for r in 0 1 2; do for hc in 4 8 16; do REPS=1 run s4-hc$hc 0,1,3,4 0 $hc rep0=$r; done; done;;
  k1) for s in 5 3c 0 4; do run k1s$s-hc16 $s 0 16; done;;
  k2) run k2s3-hc16 3,3 0 16;;
  cold) run cold-k1s3-hc16 3 0 16 warm=0; run cold-s6-hc16 0,1,3,4,5,3 0 16 warm=0;;
  smt) run s6-hc4t 0,1,3,4,5,3 0 4 cpus=0,2,4,6; run heavy-hc4t 0,1,3,4,5,3 12 4 cpus=0,2,4,6; run light-hc4t 0,1,3,4,0s,1s 12 4 cpus=0,2,4,6;;
  extra) run "$EXTRA_TAG" "$EXTRA_SERUM" "${EXTRA_OTT:-0}" "${EXTRA_HC:-16}" ${EXTRA_ARGS:-};;
  esac; done
