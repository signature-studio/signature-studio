# 0234c(mt3): ゲストのスレッドの優先度つき選択(Windows のスケジューリング優先度の契約)の実装と、音楽のテンポへの効果 — 決定 0234(mt2)の続き

状態: **仮の決定(すべての選択と推奨)**。依頼は PM(決定 0234 の続き。`mt3.diff` は `mt2.diff` とは独立)。作業者 `mt2`(mt3 の役)、2026-11-05 01:55〜。基準は HEAD **ba85d14b**(決定 0231 hyb6・0233 load1 を含む)の `git archive`(`lift/out-mt2/h3/`)。この文書は設計の調査と**契約の実装**であって、FL Studio の進捗の到達ではない(G 段階表に数えない)。

区分: **実測**=このセッションのコマンドの出力を見た事実(この機械 = 16 論理 CPU の Linux VM、他担当と共有で load 6〜30。load を併記。依頼者の機械ではない)。**読み**=コード・測定からの解釈。**推測**=根拠が弱い。**未確認**。代役・再生は含まない(実エンジンを実行した値だけ)。

## 1. 現状のスケジューラ(コードの読み。`lift/runtime/fle_thread.c`、HEAD ba85d14b)

- **切り替え点**: `schedule()` だけ。入るのは (a) 待ち・譲りの API(`Sleep`、`SwitchToThread`、`WaitFor*`、`MsgWait*`、`GetMessageW` が空、`WaitMessage`、競合した CS、`DwmFlush`、終了)、(b) 擬似プリエンプション(決定 0147): 間接呼び出し **4,096 回ごと**(`FLE_PREEMPT_SLICE_DISPATCHES`)に仮想時計を **1 ms**(`FLE_PREEMPT_SLICE_TICKS` = 10,000 ticks)進め、`fle_progress_slice()` が実行中のスレッドを READY にして `schedule()` を呼ぶ。コールバックの中(深さ > 1)では飛ばす。
- **次のスレッドの選び方(HEAD)**: `pick_other()` = `cur` の次から**作成順の巡回**で最初の READY(**優先度を見ない**)。決定 0122 の S1(出典なしの選択)『作成順の巡回、優先度は無視。`SetThreadPriority` は値を保持するだけ』。S1 の見直し条件は『実機の挙動の差がゲストの出力に出ると分かったとき』で、ref3 の実測(音楽のテンポが壁時計の 0.6 倍)がこれに当たる。
- **`SetThreadPriority`/`GetThreadPriority`**(`fle_proc.c`): 値を保持するだけ(最初のスレッドは `fle_thread_prio`、他は `T[i].ps.prio`)。値の集合は Learn の表(−15、−2〜2、15、REALTIME クラスだけ −7〜6)。
- **仮想時計**(`fle_sys.c`): `fle_clock_elapsed`(100 ns ticks)。進むのは (1) 擬似プリエンプションの 1 slice ごとに 1 ms、(2) 全スレッドが待つとき最小の期限まで(`fle_sys_clock_advance_to`)、(3) 時計を読む API(`GetTickCount`、QPC など)ごとに `FLE_CLOCK_READ_TICKS`。**実時間には結ばれていない**(ブラウザで音を再生中だけ、(2) の跳躍が実時間で待たされる: `fle_audio_wait_until`、決定 0139)。`Sleep(ms)` の満了 = 仮想時計が期限に達したこと(`poll_waiters` → `try_wake`)。DirectSound の再生位置は `fle_audio_consumed()`(ブラウザでは AudioWorklet が読んだ実時間のフレーム数)で、通知イベントは `audio_sync()` が**実時間の再生位置**が通知オフセットを越えたときに立てる(`poll_waiters` の先頭で呼ぶ)。= ミキサーが起きる時刻は実時間、ミキサーとUI の仕事量が進める時計は仮想時間(work-driven)。

## 2. FL 自身はスレッドの優先度をどう使っているか(実測。実エンジン、出荷 F2xH5 + 本書の優先度つき選択 + スレッドの trace、Chrome 156、参照プロジェクトを開いて Space で 34 秒再生。`runs3/chrome-trace1/engine-log.txt`)

`SetThreadPriority` の呼び出し(スレッド番号は作成順。レベルは Learn の値: −15 IDLE、−2 LOWEST、−1 BELOW_NORMAL、0 NORMAL、1 ABOVE_NORMAL、2 HIGHEST、15 TIME_CRITICAL):

| スレッド(この run) | レベル | 回数 | 読み |
|---|---|---|---|
| 0(最初のスレッド = UI) | 呼ばれない(NORMAL 0) | 0 | |
| 17(再生中の**ミキサー**。音声ブロックの Unlock を呼ぶスレッド。ページの audio pulse の `mixer` = 17) | **15(TIME_CRITICAL)**、再生中は **0 と 15 を 43/44 回交互に**設定 | 87 | FL は Delphi の `TThread.Priority` で、`[0x15e1ef0]` の設定バイト(TThreadPriority の 0〜6)を表 `0x13bd8c8`(−15、−2、−1、0、1、2、15)で引いて設定する(`f_525ab0`。読み) |
| 6(プロジェクトを開く前のミキサー) | 15 → 0 → 15 | 3 | 同じ型のスレッド |
| 7(1 kHz 級の短い周期で切り替わるスレッド) | 1(ABOVE_NORMAL) | 1 | |
| 1、3、4、5、11、18(作業スレッド) | −2(LOWEST) | 各 1 | |
| 2、12 | −1(BELOW_NORMAL) | 各 1 | |

= **FL はミキサーを TIME_CRITICAL、UI を NORMAL、作業スレッドを LOWEST に置く**。HEAD のスケジューラは優先度を無視して巡回するので、ミキサーが起きても UI と交互(各 4,096 回の間接呼び出し = 1 仮想 ms ずつ)に走り、ミキサーの 1 ブロック(4096 フレーム = 92.88 ms)の計算が 2 倍の壁時計時間を要する(読み)。

## 3. 実装(契約の実装。`mt3.diff`。`lift/runtime/fle_thread.c`、`fle_thread.h`、`fle_proc.c`、テスト `lift/harness/web/threads/thread_scenarios.c`)

**Learn の契約**(`Scheduling Priorities`、原文): 「The system treats all threads with the same priority as equal. The system assigns time slices in a round-robin fashion to all threads with the highest priority. If none of these threads are ready to run, the system assigns time slices in a round-robin fashion to all threads with the next highest priority. If a higher-priority thread becomes available to run, the system ceases to execute the lower-priority thread (without allowing it to finish using its time slice) and assigns a full time slice to the higher-priority thread.」`Sleep` の dwMilliseconds = 0: 「relinquish the remainder of its time slice to any other thread that is ready to run」(Windows XP だけ『同じ優先度の』。Server 2003 以降は『任意の』)。基本優先度 = プロセスのクラス × スレッドのレベルの表(同ページ)。**全ゲストスレッドは 1 プロセスなので、順序を決めるのはレベルだけ**で、表はどのクラスでもレベルについて単調非減少(IDLE −15 ≤ LOWEST −2 ≤ BELOW_NORMAL −1 ≤ NORMAL 0 ≤ ABOVE_NORMAL 1 ≤ HIGHEST 2 ≤ TIME_CRITICAL 15、REALTIME の −7〜6 はその間)なので、**レベルをそのまま優先度に使う**(クラスの違いは全スレッドに同じ移動になる)。

**変更**(計 `+82/−12` 行。コメントを含む):
1. `pick_other()`: 『`cur` の次から作成順で最初の READY』を『READY のうち**レベルが最大**のもの、同じレベルでは `cur` の次から作成順で最初のもの』に。全スレッドが同じレベルなら従来の巡回と同じ。
2. `schedule_ex(slice)`: 擬似プリエンプション(`fle_progress_slice`)から呼ぶ `slice = 1` のとき、『他の READY が `cur` より**低い**優先度ならそのまま走り続ける』(Learn: 低い優先度のスレッドは、高い優先度が実行可能な間は走らない)。同じ優先度なら巡回(従来)、高い優先度なら切り替える(起きたスレッドの方が高いなら、次の slice の終わりに切り替わる = 起床後最大 1 slice = 仮想 1 ms)。待ち・終了・自発的な譲り(`Sleep(0)`、`SwitchToThread`)は『READY のうち優先度最大』へ(自発的な譲りは Learn の『任意の他のスレッド』)。
3. `fle_proc.c`: 最初のスレッドのレベルを返す `fle_proc_first_thread_priority()`(最初のスレッドだけ `fle_thread_pstate` を持たず静的変数にある)。`SetThreadPriority` が値を保持した後に `fle_thread_note_priority()` でスレッド trace に 1 行(`priority-set 番号 レベル`。trace が有効なときだけ)。
4. テスト 2 本(`priority_yield`、`priority_slice`): 優先度の違うスレッドの Sleep(0) の順序、slice の終わりで低い優先度は走らず・同じ優先度は巡回・高い優先度は起床の直後に走る。

**モデルにしないもの**: 動的な優先度の押し上げ(Learn `Priority Boosts`: 待ちの完了、前面のウィンドウ、**飢えたスレッドの押し上げ**)、`THREAD_MODE_BACKGROUND_BEGIN` の CPU 優先度への効果(ページに値がない)、マルチプロセッサ(1 プロセッサの契約)。= 低い優先度のスレッドは、高い優先度のスレッドが待たない限り走らない。

**方針 (b)(契約の実装)の確認**: FL のスレッド名・アドレス・クラス名による分岐は無い(優先度はゲストが `SetThreadPriority` で与えた値のみ)。他の処理(ページ・窓の管理)には触れない。決定 0122 の S1(出典なしの選択『優先度は無視』)を**契約の実装で置き換える**。

## 4. 測定の方法(実エンジン。ref3 の方法を読み取りで使う)

- **ビルド**: HEAD ba85d14b の CI stage スクリプト(`ci/stage-*.sh`)を `git archive` の木に当てる。基準 = ref3 の `ciA`(出荷 F2xH5。`NO_FS=1`、混成 1,009 関数、wasm 128,083,778 B)。runtime の 2〜5 ファイルだけ作り直し(その `obj/rt/*.o` を消して `objects` → `link` → `seed` → `bundle`。リンクは約 13 分)。F2xH6 は ref3 の `ciH6`(+ oggio・Elastique の 2 モジュール。混成は 1,009 関数のまま、wasm 146 MB)、L5s は ref3 の `ciL5s`(`hybrid=L5s`、`--mx-packed --mx-rc`)を基準に同じ手順。道具は `tools3/`(`iter3.sh`、`relink.sh`、`fsbuild.sh`、`fsiter.sh`)。
- **node(仮想時計)の同一性**: `NO_FS=0`・非混成の FS 版を HEAD から自前でビルド(`fsbuild.sh`)し、優先度つきを `fsiter.sh` で差し替え。`cmp-short.g3`(16 スナップ)。
- **Chrome 156 のテンポ**: `tools3/tempo.sh`(ref3 の `scn-ref3-tempo.json` = 参照プロジェクトを開き、Enter の窓が閉じて 8 秒待ち、Space の直前に PCM の捕捉を始め、34 秒再生し、止めて `capture.f32` を読む)+ ref3 の `tempo-fit.py`(`celtix_Albiorix_Snare_05.wav` の先頭 0.25 s の相互相関のピークを、FLP の再生位置に当てはめる。n=9 と n=14)+ `pcm-repeat.py`(8192 フレーム前と一致する非無音ブロックの割合)。load は run の前後を併記。
- **ミキサーの 1 ブロック**(計測用の木だけ。マージしない): `fle_audio_ring.c` のスレッド切り替えの通知で『各スレッドの壁時計の連続実行時間から、その中の実時間の待ちを引いた物』を積算し、`Unlock` ごとに『そのスレッドが前の Unlock から使った CPU 時間』と『最初の通知イベントから Unlock までの壁時計時間』を `fle-mt3-block:` の 1 行として stderr に出す(ページが log に取る)。`tools3/blocks.py` が p50/p90/p99/max と『92.88 ms 以内の割合』を出す。計測用の木の動作の切り替え `FLE_MT3_PRIO`(0 = 巡回、1 = 優先度、2 = 優先度 + コールバックの中でも高い優先度が起きたら切り替え)は環境変数で、**計測用の木だけ**(`mt3.diff` には入れない)。

## 5. 結果(Chrome 156、出荷 F2xH5 = HEAD ba85d14b、参照プロジェクトを 34 秒再生。この機械。load を併記)

### 5.1 優先度つき選択はテンポを改善しなかった(`tables/chrome-F2xH5-*.txt`)

テンポ = 音楽の速さ(壁時計に対する倍率。1.0 = 実時間)。ref3 の方法: PCM の捕捉に対する `celtix_Albiorix_Snare_05.wav` の先頭 0.25 s の相互相関のピークを FLP の再生位置に当てはめた傾きの逆数(`tempo-fit.py`、n=9 と n=14)。`FLE_MT3_PRIO`: 0 = 現行の巡回、1 = 優先度、2 = 優先度 + コールバックの中でも高い優先度のスレッドが起きたら切り替え(計測用の木だけ)。

| 周(load) | 現行(0) | 優先度(1) | 優先度 + コールバック内(2) |
|---|---|---|---|
| 静かな周(load 3.8〜7.8) | 1.000 / 1.014、commit 10.79/s、遅れたブロック 0 | 0.956 / 0.933、10.74/s、遅れ 1 | 0.995 / 1.024、10.79/s、遅れ 0 |
| load 10〜13(他担当の処理と重なる) | 0.919 / 0.962、10.16/s、遅れ 16 | 0.818 / 0.835、9.68/s、遅れ 30 | 0.874 / 0.890、10.05/s、遅れ 16 |
| 参考: 優先度の最初の build(別の run 群。load 6〜10) | 0.840 / 0.832、0.854 / 0.860 | 0.829 / 0.841、(trace 付き)0.851 / 0.861 | — |
| 参考: 高 load(30 以上、自分の wasm-opt と重なった。汚染) | 0.574 / 0.596 | 0.597 / 0.620 | — |

(遅れたブロック = commit の間隔が 1.5 周期 = 139 ms を超えた数。実時間の commit は 10.77/s。)各構成 1 回ずつなので統計的な差は言えない。**言えること: 静かな周では現行でテンポ 1.0 に達する(= 優先度が無くても足りる)、load が上がると全構成でテンポが 0.8〜0.9 に落ちる、優先度つきが現行より良かった周は無い**。

### 5.2 ミキサーの 1 ブロックの CPU 時間(`tools3/blocks.py`。計測用の木だけ)

Unlock ごとに、commit したスレッドの『前の Unlock からの連続実行の壁時計時間 − その中の実時間の待ち』(= そのスレッドが使った CPU 時間に近い)。DirectSound の通知イベントは使われていない(`wake->commit` が 0 件。ミキサーは Sleep + 再生位置の読み)ので、『起床から締め切り』でなく『commit 間隔』と『周期 92.88 ms に対する CPU の割合』で見る。

| run(構成、load) | ブロック数 / 34 s | CPU ms p50 / p90 / p99 / max / 平均 | 92.88 ms 以内 | 周期に対する平均 | commit 間隔 p50 / p99 / max | 遅れたブロック |
|---|---|---|---|---|---|---|
| F2xH5 現行(静か、3.8〜7.8) | 367(10.79/s) | 64.1 / 77.9 / 85.0 / 93.4 / 61.2 | 100% | 66% | 94 / 124 / 131 | 0 / 366 |
| F2xH5 優先度(静か) | 366(10.74/s) | 62.8 / 80.0 / 85.8 / 94.7 / 61.2 | 100% | 66% | 95 / 130 / 219 | 1 / 365 |
| F2xH5 優先度 + コールバック内(静か) | 367(10.79/s) | 61.2 / 77.7 / 83.9 / 93.1 / 59.1 | 100% | 64% | 95 / 129 / 134 | 0 / 366 |
| F2xH5 現行(load 12.8〜12.3) | 346(10.16/s) | 66.8 / 86.6 / 98.9 / 124.2 / 65.6 | 96% | 71% | 94 / 209 / 218 | 16 / 345 |
| F2xH5 優先度(load 12.3〜10.6) | 329(9.68/s) | 75.5 / 92.9 / 100.1 / 107.7 / 70.0 | 90% | 75% | 95 / 224 / 246 | 30 / 328 |
| F2xH5 優先度 + コールバック内(load 10.6〜10.3) | 342(10.05/s) | 70.1 / 86.2 / 98.2 / 110.1 / 66.7 | 97% | 72% | 96 / 222 / 252 | 16 / 341 |

= **F2xH5 のミキサーは静かな機械で周期の 64〜66%(平均 0.64〜0.66 コア)、p99 が周期の 92%、最大が周期の境目**。CPU が 1.1 倍遅くなる(load 10〜13 の周)だけで p99 が締め切りを超え、テンポが 0.82〜0.92 に落ちる。優先度やコールバック内のプリエンプションは、ミキサーの 1 ブロックの CPU を変えない(平均 59〜61 対 61〜70 は load の差)。= ミキサーが律速するのは『起きてから走るまで』でなく『ミキサー自身の計算量 ÷ Worker が得る CPU』(読み)。
