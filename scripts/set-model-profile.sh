#!/usr/bin/env bash
# usage: scripts/set-model-profile.sh [--dry-run] split|haiku        (the owner runs this; never an agent)
#   split = main Sonnet 5.5, worker and reviewer Haiku 5.5      haiku = all Haiku 5.5
# Rewrites four "model" lines of opencode.jsonc: the top level, agents.build, agents.worker, agents.reviewer. Nothing else changes; comments are kept.
# Checks that the file is valid JSONC before and after, runs `opencode reload` (a running server keeps the old configuration until then),
# then runs scripts/preflight.sh and looks at its `agents` row. If that row is not OK the original file is put back (and reloaded).
# Refuses to write when it runs inside an opencode session (OPENCODE_SESSION_ID is set): changing the profile is the owner's operation. --dry-run is always allowed.
set -o pipefail

die() { echo "set-model-profile: $*" >&2; exit 2; }

dry=0
profile=""
for a in "$@"; do
  case "$a" in
    --dry-run) dry=1 ;;
    split | haiku) [ -z "$profile" ] || die "give one profile only"; profile="$a" ;;
    *) die "unknown argument: $a (usage: set-model-profile.sh [--dry-run] split|haiku)" ;;
  esac
done
[ -n "$profile" ] || die "usage: set-model-profile.sh [--dry-run] split|haiku"
if [ "$dry" -eq 0 ] && [ -n "${OPENCODE_SESSION_ID:-}" ]; then
  die "refusing to change the model profile from inside an opencode session. The owner runs this from a terminal."
fi

root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd) || die "cannot resolve the repository root"
file="$root/opencode.jsonc"
[ -f "$file" ] || die "no such file: $file"
command -v python3 > /dev/null || die "python3 is required"

provider=console-cloudflare-ai-gateway-custom
sonnet="$provider/claude-sonnet-5-5"
haiku="$provider/claude-haiku-5-5"
if [ "$profile" = split ]; then
  top=$sonnet; build="$sonnet#xhigh"
else
  top=$haiku; build="$haiku#high"
fi
worker="$haiku#high"
reviewer="$haiku#high"

new=$(mktemp "${TMPDIR:-/tmp}/opencode-jsonc.XXXXXX") || die "cannot create a temporary file"
orig=$(mktemp "${TMPDIR:-/tmp}/opencode-jsonc-orig.XXXXXX") || die "cannot create a temporary file"
trap 'rm -f "$new" "$orig"' EXIT
cp -p "$file" "$orig" || die "cannot back up $file"

# python: validate the original, rewrite the four lines (line based, so comments stay), validate the result.
python3 - "$file" "$new" "$top" "$build" "$worker" "$reviewer" << 'PY' || die "rewrite failed (see the message above); $file is unchanged"
import json, re, sys
src, dst, top, build, worker, reviewer = sys.argv[1:7]

def strip(t):
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
    return re.sub(r",(\s*[}\]])", r"\1", "".join(out))

text = open(src, encoding="utf-8").read()
try:
    json.loads(strip(text))
except Exception as e:
    sys.exit("set-model-profile: %s is not valid JSONC before the change: %s" % (src, e))

want = {"": top, "build": build, "worker": worker, "reviewer": reviewer}
done = set()
lines = text.split("\n")
agent = None
in_agents = False
for k, line in enumerate(lines):
    if re.match(r'^  "agents": \{', line): in_agents = True; continue
    m = re.match(r'^    "(\w+)": \{', line)
    if in_agents and m: agent = m.group(1); continue
    if in_agents and re.match(r'^    \},?\s*$', line): agent = None; continue
    top_model = (not in_agents) and re.match(r'^  "model": "', line)
    agent_model = in_agents and agent in ("build", "worker", "reviewer") and re.match(r'^      "model": "', line)
    if top_model or agent_model:
        key = "" if top_model else agent
        indent = "  " if top_model else "      "
        tail = re.search(r'",?\s*$', line).group(0)
        lines[k] = '%s"model": "%s%s' % (indent, want[key], tail)
        done.add(key)
if done != set(want):
    sys.exit("set-model-profile: expected 4 model lines (top level, build, worker, reviewer) in the layout of opencode.jsonc, rewrote %d: %s" % (len(done), sorted(done)))
result = "\n".join(lines)
try:
    json.loads(strip(result))
except Exception as e:
    sys.exit("set-model-profile: the result is not valid JSONC (%s); %s is unchanged" % (e, src))
open(dst, "w", encoding="utf-8").write(result)
PY

if cmp -s "$file" "$new"; then
  echo "set-model-profile: opencode.jsonc already has the '$profile' profile; nothing to change"
  exit 0
fi

echo "set-model-profile: '$profile' changes these lines:"
diff <(grep -n '^ *"model":' "$file") <(grep -n '^ *"model":' "$new") | grep '^[<>]'
if [ "$dry" -eq 1 ]; then
  echo "set-model-profile: --dry-run, nothing written"
  exit 0
fi

cp "$new" "$file" || die "cannot write $file"
command -v opencode > /dev/null && opencode reload > /dev/null 2>&1
row=$(bash "$root/scripts/preflight.sh" 2> /dev/null | awk '$1 == "agents"')
case "$row" in
  *" OK "*)
    echo "set-model-profile: preflight agents row: $row"
    echo "set-model-profile: done. A session that is already running keeps the model stored in the session; new sessions use the new profile."
    exit 0 ;;
esac
echo "set-model-profile: preflight agents row is not OK: ${row:-<no row>}" >&2
cp "$orig" "$file" && command -v opencode > /dev/null && opencode reload > /dev/null 2>&1
echo "set-model-profile: the original opencode.jsonc was put back" >&2
exit 1
