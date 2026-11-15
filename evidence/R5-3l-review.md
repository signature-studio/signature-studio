# R5-3l: reviewer の検査の報告(コピー)

下は reviewer(2026-11-15)の最終報告の全文のコピー。コピーした worker はこの検査を再実行していない。検査対象は commit `0651b55`・`d2d90a9`・`ab3f818` と、ROADMAP の R5-3 の完了条件。この reviewer は shell の git 系コマンドが拒否され、項目 1(変更範囲)と項目 6(実装の差分・テスト)と seed の `git log` を実行できず、(A)(B) の総合は『未確認』だった。その 3 点は、main が git で確かめた(`git show --numstat 0651b55`・`git show --stat d2d90a9`・`git diff --stat a03c903 9610708 -- lift app ci`・`git diff -U1 a03c903 9610708 -- lift/crates/fe-lift/tests/wall_fr4.rs lift/crates/fe-lift/tests/wall_fr20.rs`・`git log --oneline -n 5 -- lift/harness/seeds/vfs-seed-idle1.tsv`。結果は、変更が `lift/` の 12 ファイルと `evidence/`・`docs/decisions/` だけ・テストの期待値の値は変わらず数え方に 1 項を足しただけ・seed の履歴は `14268c2` の 1 件だけ)。この main の確認は reviewer の検査ではない。コピーの 10 項目目の判定語の列挙は、禁止語と ID・パスの接頭辞の文字列を含むため、その文字列を『禁止語 8 語と接頭辞 2 種』に置き換えた(数と判定は変えていない。コピーに ID・パスは含まれていなかった)。

## reviewer の報告(全文)

## reviewer 報告(R5-3l・R5-3 の検査)

- **環境**: シェルが 2 回とも permission で拒否されました。git 系の検査(項目 1・6 と CI の照合、seed の git log)は未確認です。
1. **未確認**: `git show --numstat` / `--stat` / `diff --stat` が拒否されました。
2. **受理**: `evidence/R5-3l/*/report.json` を `expire_grep` で確認しました。r53knew 1〜3 は `stopped: false`・`state: engine-idle`。r53kold 1〜3 は `stopped: true`・`state: exited`・`code: 134`・同じ `message`。`opErrors` は 6 件とも `[]`。`firstIdleMs` も記録と一致します。
3. **受理**(数字は 2 と一致)。**指摘**: `evidence/R5-3l.md` 3 行目「実行する(これから)」は観測後も残る定型文です。直し方は先頭に注を足すことで、行は書き換えないでください。CI の SHA 照合は gh 出力の引用で、未確認です。
4. **受理**: 0376 は予測 (a) を結果より前に書き、結果は末尾に追記しています。0375 は分類 (b)・限界・未確認を満たします。**注**: 0375 の撤回条件は文書上 4 項目です(同位置の停止・別の停止・cargo の新しい失敗・engine-bundle の失敗)。依頼文の「3 つ」は誤りです。0376 で出ていないのは、同位置の停止・別の停止・engine-bundle の失敗の 3 つです。`dwCoInit` の名前つき停止は 0 件。未観測は cargo の新しい失敗で、取り込み後の commit での cargo test は未実行です(R5-3k は commit 前)。
5. **受理**: `test result:` の 219 行(26〜244 行)を全部足しました。小計は 26〜75 行が 425。残りを順に足すと合計 2426、失敗 1(193 行)、無視 1(231 行)。FAILED は既知の `getmodulehandleexw_returns_in_every_build` の 1 件だけです。
6. **未確認**: `git diff` が拒否されました。読めた範囲は、`lift/runtime/fle_ole.c` 103 行の `R[2] != 2` による `fle_stop` と、`imports_table.rs` 777 行の 1 行追加です。テストの差分は未確認です。
7. **受理**(目視): `guilt_read` で開いた順と添付の順が一致しました(画像に名前は写っていません)。r53knew-chrome-1 はロゴ・開発者列・「trial mode」があり、日本語はありません。r53wr25-chrome-1 は同じ。r53e13-chrome-2 はロゴ・開発者列がなく、歯車だけです。記録の目視と一致します。
8. **(C)の材料**: (i) R5-3.md の目視 1・2 と一致します。(ii) `vfs-seed-idle1.tsv` の 24・240・241 行に 3 ファイルがあります。R2-7c 4b の 228 件と 225 件の差は 3 行です。git log が未確認なので、「この clone で seed を変えていない」は未確認です。(iii) 0369・0375 は分類 (b) で、撤回条件もあります。(iv) 0369 16 行目に Noto Sans CJK JP の subset・OFL 1.1・出典があります。(v) 日本語は未観測です。`ja-main-20s.png` は開いていません。
9. **(C)判定: `done` にしてはいけない(現時点)**。理由: 項目 1・6 が未確認で、「記録が揃い reviewer が受理した」を満たせません。下書きには 2 点の直しが要ります。(a)「0372 の取り込みで止まらなくなった」は原因の断定に読めます。0375 は「原因だとは断定していない」と書いています。「取り込み後の bundle は n=3 で止まらなかった(原因は未確認)」に直してください。(b)「seed の行は既にあり追加していない」は git log 未確認です。1・6 が通れば、O16 未観測の括弧書きを残して再判定してよいと読みます。完了条件に O16 は無いものの、目的には入ります。
10. **判定語・秘密**: 0375・0376・R5-3l.md で、禁止語 8 語と ID・パスの接頭辞 2 種(本文では書かない)は 0 件です。`token|secret|password|@` は R5-3l.md 133 行の `actions/checkout@v4`(版の表記)のみで、秘密ではありません。

**(A)(B) 総合: 未確認**(項目 1・6 が実行できず)。確認できた範囲では、数字の不一致・原因の断定・既存行の書き換え・判定語・秘密は見つかりませんでした。

## reviewer の指摘の扱い

1. 項目 3 の指摘(3 行目の定型文)は `evidence/R5-3l.md` の先頭の注で扱った。
2. 項目 4 の注(0375 の撤回条件は 4 項目)は、この記録の手順の依頼文の誤りで、0375 の文は 4 項目のまま正しい。
3. (C)の判定の下書きの 2 点(『0372 の取り込みで止まらなくなった』を『0372 を含む bundle は台本 A で n=3 止まらなかった(原因は未確認)』に直す。seed の行は git log で確かめる)は、ROADMAP の括弧書きに反映した。
