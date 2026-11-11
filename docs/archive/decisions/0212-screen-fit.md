# 0212: ゲストの『画面』を、ヘッダーと右のモニターを除く全領域に合わせる(起動時の寸法の受け渡し、`fit`/`fit-device`/`legacy`、DPI、実行中のリサイズ追従)

状態: **仮の決定**。依頼は PM(決定 0212、依頼者の『なぜ画面がこんなに小さいのだ?』を受けて)。実装・測定・文書は作業者 `screen1`(2026-11-04 JST 03:49〜)。作業は HEAD e7af37b の `git archive`(`lift/out-screen1/src/`)で始め、HEAD が 6f5801c(dd2 取り込み後)、0a3ab9a(`?dedupe` 既定 ON 取り込み後)、55e4e83(act1・dec1・api2・present2〔0214〕取り込み後)、a0d658a(opfs1 段階 2 取り込み後)、0156065(present2 段階 1b 取り込み後)、0235dff(aud3 取り込み後)、**62ac3e9**(opfs1 段階 3 の文書取り込み後)に進んだので、差分を各 HEAD に載せ替えて最終化した(`screen1.diff` は **62ac3e9 の `git archive` に当たる**。`patch -p1 --dry-run` は rc=0、reject なし。a0d658a への載せ替えでは `app/src/main.ts` の 1 か所と `app/src/perf/beacon.ts` の 4 か所を、opfs1 が `files` の欄をビーコンに足した直前・直後に手で載せ直した。0156065 の時点で PM のワークツリーに未コミットで入っていた aud3 とも当たるよう、`#display-box` を `<h1>` の直後に、`main.ts` の私のブロックを `installAudio` の手前に移した)。測定のうち、実エンジンのバンドル(Chrome/Firefox/node の run、費用、16 スナップ)は e7af37b 由来の木に本差分の runtime を載せたもの(`fle_win.c` ほか runtime が 55e4e83・0156065 で変わった分は含まない)で測り、HEAD + 本差分の木では Rust 全体テスト・a4・app の検査だけを走らせた(§8 に木を区別して書いた)。区分: **実測**＝このセッションのコマンドの出力を見た事実、**読み**＝コード・文書を読んだ解釈(走らせていない)、**推測**＝実機 Windows の挙動の推測、**出典なしの選択**＝`S-screen1-…`(§9、`tables/s-choices.tsv`)。環境は『この機械』= 16 コアの Linux VM(他担当の負荷が常にあり load 28〜55)、隔離 headless Chrome 156(SwiftShader)、Firefox 158(headless、geckodriver)、node 24。**依頼者の PC(Firefox 159 / Windows、dpr 1.7647)ではない。実機 Windows との照合は 0 件。**

## 冒頭の三点

1. **何を実測したか**: 実エンジン(`FLEngine_x64.dll` と同梱 5 DLL を x86-64 → C → wasm にした物。ship2 の `F2xH2` の obj 木に runtime の 10 ファイルだけ再コンパイルして web link / node link した wasm、wasm-opt なし 174 MB、`engineKind` = `emscripten-fruity-v1`、STAND-IN なし)を、(a) node(g3 スクリプト)で、画面 1400x900・2400x1300・3240x1923 と既定(1920x1080)、DPI 96 と 169、実行中の表示変更(`display` 1800x1000 → 2400x1300)で動かし、(b) 隔離 Chrome 156 で viewport 2400x1300・1400x900・800x900・2176x1092(dpr 1 と 1.7647)、`fit`・`fit-device`・`fit-device&dpi=auto`・`legacy`・`panel=0`・`follow=live`・`follow=reload` の 13 通りと、費用(アイドル・再生の CPU 秒/秒、各 3 回前後)を測り、(c) Firefox 158 でも同じページを開いた。さらに Rust 全体テスト・a4(FRUITY 無効)・16 スナップ(84 行。先頭 82 行が期待表と同一)・FLEngine の既定の生成 C の sha256・app の vitest/typecheck/e2e/Worker/変異検査を実測した。
2. **結論**: (i) **ゲストの画面寸法はページの room に一致した**(下の §4 の表の 10 行と §5 のリサイズの 3 run、全 13 通りで、エンジンが `fle_av_poll` で報告した画面 = ページが測った room〔fit〕または room × dpr〔fit-device〕、主窓 `TFruityLoopsMainForm` = `(-5,-6, W+10, H+12)`、ページはスクロールしない、実マウス位置→ゲストの `GetCursorPos` が 26 点全部一致)。従来(`legacy`)の依頼者と同じ viewport(2176x1092、dpr 1.7647)は desktop 1088x612 CSS px で room(1788x930)の幅の 61%・高さの 66% だった。(ii) 小さく見えた原因は 2 つ: 定数 1920x1080 と、`?scale=fit` の上限『1 ゲスト画素 = 1 デバイス画素』(dpr 1.7647 で 1088 CSS px)。(iii) **`fit` は費用ゼロ**(従来と差なし)。`fit-device` は 6.03 Mpx(約 2.9 倍)で、ページ主スレッド +75〜110%、Worker(再生)+22〜51%、wasm メモリ +100 MB(§7)。(iv) **FL は DPI を見て自分の UI を拡大する**(DPI 169 でウェルカムウィザード 874x570 → 1529x1003、主窓の UI も)。`fit` は `FL64 (Scaled).exe`、`fit-device&dpi=auto` は `FL64.exe`(manifest `dpiAware True/PM`)と同じ動作(§6)。(v) 実行中のリサイズ: メッセージ(`WM_DISPLAYCHANGE`/`WM_SETTINGCHANGE`)だけでは FL の主窓は動かず、最大化窓を置き直す runtime の opt-in(`FLE_SCREEN_REMAXIMIZE`)を足すと追従した(§5)。(vi) 既定の挙動は変わらない: env 無しで FLEngine の生成 C の sha256 `cd9107ce…`(215,811,678 B)・a4(基準の `a4-fruity.sh off`: e7af37b 由来の木で 942/942 など、0156065 + 本差分でも 942/942・895/895・5,310・1,009,731・exit 0)は不変、16 スナップは e7af37b 由来のバンドルで先頭 82 行が期待表とバイト同一(62ac3e9 の present2 の期待表との比較は未実施。§8)、Rust 全体テスト(62ac3e9 + 本差分)186 / 2,321 / 0〔0156065 + 本差分は 185 / 2,320 / 0 で、PM の 0156065 の測定 184 / 2,314 / 0 に `screen1.rs` の 6 件を足した値〕。
3. **未確認**: 62ac3e9 の runtime(present2・dec1・api2・act1)を含むバンドルでの実エンジンの測定(Chrome・Firefox・node・費用・16 スナップはすべて e7af37b 由来の runtime + 本差分のバンドル。62ac3e9〔a4 は 0156065〕+ 本差分の木では Rust テストと a4 と app の検査だけ。§8)、依頼者の Firefox 159 / Windows の実機(Firefox 158 on Linux の run のみ)、実機 Windows の最大化窓の置き直し(§5 の推測)、`WM_DPICHANGED`(DPI の動的変更。ブラウザのズームで DPR が変わったとき)、DPI > 96 での Sampler・Settings・File→Open・FLP・F8・プラグイン、F8 中の費用、wasm-opt 版・出荷版(hybrid)での費用、画面が 7680x4320 に近い場合、エンジン側で FL の最小窓がいくつか、実機の `devicePixelRatio` が 1.7647 以外のときの `floor` の端、ship3 の再ビルド後の確認(今の既定の入口の wasm は `FLE_MONITOR_W/H` を読まない)。

## 1. 背景と依頼

依頼者: 「なぜ画面がこんなに小さいのだ? 上部ヘッダーと右部のモニターを除く全てのエリアを画面とする、で良いはずでは?」
依頼者のブラウザ(ビーコン): Firefox 159 / Windows、画面 2176x1224 CSS px、devicePixelRatio 1.7647(物理 3840x2160)、viewport 2176x1092。

原因は 2 つ重なっていた(実測、既定の入口と同じパラメータ無しで viewport 2176x1092・dpr 1.7647、この機械):
1. ゲストの画面が `lift/runtime/fle_monitor.h` / `fle_gdc.h` の定数 1920x1080(決定 0053・0065 の仮の選択)で、ページは値を渡さなかった。
2. ページの `?scale=`(`view-scale.ts`)の既定 `fit` は上限が『1 ゲスト画素 = 1 デバイス画素』(`maxScale` 1)なので、dpr 1.7647 では 1920 ゲスト画素が 1088 CSS px にしかならない。room(ヘッダーを除く左の列)は、従来のページ(e7af37b、本差分の前。`shots/c0-2176x1092-dpr1.7647-default/`)で 1773x984 CSS px あったので、画面は幅の 61%・高さの 62% で、ページ全体が 2161x1112 にスクロールしていた。本差分の `?screen=legacy`(`shots/f-c0-*`)では同じ desktop 1088x612 で room は 1788x930、ページはスクロールしない(`scrollWidth/Height` = `clientWidth/Height`)。ヘッダーの高さは 104 → 107 px(ヘッダーに『表示』の欄を足したため。読み)。room の高さ 984 → 930 とスクロールの有無の差の原因は調べていない(`legacy` は `body.screen-fit` の規則を受けない)。どちらも desktop は 1088x612 CSS px。

## 2. ゲストが画面寸法を得る経路(読み + 観測)

### 2.1 runtime が画面寸法を答える全箇所(静的な読み、`tables/screen-size-uses.tsv`、44 箇所)

| API(runtime の関数) | 何を答えるか | 以前の値の出どころ |
|---|---|---|
| `GetSystemMetrics` SM_CXSCREEN/CYSCREEN/CXVIRTUALSCREEN/CYVIRTUALSCREEN(`fle_sysmetrics.c`) | 画面の幅・高さ | `FLE_GDC_HORZRES`/`VERTRES` |
| `GetDeviceCaps` HORZRES/VERTRES、HORZSIZE/VERTSIZE(mm)、LOGPIXELSX/Y(`fle_gdc.c`) | 画素数、物理寸法、DPI | 定数 1920/1080、508/286、96 |
| `EnumDisplayMonitors` のコールバックに渡す RECT、`GetMonitorInfoW` の rcMonitor/rcWork、`MonitorFromPoint/Rect/Window` の交差判定(`fle_monitor.c`) | モニター矩形、作業領域(= モニター、タスクバー無し: 決定 0065) | `FLE_MONITOR_RIGHT/BOTTOM` |
| `EnumDisplaySettingsW`(`fle_monitor.c`) | PELSWIDTH/PELSHEIGHT | 同上 |
| `SystemParametersInfoW(SPI_GETWORKAREA)`(`fle_spi.c`)、`GetClipCursor`(`fle_cursor.c`) | 作業領域、クリップ矩形 | 同上 |
| `GetWindowRect`/`GetClientRect` のデスクトップ窓、既定の窓サイズ(CW_USEDEFAULT)、デスクトップ DC のクリップ(`fle_win.c`) | デスクトップ窓の矩形 | 同上 |
| ポインタのクランプ(`fle_input.c`、`fle_win.c set_pointer`) | カーソルを画面内に | 同上 |
| `LPtoDP` の MM_HIMETRIC/LOMETRIC の extent(`fle_gdi.c`) | 画素数と mm の比 | 同上 |
| `shcore!GetDpiForMonitor`(`fle_appfile.c`)、`GetProcessDpiAwareness`(`fle_proc.c`) | DPI、DPI awareness | 96、UNAWARE(決定 0099、S-g8a-dpi) |

どれも定数のマクロ(名前)で書かれていた。静的な文脈(配列の大きさ、case 値)で使う箇所は 0(`-Werror` でコンパイルが通ることで確認)。窓の面(`fle_raster_window_surface`)は窓ごとに大きさを受け取るので、デスクトップ全体の面は無い。

### 2.2 エンジンが起動時と実行中に読む経路(観測、`-DFLE_SCREEN_TRACE`、node、`tables/engine-screen-reads.tsv`)

`fle_screen.h` の `FLE_SCREEN_TRACE`(観測専用、どのバンドルにも入れない)で、画面寸法を読むたびに runtime の関数名を出した。実エンジン(F2xH2、wasm)を node の g3 スクリプト(起動 → ウィザードを Enter で閉じる → 仮想 4 秒)で 1 回:

起動の最初の読みは `EnumDisplayMonitors`(1 番目)→ `GetSystemMetrics`(3 番目)→ `EnumDisplaySettingsW`(7)→ `GetClipCursor`(9)→ `SystemParametersInfoW`(SPI_GETWORKAREA、13)→ `GetMonitorInfoW`(15)。入力が来ると `set_pointer`(344 番目から)、窓が動くと `fle_monitor_rect_intersects`(414 番目から)。`SystemParametersInfoW` は 1 回の走行で 512 読み以上(= 256 呼び出し以上。1 呼び出し 2 読み)で、FL は作業領域を頻繁に読む。この走行では `GetDeviceCaps(HORZRES/VERTRES)` は呼ばれなかった(1 回の走行の観測。他の操作で呼ぶかは未確認)。`GetSystemMetrics(SM_CXMAXIMIZED)` などは答えない index で停止するが、起動と観測した操作では来なかった。

### 2.3 FL が主窓をどう初期サイズにするか(観測、`-DFLE_SCREEN_TRACE_WIN` の窓の変更の記録)

主窓 `TFruityLoopsMainForm` は、`GetMonitorInfoW` 等で読んだ画面から**FL 自身が** `(-5, -6, W+10, H+12)` を計算し、`SetWindowPos`(`(−5,−6,2410x1312)` flags 0x14)で置き、`ShowWindow(SW_SHOWNORMAL)` → 後で `ShowWindow(SW_SHOWMAXIMIZED)` を呼ぶ。`ShowWindow(SW_SHOWMAXIMIZED)` 自体は runtime では `zoomed` の印を付けるだけで窓を動かさない(`fle_win.c`)。保存された位置・サイズを使うか(どこに保存されるか)は調べていない: この run では画面を変えると主窓の大きさが画面に追従した(下の 2.4)。monitor へのクランプは、画面と窓が同じ式のため見えなかった。画面より大きくなりうる Settings 窓の高さが画面の高さ 1300 に切り詰められた(`SetWindowPos` の記録で 1856 → 1300。画面に合わせた切り詰めと読むが、確かめていない)。

### 2.4 ゲストの画面を変えたときの FL の反応(node、実測)

実エンジン(F2xH2 + 今回の runtime、wasm-opt なし)を node の g3 スクリプト(起動 → 仮想 4 秒 → Enter でウィザードを閉じる → 仮想 3 秒 → 4 秒)で動かし、環境変数だけを変えた(`tools/g3run.sh`、`runs/p-*`、`runs/d-*`、`tables/node-env-runs.tsv`)。

| 環境変数(`FLE_MONITOR_W/H`、`FLE_SCREEN_DPI`) | 主窓 `TFruityLoopsMainForm` の矩形 | ウェルカムウィザード | node の最大 RSS |
|---|---|---|---|
| (無し = 1920x1080) | (-5,-6,1930x1092) | (523,255,874x570) | 1683 MB |
| 1400x900 | (-5,-6,1410x912) | (263,165,874x570) | 1663 MB |
| 2400x1300 | (-5,-6,2410x1312) | (763,365,874x570) | 1811 MB |
| 3240x1923 | (-5,-6,3250x1935) | (1183,676,874x570) | 1888 MB |
| 3239x1863、DPI 96 | (-5,-6,3249x1875) | (1182,646,874x570) | 1865 MB |
| 3239x1863、**DPI 169** | (-5,-6,3249x1875) | (855,430,1529x1003) | 1849 MB |

- **主窓は常に `(-5,-6, W+10, H+12)`**: 既定(env 無し)は従来の `(−5,−6,1930x1092)` と同じ。ウィザードは画面の中央。画面を 3 倍の画素にしても停止しない。
- node の RSS は 1.66 GB(1400x900)→ 1.89 GB(3240x1923)。Chrome の wasm メモリは §7 の表(932 → 1030 MB)。
- DPI 169 では、ウィザードが 874x570 → **1529x1003**(×1.749、×1.760)になり、ウィザードの窓の番号(lid)が 48 → 50 に変わり、最後の窓表が 114 → 116 行になった(原因は調べていない)。DPI を渡すと FL の UI の大きさが変わる(`GetProcessDpiAwareness` = 2 と `GetDpiForMonitor`/`LOGPIXELS` の組が効いている、までは確かだが、どの API の答えが効いたかは切り分けていない)。


## 3. 実装

### 3.1 画面寸法の受け渡し(起動時。runtime)

| ファイル | 内容 |
|---|---|
| `lift/runtime/fle_screen.h`(新規、header only) | `FLE_MONITOR_W` / `FLE_MONITOR_H`(エンジンの環境。未設定 = 1920 / 1080)を最初の使用で 1 回読む。10 進数字だけ、範囲 640x480〜7680x4320 の外・不正は**無視して既定**(停止しない)。値は共有の weak 変数 1 組(`fle_screen_px[2]`)に置く。`fle_screen_dpi()`(`FLE_SCREEN_DPI` 96〜384、既定 96)、`fle_screen_set()`、`fle_screen_flag()`、観測専用の `FLE_SCREEN_TRACE` |
| `fle_gdc.h`、`fle_monitor.h` | 既存の `#define FLE_GDC_HORZRES 1920` 等の**行は残し**(Rust のテストが読む)、直後に既定値の静的アサーションを置いて `#undef` し、実行時の値のマクロに再定義。HORZSIZE/VERTSIZE は『画素数 × 254 / (DPI × 10)、四捨五入』(96 DPI で 1920 → 508、1080 → 286 = 以前の定数) |
| `fle_monitor.c` | 2 つの `_Static_assert`(RIGHT == HORZRES)をヘッダーの既定値の検査へ移した(両者が同じ関数になったので実行時には食い違えない) |
| `fle_appview.h/.c` | `fle_av_poll` の予約語 5 個のうち 2 個を `screen_w`/`screen_h`(offset 48/52)にした。**version は据え置き**(読み手は古いままでも offset 48 以降を無視する。0 = この項目が無い古い wasm で、ページは 1920x1080 とみなす)。ページは『エンジンが実際に使う画面』をここから読む |
| `fle_proc.c/.h`、`fle_appfile.c` | `FLE_SCREEN_DPI` > 96 のときだけ `GetProcessDpiAwareness` が PER_MONITOR(2)、`GetDpiForMonitor` と `GetDeviceCaps(LOGPIXELS)` がその DPI |

**ヘッダーだけにした理由**(S-screen1-4): `lift/crates/fe-lift/tests/{gdc,monitor,sysmetrics,cursor,spi,...}.rs` は runtime の `.c` を固定の一覧でコンパイルする。`fle_screen.c` を足すとその一覧が足りなくなりテストの変更が要る。header only + weak 変数ならリンク行は変わらない。**環境変数が無いときの結果はバイト単位で以前と同じ**(下の 9 節)。

### 3.2 ページ(`app/`)

- `app/src/ui/screen-fit.ts`(新規、純粋関数): `?screen=`(**無指定 = `fit`**〔PM の決定〕、`fit-device`、`legacy`、`<W>x<H>`)、`?panel=0|1`、`?dpi=auto|<96..384>`、`?follow=live|reload`、ヘッダーの選択(下の 3.4)、room からゲスト画面を決める `chooseGuestScreen`、描画スケール `drawScale`、room の変化を 300 ms の静止後に 1 回にする `RoomDebouncer`、ビーコンの `GuestScreenInfo`。
- `app/src/main.ts`: 起動前(`client.init` の直前)に room(`#stage` の `getBoundingClientRect`)を測り、`manifest.engine.env` に `FLE_MONITOR_W/H`(と `dpi` のとき `FLE_SCREEN_DPI`、`?follow=live` のとき `FLE_SCREEN_REMAXIMIZE`)を書く。`fle_av_poll` の `screen_w/h` が来たらそれを画面とする(エンジンが値を受け取らない古いバンドルは『報告なし』と注記して 1920x1080 を room に合わせて拡縮)。`window.__app.screen` に要求・出どころ・実際・room・注記・リサイズの記録を出す。
- 描画: `fit` では 1 ゲスト画素 = 1 CSS 画素(スケール厳密に 1)、`fit-device` では 1 ゲスト画素 = 1 デバイス画素(スケール = 厳密に 1/dpr、窓の端は 1/dpr の倍数に丸める: `cssBox(..., snap)`)。以前の丸めは常に整数 CSS 画素で、dpr 1.7647 では窓が 1 デバイス画素ずれうる。エンジンが頼んだ寸法を取らなかった・room が最小より小さいときは room に合わせて両方向に拡縮(最大値を超えるときは自然スケールのまま小さく表示)。
- レイアウト(`style.css`、`body.screen-fit`): ヘッダーは 1 行・高さ 36 px 固定(長い説明は省略表示)、`main` は残りの高さ全部、右の `aside`(モニターパネル)は 340 px(`?panel=0` またはヘッダーの `Monitor` ボタンで閉じる)、`#stage` が room = デスクトップ。ツールバー・エンジン状態・ポインタ行は `aside` の先頭(`#display-section`)へ移し、エラー・終了表示・リサイズ通知は room の上に重ねる。ページ全体はスクロールしない。幅 900 px 以下は従来の `@media` どおりパネルを下(高さ 30vh)へ回す。`legacy` は従来のレイアウトと 1920x1080。
- 入力: `clientToScreen` はデスクトップ矩形とゲスト画面の比で画素を決める(`attachDesktopInput` の `screen` オプション)。表示変更は人の入力ではない: 入力として数えず、遅延の標本にもしない(S-screen1-20)。

### 3.3 実行中のリサイズ追従(`?follow=live`、opt-in)

`fle_input.c` に `FLE_IN_DISPLAY`(host queue の 7 番目の種類)。ページの room が 300 ms 静止すると新しいゲスト画面を Worker 経由で(エンジンが idle のとき入力と同じ道で)渡し、GUI スレッド(thread 0)が `GetMessage`/`PeekMessage` でそれを取り出したとき: (1) 画面を新しい寸法にし、(2) 全ての最上位窓へ `WM_DISPLAYCHANGE`(wParam = 32、lParam = LOWORD 幅/HIWORD 高さ)、続けて `WM_SETTINGCHANGE`(wParam = SPI_SETWORKAREA 0x2F、lParam 0)を **SendMessage**(z 順の先頭から)。`DefWindowProcW` はこの 2 つに 0 を返す(以前は『default processing 未実装』で停止した)。

出典(Microsoft Learn、2026-11-04 取得): WM_DISPLAYCHANGE「sent to all windows when the display resolution has changed」「wParam: the new image depth of the display, in bits per pixel」「lParam: low-order word horizontal resolution, high-order word vertical」「This message is only sent to top-level windows. For all other windows it is posted.」/ WM_SETTINGCHANGE「sent to all top-level windows when the SystemParametersInfo function changes a system-wide setting」「wParam ... the value of the uiAction parameter」「some applications send this message with lParam set to NULL」「If you process this message, return zero」。`SPI_SETWORKAREA` の数値 0x2F は SDK ヘッダ(Learn のページは名前だけ)= 非公式。

Learn が書いていないので**出典なしの選択**: 子窓へは post しない(S-screen1-8)、DefWindowProc の既定は 0 で子へ転送しない(S-screen1-10)、GUI スレッドは thread 0(全窓の owner が 0: S-screen1-11)、2 つのメッセージの順序(S-screen1-9)。

**FL の反応**(node、画面 2400x1300 で起動し 1800x1000 → 2400x1300 → 2400x1300 の display 変更):
- メッセージだけ(既定): FL は `GetMonitorInfoW` を読み直し、中のパネルを組み直すが、**主窓は動かない**(-5,-6,2410x1312 のまま。主窓は `zoomed` で、FL は `SetWindowPos(NOMOVE|NOSIZE)` を呼ぶだけ。FL は『最大化された窓は OS が新しい作業領域へ置き直す』ことを当てにしていると読む)。
- `FLE_SCREEN_REMAXIMIZE=1`(opt-in): `fle_win_input_remaximize` が、`zoomed` の最上位窓の位置はそのまま、大きさを『画面の変化量』だけ増減する(FL が自分で付けた外側 (−5,−6,+10,+12) を保つ)。FL の `WM_SIZE` 等は通常の `apply_pos` で走る。主窓は 2410x1312 → 1810x1012 → 2410x1312(停止なし)。**この再配置は『Windows は最大化された窓を表示変更時に置き直す』という私の推測**(Learn のページは未確認)。窓のサイズ変更をホストが勝手に行うことの忠実性の問題は 5 節。

### 3.4 ヘッダーの『表示』選択、リサイズ通知、ビーコン(PM の決定 2026-11-04)

- ヘッダーに小さな要素 `#display-box`(`<span>`、`<h1>` の直後・`#bundle-info` の前。PM の指示は『`#bundle-info` の直後』だったが、PM のワークツリーにある aud3 の `#audio-status` が同じ場所に入り、そこへの挿入では `patch` が当たらなくなるので、HEAD にも HEAD + aud3 のワークツリーにも当たる `<h1>` の直後に移した。見た目の差は『表示』の欄がタイトルの隣、`#bundle-info` がその右になるだけ): 『表示 [標準 | 高精細]』と `Monitor` ボタン。**標準 = `fit`、高精細 = `fit-device&dpi=auto`**。選択は `localStorage['fle.display']` に保存し、**変更すると再読み込み**する(ゲストの画面寸法と DPI は起動時に決まるため。『再読み込みします…』と出す)。`?screen=` / `?dpi=` が URL にあるときは URL が勝ち、選択は無効(鍵)になる。`localStorage` が使えないときは `?screen=` / `?dpi=` を付けた URL へ移る。aud3 がヘッダーに音の状態表示を足す位置とは別の要素・別の行(MERGE-NOTES に位置)。
- リサイズ通知(`#resize-note`、room の右下に重ねる): `?follow=reload`(既定)で room が変わり、新しい room では別のゲスト画面になるときに『表示サイズが変わりました。再読み込みで反映します。[再読み込み]』を出す。room が元の寸法に戻れば消える。
- ビーコン(version 3 のまま、**加算の欄** `guestScreen`): `{ mode: fit|fit-device|fixed|legacy, source: url|stored|default, width, height, reported, dpi, roomWidth, roomHeight, dpr, follow }`。`shared/perf.ts` の `beaconProblems` が項目ごとに検査する(欄が無い beacon は従来どおり問題なし)。`PanelView`・`perf-log-summary` には足していない(最小の追加)。


## 4. 実測: 画面が room に一致した証拠

隔離 Chrome 156 + 本番ビルドの `vite preview`(`dist-C` = HEAD 6f5801c + 差分。0a3ab9a への載せ替えは dd1 の既定が変わるだけで、画面の寸法の経路は同じ。載せ替え後の再実行は Firefox の run と e2e のシナリオ)+ 実エンジン(バンドル A = F2xH2 + 今回の runtime)。`tables/chrome-evidence.tsv`、スクリーンショットは `shots/f-*/shot-1.png` と `shot-resize-*.png`(私が画像を開いて確認したのは f-c4・f-c5・f-r1・f-r2。同じ構成の開発サーバー版〔本番ビルドの前の run〕では c1・c6・n1・c0 も開いた)。

| viewport CSS px | 付けた URL パラメータ | room(`#stage`) | エンジンの報告(ゲストの画面) | 主窓 (x,y,w x h) | desktop CSS px | ページの scrollW×H = clientW×H | ポインタ位置の一致 |
|---|---|---|---|---|---|---|---|
| 2176x1092 (dpr 1.7647) | `screen=legacy` | 1788x930 | 1920x1080 | (-5,-6,1930x1092) | 1088.00x612.00 | 2176x1092 = 2176x1092 | 2/2 |
| 2400x1300 (dpr 1) | (なし = 既定 `fit`) | 2060x1264 | 2060x1264 | (-5,-6,2070x1276) | 2060.00x1264.00 | 2400x1300 = 2400x1300 | 2/2 |
| 1400x900 (dpr 1) | (なし = 既定 `fit`) | 1060x864 | 1060x864 | (-5,-6,1070x876) | 1060.00x864.00 | 1400x900 = 1400x900 | 2/2 |
| 2176x1092 (dpr 1.7647) | `screen=fit` | 1836x1056 | 1836x1056 | (-5,-6,1846x1068) | 1836.00x1056.00 | 2176x1092 = 2176x1092 | 2/2 |
| 2176x1092 (dpr 1.7647) | `screen=fit-device` | 1836x1056 | 3239x1863 | (-5,-6,3249x1875) | 1835.44x1055.70 | 2176x1092 = 2176x1092 | 2/2 |
| 2176x1092 (dpr 1.7647) | `screen=fit-device&dpi=auto` | 1836x1056 | 3239x1863 | (-5,-6,3249x1875) | 1835.44x1055.70 | 2176x1092 = 2176x1092 | 2/2 |
| 800x900 (dpr 1) | (なし = 既定 `fit`) | 800x594 | 800x594 | (-5,-6,810x606) | 800.00x594.00 | 800x900 = 800x900 | 2/2 |
| 800x900 (dpr 1) | `panel=0` | 800x864 | 800x864 | (-5,-6,810x876) | 800.00x864.00 | 800x900 = 800x900 | 2/2 |
| 2400x1300 (dpr 1) | `panel=0` | 2400x1264 | 2400x1264 | (-5,-6,2410x1276) | 2400.00x1264.00 | 2400x1300 = 2400x1300 | 2/2 |
| 2176x1092 (dpr 1.7647) | `screen=fit-device&dpi=auto&panel=0` | 2176x1056 | 3839x1863 | (-5,-6,3849x1875) | 2175.44x1055.70 | 2176x1092 = 2176x1092 | 2/2 |


- どの行も **エンジンの報告(`fle_av_poll` の screen_w/h)= ページが要求した寸法**(`window.__app.screen.reported` = `asked`)で、主窓は FL 自身が `(-5,-6,W+10,H+12)` に置いた。`document.documentElement` の `scrollWidth/Height` = `clientWidth/Height`(全行)。ポインタ位置の検査は各 run 2 点(中央と右下の隅)で、表の 10 行(20 点)と §5 のリサイズの 3 run(6 点)の全 26 点でエンジンの `GetCursorPos` = 期待のゲスト画素。
- `fit-device` の desktop CSS 幅が整数でないのは、ゲスト画素 3239 を厳密に 1/1.7647 CSS px で描くため(1 ゲスト画素 = 1 デバイス画素。窓の端も 1/dpr の倍数に丸める)。room との差は 1 デバイス画素未満。
- 狭い画面(幅 800 < 900): パネルは下(高さ 270 px = 30vh)に回り、ステージは幅いっぱい・残りの高さ(594)。`?panel=0` では 864。ゲストの画面 800x594 は FL の最小サイズ(§9 S-screen1-1 の 640x480)以上。
- `legacy` の desktop は 1088x612 CSS px、従来どおり(`.screen-fit` 無し)。

### 4.1 レイアウト

規則(`style.css`、`body.screen-fit` = 無指定または `?screen=fit|fit-device|<W>x<H>`。`legacy` と `?scale=` だけの URL は従来のレイアウト):

| 領域 | 寸法 | 備考 |
|---|---|---|
| ページ | ビューポートいっぱい(`height: 100vh`、`overflow: hidden`)。スクロールバーなし | 実測: 全 13 通りで `scrollWidth/Height` = `clientWidth/Height` |
| ヘッダー | 高さ 36 px 固定・1 行・幅いっぱい | 長い `#bundle-info` は省略表示(`text-overflow: ellipsis`)。STAND-IN のバナーはその上に付く |
| モニターパネル(`aside`) | 右の 340 px(従来と同じ) | `?panel=0`・ヘッダーの `Monitor` ボタンで 0 px(閉じる) |
| 幅 900 px 以下 | パネルを下に回す(高さ 30vh、`@media (max-width: 900px)` は従来の規則を保つ) | 実測 800x900: ステージ 800x594 + パネル 270 px |
| ステージ(room) | 残りの全部 = デスクトップ | ツールバー・エンジン状態・ポインタ行は `aside` の先頭へ。エラー・終了表示・リサイズ通知は room の上に重ねる |

実測(§4 の表)で、ステージ = ビューポート − ヘッダー(36 px) − パネル(340 px または 0 / 下の 270 px)に一致する: 2400x1300 → 2060x1264、1400x900 → 1060x864、2176x1092 → 1836x1056、800x900 → 800x594(パネルが下)/ 800x864(`panel=0`)、`panel=0` の 2400x1300 → 2400x1264、2176x1092 の `panel=0` → 2176x1056(幅が パネル分だけ広がる)。e2e(スタンドインのエンジン、dev・preview・worker の 3 ターゲット)に同じ規則を固定するシナリオを足した: `screen-default`(1400x900 の room = ビューポート − バナー − ヘッダー − パネル、スクロールなし)、`screen-legacy`(従来のレイアウト、1920x1080、パネルのボタンなし)、`screen-panel0`、`screen-narrow`(800x900: パネルが下)、`screen-fixed`(`1280x720` が 16:9 のまま room に収まる)、`screen-choice`(ヘッダーの選択の保存・URL の優先・変更で再読み込み)、不正値の 5 通り(`screen-bad-*`、`dpi-bad-*`: エラーを出し、ダウンロードしない)。


### 4.2 Firefox 158(Linux、headless)

Firefox 158.0(`/usr/bin/firefox`、headless、geckodriver の WebDriver、pref `layout.css.devPixelsPerPx` で dpr を設定)、同じ本番ビルド(`dist-D`)・同じバンドル A。`test/e2e/screen-run-ff.mjs`、`shots/ff*/result.json`。**依頼者の Firefox 159 / Windows ではない**(Linux、headless、実ディスプレイ・実マウス無し)。

| window(外寸)・dpr | URL | 内寸 | room(`#stage`) | エンジンの報告(ゲストの画面) | 主窓 | 最初の idle | ページのスクロール |
|---|---|---|---|---|---|---|---|
| 2176x1224・1.7647 | (なし = `fit`) | 2176x1139 | 1836x1103 | 1836x1103 | (-5,-6,1846x1115) | 14.1 s | なし(2176x1139 = 内寸) |
| 2176x1224・1.7647 | `screen=fit-device&dpi=auto` | 2176x1139 | 1836x1103 | **3240x1946** | (-5,-6,3250x1958) | 28.7 s(load 高) | なし |
| 1400x900・1 | (なし) | 1400x814 | 1060x778 | 1060x778 | (-5,-6,1070x790) | 15.9 s | なし |

- Firefox でも、エンジンの画面 = room(`fit`)または room × dpr(`fit-device`、3240 = floor(1836 × 1.7647))で、ページはスクロールしない。`crossOriginIsolated` は true。`fit-device&dpi=auto` の画面でも FL の UI は拡大され(ウィザード 1529x1003)、停止はなかった(スクリーンショットを開いて確認した)。
- 窓の外寸 1224 に対して内寸が 1139(ブラウザのツールバー分。headless でも差が出る)なので、依頼者の viewport 2176x1092 とは高さが違う。`devicePixelRatio` は Firefox が 1.7647058823529411 を返した。
- 最初の idle は load が高い間の値(14〜29 s)で、P1 の値ではない。
- 未確認: Windows の Firefox、実マウス・実ディスプレイ、Firefox 159、WebGL を使う経路。


## 5. 実行中のリサイズ追従の比較と推奨

| 案 | 内容 | 実測 | 忠実性 |
|---|---|---|---|
| (a) 再読み込みで反映(`?follow=reload`、**既定**) | room が変わったら、今の画面を room に合わせて拡縮して見せ、右下に『表示サイズが変わりました。再読み込みで反映します。[再読み込み]』(`#resize-note`)を出す | ゲストは触らない | 完全(ゲストに何も起きない) |
| (b) メッセージだけ(`?follow=live`、runtime の `FLE_SCREEN_REMAXIMIZE` 無し) | `WM_DISPLAYCHANGE`/`WM_SETTINGCHANGE` を送る(Learn が文書化した通知) | FL は `GetMonitorInfoW` を読み直しパネルを組み直すが、主窓は動かない(node) | 高い(Windows が送る通知だけ) |
| (c) メッセージ + 最大化窓の置き直し(`?follow=live`)| (b) の後、`zoomed` の最上位窓を画面の変化量だけ広げる/縮める | node と Chrome で FL の主窓・パネルが新しい画面に収まる(下 5.1)。ウィザードは動かさない(Windows も動かさないと読む) | 『最大化された窓は OS が置き直す』は私の推測(S-screen1-12)。ホストが窓を動かすのではなく『ゲストの `SetWindowPos` 相当をランタイムが OS の代わりに行う』形で、最大化の印(`ShowWindow(SW_SHOWMAXIMIZED)`)を付けた窓だけに限る |

**推奨**: 既定は (a)。(c) は FL の主窓が正しく追従するので opt-in `?follow=live` で提供し、PM が『実機の FL は表示変更で主窓が置き直される』ことを何かの形で確認できたら既定にする(確認の手段: Windows 上の FL でモニタ解像度を変えるか、実機の `WM_GETMINMAXINFO`/`WM_WINDOWPOSCHANGING` のログ)。(b) だけでは主窓が動かないので機能にならない。

### 5.1 Chrome での実測(`shots/f-r1-*`、`shots/f-r2-*`、`shots/f-r3-*`)

viewport 2400x1300 (dpr 1) → 1500x900(`Emulation.setDeviceMetricsOverride`)。`tables/chrome-evidence.tsv`、スクリーンショット `shots/f-r1-*/shot-resize-later.png`、`shots/f-r2-*/shot-resize-later.png`(開いて確認した)。

| 設定 | リサイズの 1.5 秒後 | 10 秒後 |
|---|---|---|
| `follow=live` | ゲスト 2060x1264 → **1160x864**(エンジンの報告も同じ)、主窓 (-5,-6,2070x1276) → **(-5,-6,1170x876)**、desktop 1160x864 CSS | 同じ(安定) |
| `follow=reload`(既定) | ゲストは 2060x1264 のまま、desktop は room に合わせて 1160x712 に縮小表示、右下に通知 | 同じ |
| `follow=live`、`fit-device&dpi=auto`、2176x1092 (dpr 1.7647) → 1600x900 | **まだ 3239x1863**(要求は 2223x1524 を送信済み) | **2223x1524**、主窓 (-5,-6,2233x1536) |

`live` で FL の中のパネル(チャンネルラック、プレイリスト、ミキサー)は新しい画面に組み直された(窓がはみ出す部分はそのまま。ウェルカムウィザードは動かない = Windows も動かさないと読む)。`fit-device` ではエンジンが 3 倍の画素の組み直しを終えるのに 1.5 秒より長くかかる(正確な時間は測っていない)。ブラウザの窓を連続して動かしたときの動作(300 ms の静止後に 1 回送る)は単体テスト(`RoomDebouncer`)だけで、実ブラウザの連続リサイズでは測っていない。

## 6. DPI と鮮明さ

依頼者の devicePixelRatio は 1.7647(非整数)。3 案を実測と文書で比べた。

| 案 | ゲスト画素 | canvas | FL の UI | 鮮明さ | コスト(7 節) |
|---|---|---|---|---|---|
| (1) `?screen=fit` | = CSS 画素(1836x1056) | CSS 画素 1:1 → ブラウザが 1.7647 倍に拡大 | 『画面が 1836 画素の FL』の大きさ(= 従来の 100% の FL) | 拡大でぼやける(`smoothing` の bilinear/nearest) | 従来と同じ(画素数 1.94 Mpx) |
| (2) `?screen=fit-device` | = 物理画素(3239x1863) | 物理画素 1:1(スケール厳密に 1/dpr) | **1.7647 倍小さい**(FL は 96 DPI のまま) | 鮮明 | 画素数 6.03 Mpx(約 2.9 倍) |
| (3) `?screen=fit-device&dpi=auto` | = 物理画素 | 物理画素 1:1 | FL 自身が拡大(下) | 鮮明 | (2) と同じ画素数 |

**FL は DPI を見て自分の UI を拡大する**(実測)。`FLE_SCREEN_DPI=169`(GetDeviceCaps LOGPIXELS、`GetDpiForMonitor`、`GetProcessDpiAwareness` = PER_MONITOR)で、ウェルカムウィザードが 874x570 → 1529x1003(×1.749、×1.760)、主窓のツールバー・トラック行・ミキサーも大きく描かれた(`shots/dpi/`、`shots/c6-*`)。停止はなかった。

出典: FL は 2 つの実行ファイルで出荷されている(`docs/investigations/host-engine-boundary.md`: `FL64.exe` は manifest に `dpiAware True/PM`、`FL64 (Scaled).exe` は `dpiAware` なし = システム DPI のまま OS が拡大)。つまり (1) は `FL64 (Scaled).exe`(OS がビットマップ拡大)、(3) は 176% の画面での `FL64.exe`(自分で拡大)と同じ動作を、ブラウザの上で再現している。決定 0099 が `GetProcessDpiAwareness` を UNAWARE にした理由は『ホスト exe の DPI 宣言が分からない』で、撤回条件に『ホスト exe の宣言が分かったとき』があった。決定 0171 の ia1 が宣言(`True/PM`)を記録したが、PM は『起動経路が変わる』ので UNAWARE を維持した(S-ia1-dpi)。この決定は**既定を変えない**: `FLE_SCREEN_DPI` が 96 以下(未設定)なら従来どおり UNAWARE・96。

推奨: 既定は (1) `fit`(PM の決定。費用ゼロ)。DPR > 1 の画面では、ヘッダーの『表示』で 高精細(= (3))へ切り替えられる(PM の決定: 選択は保存、変更時は再読み込み)。(2) は FL の UI が小さくなるだけなので推奨しない。DPR が 1 の画面では (1) と (3) は同じ。**(3) を既定にするには**、`GetProcessDpiAwareness` = PER_MONITOR が起動経路(FL の per-monitor の分岐: `SetThreadDpiAwarenessContext` 等)を変える影響の確認が要る: 今回の観測は Basic の起動・ウィザード・Enter・再生 20 秒で停止なしだが、Sampler・Settings・File→Open・FLP は DPI 付きで未確認。

DPI の動的な変更(ブラウザのズームで DPR が変わる)は未実装: ページは `matchMedia(resolution)` で再描画するだけで、エンジンに `WM_DPICHANGED` を送らない。FL が DPI 変更で再レイアウトするか(= `WM_DPICHANGED` の処理)は**未測定**(`FLE_SCREEN_DPI` は起動時にだけ読む)。ズームで DPR が変わっても『表示』の選択が高精細ならゲスト画面(デバイス画素の room)は変わらないので、DPI が古いまま残る。再読み込みで反映される。


## 7. 画素数が増えたときの費用(実測)

条件: 隔離 headless Chrome 156(SwiftShader)、**viewport 2176x1092・dpr 1.7647**(依頼者の Firefox の寸法に合わせた。ブラウザは Chrome)、本番ビルド(`vite preview`)、実エンジン(F2xH2、wasm-opt なし 174 MB の web link)、**この機械は load 28〜55**(他担当の負荷)、各設定 3 回の中央値。値は CPU 秒/秒(1.0 = 1 コア。`/proc` の utime+stime をページ主スレッド・エンジンの Worker・全プロセスごと)。『アイドル』= ウィザードを Enter で閉じて 8 秒後の 20 秒、『再生』= Space で再生を始めて 5 秒後の 20 秒(空のプロジェクト)。**F8(書き出し)は測っていない**。設定: `legacy` = 1920x1080(2.07 Mpx)、`fit` = 1836x1056(1.94 Mpx)、`fit-device`/`fit-device&dpi=auto` = 3239x1863(6.03 Mpx)。

### 7.1 `?dedupe` 既定 ON(決定 0201、HEAD 0a3ab9a 以降の既定)での費用 — **こちらが今後の値**

`tables/cost-dedupe-default.tsv`(アプリは HEAD 0a3ab9a + 本差分。`dedupe` のパラメータは付けていない = 既定)。

| 設定 | 区間 | n | ゲスト画面 | Mpx | ページ主スレッド | Worker | 全プロセス | 窓の更新 /s | wasm MB |
|---|---|---|---|---|---|---|---|---|---|
| fit | idle | 3 | 1836x1056 | 1.94 | 0.040 | 0.279 | 0.364 | 1.0 | 932 |
| fit | playing | 3 | 1836x1056 | 1.94 | 0.155 | 0.424 | 0.698 | 52.8 | 932 |
| fitdev-dpi | idle | 3 | 3239x1863 | 6.03 | 0.054 | 0.286 | 0.390 | 1.0 | 1030 |
| fitdev-dpi | playing | 3 | 3239x1863 | 6.03 | 0.355 | 0.655 | 1.202 | 58.2 | 1030 |
| legacy | idle | 3 | 1920x1080 | 2.07 | 0.026 | 0.288 | 0.345 | 0.8 | 932 |
| legacy | playing | 3 | 1920x1080 | 2.07 | 0.167 | 0.480 | 0.790 | 56.0 | 932 |


### 7.2 `?dedupe` が無かった(= 全矩形を毎フレーム送る)ときの費用(参考。HEAD 0a3ab9a より前の既定)

`tables/cost.tsv`(アプリは HEAD e7af37b + 差分)。

| 設定 | 区間 | n | ゲスト画面 | Mpx | ページ主スレッド | Worker | 全プロセス | 窓の更新 /s | wasm MB |
|---|---|---|---|---|---|---|---|---|---|
| fit | idle | 3 | 1836x1056 | 1.94 | 0.140 | 0.285 | 0.540 | 62.5 | 932 |
| fit | playing | 3 | 1836x1056 | 1.94 | 0.166 | 0.418 | 0.710 | 62.5 | 932 |
| fitdev | idle | 2 | 3239x1863 | 6.03 | 0.263 | 0.282 | 0.653 | 62.2 | 1034 |
| fitdev | playing | 2 | 3239x1863 | 6.03 | 0.326 | 0.497 | 0.985 | 61.2 | 1034 |
| fitdev-dpi | idle | 3 | 3239x1863 | 6.03 | 0.267 | 0.291 | 0.679 | 62.5 | 1030 |
| fitdev-dpi | playing | 3 | 3239x1863 | 6.03 | 0.358 | 0.621 | 1.154 | 61.9 | 1030 |
| legacy | idle | 3 | 1920x1080 | 2.07 | 0.152 | 0.292 | 0.539 | 62.5 | 932 |
| legacy | playing | 3 | 1920x1080 | 2.07 | 0.167 | 0.407 | 0.708 | 61.9 | 932 |


### 7.3 読み

- **`fit`(1.94 Mpx)は従来(2.07 Mpx)と同じ費用**(どちらの表でも主スレッド・Worker・全体に差は無い: 画面を room いっぱいにすること自体は無料)。
- **`fit-device&dpi=auto`(約 2.9 倍の画素)の費用は『再生中』に出る**。`?dedupe` 既定 ON では、アイドルはほぼ変わらない(主スレッド 0.026 → 0.054、Worker 0.29 のまま、全体 0.35 → 0.39 コア。窓の更新が 1 /s に減るため)が、**再生では 主スレッド 0.167 → 0.355(×2.1)、Worker 0.48 → 0.65(×1.4)、全体 0.79 → 1.20 コア(×1.5)**、wasm のメモリ +98 MB(932 → 1030 MB)。dedupe 無しでは アイドルの主スレッドも 0.15 → 0.27(約 +75%)。
- 再生中は 56〜58 窓更新/s で、更新される矩形が大きい(再生ヘッド・メーター・ミキサー)ので、ページが `putImageData` する画素数に比例して主スレッドが増える(読み)。ページ内の `putImageData` 単体(主窓の全面、40 回の平均、エンジンが動いている最中の 1 回ずつ、`shots/f-c5-*/result.json` の `benchPut` と最初の run): 1930x1092(2.11 Mpx)で 0.39〜0.94 ms、1846x1068(1.97 Mpx)で 0.58〜0.60 ms、3249x1875(6.09 Mpx)で 4.6〜6.1 ms(画素 2.9 倍に対し 6〜12 倍。呼び出しの後始末は次のフレームで払われ、40 回の直後のフレームまでの時間は 0.3〜1.1 / 18〜24 / 73〜92 ms)。2 回の測定の差が大きい(load と同居するエンジンのため): 目安にすぎない。
- Worker の増加(再生 +36%)は、エンジンが 2.9 倍の画素を描く(Blend2D、面の差分と BGRX→RGBA 変換)ため(読み)。アイドルの Worker は同じ。
- この機械は SwiftShader(GPU なし)で、依頼者の機械(GTX 980)とは違う。wasm-opt 版・出荷の hybrid 版では Worker の絶対値が下がる(決定 0189)ので、比は目安。


## 8. テストと基準(HEAD 62ac3e9 + 本差分 / 一部は e7af37b・0156065 + 本差分。各行に木を書いた)

各行に『どの木で測ったか』を書く。木の名前: **src**(HEAD e7af37b + 本差分の runtime。実エンジンのバンドル A はこの runtime で作った)、**src3**(HEAD 0a3ab9a + 本差分)、**src4**(HEAD 55e4e83 + 本差分)、**src5**(HEAD a0d658a + 本差分)、**src6**(HEAD 0156065 + 本差分)、**src7**(HEAD **62ac3e9** + 本差分 = 最終の `screen1.diff` を当てた木)。HEAD 0156065 → 62ac3e9 で `lift/runtime/`・`lift/crates/*/src`・a4 の道具(`lift/harness/a4*`、`two/`、`seeds/`、`hooks/`)は 1 行も変わっていない(`git diff --stat 0156065 62ac3e9 -- …` が空)。変わった `lift/` は `lift/harness/web/opfs1/`(新規)とテスト `wall_opfs1_map.rs`(新規)だけなので、src6 で走らせた a4 は src7 にそのまま当たる(src7 で走らせ直してはいない)。

| 項目 | 木 | 結果(実測) | 基準 |
|---|---|---|---|
| Rust 全体テスト(`harness/test.sh full --fresh`、`screen1.rs` 無し) | src | 178 results / 2,274 passed / 0 failed / SKIPPED 1(1,284 s) | PM の基準 178 / 2,274 / 0 |
| 新規 `screen1.rs`(6 件: 既定値・env・DPI・不正値・実行中の変更・ヘッダー) | src | 6 passed / 0 failed | 新規 |
| Rust 全体テスト(`harness/test.sh full --fresh --jobs 4 --threads 2 --no-record`) | src4 | **184 results / 2,313 passed / 0 failed** / SKIPPED 1(180 実行ファイル + doc-tests 4 を全部実行、cached=0、wall 557 s。`screen1` は 6 passed) | HEAD 55e4e83 の PM 測定 183 results / 2,307 passed / 0 failed に `screen1.rs` の 6 件を足した値(184 / 2,313 / 0)が期待値 |
| Rust 全体テスト(同じ引数) | src6 | **185 results / 2,320 passed / 0 failed** / SKIPPED 1(181 実行ファイル + doc-tests 4 を全部実行、cached=0、wall 511 s) | PM の 0156065 の測定(コミットメッセージ)184 / 2,314 / 0 に `screen1.rs` の 6 件を足した値 = 185 / 2,320 / 0 と一致 |
| Rust 全体テスト(同じ引数) | **src7**(最終) | **186 results / 2,321 passed / 0 failed** / SKIPPED 1(182 実行ファイル + doc-tests 4 を全部実行、cached=0、wall 502 s。`screen1` は 6 passed、`wall_opfs1_map` は 1 passed) | src6 の値 + `wall_opfs1_map.rs`(62ac3e9 で増えた 1 実行ファイル・1 テスト)= 186 / 2,321 / 0 と一致 |
| a4 FRUITY 無効(基準の `harness/a4-fruity.sh off`) | src | 942 / 942、895 / 895、functions 5,310、entries 1,009,731、exit 0 | PM の基準と一致 |
| a4 FRUITY 無効(同じ `a4-fruity.sh off`) | **src6**(src7 と runtime・道具が同一) | 942 / 942、895 / 895、functions 5,310、entries 1,009,731、exit 0(gen 64 s・build 488 s) | 同上 |
| (参考。基準ではない)`a4-multi.sh` に自前でモジュール列を渡した版 | src4 | **272 / 895 で停止**(`VirtualProtect: address is not in a VirtualAlloc region`、functions 4,209、entries 852,104、exit 134)。act1 の MERGE-NOTES が『ia2/dec1 の `a4off.sh` も 315/942・272/895 で止まる。基準の構成ではない』と書いているのと同じ停止。**私が最初に基準でない構成を走らせた**(自己申告)。ログ `a4src4-run3-own-module-list-NOT-THE-REFERENCE.txt` | — |
| FLEngine の既定の生成 C の sha256 | src | `cd9107cef4cbef14f154645749957b7067be7ed7c2b93f5bbe1f72fd47029e50`(215,811,678 B) | 不変(`fe-lift` のクレートは本差分で 1 行も変えていない。src4 の a4 の gen 段〔`a4-multi.sh`〕が同じ `cd9107ce…` を出したのも実測。`logs/a4src4-run1-sha-mismatch.txt`) |
| 16 スナップ(node、既定の env、バンドル A) | src のバンドル | `g3-lines` 84 行のうち先頭 82 行が `harness/b2d2/expected/g3-lines-SHIP2-baseline.txt` と**バイト同一**(残りの 2 行は script 終了時の `SIGABRT`/`the host ended the wait`〔期待表に入らない通常の終わり方。`rc=134`〕)。ship2 の `A-F2xH2-node` の 84 行とも同一 | 不変 |
| 16 スナップ(0156065 の present2 の期待表 `g3-lines-present2-runtime.txt`・`g3-lines-present2-u1.txt`) | — | **未実施**。0156065 の runtime で作ったバンドル(ship3 相当)が要る。私の A は ship2 の obj 木に e7af37b 時点の runtime の 10 ファイルを再コンパイルしたもので、present2 の変更を含まない。比較は PM が ship3 を作るときに `g3-lines-present2-u1.txt`(最新)と行う(本差分は env 無しで挙動を変えない: `fle_screen.h` の既定値に `_Static_assert`) | — |
| app 型検査(4 つの tsconfig) | **src7**(最終) | rc=0(1 回目は `tsc: not found` rc=127 で、原因は調べていない〔共有の `app/node_modules` を参照している最中。推測: 他者が入れ替え中〕。再実行で rc=0。`logs/typecheck7.txt` は再実行の分) | rc=0 |
| app の vitest | **src7**(最終) | 64 files passed + 1 skipped / **1,463 passed + 7 skipped**(62ac3e9 の素の木: 1,437 + 7 skipped〔aud3 のコミットメッセージ〕。新規 26 件。`logs/vitest7.txt`) | 失敗 0 |
| 変異検査(unit 層、自分のワークツリー `APP_WORK`) | **src7**(最終) | 14/14 検出(本差分の新規 13 件 + 既存の `screen-y-flipped` が名前に `screen-` を含むため一緒に走った)。全て `restored=true`、『diff of copy vs tree: identical』 | 全件検出 |
| e2e(Chrome 156、dev / preview / worker の 3 対象) | **src7**(最終) | **1,256 / 1,256**(失敗 0。PM の aud3 のコミットメッセージの 1,127 + 私のシナリオの 129 = 1,256 と一致。src3 の 1 回目は 1,234 / 1,236 で、失敗 2 は `dev/perf-beacon` の `rAF` の待ちの超過〔load 30〜55〕、単独の再実行は 46 / 46 で通過した。ログ `e2e.log`)。私のシナリオは 43 検査 × 3 対象 = 129 検査 | 失敗 0 |
| Worker のテスト | **src7**(最終) | 43 / 43(src3 の 1 回目は負荷で 1 件 timeout、`worker-tests-run1-timeout.log` に保存) | 失敗 0 |

- **app の検査の木**: 最終の src7(HEAD 62ac3e9 + 本差分)で型検査・vitest・e2e・Worker・変異検査を走らせた。途中の木(src5 = a0d658a + 本差分、src6 = 0156065 + 本差分)でも同じ結果だった: src5 は vitest 1,435 + 7 skipped・e2e 1,238 / 1,238・Worker 43 / 43・変異 14/14、src6 は同じ値(`#display-box` と `main.ts` のブロックを移した後)。
- `harness/testfast/deps.tsv` は更新していない(`harness/test.sh record` が要る)。`full` は全実行ファイルを走らせるので `screen1.rs` も走る。`affected`/`fast` に入れるなら record する。
- 実エンジンを走らせた測定(Chrome・Firefox・node・費用)は **src のバンドル A**(wasm-opt なし 174 MB の web link / node link)。**0156065 の runtime(present2 段階 1・1b、dec1、api2、act1)を含むバンドルでの実エンジンの測定は 0 件**。
- **PM のワークツリーに未コミットで入っていた aud3 との結合の確認**(0156065 の時点。aud3 は 0235dff で取り込まれた): 0156065 の `git archive` にワークツリーの変更済みファイル 19 個と untracked の app のファイルを重ねた木に `screen1.diff`(移す前の版は `app/index.html` と `app/src/main.ts` の 1 ハンクずつが当たらなかった。移した後の版)を当てると `patch -p1 --dry-run` rc=0(`app/index.html` の 1 ハンクだけ fuzz 2)。その木で型検査 rc=0、vitest 1,462 passed + 1 failed + 7 skipped(失敗の 1 件は `pack-assets-fruity.test.ts` の `/tmp` への書き込み失敗〔`UNKNOWN: unknown error, write`〕で、私の複製が共有の /tmp(tmpfs)を埋めていたため。空きを作ってそのファイルだけ再実行すると 9/9 通過)。この結合は 0235dff で HEAD に入ったので、最終の src7 で再度確認した(上の表)。


## 9. 出典なしの選択(S-screen1-…)と、他の決定との関係

`tables/s-choices.tsv`(21 件。代替案と撤回条件つき)。要点: 範囲 640x480〜7680x4320、環境変数名、不正値は無視、header-only + weak 変数(リンク行を変えない)、appview の予約語 2 つ(version 据え置き)、`rcWork = rcMonitor`、HORZSIZE の DPI 一般化、`WM_DISPLAYCHANGE` の送り先・順序・DefWindowProc の 0、GUI スレッド = thread 0、最大化窓の置き直しは**推測**、DPI > 96 で PER_MONITOR、room の静止時間 300 ms、`fit` 既定・ヘッダー 36 px・パネル 340 px。

- 決定 0065(仮想モニター)・0053(仮想画面 DC)・0057(GetSystemMetrics)・0121(HORZSIZE): 値の既定は不変(`_Static_assert` で既定値を検査)。『ページは値を渡さない』は変わった(opt-in の環境変数)。
- 決定 0099・0171(S-ia1-dpi): `GetProcessDpiAwareness` の UNAWARE は既定のまま。`FLE_SCREEN_DPI` > 96 のときだけ PER_MONITOR。0099 の撤回条件『ホスト exe の DPI 宣言が分かったとき』は ia1 で成立済みだが、PM の判断(起動経路を変えない)に従った。
- 決定 0182(`?hz=`)と同じ仕組み(`engine.env`)。決定 0201(`?dedupe` 既定 ON)の後の HEAD で費用を測り直した(§7)。

## 10. PM の未決の点と、見つかったこと

### PM の未決の点

1. **wasm の再ビルド(ship3)が要る**。今の既定の入口(SHIP2、`fl-engine-ship2-F2xH2`)の wasm は `FLE_MONITOR_W/H` を読まない。**実測**(`f-d1`、隔離 Chrome 156、viewport 2176x1092・dpr 1.7647、パラメータなし、本番ビルドのページ + 既定の manifest = SHIP2): ページは『エンジンが画面を報告しない』と注記し(`window.__app.screen.notes`、ページのログ)、ゲストは 1920x1080 のまま。ページは 1920x1080 を room(1836x1056 CSS px)に拡縮して描く(倍率 0.9563 CSS px/ゲスト画素 = 1.688 デバイス画素/ゲスト画素)ので、desktop は 1836x1033 CSS px と room をほぼ埋める(従来のページでは 1088x612)。ただし画面の広さは増えない(ゲストの画素は 1920x1080 のまま)ので、FL の UI は拡大されてぼやけ、エンジンが room いっぱいに UI を広げる結果にはならない。ポインタ位置は一致(2 点とも、エンジンの `GetCursorPos` = 期待のゲスト画素)。つまり、**ページだけ替えても『小さい』は直るが、『広い』は ship3 まで直らない**。runtime の差分は 10 ファイル + harness 2 ファイル(`fruity_idle.c` の `fle_g3_display`、`fruity_g3.c` の `display` コマンド)で、ビルド側の変更は無い(`RT_EXTRA_CFLAGS` 等も不要)。
2. **既定 = `fit`**(PM の決定どおり)。依頼者の機械(dpr 1.7647)では 1.76 倍に拡大されてぼやける(Windows の `FL64 (Scaled).exe` と同じ見え方)。ヘッダーの『表示』で 高精細 を選べる。**高精細を既定にするか**は §7 の費用(画素 2.9 倍、`?dedupe` 既定 ON のとき アイドルは 主スレッド 0.026 → 0.054、全体 0.35 → 0.39 コア、再生は 主スレッド 0.167 → 0.355、全体 0.79 → 1.20 コア(`legacy` → `fit-device&dpi=auto`、中央値))と、依頼者に両方を試してもらった結果で決める。
3. **`?follow=live` を既定にするか**: 現状は opt-in。条件は (a) 『最大化された窓は表示変更で OS が置き直す』の確認(私の推測。実機 Windows の FL で解像度を変えるか、`WM_GETMINMAXINFO` / `WM_WINDOWPOSCHANGING` を取る)、(b) 下の発見 2(所有された窓の z 順)の解消。
4. **DPI > 96 の起動経路**(`GetProcessDpiAwareness` = PER_MONITOR)の確認範囲を決める: 今回は Basic の起動・ウィザード・Enter・再生 20 秒で停止なし。Sampler、Settings、File→Open、FLP の読み込み、F8、プラグインは DPI 付きで未確認。
5. 出荷の wasm-opt 版・hybrid 版での費用の再測定(今回は wasm-opt なしの web link)。

### 見つかったこと(予想外、または確認が要る)

1. **FL は DPI を見て UI を拡大する**(実測)。決定 0099 の『UNAWARE なら 96 DPI と同じ』という見立ては、FL が per-monitor の分岐を持つので、DPI を渡すと描画が変わる。FL64.exe の manifest(`dpiAware True/PM`)と `FL64 (Scaled).exe` の 2 形態がそのまま `fit-device&dpi=auto` と `fit` に対応する。
2. **所有された窓の z 順が崩れる**(既存の runtime の穴。実測): FL は `WM_SETTINGCHANGE`/`WM_DISPLAYCHANGE` を受けると主窓に `SetWindowPos(… NOMOVE|NOSIZE)` を呼ぶ(`flags 0x3`: `SWP_NOZORDER` が無い = z 順の指定がある。挿入先の値は記録していない)。runtime の `apply_pos` は主窓を最前面に出すが、その窓に所有された窓(ウェルカムウィザード、owner = 主窓)を一緒に上げない。結果、ウィザードを開いたまま画面を変えると、ウィザードが主窓の後ろへ隠れる(node の `dyn-wiz-2`: `FLE_SCREEN_REMAXIMIZE` なしでも起きる。Chrome の `f-r1-*` のスクリーンショットでウィザードが見えない)。Learn(Window Features)は『所有された窓は常に所有者の上に出る』と書く。この決定では直していない(z 順は他の全ての窓の動作に効く)。ウィザードを閉じた後(通常の使用)は起きない。この観測は e7af37b 由来の runtime のもの。0156065 の `apply_pos` は present2(決定 0214 段階 1・1b)で変わっているので、同じ穴が今も同じ形であるかは**未確認**。
3. FL は作業領域を頻繁に読む(`SystemParametersInfoW(SPI_GETWORKAREA)` が起動後 4 仮想秒で 256 呼び出し以上)。画面寸法を動的にする場合、この経路が最初の変更点になる。
4. 主窓の初期サイズは、この run では読んだ画面から FL が計算していた(画面を変えると追従した)。FL が保存した位置・サイズを使うか・どこへ保存するかは調べていない(保存された窓の状態がエンジンの仮想ファイルに入ると、挙動が変わりうる: 未確認)。
5. 費用の測定器: `window.__app.perf.recorder` の `idles.total` はエンジンの idle の累計(約 1,050 /s アイドル、約 2,000 /s 再生)で、`window.__app.idles` の配列は 5,000 件で打ち切られる(PM の訂正どおり、ビーコンの 23.7 /s は上限の張り付き)。この決定の費用表は累計の方を使った。


---

## PM の決定と取り込み(2026-11-04)

- **取り込み**: `screen1.diff`(37 ファイル、`-` 行 30 本は置換のみで PM も全部読んだ)を HEAD f90b31f に当てた。clean(offset・fuzz なし)。FLEngine の既定の生成 C は `cd9107ce…`(215,811,678 B)のまま。PM 実測: 全体テスト 186 results / 2,321 passed / 0 failed(1 回目は 2,317 passed / 4 failed: noreturn_audit の変異テスト 4 件が /tmp の tmpfs 満杯〔Disk quota exceeded〕でコンパイルに失敗したため。noreturn_audit 単独の再実行は 8/8 通過。環境の問題)、a4 FRUITY 無効 942/942、895/895、関数 5,310、入口 1,009,731(不変)、型検査 rc=0、vitest 1,463 passed + 7 skipped(基準 1,437 + 7)、Worker 43/43、e2e 1,256/1,256(基準 1,127 + screen の 129)。
- **契約としての位置づけ**: ゲストの画面寸法(`GetSystemMetrics(SM_CXSCREEN)` ほか、モニターの矩形、DPI)は外部 API の返す値であり、『ページが測った表示領域をホストの環境として渡す』のは OS の画面の大きさを与えること。FL の挙動の代行ではない(FL の主窓の置き方はFL 自身が決める)。`FLE_MONITOR_W/H` の既定は従来の 1920x1080 のまま(env 無しでは 16 スナップが不変)。
- **既定**: ページは `?screen=fit`(ヘッダーと右パネルを除く全領域)を既定にする。`?screen=legacy` で従来の表示。高精細(`fit-device&dpi=auto`)は opt-in(ヘッダーの『表示 標準/高精細』)。`follow=live`(実行中の画面変更への追従)は opt-in のまま: 『所有された窓が主窓の後ろへ回る』穴(runtime の Z 順)が win2(決定 0222)の調査対象で、直った後に既定にするかを決める。DPI の既定は 96(UNAWARE。決定 0099、S-ia1-dpi)のまま。
- **今の既定の入口(SHIP2 の wasm)は `FLE_MONITOR_W/H` を読まない**ので、画面が広がるのは ship3 の再ビルド後。新しいページを旧 wasm に当てると、ページがゲストの 1920x1080 を room に拡縮するだけ(ぼやける)。
- **desk1(決定 0223)との関係**: 本差分は `desktop.ts` の配置(`cssBox`、`setScale`)に触れているが、desk1 がデスクトップ合成をランタイムに移すとページの窓配置コードは消える。screen1 の意味(room の測定、`FLE_MONITOR_W/H` の受け渡し、DPI、ヘッダーの切替、通知)は引き継ぐ。
- **未確認**: 実機 Windows / 依頼者の Firefox 159、WM_DPICHANGED、F8 の費用、wasm-opt 版・hybrid 版の費用、DPI > 96 での各操作、present2 後の Z 順、16 スナップ(present2 の期待表 `g3-lines-present2-u1.txt`)との比較。
