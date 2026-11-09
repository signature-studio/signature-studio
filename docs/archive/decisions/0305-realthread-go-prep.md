# 0305(仮): 実スレッド版を『唯一の構成』にする場合の準備 — (1) test.sh full の 16 失敗の仕分け〔10 本は足場の誤り、6 本は試験側の更新、E11 側の本物の問題は 0 本〕、(2) 変更表の項目別の工数・依存・触るファイル・No-options 違反の箇所、(3) count/limit の B の要否(mt3)

状態: **仮**(第 1 報)。作業者 mt3、2026-11-06 JST。依頼: PM(0305 仮。gpu1 の go/no-go #2 第 1 報で E11 の実スレッド版が No-go 条件に該当しなかったため、go の場合に着手できる準備)。基準: main 7373cd55(E11)+ `rt-on-e11.diff`(0301)、およびその足場が現在の main(25019fd1)に当たるかの確認。区分: **実測**=この作業のコマンドの出力を見た事実、**読み**=コード・文書の解釈と見積り、**未確認**。

## 0. 冒頭の三点(正直さ)

1. **実エンジン(wasm)の実行は 0 件。Chrome は起動していない。** この作業の実行は、(i) fe-lift の cargo 試験(22 binaries と、全体の `test.sh full`)、(ii) `grep` による旗・環境変数の箇所の数え上げ、(iii) `patch --dry-run`(`rt-on-e11.diff` を現在の main へ)だけ。**人日の数字は『読み』**(根拠を表の右列に『実測』と『読み』に分けて書いた)。代役・再生・モックは無い。
2. **16 失敗の直しは使い捨ての木(`~/work/mt3w-rt301/tree` = 7373cd55 + rt-on-e11.diff)で作って確かめた物で、製品の木には書いていない。** 差分は `impl/0305-fix-the-16-failures-on-rt-on-e11.diff`(13 ファイル、262 行)。確かめた範囲は『fe-lift の 22 binaries + 全体試験』で、**wasm の実行・ブラウザは 0**。
3. **未確認**: 現在の main(E12 以降)の上での全体試験(rt-on-e11.diff は clean に当たる〔実測〕が試験は E11 の上のみ)、wasm の実行での Export、Firefox、`count`/`limit` の A/B(§4)、各項目の実績(見積りだけ)。

## 1. 結論

1. **16 失敗の内訳: 足場の誤り 10、試験側の更新 6、E11 側の本物の問題 0**(§2。1 本ずつ根拠つき)。『足場が import 表に 2 行足したことによる固定値のずれ(期待値の更新)』と読んでいた 10 本のうち、**7 本は期待値の更新ではなく足場の誤り**: 足場は `CreateSemaphoreW`/`ReleaseSemaphore` を表の**途中**(`WaitForSingleObject` の次)に入れ、決定 0015『追加は末尾に限る。既存の行の擬似 VA は動かない』に反している。**表の末尾に移すだけで 7 本が直る(実測)**。残り 3 本(表の末尾から数える試験)だけが、試験の既存の規約(`tests/rows_0267` と同じ形の `rows_0234`)で『後ろに足された群を引く』更新になる。期待値の数(581 + k など)は契約のままで変えていない。
2. **直した木で全体試験(`test.sh full`、`FLE_TEST_NOCACHE=1 --jobs 2`)**: **222 results、2,421 passed、0 failed**(直す前: 222 results、2,405 passed、16 failed。2,405 + 16 = 2,421 で、通る試験の数が増減していない)。wall 623 s(build 230.7 s)。`tables/0305-full-test-with-fixes.log`。a4(5 モジュール構成)の再生成は第 2 報(`0305b`)で実施: 直した木で units 942/942、runnable 895/895、distinct 5,310、entries 1,009,737、終了コード 0 = **基準と同一**(実測)。
3. **変更表**(§3): 唯一の構成にする作業は、旗・環境変数の箇所が **40 ファイル・163 行**(実測)。工数の見積り(読み)は、**協調の経路を wasm から外す場合 約 18 人日(gpu1 の S2〜S6 の 17〜19 と整合)、協調を実行時の判断で残す場合 約 10〜11 人日**。**差(約 7.5 人日)の大半は wasm を走らせる試験の作り直し(約 6.5 人日)で、残りは協調の分岐の整理(約 1)。どちらも PM の判断(D1)で決まる**。
4. **Export(FL の File → Export → Wave file)**: レンダリングは**1 回のクリックの処理の中で UI スレッドが回す**(0188、0193)ので、実スレッドでも構造は同じ(Worker 0 の中で完結。他の Worker は待たれる側)。**コードの変更は要らないと読む**が、実スレッドでの実行は 0 件なので、WAV の sha256(0193: 353,904 B、`f5c6a08b…`)を実スレッド版でも取る試験を 1 人日で置く(§3.4)。
5. **count/limit の B**(§4): gpu1 の 5 分実測は count の A/B を含まないため、**『競合による問題が出ていない』とは言えるが『競合が無い』とは言えない**。競合の上限は私の合成の測定からミキサー側 約 6 ms / Unlock(実スレッドの税 +9.3 ms の約 2/3)と読めるので、**go の判定を阻む物ではなく先送りでよいが、最終判定の余裕が足りない場合の第一の候補**(§4)。

## 2. (1) test.sh full の 16 失敗の仕分け(`tables/0305-triage-16.tsv` に全列)

**方法**: 失敗の assertion の行を読み、(a) 試験が守る契約(決定の本文・試験の先頭の注釈)、(b) 足場の差分の該当箇所を突き合わせた。**期待値は実装の出力から取らず、契約から手で導いた**(下)。次に、直しを使い捨ての木に入れて 22 binaries を走らせ、全部通ることを確かめた(`tables/0305-experiment-2-all-fixes.txt`)。

| 仕分け | 本数 | 内訳 | 直し |
|---|---|---|---|
| **B 足場の誤り**(足場を直す) | **10** | (1)〜(7) 表の途中への挿入 7 本、(14) 製品コードの `getenv("FLE_PROCESSORS")`、(15) `fle_callback.c` の診断の print、(16) 旗の入れ子(`#ifdef FLE_GIL` の `#endif` が guard の中に入る) | 1 つの変更(表の末尾へ)で 7 本。ほか 3 か所は旗・診断の除去 |
| **A 試験側の更新**(足場は正しく、試験が追従する) | **6** | (8)(9)(10) 末尾から数える試験 3 本(`rows_0234` を引く)、(11) 文字列の検査 2 か所を決定の中身まで緩める、(12)(13) `wall_wrp1` の stub 表に 3 行 | 試験 8 ファイル + 新しい `rows_0234/mod.rs` |
| **C E11 側の本物の問題** | **0** | — | — |

**根拠(1 本ずつは tsv)**:

- **(1)〜(7) 『期待値の更新』でなく足場の誤り**: `imports_table.rs` の `TABLE` は行 k の擬似 VA が `0x7FF900000000 + 0x10 × (k+1)` で、決定 0015 は『追加は末尾に限り、並べ替えない』。試験の先頭の注釈も『decision 0015: append only』『the pseudo-VA of a row never moves』。足場は 2 行を 240 行目あたり(`WaitForSingleObject` の次)に挿入したため、後ろの約 540 行の擬似 VA が 0x20 動く(`GetModuleHandleA` の 0x7ff9_0000_0000 + 0x10×261 が 2 行ずれる、`HeapReAlloc` の行番号 257 が 259、など)。**期待値(469 + 73、544 + 33、577 + 4、260、257、261)は契約で、変えない**。直し = 2 行を `SetWindowPlacement` の次(`];` の前)へ。**実測**: 移した木で wall_fr9(30 passed)、wall_fr10(20)、wall_fr11(47)、wall_fr12(32)、deferred(15)、wall_c1(12)、srw(16 + 1 は下)、semaphore(6)が通る(`tables/0305-experiment-1-rows-moved-to-the-end.txt`)。
- **(8)(9)(10) 末尾から数える試験(wall_fr13、wall_fr16、wall_fr15)**: 末尾に足すと、これらの `n = rows.len() − (後ろに足された群の和)` が 2 行だけ合わなくなる(**実測**: 移した直後の同じ実験で wall_fr13・wall_fr16・wall_fr15 の 3 本だけが残った)。試験の既存の規約は、決定ごとに `tests/rows_NNNN/mod.rs` を置き、それ以前の行を押さえる試験が引く(`rows_0267/mod.rs` の先頭: 『wall_fr4、wall_fr5、wall_fr13、wall_fr15、wall_fr16、wall_fr19、wall_fr20 が共有する』)。**同じ形で `rows_0234`(= `CreateSemaphoreW`、`ReleaseSemaphore`)を足し、その 7 ファイルで `rows_0267` を引いている所に続けて引いた**。`n == 581 + k`、`n == 601 + k` などの断定は無変更。**実測**: wall_fr4(24)、wall_fr5(14)、wall_fr13(5)、wall_fr16(17)、wall_fr19(13)、wall_fr20(13)が通る。引き算を外すと wall_fr4/5/19/20 も落ちる(同じ末尾から数える式。実測、`tables/0305-final-checks.txt`)。
- **(11) wall_fr15 の『hook が決定の言う所にある』**: 決定の中身は『COM の slot は `fle_win_try_thunk` を通ってモデルに届く』『scheduler が切り替えを weak hook に知らせる』。足場は (i) `fle_win_try_thunk` を GIL の acquire/release で括り(hook の呼び出しはそのまま)、(ii) 切り替えの通知に `cur < FLE_THREAD_MAX` を足した(Worker が idle の文脈〔`BOOTW(w)` の枠〕にいると `cur` は表の外)。**どちらも決定の中身を変えない**ので、文字列の検査を決定の中身(`fle_com_try_slot && fle_com_try_slot(va)`、`if (fle_audio_switch_note && cur < FLE_THREAD_MAX) …`)に合わせた。
- **(12)(13) wall_wrp1**: 足場の loader lock(`call_dllmain` を critical section で括る。Learn の DllMain『loader lock』)で `fle_realmod.c` が `fle_imp_Initialize/Enter/LeaveCriticalSection` を呼ぶようになり、試験が `fle_realmod.c` 単体を stub 表で link するので未解決になる。**stub を 3 行足した**。断定の値(`fle_realmod_find_path` の結果)は無変更。
- **(14) srw の getenv**: 足場の `fle_sys_processors()`(`getenv("FLE_PROCESSORS")`)が SRW の節の中(`GetSystemInfo` の節の直前)に置かれ、試験が切り出す範囲に入る。同時に**製品のコードが環境変数の選択肢を読む**(No-options)。直し = 1 を返す(FLE_PROCESSORS 未設定のときの値。mt3 が測った構成)。
- **(15) callback**: 足場が `fle_callback.c` に 7 行の診断(`fle_dbg("W1 call_guest %#llx", (unsigned long long)va)`)を足した。試験は『このファイルは素の C で、host に触れない』ので `long` の語を禁じる。診断だけなので削除。
- **(16) gpu7_ring_poll**: 足場が guard の中に `#ifdef FLE_GIL … #endif` を入れ子にし、文字列の走査が入れ子の `#endif` を見る。意味は保たれているが、**唯一の構成では入れ子が無くなる**(`MYW` は native で `0u`)ので、旗の除去と同じ変更。直し = 入れ子を外して `if (MYW == 0)` を残す。

**全体試験(直した木)**: `test.sh full` 222 results、**2,421 passed、0 failed**(wall 623 s、`tables/0305-full-test-with-fixes.log`)。1 件の SKIPPED は `wall_fr9t` の環境変数が未設定の比較(元から)。

**E11 側の本物の問題が 0 本という言明の範囲**: 失敗した 16 本は全部上で説明がつき、直すと通る。**ただし試験の層(cargo、C)だけの結論で、wasm の実行・ブラウザは見ていない**。足場自身の挙動の誤り(例: 2 Worker の実行の誤り)は gpu1 の 5 分実測の側。

## 3. (2) 『唯一の構成』にする変更表(gpu1 0289 §8 を項目に分け、工数・依存・触るファイル・No-options・試験と CI)

### 3.1 先に決めること(PM の判断。工数が変わる)

| # | 判断 | 選択肢 | 工数への影響(読み) |
|---|---|---|---|
| D1 | **wasm に協調の経路(`!RT_ON`、`FLE_WORKERS=1`、仮想時計、pacing)を残すか** | (a) wasm は実スレッドだけ。native(N1・試験)は今のまま協調(= `__EMSCRIPTEN__` の違いで、既存の `#ifdef __EMSCRIPTEN__` 8 か所と同じ種類の差)。(b) wasm に協調を実行時の判断で残す(`hardwareConcurrency` ≤ 2 などで)。(c) 協調を native からも外す | (a) 約 18 人日。(b) 約 10〜11 人日(決定的な 84/55/87 行の試験が wasm でそのまま使える)が、**2 経路を試験し続ける**。(c) (a) + native の試験の作り直しで、(a) より大きい(未見積り) |
| D2 | Worker 数の決め方 | 2 固定 / ホストの論理 CPU 数の事実(`GetSystemInfo.dwNumberOfProcessors` と同じ種類の契約)から | 固定なら 0。事実から決めるなら +0.5 |
| D3 | 計測用 build(`FLE_GIL_STATS`、`FLE_CS_HOLD_STATS`、`FLE_HOLD_ATTR`)を製品の木から外してよいか | 外して、測定は gpu1 の別の patch(`mk-rts.sh`)で | 外す前提で見積り |
| D4 | count/limit B | §4 | 0.5〜1 人日(任意) |

### 3.2 項目別(順に着手できる並び。工数は読み、根拠の実測を併記)

| 項 | 内容 | 触るファイル(実測の数) | 人日(読み) | 依存 | No-options 違反が出る箇所 / 試験・CI | 根拠(実測)/ 根拠(読み) |
|---|---|---|---|---|---|---|
| **I1** | 試験 16 失敗の直し(§2) | 13 ファイル、262 行(`impl/…fix-the-16….diff`) | **0.5** | — | `FLE_PROCESSORS` の getenv を除く。試験は 22 binaries 通過、全体は §2 末尾 | 実測: 直しを入れて走らせた。読み: レビューと main への着地の手間 |
| **I2** | 足場(`rt-on-e11.diff`)を main に入れる | 60 ファイル(runtime 33、fe-lift 5、fe-wasmgen 6、harness 12〔web 3、mt2 2、gil 2 ほか〕、app 4〔build script 1 を含む〕)、追加 2,807 行・削除 132 行 | **0.25**(+ main の他の作業との調整) | I1 | 旗・環境変数が 40 ファイルに入る(I3〜I5 で除く)。**main へ入れる順は I3〜I5 の後のほうが旗が製品に残らない**ので、I2〜I5 を 1 つの系列で | **実測**: 現在の main(25019fd1)に `patch --dry-run` rc=0、60 ファイル、不成立 0、offset・fuzz 0(`tables/0305-dry-run-…log`)。E11 以降の main の変更(E12 の 0284c/0295 ほか)と衝突しない。読み: CI が走っている間は main に push しない取り決めの調整 |
| **I3** | 環境変数の選択肢・計測用 build・診断の除去 | `fle_sys.c`(`FLE_WATCH_CS` 226、`FLE_HOLD_REPORT_S` 313、`FLE_PROCESSORS` 764)、`fle_thread.c`(`FLE_WORKERS` 339、`FLE_DBG_REPORT_S` 761)、`fle_holdstat.h`、`fle_gil.c/.h`、`fle_callback.c`、`gen_wrap.py`(統計の 30 行 / 7 ファイル)、`fle_dbg(` 16 か所 | **0.5** | I2 | `FLE_WORKERS`(manifest の `engine.env` から読む= 利用者の選択肢)、`FLE_PROCESSORS`、`FLE_WATCH_CS`、`FLE_HOLD_REPORT_S`、`FLE_DBG_REPORT_S` の 5 つの getenv(**全部製品に入る**)。`FLE_GIL_STATS`/`FLE_CS_HOLD_STATS`/`FLE_HOLD_ATTR` は計測の build の旗 | 実測: 箇所の数(`tables/0305-flag-and-env-sites.txt`)。読み: 1 か所 5〜10 分の平均 |
| **I4** | `FLE_TLS_MORE`・`--hybrid-mx-tls`・`MXTLS`・`GIL=1` を常時にする(thread-local の register・MXCSR・scratch・landing) | `FLE_TLS_MORE` 20 ファイル/31 行(runtime のヘッダ群、`typed_indirect.rs`、`x87_threads.rs`、`hybrid_glue.c`)、`hybrid_mx_tls` 7 ファイル/21 行(fe-wasmgen の lib.rs 8・main.rs 3・lower.rs 3・wasm.rs 1、試験 2)、`build-engine-bundle.sh` 13 行(`GIL=1`、`MXTLS`) | **1.5** | I3 | `--hybrid-mx-tls` は fe-wasmgen の**旗**(CLI 引数)→ 常時にして引数ごと除く。`GIL=1` は環境変数で全オブジェクトを `-DFLE_GIL -DFLE_TLS_MORE` で作り直す。**生成する wasm が変わる = 基準(wasm の sha・大きさ)が動く**(a4 の生成は旗なしで変わらない: 0301 で実測済み)。試験: `typed_indirect`・`mx_hybrid_tls`・`x87_threads`・`ci/test-hybrid.sh`(`hyb_guard` の数 4 は 1 か所の呼びで保つ。0301)。CI: 全オブジェクトの key が 1 回変わる | 実測: 箇所の数。gpu1 が冷間 CI 段階(objects 406 s + link 777 s ≒ 20 分)を測った(0289 §2)。読み: native の試験の build が TLS で通るか(native N1 は register が thread-local になる。性能の影響は N1 の試験用) |
| **I5a** | `FLE_GIL` と `RT_ON` を常時にし、wasm から協調の経路を外す(D1 = a) | `FLE_GIL` 15 ファイル/44 行、`RT_ON` 18 行(`fle_thread.c`)。協調専用の物: 0279 の課金 k=4、IdleDriver の pacing(0170)、`fle_audio_wait_until`、`idle_stop`(`!RT_ON` の分岐) | **2**(`FLE_GIL` → `__EMSCRIPTEN__`: 0.5、`RT_ON` の分岐の整理: 1.5) | I3、I4 | 旗 `-DFLE_GIL`(全 `.c` の build の引数)、`RT_ON`(`rt_mode`、環境変数 `FLE_WORKERS` から) | 実測: 箇所の数。読み: 協調専用の分岐は `fle_thread.c` に集中(18 行)。ただし 0279 の課金は N1・g3 の疑似時計と結びつく |
| **I5b** | 協調を実行時の判断で残す(D1 = b) | I5a と同じ箇所だが分岐は残す | **1**(旗を実行時の変数に。ホストの論理 CPU 数から決める: D2) | I3、I4 | 『2 経路』が製品に残る。`rt_mode` を決めるのは環境変数でなくホストの事実 | 読み |
| **I6** | 音声: 段階 C(リングを wasm の共有メモリへ)。**ミキサーを持たない Worker の初期化**(§3.5)。**設計に明記する点(0305c で確定)**: gap の基準 `wr` と、読み位置の 64 bit 展開 `rdHi`/`rdPrev` は**共有状態**(共有の `hdr[1]` か `r->write_frames`)にし、**Worker ごとの JS 状態・遅延 init を製品の形にしない**。`audio_sync` は全 Worker のスケジューラ点から呼ばれ(`fle_thread.c:841` → `:481`)、Worker ごとの `A.wr` だと Worker 0 の publish が Worker 1 の未読の窓を零で上書きする(gpu1 の WRFIX=1 の PCM で確定: 零の塊 212 本・19.63% → 3 本・0.036%、協調 4 本・0.038%) | `fle_audio_transport.c`(約 200 行の置き換え)、`app/src/audio/` 約 5 ファイル、`fle-out.worklet.js`・`ring.js`、`app/test/unit/audio-*.test.ts` | **1.5**(0234l §4 は 1〜1.5) | I2(RT で確かめるため) | なし(『旗なし・常に有効』と 0234l に設計済み)。試験: 協調 node の同一性(`cmp-short` 84 行・`ref1-perf-short` 55 行)、`ds_scenarios`(sim_transport)、Chrome の 1 拍目の欠け(0241)、5 分のアンダーラン数 | 読み(0234l の設計。実装は 0)。**これが無いと Worker 1 から音が出ない**(0234k)ので、gpu1 の測定は post-link の glue 補修(`patchglue-rt.py`)で代用している |
| **I7** | wasm export の GIL 包み(`fle_g3_*`、`fle_host_post_input`、taskbar ほか)と `check_i1.py` を CI に | `fruity_idle.c`・`fruity.c`・`fruity_g3.c` の `fle_gil_enter` 12 か所、`gen_wrap.py`(`fle_imp_*` の包み約 710 を生成)、`check_i1.py`、`ci/` | **0.5** | I2 | なし。CI: `check_i1.py` は足場のファイル(main の HEAD に無い)で、`ci/`・`.github/` のどこにも接続されていない(実測: grep 0 件)ので、wasm 段階に 1 行足す | 実測: 12 か所、CI に無いこと(grep)。読み: 0.5 |
| **I8** | ページ: Blob の glue を pthread Worker に渡す、Worker 生成失敗の名前つき停止、Firefox の `Atomics.waitAsync` の確認 | `app/src/engine/fruity.ts`、`app/src/main.ts`、`app/test/e2e/browser.mjs`(足場の 80 行)、CSP の `worker-src blob:`、`fle_idle_lib.js` | **1** | I2、I6 | なし(`FLE_WORKERS` の manifest キーを外す=I3)。Firefox 159 は 0289 §8 のとおり**未確認** | 実測: 足場の 3 ファイル 80 行。読み: 1 日 |
| **I9** | L1c の入力 ring の poll を『Worker 0 のとき』だけにする(契約: 読み手は 1 人) | `fle_thread.c` の 1 か所 | **0.1** | I5 | `#ifdef FLE_GIL` を外す(§2 の (16) で試験が通ることを実測) | 実測: gpu7_ring_poll 6 passed |
| **I10** | `count`/`limit` の B(任意、§4) | `fle_lowregs.h`、`fle_ctx_more.h`、`lib.rs` のアダプタ、`lower.rs`(demoted の補正)、g3 の `progress`、`typed_indirect.rs` | **0.5〜1** | I4 | なし | 読み(0302 §7) |
| **I11** | Export(オフライン描画)の確認(§3.4) | 試験の追加(`app/test/e2e` か gpu1 の driver の 1 場面) | **1** | I6、I8 | なし | 読み: 0188・0193 |
| **I12** | 試験の作り直し(D1 で変わる) | cargo は I1 で済み。wasm の実行を使う試験: 起動 16 スナップ 84 行・参照 55 行・パリティ 552 行・e2e 229 | **D1 = a: 約 7.5**(84 行の同一性 2〜3、パリティ 3、e2e 2、音 0.5 = 0234m §2)。**D1 = b: 約 1**(e2e の pacing の差だけ) | I5 | 協調の同一性の試験は D1 = a では作り直し(『時間に依らない行 + 許容つき』) | 読み(0234m §2。実測は無い)。**cargo の層は 0.5 人日で済んだ**(I1、実測)ので、0234m の『試験 8〜9 日』のうち cargo・C の層は含まれていない |
| **I13** | CI の変更 | `ci/plan.sh`・`ci/objkeys.sh`(build の入力が変わる = 全オブジェクト key が 1 回変わる)、`ci/test-hybrid.sh`、`check_i1` の追加 | **0.5** | I4 | 冷間は約 20 分/CI 1 回(実測: gpu1) | 実測: 冷間 20 分。読み: 0.5 |
| **I14** | 製品形での再測定(足場でなく main の bundle): 5 分・音楽位置を揃えた比較・入力の T・P1 起動 | gpu1 の rig | **1**(Chrome 約 100 分、gpu1 0289 §11.3) | I2〜I8 | — | 読み(gpu1 の計画) |
| 合計 | | | **D1 = a 約 18、D1 = b 約 10.5**(I10 を除く。I10 を入れて +0.5〜1) | | | gpu1 の S2〜S6 は 17〜19(読み)。**今回の実測が効いたのは I1(0.5)と I2(0.25)**で、他は読み |

**依存順(最短の道)**: D1・D2・D3 を決める → I1 → I2〜I5 を 1 つの系列で(I2 → I3 → I4 → I5)。**I6(段階 C)は I2 の後ならいつでも並行できる**(クリティカルパス)。I7・I9 は I5 と同時。I8 は I6 の後。I12・I13 は I5 の後。I11 は I6・I8 の後。I14 が最後。クリティカルパスは **I1 → I2 → I3 → I4 → I5 → I12**(D1 = a で約 12 人日)と、**I2 → I6 → I8 → I11 → I14**(約 4.5 人日)で、前者が長い。

### 3.3 No-options 違反が出る箇所の一覧(実測。全行は `tables/0305-flag-and-env-sites.txt`)

| 種類 | 箇所 | 唯一の構成での扱い |
|---|---|---|
| **環境変数の選択肢(製品に入る)** | `getenv("FLE_WORKERS")` `fle_thread.c:339`(manifest の `engine.env.FLE_WORKERS` から)、`FLE_PROCESSORS` `fle_sys.c:764`、`FLE_WATCH_CS` `fle_sys.c:226`、`FLE_HOLD_REPORT_S` `fle_sys.c:313`、`FLE_DBG_REPORT_S` `fle_thread.c:761`(5 つ) | 定数(Worker 数 2〔D2〕、processors 1)、診断は削除 |
| **build の旗** | `-DFLE_GIL`(15 ファイル/44 行)、`-DFLE_TLS_MORE`(20 ファイル/31 行)、`GIL=1`・`MXTLS`(`build-engine-bundle.sh` 13 行)、`FLE_RING_POLL_HOOK`(3 ファイル/5 行) | 常時(wasm は `__EMSCRIPTEN__`)。`FLE_RING_POLL_HOOK` は試験の define で、既存の形 |
| **コード生成の旗** | `--hybrid-mx-tls`(fe-wasmgen の CLI。7 ファイル/21 行) | 常時にして引数を除く。wasm の基準が動く |
| **計測用 build の旗** | `FLE_GIL_STATS`・`FLE_CS_HOLD_STATS`・`FLE_HOLD_ATTR`(7 ファイル/30 行)、`SPIKE_CFLAGS` | 製品の木から外す(D3)。gpu1 の測定は別の patch |
| **実行時の分岐** | `rt_mode`/`RT_ON`(`fle_thread.c` 18 行) | D1 による |

### 3.4 Export(FL の File → Export → Wave file、オフライン描画)を実スレッドでどう扱うか

『export』は 2 つの意味があるので分けて書く。**(i) wasm の export 関数**(`fle_g3_*` など。ページが呼ぶ入口): I7。**(ii) FL の書き出し**(Ctrl+R → `TWAVRenderForm` → Start):

- **読み(0188・0193)**: Start の後、**レンダリングは 1 回のクリックの処理の中で UI スレッド(thread 0)が回して終わる**(Sampler.flp で 0.9〜1.6 s、WAV 353,904 B、sha256 `f5c6a08b1dede642f91026…`、ネイティブとバイト一致〔0193〕)。ミキサー専用のスレッドは再生していない。= 実スレッドでは **Worker 0 の中で完結**し、他の Worker の guest スレッドは待たれる側。GIL は Worker 0 が guest のコードを走らせている間 Worker 0 が持ち、待ち(`WaitForSingleObject` など)で手放す(0234 の設計、W0 優先)ので、Worker 1 のスレッドが必要な場面では譲られる。**構造上の変更は要らない**。
- **時間への依存**: WAV の内容は時計に依らない(0188: 開始の遅れ 300 ms と 2,300 ms で sha256 が同一、ディザの雑音も仮想時計に依らない)。= 実時計の実スレッドでも**同じ sha256 になるはず**(読み)。
- **危うい点(読み、未測定)**: (a) 長い曲の書き出しは、共有メモリの上限(glue の `maximum: 32768` page = 2 GiB、初期 640 MiB)に当たらないか。共有メモリの `memory.grow` は他の Worker の JS 側の `HEAPU8` を更新しない(wasm のコードは問題ない)ので、**Worker 1 の JS のライブラリ(`fle_*_lib.js`)が書き出しの最中に古い view を使わないか**は未確認。(b) 書き出しの最中の入力(Esc など)は、レンダリングの loop がメッセージを汲むときだけ届く(協調と同じ)。(c) 書き出しの速度(wasm、実スレッド)は 0 件(0188 が native -O2 だけ測った)。(d) 0193 が見つけた『出荷構成(Blend2D の swap)の Start で `BL_COMP_OP_OVERLAY` × PATH の未実装により停止』が E11 で解消済みかは**確認していない**(別件)。
- **置く試験(I11、1 人日)**: 実スレッド版で Sampler.flp の書き出しを 1 回走らせ、WAV の sha256 を 0193 の値と比べる(gpu1 の driver に 1 場面)。ディザなし。**一致すれば『実時計でも決定的』が実測になる**。

### 3.5 ミキサーを持たない Worker の初期化(読み。足場のコードの読み)

- **Worker 0(UI)**: 最初の Worker。`fle_worker_id_set(0)`。thread 0 を持つが**ミキサーのスレッドは持たない**(他の guest スレッドは全部 Worker 1)。`rt_mode` は `FLE_WORKERS ≥ 2` から決まる(I3 で定数化)。入力 ring の poll は Worker 0 だけ(I9)。idle のとき `Atomics.waitAsync(wake word)` で**非同期に**待つ(JSPI の中断。イベントループを回し、page のメッセージを受ける)。
- **Worker 1**: 最初の `CreateThread` で `wk_start(1)` → `pthread_create`(依頼は最初の Worker だけが出す。**親の最初の idle まで実際の起動が遅れる**: 0234 の Chrome の実測。P1 の 10 秒への影響は未測定)。`fle_worker_id_set(1)`。thread-local の register file(`__tls_base` 経由)・x87/MXCSR・scratch は Worker ごと。idle のときは `memory.atomic.wait32` で**ブロック**(自分のイベントループで他に処理する物が無い)。
- **Worker が guest スレッドを持たないとき**: `cur` は表の外の枠 `BOOTW(w) = FLE_THREAD_MAX + w`(Worker の idle の文脈)を指す。**表を `cur` で引く箇所は添字の検査が要る**: 足場は `switch_to` の通知、`fle_thread_off_ms`、`fle_thread_worker_of`、`wst[…]` の 4 か所で `idx < FLE_THREAD_MAX` を検査している。**§2 の (11) の『`cur < FLE_THREAD_MAX`』がこの初期化の表れ**。他に添字で引く箇所が無いかは、唯一の構成で全 `T[cur]` の呼びを洗う必要がある(I5。読み)。
- **音声(I6 の後)**: 段階 C でリングが wasm の共有メモリに載るので、**Worker ごとの JS 状態(`__fleAud`、`fleAudioSab`)は不要**。最初の Worker が 1 回だけ `postMessage('audio-ring', buffer, hdr, data, cap)` を送り、worklet の attached の語を立てる。Worker 1 は音声の JS を初期化しない(今の glue 補修〔lazy init〕が要らなくなる)。

## 4. (3) count/limit の B(0302 §7)の要否

**gpu1 の 5 分実測(`gonogo2-realthread-e11.md`、rt11、各腕 1 本)に、count の競合による問題は出ていないか**:

- **出ていない物**(実測、gpu1): 最も重い 30 s の区間でミキサー側 CPU/Unlock 最大 69 ms(基準 ≤ 85)、10 s の窓で最大 84 ms(際)。5 分の書き込み 0.938〜0.988(基準 ≥ 0.9)、12288 の割合 1〜6% で増えない(`tables/rt-arms-10s-rt11.md`: 12288 の割合は 0〜12%)、underrun 15 quanta・2 回(最長 9 quanta)。Worker の合計 0.97〜1.37 コア(基準 ≤ 1.5)。= **No-go の条件に当たらず、go の基準のほとんどを満たす**。
- **しかし count の競合が無いとは言えない**: (i) 5 分実測は count の A/B(TLS 版と大域版)を含まない。(ii) 実スレッドの CPU/Unlock は 10 s の窓で **24〜84 ms と大きく動き**、軽い窓は UI の present が 24/s に減る(35/s → 24/s)のと同時に起きている(gpu1 の読み)。UI の活動が多い窓でミキサーの CPU が増える相関は、(a) 同居 SwiftShader・SMT、(b) 音楽が重い、(c) UI の count の書き込みで line を奪われる、のどれでも説明でき、**この実測では分けられない**。(iii) 私の合成の測定(0302 §7、node、共有メモリ wasm、各 2,000 万回、1 run)では、**同じ count を 2 スレッドで更新すると 1 回 +6 ns(3.75 → 9.8〜10.1 ns)、別の line(TLS)なら +0**。
- **上限の見積り(読み、実エンジンで未測定)**: count は N1 の全体で 約 2,200 万/仮想 s、そのうち UI 側は約 5%(= 約 110 万/s)。UI の書き込み 1 回がミキサーの次の更新に line の取り直し(50〜70 ns)を起こすと、110 万/s × 60 ns ≈ 66 ms/s = ミキサー側の約 6.6%、**Unlock あたり 約 6 ms**(10.77 Unlock/s)。gpu1 の測定した実スレッドの税(協調に対し +18〜20%、音楽 90〜210 s の平均 59.8 対 50.5 = +9.3 ms/Unlock)の**約 2/3**。**『UI 側の率が 5%』は native N1 の slice の割合の読みで、ブラウザでの実測値ではない**。
- **結論**: **B は go/no-go を阻まないので先送りでよい**(基準を満たす窓が多い)。**ただし『競合が無い』ことの根拠にはならず**、上限が最終判定の際(重い窓 84 ms、基準 85 ms)に効く大きさ(6 ms = 7%)なので、**最終判定(音楽 0〜90 s の Unlock)で重い区間が基準を超えそうなら、最初に試す梃子**にする。**安いやり方**: node の実スレッド(`node-bundle-run-rt.mjs`、SwiftShader が無い)で count を TLS にした wasm と大域の wasm を同じ音楽位置で Worker 1 の CPU/Unlock を比べる(wasm の build は fe-wasmgen の変更 + link の約 13 分、Chrome 不要)。この A/B は私が受けられる(I10 の 0.5〜1 人日の前半)。

## 5. 方針との適合(必須の記載)

- **lift の忠実性**: 今回の直しは、(i) 足場の誤りの除去(表の途中への挿入〔決定 0015 の違反〕、製品の環境変数、診断)と、(ii) 試験の追従(規約どおりの `rows_0234`、決定の中身に合わせた検査、stub)。FL の挙動の補正・代行・class/アドレスの分岐は無い。(iii) 新しい処理は足していない。
- **no-options**: 変更表(§3.3)が旗・環境変数・コード生成の旗の一覧。D1〜D3 は PM の判断。
- **進捗の誠実性**: 実エンジンの実行は 0。数字は実測(箇所の数、試験の結果)と読み(人日)を分けた。

## 6. 自己申告(0305 の区間)

- 最初の仮説『足場が行を足した → 固定値のずれ(期待値の更新)』(0301 §3b、私自身の読み)は、**10 本中 7 本で誤り**だった(足場の誤り)。試験の先頭の注釈(decision 0015: append only)を読まずに、メッセージの数のずれだけから読んでいた。この是正で、0301 の『10 試験: 期待値の更新』は『7 本は足場の誤り、3 本が期待値の更新』に直る。
- `rows_0234` の引き算は、規約(`rows_0267/mod.rs` の先頭が名指す 7 ファイル)に従って 7 ファイルに一括で入れた。**後から実測で確かめた(第 2 報): 表の末尾に足した状態で、wall_fr4・wall_fr5・wall_fr19・wall_fr20 の 4 本は `rows_0234` の引き算なしでは落ちる**(wall_fr4.rs:199、wall_fr5.rs:178、wall_fr19.rs:255、wall_fr20.rs:176。`tables/0305-final-checks.txt`)。= 7 ファイルの更新はすべて必要で、3 本だけでは足りない(第 1 報で『7 ファイル』と書いたのは正しく、『3 本だけが期待値の更新』という書き方は、直接落ちる試験が 3 本という意味だった)。
- 重い job を始めた後に go/no-go #2 の Chrome(gpu1 の sdriveg2)が走り始め、PM から連絡を受けた。走っていた `test.sh full` は nice 19 に落とした(以後、始める前に確認する)。
- `sleep`・末尾 `&` は使っていない。bash ツールの誤呼びが 1 回(実際は shell で実行された)。
- `~/work/mt3w-prof303`、`mt3w-n303`、`mt3w-bundle93` を PM の許可を受けて削除した(readlink -f で確認)。

## 7. 再現・成果物

- `impl/0305-fix-the-16-failures-on-rt-on-e11.diff`(`rt-on-e11.diff` を当てた木の上に当てる。13 ファイル)。
- `tables/0305-triage-16.tsv`、`0305-failing-messages-from-0301.txt`、`0305-experiment-1-rows-moved-to-the-end.txt`、`0305-experiment-2-all-fixes.txt`、`0305-full-test-with-fixes.log`、`0305-flag-and-env-sites.txt`、`0305-dry-run-rt-on-e11-on-main-25019fd1.log`。


## PM の判断と取り込み(2026-11-06 19:45、docs のみ。製品の差分なし)

取り込み: mt3 の最終(0305・0305b・0305c・0305d、19:43)の文書・表・道具を `docs/investigations/mt3w/0305/`、差分 2 本を `docs/investigations/mt3w/patches/` に置いた。実行は fe-lift の cargo 試験・patch の dry-run・grep だけで、**Chrome・wasm の実行・依頼者の機械は 0 件**。人日は読み。

事実として受け取る:
1. 実スレッドの足場(rt-on-e11.diff)の test.sh full の 16 失敗は、足場の誤り 10・試験側の更新 6・E11 側の本物の問題 0。直した木は 222 results・2,421 passed・0 failed。足場が import 表の途中に 2 行を挿入していた(決定 0015 の『追加は末尾』に反する)ことが 7 本の原因。
2. 旗・環境変数は 40 ファイル 163 行。『実スレッドを唯一の構成にする』工数は D1 = (a')(製品は実スレッド・実時計だけ、協調+仮想時計は test tree の複製だけ)で約 12.5 人日、N 本の Worker への一般化(I15)約 8、T の梃子 2〜4。合計 約 22.5〜24.5 人日(クリティカルパス 16〜20)。
3. **PM が自分で確かめた E13(main 91400572 + 0302 の Dwm + 0284b-gp-extension、commit aaecdfa8)**: 既定の FLEngine の C は 215,740,699 B・sha256 e798422c…(不変)、shellcheck と ci の 6 試験 rc=0、test.sh full 219 results・2,417 passed・0 failed・SKIPPED 1(wall_fr9t の任意の比較)、a4 は units 942・runnable 895/895・distinct 5,310・entries 1,009,737・started_but_not_entered 0。mt3 の run 3 と一致。

判断:
1. **E13 = 0302(Dwm)+ 0284b-gp-extension だけ**。hi-check-form(+約 241 KB、効き −1.8〜−2.9%)は入れない(効きが小さく、基準を動かす理由にならない)。E13 の push は検証が通った今、deploy CI が動いていない時に行う。以後 E13 の CI(約 60 分)の間は main に何も push しない(docs も)。
2. **着手順は mt3 の S0〜S6 を採る**(受け入れ条件を早く測れる順)。S0(足場 + 16 件の修正)と S1(段階 C の最短の形: リングと wr・rdHi を共有メモリへ。Worker ごとの JS 状態は製品の形にしない)は、使い捨ての木で行い、**中間 bundle B1 は main に入れず gpu1 が局所で測る**(測るのは T・UI Worker・重い区間・5 分の書き込み・PCM の零)。旗の除去(S4)は B1〜B3 が受け入れ条件(T が協調 +2 ms 以内、UI Worker ≤ 0.6 core、重い区間 CPU/Unlock ≤ 85 ms、5 分の書き込み ≥ 0.9、12288 の割合が増えない、2 Worker ≤ 1.5 core)を満たしてから。main へのデプロイは E13 → E14(present の別 Worker P、gpu1 の決定 0311)→ 実スレッドの本命(S4 以降)の順。P と段階 C はともに app/ の engine 周辺を触るので、後から入る側が衝突を解く。
3. **D1 = (a')、D2 = host の論理 CPU 数 n から Worker 数を決める(上下限は選択として明記し、後で測って決める)、D3 = 計測用 build は製品の木から外してよい(patch -R を gpu1 の計測用に残す)を、S0〜S3 の計画として確定する**。ただし旗の除去(S4)と協調の撤去は、受け入れ条件の測定が通った後の判断(go の最終確認)。
4. 担当: mt3 = 実装(S0〜S5)、gpu1 = 各中間 bundle の測定と T の B/C の内訳、present の別 Worker P(決定 0311)、alpha1 = G9-a(Fruity Balance。ただし Wine の差の切り分け〔mixer track 別 Export〕の結果を見てから)、opt1 = Wine の参照(FL が n に応じて作るスレッド数)と WAV の差、ci1 = E13・E14 のデプロイ後の確認。
5. 後片付け: mt3w-main305(851 MB)は削除してよい。mt3w-e13/ct は E13 のデプロイ前の再確認が済むまで残す。未確認: E14 候補(E13 + 足場 + 16 件の修正 + D3a)の test.sh full と a4(cargo test --no-run は rc=0)、wasm の実行での Export、Firefox、各項目の実績。
