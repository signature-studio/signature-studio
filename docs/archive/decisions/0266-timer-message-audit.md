# 0266(仮): FL 自身の RedrawWindow の連鎖は、ランタイムの時間・タイマー・メッセージ優先順位の値のせいで Windows より多く回っていないか — 回っていない。WM_TIMER は連鎖に関与せず、順序・合体・最小周期・レートは Learn と Wine 10.0 に一致した。連鎖の速さは FL の QFC タイマースレッドの Sleep(1000 / 表示 Hz) で決まり、その Hz はページが渡す値(alpha9)

**依頼者の機械・実機 Windows・実 GPU・ブラウザ・本番 bundle(afd46e67 / 251dfabc / df0512b0)での測定は 0 件。** 以下は、b42fad36 の生成 C から作った native N1 に、HEAD aa70edb9(runtime は b00932ec と同一)の runtime + 環境変数で切る診断トレース(`diag/a9-trace.diff`)を載せ、G3 の模擬入力(`scripts/mxmove.g3` のドラッグ 40 手と `scripts/a9-idle10.g3` の 10 秒 idle。**ユーザーの操作ではない**)で測った値。native N1 の行は本番 bundle の行ではなく、native の CPU は wasm の CPU ではない。Wine 10.0 の結果と私の読みは UNOFFICIAL(Wine は Windows ではない)。代役・再生・推測は含まない。

状態: **仮の決定**。作業者 `alpha1`、2026-11-05 JST 23:00〜。基準 HEAD は aa70edb9(PM の木。runtime と試験は b00932ec と同じ)。**違反は見つからなかったので `alpha9.diff` は無い**(PM の規則: 違反が無ければ『FL 自身の挙動』として表に残して止める)。契約の表を固定する試験だけを `optional/alpha9-tests.diff` に置いた(追加のみ、runtime 変更なし)。

## 結論(先頭の数行)

1. **WM_TIMER は FL の再描画の連鎖に関与していない。** 10 秒 idle と 40 手ドラッグ(662 ms)の両方で、窓手続きに届いた WM_TIMER は **0 回**。FL は 1 手に 1 回 `SetTimer(NULL, 自動 ID, 500 ms, TIMERPROC 0x866f90)`(ra 0x86ba0a)を呼び、次の手で `KillTimer` する(40 対)ので、一度も発火しない。
2. **連鎖を回しているのは FL の QFC タイマースレッド(スレッド 6、開始 0x1d0fdb0)**: `Sleep(floor(1000 / Hz))` → `PostMessageW(TSharedAnimTimerWindow, 0x8ffc)`(ra 0x1e860e7、`f_1e860a0+0x47`)を繰り返す。Hz は起動時に `EnumDisplaySettingsW`(ra 0x1e85853、`f_1e857b0+0xa3`)の `dmDisplayFrequency` から読む = ページが `FLE_DISPLAYSETTINGS_FREQUENCY` で渡す値(`app/src/engine/hz.ts`: requestAnimationFrame の測定を 60 / 75 / 90 / 120 / 144 / 165 / 240 に丸め、`FLE_DWM_HZ` と同じ値)。**この値を変えて Sleep の引数が変わることを確かめた**: 60 → 16 ms(62.4 回/s)、90 → 11 ms(90.9)、120 → 8 ms(125.1)、240 → 4 ms(250.0)。PM が見た「約 11 ms」は 90 Hz に当たる。1 tick ごとに主スレッドが 0x8ffc を処理し、FL の輪の `RedrawWindow` と lid 14 の 1 画素帯の WM_PAINT を出す(idle でも 62.8 回/s の WM_PAINT)。
3. **ランタイムの timer / 優先順位 / 時間は、Learn と Wine 10.0 に対して「より多く・より速く」配っていない。** 17 項目の表(`tables/timer-contract.tsv`)のうち、順序(送信 > ポスト > 入力 > WM_PAINT > WM_TIMER)、WM_TIMER が最低優先度で キューが空にならなければ飢える(S7)、遅れた timer は 1 つに合体し追いつき配信しない(S2 / S4)、複数 timer の別配信(S3)、`USER_TIMER_MINIMUM` 10 ms(S5)、各周期のレート(11 ms: 90.5 対 Wine 90.0〜90.5 回/s、16 / 20 / 33 ms は同数)、忙しいループの 1 回/取得(S6: 39.5 対 38.2〜38.7)は**一致**。違いは 2 件だけで、どちらも runtime のほうが少ないか、Learn の許す範囲:
   - 遅れて配った後の次の期限: runtime は『配信時刻 + 周期』(S-timer、出典なしの選択)、Wine は位相を保つ(旧期限 + 周期)(S8)。runtime が配る数は Wine 以下。
   - `Sleep(n)` は n ms ちょうど(Learn: 「1 tick より長く 2 tick より短ければ 1〜2 tick のどこでもよい」)。範囲の下端で、モデルの選択。
4. **一番大きなてこは FL の tick の速さ(= ページが渡す Hz)。** idle の CPU は tick にほぼ比例する(最小二乗: 1 tick = 約 1.42 ms、tick に依らない分は 10 s で約 51 ms): 10 秒の idle の native CPU は 60 Hz で 976 ms(10%)、90 Hz で 1,334、120 Hz で 1,786、**240 Hz で 3,621(36%、60 Hz の 3.7 倍)**(`tables/idle-cpu-vs-refresh-rate.tsv`)。1 tick に FL がするのは SendMessageW 32 回、IsWindowVisible 24 回、IntersectRect 12 回、WaitMessage 2 回ほか(`tables/idle-tick-api-calls.tsv`)。実 Windows の FL も同じ式で動くはず(同じ API を読む)が、P1〜P3 の予算には効く。ページが `FLE_DISPLAYSETTINGS_FREQUENCY`(FL の tick を決める)と `FLE_DWM_HZ`(DwmFlush の拍。この測定では 0 回呼ばれた)を別の値にできること(決定 0166 は別の変数)は製品側の選択で、PM の判断。
5. **GetTickCount の粒度は FL の動作を変えない**: GetTickCount を 15.625 ms 刻みにしても(`A9_GTC_QUANT=15625`)、ドラッグの 1,621 件の事象(post・取得・dispatch・timer 呼び出し)は**マイクロ秒まで同一**。読んでいるのは主スレッドの 1 箇所(ra 0x1e822d0)だけ(1 手の 16 ms に 24 回)。
6. **0262 への訂正**(私の誤り。結論は変わらない): 0262 の「1 手あたり」表は、ボタンを離した後の 700 ms の idle tick まで数えていた。lid 14 と lid 17 の値と合計が過大(BeginPaint の合計 6.36 → ドラッグ窓だけでは 4.51、RedrawWindow 6.97 → 5.08、lid 14 の BeginPaint 2.21 → 1.05、lid 17 の BeginPaint 1.74 → 1.05)。lid 64 と lid 16 は不変(1.90 / 0.49)。CPU/手(g3-cpu の m001〜m020)はドラッグだけの区間なので不変。表: `tables/0262-correction-drag-window-counts.tsv`。

## 冒頭の三点(正直さ)

1. **何を実測したか**: 実エンジン(`FLEngine_x64.dll` と同梱 DLL を x86-64 → C にした b42fad36 の生成 C)の native N1 を G3 の模擬入力で走らせ、(a) Mixer ヘッダードラッグ 40 手(窓 3608283〜3608945 ms の仮想時間)と (b) 起動から 6 秒後の 10 秒 idle を、env で切る診断トレース(post・取得・dispatch・timer・Sleep・待ち・時計読み出し)で数え、(c) 表示 Hz を 60 / 90 / 120 / 240 に変えた idle 10 秒の CPU(トレースなし 2 回ずつ)、(d) 待ちの期限を tick 境界に丸める診断(`A9_TICK_US`)を測った。(e) 同じ C 試験(`harness/alpha9/msg_timer.c`)を Wine 10.0(mingw、headless、3 回)と runtime に走らせて順序・合体・レートを比べた。**本番 bundle のバイト列での測定ではない。**
2. **結論**: 上のとおり。ランタイムは timer・優先順位・時間で FL の連鎖を Windows(Wine で代用)より多く回していない。連鎖の速さは FL のタイマースレッドが読む表示 Hz で決まる。
3. **未確認**: 実 Windows での Sleep / WM_TIMER の解像度と、FL のタイマースレッドが Windows でも同じ式(Sleep(1000 / Hz))であること(native N1 では Hz を変えて因果を確かめたが、Windows では未確認)。ページが実際に FL へ渡す Hz(本番のビーコン値は 0 件)。ブラウザ(wasm)での 1 tick の費用。FL の tick を減らしたとき FL の見え方(アニメーション)がどうなるか。

## 1. 問い

PM の問い: FL 自身の RedrawWindow の連鎖(QFC のタイマー連鎖 `f_1e858e0 → … → f_820f10`、1 移動 6.97 回〔0262の数字。結論の 6 の訂正で 5.08〕、TSharedAnimTimerWindow の約 11 ms のタイマー)が、ランタイムの与える時間・タイマー・メッセージ優先順位の値のせいで Windows より多く回っていないか。

## 2. 連鎖の出所(測定)

表: `tables/a9-stats-idle10-60hz.txt`、`tables/a9-stats-drag-40moves-60hz.txt`(生の事象は `logs/traces/`)。

10 秒 idle(60 Hz): スレッド 6 が `Sleep(16)` 618 回(間隔 16.000 ms、最大 16.36)→ `PostMessageW(lid 6 = TSharedAnimTimerWindow, 0x8ffc)` 618 回(62.4/s)。主スレッド(0)の取得は PeekMessage のみ(GetMessage は呼ばない): 0x8ffc(ポスト)618 + lid 14 の WM_PAINT 622(合成)= 各 50%、空の PeekMessage 1,648 回(166/s)。WM_TIMER は 0。スレッド 18 が `Sleep(1)` を 9,826 回(992/s、間隔 1.001 ms)。**スレッド 18 は FL の DirectSound ミキサースレッド `TMixerThread_DS`** と分かった(開始 0x412ae0 = スレッドの入口、実体は f_12aeec0 の Execute が f_5264e0 の Sleep ラッパーを呼ぶ。gdb で 18 番目の `CreateThread` の引数のオブジェクトの型名を読んだ。`logs/thread18-TMixerThread_DS.txt`。起動のダイアログを閉じるキーの 3 秒後、仮想時間 3604764.562 ms に生成)。

ドラッグ(662 ms、40 手): 主スレッドの返値 334 件 = ポスト 24.3%(0x8ffc 41 + WM_MOUSELEAVE 40)、合成の入力 12.6%(WM_MOUSEMOVE 40)、合成の WM_PAINT 63.2%(lid 64 が 53、lid 14 が 42、lid 17 が 41、lid 16 が 19、lid 85 が 19、lid 83・84 が各 16、lid 86 が 3)。WM_TIMER 0。tick(0x8ffc)は 16.000 ms 間隔で 41 回 = 手と同じ周期なので 1 手に 1 tick。

## 3. 契約の全数表

`tables/timer-contract.tsv`(17 項目: 取得の順序、最低優先度と飢え、合体、複数 timer、最小周期、期限の振る舞い、レート 2 種、TIMERPROC、KillTimer、FL の実使用、連鎖の出所、GetTickCount / timeGetTime、QueryPerformanceCounter、GetMessageTime、Sleep、DwmFlush)。コード: `fle_win.c` の `fle_imp_GetMessageW` / `fle_imp_PeekMessageW`(送信 → ポスト → `fle_win_synth_message`: 入力 → WM_PAINT → WM_TIMER → WM_QUIT)、`fle_imp_SetTimer`(10 ms 〜 0x7FFFFFFF)、`fle_thread.c` の `msg_ready` / 期限の計算、`fle_sys.c` の仮想時計(読み出し 1 回 = 1 µs、`Sleep(n)` = n ms)。

Wine との比較(`logs/wine-vs-runtime/`、`tables/` の表): S1(ポスト 2 + WM_PAINT + 期限の来た WM_TIMER)も S1b(ハードウェアのマウス移動を加える)も runtime と Wine で同じ順序。S2 の合体、S3 の最初の取得、S5 のレート(1・5・10 ms は 10 ms に切り上げ、runtime はちょうど 10.000 ms 間隔で 99.5/s、Wine は 97〜99.5/s で間隔が 1〜23 ms にばらつく)、S6、S7 も同じ。S8 だけが違う(上の 3 の差)。

## 4. 時間の与え方

- `GetTickCount` / `timeGetTime`: 1 つの仮想時計。読むたびに 1 µs 進み、1 ms 刻みで単調。Learn の GetTickCount は「システムタイマーの解像度に限られる(典型的に 10〜16 ms)」ので runtime のほうが細かいが、15.625 ms 刻みにしたときドラッグの事象は完全に同じ(結論の 5)。FL が GetTickCount を読むのは主スレッドの 1 箇所(`f_1e82290+0x40`)で、ドラッグ中 1,420 回/s。
- `timeBeginPeriod`: FL は 2 回 `timeBeginPeriod(1)` を呼ぶ(ra 0x1e85e77 と 0x81c146)。runtime は値を数えるだけで Sleep や timer に使わない。Windows では tick が 1 ms になる。
- `Sleep(n)`: runtime は n ms ちょうど。待ちの期限を 1 ms の tick 境界に切り上げる診断(`A9_TICK_US=1000`)では、スレッド 18 の `Sleep(1)` は 2 ms 間隔(497.6/s、992.5 の半分)になり、スレッド 6 の tick は変わらず(位相 0 のため。実 Windows では post の費用で 17 ms になる)、60 Hz の idle CPU は 3〜15% 減る(トレース付き 1,378 → 1,175 ms)。15.625 ms の tick(`timeBeginPeriod` なし)にすると tick が半分(32/s)になるが、FL は `timeBeginPeriod(1)` を呼ぶのでこの場合ではない。表: `tables/sleep-tick-experiment.tsv`。これは Learn が許す範囲のモデルの選択で、スレッド 18 は**オーディオのミキサースレッド**(DirectSound を `Sleep(1)` で見張るポーリング)なので、`Sleep(1)` が 1 ms から 2 ms になるとバッファの補充の間隔が変わる(P3 の『音が途切れない』への影響が読めない)。idle の費用は小さい(tick に依らない分は 10 s で約 51 ms = 0.5%)。変えていない。

## 5. 表示 Hz と CPU

`tables/idle-cpu-vs-refresh-rate.tsv`: idle 10 秒の native CPU(トレースなし、2 回ずつ、1 ms 分解能、機械の揺れ ±10%): 60 Hz 976 ms、90 Hz 1,334、120 Hz 1,786、240 Hz 3,621。tick に比例(1 tick = 約 1.42 ms)。1 tick の中身は `tables/idle-tick-api-calls.tsv`。

ドラッグ中は 1 手 = 1 tick(60 Hz で手が 16 ms 間隔のとき)。手の間隔が tick より短いときの tick あたりの費用は測っていない。

## 6. 直していないもの・直せなかったもの

1. 遅れた配信の後の期限を位相保存にすること(Wine の挙動)。Learn に記述がなく、FL の経路では WM_TIMER が使われないので、変える理由がない。
2. `Sleep(n)` / 待ちを tick 境界に丸めること。範囲内で、効果は idle の 3〜15%(60 Hz)、影響が読めない(オーディオ)。
3. 表示 Hz の扱い(ページ側)。`hz.ts` が FLE_DISPLAYSETTINGS_FREQUENCY と FLE_DWM_HZ に同じ値を渡す。FL の tick を 60〜120 に抑えて DwmFlush の拍だけ表示 Hz にする案は、FL のアニメーションの滑らかさと引き換えで、PM の判断。

## 7. 試験(任意)

`optional/alpha9-tests.diff`(2 ファイル、追加のみ): `lift/harness/alpha9/msg_timer.c`(Wine と runtime の両方で動く)と `lift/crates/fe-lift/tests/alpha9_msg_timer.rs`(runtime の順序・合体・飢え・レートを固定。`cargo test -p fe-lift --test alpha9_msg_timer` 1 passed、`~/work/a7-cargo` の複製〔HEAD の runtime〕)。HEAD の木に `git apply --check` が通る。

## 8. 再現

- Wine との比較: `tools/build-msgtimer.sh TREE OUTDIR`(native.out / wine.out)。
- トレース: `tools/build-n1b.sh`(GENOBJ = b42fad36 のコンパイル済み生成 C、NO_COPY=1、TAG)に `diag/a9-trace.diff` を当てた木を渡し、`A9_TRACE=1 A9_FROM=<ms> A9_TO=<ms> tools/run-n1.sh BIN scripts/mxmove.g3 OUT`(idle は `scripts/a9-idle10.g3`)。集計は `tools/a9-stats.py OUT/stderr.txt FROM TO`。Hz の変更は `FLE_DWM_HZ` と `FLE_DISPLAYSETTINGS_FREQUENCY`。
- 0262 の訂正: `tools/a8-drag-stats.py OUT/gdb.err LABEL END_MS`(END_MS を足した)。

## 9. 次の手

1. PM: 表示 Hz をページがどう渡すかの方針(結論の 4)。ブラウザ(wasm)の idle CPU を 60 / 120 / 240 Hz で測る(ci1 の手順)。
2. 0262 の文書に、結論の 6 の訂正を一行足す(docs のみ)。
3. 実 Windows が使えるなら、FL の QFC タイマースレッドの Sleep 引数(Hz との関係)と WM_TIMER の発火の有無を確かめる。

## PM の判断と取り込み(2026-11-05 23:45、docs のみ)

取り込み: alpha1 の最終報(23:40)の文書・表・道具・診断差分を `docs/investigations/alpha9/` に、決定文書を本ファイルに置いた。**製品の木に当てた差分は無い**(`alpha9.diff` は作られていない)。測定は alpha1 の native N1 と Wine 10.0 の C 試験で、PM は再現していない。ブラウザ(wasm)・実 Windows・依頼者の機械は 0 件。
判断:
1. **runtime は変えない。** 17 項目の契約の表と Wine 10.0 との C 試験で、順序・最低優先度・合体・最小周期・レートが一致し、違反が無い。`optional/alpha9-tests.diff`(試験のみ、追加 2 ファイル)は取り込まない(製品への影響が無く、表は docs に残る)。必要になれば `docs/investigations/alpha9/optional/` から取る。
2. **Sleep の tick の丸め(診断 A9_TICK_US)は入れない。** スレッド 18 は FL の DirectSound ミキサースレッド `TMixerThread_DS`(CreateThread の ThreadRec のクラス表で確認)の見張りで、992 回/s の Sleep(1) はバッファ補充の間隔を決める。丸めると P3 に影響する。idle の CPU への寄与は 10 s で約 51 ms(0.5%)で小さい。
3. **FL の描画の輪の速さは、ページが渡す表示 Hz で決まる**(`Sleep(floor(1000/Hz))`、native の idle CPU は 60/90/120/240 Hz で 976/1,334/1,786/3,621 ms/10 s)。ページは実測の表示 Hz を FL に渡す(Windows の FL と同じ式で忠実)。P2 の目標は 120 fps で、120 を超える Hz は費用だけが増える。ブラウザでの費用は gpu1 の決定 0263(Hz 別の費用表)の結果を待ち、`app/src/engine/hz.ts` の `STANDARD_HZ` の上端を 120 にする案を PM が判断する(app/src の小変更、旗なし)。
4. 決定 0262 の「1 手あたり」の表は、この調べで run の最後までの数え方だったと分かり、訂正を 0262 に追記した(結論は不変)。
5. 未確認: 実 Windows での Sleep / WM_TIMER の解像度、FL のタイマースレッドが Windows でも `Sleep(1000/Hz)` であること、ページが実際に渡す Hz(本番ビーコン 0 件)、ブラウザでの 1 tick の費用と音への影響。
