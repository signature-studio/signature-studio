# 0234g(mt3): 第 2 段のスパイク — (1) go / no-go(入れ子の Worker の JSPI)と (2) GIL の入口と離す点の全数確認

状態: **実測と読み**(HEAD 52034ca9 = 決定 0234 mt2/mt3 の取り込み後)。区分は他の mt3 文書と同じ。

## 1. go / no-go: 入れ子の Worker での JSPI — **go**
- **現行の出荷は JSPI**(`lift/harness/web/fruity/link-web.sh` の `-sJSPI`、ページの `features.ts` は `WebAssembly.Suspending` と `promising` を必須の機能として検査する)。Asyncify の経路は無い。Worker 1 でも同じ JSPI を使う。
- **Chrome 156 と Firefox 158**(headless、実機で実行): ページ → Worker A → Worker B(入れ子、`type: 'module'`)で、`-sJSPI` の emscripten モジュール(`EM_ASYNC_JS` の `await`)が**動く**。共有メモリ版(`-pthread -matomics`、`PTHREAD_POOL_SIZE=0`)も動く。`WebAssembly.Suspending` / `promising` は両ブラウザで function(`tools/jspinest-browser/`、`result-chrome.json`、`result-firefox.json`)。node 24 でも深さ 2 の worker_threads で動く。
- **依頼者の Firefox 159**: JSPI は Firefox **153 から既定で有効**(出荷。Mozilla の Bug 2044809、2026-07-21 のリリース。Baseline は 2026-09-14)。159 はその後の版なので、現行の bundle が動いている根拠と一致する(JSPI が無ければ `features.ts` の検査で最初に止まる)。= 代替経路〔Asyncify 等〕は要らない。未確認: 159 そのもの(手元は 158)。
- 残る未確認: 入れ子の Worker で**エンジン本体**(128 MB の Module、共有メモリ、`WebAssembly.promising` の入口)を動かすこと(スパイクの (3))。

## 2. GIL: 入口と離す点の全数確認(コードを grep で全部見た結果。FL の名前は使わない)
**不変条件**: GIL を持つのは『ゲストのスレッドがランタイムのコードを実行している間』だけ。ゲストのコードを走らせるとき・スレッドが止まる(協調の切り替え、待ち、Worker の idle)ときは持たない。

### 2.1 ゲスト → ランタイムの入口(取る)
| 入口 | 数 | 取り方 |
|---|---|---|
| `fle_imp_*`(ゲストが API として呼ぶ) | 約 710 | リンクの `--wrap` で自動生成(`lift/harness/gil/gen_wrap.py`)。ランタイム同士の別ファイル間の呼び出し(Worker が既に持っている)は素通し |
| **`fle_dispatch_miss` → `fle_win_try_thunk` → `fle_com_try_slot`**(ゲストの `call [reg+disp]` が COM のスロットや IAT のサンクに当たる経路。**DirectSound のメソッド〔Lock / Unlock / GetCurrentPosition / Play〕はここから入る**) | 1 関数 | `fle_win_try_thunk` の入口で取り、`fle_dispatch(jump)` の前で離す(**`fle_imp_*` だけでは足りなかった**) |
| `fle_progress_slice`(間接呼び出し 4,096 回ごとにゲストのコードから呼ばれる) | 1 | 入口で取り、出口で離す(中の `schedule_ex(1)` が切り替えの前後で離す・取る) |
| JS → wasm の export(入力 `fle_host_post_input`、デスクトップ `fle_desk_*`、appfile / appview、audio ring、fruity_idle) | 約 25 | 各 export の入口で取る(スパイクの段階 B。1 Worker の間は不要) |
| スレッドの開始(`fle_thread_wasm_entry_cur`、最初のスレッドの `fruity.c` の `fle_call_guest`) | 2 | 開始時に取る(`fle_call_guest` が離す) |

### 2.2 ランタイム → ゲスト・停止(離す)
| 点 | 数 | 離し方 |
|---|---|---|
| **`fle_call_guest`**(WndProc・EnumWindows・タイマー手続き・フック・DirectSound の通知。**呼び出し元 29 か所は全部この 1 関数を通る**) | **1 関数** | `fle_dispatch(va)` の前後(離す → 戻ったら取る) |
| 直接の `fle_dispatch` | 2 | `fle_seh_land`(SEH の着地。この枠には戻らない)は dispatch の前で離す。`fle_win_try_thunk` の `fle_dispatch(jump)` は上の表 |
| スレッドの切り替え `switch_to` | 1 | JSPI の `transfer` の前で離し、再開された直後に取る。終了するスレッド(T_DONE)は戻らない |
| `schedule_ex` の idle の待ち(`fle_thread_idle_hook`、`fle_audio_wait_until`) | 2 | 待つ前に離し、後で取る |
| `park()` 13 か所 | — | すべて `schedule()` → `switch_to` か idle の待ちを通る(上の 2 点で網羅される) |

### 2.3 見つかった落とし穴(メモの初稿にはなかった)
1. COM スロットの経路(DirectSound = ミキサーの主 API)は `fle_imp_*` を通らない。上のとおり別に包む。
2. SEH の着地(`fle_seh_land` は生成 C がインラインで呼ぶ → **生成の単位ごとに `-DFLE_GIL` が要る**)と longjmp の枠: 着地の前に離せば、捨てられる枠の後始末は要らない。
3. ディスパッチのキャッシュ(`fle_dispatch.h`、4,096 行 × 16 バイトの 2 語書き込み)は『協調モードでは切り替えが無いのでロック不要』と書かれている → **Worker ごとの TLS にする**(第 2 段で `fle_ctx` の仲間に)。
4. `fle_host_tid()` の 14 か所の検査は wasm では定数 1 なので 2 Worker でも発火しない(GIL の漏れを検出してくれない)。→ **診断**として GIL に所有者(Worker 番号)を記録し、同じ Worker が取り直す・持っていないのに離すときに名前つきで停止する(`fle_gil.c`)。これで 1 Worker の全エンジンの実行が『離す点の漏れ』の検査になる(ただし `--wrap` の薄いラッパーは『Worker が既に持っている呼び出し』を素通しするので、API の入口では漏れを隠す。API の入口の漏れは 2 Worker での競合で出る)。

### 2.4 費用(実測)
- 非競合: 取得 + 解放 = **約 8.0 ns / 呼び出し**(wasm の `cmpxchg` + `exchange`。node 24、1 CPU 固定、load 25)→ UI の約 110 万 API / 秒で約 0.9% の CPU(`tools/gilbench.c`)。
- **競合**(`tools/gilbench-contended.mjs`。JS の Atomics。Worker A = UI 型で GIL を取って離すだけを全力で、Worker B = ミキサー型で毎秒 1,750 回 H µs 保持。2 CPU に固定、load 16〜25): A のスループット **H = 5 µs で 12.0 M 回/s、20 µs で 11.9 M、100 µs で 9.8 M**(H = 0 で 13.3 M)。必要な 1.1 M 回/s に対し 9〜12 倍の余裕。A の待ち(5 µs 超)は 3 秒に 4,000〜7,800 回、p99 100〜900 µs、最大 3〜12 ms — **最大は保持時間でなく OS に保持者が降ろされた時間**(H = 0 でも出る)。スピンの長さ(64 回 / 4,000 回)で傾向は変わらなかった。ミキサーの締め切り(92.88 ms)に対し ms 台の停止は許容範囲、UI の 16 ms のフレームには効く(読み。実機では保持者の降ろされ方が違う)。
- 許可リスト(GIL を取らない純粋な API)の案: データ駆動で、API の呼び出し回数の census(`MT2C` の計数)から上位を決める。スレッドローカルの状態だけを触る `GetCurrentThreadId` / `TlsGetValue` / `GetLastError` / `SetLastError`(今回の census で 640 万 / 290 万 / 120 万 / 120 万回)、時計の読み(`GetTickCount` 260 万)、`IntersectRect`(UI で 108 万回/s)が候補。測定してから決める(段階 A の後)。
