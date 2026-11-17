#!/usr/bin/env bash
# R4-4l: one P2 profile run (serve-once-r44l.mjs) under the machine's shared rules. Order: wait-quiet 6 30 first (no lock held while waiting); then mkdir build-lock and mkdir chrome-lock (each mkdir is the take: if it fails, the lock is someone else's and nothing runs); the run; the state before and after; both locks are removed with rmdir whatever the run's result. Usage: tools/verify-kit/tools/r44l-run.sh <absolute path to the bundle directory> <run label: [0-9]+>. Exit 3 when a lock was not free (nothing run); otherwise the exit code of the run. The output folder is printed as out=<path> in the log.
set -u
bundle=$1; label=$2
here=$(cd "$(dirname "$0")" && pwd)
repo=$(cd "$here/../../.." && pwd)
lock_b="$HOME/.cache/signature-studio/build-lock"
lock_c="$HOME/.cache/signature-studio/chrome-lock"
logdir="$HOME/.cache/signature-studio/tmp/r44l"
mkdir -p "$logdir"
log="$logdir/run-$label.txt"
node "$here/wait-quiet.mjs" 6 30 > "$log.quiet" 2>&1 || { echo "not quiet: not run" | tee "$log"; exit 3; }
if ! mkdir "$lock_b" 2>/dev/null; then echo "build-lock exists (not ours): not run" | tee "$log"; exit 3; fi
if ! mkdir "$lock_c" 2>/dev/null; then rmdir "$lock_b"; echo "chrome-lock exists (not ours): not run" | tee "$log"; exit 3; fi
{
  echo "## before run $label ($(date -u +%Y-%m-%dT%H:%M:%SZ))"
  cat "$log.quiet"
  bash "$here/r635-state.sh"
} > "$log.before" 2>&1
cd "$repo" && node tools/verify-kit/serve-once-r44l.mjs "$bundle" > "$log.run" 2>&1
rc=$?
{
  echo "## after run $label ($(date -u +%Y-%m-%dT%H:%M:%SZ)) rc=$rc"
  bash "$here/r635-state.sh"
} > "$log.after" 2>&1
rmdir "$lock_c"
rmdir "$lock_b"
cat "$log.run"
exit $rc
