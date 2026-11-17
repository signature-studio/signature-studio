#!/usr/bin/env bash
# t11: the CI workflow (.github/workflows/agent.yml) imports the login and writes the model config in run steps (decision 0402).
# It checks the step order, the Variable OPENCODE_MODEL reference, and that the generated config is valid JSON (the step's text is read with python3 and yaml; the Variable is replaced by a dummy value).
# The CI files must not name the former provider Variable, and must not contain a vendor name (the names are built here, so this file does not contain them either).
# shellcheck source=lib.sh
. "$(dirname "$0")/lib.sh"
set -o pipefail

v="anthr""opic"
old="OPENCODE""_PROVIDER"
wf="$ROOT/.github/workflows/agent.yml"

impline=$(grep -n 'opencode auth import --standalone' "$wf" | head -n 1 | cut -d: -f1)
cfgline=$(grep -n 'cat > .opencode/opencode.jsonc' "$wf" | head -n 1 | cut -d: -f1)
order_ok=no; [ -n "$impline" ] && [ -n "$cfgline" ] && [ "$impline" -lt "$cfgline" ] && order_ok=yes
expect_eq "agent.yml: the import (--standalone) comes before the config" yes "$order_ok"
expect_eq "agent.yml: the model comes from the Variable OPENCODE_MODEL (six references: model and five agents)" 6 "$(grep -c 'vars.OPENCODE_MODEL' "$wf")"
expect_eq "agent.yml: no other vars. reference" 6 "$(grep -c 'vars\.' "$wf")"
expect_eq "the former provider Variable is not named by .github, ci, scripts or tests" 0 \
  "$(grep -rc -e "$old" "$ROOT/.github" "$ROOT/ci" "$ROOT/scripts" "$ROOT/tests" | awk -F: '{ s += $NF } END { print s + 0 }')"
expect_eq "the workflow scripts are gone (ci/agents: no script, the steps are in agent.yml)" 0 "$(find "$ROOT/ci/agents" -type f 2> /dev/null | wc -l)"
expect_eq ".github/workflows/agent.yml has no vendor name" 0 "$(grep -ci "$v" "$wf")"

# The config step's text, with the Variable replaced by a dummy, is valid JSON and has the worker as the primary agent.
cfgtxt=$(python3 - "$wf" << 'EOF'
import sys, yaml
steps = yaml.safe_load(open(sys.argv[1]))["jobs"]["run"]["steps"]
run = next(s["run"] for s in steps if s.get("name") == "実行用のモデルの設定を書く")
body = run.split("cat > .opencode/opencode.jsonc << EOF\n", 1)[1].split("\nEOF", 1)[0]
print(body.replace("${{ vars.OPENCODE_MODEL }}", "dummy-provider/dummy-model"))
EOF
)
rc=0; printf '%s\n' "$cfgtxt" | jq -e . > /dev/null || rc=$?
expect_eq "the generated config is valid JSON (the Variable replaced by a dummy)" 0 "$rc"
expect_eq "the worker is the default agent, and mode all" "worker all" \
  "$(printf '%s\n' "$cfgtxt" | jq -r '[.default_agent, .agents.worker.mode] | join(" ")')"
expect_eq "the five agents use the one model" "5" \
  "$(printf '%s\n' "$cfgtxt" | jq -r '[.agents.build.model, .agents.worker.model, .agents.reviewer.model, .agents.title.model, .agents.summary.model] | map(select(. == "dummy-provider/dummy-model")) | length')"

finish
