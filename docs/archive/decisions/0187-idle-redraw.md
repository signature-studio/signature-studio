# 0187: アイドルで主窓が毎フレーム描き直される原因 — 行(層 2)の DwmIsCompositionEnabled=TRUE でエンジンが意図して行う 1 画素幅の再描画。実機の FL でも同じ分岐に入る可能性が高い(読み・未確認)。表示側の厳密な省略 `?dedupe=1`(opt-in、app/ のみ)でページの負荷を下げる

状態: **仮の決定(すべての選択)**。依頼は PM(番号 0187)。調査・実装・測定・文書は作業者 idle1(2026-11-03 17:45〜、予算 3 時間)。作業と測定は HEAD **88a4dfe** の `git archive`(`lift/out-idle1/src/`、`app/` は 4f23c01〔決定 0182〕から変わっていない)と、native の実エンジン(prune2 の D3 の objects を読み取りで借りた物と、自分で生成・コンパイルした物)。成果物は `lift/out-git/idle1b-results/`(**`idle1-results/` は 11-01 の別の作業の物で、この作業は触っていない**)。`idle1b.diff` は `app/` の 10 ファイルだけを変える(`lift/`・wasm・生成 C・ランタイム・層 2 の行・既定の入口・`lift/out-websrv/` は変えていない)。

区分の凡例: **実測**＝このセッションのコマンドの出力を見た事実。**読み**＝コードや出力を読んで得た解釈で、確かめていない。**未確認**＝確かめていない。**出典なしの選択**＝`S-idle1-…`。**実機 Windows との照合は 0 件**。この機械は SwiftShader の 16 コアで、他の担当の仕事で load が 6〜23(各表に併記。壁時計の速度は主張しない)。G 段階は変えない。

## 冒頭の三点(正直さ)

1. **何を実測したか(実行した本物と範囲)**: (a) 隔離 headless Chrome 156 で、fin1 のバンドル 8 本(D, Dp, F1, F1x, F1xn, F2, F2x, F2xn)の**実エンジン**を、同じ手順(ウィザード 8 s → Enter → 入力なしの主窓 20 s、`?hz` 無し)で動かし、アイドルの窓の更新/s と Worker・ページのコアを測った。F1xn に putImageData の計測を付けた run も 1 本。(b) native -O0 の**実エンジン**(行あり=prune2 の D3 の objects に自作の計数ラッパをリンク、行なし=HEAD から自分で生成・コンパイル)を G3 ハーネスで動かし、API 呼び出し数・面への書込・shadow stack・ゲストメモリの変化・関数入口数を測り、`DwmIsCompositionEnabled` だけを FALSE に置き換える介入を行った。(c) `?dedupe=1` と自己検証 `?dedupe=check` を実装し、Chrome の実エンジンで CPU の交互測定(60 Hz 5 組、120 Hz 5 組)、入力応答、約 13.7 分の自己検証(ウィザード・メニュー・右クリック・ダイアログ・再生 300 秒)を行った。**代役・モック・画像の再生・事前生成は無い**。
2. **結論**: 毎フレームの描き直しは、層 2 の行(`DwmIsCompositionEnabled`=TRUE)で有効になる**エンジン自身の機能**で、QFC のスレッドが 16 ms ごとに送る WM_APP+0xFFC を受けたエンジンが、主窓の左パネルの右端の 1 画素幅(x=377, y=152〜1022)を RDW_INVALIDATE し、WM_PAINT の BitBlt で同じ画素を毎回描き直している(行なしでは GetProcAddress が NULL を返し、機能が起動時にオフになる)。native の介入実験(DwmIsCompositionEnabled だけ FALSE)で書込 63/s → 0〜1/s。**ランタイムの応答の副作用ではなく、実機(Win8+ のマニフェスト)で同じ分岐に入る可能性が高い**(Learn の根拠・読み。実機照合 0、`[obj+0x49e]` の既定値は未確認)ので、応答は変えず、表示側の厳密な省略 `?dedupe=1`(opt-in)で負荷を下げた: 窓の更新 62.5 → 0.85/s、ページ主スレッド −0.057 コア(5/5 組)、Chrome 全体 −0.106 コア(120 Hz は −0.070・−0.123)。Worker(ゲストの再描画 約 66,000 関数入口/フレーム=アイドルの入口の 1.93 倍。99.6% は p1000r が lower する関数)は変わらない。
3. **未確認**: 実機の Windows での挙動(実機の FL がアイドルで 1 画素幅の無効化を毎フレームしているか、`[obj+0x49e]` の既定値、再描画の目的、周期が実機のディスプレイの更新周期に比例するか)。Chrome の 2D canvas の `contextlost`/`contextrestored` 後の再同期(`?dedupe=1` の shadow は canvas の内容を前提にするので、既定 ON にする前に要る)。`?dedupe=check` の比較は 1.5 秒ごと(フレーム単位ではない)。120 Hz 以上の実ディスプレイ、依頼者の機械、所定のプロジェクト(`serina1-serum2.zip`)での値。

## 1. 方法

- **Chrome(実エンジン)**: 隔離 headless Chrome 156(`app/test/e2e/browser.mjs`、プロファイル使い捨て、127.0.0.1 以外は名前解決不能)を 1 本ずつ、`nice -n 5`。ページは p2a(決定 0182)の `dist-after`、または本差分のビルド(`vite build` の `dist`)。バンドルは `lift/out-websrv/assets/` の `fl-engine-fin1-*` と `fl-engine-b1-D`(読み取りのみ。サーバーは触らない)。手順は p2a の `p2a-run.mjs` の写し(`tools/idle-run.mjs`): 最初の idle → ウィザード 8 s → Enter → 落ち着くまで → 入力なしの主窓 20 s。窓の更新/s = ページが窓の面に描いた回数(`PerfRecorder.update`)、CPU = `/proc/<pid>/task/<tid>/stat` のスレッド別(100 Hz の刻み)を区間の実時間で割った『コア数』。`?hz=` 無し=エンジンの 60 Hz。**この機械の load は他の担当の仕事で常に 6〜23**(各 run の開始時の load を表に併記)。
- **native -O0(実エンジン、G3 ハーネス)**: 行あり = prune2 の `D3`(決定 0178 の層 3 の行: ia1 の層 1+2 + dlg1)の native オブジェクトのハードリンク(読み取り)に、自作の計数ラッパ(`tools/nat/idle1_probe.c`、`gen_wraps.py`: 716 の `fle_imp_*` を `--wrap` し、API 呼び出し数、書込 `fle_raster_wrote` の lid・矩形・shadow stack、関数入口ごとのゲストメモリ監視、DwmIsCompositionEnabled の介入を足す)をリンクした物。行なし = HEAD(`git archive`)の runtime・harness と `fe-lift`(wrp1 のビルド。既定の FLEngine C の SHA-256 が `cd9107ce…`〔215,811,678 B〕になることを確認)で自分で生成(`tools/cycle.sh N none`、FLEngine と 5 DLL、`gen_two.sh gen split merge`)・コンパイル(83 単位、-O0 -finstrument-functions)した物。同じ G3 手順(`tools/idle2.g3` ほか: 最初の idle → 7 s → Enter(VK 13)→ 6 s → 計測区間)。**行なしの生成 C の FLEngine は `gen_two.sh` の FRUITY 設定のもの(`6d39d04f…`)で、既定の `fe-lift lift` の `cd9107ce…` とは違う**(seeds・hook・SEH が入る)。
- **読み(逆アセンブリ)**: `llvm-objdump -d -M intel` で `binaries/win64/FLEngine_x64.dll`(ImageBase 0x400000、VA = そのまま)。`tools/callers.py`・`tools/xref.py` で直接の呼び元・rip 相対の参照を探した。
- **実機の Windows は無い。実機との照合は 0 件。**

## 2. 構成ごとのアイドルの窓の更新(Chrome 156、実エンジン)

`tables/chrome-idle-by-bundle.md`(1 run ずつ。`?hz=` 無し、空のプロジェクト)。

| バンドル | 行(EXTRA_IMPORTS) | ウィザード 8 s の更新/s | 主窓アイドル 20 s の更新/s | Worker コア | ページ主スレッド コア | 全 Chrome スレッド コア | load |
|---|---|---|---|---|---|---|---|
| D (b1) | なし | 13.08 | 0.7 | 0.141 | 0.013 | 0.177 | 7.3 |
| Dp (D′) | なし | 13.08 | 0.8 | 0.102 | 0.014 | 0.142 | 6.9 |
| F1 (prune S + Blend2D 置換) | なし | 5.98 | 0.8 | 0.106 | 0.013 | 0.142 | 7.0 |
| F2 (prune S+M + 置換) | なし | 10.1 | 0.55 | 0.099 | 0.014 | 0.137 | 7.6 |
| F1x (F1 + ia1-imports.txt) | あり | 68 | **62.51** | 0.167 | 0.074 | 0.311 | 6.6 |
| F1xn (F1x + 0174 = 既定の入口) | あり | 67.05 | **62.48** | 0.168 | 0.075 | 0.312 | 6.7 |
| F2x | あり | 68.2 | **62.49** | 0.170 | 0.078 | 0.319 | 5.7 |
| F2xn | あり | 68.34 | **62.48** | 0.162 | 0.068 | 0.296 | 7.6 |

**毎フレームの描き直しは『F1 → F1x』(= 行を足したとき)で始まる**。Dp→F1(prune S と Blend2D 置換)、F1x→F1xn(決定 0174)では変わらない。F2 系も同じ。行なし(D・Dp・F1・F2)は静か、行あり(x を含む構成)はウィザード中も毎フレーム。仮説 H3(Blend2D 置換)・H4(0174)・H5(rz1)・H6(prune)は除外、H1/H2(行)が残った(次節で行の中の Dwm 行と特定)。

**アイドル 15 s の putImageData の計測**(F1xn、`tools/idle-run.mjs --hook 1`、CPU 計測とは別 run): 941 回の更新のうち **932 回が主窓(1930x1092)の同じ矩形 x=377, y=152, 幅 1, 高さ 870、画素のハッシュは 932 回すべて同一**(`tables/chrome-hook-putimagedata-F1xn.txt`)。残り 9 回はウィザードを閉じた直後の別の矩形(ハッシュ 4 通り)。

## 3. 原因の台帳(native -O0 の実エンジンでの実測と、FLEngine_x64.dll の読み)

1 秒(仮想)あたりの面への書込(`tables/native-surface-writes-per-virtual-second.tsv`、Enter の 6 s 後からの区間): 行あり 62〜64 回/s(すべて lid 14 = TFruityLoopsMainForm)、行なし 0〜1 回/s、行あり+DwmIsCompositionEnabled だけ FALSE 0〜1 回/s。1 秒間の全 API 呼び出し(`tables/api-calls-rows-vs-norows-1s.tsv`、仮想 3618〜3619 s)の差: 行ありに RedrawWindow・GetUpdateRgn・BeginPaint・GetDC・BitBlt・EndPaint・ReleaseDC が各 +62、SendMessageW +1,008、IsWindowVisible +756、IntersectRect +740 ほか(WM_PAINT の走査)。PostMessageW は行あり・行なしとも 63/s で同じ。

| 段 | 何が | 場所(VA は FLEngine/QFC の guest VA。shadow stack は実測) | 行なし | 行あり |
|---|---|---|---|---|
| A | 起動: `[0x16bca68] := (Self->0x970->0x49e ≠ 0 かつ DwmCompositionEnabled()) − 1` | 0x135cdb0(呼び元 0x11acb54, 0x11d2099)の 0x135cfcf。`DwmCompositionEnabled` = 0x65c630(関数ポインタ `[0x15f4198]` = GetProcAddress("DwmIsCompositionEnabled") があれば呼ぶ。HRESULT<0 か BOOL=0 なら FALSE) | ポインタ NULL(`[0x15f4198]` = 0、実測)→ FALSE → −1(機能オフ)。メモリ監視: t=3600960 ms に 0→−1(直前の入口は 0x65c630) | `[0x15f4198]` = 0x7ff900002790(非 0、実測)→ TRUE → 0(機能オン) |
| B | 毎フレームの刻み: quickfontcache_x64.dll(m3)のスレッド(開始 0x1d0fdb0)が `Sleep(16)` → `PostMessageW(0x7f990018, 0x8ffc)` | Sleep: サンク 0x1d1da70 ← 0x1df1db0(Sleep の薄い包み)← 0x1e85b70(QFC のペース取りの関数: `[obj+0x70]` ms を Sleep する。実測は 16 ms〔60 Hz〕。値の出所は追っていない=読み)。PostMessageW: サンク 0x1d1e0c0 ← 0x1e860a0。63/s | 同じ(63/s) | 同じ(63/s) |
| C | 主スレッドで QFC の wndproc が受け取り、エンジンのコールバックを呼ぶ | 0x1e858e0(wndproc、アタム 0xc004 のメッセージ専用窓)→ 0x1e86280 → **0x11fc770**(896 回/14 s)。0x11fd787〜0x11fd7a1: `if ([0x16bca68] >= 0) [0x16bca68] = (*0x1403bd8 >= 0)`(`*0x1403bd8` の値は 2 のまま) | −1 のまま | 0 → 1(メモリ監視で各フレーム) |
| D | メッセージループの 1 周ごと(1 フレームに 2 回)に | 0x86a250 → 0x86b440 → **0x11f0270**(1,791 回/14 s): カウンタ>0 かつ `[0x16bcc00]`≠0 かつ `[[0x16bcc00]+0x4c2]`≠1 かつ `*0x16baaf0`=0 のとき 0x11f039c で `RedrawWindow(form hwnd = [rbx+0x45c], &rect, NULL, 1 = RDW_INVALIDATE)`、rect = パネル(`[rbx+0xb30]` か `[rbx+0xb18]`)の右端の 1 画素幅(left = right−1)、top+64、bottom−64。0x11f03a1 でカウンタ=0 | 呼ばれてもカウンタ≤0 で戻る | 1 → 0(RedrawWindow の直後) |
| E | WM_PAINT | BeginPaint ← 0x8261a0 ← 0x806b70(窓の描画ハンドラ)。BitBlt(0x6bb1b0 ← 0x8261a0 ← 0x806b70 ← 0x410090 ← 0x60c3e0)で lid 14 の (377,152)-(378,1022) に 1×870 画素 | 1 回/s(別の理由) | 63 回/s |

**介入実験**(実測): 行ありのビルドで、`fle_imp_DwmIsCompositionEnabled` だけを FALSE に置き換える自作ラッパ(`FLE_IDLE1_DWM=0`)で動かすと、起動後のカウンタは −1、書込は 0〜1 回/s(行なしと同じ)になった(`runs/ov0c`、制御の `runs/ov1` は 62〜64 回/s)。ゲスト側の差は『DwmIsCompositionEnabled が TRUE を返すか』だけ。

**Chrome の画素でも一致**: 932/941 回が 1×870 の同一画素の矩形(上)。

## 4. 正しい挙動か、ランタイムの応答の副作用か(判定)

**判定: 副作用ではなく、エンジンのコードが『DWM 合成が有効』のときに意図して行う再描画。実機の FL でも同じ分岐に入る可能性が高い(読み)。ただし実機照合は 0 件で、断定はしない。**

根拠(実測と読み、出典):
1. **実測**: 起動時に 1 度だけ『DwmCompositionEnabled() の答え』が 1 個のカウンタ `[0x16bca68]`(合成が有効 → 0、無効 → −1)に入り、−1 のときは毎フレームの再描画が起きない(メモリ監視、介入実験)。応答の値で分岐する明示的なコードであり、ランタイムが『誤った値を返してエンジンが誤った経路に入った』形ではない(層 2 の行は、エンジンが存在を前提に遅延ロードする `DwmIsCompositionEnabled` を実在の関数として答えるだけ)。
2. **Learn(Microsoft)**: `DwmIsCompositionEnabled`: 『As of Windows 8, DWM composition is always enabled. If an app declares Windows 8 compatibility in their manifest, this function will receive a value of TRUE through pfEnabled. If no such manifest entry is found, ... FALSE』。FL64.exe のマニフェストは Windows 8 と 8.1 の supportedOS を宣言している(決定 0171 が読んだ事実)ので、実機の FL は TRUE を得る。`fle_dwm.c` の TRUE はこれに従った選択(S-ia1-dwm-1)。
3. **行なしの『静かな』側は実機にない環境**: 行がないと `GetProcAddress(dwmapi, "DwmIsCompositionEnabled")` は NULL と ERROR_PROC_NOT_FOUND(`fle_modules.c` の応答表に無いので)。これは『dwmapi にその関数が無い OS(Windows XP など)』の応答で、Vista 以降の実機では起こらない。行なしの基準(16 スナップの 82 行の D・Dp・F1・F2 の表、行なしのバンドル全部)は、この非現実的な側で測られていた(行ありの基準は決定 0171/0176 の別の表)。
4. **未確認**: (a) `[Self->0x970 + 0x49e]`(ユーザー設定の可能性がある 1 バイト)の実機の既定値。この機械の実行ではこれが非 0 だった(DwmIsCompositionEnabled=TRUE の側で機能がオンになったので)。値の出所(エンジンの既定値か、仮想レジストリ・ファイルの種〔`reg-seed-*.tsv`、`vfs-seed-idle1.tsv`〕か)は追っていないので、実機の既定とは限らない。(b) この再描画の**目的**(DWM に合成を続けさせるための『保活』か、境界線の描画か)。エンジンの文字列に説明は無い。(c) 実機でこの 1 画素幅の無効化が毎フレーム起きているか、実機のスクリーンショットでの確認。(d) 周期: QFC のペース取り(0x1e85b70)は `[obj+0x70]` ms を `Sleep` する(実測 16 ms = 60 Hz で 62.5/s)。p2a の `?hz=120` で 125/s になったのは Sleep(8) に相当する周期(読み、未確認)で、実機のディスプレイが 144 Hz なら 1000/6 ≒ 166 回/s の無効化になるはずだが、確かめていない。

**帰結**: 『アイドルの描画は必須』で、重いなら描画を速くするしかない側(PM の方針 (b))。実機の FL も同じ無効化をしている可能性が高いが、実機の Windows では 1 画素幅の `BitBlt` が安いだけで、エンジンの仕様は同じ。ランタイムの応答は変えない(PM の判断)。

## 5. ゲスト側の費用

同じ行ありのビルドで DwmIsCompositionEnabled=TRUE と FALSE(介入)を、Enter の 6 s 後からのアイドル 14 仮想秒で比べた(native -O0、`g3-snap` の関数入口数): **TRUE 8.78 M/s、FALSE 4.54 M/s(1.93 倍)**、1 回の再描画あたり約 **66,000 関数入口**(`tables/guest-extra-entries-per-redraw.tsv`)。1 回の再描画に含まれる BitBlt は 1 回(1×870 画素)で、費用の主体は WM_PAINT の走査(SendMessageW +16/フレーム、IsWindowVisible +12、IntersectRect +12 ほか)と Delphi の RTL ヘルパーの反復。差の 90% は 432 関数のうち 8 関数(0x50aa70, 0x410010, 0x40fe90, 0x6113f0, 0x5d76b0, 0x863f20, 0x5287c0, 0x528f20。各フレーム 約 7,100〜9,400 入口)。cov の合計は 120×2^20 と 60×2^20 ちょうどになる(計数の丸めの可能性。分布は参考、総数は `g3-snap` の値を使った)。

→ **BitBlt の省略(ランタイムで『書き込み先が既に同一なら書かない』短絡)は意味が薄い**: BitBlt は 66,000 入口中の 1 呼び出し。実装せず、検討だけ。ゲストの CPU を下げるなら、エンジンが走る側を速くする(下のハイブリッド)か、再描画の要求自体を無くす(応答を変える=PM の判断で行わない)しかない。

**直接 wasm の混成(決定 0185 の p1000r)との突き合わせ**(`lift/out-hyb4/keep/p1000r-lowered-all.txt`、994 関数): 上の 8 関数は**すべて** p1000r の lower 済みの一覧にある(FLEngine の 714 関数の中)。1 フレーム分の追加入口の **99.61%**(62,915,099 中 62,672,542)、アイドル全体の入口の 98.44% が lower 済みの関数に入る。lower されていない大きな物は 0x40c660(41,048)、0x425380(14,798、SendMessageW のサンク)、0x868c00、0x4250a0 ほか小さな物と、再描画の起点 0x11f0270・0x86b440・0x11fc770(入口数は少ない)。決定 0185 は p1000r の steady の主スレッド CPU を D 比 −42%(node)・ワーカー本体 −30%(Chrome)と測っているので、**アイドルの再描画のゲスト CPU を下げる最短の道は p1000r の採用**(読み。Chrome での効きは小さく出た前例があり、F1xn の上での測定は未実施)。

## 6. 修正: 表示側の厳密な省略 `?dedupe=1`(opt-in、`app/` だけ)

**方針(PM の指示)**: ランタイムの応答は変えない。ブラウザへの転送の前に、dirty 矩形の画素が『ページの canvas の現在の内容と**語単位で同一**』なら、変換も `postMessage` も `putImageData` もしない。近似はしない。

**設計**(`app/src/engine/dedupe.ts`、`worker.ts` の `draw()` に 1 か所):
- ウィンドウ(sink)ごとの **shadow** = 最後に送った源の 32 ビット語(B, G, R, 第 4 バイトまで)を、その窓の大きさの密な配列に持つ。
- **有効マップ**: 32×32 のタイル。タイルは『それを**完全に**覆う矩形が 1 回送られた』後にだけ有効。一度も描かれていない canvas(透明な黒)を、全語 0 の源(不透明な黒)と取り違えないための条件。幅 1 の縦線だけが何度も送られても、そのタイルは有効にならない。
- 比較は、矩形の全タイルが有効で、面積が `COMPARE_MAX_PIXELS`(262,144 = 512×512)以下のときだけ。語が 1 つでも違えば送り、shadow を更新する。上限より大きい矩形は比較せず送り、shadow に写す(タイルは有効になる)。
- **全消去の条件**: 窓の大きさの変更、layer 署名(alpha・色キー・premultiplied)の変更、`Sink.resize` が呼ばれたとき(`Presenter.pump` が新しい窓・layout 世代の変化・layer の変化で呼ぶ。**方式 A は OffscreenCanvas をその場で消し、方式 B はページが次のフレームで消す**——同じ大きさでも消える。その結果 shadow は canvas の内容を表さなくなるので、`resetShadow` で消し、窓全体を送り直す)、アラインしていない・メモリ外の面。
- `?dedupe=check`(自己検証、方式 B): すべての矩形を従来どおり送って描く。shadow が『不変』と判定した矩形だけは、ページが持つ**参照用 canvas** に描かない(`frame.omit`)。実 canvas と参照 canvas は、省略が厳密なら常に同一でなければならず、`window.__app.dedupeCheck()` が窓ごとの違う画素数を返す(Chrome の検証用。本番の経路では使わない)。
- 実装の**発見**(テストを書いて見つけた不具合、修正済み): 最初の版は layout 世代だけが変わった(窓が動いた)同じ大きさの窓の全面送り直しを『不変』と判定して省略し、`pendingResize`(方式 B)が残って、次の小さなフレームでページが canvas を消して小さな矩形しか描かない形になりうる(方式 A では `resize()` が canvas を消した後に省略されて空になる)。`dedupe-presenter.test.ts`(実 `Presenter` と表の書き手の JS モデル、方式 A/B の canvas のモデル)で再現できる形にして、`resetShadow` を足した(その変異を検出する)。**check モードの最初の約 100 s の検証(窓が動かなかった)はこの不具合を見ていなかった**。

**費用**(node のマイクロベンチ、V8、この機械、`tables/bench-dedupe-node.txt`): 比較 1×870 の線 2.0 μs、64×64 2.6 μs、256×256 46 μs、512×512(上限)174 μs。上限超えの矩形の写し(1024×600)72 μs、窓全体(1930×1092)の写し 249 μs。shadow のメモリ = 窓の画素×4 バイト(主窓 8.4 MB、ウィザード 2.0 MB ほか。Worker の JS ヒープ)。省かれる費用は 1 矩形あたり 変換(`convertRect`)+`rgba.slice()`+`postMessage`+ページの `putImageData`+合成。

**出典のない選択(S-)**:

| 番号 | 選択 | 理由・代替・撤回条件 |
|---|---|---|
| S-idle1-tile | タイル 32×32 | 正しさはタイルの大きさに依らない(小さいほど、大きな再描画の後に細い線の省略が早く効く。大きいほど旗が少ない)。代替: 16、64。撤回: 効きが悪いと分かったとき |
| S-idle1-cap | 比較は 262,144 画素まで | 比較の費用は写しと同じ桁。それ以上の矩形は全面の再描画で、同一なことは稀。代替: 無制限。撤回: 大きな同一矩形が多いと分かったとき |
| S-idle1-exact | 矩形全体が一致したときだけ省略(部分の一致で矩形を縮めない) | PM の『厳密』の解釈。代替: 差のある外接矩形だけを送る(さらに省けるが比較と複雑さが増える) |
| S-idle1-words | shadow は変換前の源の語(第 4 バイトまで)を持つ | 変換と canvas の描画は『源の語+layer 署名』だけの関数なので、語が等しければ canvas の内容も等しい。第 4 バイトだけが違う場合は送る(安全側) |
| S-idle1-check | check モードの比較は 1.5 秒ごと、全窓の全画素 | 各フレームではない(重い)。差がフレームの間だけ現れて消える形は見えない(下の未確認) |
| S-idle1-optin | 既定 OFF | PM の判断(§8) |

## 7. 検証(実エンジン=隔離 headless Chrome 156、この機械、load は他の担当で 9〜17)

### 7.1 アイドルの CPU(F1xn = 既定の入口のバンドル、60 Hz)— `?dedupe=1` あり/なしの交互 5 組 + 行なし(F1)3 本

**ビルドは最初の版(`resetShadow` を足す前。アイドルの経路は同じ)**。`tables/cpu-alternating-60hz-F1xn.md`:

| variant | n | window updates/s | Worker core | page main core | all Chrome threads core | load at start |
|---|---|---|---|---|---|---|
| f1 | 2 | 0.800 [0.700-0.900] | 0.107 [0.102-0.112] | 0.014 [0.013-0.015] | 0.145 [0.139-0.151] | 13.550 [12.400-14.700] |
| off | 5 | 62.520 [62.510-62.520] | 0.173 [0.161-0.186] | 0.073 [0.068-0.079] | 0.316 [0.296-0.340] | 11.000 [9.200-16.500] |
| on | 5 | 0.850 [0.800-1.050] | 0.168 [0.161-0.172] | 0.016 [0.016-0.017] | 0.211 [0.202-0.216] | 10.600 [9.600-15.000] |

Paired differences (on-n minus off-n, same position in the alternating order; negative = less with ?dedupe=1)
| pair | load off/on | updates/s off -> on | Worker core off -> on | page main core off -> on | all threads off -> on |
|---|---|---|---|---|---|
| 1 | 9.7/10.6 | 62.51 -> 0.85 | 0.186 -> 0.172 (-0.014) | 0.079 -> 0.017 (-0.062) | 0.340 -> 0.216 (-0.124) |
| 2 | 11.0/10.8 | 62.52 -> 0.85 | 0.161 -> 0.164 (+0.003) | 0.068 -> 0.017 (-0.051) | 0.296 -> 0.211 (-0.085) |
| 3 | 9.2/10.3 | 62.52 -> 1.05 | 0.164 -> 0.171 (+0.007) | 0.071 -> 0.016 (-0.055) | 0.304 -> 0.215 (-0.089) |
| 4 | 16.5/15.0 | 62.51 -> 1.05 | 0.175 -> 0.161 (-0.014) | 0.078 -> 0.016 (-0.062) | 0.327 -> 0.202 (-0.125) |
| 5 | 11.0/9.6 | 62.52 -> 0.8 | 0.173 -> 0.168 (-0.005) | 0.073 -> 0.016 (-0.057) | 0.316 -> 0.210 (-0.106) |

Worker: median paired difference -0.005 cores (3/5 pairs lower with dedupe)

page main: median paired difference -0.057 cores (5/5 pairs lower with dedupe)

all threads: median paired difference -0.106 cores (5/5 pairs lower with dedupe)


読み: **窓の更新は 62.5/s → 0.85/s、ページ主スレッドは 0.073 → 0.016 コア(5 組すべてで減、対の差の中央値 −0.057)、Chrome 全スレッドは 0.316 → 0.211(−0.106)**。**Worker のコアは変わらない**(0.173 → 0.168、対の差の中央値 −0.005、3/5 組で減)——Worker の費用の主体は §5 のゲストの 66,000 関数入口/フレームで、省略では減らない(省略で減るのは変換と postMessage だが 1×870 画素では μs の桁)。行なし(F1)は Worker 0.107・ページ 0.014: **`?dedupe=1` でページ主スレッドは行なし並みに戻り、Worker の +0.06 コアはゲストの再描画として残る**。

### 7.2 120 Hz(`?hz=120`)

| variant | n | window updates/s | Worker core | page main core | all Chrome threads core | load at start |
|---|---|---|---|---|---|---|
| off | 5 | 125.020 [124.930-125.020] | 0.293 [0.267-0.351] | 0.089 [0.077-0.095] | 0.458 [0.414-0.514] | 12.000 [10.400-15.000] |
| on | 5 | 0.850 [0.700-0.900] | 0.263 [0.245-0.400] | 0.016 [0.015-0.022] | 0.306 [0.286-0.457] | 11.300 [9.300-18.200] |

Paired differences (on-n minus off-n, same position in the alternating order; negative = less with ?dedupe=1)
| pair | load off/on | updates/s off -> on | Worker core off -> on | page main core off -> on | all threads off -> on |
|---|---|---|---|---|---|
| 1 | 12.0/11.3 | 125.02 -> 0.85 | 0.274 -> 0.265 (-0.009) | 0.082 -> 0.016 (-0.066) | 0.432 -> 0.309 (-0.123) |
| 2 | 10.4/9.3 | 124.93 -> 0.9 | 0.293 -> 0.250 (-0.043) | 0.089 -> 0.016 (-0.073) | 0.458 -> 0.293 (-0.165) |
| 3 | 10.6/12.9 | 125.02 -> 0.7 | 0.267 -> 0.263 (-0.004) | 0.077 -> 0.016 (-0.061) | 0.414 -> 0.306 (-0.108) |
| 4 | 12.8/18.2 | 124.98 -> 0.8 | 0.351 -> 0.400 (+0.049) | 0.092 -> 0.022 (-0.070) | 0.514 -> 0.457 (-0.057) |
| 5 | 15.0/9.3 | 125.02 -> 0.85 | 0.300 -> 0.245 (-0.055) | 0.095 -> 0.015 (-0.080) | 0.472 -> 0.286 (-0.186) |

Worker: median paired difference -0.009 cores (4/5 pairs lower with dedupe)

page main: median paired difference -0.070 cores (5/5 pairs lower with dedupe)

all threads: median paired difference -0.123 cores (5/5 pairs lower with dedupe)


### 7.3 入力の応答(単発の移動 24 回、クリック 12 回〔入力記録 36 件〕、最終ビルド、F1xn あり/なし + 行なし F1 の参照 2 本)

series runs/L1, phase taps
| run | load | n/of | input -> next frame p50 / p95 ms | engine processing p50 / p95 ms | page->inject p95 | updates/s | Worker core | page main core |
|---|---|---|---|---|---|---|---|---|
| f1a | 13.6 | 24/24 | 3.3 / 19.5 | 2.5 / 16.4 | 0.8 | 9.31 | 0.117 | 0.032 |
| f1b | 10.7 | 24/24 | 3.7 / 36.2 | 3.8 / 26.2 | 1.4 | 9.4 | 0.145 | 0.034 |
| off1 | 6.1 | 24/24 | 2.9 / 19.4 | 2.3 / 16.8 | 1.8 | 63.79 | 0.170 | 0.074 |
| off2 | 3.7 | 24/24 | 3.0 / 16.9 | 2.4 / 16.0 | 1.6 | 63.75 | 0.179 | 0.079 |
| off3 | 8.9 | 24/24 | 4.5 / 17.9 | 3.3 / 16.9 | 1.4 | 63.71 | 0.190 | 0.083 |
| on1 | 4.5 | 24/24 | 2.7 / 18.6 | 2.2 / 16.5 | 0.9 | 8.24 | 0.166 | 0.029 |
| on2 | 8.8 | 24/24 | 4.8 / 36.6 | 3.5 / 24.0 | 1.3 | 8.27 | 0.214 | 0.039 |
| on3 | 13.9 | 24/24 | 3.1 / 19.7 | 2.5 / 18.3 | 1.4 | 8.41 | 0.180 | 0.030 |

median over runs of the per-run percentile
| variant | runs | engine p50 | engine p95 | input->frame p50 | input->frame p95 |
|---|---|---|---|---|---|
| f1 | 2 | 3.1 [2.5-3.8] | 21.3 [16.4-26.2] | 3.5 [3.3-3.7] | 27.9 [19.5-36.2] |
| off | 3 | 2.4 [2.3-3.3] | 16.8 [16.0-16.9] | 3.0 [2.9-4.5] | 17.9 [16.9-19.4] |
| on | 3 | 2.5 [2.2-3.5] | 18.3 [16.5-24.0] | 3.1 [2.7-4.8] | 19.7 [18.6-36.6] |

series runs/L1, phase clicks
| run | load | n/of | input -> next frame p50 / p95 ms | engine processing p50 / p95 ms | page->inject p95 | updates/s | Worker core | page main core |
|---|---|---|---|---|---|---|---|---|
| f1a | 13.1 | 36/36 | 15.6 / 352.0 | 1.2 / 31.8 | 2.8 | 43.36 | 0.193 | 0.070 |
| f1b | 14.6 | 36/36 | 12.9 / 644.8 | 1.9 / 45.1 | 1.0 | 43.23 | 0.226 | 0.095 |
| off1 | 5.3 | 36/36 | 7.9 / 30.0 | 1.6 / 29.6 | 4.3 | 63.87 | 0.264 | 0.095 |
| off2 | 4.3 | 36/36 | 12.2 / 33.9 | 1.5 / 33.6 | 3.2 | 63.87 | 0.314 | 0.113 |
| off3 | 9.0 | 36/36 | 16.2 / 35.2 | 1.6 / 34.9 | 4.1 | 63.95 | 0.298 | 0.103 |
| on1 | 4.2 | 36/36 | 17.0 / 737.9 | 1.4 / 32.8 | 2.1 | 43.25 | 0.288 | 0.083 |
| on2 | 9.7 | 36/36 | 13.8 / 755.2 | 1.5 / 33.3 | 2.3 | 43.06 | 0.311 | 0.092 |
| on3 | 14.0 | 36/36 | 16.6 / 679.7 | 1.5 / 29.0 | 1.9 | 43.2 | 0.282 | 0.079 |

median over runs of the per-run percentile
| variant | runs | engine p50 | engine p95 | input->frame p50 | input->frame p95 |
|---|---|---|---|---|---|
| f1 | 2 | 1.5 [1.2-1.9] | 38.5 [31.8-45.1] | 14.2 [12.9-15.6] | 498.4 [352.0-644.8] |
| off | 3 | 1.6 [1.5-1.6] | 33.6 [29.6-34.9] | 12.2 [7.9-16.2] | 33.9 [30.0-35.2] |
| on | 3 | 1.5 [1.4-1.5] | 32.8 [29.0-33.3] | 16.6 [13.8-17.0] | 737.9 [679.7-755.2] |



**読み**: 単発の移動(taps)は on/off で区別できない(エンジン処理 p50 2.4〜2.5 ms、p95 16.8〜18.3 ms、input→frame p50 3.0〜3.1 ms。on の 1 本だけ load 8.8 で p95 24 ms)。クリック(clicks)は、**エンジンの処理時間は同じ**(p50 1.5〜1.6 ms、p95 約 33 ms)だが、**input→次の frame の p95 が off 30〜35 ms → on 740〜755 ms** に見える。**行なしの F1(毎フレームの更新が無い構成)の参照でも同じで、clicks の input→frame p95 は 352〜645 ms**(`?dedupe=1` は F1 と同じ形に戻る)。これは応答が遅くなったのではなく、測り方の性質: `?dedupe` なしでは窓が毎フレーム更新されるので、画素を変えない入力(ボタンの押下だけで見た目が変わらないクリックなど)にも『次の更新』が 16 ms 以内に来る(p2a が注意した点)。`?dedupe=1` ではその人工的な短さが消え、画素が変わらない入力は次の無関係な変化(数百 ms 後)まで『frame』が来ない。画素が変わる入力の frame が遅れていないことは、入力ごとの一覧で確かめた(下の L2)。**L2**(`tables/input-response-L2-per-input.md`: クリック 24 回〔入力記録 72 件〕を `?dedupe=1` なし 2 本・あり 2 本・行なし F1 1 本): 100 ms を超えて frame が来た入力は、なし 0/144 件、あり 21/144 件、F1 6/72 件。あり の 21 件はすべて button L down at screen (1100, 500)・button L up at screen (1100, 500) の押下/離し(エンジン処理 0.9〜7.5 ms=プレイリストの空の格子で、画素が変わらない入力)。それ以外の入力(ミキサー・チャンネルラック)の frame は、あり ≤ 39.2 ms、なし ≤ 36.3 ms で、遅れていない。

### 7.4 テスト

- **vitest**(`npx vitest run`、app/): 基準(HEAD の素)955 passed + 7 skipped → **1003 passed + 7 skipped**(+48: `dedupe.test.ts` 26、`dedupe-presenter.test.ts` 20、`client-init.test.ts` 2)。型検査(`tsc` 4 設定)通過。`logs/tests/vitest-final.txt`。
- **変異検査**(`test/mutation/run.mjs --only <14 件>`): 追加 14 件(単体 13、e2e 1)の結果 **14/14 検出**(`logs/tests/mutation-dedupe-all.txt`)。最初の版は 1 件(`dedupe-partly-covered-tile-marked-valid`)を見逃した(外側の y のチェックが x のチェックを隠す)ので、テストを足して検出するようにした。
- **e2e**(`node test/e2e/run.mjs`、dev・preview・worker): **952/952 checks passed**(基準 853/853、新しい場面 4c〔`?dedupe=1|0|2` のスタンドイン 5 場面〕・4d〔`?dedupe=check` の方式 A/B の 2 場面〕の分。`logs/tests/e2e-full.txt.gz`)。
- `lift/` は触っていないので `cargo test --workspace`・a4・16 スナップ・FLEngine の既定の生成 C の SHA-256 は**回していない**(変わらないはず=読み)。

### 7.5 自己検証(`?dedupe=check`)— 実エンジン、画素の一致

方式 B で、すべての矩形を従来どおり送って描き、shadow が『不変』と判定した矩形だけ参照用 canvas に描かない。1.5 秒ごとに全窓の実 canvas と参照 canvas を全画素比較(`tools/check-long.mjs`、入力は Chrome 自身の CDP `Input.dispatch*`)。**結果: 比較 549 回(約 13.7 分、最大 14 窓)で、違う画素は最大 0**。操作の列: ウィザード、Enter、アイドル、File メニュー(New / New from template / Open)、Edit・Add・Patterns・View・Options・Tools・Help のメニュー、右クリックのメニュー(チャンネル・ミキサー・プレイリスト)、F7(ピアノロール)、F5/F6/F9、チャンネルのダブルクリック、ミキサー・プレイリストのクリック、ポインタの掃引、再生 300 s(エンジンのアイドル回数が 1,500/s を超える規則で『再生中』を確認:開始時・停止時とも真)、停止。

**この範囲の限界(正直に)**: (1) **File > Save と File > Save as を押すと、この構成(F1xn)ではエンジンが終了する**(exit 134。§11)ので、Save as のダイアログ自体は比較できていない。**Save new version も同じ壁で終了する**(パス B の最初の項目)。パス C は Save 系を除いて Import と Export(とそのサブメニューの項目)を押した: Export の 4 つの項目は停止せず比較できたが、**Import の最初のサブ項目で `CompareStringW`(非 ASCII の順序)の壁により停止**した(§11)。押した後に見えた窓の一覧は各パスの表の『steps』にある。比較の内訳: パス A 299 回、B 111 回、C 139 回。(2) 比較は 1.5 秒ごとで、フレームの間だけ現れて消える差は見えない。(3) 最初の約 100 s の検証(`tables/check-first-100s-chk1.json`、62 回)は修正前の版で、窓が動かない範囲だった。

#### パス A(約 7.5 分: 再生 300 s を含む。最後の File > Save as でエンジンが終了)

run 2026-11-03T10:14:19.764Z chrome Chrome/156.0.8078.47 bundle /assets/fl-engine-fin1-F1xn/manifest.json params dedupe=check; error: None; final engine state: exited
comparisons: 299 over 449 s (every 1.5 s, all windows, all pixels); maximum differing pixels in any window at any comparison: 0

| step group | comparisons | max differing pixels | max windows compared |
|---|---|---|---|
| start | 10 | 0 | 3 |
| enter | 6 | 0 | 3 |
| idle-10s | 7 | 0 | 3 |
| file-menu-items | 14 | 0 | 6 |
| menu-edit | 3 | 0 | 7 |
| menu-add | 2 | 0 | 8 |
| menu-patterns | 3 | 0 | 9 |
| menu-view | 3 | 0 | 10 |
| menu-options | 2 | 0 | 11 |
| menu-tools | 3 | 0 | 12 |
| menu-help | 3 | 0 | 14 |
| right-click-menus | 7 | 0 | 14 |
| piano-roll-F7 | 3 | 0 | 14 |
| keys-F5-F6-F9 | 2 | 0 | 1 |
| double-click-channel | 4 | 0 | 3 |
| mixer-and-playlist-clicks | 4 | 0 | 3 |
| mouse-sweep | 4 | 0 | 3 |
| idle-after-ops-10s | 6 | 0 | 3 |
| close-wizard-if-any | 3 | 0 | 3 |
| play-start | 2 | 0 | 3 |
| play | 199 | 0 | 3 |
| play-stop | 2 | 0 | 3 |
| idle-final-10s | 7 | 0 | 3 |

frames drawn on both canvases at the end: 17311; frames the shadow called unchanged (kept off the reference canvas): 4174 (19% of all frames)
Worker's own count at the end: {'omitted': 4174, 'sent': 17307, 'comparedPixels': 583396259}

steps (engine state and the visible windows after each):
| step | state | visible windows |
|---|---|---|
| enter | engine-idle | TFruityLoopsMainForm,TApplication |
| idle-10s | engine-idle | TFruityLoopsMainForm,TApplication |
| file-menu-item-y48 | engine-idle | TFruityLoopsMainForm,TApplication |
| file-menu-item-y66 | engine-idle | TQuickPopupMenuWindow,TFruityLoopsMainForm,TApplication |
| file-menu-item-y85 | engine-idle | TFruityLoopsMainForm,TApplication |
| menu-edit | engine-idle | TFruityLoopsMainForm,TApplication |
| menu-add | engine-idle | TFruityLoopsMainForm,TApplication |
| menu-patterns | engine-idle | TFruityLoopsMainForm,TApplication |
| menu-view | engine-idle | TFruityLoopsMainForm,TApplication |
| menu-options | engine-idle | TFruityLoopsMainForm,TApplication |
| menu-tools | engine-idle | TFruityLoopsMainForm,TApplication |
| menu-help | engine-idle | TQuickPopupMenuWindow,TFruityLoopsMainForm,TApplication |
| right-click-channel | engine-idle | TFruityLoopsMainForm,TApplication |
| right-click-mixer-insert | engine-idle | TFruityLoopsMainForm,TApplication |
| right-click-playlist-track | engine-idle | TFruityLoopsMainForm,TApplication |
| piano-roll-F7 | engine-idle | TFruityLoopsMainForm,TApplication |
| keys-F5-F6-F9 | running |  |
| double-click-channel | engine-idle | TWelcomeWizard,TFruityLoopsMainForm,TApplication |
| mixer-and-playlist-clicks | engine-idle | TWelcomeWizard,TFruityLoopsMainForm,TApplication |
| mouse-sweep | engine-idle | TWelcomeWizard,TFruityLoopsMainForm,TApplication |
| idle-after-ops-10s | engine-idle | TWelcomeWizard,TFruityLoopsMainForm,TApplication |
| close-wizard-if-any | engine-idle | TFruityLoopsMainForm,TApplication |
| play-start | engine-idle | TFruityLoopsMainForm,TApplication |
| play-stop | engine-idle | TFruityLoopsMainForm,TApplication |
| idle-final-10s | engine-idle | TFruityLoopsMainForm,TApplication |
| file-menu-item-y130 | exited | TNewProjForm,TFruityLoopsMainForm,TApplication |
| file-menu-item-y150 | None |  |
| file-menu-item-y170 | None |  |
| file-menu-item-y190 | None |  |
| file-menu-item-y210 | None |  |
| file-menu-item-y232 | None |  |
| file-menu-item-y254 | None |  |
| file-menu-item-y276 | None |  |
| file-menu-item-y298 | None |  |
| file-export-submenu-item0 | None |  |
| file-export-submenu-item18 | None |  |
| file-export-submenu-item36 | None |  |
| file-export-submenu-item54 | None |  |
| file-import-submenu-item0 | None |  |
| file-import-submenu-item18 | None |  |
| file-import-submenu-item36 | None |  |
| file-import-submenu-item54 | None |  |
| file-menu-item-y111-save | None |  |
| idle-very-final-5s | None |  |

windows seen in the comparisons (largest set): ['l8:320x448:0', 'l14:1930x1092:0', 'l48:874x570:0', 'l138:206x21:0', 'l139:208x298:0', 'l140:162x30:0', 'l150:115x23:0', 'l151:345x231:0', 'l152:312x393:0', 'l153:237x619:0', 'l154:209x556:0', 'l155:437x464:0', 'l156:201x341:0', 'l157:147x42:0']

page log (dedupe/warn/error/stop lines):
   [info] ?dedupe=check: every rectangle is sent; the ones the shadow calls unchanged are kept off a reference canvas to be compared with the real one
   [warn] the first non-background pixel was drawn in lid 8 (334x474), which is not the main window: "first pixel" waits for the main window (at the latest its first idle)
   [warn] the virtual clock fell 92 ms further behind the wall clock than the cap allows (S-clk-maxlag); it stays behind (no virtual time is skipped)
   [info] dedupe: 4174 rectangles left out as unchanged, 17311 sent (583396301 pixels compared)
   [info] exit stopped code 134: the engine exited with code 134
page console errors: []


#### パス B(File の項目を Save new version から。最初の項目〔Save new version, y=150〕でエンジンが終了。再生 20 s)

run 2026-11-03T10:25:19.967Z chrome Chrome/156.0.8078.47 bundle /assets/fl-engine-fin1-F1xn/manifest.json params dedupe=check; error: None; final engine state: exited
comparisons: 111 over 166 s (every 1.5 s, all windows, all pixels); maximum differing pixels in any window at any comparison: 0

| step group | comparisons | max differing pixels | max windows compared |
|---|---|---|---|
| start | 9 | 0 | 3 |
| enter | 7 | 0 | 3 |
| idle-10s | 6 | 0 | 3 |
| file-menu-items | 14 | 0 | 6 |
| menu-edit | 3 | 0 | 7 |
| menu-add | 3 | 0 | 8 |
| menu-patterns | 2 | 0 | 9 |
| menu-view | 3 | 0 | 10 |
| menu-options | 3 | 0 | 11 |
| menu-tools | 2 | 0 | 12 |
| menu-help | 3 | 0 | 14 |
| right-click-menus | 8 | 0 | 14 |
| piano-roll-F7 | 2 | 0 | 14 |
| keys-F5-F6-F9 | 2 | 0 | 1 |
| double-click-channel | 4 | 0 | 3 |
| mixer-and-playlist-clicks | 4 | 0 | 3 |
| mouse-sweep | 4 | 0 | 3 |
| idle-after-ops-10s | 6 | 0 | 3 |
| close-wizard-if-any | 2 | 0 | 3 |
| play-start | 2 | 0 | 3 |
| play | 13 | 0 | 3 |
| play-stop | 2 | 0 | 3 |
| idle-final-10s | 7 | 0 | 3 |

frames drawn on both canvases at the end: 1835; frames the shadow called unchanged (kept off the reference canvas): 2247 (55% of all frames)
Worker's own count at the end: {'omitted': 2246, 'sent': 1835, 'comparedPixels': 59922110}

steps (engine state and the visible windows after each):
| step | state | visible windows |
|---|---|---|
| enter | engine-idle | TFruityLoopsMainForm,TApplication |
| idle-10s | engine-idle | TFruityLoopsMainForm,TApplication |
| file-menu-item-y48 | engine-idle | TFruityLoopsMainForm,TApplication |
| file-menu-item-y66 | engine-idle | TQuickPopupMenuWindow,TFruityLoopsMainForm,TApplication |
| file-menu-item-y85 | engine-idle | TFruityLoopsMainForm,TApplication |
| menu-edit | engine-idle | TFruityLoopsMainForm,TApplication |
| menu-add | engine-idle | TFruityLoopsMainForm,TApplication |
| menu-patterns | engine-idle | TFruityLoopsMainForm,TApplication |
| menu-view | engine-idle | TFruityLoopsMainForm,TApplication |
| menu-options | engine-idle | TFruityLoopsMainForm,TApplication |
| menu-tools | engine-idle | TFruityLoopsMainForm,TApplication |
| menu-help | engine-idle | TQuickPopupMenuWindow,TFruityLoopsMainForm,TApplication |
| right-click-channel | engine-idle | TFruityLoopsMainForm,TApplication |
| right-click-mixer-insert | engine-idle | TFruityLoopsMainForm,TApplication |
| right-click-playlist-track | engine-idle | TFruityLoopsMainForm,TApplication |
| piano-roll-F7 | engine-idle | TFruityLoopsMainForm,TApplication |
| keys-F5-F6-F9 | running |  |
| double-click-channel | engine-idle | TLightweightLayeredControl,TFruityLoopsMainForm,TApplication |
| mixer-and-playlist-clicks | engine-idle | TFruityLoopsMainForm,TApplication |
| mouse-sweep | engine-idle | TFruityLoopsMainForm,TApplication |
| idle-after-ops-10s | engine-idle | TFruityLoopsMainForm,TApplication |
| close-wizard-if-any | engine-idle | TFruityLoopsMainForm,TApplication |
| play-start | engine-idle | TFruityLoopsMainForm,TApplication |
| play-stop | engine-idle | TFruityLoopsMainForm,TApplication |
| idle-final-10s | engine-idle | TFruityLoopsMainForm,TApplication |
| file-menu-item-y150 | exited | TNewProjForm,TFruityLoopsMainForm,TApplication |
| file-menu-item-y210 | None |  |
| file-menu-item-y254 | None |  |
| file-menu-item-y276 | None |  |
| file-menu-item-y298 | None |  |
| file-export-submenu-item0 | None |  |
| file-export-submenu-item18 | None |  |
| file-export-submenu-item36 | None |  |
| file-export-submenu-item54 | None |  |
| file-import-submenu-item0 | None |  |
| file-import-submenu-item18 | None |  |
| file-import-submenu-item36 | None |  |
| file-import-submenu-item54 | None |  |
| idle-very-final-5s | None |  |

windows seen in the comparisons (largest set): ['l8:320x448:0', 'l14:1930x1092:0', 'l48:874x570:0', 'l138:206x21:0', 'l139:208x298:0', 'l140:162x30:0', 'l150:115x23:0', 'l151:345x231:0', 'l152:312x393:0', 'l153:237x619:0', 'l154:209x556:0', 'l155:437x464:0', 'l156:201x341:0', 'l157:147x42:0']

page log (dedupe/warn/error/stop lines):
   [info] ?dedupe=check: every rectangle is sent; the ones the shadow calls unchanged are kept off a reference canvas to be compared with the real one
   [warn] the first non-background pixel was drawn in lid 8 (334x474), which is not the main window: "first pixel" waits for the main window (at the latest its first idle)
   [warn] the virtual clock fell 62 ms further behind the wall clock than the cap allows (S-clk-maxlag); it stays behind (no virtual time is skipped)
   [info] dedupe: 2247 rectangles left out as unchanged, 1835 sent (59922980 pixels compared)
   [info] exit stopped code 134: the engine exited with code 134
page console errors: []


#### パス C(File > Import と Export、そのサブメニューの項目。Save 系は除く。再生 20 s)

run 2026-11-03T10:34:25.537Z chrome Chrome/156.0.8078.47 bundle /assets/fl-engine-fin1-F1xn/manifest.json params dedupe=check; error: None; final engine state: exited
comparisons: 139 over 206 s (every 1.5 s, all windows, all pixels); maximum differing pixels in any window at any comparison: 0

| step group | comparisons | max differing pixels | max windows compared |
|---|---|---|---|
| start | 9 | 0 | 3 |
| enter | 7 | 0 | 3 |
| idle-10s | 6 | 0 | 3 |
| file-menu-items | 16 | 0 | 6 |
| menu-edit | 3 | 0 | 7 |
| menu-add | 3 | 0 | 8 |
| menu-patterns | 2 | 0 | 9 |
| menu-view | 3 | 0 | 10 |
| menu-options | 3 | 0 | 11 |
| menu-tools | 2 | 0 | 12 |
| menu-help | 3 | 0 | 14 |
| right-click-menus | 8 | 0 | 14 |
| piano-roll-F7 | 3 | 0 | 14 |
| keys-F5-F6-F9 | 3 | 0 | 1 |
| double-click-channel | 3 | 0 | 3 |
| mixer-and-playlist-clicks | 4 | 0 | 3 |
| mouse-sweep | 4 | 0 | 3 |
| idle-after-ops-10s | 7 | 0 | 3 |
| close-wizard-if-any | 1 | 0 | 3 |
| play-start | 2 | 0 | 3 |
| play | 14 | 0 | 3 |
| play-stop | 2 | 0 | 3 |
| idle-final-10s | 7 | 0 | 3 |
| file-export-submenu-item0 | 5 | 0 | 7 |
| file-export-submenu-item18 | 4 | 0 | 9 |
| file-export-submenu-item36 | 5 | 0 | 11 |
| file-export-submenu-item54 | 4 | 0 | 13 |
| file-import-submenu-item0 | 6 | 0 | 16 |

frames drawn on both canvases at the end: 2776; frames the shadow called unchanged (kept off the reference canvas): 4698 (63% of all frames)
Worker's own count at the end: {'omitted': 4696, 'sent': 2776, 'comparedPixels': 72468327}

steps (engine state and the visible windows after each):
| step | state | visible windows |
|---|---|---|
| enter | engine-idle | TFruityLoopsMainForm,TApplication |
| idle-10s | engine-idle | TFruityLoopsMainForm,TApplication |
| file-menu-item-y48 | engine-idle | TFruityLoopsMainForm,TApplication |
| file-menu-item-y66 | engine-idle | TQuickPopupMenuWindow,TFruityLoopsMainForm,TApplication |
| file-menu-item-y85 | engine-idle | TFruityLoopsMainForm,TApplication |
| menu-edit | engine-idle | TFruityLoopsMainForm,TApplication |
| menu-add | engine-idle | TFruityLoopsMainForm,TApplication |
| menu-patterns | engine-idle | TFruityLoopsMainForm,TApplication |
| menu-view | engine-idle | TFruityLoopsMainForm,TApplication |
| menu-options | engine-idle | TFruityLoopsMainForm,TApplication |
| menu-tools | engine-idle | TFruityLoopsMainForm,TApplication |
| menu-help | engine-idle | TQuickPopupMenuWindow,TFruityLoopsMainForm,TApplication |
| right-click-channel | engine-idle | TFruityLoopsMainForm,TApplication |
| right-click-mixer-insert | engine-idle | TFruityLoopsMainForm,TApplication |
| right-click-playlist-track | engine-idle | TFruityLoopsMainForm,TApplication |
| piano-roll-F7 | engine-idle | TFruityLoopsMainForm,TApplication |
| keys-F5-F6-F9 | running |  |
| double-click-channel | engine-idle | TLightweightLayeredControl,TFruityLoopsMainForm,TApplication |
| mixer-and-playlist-clicks | engine-idle | TFruityLoopsMainForm,TApplication |
| mouse-sweep | engine-idle | TFruityLoopsMainForm,TApplication |
| idle-after-ops-10s | engine-idle | TFruityLoopsMainForm,TApplication |
| close-wizard-if-any | engine-idle | TFruityLoopsMainForm,TApplication |
| play-start | engine-idle | TFruityLoopsMainForm,TApplication |
| play-stop | engine-idle | TFruityLoopsMainForm,TApplication |
| idle-final-10s | engine-idle | TFruityLoopsMainForm,TApplication |
| file-menu-item-y254 | engine-idle | TQuickPopupMenuWindow,TFruityLoopsMainForm,TApplication |
| file-menu-item-y276 | engine-idle | TFruityLoopsMainForm,TApplication |
| file-export-submenu-item0 | engine-idle | TFruityLoopsMainForm,TApplication |
| file-export-submenu-item18 | engine-idle | TFruityLoopsMainForm,TApplication |
| file-export-submenu-item36 | engine-idle | TFruityLoopsMainForm,TApplication |
| file-export-submenu-item54 | engine-idle | TFruityLoopsMainForm,TApplication |
| file-import-submenu-item0 | exited | TMsgForm,TFruityLoopsMainForm,TApplication |
| file-import-submenu-item18 | None |  |
| file-import-submenu-item36 | None |  |
| file-import-submenu-item54 | None |  |
| idle-very-final-5s | None |  |

windows seen in the comparisons (largest set): ['l8:318x446:0', 'l14:1930x1092:0', 'l48:874x570:0', 'l138:208x298:0', 'l139:85x23:0', 'l140:208x298:0', 'l141:177x358:0', 'l142:208x298:0', 'l143:175x37:0', 'l144:208x298:0', 'l145:175x56:0', 'l146:208x298:0', 'l147:175x75:0', 'l148:208x298:0', 'l149:83x21:0', 'l152:271x182:0']

page log (dedupe/warn/error/stop lines):
   [info] ?dedupe=check: every rectangle is sent; the ones the shadow calls unchanged are kept off a reference canvas to be compared with the real one
   [warn] the first non-background pixel was drawn in lid 8 (334x474), which is not the main window: "first pixel" waits for the main window (at the latest its first idle)
   [warn] the virtual clock fell 400 ms further behind the wall clock than the cap allows (S-clk-maxlag); it stays behind (no virtual time is skipped)
   [info] dedupe: 4698 rectangles left out as unchanged, 2776 sent (72470067 pixels compared)
   [info] exit stopped code 134: the engine exited with code 134
page console errors: []


## 8. PM の判断点

1. **`?dedupe=1` を既定 ON にするか**。材料: (a) 正しさ: 語単位の厳密比較で、省略した矩形の画素は canvas の現在の内容と同一(単体の性質試験〔12 シード×1,500 手〕、実 `Presenter` との統合試験〔方式 A/B、16 本×1,500 手〕、変異検査、実エンジンの自己検証 §7.5)。(b) 効果: ページ主スレッド −0.057 コア(−78%)、Chrome 全体 −0.106 コア、窓の更新 62.5/s → 0.85/s(§7.1)。Worker は変わらない。(c) 入力の応答: §7.3。(d) 既定 ON の前に要る物: Chrome の 2D canvas の `contextlost`/`contextrestored`(GPU のリセットで canvas の内容が消えると、エンジンが画素を変えない限り戻らない=従来はアイドルの毎フレーム再描画で自然に直っていた)への対応(page→Worker の再同期メッセージ。未実装・未テスト)。(e) 『1 画素でも差が出たら ON にしない』という条件に対し、§7.5 の自己検証は**比較 549 回(約 13.7 分、再生 300 s を含む)で差分 0**。ただし File > Save / Save as ではエンジンが終了する(§11)ので、その2 つのダイアログは見ていない。
2. **エンジンの応答(層 2 の DwmIsCompositionEnabled=TRUE)は変えていない**(PM の方針)。『FALSE を返す opt-in』は native の実エンジンで効果を測っただけ(アイドルのゲストの関数入口 8.78 → 4.54 M/s、書込 63 → 0〜1 /s)で、実装していない。実装するなら `lift/runtime/fle_dwm.c` に環境変数の opt-in 1 本(忠実性の観点で PM が見送り)。
3. **ゲスト CPU(アイドルで +0.06 コア相当)を下げる最短の道は直接 wasm の混成 p1000r の採用**(§5。再描画の追加入口の 99.6% が lower 済み)。F1xn の上での測定は未実施。
4. 行(層 2)の取捨は変えない。起動の基準表は変わらない(新しい基準表は作っていない)。

## 9. 自己申告(規則違反・事故)

- **成果物・作業ディレクトリの名前の衝突**: 指示の `lift/out-git/idle1-results/` と `lift/out-idle1/` は、11-01 の別の(古い)idle1 作業(決定 0129 前後)の物として既に存在した。`idle1-results/` には `tools/` と `tables/` を作って自分のツールを 20 ファイルコピーしてしまったが、気づいてすぐ、自分が作った 2 ディレクトリ(birth 時刻 18:31 で確認)だけを消し、元の内容は無傷(`MERGE-NOTES.md`・`docs`・`*.diff`・`learn`・`logs`・`seeds`・`tools-idle1`・`wall-log.md`)。成果物は新しい名前 `idle1b-results/`。`lift/out-idle1/` は古い物(`clean/`〔27 MB〕・`inputs/`〔74 MB〕・`learn/`・`logs/`・`tools/` の古いスクリプト・`wall-log.md`)の上に自分の物を足す形で使った(birth 時刻で、古いファイルを上書きしていないことを確認)。終了時は自分の足した物だけを消し、古い物は消さない。
- 背景起動のコマンドの末尾に `&` を付けた(1 回。`background: true` との二重で、シェルは拒否しなかった。以後付けていない)。`sleep 1`(1 回)と `timeout`(1 回)を試して拒否された。存在しない `bash` ツールを 2 回呼んだ。
- 作業ディレクトリの du が一時 8.1 GB(5 GiB の上限超え)になった(native の計数リンク済みバイナリ 7 本が各 512 MB と生成 C)。気づいて余分なバイナリと生成 C を消し 3.6 GB にした。同時に重い処理が 3 本以上走った時間がある(native 2 本 + Chrome 1 本 + vitest)。**Chrome を同時に 2 本動かした時間がある**(長時間の自己検証と全 e2e を重ねた。どちらも CPU の測定ではない)。**入力応答の系列 L1 の途中に native の peek 2 本が重なった**(`runs/pk_n`・`pk_r`。L1 の on3 前後。load は 9〜14 で、表に併記)。load は他の担当の分も含めて 5〜23。
- 最初の `?dedupe=1` の版に不具合があった(§6): layout 世代だけが変わった窓の全面送り直しを省略しうる。統合試験を書く過程で見つけて修正した。**CPU の交互測定(§7.1、60 Hz)は修正前の版で行った**(アイドルの経路は同じで、`resetShadow` は layout/layer/size の変化のときだけ働く)。120 Hz・入力応答・長時間の自己検証は修正後の版。
- 中間報告 2 で『e2e の新規場面 +36 チェック』と書いたが、数え直すと **99 チェック(1 target あたり 33 × dev・preview・worker の 3 target。全 e2e は基準の 853/853 から 952/952 checks passed)**(§7.4)。

## 10. 成果物

`lift/out-git/idle1b-results/`: `idle1b.diff`、`MERGE-NOTES.md`、`docs/0187-idle-redraw.md`(本書)、`docs/README-0187-line.txt`、`tables/`(Chrome・native の表、台帳、自己検証)、`logs/`、`tools/`(`README.md`)。

## 11. 副次的な発見(本作業の範囲外・原因は調べていない)

- **Chrome の F1xn(既定の入口)で、File メニューの『Save』(画面 (40,111))または『Save as...』(画面 (40,130))を押すと、エンジンが `fle: unsupported instruction at 0x424d40` で停止する**(`exit` = stopped、コード 134 = SIGABRT、`TNewProjForm` が見えている)。**停止の記録(実測、`tools/check-long.mjs` が取った engine のログ)**: `fle: unsupported instruction at 0x424d40`(`fle_unsupported`、SIGABRT)。0x424d40 は `user32.dll!GetKeyNameTextW` の import サンクで、行が無い(`docs/investigations/ia1/tables/thunks.tsv` の 509 行目も『unsup』)。= File > Save / Save as が(ショートカットキーの名前の表示などで)`GetKeyNameTextW` を呼び、行が無くて壁になる。**新しい壁 W18 の候補**(ia1 の壁の台帳〔W1〜W17〕にない)。 再現: Save は 2 回(`tools/check-long.mjs` の最初の版と診断 run)、Save as は 2 回(長時間の run 〔再生 300 s の後〕と診断 run `runs/long2` = `tables/check-long-diag-save-as-abort.json`)。native の ia1 の fork プローブでは同じ操作(`file_save`・`file_saveas`)が停止しない記録(`docs/investigations/ia1/tables/ops-after-final.tsv`)なので、構成の差(行・prune S・Blend2D 置換・wasm)による可能性がある(読み。確かめていない)。本作業の長時間検証は、この 2 項目を最後に回して、その前を比較した。
- 同じ run で、F5/F6/F9(窓の開閉のキー)の直後に Welcome wizard(`TWelcomeWizard`)が再び現れた(2〜3 回の run で再現。run によっては `TLightweightLayeredControl` が出た)。モーダルなので、その間の入力は主窓に届かない。原因は調べていない。
- p2a(決定 0182)が F1xn で測った『入力→次の更新』は、窓が毎フレーム更新されているため、無関係な更新で短く出ていた(本書 §7.3)。`?dedupe=1` なしの値は応答時間として使えない。
- パス C(File > Import → サブメニューの最初の項目)で、エンジンが `fle: CompareStringW: character other than an ASCII letter or digit in unequal strings (not defined by the MSDN page or outside the emulated locale; not implemented)` で停止した(exit 134、`TMsgForm` が見えている)。決定 0178(dlg1)の『S-dlg1-cmp-nonascii: 仮名と ASCII の順序に出典がない』と同じ壁と読める(確かめていない)。File > Export の 4 つのサブメニュー項目は停止せず比較できた。

## PM の判断と取り込み(2026-11-03、追記)

取り込み: `idle1b.diff`(`app/` の 10 ファイル。基準 HEAD d425abd〔88a4dfe でも `app/` は同じ〕に `patch -p1 --dry-run` が offset・fuzz・reject なしで当たる)。成果物の置き場は `lift/out-git/idle1b-results/`(古い `idle1-results/` は 11-01 の別の作業の物で、PM の番号付けの不注意による名前の衝突。担当は古い物を無傷で処理した)。`-` 行 15 本を読み、既存の 1 行への引数・欄の追加だけと確認した。PM の検証: typecheck rc=0、vitest **1003 passed + 7 skipped**、Worker 43/43、e2e **952/952**(基準 853/853)。`lift/` には触れていない。PM は Chrome の CPU 測定と長時間の自己検証を再現していない(担当の実測)。進捗(G 段階)には数えない。

判断:

1. **毎フレームの再描画は『エンジンの仕様』として扱う**(DWM 合成有効時にエンジンが意図して行う。実機の FL でも同じ分岐に入る可能性が高い=読み、実機照合 0)。ランタイムの応答(層 2 の行の `DwmIsCompositionEnabled` = TRUE)は変えない。performance-targets §4c の『アイドルの更新 55/s』は、F1xn がこの再描画を毎フレーム行うためで、ポインタ移動のせいではない(決定 0182 の取り込み時に訂正済み)。
2. **`?dedupe=1` は opt-in(既定 OFF)のまま取り込む**。既定 ON にする条件: (a) Chrome の 2D canvas の `contextlost`/`contextrestored` 後の再同期の実装とテスト、(b) File > Save / Save as / Save new version のダイアログの区間で `?dedupe=check` の差分 0 を示すこと(今は下の壁 W18 で比較できていない)、(c) 長時間(再生 300 s 超、窓の開閉)の check で差分 0 の再確認。ship1(決定 0189)の出荷バンドルの測定には `?dedupe=1` あり・なしの両方を併記する。
3. **ゲスト側の費用を下げる最短の道は直接 wasm の混成 `p1000r`(決定 0185)**: アイドルの追加の関数入口の 99.6% が p1000r で lower 済み(ship1 の SHIPH で F1xn 上のアイドル CPU を測る)。BitBlt の省略は効果が小さいので実装しない。
4. **副次的な発見を次の作業の壁として記録**: (W18) File > Save / Save as / Save new version で `user32!GetKeyNameTextW`(行なし)で停止(exit 134、5 回再現)、File > Import のサブ項目の `CompareStringW`(非 ASCII の順序。dlg1 の S-dlg1-cmp-nonascii と同じ壁)、F5/F6/F9 の後に Welcome wizard が再び出る(原因未調査)。操作の壁の第 2 波として別の担当(ia2)に回す。
