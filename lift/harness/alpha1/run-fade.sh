#!/bin/bash
# Decision 0244 (alpha1): the fade of a pull-down and the fade-in of the plug-in picker on the REAL engine (a native -O2 build with the recorder tools/alpha1_rec.c of docs/investigations/alpha1/tools/), checked frame by frame.
# THE INPUT IS A HARNESS SIMULATION (a G3 script), NOT A USER OPERATION. Not part of the CI (it needs a native build of the lifted engine, an hour to make on this machine); lift/crates/fe-lift/tests/alpha1_glass.rs is the unit test of the same code.
# usage: run-fade.sh N1_BIN OUT_DIR      (N1_BIN: built by docs/investigations/alpha1/tools/build-n1.sh; run-n1.sh of that directory is used to run it)
# Result: OUT_DIR/pull1/ (api.log, crops/), and one line per check; exit status 1 when a check fails.
set -u -o pipefail
BIN=$(readlink -f "$1"); OUT=$2; HERE=$(cd "$(dirname "$0")" && pwd)
TOOLS=${A1_TOOLS:-$HERE/../../../docs/investigations/alpha1/tools}
mkdir -p "$OUT"
export A1_BBSIZE="206x279;113x108" A1_BBPIX="30,262;40,97" A1_PIX="34,298;99,133" A1_CROP="4,36,208,333;59,36,345,231"
bash "$TOOLS/run-n1.sh" "$BIN" "$HERE/g3/pull1.g3" "$OUT/pull1" "$OUT/pull1/dump" || true
rc=0
# the Add menu, row 'Simple' (the pixel (40,97) of the 113x108 window that is left after the choice; below it: the project)
python3 "$HERE/check-fade.py" "$OUT/pull1" --size 113x108 --px 40,97 --crop 1 --origin 59,36 --cropsize 345,231 --tsv "$OUT/fade-addmenu-simple.tsv" || rc=1
exit $rc
