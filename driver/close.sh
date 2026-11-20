#!/usr/bin/env bash
# driver/close.sh Issue番号 結果 runのURL : 作業ジョブの結果で、待ち行列の Issue の状態を変える。
# 通った: 閉じる。落ちた: 待ちに戻す。原因(失敗した段と、判定の出力の末尾)と、落ちた時の main の SHA と state をコメントに残す。3 回落ちたら failed にして閉じる。取り消された: 落ちた数に入れず、待ちに戻す。
# 要る環境: GH_TOKEN(issues の書き込み、actions の読み取り)、GH_REPO。
set -euo pipefail
source "$(dirname "$0")/state.sh"
num=${1:?usage: driver/close.sh 番号 結果 URL}; result=${2:?}; url=${3:?}
if [ "$result" = success ]; then
  gh issue close "$num" --reason completed --comment "通った: $url" >/dev/null
  exit 0
fi
if [ "$result" = cancelled ]; then
  gh issue edit "$num" --remove-label running --add-label ready >/dev/null
  exit 0
fi
# 原因: 報告の成果物(agent-<名前>)の judge.txt の末尾。成果物が無ければ、失敗した段の名前だけ
note() {
  local run=${url##*/} title steps tail_=""
  local d=".tmp-close-$run"
  title=$(gh issue view "$num" --json title --jq .title)
  steps=$(gh run view "$run" --json jobs --jq '[.jobs[].steps[] | select(.conclusion == "failure") | .name] | join(", ")' 2>/dev/null || true)
  rm -rf "$d"; mkdir -p "$d"
  gh run download "$run" -n "agent-$title" -D "$d" >/dev/null 2>&1 || true
  if [ -s "$d/judge.txt" ]; then tail_=$(tail -n 8 "$d/judge.txt"); fi
  rm -rf "$d"
  printf 'main-sha: %s\nstate: %s\n失敗した段: %s\n判定の出力(末尾):\n%s\n' "$(git rev-parse HEAD)" "$(state)" "${steps:-(取れなかった)}" "${tail_:-(判定の出力が無い。ジョブのログを見る)}"
}
fails=$(gh issue view "$num" --json comments --jq '[.comments[].body | select(startswith("落ちた:"))] | length')
fails=$((fails + 1))
if [ "$fails" -ge 3 ]; then
  gh issue edit "$num" --remove-label running --add-label failed >/dev/null
  gh issue close "$num" --reason "not planned" --comment "$(printf '落ちた: %s(%s 回目。failed にした)\n%s' "$url" "$fails" "$(note)")" >/dev/null
else
  gh issue edit "$num" --remove-label running --add-label ready >/dev/null
  gh issue comment "$num" --body "$(printf '落ちた: %s(%s 回目。待ちに戻した)\n%s' "$url" "$fails" "$(note)")" >/dev/null
fi
