# 0315f(仮): RT-B の `pthread mutex deadlock detected` の読みと、GIL が GUI とオーディオを直列化するかの読み(mt3。PM の instruction 12:37・13:0x、期限 12:55・13:20)

状態: **仮**。区分: **実測**=ファイル・バイナリを読んだ事実(コマンドの出力)、**読み**=コードを読んだ推論、**未確認**。
**実際に実行した物(読みのみ)**: 新しい build・Chrome・Wine・wasm の実行は **0**。読んだ物: gpu1 が RT-B の bundle(`fl-engine-test-rtb-0a33cda7`)の実 Chrome で得た停止の記録(`gpu8-results/0331/logs/rtb-o1b/{report.json,eval-st-expanded.json,log-end.log}`)、**その bundle の wasm そのもの**(`engine.ea6a15277ea0.wasm`、name section なし)から自作の道具で指定の関数だけを切り出して `wasm-objdump -d`(`tmp/mtx/carve.py`)、同 wasm の data segment の文字列、RT-B の link map(`ci-rtb/engine/hyb/link.map`)、emsdk の libc(`system/lib/libc/musl/src/thread/*.c`、`system/lib/pthread/*.c`、`dlmalloc.c`)、RT-B の木の runtime(`fle_thread.c`・`fle_gil.h`・`fle_gil.c`・`fle_callback.c`・`fle_glob.c`・`fle_env.c`)、`harness/gil/gen_wrap.py`。同時に RT-D2 の build(別報)が走っているので、重い job は走らせていない。

## 1. 停止の実体(実測)

gpu1 のページが記録した wasm の呼び出し stack は 8 frame(内側から)。**その bundle の wasm の同じ関数 index を切り出して逆アセンブルし、data の文字列と突き合わせた**:

| # | func index | 実体(根拠) |
|---|---|---|
| 0 | 134101 | **`__pthread_mutex_timedlock`**(libc-mt-debug)。`pthread_self()->tid`(global 4 の +24)を読み、`i32.const 16`(EBUSY)を返す分岐と assert の文字列を持つ。assert 文字列 `at || own != __pthread_self()->tid && "pthread mutex deadlock detected"` は data の 268629074 番地(実測) |
| 1 | 134173 | **dlmalloc の本体**(mspace の malloc。mutex の番地 284677340 / 284677388 に対して timedlock を 4 回呼ぶ) |
| 2 | 134177 | dlmalloc の aligned 経路(timedlock 1 回) |
| 3 | 134176 | `aligned_alloc` 型の wrapper |
| 4 | 133224 | **`malloc`**(export 名で確認) |
| 5 | 132796 | **`fle_w_read_alloc`**(`fle_env.c`):ゲストの UTF-16 文字列を読んで `malloc((n+1)*2)`。data の文字列 `string longer than the supported limit`・`NULL string pointer`・`out of memory` がこの関数が参照する物(実測) |
| 6 | 133101 | `fle_glob.c` の `atom_arg`(`empty atom name is not defined`・`atom name with a non-ASCII character …`) |
| 7 | 133104 | `fle_glob.c` の `atom_findx` / `atom_add`(`atom`、`"#dddd" outside 1..0xBFFF …`)= **`FindAtomW` / `GlobalFindAtomW` / `AddAtomW` / `GlobalAddAtomW` のどれか** |

= **ゲストが Atom 系 API を呼び → runtime が文字列を `malloc` し → dlmalloc の mutex を取ろうとした時に、`own == 自分の tid` で assert**(= 同じ pthread tid がすでに持っている normal mutex をもう一度取った。debug の libc は `at == NULL`〔待ち続ける形〕でだけこの assert を入れる:`pthread_mutex_timedlock.c:95`)。直前のログは paint-msg の連続と `act1-net` の InternetOpenUrlW の拒否(`last error 12007`)で、**runtime の `fle_stop` の名前つき停止はない**(実測)。

## 2. 何が分かって、何が分からないか

**分かった(実測・読み)**:
1. libc は **`libc-mt-debug.a` + `libdlmalloc-mt-debug.a`**(RT-B の link map)。**debug 版**なので `pthread_mutex_lock` は NORMAL でも必ず遅い経路を通り、所有者を記録し、再入を trap にする(`pthread_mutex_lock.c`:`XXX EMSCRIPTEN always take the slow path in debug builds so we can trap rather than deadlock`)。**non-debug の libc なら、同じ再入は trap でなく黙って永久待ち(deadlock)になる** = これは『assert が出た』ことで、実害は同じ(読み)。
2. dlmalloc の mutex は `pthread_mutex_t`(`USE_SPIN_LOCKS 0`、`USE_RECURSIVE_LOCKS` は未定義 = **normal mutex**。`dlmalloc.c` 29 行・2056〜2110 行)。**malloc は再入できない**。
3. この build の mutex を取る libc の利用者は、link map では `pthread_mutex_lock/timedlock/trylock/unlock`、dlmalloc、`proxying.o`・`em_task_queue.o`・`thread_mailbox.o`(Worker 作成・proxy)、`pthread_cond_*`、stdio(`__lockfile`・`ofl`・`vfprintf`・`fputs`・`fwrite`)。**`lift/` と runtime の C は pthread_mutex を直接使わない**(PM の grep と同じ)。wasm 内の `__pthread_mutex_timedlock` の呼び元は 30 関数(dlmalloc 本体・stdio・proxying 等)で、runtime の `fle_*` からは **`malloc`/`free`/stdio 経由**でしか入らない(実測)。

**分からない(未確認)**:
- **なぜ『持っている自分』が再入するのか**。tid は emscripten が **pthread ごと**に払い出す(`emscripten_get_next_tid.c`:atomic の連番)ので、**2 つの Worker が同じ tid を持つことはない**(読み)。再入が起こるのは **同じ pthread(= 同じ Worker)の上で、malloc の途中から別の実行文脈が malloc に入る**時だけ。候補は次の 2 つで、**どちらも wasm の実行なしには決まらない**:
  - (i) **JSPI の suspend**:この build は `-sJSPI` で、guest thread は JSPI stack で動き、`fle_thread_wasm.c` が **`WebAssembly.Suspending` の import(`__asyncjs__fle_js_wait`・`__asyncjs__fle_wasm_idle_wait_async`)で suspend し、別の JSPI stack(別のゲスト thread)を同じ pthread の上で再開する**(`flengine6.orig.js` 25472〜25479 行)。**malloc の mutex を握ったまま suspend する経路が無ければ起きない**が、libc の中(malloc・stdio)に JSPI の suspend が入る入口は **無い**(libc の関数は Suspending を import しない。`__timedwait` 等に JSPI の語は 0 件 = grep)。すなわち (i) は**単独では起きにくい**(読み)。
  - (ii) **`fle_stop` / `fprintf` の診断**:manifest のコピーに `FLE_DIAG_WIN/CB/VFS_MISS = 1`(`paint-msg` の連続)があるが、**gpu1 の切り分けの 2 本(診断あり・なし)はどちらも 4 回の display change を通っている**(PM が 12:5x に確認)ので、**診断は引き金ではない**(実測)。
- **起きたのは o1(clip を持ち上げて落とす + Edit メニュー + 畳み/開き)の run だけ**で、project を開いただけの畳みでは出ない(PM の確認)。**atom 系の呼びが連続する経路は何か**:`FindAtomW`/`AddAtomW` は `RegisterClipboardFormat`・`RegisterWindowMessage`・ドラッグ&ドロップ(OLE の `DoDragDrop` は clipboard format の atom を使う)・メニューの modal loop に出る(読み)。**clip のドラッグ(ImageList_Drag + OLE の drag 系)で atom が呼ばれた**と読むのが自然だが、**atom の呼びの引数・呼び元は記録がない**(診断がない)。

**読みの結論(最善の仮説、未確認)**: 再入は **同じ Worker の上の 2 つの JSPI stack(2 つのゲスト thread、または callback の入れ子)の間**で、**malloc の mutex を握った状態が JSPI の suspend をまたぐ**ことで起きる。ただし libc の中で suspend する入口が無いので、**『malloc の中で別のゲスト thread に切り替わる』は読みでは作れない**。残る可能性は (iii) **GIL=1 の Worker 間の同期**:`fle_gil.c` の `fle_gil_acquire_slow` は `atomic.wait`(同期の待ち)で、**`malloc` を呼んだ Worker の mutex 待ち**(`__timedwait` → `emscripten_futex_wait`)が JS の `Atomics.wait` を使う。これは別 tid の待ちなので assert にならない。**よって mutex の再入は、ゲスト thread の切り替えでなく、(iv) 同一 thread の中での再入**:malloc の中で **signal / 例外 / `longjmp`(`-sSUPPORT_LONGJMP=wasm`)** が起きて、malloc の mutex を持ったまま別の経路から malloc を呼ぶ形。ゲストの SEH の landing(`fle_seh.h`)は **`longjmp` で C の frame を捨てる**(`fle_gil_release()` の直後。`fle_seh.h:44`)。**`fle_w_read_alloc` が malloc を呼ぶ最中に例外〔ゲストの SEH〕が起きることは無い**(同期の C の関数)が、**malloc の中の mutex を握った frame を longjmp が飛び越える**条件は **malloc の中で fle_stop や trap が起き、その後 runtime が『停止して続行』する経路**に限られ、assert の直前に停止のログが無いので成り立たない(読み)。→ **決められない**。次の増分で『atom 系の API の呼びと、その時の malloc の状態』を取る診断(下 §6)が要る。

## 3. RT-A にも同じ構造があるか(差分の読み。実測ではない)

- **構造は RT-A にもある**:libc(debug)・dlmalloc の normal mutex・`-sJSPI`・GIL=1・`fle_w_read_alloc` の `malloc` は RT-A(`91bf78f6`)と RT-B で同じ(RT-B の差分は runtime の C と import 行だけで、libc・link の flags は `link-web.sh` 無変更)。
- **RT-B で増えた差分の読み**(いずれも『atom の呼びを増やす/malloc の途中で別の文脈が入る』とは直接関係しない):alpha1 の **callback-stack-limit**(`fle_call_guest` の入口に stack の検査を足しただけ。mutex も malloc も触らない)、**dcx-lockwindowupdate**・**imagelist-drag**(`fle_imagelist.c`:`ImageList_*` の drag 系。`malloc`/`free` を使う、**ドラッグ中に呼ばれる**。**o1 の再現経路と一致する**)、**Delay 2**(Atom 系に無関係)。**imagelist-drag は o1 の経路(clip のドラッグ)の新しい runtime の経路**で、RT-A には無い(RT-A は ImageList_* を `unsupported` で止める)。RT-A で o1 の clip ドラッグが **別の停止**(ImageList_*)を起こすなら、同じ run で比較できない(読み)。
- **gpu1 の切り分け(診断あり/なし・RT-A、12:55 まで)の結果が出たら読み合わせる**。RT-A で同じ assert が出るなら『既存の構造の欠陥』、出ないなら『imagelist-drag または o1 の新しい経路』。

## 4. 依頼者の追加報告への答え: GUI とオーディオは別 Worker か、GIL は直列化するか(実装の読み)

依頼者: 『限られたトラックの Solo でリアルタイム再生がギリギリ成立するが、Mixer ウインドウ表示・マウス移動で破綻する。オーディオ処理と GUI 処理が相互に影響している』。

### 4.1 (1) ゲスト thread と Worker の対応(`fle_thread.c`、読み)
- **Worker 0 は『最初のゲスト thread』だけを走らせる**(`fle_thread.c:1208` のコメントと `pick_worker`:『Worker 0 runs the first thread only』)。最初のゲスト thread は FL の `WinMain`/メッセージ loop = **GUI の主 thread**(描画 = Blend2D・`fle_gdi`・`fle_win` の paint はここ)。
- **FL が `CreateThread` するゲスト thread は Worker 1 以降**(`pick_worker`:『one live guest thread per Worker as long as there is one』:終わった thread の Worker を再利用 → まだ起動していない次の Worker → 一番空いた Worker)。mixer・DirectSound の再生 thread は FL が `CreateThread` する thread なので **Worker 0 ではない別の Worker(pthread)**に載る(RT-A/B の GIL=1、N = `clamp(hardwareConcurrency − 1, 2, 8)`)。
- **つまり GUI とオーディオは、すでに別 Worker(別の WebAssembly instance、別の JSPI stack)で動く**(読み。実際に N がいくつか・FL が何本 thread を作るかは、この run のログに記録がない = 未確認:`FLE_DIAG` に Worker の割り当ての行は出ない。thread の数は **引用**: alpha1 の native N1 〔processors=1〕で FL は 18 本、Wine の FL は 19 本〔0315 §6 の補足〕。**この Chrome の run の実測ではない**)。

### 4.2 (2) GIL の粒度(`fle_gil.h`・`gen_wrap.py`・`fle_callback.c`、読み)
- **『ゲストコードを実行している間は GIL を持たない。runtime の C を実行している間だけ持つ』**。冒頭の不変条件(`fle_gil.h`):『the lock is held exactly while a guest thread executes RUNTIME code; it is released while the thread executes GUEST code, while it is parked … and while a callback of the runtime runs in the guest (fle_call_guest)』。
- **取る所**: ゲストが呼ぶ**全 `fle_imp_*` の入口**(`gen_wrap.py` の `W(n)`:`fle_gil_acquire(); __real_fle_imp_n(); fle_gil_release();`。**13 個だけ lock なし**: `SetRect`・`IntersectRect`・`UnionRect`・`OffsetRect`・`InflateRect`・`EqualRect`・`PtInRect`・`SubtractRect`・`GetTickCount`・`timeGetTime`・`QueryPerformanceCounter`・`QueryPerformanceFrequency`・`GetCurrentThreadId`)、`fle_progress_slice`(preempt)、COM の thunk、JS の export。
- **放す所**: ゲストの callback(`fle_call_guest`:`fle_gil_release(); fle_dispatch(va); fle_gil_acquire();`)、park(`fle_thread.c:562`)、`fle_idle_wait`、ファイルダイアログ。**ゲストのコード(lift したコード)の実行中は持たない**。
- **GUI の paint の間の保持時間(読み)**:`WM_PAINT` の処理は **ゲスト(FL の Delphi のコード)が `BeginPaint` → 多数の `fle_imp_*`(`BitBlt`・`StretchDIBits`・`SelectObject`・`FillRect` …)→ `EndPaint`** を呼ぶ。**1 回の `fle_imp_*` ごとに lock を取って放す**ので、GIL は **API 1 回分(多くは数 μs〜数百 μs、`BitBlt` や Blend2D の `fle_raster` の塗りは数 ms)**しか保持されない。**paint 全体(数十 ms)を通しては保持されない**。ただし **`BitBlt`・`StretchDIBits`・`fle_raster_wrote`(window surface への書き込みと dirty の通知)は 1 回が数 ms〜10 ms 超になりうる**(1920×1080 の surface に 1 枚の画面全体を書く形)。Blend2D の描画(lift したコード = **ゲストのコード**)は GIL **外**で走る。**見積り**:GUI 側の GIL の保持は『ゲストの全実行時間の中の runtime の C の部分』で、paint の間は **fle_gdi の塗り(数 ms の塊)の連続**。オーディオ thread が `fle_imp_*`(DirectSound の `GetCurrentPosition`・`Sleep`・`WaitForSingleObject`)を呼ぶ頻度は **数百〜千 回/秒**(RT-A で FL の audio thread が 5 ms 周期のため。実測ではない)なので、**GUI の 1 回の塗り(最大 10 ms 超)の間にオーディオ thread の呼びが GIL 待ちで最大その分だけ遅れる**のは読みで成り立つ。**直列化の程度は計測が要る(未確認)**。
- GIL の待ちの方式(`fle_gil_acquire_slow`):**2000 回の spin のあと `atomic.wait`(最大 3 s)**。待つ Worker の優先順位は **ゲスト thread の優先度**(`SetThreadPriority`:`fle_thread_gil_prio`)と到着順。**オーディオ thread が高い優先度(THREAD_PRIORITY_TIME_CRITICAL 等)なら、待つ列で GUI より先に取る**(読み)が、**GUI の保持中は取れない**(preempt しない)。

### 4.3 (3) GIL の待ち時間の診断の設計(`FLE_DIAG_GIL=1`、fle_gil.h・fle_gil.c のみ。実装は PM の許可の後)
- **観測だけ(挙動を変えない)**。`fle_gil_acquire`/`fle_gil_release` の中で、取得ごとに (a) **待った時間** = 最初の CAS 失敗から取得まで(速い経路で取れたら 0)、(b) **保持した時間** = 取得から解放まで、を thread(Worker)別に数える。区切り: `<0.1 / <1 / <5 / <20 / <50 / ≥50 ms` の件数と最大、**どの API(`fle_dbg_note` の文字列)で保持が長かったか**の上位 5(件数と最大)。出力は `FLE_DIAG_GIL=1` のときだけ stderr に 1 秒ごと(`fle_dbg`)。
- **コスト**:**速い経路(待たずに取れた時)に時計を読むのを避ける**: 取得の成功時に `clock` を読むのは**保持の計測のために必要**なので、**取得と解放で各 1 回**の `emscripten_get_now`(= `performance.now()` の JS 呼び = 約 50〜100 ns)ではなく **`__builtin_wasm_...` には時計がない**ので、`clock_gettime(CLOCK_MONOTONIC)` の wasm 実装(`emscripten_get_now` → JS)を使う。**1 取得あたり 2 回 × 約 0.1 μs = 約 0.2 μs**、**GIL の取得は GUI paint で数万回/秒**(`fle_imp_*` の呼びごと)になりうるので、**約 数 ms/秒のコスト = 0.5% 未満の見積り(未確認)**。精度を落とすなら **「待った(遅い経路に入った)ときだけ」待ち時間を測り、保持時間は 16 回に 1 回だけ測る**(サンプリング)。**診断が無い時は `#ifdef` で消える**(製品の形に影響しない)。**既存の `fle_gil_contended` / `fle_gil_spins` / `fle_gil_nwait` のカウンタ**が遅い経路の回数をすでに数えているので、**待ちの時間の分布だけ**を足すのが最小。

### 4.4 (4) 『オーディオと GUI を別 Worker に分ける』を lift-fidelity で評価
- **現状でも別 Worker**(§4.1)。だから『別 Worker に分ける』は新しい分離でなく、**『別 Worker 間の GIL による直列化を減らす』**問題。**lift-fidelity の方針**:(a) 我々が足した余計な処理(GIL そのもの。FL は Windows では mixer thread と GUI thread が真に並列に走り、**各 API が自分の状態を OS の中で保護する**)の**縮小**、(b) 外部 API の契約(各 `fle_imp_*` がスレッドセーフであること)の**拡充**。= **GIL を細かくする/外すのは (a) の除去と (b) の拡充の組**で、**局所的な近道ではない**(方針に合う)。
- **3 案**:
  1. **粗いまま、保持時間を短くする(最小・低リスク)**:長い `fle_imp_*`(`BitBlt`・`StretchDIBits`・`fle_raster_wrote`・Blend2D の dirty の合成)の**中で GIL を一時的に放す**(lock の間にゲストのメモリの読みだけの区間がある物)。変更範囲:`fle_gdi.c`・`fle_raster.c` の数か所。リスク:放している間に別 Worker が同じ DC・surface を変更する(**DC・surface ごとの 2 段目の lock**が要る)。期待効果:GUI の長い塗りの間にオーディオ thread の `fle_imp_*` が通る(**待ち ≤ 1 ms**)。**まず診断 §4.3 で『長い保持の API の上位』を見てから**。
  2. **lock を状態ごとに分ける(中)**:`fle_win`(窓 table)・`fle_gdi`(DC・描画)・`fle_thread`(scheduler)・`fle_event`(event・wait)・`fle_vfs`・heap(`malloc` = libc の mutex)を**別の lock**にし、**オーディオ thread が通る API(DirectSound・event・wait・Sleep・時計)は `fle_thread`/`fle_event` の lock だけ**にする。GUI の描画(`fle_gdi`/`fle_raster`/`fle_win`)は別 lock なので並列。変更範囲:runtime の全 `fle_imp_*` の入口(**867 個**の wrapper〔RT-B の `engine/gil/names.txt` と `fle_gil_wrap.c` の行数、実測〕の分類)と、**複数の状態を跨ぐ API**(`SendMessage` が callback を呼ぶ = lock の入れ子、`GetMessage` が event と窓 table を見る)の順序。リスク:**lock の順序(デッドロック)**、`fle_win` の `tab[]` に触る全部。期待効果:**GUI とオーディオの直列化がほぼなくなる**が、実装と検証が最大(tests/gil の全部)。
  3. **オーディオ thread だけ GIL 外で走らせる(狭い・中リスク)**:**mixer の `CreateThread` の thread が呼ぶ API の集合を限る**(DirectSound の `IDirectSoundBuffer::*`、`WaitForSingleObject`/`SetEvent`/`Sleep`、`timeGetTime`、`QueryPerformanceCounter`、`InterlockedXxx`)**だけを lock なし**(いま 13 個だけ lock なし=`fle_gil_lockfree.inc`)にし、**その集合が触る状態(event table・DirectSound buffer・thread table)を専用の小さい lock**にする。『どの thread がオーディオか』は **FL のクラス名や番地の分岐ではなく、『その API の集合』**で決める(= 状態の分離)。変更範囲:`fle_event.c`・DirectSound(`fle_ds*`)・`fle_thread.c` の wait/wake。リスク:event と thread table の lock が GUI の `GetMessage`/`MsgWaitForMultipleObjects` と共有されるので順序が要る。期待効果:オーディオが GUI の長い塗りに止められなくなる(案 1・2 の中間)。
- **推奨(読み)**:**まず §4.3 の診断で『GIL の待ちが本当にオーディオを止めているか・止めている API は何か』を取る(型 C、fle_gil.h/fle_gil.c のみ、約 27 分)**。結果が『長い保持の少数の API』なら案 1(小さく)、『多数の短い保持の競合』なら案 3 → 2。**診断なしに GIL を分けない**(局所的な近道の禁止)。

## 4.5 この節の数字の区分(誤解を避けるため)
- **実測**: 867 個の wrapper、lock なし 13 個(`fle_gil_lockfree.inc`)、停止 stack の 8 frame の実体、libc が debug 版、dlmalloc が normal mutex。
- **読み**: GIL の保持が『API 1 回分』であること、`BitBlt` 等が数 ms になりうること。
- **私の見積り(根拠の弱い数)**: 『オーディオ thread の `fle_imp_*` が数百〜千 回/秒』『paint 中の GIL 取得が数万回/秒』『診断のコスト 1 取得あたり約 0.2 μs、全体 0.5% 未満』は、**どれも測っていない目安**。診断の設計(§4.3)は、この見積りが外れても成り立つように『遅い経路に入った時だけ待ち時間を測り、保持時間はサンプリング』にしてある。
- **sbrk の確認(実測)**: `emscripten_resize_heap` は sbrk の中で **lock なし**(CAS の再試行だけ。`sbrk.c`)なので、**メモリの伸長(`memory.grow`)は malloc の mutex を握った状態で起きうる**が、それ自体は同じ pthread の再入ではない。assert の原因の候補から外す。

## 5. 未確認(正直に)
- 停止の再入の**原因そのもの**(§2)。atom 系の呼びの引数と呼び元、その時の malloc の状態の記録がない。
- GIL 待ちの**実測値**(いま `FLE_STAT_ACQ()` は空)。依頼者の観測が GIL 起因かは未確認(読みでは成り立つ)。
- N(Worker の数)と FL が作る thread の数・割り当て(この run のログに無い)。

## 6. 次の診断の提案(runtime のみ、型 C、PM の許可の後)
1. **FLE_DIAG_GIL**(§4.3)。
2. **atom 系と malloc の診断**:`fle_glob.c` の `atom_arg` で、引数の文字列(先頭 40 文字)・`fle_host_tid`・`fle_worker_id`・`fle_callback_depth`・現在のゲスト thread の idx を `FLE_DIAG_G9F` 型の getenv で 1 行出す(最初の 200 回)。次に再現した時、**停止の直前の atom 呼びが何か・どの thread・どの入れ子か**が分かる。
3. 再現は gpu1 の o1(診断あり・なし・RT-A)の結果を先に読み合わせる。
