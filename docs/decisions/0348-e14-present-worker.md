# 0348: E14(present を別 Worker に出す差分)の分類と取り込みの判断(R2-8a。適用していない)

分類: 書けない。本依頼の数え上げ(下の 4)の規則では (c)(通信・変換の削減)に当たらない。(a)・(b) は本依頼では判定していない(未確認)。AGENTS §6 により、(a)・(b) のどちらかを事実で書けるまで適用しない。
撤回条件: 取り込む場合、R4-2 と同じ手順(同じ窓・交互・各 n=5 の frame/s と T(Esc) の前後)で比べ、frame/s が増えない、または Esc の T の中央値が基準より遅いなら外す。
日付: 2026-11-15   ROADMAP: R2-8   入力: `patches/e14-present-worker/`

## 決定
決定: 今回は適用しない。根拠: 下の事実。(c) に当たらず、(a)・(b) は未確認のため、AGENTS §6 により足さない。

## 事実
1. dry-run: `patch -p1 --dry-run < patches/e14-present-worker/tracked.patch` は rc=0。8 ファイルすべてが `checking file` で通り、失敗した hunk は無し(出力は `evidence/R2-8-dryrun.txt`)。
2. 新規ファイル 4 件は `patches/e14-present-worker/status.txt` の `??` 4 件と一致。行数: `desk-link.ts` 99、`desk-worker.ts` 133、`desk-link.test.ts` 76、`desk-gl-link.test.ts` 76。リポジトリ側の同じパスは 4 件とも無し(`ls` で確認)。
3. 語ごとの出現数(`expire_grep`、`literal`。件数はコード上の出現数で、通信の量〔メッセージ数・バイト数・変換の回数〕ではない。コメントの行も数える)。

| 語 | tracked.patch 追加(+) | tracked.patch 削除(-) | tracked.patch 文脈(変更なし) | new-files(全行追加) |
|---|---|---|---|---|
| postMessage | 6 | 1 | 0 | 1 |
| transfer | 1 | 1 | 5 | 1 |
| SharedArrayBuffer | 2 | 1 | 0 | 10 |
| MessageChannel | 1 | 0 | 0 | 0 |
| new Worker | 1 | 0 | 0 | 0 |
| createImageBitmap | 0 | 0 | 0 | 0 |
| ImageData | 1 | 0 | 1 | 0 |
| getImageData | 0 | 0 | 0 | 0 |
| putImageData | 1 | 0 | 1 | 0 |
| OffscreenCanvas | 2 | 1 | 1 | 6 |
| transferControlToOffscreen | 0 | 0 | 2 | 0 |

行番号(tracked.patch): postMessage 追加 47・242・250・275・362・387、削除 386(テスト用 FakeWorker の定義)。transfer 追加 43(`transfer.push(off, chan.port1)`)、削除 34(`transfer.push(off)`)。SharedArrayBuffer 追加 122・124、削除 120。MessageChannel 追加 41。new Worker 追加 37。ImageData・putImageData 追加 260、文脈 255。OffscreenCanvas 追加 122・345、削除 120、文脈 251。
行番号(new-files): postMessage・transfer は `desk-worker.ts:16`。SharedArrayBuffer 10 件は `desk-link.ts` 2・30・35・36、`desk-worker.ts` 31、`desk-link.test.ts` 5・6、`desk-gl-link.test.ts` 1・41・42(2 と 1 はコメント)。OffscreenCanvas 6 件は `desk-worker.ts` 1・24・37・38・47・129(1 はコメント)。

4. 規則の適用(件数だけで判定)。
   - 追加が削除を上回る語(増える語): postMessage(+6 / −1)、MessageChannel(+1 / 0)、new Worker(+1 / 0)、SharedArrayBuffer(+2 / −1 ほか new-files に +10)、OffscreenCanvas(+2 / −1 ほか new-files に +6)、ImageData(+1 / 0)、putImageData(+1 / 0)。
   - 同数: transfer(+1 / −1)。
   - 削除が追加を上回る語(減る語): 無し。
   - 結果: 増える語が 1 つ以上あり、減る語が無いので、(c) に当たらない。
   - 補足(重なり): ImageData の `+` 1 件は、putImageData の `+` 1 件(同じ 260 行目)の部分一致で、別の出現ではない。ImageData と putImageData は同じ出現を二重に数えている。
   - 補足(件数の読み方の限界): transfer の同数は、削除の `transfer.push(off)` が追加の `transfer.push(off, chan.port1)` に置き換わったもので、移す物が 1 つ増えたことを示す。件数は量を示さないので、通信・変換の量の増減は本依頼では言えない。

5. 0311 の引用: 「待ちの削減で、通信・変換の削減ではない」の語は、`docs/decisions/` には無い。`docs/archive/decisions/0311-present-worker.md:446` に「P の追加は『待ちの削減』で、…入力の答えの遅れ…という退行を作った。方針の (a)(b)(c) のどれにも綴じない新しい構造で、PM/オーナーで未決。」がある(`expire_grep` の `待ちの削減`、path `docs`)。「通信・変換の削減ではない」の文字列は、0311 には無い(`docs/archive/decisions/0315-rt1-real-thread-candidate.md:94` に別の対象〔実スレッド化〕についての同種の文がある)。

## 未確認
- (a)(b) への当てはまり。本依頼は (c) の数え上げだけを行った。
- 通信・変換の量(メッセージ数・バイト数・変換の回数)。未測定。
- 実エンジンの実行、ビルド、テスト。いずれも未実施。

## 補い(2026-11-15、reviewer の指摘)
1. 0311 は、P の追加が「エンジンの速度は上がった(0.889 → 1.000)」と書いている(`docs/archive/decisions/0311-present-worker.md:446`。`guilt_read` の offset 440 で確認)。本決定は、速度の改善の有無を判断の根拠にしていない。根拠は上の 4 の件数の数え上げと、AGENTS §6 の「書けないなら足さない」。
2. 5 節の `docs/archive/decisions/0315-rt1-real-thread-candidate.md:94` の引用は、確認した。94 行目に「通信・変換の削減ではない。」の文が実在する。対象は実スレッド化(協調 scheduler を実スレッドに替える話)で、E14 の present の分離とは別の対象。
3. この決定は、今回の条件(件数の数え上げと AGENTS §6)で適用しない、の意味。E14 を恒久的に採らない決定ではない。R4-4 で P2 の最大のボトルネックを測定で特定した結果、present の分離が (c) の根拠になる場合は、別の依頼で再検討する。
