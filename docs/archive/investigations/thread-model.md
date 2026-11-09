# スレッドモデルの調査: 壁 21(`kernel32!CreateThread`)を偽らずに通す選択肢と推奨

調査のみ(静的分析と文書作成。実装・ビルド・実行はしていない)。**決めるのは PM**。この文書は推奨を書くが何も決めない。依頼時の HEAD は cfb2b1e、作業中に HEAD が f47436c まで進んだ(`lift/runtime` の差は `fle_vmap.c/.h` の 20 行だけで、スレッド・同期・窓・TLS の runtime は cfb2b1e と同じ。`git diff --name-status cfb2b1e HEAD -- lift/runtime lift/crates docs/decisions`)。数値と読みは cfb2b1e の決定 0121 の測定(`docs/investigations/fruity7/stop21-record.txt`)に基づく。

## 0. 要約

| 項目 | 結果(区分は §1) |
|---|---|
| **CreateThread の呼び出し点** | **7 点**(FLEngine 3、QFC 1、ILTools 2、Blend2D 1。freetype と dsp は 0)。すべて `call thunk`/`call [IAT]` で `.pdata` の関数内にあり、逆アセンブルで検証した(実測、§3)。**`lpStartAddress` が定数 `lea` の点は 7 点中 5 点**(3 つの Delphi `BeginThread` ラッパー = 各 DLL の共通 1 点 `lea r8,[rip-0xcb]`、FLEngine `f_8b42c0` の `lea r8` = 汎用ランナー、Blend2D の `lea r8`)。**残り 2 点**(FLEngine `f_8670c0`、ILTools `f_6968e0`)は `mov r8,[rip+slot]` で、データスロットの値は FLEngine で `0x866ff0`、ILTools で `0x696810` と静的に読めた(スロットに再配置が無いか、実行時に書き換わらないかは未確認)。**したがって「静的に特定できない開始アドレス」は 0 点**。ただし Delphi の `TThread` 経由の 3 点は開始アドレス(ThreadProc)が共通で、**スレッドの本体(`Execute`)は仮想呼び出しなので TThread 派生クラスごと**(§3.3) |
| **TThread 派生クラス** | FLEngine に **28 クラス**の `Execute` を VMT から引けた(実測、`execute_waits.tsv`)。生成点は **33 種の ctor、55 の直接呼び出し点**(`thread_classes_in.tsv`。**名前が取れない呼び出し点が 13、ctor は 7 種**) |
| **`CreateFruityInstance` の初期化で作られうるスレッド** | 静的に届く(直接 call 辺、tier 1)TThread 派生の呼び出し点は **10 点(ctor 9 種。クラス名が取れたのは 8 クラス + 名前不明 2 点)**(`f_13751e0` からは 8 点)。**実際に作られるかは分岐次第で未測定**。壁 21 の直後に実際に作られると**読める**ものは **2 本**(QFC の `TTimerThread` = 壁 21、FLEngine の `TLayeredControlThread` = 同じ関数 `f_71e900` の次の行)。根拠の強さは §4 |
| **壁 21 のスレッド本体**(読み) | `TTimerThread.Execute`(QFC `0x1e85bd0`): FPU/MXCSR 初期化の後、**`DwmFlush` を呼んで、1 周ごとに `f_1e860a0`(所有者への tick)を呼ぶループ**を `Terminated`(`[+0x1a]`)まで回す。待つのは `DwmFlush`(実機では画面の Present まで待つ)と `Sleep`(`f_1df1db0`)。**`MsgWaitForMultipleObjects` と `PeekMessageW` は `Execute` ではなく `TThread.WaitFor`(`f_1df1ee0`)の中**(呼び出し側がメインスレッドの場合)で、**壁 21 のスレッドの側ではない**(決定 0121 の読みの訂正、§5.1) |
| **選択肢の要約** | T0 止まる: 偽りなし、G1 は動かない。T1 作ったことにする: **この壁の範囲では偽らずには言えない**(デストラクタが `Finished` フラグを読む、`DwmFlush` 周期の tick がゲストの状態を更新する)。T2 協調スレッド: 設計は大きいが偽りは最小。G7/G8 まで同じ機構で伸ばせる。T3 実スレッド: 再現性を失う、全再ビルド、COOP/COEP。T4 コールバック同期実行: `Execute` が `Terminated` まで戻らないので単独では成立しない |
| **推奨(§8)** | **T2(JSPI による協調スレッド)を最終形とし、段階導入する**。壁 21 を通す最短の入口は「T2 の最小形(スレッド表 + 待ちでの切り替え + 仮想時計の進行)を `CreateThread`/`ResumeThread`/`WaitForSingleObject`/`Sleep`/`DwmFlush` に限って入れる」こと。T1 は採らない(偽る)。T4 単独は採らない。切り替え基準を §8 に書く |
| **G4 の最初のフレームまでにスレッドの本物が必須か**(読み) | **壁 21 のタイマースレッドは必須**(`CreateFruityInstance` が作る。`TTimerThread` の `Terminated` と `Finished` をデストラクタが `Sleep` ループで待つ)。**描画・音声のスレッドは G1〜G4 では必須でない可能性が高いが、未測定**(§4.4) |
| **この調査で確認できなかったこと** | 実行していない(壁 21 の先は 1 点も測っていない)。ブラウザ・wasm は未測定。実機 Windows との比較は無い。TThread 派生 28 クラスのうち `Execute` の中身(命令)まで手で読んだのは 5 つ(§9)で、残りは直接 call 深さ 3 の待ち系 API の一覧(`execute_waits.tsv`)だけ。仮想呼び出しと vtable は静的な到達に含めていない(下界)。詳細は §9 |

## 1. 凡例と手順

- **実測** = 今回のスクリプトの出力、または Microsoft Learn・Emscripten・MDN・W3C の文書をこの作業で取得して読んだ文言。**読み** = 逆アセンブルの静的な読み(実行していない)。**未確認** = 調べていない、または確かめられない。**引用** = 既存の文書(決定、調査)の記述で、ここでは再測定していない。
- 実機の挙動は Learn で確認できる範囲だけを事実とした。それ以外(実 Windows でのスケジューリングの順序など)は読み。
- **手順**: `docs/investigations/thread-model/` にスクリプトと結果がある。
  1. `scan_sites.py`: PE の import と delay-import の IAT スロットから、`.text` を生バイト走査(`FF 25`=thunk、`FF 15`=間接 call、`E8`/`E9`=thunk への相対 call)し、`.pdata` の関数内にあること、capstone の線形掃引で命令境界に `call`/`jmp` があることで検証した(`verified=yes`)。結果は `thread_api_sites.tsv`(ノイズの多い API を除いた 691 行)と `api_counts.tsv`(全 API の件数)。
  2. `createthread_args.py`: 各 CreateThread 点の直前 40 命令から r8(開始)、r9(引数)、`[rsp+0x20]`(フラグ)の書き込みを読む。
  3. `reach_threads.py`、`classreach.py`: 他の作業者の索引(`docs/investigations/goal-ladder/fle_index.py` を私が再実行して作った pickle。直接 call/jmp の辺、`lea` の関数アドレス)で、`CreateFruityInstance`(0x1375990)、`f_13751e0`、Run(0x86a660)から各 TThread 派生 ctor に届くかを BFS で調べた(tier 1 = 直接 call のみ、tier 2 = `lea` の関数アドレスも辺にする)。**仮想呼び出し、vtable、ポインタ表は辿っていない(下界)。分岐の向きを見ていない(上界)**。
  4. `execute_waits.py`: ctor 名 → RTTI(`07 <len> <name> <vmt>`)→ VMT+0x10(`Execute`。QFC `ThreadProc` の `call [rbx+0x10]` で確認)→ 直接 call 深さ 3 以内の待ち系 import。
  5. `disasm_pe.py`、`dump_pe.py`、`rtti_names.py`: 手読み用(capstone)。QFC は測定の基底 0x1d00000 に合わせて `--base 0x1d00000` で表示した(ファイル上は 0x400000)。
  6. `lock_count.py`、`spin_sleep_scan.py`: LOCK 接頭辞付き命令と `pause`、`Sleep` の引数の種類の計数。
- 入力 DLL の SHA-256(先頭 16 桁): FLEngine `b2c64da40921cf26`(依頼の SHA と一致)、QFC `afc5a0ed70eb7e4a`、ILTools `20409dcbc5eb5055`、Blend2D `66634d24412084b7`、freetype `6ea6c106ad2695b5`、dsp `497dd4f77e859df9`。ディスク上のファイルをそのまま読んだ(インストーラからの抽出は不要だった)。
- 取得した外部文書(`/tmp/opencode/thr/learn`、`/tmp/opencode/thr/emsc` に取得。**終了時に削除したので再現は URL から**。HTML の SHA-256 は取得時に記録した): Learn の CreateThread、ResumeThread、SuspendThread、ExitThread、GetExitCodeThread、TerminateThread、SwitchToThread、WaitForSingleObject、WaitForMultipleObjects、MsgWaitForMultipleObjects、Sleep、SetEvent、CreateSemaphoreW、ReleaseSemaphore、PeekMessageW、GetMessageW、PostMessageW、SendMessageW、GetCurrentThreadId、GetLastError、SetThreadPriority、TlsAlloc、DwmFlush、DllMain(取得日 2026-10-30)。Emscripten の `docs/porting/pthreads.html`、`docs/porting/asyncify.html`、`docs/tools_reference/settings_reference.html`(2026-10-31 取得)、V8 の `v8.dev/blog/jspi`、MDN の `SharedArrayBuffer`、`Window.crossOriginIsolated`、`Atomics.wait`、WebAssembly threads 提案の `Overview.md`。

## 2. スレッド系 API の棚卸し(実測。`api_counts.tsv`、`thread_api_sites.tsv`)

セルは **thunk 数 / 呼び出し点 / 検証済みの呼び出し点 / 呼び元関数の数**。thunk 数は `jmp [IAT]` の個数(FLEngine、QFC、ILTools は呼び出しが thunk 経由、Blend2D・freetype・dsp は `call [IAT]` で直接)。`-` は import なし、または 0。

| API | FLEngine | QFC | ILTools | Blend2D | freetype | dsp |
|---|---|---|---|---|---|---|
| **CreateThread** | 2/3/3/3 | 1/1/1/1 | 2/2/2/2 | 0/1/1/1 | - | - |
| ExitThread | 1/1/1/1 | 1/1/1/1 | 1/1/1/1 | 0/2/2/2 | - | - |
| FreeLibraryAndExitThread | - | - | - | 0/1/1/1 | - | - |
| ResumeThread | 1/2/2/2 | 1/2/2/2 | 1/2/2/2 | - | - | - |
| SuspendThread | 1/1/1/1 | 1/1/1/1 | 1/1/1/1 | - | - | - |
| GetExitCodeThread | 1/1/1/1 | 1/1/1/1 | 1/1/1/1 | - | - | - |
| SetThreadPriority / GetThreadPriority | 1/1/1/1 / 1/2/2/2 | 1/2/2/2 / 1/2/2/2 | 1/1/1/1 / 1/1/1/1 | - | - | - |
| SwitchToThread | 2/3/3/3 | 2/3/3/3 | 2/3/3/3 | - | - | - |
| GetCurrentThread / GetCurrentThreadId | 1/3/3/3 / 2/51/51/47 | 1/2/2/2 / 2/16/16/14 | 1/1/1/1 / 2/34/34/31 | - / 0/1/1/1 | - / 0/5/5/5 | - / 0/1/0/0 |
| WaitForSingleObject(Ex) | 1/16/16/16 | 1/8/8/8 | 1/7/7/7 | 0/2/2/2 + Ex 0/1/1/1 | - | - |
| WaitForMultipleObjects(Ex) | 1/1/1/1 + Ex 1/3/3/3 | - | Ex 1/2/2/2 | - | - | - |
| MsgWaitForMultipleObjects(Ex) | 1/3/3/3 + Ex 1/1/1/1 | 1/1/1/1 | 1/2/2/2 + Ex 1/1/1/1 | - | - | - |
| SignalObjectAndWait | 1/1/1/1 | - | - | - | - | - |
| WaitMessage / GetMessageW | 1/2/2/2 / 1/1/1/1 | - | WaitMessage 1/2/2/2 | - | - | - |
| CreateEventW / SetEvent / ResetEvent | 1/17/17/14 / 1/17/17/17 / 1/8/8/7 | 1/7/7/5 / 1/9/9/9 / 1/4/4/4 | 1/9/9/7 / 1/8/8/8 / 1/5/5/5 | 0/1/1/1 / 0/1/1/1 / 1/0/0/0 | - | - |
| CreateSemaphoreW / ReleaseSemaphore | - | 1/1/1/1 / 1/2/2/2 | - | - | - | - |
| Sleep | 3/33/33/25 | 3/17/17/11 | 3/16/16/10 | - | 0/3/3/3 | - |
| SleepConditionVariableSRW / Wake(All)ConditionVariable | - | - | - | 0/4/4/4 / 0/4/4/4 + 0/1/1/1 | - | - |
| Acquire/ReleaseSRWLock(Exclusive/Shared) | - | - | - | Excl 0/20/20/18、Shared 0/3/3/2 | - | - |
| CreateTimerQueueTimer / Change / Delete | 1/1/1/1 ×3 | - | 1/1/1/1 ×3 | - | - | - |
| SetTimer / KillTimer | 1/13/13/13 / 1/4/4/4 | - | 1/3/3/3 / 1/3/3/3 | - | - | - |
| timeBeginPeriod | 1/2/2/2 | 1/1/1/1 | 1/2/2/2 | - | - | - |
| TlsAlloc / TlsFree / TlsGetValue / TlsSetValue | 1/1/1/1 / 1/1/1/1 / 1/4/4/3 / 1/2/2/2 | 同左 | 同左 | thunk 2 (IAT 経由の直接 call なし)/ -/ -/ 0/2/2/2 | - / 0/3/3/3 / - / 0/1/1/1 | thunk 1(call 0)×4 |
| FlsAlloc / Free / Get / Set | - | - | - | - | 0/1 / 0/3 / 0/6 / 0/5(各 検証済み) | thunk 1(call 0) |
| PostThreadMessageW / AttachThreadInput / EnumThreadWindows / GetWindowThreadProcessId | 1/1/1/1 / 1/3/3/2 / 1/5/5/5 / 2/12/12/11 | - | EnumThreadWindows 1/5/5/5、GetWindowThreadProcessId 2/10/10/9 | - | - | - |
| PostMessageW / PeekMessageW | 1/139/139/122 / 1/21/21/18 | 1/2/2/2 / 1/2/2/2 | 1/23/23/20 / 1/10/10/8 | - | - | - |
| InitializeCriticalSection / Enter / Leave / Delete | 65 / 699 / 799(798 検証)/ 65 | 10 / 35 / 58 / 10 | 10 / 24 / 38 / 10 | 1+3(SpinCount)/ 6 / 10 / 7 | 1 / 3 / 3 / 4 | 0 / 0 / 0 / 5 |

**確認した「無い」もの**(`api_counts.tsv` に行が無い、実測): `TerminateThread`、`SleepEx`、`OpenThread`、`SetThreadAffinityMask`、`_beginthread(ex)`(Delphi は RTL の `BeginThread`)、`QueueUserWorkItem`、`CreateTimerQueue`、`timeSetEvent`/`timeKillEvent`、`CreateMutexW`、`CreateWaitableTimerW`、`DisableThreadLibraryCalls`、スレッドプール API(6 DLL とも import に無い。`imp_list.py` の出力)。FLEngine の TLS ディレクトリは無い(データディレクトリ 9 が 0。QFC、ILTools、freetype、dsp も 0)。**Blend2D だけ TLS ディレクトリがある**(rva 0x1e1c00、40 バイト。決定 0102 以降の暗黙 TLS の設計 `implicit-tls-design.md` の対象)。

**LOCK 接頭辞付き命令**(実測、`lock_count.txt`): FLEngine 257(`lock xadd` 159、`lock cmpxchg` 52、`lock xchg` 35、`lock cmpxchg16b` 5、暗黙ロックの `xchg [mem]` 5、`lock add` 1)、QFC 113、ILTools 113、Blend2D 201(`lock xadd` 147、`xchg [mem]` 20 ほか)、freetype 21。既存の生成器はこれらの `lock` を**無視して通常の読み書きにする**(`fe-lift` の `int_more.rs` と `int_more2.rs` の冒頭に「LOCK is ignored: FLEngine runs on one thread」。決定 0013 項 9)。`pause` は各 DLL に 5 個ずつ。

**`Sleep` の引数**(読み、`spin_sleep_scan.txt`。直前の ecx への書き込みの線形読みで、分岐をまたぐものは `variable` または `unknown`): FLEngine は `Sleep(0)` 12、`Sleep(1)` 3、`Sleep(10)` 6、`Sleep(50/100/200/500/1000)` 計 9、変数 3、呼び元の引数 2。`Sleep(0)` と `SwitchToThread` は「他のスレッドに譲る」意味で使われている点があり、**単一スレッドでは譲る相手がいない**。runtime の現行は `Sleep(0)` で仮想時計を 0 進めるだけ、`SwitchToThread` は 0 を返して 100 万回で止まる(`fle_proc.c`、`fle_sys.c`)。

## 3. CreateThread の全呼び出し点(実測 + 読み)

| # | モジュール | 呼び元 | thunk / 形 | `lpStartAddress` | 種類(読み) |
|---|---|---|---|---|---|
| 1 | FLEngine | `f_412b50`(0x412bc6) | thunk 0x406f80 | `lea r8,[rip-0xcb]` = `0x412ae0`(定数) | **Delphi RTL `BeginThread`**。開始 = RTL の汎用 ThreadWrapper(`{proc,param}` のレコードを読み `call [rbp+0x38]`)。`IsMultiThread := True`(`[0x15e60a9]`)を CreateThread の直前に書く(0x412b96) |
| 2 | FLEngine | `f_8670c0`(0x867152) | thunk 0x423260 | `mov r8,[rip+..]` → スロット `0x1409c98` の値 = `0x866ff0`(スタックサイズ 1000、フラグ 0 = **即時実行**) | **直接の CreateThread**(TThread 経由でない)。開始 `0x866ff0` は `WaitForSingleObject(ev,100)` のループ(`WAIT_TIMEOUT` 0x102 なら再試行)で `GetCursorPos` などを呼ぶ補助スレッドに読める。呼び元 `f_86c0f0` ← `f_86ba70`/`f_86bbf0`。**`CreateFruityInstance` の閉包には入らない**(tier 1、2 とも不到達) |
| 3 | FLEngine | `f_8b42c0`(0x8b42ea) | thunk 0x423260 | `lea r8,[rip-0x33]` = `0x8b42a0`(定数。`call [obj]` を呼ぶだけの汎用ランナー) | **直接の CreateThread**(フラグ 0 = 即時実行)。呼び元 `f_8cb3b0` ← `f_8cb150`/`f_8cb4d0`。閉包には入らない |
| 4 | QFC | `f_40fe20`(0x40fe96。測定の VA 0x1d0fe96) | thunk 0x406d80 | `lea r8` = `0x40fdb0`(測定 VA `0x1d0fdb0`) | **Delphi RTL `BeginThread`**(同じ形)。**壁 21**。呼び元は `f_4f10f0`(TThread.Create の本体)のみ |
| 5 | ILTools | `f_40e4f0`(0x40e566) | thunk 0x404bb0 | `lea r8` = RTL の ThreadWrapper | **Delphi RTL `BeginThread`**。呼び元 `f_4ff8f0` ← `f_4ff800`。公開 export から届く: `CreateCursorFromFont`、`ShowEngineChoice`(読み。直接 call 辺で 12〜14 段) |
| 6 | ILTools | `f_6968e0`(0x696972) | thunk 0x41ce10 | `mov r8,[rip+..]` → スロット `0x6fe568` の値 = `0x696810`(フラグ 0) | **直接の CreateThread**。FLEngine #2 と同じ形(`mov edx,0x3e8`、スロット)。呼び元 `f_69b570`。公開 export からは直接 call で届かない |
| 7 | Blend2D | `f_1801a4888`(0x1801a48ed) | `call [IAT]` | `lea r8,[rip-0x183]` = `0x1801a4765` | **Blend2D のスレッドプールのワーカー**(`ExitThread`/`FreeLibraryAndExitThread`、`GetModuleHandleExW` を持つ形)。呼び元 `f_18019ccf0` ← `f_18019cfa0`(`.data` のポインタ表のエントリ。決定 0102 の読み)。**FL Studio が `thread_count` を渡すかは未確認**(`blend2d-lift-feasibility.md` B4 の引用) |

- **`lpStartAddress` が定数 `lea` の点は 5(#1、#3、#4、#5、#7)、`mov r8,[rip+slot]` の点が 2(#2、#6)**。#2/#6 のスロットは `.data` の qword で、値は静的に読める(FLEngine `0x1409c98` = `0x866ff0`、ILTools `0x6fe568` = `0x696810`。スロットの読み出し元の命令 `mov r8,[rip+disp]` をデコードして計算した)。**スロットを書く別の命令があるか、再配置の対象かは確かめていない**(未確認)。ILTools のスロットは `0x696810` が関数開始に見えるが、**ILTools はまだ測定で CreateThread に届いていない**。
- **Delphi の TThread 経由は共通の 1 点か**: **各 DLL に 1 点ずつ**(FLEngine `f_412b50`、QFC `f_40fe20`、ILTools `f_40e4f0`)。FLEngine の BeginThread の直接の呼び元は `f_525400`(TThread.Create の本体)1 つ、その呼び元 `f_5252d0`(TThread のコンストラクタ)を 34 の ctor が呼ぶ(直接 `call rel32` の走査で 34 点: `count_direct_calls.py`)。QFC は `f_4f10f0` ← `f_4f1000`(コンストラクタ)← 3 点(`TTimerThread`、`TSTBThread`、`TTimerThread` の別の経路)。**開始アドレスは共通の ThreadWrapper で、そこからレコード `{proc,param}` の `proc`(`TThread` の `ThreadProc`)を呼び、さらに `Execute` を `call [vmt+0x10]` で呼ぶ**(QFC `0x1df0ee0` で確認)。**スレッドが何をするかは `Execute`(クラスごと)で決まる**。
- **フラグ**: Delphi の 3 点は常に `CREATE_SUSPENDED`(`mov dword ptr [rsp+0x20],4` または `0x10004` の引数付き分岐。QFC `0x1df11cf`、`0x1df119c`)。直接の 3 点(#2、#3、#6)は 0(即時実行)。#7 は eax をそのまま渡す(読んでいない)。
- **`SuspendThread`**(各 DLL 1 点)は `TThread.Suspend` 相当(QFC `f_4f1de0` → `f_4f1e0d`)、`GetExitCodeThread` は `TThread.WaitFor` の最後(QFC `0x1df1fab`)。

## 3.5 ResumeThread・Terminated・WaitFor の構造(読み。QFC `0x1df....`、FLEngine は同形の `f_5259a0`、`f_526500`、`f_526610`)

TThread の主要な状態(QFC の `TThread` オブジェクトの読みで確認したオフセット。フィールド名は Delphi RTL の名前に当てた**読み**): `[+0x08]` スレッド ID、`[+0x10]` スレッドハンドル、`[+0x18]` Started(`ThreadProc` の先頭で 1)、`[+0x19]` 初期中断(CreateSuspended かつ外部でない)、`[+0x1a]` Terminated、`[+0x1b]` Suspended、`[+0x1c]` FreeOnTerminate、`[+0x1d]` Finished、`[+0x20]` ReturnValue、`[+0x40]` 外部スレッド(`TExternalThread`。CreateThread を飛ばして `GetCurrentThread/ThreadId` を使う)、`[+0x41]` Destroying。

- **Create**(`f_1df10f0`): 初期中断を `[+0x19]`(= CreateSuspended かつ非外部)、Suspended を `[+0x1b]`(= 非外部なら 1)に書き、**`CreateThread(NULL, 0, 0x1df0ee0, this, 4, &[+8])`**(スレッド ID を `[+8]` へ)、戻りを `[+0x10]` へ。失敗(0)なら `GetLastError` を引いた例外。
- **誰がいつ ResumeThread を呼ぶか**(読み): Resume 本体は `f_1df1640`(QFC `ResumeThread` は 0x1df1667)。入口の検査は「`[+0x19] | dl` が 0 なら例外」「`[+0x1d]`(Finished)が非 0 なら例外」「`[+0x40]`(外部)が非 0 なら例外」で、通れば `[+0x1b]` と `[+0x19]` を 0 にして `ResumeThread`、**`cmp eax,1`: 戻り値(以前の中断数)が 1 でなければ例外**。呼び出し元は 2 つ。(i) **`AfterConstruction`**(VMT のスロット `f_1df1410`。クラス生成の末尾の `f_1d0d630` が呼ぶ): `[+0x19]==0` かつ非外部なら `f_1df1640(dl=1)`。つまり `TThread.Create(CreateSuspended=False)` は生成の最後に自動で `ResumeThread` する。(ii) **`TThread.Start`**(`f_1df1dd0` → `f_1df1640(dl=0)`): `[+0x19]==1` のときだけ通る。`Resume`(`f_1df1e60`)は `ResumeThread` の戻りが 1 なら `[+0x1b]` を 0 にする。**壁 21 の経路**: `f_1e85eb0`(`TSharedAnimTimer` の起動)が `f_1df1000(TTimerThread, alloc=1, CreateSuspended=1)` でスレッドを作る(このとき `AfterConstruction` は `[+0x19]==1` なので Resume しない)→ **`f_1df1750(dl=1)`(`SetThreadPriority`。優先度表 `[-15,-2,-1,0,1,2,15]`(`0x1ebe998` の 7 dword。実測)の index 1 = −2 = THREAD_PRIORITY_LOWEST)→ `[+0x48] = 所有者` → `f_1df1790(dl=1)`(FreeOnTerminate := True)→ `f_1df1dd0`(Start = `ResumeThread`)**。FLEngine 側は `f_526500`(Start、39 関数が呼ぶ)と `f_5259a0`(Resume 本体、`ResumeThread` 戻り 1 の検査 0x5259cc)。**`f_13751e0` の閉包内の Start 呼び出し元は 7 関数**(実測、BFS)。
- **ThreadProc**(QFC `f_1df0ee0`): `[+0x18]=1`(Started)→ `Terminated` が 0 なら `Execute`(`call [vmt+0x10]`)→ 例外時は `[+0x38]` に捕まえた例外 → ReturnValue を `[+0x20]` から、FreeOnTerminate を `[+0x1c]` から退避 → **`[+0x1d]=1`(Finished)→ `f_1df0b10`(`SetEvent(SyncEvent)`)** → FreeOnTerminate なら自分を解放(`f_1d0cdc0`)→ `EndThread(ReturnValue)`(`f_1d0fec0` → `ExitThread`)。**スレッドは `ExitThread` を呼んで終わる**(戻り値を ThreadWrapper に返さない)。
- **終了条件とデストラクタ**(読み。QFC `f_1df1430` = TThread の解体処理): `Destroying`(`[+0x41]`)を立て、スレッド ID が非 0 で `Finished` でも外部でもなければ `Terminate`(`f_1df1ea0`: `[+0x1a]=1` にして仮想 `[vmt+8]` を呼ぶ)→ `Suspended` か初期中断なら `Resume`(`f_1df1e60`)→ **`while Started == 0: SwitchToThread`**(`f_1df1dc0` → `kernel32!SwitchToThread`。`0x1df1472`〜`0x1df147b` のループ。`[+0x18]` を見ているので、スレッドが**開始**するまで譲り続ける)→ `WaitFor`(`f_1df1ee0`)→ ハンドル解放(`f_1df1c80`)。**所有者の `TSharedAnimTimer` のデストラクタ**(`f_1e85fc0`)は別の経路で、`f_1df1750(dl=4)`(優先度 index 4 = +1 に上げる)→ `Terminate`(`f_1df1ea0`)→ `GetTickCount` を取り、**`[+0x20](スレッドオブジェクト)!= 0` の間、`Sleep(0)` を呼んで `GetTickCount` の差が 1000 ms 未満ならループ**(スレッド側の `DoTerminate` = VMT +0x00 の override `f_1e85c20` が `[rcx+0x48]`(所有者)の `[+0x20]` を 0 にして終わらせる)。**1 秒で打ち切る**。
- **`WaitFor`**(QFC `f_1df1ee0`、FLEngine `f_526610`): 呼び出しスレッドが `MainThreadID` と同じなら **`MsgWaitForMultipleObjects(2, {スレッド, SyncEvent}, FALSE, 1000, QS_SENDMESSAGE=0x40)` のループ**(戻り 1 なら `CheckSynchronize`)、そうでなければ `WaitForSingleObject(スレッド, INFINITE)`、最後に `GetExitCodeThread`。**したがって `MsgWaitForMultipleObjects` と `PeekMessageW`(`f_1d1e0b0` の呼び出し 0x1df1f51)は、`WaitFor` を呼んだスレッドの側の待ちで、スレッド本体ではない**。

## 4. `CreateFruityInstance` の初期化で作られうるスレッド(静的。上界と下界あり)

### 4.1 方法の限界(先に)

- **下界**: 直接 call/jmp の辺と `lea` の関数アドレスだけを辿った。**仮想呼び出し(`call [reg+disp]`)、vtable、ポインタ表は辿っていない**。Delphi は生成(`TObject.Create` の仮想コンストラクタ)、`Show`、`Idle` などを仮想で呼ぶので、**実際には届くのに数えていないスレッド生成がある**(例: 壁 21 の経路の `f_71e900` は直接 call の閉包に入らない(`reach` で tier 1・2 とも不到達)のに、実測では `CreateFruityInstance` の下で実行された。`f_721bb0` が `call [rbx+0xb0]`(0x722069)で呼ぶ仮想メソッドの 1 つで、その vtable スロットは FLEngine の複数の VMT にある: ファイル 0x31cb90、0x31e878、0x3205d8、0x415958、0xf6bcc8)。
- **上界**: 分岐の向きを見ていないので、実行時に通らない経路を含む。
- したがって次の表は「静的に直接 call で届く」であり、**「作られる」の証拠ではない**。作られると読める 2 本(§4.3)は、実測(壁 21 の停止点)と逆アセンブルによる。

### 4.2 `CreateFruityInstance` から直接 call で届く TThread 派生(実測。`classreach.tsv`、`reach_threads.tsv`)

TThread の ctor の直接呼び出し点 55(`thread_classes_in.tsv`。**33 種の ctor**、クラス名が取れた 28 クラス + 名前不明 13 点・7 ctor)のうち、`CreateFruityInstance`(0x1375990)から tier 1(直接 call のみ)で届く呼び出し点は **10**(ctor 9 種。`TMixerThread_DS`/`TMixerThread_ASIO`/`TRenderThread`/`TDWPRenderThread` は同じ ctor `f_12b0050` の別の呼び出し元で、クラス名で数えると表の行が増える)、`f_13751e0`(初期化の本体)から tier 1 で届くのは 8 点(`CreateFruityInstance` からは届くが `f_13751e0` からは tier 2 = `lea` の辺を足したときだけ届くのが、`TBrowserTagDiscoveryThread` の呼び出し点 `f_1081a20` と名前不明の `f_11efbf0` の 2 点)、**Run(`f_86a660`)からは 0**(実測。`classreach.tsv`)。

| クラス(RTTI) | 作る関数(ctor 呼び出し元) | 何をするか(`Execute` の読み。深さ 3 の直接 call で届く待ち系 API) | `CreateFruityInstance` から届く経路の例(関数の連鎖) | 根拠の強さ |
|---|---|---|---|---|
| **TMixerThread_DS** | `f_12aedf0`(DirectSound の初期化 `f_12afaf0` から) | `Terminated` までループし、`WaitForMultipleObjects(2, 0x16bd4e0, FALSE, INFINITE)`(DirectSound の通知イベント)、または `[0x15d3294]` が非 0 のとき `TMonitor` の Enter/Leave とカウンタ。`timeGetTime`、`QueryPerformanceCounter`、`EnterCriticalSection` を呼ぶ | `13751e0 > 11be0a0 > 11a57c0 > 1196c00 > 13582c0 > 1253610 > 1253260 > c58940 > c55630 > 12b0050` は `TMixerThread_ASIO` の経路。DS は `1253610 > 1253260 > 12afaf0 > 12aedf0`(閉包に `f_12afaf0` が入ることは実測、上の `reach`) | 静的に届く(強)。**実際に作られるかは、`DirectSoundCreate` の分岐次第で未測定**(`goal-ladder.md` §3.4 の読み) |
| **TMixerThread_ASIO** | `f_c55630`(`f_c58940` から) | `WaitForSingleObject(INFINITE)`、`SetEvent`、`EnterCriticalSection` | 上記 | 静的に届く(強)。ASIO ドライバーの COM が要る(`CoCreateInstance`)ので実際には作られない可能性がある(読み) |
| **TRenderThread / TDWPRenderThread** | `f_d18130`/`f_c4dba0`(`f_12b0050` の別の呼び出し元) | `EnterCriticalSection`、`SetEvent`、`Sleep`、`PostMessageW` | `f_12b0050` へは届くが、この 2 つの呼び出し元(`d18130`、`c4dba0`)は閉包に**入らない**(`classreach.tsv` で 0) | 弱(レンダー/書き出し時) |
| **TWAVStreamingThread** | `f_678360`(`f_6785e0` から) | `EnterCriticalSection` と仮想呼び出し。待ちの API は直接には見えない | `13751e0 > 11be0a0 > 11a57c0 > e90420 > eae4c0 > f54760 > f51fd0 > f46490 > ff0d70 > 692d70 > 695d20 > 678480 > 6785e0 > 678360`(**`f_f51fd0` = FLP を読む関数**を通る経路) | 静的に届く。プロジェクトを開くとき(読み) |
| **TFileDeletionThread** | `f_1014d60`(キューに入れるときに遅延して作る。`cmp [0x1545648],0`) | 自分のイベントを `TEvent.WaitFor(INFINITE)` で待ち、キューを入れ替えて `DeleteFileW` | `13751e0 > 11be0a0 > 1016810 > 10195d0 > 1016680 > 101d180 > 1014d60 > 10204f0` | 静的に届く。**ファイルの削除を要求しなければ作られない**(読み。遅延生成) |
| **TActivityThread** | `f_92f420`(`f_938440`、`f_92f1c0` ほか) | `TEvent.WaitFor`(`f_526700` = スレッド名付け)後に `[+0x48]` のオブジェクトの仮想メソッド(`[vmt+0x10]`)を `Terminated` までループ。`SetEvent` | `13751e0 > 11be0a0 > 11a57c0 > 1196c00 > 938440 > 92f420 > 92f550` | 静的に届く(強)。**生成が条件付きか未確認**(`sync-thread-survey.md` §2.3 は `f_92f1c0` の 5 番目の引数が 0 なら作らないと読んだ。`f_938440` 経由は別の呼び元) |
| **TScanThread** | `f_b7ce00` | `DeleteFileW`(深さ 2)ほか | `... > 1196c00 > b7aa30 > b7aab0 > b7ab80 > b7ce00 > b7ee20` | 静的に届く。サンプルのスキャン(ブラウザ)用と読める |
| **TMIDIThread** | `f_bb0690` | `Sleep`、`WaitForSingleObject`、`SetEvent`、`midiOutShortMsg` | `... > 1196c00 > bb0690 > bb1410` | 静的に届く。MIDI 出力を開いたとき(読み) |
| **TCloudSearchThread** | `f_da18a0` | `InternetOpenW`、`InternetConnectW`、`InternetReadFile`、`HttpSendRequestExW`、`PostMessageW` | `... > f51fd0 > f46490 > da1050 > da18a0 > db2030` | 静的に届く。ネットワーク(G11、読み) |
| **TBrowserTagDiscoveryThread**、名前不明の ctor `0x11ef180` の呼び出し点 | `f_1081a20`、`f_11efbf0` | (`f_11ef180` は読んでいない。`Execute` は前者のみ `execute_waits.tsv` にある) | `CreateFruityInstance` から tier 1 で届く: `1375990 > 1375950 > 11efbf0 > 11ef180 > 1081a20 > b8f620`(`reach_threads.tsv`)。`f_1375950` は Run の入口(`CreateFruityInstance` が 0x1376573 で呼ぶ。実測)で、`[0x15e3a78]` が指す byte が 0 なら `Application.Run`(`f_86a660`)、非 0 なら `f_11efbf0`(`goal-ladder.md` §3.4 が「`Flags` のビット 0 が立つときの別ループ」と読んだもの。この byte が `Flags` と同じものかは**未確認**)を呼ぶ(0x1375954〜0x137597d、読み)。`f_13751e0` からは `f_11be0a0 > f_11efbf0` の経由(tier 2 = `lea` の辺を足したときだけ) | 弱(byte の値次第の別ループ。通常の起動で通るかは未確認) |

直接の CreateThread(#2、#3、#6)と Blend2D(#7)は `CreateFruityInstance` の閉包に**入らない**(`reach_threads.tsv` の creator 行)。ただし閉包の外の経路(仮想呼び出し)から届く可能性は排除できない。

**他の呼び出し点 45**(`TCustomHintShowHideThread`、`TWorkerThread`、`TDropTargetTransferThread`、`TDropSourceThread`、`TCallbackThread`、`TNewsThread`、`TSampleLoaderThread`、`TBrowser*Thread`、`TAnalyseThread`、`TPreview*Thread`、`TUpdaterThread`、`TCrashLogUploadItem`、`TStreamThread` ほか)は直接 call でも `lea` でも届かない(`classreach.tsv` の `CFI_t1`、`CFI_t2` がともに 0。名前不明の 13 点のうち 12 点を含む)。UI の操作・書き出し・更新確認・ネットワークなどの機能に伴うスレッドと読める。

### 4.3 壁 21 の直後に作られると**読める**スレッド(強い根拠)

**停止点の経路**(実測、`stop21-record.txt` のシャドウスタック): `f_136cba0`(`TAboutForm` の生成: クラス参照 `0x136c750` の RTTI が `TAboutForm`。決定 0121 は「スプラッシュ画面のフォームのコンストラクタ」と読んでいる。**`TAboutForm` と読めたのが RTTI の実測で、スプラッシュとの関係は未確認**)→ `f_722200` → `f_721bb0` → **`call [rbx+0xb0]`**(仮想)→ **`f_71e900`** → `f_736ff0`(QFC の `AcquireSharedAnimTimer` を `[0x15e1fc0]` から引いて `[0x15f61a8]` に保存)→ QFC `AcquireSharedAnimTimer`(壁 21)。

`f_71e900`(0x71e900)の読み(**読み**): `[rbx+0xc0]`(`TLightweightLayeredControl`、RTTI の短い断片。FLEngine の文字列表にある)が非 0 なら、(1) `f_736ff0`(共有アニメタイマーの取得)→ (2) `TLayeredControlThread` を `f_71db30(class, alloc=1, owner=rbx)` で**作り**(`CreateSuspended=True` の `f_5252d0(dl=0, r8b=1)`)、`f_525ab0(dl=2)`(`SetThreadPriority`、FLEngine の優先度表 `0x13bd8c8`)を呼び、(3) **直後に `f_526500`(Start = Resume)**(0x71e954)を呼ぶ。`[rbx+0xc0]` が 0 なら `f_71e020` と `f_737020` の別経路(スレッドを作らない)に行く。**`[rbx+0xc0]` はフォームの旗で、`f_136cba0` が `f_71e8f0(dl=1)`(0x71e8f0 = `mov [rcx+0xc0],dl`)で 1 を書く**(0x136cbff、`f_7222a0` で作った `0x196ee28` のオブジェクト)。したがって**このフォームの表示では 2 本とも作られる**(読み。停止点が `f_71e900` を通過した実測と整合)。

| # | スレッド | 作る関数 | 本体の読み | 待つもの | 共有する状態 |
|---|---|---|---|---|---|
| 1 | `TTimerThread`(QFC。`TSharedAnimTimer` が所有) | QFC `f_1e85eb0` ← `AcquireSharedAnimTimer`(最初の 1 回だけ。2 回目以降は参照カウント `[+0x18]` を増やすだけ) | §5 | `DwmFlush`、`Sleep` | 所有者の `TSharedAnimTimer`(`[+0x48]`)、窓への `PostMessageW(0x8ffc)` |
| 2 | `TLayeredControlThread`(FLEngine `0x71d588`。**フォームのレイヤード窓用**) | `f_71db30`(`f_71e900` から) | `f_63a660`、`f_63a6d0`(FPU/MXCSR の設定)→ `PeekMessageW`(`PM_NOREMOVE`、キューを作る)→ `f_71e080`(**`CreateWindowExW(ex=0x80800ac, ..)` でレイヤード窓を作る**。この窓は**スレッド側の窓**)→ **`GetMessageW` / `TranslateMessage` / `DispatchMessageW` のメッセージループ**(`GetMessageW` が 0 = `WM_QUIT` まで)→ 終了時 `f_71e170`(`DestroyWindow`) | **`GetMessageW`(ブロッキング。FLEngine 全体で唯一の `GetMessageW`。呼び出し点は `0x71dae4` の 1 つ。`api_counts.tsv`)** | スレッド側の窓とそのメッセージキュー。`f_71e990`(`f_1369240` から呼ばれる)は **`Sleep(0)` で `[obj+0xc4]->[+0x1d]`(Finished)を待つ旋回**(`f_81c150` = `Sleep(0)`)。`f_71e9e0` は `PostMessageW(窓, 0xb021)` |

**2 本目はスレッド側の窓とメッセージループを持つ**: 単一スレッドの runtime の窓表(決定 0071、0119、0121)は、`CreateWindowExW` を**呼んだスレッド**を所有者とする仮定(「窓は最初の呼び出しのスレッドだけ」)で書かれている。この窓を作るのは別スレッドなので、**窓表とメッセージキューのスレッド別化**(`PostMessage`/`SendMessage` が他スレッドの窓に送る場合、Learn の SendMessageW は「窓が別のスレッドで作られたなら、システムはそのスレッドに切り替えて窓手続きを呼ぶ。受信スレッドがメッセージ取得コードを実行するまで処理されない。送信スレッドは受信スレッドが処理するまでブロックされる」と書く)が要る。

### 4.4 G1〜G4 でスレッドの本物が要るか(読み)

- 壁 21 のタイマースレッドと `TLayeredControlThread`(#2)は、**フォームを表示するとき(`Show`)に作られる**ので、`CreateFruityInstance` がフォームを作って表示する経路(G2 の可視ウィンドウ)で必ず通る。**この 2 本は G1/G2 の経路の上にある**(読み)。
- ミキサー(DS/ASIO)、`TWAVStreamingThread`、`TFileDeletionThread`、`TMIDIThread` などは、音声デバイス・プロジェクトの読み込み・MIDI の開始で作られる。`TMixerThread_DS` の生成は `f_12afaf0` に入ることが静的に分かるが、**実際に G1 の途中で通るかは `DirectSoundCreate` 以前の条件(レジストリ値など)による。未測定**(`goal-ladder.md` §3.4 の読みを引用)。
- したがって **G4 の最初のフレームまでに最低 2 本(タイマー、レイヤード窓)が要る**(読み)。**G7(音声)で DS ミキサー、G8(再生)でミキサーとディスク(`TWAVStreamingThread`)が加わる**(読み)。

## 5. 壁 21 のスレッド本体の読み(QFC。基底 0x1d00000 の VA。ファイル上は 0x400000)

### 5.1 `lpStartAddress` `0x1d0fdb0` は Execute ではなく Delphi RTL の ThreadWrapper

依頼文と決定 0121 の「`0x1d0fdb0`(QFC の Delphi のスレッド起動の手続き)」「スレッドの本体(0x1df0ee0 → `Execute`)」を、逆アセンブルで次のように整理し直した(**決定 0121 の読みの訂正を含む**)。

| 段 | 関数 | 読み |
|---|---|---|
| 開始 | `0x1d0fdb0`(= `lea r8,[rip-0xcb]` の対象。`f_40fe20` の中で定数) | RTL の汎用 ThreadWrapper。`ldmxcsr`(`f_1d0c850`)でレコード `{proc,param}`(`BeginThread` が 16 バイトを確保して `[+0]=proc, [+8]=param` を書いたもの)を読み、**`call [rbp+0x38]` で `proc(param)` を呼ぶ**。戻り値は `[rbp+0x4c]` に取り、例外は `f_1d0fd60`/`f_1d0f290`。 |
| 起動手続き | `0x1df0ee0`(= `f_1df10f0` の `lea r8` 引数、**`BeginThread` の `proc`**。`this` = TThread を `param` にして渡す) | `TThread.ThreadProc`(RTL。**FLEngine の `f_5251b0` と同形**)。§3.5 のとおり `Started := True`、`Terminated` が 0 なら **`call [vmt+0x10]` = `Execute`**、`Finished := True`、`SetEvent(SyncEvent)`(`f_1df0b10`)、FreeOnTerminate なら解放、`EndThread(ReturnValue)`(`ExitThread`) |
| **本体** | `TTimerThread.Execute` = VMT `0x1e85598` のスロット +0x10 = **`0x1e85bd0`** | 下 |

`TTimerThread` の VMT は `0x585598`(測定 `0x1e85598`)で、RTTI の短い断片は `TTimerThread`、親は `TThread`(`07 0c "TTimerThread" ...` 、実測 `rtti_names.py`)。クラス参照 `0x1e854d0` が指す。`TSharedAnimTimer`(VMT `0x5856d8`)はこのスレッドを所有するオブジェクトで、`AcquireSharedAnimTimer`(`0x1e864f0`)が最初の呼び出しで `f_1e85c90`(コンストラクタ)で作り、`f_1e85eb0`(起動)でスレッドを開始する。2 回目以降は参照カウント `[+0x18]` を増やすだけ(`f_1e86540`)。

### 5.2 `TTimerThread.Execute`(0x1e85bd0)の読み

```
Execute:                       ; 命令は抜粋(合計 10 行)
  call f_1e1bf80               ; fnclex; fldcw [0x1ec5458] (= 0x033F); emms   ← FPU 制御ワードの設定
  call f_1e1bff0               ; ldmxcsr [0x1ec5454] (= 0x9F80)                ← MXCSR の設定(FTZ ビット 0x8000 を含む)
  call f_1e85b70(self-frame)   ; 1 周分の処理
loop:
  ... (Terminated = [self+0x1a]) == 0 の間、
  call f_1e860a0([self+0x48])  ; 所有者への tick(下)
  call f_1e85b70
```

- **`f_1e85b70`**(1 周分): 所有者(`TSharedAnimTimer`、`[self+0x48]`)の `[+0x70]`(=「間隔(ms)」を表す `int`)を読み、**負なら `Sleep(−値)`(`f_1df1db0` → `kernel32!Sleep`)、正なら `Sleep(値)`、その後 `DwmFlush`(`dwmapi!DwmFlush`、`0x1e48ad0` の遅延 thunk)を呼び、戻りが 0(`S_OK`)でなければ(= DWM が使えない場合)`[+0x6c]`(別の間隔)と `[+0x70]` の差だけ追加で `Sleep`**。**待つ API は `Sleep` と `DwmFlush` の 2 つだけ**。`DwmFlush` は Learn で「次の Present の呼び出しまで呼び出し元をブロックする」(DwmFlush ページ、2026-10-30)。
- **`f_1e860a0`**(所有者への tick): `[obj+0xc] != 0` なら `[obj+8] += 1`(取りこぼしの数えあげ)で戻り(= 前の tick がまだ処理されていないので、メッセージを重ねて投げない)。そうでなければ `[obj+8]` を `ecx` に取り、`[obj+8] = 0`、`[obj+0xc] = 1` にして **`PostMessageW(hwnd = [obj+0x10], msg = 0x8ffc, wParam = 取りこぼし数, lParam = obj)`**(`f_1d1e0c0` = `PostMessageW` の thunk)。続いて `[obj+0x58]->[+0x10]`(購読者のリストの数)が 0 より大きければ、`EnterCriticalSection([obj+0x30])` → 購読者のコールバック `{関数, 引数}` を**末尾から**順に `call [r13]`(`rcx = [r13+8]`)→ `LeaveCriticalSection`。**つまり `PostMessageW` で窓に通知し、購読者のコールバックをこのスレッドから直接呼ぶ**。
- 窓 `[obj+0x10]` = **`TSharedAnimTimerWindow`**(壁 19・20 で作ったメッセージ専用窓。`f_1e85950` が `GetClassInfoW`/`RegisterClassW`/`CreateWindowExW(hWndParent = HWND_MESSAGE = −3)` で作り `[0x1eda6f0]` に保存)。窓手続き `0x1e858e0` は **`WM_USER+0x0FFC`(0x8ffc)なら `f_1e86280`(受信側の処理)を呼び、`[rsi+0xc] = 0`(= 次の tick を許す)にして戻る**。`[rsi+0x74]`(wParam)を取りこぼし数として保存する。それ以外は `DefWindowProcW`(壁 20 の応答済み)。
- **`f_1e86280`**(窓手続きから呼ぶ側。**メインスレッドで動く**): `[+0x68] = 1`、`[+0x60]` のリスト(メインスレッド用の購読者)を順に `call [r14]`、その後 `[+0x68] < 0` なら逆順に `[+0x8]` が 0 の要素を削除、`[+0x68] = 0`。

**(a) 待つ API とミリ秒**: `Sleep(|[owner+0x70]|)` と `DwmFlush`。**`[owner+0x70]` の値は `f_1e857b0`(`f_1e85eb0` が 0x1e85ee6/0x1e85eff で呼ぶ。引数 `ecx = |[+0x6c]|`(`[+0x6c]` の初期値は −1 なので 1)、戻りが `[+0x70]` の元)が決める**(読み。0x1e85edb〜0x1e85f5d を読んだ): `f_1e857b0` は、`f_1e48b00`(`DwmIsCompositionEnabled` を `[0x5d99d8]` から呼ぶ。関数ポインタが NULL なら 0 を返す)が真のとき、`f_1e48b40`(`DwmGetCompositionTimingInfo` を `[0x5d99c8]` から呼ぶ。NULL なら `E_NOTIMPL` = `0x80004001` を返す)で `DWM_TIMING_INFO` を取り、その中の 2 つの値(`[rbp+0x100]` と `[rbp+0x104]`。`0x1e857fb`〜`0x1e85812`)から周期を求める(`ceil` 相当の丸め)。取れなければ(戻りが 0 でなければ) `EnumDisplaySettingsW(NULL, ENUM_CURRENT_SETTINGS, …)` の `dmDisplayFrequency`(runtime の応答は **60**。決定 0098)が **50 以上なら `esi = −round(1000 / freq × (n+1) − 0.5)`**(`0x5858c8` = 1000.0、`0x5858c0` = −0.49999999999999994。丸めは `cvtsd2si`)を返す。**戻りが負なら `[+0x70]` にそのまま入れて終わり**(60 Hz、n=1: 1000/60 × 2 = 33.3 → **`[+0x70] ≈ −33`**。値の丸めの細部は未確認)。負でなければ別の式(`(round(|[+0x6c]| / [0x1eda6e8]) − 1) × [0x1eda6e8]`、正なら `+ [0x1eda6e8]/2`)。`Execute` の `f_1e85b70` は `[+0x70]` が負なら `Sleep(−値)` を呼び、**続いて `DwmFlush`**(戻りが 0 でなければ `[+0x6c]` との差を `Sleep`)。**runtime の現行 `dwmapi` は「未ロード」扱い**(`fle_load.c` の一覧: `GetModuleHandleW` は 126。実測 `grep`)で、`QFC` は `[0x5d99d8]`/`[0x5d99c8]` を `LoadLibraryW(L"dwmapi")` + `GetProcAddress`(UTF-16 の名前 `DwmIsCompositionEnabled`、`DwmGetCompositionTimingInfo`。実測)で埋める(`f_548e00`)ので、`LoadLibraryW` の応答が失敗なら両ポインタは NULL のまま、`DwmIsCompositionEnabled` 側は 0 を返し、**`EnumDisplaySettingsW` の分岐に行く**(読み)。一方 `DwmFlush` は QFC の**遅延 import**(`.didata`、記述子 `DWMAPI.DLL`。スロット(測定 VA `0x1edd118`)の初期値は遅延ロードのヘルパー `0x1e48ac0`(`lea rdx,[…]; jmp 0x1e48a60`)を指す。実測)で、`quickfontcache-lift-feasibility.md` が「`DwmFlush`(行なし)」「遅延 import は `jmp [IAT]` のスタブを持たず、ヘルパー経由で `GetProcAddress` に届く」と書く。**`DwmFlush` を呼ぶ `f_1e85b70` は、上の分岐と関係なく毎周回呼ぶ**(0x1e85b9e)ので、スレッドが実際に走ると、遅延ロードのヘルパー(`f_1e48a60`)が `dwmapi` を読もうとする。`dwmapi` は runtime で「未ロード」(`GetModuleHandleW` が 126)なので、失敗時の経路(同ドキュメントは `RaiseException(0xc06d007e)` 系と読むが、**ヘルパーの中身は読んでいない = 未確認**)に入る。**したがってスレッドを走らせる設計にしたとき、`dwmapi.dll` を仮想モジュールにして `DwmFlush` に答える必要がある**(読み)(**`DwmFlush` は Learn で「次の Present まで呼び出しをブロックする」。ブラウザには同等物が無い。§7**)。**`MsgWaitForMultipleObjects`/`PeekMessageW` は `Execute` の中に無い**(`execute_waits.tsv` の `TTimerThread` 行は QFC にあるので別表。FLEngine 側のクラスの表には含まれない。直接 call 深さ 3 の読み: `f_1e860a0` の `PostMessageW`、`f_1df1db0` の `Sleep`、`f_1e48ad0` の `DwmFlush`、`EnterCriticalSection`/`LeaveCriticalSection`)。決定 0121 の「`MsgWaitForMultipleObjects(2, handles, FALSE, 1000, 0x40)` と `PeekMessageW` のループ(0x1df1f51、0x1df1f70)はスレッドの本体」は**誤り**で、その命令は `TThread.WaitFor`(`f_1df1ee0`)の中(`WaitFor` を呼んだスレッドがメインスレッドのときの待ち)。**ただし**、決定 0121 の結論(スレッドの側にメッセージ待ちがあり、`CreateFruityInstance` の側に無い)は、`Execute` の実体が `Sleep`/`DwmFlush` と `PostMessageW` であるという別の形で成り立つ。
**(b) 他スレッドと共有する状態**: ① `TSharedAnimTimer` オブジェクトの `[+0x08]`(取りこぼしの数)、`[+0x0c]`(tick 待ちの旗。窓手続きが 0 に戻す)、`[+0x10]`(窓ハンドル)、`[+0x20]`(スレッドオブジェクトへの参照。デストラクタが 0 を待つ)、`[+0x2c]`(動作中)、`[+0x58]`、`[+0x60]`(購読者のリスト 2 本)、`[+0x68]`、`[+0x6c]`/`[+0x70]`(間隔)、`[+0x74]`。② クリティカルセクション `[obj+0x30]`(購読者リストの保護。QFC の `InitializeCriticalSection` を `f_1e85c90` が呼ぶ)。③ グローバル: `[0x1eda6f0]`(メッセージ専用窓)、`[0x1eda6f8]`(共有タイマー本体)、`[0x1eda6e8]`(`f_1e857b0` が決めた間隔)、`[0x1eda6e4]`(`timeBeginPeriod` の参照カウント)。④ Win32 オブジェクト: `PostMessageW` が窓のキューに積む(窓を作ったのはメインスレッド。Learn の PostMessage: 「窓を作ったスレッドのメッセージキューに投稿し、処理を待たずに戻る。キューは 1 つに 10,000 件まで」)。
**(c) 終了条件**: `Terminated`(`[+0x1a]`)が 1 になること(所有者のデストラクタ `f_1e85fc0` が `Terminate`、`f_1df1ea0` を呼ぶ)。**スレッドは `Sleep`/`DwmFlush` から戻らないと `Terminated` を見ない**ので、`DwmFlush` が長くブロックすればデストラクタの 1000 ms の打ち切りに当たる。所有者のデストラクタの待ち: `Sleep(0)` + `GetTickCount` で最大 1000 ms、スレッド側が終了時に `DoTerminate`(VMT +0x00 の override `f_1e85c20` = `[owner+0x20] := 0`)を呼ぶのを待つ。1000 ms 経っても終わらないと待つのをやめて先に進む(`jb 0x585fef` のループが抜ける)。その後 `f_1e85a70`(`PeekMessageW` で自分の窓の `WM_QUIT(0x12)` 以外を捨て、`PostQuitMessage` を再投稿する掃除)→ `[+0xc] = 0` → 仮想メソッド `[vmt+0x38]`。

### 5.3 「誰がいつ ResumeThread を呼ぶか」(壁 21 の経路の整理)

1. `f_1e85eb0`(0x1e85eb0)が `f_1e85fc0`(デストラクタ側の掃除。スレッドがまだ無いので `[+0x20] == 0` で何もしない)→ `f_1e85950`(窓。壁 19・20 で通過済み)→ `[+0x10] = 窓` → 仮想呼び出し `[vmt+0x38]`(`TSharedAnimTimer` では `[+0x08] := 0` だけ。0x1e85c40)→ `f_1e857b0`(間隔の決定。`EnumDisplaySettingsW`、壁 21 の直前の import thunk)→ `f_1df1000(TTimerThread, alloc=1, CreateSuspended=1)`(**壁 21: `CreateThread`**)。
2. 戻ったあと(`CreateThread` を通した場合): `[self+0x20] := スレッド`、`f_1df1750(dl=1)`(`SetThreadPriority(handle, table[1])` = THREAD_PRIORITY_LOWEST(−2)。Learn の SetThreadPriority: 優先度値の一覧)、`[thread+0x48] := self`(所有者)、`f_1df1790(dl=1)`(FreeOnTerminate := True)、**`f_1df1dd0`(Start → `f_1df1640(dl=0)` → `ResumeThread`(0x1df1667)**。戻り値が 1 でなければ例外)。
3. したがって **`ResumeThread` を呼ぶのは `AcquireSharedAnimTimer` の呼び出しスレッド自身で、`CreateThread` の直後**。誰かが後で呼ぶのではない。

**`SetThreadPriority(handle, …)` は runtime の現行実装が `GetCurrentThread` の疑似ハンドル(`0xFFFFFFFFFFFFFFFE`)以外を名前付きで停止する**(`fle_proc.c` の `SetThreadPriority`)。新しいスレッドのハンドルを `CreateThread` が返す設計にすると、ここも直す必要がある。

## 6. スレッドモデルの選択肢と評価

### 6.1 評価に使う事実(出典つき)

**スレッドの需要(§4、§5)**:
- G1〜G4 の入口で**確実に要る**のは 2 本(`TTimerThread`、`TLayeredControlThread`)で、**どちらも `Execute` が待ち(`Sleep`+`DwmFlush`、`GetMessageW`)のループ**。壁 21 の `TTimerThread` は呼び出しスレッドが `ResumeThread` の直後に自分のメッセージ専用窓の `PostMessageW` を受ける側になる。
- G7 以降: `TMixerThread_DS`(`WaitForMultipleObjects(INFINITE)`、`timeGetTime`)、`TWAVStreamingThread`、`TFileDeletionThread`(`TEvent.WaitFor(INFINITE)`)。

**既存 runtime の前提**(読み。`lift/runtime`):
- ゲストの状態はすべて**グローバル**: `R[16]`(`fle_runtime.h:12`)、`X[16]`、`YH[16]`、`FL`、`fle_x87_r[8]`、`fle_x87`(制御ワード。`fle_x87.c`)、`fle_mxcsr`(`fle_mxcsr.c:8`)、`fle_teb[0x2000]`(`fle_vex.h`)、`fle_tls_values[1088]`(`fle_imports.c`)、`fle_last_error`(`fle_imports.c:70`)。`fle_call_guest` の深さ `depth`(`fle_callback.c:11`)、`fle_landing_top`(`fle_dispatch.h:75`)。優先度・背景モードは `fle_proc.c:105-107` の static。
- **`__thread` はロケールの 1 つだけ**(`fle_locale.c:52`、`fle_thread_lcid`)。
- **単一スレッドの規則**(決定 0017、0054 ほか): 各ファイルが `fle_host_tid()` を最初の呼び出しに記録し、別の値からの呼び出しを `fle_stop`。**14 ファイル**(`fle_class`、`fle_event`、`fle_gdc`、`fle_gdi`、`fle_glob`、`fle_imports`、`fle_ole`、`fle_oleaut`、`fle_proc`、`fle_sys`、`fle_sysinfo`、`fle_vmap`、`fle_vol`、`fle_win`。`grep -l fle_host_tid`)。wasm では `fle_host_tid()` が定数 1(`fle_host.h:25`、決定 0045)。ゲストに見せるスレッド ID は固定の 0x1001(決定 0056)。**この規則を固定するテストがある**(`crates/fe-lift/tests/` の 16 ファイルが「second thread / another host thread」を検査。`tid.rs` の `guest_thread_id_is_the_same_from_another_host_thread` は決定 0056 が「第 2 スレッドを作る変更で意図して失敗する」と書く)。
- 生成器: ゲストの `call` は C の `f_XXXX();`(戻りアドレスは `R[4]` 経由でゲストのスタックに積む)、間接 call は `fle_dispatch`。C のローカルは call をまたいで生きていない(`browser-event-loop.md` §5.2 の引用)。`lock` 接頭辞は**無視して通常の読み書き**(§2)。
- 同期オブジェクトのハンドル領域: `0x7FFA80000000`(イベント。決定 0050)など。`0x7FFAA0000000` 以上と `0x7FFA30000000`、`0x7FFA40000000`(決定 0024 が他担当の予約候補として空けた)は runtime のどこでも使われていない(`grep`)。`CloseHandle` のフック枠は 4 のうち 3 使用(`fle_event`、`fle_vmap`、`fle_vol`)。
- 時計: 仮想時計。`Sleep(ms)` は `ms * 10,000` tick 進めるだけ(`fle_sys.c`)、`WaitForSingleObject` は時計を進めない(決定 0050)。時計を読むたびに 10 tick(1 µs)進む。

**実測済みの機構**(引用。再測定していない):
- JSPI + `SUPPORT_LONGJMP=wasm` で、a4 の完走(1,007,942 入口)を全入口で中断しても一致(`jspi-real-wasm.md`)。JSPI のスタックは 984 kB が既定で、a4 の経路は 150 kB で通る。**実エンジンのメッセージループの深さは未測定**。JS フレームを間に挟めない制約(`SuspendError`)があり、emscripten 既定の SJLJ は JSPI と共存できない。
- JSPI の中断・再開は深さにほぼ依存せず約 0.1〜0.2 µs(`browser-event-loop.md` §4.2)。Asyncify は実 wasm(389 MB、`-O0`)への外挿で約 722 MB(V8 上限の 67%)。
- 中断中の JS ハンドラは**中断中のゲストのグローバル状態の上で動く**ので、ハンドラはキューへの書き込みに限る(`browser-event-loop.md` §4.3、`sync-thread-survey.md` §3.2 の probe が `R[3]` の上書きを実測)。
- `sync-thread-survey.md` §3.2 の probe: JSPI は wasm のネイティブスタックしか切り替えず、線形メモリ上の C スタック(shadow stack)はスレッドごとに分けて `stackSave`/`stackRestore` が要る(probe3)。2,000 本を同時に中断して RSS +30 MB(probe4)。

**この調査で取得した外部の文言**(2026-10-30〜31):
- V8 `jspi` ブログ: 「JSPI is currently phase 4 in the W3C WebAssembly WG … available in Chrome 137, and in Firefox 139」、「suspending and resuming … essentially constant time」。**Safari は本文に無い**(`sync-thread-survey.md` は「Safari 27」と書くが二次情報で、この調査では確認していない)。
- Emscripten `pthreads.html`: 「Browsers that have implemented and enabled SharedArrayBuffer, are gating it behind … COOP and COEP headers. Pthreads code will not work in deployed environment unless these headers are correctly set」、「It is not possible to build one binary that would be able to leverage multithreading when available and fall back to single threaded」、「pthread_create() … if we need to create a new Web Worker, then that requires returning the main event loop … This is a violation of POSIX behavior and will break common code which creates a thread and immediately joins it」、「The Web API for atomics does not allow blocking on the main thread (specifically, `Atomics.wait` doesn't work there)」、「The Emscripten implementation does not support POSIX signals」。
- MDN: 共有メモリには「secure context and cross-origin isolated」が要る。`Atomics.wait` は「blocking and cannot be used in the main thread」。
- WebAssembly threads 提案: 原子操作は `i32`/`i64` の `rmw`(`add/sub/and/or/xor/xchg/cmpxchg`)で、**128 ビットの原子操作は無い**(提案本文の `atomicop ::= add | sub | and | or | xor | xchg | cmpxchg` と表。`cmpxchg16b` に当たる命令は無い)。原子アクセスは**逐次一貫**(提案本文:「Currently all atomic memory access instructions are sequentially consistent」)。
- Learn: `SendMessageW`「If the specified window was created by a different thread, the system switches to that thread and calls the appropriate window procedure. Messages sent between threads are processed only when the receiving thread executes message retrieval code. The sending thread is blocked until the receiving thread processes the message」、`GetLastError`「maintained on a per-thread basis」、`TlsAlloc`「each thread receives its own slot for the index」、`DllMain`「DLL_THREAD_ATTACH … The current process is creating a new thread. When this occurs, the system calls the entry-point function of all DLLs currently attached to the process」、`CreateThread`「If the thread is created in a runnable state … the thread can start running before CreateThread returns」、`DwmFlush`「blocks the caller until the next call to a Present method」。

### 6.2 選択肢の表

凡例の「差」は**実機で観測できる差**。規模は桁だけ(行数: S = 〜数百、M = 数千、L = 万以上。日数は書かない)。

| | T0 止まる(現状) | T1 作ったことにするだけ | T2 協調スレッド | T3 実スレッド | T4 コールバック同期実行 |
|---|---|---|---|---|---|
| **内容** | `CreateThread` は名前付き停止(`fle: unsupported instruction`)。決定 0121 の Z1 | `CreateThread` がハンドルを返し、`ResumeThread` は前の中断数 1 を返し、スレッドは**一度も走らない**(決定 0121 の Z2) | スレッドごとに独立した C スタックと状態を持ち、**待つ API(`WaitForSingleObject`、`Sleep`、`MsgWaitForMultipleObjects`、`GetMessageW`、`DwmFlush` ほか)でだけ**切り替える。決定的なスケジューラ(ラウンドロビン、仮想時計) | native は pthread、wasm は Worker + SharedArrayBuffer + `-pthread` | `ResumeThread` で `fle_call_guest` を呼び、最初の待ちか終了まで同期実行して戻る。待ちに入ったら止まる(名前付き停止) |
| **実機との差** | 先に進めない(差を測る以前) | **観測できる差が多い**: ① `TThread.Finished`(`[+0x1d]`)を書く者がいない。デストラクタ(`f_1df1430`)は `while Started == 0: SwitchToThread` を回すので、`Started`(`[+0x18]`)が立たないと**永遠に回る**(`SwitchToThread` は 100 万回で停止)。② 所有者のデストラクタ(`f_1e85fc0`)は `[+0x20] != 0` の間 `Sleep(0)` + `GetTickCount` で 1000 ms 待つ。仮想時計は `Sleep(0)` で進まないが、`GetTickCount` の読みで 1 µs ずつ進むので 1 秒ぶんの読み(約 100 万回)で抜ける。③ `TTimerThread` が `PostMessageW(0x8ffc)` を送らないので、**購読者のコールバックが一度も呼ばれない**(アニメーションの tick が来ない)。④ `TLayeredControlThread` の窓がスレッド側で作られず、`f_71e990`(`f_1369240` から。`[obj+0xc4]->[+0x1d]` を `Sleep(0)` で待つ旋回)は `Finished` が立たないので**永遠に回る**(読み。この関数に届く条件は未確認、§4.3) | **最小**: Windows の「同時に走る」を実機のようには再現しないが、**実行する命令列は同じ**(スレッド本体はゲストのコードをそのまま走らせる)。差は(a) スレッドの切り替え点が待ちの API に限られる(実機は任意の命令の間で切り替わる)、(b) スケジューリングの順序・時間(仮想時計)が実機と違う | **同時性の意味は最も近い**(実機同様の競合が起きる)が、**結果が実行ごとに変わる**。`lock` 接頭辞を無視した現行の生成 C は**不正確**(§6.3 T3) | ① スレッドが `Execute` から戻る(`Terminated` まで走る)までは呼び出し元に戻らないので、`TTimerThread`・`TLayeredControlThread` では**最初の待ち(`Sleep`/`DwmFlush`/`GetMessageW`)で止まる**。② `ResumeThread` の時点で実行すると、実機で `ResumeThread` の戻りより後に起きるはずの副作用が先に起きる |
| **規模** | なし | S(runtime に 10 個程度の API 行) | **L**: スレッド表、スタック切り替え、ゲスト状態の退避/復元、待ち/起床、既存 14 ファイルの単一スレッド規則の置き換えとテスト 16 ファイルの更新、窓表・メッセージキューのスレッド別化(§7.4) | **L〜最大**: 全オブジェクトの `-pthread` 再ビルド、`R`/`X`/`YH`/`FL`/x87/MXCSR/`fle_teb`/`fle_tls_values`/`fle_last_error` の thread-local 化(生成 C のグローバル参照すべて)、`lock` 付き命令の `__atomic` 化(生成器の変更)、`cmpxchg16b` の 128 ビット原子の扱い、14 ファイルの `owner_tid` を本物のロックに(`sync-thread-survey.md` §3.3 の D の引用) | S〜M(`fle_call_guest` を使うだけ。ただし待ちの扱いと戻り先の規則が要る) |
| **wasm・ブラウザ** | 動く(止まる) | 動く | JSPI なら変換なしで掛かる(実測: `jspi-real-wasm.md`)。Chrome 137、Firefox 139(V8 ブログの文言)。**Safari は未確認**。**ネイティブ(clang の harness)側に別実装が要る**(ucontext または手書きのスタック切り替え。`sync-thread-survey.md` §3.3 の「欠点」) | **COOP/COEP が必須**(Emscripten と MDN の文言)。JSPI(Worker 方式の `Atomics.wait`)と共存可能だが別系統。`ImageData` は共有メモリの `Uint8ClampedArray` から作れない(`browser-event-loop.md` §7.1 の実測: `TypeError`)ので毎フレーム 0.15 ms 前後のコピーが要る。`-pthread` の有無は**1 つのバイナリでは切り替えられない**(Emscripten の文言) | 動く(JSPI は不要) |
| **偽りの有無** | なし | **あり**。実機でスレッドが書くフラグ(`Started`、`Finished`)・窓へのメッセージ・デストラクタが待つ状態を、**ゲストの命令を走らせずに runtime が決める**ことになる。「走らない」を明示した仮の応答にできる範囲は下の §6.3 T1 | **なし**(スレッドの命令はゲストのものがそのまま走る)。ただしスケジューリングの順序と時間は選択(仮想時計。実機の値ではない)で、その部分は「出典なしの選択」として列挙する必要がある | なし(ただし結果の非決定性は「偽り」ではなく測定上の問題) | **偽りは小さいが、範囲が狭い**(待ちに入ったら止まるので、通せるスレッドが限られる) |
| **決定 0101 の「仮の応答」の扱い** | 該当なし | **仮の応答の数が増える**(スレッドが走らないという仮定が各所に染み込む。PM が Z2 を「偽り」と呼んだ理由) | スケジューラの選択(切り替え点、仮想時計の進め方)が仮の決定になる。**実機のスレッドの結果を決めるのではなく、実機の命令列のうちの順序を 1 つ選ぶ**ので、「仮の応答」というより「許される実行の 1 つ」。報告は「A4 完走(仮の応答 N 件を含む)」の N に加算されない扱いにするか、PM が決める | 仮の応答は増えないが、結果が再現しない | 仮の応答が増える(切り替え点の選択) |
| **主なリスク** | G1 が先へ進まない | 上の ①④ の永久ループに当たる。`SwitchToThread` の 100 万回停止(`fle_proc.c`)で名前付き停止になるだけで、結局止まる | (1) JSPI のスタック上限(150 kB 〜 984 kB)が実エンジンのメッセージループの深さに足りるか**未測定**。(2) `fle_call_guest` の入れ子とスレッドの切り替えの相互作用。(3) setjmp 着地(決定 0100)が C スタックを跨ぐ。(4) 窓表・キューのスレッド別化(大) | 再現性の喪失(A5 の bit-exact、仮想時計、測定の再現性)。ビルドの全面やり直し。配信の COOP/COEP 制約。`cmpxchg16b`(FLEngine で 5 点)に対応する原子操作が wasm に無い | 実際には**壁 21 だけでも通らない** |

### 6.3 各選択肢の詳細

**T0 止まる**: 偽りなし。到達点は 41,177 入口、572 関数のまま(決定 0121)。スレッドの扱いは PM の決定待ち。G1 は進まない。

**T1 作ったことにするだけ**(**偽らずに言える範囲は、この壁では無い**と読む):

- **言える範囲(一般論)**: スレッド本体が UI の描画に必須でなく、**生成後に誰も状態を読まない**スレッドなら、「走らない」を明示した仮の応答にできる。たとえば `TFileDeletionThread`(ファイル削除の遅延スレッド)、`TCloudSearchThread`(ネットワーク)は、その機能を呼ばない限り作られず、作られても削除・検索の結果が必要になる経路は別の API(`DeleteFileW`、`InternetOpenW`)で止まる。
- **言えない範囲(この壁)**: (1) `TTimerThread`: 所有者が **`PostMessageW(0x8ffc)` と購読者コールバックの呼び出し**を期待する(§5.2)。走らなければアニメーション・再描画の駆動が来ない = **G4 のフレームの更新が来ない**。(2) `TThread` のデストラクタが `Started`/`Finished` を読む(§3.5): `Started` が立たないと `SwitchToThread` を回す。(3) `TLayeredControlThread`: スレッド側が作る**窓**が存在しない。`f_71e990` のような待ちが `Finished` を読む(§4.3。ただし到達条件は未確認)。
- したがって **T1 は「止まる」を先送りするだけで、次の壁(`SwitchToThread` の 100 万回、または `Finished` を待つ旋回)で結局止まる**(読み)。実機の挙動なしに結果を決める点でも、決定 0121 の Z2 と同じ。**採らない**(推奨 §8)。

**T2 協調スレッド**(`swapcontext`/ucontext、手書きのスタック切り替え(native)、JSPI(wasm)):

- **native**: `makecontext`/`swapcontext`(POSIX)、または手書きのアセンブリで C スタックを切り替える。ただし**生成 C のゲストの呼び出しは C の呼び出し**なので、スレッドの途中状態は C のスタック自体にある。ゲストのスタック(`R[4]` が指す線形メモリ)も別に要る。これは `sync-thread-survey.md` §3.1 の既存の分析と同じ。
- **wasm**: JSPI(`WebAssembly.Suspending`/`promising`)で wasm のスタックを切り替える。**線形メモリ上の C の shadow stack は JSPI が切り替えない**ので、スレッドごとに別の領域を割り当て、切り替えのたびに `stackSave`/`stackRestore`(probe3 の実測)。`-O0` の生成 C は 1 つのゲスト呼び出しが複数の wasm フレームになる(`sync-thread-survey.md` §3.2)ので、**スレッドのスタックの深さの上限は実コードで測る必要がある**。Asyncify は実 wasm への外挿で現実的でない(`browser-event-loop.md` §4.4)。
- **待つ API でのスイッチ**: `WaitForSingleObject`/`WaitForMultipleObjects`/`MsgWaitForMultipleObjects`/`GetMessageW`/`PeekMessageW`(空のとき)/`Sleep`/`SwitchToThread`/`DwmFlush`/`SignalObjectAndWait`。これらは Learn の仕様に沿って「待つ対象が信号になるか、時間(仮想時計)が経つまで他のスレッドを走らせる」。**ブロックするスレッドがいなければ(全員が待っている)、仮想時計を最も近い期限まで進める**(これで `Sleep(INFINITE)` と `WaitForSingleObject(INFINITE)` を除き、デッドロックが実機同様に「止まる」ことを検出できる)。
- **決定的なスケジューリング**: ラウンドロビン + 仮想時計。優先度(`SetThreadPriority` の値)は初版では無視する(出典なしの選択として列挙)。
- **runtime の変更点**(§7 に設計メモ): スレッド表、現在のスレッドの文脈(`R[16]`、`X[16]`、`YH[16]`、`FL`、`fle_x87_r[8]`、`fle_x87`、`fle_mxcsr`、`fle_teb`、`fle_tls_values`、`fle_last_error`、`fle_call_guest` の `depth`、`fle_landing_top`)の退避・復元、`fle_host_tid()` の規則の置き換え(決定 0054)。
- **JSPI 実測との整合**(`jspi-real-wasm.md`): ① 中断点は `JS の Suspending import` に限る。runtime の待ち API が `Suspending` な JS import を呼ぶ形にする。② `SUPPORT_LONGJMP=wasm` が前提(emscripten 方式の SJLJ は不可)。③ 全入口の中断で a4 が一致した実績があるので、機構としては成立している。④ 実エンジンの深さは未測定。**スレッドが増えると JSPI のスタックが同時に複数要る**(probe4: 2,000 本で RSS +30 MB。ただし各スレッドの深さは浅い probe)。
- **`longjmp`(決定 0100 の landing)との相互作用**: `fle_landing_top` は**スレッドごと**でなければならない(別スレッドの C フレームへ `longjmp` してはならない)。`fle_landing_try` は `l->sp == R[4]` で照合するので、スレッドごとのスタックを使えば誤って別スレッドの着地に当たることはないが、`fle_landing_top` の連結リストがスレッドの切り替えで混ざらないよう、スレッドごとの退避が要る。
- **`fle_call_guest` の入れ子**: `depth`(最大 16)もスレッドごと。スレッドの切り替えが `fle_call_guest` の中(窓手続きの中)で起きうる(待ちの API が窓手続きから呼ばれる)ので、`depth` と `R[4]` を一緒に退避する。
- **TEB/TLS(gs:)**: `gs:[0x30]`(Self)と `gs:[0x58]`(`ThreadLocalStoragePointer`)を、スレッドごとの `fle_teb` と `fle_tls_values` を指すようにする(`fle_gs_ld8` の `fle_teb`/`fle_tls_values` を現在のスレッドの文脈から引く)。`gs:[0x10]`(StackLimit)は `fle_teb_set_stack` をスレッドの切り替えごとに更新する(決定 0076)。**FS は生成器が LEA 以外を `Err` にする**(`vex.rs`)ので、FS を使う命令が新しいスレッドで出ても静的に分かる。暗黙 TLS(Blend2D の TLS ディレクトリ、`implicit-tls-design.md`)は**スレッドごとのブロック**が要る(`DLL_THREAD_ATTACH` の配布。**FLEngine、QFC、ILTools には TLS ディレクトリが無い**ので、FLEngine 系の 3 DLL は対象外)。`FlsAlloc` は freetype と dsp が import するが、スレッドの数に影響する使い方かは未確認。
- **`DllMain` の `DLL_THREAD_ATTACH/DETACH`**(Learn の文言は §6.1): 決定 0080 D9 は受けない設計。**`DisableThreadLibraryCalls` は 6 DLL のどれも import していない**(§2)ので、実機では新しいスレッドごとに FLEngine、QFC、ILTools の `DllMain(reason 2)` が呼ばれる。FLEngine の DllMain(`0x41d8f0`)は `reason == 1` のときだけ初期化し(`0x41d905`: `cmp r13d,1; jne 0x41d94d`)、他の reason は 0x41d94d 以降(RTL の `DLLProc` 呼び出しの引数を整える形。読み)に進む。**reason 2/3 の応答が何をするかは読んでいない(未確認)**。T2 でスレッドを作るとき、reason 2/3 を呼ぶか呼ばないかを決める必要がある(実機は呼ぶ。呼ばない場合の差を出典なしの選択として列挙する)。

**T3 実スレッド**:

- native は pthread で素直。wasm は Worker + SharedArrayBuffer(`-pthread`)。
- **ゲストのメモリは共有メモリ**になる。lifted C の `fle_ld`/`fle_st` は `memcpy`(`fle_runtime.h:66-74`)で非原子。x86 の通常の `mov` はアラインされた 1/2/4/8 バイトで原子的に見える(TSO)ので、**実機のレースと同じ意味を持たせるには**、`fle_ld/fle_st` を原子アクセス(relaxed か、wasm では逐次一貫のみ)にする必要がある(読み。x86 の TSO は wasm の逐次一貫より弱いので、wasm の逐次一貫はそれを包含する = 安全側で、遅い)。
- **`lock` 付き命令**(§2): 現行の生成器は無視して通常の読み書きにする(決定 0013 項 9)。実スレッドでは**不正確**。`lock xadd` 159、`lock cmpxchg` 52、`lock xchg` 35、`lock cmpxchg16b` 5、暗黙ロックの `xchg [mem]` 5(FLEngine)を `__atomic` に変える生成器の変更が要る。**`cmpxchg16b`(FLEngine と QFC/ILTools に各 5 点)は 128 ビットの原子操作で、wasm threads 提案に対応する命令が無い**(§6.1)。native の gcc/clang の `__atomic` は `-mcx16` で出せるが、wasm では別の実装(ロック)が要る。**決定 0023 項 5 は `cmpxchg16b` を SDM の擬似コードどおりに実装しており、LOCK は無視する(同決定の「LOCK」節: 「FLEngine が別スレッドを使わないことは、ここでも未確認の前提」)ので、現行の実装は非原子**。
- **決定性の喪失**: 実行ごとに結果が変わる。A5 の bit-exact の測定、native SHA の同一性の確認(決定 0119、0121 は「独立 2 回で同一」を検証に使った)、停止点の再現(`41,177 入口`)が成り立たない。仮想時計(決定 0017)も複数スレッドの実時間と合わない。
- **ブラウザの配信**: COOP `same-origin` + COEP `require-corp`(`browser-event-loop.md` §3.5、§10)。`crossOriginIsolated` が `true` になる(`browser-event-loop.md` 実測)。音声を SAB のリングで渡すなら、**どの方式でも**必要になる(同 §10)。
- **JSPI との関係**: `browser-event-loop.md` の M-A(Worker + `Atomics.wait`)は実測済みで、**ゲストの協調スレッドを増やせない**(`Atomics.wait` が全体を止める)。増やすなら本物のスレッド(T3)。T3 と JSPI は両立しうるが、スレッドごとに JSPI のスタックが要る。

**T4 コールバックとして同期実行**:

- `ResumeThread` の時点で `fle_call_guest(ThreadWrapper, …)` を呼び、スレッドが `Terminated` まで走るか最初の待ちに入るまで実行する。**待ちに入ったら止める**という規則。
- `TTimerThread`: `Execute` は `Sleep`/`DwmFlush` のループ。`Sleep` を「仮想時計を進めて戻る」と答えれば**止まらずにループが回り続け、`Terminated` が立たないので戻ってこない**(`ResumeThread` が永遠に戻らない)。`Sleep` の最初の呼び出しで止める規則なら、**壁 21 を通っても次の壁(`Sleep` の名前付き停止)で止まる**。
- **T4 単独は成立しない**。T4 は「最初の待ちまで走らせる」半分の仕組みで、**T2 の一部(スレッドの最初の起動)として使える**。

### 6.4 最小で足りる選択と、G7・G8 まで見た選択

| 目的 | 必要なスレッド | 最小で足りる選択 | 注 |
|---|---|---|---|
| 壁 21 を通す(G1 の入口) | `TTimerThread` | **T2 の最小形**(2 本目のスレッドを持てる、`Sleep`/`DwmFlush`/`WaitForSingleObject`/`ResumeThread`/`PostMessageW` に対する切り替え) | T1/T4 では次の壁で止まる(§6.3) |
| G1〜G2(フォームの表示まで) | `TTimerThread`、`TLayeredControlThread`(スレッド側の窓、`GetMessageW`、窓表のスレッド別化) | T2(窓表・メッセージキューのスレッド別化を含む。§7.4 が最大の部分) | `GetMessageW` の待ちが入るので、**メッセージキューの設計が先に要る**(決定 0119 の PM の判断点 2) |
| G3(メッセージループ) | 上に加えてメインスレッドの `PeekMessageW`/`WaitMessage`/`MsgWaitForMultipleObjects` | T2。メインスレッドの待ちを JS に戻す(JSPI)ので、**スレッドの切り替えとブラウザへの制御の戻りが同じ機構** | `browser-event-loop.md` M-B の延長 |
| G4(最初のフレーム) | 上と同じ(描画スレッドが G4 に必須かは未測定) | T2 | Blend2D の `CreateThread`(#7)は `thread_count` 次第で、**FL Studio が渡すかは未確認** |
| G7(音声) | `TMixerThread_DS`(`WaitForMultipleObjects(INFINITE)` + DirectSound の通知イベント) | T2(通知イベントを AudioWorklet の消費に結ぶ `SetEvent` を別の起床源として持つ) | 音声スレッドが CPU を使い続けるかは未測定(`browser-event-loop.md` §12 の 5) |
| G8(再生) | `TMixerThread_DS`、`TWAVStreamingThread`、`TFileDeletionThread`、レンダー(書き出し) | T2。**先取りが無い**ので、ミキサースレッドが長い計算を連続して行うとメインスレッドの UI が止まる(未測定) | T3 が要るかの判断材料は G8 の実測 |

## 7. runtime への入口の設計メモ(実装はしない)

T2(協調スレッド)を最終形とする前提で、**最小の差分の入口**を書く。T1・T4 の入口は T2 の部分集合なので別には書かない。`fle_win.c` や `fle_event.c` と同じく、**新しい 1 ファイル(仮に `fle_thread.c`/`fle_thread.h`)に局所化**し、`imports_table.rs` の末尾に行を足す(「行を末尾に足し、既存の疑似 VA を動かさない」決定 0015 の規則。`imports_table.rs:320`、`331` のコメント)。

### 7.1 スレッド表(ID、状態、スタック、エントリ、パラメータ、サスペンドカウント、終了コード、待ち対象)

```
thread_ent {
  vid            : 仮想スレッド ID。最初のスレッドは FLE_EMULATED_TID(0x1001。決定 0056)、以降 0x1002, 0x1003, …(決定 0056「将来の拡張点」)
  handle         : FLE_THREAD_HANDLE_BASE + 4*(i+1)  (§7.2。再利用しない。0x7FFAA0000000 を提案: 未使用と確認済み)
  state          : CREATED_SUSPENDED / READY / RUNNING / WAITING / TERMINATED
  suspend_count  : CreateThread(CREATE_SUSPENDED) で 1。ResumeThread が 1 減らし、0 になったら READY
  start, param   : lpStartAddress, lpParameter(エントリは fle_dispatch の既知の関数 VA であること。違えば名前付き停止)
  exit_code      : 終了時に ExitThread の引数またはスレッド手続きの戻り(STILL_ACTIVE = 259 の間は未終了。Learn の GetExitCodeThread)
  wait           : { kind(none / single / multi / msgwait / sleep / message), handles[], wait_all, deadline(仮想時計), wake_mask, result }
  ctx            : 退避したゲストの状態: R[16], X[16], YH[16], FL, fle_x87_r[8], fle_x87(cw, sw, tw), fle_mxcsr,
                   fle_teb(0x2000 バイトのうち 0x08/0x10/0x30/0x58 が意味を持つ。他は 0), fle_tls_values[1088], fle_last_error,
                   fle_call_guest の depth, fle_landing_top, 優先度・背景モード(fle_proc.c の static)
  stacks         : ゲストのスタック(線形メモリ。既定 1 MB を VirtualAlloc 相当の領域から。StackBase/Limit を fle_teb に反映 = 決定 0076)
                   と C のスタック(native: mmap した領域 + swapcontext、wasm: JSPI のスタックは V8 が持ち、shadow stack を別領域に)
  msgq           : そのスレッドのメッセージキュー(§7.4)
}
```

- **1 本目(`CreateFruityInstance` を呼んだ main)も同じ表の 0 番**として扱う。
- **スレッド ID は決定論的**(連番)。`GetCurrentThreadId` は現在のスレッドの `vid` を返す(決定 0056 の拡張点)。**ホストの tid は使わない**。
- **TEB の `ClientId`**(`gs:[0x48]` = UniqueThread。決定 0056 項 4)をモデル化するなら同じ `vid` を書く(現在は `gs:` の 0x08/0x10/0x30/0x58 だけがモデル化されていて、他は abort。`fle_vex.h` の `fle_gs_ld8`)。

### 7.2 API 一覧と Learn に基づく規則の概要

(Learn の文言は §6.1 と §1 の取得物。**値のうち Learn に書かれないものは「選択」**として決定に列挙する。)

| API | 規則(Learn。引用は取得した本文) | 書かれていない・選択になる点 |
|---|---|---|
| `CreateThread(attr, stack, start, param, flags, &tid)` | NULL 属性なら既定、スタック 0 なら既定、`CREATE_SUSPENDED`(4)なら `ResumeThread` まで走らない、`STACK_SIZE_PARAM_IS_A_RESERVATION`(0x10000)、`lpThreadId` が NULL でなければ ID を書く。成功はハンドル、失敗は NULL + `GetLastError`。「**CreateThread may succeed even if lpStartAddress points to data, code, or is not accessible**。…invalid なら、スレッドが走るとき例外」。スレッドは `THREAD_PRIORITY_NORMAL` で作られる。「When a thread terminates, the thread object attains a signaled state」 | ハンドルの値(選択。`0x7FFAA0000000 + 4*(i+1)`)、`lpThreadAttributes` が非 NULL のときの扱い(停止)、スタックサイズの既定値(1 MB は Learn の Remarks「By default, every thread has one megabyte of stack space」)、**フラグ 0(即時実行)のときの最初の実行時刻**(「before CreateThread returns」にも走りうる。決定的スケジューラの選択) |
| `ResumeThread(h)` | 「Decrements a thread's suspend count. When the suspend count is decremented to zero, the execution of the thread is resumed」。戻りは**以前の**中断数。0 なら中断されていなかった、1 なら再開された、1 超ならまだ中断中。失敗は `(DWORD)-1`。**TThread は戻りが 1 でなければ例外を作る**(§3.5)ので、1 を返すことが必須 | 無効ハンドルの last error(選択) |
| `SuspendThread(h)` | 中断数を増やし、戻りは以前の中断数。「primarily designed for use by debuggers」(Learn)。TThread の `Suspend` が使う | `MAXIMUM_SUSPEND_COUNT` の値(Learn は名前だけ)。**自分自身の SuspendThread**(疑似ハンドル)の扱い |
| `ExitThread(code)` | スレッドを終了し、`GetExitCodeThread` が code を返す。スレッドオブジェクトが信号になる。最後のスレッドなら「the thread's process is also terminated」。DLL の `DllMain(DLL_THREAD_DETACH)` が呼ばれる(Learn の DllMain) | DETACH を呼ぶか(§6.3 T2 の注)。`FreeLibraryAndExitThread`(Blend2D)は別 API |
| `GetExitCodeThread(h, &code)` | 「This function returns immediately. If the specified thread has not terminated … the status returned is STILL_ACTIVE (259)」。終了後は終了コード | 失敗時の last error |
| `TerminateThread(h, code)` | **どの DLL も import していない**(§2)。Learn は「dangerous function … should only be used in the most extreme cases」と書く。**名前付き停止のままにする**(推奨) | — |
| `GetCurrentThread()` | 疑似ハンドル。現行は `0xFFFFFFFFFFFFFFFE`(`fle_proc.c`、「値は Learn に無い」という注つき)。**スレッドごとに同じ疑似ハンドルで、API 側が「呼び出し元のスレッド」に解決する** | — |
| `GetCurrentThreadId()` | 現在のスレッドの `vid`(決定 0056 の拡張) | — |
| `SetThreadPriority(h, p)` | 現行は疑似ハンドル以外を停止(`fle_proc.c`)。**新しいスレッドのハンドルを受ける形に拡張**(Learn のハンドル権限 `THREAD_SET_INFORMATION`)。値の検査は現行のまま | 優先度はスケジューラに**使わない**(選択) |
| `SwitchToThread()` | 「yield execution to another thread that is ready to run … If there are no other threads ready to execute, the operating system does not switch execution … return value is zero」。**T2 では他に READY なスレッドがあれば切り替えて 1、なければ 0**。現行の「1,000,000 回で停止」は他のスレッドがある間は数えない | — |
| `Sleep(ms)` | 「zero causes the thread to relinquish the remainder of its time slice … If there are no other threads ready to run, the function returns immediately」。`INFINITE` は「should not time out」。**T2 では `ms > 0` のスレッドを WAITING(deadline = now + ms)にして他を走らせる。全員が待ちなら仮想時計を最小の deadline まで進める**。`Sleep(INFINITE)` で全スレッドが無期限に待つ状態は名前付き停止(現行の規則を一般化) | 起床の順序が同じ deadline のとき(ラウンドロビン順) |
| `WaitForSingleObject(h, ms)` | Learn の戻り値(`WAIT_OBJECT_0`、`WAIT_TIMEOUT` 0x102、`WAIT_FAILED`)。信号のオブジェクトは即返し、「The function modifies the state of some types of synchronization objects」(自動リセットイベントは非信号に、セマフォは 1 減る)。`ms == 0` は「it always returns immediately」。対象: Event、Semaphore、**Thread**(終了で信号)、Mutex ほか | 現行 `fle_event.c` は「INFINITE の非信号」と「有限の非信号」を停止していた(単一スレッドなので)。T2 ではこれを WAITING にする |
| `WaitForMultipleObjects(n, h[], all, ms)` | 「the maximum number of object handles is MAXIMUM_WAIT_OBJECTS」(Learn は名前のみ。**値 64 は Learn 本文に無く、MSDN の別ページ**)。`all == FALSE` なら信号になった**最小インデックス**を返す。`WAIT_OBJECT_0 + i` | `MAXIMUM_WAIT_OBJECTS` の数値(選択 64)。`all == TRUE` の同時取得 |
| `MsgWaitForMultipleObjects(n, h[], all, ms, mask)` | 戻りは `WAIT_OBJECT_0 + nCount` = 「New input of the type specified in the dwWakeMask parameter is available in the thread's input queue」。「PeekMessage、GetMessage、WaitMessage mark messages in the queue as old messages. Therefore, after you call one of these functions, a subsequent call to MsgWaitForMultipleObjects will not return until new input of the specified type arrives」。「can return even though no appropriate input is available and even if dwWakeMask is set to 0」(システムイベントのとき) | **キューの「新しい/古い」状態(`QS_*` のビット管理)の実装が要る**。壁 21 の `TThread.WaitFor`(`QS_SENDMESSAGE` = 0x40)はこの API を使う |
| `SetEvent`/`ResetEvent`/`CreateEventW`/`CloseHandle` | 既存(`fle_event.c`、決定 0050)。「SetEvent: 自動リセットは**待機中のスレッド 1 つを解放するまで**信号のまま」。T2 では**待機者がいれば 1 つを起こして非信号に**する(現行は「待機者がいない」前提) | `fle_event.c` の「`fle_stop`(待てない)」を WAITING への遷移に置き換える |
| `CreateSemaphoreW`/`ReleaseSemaphore` | QFC の `AcquireSharedThreadBank` が使う(`f_584c20`)。Learn: 「The state of a semaphore is signaled when its count is greater than zero and nonsignaled when it is zero」、`ReleaseSemaphore`「If the specified amount would cause the semaphore's count to exceed the maximum count … the count is not changed and the function returns FALSE」。**QFC のバンクのワーカー数は `GetSystemInfo.dwNumberOfProcessors` で決まり、runtime は 1 を答える**(決定 0017、`fle_sys.c` の `nproc = 1`。QFC `f_5853f0` が `max(n, 1)` を `[0x5da6a0]` に保存 → `AcquireSharedThreadBank` の `f_584c20` は `cmp eax,1; jbe 0x584d2e` で、`n <= 1` ならワーカー(`CreateSemaphoreW` と `TSTBThread` の生成)を**作らない**。読み。**FLEngine は `f_93f160` で `AcquireSharedThreadBank(1)` を呼ぶ**(`[0x15e1160]` 経由、0x93f168 `mov ecx,1`。`CreateFruityInstance` の閉包にある。実測)ので、**現行の応答(プロセッサ数 1)ではバンクのスレッドは作られない**。FLEngine 側の `GetSystemInfo`/`GetNativeSystemInfo`/`GetLogicalProcessorInformation` の利用(各 2〜4 点、`f_41ca30` は 1 点の呼び元 `f_41ce10` が仮想呼び出しからしか届かない)が、ほかのスレッド数を決めているかは**未確認**) | **プロセッサ数を 1 のままにすれば、バンクのスレッド(`TSTBThread`)は作られない**(`f_584c20` の `cmp eax,1; jbe 0x584d2e`)。この応答を変えると、スレッドが作られる(決定 0017 の変更になる) |
| `WaitMessage`/`GetMessageW`/`PeekMessageW`/`PostMessageW`/`SendMessageW`/`PostQuitMessage` | §7.4 | §7.4 |
| `DwmFlush` | 「blocks the caller until the next call to a Present method」。**ブラウザに Present は無いので、`requestAnimationFrame`(または仮想時計の 1 フレーム = 1000/60 ms)で解く選択**になる(出典なしの選択) | 戻り値(`S_OK`)、`dwmapi.dll` を仮想モジュールにする(§5.2) |
| `TlsAlloc`/`TlsGetValue`/`TlsSetValue`/`TlsFree` | Learn: 「each thread receives its own slot for the index」。`fle_tls_values` を**スレッドごと**に。現行は 1 つ(`fle_imports.c:69`)で、別スレッドを `fle_tls_check_thread` が停止 | 新スレッドのスロットは 0 で始まる(Learn: 「The slots for the index are initialized to zero」) |
| `GetLastError`/`SetLastError` | 「per-thread basis」(Learn)。`fle_last_error` をスレッドごとに | — |
| `InitializeCriticalSection` 系 | 現行は「最初の呼び出しのスレッドだけ」で、別スレッドを停止(決定 0017)。**T2 ではクリティカルセクションが本当に競合しうる**(切り替えは待ちの API でだけ起きるので、`EnterCriticalSection` の中では切り替わらない = **ロックを保持したまま別スレッドが走ることは、保持者が `Sleep`/`Wait` を呼ぶ間だけ**起きる)。競合したら待ち(WAITING)。Learn の「所有しない Leave は別スレッドを永遠に待たせうる」は現行の停止を維持 | 再入カウントは現行の表を保つ。**所有スレッドの記録を足す** |
| `SRWLock`/`ConditionVariable`(Blend2D) | 決定 0092 の表(アドレスをキー)。T2 でも同じ形で、**競合したら WAITING**。Blend2D のスレッドプール(#7)を使うなら要る | Blend2D が `thread_count` を渡すかは未確認 |
| `CloseHandle(スレッド)` | ハンドルを閉じる。スレッドオブジェクトは最後のハンドルが閉じ、かつスレッドが終了すると消える(Learn の CreateThread「The thread object remains in the system until the thread has terminated and all handles to it have been closed」)。`fle_vfs_add_close_hook` の枠は 4 のうち 3 使用なので **4 つ目が最後**(これ以上は枠を増やす) | — |

### 7.3 決定 0054(単一スレッド規則)との整合の変更点

- 現行: 14 ファイルが `fle_host_tid()` を最初の呼び出しの値として記録し、別の値を `fle_stop`。wasm は定数 1。
- **T2 では全ゲストスレッドが同じホストスレッド(wasm は単一、native は 1 本)で走る**ので、`fle_host_tid()` は**変わらない**(JSPI は同じ JS スレッドの上でスタックを切り替えるだけ)。つまり **14 ファイルの `owner_tid` チェックは T2 では発火しない**。決定 0054 の規則の「意味」(「別のホストスレッドから呼ばれたら停止」)はそのまま保たれ、**ゲストの別スレッドは「同じホストスレッドから呼ばれる」ので通る**。ただし**状態がスレッドごとに必要な表**(`fle_tls_values`、`fle_last_error`、優先度、ロケール `fle_thread_lcid`(`__thread`。これは**ホストのスレッド別**で、ゲストのスレッド別ではないので**直す必要がある**))は、**現在のゲストスレッドの文脈を引く**形に変える。
- **変更が要る点**(最小): (1) `fle_imports.c` の `fle_tls_values`、`fle_last_error`、(2) `fle_proc.c` の優先度・背景(`fle_thread_prio`、`fle_thread_background`)、(3) `fle_locale.c:52` の `__thread`(→ ゲストのスレッド文脈)、(4) `fle_callback.c` の `depth`、(5) `fle_dispatch.h` の `fle_landing_top`、(6) `fle_vex.h` の `fle_teb` とその `gs:` の答え、(7) `fle_event.c` の待ちの停止(WAITING への置き換え)、(8) `fle_sys.c` の `Sleep`/`SwitchToThread`、`fle_proc.c` の `SwitchToThread`(1,000,000 回の停止の対象は「他に READY なスレッドが無いとき」に限る)、(9) テスト **16 ファイル**のうち「第 2 スレッドで停止する」ことを固定するものの更新(`tid.rs` の `guest_thread_id_is_the_same_from_another_host_thread` は決定 0056 が意図して失敗すると書く)。
- T3(実スレッド)にする場合は、`fle_host_tid()` が本当に別の値を返すので、**14 ファイルの `owner_tid` を本物のロックに変える**必要がある(`sync-thread-survey.md` §3.3 D の引用)。T2 とは変更の大きさが違う。
- **決定 0056 の `FLE_EMULATED_TID`**: 最初のスレッドの値として維持し、2 本目以降を連番にする(決定 0056 「将来の拡張点」がそのまま使える)。

### 7.4 窓表とメッセージキューのスレッド別化(T2 の最大の部分)

壁 21 の先で実際に要るもの(§4.3、§5.2):
- `PostMessageW(hwnd, 0x8ffc, …)`(`TTimerThread` → メインスレッドの窓): **窓を作ったスレッド(main)のキューに積む**。現行は未実装(決定 0119 の PM の判断点 3: `PostMessageW` の NULL の hWnd は Learn が「現在のスレッドへの `PostThreadMessage`」と書き、PM の決定 1(NULL は 0 と 1400)と食い違う)。
- `GetMessageW`(`TLayeredControlThread`): 自分のキューが空なら WAITING。他スレッドの `PostMessageW`/`SendMessageW`/`PostQuitMessage` で起きる。Learn: 「The function dispatches incoming sent messages until a posted message is available」、`WM_QUIT` で 0。
- `PeekMessageW`(`TLayeredControlThread` の先頭、`TThread.WaitFor`、`TSharedAnimTimer` のデストラクタ): 空なら 0 で即返し(`PM_NOREMOVE` はキューを作るだけ)。
- `SendMessageW` が**別スレッドの窓**に送るとき(Learn の引用 §6.1): 送り手を WAITING にし、受け手が `GetMessageW`/`PeekMessageW`/`MsgWait…` に入ったときに窓手続きを `fle_call_guest` で受け手のスレッドの文脈で呼び、戻り値を送り手に返して起こす。**現行の `SendMessageW`(`fle_win.c`)は同一スレッドの窓にだけ直接呼ぶ**(決定 0119 D9)。
- **窓の所有スレッド**を窓表に持たせる(現行の `win` 構造体にない)。`CreateWindowExW` の呼び出しスレッドを記録する。
- 範囲: **`PostMessageW`/`GetMessageW`/`PeekMessageW`/`WaitMessage`/`MsgWaitForMultipleObjects`/`PostQuitMessage`/`DispatchMessageW`/`TranslateMessage`(`TLayeredControlThread` が使う)**。決定 0119 が「実装しない」とした一群と同じで、**スレッドとは別のワークストリームとして先に設計が要る**(決定 0119 の PM の判断点 2)。

### 7.5 最小の差分の入口(段階)

| 段 | 内容 | 前提 | 壁 21 の先で何が通るか(読み) |
|---|---|---|---|
| **E0** | `fle_thread.c` を新設し、スレッド表と `CreateThread`/`ResumeThread`/`GetExitCodeThread`/`ExitThread`/`SuspendThread`/`GetCurrentThreadId`/`SwitchToThread`/`Sleep`/`WaitForSingleObject`(スレッドと既存のイベント)の**行**を `imports_table.rs` の末尾に足す。スケジューラは native では ucontext、wasm は JSPI を後。`CreateThread` はこの段でも `CREATE_SUSPENDED` を保持し、**ResumeThread で READY にして、メインスレッドが最初に待つ(`Sleep`/`Wait`)まで走らせない**(T4 相当の半分) | 決定(PM) | 壁 21 の `CreateThread`/`SetThreadPriority`/`ResumeThread` を通る。直後にメインスレッドが `PostMessageW`/`EnumDisplaySettingsW` の続きを通って、**次の壁**(例: メインスレッドが `TThread.WaitFor`/`Sleep(0)` で待つか、`TLayeredControlThread` の `CreateThread`)に進む |
| **E1** | `Sleep`/`WaitFor…`/`SetEvent` で**実際に切り替える**(native: swapcontext)。`TTimerThread` の `Execute` が走り、`DwmFlush` の thunk(行なし)で名前付き停止 | `dwmapi.dll` を仮想モジュールにする決定(§5.2) | `TTimerThread` の最初の `Sleep` と `DwmFlush` まで |
| **E2** | メッセージキューのスレッド別化(§7.4)。`PostMessageW`/`GetMessageW`/`PeekMessageW`/`MsgWaitForMultipleObjects` | 決定 0119 の判断点 2、3 の PM の決定 | `TTimerThread` → main の tick、`TLayeredControlThread` のループ |
| **E3** | wasm: JSPI での切り替え(スレッドごとの shadow stack の `stackSave/Restore`、`SUPPORT_LONGJMP=wasm`)。native と wasm の同じ API の別実装 | `jspi-real-wasm.md` の構成 C | ブラウザで同じ停止点 |
| **E4** | `DLL_THREAD_ATTACH/DETACH` の扱い、暗黙 TLS のスレッド別化、`TlsAlloc` のスレッド別化、FLS | 決定 0080 D9 の変更 | Blend2D のスレッドプールを使う場合 |

## 8. 推奨(決めない。根拠つき)

### 8.1 推奨の要旨

**最終形は T2(協調スレッド。wasm は JSPI、native は swapcontext 系)。壁 21 はその最小形で通す。T1(作ったことにする)と T4 単独は採らない。T3(実スレッド)は G8 の実測で必要が分かったときに限る。**

**根拠**:

1. **T1 は偽る上に、結局次の壁で止まる**(§6.3)。`TThread` のデストラクタは `Started`(`[+0x18]`)を `SwitchToThread` で待つ(`while Started == 0`、`f_1df1430`)。`Started` を立てる者はスレッド自身(`ThreadProc` の先頭)なので、走らせないなら runtime が `Started` を書くしかなく、それは「ゲストの状態を runtime が決める」= 決定 0121 の Z2 と同じ偽り。`TTimerThread` が走らなければ `PostMessageW(0x8ffc)` と購読者コールバックが一切起きず、**フレーム更新の駆動が G4 まで来ない**。
2. **T4 単独は成立しない**(§6.3)。`TTimerThread.Execute` も `TLayeredControlThread.Execute` も待ちのループで、`Terminated` まで戻らない。最初の待ちで止める規則にすると、壁 21 の直後の `Sleep`/`DwmFlush` で止まる。
3. **T2 は 3 つの観点で最も得**:
   - **偽りが最小**: スレッドの命令列はゲストのものがそのまま走り、runtime が決めるのは**待ちの API でのスケジューリング(実機が任意に決める部分)だけ**。
   - **機構が既に実測されている**: JSPI(`jspi-real-wasm.md`: 全入口で中断して a4 が一致、スタック 150 kB で通る)と、決定的な仮想時計(決定 0017)。メッセージループを JS のイベントループに戻す仕組み(G3、`browser-event-loop.md` M-B)と**同じ機構**なので、G3 のために作る部分が T2 の土台になる。
   - **G7・G8 に作り直しが要らない**: ミキサー(`WaitForMultipleObjects(INFINITE)` + 通知イベント)、`TWAVStreamingThread`(CS + 仮想呼び出し)、`TFileDeletionThread`(`TEvent.WaitFor(INFINITE)`)は、**待ちの API でだけ切り替わる協調スレッドで足りる形**(読み。いずれも CPU を連続して使い続ける形には読めなかったが、**ミキサーが 1 回の起床で長い計算をするかは未測定**)。
4. **T3 は重く、再現性を失う**: 全オブジェクトの `-pthread` 再ビルド、`R`/`X`/`YH`/`FL`/x87/MXCSR/`fle_teb`/`fle_tls_values`/`fle_last_error` の thread-local 化、`lock` 付き命令 257(FLEngine)の `__atomic` 化(生成器の変更)、`cmpxchg16b` の原子 128 ビットが wasm に無い(§6.1)、COOP/COEP の配信要件。決定性(A5、native SHA の一致、停止点の再現)を失う。

### 8.2 段階導入案

| 段階 | 内容 | 目的 | 切り替え・止まる基準 |
|---|---|---|---|
| **S0**(決定前の準備。実装なし) | PM が次を決める: (a) T2 を採るか、(b) `dwmapi.dll` を仮想モジュールにして `DwmFlush` の答えを決めるか、(c) メッセージキューの設計(決定 0119 判断点 2、3)を先に作るか、(d) `DLL_THREAD_ATTACH/DETACH` の扱い、(e) スケジューリングの選択(切り替え点、優先度を無視、仮想時計の進め方)を仮の決定とするか | 決定 | — |
| **S1** | **native で T2 の最小形(E0〜E1)**: スレッド表、`CreateThread`/`ResumeThread`/`SuspendThread`/`ExitThread`/`GetExitCodeThread`/`SwitchToThread`/`Sleep`/`WaitForSingleObject`/`SetEvent`、スレッドごとの TLS・last error・`R[]` ほかの退避。**メインスレッドが最初に待つまで新スレッドを走らせない**(T4 相当の入口を含む) | 壁 21 を通す。**native の停止点が再現し、独立 2 回で同一**(決定 0121 の検証の流儀) | 壁 21 を通った先で、**`DwmFlush`(thunk、行なし)で止まる**ことが想定される(§5.2)。止まる基準: 名前付き停止(新しい壁)のたびに分類して PM に戻す(決定 0119 の止まる条件を維持) |
| **S2** | **メッセージキューのスレッド別化(E2)** と、`PostMessageW`/`GetMessageW`/`PeekMessageW`/`WaitMessage`/`MsgWaitForMultipleObjects`。`DwmFlush` の答え(**`requestAnimationFrame` に結ぶか、仮想時計の 16.7 ms にするか**。出典なしの選択) | `TTimerThread` の tick が main の窓に届き、`TLayeredControlThread` のループが回る。G2〜G3 | **ここで `TTimerThread` の購読者コールバックが何を呼ぶか**(描画の要求か)を実測する。**その結果が「本物のスレッドが要るか」の判断材料**(下) |
| **S3** | **wasm で JSPI(E3)**。`CreateFruityInstance` の入口を wasm に足し、native と同じ停止点が wasm で出るか(`goal-ladder.md` §5 X4 の「小さい」実験)を先に確認 | ブラウザで成立するかの実測(**未実測**。実エンジンの深さの上限が最大の不確実性) | JSPI のスタック不足(`RangeError`)が出たら、(a) `--wasm-stack-switching-stack-size` を増やす、(b) スレッドの深さを測って設計を見直す、(c) T3 に倒す(Worker + `-pthread`)のどれかを PM に戻す |
| **S4** | G7(`TMixerThread_DS`)、G8(`TWAVStreamingThread`、`TFileDeletionThread`、レンダー)を T2 で追加。DirectSound の通知イベントを AudioWorklet の消費に結ぶ | 音声 | **ミキサースレッドが 1 回の起床で CPU を使い続けて UI が止まる**(先取りが無い)ことが実測で分かったら、**T3 または先取りの導入(`requestAnimationFrame` ごとの強制切り替え。ただし x86 のコードは任意の点で切れない)を PM に戻す** |

**止まる・切り替える基準(まとめ)**:
- **T2 から T3 へ**: (1) JSPI で実エンジンのスレッドの深さが足りず、スタックを増やしても通らない、(2) ミキサー/ディスクスレッドが長い連続計算をして、協調では UI の応答が保てない(G7/G8 の実測)、(3) JSPI が使えないブラウザを対象に含めると決めた(Safari の対応は**未確認**。V8 ブログは Chrome 137、Firefox 139 を挙げる)。T3 にするなら、`lock` 付き命令の `__atomic` 化と `cmpxchg16b` の扱いを先に決める。
- **T2 の途中で止まる**: スレッドの本体が**ゲストの任意の点で他スレッドの状態を競合して読む**(ロックなしのスピン)ことが分かったとき。`pause` 命令は各 DLL に 5 個ずつ(`spin_sleep_scan.txt`)あるが、**スピンの待ち先が別スレッドのフラグかは未確認**(`Sleep(0)`/`SwitchToThread` を挟むスピンは協調で解ける。純スピンは解けない)。

### 8.3 スレッドの本物が G4 の最初のフレームまでに必須か(読み)

- **必須の可能性が高い**。壁 21 の `TTimerThread` は `CreateFruityInstance` の初期化の途中で作られ(`f_71e900` の経路、§4.3)、`TLayeredControlThread` も同じ関数が作る。**この 2 本の `Execute` は待ちのループで、`TTimerThread` は main の窓へ `PostMessageW(0x8ffc)` を送り、購読者のコールバックを直接呼ぶ**(§5.2)。ここが**アニメーション・再描画の駆動源**と読めるが、**購読者が何を呼ぶか(最初のフレームに必要か)は未読**。
- **ただし「協調スレッドで足りるか」と「本物(並列)が要るか」は別**: G4 までの 2 本は、(a) CPU を使い続けず待ちに入る、(b) 共有状態は所有者オブジェクトと CS で保護されている、(c) 協調スケジューラで順序を決められる、と読めるので、**T2 で足りる可能性が高い**(根拠の強さ: 読み)。
- **G4 までに必須でない可能性があるもの**: ミキサー(DirectSound の生成は `DirectSoundCreate` の分岐次第、未測定)、`TWAVStreamingThread`(プロジェクトを開くまで)、`TFileDeletionThread`(ファイル削除の要求まで)、`TMIDIThread`(MIDI 出力を開くまで)、クラウド・ネットワーク系。
- **`AcquireSharedThreadBank` のワーカー(`TSTBThread`)は、プロセッサ数 1 の現行応答では作られない**(§7.2 の表。読み)。**プロセッサ数の応答を変えない限りスレッドは増えない**。この応答を決定 0017 から変える場合は、スレッドが増える影響を併せて決める必要がある。

## 9. 限界と、確認できなかったこと

- **実行していない**。壁 21 の先は 1 点も測っていない。ここまでの「作られる」「走る」は静的な読みで、実行時の分岐(`[rbx+0xc0]` の値、`Flags`、レジストリ値)は未測定。
- **仮想呼び出しと vtable を辿っていない**。§4 の到達は直接 call と `lea` の下界で、壁 21 の `f_71e900` 自体が直接 call の閉包に入らない(`f_721bb0` の `call [rbx+0xb0]` から届く)ことが、下界であることの実例。**実際に作られるスレッドは §4.2 の表より多いはず**。
- **TThread 派生 28 クラスのうち `Execute` の中身まで手で読んだのは 5 つ**(`TTimerThread`、`TLayeredControlThread`、`TActivityThread` の冒頭、`TMixerThread_DS` の冒頭、`TWAVStreamingThread` の冒頭)。残りは `execute_waits.tsv` の直接 call 深さ 3 の待ち系 API の一覧だけ。
- **`TTimerThread` の購読者コールバック**(`[obj+0x58]`、`[obj+0x60]` の 2 本のリスト)が何を呼ぶか、`[0x1eda6f8]` の共有タイマーを誰が購読するかは読んでいない。**G4 の最初のフレームに本物のスレッドが要るかの最も重要な未確認**。
- **`f_1e48a60`(DWM 遅延ロードのヘルパー)の中身**、`DwmFlush` の thunk が失敗したときの経路は読んでいない。
- **`DLL_THREAD_ATTACH/DETACH`(reason 2/3)での FLEngine/QFC/ILTools の DllMain の動作**は読んでいない(FLEngine は `0x41d905` で `reason == 1` のときだけ初期化に進み、他は RTL の `DLLProc` 呼び出しに行く形に見える。読み)。
- **ILTools のスレッド**は、ILTools が測定で `CreateThread` に届いていないので何も測っていない(静的に 2 点)。**Blend2D の `CreateThread`** も `thread_count` を FL Studio が渡すかが未確認(`blend2d-lift-feasibility.md` B4)。
- **ブラウザ・wasm は未測定**。JSPI の挙動は `jspi-real-wasm.md` と `browser-event-loop.md` の引用(Chrome 156 と node 24 の測定)で、Firefox/Safari は未確認。Safari の JSPI 対応は V8 ブログに書かれておらず、`sync-thread-survey.md` の「Safari 27」は二次情報で未検証。
- **実機 Windows との比較は無い**。実機のスレッドの起動時刻、`DwmFlush` の周期、`SwitchToThread` の戻りは Learn の文言の範囲でだけ事実。
- **決定論的スケジューラの選択**(ラウンドロビン、優先度無視、仮想時計の進め方)は**出典なしの選択**で、どれも撤回条件つきで列挙する必要がある。
- **外部文書の取得物は `/tmp/opencode/thr/` に置いたが、終了時に削除する**。本書は取得した文言を短く引用している。URL は §1。

## 10. PM の判断が要る点

1. **T2 を最終形として採るか**。採らない場合、壁 21 は T0(止まる)のまま。T1(偽る)は決定 0121 の PM の決定 4 に反するので採らない提案。
2. **`dwmapi.dll` を仮想モジュールにして `DwmFlush` に答えるか**。`TTimerThread` は毎周回 `DwmFlush` を呼ぶ(§5.2)。答え方の選択: ブラウザの `requestAnimationFrame` に結ぶ(実時間と結びつく。決定論と衝突)か、仮想時計で 1000/60 ms 進める(決定的。実機の Present とは別)。**出典のない選択**。
3. **メッセージキューを先に設計するか**。`TLayeredControlThread` は `GetMessageW` を持ち、スレッド側の窓を作る。決定 0119 の PM の判断点 2(キューの設計)と 3(`PostMessageW` の NULL の扱い: Learn は PostThreadMessage と同じ)が未決。
4. **スケジューリングの選択を仮の決定とするか**(切り替え点 = 待ちの API のみ、優先度を無視、ラウンドロビン、全員が待ちなら仮想時計を最小の期限まで進める)。
5. **`DLL_THREAD_ATTACH/DETACH` を呼ぶか**(決定 0080 D9 の変更。実機は呼ぶ)。
6. **プロセッサ数の応答(現行 1、決定 0017)を維持するか**。維持すれば `AcquireSharedThreadBank` のワーカーは作られない。変えるとスレッドが増える。
7. **対応ブラウザに JSPI の無いものを含めるか**(含めるなら T3 または M-C' の保険が要る。`browser-event-loop.md` §12 の 3)。
8. **ネイティブ側(clang の harness)に T2 の別実装を作るか**。native は決定性の検証(独立 2 回で同一の SHA)の基盤なので、wasm だけの実装は検証の軸を失う。
9. **T3 を保険として残すか**(G8 の実測で協調が足りないと分かったとき)。

## 11. 成果物

- `docs/investigations/thread-model.md`(本書)
- `docs/investigations/thread-model/`(合計 約 180 KB):
  - 棚卸し TSV: `api_counts.tsv`(全 API の件数)、`thread_api_sites.tsv`(691 行。ノイズの API を除外)、`thread_classes_in.tsv`(55 行。他の作業者の `lift/out-sync/thread_classes.tsv` の複製)、`classreach.tsv`、`reach_threads.tsv`、`execute_waits.tsv`、`lock_count.txt`、`spin_sleep_scan.txt`
  - スクリプト: `scan_sites.py`、`createthread_args.py`、`reach_threads.py`、`classreach.py`、`execute_waits.py`、`callers_reach.py`、`agg_counts.py`、`count_direct_calls.py`、`list_imports.py`、`lock_count.py`、`spin_sleep_scan.py`、`disasm_pe.py`、`dump_pe.py`、`rtti_names.py`
  - **`dis.py`(間違いで作った。ファイル名が Python 標準ライブラリの `dis` と衝突する。後始末は §12)**

## 12. 規則の遵守と自己申告、後始末

**触ったもの**: 新規のみ。`docs/investigations/thread-model.md`、`docs/investigations/thread-model/`(約 204 KB。うち `__pycache__` 24 KB は下記)。リポジトリの他のファイル、`lift/out-*`、他の `/tmp/opencode` のディレクトリは**読み取りのみ**。`git status` は `?? docs/investigations/thread-model.md`、`?? docs/investigations/thread-model/` が増えただけ(`README-0101-line.txt` と `lift/iat-unsup-closure.txt` は私の作業の前からある未追跡ファイル)。commit、git の変更系、引数なし `checkout` はしていない。ビルド・実行(cargo/clang/gcc/emcc/gdb)はしていない(python の capstone と pefile、`curl`、`objdump` ではなく自作の `disasm_pe.py` のみ。`objdump` は結果として使っていない)。`sleep`/`timeout`/`nohup`/末尾 `&` は書いていない。製品の機械語の引用は §5.2 の 10 行の抜粋のみ(60 行以内)、文字列は DWM の関数名と RTTI の短い断片。

**規則違反(自己申告)**:

1. **`docs/investigations/thread-model/dis.py` を作った**。ファイル名が Python 標準ライブラリの `dis` と衝突する(依頼の「python 標準ライブラリと衝突する名前のファイルを作らない」に違反)。実際に `capstone` の import が `inspect` → `dis` の循環で失敗した。**作った直後に `disasm_pe.py` に作り直して代替し、`dis.py` は標準ライブラリの `dis` を読み込むだけの shim に書き換えた**(中身は上書き。内容はコメントに書いた)。削除は `/tmp/opencode/thr/` の外なので許可されておらず、**私は消せない。PM が削除してよい**(他のスクリプトは `dis.py` に依存しない。`scan_sites.py` などは `disasm_pe.py` を使う)。ただし shim が残っている間は、このディレクトリで `python3 -c "import capstone"` を実行すると標準の `dis` の代わりに shim が読まれる。shim は標準の `dis` を読み込む形にしてあるので動くが、**削除が望ましい**。
2. **`__pycache__` が `docs/investigations/thread-model/` にできた**(`callers_reach.cpython-314.pyc`、`dis.cpython-314.pyc`、`scan_sites.cpython-314.pyc` の 3 ファイル、24 KB)。原因: `PYTHONDONTWRITEBYTECODE=1` を付けずに、`python3 -` と `python3 -c` でこのディレクトリのモジュールを `import` した実行が 3 回あった(`callers_reach` と `scan_sites` の import、`dis` の shim の確認)。`.gitignore`(43 行目の `__pycache__/`)で無視されるので `git status` には出ないが、**規則違反**。**これも `/tmp/opencode/thr/` の外なので私は消せない。PM が削除してよい**。
3. **`/tmp/opencode/thr/work/` に作った `fle.pkl`(約 4 MB)などの索引は終了時に削除した**(`/tmp/opencode/thr/` の下の自分の物。150 MB の上限内。最大でも 7 MB)。外部文書(Learn、Emscripten、MDN、V8、W3C)の取得物も同じ場所に置き、**終了時に削除する**。
4. **サーバの再起動が 3 回あり**、`/tmp/opencode/thr/` のうち一度は全体が消えていた(再起動後の `ls` で `No such file or directory`)。再生成できるもの(索引 pickle、取得文書)は作り直した。**再起動で失われた可能性があるのは、`/tmp` 上の作業メモだけ**で、リポジトリ内の成果物は `git status` と `ls` で残っていることを確認した。
5. **本文の記述の訂正(作業中に自分で見つけて直した)**: (a) 決定 0121 の「`MsgWaitForMultipleObjects`/`PeekMessageW` はスレッド本体」を、`TThread.WaitFor` の中の命令と読み直した(§5.1。**決定 0121 の読みの訂正**であり、同決定の結論(壁 21 は (d) スレッド)は変わらない)。(b) `TTimerThread` の本体は `Sleep` と `DwmFlush` のループで、`DwmFlush` は QFC の遅延 import(最初に `jmp [IAT]` の thunk と書いたのを遅延ロードのヘルパー経由に直した)。(c) `DwmGetCompositionTimingInfo` と `DwmIsCompositionEnabled` の名前は、最初 `split(b'\0')` で ASCII として読んで 1 文字しか取れなかったのを、UTF-16 として読み直して確認した。(d) 「`cmpxchg16b` を決定 0023 が持つ」の記述は、決定の本文を読む前に書いていたので、読んでから直した(§6.3 T3)。(e) 「`CreateFruityInstance` の本体が `f_11efbf0` を直接呼ぶ」と書いたのを、実際は `f_1375950`(Run の入口)経由と確認して直した(§4.2)。(f) 件数(「33 種の ctor」「名前不明 13 点のうち 12 点」など)は集計してから書き直した。**ほかにも確認しないまま残っている記述がありうる**ので、§9 の「未確認」を信用の範囲の上限として読んでほしい。
