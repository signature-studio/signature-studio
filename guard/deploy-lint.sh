#!/usr/bin/env bash
# guard/deploy-lint.sh [FILE] : 依頼者の deploy.yml の制約(2026-11-16)。FILE の既定は .github/workflows/deploy.yml。ファイル全体は 220 行以内、job ごとに build は 120 行以内・他は 40 行以内(2026-11-26 に、3 つの job に分けたため改めた。理由は pm-work/workflow-design.md の「19」)、1 行 80 字以内、シェルスクリプトを実行しない、
# ローカル action・再利用ワークフロー・composite action を使わない、Makefile を作らない。終了コード 0 = 守っている。
f=${1:-.github/workflows/deploy.yml}; bad=0
chk() { echo "deploy-lint: $1"; bad=1; }
[ "$(wc -l < "$f")" -le 220 ] || chk "220 行を超えている"
over=$(awk 'match($0,/^  [a-z][a-z0-9_-]*:$/){if(j!="")print j,n; j=$1; n=0} j!=""{n++} END{if(j!="")print j,n}' "$f" | awk '{l=($1=="build:")?120:40; if ($2 > l) print $1, $2, l}')
while read -r j c l; do [ -n "$j" ] && chk "job ${j%:} が $c 行(上限 $l 行)"; done <<<"$over"
w=$(awk 'length($0)>80{print NR; exit}' "$f"); [ -z "$w" ] || chk "80 字を超える行がある(行 $w)"
grep -nE '^[[:space:]]*-?[[:space:]]*run:.*(\.sh\b|\bbash\b|\bsh\b[[:space:]]+-c|\bmake\b|\./[A-Za-z])' "$f" >/dev/null && chk "シェルスクリプトか make を実行している"
grep -nE 'uses:[[:space:]]*\./|workflow_call' "$f" >/dev/null && chk "ローカル action か再利用ワークフローを使っている"
[ -z "$(git ls-files -- Makefile makefile GNUmakefile '*/Makefile' 2>/dev/null)" ] || chk "Makefile がある"
[ "$bad" -eq 0 ]
