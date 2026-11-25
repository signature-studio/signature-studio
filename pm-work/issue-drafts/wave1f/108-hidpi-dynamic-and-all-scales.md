---
draft: "108"
title: "Fractional Scaling の動的な変化と、全ての拡大率(100%〜300%)・全ての窓の種類の検証"
type: Task
milestone: "M2 HiDPI と P2 の測定(P2)"
parent: "005"
blocked_by: ["107", "113", "114", "149"]
labels: ["hidpi", "verify"]
size: L
priority: High
project: signature-studio/3
---

## 目的

#{{107}} で、起動時の DPR に対して Windows と同じ Fractional Scaling を再現した。この Issue は、その結果を全ての拡大率と全ての窓の種類で確かめ、ブラウザのズームや窓の移動による DPR の変化(動的な変化)で、FL の仕様(拡大率は再起動で適用される)に合った振る舞いを確かめる。

依頼者の要求: 全ての GUI が操作できる大きさで、ぼやけないこと。拡大されてぼやけた画面は棄却する(依頼者の指示)。特定の拡大率の数字を、定数・表・seed として予め用意しない(拡大率は、環境の DPR から Windows と同じ計算で導く)。HiDPI は依頼者の実機で正常に動いている(2026-11-24 確認)ので、この Issue の範囲は、動的な変化と全ての拡大率の確認に限る。

## 背景

- #{{107}} の道具 `tools/feat-c1-hidpi/measure-size.mjs` を、この Issue で拡張して使う(下の「やること」1)。
- FL の仕様(FL 公式マニュアルの General settings の Display の節。実機では未確認):
  - Main GUI Scaling・Pop-ups・Toolbars・Legacy scaling は、FL の再起動で効く。実行中に変えても適用されない。
  - ネイティブプラグインの GUI は、100%・200%・300%・400% の整数だけ(Legacy scaling)。
  - Cursors scaling と Thick lines(200% 以上の線の太さ)がある。
- Windows の動的な変化: DPI が変わると、DPI 対応のアプリは `WM_DPICHANGED` を受け取り、画面の大きさが変わると `WM_DISPLAYCHANGE` が届く。ただし FL は拡大率を再起動で適用するので、Windows の振る舞いは FL の仕様より優先しない。
- Windows の FL の実際の挙動は、この機械では確かめられない。確認できない部分は「確認できない」と報告に書く。
- 起動用の実行ファイル: `FL64.exe`(マニフェストに `dpiAware True/PM`)と `FL64 (scaled).exe`(DPI の宣言なし。Windows が拡大する形と読める。推定)。この製品は `FL64.exe` の振る舞いに合わせる。`(scaled)` の方式は、この Issue の範囲外。
- 既存の経路: 画面の大きさの更新は `app/src/main.ts`(`applyScreenParam`・`applyDisplay`・DPR の監視)と `lift/runtime` の `WM_DISPLAYCHANGE`(`grep -rn WM_DISPLAYCHANGE lift/runtime` で探す)。
- DPI が 240 以上(DPR 2.5 以上)の起動は、#{{114}} で直った。しかし、#{{112}} 段 A(commit `eb118ef2`)で、この修正の効果は無効になった(起動が止まる。下の「前回の試行で分かったこと」の 3)。原因の Issue は #{{149}}。この Issue では DPR 2.5〜3.0 を含めて確かめる。#{{149}} の修正は main に入っており、DPR 2.5 の行も測れる(前回の試行で、DPR 2.5 と 3 は各段 1 回ずつ止まらなかった)。
- #{{113}}(FL の UI の拡大の実測)と #{{115}}(Browser のパネル)と #{{118}}(WebView2 の面)の結果を使う。

## 前回の試行で分かったこと(run 45501335817。`failed`)

前回の試行: run 45501335817 の作業 #1348 のジョブ(22:33〜02:00 UTC)。作業者は 22:33〜00:53 UTC(2 時間 20 分)、判定の段は 00:53〜02:00 UTC(67 分)。判定は終了コード 1 で、push は無し。作業は保存された(差分 `work.diff`。commit `4e5d7bf9` と `71675632` の内容を含む)。次の試行は、その差分を `.prior-work.md` と共に作業ツリーで受け取る。

- 作業者の受け入れは 2 回とも通った(`judge: 通った`)。1 回目の乱数は 2.36・1.79・1.81(動的の中間 2.04)、2 回目は 1.99・1.17・1.22(動的の中間 1.83)。
- 判定の段の再実行で落ちた: 乱数の DPR 2.81(DPI 270)の静的の行。8 窓の全行で `judge: 失敗 static`(`judge.txt` の 11240〜11241 行)。判定の他の基準は全て通った(判定の行で `static=false` だけ)。
- 外れたのは採用段の式だけ(旧: DPI ÷ 96 × 100 を 25 刻みの最近接に丸め、丸めが 225 の時だけ例外)。FL の採用段の実測の値は、判定の出力に出ていない(未確認。表の測定で確かめる)。
- 今回の 2 回の受け入れで、式と合った点: DPI 191 → 200(DPR 1.99)、DPI 227 → 250(DPR 2.36)、乱数 1.17・1.22・1.79・1.81。
- 式が未確認の範囲: DPR 2.5 より上の大半(2.81 はその中の 1 点)。
- DPR 2.5 と 3 は、2 回の受け入れで各 2 回、計 4 回とも止まらなかった。DPR 2.5 の main の縁は 3.543 px、3 は 3.048 px。menu の縁は 2.5 と 3 の両方で 0.800 px(鋭い)。
- 静的の 10 段(窓 8 種。80 行)で開けなかったのは plugin と tooltip の 20 行(#1381)。welcome・mixer・playlist・piano-roll は縁が null(判定しない)。main と menu は全段で縁が測れた。
- DPR 2 の縁: main 3.199 px、menu 3.038 px(DPR 1 は 2.243 px と 2.194 px)。広がりがぼけか枠の描き方かは未確認(画面は見ていない)。
- 動的(1 起動の中で 1 → 2.04 → 2 → 1): 段 2・3・4 で `WM_DISPLAYCHANGE` が 1 件ずつ。`WM_DPICHANGED` は 0 件。DPI(96)と採用段(100)は起動時の値のまま。再起動の案内は段 2・3 だけに出た。canvas と表示は全段で一致し、ページの例外は 0。
- 段 2・3 の main の縁(3.505 px と 1.605 px)と Tempo 枠の検出の外れは、根拠にならない(判定の外。#1381)。
- Settings(F10)の窓は、10 段のうち 5 段でしか開かなかった(係数は起動時のログから読んだので、判定には影響しない)。
- ページの画面の黒は DPR 1 で 0(判定の値)。guest の canvas の黒は 0.929(判定の外)。
- 測定の記録は `pm-work/findings/hidpi-dynamic-1348.md`(前回の差分に在る)。測定の値は runner の値(nproc 4・loadavg 0.8〜2.7)。作業者の 1 回の受け入れは、ビルドを含めて約 60 分。

## 1 回前の試行で分かったこと(run 45357788913)

前回の試行: run 45357788913(約 3 時間)。判定は終了コード 1 で、push は無し。作業は commit `89d21bb9` として保存された。次の試行は、その差分(前回の作業)を作業ツリーで受け取る(`git status` で載っているかを確かめる。`.prior-work.md` が在れば先に読む)。engine の実験の変更は取り除いてあり、`lift/runtime/fle_thread.c` は HEAD に戻してある。

- 実測の値は runner の値(nproc 4・loadavg 1.1〜1.9)。採用段は FL のログ行 `Main GUI scaling factor` の値。DPI は round(96 × DPR)。

| DPR(実測) | DPI | 採用段(実測) | main の縁 (px) | menu の縁 (px) |
|---|---|---|---|---|
| 1 | 96 | 100 | 2.243 | 2.194 |
| 1.25 | 120 | 125 | 2.410 | 2.194 |
| 1.5 | 144 | 150 | 2.536 | 2.194 |
| 1.55(乱数) | 149 | 150 | 2.532 | 2.194 |
| 1.7(乱数) | 163 | 175 | 2.635 | 2.194 |
| 1.75 | 168 | 175 | 2.635 | 2.194 |
| 1.88(乱数) | 180 | 200 | 3.199 | 3.038 |
| 2 | 192 | 200 | 3.199 | 3.038 |
| 2.5 | 240 | 起動せず(3 の節) | 未確認 | 未確認 |
| 3 | 288 | 300 | 3.048 | 0.800(修正後。4 の節) |

- 静的の 10 段の結果: 起動できた 9 段で、窓 8 種のうち、plugin と tooltip 以外は開けた。plugin はこのプロジェクトに内蔵プラグインを挿入する手順が無く、tooltip は上部バーに乗せても新しい最上位の窓が出なかった(#{{137}} で扱う)。主窓の `pointerErrPx` は 9 段で 0。Debug ログは 72 行が captured、`scalingLines` の error・warn は 0 行。
- 動的(1 → 1.21 → 2 → 1。実測): 全段 ok。`WM_DISPLAYCHANGE` は段 2・3・4 で 1 件ずつ(段 1 は 0)、`WM_DPICHANGED` は 0 件。DPI(96)と採用段(100)は起動時の値のまま。再起動の案内は段 2・3 だけに出た。canvas と表示は一致し、ページの例外は 0。
- 乱数: `randomDprs` の種 376198107、値 1.55・1.88・1.7。動的の中間 1.21。Tempo 枠の検出が外れたのは、乱数 3 点と動的の段 2・3(判定の外。#{{137}})。
- ページの画面の黒(`tools/browser-panel/measure.mjs`、DPR 1): 0。
- 3. DPR 2.5 の起動の停止(原因は #{{149}}。推定)
  - HEAD の engine: DPR 2.5 は 5 回中 4 回が、180 秒経っても engine-idle に届かなかった(engine のログ 0 行、メモリの値 null、ページの例外 0、console 0)。DPR 3 は 9 回中 1 回が止まった。
  - app の変更を HEAD に戻した版でも、DPR 2.5 は 3 回中 3 回止まった(同じ engine で計 8 回中 7 回)。app は原因ではない。
  - 実験(製品には入れていない。取り消し済み): `fle_loop_slice` を何もしない版では、DPR 2.5 は 3 回中 3 回 engine-idle に届いた(採用段 250、edgeWidth 3.543)。
  - 判定の段の 1 回では、DPR 3 が 180 秒で止まり、乱数の DPR 2.4 は CDP の `Runtime.evaluate` が 30 秒で timeout した(ページの JS が返らない。止まり方が違う)。乱数の DPR 2.14 は通った。
  - 推定の原因: #{{112}} 段 A の後ろ向きの goto の切り替え(`fle_loop_tick`・`fle_loop_slice`)。止まる仕組みは未確認。切り替えを外す修正は、#{{112}} の効果を失うので採らない(#{{149}} で直す)。
- 4. menu の縁(DPR 3): 外側は黒(0)、4 px の枠(28)、内側は 192。縁のコントラストが 28 で、閾値 40 を下回ったため null だった(描画の遅れではない)。直し方: menu の縁だけ、コントラスト 20 以上で向きを問わず測る(`MENU_MIN_CONTRAST`)。main と他の窓の閾値は 40 のまま。結果は DPR 3 で 0.80 px。
- 5. 採用段の式(前回の判定が落ちた理由の 1)。実測の点は、上の表の 9 点と、判定の段の DPR 2.14(DPI 205。採用段 200)。
  - 床の式(DPI ÷ 96 × 4 の切り捨て × 25): DPR 1.7 と 1.88 で外れる(実測 175・200 に対して 150・175)。
  - 25% 刻みの最近接: 判定の段の DPR 2.14 で外れる(225 になる。実測は 200)。
  - 225 の段を外した最近接(次の式。下の「次の試行の決まり」の ①): 上の 9 点と 2.14 の全てに合う。225 の段が無いことは、実測の 1 点(DPI 205 → 200)からの推定。
- 6. 判定の結果: 作業者の判定は、DPR 2.5 の起動の停止(3 の節)と、乱数 1.7・1.88 の採用段の式(5 の節)で落ちた。判定の段(別の実行。乱数 2.14・2.4・1.56)は、DPR 2.5・3 と乱数 2.4 の起動、および main の 3 点のポインタで落ちた。受け入れは、どの試行でも通っていない。

## 2 回前の試行で分かったこと(run 45327829718)

1 回前の試行: run 45327829718(3 時間 37 分)。判定は終了コード 1 で、push は無し。作業は保存され、その差分を `.prior-work.md` と共に作業ツリーで受け取った(取り込めたかは、`.prior-work.md` と `git status` で確かめる)。

- 製品の変更(前回の作業の中身。作業ツリーに在る):
  - `app/src/main.ts`: DPR の変化を、部屋の測り直しと画面の追随へ渡す。`matchMedia` の参照を保持し、`resize` と 250 ms の確認を足した。
  - `app/src/ui/screen-fit.ts`(`restartNeeded`: DPR の百分率が起動時と違うか)、`app/index.html`・`app/src/style.css`(`#restart-note`。`data-restart-notice`)。
  - `lift/runtime/fle_input.c`: 画面の大きさの変化を、engine のログに `input: WM_DISPLAYCHANGE` の 1 行で出す。
  - `tools/feat-c1-hidpi/measure-size.mjs`: `--windows`・`--dynamic`・`randomDprs`・`restartNotice`・`debugLog`・`pointerErrPx` を足した。
  - 転送後の canvas の幅を設定すると、例外(`Cannot resize canvas after call to transferControlToOffscreen()`)で `applyDisplay` が止まる。canvas の大きさは既存の経路で正しく追随するので、その設定は入れない。
- 動的(4 段: DPR 1 → 乱数の中間 → 2 → 1): ゲストの画面は round(部屋 × DPR) ±1 で追随し、canvas と表示が一致した。ページの例外 0、DPI の問い合わせ 0。FL の DPI(96)と拡大率(100)は起動時の値のまま。再起動の案内は、DPR の百分率が起動時と違う段(段 2・3)だけに出た。`WM_DISPLAYCHANGE` は段ごとに 1 行。WM_DPICHANGED は送らなかった(FL の DPI は起動時に決まるため。FL の拡大率は再起動まで変わらない)。
- 静的(10 段 × 窓 8 種 = 80 行): 60 行で窓を開けて測れた。開けなかったのは plugin・tooltip の 20 行(#{{137}} で扱う)。主窓の 3 部品のポインタの誤差は、全 10 段で 0 画素。
- 採用段(エンジンのログの値): 固定の 7 点と乱数の 3 点の 10 点で、DPI ÷ 96 × 4 を切り捨て、25 を掛けた値(25% 刻みで下に丸める)と一致した。
- 主窓の縁(`edgeWidth`): 2.2〜3.5 px(DPR 1 と乱数の 1.04 で 2.24 px)。縁の幅は採用段に比例して広がらず、物理画素で描かれている。
- menu の縁: 2.5・2.69・3 以外の段で測れた(2.19〜3.04 px)。2.5・2.69・3 では測れなかった(null。原因は未確認)。menu は必須の窓なので、この 3 点で測れるように直す。
- blackShare: guest の canvas の Browser 矩形は全段で 1.0(#{{115}} の問題)。ページの画面では 0(#{{118}} の面)。判定はページの画面で測る(`tools/browser-panel/measure.mjs`)。
- 部品の座標: 固定の DPR では、CSS px で 1 px 以内に揃った。乱数の DPR と、動的の段 2・3 では Tempo 枠の検出が外れた(理由は未確認。#{{137}} で調べる)。判定からは外す。
- 前回の判定が落ちた理由(判定の式の欠陥。製品の失敗ではない):
  1. 縁の「2 以下」は、DPR 1 の 2.24 px と両立しない。
  2. 採用段の「round(100 × DPR) ±3」は、乱数の DPR で丸めの規則と外れた。
  3. 部品の座標を guest 画素の生の値で比べ、DPR ≠ 1 で必ず外れた。
  4. plugin・tooltip の 20 行が開けず、必須の判定が落ちた。
  5. blackShare が guest の canvas の黒を見ていた。
  6. 縁が null のとき「null ≤ 2」が真になり、測れなかった縁が通った。
  7. ソースの検査の grep が `refs/judge/main` を使い、作業者のセッションでは空振りした(判定の段では有る)。

## 次の試行の決まり(PM の決定。run 45501335817 の後)

1. **採用段の表を実測で作る**(式の代わり)。道具 `tools/feat-c1-hidpi/scale-grid.mjs`(新規。「やること」10)で、DPR 1.00 から 3.00 まで 0.05 刻みの 41 点(2.00 は 1 点)を測る。各点の FL の採用段(`scaleFactor`)と DPI を、`pm-work/findings/hidpi-fl-scale-table-1348.md` に表で残す。
2. **境目は DPI の整数で決める**。隣り合う 2 点で採用段が変わる所は、その間を 0.01 刻みで測る(`--dprs` に渡す)。0.01 刻みの DPR は、DPI が 1 つずつ変わる刻みより細かい。
3. **判定の表**。1 と 2 の結果を `tools/feat-c1-hidpi/fl-scale-steps.json`(新規。判定の道具。製品には入れない)にする。形式は `[{"from": 96, "scale": 100}, ...]`(from は DPI の下限。昇順)。DPI d の採用段は、from ≤ d の最後の要素の scale。表は 41 点と精密化の全点を再現すること。判定の式(旧の 25 刻みの式)は、この表に置き換える。
4. **判定の範囲**。accept は、表の測定(41 行。`grid.jsonl`)・静的の 10 段と乱数の 3 点・動的の 4 段の採用段を、表で照合する。乱数の範囲は [1.01, 3.00] のまま(既に全域)。乱数の DPR は受け入れのたびに選ぶ(固定の値に合わせた実装は通らない)。
5. **2 回通す**。作業者の受け入れが通り、判定の段が再実行して、もう一度通ること。2 回とも通ったことを報告に書く。
6. **Tempo 枠の検出(#1381)は直さない**。判定の外のまま。報告には、乱数と動的の段 2・3 で外れた値を書く。
7. **直すのは判定の表とその道具だけ**。製品の拡大率の扱い(FL が決める)は変えない。その他の受け入れ(静的の 80 行・ポインタ・Debug ログ・ブラウザの黒・単体テスト・ソースの検査)は、そのまま。

## 前回までの決まり(run 45357788913 の後。run 45501335817 で実行済み)

1. **採用段の式を、最近接の段の式に直す**(判定の (a) の式。計算であり、定数・表ではない)。p = DPI ÷ 96 × 100 を求め、25 刻みの最近接に丸める(`Math.round(p / 25) * 25`)。丸めた値が 225 の時は、p < 225 なら 200、それ以外は 250 とする。判定は、FL のログの factor × 100 がこの値と一致すること。1 段ずれた行は落ちる。
   - 根拠(実測): 上の表の 9 点、判定の段の DPR 2.14 と 1.56、実験の DPR 2.5(採用段 250)を、この式が全て満たす。床の式は 2 点、25 刻みの最近接は 1 点(2.14)で外れる。
   - 225 の段を外すのは、実測の 1 点(DPI 205 → 200)からの推定。DPI 211〜230(DPR 2.2〜2.4)は未測定。#{{149}} の後に DPR 2.2〜2.4 を測り、式が合うかを確かめる。合わなければ、式を直して報告に書く。
2. **menu の縁**: `MENU_MIN_CONTRAST`(コントラスト 20 以上・向きを問わず)を残す。main と他の窓の閾値は 40 のまま。
3. **DPR 2.5 と 2.69**: #{{149}} が直った後に測る。それまでは、この Issue の受け入れを走らせない(DPR 2.5 の起動が止まり、判定が落ちるだけになる)。
4. **乱数の DPR**: 受け入れのたびに選ぶ(固定の値に合わせた実装は通らない)。
5. **前回の作業**: 差分を作業ツリーで受け取り、上の 1 と 2 を当てた上で、受け入れを通す。

## 読む物

- #{{107}} の成果物(`tools/feat-c1-hidpi/measure-size.mjs`、`lift/runtime` の DPI の経路、`app/src/ui/screen-fit.ts`・`desktop.ts`)
- 前回の作業の差分(作業ツリーの `.prior-work.md` と `git diff HEAD`)
- `lift/runtime/fle_win.c`(窓の大きさの変更と、`WM_DISPLAYCHANGE`・`WM_DPICHANGED` の送り方)
- `app/src/main.ts`(`applyScreenParam`・`applyDisplay`・DPR の監視)
- `app/src/engine/render-worker.ts`(窓ごとのテクスチャ)、`app/src/input/pointer.ts`(ポインタの変換)
- `pm-work/findings/hidpi-fl-scaling.md`(#{{107}} の測定の結果。FL の拡大の決め方と、Debug ログの取り方)
- `tools/browser-panel/measure.mjs`(ページの画面の blackShare の測り方)
- 前回の差分の `pm-work/findings/hidpi-dynamic-1348.md`(前回の試行の記録。`.prior-work.md` と共に受け取ってから読む)

## やること

1. 道具 `tools/feat-c1-hidpi/measure-size.mjs` を、前回の差分を土台に直す(受け入れの引数と同じ形を出す)。
   - `--dprs 1,1.25,1.5,1.75,2,2.5,3`(固定の 7 点。DPR の入力としてだけ使い、期待値には使わない)と、`--random 3 --random-range 1,3`(乱数の 3 点。0.01 刻み。起動のたびに種を選び、種と値を `randomDprs` に出す)。静的の行は 10 段 × 窓 8 種 = 80 行。
   - `--windows main,welcome,mixer,playlist,piano-roll,plugin,tooltip,menu`(窓の種類ごと。開けない窓は、その行の `error` に理由を書く)。
   - `--dynamic 1,<中間>,2,1`(1 回の実行の中で DPR をこの順に変える。`Emulation.setDeviceMetricsOverride`)。中間の値は受け入れのたびに乱数で選ぶ(下の受け入れ)。動的の行は段ごとに 1 行。
   - 各行に `window`・`mode`・`randomDprs`・`debugLog`(FL の Debug ログ。`scalingFactor` を含む)・`flScalingPercent`・`edgeWidth`・`pointerErrPx`・`restartNotice`(動的の行)を出す。
2. 静的の行: 下の判定の基準を全ての行で満たす。
3. 動的な変化: 段ごとに、guest が round(部屋 × 段の DPR) に追随し、canvas と表示が一致し、FL の DPI と採用段が起動時の値のまま変わらず、再起動の案内が DPR の百分率が違う段だけに出ること。`WM_DPICHANGED` を送るかどうかは、FL の実測で決め、理由を報告に書く(前回は送らなかった)。
4. 直す(動的の変化で崩れる所だけ。`lift/runtime` と `app/src`)。
   - 拡大率(FL が見る DPI と、FL の UI の拡大率)は、再起動まで変えない。
   - 利用者への案内: 既存の `#restart-note` で、拡大率の変更は FL を再起動すると適用される旨を出す。自動で再起動を提案しない(FL の再起動は利用者の操作)。
   - 転送後の canvas の幅を、設定しない(例外になる)。
   - 新しい環境変数・URL パラメータ・ビルド変数を作らない。拡大率の数値を runtime・app・seed に書かない。
5. menu の縁: DPR 2.5・2.69・3 で測れなかった原因を調べ、測れるようにする(測り方か、窓の開き方を直す。製品の既定の動作は、測定のためだけに変えない)。前回の試行で、DPR 3 の原因(縁のコントラストの閾値)は直した(`MENU_MIN_CONTRAST`。残す)。DPR 2.5 と 2.69 は、起動が直った後(#{{149}})に測る。
6. 部品の座標と Tempo 枠の検出: 判定から外す。報告には、乱数の DPR と動的の段 2・3 で外れた値を書く。理由の調査は #{{137}} で行う。
7. ポインタ: 静的の主窓で、3 部品の中心を押し、engine が受け取った画素と押した画素の差(`pointerErrPx`)が 1 以下であること。
8. plugin・tooltip の窓: この Issue では開けなくてよい(行の `error` に理由を書く)。#{{137}} で扱う。
9. Debug ログ: 全ての行で、`debugLog.scalingLines` に error・warn の行が無いこと。出ていれば、その行を報告に書き、原因を直す。
10. 表の測定の道具 `tools/feat-c1-hidpi/scale-grid.mjs`(新規)を作る。`tools/feat-c1-hidpi/measure-size.mjs` の起動の部分を使い、DPR ごとに 1 回ずつ起動して、FL の `Main GUI scaling factor` のログが出るまで待つ(静的の 8 窓の測定はしない。1 点約 15 秒)。出力は 1 点 1 行の JSON だけ: `dpr`・`dpi`(FL に渡った DPI。round(96 × DPR) と比べる)・`scaleFactor`(ログの factor の値)・`state`・`standIn`・`error`。既定の点は DPR 1.00 から 3.00 まで 0.05 刻みの 41 点(2.00 は 1 点)。`--dprs <list>` で任意の点も測れる。
11. 境目を決める。表の隣り合う 2 点で採用段が変わる所は、その間を 0.01 刻みで `--dprs` に渡して測る。全ての点の結果(DPR・DPI・採用段)を `pm-work/findings/hidpi-fl-scale-table-1348.md` に表で書く。
12. 判定の表 `tools/feat-c1-hidpi/fl-scale-steps.json`(新規)を、11 の結果から作る(形式は「次の試行の決まり」の 3)。判定の式(旧の `adp`)は、この表で置き換えた。製品には入れない。

## 受け入れ

```accept
npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false?0:1)' "$RUNNER_TEMP/boot.json" && D=$(node -e 'process.stdout.write((require("crypto").randomInt(101,301)/100).toFixed(2))') && node tools/feat-c1-hidpi/measure-size.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" --dprs 1,1.25,1.5,1.75,2,2.5,3 --windows main,welcome,mixer,playlist,piano-roll,plugin,tooltip,menu --random 3 --random-range 1,3 --dynamic 1,$D,2,1 > "$RUNNER_TEMP/fs.jsonl" && node tools/feat-c1-hidpi/scale-grid.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/grid.jsonl" && node tools/browser-panel/measure.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/browser.json" && node -e 'const fs = require("fs"); const L = fs.readFileSync(process.argv[1], "utf8").trim().split("\n").map((l) => JSON.parse(l)); const B = JSON.parse(fs.readFileSync(process.argv[2], "utf8")); const G = fs.readFileSync(process.argv[3], "utf8").trim().split("\n").map((l) => JSON.parse(l)); const W = ["main", "welcome", "mixer", "playlist", "piano-roll", "plugin", "tooltip", "menu"]; const REQ = ["main", "menu"]; const eq = (x, y) => Math.abs(x - y) < 0.0001; const S = L.filter((x) => x.mode === "static"); const Dy = L.filter((x) => x.mode === "dynamic"); const R = S.length ? S[0].randomDprs.values : []; const F = [1, 1.25, 1.5, 1.75, 2, 2.5, 3].concat(R); const a = S.find((x) => eq(x.dpr, 1) && x.window === "main"); if (!a || a.edgeWidth === null || !a.room) { console.log("judge: DPR 1 の主窓の行が無い、または縁が測れない"); process.exit(1); } const steps = JSON.parse(fs.readFileSync("tools/feat-c1-hidpi/fl-scale-steps.json", "utf8")); const adp = (x) => { let s = null; for (const t of steps) if (x.dpi >= t.from) s = t.scale; return s; }; const boot = (x) => x.state === "engine-idle" && x.standIn === false && x.room !== null && x.guest !== null && x.canvas !== null && x.display !== null && x.dpi !== null; const reqOk = (x) => !REQ.includes(x.window) || x.error === null; const geo = (x) => Math.abs(x.guest.w - Math.round(x.room.w * x.dpr)) <= 1 && Math.abs(x.guest.h - Math.round(x.room.h * x.dpr)) <= 1 && Math.abs(x.canvas.w - x.display.w) <= 1 && Math.abs(x.canvas.h - x.display.h) <= 1; const dbg = (x) => x.debugLog.captured === true && x.debugLog.lines >= 1 && !x.debugLog.scalingLines.some((l) => /error|warn/i.test(l)); const baseOf = (w) => { const b = S.find((x) => eq(x.dpr, 1) && x.window === w); return b ? b.edgeWidth : null; }; const edge = (x) => { if (x.edgeWidth === null) return !REQ.includes(x.window); const b = baseOf(x.window); return b === null || x.edgeWidth <= b * (1 + (x.dpr - 1) / 2) + 0.5; }; const gridRow = (x) => x.state === "engine-idle" && x.standIn === false && x.error === null && Math.abs(x.dpi - Math.round(96 * x.dpr)) <= 1 && x.scaleFactor !== null && Math.round(x.scaleFactor * 100) === adp(x); const sRow = (x) => boot(x) && reqOk(x) && geo(x) && x.mainGuiScaling === "System" && dbg(x) && Math.abs(x.dpi - Math.round(96 * x.dpr)) <= 1 && x.debugLog.scalingFactor !== null && Math.round(x.debugLog.scalingFactor.factor * 100) === adp(x) && edge(x); const dRow = (x) => boot(x) && reqOk(x) && geo(x) && dbg(x) && x.dpi === a.dpi && x.flScalingPercent === adp(a) && edge(x) && x.restartNotice === (Math.round(x.dpr * 100) !== Math.round(a.dpr * 100)); const ptrOk = (x) => x.pointerErrPx !== null && x.pointerErrPx <= 1; const bad = (rows, f) => rows.filter((x) => !f(x)).map((x) => `${x.window}@${x.dpr}${x.mode === "dynamic" ? " step " + x.step : ""}`); const last = Dy.length === 4 ? Dy[3] : { edgeWidth: null }; const checks = { rows: S.length === 80 && Dy.length === 4, grid: G.length === 41 && G.every(gridRow), dprs: F.every((d) => W.every((w) => S.some((x) => eq(x.dpr, d) && x.window === w))), rnd: R.length === 3 && R.every((d) => d >= 1 && d <= 3), room: Math.abs(a.room.w - 1836) <= 2 && Math.abs(a.room.h - 1056) <= 2, static: S.every(sRow), pointer: S.filter((x) => x.window === "main").every(ptrOk), steps: Dy.length === 4 && Dy.every((x, i) => x.step === i + 1) && eq(Dy[0].dpr, 1) && Dy[1].dpr > 1 && Dy[1].dpr <= 3 && eq(Dy[2].dpr, 2) && eq(Dy[3].dpr, 1), dynamic: Dy.every(dRow), last: last.edgeWidth !== null && Math.abs(last.edgeWidth - a.edgeWidth) <= 0.5, browser: B.error === null && B.state === "engine-idle" && B.standIn === false && B.blackShare !== null && B.blackShare < 0.5, }; for (const k of Object.keys(checks)) if (!checks[k]) console.log(`judge: 失敗 ${k}`); const badG = G.filter((x) => !gridRow(x)).map((x) => `grid@${x.dpr}`); if (badG.length) console.log(`judge: 表の測定で外れた: ${badG.join(", ")}`); const badS = bad(S, sRow); if (badS.length) console.log(`judge: 静的の行で外れた: ${badS.join(", ")}`); const badD = bad(Dy, dRow); if (badD.length) console.log(`judge: 動的の行で外れた: ${badD.join(", ")}`); const badP = S.filter((x) => x.window === "main" && !ptrOk(x)).map((x) => `main@${x.dpr}`); if (badP.length) console.log(`judge: ポインタが外れた: ${badP.join(", ")}`); console.log(`judge: ${Object.entries(checks).map(([k, v]) => `${k}=${v}`).join(" ")}`); const ok = Object.values(checks).every(Boolean); console.log(ok ? "judge: 通った" : "judge: 通らない"); process.exit(ok ? 0 : 1);' "$RUNNER_TEMP/fs.jsonl" "$RUNNER_TEMP/browser.json" "$RUNNER_TEMP/grid.jsonl" && b=$(git merge-base HEAD origin/main) && d=$(git diff -U0 "$b" -- lift/runtime lift/harness app/src) && ! grep -E '^\+[^+]' <<<"$d" | grep -qE '\b(125|150|168|169|175|176)\b|1\.75\b|1\.7647|0\.5667' && npm test --prefix app
```

判定の基準(道具の出力の生の値を、上の node -e が読む。4 本の出力: `fs.jsonl`(測定)・`grid.jsonl`(表の測定)・`browser.json`(ページの画面の黒)・ソースの検査):

- (a) 静的の行は 80 行。固定の 7 点と `randomDprs.values` の 3 点の全てに、窓 8 種の行が在る。全ての行で、state が engine-idle、standIn が false、guest が round(部屋 × DPR) の ±1 画素、canvas と display が ±1 画素、dpi が round(96 × DPR) の ±1、Main GUI Scaling が "System"、debugLog が captured(lines が 1 以上、error・warn の行が無い)。FL の採用段(`debugLog.scalingFactor` の factor × 100)が、表(`tools/feat-c1-hidpi/fl-scale-steps.json`。DPI から採用段を引く。「次の試行の決まり」の 3)と一致する。
- (a2) 表の測定(`tools/feat-c1-hidpi/scale-grid.mjs`。DPR 1.00 から 3.00 まで 0.05 刻みの 41 行)の全行で、state が engine-idle、standIn が false、error が null、dpi が round(96 × DPR) の ±1、`scaleFactor` × 100 が表の採用段と一致する。
- (b) 窓: main と menu は、error が null で、縁(`edgeWidth`)が測れている。他の窓(welcome・mixer・playlist・piano-roll・plugin・tooltip)は、error と null を許す。縁が測れた行は、同じ窓の DPR 1 の縁(`base`。DPR 1 が測れていなければ判定しない)に対して、`edgeWidth` ≤ base × (1 + (DPR − 1) ÷ 2) + 0.5 画素を満たす。拡大による縁のぼやけは、DPR に比例して縁が広がるので、この上限を超える。
- (c) 主窓の静的の行: `pointerErrPx` が 1 以下。
- (d) 動的の行は 4 行(段 1〜4)。段 1 と 4 は DPR 1、段 2 は乱数の中間(1.01〜3.00)、段 3 は DPR 2。各段で、guest・canvas と display・debugLog の基準を満たし、DPI が起動時(DPR 1)の値のまま、FL の採用段が起動時の値のまま、縁が (b) の基準を満たす。`restartNotice` が、DPR の百分率が DPR 1 と違う段だけ true。段 4 の縁は、DPR 1 の主窓の縁と ±0.5 画素以内。
- (e) ページの画面の黒: `tools/browser-panel/measure.mjs` の `blackShare` が 0.5 未満(error が null、state が engine-idle)。guest の canvas だけの黒は判定から外す(報告に書く)。
- (f) 乱数の DPR と、動的の中間の DPR は、作業者の実行と判定の実行で別になる。固定の値に合わせた実装は通らない。
- (g) ソースの検査(受け入れの最後の段): `git merge-base HEAD origin/main` からの差分の、`lift/runtime`・`lift/harness`・`app/src` の追加行に、特定の拡大率・DPI・DPR の値が、拡大率・DPI・DPR の意味で現れない(一致 0 件)。
- (h) 単体テスト(`npm test --prefix app`)が通る。
- 判定の式は、部品の座標・Tempo 枠の検出・plugin と tooltip の拡大・guest の canvas の黒を見ない(報告にだけ書く)。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には「runner の値。この機械とは別枠」と書き、nproc と loadavg を添える。
- 実測と推定を分ける。「達成」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。出力の行を付ける。
- 表の測定(`scale-grid.mjs`)の 41 行(DPR・DPI・採用段)、精密化の境目の値、`tools/feat-c1-hidpi/fl-scale-steps.json` の全文、2 回の受け入れ(作業者の実行と判定の段の再実行)の結果を報告に書く。
- 報告に書く: 拡大率と窓の種類ごとの表(`edgeWidth`・`blackShare`(ページの画面)・採用段)、`randomDprs` の種と値、動的の変化の結果(段ごとの guest・DPI・採用段と、engine のログの `WM_DISPLAYCHANGE`・`WM_DPICHANGED` の行)、`WM_DPICHANGED` を送ったかどうかと理由、再起動の案内の方式と理由、ポインタの結果、Debug ログの error・warn の有無、開けなかった窓と理由、menu の縁が測れなかった原因、部品の座標と Tempo 枠の検出の値(判定の外)、確認できなかった Windows の実際の挙動、ソースの検査の grep の出力。

## 制約

- 製品に新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足さない。
- 特定の拡大率・DPI・DPR の値を、runtime・app・seed・設定・テストに、定数・表・分岐・seed の値として入れない。拡大率は、実行時の DPR から式で導く。
- 論理の小さい画面を DPR 倍に拡大して表示する実装を、作らない。
- 製品の既定の動作を、測定のためだけに変えない。
- ベンダー名と Cloudflare の識別子を書かない(禁止されているのは、この 2 種類だけ。FL の製品名・`Image-Line` のインストール先・変数名は書いてよい)。
- 採用段の表(`tools/feat-c1-hidpi/fl-scale-steps.json`)は判定の道具に置く。製品(`app/`・`lift/`)には置かない。
- sleep・timeout・nohup・末尾の `&` は使わない。一時ファイルは `.tmp-*` に置き、終わったら消す。

## 作業の進め方

- ビルド(engine の作り直し)は約 35 分。表の測定(`scale-grid.mjs`。1 点約 15 秒で 41 点)は約 10 分。受け入れ 1 回は約 70 分(ビルドを含む)。判定の段で、もう一度走る。
- ビルドは約 35 分。フォアグラウンドで直接、シェルツールの `timeout` 引数は最大 5400000。
- 長いコマンド(ビルド・受け入れ)は、フォアグラウンドで直接実行し、シェルツールの `timeout` 引数に 5400000 を渡す。背景で起動して待たない。
- 開始から 2 時間 30 分を過ぎたら、受け入れを新たに始めない。開始から 4 時間 30 分を過ぎたら、受け入れが通っていなくても、作業ツリーを commit して、報告を書いて終える(判定の段が約 70 分走るので、受け入れは 2 時間 30 分までに始める)。
- 作業の最後に、受け入れが通らなくても、作業ツリーを commit する。

## 他のタスクとの関係

- blocked_by: #{{107}}(起動時の DPR に対する Windows と同じ Fractional Scaling)、#{{113}}(FL の UI の拡大の実測)、#{{114}}(DPI 240 以上の停止。効果は #{{112}} 段 A で無効になった)、#{{149}}(DPR 2.5 以上の起動が first idle に届かない障害。修正は main に入っている(DPR 2.5 の行は測れる))。
- #{{137}}(plugin と tooltip の窓。Tempo 枠の検出の理由の調査)は、この Issue の判定の外。
- #{{109}}(音が途切れる現象)は、この Issue と並行して進めてよい。ただし、物理画素の画面で測る。
- 同じ描画の経路を変える #{{040}} と #{{106}} と衝突しうる。衝突したら、作業の最後に `git pull --no-rebase --no-edit` で main を取り込み、衝突を直し、受け入れをもう一度通す。
