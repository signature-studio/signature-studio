# W6 同期・スレッド系の事前調査（CreateEventW の停止点、スレッド設計の判断材料）

日付: 2026-10-21。読み取り専用の調査（`lift/crates`、`lift/runtime`、`lift/harness` は変更していない。決定は PM）。依頼時の HEAD は 5e7d7e6。調査中に別担当のコミット 00858ec（0047、A2a）が入った（本書の数値には影響しない。生成 C は `out-wasm2/out/` の 03:39 版、`out-inv/funcs.json` は古い生成 C 由来）。

表記: **実測** = 下記スクリプトの出力。**読み** = objdump の静的な読み（実行はしていない）。**推測** = 未検証の解釈。数値の根拠は `lift/out-sync/`（`README.txt` に実行順）。

## 0. 要約

1. **現在の停止点**（0x524c40）は Delphi RTL の `Classes.InitThreadSynchronization` と読める（`ThreadLock := TObject.Create; SyncEvent := CreateEvent(nil, True, False, nil)`）。引数は属性 NULL、手動リセット TRUE、初期状態 FALSE、名前 NULL。ハンドルが非 0 なら通る。**同じ unit（InitTable index 41）に SetEvent / Wait は無い**（実測）。SyncEvent を使う関数は InitTable の 895 unit のうち 2 unit（895、898）の直接閉包にしか入らない。
2. **InitTable の範囲で CreateThread に静的に届くのは、保守的に絞って 2 unit（index 895 と 898、実行順で 851 番目と 854 番目 / 895）だけ**。素の到達は 13 unit だが、11 unit は「スレッドを作らない経路」を通る（§3.3）。それより前（850 unit）には、スレッド生成が出ない。ただし間接 call と vtable は追っていない（下界）。
3. スレッドは **Delphi RTL の TThread**（RTTI で確認: `TThread`、`TExternalThread`、`TEvent`、`TMultiReadExclusiveWriteSynchronizer`）。OS には必ず **CREATE_SUSPENDED で作り、`ResumeThread`（戻り値が 1 であること）で始める**。33 種の TThread 派生コンストラクタ（TMixerThread_DS/ASIO、TRenderThread、TWAVStreamingThread、TFileDeletionThread ほか）がある。読んだ 2 つの Execute は、**停止まで INFINITE で待つループ**で、同期的に完走させる案（B）は成立しない。
4. **推奨**: **(C) をいま実装し、スレッド生成に届いた時点で (A) の JSPI 版へ拡張する**。(C) のイベント表・ハンドル領域・待機の意味は (A) の部分集合になるように作る（作り直しにしない）。ブラウザで成立する「スレッド」の実体は、JSPI（Chrome/Edge 137、Firefox 153、Safari 27 で出荷済み）で wasm のスタックを中断する協調スケジューラが唯一、現実的（pthreads は §4 の理由で重い）。JSPI の動作は node 24 で実験した（§4.2）。
5. 決定が要る点: §6。

## 1. 閉包のイベント系 API（調査 1）

### 1.1 方法（実測のスクリプト）

`sync_callers.py`: PE の import / delay-import から IAT スロットを引き、`.text` の objdump から `jmp [IAT]`（thunk）を拾い、`call thunk` / `jmp thunk` の地点を `.pdata` で関数に対応づけた。73 thunk（対象名）、地点 1,978。InitTable の閉包への到達は `unit_reach.py` / `closure_membership.py`（`out-inv/bfs_units.py` と同じ表・同じ到達規則: 直接 call の辺だけ、分岐の向きは見ない。したがって「到達」は過大見積もり、間接 call を追わない点は過小見積もり）。InitTable は 942 件、実行対象 895 件（実測、`out-inv` と一致）、閉包 U = 6,297 関数。

### 1.2 呼び出し地点の数（実測、`counts.txt`、`summary_tables.txt`）

| API | thunk | 地点 | 関数 | U に入る地点 | 備考 |
|---|---:|---:|---:|---:|---|
| CreateEventW | 1 | 17 | 14 | 8（6 関数、unit 41 / 45 / 314 / 365 ほか） | 停止点 |
| SetEvent | 1 | 17 | 17 | 7 | |
| ResetEvent | 1 | 8 | 7 | 4 | |
| WaitForSingleObject | 1 | 16 | 16 | 3 | |
| WaitForMultipleObjects / Ex | 1 / 1 | 1 / 3 | 1 / 3 | 0 | |
| SignalObjectAndWait | 1 | 1 | 1 | 0 | f_c56ae0 |
| MsgWaitForMultipleObjects / Ex（user32） | 1 / 1 | 3 / 1 | 3 / 1 | 2 / 0 | |
| WaitMessage（user32） | 1 | 2 | 2 | 1 | |
| CloseHandle | 2 | 73 | 59 | 既存の応答あり | 既に fle_vfs.c が答える |
| DuplicateHandle | 1 | 2 | 2 | 1 | 既存（fle_vol.c）。イベントは未対応 |
| CreateMutex / OpenMutex / ReleaseMutex / CreateSemaphore / ReleaseSemaphore / PulseEvent / OpenEvent / CreateWaitableTimer | - | **0** | - | - | import に無く、名前の文字列も無い（`strings_scan.txt`。OpenEvent の UTF-16 が 1 件あるのは別の部分文字列） |
| InitializeSRWLock / InitializeConditionVariable / SleepConditionVariable | - | - | - | - | import に無く、`GetProcAddress` の名前（UTF-16 文字列）として現れる（xref: 0x65ce31、0x482b77、0x482bfd）。0x482b94 以降で、取得に失敗したら DLL 自前の実装（0x481910〜0x481940）を使う**読み**。SRWLock の 0x65ce31 側のフォールバックは未読 |
| EnterCriticalSection / LeaveCriticalSection / InitializeCriticalSection / DeleteCriticalSection | 2 | 699 / 798 / 65 / 65 | 550 / 634 / 48 / 48 | 既存 | 決定 0017 |

合計: イベント・待機系 67 地点（Create 17、Set 17、Reset 8、Wait 16+1+3+1+4 = 25 の待ち）。**名前付きのイベントは U の中に無い**（§1.3）。

### 1.3 引数の使われ方（実測 + 読み）

**CreateEventW の直接 17 地点**（`createevent_args.tsv`）: lpEventAttributes は 16 地点で NULL（残り 1 は TEvent.Create の引数）、lpName も同じ。bManualReset は TRUE が 2（0x44cfff と **0x524c64 = 停止点**）、FALSE が 14、変数が 1。bInitialState は TRUE が 1（0x44cfff）、変数 1、FALSE 15。

**TEvent.Create（f_481550。CreateEventW の 17 地点のうち 1 つ、0x48160c）**: 引数は rcx=クラス、dl=割り当てフラグ、r8=属性、r9b=手動リセット、[rsp+0x20]=初期状態、[rsp+0x28]=名前、[rsp+0x30]=UseCOMWait（読み。0x481550〜0x48160c の読み）。26 地点（`tevent_args.tsv`、`site_args_481550.txt`）の集計: 手動リセット TRUE 2（ラッパ f_481690 内の 0x4816ea と 0x1157416）、FALSE 24。初期状態 TRUE 1（0x92f451）、変数 1（0x92f81e）。名前は 22 が NULL、4 が変数（c8489f、c848d4、c85b39、c85b6b。どれも U に入らない）。属性は全地点で NULL。f_481690 は TEvent の `Create(UseCOMWait)` に当たる（手動 TRUE、初期 FALSE、名前なし。読みと Delphi RTL の知識による推測）。U に入る TEvent.Create の地点は 5（`tevent_sites_in_U.tsv`）。

**重要**: 24 地点が**自動リセット**なので、Wait が信号を消費する意味を正しく持つ必要がある。

**待ち 25 地点の待ち時間**（`wait_args2.tsv`、`wait_summary2.txt`。`wait_args.py` は MsgWaitForMultipleObjectsEx の時間を r9d で読む誤りがあり、訂正版が `wait_args2.py`）:

| 種類 | 地点 | U に入る |
|---|---:|---:|
| INFINITE | 7 | 2（0x5266ce = TThread.WaitFor、0x8671fc = f_867170 内の MsgWait） |
| 有限の定数（100、200、1000×3、10000 ms） | 6 | 1（0x5266a0 = WaitFor の MsgWait、1000 ms） |
| 変数（引数、構造体のフィールド） | 11 | 2（0x44d19e = MREW のロック競合時、0x524dc0 = f_524db0） |
| 不明（thunk を素通しする f_44a5b0） | 1 | 0 |

0x44d19e/0x44d17e は TMultiReadExclusiveWriteSynchronizer の待ち。タイムアウトは初期化（0x44d027）で `[+0x30] = -1` なので実質 INFINITE。`lock xadd` で数えるロックの**競合が起きたときだけ**踏む（読み。f_44d1c0 の 0x44d219〜0x44d232）。単一の実行では競合しない（推測）。

### 1.4 既存のハンドル・CloseHandle との整合（読み）

- **`fle_sys.c` に仮想ハンドル表は無い**（依頼の前提の訂正）。そこにあるのは仮想時計、`CRITICAL_SECTION` のアドレスをキーにした表（決定 0017）、`FLE_PROCESS_HEAP`（0x7FF700000000）の固定トークン。ハンドル表を持つのは `fle_vfs.c`（ファイル `0x7FFA00000000`、検索 `0x7FFA10000000`）、`fle_reg.c`（`0x7FFA20000000`）、`fle_glob.c`（`0x7FFA50000000`）、`fle_vol.c`（プロセス/スレッドの複製 `0x7FFA60000000`）、`fle_vmap.c`（ファイルマッピング `0x7FFA70000000`）。形式は 4×(i+1)、再利用しない、各領域 0x10000000。
- `fle_imp_CloseHandle`（fle_vfs.c:685）は、登録された**クローズフック**（`fle_vfs_add_close_hook`、最大 4 個。現在 vol と vmap の 2 個）に先に尋ね、どれも自分のものと言わなければファイルハンドルとして `file_arg` に渡す。`file_arg` は、仮想ファイル系が発行していない値で `fle_stop`（「handle was not issued by the virtual file system」）。したがって**イベントのハンドルを CloseHandle で閉じるにはフックの登録が要る**（登録がないと停止する）。thunk 2 つ（0x406e60、0x4231a0）とも `fle_imp_CloseHandle` に結ばれている。
- 0x7FFA30000000、0x7FFA40000000 は 0024 が他担当の予約の可能性として空けた。**0x7FFA80000000 は runtime / docs のどこにも使われていない**（grep 実測）。`fle_vol_source`（DuplicateHandle）は、イベントのハンドルを渡されたら「was not issued by this runtime」で止まる（読み）。DuplicateHandle の 2 地点（0x692f5d、0xc91811）はいずれもスレッド/プロセスの擬似ハンドルの複製に見え（読み）、U に入るのは 1 地点（unit 895/898）。
- 「単一スレッド」の規則（各ファイルの `owner_tid` と `fle_host_tid()`）は **9 ファイル**にある（fle_proc、fle_imports〔TLS〕、fle_vol、fle_sysinfo、fle_ole、fle_vmap、fle_sys、fle_glob、fle_oleaut）。wasm では `fle_host_tid()` が定数 1（決定 0045）なので、wasm では「別スレッドで止まる」規則は起こり得ない。

## 2. スレッド系 API と、InitTable の範囲でスレッドが作られるか（調査 2）

### 2.1 API の呼び出し地点（実測、`counts.txt`）

| API | thunk | 地点 | 関数 | U に入る地点 |
|---|---:|---:|---:|---:|
| CreateThread | 2（0x406f80、0x423260） | 3 | 3 | 1（f_412b50 のみ） |
| ExitThread | 1（0x406fb0） | 1 | 1 | 0 |
| ResumeThread | 1 | 2 | 2 | 1 |
| SuspendThread / TerminateThread / GetThreadContext / SetThreadContext | 1 / 0 / 0 / 0 | 1 / 0 / 0 / 0 | 1 | 0 |
| GetExitCodeThread | 1 | 1 | 1 | 1 |
| GetThreadPriority / SetThreadPriority | 1 / 1 | 2 / 1 | 2 / 1 | 0 / 1 |
| SwitchToThread（既存） | 2 | 3 | 3 | 1 |
| GetCurrentThread / GetCurrentThreadId（既存） | 1 / 2 | 3 / 51 | 3 / 47 | 1 / - |
| _beginthreadex 相当 | - | - | - | **0**（C ランタイムではなく Delphi の BeginThread） |
| タイマーキュー（Create / Change / Delete） | 3 | 1 / 1 / 1 | 1 | 0 |
| SetTimer / KillTimer（user32） | 2 | 13 / 4 | 13 / 4 | 3 / 2（unit 895/898 のみ） |
| PostThreadMessageW / AttachThreadInput / EnumThreadWindows / GetWindowThreadProcessId | 4 | 1 / 3 / 5 / 12 | - | 0 / 0 / 1 / 4 |
| OpenProcess / CreateProcessW / GetExitCodeProcess | 3 | 1 / 2 / 3 | - | 0 |
| HeapCreate / HeapDestroy / TlsFree | 3 | 1 / 2 / 1 | - | 0 |

### 2.2 スレッドの実体は Delphi の TThread（読み + RTTI）

RTTI のクラス名（`rtti_find.py`、`ctor_classes.py`）: `TThread`（RTTI 0x4dac40）、`TExternalThread`（VMT 0x522648）、`EThread`、`THandleObject`、`TEvent`（VMT 0x4803b0）、`TMultiReadExclusiveWriteSynchronizer`、`TInterfacedObject`、`TStringList`。よって**プラットフォーム固有のスレッドではなく Delphi RTL（Classes）の TThread**。

- `f_412b50` = `BeginThread`。`0x412b96` で `IsMultiThread := True`（byte 0x15e60a9）に相当する書き込みの後、`CreateThread` を呼ぶ。開始アドレスは常に `0x412ae0`（スレッドラッパ。引数のレコード {proc, param} を読み、`call [rbp+0x38]` で proc を呼ぶ）。`f_412bf0` = `EndThread`（フック [0x13a5068] があれば呼び、`ExitThread`）。
- `f_525400`（TThread の本体の初期化）: **作成フラグは常に 0x4（CREATE_SUSPENDED）**。スタックサイズ指定があれば 0x10004（| STACK_SIZE_PARAM_IS_RESERVATION）。proc は `f_5251b0`（ThreadProc: Execute を `call [vmt+0x10]` で呼び、終了時に `f_524de0`（SyncEvent の SetEvent）と `EndThread`）。
- 開始は `f_5259a0`（Start/Resume）の `ResumeThread`（0x5259c7）。その後で `cmp eax,1`（0x5259cc）: **ResumeThread の戻り値が 1（以前の中断数）でなければ例外を作る**。`f_526500` が `f_5259a0(dl=0)` を呼ぶ入口で、39 関数が呼ぶ。
- `f_526610`（WaitFor）: 呼び出し元が主スレッドなら、`MsgWaitForMultipleObjects(2, {スレッド, SyncEvent}, FALSE, 1000, QS_SENDMESSAGE)` のループ（戻り値 1 なら `CheckSynchronize(0)`）。そうでなければ `WaitForSingleObject(スレッド, INFINITE)`。最後に `GetExitCodeThread`。主スレッドかどうかは `MainThreadID`（0x15e4f18）と現在のスレッド ID の比較。**仮想スレッドごとに別の ID が要る**（決定 0001 の「スレッドごとに一意」を保つ）。
- `f_526700`（0x406D1388 の RaiseException）は `IsDebuggerPresent` が 0 なら素通し（読み）。スレッド名付けで、`IsDebuggerPresent` の応答（W5）に依存する。

### 2.3 InitTable の範囲でスレッドが作られるか（実測 + 読み）

`unit_reach.py` → `unit_reach2.py` → `unit_reach3.py`（`unit_reach3.txt`）。`f_412b50`（BeginThread）に届く unit 数:

| 条件 | unit 数 | unit |
|---|---:|---|
| 素の直接閉包 | 13 | 114、245、279、280、365、647、659、745、876、880、895、898、935 |
| `f_522710`（TExternalThread のコンストラクタ）を除く | 6 | 365、876、880、895、898、935 |
| さらに辺 `f_92f1c0 → f_92f420` を除く | **2** | **895、898** |

- TExternalThread は、`[self+0x40] = 1` にする（0x522741）。`f_525400` は 0x525489 の `jne 0x525557` で CreateThread を飛ばし、`GetCurrentThread` / `GetCurrentThreadId` を使う（読み）。「いまのスレッドを TThread に見せる」だけで、**スレッドは作らない**。7 unit がこの経路だけで届いていた。
- `f_92f1c0 → f_92f420`（TActivityThread の生成）は、`f_92f1c0` が 5 番目の引数 `[rbp+0x70]` が 0 でないときだけ呼ぶ（0x92f254）。U に入る呼び出し元は `f_938580` だけで、渡す値は 0（0x9385c2 の `mov BYTE PTR [rsp+0x20],0x0`）（読み）。
- **残る 2 unit は、InitTable 実行順で 851 番目（index 895）と 854 番目（index 898）**（実測、`msgloop_counts.txt`）。両 unit は `f_1305fd0 → f_12f98f0`（159 命令、アプリ起動の本体に見える。推測）の先に入り、直接閉包がそれぞれ 4,626 / 4,620 関数、151 種の API を踏む（`units.tsv`）。**過大見積もり**: 分岐の向きを見ていない。
- 届く TThread 派生のコンストラクタ（`ctor_classes.txt`）: TWAVStreamingThread（f_6787f0）、TFileDeletionThread（f_10204f0）、TMixerThread_ASIO（f_c55630 内）、TMixerThread_DS（f_12aedf0 内）、および TRenderThread / TDWPRenderThread は同じ f_12b0050 の別の呼び出し元（U 外）。TThread 派生は全体で 33 種類のコンストラクタ、55 の直接呼び出し地点（`thread_classes.tsv`）: TCustomHintShowHideThread、TWorkerThread、TDropTargetTransferThread、TDropSourceThread、TLayeredControlThread、TActivityThread、TCallbackThread（10 地点）、TNewsThread、TSampleLoaderThread、TScanThread、TBrowser{Cache,CacheValidity,TagDiscovery,Search}Thread、TMIDIThread、TAnalyseThread、TCloudSearchThread、TPreviewLoader/DownloadThread、TUpdaterThread、TCrashLogUploadItem、TStreamThread、TMixerThread_{DS,ASIO}、TRenderThread、TDWPRenderThread ほか、名前が取れない ctor が 7（クラス参照の読み方が合わなかったもの。`thread_classes.tsv` で class=None）。
- **下界である点**: 間接 call、vtable、データポインタ経由（`TObject.Create` の仮想コンストラクタ、クラス参照の変数）は追っていない。33 ctor の全部に直接の `call ctor` が見つかった（`thread_classes.tsv`）が、U に入る呼び出し元を持つのは 6 ctor だけ。ただし仮想コンストラクタ（`TObject.Create` の経路）や継承したコンストラクタの連鎖は追っていないので、InitTable がスレッドを作らないとは言い切れない。**上界である点**: 分岐の向きを見ていない（上記 2 unit も、実際にスレッドを作るかは未確認）。

### 2.4 スレッドが何をするか（読み。Execute を読んだのは 2 つ）

- **TFileDeletionThread**（f_10206d0）: `[self+0x48]` のイベントを `call [vmt+0x10]`（TEvent.WaitFor、タイムアウト -1 = INFINITE。0x10206fc の `or edx,0xffffffff`）で待ち、2 つのキューを入れ替えて削除を処理し、`[self+0x1a]`（Terminated）が立つまで繰り返す。生成は f_1014d60 が、キューにファイルを入れるときに遅延して行う（0x1014de4 の `cmp [0x1545648],0`）。
- **TMixerThread_DS**（f_12aeec0）: `[rbx+0x1a]`（Terminated）が立つまでのループで、`WaitForMultipleObjects(2, 0x16bd4e0, FALSE, INFINITE)`（0x12aef73）、または `[0x15d3294]` が非 0 のときは `TMonitor` の `Enter`/`Leave` とカウンタの更新。オーディオ装置のイベントを待つ作業スレッドに見える（推測）。TMixerThread_ASIO（f_c55740）も INFINITE の `WaitForSingleObject`（0xc55760）。
- TWAVStreamingThread、TRenderThread、TDWPRenderThread ほかの Execute は読んでいない（メッセージ待ち、タイマー、ワーカープールのどれかは推測できない）。
- **スレッドを作る直接の CreateThread（TThread を通らない）** 2 地点（どちらも U 外）: f_8670c0（0x867152。スタックサイズ 1000、開始 0x866ff0 = `WaitForSingleObject(ev, 100)` のループで `PeekMessageW` を呼ぶ。同じ関数が `SetWindowsHookExW` も呼ぶ。メッセージフックの補助スレッドに見える。推測）、f_8b42c0（0x8b42ea。開始 0x8b42a0 が `call [obj]` を呼ぶだけの汎用ランナー）。

## 3. 設計の選択肢（調査 3）

### 3.1 前提（実測 + 読み）

- 生成 C は、**ゲストの関数呼び出しを C の関数呼び出し（`f_XXXX();`）に 1 対 1 で写す**（間接 call は `fle_dispatch`、40,725 か所）。したがって「ゲストのスレッドの途中状態」は **C（wasm）の呼び出しスタックそのもの**にある。ゲストのスタック（`R[4]`）とは別。`ucontext` は wasm で使えない（依頼のとおり）。
- ゲストの状態は大域: `R[16]`、`X[16]`（SSE）、`YH[16]`、`FL`、`fle_x87_r[8]` と `fle_x87`（制御ワード）、`fle_mxcsr`、`fle_last_error`、`fle_tls_values`（GS:[0x58] が指す表）、`fle_teb`。複数の仮想スレッドには**それぞれ持たせて切り替え時に入れ替える**必要がある（実測の実験: §3.2、`R[3]` が他のスレッドで壊れた）。
- 生成 C は `lock` 接頭辞を無視し、`Xadd` 159、`Cmpxchg` 52、`Cmpxchg16b` 5、`Xchg` 37 を**通常の読み書き**に落とす（`atomics_count.txt`、`wasm-pdata.c` の L_412d86 の Xadd は `fle_ld` + `fle_op_add` + `fle_st`）。協調的に切り替える（API 呼び出しの中でだけ）限り、これで正しい。本物の並列実行（D）では正しくない。
- ブロッキング API は、スレッドの有無に関係なく、**ブラウザでは同期的に待てない**。メッセージ待ち系の呼び出し地点（`msgloop_counts.txt`）: GetMessageW 1、PeekMessageW 21、WaitMessage 2、MsgWaitForMultipleObjects(Ex) 3+1、DispatchMessageW 14、PostMessageW 139、SendMessageW 216、Sleep 33（仮想時計で即座に進める）、MessageBox 10。ブラウザで「あらゆる操作に反応する」には、ゲストのメッセージループを、ブラウザのイベントで起こす形に結びつける必要がある。**これはスレッド設計と同じ仕組み（スタックの中断と再開）で解ける**（推測）。

### 3.2 JSPI の動作確認（実測、`lift/out-sync/jspi-probe/`）

emsdk 6.0.12、node v24.19.0（`--experimental-wasm-jspi` が要る。無いと `JSPI not supported by current environment`、`run.txt`）。ブラウザでは試していない。ブラウザの対応は検索で確認（caniuse、web-features、2026-10-21 時点: Chrome/Edge 137、Firefox 153、Safari 27、Baseline 到達 2026-09-14。本書では未検証の二次情報）。

| 実験 | 結果 |
|---|---|
| probe: 仮想スレッド T1 が待ち（Promise）で中断、T2 が同じインスタンスで SetEvent | T1 が再開し戻り値 5。T1 の `R[3]`（共有の大域）は **T2 に上書きされた**（`run-jspi.txt`）。切り替えで `R[]` 等を保存・復元する必要がある |
| probe2: 線形メモリの C スタック（shadow stack）をスレッドごとに分けない | T1 を先に再開（LIFO でない順）すると **T2 のバッファが壊れた**（`run2.txt`）。JSPI は wasm のネイティブスタックしか切り替えない |
| probe3: スレッドごとに C スタック領域を割り当て、中断の前後で `stackSave` / `stackRestore` | 両方のバッファが無事（`run3.txt`） |
| probe4: 2,000 本を同時に中断 | 完了、RSS +30 MB、生成 26 ms（`run4.txt`） |
| probe4: ExitThread 相当を JS の例外で C のフレームを抜けて投げる | export の境界で捕まり、インスタンスは使い続けられた（`run4.txt`） |
| probe4/5: 再帰の深さ（160 B のフレーム） | 10,000 は通る。20,000 が `Maximum call stack size exceeded`、50,000 は通る、100,000 以上は失敗（`run4.txt`、`run5-default.txt`）。**単調でない理由は未確認**（推測: 最適化後のフレームの大きさの変化）。`--wasm-stack-switching-stack-size=8192` を足しても同じ（`run5-8mb.txt`）。生成 C は `-O0` で、1 つのゲスト呼び出しが複数の wasm フレームになるので、**ゲストの呼び出しの深さの上限を実コードで測る必要がある**（未測定） |

### 3.3 比較表

| 案 | ブラウザで成立 | 決定論 / 再現性 | 既存コードへの影響 | スレッド実体化と最終目標 | 主な欠点 |
|---|---|---|---|---|---|
| **A1 協調スケジューラ（JSPI）** | ○（出荷済みの 3 エンジン。要ブラウザでの実測） | ○（切り替えは API 呼び出しの中だけ。仮想時計と整合） | 中。大域状態のスレッド別化、wait 系、スレッド ID、`owner_tid` 規則 9 ファイルの置換。生成 C と lift は変更不要 | ◎ メッセージループ・タイマー・オーディオの起こし方と同じ仕組み | **先取りの切り替えが無い**（CPU を使い続けるスレッドが UI を止める。実コードでは未測定）。ネイティブ側（clang の harness）に同じ API の別実装（ucontext 等）が要る。JSPI の再帰上限（§3.2） |
| A2 協調（Asyncify） | ○（古いブラウザでも） | ○ | 中 + ビルド | A1 と同じ | 215 MB の C、58,695 関数、間接 call 40,725 か所全てを計装することになる。サイズと速度の増加は**未測定**。`-O0` では特に大きい（Emscripten のドキュメント） |
| A3 手書きの状態機械 | ○ | ○ | 大（lift の CPS 変換） | A1 と同じ | 実質 Asyncify を自作する。却下 |
| A4 JS の Worker だけ（共有メモリなし） | × | - | - | - | ゲストのポインタが共有メモリを指す。成立しない |
| **B 同期実行**（CreateThread の中で Execute を完走） | ○ | ○ | 小 | × | 読んだ 2 つの Execute（TFileDeletionThread、TMixerThread_DS）は INFINITE の待ちループ。**戻ってこない**。待たない TThread にしか使えない。却下 |
| **C fail-closed**（スレッド生成で名前付き abort。それ以前のイベントは単一スレッドの状態機械） | ○ | ○ | 小（新規ファイル + TABLE 行） | △ まだスレッドが無い | InitTable の 851 番目（index 895）で止まる。そこから先に進めない |
| D pthreads（SharedArrayBuffer） | △（COOP/COEP ヘッダが要る。デプロイの制約） | × 実行ごとに変わる。A5 の bit-exact、決定論的な仮想時計（0017）と合わない | 大。**全オブジェクトを `-pthread` で再ビルド**。`R`、`X`、`FL`、x87、MXCSR を thread-local に（全メモリ・レジスタ参照が TLS 経由）。`lock` を `__atomic` に変える lift の変更。9 ファイルの `owner_tid` を本物のロックに。メモリ増加（`ALLOW_MEMORY_GROWTH` と共有メモリ）、512 MiB のアリーナ | ○ 本物の並列（オーディオ・レンダー） | 変更量が最大。再現性を失う |

### 3.4 推奨（1 つ）: C を今すぐ、スレッド生成に届いたら A1（JSPI）へ

**理由**:

1. 静的な範囲では、スレッド生成に届く前に 850 unit 分の進展がある（§2.3）。その間に必要なのは、イベントの状態機械と、競合しない前提の待機だけ（§4）。**スレッドの設計決定を今、急ぐ必要が無い**。
2. C の実装（イベント表、waitable なハンドル領域、待機の意味）は、A1 の部分集合になる。A1 が足すのは、ブロックしたスレッドを中断するスケジューラと、大域状態の切り替えだけ。
3. B は成立しない（§3.3）。D は変更が最大で、再現性と bit-exact の検証方針（A5、0017）を壊す。A2 は計装のコストが未測定。
4. ブラウザ側は、ゲストのメッセージ待ち（GetMessage / WaitMessage / MsgWait）をブラウザのイベントに結びつけるのに、いずれにせよ**スタックの中断**（JSPI か Asyncify）が要る。スレッドとメッセージループを同じ仕組みにできる（推測）。

**C を A1 に拡張するための準備（今すぐ入れてよい、小さい）**:
- 待機可能なオブジェクトを一つの表（種類、状態）にし、イベントとスレッド（後で）を同じ領域に置く。ハンドルは `0x7FFA80000000 + 4*(i+1)` を提案（§4.2）。
- 仮想スレッドごとに持つべき状態（§3.1）を、「現在のスレッドの文脈」を指す一か所にまとめる。当面は 1 つだけ。
- 仮想時計は単調に進める（待ちの有限タイムアウトで進める）。複数スレッドでも、時計は 1 つのまま。

**A1 に進むときに要る決定**（PM、§6）: 先取りの有無、ネイティブ側の実装（ucontext）、JSPI の再帰上限の実測。

## 4. 現在の停止点の最小実装案（調査 4）

### 4.1 呼び出しの読み（読み）

0x524c40（`f_524c40`、呼び出し元 `f_525600` = InitTable index 41 の proc。実行順 39 番目）:

```
524c44 mov rcx,[0x401970]   ; クラス参照（RTTI: TObject）
524c4b mov dl,1 ; call f_40f9e0     ; ThreadLock := TObject.Create → [0x15f26f0]
524c59 xor ecx,ecx ; or edx,-1 ; xor r8,r8 ; xor r9,r9
524c64 call 0x423220                 ; CreateEventW(NULL, TRUE, FALSE, NULL) → [0x15f2688] = SyncEvent
524c70 cmp [0x15f2688],0 ; jne ok ; call f_44cb40  ; 0 なら RaiseLastOSError（GetLastError を使う）
```

Delphi の `Classes.InitThreadSynchronization`（`ThreadLock := TObject.Create; SyncEvent := CreateEvent(nil, True, False, nil)`）と一致する（推測。名前は RTTI から取れない）。**ハンドルが 0 でなければ、エラーの経路は踏まれない**。続く `f_525600` は `MainThreadID`（[0x15e4600] の指す値）を `0x15f2650` に保存して戻る。

### 4.2 仕様（Microsoft Learn、2026-10-21 取得、`CreateEventW`、`SetEvent`、`ResetEvent`、`WaitForSingleObject`、`ResumeThread`）

- 引数の意味: `lpEventAttributes` NULL → 継承不可・既定のセキュリティ。`bManualReset` TRUE → 手動リセット（`ResetEvent` で非シグナルに）。`bInitialState` FALSE → 非シグナル。`lpName` NULL → 名前なし。
- 戻り値: 成功はハンドル（EVENT_ALL_ACCESS）、失敗は NULL + GetLastError。**名前付きで既存なら**同じオブジェクトへのハンドルと `ERROR_ALREADY_EXISTS`（この場合 `bManualReset`、`bInitialState` は無視）。名前は大文字小文字を区別、MAX_PATH まで、同じ名前空間を他のカーネルオブジェクトと共有（衝突は `ERROR_INVALID_HANDLE`）。成功時に last error を変えるとはページに書かれていない（変えない、が素直な読み。**選択**）。
- `SetEvent`: 手動は ResetEvent まで信号のまま。自動リセットは**待機中のスレッド 1 つを解放するまで**信号のまま（待機者がいなければ信号のまま残る）。`ResetEvent`: 非信号に。`WaitForSingleObject`: 信号なら即戻り（自動リセットは信号を消費）、非信号で `dwMilliseconds` 0 なら待たずに `WAIT_TIMEOUT`（0x102）、INFINITE は信号になるまで戻らない、失敗は `WAIT_FAILED`（0xFFFFFFFF）。`CloseHandle`: 最後のハンドルが閉じられたときにイベントが破棄される。

### 4.3 最小実装案

新規 `lift/runtime/fle_sync.c`、`fle_sync.h`、`imports_table.rs`（または W0 後の自分のファイル）に TABLE 行 `("kernel32.dll", "CreateEventW", "fle_imp_CreateEventW", "jmp_imp:CreateEventW")`。

- **CreateEventW**: `manual = (uint32_t)R[2] != 0`（BOOL は 32 ビット。ゲストは `edx = 0xFFFFFFFF`）、`initial = (uint32_t)R[8] != 0`。`R[1]`（属性）と `R[9]`（名前）が NULL でなければ `fle_stop`（U の中で非 NULL が出るのは名前付きの TEvent.Create 4 地点だけだが、いずれも U 外。fail-closed）。イベント表（{manual, signaled}）に追加し、`FLE_SYNC_HANDLE_BASE + 4*(i+1)` を `R[0]` に返す。last error は変えない。
- **ハンドル値**: `FLE_SYNC_HANDLE_BASE = 0x7FFA80000000`、領域 0x10000000、再利用しない（既存の領域と重ならないことを `tests/vol.rs` と同形のテストで確認する。0x7FFA30/40 は他担当の予約の可能性があるので使わない）。
- **CloseHandle**: 初回の CreateEventW で `fle_vfs_add_close_hook`（フックの枠は 4 のうち、vol と vmap が使用中なので 3 個目）。フックは自分の領域の値だけを引き受け、イベントを破棄済みにして `R[0] = 1`。閉じ済みの値は `0` と `ERROR_INVALID_HANDLE`（6）。
- **SetEvent / ResetEvent**（同時に入れる。同じ表の更新だけ）: 信号 / 非信号にして `R[0] = 1`。他の領域の値は `fle_stop`。閉じ済みは 0 + 6。
- **WaitForSingleObject**（U では TMonitor / MREW の競合時と、unit 895/898 の経路にしかない。停止点には不要。入れるなら単一スレッドの意味で）: 信号なら（自動リセットは消費して）0。非信号で ms=0 は 0x102。有限は仮想時計を ms 進めて 0x102（待たせる相手のスレッドが存在しないので、実際にもタイムアウトする）。**INFINITE は `fle_stop`**（永遠に戻らない。`Sleep(INFINITE)` の決定 0017 と同じ扱い）。仮想時計を進める窓口（現在は `fle_clock_ticks` の読み出しだけ）を `fle_sys.c` に 1 つ足す必要がある。
- テスト: `tests/` に、手動 / 自動リセット、初期状態、Set / Reset / Wait の組み合わせ、CloseHandle、名前付き・属性ありの停止。

### 4.4 先にあるもの

- **このイベントの使われ方**: `f_524d90`（ResetEvent(SyncEvent)）、`f_524db0`（WaitForSingleObject(SyncEvent, ms)）、`f_524de0`（SetEvent(SyncEvent)）はすべて `f_524e00`（CheckSynchronize）か ThreadProc（0x5251b0、0x525270）、`f_525ec0`（Synchronize / Queue）から呼ばれる。`f_524de0` は U のどの unit にも入らず、`f_524d90` / `f_524db0` は unit 895/898 のみ（実測、`unit_reach2.txt`）。**unit 41 から 894 の間では使われない**（静的）。
- 次に Event 系が要る unit（`out-inv/units_first_api.tsv`）: index 45（f_52f1f0 が直接 `TMultiReadExclusiveWriteSynchronizer` のコンストラクタ f_44cfb0 を呼ぶ。CreateEventW を 2 回: **手動 TRUE・初期 TRUE** と **自動・非信号**）、314（同じ MREW ほか。SetEvent / ResetEvent / WaitForSingleObject は競合時のみ）、365 / 876 / 880 / 935（f_481690、手動・非信号の TEvent）、745（f_666960。TMonitor の関連。自動リセット）。**CreateEventW が 4 つの異なる属性の組み合わせで要る**（手動 TRUE/FALSE × 初期 TRUE/FALSE）ので、最初から 4 通りを実装・テストするのが安全。
- 停止点が通った先の実行は**していない**（本書は実装していない）。他の同じ unit 内の未対応（delay `GetLogicalProcessorInformation`、`MessageBoxA`、`FormatMessageW`、`SysFreeString` など、`unit_blockers.tsv` の index 41 の行）が次の停止点になる可能性が高い（推測）。

## 5. 実測と推測の区別（要点）

- 実測: 呼び出し地点の数（73 thunk、1,978 地点）、U / unit への到達（13 → 6 → 2）、引数表（CreateEventW 17 地点、TEvent.Create 26 地点、待ち 25 地点）、RTTI のクラス名、`lock` 相当命令の数（生成 C）、JSPI の動作（node 24、5 つの実験）、既存ハンドル領域の値、Microsoft Learn の仕様。
- 読み: Delphi RTL の各関数の役割、0x524c40 が `InitThreadSynchronization`、CreateSuspended + Resume の挙動、Execute の 2 つ、TExternalThread が CreateThread を飛ばすこと。
- 推測: 「名前の無い」クラスの役割、unit 895/898 が「アプリ起動」、先取り無しの影響、ブラウザでの JSPI の振る舞い（node のみで実験）、次の停止点。
- 未確認: ブラウザでの JSPI、JSPI の再帰の上限（実コード）、SRWLock のフォールバック（0x65ce31）、残りの Execute、間接 call の先。`lock_count.txt`（objdump の線形走査で 3,351 行。データを命令として読むので過大）は参考値で、本文では使っていない。

## 6. PM が決める点

1. **C → A1 の方針でよいか**（B と D を却下する根拠は §3.3）。
2. CreateEventW の last error（成功時に変えない）と、仮想時計を進める窓口を `fle_sys.c` に足す件。
3. `0x7FFA80000000` を同期オブジェクトの領域にしてよいか。
4. A1 に進むときの先取りの有無、ネイティブ harness の実装（ucontext）、JSPI のブラウザでの実測（ブラウザ版 harness は未作成。`NODERAWFS` は node 限定、決定 0045）。
5. `IsDebuggerPresent`（W5）の応答は、TThread のスレッド名付け（f_526700）に影響する。W5 と同時に決める。

## 7. 触ったファイルと制約

新規のみ: `docs/investigations/sync-thread-survey.md`、`lift/out-sync/`（スクリプト、`*.tsv`、`*.txt`、`README.txt`、`jspi-probe/`、`tmp/`、`jspi-probe/emcache/`（emcc のキャッシュ約 30 MB、`tmp/`））。ソース（`lift/crates`、`lift/runtime`、`lift/harness`）、既存ファイルは変更していない。`out-inv/tmp/text.dis`（270 MB）、`out-wasm2/out/*.c` は複製せず参照。`rm`、`>` による上書き、git 操作、`/tmp`、sleep / timeout は使っていない。権限拒否はなし。

訂正の記録: `lift/out-sync/rev.py` は callees の形を取り違えた初版で動かない（削除禁止のため残し、`rev2.py` が正）。`wait_args.py` は MsgWaitForMultipleObjectsEx の時間を誤って読む（`wait_args2.py` が訂正版）。両方の理由は `README.txt` にある。`lift/iat-unsup-closure.txt` は本調査の前からある未追跡ファイル（本調査のものではない）。
