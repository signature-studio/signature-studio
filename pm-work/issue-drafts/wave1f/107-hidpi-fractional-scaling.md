---
draft: "107"
title: "Windows と同じ Fractional Scaling を再現し、FL が自分で UI を拡大して、操作できる大きさで鮮明に描く(拡大して表示する版は作らない)"
type: Task
milestone: "M2 HiDPI と P2 の測定(P2)"
parent: "005"
blocked_by: []
labels: ["hidpi", "fl-exe"]
size: XL
priority: Urgent
project: signature-studio/3
---

## 目的

依頼者の是正(2026-11-23)。要点は次のとおり。

- HiDPI の方針が完全に間違っている。物理画素に 1:1 で結びついており、全ての GUI が操作できないほど小さい。
- 描画の崩壊も起きている。
- 依頼者の追加の指示: 拡大されてぼやけた画面は即座に棄却する。Windows と同じ Fractional Scaling を正確に再現すること。
- 依頼者の追加の指示: FL のネイティブでは、Main GUI Scaling の System が OS の拡大率に自動で従う。今の版では従っていない。DPI・拡大率に関するメトリクス(FL 自身の Debug ログを含む)を、きちんと取ること。
- 依頼者の追加の指示: 特定の拡大率の数字を、予め用意しない。拡大率は、環境(DPR)から、Windows と同じ計算で導く。

この Issue の目標は、Windows で表示の拡大率(任意の値)を設定した時と同じ見え方にすること。FL 自身が、起動時の環境の値から拡大率を決め、UI を自分で描き直すこと。

1. 画面は物理画素の大きさで見える。guest の 1 画素は、表示の 1 物理画素にそのまま対応する(再標本化しない)。
2. 拡大率は、ページが起動時に engine へ渡す DPR(実数)から、Windows と同じ計算で導く。拡大率(%) = round(DPR × 100)。DPI = MulDiv(96, 拡大率(%), 100)。
3. システムは拡大も縮小もしない。アプリが自分で、部品・フォント・余白を拡大率に合わせて描く。
4. 結果として、UI の大きさは拡大率 100% の時と同じ見かけの大きさになり、かつ鮮明になる。

禁止: 論理の小さい画面を作り、DPR 倍に拡大して表示する実装(ぼやけた画面)は、中間の版としても作らない。この方式の成果物は依頼者が棄却する。受け入れが通らない時は、判定の段で止まり、push されない。通らなかった理由と到達点を報告に書く。

## 前回の試行で分かったこと

前回の試行(run 45061489176。約 4 時間 40 分)は、受け入れに届かず落ちた。作業は保存されている(artifact `agent-1347` の `work.diff`)。`failed` を外すと、次の run がその差分を作業ツリーに載せ、`.prior-work.md` で続きから行うよう指示する。判定を通らなかったので、commit はされていない。最初から作り直さない。

保存された差分に含まれるもの: `app/src/ui/`(`screen-fit.ts`・`desktop.ts`)・`app/src/main.ts`・`app/test/unit/screen-fit.test.ts`・`lift/runtime/`(`fle_screen.h`・`fle_gdc.h`・`fle_sysmetrics.c`・`fle_appfile.c`・`fle_proc.c`・`fle_proc.h`)・`lift/crates/fe-lift/tests/`(`gdc.rs`・`screen1.rs`・`wall_s.rs`)・`tools/feat-c1-hidpi/measure-size.mjs`・`pm-work/findings/hidpi-fl-scaling.md`。

できたこと(実測。engine の作り直し・単体テストの通過まで):

- guest の画面 = `round(部屋 × DPR)`。DPR 1・1.25・1.5・1.7647・2 で、canvas = 表示の物理画素。
- 拡大率と DPI は式で求める。`lift/runtime/fle_screen.h` の `fle_screen_scale_percent`(拡大率 = round(DPR × 100))と `fle_screen_dpi`(DPI = MulDiv(96, 拡大率, 100))。
- GetDeviceCaps・GetDpiForMonitor・font の高さ・アイコンとカーソルの寸法が、DPI に従う。
- `app/src/ui/screen-fit.ts`・`desktop.ts`・`main.ts` で、物理画素の画面にした(拡大して表示しない)。単体テスト 1333 件が通った。
- 特定の拡大率・DPI・DPR の値は、製品・seed・設定・テストに入れていない。

できていないこと:

- (a) DPI・画面の API の記録(起動の時点から有効にする仕組み)。
- (c) F10 の Display の節。(d) FL が採用した拡大率の実測。(e) Windows の値との突き合わせ。(g)〜(j) Debug ログ・前後の比較・他のタブ・`debugLog`。
- `dpi`・`flScalingPercent`・`parts`・`edgeWidth` は全 DPR で未測定(null)。`debugLog` は `captured: false`。
- ポインタの座標の変換(±1 画素)は未実施。崩壊(Browser の黒い領域・Mixer の位置)は未分析。`blackShare` は DPR 1〜2 で約 0.91 で同じ、DPR 1.87 で 0.976。
- 認識の値(per-monitor = 2)は推定。FL の実行ファイルのマニフェストは、作業の場所に無かったので読めていない。

新しい障害: DPI が 240 以上(DPR 2.5 以上)で、FL が起動しない。`[js] fle thread 2 failed: RangeError: Maximum call stack size exceeded`。

- 画面の大きさ・アイコンとカーソルの寸法・認識の値は原因ではない(実験 A・B。部屋 1000×600・DPR 2.5 の 2500×1500 の画面でも、DPI 240 で止まった)。
- DPI の値を使う経路(font の高さ `FLE_SPI_FONT_HEIGHT`・`GetDeviceCaps` の LOGPIXELS・`GetDpiForMonitor`)のどれかと推定する。未分離。DPI 216 は未確認。
- 乱数の DPR 3.00 で、idle に達しない。

環境変数: `FLE_MONITOR_DPR` を、前回の作業が新しく足した(下の「制約」を参照)。

## 起動用の実行ファイル(ラベル fl-exe)

この Issue には、ラベル `fl-exe` が付いている。そのため、作業の run では、作業ツリーの `lift/out-app/host-extra/exe/` に、起動用の実行ファイル(`FL64.exe`・`FL64 (scaled).exe`)が在る(commit はしない。git の対象外)。

- 両方のマニフェスト(PE の `RT_MANIFEST` の資源。`dpiAware`・`dpiAwareness`・`gdiScaling`・`longPathAware` など)を、node の PE の読みで取り出し、2 つの違いを表にする。
- **2 つの違いが、FL が DPI をどう扱うかの一次の情報**。`FL64 (scaled).exe` は、Windows が拡大する(DPI 非対応として扱う)ための起動用と推定する(未確認。マニフェストの違いで確かめる)。
- ゲストの「プロセス」の DPI の認識は、起動用の実行ファイルのマニフェストが決める。この製品は実行ファイルを実行せず FLEngine の DLL を読むので、runtime は、マニフェストの宣言どおりの認識を返す。
- 製品には入れない。読みの道具は `tools/` か、その場の `.tmp-*` の script にする(commit はしない)。

## 優先順位(前回の試行の後に定めた。この順に行う)

1. **最初に、メトリクスの取得を終える**(実装を増やす前に)。
   - マニフェストの読み(`FL64.exe`・`FL64 (scaled).exe`。上の「起動用の実行ファイル」)。
   - DPI・画面の API の記録(起動の時点から有効にする仕組み。(a) の対象の API と `GetDeviceCaps`)。
   - F10 の Display の節・Debug タブ・Info タブの取得(`debugLog`。(c)・(g)・(i))。
   - DPI を変えた起動での FL の拡大率の実測(`flScalingPercent`。(d))。
   - 部品の位置(`parts`)・縁の遷移の幅(`edgeWidth`)。
   - 測定の道具(`tools/feat-c1-hidpi/measure-size.mjs`)が、`dpi`・`parts`・`edgeWidth`・`debugLog` を全て出すようにする。null のままの値を残さない。
2. **DPI 240 以上の停止を直す**。最初の実験として、切り分けを行う。(i) font の高さだけを DPI 96 にする → (ii) `GetDeviceCaps` の LOGPIXELS だけを 96 にする → (iii) `GetDpiForMonitor` だけを 96 にする。原因を特定して直す。DPI 216(DPR 2.25)も確かめる。受け入れは、乱数の DPR が 1.00〜3.00 なので、DPR 2.5 以上でも idle に達することを求める。
3. **認識の値**: 推定の「2」でなく、マニフェストの宣言から決める。
4. 崩壊(`blackShare` が DPR 1.87 で 0.976)・ポインタの座標の変換(物理画素の位置。±1 画素)・`WM_DPICHANGED` の扱い(起動時の DPR のままでよい。再起動の案内は #{{108}})。

## 背景

依頼者の実機の条件(依頼者が貼ったデバッグパネルの実測。runner の値ではない)。

- `guest screen 3240x1863 device px in a room of 1836x1056 CSS px (scale 0.5667 CSS px per guest px, device pixel ratio 1.7647, 6.04 Mpx)`
- guest(FL)の画面は物理画素(3240×1863)で、FL の UI は 100% の大きさのまま描かれる。CSS では 0.5667 倍に縮むため、操作できない。
- 左の Browser のパネルの下の領域(ブラウザの一覧)が黒いまま。Channel rack・Playlist・Mixer の配置が崩れ、Mixer が左下に小さく出る。ウェルカムのダイアログも小さい。
- 表示方式(現状): `display method D: the runtime describes the desktop (windows, positions, written rectangles) and the render Worker draws it with WebGL2, one texture per window`
- 依頼者が貼った FL の画面: 窓 `Settings - Debugging log` は、画面全体の約 1/4 の大きさ(物理画素 3240×1863 に対して、UI が 100% のまま)で出る。ヘッダーの `直近 60 秒の途切れ` は 343。`engine cycle 0.00 s (1058.1/s)`(入力 837 件・処理 835 件。マウスの 1 イベントごとに engine が 1 周している)。
- 依頼者の観察(ネイティブの Windows): Main GUI Scaling の System が、OS の拡大率(175%)に自動で従う。今の版では従っていない。

FL の仕様(FL 公式マニュアルの General settings の Display の節の記述。実機では未確認。確かめる対象とする):

- 設定の場所: `Options > General settings`(F10)の Display の節。
- Main GUI Scaling / resizing(FL の再起動で効く): 既定は System。System のとき、OS の拡大率に FL が自動で従う(例: OS が 200% なら FL も 200%)。カスタムの割合を手で指定もできる。UI はベクターで描き直される(画素を引き伸ばすのではない)。
- Pop-ups scaling・Toolbars scaling(再起動で効く): 既定は Main(Main GUI Scaling に従う)。個別にも指定できる。
- Cursors scaling: 「OS の設定を使う、またはカスタム」。
- Define 1 inch / set display's PPI: Windows 8.1 以降は画面の PPI が自動で検出される。タッチの校正用。値は FL の見た目を変えないが、一部のプラグインに影響する。
- Legacy scaling(再起動で効く。Advanced の節): 既定は Main(System の拡大に従う)。FL のネイティブプラグインの GUI を拡大する。100%・200%・300%・400% の整数だけ。分数の拡大はできない(Fractional scaling is not possible)。ネイティブプラグインは、プラグインの wrapper の Scaling の倍率でも拡大できる。サードパーティのプラグインは、Bridged の設定で扱う。
- Thick lines(200% 以上の線の太さ)・Animations・Smoothing・Animation refresh rate も、この節にある。

推定(未確認。実測で確かめる):

- FL が UI を拡大するかどうかは、FL が環境から読む値で決まる(実行ファイルのマニフェストの DPI 対応の宣言、DPI の API の戻り値、`GetDeviceCaps` の LOGPIXELS、レジストリの値)。runtime は FL のプロセスを DPI 非対応として扱い、96 を返している。
- FL は、96 DPI の 1 倍の UI を、画素数の多い画面に描いている。これが「小さい」の原因とみられる。

コードの事実(main の現在の版):

- DPI は 96 のまま。`lift/runtime/fle_gdc.h` 31 行(`FLE_GDC_LOGPIXELSX` は 96)、`lift/runtime/fle_screen.h` 57〜59 行(仮想ディスプレイの DPI は 96)、`lift/runtime/fle_appfile.c` 225〜239 行(`GetDpiForMonitor` は 96 を返す)、`lift/runtime/fle_proc.h` 94〜96 行(`GetProcessDpiAwareness` はプロセスを DPI 非対応(UNAWARE)として返す)。
- #1306 の commit `56f5d0e6`(guest の画面を物理画素にする)は、guest の画面を「部屋の CSS × 拡大率」の物理画素にし、canvas を 1 / 拡大率 の CSS px で表示した。その commit 本文に「DPI(LOGPIXELSX)は 96 のまま」と明記されている。commit `24e86883`(題名は「DPI を guest に伝える: 完了 (Closes #1306)」)は変更を含まない(空の commit)。つまり、DPI を guest に伝える変更は main に入っていない。
- `56f5d0e6` の親の版では、guest の画面は部屋の CSS 画素と同じ大きさで、canvas を DPR で拡大して表示していた。ぼやけるが、操作はできた。

Windows の仕様(確かめる対象。DPI の公式の文書で確認する):

- DPI 対応のアプリは、DPI を受け取り、物理画素で描く。DPI は拡大率から決まる(上の 2 の式)。
- DPI 非対応のアプリは、システムが画面を拡大して表示する(ぼやける)。
- 拡大率 100% の時は、どちらも同じ。

## 読む物

- `lift/runtime/fle_gdc.h`(31 行と 41〜70 行)、`lift/runtime/fle_screen.h`(1〜80 行)、`lift/runtime/fle_sysmetrics.h`(7 行・43〜51 行)、`lift/runtime/fle_proc.h`(94〜97 行)、`lift/runtime/fle_appfile.c`(225〜239 行)、`lift/runtime/fle_spi.h`(90〜93 行。フォントの高さは LOGPIXELSY で決まる)
- `lift/harness/seeds/`(seed の表・レジストリの seed・設定ファイルの seed。FL の設定の初期値)
- `app/src/ui/screen-fit.ts`(1〜6 行の注釈、`chooseGuestScreen`・`drawScale`)、`app/src/ui/desktop.ts`(21〜25 行 `deskCanvas`)、`app/src/main.ts`(`measureRoom`・`applyDisplay`)
- `app/src/engine/render-worker.ts`、`app/src/engine/desk.ts`、`app/src/engine/desk-gl.ts`、`app/src/input/pointer.ts`
- `app/test/unit/screen-fit.test.ts`
- `tools/feat-c1-hidpi/measure.mjs`(#1306 の道具。土台にする)、`tools/boot-check/cdp.mjs`
- `tools/fl-installer-extract.mjs`(インストーラの抽出。FL の実行ファイルを取り出す)、`inputs/fl-installer-files.tsv`(`imports` の列)
- `git show 56f5d0e6`、`git show 56f5d0e6^:app/src/ui/screen-fit.ts`(拡大して表示していた版の基準)
- `pm-work/findings/survey-c1-hidpi.md`

## やること

上の「優先順位」の順に行う。(a)〜(j) の測定は、実装より先に終える。

1. メトリクスの取得(実装より先に、必ず行う)。道具 `tools/feat-c1-hidpi/measure-size.mjs`(新規)で測り、以下の (a)〜(j) の結果を、`pm-work/findings/hidpi-fl-scaling.md`(新規。PM の資料)に保存する。生のログの長い引用は載せず、行数と要点と表にする。
   - (a) 記録の仕組み: 既定では出さない。測定の道具だけが有効にできる既存の仕組みで、FL の起動から idle までの、DPI・拡大率・画面の大きさ・システムのメトリクスに関係する全ての呼び出しを、順序・API 名・引数・戻り値・呼び出し元のモジュールとアドレスつきで記録する。対象: DPI の API(`SetProcessDpiAwarenessContext`・`SetProcessDpiAwareness`・`SetProcessDPIAware`・`GetProcessDpiAwareness`・`GetThreadDpiAwarenessContext`・`GetAwarenessFromDpiAwarenessContext`・`GetDpiForWindow`・`GetDpiForSystem`・`GetDpiForMonitor`・`GetSystemMetricsForDpi`・`SystemParametersInfoForDpi`・`AdjustWindowRectExForDpi`・`EnableNonClientDpiScaling`・`SHGetScaleFactorForMonitor`・`GetScaleFactorForMonitor`・`GetScaleFactorForDevice`)、`GetDeviceCaps` の全ての index と戻り値(`LOGPIXELSX/Y`・`HORZSIZE/VERTSIZE`・`HORZRES/VERTRES`・`DESKTOPHORZRES/DESKTOPVERTRES`・`BITSPIXEL`・`SCALINGFACTORX/Y`・`ASPECTX/Y` など)、`GetSystemMetrics` の全ての index と戻り値(`SM_CXSCREEN/SM_CYSCREEN`・`SM_CXVSCROLL`・`SM_CYHSCROLL`・`SM_CYCAPTION`・`SM_CXFRAME`・`SM_CXICON`・`SM_CXSMICON`・`SM_CXCURSOR` など。Windows に公開されている定数だけを使う。戻った値は呼び出しごとに 1 行)、`SystemParametersInfoW` の全ての `uiAction` と戻り値(`SPI_GETNONCLIENTMETRICS`・`SPI_GETICONMETRICS`・`SPI_GETWORKAREA`・`SPI_GETMOUSE`・`SPI_GETSCREENREADER` など)、`EnumDisplayMonitors`・`GetMonitorInfoW`・`EnumDisplaySettingsW`・`MonitorFromWindow` の戻り値、レジストリの読み(キーの名前・値の名前・戻り値。DPI 関係では `HKCU\Control Panel\Desktop` の `LogPixels`・`Win8DpiScaling`、`HKCU\Control Panel\Desktop\WindowMetrics` の `AppliedDPI`、`HKLM\SOFTWARE\Microsoft\Windows NT\CurrentVersion\FontDPI` の各値。FL 自身の設定のキーの値: FL が `HKCU\Software\` の下に持つ、GUI の拡大・Main GUI Scaling・Legacy scaling・Pop-ups・Toolbars・Cursors scaling に当たる値)、設定ファイルの読み(FL の設定フォルダ `Settings\` の下のファイルの名前と大きさ)、カーソルの大きさに関係する呼び出し(`GetCursorInfo`・`GetIconInfo`・`LoadCursor`・`LoadImage`)。
   - (b) seed の確認: `lift/harness/seeds/` の seed 表・レジストリの seed・設定ファイルの seed に、GUI の拡大に関係する値がどう入っているかを調べ、表にする。Main GUI Scaling が System か、数値に固定されていないかを確かめる。数値に固定されていれば、値を記号の `System` に直す(seed に数値の拡大率を書かない)。
   - (c) F10 の Display の節: スタートの Default のプロジェクトを開き、F10 の General settings の General タブの Display の節を、スクリーンショットで撮る(Main GUI Scaling・Pop-ups・Toolbars・Cursors・Legacy scaling の選択が読める大きさで)。読み取れる選択値を報告に書く。
   - (d) FL が採用した拡大率の実測: 同じ起動を、DPR の入力を変えて(固定の 1・1.25・1.5・1.75・2 と、乱数の 2 つ。8 の道具の仕様)行い、guest が見る DPI(記録の `GetDeviceCaps(LOGPIXELSX)`)と、FL が採用した拡大率(%)を、UI の部品の大きさ(Tempo の部品の幅の画素数 ÷ DPI 96 の時の幅の画素数)から求める。表にする: DPR・DPI・FL の拡大率。FL が段に丸める場合は、段の値と境界を特定して書く。期待する値は、先に書かない。
   - (e) Windows の基準の値との突き合わせ: 同じ DPR について、Windows の公開の仕様から計算した値(`GetDpiForSystem`・`GetSystemMetricsForDpi(n, dpi)` = `MulDiv(96 DPI の時の値, dpi, 96)` など。式は 3 のとおり)と、記録の値を並べ、食い違う行を全て列挙する。計算値と実測値は区別して書く。実機の Windows の値は、この機械では取れない。
   - (f) 全ての記録・表を findings に保存する(上の (a)〜(e) の要点と表)。
   - (g) FL の Debug ログ: 起動して idle の後、スタートの Default のプロジェクトを開き、F10 の General settings の窓を開いて、Debug のタブ(`Settings - Debugging log`)を出す。ログの全文を、次のどちらか(または両方)で取る。① ゲストの窓のテキストを engine の側から読む(窓の部品が標準のテキストの部品なら `WM_GETTEXT`・`GetWindowTextW`・`EM_GETLINE` の読み。FL 自作の描画なら `ExtTextOutW`・`DrawTextW`・`TextOutW` の文字列の記録。測定の道具が有効にした時だけ記録する)。② 全文が取れない場合の代替として、スクロールしながらスクリーンショットを撮る(文字が読める拡大で)。作業者が、どの方式で取れるかを実測で調べて選び、理由を報告に書く。ログは findings に、行数・先頭と末尾の数十行・絞った行(`grep -i 'dpi\|scal\|screen\|monitor\|display\|resol\|zoom\|window\|error\|warn'` で絞った全行)を載せる。長い全文は載せない。
   - (h) 前後の比較: 同じ Debug ログを、修正の前(今の版)と、修正の後の両方で取り、拡大率に関係する行の差を表にする。FL が、後の版で何を見て拡大率を決めたかを、FL 自身の言葉(ログに出た行)で示す。
   - (i) 他のタブも同じ方法で取る: General タブの Display の節(選択値)、Info タブ(FL・OS・画面の情報が出るなら、その全文)。
   - (j) 測定の道具の JSON に `debugLog`(`{"captured":true|false,"lines":N,"method":"text-api|draw-text|screenshot","scalingLines":[…絞った行…]}`)を出す。取れない場合は `captured` を false のままにし、報告に理由を書く。その場合、受け入れは通らない扱いにする(取る方法を見つけて実装するまでが、この Issue の範囲)。
   - 推測を事実として書かない。ログに出た行と、記録された API の戻り値だけを根拠にする。
2. FL の設定 Main GUI Scaling を System にする。seed の値は記号の `System` にする。数値の拡大率を、seed・設定・runtime・app に書かない。Pop-ups・Toolbars・Legacy scaling は既定の Main のまま、Cursors scaling は既定の OS の設定のままにする。(1 の (b) で直す必要があった場合は、その事実を報告に書く。)
3. System が OS の拡大率に従う経路を直す。FL が System を解釈するときに読む API とレジストリ(1 の (a) で特定したもの)の戻り値を、ページが渡した DPR から Windows と同じ計算で返す。式: 拡大率(%) = round(DPR × 100)、DPI = MulDiv(96, 拡大率(%), 100)、`GetSystemMetricsForDpi(n, dpi)` = `MulDiv(96 DPI の時の値, dpi, 96)`、`LogPixels` の値 = DPI。FL が読んだ値から UI の拡大率を決めるのは FL 自身であり、拡大率を runtime が決めて表で渡さない。
4. 起動時の DPR の実数を、ページから engine へ渡す。既存の engine の環境の仕組み(`FLE_MONITOR_W`・`FLE_MONITOR_H` と同じ経路)を使う。engine は 3 の式で拡大率と DPI を計算する。新しい環境変数・URL パラメータ・ビルド変数を作らない。既定で動く形にする。
5. ネイティブプラグインの GUI は、整数の拡大だけ(Legacy scaling。100・200・300・400%)。プラグインの窓(Fruity Balance など)の大きさは、1 の実測で、FL が実際に出す拡大(特定した丸め)を確かめ、報告に書く。分数の拡大を FL が出した場合は、その事実を報告に書き、推定で直さない。
6. FL が DPI をどう決めるかを特定する(1 の (a)・(e) と合わせて行う)。
   - a. 実行ファイルのマニフェスト: インストーラを `tools/fl-installer-extract.mjs` で抽出し、主の実行ファイルの PE の RT_MANIFEST 資源を読む(node の道具で。製品には入れない)。`dpiAware`・`dpiAwareness`(PerMonitorV2 など)・`gdiScaling` の値を報告に書く。
   - b. 結論を報告に書く: 「FL が拡大を決める情報は X・Y・Z。runtime の現在の値は A・B・C」。実測と推定を分ける。
7. 直す(`lift/runtime` と `app/src`)。
   - guest の画面: round(部屋の CSS 幅 × DPR) × round(部屋の CSS 高さ × DPR) の物理画素。部屋の大きさは `measureRoom` の値を使う。
   - DPI: 3 の式(`MulDiv(96, round(DPR × 100), 100)`)。FL が見る LOGPIXELSX/Y・`GetDpiForWindow`・`GetDpiForMonitor`・マニフェストの宣言の経路を、Windows の DPI 対応のアプリと同じ値にする。
   - ブラウザ側: canvas の描画領域 = 表示領域の物理画素(`ResizeObserver` の `devicePixelContentBoxSize` を使い、端数の CSS 寸法でも表示の物理画素に揃える)。1 guest 画素 = 1 物理画素。`image-rendering` などで誤魔化さない。
   - ポインタ: ブラウザのポインタ(CSS 座標、小数)を、物理画素の位置で guest の画素に変換する(`floor(clientX × DPR)` と同じ。ずれは ±1 画素以内)。
   - 単体テスト `app/test/unit/screen-fit.test.ts` の #1306 の期待値を、物理画素の規則に合わせて直す。期待値は、DPR から式で計算して書く(値の表を埋め込まない)。
   - 画面の大きさと DPI が実行中に変わった時(動的な変化)は、既存の `WM_DISPLAYCHANGE` の経路で engine に伝える。`WM_DPICHANGED` の扱いは #{{108}} が検証する。この Issue の受け入れは、起動時の DPR で正しいこと。
   - 崩壊: Browser のパネルの黒い領域、Mixer の位置、配置のずれが、物理画素の画面で出ないこと。原因が guest の画面の大きさ(部品の座標の計算)なのか、render Worker(WebGL2・1 窓 1 テクスチャ・テクスチャの大きさの上限・描画の矩形の通知)なのかを切り分けて直し、報告に書く。
   - FL が自分で拡大しないと実測で分かった場合は、受け入れを満たせない。その証拠(呼ばれた API・返した値・マニフェストの値・Debug ログの行)を報告に書く。拡大して表示する版で代えない。
8. 測定の道具 `tools/feat-c1-hidpi/measure-size.mjs` を完成させる。道具の先頭に、部品の位置の求め方と、各閾値の根拠を書く。DPR の入力は、固定の一覧(1・1.25・1.5・1.7647・2)に、乱数の 2 つを加える。乱数は、起動のたびに crypto の乱数で種を選び、その種から範囲 1.00〜3.00・0.01 刻みの値を 2 つ決める。種と 2 つの値を、JSON の `randomDprs` に出す。固定の一覧は DPR の入力としてだけ使い、期待値には使わない。期待値は、各行の DPR から式で計算する。

## 測定の道具の出力

stdout に、DPR ごとに 1 行の JSON を出す(固定 5 + 乱数 2 = 7 行)。

```
{"dpr","randomDprs":{"seed","values"},"room":{"w","h"},"state","standIn","error","guest":{"w","h"},"dpi","flScalingPercent","mainGuiScaling","parts":{"tempo":{"x","y"},"song-position":{"x","y"},"pattern-selector":{"x","y"}},"canvas":{"w","h"},"display":{"w","h"},"edgeWidth","blackShare","debugLog":{"captured","lines","method","scalingLines"}}
```

- `randomDprs`: 起動のたびに選んだ乱数の種(`seed`)と、2 つの値(`values`)。全ての行に同じ値を入れる。
- `room`: 道具が測った部屋の CSS の大きさ。窓の大きさは、部屋が 1836×1056 CSS px になるように決める。
- `guest`: engine の記録の `GetSystemMetrics(SM_CXSCREEN/SM_CYSCREEN)`。
- `dpi`: guest が見た `GetDeviceCaps(LOGPIXELSX)`。
- `flScalingPercent`: FL が採用した拡大率(%)。1 の (d) の求め方(UI の部品の大きさの比)で求める。FL が段に丸める場合は、丸めた後の値を入れ、丸め方を報告に書く。
- `mainGuiScaling`: FL が起動時に読んだ Main GUI Scaling の値(engine の記録の、FL の設定の読み)。
- `parts`: 部品(Tempo・Song Position・Pattern Selector)の左上の、ページの CSS 座標。
- `canvas`: `canvas[data-desk="1"]` の width・height(描画領域)。
- `display`: canvas の getBoundingClientRect × DPR(物理画素。四捨五入)。
- `edgeWidth`: スクリーンショットの、Tempo の部品の枠の縁の遷移の幅(画素。10%〜90% の遷移の、水平と垂直の中央値)。FL が自分で描いた縁は 2 画素以下。拡大されてぼやけた縁は 3 画素以上に広がる。
- `blackShare`: Browser のパネルの矩形(CSS 座標で道具が決め、DPR に依らず同じ矩形)の中で、RGB が全て 16 以下の画素の割合(0〜1)。
- `debugLog`: 1 の (g)・(j) の結果。

## 受け入れ

```accept
npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false?0:1)' "$RUNNER_TEMP/boot.json" && node tools/feat-c1-hidpi/measure-size.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/size.jsonl" && node -e 'const L=require("fs").readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const P=["tempo","song-position","pattern-selector"];const near=(x,y,t)=>Math.abs(x-y)<=Math.max(2,t*Math.abs(y));const eq=(x,y)=>Math.abs(x-y)<0.0001;const a=L.find((x)=>eq(x.dpr,1));const R=L[0].randomDprs;const row=(x)=>x.error===null&&x.state==="engine-idle"&&x.standIn===false&&Math.abs(x.guest.w-Math.round(x.room.w*x.dpr))<=1&&Math.abs(x.guest.h-Math.round(x.room.h*x.dpr))<=1&&Math.abs(x.dpi-Math.round(96*x.dpr))<=1&&Math.abs(x.flScalingPercent-Math.round(100*x.dpr))<=3&&x.mainGuiScaling==="System"&&Math.abs(x.canvas.w-x.display.w)<=1&&Math.abs(x.canvas.h-x.display.h)<=1&&x.edgeWidth<=2&&x.debugLog.captured===true&&x.debugLog.lines>=1&&P.every((p)=>near(x.parts[p].x,a.parts[p].x,0.05)&&near(x.parts[p].y,a.parts[p].y,0.05))&&Math.abs(x.blackShare-a.blackShare)<=0.02;const ok=a!==undefined&&L.length===7&&[1,1.25,1.5,1.7647,2].every((d)=>L.some((x)=>eq(x.dpr,d)))&&R.values.length===2&&R.values.every((d)=>d>=1&&d<=3&&L.some((x)=>eq(x.dpr,d)))&&L.every(row)&&a.blackShare<0.5&&Math.abs(a.room.w-1836)<=2&&Math.abs(a.room.h-1056)<=2;process.exit(ok?0:1)' "$RUNNER_TEMP/size.jsonl" && ! git diff -U0 refs/judge/main -- lift/runtime lift/harness app/src | grep -E '^\+[^+]' | grep -qE '\b(125|150|168|169|175|176)\b|1\.75\b|1\.7647|0\.5667' && npm test --prefix app
```

判定の基準: 道具の 7 行の生の値を、上の node -e が読む。
- (a) 固定の一覧(1・1.25・1.5・1.7647・2)と、`randomDprs.values` の 2 つが、全て行に在る。全行が、正常な起動(`error` が null・`state` が engine-idle・`standIn` が false)。
- (b) guest の w と h が round(room × DPR) の ±1 画素。
- (c) dpi が round(96 × DPR) の ±1。flScalingPercent が round(100 × DPR) の ±3(173〜179 の範囲。依頼者の OS の拡大率 175% と、DPR から求める値の、両方を含む)。`mainGuiScaling` が `"System"`。
- (d) 部品の parts が DPR 1 の値と ±5% 以内(小さい値は 2 CSS px まで)。
- (e) canvas と display が ±1 画素で一致。
- (f) edgeWidth が 2 以下。blackShare が DPR 1 の値と 2 ポイント以内。DPR 1 の blackShare は 0.5 未満。DPR 1 の room は 1836×1056 ±2 CSS px。
- (g) `debugLog.captured` が true で、`lines` が 1 以上。
- 乱数の DPR は、作業者の実行と判定の実行で別になる。そのため、固定の値に合わせた実装は通らない。
- ソースの検査(受け入れの最後の段): `lift/runtime`・`lift/harness`・`app/src` の、この変更で足した行(`git diff` の追加行)に、特定の拡大率・DPI・DPR の値(`125`・`150`・`168`・`169`・`175`・`176`・`1.75`・`1.7647`・`0.5667`)が、拡大率・DPI・DPR の意味で現れないこと。一致が 0 件であること。別の意味で一致する場合は、定数名を変えるか、式で書く。
- 単体テスト(`npm test --prefix app`)が通ること。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には「runner の値。この機械とは別枠」と書き、nproc と loadavg を添える。
- 実測と推定を分ける。「達成」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。出力の行を付ける。
- 報告に書く: 1 の (b) の seed の表、(c) の F10 の読み取った選択値、(d) の表(DPR・DPI・FL の拡大率と、FL の丸め方)、(e) の表(Windows の計算値と記録の突き合わせ。食い違う行を全て列挙)、(g)〜(i) の Debug ログの取得方法と絞った行、(h) の前後の差、FL が拡大を決める情報(マニフェストの値、呼ばれた API と戻り値)、DPR ごとの表(guest・dpi・flScalingPercent・部品の座標・edgeWidth・blackShare・debugLog の captured と lines)、`randomDprs` の種と値、崩壊の原因(切り分けの結果)、変えたパス(app/・lift/・tools/ を含む)、拡大率を導く式の場所(ファイルと行)、特定の値を用意していないことの確認(上の source の検査の grep の出力。一致 0 件)、拡大して表示する版を入れていないことの確認、#{{108}} に残した点。

## 制約

- 製品に、利用者向けの新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足さない。既定で動く形にする。
- `FLE_MONITOR_DPR` は、`FLE_MONITOR_W/H` と同じ、ホストが engine へ渡す実行時の入力で、利用者向けの選択肢ではない。前回の作業が、この経路に置いた。新しい利用者向けの旗・URL パラメータ・ビルド変数は作らない。
- 特定の拡大率・DPI・DPR の値を、runtime・app・seed・設定・テストに、定数・表・分岐・seed の値として入れない。この Issue の本文に書いた例の数字(100・125・150・175・176・200%、DPI の 96 以外の値、DPR 1.7647 など)も、製品には入れない。拡大率は、実行時の DPR から式で導く。
- 論理の小さい画面を DPR 倍に拡大して表示する実装を、中間の版としても作らない。
- 製品の既定の動作を、測定のためだけに変えない。
- ベンダー名と Cloudflare の識別子を書かない。
- sleep・timeout・nohup・末尾の `&` は使わない。一時ファイルは `.tmp-*` に置き、終わったら消す。
- 依頼者の資格情報・設定に触れない。
- Python は製品に使わない(調査と検証にだけ使う)。

## 作業の進め方

- engine の作り直しは約 30 分。ビルドは、フォアグラウンドで、シェルツールの `timeout` 引数に大きい値(例 3600000)を渡して待つ。
- 1 回の試行(最大約 5.5 時間)で、ビルドは数回までなので、実験はビルドをまとめて計画する。
- 作業の最後に、受け入れが通らなくても、作業ツリーを commit する。通らなければ push されず、差分が保存される(次の run が続きから取り込む)。

## 他のタスクとの関係

- #{{108}}(動的な変化と、全ての拡大率・全ての窓の種類の検証)は、この Issue の後に行う。
- #1349(音が途切れる現象)は、この Issue の後の物理画素の画面で測る。
- #1334(音声の圧迫)と同じ描画の経路(`app/src/engine/client.ts`・`desk.ts`・`render-worker.ts`)を変えうる。衝突したら、作業の最後に `git pull --no-rebase --no-edit` で main を取り込み、衝突を直し、受け入れをもう一度通す。
- #1306・#1308 の結果(DPI の経路と、その実測)は、この Issue が置き換える。
- #1346(トップバーの右クリック)は、部品の座標を使う。この Issue で座標の基準が変わる場合は、同じ手順で main を取り込む。

