#!/usr/bin/env bash
# guard/scan.sh [--redact] [PATH...] : 依頼者の決まりへの違反を、ファイルの内容から見つける。作業ジョブの判定の段で呼ぶ。
# PATH が無いときは、BASE(既定 origin/main)からの変更ファイルを検査する。終了コード 0 = 指摘なし、1 = 指摘あり、2 = 使い方の誤り。
# --redact PATH...: PATH の中の vendor の一致を [伏せ字] に置き換えてから、同じ検査をする(報告の検査の段だけで使う)。置き換えた行は「redact ファイル:行」を出す(値は出さない)。ほかの規則の指摘は、従来どおり失敗にする。
# 規則名・ファイル・行番号を出す。規則: vendor(台帳 L-005)、cfhex(lift/crates/*/tests/ の試験のデータを除く)・cfhost(L-006)、python(L-008)、mask(L-031: .github/workflows/ の下だけ)、wranglergen(L-032: wrangler の設定ファイルを作る・書く行。spec/・pm-work/・*.md は対象外)。scan 自身は一時ファイルを作らない(--redact は sed -i で書き換える)。
set -o pipefail
v1=anthr; v2=opic; vendor="$v1$v2"   # 名前を、このファイル自身に含めないため、半分ずつ
cfhex='(^|[^0-9A-Za-z])[0-9a-fA-F]{32}($|[^0-9A-Za-z])'
tdata='^lift/crates/[^/]+/tests/.+[.](rs|txt|tsv|hex)$'   # 試験のデータの場所(cfhex の対象から除く)
cfhost='[A-Za-z0-9-]+\.r2\.cloudflarestorage\.com|[A-Za-z0-9-]+\.workers\.dev'
pyword='(^|[^[:alnum:]_.-])python3?(\.[0-9]+)*(\.exe)?($|[^[:alnum:]_./\\-]|\\$)'
mask='add-mask'   # 値を隠す命令(台帳 L-031)。ワークフローの中にだけ、規則として見る
w='wrangler[.]jsonc'   # wrangler の設定ファイル(台帳 L-032)。名前の語の境界で見る。読むだけの呼び出しは、書きの判定の前に取り除く
wb="(^|[^A-Za-z0-9_.-])$w"
qt="[\"']"
wg_redir="[>][>|]?[[:space:]]*${qt}?[^[:space:]\"';&|]*${wb}"
wg_tee="(^|[^A-Za-z0-9_.-])tee[^;&|]*${wb}"
wg_inplace="(^|[^A-Za-z0-9_.-])(sed|perl)[^;&|]*[[:space:]]-[A-Za-z]*i[^;&|]*${wb}"
wg_dd="(^|[^A-Za-z0-9_.-])dd[^;&|]*of=[^[:space:];&|]*${wb}"
wg_lastarg="(^|[^A-Za-z0-9_.-])(cp|mv|install|ln|touch|truncate)[[:space:]]([^;&|]*[[:space:]])?${qt}?[^[:space:];&|\"']*${w}${qt}?[[:space:]]*($|[;&|)])"
wg_js="(writeFile|writeFileSync|appendFile|appendFileSync|createWriteStream|copyFile|copyFileSync|cpSync|rename|renameSync|symlink|symlinkSync|truncateSync|fs::write|fs::copy|fs::rename|fs::remove_file|File::create|OpenOptions)"
wg_read="(readFileSync|readFile|read_to_string)[(][^)]*[)]"
redact=0
if [ "${1:-}" = --redact ]; then
  redact=1; shift
  [ "$#" -gt 0 ] || { echo "scan: --redact には PATH が要る" >&2; exit 2; }
fi
if [ "$#" -gt 0 ]; then files=("$@")
else mapfile -t files < <(git diff --name-only --diff-filter=AM "${BASE:-origin/main}"...HEAD) || { echo "scan: git diff failed" >&2; exit 2; }; fi
n=0
rep() { printf '%s\t%s\n' "$1" "$2"; n=$((n+1)); }
if [ "$redact" -eq 1 ]; then
  for f in "${files[@]}"; do
    [ -f "$f" ] && [ ! -L "$f" ] || continue
    grep -qI . "$f" 2>/dev/null || continue
    while read -r l; do printf 'redact\t%s:%s\n' "$f" "$l"; done < <(grep -niE -- "$vendor" "$f" | cut -d: -f1)
    sed -i -E "s/$vendor/[伏せ字]/gI" "$f"
  done
fi
for f in "${files[@]}"; do
  [ -f "$f" ] && [ ! -L "$f" ] || continue
  [[ "${f,,}" == *"$vendor"* ]] && rep vendor "$f:0"
  grep -qI . "$f" 2>/dev/null || continue   # バイナリは内容を見ない
  while read -r l; do rep vendor "$f:$l"; done < <(grep -niE -- "$vendor" "$f" | cut -d: -f1)
  # 試験の機械語・画素のデータは、32 桁の 16 進を多く含み、識別子ではない。試験の外の場所では、今まで通り検査する
  [[ "$f" =~ $tdata ]] || while read -r l; do rep cfhex "$f:$l"; done < <(grep -nE -- "$cfhex" "$f" | cut -d: -f1)
  while read -r l; do rep cfhost "$f:$l"; done < <(grep -nE -- "$cfhost" "$f" | cut -d: -f1)
  case "$f" in .github/workflows/*) while read -r l; do rep mask "$f:$l"; done < <(grep -nF -- "$mask" "$f" | cut -d: -f1) ;; esac
  case "$f" in spec/*|pm-work/*|*.md) ;; *)
    while read -r l; do rep wranglergen "$f:$l"; done < <({ grep -nE -- "$wg_redir" "$f"; grep -nE -- "$wg_tee" "$f"; grep -nE -- "$wg_inplace" "$f"; grep -nE -- "$wg_dd" "$f"; grep -nE -- "$wg_lastarg" "$f"; sed -E "s/$wg_read//g" "$f" | grep -nE -- "$wg_js" | grep -E -- "$wb"; } | cut -d: -f1 | sort -un) ;;
  esac
  case "$f" in
    *.md) ;;
    .github/*|ci/*|app/*|lift/*|driver/*)
      case "$f" in *.py) rep python "$f:0" ;; esac
      while read -r l; do rep python "$f:$l"; done < <(grep -nE -- "$pyword" "$f" | grep -vE '^[0-9]+:[[:space:]]*(#|//|/\*|\*|<!--|--)' | grep -vE '^[0-9]+:#!.*(sh|bash|node)' | cut -d: -f1) ;;
  esac
done
echo "scan: $n finding(s) in ${#files[@]} file(s)." >&2
[ "$n" -eq 0 ]
