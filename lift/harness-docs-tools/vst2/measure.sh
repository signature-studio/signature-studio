#!/bin/bash
# measure.sh CFGDIRNAME... : runs the scenarios on lift/out-vst2/native/<cfg>/vst-native, 3 times each for the default input, once for the health scenarios; the log
# lines (RESULT ...) go to lift/out-vst2/log/measure-<cfg>.txt with the load average before each run. CPU time is the thread CPU time (clock_gettime THREAD_CPUTIME).
cd "$(readlink -f "$(dirname "${BASH_SOURCE[0]}")/../../..")"
OUT=$PWD/lift/out-vst2; FILE=$PWD/lift/out-vst1/bin/OTT.vst3
export FLE_VFS_SEED=$OUT/edit/lift/harness/vst/vfs-seed-vst.tsv
for cfg in "$@"; do
    bin=$OUT/native/$cfg/vst-native; log=$OUT/log/measure-$cfg.txt; : > $log
    run() { echo "## $1 | load $(cut -d' ' -f1-3 /proc/loadavg)" >> $log; shift; nice -n 15 $bin $FILE "$@" 2>&1 | grep -E "^RESULT|fle:|SIGSEGV" >> $log; }
    for r in 1 2 3; do run "default run $r" --out /tmp/opencode/vst2/out-$cfg-$r.f32; done
    run "scale 0.5" --scale 0.5
    run "silence" --silence
    run "param-first 0.25" --param-first 0.25
    run "param-first 0.0" --param-first 0.0
    run "rate 48000" --rate 48000
done
