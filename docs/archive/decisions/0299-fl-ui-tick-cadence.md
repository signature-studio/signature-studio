# 0299(仮) FL の UI の刻みの決まり方 — 何が刻みを決めるか、runtime が返している『リフレッシュ』の値、60 → 120 で追従するか(alpha1 = alpha18)

作業者 `alpha1`(成果物名 alpha18)、2026-11-06 JST 15:07〜。依頼: PM 15:12(決定 0299 仮、node/native のみ。Chrome は起動していない。第 1 報 16:00、最終 16:45)。基準: main 7373cd55(E11、CI 実行中)。区分: **実測**=本書の run の出力、**読み**=逆アセンブル・文書からの解釈、**未確認**。**この機械(16 vCPU の VM、load 8〜12)の値で、依頼者の機械・実 Windows の値は 0 件。**

**実エンジンの実行について**: §3 の実験は E11 の**テスト用 wasm**(`lift/out-opt2/tw-e17/bundle/fl-test-e17`、engine.abecc01c7b30 = FLEngine_x64.dll と同梱 DLL の lift 結果)を node 24.19 の harness(JSPI、仮想時計、G3 スクリプト `ref1-perf-short.g3`)で走らせた物。入力は harness の模擬(利用者の操作ではない)。仮想時計なので作業量が毎回同一(2 本の同条件の run で数値が一致)。**N1(native)のバイナリは 0291 の後始末で消してあり、本件では使っていない**(node の wasm で代用)。代役・再生・期待値の貼り付けは無い。

## 0. 先頭の結論

1. **FL の UI の刻みを決めているのは `EnumDisplaySettingsW` の `dmDisplayFrequency`(runtime の `FLE_DISPLAYSETTINGS_FREQUENCY`)で、`DwmFlush` の周期(`FLE_DWM_HZ`)ではない**(実測、§3)。2 つを別々に 60 / 120 にした 4 通りの組を走らせると、アイドルの UI の仕事(間接呼び出し/仮想 s)は **dmDisplayFrequency に比例(60 → 10.31 M/s、120 → 20.48 M/s = 1.99 倍)し、DwmFlush の Hz を 60 のままにしても 120 のままにしても変わらない**。= 『DwmFlush が 60 Hz の刻みを作っている』(0166 の読み)は半分違い、**刻みの量を決めるのは dmDisplayFrequency から FL が計算する『フレーム時間』**。
2. **仮に 120 Hz と伝えると FL の UI は 120 に追従する**(実測、E11、§3): アイドル 10.31 → 20.48 M/仮想 s(1.99 倍)、再生中 16.81 → 27.58 M/仮想 s(1.64 倍。ミキサー分は変わらず、UI 分が 1.99 倍)。**UI だけの CPU/仮想 s は 197 → 377 ms(κ = 1.92、アイドル、プロファイラ無し、60 仮想 s の差から。§3.2)**。run 全体では +27〜+34%(揺れ ±8%)。0166(E5 の頃、1 回・load 19)の 1.62 倍より大きい。**追従する → 『契約に忠実な作り』は、ブラウザの実際の表示 Hz(rAF の cadence)を dmDisplayFrequency と DwmGetCompositionTimingInfo の rateRefresh と DwmFlush の周期に正しく返すこと**で、製品はすでに『ページが rAF を測って 60/75/90/120 に丸めて `FLE_DISPLAYSETTINGS_FREQUENCY`/`FLE_DWM_HZ` で渡す』(0268)。
3. **UI を 120 Hz にすると P3 と 1 Worker では衝突する**(式、§4): UI 側の CPU を κ = 1.92 倍すると、60 Hz、v = 1 の UI(0.30〜0.52 コア、中央 0.39)は 120 Hz で **0.57〜0.99 コア(中央 0.75)**。1 Worker の予算 0.735(idle 26.5% を残す)に対し、ミキサー 0.62 コアと合わせた需要は **1.19〜1.62 コア(中央 1.37)**(60 Hz は 0.92〜1.15)。必要な全 CPU の削減は **38〜55%(中央 46%)**(60 Hz は 20〜36%、中央 27%)で、忠実な梃子の高側 27% でも届かない。**UI 専用のコア(UI Worker + ミキサー Worker の実スレッド版)が前提**(UI Worker 0.57〜0.99 コア、ミキサー Worker 0.71〜0.98 コア。UI Worker の上端は 1 コアに近く、UI 側の質〔L3 など〕の余地が要る)。
4. **QuickFontCache は DWM のタイミング情報を使えない状態で動いている**(静的、§2): QFC は `DwmIsCompositionEnabled` と `DwmGetCompositionTimingInfo` を `GetProcAddress` で探す。runtime の応答表(生成 C の `fle_resp_table`)には **`DwmIsCompositionEnabled` の行がある**(0171、TRUE を返す。`fle_modules.c:84-87` のコメントは 0122 の時点の記述で古い)が、**`DwmGetCompositionTimingInfo` の行は無く NULL**。QFC の wrapper(`0x1e48b40`)は関数ポインタが NULL のとき `E_NOTIMPL`(0x80004001)を返し、`f_1e857b0` は失敗として `EnumDisplaySettingsW` の分岐に落ちる。**実 Windows 8+ では後者が S_OK で rateRefresh を返す**ので、runtime の『行を持たない』は契約との差(§2 の表)。**行を足す実験(§3.3、実測)では、同じ Hz でも UI の仕事が −3.9%(60 Hz)/ −4.0%(120 Hz)、g3 の行が 55 行中 33 / 29 行動く**(QFC の分岐の丸め方が 2 経路で違う)ので『同じ Hz なら同じ』ではない。表示 Hz が 60 でないとき(rAF が 75/90/120)の整合は dmDisplayFrequency 1 つに依る。
5. **『120 fps』の解釈(§5)**: FL の UI の刻み(tick)は dmDisplayFrequency に追従して 120/仮想 s に上げられるが、**画面に出る更新(present)は画素が変わった刻みだけ**で、0166・0268 の実測では 1 秒に数回〜十数回(再生中 約 12/s)。= 『FL が描くものが 120 fps 表示可能であること』と『画面更新の遅延が小さいこと』は別で、前者は刻み = 120 に追従する(CPU 約 1.6 倍)、後者は『入力 → 次の刻みまでの待ち』が 16.7 ms → 8.3 ms に縮む(最悪値)。gpu1 の 0297 が present の実測を出すはず。

## 1. 方法と範囲

- (a) 逆アセンブル: `lift/out-opt2/x17/engine/in/dll/` の FLEngine_x64.dll・quickfontcache_x64.1d00000.dll・ILTools_x64.6000000.dll(`objdump -d`、`pefile` + `capstone`〔`tools/a19-qfc-xrefs.py`: import の jmp stub を拾い、呼び元の関数を `.pdata` で当てる〕)。
- (b) runtime の読み: `lift/runtime/fle_thread.c`(`fle_display_hz`・`fle_display_settings_hz`・`fle_imp_DwmFlush`)、`fle_monitor.c/.h`(EnumDisplaySettingsW)、`fle_gdc.c`(GetDeviceCaps)、`fle_winmm.c`(timeBeginPeriod)、`fle_sys.c`(QPC)、`fle_dwm.c`、`fle_modules.c`(dwmapi の行)。
- (c) 実験: §3。E11 のテスト用 wasm、node、`FLE_DWM_HZ` と `FLE_DISPLAYSETTINGS_FREQUENCY` を harness の env(manifest の `engine.env` と同じ経路。**再 build なし**、製品の runtime が既に持つ env)で変えた。各設定 1〜2 本(同条件の 2 本は数値が一致)。
- 先行の決定: 0166(E5 の頃、runtime の 2 つの定数を 60 → 120 に build し直して比べた。UI の周期 2.00 倍、CPU 1.62 倍/仮想 s)、0268(ページが rAF を測って 60/75/90/120 を渡す)、0122(DwmFlush)、0098(EnumDisplaySettings)。**本書は E11 の上で env だけを変え、2 つの値を別々に動かして『どちらが刻みを決めるか』を分けた**のが新しい点。

## 2. 表 1・2 — 刻みに関わる API の表(関数名つき)と、『リフレッシュ』の値の契約対応

### 2.1 FL が刻みの周辺で使う API(静的な import と呼び元、`tables/0299-qfc-api-callers.txt`)

| API | FLEngine | QuickFontCache(QFC) | ILTools | 刻みへの関与(根拠) |
|---|---|---|---|---|
| `EnumDisplaySettingsW`(`dmDisplayFrequency`) | 有 | 有(**`f_1e857b0` が呼ぶ**) | 有 | **律速**(§3 の分離実験)。QFC の `f_1e857b0(n)` が『n+1 フレームの時間(ms)』を返す: 先に DWM のタイミング情報を試し(`cbSize = 0x124` の `DWM_TIMING_INFO`、`rateRefresh` の分数から `(n+1)×1000×den/num`)、失敗なら `EnumDisplaySettingsW` の `dmDisplayFrequency`(**50 以上なら** `1000/Hz × (n+1)` を四捨五入、未満なら固定の係数)。呼び元(`0x1e85ee6`・`0x1e85eff`)は、要求された間隔 `[this+0x6c]` を『フレーム時間』の整数倍に丸めて `[this+0x70]` に書く(`(間隔/フレーム時間 − 1) × フレーム時間 + フレーム時間/2`)。= **タイマの間隔を表示のフレームに量子化する** |
| `DwmFlush`(`LoadLibraryA("DWMAPI.DLL")` + `GetProcAddress`) | 無 | 有(名前の文字列が 1 件。TTimerThread が毎周期) | 無 | runtime は『仮想時計の次の Hz 境界まで待って S_OK』(`fle_imp_DwmFlush`)。**Hz を 60 のまま / 120 に変えても UI の仕事量は変わらない**(§3)= スレッドの起床の周期であって、UI の刻みの量ではない |
| `DwmGetCompositionTimingInfo`(`GetProcAddress`)/ `DwmIsCompositionEnabled` | 無 | 有 | 無 | **`DwmGetCompositionTimingInfo` は runtime に行が無く NULL**(QFC の wrapper が `E_NOTIMPL` を返す)。`DwmIsCompositionEnabled` は行があり TRUE(0171)。QFC は前者の失敗で `EnumDisplaySettingsW` の分岐に落ちる |
| `SetTimer`/`KillTimer`(WM_TIMER) | 有 | 無 | 有 | **刻みを作っていない**: G3 の `timers` 出力 `g3-timers-so-far` は全 run で 0(10 回)。0166 も WM_TIMER の配送 0 件 |
| `timeBeginPeriod`/`timeEndPeriod`/`timeGetTime` | 有 | 有(`f_1e85e20`) | 有 | runtime は範囲検査と数え上げだけ(`fle_winmm.c`)、仮想時計の粒度は変えない。**刻みに無関係** |
| `QueryPerformanceCounter`/`GetTickCount` | 有 | 有(`f_1e1c020`・`f_1e60460`・`f_1e82290` ほか) | 有 | 時間の差を取る用途(読み)。仮想時計の 10 MHz(`FLE_QPF_HZ`)。**刻みの決定には出ない** |
| `InvalidateRect`/`UpdateWindow`/`RedrawWindow`/`ValidateRect`(WM_PAINT) | 有 | 無 | 有 | 描画の要求。**画素が変わらない刻みでは何も描かない**(0166: 画素の書き込みは 0.96 仮想 s に 1 回)。刻みを律速しない |
| `MsgWaitForMultipleObjects`/`PeekMessageW`/`PostMessageW`/`Sleep` | 有 | 有(`f_1df1ee0`・`f_1e85a70`・`f_1e84d90` ほか) | 有 | TTimerThread のメッセージ伝達。刻みの上限は dmDisplayFrequency から作る間隔 |
| `GetDeviceCaps(VREFRESH)` | import は有(`GetDeviceCaps` 自体) | 有 | 有 | **runtime は VREFRESH(116)を答えない**(`fle_gdc.c`: 答える 9 index 以外は名前つきで stop。『guest の call site は使わない』という記述)。全 run で stop は無い。**静的にも確認**(`tools/a19-gdc-args.py`、`tables/0299-getdevicecaps-indexes.txt`): FLEngine 31・QFC 11・ILTools 16 箇所の GetDeviceCaps 呼び出しの index は全部定数で 4・6・8・10・12・14・88・90・104 のみ、**116(VREFRESH)は 0 箇所** = FL は VREFRESH を問い合わせない |

### 2.2 runtime が返している値と、Learn の契約(`fle_*.c` の読み + Learn の頁)

| API | runtime が返す値(E11) | Learn の契約 | 差・判定 |
|---|---|---|---|
| `EnumDisplaySettingsW`(ENUM_CURRENT_SETTINGS) の `dmDisplayFrequency` | `fle_display_settings_hz()` = env `FLE_DISPLAYSETTINGS_FREQUENCY`(24..1000)、無ければ `FLE_DWM_HZ`、無ければ 60。**製品ではページが rAF を測って 60/75/90/120 に丸めて渡す**(0268) | DEVMODEW:『For displays, specifies the frequency, in hertz, of a display device in its current mode』 | **一致**(値は実際の表示 Hz を渡す限り契約どおり。丸め(0268)は『現在のモード』の Hz を近似する選択)。**刻みを決める値** |
| `DwmFlush` | 仮想時計の次の `FLE_DWM_HZ` 境界まで待って `S_OK`(`fle_imp_DwmFlush`。1 スレッドなら時計を進める) | 『blocks the caller until the next call to a Present method, when all of the DirectX surface updates currently outstanding have been made... waits for any queued DirectX changes... to be drawn to the screen before returning』。`S_OK` | **選択**(Learn は待つ長さを書かない)。実 Windows では compositor の次のフレームで返るので表示 Hz に近い。runtime は**仮想時計の境界**で、ブラウザの実際の rAF/vblank とは結んでいない(位相も固定されない)。刻みの量は変えない(§3)が、**入力 → 画面の遅延の位相**は rAF と無関係になる |
| `DwmGetCompositionTimingInfo(NULL, &info)`(`cbSize` 設定) | **行なし(GetProcAddress が NULL)** | 『Retrieves the current composition timing information』、8.1 以降は hwnd が NULL でなければ `E_INVALIDARG`。`DWM_TIMING_INFO.rateRefresh`(UNSIGNED_RATIO)ほかを返し `S_OK` | **差**(実 Windows 8+ では存在)。QFC の `f_1e857b0` は DWM の rateRefresh を先に使う設計なので、実 Windows では `rateRefresh` = 表示 Hz の分数から n フレームの時間を作る。runtime が NULL を返すので EnumDisplaySettings の分岐になる。**契約に忠実にするなら『rateRefresh = dmDisplayFrequency/1 の分数、cbSize 0x124 の構造体を埋めて S_OK』を返す**(外部 API の実装。今は結果の Hz が同じなので刻みは変わらない見込み = **未確認**) |
| `DwmIsCompositionEnabled` | `TRUE`(`fle_dwm.c`、生成 C の `fle_resp_table` に行あり。0171) | 『As of Windows 8, always enabled... receives TRUE if the app declares Windows 8 compatibility』 | **一致**(FL64.exe の manifest が Windows 8/8.1 を宣言、0171 の S-ia1-dwm-1)。(第 1 報の『QFC の GetProcAddress は NULL』は誤り: 行は全モジュール共通の応答表にある) |
| `GetDeviceCaps(hdc, VREFRESH=116)` | **stop**(答えない 9 index の外) | 『the current vertical refresh rate of the device, in cycles per second (Hz); a value of 0 or 1 represents the display hardware's default refresh rate』 | FL が問い合わせていない(全 run で stop なし)ので今は無害。**問い合わせられたら dmDisplayFrequency と同じ Hz を返すのが契約**(cheap) |
| `timeBeginPeriod`/`timeGetTime`/`QueryPerformanceFrequency` | NOERROR / 仮想時計の ms / 10 MHz | Learn の定義どおり(範囲外は TIMERR_NOCANDO、QPF は固定) | 一致(刻みに無関係) |

### 2.3 1 刻みの中で FL の UI thread が何を走らせているか(E11 の Chrome の cpuprofile、再生 30〜50 s の 192 ブロック、FL main の呼び出しの木、ms/ブロック)

(0296 の入力 `ser-e17p` の再集計。ブロックは音声の 4096 フレーム。関数名は VA。役割は呼び出しの構造からの読みで、シンボルではない。)

| 経路(`f_86a250` = UI thread の main loop の下) | 包含 ms/ブロック | 読み |
|---|---|---|
| `f_86a250.body` 全体 | **21.14** | FL main(0296 の 21.4 ms/ブロック)の 99% |
| ├ `f_86b440 → f_86b300 → f_863c20 → f_863b50`(再帰) | **11.4**(54%) | **idle 側**: メッセージが空のときの VCL の木の歩き(`f_863b50`・`f_863f60` の再帰走査、アクションの更新。0166 の『描かない周期のコントロール木の歩き』) |
| └ `f_86a0f0 → fle_imp_DispatchMessageW → 窓 proc(wasm#99804)` | **9.7**(46%) | **メッセージ側**: 配送されたメッセージの処理 |
| 　├ `→ f_52a570` | 6.25 | VCL の窓 proc(アプリのメッセージ処理) |
| 　└ `→ f_1e858e0 → f_1e86280.body` | **3.44** | **QuickFontCache の窓 proc**(`f_1e858e0` は QFC の VA 範囲 0x1e858e0。TTimerThread が `PostMessageW` した刻みのメッセージを受ける側) |

= 1 刻みは **(i)QFC が投函するメッセージの処理(3.4 ms/ブロック)と VCL のメッセージ処理(6.3)、(ii)メッセージが空になった後の idle の木の歩き(11.4)**。(ii)が過半で、**刻みの数 × 木の大きさ**に比例する(0166・0282: 1 刻みあたり約 5 ms のうち `f_863f60` が 1.9 ms)。= dmDisplayFrequency を 120 にして刻みが 2 倍になれば、(i)(ii)とも 2 倍になる(§3 の 1.99 倍と整合)。

### 2.4 刻みを発行する経路 — QFC 以外はあるか(PM 15:38 の (1))

**取れなかった物(理由 1 行)**: 命令単位の呼び出し追跡は無い。G3 の API トレース `FLE_FRUITY_API_TRACE` は native の idle ハーネス(`lift/harness/idle/idle.c`)専用で、wasm のハーネスは読まない(env を渡した run で出力 0 行、実測)。mt2 の census(`mt2_trace.c`)は `-Wl,--wrap` の別 link が要る。→ 次の 3 つの証拠で代用した。

| 経路の候補 | 証拠 | 結果 |
|---|---|---|
| WM_TIMER(`SetTimer`) | 静的: `SetTimer` の呼び出しは FLEngine 13 箇所・ILTools 3 箇所(`tools/a19-gdc-args.py FLEngine_x64.dll SetTimer r8d r8`)。間隔は定数 50 ms と 20 ms が各 1 箇所、残りはコントロールのメンバ値(VCL の `TTimer.Interval` 型)。実測: G3 の `g3-timers-so-far`(runtime の WM_TIMER 配送ログ `fle_win_timer_log_count`)は **全 10 snap で 0**(起動 → プロジェクトを開く → アイドル → 再生) | **この場面では刻みを発行していない**(WM_TIMER の配送 0 件)。FL のどこかが条件つきで使う可能性はある(未確認) |
| QFC の TTimerThread(`DwmFlush` → `PostMessageW`) | E11 の Chrome プロファイル: UI main loop の**メッセージ側** 9.7 ms/ブロックに QFC の窓 proc `f_1e858e0 → f_1e86280` 3.44 ms/ブロックが載る(§2.3)。実験: dmDisplayFrequency を 2 倍にするとメッセージ側も idle 側も 2 倍(UI の仕事 1.99 倍、§3) | **QFC が投函する刻みが主**。ただし『メッセージ側の過半 6.25 ms/ブロック(`f_52a570`、VCL の窓 proc)は、QFC 以外のメッセージ』で、何のメッセージかは未確認 |
| main loop の idle(`f_86b440 → … → f_863b50`、VCL の `Application.Idle` の木の歩き) | プロファイル: idle 側 11.4 ms/ブロック(UI の 54%) | メッセージが空になるたびに走る = **メッセージの数(刻み)に比例**。刻みを発行するのではなく、刻みごとに走る後始末 |
| 他のスレッドからの投函(音声・プラグインのスレッド) | 未測定 | 未確認 |

= **刻みの発行源は、この場面では QFC の投函(量は dmDisplayFrequency 由来のフレーム時間で量子化)が確認できた唯一の経路**。VCL の窓 proc 側のメッセージ(6.25 ms/ブロック)の出どころは追跡できていない。

## 3. 実験 — 2 つの値を別々に動かす(E11 のテスト用 wasm、node、`tables/0299-hz-experiment.tsv`)

E11 のテスト用ビルド(engine.abecc01c7b30、再 build なし)に env だけを与えた。シナリオ `ref1-perf-short.g3`: 起動 → 参照プロジェクトを開く → 『欠けたプラグイン』の窓を Enter で閉じる → **アイドル 8 仮想 s** → Space で**再生 10 仮想 s** →停止。計測 = harness の `g3-progress`(間接呼び出しの数 = `fle_progress_count`、課金 ticks、スライス、thread 切り替え)の差を各区間の仮想秒数で割った値(仮想時計なので毎回同一)。

| FLE_DWM_HZ(DwmFlush) | FLE_DISPLAYSETTINGS_FREQUENCY(dmDisplayFrequency) | アイドル M/仮想 s | 再生 M/仮想 s | アイドルの thread 切替/s | 再生の thread 切替/s |
|---|---|---|---|---|---|
| 60 | 60 | **10.31** | 16.81 | 2 | 398 |
| 120 | 120 | **20.48** | 27.58 | 4 | 439 |
| **60** | **120** | **20.48** | 27.58 | 4 | 439 |
| **120** | **60** | **10.31** | 16.81 | 2 | 398 |

- 各設定の組で**同じ値が再現**(60/60 と 120/120 は 2 本ずつ、4 桁一致)。env を 2 つ別々に与えると runtime が『the guest reads one and is paced by the other』を stderr に出す(出ている)。
- **UI の仕事は dmDisplayFrequency に比例し(10.31 → 20.48 = 1.99 倍)、DwmFlush の Hz とは無関係**。再生中は 16.81 → 27.58(1.64 倍)で、差の 10.77 M/s がアイドルの UI の増分 10.17 M/s とほぼ等しい = **ミキサー分(約 6.5 M/s)は変わらず UI 分だけが 1.99 倍**。
- CPU(run 全体、約 42 仮想 s): V8 の 2 ms プロファイルの非 idle は 60 Hz 29.7 / 25.5 s、120 Hz 37.4 / 36.4 s(+34%、load 8〜12 で揺れ ±8%)。`/usr/bin/time` の user+sys は 60 Hz 38.4 / 33.2 s、120 Hz 47.3 / 44.4 s(+27%)。DWM / SET を別々にした 2 本は SET に従う(60/120: 43.8 s = 120 側、120/60: 35.3 s = 60 側)。

### 3.2 UI だけの CPU/仮想 s(κ の分離、`tables/0299-ui-cpu-kappa.tsv`、実測)

アイドルの区間だけを長さ違いの 2 本で引き算した(起動・プロジェクトを開く・Problems の窓を閉じるまでが共通、その後のアイドルが 8 仮想 s 対 68 仮想 s。プロファイラ無し、node の `/usr/bin/time` の user+sys、各 2 回): **60 Hz: 20.41 → 32.20 s(60 仮想 s の差 11.80 s)= 197 ms の CPU/仮想 s、120 Hz: 25.90 → 48.53 s(22.63 s)= 377 ms/仮想 s、κ = 1.92**(間接呼び出しの 1.99 倍に近い。0166 の 1.62 は 1 回・load 19 の値)。2 回の揺れは ±5% 以内。
- 読み(§2.1 の逆アセンブルと合わせて): QFC は表示のフレーム時間(dmDisplayFrequency から)を単位に**タイマの間隔を量子化**する(`[this+0x70]`)。DwmFlush は TTimerThread の起床を表示に揃えるだけで、起床した後の仕事の量(メッセージの数)は間隔が決める。**確かめていない点**: FL の UI の tick を実際に発行する関数が、どの間隔(15 ms / 33 ms など)をどう丸めるか(QFC 以外の FL 本体の経路もあるか)は読み切っていない(命令単位の追跡が要る)。

### 3.4 再生中の UI の CPU(60 Hz と 120 Hz の差から、実測 + κ の仮定、`tables/0299-play-cpu.tsv`)

再生の区間だけを長さ違い(10 仮想 s 対 40 仮想 s の再生、前段のアイドル 8 s + Space + 3 s は共通)の 2 本の差で取り、60 Hz と 120 Hz で比べた(プロファイラ無し、user+sys、各 2 回、load 6〜12): **CPU/再生の仮想 s(UI + ミキサー + 他)は 60 Hz 1.175 core(34.13 → 69.38 s の 30 仮想 s)、120 Hz 1.462 core(45.03 → 88.88 s)**。ミキサーの分は Hz に依らないので、**差 0.287 core = (κ − 1) × UI(60 Hz、再生)**。κ = 1.92(§3.2、アイドルの実測)を再生にも当てると **UI(60 Hz、再生、v = 1)= 0.31 core**(κ = 1.64〜1.99 なら 0.45〜0.29)、ミキサー + 他 = 0.86 core/仮想 s(node は Chrome より重く、user+sys は背景 thread を含む)。**揺れが大きい**(同条件の 2 本で 60 Hz の p10 が 31.0 / 37.3 s、p40 が 62.4 / 76.4 s。±10%)ので、差 0.287 core の不確かさは約 ±35%。

= §4 の U60 の 3 通り(0.30 / 0.39 / 0.52)のうち、**node の再生の実測から逆算した 0.31 は『UI は壁時間あたり一定』の 0.30 に近い**(Chrome のプロファイラ付きの値 0.52 や E7b の端点 0.39 より小さい)。幅として §4 に入れる。

### 3.3 `DwmGetCompositionTimingInfo` の行を足す実験(PM 15:38 の (2)、実測。製品には入れない、差分案のみ)

**build**: E11 のテスト用 work dir の複製(`/home/ubuntu/work/alpha18w`、PM の木の外)に、runtime の `fle_dwm.c` へ `fle_imp_DwmGetCompositionTimingInfo`(hwnd ≠ NULL は `E_INVALIDARG`、`cbSize` < 12 は `E_INVALIDARG`、それ以外は `rateRefresh` = `{dmDisplayFrequency, 1}` を +4・+8 に書いて `S_OK`。他のメンバは触らない)を足し、生成 C の 2 つの表(dispatch の表と `fle_resp_table`)に `dwmapi.dll / DwmGetCompositionTimingInfo` の行を足した。`wasm weblink wasmopt hybrid seed bundle` の段階を E11 と同じ設定で回した(15:45〜15:58、nice 19、`BINARYEN_CORES=3`)。engine.62dd2c67b877。差分案: `tables/0299-dwm-timing-info-EXPERIMENT-NOT-FOR-MERGE.diff`。**同じ build で env `FLE_EXP_NO_DWMTI=1` を渡すと関数が `E_NOTIMPL` を返す = 行が無いのと同じ**(QFC 自身の wrapper が NULL のとき返す値と同じ)ので、1 つの build で『行なし(base)』『行あり(row)』の 2 つの腕を比べた。

| 腕 | Hz(DWM = SET) | アイドル M/仮想 s | 再生 M/仮想 s | アイドルの thread 切替/s | 再生の thread 切替/s | g3 の行(55 行)のうち base と違う行 |
|---|---|---|---|---|---|---|
| base(行なし) | 60 | **10.306** | 16.808 | 2 | 403 | — |
| row(行あり) | 60 | **9.900(−3.9%)** | 16.387(−2.5%) | 2 | 391 | **33 / 55**(snap 10/10、progress 10/10、surface 12/23、wintable 1/10) |
| base | 120 | **20.478** | 27.585 | 6 | 450 | — |
| row | 120 | **19.664(−4.0%)** | 26.765(−3.0%) | 3 | 432 | **29 / 55**(snap 10/10、progress 10/10、surface 9/23、wintable 0/10) |

(`tables/0299-dwm-row-experiment.tsv`。base の腕は、行を足していない既存の E11 のテスト用 wasm の数(10.31 / 16.81 / 20.48 / 27.58)を**再現**している。)

**結果(実測)**:
1. **行を足すと QFC の分岐が実際に変わる**(進捗の数が違う)が、**フレーム時間が同じでも UI の仕事は同じにならない**: アイドルで **−3.9%(60 Hz)/ −4.0%(120 Hz)**、再生で −2.5% / −3.0%。120 Hz の 2 倍の比は保たれる(20.48 / 10.31 = 1.99 → 19.66 / 9.90 = 1.99)。
2. **g3 の行は一致しない**: 60 Hz で 55 行中 33 行、120 Hz で 29 行が違う。内訳は**時刻・進捗の行(`g3-snap` の `clock_ms`、`g3-progress` の数。全 10 snap)**と、**画素の sha(`g3-surface` 23 のうち 12 / 9)**。**窓の表(`g3-wintable`)は 60 Hz で 1/10、120 Hz で 0/10 しか違わない**(窓の構成・位置・大きさはほぼ同じ。画素の差は時刻のずれによるアニメ・メーターの位相と読む)。
3. **読み(逆アセンブルの `f_1e857b0` と呼び元 0x1e85ee6/0x1e85eff から)**: DWM 経路は**正の値**(フレーム時間 ms)を返し、呼び元はそれを使って要求間隔 `[this+0x6c]` を『フレーム時間の整数倍』に丸める式(`(round(間隔/フレーム時間) − 1) × フレーム時間 + フレーム時間/2`、0x1e85f0d 以降)に進む。EnumDisplaySettingsW の経路(fallback)は**負の値**を返し、呼び元はそれを `[this+0x70]` にそのまま書く。= **同じ Hz でも 2 つの経路の刻みの丸め方が違い**、実 Windows 8+ の FL は前者(DWM 経路)で動く。

**評価(外部 API の契約の実装としての候補)**:
- **忠実さ**: 契約どおり(Learn: Windows 8.1 以降は `hwnd` が NULL でなければ `E_INVALIDARG`、`DWM_TIMING_INFO` の `rateRefresh` を返し `S_OK`。実 Windows 8+ に存在する関数)。runtime が FL の刻みを代行して増やすのではなく、**FL が実 Windows で通る分岐を通すための値を返す**。→ 分類 A(忠実)。
- **効果**: UI の仕事 −2.5〜−4%(60 Hz でも 120 Hz でも同程度)。= 取り得る CPU 削減の梃子としても小さな正の値(ただし『同じ Hz なら同じ』ではなく、**挙動が変わる**変更なので、node の行 55/87・a4 の期待値・UI の画像の基準が動く)。
- **リスク**: 実装した `DWM_TIMING_INFO` は `rateRefresh` だけを埋める。QFC が他のメンバ(`qpcVBlank`・`cRefresh` など)を読む経路があるか、FL 本体や他の DLL が使うかは**未確認**(QFC の `f_1e857b0` は rateRefresh のみ。FLEngine・ILTools は `DwmGetCompositionTimingInfo` の名前を持たない)。実装する場合は Learn に載る全メンバを仮想時計から埋める(qpcRefreshPeriod = QPF/Hz、cRefresh = 仮想時計の境界の数、など)のが契約に忠実で、その場合の追加の影響は未測定。
- **勧め**: 入れるなら E12 以降の『挙動が変わる契約の追加』として、node の行・a4・画像の基準を同時に更新する別の変更に(今回の差分案は node の行を 29〜33 行動かす)。単独の CPU の梃子としては小さい。

## 4. 式 — UI を 120 Hz にしたときの CPU と P3(1 Worker では不可、UI 専用コア)

記号(0296): v = 仮想 s/実 s、r = 書き込み率、s = 0.265(ゲストの仮想時間の眠りの割合。v = idle ÷ s の当て嵌め)、1 Worker の稼働予算 = 1 − s·v。音声側の需要 A(r) = 0.622 core(r = 1、ミキサー + 他のゲスト thread、E11 協調)、UI 側の需要 U(v, Hz)。

- 60 Hz、v = 1 の UI 側 U60: 0296 の 3 通り = **0.30**(UI は壁時間あたり一定)/ **0.39**(E7b の実測 UI+ループ 0.342 + (program) 0.048、0282)/ **0.52**(v に比例)。需要 D60 = A + U60 = 0.622 + U60 = 0.92 / **1.012** / 1.14、予算 0.735 → 必要な削減 20 / **27%** / 36%(0296 §4.2)。
- **120 Hz**: U120 = U60 × κ、κ = **1.92**(§3.2、CPU、アイドル)。U120 = 0.57 / **0.75** / 0.99 core。D120 = 0.622 + U120 = 1.19 / **1.37** / 1.62。必要な全 CPU の削減 = 1 − 0.735 / D120 = **38% / 46% / 55%**(60 Hz の 20 / 27 / 36% に対し)。**node の再生の実測から逆算した U60 = 0.31(§3.4)を使うと U120 = 0.60、D120 = 1.22、削減 40%**(60 Hz は 0.622 + 0.31 = 0.93 で 21%)。= 中央を 40〜46% と読む。(注: node のアイドルの UI は 60 Hz で 0.197 core、再生中の Chrome のプロファイルから逆算した UI は 0.52 core。条件が違う〔再生・プロファイラ・ブラウザ〕ので幅として扱う。)
- 0296 の梃子の合計(低 3% / 中 13% / 高 27%)は**どれも 38% に届かない** → **1 Worker のままで『120 Hz の UI + P3(r ≥ 0.95)』は成立しない**。UI の増分 +0.27〜+0.47 core(U60 の中央 0.39 なら +0.36)が丸ごと不足になる。
- **実スレッド版(UI Worker + ミキサー Worker)**: UI Worker = U120 = **0.57〜0.99 core**(中央 0.75。上端は 1 コアに近い)、ミキサー Worker = 0.62 × (1 + 実スレッドの税 +15〜+58%) = **0.71〜0.98 core**(0296 §6)。= 2 Worker 構成なら 120 Hz でも各 Worker が 1 コア未満の見込み。**ただし 3 コア以上(UI + ミキサー + ブラウザの他のスレッド)が要る**(0289 §8: 2 コアでは協調 1 Worker より悪い)。
- 感度: v < 1 の協調では UI の tick は v に比例して減る(再生中 v = 0.575 なら 120 Hz の UI は実時間あたり 69 tick/s に下がる)ので、**v を上げるほど UI の CPU が増える**(式の U(v, Hz) ∝ v × Hz)。120 Hz を本当に 120 tick/実 s にするには v = 1 が条件で、そこで上の 1.19〜1.62(中央 1.37)になる。

## 5. 『120 fps』の解釈と、追従するときの作り(契約に忠実な外部 API の実装)

- FL は刻み(tick)で再計算し、**画素が変わった刻みだけが画面に出る**(0166: 画素書き込み 0.96 仮想 s に 1 回、0268: 提示は『画素が変わった rAF だけ』)。= **tick が 120 になっても present は増えない**(0268 の 60 → 120 Hz の実測で、提示は 5/s 前後)。依頼者の『120 fps』の読み方:
  (A) **『FL が描くものが 120 fps で表示できること』**: 画面上で動く物(再生位置の線、メーター、時計、ドラッグ中の物)が 120 Hz の刻みで更新されること。→ dmDisplayFrequency = 120 を返せば FL の刻みは 120 に追従する(§3)。CPU は §4。present は動く物の数に比例(再生中 約 12/s、ドラッグ 30 更新/s 程度)で 120/s には達しない設計(動いていない物は描かない)。
  (B) **『画面更新の遅延が小さいこと』**: 入力 → 次の刻み → 描画 → present。刻みが 120 Hz なら刻みの待ちが最悪 8.3 ms(60 Hz は 16.7 ms)に縮む。present 自体はブラウザの rAF(表示の vblank)に載る。
- **追従するときの契約に忠実な作り(外部 API の実装、runtime が FL の刻みを代行して増やすのではない)**: ① `dmDisplayFrequency` = ブラウザの実際の表示 Hz(0268 の rAF の測定。上限 120)を返す(**製品に既にある**)。② `DwmGetCompositionTimingInfo` を実装し `rateRefresh` に同じ Hz、`cRefresh`/`qpcVBlank` に仮想時計から作る値、`S_OK` を返す(現状は NULL)。③ `DwmFlush` を**仮想 Hz 境界でなく、ページの rAF(実際の vblank)に結ぶ**(Worker → ページの通知。v < 1 の協調では仮想時計が実時間に追いつかない問題が残る = §4 の v の議論)。④ `GetDeviceCaps(VREFRESH)` を同じ Hz で答える。②〜④は『値を正しく返す』で、刻みを runtime が増やす物ではない。

## 6. 未確認・出典なしの選択・逸脱

**未確認**:
1. FL の UI の tick を発行する関数が、どの間隔をどう `f_1e857b0` の『フレーム時間』に丸めるか(QFC 以外の FL 本体の経路の有無)。逆アセンブルは `f_1e857b0` とその呼び元までで、命令単位の追跡は未実施。
2. 実 Windows での `DwmGetCompositionTimingInfo` の `rateRefresh` と QFC の動作(実機の値は 0 件)。runtime が行を持たない差は**測定した**(§3.3: 同じ Hz で UI の仕事 −4%、g3 の行が動く)。実装した関数が埋めるのは `rateRefresh` だけで、Learn の全メンバを埋めた場合の影響は未測定。
3. 120 Hz の UI の CPU(κ)は **アイドルでは 1.92 を測った**(§3.2)。再生中・ドラッグ中の κ は未測定(再生の UI 分は間接呼び出しで 1.99 倍)。
4. ブラウザでの present の実数(gpu1 の 0297 の担当)。
5. N1(native)での同じ実験(バイナリが無い。node の wasm で代用)。

**出典なしの選択**: S-a19-1 κ = 1.92(アイドルの実測を再生にも当てる)。S-a19-2 UI の 60 Hz の需要 0.30 / 0.39 / 0.52(0296 の 3 通り)。S-a19-3 ミキサー Worker の税 +15〜+58%(0296)。

**逸脱・自己申告**: node の run 6 本(各 26〜38 秒、nice 10、load 8〜12)と 2 ms プロファイル 4 本(他の測定と重なり得た)。`/tmp/opencode/a19` の中間物(qfc の逆アセンブル 数 MB、run の出力)は最後に削除する。

## 7. 成果物(`lift/out-git/alpha18-results/`)

| 物 | 内容 |
|---|---|
| `docs/0299-fl-ui-tick-cadence.md` | 本書 |
| `tables/0299-hz-experiment.tsv`、`0299-ui-cpu-kappa.tsv`、`0299-getdevicecaps-indexes.txt` | §3 の表(4 組の設定、6 run)、§3.2 の κ、FL の GetDeviceCaps の index |
| `tables/0299-qfc-api-callers.txt` | QFC の import API ごとの呼び元関数(VA 範囲) |
| `tables/0299-qfc-f_1e857b0-disasm.txt` | QFC の `f_1e857b0`(フレーム時間の計算)の逆アセンブル |
| `logs/0299-g3-*-g3-all.txt` | 各 run の `g3-` 行(`g3-progress` を含む) |
| `tools/a19-g3-run.sh`、`a19-qfc-xrefs.py`、`a19-gdc-args.py`、`a19-uitree.py`、`a19-i8.g3`・`a19-i68.g3` | 実験の駆動(`EXTRA_ENV` で env を渡す)、呼び元の探索、GetDeviceCaps の index、UI の呼び出しの木、κ の分離用のスクリプト |

## PM の決定と取り込み(2026-11-06 16:19、docs のみ。製品の差分なし)

取り込み: alpha1(alpha18)の最終の文書・表・道具を `docs/investigations/alpha18/` に置いた(生のログは入れない。原本は `lift/out-git/alpha18-results/logs/`)。実行したのは E11 のテスト用 wasm とその複製 build を node(JSPI、仮想時計、G3 の模擬入力)で走らせた実験と逆アセンブルで、**Chrome・N1・実 Windows・依頼者の機械は 0 件**。`0299-dwm-timing-info-EXPERIMENT-NOT-FOR-MERGE.diff` は実験用で、製品には入れない。

事実として受け取る:
1. **UI の刻みの量を決めるのは `EnumDisplaySettingsW` の dmDisplayFrequency**(QuickFontCache の `f_1e857b0` が「n+1 フレームの時間」を作り、タイマ間隔をフレームに量子化)。DwmFlush の周期(FLE_DWM_HZ)は刻みの量に無関係(4 通りの分離実験)。120 Hz と伝えれば FL の UI は 1.99 倍で追従し、UI だけの CPU は κ = 1.92。FL は GetDeviceCaps(VREFRESH) を問い合わせない。WM_TIMER の配送は 0 件。確認できた刻みの発行源は QFC の投函だけで、VCL 側の窓 proc のメッセージ(6.25 ms/ブロック)の出どころは未追跡。
2. runtime には DwmIsCompositionEnabled の行はあるが DwmGetCompositionTimingInfo の行が無く(E_NOTIMPL)、QFC は EnumDisplaySettingsW の経路に落ちる。行を足す実験(rateRefresh = dmDisplayFrequency)は UI の仕事 −2.5〜−4%、ただし g3 の行が 55 中 29〜33 動く(時刻・進捗・画素)。
3. 再生中の UI(60 Hz)は約 0.31 core(不確かさ ±35%)。120 Hz の UI + P3 を 1 Worker で満たすには全 CPU の 40%(範囲 38〜55%)の削減が要り、忠実な梃子の高側 27% でも届かない。= **120 Hz の UI は UI 専用コア(実スレッド版)が前提**。

判断:
1. **runtime が FL の刻みを増やす・間引く代行はしない**(lift 忠実性)。契約に忠実な作りは、dmDisplayFrequency・DwmGetCompositionTimingInfo の rateRefresh・DwmFlush にブラウザの実際の rAF の Hz を返すこと。製品は既に rAF の Hz を 60/75/90/120 に丸めて dmDisplayFrequency に渡している(決定 0268)。依頼者の機械(GTX 980、モニタの Hz は不明)でこの値がどうなるかは 0 件。
2. **DwmGetCompositionTimingInfo の行を足す変更は、分類 A(外部 API の契約の実装、実 Windows 8+ の QFC は DWM の経路を使う)で忠実**。ただし挙動が変わる(g3 の行・a4/画素の基準が動く)ので、**E12 には入れない**。基準の更新を伴う別の変更として、実スレッド版の判断(gpu1 の 0297、mt3 の 0301)の後に、1 回の push にまとめて入れるか決める。
3. 「120 fps」の解釈: (A)「FL の描画が 120 fps で表示できる」= tick が 120 に追従し CPU が上がる(UI 専用コア前提)、(B)「画面更新の遅れが小さい」= 入力から次の tick までの待ちの最悪が 16.7 → 8.3 ms。画面に出る更新(present)は画素が変わった刻みだけで再生中約 12/s と測られている。**present の実数と 120 Hz の場合の測定は gpu1 の 0297**(120 Hz を渡す条件を含む)で取る。
4. 未確認: VCL 側のメッセージの出どころ、Learn の全メンバを埋めた DWM_TIMING_INFO の影響、実 Windows での QFC の DWM 経路、ブラウザでの present の実数、再生中の κ の直接測定(差から逆算)。
