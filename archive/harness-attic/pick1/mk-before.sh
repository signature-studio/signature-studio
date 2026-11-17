#!/bin/bash
# pick1 (decision 0242 addendum): the "before" recorder binary = the same objects as the fixed one, except fle_win.o which is the HEAD one (src3-orig)
set -eu -o pipefail
P=/home/ubuntu/work/signature-studio-develop/lift/out-pick1
B=$P/ci2/engine-before; rm -rf $B; mkdir -p $B/nat
ln -s $P/ci2/engine/nat/obj $B/nat/obj
cp -r $P/ci2/engine/nat/rt $B/nat/rt
clang -std=gnu11 -O2 -ffp-contract=off -fno-fast-math -Wall -Wextra -fPIE -I$P/src3-orig/lift/runtime -c $P/src3-orig/lift/runtime/fle_win.c -o $B/nat/rt/fle_win.o 2>/dev/null
bash $P/tools/link-native.sh $B $P/work/two-native-api-before
