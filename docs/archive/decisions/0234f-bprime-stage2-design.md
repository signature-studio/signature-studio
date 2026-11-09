# 0234f(mt3): B′ 第 2 段(ミキサースレッドの実スレッド化。Worker 1)— 設計メモ(スパイク前。読み。実装前)

状態: **設計メモ(読み)**。コードを読んだ事実(`lift/runtime/`、`fle_thread.c` 1,460 行、HEAD 4977340a)と mt2/mt3 の実測を土台にした提案で、**実装も測定もまだ**。第 1 段(R/FL/X の TLS、決定 0234e)は済み。目的 = UI(Worker 0)とミキサー(Worker 1)を別の実 CPU にする(P3: F2xH6 のミキサーは 1.5 コア要る〔mt3 第 3 報〕ので 1 Worker には載らない)。

## 方針の骨子
**『協調スケジューラが切り替えてよい点』と『ランタイムが他スレッドの割り込みを許す点』は同じ**(`schedule()` の待ち・譲り、`fle_call_guest` のコールバック)。ランタイム(約 300 のファイル範囲の可変大域、45,073 行)はそのために**すでに切り替えに耐える**ように書かれている。だから実スレッドの保護は『1 本の大きな lock(GIL)を、その点で手放す』で、細かい lock を各状態に足さずに済む(読み)。並列に走るのは **ランタイムの外のゲストコード(リフトされた wasm)**で、ミキサーの CPU の大半はそこ(Elastique・IPP・QFC。ランタイム呼び出しは約 1,750 回/s、mt2 §2.5)。

## ① 共有ランタイム状態の保護 — GIL(ランタイムの入口で取り、ゲストへ戻るとき・待つとき・コールバックを呼ぶ前に離す)
- **取る**: ゲスト → ランタイムの入口(`fle_imp_*` 約 710 個)。生成 C は触らず、リンクの `--wrap=fle_imp_<名前>` を**インポート表から自動生成**した `fle_gil_wrap.c`(mt2 の計測で同じ道具を使った。FL 固有の分岐なし)。ほかの入口: JS → wasm の export(入力 `fle_input_host_*`、デスクトップ `fle_desk_*`、idle フック)。
- **離す**: (1) ランタイム → ゲストに戻る出口、(2) **`fle_call_guest`(29 か所、14 ファイル。WndProc・EnumWindows のコールバック・タイマー手続き)を呼ぶ前**(戻ったら取り直す。ゲストのコールバックの実行中は持たない)、(3) `schedule()` の待ち・`switch_to`・JSPI の中断・`Atomics.wait` の前。再取得は再開の直後。= 協調モードの切り替え点と一致するので、既存の不変条件はそのまま通る。
- **費用**(micro、実測。`tools/gilbench.c`、node 24、1 CPU に固定、load 25): GIL の取得 + 解放(wasm の `cmpxchg` + `exchange`)は**非競合で 1 呼び出しあたり約 8.0 ns**(素の呼び出し 0.4 ns に対し 8.4 ns)。UI は約 110 万 API 呼び出し/秒(IntersectRect が 108 万)→ 約 8.8 ms/s = **約 0.9% の CPU**(wrap 関数の呼び出しの分を足しても 1〜2%)。競合したときの費用は未測定(スパイクで)。**頻度の高い純粋な API は GIL を取らない許可リスト**(データ駆動。測定で上位から決める。FL 固有ではない)で避ける。ミキサーのランタイム呼び出しは 1,750/s、保持は数 µs なので UI の待ちは小さい(mt2 §2.4 の CS の保持時間 12〜49 µs と同じ桁)。スパイクの最初の測定 = 非競合の GIL の CAS を 100 万回/s 回したときの UI の CPU。
- 細かい lock(CS 表・SList 表・窓表を別々に)は、GIL の競合が測定で問題になってから(第 3 段)。
- `fle_host_tid()`(14 か所の『第 2 のスレッドで呼ばれた』検査)は wasm では定数 1 を返すので、**2 Worker でも発火しない**(検出されない = GIL が守る)。

## ② 時計 — 実スレッドのモードは実時計(仮想時計は 1 Worker の協調モードだけ)
- 仮想時計(`fle_sys.c` の `fle_clock_elapsed`)は、slice(間接呼び出し 4,096 回 = 1 ms)・全スレッドの待ち・時計を読む API で進む『仕事量の時計』で、**Worker をまたぐと意味を失う**(どの Worker の仕事で進めるか決まらない)。実スレッドのモードでは `fle_sys_clock_ticks()` を **エポック時刻**(`performance.timeOrigin + performance.now()` − Worker 0 が起動時に共有メモリへ書いた原点)にする。Worker 間で同じ式 = 矛盾しない。時計の実装は `fle_sys.c` に閉じている(`fle_clock_elapsed` の参照 5 か所、`fle_sys_clock_ticks` の他ファイルの参照 17)ので差し替えは小さい。
- `Sleep(ms)` / `WaitFor*` の期限 / タイマー(WM_TIMER)/ `DwmFlush`(60 Hz の境界)は、その時計の期限になる。各 Worker のスケジューラは『準備できたスレッドが無いとき、**自分の** 待ちの最小の期限まで待つ』を、仮想時計の跳躍でなく**実時間の待ち**にする(Worker 0: JSPI の async 待ち〔イベントループが回り、ページの入力が入る。既存の `fle_audio_wait_until` と同じ型〕、Worker 1: `Atomics.wait(自分の futex, 期限)`)。slice は各 Worker の協調切り替えの点として残るが、時計は進めない。
- DirectSound の再生位置は今も実時間(AudioWorklet の消費フレーム数)なので変わらない。**G3(決定的な試験)は協調モード + 仮想時計のまま**(既定は変わらない)。実スレッドは別のゲート(§⑥)。

## ③ スレッド生成の振り分けと Worker 1 の協調スケジューラ
- 規則は『最初のスレッド(番号 0)は Worker 0、それ以外は Worker 1』だけ(クラス名・アドレスを見ない)。スレッド表 `T[]` は共有メモリの大域のまま(`nthr`、`wait_seq` はアトミック)で、各エントリに `worker` を足す。`pick_other()` は自分の Worker のスレッドだけから選ぶ。優先度つき選択(`fle_thread_set_priorities`)は各 Worker の中で使える。
- 最初の `CreateThread` で Worker 0 が Worker 1 を起動する(入れ子の Worker は Chrome 156 / Firefox 158 で動作確認済み〔mp1〕): `postMessage` で `WebAssembly.Module`(コンパイル済み。構造化複製で共有)と共有 `Memory` を送り、Worker 1 は同じ imports で `instantiateWasm` し、輸出関数 `fle_thread_worker_main()` に入って **自分の idle ループ**(READY を探して JSPI スタックで走らせ、無ければ futex で待つ)に留まる。JSPI のスタックは各 Worker の JS(`fle_thread_lib.js` のインスタンス)が持つ。新しいスレッドは Worker 0 が `T[i]` を READY にして Worker 1 の futex を `notify` する。TEB・TLS・ゲストのスタック領域は共有ヒープ/ゲストのアドレス空間なのでどちらの Worker からも見える。
- Worker をまたぐ起床(SetEvent・ReleaseSemaphore・PostMessage・CS の解放・スレッドの終了)は、起こされるスレッドの Worker の futex への `notify`(`poll_waiters` の延長。起床 p50 5〜20 µs〔mp1〕)。

## ④ ほかの文脈変数の TLS 化(第 1 段は R / FL / X だけ)— 残り 13 + 直 wasm が触る物
C 側だけ(単純な `_Thread_local` の宣言。協調モードの保存・復元はそのまま): `YH`、`MM`、`fle_x87_r[8]`、`fle_x87`、`fle_x87_mmx_active`、`fle_seh_top`、`fle_seh_cur`、コールバックの深さ、トレースの深さ、`fle_teb_cur`、`fle_tls_cur`(約 50 行)。**直 wasm が絶対アドレスで触る物**(`--hybrid-mx` の `fle_mxcsr` と `fle_hyb_buf`、`--hybrid-progress` の進捗カウンタ/上限、`--teb` の TEB の読み、`fle_landing_top`)は、`fle_ctx` のブロックの後ろ(+0x300…)に置いて `--tls-ctx` の基底に載せる(第 1 段と同じ型で、fe-wasmgen の約 10 か所 + 約 100 行 + テスト)。第 1 段の測定(再生 +3.3〜4.5%)はこの増分でさらに少し増える(読み)。`fle_progress_count` は Worker ごとに数えるのが slice の意味に合う。
- ゲストの `GetLastError`・TLS スロット・FLS は T[i] の中にあり、スレッドごと(Worker に依らず)。

## ⑤ ミキサーの依存の安全化(mt2 §2.5: ランタイム呼び出し約 1,750/s、窓・GDI・メッセージは 0)
| 依存 | 安全化 |
|---|---|
| CS(UI と共有する 4 個。熱い 2 個は 5,000〜7,000 回/s、保持 12〜49 µs) | CS 表は GIL の下。競合したら W_CS で待ち、解放側が相手の Worker の futex を notify |
| SList(約 1,650/s)・Interlocked | GIL の下(後でアトミックに) |
| `HeapAlloc`/`HeapFree`(約 100/s) | `malloc`(`-pthread` の libc はスレッド安全) |
| `GetThreadPriority`/`GetCurrentThread`(1,650/s)、`Sleep`(195/s)、QPC | 純粋または時計(②)。許可リスト |
| DirectSound(`Lock`/`Unlock`/`GetCurrentPosition`/`Play`)と ring への書き込み | デバイスの状態は共有 = GIL の下。**ring への publish と消費フレームの読み出しの JS(`fle_audio_ring.c` の EM_JS 9 個)は `globalThis.__fleAud`(Worker 0 の JS の状態)を前提**なので Worker 1 では動かない → ring を共有メモリの上で AudioWorklet が直接読み、消費フレーム数も worklet が共有メモリへ Atomics で書く(変換・通信の削減。postMessage の中継が要らなくなる) |
| `PostMessage`/`SendMessage` | ミキサーは使わない(0 回)。UI 側の入力は Worker 0 の JS → `fle_input_host_*`(GIL)→ 起床の notify |

## ⑥ 検証計画(ゲートを 2 つに分ける)
- **協調ゲート(既定、変更なし)**: 84 行 / 55 行 / 552 行が既定と全行一致(第 1 段で確認済みの手順)。
- **実スレッドのゲート(新)**: (1) native の `-fsanitize=thread` で thread_scenarios を実 pthread で(GIL の抜けの検出)、(2) node の `worker_threads` で Worker 1 を起こす最小の試験(mp1 の拡張: GIL の CAS、起床、`Atomics.wait` の時間切れ)、(3) node で全エンジン: 起動の 16 スナップを **静止点**(全スレッドが待っている点)で取り、面と窓表のハッシュが協調モードの同じ静止点と一致(非決定な中間は比べない)、10 回繰り返し、(4) 参照プロジェクトの再生が **5 分止まらない**(名前つきの停止 0)、PCM のハッシュの安定、(5) Chrome → Firefox の実ページ。
- **打ち切り(PM の基準)**: node で『ミキサーを Worker 1 に載せて参照プロジェクトの再生が 5 分止まらず、起動の 84 行が協調モードと一致』に 4 時間で届かなければ止めて原因と見積りを報告。

## 主な危険(読み)
1. GIL の競合(UI の API 呼び出し 110 万/s)— 許可リストと測定で。2. 入れ子の Worker での JSPI(**node 24 では確認済み**: 深さ 2 の worker_threads で `EM_ASYNC_JS` が動く〔`tools/jspinest-run.mjs`〕。Chrome・Firefox の入れ子は未確認 — エンジンの Worker 0 自体がページから起動された専用 Worker で JSPI を使っているので同じ型のはずだが、スパイクの最初に確認)。3. 128 MB の wasm Module の第 2 インスタンス化の時間とメモリ(共有メモリの成長時に JS 側の view が古くなる点も)。4. コールバックの中で GIL を持ったままになる道の見落とし(`fle_call_guest` 29 か所 + `fle_thread.c` 内の switch 経路の全数確認が最初の作業)。5. 実時計のモードで失う決定性 — 別のゲートで補う。

## 見積り(読み)
スパイク(第 2 段の最小): GIL 入口の自動生成 + 解放点(約 400 行)、実時計と Worker ごとの待ち(約 300 行)、`worker` 振り分けと Worker 1 の起動・idle ループ(約 500 行 + JS 約 300 行)、残りの文脈の TLS(約 250 行)、audio ring の共有メモリ化(約 200 行 + worklet)、検証(約 300 行)= **約 2,000〜2,500 行、スパイクの目標 4 時間(PM の打ち切り基準)**。
