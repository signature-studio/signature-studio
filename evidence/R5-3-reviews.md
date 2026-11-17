# R5-3 の reviewer の検査の報告(保存)

※ 完了ではない: P2 の 120 fps は未確認、P3 は音切れが残る、O9 は未特定、依頼者機の値は 0 件。この記録は R5-3 の reviewer の検査の報告のコピーで、機能の進捗ではない。

0 回目(最初の検査の報告)は `evidence/R5-3l-review.md` に保存済み。ここにはコピーしない(名前だけを指す)。

## 保存の方法

- 各 reviewer のセッションの最後の assistant の発言を、`~/.agents/skills/opencode-send-message/scripts/oc-msg.sh last <セッション ID>` で取り出した。出力は作業用の一時ファイルに保存してから読んだ。
- 7 件すべてで取り出せた。空の出力も途中の発言も無かったため、`oc-msg.sh log` による探し直しは行っていない。取り出せなかった件: 0。
- 検査の順に、節「## 検査 N」の下へ全文を置いた。報告の中の見出し(`##` で始まる行)も、そのまま残した。
- 報告の本文は要約していない。書き換えたのは、下の「置き換えの記録」の箇所だけ。

## 置き換えの記録

- 禁止語の置き換え: scan-forbidden の term の規則に当たる語 1 種を「ワークツリー」に置き換えた。3 箇所(検査 1・5・6)。
- セッション ID・絶対パス: 0 箇所。
- 検索語として報告の中に書かれた `/home/` の文字列(grep の対象の語。絶対パスではない)は置き換えていない。5 箇所(検査 1・2・3・4・6)。
- 同じく検索語として書かれた `ses_` の文字列(ID ではない)は、置き換えていない。
- 置き換えの前後で、判定と数字は変えていない。

## 検査 1(再検査。R5-3 を todo に戻すきっかけ。判定: 未確認、差し戻し 1)

## 判定: 未確認

`done` の根拠は、この検査では確定できませんでした。シェルの git 系コマンドが拒否され、変更範囲のうち `d2d90a9` の stat、seed の履歴、テストの差分を読めませんでした。

- **差し戻し 1 件(表示の不一致)**: 51 行目は状態が `done` ですが、括弧書きに「reviewer の検査は…総合は『未確認』」と書かれています。`evidence/R5-3l-review.md` の 9 行目と 18 行目も「`done` にしてはいけない(現時点)」としています。`done` の定義(reviewer が受理)と合っていません。
- **一致した項目**: 項目 1(numstat)、項目 2 の列数、②③⑤⑥⑦⑧、項目 4(2426・1・1)、項目 5、項目 6 の禁止文の検査。
- **未確認**: `d2d90a9` の stat、seed の git 履歴(`14268c2` が 1 件かどうか)、テストの期待値の値が変わっていないこと、`main が git で確かめた` という記述。

## 項目ごとの結果

**1. 変更範囲**: `git show --numstat --format=%h e898d7f` の出力は次のとおりです。`docs/ROADMAP.md` 1・1、`evidence/R5-3l-review.md` 27・0(新規)、`evidence/R5-3l.md` 2・0。報告どおりで、余計なファイルはありません。`git show --stat d2d90a9` は拒否されたため未確認です。

**2. ROADMAP 51 行目**
- (a) `^([^|]*\|){10}[^|]*$` で 51 行目と 52 行目がともに一致しました。どちらも縦棒 10 本で 9 列です。括弧書きに `|` はありません。

| 主張 | 出典 | 判定 |
|---|---|---|
| ① E13 の About に画が出ず、work-r25 と work-r53k に出る | `evidence/R5-3/r53e13-chrome-2/about-28s.png`(開いた。歯車だけ、ロゴ・staff roll なし)、`evidence/R5-3/r53wr25-chrome-1/about-28s.png`(開いた。ロゴと開発者列あり)、`evidence/R5-3l/r53knew-chrome-1/about-28s.png`(開いた。ロゴと開発者列あり) | 一致(各 1 枚の目視) |
| ② O16 は未観測 | `evidence/R5-3/r53e13-chrome-2/ja-main-20s.png`(開いた。日本語なし) | 一致 |
| ③ seed の 24・240・241 行に 3 ファイル | `lift/harness/seeds/vfs-seed-idle1.tsv`(grep で 3 行確認) | 一致 |
| ④ この clone に既にあり追加していない。`git log` は取り込みの 1 件 | `git log` が拒否された | 未確認 |
| ⑤ msgothic.ttc は Noto Sans CJK JP の subset(OFL)、出典と分類は 0369 | `docs/decisions/0369-seed-assets-about-ja.md` 3・16〜17 行(分類 b、OFL、subset)。COPYRIGHT ファイルは未読 | 一致(0369 の記述) |
| ⑥ 0372・0375 は分類 (b)、撤回条件あり | 0372 の 2・3 行、0375 の 2・3 行 | 一致 |
| ⑦ r53h・r53k は各 n=3 止まらず、r25・r53j は 3/3 停止 | `stopped` と `state` を grep で数えた。r53k(`R5-3l/r53knew-chrome-{1,2,3}`): 3 件とも `stopped: false`、`engine-idle`。r53h(`R5-3i/r53-r53hnew-chrome-{1,2,3}`): 3 件とも false。r25(`R5-3l/r53kold-chrome-{1,2,3}`): 3 件とも `stopped: true`、`exited`、code 134。r53j(`R5-3j/r53jold-chrome-{1,2,3}`): 3 件とも true。r53j の new 側も 3 件 false | 一致 |
| ⑧ 括弧書きに肯定の判定語・原因の断定がない | 51 行目で `直った` と `達成` が各 1 回。どちらも「O3・O16 が直った・P1〜P3 の達成の意味ではない」の否定文脈。`効いた`・`問題ない`・`整合`・`到達` は 0 件 | 否定のみ(肯定 0) |

- 「台本 A」の定義は 0373・0374 を読んでおらず、未確認です。

**3. 実装の差分(0372)**
- `lift/runtime/fle_ole.c` 94〜105 行: `fle_imp_CoInitializeEx` は `(uint32_t)R[2] != 2` のとき `fle_stop`。それ以外は `com_init(api, 0)` を返します。`com_init` は 53〜64 行で、`R[1] != 0` のとき停止します。0372 の記述(分類 b。`dwCoInit` が 2 のときだけ `com_init` と同じ答え)と一致します。
- `lift/crates/fe-lift/src/imports_table.rs` 776〜777 行: 0372 の行が 1 つあります。
- テストの期待値: `wall_fr4.rs`・`wall_fr20.rs` などに `mod rows_0372` と `rows_0372::ROWS_0372.len()` の項が足されています(数え方の 1 項)。取り込み前の数字との差は、`git diff` が拒否されたため未確認です。
- `d2d90a9` の stat: 未確認。

**4. 保存物(`evidence/R5-3k-cargo.txt`)**
- `test result:` の行は 219 行(26〜244 行)です。全部足すと、通過 2426・失敗 1・無視 1 で、記録と一致します。25 行目までと 26〜75 行の小計(425)も確かめました。
- FAILED の名前は `getmodulehandleexw_returns_in_every_build` だけです(18・19・20 行)。
- この cargo は取り込み後・commit 前のワークツリーで実行されています。commit 後の cargo は未実行です。

**5. `evidence/R5-3l-review.md` と `evidence/R5-3l.md`**
- `ses_`・`/home/`・`token|secret|password`・32 桁の hex を grep しました。ヒットは、commit の SHA(R5-3l.md 132 行)、sha256(140・141 行)、`actions/checkout@v4`、reviewer 自身の文の語だけです。秘密や ID はありません。
- R5-3l.md の 1 行目に注があります。R5-3l-review.md の 3 行目に、判定語の列挙を置き換えた改変の旨があります。
- 報告の原文との照合は、原文がないため未確認です。

**6. 報告の誠実性(51 行目)**
- (a) P1〜P3 の達成と読める語: なし(否定文脈のみ)。
- (b) 依頼者機の値・実機との比較の誤読: なし。「実機との比較は 0 件」と明記されています。
- (c) 出典のない原因の断定: なし。「原因は未確認」と書かれています。
- 出典未確認の断定が 3 件あります(本検査で確かめられなかったもの): 「この clone に既にあり、追加していない」、「`git log` の履歴は取り込みの 1 件」、「main は git で確かめた」。

## 不一致の一覧
| 項目 | ROADMAP の記述 | 実測 | 実行したコマンド |
|---|---|---|---|
| 状態と本文 | 状態 `done`。本文に「reviewer の検査…総合は『未確認』」 | reviewer は「done にしてはいけない(現時点)」 | `evidence/R5-3l-review.md` の 9・18 行を読んだ |

内容の数字の不一致はありませんでした。

## 差し戻しの指示案(追加のみ)
1. git が使えるセッションで次を実行し、出力を `evidence/R5-3l-review2.md` に追記する: `git show --stat d2d90a9`、`git log --oneline -- lift/harness/seeds/vfs-seed-idle1.tsv`、`git show --stat 14268c2`、`git diff -U1 a03c903 HEAD -- lift/crates/fe-lift/tests/wall_fr4.rs lift/crates/fe-lift/tests/wall_fr20.rs`。
2. 追記の結果を reviewer が読み、受理か差し戻しを新たに書く。それまで 51 行目の状態を `done` にしておかない(`todo` へ戻すか、受理の記録を足すまで)。
3. 括弧書きの「main が git で確かめた」は、上の出力の場所を書く形に直す。

## 検査 2(2 回目。差し戻し 2、未確認 1。0372 のテストの値と注)

## 判定

R5-3 を `done` にしてよいかの判定は **未確認**です。`done` にしてよいとは判定できません。

- **受理**: 項目 1、項目 2(変更範囲)、項目 3(seed の履歴)、項目 4(テストの期待値の式と定数。テストの実行結果は除く)、項目 7(b)、項目 8。
- **差し戻し 2 件**: (a) テスト `coinitex-speed-over-memory` の値が決定 0372 の「2|8」(= 10)と違い、値 8 になっている。(b) 決定 0372 末尾の注が「`rows_0372/mod.rs` は main に無い」と書いており、実測と違う(決定 0375 の未確認 28 行が自認している)。
- **未確認 1 件**: 決定 0375 の撤回条件のうち cargo の項。取り込み後の commit で `cargo test` が未実行です。done の可否に影響します(理由は項目 6)。
- **ROADMAP 51 行目**: 括弧書きの 1 文に追加の更新が要ります(項目 7)。

reviewer はこの検査で `cargo`・`npm`・`scripts/*`・`git ls-remote` を実行していません。ファイルの変更もしていません。`git merge-base --is-ancestor` は `permission.rejected` で拒否されました。同じ内容を後述の `git log` で確かめました。

## 項目ごとの結果

**1. 保存物 — 受理**
- `ls -la evidence/R5-3l-review2/`: 6 ファイル、51〜8404 B、すべて非空。
- `evidence/R5-3l-review2.md` を read で読んだ。記載の行数(15・1・4・7・66・76)は各ファイルの行数と一致。

**2. (i) 変更範囲 — 受理**
- `d2d90a9-stat.txt`: 12 ファイル、110 行追加、8 行削除。
- 12 ファイルすべてが `lift/` 配下(`lift/` 外は 0 件)。名前は記録の 12 件と一致。
- 生データの照合: `git show --stat --format=%h%x20%s d2d90a9` と `git show --name-only --format= d2d90a9` が保存物と一致。

**3. (ii) seed の履歴 — 受理(「既にあり、追加していない」は言える)**
- `git log --oneline -- lift/harness/seeds/vfs-seed-idle1.tsv`: 1 件(`14268c2`)。
- `git log --oneline d2d90a9..14268c2`: 出力 0 行。よって 14268c2 は d2d90a9 の祖先。
- seed TSV の grep: 24 行 `BigFruit.png`、240 行 `AboutTxt.txt`、241 行 `msgothic.ttc`。
- `git status --short -- lift/harness/seeds/vfs-seed-idle1.tsv`: 出力 0 行(作業ワークツリーと HEAD が一致)。
- 注: 「取り込み」の中身は commit の件名だけで確かめた。

**4. (iii) テストの期待値 — 受理(テキストの検査。テストの実行は未確認)**
- numstat: 7 ファイル、15 追加・8 削除。
- `wall-fr-diff-u0.txt`(66 行)の grep: `^-[^-]` 8 行、`^\+[^+]` 15 行、`^\+mod rows_0372;$` 7 行。
- `+` 行のうち `rows_0372` を含まない行: 0 件(`^\+[^+].*rows_0372` も 15 件)。
- 8 対の突き合わせ: `+` 側から `- rows_0372::ROWS_0372.len()` を除くと、`-` 側のコード部分と 8 対とも一致。
- 数値の定数は両側で同じ: `9 + 15 + 73 + 2 + 33 + 4`(wall_fr4)、`8 + 15 + 73 + 2 + 33 + 4`(wall_fr5)、`rows.len() - 10`(wall_fr13)。
- wall_fr20 の 2 つ目のハンク(48 行): `table[table.len() - 1 - rows_0267… - rows_0372…]`。記録どおり 1 項追加。
- `rows_0372/mod.rs` 7 行目: `ROWS_0372: [(&str, &str); 1]`。追加項の値は 1。

**5. 0372 のテスト(`oleaut-ole-diff-u0.txt`、76 行)— 差し戻し 1 件(値 8)**

| テスト | 内容 | 決定 0372 との対応 |
|---|---|---|
| `co_initialize_ex_apartment_threaded_s_ok_then_s_false_and_balance` | 0,2 → S_OK depth 1、再度 → S_FALSE depth 2、CoInitialize → S_FALSE depth 3、Uninitialize 3 回で 2,1,0、再初期化で S_OK | 範囲 4 の「S_OK・S_FALSE・com_depth・釣り合い」。一致 |
| `co_initialize_ex_reads_only_the_low_32_bits_of_dword_argument` | 0x100000002 → S_OK | 範囲 4 の「上位 32 ビットを読まない」。一致 |
| `co_initialize_ex_inputs_the_pages_leave_open_abort` | 停止 5 件: pvReserved=1、値 0、値 6(2\|4)、値 **8**、CoInitializeEx(2) の後に OleUninitialize | pvReserved と STA 以外の停止は一致。値 8 は 2\|8(= 10)ではない(不一致)。OleUninitialize の件は決定の文言に無い(既存の `fle_ole.c` 84〜92 行の停止を固定) |
| `co_initialize_ex_row_is_in_the_table_and_declared_in_the_header` | 表の行が 1 件、`fle_ole.h` に宣言 | 範囲 4 の「行と宣言が 1 つ」。`imports_table.rs` 777 行と `fle_ole.h` 21 行で確認。一致 |

- 実装側の確認(読み): `fle_ole.c` 103 行で `(uint32_t)R[2] != 2` のとき停止、58 行で `R[1] != 0` のとき停止。テストの期待とは読みで一致。テストは未実行。
- 限界: 「2 以外は名前つき停止」は値 0・6・8 の 3 点の標本でしか検査していない。
- 不一致: 決定 0372 の 28 行は「2|4・2|8」を 2 以外として停止と書く。`fle_ole.c` 97 行は「10 (2|COINIT_SPEED_OVER_MEMORY)」と書く。テストの 1257 行は `CALL(fle_imp_CoInitializeEx, 0, 8);`。

**6. 実行の根拠 — 数は受理(commit 前の記録として)。commit 後は未確認**
- `evidence/R5-3k-cargo.txt` の `test result:` 行: 本文 219 行(grep の 26〜244 行。20 行目は要約の抜粋)。記録と一致。
- 失敗: 193 行(ログ 3554 行)`FAILED. 1 passed; 1 failed`。失敗は `getmodulehandleexw_returns_in_every_build`(`wall_wrp2.rs:71`)。取り込み前から残る 1 件(`evidence/R1-2.md` 4・80・93 行)。
- 無視: 231 行(ログ 3992 行)`1 ignored`。
- 通過: 219 行を読み取って手で加算すると 2426。記録と一致(自動集計はしていない)。
- commit 後の cargo: 未実行。根拠は `evidence/R5-3l-review.md` 13 行(「取り込み後の commit での cargo test は未実行です」)と、`evidence/R5-3l.md` に cargo の記録が無いこと(grep 0 件)。
- lift は d2d90a9 から変わっていない(`git diff --stat d2d90a9 HEAD -- lift`: 出力 0 行)。ただし commit 前のどの木で cargo が走ったかの記録(時刻・木の識別子)は無い。
- engine-bundle: 9610708(lift は d2d90a9 と同じ)の run `43687019712` は success(`evidence/R5-3l.md` 123・132 行)。d2d90a9 の push による run は一覧に無い(同 130・194 行)。
- 台本 A: work-r53k(9610708)は 3/3 止まらず(`evidence/R5-3l.md` 148 行)。
- **done への影響: あり。** 決定 0375 は撤回条件を「取り込み後の main の HEAD」で測ると定めている。cargo はその HEAD で測られていない。lift が同じでも、測定の記録が無い。ROADMAP の括弧書きはこの点を書いていない。reviewer は cargo を実行できないため、worker に HEAD で実行させ、結果を保存させる必要がある。

**7. ROADMAP 51 行目 — (a) 更新が要る、(b) 受理**
- (a-1) 「seed の行は…追加していない(`git log` の履歴は取り込みの 1 件)」: 項目 3 で言えると確認。更新は不要。
- (a-2) 「reviewer の検査は…総合は『未確認』。その部分(変更範囲・実装の差分・seed の履歴)は main が git で確かめた」: 更新が要る。括弧書きに (iii) テストの期待値が無く、「実装の差分」が (iii) を指すとも読めない。2 回目の検査の結果も書かれていない。
- (b) 括弧書きの語: 「直った」1 件と「達成」1 件は、いずれも「O3・O16 が直った・P1〜P3 の達成の意味ではない」の否定。「効いた」「整合」「到達」は 0 件。原因の断定は無く、「原因は未確認」と書かれている。

**8. 秘密・ID — 受理**
- `evidence/R5-3l-review2.md` と `evidence/R5-3l-review2/` の 6 ファイルで、`ses_[A-Za-z0-9]|/home/|[A-Za-z0-9]{32,}|gh[pousr]_|AKIA` を検索: 0 件。
- 下線を含む長い語の検索では 13 件が当たったが、Rust のテスト関数名と diff の hunk 見出しだった(開いて確認)。

## 不一致の一覧

| # | 項目 | 記録の記述 | 実測 | 実行したコマンド・開いた箇所 |
|---|---|---|---|---|
| 1 | 5 | 決定 0372 28 行: 「2\|4・2\|8」は 2 以外で停止 | テストの値 8(2\|8 は 10) | grep `coinitex-speed` の `oleaut_ole.rs` 1257 行。保存物 63 行 |
| 2 | 0372 の注 | 決定 0372 54 行: `rows_0372/mod.rs` は main に無い | 存在する(d2d90a9 で追加。stat に含む) | read `rows_0372/mod.rs` 7 行 |
| 3 | 6(未確認) | 決定 0375 の撤回条件(c)を HEAD で測る | commit 後の cargo 未実行 | `R5-3l-review.md` 13 行、grep `cargo` `R5-3l.md`(0 件) |
| 4 | 7 | ROADMAP 51 行目 (a-2) | (iii) が無い。2 回目の検査が無い | read ROADMAP 51 行 |

## 出典のない断定の数

**0 件。** 上の断定はすべて、コマンドの出力・開いたファイルの行・記録の行を併記。推定は「未確認」と書いた。

## done にするための追加の文(両方の差し戻しと cargo の記録が閉じてから書く)

ROADMAP 51 行目の末尾に、次を追加してください(1 文ずつ、`|` は含まない)。

> 追記: 2 回目の reviewer の検査(`evidence/R5-3l-review2.md`)は、未確認だった 3 点(commit `d2d90a9` の変更範囲・seed の履歴・テストの期待値)を、保存した git の出力(`evidence/R5-3l-review2/`)で検査し、一致を確かめた。取り込み後の commit での cargo test の記録(`evidence/<記録のパス>`)と、テストの値の修正(commit `<ハッシュ>`)を追加した。

閉じるまでの作業(worker への依頼):
1. テスト `coinitex-speed-over-memory` の値を 8 から 10 に直す(決定 0372 の 2|8 と一致させる)。
2. 決定 0372 末尾の注に、「`rows_0372/mod.rs` は取り込み後に main にある」を追加する(追加のみ)。
3. 直した後の HEAD で `cargo test --workspace --no-fail-fast` を実行し、結果の要約を保存する(通過・失敗・無視の合計と、失敗名)。

## 検査 3(3 回目。判定: 未確認、差し戻し 0。前提 2 件)

## 1. 判定

R5-3 を `done` にする判定は **未確認**(差し戻し 0 件)。`done` の前提が揃うまで `todo` のままにしてください。

- **受理**: 項目 1・2・3・5・8。差し戻し 1(テストの値)と差し戻し 2(0372 の注)は、記録と実物が一致します。
- **未確認**: 項目 4(0372 の 4 テストの個別の `ok` 行が R5-3m-cargo.txt に無い。実行前の git 状態は記述のみ)と、項目 6(commit A の engine-bundle run `43780207694` の結論)。
- **不一致 1 件(項目 3 の範囲外)**: 0372 本文 4 行目「main には入れていない」は、取り込み後の状態と合いません。
- **`done` の前提にする 2 件**(worker に依頼):
  - (a) `gh run view 43780207694 --json status,conclusion,headSha` を実行し、status が completed、conclusion が success、headSha が `123deac3c1c6cff9551170cb3959c2d86b22f900` であることを出力で示す。出力は evidence に保存する。
  - (b) 4 テストの `ok` 行を保存する。TMPDIR のログが残っていれば grep する。無ければ `cargo test -p fe-lift --test oleaut_ole` を実行して保存する。
- **根拠**: 0375 の撤回条件 (4) は「engine-bundle.yml が失敗する」です。commit A の run が失敗なら取り込みを外すことになるため、その結論は前提です。
- **注意**: `assert_aborts` は停止の有無だけを見ます。8 でも 10 でも `!= 2` で停止し、判定は同じです(読み)。cargo の結果は、この修正の効果の証拠にはなりません。

## 2. 項目別

**項目 1 変更範囲: 受理**
- `git show --numstat --format=%h 123deac`: `1 1 lift/crates/fe-lift/tests/oleaut_ole.rs`。
- `git show --numstat --format=%h 813bc1f`: `2 0 docs/decisions/0372-coinitializeex-response.md`、`269 0 evidence/R5-3m-cargo.txt`、`64 0 evidence/R5-3m.md`。
- `git diff -U0 123deac~1 123deac`: `@@ -1257 +1257 @@` の 1 hunk、`8` → `10`。
- shell の `git -C …` は拒否されました。同じ内容を `workdir` 指定で再実行しています。

**項目 2 差し戻し 1: 受理(文字列)**
- `oleaut_ole.rs` 1256 行は `coinitex-disable-ole1dde` が `6`(2|4)、1257 行は `coinitex-speed-over-memory` が `10`(2|8)。
- 0372 の 26 行は COINIT 値、28 行は「`2|4`・`2|8` … 2 以外を名前つき停止」。`fle_ole.c` の 96–98 行は「6 (2|COINIT_DISABLE_OLE1DDE) and 10 (2|COINIT_SPEED_OVER_MEMORY)」、103 行は `if ((uint32_t)R[2] != 2) fle_stop(...)`。一致します。
- `assert_aborts`(189–198 行)は、停止・SIGABRT・`fle:`・`returned` の無しだけを見ます。10 は停止の期待と一致します。

**項目 3 差し戻し 2: 受理(注)**
- `git show 813bc1f -- docs/decisions/0372-…md`: `@@ -52,3 +52,5 @@`。追加 2 行のみ。54 行の 2026-11-15 の注は無変更です。
- 56 行の注の 3 点はいずれも実物と一致します。
  - `rows_0372/mod.rs`: `git show --stat d2d90a9` に `lift/crates/fe-lift/tests/rows_0372/mod.rs | 7 +++`。`git log --oneline -n 200` で d2d90a9 と 9610708 は main の履歴内。glob でファイルの存在を確認しました。
  - テストの値 8→10 の commit A `123deac`: `git log` の 2 件目。
  - `evidence/R5-3m-cargo.txt`: 存在します。

**項目 4 差し戻し 1 の cargo 記録: 未確認**
- (a) 先頭行: HEAD `123deac3c1c6cff9551170cb3959c2d86b22f900` は `git log -n 1 --format=%H 123deac` と一致します。終了コード 101 と UTC 00:25–00:29 は記述のみです(ログ本体はリポジトリに無い)。
- (b) `grep '^[0-9]+:test result:'` は 220 件。内訳は本体 219 件(46〜264 行)と、失敗抜粋 36 行の重複 1 件。記録の 219 は本体と一致します。
- (c) 本体 219 行を 10 行ずつ 22 組に小計し、22 個を足しました(最後の組は 9 行)。通過 2426、失敗 1(213 行)、無視 1(251 行)。記録と一致します。
- (d) `... FAILED` は 28 行の `getmodulehandleexw_returns_in_every_build` の 1 件のみ。一致します。
  - 0372 の 4 テストの `ok`: `grep co_initialize_ex_` は R5-3m-cargo.txt で 0 件。未確認です。
  - 間接の材料: 記録の抜粋で `error: 1 target failed` の対象は wall_wrp2 の 1 件だけなので、oleaut_ole は失敗していないと読めます。個別の `ok` は R5-3h-cargo.txt(別の HEAD の古い記録)にのみあります。
- (e) R5-3k-cargo.txt: 219 行の通過数の列を全行照合しました。列は同一で、合計 2426・1・1 も同じです。失敗名も同じ(18・20・193 行)。
- (f) 実行前の `git status --short -- lift` が空、`git rev-parse HEAD` が A、の記述は出力の貼り付けが無く、未確認です。`git rev-parse HEAD` は shell で拒否されました。現在の HEAD は `git log -n 1 --format=%H` で 813bc1f(commit B)。
- (g) `git diff --stat 123deac HEAD -- lift` と `git status --short -- lift` は、どちらも空です。一致します。

**項目 5 R5-3m.md: 受理**
- 3 行目「※ 完了ではない」あり。51 行「実エンジンでの実行はしていない」、52 行「engine-bundle の run の結果は待っていない」あり。59 行「本記録に判定は書かない」。
- 禁止語 `効いた|直った|達成|問題ない|整合|到達` は R5-3m.md・R5-3m-cargo.txt・0372 のいずれも 0 件。
- 判定語の出現は、5 行(前回の判定の引用)と 57・59 行(見出しと否定文)のみです。

**項目 6 0375 の撤回条件: 未確認(commit A の run)**

| 条件 | 記録からの読み | 出典 |
|---|---|---|
| (1) 同位置の停止 | 9610708 の bundle(work-r53k)は 3/3 止まらず。commit A の bundle は未観測 | R5-3l.md 148・151 行 |
| (2) 別の停止 | 0 件 | R5-3l.md 153 行 |
| (3) cargo の新しい失敗 | 既知の 1 件のみ | R5-3m-cargo.txt 28・213 行 |
| (4) engine-bundle.yml の失敗 | 9610708 の run `43687019712` は success(記録の引用。reviewer は確認不可)。commit A の run `43780207694` は、リポジトリに ID が無く結論も無い → 未確認 | R5-3l.md 123・132 行、0376 の 61 行 |

- 観測の HEAD `9610708` から commit A までの製品の差: `git diff --stat 9610708 123deac -- lift app ci` は 1 file(テスト 1 行)のみです。
- トリガー: `.github/workflows/engine-bundle.yml` の `on.push.paths` に `lift/crates/**` があるので、commit A は run を起こします。commit B(docs と evidence のみ)は、列挙された paths に当たりません(読み)。
- R5-3m.md は run を引用していません(引用は R5-3l.md と 0376 のみ)。
- **`done` への影響: ある。** (4) は失敗なら取り込みを外すと定めています。commit A の run が失敗なら `done` は不可です。
- 追加の未確認: AGENTS §4 が求める push の到達の確認(`git ls-remote origin main` と `git rev-parse HEAD` の出力)が R5-3m に無いです。reviewer は ls-remote を実行できません。

**項目 7 括弧書きの案: 次の 5 を参照**

**項目 8 秘密・ID: 受理**
- `ses_|/home/|[A-Za-z0-9]{32,}` の検索で、R5-3m.md の 9・33 行と cargo の 4 行は commit A の sha(40 字)のみでした。0372 は 0 件です。`/home/` は無いです。

## 3. 不一致の一覧

| 項目 | 記録の記述 | 実測 | 実行したコマンド |
|---|---|---|---|
| 3 | 0372 本文 4 行目「main には入れていない」 | 取り込み後は main に有る(d2d90a9 と 9610708 は履歴内) | read(0372 の 4 行目)、`git log --oneline -n 200` |
| 4(d) | R5-3m.md 41 行「ok(ログの行 1792)」 | cargo 記録に個別の `ok` 行が無い | grep `co_initialize_ex_` → 0 件 |
| 6 | commit A の run `43780207694` の結論 | 記録に ID も結論も無い | grep 全体 → 0 件 |

項目 4(b) は不一致ではありません(本体 219、grep 全体 220。重複 1 件は失敗抜粋)。

## 4. 出典のない断定: 7 件

リポジトリ外のログか、貼り付けの無い出力に依るものです。
1. R5-3m.md 5 行: 前回の検査の判定の内容。
2. R5-3m.md 33 行・cargo 4 行: 実行前の `git status`・`git rev-parse HEAD` の結果。
3. R5-3m.md 41 行: 0372 テストの `ok`(ログの行 1792)。
4. R5-3m.md 52 行: commit A の run が進行中。
5. R5-3m.md 54 行: 「今回が初回の記録」。
6. cargo 267 行: ignored の行 3987。
7. cargo 5 行: 終了コード 101 と実行日時。

## 5. `done` にするときに追加する括弧書きの案(追加のみ。`|` を含まない)

> (追記 2026-11-16: 2 回目の検査の差し戻し 2 件に対応した(テストの値を 10 に直した commit `123deac`、0372 の取り込み後の注 commit `813bc1f`)。取り込み後の HEAD の cargo test の全保存は `evidence/R5-3m-cargo.txt`(既知の 1 件のみ)で、3 回目の検査の判定に従う。残る未確認: O16 の日本語の字形・原因・実機・commit `123deac` の engine-bundle の run の結論、0372 の 4 テストの個別の結果(個別の ok 行を保存できれば外す)。)

## 検査 4(4 回目。差し戻し 1。R5-3n.md 38 行)

## 1. 判定

- **R5-3 を `done` にしてよいか: 未確認**(項目 3 (iv) と項目 4 の一部が閉じない)。
- **差し戻し 1 件**: `evidence/R5-3n.md` 38 行の「全記録は `evidence/R5-3n-oleaut-ole.txt`」は誤り。このファイルは grep の抜粋で、全出力は一時ファイルにあり、リポジトリには無い(同ファイル 2 行目が自分でそう書いている)。
- 撤回条件 (4)(0375)は、文言どおりには発動していないと読める。9610708 の run は success、123deac の run は cancelled で、記録は failure とは書いていない。ただし、(a) の代わりとして足りるかは未確認。HEAD の bundle を作った記録が無いため。
- 閉じるために要るもの: (i) 38 行の直し。(ii) HEAD で 9 段を新しい WORK で回し、wasm を work-r53k と `cmp` で照合する(項目 3 (iv) のコマンド案)。
- 注記: `git show` と `git rev-parse HEAD`、`ls -la evidence` は shell で拒否された(permission.rejected)。代わりに `git log`・`git diff`・read・grep・glob を使った。現在の HEAD は `git log` の先頭で `c35b233`。記録の HEAD `20224a6` はその祖先。

## 2. 項目ごとの結果

**項目 1: R5-3n の記録**
- `git log --stat -n 6 --format='%h %ci %s'`: 45328cd の stat は `0372` +2・−0、`R5-3n.md` +65・−0、`R5-3n-oleaut-ole.txt` +16・−0(合計 83 insertions)。**一致**。
- `git diff 45328cd~1 45328cd -- docs/decisions/0372-coinitializeex-response.md`: 追加の 2 行のみ。**追加のみ、一致**。
- read `R5-3n-oleaut-ole.txt`: 4 テストの ok 行(ファイル 11〜14 行。全出力の行番号は 5・8・12・39)の名前は 4 つとも一致。`44 passed; 0 failed`(10 行。全出力の 50 行)、終了コード 0(7 行)、HEAD `20224a624e0e…`(4 行)、`git status --short -- lift` 空・`git diff --stat 123deac HEAD -- lift` 空(5・6 行)。**一致**。
- `git log -n 1 --format=%H 20224a6` = `20224a624e0ef21b9d1f61e8df45d939a267ca7d`(記録と一致)。`git log --ancestry-path 123deac..HEAD` で 20224a6 は 123deac の子孫。`git diff --stat 123deac 20224a6 -- lift` は空。つまり 20224a6 の lift は 123deac と同一で、そこで実行したテストに commit A の値 10 が入る(git の内容で言える)。20224a6 を実際に checkout して実行したことは記録の主張で、未確認。
- 補強: `evidence/R5-3m-cargo.txt` 1803 行に oleaut_ole の `44 passed; 0 failed`(commit A での全テスト実行)。

**項目 2: run の結論**
- `git log -n 1 --format='%h %ci' 7157394` = 09:44:36 +0900(00:44:36Z)。記録の updatedAt 00:45:43Z との差は **67 秒**(ジョブの completedAt 00:45:42Z なら 66 秒)。ただし `%ci` はコミットの時刻で、push の時刻ではない。
- `R5-3n.md`: conclusion `cancelled`(31 行)、headSha `123deac…`(一致)、createdAt 00:24:46Z(123deac のコミット 00:24:14Z の 32 秒後)、updatedAt 00:45:43Z(記録どおり)。「取り消しの主体と理由は未確認」(33・64 行)、「(a) は満たしていない」(34 行)。failure・失敗の語は無く、cancelled を失敗とも成功とも書いていない。
- 削除前のワークフロー(`git diff 7157394^ 7157394 -- .github/workflows/engine-bundle.yml`)に `concurrency` の指定は無い。この YAML からは、新しい push による自動取り消しは読めない。ほかの原因は未確認。
- gh の出力は reviewer が再現できず、記録の引用として残る。

**項目 3: 撤回条件 (4) の読み**
- (i) `R5-3l.md` 123 行(`43687019712 9610708 completed success`)、132 行(`conclusion: success`、`headSha: 96107088990d…`)、`0376` 61 行(success)。`git log -n 1 --format=%H 9610708` = `96107088990d5124abdf219cb4279c1d33f7e073`。132 行の headSha と**一致**(R5-3l §9 の未確認は、この SHA の照合で閉じた)。
- (ii) `git diff --stat 9610708 123deac -- lift app ci` = `lift/crates/fe-lift/tests/oleaut_ole.rs | 2 +-`(1 行変更)。`git log --oneline 9610708..123deac -- lift app ci` = 123deac のみ。**一致**。
- (iii) 削除前のワークフローの段: checkout、アカウント ID の取得、インストーラの取得と照合、抽出、ツールチェーン、`cargo fetch --locked`、上流 Blend2D、9 段(`bash app/scripts/build-engine-bundle.sh`)、SHA-256 の出力。**`cargo test` の段は無い**。`paths` に `lift/crates/**` を含む(だから 123deac の push で起動した)。
  - `app/scripts/build-engine-bundle.sh` の `tests` の語は 0 件(grep)。cargo は `cargo build --release --offline -p fe-pe -p fe-lift`(94 行)と `-p fe-wasmgen`(196 行)のみ。
  - 9 段は `tests/` の一部を入力にする: `gen-direct.sh` → `gen_two.sh`(36 行で呼ぶ)→ `TOOLS="$LIFT/crates/fe-lift/tests/realmod_s1/tools"`(gen_two.sh 45 行)。
  - ただし `oleaut_ole.rs` は入力でない: ビルド用の `.sh`・`.py`・`.c`・`.mjs`・`.ts` に `oleaut_ole` の参照は 0 件(lift/harness・app/scripts・ci を grep)。`oleaut_ole` を名指すのはテストの依存表 `lift/harness/testfast/deps.tsv` だけ。`tests/*.rs` のグロブも無い。`fe-lift/Cargo.toml` は `[lib]` と `[[bin]]` のみ。cargo の既定では `cargo build` はテストの .rs をビルドしない(公式仕様の読み。この検査では実行していない)。
- (iv) 結論:
  - 文言どおり「engine-bundle.yml が失敗する」: 9610708 は success、123deac は cancelled。記録の範囲では発動していないと**読める**。
  - 取り込みの push の run の同定は未確認。`d2d90a9`・`ab3f818` を headSha とする run は一覧(R5-3l の 5 件)に無い。コミット時刻は d2d90a9 15:46:02・ab3f818 15:46:09・9610708 15:46:54(JST)、run の作成は 15:47:17(JST)。1 回の push で tip の 9610708 が run を起動したと読むのが自然だが、event と headBranch は記録が無い。
  - (a) の代わりとして十分かは**未確認**。根拠は次のとおり: 123deac 以後の lift・app・ci の差は oleaut_ole.rs の 1 行だけ(git)。ビルドはそれを読まない(iii)。2f1915b と 9610708 は lift・app・ci が同一(`git diff --stat 2f1915b 9610708 -- lift app ci` は空)で、wasm は cmp 同一(R5-3l §8)。9610708 のローカルのビルドと CI は全桁一致(R5-3l §7、記録の引用)。ただし HEAD(c35b233)で 9 段を回した記録は無い。
  - 足りないもの: HEAD で 9 段を新しい WORK で回し、wasm を照合する。WORK は存在しないパスにすること(既存の段の出力は再利用されるため)。待ちは `wait-file.mjs`(R5-3l の記録では約 40 分)。
    ```
    WORK=~/.cache/signature-studio/work-r5-3o HOST_ROOT=~/work/signature-studio/artifacts/host JOBS=2 B2D_UPSTREAM_LIB=~/.cache/signature-studio/b2d/build-wasm/libblend2d.a B2D_UPSTREAM_SRC=~/.cache/signature-studio/b2d/src/src bash app/scripts/build-engine-bundle.sh
    cmp ~/.cache/signature-studio/work-r5-3o/bundle/fl-Oz/engine.c963715a4410.wasm ~/.cache/signature-studio/work-r53k/bundle/fl-Oz/engine.c963715a4410.wasm
    grep 'stage-command' ~/.cache/signature-studio/work-r5-3o/logs/stages.log | grep -vc 'rc=0'
    ```
    期待: `cmp` の rc 0、最後の行 0(stage-command の件数は R5-3l と同じ 32)。

**項目 4: 残りの未確認**
- 3 回目の検査の本文(項目 4 の (d)・(f)、出典のない断定 7 件)は、リポジトリの `evidence/`・`docs/` にどこにも見つからなかった(「出典のない断定」「差し戻し 0 件」の検索で該当は R5-4-review.md のみ)。そのため (d)・(f) と 7 件の内容は**検査できない(未確認)**。代わりに、R5-3n・R5-3l の未確認の一覧を閉じ方で分けた。

| 未確認の項目 | 今回の保存物・確認で | 判定 |
|---|---|---|
| 0372 の 4 テストの ok 行 | 閉じた(R5-3n-oleaut-ole.txt の抜粋。全出力は未保存) | 閉じた(抜粋の範囲) |
| 4 テストが commit A の値を含む HEAD で実行 | 閉じた(git: lift 同一) | 閉じた |
| 0372 の注が追加のみ | 閉じた(diff) | 閉じた |
| 9610708 の完全 SHA と gh の headSha | 閉じた(git の SHA と 132 行が一致) | 閉じた |
| wasm が work-r53k と work-r53h で同一になる理由(R5-3l §8) | 閉じた(2f1915b と 9610708 の入力が同一) | 閉じた(入力が同じ、の意味) |
| commit A の CI の bundle 作成の成功 | 閉じていない(cancelled、ワークフローは削除済み) | 未確認 |
| 取り消しの主体と理由 | 閉じていない | 未確認 |
| d2d90a9・ab3f818 の push で起動した run | 閉じていない(一覧に無い。event 未記録) | 未確認 |
| HEAD の bundle が work-r53k と同一 | 閉じていない(再ビルドの記録が無い) | 未確認 |
| 実エンジンでの実行(commit A 側) | 閉じていない(cargo のみ) | 未確認 |
| O16 の日本語の字形・原因・実機 | 閉じていない | 未確認 |
| seed pack に 3 ファイルがある bundle だけがこの経路に入る理由 | 閉じていない | 未確認 |
| dwCoInit の実際の値・他の 5 つの名前の NULL の影響 | 閉じていない | 未確認 |
| 組 1 の ja-main png・組 2 の停止直前の行 | 閉じていない | 未確認 |
| 依頼者機の値・P2 の 120 fps・P3・O9 | 対象外 | 未確認 |

**項目 5: ROADMAP R5-3 の行(51 行目)**
- 既存の括弧書きは書き換えていない。`docs/ROADMAP.md` は変更していない。R5-3 は todo のまま。
- 追記案(判定が受理になってから使う。3 文。`|` は含まない):
  > (done 時の追記案: 2 回目の検査の差し戻し 2 件には、commit `123deac`(テスト 1 行。決定 0372 の値を 10 に合わせた)と決定 0372 の末尾の注(`813bc1f`・`45328cd`)で応じた。cargo の結果は `evidence/R5-3m-cargo.txt`(commit `123deac`。既知の 1 件のみ)と `evidence/R5-3n-oleaut-ole.txt`(0372 の 4 テスト ok、44 passed)。撤回条件 (4) は、run `43780207694`(`123deac`)が cancelled で取り消しの主体は未確認、ワークフローは commit `7157394` で削除済みのため、9610708 の run `43687019712`(success。`evidence/R5-3l.md`)と、以後の lift・app・ci の差が oleaut_ole.rs の 1 行だけであることで読んだ(HEAD の bundle の再ビルドは未確認)。残る未確認: O16 の日本語の字形・原因・実機。)

**項目 6: 判定語と秘密**
- `効いた|直った|達成|問題ない|整合|到達`: `R5-3n.md` 0 件、`R5-3n-oleaut-ole.txt` 0 件。
- `ses_|/home/|[A-Za-z0-9]{32,}`: `ses_` 0 件、`/home/` 0 件。32 桁以上の並びは SHA と run の識別子だけ(`R5-3n.md` の 5・12・27・30・38 行、`R5-3n-oleaut-ole.txt` の 4 行)。

## 3. 不一致の一覧

| 項目 | 記録の記述 | 実測 | 実行したコマンド |
|---|---|---|---|
| 1 | `R5-3n.md` 38 行「全記録は `evidence/R5-3n-oleaut-ole.txt`」 | 同ファイル 2 行目「全出力は作業用の一時ファイルに保存し、ここには grep -n の抜粋を書く」。リポジトリには抜粋のみ | read(`R5-3n.md` 38 行、`R5-3n-oleaut-ole.txt` 2 行目) |

その他の記録は、確認できた範囲では一致した。gh の出力は再現できない(記録の引用)。

## 4. 出典のない断定の数

- `R5-3n.md`: 0 件。`R5-3n-oleaut-ole.txt`: 0 件(肯定の断定を数えた範囲)。
- 再現できない出典(gh の出力、run の一覧、watch の終了コード)は、記録の中に引用として残る。
- この検査での私の断定のうち、「cargo の `build` はテストをビルドしない」は公式仕様の読みで、実行していない(未確認の扱い)。

## 5. done にするときの括弧書きの案

項目 5 の追記案を参照。判定が受理になるまでは使わない。

## 検査 5(5 回目。差し戻し 3。D1・D2・D3)

## 判定: R5-3 を `done` にしてよいか → 差し戻し(3 件)。今は `done` にできません

いずれも追記で閉じる見込みです。追記のあとで、reviewer が再検査します。

- **D1**(4 回目の (i)): `evidence/R5-3n.md` 38 行の「全記録は `evidence/R5-3n-oleaut-ole.txt`」は、まだ誤りのままです。
- **D2**(4 回目の (ii)): 撤回条件 (4) の読みは内容として足りますが、R5-3 の記録にも決定 0375 にも書かれていません。
- **D3**: `docs/decisions/0369-seed-assets-about-ja.md` の「結果の追記」(2026-11-15)は、ロゴと staff roll を「未確定」、撤回条件を「判定できない」のままにしています。後の目視(`evidence/R5-3.md` の補いと、この検査の目視 3 枚)と食い違います。

完了条件「About で画像が出る」は、目視 3 枚で確かめました(項目 3・5)。O16 は完了条件の文言に含まれません。

**道具の記録**: シェルの 3 回が `permission.rejected` で拒否されました(`git status --short && git log … && git rev-parse HEAD`、`git rev-parse HEAD`、引用符を含む `grep -o`)。代わりに `git log -1 --format=%H`、`git diff --stat`、`git show --stat`、`git log --oneline`、grep・read ツールで同じ内容を読みました。その後のシェルの git 読み取りは通りました。`gh`・`sha256sum`・`cargo`・`wrangler`・`cf`・`portal_*`・`execute` は呼んでいません。HEAD は `da841cd`。ワークツリーの未 commit は `evidence/R0-6-failfast.txt` の 1 件だけで、R5-3 とは無関係です。

## 項目 1: `evidence/R6-7.md` の検査 → 一致(記録どうしの比較。再計算は未確認)

- read: 実行 `43786377286`、結論 success、headSha `68499eba9fe6992c1a13feebbbea3e824eafe5b7`、所要 43m1s(l.7–9)。gh の出力の引用で、再現していません。
- wasm: sha256 `c963715a4410dd56cd77e31aac44d2fd2f3b8616098cc57b9bdfae8742e6963d`(64 桁)、143118360 B(l.17–18、l.59–60)。
- 9 段: stage-command 32 件、rc は全て 0(l.56)。引用です。
- grep `c963715a[0-9a-f]{56}([^0-9a-f]|$)` を `evidence/` に実行: `R6-7.md` l.18・l.60、`R5-3l.md` l.140(CI の SHA256SUMS 行)・l.141(手元の work-r53k の全桁)でヒット。同じ 64 桁の文字列です。
- `R5-3l.md` §7 の「全桁一致」は、CI(9610708)と手元の比較です。R6-7 の値は手元の値(l.141)と同じなので、推移で一致します。R6-7 の CI(68499eb)との直接の比較行はありません。
- sha256 の再計算は、reviewer が `sha256sum` を使えないため未確認です。

## 項目 2: CI の入力と手元の入力の差 → 一致

- `git diff --stat 9610708 68499eb -- <deploy-engine.yml の prefix 入力一覧(lift・app/scripts・app/src/bundle・app/package.json・ci の 5 件・tools の 2 件)>` → `lift/crates/fe-lift/tests/oleaut_ole.rs | 2 +-`(1 件のみ)。
- 9 段の cargo は `cargo build --release --offline -p fe-pe -p fe-lift`(`app/scripts/build-engine-bundle.sh` l.94)と `-p fe-wasmgen`(l.196)だけで、テストを含みません。テストの 1 行は wasm のビルドに入りません。
- `git diff --stat 123deac 68499eb -- lift app ci`(順序は逆にした): `lift` と `app` は無変更。`ci/agents/*`(7 件・新規)、`ci/lib/wrangler_config.py`(新規・97 行)、`ci/verified-bundles.tsv`(+2)。いずれも入力一覧に無いです。
- `git diff --stat 68499eb da841cd -- lift app ci tools .github`: `app/test` 2 件、`ci/agents` 4 件、`.github/workflows/agent.yml`。入力一覧に無いです。`lift` は 123deac から da841cd まで無変更です。
- merge `71945dd` の中身(`git diff --stat 8f4332f 71945dd`): 13 件、`lift`・`app`・`ci`・`tools` に無し。
- `git show --stat 7157394`: `.github/workflows/` の 4 件だけ(計 710 行削除)。

## 項目 3: 決定 0375 の撤回条件 (1)〜(4)

| 条件 | 出典 | 読み |
|---|---|---|
| (1) 同位置の停止 | `R5-3l.md` §8 l.148–151 | work-r53k 3/3 止まらず。未発生 |
| (2) 別の停止 | `R5-3l.md` §8 | 「別の結果 0 件」。未発生 |
| (3) cargo の新しい失敗 | `R5-3m-cargo.txt`(123deac。219 行・通過 2426・失敗 1・無視 1)、`R5-3n-oleaut-ole.txt`(44 passed) | 失敗は既知の `getmodulehandleexw_returns_in_every_build` のみ。lift は da841cd まで無変更なので、この結果は現 HEAD の lift に当たる。未発生 |
| (4) `engine-bundle.yml` の失敗 | 下記 | 観測 HEAD については足りる。記録が無い(D2) |

条件 (4) の詳細:
- 観測 HEAD 9610708 の CI run `43687019712` は success(`R5-3l.md` §7。gh の引用)。その headSha `96107088…` は、`.git/logs/HEAD` の 9610708 の full SHA と一致します(l.404)。
- 123deac の run `43780207694` は cancelled(`R5-3n.md` (a)。取り消しの主体と理由は未確認)。123deac は観測 HEAD ではなく、差はテスト 1 行です。
- `engine-bundle.yml` は 7157394 で削除済みなので、文言の「失敗」はこれ以後評価できません。
- 同じ 9 段の CI は、68499eb の `deploy-engine` run `43786377286` が success で、wasm は同一です(`R6-7.md`。引用)。
- 読み: 観測 HEAD の 9 段の CI は成功しています。ビルドの失敗という懸念は、観測 HEAD については足ります。ただし、この読みを書いた記録が R5-3 にありません(D2)。

## 項目 4: `evidence/R5-3n.md` 38 行 → 不一致(D1)

- `R5-3n.md` l.38 は未修正です。`R5-3n-oleaut-ole.txt` l.2 は「全出力は作業用の一時ファイルに保存し、ここには grep -n の抜粋を書く」と書いています。誤りは確定しています。
- 追記だけで直す案は §5(a)。追記があれば、(i) は対応ありと読めます。本文は書き換えません。

## 項目 5: 残りの未確認(4 回目の表の更新)

| 残り | 出典 | 今回 | done への影響 |
|---|---|---|---|
| cargo を実行していない | `R5-3l-review2.md` l.35 | 対応あり: `R5-3m-cargo.txt`・`R5-3n-oleaut-ole.txt` | なし |
| 実エンジンの実行をしていない(cargo のみ) | `R5-3n.md` (d) | 対応あり(手元の実行): `R5-3l.md` の 6 回 | なし。公開ホストは別項 |
| run `43780207694` が cancelled・bundle 未作成 | `R5-3n.md` (a) | 代替あり: 観測 HEAD の CI success(`R5-3l.md` §7)、68499eb の deploy-engine success(`R6-7.md`) | 条件 (4) の読みに使う(D2) |
| 取り消しの主体と理由 | `R5-3n.md` (a) | 未確認のまま | なし(条件 (4) は観測 HEAD で評価) |
| 9610708 の full SHA と gh の headSha の一致 | `R5-3l.md` §9 | 対応あり: 両方 `96107088990d5124abdf219cb4279c1d33f7e073` | なし |
| d2d90a9 の push で engine-bundle が起動したか | `R5-3l.md` §7 | 未確認(一覧に無い) | なし |
| O16 の日本語の字形 | `R5-3.md` 補い、`R5-3n.md` (d) | 未観測。所定プロジェクトに日本語名が無い | 完了条件の文言には無い。括弧に「未観測」を残す |
| O16 の原因・実機・O9 | `R5-3n.md` (d)、`R5-3l.md` §9 | 未確認。実機 0 件 | なし(完了条件の外) |
| 公開ホストの engine-idle | `R6-7.md` §6 | 未確認(Access) | R5-3 の完了条件の外。下の範囲外の指摘に関係 |
| seed pack の 3 ファイルが経路に入る理由・dwCoInit・他 5 名・停止直前の行 | `R5-3l.md` §9 | 未確認 | なし(括弧で「原因は未確認」) |
| `wall_fr` の期待値が正しいか | `R5-3l-review2.md` l.35 | 未判断(cargo は通過) | なし |
| `oleaut_ole.rs` の 71 行の中身 | `R5-3l-review2.md` l.36 | 未読 | なし |
| 依頼者機の値 | `R5-3n.md` (d) | 0 件 | なし(完了条件の外) |
| 0369 の撤回条件 | `0369` l.64 | 「判定できない」(定義 3 と work-r25 の vfs が未観測) | D3 の注で明記 |

**完了条件の文言との照合**: 完了条件は「seed 再ビルド→ About で画像が出る(画を `evidence/R5-3.md` に)」です。O16・原因・実機・公開ホストは、この文言に含まれません。ただし行のタイトル(O16 を含む)と 0369 の撤回条件(「日本語が豆腐のまま」)には関係するため、括弧に「未観測」を残す必要があります。

**目視の結果(reviewer が 3 枚を開いた)**:
- `evidence/R5-3/r53e13-chrome-2/about-28s.png`: ロゴも staff roll も見えない(E13)。
- `evidence/R5-3/r53wr25-chrome-1/about-28s.png`: フルーツのロゴと開発者名の列が見える(work-r25)。
- `evidence/R5-3l/r53knew-chrome-1/about-28s.png`: 同じく見える(work-r53k)。
- いずれも n=1 です。`R5-3.md` の補い(main の目視)と一致します。

## 項目 6: ROADMAP R5-3 の行(`docs/ROADMAP.md` l.51)

- `| R5-3 | todo |` のままです。既存の括弧は書き換えません。
- 括弧の案は §5(d)。D1〜D3 の追記のあとに入れてください。

## 項目 7: 秘密・ID → 該当なし

- `evidence/R6-7.md` を 32 桁以上の 16 進、R2 のホスト、トークン様の語、セッション ID、ホームの絶対パスで grep しました。
- ヒットは 40 桁の commit SHA(l.9)と 64 桁の sha256(l.18・38・40・60)だけです。

## 範囲外の指摘(main 宛て。D には数えない)

R6-2 の条件 1 は、差し替える bundle について「パラメータ無しの URL で開き、`engineKind`・`standIn=false`・`vfs seed:` の行を確かめた」ものだけを許しています(`evidence/R6-2.md` l.19)。R6-7 の root 差し替えの前に、この確認の記録はありません。`evidence/R5-3l/r53knew-chrome-1/report.json` は `state` と `timeline` だけで、`engineKind`・`standIn`・`vfs seed`・URL は grep で 0 件です。R6-7 の公開ホストの確認は未確認です(Access の背後)。0381 の「一覧に在る wasm」の基準が R6-2 の条件 1 を置き換えるかは、0381 にも R6-2 にも書かれていません。AGENTS §5 の「案内する URL は headless Chrome で開いた URL と同一」にも関わります。main が、この差し替えを維持するか戻すか、確認を足すかを決めて記録してください。R5-3 の判定には影響しません。

## 3. 不一致の一覧

- D1: `R5-3n.md` l.38 と `R5-3n-oleaut-ole.txt` l.2。
- D2: 決定 0375 の (4)「engine-bundle.yml が失敗する」と、7157394 での削除。
- D3: `0369` の結果の追記と、`R5-3.md` の補い・目視 3 枚。
- 範囲外: R6-2 の条件 1 と R6-7 の記録。
- 文書上の説明済み: R5-3 のやること「seed に行を足す」と 0369「行は足さない」。括弧で説明済みです。

## 4. 出典のない断定の数: 0 件

## 5. 追記の案と括弧の案

**(a) `evidence/R5-3n.md` の末尾に追記(D1・D2)**

```
注(2026-11-16、R5-3 の 4 回目の検査の差し戻し (i) への対応。上の本文は書き換えていない): 上の (b) の「全記録は `evidence/R5-3n-oleaut-ole.txt`」は誤り。`evidence/R5-3n-oleaut-ole.txt` は `grep -n` の抜粋で、全出力は作業用の一時ファイルにあり、リポジトリには無い(同ファイル 2 行目)。

注(2026-11-16、撤回条件 (4) の読み。決定 0375 の (4)): `engine-bundle.yml` は 7157394 で削除済み。(α) 観測の HEAD 9610708 の 9 段の CI は success(run 43687019712。`evidence/R5-3l.md` §7)。(β) 9610708 から 68499eb までの 9 段の入力の差は `lift/crates/fe-lift/tests/oleaut_ole.rs` の 1 行(テスト。wasm のビルドは `app/scripts/build-engine-bundle.sh` の cargo build 2 件で、テストを含まない)。(γ) 68499eb の 9 段の CI(deploy-engine、run 43786377286)は success で、wasm は `c963715a…`(`evidence/R6-7.md`)。123deac の run 43780207694 は cancelled で、取り消しの主体と理由は未確認。
```

**(b) 決定 0375 の末尾に注(D2)**

```
注(2026-11-16): 撤回条件 (4) の「engine-bundle.yml」は 7157394 で削除済み。(4) の読みは `evidence/R5-3n.md` 末尾の注に書く。
```

**(c) 決定 0369 の末尾に注(D3)**

```
注(2026-11-16、R5-3 の 5 回目の検査): 上の『結果の追記』(2026-11-15)の『ロゴ・staff roll の目視は未確定』は、後の目視で置き換わる(`evidence/R5-3.md` の補い。reviewer の目視 3 枚も同じ結果)。E13(`evidence/R5-3/r53e13-chrome-2/about-28s.png`)は、ロゴ・staff roll が見えない。work-r25(`evidence/R5-3/r53wr25-chrome-1/about-28s.png`)と work-r53k(`evidence/R5-3l/r53knew-chrome-1/about-28s.png`)は、フルーツのロゴと開発者名の列が見える(各 1 枚。n=1)。撤回条件の「判定できない」は、定義 3(日本語)と work-r25 の vfs-miss が未観測のため、そのまま残る。
```

**(d) ROADMAP R5-3 の括弧の案(項目 6。D1〜D3 の追記のあとに入れる。`|` は含まない)**

```
(追記 2026-11-16: 2 回目〜4 回目の検査の差し戻しのうち、テストの値(123deac)・0372 の注(813bc1f・45328cd)・cargo の全保存(evidence/R5-3m-cargo.txt、evidence/R5-3n-oleaut-ole.txt)・R5-3n.md の誤りの注・0369 の注を記録した。撤回条件 (4) は、観測の HEAD 9610708 の 9 段の CI の成功(evidence/R5-3l.md §7)と、68499eb の deploy-engine の成功・wasm の一致(evidence/R6-7.md)で読む(engine-bundle.yml は 7157394 で削除済み。run 43780207694 は cancelled で、原因は未確認)。未確認のまま: O16 の日本語の字形・原因・実機、公開ホストの engine-idle。)
```

## 検査 6(6 回目。追記 3 件を受理。ROADMAP の案の差し戻し 2)

## 判定

- **R5-3 を done にする判定: 差し戻し(2 件)。** 対象は ROADMAP 案の文言です。
- **追記 3 件は受理。** 0369 注・0375 注・R5-3n.md 注は、追加のみ(削除 0)で、文面は依頼文の要約どおり、指す事実は実物と一致しました。
- **未確認**: 9610708 側の engine-bundle.yml の手順は未読(wasm の一致で間接に確認)。68499eb の完全 SHA との照合は未確認(rev-parse が拒否)。0369 の撤回条件のうち work-r25 の vfs-miss と日本語は未観測のまま。
- 直した最終案を ROADMAP に入れた後は、その行の再検査が要ります(本検査は案の検査)。
- 対象外: ワークツリーの ci/agents 等の未コミット変更(別 worker のもの)には触れていません。

## 1. 変更範囲
- `git show --numstat 9731e68`: `docs/decisions/0369-seed-assets-about-ja.md` +2/−0、`0375-coinitializeex-merge.md` +2/−0、`evidence/R5-3n.md` +4/−0。削除 0。
- `git diff -U0 9731e68~1 9731e68` の hunk は `@@ -68,0 +69,2 @@`・`@@ -28,0 +29,2 @@`・`@@ -65,0 +66,4 @@` の 3 件で、追加のみ。
- HEAD は 24a8424 に進んでいるが、この commit は `ci/agents/import-login.sh` と `ci/tasks/0001-read-only-check.md` のみで、3 ファイルは変わっていません。

## 2. 追記の文面(read で確認)
- **(D1)** `evidence/R5-3n.md` 67 行: 「上の (b) の『全記録は `evidence/R5-3n-oleaut-ole.txt`』は誤り。…全出力は作業用の一時ファイルにあり、リポジトリには無い(同ファイル 2 行目)。」
- **(D2)** 同 69 行: (α) run 43687019712 は success、(β) 入力の差は oleaut_ole.rs の 1 行、(γ) run 43786377286 は success で wasm は c963715a…。123deac の run 43780207694 は cancelled。0375 末尾も「(4) の読みは `evidence/R5-3n.md` 末尾の注に書く」と指している。
- **(D3)** 0369 の 70 行: E13 はロゴ・staff roll が見えない。work-r25 と work-r53k は見える(各 1 枚。n=1)。撤回条件の「判定できない」は残る。

## 3. 事実の照合
| 項 | 結果 | 出典 |
|---|---|---|
| (a) 36〜38 行と訂正の指し先 | 一致 | R5-3n.md 38 行の「全記録は…」と 67 行の訂正。`R5-3n-oleaut-ole.txt` 2 行目は「全出力は作業用の一時ファイルに保存し、ここには grep -n の抜粋を書く」 |
| (b) §7 の success | 一致 | R5-3l.md 123 行(9610708、success)、132 行(conclusion success、headSha `96107088990d5124…`)。§7 は 118 行から。120 行の `gh run list --workflow engine-bundle.yml`。`git show -s --format=%H 9610708` も `96107088990d5124abdf219cb4279c1d33f7e073` |
| (c) 68499eb の CI と wasm | 一致 | R6-7.md 9 行(headSha `68499eba9fe6…`)、84 行(43786377286 completed success)、18 行(wasm `c963715a4410dd56…`) |
| (d) 入力の差 | 一致(範囲の条件つき) | 指定パスの stat は 6 件: oleaut_ole.rs(1 行変更)と `tools/verify-kit/` の 5 件(+205)。68499eb の deploy-engine.yml の prefix 鍵の対象(`lift/crates/**`・`app/scripts/build-engine-bundle.sh` 等)に tools/verify-kit は含まれず、build-engine-bundle.sh も参照しない(全行を読んだ)。全体 stat の 90 件では lift/ の変更は oleaut_ole.rs のみ、app/ は 0。`git show --stat 123deac` は 1 file +1−1。cargo build は `-p fe-pe -p fe-lift` と `-p fe-wasmgen` の 2 件で、テストを含まない |
| (e) 画像 3 枚 | 一致 | 下に記載 |

画像 3 枚(read で開いた):
- `evidence/R5-3/r53e13-chrome-2/about-28s.png`: パネル中央に灰色の歯車の図(フルーツのロゴではない)。開発者名の列は無い。ロゴ・staff roll は見えない。
- `evidence/R5-3/r53wr25-chrome-1/about-28s.png`: オレンジの果実のロゴと開発者名の列(Kyle Spratt Research & Development ほか)、Past developers・In memoriam・FL Cloud の欄が見える。
- `evidence/R5-3l/r53knew-chrome-1/about-28s.png`: 同上。

0369 注が参照する `evidence/R5-3.md` は存在を確認(read)。

## 4. 4 回目・5 回目の差し戻しの閉じ方
| 項目 | 状態 | 根拠 |
|---|---|---|
| (i-1) テストの値 2\|8=10 (`123deac`) | 閉じた | diff は `0, 8` → `0, 10`(oleaut_ole.rs 1257 行)。0372 の 56 行に記録 |
| (i-2) 0372 の注 (`813bc1f`・`45328cd`) | 閉じた | 0372 の 56 行(R5-3m)・58 行(R5-3n)。各 commit は 2 行追加 |
| (ii-1) 0372 の 4 テストの ok 行 | 閉じた(抜粋) | `R5-3n-oleaut-ole.txt` 11〜14 行に 4 件の ok、10 行に `test result: ok. 44 passed`。4 テスト名は oleaut_ole.rs の 1201・1235・1249・1264 行に定義あり |
| (ii-2) cargo | 閉じた(要約) | `R5-3m-cargo.txt` に test result 行 219 件、失敗 1 件(既知の `getmodulehandleexw_returns_in_every_build`)。269 行に「全ログの要約」と自己申告 |
| (iii-1) D1 | 閉じた | 67 行の訂正。同ファイル 2 行目の文言と一致 |
| (iii-2) D2 | 閉じた | 0375 末尾と R5-3n.md 69 行。(α)(β)(γ) は項目 3 のとおり |
| (iv) D3 | 閉じた | 0369 注の 3 枚の記述は画像と一致 |

未確認として残るもの: 9610708 の engine-bundle.yml の手順(未読)、68499eb の完全 SHA(未照合)、0369 の撤回条件の vfs-miss と日本語(未観測)。

## 5. ROADMAP 案の検査
- **「2 回目〜4 回目」は誤り。** 0369 の注は 5 回目の D3 なので「2 回目〜5 回目」にする。
- **「cargo の全保存」は誤り。** `R5-3m-cargo.txt` は 1 行目で「全保存」と書くが、269 行目で「全ログの要約」と自己申告している。`R5-3n-oleaut-ole.txt` は 2 行目で「grep -n の抜粋」と書いている。
- テストの値・0372 注・R5-3n 注・0369 注・撤回条件 (4) の記述は事実と合う。判定語・原因の断定・「直った」の類は無い。
- 0369 の撤回条件の判定が未観測であることは、未確認の項目に足す。

直した最終案(追加のみ):

`(追記 2026-11-16: 2 回目〜5 回目の検査の差し戻しのうち、テストの値(123deac)・0372 の注(813bc1f・45328cd)・cargo test の要約(evidence/R5-3m-cargo.txt)と 0372 の 4 テストの ok 行の抜粋(evidence/R5-3n-oleaut-ole.txt)・R5-3n.md の誤りの注・0369 の注を記録した。撤回条件 (4) は、観測の HEAD 9610708 の 9 段の CI の成功(evidence/R5-3l.md §7)と、68499eb の deploy-engine の成功・wasm の一致(evidence/R6-7.md)で読む(engine-bundle.yml は 7157394 で削除済み。run 43780207694 は cancelled で、原因は未確認。)未確認のまま: O16 の日本語の字形・原因・実機、公開ホストの engine-idle、0369 の撤回条件の判定(work-r25 の vfs-miss と日本語)は未観測。)`

## 6. 秘密・ID・判定語
追記の行(0369 の 70 行、0375 の 30 行、R5-3n.md の 67〜69 行)に、`ses_`・`/home/`・token・ghp_・github_pat・secret・「直った」「達成」「効いた」「問題ない」「整合」「到達」「受理」「完了」「done」は無い(grep の件数 0)。R5-3n.md の 3 行目と 62 行の「完了」「done」は 9731e68 より前からの文で、追記の対象外。

## 4'. 出典のない断定
0 件。本報告の断定には出典を付け、確かめられなかった点は「未確認」と書いた。

## 拒否された shell コマンド(事実の記録)
1. `git grep -n "tools/" 68499eb -- app/scripts ci .github/workflows/deploy-engine.yml`: 拒否。代わりに `git show 68499eb:app/scripts/build-engine-bundle.sh` と `git show 68499eb:.github/workflows/deploy-engine.yml` を読んだ(成功)。
2. `grep -c 'test result:' … && wc -l …`: 拒否。代わりに grep ツールの件数を使った(219 件。見出し行を除く)。
3. `awk '/test result:/…' … && wc -l …`: 拒否。代わりに同じく grep ツールを使った。
4. `git rev-parse 9610708 68499eb && git show --stat … | head`: 拒否。代わりに `git show -s --format=%H 9610708` と `git show --stat --format=%h%x20%s 7157394` を使った(成功)。68499eb の完全 SHA は未確認。

## 検査 7(7 回目。ROADMAP の行を検査。判定: 未確認。文書側の不足 3)

## 判定

**R5-3 の `done` は未確認。** ROADMAP の R5-3 の行(項目 1〜5)は最終案と字句で一致し、差し戻しは 0 件です。項目 6 の (b) は、保存された reviewer の検査記録では受理を確かめられません。文書側の差し戻しは 3 件です(下の「不一致と直し方」)。

## 項目 1〜6

| 項目 | 結果 | 出典 |
|---|---|---|
| 1 変更範囲 | 一致。変更は `docs/ROADMAP.md` の 1 行のみ。語の差分は `todo`→`done` と、末尾の括弧書きの追加だけ。既存の括弧書きは書き換えなし | `git show --numstat --format=%h 2148d02` が `1 1 docs/ROADMAP.md`。`git diff -U0`、`--word-diff=porcelain` の削除は `todo` の 1 語のみ |
| 2 括弧書きの照合 | 一致(字句の差 0 件)。最終案の全文を literal 検索すると 51 行に 1 件。前は「done にする) (追記」、後は「未観測。) \| O3・O16、0320 \|」で、余計な字句なし | grep(literal) 3 回 |
| 3 表の形 | 一致。R5-3 と R5-4 の行は `^([^\|]*\|){10}[^\|]*$` に一致(9 列)。追加の括弧書きに `\|` なし | grep |
| 4 状態の語 | 行の中は一致。状態は `done`。`todo` は 2026-11-15 の「状態を todo に戻した」の履歴のみ。未確認は 5 つ残る。ただし `evidence/R5-3.md` の 5 行目「状態: 途中」と矛盾(差し戻し 1) | read、grep |
| 5 判定語・原因の断定 | 0 件。追加の括弧書きで肯定の判定語 6 語・「断定」「原因は確定」を検索し、一致なし。原因は「未確認」と書かれている | grep |
| 6 `done` の条件 | (a) 満たす: `evidence/R5-3.md` あり。(b) 満たさない(確かめられない): 下の表 | 下 |

項目 6 (b) の内訳:

| 検査 | 記録 | 確かめた内容 |
|---|---|---|
| 保存された報告 | `evidence/R5-3l-review.md` の 1 件 | 総合「未確認」。(C)「`done` にしてはいけない(現時点)」 |
| 2 回目 | `evidence/R5-3m.md` 5 行目の言及のみ | 判定「未確認」と書かれるが、報告本文は無い。保存された報告のどれに当たるかは確定できない |
| 3 回目 | `evidence/R5-3n.md` 3 行目の言及のみ | 前提 2 件。報告本文は無い |
| 4 回目 | `evidence/R5-3n.md` 67 行の言及 | 差し戻し (i)。報告本文は無い |
| 5 回目 | `docs/decisions/0369-seed-assets-about-ja.md` 70 行の言及 | 目視 3 枚の結果のみ。判定の報告本文は無い |
| 6 回目 | 保存なし | 依頼文の記述(追記 3 件の受理)のみ。リポジトリでは確かめられない |

そのほかの不足:
- 完了条件の「seed 再ビルド」の記録が無い。`evidence/R5-3*.md` に「再ビルド」は 0 件。決定 0369 の決定 1 は「行は足さない」。`lift/harness/seeds/vfs-seed-idle1.tsv` の 24・240・241 行に 3 件あることは確認した。
- About の画は、E13 では出ず、work-r25 と work-r53k で出る(各 1 枚・目視)。目視は main が行ったと `evidence/R5-3.md` 171〜178 行に書かれ、reviewer の検査ではない。
- 撤回条件 (4) の参照: `evidence/R5-3l.md` 132 行の run 43687019712(headSha 9610708)は success。`evidence/R6-7.md` の run 43786377286(headSha 68499eb)は success。run 43780207694 が cancelled であることは、私は確かめていない(未確認)。

## 不一致と直し方

ROADMAP の行そのもの(差し戻し 0 件)は直す必要がありません。直すのは次の 3 件です。

1. `evidence/R5-3.md` の先頭に注を 1 行足す(本文は書き換えない)。文案: 「注(2026-11-16): 5 行目の『状態: 途中』と表の『目視: 未確定』は、このファイルを書いた時点の記述。About の目視の結果は 171 行以降(main の目視。reviewer の検査ではない)。」
2. `evidence/R5-3.md` に 1 行足す。文案: 「完了条件の『seed 再ビルド』は行っていない。根拠: 決定 0369 の決定 1(行は現行 source に既にあり、足さない)。」
3. 2〜6 回目の reviewer の報告と、6 回目の受理の本文を `evidence/` に保存する(`evidence/R5-2-review.md`・`evidence/R4-1-review.md` と同じ形)。保存できない間は、R5-3 を `todo` に戻し、括弧書きに「検査記録の保存は未了」と書く。この 3 点は main か worker が行う。reviewer は作れない。

## 実行の記録

- シェルの呼び出し(`python3` による差分照合。`/tmp/opencode` への書き出しを含む)は `permission.rejected: Permission denied: shell` で拒否された。以後は git 読み取り・grep・read・glob だけで確かめた。
- テスト・ビルド・`gh`・`scripts/*` は実行していない。
- 報告・ROADMAP・evidence・決定文書は変更していない。
