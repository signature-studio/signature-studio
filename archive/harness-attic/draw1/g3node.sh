#!/bin/bash
# draw1: a G3 script on the node link of a variant (lift/harness/b2d2/run-g3.sh). usage: g3node.sh VARIANT SCRIPT OUTNAME
R=/home/ubuntu/work/signature-studio-develop; W=$R/lift/out-draw1
V=$1; G3=$2; ON=$3
export PATH=/home/ubuntu/.local/bin:$PATH
DLLDIR=$(ls -d $W/$V/onat/v-*)/dll
OUT=$W/runs/g3/$ON; rm -rf $OUT; mkdir -p $OUT
REPO=$R DLLDIR=$DLLDIR WASMDIR=$W/$V/node/engine/obj/bin G3=$G3 OUT=$OUT nice -n 10 bash $W/${SRCD:-src}/lift/harness/b2d2/run-g3.sh
grep -E "Elapsed|Maximum resident|User time" $OUT/stderr.txt
