#!/usr/bin/env bash
# guard/scan.sh [PATH...] : 依頼者の決まりへの違反を、ファイルの内容から見つける。作業ジョブの判定の段で呼ぶ。
# PATH が無いときは、BASE(既定 origin/main)からの変更ファイルを検査する。終了コード 0 = 指摘なし、1 = 指摘あり、2 = 使い方の誤り。
# 規則名・ファイル・行番号を出す。規則: vendor(台帳 L-005)、cfhex・cfhost(L-006)、python(L-008)、mask(L-031: .github/workflows/ の下だけ)。一時ファイルは作らない。
set -o pipefail
v1=anthr; v2=opic; vendor="$v1$v2"   # 名前を、このファイル自身に含めないため、半分ずつ
cfhex='(^|[^0-9A-Za-z])[0-9a-fA-F]{32}($|[^0-9A-Za-z])'
cfhost='[A-Za-z0-9-]+\.r2\.cloudflarestorage\.com|[A-Za-z0-9-]+\.workers\.dev'
pyword='(^|[^[:alnum:]_.-])python3?(\.[0-9]+)*(\.exe)?($|[^[:alnum:]_./\\-]|\\$)'
mask='add-mask'   # 値を隠す命令(台帳 L-031)。ワークフローの中にだけ、規則として見る
if [ "$#" -gt 0 ]; then files=("$@")
else mapfile -t files < <(git diff --name-only --diff-filter=AM "${BASE:-origin/main}"...HEAD) || { echo "scan: git diff failed" >&2; exit 2; }; fi
n=0
rep() { printf '%s\t%s\n' "$1" "$2"; n=$((n+1)); }
for f in "${files[@]}"; do
  [ -f "$f" ] && [ ! -L "$f" ] || continue
  [[ "${f,,}" == *"$vendor"* ]] && rep vendor "$f:0"
  grep -qI . "$f" 2>/dev/null || continue   # バイナリは内容を見ない
  while read -r l; do rep vendor "$f:$l"; done < <(grep -niE -- "$vendor" "$f" | cut -d: -f1)
  while read -r l; do rep cfhex "$f:$l"; done < <(grep -nE -- "$cfhex" "$f" | cut -d: -f1)
  while read -r l; do rep cfhost "$f:$l"; done < <(grep -nE -- "$cfhost" "$f" | cut -d: -f1)
  case "$f" in .github/workflows/*) while read -r l; do rep mask "$f:$l"; done < <(grep -nF -- "$mask" "$f" | cut -d: -f1) ;; esac
  case "$f" in
    *.md) ;;
    .github/*|ci/*|app/*|lift/*|driver/*)
      case "$f" in *.py) rep python "$f:0" ;; esac
      while read -r l; do rep python "$f:$l"; done < <(grep -nE -- "$pyword" "$f" | grep -vE '^[0-9]+:[[:space:]]*(#|//|/\*|\*|<!--|--)' | grep -vE '^[0-9]+:#!.*(sh|bash|node)' | cut -d: -f1) ;;
  esac
done
echo "scan: $n finding(s) in ${#files[@]} file(s)." >&2
[ "$n" -eq 0 ]
