reviewer の再検査の報告(2026-11-16、対象 e7ccb04・5b43b99)。判定: 差し戻し(2 件)。要点の保存(原文ではない)

## 判定

- 差し戻し 2 件。
- 受理: 差し戻し 1 の直しのうち、§0 の注(6 行)と §3 の git log・git diff --stat の貼付(9610708 = 2026-11-15 15:46:54 +0900、cb81958 = 2026-11-16 12:30:57 +0900、22 files・650 insertions・12 deletions を reviewer が自分で実行して一致)。差し戻し 2(S01 の 3 値は未確認に、§4.1 の注 105 行)。注 3〜5・a〜e(答え数 31〜36 は 44 セルを数えて一致、S09 を含めた腕 A の中央値 48.33(45.39〜52.61)は一致、「協調版」は 0297 の 7・26・116 行を根拠にし、この bundle の構成は未確認、0297 の引用(47 行)は原文と一致、lat-ab.py 59 行の ioStart の対応は一致)。
- 数字の不変: 表 1・表 2・§4 の数値は変わっていない。
- 60 Hz の訂正: STATE 32・performance 3・ROADMAP R4-2 の追記・R4-2c 行(todo・9 列)は R4-2c の値だけを使う。
- 範囲: e7ccb04 は 2 ファイル、5b43b99 は 3 ファイル。

## 差し戻し A(使った bundle の断定)

evidence/R4-2c.md の 6 行は「対象: work-r53k の bundle」、140 行は「今回は work-r53k の bundle」と書くが、62 行は「どちらのフォルダを使ったかは未確認」。使った場所を示す記録は無い(evidence/R4-2c に work-r5・/home/・.cache は 0 件。tools/verify-kit/out 全体に work-r5 は 0 件)。

## 差し戻し B(§9 の件数と行番号)

§9 の件数と行番号が grep と合わない。「evidence/*.md(10 件)」は実測 15 行(6 ファイル)。「R4-2c.md 6 行・105 行」は 105 行に該当語なし。「ROADMAP.md 62 行」は 63 行(R6-7)。「HEAD と一致(R4-2c.md 6・108 行)」は 108 行に該当なし(「HEAD と一致」は 6 行と 155 行。143 行は「HEAD の app/ と一致」)。

## 新しい不一致(探索範囲)

§9 の探索は docs/ROADMAP.md と docs/STATE.md に限られ、docs/archive/decisions/0244-translucent-windows.md の 117・139 行(「直前の HEAD の bundle」)が列挙に無い。過去の HEAD を指すので同じ型ではないと読めるが、探索範囲の記述と合わない。

## 任意

- evidence/R4-2c-review.md 8 行の「整合」を「一致」に。
- docs/ROADMAP.md 40 行(R3-3)の「headless は 60 Hz の限界を書く」の一般化。
- docs/ROADMAP.md 90 行の「既定の条件で画面の rAF が 60 Hz のため」の「のため」(旧の理由づけと同じ型)。

## 未確認(残る)

使った bundle の特定、manifest の HEAD 9610708 と測定時 cb81958 の wasm の同一性、app/dist と HEAD の一致、個別 T の再計算、実ディスプレイの 120 Hz・依頼者機・音(P3)、走行後の rAF 57 Hz の原因・53.03/s と 57 Hz の関係・movesPerS の頭打ち、S01 の開始前 5 列と総量 15,417 MB、pre-S04・S06・S07 の待ちの記録。

---

(以下は作成者の注。報告の一部ではない)この報告への対応: 差し戻し A・B と任意の 3 件を直した。直しの後の reviewer の再検査は未実施。
作成者の注(2026-11-16): 両方の manifest の 7 行目を読んだ(work-r53k は HEAD 9610708、work-r53h は HEAD 2f1915b)。両方の wasm を sha256sum で確かめた(どちらも c963715a4410dd56cd77e31aac44d2fd2f3b8616098cc57b9bdfae8742e6963d)。使った bundle のフォルダは、この記録からは決められない(未確認)。
