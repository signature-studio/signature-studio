#!/usr/bin/env bash
# t11: the CI runner's model is chosen from the provider list of the imported login: the one model that a console- provider lists with an id that ends with haiku-5-5 (decision 0399). No Variable is read.
# runner-config.sh picks it and writes the config; check-agents.sh reads that config and checks the agents (a fake `opencode` answers `models --standalone` and `debug agents`).
# .github/workflows/agent.yml: the order import -> config -> check; the workflow reads no Variable; every call that starts a server before the config is written is --standalone (decision 0384).
# The provider and model ids are dummies (console-dummy/...), so no vendor name is written here; the CI files must not contain it. No real opencode, service or model call.
# shellcheck source=lib.sh
. "$(dirname "$0")/lib.sh"
set -o pipefail

sb=$(new_sandbox t11)
v="anthr""opic"
fake_provider=console-dummy
fake_model=$fake_provider/some-model-claude-haiku-5-5
mkdir -p "$sb/fake"
cat > "$sb/fake/opencode" << 'EOF'
#!/usr/bin/env bash
[ "$1" = debug ] && [ "$2" = agents ] && cat "$FAKE_AGENTS" && exit 0
[ "$1" = models ] && [ "$2" = --standalone ] && cat "$FAKE_MODELS" && exit 0
echo "fake opencode: unexpected call: $*" >&2
exit 64
EOF
chmod +x "$sb/fake/opencode"
printf '%s\n' "$fake_model" "console-dummy/other-model-1" > "$sb/models-one.txt"
printf '%s\n' "$fake_model" "console-dummy/other-model-1" "other-provider/some-model-claude-haiku-5-5" > "$sb/models-with-other.txt"
printf '%s\n' "$fake_model" "console-other/second-claude-haiku-5-5" > "$sb/models-two.txt"
printf '%s\n' "console-dummy/some-model-claude-sonnet-5-5" > "$sb/models-zero.txt"
printf '%s\n' "other-provider/some-model-claude-haiku-5-5" > "$sb/models-nonconsole.txt"
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
rc_of() { # models file, config path -> the output of runner-config.sh (its exit code is the function's)
  (cd "$sb" && env -u "OPENCODE""_PROVIDER" FAKE_MODELS="$1" PATH="$sb/fake:$PATH" bash "$ROOT/ci/agents/runner-config.sh" "$2" 2>&1)
}
ca() { # agents file, config path -> the output of check-agents.sh (its exit code is the function's)
  (cd "$sb" && env FAKE_AGENTS="$1" PATH="$sb/fake:$PATH" bash "$ROOT/ci/agents/check-agents.sh" "$2" 2>&1)
}

# ---- 1. runner-config: no console- candidate (none, or only a non-console provider) -> nothing written, exit 1, the count is printed
nocfg="$sb/none/opencode.jsonc"
o=$(rc_of "$sb/models-zero.txt" "$nocfg"); rc=$?
expect_eq "(b) no haiku-5-5 model under a console- provider: runner-config exits 1" 1 "$rc"
expect_contains "(b) and the count is printed (0 件)" "$o" "候補が 0 件"
check "(b) no runner config is written" test ! -e "$nocfg"
o=$(rc_of "$sb/models-nonconsole.txt" "$nocfg"); rc=$?
expect_eq "(d) a haiku-5-5 model under a non-console provider alone: runner-config exits 1" 1 "$rc"
expect_contains "(d) and the count is 0" "$o" "候補が 0 件"
o=$(rc_of "$sb/models-variant.txt" "$nocfg"); rc=$?
expect_eq "a candidate with a #variant is not a candidate: runner-config exits 1" 1 "$rc"
check "no runner config is written by the failing cases" test ! -e "$nocfg"

# ---- 2. runner-config: two console- candidates -> exit 1, nothing written, the count is printed
cfg2="$sb/two/opencode.jsonc"
o=$(rc_of "$sb/models-two.txt" "$cfg2"); rc=$?
expect_eq "(c) two haiku-5-5 models under console- providers: runner-config exits 1" 1 "$rc"
expect_contains "(c) and the count is printed (2 件)" "$o" "候補が 2 件"
check "(c) no runner config is written" test ! -e "$cfg2"

# ---- 3. runner-config: one console- candidate -> the five agents use it; the value is not printed
cfg="$sb/value/opencode.jsonc"
o=$(rc_of "$sb/models-one.txt" "$cfg"); rc=$?
expect_eq "(a) one candidate: runner-config exits 0" 0 "$rc"
o_visible=$(printf '%s\n' "$o" | grep -v '^::add-mask::')
expect_not_contains "(e) the output (without the add-mask workflow command, which the runner consumes) does not print the model id" "$o_visible" "some-model"
expect_not_contains "(e) the output does not print the provider name" "$o_visible" "$fake_provider"
expect_eq "the five agents use the one model" "$fake_model $fake_model $fake_model $fake_model $fake_model" \
  "$(jq -r '[.agents.build.model, .agents.worker.model, .agents.reviewer.model, .agents.title.model, .agents.summary.model] | join(" ")' "$cfg")"
expect_eq "the top-level model is the same value" "$fake_model" "$(jq -r .model "$cfg")"
expect_eq "the worker is mode all, and the default agent" "all worker" "$(jq -r '[.agents.worker.mode, .default_agent] | join(" ")' "$cfg")"
expect_eq "no #variant is written" 0 "$(grep -c '#' "$cfg")"

# ---- 4. runner-config: a haiku-5-5 model under a non-console provider is not counted; the former provider Variable is not read (its name is built here, so no CI file names it)
cfg4="$sb/other/opencode.jsonc"
o=$(cd "$sb" && env "OPENCODE""_PROVIDER=other-provider" FAKE_MODELS="$sb/models-with-other.txt" PATH="$sb/fake:$PATH" bash "$ROOT/ci/agents/runner-config.sh" "$cfg4" 2>&1); rc=$?
expect_eq "(d) the non-console haiku line is not counted, and the former provider Variable is not read: exit 0" 0 "$rc"
expect_eq "(d) the config uses the console- candidate" "$fake_model" "$(jq -r .model "$cfg4")"

# ---- 5. check-agents: the config's model, the agents' models, the deny rules
agents_file "$sb/agents-ok.json" "$fake_model"
out=$(ca "$sb/agents-ok.json" "$cfg"); rc=$?
expect_eq "all five agents use the config's value: check-agents exits 0" 0 "$rc"
expect_contains "and the worker's eight deny rules are there" "$out" "8 件を確認"
out=$(ca "$sb/agents-ok.json" "$nocfg"); rc=$?
expect_eq "no config: check-agents exits 1" 1 "$rc"
expect_contains "no config: the stop says runner-config.sh did not write it" "$out" "runner-config.sh が書いていない"
jq '.model = "console-dummy/some-model-claude-sonnet-5-5"' "$cfg" > "$sb/cfg-sonnet.jsonc"
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

# ---- 6. the workflow: the order, no Variable read, the --standalone calls, no vendor name in the CI files, no old Variable name
wf="$ROOT/.github/workflows/agent.yml"
impline=$(grep -n 'bash ci/agents/import-login.sh' "$wf" | head -n 1 | cut -d: -f1)
cfgline=$(grep -n 'bash ci/agents/runner-config.sh' "$wf" | head -n 1 | cut -d: -f1)
chkline=$(grep -n 'bash ci/agents/check-agents.sh' "$wf" | head -n 1 | cut -d: -f1)
order_ok=no; [ -n "$impline" ] && [ -n "$cfgline" ] && [ -n "$chkline" ] && [ "$impline" -lt "$cfgline" ] && [ "$cfgline" -lt "$chkline" ] && order_ok=yes
expect_eq "agent.yml: the import, then the config, then the check" yes "$order_ok"
expect_eq "agent.yml reads no Variable (no vars. reference)" 0 "$(grep -c 'vars\.' "$wf")"
expect_eq "the old Variable names are not used by .github, ci, scripts or tests" 0 \
  "$(grep -rc -e "OPENCODE_""PROVIDER" -e "OPENCODE_""MODEL" "$ROOT/.github" "$ROOT/ci" "$ROOT/scripts" "$ROOT/tests" | awk -F: '{ s += $NF } END { print s + 0 }')"
expect_eq "the calls that start a server before the config is written use --standalone" 0 \
  "$(grep -E 'opencode (models|auth)' "$ROOT/ci/agents/import-login.sh" "$ROOT/ci/agents/runner-config.sh" | grep -vc -- '--standalone')"
for f in ci/agents/runner-config.sh ci/agents/check-agents.sh ci/agents/import-login.sh .github/workflows/agent.yml; do
  expect_eq "$f has no vendor name" 0 "$(grep -ci "$v" "$ROOT/$f")"
done

finish
