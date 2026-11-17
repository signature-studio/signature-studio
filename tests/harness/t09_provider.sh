#!/usr/bin/env bash
# t09: scripts/set-model-profile.sh gateway|bedrock [session-id] (fixed settings + session.switchModel), and the preflight agents row with the local override.
# Test double `opencode`: logs every call (one JSON line in $FAKE_OC_LOG); reload and api -> ok; debug agents -> the REAL resolver run in an isolated sandbox on the
# CURRENT tree's opencode.jsonc and .opencode/opencode.jsonc (the isolated server is stopped again); anything else is refused.
# Bedrock names are built from parts ($v), so this file does not contain the vendor name. No real session, no model call, no real service.
# shellcheck source=lib.sh
. "$(dirname "$0")/lib.sh"
set -o pipefail

sb=$(new_sandbox t09)
v="anthr""opic"
FAKE="$HERE/fakebin"
REAL_OC=$(command -v opencode 2> /dev/null)
export FAKE_SANDBOX="$sb" FAKE_REAL_OC="$REAL_OC"

mkdir -p "$sb/fake"
cat > "$sb/fake/opencode" << 'EOF'
#!/usr/bin/env bash
jq -nc --arg a "$*" '{args:$a}' >> "${FAKE_OC_LOG:-/dev/null}"
case "$1" in
  reload) echo "Configuration reloaded" ;;
  api) echo '{}' ;;
  debug)
    [ "$2" = agents ] || exit 64
    d=$(mktemp -d "$FAKE_SANDBOX/resolve.XXXXXX") || exit 1
    mkdir -p "$d/proj" "$d/xdg/config" "$d/xdg/data" "$d/xdg/state" "$d/xdg/cache"
    cp "$PWD/opencode.jsonc" "$d/proj/"
    if [ -f "$PWD/.opencode/opencode.jsonc" ]; then mkdir -p "$d/proj/.opencode"; cp "$PWD/.opencode/opencode.jsonc" "$d/proj/.opencode/"; fi
    export XDG_CONFIG_HOME="$d/xdg/config" XDG_DATA_HOME="$d/xdg/data" XDG_STATE_HOME="$d/xdg/state" XDG_CACHE_HOME="$d/xdg/cache"
    (cd "$d/proj" && "$FAKE_REAL_OC" --standalone debug agents 2> /dev/null); rc=$?
    "$FAKE_REAL_OC" service stop > /dev/null 2>&1
    exit "$rc" ;;
  *) exit 64 ;;
esac
EOF
chmod +x "$sb/fake/opencode"

repo="$sb/repo"; make_repo_copy "$repo"
git -C "$repo" remote add origin "https://github.com/fake-owner/fake-repo.git"
git -C "$repo" add . > /dev/null 2>&1; git -C "$repo" commit -q -m init > /dev/null 2>&1
log="$sb/oc.log"
override="$repo/.opencode/opencode.jsonc"

smp() { (cd "$repo" && env FAKE_OC_LOG="$log" PATH="$sb/fake:$PATH" bash scripts/set-model-profile.sh "$@" 2>&1); }
row_of() { # tree -> the agents row of preflight, as the real check prints it
  (cd "$1" && env FAKE_GIT_PUSH=ok FAKE_DF_REPO_GIB=600 PATH="$sb/fake:$FAKE:$PATH" bash scripts/preflight.sh 2> /dev/null | awk '$1 == "agents"')
}
tracked_sum() { (cd "$repo" && git ls-files -z | xargs -0 md5sum | md5sum); }
six() { jq -r '[.model, .agents.build.model, .agents.worker.model, .agents.reviewer.model, .agents.title.model, .agents.summary.model] | join(" ")' "$override"; }
calls() { jq -r '.args' "$log"; }
switch_data() { calls | sed -n 's/^api session.switchModel .* --data //p'; }

orig_tracked=$(tracked_sum)
B="console-aws-bedrock/global.$v.claude-"

# ---- 1. bedrock, no session id: the override is written, the server reloads, no model switch
: > "$log"; out=$(smp bedrock); rc=$?
expect_eq "bedrock exits 0" 0 "$rc"
check "the override exists" test -f "$override"
expect_eq "the override has six model lines" 6 "$(grep -c '"model"' "$override")"
expect_eq "the override has no # (no variant anywhere)" 0 "$(grep -c '#' "$override")"
expect_eq "main and build are Sonnet 5.5, the other four Haiku 5.5" "${B}sonnet-5-5 ${B}sonnet-5-5 ${B}haiku-5-5 ${B}haiku-5-5 ${B}haiku-5-5 ${B}haiku-5-5" "$(six)"
expect_eq "every tracked file is unchanged (opencode.jsonc too)" "$orig_tracked" "$(tracked_sum)"
check "the override is git-ignored" git -C "$repo" check-ignore -q .opencode/opencode.jsonc
expect_eq "the only opencode call is reload (no session id: no switchModel)" "reload" "$(calls)"
if [ -n "$REAL_OC" ]; then expect_contains "the agents row is OK for the bedrock override (real resolution)" "$(row_of "$repo")" "OK"; else skip "opencode missing: agents row with the bedrock override"; fi

# ---- 2. bedrock with a session id: reload, then exactly one session.switchModel with the Sonnet model and no variant
: > "$log"; out=$(smp bedrock ses_TEST1); rc=$?
expect_eq "bedrock with a session id exits 0" 0 "$rc"
expect_eq "two calls: reload, then session.switchModel" "reload api" "$(calls | awk '{print $1}' | paste -sd' ' -)"
expect_eq "exactly one session.switchModel" 1 "$(calls | grep -c '^api session.switchModel ')"
expect_contains "for the given session" "$(calls | tail -1)" " --param sessionID=ses_TEST1 --data "
expect_eq "the data holds only the model: provider, id, no variant" "console-aws-bedrock global.$v.claude-sonnet-5-5 [\"model\"] false" \
  "$(jq -r '[.model.providerID, .model.id, (keys | tostring), (.model | has("variant") | tostring)] | join(" ")' <<< "$(switch_data)")"
expect_eq "the override is the same as without a session id" "${B}sonnet-5-5 ${B}sonnet-5-5 ${B}haiku-5-5 ${B}haiku-5-5 ${B}haiku-5-5 ${B}haiku-5-5" "$(six)"

# ---- 3. bad arguments: exit 2, usage on stderr, nothing written, no opencode call
before=$(md5sum < "$override"); : > "$log"
bad() { out=$(smp "$@"); rc=$?; expect_eq "arguments ($*) exit 2" 2 "$rc"; expect_contains "(the usage is printed)" "$out" "usage: scripts/set-model-profile.sh gateway|bedrock [session-id]"; }
bad
bad bogus
bad split
bad --dry-run gateway
bad bedrock ses_1 extra
bad gateway ses_1 extra
expect_eq "bad arguments left the override as it was" "$before" "$(md5sum < "$override")"
expect_eq "bad arguments called no opencode command" 0 "$(wc -l < "$log" | tr -d ' ')"
err=$(cd "$repo" && PATH="$sb/fake:$PATH" bash scripts/set-model-profile.sh bogus 2>&1 > /dev/null)
expect_contains "the usage goes to stderr" "$err" "usage:"

# ---- 4. gateway: the override is removed; with a session id, the model is the same as agents.build.model of opencode.jsonc
: > "$log"; out=$(smp gateway); rc=$?
expect_eq "gateway exits 0" 0 "$rc"
check "the override is removed" test ! -f "$override"
expect_eq "every tracked file is unchanged" "$orig_tracked" "$(tracked_sum)"
expect_eq "the only opencode call is reload" "reload" "$(calls)"
if [ -n "$REAL_OC" ]; then expect_contains "the agents row is OK for the gateway state (real resolution)" "$(row_of "$repo")" "OK"; else skip "opencode missing: agents row for the gateway state"; fi
: > "$log"; out=$(smp gateway ses_TEST2); rc=$?
expect_eq "gateway with a session id (no override to remove) exits 0" 0 "$rc"
want=$(python3 -c 'import sys; sys.path.insert(0, sys.argv[1]); import rules; print(rules.load(sys.argv[2])["agents"]["build"]["model"])' "$HERE/lib" "$repo/opencode.jsonc")
p=${want%%/*}; r=${want#*/}
expect_eq "the session model is agents.build.model of opencode.jsonc ($want)" "$p ${r%%#*} ${r#*#}" \
  "$(jq -r '[.model.providerID, .model.id, .model.variant] | join(" ")' <<< "$(switch_data)")"
expect_contains "for the given session" "$(calls | tail -1)" " --param sessionID=ses_TEST2 --data "
expect_eq "exactly one session.switchModel" 1 "$(calls | grep -c '^api session.switchModel ')"

# ---- 5. no vendor name in the script, and none in any tracked file after the bedrock switch
expect_eq "the script does not contain the vendor name" 0 "$(grep -ci "$v" "$ROOT/scripts/set-model-profile.sh")"
smp bedrock > /dev/null
hits=$(cd "$repo" && git ls-files -z | xargs -0 -r grep -liI "$v" 2> /dev/null | head -5)
expect_eq "no tracked file contains the vendor name after the bedrock switch" "" "$hits"
check "the plugins stay tracked (not ignored)" bash -c "! git -C '$repo' check-ignore -q .opencode/plugins/model-pin-gate.ts"

# ---- 6. the agents row catches a mix of providers in the six settings
if [ -n "$REAL_OC" ]; then
  mixed="$sb/mixed"; make_repo_copy "$mixed"
  cat > "$mixed/.opencode/opencode.jsonc" << EOF
{ "model": "amazon-bedrock/global.$v.claude-sonnet-5-5", "agents": { "title": { "model": "console-cloudflare-ai-gateway-custom/claude-haiku-5-5#low" } } }
EOF
  mrow=$(row_of "$mixed")
  expect_contains "a mix of providers in the six settings is FAIL" "$mrow" "FAIL"
  expect_contains "and the row says more than one provider" "$mrow" "more than one provider"
else
  skip "opencode missing: agents row with a mix of providers"
fi

finish
