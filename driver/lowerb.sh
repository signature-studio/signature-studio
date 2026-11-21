#!/usr/bin/env bash
# driver/lowerb.sh 番号 [check] | rows : 関数のバッチ(40 件)を直接 wasm にする(型 lowerb。台帳 L-037)。
# バッチ: 関数の表(queue/inputs/lift-function-classes.tsv)から atomic(lift/harness/seeds/mt4-atomic-functions.tsv)を除き、表の順に 40 件ずつ。番号は 0 から 4 桁。表だけで決まる(進み具合で動かない)。
# 既定: バッチの関数のうち、lowered-all.txt にも lower-rejected.tsv にも無い物を、lists と lowered-all.txt の末尾に 1 行ずつ足し、lower-check.sh(ビルドと起動確認)で確かめて、1 つの commit にする。
#   engine の段で listed but not emitted の関数は外して記録する。boot の段で通らないときは、二分探索で通る最大の前半を残し、後半を boot として記録する。ビルドは合計 6 回まで。通る物が無ければ、変更を戻して 1 で終わる。
# check: バッチの関数が全て lowered-all.txt か lower-rejected.tsv に在るかを確かめる(受け入れの前半)。
# rows: 全バッチの行(番号、VA、module)を出す。generate.sh が使う。
set -euo pipefail
MAX=6

rows() {
  awk -F'\t' '
    function norm(v) { v = tolower(v); while (v ~ /^0x0[0-9a-f]/) v = "0x" substr(v, 4); return v }
    FILENAME == ARGV[1] { if ($1 ~ /^0[xX][0-9a-fA-F]+$/) atomic[norm($1)] = 1; next }
    $1 ~ /^0[xX][0-9a-fA-F]+$/ && !(norm($1) in atomic) { k = int(n / 40); n++; printf "%04d\t%s\t%s\n", k, $1, $2 }
  ' lift/harness/seeds/mt4-atomic-functions.tsv queue/inputs/lift-function-classes.tsv
}
batch() { rows | awk -F'\t' -v k="$1" '$1 == k { print $2 "\t" $3 }'; }
# 標準入力の「VA<TAB>module」のうち、lowered-all.txt にも lower-rejected.tsv にも無い物
pending() {
  awk -F'\t' '
    function norm(v) { v = tolower(v); while (v ~ /^0x0[0-9a-f]/) v = "0x" substr(v, 4); return v }
    FILENAME == ARGV[1] { if ($1 ~ /^0[xX]/) done[norm($1)] = 1; next }
    FILENAME == ARGV[2] { if (FNR > 1) done[norm($1)] = 1; next }
    !(norm($1) in done) { print $1 "\t" $2 }
  ' lift/harness/hyb/lowered-all.txt queue/inputs/lower-rejected.tsv -
}

cmd=${1:?usage: driver/lowerb.sh 番号|rows [check]}
if [ "$cmd" = rows ]; then rows; exit 0; fi
[[ $cmd =~ ^[0-9]+$ ]] || { echo "lowerb: 番号が不正: $cmd" >&2; exit 1; }
k=$(printf '%04d' "$((10#$cmd))")
all=$(batch "$k")
[ -n "$all" ] || { echo "lowerb: バッチ $k が無い" >&2; exit 1; }
if [ "${2:-}" = check ]; then
  left=$(printf '%s\n' "$all" | pending | cut -f1 | paste -sd' ' -)
  [ -z "$left" ] || { echo "lowerb: バッチ $k に未処理の関数が残っている: $left" >&2; exit 1; }
  echo "lowerb: バッチ $k は全て処理済み"
  exit 0
fi

: "${RUNNER_TEMP:?}"
[ -z "$(git status --porcelain -- lift/harness queue/inputs)" ] || { echo "lowerb: lift/harness と queue/inputs に、commit していない変更がある" >&2; exit 1; }
cand=$(printf '%s\n' "$all" | pending)
if [ -z "$cand" ]; then echo "lowerb: バッチ $k: 足す関数が無い"; exit 0; fi
mapfile -t S <<<"$cand"
builds=0; STAGE=""; EMIT=""; rej=()

addline() { if [ -s "$1" ] && [ -n "$(tail -c1 "$1")" ]; then printf '\n' >> "$1"; fi; printf '%s\n' "$2" >> "$1"; }
# 一覧を HEAD に戻してから、標準入力の「VA<TAB>module」を末尾に足す
apply() {
  git checkout -q -- lift/harness/hyb/lists lift/harness/hyb/lowered-all.txt
  local va mod
  while IFS=$'\t' read -r va mod; do
    [ -n "$va" ] || continue
    addline "lift/harness/hyb/lists/lowered-$mod.txt" "$va"
    addline lift/harness/hyb/lowered-all.txt "$va"
  done
}
reject() { rej+=("$1"$'\t'"$2"$'\t'"$3"); }
# 先頭 $1 件を足して確かめる。通れば 0。落ちた段(STAGE)と、listed but not emitted の VA(EMIT)を残す
attempt() {
  local n=$1 log
  builds=$((builds + 1)); log="$RUNNER_TEMP/lowerb-$k-$builds.txt"
  printf '%s\n' "${S[@]:0:n}" | apply
  if bash driver/lower-check.sh > "$log" 2>&1; then echo "lowerb $k: ビルド $builds 回目は通った($n 件)"; return 0; fi
  STAGE=$(sed -n 's/^lower-check: 段: //p' "$log" | tail -n 1)
  EMIT=$(grep -o 'not emitted: 0x[0-9a-fA-F]*' "$log" | sed 's/^not emitted: //' | tr 'A-F' 'a-f' | sort -u || true)
  echo "lowerb $k: ビルド $builds 回目は落ちた($n 件、段: ${STAGE:-不明})"
  return 1
}
drop_emitted() {
  local keep=() s va mod
  for s in "${S[@]}"; do
    IFS=$'\t' read -r va mod <<<"$s"
    if grep -qxF "$va" <<<"$EMIT"; then reject "$va" "$mod" "listed but not emitted"; else keep+=("$s"); fi
  done
  S=("${keep[@]}")
}

OUT=fail
if attempt "${#S[@]}"; then
  OUT=pass
else
  while [ "$builds" -lt "$MAX" ]; do
    if [ "$STAGE" = engine ] && [ -n "$EMIT" ]; then
      before=${#S[@]}; drop_emitted
      [ "${#S[@]}" -lt "$before" ] || break
      if [ "${#S[@]}" -eq 0 ]; then OUT=pass; break; fi
      if attempt "${#S[@]}"; then OUT=pass; break; fi
      continue
    fi
    if [ "$STAGE" = boot ] && [ "${#S[@]}" -gt 1 ]; then
      all_n=${#S[@]}; lo=0; hi=$all_n
      while [ $((hi - lo)) -gt 1 ] && [ "$builds" -lt "$MAX" ]; do
        mid=$(((lo + hi) / 2))
        if attempt "$mid"; then lo=$mid; else hi=$mid; fi
      done
      if [ "$lo" -gt 0 ]; then
        for ((i = lo; i < all_n; i++)); do IFS=$'\t' read -r va mod <<<"${S[i]}"; reject "$va" "$mod" boot; done
        S=("${S[@]:0:lo}"); OUT=pass
      fi
    fi
    break
  done
fi
if [ "$OUT" != pass ]; then
  git checkout -q -- lift/harness queue/inputs
  echo "lowerb $k: 通る物が無かった(ビルド $builds 回)。変更を戻した" >&2
  exit 1
fi

printf '%s\n' "${S[@]}" | apply
date=$(TZ=Asia/Tokyo date +%F)
for r in ${rej[@]+"${rej[@]}"}; do printf '%s\t%s\n' "$r" "$date" >> queue/inputs/lower-rejected.tsv; done
git add -- lift/harness/hyb/lists lift/harness/hyb/lowered-all.txt queue/inputs/lower-rejected.tsv
if git diff --cached --quiet; then echo "lowerb $k: 変更が無い"; exit 0; fi
n=${#S[@]}
if [ "$n" -gt 0 ]; then
  mods=$(printf '%s\n' "${S[@]}" | cut -f2 | sort | uniq -c | awk '{ printf "%s%s: %d", (NR > 1 ? ", " : ""), $2, $1 }')
  subj="lowerb-$k: $n 関数を直接 wasm にする($mods)"
else
  subj="lowerb-$k: 0 関数を直接 wasm にする"
fi
if [ "${#rej[@]}" -gt 0 ]; then
  git commit -q -m "$subj" -m "外した: ${#rej[@]} 件(queue/inputs/lower-rejected.tsv に記録)"
else
  git commit -q -m "$subj"
fi
echo "lowerb $k: commit した($n 件を足し、${#rej[@]} 件を外した)"
