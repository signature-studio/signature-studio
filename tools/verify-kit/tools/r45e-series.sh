#!/usr/bin/env bash
# R4-5e: holds the build-lock with mkdir (not run if it exists and is not held by this script), marks it with the file r45e-hold, runs the P3 profiled runs (scn-r45e-mixprof.json) of the diagnostic bundle one after another with r45e-run.sh, and releases the build-lock with rmdir at the end. Usage: tools/verify-kit/tools/r45e-series.sh <absolute bundle directory> <absolute app directory with dist/> <position id>... (e.g. P2-D1 P2-D2 P2-D3). Exit 3 when the lock is held by another work (nothing run).
set -u
bundle=$1; app=$2; shift 2
lock="$HOME/.cache/signature-studio/build-lock"
if ! mkdir "$lock" 2>/dev/null; then
  echo "build-lock held by another work: series not run"; exit 3
fi
if [ -e "$HOME/.cache/signature-studio/chrome-lock" ]; then
  rmdir "$lock" 2>/dev/null; echo "chrome-lock held by another work: series not run"; exit 3
fi
touch "$lock/r45e-hold"
trap 'rm -f "$lock/r45e-hold"; rmdir "$lock" 2>/dev/null' EXIT
here=$(cd "$(dirname "$0")" && pwd)
rc=0
for pos in "$@"; do
  "$here/r45e-run.sh" "$bundle" p "${R45E_SCEN:-mixprof}" "$pos" "$app"
  r=$?
  echo "series: $pos rc=$r"
  if [ "$r" -ne 0 ]; then rc=$r; break; fi
done
exit $rc
