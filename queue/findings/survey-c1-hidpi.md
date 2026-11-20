実エンジン: lift/out-app の engine と bundle を fe-build で作り(ビルドの 5 段とも終了コード 0)、その bundle を headless Chrome で 5 回起動し、engine-idle の後に canvas[data-desk="1"] の寸法を読んだ(出力は JSON の行)。

# survey-c1-hidpi: C-1(HiDPI で画面がぼやける)の現状の測定

以下の測定の値は、すべて runner の値。この機械とは別枠。runner の種類は nproc = 4。loadavg は各 JSON の行の値(終了時の 1 分・5 分・15 分)。製品は変えていない(変更は本ファイルだけ)。

## 実測

### 手順 1: 版と抽出物
- `grep '^EMSDK_VERSION=' ci/versions.env` の出力: `EMSDK_VERSION=7.0.0`
- `find lift/out-app/host -type f | wc -l` の出力: `238`

### 手順 2: ビルド(段ごとの終了コード)

| 段 | 終了コード | 出力の要点(抜粋) |
|---|---|---|
| `npm ci --prefix app` | 0 | `added 74 packages, and audited 75 packages in 3s`、`found 0 vulnerabilities`(1 回目は終了コードを取らずに実行し、2 回目で 0 を確認) |
| `npm run build --prefix app` | 0 | typecheck の後に vite build。`built in 144ms` |
| `cargo build --release --locked --manifest-path lift/Cargo.toml` | 0 | `Finished release profile [optimized] target(s)`(fe-wasmgen の警告 7 件。エラーなし) |
| `lift/target/release/fe-build engine` | 0 | `hybrid: 167414828 bytes, 8 direct modules` |
| `lift/target/release/fe-build bundle` | 0 | `engine wasm  167414828 B`、`nothing was uploaded` |

bundle の場所: `lift/out-app/bundle/raw/fl-Oz`(br は `lift/out-app/bundle/br/fl-Oz`)。

### 手順 4: 実測(headless Chrome、拡大率 2)
- Chrome は 157.0.8103.50(JSON の chromeVersion)。
- `--force-device-scale-factor=2` で dpr は 2 になった(JSON の hidpi.dpr)。CDP の Emulation.setDeviceMetricsOverride には替えていない。
- 下の JSON の行は、各実行の出力から `state`・`elapsedMs`・`loadavg`・`error`・`hidpi`・`canvasInfo` の鍵だけを抜いたもの(engineLog30 など他の鍵は省略。数字には関係しない)。

手順どおりの起動(3 回):

- 実行 1: `{"state":"engine-idle","elapsedMs":10584,"loadavg":[1.44,1.26,1.94],"error":null,"hidpi":{"dpr":2,"canvasW":640,"canvasH":480,"cssW":132,"cssH":99,"ratio":2.4242424242424243},"canvasInfo":"guest screen 640x480 in a room of 500x99 CSS px (scale 0.2065, device pixel ratio 2, 0.31 Mpx)"}`
- 実行 2: `{"state":"engine-idle","elapsedMs":10205,"loadavg":[1.14,1.2,1.9],"error":null,"hidpi":{"dpr":2,"canvasW":640,"canvasH":480,"cssW":132,"cssH":99,"ratio":2.4242424242424243},"canvasInfo":"guest screen 640x480 in a room of 500x99 CSS px (scale 0.2065, device pixel ratio 2, 0.31 Mpx)"}`
- 実行 3: `{"state":"engine-idle","elapsedMs":10211,"loadavg":[1.72,1.33,1.92],"error":null,"hidpi":{"dpr":2,"canvasW":640,"canvasH":480,"cssW":132,"cssH":99,"ratio":2.4242424242424243},"canvasInfo":"guest screen 640x480 in a room of 500x99 CSS px (scale 0.2065, device pixel ratio 2, 0.31 Mpx)"}`

| 実行 | dpr | canvas 幅×高 | CSS 幅×高 | canvas 幅 ÷ CSS 幅(計算) | ratio(式の値) |
|---|---|---|---|---|---|
| 1 | 2 | 640×480 | 132×99 | 4.85 | 2.4242 |
| 2 | 2 | 640×480 | 132×99 | 4.85 | 2.4242 |
| 3 | 2 | 640×480 | 132×99 | 4.85 | 2.4242 |

読み: この既定の窓では room が 500×99 CSS px しかなく、engine の下限 640×480 に guest 画面が丸められた(canvasInfo の `scale 0.2065`)。canvas は CSS 132×99 に縮められて表示される。手順どおりの起動では、guest 画素と CSS 1px が 1:1 になる場合は測れていない。

手順外の追加測定(2 回): 手順 4 と同じ起動に `--window-size=1920,1080` を足した(一時ファイルの extraArgs だけを変えた)。

- 追加 1: `{"state":"engine-idle","elapsedMs":10404,"loadavg":[1.58,1.32,1.9],"error":null,"hidpi":{"dpr":2,"canvasW":1580,"canvasH":957,"cssW":1580,"cssH":957,"ratio":0.5},"canvasInfo":"guest screen 1580x957 in a room of 1580x957 CSS px (scale 1.0000, device pixel ratio 2, 1.51 Mpx)"}`
- 追加 2: `{"state":"engine-idle","elapsedMs":10231,"loadavg":[1.59,1.33,1.88],"error":null,"hidpi":{"dpr":2,"canvasW":1580,"canvasH":957,"cssW":1580,"cssH":957,"ratio":0.5},"canvasInfo":"guest screen 1580x957 in a room of 1580x957 CSS px (scale 1.0000, device pixel ratio 2, 1.51 Mpx)"}`

| 実行 | dpr | canvas 幅×高 | CSS 幅×高 | canvas 幅 ÷ CSS 幅(計算) | ratio(式の値) |
|---|---|---|---|---|---|
| 追加 1 | 2 | 1580×957 | 1580×957 | 1.00 | 0.5 |
| 追加 2 | 2 | 1580×957 | 1580×957 | 1.00 | 0.5 |

読み: guest 画面が room と一致し(scale 1.0000)、guest 1 画素 = CSS 1px。描画バッファは CSS と同じ画素数で、dpr 2 の物理画素の半分。

参考(P1 に関係。runner の値): navigate から engine-idle の確認までの elapsedMs は、手順どおりの 3 回で 10205〜10584 ms、追加の 2 回で 10231〜10404 ms。確認は 1 秒おきの poll である。

### 式の読み
式 `ratio = canvas 幅 ÷ (CSS 幅 × dpr)` は、描画バッファの画素数を物理画素数で割った値である。
- ratio = 1: 描画バッファと物理画素が 1:1(HiDPI に合っている)。
- ratio = 1/dpr = 0.5: 描画バッファが CSS 1px に 1 画素(ブラウザが 2 倍に引き伸ばす)。

手順 5 の文面は「ratio が 1 に近ければ CSS 1px に 1 画素」としていたが、式の値では逆になる。このため本報告は、式の値(ratio)と、canvas 幅 ÷ CSS 幅(CSS 1px あたりの描画バッファ画素数)の両方で読む。plan.md の C-1 の目標は canvas = CSS × 拡大率(ratio = 1)なので、実測の 0.5 はその逆側にある。

## コードの読み

### 手順 3: grep の結果(実行していない。読みのみ)
`grep -n 'canvas.width\|canvas.height\|devicePixelRatio' app/src/ui/desktop.ts app/src/main.ts app/src/ui/screen-fit.ts app/src/engine/desk.ts app/src/engine/desk-gl.ts` の出力:

```
app/src/ui/desktop.ts:24:    canvas.width = this.screen.width;
app/src/ui/desktop.ts:25:    canvas.height = this.screen.height;
app/src/main.ts:262:  return { width: r.width, height: r.height, dpr: window.devicePixelRatio || 1 };
app/src/main.ts:265:  const dpr = window.devicePixelRatio || 1;
app/src/main.ts:304:window.matchMedia?.(`(resolution: ${window.devicePixelRatio}dppx)`).addEventListener?.('change', applyDisplay);
app/src/main.ts:437:  return { ...collectBrowserInfo({ navigator, screen, devicePixelRatio: window.devicePixelRatio, ... (中略) ...
app/src/main.ts:450:  return { mode: 'fit', source: 'default', ... dpr: Math.round((window.devicePixelRatio || 1) * 10000) / 10000, follow: 'live' };
```

(app/src/ui/screen-fit.ts、app/src/engine/desk.ts、app/src/engine/desk-gl.ts には、この grep の一致は無い。)

読み: canvas の大きさに devicePixelRatio を掛けている行は、この 5 ファイルに無い。canvas の大きさを決めているのは desktop.ts:24〜25 の guest 画面の大きさだけで、dpr は掛けていない。main.ts の dpr の参照は、room の記録(262)、表示の文言(265、275)、dppx の変化の監視(304)、beacon の記録(437、450)に限られる。

### 読んだ物の要点(パスと行番号つき)
- app/src/ui/desktop.ts:21〜27: deskCanvas() が canvas を作る。大きさは guest 画面(24〜25)。
- app/src/ui/desktop.ts:42: screenCssSize は guest 画面 × scale を snap で丸めた値。dpr は掛けない。
- app/src/ui/desktop.ts:45〜52: apply() が root と canvas の CSS の大きさを screenCssSize にする(47〜48、50〜51)。
- app/src/ui/screen-fit.ts:3〜4(注釈): 「1 guest pixel = 1 CSS pixel. ブラウザが canvas を devicePixelRatio で拡大する」。拡大は、設計として、ブラウザに任せている。
- app/src/ui/screen-fit.ts:6、8、9: SCREEN_DEFAULT は 1920×1080、SCREEN_MIN は 640×480、SCREEN_MAX は 7680×4320。
- app/src/ui/screen-fit.ts:44〜50: chooseGuestScreen は、room を限界の中に入れる(clamped)。57〜60: drawScale は、engine が要求どおりの大きさを採った時、scale = 1 にする。
- app/src/main.ts:260〜262: measureRoom は stage の CSS の大きさを room とする。
- app/src/main.ts:264〜276: applyDisplay は guest 画面を room に合わせ、view.setScreen(273)と view.setScale(274)に渡す。dpr は表示の文言(275)だけ。
- app/src/main.ts:303〜304: ResizeObserver と dppx の変化で applyDisplay を呼ぶ(再描画。大きさは変えない)。
- app/src/main.ts:450: beacon の dpr は記録のみ(dpi は null)。
- app/src/engine/client.ts:86: 同じ canvas を transferControlToOffscreen で Worker に渡す。
- app/src/engine/desk.ts:87、app/src/engine/desk-gl.ts:208: Worker 側の surface の大きさは guest 画面の大きさ(st.width/height、sc.width/height)。両方の経路で dpr は掛けていない。
- lift/runtime/fle_desk.h:3: 画面の大きさは fle_screen.h で決まり、既定は 1920×1080。1 画素 4 バイト(RGBA)。
- tools/boot-check/boot-bundle.mjs:70: launchChrome に ISOLATION の引数だけを渡している。tools/boot-check/cdp.mjs:41〜46: launchChrome の extraArgs は固定の引数の後ろに付く。

読みと実測の対応: desktop.ts:24〜25 と 47〜51 の通り、canvas は guest 画面の画素数で、CSS は guest 画面 × scale。dpr 2 で scale 1 の場合、ratio = 1/dpr = 0.5 になる(追加 1・2 の実測)。読みと実測は一致する。

## 判定
- この task の測定: 受理。手順 1〜4 の出力がそろい、5 回の実行で値が一致し、コードの読み(desktop.ts:24〜25、47〜51、screen-fit.ts:3)と一致した。
- 「ぼやける」の見た目: 未確認(画像の比較はしていない。描画バッファの大きさの測定だけ)。
- C-1 の受け入れ条件(案: canvas の幅と高さ = CSS × 拡大率、plan.md:36)は、現状では合わない(実測: canvas 1580 に対し CSS × 2 = 3160。ratio 0.5)。

## 未確認
- ぼやけの見た目: 画像で比べていない。
- 拡大率 1.5: 測っていない(plan.md:36 の受け入れ条件は 2 と 1.5)。--force-device-scale-factor=1.5 で測る必要がある。
- 既定の窓(手順どおりの起動)で room が 500×99 CSS px になった理由: 調べていない(推定: 既定の窓の大きさと stage のレイアウト)。
- 実行の経路(2D の desk.ts か WebGL の desk-gl.ts か): 記録していない。両方とも dpr は掛けていない(読み)。
- 窓の大きさを変えた後(リサイズ後)の canvas: 測っていない。engine-idle の時点だけ。
- ブラウザが拡大した後の物理画素の色や鮮明さ: 測っていない。DOM の寸法と式だけで読んだ。
- 他のブラウザや他の GPU: 未確認。runner の値だけ。
- 追加測定(窓の大きさを 1920×1080 にした 2 回)は、手順の外の起動。手順どおりの 3 回と分けて扱った。

## 次の手順の案
1. C-2(P3、描画と音声の分離)を先に行う。理由: plan.md:142 の順序どおり。画素数を増やすと、音声と同じ Worker の描画の負荷が増える(推定)。AGENTS.md の優先は P3 > P2。C-1 の受け入れ(plan.md:36)も、C-2 の受理の後に P3 の underrun が 0 であることを確かめる。
2. C-1 の実装の案は次の 2 つ。どちらも影響は推定。
   - 案 A(ぼやけを直す方向): guest 画面の大きさを room × dpr(物理画素)にし、canvas の大きさも同じにする(desktop.ts:24〜25、47〜51、main.ts:273〜274 の扱いを dpr で変える)。engine へは FLE_MONITOR_W/H に物理画素を渡す。
     - 画素数は拡大率の 2 乗で増える。計算: 1580×957(1.51 Mpx)→ 3160×1914(6.05 Mpx)、4 倍。既定の 1920×1080(2.07 Mpx)→ 3840×2160(8.29 Mpx)、4 倍。
     - 描画の所要も、概ね同じ比で増える(推定)。P2(120 fps)と P3 に効くので、C-2 の後に測る。
     - 描画の記憶も増える。4 バイト/画素で 6.05 Mpx は約 24.2 MB(計算。推定の対象は engine の picture と canvas の両方)。
     - guest に DPI を伝えないと、窓の文字や部品が小さく見える(推定。現在の beacon は dpi が null: main.ts:450)。DPI の扱いは、監査が決める。
   - 案 B(canvas だけ拡大する方向): guest の大きさは据え置き、canvas だけ CSS × dpr にして Worker で拡大する。描画の元が増えないので、ぼやけは直らない(推定)。画素数の負荷だけが増える。
3. C-1 の後に、拡大率 2 と 1.5 で、canvas 幅 ÷(CSS 幅 × 拡大率)が 1 になることを測る(plan.md:36)。
4. 測定の窓の大きさを、手順の中で固定する(既定の窓では room が小さく、guest が下限に丸められる)。製品には、新しい環境変数・#ifdef・URL パラメータ・ビルド変数を足さない(AGENTS.md)。
