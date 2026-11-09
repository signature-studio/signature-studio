#!/bin/bash
# wasm-final0129 (decision 0134): an OUT for build_nmod_wasm.sh whose translation units are the SAME generated C (oonat/gen/flengine.c and oonat/v-TAG/gen/mK.c, the output of tools/run-native.sh = decision 0129's final
# configuration) cut by a byte budget (harness/two/split_bytes.py, decision 0130) instead of 3,700 / 800 functions per part. common.c (the merged dispatch table) is the file of the native build. usage: resplit.sh BUDGET_BYTES   (OUT = out-wasm0129/wsrc)
. /home/ubuntu/work/signature-studio-develop/lift/out-wasm0129/src/docs/investigations/wasm-final0129/tools/env.sh
BUDGET=${1:-10000000}
NO=$W/oonat
TAGDIR=$(ls -d $NO/v-* | head -1)
TAG=$(basename $TAGDIR | sed 's/^v-//')
OUT=$W/wsrc
[ ! -e $OUT/src ] || { echo "exists: $OUT/src"; exit 1; }
mkdir -p $OUT/src $OUT/v-$TAG/src $OUT/tmp
python3 $H/two/split_bytes.py $NO/gen/flengine.c $OUT/src/fle $BUDGET
for k in 1 2 3 4 5; do
  python3 $H/two/split_bytes.py $TAGDIR/gen/m$k.c $OUT/v-$TAG/src/m$k $BUDGET
done
cp $TAGDIR/src/common.c $OUT/v-$TAG/src/common.c
cp $TAGDIR/modules.txt $OUT/v-$TAG/modules.txt
echo "TAG=$TAG"
ls $OUT/src | wc -l; ls $OUT/v-$TAG/src | wc -l
sha256sum $NO/gen/flengine.c $TAGDIR/gen/m*.c $TAGDIR/src/common.c > $OUT/inputs-sha256.txt
