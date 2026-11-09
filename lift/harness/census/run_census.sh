#!/bin/bash
# Census run (docs/decisions/0097-census-diagnostic.md): the whole unit table of FLEngine_x64.dll in two configurations, then the report.
# THE RESULT IS A DIAGNOSTIC AND NOT A4 PROGRESS (decisions 0046, 0060): a unit that stops, faults or hangs is abandoned, not completed.
#
# The binaries are the ones of the normal builds (the census is inside them and is off unless FLE_CENSUS is set):
#   single : OUT=<dir> bash lift/harness/wasm/build_native.sh                       -> $OUT/native/flengine-native
#   two    : gen_two.sh + build_two_native.sh of this directory (FLEngine + the reduced dsp_ippv2 rebased to 0x2000000)
#                                                                                   -> $OUT/v-$TAG/native/two-native
#
# usage: SINGLE=<flengine-native> TWO=<two-native> REAL=<rebased dsp_ippv2_x64.2000000.dll> RUN=<new output dir> \
#        [DLL=binaries/win64/FLEngine_x64.dll] [SITES=<0xVA<TAB>Mnemonic list>] bash lift/harness/census/run_census.sh
# env:   FLE_CENSUS_UNIT_SECONDS (default 20)   FLE_CENSUS_UNIT_CALLS (default 20000000)   per-unit limits, passed through
# Output: $RUN/{single,dsp}-raw.tsv (the harness), $RUN/report-{dsp,single}.tsv and $RUN/report.md (census_report.py), the stdout and
#         stderr of both runs. Nothing is overwritten ($RUN must be new; the harness refuses an existing FLE_CENSUS file).
# SITES: the fle_unsupported sites of the generated C, to name the instruction of an unsupported-instruction wall:
#   grep -h 'fle_unsupported(UINT64_C(0x' <generated C files> | sed -E 's/^L_[0-9a-f]+: \{ \/\* (0x[0-9a-f]+) ([A-Za-z0-9_]+) \*\/.*/\1\t\2/' | sort -u
set -u
HERE=$(cd "$(dirname "$0")" && pwd)
LIFT=$(cd "$HERE/../.." && pwd)
DLL=${DLL:-$LIFT/../binaries/win64/FLEngine_x64.dll}
SINGLE=${SINGLE:?SINGLE is not set (flengine-native)}
TWO=${TWO:?TWO is not set (two-native)}
REAL=${REAL:?REAL is not set (the rebased dsp_ippv2_x64.2000000.dll)}
RUN=${RUN:?RUN is not set (a new directory)}
SITES=${SITES:-}
[ -e "$RUN" ] && { echo "run_census: $RUN exists (use a new directory)" >&2; exit 2; }
mkdir -p "$RUN/tmp" || exit 2
export TMPDIR="$RUN/tmp"
unset FLE_CENSUS
FLE_CENSUS="$RUN/single-raw.tsv" "$SINGLE" "$DLL" > "$RUN/single.out" 2> "$RUN/single.err"
echo "single: exit $? (134 is the expected end: a stop after the unit loop, outside every unit)"
FLE_CENSUS="$RUN/dsp-raw.tsv" "$TWO" "$DLL" "$REAL" > "$RUN/dsp.out" 2> "$RUN/dsp.err"
echo "dsp   : exit $?"
ARGS=(--run "dsp=$RUN/dsp-raw.tsv" --run "single=$RUN/single-raw.tsv" --out-prefix "$RUN/report")
[ -n "$SITES" ] && ARGS+=(--sites "$SITES")
PYTHONDONTWRITEBYTECODE=1 python3 "$HERE/census_report.py" "${ARGS[@]}"
grep '^census:' "$RUN/single.out" "$RUN/dsp.out"
