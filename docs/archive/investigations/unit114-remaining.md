# unit 114（proc 0x623840）を CreateWindowExW の先で完了させるために要るもの（調査。実装なし）

実行日: 2026-10-22。対象は A4 の init_count が 115 / 942 から動かない件。凡例: **実測** = コマンドの出力（a4-survey、gdb、生成 C）、**読み** = DLL のバイト列の静的な読み（capstone、`fe-lift` の生成 C）、**文書** = Microsoft Learn（2026-10-22 取得）、**仮説** = 未検証、**未確認** = 確認できていない。

## 0. 作業環境と方法

- 作業ツリーの `lift/` は読んでいない。`git archive HEAD` を `lift/out-u114/stage` に展開したコピーだけを使った。コピーでの a4-survey は既存の記録と同じ停止を再現した: `fle: unsupported instruction at 0x425780`、init_count 115 / 942、最後の unit は index 114（proc 0x623840、in_progress）、started 111 / completed 110、関数実行数 645、入口数 13,910（`lift/out-u114/build/a4/20261022T124356Z-*`）。
- コピーで `fe-lift lift ... --pdata` を実行し（生成 C は 59,297 関数、215 MB）、README の build_native.sh でネイティブの harness を作った。ランタイムとテーブルはコピーのまま。変更なし。
- 静的解析: `lift/out-u114/work/{pelib,closure,dump,tail}.py`（pefile と capstone。`.pdata` の関数範囲を使い、直接 call と `jmp [rip+x]` の IAT thunk を辿る閉包。間接呼び出しは辿れず、件数だけ数えた）。
- **探り（probe）**: ネイティブの harness を gdb で動かし、未実装の IAT thunk に当たったら**仮の値**を R[0] に入れて `return` し、続きを実行した（`lift/out-u114/probe/`、`lift/out-u114/work/probe*.py`）。実装ではない。ランタイムとテーブルのソースは変えていない。仮の値で得た結果は、ゲストが「どの API の後に何を要求するか」の測定であって、実機の値ではない。
- IAT 経由の呼び出しは thunk（`jmp [rip+x]`）に集約されている。表の thunk VA はその thunk、スロットは IAT スロット。

## 1. 結論

1. **CreateWindowExW を越えたあと unit 114 を完了させるのに、未実装の import は 2 つだけだった（実測）**: `user32!CreateWindowExW`（0x52a8be）と `user32!SetWindowLongPtrW`（0x52a8df）。仮の値（CreateWindowExW → 0x10001、SetWindowLongPtrW → 0）で unit 114 は完了し、unit 116（proc 0x630100）に入った。CreateWindowExW → NULL でも同じだった（`probe/seq3.*`）。ゲストはこの経路で HWND の値を検査しない（読み。実機では NULL は失敗の意味）。
2. この 2 つの間に、**ウィンドウプロシージャの呼び出し（`fle_call_guest`）は要らなかった**（実測）。unit 114 の中でゲストのウィンドウプロシージャは 1 度も実行されていない。クラスの `lpfnWndProc` が 0x424920（DefWindowProcW の thunk）であり、ゲストは SetWindowLongPtrW で差し替えた後、メッセージを送らないため（読み。§3）。
3. **f_867e10（TApplication のウィンドウ）、2 つ目の GetClassInfoW / RegisterClassW、UnregisterClassW、GetLastError を読む経路は、unit 114 では実行されない**（実測。§4）。f_867e10 は 0x867a65 の守り（`*[0x15dfcb8]` = 0x15e6084 の 1 バイト）で飛ばされる。この 1 バイトは `f_412160` の 0x41227a が 1 にしている（gdb の watchpoint で書き込み元を確認）。
4. 仮の値で進めると、unit 116〜143 は未実装 import に当たらず（unsupported の IAT thunk を 1 つも踏まない）、**unit 144（proc 0x65c540）の `CompareStringW`（既存のランタイムの停止）で止まる**（実測。init_count 145）。§6。
5. 注意（設計上の含意。仮説）: ゲストが SetWindowLongPtrW に渡す新しいウィンドウプロシージャは、**ゲストのヒープ上に動的に作られた機械語のスタブ**で、イメージ内の関数ではない（実測。§3）。後でメッセージを配送する段になると、`fle_dispatch(va)` はこの VA を知らず停止する。unit 114 の完了には影響しない。

## 2. unit 114 の呼び出しツリー（CreateWindowExW まで、および以降）

入口: `f_1376820`（DLL のエントリ）→ `f_41d8f0` → `f_412160` → `f_4120c0`（InitUnits）→ **`f_623840`（unit 114）**。

`f_623840`（0x623840）:

| 呼び出し点 | 呼び先 | 備考 |
|---|---|---|
| 0x623859 | thunk 0x4237e0 = kernel32!GetVersion | 既存のランタイムが応答 |
| 0x623869 | `f_620cd0` | 本体。下 |
| 0x623875 / 0x623881 / 0x623894 / 0x6238a7 / 0x6238ba / 0x6238cd | `f_4fb8a0`、`f_4fb980`、`f_4fb910` ×4 | f_620cd0 から戻った後。IAT 呼び出しは閉包に無い（MessageBoxA(delay) だけ。エラー報告の経路で、実行では踏まない） |

`f_620cd0`（0x620cd0。実測と読み）: GetCurrentProcessId（0x620cea）、GlobalAddAtomW（0x620d2c）、GetCurrentThreadId（0x620d47）、RegisterWindowMessageW（0x620d9f）、`f_620a20`（0x620dbf）、`f_8651a0`（0x620de5。EnumDisplayMonitors ほか、既存の停止点の連鎖）、**`f_867710`（0x620e00）**、`f_86b240`（0x620e1b）、`f_6082c0`（0x620e2c）、GetModuleHandleW(L"USER32")（0x620e3f。文字列 0x620f64）、`f_4fe190`（0x620e4e）、`f_4fe150`（0x620e6d。`f_4fe190` が 0 を返したときだけ）、GetProcAddress 包装 `0x4235f0`（0x620e81、名前 L"AnimateWindow" = 0x620f72）、`f_412d60` ×2（0x620e92、0x620e97）。

`f_867710`（0x867710。TApplication の構築に見える。仮説。DLL にシンボル名は無い）: 引数 rcx = クラス参照（`[0x…]`）、dl = 1、r8 = 0。

| 呼び出し点 | 呼び先 | 備考 |
|---|---|---|
| 0x867769 | `f_5267f0` | 既存の経路 |
| 0x86777c | ole32!OleInitialize（thunk 0x469140） | 0x867775 の守り（`*[0x15dfcb8]` ≠ 0）で**飛ばされる**（実測。OleInitialize は unit 114 で呼ばれていない） |
| 0x867934 | user32!LoadIconW(NULL, L"MAINICON") | 決定 0069 |
| 0x86799b | kernel32!GetModuleFileNameW(NULL, buf, 0x100) | 既存の応答 |
| 0x867a00 / 0x867a08 | user32!CharNextW、CharLowerW | 既存の応答 |
| **0x867a44** | **`f_858700`** → 0x858719 → **`f_52a7f0`** | 下 |
| 0x867a78 | `f_867e10` | 守りで**飛ばされる**（§4） |
| 0x867b03 / 0x867b0f / 0x867b2b | `f_86b9a0`、`f_86cb40`、`f_410360` | 新しい IAT 呼び出しは出なかった（実測。`f_86cb40` は 1 関数で API なし） |

`f_858700`（0x858700）は 16 バイト（`TMethod` = {code, data}）を rcx の指す先から局所へ写し、`f_52a7f0` に渡すだけ。ここでの値は、code = 0x869fd0（`lea rax,[rip+0x2594]`、0x867a35）、data = `self`（実測。gdb: code=0x869fd0、data=0x7ffff6b739c0）。

`f_52a7f0`（0x52a7f0。Delphi の `AllocateHWnd` に見える。仮説。`getclassinfo-caller.md` §3 と同じ読み）:

| VA | 呼び出し | 読み |
|---|---|---|
| 0x52a820 | GetClassInfoW（thunk 0x424ba0、スロット 0x1970a90） | 既存（決定 0070）。未登録なら 0 |
| 0x52a84d | UnregisterClassW（thunk 0x425670、スロット 0x1970c38） | Get が非 0 かつ wndproc ≠ 0x424920 のときだけ。実測: 呼ばれない |
| 0x52a859 | RegisterClassW（thunk 0x425260） | 既存（決定 0070） |
| **0x52a8be** | **CreateWindowExW**（thunk 0x425780、スロット 0x1970db8） | 未実装。§3 |
| 0x52a8cf | `f_52a5b0`（スタブの組み立て。`[rbx]` ≠ 0 のとき） | kernel32!VirtualAlloc（thunk 0x423ce0。0x52a5dd）。既存の応答 |
| **0x52a8df** | **SetWindowLongPtrW**（thunk 0x425980、スロット 0x1970c68） | 未実装。§3 |

戻り値の扱い: `f_52a7f0` は CreateWindowExW の戻り（rsi）を SetWindowLongPtrW の hWnd に渡し、最後に rax = rsi で返す。SetWindowLongPtrW の戻り値は**使わない**（0x52a8e4 以降、rax を上書き）。

## 3. CreateWindowExW 以降に到達しうる IAT 呼び出し（表）

分類: (a) 純粋・環境値のみ、(b) 仮想ウィンドウ表が要る、(c) コールバック（`fle_call_guest`）が要る、(d) その他。「応答済み」は `crates/fe-lift/src/imports_table.rs` の表の有無（224 行）。表にあっても、引数の形によっては停止する（fail-closed。例: CompareStringW）。

### 3.1 unit 114 の残りで実行された未実装の import（実測）

| API | DLL | thunk / スロット | 呼び出し元 VA | 引数の読み | 分類 | 応答 |
|---|---|---|---|---|---|---|
| **CreateWindowExW** | user32 | 0x425780 / 0x1970db8 | 0x52a8be（`f_52a7f0`） | dwExStyle = 0x80（WS_EX_TOOLWINDOW）、lpClassName = 0x52a7c8 = L"TPUtilWindow"、lpWindowName = 0x52a8f4 = L""（実測 r8）、dwStyle = 0x80000000（WS_POPUP）、x = y = w = h = 0、hWndParent = NULL、hMenu = NULL、hInstance = `[0x15ec298]` = 0x400000、lpParam = NULL。**x、y、w、h は 32 ビットだけ書かれ、スタック上の上位 32 ビットは不定**（実測: w の 8 バイトが 0x7fff00000000）。32 ビットで読む必要がある | (b)。(c) は、クラスの wndproc が 0x424920（DefWindowProcW）の間は不要（読み。§3.3） | **未応答** |
| **SetWindowLongPtrW** | user32 | 0x425980 / 0x1970c68 | 0x52a8df（`f_52a7f0`） | hWnd = CreateWindowExW の戻り、nIndex = `mov edx,0xfffffffc` = -4（GWLP_WNDPROC。**rdx の上位 32 ビットは 0。符号拡張されない**。実測 rdx = 0xfffffffc）、dwNewLong = スタブの VA（実測 0x7ffff7fb0fd8 = ゲストのヒープ。イメージ外） | (b) | **未応答** |

Learn（文書）: CreateWindowExW は WM_NCCREATE、WM_NCCALCSIZE、WM_CREATE をそのウィンドウに送る。失敗は NULL で GetLastError。SetWindowLongPtrW の戻りは「指定オフセットの以前の値。失敗は 0」。**未確認**: 失敗時の GetLastError の値、HWND の値の範囲、GWLP_WNDPROC の以前の値として実機が返す数値（ゲストはこの戻りを使わないので unit 114 には無関係）。

### 3.2 unit 114 の残りで実行された、応答済みの import（実測）

GetModuleHandleW(L"USER32")（0x620e3f）→ 仮想 user32 のハンドル 0x7ff800130000。GetProcAddress(それ, "AnimateWindow")（0x4235e0 から）→ **現状は NULL と ERROR_PROC_NOT_FOUND**（`fle_resp_table` に無い名前。`fle_modules.c` の既存の挙動）。ゲストは NULL を保存して先へ進む（スロット 0x13dfd78）。**後の unit が AnimateWindow を呼ぶか、NULL のままでよいかは未確認**（unit 144 までに呼ばれた形跡は無いが、全体は見ていない）。ほかに EnterCriticalSection、LeaveCriticalSection、TlsGetValue、GetCurrentThreadId、WideCharToMultiByte など（既存の応答）。

### 3.3 ウィンドウプロシージャの呼び出しが要るか（(c) の判定）

- 登録時の `lpfnWndProc` は 0x424920（user32!DefWindowProcW の thunk。`getclassinfo-caller.md` §2.2、WNDCLASSW 0x13be810 の +8 が 0x424920。実測）。CreateWindowExW が WM_NCCREATE などを送る時点では、wndproc は DefWindowProcW（この時点でゲストの関数ではない）。ランタイムが内部で DefWindowProcW の既定動作を扱えば、`fle_call_guest(0x424920, …)` は要らない（読み。仮説: `fle_dispatch(0x424920)` に行くと、DefWindowProcW の thunk が `fle_unsupported` で止まる）。
- SetWindowLongPtrW(GWLP_WNDPROC) の後にゲストがこのウィンドウへメッセージを送る箇所は、unit 114 の実行経路に無かった（実測: 未実装 thunk に当たるのは CreateWindowExW と SetWindowLongPtrW の 2 回だけ。SendMessageW、PostMessageW、DispatchMessageW などは unit 144 まで出なかった）。

**ゲストが渡すスタブの正体（実測と読み）**: `f_52a5b0` は VirtualAlloc(NULL, 0x1000, MEM_COMMIT, PAGE_EXECUTE_READWRITE = 0x40) で 0x1000 バイトのブロックを取り、その先頭 +8 に `pop r11; jmp [rip+0]`（バイト `41 5b ff 25 00 00 00 00` + 絶対アドレス 0x52a570）、各スロットに「`e8 rel32`（call ブロック+8）+ TMethod {code, data}」の 21 バイトを書く。SetWindowLongPtrW に渡されるのはそのスロットで、gdb で読んだバイトは `e8 2b f0 ff ff | d0 9f 86 00 00 00 00 00 | c0 39 b7 f6 ff 7f 00 00`（rel32 = -4053 → ブロック+8、TMethod = {0x869fd0, 0x7ffff6b739c0}）。0x52a570 は Delphi の `StdWndProc` に見える（仮説。名前は DLL に無い）: `edx`（msg）、`r8`、`r9` を局所の 4 スロット構造体に入れ、`rcx = [r11+8]`（self）、`rdx = &構造体` で `call [r11]`（= 0x869fd0）、戻りは構造体の +0x18。**ランタイムがこのウィンドウへメッセージを配送するなら**、スタブのバイトを解読して `R[11] = スロット+5` を設定し `fle_call_guest(0x52a570, …)` を呼ぶ形になる（仮説。このスタブ自体は lift されておらず、実行できない）。unit 114 の完了には不要。

0x869fd0 の閉包（静的、286 関数）: 未応答の IAT は DefWindowProcW（thunk 0x424920）、MessageBoxA(delay)、GetLogicalProcessorInformation(delay) だけ（読み。間接呼び出し 53 か所は辿っていない）。

### 3.4 静的に到達するが、unit 114 では実行されなかったもの

`f_867e10`（0x867e10。TApplication のウィンドウ作成に見える。仮説）を呼ぶ 0x867a78 は、0x867a65 の守りで飛ばされる（実測。§4）。参考として、f_867e10 の静的閉包（179 関数）にある未応答の import:

| API | DLL | thunk / スロット | 最初の呼び出し点（呼び出し連鎖） | 分類 |
|---|---|---|---|---|
| CreateWindowExW | user32 | 0x425780 | 0x867f7f（f_867e10。style 0x84ca0000、位置は画面中央、`GetSystemMetrics(0/1)`） | (b) |
| SetWindowLongPtrW | user32 | 0x425980 | 0x86800e（hwnd = `[self+0x2b0]`、-4、新 wndproc = `[self+0x2b8]`） | (b) |
| SendMessageW | user32 | 0x425390（0x425380、0x4253b0 も） / 0x1970c48 | 0x868041（WM_SETICON = 0x80、wParam = 1、lParam = `f_869680` の戻り） | (b) + (c) |
| SetClassLongPtrW | user32 | 0x4259a0 / 0x1970eb0 | 0x868065（GCLP_HICON = -14） | (b) |
| GetSystemMenu | user32 | 0x424ed0 / 0x1970e38 | 0x868077（bRevert = 0） | (b) + メニュー（**実機の戻りは未確認**） |
| DeleteMenu | user32 | 0x424940 / 0x1971118 | 0x86808a、0x86809a（SC_… 0xf030、0xf000）、0x8680b6（0xf010） | メニュー（d） |
| UnregisterClassW | user32 | 0x425670 / 0x1970c38 | 0x52a84d（f_52a7f0）。ほか静的に複数 | (b)（クラス表） |
| CreateIcon | user32 | 0x4248b0 / 0x1971020 | 0x5c59a1（`f_5c5500`。`f_5d0090` が `[rsi+0x10]` ≠ 0 のときだけ下りる。決定 0069） | (d) GDI/アイコン |
| CreateBitmap、CreateCompatibleBitmap、CreateCompatibleDC、CreateDIBitmap、SelectObject、StretchBlt、GetBitmapBits、DeleteDC | gdi32 | 0x423f30、0x423f50、0x423f60、0x423f90、0x424520、0x424690、0x424180、0x424070 | `f_5c5500` / `f_5c5170` / `f_5c44b0` の中（0x5c4533 ほか） | (d) GDI（DC とビットマップの表。応答済みの GDI は GetDeviceCaps、CreatePalette、DeleteObject、GetStockObject、CreateFontIndirectW、GetObjectW だけ） |
| FormatMessageW | kernel32 | 0x4233b0（0x4233c0） / 0x1971478 | 0x5c4417（`f_5c43d0`。GetLastError の後） | (a)/(d) |
| WTSRegisterSessionNotification | wtsapi32（遅延） | 0x425750 / 0x19762e8 | 0x867fbf（OS バージョンの守りの後） | (d) |
| BufferedPaintInit | uxtheme（遅延） | 0x5dc640 / 0x1976420 | 0x867fe8（同上） | (d) |
| MessageBoxA | user32（遅延） | 0x4071e0 / 0x19762d8 | 0x412886（エラー報告 `f_411ef0` の連鎖。0x867eda → 0x448d40 → … ） | (d)（致命的エラーの報告。到達したら失敗扱い） |
| GetLastError（応答済み） | kernel32 | 0x423540、0x407010 | `f_5c43d0`（0x5c43e4）、`f_44cb40` 側 | 応答済み。unit 114 は実行全体で 2 回しか呼ばない（実測。unit 114 の前） |

注: f_867e10 は DLL のエクスポート `CreateFruityInstance`（0x1375990）の中の 0x137650e からも呼ばれる（読み。エクスポート 3 は `pe-exports`）。InitUnits の外で、**ホストがそのエクスポートを呼んだときの話**（unit 完了とは別。仮説）。

その他、静的に `f_6082c0`（0x620e2c）の閉包に kernel32!CreateThread（thunk 0x406f80 / 0x423260、スロット 0x19713f0。連鎖 `f_6082c0 → f_5c0d40（0x60833d）→ f_5c00e0 → f_5c0d91 → f_525940 → f_522710 → f_5252d0 → f_525400 → f_412b50（0x5254c1）→ 0x406f80（0x412bc6）`）がある。**実行では踏まなかった**（実測: 仮の値の実行で CreateThread に当たらない）。

## 4. f_867710 の残り（依頼された 4 点）

| 項目 | 実測・読み |
|---|---|
| **0x867a78 の f_867e10** | 呼ばれない（実測）。0x867a65: `mov rax,[rip+0xd7824c]`（= 0x15dfcb8、値 0x15e6084）、`cmp byte [rax],0; jne 0x867a7d`。`*0x15e6084` は f_52a7f0 の入口で **1**（gdb）。gdb の watchpoint で、書き込みは `f_412160`（生成 C 132266 行 = 0x41227a: `cmp qword [0x15e6078],0` が等しいとき `mov byte [0x15e6084],1`）。`[0x15e6078]`（MainInstance）は 0（決定 0069）。つまり「MainInstance = 0 のとき 1」になるフラグ（Delphi の `IsLibrary` に見える。仮説。名前は DLL に無い）。同じフラグが 0x867775 の OleInitialize も飛ばしている（実測: OleInitialize の呼び出しは無い）。gdb の breakpoint `f_867e10` は 0 回 |
| **2 つ目の GetClassInfoW / RegisterClassW** | f_867e10 の 0x867e95（TApplication、`[0x1409cf8]`）と 0x867eb3。**実行されない**（上）。GetClassInfoW（unit 114）の呼び出しは `f_52a7f0` の 1 回、RegisterClassW も 1 回（実測: `gdb-class2.txt`、`SEEN` の API 呼び出し回数） |
| **UnregisterClassW** | `f_52a7f0` の 0x52a84d は、GetClassInfoW が 0 のとき飛ばされる（`getclassinfo-caller.md` §3 と同じ読み。実測: 呼ばれない）。ほかの呼び出し点は静的に複数あるが、unit 114 の実行では呼ばれない |
| **GetLastError を読む経路** | 決定 0069 の読み（f_867e10 → `f_869680`（0x868023）→ `f_5cfff0` → `f_5d0090` → `f_5c5500` → `f_5c43d0`（0x5c43e4））。f_867e10 に入らないので**到達しない**。GetLastError の実行は 2 回（unit 114 の前）。CreateIcon の失敗でだけ読む（0069）。RegisterClassW の失敗（0x867eb8 の `test ax,ax`）でも失敗報告へ行くが、成功（非 0）を返せば入らない（`getclassinfo-caller.md` §2.2）。SetLastError はこの経路で呼ばれない |

## 5. unit 114 以降の unit（`fe-pe pe-init-table`）

コピーでの `fe-pe pe-init-table`（`lift/out-u114/init-table.tsv`）: unit_count 942、runnable 895、empty 46、noproc 1。

| index | proc | kind | 備考（静的な閉包、実測） |
|---|---|---|---|
| 114 | 0x623840 | runnable | 本調査の対象 |
| 115 | 0x13a9390 | **empty** | `LD64(proc) = 0` で InitUnits が飛ばす（0060）。空の unit で InitCount が書かれるかは未確認（実測では unit 116 が始まった後の値だけ見た。unit 144 で init_count = 145 = index + 1） |
| **116** | **0x630100** | runnable | 1 関数（`sub [..],1; cmp [..],-1; ret`）。IAT なし |
| 117 | 0x6324b0 | runnable | 同じ形。IAT なし |
| 118 | 0x634e60 | runnable | 同じ形 |
| 119 | 0x6374e0 | runnable | 同じ形 |
| 120 | 0x638910 | runnable | 同じ形 |
| 121〜125 | 0x13a9390 | empty | 飛ばされる |
| 126 | 0x639810 | runnable | 同じ形（1 関数） |
| 127 | 0x13a9390 | empty | |
| 128〜130 | 0x639830、0x639850、0x639870 | runnable | 同じ形 |
| 131、132 | 0x13a9390 | empty | |
| 133 | 0x6398a0 | runnable | 同じ形 |
| 134 | 0x63a5e0 | runnable | 50 関数。未応答の IAT は MessageBoxA(delay) だけ（エラー経路） |
| 135 | 0x63aae0 | runnable | 6 関数。QueryPerformanceFrequency（応答済み）を実行 |
| 136 | 0x63b230 | runnable | 53 関数。GetModuleHandleW(L"shell32")、GetProcAddress("SHGetKnownFolderPath") → **NULL（応答表に無い）** |
| 137 | 0x63bef0 | runnable | 234 関数。GetModuleHandleW(L"kernel32")、GetProcAddress("IsWow64Process")（応答表にある） |
| 138 | 0x643270 | runnable | 235 関数。TlsGetValue |
| 139〜143 | 0x645ce0、0x64f690、0x6516c0、0x6524f0、0x653c50 | runnable | 1 関数。IAT なし |
| **144** | **0x65c540** | runnable | 347 関数。**CompareStringW で停止**（§6） |

結論: **116〜143 の範囲で、unit 114 の次に未実装 import を要求する unit は無かった**（実測。仮の値の実行で、未実装 thunk に当たったのは CreateWindowExW と SetWindowLongPtrW の 2 回だけ。静的な閉包の「未応答」は MessageBoxA(delay)（エラー報告）と GetLogicalProcessorInformation(delay)（遅延 import。ゲストは GetProcAddress 経由で自分で解決し、応答表に行がある。unit 1 で実測）だけ）。

## 6. 次の壁（unit 144。仮の値の実行。参考）

`CompareStringW`（kernel32、既存のランタイム `fle_locale.c`）が `fle_locale_stop`: `character other than an ASCII letter or digit in unequal strings`。引数（実測）: Locale = 0x409、dwCmpFlags = 0、cchCount1 = 8、cchCount2 = 8、lpString1 = L"_x64.dll"（0x6539c8）、lpString2 = L"ppv2.dll"。呼び出しスタック: `f_4231c0`（CompareStringW の包装）← `f_44ec40` ← `f_44f040` ← `f_653660`（0x653797）← `f_6539e0` ← `f_653f40` ← `f_65c540`（unit 144）。実行の a4: init_count 145 / 942、最後の unit index 144、started 132 / completed 131、関数実行数 802、入口数 15,271（`probe/run4.stdout`）。

これは**未実装 API ではなく、既存の応答の方針の停止**（文書に「英数字以外を含む不等な文字列の順序」が無い。`fle_locale.c` の実装方針）。文字列は DLL 名の末尾 `_x64.dll` と `ppv2.dll` の比較に見える（仮説。呼び出し元の意図は追っていない）。この比較が順序を使うか、等しいかだけを見るかは未確認。

## 7. 未確認と限界

- 仮の値の実行は、CreateWindowExW と SetWindowLongPtrW の 2 つに「仮の値」を入れただけで、**実機での戻り値、GetLastError の値、WM_* の送信順序は検証していない**。HWND の値（0x10001）は仮。
- 静的な閉包は、直接 call と IAT thunk だけを辿る。間接 call（`call [rbx-0x20]` などの仮想呼び出し）の先は辿っていない。f_867e10 に 42 か所、`f_869fd0` に 53 か所など。動的な実行で補えたのは、実行された経路だけ。
- unit 114 の「完了」は、`f_623840` から戻って unit 116 が始まったこと（`UNIT-START index=116`）と、a4 の started / completed の増加で確認した。
- 145 以降は調べていない（§6 の停止の先は未測定）。
- 名前（AllocateHWnd、StdWndProc、IsLibrary、TApplication など）は Delphi の知識からの**仮説**で、DLL にシンボルは無い。
- 他の作業者が編集中の `lift/` の作業ツリーと決定 0071、`window-design.md` は読んでいない。ここでの「仮想ウィンドウ表が要る」の分類は、API の意味からの分類で、その設計への賛否ではない。

## 8. 成果物（`lift/out-u114/`、gitignore 済み）

`stage/`（HEAD のコピー）、`gen/out.c`（生成 C）、`build/`（ネイティブの harness と a4 のログ `build/a4/20261022T124356Z-*`）、`init-table.tsv`、`thunks.json`（`.text` の IAT thunk 889 個、名前つき）、`answered.json`（応答済み 224 API）、`tail-apis.json`、`units-116-145-static.json`、`work/`（`pelib.py`、`closure.py`、`dump.py`、`tail.py`、`probe2〜5.py`）、`probe/`（gdb スクリプトとログ: `run4.log`、`seq*.log`、`seq2.log`、`guards.gdbout`、`watch3.gdbout`、`stub.gdbout`、`run6.gdbout`）。
