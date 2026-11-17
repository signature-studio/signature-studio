# 0029: R6-37 の証跡と規則が、3 回目の差し戻しまで、書いた数字と作った規則の実物と合わなかった

- 日付: 2026-11-17
- 誰が: worker(R6-37・R6-37b。作業の記録は `evidence/R6-37.md`)
- 指摘: reviewer の検査(3 回目の差し戻し。基準 commit d8db597a。main の指示に引用)。依頼者の発言ではない。依頼者の原文は `docs/failures/0026-python-in-production-path.md` に同じ。

## 何が起きたか

reviewer が指摘した 8 件(main の指示の番号どおり):

1. `evidence/R6-37.md` §5 の t12 の内訳「違反の例 14・許可の例 9」は、その木の t12 と合わない。`git show d378cb04:tests/harness/t12_python.sh | grep -c '^expect_not_contains'` の出力は 10、`grep -c '^expect_contains'` の出力は 10。
2. ROADMAP R6-37b の完了条件「git ls-files の *.py に lift/runtime/ で始まる行が無い」は満たせない。`lift/runtime/test/` の .py 4 件(gen_c1_ucd_expect.py・gen_c1_wine_data.py・gen_case_wctable_expect.py・gen_cp1252_expect.py)が残る(`git ls-files lift/runtime | grep '\.py$'` の出力)。
3. `scripts/forbidden-allowlist.txt` の `lift/harness/wasm/` の dir 許可は、deploy の入力 `lift/harness/wasm/trace_fruity.c`(`lift/harness/two/harness-inputs.txt` の 5 行目が読む)を含み、基準に反した。理由の空の非 .py の 6 件: lift/harness/test.sh・two/build_two_native.sh・two/chain.sh・two/gen_two.sh・web/threads/compare_scen.sh・scripts/forbidden-allowlist.txt。`tests/harness/t12_python.sh` のヘッダーの「t12 が workflow を検査する」は、workflow 本文の python 行だけを見ることを正確に書いていなかった(許可された .sh を呼ぶ行は検出しない)。
4. scan の単語規則は、`/usr/bin/python3`(直前がスラッシュ)、`python3.11`、行末のバックスラッシュ続き、大文字の `PYTHON3` を検出しなかった(`scripts/scan-forbidden.sh` の旧 `py_word` の正規表現。区切りの文字の集合が `/` と `\` と `-` を含んでいた)。
5. `evidence/R6-37.md` §1 の出典の script は、git の無視対象の作業用の出力先にあり、ワークツリーに無い(script の名は書かない)。コマンドの全文を書いていなかった。
6. 報告に、検査の基準にした commit を書いていなかった。
7. `docs/decisions/0415-no-python-in-product.md` に、製品の入力データの生成元(`tools/authenticode_check.py`・`lift/harness/seeds` の .py 4 件)の扱いが無かった。
8. 0415 に、範囲の外の .py の件数と、範囲の内の許可の理由の節が無かった。

## 原因(その時の判断)

- 1: 数字を、コマンドの出力を貼らずに書いた。
- 2: R6-37b の完了条件を、lift/runtime の .py が全部消える想定で書き、`lift/runtime/test/` の残りを `git ls-files` で確かめなかった。
- 3: wasm を dir 単位に括るとき、その dir に deploy の入力があるかを、`two/harness-inputs.txt` の参照で確かめなかった。非 .py の全体許可に理由の行を付けなかった。t12 のヘッダーは、検査の範囲を言い切る書き方のまま残した。
- 4: 単語規則を、既知の 1 例(`python3` の直後が区切り記号)だけで作った。変種(パスの前置・版番号・行末の `\`・大文字)の検査例を t12 に置かなかった。
- 5: 証跡の根拠を、無視対象の作業物(作業用の出力先)に置いた。ワークツリーに残らない物を出典にした。
- 6・7・8: 差し戻しの項目を、決定と報告の必要な範囲として数えなかった。

## 影響

- 証跡と ROADMAP の数字・完了条件が、実物と合わない状態で commit 69525d0d まで残った(evidence §5・§1、ROADMAP の R6-37b の行)。
- 規則の穴: 増えた検出の確かめ(`evidence/R6-37.md` §5.1)で、新しい規則が増やした検出は、許可の中の prose の 8 件だけだった。この木の追跡ファイルに、穴の形の呼び出しは無かった(`git grep -n -i python -- .github` の出力は 0 件)。deploy の起動条件(`app/**` `lift/**` `ci/**` `deploy.yml`)に、今回の変更は触れない。

## 是正

- scan の単語規則を直した(`scripts/scan-forbidden.sh`)。大文字小文字を問わず、`/usr/bin/python3`・`python3.11`・行末の `\` を検出する。C 文字列の `python\\` と、ホスト名・識別子は検出しない。
- t12 に、違反の例 14 と許可の例 12 と、rc・件数の確かめ 4 を置いた(計 30。`tests/harness/t12_python.sh`)。ヘッダーを、検査する範囲と検査しない範囲に書き直した。
- wasm を file 単位にした(split_c.py と build.sh の 1 行)。.sh の python 行は行単位にし、理由の行を付けた。
- ROADMAP R6-37b の完了条件を、9 件の生成元の `git ls-files` が 0 行であることに改めた。
- 0415 に §10(入力データの生成元の扱い)と §範囲と件数を足した。証跡 §1・§5・§8 を、コマンドの全文と出力に直した。

## 同じ型の探索

- 範囲: (a) docs・evidence・tests の README・ROADMAP の、件数と完了条件の文。(b) 許可リストの dir 単位の全数(13 件)。(c) 単語規則の変種(許可を外した版の raw の比較)。(d) 理由の空の許可。
- 方法と出典: (a) `git grep -n "違反の例\|許可の例" -- docs evidence scripts tests`、`git grep -n "out-pyrm" -- ':!scripts'`、`git grep -n "lift/runtime/ で始まる" -- docs evidence`。(b) 証跡 §8 のコマンド。(c) 証跡 §5.1 のコマンド。(d) 許可リストの非 .py の全体許可の行。
- 結果: (a) out-pyrm の参照は `evidence/R6-37.md` の 1 件(§1 を書き直す)。違反の例の数の文は同じ証跡の 1 件(§5 を書き直す)。R6-37b の完了条件の文は ROADMAP の 1 件(書き直す)。(b) dir 単位 13 件のうち、deploy の入力を含むのは 0 件(§8 の確かめ)。wasm は dir 単位から外して file 単位にした。(c) 増えた検出は 8 件、すべて prose で許可の中。(d) 非 .py の全体許可は 6 件(上の 3)。

## 再発防止

- 検査: t12 に、新しい形の違反と、誤検出にならない許可例を置いた。
- 未実装: dir 単位の許可が deploy の入力を含まないことを、スクリプトで検査する仕組みは無い。`evidence/R6-37.md` §8 の手順は文書である。自動化は未了。

## 未確認

- 範囲の外と範囲の内の検証の道具の扱い(0415 の §範囲と件数)について、依頼者の判定。
- reviewer の次の検査の結果。
