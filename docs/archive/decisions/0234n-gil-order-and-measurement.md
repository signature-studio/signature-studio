# mt3: GIL の待ち順(優先度)、GIL の保持時間の測定、セマフォ/プロセッサ数の設計(PM 15:00 前の指示 (ii)(iii)(4) への回答。決定番号は PM)

状態: **作業記録**。測定は非ハイブリッド(最適化版、Chrome 156、この機械、他の担当の負荷 9〜28)。ハイブリッド + GIL の測定は別記(ビルド中)。

## 1. (ii) GIL の待ち順を『W0 優先』から『待っているゲストスレッドの優先度順』へ(実装済み)
- 旧: 最初の Worker(UI)が待っている間、他の Worker はロックを取らない(UI が 27 s 飢えた不具合の対症)。
- 新(`fle_gil.h/.c`): ロックが空いていて待ち手がいなければ CAS 一発。待ち手がいれば、Worker は **自分が走らせているゲストスレッドの優先度**(`fle_thread_gil_prio()` = SetThreadPriority が保持した水準。最初のスレッドは `fle_proc_first_thread_priority`)と到着順の鍵 `((prio+32) << 32) | (0xFFFFFFFF - ticket)` を公開して待つ。離すとき `gil_gen` を上げて全員を起こし、**鍵が最大の者(優先度が高い順、同じなら先着順)だけが取る**。Worker の番号による優遇は無い。エージングは入れていない(Windows の契約: 高い優先度が先、同じ優先度は順番。飢えは同じ優先度の FIFO が防ぐ)。
- 試験: Chrome 156 で 3 走行。結果は負荷(13〜28)に埋もれ、効果は**判別できない**(下の 3)。FL が音声スレッドに SetThreadPriority を設定しているかは未確認(スレッド表に水準は出していない)。

## 2. (iii) GIL の保持時間の測定の道具(`FLE_GIL_STATS`、スパイク用。成果物にしない)
- `fle_gil.h/.c`: Worker ごとの保持時間(合計・回数・平均・最大)と、スローパスの待ち時間(合計・回数・最大・ヒストグラム <10us/<100us/<1ms/<10ms/<100ms/それ以上)。`gen_wrap.py`: import 名ごとの『ラッパー内の時間』。`fle_thread.c`: スレッドごとに『どの API で何 ms 止まっていたか』(`park`)。`FLE_DBG_REPORT_S=n` で n 秒ごとにエンジンのログへ出る。
- 注意: import 名ごとの数字は**ロックの保持時間ではない**(`Sleep`、`WaitMessage`、`WaitForMultipleObjectsEx` は中でロックを手放して待つので、ラッパー内の経過時間に待ちが入る)。保持時間は Worker ごとの数字を見る。

## 3. 測定(非ハイブリッド、最適化版、再生を含む約 130 s、負荷 11〜19)
- **GIL は混んでいない**: Worker 0(UI)の保持は 11.8 s(約 9% の時間)、12,681,742 回の保持で**平均 0.9 us**。Worker 1(ミキサーと見られる)は 1.7 s。全 Worker の保持の合計は約 14.5 s / 130 s = **約 11% の時間**。スローパス(待ちが出た取得)は W0 が 319,895 回(うち 92% が 10 us 未満、10 ms 以上が 14 回)、合計待ち 10.6 s(約 8%)。W1 は 195,746 回、合計 9.0 s(約 7%)。→ **ミキサーが 4 割待つ原因は GIL ではない**(読みを実測で否定)。
- **ただし 1 件、GIL を長く握る箇所があった**: `IFileOpenDialog_Show`(ファイルダイアログ)が、ページの人の操作を待つ間(2.5 s)ロックを握ったままで、W1 が 2530 ms、W5 が 2519 ms ロックを待った(最大待ち時間がそれ)。→ `fle_filedlg.c` で `fle_filedlg_host_pick` の前後にロックを手放す(`fle_gil_release/acquire`)。実装済み。
- ゲストの runtime 呼び出しは毎秒約 10 万回(`GetTickCount` 約 1.5 万/s、`IntersectRect` 約 4.5 万/s、`Sleep` 約 1.4 万/s)で、1 回ごとにロックの取得・解放が入る。保持が短いので害は小さいが、純計算の leaf(`IntersectRect`、`GetTickCount`)はロック無しにできる(設計: 引数だけを読み、共有の表に触れない import を『GIL 不要』と生成側の表で指定。規模 約 80 行)。
- **ミキサーが実時間に届かない本当の理由**: DS の書き込み量(`unlock_bytes`)から、FL のミキサーは **実時間の 0.56〜0.60 倍**でしか書いていない(20 s あたり 121〜130 ブロック、実時間なら 215.4)。出力側(AudioWorklet)の読み位置は実時間(1.0)で進むので、書き込みが遅れた分は無音で埋まる(非無音 5〜9%)。ミキサーの Worker の CPU は 57〜70%(1 本)で、残りの 3〜4 割は GIL 待ちではない待ち(`Sleep` / イベント)。この内訳は下の 3b。
- 非ハイブリッドの CPU は UI + ミキサーで 2.1 CPU 秒/秒(測定済み)。ミキサーだけでも 1 本の Worker では足りない見込み。

## 4. セマフォ(`CreateSemaphoreW` / `ReleaseSemaphore`)とプロセッサ数の設計(ランタイムと単体試験は 6eb1737d 基準の作業木で実装済み: 0234o。再 lift と全試験は未)
- **プロセッサ数**: 今は GetSystemInfo / GetLogicalProcessorInformation が 1 を返す(`fle_sys.c`、`fle_sysinfo.c`。決定 0034 の『選択』)。実スレッド版では、答えは『仮想システムの実際の状態』= Worker を立てられる数にする: ページが `navigator.hardwareConcurrency` を**事実として**エンジンへ渡し(環境変数 `FLE_PROCESSORS=n`。旗ではなく、ページが測った値の伝達)、ランタイムは GetSystemInfo の `dwNumberOfProcessors` / `dwActiveProcessorMask`、GetLogicalProcessorInformation の RelationProcessorCore を n 個の記録(各 1 ビット)にする。`FLE_MAXW` の上限(31)を超えない。
- 実験(Chrome 156): n を 2 以上にすると、FL が最初に呼ぶ未実装の API は **`CreateSemaphoreW`**(`fle: unsupported instruction at 0x1d1d5a0 [import kernel32.dll!CreateSemaphoreW] [guest return 0x1e84c73]`)。その先で他に止まる API があるかは、セマフォが無いので**まだ見えない**。
- 実装の形: `fle_event.c` の表に種別『セマフォ』を足す(`count`、`max`)。`CreateSemaphoreW(attrs, initial, max, name)`(max > 0、0 ≦ initial ≦ max、さもなくば ERROR_INVALID_PARAMETER 87、名前付きは今のイベントと同じ扱い)、`ReleaseSemaphore(h, n, *prev)`(count + n > max なら失敗、ERROR_TOO_MANY_POSTS 298)、待ちは `fle_event_wait_state(h, consume)` に『count > 0 / consume で count--』を足し、WaitForSingleObject / WaitForMultipleObjects / CloseHandle は既存の経路に載る。スケジューラ側は `try_wake` が待ち手を起こす(Worker が違えば wake word)。**import 表に無い API なので生成 C が変わる(再 lift が要る)**: opt1 の層 b+c+d の push の後。規模: ランタイム 約 150 行 + 試験(単体 + 4 スレッド) + 再 lift と全試験。

## 3b. ミキサーの待ちの内訳(`park` 計測、非ハイブリッド、最適化版、約 130 s の走行 2 回、負荷 11〜12)
- ミキサーと見られるスレッド(thread 17、Worker 1。`Sleep(1)` で DS の位置を見に行き、DS バッファへ書く方): 29.1 s 止まっていた。内訳: **`Sleep` 21.4 s / 18,832 回(1 回 1.14 ms)**、**`EnterCriticalSection` 7.7 s / 309 回(1 回 25 ms)**。別の走行では同じ CS を **6.4 s 待った 1 回**を含む(合計 7.2 s / 8 回)。
- その CS は **0x16b6aa8、保持者はいつも thread 0(UI)**(`cs wait: thread 17 ... for the critical section 0x16b6aa8 held by thread 0`)。UI の Worker は 92〜94% busy(ゲストの描画)で、FL は UI の更新の間この CS を握るため、ミキサーは UI の描画の長さだけ待つ(25 ms/回 = Windows の 10 倍遅い描画が CS の中にある)。→ **ミキサーが実時間の 0.56 倍にとどまる主因の候補は『UI の描画時間』であって GIL ではない**。UI を速くする手(ハイブリッドで描画のゲストコードを直 wasm に、alpha3 の更新領域の矩形並び、描画の無駄な再描画の削減)がテンポに直接効く。
- 他: thread 7(Worker 7)は `Sleep(1)` を 27,360 回(31.3 s)と `WaitForMultipleObjectsEx` 27.7 s(1 回)。thread 5(Worker 5、start 0x1d0fdb0)は `Sleep` 59 s / 3,615 回(平均 16 ms の周期スレッド)。thread 0 は `WaitMessage` 14 s(アイドル)。FL 自身の複数コアのミキシングのヘルパーは、プロセッサ数が 1 なので仕事を受けない。

## 5. ハイブリッド + GIL のビルドで見つかった追加の壁(15:20)
- 『直 wasm → C の呼び出しを GIL の包みへ』は、`--c-imports` の `Name=__wrap_fle_imp_Name` の対応表(既存の機能)で直せた(ビルドスクリプト +6 行、h5 の木)。hybn0・weblink(831 の包み)・wasmopt・C 部の最適化は通った。
- **壁**: 実スレッド版は MXCSR / x87 / `fle_landing_top` / SEH を Worker ごとのブロック(`fle_ctx`、`FLE_TLS_MORE`)に置く。ハイブリッドの段階は `fle_mxcsr` の番地をリンクマップから読んで直 wasm に定数で埋め込む(`--hybrid-mx MXCSR BUF`、`lower.rs` の `i32.load offset=mx_addr` が 3 か所)ので、TLS 版では番地が無く `an address is missing in link.map` で止まる。直すには fe-wasmgen が MXCSR の番地を `__tls_base + オフセット`(スレッドごと)で出し、ハイブリッドの C 側の接着(`hybrid_glue.c` の `fle_hyb_mx_report`、`fle_mxcsr_load/store` の包み)も TLS の `fle_mxcsr` にする必要がある。規模 約 150 行 + 差分試験(TLS と通常で同じ答え) + ハイブリッドのビルド 40 分: **0.5〜1 日**。これが『ハイブリッド + GIL』の関門測定の前提で、今日の残り時間では終わらない。


## 6. Worker 1 の 5 ms 周期(audio_sync を周期的に走らせる案)の A/B — 効果が見えず、採らない(15:50)
- 案: 実時計 + 外部 consumer のとき、`fle_audio_next_tick` が Worker 1 だけに『now + 5 ms』を返し、`audio_sync` を一定の周期で走らせる(ミキサーが CS 待ちで遅れても、再生位置が進んだ分のハンドオーバーが遅れないように)。
- 測定(Chrome 156、非ハイブリッド GIL 最適化版、負荷 3.6〜5.7、同じシナリオ。S3 = 周期なし、T1/T2 = 周期あり):
  - エンジンの書き込み率(`unlock_bytes` / `t_ms`、176,400 B/s = 1.0)、10 s ごと 6 区間: S3 0.92 0.37 1.00 0.86 0.81 0.98 / T1 0.92 0.41 1.00 0.97 0.93 0.98 / T2 0.93 0.41 0.99 0.91 0.96 1.00。差は区間内のばらつきの範囲(0.37〜0.41 の区間は 3 本とも同じ位置 = 描画/CS の都合で、周期の有無に依らない)。
  - PCM の非無音の割合: T1 1,245 / 18,582 quanta(6.7%)、T2 1,427 / 18,774(7.6%)。周期なしは 5〜9%。underrun は 0(ただしこの指標は『エンジンが無音で穴を埋めた』ことしか表さない)。PCM テンポ 0.341 / 0.479(周期なし 0.19〜0.56)。
- 結論: 見える効果なし。『効果の見えない特別扱いは足さない』(lift の忠実性の方針)に従い**採らない**(h6 の差分には入っていない。t3 のスパイクにだけ残る)。同じ理由で DS 通知の予測も削除済み。
- 注意(未解決): エンジンの書き込み率は低負荷で 0.9〜1.0 なのに PCM の 9 割以上が無音という点は、この計測では説明できない。(a) 参照プロジェクトの冒頭 34 s の大半が実際に無音(トラックの配置)か、(b) ミキサーが書いている内容が無音(プラグインの音源が鳴らない、等)かを切り分けるには、同じ区間を main の単一 Worker 版でも取って非無音の割合を比べる必要がある(main の PCM も 0.27〜0.35 と疎で、実スレッド版が特に悪いとは言えない)。
