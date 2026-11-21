#!/usr/bin/env bash
# driver/generate.sh 件数 : 機械の生成器(LLM を使わない)。新しいタスクの名前を、件数まで出す。
# 標準入力: 除く名前(1 行に 1 つ。待ち・実行中・失敗のタスク)。
# 型 lowerb(台帳 L-037): 関数のバッチ lowerb-<k> のうち、まだ足す関数が残っている物を、番号の昇順に出す。バッチの中身は driver/lowerb.sh rows が決める。
# 古い型 lower-<VA> は出さない(台帳 L-037 の是正で、待ちの lower-0x は閉じる)。atomic の関数は、バッチに入らない(lowerb.sh rows)。
set -euo pipefail
n=${1:?usage: driver/generate.sh 件数}
[ "$n" -gt 0 ] || exit 0
awk -F'\t' -v n="$n" '
  BEGIN { cur = "-" }
  function norm(v) { v = tolower(v); while (v ~ /^0x0[0-9a-f]/) v = "0x" substr(v, 4); return v }
  FILENAME == ARGV[1] { skip[$0] = 1; next }
  FILENAME == ARGV[2] { if ($1 ~ /^0[xX]/) done[norm($1)] = 1; next }
  FILENAME == ARGV[3] { if (FNR > 1) done[norm($1)] = 1; next }
  { if ($1 == cur || (norm($2) in done)) next; cur = $1; if (("lowerb-" $1) in skip) next; print "lowerb-" $1; if (++c >= n) exit }
' <(cat) lift/harness/hyb/lowered-all.txt queue/inputs/lower-rejected.tsv <(bash driver/lowerb.sh rows)
