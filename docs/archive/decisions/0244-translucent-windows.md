# 0244: 半透明の窓 — プルダウンの選択後が『黒へフェード』する原因とプラグインピッカーが不透明な原因は、DWM の拡張フレーム(`DwmExtendFrameIntoClientArea`)・BitBlt のアルファ・`SetWindowRgn` の契約の不足だった。仮の決定

状態: **仮の決定**。担当 alpha1(PM の指示、2026-11-05 07:27〜)。作業は HEAD **a61760ba** の `git archive` の木(最後に HEAD c8cdfd07 で `patch -p1 --dry-run` を確認)。凡例: **実測** = このセッションのコマンドの出力を見た事実(2026-11-05、この機械〔16 コア・SwiftShader・高負荷〕)、**読み** = コード・文書を読んだ解釈、**UNOFFICIAL** = Learn が沈黙し、出典(Wine など)つきで選んだ物、**未確認** = 確かめていない。**依頼者の機械・実 GPU・実機 Windows の値は 0 件。**

**何を実測したか**: 実エンジン(FLEngine と同梱 DLL を x86-64→C にした物。production 形 F2xH7 の生成 C を clang -O2 でビルドし、HEAD の runtime を `-O2 -DFLE_DESKTOP -DFLE_R_FAST_COPY=1` で自前ビルドして link した native N1)に、起動直後の既定プロジェクトで File / Add / View のメニュー・チャンネルラックの選択式のコンボ・F8 のプラグインピッカーを G3 の模擬入力(ユーザー操作ではない)で操作させ、ゲストが呼ぶ API の引数と戻り値、窓の面に BitBlt されるビットマップのアルファ、`fle_desk_compose` が合成した画素を 16 ms ごとに記録した。同じ操作を修正の前後で比べた。wasm は production 形の bundle(F2xH7)を CI の段スクリプトで作り直し(+ node 用の FS 版)、node の仮想時計と Chrome 156 と Firefox 158(headless)で見た。
**結論**: 原因は **(b) 外部 API の契約の不足** の 3 点。FL の窓のポップアップメニューとプラグインピッカーは層状窓ではなく、`DwmExtendFrameIntoClientArea`(余白 −1 = ガラスのシート)で拡張フレームにした窓に、**事前乗算 ARGB の DIB を BitBlt** して半透明を作る。(1) runtime は窓の面の 4 バイト目(A)を BitBlt で捨て、(2) 合成(`fle_desk.c`)は層状でない窓を不透明に描き、(3) `SetWindowRgn`(選択項目だけを残すリージョン)は『状態のみ』だった。結果、選択項目の A が 251→0 に減るフェードは『事前乗算の RGB が減るだけ』=不透明な黒へのフェードになり、プラグインピッカー(全画素が A≈34→236 のフェードイン)は不透明な暗い面になった。直しは **無条件の契約の実装**(旗なし・クラス名分岐なし): 余白の保持、拡張フレームの窓の面での A の保持、余白の領域の事前乗算合成、`SetWindowRgn` の表示と当たり判定への適用。
**未確認**: 実機 Windows の DWM との画素の照合(0 件。部分的な A の合成式は BLENDFUNCTION の式を流用した UNOFFICIAL)、依頼者の Firefox 159(Windows)での表示、GL 合成器(gpu1)との画素の一致(条件の表だけ書いた)、非矩形のリージョン(GDI モデルが矩形のみ)。

## 1. 先に結論(表)

| 項目 | 内容 |
|---|---|
| 依頼者の報告 | 『プルダウンの選択後、選んだ項目だけが半透明にフェードアウトするはずが黒にフェードする』『プラグインピッカーが半透明になっていない』(2026-11-05) |
| ゲストが半透明をどう作るか(**実測**) | **層状窓ではない**。`TQuickPopupMenuWindow`(ポップアップメニュー、コンボの選択肢)も `TPlugListForm`(F8 / Add の `Plugin picker`、1920x1080 の全画面窓)も `exstyle 0x10000`(WS_EX_CONTROLPARENT のみ)の最上位ポップアップで、作った直後に `DwmExtendFrameIntoClientArea(hwnd, MARGINS{-1,-1,-1,-1})`(ガラスのシート)を呼び、**事前乗算 ARGB の 32bpp DIB を窓 DC へ BitBlt**(SRCCOPY)する。ULW は起動直後のロゴ(`TLightweightLayeredControl`)の 93 回だけ、SLWA / AlphaBlend / GradientFill / AnimateWindow / DwmEnableBlurBehindWindow は **0 回**(FLEngine の import に AlphaBlend・GradientFill・AnimateWindow が無い)。 |
| 選択後のフェード(**実測**) | 選択すると FL は窓の exstyle を `0x10028`(TOPMOST | TRANSPARENT)にし、`SetWindowRgn`(選んだ行だけの矩形リージョン)を呼び、以後 **2 フレームごと(約 32 ms)に同じ窓へ BitBlt**。ソース DIB の選んだ行の画素は A = 251→201→159→124→98→77→61→48→37→29→23→18→14→11→8→6→5→4→3→2→1→0(約 0.8 秒)と減り、事前乗算の RGB も一緒に減る。リージョンの外の部分は下の窓の写しで A = 255。つまり FL は『A を下げて下の窓を透かす』フェードをしている。 |
| ピッカー(**実測**) | `TPlugListForm` に毎フレーム `BitBlt (0,0 1920x1080)`。ソースは **2,073,600 画素すべてが半透明**(A は 34→64→90→…→203→224→236→249 とフェードイン、暗い色の約 92〜98%)。閉じるときは exstyle を 0x10028 にして A を下げる。 |
| 黒へのフェードの原因 | **(b) 外部 API の契約の不足**の 3 点が重なった。(1) `fle_r_blit` は窓の面の 4 バイト目を捨てる(窓の面は `keep_x = 0`)ので A が残らない。(2) `fle_desk.c` は層状でない窓を A = 255 で不透明に描く。(3) `SetWindowRgn` は『状態のみ・描画を切らない』(S-ia1-rgn-1)。→ 選んだ行の事前乗算 RGB(A で暗くなった色)が不透明のまま出て『黒へ』。窓が消える瞬間に下の色へ跳ぶ。 |
| ピッカーが不透明の原因 | 同じ。全画素の A(34→249)が捨てられ、事前乗算で暗くなった色が不透明な面として出る。 |
| 層状窓の式は原因か | **違う**。ULW / SLWA / ULW_OPAQUE / ULW_COLORKEY / LWA_ALPHA / LWA_COLORKEY / 2 窓の重なりを Learn の式から作った表(厳密有理数)と比べて 0 失敗(`glass_seq.c` 第 7 節)。SPI・AnimateWindow・DwmEnableBlurBehindWindow・AlphaBlend は呼ばれない。 |
| 直し(`alpha1.diff`) | (1) `DwmExtendFrameIntoClientArea` の MARGINS を窓に保持(`fle_win.c/h`、`fle_dwm.c`)。(2) 余白を持つ窓の面では 32bpp DIB からの無変換 SRCCOPY が 4 バイト目を運ぶ(`fle_gdi.c`: `keep_x`)。(3) `fle_desk.c` が余白の領域の画素を事前乗算 ARGB として `Dst = Src + (1 − A) × Dst` で合成(ULW と同じ式・同じ丸め)。(4) `SetWindowRgn` を合成(窓リージョンの外を描かない)と当たり判定(`WindowFromPoint`、入力)に適用。旗なし・クラス名分岐なし。 |
| 直した後(**実測**、native 実エンジン、同じ G3) | Add メニューの `Simple` 行: 修正前 [92 106 115]→…→[1 1 1]→[0 0 0](黒を経由)/ 修正後 [92 107 116]→[81 93 102]→[55 66 73]→[32 41 46]→[31 40 45](下の窓の色)、41 フレームすべて `composite = src + (1 − A/255) × under` に ±1 で一致、黒 0 フレーム。ピッカー: (500,300) が修正前 (95,102,110)→(3,5,5)→(24,31,37) / 修正後 (95,102,110)→(86,94,101)→…→(31,39,45)。 |
| 検証 | 単体 256 項目 0 失敗・mutation 27/27 検出、cargo 全体 **197 results / 2,388 passed / 0 failed / 1 ignored**(193 test executables + doc-tests 4)、a4 FRUITY 無効 942/942・895/895・functions 5,310・entries 1,009,737 不変、既定の生成 C の sha256 `cd9107ce…e50`(215,811,678 B)不変、起動の 16 スナップ native 68 行・wasm 84 行とも全行一致。wasm node の仮想時計(F2xH7 の FS 版)の pull1・f8 の合成画素は native と **全 771 点で同一**、修正前の wasm とは pull1 の 261 点中 168 点(フェード中の点)が意図どおり違う。Chrome 156 と Firefox 158 の実ブラウザ(模擬入力)でフェードが黒を経由せず、ピッカーが半透明(§7) |
| 費用(**実測**) | `fle_desk_compose` の CPU 時間(ピッカーの 207 万画素/フレーム): native -O2 で 3.4 ms → 約 9〜10 ms。wasm(node)の中央値 7.9 ms(最大 13.7 ms)、Chrome 156 の Worker ではピッカー定常状態の `fle_desk_compose` が 128 → 481 ms/s(1 フレーム約 2.0 → 7.7 ms、§8)。 |

## 2. 観測(実エンジン)

方法: native -O2 の N1(production 形 F2xH7 の生成 C を pick1〔決定 0242〕が clang -O2 でビルドした物を無改変で流用 + この木の runtime を `-O2 -DFLE_DESKTOP -DFLE_R_FAST_COPY=1` で自前ビルド + Blend2D)に、テスト道具 `tools/alpha1_rec.c`(リンク時の `--wrap` で全 `fle_imp_*` を数え、半透明に関わる API の引数・戻り値・BitBlt のソースのアルファ統計を記録し、`cpu LABEL` のたびに `fle_desk_compose` の画素を取る。runtime は変えない)を付け、`scripts/*.g3` を走らせた。**入力はハーネスの模擬(ユーザー操作ではない)**。G3 の `advance 16` + `cpu fNN` で 1 フレームずつ記録した。

### 2.1 窓とその呼び出し(`tables/glass-calls-timeline.tsv`、`tables/api-call-counts-pick1s.tsv`)

- File メニュー: `CreateWindowExW(ex=0x10000, style=0x86000000, "TQuickPopupMenuWindow")` → `SetWindowLongPtrW(GWL_EXSTYLE, 0x10000)` → **`DwmExtendFrameIntoClientArea(margins l=-1 r=-1 t=-1 b=-1)`** → `ShowWindow`。表示中は BitBlt のソース(1930x1092 の共有バックバッファ)の A は **251**(メニュー背景 (199,194,186) を 251/255 で事前乗算、`above_A` 0)。
- 項目を選ぶと: `SetWindowLongPtrW(GWL_EXSTYLE, 0x10028)`、`SetWindowRgn(hwnd, rect(2, 260, 206, 279), TRUE)`、以後 2 フレームごとに BitBlt、`ShowWindow(SW_HIDE)` + `DestroyWindow`(約 820 ms 後)。
- F8: `CreateWindowExW("TPlugListForm", 0,0 1920x1080)` → `DwmExtendFrameIntoClientArea(-1 …)` → `ShowWindow`、BitBlt 毎フレーム、閉じるとき exstyle 0x10028、約 460 ms 後に `ShowWindow(SW_HIDE)` + `DestroyWindow`。
- 実行全体の呼び出し数(F8 の run): UpdateLayeredWindowIndirect 93(すべて起動時の `TLightweightLayeredControl`、`SourceConstantAlpha=255 AC_SRC_ALPHA`)、DwmExtendFrameIntoClientArea 2、SetWindowRgn 1、BitBlt 2,162、StretchDIBits 55、CreateDIBSection 531。**SLWA・AlphaBlend・GradientFill・AnimateWindow・DwmEnableBlurBehindWindow・DwmSetWindowAttribute・DwmGetColorizationColor・IsThemeActive・BeginBufferedPaint は 0 回**。

### 2.2 システムの能力問い合わせ(`tables/capability-probes.tsv`)

ゲストが呼ぶのは `SystemParametersInfoW` の 0x48(ANIMATION: 我々は iMinAnimate = 0)・0x101a(CURSORSHADOW = 0)・0x1f・0x29・0x30(WORKAREA)・0x68・0x200a(FONTSMOOTHINGTYPE = 1)・0x200c・0x2012 だけ。**SELECTIONFADE / MENUFADE / MENUANIMATION / COMBOBOXANIMATION / TOOLTIPFADE / UIEFFECTS / GRADIENTCAPTIONS は 0 回**=『我々の答えがゲストの半透明の経路を止めている』問い合わせは無い。`DwmIsCompositionEnabled` は TRUE を答えて(29 回)ゲストはガラスの経路に入る。`GetDeviceCaps(SHADEBLENDCAPS=104)` は 0 を返しているが FLEngine は AlphaBlend / GradientFill を import しないので経路に影響しない(**読み**)。`GetSystemMetrics(SM_REMOTESESSION)`・`IsThemeActive`・`DwmGetColorizationColor`・`GetVersionExW`/`VerifyVersionInfoW`(Windows 8 以上を答える)のうち、半透明の分岐に使われると確認できた物は DwmIsCompositionEnabled だけ。GetProcAddress が NULL になる Dwm*/AnimateWindow の出口(pick1 の表)は FL が呼んでいないので、この決定では触らない。

### 2.3 フェードの列(`tables/fade-*.tsv`、`tables/fade-compare-sel1-dd1.tsv`、`tables/picker-source-alpha.tsv`)

Add メニューの `Simple` 行(窓 59,36 の 113x108、リージョン (2,89,113,108)、行の画素 (40,97))。`composite` は `fle_desk_compose` が作った画面の画素、`src` は FL が BitBlt した DIB の事前乗算の画素:

| フレーム | A | src R,G,B | 修正前 composite | 修正後 composite | 式の期待 |
|---:|---:|---|---|---|---|
| 46 | 251 | 92,106,115 | 92,106,115 | 92,107,116 | 92,107,116 |
| 48 | 201 | 74,85,92 | 74,85,92 | 81,93,102 | 81,93,102 |
| 50 | 159 | 58,67,73 | 58,67,73 | 70,82,90 | 70,82,90 |
| 52 | 124 | 45,52,57 | 45,52,57 | 61,73,80 | 61,73,80 |
| 64 | 29 | 11,12,13 | 11,12,13 | 38,47,53 | 38,47,53 |
| 68 | 18 | 6,8,8 | 6,8,8 | 35,45,50 | 35,45,50 |
| 86 | 0 | 0,0,0 | 0,0,0 | 31,40,45 | 31,40,45 |
| 窓が消えた後(下の窓) | | | 31,40,45(**跳ぶ**) | 31,40,45 | |

別の選択(View メニューの行 sel1、チャンネルラックのコンボ dd1)も同じ機構(`TQuickPopupMenuWindow` + 行のリージョン)で、dd1 は修正前 212→112→55→13→(84 に跳ぶ)、修正後 218→155→119→92→84。File メニューの `Basic.flp` 行は下の窓がほぼ黒([0,0,0])の位置なので、修正の前後で composite が同じになる(黒の上の事前乗算 = 事前乗算の RGB そのもの): 依頼者の『黒』は下が明るい所で見えやすい。

ピッカー(F8、`TPlugListForm`、全面): ソースの A と、下のプロジェクトの色が混ざること(`tables/picker-source-alpha.tsv`、`runs/f8-*`): 点 (500,300) の composite は、修正前 (95,102,110)→(3,5,5)→(6,8,9)→…→(24,31,37)(不透明な暗い面)、修正後 (95,102,110)→(86,94,101)→(78,86,93)→…→(41,49,56)→(31,39,45)(単調。最終は A≈236 の暗色 + 約 8% の下の色)。閉じるときは (94,102,109) に戻る(最後の 1 段は A = 1 の残り)。

## 3. 原因の絞り込み(PM の候補 (i)〜(vi) の判定)

| 候補 | 判定 | 根拠(実測) |
|---|---|---|
| (i) ULW / SLWA の式(事前乗算、四捨五入、AC_SRC_ALPHA の有無、ULW_OPAQUE) | **原因ではない** | 起動時のロゴ以外 ULW は呼ばれない。式は手計算の表と 7 種 + 2 窓の重なりで 0 失敗(`glass_seq.c` 第 7 節) |
| (ii) 層状窓の下に合成されるはずの下の窓の画素が使われない | 層状窓では起きない(表 TWO_LAYERS、赤い窓の上の SLWA / ULW)。**ポップアップ/ピッカーは層状窓でないので、そもそも下の窓を混ぜる経路が無かった** | 修正前の `fle_desk.c` は非層状の窓を `bgrx_to_rgba`(A = 255)で描く |
| (iii) ソースのビットマップの A が GDI 描画で 0 のまま事前乗算 → 黒 | **近い**: ただし ULW ではなく BitBlt で、FL は A を正しく作っている(`above_A=0`、A は 251→0 と単調)。**捨てているのは我々**(`fle_r_blit` が窓の面の 4 バイト目を 0 にする) | `srcA:` の統計と `pix(x,y)=A.R.G.B` の列 |
| (iv) AnimateWindow / SPI の問い合わせの我々の答え | **原因ではない** | 呼ばれない(§2.2) |
| (v) 窓が層状でなく TRANSPARENT / 別の経路 | **これが本体**: 拡張フレーム(DWM のガラス)+ `WS_EX_TRANSPARENT`|`TOPMOST` + `SetWindowRgn` | §2.1 |
| (vi) 表示と非表示の通知(WM_PRINT、WM_ERASEBKGND、WM_PRINTCLIENT) | 関与なし(フェードの BitBlt は WM_PAINT の BeginPaint/EndPaint の間。メッセージの種類は変えていない) | `A BeginPaint` 347 ≈ `A BitBlt` 347 の対 |

余計な処理(a)の点検: `fle_desk.c`・`fle_gdi.c`・`fle_raster.c` に FL のクラス名・窓の種類・位置による分岐は無かった(`grep` でクラス名の文字列 0 件)。『窓の面に 4 バイト目を持たせない』『非層状の窓は不透明に描く』『SetWindowRgn は描かない』は契約の不足であって FL の代行ではない(§4 の表)。ページ側(`desktop.ts` など)にも半透明の処理は無い(決定 0223 で 1 枚のデスクトップ面になった)。**除去すべき余計な処理は見つからなかった。**

## 4. 契約との照合(`tables/contract-check.md` が正本。要点)

Learn: `DwmExtendFrameIntoClientArea`(『フレームをクライアント領域へ広げる』、負の余白 = 『ガラスのシート』、サンプルは `MARGINS {-1}`)、DWM の custom frame の頁(『広げたフレームが見えるには、その下の画素の α が 0』)、`DwmEnableBlurBehindWindow` の頁(『窓のアルファ値は尊重される』『GDI の操作の中にはアルファを保たないものがある』)、`BufferedPaintSetAlpha` の頁(『GDI でバッファに描いてからガラスに出すときは不透明にする』= GDI の描画はアルファを作らない)、`SetWindowRgn`(『システムは窓リージョンの外の部分を表示しない』『座標は窓の左上基準』)、`BLENDFUNCTION`・`UpdateLayeredWindow`(式)、`BitBlt`・`WindowFromPoint`・`AnimateWindow`(コピーは `docs/investigations/alpha1/learn/`、層状窓の系は `docs/investigations/lay1/learn/`)。

契約が沈黙して選んだ物(**S-**):

| 名前 | 内容 | 出典 / 代替 |
|---|---|---|
| S-alpha1-dwm-negative | 余白のどれかが負なら窓全体が拡張フレーム(ガラスのシート) | Learn のサンプルが `MARGINS {-1}`(最初のメンバーだけが −1)で『ガラスのシート』と書く。代替: 4 つとも負のときだけ |
| S-alpha1-dwm-zero | 余白が全部 0 なら拡張を取り消す | 頁は沈黙。0 の余白は何も広げない(自然な読み) |
| S-alpha1-glass-over | 拡張フレームの領域の画素の部分的な α の混ぜ方 = BLENDFUNCTION の式(事前乗算、丸めは ULW と同じ) | **UNOFFICIAL**: 頁にあるのは α = 0 のとき見える/『窓のアルファ値は尊重される』まで。Windows 8 以降のぼかし無しの DWM では下の窓が透けて見える(推測、実機 0 件) |
| S-alpha1-gdi-a | 無変換 SRCCOPY(32bpp DIB → 拡張フレームの窓)だけが 4 バイト目を運ぶ。他の GDI 描画は 4 バイト目 = 0(A を作らない)=合成では『光を足す』 | `BufferedPaintSetAlpha` の頁(GDI はアルファを作らない)+ DIB 同士の無変換コピーが未使用バイトを運ぶ決定 0111 の先例。代替: 全 GDI 描画で A = 255 を作る(GDI の仕様に反する) |
| S-alpha1-additive | A = 0 で色のある画素は、式どおり下に光を足す(上限 255)。FL の BitBlt には現れない(`above_A = 0`) | BLENDFUNCTION の式の帰結。代替: 透明扱い(式に反する) |
| S-alpha1-rgn-draw | 合成が窓リージョンの外を描かない。窓の面の中身は切らない | `SetWindowRgn` の頁: 『表示しない』。リージョンは矩形のみ(GDI モデルが `CreateRectRgn` 系だけ。他は従来どおり名前つきの停止) |
| S-alpha1-rgn-hit | リージョンの外の点は窓に含まれない(`WindowFromPoint`、入力のヒットテスト) | **UNOFFICIAL**: Wine 10.0 `win32u/window.c` の `is_point_in_window`。頁は『点を含む窓』 |
| S-alpha1-glass-layered | 層状窓が拡張フレームを持つ場合は層状の経路が優先 | 頁は沈黙。FL では起きない |

## 5. 直し(`alpha1.diff`)

| ファイル | 変更 |
|---|---|
| `lift/runtime/fle_win.c`, `fle_win.h` | `win` に `dwm_margins[4]`(スロットは作成時にゼロ初期化されるので窓の破棄で消える)。`fle_win_set_frame_margins` / `fle_win_frame_margins` / `fle_win_window_region_box`。`hit_test`(WindowFromPoint)と `hit_visible`(入力)がリージョンの外を除く(`in_window_region`)。SetWindowRgn の注釈を『状態のみ』から更新 |
| `lift/runtime/fle_dwm.c` | `DwmExtendFrameIntoClientArea` が MARGINS(4 つの int)を窓に渡す |
| `lift/runtime/fle_gdi.c` | 窓の DC の対象で、拡張フレームの窓(`q.root`)なら `keep_x = 1`(§S-alpha1-gdi-a) |
| `lift/runtime/fle_desk.c`, `fle_desk.h` | `over_premul`(ULW と拡張フレームで共通の 1 関数。ULW の式・丸めは変えていない)、`LF_GLASS` と窓ごとの `gx0..gy1`(余白が届かない内側)・`cl..cb`(リージョンの箱)、`draw_rect` の拡張フレームの経路、`paint` がリージョンで絞る、`same_win` / `gather` / 説明行(`glass(..)` `region(..)` を層の語の位置に。行末は従来どおり class と title) |
| `lift/crates/fe-lift/tests/wall_ia1.rs` | 『DwmExtend は状態のみ』を固定していた 2 本の期待を更新(MARGINS が窓に渡る) |
| 新規 `lift/crates/fe-lift/tests/alpha1_glass.rs`, `lift/harness/alpha1/*` | §6 |
| 新規 `docs/investigations/alpha1/*` | 表・道具・Learn のコピー・MERGE-NOTES |

変更の分類(lift の忠実性の点検表): 外部 API の契約の実装 = 上の 4 点すべて。FL の観測だけ = 道具(`alpha1_rec.c`)は製品の外。代行・補正・クラス名分岐 = 0。新しいオプション = 0。生成 C の変化 = 0(`lift/crates/*/src` は 0 行)。

## 6. テスト

| テスト | 結果 |
|---|---|
| `glass_seq.c`(runtime そのものを native で動かす。cargo の `alpha1_glass`、通常と `-DFLE_R_FAST_COPY=1`) | **256 項目 0 失敗**: ガラスのシートの 16 種の画素 × 2 つの下(背景・赤い窓)、余白(左・右・上・下)、4 バイト目の保持/非保持、`SetWindowRgn`(内側・4 辺の外側・窓より大きいリージョン・説明行)、ヒットテスト(WindowFromPoint と入力)、フェード、ULW(AC_SRC_ALPHA + SCA 128/77、SCA 100 の定数アルファ、OPAQUE、COLORKEY)・SLWA(ALPHA / COLORKEY / 両方)・2 窓の重なり・明るい下の窓、同一内容の再描画は報告なし、最後の行列の 1 画素・A だけの変更の報告。期待値は `tools/mk-expect.py` が Learn の式から厳密有理数で作ったリテラル(C のコードは読まない) |
| mutation(`tools/mutate-glass.py`、`logs/mutate-glass.txt`) | **27 本すべて検出**(余白を保持しない・A を運ばない・拡張を無視・リージョンの左/右/下・ヒットテスト 2 本・丸め・(1−A) 無し・内外の取り違え・負の余白・右/上/下の余白・加算・クランプ・ゼロ余白・余白の辺の取り違え・LWA_ALPHA の補数・LWA_COLORKEY の青・ULW_OPAQUE の α・COLORKEY の 1 バイト・AC_SRC_ALPHA 無しで A を使う・SCA を無視・Z 順)。最初の 3 本(リージョンの左端・右の余白・窓より大きいリージョン)は生き残り、テストを足して検出 |
| 実エンジンのフェード列(`lift/harness/alpha1/run-fade.sh`、`check-fade.py`) | 修正後 0 違反 / 修正前 132 違反(Add メニュー)。View メニュー(22 フレーム)・コンボは 0 違反 |
| `test.sh full --jobs 3`(cargo) | **197 results / 2,388 passed / 0 failed / 1 ignored**(193 test executables + doc-tests 4) |
| a4 FRUITY 無効(`a4-fruity.sh off`、`logs/a4-off.log`) | `init_count=942 / 942`、`run_rank=895/895`、functions executed (distinct) **5,310**、function entries **1,009,737**、`== stop: returned from entry`、exit 0 — **不変** |
| FLEngine の既定の生成 C(`fe-lift lift FLEngine_x64.dll X --pdata`、この木の fe-lift) | sha256 **`cd9107cef4cbef14f154645749957b7067be7ed7c2b93f5bbe1f72fd47029e50`**、215,811,678 B — **不変**(`logs/felift-default.sha256`) |
| 起動 16 スナップ | native(`cmp-short.g3`、`g3-lines` 68 行)は修正前後で全行一致。**wasm(node の仮想時計)の `cmp-short.g3` の 84 行は、修正前の bundle(同じ F2xH7 の FS 版を HEAD の木で作った物、engine.8f30ec7c08cd.wasm)と修正後(engine.b0e39daa2d7a.wasm)で全行一致、記録されている基準の表(`lift/out-git/ref3-results/tables/g3-lines-base-84.txt`)とも全行一致**(`diff` の差 0 行、`g3-surface` 34 行も 0 差) |

## 7. wasm と実ブラウザ(production 形 F2xH7 の bundle を CI の段スクリプトで作り直した物)

bundle: `ci/stage-wasm.sh objects link` + `ci/stage-bundle.sh seed bundle`(gpu1 の `mk-inc.sh` の手順で、変えていない生成 C の 44 単位の wasm オブジェクトは前の bundle から再利用し、runtime とハーネスだけを作り直した。`tools/mk-wasm.sh`)。engine.wasm **133,926,537 B**(直前の HEAD の bundle〔gpu1 の ciA〕133,924,267 B)。S1 の線(ネイティブの 2 倍)への影響は +2,270 B で、計算に出ない大きさ。

### 7.1 node(仮想時計、F2xH7 の FS 版 bundle: engine.b0e39daa2d7a.wasm)

`tools/node-desk-run.mjs`(`lift/harness/parity/node-bundle-run.mjs` の写し + `snap` のたびにエンジン自身の `fle_desk_compose` を呼んで指定点の画素と説明行を書く)で、G3 の `pull1.wasm.g3`(`cpu` を `snap` に替えた同じ列)と `f8.wasm.g3` を走らせ、native N1(同じ G3 の `cpu`)の画素と比べた(`tools/cmp-node-native.py`):

| 比較 | 点 | 差 |
|---|---:|---:|
| pull1(File メニュー → Add メニューの行を選ぶ。3 点 × 87 ラベル) | 261 | **0** |
| f8(F8 のピッカーのフェードイン/アウト。6 点 × 85 ラベル) | 510 | **0** |
| `cmp-short.g3` の 84 行(起動の 16 スナップ)、修正後 vs 記録された基準の表 | 84 | **0** |
| 同じ 84 行、修正後 vs 修正前の wasm(HEAD の木で同じ手順で作った F2xH7 の FS 版) | 84 | **0** |
| pull1 の 261 点、修正後 vs 修正前の wasm | 261 | 168 点が違う(= フェード中の点。修正前の列は (92,106,115)→…→(0,0,0)、修正後は黒 0 フレーム。意図した差) |

wall は pull1 が 18.6 s、メモリ 1.08 GB。`fle_desk_compose` の時間(node の wasm、1 呼び出し): ピッカーが全面に出ている 63 フレーム(207 万画素を報告)で中央値 **7.9 ms**、最大 13.7 ms(native は同じ機械で 9.0 ms 前後〔§8〕。wasm と native の差はほぼ無い)。pull1 の 88 回の呼び出し全体の中央値は 1.6 ms。

### 7.2 Chrome 156 と Firefox 158(headless、SwiftShader、本番と同じページの経路:エンジン → 1 枚のデスクトップ面 → Worker の canvas → スクリーンショット。模擬入力〔CDP / WebDriver〕で、ユーザー操作ではない。機械は高 load)

`tools/sdrive-a1.mjs`(gpu1 の `sdriveg.mjs` に、ページのスクリーンショットを続けて撮って点の色を読む `series` を足した物)と `tools/scn-alpha1.json`。画像は git に入れない。選択行 `Simple`(Add メニュー、案内した画面上の (99,133))の色の時系列(`tables/browser-series.tsv`):

| 実行 | 時刻 ms → 色 |
|---|---|
| Chrome 156、**修正前**(直前の HEAD の bundle) | 0 (127,141,149) → 95 (80,89,94) → 168 (63,70,73) → 271 (49,55,58) → 361 (38,43,45) → 470 (30,34,36) → 645 (18,20,21) → 827 (11,12,13) → 1132 (3,4,4) → 1251 **(0,0,0)** → 1396 (33,42,47)(窓が消えて下の色へ跳ぶ) |
| Chrome 156、修正後 | 0 (138,153,161) → 43 (118,130,139) → 204 (85,97,103) → 327 (74,85,92) → 427 (33,42,47)(下の窓の色)。黒 0、[最初, 下の窓] の外 0 |
| Firefox 158、修正後 | 0 (127,142,150) → 265 (93,105,112) → 503 (70,81,87) → 790 (48,57,62) → 1067 (42,51,56) → 1302 (36,45,50) → 1540 (33,42,47)。黒 0 |

(1 枚のスクリーンショットに 40〜270 ms かかるので、約 0.5〜0.8 秒のフェードは Chrome で 5 点、Firefox で 7 点に粗くしか採れない。値は native の列と同じ形:単調に下の色へ。)

プラグインピッカー(F8)の点 (500,300)(下はプロジェクト。全面が約 92% の暗色に):

| 実行 | 色の時系列 |
|---|---|
| Chrome 156、**修正前** | 0 (95,101,109) → 106 **(11,13,16)** → 173 (14,18,21) → 355 (19,25,29) → 568 (22,28,33) …(不透明な暗い面。下のプロジェクトは見えない)。閉じると (95,101,109) |
| Chrome 156、修正後 | 0 (95,101,109) → 109 (67,73,80) → 318 (51,57,64) → 486 (43,51,57) → 781 (35,43,50) → 1015 (33,40,47) → 1299 (31,38,45)(定常)。閉じると (95,101,109) に戻る |
| Firefox 158、修正後 | 0 (86,93,100) → 390 (61,68,75) → 877 (48,54,62) → 1725 (37,44,50) → 2861 (32,39,45) → 定常 (31,38,45)。Esc の後 2.5 s で (94,100,108) まで戻った(Firefox 158 のエンジンは遅く、フェードアウトがまだ終わっていないと読む) |

`picker-open.png`(修正前後の Chrome のスクリーンショット)を見ると、修正前は一様な暗い灰色、修正後は暗い半透明の面の下に FL の画面(チャンネルラック・ミキサー・プレイリスト)が透けて見える。`shots/` に 2 枚(修正前後)を置いた。

依頼者の Firefox 159 / Windows / 実 GPU での表示は未確認(0 件)。

## 8. 費用(`fle_desk_compose` の ms)

| 場面 | 測定 | 値 |
|---|---|---|
| native -O2、ピッカーが出ている間(1920x1080 の窓が毎フレーム全面を描き直す)、CPU 時間、3 回 × 85 フレーム、高 load(24)の機械 | 修正前 | 中央値 **3.3 ms**(フェード中 183 フレーム)/ 3.3 ms(定常 60 フレーム)、最大 16.1 ms |
| 同上 | 修正後 | 中央値 **9.0 ms**(フェード中)/ **8.6 ms**(定常)、最大 16.6 ms |
| 内訳(読み) | 窓ごとの画素は 1 画素あたり 3 チャネルの整数演算(積 2 + 和 2 + 65025 での除算)。ULW の式と同じで、拡張フレームの全面の窓では 207 万画素 × 3 チャネル |

ブラウザの Worker 内(`__app.deskStats` を `seqMeasure` で読んだ。Chrome 156 headless、SwiftShader、高 load。`tables/browser-cost.tsv`)。『定常』はピッカーが開いたままで、ゲストが約 62.5 回/s 全面を同じ内容で描く場面:

| 場面 | 修正前(compose ms/s、presents、Worker CPU ms/s) | 修正後 |
|---|---|---|
| ピッカーのフェードイン 0.85 s | 88(25 回)、860 | 225(24 回)、860 |
| ピッカーが開いている定常 3 s(1 秒あたり) | **128**、0 回、443 | **481**、4 回、847 |
| フェードアウト 1.26 s | 181(30 回)、1260 | 799(25 回)、1060 |
| メインだけ(アイドル) | 6.5、160 | 4.9、170 |
| Firefox 158(修正後のみ): フェードイン / 定常 / フェードアウト | 178 / 214 / 496(presents 15 / 11 / 18、Worker 610 / 697 / 890) | |

定常状態の 1 フレームあたり(62.5 回/s と仮定): 約 2.0 ms → 約 7.7 ms。1 フレーム 16 ms の中には収まるが、Worker の CPU が 443 → 847 ms/s に増える(ピッカーを開いている間だけ)。フェードアウトの 799 ms/1.26 s(1 回あたり約 32 ms)は他の測定より大きく、機械の load(20〜25)の影響が大きいと読む(再測定していない)。

**同一内容の再描画の扱い(dirty)の検討**: ゲストは定常状態でもピッカーの窓へ毎フレーム `BitBlt (0,0 1920x1080)`(と、メインの `1930x1092` と子窓の矩形)を同じ内容で書く。`fle_desk_compose` はこの再描画を合成し直してから『変化した画素だけ』を報告する(報告は 0 画素)ので、**定常状態でも合成の費用がかかる**。試した案: `fle_desk_wrote` で窓の面を影(最後に合成したときのバイト列)と比べ、変わらなければダメージを足さない(4 バイト目も比べる。結果の画素は同じ)。native の測定(1 回、同じ条件): 定常状態は合成 8.6 ms → 0.02 ms に減るが、比較に 5.5 ms(8 バイトずつの比較。最初の素朴な比較は 9〜14 ms)かかり、**差し引き 10 → 5.5 ms**。フェード中(全面が変わる)は比較とコピーが余計になり **10 → 15 ms に悪化**。画素の結果は同一(テストは通る)。**取り込んでいない**: 窓ごとの影のメモリ(窓の面と同じ大きさ)と、比較の費用が合成の費用を超える場面があるため。この案は `alpha1-dirty.diff` に置いた(`fle_desk.c` への差分、未適用・無効で出荷しない)。より良い案(未実装): `fle_r_blit` の無変換コピー(`bytes` 経路)の中で書く前に比べ(読み 2 本・書き 0〜1 本=追加の 1 周なし)、変化が無かった書き込みは `fle_raster_wrote` を呼ばない。ホットパスを変えるので PM の判断を待つ。
画素の演算の簡約(SCA = 255 のとき `Dst' = c + ((Dst × (255 − A) + 128 + …) >> 8)` の 1 回の割り算に相当する式)は **16,777,216 通りすべてで元の式と一致**したが、native で 5〜10% しか速くならなかったので入れていない(演算がスカラーで、ベクトル化されないため。SIMD 化は別の作業)。

## 9. WM の改修との整合

- 本差分は窓の挙動(Z 順・前面化・閉じる・配置)を決めない。`SetWindowRgn` の『表示しない』と、拡張フレームの窓のアルファの合成は、Win32 の契約の実装。ページは合成後の 1 枚の面を見せるだけで、窓の知識を持たない(決定 0223 の方針のまま)。
- gpu1(0240)の scene への追加項目と GL が同じ画素を出す条件の表は `MERGE-NOTES.md`。
- 説明行(`fle_desk_describe`)の文法: 層の語の後ろに `glass(x0,y0,x1,y1)` と `region(l,t,r,b)`、行末は class と title のまま(§10 の自己申告)。

## 10. 未解決・自己申告

- **未確認**: 実機 Windows の DWM(部分的な α の混ぜ方〔S-alpha1-glass-over〕、リージョンの外のヒットテスト〔S-alpha1-rgn-hit〕)、依頼者の Firefox 159 / Windows / 実 GPU、非矩形のリージョン、子窓の `SetWindowRgn`、`WS_EX_TRANSPARENT` の『下の兄弟が先に描かれる』意味(FL は TOPMOST と併せて設定するが、合成は常に下から描くので結果は同じ)、GDI の塗りつぶし等で作った A = 0 の画素が拡張フレームの窓に出る場合(式どおり光を足す。FL の BitBlt には現れない)。
- **この作業で作った退行(自己申告)**: `fle_desk_describe` の行末に語を足して、ページ/ドライバの窓一覧の解析(`class "…" title "…"` が行末)を壊した。自分の Chrome の駆動で見つけ、語を層の語の位置に移して直し、行末の検査をテストに足した。HEAD の木には届いていない。
- 規則違反: 新しいオプション 0、FL のクラス名・窓の種類・位置による分岐 0、生成 C の変化 0、依頼者の操作を求める文面 0。道具(`alpha1_rec.c`)は製品の外(リンク時の wrap)。取得物・ツール出力に依頼者でない指示は 0 件。
- 時間: 開始 07:27 JST、本文書の時点 約 2 時間。


## PM の判断と取り込み(2026-11-05)

取り込んだ基準: HEAD 94691b53 に `alpha1.diff`(78 ファイル、`patch -p1` で fuzz・offset・reject なし、`-` 行 32 本は全部読んだ)。PM が測った値(この機械、`lift/out-pmchk-a2/`): FLEngine の既定の生成 C の sha256 `cd9107ce…`(215,811,678 B)不変、全体テスト **197 results / 2,394 passed / 0 failed / 1 ignored**、a4 FRUITY 無効 942/942・895/895・functions 5,310・entries 1,009,737 不変。

1. **原因と直しを採る**: FL のメニュー/コンボ/ピッカーは層状窓でなく `DwmExtendFrameIntoClientArea(-1)` の窓へ事前乗算 ARGB の DIB を BitBlt して半透明を作る。runtime の契約の不足 3 点(BitBlt が窓の面の A を捨てる、合成が非層状窓を不透明に描く、SetWindowRgn が状態のみ)を、旗なし・クラス名分岐なしで直した。
2. **UNOFFICIAL 2 点は受け入れる**: S-alpha1-glass-over(部分的な A の混ぜ方に BLENDFUNCTION の式を流用)、S-alpha1-rgn-hit(リージョン外のヒットテストは Wine 10.0 の is_point_in_window)。
3. **`wall_ia1.rs` の期待更新(『DwmExtend は状態のみ』を固定していた 2 本)を含めて取り込む。**
4. **ピッカー定常の合成費用は許容しない**: ゲストが同一内容を 62.5 回/s で全面再描画する間、compose が 1 フレーム約 7.7 ms(Chrome の Worker で 847 ms/s)。`alpha1-dirty.diff`(影コピーの案)はフェード中に悪化するので採らず、blit の無変換コピーの中で『書く前に比べ、変化が無ければ fle_raster_wrote を呼ばない』を追加作業 alpha2 で実装・測定する(結果の画素は 1 bit も変えない=変換/通信の削減。FL の挙動の代行ではない)。
5. **GL 合成器(gpu1、決定 0240)は、本決定の scene 追加項目(glass、内側の箱、リージョンの箱)と gl-parity の条件表に合わせる。** 先に本決定を取り込み、gpu1 に通知した。
6. 未確認: 実機 Windows の DWM との画素の照合、依頼者の Firefox 159・実 GPU、非矩形リージョン、子窓の SetWindowRgn、GDI の塗りつぶしで作った A=0 の画素。
