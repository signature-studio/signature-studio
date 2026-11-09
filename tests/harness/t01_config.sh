#!/usr/bin/env bash
# t01: opencode.jsonc is valid, contains no vendor name, resolves to the expected agents, and the fail-open detection works.
# Uses an isolated opencode (XDG_* in a sandbox): no real session, no model call, the real service is not touched.
# Fail-open (R3 B.1): with ONE quote missing, opencode silently drops the whole file. scripts/preflight.sh must turn that into a FAIL.
# shellcheck source=lib.sh
. "$(dirname "$0")/lib.sh"
set -o pipefail

# --- the file itself
if python3 "$HERE/lib/rules.py" "$ROOT/opencode.jsonc" build read x > /dev/null 2>&1; then pass "opencode.jsonc parses as JSONC"; else fail "opencode.jsonc does not parse as JSONC"; fi
v="anthr"; v2="opic"
if grep -rqiI "$v$v2" "$ROOT/opencode.jsonc" "$ROOT/.opencode" "$ROOT/.githooks" "$ROOT/scripts" "$ROOT/tests" "$ROOT/.gitignore" 2> /dev/null; then fail "a vendor name is in the harness files"; else pass "no vendor name in opencode.jsonc, plugins, hooks, scripts, tests, .gitignore"; fi
n=$(grep -cE '^ {2}"model": "|^ {6}"model": "' "$ROOT/opencode.jsonc")
expect_eq "own-line model entries that set-model-profile.sh rewrites (top, build, worker, reviewer; title and summary are one-liners and stay Haiku)" 4 "$n"

# --- real opencode, isolated
if ! command -v opencode > /dev/null; then skip "opencode not installed: resolved-agent tests"; finish; exit $?; fi
sb=$(new_sandbox t01)
export XDG_CONFIG_HOME="$sb/xdg/config" XDG_DATA_HOME="$sb/xdg/data" XDG_STATE_HOME="$sb/xdg/state" XDG_CACHE_HOME="$sb/xdg/cache"
mkdir -p "$XDG_CONFIG_HOME" "$XDG_DATA_HOME" "$XDG_STATE_HOME" "$XDG_CACHE_HOME"
trap 'opencode service stop > /dev/null 2>&1' EXIT   # stops only the isolated server these commands started (XDG_* above)

resolve() { (cd "$1" && opencode --standalone debug agents 2> /dev/null); }

good="$sb/good"; mkdir -p "$good"; cp "$ROOT/opencode.jsonc" "$good/"
agents=$(resolve "$good")
printf '%s' "$agents" > "$sb/agents.json"
expect_eq "agent ids from the real resolver" '["build","compaction","reviewer","summary","title","worker"]' "$(jq -c '[.[].id] | sort' <<< "$agents" 2> /dev/null)"
for a in general explore plan; do
  expect_eq "agent '$a' does not exist (count)" 0 "$(jq --arg a "$a" '[.[] | select(.id==$a)] | length' <<< "$agents" 2> /dev/null)"
done
m() { jq -r --arg a "$1" '.[] | select(.id==$a) | .model.providerID + "/" + .model.id + "#" + .model.variant' <<< "$agents"; }
expect_eq "build model" "console-cloudflare-ai-gateway-custom/claude-sonnet-5-5#xhigh" "$(m build)"
expect_eq "worker model" "console-cloudflare-ai-gateway-custom/claude-haiku-5-5#high" "$(m worker)"
expect_eq "reviewer model" "console-cloudflare-ai-gateway-custom/claude-haiku-5-5#high" "$(m reviewer)"
expect_eq "title model" "console-cloudflare-ai-gateway-custom/claude-haiku-5-5#low" "$(m title)"
expect_eq "summary model" "console-cloudflare-ai-gateway-custom/claude-haiku-5-5#low" "$(m summary)"
expect_eq "compaction has no pinned model (by design)" "null" "$(jq -r '.[] | select(.id=="compaction") | .model // "null"' <<< "$agents")"

# --- the rules the real resolver produced contain, as one block, the rules rules.py builds from the file (so t02 tests what opencode loads)
cmp_out=$(python3 - "$ROOT" "$sb/agents.json" << 'PY'
import json, sys
root = sys.argv[1]
sys.path.insert(0, root + "/tests/harness/lib")
import rules
cfg = rules.load(root + "/opencode.jsonc")
real = json.load(open(sys.argv[2]))
bad = []
for a in ("build", "worker", "reviewer"):
    flat = [(r["action"], r["resource"], r["effect"]) for x in real if x["id"] == a for r in x["permissions"]]
    want = [(r["action"], r["resource"], r["effect"]) for r in rules.config_rules(cfg, a)]
    if not any(flat[i:i + len(want)] == want for i in range(len(flat) - len(want) + 1)):
        bad.append(a)
print("MISMATCH:" + ",".join(bad) if bad else "same")
PY
)
expect_eq "rules from opencode.jsonc = the block in the real resolver's output (build, worker, reviewer)" "same" "$cmp_out"

# --- fail-open: remove ONE quote from a deny rule; opencode drops the file silently ...
bad="$sb/bad"; mkdir -p "$bad"
sed '0,/"resource": "sleep \*"/s//resource": "sleep *"/' "$ROOT/opencode.jsonc" > "$bad/opencode.jsonc"
expect_eq "the broken copy differs from the good one in exactly one line" 1 "$(diff "$ROOT/opencode.jsonc" "$bad/opencode.jsonc" | grep -c '^>')"
bad_agents=$(resolve "$bad")
expect_contains "(fact, R3 B.1) a broken file silently falls back to the built-in agents" "$(jq -c '[.[].id] | sort' <<< "$bad_agents" 2> /dev/null)" '"general"'
expect_eq "(fact) build has no model when the file is dropped" "null" "$(jq -r '.[] | select(.id=="build") | .model // "null"' <<< "$bad_agents")"

# ... and preflight turns that into FAIL. preflight runs `opencode debug agents` in its own repository root, so run it on a repository copy.
run_preflight() { (cd "$1" && PATH="$HERE/fakebin:$PATH" bash scripts/preflight.sh 2> /dev/null); }
repo="$sb/repo-broken"; make_repo_copy "$repo"
cp "$bad/opencode.jsonc" "$repo/opencode.jsonc"
out=$(run_preflight "$repo"); rc=$?
row=$(awk '$1 == "agents"' <<< "$out")
expect_contains "preflight 'agents' row is FAIL for the broken file" "$row" " FAIL "
expect_contains "the FAIL row names the problem" "$row" "agent ids are"
expect_eq "preflight exits 1 for the broken file" 1 "$rc"

repo2="$sb/repo-good"; make_repo_copy "$repo2"
out2=$(run_preflight "$repo2"); rc2=$?
expect_contains "preflight 'agents' row is OK for the good file" "$(awk '$1 == "agents"' <<< "$out2")" " OK "
echo "      (preflight rc on the good copy: $rc2; other rows there depend on this machine: $(awk '$2 == "FAIL" {print $1}' <<< "$out2" | paste -sd, -))"

# a rule removed from the file (not a syntax error) is caught too: delete the `* deny` of build
repo3="$sb/repo-nodeny"; make_repo_copy "$repo3"
python3 - "$repo3/opencode.jsonc" << 'PY'
import sys
p = sys.argv[1]
s = open(p).read()
old = '        { "action": "*", "resource": "*", "effect": "deny" },\n'
assert s.count(old) == 1
open(p, "w").write(s.replace(old, ""))
PY
out3=$(run_preflight "$repo3")
expect_contains "preflight 'agents' row is FAIL when build lost its '* deny'" "$(awk '$1 == "agents"' <<< "$out3")" "no \`* deny\` rule"

finish
