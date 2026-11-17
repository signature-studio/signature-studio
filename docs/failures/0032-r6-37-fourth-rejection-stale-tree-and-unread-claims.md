# 0032: R6-37 の 4 回目と 5 回目の差し戻し。取り込み前の古い木から書いた手順と、読まずに書いた「実行されない」「E 0 件」「基準 commit」

- 日付: 2026-11-17
- 誰が: worker(R6-37・R6-37b。作業の記録は `evidence/R6-37.md`、決定は `docs/decisions/0415-no-python-in-product.md`)
- 指摘: reviewer の検査(4 回目。基準 d1dda509。main の指示に引用)と、その後の 5 回目(main の指示に引用)。依頼者の発言ではない。

## 何が起きたか

4 回目の指摘(main の指示の番号どおり):

- (A1) `evidence/R6-37.md` §6 の deploy.yml・agent.yml の記述が origin/main と合わない。node の抽出と printf を deploy.yml の行として書いていた。runs-on/cache・fe-build extract・setup-emsdk@v16・wrangler-action を書いていなかった。古い木の版から書いた(取り込み前の 5f6cc8e6 より前の版)。
- (A2) 0415 と `evidence/R6-37.md` §1.1b が、archive の .py と archive の物を「実行されない」「どの test も build も読まない」と書いた。`lift/crates/fe-lift/tests/` の test が a4-multi.sh を実行し、vis1・web/opfs1・vst を読む・コピーすることを、テストのコードを読まずに書いた。
- (A3) 「E 0 件」を検査の結果として書いた。`classify2.sh` は E を代入しない(初期値の E は、必ず B・D・C・OTHER のどれかに上書きされる)。測定の結果ではなかった。
- (A4) 基準 commit を、古い sha(3f0277c9)のまま書いた。
- (A5) 入力データの生成元を、自分が書いた一覧(8 件)だけで数えた。deploy が読む閉包(83 件)を求めていなかった。reg-seed-nsis.tsv・iltools-6000000-seeds400.txt・hyb-U と lowered-*.txt・filter-hybrid-list.py・extract_nsis2.py・nsis_script_dump.py が抜けた。
- (A6) allowlist の「the four generators below are the provenance of those tables」と「filter-hybrid-list.py は a measurement input」を、記憶で書いた。lea_leaf_candidates.py は 4 表の来歴ではなく、filter-hybrid-list.py は lowered-*.txt の VA の集合の元になる hyb-U を書き換える。
- (K1) scan の py_comment が、shebang の行(`#!`)をコメントとして除外していた。`#!/usr/bin/env python3` を持つ、拡張子の無いファイルが検出されなかった。
- (K2) 行単位で許可した python3 - の heredoc の本体(Python の本体)が、検査されなかった。scripts/ は製品の範囲。
- (任意) §1.2.2 の確かめのコマンドが、fle_raster.c(18 行目)を出さなかった(先頭 6 行だけを見ていた)。

5 回目の指摘(main の指示の番号どおり):

- (a) 0415 の範囲の文と `evidence/R6-37.md` §1.1b の「どの test も build も読まない」は誤り。a4-multi.sh は wall_fr3.rs:575・616 と wall_fr4.rs:996・1035 が実行・読込する。vis1(wall_fr21.rs:414-415)・web/opfs1(wall_opfs1_map.rs:15)・vst(noreturn_audit.rs:69)も test が読む。
- (b) lowered-all.txt の path を、lists/ の下と書いていた。実体は lift/harness/hyb/lowered-all.txt(b2d2/gen.args:12 が読む)。
- (c) hyb-U の除去件数を「32 件」と書いた。各ファイルの注釈の合計は 66(fle 32・ilt 10・b2d 9・qfc 9・ft 3・dsp 2・oggio 1。elastique は件数の注釈が無い)。
- (d) hooks/*.txt の 7 件目(wrapper-hook-points.txt)の扱いを書いていなかった。
- (e) 「参照ありの 99 件のうち、範囲の外は 21 件」の一文の読み違え(範囲の内の参照なし 21 件と混ざる)。2 × 2 の表に直した。
- (f) check-consistency の移植前後の出力を、「IDENTICAL」の一文だけで示した。全文を貼っていなかった。
- (g) ROADMAP の t12 の件数(30)と、次の一手(3 回目の差し戻し)を、現状に直していなかった。
- (h) 4 回目の失態の記録がなかった(この記録)。0029 の原因文(コマンドの出力を貼らずに書いた)と README の文言(記憶で書いた)が揃っていなかった。
- (i) G1〜G5 の件ごとの根拠、os-fonts の 4 件と README・COPYRIGHT の扱い、ci/extract-dlls.tsv の削除済みの注記が無かった。

## 原因(その時の判断)

- A1: deploy.yml と agent.yml の steps を、取り込みの前に読んだ版の記憶で書いた。取り込み(git pull)の後に読み直さなかった。
- A2: archive の参照を、基底名の検索(`inventory2.tsv` の参照の列)だけで判定した。archive の path を名指す `git grep -n "archive/"` を、テストのコードに対して実行しなかった。「実行されない」を、参照の無さの言い換えとして書いた。
- A3: E の件数を、`classify2.sh` の分岐を読まずに、結果の表の項目として書いた。
- A4・A5: 基準 commit と閉包を、作業の途中の値(3f0277c9・自分の一覧)のまま書いた。閉包の計算をしていなかった。
- A6・K1・K2: 許可の来歴の文と検査の範囲を、記憶と既知の 1 例で書いた。shebang の行を含む入力・heredoc の本体を、規則のテストで確かめていなかった。
- 任意: 確かめのコマンドを、先頭の 6 行の制限つきで書いた。

## 影響

- 証跡・決定・ROADMAP・allowlist の文に、実物と合わない記述が 4 回の commit にわたって残った(01bc7124 まで)。製品・deploy・agent の動作への影響は無い。今回の commit は ci/extract-dlls.tsv と lift/harness/seeds/os-fonts/README.md に注記を足すため、deploy.yml の起動条件(ci/**・lift/**)に当たる。run の結果は push の後に確かめる(下の証跡 §5.4)。
- 規則の穴(shebang・heredoc)は、この木の追跡ファイルでは検出 0 件だった(`git ls-files -z | xargs -0 bash scripts/scan-forbidden.sh` の rc 0)。ただし、穴があったこと自体は事実。

## 是正

- 0415 の範囲と件数を、2 × 2 の表と、archive の名指し(a4-multi.sh・vst・vis1・opfs1)で書き直した。main の判定を 0415 に転記した。
- `evidence/R6-37.md` の §1.1b・§5.5・§6・§8・§11 を、コマンドの全文と出力で書き直した。§5.5 に check-consistency の出力の全文(実リポジトリと sandbox の 13 件)を貼った。
- scripts/ の 3 本の python を Node に移した(heredoc の node)。scripts/ に python は 0 件。
- scan の py_comment を直し、shebang の行を検査する。t12 に例(違反 1・許可 1)。
- hyb-U の除去件数を 8 ファイルの表で書き、閉包の外の wrapper-hook-points.txt の理由を書いた。
- ci/extract-dlls.tsv と os-fonts/README.md に、削除済みの注記を足した(呼び出しではない)。
- ROADMAP の R6-37 と R6-37b の t12 の件数と次の一手を、現状に直した。

## 同じ型の探索

- 範囲: (1) `git grep -n "archive/" -- lift/crates tests lift/harness lift/runtime ci app scripts .github`(archive を読む・実行する参照)。(2) `grep -n "c=E" <classify2.sh の全文>`(E の代入。証跡 §1.1 の全文)。(3) `git grep -n "extract_fl\|plan.sh" -- ci lift app docs`(削除済みの名前を現在形で書いた箇所)。(4) `grep -n "py_comment=" scripts/scan-forbidden.sh`(コメントの除外)。(5) 閉包の計算(`evidence/R6-37.md` §8.1 の全文)。
- 結果: (1) a4-multi.sh・vis1・opfs1・vst の 4 つを名指しした(evidence §1.1b)。(2) 代入は無く、E は出力されない。(3) ci/extract-dlls.tsv の 13・16 行と、os-fonts/README.md の 26 行。注記を足した。(4) shebang の除外の 1 件を直した。(5) 閉包 83 件。区分 G1〜G5。

## 再発防止

- 手順: 取り込みの後に、記録の手順・基準・件数を読み直し、コマンドの出力を貼る(0415 の決定 10 と `evidence/R6-37.md` §8)。
- 未実装: 閉包の計算の自動化は、tests/ に入っていない(証跡の §8.1 の script は手で実行した)。

## 未確認

- reviewer の次の検査の結果。
- a4-multi.sh の python3 への実行に届くか(テストが早期に拒否するため、未確認)。
