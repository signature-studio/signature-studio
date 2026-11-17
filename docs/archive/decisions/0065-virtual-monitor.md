# 0065: 仮想モニタ（EnumDisplayMonitors と GetMonitorInfoW）を固定値で答える

状態: 作業者（サブエージェント）が決定。方針（選択肢 B: コールバックをランタイムが呼ぶ。ホストの実ディスプレイを読まない。値は仮想で固定する）は依頼者（PM）。**HMONITOR の値、rcWork、lprcMonitor の置き場所、停止による列挙の戻り値は仮の決定**（下の表で出典を分けた）。実装は `lift/runtime/fle_monitor.h`、`fle_monitor.c`（コールバックは決定 0064 の `fle_callback.c`）、`lift/crates/fe-lift/src/imports_table.rs`（3 行追記）、テストは `lift/crates/fe-lift/tests/monitor.rs`（25 件）。調査と実行記録は `docs/investigations/callback-caller.md`。「一次」は Microsoft Learn（2026-10-22 取得）、「決定」は 0053 / 0057、「読み」はゲストの逆アセンブルの静的な読み、「非公式」は Learn 以外。

## 背景

harness は `fle: unsupported instruction at 0x5dca50`（`user32!EnumDisplayMonitors` の thunk）で止まっていた。呼び出し元は f_8651a0 の 0x8652ee（`EnumDisplayMonitors(NULL, NULL, 0x85df70, [obj+0xbc])`）。コールバック 0x85df70 は rcx（hMonitor）と r9（dwData）だけを使い、rdx と r8 は読まない（`next-stop-enumdisplaymonitors.md` §2、`callback-caller.md` §2）。GetMonitorInfoW の呼び出し点は 6 か所（0x807862、0x864fbd、0x864ffd、0x865037、0x86a9ea、0x8805c4）で、すべて cbSize = 0x28（実測: テスト `the_six_getmonitorinfow_sites_pass_cbsize_0x28...`）。

## 一次資料で確認した規則（2026-10-22 取得）

| ページ | 確認した内容 |
|---|---|
| [EnumDisplayMonitors](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-enumdisplaymonitors)（`updated_at` 2024-02-22） | `BOOL EnumDisplayMonitors(HDC hdc, LPCRECT lprcClip, MONITORENUMPROC lpfnEnum, LPARAM dwData)`。表: hdc NULL かつ lprcRect NULL は「Enumerates all display monitors. The callback function receives a NULL HDC.」。hdc NULL のとき対象は「the virtual screen that encompasses all the displays on the desktop」。dwData は「Application-defined data that EnumDisplayMonitors passes directly to the MonitorEnumProc function」。戻り値: 「If the function succeeds, the return value is nonzero. If the function fails, the return value is zero.」。**コールバックが FALSE を返したときの戻り値は書かれていない。GetLastError の記述も無い。** |
| [MONITORENUMPROC](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nc-winuser-monitorenumproc)（`updated_at` 2025-10-01） | `BOOL f(HMONITOR, HDC, LPRECT, LPARAM)`。hMonitor は「This value will always be non-NULL」。hdcMonitor は「This value is NULL if the hdc parameter of EnumDisplayMonitors was NULL」。lprcMonitor は hdcMonitor が NULL のとき「this rectangle is the display monitor rectangle. The rectangle coordinates are virtual-screen coordinates」。戻り値は「To continue the enumeration, return TRUE. To stop the enumeration, return FALSE.」。「calls a specified MonitorEnumProc function once for each display monitor in the calculated enumeration set」。 |
| [GetMonitorInfoW](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-getmonitorinfow)（`updated_at` 2024-11-20） | `BOOL GetMonitorInfoW(HMONITOR hMonitor, LPMONITORINFO lpmi)`。「You must set the cbSize member of the structure to sizeof(MONITORINFO) or sizeof(MONITORINFOEX) before calling … Doing so lets the function determine the type of structure you are passing to it.」成功は非 0、失敗は 0。**失敗の条件（不正なハンドル、cbSize の誤り）と GetLastError は書かれていない。** |
| [MONITORINFO](https://learn.microsoft.com/en-us/windows/win32/api/winuser/ns-winuser-monitorinfo)（`updated_at` 2024-02-22） | `{ DWORD cbSize; RECT rcMonitor; RECT rcWork; DWORD dwFlags; }`。rcMonitor は「the display monitor rectangle, expressed in virtual-screen coordinates」。rcWork は「the work area rectangle of the display monitor, expressed in virtual-screen coordinates」。dwFlags の定義済みの値は MONITORINFOF_PRIMARY「This is the primary display monitor」**（名前だけで数値は書かれていない）**。 |
| [MONITORINFOEXW](https://learn.microsoft.com/en-us/windows/win32/api/winuser/ns-winuser-monitorinfoexw) | MONITORINFO に `WCHAR szDevice[CCHDEVICENAME]` を足した構造体。 |
| [The Virtual Screen](https://learn.microsoft.com/en-us/windows/win32/gdi/the-virtual-screen)（0057 が取得済み） | 「The primary monitor contains the origin (0,0)」。 |
| [x64 calling convention](https://learn.microsoft.com/en-us/cpp/build/x64-calling-convention) | 決定 0064 を参照。 |

## 値の出典の区別

| 項目 | 値 | 出典 | 状態 |
|---|---|---|---|
| コールバックの引数の形 | (hMonitor 非 NULL, hdcMonitor NULL, lprcMonitor = モニタ矩形へのポインタ, dwData そのまま) | **一次**（MONITORENUMPROC と EnumDisplayMonitors の表） | 一次 |
| 呼び出し回数 | 1 回（モニタは 1 つ） | 一次（「once for each display monitor」）と、決定 0057 の SM_CMONITORS = 1 | 決定 |
| 矩形 rcMonitor | (0, 0, 1920, 1080) | 原点: 一次（The Virtual Screen）と決定 0057。大きさ: **決定 0053**（`FLE_GDC_HORZRES` × `FLE_GDC_VERTRES`。0053 自身が「出典のない仮の選択」と書く）。`fle_monitor.c` は `_Static_assert` で名前を結ぶ | 決定（0053 の仮を継ぐ） |
| dwFlags | MONITORINFOF_PRIMARY = 1 | 名前は一次。**数値 0x00000001 は Windows SDK ヘッダ**（mingw-w64 の `/usr/share/mingw-w64/include/winuser.h` 5885 行を読んだ）。Learn は数値を書かない。ゲストは `test eax, 1`（読み。`next-stop-enumdisplaymonitors.md` §2）で使う | **非公式**（ヘッダとゲストの読みが一致） |
| MONITORINFO の寸法 | cbSize 0x28、rcMonitor はオフセット 4、rcWork は 20、dwFlags は 36 | 構造体の定義は一次（DWORD + RECT + RECT + DWORD = 4 + 16 + 16 + 4）。ゲストの cbSize = 0x28 は読み（6 か所） | 一次から導出 |
| HMONITOR の値 | `0x7FEA0004`（`FLE_MONITOR_HANDLE_BASE + 4`）。領域 `0x7FEA0000` + 1 MiB | **なし**。Learn は値を書かない。「Interprocess communication」が下位 32 ビットだけが有意とするのは窓と GDI オブジェクトで、モニタは挙がっていない。同じ規則を**選択で**当てはめ、0x80000000 未満に収めた | **仮の決定** |
| rcWork | rcMonitor と同じ (0, 0, 1920, 1080) | **なし**。Learn は「work area」の値を書かない（タスクバーの有無を決めない）。仮想環境にタスクバーは無い、という**この環境の選択** | **仮の決定** |
| lprcMonitor の置き場所 | ゲストのスタック上、API の戻り番地より下の 16 バイト（16 バイト境界）。コールバックの呼び出しごとに書き直す | **なし**。Learn は「a pointer to a RECT」としか書かない。ゲストの 0x85df70 は読まないが、規約どおり有効なポインタを渡す。専用の領域（静的なバッファ）にすると、コールバックが書き換えた値が次の呼び出しに残る | **仮の決定** |
| EnumDisplayMonitors の戻り値 | コールバックの `al != 0` なら 1、`al == 0` なら 0 | 成功＝非 0 は一次。**FALSE で止めたときの戻り値は書かれていない**。PM の指示「FALSE（al == 0）で列挙を止める。戻り値は成功なら非 0」に従い、止めたときも「コールバックの BOOL をそのまま」とした。ゲストは戻り値を見ない（0x8652ee の直後は戻り値を使わない。読み） | **仮の決定** |

## 決定

1. **EnumDisplayMonitors**: `hdc == NULL` かつ `lprcClip == NULL` のときだけ。`lpfnEnum` を `fle_call_guest(lpfnEnum, 0x7FEA0004, 0, &rect, dwData)` で **1 回**呼ぶ（決定 0064）。`rect` は 16 バイトの RECT で、(0, 0, 1920, 1080) を呼び出しの前に書く。`al == 0`（FALSE）なら列挙を止める（1 モニタなので、結果は 1 回で終わるのと同じ）。戻り値は `al != 0 ? 1 : 0`。last error には触れない。
   - **コールバックの戻り値は `al` で判定する**（PM の規則）。Learn は BOOL（32 ビット）と書くので、`eax != 0` かつ `al == 0`（例 0x100）は実 Windows では TRUE、ここでは FALSE になる。ゲストのコールバックは `mov al, 1`（0x85df9f）で `al` だけを書くので、このずれはゲストでは起きない。他のゲストが使う別のコールバックを `fle_call_guest` で呼ぶときは、その API が戻り値の何ビットを見るかをその API の決定で決める（`fle_call_guest` は rax の 64 ビットを返す）。
   - `hdc != NULL`、`lprcClip != NULL`、`lpfnEnum == NULL` は `fle_stop`（`fle: EnumDisplayMonitors: ...`、理由と「decision 0065」付き）。hdc と clip は 64 ビット全体を見る。
   - `lpfnEnum` が関数の入口でない値のときは `fle_dispatch_miss` で止まる（決定 0064）。
2. **GetMonitorInfoW**: `hMonitor == 0x7FEA0004` かつ `cbSize == 0x28`（`lpmi` の先頭の DWORD）のときだけ。`rcMonitor = (0, 0, 1920, 1080)`、`rcWork = rcMonitor`、`dwFlags = 1`（MONITORINFOF_PRIMARY）を書き、1 を返す。書くのは構造体の 40 バイトだけ（cbSize はそのまま）。last error には触れない。
   - 別の hMonitor（NULL を含む）、`lpmi == NULL`、`cbSize == 0x68`（MONITORINFOEXW = 0x28 + WCHAR[32]）、`cbSize == 0x48`（MONITORINFOEXA）、そのほかの cbSize は `fle_stop`。**ページは失敗の条件を書かないので、0 を返さず止める**。停止のとき構造体には何も書かない（テスト `another_monitor_handle_stops...`）。
   - **PM の指示にあった「MONITORINFOEX の 0x48」は誤り**: `sizeof(MONITORINFOEXW)` は 0x28 + 2×32 = **0x68**、0x48 は `MONITORINFOEXA`（CHAR[32]）。この API は W 版なので 0x68 を「MONITORINFOEX」の停止とし、0x48 は別の理由文で止める。どちらも止まるので動作は PM の意図と同じ。
3. **MonitorFromWindow / MonitorFromRect / MonitorFromPoint は実装していない**。次の停止点（SPI 0x1f）の前には届いていない。MonitorFromWindow の 5 か所は窓（hwnd）の表が要る形で、規則により停止のまま。判断材料は `callback-caller.md` §6。
4. **追加で実装した API は 0 個**（EnumDisplayMonitors と GetMonitorInfoW だけ）。
5. ホストのディスプレイ・モニタ・EDID・環境・時計・スレッドは読まない（テスト `source_uses_no_host_display_monitor_clock_thread_process_or_file`）。

## rcWork と SPI_GETWORKAREA（決定 0058）の関係（今回は 0058 を変更しない）

rcWork（MONITORINFO）と `SystemParametersInfoW(SPI_GETWORKAREA = 0x30)` は同じ量（プライマリモニタの作業領域）。0058 は 0x30 を名前付きの停止にしており、今回は変えない。ゲストの呼び出し点は 4 か所（0x8190a6、0x866d1a、0x8673a5、0x11c3cd8）。

**0x30 に答えるときに満たす条件**:
1. `SPI_GETWORKAREA` の出力 RECT は、プライマリモニタの `rcWork` と**同じ値**にする。つまり `FLE_MONITOR_WORK_LEFT/TOP/RIGHT/BOTTOM`（`fle_monitor.h`）を使い、数字を複製しない。いまの値は (0, 0, 1920, 1080)。
2. 一方を変えるなら、同じ決定で両方を変える（タスクバーを導入する場合は、タスクバーの位置と寸法を決める決定を先に作り、rcWork、SPI_GETWORKAREA、SM_CXMAXIMIZED（0057 が停止にしている index 61）を一緒に決める）。
3. ゲストの 0x8190a6 は「SPI_GETWORKAREA が失敗（0 を返す）したら画面全体 (0, 0, SM_CXSCREEN, SM_CYSCREEN) を使う」形（読み。`spi-caller.md` §2.1）なので、0x30 が 0 を返す選択肢でも画面全体になる。ただし他の 3 か所が同じとは限らない（未追跡）。

## 選択肢

- HMONITOR の値: A. `0x7FEA0004`（採用。他の仮想ハンドルの領域の直後、0x80000000 未満）。B. 0x10001（Wine の `VIRTUAL_HMONITOR` が 0x10000 + 1。`/dll/win32u/sysparams.c` を読んだ。非公式）。C. 実 Windows の値（不明）。**採用理由**: 0053/0054/0059/0061 の流儀（領域を 1 MiB 取り、4 バイト刻み）に揃え、他の表と重ならないことをテストで固定できる。値自体に根拠はない。
- rcWork: A. rcMonitor と同じ（採用）。B. 下 40 ピクセルのタスクバーを引く。**採用理由**: タスクバーの高さに出典が無く、仮想環境に無い。B は 1080 − 40 = 1040 のような値を創作することになる。
- lprcMonitor の置き場所: A. 呼び出しごとにゲストのスタックの上に書く（採用）。B. 静的な 16 バイトのバッファ（C の static）をゲストのアドレスとして渡す。**採用理由**: B は native と wasm で host のアドレスの意味が違い（wasm は 32 ビット）、`FLE_CHECKED_MEM` の領域検査の範囲外になりうる。A は常にゲストのスタック内。
- FALSE のときの戻り値: A. コールバックの BOOL をそのまま 0/1 にする（採用）。B. 常に 1。C. 常に 0 で「失敗」とする。**採用理由**: ページは書かない。ゲストは見ない。A は PM の指示に沿う。
- MONITORINFOEX: A. 停止（採用。szDevice の値が決まっていない）。B. `\\.\DISPLAY1` を返す。**採用理由**: ゲストは使わない。
- 不正なハンドル: A. 停止（採用）。B. 0 を返す。**採用理由**: ページは失敗値の条件を書かない。他の仮想 API（0053 の GetDeviceCaps）と同じ扱い。

## 撤回条件

- Learn または実 Windows が、MONITORINFOF_PRIMARY の数値、HMONITOR の値の規則、rcWork の既定値、FALSE で止めたときの EnumDisplayMonitors の戻り値のどれかを示し、上の表と食い違う。
- ゲストが rcWork、rcMonitor の値（1920×1080）に依存して破綻する、または SPI_GETWORKAREA の応答と食い違いが出る（値の整合）。
- ゲストが HMONITOR を 32 ビットに切り詰めて使う、または HMONITOR を別の値の範囲として検査する（領域を変える）。
- hdc ≠ NULL、lprcClip ≠ NULL、MONITORINFOEX、複数モニタ、MonitorFromWindow（hwnd が非 NULL）に実行が届く（窓の表やモニタの表を設計する）。
- 0053 が解像度を変えた（`fle_monitor.c` の `_Static_assert` が落ちるので気付く）。または 0057 の SM_CMONITORS が 1 以外になる。

## 確認できていないこと

- 実 Windows が返す HMONITOR、rcWork（タスクバーあり／なし）、EnumDisplayMonitors の FALSE のときの戻り値（比較していない）。
- MONITORINFOF_PRIMARY の数値は Learn のページに無く、SDK ヘッダ（mingw-w64）とゲストの `test eax, 1` の一致のみ。
- コールバックの先（0x40f9e0、0x50a830 の中身）の意味は追っていない。動的に実行した 16 関数の `R[4]` の使い方だけ確かめた（決定 0064）。
- GetMonitorInfoW の他の 5 つの呼び出し点が、結果の rcWork をどう使うか（未追跡）。
