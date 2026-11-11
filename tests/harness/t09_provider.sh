#!/usr/bin/env bash
# t09: scripts/set-model-profile.sh --provider gateway|bedrock, and the preflight agents row with the local override.
# Test doubles: `opencode` (models: the list in $FAKE_OC_MODELS; debug agents: the REAL resolver run in an isolated sandbox on the CURRENT tree's
# opencode.jsonc and .opencode/opencode.jsonc, so the answer is for the state being checked; the isolated server is stopped again; reload: ok).
# Bedrock names are built from parts ($v), so this file does not contain the vendor name. No real session, no model call, no real service.
# shellcheck source=lib.sh
. "$(dirname "$0")/lib.sh"
set -o pipefail

sb=$(new_sandbox t09)
v="anthr""opic"
GW="console-cloudflare-ai-gateway-custom"
FAKE="$HERE/fakebin"
REAL_OC=$(command -v opencode 2> /dev/null)
if [ -z "$REAL_OC" ]; then skip "opencode missing: provider switch tests"; finish; exit $?; fi
export FAKE_SANDBOX="$sb" FAKE_REAL_OC="$REAL_OC"

mkdir -p "$sb/fake"
cat > "$sb/fake/opencode" << 'EOF'
#!/usr/bin/env bash
# test double (t09): models -> $FAKE_OC_MODELS; debug agents -> the real resolver on this directory; reload -> ok; anything else is refused
case "$1" in
  models) cat "$FAKE_OC_MODELS" ;;
  reload) echo "Configuration reloaded" ;;
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

# the model lists (what `opencode models` would print)
M1="$sb/models-one.txt"; M2="$sb/models-two.txt"; MG="$sb/models-gateway.txt"
printf '%s\n' "$GW/claude-sonnet-5-5" "$GW/claude-haiku-5-5" \
  "amazon-bedrock/global.$v.claude-sonnet-5-5" "amazon-bedrock/global.$v.claude-haiku-5-5" > "$M1"
printf '%s\n' "$GW/claude-sonnet-5-5" "$GW/claude-haiku-5-5" \
  "amazon-bedrock/global.$v.claude-sonnet-5-5" "amazon-bedrock/global.$v.claude-haiku-5-5" \
  "amazon-bedrock/us.$v.claude-sonnet-5-5" "amazon-bedrock/us.$v.claude-haiku-5-5" "amazon-bedrock/us.amazon.nova-pro-v1:0" > "$M2"
printf '%s\n' "$GW/claude-sonnet-5-5" "$GW/claude-haiku-5-5" > "$MG"

repo="$sb/repo"; make_repo_copy "$repo"
git -C "$repo" remote add origin "https://github.com/fake-owner/fake-repo.git"
git -C "$repo" add . > /dev/null 2>&1; git -C "$repo" commit -q -m init > /dev/null 2>&1

smp() { # models-file, then the script's arguments: the output, and the script's exit code
  local m=$1; shift
  (cd "$repo" && env -u OPENCODE_SESSION_ID FAKE_OC_MODELS="$m" FAKE_GIT_PUSH=ok FAKE_DF_REPO_GIB=600 PATH="$sb/fake:$FAKE:$PATH" bash scripts/set-model-profile.sh "$@" 2>&1)
}
row_of() { # tree -> the agents row of preflight, as the real check prints it
  (cd "$1" && env FAKE_GIT_PUSH=ok FAKE_DF_REPO_GIB=600 PATH="$sb/fake:$FAKE:$PATH" bash scripts/preflight.sh 2> /dev/null | awk '$1 == "agents"')
}
tracked_sum() { (cd "$repo" && git ls-files -z | xargs -0 md5sum | md5sum); }
main_sum() { md5sum < "$repo/opencode.jsonc" | cut -d' ' -f1; }
six() { # the six model settings of the override, in order: top, build, worker, reviewer, title, summary
  grep -v '^//' "$repo/.opencode/opencode.jsonc" | jq -r '[.model, .agents.build.model, .agents.worker.model, .agents.reviewer.model, .agents.title.model, .agents.summary.model] | join(" ")'
}

orig_main=$(main_sum)
orig_tracked=$(tracked_sum)
B="amazon-bedrock/global.$v.claude-"

# ---- 1. gateway -> bedrock (split): the tracked files do not change, the override carries the six settings
out=$(smp "$M1" --provider bedrock --bedrock-region global split); rc=$?
expect_eq "bedrock switch with one candidate per model exits 0" 0 "$rc"
expect_eq "opencode.jsonc is byte-identical after the bedrock switch" "$orig_main" "$(main_sum)"
expect_eq "every tracked file is unchanged after the bedrock switch" "$orig_tracked" "$(tracked_sum)"
check "the override exists after the bedrock switch" test -f "$repo/.opencode/opencode.jsonc"
expect_eq "the override holds the six settings (split: Sonnet main, Haiku for the rest)" \
  "${B}sonnet-5-5 ${B}sonnet-5-5#xhigh ${B}haiku-5-5#high ${B}haiku-5-5#high ${B}haiku-5-5#low ${B}haiku-5-5#low" "$(six)"
expect_contains "the output says what was switched, in plain words" "$out" "切り替えました"
expect_not_contains "the output does not say the change was undone" "$out" "取り消しました"
expect_contains "the agents row is OK for the bedrock state (real resolution)" "$(row_of "$repo")" "OK"
hits=$(cd "$repo" && git ls-files -z | xargs -0 -r grep -liI "$v" 2> /dev/null | head -5)
expect_eq "no tracked file contains the vendor name after the bedrock switch" "" "$hits"
check "the override is git-ignored" git -C "$repo" check-ignore -q .opencode/opencode.jsonc
check "the plugins stay tracked (not ignored)" bash -c "! git -C '$repo' check-ignore -q .opencode/plugins/model-pin-gate.ts"

# ---- 2. the profile is kept when no provider is given (current = bedrock)
out=$(smp "$M1" haiku); rc=$?
expect_eq "no --provider keeps bedrock and changes to haiku: exit 0" 0 "$rc"
expect_eq "all six settings are Haiku 5.5 under haiku (bedrock kept; top level has no variant, as in opencode.jsonc)" \
  "${B}haiku-5-5 ${B}haiku-5-5#high ${B}haiku-5-5#high ${B}haiku-5-5#high ${B}haiku-5-5#low ${B}haiku-5-5#low" "$(six)"
expect_eq "every tracked file is still unchanged" "$orig_tracked" "$(tracked_sum)"
expect_contains "the agents row is OK for all-haiku (real resolution)" "$(row_of "$repo")" "OK"
out=$(smp "$M1" --dry-run split); expect_eq "--dry-run is allowed (exit 0)" 0 "$?"
expect_contains "--dry-run says nothing was written" "$out" "何も書き換えていません"
expect_eq "(the override is still the haiku one)" "${B}haiku-5-5 ${B}haiku-5-5#high ${B}haiku-5-5#high ${B}haiku-5-5#high ${B}haiku-5-5#low ${B}haiku-5-5#low" "$(six)"

# ---- 3. bedrock -> gateway: the override is removed, the main file is restored (split = the committed profile)
out=$(smp "$M1" --provider gateway split); rc=$?
expect_eq "gateway split exits 0" 0 "$rc"
check "the override is removed when going back to the gateway" test ! -f "$repo/.opencode/opencode.jsonc"
expect_eq "back to the original opencode.jsonc, byte for byte" "$orig_main" "$(main_sum)"
expect_contains "the agents row is OK for the gateway state" "$(row_of "$repo")" "OK"
out=$(smp "$M1" --provider gateway haiku); rc=$?
expect_eq "gateway haiku: exit 0" 0 "$rc"
expect_eq "gateway haiku: the main file's six lines are all Haiku 5.5 (top, build, worker, reviewer, title, summary)" "6" \
  "$(grep -c "\"model\": \"$GW/claude-haiku-5-5" "$repo/opencode.jsonc")"
out=$(smp "$M1" --provider gateway split); expect_eq "gateway split again: exit 0" 0 "$?"
expect_eq "and the main file is the original again" "$orig_main" "$(main_sum)"

# ---- 4. the choice is ambiguous: nothing changes, the candidates are shown
out=$(smp "$M2" --provider bedrock haiku); rc=$?
expect_eq "several candidates without a region exit 3 (stop)" 3 "$rc"
expect_contains "the candidates are shown" "$out" "候補（Haiku 5.5）"
expect_contains "and it says nothing was changed" "$out" "何も変えていません"
check "no override is written when the choice is ambiguous" test ! -f "$repo/.opencode/opencode.jsonc"
expect_eq "(the main file is untouched)" "$orig_main" "$(main_sum)"
out=$(smp "$MG" --provider bedrock split); rc=$?
expect_eq "no Bedrock name at all exits 3 (stop)" 3 "$rc"
expect_contains "and it says the model was not found" "$out" "見つかりません"
out=$(smp "$M2" --provider bedrock --bedrock-region us split); rc=$?
expect_eq "with a region given the choice is clear: exit 0" 0 "$rc"
expect_eq "(the region us is in the override)" "amazon-bedrock/us.$v.claude-sonnet-5-5 amazon-bedrock/us.$v.claude-sonnet-5-5#xhigh amazon-bedrock/us.$v.claude-haiku-5-5#high amazon-bedrock/us.$v.claude-haiku-5-5#high amazon-bedrock/us.$v.claude-haiku-5-5#low amazon-bedrock/us.$v.claude-haiku-5-5#low" "$(six)"
out=$(smp "$M2" --provider gateway split); expect_eq "back to the gateway: exit 0" 0 "$?"

# ---- 5. a session is refused (the owner runs it from a terminal)
out=$(cd "$repo" && env OPENCODE_SESSION_ID=ses_abc FAKE_OC_MODELS="$M1" PATH="$sb/fake:$FAKE:$PATH" bash scripts/set-model-profile.sh --provider bedrock --bedrock-region global split 2>&1); rc=$?
expect_eq "refuses inside a session (exit 2)" 2 "$rc"
expect_contains "and says the owner runs it" "$out" "セッションの中からは切り替えられません"
check "(no override was written)" test ! -f "$repo/.opencode/opencode.jsonc"

# ---- 6. the agents row catches a mix of providers in the six settings
mixed="$sb/mixed"; make_repo_copy "$mixed"
cat > "$mixed/.opencode/opencode.jsonc" << EOF
{ "model": "amazon-bedrock/global.$v.claude-sonnet-5-5", "agents": { "title": { "model": "$GW/claude-haiku-5-5#low" } } }
EOF
mrow=$(row_of "$mixed")
expect_contains "a mix of providers in the six settings is FAIL" "$mrow" "FAIL"
expect_contains "and the row says more than one provider" "$mrow" "more than one provider"

finish
