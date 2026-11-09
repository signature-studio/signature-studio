#!/bin/bash
# ci/test-lifecycle.sh - checks ci/lib/r2_lifecycle.py offline (the rule for perf/ of the cache bucket: added when missing, left alone when there, others kept)
set -u -o pipefail
here=$(cd "$(dirname "$0")" && pwd); fail=0
t=$(mktemp -d); trap 'rm -rf "$t"' EXIT
run() { python3 "$here/lib/r2_lifecycle.py" "$t/cur.json" "$t/new.json" 180 perf/ perf-expire; }
chk() { local want=$1 rc; shift; run; rc=$?; if [ "$rc" = "$want" ]; then echo "ok   $*"; else echo "FAIL $* (rc=$rc, want $want)"; fail=1; fi; }
echo '{"Rules": []}' > "$t/cur.json"; rm -f "$t/new.json"; chk 10 "no rule: added"
python3 - "$t/new.json" <<'PY' || { echo "FAIL the added rule is wrong"; fail=1; }
import json, sys
r = json.load(open(sys.argv[1]))['Rules']
assert len(r) == 1 and r[0] == {'ID': 'perf-expire', 'Status': 'Enabled', 'Filter': {'Prefix': 'perf/'}, 'Expiration': {'Days': 180}}, r
PY
cp "$t/new.json" "$t/cur.json"; rm -f "$t/new.json"; chk 0 "the rule is there: nothing to do"; [ ! -e "$t/new.json" ] && echo "ok   nothing written" || { echo "FAIL a file was written"; fail=1; }
echo '{"Rules": [{"ID": "perf-expire", "Status": "Enabled", "Filter": {"Prefix": "perf/"}, "Expiration": {"Days": 30}}, {"ID": "tmp", "Status": "Enabled", "Filter": {"Prefix": "tmp/"}, "Expiration": {"Days": 7}}]}' > "$t/cur.json"
chk 10 "another number of days: replaced"
python3 - "$t/new.json" <<'PY' || { echo "FAIL the replaced rule is wrong"; fail=1; }
import json, sys
r = json.load(open(sys.argv[1]))['Rules']
assert [x['ID'] for x in r] == ['tmp', 'perf-expire'] and r[1]['Expiration']['Days'] == 180, r
PY
echo '{"Rules": [{"ID": "perf-expire", "Status": "Disabled", "Filter": {"Prefix": "perf/"}, "Expiration": {"Days": 180}}]}' > "$t/cur.json"
chk 10 "a disabled rule: replaced by an enabled one"
echo '{"Rules": [{"ID": "x", "Status": "Enabled", "Prefix": "perf/", "Expiration": {"Days": 180}}]}' > "$t/cur.json"
chk 0 "the old form (a Prefix at the top of the rule) is read"
# the step itself (ci/stage-deploy.sh lifecycle) with a stand-in `aws`: a credential that may not touch bucket settings must not stop the deploy (run 41987198683); a real error must
mkdir -p "$t/bin"
cat > "$t/bin/aws" <<'AWS'
#!/bin/bash
case "$1 $2" in
  "s3api get-bucket-lifecycle-configuration") case "$FAKE_GET" in
      ok) echo '{"Rules": []}';;
      nosuch) echo "An error occurred (NoSuchLifecycleConfiguration) when calling the GetBucketLifecycleConfiguration operation: none" >&2; exit 254;;
      denied) echo "aws: [ERROR]: An error occurred (AccessDenied) when calling the GetBucketLifecycleConfiguration operation: Access Denied" >&2; exit 254;;
      *) echo "Could not connect to the endpoint URL" >&2; exit 255;; esac ;;
  "s3api put-bucket-lifecycle-configuration") case "$FAKE_PUT" in
      ok) exit 0;;
      denied) echo "An error occurred (AccessDenied) when calling the PutBucketLifecycleConfiguration operation: Access Denied" >&2; exit 254;;
      *) echo "Could not connect to the endpoint URL" >&2; exit 255;; esac ;;
esac
AWS
chmod +x "$t/bin/aws"
step() { local want=$1 warn=$2 get=$3 put=$4 out rc has=no; out=$(cd "$here/.." && PATH="$t/bin:$PATH" CI_WORK="$t" R2_ENDPOINT=x <CREDENTIAL-NAME>=x <CREDENTIAL-NAME>=x R2_CACHE_BUCKET=b FAKE_GET=$get FAKE_PUT=$put bash ci/stage-deploy.sh lifecycle 2>&1); rc=$?
  printf '%s' "$out" | grep -q '::warning title=R2 lifecycle not applied' && has=yes
  if [ "$rc" = "$want" ] && [ "$has" = "$warn" ]; then echo "ok   step: get=$get put=$put -> rc $rc, warning=$has"; else echo "FAIL step: get=$get put=$put -> rc $rc warning=$has (want $want, $warn): $(printf '%s' "$out" | tail -2)"; fail=1; fi; }
step 0 yes denied ok          # the credential may not read the bucket settings: a warning, the deploy goes on
step 0 no  nosuch ok          # no lifecycle yet: the rule is added
step 0 yes nosuch denied      # it may read but not set: a warning, the deploy goes on
step 2 no  other ok           # any other error is an error
step 2 no  nosuch other       # so is a failed set that is not a refusal
[ "$fail" = 0 ] && echo "test-lifecycle: all ok" || exit 1
