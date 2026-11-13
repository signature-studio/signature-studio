#!/bin/bash
# mk-s3a-assets.sh [B2DIR]   (S3a sweep, 0312c 5c; ci1 0326). Makes assets-b2w{2,4,8,16}/ next to the kit's other assets dirs: the manifest of the B2 bundle with ONE change, engine.env FLE_WORKERS = n; the bundle's files are linked, not copied.
# Reads and writes files only (no browser, no server, no build). The bundle must be the B2 bundle of mt3 (fl-engine-ci-8b2eb5d7); its SHA256SUMS is checked first.
set -eu
. "$(dirname "${BASH_SOURCE[0]}")/../env.sh"
B2=${1:-$HOME/work/mt3w-s1/ci-b2/engine/bundle/fl-engine-ci-8b2eb5d7}; BID=$(basename "$B2")
[ -f "$B2/manifest.json" ] || { echo "no manifest in $B2"; exit 2; }
(cd "$B2" && sha256sum -c SHA256SUMS >/dev/null) || { echo "SHA256SUMS of $B2 does not verify: stop"; exit 3; }
for n in 2 4 8 16; do
  a=$K/assets-b2w$n; rm -rf "$a"; mkdir -p "$a"
  ln -sfn "$B2" "$a/$BID"
  python3 - "$B2/manifest.json" "$a/manifest.json" "$n" <<'PY'
import json, sys
m = json.load(open(sys.argv[1])); n = sys.argv[3]
assert 'FLE_WORKERS' not in m['engine']['env'], 'the B2 manifest already has FLE_WORKERS: ' + m['engine']['env']['FLE_WORKERS']
m['engine']['env']['FLE_WORKERS'] = n
json.dump(m, open(sys.argv[2], 'w'), indent=2)
PY
  ln -sfn "$K/proj" "$a/proj"; [ -d "$K/proj-solo12" ] && ln -sfn "$K/proj-solo12" "$a/proj-solo12"
  echo "assets-b2w$n: FLE_WORKERS=$n (only difference from $B2/manifest.json: $(diff <(python3 -c "import json;print(json.dumps(json.load(open('$B2/manifest.json')),indent=2,sort_keys=True))") <(python3 -c "import json;print(json.dumps(json.load(open('$a/manifest.json')),indent=2,sort_keys=True))") | grep -c '^[<>]') line(s))"
done
# the control E13 (production): its assets dir exists (verify-bundle.sh fetch); it needs the link to the solo project too
c=$K/assets-fl-engine-ci-0363b050; [ -d "$K/proj-solo12" ] && [ -d "$c" ] && ln -sfn "$K/proj-solo12" "$c/proj-solo12" || true
