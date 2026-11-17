# R3-1: reviewer の検査の報告(コピー)

下は reviewer(2026-11-15)の最終報告の全文のコピー。コピーした worker はこの検査を再実行していない。検査対象は commit `5ecdd76`・`8379215` と、R3-1a・R3-1b・R3-1d・R3-1e の記録(`evidence/R3-1a.md`・`R3-1b.md`・`R3-1d.md`・`R3-1d-recompute-all.txt`)と `docs/decisions/0349-remove-pixel-dedup.md`。この前に、同じ記録について reviewer が差し戻している(数えない回の読みの食い違い・括弧書きの事実)。それらの報告のコピーは保存していない(未保存)。直しは `evidence/R3-1d.md` の補い 2・先頭注と `evidence/R4-3c.md`・`evidence/R4-3d.md` の補いにある。

## reviewer の報告(全文)

検査は完了しました。読み取りだけで、編集・実行はしていません。shell の `grep` と `cmp`・`du` は権限で拒否されたため、その分は未確認です。

1. **受理**: `git show --numstat` は 5ecdd76 が R3-1d.md 43・0、R4-3c.md 41・0、8379215 が R4-3d.md 61・0 だけ。`git diff -U0` の `-` 行は 0。`git status --short` は `?? evidence/R0-6-failfast.txt` のみ。
2. **一部未確認**: bf9263e は 07:34:30 +0900。13 run の birth と report.json の更新を `stat` で読み、表の時刻と前後の判定はすべて一致。撮り直し(r31-5・r25-6)は run 作成が commit 後なので「後」は確認。位置 3(r25-2)の再生 07:34:38 は `marks.play` を読めず未確認。r25-2 の audio-p3.json では consumed>0 が 07:33:58 JST のパルスから現れており、補いが「再生開始」を定義していないため 8 秒後の判定は確かめられない。
3. **受理**: loadStart 10 件(0.17, 3.10, 2.37, 1.88, 1.92, 3.19, 2.27, 2.31, 2.43, 2.71)は補いの表と一致。§3 と表 A は 1〜9 が同値、10 が 2.94 対 2.71。R4-3c-recompute.txt の loadStart 列も一致。
4. (i) **受理**: 10 行目は無変更、先頭注は 1 行目に追加。ok・stopped・error・state は 10 run で計 50 件。(ii) **受理**: 10 位置の比較(同じ 4、違う 6)は読んだ値と一致。(iii) **未確認**: cmp は呼べない。`wc -c evidence/R4-3d/*/*` の合計 12,981,855 は補いと一致。(iv) **受理**: /tmp/opencode に pos1〜10 があり、pos1 は 8 行、loadavg 行は 0 件。
   - 指摘(差し戻し理由にしない): 補いの「loadStart の書き込み元は 0 件」は検索が不完全。`tools/verify-kit/tools/sdriveg2-lat.mjs` の 641・658 行が書いている。行番号もずれ(「行 35」は 37、「108〜117」は 110〜119)。
5. **受理**: 2 commit は recompute の 3 ファイルに触れていない。
6. **受理**: 判定語・秘密語の検索で、「直した」は R4-3d の 237・246・251 行の引用のみ。ses_・token 系は 0 件。
7. 括弧書きに足すべき事実:
   - 主指標は underrun(0〜300 s)。書き込み率と Worker CPU は副指標(判定に使わない)。BitBlt CPU は未測定。ROADMAP の行が挙げる p3-win.py は記録の計算に使われていない(計算は p3-recompute-dirs.py)。
   - R3-1b は起動確認 n=3 のみで、P3 は測っていない。boot の道具の差は未確認(R3-1b.md の 1・161 行)。
   - 主の読み B は、測定の途中(位置 3 の前)で入った bf9263e の補い (5) に基づく。位置 1〜2 はそれより前に測定。
   - 「reviewer 受理 2026-11-15」は、この検査の結果より前に書かれた判定なので削る。

総合判定: **受理**(R3-1 を main の決定案のとおり done にしてよい。括弧書きに足すべき事実: 7 の 4 点)。数字・事実の不一致、既存行の書き換え、肯定の判定語、秘密は 0 件を確認した。未確認 2 件(項目 2 の位置 3 の再生時刻、項目 4(iii) の cmp)は報告どおり未確認として残す。

## reviewer の指摘の扱い

1. 「項目 2 の未確認: 位置 3 の再生開始(07:34:38)の `marks.play` は reviewer が読めなかった。`evidence/R3-1d.md` の補い 2 の『再生開始』の定義は、`audio-p3.json` の `marks.play`(epoch ms を JST に換算)。この未確認はそのまま残す」
2. 「項目 4 の指摘: `evidence/R4-3d.md` の補いの『`loadStart` の書き込み元を検索して 0 件』は検索が不完全で、`tools/verify-kit/tools/sdriveg2-lat.mjs` の 641・658 行付近が書いている(reviewer の読み)。補いの行番号にずれがある(『行 35』は 37、『108〜117』は 110〜119)。補いは書き換えず、この指摘を本ファイルに残す」
3. 「項目 4(iii) の `cmp` は reviewer が呼べず未確認。補いの『30 件すべて rc=0』は worker の実行の記録」
