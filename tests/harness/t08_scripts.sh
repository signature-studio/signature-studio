#!/usr/bin/env bash
# t08: scripts/launch.sh, scripts/env.sh. (scripts/set-model-profile.sh is tested in t09.)
#  - launch.sh runs ONLY with a fake opencode (tests/harness/fakebin-opencode) and a redirected HOME: no real session is created, no registry of the real machine is touched.
#  - env.sh is sourced in a subshell.
# shellcheck disable=SC2016  # the strings given to envrun are code for the inner bash: they must not expand in this shell
# shellcheck source=lib.sh
. "$(dirname "$0")/lib.sh"
set -o pipefail

sb=$(new_sandbox t08)
FAKE="$HERE/fakebin"; FAKEOC="$HERE/fakebin-opencode"

# the REAL resolver's `debug agents` output for a file, in an isolated sandbox (the fake opencode of launch.sh answers with it)
resolve_real() { # file -> JSON on stdout
  local d="$sb/res-$$-$RANDOM"; mkdir -p "$d/proj"; cp "$1" "$d/proj/opencode.jsonc"
  (
    export XDG_CONFIG_HOME="$d/xdg/config" XDG_DATA_HOME="$d/xdg/data" XDG_STATE_HOME="$d/xdg/state" XDG_CACHE_HOME="$d/xdg/cache"
    mkdir -p "$XDG_CONFIG_HOME" "$XDG_DATA_HOME" "$XDG_STATE_HOME" "$XDG_CACHE_HOME"
    cd "$d/proj" && opencode --standalone debug agents 2> /dev/null
    opencode service stop > /dev/null 2>&1
  )
}

# ====================== launch.sh (fake opencode only; HOME redirected)
lrepo="$sb/launch"; make_repo_copy "$lrepo"
lhome="$sb/lhome"; mkdir -p "$lhome/.config/opencode"
log="$sb/launch.log"
# one launch in the repository copy $1 (the other arguments go to launch.sh). HOME is $LHOME if set, else $lhome; the fake gives preflight the resolver answer in $FAKE_AGENTS if set, else the good file's
launch_in() { local d=$1; shift; (cd "$d" && env -u OPENCODE_SESSION_ID FAKE_GIT_PUSH=ok FAKE_DF_REPO_GIB=600 HOME="${LHOME:-$lhome}" FAKE_GH=private FAKE_OC_LOG="$log" FAKE_OC_AGENTS="${FAKE_AGENTS:-$sb/agents-launch.json}" PATH="$FAKEOC:$FAKE:$PATH" bash scripts/launch.sh "$@" 2>&1); }
# the model of the last session.create the fake received, as launch.sh sent it
create_model() { jq -r '.args' "$log" | sed -n 's/^api session\.create --data //p' | tail -1 | jq -c '.model'; }
# preflight (the first step) needs a resolver answer for the good file
resolve_real "$ROOT/opencode.jsonc" > "$sb/agents-launch.json"

# the real launch against the fake
: > "$log"
out=$(cd "$lrepo" && env -u OPENCODE_SESSION_ID FAKE_GIT_PUSH=ok FAKE_DF_REPO_GIB=600 HOME="$lhome" FAKE_GH=private FAKE_OC_LOG="$log" FAKE_OC_AGENTS="$sb/agents-launch.json" PATH="$FAKEOC:$FAKE:$PATH" bash scripts/launch.sh 2>&1); rc=$?
if [ "$rc" -eq 0 ]; then
  pass "launch.sh (against the fake) exits 0"
  expect_contains "reports the new session id" "$out" "セッション ses_FAKELAUNCH0001 を作成しました"
  calls=$(jq -r '.args' "$log")
  expect_eq "no override: the session is created on agents.build.model of opencode.jsonc (with its variant)" '{"providerID":"console-cloudflare-ai-gateway-custom","id":"claude-sonnet-5-5","variant":"xhigh"}' "$(create_model)"
  expect_contains "session.create: agent build, in the repository directory" "$(grep 'api session.create' <<< "$calls")" "\"agent\":\"build\",\"model\":{"
  expect_contains "(and the repository directory)" "$(grep 'api session.create' <<< "$calls")" "\"location\":{\"directory\":\"$lrepo\"}"
  order=$(grep -oE '^debug agents|^api session\.(create|prompt|command)' <<< "$calls" | sed 's/^api //' | paste -sd' ' -)
  expect_eq "order: preflight's agent check, create, prompt, command" "debug agents session.create session.prompt session.command" "$order"
  expect_eq "exactly one session.create" 1 "$(grep -c 'api session.create' <<< "$calls")"
  expect_eq "exactly one session.prompt (the human's launch)" 1 "$(grep -c 'api session.prompt' <<< "$calls")"
  expect_eq "session.synthetic is not used" 0 "$(grep -c 'session.synthetic' <<< "$calls")"
  expect_contains "the prompt goes to the new session" "$(grep 'api session.prompt' <<< "$calls")" "sessionID=ses_FAKELAUNCH0001"
  # the fake logs the arguments; the body is JSON with the whole START.md as one string
  pbody=$(tr '\n' ' ' <<< "$calls" | grep -o 'api session.prompt.*api session.command' | sed 's/ *api session.command$//')
  expect_contains "the prompt body is the whole START.md (its first line)" "$pbody" '"text": "start\n'
  expect_contains "(and its GOAL section, so the file was sent unmodified)" "$pbody" '## GOAL\nmake it work'
  expect_contains "the goal command goes to the new session with name goal" "$(grep 'api session.command' <<< "$calls")" '"name":"goal"'
  expect_contains "the goal text is the body of the GOAL section" "$(grep 'api session.command' <<< "$calls")" '"text":"make it work"'
  reg="$lhome/.config/opencode/goal-nudge.sessions"
  expect_eq "the new id is in the registry exactly once" 1 "$(grep -c '^ses_FAKELAUNCH0001' "$reg")"
  out2=$(cd "$lrepo" && env -u OPENCODE_SESSION_ID FAKE_GIT_PUSH=ok FAKE_DF_REPO_GIB=600 HOME="$lhome" FAKE_GH=private FAKE_OC_LOG="$log" FAKE_OC_AGENTS="$sb/agents-launch.json" PATH="$FAKEOC:$FAKE:$PATH" bash scripts/launch.sh 2>&1)
  expect_eq "a second launch (same fake id) does not duplicate the registry line" 1 "$(grep -c '^ses_FAKELAUNCH0001' "$reg")"
  expect_contains "and says it is already there" "$out2" "既に"
else
  fail "launch.sh against the fake exited $rc: $(printf '%s' "$out" | tail -3 | tr '\n' '|' | cut -c1-300)"
fi

# --- the model follows opencode.jsonc (agents.build.model set to Haiku -> the session is created on Haiku; the fake resolver answers accordingly so preflight passes)
hrepo="$sb/launch-haiku"; make_repo_copy "$hrepo"
sed -i '0,/"model": "console-cloudflare-ai-gateway-custom\/claude-sonnet-5-5#xhigh"/s//"model": "console-cloudflare-ai-gateway-custom\/claude-haiku-5-5#high"/' "$hrepo/opencode.jsonc"
jq 'map(if .id == "build" then .model.id = "claude-haiku-5-5" | .model.variant = "high" else . end)' "$sb/agents-launch.json" > "$sb/agents-haiku.json"
: > "$log"; LHOME="$sb/lhome-haiku" FAKE_AGENTS="$sb/agents-haiku.json" launch_in "$hrepo" > /dev/null; rc=$?
expect_eq "no override, agents.build.model set to Haiku: exit 0" 0 "$rc"
expect_eq "(the session is created on Haiku with its variant)" '{"providerID":"console-cloudflare-ai-gateway-custom","id":"claude-haiku-5-5","variant":"high"}' "$(create_model)"

# --- the local override .opencode/opencode.jsonc (set-model-profile.sh bedrock writes it; not in Git): its agents.build.model is the effective pin, so the session is created on it, without a variant.
# The names are fake Bedrock-like names built here (no vendor word); the fake resolver answers with the override applied, as the real one does.
BSN=console-aws-bedrock/global.fake.claude-sonnet-5-5; BHN=console-aws-bedrock/global.fake.claude-haiku-5-5
orepo="$sb/launch-override"; make_repo_copy "$orepo"
cat > "$orepo/.opencode/opencode.jsonc" << EOF
// local override: one line per agent, no variant
{ "model": "$BSN",
  "agents": {
    "build": { "model": "$BSN" },
    "worker": { "model": "$BHN" },
    "reviewer": { "model": "$BHN" },
    "title": { "model": "$BHN" },
    "summary": { "model": "$BHN" } } }
EOF
jq --arg p "${BSN%%/*}" --arg s "${BSN#*/}" --arg h "${BHN#*/}" 'map(if .id == "build" then .model = {providerID: $p, id: $s} elif (.id == "worker" or .id == "reviewer" or .id == "title" or .id == "summary") then .model = {providerID: $p, id: $h} else . end)' "$sb/agents-launch.json" > "$sb/agents-override.json"
: > "$log"; out=$(LHOME="$sb/lhome-override" FAKE_AGENTS="$sb/agents-override.json" launch_in "$orepo"); rc=$?
expect_eq "override: launch.sh exits 0 (preflight accepts the override's models)" 0 "$rc"
expect_eq "override: the session is created on the override's agents.build.model, with no variant" '{"providerID":"console-aws-bedrock","id":"global.fake.claude-sonnet-5-5"}' "$(create_model)"
expect_contains "override: the message names that model" "$out" "モデル $BSN、"
# an override without agents.build.model (here: only the top-level model) leaves the pin to opencode.jsonc
nrepo="$sb/launch-override-nobuild"; make_repo_copy "$nrepo"
printf '{ "model": "console-cloudflare-ai-gateway-custom/claude-haiku-5-5" }\n' > "$nrepo/.opencode/opencode.jsonc"
: > "$log"; LHOME="$sb/lhome-nobuild" launch_in "$nrepo" > /dev/null; rc=$?
expect_eq "override without agents.build.model: exit 0" 0 "$rc"
expect_eq "(the session is created on agents.build.model of opencode.jsonc)" '{"providerID":"console-cloudflare-ai-gateway-custom","id":"claude-sonnet-5-5","variant":"xhigh"}' "$(create_model)"
# an override that is not valid JSONC stops the launch before anything is created
xrepo="$sb/launch-override-broken"; make_repo_copy "$xrepo"
printf '{ "agents": \n' > "$xrepo/.opencode/opencode.jsonc"
: > "$log"; out=$(LHOME="$sb/lhome-broken" launch_in "$xrepo"); rc=$?
expect_eq "a broken override stops the launch (exit 1)" 1 "$rc"; expect_contains "(and says the model cannot be read)" "$out" "メインセッションのモデルを読めません"
expect_eq "(no opencode command ran)" 0 "$(wc -l < "$log" | tr -d ' ')"

# --- failure paths: nothing is created when preflight FAILs; a failing step is reported with what already exists
lh2="$sb/lhome2"; mkdir -p "$lh2/.config/opencode"; : > "$log"
out=$(cd "$lrepo" && env -u OPENCODE_SESSION_ID FAKE_GIT_PUSH=ok FAKE_DF_REPO_GIB=600 HOME="$lh2" FAKE_OC_LOG="$log" FAKE_OC_AGENTS="/dev/null" PATH="$FAKEOC:$FAKE:$PATH" bash scripts/launch.sh 2>&1); rc=$?
expect_eq "preflight FAIL stops the launch (exit 1)" 1 "$rc"
expect_contains "and says nothing was created" "$out" "何も作っていません"
expect_eq "no session.create was called" 0 "$(grep -c 'api session.create' "$log")"
expect_eq "no registry file was written" 0 "$(find "$lh2" -name 'goal-nudge.sessions' | wc -l | tr -d ' ')"
for step in session.create session.prompt session.command; do
  : > "$log"; rm -f "$lh2/.config/opencode/goal-nudge.sessions"
  out=$(cd "$lrepo" && env -u OPENCODE_SESSION_ID FAKE_GIT_PUSH=ok FAKE_DF_REPO_GIB=600 HOME="$lh2" FAKE_GH=private FAKE_OC_FAIL="$step" FAKE_OC_LOG="$log" FAKE_OC_AGENTS="$sb/agents-launch.json" PATH="$FAKEOC:$FAKE:$PATH" bash scripts/launch.sh 2>&1); rc=$?
  expect_eq "a failing $step stops the launch with exit 1" 1 "$rc"
  case "$step" in
    session.create) expect_eq "(no registry entry when no session exists)" 0 "$(find "$lh2" -name 'goal-nudge.sessions' | wc -l | tr -d ' ')" ;;
    *) expect_eq "(the session exists, so it was registered before the failing step)" 1 "$(grep -c '^ses_FAKELAUNCH0001' "$lh2/.config/opencode/goal-nudge.sessions")"; expect_contains "(and the message says the session exists)" "$out" "セッション ses_FAKELAUNCH0001 は作成済み" ;;
  esac
done
# argument handling and the in-session guard
out=$(cd "$lrepo" && OPENCODE_SESSION_ID=ses_abc HOME="$lh2" bash scripts/launch.sh 2>&1); rc=$?
expect_eq "refuses to launch from inside an opencode session" 1 "$rc"; expect_contains "(says the human runs it)" "$out" "セッションの中からは起動できません"
out=$(cd "$lrepo" && OPENCODE_SESSION_ID=ses_abc HOME="$lh2" bash scripts/launch.sh extra 2>&1); rc=$?
expect_eq "an argument is refused before anything else (launch.sh takes none)" 1 "$rc"; expect_contains "(and says so)" "$out" "引数はありません"
: > "$log"; out=$(launch_in "$lrepo" --dry-run); rc=$?
expect_eq "--dry-run does not exist any more: it is refused like any argument" 1 "$rc"
expect_contains "(and names it)" "$out" "引数はありません: --dry-run"
expect_eq "(no opencode command ran)" 0 "$(wc -l < "$log" | tr -d ' ')"
nogoal="$sb/launch-nogoal"; make_repo_copy "$nogoal"; printf 'start only\n' > "$nogoal/prompts/START.md"
: > "$log"; out=$(launch_in "$nogoal"); rc=$?
expect_eq "a START.md without a '## GOAL' section is refused" 1 "$rc"; expect_contains "(and says what is missing)" "$out" "「## GOAL」の節がない"
expect_eq "(and no opencode command ran)" 0 "$(wc -l < "$log" | tr -d ' ')"
long="$sb/launch-long"; make_repo_copy "$long"; { printf 'start\n\n## GOAL\n'; head -c 4100 /dev/zero | tr '\0' 'x'; printf '\n'; } > "$long/prompts/START.md"
: > "$log"; out=$(launch_in "$long"); rc=$?
expect_eq "a goal text over 4000 characters is refused (the /goal limit)" 1 "$rc"
expect_eq "(and no opencode command ran)" 0 "$(wc -l < "$log" | tr -d ' ')"
multi="$sb/launch-multi"; make_repo_copy "$multi"; printf 'start\n\n## GOAL\nline one\n\nline three\n\n## NEXT\nignored\n' > "$multi/prompts/START.md"
: > "$log"; LHOME="$sb/lhome-multi" launch_in "$multi" > /dev/null
goal_call=$(jq -r '.args' "$log" | grep 'api session.command')
expect_contains "a multi-line goal keeps its inner blank line" "$goal_call" '"text":"line one\n\nline three"'
expect_not_contains "and stops at the next heading" "$goal_call" "ignored"
# the launch script never touches the real registry or sessions in these tests
if [ -e "$HOME/.config/opencode/goal-nudge.sessions" ] && grep -q "FAKELAUNCH" "$HOME/.config/opencode/goal-nudge.sessions" 2> /dev/null; then fail "the REAL registry contains a fake id"; else pass "the real goal-nudge registry was not written by these tests"; fi

# ====================== env.sh
envrun() { env -i HOME="$sb/envhome" PATH="/usr/bin:/bin" ENVSH="$ROOT/scripts/env.sh" bash -c "$1"; }
envout=$(envrun '. "$ENVSH" && printf "%s\n" "$PATH" "$EM_CONFIG" "$EM_CACHE" "$TMPDIR"')
mapfile -t ev <<< "$envout"
case "${ev[0]}" in /opt/emsdk/upstream/bin:*) pass "wasm-opt's directory is first on PATH (before /usr/bin)" ;; *) fail "PATH starts with: ${ev[0]%%:*}" ;; esac
expect_contains "emscripten is on PATH" "${ev[0]}" "/opt/emsdk/upstream/emscripten"
expect_contains "node/npx directory is on PATH" "${ev[0]}" "/opt/emsdk/node/"
expect_eq "EM_CONFIG" "/opt/emsdk/.emscripten" "${ev[1]}"
expect_eq "EM_CACHE is outside /opt/emsdk" "$sb/envhome/.cache/signature-studio/em_cache" "${ev[2]}"
expect_eq "TMPDIR is under the home directory (not tmpfs /tmp)" "$sb/envhome/.cache/signature-studio/tmp" "${ev[3]}"
check "the cache and tmp directories were created" test -d "$sb/envhome/.cache/signature-studio/em_cache" -a -d "$sb/envhome/.cache/signature-studio/tmp"
expect_eq "nothing else was created under HOME (2 directories: em_cache and tmp)" "2" "$(find "$sb/envhome" -mindepth 3 -type d | wc -l | tr -d ' ')"
if [ -x /opt/emsdk/upstream/bin/wasm-opt ]; then
  expect_contains "wasm-opt resolves to version 133 after sourcing" "$(envrun '. "$ENVSH" && wasm-opt --version')" "version 133"
  expect_contains "emcc is found and runs" "$(envrun '. "$ENVSH" && emcc --version 2>&1 | head -2')" "emcc"
  expect_contains "npx is found" "$(envrun '. "$ENVSH" && command -v npx')" "/npx"
else skip "/opt/emsdk is not present: wasm-opt/emcc/npx resolution"; fi
expect_eq "sourcing twice does not duplicate PATH entries" 0 "$(envrun '. "$ENVSH"; . "$ENVSH"; printf "%s" "$PATH"' | tr ':' '\n' | sort | uniq -d | wc -l | tr -d ' ')"
expect_eq "env.sh leaves no helper variables or functions behind" "" "$(envrun '. "$ENVSH"; compgen -v | grep "^_ss_"; declare -F | grep _ss_')"

finish
