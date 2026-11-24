# hidpi-fl-scaling: Issue #1347 の測定と結果(受理ではない。未確認と差し戻し)

測定値は runner の値。この機械とは別枠。nproc = 4。loadavg は測定の開始時 1.74(0.62 / 0.22)。実行の途中では取っていない。

## 判定

- 受け入れ(Issue #1347 の `accept`)は **通らない**。理由:
  - DPR 2.5 以上(DPI 240 以上)で engine-idle に届かない。guest の再帰で停止する(節 2〜4)。乱数の DPR は 1.00〜3.00 なので、この停止が出れば受け入れは通らない。
  - `flScalingPercent`・`parts`・`edgeWidth`・`debugLog` は測っていない(null / `captured: false`。節 6)。
- 実測で確かめたこと: 起動の時点から、窓の API と DPI の応答の記録が取れる(節 1・2)。FL が決める DPI の値は、DPR から式(`MulDiv(96, round(DPR × 100), 100)`)どおり(DPR 1 = 96、DPR 2.25 = 216、DPR 2.5 = 240)。
- FL64.exe の manifest は `dpiAware True/PM`(節 0)。DPI の認識の値 2 は、この宣言から決まる。

## 0. 起動用の実行ファイルのマニフェスト(実測)

`lift/out-app/host-extra/exe/` の 2 つの PE から、RT_MANIFEST(資源 24/1/1033)を `tools/feat-c1-hidpi/read-manifest.mjs`(node だけで読む道具。製品には入れない。`node tools/feat-c1-hidpi/read-manifest.mjs <file.exe>`)で取り出した。

| 項目 | `FL64.exe` | `FL64 (scaled).exe` |
|---|---|---|
| 大きさ・sha256 | 597064 B・`4c839f24…520c65` | 597064 B・`0ab91f0f…a2e979` |
| `assemblyIdentity` | ImageLine.FLStudio.FLExe 26.1.7.0 amd64 | 同じ |
| `dpiAware` | `True/PM` | (宣言なし) |
| `dpiAwareness` | (宣言なし) | (宣言なし) |
| `gdiScaling` | (宣言なし) | (宣言なし) |
| `longPathAware` | true | true |
| `requestedExecutionLevel` | (宣言なし) | asInvoker |

読み: `dpiAware=True/PM` は per-monitor DPI 対応(PROCESS_PER_MONITOR_DPI_AWARE = 2)の宣言。`FL64.exe` は DPI を自分で扱う。`FL64 (scaled).exe` は DPI 非対応の宣言が無く、Windows が画面を拡大する(ぼやける)ための起動用と読める(これは推定。実機では未確認)。

製品の応答: `GetProcessDpiAwareness` は 2 を返す(trace の行 `window-api: GetProcessDpiAwareness -> 2`)。この値は上の宣言から決める。

## 1. 記録の仕組み(実装)

- 窓の API の記録(`fle_win_api_trace_set`、C-6 / #1310)を、起動の前から有効にできるようにした。`window.__app.windowApiTrace(true)` を、ページの起動の直前(`client.start()` の前)に送る。ワーカーは起動の前の指示を覚え、wasm の `preRun` で有効にする(`app/src/main.ts` の `apiTraceWanted`、`app/src/engine/fruity.ts` の `traceOn`)。既定は無効。
- 記録する行(`window-api:` の接頭辞。既存の道具がそのまま読める): `GetDeviceCaps index=N -> V from <呼び出し元の戻り番地>`、`GetSystemMetrics index=N -> V`、`GetDpiForMonitor type=T -> V`、`GetProcessDpiAwareness -> V`。
- 戻り番地は、guest のスタックの `[rsp]`(`fle_arg` と同じ規則)。

## 2. DPI と拡大率の応答(実測。engine のログ)

| DPR | LOGPIXELSY(90) | GetDpiForMonitor | SM_CXCURSOR(13) | SM_CXSMICON(49) | 式の値(MulDiv) |
|---|---|---|---|---|---|
| 1 | 96 | 96 | 32 | 16 | DPI 96 |
| 2.25 | 216 | 216 | 72 | 36 | DPI 216 |
| 2.5 | 240 | 240 | 80 | 40 | DPI 240 |

guest は LOGPIXELSY(90)だけを読む(LOGPIXELSX(88)は呼ばれない)。SM の値は `MulDiv(32, DPI, 96)` と一致する(72 = 32 × 216 / 96)。

`flScalingPercent`(FL が採用した拡大率)は測っていない。FL の UI の部品の大きさを、この機械の出力から取れていない(節 6)。

## 3. 停止の切り分け(実測)

DPR 2.5 の起動。切り替えは `.tmp-exp` の一時的な実験(製品には残していない)。各 start-up の全文は `.tmp-exp/log-*.txt`(commit しない)。

| 実験 | 切り替え | 結果 | engine-idle |
|---|---|---|---|
| 基準(製品の差分) | なし | 60 秒で届かない。`[js] fle thread 2 failed: RangeError: Maximum call stack size exceeded` | 届かない |
| (i) フォントの高さだけ DPI 96 | `FLE_EXP_FONT96=1` | 届かない | 届かない |
| (ii) GetDeviceCaps の LOGPIXELS だけ 96 | `FLE_EXP_LOGPIX96=1` | 届いた(idle 4 回・9.2 秒) | **届く** |
| (iii) GetDpiForMonitor だけ 96 | `FLE_EXP_MON96=1` | 届かない | 届かない |
| 仮説: 物理サイズを機器の密度(96 × DPR)から求める | `FLE_EXP_MMDPI=1` | DPR 2.5 も DPR 3.0 も届かない | 届かない |
| 基準 DPR 2.25(DPI 216) | なし | 届いた(idle 1 回・11.1 秒) | 届く |
| 基準 DPR 1(DPI 96) | なし | 届いた(idle 4 回・9.4 秒) | 届く |

読み(実測):
- 停止の原因は、guest が **LOGPIXELSY の値 240** を受け取ること。フォントの高さも GetDpiForMonitor も、単独では原因ではない。216 は通る。しきい値は 216 と 240 の間。
- 物理サイズの矛盾(96 DPI 固定の mm と、LOGPIXELS の 240 の食い違い)は原因ではない(仮説の実験で否定)。
- 停止は窓の作成の直後: `CreateWindowExW 0x…24 ex=0x80 style=0x80000000 parent=0`(隠れた `TPUtilWindow` 相当)の後、描画(`UpdateLayeredWindowIndirect`)より前。DPR 1 では、同じ位置の後に描画が進む。
- 窓の再配置の反復(`SetWindowPos`)は、停止までに無い。

## 4. 停止の位置の絞り込み(実測と推定)

- guest の LOGPIXELSY の読み出し元(trace の `from`): 5 か所。戻り番地が含まれる関数は、生成物のソースの定義から `f_<番地>` の名前で引ける(`lift/out-app/gen/src/`)。書き込みは `f_613df40`(戻り番地 `0x613df66`)で、値を guest の大域 `0x630e924` に置く。
- この大域を読むのは 7 か所。読み手の関数: `f_6135c50`・`f_6135e10`・`f_61365b0`・`f_6136750`・`f_6136d10`・`f_613e020`(font の高さを DPI から求める所。MulDiv 相当)。どれも再帰はしない(直接の呼び出しを辿ると、読み手から先は 97 関数で、閉路は無い)。
- 停止時のスタック(診断。`fle_thread_lib.js` の一時的な表示): 停止の直前の窓の作成の rsp から、下へ 32 MB を読むと、戻り番地らしい値の並びが **12 個の周期で繰り返す**(`410002 518ac7 725b9f 51b687 51b687 519acc 410e00 5199f5 5193db 5273b2 4d78a8 519707`)。`51b687` が 2 回続くので、ある関数の自己呼び出しを含む。
- ただし、この番地の多くは生成物の中に命令の位置として見つからない(`/* 0x<番地> ` の検索)。よって、関数の名前への対応は確定していない。閉路は直接の `call` ではなく、仮想呼び出し(関数ポインタ)を経由すると推定する(静的な呼び出しの辿りでは閉路が見えないため)。
- 結論(推定): LOGPIXELSY = 240 のとき、guest の font・寸法の計算の経路が、間接呼び出しを経由する閉路に入り、無限に再帰する。閉路の入口と、どの runtime の応答がそこへ導くかは、この作業では特定できていない。

## 5. 一時的な診断の扱い

(この節の実験の全文 log は commit していない。`.tmp-exp/` は作業の後に削除する。上の表の行は、その log から抜いた行。)

- 診断(`fle_g3_rsp_mark`・戻り番地の表示)と、実験の切り替え(`FLE_EXP_*`・`__fleExpEnv`)は、commit の前に外した(`grep -rn 'TEMP-EXP\|FLE_EXP_\|fle diag' lift app tools` は 0 件)。
- 窓の API の記録の行(節 1)は残した(既定では出ない。測定の道具だけが有効にする)。

## 6. 未測定(Issue の (a)〜(j))

| 項目 | 状態 | 理由 |
|---|---|---|
| (a) DPI・画面の API の記録 | 一部 | 節 1・2。RegQueryValueExW・SystemParametersInfoW の記録は未 |
| (b) seed | 確認済み | `lift/harness/seeds/reg-seed-app.tsv` と `reg-seed-nsis.tsv` は、HKCU の Shared Paths と、インストーラが書く値だけ。GUI の拡大の値は無い。数値の拡大率は seed に無いので、直す対象は無い |
| (c) F10 の Display の節 | 未 | 窓の操作を自動化していない |
| (d) FL の採用した拡大率 | 未 | UI の部品の大きさを、この道具では取れていない |
| (e) Windows の計算値との突き合わせ | 一部 | LOGPIXELSY・GetDpiForMonitor・SM の値は式どおり(節 2)。FL の採用値は未 |
| (g)〜(i) Debug ログ | 未 | ログの文字を窓の部品から読む経路を作っていない |
| (j) debugLog | captured false | 同上 |
| FL の Main GUI Scaling の読み | 未 | FL が設定を読む経路(レジストリ・設定ファイル)が trace に出ていない |
| parts・edgeWidth | 未 | トップバーの部品は窓の表(`fle_desk_describe`)に出ない(最上位の窓だけ)。FL が主窓の面に描く |

## 7. 最終の測定(commit する状態。`measure-size.mjs` の出力そのまま)

測定の道具の出力 7 行(`randomDprs`: seed 3567857356、values [1.77, 2.71])。

| DPR | 状態 | guest(w×h) | round(部屋×DPR) | dpi(LOGPIXELSY) | round(96×DPR) | GetSystemMetrics 13(カーソル)/ 49(小アイコン) | canvas / display | blackShare |
|---|---|---|---|---|---|---|---|---|
| 1 | engine-idle | 1836×1056 | 1836×1056 | 96 | 96 | 32 / 16 | 1836×1056 / 1836×1056 | 0.910 |
| 1.25 | engine-idle | 2295×1320 | 2295×1320 | 120 | 120 | 40 / 20 | 2295×1320 / 2295×1320 | 0.912 |
| 1.5 | engine-idle | 2754×1584 | 2754×1584 | 144 | 144 | 48 / 24 | 2754×1584 / 2754×1584 | 0.912 |
| 1.7647 | engine-idle | 3240×1864 | 3240×1863 | 169 | 169 | 56 / 28 | 3240×1864 / 3240×1864 | 0.905 |
| 2 | engine-idle | 3672×2112 | 3672×2112 | 192 | 192 | 64 / 32 | 3672×2112 / 3672×2112 | 0.914 |
| 1.77(乱数) | engine-idle | 3250×1869 | 3250×1869 | 170 | 170 | 57 / 28 | 3250×1869 / 3250×1869 | 0.903 |
| 2.71(乱数) | **停止**(idle に届かない) | — | — | — | 260 | — | — | — |

- 停止の行: `error`: `state running before the first engine-idle` ・ 最後のログ行 `[js] fle thread 2 failed: RangeError: Maximum call stack size exceeded`。窓の作成 `CreateWindowExW 0x…24` の直後(節 3)。DPI 260 は、既に確かめた DPI 240 の停止と同じ経路。
- `flScalingPercent`・`mainGuiScaling`・`parts`・`edgeWidth`・`debugLog` は null / false(節 6)。
- `blackShare` は DPR 1 で 0.910(閾値 0.5 を超える)。Browser の矩形(CSS の (0, 580) から 400×470)が、この版の DPR 1 でも大部分が黒い、または矩形が Browser の一覧の外にある。どちらかは未分析。

受け入れの判定(Issue の `accept` の node 部分を、この出力に当てた結果): `false`(上の null と停止と blackShare による)。源コードの検査(追加行に特定の拡大率・DPI・DPR の値が無いこと): 一致 0 件。
単体テスト(`npm test --prefix app`): 68 件のファイル、1333 件が通る。lift の `gdc`・`screen1`・`wall_s`: 合計 38 件が通る。

## 8. 次にやること

1. 閉路の入口を特定する: 停止の時点の guest のスタックを、関数の名前に結ぶ(生成物の命令の位置を、wasm の関数番号と突き合わせる、または guest の関数の名前の表を作る)。
2. 閉路を起こす runtime の応答を、Windows の値と突き合わせる(LOGPIXELSY 以外の、DPI に依る応答の全部)。
3. 修正の後、DPR 1〜3.0 の乱数で engine-idle に届くことを確かめる。
4. 部品の位置・縁の遷移・拡大率・Debug ログを、測定の道具に入れる。

## 9. Issue #1353 の測定(2026-11-23。実エンジン: #1357 の後の main で bundle `fl-Oz` を作り、`measure-size.mjs` を実行。固定 5 行 + 乱数 2 行 + DPR 2.5・3 の停止)

測定の値は runner の値。この機械とは別枠。nproc = 4。loadavg(起動の時)= 1.62 / 1.37 / 1.90。判定: **受理**(受け入れの判定式を、この実測の出力に当てて終了コード 0)。拡大率の段が Windows の値と一致するかは、Windows 実機が無いため **未確認**。

### 9.0 前回までの trap は、この main では起きない(実測)

- 前回(節 9 の 2 回目)の `fle thread 13 failed`(`System/Internet/Default news.rss` の読み)は、この main の 7 行では 0 件(`engineFaults.threadFailed` が全行 0)。engine のログは 50000 行まで保持するので、起動の全体が対象。
- 消えた理由は、#1357 の修正(seed と DLL の pack を永続にする。`91b4c973`)と推定。同じ測定を修正の前の main で行ってはいない(未確認)。`Default news.rss` の VFS は直していない(直す必要が無かった)。
- ポインタの押下は、7 行すべてで engine が受けた(9.4)。

### 9.1 FL の拡大率は 25 % の段に丸まる(実測)

FL のウェルカム窓(`TWelcomeWizard`)の幅 ÷ DPR 1 の幅(874)× 100 を、FL の採った拡大率とした。

| DPR | LOGPIXELSY(DPI) | FL の拡大率 | round(100 × DPR) | 差 | 区分 |
|---|---|---|---|---|---|
| 1 | 96 | 100 | 100 | 0 | 固定 |
| 1.25 | 120 | 125 | 125 | 0 | 固定 |
| 1.5 | 144 | 150 | 150 | 0 | 固定 |
| 1.7647 | 169 | 175 | 176 | −1 | 固定 |
| 2 | 192 | 200 | 200 | 0 | 固定 |
| 1.11(乱数) | 107 | 100 | 111 | −11 | 乱数(±25 の内) |
| 1.94(乱数) | 186 | 200 | 194 | +6 | 乱数(±25 の内) |
| DPR 2.5・3 | — | — | — | — | 停止(#1354。判定に入れない) |

- 100 と 125 の境界: DPI 107 → 100(今回)、108 → 100(前回)、111 → 125(1 回目)。境界は DPI 109〜111 のどこか(未確定)。
- 式 `min(200, 100 + 25 × round((DPI − 96) / 24))` は DPI 108 で 125 を返すが、FL は 100 を採った。段の式は runtime に入れていない(この機械では FL の段の決め方を確かめられない)。
- DPI は `MulDiv(96, round(DPR × 100), 100)` どおり(1.11 → 107、1.94 → 186、1.7647 → 169)。

### 9.2 DPI と API の記録(実測)

- FL が読むのは LOGPIXELSY(90)。LOGPIXELSX(88)は読まれない(全行で `logpixelsX` が null)。
- 記録の件数は 7 行とも 211 件(DPI・寸法・アイコン・カーソルの応答)。

### 9.3 Main GUI Scaling の読み(実測)

- 起動から idle までの trace(`RegQueryValueExW`)に、名前に scaling・GUI・uiscale を含む読みは無い(`mainGuiScaling` は 7 行とも null)。判定から外す(#1358 が F10 で確かめる)。
- `debugLog` は 7 行とも `captured: false`(判定から外す。#1358)。

### 9.4 縁(edgeWidth)・部品・ポインタ(実測と推定)

- edgeWidth(10〜90 % の遷移の中央値。画素): DPR 1 → 2.261、1.25 → 2.425、1.5 → 2.527、1.7647 → 2.626、2 → 3.199、1.11 → 2.261、1.94 → 3.199。
- 受け入れ (d) の比(edgeWidth ÷ (拡大率 ÷ 100)、DPR 1 の値で割る): 最大 1.000(1.2 以下)。
- parts: Tempo の枠は画像から測った(DPR 1.5 で 654,16、2 で 874,20)。Song Position と Pattern Selector は、DPR 1 の座標を Tempo の比で求めた推定(画像からは測っていない。道具の `measured: false`)。
- ポインタ: Welcome 窓の題名帯の 3 点(窓の幅の 0.3・0.5・0.7 の位置)を CDP で押した。`pointerErrPx`(engine が受けた画素と、floor((clientX − canvas 左) × DPR) の差)は **7 行とも 0**。`pointerSentErrPx` も 7 行とも 0。押下の要素は全て canvas。
- 測り方は変えていない。主窓の部品の中心を押す測り方は、任意のため行っていない(Welcome 窓が開いている間は、主窓に押下が届かないと見ている。未確認)。

### 9.5 道具の修正(今回の変更)

- 乱数の DPR が固定の値と重なると、行は 7 でも種類は 6 になり、受け入れの「7 種類」が落ちる(1 回目の判定で、乱数 1.5 が固定の 1.5 と重なって落ちた)。`randomDprs` は、固定の値と、乱数どうしで重ならないように引き直す(引けない時は止まる)。合成の seed 20000 通りで、固定の値との重なり・同じ値の 2 回の出現は 0。
- 変えていないもの: 拡大率・縁・部品・ポインタの定義、trace の既定の動作。

### 9.6 受け入れの判定(この実測の出力に当てた結果)

受け入れの node の式: **終了コード 0**。

- (a) 7 行(固定 5 と乱数 2 種類)が在る。全行 error null・engine-idle・standIn false。
- (b) DPR 2.0 以下の差: 0・0・0・−1・0(固定)、−11(1.11)、+6(1.94 は 2.0 以下。±25 の内)。
- (c) mainGuiScaling は判定から外れる(null)。
- (d) 比の最大 1.000 ≤ 1.2。
- (e) parts の 3 部品に座標がある。pointerErrPx は全行 0 ≤ 1。null の行は無いので、再測定は要らなかった。
- (f) debugLog は判定から外れる。
- 合成データ(`.tmp-c1-1353` の作業用。判定の式の確認): 前回の 7 行を入れると 0(通る)。DPR 1.5 の拡大率を 180 にすると 1(落ちる)。
- ソースの検査: `refs/judge/main` は作業者の checkout に無いので、HEAD を基準に確かめた。足した行 57、拡大率・DPI・DPR の値(125・150・168・169・175・176・1.75・1.7647・0.5667)の一致 0 件。
- 単体テスト(`npm test --prefix app`): 68 ファイル合格(1 件スキップ)、1335 件合格(10 件スキップ)、終了 0。
- 起動確認(`boot-bundle.mjs`): state engine-idle、error null、standIn false、elapsedMs 11273。

### 9.7 次にやること

1. 100 と 125 の境界(DPI 109〜111)を確かめる。段の式は直さない(runtime で段を作って合わせない)。FL が段を決める規則が FL 自身か Windows の値の違いかは、Windows の実機の値が無いと判定できない(未確認)。
2. Main GUI Scaling(F10 の設定の窓)・Debug ログ・Info・Display の節は #1358 → 節 10 を参照(Debug ログと係数はゲストのメモリから読めた。Display の選択と Info は未確認)。
3. DPR 2.5 と 3 は停止する(#1354。`fle thread 2 failed: RangeError: Maximum call stack`)。
4. 主窓の部品の中心を押す測り方は、Welcome 窓を閉じられるようになった後(#1357)に行える。任意。

## 10. Issue #1358 の測定(F10 の設定窓・Display・Debug・Info。数値の判定は受理)

測定の値は runner の値。この機械とは別枠。nproc = 4。loadavg(測定の前)= 0.37 / 0.40 / 0.26、(測定の後)= 0.36 / 0.92 / 1.48。実エンジン: `measure-size.mjs` を、この作業の差分を載せた main で作った bundle `fl-Oz` に対して実行(固定 5 点・乱数 2 点・DPR 2.5 と 3)。2 回実行した。1 回目は乱数が 1.42・1.47、2 回目は乱数が 2.02・2.01。下の表は 2 回目の出力(`size.jsonl` の 7 行 + 高い DPR の 1 行)。数値の判定は 2 回目の出力に当てた。

### 10.0 窓の文字を読む経路(実測)

- 窓の表の文字(前回の経路)では、Debug の本文は `text ""` のまま(この実測でも同じ。`settings.debugMemo`)。この経路は使わない。
- 今回は、ゲストのメモリ(wasm の線形メモリ。DPR 1 で 1,074,462,720 B)を、読むだけで走査した。経路: `window.__app.guestText(markers, radius)` → `app/src/engine/client.ts` の `guestText` → Worker の `guest-text`(`app/src/engine/worker.ts` の `guestText`)。既定では呼ばれない(測定の道具だけが呼ぶ)。
- 目印: FL の起動の行の先頭(`Browser caching took 0 milliseconds.`・`There were no updates for MIDI scripts.`・`There were no updates for themes.`・`Mixer track map: 1,2,3,4`・`There were no updates for`・`Browser caching took`)。ASCII と UTF-16LE の両方で探し、見つかった場所の前後の印字可能な文字の連続を、改行で行に分けた。行の最も多い文字列を取った。
- 見つかった時刻(`debugLog.foundAt`: 行の多い読みの時刻): 起動の idle(Debug 頁を押す前)か、Debug 頁を押した後か。DPR 1・1.25・1.5・1.7647 は **idle** の読みが選ばれた(105 行)。DPR 2・2.02・2.01 は **Debug 頁を押した後**の読みが選ばれた(174 行)。
- 見つかった形式: 目印は全て **ASCII**(UTF-8 と同じ)で見つかった。UTF-16LE の目印では、行の最も多い文字列にならなかった。

### 10.1 F10 の設定窓・タブ(実測)

| DPR | 帯に出た頁 | Debug の頁 |
|---|---|---|
| 1 | MIDI・Audio・General・Theme・Project・Account・About | 無 |
| 1.25 | 同上 | 無 |
| 1.5 | MIDI・Audio・General・Theme・Project・Account(About は届かず) | 無 |
| 1.7647 | MIDI・Audio・General・Theme・Project・Account・About | 無 |
| 2・2.02・2.01 | MIDI・Audio・General・Theme・Project・Debug・Account・About | 有(本文は `text ""`) |

- File・Info は、どの DPR でも帯に出なかった。Info の頁は押せず、Info の欄は読めていない(10.5)。
- General 頁の見出し(窓の表。9 件): `System`・`Display (requires a restart to take effect)`・`Advanced`・`Miscellaneous`・`Input`・`Updates and privacy`・`Language (requires a restart to take effect)`・`Hints and warnings`・`Undo history`。窓の表で非 ASCII の文字は `?` と出る(`u16_ascii` の変換。見出しの空白の一部)。

### 10.2 Debug ログ(実測)

- 中身は FL の起動のログの行。ファイル名は `FL Studio\Support\Logs\Startup\Startup 2021-01-01_000000.log`(ゲストの時計は 2021-01-01 00:00:00)。
- 行数(`debugLog.lines`・`captured`・`method`): DPR 1 → 105 行、1.25 → 105、1.5 → 105、1.7647 → 105、2 → 174(Debug 頁を押した後)、2.02 → 174、2.01 → 174。7 行とも `captured` true・`method` `"guest-memory-scan"`。(1 回目の実測では DPR 1 が 109 行。起動の後の行の数が時刻で変わる。)
- 先頭 30 行(DPR 1。idle の読み):

```
[01/01/2021 00:00:00] C:\Users\user\Documents\Image-Line\FL Studio\Support\Logs\Startup\Startup 2021-01-01_000000.log
[01/01/2021 00:00:00] HostVersion=1 Flags=00000000
[01/01/2021 00:00:00] Init module
[01/01/2021 00:00:00] Init core paths
[01/01/2021 00:00:00] FL Studio v26.1.7.5653
[01/01/2021 00:00:00] BeforeRunning
[01/01/2021 00:00:00] Starting at 12:00:00 AM
64Bit addressing mode
3DNow! support: no
SSE    support: yes
SSE2   support: yes
SSE3   support: no
SSSE3  support: no
AVX    support: no
Touchscreen not available
Main GUI scaling factor: 1 (system set to 96PPI)
Temporary folder for this session:
C:\Windows\Image-Line\4096-{1C588F8C-DCE4-4D23-BFD8-2E006150E3EF}\
FL Studio language: English (US)
Windows display language: en-US
---
[01/01/2021 00:00:00] Loading tags started...
[01/01/2021 00:00:00] Browser caching took 2 milliseconds.
[01/01/2021 00:00:00] ReadIniFile
[01/01/2021 00:00:00]   Restore window state
[01/01/2021 00:00:00]   Restore settings window
[01/01/2021 00:00:00]     Assigned(MIDIForm) = 
[01/01/2021 00:00:00]   Read various options
[01/01/2021 00:00:00]   Read more options
[01/01/2021 00:00:00]   Load MRU list
```

- 絞った行(`scalingLines`・DPR 1。8 行。`dpi`・`scal`・`screen`・`monitor`・`display`・`resol`・`zoom`・`window`・`error`・`warn` を含む行): `Touchscreen not available`・`Main GUI scaling factor: 1 (system set to 96PPI)`・`C:\Windows\Image-Line\4096-{…}\`・`Windows display language: en-US`・`Restore window state`・`Restore settings window`・`Restore more window settings`・`Restore settings for most windows`。

### 10.3 FL 自身の拡大率の記録(実測)

FL のログの行 `Main GUI scaling factor: F (system set to N PPI)` が、FL の使った係数 F と、そのもとのシステムの PPI N を書く。

| DPR | 係数 F(ログ) | PPI N(ログ) | N ÷ 96 | 拡大率 `flScalingPercent` |
|---|---|---|---|---|
| 1 | 1 | 96 | 1.000 | 100 |
| 1.25 | 1.25 | 120 | 1.250 | 125 |
| 1.5 | 1.5 | 144 | 1.500 | 150 |
| 1.7647 | 1.75 | 169 | 1.760 | 175 |
| 2 | 2 | 192 | 2.000 | 200 |
| 2.02 | 2 | 194 | 2.021 | 200 |
| 2.01 | 2 | 193 | 2.010 | 200 |
| (1 回目)1.42 | 1.5 | 136 | 1.417 | 150 |
| (1 回目)1.47 | 1.5 | 141 | 1.469 | 150 |

- 実測: 係数 F は、N ÷ 96 を 0.25 の段に丸めた値と、9 行とも一致した(FL のログの値。FL 自身の規則の材料)。Windows 側の値の違いは、この機械では判定できない(未確認)。
- 推定: ログは `system set to N PPI` と書くので、Main GUI Scaling は System だと推定する。ただし、選択の欄そのものは読めていない(10.4)。

### 10.4 Display(Main GUI Scaling)(実測と未確認)

- `Main GUI Scaling` の文字(ASCII・UTF-16LE)は、メモリに **見つからなかった**(0 箇所)。
- `Pop-ups` の近傍(ASCII 2 箇所・UTF-16LE 1 箇所)に、選択肢の列 `100%`・`125%`・`150%`・`175%`・`200%`・`250%`・`300%`・`350%`・`400%` が出た(9 件)。同じ近傍に `System` は出なかった。
- 選ばれている添字は読めていない。よって `display` は null、`displayChoices` は 5 項目とも null(**未確認**)。Main GUI Scaling が System か否かは、判定できない(推定は上の 10.3)。
- `Toolbars` の近傍(21 箇所)は、操作の名前(`RealignWindowsActionH`・`ArrangeWindowsIntoWorkSpaceActionP`・…)で、選択肢ではない。近傍の探索は 24 箇所で打ち切ったため、`Cursors`・`Legacy scaling` は一覧に入らなかった(**未確認**)。

### 10.5 Info 頁(未確認)

- Info の頁は、どの DPR でも帯に出ず、押せなかった。`info` は null(**未確認**)。
- 前の版は Info の欄の代わりに、別のプラグインの説明文を拾っていた(`Windows`・`FL Studio` の目印の近傍)。それは取り除いた。Info の頁を押した後の読みだけを `info` に入れる。

### 10.6 FL の設定の読み(実測)

- `scalingRegistry`(名前に `scaling`・`GUI`・`uiscale`・`PPI` を含む `RegQueryValueExW`): 7 行とも null。FL は、この経路では拡大率の設定を読んでいない。係数は起動のログに書かれている(10.3)。

### 10.7 判定(受け入れ)

- 数値の判定(2 回目の出力の 7 行): `error` null・`state` engine-idle・`standIn` false・`flScalingPercent`(100・125・150・175・200・200・200)・`edgeWidth`(2.261・2.425・2.527・2.626・3.199・3.199・3.199)・`pointerErrPx` 0(全行)・`parts` 有限。→ **受理**(`#1353` の判定のまま)。
- DPR 1 と DPR 1.7647 の Debug ログ: `captured` true・`lines` 105・`method` `"guest-memory-scan"`。→ **受理**。
- ソースの検査: 追加行に 125・150・168・169・175・176・1.75・1.7647・0.5667 は無い(`refs/judge/main` は、この機械には無いので、HEAD との差分で代わりに確かめた。0 件)。→ **受理**。
- 単体テスト `npm test --prefix app`: 68 ファイル合格(1 件スキップ)、1335 件合格(10 件スキップ)、終了 0。→ **受理**。
- DPR 2.5・3: 停止(`fle thread 2 failed: RangeError: Maximum call stack size exceeded`。`#1354`。判定から外す)。
- 判定から外した項目(JSON と報告には出す): `display`・`displayChoices`・`info`(未確認)、`scalingRegistry`(null)、`canvasDisplay`(canvas と一致)。

### 10.8 道具の変更(今回)

- `app/src/engine/worker.ts`: `guest-text` の要求(`guestText`。メモリの読みのみ。目印を ASCII と UTF-16LE で探し、行に分け、近傍の印字可能な連続を取る)。`app/src/engine/client.ts`: `guestText`。`app/src/engine/protocol.ts`: 要求と応答の型。`app/src/main.ts`: `window.__app.guestText`(測定の道具だけが呼ぶ)。
- `tools/feat-c1-hidpi/measure-size.mjs`: `debugLog`(起動の idle と Debug 頁の後の 2 回の読み。`foundAt`・`marker`・`enc`・`offset`・`head`・`scalingLines`・`scalingFactor`)、`displayNear`・`displayChoices`・`display`(Display の節。未確認は null)、`info`(Info 頁を押した後の読み)、`canvasDisplay`(canvas の寸法。旧 `display`)。`mainGuiScaling` の欄は `settings.scalingRegistry` に移した。
- 製品の既定の動作(利用者の画面)は変えていない。新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数は足していない。ゲストのメモリには書かない(読むだけ)。

### 10.9 次にやること(#1358 の後)

1. Display の選択値: `Pop-ups` の近傍の選択肢の列(100% … 400%)を持つ構造を、メモリの中で探し、選ばれている添字を読む。`System` の選択肢が同じ列にあるかも確かめる。
2. 近傍の探索の上限(24 箇所)を、目印ごとに分ける(`Cursors`・`Legacy scaling` を一覧に入れる)。
3. Info の頁を帯に出す条件(窓の幅・FL の設定)を確かめる。
