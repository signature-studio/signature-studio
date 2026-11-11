#!/usr/bin/env bash
# t08: scripts/set-model-profile.sh, scripts/launch.sh, scripts/env.sh.
#  - set-model-profile.sh runs in a repository COPY; its post-check (`preflight` row `agents`) is given a real resolver answer for the rewritten file (isolated sandbox).
#  - launch.sh runs ONLY with a fake opencode (tests/harness/fakebin-opencode) and a redirected HOME: no real session is created, no registry of the real machine is touched.
#  - env.sh is sourced in a subshell.
# shellcheck disable=SC2016  # the strings given to envrun are code for the inner bash: they must not expand in this shell
# shellcheck source=lib.sh
. "$(dirname "$0")/lib.sh"
set -o pipefail

sb=$(new_sandbox t08)
FAKE="$HERE/fakebin"; FAKEOC="$HERE/fakebin-opencode"
SON="console-cloudflare-ai-gateway-custom/claude-sonnet-5-5"; HAI="console-cloudflare-ai-gateway-custom/claude-haiku-5-5"
models() { grep -oE '"model": "[^"]*"' "$1" | sed -E 's/"model": "([^"]*)"/\1/' | paste -sd' ' -; }   # file order: top, build, title, summary, worker, reviewer

# ====================== set-model-profile.sh
repo="$sb/smp"; make_repo_copy "$repo"; git -C "$repo" remote add origin "https://github.com/fake-owner/fake-repo.git"
orig_models=$(models "$repo/opencode.jsonc")
expect_eq "start: split profile (file order: top, build, title, summary, worker, reviewer)" "$SON $SON#xhigh $HAI#low $HAI#low $HAI#high $HAI#high" "$orig_models"
orig_hash=$(md5sum "$repo/opencode.jsonc" | cut -d' ' -f1)
comments_before=$(grep -cE '^\s*//' "$repo/opencode.jsonc")

smp() { (cd "$repo" && env -u OPENCODE_SESSION_ID bash scripts/set-model-profile.sh "$@" 2>&1); }

out=$(smp --dry-run haiku); rc=$?
expect_eq "--dry-run exits 0" 0 "$rc"
expect_contains "--dry-run shows what would change" "$out" "次の内容に切り替えます"
expect_contains "--dry-run says nothing is written" "$out" "何も書き換えていません"
expect_eq "--dry-run left the file byte-identical" "$orig_hash" "$(md5sum "$repo/opencode.jsonc" | cut -d' ' -f1)"
out=$(smp --dry-run split); rc=$?
expect_eq "--dry-run split on a split file: exits 0" 0 "$rc"; expect_contains "(already the split profile)" "$out" "何も変えません"

# a fake opencode that answers `debug agents` with the REAL resolver's output for the file in the repo copy, and `reload` with success.
# To get real output for the rewritten file we resolve it in the isolated sandbox first (below), then point FAKE_OC_AGENTS at it.
resolve_real() { # file -> JSON on stdout
  local d="$sb/res-$$-$RANDOM"; mkdir -p "$d/proj"; cp "$1" "$d/proj/opencode.jsonc"
  (
    export XDG_CONFIG_HOME="$d/xdg/config" XDG_DATA_HOME="$d/xdg/data" XDG_STATE_HOME="$d/xdg/state" XDG_CACHE_HOME="$d/xdg/cache"
    mkdir -p "$XDG_CONFIG_HOME" "$XDG_DATA_HOME" "$XDG_STATE_HOME" "$XDG_CACHE_HOME"
    cd "$d/proj" && opencode --standalone debug agents 2> /dev/null
    opencode service stop > /dev/null 2>&1
  )
}
# preflight (run by set-model-profile.sh) calls `opencode debug agents`; this wrapper returns the real resolution of the CURRENT file in the repo copy.
mkdir -p "$sb/wrapbin"
cat > "$sb/wrapbin/opencode" << EOF
#!/usr/bin/env bash
case "\$1" in
  reload) echo "Configuration reloaded"; exit 0 ;;
  debug) [ "\$2" = agents ] && { cat "$sb/current-agents.json"; exit 0; } ;;
  api) echo '{}'; exit 0 ;;
esac
exit 64
EOF
chmod +x "$sb/wrapbin/opencode"
if command -v opencode > /dev/null; then
  # --- split -> haiku, post-check sees the real resolution of the rewritten file
  cp "$repo/opencode.jsonc" "$sb/before.jsonc"
  # First a post-check that finds no agents: the script must fail and put the original back.
  printf '%s' "[]" > "$sb/current-agents.json"
  out=$(cd "$repo" && env -u OPENCODE_SESSION_ID FAKE_GIT_PUSH=ok PATH="$sb/wrapbin:$FAKE:$PATH" bash scripts/set-model-profile.sh haiku 2>&1); rc=$?
  expect_eq "with a post-check that finds no agents the script FAILS (exit 1)" 1 "$rc"
  expect_contains "and says the post-check was not OK" "$out" "切り替えを取り消しました"
  expect_eq "and the ORIGINAL file is back, byte for byte" "$orig_hash" "$(md5sum "$repo/opencode.jsonc" | cut -d' ' -f1)"
  expect_contains "and says it put the original back" "$out" "元の状態に戻しました"

  # build the haiku file with the script's rewrite (python part), resolve it for real, then let the script run with that answer
  haiku_file="$sb/haiku.jsonc"
  python3 - "$repo/opencode.jsonc" "$haiku_file" << 'PY'
import re, sys
s = open(sys.argv[1]).read().split("\n")
HAI = "console-cloudflare-ai-gateway-custom/claude-haiku-5-5"
agent = None; in_agents = False
for k, line in enumerate(s):
    if re.match(r'^  "agents": \{', line): in_agents = True; continue
    m = re.match(r'^    "(\w+)": \{', line)
    if in_agents and m: agent = m.group(1); continue
    if in_agents and re.match(r'^    \},?\s*$', line): agent = None; continue
    if (not in_agents) and re.match(r'^  "model": "', line): s[k] = '  "model": "%s",' % HAI
    if in_agents and agent == "build" and re.match(r'^      "model": "', line): s[k] = '      "model": "%s#high",' % HAI
open(sys.argv[2], "w").write("\n".join(s))
PY
  resolve_real "$haiku_file" > "$sb/current-agents.json"
  out=$(cd "$repo" && env -u OPENCODE_SESSION_ID FAKE_GIT_PUSH=ok PATH="$sb/wrapbin:$FAKE:$PATH" bash scripts/set-model-profile.sh haiku 2>&1); rc=$?
  expect_eq "split -> haiku: exits 0 when the real resolution of the new file matches" 0 "$rc"
  expect_contains "the switch is reported in plain words" "$out" "切り替えました"
  expect_eq "after haiku: every model is Haiku (file order: top, build, title, summary, worker, reviewer)" "$HAI $HAI#high $HAI#low $HAI#low $HAI#high $HAI#high" "$(models "$repo/opencode.jsonc")"
  expect_eq "the haiku file is the one predicted (only the model lines changed)" "$(md5sum < "$haiku_file")" "$(md5sum < "$repo/opencode.jsonc")"
  expect_eq "only model lines differ from the original (diff has exactly 2 changed lines: top and build)" 2 "$(diff "$sb/before.jsonc" "$repo/opencode.jsonc" 2> /dev/null | grep -c '^>' || true)"
  expect_eq "comments are kept (same number of // lines)" "$comments_before" "$(grep -cE '^\s*//' "$repo/opencode.jsonc")"
  expect_eq "agents and rules are untouched: same structure apart from model values" "$(sed -E 's/"model": "[^"]*"/"model": "M"/' "$sb/before.jsonc" | md5sum)" "$(sed -E 's/"model": "[^"]*"/"model": "M"/' "$repo/opencode.jsonc" | md5sum)"
  if python3 "$HERE/lib/rules.py" "$repo/opencode.jsonc" build read x > /dev/null 2>&1; then pass "the rewritten file is valid JSONC"; else fail "the rewritten file is not valid JSONC"; fi
  # permissions identical in both profiles (the structure is the same: "deny, gate and marker are the same")
  same=$(python3 - "$sb/before.jsonc" "$repo/opencode.jsonc" "$ROOT" << 'PY'
import sys
sys.path.insert(0, sys.argv[3] + "/tests/harness/lib")
import rules
a, b = rules.load(sys.argv[1]), rules.load(sys.argv[2])
same = a["policies"] == b["policies"] and a["permissions"] == b["permissions"] and all(a["agents"][k].get("permissions") == b["agents"][k].get("permissions") for k in a["agents"])
print("same" if same else "DIFFERENT")
PY
)
  expect_eq "profile haiku keeps every policy and every permission rule identical" same "$same"

  # --- haiku -> split
  resolve_real "$sb/before.jsonc" > "$sb/current-agents.json"
  out=$(cd "$repo" && env -u OPENCODE_SESSION_ID FAKE_GIT_PUSH=ok PATH="$sb/wrapbin:$FAKE:$PATH" bash scripts/set-model-profile.sh split 2>&1); rc=$?
  expect_eq "haiku -> split: exits 0" 0 "$rc"
  expect_eq "back to the original file, byte for byte" "$orig_hash" "$(md5sum "$repo/opencode.jsonc" | cut -d' ' -f1)"
  out=$(cd "$repo" && env -u OPENCODE_SESSION_ID FAKE_GIT_PUSH=ok PATH="$sb/wrapbin:$FAKE:$PATH" bash scripts/set-model-profile.sh split 2>&1); rc=$?
  expect_eq "split on split is a no-op, exit 0 (idempotent)" 0 "$rc"
else
  skip "opencode missing: set-model-profile post-check tests"
fi

# --- refuses inside a session
out=$(cd "$repo" && OPENCODE_SESSION_ID=ses_abc bash scripts/set-model-profile.sh haiku 2>&1); rc=$?
expect_eq "refuses to write inside an opencode session" 2 "$rc"; expect_contains "and says the owner runs it" "$out" "セッションの中からは切り替えられません"
expect_eq "(file untouched)" "$orig_hash" "$(md5sum "$repo/opencode.jsonc" | cut -d' ' -f1)"
out=$(cd "$repo" && OPENCODE_SESSION_ID=ses_abc bash scripts/set-model-profile.sh --dry-run haiku 2>&1); expect_eq "--dry-run is allowed inside a session" 0 "$?"
# --- bad arguments and bad files
smp bogus > /dev/null; expect_eq "unknown profile exits 2" 2 "$?"
smp > /dev/null; expect_eq "no profile exits 2" 2 "$?"
smp split haiku > /dev/null; expect_eq "two profiles exit 2" 2 "$?"
smp --set haiku > /dev/null; expect_eq "an unknown option exits 2 (no options beyond --dry-run)" 2 "$?"
broken="$sb/smp-broken"; make_repo_copy "$broken"
sed -i '0,/"resource": "sleep \*"/s//resource": "sleep *"/' "$broken/opencode.jsonc"
bh=$(md5sum "$broken/opencode.jsonc" | cut -d' ' -f1)
out=$(cd "$broken" && env -u OPENCODE_SESSION_ID bash scripts/set-model-profile.sh haiku 2>&1); rc=$?
expect_eq "a file that is already invalid JSONC is refused (exit 2) before anything is written" 2 "$rc"
expect_contains "and says it is not valid JSONC" "$out" "設定ファイルの形式が正しくありません"
expect_eq "(and the file is untouched)" "$bh" "$(md5sum "$broken/opencode.jsonc" | cut -d' ' -f1)"
odd="$sb/smp-odd"; make_repo_copy "$odd"; sed -i 's/^  "model": /  "mdl": /' "$odd/opencode.jsonc"
oh=$(md5sum "$odd/opencode.jsonc" | cut -d' ' -f1)
out=$(cd "$odd" && env -u OPENCODE_SESSION_ID bash scripts/set-model-profile.sh haiku 2>&1); rc=$?
expect_eq "a layout it does not recognise (a model line is missing) exits 2, untouched" 2 "$rc"
expect_contains "and says which model lines it expected" "$out" "6 か所"
expect_eq "(untouched)" "$oh" "$(md5sum "$odd/opencode.jsonc" | cut -d' ' -f1)"
expect_eq "the script reads no environment variable except OPENCODE_SESSION_ID and TMPDIR" "" "$(grep -v '^ *#' "$ROOT/scripts/set-model-profile.sh" | grep -oE '\$\{?[A-Z][A-Z_]{3,}[}:-]?' | tr -d '${}:-' | sort -u | grep -vE '^(OPENCODE_SESSION_ID|TMPDIR|BASH_SOURCE)$' | tr '\n' ' ')"

# ====================== launch.sh (fake opencode only; HOME redirected)
lrepo="$sb/launch"; make_repo_copy "$lrepo"
lhome="$sb/lhome"; mkdir -p "$lhome/.config/opencode"
log="$sb/launch.log"
launch() { (cd "$lrepo" && env -u OPENCODE_SESSION_ID FAKE_GIT_PUSH=ok FAKE_DF_REPO_GIB=600 HOME="$lhome" FAKE_OC_LOG="$log" FAKE_OC_AGENTS="$sb/agents-launch.json" PATH="$FAKEOC:$FAKE:$PATH" bash scripts/launch.sh "$@" 2>&1); }
# preflight (the first step) needs a resolver answer for the good file
resolve_real "$ROOT/opencode.jsonc" > "$sb/agents-launch.json"

: > "$log"
out=$(launch --dry-run); rc=$?
expect_eq "launch.sh --dry-run exits 0" 0 "$rc"
expect_eq "--dry-run runs no opencode command at all (not even preflight)" 0 "$(wc -l < "$log" | tr -d ' ')"
expect_contains "--dry-run shows session.create with agent build" "$out" 'session.create --data'
expect_contains "--dry-run shows the model copied from opencode.jsonc" "$out" '"providerID":"console-cloudflare-ai-gateway-custom","id":"claude-sonnet-5-5","variant":"xhigh"'
expect_contains "--dry-run shows the repository directory" "$out" "\"location\":{\"directory\":\"$lrepo\"}"
expect_contains "--dry-run shows the registry step" "$out" "goal-nudge.sessions"
expect_contains "--dry-run shows that session.prompt is the human's launch" "$out" "session.prompt を使うのは、この起動の 1 回だけ"
expect_contains "--dry-run shows the goal text from START.md" "$out" "make it work"
expect_eq "--dry-run created no registry file" 0 "$(find "$lhome" -name 'goal-nudge.sessions' | wc -l | tr -d ' ')"

# model follows opencode.jsonc (profile haiku -> the session is created on Haiku)
hrepo="$sb/launch-haiku"; make_repo_copy "$hrepo"
sed -i '0,/"model": "console-cloudflare-ai-gateway-custom\/claude-sonnet-5-5#xhigh"/s//"model": "console-cloudflare-ai-gateway-custom\/claude-haiku-5-5#high"/' "$hrepo/opencode.jsonc"
out=$(cd "$hrepo" && env -u OPENCODE_SESSION_ID HOME="$lhome" PATH="$FAKEOC:$FAKE:$PATH" bash scripts/launch.sh --dry-run 2>&1)
expect_contains "the model in session.create follows agents.build.model (Haiku profile)" "$out" '"id":"claude-haiku-5-5","variant":"high"'

# the real launch against the fake
: > "$log"
out=$(cd "$lrepo" && env -u OPENCODE_SESSION_ID FAKE_GIT_PUSH=ok FAKE_DF_REPO_GIB=600 HOME="$lhome" FAKE_GH=private FAKE_OC_LOG="$log" FAKE_OC_AGENTS="$sb/agents-launch.json" PATH="$FAKEOC:$FAKE:$PATH" bash scripts/launch.sh 2>&1); rc=$?
if [ "$rc" -eq 0 ]; then
  pass "launch.sh (against the fake) exits 0"
  expect_contains "reports the new session id" "$out" "セッション ses_FAKELAUNCH0001 を作成しました"
  calls=$(jq -r '.args' "$log")
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
out=$(cd "$lrepo" && OPENCODE_SESSION_ID=ses_abc HOME="$lh2" bash scripts/launch.sh --dry-run 2>&1); expect_eq "--dry-run is allowed inside a session" 0 "$?"
launch --nothing > /dev/null; expect_eq "an unknown option exits 1 (no options beyond --dry-run)" 1 "$?"
nogoal="$sb/launch-nogoal"; make_repo_copy "$nogoal"; printf 'start only\n' > "$nogoal/prompts/START.md"
out=$(cd "$nogoal" && env -u OPENCODE_SESSION_ID HOME="$lh2" PATH="$FAKEOC:$PATH" bash scripts/launch.sh --dry-run 2>&1); rc=$?
expect_eq "a START.md without a '## GOAL' section is refused" 1 "$rc"; expect_contains "(and says what is missing)" "$out" "「## GOAL」の節がない"
long="$sb/launch-long"; make_repo_copy "$long"; { printf 'start\n\n## GOAL\n'; head -c 4100 /dev/zero | tr '\0' 'x'; printf '\n'; } > "$long/prompts/START.md"
out=$(cd "$long" && env -u OPENCODE_SESSION_ID HOME="$lh2" PATH="$FAKEOC:$PATH" bash scripts/launch.sh --dry-run 2>&1); rc=$?
expect_eq "a goal text over 4000 characters is refused (the /goal limit)" 1 "$rc"
multi="$sb/launch-multi"; make_repo_copy "$multi"; printf 'start\n\n## GOAL\nline one\n\nline three\n\n## NEXT\nignored\n' > "$multi/prompts/START.md"
out=$(cd "$multi" && env -u OPENCODE_SESSION_ID HOME="$lh2" PATH="$FAKEOC:$PATH" bash scripts/launch.sh --dry-run 2>&1)
expect_contains "a multi-line goal keeps its inner blank line" "$out" "#   line one
#   
#   line three"
expect_not_contains "and stops at the next heading" "$out" "ignored"
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
