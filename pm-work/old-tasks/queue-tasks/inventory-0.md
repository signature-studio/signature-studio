# inventory-0: インストーラの全ファイルの一覧を queue/inputs に置く(台帳 L-033。C-3 と C-4 の入力)

mode: script
run: bash driver/inventory-commit.sh

## 目的
作業ジョブは R2 の鍵を持たないので、インストーラの全ファイルの一覧を作れない。extract ジョブ(鍵を持つ)が、インストーラ(ci/artifacts.lock の 1 件目)から一覧を作り、キャッシュ(inventory-<run_id>-<run_attempt>)に入れる。この作業ジョブが、それを $RUNNER_TEMP/inventory/ に復元して、queue/inputs/fl-installer-files.tsv に置き、commit する。C-3(言語パック・テーマ・テンプレート)と C-4(内蔵のエフェクトとインストゥルメントの一覧)の入力になる。読みの根拠は queue/plan.md の「共通の前提: インストーラの一覧」。

## やること(スクリプトが行う。エージェントは動かない)
1. extract ジョブ(このタスクの run のときだけ)が、インストーラを取り、`node driver/inventory.mjs` で表を作り、キャッシュに保存する。
2. run ジョブが、キャッシュを $RUNNER_TEMP/inventory/ に復元する(fail-on-cache-miss)。
3. `driver/inventory-commit.sh` が、表を並べて queue/inputs/fl-installer-files.tsv に置き、1 つの commit にする。

## 受け入れ
表(見出しの 1 行を除く)が 100 行以上。Plugins\Fruity\Effects\ の下の dll が 20 以上、Plugins\Fruity\Generators\ の下の dll が 10 以上。dll の全行に 64 桁の SHA-256 がある。

accept: f=queue/inputs/fl-installer-files.tsv; test -f "$f" && test "$(wc -l < "$f")" -ge 101 && test "$(awk -F'\t' 'index($1, "Plugins\\Fruity\\Effects\\") == 1 && $4 == "dll"' "$f" | wc -l)" -ge 20 && test "$(awk -F'\t' 'index($1, "Plugins\\Fruity\\Generators\\") == 1 && $4 == "dll"' "$f" | wc -l)" -ge 10 && test "$(awk -F'\t' '$4 == "dll" && (length($3) != 64 || $3 !~ /^[0-9a-f]+$/)' "$f" | wc -l)" -eq 0
