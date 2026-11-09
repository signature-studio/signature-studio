#!/bin/bash
# Decision 0177 (srm1): interleaved A/B measurement of several builds of the Serum 2 harness (native and wasm-under-node), so that every build sees about the same machine load.
# For rep in 1..REPS, for each case (state / chord), for each build in BUILDS: one run (40 blocks of 4096 frames, one held note C4), the RESULT lines are appended to OUT as
#   RUN <build> <case> rep=<r> load_before=<l> load_after=<l> | RESULT blocks=... | RESULT out_hash=...
# usage: BUILDS="native=/path/vst-native wasmO2=/dir/with/vst.js wasmOz=/dir" PLUGIN_NATIVE=<PE for native> PLUGIN_WASM=<rebased PE> STATE_DIR=.. SEED=.. CASES="0 1 3 4 5 chord" REPS=5 OUT=file bash ab_measure.sh
# The native build takes the PE it was built from (PLUGIN_NATIVE; the same rebased PE when it was built from the same units).
set -u -o pipefail
HERE=$(cd "$(dirname "$0")/.." && pwd)
BUILDS=${BUILDS:?}; PLUGIN_WASM=${PLUGIN_WASM:?}; PLUGIN_NATIVE=${PLUGIN_NATIVE:-$PLUGIN_WASM}; STATE_DIR=${STATE_DIR:?}; SEED=${SEED:?}; OUT=${OUT:?}
CASES=${CASES:-"0 1 3 4 5 chord"}; REPS=${REPS:-3}; BLOCKS=${BLOCKS:-40}; NODEFLAGS=${NODEFLAGS:-}
export FLE_VFS_SEED=$SEED
: >> "$OUT"
for rep in $(seq 1 "$REPS"); do
  for c in $CASES; do
    for b in $BUILDS; do
      name=${b%%=*}; path=${b#*=}
      case $c in
        chord) args=(--blocks "$BLOCKS" --notes 60,64,67,72 --note --state 3 --state-dir "$STATE_DIR" --presets-dir 'C:\SerumData\Presets');;
        idle) args=(--blocks "$BLOCKS" --state 3 --state-dir "$STATE_DIR" --presets-dir 'C:\SerumData\Presets');;
        5) args=(--blocks "$BLOCKS" --note --state 5 --state-proc-only --state-dir "$STATE_DIR" --presets-dir 'C:\SerumData\Presets');;
        *) args=(--blocks "$BLOCKS" --note --state "$c" --state-dir "$STATE_DIR" --presets-dir 'C:\SerumData\Presets');;
      esac
      l0=$(cut -d' ' -f1 /proc/loadavg)
      case $name in
        native*) o=$(nice -n 15 "$path" "$PLUGIN_NATIVE" "${args[@]}" 2>&1);;
        *) o=$(nice -n 15 node $NODEFLAGS "$HERE/run_vst3a_node.cjs" "$path" "$PLUGIN_WASM" "${args[@]}" 2>&1);;
      esac
      l1=$(cut -d' ' -f1 /proc/loadavg)
      echo "RUN $name case=$c rep=$rep load_before=$l0 load_after=$l1" >> "$OUT"
      echo "$o" | grep -a "^RESULT blocks\|^RESULT out_hash\|^RESULT init_cpu_ms\|wasm run failed\|Aborted\|misaligned" | cut -c1-400 >> "$OUT"
    done
  done
done
