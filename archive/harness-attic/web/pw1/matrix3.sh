#!/bin/bash
# pw1 (decision 0186): the last groups of the measurement matrix (physical-core placements, 1 and 2 Serum, cold start, OTT alone). usage: matrix3.sh GROUP...
# groups: cold smt k1 k2 is6 iott
# environment: PW1_OUT PW1_DATA PW1_RESULTS; REPS (default 3), BLOCKS (default 650); WARMARGS (default "warmms=25000 settle=3000")
set -u
H=$(cd "$(dirname "$0")" && pwd)
REPS=${REPS:-3}; BLOCKS=${BLOCKS:-650}
run() { # tag serum ott hc [extra...]
  local tag=$1 serum=$2 ott=$3 hc=$4; shift 4
  echo "== $tag serum=[$serum] ott=$ott hc=$hc $* $(date +%T) load=$(cut -d' ' -f1-3 /proc/loadavg)"
  nice -n 15 node $H/run-pw1.mjs serum=$serum ott=$ott hc=$hc blocks=$BLOCKS reps=$REPS tag=$tag ${WARMARGS-warmms=25000 settle=3000} "$@" 2>&1 | grep -v '^\['
}
for g in "$@"; do case $g in
  cold) REPS=1 run cold-k1s3-hc16 3 0 16 warm=0; REPS=2 run cold-s6-hc16 0,1,3,4,5,3 0 16 warm=0;;
  # one thread per physical core: cpus 0,2,4,6 (4 physical cores) and 0,2,...,14 (8 physical cores); the SMT siblings (1,3,5,..) stay free unless another process takes them
  smt) for r in 0 1 2; do REPS=1 run heavy-hc4t 0,1,3,4,5,3 12 4 cpus=0,2,4,6 rep0=$r; REPS=1 run light-hc4t 0,1,3,4,0s,1s 12 4 cpus=0,2,4,6 rep0=$r; REPS=1 run heavy-hc8t 0,1,3,4,5,3 12 8 cpus=0,2,4,6,8,10,12,14 rep0=$r; done;;
  k1) for s in 5 3c; do run k1s$s-hc16 $s 0 16; done; for s in 0 4; do REPS=2 run k1s$s-hc16 $s 0 16; done;;
  k1b) for s in 0 4; do REPS=2 run k1s$s-hc16 $s 0 16; done;;
  k2) run k2s3-hc16 3,3 0 16;;
  # more repetitions of the two reference-like compositions at HC=8 and HC=16, interleaved (taken when the machine was quiet)
  q) for r in 3 4; do for hc in 8 16; do REPS=1 run light-hc$hc 0,1,3,4,0s,1s 12 $hc rep0=$r; REPS=1 run heavy-hc$hc 0,1,3,4,5,3 12 $hc rep0=$r; done; done;;
  is6) for r in 0 1 2; do for hc in 4 8 16; do REPS=1 run s6-hc$hc 0,1,3,4,5,3 0 $hc rep0=$r; done; done;;
  iott) for r in 0 1; do for hc in 4 16; do REPS=1 run ott14-hc$hc "" 14 $hc rep0=$r; done; done;;
  esac; done
