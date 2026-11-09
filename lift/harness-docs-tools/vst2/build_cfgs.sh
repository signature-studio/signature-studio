#!/bin/bash
# builds the measurement configurations of TAG=ott18 one after the other (JOBS=3), logs to lift/out-vst2/log/build-<cfg>.txt
cd /home/ubuntu/work/signature-studio-develop
export OUT=$PWD/lift/out-vst2 FILE=$PWD/lift/out-vst1/bin/OTT.vst3
for cfg in "$@"; do
    IFS=: read opt bi cnt <<< "$cfg"
    ( ulimit -v 14000000; /usr/bin/time -v env TAG=ott18 OPT=$opt BUILTIN=$bi COUNT=$cnt JOBS=3 bash $OUT/edit/lift/harness/vst/build_vst_native.sh ) > $OUT/log/build-$opt-b$bi-c$cnt.txt 2>&1
done
echo done > $OUT/log/build-cfgs.done
