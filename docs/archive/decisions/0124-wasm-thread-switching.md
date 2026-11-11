# 0124: 協調スレッド（決定 0122）の wasm での切り替え（S3）。JSPI（スレッド 1 本 = 別の `WebAssembly.promising` 呼び出し）+ wasm シャドウスタックの退避・復帰。すべて仮の決定

状態: **仮の決定**。依頼は PM（決定 0124 の内容は PM の仮の決定。番号 0124 は PM の仮の割り当て）。測定・実装・テスト・文書は作業者が行った。**作業は HEAD 8c8131b の `git archive`（`lift/`、`docs/`、`tools/`）で始め、clean な写しを `clean/` に置いた。測定はすべて 8c8131b + 差分（`s3.diff`）の木のもの。** 差分は `clean/` に `patch -p1` で当たる（`MERGE-NOTES.md`）。**実機 Windows との照合は 0 件。実エンジンのスレッド本体（QFC の `TTimerThread`、FLEngine の別のスレッド）は一度も走っていない（`CreateFruityInstance` は壁 22 で止まる。決定 0122、別作業の決定 0123）。Firefox・Safari は未測定。**

区分の凡例: **実測**＝コマンドの出力を見た事実。**読み**＝コードや出力からの解釈（確かめていない）。**未確認**。**出典なしの選択**＝ページにも実測にも根拠が無く、作業者または PM が選んだ値（撤回条件つき）。

## 先に結論

| 項目 | 内容 |
|---|---|
| JSPI でのスレッド切り替えは成立したか | **成立した**（止まる条件 6 には当たらない）。最小の実験（`toy/`）: 3 スレッドが各 20 段の再帰で毎段 yield、局所変数の破損 0、hand-over 126 回。同じ C 関数の `setjmp`〜`longjmp` の間で他スレッドへ 2 回切り替え（`SJLJ=wasm`）して 3 本とも自分のフレームに着地。スレッドの終了（return と、深い所からの JS throw）も成立 |
| native と wasm のイベント列の一致 | スタブのゲスト関数 + 実ランタイムの**36 シナリオ**（通常 33、名前付き停止 3）で、native（`swapcontext`）と **node** の stdout・stderr（スケジューラの切り替えトレース）が **36/36 一致**（独立 2 回、wasm 出力は 2 回間でバイト一致）。**headless Chrome 156 でも 36/36 一致**（独立 2 回）。通常 33 件は stdout・stderr がバイト一致。停止 3 件は `rsp 0x…`（ホストのアドレス。native でも実行ごとに違う）を `rsp X` に置き換え、wasm の abort 報告の行を除いて比較（比較を弱めた。「解釈が分かれた点」） |
| スタック | **JSPI のスタックの上限は「スレッドごと」**（全体ではない）。1・2・4・8・16 本が同時に最大の深さにいても各スレッドが同じ深さまで届く（984 kB で 8,042 段、150 kB で 1,283〜1,284 段、145 kB で 1,249 段。node と Chrome で最大 1 段違い）。超えたスレッドだけが `RangeError` で失敗し、スレッド番号つきで報告される。**toy のフレームでの値**。実エンジンの深さは未測定。別に、wasm のシャドウスタック領域（非先頭スレッドごとに 1 MiB。出典なしの選択）が上限で、超過は切り替え時の検査で名前付き停止（ただし超過の書き込みは防げない）。メモリ: 1 スレッドあたり約 1,091 kB（RSS。ゲストの 1 MiB のスタックが主と読む） |
| 切り替えのコスト（回/秒。1 回 = 実行スレッドの 1 回の交代） | node（wasm、最終のソースで 5 回の中央値。Chrome だけは自己検査の前のビルドで各 1 回）: ping 3.04 M、yield 5.02 M、sleep0 5.12 M、timer 型 1.73 M、idle 3.52 M。native（swapcontext、-O0）: 1.78 M、3.06 M、3.11 M、1.90 M、2.72 M。**native は 1 切り替えあたり `rt_sigprocmask` + `gettid` の 2 システムコールがあり、「JSPI が速い」とは言えない**。Chrome 156: 1.0〜1.9 M/秒（各 1 回測定）で node の 1.7〜3.1 倍遅い（理由は未調査）。60 Hz の timer 型は 1 フレームに 2 回で足りる |
| a4 の native/wasm の再確認 | FRUITY 無効: **942/942、895/895、`returned from entry`、functions 5,308、entries 1,009,189**（決定 0122 の基準どおり）。native 独立 2 回、wasm（`THREADS=1`、最終のソース）独立 2 回、wasm（`THREADS` なし = clean とバイト一致）2 回。stdout は native と wasm で同一（`calling entry` の行とシード一覧のパスの行を除く）。**native の SHA-256 は変わった: 09528ebc… → 5968c889…**。原因は新規ファイル `fle_thread_wasm.c`（native では空の翻訳単位）が glob で入り `.o` がリンクされること（実測。それを除いて再リンクすると 09528ebc… に完全一致）。「PM の判断点」 |

## PM の決定（0124、仮。変えていない）と、この作業での扱い

1. **規則は決定 0122 のまま**: スケジューラ（作成順の巡回、優先度無視、切り替えは待ち／譲りの API のみ、全員待ちで仮想時計を最小の期限へ、期限なし全員待ちはデッドロック停止）には**触れていない**。`fle_thread.c` の差は切り替えの手段のフックだけ（下）。native と wasm の同一性は 36 シナリオで測った。
2. **方式**: 推奨のとおり。スレッド 1 本 = 別の `WebAssembly.promising` 呼び出し（別の JSPI スタック）。待ち・譲りは `WebAssembly.Suspending` の import（`fle_wasm_thread_switch`）で呼び手を中断する。スケジューラの判断は C 側（`fle_thread.c` の `schedule()`）で済み、JS は「次に走らせる添字」を受けて起動・再開するだけ（方針を持たない。テストで固定）。`__stack_pointer` は切り替えごとに退避・復帰する。**推奨と違った点は 1 つ**（hand-over を microtask に積む理由。下「出典なしの選択」と「解釈が分かれた点」）。
3. **追加ファイル**: `lift/runtime/fle_thread_wasm.c`／`.h`、`lift/harness/web/threads/`（JS の転送、node と Chrome の実行器、36 シナリオの C、比較・スタック測定・net-log 検査）、`tests/wall_fr9t.rs`。`fle_thread.c` には切り替えの手段を差し替えるフックだけ（`#ifdef FLE_THREAD_WASM`／`__EMSCRIPTEN__` の条件つき。約 50 行）。`.h` は変えていない。**native の `fle_thread.o` は clean と cmp 一致、`-DFLE_THREAD_WASM` なしの emcc の `fle_thread.o` も clean と cmp 一致**（実測）。
4. **検証の範囲**: (i) 同一性: 36 シナリオ（native と node）。(ii) headless Chrome。(iii) スタック。(iv) コスト。(v) 実エンジン 6 モジュール wasm の a4。いずれも実測（上の表）。(ii) の**ネットワーク遮断は完全ではない**（下「検証」）。
5. **実エンジンのスレッド本体は走らせていない**。QFC の `TTimerThread`（`DwmFlush` + `Sleep` + 窓へ `PostMessageW` + 購読者の呼び出し）と FLEngine の `TLayeredControlThread`（自スレッドで窓を作る + `GetMessageW` ループ）と**同じ形のスタブのスレッド**でシナリオ `timer_thread`・`layered_thread` を組んだ。形は `thread-model.md` §4〜5 の読みに基づくもので、実エンジンの本体と同じ命令列ではない。
6. **止まる条件**: 当たらなかった（成立した）。

## 方式（実装した形）

| 要素 | 内容 |
|---|---|
| C（`fle_thread.c`、共有） | `schedule()`・`pick_other()`・`poll_waiters()`・仮想時計は不変。`switch_to()` は `save_ctx`／`cur = to`／`load_ctx`／`set_cur_pointers` まで同じで、最後の `swapcontext` だけを `#ifdef FLE_THREAD_WASM` で `fle_thread_wasm_transfer(from, to, from_done)` に差し替え。`CreateThread` は `FLE_THREAD_WASM` のとき `mmap`・`makecontext` の代わりに `fle_thread_wasm_stack_alloc()`。`thread_main`／`thread_finish` は共有（`ExitThread` も同じ）。`FLE_THREAD_WASM` なしの emscripten は従来どおり最初の `CreateThread` で名前付き停止 |
| C（`fle_thread_wasm.c`） | 非先頭スレッドごとに wasm のシャドウスタック領域（`malloc`、既定 1 MiB、低位に canary 32 B）。`fle_thread_wasm_transfer`: ① `emscripten_stack_get_current()` を保存 ② 離れるスレッドの canary と領域内のポインタを検査（違えば名前付き停止）③ 終了したスレッドは `fle_wasm_thread_exit`（JS の throw で JSPI スタックを巻き戻して終了）④ それ以外は Suspending の `fle_wasm_thread_switch` で中断 ⑤ 再開後、`__stack_pointer` が保存した値に戻っていなければ名前付き停止（自己検査）。新スレッドの本体は export `fle_wasm_thread_entry(idx)`（JS が `WebAssembly.promising` で起動） |
| JS（`fle_thread_lib.js`、emscripten の `--js-library`。node とブラウザで同じファイル） | `fle_wasm_thread_switch(from, to, spNow, spNew)`: `sp[from]` を記録し、`from` の resolver を保存した Promise を返し、`to` への hand-over を `queueMicrotask` に積む。hand-over: 新スレッドなら `stackRestore(spNew)` してから `promising(entry)(to)`、中断中のスレッドなら `stackRestore(sp[to])` してから resolver を呼ぶ。終了したスレッドの Promise は reject され、その handler が次への hand-over をする。失敗（trap、`RangeError`、abort）はスレッド番号つきで報告。**方針（優先度、期限、ready の判定、時計）を持たない** |
| ビルド | `build_nmod_wasm.sh` に `THREADS=1`（`JSPI=1` と `SJLJ=wasm` が前提）。runtime を `-DFLE_THREAD_WASM` でコンパイルし、リンクに `--js-library` と export `fle_wasm_thread_entry`。**未指定なら従来と同じコマンド**（`THREADS` なしの wasm は clean とバイト一致: 実測） |
| 実行 | node: `--experimental-wasm-jspi`。Chrome 156: フラグなし（JSPI は既定で有効）。`SJLJ=wasm` が前提（`jspi-real-wasm.md` の F1・F2）。スレッドごとの `setjmp` 着地は、同じ C 関数を 4 本が同時に着地の間にいて各自のフレームに戻る（シナリオ `landing`）で成立した |

## 実測

### 同一性（36 シナリオ）

`thread_scenarios.c` は **1 つの C のソース**を native（clang）と wasm（emcc）の両方でビルドする。実ランタイム（`fle_thread.c`、`fle_win.c`、`fle_event.c`、`fle_sys.c`、`fle_imports.c`、`fle_proc.c`、`fle_callback.c` ほか 18 ファイル）をリンクし、ゲスト関数は C のスタブ。内容: 決定 0122 のテスト（`wall_fr8.rs`）の素材（作成・フラグ・中断カウント・巡回・仮想時計・`SwitchToThread`・待ち・auto-reset イベント・CS・`ExitThread`・TLS・last error・レジスタ・`gs:`・コールバック深さ・ロケール・`PostMessageW`/`GetMessageW`/`WaitMessage`/`MsgWait`/`SendMessageW`）、`landing`、QFC 型 `timer_thread`、FLEngine 型 `layered_thread`、`spawn`（スレッドがスレッドを作る）、`many_threads`（40 本、`Sleep(0)`・`Sleep(n)`・`SwitchToThread` を混ぜる）、`dwm_flush`、`ping_pong`（2,000 往復 = stderr 4,003 行）、`deep_locals`、`stack_churn`、停止 3（`stop_deadlock`、`stop_bad_start`、`stop_in_thread`）。比較は stdout と stderr（`fle-thread: create/switch/start/exit … clock=` のトレース）。

| 実行 | 結果（実測） |
|---|---|
| native 対 node、初回（34 シナリオ） | 通常 31 件は**無修正で初回から一致**。停止 3 件は初回は不一致（比較側の問題。下） |
| native 対 node、最終ソース、独立 2 回 | 36/36 一致、各回 0 件の LEAK（JS スケジューラの `starts == ends`）。2 回の wasm 出力はバイト一致 |
| native 対 headless Chrome 156、最終ソース、独立 2 回 | 36/36 一致。`starts == ends` を通常 33 件で確認 |
| 比較の強さ | 通常 33 件: stdout・stderr がバイト一致。停止 3 件: `rsp 0x…` の値（ホストのアドレス。native でも 2 回で違う: 実測）を `rsp X` に置換、wasm の abort の報告行（`Aborted(…)`、`[js] fle thread N failed`、`at …`）を除外、終了コードは native 134・wasm 99 で「両方非 0」だけ。**この正規化は不一致を見た後に足した**。 |

### スタック（node 24 と Chrome 156、toy のフレーム）

**測定のビルド（時刻はファイルの更新時刻から。ビルドごとのソースの記録は取っていない）**: 再開後の `__stack_pointer` の自己検査（W4）を足したのは 14:29。スタックの二分探索（node と Chrome）は 13:49〜14:01 の**自己検査なし**のビルドで取った。`parked N` のスレッド数とメモリの表は 14:34 で、14:32 に作った `scen/wasm-node`（自己検査**あり**）と、14:34 に作った領域サイズ違いの 2 ビルド（自己検査あり）で取った。切り替えのコストの Chrome は 14:01 までの（自己検査なし）ビルドで各 1 回、node と native は最終のソースで 5 回。自己検査は切り替えごとに `emscripten_stack_get_current()` を 1 回読むだけで、深さの上限に効く経路ではないと読むが、**最終のソースでスタックの二分探索と Chrome のコストをやり直してはいない**（最終のソースで再測定したのは、node と native のコスト、36 シナリオ（node と Chrome、各 2 回）、a4）。

`deep <深さ> <スレッド数> <マスク> <フレーム>`: マスクの立ったスレッドが指定の深さまで再帰し、一番下で `SwitchToThread`（全員が同時に最大の深さにいる）。二分探索。V8 のフラグ `--wasm-stack-switching-stack-size`。

| JSPI スタック | 深い 1 本（スレッド 0/1/2） | 1/2/4/8/16 本が同時に最大 | 1 段超過 |
|---|---|---|---|
| 984 kB（既定） | 8044 / 8042 / 8042 | 8047 / 8042 / 8042 / 8042 / 8042 | そのスレッドだけ失敗、番号つきで報告 |
| 300 kB | 2548 / 2546 / 2546 | 2552 / 2546 / 2546 / 2546 / 2546 | 同 |
| 150 kB | 1285 / 1284 / 1284 | 1289 / 1284 / 1284 / 1284 / 1284 | 同 |
| 145 kB | 1251 / 1249 / 1249 | 1254 / 1249 / 1249 / 1249 / 1249 | 同 |

Chrome は同じ形で、150 kB のスレッド 1・2 が 1,283（node 1,284）、他は同じ。**読み**: 上限は JSPI の各スタックに別々に効き、スレッド数で下がらない。深さは大きさに比例する（約 125 B / wasm フレーム。toy）。**実エンジンの 145〜150 kB の境界（`jspi-real-wasm.md` §8。a4 の経路の 68 フレーム）が各スレッドに別々に効くかは、実エンジンのスレッド本体が走らないため未測定。toy のフレームでは別々に効く**。

- wasm シャドウスタック領域（既定 1 MiB）: 1,500 段（約 773 kB）可、3,000 段（約 1.5 MB）は名前付き停止 `the shadow stack pointer of thread 1 (…) is outside its region […] (at a switch away from the thread)`（`check_overflow.sh`）。**限界（読み。停止時のポインタが領域の下にあったことと整合）**: 検査は切り替え時にだけ走るので、領域を溢れたスレッドは検出の前に領域の下の別のメモリへ書く。検出であって防止ではない。wasm の線形メモリにはガードページが作れない。
- スレッド数とメモリ（`parked N`: N 本が同時に `WaitForSingleObject` で中断）: 0〜1 本 56〜58 MB、16 本 72 MB、64 本 126 MB、128 本 196 MB、250 本 328 MB（RSS）。**1 本あたり約 1,091 kB**。JSPI スタックを 984/150/32 kB にしても、シャドウ領域を 64 KiB/1 MiB/4 MiB にしても変わらない（1,091〜1,093 kB）。**読み**（5 つの値と整合するが単独の実験では分けていない）: ゲストの 1 MiB のスタック（決定 0122）と TEB・TLS・FLS の表が主で、JSPI スタックとシャドウ領域は触れていないページが RSS に入らない。**RSS は JSPI スタックのコストを測っていない**。250 本が同時に中断して全部終了できた（`FLE_THREAD_MAX` は先頭を含め 256）。

### 切り替えのコスト

`bench <種類> <n>`。「切り替え」= 実行スレッドの 1 回の交代（回数はループの構造から計算。wasm では JS スケジューラ自身の数 `switches` と 1 以内で一致: ping 399,999 対 400,000 など）。時間はプログラム内の時計（native: `CLOCK_MONOTONIC`、wasm: 同じ呼び出し）。node と native は 5 回の中央値。

| bench（何が切り替わるか） | native（swapcontext、-O0） | wasm node 24 | wasm Chrome 156（隔離なし / あり。各 1 回） |
|---|---|---|---|
| ping: 2 スレッドが auto-reset イベント 2 つで交互（40 万回） | 1.78 M/秒（0.562 µs） | 3.04 M（0.329 µs） | 1.30 M / 1.07 M |
| yield: 3 スレッドが `SwitchToThread`（60 万回） | 3.06 M（0.327 µs） | 5.02 M（0.199 µs） | 1.65 M / 1.74 M |
| sleep0: 3 スレッドが `Sleep(0)`（60 万回） | 3.11 M（0.321 µs） | 5.12 M（0.196 µs） | 1.84 M / 1.91 M |
| timer: `TTimerThread` 型（`DwmFlush` + `Sleep(2)` + `PostMessageW`、主は `GetMessageW`。4 万回） | 1.90 M（0.526 µs） | 1.73 M（0.577 µs） | 1.02 M / 1.02 M |
| idle: `Sleep(1)` のループ（仮想時計が飛ぶ。10 万回） | 2.72 M（0.368 µs） | 3.52 M（0.284 µs） | 1.58 M / 1.56 M |

読み方（正直に）: (1) 値にはスケジューラ自身（`poll_waiters`、`save_ctx`／`load_ctx`: R・X・YH・MM・x87 などのコピー）が入る。(2) native の `bench yield 20000` を `strace -c` にかけると、60,000 回の切り替えで `rt_sigprocmask` 60,005 回と `gettid` 60,012 回（実測）。つまり 1 切り替えあたり 2 回のシステムコール。`rt_sigprocmask` は glibc の `swapcontext` がシグナルマスクを保存・復帰するため（読み。glibc のソースは見ていない）。`gettid` の呼び手は見ていない（読み: `fle_host_tid()` を使う単一スレッドの所有者検査）。strace なしでそれが native の 0.3〜0.5 µs のどれだけかは**測っていない**。したがって**この表は JSPI が swapcontext より速いことを示さない**: 2 つの特定のビルドの比較で、native 側には 2 回のシステムコールと -O0 のオブジェクトがある。(3) wasm 側も -O0 の C を emcc が作り、V8 が最適化する。Chrome は同じ wasm で node の 1.7〜3.1 倍遅い（理由は未調査）。(4) Chrome の時計の刻みは 100 µs（隔離ありで 5 µs）。実行は 40〜370 ms なので量子化は 0.3 % 未満だが、Chrome は各 1 回の測定で分布ではない。

## 出典なしの選択（撤回条件つき）

| # | 選択 | 撤回条件 |
|---|---|---|
| W1 | **非先頭スレッドの wasm シャドウスタック領域は 1 MiB**（`FLE_THREAD_WASM_CSTACK_BYTES`。canary は低位 32 B）。1 スレッドのゲスト呼び出しの C フレームがこれに収まる根拠は無い（実エンジンの深さは未測定）。native の C スタックは 16 MiB（決定 0122 S4）で、それとの対応も無い | 実エンジンのスレッド本体の C フレームが超えたとき（名前付き停止が出る） |
| W2 | **hand-over を `queueMicrotask` に積む**（呼び手の import が先に Promise を返して中断してから次を起動）。設計メモの当初の理由（import の中で次を起動すると、中断していない呼び手を次が再開できない）は**実測で支持されなかった**: hand-over を同期にした変異（M2）は 36 シナリオと 250 本の同時中断で通った。microtask は保守的な順序として残したが、**必要であることは示していない** | 同期の hand-over で壊れる例が見つかったとき（microtask が必須と分かる）。または microtask が遅さの原因と分かったとき |
| W3 | **終了したスレッドは中断せずに JS の throw で JSPI スタックを巻き戻す**（`fle_wasm_thread_exit`）。中断したままにすると JSPI スタックが残る（変異 M7: 出力は同一のまま `starts 2 / ends 0`） | JSPI の仕様が変わり、throw で巻き戻せなくなったとき |
| W4 | 再開後の `__stack_pointer` の自己検査（保存した値と違えば名前付き停止）を**既定で有効**にした（`FLE_THREAD_WASM_NO_SPCHECK` で切れる） | 実エンジンで誤検出が出たとき |
| W5 | `THREADS=1` は `-DFLE_THREAD_WASM` の**オプトイン**（既定の emscripten は従来どおり最初の `CreateThread` で停止）。PM の項目 3（「`makecontext` 経路を既定のまま」）の解釈 | PM が wasm のスレッドを既定にすると決めたとき |
| W6 | ブラウザの隔離: `--host-resolver-rules="MAP * 127.0.0.1:9, EXCLUDE 127.0.0.1"`、`--no-proxy-server`、`--disable-ipv6`、`--disable-quic`、バックグラウンド機能・Safe Browsing 等の無効化、新しいブラウザコンテキスト、使い捨てプロファイル（`lift/out-s3` の下） | PM が別の遮断（ネットワーク名前空間など）を用意したとき |

## テスト

`tests/wall_fr9t.rs`（**20 件**）。native でビルドしたシナリオのプログラム（wasm と同じ C のソース）の出力を、手計算の期待値と比べる 12 件、フックが native を変えないことの 3 件、wasm だけが走らせるコードのソース検査 5 件。**期待値の出所**: `round_robin`、`clock_order`、`flags0`、`ids`/`suspended`/`suspend_count`、`dwm_flush` の境界、timer 型の 60 Hz の刻み（`ceil(k·10^7/60)` + 2 ms を手で再計算）、`sum 780` は決定 0122 の規則からの手計算。**一方、`landing`・`spawn`・`layered_thread` の順序、`send result 9003`、`ping_pong` の stderr 行数 4003 は、native の出力を見た後に書いた**ので独立の根拠ではなく、現在の挙動の固定（切り替えの手段が変わったときに出る）。`many_threads` の決定性の検査は同じバイナリの 2 回の比較で、規則の変更は検出せず非決定性だけを検出する。ソース検査は文字列を読むだけで弱い（wasm の実走行は node と Chrome の実行器が本体）。

実行器（cargo テストではない）: `compare_scen.sh`（native 対 node。leak 検査つき）、`run_chrome_scen.mjs`（Chrome。net-log 検査つき）、`measure_stack.mjs`、`check_overflow.sh`、`check_netlog.mjs`。

## 検証（実測）

- **a4**（上の表。最終のソースで、独立 2 回ずつ）: native `ofinal1`・`ofinal2`（再生成・再ビルド、各 54 秒）、wasm（`THREADS=1`、389,502,138 B、`33d17248…`）node 2 回、wasm（`THREADS` なし = clean と同じ 389,497,664 B、`87037fa1…`）node 2 回。すべて終了 0、stderr 0 B、942/942、895/895、functions 5,308、entries 1,009,189。native の runtime の `fle_*.o`（基準の 47 個）、ハーネスの `.o`（4）、生成コードの `.o`（全部）は clean の基準とバイト一致で、**差は新規の `fle_thread_wasm.o`（最終は 48 個）が増えただけ**。それを除いて再リンクすると基準の SHA `09528ebc…` に完全一致（実測）。
- **変異検査**（木のコピー。検出は「変異した木から wasm のシナリオを作り直し、変異していない native の出力と比べる」か cargo テスト）。実行 12 回、変更は 8 種。**最終のコードで検出**: 巡回を逆回り（9/36 不一致）、離れるスレッドの `__stack_pointer` を保存しない（33/36）、新スレッドが作り手の `__stack_pointer` のまま（19/36）、領域の検査を切る（`check_overflow.sh`）、終了したスレッドが中断する（33/36。**stdout・stderr は同一で、leak 計数でだけ検出**）、native の `#error` を外す（強めたテストで検出）、中断中のスレッドの再開で `__stack_pointer` を戻さない（**自己検査を足した後にだけ検出**）。**検出しなかった**: hand-over の同期化（M2。有害でないことが分かった）。**最初の実行で生き残った 3 件と、その対処（検出器を後から足した）**: (a) M1（再開時の復帰なし）はシナリオ 34 件で生存 → `deep_locals`（12 段）を足しても生存 → `stack_churn`（スレッドごと・反復ごとに深さを変える 1,452 回の検査）を足しても生存（自己検査を `-DFLE_THREAD_WASM_NO_SPCHECK` で切って測った）→ **再開後の `__stack_pointer` の自己検査**で初めて検出。toy の対照が壊れたのは**両方**の復帰を外したためで、再開時だけを外すと、各スレッドは自分の領域で始まり、再開後は直前に走ったスレッドの空き領域に書くので、特定の交代の順でしか重なりを起こさない（読み）。(b) M7: leak 計数を足して検出。(c) M8（native の `#error`）: テストが `<emscripten.h>` の不在でも通る弱さだったので、正確なメッセージを検査するよう強めて検出。
- **関連テスト**（木のコピーで実行。`cargo test --workspace` は走らせていない）: `wall_fr9t` 20、`wall_fr8` 45、`tid` 7、`setjmp_landing` 8、`wasm_port` 6、`event` 16、`modules` 16、`window` 36 を独立 2 回（各回 5 分前後）で全通過。`callback` 18、`deferred` 15、`imports_table` 1、`load` 22、`locale` 32、`proc` 18、`realmod` 16、`sys` 19、`vcall_thunk_dll` 2、`wall_fr4` 24、`wall_fr5` 14、`wall_fr6` 33、`wall_fr7` 18 を 1 回（11 分 32 秒）で全通過。**最初の 1 回で `wall_fr4` の 1 件が失敗**: 私の環境変数 `VFS_SEED_ROOT` を `cargo test` のシェルに export していたため（`a4-multi.sh` が `VFS_SEED_ROOT` だけの指定を拒む）。コードの問題ではなく、環境を直して再実行して通った。
- **headless Chrome の隔離（PM の要求 =「127.0.0.1 以外への接続 0 件」）: 達成できなかった**。net-log の検査（`check_netlog.mjs`）で、**TCP 接続は 127.0.0.1 だけ（非ループバック 0）**。しかし **UDP の `connect()` が 1〜2 回 `[2001:4860:4860::8888]:443`（Google の公開 DNS の IPv6）に残った**。ホストリゾルバの IPv6 到達性チェック（`HOST_RESOLVER_MANAGER_IPV6_REACHABILITY_CHECK`）と読む（ソケットの出所の連鎖は `UDP_SOCKET` ← `UDP_CLIENT_SOCKET` ← `NETWORK_SERVICE_HOST_RESOLVER`）。**その UDP ソケットに `UDP_BYTES_SENT` は無い**（`connect()` は経路を選ぶだけでパケットを送らないと読む）。試して消えなかったもの: `--disable-ipv6`、`--disable-quic`、`--enable-features=EnableIPv6ReachabilityOverride`、Local State の `net.ipv6_reachability_override_enabled`、Safe Browsing と HPRT の無効化（www.gstatic.com への Chrome 自身の要求は消えたが、この UDP は残った）。ネットワーク名前空間は使えない（`unshare -rn` が許可されない）。Chrome 自身が起こす別のホスト（accounts.google.com、www.google.com、update.googleapis.com）への URL 要求は、リゾルバの規則で 127.0.0.1:9（何も listen していない）に写されて全部 net error −102（拒否）で終わる。runner に `NETLOG_TOLERATE_IPV6_PROBE=1` があり、**Chrome の全結果はこのオプションで実行し、出力行に「TOLERATED by option, not removed」と書いてある**。ブラウザのプロファイルは `lift/out-s3` の下の使い捨てで、依頼者のプロファイルには触れていない（ただし Chrome が `~/.config/google-chrome/Crash Reports/` を更新しうることは決定 jspi-real-wasm.md と同じ。今回は `--crash-dumps-dir` を使い捨てプロファイルの下に指定し、`--disable-breakpad` も付けた。更新が無かったかは確認していない）。
- 最初の Chrome の実行は 2 つの失敗があった（記録）: `window.runScen` が関数でない（私の `--proxy-bypass-list` が暗黙の loopback の除外を外し、ページ自体がプロキシに行って死んでいた。net-log の `net_error −2/−130` から読んだ）。シナリオの比較は有効な隔離の実行（上）でやり直した。

## 止まった理由

止まっていない（条件 6 に当たらなかった）。JSPI の制約（中断中のスタックから別のスタックを再開できない、`promising` の入れ子が使えない）は**観測されなかった**。代替案（T3 = Worker + `Atomics.wait` + 共有メモリ、Asyncify）の評価は書いていない（必要が生じなかった）。

## 解釈が分かれた点

1. **hand-over を microtask にしたこと**（W2）。設計メモの当初の理由は実測で支持されなかった（M2）。保守的な順序として残した。
2. **停止 3 件の比較を弱めたこと**。`rsp 0x…` はホストのアドレスで native でも実行ごとに違う（実測）ので正規化した。wasm の abort の報告行は除いた。終了コードは native 134・wasm 99 で「両方非 0」だけ。**不一致を見た後に足した**。名前付き停止の本文（`fle: …`）とスレッド表は一致する。
3. **PM の受け入れ条件「native の SHA が変わらない」の読み**: 項目 3 は「native の挙動と生成物は変えない」。**挙動（entries、stdout）と生成コードのオブジェクトは変わらない（実測）が、実行ファイルの SHA は新規の空のオブジェクトのぶん変わった**。
4. **Chrome の隔離が不完全なまま結果を出したこと**（上）。結果は出したが、「遮断が完全」とは書いていない。
5. **`stack_churn`・`deep_locals`・自己検査・leak 計数は、変異検査の生存を見た後に足した**。したがって、これらがあるから変異が検出できる。最初から持っていた検出器ではない。

## 未決の点(PM)

1. **native の SHA の変化**（`09528ebc…` → `5968c889…`）。原因は新規の `fle_thread_wasm.c` が glob `runtime/fle_*.c` に入ること。選択肢: (A) このまま（SHA は変わるが、基準の 47 の runtime オブジェクトと生成物は不変）、(B) `fle_thread_wasm.c` を native の glob の外に置く（例: `harness/web/threads/` に移す。`build_nmod_wasm.sh` が runtime の glob で拾うので、wasm 側のスクリプトの変更が要る）。依頼は `lift/runtime/fle_thread_wasm.c` を指定していたので (A) にした。
2. **Chrome の遮断を UDP の `connect()` 1〜2 回で許すか**。許さないなら、ネットワーク名前空間（root 権限、または `unshare` の許可）が要る。
3. **シャドウスタック領域 1 MiB（W1）と JSPI スタック 984 kB の既定を、実エンジンのスレッド本体の深さを測る前に前提にしてよいか**。実エンジンのスレッド本体は走らないので、壁 22 より先が通るまで測れない。
4. **`THREADS=1` をオプトインのままにするか**（W5）。
5. **`-sSUPPORT_LONGJMP=wasm` と `-sJSPI` を実エンジンの wasm の既定にするか**（`jspi-real-wasm.md` §10 の判断点 1・2 の続き。この作業は wasm の既定を変えていない）。
6. **Firefox・Safari の JSPI**。未測定。JSPI が無いブラウザを対象に含めるなら、Worker + `Atomics.wait`（T3）か Asyncify の評価が要る。

## 限界

- **実機 Windows との照合は 0 件。実エンジンのスレッド本体は一度も走っていない**。スタブのスレッドの形（`TTimerThread` 型、`TLayeredControlThread` 型）は `thread-model.md` の読みに基づく。実エンジンの JSPI スタックの深さ（a4 の経路で約 150 kB の下限）は、スレッドごとの上限が別々に効くことを toy のフレームで見ただけで、**実エンジンのフレームで各スレッドが何 kB 要るかは未測定**。
- **シャドウスタックの超過は検出であって防止ではない**（領域の下のメモリを先に壊す）。
- スレッド 1 本あたりのメモリ（約 1,091 kB）は RSS で、JSPI スタックの確保量ではない。V8 の内部（JSPI スタックの確保・解放の方式）は見ていない。
- Chrome の切り替えコストは各 1 回の測定。node の 1.7〜3.1 倍遅い理由は調べていない。native の 2 システムコールが strace なしでどれだけかは測っていない。
- Firefox、Safari、headed の Chrome、Edge は未測定。Chrome は headless 156 の 1 構成。
- `MsgWaitForMultipleObjects` の `fWaitAll = TRUE`、`HWND_BROADCAST`、`WM_TIMER`／`WM_PAINT`、`TerminateThread` などは決定 0122 のまま答えない（停止）。
- `cargo test --workspace` は走らせていない。関連テストの一部だけを実行した（各 1〜2 回）。
