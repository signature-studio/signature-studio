# 0050: イベント同期 API（CreateEventW、SetEvent、ResetEvent、WaitForSingleObject、CloseHandle フック）を単一スレッドの状態機械として答える

状態: PM の決定（`docs/investigations/sync-thread-survey.md` §6 への回答）に従う作業者の実装判断。実装は `lift/runtime/fle_event.h`、`fle_event.c`、`lift/crates/fe-lift/src/imports_table.rs`（4 行追記）、テストは `lift/crates/fe-lift/tests/event.rs`、実行記録は `docs/investigations/event-sync-run.md`。「公式」は Microsoft Learn（2026-10-21 取得）、「非公式」はそれ以外。

## 文脈

harness の停止点は `CreateEventW`（thunk 0x423220、呼び出し元 0x524c40: Delphi RTL の `InitThreadSynchronization` と読める。調査 §4.1）だった。PM の回答: (1) 案 C を実装する（スレッドが作られる箇所に届いたら名前付き abort。スレッド生成は実装しない）。(2) CreateEventW の last error は成功時に変えない。仮想時計を進める窓口は必要になるまで足さない。(3) ハンドル領域は 0x7FFA80000000 から。(4) 先取りはしない。(5) IsDebuggerPresent は実装しない。

## 決定

### A. 実装する API（4 つ + CloseHandle のフック 1 個）

`CreateEventW`、`SetEvent`、`ResetEvent`、`WaitForSingleObject`（`imports_table.rs` の `TABLE` に 4 行）。`CloseHandle` は既存の `fle_imp_CloseHandle`（fle_vfs.c）が登録済みクローズフックに先に尋ねるので、`fle_event.c` が初回の `CreateEventW` でフックを 1 個登録する（`fle_vmap.c`、`fle_vol.c` と同じ `hook_once` の流儀）。

- **フックの使用数: 4 のうち 3（vol、vmap、event）**。上限 4 には達していない。残りは 1。4 個目を足す作業が出たら、そこで上限の拡大を検討する。テスト `the_runtime_sources_use_at_most_four_close_hooks` が登録の数を数える（3 でなければ失敗）。
- **`WaitForSingleObjectEx` は実装しない**。依頼文は「同じ（bAlertable は FALSE のみ許可）」としていたが、調査の実測（`lift/out-sync/counts.txt`: thunk 0、地点 0。`strings_scan.txt`: DLL の文字列にも無い。DLL のバイト列の `strings` でも 0 件）で、この DLL は `WaitForSingleObjectEx` を import していない。PM の決定 4（先取りしない）に従い、ゲストが呼ばない API は足さない。テスト `rows_are_in_the_table_and_declared_in_the_header` が「TABLE に無い」ことを固定する（誰かが足すときは、bAlertable TRUE の abort と合わせて決める）。

### B. イベントの状態機械（公式の記述の範囲）

状態は {手動 / 自動リセット、シグナル / 非シグナル、生きているか}。1 回の `CreateEventW` ごとに表に 1 件。

| 呼び出し | 動作 | 根拠 |
|---|---|---|
| `CreateEventW(NULL, bManualReset, bInitialState, NULL)` | bManualReset 非ゼロ（BOOL は下位 32 ビット）で手動、0 で自動。bInitialState 非ゼロで初期シグナル。ハンドルを返す。**成功時に last error を変えない** | 公式 CreateEventW。last error は PM 決定 2 |
| `SetEvent` | シグナルにする。戻り値 1。手動も自動も、待機者がいない（単一スレッドなので常にいない）ので信号のまま残る | 公式 SetEvent: 「自動リセットは待機スレッド 1 つが解放されるまで信号のまま。待機者がいなければ信号のまま残る」「すでにセットされたイベントのセットは無効」 |
| `ResetEvent` | 非シグナルにする。戻り値 1 | 公式 ResetEvent |
| `WaitForSingleObject`（シグナル済み） | `WAIT_OBJECT_0`（0）。**自動リセットは非シグナルに戻す。手動は戻さない**。タイムアウトの値（INFINITE を含む）に関係なく即座に返す | 公式 WaitForSingleObject（戻り値、「オブジェクトの状態を変更するものがある」）、CreateEventW の自動リセットの記述 |
| `WaitForSingleObject`（未シグナル、ms = 0） | `WAIT_TIMEOUT`（0x102）。状態は変えない | 公式: ms = 0 は「待機状態に入らず、常に即座に戻る」 |
| 無効なハンドル（NULL、閉じ済み） | `SetEvent`/`ResetEvent` は 0、`WaitForSingleObject` は `WAIT_FAILED`（0xFFFFFFFF）。last error は `ERROR_INVALID_HANDLE`（6） | 戻り値は公式。**6 は選択（仮定）**: 3 つのページは無効ハンドルの失敗コードを書いていない。fle_vfs.c の `CloseHandle`（0018、公式に書かれていない 6）と揃えた |
| `CloseHandle`（イベント） | 閉じて 1。閉じ済みの再 Close は 0 と 6。閉じ済みのハンドルは以後の Set/Reset/Wait で 0 / `WAIT_FAILED` と 6 | 公式 CloseHandle（最後のハンドルが閉じられたらオブジェクトは破棄される。このランタイムは 1 イベントにつきハンドル 1 個）。再 Close の 6 は fle_vfs.c と同じ推測 |

数値: `WAIT_OBJECT_0` = 0、`WAIT_TIMEOUT` = 0x102、`WAIT_FAILED` = 0xFFFFFFFF は公式（WaitForSingleObject の戻り値の表）。`INFINITE` = 0xFFFFFFFF は公式ページの本文に数値が無く、windows-sys（docs.rs、非公式）で確認した。`ERROR_INVALID_HANDLE` = 6 は fle_env.h の定義（System Error Codes の既存の記録）。

### C. 有限タイムアウトで未シグナルのときは abort にする（時計を進めて `WAIT_TIMEOUT` を返さない）

調査 §4.3 の最小案は「有限は仮想時計を ms 進めて 0x102」だった。PM の決定 2 は「仮想時計を進める窓口は必要になるまで足さない」なので、その案は取らない。残る 2 択を比べた。

- **時計を進めずに `WAIT_TIMEOUT` を返す**: 公式の `WAIT_TIMEOUT` は「タイムアウトの間隔が経過し、オブジェクトは非シグナル」。時計を進めなければ、決定 0017 の仮想時計（時間が進むのは `Sleep(ms)` と時計を読む呼び出しの後の 10 tick だけ）の上で、「間隔が経過した」と報告しながら何も経っていない状態になる。ゲストが `GetTickCount`/`QueryPerformanceCounter` で待った時間を測る、または「待った時間が ms に達するまで繰り返す」場合に、整合しない結果を返す（後者は時計を読むたびの 10 tick でしか進まず、ms = 100 の待ちを 10 万回繰り返す）。**公式の意味と 0017 の仮想時計に反する応答を黙って返すことになる**。
- **abort**: 0017 の方針（ページが定めない、または永遠に待つ入力は abort で止め、黙って成功にしない）と同じ。可逆（あとで時計の窓口を足したら、この 1 行を置き換える）。

**abort を選んだ**。メッセージは `a finite, nonzero time-out on an event that is not signaled needs the virtual clock to advance (decision 0050: not implemented)`。ms = 0 は時計と無関係なので `WAIT_TIMEOUT` を返す（B）。INFINITE で未シグナルは別メッセージで abort（待つ相手のスレッドがなく永遠に戻らない。`Sleep(INFINITE)` の 0017 と同じ扱い）。

実際にゲストがこの経路を踏むか（有限・未シグナル）は未確認。調査 §1.3: U に入る待ちで有限の定数は `0x5266a0`（MsgWaitForMultipleObjects、1000 ms。本決定の範囲外の API）だけ。`WaitForSingleObject` の U 内の 3 地点のうち、変数のものは TMonitor/MREW の競合時と `f_524db0`（unit 895/898 のみ）で、停止点の先の通常経路では踏まない見込み（調査の読み。推測）。

### D. ハンドル

`FLE_EVENT_HANDLE_BASE + 4 * (i + 1)`、`FLE_EVENT_HANDLE_BASE` = 0x7FFA80000000（PM 決定 3）、領域 0x10000000、再利用しない（閉じた値は二度と発行しない）。他の領域（0x7FFA00000000 〜 0x7FFA70000000。0x7FFA30000000 と 0x7FFA40000000 は 0024 が他担当の予約として空けたまま）と重ならないことを `tests/event.rs` が確認する。フックは領域の内側（`BASE < h < BASE + SIZE`）だけを引き受け、領域内で発行されていない値の `CloseHandle` は `fle_stop`（fle_vfs.c の既存の流儀: 発行していない値は止める）。

### E. 止める入力（fail-closed）

- `CreateEventW` の `lpEventAttributes` が非 NULL、または `lpName` が非 NULL（名前付きは `ERROR_ALREADY_EXISTS`、他のカーネルオブジェクトと共有する名前空間が要る。調査 §1.3: 閉包の名前付きは 4 地点で、いずれも U 外）。
- `SetEvent`/`ResetEvent`/`WaitForSingleObject` に、このランタイムが発行していないハンドル（他種のハンドル、`GetCurrentProcess`/`GetCurrentThread` の擬似ハンドル、`INVALID_HANDLE_VALUE`、領域内で未発行の値、4 の倍数でない値）。NULL と閉じ済みの値は上の失敗応答（B）。
- 別スレッドからの呼び出し（9 ファイルと同じ `owner_tid` 規則、`fle_host_tid()`。wasm では起きない）。
- イベントの `DuplicateHandle`（`fle_vol.c` が「このランタイムが発行していない」で止める。変更していない）。
- スレッド系（`CreateThread` など）、ミューテックス、セマフォ、タイマー、`WaitForMultipleObjects` 系、`MsgWaitForMultipleObjects`、`SignalObjectAndWait`、`OpenEvent`、`PulseEvent`、`IsDebuggerPresent` は実装しない（PM 決定 1、5。調査の閉包の外、または今回の範囲外）。

### F. 将来の拡張（A1 の JSPI 版）との関係

調査 §3.4: 待機可能なオブジェクトを 1 つの表（種類、状態）にし、領域を共有する。本実装は「イベントだけの表」で、種類のフィールドは持たない（先取りしない）。スレッドを足すときに、同じ 0x7FFA80000000 の領域へ種類を持つ表に広げる。待機の意味（シグナル済みなら即戻り、自動リセットは消費）は A1 の部分集合なので作り直しにならない（調査の予想。未検証）。

## 代替案

- 有限タイムアウトで時計を進めて `WAIT_TIMEOUT`（調査 §4.3）: PM 決定 2（窓口を足さない）に反するため見送り。
- 有限タイムアウトで時計を進めず `WAIT_TIMEOUT`: C のとおり却下。
- `WaitForSingleObjectEx` を足す: A のとおり、DLL が import していないので見送り。
- 全部 no-op（イベントを作るだけ、`Wait` は常に 0）: 手動/自動リセットの意味と未シグナル待ちの違反を見逃すため却下（0017 と同じ理由）。

## 限界

- 値（ハンドル、無効ハンドルの 6）は選択。実機の Windows と比べていない。
- 待機者がいる場合の `SetEvent`（自動リセットが待機者 1 つを解放して非シグナルに戻る）は、単一スレッドでは起きないので実装もテストもない。
- 停止点より先の実行で、4 つの API がどの経路から何回呼ばれたかは、`entry log` が最初の 256 件までなので全数は見ていない（`event-sync-run.md`）。
- 他のカーネルオブジェクト（ファイル、マッピング、プロセス/スレッドの複製）のハンドル値と 1 つの名前空間を共有する名前付きイベントは未実装。
