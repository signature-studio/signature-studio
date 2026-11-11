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
    *) die "知らない指定です: $a（使い方: launch.sh [--dry-run]）" ;;
  esac
done
if [ "$dry" -eq 0 ] && [ -n "${OPENCODE_SESSION_ID:-}" ]; then
  die "セッションの中からは起動できません。依頼者が端末から実行します。"
fi

root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd) || die "リポジトリの場所を特定できません"
cd "$root" || die "$root に移動できません"
for tool in jq python3 opencode; do command -v "$tool" > /dev/null || die "$tool が必要です"; done
start="$root/prompts/START.md"
[ -r "$start" ] || die "$start を読めません"
registry="$HOME/.config/opencode/goal-nudge.sessions"

# the goal text: the body of "## GOAL" in START.md, up to the next "## " heading or the end of the file
goal=$(awk '/^## GOAL[[:space:]]*$/ {on=1; next} on && /^## / {exit} on {print}' "$start" | sed -e :a -e '/^[[:space:]]*$/{$d;N;ba' -e '}' | sed '/./,$!d')
[ -n "$goal" ] || die "prompts/START.md に「## GOAL」の節がないか、空です。目標の文は、その節の本文です"
[ "${#goal}" -le 4000 ] || die "目標の文が ${#goal} 文字です。/goal は 4000 文字までです"

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
) || die "opencode.jsonc からメインセッションのモデルを読めません"
[ -n "$model" ] || die "opencode.jsonc のメインセッションのモデルが空です"
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
) || die "モデル '$model' を分けられません"

create_body=$(jq -nc --arg dir "$root" --argjson model "$model_json" '{agent:"build", model:$model, location:{directory:$dir}, title:"signature-studio main"}')

if [ "$dry" -eq 1 ]; then
  echo "# launch.sh --dry-run: 何も実行しません。"
  echo "1. scripts/preflight.sh                      （FAIL の行があれば起動を止める）"
  echo "2. opencode api session.create --data '$create_body'"
  echo "3. 新しいセッション ID を $registry に追記する（無い場合のみ）"
  echo "4. opencode api session.prompt --param sessionID=<新しい ID> --data '{\"text\": <prompts/START.md の中身。目標の文 ${#goal} 文字は省略>}'   [session.prompt を使うのは、この起動の 1 回だけ。依頼者の操作]"
  echo "5. opencode api session.command --param sessionID=<新しい ID> --data '{\"name\":\"goal\",\"text\":<目標の文 ${#goal} 文字>}'"
  echo "# 目標の文:"
  printf '%s\n' "$goal" | sed 's/^/#   /'
  exit 0
fi

bash scripts/preflight.sh || die "preflight に FAIL の行があります。直してから launch.sh をもう一度実行してください（何も作っていません）"

created=$(opencode api session.create --data "$create_body") || die "セッションの作成に失敗しました: $created"
sid=$(jq -r '.data.id // empty' <<< "$created")
[[ "$sid" =~ ^ses_[A-Za-z0-9]+$ ]] || die "セッションの作成で ID が返りませんでした: $(printf '%s' "$created" | head -c 300)"
echo "launch: セッション $sid を作成しました（メインセッション、モデル $model、フォルダ $root）"

# register BEFORE the first message: if a later step fails, the timer still resumes the goal once it exists
mkdir -p "$(dirname "$registry")" || die "$(dirname "$registry") を作れません"
if [ -f "$registry" ] && grep -qE "^[[:space:]]*$sid([[:space:]]|$)" "$registry"; then
  echo "launch: $sid は $registry に既にあります"
else
  printf '%s  # signature-studio main (launch.sh)\n' "$sid" >> "$registry" || die "$registry に追記できません（セッション $sid は作成済み。手で追記してください）"
  echo "launch: $sid を $registry に追加しました"
fi

prompt_body=$(jq -Rs '{text: .}' < "$start") || die "$start を変換できません"
opencode api session.prompt --param "sessionID=$sid" --data "$prompt_body" > /dev/null || die "session.prompt が失敗しました（セッション $sid は作成済みで、登録済み）"
echo "launch: prompts/START.md を依頼者の発言として送りました"

goal_body=$(jq -nc --arg t "$goal" '{name:"goal", text:$t}')
opencode api session.command --param "sessionID=$sid" --data "$goal_body" > /dev/null || die "目標の設定に失敗しました（セッション $sid は作成済み、START.md は送信済み）。手で設定してください: /goal <本文>"
echo "launch: 目標を設定しました"
echo "launch: 完了。セッション $sid。確認: opencode api session.active ; scripts/audit-session.sh $sid"
