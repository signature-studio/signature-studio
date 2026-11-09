#!/usr/bin/env bash
# t02: what the permission rules allow and deny, command by command (shell-cases.tsv) and tool by tool.
# Level E2: a re-implementation of opencode's evaluation (tests/harness/lib/rules.py) over the pieces opencode's own scanner makes of each command.
# t01 shows that the rules rules.py builds equal the rules the real resolver loads. The real tool path (a session calling a tool) is NOT exercised: see README.md.
# shellcheck source=lib.sh
. "$(dirname "$0")/lib.sh"
set -o pipefail

tsv="$HERE/shell-cases.tsv"
[ -r "$tsv" ] || { fail "shell-cases.tsv is missing (generate it: bun run tests/harness/lib/gen-shell-cases.ts)"; finish; exit $?; }

# Is the .tsv still what opencode's scanner produces for shell-cases.src? Only checkable with bun and the opencode clone.
if command -v bun > /dev/null && [ -f /home/ubuntu/work/opencode/packages/core/src/shell/parse.ts ] ; then
  if (cd "$ROOT" && bun run tests/harness/lib/gen-shell-cases.ts --check > /dev/null 2>&1); then pass "shell-cases.tsv equals a fresh scan by opencode's scanner"; else fail "shell-cases.tsv is stale: bun run tests/harness/lib/gen-shell-cases.ts"; fi
else
  skip "shell-cases.tsv freshness (needs bun and the opencode source clone)"
fi

results=$(python3 - "$ROOT" "$tsv" << 'PY'
import json, sys
root, tsv = sys.argv[1:3]
sys.path.insert(0, root + "/tests/harness/lib")
import rules
cfg = rules.load(root + "/opencode.jsonc")
ok = bad = 0
limits = []
for line in open(tsv, encoding="utf-8"):
    if line.startswith("#") or not line.strip(): continue
    agent, expect, res_json, command = line.rstrip("\n").split("\t", 3)
    effect, why = rules.decide_shell(cfg, agent, json.loads(res_json))
    if expect == "LIMIT-allow":
        got = "LIMIT-allow" if effect == "allow" else effect
        if got != "LIMIT-allow":
            print("FAIL\t%s: %s -> %s (%s). A documented limit is now stopped: update README.md 'Limits' and shell-cases.src" % (agent, command, effect, why)); bad += 1
        else:
            limits.append(agent + ": " + command)
            ok += 1
        continue
    if effect == expect:
        ok += 1
    else:
        print("FAIL\t%s: expected %s, got %s: %s  [%s]" % (agent, expect, effect, command, why)); bad += 1
print("SUMMARY\t%d\t%d\t%d" % (ok, bad, len(limits)))
PY
)
while IFS=$'\t' read -r kind a b c; do
  case "$kind" in
    FAIL) fail "shell: $a" ;;
    SUMMARY) if [ "$b" = 0 ]; then pass "shell cases: $a agree with the rules ($c of them are documented LIMIT-allow forms)"; fi ;;
  esac
done <<< "$results"

# --- non-shell tools: one line per case "agent action resource expect"
python3 - "$ROOT" << 'PY' > "${TMPDIR:-/tmp}/t02-nonshell.$$"
import os, sys
sys.path.insert(0, sys.argv[1] + "/tests/harness/lib")
import rules
cfg = rules.load(sys.argv[1] + "/opencode.jsonc")
H = os.path.expanduser("~")
cases = [
  # (agent, action, resource, expect)
  ("build", "edit", "src/a.ts", "deny"), ("build", "edit", "README.md", "deny"), ("build", "edit", "docs/STATE.md", "deny"),
  ("build", "read", "src/a.ts", "allow"), ("build", "read", "AGENTS.md", "allow"),
  ("build", "read", ".env", "deny"), ("build", "read", "app/.env.local", "deny"), ("build", "read", ".env.example", "allow"),
  ("build", "read", H + "/.config/opencode/opencode.jsonc", "deny"), ("build", "read", H + "/.config/opencode/goal-nudge.sessions", "deny"),
  ("build", "read", H + "/.local/share/opencode/auth.json", "deny"), ("build", "read", H + "/.local/share/opencode/opencode.db", "deny"),
  ("build", "read", H + "/.local/share/botid/identities/x/gh/hosts.yml", "deny"), ("build", "read", H + "/.config/botid/config.json", "deny"),
  ("build", "read", H + "/.local/share/opencode/tool-output/tool_abc", "allow"), ("build", "read", H + "/.local/share/opencode/shell/p/s.out", "allow"),
  ("build", "read", H + "/work/signature-studio-old/README.md", "allow"),
  ("build", "glob", "**/*.ts", "allow"), ("build", "grep", "foo", "allow"), ("build", "skill", "opencode", "allow"),
  ("build", "external_directory", H + "/work/other/*", "allow"),
  ("build", "external_directory", H + "/.config/opencode/*", "deny"), ("build", "external_directory", H + "/.local/share/opencode/*", "deny"),
  ("build", "external_directory", H + "/.config/botid/*", "deny"), ("build", "external_directory", H + "/.local/share/botid/*", "deny"),
  ("build", "external_directory", H + "/.local/share/opencode/tool-output/*", "allow"),
  ("build", "subagent", "worker", "allow"), ("build", "subagent", "reviewer", "allow"),
  ("build", "subagent", "general", "deny"), ("build", "subagent", "explore", "deny"), ("build", "subagent", "plan", "deny"), ("build", "subagent", "build", "deny"),
  ("build", "webfetch", "https://example.com", "deny"), ("build", "websearch", "q", "deny"), ("build", "execute", "*", "deny"),
  ("build", "question", "*", "deny"), ("build", "write", "x", "deny"), ("build", "patch", "x", "deny"),
  ("build", "opencode_read_mcp_resource", "s:u", "deny"), ("build", "experimental_image_generate", "x/y", "deny"),
  ("worker", "edit", "src/a.ts", "allow"), ("worker", "edit", "lift/crates/x/src/lib.rs", "allow"), ("worker", "edit", "docs/STATE.md", "allow"),
  ("worker", "edit", "opencode.jsonc", "deny"), ("worker", "edit", H + "/work/signature-studio/opencode.jsonc", "deny"), ("worker", "edit", "opencode.json", "deny"),
  ("worker", "edit", ".opencode/plugins/harness-guard.ts", "deny"), ("worker", "edit", H + "/work/signature-studio/.opencode/plugins/x.ts", "deny"),
  ("worker", "edit", ".githooks/pre-commit", "deny"), ("worker", "edit", H + "/work/signature-studio/.githooks/pre-push", "deny"),
  ("worker", "read", "src/a.ts", "allow"), ("worker", "read", ".env", "deny"), ("worker", "read", ".env.example", "allow"),
  ("worker", "read", H + "/.config/opencode/opencode.jsonc", "deny"), ("worker", "read", H + "/.local/share/botid/identities/x", "deny"),
  ("worker", "read", H + "/.local/share/opencode/tool-output/t", "allow"),
  ("worker", "subagent", "worker", "deny"), ("worker", "subagent", "reviewer", "deny"), ("worker", "subagent", "general", "deny"),
  ("worker", "question", "*", "deny"), ("worker", "execute", "*", "deny"),
  ("worker", "webfetch", "https://example.com", "allow"), ("worker", "websearch", "q", "allow"),
  ("worker", "glob", "**/*", "allow"), ("worker", "grep", "x", "allow"), ("worker", "skill", "opencode", "allow"),
  ("worker", "external_directory", "/tmp/opencode/*", "allow"), ("worker", "external_directory", H + "/.config/opencode/*", "deny"),
  ("worker", "external_directory", H + "/.config/botid/*", "deny"),
  ("reviewer", "edit", "src/a.ts", "deny"), ("reviewer", "write", "x", "deny"), ("reviewer", "patch", "x", "deny"),
  ("reviewer", "read", "src/a.ts", "allow"), ("reviewer", "read", ".env", "deny"), ("reviewer", "grep", "x", "allow"), ("reviewer", "glob", "*", "allow"),
  ("reviewer", "subagent", "worker", "deny"), ("reviewer", "execute", "*", "deny"), ("reviewer", "question", "*", "deny"),
]
for a, act, res, exp in cases:
    # `write` and `patch` are checked as `edit` by opencode (one action for the three tools)
    action = "edit" if act in ("write", "patch") else act
    got, why = rules.decide(cfg, a, action, res)
    print("\t".join([a, act, res, exp, got, why]))
PY
n=0; bad=0
while IFS=$'\t' read -r a act res exp got why; do
  n=$((n + 1))
  if [ "$exp" != "$got" ]; then bad=$((bad + 1)); fail "$a $act $res: expected $exp, got $got [$why]"; fi
done < "${TMPDIR:-/tmp}/t02-nonshell.$$"
rm -f "${TMPDIR:-/tmp}/t02-nonshell.$$"
[ "$bad" -eq 0 ] && pass "non-shell cases: $n agree with the rules (build cannot edit/write/patch, worker cannot touch the harness files, reviewer is read-only, credential paths are denied for all)"

# --- `ask` must not be left anywhere in the effective rules of the three agents (nobody is there to answer; cli.json auto-accepts it)
asks=$(python3 - "$ROOT" << 'PY'
import sys
sys.path.insert(0, sys.argv[1] + "/tests/harness/lib")
import rules
cfg = rules.load(sys.argv[1] + "/opencode.jsonc")
print(sum(1 for a in ("build", "worker", "reviewer") for r in rules.config_rules(cfg, a) if r["effect"] == "ask"))
PY
)
expect_eq "no rule with effect 'ask' in opencode.jsonc" 0 "$asks"

# --- the main session's allowlist: every shell rule that ALLOWS must be one the documentation (AGENTS.md section 1) lists. A new allow needs a conscious edit here.
allows=$(python3 - "$ROOT" << 'PY'
import sys
sys.path.insert(0, sys.argv[1] + "/tests/harness/lib")
import rules
cfg = rules.load(sys.argv[1] + "/opencode.jsonc")
for r in cfg["agents"]["build"]["permissions"]:
    if r["effect"] == "allow" and r["action"] in ("shell", "edit", "write", "patch", "execute", "webfetch", "websearch", "question"):
        print(r["action"] + " " + r["resource"])
PY
)
expected_allows="shell git status *
shell git diff *
shell git log *
shell git show *
shell git -C * status *
shell git -C * diff *
shell git -C * log *
shell ls *
shell stat *
shell wc *
shell opencode api session.active
shell opencode api session.get *
shell opencode api session.message.list *
shell opencode api session.synthetic *
shell opencode api POST /api/rpc/opencode.goal/get *
shell opencode api session.command --param sessionID=* --data '{\"name\":\"goal\",\"text\":\"resume\"}'
shell ~/.agents/skills/opencode-send-message/scripts/oc-msg.sh *
shell \$HOME/.agents/skills/opencode-send-message/scripts/oc-msg.sh *
shell /home/*/.agents/skills/opencode-send-message/scripts/oc-msg.sh *
shell scripts/preflight.sh
shell ./scripts/preflight.sh
shell scripts/audit-session.sh *
shell ./scripts/audit-session.sh *"
expect_eq "the main session's shell allowlist is exactly the documented one" "$expected_allows" "$allows"

finish
