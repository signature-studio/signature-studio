#!/usr/bin/env bash
# guard/deploy-lint.sh [FILE] : 依頼者の deploy.yml の制約(2026-11-16)。FILE の既定は .github/workflows/deploy.yml。100 行以内、1 行 80 字以内、シェルスクリプトを実行しない、
# ローカル action・再利用ワークフロー・composite action を使わない、Makefile を作らない。終了コード 0 = 守っている。
f=${1:-.github/workflows/deploy.yml}; bad=0
chk() { echo "deploy-lint: $1"; bad=1; }
[ "$(wc -l < "$f")" -le 100 ] || chk "100 行を超えている"
w=$(awk 'length($0)>80{print NR; exit}' "$f"); [ -z "$w" ] || chk "80 字を超える行がある(行 $w)"
grep -nE '^[[:space:]]*-?[[:space:]]*run:.*(\.sh\b|\bbash\b|\bsh\b[[:space:]]+-c|\bmake\b|\./[A-Za-z])' "$f" >/dev/null && chk "シェルスクリプトか make を実行している"
grep -nE 'uses:[[:space:]]*\./|workflow_call' "$f" >/dev/null && chk "ローカル action か再利用ワークフローを使っている"
[ -z "$(git ls-files -- Makefile makefile GNUmakefile '*/Makefile' 2>/dev/null)" ] || chk "Makefile がある"
[ "$bad" -eq 0 ]
