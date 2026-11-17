#!/bin/bash
# draw1: the 16 snapshots (cmp-short.g3, 82 lines) of a variant's node link against the F1x layer-3 baseline of lift/harness/b2d2/expected. usage: g3cmp16.sh VARIANT SRCD
V=$1; export SRCD=${2:-src}
W=/home/ubuntu/work/signature-studio-develop/lift/out-draw1
$W/tools/g3node.sh $V $W/src/docs/investigations/b1/tools/g3/cmp-short.g3 16-$V 2>&1 | tail -3
python3 $W/$SRCD/lift/harness/b2d2/g3cmp.py $W/$SRCD/lift/harness/b2d2/expected/g3-lines-F1x-layer3-baseline.txt $W/runs/g3/16-$V/g3-lines.txt 16-$V | tee $W/runs/g3/16-$V/cmp.txt | tail -5
