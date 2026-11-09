# 仮想ウィンドウの設計調査（CreateWindowExW、WM_NCCREATE / WM_CREATE、SetWindowLongPtrW、DefWindowProcW）。実装は決定 0071

実行日: 2026-10-22。HEAD 92b9e50 に未コミットの変更を載せた状態。「文書」は Microsoft Learn（2026-10-22 取得）、「実測」は DLL のバイト列と `objdump` の走査（`lift/objdump-text.txt`、成果物は `lift/out-win/`、gitignore 済み）、「読み」は静的な読み、「参考」は Wine の実装（`lift/out-win/wine-window.c`、`wine-defwnd.c` = wine-mirror/wine master の `dlls/win32u/window.c`、`defwnd.c`。**一次資料ではない**。実機の挙動の代わりにならない）、「推測」は未検証。

## 0. 結論

- 停止点は **`f_52a7f0` の 0x52a8be（`call 0x425780` = user32!CreateWindowExW の thunk）**。前の調査の読み（`CreateWindowExW(0x80, L"TPUtilWindow", L"", WS_POPUP, 0,0,0,0, NULL, NULL, HInstance, NULL)`、直後に `SetWindowLongPtrW(hwnd, GWL_WNDPROC, f_52a5b0(rbx))`）を、バイト列で確認した（§2.2）。
- **CreateWindowExW はウィンドウプロシージャを今呼ぶ必要がある**（Learn: CreateWindowEx は WM_NCCREATE、WM_NCCALCSIZE、WM_CREATE を、戻る前に、作られるウィンドウに送る。§1）。ただしこの停止点のクラス `TPUtilWindow` の lpfnWndProc は **0x424920 = DefWindowProcW の thunk**（実測。決定 0070 の調査）。つまり送った 3 つのメッセージは**ゲストの独自コードではなく DefWindowProcW に届く**。DefWindowProcW を答えれば、ゲストの wndproc は呼ばれない（§3）。
- 推奨は **B'**: 窓表 + CreateWindowEx の規則に従う送信（Learn が列挙する **WM_NCCREATE → WM_NCCALCSIZE → WM_CREATE** の順。PM の想定は NCCREATE と CREATE の 2 つだったが、Learn の CreateWindowEx のページが WM_NCCALCSIZE も挙げるので 3 つにする。§1、§5）。`fle_call_guest`（決定 0064）で送る。
- 段 2 の条件 (i)〜(iv) を満たす（§6）。実装は決定 0071。
- **重要な見つけ物（次の段階の判断材料）**: この停止点の直後、ゲストは `SetWindowLongPtrW(hwnd, GWL_WNDPROC, instance)` で、**実行時に生成したマシンコードのトランポリン**（Delphi の `MakeObjectInstance`。`f_52a5b0` が VirtualAlloc した PAGE_EXECUTE_READWRITE のブロックに `E8 rel32`、`41 5B FF 25 00 00 00 00 <0x52a570>` を書く。§3.3）をウィンドウプロシージャとして登録する。このウィンドウにメッセージを**送る**段階（SendMessageW、DispatchMessageW、WM_TIMER など）になると、`fle_call_guest` は「その VA は関数の入口でない」で止まる（`fle_dispatch_miss`）。この停止点では送らないので当たらない。

## 1. 一次資料（Learn）

- **CreateWindowExW**（https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-createwindowexw）:
  - `HWND CreateWindowExW(DWORD dwExStyle, LPCWSTR lpClassName, LPCWSTR lpWindowName, DWORD dwStyle, int X, int Y, int nWidth, int nHeight, HWND hWndParent, HMENU hMenu, HINSTANCE hInstance, LPVOID lpParam)`。
  - lpClassName: 「A null-terminated string or a class atom. If a null-terminated string, it specifies the window class name. The class name can be any name registered with the RegisterClass or RegisterClassEx function, provided that the module that registers the class is also the module that creates the window. The class name can also be any of the predefined system class names.」atom は MAKEINTATOM（上位ワードが 0）。
  - lpParam: 「Pointer to a value to be passed to the window through the CREATESTRUCT structure (lpCreateParams member) pointed to by the lParam param of the **WM_CREATE** message. This message is sent to the created window by this function **before it returns**.」
  - hWndParent: 「optional for pop-up windows」。「To create a message-only window, supply HWND_MESSAGE」。hMenu: 「For an overlapped or pop-up window, hMenu identifies the menu to be used with the window; it can be NULL if the class menu is to be used」。X: 「CW_USEDEFAULT is valid only for overlapped windows; if it is specified for a pop-up or child window, the x and y parameters are set to zero」（nWidth、nHeight も同じ）。
  - 戻り値: 「If the function succeeds, the return value is a handle to the new window. If the function fails, the return value is NULL. To get extended error information, call GetLastError.」「This function typically fails for one of the following reasons: an invalid parameter value; the system class was registered by a different module; The WH_CBT hook is installed and returns a failure code; if one of the controls in the dialog template is not registered, or its window window procedure fails WM_CREATE or WM_NCCREATE」。**GetLastError の値は書かれていない**。
  - Remarks: 「**The CreateWindowEx function sends WM_NCCREATE, WM_NCCALCSIZE, and WM_CREATE messages to the window being created.**」（この順序で列挙。ページはこれが送信の順序だと明言はしない。**WM_NCCALCSIZE の wParam と lParam の値は CreateWindowEx のページに書かれない**）。「If the created window is a top-level window, its default position is at the top of the Z-order」。
- **WM_NCCREATE**（…/winmsg/wm-nccreate）: 「Sent prior to the WM_CREATE message when a window is first created.」`#define WM_NCCREATE 0x0081`。wParam: 「not used」。lParam: 「A pointer to the CREATESTRUCT structure that contains information about the window being created. The members of CREATESTRUCT are identical to the parameters of the CreateWindowEx function.」戻り値（LRESULT）: 「If an application processes this message, it should return **TRUE** to continue creation of the window. If the application returns **FALSE**, the CreateWindow or CreateWindowEx function will return a **NULL** handle.」
- **WM_CREATE**（…/winmsg/wm-create）: 「Sent when an application requests that a window be created by calling the CreateWindowEx or CreateWindow function. (The message is sent before the function returns.) The window procedure of the new window receives this message after the window is created, but before the window becomes visible.」`#define WM_CREATE 0x0001`。wParam: 「not used」。lParam: CREATESTRUCT へのポインタ。戻り値: 「If an application processes this message, it should return **zero** to continue creation of the window. If the application returns **–1**, the window is destroyed and the CreateWindowEx or CreateWindow function returns a **NULL** handle.」
- **WM_NCCALCSIZE**（…/winmsg/wm-nccalcsize）: `#define WM_NCCALCSIZE 0x0083`。「If wParam is FALSE, lParam points to a RECT structure. On entry, the structure contains the proposed window rectangle for the window. On exit, the structure should contain the screen coordinates of the corresponding window client area.」「If the wParam parameter is FALSE, the application should return zero.」DefWindowProc の処理は「the default, backward-compatible processing of this message by the DefWindowProc function (in addition to the usual client rectangle calculation …)」とだけ書く。
- **Managing Application State**（…/learnwin32/managing-application-state-）: 「When this function is called, it sends the following two messages to your window procedure: WM_NCCREATE, WM_CREATE. These messages are sent in the order listed. (These are not the only two messages sent during CreateWindowEx, but we can ignore the others for this discussion.)」 → **順序の根拠（NCCREATE が先、CREATE が後）。他のメッセージがあることも認めている**。
- **CreateWindow**（…/nf-winuser-createwindoww）: 「For overlapped, pop-up, and child windows, CreateWindow sends WM_CREATE, WM_GETMINMAXINFO, and WM_NCCREATE messages to the window.」（列挙の順が WM_CREATE、WM_GETMINMAXINFO、WM_NCCREATE で、NCCREATE が先という上のページと食い違う。**列挙は順序でない**と読める。）一方 **Window Features**（…/winmsg/window-features）: 「A window must have the WS_THICKFRAME or WS_CAPTION style to receive WM_GETMINMAXINFO. A window with the WS_THICKFRAME style receives this message during the window-creation process」。→ **WS_POPUP だけのウィンドウが WM_GETMINMAXINFO を受けるかは Learn の中で食い違う**（CreateWindow のページは pop-up も受けると書き、Window Features は CAPTION か THICKFRAME が要ると書く）。「The system sends the WM_NCCALCSIZE message to a window when the window is created or sized.」（Window Features）。
- **CREATESTRUCTW**（…/ns-winuser-createstructw）: `LPVOID lpCreateParams; HINSTANCE hInstance; HMENU hMenu; HWND hwndParent; int cy; int cx; int y; int x; LONG style; LPCWSTR lpszName; LPCWSTR lpszClass; DWORD dwExStyle;`。**sizeof は書かれない**。x64 の整列から導いた値: lpCreateParams @0、hInstance @8、hMenu @16、hwndParent @24、cy @32、cx @36、y @40、x @44、style @48（@52 に 4 バイトの穴）、lpszName @56、lpszClass @64、dwExStyle @72（@76 に 4 バイトの末尾パディング）、**sizeof = 80**（導出）。Remarks: 「Because the lpszClass member can contain a pointer to a local (and thus inaccessible) atom, do not obtain the class name by using this member. Use the GetClassName function instead.」lpszName: 「The name of the new window」、lpszClass: 「A pointer to a null-terminated string or an atom」。
- **SetWindowLongPtrW**（…/nf-winuser-setwindowlongptrw）: `LONG_PTR SetWindowLongPtrW(HWND hWnd, int nIndex, LONG_PTR dwNewLong)`。nIndex: GWL_EXSTYLE -20、GWLP_HINSTANCE -6、GWLP_HWNDPARENT -8、GWLP_ID -12、GWL_STYLE -16、GWLP_USERDATA -21（「Its value is initially zero」）、**GWLP_WNDPROC -4**（「Sets a new address for the window procedure」）。戻り値: 「If the function succeeds, the return value is the previous value of the specified offset. If the function fails, the return value is zero. To get extended error information, call GetLastError. If the previous value is zero and the function succeeds, the return value is zero, but the function does not clear the last error information.」About Window Procedures: 「When passed the GWL_WNDPROC flag, SetWindowLongPtr returns the address of the window's original window procedure.」「the window procedure must conform to the guidelines specified in … WindowProc」。失敗時の GetLastError の値は書かれていない。
- **DefWindowProcW**（…/nf-winuser-defwindowprocw）: 「Calls the default window procedure to provide default processing for any window messages that an application does not process. … DefWindowProc is called with the same parameters received by the window procedure.」戻り値: 「The return value is the result of the message processing and depends on the message.」→ **DefWindowProcW が WM_NCCREATE と WM_CREATE に何を返すかは、ページに書かれない**。WM_NCCREATE のページは「アプリケーションが処理するなら TRUE を返して作成を続ける」と書くだけで、既定処理の戻り値は書かない。**DefWindowProcW(WM_NCCREATE) = TRUE、DefWindowProcW(WM_CREATE) = 0 は、Learn に書かれていない。「既定処理がウィンドウの作成を止めない」という要件からの推論と、参考（Wine の `defwnd.c`: WM_NCCREATE は lParam が非 NULL なら result = 1、WM_CREATE は結果 0）に基づく**（仮）。
- **DestroyWindow**（…/nf-winuser-destroywindow）: 「The function sends WM_DESTROY and WM_NCDESTROY messages to the window」。CreateWindowEx の失敗（NCCREATE が FALSE、CREATE が -1）が DestroyWindow 相当の WM_DESTROY / WM_NCDESTROY を送るかは、CreateWindowEx、WM_NCCREATE、WM_CREATE のどのページにも書かれない（WM_CREATE のページは "the window is destroyed" とだけ書く）。
- **HWND の値**（Windows Data Types: 「HWND: A handle to a window」`typedef HANDLE HWND`。Interprocess communication: 「64-bit versions of Windows use 32-bit handles for interoperability. When sharing a handle between 32-bit and 64-bit applications, only the lower 32 bits are significant, so it is safe to truncate the handle … or sign-extend the handle … Handles that can be shared include handles to user objects such as windows (HWND)」）。**これが HWND について Learn が書く唯一の値の性質**（「下位 32 ビットだけが有効」。決定 0054 と同じ）。具体的な値、増え方、再利用は書かれていない。→ 領域は 0x80000000 未満に取る（切り詰めと符号拡張が同じ値になる）。
- Window Styles（…/winmsg/window-styles）: WS_POPUP 0x80000000、WS_VISIBLE 0x10000000、WS_BORDER 0x00800000、WS_CAPTION 0x00C00000、WS_DLGFRAME 0x00400000、WS_THICKFRAME 0x00040000、WS_SYSMENU 0x00080000、WS_CHILD 0x40000000。Extended Window Styles: WS_EX_TOOLWINDOW 0x00000080（「intended to be used as a floating toolbar … A tool window does not appear in the taskbar or in the dialog that appears when the user presses ALT+TAB」）。

**書かれていないこと**: CreateWindowEx の失敗時の GetLastError の値。HWND の具体的な値。WM_NCCALCSIZE を CreateWindowEx が送るときの wParam と lParam の RECT の中身。WS_POPUP だけのウィンドウが WM_GETMINMAXINFO を受けるか（Learn 内で食い違う）。WM_SIZE、WM_MOVE、WM_WINDOWPOSCHANGING、WM_WINDOWPOSCHANGED を CreateWindowEx が送るか（CreateWindowEx のページは挙げない。Window Features は WM_WINDOWPOSCHANGING / CHANGED を「サイズ、位置、Z オーダー、表示状態が変わるとき」に送ると書くが、作成時については書かない）。DefWindowProcW の NCCREATE、CREATE、NCCALCSIZE への戻り値。NCCREATE の戻りで TRUE 以外の非 0 値（例: 2）の扱い。WM_CREATE の戻りで 0 と -1 以外の値の扱い（「should return zero」と書くだけ）。失敗時に WM_DESTROY / WM_NCDESTROY が送られるか。SetWindowLongPtrW(GWLP_WNDPROC) の戻り値が、システムクラスのウィンドウプロシージャのときに実機で何の値になるか。

## 2. ゲストの使用の全体像（実測。`call rel32` の走査と IAT の対応）

### 2.1 呼び出し点の数（`.text` 全体の `E8 rel32` のバイト走査。thunk は IAT スロットから特定。成果物 `lift/out-win/calls-bytes.json`）

**訂正（作業中の自己訂正）**: 最初に数えた `lift/objdump-text.txt` は **0x11fb4ae で終わっている**（.text は 0x13a44b0 まで。末尾は別ファイル `out-hi/objdump-tail.txt` にあるが、ここでは使わなかった）。その走査は GetWindowLongPtrW 65、SendMessageW 213、PostMessageW 128 と数えたが、**バイト走査（tests/class.rs と同じ方法）では 66、216、139**。下の表はバイト走査の値（テストが固定）。

| API | thunk | 呼び出し点 |
|---|---|---|
| CreateWindowExW | 0x425780 | **9** |
| SetWindowLongPtrW | 0x425980 | 43 |
| GetWindowLongPtrW | 0x425970 | **66**（末尾の 0x121d0c8 を含む） |
| DefWindowProcW | 0x424920 | 22 |
| DestroyWindow | 0x424990 | 8 |
| SendMessageW | 0x425380、0x425390、0x4253b0（同じ IAT スロット 0x1970c48） | **216** |
| SendMessageA | 0x4253a0 | 1 |
| PostMessageW | 0x425210 | **139** |
| UnregisterClassW | 0x425670 | 10 |
| DispatchMessageW | 0x4249a0、0x4249c0 | 14 |
| GetMessageW | 0x424e10 | 1 |
| PeekMessageW | 0x425200 | 21 |
| ShowWindow | 0x4255b0 | 28 |
| CallWindowProcW | 0x4247a0 | 12 |
| SetTimer / KillTimer | 0x4254f0 / 0x4250c0 | 13 / 4 |

SetWindowLongPtrW の nIndex（直前の `mov edx,imm32` / `xor edx,edx` から読んだ**静的な近似**。バイト走査）: -4（WNDPROC）13、-12（ID）3、-16（STYLE）7、-20（EXSTYLE）18、-21（USERDATA）1、0（extra）1。GetWindowLongPtrW: -4 が 6、-16 が 25、-12 が 3、-8 が 3、-20 が 26、-21 が 2、-6 が 1。

### 2.2 CreateWindowExW の 9 か所（実測 + 読み）

| 呼び出し点 | 関数 | dwExStyle | クラス | dwStyle | x, y, w, h | 親 | hMenu | lpParam |
|---|---|---|---|---|---|---|---|---|
| **0x52a8be** | f_52a7f0（10 か所から呼ばれる: 0x6056ec、0x66f60e、0x71e92d、0x82c5ad、0x8318e0、0x84565a、0x858719、0x976f6d、0x983397、0xba25a9） | **0x80** | **TPUtilWindow**（`[0x13be850]`） | **0x80000000** | **0, 0, 0, 0** | **NULL** | **NULL** | **NULL** |
| 0x425805 | 0x425790（CreateWindowW 形の薄いラッパー。ex = 0。呼び出し元は下の 3 か所） | 0 | 引数 | 引数 | 引数 | 引数 | 引数 | 引数 |
| （0x6d8468 経由） | 0x6d83a0 | 0 | TTimerWindow | 0 | 0,0,0,0 | **-3（HWND_MESSAGE）** | NULL | NULL |
| （0x81d002 経由） | f_81cf60 | 0 | ICMCALLBACKWND（`lea 0x81d014`） | 0x80880000（WS_POPUP\|WS_BORDER\|WS_SYSMENU） | 0,0,0,0 | NULL | NULL | rbx |
| （0xd70738 経由） | 0xd70670 | 0 | TILRemoteWindow | 0 | 0,0,0,0 | **-3** | NULL | NULL |
| 0x611d09 | TWinControl.CreateWnd に見える（`TCreateParams` を `[rsp+0x78]` から読む。推測） | `[p+0xc]` | 動的（`p+0x78`） | 動的 | 動的 | 動的 | NULL（`[rsp+0x48] = 0` 後に hMenu は 0） | `[p+0x28]` |
| 0x6327fb | 同上（`style & 0xfffff7ff`） | 動的 | 動的 | 動的 | 動的 | 動的 | NULL | 動的 |
| 0x6328ce | 同上 | 動的 | 動的 | 動的 | 動的 | 動的 | NULL | 動的 |
| 0x71e133 | 0x71e0xx | **0x080800ac** | TLightweightLayeredControl | 0x80000000 | 動的 | 動的 | NULL | rbx |
| 0x85fb6c | 0x85fa..（MDI） | 0x200 | **MDICLIENT**（`lea 0x85fd00` = L"MDICLIENT"） | 0x56330001 | 動的 | 動的 | NULL | `&CLIENTCREATESTRUCT` |
| 0x867f7f | f_867e10 | **0x80** | **TApplication** | 0x84ca0000（WS_POPUP\|WS_CAPTION\|WS_SYSMENU\|…） | 動的（中央）, 0, 0 | NULL | NULL | NULL |
| 0xa1bb35 | f_a1ba00 | **0x80** | VTPanningWindow | **0x80000000** | 動的, 動的, 0x20, 0x20 | NULL | 動的 | NULL |

（9 か所 = 0x425805、0x52a8be、0x611d09、0x6327fb、0x6328ce、0x71e133、0x85fb6c、0x867f7f、0xa1bb35。ラッパー経由の 3 つは 0x425805 の先。）

**停止点までに届く形は 0x52a8be の 1 つ**（`f_52a7f0` は a4 の実行で `RegisterClassW` の次の命令）。f_52a7f0 のスタック引数（実測。バイト列 `c7442420 00000000` ほか）: `[rsp+0x20]`、`[rsp+0x28]`、`[rsp+0x30]`、`[rsp+0x38]` は **DWORD の 4 バイト書き込み**（x、y、cx、cy = 0。**スロットの上位 4 バイトは不定**）、`[rsp+0x40]`（親）= 0、`[rsp+0x48]`（hMenu）= 0、`[rsp+0x50]` = `[0x15ec298]`（HInstance = 0x400000）、`[rsp+0x58]`（lpParam）= 0。lpWindowName は 0x52a8f4 の L""（空文字列。バイト 00 00）。

0x52a8c3 以降（実測）: `mov rsi,rax` → `cmp [rbx],0`（`rbx` は AllocateHWnd に渡された TMethod の写し。`f_858700` が 16 バイトを `[rbp+0x20]` に写して渡す）→ 非 0 なら `rcx = f_52a5b0(rbx)`（オブジェクトインスタンスを作る。§3.3）→ `SetWindowLongPtrW(rsi, 0xfffffffc, rax)`（戻り値は使わない）→ `rax = rsi` で HWND を返す。GetLastError、SetLastError は f_52a7f0 全体で呼ばない（決定 0070 の調査）。CreateWindowExW の戻り（HWND）が 0 のときの処理はない（`rsi` を検査せずに SetWindowLongPtrW へ渡す）。→ **実装が NULL を返すと、ゲストは NULL の HWND に SetWindowLongPtrW を呼ぶ**。今回の形は NULL を返さない（成功する）ので当たらない。

## 3. ゲストのウィンドウプロシージャの読み

### 3.1 クラスの lpfnWndProc（実測。WNDCLASSW の 72 バイト。決定 0070 の調査）

| クラス | lpfnWndProc | 中身（読み） |
|---|---|---|
| TPUtilWindow | **0x424920 = DefWindowProcW の thunk** | 独自処理なし |
| TApplication | 0x424920 | 同上 |
| VTPanningWindow | 0x424920 | 同上 |
| TTimerWindow | 0x6d8330 | `rdx == 0x8ffc`（WM_USER + 0x8fc）なら自分で処理（`[rsi+0x78]` の関数を呼ぶ）、それ以外は `DefWindowProcW(rcx, rdx, r8, r9)` へ |
| TLightweightLayeredControl | 0x71dd10 | **`WM_NCCREATE`（0x81）だけ処理**: `rax = [lParam]`（CREATESTRUCT.lpCreateParams = オブジェクト）、`[rax+0xa8] = hwnd`、`SetWindowLongPtrW(hwnd, -4, [rax+0x78])`、**return 1**。それ以外は DefWindowProcW |
| TILRemoteWindow | 0xd70600 | `rdx == 0x8ffd` なら独自、それ以外は DefWindowProcW |
| ICMCALLBACKWND（RegisterClassW の WNDCLASSW は f_81cf60 がスタックに作る） | 0x81ce10 | `WM_CREATE`（1）、0x2b..、0xec 系を switch（WM_CREATE で `SetWindowLongPtrW(hwnd, GWLP_USERDATA = -21, …)` して DefWindowProcW）。その他は DefWindowProcW |
| 動的（f_6118a0 の 15 の呼び出し元、TWinControl） | 0x604d10（InitWndProc に見える） | `[obj+0x45c] = hwnd`、`IsWindowUnicode` ほか、`SetWindowLongPtrW(hwnd, -4, [obj+0x464])`（Delphi の `InitWndProc` の形。推測） |

**今回の停止点（TPUtilWindow）の NCCREATE、NCCALCSIZE、CREATE はすべて DefWindowProcW（0x424920 の thunk）に届く**。ゲストの独自の wndproc は呼ばれない。ただし**他の 4 つのクラスの wndproc は NCCREATE / CREATE で独自の処理をする**（0x71dd10 は NCCREATE で戻り値 1 を返し、SetWindowLongPtrW(GWLP_WNDPROC) を **CreateWindowEx の最中に**呼ぶ。0x81ce10 は CREATE で SetWindowLongPtrW を呼ぶ）。したがって窓表は「wndproc の呼び出し中に HWND が引ける」「WNDPROC の差し替えが同じ CreateWindowEx の次のメッセージから効く」を満たす必要がある（決定 0071 の設計に反映）。

### 3.2 Delphi の標準の `DefWindowProc` へのフォワードか、独自処理か

TPUtilWindow、TApplication、VTPanningWindow: 登録した時点の wndproc が `DefWindowProcW` そのもの（独自処理なし）。TTimerWindow、TILRemoteWindow: 独自のメッセージ（0x8ffc、0x8ffd）だけ自分で処理し、NCCREATE、CREATE を含む他は DefWindowProcW へ転送する。TLightweightLayeredControl: NCCREATE は独自、他は転送。→ **今届く NCCREATE、NCCALCSIZE、CREATE の既定処理は DefWindowProcW のものだけで足りる**（停止点の形）。

### 3.3 `f_52a5b0`（MakeObjectInstance に見える。読み）

- 0x52a5b0: `rcx` = TMethod（Code、Data の 16 バイト）へのポインタ。空きリスト `[0x15f2708]` が空なら `VirtualAlloc(NULL, 0x1000, 0x1000 /*MEM_COMMIT*/, 0x40 /*PAGE_EXECUTE_READWRITE*/)` で 4 KiB のブロックを確保し、`[blk+0] = [0x15f2700]`（ブロックの連結）、`[blk+8..16)` に `[0x13be804]` の 8 バイト（**41 5B FF 25 00 00 00 00 = `pop r11; jmp qword ptr [rip+0]`**。実測）を写し、`[blk+0x10] = 0x52a570`（ゲストの共通スタブ）、`[blk+0x18 …)` を 0x15 バイトのスロット（`E8 rel32`、次の空き、TMethod の 16 バイトの置き場）に分けて空きリストへ。1 つ取り出して `TMethod` を `[slot+5]` に写し、**スロットのアドレスを返す**。
- 共通スタブ 0x52a570（実測）: `[rbp+0x20] = edx`（Msg）、`[+0x28] = r8`（WParam）、`[+0x30] = r9`（LParam）、`[+0x38] = 0`（Result）、`rdx = &TMessage`、`rcx = [r11+8]`（TMethod.Data）、`call [r11]`（TMethod.Code）、`rax = [rbp+0x38]` を返す。`r11` は `pop r11` で入れた**スロット + 5**（`E8` の戻り番地）。
- つまりゲストが `SetWindowLongPtrW(hwnd, -4, slot)` で登録する wndproc は、**ゲストのコードではなく、VirtualAlloc したメモリのマシンコード**（`E8 rel32` → ブロックの `pop r11; jmp [rip+0]` → 0x52a570）。`fle_call_guest(slot, …)` は `fle_dispatch(slot)` で「関数の入口でない」と止まる。**この環境でそのウィンドウにメッセージを送る段階では、スロットを解釈するか（`E8 rel32` と `pop r11; jmp` の形を認識して、`r11 = slot + 5` で 0x52a570 を呼ぶ）、生成コードを実行できる仕組みが要る**。今回の実装（CreateWindowEx が TPUtilWindow のクラス wndproc だけを呼ぶ）には関係しない。

## 4. ゲストの wndproc を今呼ぶ必要があるか

**ある**（Learn: CreateWindowEx は NCCREATE、NCCALCSIZE、CREATE を戻る前に送る）。ただし呼ぶのは**クラスの wndproc**で、今回は **DefWindowProcW の thunk（0x424920）**。呼ぶ機構は `fle_call_guest(0x424920, hwnd, msg, wParam, lParam)`（決定 0064）。0x424920 は `fle_dispatch_table` の関数入口なので `fle_dispatch` で引ける。thunk は `fle_imp_DefWindowProcW(); return;` になる（決定 0071 が表に行を足す）。thunk の `ret` は C の `return;` で `R[4]` を動かさないので、`fle_call_guest` の検査（`R[4] == call_sp - 8`、戻り番地のスロット不変、callee-saved 不変）を通る。再入の上限 16 段（FLE_CB_MAX_DEPTH）は足りる（今回の入れ子は CreateWindowEx → DefWindowProcW の 1 段。クラスの wndproc が CreateWindowEx を呼ぶ形は、停止点の形に無い）。

## 5. 設計案

- **A. 窓表 + 完全な WM_* 送信**（Wine の `create_window` 相当: WM_GETMINMAXINFO、WM_NCCREATE、WM_NCCALCSIZE、WM_CREATE、WM_SIZE、WM_MOVE、WM_PARENTNOTIFY、WS_VISIBLE なら WM_SHOWWINDOW ほか）。**不採用**: Learn が CreateWindowEx について書くのは NCCREATE、NCCALCSIZE、CREATE の 3 つだけ。他は参考（Wine）にしか根拠が無く、WM_GETMINMAXINFO は Learn の中でも食い違う。ゲストの wndproc（DefWindowProcW）が各メッセージで何を返すかも決めることになり、DefWindowProcW の範囲が大きくなる（条件 iii に反する）。
- **B'. 窓表 + CreateWindowEx の規則（Learn が列挙する NCCREATE → NCCALCSIZE → CREATE）だけ送る**（採用）。**PM の推奨 B は「NCCREATE と CREATE の 2 つ」。Learn の CreateWindowEx のページが WM_NCCALCSIZE も明記するので、「Learn に書かれていれば送る」の規則に従って 3 つにした**。順序は Learn の Remarks の列挙順（NCCREATE、NCCALCSIZE、CREATE）で、参考（Wine の `create_window`: NCCREATE → NCCALCSIZE → CREATE）と一致する。Managing Application State が NCCREATE、CREATE の順を明記。**送らないメッセージ**: WM_GETMINMAXINFO（WS_POPUP だけの形で、Window Features の「CAPTION か THICKFRAME が要る」に従う。CreateWindow のページとは食い違う。決定 0071 に未確認として書く）、WM_SIZE、WM_MOVE、WM_WINDOWPOSCHANGING/CHANGED、WM_PARENTNOTIFY、WM_SHOWWINDOW（WS_VISIBLE なし）、WM_ACTIVATE 系（不可視）。
- **C. 停止のまま**。**不採用**: PM の決定は実装する方針。条件 (i)〜(iv) を満たす（§6）。

## 6. 段 2 の条件（i〜iv）の判定

| 条件 | 判定 | 根拠 |
|---|---|---|
| (i) 「不可視のユーティリティウィンドウ（WS_POPUP、サイズ 0、親なし、WS_VISIBLE なし）」の形で、停止点までに届く形だけ | **満たす** | 0x52a8be の形は dwExStyle 0x80、WS_POPUP、x = y = w = h = 0、親 NULL、hMenu NULL、lpParam NULL。9 か所のうち、この形に合うのは 0x52a8be だけ（他は HWND_MESSAGE、ex = 0x80 でも style 0x84ca0000、動的、MDICLIENT など）。他の形は名前付き停止 |
| (ii) NCCREATE / CREATE の送信が `fle_call_guest` で行える（callee-saved を守る、再入 16 段で足りる） | **満たす** | §4。クラスの wndproc は DefWindowProcW の thunk（生成 C の関数）。callee-saved は生成 C の関数が守る（決定 0064）。1 段 |
| (iii) 追加する API は CreateWindowExW、SetWindowLongPtrW（GWLP_WNDPROC のみ）、DefWindowProcW（今届くメッセージ種別だけ）の 3 つ | **満たす** | 3 つだけ。DefWindowProcW は NCCREATE、NCCALCSIZE（wParam FALSE）、CREATE のみ。GetWindowLongPtrW、DestroyWindow、SendMessageW、PostMessageW、UnregisterClassW、GetMessageW、DispatchMessageW、ShowWindow は実装しない（次の停止点の判断材料は §8） |
| (iv) 仮想ウィンドウ表が小さく決定論的 | **満たす** | 表は {HWND、クラスのアトム、hInstance、style、exstyle、wndproc、クラスの wndproc、live}。ホストの実ウィンドウ、実ディスプレイ、実メッセージキュー、時計を読まない |

## 7. 確定できなかったこと（限界）

- 実機の CreateWindowExW の失敗時の GetLastError の値（Learn に無い）。**last error には触れない**（仮）。
- 実機の HWND の値（Learn は「下位 32 ビットだけが有効」とだけ）。領域は決定 0071 の仮の選択。
- WM_NCCALCSIZE を CreateWindowEx が送るときの wParam と lParam の RECT（Learn の CreateWindowEx のページには無い。WM_NCCALCSIZE のページは wParam FALSE のとき lParam が RECT で、「提案されたウィンドウの矩形」が入ると書く）。**wParam = FALSE、RECT = (x, y, x + cx, y + cy)**（参考: Wine の `create_window`）。
- DefWindowProcW の NCCREATE、CREATE、NCCALCSIZE への戻り値（Learn に無い。§1）。
- WS_POPUP だけのウィンドウが WM_GETMINMAXINFO を受けるか（Learn の中で食い違う）。送らない（仮）。
- 失敗時（NCCREATE が FALSE、CREATE が -1）に WM_DESTROY / WM_NCDESTROY を送るか（Learn に無い）。**送らない**（仮。今回の形では wndproc が DefWindowProcW なので失敗しない。ゲストの独自 wndproc が失敗を返す形は、この呼び出しの外）。
- NCCREATE の戻りで TRUE（1）、FALSE（0）以外の値、CREATE の戻りで 0、-1 以外の値、NCCALCSIZE（wParam FALSE）の戻りで 0 以外の値: Learn は「should return」とだけ書き、他の値の扱いを書かない → **停止**（値は 64 ビット全部を比べる。-1 は 0xFFFFFFFFFFFFFFFF）。
- SetWindowLongPtrW(GWLP_WNDPROC) の戻り値が、システムのウィンドウプロシージャのクラスで実機に何の値になるか。About Window Procedures は「the address of the window's original window procedure」と書く。ここでは**そのウィンドウが持っていた wndproc の値**（クラスの lpfnWndProc のまま）を返す。
- ゲストが MakeObjectInstance のスロットを wndproc にして、その後にメッセージが送られる形（§3.3）。
- 動的な CreateWindowExW 3 か所（0x611d09、0x6327fb、0x6328ce）の実際のクラス名と引数（実行に届いていない）。

## 8. 実装後の次の停止点の見通し（実装しない。判断材料のみ）

`SetWindowLongPtrW`（GWLP_WNDPROC）まで実装すると、`f_52a7f0` は HWND を返す。続く経路は、AllocateHWnd の呼び出し元（f_858700 ← f_867710 ほか 9 か所の `call 0x52a7f0`）に戻る。次に出る API は、HWND を使う `GetWindowLongPtrW`（66 か所、nIndex は -4、-16、-20、-8、-12、-21、-6）、`SetTimer`（TTimer の AllocateHWnd + SetTimer の形）、`PostMessageW`（139）、`SendMessageW`（216）、`DestroyWindow`（8）、`UnregisterClassW`（10）、`ShowWindow`（28）、`GetMessageW`/`PeekMessageW`/`DispatchMessageW`（メッセージループ）。どれも**この調査の時点では実装しない**（PM の条件 iii）。それぞれに必要な状態:

- GetWindowLongPtrW: 窓表の style、exstyle、wndproc、USERDATA、ID、親、hInstance の読み取り。USERDATA は SetWindowLongPtrW(-21) の書き込みが対（実装は小さい）。
- SendMessageW: 窓の wndproc を `fle_call_guest` で呼ぶ（**スロットのトランポリン（§3.3）の解釈が要る**。SendMessage の相手が AllocateHWnd のウィンドウなら、`E8 rel32` と `pop r11; jmp [rip+0]` を認識して `r11 = slot + 5` で 0x52a570 を呼ぶ形）。戻り値は LRESULT。
- PostMessageW: メッセージキュー（決定論的な FIFO を持つか。ホストの実キューは読まない）。GetMessageW/PeekMessageW/DispatchMessageW とセット。
- DestroyWindow: WM_DESTROY、WM_NCDESTROY の送信と窓表の破棄（今回の「破棄」は WM_DESTROY/NCDESTROY を送らない仮の簡略）。
- UnregisterClassW: クラス表の削除（「クラスにウィンドウがある」ときの失敗と GetLastError は Learn のページを要確認）。

見積もり: GetWindowLongPtrW + USERDATA は小（窓表の読み書き。1 日未満）、DestroyWindow は中（WM_DESTROY/NCDESTROY の規則と子ウィンドウ、所有ウィンドウ）、メッセージキュー + ループは大（タイマー、WM_QUIT、スレッド、再入）、トランポリンの解釈は中（形の認識と 0x52a570 の呼び出し。R[11] の扱い）。

## 9. 実装後の実測（決定 0071）

- 実行: `OUT=.../lift/out-win WASM=1 bash lift/harness/a4-survey.sh`。生成 C は `lift/out-win/out` に再生成（thunk 3 つが `fle_imp_*` を呼ぶ）。native と wasm（node）は rsp= の行を除いて完全に一致（stdout、stderr とも）。
- gdb（`lift/out-win/gdb-win.txt`）で、停止点を越えた実際の API の順序: `CreateWindowExW(rcx=0x80, rdx=0x52a7c8)` → `DefWindowProcW(0x7f990004, 0x81, 0, cs)` → `DefWindowProcW(0x7f990004, 0x83, 0, rect)` → `DefWindowProcW(0x7f990004, 0x01, 0, cs)` → `SetWindowLongPtrW(0x7f990004, -4, <MakeObjectInstance のスロット。ヒープのアドレス>)`。つまり**設計どおり、クラスの wndproc（DefWindowProcW の thunk）が NCCREATE、NCCALCSIZE、CREATE をこの順に受け、その後ゲストが §3.3 のトランポリンを GWLP_WNDPROC に登録した**。
- 次の停止点は CreateWindowExW 周りではなく **CompareStringW**（thunk 0x4231c0、`f_44ec40` ← `f_44f040` ← `f_653660` ← `f_6539e0` ← `f_653f40` ← unit 144 の proc 0x65c540）。既存の `fle_string.c` の停止（「ASCII の英数字以外を含む等しくない文字列」）。ウィンドウとは無関係。a4: init_count 145 / 942（前回 115）、最後の unit 144（proc 0x65c540、実行中）、started 132 / completed 131、関数実行数 803（前回 645）、入口数 15,274（前回 13,910）。
- §8 の次の停止点（GetWindowLongPtrW、SendMessageW など）には、まだ届いていない（CompareStringW が先）。
