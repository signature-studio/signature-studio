---
draft: "159"
title: "上部バーの CPU Load と Frame Rate が、実際の値を出す(段階 2)"
type: Task
milestone: "M3 トップバーのコンテキストメニュー(P2)"
parent: "009"
blocked_by: ["140"]
labels: ["topbar", "verify"]
size: L
priority: High
project: signature-studio/3
---

## 目的

依頼者の指示(原文): 「トップバーの Memory Usage と CPU Load はずっと 0MB のまま、Frame Rate は 120 で固定されていますが、どう見ても 30fps すら満たしていません。(CPU Load History グラフはそれっぽい値が出ています)実際の数値を反映するようにしてください。」

この Issue は**段階 2(CPU Load と Frame Rate)**。段階 1(Memory Usage。#{{140}})が閉じた後に起動する(blocked by)。段階 1 の判定(Memory)は、この Issue の判定にも入る(回帰の確認)。

この Issue の目標は次の 2 つ。
1. CPU Load(FL の表示の値)が、エンジンの Worker が実際に計算していた割合(忙しい時間。%)と、無操作・再生の両方で ±10 ポイント以内で一致する。
2. Frame Rate(FL の表示の値)が、ページの描画の周期(rAF の頻度)と ±15% で一致する。120 の固定ではない。

PM の決定(定義): 数値は、実際の値を出す。値を良く見せるために、数字を作らない。低い値が本当なら、そのまま出す(低い理由は #1351 の描画の費用で扱う)。

## 前回の試行で分かったこと(run 45579415277)

- 判定は差し戻し(失敗の段: 計測の判定。`ACCEPT EXIT 1`)。`cpu.idlePct`・`cpu.playingPct`・`fps.samples[].shown` は null のまま。製品(`app/`・`lift/`・`ci/`)は変えていない。
- 段階 1 の判定は通る(K32 の回数 80、標本 40、`zeroMbNew` 0)。
- 表示の値(作業者が画面で見た。runner の画面は 60 Hz): Frame Rate は DPR 1 で「60」(rAF 60.04・59.90 と一致)。CPU Load は「0」(idle・再生とも)。
- 忙しい時間は 0.3〜1.3% で、整数の表示では 0 になる。本当の値なので、そのまま出す(低い理由は #1351 の側)。
- 表示は既に実際の値に従っている。前回の「120」は、120 Hz の画面で rAF と一致した値だった。診断の複製で rAF を 120 Hz にすると、表示は 120 になり、240 にはならなかった(DwmFlush の 1 倍)。
- FPS の表示は、CPU パネルの右クリック →「View FPS」で出る(既定はボイス数)。FL が呼んだ API と表示の対応は、未確認。
- 数字を自動で読む判定は、やめる(4 回目の PM の決定)。OCR でない方法も、制御した変化でも、1 回の試行で実装できなかった。
- 未確認(判定 ② で確かめる): 起動の後に rAF の頻度が変わった時、エンジンの周期が追随するか。`fle_thread.c` は FLE_DWM_HZ を起動時に 1 回だけ読み(read once)、`hz.ts` は起動時に 1 回だけ rAF を測る。
- 丸め: `roundToStandardHz`(hz.ts)は 60 未満を 60 に丸める。約 30 の rAF では、エンジンの Hz は 60 のまま(推定)。
- ブランチ #1356 の実機の観測(`pm-work/findings/perf-windows-browser/README.md` の 11 番): rAF を測れない起動時に、エンジンの Hz が 60 のまま固定され、表示が高い Hz になっても追随しなかった(headless の runner では再現しない)。
- 時計の速さ(`clockRate`): 前々回の記録は全窓 1.000。前回の報告には値が無い。
- 起動(`boot-bundle`): elapsedMs 12,252(直後の負荷の回は 13,273・12,368)。上限 12,500 に近いので、3 回の中央値で判定する。
- 音声 ×3(`measure.mjs --boot`): quantaDelta 20,672〜20,676(≥ 19,000)。`bootLast60sAt10s`・`70s`・`90s` は全て 0。
- 流れ A〜D: 4 本とも engine-idle・standIn false・hostErrors 0・opened true。elapsedMs 12,082〜12,355(≤ 12,500)。
- `npm test --prefix app`: 73 files passed・1 skipped、1,432 tests passed・10 skipped。
- 止まり(`boot-hang --dprs 1,2.5`): 起動 2 回、hung 0。DPR 2.5 の起動は出力に出ない(前回と同じ。道具の解釈は未確認)。
- 前回の差分(`work.diff`: `measure.mjs` の `clockRate`・`desk`・`windows`、`pm-work/findings/topbar-cpu-fps-1402.md`)は、`.prior-work.md` で取り込まれる。

## その 1 つ前の試行で分かったこと(run 45551573330)

- 判定は差し戻し(失敗の段: 判定。③ と ④ が判定できない)。FL の表示の CPU「0」と Frame「120」を、ゲストのメモリから読めなかった。`cpu.idlePct`・`cpu.playingPct`・`fps.samples[].shown` は null のまま。製品(`app/`・`lift/`・`ci/`)は変えていない。runner の値。nproc 4、loadavg 0.77〜1.65。
- 作業の差分は `work.diff` に保存された(`tools/topbar-metrics/measure.mjs` の `clockRate`・`desk`・`windows` の追加と、`pm-work/findings/topbar-cpu-fps-1402.md`)。次の試行は、`.prior-work.md` で取り込まれた差分を確かめてから続ける。戻り番地を記録する一時の診断は commit していない。
- 段階 1 の式は通る(回帰なし): K32 の回数 98、最後の値 172,479,892 B(床 164 MiB)、`shownMb` 164、候補に 164 が在る、標本 40 個。
- 時計の速さ: 窓ごとの `clockRate` は、無操作・再生・DPR の全窓で 1.000。遅れの上限(1 秒)に届いた記録は 0 件。時計は、idle の pacing で既に実時間の速さで進む。
- 戻り番地(一時の診断。ビルドの関数名の表で引いた): DwmFlush は `f_1e85b70+0x33`(本体の外の像)。仮想の 60 Hz で、1024 → 2048 回が 17.07 s(60.0 回/s)。
- QPC の候補(未確認): `f_1253800+0x30`・`f_1253840+0x22`(本体の中。QPC の差から比を取る関数。512 回に達したのは仮想 92 s の時点で、約 5.5 回/s)。CPU 表示の候補。`f_442c30+0x14`(256 回)は Frame 表示の候補。どちらも表示の数字との対応は取れていない。
- GetTickCount と Sleep(0) は本体の外(`f_1e82290+0x40`・`f_1e85fc0+0x36`。待ちの空回し)。timeGetTime は `f_71eaa0+0xd`(本体の中。32 回)。
- 窓の一覧: 本体の窓 1 つ(タイトル「FL Studio 2026」)だけ。表示の数字は窓の文字ではないので、窓の一覧からは読めない。
- 文字列の走査: 「0」や「120」だけでは当たりが多すぎ、特定できない。
- engine の実測(無操作/再生): rAF 59.93/s・59.95/s。合成の依頼(`ticks`)0/s・60.07/s。提示(画面が変わった回数)0/s・11.49/s。合成の時間 0.13 ms/s・9.59 ms/s。
- 再生の合成の依頼 60/s のうち約 80% は画面の変化なし。合成 1 回は約 0.16 ms で、低い値の原因は合成の時間ではない。変化なしの理由(FL が描かなかったか、合成の世代が変わらなかったか)は未確認。
- 忙しい時間(Worker の busyMs ÷ 実時間): 無操作 0.65%(実行によって 0.37〜0.71%)・再生 1.27%(0.69〜1.27%)。差は 1 ポイント未満。表示は整数なので、この桁では差が出ない。
- 判定の run の再生(DPR 1 と乱数の DPR 2.61): rAF 60.04/s・59.97/s、提示 11.36/s・11.12/s、忙しい時間 0.48%・0.80%。
- 起動: `boot-bundle` で engine-idle・standIn false・error null・elapsedMs 11,197(判定の run は 11,308。≤ 12,500)。
- 音声 ×3(`measure.mjs --boot`): quantaDelta 20,676・20,675・20,672(≥ 19,000)。`bootLast60sAt70s`・`bootLast60sAt90s`・`bootLast60sAt10s` は全て 0。参考(判定に入らない): `maxRefillStallMs` 821.2・800.0・810.3。
- 流れ A〜D: 4 本とも engine-idle・standIn false・trapped false・hostErrors 0・opened true。elapsedMs 10,493〜10,653(≤ 12,500)。preBootBytes 311,120,481(≤ 314,385,095)。
- 止まりの道具(`tools/boot-hang/measure.mjs`): 3 起動、hung 0・rate 0。firstIdleMs 10,378〜10,609。`--dprs 1,2.5` の DPR 2.5 の起動は出力に出なかった(道具の解釈は未確認)。
- `npm test --prefix app`: 1,406 件が通る(71 files passed・1 skipped)。
- 前回の報告が PM に要求した点(a)(b)(c)は、下の「決定(PM)」で答えた: (a) DwmFlush を rAF に 1 対 1 で結ぶ。(b) CPU の定義は engine の忙しい時間。(c) Frame の真の値は rAF の頻度。

## その前の試行で分かったこと(run 45469639595)

- その前の試行は、段階 1(#{{140}})の中で行った(run 45469639595。判定は `failed`)。段階 2 の作業は、まだしていない。以下は、その測定の結果。
- CPU(Worker の実測。runner の値): 無操作 0.565%・再生 0.456%。Worker の計測は、idle の時刻が絶対時刻なのに、窓の境界が `performance.now()` だったため、窓の中の記録が 0 件だった。両方を絶対時刻に揃え、取れるようになった。
- Frame(エンジンの提示): 無操作 0.13/s・再生 11.4/s(DPR 1 で 11.6/s、DPR 2.7 で 11.1/s)。提示の数は、engine の DeskCounters の増分 ÷ 実時間。
- 表示の値の出どころ(未確定): 下段右の CPU「0」(単位なし)と、Frame「120」は、ゲストのメモリの走査で読めていない(周りに数字の文字列が在るだけ)。
- 呼び出しの記録(FL 本体): 時間の読みは `GetTickCount`・`QueryPerformanceCounter`・`timeGetTime`。QPC の 2 つの差を積んで比を取る関数(FL 側の計時。`f_1253840`)が在り、CPU の値の候補。表示との接続は未確認。
- `DwmFlush` を呼ぶのは本体の外のモジュール(QuickFontCache の TTimerThread)。仮想の DWM は 60 Hz で、描画は 2 倍(120 回/秒。決定 0166)。表示の「120」がこれかは未確認。
- 仮想の時計: 時計の読み 1 回で 10 ticks 進み、`DwmFlush` は仮想の枠の境界まで進める(決定 0122・0166)。実時間には連動しない。
- 道具の状態: `tools/topbar-metrics/measure.mjs` の `cpu.idlePct`・`cpu.playingPct`・`fps.samples` は、前回の版では null のまま。この段階で埋める。

## 背景

- 事実 1(時計が仮想): `lift/runtime/fle_sys.h` の 9〜20 行: 時計は 100 ns の目盛りの仮想の時計。時間が進むのは、Sleep、時計の読み、DwmFlush の 3 つだけ。実時間には、そのままでは連動しない。ただし前回の実測では、idle の pacing(`app/src/engine/idle-driver.ts`)によって、窓ごとの速さ(`clockRate`)は 1.000 だった。
- 事実 2(画面の周期が仮想): `lift/runtime/fle_thread.h` の FLE_DWM_HZ の既定は 60(環境変数で変えられる)。`lift/runtime/fle_thread.c` の `fle_imp_DwmFlush` は、仮想の時計を次の枠の境界まで進める(スレッドが 1 つの時)。決定 0166 の注釈(`fle_thread.h` の 72 行付近): 「the guest's UI cycle follows DwmFlush, measured 2.00x the cycles and the draws at 120」。
- 事実 3(仮想の時計と実時間の差): `app/src/engine/worker.ts` の `lagCut`(S-clk-maxlag)は、仮想の時計が実時間より遅れすぎた時、その遅れを残す(仮想の時間は飛ばさない)と記録する。
- 事実 4(忙しい時間): `app/src/engine/worker.ts` の driver の `idle(n, now, next, busyMs, at)` は、1 回の idle の busyMs を受け取る。上部バーには届いていない(推定)。
- 事実 5(提示の数): engine の提示の回数(presents)は、`app/src/perf/beacon.ts` の DeskCounters と、終了の記録(`app/src/main.ts` の exit)に在る。
- 事実 6(ページの表示): `app/src/perf/display.ts` の表示の計測は、既定では rAF を連続で回さない(burst。決定 0202)。
- 事実 7(文字の読み): 上部バーの値の文字は、ゲストのメモリの中の文字列として読める(`app/src/engine/worker.ts` の `guestText`。#1358 と #1376 の方式。OCR は使わない)。

## 決定(PM)

- 4 回目の決定(PM): 数字そのものを自動で読む判定は、やめる。表示の数字は FL が作るので、FL に渡す入力の連鎖(画面の Hz → DwmFlush → 表示)を判定する。
- 判定 ①(時計): 4 つの窓(無操作・再生・DPR 2 点)の `clockRate` が、すべて 0.95〜1.05。
- 判定 ②(Hz の追随): FL に渡す画面の Hz(`envHz`)と、DwmFlush の頻度(`dwmPerSec`)が、ページの rAF の頻度(`rafPerSec`)に ±15% で追随する。試験は、native → 約 30 Hz → native の順に rAF を変え、各点を変えてから 10 秒後に読む。
- 判定 ③(表示の目視): 作業者が、読み取り直後の画面(PNG)を見て、Frame Rate の表示と rAF が ±15% で一致することを、報告に書く。判定には入れない。
- 判定 ④(CPU): `cpu.busyIdlePct`・`cpu.busyPct` が数値であること。表示の数字は判定しない(表示は整数で、低い値は本当の値)。
- 判定 ⑤(起動): 3 回の elapsedMs の中央値が 12,500 以下。音声・流れ・テストは元のまま。
- 追随しない時の直し(PM): ページが測った rAF の頻度を 2 秒ごとに更新し、エンジンに渡す。既存の FLE_DWM_HZ の経路を使う。新しい環境変数・`#ifdef` は作らない。起動時に測れない時は、ブラウザの目標(120)を保つ。
- 約 30 Hz の試験で追随させるには、`roundToStandardHz` の 60 未満の丸めを直す(上限 120 の決定 0268 は保つ)。
- 道具は、FL の設定(View FPS など)を切り替えない。
- 以下の旧い決定のうち、画面の周期・表示の読み・判定 ③④ は、この 4 回目の決定で置き換える(前回の作業で、表示は既に rAF に一致していた)。
- 依頼者の指示(上部バーの数字は、実際の値を出す)が、決定 0122・0166(仮想の時計・仮想の DWM)に優先する。
- 時計: 前回の実測で、時計は既に実時間の速さ(`clockRate` 1.000)で、遅れの上限に届いた記録も無い。FL が見る時計(`QueryPerformanceCounter`・`GetTickCount`・`timeGetTime` など)の結び方を変えるのは、表示が読めた後に、表示の変化で必要と分かった時だけ。音声・起動・流れ・止まりを悪くする変更は入れない。
- (4 回目で置き換え)画面の周期(`DwmFlush`): 仮想の 60 Hz の枠の境界を使わず、ページの実際の描画の周期(`requestAnimationFrame`)に 1 対 1 で結ぶ。Worker が、ページの rAF の通知を待つ。決定 0166 の「1 回の DwmFlush で UI の周期が 2 回」の記述(`lift/runtime/fle_thread.h` の 72 行付近の注釈)を、読んで確かめる。「120」の表示がこの 2 回の仮定から来るなら、その仮定を外し、DwmFlush 1 回 = 1 周期にする。
- CPU の定義: engine の Worker の忙しい時間 ÷ 実時間(`cpu.busyIdlePct`・`cpu.busyPct`)。合成とページの描画は含まない(FL の CPU Load は、音声の処理の負荷を表す物)。
- (4 回目で置き換え)判定 ③(CPU): FL の表示値(`cpu.idlePct`・`cpu.playingPct`)が、同じ区間の engine の忙しい時間と、無操作・再生の両方で ±10 ポイント以内(`|表示 − 忙しい時間| ≤ 10`)。前回の「無操作と再生で 1 ポイント以上違う」は外す。表示は整数で、低い値は実際の値が小さいから(0.65% が「0」・1.27% が「1」)。
- (4 回目で置き換え)判定 ④(Frame): 表示値(`fps.samples[].shown`)が、同じ区間のページの rAF の頻度(`fps.samples[].rafPerSec`)と ±15% 以内。エンジンの提示(`presentsPerSec`)は判定に使わない(FL の UI の周期は、画面が変わらなくても DwmFlush に従って回る)。PM の読み: 表示が 120 になるのは、ページの rAF が 120 の時だけ。
- (4 回目で置き換え)表示の読み(判定の前提): 「0」「120」のような小さい数の文字列は、メモリに多数在るので、文字列の検索だけでは特定できない。入力を変えて、表示が追随する場所を見つける。(a) Frame: ページの rAF の周期を変える(`tools/e2e/lib/browser.mjs` の `launchBrowser(clock)`: 既定の `raf` と、120 Hz の BeginFrame の `begin`。または、ページの主スレッドを止めて rAF を 30 Hz 付近に落とす)。表示の数字(UTF-16。Delphi の `UnicodeString`: 参照数と長さの 8 バイトの header つき)が変わる場所を、周期ごとの集合の差で特定する。(b) CPU: 音声の負荷を変える(再生の有無・Mixer のインサートなど)。同様に特定する。(c) 文字列で特定できないときは、#{{140}} の判定 (ii) と同じ、画素の変化の対応を判定にする(上部バーの小さい矩形の画素列のハッシュが、制御した変化の前後で変わるか。画素の文字は読まない。OCR は使わない)。
- 負荷の高い場面(demo の曲の再生)での検証は、この Issue では扱わない(#{{158}} と #{{160}} が素材を揃えてから、別の Issue で行う)。

## 読む物

- `lift/runtime/fle_sys.h`・`lift/runtime/fle_sys.c`・`lift/runtime/fle_thread.h`・`lift/runtime/fle_thread.c`(時計・DwmFlush・`fle_display_settings_hz`)
- `lift/runtime/fle_monitor.c`(EnumDisplaySettingsW)
- `app/src/engine/hz.ts`(`measureFrameRate`・`applyHz`・`roundToStandardHz`)・`app/src/perf/display.ts`(`hzFit`)・`lift/runtime/fle_thread.c`(FLE_DWM_HZ を起動時に 1 回読む。4 回目の判定 ② の読む物)
- `app/src/engine/worker.ts`(`lagCut`・driver の `idle`・`guestText`)・`app/src/engine/client.ts`・`app/src/engine/protocol.ts`・`app/src/main.ts`
- `app/src/perf/beacon.ts`・`app/src/perf/display.ts`
- `tools/topbar-metrics/measure.mjs`(段階 1 の道具)・`tools/p2-frames/measure.mjs`・`tools/p3-audio-probe/measure.mjs`(`--boot`)
- `tools/e2e/lib/browser.mjs`(`launchBrowser(clock)`・120 Hz の BeginFrame)・`lift/runtime/fle_monitor.h`(決定 0166 の注釈)・`lift/runtime/fle_thread.h`(72 行付近の注釈)
- `pm-work/findings/topbar-metrics-1391.md`(前回の記録)・`pm-work/findings/p2-frames-1350.md`
- 前回の差分(`work.diff` の中): `pm-work/findings/topbar-cpu-fps-1402.md`(前回の記録。`.prior-work.md` の取り込みで入る)

## やること

0. 作業ツリーの確認。`.prior-work.md` があれば最初に読む。前回の差分(`tools/topbar-metrics/measure.mjs` の `clockRate`・`desk`・`windows`、`pm-work/findings/topbar-cpu-fps-1402.md`)が取り込まれていることを確かめる。段階 1 が main に在ることを、`git log` で確かめる。
1. **Hz の追随(判定 ②。受け入れの要)**: `app/src/engine/hz.ts`・`app/src/perf/display.ts`(`hzFit`)・`app/src/main.ts`(`engineEnvHz`)・`lift/runtime/fle_thread.c`(FLE_DWM_HZ を起動時に 1 回読む)を読み、起動の後に rAF の頻度が変わった時に、エンジンの周期が 10 秒以内に追随する経路を確かめる。追随しなければ、上の「4 回目の決定」の直し(2 秒ごとに更新して渡す)を入れる。
2. **丸め**: `roundToStandardHz` の 60 未満の丸めを直す(約 30 Hz の試験で、30 付近の値をエンジンに渡す)。上限 120 は保つ。直した理由を報告に書く。
3. **測定の道具**: `tools/topbar-metrics/measure.mjs` に、判定 ② の測定(`hz.steps`)を足す。native → 約 30 Hz → native の順。約 30 Hz は、道具の中だけで、ページの主スレッドの rAF の合間に 16 ms の遅れを入れて作る(製品には入れない)。各点は変えてから 10 秒後に読み、直前 5 秒の rAF の頻度を `rafPerSec` とする。`envHz` は、エンジンの環境に渡している Hz(`engineEnvHz`。FLE_DWM_HZ と FLE_DISPLAYSETTINGS_FREQUENCY の値。EnumDisplaySettings の `dmDisplayFrequency` も同じ値であること)。`dwmPerSec` は、FLE の DwmFlush の回数(呼び出しに数えを 1 つ足す)。
4. **時計と忙しい時間**: 前回の差分の `clockRate` を、`phases.idle`・`phases.playing`・`fps.samples[]` の 4 窓に出す(判定 ①)。`cpu.busyIdlePct`・`cpu.busyPct` は既存(判定 ④)。
5. **音声・起動を守る**: 変更の前後で boot-bundle ×3(elapsedMs の中央値)・`measure.mjs --boot` ×3・流れ A〜D を測り、悪化しないことを確かめる。悪化したら、結び方を段階に分ける。
6. **止まりの道具**(`tools/boot-hang/measure.mjs`。#1393 の道具): main に在れば、同じ判定(止まり 0)で確かめる。無ければ、報告に「確認できない」と書く。
7. **目視(判定 ③。報告のみ)**: 作業者が、読み取り直後の画面(PNG)を見て、Frame Rate の表示と rAF の一致を、報告に書く。FL の設定を変えて表示を見る場合は、作業者の一時の診断に限り、判定の道具には入れない。
8. **受け入れ**: 下の「受け入れ」を、そのまま通す。

## 受け入れ

```accept
npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && for i in 1 2 3; do node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/bootbundle-$i.json" || exit 1; done && node -e 'const fs=require("fs");const L=[1,2,3].map((i)=>JSON.parse(fs.readFileSync(process.argv[1]+"/bootbundle-"+i+".json","utf8")));const e=L.map((r)=>r.elapsedMs).sort((x,y)=>x-y);process.exit(L.every((r)=>r.error==null&&r.state==="engine-idle"&&r.standIn===false)&&e[1]<=12500?0:1)' "$RUNNER_TEMP" && node tools/topbar-metrics/measure.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/metrics.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));const m=r.memory,c=r.cpu,s=r.fps.samples,p=r.phases,z=r.hz.steps;process.exit(r.error===null&&((()=>{const h=m.hash,hs=h.series,hf=h.floors,fl=Math.floor(m.k32Bytes[m.k32Bytes.length-1]/1048576),near=[...new Set(m.candidates.filter((q)=>Math.abs(q-fl)<=1))],ex=near.includes(fl),fc=[],hc=[];for(let i=1;i<hs.length;i++){if(hf[i]!==hf[i-1])fc.push(i);if(hs[i]!==hs[i-1])hc.push(i);}return m.k32Bytes.length>=1&&m.k32Bytes.every((v)=>Number.isInteger(v))&&near.length>=1&&(ex?m.shownMb===fl:(near.length===1&&m.shownMb===near[0]))&&m.zeroMbNew===0&&hs.length>=40&&hs.length===hf.length&&(h.method==="band"||(h.method==="rect"&&fc.length>=1&&hc.length>=fc.length&&hc.every((x)=>fc.some((j)=>j>=x-2&&j<=x))));})())&&Number.isFinite(c.busyIdlePct)&&Number.isFinite(c.busyPct)&&s.length===2&&[p.idle.clockRate,p.playing.clockRate,s[0].clockRate,s[1].clockRate].every((x)=>Number.isFinite(x)&&x>=0.95&&x<=1.05)&&z.length===3&&["native","slow","back"].every((n,i)=>z[i].name===n)&&z.every((x)=>[x.rafPerSec,x.envHz,x.dwmPerSec].every((v)=>Number.isFinite(v))&&x.rafPerSec>0&&Math.abs(x.envHz-x.rafPerSec)<=0.15*x.rafPerSec&&Math.abs(x.dwmPerSec-x.rafPerSec)<=0.15*x.rafPerSec)&&z[1].rafPerSec<=0.75*z[0].rafPerSec&&z[2].rafPerSec>=0.85*z[0].rafPerSec?0:1)' "$RUNNER_TEMP/metrics.json" && for i in 1 2 3; do node tools/p3-audio-probe/measure.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" --boot > "$RUNNER_TEMP/boot-$i.json" || exit 1; done && node -e 'const fs=require("fs");const L=[1,2,3].map((i)=>JSON.parse(fs.readFileSync(process.argv[1]+"/boot-"+i+".json","utf8").trim()));const a=L.map((r)=>r.bootLast60sAt10s).filter((v)=>typeof v==="number").sort((x,y)=>x-y);process.exit(L.length===3&&a.length===3&&L.every((r)=>r.error===null&&r.state==="engine-idle"&&r.standIn===false&&r.ctxState==="running"&&r.quantaDelta>=19000&&r.bootLast60sAt70s===0&&r.bootLast60sAt90s===0)&&a[1]<=30&&a[2]<=60?0:1)' "$RUNNER_TEMP" && node tools/smoke/welcome-flow.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/flow.jsonl" && node -e 'const fs=require("fs");const L=fs.readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const F=(f)=>L.find((r)=>r.flow===f);const ok=(r)=>!!r&&r.error===null&&r.state==="engine-idle"&&r.standIn===false&&r.trapped===false&&r.hostErrors===0&&r.opened===true;process.exit(L.length===4&&["A","B","C","D"].every((f)=>ok(F(f)))&&F("B").welcomeClosed===true&&F("D").welcomeClosed===true&&Math.max(...L.map((r)=>r.elapsedMs))<=12500&&Math.max(...L.map((r)=>r.preBootBytes))<=314385095?0:1)' "$RUNNER_TEMP/flow.jsonl" && npm test --prefix app
```

判定の基準:
- 段階 1 の判定(`memory` の (i) と (ii)。#{{140}} の「判定の基準」と同じ式)が、そのまま通る(回帰)。
- `error`: null。`cpu.busyIdlePct`・`cpu.busyPct` が数値(判定 ④)。表示の数字は判定しない。旧 ③(表示と忙しい時間の比較)と旧 ④(表示と rAF の比較)は、表示の数字を自動で読めないので外した。
- 判定 ①(時計): `phases.idle.clockRate`・`phases.playing.clockRate`・`fps.samples[0].clockRate`・`fps.samples[1].clockRate` が、すべて 0.95 以上 1.05 以下。
- 判定 ②(Hz の追随): `hz.steps` が 3 点(`name` が native・slow・back の順)。各点で `rafPerSec` > 0、`|envHz − rafPerSec| ≤ 0.15 × rafPerSec`、`|dwmPerSec − rafPerSec| ≤ 0.15 × rafPerSec`。試験が効いたこと: `slow` の `rafPerSec` ≤ 0.75 × `native` の `rafPerSec`、`back` の `rafPerSec` ≥ 0.85 × `native` の `rafPerSec`。
- 起動(`bootbundle-1`〜`3` の 3 回): 3 回とも error null・engine-idle・standIn false。elapsedMs の中央値が 12,500 以下。
- 音声(`measure.mjs --boot` の 3 回)・流れ A〜D・`npm test --prefix app`: 元のまま。
- 判定の式は、合成データで確かめてある(通る組: 時計 1.0・Hz が rAF に追随。落ちる組: 60 のまま rAF 30・追随しない・時計 0.9・試験が効いていない)。
- 変更の前後の比較(報告に書く): 起動の elapsedMs(中央値)、P3 の 3 回、流れ A〜D の elapsedMs、止まりの道具の結果(main に在る時)。
- 報告に書く値: CPU(`busyPct`)・Frame(各点の `rafPerSec`・`envHz`・`dwmPerSec`)・Memory、各窓の `clockRate`。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には「runner の値。この機械とは別枠」と書き、nproc と loadavg を添える。
- 「達成」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。出力の行を付ける。
- 報告に書く: 表示の値の出どころ(FL が呼んだ API と戻り値)、仮想の時計を変えたか(変えたなら、起動と音声への影響を、変更の前後で)、3 つの値(CPU・Frame・Memory)の直す前と後、低い値が出た時は、その理由(描画の費用・#1351 との関係)。DwmFlush の結び方と、決定 0166 の「2 回」の仮定の確認の結果も書く。

## 制約

- 製品に、利用者向けの新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足さない。オプトインを作らない。既存の FLE_DWM_HZ を読むのは可。診断は一時的にし、commit しない。
- 数字を作らない。値を良く見せない。低い値が本当なら、そのまま出す。
- ゲストのメモリには書かない(読むだけ)。
- 新しい周期の処理を増やさない(数えるのは、既存の情報: busyMs・presents)。DwmFlush の回数は、呼び出しに数えを 1 つ足すだけにする(4 回目の決定)。
- Python・OCR は使わない。
- ベンダー名と Cloudflare の識別子を、ファイルに書かない。
- sleep・timeout コマンド・nohup・末尾の `&` は使わない。一時ファイルは `.tmp-*` に置き、終わったら消す。
- 製品の作業は Actions で行う。この機械で製品のビルドや測定をしない。
- 名前の扱い: 禁止されているのは、AI のベンダー名と Cloudflare の識別子だけ。FL の製品名・インストール先の名前・変数名は、書いてよい。

## 作業の進め方

- 開始の時刻を、最初に `date -u` で記録する。4 時間 30 分を過ぎたら、受け入れが通らなくても、作業ツリーを commit し、報告を書いて終える。
- engine の作り直しは約 35 分。ビルドは、フォアグラウンドで、シェルツールの `timeout` 引数に 5400000 を渡して待つ。背景で起動して待たない。
- 作業の最後に、受け入れが通らなくても、作業ツリーを commit する。
- この Issue は 1 回の試行で終わらない可能性が高い(表示の読み・DwmFlush の結び方・判定の 2 つ)。その場合は、次の試行が `.prior-work.md` から続ける。

## 他のタスクとの関係

- blocked_by: #{{140}}(段階 1)。
- #{{138}}・#{{144}}: 同じ engine の経路の作業。直列の順で扱う。
- #{{149}}(起動の間欠の停止): 流れ A〜D・起動確認が落ちたら、報告に書いて、commit して終える。再試行で続く。この Issue の依存には足さない。
- #1351(P2 の描画の費用。Frame Rate が低いと出た時の原因の側。この Issue では扱わない)。提示が約 11/s の件は、その後の #1423 とともに、この Issue では扱わない。
- #{{158}}(P3 の測定に使う demo を決める)・#{{160}}(demo 2 件の素材の配信): 負荷の高い場面(demo の曲の再生)で CPU の表示を検証するのは、別の Issue で行う。この Issue では、現在の道具の再生の状態で判定する。

## 他の Issue との重なり

#{{139}}(マウスカーソル段 1)と、engine から page への通知のメッセージ型を足す変更が、`app/src/engine/protocol.ts` と `app/src/engine/worker.ts` で重なりうる。**足す変更どうしなので、`main` への push で衝突したら、両方の型・handler を残して解く**(片方を消さない)。解いた後、`npm test --prefix app` と受け入れを通す。
