実エンジン: `fe-build engine`・`fe-build bundle`・`tools/boot-check/boot-bundle.mjs`・`tools/topbar-metrics/measure.mjs`(基準と、診断の複製で 9 回)・右クリックと「View FPS」の操作の確認(画面の画素を目で読んだ)・rAF を 120 Hz に差し替えた確認を実行した。測定の値は runner の値。この機械とは別枠。nproc 4、loadavg は測定ごとに 0.5〜2.2。

# topbar-cpu-fps-1402: 上部バーの CPU Load と Frame Rate(段階 2)

判定: **差し戻し**(受け入れの判定 ③ と ④ は、まだ自動では読めない)。今回の試行で、表示の場所と、表示が何の周期に従うかは分かった。製品(`app/`・`lift/`・`ci/`)は変えていない(§3 の理由)。前回の「Frame 120 は読めない」は、前提が違っていた(§1)。

## 0. 段階 1 の確認(回帰)

- `git log`: `13565778 上部バーの Memory Usage が、実際の値を出す(段階 1): 完了 (Closes #1391)` が main に在る。
- 基準の測定(今回): K32 の回数 97、表示の候補の中に K32 の床(164 MiB)の近くの 165 MB が在る。段階 1 の式は、受け入れの実行で確かめる(§6)。

## 1. 表示の場所(FL の上部バー。画面で確かめた)

上部バーの右側の計器(CPU パネル)は、次の 3 つの欄から成る(`[CPU パネル]` の中の部品名は FL の RTTI の文字列。`CPUPanel`・`CPUGauge`・`MemGauge`・`PolyDigiLabel`・`fpsmeasurepaintbox`・`fpsicon` など):

| 欄(画面の位置) | 中身 | 値(今回の runner) | 根拠 |
|---|---|---|---|
| 左上の「0」(下に CPU の履歴グラフ) | **CPU Load**(% の整数。単位なし) | 0(idle・再生とも) | 右クリックの「Misc」の項目は、右下の欄だけを変える。CPU の履歴グラフが左下に在る |
| 右上「165 MB」 | **Memory Usage** | 165 MB(K32 の床と一致) | 段階 1 |
| 右下「0」(チップの絵の横) | 既定は **ボイス数**(View voice count) | 0 | 右クリックのメニュー |
| 右下「60 FPS」 | 右下の欄を **View FPS** にした時の **Frame Rate** | 60(DPR 1・idle) | 下の操作で切り替えた画面 |

- 右下の欄の切り替え: CPU パネルの欄で右クリック → メニューの「Misc」の「View FPS」を左クリック。メニューは右クリック点の右下に出る(右クリック点 +38, +119 px。guest の画素で)。切り替えの後、右下の欄は「60 FPS」(FPS の絵が付く)になる。
- 既定の「View voice count」では、Frame Rate の数字は画面に出ない。「Frame 120」は、FPS を選んだ時の右下の欄の数字。
- FPS の計測箱(`fpsmeasurepaintbox`)は、この欄の値の出どころと考えられる(未確認。FL のコードの中の呼び出しは未確認)。

## 2. 表示の値と周期(画面で確かめた)

| 条件 | ページの rAF(`rafPerSec`) | エンジンの DwmFlush(`FLE_DWM_HZ`) | 表示(右下 FPS) | 判定 ④(±15%) |
|---|---|---|---|---|
| 基準(runner の rAF 60) | 60.08(DPR 1) | 60 | **60** | 一致(目視) |
| rAF を 120 Hz に差し替え(診断の複製。ページの測定で 120 Hz) | 119.97 | 120 | **120** | 一致(目視) |

- 表示は **1 × DwmFlush の周期**。2 倍ではない(120 Hz の時に 240 にならない)。したがって、決定 0166 の「UI の周期が DwmFlush の 2 倍」の仮定は、Frame 表示の数字の出どころではない(表示は DwmFlush の 1 倍で、そのままの値)。
- 周期の設定は、起動時にページが測った rAF で決まる(`app/src/engine/hz.ts`・決定 0182/0268。60/75/90/120 に丸める)。runner は 60 Hz なので 60。実際の 120 Hz の画面では 120 になり、rAF と一致する。
- したがって、「120 の固定」は、上の 120 Hz の画面の時の、rAF と一致した値。runner の表示は 60 で、既に rAF と一致する(判定 ④ の式には合う)。
- 注意: 周期は起動時の測定で決まるので、起動の後に画面の周期が変わると、表示は追随しない(未検証)。

## 3. DwmFlush を rAF に結ぶ変更(PM の 2 番)は、入れない

- 理由: 表示は 1 × DwmFlush で、起動時に rAF に合わせて既に一致している(§2)。「120 の表示が 2 回の仮定から来る」ことは、今回の測定で否定された。
- 変えると、起動・音声・流れに影響し、受け入れで測り直しが要る(決定の「悪化したら段階に分ける」)。今回は判定に必要な値が読めていないので、変える根拠が無い。
- 注意(P3): rAF を 120 Hz に差し替えた複製では、ページの主スレッドにタイマーを回すので、音の途切れの計数(ヘッダーの「直近 60 秒の途切れ」)は 60 Hz の基準(6585)と同じ桁(5909)だった。この計数は、検査の流れの中で累積する値で、120 Hz だけで増えるものではない。P3 の比較には使わない。

## 4. 値(直す前。製品は変えていない)

| 値 | 表示 | 実測(engine の忙しい時間) | 判定 |
|---|---|---|---|
| CPU Load(左上) | 0(idle・再生とも。目視) | 無操作 0.31〜0.40%・再生 0.77〜0.85%(今回の runner。基準の実行) | 自動では未読。目視では、表示と実測の差が 10 以下 |
| Frame Rate(右下 FPS) | 60(目視)/ 120(120 Hz の時。目視) | rAF 60.08・59.94 / 119.97 | 自動では未読。目視では ±15% の内 |
| Memory Usage | 165 MB | K32 97 回 | 段階 1 の式(受け入れの実行で確かめる) |

## 5. 起動・音声・流れ(製品は変えていない)

製品を変えていないので、変更の前後の差は無い。以下は、この試行の runner の値。

- `boot-bundle.mjs`(受け入れの実行): engine-idle・standIn false・error null・elapsedMs 12,252(≤ 12,500)。直前のビルドの直後の 1 回は 13,273・12,368(負荷の差。上限に近い)。
- 音声 `p3-audio-probe --boot` ×3: error null・engine-idle・standIn false・ctxState running・quantaDelta 20,676 / 20,672 / 20,676(≥ 19,000)・`bootLast60sAt10s` / `70s` / `90s` が全て 0。参考(判定に入らない): `maxRefillStallMs` 709.9 / 672.6 / 670.2。判定は通る。
- 流れ A〜D(`welcome-flow.mjs`): 4 本とも engine-idle・standIn false・trapped false・hostErrors 0・opened true・welcomeClosed(B・D)true。elapsedMs 12,084 / 12,194 / 12,082 / 12,355(≤ 12,500)。preBootBytes 312,230,815(≤ 314,385,095)。判定は通る。
- `npm test --prefix app`: 73 files passed・1 skipped、1,432 tests passed・10 skipped。
- 止まりの道具(`tools/boot-hang/measure.mjs --dprs 1,2.5`。main に在る): 起動 2 回(DPR 1 の 2 回。DPR 2.5 は出力に出なかった。前回と同じで、道具の解釈は未確認)。hung 0・rate 0・firstIdleMs 12,225 / 12,629。

## 6. 受け入れの実行と、未確認

- 受け入れのコマンド(全文・そのまま)を実行した: `ACCEPT EXIT 1`。計測の判定の段で落ちた。段階 1(memory)の判定は通る(K32 80 回・床 169 MiB・`shownMb` 169・候補に 169・`zeroMbNew` 0・標本 40)。判定 ③ と ④ は、`cpu.idlePct`・`cpu.playingPct`・`fps.samples[].shown` が null のため、落ちる(`cpu.busyIdlePct` 0.341、`cpu.busyPct` 1.26、rAF 60.04・59.90、提示 11.36・11.11)。
- その後の音声・流れ・`npm test` は、受け入れの `&&` の連鎖の外で測った(§5)。
- 未確認(次の試行の課題): **表示の数字を自動で読むこと**。候補:
  1. 右クリック → 「View FPS」で、右下の欄を FPS にする(§1 の座標)。その後、右下の欄の数字の文字列(UTF-16 の Delphi 文字列)を、ゲストのメモリから読む。「ボイス数 0」→「FPS 60」の切り替えで変わった文字列を、候補として取る(切り替えの前後の差)。
  2. 画素の変化の対応(判定 (c))で、欄の矩形の画素のハッシュが、切り替えの前後で変わることを確かめる。数字は読まない。
  3. 文字列の走査は、`guestText` の上限(64 か所)のため、「0」や「60」のような数字では当たりが多すぎる。(1) の差で絞る必要がある。
- 「値を作らない」の基準から、候補を「rAF に近いもの」で選ぶことはしない。読んだ文字列が、切り替えで変わったことを根拠にする。
- PM の判断が要る点: 判定 ④ の定義(表示 ≈ rAF)は、今の製品で既に成り立つ(§2)。報告の「Frame 30fps すら満たさない」の本当の意味は、表示ではなく描画(提示 `presentsPerSec` が約 11/s)。④ の式で、この差を扱うかは PM が決める(#1351 の描画の費用の側)。

## 7. 道具の変更

- `tools/topbar-metrics/measure.mjs`: 窓ごとに `clockRate`(時計の速さ)と `desk`(render Worker の計数)を出す。`windows` を出す。計測のみで、判定には入れていない(前回の差分)。
- 今回の診断(右クリック・View FPS・rAF の差し替え・文字列の一覧)は、`tools/topbar-metrics/.tmp-diag.mjs` と `.tmp-run/` で行い、commit していない(消した)。

## 8. 段階 2 の作業: 実行中の Hz の追随(判定 ②)

- 変えたもの(製品): `lift/runtime/fle_thread.c` に、ページが測った Hz を入れる `fle_display_hz_put`(24..1000。既定は環境変数の値のまま)を足した。`fle_display_hz()` と `fle_display_settings_hz()` は、入れた値を返す。DwmFlush の呼び出しに数えを 1 つ足した(`fle_thread_dwm_flushes`)。`lift/harness/fruity_idle.c` に export を 2 つ足した(`fle_g3_hz`・`fle_g3_dwm_count`。既存の `fle_g3_display` と同じ作り)。新しい環境変数・`#ifdef`・URL パラメータは足していない。
- 変えたもの(page): `app/src/main.ts` の `trackHz` が、エンジンの起動の後、2 秒ごとに rAF の頻度を 500 ms 測る(起動の測定と同じ `measureFrameRate`)。丸めた値が変わったら `client.setHz` で Worker に渡す。Worker は `IdleDriver.setHz` で受け、次の idle で `fle_g3_hz` を呼ぶ(エンジンは止まっている時にだけ呼ばれる。既存の入力と同じ道)。
- 丸め(`app/src/engine/hz.ts`): 標準の一覧に 30 を足した(`[30, 60, 75, 90, 120]`)。60 未満を 60 に丸めていたため、約 30 の rAF でエンジンが 60 のまま追随しなかった。上限 120 は保った(決定 0268)。単体テストの決定値(30 → 60、24 → 60)を、新しい規則に直した。
- 道具(`tools/topbar-metrics/measure.mjs`): `hz.steps`(native → slow → back)。slow は道具の中だけで、rAF の各コールバックを 1 フレーム遅らせる(約 30 Hz。16 ms の遅れでは次の垂直同期を飛ばせないため、1 フレームにした)。各点は変えて 10 秒後に、直前 5 秒を読む。`dwmPerSec` は、idle の報告に付けた DwmFlush の数の差 ÷ 時間。
- 道具(`tools/boot-hang/measure.mjs`): 偶奇(viewStart)を、起動の通し番号ではなく、点ごとの番号で数えるように直した。前は、1 回ずつの点のうち 2 つ目が必ず DPR を設定されず(Chrome の既定 DPR 1)、`--dprs 1,2.5` の DPR 2.5 は一度も 2.5 で起動していなかった。直した後は、2.5 で engine-idle に達する(12,054 ms)。

### 測定(runner の値。この機械とは別枠。nproc 4)

- Hz の追随(2 回の run とも同じ結果):

| 点 | envHz(エンジンに渡した Hz) | rafPerSec(ページの rAF) | dwmPerSec(DwmFlush) |
|---|---|---|---|
| native | 60 | 60.105 / 60.076 | 59.706 / 59.876 |
| slow | 30 | 29.931 / 30.117 | 30.130 / 29.917 |
| back | 60 | 60.092 / 60.084 | 59.892 / 59.884 |

  - 追随の判定: 6 点とも ±15% 以内。試験の効き: slow ≤ 0.75 × native、back ≥ 0.85 × native。
  - DwmFlush の数は、ページの rAF と同じ頻度(1 フレームに 1 回)。決定 0166 の「1 回の DwmFlush で UI の周期が 2 回」は、この測定の数では出ていない。ただし FL の UI の周期の数そのものは測っていない(表示の数字を読んでいない)。注釈(`fle_thread.h` の 72 行付近)は、直していない。
- 時計(`clockRate`): 4 窓とも 1.000(無操作・再生・DPR 1・DPR 1.32〜1.82)。
- 忙しい時間(engine の Worker の busyMs ÷ 実時間): 無操作 0.091%・再生 0.452% と 0.513%(2 回の run)。整数の表示では、どちらも「0」になる値。
- 段階 1(Memory): K32 の回数は 121(1 回目)と 139(2 回目)。最後の K32 の床と `shownMb` は、1 回目 167、2 回目 174(2 回目の画面の上部バーに「174 MB」)。`zeroMbNew` は 0。
- 起動(`boot-bundle` ×3): elapsedMs 9,635・9,199・9,198(中央値 9,199 ≤ 12,500)。前回の run は 12,252(直後の負荷の回は 13,273・12,368)。同じ機械の前後の比較ではないので、差は読まない。
- 音声(`measure.mjs --boot` ×3): quantaDelta 20,672・20,672・20,675(≥ 19,000)。`bootLast60sAt10s`・`70s`・`90s` は全て 0。前回は 20,672〜20,676(同じ範囲)。
- 流れ A〜D: 4 本とも engine-idle・standIn false・trapped false・hostErrors 0・opened true。elapsedMs 8,252〜8,545(≤ 12,500)。preBootBytes 312,229,571(上限 314,385,095。前回は 311,120,481。エンジンの wasm が変わったため増えた)。
- 止まり(`boot-hang --dprs 1,2.5`、道具を直した後): 起動 2 回、hung 0。DPR 1 は 8,905 ms、DPR 2.5 は 12,054 ms で engine-idle。
- `npm test --prefix app`: 74 files passed・1 skipped、1,443 tests passed・10 skipped。`npm run typecheck`: 通る。`spec/guard.test.sh` 77 件・`spec/workflow.test.sh` 499 件: 通る。`guard/scan.sh`: 0 件。

### 目視(判定 ③。報告のみ)

- 撮った画面は `04-hz-slow`(道具の `--shots`)。上部バーの CPU の表示は「0」、Memory は「174 MB」。下段右(Frame の場所)は「0」で、これは既定の表示のボイス数。View FPS に切り替えていない(道具は FL の設定を変えない)。したがって、Frame の表示の値は、この run では読んでいない(未確認)。
- CPU の表示(整数の「0」)は、忙しい時間 0.09〜0.5% と整合する(表示の値は判定していない)。

### 未確認(この試行)

- FL の表示の値(Frame の FPS・CPU Load の数字)は、読んでいない。View FPS に切り替えないと、Frame はボイス数のまま。判定 ③ と ④(表示と rAF・忙しい時間の比較)は、表示を読む手段がないので、報告のみ。
- 負荷の高い場面(demo の再生)の CPU は、#1401・#1403 の後で扱う。
- 決定 0166 の「2 回」の仮定は、UI の周期の数で確かめていない(DwmFlush の数は rAF と 1:1)。
