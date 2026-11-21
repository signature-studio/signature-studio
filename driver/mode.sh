#!/usr/bin/env bash
# driver/mode.sh 名前 : タスク本文の mode: の値を出す(台帳 L-037)。script = スクリプトが作業する、agent = 作業者が作業する。
# 行が無い・値が script と agent 以外・本文の無い名前は、何も出さずに非 0 で終わる。
set -euo pipefail
id=${1:?usage: driver/mode.sh 名前}
text=$(bash "$(dirname "$0")/task.sh" "$id" 2>/dev/null) || exit 1
m=$(awk '/^mode: / { print substr($0, 7); exit }' <<<"$text")
case $m in script|agent) echo "$m" ;; *) exit 1 ;; esac
