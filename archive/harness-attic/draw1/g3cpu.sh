#!/bin/bash
# draw1: user CPU seconds of a G3 script on the node link of a variant, alternating variants and the control script; usage: g3cpu.sh OUT ROUNDS VARIANT:SRCD...
W=/home/ubuntu/work/signature-studio-develop/lift/out-draw1
O=$1; N=$2; shift 2; mkdir -p $O
for r in $(seq 1 $N); do
  for spec in "$@"; do
    IFS=: read V S <<< "$spec"
    for sc in f8 f8-control; do
      SRCD=$S $W/tools/g3node.sh $V $W/g3/$sc.g3 cpu-$V-$sc-r$r > $O/$V-$sc-r$r.txt 2>&1
      echo "$V $sc r$r load $(cut -d' ' -f1 /proc/loadavg) $(grep 'User time' $O/$V-$sc-r$r.txt) $(grep 'Elapsed' $O/$V-$sc-r$r.txt | sed 's/.*: //')"
    done
  done
done
