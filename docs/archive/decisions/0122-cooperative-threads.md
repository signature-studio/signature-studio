# 0122: 協調スレッド（T2: `CreateThread`・待ち・メッセージキュー・`dwmapi.dll`／`DwmFlush`・DLL_THREAD_ATTACH／DETACH）と `CreateFruityInstance` の壁 21。壁 22（`GetFileAttributesW("")`）で停止。すべて仮の決定

状態: **仮の決定（すべての応答と選択）**。依頼は PM（決定 0122 の内容は PM の仮の決定。番号 0122 は PM の仮の割り当て）。測定・実装・テスト・文書は作業者が行った。**作業は HEAD c6ac416 の `git archive`（`src/`）で始め、clean な写しを `clean/` に置いた。作業の途中でリポジトリの HEAD は進んだ。9f2d9c9 は c6ac416 と `lift/` の tree が同一（`a74faf8f…`）。その後の 6c8bbfb（diag1）は `lift/harness/fruity.c`・`a4-multi.sh` などを変えており、193b5c2 まで進んでいる。この作業の測定・テストはすべて c6ac416 + 差分の木のもので、diag1 を含む木では行っていない。** 差分 `fruity8.diff` は `clean/`（c6ac416）と 193b5c2 の `git archive` の両方に `patch -p1 --dry-run` が当たる（`MERGE-NOTES.md`。193b5c2 では `fruity.c` の hunk が数十行ずれて当たる。当てた後のコンパイル・テストは未実施）。**実機 Windows との照合は 0 件。wasm・ブラウザは未測定（native のみ。S3 は作っていない）。**

**肯定的に見せる点は少ない。** 壁 21（`kernel32!CreateThread`）は**通した**（スレッド 3 本が作られる）。しかし `CreateFruityInstance` は**戻らない**。メッセージ待ちにも**入っていない**（目的 (a)、(b) のどちらにも到達していない）。次の停止は壁 22 `GetFileAttributesW("")`（51,529 入口目）で、これは**スレッドの壁ではない**。停止の理由は「出典のない値（PM の判断）」で、止まる条件 (c)／(f) と読む。壁 23〜25 は、偽の答えを入れた**実験**（差分に入れない）で見えただけで、直していない。

区分の凡例は決定 0121 と同じ。**実測**＝コマンドの出力を見た事実。**読み**＝逆アセンブルやコードを読んだ上での解釈。**Learn**＝Microsoft Learn のページ（取得日 2026-10-30。取得した 41 ページと SHA-256 は `learn/index.tsv`）。**出典なしの選択**＝ページにも実測にも根拠が無く、作業者または PM が選んだ値。**非公式**＝Microsoft 以外の出典（mingw-w64 などの値）。**未確認**。

## 先に結論

| 項目 | 内容 |
|---|---|
| 全体テスト | `cargo test --workspace --no-fail-fast --offline` を**有効な 1 回の通し**（2026-10-31T01:09:49Z〜01:55:41Z、45 分 52 秒、終了 0）で **123 test result・1,761 passed・0 failed・ignored 0**。基準（HEAD c6ac416）は 122・1,716・0 failed。差: +1 バイナリ（`wall_fr8`）、+45 件（`wall_fr8` の 45 件）。削除したテストは 0。**1 回目の通し（95 件失敗）は私の誤りで無効**（変異検査が木のコピーのテストバイナリを共有の cargo target に残した。「検証」） |
| FRUITY 無効の a4（独立 2 回） | 942/942、895/895、`returned from entry`、終了 0、functions **5,308**（不変）、entries **1,009,189**（基準 1,007,947 から **+1,242**）、native `09528ebc567003d147702919bb43a901807393a66f25e040afd75b857f8b48a9`（基準 `60a25acc0f9cac5410aa07d6cb1c326b3ad2508058652b87d086a9d2238162f5`）。**entries が動いた原因は実測した**: `dwmapi.dll` を仮想モジュールにした（PM の項目 5）ため、ゲストの遅延ロードの初期化が `GetProcAddress` を 69 回余分に呼ぶ（23 名 × 3 回）。dwmapi を不在に戻した木のコピーでは 1,007,947 に一致する |
| FRUITY 有効の最終構成（独立 2 回） | 停止 `fle: GetFileAttributesW: empty path is not defined`（thunk `f_4234f0`）、**51,529 入口目**（壁 21 は 41,177 入口目。+10,352）、新規の関数 **640**（壁 21 では 572）、終了 134、**スレッド 3**（0x1001 実行中。0x1002 開始 0x1d0fdb0 と 0x1003 開始 0x412ae0 は `ready (not started)`）、native `89393a68afde4f5114f8d94c91ac2593021e92f94b5e2bc61b07a82b26d69232`（2 回とも同一） |
| 直した壁 | **1 壁**（壁 21 `CreateThread`）。壁 22 で停止。上限 12 壁のうち 1 壁しか使っていない |
| 生成 C | 旧新の `fe-lift` 出力を関数本体単位で比較: 変わった本体は **42**（FLEngine 16、Blend2D 3、QFC 9、ILTools 14、dsp 0、freetype 0）。42 のすべてが import thunk の `fle_unsupported(…)` → `fle_imp_<名前>()`。thunk 以外の本体の変化は 0。関数の追加・削除は 0 |
| 変異検査 | 8 件（1 件ずつ、木のコピーで）。**8 件とも検出**（生き残り 0） |
| 到達していないもの | (a) 戻る、(b) 空キューのメッセージ待ち。**どちらにも到達していない**。理由は壁 22〜25（下）。ゲストの最初のスレッドは、51,529 入口まで一度も待ち・譲りの API を呼ばない（実測）ため、**作った 2 本のスレッドはゲストの中ではまだ一度も走っていない** |

## 背景

- 前段（決定 0121）: `CreateFruityInstance` は 41,177 関数入口目の `kernel32!CreateThread`（壁 21、停止条件 (d) スレッド）で止まった。調査 `docs/investigations/thread-model.md`（静的解析のみ。何も実行・決定していない）が選択肢 T1〜T4 を挙げた。
- PM の決定（0122、変えない）: **T2 協調スレッド**。ゲストのコードはそのまま走り、runtime が待ち・譲りの API でだけ切り替える。作成順の巡回（round robin）、優先度は無視。切り替えは `Sleep(ms>0)`／`Sleep(0)`／`SwitchToThread`／`WaitFor*`／`MsgWait*`／`GetMessageW`（空）／`WaitMessage`／競合した CS／`DwmFlush`／スレッド終了だけ。`flags 0` のスレッドは、作った側が最初に待つか譲るまで走らない。全員が待つと仮想時計を最小の期限まで進める。期限なしで全員が待つと名前付き停止（デッドロック）。スレッド ID は 0x1001 から順に、ハンドルは `0x7FFAA0000000 + 4*(i+1)`。スレッドごとの状態: TLS、last error、優先度／背景モード、ロケール、コールバックの深さ、landing の先頭、TEB／`gs:`、R／X／YH／FL／x87／MXCSR。`fle_host_tid()` は不変。キューはスレッドごと、窓は作ったスレッドが所有、`PostMessageW(NULL)` は現スレッドのスレッドメッセージ、WM_TIMER／WM_PAINT／WM_QUIT 以外の合成メッセージなし、決定 0119 の NULL hWnd の 1400 の規則は不変。`dwmapi.dll` は仮想モジュール N=39、`DwmFlush` は次の 60 Hz の仮想時計の境界まで待って S_OK。DLL_THREAD_ATTACH／DETACH を実モジュールへ（大きくなれば止めて記録）。プロセッサ数は 1 のまま。偽らない。出典なしの選択は撤回条件つきで列挙。`TerminateThread` は答えないまま。
- 止まる条件: (a) `CreateFruityInstance` が戻る、(b) 最初のスレッドが空キューのメッセージ待ちに入り他は待つ、(c) 偽らないと進めない、(d) 大きなモデル、(e) lift の穴、(f) PM の値、(g) 12 壁。進捗は (a)／(b) への到達だけ。
- 土台の再現（実測。`logs/a4-keep/`）: 入力はすべて実在し SHA が一致。HEAD c6ac416 の clean な木から FRUITY 無効: 942/942、895/895、functions 5,308、entries 1,007,947、終了 0、native `60a25acc…`。FRUITY 有効: 壁 21 を再現（41,177 入口、新規の関数 572、終了 134、native `3f7fce0b…`）。**調査の引用と Learn の現物の差**（実測）: `thread-model.md` §6.1 は DwmFlush を「次の Present まで呼び手を止める」と引くが、現物のページは「`DwmFlush` waits for any queued DirectX changes that were queued by the calling application to be drawn to the screen before returning」と書き、「次の Present」の語は無い（`learn/DwmFlush.txt`）。どの境界まで待つかはページに無い。

## 壁の表

| # | 停止メッセージ | 原因（区分） | 種類 | 状態 | 測定（入口数／新規の関数） |
|---|---|---|---|---|---|
| 21 | `fle: unsupported instruction at 0x1d06d80`（`kernel32.dll!CreateThread`） | QFC の `TThread.Create` | スレッド（PM が T2 を選択） | **直した**（`fle_thread.c`、`imports_table.rs` の 15 行） | 41,177／572 → 51,529／640。`ResumeThread` は 41,187 入口で呼ばれる（呼び手 = 0x1001、相手 = 0x1002、前の中断回数 1） |
| 22 | `fle: GetFileAttributesW: empty path is not defined` | `f_111c010` が `f_7585c0` でテーマファイル `__current.flstheme` をフォルダ表 0x413 から探す。見つからず Delphi の文字列は空。`FileExists('')` → `GetFileAttributesW("")`（読み。実測のシャドウスタック `f_43ba70` ← `f_819110` ← `f_111c010` ← `f_11be0a0` ← … ← `CreateFruityInstance`）。Learn（`learn/GetFileAttributesW.txt`、`learn/Naming-Files.txt`）は空の名前について何も書かない | **値の出典なし**（(c)／(f)）。決定 0116 の壁 1 が同じ API・同じ理由で拒んだ答え | **未解決（止まった）** | 51,529／640 |
| 23 | `fle: GetSysColor: index 15 is not answered ...` | `ColorToRGB(clBtnFace)` → `GetSysColor(COLOR_3DFACE = 15)`。Learn は要素名を挙げ、色を書かない | 値の出典なし（(f)） | **未解決**（実験 e1 で見えた。差分に入れない） | 51,999／700 |
| 24 | `fle: unsupported instruction at 0x4250a0`（`user32!IsWindowVisible`） | Delphi の TApplication／フォーム（`f_86b800` … `f_86a470`） | 窓管理（ShowWindow、IsWindowVisible、SetWindowPos …）= 大きなモデル（(d)） | **未解決**（実験 e2。差分に入れない） | 52,081／706 |
| 25 | `fle: VirtualQuery: address is not in a VirtualAlloc region or a file view (…)`、その先 `FindResourceW: hModule 0x800000 …` | `FindHInstance`（`f_418b40`）の `VirtualQuery(address, &mbi, 0x30)` | runtime が追跡しないメモリへの照会 = 大きなモデル／偽（(c)(d)） | **未解決**（実験 x3。差分に入れない） | 52,233／730、52,237／732 |

実験の内容（差分に入れない。作業の木は消した）: e1 = `GetFileAttributesW("")` に INVALID_FILE_ATTRIBUTES＋last error 3 を答える。e2 = e1 に加えて `GetSysColor` の全添字に 0xC0C0C0（作った値）を答える。x3 = e1・e2 に加えて、残りの未回答 user32 import 178 個すべてに 0 を答え、さらに `VirtualQuery` に作った答えを入れる。**どの実験でも最初のスレッドは待ち・譲り・投稿の API を一度も呼ばず、スレッド 1・2 は `ready (not started)` のまま**（実測、52,237 入口まで）。偽の答えを 1 つ入れるたびに、次の「同じ未知のものについての問い」が出る（連鎖）。これは PM の「偽らない」が禁じる形なので、x3 でやめた。

壁の種類の件数（壁 21〜25）: スレッド 1（直した）、値の出典なし 2（22、23）、窓管理（大きなモデル）1（24）、追跡しないメモリ 1（25）、メッセージループ 0。

### 壁 22 の判断（私が止めた理由）

壁 22 の答え（`INVALID_FILE_ATTRIBUTES` と `ERROR_PATH_NOT_FOUND`（3））は 1 行で書ける。**書いて、測る前に戻した。** 理由: (1) 実機が空の名前に 3 を返すかは、読んだページのどこにも無い（私の経験の記憶に過ぎず、未確認）。(2) 決定 0116 の壁 1 が同じ API で同じ理由（「empty path is not defined」）で答えを拒んだ。(3) PM の規則は「出典のない値は決めずに列挙」。ここで私が答えると、壁 23（GetSysColor の色）、壁 24（窓の可視性）でも同じ判断を私が繰り返すことになる。`src/lift/runtime/fle_vfs.c` は HEAD とバイト単位で同一（`diff -q`）。**PM が 22・23・24 の答えを決めれば、実験 e1／e2／x3 の記録が次の停止点を示す。** この判断が過剰に慎重なら、PM が覆してよい（1 行）。

## 決定（仮）

### 決定の表

| # | 場所 | 旧 | 新 | 根拠 |
|---|---|---|---|---|
| E1 | `runtime/fle_thread.c`（新規）`CreateThread`／`ExitThread`／`ResumeThread`／`SuspendThread`／`GetExitCodeThread` | `CreateThread` は `fle_unsupported`（壁 21） | スレッド 1 本ごとにホストの C スタック（mmap 16 MiB＋ガードページ、`makecontext/swapcontext`）、ゲストのスタック（VirtualAlloc。既定 1 MiB、`STACK_SIZE_PARAM_IS_A_RESERVATION` なら dwStackSize、下限 64 KiB）、自前の TEB／TLS（1,088 スロット）／FLS（4,096）を持つ。ID は 0x1001 + 添字、ハンドルは `0x7FFAA0000000 + 4*(添字+1)`（再利用しない）。`CREATE_SUSPENDED`（0x4）で中断カウント 1。`ResumeThread` は**前の**中断カウントを返す。`GetExitCodeThread` は終了前 259（STILL_ACTIVE）、終了後は終了コード。`lpThreadAttributes` が非 NULL、未掲載のフラグ、終了済みスレッドへの Resume／Suspend は**名前付き停止**。最初のスレッド（ハーネスの呼び出し）の `ExitThread` は停止 | Learn CreateThread（「By default, every thread has one megabyte of stack space」、フラグ表、「the thread can start running before CreateThread returns」）、ResumeThread（前の中断カウントを返す）、GetExitCodeThread（STILL_ACTIVE = 259）。**数値（ID、ハンドル、スタック寸法、上限 256、MAXIMUM_SUSPEND_COUNT 127）はページに無い**（S 一覧） |
| E2 | `fle_thread.c` スケジューラ | スレッドは 1 本（`Sleep(ms)` は仮想時計を進めるだけ） | 切り替えは待ち・譲りの API の中だけ。次に走るのは**作成順で現スレッドの次の READY**。待ちは**待ち始めた順**に判定（auto-reset イベントの 1 回の `SetEvent` は最初に待ったスレッドへ）。全員が待つと仮想時計を最小の期限まで進める。期限なしで全員が待つと `deadlock: no thread can run …` の名前付き停止（メッセージに全スレッドの表と窓の表） | PM の決定。**どのスレッドを選ぶかは出典なしの選択**（thread-model.md の T2 の定義） |
| E3 | `fle_event.c`（`WaitForSingleObject`）、`fle_sys.c`（`Sleep`、`EnterCriticalSection`／`LeaveCriticalSection`）、`fle_thread.c`（`WaitForMultipleObjects`、`MsgWaitForMultipleObjects`、`SwitchToThread`（`fle_proc.c`）） | 未充足の待ちは名前付き停止、`Sleep` は時計を進めるだけ | 未充足の待ちは呼び手を止めて他を走らせる。`Sleep(0)`／`SwitchToThread` は譲る（`SwitchToThread` は他が走れば 1、無ければ 0）。CS は所有スレッドを持ち、他スレッドの所有中は待つ。所有者でないスレッドの Leave は停止。`nCount` が 0 または 64 超は 87 で WAIT_FAILED。`MsgWait` は入力で `WAIT_OBJECT_0 + n`、`fWaitAll = TRUE` は停止。**スレッドが 1 本のときの挙動は旧と同じ**（最初のスレッドは旧の保存先を使う） | Learn（WaitForSingleObject、WaitForMultipleObjects、MsgWaitForMultipleObjects の「258L」、Sleep、SwitchToThread、EnterCriticalSection）。87 は**出典なしの選択** |
| E4 | `fle_win.c`（キュー）、`fle_thread.c` | `PostMessageW`／`GetMessageW`／`PeekMessageW`／`WaitMessage`／`PostQuitMessage`／`TranslateMessage`／`DispatchMessageW` は未回答 | スレッドごとに FIFO（上限 10,000、満杯は 0＋ERROR_NOT_ENOUGH_QUOTA 1816）。窓は作ったスレッドが所有し、`PostMessageW` はその窓の所有スレッドのキューへ。`hWnd = NULL` は現スレッドのスレッドメッセージ（`MSG.hwnd` は NULL）。WM_QUIT は FIFO に入れず、**他の投稿メッセージが無いときだけ**配送し、`GetMessageW` は 0 を返す。`GetMessageW` は空なら待つ。他スレッドの窓への `SendMessageW` は送り手が待ち、受け手の取り出し系 API の中で窓プロシージャが走る。`DispatchMessageW` は所有スレッドの文脈で窓プロシージャを呼ぶ。WM_TIMER／WM_PAINT は作らない。決定 0119 の 1400（窓でない hWnd）は不変 | Learn（PostMessageW の「limit of 10,000」「NULL → PostThreadMessage」、GetMessageW、PeekMessageW、WaitMessage、PostQuitMessage、TranslateMessage、DispatchMessageW、SendMessageW、MSG、About Messages and Message Queues の WM_QUIT）。**満杯の last error 1816 は System Error Codes にある**。1400 と WM_QUIT の置換などは出典なしの選択 |
| E5 | `fle_modules.c`、`fle_load.c`、`imports_table.rs`、`fle_thread.c` | `dwmapi.dll` は不在（`fle_load_absent[]`、126） | **仮想モジュール N=39**（ハンドル = `0x7FF800000000 + 0x10000 * 39` = 0x7ff800270000）。不在の名前は 5 から 4（imm32、msimg32、propsys、windowscodecs）。`DwmFlush` は応答表の行（dwmapi.dll）で、**次の 60 Hz の仮想時計の境界（`ceil(k·10^7/60)` 刻み、現在より後ろ）まで待ち、S_OK（0）**。他の Dwm* 名は行が無く `GetProcAddress` は NULL＋127 | 決定 0014（Windows 10 は DLL を持つ）、決定 0015（追加のみ）。Learn DwmFlush（上の差）。**待つ境界は出典なしの選択** |
| E6 | `fle_realmod.c`、`fle_thread.c` | 実モジュールにスレッド通知なし | 新しいスレッドの文脈で、ロード済みの実モジュールの `DllMain(基底, 2, NULL)`（ロード順）。スレッドの終了で `DllMain(基底, 3, NULL)`（逆順）。戻り値は無視。仮想モジュールには送らない。`FLE_THREAD_NO_DLL_NOTIFY=1` で送らない（**Windows の挙動ではない**。決定の保険） | Learn DllMain（THREAD_ATTACH「The call is made in the context of the new thread」、「any value other than DLL_PROCESS_ATTACH, the return value is ignored」）。順序と `lpvReserved = NULL` は出典なしの選択。**大きな設計にはならなかった**（呼び出しは `call_dllmain` の再利用） |
| E7 | `fle_imports.c`、`fle_proc.c`、`fle_locale.c`、`fle_callback.c`、`fle_vex.h` | TLS・last error・FLS・優先度・背景モード・スレッドのエラーモード・ロケール・コールバックの深さは 1 本分 | 作ったスレッドは自前の表を持つ。最初のスレッドは旧の保存先（アクセサは NULL を返す）。`GetCurrentThreadId` はスレッドの ID。`SetThreadPriority` はスレッドのハンドルを受けて**値を保持するだけ**（スケジューラは無視） | PM の決定。Learn（TlsAlloc、SetThreadPriority、GetCurrentThreadId） |
| E8 | `harness/fruity.c` | 停止報告にスレッドなし | 停止時にスレッド表・キュー・仮想時計・窓を出す。最初のスレッドが空キューで入力を待ち他が走れないときの名前付き停止（条件 (b)）。`FLE_FRUITY_NO_IDLE_STOP=1` で切る。`FLE_FRUITY_THREAD_TRACE=1` で切り替えを stderr に出す | PM の止まる条件 (b) |
| E9 | 既存のテスト | 下の表 | 同 | 期待値の更新（旧→新）。弱めて通したものは 0 |

### 既存のテストの更新（旧 → 新と理由。いずれも新しい木で実際に落ちた／落ちる期待で、通すために弱めたものは無い）

| ファイル | テスト | 旧 | 新 | 理由 |
|---|---|---|---|---|
| `event.rs` | `the_runtime_sources_use_at_most_four_close_hooks` | フック登録 3 | 4（スレッドのハンドル） | `fle_thread.c` が 4 つ目を登録 |
| `event.rs` | `source_uses_no_host_clock_thread_process_or_file` | 関数 `fle_event_poll` | `fle_event_wait_state` に改名 | テストが `fle_event.c` の部分文字列 `poll(` を禁じる（ホストを読まないための検査）。挙動は不変 |
| `modules.rs`、`msctf.rs`、`load.rs`、`deferred.rs`、`wall_s.rs`、`wall_fr5.rs`、`wall_fr6.rs` | モジュール数、表の外のハンドル、不在の名前の一覧 | `count 38`、外のハンドル 38／39、不在 5 名（dwmapi 先頭） | `count 39`、39／40、不在 4 名。`LoadLibrary("dwmapi")` を「不在の例」にしていたテストは `imm32` に替え、dwmapi の期待は `h(39) 5` | PM の項目 5 |
| 改名 | `the_five_lacking_names_still_fail_with_126` → `the_four_lacking_…`、`the_five_names_that_remain_absent_…` → `the_four_names_…`、`rpcrt4_is_no_longer_in_the_list_of_lacking_names_and_the_five_others_are` → `…the_four_others_are`、`the_eight_new_rows_are_the_last_eight_…` → `the_eight_new_rows_are_a_block_of_eight_…`（後ろに 15 行あるので最初の行で探す）、`wall_fr7` の `…make_no_message_queue` → `…_apart_from_decision_0122` | | | 名前が期待の数を言っているため |
| `wall_fr5.rs`、`wall_fr4.rs` | 後ろの行数（`LATER_ROWS`） | 8、9 | 8+15、9+15 | 15 行を追加 |
| `window.rs`、`wall_fr6.rs`、`wall_fr7.rs` | 窓の源の検査 | 「PostMessageW 等は未回答」「`fle_win.c` にキューの語なし」「`fle_set_last_error(` 1 箇所」「`invalid_window();` 5 箇所」 | これらの名前が `fle_win.c` に入る。`fle_set_last_error(` 2 箇所（満杯の 1816）、`invalid_window();` 7 箇所。`MsgWaitForMultipleObjects` だけは `fle_thread.c` に残す | E4 |
| `realmod.rs` | `without_a_table_the_loader_is_what_it_was`、`a_table_does_not_change_what_it_does_not_name` | `LoadLibraryW("dwmapi")` → `returned 0 le 126` | → `returned 7ff800270000 le 5`（定数 `DWMAPI_HANDLE`）。不在の確認は `propsys` で行う | PM の項目 5 |
| `vcall_thunk_dll.rs` | `after_the_memory_form_jmp_only_the_import_thunks_remain_unsupported` | FLEngine の未対応 thunk 371 | 355（= 371 − 16。16 は生成 C の比較で測った thunk） | 16 本の thunk が答えを持った |
| 変えていない | `tid.rs` と 14 件の「2 本目のスレッド」テスト | | | ホストの pthread を使う検査で、`fle_host_tid()` は不変のため通る |
| 注意 | 全体テストで最初に落ちた 4 件（`realmod` 2、`vcall_thunk_dll` 1、`wall_fr4` 1）は、**更新漏れ**だった。部分実行では当たっておらず、全体の 1 回目で初めて見えた | | | |

## 実装した場所

- 新規: `lift/runtime/fle_thread.c`（1,118 行）／`fle_thread.h`（183 行）、`lift/crates/fe-lift/tests/wall_fr8.rs`（45 件）、`docs/decisions/0122-cooperative-threads.md`。
- 変更: `fle_event.c`／`.h`、`fle_sys.c`（仮想時計の前進、CS の所有者と競合、`fle_sys_cs_try_take`）、`fle_imports.c`（スレッドごとの TLS・last error・FLS、`GetCurrentThreadId`）、`fle_proc.c`、`fle_locale.c`、`fle_callback.c`／`.h`（`fle_callback_depth_set`）、`fle_vex.h`（`fle_teb_cur`／`fle_tls_cur`）、`fle_win.c`／`.h`、`fle_realmod.c`／`.h`、`fle_modules.c`、`fle_load.c`、`imports_table.rs`（15 行を末尾に追加）、`harness/fruity.c`。runtime は `-Wall -Wextra -Werror` で通る。
- 変えていない: `fle_vfs.c`（壁 22 の答えは入れていない）、fe-decode、fe-pe、擬似 VA（既存の行は動かない）。

## 出典なしの選択の一覧（撤回条件つき）

| # | 場所 | 選択 | 撤回条件 |
|---|---|---|---|
| S1 | E2 | **どのスレッドが次に走るか**: 作成順の巡回、優先度は無視（`SetThreadPriority` は値を保持するだけ） | 実機の挙動の差がゲストの出力に出ると分かったとき。PM が別の方針（T1／T3／T4）を選んだとき |
| S2 | E1 | **ハンドルの値**（`0x7FFAA0000000 + 4*(i+1)`、再利用なし）と**スレッド ID**（0x1001 + 添字） | ゲストがハンドルや ID の値に依存したとき |
| S3 | E1 | スレッド表の上限 256（超えると ERROR_NOT_ENOUGH_MEMORY（8）。ページは上限の失敗コードを書かない） | 256 本を超えるゲストが現れたとき |
| S4 | E1 | **スタック寸法**: ゲストのスタックは全量をコミット、下限 64 KiB（既定 1 MiB はページの値）。ホストの C スタックは 16 MiB（この環境の制限） | スタックの使用量が超えたとき |
| S5 | E1 | MAXIMUM_SUSPEND_COUNT = 127、MAXIMUM_WAIT_OBJECTS = 64（ページは定数名だけ。値は非公式の winnt.h） | 実機の値と違うと分かったとき |
| S6 | E1 | 新しいスレッドの初期状態: 汎用レジスタ 0、フラグ未定義、x87 は reset（CW 037F／SW 0／TW FFFF）、MXCSR 1F80、`rsp = スタック上端 − 64`（16 揃え） | ゲストがスレッド開始時のレジスタに依存したとき |
| S7 | E3 | `WaitForMultipleObjects` の `nCount` が 0 または 64 超のとき 87（ページは「できない」と書き、コードを書かない） | 実機の値と違うと分かったとき |
| S8 | E3 | `WaitForMultipleObjects` の同じハンドルの重複は停止（ページは「してはならない」とだけ書く） | ゲストが重複して渡したとき |
| S9 | E4 | 窓でない `hWnd` を渡された `PostMessageW`／`GetMessageW` は 0／−1＋last error 1400（決定 0119 の 1400 に合わせた）。`PeekMessageW` は停止（ページが失敗値を書かない） | 実機の値と違うと分かったとき |
| S10 | E4 | `PostQuitMessage` の WM_QUIT はキューに 1 つだけ保持し、2 回目はコードを置き換える。`pt = (0, 0)`。WM_QUIT を `PostMessageW` で投げると停止 | ゲストが 2 回呼んだとき |
| S11 | E4 | `TranslateMessage` は常に 0（キー系メッセージは停止。キーボード配列は持たない）。`DispatchMessageW` のスレッドメッセージ（hwnd NULL）は 0 を返し何も呼ばない（「dispatch できない」というページの読み） | キー入力を扱う段階 |
| S12 | E5 | **DwmFlush が待つ境界**: 60 Hz（決定 0098 の `EnumDisplaySettingsW` の 60 Hz と整合）、フレーム k は `ceil(k·10^7/60)` 刻み、現在より後ろ。ページは何を・どれだけ待つかを書かない | 実機の挙動の差が出たとき。PM が別の周期にしたとき |
| S13 | E6 | DllMain の通知の順序（ATTACH はロード順、DETACH は逆順）。`lpvReserved = NULL` | 実機の順序と違うと分かったとき |
| S14 | E4 | 投稿メッセージの `time` は仮想時計の ms | ゲストが `MSG.time` を使ったとき |
| S15 | E2 | スレッド終了時の仮想時計は進めない。`STILL_ACTIVE` 259 の使い方はページどおり | — |

壁 22 の `GetFileAttributesW("")` の答え（INVALID_FILE_ATTRIBUTES＋3）、`GetSysColor` の色、`IsWindowVisible` などの窓の答え、`VirtualQuery` の答えは、**選択として入れていない**。PM の判断（下）。

## テスト

新規 `tests/wall_fr8.rs`（**45 件**）。期待値は決定の算術と Learn の文言から書き、C のコードから読み戻していない。実 `fle_thread.c`／`fle_win.c`／`fle_event.c`／`fle_sys.c`／`fle_imports.c`／`fle_proc.c` などをリンクし、ゲスト関数を C のスタブで置いた駆動プログラムで検査する。

- 生成器（3）: 15 行が末尾に決めた順で並び擬似 VA が動かない、各 thunk は自分の DLL でだけ答える、答えない名前（`TerminateThread`、`SleepEx`、`OpenThread` など）は行でない。
- スレッド API とスケジューラ（約 18）: ID・ハンドル、`flags 0` は作った側が待つまで走らない、`CREATE_SUSPENDED` と前の中断カウント、巡回が作成順、仮想時計が最小の期限へ進む、`SwitchToThread` の 1／0、デッドロックの名前付き停止（スレッド表つき）、スレッドハンドルの待ち、`WaitForMultipleObjects`、auto-reset イベント（最初に待ったスレッドへ）、競合した CS と非所有者の Leave、`ExitThread`、`CloseHandle`、未掲載のフラグ・属性・不正なハンドルの停止、開始アドレスが関数入口でないときの停止。
- メッセージ（約 14）: 窓の所有スレッド、所有スレッドのキューへ、`hWnd = NULL`、`PeekMessageW`、WM_QUIT の順序、空キューで待ち他が走る、`WaitMessage` の「新しい」入力、`MsgWait` の `n`／インデックス／タイムアウト、待ち中の投稿、他スレッドの窓への `SendMessageW`、`DispatchMessageW`、キュー上限 10,000／1816、ブロードキャスト等の停止、idle 停止の報告。
- スレッドごとの状態（約 7）: last error・TLS、`SetThreadPriority`、ロケール、R／X／FL／x87／MXCSR の切り替えをまたぐ保持、`gs:` の TEB・スタック境界・TLS ポインタ、コールバック深さと窓プロシージャ内の切り替え、DllMain の ATTACH／DETACH（合成した実モジュール）、環境変数で通知を切る。
- dwmapi（3）: N=39 とハンドル、`DwmFlush` が 60 Hz の境界まで待つ、境界の算術（手計算の点）。

## 検証（実測）

- **全体テスト**（`logs/full/cargo-test.txt`、`start.txt`、`end.txt`、`rc.txt`）: 上の表のとおり。環境変数は `tools/env.sh`（`FLENGINE_PE`／`FLENGINE_DLL` に原本、`BLEND2D_PE` にリベース済みの像、など決定 0121 と同じ）。
- **全体テストの経過（正直な記録）**: 通しを 2 回行った。**1 回目（`logs/full-run1-INVALID-stale-mut-binaries/`）は無効**: 123 result・1,666 passed・**95 failed**。原因は私の誤り: 変異検査が `mut/lift`（木のコピー）から作ったテストバイナリを共有の `CARGO_TARGET_DIR` に残し、cargo が `src/lift` に対して新しいと判断し、`CARGO_MANIFEST_DIR` が削除済みの `mut/lift` を指すバイナリで走った（失敗メッセージに `out-fruity8/mut` が含まれるもの 83 件、パスを出さない同種のもの 8 件）。残り 4 件は本物（上の「更新漏れ」）。1 回目を始める前にこれに気づけなかった。`target` を消し、4 件を直し、2 回目を通した。**数えるのは 2 回目の 1 回**。
- **全体テストの後のソース変更（1 件、コメントのみ）**: `imports_table.rs` のコメントが「16 rows」と書いていたのを「15 rows」に直した。直した後に `fe-lift` を再ビルドし、SHA-256 が変わらない（`f7c33229748a5a0df850fbad1d19269e2daae58b319c87d19ae97d124fdf8ef5`）ことと `wall_fr8` の 45 件が通ることを確認したが、**全体テストは再実行していない**（PM は 1 回の通しを求めた）。
- **FRUITY 無効の a4**（`logs/a4/fin-off1`、`fin-off2`。別の OUT、再生成・再ビルド）: 上の表。2 回とも同一の数字・同一の native SHA。使った `fe-lift` は SHA-256 `f7c33229…8ef5`、`fe-pe` は `743e2faf…70b0`（決定 0121 の `fe-pe` と同じ）。1 回の所要は約 1 分 2 秒〜1 分 12 秒、最大 RSS 約 2.7 GB。
- **entries の +1,242 の原因**（実測。`wall-log.md`）: (1) 新しい 17 API（`CreateThread` …）の呼び出しは a4 無効の全走行で **0 回**（gdb で数えた）。(2) dwmapi を不在に戻した木のコピーで a4 無効を走らせると **entries 1,007,947、functions 5,308 で基準と完全一致**。(3) 新しい木では `GetProcAddress` が 1,681 回、コピーでは 1,612 回。余分な 69 回はすべて dwmapi のハンドル（0x7ff800270000）への呼び出しで、**同じ 23 名の問い合わせ 3 回**（`DwmEnableBlurBehindWindow` … `DwmFlush` … `DwmDefWindowProc`）。HEAD では `LoadLibraryW("dwmapi")` が失敗してゲストは「DWM なし」の道を通り、仮想モジュールがあると 23 名を問い合わせて 22 名は NULL＋127、`DwmFlush` だけが擬似 VA を得る。余分な入口は、それぞれの答えを処理する lifted code。**新しい関数は 0**。
- **FRUITY 有効の最終構成**（種 = nsis + app、主張の表あり。`logs/a4/fin-on1`、`fin-on2`）: 上の表。2 回とも同一。シャドウスタックの先頭は `f_4234f0`（`GetFileAttributesW` の thunk）← `f_414850` ← `f_43ba70` ← `f_423b80`（`SetThreadErrorMode`）← `f_819110` ← … 。停止時の窓は 7 つで、すべて 0x1001 の所有（決定 0121 の 6 つ + `TPUtilWindow` 1 つ）。メッセージキューは空。
- **生成 C の比較**（`wall-log.md`）: HEAD の木で FRUITY 無効を再現し（`cmpbase`。native `60a25acc…`、entries 1,007,947）、`fe-lift` の旧新を FLEngine（`--pdata --hook-points flengine-hook-points.txt`）と m1〜m5（それぞれの seeds と hook points）に同じ引数で走らせ、`void f_XXXX(void)` の本体ごとに SHA-256 を比べた。再生成した各ファイルの SHA-256 は a4-multi が記録した値と一致した（FLEngine 旧 `b92c4b12…`、新 `c9a443eb…`）。**変わった本体 42**（FLEngine 16: `CreateThread` 2、`ExitThread`、`GetExitCodeThread`、`ResumeThread`、`SuspendThread`、`WaitForMultipleObjects`、`DispatchMessageW` 2、`GetMessageW`、`MsgWaitForMultipleObjects`、`PeekMessageW`、`PostMessageW`、`PostQuitMessage`、`TranslateMessage`、`WaitMessage`。m2 3、m3 9、m5 14。m1・m4 は 0）。ファイルの本体以外の差は、15 の `fle_imp_*` 宣言、dispatch 表 15 行、import 表 15 行、`fle_resp_count`／`fle_dispatch_count`（m1 の diff で確認）。
- **変異検査**（`logs/mutations.txt`、`tools/mutate.py`。木のコピー `mut/` で 1 件ずつ変更 → 該当テストを実行 → `src/` から戻す。最後に `diff -rq` が空）: **8 件とも検出**。M1 巡回を逆回り（`wall_fr8` 5 件が落ちる）、M2 `ResumeThread` が新しい中断カウントを返す（2）、M3 WM_QUIT を他の投稿より先に配送（1）、M4 60 Hz の境界を「現在以上」にする（1）、M5 キュー上限 10001（1）、M6 last error を 1 つの大域に戻す（1）、M7 `CREATE_SUSPENDED` を無視（2）、M8 dwmapi を N=38 に（`wall_fr8` 1、`modules` 4、`load` 1。`wall_fr7` は通る = 添字を見ないため）。**変異検査はテストが変異した規則を見ていることを示すだけで、その規則が Windows の挙動であることは示さない。** 経過: 変異検査のために作った木のコピーが、上の無効な 1 回目の原因になった。

## 止まった理由

停止点は壁 22（`kernel32!GetFileAttributesW("")`、51,529 入口）。分類は PM の止まる条件 **(c) 偽らないと進めない／(f) PM の値**（上の「壁 22 の判断」）。これはスレッドの壁ではなく、**最初のスレッドがゲストの初期化の中で一度も待たない（実測）ため、作った 2 本のスレッド（QFC の TTimerThread = 開始 0x1d0fdb0、FLEngine の別のスレッド = 開始 0x412ae0）は一度も走っていない**。壁 22〜25 はどれも最初のスレッドの上で起きる。スケジューラ・キュー・DwmFlush は、スタブのゲスト関数を使うテスト（45 件）と、実ゲストの中での `CreateThread`／`SetThreadPriority`／`ResumeThread`（41,177〜41,187 入口）だけで検査され、**ゲストのスレッド本体が走ったときの挙動は測っていない**。

## 解釈が分かれた点

1. **壁 22 を (c)／(f) と読んだこと**: 1 行で書ける答えを入れずに止めた（上）。過剰に慎重なら PM が覆せる。
2. **`DwmFlush` のページと調査の引用の差**: `thread-model.md` の引用は現物のページと違う（上の「背景」）。PM の規則（60 Hz の境界）はページの有無にかかわらず出典なしの選択。
3. **`ResumeThread` の「前の中断カウント」**: ページは前のカウントを返すと書く。実ゲストの呼び出し（0x1001 → 0x1002、41,187 入口）は前のカウント 1 を受け取った（実測）。TThread 側の `== 1` の検査は、ゲストのコードを読んだ上の推定で、スレッド本体が走るまで未確認。
4. **entries の基準が動いたこと**: PM の規則は「基準を動かしてよい。原因は説明する」。原因は項目 5（dwmapi を仮想モジュールに）。PM が dwmapi を不在のままにすると言えば、`fle_load_absent[]` に戻すだけで基準は 1,007,947 に戻る（実測）。ただしそうすると QFC の `DwmFlush` の遅延ロードは壁になる。
5. **DLL_THREAD_ATTACH／DETACH を実装したこと**: 大きな設計にならなかったので実装した（PM の項目 6）。`FLE_THREAD_NO_DLL_NOTIFY=1` の保険は Windows の挙動ではない。実モジュールを持つ構成でスレッドが実際に走るのは壁 22 の先なので、ゲストの中での通知は**未測定**（合成した実モジュールのテストだけ）。

## PM の判断が要る点

1. **壁 22: `GetFileAttributesW("")` の答え**。選択肢: (A) INVALID_FILE_ATTRIBUTES＋ERROR_PATH_NOT_FOUND（3）（Windows の実際の答えだと私は記憶しているが、どのページにも無い。**未確認**）、(B) 名前付き停止のまま、(C) 実機での測定を待つ。決定 0116 の壁 1 と同じ判断なので、同じ答えにするかどうか。
2. **壁 23: `GetSysColor` の値**。Learn は要素名を挙げ、色を書かない（ページの表は Aquatic のコントラストテーマ用）。決定 0108 は添字 8 だけ黒にした。COLOR_3DFACE（15）の値をどうするか。実験では全添字に 0xC0C0C0（作った値）を入れて先へ進めたが、**これは差分に入れていない**。
3. **壁 24: 窓管理のモデル**（`ShowWindow`、`IsWindowVisible`、`SetWindowPos`、`GetWindowRect`、フォーカス、キャプチャ）。決定 0119 の「窓は可視でない」を拡張するのか、可視性・Z 順のモデルを作るのか（大きなモデル）。
4. **壁 25: 追跡していないメモリへの `VirtualQuery`**（`FindHInstance`）。runtime が `VirtualAlloc`／ファイルビュー以外のメモリを追跡しない。モデルを広げるか、止めるか。
5. **60 Hz の `DwmFlush`**（S12）、**作成順の巡回**（S1）、**ハンドル・ID の値**（S2）を、このまま「仮」にするか。
6. **dwmapi の基準の動き**（解釈 4）を受け入れるか。
7. **wasm（S3）**: 作っていない。スレッドの切り替えは `makecontext/swapcontext`（native のみ）。ブラウザは JSPI／Asyncify の設計が要る（調査 `thread-model.md`）。ビルドは emscripten では「最初の `CreateThread` で名前付き停止」にしてある。
8. **docs の索引**: `docs/README.md` の 0122 の行は `docs/README-0122-line.txt`（この作業では `docs/README.md` を編集していない）。HEAD の `docs/README.md` が c6ac416 から変わっている（9f2d9c9）ので、足す側は 9f2d9c9 の行を見る。

## 撤回条件

- **E1〜E3（スレッドと待ち）**: PM が T1／T3／T4 を選んだとき。ゲストのスレッド本体が走って実機と違う順序・結果を出すと分かったとき。
- **E4（キュー）**: ゲストが WM_TIMER／WM_PAINT／キー入力を必要としたとき。窓でない `hWnd` の値が実機と違うと分かったとき。
- **E5（dwmapi／DwmFlush）**: PM が dwmapi を不在に戻すと決めたとき（`fle_load_absent[]` に戻す）。実機の `DwmFlush` の待ちが違うと分かったとき。
- **E6（DllMain の通知）**: 実機の順序が違うと分かったとき。通知がゲストの実モジュールで問題を起こすとき（`FLE_THREAD_NO_DLL_NOTIFY=1`）。
- **S 一覧の各項目**: 表の撤回条件。

## 限界

- 実機 Windows との照合は 0 件。wasm・ブラウザは未測定（native のみ）。
- 目的 (a) にも (b) にも**到達していない**。停止は壁 22。壁 23〜25 は実験で見えただけで、直していない。
- **ゲストのスレッド本体（QFC の TTimerThread、FLEngine の別のスレッド）は一度も走っていない**。スケジューラ・キューの挙動は、スタブのスレッドを使うテストでだけ検査した。
- スレッドの切り替えは native の `makecontext/swapcontext` だけ。ブラウザの JSPI／Asyncify は未設計。
- `MsgWaitForMultipleObjects` の `fWaitAll = TRUE`、`HWND_BROADCAST`、`WM_TIMER`／`WM_PAINT`、キー入力、`TerminateThread`、`PostThreadMessageW`、`OpenThread` は答えない（停止）。プロセッサ数は 1 のまま。
- 時間と RAM: 6 モジュールのビルドが約 1 分、最大 RSS 約 2.7 GB。全体テストは 45 分 52 秒。
