#!/bin/bash
# srm1 (decision 0177): the Chrome Worker series: one Serum 2 instance in one Worker, states 0,1,3,4,5 and the 4-note chord of state 3, MOD (a wasm file in $SRM1_OUT/data: O2 | Oz | O2simd ...), REPS runs each.
# usage: SRM1_OUT=<lift/out-srm1> MOD=O2 REPS=2 TAG=ch1 bash series.sh
set -u
HERE=$(cd "$(dirname "$0")" && pwd)
MOD=${MOD:-O2}; REPS=${REPS:-2}; TAG=${TAG:-ch}; BLOCKS=${BLOCKS:-40}
cd "$HERE"
for s in 0 1 3 4 5; do nice -n 15 node run-serum.mjs mod=$MOD n=1 state=$s blocks=$BLOCKS reps=$REPS tag=$TAG 2>&1 | grep '^{' ; done
nice -n 15 node run-serum.mjs mod=$MOD n=1 state=3 blocks=$BLOCKS reps=$REPS tag=$TAG-chord notes=60,64,67,72 2>&1 | grep '^{'
