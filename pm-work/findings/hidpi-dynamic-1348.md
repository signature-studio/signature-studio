# hidpi-dynamic-1348: Issue #1348 の測定と結果(受理。未確認の項目あり)

測定の値は runner の値。この機械とは別枠。nproc = 4。loadavg は作業の中で 0.3〜2.7(測定の最中は 2.4〜2.7)。

実エンジン: 受け入れコマンドの各段を順に実行した。`npm run build --prefix app` → `cargo build --release --locked` → `fe-build engine` → `fe-build bundle` → `boot-bundle.mjs`(engine-idle・error null・standIn false)→ `measure-size.mjs --dprs 1,1.25,1.5,1.75,2,2.5,3 --windows main,welcome,mixer,playlist,piano-roll,plugin,tooltip,menu --random 3 --random-range 1,3 --dynamic 1,2.04,2,1`(出力 84 行、終了 0)→ `browser-panel/measure.mjs`(blackShare 0、engine-idle、standIn false)→ 判定 `judge: 通った`(終了 0)。ソースの検査: 追加行 40 行、禁止の値 0 件。`npm test --prefix app`: 71 files passed・1 skipped、1403 tests passed・10 skipped。ハーネス(`tools/` を変えたため): `spec/guard.test.sh` 67 件 ok、`spec/workflow.test.sh` 442 件 ok。

## 判定

受理(判定の基準どおり)。ただし、下の「未確認」の項目は判定の外か、判定の数値が根拠にならない。

2 回目の受け入れ(build・起動確認・測定・判定を、乱数と動的の中間を選び直して実行。終了 0、`judge: 通った`)も通った。乱数は seed 1465846036、values [1.99, 1.17, 1.22]、動的の中間は 1.83。採用段は、1.99 → DPI 191 → 200、1.17 → DPI 112 → 125、1.22 → DPI 117 → 125 で、いずれも `adp` と一致した。主窓のポインタ誤差は 0、判定の外の段 2・3 の縁の問題(2 節の注)も同じく残る。

## 1. 拡大率ごとの表(静的。各 1 起動)

FL の係数は、FL のログ行 `Main GUI scaling factor: F (system set to N PPI)` の F × 100。DPI は engine の LOGPIXELSY。

| DPR | DPI | 採用段(FL の F × 100) | main の縁 (px) | menu の縁 (px) | 起動 |
|---|---|---|---|---|---|
| 1 | 96 | 100 | 2.243 | 2.194 | engine-idle |
| 1.25 | 120 | 125 | 2.410 | 2.194 | engine-idle |
| 1.5 | 144 | 150 | 2.536 | 2.194 | engine-idle |
| 1.75 | 168 | 175 | 2.636 | 2.194 | engine-idle |
| 2 | 192 | 200 | 3.199 | 3.038 | engine-idle |
| 2.5 | 240 | 250 | 3.543 | 0.800 | engine-idle |
| 3 | 288 | 300 | 3.048 | 0.800 | engine-idle |
| 2.36(乱数) | 227 | 250 | 3.543 | 0.800 | engine-idle |
| 1.79(乱数) | 172 | 175 | 2.635 | 2.194 | engine-idle |
| 1.81(乱数) | 174 | 175 | 2.636 | 2.194 | engine-idle |

- 採用段は、全 10 段で `adp`(DPI ÷ 96 × 100 を 25 刻みの最近接に丸め、丸めが 225 のときは p < 225 なら 200、それ以外は 250)と一致した。新しい点: DPI 227 → 250(p = 236.5)。前回の DPI 205 → 200 と合わせて、225 の段が無いことと整合する。225 の境目の正確な値は、DPI 206〜226 を測っていないので未確認。
- 静的の 80 行は全て engine-idle・standIn false・guest と canvas と display が DPR に追随・Main GUI Scaling が System(ログの `system set to N PPI`)。
- 窓の種類: main と menu は全段で縁を測れた。welcome・mixer・playlist・piano-roll は縁を測れなかった(null。DPR 1.75 の welcome だけ 4.204 px。DPR 1 の基準が無いので判定しない)。plugin と tooltip は開けなかった(8 節)。
- DPR 2 の縁(main 3.199、menu 3.038)は DPR 1(2.243、2.194)より広い。判定の式(DPR に比例して広がる上限)の内側だが、縁の幅が広がったことの意味(ぼけか、枠の描き方か)は、画面を見ずに断定できない。未確認。
- DPR 2.5 と 3 の menu の縁は 0.80 px(鋭い縁)。前回の閾値の問題(コントラストが 28 で閾値 40 を下回った)は、`MENU_MIN_CONTRAST`(20、向きを問わず)で測れるようになった。

## 2. 乱数と動的の変化

- 乱数の DPR: seed 983630966、values [2.36, 1.79, 1.81]。動的の中間の値: 2.04(受け入れのたびに乱数で選ぶ)。
- 動的(1 回の起動の中で DPR を 1 → 2.04 → 2 → 1):

| 段 | DPR | guest(engine の報告) | canvas = display | DPI | 採用段 | 案内 | `WM_DISPLAYCHANGE` の行 | `WM_DPICHANGED` | DPI の問い合わせ | main の縁 |
|---|---|---|---|---|---|---|---|---|---|---|
| 1 | 1 | 1836×1056 | 1836×1056 | 96 | 100 | false | 0 | 0 | 0 | 2.261 |
| 2 | 2.04 | 3745×2154 | 3745×2154 | 96 | 100 | true | 1 | 0 | 0 | 3.505(2 節の注) |
| 3 | 2 | 3672×2112 | 3672×2112 | 96 | 100 | true | 1 | 0 | 0 | 1.605(2 節の注) |
| 4 | 1 | 1836×1056 | 1836×1056 | 96 | 100 | false | 1 | 0 | 0 | 2.244 |

- engine のログ: `[err] input: WM_DISPLAYCHANGE 3745x2154 dpi 96 to 27 top-level windows`(段 2)、`3672x2112 dpi 96 to 27 top-level windows`(段 3)、`1836x1056 dpi 96 to 27 top-level windows`(段 4)。DPI は 96 のまま、採用段は 100 のまま。
- ページの例外: 0。DPI の問い合わせ(GetDeviceCaps 等): 段 2〜4 で 0 件(FL は再度尋ねない)。
- 注(判定の外。根拠にならない): 段 2・3 の main の縁は、Tempo 枠の検出が別の場所を測ったもの。検出の範囲は guest の幅の 0.2〜0.3 倍(`tempoBox`)で、FL の拡大率は動的の間は変わらず、上部バーは guest の同じ位置(DPR 1 の x ≒ 436〜492)に残る。段 2 の検出は x = 1090、段 3 は x = 740 で、どちらも Tempo ではない(段 1・4 は x = 464 で正しい)。よって段 2・3 の縁の数値は、判定の基準を満たしても、動的の縁の根拠にはならない。推定(検出の数値から)。直すのは #1381。

## 3. WM_DPICHANGED を送ったか

送らなかった。理由: FL は DPI と拡大率を起動時に読み、再起動まで変えない(FL の仕様)。runtime の DPI は起動時の DPR から決まり、動的の間は 96 のまま(WM_DISPLAYCHANGE の行に `dpi 96`)。DPI が変わらないのに WM_DPICHANGED を送ると、FL が適用しない変化を伝えることになる。実機の Windows の FL が、DPI の変化の通知に何をするかは未確認(この機械では確かめられない)。

## 4. 再起動の案内

既存の `#restart-note`(`data-restart-notice`)を使う。拡大率の百分率(round(DPR × 100))が起動時と違う段だけ出る(段 2・3 で出て、段 1・4 で消えた)。文言は「表示の拡大率が変わりました。拡大率は FL Studio の再起動で適用されます」。自動で再起動を提案しない(FL の再起動は利用者の操作)。

## 5. ポインタ

主窓の 3 部品(Tempo・song-position・pattern-selector)の中心を押した結果の `pointerErrPx` は、静的の 10 段の全てで 0 画素(`pointerSentErrPx` も 0)。

## 6. Debug ログ

全行(静的 80 行・動的 4 行)で `debugLog.captured` が true。`scalingLines` の error・warn は 0 行(全行の合計 0)。

## 7. 測り方の注意

- `browser-panel/measure.mjs` の blackShare は DPR 1 だけで測る道具(`const DPR = 1`)。DPR 1 のページの画面の黒は 0。他の DPR のページの画面の黒は未測定。guest の canvas の黒は 0.929(判定の外)。
- Settings(F10)の窓は、10 段のうち 5 段(DPR 1.25・1.5・1.75・1.79・1.81)でしか開かなかった。残りの段では、F10 の後 15 秒以内に設定の窓(TMIDIForm)が出なかった。拡大率の係数は起動時のログ(idle の時点の guest のメモリ)から読んだので、判定には影響しない。Display の選択そのものは、この道具では見ていない。

## 8. 開けなかった窓

- plugin(全段): このプロジェクトに内蔵のネイティブプラグイン(Fruity Balance)が無く、道具はミキサーの挿し込みをしない。#1381 で扱う。
- tooltip(全段): 上部バーの 3 点に 2.5 秒ずつホバーしても、新しい最上位の窓が出ない。#1381 で扱う。

## 9. 部品の座標と Tempo 枠の検出(判定の外)

Tempo の枠の中心(guest 画素。`parts.tempo.cx, cy`):
- 静的(DPR 1〜3 と乱数): 1 → (464, 20)、1.25 → (580, 25)、1.5 → (695, 31)、1.75 → (811, 36)、2 → (928, 41)、2.5 → (1160, 51)、3 → (1392, 61)、乱数 2.36 → (1160, 51)、1.79 → (811, 36)、1.81 → (811, 36)。乱数の値は FL の採用段と同じ位置に来た(外れなし)。
- 動的: 段 1・4 (DPR 1) → (464, 20)、段 2 (DPR 2.04) → (1090, 57)、段 3 (DPR 2) → (740, 61)。段 2・3 は外れた(2 節の注。#1381)。

## 10. 確認できなかったこと

- Windows の実機の FL の拡大率の段(この機械では FL のコードの結果しか見られない)と、DPI の変化に対する FL の実際の反応(WM_DPICHANGED を含む)。
- DPR 2.2〜2.4 のうち、DPI 206〜226 の段の境目(225)の正確な位置。
- 縁の幅が DPR 2 で広がる意味(ぼけか枠の描き方か。画面を見ていない)。
- 動的の段 2・3 の縁(Tempo の検出が外れた。#1381)。
- plugin と tooltip の窓(#1381)。
- 起動の停止(#1393)は、今回は 1 回ずつの起動(DPR 2.5 と 3 を含む)で起きなかった。繰り返しの確認は、#1393 の担当。

## 11. 表で判定した本試行(採用段は `fl-scale-steps.json`。式は使わない)

測定の値は runner の値。この機械とは別枠。nproc 4、loadavg 2.0〜2.2。受け入れのコマンドの段を順に実行した(ビルド・起動確認・測定・表の測定・ページの黒・判定・ソースの検査・単体テスト)。判定: `judge: 通った`(終了 0)。ソースの検査: 禁止の値 0 件。`npm test --prefix app`: 71 files passed・1 skipped、1408 tests passed・10 skipped。乱数: seed 3773212210、values [2.78, 1.49, 2.06]。動的の中間: 1.51。

| DPR | DPI | 採用段 | main の縁 (px) | menu の縁 (px) | 主窓のポインタ誤差 |
|---|---|---|---|---|---|
| 1 | 96 | 100 | 2.243 | 2.194 | 0 |
| 1.25 | 120 | 125 | 2.410 | 2.194 | 0 |
| 1.5 | 144 | 150 | 2.536 | 2.194 | 0 |
| 1.75 | 168 | 175 | 2.636 | 2.194 | 0 |
| 2 | 192 | 200 | 3.199 | 3.038 | 0 |
| 2.5 | 240 | 250 | 3.543 | 0.800 | 0 |
| 3 | 288 | 300 | 3.048 | 0.800 | 0 |
| 2.78(乱数) | 267 | 300 | 3.048 | 0.800 | 0 |
| 1.49(乱数) | 143 | 150 | 2.532 | 2.194 | 0 |
| 2.06(乱数) | 198 | 200 | 3.199 | 3.038 | 0 |

- 静的の 10 段 × 窓 8 種 = 80 行。engine-idle・standIn false・guest と canvas と display が DPR に追随・Main GUI Scaling が System・debugLog が captured(error・warn の行 0)。
- 開けなかった窓: plugin と tooltip の 20 行(#1381)。welcome・mixer・playlist・piano-roll の縁は null(判定しない。welcome は DPR 1.75 と 1.49 で 4.204 px)。
- ページの画面の黒(`browser-panel/measure.mjs`、DPR 1): 0。guest の canvas の黒は 1(判定の外。Browser の面は guest の画面に無い)。
- 動的(1 起動で DPR 1 → 1.51 → 2 → 1):

| 段 | DPR | guest(engine) | canvas = display | DPI | 採用段 | 案内 | WM_DISPLAYCHANGE | WM_DPICHANGED | DPI の問い合わせ | main の縁 |
|---|---|---|---|---|---|---|---|---|---|---|
| 1 | 1 | 1836×1056 | 1836×1056 | 96 | 100 | false | 0 | 0 | 0 | 2.261 |
| 2 | 1.51 | 2772×1595 | 2772×1595 | 96 | 100 | true | 1 | 0 | 0 | 1.689(判定の外) |
| 3 | 2 | 3672×2112 | 3672×2112 | 96 | 100 | true | 1 | 0 | 0 | 1.605(判定の外) |
| 4 | 1 | 1836×1056 | 1836×1056 | 96 | 100 | false | 1 | 0 | 0 | 2.244 |

- engine のログ: 段 2 `[err] input: WM_DISPLAYCHANGE 2772x1595 dpi 96 to 27 top-level windows`、段 3 `3672x2112 dpi 96 ...`、段 4 `1836x1056 dpi 96 ...`。DPI は 96 と採用段は 100 のまま。ページの例外 0。
- 段 2・3 の Tempo 枠の検出は外れた(中心が (695, 61) と (740, 61)。DPR 1.5 と 2 の正しい位置は (695, 31) と (928, 41))。判定の外(#1381)。
- WM_DPICHANGED は送らなかった(理由は 3 の節と同じ。FL は DPI と拡大率を起動時に決め、再起動まで変えない)。
- 再起動の案内は、拡大率の百分率が起動時と違う段 2・3 だけに出た(既存の `#restart-note`)。
- 採用段の表(`tools/feat-c1-hidpi/fl-scale-steps.json`)の判定: 静的の 10 段・表の 65 点・動的の 4 段の採用段は、全て表と一致した。
