#!/bin/bash
# ci/test-wrangler-config.sh - offline checks of ci/lib/wrangler_config.py (decision 0224: the PERF_BUCKET binding). No network, no credential. Exit 0 = all cases behaved.
set -u -o pipefail
. "$(dirname "$0")/lib/common.sh"
T=$(mktemp -d); trap 'rm -rf "$T"' EXIT
CFG=$REPO/app/wrangler.jsonc
AUD=$(printf 'a%.0s' $(seq 64))
export ACCESS_TEAM_DOMAIN=team-x.cloudflareaccess.com ACCESS_AUD=$AUD PUBLIC_HOSTNAME=fl.example.com
fail=0
run() { python3 "$CI_DIR/lib/wrangler_config.py" "$1" "$T/out.json" >"$T/log" 2>&1; }
ok()  { if "$@"; then echo "ok   $DESC"; else echo "FAIL $DESC"; cat "$T/log"; fail=1; fi; }
refuse() { if "$@"; then echo "FAIL $DESC (accepted)"; fail=1; else echo "ok   $DESC (refused: $(tail -1 "$T/log"))"; fi; }
bucket() { python3 -c 'import json,sys; d=json.load(open(sys.argv[1])); print([b["bucket_name"] for b in d["r2_buckets"] if b["binding"]==sys.argv[2]][0])' "$T/out.json" "$1"; }

DESC="both buckets are named"; ASSETS_BUCKET_NAME=fl-web-assets PERF_BUCKET_NAME=fl-web-cache ok run "$CFG"
DESC="ASSETS_BUCKET is the delivery bucket"; [ "$(bucket ASSETS_BUCKET)" = fl-web-assets ] && echo "ok   $DESC" || { echo "FAIL $DESC"; fail=1; }
DESC="PERF_BUCKET is the beacon bucket"; [ "$(bucket PERF_BUCKET)" = fl-web-cache ] && echo "ok   $DESC" || { echo "FAIL $DESC"; fail=1; }
DESC="no placeholder name is left"; ! grep -q changeme "$T/out.json" && echo "ok   $DESC" || { echo "FAIL $DESC"; fail=1; }
DESC="PERF_BUCKET_NAME missing"; ASSETS_BUCKET_NAME=fl-web-assets PERF_BUCKET_NAME='' refuse run "$CFG"
DESC="PERF_BUCKET_NAME is not a bucket name"; ASSETS_BUCKET_NAME=fl-web-assets PERF_BUCKET_NAME='Bad Name!' refuse run "$CFG"
DESC="the beacon bucket is the delivery bucket"; ASSETS_BUCKET_NAME=fl-web-assets PERF_BUCKET_NAME=fl-web-assets refuse run "$CFG"
python3 - "$CFG" "$T/nobinding.jsonc" <<'PY'
import re, sys
s = open(sys.argv[1], encoding="utf-8").read()
s = re.sub(r',\s*//[^\n]*\n(\s*//[^\n]*\n)*\s*\{ "binding": "PERF_BUCKET"[^\n]*\n', '\n', s)
assert "PERF_BUCKET" not in s.replace("// decision", "")
open(sys.argv[2], "w", encoding="utf-8").write(s)
PY
DESC="app/wrangler.jsonc without a PERF_BUCKET binding"; ASSETS_BUCKET_NAME=fl-web-assets PERF_BUCKET_NAME=fl-web-cache refuse run "$T/nobinding.jsonc"
[ "$fail" = 0 ] && echo "all cases behaved" || { echo "SOME CASE FAILED"; exit 1; }
