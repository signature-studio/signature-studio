#!/usr/bin/env bash
# R4-5d: one run (a copy of r635-run.sh with the output in evidence/R4-5d; the page is the app of the same tree as the bundle: serve-once-r45d-bundle.mjs). Before the run: the machine state (r635-state.sh) and wait-quiet 6 30 (the 1-minute load <= 6, I/O pressure 0, no swap moves over 2 s; a build-lock blocks the run). The run: serve-once-r45d-bundle.mjs (the app of the same tree as the bundle). After it: the machine state again. The log goes to evidence/R4-5d/log/<position>.txt and the run's report and records (report.json, report.md, lat-*.json, audio-p3.json) are copied to evidence/R4-5d/<run folder>/. Usage: tools/verify-kit/tools/r45d-run.sh <absolute bundle directory> <label: [a-z0-9]+> <lat|latlog|song|front|mixmove> <position id, e.g. P2-D1> <absolute app directory with dist/>. Exit 3 when the machine was not quiet or a build-lock exists (nothing run); otherwise the exit code of the run.
set -u
bundle=$1; label=$2; scen=$3; pos=$4; app=$5
here=$(cd "$(dirname "$0")" && pwd)
repo=$(cd "$here/../../.." && pwd)
kit="$repo/tools/verify-kit"
ev="$repo/evidence/R4-5d"
mkdir -p "$ev/log"
log="$ev/log/$pos.txt"
: > "$log"
if [ -e "$HOME/.cache/signature-studio/build-lock" ] && [ ! -e "$HOME/.cache/signature-studio/build-lock/r45d-hold" ]; then
  echo "build-lock present: not run" | tee -a "$log"; exit 3
fi
if ! mkdir "$HOME/.cache/signature-studio/chrome-lock" 2>/dev/null; then
  echo "chrome-lock present: not run" | tee -a "$log"; exit 3
fi
trap 'rmdir "$HOME/.cache/signature-studio/chrome-lock" 2>/dev/null' EXIT
{
  echo "## before $pos ($(date -u +%Y-%m-%dT%H:%M:%SZ))"
  "$here/r635-state.sh"
  node "$here/wait-quiet.mjs" 6 30
} >> "$log" 2>&1
if ! grep -q '^quiet:' "$log"; then
  echo "not quiet: not run" | tee -a "$log"; exit 3
fi
node "$kit/serve-once-r45d-bundle.mjs" "$bundle" "$label" "$scen" "$app" > "$log.run" 2>&1
rc=$?
out=$(grep -m1 -o 'out=[^ ]*' "$log.run" | sed 's/^out=//')
cat "$log.run" >> "$log"; rm -f "$log.run"
echo "## after $pos ($(date -u +%Y-%m-%dT%H:%M:%SZ))" >> "$log"
"$here/r635-state.sh" >> "$log" 2>&1
if [ -n "$out" ] && [ -d "$out" ]; then
  dest="$ev/$(basename "$out")"
  mkdir -p "$dest"
  for f in "$out"/report.json "$out"/report.md "$out"/audio-p3.json "$out"/lat-*.json "$out"/*.log; do
    [ -e "$f" ] && cp "$f" "$dest/"
  done
  sed -i "s#$HOME#~#g" "$dest"/* 2>/dev/null
  if [ -s "$dest/report.json" ]; then
    echo "saved: evidence/R4-5d/$(basename "$out") ($(ls "$dest" | wc -l) files)" >> "$log"
  else
    echo "COPY FAILED: evidence/R4-5d/$(basename "$out") has no report.json" >> "$log"
  fi
fi
sed -i "s#$HOME#~#g" "$log"
exit $rc
