#!/bin/bash
# A4 survey: how far DllMain -> InitUnits got (docs/investigations/a4-survey-tool.md, decision 0060).
# One screen: the static unit table of the DLL, the stop point of the native harness, init_count / 942, the last started unit,
# the function counts; with WASM=1 the same run under node, and a check that both print the same a4 lines.
#
# usage: bash lift/harness/a4-survey.sh
# env:   OUT=lift/out-surv   (where the binaries are: $OUT/native/flengine-native, $OUT/bin-base/ for WASM=1;
#                              build them with: OUT=$OUT bash lift/harness/wasm/build_native.sh   [and build.sh for wasm])
#        DLL=binaries/win64/FLEngine_x64.dll    WASM=0|1    NATIVE_BIN=<path>
# Logs go to $OUT/a4/<UTC timestamp>-{units.tsv,native.txt,native.err,node.txt,node.err} (new files each run, nothing overwritten).
# Exit: 0 when the survey ran (the stop point is the result, not an error), 2 when something needed is missing,
#       3 when WASM=1 and the a4 lines of native and wasm differ.
set -u
HERE=$(cd "$(dirname "$0")" && pwd)
LIFT=$(cd "$HERE/.." && pwd)
OUT=${OUT:-$LIFT/out-surv}
DLL=${DLL:-$LIFT/../binaries/win64/FLEngine_x64.dll}
WASM=${WASM:-0}
NATIVE_BIN=${NATIVE_BIN:-$OUT/native/flengine-native}
mkdir -p "$OUT/tmp" "$OUT/a4"
export TMPDIR="$OUT/tmp"
TS=$(date -u +%Y%m%dT%H%M%SZ)
P="$OUT/a4/$TS"

[ -r "$DLL" ] || { echo "a4-survey: DLL not found: $DLL" >&2; exit 2; }
[ -x "$NATIVE_BIN" ] || { echo "a4-survey: native harness not built: $NATIVE_BIN" >&2; echo "  build: OUT=$OUT bash $HERE/wasm/build_native.sh (needs $OUT/out/wasm-pdata-part-*.c from build.sh gen split)" >&2; exit 2; }
FEPE=${FEPE:-$OUT/target/release/fe-pe}
if [ ! -x "$FEPE" ]; then
    (cd "$LIFT" && CARGO_TARGET_DIR="$OUT/target" cargo build --release --offline -p fe-pe >/dev/null 2>&1) || { echo "a4-survey: cargo build of fe-pe failed" >&2; exit 2; }
fi

ST=$("$FEPE" pe-init-table "$DLL" "$P-units.tsv") || { echo "a4-survey: fe-pe pe-init-table failed" >&2; exit 2; }
sv() { echo "$ST" | awk -v k="$1" '$1 == k { print $2 }'; }

"$NATIVE_BIN" "$DLL" > "$P-native.txt" 2> "$P-native.err"
RC=$?
a4lines() { grep '^a4:' "$1"; }
msg=$(sed -n 's/^runtime message: //p' "$P-native.txt" | head -1)
[ -n "$msg" ] || msg=$(sed -n 's/^== stop: //p' "$P-native.txt" | head -1)
top=$(awk '/^call depth at stop/ { f = 1; next } f && /^  #0 / { print $2, $3; exit }' "$P-native.txt")
fns=$(sed -n 's/^functions executed (distinct): //p' "$P-native.txt")
ent=$(sed -n 's/^function entries (calls): //p' "$P-native.txt")
ic=$(sed -n 's/^a4: init_count=\([0-9]*\) of \([0-9?]*\),.*/\1 \2/p' "$P-native.txt")
lu=$(sed -n 's/^a4: last_unit index=\([0-9]*\) run_rank=\([0-9]*\) of \([0-9]*\) proc=\(0x[0-9a-f]*\) state=\(.*\)/index=\1 run_rank=\2\/\3 proc=\4 \5/p' "$P-native.txt")
st=$(sed -n 's/^a4: runnable_started=\([0-9]*\) completed=\([0-9]*\) in_progress=\([0-9]*\) of \([0-9]*\) runnable/started=\1 completed=\2 in_progress=\3 of \4 runnable/p' "$P-native.txt")
ec=$(sed -n 's/^a4: entry_check //p' "$P-native.txt")
done_=$(sed -n 's/^a4: all_units_done=\([a-z]*\).*/\1/p' "$P-native.txt")

echo "A4 survey $TS   dll: $DLL   harness exit: $RC"
echo "static : units=$(sv unit_count) runnable=$(sv runnable) empty=$(sv empty) noproc=$(sv noproc) (header $(sv header), units $(sv units_ptr))"
echo "stop   : ${msg:-"(no stop report: see $P-native.txt)"}"
echo "         top of stack: ${top:-?}"
echo "depth  : init_count=${ic%% *} / ${ic##* }   last unit: ${lu:-none}"
echo "         $st   all_units_done=$done_"
echo "         entry check: $ec"
echo "funcs  : executed (distinct)=${fns:-?}  entries=${ent:-?}"

EXIT=0
if [ "$WASM" = 1 ]; then
    WDIR=${FLE_WASM_DIR:-$OUT/bin-base}
    if [ ! -e "$WDIR/flengine.js" ] || ! command -v node >/dev/null; then
        echo "wasm   : not run (no $WDIR/flengine.js or no node)"; EXIT=2
    else
        FLE_WASM_DIR="$WDIR" node "$HERE/wasm/run_node.cjs" "$DLL" > "$P-node.txt" 2> "$P-node.err"
        WRC=$?
        if [ "$(a4lines "$P-native.txt")" = "$(a4lines "$P-node.txt")" ] && [ -n "$(a4lines "$P-node.txt")" ]; then
            echo "wasm   : same a4 lines as native (exit $WRC)"
        else
            echo "wasm   : a4 lines DIFFER from native (exit $WRC): diff <(grep ^a4: $P-native.txt) <(grep ^a4: $P-node.txt)"; EXIT=3
        fi
    fi
fi
echo "logs   : $P-*"
exit $EXIT
