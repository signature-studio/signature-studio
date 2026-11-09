# 0071: 仮想ウィンドウ表（CreateWindowExW、SetWindowLongPtrW、DefWindowProcW）。隠しツールウィンドウの形だけを答え、WM_NCCREATE → WM_NCCALCSIZE → WM_CREATE を fle_call_guest で送る（仮の決定）

状態: **仮の決定**。PM の規則（段 1 の設計調査 → 条件 i〜iv を満たすときだけ実装）に従い、作業者（サブエージェント）が実装した。**この環境で最初の「ウィンドウ」の実体**で、以降の user32（GetWindowLongPtrW、DestroyWindow、SendMessageW、PostMessageW、メッセージループ）の基盤になる。「一次」は Microsoft Learn（2026-10-22 取得）の API の規則、「実測」は DLL のバイト列・gdb・コマンド出力（ゲストの引数と呼び出し点）、「読み」は `objdump` の静的な読み、「参考」は Wine の実装（`dlls/win32u/window.c`、`defwnd.c`。**一次ではない**。実機の挙動の代わりにならない）、「仮」は根拠となるページが無い選択。調査は `docs/investigations/window-design.md`。実装は `lift/runtime/fle_win.c`、`fle_win.h`（新規）、`fle_class.c`/`fle_class.h`（`fle_class_lookup` の追加）、`lift/crates/fe-lift/src/imports_table.rs`（4 行の追記: 3 行 + コメント）。テストは `lift/crates/fe-lift/tests/window.rs`（36 件）。

## 背景

決定 0070 で GetClassInfoW と RegisterClassW を答えると、harness は `CreateWindowExW`（thunk 0x425780、`f_52a7f0` の 0x52a8be）で止まった。`f_52a7f0` は Delphi の `AllocateHWnd` に当たる（読み）: `CreateWindowExW(WS_EX_TOOLWINDOW 0x80, L"TPUtilWindow", L"", WS_POPUP 0x80000000, 0, 0, 0, 0, NULL, NULL, HInstance, NULL)` の直後に、引数の TMethod が非 NULL なら `SetWindowLongPtrW(hwnd, GWL_WNDPROC -4, f_52a5b0(rbx))`。Learn に従えば CreateWindowEx は WM_NCCREATE、WM_NCCALCSIZE、WM_CREATE を戻る前に送る。クラス `TPUtilWindow` の lpfnWndProc は 0x424920（user32!DefWindowProcW の thunk）なので、送ったメッセージはゲストの独自コードではなく **DefWindowProcW** に届く（調査 §3、§4）。

## 決定

### ハンドル（仮）

- HWND は **`0x7F990000 + 4 * (i + 1)`**（i = 作った順、0 から。再利用しない）。領域は 1 MiB（`0x7F990000..0x7FA90000`）で、**画面 DC の領域（決定 0053、0x7FA90000）の直前**に置き、GDI（0x7FBA0000）、標準カーソル（0x7FCA0000）、DLL のカーソルとアイコン（0x7FDA0000）、モニター（0x7FEA0000）と重ならない（`fle_win.c` の `_Static_assert`、テスト `the_handle_region_overlaps_no_other_virtual_handle_region_and_is_below_2_gib` は 5 領域全部と、`runtime/*.h` の `_BASE` の 4 GiB 未満の領域が増えたとき気づくための走査）。0x80000000 未満（Learn「Interprocess communication」: 64 ビット Windows のハンドルは下位 32 ビットだけが有効で、切り詰めても符号拡張しても安全。HWND が挙がる）。値の式（4 ずつ増やす）は 0053、0054、0059 と同じ流儀。**具体的な値は Learn に無い**（仮）。
- 表に載っていない値（0、領域の外、4 の倍数でない、**上位 32 ビットだけ違う**、符号拡張した値、他の領域のハンドル、まだ発行していない値）は、SetWindowLongPtrW と DefWindowProcW が名前付きで停止する（値の全 64 ビットを比べる）。

### 窓表

窓 = `{wndproc（64 ビットの数）, hInstance, style, exstyle, クラスのアトム, live}`。**ホストの実ウィンドウ、実ディスプレイ、実メッセージキュー、時計を読まない**（呼び出しの純関数）。単一スレッド（最初の呼び出しのスレッドだけ。2 番目のスレッドは停止。0054 と同じ `fle_host_tid()`）。窓は削除しない（DestroyWindow は答えない）。失敗した窓（NCCREATE が FALSE、CREATE が -1）は表に残り live = 0（そのハンドルは再び使われず、使うと停止する）。

### CreateWindowExW（`HWND f(dwExStyle rcx, lpClassName rdx, lpWindowName r8, dwStyle r9, X, Y, nWidth, nHeight, hWndParent, hMenu, hInstance, lpParam)`。引数 5〜12 はスタック）

**答える形（0x52a8be。ゲストの 9 か所の呼び出しのうち、停止点までに届くのはこれだけ。調査 §2.2）**: `dwExStyle == WS_EX_TOOLWINDOW`（0x80）、`dwStyle == WS_POPUP`（0x80000000。DWORD 全部）、X = Y = nWidth = nHeight = 0（各スタックスロットの**下位 32 ビット**。ゲストは 4 バイトの `mov DWORD PTR [rsp+0x20], 0` で書き、スロットの上位 4 バイトは不定。Win64 の `int` 引数の規則）、`hWndParent == NULL`、`hMenu == NULL`、`hInstance == 0x400000`（FLE_EXE_HANDLE）、`lpParam == NULL`、`lpWindowName` が NULL または空文字列、`lpClassName` が `fle_class_lookup`（下）で引ける文字列。

- **戻り値**: 成功なら HWND（上）。**失敗なら NULL（Learn）**。失敗する形は、ゲスト自身の静的な 6 つの名前（決定 0070）で、まだ RegisterClassW で登録されていないクラス名（表は変わらない、メッセージは送らない）。**last error には触れない**（Learn に GetLastError の値が無い。仮）。
- **メッセージの送信**: 窓を表に入れて（live = 1）から、**WM_NCCREATE（0x81）→ WM_NCCALCSIZE（0x83）→ WM_CREATE（0x01）**を、**その窓のその時点の wndproc**に `fle_call_guest`（決定 0064）で 1 つずつ送る（wndproc は送信ごとに表から読み直す。WM_NCCREATE の中の SetWindowLongPtrW が次から効く。ゲストの TLightweightLayeredControl の wndproc がこの形）。
  - 順序: Learn の CreateWindowEx の Remarks が「sends WM_NCCREATE, WM_NCCALCSIZE, and WM_CREATE」と列挙する。Managing Application State が「WM_NCCREATE、WM_CREATE ... in the order listed」と明記。参考（Wine の `create_window`）も同じ順。PM の想定は NCCREATE と CREATE の 2 つだったが、Learn が WM_NCCALCSIZE を明記するので 3 つにした（調査 §5）。
  - wParam: NCCREATE と CREATE は 0（「not used」）、NCCALCSIZE は **FALSE（0）**。lParam: NCCREATE と CREATE は同じ CREATESTRUCTW へのポインタ、NCCALCSIZE は RECT へのポインタ。
  - **CREATESTRUCTW**（Learn のフィールド。x64 の整列から導いた値。Learn に sizeof は無い）: lpCreateParams @0（= lpParam）、hInstance @8、hMenu @16、hwndParent @24、cy @32、cx @36、y @40、x @44、style @48（@52 は 4 バイトのパディング = 0）、lpszName @56（= lpWindowName のポインタ）、lpszClass @64（= lpClassName のポインタ。**Learn は「ローカルなアトムかもしれないので GetClassName を使え」と書く。文字列の形だけを受けるので、渡したポインタをそのまま入れる**）、dwExStyle @72（@76 は 4 バイトの末尾パディング = 0）、**sizeof 80**。ゲストのスタック上に、API のフレームの下の 16 バイト境界（`(R[4] - 16) & ~15` の RECT、その下の `& ~15` の CREATESTRUCTW）に置き、3 つの送信の間ずっと有効（`R[4]` はその下から呼ぶ）。戻ったとき `R[4]` は入口の値。
  - NCCALCSIZE の **RECT** は **(x, y, x + cx, y + cy) = (0, 0, 0, 0)**（CreateWindowEx のページに値は無い。WM_NCCALCSIZE のページは wParam FALSE のとき「提案されたウィンドウの矩形」が入ると書く。参考: Wine）。仮。
  - **LRESULT（64 ビット全部）の扱い**: WM_NCCREATE は 1 で続行、**0 で失敗**（Learn「FALSE ... returns a NULL handle」）、他の値は**停止**（ページが「TRUE を返す」とだけ書き、他の値の意味を書かない。例: 2、-1、上位 32 ビットだけ非 0）。WM_NCCALCSIZE（wParam FALSE）は 0（「should return zero」）、他は停止。WM_CREATE は 0 で続行、**-1（0xFFFFFFFFFFFFFFFF）で失敗**（Learn「the window is destroyed and ... returns a NULL handle」）、他は停止（0xFFFFFFFF の 32 ビットの -1 を含む）。失敗のとき `R[0] = 0`、窓は live = 0。
- **送らないメッセージ（決定。仮）**: WM_GETMINMAXINFO、WM_SIZE、WM_MOVE、WM_WINDOWPOSCHANGING、WM_WINDOWPOSCHANGED、WM_SHOWWINDOW、WM_PARENTNOTIFY、WM_ACTIVATE 系、**失敗時の WM_DESTROY / WM_NCDESTROY**。理由: CreateWindowEx のページが挙げるのは 3 つだけ。WM_GETMINMAXINFO は Learn の中で食い違う（CreateWindow のページは pop-up も受けると書き、Window Features は WS_THICKFRAME か WS_CAPTION が要ると書く。ここは WS_POPUP だけ）。WM_SIZE、WM_MOVE、WM_WINDOWPOS* は作成時に送るとは書かれない（参考の Wine は送る）。WS_VISIBLE がないので WM_SHOWWINDOW は送らない（Window Features: WS_VISIBLE を指定したときだけ）。失敗時の WM_DESTROY は CreateWindowEx、WM_NCCREATE、WM_CREATE のどのページにも書かれない。今回の形では wndproc が DefWindowProcW なので送らなくても結果が変わらない（実機が送って、ゲストの wndproc がそれに反応する形は、この呼び出しの外）。テスト `source_uses_no_host_...` がソースにこれらの名前が無いことを固定する。
- **停止する形（名前付き）**: 上の形以外すべて。dwExStyle が 0x80 でない（0、WS_EX_TOPMOST、WS_EX_LAYERED、0x80 に他のビットが付く、…）、dwStyle が WS_POPUP でない（WS_VISIBLE、WS_BORDER、WS_CAPTION、WS_CHILD、WS_POPUPWINDOW、他のビット、…）、X、Y、nWidth、nHeight のどれかが 0 でない（CW_USEDEFAULT、-1、…）、hWndParent が NULL でない（窓、HWND_MESSAGE = -3、上位 32 ビットだけ非 0、…）、hMenu が NULL でない、hInstance が 0x400000 でない（NULL、MainInstance の 0 を含む）、lpParam が NULL でない、lpWindowName が空でない（先頭の UTF-16 ユニットの**全 16 ビット**を見る）、lpClassName が atom / NULL / 空 / 257 ユニット以上 / ASCII でない / 未登録で静的な 6 名でもない（EDIT、BUTTON、MDICLIENT、RICHEDIT20W、ICMCALLBACKWND、…）/ 大文字小文字だけが違う登録済みの名前、2 番目のスレッド、ハンドル領域の枯渇。**ゲストの 9 か所のうち 8 か所はこれらのどれかで停止する**（0x425805 経由の 3 つ = HWND_MESSAGE × 2 と ICMCALLBACKWND（style 0x80880000）、0x611d09 ほか動的な TWinControl 3 つ、0x71e133（ex 0x080800ac）、0x85fb6c（MDICLIENT）、0x867f7f（style 0x84ca0000）、0xa1bb35（cx = cy = 0x20）。調査 §2.2）。
- クラスの wndproc が関数の入口でない値（Delphi の `MakeObjectInstance` が書いたスロットなど）のとき、`fle_call_guest` が `indirect call to ... is not a known function entry` で停止する（決定 0064）。**今回の送信先は DefWindowProcW の thunk（0x424920）で、関数の入口**。

### SetWindowLongPtrW（`LONG_PTR f(HWND rcx, int nIndex rdx, LONG_PTR dwNewLong r8)`）

答える形: 表の live な窓、**nIndex = GWLP_WNDPROC（-4。rdx の下位 32 ビット。`int`）**、dwNewLong が 0 でない。新しい値を**数として**保存し（関数の入口でなくてよい。Delphi の `MakeObjectInstance` は自分が書いたメモリのアドレスを渡す。調査 §3.3）、**戻り値はその窓がそれまで持っていた wndproc**（Learn About Window Procedures: 「returns the address of the window's original window procedure」。窓の wndproc が差し替え済みなら直前の値）。保存した値は**呼ばない**（次のメッセージ送信まで）。last error には触れない（失敗時の値は Learn に無い。この形は失敗しない）。

停止: GWL_STYLE（-16）、GWL_EXSTYLE（-20）、GWLP_USERDATA（-21）、GWLP_ID（-12）、GWLP_HWNDPARENT（-8）、GWLP_HINSTANCE（-6）、extra memory（0 以上）、他の値（ゲストは GWLP_WNDPROC 13 か所のほか、-16 が 7、-20 が 18、-12 が 3、-21 が 1、0 が 1 か所で使う。調査 §2.1）。dwNewLong が 0（NULL の wndproc の意味は Learn に無い）。窓がない、live でない。2 番目のスレッド。

### DefWindowProcW（`LRESULT f(HWND rcx, UINT Msg rdx, WPARAM wParam r8, LPARAM lParam r9)`）

Learn は「戻り値は処理の結果で、メッセージによる」と書き、**どのメッセージの既定の戻り値も書かない**。WM_NCCREATE は「処理するなら TRUE を返して続行」、WM_CREATE は「処理するなら 0 を返して続行」、WM_NCCALCSIZE（wParam FALSE）は「0 を返すべき」と書くだけ。**既定処理の戻り値は Learn に無く、「既定処理が作成を止めない」という要件からの推論と参考（Wine の `defwnd.c`）による**:

- **WM_NCCREATE（0x81）→ 1**（lParam が非 NULL。NULL は停止）。
- **WM_CREATE（0x01）→ 0**（lParam が非 NULL。NULL は停止）。
- **WM_NCCALCSIZE（0x83）、wParam が 0（FALSE）→ 0**、lParam の RECT は**変えない**（表の窓は WS_POPUP のみで枠が無く、クライアント矩形はウィンドウ矩形に等しい。表が持つ唯一の形）。wParam が 0 でない（NCCALCSIZE_PARAMS）、lParam が NULL は停止。
- **ほかのメッセージはすべて名前付き停止**（`the default processing is not answered`）。Msg は下位 32 ビット（`UINT`）。表の live な窓でなければ停止（SetWindowLongPtrW と同じ）。2 番目のスレッド。wParam（NCCREATE、CREATE）は見ない（Learn: 使われない）。last error には触れない。

### fle_class.c への追加（最小限）

`int fle_class_lookup(const char *api, uint64_t name, uint64_t hinst, uint16_t *atom, uint64_t *wndproc)`: 決定 0070 のクラス表への問い合わせ（GetClassInfoW と同じ名前の規則）。登録済みなら 1 と、アトムと lpfnWndProc（数）。ゲストの静的な 6 名で未登録なら 0。ほかはすべて名前付き停止（api 名は呼び出し元が渡す）。状態は動かさず、wndproc は呼ばない、last error に触れない。既存のテストは変えていない（`tests/class.rs` の「他の API は答えない」一覧から CreateWindowExW と DefWindowProcW を外した。理由はコメント）。決定 0070 の本文は変更せず、該当節に「0071 で追記」を足した。

## 選択肢

- A. 窓表 + 完全な WM_* 送信（WM_GETMINMAXINFO、WM_SIZE、WM_MOVE、WM_PARENTNOTIFY ほか）。**不採用**: Learn が CreateWindowEx について書くのは 3 つだけで、他は参考にしか根拠が無く、WM_GETMINMAXINFO は Learn の中で食い違う。DefWindowProcW の範囲が大きくなる。
- **B'. 窓表 + CreateWindowEx の規則（NCCREATE → NCCALCSIZE → CREATE）だけ送る（採用）**。PM の推奨 B（NCCREATE と CREATE）から、Learn の CreateWindowEx のページが WM_NCCALCSIZE も挙げる点だけ変えた。
- C. 停止のまま。**不採用**: PM の決定は実装する方針で、条件 (i)〜(iv) を満たす（調査 §6）。
- D. メッセージを送らず、HWND だけ返す（「ウィンドウを作ったことにする」）。**不採用**: Learn が「戻る前に送る」と書く。他のクラス（TLightweightLayeredControl）は NCCREATE で SetWindowLongPtrW を呼ぶので、送らないとゲストの状態が実機と変わる。

## 値と出典の区別

| 項目 | 内容 | 出典 | 状態 |
|---|---|---|---|
| CreateWindowEx は WM_NCCREATE、WM_NCCALCSIZE、WM_CREATE を、戻る前に送る | CreateWindowExW ページ（Remarks、lpParam）、WM_CREATE ページ | **一次** | 確認済み |
| WM_NCCREATE が WM_CREATE より先 | Managing Application State、WM_NCCREATE ページ（「Sent prior to the WM_CREATE message」） | **一次** | 確認済み |
| NCCALCSIZE が NCCREATE と CREATE の間 | CreateWindowEx の Remarks の列挙順と参考（Wine）。ページは順序を明言しない | **一次（列挙順）** + 参考 | 確認済み（限界あり） |
| lParam が CREATESTRUCT へのポインタ（NCCREATE、CREATE） | WM_NCCREATE、WM_CREATE ページ | **一次** | 確認済み |
| WM_NCCREATE: TRUE で続行、FALSE で NULL | WM_NCCREATE ページ | **一次** | 確認済み |
| WM_CREATE: 0 で続行、-1 で破棄して NULL | WM_CREATE ページ | **一次** | 確認済み |
| WM_NCCALCSIZE（wParam FALSE）: lParam は RECT、戻り値 0 | WM_NCCALCSIZE ページ | **一次** | 確認済み |
| NCCALCSIZE の RECT が (x, y, x + cx, y + cy) | ページは「提案された矩形」とだけ | 参考 | **仮** |
| CREATESTRUCTW のフィールドと sizeof 80、オフセット | フィールドの型と x64 の整列から導いた（Learn に sizeof は無い） | **導出** | 確認済み（C コンパイラのレイアウトと `_Static_assert` で一致） |
| 失敗時の戻り値 NULL | CreateWindowExW ページ | **一次** | 確認済み |
| 失敗時の GetLastError の値 | ページに書かれない | — | **未確認**（触れない: 仮） |
| HWND の値（0x7F990000 + 4 * (i + 1)） | Learn は「下位 32 ビットだけ有効」とだけ | **仮** | 仮 |
| SetWindowLongPtrW(GWLP_WNDPROC) が直前の wndproc を返す | SetWindowLongPtrW ページ（直前の値）、About Window Procedures | **一次** | 確認済み |
| DefWindowProcW(NCCREATE) = 1、(CREATE) = 0、(NCCALCSIZE, FALSE) = 0 | ページに書かれない。作成を止めない要件と参考（Wine） | 推論 + 参考 | **仮** |
| 送らないメッセージ（GETMINMAXINFO ほか） | ページに書かれない、または Learn の中で食い違う | — | **仮** |
| 失敗時に WM_DESTROY / WM_NCDESTROY を送らない | ページに書かれない | — | **仮** |
| ゲストの CreateWindowExW 9 か所の引数 | バイト列と objdump の静的な読み | **実測** + 読み | 確認済み（テストで固定） |
| クラス TPUtilWindow の lpfnWndProc は 0x424920（DefWindowProcW の thunk） | DLL の .data の WNDCLASSW | **実測** | 確認済み（`tests/class.rs` とこの決定のテスト） |
| MakeObjectInstance のスロットが VirtualAlloc したメモリのマシンコード | `f_52a5b0` の読みと gdb（`SetWindowLongPtrW hwnd=7f990004 idx=-4 new=<ヒープのアドレス>`） | **実測** + 読み | 確認済み |
| 領域が既存の 5 領域と重ならない | ヘッダの定数（`_Static_assert` とテスト） | **実測** | 確認済み |

## 採用条件（PM の i〜iv）の確認

- (i) 不可視のユーティリティウィンドウの形（WS_POPUP、サイズ 0、親なし、WS_VISIBLE なし）で、停止点までに届く形だけ: **満たす**（調査 §2.2、§6）。
- (ii) 送信が `fle_call_guest` で行える（callee-saved を守る、再入 16 段で足りる）: **満たす**。送信先は生成 C の関数（DefWindowProcW の thunk）。入れ子は CreateWindowExW → DefWindowProcW の 1 段。テストで wndproc の中からの SetWindowLongPtrW、CreateWindowExW の入れ子（2 段）、callee がシャドウ空間に書く形を確認した。
- (iii) 追加する API は 3 つまで: **満たす**。GetWindowLongPtrW（66 か所）、DestroyWindow、SendMessageW（216）、PostMessageW（139）、UnregisterClassW、GetMessageW、DispatchMessageW、ShowWindow は実装しない（`tests/window.rs` が表にこれらが無いことを固定）。
- (iv) 窓表が小さく決定論的: **満たす**。

## 決定論

表は呼び出しの純関数で、ホストの実ウィンドウ、実ディスプレイ、実メッセージキュー、モジュール、時計を読まない。テスト `source_uses_no_host_...` が、ホストの API、`fle_dispatch`、`fle_set_last_error`、`long`、0071 が送らないメッセージの名前を、`fle_win.c` に使わないことを固定している。native と wasm（node）は rsp= の行を除いて完全に一致した（a4 の測定は `docs/progress.md` の担当の外。結果は下）。

## 実装後の実測

CreateWindowExW を越えて、`f_52a7f0` は HWND 0x7F990004 を返し、gdb で次の順に API が呼ばれた: `CreateWindowExW`（ex = 0x80、`L"TPUtilWindow"`）→ `DefWindowProcW(0x7F990004, 0x81)` → `DefWindowProcW(.., 0x83)` → `DefWindowProcW(.., 0x01)` → `SetWindowLongPtrW(0x7F990004, -4, <MakeObjectInstance のスロット>)`。次の停止点は **CompareStringW（thunk 0x4231c0。`f_44ec40` ← `f_44f040` ← `f_653660` ← … ← unit 144 の proc 0x65c540）**: `fle: CompareStringW: character other than an ASCII letter or digit in unequal strings (not defined by the MSDN page or outside the emulated locale; not implemented)`（既存の `fle_string.c` の停止。ウィンドウとは別の API）。a4: init_count 145 / 942、最後の unit 144（proc 0x65c540、実行中）、started 132 / completed 131、関数実行数 803、入口数 15,274。native と wasm は rsp= の行を除いて完全に一致。

## 撤回条件

- 実 Windows の CreateWindowExW の失敗時の GetLastError が分かり、ゲストがそれを読むと分かったとき（その値にする。last error に触れない仮を改める）。
- Learn が HWND の値、DefWindowProcW の NCCREATE / CREATE / NCCALCSIZE への戻り値、WM_NCCALCSIZE の RECT、失敗時の WM_DESTROY / WM_NCDESTROY の有無、WS_POPUP の WM_GETMINMAXINFO の有無を書いたとき（その値にする）。
- ゲストの wndproc が NCCREATE / NCCALCSIZE / CREATE のほかのメッセージ（WM_SIZE、WM_MOVE、WM_GETMINMAXINFO）に反応し、それが CreateWindowEx から実機で送られると分かったとき（そのメッセージを送る形に広げる）。
- ゲストのクラスの wndproc が「関数の入口でない値」（MakeObjectInstance のスロット）になる形に届いたとき（`fle_call_guest` の前にスロットを解釈するか、別の決定）。調査 §3.3、§8。
- CreateWindowExW の他の 8 か所（HWND_MESSAGE、ICMCALLBACKWND、TWinControl の動的な形、MDICLIENT、TApplication、VTPanningWindow、TLightweightLayeredControl）に届いたとき（形ごとに別の判断。システムクラスの MDICLIENT はシステムクラス表が要る）。
- GetWindowLongPtrW、DestroyWindow、SendMessageW、PostMessageW などを実装するとき（窓表の読み書きの拡張と、`DefWindowProcW` の範囲の見直し。調査 §8）。

## 確認できていないこと

- 実 Windows の CreateWindowExW、DefWindowProcW の実際の戻り値と、失敗時の GetLastError（Learn に無い）。
- 実 Windows が WS_POPUP のウィンドウの作成時に WM_GETMINMAXINFO、WM_SIZE、WM_MOVE を送るか（Learn の中で食い違う、または書かれていない。ここでは送らない）。
- 動的な CreateWindowExW 3 か所（0x611d09、0x6327fb、0x6328ce）の実際のクラス名と引数（実行に届いていない）。
- `SetWindowLongPtrW(GWLP_WNDPROC, slot)` で登録された MakeObjectInstance のスロットにメッセージが送られる段階（SendMessageW、DispatchMessageW、WM_TIMER など）の挙動。
