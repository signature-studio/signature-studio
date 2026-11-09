# 同期・時間・メモリ・システム情報 API の仕様の出典と実装状況

作業: 作業者（M1、サブエージェント）。日付: 2026-10-20。判断は `docs/decisions/0017-sync-time-memory.md`。実行記録は `docs/investigations/sys-run.md`。
出典はすべて learn.microsoft.com（2026-10-20 に取得）。ページに書かれていない値は「ページにない」と書き、実装の選択は「選択」「推測」と区別した。

## 1. 閉包で呼ばれる担当 API（実測。`lift/out-usage/unhandled-imports-usage-v3.tsv` と `import-callers-v3.tsv`）

| API | DLL | IAT slot | 未対応 jmp（thunk） | 呼び出し箇所 | 呼び出し元の関数 | 実装 |
|---|---|---|---|---|---|---|
| EnterCriticalSection | kernel32 | 0x1971530 | 2 | 173 | 119 | した |
| LeaveCriticalSection | kernel32 | 0x19715f8 | 2 | 181 | 120 | した |
| InitializeCriticalSection | kernel32 | 0x19715a8 | 2 | 25 | 24 | した |
| DeleteCriticalSection | kernel32 | 0x1971768 | 0 | 0 | 0 | した（閉包では呼ばれない。対の API として） |
| Sleep | kernel32 | 0x1971520 | 2 | 15 | 9 | した |
| GetTickCount | kernel32 | 0x1971550 | 2 | 13 | 8 | した |
| timeGetTime | winmm | 0x1971cb0 | 1 | 21 | 17 | した（仮想時計を一つにするため。依頼の一覧には名前がない） |
| QueryPerformanceCounter | kernel32 | 0x19716a8 | 2 | 9 | 9 | した |
| QueryPerformanceFrequency | kernel32 | 0x19712d0 | 1 | 3 | 3 | した |
| GetSystemInfo | kernel32 | 0x19715e8 | 2 | 4 | 4 | した |
| VirtualAlloc | kernel32 | 0x19715d0 | 2 | 6 | 5 | した |
| VirtualFree | kernel32 | 0x19712f0 | 1 | 3 | 2 | した |
| GetProcessHeap | kernel32 | 0x19712f8 | 1 | 2 | 1 | した |
| HeapAlloc | kernel32 | 0x1971308 | 1 | 1 | 1 | した |
| HeapFree | kernel32 | 0x19716c0 | 1 | 1 | 1 | した |

呼び出し元の関数は、担当 15 API のどれかを使う関数の和集合で 199 件（`lift/out-sys/sys-callers.txt`。`import-callers-v3.tsv` の集計。間接呼び出しは含まない、という v3 の定義に従う）。

呼ばれない、または import にない API（実測）:

| API | 状況 | 扱い |
|---|---|---|
| HeapCreate | 通常 import（0x1971628）。thunk 0、呼び出し 0。DLL 内に文字列 `HeapCreate` が 1 つある | しない（呼ばれていない。応答表にもない） |
| HeapDestroy | 通常 import。呼び出し 0 | しない |
| InitializeCriticalSectionAndSpinCount | import にない | しない。ページは読んだ（§2） |
| TryEnterCriticalSection | import にない | しない |
| GetNativeSystemInfo | 遅延 import（0x1976390）。呼び出し 0。DLL 内に文字列が 1 つ | しない。ページは読んだ（§2）。64 ビットでは GetSystemInfo と同じ、と書かれている |
| GetLogicalProcessorInformation | 遅延 import（0x19762d0）。呼び出し 0。DLL 内に文字列が 3 つ | しない。GetProcAddress は NULL を返す（決定済み。0015）。ページは読んでいない |
| GetTickCount64 | import にない。DLL 内に文字列が 1 つ（GetProcAddress で引く名前と推測） | しない。ページは読んでいない。GetProcAddress は NULL を返す |
| GetSystemTimeAsFileTime | import にない | しない |
| InterlockedXxx | import にない（DLL に `lock` 付き命令として直接ある。`lock cmpxchg` は 0x4074f3 など） | しない（thunk がない） |
| timeBeginPeriod、timeGetDevCaps | winmm の import。各 1 thunk | 担当外。応答なし |
| GetLocalTime、FileTimeToSystemTime など時刻変換系 | kernel32 の import | 依頼の担当一覧にない。応答なし |
| GetCurrentProcess、GetCurrentProcessId、VirtualProtect、VirtualQuery、GlobalAlloc ほか | kernel32 の import | 担当一覧にない。応答なし |

DLL 内の文字列は `strings -a` の集計で、使われ方は読んでいない（推測）。

## 2. 仕様の出典と読み

各ページで「書かれていること」と「書かれていないこと」を分ける。

### 2.1 クリティカルセクション

- InitializeCriticalSection: https://learn.microsoft.com/en-us/windows/win32/api/synchapi/nf-synchapi-initializecriticalsection
  - 書かれている: VOID。オブジェクトは移動も複製もできず、論理的に不透明に扱う。初期化済みのものを再初期化するのは未定義。再初期化の前に削除が必要。Vista 以降は常に成功する。
- EnterCriticalSection: https://learn.microsoft.com/en-us/windows/win32/api/synchapi/nf-synchapi-entercriticalsection
  - 書かれている: 所有権を得るまで待つ。所有中のスレッドが再び Enter しても待たない。成功のたびに 1 回入ったと数え、同じ回数の Leave が要る。待ちが（レジストリの時間で）タイムアウトすると EXCEPTION_POSSIBLE_DEADLOCK を上げ得る。スレッドが所有したまま終了した場合、削除中に待っている場合の状態は未定義。
- LeaveCriticalSection: https://learn.microsoft.com/en-us/windows/win32/api/synchapi/nf-synchapi-leavecriticalsection
  - 書かれている: 入った回数だけ呼ぶ。所有していないスレッドが呼ぶとエラーになり、別スレッドの Enter が永遠に待つことがある。
- DeleteCriticalSection: https://learn.microsoft.com/en-us/windows/win32/api/synchapi/nf-synchapi-deletecriticalsection
  - 書かれている: 所有されていないこと（呼び出し側の責任）。削除後は Initialize 系以外で参照しない（メモリ破壊などが起き得る）。所有中に削除すると待ちスレッドの状態は未定義。
- InitializeCriticalSectionAndSpinCount: https://learn.microsoft.com/en-us/windows/win32/api/synchapi/nf-synchapi-initializecriticalsectionandspincount
  - 書かれている: BOOL。Vista 以降は常に成功し非ゼロ。単一プロセッサではスピン回数は無視される。実装していない（import にない）。
- 確認できなかった点: CRITICAL_SECTION 構造体のページ（`.../api/winnt/ns-winnt-rtl_critical_section`）は 404 だった。構造体の配置は Microsoft のページでは確認できていない（ローカルの mingw ヘッダに `LockCount`、`RecursionCount`、`OwningThread`、`LockSemaphore`、`SpinCount` があるのは見たが、二次資料であり、実装は使っていない）。実装は構造体の中を読まず書かず、状態を外の表に持つ。

### 2.2 Sleep

- https://learn.microsoft.com/en-us/windows/win32/api/synchapi/nf-synchapi-sleep
  - 書かれている: VOID。0 は残りの時間枠を譲り、他に実行可能なスレッドがなければすぐ戻る。INFINITE は時間切れなし。時計の分解能より短い指定は短く眠ることがある。実際の眠りは指定以上かもしれない。無限に眠るスレッドがウィンドウを持つと、システムがデッドロックする。
  - 書かれていない: INFINITE の定数値（ページの Sleep にはない。mingw の `winbase.h:518` に `0xffffffff` を見た。二次資料）。

### 2.3 時間

- GetTickCount: https://learn.microsoft.com/en-us/windows/win32/api/sysinfoapi/nf-sysinfoapi-gettickcount
  - 書かれている: システム開始からのミリ秒（DWORD）。分解能は 10〜16 ms 程度。49.7 日で 0 に戻る。
- timeGetTime: https://learn.microsoft.com/en-us/windows/win32/api/timeapi/nf-timeapi-timegettime
  - 書かれている: Windows 開始からのミリ秒（DWORD）。2^32 ms（約 49.71 日）で 0 に戻る。既定の精度は 5 ms 以上。差分で使うこと。
  - 書かれていない: GetTickCount と同じ時計であること。同じ時計を読むのは選択（推測）。
- QueryPerformanceCounter: https://learn.microsoft.com/en-us/windows/win32/api/profileapi/nf-profileapi-queryperformancecounter
  - 書かれている: BOOL。XP 以降は、有効なポインタなら必ず成功（非ゼロ）。
- QueryPerformanceFrequency: https://learn.microsoft.com/en-us/windows/win32/api/profileapi/nf-profileapi-queryperformancefrequency
  - 書かれている: BOOL。周波数は起動時に固定され、全プロセッサで共通。XP 以降は常に成功。
- Acquiring high-resolution time stamps: https://learn.microsoft.com/en-us/windows/win32/sysinfo/acquiring-high-resolution-time-stamps
  - 書かれている: ハイパーバイザ下では周波数が 10 MHz に固定されることがある（「always in some newer versions」とも書かれている）。周波数をハードウェアから導けると思ってはならない。カウンターは OS 開始からの ticks。
  - 10 MHz を選んだ理由はこの記述に合うため。これが Windows の実機の値だという主張ではない。
- 書かれていない: GetTickCount と QueryPerformanceCounter の起点の関係（同じ起点であること）。両者が同じ仮想時計を共有するのは選択。

### 2.4 システム情報

- GetSystemInfo: https://learn.microsoft.com/en-us/windows/win32/api/sysinfoapi/nf-sysinfoapi-getsysteminfo
  - 書かれている: VOID。SYSTEM_INFO に書く。
- GetNativeSystemInfo: https://learn.microsoft.com/en-us/windows/win32/api/sysinfoapi/nf-sysinfoapi-getnativesysteminfo
  - 書かれている: 64 ビットのアプリから呼ぶと GetSystemInfo と同等。実装していない。
- SYSTEM_INFO: https://learn.microsoft.com/en-us/windows/win32/api/sysinfoapi/ns-sysinfoapi-system_info
  - 書かれている: メンバー（型と順序）。wProcessorArchitecture の値（AMD64 = 9）。dwPageSize は VirtualAlloc が使うページ。dwProcessorType の値に PROCESSOR_AMD_X8664（8664）。wProcessorLevel は表示用。dwActiveProcessorMask は bit 0 がプロセッサ 0。dwNumberOfProcessors は現在のグループの論理プロセッサ数。
  - 書かれていない: dwAllocationGranularity の数値、lpMinimumApplicationAddress と lpMaximumApplicationAddress の数値、構造体の合計サイズとオフセット。オフセットは型と自然な整列からの導出（48 バイト）。

### 2.5 VirtualAlloc、VirtualFree、保護定数

- VirtualAlloc: https://learn.microsoft.com/en-us/windows/win32/api/memoryapi/nf-memoryapi-virtualalloc
  - 書かれている: 予約、コミット、両方同時。新しいページは 0。lpAddress が NULL なら OS が選び、dwSize はページへ切り上げ。NULL でなければ、予約はアドレスを割り当て粒度へ、コミットはページへ切り下げ、範囲内の全バイトを含むページが対象。アドレスを指定したコミットだけ（MEM_RESERVE なし）は、範囲全体が予約済みでなければ ERROR_INVALID_ADDRESS で失敗する。コミット済みページのコミットは失敗しない。予約済みページの予約はできない。失敗は NULL、詳細は GetLastError。その他の値（MEM_RESET、MEM_RESET_UNDO、MEM_LARGE_PAGES、MEM_PHYSICAL、MEM_TOP_DOWN、MEM_WRITE_WATCH）がある。
  - 書かれていない: 予約済みの範囲に予約を要求した失敗のエラーコード。MEM_COMMIT のみ、lpAddress が NULL のときの「予約も同時に行うか」の明示（MEM_COMMIT の説明は予約済みページへのコミット。実際の Windows ではこの呼び出しが使われる、と一般に知られているが、このページの文面だけでは明示がない。推測）。dwSize が 0 のときの扱い。
- VirtualFree: https://learn.microsoft.com/en-us/windows/win32/api/memoryapi/nf-memoryapi-virtualfree
  - 書かれている: MEM_DECOMMIT と MEM_RELEASE のどちらか一つ。MEM_RELEASE は dwSize が 0 で、lpAddress が VirtualAlloc の返した基準アドレスでなければ失敗する。リリース時に、コミット済みならまず解除する。MEM_DECOMMIT は範囲のバイトを含むページが対象で、未コミットページでも失敗しない。dwSize 0 で基準アドレスなら領域全体。解除後は予約状態に戻り、読み書きはアクセス違反。
  - 書かれていない: 失敗時のエラーコード。領域の外や複数領域にまたがる範囲の MEM_DECOMMIT。
- Memory protection constants: https://learn.microsoft.com/en-us/windows/win32/memory/memory-protection-constants
  - 書かれている: PAGE_NOACCESS 0x01、READONLY 0x02、READWRITE 0x04、WRITECOPY 0x08（VirtualAlloc 未対応）、EXECUTE 0x10、EXECUTE_READ 0x20、EXECUTE_READWRITE 0x40、EXECUTE_WRITECOPY 0x80（VirtualAlloc 未対応）、修飾子 PAGE_GUARD 0x100 など。
- システムエラーコード: https://learn.microsoft.com/en-us/windows/win32/debug/system-error-codes--0-499-
  - 書かれている: ERROR_NOT_ENOUGH_MEMORY 8、ERROR_INVALID_PARAMETER 87、ERROR_INVALID_ADDRESS 487。

### 2.6 ヒープ

- GetProcessHeap: https://learn.microsoft.com/en-us/windows/win32/api/heapapi/nf-heapapi-getprocessheap
  - 書かれている: プロセスの既定ヒープのハンドル。失敗は NULL。
- HeapAlloc: https://learn.microsoft.com/en-us/windows/win32/api/heapapi/nf-heapapi-heapalloc
  - 書かれている: 引数 hHeap、dwFlags（HEAP_GENERATE_EXCEPTIONS 0x4、HEAP_NO_SERIALIZE 0x1、HEAP_ZERO_MEMORY 0x8）、dwBytes。成功はブロックへのポインタ、少なくとも要求量。失敗は（例外フラグなしで）NULL、SetLastError は呼ばない。整列は Win64 で 16 バイト（MEMORY_ALLOCATION_ALIGNMENT）。単一スレッドなら HEAP_NO_SERIALIZE を安全に使える。
  - 書かれていない: dwBytes が 0 のときの結果。
- HeapFree: https://learn.microsoft.com/en-us/windows/win32/api/heapapi/nf-heapapi-heapfree
  - 書かれている: 成功は非ゼロ、失敗はゼロ（GetLastError）。lpMem は NULL でもよい。同じポインタを二重に解放するとヒープが壊れ得る。
  - 書かれていない: lpMem が NULL のときの戻り値。
- HeapCreate: https://learn.microsoft.com/en-us/windows/win32/api/heapapi/nf-heapapi-heapcreate
  - 読んだが、閉包で呼ばれないため実装していない。

## 3. 実装した／しなかったの一覧と、仕様にない選択

実装は `lift/runtime/fle_sys.c`（コメントに各選択を「assumption」「選択」と記した）。

- した: §1 の表の 15 API（`imports_table.rs` に 15 行を追記）。
- しなかった: HeapCreate、HeapDestroy、InitializeCriticalSectionAndSpinCount、TryEnterCriticalSection、GetNativeSystemInfo、GetLogicalProcessorInformation、GetTickCount64、GetSystemTimeAsFileTime、InterlockedXxx（理由は §1 の表）。
- 仕様の外の選択（推測を含む）:
  - 時計の起点 3,600,000 ms、周波数 10 MHz、読み取り 1 回ごとに 10 ticks（1 µs）進める（0017 の A）。
  - GetSystemInfo の値: プロセッサ 1 個、マスク 1、レベルとリビジョン 0、割り当て粒度 65536、アドレス範囲 0x10000〜0x7FFFFFFEFFFF（最後の 2 つは Microsoft のページで未確認）。
  - 失敗時のエラーコード: 予約の失敗は 487、メモリ不足は 8、VirtualFree の失敗は 87（ページにコードなし）。
  - HeapAlloc の dwBytes 0 は 1 バイトとして確保。HeapFree(NULL) は 1 を返す。
  - GetProcAddress の応答表に担当 API が入る（行数 13 → 28）。
