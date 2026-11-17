# 依頼者の観察 O1〜O15 の検証 scenario と受け入れ基準（再現手順書。実行は gpu1 が先にする）

**先頭**: **実エンジンの実行は 0**（ci1、2026-11-07 07:0x machine date。読みと設計だけ。Chrome・Firefox・Wine・ビルドは起動していない）。出典: `docs/feedback/2026-11-07-owner-observations.md`（O1〜O15 と PM の決定）、`lift/out-ci1/kit/tools/sdriveg2-export.mjs` と `sdriveg2-lat.mjs`（op の一覧と引数）、`scn-*.json`（既存の座標）、`app/src/main.ts`（`window.__app`）、`app/test/e2e/engine-desk.mjs`（絵の一致の検査）。**依頼者の機械の値は 0 件。依頼者の観察は我々の測定ではない**（定性的な報告）。区分（A = 我々の余計な処理、B = 外部 API の契約の不足、C = 資材の欠落、D = FL 本来の挙動、E = 未分類）は PM の暫定で、本書は検証の手順と合否を決める。**方針（lift の忠実性）**: 原因は (a) 余計な処理か (b) 外部 API の契約の実装不足のどちらかだけ。各項目の『直す全体の仕組み』は (a) の除去か (b) の契約の網羅（import の census）で、画面ごと・症状ごとの対症、FL のクラス名・窓の種類・位置による分岐は足さない。座標は既存の scenario から取ったもの以外は**未確認**（gpu1 が画像で決める）。

## 0 共通の規約
- **道具**: `sdriveg2-export.mjs`（Chrome、CDP の模擬入力）。op: `closeWizard` `wait` `waitIdle` `click` `clickIn{cls,dx,dy}` `down` `up` `move` `drag{button,x,y,x2,y2,steps,stepMs}` `burst{n,per,gapMs}`（button を押したまま `sendRaw` で n 回の移動）`dragRate{secs,steps}`（updates/s・rects/s・pixels/s）`dragMeasure{name,button,x,y,x2,y2,n,secs,settle,idle}`（Worker の CPU/移動、presents・rects・Mpx）`seqMeasure{ops,units}` `wheel{x,y,dx,dy}` `key{code}` `ctrlKey` `type` `openProject` `openZipProject` `openFile` `waitWindow{cls,visible,ms}` `listWindows` `describe` `shot` `savePic{name,peek}` `pixel` `expectPixel` `cmpShot` `hashes`（canvas・peek・whole の 3 つの sha256）`series{secs,step}`（CPU と `driverStats`）`audioSnap` `audioMark` `audioDump` `mem`（ブラウザの木の RSS/PSS）`logMark` `saveLog{name,mark|all}` `eval` `evalWait` `expectRunning` `expectExit` `expectWindow`。
- **共通の失敗の判定（F0）**: `window.__app.state` が `exited` か `error`（`expectRunning` が false）、report の `opErrors` が空でない、`engineLog` に `fle: unsupported`・`abort`・`pruned function`・`not supported`・`stop:` の行（`saveLog` で取り、`logMark` で scenario の区間だけ切る）。**停止が出たら VA を表で API 名にして『どの契約の不足か』を 1 行に書く**（個別に直さない）。
- **共通のオラクル（絵）**: (i) `deskDumpBase64(true)`（peek = 保持している絵）と `(false)`（whole = 窓表から描き直した絵）の差が動くメーターの箱（`METER_BOX` = x 1037..1122・y 6..38、≤ 200 画素）以外で 0（`engine-desk.mjs:198, 369` の『damage tracking lost nothing』）。(ii) 残像の消える速さ T_clear = op `tClear`: 操作の直後から保持している絵（peek）を繰り返し読み、メーターの箱の外で `stableMs`（300 ms）変わらなくなるまでの時間 `tStillMs`、静止後に whole を 1 回だけ取って peek と箱の外で比べた `ghostPx`（0 = 残像なし）。whole は resync するので繰り返さない。(iii) メーターの更新: 箱の画素が変わる回数/s（peek を 100 ms ごと）。(iv) P の経路（E14 以降）でも `peek` は P 経由で通る（`E2E-INVENTORY.md` §4）。
- **共通の前提**: 開始前に ps（他の Chrome/Firefox/Wine/clang/wasm-opt/node の Export が 0 本）、`free -m | sed -n 2,3p`、`vmstat 1 3 | tail -2`（si/so が 0、MemAvailable ≥ 8 GB）、`wait-quiet.mjs`。Chrome は 1 本ずつ。計時の判定（fps の閾値）は PM の窓で。**headless Chrome の rAF は 60 Hz なので、present/s の上限は約 60**。60 を超える値を測るときは rAF 無制限のフラグ（0297 が使った。e2e の `browser.mjs` は `FL_CHROME_ARGS` で Chrome の引数を足せるが、sdriveg2 での指定方法は gpu1 に確認、**未確認**）で、page が 185 Hz などを測って 120 に丸めた値を FL に渡す製品の経路（0268: 上限 120）を確認する。
- **project**: 既存の kit は `serina1-serum2`（`openProject` か `openZipProject`）。**依頼者の『評価用プロジェクト』が同じか未確認**。

## 1 native の目標値（O6）を我々が測れる指標に対応づける
| native の目標（依頼者の機械、0 件の我々の値） | 我々が測れる指標 | 取り方 | 読み方・限界 |
|---|---|---|---|
| 再生中 160〜240 fps | (1) **present/s**（page が画を出す回数。`deskStats.presents` の差 ÷ 秒）、(2) FL の UI の刻み（`deskStats.ticks`、`idleTicks`/s）、(3) **メーターの更新回数/s**（箱の画素の変化。上の (iii)）、(4) 仮想 s/実 s、(5) 渡した Hz（`window.__app.hz.hz`）と FL の『Animation timer』（O8） | `scn-p3.json` 型（`openZipProject` → `Space` → `audioSnap`/`series`）、60 s、10 s 窓 | 製品は Hz を {60, 75, 90, 120} に丸め上限 120（0268）。native の 240 Hz 環境の 160〜240 fps は**設計上この上限を超える**。上限を外すか（依頼者の画面 240 Hz）は未決（PM の決定が要る）。**現在の測定**（この機械）: 純再生の present は 13.6〜34.6/s（協調 / P + V7）、FL の画素変化は仮想 s あたり約 21〜27 回で Hz によらない（0297 の読み） |
| 発音数 voices 16〜20 | **直接の指標は無い**（FL の発音数表示は画素）。代用: ミキサーの CPU/音声 1 秒、書き込み率、underrun | `audioSnap`・`cpuStart/Stop` | 未確認。FL の poly 表示の画素を gpu1 が `savePic` で目視（定量は不可） |
| Playlist を激しく動かして 60〜100 fps | **present/s、rects/s、pixels/s（`dragRate`）、Worker CPU/移動（`dragMeasure`）、メーターの更新/s** | 下の O1 | rAF 60 Hz が上限（無制限フラグで 60 超を見る）。present/s ≥ 0.9 × min(rAF, 渡した Hz) を『追従』とする暫定の基準（仮） |
| メモリ約 3750 MB | **WebAssembly メモリ**（`window.__app.perf.snapshot().memory.wasmBytes`、上限 2 GiB = `wasmLimitBytes`）、**ブラウザの木の RSS/PSS**（`mem` op） | 起動後・再生 60 s 後・Data 取得後 | native の 3750 MB は Data・Plugins・サンプルを含む。製品の wasm の線形メモリ（上限 2 GiB、基底 約 1 GB。0312j）は**同じ物の比較ではない**。資材を遅延取得にする（O12）と OPFS の使用量（`navigator.storage.estimate`）も併記 |
| dirty 矩形 | `deskStats.rects`、`pixels`、`composeMs`、`putMs`（1 present あたり） | `dragRate`/`seqMeasure` | 再描画の過剰（A）と不足（B）を分ける: 矩形が小さく数が少ないのに残像 = 不足、全面再描画が続く = 過剰 |

## 2 項目別（O1〜O15）
### O1 Playlist の Pattern のドラッグ — 慣性/重力のアニメと配置先の骨格が出ない・残像・低 fps・激しいドラッグでエンジンが死ぬ（暫定 E）
- **再現（sdriveg2）**: `closeWizard` → `openZipProject`（約 10 s）→ `wait 3000` → `key Space`（再生中、再生なしの 2 通り）→ Playlist の既存の Pattern clip の位置を画像で決める（**座標は未確認**。既存の Playlist 領域は `d4-playlist-middle-drag` の (1417,436)〜(1700,560) の近く）→ `down`（左ボタン）→ ①穏やか: `drag` 12 steps × 50 ms、②激しい: `burst{n:300, per:10, gapMs:0}` を 8 s、往復 3 回 → `up` → 直後に `tClear`（T_clear）。`dragRate{button:"left", secs:8, steps:20}` と `dragMeasure` を併用。
- **合格**: (1) 激しいドラッグ 3 試行（各 8 s）で `state` が `engine-idle` のまま、F0 の行が 0。(2) ドラッグ中の present/s ≥ 0.9 × min(rAF, 渡した Hz)（headless は 54/s 以上、無制限フラグなら 60 以上）、メーターの更新/s は同じ区間のドラッグ無しの値の 0.9 倍以上。(3) 配置先の骨格とアニメ: 落とした後の 1 s に peek を 33 ms ごとに 30 枚取り、配置先の矩形の画素が**複数のフレームにわたって変わる**（アニメが存在する）。native の枚数は未測定なので閾値は置かず記録（依頼者の観察に対する再現の証拠として、画像 6 枚を保存）。(4) T_clear ≤ 500 ms（暫定。O15 の native 参照が出るまで『記録』）。
- **失敗**: 停止、残像が 2 s 続く（peek と whole の差がメーター以外に残る）、present/s < 0.5 × rAF。
- **直す全体の仕組み（案）**: 停止は (b) タイマー・メッセージ優先順位・`SetCapture` 系の契約の census（0266 の alpha9 型）。低 fps は (a) の除去（余計な待ち・間引き）か Hz/タイマーの契約（O8）。クラス名分岐は足さない。

### O2 起動アニメーション（フェードアウト）が低 fps（暫定 E）
- **再現**: 新しいプロファイルで起動し、`first_pixel`〜`first idle` の間を **100 ms ごとに `deskStats`**（`eval`/`series`）で取る。ウィザードを閉じる `closeWizard` 後の 3 s も同様。
- **合格**: フェードの区間の present/s ≥ 0.9 × min(rAF, 渡した Hz)、FL の刻み（`ticks`/s）が渡した Hz の ±10%。**失敗**: present/s < 0.5 × rAF。native の fps は依頼者の目視のみ（未測定）。
- 仕組み: Hz/タイマー（O8 の Animation timer）と present の頻度の契約。

### O3 Settings → About のロゴの浮遊とスタッフロールが出ない（暫定 E: 資材 C か描画の契約 B）
- **再現**: `closeWizard` → `click (202,20)` → `wait 1500` → `click (232,101)` → `waitWindow TMIDIForm visible`（`d3-settings-all-tabs` と同じ）→ **About タブの `clickIn{cls:"TMIDIForm", dx:?, dy:48}`（dx は未確認。d3 は 27・81・146・203・258・328・393 を押す。About はその右の可能性）**→ `savePic` を 500 ms ごとに 4 枚。
- **合格**: ロゴの領域（画像で矩形を決める）に背景以外の画素があり、4 枚で**画素が変わる**（浮遊アニメ）。スタッフロールの領域も同様。エンジンのログに『ファイルが無い/LoadImage/FindResource の失敗』の行が無い（有れば C）。**失敗**: 領域が一定、または背景のまま。
- 仕組み: C なら資材（O12 の Data の遅延 VFS）。B なら描画 API の契約（`StretchDIBits`・`AlphaBlend`・レイヤー窓の契約）の census。

### O4 アニメーション全般が足りない（推測）
- **再現**: アニメの一覧表を作る: スタートアップのフェード、メニューの fade（`add-menu-simple-fade`）、ピッカーの fade（`picker-f8-fade`）、About、Pattern ドラッグ、プラグイン窓の開閉。各々で O2 と同じ present/s と FL の刻みを表にする。
- **合格**: 各アニメ区間の present/s ≥ 0.9 × min(rAF, 渡した Hz)。**失敗**: いずれかが 0.5 倍未満。仕組み: Hz/タイマー（O8）と present の経路の一箇所。

### O5 DOM ヘッダの『Monitor >』で右ドロワーを畳むとエンジンが止まる（暫定 A または E）
- **再現**: `#panel-toggle`（`app/index.html`、`aria-pressed`）を `eval`（`document.getElementById('panel-toggle').click()`）か CDP の実マウスで押す。①idle、②再生中、③Pattern ドラッグ中、の 3 状態で畳む/開くを各 10 回、間に `wait 1500`。`window.__app.screen.panelOpen`・`screen.resizes`（`guest` と `sent`）・`screen.engineFollows` を各回記録。
- **合格**: 全 30 回で `state` が `engine-idle`、F0 の行が 0、`resizes` の `sent` が true のとき engine のログに display change の処理の行（停止でない）。**失敗**: `exited`/`error`、または `engineFollows = false` で resize を送っている。
- 仕組み: 畳みはページの DOM の処理で、ゲストの『画面』の大きさが変わる（0212）。engine 側の display change の契約（`WM_DISPLAYCHANGE`・`EnumDisplaySettings`・`GetSystemMetrics`）の網羅（B）か、ページ側の過剰な処理（A）。既存 e2e の `engine-desk-resize` は『display change が無い build は NOT CHECKED』と記録するだけ（`engine-desk.mjs:469`）なので、この scenario が本体の検査になる。

### O6 数値目標 — §1 の対応表のとおり。scenario は O1（Playlist）と `scn-p3.json`（再生 60 s）。**合否は PM の窓で**。

### O7 画面がぼやける（HiDPI 未対応?）（暫定 B/E）
- **再現**: sdriveg2 の `newPage({ width, height, dpr:1 })` を **`dpr: 1.75`（依頼者の 168 PPI）と 2** にする（**sdriveg2 に `--dpr` の引数が無い = ツールの 1 行の変更が要る。未実装**）。起動後に `eval` で: `devicePixelRatio`、`#desktop canvas` の `width`・`height`（backing）と `getBoundingClientRect()` の CSS 寸法、`window.__app.screen.asked/reported`（ゲストの画面）。`shot` と peek を比べる。
- **合格**: backing の画素数 ≥ CSS 寸法 × dpr（拡大されていない）、ゲストが報告する PPI が 96 × dpr 前後で、エンジンの『Main GUI scaling factor』（O8）が 1.75 になる。**失敗**: backing が CSS × 1（ブラウザが拡大 = ぼやけ）。
- 仕組み（B）: ゲストの DPI・画面寸法の契約（`GetDeviceCaps(LOGPIXELSX)`・`GetDpiForWindow`・DPI awareness・`GetSystemMetrics`）をページの dpr に合わせて返す。ページ側のぼかし用の独自処理（A）が有れば除去。

### O8 Settings → Debug のログが native と違う（暫定 B/E）
- **再現**: O3 と同じ手順で Debug タブ（dx 未確認）→ `savePic`。**文字は画素なので、同じ文面を `engineLog` から取れるか**を最初に確認する（`saveLog all` で `Starting at`・`SSE3` を grep）。取れなければ画像で読む（OCR 無し、目視）。
- **合格の条件（行ごと。native の値は依頼者の観察）**: (1) `Starting at` の時刻が page の現在時刻と ±2 s（製品は『12:00:00 AM』= 仮想時計の起点。時刻の契約 `GetLocalTime`/`GetSystemTime` が page の時刻を返すか）。(2) `SSE3/SSSE3/AVX`: **製品は CPUID を SSE2 に固定している（決定 0307: AVX を答えると IPP・Elastique が止まる）= 意図した差**として『期待どおり no』を合格とし、方針を変えるなら PM の決定（記録のみ）。(3) `GUI scaling factor` は O7 の dpr に一致（1.75 なら 168PPI）。(4) `Temporary folder` が `C:\Users\<name>\AppData\Local\Temp\Image-Line\…`（製品は `C:\Windows\Image-Line\4096-{…}` = `GetTempPathW` の契約）。(5) `FL Studio language`/`Windows display language` が page の `navigator.language` に対応（ja → Japanese / ja-JP。`GetUserDefaultLocaleName`・`GetUserDefaultUILanguage`）。(6) `Loading tags finished.` が `started` の後に出る（バックグラウンドのスレッド/タイマーの完了。出ないのは B のスレッド・Data 欠落 C）。(7) `Browser caching took N milliseconds` が非 0 の実時間に見える。(8) `Animation timer set to 8ms`（製品。native 4 ms = 240 Hz）: 渡した Hz の 1000/Hz に一致すれば合格、240 Hz 対応は O6 の判断。
- **失敗**: (1)(3)(4)(5)(6) の不一致。仕組み: 環境の契約（時刻・ロケール・パス・DPI・CPUID）の census と、各 API の返値を『ページの実際の環境』から返すこと（個別の文字列の書き換えはしない）。

### O9 Settings → General をスクロールすると描画が崩れる（暫定 B: ScrollWindowEx/ScrollDC/無効領域の契約、フォントの幅）
- **再現**: Settings を開き General タブ（`clickIn dx 146 dy 48`、d3 と同じ）→ `savePic P0`（スクロール前）→ `wheel{x,y,dy:120}` を 10 回（間に 200 ms）→ `savePic P1` → `wheel{dy:-120}` を 10 回 → `savePic P2` → **ホバー掃引**: 窓の全行を `move` で 2 往復 → `savePic P3`。各段階で peek/whole も取る。スクロールバーのつまみの `drag` でも同じ。
- **合格（オラクル）**: (1) **往復不変**: P2 = P0（メーター等の動く箱以外で画素が同一）。(2) **ホバー不変**: P3 = P2（ホバーで『直る』= 遅延の描画が無い。差が出たら『無効領域の不足』）。(3) peek = whole（damage の欠落が無い）。(4) 文字が横に潰れない: 同じ行の文字列の幅が P0 と P2 で同じ。**失敗**: (1)〜(4) のどれか。(2) が失敗で (3) が成功なら surface 自体が古い = ゲストの描画の契約（B）、(3) が失敗なら我々の damage 追跡（A）と切り分ける。
- 仕組み: `ScrollWindowEx`・`ScrollDC`・`ScrollWindow`・`InvalidateRect`・`GetUpdateRgn` の census（alpha1。`traceStart` で呼び出しを数える）を契約文書どおりに。フォントの幅は GDI の `GetTextExtentPoint32`・`CreateFont` の契約。

### O10 コアなデータ（テーマ・言語・テンプレート等）が無い（C。判断済み: 全体を遅延 VFS）
- **再現（仕様が決まった後）**: Settings → Theme タブで項目数、General の言語のドロップダウン、File → New from template の一覧、をそれぞれ `savePic`。
- **合格**: ネイティブの記録（opt1 の census）と同数のテーマ・言語・テンプレートが出る。FindFirstFile/GetFileAttributes が `Data` 全域で効く（engine ログの trace か Browser の一覧で、記録の件数・サイズと一致）。**P1 を壊さない**: first idle ≤ 10 s（Chrome、静かな窓）かつ前の bundle + 100 ms の基準（索引と seed を先に読む設計）。**失敗**: 件数の不足、first idle の劣化。**未確認**: 遅延 VFS は未実装（PM の決定 1 の待ち）なので、本 scenario は仕様待ち。
- 測れる指標: HTTP の Range 要求の数と bytes（`preview-logged.mjs` の access log）、OPFS の使用量（`navigator.storage.estimate()`）、背景の先読みの進み。

### O11 FL 内蔵のサンプルが使えない（`Data\Patches\Packs\Drums (ModeAudio)\Kicks` に `*.wv` 349 個。C + B: WavPack の復号）
- **再現（資材の遅延取得ができた後）**: Browser パネル → Packs → Drums (ModeAudio) → Kicks を開き `savePic`、1 つを Sampler にドラッグ/ダブルクリックで読み込み、`Space` か鍵盤でプレビュー → `audioDump`。
- **合格**: 一覧が 349 件、サンプルが読めて**非無音の PCM**（`audioDump` の peak > 0）、`fle: unsupported` 0。**失敗**: 一覧が空、読み込みで停止、無音。仕組み: 復号の実体（FL 自身の DLL か、Windows の API 経由か）は**未確認**。MP3 の ACM（msacm32、0229）や Vorbis の oggio（0238）と同じ型なら、外部 API の不足（B）を census で。

### O12 `Data` 792 MB と `Plugins` 867 MB の全部が使える（PM の決定 1 の待ち: 遅延 VFS）— O10・O11 の合格と同じ。**追加の指標**: 起動から first idle までに取得した bytes（索引 + seed のみ）、1 分後の先読み量、OPFS の総量が 1.62 GB に近づく過程。**失敗**: first idle 前に 1.62 GB を取りに行く（P1 を壊す）。

### O13 音を出す最初の一歩ができない（A/B/C。メトロノーム以外が鳴らない、1 拍目が鳴らない、OPFS の音声を FL が認識しない、Sampler に音声を指定すると落ちる、コード進行ツールで落ちる）
- **再現**: (a) **1 拍目**: `closeWizard` → メトロノームを有効（`ctrlKey KeyM`、0211 の手順）→ `audioMark play` → `key Space` → `audioDump` を 4 拍ぶん（テンポ 140 で約 1.7 s）→ 停止 → 繰り返し 10 回。(b) **OPFS の音声**: page のファイルマネージャー（0216）で WAV を OPFS に置く → Sampler の読み込みダイアログ（page の `fileDlg`、`openFile`）で選ぶ。(c) **Sampler に音声**: 選択の直後に `expectRunning` と `saveLog`。(d) **コード進行ツール**: ツールのボタンを `click`（座標は未確認）→ `expectRunning`。
- **合格**: (a) 開始から 4 拍の**クリックが 4 個**あり、1 拍目の peak が他の拍の 0.5 倍以上（`audioDump` の onset 検出。0211 は 10 s で 24 個・間隔 0.424〜0.429 s を観測）、10 回のうち欠落 0 回。『無音で切る』処理が app/ か runtime にないことの静的点検（grep: 無音の閾値で出力を捨てる・`soft-sink.ts` の役割・`audio-ring` の埋め）は mt3 の宿題（PM の決定 3 の待ち）。(b)(c)(d) 停止 0、(b) は FL が当該ファイルを開く（Problems 窓に出ない）。**失敗**: 1 拍目欠落、停止、無音。
- 仕組み: 1 拍目は DirectSound の write cursor・遅れた書き込み（0241・0295）の契約の再点検（B）と、我々の無音処理の除去（A）。停止は census。

### O14 変な落ち方がまだある（特定の画面の特定のつまみ）／『仕組みを直す』方針の徹底（方針）
- **再現（系統的な走査。個別の再現ではない）**: 0215 の走査（native -O0 で 8,552 操作）のブラウザ版の抜粋: 各 plugin/窓（Mixer、Channel rack、Piano roll、Playlist、Settings 各タブ、プラグイン窓）の**つまみを 16 画素格子 × 左ドラッグ**で順に回し、各操作の後に `expectRunning` と `saveLog`。停止したら VA → API 名 → 契約を表にする。
- **合格**: 走査した N 操作（N は PM が決める。時間は未測定）で停止 0。**停止が出た場合の成果**: 個別の修正ではなく『欠けた契約』の一覧（API 名・出現回数・契約の出典）。**失敗**: 停止を 1 件ずつ直す形の報告（方針違反）。成果の報告に『(a) か (b) か、全体のどの仕組みを直すか』を必ず書く。

### O15 native でも再生中に窓を動かすと window ghosting が出る（D: FL 本来の挙動。我々は再描画・隠蔽の処理を足さない）
- **再現**: 再生中（`openZipProject` → `Space`）に Mixer のヘッダードラッグ（既存の座標 (715,564)→(1015,564)、`scn-move1920.json` の `dragMeasure`）を 8 s、`audioSnap` を前後に。窓を動かさない対照を同じ長さで。
- **比べるのは『消えるまでの速さ』と『メーターの fps・音が影響を受けないこと』だけ**: (1) T_clear（ドラッグを止めてから peek = whole（メーター除く）になるまで）を記録（native の参照は依頼者の目視『ギリギリ認識できる短時間』で数値は 0 件。**閾値は置かず記録**。500 ms を超えたら注記）。(2) メーターの更新/s（ドラッグ中 ÷ 対照）≥ 0.9。(3) 音: ドラッグ中の underrun quanta の増分が対照と同じ（±2 quanta）。
- **失敗**: メーターの更新/s が 0.9 倍未満、underrun の増分が対照より多い。**ghosting の存在自体は失敗にしない**。静的点検: 我々が窓の再描画・隠蔽を独自に足していないこと（`rg` で runtime/app の再描画・隠蔽のコードを列挙し、FL の挙動を代行する分岐が無いこと。A の監査は PM）。

## 3 実行の順と準備（提案。決めるのは PM）
1. gpu1 が先に O1・O5・O9・O13(a) の再現（Chrome 非計時、1 本ずつ、F0 とオラクルで判定、画像と engine log を保存）。ci1 は後で計時（present/s・T_clear）を取る。
2. **道具（07:1x に追加済み。実行は未）**: (i) `--dpr <n>`（O7。Chrome のみ、Firefox は無視して言う）、(ii) op `tClear{name,box,everyMs,stableMs,maxMs}`（操作の直後に呼ぶ。保持している絵〔peek〕が**メーターの箱の外で静止するまでの時間** `tStillMs` と、静止後に whole を **1 回**だけ取って peek と箱の外で比べた `ghostPx`〔0 = 残像なし〕、`series`。whole は resync するので繰り返さない。この定義が §0 の T_clear の実装）、(iii) op `meterRate{name,box,secs}`（rAF ごとに箱を小さい canvas に写してハッシュし、変化の回数/s と rAF/s を出す in-page の eval。読むだけ）、(iv) op `wheelSeq{x,y,dx,dy,n,gapMs}`（Settings → General 用。`wheel` は既存）。`tools/sdriveg2-export.mjs` のみ変更、`node --check` と `tools/scn-lint.mjs`（既存の scn が各 driver の op に対して 0 件の問題）で確認。**未確認**: About・Debug の tab の dx、Pattern clip・コード進行ツールの座標（画像で決める）、`drawImage` で transferred canvas の箱が読めるか（`meterRate` の `errs` で分かる）。いずれも**ツールの追加で、製品のコードには触れない**。
3. 資材（O10〜O12）は遅延 VFS の実装後。それまでは『一覧の件数の census 表（opt1）』だけ用意。
4. **未確認**: 依頼者の評価用プロジェクト、native の各 fps の我々の測定値（0 件）、Hz 上限 120 の扱い、遅延 VFS の仕様、`engineLog` に Debug タブの文面が出るか。
