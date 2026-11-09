#!/bin/bash
# mk-bundle-assets.sh BUNDLE_DIR NAME [KEY=VALUE ...]   (ci1, 2026-11-07): assets-NAME for serve.sh = the bundle's own manifest with engine.env + each KEY=VALUE (none of them may be there already); proj and proj-solo12 linked. No browser, no build.
set -eu
. /home/ubuntu/work/signature-studio-develop/lift/out-ci1/kit/env.sh
B=$(readlink -f "$1"); N=$2; shift 2; BID=$(basename "$B"); a=$K/assets-$N
[ -f "$B/manifest.json" ] || { echo "no manifest in $B"; exit 2; }
(cd "$B" && sha256sum -c SHA256SUMS | grep -vc ': OK$' | grep -qx 0) || { echo "SHA256SUMS of $B has a non-OK line: stop"; exit 3; }
rm -rf "$a"; mkdir -p "$a"; ln -sfn "$B" "$a/$BID"
python3 - "$B/manifest.json" "$a/manifest.json" "$@" <<'PY'
import json, sys
m = json.load(open(sys.argv[1])); env = m['engine']['env']
for kv in sys.argv[3:]:
    k, v = kv.split('=', 1); assert k not in env, k + ' is already in the manifest'; env[k] = v
json.dump(m, open(sys.argv[2], 'w'), indent=2)
PY
ln -sfn "$K/proj" "$a/proj"; [ -d "$K/proj-solo12" ] && ln -sfn "$K/proj-solo12" "$a/proj-solo12"
echo "assets-$N: $B + $*"
