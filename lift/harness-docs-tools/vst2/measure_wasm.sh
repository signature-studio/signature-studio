#!/bin/bash
# measure_wasm.sh CFG... (e.g. ottw-O2-b1 ottw-Oz-b1): node runs of the wasm builds; log lift/out-vst2/log/measure-wasm-<cfg>.txt with the load before each run.
cd "$(readlink -f "$(dirname "${BASH_SOURCE[0]}")/../../..")"
OUT=$PWD/lift/out-vst2
export FLE_VFS_SEED=$OUT/edit/lift/harness/vst/vfs-seed-vst.tsv
for cfg in "$@"; do
    log=$OUT/log/measure-wasm-$cfg.txt; : > $log
    run() { echo "## $1 | load $(cut -d' ' -f1-3 /proc/loadavg)" >> $log; shift; nice -n 15 node $OUT/edit/lift/harness/vst/run_vst_node.cjs $OUT/wasm/$cfg $OUT/rebased/OTT.2000000.vst3 "$@" 2>&1 | grep -E "^RESULT|fle:|failed|vst: block|DllMain\(" >> $log; }
    for r in 1 2 3; do VST_BLOCKS=1 run "default run $r" --out /tmp/opencode/vst2/w-$cfg-$r.f32; done
    run "scale 0.5" --scale 0.5
    run "silence" --silence
    run "param-first 0.25" --param-first 0.25
done
