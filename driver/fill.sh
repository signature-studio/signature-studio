#!/usr/bin/env bash
# driver/fill.sh : 閾値補充。実行中(running)が閾値(queue/threshold)より少なければ、待ち(ready)から不足分を起動する。
# 待ちが 100 件を下回れば、先に生成器で足す。待ち行列は Issues(題名がタスクの名前、ラベルが状態)。
# 要る環境: GH_TOKEN(actions と issues の書き込み)、GH_REPO。main の作業ツリーで実行する。
set -euo pipefail
source "$(dirname "$0")/state.sh"
threshold=$(cat queue/threshold)
list() { gh issue list --state open --label "$1" --limit 1000 --json number,title --jq '.[] | "\(.number)\t\(.title)"'; }
count() { grep -c . <<<"$1" || true; }
ready=$(list ready); running=$(list running)
if [ "$(count "$ready")" -lt 100 ]; then
  for l in ready running failed; do gh label create "$l" --force >/dev/null; done
  failed=$(gh issue list --state closed --label failed --limit 10000 --json title --jq '.[].title')
  { cut -f2 <<<"$ready"; cut -f2 <<<"$running"; printf '%s\n' "$failed"; } |
    bash driver/generate.sh $((100 - $(count "$ready"))) |
    while read -r id; do
      gh issue create --title "$id" --label ready --body "生成器が作ったタスク。本文は \`bash driver/task.sh $id\` が出す。" >/dev/null
    done
  ready=$(list ready)
fi
k=$((threshold - $(count "$running")))
echo "fill: 閾値 $threshold、実行中 $(count "$running")、待ち $(count "$ready")、起動 $((k > 0 ? k : 0))"
[ "$k" -gt 0 ] || exit 0
# 落ちた時の state(Issue の state: 行)と今の state が同じ Issue は、同じ結果になるので起動しない(driver/state.sh)。飛ばした件数はログに出す
# 起動の順: 題名が lower- で始まらない物を先に、lower を後に。どちらも番号の昇順(台帳 L-036。lower は件数が多く、先に並ぶと他の物が待つ)
order=$(sort -n <<<"$ready" | awk -F'\t' '$2 !~ /^lower-/'; sort -n <<<"$ready" | awk -F'\t' '$2 ~ /^lower-/')
cur=$(state); launched=0; skipped=0
while IFS=$'\t' read -r num id && [ "$launched" -lt "$k" ]; do
  [ -n "$num" ] || continue
  text=$(bash driver/task.sh "$id" 2>/dev/null || true)
  if ! grep -q '^accept: ' <<<"$text"; then
    gh issue edit "$num" --remove-label ready --add-label failed >/dev/null
    gh issue close "$num" --reason "not planned" --comment "受け入れコマンド(accept: の行)が無いので、待ち行列に入れない。" >/dev/null
    continue
  fi
  last=$(gh issue view "$num" --json comments --jq '[.comments[].body | split("\n")[] | select(startswith("state: ")) | ltrimstr("state: ")] | last // ""')
  if [ "$last" = "$cur" ]; then skipped=$((skipped + 1)); continue; fi
  gh issue edit "$num" --remove-label ready --add-label running >/dev/null
  gh workflow run agent.yml -f task="$id" -f issue="$num"
  launched=$((launched + 1))
done < <(printf '%s\n' "$order")
echo "fill: 状態が同じで起動を飛ばした $skipped 件、起動 $launched 件"
