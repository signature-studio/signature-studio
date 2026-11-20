#!/usr/bin/env bash
# driver/generate.sh 件数 : 機械の生成器(LLM を使わない)。新しいタスクの名前を、件数まで出す。
# 標準入力: 除く名前(1 行に 1 つ。待ち・実行中・失敗のタスク)。
# 型 lower: 関数の表のクラス b のうち、lowered-all.txt に無い関数を、命令の多い順に出す。
# atomic(mt4-atomic-functions.tsv の関数。LOCK 付きの命令を含む)は、fe-wasmgen が直接 wasm にしない(設計)ので、出さない。
set -euo pipefail
n=${1:?usage: driver/generate.sh 件数}
[ "$n" -gt 0 ] || exit 0
awk -F'\t' -v n="$n" '
  function norm(v) { v = tolower(v); while (v ~ /^0x0[0-9a-f]/) v = "0x" substr(v, 4); return v }
  FNR == NR { skip[norm($1)] = 1; next }
  FNR == 1 { next }
  !(norm($1) in skip) { print "lower-" $1; if (++c >= n) exit }
' <(sed -n 's/^lower-//p'; cat lift/harness/hyb/lowered-all.txt; awk -F'\t' '/^0[xX][0-9a-fA-F]/ { print $1 }' lift/harness/seeds/mt4-atomic-functions.tsv) queue/inputs/lift-function-classes.tsv
