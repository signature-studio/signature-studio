#!/bin/bash
# post-pure.sh E: analysis and copy after `verify-bundle.sh pure chrome R` (no browser, light CPU). E = e13 etc.
# env: NEW PREV REF QUIET_LOAD (as for verify-bundle.sh). Writes out/P3-chrome.md, out/RESULTS.md, then copies the run files to lift/out-git/ci1-results/0277/E/data
set -u
E=${1:?e13}; K=$(cd "$(dirname "$0")/.." && pwd); D=/home/ubuntu/work/signature-studio-develop/lift/out-git/ci1-results/0277/$E
cd "$K" || exit 1
export QUIET_LOAD=${QUIET_LOAD:-8}
python3 tools/p3-win.py chrome "$QUIET_LOAD" > out/P3-chrome.stdout.txt 2>&1; echo "p3-win rc=$?"
./verify-bundle.sh report > out/report.stdout.txt 2>&1; echo "report rc=$?"
mkdir -p "$D/data" && cd out || exit 1
for d in p3-chrome-new-* p3-chrome-prev-*; do
  [ -d "$d" ] || continue
  [ -f "$d/audio-p3.json" ] && cp "$d/audio-p3.json" "$D/data/$d-audio.json"
  [ -f "$d/report.json" ] && cp "$d/report.json" "$D/data/$d-report.json"
  [ -f "$d/wait.txt" ] && cp "$d/wait.txt" "$D/data/$d-wait.txt"
done
for f in P3-chrome.md p3-chrome.tsv p3-chrome-more.tsv p3-reference.json chrome-count.tsv steps.log RESULTS.md; do [ -f "$f" ] && cp "$f" "$D/data/$f"; done
cp RESULTS.md "$D/RESULTS-${E^^}.md"
echo "copied to $D/data: $(ls "$D/data" | wc -l) files"
