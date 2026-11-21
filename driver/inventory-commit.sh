#!/usr/bin/env bash
# driver/inventory-commit.sh : extract ジョブが作ったインストーラの一覧($RUNNER_TEMP/inventory/fl-installer-files.tsv)を、queue/inputs/ に置いて commit する(タスク inventory-0。台帳 L-033)。
# 見出しの行は先頭に残し、残りを C ロケールで並べる。中身は driver/inventory.mjs が作る(この段は並べ替えと置き場所だけ)。
# 要る環境: RUNNER_TEMP。リポジトリの根で実行する。名義は環境の git の設定のまま(名義を指定する引数は付けない)。
set -euo pipefail
: "${RUNNER_TEMP:?}"
src=$RUNNER_TEMP/inventory/fl-installer-files.tsv
dst=queue/inputs/fl-installer-files.tsv
[ -s "$src" ] || { echo "inventory-commit: 一覧が無い: $src" >&2; exit 1; }
[ -z "$(git status --porcelain -- queue/inputs)" ] || { echo "inventory-commit: queue/inputs に、commit していない変更がある" >&2; exit 1; }
mkdir -p queue/inputs
{ head -n 1 "$src"; tail -n +2 "$src" | LC_ALL=C sort; } > "$dst"
git add -- "$dst"
n=$(($(wc -l < "$dst") - 1))
if git diff --cached --quiet; then echo "inventory-commit: 変更が無い"; exit 0; fi
git commit -q -m "inventory-0: インストーラの全ファイルの一覧を queue/inputs に置く($n 件)"
echo "inventory-commit: commit した($n 件)"
