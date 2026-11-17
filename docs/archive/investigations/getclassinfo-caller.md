# GetClassInfoW の呼び出し元（f_52a7f0 ほか）と RegisterClassW の形（調査。実装は決定 0070）

実行日: 2026-10-22。HEAD 8573601 に未コミットの変更を載せた状態。「文書」は Microsoft Learn（2026-10-22 取得）、「実測」はコマンドの出力（gdb、a4-survey、生成 C）と DLL のバイト列、「読み」は `objdump` の静的な読み、「推測」は未検証。成果物は `lift/out-cl/`（gitignore 済み）: `gdb-class2.txt`、`gdb-lasterror.txt`、`closure-class-sites.txt`、`closure-path-867e10.txt`、`a4-run*.txt`、`work/closure_path.py`。

## 0. 結論

- LoadIconW(NULL, "MAINICON") が NULL を返す（決定 0069）と、停止点は予告どおり **GetClassInfoW（thunk 0x424ba0、`f_52a7f0` の 0x52a820、rcx = 0x400000、rdx = 0x52a7c8 = L"TPUtilWindow"）**になった（実測）。
- ゲストの形は **Delphi の標準の `Windows.RegisterClass` の前置**: 「GetClassInfoW でクラスを探し、無ければ（または別のウィンドウプロシージャなら UnregisterClassW してから）RegisterClassW で登録する」。ウィンドウプロシージャは登録では呼ばれない（読み。§3、§4）。
- **決定 0070（GetClassInfoW と RegisterClassW の 2 API の仮想クラス表）を実装した**。4 条件（i〜iv）は満たした（§6）。
- 実装後の次の停止点は **CreateWindowExW（thunk 0x425780、`f_52a7f0` の 0x52a8be）**。実装範囲（条件 iv）の外。判断材料は §7。

## 1. 一次資料（Learn）

- GetClassInfoW（https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-getclassinfow）:
  - `BOOL GetClassInfoW(HINSTANCE hInstance, LPCWSTR lpClassName, LPWNDCLASSW lpWndClass)`。
  - hInstance: 「A handle to the instance of the application that created the class. To retrieve information about classes defined by the system (such as buttons or list boxes), set this parameter to NULL.」
  - lpClassName: 「The name must be that of a preregistered class or a class registered by a previous call to the RegisterClass or RegisterClassEx function. Alternatively, this parameter can be an atom. … The atom must be in the low-order word of lpClassName; the high-order word must be zero.」
  - 戻り値: 「If the function finds a matching class and successfully copies the data, the return value is nonzero. If the function fails, the return value is zero. To get extended error information, call GetLastError.」
  - **GetLastError の値は書かれていない**（このページに ERROR_CLASS_DOES_NOT_EXIST の記載は無い）。
- RegisterClassW（…/nf-winuser-registerclassw）:
  - `ATOM RegisterClassW(const WNDCLASSW *lpWndClass)`。戻り値: 「the return value is a class atom that uniquely identifies the class being registered. This atom can only be used by the CreateWindow, CreateWindowEx, GetClassInfo, GetClassInfoEx, FindWindow, FindWindowEx, and UnregisterClass functions …」「If the function fails, the return value is zero. To get extended error information, call GetLastError.」
  - 「No window classes registered by a DLL are unregistered when the DLL is unloaded. A DLL must explicitly unregister its classes when it is unloaded.」
  - **失敗時の GetLastError の値は書かれていない**。
- WNDCLASSW（…/ns-winuser-wndclassw）: フィールドは `UINT style; WNDPROC lpfnWndProc; int cbClsExtra; int cbWndExtra; HINSTANCE hInstance; HICON hIcon; HCURSOR hCursor; HBRUSH hbrBackground; LPCWSTR lpszMenuName; LPCWSTR lpszClassName;`。x64 の sizeof は **72**（style @0、lpfnWndProc @8（パディング 4 バイト後）、cbClsExtra @16、cbWndExtra @20、hInstance @24、hIcon @32、hCursor @40、hbrBackground @48、lpszMenuName @56、lpszClassName @64。8 バイトのポインタの整列から。ページは sizeof を直接書かない: **レイアウトは文書の型から導いた値**）。「The maximum length for lpszClassName is 256. If lpszClassName is greater than the maximum length, the RegisterClass function will fail.」「lpfnWndProc: A pointer to the window procedure. You must use the CallWindowProc function to call the window procedure.」「hInstance: A handle to the instance that contains the window procedure for the class.」
- About Window Classes（…/winmsg/about-window-classes）:
  - システムクラス（全プロセス用）: Button、ComboBox、Edit、ListBox、MDIClient、ScrollBar、Static。システム専用: ComboLBox、DDEMLEvent、Message、#32768〜#32772。「The system registers the system classes for a process the first time one of its threads calls a User or a GDI function.」「Each application receives its own copy of the system classes.」
  - 探索順: アプリケーションローカルクラス（名前と**インスタンスハンドルが一致**）→ アプリケーショングローバル → システムクラス。
  - 「RegisterClassEx fails if the amount of extra class memory requested is greater than 40 bytes」（ウィンドウのメモリも同じ）。
  - 「For DLLs, the hInstance member must be the handle to the .dll instance.」
- About Atom Tables（…/dataxchg/about-atom-tables）: 「registered classes are stored in an internal atom table … An application adds atoms to this atom table using the RegisterClass or RegisterClassEx function.」「The values of string atoms are in the range 0xC000 (MAXINTATOM) through 0xFFFF.」整数アトムは 0x0001〜0xBFFF。MAKEINTATOM（winbase.h）のページは「Converts the specified atom into a string, so it can be passed to functions which accept either atoms or strings」と書く。
- System Error Codes（1300–1699）: `ERROR_CLASS_ALREADY_EXISTS` 1410 (0x582)、`ERROR_CLASS_DOES_NOT_EXIST` 1411 (0x583)、`ERROR_CLASS_HAS_WINDOWS` 1412 (0x584)。これは**エラーコード表に値があるだけで、GetClassInfoW / RegisterClassW のページにそれが返ると書かれてはいない**。

**書かれていないこと**: GetClassInfoW / RegisterClassW が失敗したときの GetLastError の値。既登録の名前を再び登録したときの結果。クラス名の大文字小文字の扱い。最初に発行されるアトムの値。→ 決定 0070 は、これらの形を停止にするか「触れない」にする。

## 2. ゲストの呼び出し点（全部。実測 + 読み）

### 2.1 GetClassInfoW（thunk 0x424ba0、IAT スロット 0x1970a90）: 11 か所

`call rel32` の走査（`lift/objdump-text.txt` + `out-hi/objdump-tail.txt`）。テスト `the_guest_has_11_get_class_info_sites_and_8_register_class_sites` が同じ一覧を固定。

| 呼び出し点 | 関数 | hInstance | lpClassName（静的な値） | 形 |
|---|---|---|---|---|
| 0x52a820 | f_52a7f0 | `[0x15ec298]`（HInstance = 0x400000） | `[0x13be850]` → L"TPUtilWindow" | Get → 戻り値 ≠ 0 かつ `wc.lpfnWndProc == 0x424920`（DefWindowProcW の thunk）なら登録を飛ばす。それ以外 → Unregister + Register（§3） |
| 0x611574 | f_611550 | HInstance | `r8` = 引数（呼び出し元は EDIT、COMBOBOX、BUTTON×2、RICHEDIT20W） | 4 段の探索: HInstance → `[[0x15e15c8]]`（MainInstance = 0）→ NULL → `[rbx+0x48]`（Delphi の `TWinControl` の `WindowClass.hInstance`）。**システムクラスと他モジュールのクラスに向く** |
| 0x61158e | f_611550 | MainInstance（0） | 同上 | 同上 |
| 0x6115a0 | f_611550 | NULL | 同上 | 同上 |
| 0x6115b4 | f_611550 | `[rbx+0x48]` | 同上 | 同上 |
| 0x61199f | f_6118a0 | `[rbp+0xf0]`（読み。Delphi の `TWinControl` の WindowClass の hInstance に見える。推測） | `rdx = &[rbp+0x120]`（出力でなく、`[rbp+0xa8]` の仮想呼び出しで埋めた WNDCLASS のコピー先と同じ領域）。名前は動的 | Get → 見つかり wndproc ≠ 0x604d10 なら Unregister → Register（0x6119f4）。0x6119f9 で戻り値 ax を検査し、0 なら 0x44cb40（エラー報告） |
| 0x6d83d6 | 0x6d83a0 | HInstance | `[0x13f0688]` → L"TTimerWindow" | `f_52a7f0` と同じ形（wndproc は 0x6d8330） |
| 0x71ddaf | 0x71dd80 | HInstance | `[0x13f5f60]` → L"TLightweightLayeredControl" | 同じ形（wndproc は 0x71dd10） |
| 0x867e95 | f_867e10 | HInstance | `[0x1409cf8]` → L"TApplication" | 同じ形（wndproc は 0x424920） |
| 0xa1ba6f | f_a1ba00 | HInstance | `[0x14d6b88]` → L"VTPanningWindow" | 同じ形（wndproc は 0x424920） |
| 0xd706a6 | 0xd70670 | HInstance | `[0x151c9b0]` → L"TILRemoteWindow" | 同じ形（wndproc は 0xd70600） |

（0x61199f の読み: `rcx = [rbp+0xf0]`、`rdx = &[rbp+0x120]`、`r8 = &[rbp+0x60]`。Get の戻りの `wc.lpfnWndProc`（`[rbp+0x68]`）を 0x604d10 と比べ、同じなら登録を飛ばす。違えば UnregisterClassW(`&[rbp+0x120]`, `[rbp+0xf0]`) → `[rbp+0xe0] = 0x604d10`、`[rbp+0x118] = &[rbp+0x120]` を入れて RegisterClassW(`&[rbp+0xd8]`) を呼ぶ。Delphi VCL の `TWinControl.CreateWnd` の標準形に見える（推測。DLL に名前は無い）。）

**静的な値が分かる 6 か所**（f_52a7f0 ほか 5 つ）は hInstance = HInstance（実行時 0x400000）、名前は TPUtilWindow、TTimerWindow、TLightweightLayeredControl、TApplication、VTPanningWindow、TILRemoteWindow（実測。DLL のバイト列。テスト `the_static_classes_have_the_names_the_runtime_knows_and_a_window_procedure_and_no_class_style`）。0x6d83a0（TTimerWindow）、0x71dd80（TLightweightLayeredControl）、0xd70670（TILRemoteWindow）の 3 関数は先頭に「保存したハンドルが 0 でなければ何もしない」守り（`cmp [0x15f6030],0`、`cmp [0x13f5f38],0`、`cmp [0x16ba600],0`。それぞれ 0x6d83a7、0x71dd84、0xd7067e）を持つ（読み）。

**動的な名前の 5 か所**: f_611550（0x611574、0x61158e、0x6115a0、0x6115b4）と f_6118a0（0x61199f）。f_611550 の呼び出し元は 5 か所（0x631404、0x635224、0x6382d4、0x639514、0x6d6486）で、`r8` は EDIT、COMBOBOX、BUTTON、BUTTON、RICHEDIT20W（実測。DLL の文字列定数）。**システムクラスと RICHEDIT20W**。ここは決定 0070 の対象外（停止）。

### 2.2 RegisterClassW（thunk 0x425260、IAT スロット 0x1970b98）: 8 か所

| 呼び出し点 | 関数 | WNDCLASSW | クラス名 | 備考 |
|---|---|---|---|---|
| 0x52a859 | f_52a7f0 | 0x13be810（.data） | TPUtilWindow | Get が 0 のとき（または別 wndproc を Unregister したあと） |
| 0x6119f4 | f_6118a0 | `[rbp+0xd8]`（スタック） | 動的 | Get で見つからないとき |
| 0x6d8410 | 0x6d83a0 | 0x13f0648 | TTimerWindow | 〃 |
| 0x71dde9 | 0x71dd80 | 0x13f5f20 | TLightweightLayeredControl | 〃 |
| 0x81cfab | f_81cf60 | `[rbp+0x68]`（スタック。f_81cf60 が 0x48 バイトを 0 埋めして wndproc = 0x81ce10、hInstance = HInstance。名前のポインタは `[rbp+0xa8]` = 0x81d014 = L"ICMCALLBACKWND"） | ICMCALLBACKWND | **GetClassInfoW の前置なし**。直後に 0x425790 を呼ぶ（import の thunk ではなく、`push rbx; sub rsp,0x60; xor eax,eax; mov r10,rcx ...` で始まる DLL 内の関数。中身は読んでいない。0x425780（CreateWindowExW の thunk）の次の関数）。同じ名前を 2 度登録する形が来れば決定 0070 の実装は「既に登録済み」で停止する |
| 0x867eb3 | f_867e10 | 0x1409cb8 | TApplication | Get が 0 のとき |
| 0xa1baab | f_a1ba00 | 0x14d6b48 | VTPanningWindow | 〃 |
| 0xd706e0 | 0xd70670 | 0x151c970 | TILRemoteWindow | 〃 |

静的な WNDCLASSW（6 つ）: style = 0（CS_GLOBALCLASS なし）、cbClsExtra = cbWndExtra = 0、hIcon = hCursor = hbrBackground = NULL、menu = NULL、lpfnWndProc は 0x424920（DefWindowProcW の thunk、3 つ）または DLL の関数（0x6d8330、0x71dd10、0xd70600）、hInstance は .data では 0 で、呼び出しの直前に `mov [wc+hInstance], [0x15ec298]` で 0x400000 が入る（0x52a807、0x6d83bc、0x71dd95、0x867ea5、0xa1ba53、0xd7068c）。`lpszClassName` は L"..." の定数（0x52a7c8 ほか）。

RegisterClassW の**戻り値（アトム）**: f_52a7f0（0x52a859）は検査しない。0x867eb8（TApplication）は `test ax,ax; jne` で、0 のときだけ失敗の報告（0x41c820、0x448d40、0x411e80。直接 call の閉包に GetLastError の thunk は無い。実測の `closure.py`）へ進む。0x6119f9（f_6118a0）は 0 のとき `call 0x44cb40`（この関数の閉包に GetLastError の thunk 0x423540、0x407010 がある）。つまり**成功（非 0）を返せば、GetLastError を読む経路には入らない**（読み）。他の静的な呼び出し点（0x6d8410、0x71dde9、0xa1baab、0xd706e0）の戻り値の検査は個別には読んでいない。

## 3. f_52a7f0 の読み（実測の停止点）

```
52a800 mov rax,[0x15ec298]          ; HInstance
52a807 mov [0x13be828],rax          ; wc.hInstance = HInstance
52a81c lea r8,[rbp+0x68]            ; 出力の WNDCLASSW
52a820 call 0x424ba0                ; GetClassInfoW(HInstance, L"TPUtilWindow", &out)
52a825 test eax,eax ; setne al
52a82c je 0x52a83b
52a82e lea rcx,[0x424920] ; cmp [rbp+0x70],rcx ; je 0x52a85e   ; 見つかり、wndproc == DefWindowProcW のスタブなら登録を飛ばす
52a83b test al,al ; je 0x52a852
52a83f UnregisterClassW(L"TPUtilWindow", HInstance)             ; 見つかったが wndproc が違うとき
52a852 RegisterClassW(&0x13be810)
52a85e CreateWindowExW(WS_EX_TOOLWINDOW 0x80, L"TPUtilWindow", L"", WS_POPUP 0x80000000, 0,0,0,0, NULL, NULL, HInstance, NULL)   ; thunk 0x425780（0x52a8be）
52a8c6 cmp [rbx],0 ; je 0x52a8e4 ; ... SetWindowLongPtrW(hwnd, GWL_WNDPROC -4, ...)
```

- RegisterClassW の戻り（ax）は**見ない**（0x52a859 の直後はウィンドウ作成の準備）。GetLastError、SetLastError を `f_52a7f0` の 0x52a7f0〜0x52a8f4 は呼ばない（呼び出しは 0x424ba0、0x425670、0x425260、0x425780、0x425980 だけ。テスト `f_52a7f0_asks_for_its_class_then_registers_it_and_then_creates_the_window`）。
- GetClassInfoW が 0 を返す（まだ登録されていない）なら、「未登録」の分岐で **UnregisterClassW を呼ばずに RegisterClassW に進む**（`al = 0` のとき 0x52a83b の `test al,al; je 0x52a852` で飛ぶ）。つまり **UnregisterClassW は、仮想クラス表が正しく「未登録 = 0」を返す限り呼ばれない**。登録後に同じ名前を再び問い合わせる経路は、3 関数（0x6d83a0、0x71dd80、0xd70670）では先頭の守り（上）で避けている（読み）。f_52a7f0 側は呼び出し元（f_858700 ← f_867710）が 1 回しか呼ばない形に見えるが、再呼び出しの有無は追っていない（限界）。同じ名前を 2 度登録する形が来れば、決定 0070 の実装は「既に登録済み」で停止する。

実測（gdb、`gdb-class2.txt`）: `LoadIconW rcx=0 rdx=0x7f00 → LoadIconW rcx=0 rdx=0x867ba0 → GetClassInfoW rcx=0x400000 rdx=0x52a7c8 → RegisterClassW rcx=0x13be810 → 停止: CreateWindowExW`。GetLastError / SetLastError は GetClassInfoW の前の 2 回だけ（LoadIconW より前）。

## 4. ウィンドウプロシージャを今呼ぶ必要があるか（条件 i）

**無い**。RegisterClassW は WNDCLASSW を保存するだけ。ウィンドウプロシージャが呼ばれるのは CreateWindowExW（WM_NCCREATE、WM_CREATE）以降。GetClassInfoW は保存した `lpfnWndProc` の**値**を `wc` に写すだけで、ゲストは 0x52a82e で `lpfnWndProc == 0x424920` と**比べる**（呼ばない）。つまり (i) は満たされる。

## 5. システムクラス（組み込みクラス）に向く形（条件 iii）

- 静的な 6 か所（f_52a7f0 ほか）は、**ゲスト自身のクラス**（hInstance = HInstance、名前は DLL 内の定数）にだけ向く。システムクラスに向く静的な呼び出し点は無い（実測）。
- 動的な f_611550 は EDIT、COMBOBOX、BUTTON、RICHEDIT20W に向き、hInstance を HInstance → MainInstance（0）→ NULL → `[rbx+0x48]` の順に変えて 4 回呼ぶ。**この 4 か所はシステムクラスに向く**（読み。f_611550 の呼び出し元 6 か所のうち、`lea r8,[rip+..]` の直前 5 か所を DLL 内の文字列定数から確認: 0x631404 EDIT、0x635224 COMBOBOX、0x6382d4 BUTTON、0x639514 BUTTON、0x6d6486 RICHEDIT20W。6 か所目の呼び出し元 0x6d6486 の 1 つ前は別の文脈で、追っていない）。f_6118a0（0x61199f）は名前が動的で、15 か所から呼ばれる（0x621af4 ほか）。名前は追っていない。
- 決定 0070 の設計: **hInstance が 0x400000 でない形（NULL、MainInstance の 0 を含む）は停止**。名前が表に無く、ゲストの静的な 6 つの名前でもなければ停止（システムクラス、RICHEDIT20W を含む）。つまり (iii)「ゲストがシステムクラスに GetClassInfoW を向ける形は停止」を満たす。動的な 5 か所は今回の実行で到達しない（停止点より先）。

## 6. 実装の条件（i〜iv）の判定

| 条件 | 判定 | 根拠 |
|---|---|---|
| (i) 「GetClassInfoW で未登録を確認し、RegisterClassW で登録する」だけで、ウィンドウプロシージャを今呼ぶ必要がない | **満たす** | §3、§4。登録も問い合わせも wndproc の値を保存、比較するだけ |
| (ii) Learn から失敗時の戻り値（0）と GetLastError の値が書かれている、または書かれていない値は「last error に触れない」で済む | **満たす（後者）** | 失敗時の戻り値 0 は両ページに書かれる。GetLastError の値は書かれない。f_52a7f0 とほかの 5 つの静的な呼び出し点は、Get の 0 の後に GetLastError を読まず、Register の戻り（アトム）は検査しないか、非 0 なら下りない。0 を返す経路（GetClassInfoW の未登録）は last error に触れずに済む |
| (iii) 仮想クラス表の設計が小さく、システムクラスにゲストが GetClassInfoW を向ける形がない（ある場合は停止） | **満たす** | §5。表は {名前、hInstance、WNDCLASSW の 72 バイト、アトム}。システムクラスは停止 |
| (iv) 追加する API は GetClassInfoW と RegisterClassW の 2 つまで | **満たす** | 実装は 2 つだけ。UnregisterClassW（IAT 0x1970c38、thunk 0x425670）、GetClassNameW（thunk 0x424bb0、呼び出し点 0xb548da の 1 か所）、GetClassInfoExW、RegisterClassExW はゲストが輸入しない（RegisterClassExW、GetClassInfoExW）か、実装しない（Unregister、GetClassNameW）。RegisterClassW の呼び出し 8 か所の形は全部確認した（§2.2） |

## 7. 次の停止点: CreateWindowExW（実装しない。判断材料のみ）

実装後の実測（native と wasm が rsp= の行を除いて一致）: `fle: unsupported instruction at 0x425780`、top of stack `f_425780`（`f_52a7f0` の 0x52a8be の呼び出し）。a4: init_count 115 / 942、最後の unit 114（proc 0x623840、実行中）、started 111 / completed 110、関数実行数 645、入口数 13,910。

- 0x52a8be: `CreateWindowExW(0x80 /*WS_EX_TOOLWINDOW*/, L"TPUtilWindow", L"" /*0x52a8f4*/, 0x80000000 /*WS_POPUP*/, 0, 0, 0, 0, NULL, NULL, HInstance, NULL)`。直後（0x52a8c6）で `[rbx]`（引数のコールバック）が非 NULL なら `SetWindowLongPtrW(hwnd, GWL_WNDPROC = -4, f_52a5b0(rbx))`（`f_52a5b0` は 0x1000 バイトのブロックを `0x423ce0`（VirtualAlloc の thunk に見える）で確保して呼び出し側のオブジェクトを包むスタブを作る形。読み）。名前 `TPUtilWindow`、WS_EX_TOOLWINDOW、WS_POPUP、サイズ 0 から、**Delphi の `AllocateHWnd`（メッセージ用の隠しウィンドウ）に当たると読む**（推測。DLL にシンボル名は無い）。
- CreateWindowExW の呼び出し点（9 か所）: 0x425805、0x52a8be、0x611d09、0x6327fb、0x6328ce、0x71e133、0x85fb6c、0x867f7f、0xa1bb35。
- 必要になるもの（推測。読みから）: (1) 仮想ウィンドウ表（HWND の発行、クラスから wndproc を引く、WM_NCCREATE / WM_CREATE の送信の規則）。**CreateWindowExW は wndproc を呼ぶ**（ここで初めて wndproc の実行が要る。決定 0064 の `fle_call_guest` 機構が使える可能性）。(2) SetWindowLongPtrW(GWL_WNDPROC)、DefWindowProcW（0x424920 の thunk。DefWindowProcW を wndproc として呼ぶ）、DestroyWindow（thunk 0x424990）。(3) 「ウィンドウを作ったあとの実機の戻り値」: 作ったウィンドウの HWND の値、WM_NCCREATE への DefWindowProcW の戻り、失敗の戻りと GetLastError（Learn の CreateWindowExW は失敗時 NULL。値は書かれていない）。
- 判断（PM）: CreateWindowExW を「メッセージ専用の隠しウィンドウ」だけ答える仮想ウィンドウ表（HWND 発行、クラス表との連携、SetWindowLongPtrW、DestroyWindow）で別の決定にするか、止めて報告するか。条件 (iv) により今回は実装しない。

## 8. 確定できなかったこと（限界）

- 実機の GetClassInfoW / RegisterClassW の失敗時の GetLastError の値（Learn に無い。ERROR_CLASS_DOES_NOT_EXIST、ERROR_CLASS_ALREADY_EXISTS はエラーコード表にあるが、両ページはそれが返ると書かない）。
- 実機で最初に発行されるアトムの値。0xC000 は文書の「string atom の範囲の下限」から取った**仮の値**。
- クラス名の大文字小文字の扱い（書かれていない。決定 0070 は完全一致のみで、大文字小文字だけが違う名前は停止）。
- f_611550 と f_6118a0 の呼び出し元（動的な名前）の全部の名前。今回の実行は到達しない。
- 停止点（CreateWindowExW）より先で、ゲストが UnregisterClassW を呼ぶ経路（`DLL_PROCESS_DETACH` の `0x6d84b3` ほか）に届くか。届けば UnregisterClassW の扱いが未決。
- 0x621af4 ほか f_6118a0 の 7 つの呼び出し元の実際のクラス名。
