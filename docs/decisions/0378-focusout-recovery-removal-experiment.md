# 0378: focusout 復帰の除去実験(R3-6a)の定義・予測・判断の規則を、測定の前に固定する
分類: (a 除去)。除去の対象は `app/src/input/pointer.ts` の `focusout` 復帰(決定 0260 の処理)。(b)(c) の対象外。
撤回条件: 本測定(R3-6b)の前に、この文書の定義・予測・判断の規則を変えない。変えるなら、理由と日付を追記し、変更前の定義の結果を別に書く。この実験の結果は「除去を main に取り込むか残すか」の選択にだけ使い、別の窓・別の道具の結果と混ぜない。取り込みは reviewer の検査の後に別の依頼で行い、この実験だけでは取り込まない。
日付: 2026-11-16   ROADMAP: R3-6   一覧: R3-6(ROADMAP の行)   コミット: この文書を先に commit して push する。その後に道具と 2 つの腕を作り、道具の確認の run を行う(`evidence/R3-6a.md`)。

## 決定

決定: 除去実験 F-11 を、腕 A(今の main のページ)と腕 B(`focusout` 復帰を除いたページ)の 2 腕で行い、File メニュー ×100 の取りこぼしを数える。ゲストは両腕とも同じ bundle(work-r53k)。本測定は R3-6b で行う。この依頼(R3-6a)は本測定をしない。根拠: ROADMAP R3-6 の「前後の回数と判断が決定文書にある」を、結果を見る前に定義を固定して満たすため。

## 定義(測定の前に固定)

- 「周期」= File メニュー(座標 18,20)の click → `TQuickPopupMenuWindow` の可視を 6 s 待つ → Escape → 消えるまで待つ。`tools/verify-kit/tools/sdriveg2-export.mjs` の `menuCycles` op(302〜321 行)の形。
- 「セッション」= `serve-once-r36-bundle.mjs` の 1 回(起動 → 台本 `scn-r36-file100.json` の `closeWizard` → `wait 2500` → `menuCycles` n=100 → `expectRunning`)。
- 「取りこぼし」= `report.json` の `menuCycles.notVisible` の要素数(6 s 以内に `TQuickPopupMenuWindow` が見えなかった周期)。0260 と同じ定義。
- 参考(判定に使わない): `menuCycles.popupMissingSurface`(窓表には見えたが、ページに surface が無かった周期)。完走した run では `popupSeen + notVisible の数 + popupMissingSurface の数 = completed` を確かめる。
- 「完走」= `menuCycles.completed` が 100。
- 「停止」= `report.json` の `state` が `exited` か `error` で、`menuCycles.stoppedAt` がある。停止までの取りこぼしは書くが、完走とは書かない。
- 「数えない回」= 道具・ページ・ドライバーのエラーで結果が読めなかった回(`menuCycles` が無い、`opErrors` が非空、`report.json` が無い)。
- 撮り直し: 数えない回と停止の回は、同じ位置で撮り直す。撮り直しは最大 3 回(試行は最大 4 回)。撮り直しの回も全部書く。撮り直しを load・swap の値で決めない。
- 各回の直前に `free -m` の 2 行・`/proc/loadavg`・`ps`(headless Chrome の行)を記録するだけ。

## 腕

- 腕 A: 今の main のページ(`app/` は変えない)。ブランチ `r3-6-arm-a`(main と同じ commit)。
- 腕 B: 腕 A から、次を除いたページ。ブランチ `r3-6-arm-b`(ローカルの commit のみ。main へ merge・push しない)。
  - `app/src/input/pointer.ts` の 78〜90 行(`userMovedAt`・`pressed`・`notePress`・`noteTab`・`isTextEntry`・`onDesktopFocusOut`)、111 行(`focusout` の登録)、112〜113 行(`notePress`・`noteTab` の登録)、120 行(`focusout` の解除)、121〜122 行(同の解除)。先頭コメントの 8 行目(決定 0260 の項)は、除いた旨の 1 行に置き換える。
  - `app/test/unit/pointer-dom.test.ts` の 43〜96 行(決定 0260 のテスト項だけ)。ほかのテスト項は変えない。

## 判断の規則(結果の前に固定)

判定は次の順で行う: (4) → (3) → (1) → (2)。

- (4) 停止・数えない回が、同じ位置で撮り直し 3 回を使い切っても残る → 「測れなかった」。その位置と理由を書き、取りこぼしの数は参考として書く。
- (3) 腕 A の取りこぼしの合計が 1 以上 → 「別の結果」。件数をそのまま書き、判断を保留する(腕 B の値によらない)。
- (1) 腕 A の合計が 0 で、腕 B の合計が 1 以上 → 「残す」。除去は取り込まない。引き金(ブラウザがフォーカスを移す原因)の探索は別の行(別の決定)にする。
- (2) 腕 A と腕 B の合計がどちらも 0 → 「この機械・headless Chrome・600 周期では再現せず」。除去の取り込みはこの実験では決めない。

## 予測(測定の前に書く)

- main の予測: 腕 B で 1 件以上(0260 の再現)、腕 A は 0 件。根拠: 0260 の A+B+C の割合 19/2,760(0.69%)。
- 感度(計算。下の出力を参照): 0.69% が今も同じなら、腕 B の 600 周期で取りこぼし 0 件になる確率は 0.0158(約 1.6%)。「約 30 回に 1 回(3.3%)」なら 1.5e-9。両方とも「旧の別の測定の割合」であって、今の割合ではない。
- 外れ得る: 0260 は 2026-11-06 のページと bundle の測定。その後、ページ・bundle は変わった。

計算(実測ではない。`python3` で計算):

```
python3 -c "p1=19/2760; p2=1/30; print((1-p1)**600, (1-p2)**600, (1-0.0069)**600)"
```

出力(この文書の作成時): `0.015848…`(19/2760 の場合)、`1.466e-09`(1/30 の場合)、`0.015696…`(0.0069 の場合)。

## 組(R3-6b で行う。この依頼では行わない)

- 各腕 6 セッション = 600 周期。腕 A・腕 B・腕 A・… の順に交互に 12 セッション(腕 A から)。直列。
- 各回: `node tools/verify-kit/serve-once-r36-bundle.mjs <bundle の絶対パス> r36<label>`(foreground、`timeout` 1800000)。bundle は `~/.cache/signature-studio/work-r53k/bundle/fl-Oz`。出力は各腕のワークツリーの `tools/verify-kit/out/`。
- 各回の前に、上の「定義」の記録を取る。

## 未確認

- 依頼者の機械・実機の Chrome・Firefox での再現は 0 件。この機械の headless Chrome 156 だけ。
- 引き金は未特定。0260 の読み(headless Chrome の挙動)と同じく推測。
- 「約 30 回に 1 回」の出どころは未確認。候補は `app/src/input/pointer.ts` の 8 行目の「about once in 30 clicks」。0260 の 0.69%(約 145 回に 1 回)と食い違う理由も未確認。
- 支援技術の経路(0260 の副作用の点検の論点)は測らない。

## 出典

- `app/src/input/pointer.ts` の 8 行目、78〜90 行、111〜122 行。`app/test/unit/pointer-dom.test.ts` の 43〜96 行。
- `tools/verify-kit/tools/sdriveg2-export.mjs` の `menuCycles`(302〜321 行)。台本の形は `tools/verify-kit/tools/scn-client.json` の `d8-file-menu-100`。
- `docs/archive/decisions/0260-file-menu-miss.md` §0(19/2,760)。
- ROADMAP R3-6 の行。前例: `docs/decisions/0349-remove-pixel-dedup.md`、`evidence/R3-1a.md`、`evidence/R3-1d.md`(別ブランチの腕)。
- 決定文書の形: `docs/decisions/0377-export-sha-observation.md`。

## 結果の追記(2026-11-16、`evidence/R3-6b.md`)

- 取りこぼしの合計: 腕 A 0 件(600 周期)、腕 B 0 件(600 周期)。12 セッションすべて完走(停止・数えない回・撮り直しは 0 回)。
- 判断の語: 再現せず(規則 (2) に当てはまる。この機械・headless Chrome・600 周期では再現せず)。
- 予測との対応: 腕 B で 1 件以上は外れ、腕 A は 0 件は当たり。
- 除去の取り込みは、この実験では決めない(規則 (2))。取り込みは reviewer の検査の後の別の依頼にする。初回の検査は差し戻し(evidence/R3-6b-review.md)。差し戻しの後の文面直し(b754613)の再検査も差し戻し(2 件。文面の件)。3 回目の検査(対象 276787c)は差し戻し 1 件(evidence/R3-6b.md の 37 行)。evidence/R3-6b-review3.md。4 回目の検査(対象 338f9a6・2d797d1)は差し戻し 1 件(この行の検査回数の記述。evidence/R3-6b-review4.md)。この直しの後の reviewer の再検査は未実施。判定の範囲と読みは `evidence/R3-6b.md` §5。
