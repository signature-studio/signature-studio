#!/usr/bin/env bash
# R4-5d: builds the diagnostic engine bundle from the worktree of the branch r4-5d-rt (its app/scripts/build-engine-bundle.sh, the same procedure as R4-5c and R5-5). The build-lock is taken with mkdir (not built if it exists), the build needs MemAvailable >= 8192 MB, the cargo target is its own, and the build-lock is released with rmdir when the build ends. Usage: tools/verify-kit/tools/r45d-build.sh <absolute path of the worktree>. Exit 3: not built (lock or memory); otherwise the exit code of the build.
set -u
wt=$1
case "$wt" in /*) ;; *) echo "r45d-build: the worktree must be an absolute path" >&2; exit 2 ;; esac
lock="$HOME/.cache/signature-studio/build-lock"
if ! mkdir "$lock" 2>/dev/null; then
  echo "build-lock present: not built"; exit 3
fi
trap 'rmdir "$lock" 2>/dev/null' EXIT
avail=$(awk '/MemAvailable/ {print int($2/1024)}' /proc/meminfo)
echo "MemAvailable ${avail} MB"
if [ "$avail" -lt 8192 ]; then
  echo "MemAvailable below 8192 MB: not built"; exit 3
fi
cd "$wt" || exit 2
source scripts/env.sh
export CARGO_TARGET_DIR="$HOME/.cache/signature-studio/cargo-r45d"
WORK="$HOME/.cache/signature-studio/work-r45d-diag" HOST_ROOT="$HOME/work/signature-studio/artifacts/host" JOBS=2 B2D_UPSTREAM_LIB="$HOME/.cache/signature-studio/b2d/build-wasm/libblend2d.a" B2D_UPSTREAM_SRC="$HOME/.cache/signature-studio/b2d/src/src" bash app/scripts/build-engine-bundle.sh
rc=$?
echo "build rc=$rc"
exit $rc
