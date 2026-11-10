# 0302(仮): DwmGetCompositionTimingInfo の行の差分化(Learn の全メンバ)、基準の更新の一覧、count/limit の見積り(mt3)

状態: **仮**(第 1 報。差分は保留 = push は PM の判断、実スレッド版の結果の後)。作業者 mt3、2026-11-06 JST。依頼: PM(0302)。基準: **E12c = 000091ae**(`git archive 000091ae` = E11 + 0284c + 0284b-on-0284c + 0295 の 2 つの差分)。出発点: alpha1 の 0299 の実験(`docs/investigations/alpha18/tables/0299-dwm-timing-info-EXPERIMENT-NOT-FOR-MERGE.diff`)。区分: **実測**=この作業の出力を見た値、**読み**=資料・コードを読んだ推論、**未確認**。

## 0. 冒頭の三点(正直さ)

1. **何をしたか**: 実装(runtime と import 行とテスト)と、E12c の木での `cargo test --no-run` / `test.sh full`(キャッシュなし)/ a4 の実行だけ。**実エンジン(wasm/ブラウザ/G3 の実行)の結果は 0 件**。QFC がこの関数をどう呼ぶかは alpha1 の逆アセンブル(`0299-qfc-f_1e857b0-disasm.txt`)を**読んで**確かめた(§2.1)。g3 の行の動きは alpha1 の実測(node/実エンジン、`rateRefresh` だけを埋める版)を引用する。**全メンバを埋めたこの版での g3 の行は、測っていない**(§5)。
2. **実 Windows の値は 0 件**。Learn に載るのは各メンバの一行の説明だけで、値の規則(特に GPU の無い機械での各カウンタ)は載っていない。決めた物は全て **S-dwm-ti-\*** の選択として明記した(§3)。実 Windows でこれらがどう見えるかは未測定。
3. **保留**: この差分は『別の変更として保留』(PM の指示)。E12 には入れない。入れる場合は基準の更新(§5)を伴う。

## 1. 結論

1. `DwmGetCompositionTimingInfo(NULL, &info)` を、**Learn の DWM_TIMING_INFO の全 40 メンバ**を仮想時計と表示 Hz から埋めて `S_OK` を返す形で実装した(`runtime/fle_dwm.c`、約 90 行)。hwnd ≠ NULL は E_INVALIDARG(Learn)、cbSize ≠ 0x124 は E_INVALIDARG(選択)、NULL の出力先は名前つきで停止(Learn に挙動が無い。同じファイルの `DwmIsCompositionEnabled` と同じ扱い)。
2. **構造体は 1 byte 詰めの 292 byte(0x124)**。Learn は順序だけを載せ詰めを載せないが、実際の呼び元(QuickFontCache)が渡す cbSize = 0x124 は『40 メンバの大きさの和を 1 byte 詰めで足した値』に一致し、自然な整列なら 0x140 になる(テストで再計算している)。
3. **仮想 DWM は DwmFlush が待つ表示と同一**: 表示は `fle_display_hz()` Hz の 1 本、vblank k は `ceil(k × 10^7 / hz)` tick(`fle_thread.c`)。QueryPerformanceCounter は tick 数そのもの(`FLE_QPF_HZ` = 10^7)。→ **DwmFlush が返った tick で呼べば、cRefresh = k、qpcVBlank = その tick**(境界ごとにテストで確認)。
4. **製品への配線**: 基本の `imports_table.rs` には触らず(`DwmFlush` だけが基本表。`DwmIsCompositionEnabled` 等は `--extra-imports` の行)、`dwmti-imports.txt`(1 行)を新設し、`ship6-imports.txt = ship5 + dwmti`(ship5 は変更なし・先頭、新しい行は最後 = 疑似 VA は動かない)を `EXTRA_IMPORTS` にした。`ci/test-hybrid.sh` に ship6 の検査(行数 = ship5 + 1、先頭 209 行は ship5 と同一、最後は dwmti、組の重複なし)を足した。
5. **基準の更新(§5)**: cargo の既存の期待値は動かない(基本表に行を足さないため。`test.sh full` 2,417 passed / 0 failed。ただし `wall_ia1` の DWM の単体 link の 2 本は、新しい関数の依存を満たすスタブ 2 つの追加が要った。§6)。動くのは製品の生成 C(表が +1 行)、wasm、CI のキー、**製品の行を渡した a4(5,312 / 1,009,745 → 5,310 / 1,009,743。標準の a4 は不変)**、そして **実エンジンの g3 の行と画素 sha**(alpha1 の実測: 55 行中 29〜33 行。理由の種類は §5 の表)。
6. **count/limit(§7)**: 実スレッド版で `fle_progress_count` を大域のまま共有すると、E11 の型付き間接呼びの hot path が 2 つの Worker から同じ cache line を書く。測った(§7)。推奨は『TLS(fle_ctx の欄)にする』。

## 2. 契約(出典)と実装

出典: Microsoft Learn の DwmGetCompositionTimingInfo、DWM_TIMING_INFO、UNSIGNED_RATIO、DwmFlush(写しは `docs/learn/`)。

### 2.1 関数

| 項目 | Learn の記述 | 実装 |
|---|---|---|
| `hwnd` | 『Starting with Windows 8.1, this parameter must be set to NULL. If this parameter is not set to NULL, DwmGetCompositionTimingInfo returns E_INVALIDARG』 | ≠ 0 → `0x80070057`(構造体は書かない) |
| `pTimingInfo` | 『A pointer to a DWM_TIMING_INFO structure that, when this function returns successfully, receives the current composition timing information』 | NULL → `fle_stop("DwmGetCompositionTimingInfo", "pTimingInfo is NULL: …")`(Learn に挙動が無い) |
| `cbSize` | 『The cbSize member of this structure must be set before this function is called』 | ≠ 0x124 → `0x80070057`、構造体は書かない(**S-dwm-ti-cbsize**: Learn は『設定が要る』とだけ言い、誤りの HRESULT を載せない。hwnd の規則と同じ E_INVALIDARG を選んだ。実 Windows の応答は未測定) |
| 戻り値 | 『If this function succeeds, it returns S_OK. Otherwise, it returns an HRESULT error code.』 | 成功 `0` |

**QFC の呼び方(読み、alpha1 の逆アセンブル)**: `f_1e857b0` は `[rbp+0xfc] = 0x124`(cbSize)を設定し、`0x1e48b40`(wrapper)を `(NULL, &info)` で呼び、戻りが 0 でなければ EnumDisplaySettingsW の分岐へ落ち、0 なら `[rbp+0x100]`(= offset 4、rateRefresh.uiNumerator)と `[rbp+0x104]`(offset 8、uiDenominator)だけを読んで `round(n × 1000 × den / num)` ms を作る。**他のメンバは読まない**。→ cbSize の厳密な検査は QFC の呼び出しを通す(0x124)。

### 2.2 構造体の配置(S-dwm-ti-layout)

Learn は順序だけを載せる。40 メンバの大きさ(UNSIGNED_RATIO = UINT32 × 2、QPC_TIME / DWM_FRAME_COUNT / ULONGLONG = 8、UINT = 4。『Both DWM_FRAME_COUNT and QPC_TIME are defined in Dwmapi.h as ULONGLONG』)を順に 1 byte 詰めで足すと **292 = 0x124**。自然な整列なら 320 = 0x140。QFC は 0x124 を渡す → SDK ヘッダは 1 byte 詰め(読み。`pshpack1.h`)。offset は 1 byte 詰めのもの(rateRefresh が 4、qpcRefreshPeriod が **12**)。`tests/dwm_timing.rs` が Learn の順序から再計算して固定している。

### 2.3 仮想 DWM の世界(S-dwm-ti-hz、S-dwm-ti-values)

- 表示は 1 本で、`fle_display_hz()`(DwmFlush が使う値。環境 `FLE_DWM_HZ` か 60。製品ではページが `FLE_DWM_HZ` と `FLE_DISPLAYSETTINGS_FREQUENCY` を同じ値で渡す)。**EnumDisplaySettingsW の dmDisplayFrequency(`fle_display_settings_hz()`)と同じ値**だが、両方を別の値にする既存の設定(`fle_thread.c` が stderr に警告を出す)では、この関数は **DwmFlush の側**(DWM の表示)に従い、dmDisplayFrequency には従わない(**S-dwm-ti-hz**。理由: 同じ呼び出しの中で period と vblank の間隔を矛盾させない。PM の依頼は『dmDisplayFrequency から』だったので、PM の判断で `fle_display_settings_hz()` に替えられる = 1 語。ただし分岐設定では period と DwmFlush の間隔が食い違う)。
- vblank k は `ceil(k × 10^7 / hz)` tick。`now` 以前の最後の vblank は `k = floor(now × hz / 10^7)`(`ceil(k × 10^7 / hz) ≤ now ⇔ k ≤ now × hz / 10^7`)。QPC = tick、QPF = 10^7。
- 時計を読む API として、呼び出しごとに仮想時計を `FLE_CLOCK_READ_TICKS` = 10 tick(1 µs)進める(決定 0017、`S-ds-pos-read-tick` と同じ。これで『この関数だけをポールするループ』が終わる)。
- 『合成は毎 refresh に 1 回、合成した frame は直ちに submit・confirm・display され、遅れも drop も無く、DirectX の present は無い』(**S-dwm-ti-values**。Learn は各メンバを一行で説明するだけで、GPU の無い仮想機械の値は載せていない)。『valid only after a second call』のメンバと『new』の数は前回の呼び出しからの増分(**S-dwm-ti-delta**、最初の呼び出しは 0)。

### 2.4 全メンバ(offset は 1 byte 詰め。k = cRefresh)

| # | member | offset | size | Learn(DWM_TIMING_INFO の記述) | 値 | 選択 |
|---|---|---|---|---|---|---|
| 1 | `cbSize` | 0 | 4 | The size of this DWM_TIMING_INFO structure. | 呼び出し側が設定した値(0x124 のみ受理) | S-dwm-ti-cbsize |
| 2 | `rateRefresh` | 4 | 8 | The monitor refresh rate. | {hz, 1}(UNSIGNED_RATIO: uiNumerator = hz、uiDenominator = 1) | S-dwm-ti-hz: hz = fle_display_hz() |
| 3 | `qpcRefreshPeriod` | 12 | 8 | The monitor refresh period. | round(QPF / hz) = (10^7 + hz/2) / hz ticks | 60 Hz: 166,667、120 Hz: 83,333 |
| 4 | `rateCompose` | 20 | 8 | The composition rate. | {hz, 1}(1 refresh に 1 回合成する) | S-dwm-ti-values |
| 5 | `qpcVBlank` | 28 | 8 | The query performance counter value before the vertical blank. | 直近の vblank k の tick = ceil(k × 10^7 / hz) | DwmFlush が待つ境界と同一(fle_thread_next_frame_tick) |
| 6 | `cRefresh` | 36 | 8 | The DWM refresh counter. | k = floor(now × hz / 10^7)(now 以前の最後の vblank の番号) | 仮想 boot が 0 |
| 7 | `cDXRefresh` | 44 | 4 | The DirectX refresh counter. | k の下位 32 bit | S-dwm-ti-values(DirectX の refresh = DWM の refresh) |
| 8 | `qpcCompose` | 48 | 8 | The query performance counter value for a frame composition. | qpcVBlank(合成の pass は vblank にある) | S-dwm-ti-values |
| 9 | `cFrame` | 56 | 8 | The frame number that was composed at qpcCompose. | k(1 refresh に 1 frame 合成) | S-dwm-ti-values |
| 10 | `cDXPresent` | 64 | 4 | The DirectX present number used to identify rendering frames. | 0(DirectX の present は無い) | S-dwm-ti-values |
| 11 | `cRefreshFrame` | 68 | 8 | The refresh count of the frame that was composed at qpcCompose. | k | S-dwm-ti-values |
| 12 | `cFrameSubmitted` | 76 | 8 | The DWM frame number that was last submitted. | k(合成した frame は直ちに submit) | S-dwm-ti-values |
| 13 | `cDXPresentSubmitted` | 84 | 4 | The DirectX present number that was last submitted. | 0 | S-dwm-ti-values |
| 14 | `cFrameConfirmed` | 88 | 8 | The DWM frame number that was last confirmed as presented. | k(仮想の GPU は即座に完了) | S-dwm-ti-values |
| 15 | `cDXPresentConfirmed` | 96 | 4 | The DirectX present number that was last confirmed as presented. | 0 | S-dwm-ti-values |
| 16 | `cRefreshConfirmed` | 100 | 8 | The target refresh count of the last frame confirmed as completed by the GPU. | k | S-dwm-ti-values |
| 17 | `cDXRefreshConfirmed` | 108 | 4 | The DirectX refresh count when the frame was confirmed as presented. | k の下位 32 bit | S-dwm-ti-values |
| 18 | `cFramesLate` | 112 | 8 | The number of frames the DWM presented late. | 0(仮想の DWM は遅れない) | S-dwm-ti-values |
| 19 | `cFramesOutstanding` | 120 | 4 | The number of composition frames that have been issued but have not been confirmed as completed. | 0 | S-dwm-ti-values |
| 20 | `cFrameDisplayed` | 124 | 8 | The last frame displayed. | k | S-dwm-ti-values |
| 21 | `qpcFrameDisplayed` | 132 | 8 | The QPC time of the composition pass when the frame was displayed. | qpcVBlank | S-dwm-ti-values |
| 22 | `cRefreshFrameDisplayed` | 140 | 8 | The vertical refresh count when the frame should have become visible. | k | S-dwm-ti-values |
| 23 | `cFrameComplete` | 148 | 8 | The ID of the last frame marked as completed. | k | S-dwm-ti-values |
| 24 | `qpcFrameComplete` | 156 | 8 | The QPC time when the last frame was marked as completed. | qpcVBlank | S-dwm-ti-values |
| 25 | `cFramePending` | 164 | 8 | The ID of the last frame marked as pending. | k | S-dwm-ti-values |
| 26 | `qpcFramePending` | 172 | 8 | The QPC time when the last frame was marked as pending. | qpcVBlank | S-dwm-ti-values |
| 27 | `cFramesDisplayed` | 180 | 8 | The number of unique frames displayed. This value is valid only after a second call. | 前回の呼び出しからの k の増分(最初の呼び出しは 0) | S-dwm-ti-delta |
| 28 | `cFramesComplete` | 188 | 8 | The number of new completed frames that have been received. | 同上(前回からの増分) | S-dwm-ti-delta |
| 29 | `cFramesPending` | 196 | 8 | The number of new frames submitted to DirectX but not yet completed. | 0 | S-dwm-ti-values |
| 30 | `cFramesAvailable` | 204 | 8 | The number of frames available but not displayed, used, or dropped. Valid only after a second call. | 0 | S-dwm-ti-delta |
| 31 | `cFramesDropped` | 212 | 8 | The number of rendered frames that were never displayed because composition occurred too late. Valid only after a second call. | 0 | S-dwm-ti-delta |
| 32 | `cFramesMissed` | 220 | 8 | The number of times an old frame was composed when a new frame should have been used but was not available. | 0 | S-dwm-ti-values |
| 33 | `cRefreshNextDisplayed` | 228 | 8 | The frame count at which the next frame is scheduled to be displayed. | k + 1 | S-dwm-ti-values |
| 34 | `cRefreshNextPresented` | 236 | 8 | The frame count at which the next DirectX present is scheduled to be displayed. | k + 1 | S-dwm-ti-values |
| 35 | `cRefreshesDisplayed` | 244 | 8 | The total number of refreshes that have been displayed for the application since DwmSetPresentParameters was last called. | k(DwmSetPresentParameters を呼ぶ者はいない = 仮想 boot から) | S-dwm-ti-values |
| 36 | `cRefreshesPresented` | 252 | 8 | The total number of refreshes that have been presented by the application since DwmSetPresentParameters was last called. | 0(DirectX で present する application が無い) | S-dwm-ti-values |
| 37 | `cRefreshStarted` | 260 | 8 | The refresh number when content for this window started to be displayed. | 0 | S-dwm-ti-values(hwnd = NULL、仮想 boot から表示) |
| 38 | `cPixelsReceived` | 268 | 8 | The total number of pixels DirectX redirected to the DWM. | 0 | S-dwm-ti-values |
| 39 | `cPixelsDrawn` | 276 | 8 | The number of pixels drawn. | 0 | S-dwm-ti-values |
| 40 | `cBuffersEmpty` | 284 | 8 | The number of empty buffers in the flip chain. | 0 | S-dwm-ti-values |

`tests/dwm_timing.rs`: 全メンバの値を 60 / 75 / 120 Hz と 9 つの時刻で、Learn と DwmFlush の時計の式から手で導いた値と比較(実装の出力から作った期待値ではない)。構造体の後ろの byte が書かれないこと。vblank の境界(60/75/120/144 Hz × 7 つの k)での cRefresh/qpcVBlank の一致と、1 tick 前の k − 1。2 回目の呼び出しの増分(24 frame)。hwnd・cbSize・NULL。**対照実験**: `qpcRefreshPeriod` の offset を 12 → 16 にすると 2 本が落ちる。

## 3. 選択の一覧(S-dwm-ti-\*、出典なしの物)

| 名前 | 選んだ物 | 理由 | 実 Windows |
|---|---|---|---|
| S-dwm-ti-layout | 1 byte 詰め(292 byte) | QFC の cbSize = 0x124 = 40 メンバの和(1 byte 詰め) | 構造体の大きさは QFC が示す。SDK ヘッダの `pshpack1.h` は読みの推論 |
| S-dwm-ti-cbsize | cbSize ≠ 0x124 は E_INVALIDARG、構造体は書かない | Learn: 『must be set』。HRESULT は hwnd の規則と同じ一般形 | 未測定 |
| S-dwm-ti-hz | 全て `fle_display_hz()`(DwmFlush の表示)から | 1 回の呼び出しの中の period と vblank、DwmFlush との一致 | 製品では dmDisplayFrequency と同値 |
| S-dwm-ti-values | 毎 refresh に合成、即 confirm/display、遅れ・drop・DirectX present は 0、『since DwmSetPresentParameters』は仮想 boot から | GPU も DirectX の application も無い仮想機械で、Learn の一行説明と矛盾しない最小の世界 | 未測定(実機の各カウンタの値は 0 件) |
| S-dwm-ti-delta | 『valid only after a second call』と『new』の数は前回の呼び出しからの増分(最初は 0) | Learn の文言 | 未測定 |
| (時計) | 呼び出しごとに 10 tick | 決定 0017 | — |

## 4. 製品への配線

- `lift/runtime/fle_dwm.{c,h}`: 関数と宣言(+約 90 行)。
- 行: `lift/harness/seeds/dwmti-imports.txt`(`dwmapi.dll DwmGetCompositionTimingInfo fle_imp_DwmGetCompositionTimingInfo`、1 行)。`ship6-imports.txt = ship5 + dwmti`。`app/scripts/build-engine-bundle.sh` の `EXTRA_IMPORTS` を ship6 に。疑似 VA は行の位置で決まる(決定 0015)ので、**最後に足す** = 既存の疑似 VA は動かない。
- `lift/crates/fe-lift/src/imports_table.rs`(基本表)は**触らない**: 基本表の dwmapi は `DwmFlush` だけで、`DwmIsCompositionEnabled`・`DwmExtendFrameIntoClientArea` も `--extra-imports` の行(ship*-imports.txt)。基本表に足すと、基本表の長さを固定する 0267 型のテスト(`rows_0267` を引く `wall_fr4/5/6/9/13/15/16/17/19/20`・`window`・`vcall_thunk_dll` の期待)が全部動く(0267 の先例: 14 ファイル)。extra-imports にすれば動かない。
- `ci/test-hybrid.sh`: ship6 = ship5(先頭・不変)+ dwmti(最後)、行の重複なし、の検査を足した。`ci/plan.sh` は `lift/harness/seeds` を丸ごとハッシュする(`H_SEED`)ので、seed・lift・bundle の段のキーが変わる(E12 は冷間)。
- テスト: `tests/dwm_timing.rs` の 6 本(構造体の大きさ、全メンバ、vblank の一致、増分、エラー、行の配線)。

## 5. 基準の更新の一覧(動く理由の『種類』つき)

期待値は実装の出力ではなく契約から手で導く、という方針に従い、**契約から導ける物**(関数そのもの、行の位置、表の長さ)と、**契約からは導けず実エンジンで取り直す物**(g3 の行、画素)を分ける。

| 基準 | 動くか | 理由の種類 |
|---|---|---|
| 基本表の行を固定する cargo テスト(`wall_fr4/5/6/9/10/11/12/13/15/16/17/19/20`、`window`、`vcall_thunk_dll`、`sweep1`、`deferred`、`srw`) | **動かない**(`test.sh full` で確認: 0 failed、§6) | 行は extra-imports で、基本表の長さと位置は不変 |
| **`wall_ia1` の DWM の単体 link のテスト**(`fle_dwm.c` だけを link する 2 本) | **動いた**(link 失敗)→ スタブ 2 つを足して直した | **link 依存の追加**: 新しい関数が仮想時計の 2 関数を参照する。契約の値ではなく、テストの足場(スタブ)の更新 |
| `test-hybrid.sh` の ship5 = ship4 + ref1 の検査 | 動かない(ship5 は不変) | ship6 の検査を**足す**(既存の検査は変えない) |
| 生成 C(`--extra-imports ship6` の製品の呼び出し)の `common.c` | 動く: 原型 +1、`fle_dispatch_table` +1 行、`fle_resp_table` +1 行、`fle_dispatch_count` / `fle_resp_count` +1(alpha1 の E11 の数: 101,584 → 101,585、819 → 820)。**全 module で同じ表**(merge_tables)。既定の呼び出し(`--extra-imports` なし)の sha は不変 | 契約から導ける(表の長さ + 1) |
| wasm / bundle の sha、C 部分の大きさ | 動く(C 部分に関数が増える。E12 は冷間ビルドなので E12 の sha は変わる) | 関数の追加 |
| CI のキー(`H_SEED`) | 動く(seeds 丸ごと) | 入力ファイルの追加 |
| a4(5 module、`--extra-imports` なし = 標準) | **動かない**: 942/942、895/895、5,310、1,009,737、終了コード 0(実測、§6) | a4 の既定は extra-imports を渡さない = この行は表に無い |
| a4(製品の行 `FLE_EXTRA_IMPORTS=ship5` → ship6) | **動く**: 5,312 / 1,009,745 → **5,310 / 1,009,743**(distinct −2、entries −2。実測、§6) | **QFC の分岐が変わる**(DWM の分岐を通る = EnumDisplaySettingsW の分岐の関数を通らない)。契約からは導けない(実行して取り直す) |
| 実エンジンの g3 の行(opt1 の node 84/55/87 行など。55 行) | **動く**(alpha1 の実測: 60 Hz で 33 行、120 Hz で 29 行) | 下の 3 種類。**契約からは導けない**(emergent): 取り直す |
| 〃 内訳 1: `g3-snap` の `clock_ms`(10/10)と `g3-progress` の進捗・slices・switches(10/10) | 全 snap が動く | **時刻のずれ・進捗の数**: QFC の分岐が変わり(DWM の分岐は正の値を返し、要求間隔を『フレーム時間の整数倍』に丸める。EnumDisplaySettingsW の分岐は負の値をそのまま使う)、UI の待ち(Sleep/DwmFlush)の刻みが変わる |
| 〃 内訳 2: `g3-surface` の画素 sha(23 のうち 12 / 9) | 動く | **時刻に依る画素**(メーター・アニメの位相)。窓の構成の違いではない(nonzero の数はほぼ同じ) |
| 〃 内訳 3: `g3-wintable` の sha(10 のうち 1 / 0) | ほぼ動かない | 窓の構成・位置・大きさは同じ |
| 〃 `g3-win`・`g3-layered`・`g3-timers` | 動かない | — |
| UI の仕事量(M/仮想 s) | −2.5〜−4%(alpha1 の実測。60 Hz でも 120 Hz でも) | 分岐の違い。P2 に小さな正 |
| P1(起動)・P3(音) | 未測定 | — |

**注**: alpha1 の実験は `rateRefresh` だけを埋める版で、時計を読まない。この版は(a)全メンバを埋め、(b)呼び出しごとに 10 tick 進める、という違いがある。QFC は `rateRefresh` だけを読む(§2.1)ので、**QFC の分岐は同じ**で g3 の行の動きも同じ種類と読むが、(b)の分は UI の待ちが 1 回あたり 1 µs 進むだけ(無視できる大きさ)で、**この版での g3 の行は取り直す必要がある**(未測定)。

## 6. 検証(E12c = 000091ae の木 + 差分)

| 検証 | 木 | 結果(実測) | ログ(`tables/`) |
|---|---|---|---|
| 差分の適用 | E12c(000091ae)/ main a977401d(E12) | `patch --dry-run` rc=0、offset も fuzz も無し(8 ファイル、736 行) | verify-patch-on-E12c-000091ae.log、verify-patch-on-main-a977401d.log |
| `cargo test --no-run` | E12c + 差分 | rc=0(215 test executables = 214 + `dwm_timing`) | verify-cargo-test-no-run.txt |
| `test.sh full`(`FLE_TEST_NOCACHE=1 --jobs 2`) | E12c + 差分 | **219 results、2,417 passed、0 failed**(E12 の基準 218 / 2,411 + 新しいテスト 6 本)、壁時計 471.9 s | verify-testfull.log |
| (同、最初の走り) | 〃(`wall_ia1` の link 用スタブを足す前) | 219 results、2,415 passed、**2 failed**(`wall_ia1` の DWM の 2 本: 新しい関数が `fle_sys_clock_ticks` / `fle_sys_clock_advance_to` を参照し、`fle_dwm.c` だけを link する単体テストが link に失敗)→ `DWM_PRE` にスタブ 2 つを足して解消 | testfull-run1-before-wall_ia1-fix.log |
| a4(build 段階まで、5 module、**標準 = `--extra-imports` なし**) | E12c + 差分 | units 942/942、runnable 895/895、distinct **5,310**、entries **1,009,737**、終了コード 0(**基準と同一**) | verify-a4-standard.log、a4run.sh |
| a4(`FLE_EXTRA_IMPORTS=ship5`、製品の行) | 〃 | distinct **5,312**、entries **1,009,745**、終了コード 0 | a4-with-extra-imports-ship5.log |
| a4(`FLE_EXTRA_IMPORTS=ship6` = ship5 + dwmti) | 〃 | distinct **5,310**、entries **1,009,743**、終了コード 0 | a4-with-extra-imports-ship6.log |
| `ci/test-hybrid.sh` | 〃 | all checks hold(ship6 の検査を 4 本足した) | verify-ci-test-hybrid.log |

**a4 の読み(実測 + 読み)**: 標準の a4 は extra-imports を渡さないので、この行は表に無く、**数が動かない**。製品の行(ship5)を渡すと 5,312 / 1,009,745 になり、**DWM の行を足す(ship6)と 5,310 / 1,009,743 に、関数が 2 つ・entries が 2 つ減る**(= a4 が QFC の DWM の分岐を実際に通り、EnumDisplaySettingsW の分岐の関数 2 つを通らなくなった、と読む。どの 2 関数かは調べていない)。つまり**製品の行を使う a4 の基準を持つ人(PM の標準の a4 は持たない)は 5,312 / 1,009,745 → 5,310 / 1,009,743 に更新が要る**。


## 7. count/limit(`fle_progress_count` / `fle_progress_limit`)を実スレッド版で厳密に扱うには(見積り 1 ページ)

**使われ方(読み)**: `fle_progress_count`(u64、`fle_dispatch.h` の weak 大域〔.bss〕)は間接呼び 1 回ごとに +1 される: C の経路は `fle_dispatch` の `++fle_progress_count >= fle_progress_limit`、直 wasm は E11(0285)の型付き間接呼びの各アダプタ(`lib.rs`: 『count を load → +1 → store → limit を load → `count >= limit` なら `fle_hyb_slice`』)と、demoted な呼びの `-1` の補正(`lower.rs` 5240 行)。`fle_progress_limit`(u64、初期値 UINT64_MAX の weak 大域〔.data〕)は `fle_progress_slice()`(`fle_thread.c`)が『count + slice_dispatches』に設定する。用途は 3 つ: (1) 協調(仮想時計)の疑似 preemption の slice の合図、(2) 0279 の課金(協調のときだけ意味がある)、(3) 診断の数(g3 の `progress`、native N1 の 36 仮想 s で 789,666,656 = 約 2,200 万/仮想 s)。実スレッド版(OS の preemption)では (1)(2) を使わない(gpu1 の 0289 の読み)ので、limit は UINT64_MAX のままで比較は常に偽、count は診断だけ。

**今のままの費用(実測、合成)**: 足場(rt-on-e9.diff / rt-on-e11.diff)は count を大域のまま使う。2 つの Worker が非 atomic の `load; +1; store` で同じ 8 byte を書く。合成の測定(`tables/count-bench/`、node、共有 memory の wasm、ループ = 『count の load/+1/store + 不透明な呼び出し 1 回』、各 2,000 万回、2 コアに固定、1 回の run): 

| 場合 | ns / 回 |
|---|---|
| 1 スレッドだけ | 3.75 |
| 2 スレッド、**同じ count**(足場) | **9.82 / 10.14** |
| 2 スレッド、同じ cache line の別の語(false sharing) | 11.24 / 11.19 |
| 2 スレッド、別の cache line(Worker ごとの count = TLS) | 3.73 / 3.93 |

= 両方が全力で書くと 1 回あたり +6 ns(2.6 倍)。**実際の率はこれより低い**: 総数 約 2,200 万/s のほぼ全てがミキサー側(N1 の仮想 CPU の slice の 95%)で、UI 側は 1 桁少ない読みなので、line の奪い合いは UI の書き込みの回数に比例する。上限(両方が同じ率)で +6 ns × 2,200 万 = 1 コアの約 13%、UI の率が 1/10 なら奪われるのは UI の書き込みの分だけ(cache line の取り直し 50〜70 ns の読み。**未測定**)で約 5〜7%。= **実スレッド版の『税 +20〜30%』(gpu1 0289)の一部になりうる大きさ**だが、実エンジンでは測っていない(ブラウザ 0)。`count` と `limit` は別の節(.bss / .data)なので同じ line とは限らない(未確認、link map で見る)。

**厳密にする案**:

| 案 | 内容 | 厳密さ | hot path への影響(命令・メモリ) | 変更量 |
|---|---|---|---|---|
| A | 今のまま(大域・非 atomic) | 総数は欠ける(lost update)、診断用なら許容 | 命令 0 増。**2 コアで line を奪い合う**(上の費用) | 0 |
| B(推奨) | **count と limit を fle_ctx の欄(TLS)にする**。アダプタが `reg_base` 経由で読み書きする | **Worker ごとに厳密**(書くのは持ち主だけ)。総数は各 block の和(診断の読み出しは持ち主の書き込みと競合するが 1 語ずつの torn なし) | 命令数は同じ(`i32.const 0` → tls base の `local.get`、同じバイト数)。アダプタは別 function なので tls base を取る `global.get __tls_base` が 1 回増える(L1 hit、1 ns 未満の読み)か、型付き呼びの共有ヘルパーの引数に tls base を足す(7 → 8 引数)。**line の奪い合いは無くなる**(上の表の最下行) | `fle_lowregs.h` に 2 欄(count、limit)、`fle_ctx_more.h` に 2 マクロ(C の `fle_dispatch` と `fle_thread.c` が使う名前のまま)、`lib.rs` のアダプタと `lower.rs` の demoted の補正を `reg_base` に(`hybrid_mx_tls` のとき。新しい旗なし)、g3 の `progress` は各 block の和(thread 登録簿を読む関数)、`hyb_guard cpart` の PRG を TLS では 0x0 に、テスト: `typed_indirect.rs` の TLS 版に『別 Worker の count が動かない』を足す。約 100 行 + テスト、0.5〜1 人日 |
| C | atomic add(`i64.atomic.rmw.add`) | 総数は厳密 | lock 付きの RMW。奪い合いの中で 1 回 100 ns 級(A より悪い)。値を使う `count >= limit` の判断も要る | 小さいが **却下**(hot path に最悪) |
| D | RT の構成では count をやめる(アダプタに count/limit の load/store を出さない) | 診断の数が無くなる(g3 の `progress`、`progress` に依る基準は RT では取り直し) | 最小(1 呼びあたり load 2・store 1 が減る) | アダプタの生成を条件付きに。『唯一の構成』が RT になるなら旗ではなく結果。協調経路を残す間は 2 経路のテストが要る |

**推奨**: 実スレッドを『唯一の構成』にするなら **B**(診断を保ち、奪い合いが無く、命令数が同じ)。D は B よりさらに速いが診断を失う。**判断が要る点**: (1) 協調の経路(`FLE_WORKERS=1` 相当)を残すか(残すなら B の limit は Worker ごとで、疑似 preemption の limit を設定する側は『自分の block』だけを触る)、(2) g3 の `progress` を RT でどう定義するか(各 block の和でよいか)。**E11 の hot path への影響**: B は命令数を変えない。測定は gpu1 の Chrome の A/B(count を TLS にした build と大域の build、同じ音楽位置)が要る(私は build しない)。

## 8. 方針との適合(必須の記載)

- **lift の忠実性**: 新しい処理は (b) 外部 API の契約の実装(DwmGetCompositionTimingInfo、実 Windows 8+ に存在)で、FL の挙動の補正・代行・class/アドレスの分岐ではない。QFC が実 Windows で通る分岐を通すための値を返すだけ(0299 §3.3 の分類 A)。
- **no-options**: 旗・環境変数・オプションの追加は無い(既存の `FLE_DWM_HZ` を使うだけ)。行は seeds のファイルで、製品の構成に常に入る(オプトインではない)。
- **進捗の誠実性**: 実エンジンの結果は無い(§0)。実 Windows の値は 0 件。数字は全て実測かその旨の引用。

## 9. 自己申告(0302 の区間)

- 最初の版は `fle_sys_clock_ticks` / `fle_sys_clock_advance_to` を新しく参照し、`wall_ia1.rs` の DWM の単体 link(`fle_dwm.c` だけを link する)を壊した。`test.sh full` を走らせて初めて分かった(先に `cargo test -p fe-lift --test wall_ia1` を走らせなかった)。`DWM_PRE` に 2 つのスタブを足して直し、全体をもう一度走らせた(最初の走り: 219 results / 2,415 passed / 2 failed、ログは `tables/testfull-run1-before-wall_ia1-fix.log`)。
- PM の依頼は『dmDisplayFrequency から』だったが、`fle_display_hz()`(DwmFlush の表示)から作った(S-dwm-ti-hz)。理由は書いたが、依頼からの逸脱なので PM の判断で替えられる。
- g3 の行の動きは alpha1 の実測の引用で、全メンバ版では測っていない(N1 の再構築は、gen3 に common.c が無く、E12c 用の再生成が要るため見送った)。
- 実 Windows の値は 0 件。Learn の写し(`docs/learn/`)は HTML から機械的に取ったテキストで、ナビゲーションの文字が混ざる。
- count/limit のマイクロベンチ: 最初の版(不透明な呼び出しなし)は V8 が load/store をループの外に出して 0.6 ns/回になり、意味が無かった(`tables/count-bench/bench-run1.txt`、捨てた run)。呼び出しを足した版(`bench2-run1.txt`)が本体。1 回の run、node の合成で、実エンジンの率ではない。ベンチは私の固定コア(8〜15)の外の 6・7 で 15 秒走らせた(gpu1 の Chrome の窓の終わり近く)。
- `cargo test --no-run` の出力ファイルを、`cd lift` 後の相対パスで木の中(`tree/`)に作ってしまい、走行中に外へ移した(結果は合成した)。
- 禁止された `sleep 1` をまた 1 回呼んで拒否された。



## PM の判断と取り込み(2026-11-06 18:54、docs のみ。製品の差分なし)

取り込み: mt3 の最終(`DwmGetCompositionTimingInfo` の 40 メンバ実装の文書、Learn の写し、表)を `docs/investigations/mt3w/0302/`、差分を `docs/investigations/mt3w/patches/0302-dwm-timing-info.diff` に置いた。実エンジンの wasm・ブラウザ・実 Windows の実行は 0 件(cargo テスト・a4 のみ)。**依頼者の機械は 0 件**。

事実として受け取る(mt3 の実測): 差分は E12c と main a977401d に `patch --dry-run` rc=0・offset/fuzz なし(8 ファイル 736 行)。構造体は 1 byte 詰めの 292 byte(0x124)で、QuickFontCache が渡す cbSize と一致。`test.sh full` 219 results / 2,417 passed / 0 failed(E12 の基準 218 / 2,411 + 新しいテスト 6)。標準の a4 は 942/942・895/895・5,310・1,009,737 で不変。`FLE_EXTRA_IMPORTS` を ship5 から ship6 に替えた a4 は 5,312 / 1,009,745 → 5,310 / 1,009,743。

判断:
1. **関数の契約(Learn の記述 + 選択 S-dwm-ti-cbsize / S-dwm-ti-hz)を採る**。値は `fle_display_hz()`(DwmFlush と同じ表示)から作る。製品では `FLE_DWM_HZ` と `dmDisplayFrequency` が同値なので、私の『dmDisplayFrequency から』との差は許容する(両方を別の値にする分岐設定の差は文書に残す)。分類 A(外部 API の契約の実装)で、FL の挙動の補正・代行・class/アドレス分岐は無い。
2. **この差分は E13 に入れる(まだ push しない)**。g3 の行(参照再生の 55 行中、alpha1 の rateRefresh だけの版で 60 Hz で 33 行・120 Hz で 29 行)・wasm/bundle の sha・CI の鍵が動くため、0284b-gp-extension(156 の逆順の対)と同じ冷間ビルドにまとめる。opt1 が FS 付きのテストビルド tt-e20 で node 行 84/55/87 の動く種類を取る(E13 の基準更新の準備)。
3. count/limit の案 B(`fle_ctx` の欄に移す。約 100 行、0.5〜1 人日)は、実スレッドの go/no-go #2 が『続ける』になるまで着手しない(決定 0305 の見積りも参照)。
4. 未確認: この全メンバ版での実エンジンの g3 の行、P1・P3 への効き、実 Windows の各メンバの値。
