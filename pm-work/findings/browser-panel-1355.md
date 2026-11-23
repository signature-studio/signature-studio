# #1355 Browser パネルの黒い領域: 切り分けの記録

測定の道具: `tools/browser-panel/measure.mjs`(DPR 1・部屋 1836 x 1056 CSS px)。基準: `browser-baseline-115.json`(runner の値。nproc 4、loadavg 1.48 1.21 1.67)。

## 基準(変更の前。製品のビルド。2 回の平均ではなく、1 回目を commit)

| 項目 | 1 回目 | 2 回目 |
| --- | --- | --- |
| blackShare | 0.929158 | 0.929158 |
| browserComponents | 1 | 1 |
| elapsedMs | 7751 | 7797 |
| preBootBytes | 314,326,602 | 314,326,602 |

矩形は x 0・y 76・364 x 974 CSS px(道具の定義。変えない)。窓の表(`summary.windows`)には Browser が無い(子窓のため)ので、fallback を使った。

## 結論(今回の試行の probe で確定)

黒い領域は、一覧の面を載せた窓 `TEdgeBrowser`(Delphi の WebView2 の wrapper)の面そのもの。一覧は GDI で描かれていない。runtime は WebView2 を持たない(`lift/runtime/fle_load.c`、decision 0126 wall 86、S-webview2-absent: `WebView2Loader.dll` は意図的に無いものとして扱う)。そのため、この窓は何も描かない。列挙の欠落も、WM_PAINT の配送の欠陥も無い。

Issue の前提(「起動後にフォルダを列挙して一覧を作る」が描画の原因)は、この切り分けでは当てはまらない。列挙は件数を返している(下の 4)。

## 根拠(実測。probe は trace の既定を 1 にし、窓の作成・WM_PAINT の配送・BeginPaint・既定の処理を記録した。probe は製品に残していない)

1. 黒い面の位置: 絵の行 145..1049 が全幅(x 0..363)で黒い。905 行 = 364 x 905 の面。黒い割合 = 905 / 974 = 0.929158(基準の blackShare と一致)。行 76..144(ヘッダーとタブ)には黒い画素が無い。
2. 窓と配送先:
   - lid 107 `TVectorPanel`(0x7f9901ac、364 x 905、親は TSampleListForm lid 68)。`synth-paint lid=107 upd=0,0,364,905` の後、`DispatchMessageW` が wndproc(Delphi の thunk 0x13240264)へ届ける。BeginPaint も DefWindowProcW も呼ばれない(Delphi の処理が WM_PAINT を取り込んでいる)。
   - lid 108 `TEdgeBrowser`(0x7f9901b0、364 x 905、親は lid 107)。`call DispatchMessageW ... msg=0xf proc=0x14570909` → `call CallWindowProcW ... proc=0x7ff900000e30`(前の wndproc = DefWindowProcW)→ `def-paint lid=108 upd=0,0,364,905 erase=1 no_redraw=0 chain_no_redraw=0`。消去は窓のクラスに背景ブラシが無く(WM_ERASEBKGND の答えは 0)、何も描かれない。
3. WebView2: `missing-path: LoadLibraryW 126 "WebView2Loader.dll"`(TEdgeBrowser の生成時。126 = ERROR_MOD_NOT_FOUND。runtime の意図的な不在)。
4. 除外した候補:
   - WM_SETREDRAW(no_redraw): lid 49(TWPControl)は wp=0 の後に wp=1 を受けて釣り合う。lid 108 の no_redraw は 0。
   - 表示の欠落: SWP_SHOWWINDOW と ShowWindow の記録があり、表示は無い。
   - 列挙の欠落: `enum-dir:` の件数が返る(`Mixer presets\*` 18、`Speech\*` 71、ユーザーの `Presets\Plugin database\*` 0)。

## 矩形の定義

黒い面は絵の y 145..1049 で、矩形 y 76..1049 がそれを含む。矩形は Browser パネル(ヘッダーの行 76 から下端まで)を覆う。定義は正しい。窓の表に Browser が無いため `summary.windows` とは比べられない。

## 変更の後の値(受け入れの実行。製品の変更なし。runner の値。nproc 4、loadavg 2.62 1.71 2.12)

| 項目 | 基準 | 今回 | 判定の式 | 結果 |
| --- | --- | --- | --- | --- |
| blackShare | 0.929158 | 0.929158 | ≤ 0.464579(基準の半分) | 差し戻し |
| browserComponents | 1 | 1 | ≥ 10 | 差し戻し |
| elapsedMs | 7751 | 10716 | ≤ 8251(基準 + 500) | 未確認(負荷の高い実行。基準の再測定が要る) |
| preBootBytes | 314,326,602 | 314,328,049 | ≤ 315,375,178(基準 + 1 MB) | 受理 |
| 起動 | — | engine-idle・standIn false・error null | — | 受理 |
| `npm test --prefix app` | — | 68 files passed、1332 tests passed、10 skipped | 通る | 受理 |

出力の行: `{"dpr":1,"state":"engine-idle","standIn":false,"error":null,"blackShare":0.9291581108829569,"browserComponents":1,"elapsedMs":10716,"preBootBytes":314328049,...}`

probe の変更は戻した(`git checkout` で `lift/runtime/fle_win.c` を HEAD に戻した。差分は probe の行だけだった。戻した後の engine.wasm に probe の文字列は 0 件)。

## 次に決めること(依頼者・監査の判断が要る。作業者は決めない)

この Issue の目標(一覧の行が描かれる)は、WebView2 の窓(`TEdgeBrowser`)を runtime がどう扱うかで決まる。

- (a) WebView2 のモデルを作る(`LoadLibrary` の応答、COM の API、HTML の描画)。decision 0126 wall 86 が「代案」として挙げた大きな設計。
- (b) 窓の面をページ側の要素(iframe など)で置き換える。一覧の HTML の出所(FL が TEdgeBrowser に渡す内容)を先に確かめる必要がある。
- (c) 一覧は空のままとし、この Issue の目標を変える。

作業者は、黒い面を背景色で塗るなどして割合だけを下げる変更はしない(一覧の行が描かれないまま、測定の値だけが変わるため)。
