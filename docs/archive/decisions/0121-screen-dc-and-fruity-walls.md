# 0121: 仮想画面 DC（`GetDC(NULL)`）の属性モデルと物理寸法（HORZSIZE 508 mm・VERTSIZE 286 mm）、QFC の窓クラス名、メッセージ専用窓、`CreateFruityInstance` の壁 18〜20。壁 21（`kernel32!CreateThread`）で停止。すべて仮の決定

状態: **仮の決定（すべての応答と選択）**。依頼は PM（2026-10-30。決定 0121 の内容は PM の仮の決定。番号 0121 は PM の仮の割り当て）。測定・実装・テスト・文書は作業者（サブエージェント）が行った。**作業は HEAD 167a1db の `git archive HEAD lift docs tools` で始めた。差分 `fruity7.diff` は同じ HEAD の clean な archive に `patch -p1` で当たる（`MERGE-NOTES.md` に dry-run の結果）。** コンパイル（`-Wall -Wextra -Werror`、警告・エラーなし）、全体テスト 1 回の通し、生成 C の関数単位の比較、変異検査、FRUITY 無効の独立な測定と FRUITY 有効の独立な 2 回の測定まで実施した（件数は「検証」）。**実機 Windows との照合は 0 件。wasm・ブラウザは未測定（native のみ）。**

**測定用の基底・種・註釈（dsp 0x2000000、Blend2D 0x4700000、QFC 0x1d00000、freetype 0x4b00000、ILTools 0x6000000、フォント 2 ファイル、レジストリの種、署名の主張の表）と、仮の応答を含む構成での値である。** 決定 0101 の表記では「A4 完走（仮の応答 N 件を含む）」の形の値で、達成の判定は PM が行う。

**肯定的に見せる点は少ない。** `CreateFruityInstance` は**戻らない**。メッセージループにも**入っていない**（目的 (a)、(b) のどちらにも到達していない）。壁 18〜20 の 3 壁を直し、**41,177 関数入口目で `kernel32!CreateThread`（壁 21）に止まる**（決定 0119 の停止点 40,512 入口目から +665 入口、新規の関数 485 → 572）。壁 21 は PM の止まる条件 (d)（スレッド）に当たるので、ここで止めた。壁は 12 個の上限のうち 3 個しか直していない（残り 9 は使っていない）。

区分の凡例: **実測**＝コマンドの出力を見た事実。**読み**＝逆アセンブルやコードを読んだ上での解釈。**Learn**＝Microsoft Learn のページに書かれていること（取得日 2026-10-30。取得したページの URL と引用は各所）。**出典なしの選択**＝ページにも実測にも根拠が無く、作業者または PM が選んだ値。**非公式**＝Microsoft 以外の出典。**未確認**。

## 先に結論

| 項目 | 内容 |
|---|---|
| `CreateFruityInstance`（種あり・主張の表あり） | **戻らない。ブロッキングのメッセージ待ちにも入っていない**。最後の停止点: `fle: unsupported instruction at 0x1d06d80`（`kernel32.dll!CreateThread` の QFC 側の thunk `f_1d06d80`）。**41,177 関数入口目**、新規の関数 572、終了 134。独立な 2 回のビルド（別の OUT。同じ cargo target）で同一（native `5c9ba474a5ff7b986900c6414d8cc3430dca4ab6434d70fda711b2637751d25c`）。最後の 20 入口は `logs/fin1/a4-multi.stdout` |
| 直した壁 | **3 壁**（壁 18、19、20。壁の通し番号は決定 0119 の続き）。壁 21 で停止（停止条件 (d) スレッド） |
| 壁 18 の訂正 | PM の指示文と決定 0119 は壁 18 を「MM_LOMETRIC (3)」と書くが、**ゲストが渡した `iMode = 3` は MM_HIMETRIC**（0.01 mm）。MM_LOMETRIC は 2。根拠は Learn の [MS-WMF]／[MS-EMF] MapMode Enumeration（「解釈が分かれた点」の 1） |
| FRUITY 無効の既定ビルド | a4: init_count 942/942、runnable 895/895、`returned from entry`、終了 0 は不変。**functions 5,308、entries 1,007,947 は不変**。native `49ce46e505089c118e4fbaef9b05b56db6b2fabdfbea8799016dc86a8d9f1d03` → **`4ea77a4f2436b50ecb15a650ba9794b97439a06afd23f2d47b5b06885e61e91c`**（最終の木。独立 2 回で同一。途中の木では `2a0511c9…16bd`）。SHA が変わった原因は**実行経路の変化ではなく runtime のオブジェクトの変化**（生成 C は FLEngine 59,380 関数と 5 モジュールで本体の変わった関数 0。「FRUITY 無効の既定ビルドの native の SHA-256」） |
| 全体テスト | **`cargo test --workspace --no-fail-fast --offline` を **1 回の通し**（47 分 37 秒、2026-10-30T15:22:22Z〜16:09:59Z、終了 0）で **121 test result・1,699 passed・0 failed・ignored 0**。基準（決定 0119）は 120・1,680・1 failed。差: +1 バイナリ（`wall_fr7`、18 件）、`wall_fr5` の 1 件が通る（0119 では失敗）= 1,680 + 18 + 1 = 1,699。`wall_fr5` は 14 件すべて通る。`No space left`・`Disk quota`・`panicked`・`FAILED` は 0。警告は既存のテスト（`monitor`、`sse_shuf`、`INIT_HEX`）のもの** |
| 生成 C | FLEngine 59,380 関数・dsp 8,479・Blend2D 2,067・QFC 6,723・freetype 1,653・ILTools 12,931 のどれも**本体が変わった関数は 0**（runtime だけの変更。生成 C の SHA-256 は旧新で同一: FLEngine `b92c4b12b6bfb590f7aaaaa5ca05d4fce311d4e2addea3b2162300d9668df25d`）。`imports_table.rs` は変えていない（応答表は 454 行のまま） |
| `wall_fr5` の失敗 | **テストの期待の誤りと判定し、テスト側を直した**（256×256 の MAINICON は PNG 圧縮。HEAD 167a1db の clean archive でも `FLENGINE_PE` に原本を渡すと落ちることを再現）。`wall-fr5-triage.md` |
| wasm | runtime が変わるので wasm の SHA は変わる。再ビルドは別の作業（この作業では native のみ） |

## 背景

- 前段（決定 0119）: `CreateFruityInstance(0xC0, 0xEDE95A, 偽ホスト, &instance)` は 40,512 関数入口目の `gdi32!SetMapMode(GetDC(NULL), 3)`（壁 18）で止まった。実験 X18（差分に入れない）は 96 DPI から逆算して答え、41,150 入口の `GetClassInfoW(0x1d00000, "TSharedAnimTimerWindow")` まで進んだ。
- PM の決定（0121、変えない）: (1) 仮想画面 DC は決定 0111 の表示 DC の既定値と同じ属性モデルで扱う（属性の設定・照会のみ。画素を書く操作は名前付き停止のまま）。物理寸法は決定 0053 の仮想画面（1920×1080、96 DPI）から導く: HORZSIZE = 1920×25.4/96 = 508 mm、VERTSIZE = 1080×25.4/96 = 285.75 → 286 mm（丸めは出典なしの選択として列挙）。実機未照合と書く。ホストの画面・DPI は読まない。(2) スプラッシュを避けるレジストリの種は採らない。(3) メッセージキューは、ブロッキングなメッセージ待ちに届くまでに必要になった壁で最小の FIFO を設計してよい。待ち自体は実装しない。偽のメッセージを注入しない。(4) 偽らない。(5) A4 の基準を動かしてよいが原因を説明する。(6) 止まる条件 (a)〜(g)。
- 土台の再現（実測。`logs/base1`、`logs/base2`、`logs/fr0`）: 入力 DLL・フォントはすべて実在し SHA が一致（`lift/out-git/disk-inventory/inputs-sha256.tsv`。再抽出は不要）。FRUITY 無効を独立 2 回で再現（TAG `t2331d00e8ebb`、942/942、895/895、functions 5,308、entries 1,007,947、native `49ce46e5…1d03`）。FRUITY 有効で壁 18 を再現（`fle: SetMapMode: hdc is a DC of GetDC(NULL): its attributes are not modelled (only memory DCs are)`、40,512 入口、新規の関数 485、終了 134、native `029bb30e…5763`。シャドウスタックは決定 0119 と同じ `f_4245c0` ← `f_6f6410` ← `f_6f6570` ← `f_6ba930` ← … ← `CreateFruityInstance`）。

## 壁の表

各壁の全記録は `wall-log.md`（`docs/investigations/fruity7/wall-log.md`）。入口数は `fruity:` の「lifted entries since the call began」（`logs/<名>/`）。

| # | 停止メッセージ | 原因（区分） | 種類 | 修正（場所） | 測定（入口数／新規の関数） |
|---|---|---|---|---|---|
| 18 | `fle: SetMapMode: hdc is a DC of GetDC(NULL): its attributes are not modelled (only memory DCs are)` | `f_6f6410`: `hdc = GetDC(NULL); SetMapMode(hdc, 3); pt.y = -pt.y; LPtoDP(hdc, &pt, 1); ReleaseDC(NULL, hdc)`（実測のバイト列）。**3 は MM_HIMETRIC**（[MS-WMF]）。`f_6f6570` は点の x に何かの値、y に 0 を入れて呼ぶ（読み） | モデルの拡張（PM の決定 1） | D1〜D4（`fle_gdi.c`、`fle_gdc.c`、`fle_gdc.h`） | 40,512／485 → 40,513／486（次の停止: `LPtoDP` の「mapping mode is 3, not MM_TEXT」。属性の保存だけ通した段階）→ **41,150／559**（次の停止: 壁 19） |
| 19 | `fle: GetClassInfoW: the name is not registered here and is not one the guest registers itself at a static site (a predefined or another module's class?): not answered`（`hInstance = 0x1d00000`、`TSharedAnimTimerWindow`） | QFC の `AcquireSharedAnimTimer`（0x1e864f0）→ `f_1e85eb0` → `f_1e85950`（0x1e85950）が自前のクラスを `GetClassInfoW` → 無ければ `RegisterClassW`（決定 0070 と同じ形。読み） | 名前付き停止の拡張（窓クラス表） | D5（`fle_class.c`） | 41,150／559 → 41,153／562（次の停止: 壁 20） |
| 20 | `fle: CreateWindowExW: dwExStyle 0x0 is not WS_EX_TOOLWINDOW (0x80): only the hidden tool windows of the guest are answered (decisions 0071, 0119)` | `CreateWindowExW(0, "TSharedAnimTimerWindow", "", 0, 0, 0, 0, 0, HWND_MESSAGE, NULL, 0x1d00000, NULL)`（0x1e85a18。実測の引数）。メッセージ専用窓 | 窓表の拡張 | D6（`fle_win.c`、`fle_win.h`） | 41,153／562 → **41,177／572**（次の停止: 壁 21） |
| 21 | `fle: unsupported instruction at 0x1d06d80`（`kernel32.dll!CreateThread`） | `TSharedAnimTimer` の TThread を `CREATE_SUSPENDED` で作る（`f_1df10f0` 0x1df11cf。start = 0x1df0ee0） | **大きなモデル（停止条件 (d) スレッド）** | なし（**ここで止めた**） | 41,177／572 で停止 |

壁の種類の件数（この作業の壁 18〜21。各壁を主な種類 1 つに数える）: モデルの拡張 1（壁 18）、名前付き停止の拡張 1（壁 19）、窓表の拡張 1（壁 20）、大きなモデル 1（壁 21）、メッセージループ 0。数え方: 壁 18〜20 の 3 壁を直し、壁 21 で停止（上限 12 壁のうち 9 壁を残した）。

## 決定（仮）

### 旧 → 新の表（既存の応答・既存のテストの期待値を変えるもの）

| # | 場所 | 旧 | 新 | 根拠 |
|---|---|---|---|---|
| D1 | `runtime/fle_gdi.c` の属性の 14 関数（`MoveToEx`、`GetCurrentPositionEx`、`SetBkMode`、`SetROP2`、`SetMapMode`、`SetStretchBltMode`、`GetStretchBltMode`、`SetWindowOrgEx`、`SetViewportOrgEx`、`GetWindowOrgEx`、`LPtoDP`、`SetBrushOrgEx`、`GetBrushOrgEx`、`SaveDC`、`RestoreDC`。数えると 15 関数） | `GetDC(NULL)` の DC は名前付き停止 `hdc is a DC of GetDC(NULL): its attributes are not modelled (only memory DCs are)` | **画面 DC も属性状態を持つ**。`need_attr_dc` が、画面 DC には `GetDC` のハンドルごとの状態（メモリ DC と同じ `gdi_dc`、同じ既定値）を、メモリ DC にはその DC の状態を返す。他のハンドル（NULL、削除済みのメモリ DC、ビットマップ）は従来どおり停止。状態は `ReleaseDC` で破棄（`fle_gdi_screen_released`。`fle_gdc.c` から弱参照で呼ぶ）、次の `GetDC(NULL)` は既定値から始まる | Learn Display Device Context Defaults: 「Upon first creating a display device context, the system assigns default values for the attributes … Background mode OPAQUE … Brush origin (0,0) … Current pen position (0,0) … Drawing mode R2_COPYPEN … Mapping mode MM_TEXT … Stretch mode BLACKONWHITE … Viewport extent (1,1) … Viewport origin (0,0) … Window extent (1,1) … Window origin (0,0)」「Changes to the attribute values of a common, parent, or window device context are not permanent. When an application releases these device contexts, the current selections, such as mapping mode and clipping region, are lost as the context is returned to the cache」（https://learn.microsoft.com/en-us/windows/win32/gdi/display-device-context-defaults）。**範囲**: 属性の設定・照会だけ。`SelectObject`（ビットマップは従来どおり NULL、他の物は停止）、`SelectPalette`、`SetTextColor`、`SetBkColor`、`GetClipBox`、画素の操作（`BitBlt`、`PatBlt`、`FillRect`、`StretchDIBits`、…）は画面 DC で従来どおり名前付き停止（描画しない。PM の決定 1） |
| D2 | `fle_imp_LPtoDP` | MM_TEXT 以外の写像モードはすべて停止（「the extents of the other modes are not on the pages」） | 画面 DC の **MM_LOMETRIC（2）と MM_HIMETRIC（3）**は、窓の範囲 = HORZSIZE × 10（LOMETRIC）または × 100（HIMETRIC）、VERTSIZE × 同じ係数、ビューポートの範囲 = HORZRES、−VERTRES（y 軸は上向き）として、Learn の式 `Dx = ((Lx − WOx) × VEx / WEx) + VOx` で変換する。丸めは「最も近い整数、半端は 0 から遠い方」（出典なしの選択）。27 ビット（符号つき）に収まらなければ従来どおり停止。**画面 DC の他のモード（LOENGLISH、HIENGLISH、TWIPS、ISOTROPIC、ANISOTROPIC）、メモリ DC の MM_TEXT 以外は停止のまま**。MM_TEXT（既定）は不変（範囲 1 と 1） | Learn Mapping Modes and Translations: MM_LOMETRIC「Each unit in page space is mapped to 0.1 millimeter in device space. The value of x increases from left to right. The value of y increases from bottom to top.」、MM_HIMETRIC「0.01 millimeter」、「the extents are set by the system when SetMapMode is called. They cannot be changed.」、式（https://learn.microsoft.com/en-us/windows/win32/gdi/mapping-modes-and-translations）。LPtoDP「LPtoDP calculates complex floating-point arithmetic, and it has a caching system for efficiency. Therefore, the conversion result of an initial call to LPtoDP might not exactly match…」（丸めの規則は書かれていない）。**範囲の導出（HORZSIZE と HORZRES から作る）は出典なしの選択**（非公式の corroboration: Wine の `win32u/mapping.c` の `set_map_mode` は同じ式 `wnd_ext = virtual_size × 10`、`vport_ext = (virtual_res, −virtual_res_y)`。非公式） |
| D3 | `runtime/fle_gdc.c`／`fle_gdc.h` の `GetDeviceCaps` | HORZSIZE（4）、VERTSIZE（6）は名前付き停止「not decided」（決定 0053 が PM の判断に残した値） | **HORZSIZE = 508、VERTSIZE = 286**。1920 px × 25.4 mm/inch ÷ 96 px/inch = 508 mm（割り切れる）、1080 × 25.4 ÷ 96 = 285.75 mm → 最も近い整数 286（丸めは出典なしの選択）。LOGPIXELSX/Y = 96 と整合。ホストの画面・DPI を読まない | PM の決定 1。Learn GetDeviceCaps: 「HORZSIZE Width, in millimeters, of the physical screen.」「VERTSIZE Height, in millimeters, of the physical screen.」「GetDeviceCaps might calculate incorrect info because it assumes either fixed DPI (96 DPI) or a fixed size」（https://learn.microsoft.com/en-us/windows/win32/api/wingdi/nf-wingdi-getdevicecaps）。値は書かれていない。**実機が答える値と違う可能性がある（実機未照合）** |
| D4 | `fle_gdi.h` | `FLE_GDI_MM_TEXT` と `FLE_GDI_MM_ANISOTROPIC` だけ | `FLE_GDI_MM_LOMETRIC`（2）と `FLE_GDI_MM_HIMETRIC`（3）を足す | 数値 1〜8 は、通常の API ページでは名前だけ（非公式の mingw-w64 と windows-sys の値と一致）。**Learn の [MS-WMF] MapMode Enumeration（https://learn.microsoft.com/en-us/openspecs/windows_protocols/ms-wmf/97b31821-a0db-4113-a210-fffbcedcec4a）と [MS-EMF] MapMode Enumeration（…/ms-emf/109d7955-767e-4955-a0e2-f8b5e14f757a）が `MM_TEXT = 0x0001, MM_LOMETRIC = 0x0002, MM_HIMETRIC = 0x0003, MM_LOENGLISH = 0x0004, MM_HIENGLISH = 0x0005, MM_TWIPS = 0x0006, MM_ISOTROPIC = 0x0007, MM_ANISOTROPIC = 0x0008` と書く**（メタファイル形式の仕様。一次資料） |
| D5 | `runtime/fle_class.c` の `is_own_static_name` | 6 つ（`TPUtilWindow`、`TTimerWindow`、`TLightweightLayeredControl`、`TApplication`、`VTPanningWindow`、`TILRemoteWindow`） | **7 つ目 `TSharedAnimTimerWindow` を足す**。未登録の間 `GetClassInfoW` は 0（バッファにも last error にも触らない）、登録後は 72 バイトを返す。他の名前（大文字小文字違いと前後の差を含む）は従来どおり名前付き停止 | 決定 0070 の規則（ゲストが静的な呼び出し点で `GetClassInfoW` → 直後に `RegisterClassW` する自前の名前だけ「見つからない」を答える）。QFC の 0x1e85981〜0x1e859c0 が同じ形（読み）。Learn GetClassInfoW: 「If the function fails, the return value is zero.」 |
| D6 | `runtime/fle_win.c`／`fle_win.h` の `CreateWindowExW` | `hWndParent` は NULL だけ。`HWND_MESSAGE`（−3）は停止（決定 0071。`window.rs` が `("message-only", 8, 0xFFFFFFFFFFFFFFFD)` で固定） | **第 3 の形を足す: メッセージ専用窓**。`hWndParent == HWND_MESSAGE（0xFFFFFFFFFFFFFFFD）`のとき、dwExStyle 0、dwStyle 0、位置・大きさ 0、menu NULL、lpParam NULL、窓名は空か NULL、hInstance は既知のモジュールの基底、クラスは `fle_class_lookup` が解くもの、を答える。メッセージは同じ順（WM_NCCREATE → WM_NCCALCSIZE → WM_CREATE）。窓表の項目と `fle_win_view` に `msgonly` を足す。**`GetWindowLongPtrW(GWLP_HWNDPARENT)` は message-only の窓で名前付き停止**（親が何かは Learn に無い）。他の parent（子窓・オーナー窓、HWND_MESSAGE の近傍の値）は従来どおり停止。決定 0071・0119 の第 1・第 2 の形は不変 | Learn CreateWindowExW: 「To create a message-only window, supply HWND_MESSAGE or a handle to an existing message-only window.」「The CreateWindowEx function sends WM_NCCREATE, WM_NCCALCSIZE, and WM_CREATE messages to the window being created.」（https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-createwindowexw）。Window Features: 「A message-only window enables you to send and receive messages. It is not visible, has no z-order, cannot be enumerated, and does not receive broadcast messages. The window simply dispatches messages.」（…/winmsg/window-features）。**`HWND_MESSAGE` の数値はこれらのページに無い**: `(HWND)-3` は windows-sys（docs.rs）の `HWND_MESSAGE = 0xfffffffffffffffd`（非公式）で、ゲストが渡す値（`48 c7 44 24 38 fd ff ff ff`、実測）でもある |
| D7 | 既存テストの期待値 | 下の「既存のテストの更新」 | 同 | 期待値の更新漏れ |

## 止まった壁（壁 21）の内容・選択肢・推奨

**実測**（`logs/fin1/`、2 回目 `logs/fon2/`）: 停止点は `kernel32!CreateThread`（QFC の thunk `f_1d06d80`。41,177 入口、新規の関数 572、終了 134）。引数（実測）: `lpThreadAttributes 0`、`dwStackSize 0`、`lpStartAddress 0x1d0fdb0`（QFC の Delphi のスレッド起動の手続き）、`lpParameter 0x…10d0`、`dwCreationFlags` はスタック上（呼び元 `f_1df10f0` の 0x1df11cf が `mov dword ptr [rsp+0x20], 4` = **CREATE_SUSPENDED**。引数付きの分岐は 0x1df119c の 0x10004 = STACK_SIZE_PARAM_IS_A_RESERVATION | CREATE_SUSPENDED）。シャドウスタック: `f_1d06d80` ← `f_1d0fe20`（BeginThread のラッパー）← `f_1df10f0`（TThread.Create）← `f_1df1000` ← `f_1e85eb0`（`TSharedAnimTimer` のコンストラクタ）← `AcquireSharedAnimTimer`（0x1e864f0）← `f_736ff0` ← `f_71e900` ← `f_721bb0` ← `f_722200`（スプラッシュ画面のフォームのコンストラクタと読む）← `f_136cba0` ← `f_13734d0` ← `f_11b3810` ← `f_13751e0` ← `CreateFruityInstance`。直前の import thunk: `EnumDisplaySettingsW(NULL, ENUM_CURRENT_SETTINGS, …)`（応答あり）、`DefWindowProcW`×3（0x81、0x83、0x01）、`CreateWindowExW`、`RegisterClassW`、`GetClassInfoW`、`timeBeginPeriod`、`timeGetDevCaps`（`logs/fin1/a4-multi.stdout` の「last 48 entries」）。

**読み**: ゲストは `CreateThread(CREATE_SUSPENDED)` の後に `SetThreadPriority`（0x1df176a）→ `ResumeThread`（0x1df1667）で始める。スレッドの本体（0x1df0ee0 → `Execute`）は `MsgWaitForMultipleObjects(2, handles, FALSE, 1000, 0x40)` と `PeekMessageW` のループ（0x1df1f51、0x1df1f70）で、**メッセージ待ちはスレッドの側にあり、`CreateFruityInstance` を呼んだスレッドの側にはこの時点で無い**。デストラクタ側（0x1e85fc0）は `GetTickCount`／`Sleep(0)` で最大 1000 ms 待ってスレッドの終了を確かめる。QFC の呼び出し点: `CreateThread` 1、`ResumeThread` 2、`SuspendThread` 1、`SetThreadPriority` 2、`GetExitCodeThread` 1、`WaitForSingleObject` 8、`PeekMessageW` 2、`MsgWaitForMultipleObjects` 1、`PostMessageW` 2（`call rel32` の走査）。

**分類**: PM の止まる条件 **(d) 大きなモデルが要る壁（スレッド）**。

**選択肢**

| 案 | 内容 | 結果（読み） | 注意 |
|---|---|---|---|
| Z1. 止まる（推奨。この作業の選択） | スレッドは実装しない | 41,177／572 で停止 | 到達点は動かない。スレッドの扱いは次の段階（ブラウザのイベントループ統合）と一緒に PM が決める |
| Z2. 「作ったことにする」（`CreateThread` が CREATE_SUSPENDED のハンドルを返し、`ResumeThread` は何も実行せず前の中断回数 1 を返す） | スレッド手続きは一度も走らない | 先へ進める見込み（次の壁に当たる） | **偽る**: 実機ではスレッドが走って TThread のフラグ（`[+0x18]` Started、`[+0x1d]` Finished など）を書き、デストラクタの `Sleep` ループはそれを読む。結果を実機の挙動なしに決める = PM の決定 4 に反する。PM の判断が要る |
| Z3. 協調スレッド | `ResumeThread` で別のシャドウスタック・C スタック・`R[]` を持つ実行を作り、`MsgWaitForMultipleObjects`／`PeekMessageW` で制御を戻す | 大きなモデル（lift の C はグローバル `R[]` を共有。スレッドごとの `fs:`／TEB／TLS、C スタックの切り替えが要る）。fe-lift の局所変更ではなく runtime の設計 | 停止条件 (d)(e)。wasm は JSPI／Asyncify の設計（決定 0115 周辺）と同じ問題で、ブラウザのイベントループ統合と同時に設計すべき |
| Z4. 経路を避ける | レジストリの種などで `AcquireSharedAnimTimer` を呼ばせない | **採れない**（PM の決定 2）。共有タイマーはスプラッシュのフォームの作成が呼び、避ける設定を作る根拠もない | — |

**推奨**: Z1（止まる）。理由: Z2 は偽る。Z3 はスレッドごとの実行状態という大きな設計で、ブラウザのイベントループ統合（次の段階）と一緒に決めるべき対象。PM がスレッドの扱いを決めれば、Z2 か Z3 を実装して壁 22 以降を測れる。

## 出典なしの選択の一覧（撤回条件つき）

| # | 場所 | 選択 | 撤回条件 |
|---|---|---|---|
| S1 | D3 | **VERTSIZE の丸め 285.75 → 286**（最も近い整数。切り捨てなら 285）。HORZSIZE の 508 は割り切れる。物理寸法そのものを 1920×1080 と 96 DPI から導く（PM の決定 1）。**実機未照合**（実機の GetDeviceCaps は表示ドライバの値か 96 DPI の計算で、別の値を返しうる） | 実機 Windows の測定で値が違うと分かったとき。ゲストが HORZSIZE／VERTSIZE の値に依存する箇所に届いたとき |
| S2 | D2 | **窓の範囲を HORZSIZE×10（×100）、VERTSIZE×10（×100）から作る**（96/254 から直接ではなく）。x では 1920/5080（LOMETRIC）= 0.37795 で 96/254 と同じ。y では 1080/2860 = 0.37762 と 96/254 = 0.37795 で、**差がある**（HIMETRIC の x = 2540 → 96 px、y = 2540 → 約 95.9 px が 96 px にならない）。これは VERTSIZE を整数ミリに丸めた結果で、S1 の帰結 | S1 と同じ。実機の LPtoDP の結果と食い違うと分かったとき |
| S3 | D2 | **LPtoDP の丸め**: 商を最も近い整数に、半端は 0 から遠い方へ（MulDiv のページの規則を借りた。LPtoDP のページは「complex floating-point arithmetic」「initial call … might not exactly match」と書くだけで規則を書かない）。変換に浮動小数点の演算は使わない（整数の演算のみ） | 実機の LPtoDP が別の丸めを使うと分かったとき |
| S4 | D1 | **画面 DC の属性状態は `GetDC` のハンドルごと**に持ち、`ReleaseDC` で破棄する。Learn は「common, parent, or window device context」の属性が release で失われると書く。`GetDC(NULL)` の DC がその種類かどうか（画面全体の DC）はページが明示しない（読み） | 実機の DC キャッシュの挙動が違うと分かったとき（同じ値が 2 回目の `GetDC` で返るなど） |
| S5 | D1 | 画面 DC の `SaveDC`／`RestoreDC` の対象はメモリ DC と同じ（選択物 5 つは常に 0、モード・原点・位置・ブラシ原点）。画面 DC に選択物と色を持たないので、`RestoreDC` の「色・位置・ブラシ原点が違えば停止」の規則は画面 DC でも同じ | 画面 DC の選択物・色をモデル化するとき |
| S6 | D5 | `TSharedAnimTimerWindow` を「ゲスト自身の名前」に数える（決定 0070 の規則の適用。QFC の 1 か所の呼び出しの形に基づく） | ゲストが別の形で呼ぶと分かったとき |
| S7 | D6 | **HWND_MESSAGE = (HWND)−3**（SDK ヘッダの値と読む。Learn の API ページに数値は無い。ゲストの値と一致）。message-only 窓にも 3 つのメッセージを送る。矩形は作成時の値（0×0）。`GWLP_HWNDPARENT` は停止。WM_CREATE などのクラスの窓プロシージャは決定 0071・0119 と同じ扱い | 実機で message-only の窓の親や矩形が別と分かったとき |
| S8 | D4 | `FLE_GDI_MM_LOMETRIC` = 2、`FLE_GDI_MM_HIMETRIC` = 3（[MS-WMF]／[MS-EMF] が裏付け。つまり 1〜8 の数値は一次資料に確認できた） | — |

## 実装した場所

- 変更: `lift/runtime/fle_gdi.c`（`need_attr_dc`、画面 DC の状態の表 `scr_dc`、`fle_gdi_screen_released`、`fle_gdi_screen_attr`、`LPtoDP` の拡張。属性の 15 関数の `need_mem_dc(…, screen_dc_why)` を `need_attr_dc` に置換）、`fle_gdi.h`（定数 2 つ、検査用の関数 2 つ）、`fle_gdc.c`（HORZSIZE／VERTSIZE の応答、`ReleaseDC` から `fle_gdi_screen_released` を弱参照で呼ぶ）、`fle_gdc.h`（`FLE_GDC_HORZSIZE`、`FLE_GDC_VERTSIZE`）、`fle_class.c`（名前 1 つ）、`fle_win.c`／`fle_win.h`（第 3 の形、`msgonly`、`FLE_HWND_MESSAGE`）。runtime に新しい `.c`／`.h` は作っていない。`imports_table.rs`、fe-lift、fe-decode は変えていない。
- 新規: `lift/crates/fe-lift/tests/wall_fr7.rs`、`docs/decisions/0121-screen-dc-and-fruity-walls.md`、`docs/investigations/fruity7/wall-log.md`。
- 既存のテストの更新（期待値の更新漏れ。旧 → 新と理由）:
  - `gdc.rs`: `horzsize_and_vertsize_stop_by_name` を `horzsize_and_vertsize_are_derived_from_the_virtual_display_at_96_dpi` に置換（旧: 停止「not decided」→ 新: 508 と 286、整数の算術の検算つき、解放済みと NULL の hdc は従来どおり停止）。
  - `gdi_bulk.rs`: 属性の関数の「画面 DC は停止」の期待を外した（`attr_dc_refusals` = 画面を除く NULL・削除済み・ビットマップ。`GetClipBox` は従来の `dc_refusals` のまま）。4 テスト（`the_brush_origin_is_kept…`、`a_mode_the_pages_do_not_list…`、`lptodp_and_the_origin_calls_stop…`、`restore_dc_stops…`）。画面 DC の挙動は `wall_fr7.rs` が固定。
  - `window.rs`: `a_parent_a_menu_an_instance…` の「message-only は停止」を外し、近傍の値（−2、−3 の下位 32 ビット）を足し、HWND_MESSAGE + 第 1 の形のスタイルの停止を足した。
  - `wall_fr5.rs`: **`wall_fr5` の失敗の直し**（旧: 256×256 と 1000×1000 の要求が `rt_icon 6` を選ぶ → 新: PNG 圧縮で名前付き停止。64×64 のケースを足した）。`wall-fr5-triage.md`。
- 既定の挙動: FRUITY 無効の既定ビルドの a4 は、runtime のオブジェクト（`fle_gdi.o`、`fle_gdc.o`、`fle_class.o`、`fle_win.o`）の変化を除いて変わらない（942/942、895/895、functions 5,308、entries 1,007,947、`returned from entry`、終了 0）。

## FRUITY 無効の既定ビルドの native の SHA-256（理由）

基準 `49ce46e505089c118e4fbaef9b05b56db6b2fabdfbea8799016dc86a8d9f1d03`（HEAD 167a1db、FRUITY 無効。独立 2 回で再現）→ **`2a0511c90a18c080d17f29ef1292e4dd8a57d9a7e6127808756adbc9b8cb16bd`**（この差分、FRUITY 無効。独立 3 回: `logs/finoff1`、`foff1`、`foff2` で同一）。a4 の数字は不変（942/942、895/895、functions 5,308、entries 1,007,947、`returned from entry`、終了 0）。理由（実測）:

1. **FLEngine と 5 モジュールの生成 C は同一**（旧新の `fe-lift` 出力の SHA-256 が全 6 ファイルで一致: FLEngine `b92c4b12…df25d`、`gen/old` と `gen/new` の `sha256.txt` で比較。関数単位の比較で本体が変わった関数は FLEngine 0／59,380、dsp 0／8,479、Blend2D 0／2,067、QFC 0／6,723、freetype 0／1,653、ILTools 0／12,931）。
2. **変わったのは runtime のオブジェクトだけ**: `fle_gdi.c`、`fle_gdc.c`、`fle_class.c`、`fle_win.c`（と、`fle_gdi.h`／`fle_gdc.h`／`fle_win.h` を include するもの）。native の実行ファイルはこれらをリンクするので SHA が変わる。**FRUITY 無効の a4 の経路に、変わった関数は入らない**（実測: functions 5,308、entries 1,007,947 が不変。FRUITY 無効の a4 は画面 DC の属性・メッセージ専用窓・QFC の共有タイマーに触れない）。
3. したがって「entries／functions が動いた場合の経路の変化」は無く、SHA の変化は runtime のオブジェクトの変化として説明できる。

## テスト

新規 `tests/wall_fr7.rs`（**18 件**）と、更新（上の「既存のテストの更新」）。`gdi_bulk.rs` と同じ流儀の「実 `fle_gdi.c`／`fle_gdc.c` をリンクした C のプログラム」（画面 DC）と、`wall_fr6.rs` と同じ流儀の「実 `fle_win.c`／`fle_class.c`／`fle_callback.c`／`fle_imports.c`／`fle_env.c` をリンクし、置換 dispatch 表と test 側の `fle_realmod_find_handle`（0x1d00000 を実モジュールの基底とする）を持つ C のプログラム」（窓）。期待値は決定の算術（整数）と Learn の文言から書き、C のコードから読み戻していない。

- 壁 18（GDI。10 件）: 新しい画面 DC の既定値（Display Device Context Defaults の 13 項目）、`SetXxx` が前の値を返す、`GetDC` のハンドルごとの状態、`ReleaseDC` で破棄・次の `GetDC` は既定値、MM_TEXT の `LPtoDP`（原点の加算、27 ビットの端）、MM_HIMETRIC・MM_LOMETRIC の `LPtoDP`（手計算した丸めを含む 8 点。y 軸の向き、原点との合成）、ゲストの呼び出し（`f_6f6410` の形）の 6 点（2540 → 96 など）、`SaveDC`／`RestoreDC`、メモリ DC は画面 DC の状態に影響されない、画面 DC に残る停止（`SelectObject` の brush、`SetTextColor`、`SetBkColor`、`GetClipBox`、`PatBlt`、`FillRect`、`SelectObject` のビットマップは NULL で停止しない）、`LPtoDP` の他モード・NULL・個数 0・オーバーフローの停止、`SetMapMode` の 0 と 9 の停止。
- 壁 18（GetDeviceCaps。`gdc.rs`）: HORZSIZE 508・VERTSIZE 286、ピクセル寸法と 96 DPI との整合、解放済み・NULL の停止。
- 壁 19: `TSharedAnimTimerWindow` が登録前は 0 で last error・バッファに触れず、登録後に 72 バイト（wndproc 0x1e858e0）を返す、別の hInstance では見えない、他の名前（前後の差、`EDIT`、大文字小文字違い）と他の hInstance は名前付き停止。
- 壁 20: メッセージ専用窓の作成（3 つのメッセージの順序、CREATESTRUCTW の parent と hInstance、窓表の項目、2 つ目のハンドル）、`GetWindowLongPtrW`／`SetWindowLongPtrW` の答えと `GWLP_HWNDPARENT` の停止、他のスタイル・位置・大きさ・menu・lpParam・hInstance・窓名・登録外の名前・親の近傍の値（−2、−4、下位 32 ビット、1）の停止、未登録の自前の名前は NULL でテーブルに触れない。
- 実 DLL のバイト列: `FLEngine_x64.dll` の `f_6f6410`（`mov edx, 3`、`neg dword ptr [rbp+0x24]`、`mov r8d, 1` ほか）と MapMode の番号（[MS-WMF]）、QFC の呼び出し点（0x1e85981 の `GetClassInfoW`、クラス名の UTF-16、0x1e859f1 の `hWndParent = -3`、0x185a2c の空の窓名、`CreateThread` の thunk と CREATE_SUSPENDED の即値）。
- ソース検査: ホストの画面・DPI・時計・ファイルを読む関数を使わない、メッセージキューの関数（`PostMessageW`、`GetMessageW`、`PeekMessageW`、`WaitMessage`、`MsgWaitForMultipleObjects`、`DispatchMessageW`、`TranslateMessage`、`PostQuitMessage`）が `fle_win.c` に無い。

## 検証（実測）

- **全体テスト**（`logs/fulltest/cargo-test.txt`、`.err`、`start.txt`、`end.txt`、`rc.txt`）: 上の表のとおり。環境変数は `tools/fulltest.sh`（`FLENGINE_PE`／`FLENGINE_DLL` に原本の `FLEngine_x64.dll`、`ILTOOLS_DLL`、`BLEND2D_PE` に `fe-pe rebase` が作ったリベース済みの像（SHA-256 `24e01ec9…`）、`DSP_IPPV2_PE`、`DSP_IPPV2_PE_2000000`（`4fd25502…`）、`FL64_EXE`）。
- **全体テストの経過（正直な記録）**: 全体の通しを 3 回起動した。(1) 最初の通し（`interrupted-run-1.*`）は、ツールの呼び出しの時間切れ（3,600,000 ms）とサーバーの再起動で中断され（90 バイナリ目まで。`rc.txt` なし）、数えない。(2) 2 回目の通し（`run-A-before-states-fix.*`。14:06:12Z〜15:05:41Z、59 分 29 秒、終了 0）は **121・1,699・0 failed** だったが、その後の変異検査で **M1、M2、M3 が生き残った**（下）ので、テスト側の弱さを直した（`wall_fr7` に `fle_gdi_screen_states` の検査を足し、変異の対象のテストの指定を直した。runtime には検査用の関数 `fle_gdi_screen_states` を足した）。(3) 直した後の木で通し直し（上の表。数えるのはこの 1 回）。2 回目と 3 回目は同じ結果（121・1,699・0 failed）。最後の通しの後にソースを変えていない（`find -newer` で確認）。
- **変異検査**（`logs/mutation-run-121.txt`、`tools/mutate121.py`。木のコピー `mut/`（`src/` の runtime・crates・harness だけ）で 1 件ずつ変更 → 該当テストを実行 → 元に戻す）: 対照 M0（変更なし）は通る。**M1〜M15 の 15 件すべて検出**。M1 HORZSIZE を 509 に、M2 VERTSIZE を 285（切り捨て）に、M3 画面 DC の状態を `ReleaseDC` で破棄しない、M4 画面 DC の既定のモードを MM_HIMETRIC に、M5 メトリックのモードで y 軸を反転しない、M6 MM_HIMETRIC の係数を 10 に、M7 LPtoDP を 0 方向の切り捨てに、M8 画面 DC のどのモードも答える、M9 画面 DC の `SelectObject`（ブラシ）を答える、M10 `TSharedAnimTimerWindow` を自前の名前の一覧から外す、M11 message-only の `HWND_MESSAGE` を下位 32 ビットに、M12 message-only がツール窓のスタイルを受理、M13 message-only を印づけない、M14 message-only の `GWLP_HWNDPARENT` が 0 を答える、M15 `wall_fr5` の PNG の停止を外す。**経過の正直な記録**: 1 回目の変異検査（`mutation-run-121-first-pass-with-survivors.txt`）は 3 件が生き残った: M1、M2 は変異の対象のテストの指定が `wall_fr7`（該当テストが無い）で、フィルタが何にも一致せずに「通った」と数えられていた（ランナーを直して「テストが 0 件走った」を失敗にした。指定を `gdc` に直したら検出）。M3 は `wall_fr7` のテストが解放後の状態の破棄を観測しておらず、本当に生き残った（`fle_gdi_screen_states` を足して検出）。M15 は `FLENGINE_PE` を変異ランナーの環境に渡していなかったので、テストが SKIP して通っていた（環境を直したら検出）。
- **FRUITY 無効の既定ビルド**（`logs/base1`、`base2`、`finoff1`、`foff1`、`foff2`、`gof1`、`gof2`。別の OUT、同じ cargo target。fe-lift `bb0c20fc…fa14d`、fe-pe `743e2faf…70b0` は決定 0119 と同一）: a4 942/942、895/895、functions **5,308**、entries **1,007,947**、`returned from entry`、終了 0。native: HEAD `49ce46e505089c118e4fbaef9b05b56db6b2fabdfbea8799016dc86a8d9f1d03`（`base1`、`base2`。独立 2 回で同一）。この差分の木: `2a0511c9…16bd`（`finoff1`、`foff1`、`foff2`。同一）。最後の通しの前に runtime に検査用の関数 1 つ（`fle_gdi_screen_states`）を足したので、最終の木では **`4ea77a4f2436b50ecb15a650ba9794b97439a06afd23f2d47b5b06885e61e91c`**（`gof1`、`gof2`。独立 2 回で同一）。a4 の数字は 3 つの木で同一。SHA が変わった原因は runtime のオブジェクトの変化（上の「FRUITY 無効の既定ビルドの native の SHA-256」）。
- **FRUITY 有効の最終構成**（種 = nsis + app、主張の表あり。`logs/gon1`、`gon2`。別の OUT、同じ cargo target）: `fle: unsupported instruction at 0x1d06d80`（`kernel32.dll!CreateThread`）、**41,177 入口、新規の関数 572、終了 134**、native **`924363226af42236122a1367e816452083e4caec3deea8a2dfcd01b887b681a7`**（2 回とも同一。`fruity:` の出力は経過時間と PIE のポインタ値を除いて同一）。最後の測定の前の木（`fin1`、`fon1`、`fon2`）では native `5c9ba474…d25c`（同じ停止点・入口数）。functions／entries（DllMain の後、呼び出しの前）は 5,325／1,008,084 で、呼び出しの間に 41,177 入口・新規の関数 572（決定 0119 の停止点 40,512／485 から +665／+87）。
- **停止点の詳細**（壁 21。`logs/stop21/`、`stop21-record.txt`。最終の木に、窓表・クラス表を `fruity:` の行で出す測定専用の変更（差分に入れない）を足した木で取った。停止点・入口数・新規の関数・直前の入口は最終の測定と同じ）: C スタック（シンボル化）、シャドウスタック（深さ 27、下の 12 は DllMain の longjmp の残り）、レジスタ、直前の import thunk の列、直前の 20 関数入口（`stop21-record.txt`）、窓 6 つとクラス 5 つ（下の「到達点の状態」）。
- **生成 C**（`logs/gencmp-all.txt`、`genall-new.txt`、`genall-old.txt`）: FLEngine と 5 モジュールの 6 つの生成 C の SHA-256 が旧新で**同一**。関数単位の比較で本体が変わった関数は 0（FLEngine 59,380、dsp 8,479、Blend2D 2,067、QFC 6,723、freetype 1,653、ILTools 12,931）。
- **差分**: `fruity7.diff`（14 ファイル、149 KB）。HEAD 167a1db の clean な archive（`git archive HEAD lift docs tools`）に `patch -p1 --dry-run` が当たることを確認する（`MERGE-NOTES.md`、`logs/patch-verify.txt`）。

## 到達点の状態（`CreateFruityInstance` の停止時点。メッセージループではない）

(a)（戻る）にも (b)（ブロッキングのメッセージ待ち）にも到達していない。停止時点（壁 21）の状態（実測。`logs/stop21/`）:

ゲストが作った窓（仮想窓表。6 窓。決定 0119 の 5 窓に message-only の窓が 1 つ増えた）:

| HWND | クラス（アトム、hInstance） | style／exstyle | 矩形（x, y, w × h） | 窓プロシージャ | その他 |
|---|---|---|---|---|---|
| 0x7f990004 | `TPUtilWindow`（0xc000、0x400000） | 0x80000000／0x80 | (0, 0, 0 × 0) | `MakeObjectInstance` のスロット | — |
| 0x7f990008 | `TPUtilWindow`（0xc001、0x6000000 = ILTools） | 0x80000000／0x80 | (0, 0, 0 × 0) | スロット | — |
| 0x7f99000c | `TPUtilWindow`（0xc000、0x400000） | 0x80000000／0x80 | (0, 0, 0 × 0) | スロット | — |
| 0x7f990010 | `TPUtilWindow`（0xc000、0x400000） | 0x80000000／0x80 | (0, 0, 0 × 0) | スロット | — |
| 0x7f990014 | `TApplication`（0xc002、0x400000） | 0x84ca0000／0x80 | (960, 540, 0 × 0) | スロット | icon_big 0x7fba0624 |
| **0x7f990018** | **`TSharedAnimTimerWindow`（0xc004、0x1d00000 = QFC）** | **0／0** | (0, 0, 0 × 0) | **0x1e858e0（QFC のクラスの窓プロシージャ。`msgonly` = 1）** | 壁 20 で作られた |

クラス表は 5 クラス（`TPUtilWindow` ×2、`TApplication`、`TLightweightLayeredControl`、`TSharedAnimTimerWindow`）。ゲストが作った窓は**どれも可視でない**（`WS_VISIBLE` なし）。画面 DC: 生存 0、属性の状態 0（壁 18 の `f_6f6410` は `ReleaseDC` を呼ぶ）。**メッセージキュー: なし**（`PostMessageW`、`GetMessageW`、`PeekMessageW`、`WaitMessage`、`MsgWaitForMultipleObjects` は実装していない。`CreateFruityInstance` を呼んだスレッドの側ではどれにも届いていない。QFC のスレッドの `Execute` が `MsgWaitForMultipleObjects`／`PeekMessageW` を持つが、そのスレッドは作られていない）。

直前の 20 関数入口と import thunk の列、C スタック、シャドウスタックは `stop21-record.txt`（1 本のファイル）。

## 解釈が分かれた点

1. **壁 18 の「MM_LOMETRIC (3)」**: PM の指示文と決定 0119 は MM_LOMETRIC (3) と書く。Learn の [MS-WMF]／[MS-EMF] は MM_LOMETRIC = 2、MM_HIMETRIC = 3。ゲストの `mov edx, 3`（0x6f642d。実測のバイト列）は **MM_HIMETRIC（0.01 mm）**。窓の範囲の係数は 100（LOMETRIC なら 10）。PM の決定 1 は MM_LOMETRIC を名指すので、**LOMETRIC も同じ規則で答える**（ゲストの呼び出しで通る道はなく、未実行）。「0.1 mm の点をピクセルに換算」という決定 0119 の読み（`f_6f6410` の用途）は、実際は 0.01 mm（HIMETRIC。IPicture の `get_Width`／`get_Height` が HIMETRIC で返す Learn の記述と整合する。読み）。
2. **画面 DC の MM_LOMETRIC／MM_HIMETRIC の範囲を HORZSIZE から作ること（S2）**: PM の決定 1 は「HORZSIZE/VERTSIZE を決定 0053 から導く」とだけ書く。LPtoDP の範囲をそれらから作るのは作業者の導出（Wine も同じ構造）。96/254 を直接使えば y の値が 0.1% ほど違う（S2）。HORZSIZE を使う側に倒した（Learn の式 `VEx/WEx` に最も素直に当てはまり、`GetDeviceCaps` の値と LPtoDP の結果が整合する）。
3. **属性の制限を画面 DC にも広げたこと（D1）**: PM の決定 1 は「属性の設定・照会のみ」。`SelectObject`・色・`GetClipBox`・画素の操作は画面 DC で停止のままにした。`SaveDC`／`RestoreDC` は属性なので広げた（選択物と色は持たない）。広げ過ぎかもしれない判断なので、PM が戻すと言えば戻せる（`need_attr_dc` を `need_mem_dc` に戻すだけ）。
4. **`HWND_MESSAGE` の数値（S7）**: Learn のページは値を書かない。非公式の `0xFFFFFFFFFFFFFFFD` とゲストの値の一致で足した。決定 0071 が名前付き停止にしていた形を解禁した（決定 0071 の撤回条件「ゲストがその形に届いたとき」に当たる）。
5. **壁 21 を (d) と読んだこと**: `CreateThread(CREATE_SUSPENDED)` は「スレッドを作るだけ」で偽らずに済む読みもありうる（Z2 の一部）が、`ResumeThread` と、スレッドが書くフラグをゲストが読む点（デストラクタの `Sleep` ループ）で偽ることになる。PM の決定 4（偽らない）と (d)（スレッド）の両方に当たるので止めた。
6. **全体テストの与え方**: 依頼の環境変数（`FLENGINE_PE` に原本、`BLEND2D_PE` にリベース済みの像）で通した。`BLEND2D_PE` に原本（`lift/out-blend2d/Blend2D.dll`）を渡すと `msvc_switch` が落ちる（決定 0119 の記録と同じ）。

## PM の判断が要る点

1. **壁 21（スレッド）の扱い**（Z1 止まる／Z2 作ったことにする／Z3 協調スレッド）。推奨は Z1。
2. **メッセージキューの設計**: この作業の実行はキュー（`PostMessageW`、`GetMessageW`、`PeekMessageW`、`WaitMessage`、`MsgWaitForMultipleObjects`）に**届かなかった**。ゲストの `CreateFruityInstance` の側にメッセージ待ちがあるのかは、壁 21 の先を測るまで分からない（QFC のスレッドの側には `MsgWaitForMultipleObjects`／`PeekMessageW` がある）。キューを実装していないので、(b)（最初のブロッキングなメッセージ待ち）にも到達していない。`PostMessageW` の NULL の扱い（決定 0119 の PM の判断点 3）は未決のまま。
3. **VERTSIZE の丸め（S1）、LPtoDP の丸め（S3）、画面 DC の状態を `GetDC` のハンドルごとにすること（S4）**。
4. **壁 18 の MM_LOMETRIC → MM_HIMETRIC の訂正**（解釈 1）。決定 0119 の文言の訂正が要る。
5. **決定 0119 から持ち越し**: FL64.exe のパス（実機値）、スプラッシュを避けるレジストリの種（PM が採らないと決定済み）、システムメニューの項目の集合と HMENU の値（0119 の S6）、`SetWindowTextW` を決定 1 の範囲に含めたこと、時計と署名の有効期間（決定 0117）。
6. **docs の索引**: `docs/README.md` の 0121 の行は `docs/README-0121-line.txt`。注記の件数（決定ファイル、番号の種類、索引の行、仮の決定）は、足す側の確認を要する（この作業では `docs/README.md`、`docs/progress.md` を編集していない）。決定 0120 の番号が決定ファイルとして存在するかは未確認（HEAD の `docs/decisions/` に 0120 は無い。この作業が 0121 を付けるのは PM の仮の割り当て）。

## 撤回条件

- **D1（画面 DC の属性）**: PM が画面 DC を再び属性なしに戻すと決めたとき（`need_attr_dc` を `need_mem_dc` に戻す）。実機の DC キャッシュの挙動がゲストの読み出しで観測され、`GetDC` ごとの状態という選択（S4）と食い違うとき。
- **D2、D3（寸法と LPtoDP）**: 実機 Windows の測定（Issue #1 の再生ツールの流儀で依頼できる）で HORZSIZE／VERTSIZE／LPtoDP の値が違うと分かったとき。ゲストが値に依存する箇所に届いたとき。
- **D5（クラス名）**: ゲストが別の形で `TSharedAnimTimerWindow` を扱うと分かったとき。
- **D6（メッセージ専用窓）**: 実機で message-only の窓の親・矩形・メッセージの順序が違うと分かったとき。
- **wall_fr5 の期待の直し**: 実機が PNG の MAINICON を別に扱い、`fle_res.c` が PNG を読むようになったとき（そのとき 256×256 のケースを選ぶ期待に戻す）。

## 限界

- 実機 Windows との照合は 0 件。wasm・ブラウザは未測定（native のみ）。
- 目的 (a)（`CreateFruityInstance` が戻る）にも (b)（ブロッキングのメッセージ待ち）にも**到達していない**。停止は壁 21（停止条件 (d) スレッド）。壁 21 の先は未測定。
- **メッセージキューを実装していない**（実行が届かなかった）。ブロッキング待ちの API に至る壁を、`CreateFruityInstance` を呼んだスレッドの側では 1 つも測っていない。
- 画面 DC の選択物・色・クリップ・画素は持たない。窓の実際の矩形・可視性・Z 順・フォーカス・アクティブ窓のモデルは持たない（決定 0119 の限界のまま）。
- LPtoDP の画面 DC の範囲は MM_LOMETRIC と MM_HIMETRIC だけ（ほかの固定モードは停止）。丸め（S3）は実機と一致するか未確認。
- 時間と RAM: 1 回の 6 モジュールのビルド（gen + build）が約 1:00、最大 RSS 約 2.7 GB。実行は 0.06 秒の呼び出し + 約 1 秒のハーネス。全体テストは約 46 分（決定 0119 の記録）。
