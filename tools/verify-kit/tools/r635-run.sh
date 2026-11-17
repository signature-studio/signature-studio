#!/usr/bin/env bash
# R6-35: one run for the HEAD engine comparison. Before the run: the machine state (r635-state.sh) and wait-quiet 6 30 (the 1-minute load <= 6, I/O pressure 0, no swap moves over 2 s; a build-lock blocks the run). The run: serve-once-r635-bundle.mjs. After it: the machine state again. The log goes to evidence/R6-35/log/<position>.txt and the run's report and records (report.json, report.md, lat-*.json, audio-p3.json) are copied to evidence/R6-35/<run folder>/. Usage: tools/verify-kit/tools/r635-run.sh <absolute path to the bundle directory> <label: [a-z0-9]+> <lat|song|front|mixmove> <position id, e.g. P3-song-N1>. Exit 3 when the machine was not quiet or a build-lock exists (nothing run); otherwise the exit code of the run.
set -u
bundle=$1; label=$2; scen=$3; pos=$4
here=$(cd "$(dirname "$0")" && pwd)
repo=$(cd "$here/../../.." && pwd)
kit="$repo/tools/verify-kit"
ev="$repo/evidence/R6-35"
mkdir -p "$ev/log"
log="$ev/log/$pos.txt"
: > "$log"
if [ -e "$HOME/.cache/signature-studio/build-lock" ]; then
  echo "build-lock present: not run" | tee -a "$log"; exit 3
fi
{
  echo "## before $pos ($(date -u +%Y-%m-%dT%H:%M:%SZ))"
  "$here/r635-state.sh"
  node "$here/wait-quiet.mjs" 6 30
} >> "$log" 2>&1
if ! grep -q '^quiet:' "$log"; then
  echo "not quiet: not run" | tee -a "$log"; exit 3
fi
node "$kit/serve-once-r635-bundle.mjs" "$bundle" "$label" "$scen" > "$log.run" 2>&1
rc=$?
out=$(grep -m1 -o 'out=[^ ]*' "$log.run" | sed 's/^out=//')
cat "$log.run" >> "$log"; rm -f "$log.run"
echo "## after $pos ($(date -u +%Y-%m-%dT%H:%M:%SZ))" >> "$log"
"$here/r635-state.sh" >> "$log" 2>&1
if [ -n "$out" ] && [ -d "$out" ]; then
  dest="$ev/$(basename "$out")"
  mkdir -p "$dest"
  for f in "$out"/report.json "$out"/report.md "$out"/audio-p3.json "$out"/lat-*.json; do
    [ -e "$f" ] && cp "$f" "$dest/"
  done
  sed -i "s#$HOME#~#g" "$dest"/* 2>/dev/null
  if [ -s "$dest/report.json" ]; then
    echo "saved: evidence/R6-35/$(basename "$out") ($(ls "$dest" | wc -l) files)" >> "$log"
  else
    echo "COPY FAILED: evidence/R6-35/$(basename "$out") has no report.json" >> "$log"
  fi
fi
sed -i "s#$HOME#~#g" "$log"
exit $rc
