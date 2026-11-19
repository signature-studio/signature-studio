#!/usr/bin/env bash
# driver/task.sh 名前 : タスクの本文を出す。TASK_REF(既定 HEAD)の木から読む。
# 探す順: queue/tasks/<名前>.md、ci/tasks/<名前>.md、queue/kinds の型(lower-<VA>)。
set -euo pipefail
id=${1:?usage: driver/task.sh 名前}
ref=${TASK_REF:-HEAD}
[[ $id =~ ^[A-Za-z0-9._-]+$ ]] || { echo "task: 名前が不正: $id" >&2; exit 1; }
show() { git show "$ref:$1" 2>/dev/null; }
show "queue/tasks/$id.md" && exit 0
show "ci/tasks/$id.md" && exit 0
case $id in
  lower-0x*)
    va=${id#lower-}
    [[ $va =~ ^0x[0-9a-f]+$ ]] || { echo "task: VA が不正: $va" >&2; exit 1; }
    row=$(show queue/inputs/lift-function-classes.tsv | awk -F'\t' -v v="$va" '$1 == v && !f { print; f = 1 }')
    [ -n "$row" ] || { echo "task: 関数の表に $va が無い" >&2; exit 1; }
    IFS=$'\t' read -r _ module ins <<<"$row"
    show queue/kinds/lower.md | sed -e "s/{{va}}/$va/g" -e "s/{{module}}/$module/g" -e "s/{{instructions}}/$ins/g"
    ;;
  *) echo "task: $id の本文が無い" >&2; exit 1 ;;
esac
