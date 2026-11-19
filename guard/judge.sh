#!/usr/bin/env bash
# guard/judge.sh BASE TASKFILE : 作業ジョブの判定。通れば 0 で終わる。作業ジョブの最後の段で、取り直した main の物を呼ぶ。
# 1) 作業ツリーに、commit していない変更が無い(判定するのは push される物)
# 2) タスクの受け入れコマンド(タスクのファイルの「accept: 」の行)が 0 で終わる。行が無いタスクは、この段を飛ばす
# 3) BASE からの変更ファイルに、依頼者の決まりへの違反が無い(scan.sh、flags.sh)
base=${1:?usage: guard/judge.sh BASE TASKFILE}; task=${2:?usage: guard/judge.sh BASE TASKFILE}
here=$(cd "$(dirname "$0")" && pwd)
if [ -n "$(git status --porcelain)" ]; then echo "judge: commit していない変更がある"; git status --short | head -20; exit 1; fi
cmd=$(grep -m1 '^accept: ' "$task" | cut -c9-)
if [ -n "$cmd" ]; then bash -c "$cmd" || { echo "judge: 受け入れコマンドが失敗した"; exit 1; }; fi
mapfile -t files < <(git diff --name-only --diff-filter=AM "$base"...HEAD)
if [ "${#files[@]}" -gt 0 ]; then bash "$here/scan.sh" "${files[@]}" || { echo "judge: 違反がある"; exit 1; }; fi
bash "$here/flags.sh" "$base" HEAD || { echo "judge: 製品に新しい旗がある"; exit 1; }
echo "judge: 通った"
