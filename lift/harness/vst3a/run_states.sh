#!/bin/bash
# Decision 0177 (srm1): the 5 states of the reference project (0,1,3,4,5; one held note C4, 40 blocks of 4096 frames) + the 4-note chord on state 3, with a native or a wasm (node) build
# of the Serum 2 harness. Prints one block per run: the RESULT lines, the load before/after, and the stage lines (setState, DllMain, ...).
# usage: KIND=native|wasm BIN=<vst-native | dir with vst.js> PLUGIN=<Serum2.vst3 or the rebased one> STATE_DIR=<dir with sN_c2.bin, sN_c3.bin> SEED=<vfs seed tsv> \
#        [STATES="0 1 3 4 5"] [CHORD=1] [BLOCKS=40] [REPS=1] [OUTDIR=<dir>] bash run_states.sh
set -u -o pipefail
HERE=$(cd "$(dirname "$0")" && pwd)
KIND=${KIND:?}; BIN=${BIN:?}; PLUGIN=${PLUGIN:?}; STATE_DIR=${STATE_DIR:?}; SEED=${SEED:?}
STATES=${STATES:-"0 1 3 4 5"}; BLOCKS=${BLOCKS:-40}; REPS=${REPS:-1}; CHORD=${CHORD:-1}; OUTDIR=${OUTDIR:-}
export FLE_VFS_SEED=$SEED
run() {   # $1 label, rest = harness args
    local label=$1; shift
    local l0 l1 out
    l0=$(cut -d' ' -f1 /proc/loadavg)
    if [ "$KIND" = native ]; then out=$(nice -n 15 "$BIN" "$PLUGIN" "$@" 2>&1); else out=$(nice -n 15 node "$HERE/run_vst3a_node.cjs" "$BIN" "$PLUGIN" "$@" 2>&1); fi
    l1=$(cut -d' ' -f1 /proc/loadavg)
    echo "== $label  load_before=$l0 load_after=$l1"
    echo "$out" | grep -a "^RESULT blocks\|^RESULT out_hash\|setState\|wasm run failed\|abort\|RuntimeError\|Error:" | cut -c1-420
    [ -n "$OUTDIR" ] && echo "$out" > "$OUTDIR/$label.full.txt"
}
[ -n "$OUTDIR" ] && mkdir -p "$OUTDIR"
for rep in $(seq 1 "$REPS"); do
    for s in $STATES; do
        extra=""; [ "$s" = 5 ] && extra="--state-proc-only"
        run "state$s-rep$rep" --blocks "$BLOCKS" --note --state "$s" --state-dir "$STATE_DIR" $extra --presets-dir 'C:\SerumData\Presets'
    done
    [ "$CHORD" = 1 ] && run "state3-chord4-rep$rep" --blocks "$BLOCKS" --notes 60,64,67,72 --note --state 3 --state-dir "$STATE_DIR" --presets-dir 'C:\SerumData\Presets'
done
