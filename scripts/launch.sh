#!/usr/bin/env bash
# usage: scripts/launch.sh [--dry-run]        (the HUMAN runs this; an agent must not)
# 1. scripts/preflight.sh (a FAIL stops the launch)
# 2. opencode api session.create: agent build, location = this repository, model = the same value as agents.build.model in opencode.jsonc
# 3. add the new session id to ~/.config/opencode/goal-nudge.sessions (one id per line, no duplicates)
# 4. send prompts/START.md with session.prompt, as the human's message. This is the ONLY place in the project where session.prompt is used:
#    the launch is the human's own act. Agents talk to each other with session.synthetic.
# 5. set the goal with session.command (the text is the body of the "## GOAL" section of prompts/START.md)
# --dry-run prints the commands and runs none of them (not even preflight).
# Refuses to run inside an opencode session (OPENCODE_SESSION_ID is set), except with --dry-run.
set -o pipefail

die() { echo "launch: $*" >&2; exit 1; }

dry=0
for a in "$@"; do
  case "$a" in
    --dry-run) dry=1 ;;
    *) die "unknown argument: $a (usage: launch.sh [--dry-run])" ;;
  esac
done
if [ "$dry" -eq 0 ] && [ -n "${OPENCODE_SESSION_ID:-}" ]; then
  die "refusing to launch from inside an opencode session. The human runs this from a terminal."
fi

root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd) || die "cannot resolve the repository root"
cd "$root" || die "cannot enter $root"
for tool in jq python3 opencode; do command -v "$tool" > /dev/null || die "$tool is required"; done
start="$root/prompts/START.md"
[ -r "$start" ] || die "cannot read $start"
registry="$HOME/.config/opencode/goal-nudge.sessions"

# the goal text: the body of "## GOAL" in START.md, up to the next "## " heading or the end of the file
goal=$(awk '/^## GOAL[[:space:]]*$/ {on=1; next} on && /^## / {exit} on {print}' "$start" | sed -e :a -e '/^[[:space:]]*$/{$d;N;ba' -e '}' | sed '/./,$!d')
[ -n "$goal" ] || die "prompts/START.md has no '## GOAL' section (or it is empty): the goal text is the body of that section"
[ "${#goal}" -le 4000 ] || die "the goal text is ${#goal} characters; /goal accepts at most 4000"

# the model: copied from opencode.jsonc (agents.build.model), never typed here
model=$(python3 - "$root/opencode.jsonc" << 'PY'
import json, re, sys
t = open(sys.argv[1], encoding="utf-8").read()
out, i, n, ins = [], 0, len(t), False
while i < n:
    c = t[i]
    if ins:
        out.append(c)
        if c == "\\":
            out.append(t[i + 1]); i += 2; continue
        if c == '"': ins = False
        i += 1; continue
    if c == '"': ins = True; out.append(c); i += 1; continue
    if t.startswith("//", i):
        while i < n and t[i] != "\n": i += 1
        continue
    if t.startswith("/*", i):
        i = t.index("*/", i + 2) + 2; continue
    out.append(c); i += 1
d = json.loads(re.sub(r",(\s*[}\]])", r"\1", "".join(out)))
print(d["agents"]["build"]["model"])
PY
) || die "cannot read agents.build.model from opencode.jsonc"
[ -n "$model" ] || die "agents.build.model is empty in opencode.jsonc"
# "provider/model#variant" -> JSON object, split the way opencode's Model.Ref.parse does (first "/" ends the provider id, the first "#" after it starts the variant)
model_json=$(python3 - "$model" << 'PY'
import json, sys
s = sys.argv[1]
p = s.index("/")
rest = s[p + 1:]
v = rest.find("#")
ref = {"providerID": s[:p], "id": rest if v < 0 else rest[:v]}
if v >= 0: ref["variant"] = rest[v + 1:]
print(json.dumps(ref))
PY
) || die "cannot split the model '$model'"

create_body=$(jq -nc --arg dir "$root" --argjson model "$model_json" '{agent:"build", model:$model, location:{directory:$dir}, title:"signature-studio main"}')

if [ "$dry" -eq 1 ]; then
  echo "# launch.sh --dry-run: nothing is executed."
  echo "1. scripts/preflight.sh                      (a FAIL stops the launch)"
  echo "2. opencode api session.create --data '$create_body'"
  echo "3. append the new session id to $registry (if it is not there yet)"
  echo "4. opencode api session.prompt --param sessionID=<new id> --data '{\"text\": <contents of prompts/START.md, ${#goal} chars of goal not included>}'   [the only session.prompt in this project: the human's launch]"
  echo "5. opencode api session.command --param sessionID=<new id> --data '{\"name\":\"goal\",\"text\":<goal text, ${#goal} characters>}'"
  echo "# goal text:"
  printf '%s\n' "$goal" | sed 's/^/#   /'
  exit 0
fi

bash scripts/preflight.sh || die "preflight reported a FAIL. Fix it and run launch.sh again (nothing was created)"

created=$(opencode api session.create --data "$create_body") || die "session.create failed: $created"
sid=$(jq -r '.data.id // empty' <<< "$created")
[[ "$sid" =~ ^ses_[A-Za-z0-9]+$ ]] || die "session.create returned no session id: $(printf '%s' "$created" | head -c 300)"
echo "launch: created $sid (agent build, model $model, directory $root)"

# register BEFORE the first message: if a later step fails, the timer still resumes the goal once it exists
mkdir -p "$(dirname "$registry")" || die "cannot create $(dirname "$registry")"
if [ -f "$registry" ] && grep -qE "^[[:space:]]*$sid([[:space:]]|$)" "$registry"; then
  echo "launch: $sid is already in $registry"
else
  printf '%s  # signature-studio main (launch.sh)\n' "$sid" >> "$registry" || die "cannot append to $registry (session $sid exists; add it by hand)"
  echo "launch: added $sid to $registry"
fi

prompt_body=$(jq -Rs '{text: .}' < "$start") || die "cannot encode $start"
opencode api session.prompt --param "sessionID=$sid" --data "$prompt_body" > /dev/null || die "session.prompt failed (session $sid exists and is registered)"
echo "launch: sent prompts/START.md as the human's message"

goal_body=$(jq -nc --arg t "$goal" '{name:"goal", text:$t}')
opencode api session.command --param "sessionID=$sid" --data "$goal_body" > /dev/null || die "session.command (goal) failed (session $sid exists, START.md was sent). Set the goal by hand: /goal <text>"
echo "launch: goal set"
echo "launch: done. Session $sid. Check: opencode api session.active ; scripts/audit-session.sh $sid"
