# テーマのタイルが見えない件(Issue #1505、2026-11-27 UTC)

判定: 受理。原因は E2E の探し方の誤り(b)。製品は変えていない。R10(`theme-switch-all-13`)の印を外し、2 表示で 13 件を切り替えた。

実エンジン: `fe-build engine`・`fe-build bundle`(exit 0。23:59〜00:50 UTC)、`tools/boot-check/boot-bundle.mjs`(engine-idle・standIn false・error null・`elapsedMs` 12,482)、`tools/smoke/welcome-flow.mjs`(流れ A〜D 4/4 通過、B・D の `welcomeClosed` true)、`tools/theme-switch/measure.mjs --dpr 1 --shots`(00:52〜00:56 UTC。13/13、停止 0)、`tools/e2e/run.mjs --scenarios regression --only theme-switch-all-13 --displays 4k-dpr1,4k-dpr2`(修正前の R10 と、修正後の R10 の 2 回)、`npm test --prefix app`(通過)、`node --test` の E2E の単体(visual・scenario・coverage・report-only。通過)、`bash guard/scan.sh`(0 件)。測定の値は runner の値。この機械とは別枠。nproc 4、loadavg 2.3〜2.5。

## 1. 再現(実測)

修正前の R10 の出力(両表示で同じ):

```
theme "Ash": no tile in the Theme page (captions there: ["Presets","Adjustments","Options"])
switches: Ash not found
```

製品側の測定(`measure.mjs --dpr 1 --shots`)の Theme 頁の窓の表(子の窓。頁の左上を基点にした矩形):

| 子の窓 | クラス | 文字 | 矩形(guest px) |
|---|---|---|---|
| 177 | TVectorPanel | (空) | 590,143 656x829 |
| 178 | TVectorPanel | Presets | 614,159 608x494 |
| 180 | TVectorPanel | Adjustments | 614,661 608x169 |
| 181 | TVectorPanel | Options | 614,838 608x81 |
| 179 | TQuickStringGrid | (空) | 614,186 608x467 |
| 182 | TQuickScrollBoxScroller | (空) | 1205,186 17x467 |

テーマのタイルは、窓の表の子に無い。タイルの名前は、`TQuickStringGrid`(179)の画素として描かれている。

画素(`.tmp-shots-dpr1/theme-page-x.0.png` を `pm-work/findings/theme-tiles-1505-theme-page.png` に保存): 格子の 1 行目〜3 行目に、`(Default)`・`DEDZII Light`・`DEDZII`・`PPRIIME Dark`・`PPRIIME Light`・`Veela`・`Ash`・`Dark orange`・`Dark` の名前が読める。

スクロール(つまみのドラッグ。`measure.mjs` の `gridPoint`): 格子の内容は 238 px 動く。各タイルの位置は、上端か下端の画素から求めた(例: `DEDZII Light` は (918, 233)、`Ultra green` は (918, 562))。13 件とも、画面は押した後に変わった。

## 2. 原因

- **製品の欠陥ではない**: 格子は描画されている(画素。スクロールで 238 px 動く)。名前も読める(画素)。エンジンは押したタイルのファイルを、ログに書く(修正後の R10 で 13 件とも `named`)。
- **E2E の誤り**: 旧 R10 は、テーマの名前を窓の表の文字で探した(`w.text === name`)。タイルは窓ではないので、名前の文字を持つ窓は無い。Theme 頁の文字は Presets・Adjustments・Options だけ(上の表と、R10 の出力の `captions there`)。
- 旧 R10 は、ホイールで格子をスクロールしていた。ホイールは DPR 1.23 では格子に効かなかった(`measure.mjs` の記録。原因は未確認)。つまみのドラッグに変える。

## 3. 直したこと(E2E だけ。製品は変えていない)

- `tools/e2e/scenarios/regression/r10-theme-switch-all-13.mjs`:
  - テーマの一覧は `inputs/fl-installer-files.tsv` の `Artwork\Themes\` から作る(`themeOrderNames`。名前の表は書かない)。
  - タイルの位置は画素で求める(`lib/visual.mjs` の `tilePoint`。つまみのドラッグ)。窓の表の文字は使わない。
  - 押した後に、エンジンのログが、そのテーマの `.flstheme` を名指すこと(`named`)を、判定に加えた。
  - 見つからないテーマがあっても、残りを続ける(13 件の行を全部書く)。
  - 印(`expectFail`)を外した。
- `tools/e2e/lib/visual.mjs`: `tilePoint` を export した(関数の中身は変えていない)。
- `tools/e2e/scenarios/regression/common.mjs`: `OPEN_ISSUES` から `#1505` を外した(R10 の印だけが使っていた)。

## 4. 13 件の切り替え(修正後の R10。両表示で同じ結果)

| # | テーマ | タイル | 見つかった | 状態(3 秒後) | 画面 | エンジンのログ(`.flstheme`) |
|---|---|---|---|---|---|---|
| 1 | DEDZII Light | 1 | ○ | engine-idle | 変わった | 名指した |
| 2 | DEDZII | 2 | ○ | engine-idle | 変わった | 名指した |
| 3 | PPRIIME Dark | 3 | ○ | engine-idle | 変わった | 名指した |
| 4 | PPRIIME Light | 4 | ○ | engine-idle | 変わった | 名指した |
| 5 | Veela | 5 | ○ | engine-idle | 変わった | 名指した |
| 6 | Ash | 6 | ○ | engine-idle | 変わった | 名指した |
| 7 | Dark orange | 7 | ○ | engine-idle | 変わった | 名指した |
| 8 | Dark | 8 | ○ | engine-idle | 変わった | 名指した |
| 9 | Light cherry | 9 | ○ | engine-idle | 変わった | 名指した |
| 10 | Light pear | 10 | ○ | engine-idle | 変わった | 名指した |
| 11 | Light tangerine | 11 | ○ | engine-idle | 変わった | 名指した |
| 12 | Sytrus | 12 | ○ | engine-idle | 変わった | 名指した |
| 13 | Ultra green | 13 | ○ | engine-idle | 変わった | 名指した |

4k-dpr1(DPR 1): `ok` true、`pageErrors` 0、`consoleErrors` 0、`failedRequests` 0、`stuckToasts` 0、`rows` 13。4k-dpr2(DPR 2): 同じ。

## 5. 未確認

- 格子の各タイルの見本の絵は、`(Default)` だけが実際の画像で、他は汎用の画像のアイコン(上の画素)。原因は未確認。受け入れの項目ではない。
- 起動の `elapsedMs` は 12,482 で、基準の 12,500 にほぼ届いている(runner の値)。
- `Artwork\Themes\__current.flstheme` の存在の確認は、起動の間の要求の記録(`missing-paths.md`)のとおり。この件とは別。
