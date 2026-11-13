# 0352: 読まれない変数 ev_total・clip_owner を除去
分類: (a 除去)
撤回条件: これらの変数を読む処理が必要になったとき(診断の出力など)。そのときは読む側を足して戻す。
日付: 2026-11-13   ROADMAP: R0-6   一覧: -   コミット: 43771fd..7c138e4

## 決定
決定: `ev_total`(`lift/runtime/fle_input.c`)と `clip_owner`(`lift/runtime/fle_clip.c`)を除去する。根拠: 両方とも書き込みだけで読む側が無く、clang 24 の `-Wunused-but-set-global` が `-Werror` で停止させていた(失敗 42 件)。

## 実測(コマンド・条件・日付・n)
- 読む側の確認: `git grep -nw -e ev_total -e clip_owner -- lift app ':!lift/out*'` の結果は、`fle_input.c` の宣言 1 行と `ev_total++` 4 行、`fle_clip.c` の宣言 1 行と代入 2 行の計 8 行のみ(読む側 0)。
- 除去: 8 行(`git diff --stat`: `fle_clip.c` 3 行、`fle_input.c` 5 行)。
- テストの前: 失敗 42(`evidence/R0-6.md` の表。今回は再測定していない)。
- テストの後(2026-11-13、`cargo test --offline -p fe-lift --test wall_fr14 --test wall_fr19 --test wall_ia1w --no-fail-fast`、この機械): `wall_fr14` 26 passed・`wall_fr19` 13 passed・`wall_ia1w` 7 passed。失敗 0。

## 読み(解釈。実行していない)
- 両変数は、将来の診断のために置かれた計数器と読める。読まれないので、警告の原因になるだけで挙動には効かない。
- `-Wno-unused-but-set-global` で警告を黙らせる案は採らない。警告を黙らせると、読まれない処理が残る。

## 未確認
- `lift/out-wallFR18/` 配下の生成物(`.gitignore` の `lift/out*/`、追跡なし)には同名の変数が残る。今回の除去の対象外。
- テストが呼んだ clang の版は、`evidence/R0-6.md` の記述どおり未確認(`source scripts/env.sh` の後は 24.0.0git)。

## 出典
- `evidence/R0-6.md`(失敗の表)、`evidence/R0-6.log`(`set but not used` の行)
- `docs/ops/fidelity-and-performance.md` §2(分類)、`AGENTS.md` §6

## テストの入力パスの修正(`wall_fr3.rs`・`wall_fr4.rs`)の分類
- 対象: `lift/crates/fe-lift/tests/wall_fr3.rs:575` と `wall_fr4.rs:995` の `.env("LIFT", …)`。スクリプトの場所は `archive/harness-attic/a4-multi.sh`(R0-6b §4)。
- 出典(`grep -n 'LIFT' archive/harness-attic/a4-multi.sh`、2026-11-14 に実行):
  - 26 行目: `#        LIFT=<tree with crates/ runtime/ harness/>   (default: the lift/ above this script; a `git archive` tree works)`
  - 86 行目: `LIFT=${LIFT:-$(cd "$HERE/.." && pwd)}`
- 実測の注意: 86 行目の既定は「スクリプトの置き場の親」(`$HERE/..`)。`archive/harness-attic/` に在るスクリプトでは既定が `archive/` になり、`harness/fruity-rows.txt` が無いため使えない。そこでテストが子プロセスに `LIFT` = `lift/` を渡す。
- 分類: (b) 契約の実装に当たらない、テストの入力パスの修正。`LIFT` は `archive/harness-attic/a4-multi.sh` が既定値つきで読む既存のパラメータで、テストが子プロセスに渡す値を、移動先(`archive/`)のスクリプトの既定から `lift/` に直した。新しい環境変数ではない。製品(wasm)に入らない(テスト専用)。
- 撤回条件: スクリプトを `lift/harness/` に戻したら、この指定を外す。
