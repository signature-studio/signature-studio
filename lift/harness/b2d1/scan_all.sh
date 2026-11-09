#!/bin/bash
# Decision 0160 (provisional): runs the Agg2D scan binary (link_b2d1.sh MODE=scan) once per test, each in its own process, like lift/out-x2/tools/scan_all.sh of
# decision 0120's work (the argv of the harness: FLEngine, the five rebased module files; the VFS font seed), and keeps stdout/stderr/exit code per test.
# usage: LIFT=<lift dir> DLLDIR=<dir with the five rebased files> [JOBS=2] scan_all.sh BIN LOGDIR [TEST[:ARG] ...]     (no TEST = every test of `list` + blendmode:0..28 + fuzz:1..40)
set -u
export PYTHONDONTWRITEBYTECODE=1
BIN=$1; LOG=$2; shift 2
HERE=$(cd "$(dirname "$0")" && pwd); LIFT=${LIFT:-$(cd "$HERE/../.." && pwd)}; R=${REPO:-$LIFT/..}
DLLDIR=${DLLDIR:?}; JOBS=${JOBS:-2}
DLLS=$DLLDIR/dsp_ippv2_x64.2000000.dll:$DLLDIR/Blend2D.4700000.dll:$DLLDIR/quickfontcache_x64.1d00000.dll:$DLLDIR/freetype_x64.4b00000.dll:$DLLDIR/ILTools_x64.6000000.dll
FLE=$R/binaries/win64/FLEngine_x64.dll
mkdir -p "$LOG"
export FLE_VFS_SEED=$LIFT/harness/seeds/vfs-seed-fonts.tsv FLE_VFS_SEED_ROOT=$R
if [ $# -eq 0 ]; then
    set -- $(FLE_B2D_SCAN=list "$BIN" "$FLE" "$DLLS" 2>/dev/null | sed -n 's/^scan: TEST \([a-z0-9]*\) .*/\1/p' | grep -v -x -e blendmode -e fuzz -e extreme) $(seq -f 'blendmode:%g' 0 28) $(seq -f 'fuzz:%g' 1 40)
fi
one() {
    t=$1; f=$(echo "$t" | tr ':' '_')
    FLE_B2D_SCAN=$t "$BIN" "$FLE" "$DLLS" > "$LOG/$f.out" 2> "$LOG/$f.err"
    echo "$?" > "$LOG/$f.rc"
    printf '%s rc=%s %s pass %s fail\n' "$t" "$(cat "$LOG/$f.rc")" "$(grep -c '^scan: CHECK .* PASS' "$LOG/$f.out")" "$(grep -c '^scan: CHECK .* FAIL' "$LOG/$f.out")"
}
export -f one; export BIN LOG FLE DLLS
printf '%s\n' "$@" | xargs -P "$JOBS" -I{} bash -c 'one {}'
