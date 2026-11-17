#!/bin/bash
# Run the two-module native binary under gdb with the TEMPORARY chain values of patches_chain.py (a measurement, not the runtime's behaviour;
# docs/investigations/stage23-two-module-real-dll.md). Writes a new log set per run under $OUT/run (nothing overwritten).
#
# usage: CHAIN_MAX=<0..6> bash lift/harness/two/chain.sh NAME
# env:   OUT=lift/out-stage2   TAG=tbl2 (the gen_two.sh TAG whose native binary is run)   CHAIN_MAX (patch stages 1..n active, default 0)
#        CHAIN_ABSENT=avrt.dll,...  (names that the stage-6 patch answers NULL for)   GDBI_GPA=1 (log GetProcAddress results)
#        GDBI_SCRIPT=<gdb helper, default gdb_imports.py>
# Outputs: $OUT/run/NAME-imports.txt (every fle_imp_* call after DllMain entry), NAME-gdb.out (the program's own stop report: a4 lines etc.),
#          NAME-gdb.err (the "fle: ..." line), NAME-gdb.log (gdb's own output, including the patch notes "(stage N)").
set -u
HERE=$(cd "$(dirname "$0")" && pwd)
LIFT=$(cd "$HERE/../.." && pwd)
OUT=${OUT:-$LIFT/out-stage2}
TAG=${TAG:-tbl2}
NAME=${1:?name of the run}
CHAIN_MAX=${CHAIN_MAX:-0}
mkdir -p "$OUT/run" "$OUT/tmp"
export TMPDIR="$OUT/tmp" CHAIN_MAX CHAIN_ABSENT=${CHAIN_ABSENT:-avrt.dll}
export GDBI_PATCHES="$HERE/patches_chain.py" GDBI_BIN="$OUT/two/v-$TAG/native/two-native" GDBI_REAL="$OUT/two/dll/dsp_ippv2_x64.2000000.dll" \
    GDBI_FLE="$LIFT/../binaries/win64/FLEngine_x64.dll" GDBI_OUT="$OUT/run/$NAME-imports.txt" GDBI_STDOUT="$OUT/run/$NAME-gdb.out" \
    GDBI_STDERR="$OUT/run/$NAME-gdb.err"
gdb -q -batch -x "$HERE/${GDBI_SCRIPT:-gdb_imports.py}" > "$OUT/run/$NAME-gdb.log" 2>&1
echo "exit=$?  stop: $(cat "$OUT/run/$NAME-gdb.err")"
grep -E "^(functions|function entries|a4: init_count|a4: last_unit index)" "$OUT/run/$NAME-gdb.out"
