#!/bin/bash
# usage: check_overflow.sh WASM_DIR : the shadow stack region of a non-first thread is 1 MiB (FLE_THREAD_WASM_CSTACK_BYTES); a thread that uses more must end the run by name at its next switch
# (it has written below its region before that: the check detects, it cannot prevent; see docs/decisions/0124). Exit 0 when the stop message names thread 1 and the region.
HERE=$(cd "$(dirname "$0")" && pwd)
out=$(node --experimental-wasm-jspi --wasm-stack-switching-stack-size=16000 "$HERE/run_node_scen.cjs" "$1" deep 3000 2 2 512 2>&1)
echo "$out" | grep -E "^fle: scheduler: the shadow stack pointer of thread 1 .* is outside its region|canary at the low end of the shadow stack of thread 1" | head -1
echo "$out" | grep -qE "^fle: scheduler: (the shadow stack pointer of thread 1 .* is outside its region|the canary at the low end of the shadow stack of thread 1)"
