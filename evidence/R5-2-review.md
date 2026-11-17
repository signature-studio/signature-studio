# R5-2: reviewer の検査の報告(コピー)

下は reviewer(セッション `ses_e241c5375ffe5LNO6FfeMmbE4c`、2026-11-15)の最終報告の全文のコピー。コピーした worker はこの検査を再実行していない。検査対象は `evidence/R5-2.md` の HEAD `8408865` 時点の内容。

## reviewer の報告(全文)

## R5-2 検査結果(reviewer)

**項目 1 の判定: 受理.** `ls evidence/R5-2`: PNG 6 枚・JSON 3 本が実在。`ls -la evidence/R5-2`: compositor 29,623/29,788/29,745 B、page 39,480/61,215/41,318 B、JSON 4,774/4,753/4,777 B。`git status --short -- evidence/R5-2 evidence/R5-2.md evidence/R5-2-recompute.txt` は出力なし(未コミット変更なし)。

**項目 2 の判定: 受理.** `expire_grep` の `timelineIdle`: run-1 4964.52、run-2 4510.66、run-3 5730.88(記録 4964.5・4510.7・5730.9 と一致)。`emscripten-fruity-v1`・`engine-idle`・`"error": null` は 3 本とも一致。

**項目 3 の判定: 受理(指摘あり).** `guilt_read` で `page-run1.png`・`page-run2.png`・`page-run3.png` を目視。run1・run3 は canvas 領域が全域黒、run2 は wizard が描かれる。「Load a d」がダイアログの下に隠れる点も run2 で見える。豆腐は見えない。指摘: run2 では「Learn abo」の切れが x≈502 の canvas 右端で起きている。記録の「窓の幅 874 に対して画面の幅は 780」は compositor 側の説明で、page 画像の切れの位置とは別の要因の可能性がある(原因は未確認)。また記録は run2 の文字を「読めるかは判定できず」とするが、page-run2 では「Help us improve FL Studio…」の文字は読めた。

**項目 4 の判定: 受理.** 7 行目に「実機との比較: 0 件(H6)」。`expire_grep`(`直った|達成|効いた|問題ない|整合|到達`)のヒットは 8 行目・11 行目・86 行目で、すべて否定の文脈(「到達とは書かない」「到達とは書いていない」「断定ではない」)。

**項目 5 の判定: 受理(指摘あり).** `guilt_read` の `evidence/R5-2-recompute.txt`: compositor の SHA-256 先頭 8 桁(04e2315d・126167a5・b0fdece7)、非背景画素 124105、色数 1034・1049・1063、差の画素 25732・25738、DOM canvas の色数 1・1193・1、黒 155118 が記録の表と一致。指摘: 再実行はしていない(指示どおり、値の比較のみ)。

**項目 6 の判定: 受理.** 11 行目に P2 の 120 fps 未確認、P3 の音切れ、O9 未特定、8 件のプラグイン、依頼者機 0 件、reviewer の最終検査未了。「G5 の到達・O1・O3・O16 の原因の断定ではない」と明記。

**項目 7 の判定: 受理.** `expire_grep`(`token|secret|password|account`、`evidence/R5-2.md`)はヒットなし。

**項目 8 の判定: 受理.** 他の記録との比較は行わず、本記録の数字は JSON と一致した(項目 2 で確認)。

**総合判定: 受理.**
数字・事実の不一致は 0 件。指摘は項目 3(切れの位置の帰属、run2 の文字の読みの扱い)と項目 5(再実行なし)の 2 点。R5-2 を `done` にする判定は、この記録の完了条件(画像・欠けの一覧・実機比較 0 件の明記)に限る。G5 の到達や原因の断定の判定には使わない。

## reviewer の指摘の扱い

- (1) 項目 3 の指摘: run2 の page 画像で「Learn abo」の切れは x≈502 の canvas 右端で起きており、`evidence/R5-2.md` の「窓の幅 874 に対して画面の幅は 780」は compositor 側の説明で、page 画像の切れの位置とは別の要因の可能性がある(原因は未確認)。また `evidence/R5-2.md` は run2 の文字を「読めるかは判定できず」とするが、reviewer は `page-run2.png` で「Help us improve FL Studio…」の文字が読めたと書いている。`evidence/R5-2.md` の本文はこの決定により書き換えない。この項目は、本文より後に出た別の目視の読み(reviewer)として扱う。
- (2) 項目 5 の指摘: `evidence/R5-2-recompute.txt` の再実行は reviewer は行っていない(値の比較のみ)。
