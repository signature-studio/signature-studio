#!/usr/bin/env bash
# driver/generate.sh 件数 : 機械の生成器(LLM を使わない)。新しいタスクの名前を、件数まで出す。
# 標準入力: 除く名前(1 行に 1 つ。待ち・実行中・失敗のタスク)。
# 型 lower: 関数の表のクラス b のうち、lowered-all.txt に無い関数を、命令の多い順に出す。
set -euo pipefail
n=${1:?usage: driver/generate.sh 件数}
[ "$n" -gt 0 ] || exit 0
awk -F'\t' -v n="$n" '
  FNR == NR { skip[$1] = 1; next }
  FNR == 1 { next }
  !($1 in skip) { print "lower-" $1; if (++c >= n) exit }
' <(sed -n 's/^lower-//p'; cat lift/harness/hyb/lowered-all.txt) queue/inputs/lift-function-classes.tsv
