#!/usr/bin/env bash
# t11: the CI runner's model is the Variable OPENCODE_MODEL (decision 0384).
# ci/agents/runner-config.sh: an empty Variable writes nothing and exits 0 (the stop is check-agents', after the import); a value -> the five agents use it.
# ci/agents/check-agents.sh: an empty Variable stops with the Issue 0006 message; the value must end with claude-haiku-5-5 and carry no #variant; the agents must use exactly that value (a fake `opencode debug agents` answers).
# .github/workflows/agent.yml: the config step runs before the import step (the server that the import starts reads the file; decision 0384), and the two steps get the Variable.
# The model ids are dummies (dummy-provider/...), so no vendor name is written here; the three CI files must not contain it. No real opencode, service or model call.
# shellcheck source=lib.sh
. "$(dirname "$0")/lib.sh"
set -o pipefail

sb=$(new_sandbox t11)
v="anthr""opic"
fake_model=dummy-provider/some-model-claude-haiku-5-5
mkdir -p "$sb/fake"
cat > "$sb/fake/opencode" << 'EOF'
#!/usr/bin/env bash
[ "$1" = debug ] && [ "$2" = agents ] && cat "$FAKE_AGENTS" && exit 0
echo "fake opencode: unexpected call: $*" >&2
exit 64
EOF
chmod +x "$sb/fake/opencode"

agents_file() { # file, provider/model -> the list that `opencode debug agents` prints: five agents with that model, and the eight deny rules of the worker
  jq -n --arg p "${2%%/*}" --arg m "${2#*/}" '
    [ "build", "worker", "reviewer", "title", "summary" ] | map({
      id: .,
      mode: (if . == "worker" then "all" else "primary" end),
      model: { providerID: $p, id: $m },
      permissions: (["wrangler *", "cf *", "gh secret *", "gh variable *", "gh api *secrets*", "gh api *variables*", "git push --force*", "git clean *"] | map({ action: "shell", effect: "deny", resource: . }))
    })' > "$1"
}
ca() { # agents file, Variable value -> the output of check-agents.sh (its exit code is the exit code of the function)
  (cd "$sb" && env FAKE_AGENTS="$1" PATH="$sb/fake:$PATH" OPENCODE_MODEL="$2" bash "$ROOT/ci/agents/check-agents.sh" 2>&1)
}

# ---- 1. runner-config: no Variable -> nothing written, exit 0, the notice names Issue 0006
nocfg="$sb/none/opencode.jsonc"
o=$(env -u OPENCODE_MODEL bash "$ROOT/ci/agents/runner-config.sh" "$nocfg" 2>&1); rc=$?
expect_eq "unset Variable: runner-config exits 0 (the stop is after the import)" 0 "$rc"
expect_contains "unset Variable: the notice names Issue 0006" "$o" "Issue 0006"
check "unset Variable: no runner config is written" test ! -e "$nocfg"
o=$(env OPENCODE_MODEL= bash "$ROOT/ci/agents/runner-config.sh" "$nocfg" 2>&1); rc=$?
expect_eq "empty Variable: runner-config exits 0" 0 "$rc"
check "empty Variable: no runner config is written" test ! -e "$nocfg"

# ---- 2. runner-config: a value -> the five agents, the top level and the worker's mode
cfg="$sb/value/opencode.jsonc"
o=$(env OPENCODE_MODEL="$fake_model" bash "$ROOT/ci/agents/runner-config.sh" "$cfg" 2>&1); rc=$?
expect_eq "with the Variable: runner-config exits 0" 0 "$rc"
expect_eq "the five agents use the Variable's value" "$fake_model $fake_model $fake_model $fake_model $fake_model" \
  "$(jq -r '[.agents.build.model, .agents.worker.model, .agents.reviewer.model, .agents.title.model, .agents.summary.model] | join(" ")' "$cfg")"
expect_eq "the top-level model is the same value" "$fake_model" "$(jq -r .model "$cfg")"
expect_eq "the worker is mode all, and the default agent" "all worker" "$(jq -r '[.agents.worker.mode, .default_agent] | join(" ")' "$cfg")"
expect_eq "no #variant is written" 0 "$(grep -c '#' "$cfg")"

# ---- 3. check-agents: the Variable's shape, the agents' models, the deny rules
agents_file "$sb/agents-ok.json" "$fake_model"
out=$(ca "$sb/agents-ok.json" "$fake_model"); rc=$?
expect_eq "all five agents use the Variable's value: check-agents exits 0" 0 "$rc"
expect_contains "and the worker's eight deny rules are there" "$out" "8 件を確認"
out=$(ca "$sb/agents-ok.json" ""); rc=$?
expect_eq "empty Variable: check-agents exits 1" 1 "$rc"
expect_contains "empty Variable: the stop names Issue 0006" "$out" "Issue 0006"
expect_not_contains "empty Variable: it is not reported as a model mismatch" "$out" "になっていない"
out=$(ca "$sb/agents-ok.json" "dummy-provider/some-model-claude-sonnet-5-5"); rc=$?
expect_eq "a value that does not end with claude-haiku-5-5: exit 1" 1 "$rc"
expect_contains "(the message names claude-haiku-5-5)" "$out" "claude-haiku-5-5"
out=$(ca "$sb/agents-ok.json" "$fake_model#high"); rc=$?
expect_eq "a value with a #variant: exit 1" 1 "$rc"
jq '(.[] | select(.id == "reviewer") | .model.id) = "dummy-other-claude-haiku-5-5"' "$sb/agents-ok.json" > "$sb/agents-bad.json"
out=$(ca "$sb/agents-bad.json" "$fake_model"); rc=$?
expect_eq "one agent with another model: check-agents exits 1" 1 "$rc"
expect_contains "and it names that agent" "$out" "reviewer"
expect_not_contains "and it does not print the model value" "$out" "dummy-other"
jq '(.[] | select(.id == "worker") | .permissions) |= map(select(.resource != "gh variable *"))' "$sb/agents-ok.json" > "$sb/agents-nodeny.json"
out=$(ca "$sb/agents-nodeny.json" "$fake_model"); rc=$?
expect_eq "a missing deny rule of the worker: check-agents exits 1" 1 "$rc"

# ---- 4. the workflow: the config before the import, the Variable in both steps, no vendor name in the CI files
wf="$ROOT/.github/workflows/agent.yml"
cfgline=$(grep -n 'bash ci/agents/runner-config.sh' "$wf" | head -n 1 | cut -d: -f1)
impline=$(grep -n 'bash ci/agents/import-login.sh' "$wf" | head -n 1 | cut -d: -f1)
order_ok=no; [ -n "$cfgline" ] && [ -n "$impline" ] && [ "$cfgline" -lt "$impline" ] && order_ok=yes
expect_eq "agent.yml writes the config before the import (the first opencode command of the job)" yes "$order_ok"
# shellcheck disable=SC2016  # the workflow's own expression, matched literally
needle='OPENCODE_MODEL: ${{ vars.OPENCODE_MODEL }}'
expect_eq "agent.yml gives the Variable to the config step and to the check step" 2 "$(grep -cF "$needle" "$wf")"
for f in ci/agents/runner-config.sh ci/agents/check-agents.sh .github/workflows/agent.yml; do
  expect_eq "$f has no vendor name" 0 "$(grep -ci "$v" "$ROOT/$f")"
done

finish
