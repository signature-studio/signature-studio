#!/usr/bin/env bash
# driver/close.sh Issue番号 結果 runのURL : 作業ジョブの結果で、待ち行列の Issue の状態を変える。
# 通った: 閉じる。落ちた: 待ちに戻す。3 回落ちたら failed にして閉じる。取り消された: 落ちた数に入れず、待ちに戻す。
# 要る環境: GH_TOKEN(issues の書き込み)、GH_REPO。
set -euo pipefail
num=${1:?usage: driver/close.sh 番号 結果 URL}; result=${2:?}; url=${3:?}
if [ "$result" = success ]; then
  gh issue close "$num" --reason completed --comment "通った: $url" >/dev/null
  exit 0
fi
if [ "$result" = cancelled ]; then
  gh issue edit "$num" --remove-label running --add-label ready >/dev/null
  exit 0
fi
fails=$(gh issue view "$num" --json comments --jq '[.comments[].body | select(startswith("落ちた:"))] | length')
fails=$((fails + 1))
if [ "$fails" -ge 3 ]; then
  gh issue edit "$num" --remove-label running --add-label failed >/dev/null
  gh issue close "$num" --reason "not planned" --comment "落ちた: $url($fails 回目。failed にした)" >/dev/null
else
  gh issue edit "$num" --remove-label running --add-label ready >/dev/null
  gh issue comment "$num" --body "落ちた: $url($fails 回目。待ちに戻した)" >/dev/null
fi
