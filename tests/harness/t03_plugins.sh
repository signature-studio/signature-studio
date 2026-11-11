#!/usr/bin/env bash
# t03: the three plugins, with a MOCK plugin context (level E3). They are stateless functions of (event, ctx); the mock gives them a ctx.
# This does not show that opencode calls them in a real session: README.md "Canary" is the check for that.
# Also checks, with the real opencode loader (isolated, no model call), that the three files load and register without an error.
# shellcheck source=lib.sh
. "$(dirname "$0")/lib.sh"
set -o pipefail

if ! command -v node > /dev/null; then skip "node missing: plugin tests"; finish; exit $?; fi
sb=$(new_sandbox t03)
cp "$ROOT"/.opencode/plugins/*.ts "$sb/"
mkdir -p "$sb/repo/prompts"
printf '<!-- worker-rules v1 -->\nRULES-BODY\n' > "$sb/repo/prompts/worker-rules.md"
printf 'no marker here\n' > "$sb/nomarker.md"

cat > "$sb/run.mjs" << 'JS'
import guard from "./harness-guard.ts"
import rules from "./worker-rules.ts"
import gate from "./model-pin-gate.ts"
import { mkdirSync, writeFileSync, rmSync } from "node:fs"

const out = []
const rec = (name, ok, extra = "") => out.push((ok ? "PASS" : "FAIL") + "  " + name + (extra ? " (" + extra + ")" : ""))
const load = async (plugin, ctx) => { let cb; await plugin.setup({ ...ctx, tool: { hook: async (n, c) => { if (n === "execute.before") cb = c } } }); return cb }
const rejects = async (cb, event) => { try { await cb(event); return null } catch (e) { return e.message } }

// ---- harness-guard
const g = await load(guard, {})
const sh = (command) => ({ tool: "shell", agent: "worker", input: { command } })
for (const [name, ev, want] of [
  ["subagent with model is rejected", { tool: "subagent", input: { agent: "worker", prompt: "x", model: "p/m" } }, "model"],
  ["subagent with model+variant is rejected", { tool: "subagent", input: { agent: "worker", prompt: "x", model: "p/m#high" } }, "model"],
  ["trailing & is rejected", sh("npm run dev &"), "trailing"],
  ["trailing & with spaces is rejected", sh("npm run dev &   "), "trailing"],
  ["FOO=1 sleep is rejected", sh("FOO=1 sleep 1"), "banned"],
  ["env sleep is rejected", sh("env sleep 1"), "banned"],
  ["env -i sleep is rejected", sh("env -i sleep 1"), "banned"],
  ["sudo nohup is rejected", sh("sudo nohup foo"), "banned"],
  ["VAR=x timeout is rejected", sh("TMPDIR=/x timeout 100 npx y"), "banned"],
  ["after && : env sleep rejected", sh("cd x && env sleep 1"), "banned"],
  ["after ; : FOO=1 timeout rejected", sh("echo a; FOO=1 timeout 5 ls"), "banned"],
  ["after | : exec systemd-run rejected", sh("echo a | exec systemd-run --user x"), "banned"],
  ["after newline : xargs sleep rejected", sh("ls\nxargs sleep 1"), "banned"],
]) { const m = await rejects(g, ev); rec("guard: " + name, m !== null && m.includes(want), m ? m.slice(0, 40) : "passed") }
for (const [name, ev] of [
  ["subagent without model passes", { tool: "subagent", input: { agent: "worker", prompt: "x" } }],
  ["subagent with empty model passes (opencode drops it)", { tool: "subagent", input: { agent: "worker", prompt: "x", model: "" } }],
  ["a && b passes", sh("make && make test")],
  ["a &> file passes", sh("make &> out.log")],
  ["2>&1 passes", sh("make 2>&1 | tail -5")],
  ["URL with & passes", sh("curl 'http://x/?a=1&b=2'")],
  ["quoted & at the end passes", sh("echo 'a &'")],
  ["background:true passes", { tool: "shell", input: { command: "npm run dev", background: true } }],
  ["word 'sleep' as an argument passes", sh("git commit -m 'fix sleep handling'")],
  ["grep sleep passes", sh("grep -rn sleep src")],
  ["env without banned command passes", sh("env FOO=1 make")],
  ["FOO=1 make passes", sh("FOO=1 make test")],
  ["a path containing sleep passes", sh("ls ./sleeper/")],
  ["read tool passes", { tool: "read", input: { path: "x" } }],
  ["shell without input passes", { tool: "shell" }],
  ["bash -c sleep passes (known limit: only audit sees it)", sh('bash -c "sleep 1"')],
]) { const m = await rejects(g, ev); rec("guard: " + name, m === null, m ? m.slice(0, 60) : "") }

// ---- worker-rules
const dir = process.argv[2]
const wr = await load(rules, { location: { directory: dir + "/repo" } })
const run = async (input) => { const ev = { tool: "subagent", agent: "build", input }; const m = await rejects(wr, ev); return { m, input: ev.input } }
let r = await run({ agent: "worker", description: "d", prompt: "do the thing" })
rec("worker-rules: worker prompt gets the rules in front", r.m === null && r.input.prompt.startsWith("<!-- worker-rules v1 -->\nRULES-BODY") && r.input.prompt.endsWith("do the thing"))
rec("worker-rules: the original prompt is kept in full", r.input.prompt.includes("\n\ndo the thing"))
rec("worker-rules: other fields are kept", r.input.description === "d" && r.input.agent === "worker")
r = await run({ agent: "reviewer", description: "d", prompt: "check" })
rec("worker-rules: reviewer prompt gets the rules too", r.m === null && r.input.prompt.startsWith("<!-- worker-rules v1 -->"))
r = await run({ agent: "worker", description: "d", prompt: "<!-- worker-rules v1 -->\nalready here\ntask" })
rec("worker-rules: a prompt that already starts with the marker is left alone", r.m === null && r.input.prompt === "<!-- worker-rules v1 -->\nalready here\ntask")
r = await run({ agent: "worker", description: "d", prompt: "\n  <!-- worker-rules v1 -->\nx" })
rec("worker-rules: marker after leading whitespace counts as present", r.m === null && r.input.prompt.includes("x") && !r.input.prompt.includes("RULES-BODY"))
r = await run({ agent: "worker", sessionID: "ses_abc", prompt: "follow-up" })
rec("worker-rules: a follow-up to an existing session is left alone", r.m === null && r.input.prompt === "follow-up")
r = await run({ agent: "general", description: "d", prompt: "x" })
rec("worker-rules: another agent name is left alone (the permission denies it)", r.m === null && r.input.prompt === "x")
const e2 = { tool: "shell", input: { command: "ls" } }
rec("worker-rules: a non-subagent tool is left alone", (await rejects(wr, e2)) === null && e2.input.command === "ls")
const wrMissing = await load(rules, { location: { directory: dir + "/nowhere" } })
const miss = await rejects(wrMissing, { tool: "subagent", input: { agent: "worker", prompt: "x" } })
rec("worker-rules: unreadable prompts/worker-rules.md is REJECTED with a visible reason (never silently passed)", miss !== null && miss.includes("cannot read") && miss.includes("worker-rules.md"), miss ? miss.slice(0, 60) : "passed")
mkdirSync(dir + "/repo2/prompts", { recursive: true }); writeFileSync(dir + "/repo2/prompts/worker-rules.md", "no marker line\n")
const wrBad = await load(rules, { location: { directory: dir + "/repo2" } })
const bad = await rejects(wrBad, { tool: "subagent", input: { agent: "worker", prompt: "x" } })
rec("worker-rules: a worker-rules.md without the marker is rejected", bad !== null && bad.includes("must start with"), bad ? bad.slice(0, 60) : "passed")

// ---- model-pin-gate
const SON = { providerID: "console-cloudflare-ai-gateway-custom", id: "claude-sonnet-5-5" }
const HAI = { providerID: "console-cloudflare-ai-gateway-custom", id: "claude-haiku-5-5" }
let sessionModel, agentModels = { build: SON, worker: HAI, reviewer: HAI }
const gt = await load(gate, {
  agent: { get: async ({ agentID }) => { if (!(agentID in agentModels)) throw new Error("Agent not found: " + agentID); return { location: {}, data: { model: agentModels[agentID] } } } },
  session: { get: async () => ({ model: sessionModel }) },
})
const gev = (agent) => ({ tool: "read", agent, sessionID: "ses_x", input: {} })
for (const [name, agent, model, want] of [
  ["build + Sonnet passes", "build", SON, null],
  ["build + Haiku is rejected", "build", HAI, "ツールを止めました"],
  ["build + other provider's Sonnet is rejected", "build", { providerID: "other", id: "claude-sonnet-5-5" }, "ツールを止めました"],
  ["worker + Haiku passes", "worker", HAI, null],
  ["worker + Sonnet is rejected", "worker", SON, "ツールを止めました"],
  ["reviewer + Haiku (variant differs) passes", "reviewer", { ...HAI, variant: "default" }, null],
  ["reviewer + Sonnet is rejected", "reviewer", SON, "ツールを止めました"],
  ["no stored session model passes (the agent's own model is used)", "worker", undefined, null],
  ["an ungated agent (compaction) passes", "compaction", SON, null],
]) { sessionModel = model; const m = await rejects(gt, gev(agent)); rec("gate: " + name, want === null ? m === null : m !== null && m.includes(want), m ? m.slice(0, 50) : "") }
agentModels = { build: HAI, worker: HAI, reviewer: HAI }   // the profile `haiku`: the pin follows the configuration with no change in the plugin
sessionModel = HAI
rec("gate: profile haiku: build + Haiku passes (pin read from the agent definition)", (await rejects(gt, gev("build"))) === null)
sessionModel = SON
rec("gate: profile haiku: build + Sonnet is rejected", (await rejects(gt, gev("build"))) !== null)
agentModels = { build: undefined, worker: HAI, reviewer: HAI }
sessionModel = SON
const nomodel = await rejects(gt, gev("build"))
rec("gate: an agent without a model in the configuration is rejected visibly (config not loaded)", nomodel !== null && nomodel.includes("モデルが設定されていません"), nomodel ? nomodel.slice(0, 50) : "passed")

// ---- statelessness: the same event gives the same answer twice, in any order
sessionModel = SON; agentModels = { build: SON, worker: HAI, reviewer: HAI }
const a1 = await rejects(gt, gev("worker")), a2 = await rejects(gt, gev("worker")), a3 = await rejects(gt, gev("build"))
rec("stateless: repeated calls give the same answers", a1 !== null && a1 === a2 && a3 === null)
console.log(out.join("\n"))
JS
res=$(cd "$sb" && node run.mjs "$sb" 2>&1)
while IFS= read -r line; do
  case "$line" in
    PASS*) pass "${line#PASS  }" ;;
    FAIL*) fail "${line#FAIL  }" ;;
    *) fail "unexpected output: $line" ;;
  esac
done <<< "$res"

# --- structural limits: small, no state, no timers, no events
for f in harness-guard worker-rules model-pin-gate; do
  lines=$(wc -l < "$ROOT/.opencode/plugins/$f.ts")
  if [ "$lines" -le 40 ]; then pass "$f.ts is $lines lines (limit 40)"; else fail "$f.ts is $lines lines (limit 40)"; fi
  if grep -nE '\b(new (Map|Set|WeakMap)|setTimeout|setInterval|subscribe|globalThis|process\.env|writeFile|appendFile)\b|^\s*(let|var) ' "$ROOT/.opencode/plugins/$f.ts" > /dev/null; then
    fail "$f.ts holds state or uses a timer/event/environment/file write: $(grep -nE '\b(new (Map|Set|WeakMap)|setTimeout|setInterval|subscribe|globalThis|process\.env|writeFile|appendFile)\b|^\s*(let|var) ' "$ROOT/.opencode/plugins/$f.ts" | head -2 | tr '\n' ' ')"
  else pass "$f.ts: no Map/Set, no let/var, no timers, no events, no environment, no file writes"; fi
done
# the only file a plugin may read is prompts/worker-rules.md
reads=$(grep -hoE 'readFile\([^)]*\)' "$ROOT"/.opencode/plugins/*.ts | sort -u | paste -sd'|' -)
expect_eq "the only file read by a plugin" "readFile(file, \"utf8\")" "$reads"

# --- the real loader: the three files are found, loaded, and register (isolated opencode; no model call)
if command -v opencode > /dev/null; then
  lsb="$sb/loader"; mkdir -p "$lsb/proj/.opencode" "$lsb/xdg"
  cp "$ROOT/opencode.jsonc" "$lsb/proj/"; cp -r "$ROOT/.opencode/plugins" "$lsb/proj/.opencode/"
  mkdir -p "$lsb/proj/prompts"; cp "$sb/repo/prompts/worker-rules.md" "$lsb/proj/prompts/"
  logf="$lsb/log.txt"
  (
    export XDG_CONFIG_HOME="$lsb/xdg/config" XDG_DATA_HOME="$lsb/xdg/data" XDG_STATE_HOME="$lsb/xdg/state" XDG_CACHE_HOME="$lsb/xdg/cache"
    mkdir -p "$XDG_CONFIG_HOME" "$XDG_DATA_HOME" "$XDG_STATE_HOME" "$XDG_CACHE_HOME"
    cd "$lsb/proj" && opencode --standalone --print-logs --log-level info debug agents > /dev/null 2> "$logf"
    opencode service stop > /dev/null 2>&1
  )
  for f in harness-guard worker-rules model-pin-gate; do
    if grep -q "msg=\"loading plugin\" id=.*/$f.ts" "$logf"; then pass "opencode's loader loads $f.ts"; else fail "opencode's loader did not log 'loading plugin' for $f.ts"; fi
  done
  if grep -iE 'level=(ERROR|WARN)' "$logf" | grep -iE 'plugin|harness|worker-rules|model-pin' > /dev/null; then fail "plugin load logged an error/warning: $(grep -iE 'level=(ERROR|WARN)' "$logf" | grep -iE 'plugin' | head -2 | cut -c1-200)"; else pass "no plugin error or warning in the loader's log"; fi
else
  skip "opencode missing: real loader test"
fi

finish
