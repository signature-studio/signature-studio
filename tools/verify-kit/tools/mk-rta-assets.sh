#!/bin/bash
# mk-rta-assets.sh   (ci1, 2026-11-07, PM 09:3x): the RT-A test bundle (real threads, GIL=1; mt3 0315) with engine.env changed ONLY by one key each, for the P3 sweep and the Export check. No browser, no server, no build.
#   assets-rta     the manifest as it is (n = navigator.hardwareConcurrency, Workers N = clamp(n-1, 2, 8))
#   assets-rta-w2  + FLE_WORKERS=2          (N = 2, n as the host's)
#   assets-rta-p2  + FLE_PROCESSORS=2       (n = 2; N as the bundle derives it)
#   assets-rta-p2w2 + FLE_PROCESSORS=2 FLE_WORKERS=2
set -eu
. /home/ubuntu/work/signature-studio-develop/lift/out-ci1/kit/env.sh
B=${1:-$HOME/work/mt3w-s1/ci-rt1/engine/bundle/fl-engine-test-rt1-91bf78f6}; BID=$(basename "$B")
[ -f "$B/manifest.json" ] || { echo "no manifest in $B"; exit 2; }
(cd "$B" && sha256sum -c SHA256SUMS | grep -vc ': OK$' | grep -qx 0) || { echo "SHA256SUMS of $B has a non-OK line: stop"; exit 3; }
mk() { # name key=value ...
  local n=$1; shift; local a=$K/assets-$n; rm -rf "$a"; mkdir -p "$a"; ln -sfn "$B" "$a/$BID"
  python3 - "$B/manifest.json" "$a/manifest.json" "$@" <<'PY'
import json, sys
m = json.load(open(sys.argv[1])); env = m['engine']['env']
for kv in sys.argv[3:]:
    k, v = kv.split('=', 1); assert k not in env, k + ' is already in the manifest'; env[k] = v
json.dump(m, open(sys.argv[2], 'w'), indent=2)
PY
  ln -sfn "$K/proj" "$a/proj"; [ -d "$K/proj-solo12" ] && ln -sfn "$K/proj-solo12" "$a/proj-solo12"
  echo "assets-$n: $* (diff vs bundle manifest: $(diff <(python3 -c "import json;print(json.dumps(json.load(open('$B/manifest.json')),indent=2,sort_keys=True))") <(python3 -c "import json;print(json.dumps(json.load(open('$a/manifest.json')),indent=2,sort_keys=True))") | grep -c '^[<>]') lines)"
}
mk rta
mk rta-w2 FLE_WORKERS=2
mk rta-p2 FLE_PROCESSORS=2
mk rta-p2w2 FLE_PROCESSORS=2 FLE_WORKERS=2
