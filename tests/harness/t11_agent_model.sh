#!/usr/bin/env bash
# t11: the CI runner's model comes from the Variable OPENCODE_PROVIDER (decision 0388). runner-config.sh picks the one model of that provider whose id contains haiku-5-5 and writes the config;
# check-agents.sh reads that config and checks the agents (a fake `opencode` answers `models --standalone` and `debug agents`).
# .github/workflows/agent.yml: the order import -> config -> check; the Variable is read in the config step only; every call that starts a server before the config is written is --standalone (decision 0384).
# The model ids are dummies (dummy-provider/...), so no vendor name is written here; the CI files must not contain it. No real opencode, service or model call.
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
[ "$1" = models ] && [ "$2" = --standalone ] && cat "$FAKE_MODELS" && exit 0
echo "fake opencode: unexpected call: $*" >&2
exit 64
EOF
chmod +x "$sb/fake/opencode"
printf '%s\n' "$fake_model" "dummy-provider/other-model-1" "other-provider/some-model-claude-haiku-5-5" > "$sb/models-one.txt"
printf '%s\n' "$fake_model" "dummy-provider/second-claude-haiku-5-5" > "$sb/models-two.txt"
printf '%s\n' "dummy-provider/some-model-claude-sonnet-5-5" > "$sb/models-zero.txt"
printf '%s\n' "$fake_model#high" > "$sb/models-variant.txt"

agents_file() { # file, provider/model -> the list that `opencode debug agents` prints: five agents with that model, and the eight deny rules of the worker
  jq -n --arg p "${2%%/*}" --arg m "${2#*/}" '
    [ "build", "worker", "reviewer", "title", "summary" ] | map({
      id: .,
      mode: (if . == "worker" then "all" else "primary" end),
      model: { providerID: $p, id: $m },
      permissions: (["wrangler *", "cf *", "gh secret *", "gh variable *", "gh api *secrets*", "gh api *variables*", "git push --force*", "git clean *"] | map({ action: "shell", effect: "deny", resource: . }))
    })' > "$1"
}
rc_of() { # Variable value ("-" = unset), models file, config path -> the output of runner-config.sh (its exit code is the function's)
  if [ "$1" = "-" ]; then
    (cd "$sb" && env -u OPENCODE_PROVIDER FAKE_MODELS="$2" PATH="$sb/fake:$PATH" bash "$ROOT/ci/agents/runner-config.sh" "$3" 2>&1)
  else
    (cd "$sb" && env OPENCODE_PROVIDER="$1" FAKE_MODELS="$2" PATH="$sb/fake:$PATH" bash "$ROOT/ci/agents/runner-config.sh" "$3" 2>&1)
  fi
}
ca() { # agents file, config path -> the output of check-agents.sh (its exit code is the function's)
  (cd "$sb" && env FAKE_AGENTS="$1" PATH="$sb/fake:$PATH" bash "$ROOT/ci/agents/check-agents.sh" "$2" 2>&1)
}

# ---- 1. runner-config: the Variable is empty or unset -> nothing written, exit 1, the stop names Issue 0007
nocfg="$sb/none/opencode.jsonc"
o=$(rc_of - "$sb/models-one.txt" "$nocfg"); rc=$?
expect_eq "unset Variable: runner-config exits 1" 1 "$rc"
expect_contains "unset Variable: the stop says the Variable is empty and names Issue 0007" "$o" "OPENCODE_PROVIDER)が空。Issue 0007"
check "unset Variable: no runner config is written" test ! -e "$nocfg"
o=$(rc_of "" "$sb/models-one.txt" "$nocfg"); rc=$?
expect_eq "empty Variable: runner-config exits 1" 1 "$rc"
check "empty Variable: no runner config is written" test ! -e "$nocfg"

# ---- 2. runner-config: one model of the provider -> the five agents use it; the value is not printed
cfg="$sb/value/opencode.jsonc"
o=$(rc_of dummy-provider "$sb/models-one.txt" "$cfg"); rc=$?
expect_eq "one candidate: runner-config exits 0" 0 "$rc"
o_visible=$(printf '%s\n' "$o" | grep -v '^::add-mask::')
expect_not_contains "one candidate: the output (without the add-mask workflow command, which the runner consumes) does not print the model id" "$o_visible" "some-model"
expect_not_contains "one candidate: the output does not print the provider" "$o_visible" "dummy-provider"
expect_eq "the five agents use the one model" "$fake_model $fake_model $fake_model $fake_model $fake_model" \
  "$(jq -r '[.agents.build.model, .agents.worker.model, .agents.reviewer.model, .agents.title.model, .agents.summary.model] | join(" ")' "$cfg")"
expect_eq "the top-level model is the same value" "$fake_model" "$(jq -r .model "$cfg")"
expect_eq "the worker is mode all, and the default agent" "all worker" "$(jq -r '[.agents.worker.mode, .default_agent] | join(" ")' "$cfg")"
expect_eq "no #variant is written" 0 "$(grep -c '#' "$cfg")"

# ---- 3. runner-config: no candidate, two candidates, a #variant, a provider name with a slash -> exit 1, nothing written
cfg3="$sb/bad/opencode.jsonc"
o=$(rc_of dummy-provider "$sb/models-zero.txt" "$cfg3"); rc=$?
expect_eq "no candidate: runner-config exits 1" 1 "$rc"
expect_contains "no candidate: the count is printed (0 件)" "$o" "候補が 0 件"
o=$(rc_of dummy-provider "$sb/models-two.txt" "$cfg3"); rc=$?
expect_eq "two candidates: runner-config exits 1" 1 "$rc"
expect_contains "two candidates: the count is printed (2 件)" "$o" "候補が 2 件"
o=$(rc_of dummy-provider "$sb/models-variant.txt" "$cfg3"); rc=$?
expect_eq "a candidate with a #variant: runner-config exits 1" 1 "$rc"
o=$(rc_of dummy-provider/x "$sb/models-one.txt" "$cfg3"); rc=$?
expect_eq "a Variable with a slash: runner-config exits 1" 1 "$rc"
check "no runner config is written by the failing cases" test ! -e "$cfg3"

# ---- 4. check-agents: the config's model, the agents' models, the deny rules
agents_file "$sb/agents-ok.json" "$fake_model"
out=$(ca "$sb/agents-ok.json" "$cfg"); rc=$?
expect_eq "all five agents use the config's value: check-agents exits 0" 0 "$rc"
expect_contains "and the worker's eight deny rules are there" "$out" "8 件を確認"
out=$(ca "$sb/agents-ok.json" "$nocfg"); rc=$?
expect_eq "no config: check-agents exits 1" 1 "$rc"
expect_contains "no config: the stop names Issue 0007" "$out" "Issue 0007"
jq '.model = "dummy-provider/some-model-claude-sonnet-5-5"' "$cfg" > "$sb/cfg-sonnet.jsonc"
out=$(ca "$sb/agents-ok.json" "$sb/cfg-sonnet.jsonc"); rc=$?
expect_eq "a config model that does not end with the worker's type: exit 1" 1 "$rc"
jq --arg m "$fake_model#high" '.model = $m' "$cfg" > "$sb/cfg-variant.jsonc"
out=$(ca "$sb/agents-ok.json" "$sb/cfg-variant.jsonc"); rc=$?
expect_eq "a config model with a #variant: exit 1" 1 "$rc"
jq '(.[] | select(.id == "reviewer") | .model.id) = "dummy-other-claude-haiku-5-5"' "$sb/agents-ok.json" > "$sb/agents-bad.json"
out=$(ca "$sb/agents-bad.json" "$cfg"); rc=$?
expect_eq "one agent with another model: check-agents exits 1" 1 "$rc"
expect_contains "and it names that agent" "$out" "reviewer"
expect_not_contains "and it does not print the model value" "$out" "dummy-other"
jq '(.[] | select(.id == "worker") | .permissions) |= map(select(.resource != "gh variable *"))' "$sb/agents-ok.json" > "$sb/agents-nodeny.json"
out=$(ca "$sb/agents-nodeny.json" "$cfg"); rc=$?
expect_eq "a missing deny rule of the worker: check-agents exits 1" 1 "$rc"

# ---- 5. the workflow: the order, the one step that reads the Variable, the --standalone calls, no vendor name in the CI files
wf="$ROOT/.github/workflows/agent.yml"
impline=$(grep -n 'bash ci/agents/import-login.sh' "$wf" | head -n 1 | cut -d: -f1)
cfgline=$(grep -n 'bash ci/agents/runner-config.sh' "$wf" | head -n 1 | cut -d: -f1)
chkline=$(grep -n 'bash ci/agents/check-agents.sh' "$wf" | head -n 1 | cut -d: -f1)
order_ok=no; [ -n "$impline" ] && [ -n "$cfgline" ] && [ -n "$chkline" ] && [ "$impline" -lt "$cfgline" ] && [ "$cfgline" -lt "$chkline" ] && order_ok=yes
expect_eq "agent.yml: the import, then the config, then the check" yes "$order_ok"
# shellcheck disable=SC2016  # the workflow's own expression, matched literally
expect_eq "agent.yml reads the Variable OPENCODE_PROVIDER in the config step only" 1 "$(grep -cF 'OPENCODE_PROVIDER: ${{ vars.OPENCODE_PROVIDER }}' "$wf")"
expect_eq "agent.yml has one vars. reference in all (the provider)" 1 "$(grep -c 'vars\.' "$wf")"
expect_eq "the old Variable name is not used by the CI files" 0 "$(grep -c 'OPENCODE_MODEL' "$wf" "$ROOT/ci/agents/runner-config.sh" "$ROOT/ci/agents/check-agents.sh" "$ROOT/ci/agents/import-login.sh" | awk -F: '{ s += $2 } END { print s + 0 }')"
expect_eq "the calls that start a server before the config is written use --standalone" 0 \
  "$(grep -E 'opencode (models|auth)' "$ROOT/ci/agents/import-login.sh" "$ROOT/ci/agents/runner-config.sh" | grep -vc -- '--standalone')"
for f in ci/agents/runner-config.sh ci/agents/check-agents.sh ci/agents/import-login.sh .github/workflows/agent.yml; do
  expect_eq "$f has no vendor name" 0 "$(grep -ci "$v" "$ROOT/$f")"
done

finish
