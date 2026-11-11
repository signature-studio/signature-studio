#!/usr/bin/env bash
# usage: scripts/set-model-profile.sh [--dry-run] [--provider gateway|bedrock] [--bedrock-provider ID] [--bedrock-region PREFIX] split|haiku   (run by the owner, never by an agent)
#   split = main Sonnet 5.5; worker, reviewer, title and summary Haiku 5.5      haiku = all Haiku 5.5
#   --provider gateway: the Cloudflare AI gateway. Rewrites the six model lines of opencode.jsonc (top level, build, worker, reviewer, title, summary)
#                       and removes the local override .opencode/opencode.jsonc.
#   --provider bedrock: Amazon Bedrock. opencode.jsonc is NOT changed (its names must not contain the vendor name, see docs/ops/models.md). The same six
#                       model lines are written to .opencode/opencode.jsonc (git-ignored, this machine only). The names come from `opencode models`.
#                       If more than one name fits, nothing is changed and the candidates are shown. Then choose with
#                       --bedrock-provider <provider id> and --bedrock-region <prefix, e.g. global, us, eu, jp, au, in>.
#   Without --provider the current provider is kept.
# --dry-run only shows what would change (a test run of the choice); it writes nothing.
# After a change: `opencode reload`, then the agents row of scripts/preflight.sh. If that row is not OK, the previous state is put back.
# Refuses to write inside an opencode session (OPENCODE_SESSION_ID is set): changing the profile is the owner's operation. --dry-run is always allowed.
set -o pipefail

die() { echo "切り替えを始められません: $*" >&2; exit 2; }

dry=0
profile=""
provider=""
bedrock_provider=""
bedrock_region=""
usage="使い方: set-model-profile.sh [--dry-run] [--provider gateway|bedrock] [--bedrock-provider ID] [--bedrock-region 地域] split|haiku"
while [ "$#" -gt 0 ]; do
  case "$1" in
    --dry-run) dry=1 ;;
    --provider) [ "$#" -ge 2 ] || die "--provider には gateway か bedrock を指定してください"; provider=$2; shift ;;
    --bedrock-provider) [ "$#" -ge 2 ] || die "--bedrock-provider には提供元の ID を指定してください"; bedrock_provider=$2; shift ;;
    --bedrock-region) [ "$#" -ge 2 ] || die "--bedrock-region には global や us などの地域を指定してください"; bedrock_region=$2; shift ;;
    split | haiku) [ -z "$profile" ] || die "切り替え方は 1 つだけ指定してください"; profile="$1" ;;
    *) die "知らない指定です: $1（$usage）" ;;
  esac
  shift
done
[ -n "$profile" ] || die "$usage"
case "$provider" in '' | gateway | bedrock) ;; *) die "--provider は gateway か bedrock です" ;; esac
{ [ -z "$bedrock_provider" ] && [ -z "$bedrock_region" ]; } || [ "$provider" = bedrock ] || die "--bedrock-provider と --bedrock-region は --provider bedrock のときだけ使えます"
if [ "$dry" -eq 0 ] && [ -n "${OPENCODE_SESSION_ID:-}" ]; then
  die "セッションの中からは切り替えられません。切り替えは依頼者が端末から行います。"
fi

root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd) || die "リポジトリの場所を特定できません"
file="$root/opencode.jsonc"
override="$root/.opencode/opencode.jsonc"
[ -f "$file" ] || die "設定ファイルがありません: $file"
command -v python3 > /dev/null || die "python3 が必要です"

current=gateway
[ -f "$override" ] && current=bedrock
[ -n "$provider" ] || provider=$current

tmp=$(mktemp -d "${TMPDIR:-/tmp}/set-model-profile.XXXXXX") || die "一時フォルダを作れません"
trap 'rm -rf "$tmp"' EXIT

# the model names
if [ "$provider" = gateway ]; then
  gw=console-cloudflare-ai-gateway-custom
  sonnet="$gw/claude-sonnet-5-5"
  haiku="$gw/claude-haiku-5-5"
else
  command -v opencode > /dev/null || die "opencode が見つかりません（Bedrock のモデル名を一覧から探すのに必要です）"
  opencode models > "$tmp/models.txt" 2> /dev/null || die "モデルの一覧を取得できませんでした"
  pick=$(python3 - "$tmp/models.txt" "$bedrock_provider" "$bedrock_region" << 'PY'
import re, sys
vendor = "anthr" + "opic"
found = {"sonnet": [], "haiku": []}
want, region = sys.argv[2], sys.argv[3]
pats = {f: re.compile(r"^" + (re.escape(region) + r"\." if region else r"(?:[a-z]+\.)?") + vendor + r"\.claude-" + f + r"-5-5$") for f in found}
for line in open(sys.argv[1], encoding="utf-8"):
    prov, sep, mid = line.strip().partition("/")
    if not sep or "bedrock" not in prov or (want and prov != want):
        continue
    for f in found:
        if pats[f].match(mid) and (prov, mid) not in found[f]:
            found[f].append((prov, mid))
if any(len(found[f]) != 1 for f in found):
    for f, label in (("sonnet", "Sonnet 5.5"), ("haiku", "Haiku 5.5")):
        if not found[f]:
            print("Amazon Bedrock に %s が見つかりません。" % label, file=sys.stderr)
        for prov, mid in found[f]:
            print("候補（%s）: %s/%s" % (label, prov, mid), file=sys.stderr)
    print("候補が 1 つに決まらないため、何も変えていません。上の候補から使う先を選び、--bedrock-provider と --bedrock-region で指定してください。", file=sys.stderr)
    sys.exit(3)
for f in ("sonnet", "haiku"):
    print("%s=%s/%s" % ((f,) + found[f][0]))
PY
  ) || exit 3
  sonnet=$(printf '%s\n' "$pick" | sed -n 's/^sonnet=//p')
  haiku=$(printf '%s\n' "$pick" | sed -n 's/^haiku=//p')
fi

if [ "$profile" = split ]; then
  top=$sonnet; build="$sonnet#xhigh"
else
  top=$haiku; build="$haiku#high"
fi
worker="$haiku#high"; reviewer="$haiku#high"; title="$haiku#low"; summary="$haiku#low"

# the new files
if [ "$provider" = gateway ]; then
  python3 - "$file" "$tmp/new.jsonc" "$top" "$build" "$worker" "$reviewer" "$title" "$summary" << 'PY' || die "設定ファイルの書き換えに失敗しました（上のメッセージを見てください）。$file は変えていません"
import json, re, sys
src, dst = sys.argv[1:3]
want = dict(zip(["", "build", "worker", "reviewer", "title", "summary"], sys.argv[3:9]))

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
    sys.exit("設定ファイルの形式が正しくありません（変更の前から）: %s: %s" % (src, e))

done = set()
lines = text.split("\n")
agent = None
in_agents = False
for k, line in enumerate(lines):
    if re.match(r'^  "agents": \{', line):
        in_agents = True; continue
    m = re.match(r'^    "(\w+)": \{', line)
    if in_agents and m:
        agent = m.group(1)
        if agent in ("title", "summary") and '"model":' in line:
            lines[k] = re.sub(r'"model": "[^"]*"', '"model": "%s"' % want[agent], line, count=1)
            done.add(agent)
        continue
    if in_agents and re.match(r'^    \},?\s*$', line):
        agent = None; continue
    top_model = (not in_agents) and re.match(r'^  "model": "', line)
    agent_model = in_agents and agent in ("build", "worker", "reviewer") and re.match(r'^      "model": "', line)
    if top_model or agent_model:
        key = "" if top_model else agent
        indent = "  " if top_model else "      "
        tail = re.search(r'",?\s*$', line).group(0)
        lines[k] = '%s"model": "%s%s' % (indent, want[key], tail)
        done.add(key)
if done != set(want):
    sys.exit("設定ファイルの形が想定と違います。モデルの行を 6 か所（最上位、メインセッション、作業用・検証用のサブエージェント、表題、要約）見つける必要がありましたが、%d か所でした: %s" % (len(done), sorted(done)))
result = "\n".join(lines)
try:
    json.loads(strip(result))
except Exception as e:
    sys.exit("書き換えた結果の形式が正しくありません（%s）。%s は変えていません" % (e, src))
open(dst, "w", encoding="utf-8").write(result)
PY
  new_main="$tmp/new.jsonc"
  if cmp -s "$file" "$new_main" && [ ! -f "$override" ]; then same=yes; else same=no; fi
else
  python3 - "$tmp/override.jsonc" "$top" "$build" "$worker" "$reviewer" "$title" "$summary" << 'PY' || die "上書きの設定ファイルを作れませんでした"
import json, sys
dst = sys.argv[1]
top, build, worker, reviewer, title, summary = sys.argv[2:8]
body = {"model": top, "agents": {"build": {"model": build}, "worker": {"model": worker}, "reviewer": {"model": reviewer},
                                  "title": {"model": title}, "summary": {"model": summary}}}
text = ("// このファイルは scripts/set-model-profile.sh --provider bedrock が書きます。この機械だけの設定で、Git には入りません。\n"
        "// モデルの行だけを書きます。\n"
        + json.dumps(body, indent=2, ensure_ascii=False) + "\n")
open(dst, "w", encoding="utf-8").write(text)
PY
  if [ -f "$override" ] && cmp -s "$override" "$tmp/override.jsonc"; then same=yes; else same=no; fi
fi

case "$profile" in
  split) plan="メイン = Sonnet 5.5、作業用・検証用のサブエージェント・表題・要約 = Haiku 5.5" ;;
  haiku) plan="すべて Haiku 5.5" ;;
esac
if [ "$provider" = gateway ]; then where="Cloudflare の AI ゲートウェイ経由"; else where="Amazon Bedrock"; fi
if [ "$same" = yes ]; then
  echo "すでに「$where・$plan」になっています。何も変えません。"
  exit 0
fi
if [ "$dry" -eq 1 ]; then
  echo "次の内容に切り替えます: $where、$plan"
  echo "（確認だけなので、何も書き換えていません）"
  exit 0
fi

# the previous state, for putting it back
cp -p "$file" "$tmp/orig.jsonc" || die "$file を退避できません"
if [ -f "$override" ]; then cp -p "$override" "$tmp/orig-override.jsonc" && had_override=1; else had_override=0; fi
restore() {
  cp "$tmp/orig.jsonc" "$file"
  if [ "$had_override" -eq 1 ]; then cp "$tmp/orig-override.jsonc" "$override"; else rm -f "$override"; fi
  command -v opencode > /dev/null && opencode reload > /dev/null 2>&1
}

if [ "$provider" = gateway ]; then
  cp "$new_main" "$file" || die "$file に書けません"
  rm -f "$override"
else
  mkdir -p "$root/.opencode"
  cp "$tmp/override.jsonc" "$override" || die "$override に書けません"
fi
command -v opencode > /dev/null && opencode reload > /dev/null 2>&1
row=$(bash "$root/scripts/preflight.sh" 2> /dev/null | awk '$1 == "agents"')
case "$row" in
  *" OK "*)
    echo "切り替えました: 使う先 = $where、$plan"
    echo "動いているセッションは自動では切り替わりません。モデルを選び直すか、新しいセッションを作り直してください。"
    exit 0 ;;
esac
restore
detail=$(printf '%s' "$row" | sed -E 's/^agents +(OK|FAIL|WARN) +//')
echo "切り替えを取り消しました。確認で問題が見つかったため、元の状態に戻しました。" >&2
echo "見つかった問題: ${detail:-（確認の結果が出ませんでした）}" >&2
exit 1
