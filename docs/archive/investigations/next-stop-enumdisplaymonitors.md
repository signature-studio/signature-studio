# 次の停止点: user32!EnumDisplayMonitors（0x5dca50）の調査と判断材料（実装していない）

実行日: 2026-10-22。作業ディレクトリ `signature-studio-develop`、HEAD b3d88bf に決定 0063（GetKeyboardLayout）の変更を載せた状態。DLL は `binaries/win64/FLEngine_x64.dll`。「一次」は Microsoft Learn（2026-10-22 取得）、「実測」はコマンドの出力、「読み」は `lift/objdump-text.txt` の静的な読み、「推測」は未検証。成果物は `lift/out-kbd/`（gitignore 済み）。

**実装していない。** 理由: 別分野（ディスプレイ）の API であり、依頼の規則（GetKeyboardLayout と HKL の整合が要る API だけ追加してよい）の範囲外。さらにコールバック（ランタイムからゲスト関数を呼ぶ仕組み）が要り、設計判断が先に要る（§4）。

## 1. 停止点（実測。native と wasm が一致）

```
stop   : fle: unsupported instruction at 0x5dca50        (user32!EnumDisplayMonitors の thunk、IAT スロット 0x1971058)
depth  : init_count=115 / 942   last unit: index=114 run_rank=111/895 proc=0x623840 in_progress
         started=111 completed=110 in_progress=1 of 895 runnable   all_units_done=no
funcs  : executed (distinct)=582  entries=13379   unsupported_total=540
```

スタック: `f_5dca50` ← `f_8651a0`（return 0x8652f3）← `f_620cd0` ← `f_623840` ← `f_4120c0` ← `f_412160` ← `f_41d8f0` ← エントリ。ログは `lift/out-kbd/a4/20261021T200747Z-*`、`lift/out-kbd/survey1.txt`。IAT スロット 0x1971058 が EnumDisplayMonitors であることはインポートテーブルを Python で読んで確認した（`lift/out-kbd/iat-slots.json`）。

## 2. 呼び出し点（読み）

`EnumDisplayMonitors` の呼び出しは 2 か所で、どちらも `hdc = NULL`（`xor ecx, ecx`）、`lprcClip = NULL`（`xor edx, edx`）、`lpfnEnum = 0x85df70`、`dwData = [obj+0xbc]`（リスト）。

| 呼び出し点 | 関数 | 内容 |
|---|---|---|
| 0x8652ee（今の停止点） | f_8651a0（初期化。直線的なコード。GetKeyboardLayout の直後） | `[obj+0xbc]`（0x86528a で `0x40f9e0` が作ったリスト）を dwData に渡す。 |
| 0x867532 | f_867510（0x8674a0 で `[obj+0xbc]` の要素を破棄 → 再列挙） | 同じコールバック、同じリスト。f_867510 の呼び出し元は 0x85e02e、0x865c31（f_865c10）、0x866694、0x8675dd（f_8675c0）、0x869629 の 5 か所。 |

**コールバック 0x85df70**（MONITORENUMPROC。全体が 25 命令）: `rbx = rcx`（hMonitor）、`rsi = r9`（**元の dwData がそのまま渡る**。EnumDisplayMonitors は dwData をコールバックの第 4 引数にする）。`0x40f9e0(rcx = [0x8537f0], dl = 1)` でノードを作り（大きさは追っていない）、`[node+8] = hMonitor`、`[node+0x10] = [rsi+0x10]`（リストの現在の要素数。インデックスになる）、`0x50a830(list, node)`（push_back）を呼び、`al = 1`（列挙を続ける）で返る。**hdcMonitor（rdx）と lprcMonitor（r8）は読まない**。つまり ゲストはコールバックで hMonitor の値だけを使う。

hMonitor の使われ方（読み）:
- `GetMonitorInfoW(hMonitor, &mi)` の呼び出し 6 か所（0x807862、0x864fbd、0x864ffd、0x865037、0x86a9ea、0x8805c4）。0x864fbd、0x864ffd、0x865037 は `cbSize = 0x28`（MONITORINFO のサイズ）を渡し、結果の `[rbp+0x2c]`（rcMonitor の 2 つ目以降）、`[rbp+0x3c]`（rcWork 以降）、`[rbp+0x4c]`（dwFlags。`test eax, 1` = MONITORINFOF_PRIMARY の判定）を読む。
- `MonitorFromWindow` 5 か所（0x85dfc8、0x86748a、0x86a999、0x86a9b8、0x8805a9）、`MonitorFromRect` 2 か所（0x807849、0x867451）、`MonitorFromPoint` 1 か所（0x86741a）。dwFlags は `[0x1409ca8 + r8*4]` の表から取る。結果を `0x865c10` が要素と比べる（0x865b90 は `[node+8] == hMonitor` の等値比較でリストを検索する）。
- `EnumDisplaySettingsW` 2 か所（0x666d16、0x666dee）は別の経路。

## 3. Learn で確認できたこと（一次。2026-10-22 取得）

- **EnumDisplayMonitors**（`updated_at` 2024-02-22）: `BOOL EnumDisplayMonitors(HDC hdc, LPCRECT lprcClip, MONITORENUMPROC lpfnEnum, LPARAM dwData)`。hdc NULL かつ lprcClip NULL の組は「Enumerates all display monitors. The callback function receives a NULL HDC.」。hdc NULL のとき対象領域は「the virtual screen that encompasses all the displays on the desktop」。戻り値は「If the function succeeds, the return value is nonzero. If the function fails, the return value is zero.」。**GetLastError の記述は無い。**
- **MONITORENUMPROC**（`updated_at` 2025-10-01）: `BOOL f(HMONITOR, HDC, LPRECT, LPARAM)`。hMonitor は「This value will always be non-NULL」。hdcMonitor は「NULL if the hdc parameter of EnumDisplayMonitors was NULL」。lprcMonitor は hdcMonitor が NULL のとき「the display monitor rectangle. The rectangle coordinates are virtual-screen coordinates」。戻り値は「To continue the enumeration, return TRUE. To stop the enumeration, return FALSE.」。「calls a specified MonitorEnumProc function once for each display monitor in the calculated enumeration set」。
- **GetMonitorInfoW**（`updated_at` 2024-11-20）: `BOOL GetMonitorInfoW(HMONITOR hMonitor, LPMONITORINFO lpmi)`。「You must set the cbSize member … to sizeof(MONITORINFO) or sizeof(MONITORINFOEX)」。成功で非 0、失敗で 0。**失敗時の GetLastError は書かれていない。** MONITORINFO のページ（フィールド、MONITORINFOF_PRIMARY の値）は取得していない。cbSize 0x28 と dwFlags `test 1` はゲストのコードの読みだけ。
- **書かれていないこと**: HMONITOR の値、モニタ矩形の具体的な値、作業領域（rcWork）の値、主モニタのフラグの値（MONITORINFOF_PRIMARY の数値は MONITORINFO のページ）。

## 4. この lift 環境で既にある決定（関係するもの）と、判断が要る点

- 画面: 単一の仮想ディスプレイ 1920×1080、DPI 96、色深度 32（決定 0053 の `FLE_GDC_*`。**解像度は出典のない仮の選択**）。`GetSystemMetrics`: SM_CMONITORS = 1、SM_CXSCREEN/CYSCREEN = 1920/1080、SM_*VIRTUALSCREEN は原点 (0, 0)（決定 0057）。モニタ矩形は (0, 0, 1920, 1080)、列挙は 1 回で、これらと整合させるのが自然（推測）。
- 作業領域: `SystemParametersInfoW(SPI_GETWORKAREA = 0x30)` は 0058 が名前付きで**停止**にしている（ゲストの呼び出し 4 か所: 0x8190a6、0x866d1a、0x8673a5、0x11c3cd8。`tests/spi.rs` の SITES）。`GetMonitorInfoW` の rcWork は同じ量なので、一方を決めるともう一方も決まる必要がある（値の整合）。
- **コールバックの仕組みが無い（最大の判断点）**: `EnumDisplayMonitors` はゲストのコールバック 0x85df70 を呼ぶ。ランタイムがゲストの関数を呼んだ前例を探した（`runtime/*.c` の grep）。`fle_dispatch`（`fle_dispatch.h`）は生成 C から使われるだけで、ランタイムの API 実装がゲストの関数を呼ぶ例は無い（`fle_winmm.c` の「callback」は `midiOutOpen` のフラグの検査だけ）。呼ぶなら、生成 C の `call` と同じ形（`R[4] -= 8; fle_st(R[4], 8, <戻り番地>); fle_dispatch(0x85df70); R[4] += 8`、引数は R[1]、R[2]、R[8]、R[9]）でランタイムが呼ぶことになる。**呼び出し規約（スタック、callee-saved レジスタ）、再入、スタックのアラインメントを決める最初の例になる**。コールバックの先（0x85df70）は静的に既知で、`rdx`/`r8` を読まないので、仮の `hdcMonitor = NULL`（Learn どおり）と矩形へのポインタ（書き込み済みの領域）を渡せば足りる（読み）。

## 5. 選択肢

EnumDisplayMonitors（hdc = NULL、lprcClip = NULL のみ）:
- **A. 停止のまま**（今の状態）。
- **B. ランタイムがコールバックを 1 回呼ぶ**。HMONITOR は仮想ハンドル 1 つ（0054/0059/0061 と同じ種類の仮の決定。値の領域は 0x7FDA0000 + 1 MiB（0061 のカーソルの領域）の次などで重ならないものを選ぶ）、矩形は (0, 0, 1920, 1080)（0053、0057 と整合）。戻り値はコールバックが TRUE を返したら 1。hdc ≠ NULL、lprcClip ≠ NULL は停止（ゲストは使わない。読み）。同時に必要になる API: `GetMonitorInfoW`（rcMonitor、rcWork、dwFlags = PRIMARY の値）、`MonitorFromWindow`（5 か所）、`MonitorFromRect`（2 か所）、`MonitorFromPoint`（1 か所）。どれも「モニタは 1 つ」で単純に答えられるが、**rcWork（SPI_GETWORKAREA）と作業領域の値（タスクバーの有無）は出典がなく**、仮の決定になる。
- **C. 仮想ディスプレイ全体の設計**（複数モニタ、DPI、EnumDisplaySettingsW、SPI_GETWORKAREA、GetDeviceCaps との整合）。

## 6. その先の見通し（推測）

- EnumDisplayMonitors の次は、f_8651a0 の 0x8652f3 以降（`0x5c0d40` × 5、`0x8663b0`）。`0x8663b0` の中に `SystemParametersInfoW(0x1f)`（SPI_GETICONTITLELOGFONT）と `SystemParametersInfoW(0x29)`（SPI_GETNONCLIENTMETRICS）と `CreateFontIndirectW`、`GetStockObject` の呼び出しがある（読み。条件分岐 0x8663d5、0x8663f5 の評価は未確認）。0058 はこの 2 つの uiAction を名前付き停止にしているので、そこで止まる可能性が高い。
- GetKeyboardLayout の残りの 2 呼び出し点（0x84364c、0xa16502）は、この停止点までには実行されていない。

## 7. 限界

- MONITORINFO のページ、HMONITOR のページは取得していない（cbSize 0x28、dwFlags の読みはゲストのコードからだけ）。
- コールバック 0x85df70 の `0x40f9e0`、`0x50a830` の中身は追っていない（push_back とオブジェクト生成の形だけ読んだ）。
- 実 Windows の列挙の順序、HMONITOR の値は未測定。
