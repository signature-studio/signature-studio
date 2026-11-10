# 0119: 仮想ウィンドウ表の拡張（SendMessageW・SetClassLongPtrW・GetWindowLongPtrW・SetWindowTextW・システムメニュー・TApplication 形の CreateWindowExW ほか）と、`CreateFruityInstance` の壁 9〜17。壁 18（`SetMapMode(GetDC(NULL), MM_LOMETRIC)`）で停止。すべて仮の決定

状態: **仮の決定（すべての応答と選択）**。依頼は PM（2026-10-30。決定 0119 の内容は PM の仮の決定。番号 0119 は PM の仮の割り当て）。測定・実装・テスト・文書は作業者（サブエージェント）が行った。**作業は HEAD 9013ad5 の `git archive HEAD lift docs`（と、全体テストの準備として `git archive HEAD tools`）で始めた。差分 `fruity6.diff` は同じ HEAD の clean な archive に当たる。** コンパイル（`-Wall -Wextra -Werror`、警告・エラーなし）、全体テスト 1 回の通し（**120 test result・1,680 passed・1 failed**。失敗の 1 件は HEAD の clean archive でも再現する既存の失敗で、この差分とは無関係）、生成 C の関数単位の比較、変異検査 24 件（24 件検出。対照は通る）、FRUITY 無効の独立 2 回と FRUITY 有効の独立 2 回の測定まで実施した。**実機 Windows との照合は 0 件。wasm・ブラウザは未測定（native のみ）。**

**測定用の基底・種・註釈（dsp 0x2000000、Blend2D 0x4700000、QFC 0x1d00000、freetype 0x4b00000、ILTools 0x6000000、フォント 2 ファイル、レジストリの種、署名の主張の表）と、仮の応答を含む構成での値である。** 決定 0101 の表記では「A4 完走（仮の応答 N 件を含む）」の形の値で、達成の判定は PM が行う。

**肯定的に見せる点は少ない。** `CreateFruityInstance` は**戻らない**。メッセージループにも**入っていない**（目的 (a)、(b) のどちらにも到達していない）。壁 9〜17 の 9 壁を直し、**40,512 関数入口目で `gdi32!SetMapMode(GetDC(NULL), MM_LOMETRIC)`（壁 18）に止まる**（決定 0118 の 38,913 入口目から +1,599 入口）。壁 18 は停止条件 (i)（描画・GDI の大きなモデル）と (iii)（PM の判断が要る値）の両方に当たるので、ここで止めた。止めたとき、予算 12 壁のうち 10 壁目（9 壁を直し、10 壁目で停止）で、2 壁を残している。壁 18 の先の見通し（実験 X18）も、メッセージループではなく、スプラッシュ画面のフォームの作成（TBitmap／PNG、QFC の `AcquireSharedAnimTimer`）の途中である。

区分の凡例: **実測**＝コマンドの出力を見た事実。**読み**＝逆アセンブルやコードを読んだ上での解釈。**Learn**＝Microsoft Learn のページに書かれていること（取得日 2026-10-30。タグを除いたテキストを `learn/` に保存。ページごとの URL・取得時刻・HTML の SHA-256 は各ファイルの先頭 3 行）。**出典なしの選択**＝ページにも実測にも根拠が無く、作業者または PM が選んだ値。**未確認**。

## 先に結論

| 項目 | 内容 |
|---|---|
| `CreateFruityInstance`（種あり・主張の表あり） | **戻らない。ブロッキングのメッセージ待ちにも入っていない**。最後の停止点: `fle: SetMapMode: hdc is a DC of GetDC(NULL): its attributes are not modelled (only memory DCs are)`（`gdi32!SetMapMode` の thunk `f_4245c0`、`hdc = GetDC(NULL)`、`iMode = 3 = MM_LOMETRIC`）。**40,512 関数入口目**、新規の関数 485（決定 0118 の停止点 38,913／379 から +1,599 入口／+106 関数）、終了 134。独立な 2 回のビルド（別の OUT・別の cargo target）で同一（native `029bb30e4bb47d237cd0b38f8a404aaf4fd3540eba895778fb4890a49da65763`。`fruity:` の出力は経過時間の 1 行と PIE のポインタ値を除いて同一: `diff` で 0 行） |
| 直した壁 | **9 壁**（壁 9〜17。壁の通し番号は決定 0118 の続き）。壁 18 で停止（停止条件 (i)(iii)） |
| FRUITY 無効の新基準 | a4: 942/942、895/895、`returned from entry`、終了 0 は不変。**functions 5,308、entries 1,007,947 は不変**。native `27cdf764f10ffabc08bba2d4afb45815441cdb4e35c05221b102ce5908d23f6d` → **`49ce46e505089c118e4fbaef9b05b56db6b2fabdfbea8799016dc86a8d9f1d03`**（独立 2 回で同一。別の OUT・別の cargo target、fe-lift／fe-pe の SHA-256 同一）。SHA が変わった原因は**実行経路の変化ではなく runtime のオブジェクトの変化**（下の「FRUITY 無効の既定ビルドの native の SHA-256」） |
| 全体テスト | `cargo test --workspace --no-fail-fast --offline` を **1 回の通し**（46 分 12 秒、2026-10-30T09:23:33Z〜10:09:45Z、終了 101）で **120 test result・1,680 passed・1 failed・ignored 0**。基準は 119・1,647・0 failed。**失敗の 1 件は `wall_fr5::the_entry_is_chosen_by_the_rules_of_lookup_icon_id_from_directory_ex`**（`FLENGINE_PE` を設定すると 256×256 の RT_ICON 6 が PNG 圧縮で `fle_icon_group_pick` が名前付きで止まる。**HEAD の clean archive でも同じ結果**。この差分は `fle_res.c` を変えていない（`cmp` で同一）。「検証」の節） |
| 生成 C | FLEngine の生成 C（註釈あり）の SHA-256 `e7669cdc…ccbb7d`（HEAD。決定 0118 の値と一致）→ `b92c4b12b6bfb590f7aaaaa5ca05d4fce311d4e2addea3b2162300d9668df25d`。**関数 59,380 個のうち本体が変わったものは 8**（`f_424940` DeleteMenu、`f_424ed0` GetSystemMenu、`f_425380`・`f_425390`・`f_4253b0` = SendMessageW の 3 thunk、`f_425530` SetWindowTextW、`f_425970` GetWindowLongPtrW、`f_4259a0` SetClassLongPtrW）。**ILTools の生成 C でも 8 関数**（`f_601deb0` ほか。同じ 8 API の thunk）。dsp・Blend2D・QFC・freetype は 0。`fle_dispatch_count` 59,826 → 59,834、`fle_resp_count` 446 → 454、`fle_unsupported(UINT64_C(` の数 379 → 371 |
| wasm | runtime が変わるので wasm の SHA は変わる。再ビルドは別の作業（この作業では native のみ） |

## 背景

- 前段（決定 0118）: `CreateFruityInstance(0xC0, 0xEDE95A, 偽ホスト, &instance)` は 38,913 関数入口目の `user32!SendMessageW(NULL, WM_SETICON, ICON_BIG, hIcon)` で止まった（壁 9）。実験 E5 で NULL の hWnd に 0 と last error 1400 を答えると、次の壁 10 は `CreateWindowExW` の dwStyle 0x84ca0000（`TApplication`。決定 0071 の「隠れた WS_POPUP の道具ウィンドウ」の範囲外）だった。
- PM の仮の決定（0119、変えない）: (1) NULL の hWnd（窓表に無い hWnd）への `SendMessageW`・`SetClassLongPtrW`（と同系の `PostMessageW`/`GetWindowLongPtrW`/`SetWindowLongPtrW`）は 0 と last error 1400。出典なしの選択として列挙し、撤回条件は「実機 Windows の測定で値が違うと分かったとき」。(2) `IsProcessorFeaturePresent` を CPUID と食い違わない値に（PF 2・3 = 1）。(3) 決定 0071 のウィンドウ表を**必要最小限だけ**拡張し、`CreateWindowExW("TApplication", 0x84ca0000)` を決定 0071 の流儀で通し、その先の窓まわりの API を Learn の仕様から導ける範囲で実装する。メッセージキューは最初のブロッキング待ちに届くのに必要な最小の形まで。ブロッキング待ちそのものは実装しない。(4) 偽らない。創作値は出典なしの選択として列挙。(5) 止まる条件: (i) 描画・グリフ・音声・スレッド生成・ネットワーク・COM の大きなモデル、(ii) lift の穴で設計が大きいもの、(iii) PM の判断が要る値、(iv) (a) か (b) に到達、(v) 12 壁。
- 土台（実測。HEAD 9013ad5 の `git archive`）: 入力 DLL・フォントはすべて実在し SHA が一致（再抽出は不要）。FRUITY 無効の基準を独立 2 回で再現（TAG `t2331d00e8ebb`、942/942、895/895、functions 5,308、entries 1,007,947、native `27cdf764…3f6d`）。FRUITY 有効（種 = nsis + app、主張の表あり）で決定 0118 の停止（`unsupported instruction at 0x425390`、38,913 入口、新規の関数 379、native `48a81021…4259`）を再現した。

## PM の決定 1 の前提の確認（実測。Learn、取得 2026-10-30）

PM の決定 1 は「Learn のこれらの API のページは NULL／無効な hWnd の結果を明記していない」を前提にする。取り直した 5 ページで確かめた:

| API | 記述 | 前提 |
|---|---|---|
| `SendMessageW`（https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-sendmessagew） | 「hWnd: A handle to the window whose window procedure will receive the message. If this parameter is HWND_BROADCAST ((HWND)0xffff), …」。NULL・無効の結果は書かれていない | 成り立つ |
| `SetClassLongPtrW`（…/nf-winuser-setclasslongptrw） | 「If the function fails, the return value is zero. To get extended error information, call GetLastError」。NULL・無効は書かれていない | 成り立つ |
| `GetWindowLongPtrW`、`SetWindowLongPtrW`（…/nf-winuser-getwindowlongptrw、…setwindowlongptrw） | 同上（失敗は 0 と GetLastError） | 成り立つ |
| **`PostMessageW`**（…/nf-winuser-postmessagew） | 「[in, optional] hWnd … **NULL: The function behaves like a call to PostThreadMessage with the dwThreadId parameter set to the identifier of the current thread.**」 | **成り立たない**（NULL の結果が明記されている） |
| System Error Codes 1300–1699（…/debug/system-error-codes--1300-1699-） | 「ERROR_INVALID_WINDOW_HANDLE 1400 (0x578) Invalid window handle.」（どの API が返すかは書かれていない） | 定義は確認 |

扱い: 決定 1 は `SendMessageW`・`SetClassLongPtrW`・`GetWindowLongPtrW`・`SetWindowLongPtrW`（と、同系として `SetWindowTextW`）の「窓表に無い hWnd」（NULL、未発行、作成に失敗した窓）に適用した。**`PostMessageW` は実装していない**（この作業の実行が届かなかった。実装するときは、NULL を 1400 にせず Learn どおりに現スレッドのスレッドメッセージとして扱う必要がある。PM の決定からの逸脱になる。「PM の判断が要る点」）。

## 決定（仮）

### 旧 → 新の表（既存の応答・既存のテストの期待値を変えるもの）

| # | 場所 | 旧 | 新 | 根拠 |
|---|---|---|---|---|
| D1 | `runtime/fle_win.c` の `CreateWindowExW` | dwStyle は `WS_POPUP`（0x80000000）だけ。位置・サイズは (0, 0, 0, 0) だけ。窓名は空だけ | **第 2 の形を足す**: dwStyle 0x84ca0000（WS_POPUP \| WS_CLIPSIBLINGS \| WS_CAPTION \| WS_SYSMENU \| WS_MINIMIZEBOX）、位置は CW_USEDEFAULT 以外の任意、サイズ 0×0、窓名は 256 ユニット以下。第 1 の形（決定 0071）は不変。WM_NCCALCSIZE の RECT は**提案矩形** (x, y, x+cx, y+cy)（旧: 常に (0, 0, 0, 0)。第 1 の形では x = y = cx = cy = 0 なので値は同じ） | Learn Window Styles の値の和。WM_NCCALCSIZE「On entry, the structure contains the proposed window rectangle」。「WS_CAPTION で WS_THICKFRAME の無い窓は作成中に WM_GETMINMAXINFO を受けない」（Window Features: 「A window with the WS_THICKFRAME style receives this message during the window-creation process」） |
| D2 | `fle_win.c` の窓プロシージャの呼び出し | `fle_call_guest` を直接 3 か所 | **`call_wndproc`** 1 つに集約（`CreateWindowExW` の 3 メッセージ、`SendMessageW`、`SetWindowTextW`）。窓プロシージャが VirtualAlloc の領域の中のアドレスのときだけ、Delphi の `MakeObjectInstance` のスロットの形（`E8 rel32` → `41 5B FF 25 00 00 00 00`）を厳密に確かめて、`r11 = slot + 5` で、テンプレートの直後の 8 バイトの関数入口を呼ぶ。形が違えば名前付き停止。領域の外のアドレスは従来どおり `fle_call_guest` | 決定 0071 の調査 §3.3 が予告した形（「スロットを解釈するか、別の決定」）。ゲストのバイト列を命令どおりに解釈するだけで、結果を創作しない |
| D3 | `SetWindowLongPtrW`（決定 0071） | GWLP_WNDPROC 以外の index はすべて名前付き停止。窓表に無い hWnd は名前付き停止 | **GWL_EXSTYLE（-20）と GWLP_USERDATA（-21）も答える**（前の値を返す）。GWLP_WNDPROC の分岐は不変。**窓表に無い hWnd は 0 と last error 1400**（PM の決定 1） | Learn SetWindowLongPtrW「GWL_EXSTYLE -20 Sets a new extended window style」「the previous value of the specified offset」「GWLP_USERDATA … Its value is initially zero」。ゲストが TApplication の窓に GWL_EXSTYLE を書く（壁 17） |
| D4 | `DefWindowProcW`（決定 0071） | WM_NCCREATE、WM_NCCALCSIZE（wParam FALSE）、WM_CREATE だけ。他は名前付き停止 | **WM_SETICON（0x80）と WM_SETTEXT（0x0C）を足す**。他は停止のまま。窓表に無い hWnd は停止のまま（決定 1 の列挙に無い） | Learn WM_SETICON「The DefWindowProc function returns a handle to the previous large or small icon」、WM_SETTEXT「The DefWindowProc function sets and displays the window text」 |
| D5 | `fle_win.c` の `fle_set_last_error` | 使わない（テストが語を禁じていた） | **1 か所だけ使う**（`invalid_window()`: 0 と 1400） | PM の決定 1。出典なしの選択（下の一覧 S1） |
| D6 | `runtime/fle_sysinfo.c` の `IsProcessorFeaturePresent` | PF 2（PF_COMPARE_EXCHANGE_DOUBLE）= 0、PF 3（PF_MMX_INSTRUCTIONS_AVAILABLE）= 0（CPUID は CX8・MMX を報告するのに 0） | **PF 2 = 1、PF 3 = 1**。他は不変（SSE・SSE2・PAE・FASTFAIL = 1、他の列挙済みは 0、未列挙は停止） | PM の決定 2。Learn の表（「PF_COMPARE_EXCHANGE_DOUBLE 2: The atomic compare and exchange operation (cmpxchg) is available」「PF_MMX_INSTRUCTIONS_AVAILABLE 3: The MMX instruction set is available」）。**PF 2 と CPUID EDX bit 8（CMPXCHG8B）の対応は Learn に書かれていない**（読み。「解釈が分かれた点」） |
| D7 | `fle_load.c` の `fle_load_absent[]` | 7 名（dwmapi、imm32、msimg32、propsys、uxtheme、windowscodecs、wtsapi32） | **5 名**（wtsapi32.dll と uxtheme.dll を外す）。`fle_modules[]` の末尾に wtsapi32.dll（N = 37）、uxtheme.dll（N = 38）を足す（決定 0099 の shcore、0118 の rpcrt4 と同じ形） | 遅延ロードの `LoadLibraryA` が NULL → `RaiseException(0xc06d007e)` になる壁（壁 11、12） |
| D8 | 既存テストの期待値（`window.rs`、`class.rs`、`cursor.rs`、`gdi_bulk.rs`、`k32_dsp.rs`、`modules.rs`、`load.rs`、`deferred.rs`、`msctf.rs`、`wall_s.rs`、`wall_fr4.rs`、`wall_fr5.rs`、`vcall_thunk_dll.rs`） | 隣接 API を「答えない」と固定、モジュール数 36、表の末尾の次の位置 36、PF 2・3 = 0、未対応 thunk 379 ほか | 上の D1〜D7 に合わせて更新（旧→新と理由は各所のコメントと「実装した場所」） | 期待値の更新漏れ |

### 新しい応答（これまで停止または未応答だったもの）

| # | 場所 | 内容 |
|---|---|---|
| D9 | `fle_win.c`、応答表の末尾 | `user32!SendMessageW`: 窓表の live な窓に `call_wndproc` で送り、LRESULT の全 64 ビットを返す（Learn「If the specified window was created by the calling thread, the window procedure is called immediately as a subroutine」）。HWND_BROADCAST（0xFFFF。ページが定義）は未実装（名前付き停止）。窓表に無い hWnd は 0 と 1400 |
| D10 | `fle_win.c`、`fle_class.c`／`.h`（`fle_class_set_hicon`）、応答表 | `user32!SetClassLongPtrW`: GCLP_HICON（-14）だけ。窓のクラス（アトム）の WNDCLASSW の hIcon（オフセット 32）を置き換え、前の値を返す。`GetClassInfoW` が新しい値を返す。他の index（GCLP_HCURSOR、GCLP_WNDPROC、GCL_STYLE、…、extra memory）は名前付き停止。窓表に無い hWnd は 0 と 1400 |
| D11 | `fle_win.c`、応答表 | `wtsapi32!WTSRegisterSessionNotification`: 窓表の窓に「登録済み」と最初の dwFlags（0 か 1）を記録して TRUE（Learn「If the window handle passed in this function is already registered, the value of the dwFlags parameter is ignored」）。WM_WTSSESSION_CHANGE は決して送らない。窓表に無い hWnd、dwFlags が 0・1 以外は停止 |
| D12 | `fle_win.c`、応答表 | `uxtheme!BufferedPaintInit`: S_OK を返し、呼び出し回数を数える。`BeginBufferedPaint`／`EndBufferedPaint`／`BufferedPaintUnInit`／`BufferedPaintSetAlpha` には行を足さない（呼ばれれば `GetProcAddress` が NULL と 127 → 遅延ロードの `0xc06d007f` で名前付き停止） |
| D13 | `fle_win.c`、応答表 | `user32!GetSystemMenu`／`DeleteMenu`: 窓ごとに 1 つのシステムメニュー（HMENU = `0x7F980000 + 4 × (窓の添字 + 1)`、領域 64 KiB）。作成時の項目は 6 つ（SC_SIZE 0xF000、SC_MOVE 0xF010、SC_MINIMIZE 0xF020、SC_MAXIMIZE 0xF030、SC_CLOSE 0xF060、SC_RESTORE 0xF120）。`DeleteMenu(…, MF_BYCOMMAND)` は項目があれば消して TRUE、無ければ停止。`GetSystemMenu(hwnd, TRUE)` は 6 項目に戻して NULL。WS_SYSMENU の無い窓、MF_BYPOSITION ほかは停止。描画しない、WM_INITMENU／WM_SYSCOMMAND は送らない |
| D14 | `fle_win.c`、応答表 | `user32!GetWindowLongPtrW`: 窓表の live な窓で GWL_EXSTYLE、GWL_STYLE（32 ビットを符号拡張）、GWLP_HINSTANCE、GWLP_HWNDPARENT（親なし = 0）、GWLP_USERDATA（初期値 0）、GWLP_WNDPROC。他の index は停止。窓表に無い hWnd は 0 と 1400。`user32!SetWindowTextW`: 窓表の窓に WM_SETTEXT（wParam 0、lParam lpString）を `call_wndproc` で送り、その結果を戻り値にする。NULL は空文字列。256 ユニット超は停止。窓表に無い hWnd は 0 と 1400 |

## 壁の表

各壁の記録は `wall-log.md`。「種類」は 未応答 import／名前付き停止／lift の穴／環境の主張／大きなモデル／メッセージループ。入口数は `fruity:` の「lifted entries since the call began」（FRUITY 有効の最終ビルドの途中経過。`logs/w9`〜`w17`、`fr0`）。

| # | 停止メッセージ | 原因（区分） | 種類 | 修正（場所） | 測定（入口数／新規の関数） |
|---|---|---|---|---|---|
| 9 | `fle: unsupported instruction at 0x425390`（`user32.dll!SendMessageW`） | `f_86b680` が `SendMessageW([self+0x2b0] = NULL, WM_SETICON, ICON_BIG, hIcon)`、続けて `SetClassLongPtrW(NULL, GCLP_HICON, hIcon)`。窓がまだ無い（読み + 実測） | 環境の主張・出典なしの選択（PM の決定 1） | D5、D9、D10 | 38,913／379 → **38,928／384**（次の停止: `CreateWindowExW` の dwStyle 0x84ca0000） |
| 10 | `fle: CreateWindowExW: dwStyle 0x84ca0000 is not WS_POPUP (0x80000000): only the hidden tool window of the guest is answered (decision 0071)` | `f_867e10`（`TApplication` のハンドル作成と読む）が `CreateWindowExW(WS_EX_TOOLWINDOW, "TApplication", "FL Studio", 0x84ca0000, 960, 540, 0, 0, NULL, NULL, 0x400000, NULL)`（X・Y = `GetSystemMetrics(SM_CXSCREEN / SM_CYSCREEN) / 2`。実測 + 読み） | 窓表の拡張（PM の決定 3） | D1 | 38,928／384 → **38,937／387**（次の停止: `RaiseException 0xc06d007e`） |
| 11 | `fle: RaiseException: exception code 0xc06d007e`（`LoadLibraryA("wtsapi32.dll")` が NULL） | `f_867e10` が Windows のバージョンが 5 より上のとき `WTSRegisterSessionNotification(hwnd, 0)` を遅延ロードで呼ぶ。`wtsapi32.dll` が `fle_load_absent[]` にあった（読み + 実測） | 未応答 import（遅延ロード） | D7、D11 | 38,937／387 → **38,943／388**（次の停止: 同じ種類、`uxtheme.dll`） |
| 12 | `fle: RaiseException: exception code 0xc06d007e`（`LoadLibraryA("uxtheme.dll")` が NULL） | `f_867e10` が `BufferedPaintInit()` を遅延ロードで呼ぶ（呼び元はこの 1 か所だけ。静的な `call` の全数走査）。`uxtheme.dll` が absent だった | 未応答 import（遅延ロード） | D7、D12 | 38,943／388 → **38,948／386**（次の停止: `fle_dispatch_miss`） |
| 13 | `fle: indirect call to 0x7521b3350f84 is not a known function entry` | `SetWindowLongPtrW(hwnd, GWLP_WNDPROC, slot)` で保存した窓プロシージャは Delphi の `MakeObjectInstance`（`f_52a5b0`）が VirtualAlloc した RWX ブロックの中のスロット。`SendMessageW(hwnd, WM_SETICON, …)` の `fle_call_guest` が「関数入口でない」と止めた | lift の穴に近い（実行時に書かれたマシンコードの呼び出し。停止条件 (ii) を検討して局所に収めた） | D2 | 38,948／386 → **38,953／390**（次の停止: `DefWindowProcW` の WM_SETICON） |
| 14 | `fle: DefWindowProcW: message 0x80: the default processing is not answered (decision 0071 answers WM_NCCREATE 0x81, WM_NCCALCSIZE 0x83 with wParam FALSE and WM_CREATE 0x01 only)` | `TApplication.WndProc`（`f_868c00`）が WM_SETICON を既定の処理へ転送（`f_868b40` → `DefWindowProcW`） | 名前付き停止（窓表の拡張） | D4（WM_SETICON） | 38,953／390 → **38,957／390**（次の停止: `SetClassLongPtrW` の live な窓） |
| 15 | `fle: SetClassLongPtrW: a live window of the table: the class memory (GCLP_*) is not modelled (decision 0119)` | `f_867e10` の 0x868065: `SetClassLongPtrW(hwnd, GCLP_HICON, hIcon)` | 名前付き停止（窓表の拡張） | D10（`fle_class_set_hicon`） | 38,957／390 → **38,958／391**（次の停止: `GetSystemMenu`） |
| 16 | `fle: unsupported instruction at 0x424ed0`（`user32.dll!GetSystemMenu`） | `f_867e10` が `GetSystemMenu(hwnd, 0)` → `DeleteMenu(hMenu, SC_MAXIMIZE 0xf030 / SC_SIZE 0xf000 / SC_MOVE 0xf010, MF_BYCOMMAND)` | 未応答 import（小さなメニュー表） | D13 | 38,958／391 → **38,964／394**（次の停止: `GetWindowLongPtrW`） |
| 17 | `fle: unsupported instruction at 0x425970`（`user32.dll!GetWindowLongPtrW`） | `f_86ccd0`（`TApplication` の可視／不可視の切り替えと読む）が `GetWindowLongPtrW(hwnd, GWL_EXSTYLE)` → `SetWindowLongPtrW(hwnd, GWL_EXSTYLE, ex \| 0x80)` → `SetWindowTextW(hwnd, NULL)` | 未応答 import | D3、D4（WM_SETTEXT）、D14 | 38,964／394 → **40,512／485**（次の停止: 壁 18。+1,548 入口／+91 関数。VCL の初期化を通る） |
| 18 | `fle: SetMapMode: hdc is a DC of GetDC(NULL): its attributes are not modelled (only memory DCs are)` | `f_6f6410`（論理 0.1 mm の点をデバイス単位へ。`GetDC(NULL)` → `SetMapMode(hdc, MM_LOMETRIC)` → `LPtoDP` → `ReleaseDC`）。`f_13751e0` → `f_11b3810` → `f_13734d0` → `f_136cba0`（スプラッシュ画面の作成と読む）→ … の経路 | **大きなモデル・PM の判断が要る値（停止条件 (i)(iii)）** | なし（**ここで止めた**） | 40,512／485 で停止 |

壁の種類の件数（この作業の壁 9〜18。各壁を主な種類 1 つに数える）: 環境の主張・出典なしの選択 1（壁 9）、窓表の拡張・名前付き停止 3（壁 10、14、15）、未応答 import 4（壁 11、12、16、17）、lift の穴に近いもの 1（壁 13）、大きなモデル・PM の判断 1（壁 18）、メッセージループ 0。計 10。決定 0116〜0118 の壁 1〜8 と合わせて壁 18 まで。数え方: 壁 9〜17 の 9 壁を直し、壁 18 が 10 壁目で停止（上限 12 壁のうち 2 壁を残した）。

## 止まった壁（壁 18）の内容・選択肢・推奨

**実測**（`logs/stop18/`、`logs/fin1/`）: 停止点は `gdi32!SetMapMode`（40,512 入口）。引数は `hdc = 0x7fa9013c`（`GetDC(NULL)` が返した仮想画面 DC。`FLE_GDC_HANDLE_BASE` の領域）、`iMode = 3`（MM_LOMETRIC。`mov edx, 3` は 0x6f642d）。シャドウスタック: `f_4245c0`（SetMapMode の thunk）← `f_6f6410` ← `f_6f6570` ← `f_6ba930` ← `f_6baae0` ← `f_6ce880` ← `f_6cc320` ← `f_6cc4d0` ← `f_136cba0` ← `f_13734d0` ← `f_11b3810` ← `f_13751e0` ← `CreateFruityInstance`。C スタック（`logs/stop18/c-stack-symbolised.txt`）: `fle_stop+0x35` ← `need_mem_dc+0x60` ← `fle_imp_SetMapMode+0x43` ← `f_4245c0+0x19` ← `f_6f6410+0x229` ← `f_6f6570+0x189` ← `fle_dispatch+0x128` ← `f_6ba930+0x36e` ← `f_6baae0+0x531` ← `f_6ce880+0x361` ← `fle_dispatch+0x128` ← `f_6cc320+0x3ee` ← `f_6cc4d0+0x7e0` ← `fle_dispatch+0x128` ← `f_136cba0+0xe22` ← `f_13734d0+0x80` ← `fle_dispatch+0x128` ← `f_11b3810+0x475` ← `f_13751e0+0x238` ← `f_1375990（CreateFruityInstance）+0x57af` ← `fle_dispatch+0x110` ← `fle_call_guest+0x169` ← `fruity_run+0x535` ← `main+0x1b3b`。

**読み**: `f_13751e0` は `[0x15e0638]->[+0xc]` のフラグの bit 2 が 0 なら `f_11b3810` を呼ぶ。`f_11b3810` はレジストリ `General\MIDIForm`／`SplashBox` を読み、条件が揃えば `f_13734d0` → `f_136cba0` を呼ぶ。`f_136cba0` は **スプラッシュ画面の作成**（`[0x196ee28]` = スプラッシュのフォームを `TForm` のコンストラクタ `f_7222a0` で作り、`General\SplashImage` が空なら内蔵の `Title.png`、あればファイルから画像を読む。`f_6ba1f0` で `TBitmap` を作る）。`f_6f6410` は `hdc = GetDC(NULL); SetMapMode(hdc, MM_LOMETRIC); pt.y = -pt.y; LPtoDP(hdc, &pt, 1); ReleaseDC(NULL, hdc)` で、論理 0.1 mm の点をピクセルに換算する（TBitmap／TPicture の DPI からピクセルサイズへの換算と読む）。

**分類**: 停止条件 **(iii) PM の判断が要る値**（画面 DC の `LPtoDP` は MM_LOMETRIC の規則（Learn Mapping Modes: 「Each unit in page space is mapped to 0.1 millimeter in device space ... y increases from bottom to top」）に、画面の物理寸法（HORZSIZE／VERTSIZE。決定 0053 は「値の根拠がない」として停止のままにした）か「96 DPI からの逆算」を要る）と **(i) 描画・GDI の大きなモデル**（決定 0108/0111 は画面 DC の属性を「メモリ DC だけ」に区切った。壁 18 の先は、スプラッシュ画面のフォームの作成の途中で、PNG の読み込み・TBitmap・GDI の描画経路の入口）に当たる。

**選択肢**

| 案 | 内容 | 結果（実測／読み） | 注意 |
|---|---|---|---|
| Y1. 止まる（推奨。この作業の選択） | 何も足さない | 40,512／485 で停止 | 到達点は動かない |
| Y2. 画面 DC に `SetMapMode` の保存と、MM_LOMETRIC の `LPtoDP`（96 DPI からの逆算 = 論理 1 単位 = 96 / 254 ピクセル、y 反転）を答える | **実験 X18**（`experiment-x18.diff`。差分に入れない。`fle_gdi.c` の 2 関数） | 壁 18 を越え、次の停止は `GetClassInfoW(hInstance = 0x1d00000, "TSharedAnimTimerWindow")`（QFC の `AcquireSharedAnimTimer`、窓クラスの登録、スプラッシュのフォームの作成の中）。**41,150／559** | 物理寸法を 96 DPI から逆算するのは**出典なしの選択**（決定 0053 の PM の判断）。メッセージループには近づかない（フォームの作成の途中） |
| Y3. スプラッシュの経路を避ける（`General\SplashImage` や `MIDIForm\SplashBox` の種で `f_136cba0` を呼ばない） | 未測定。`f_11b3810` の条件（レジストリ `General\MIDIForm`／`SplashBox`）と、`[0x15e0638]->[+0xc]` のフラグの bit 2 で決まる（読み） | 未測定。**どの値がエンジンの通常の動作かは未確認** | 環境の主張（レジストリの種）であり、偽の成功ではない。ただし「ユーザーが SplashBox を切った構成」を選ぶことになる。PM の判断 |

**推奨**: Y1（止まる）か Y3（レジストリの種の判断を PM が決める）。理由: Y2 は物理寸法という出典なしの値を足すうえ、次の壁（QFC の窓クラスの登録、そのモジュールの窓表への組み込み）も描画前の準備段階で、(a)(b) に近づく見込みを示せない（X18 は 41,150 入口で止まる）。Y3 は PM が構成（スプラッシュの有無）を決める必要があり、この作業者は決められない。

## ウィンドウモデルの範囲

**実装したもの**（決定 0071 の仮想ウィンドウ表の拡張。ホストの実ウィンドウ・実メッセージキュー・時計・ディスプレイを読まない。単一スレッド）:

- 窓表の項目に、作成時の矩形・窓名、WM_SETICON のアイコン、GWLP_USERDATA、WTS の登録、システムメニューの項目を足した。
- `CreateWindowExW` の第 2 の形（TApplication、0x84ca0000）、`SendMessageW`、`SetWindowTextW`、`GetWindowLongPtrW`／`SetWindowLongPtrW`（GWL_EXSTYLE、GWLP_USERDATA）、`SetClassLongPtrW`（GCLP_HICON）、`GetSystemMenu`／`DeleteMenu`、`DefWindowProcW`（WM_SETICON、WM_SETTEXT）、`WTSRegisterSessionNotification`、`BufferedPaintInit`。
- 窓プロシージャの呼び出し点の一本化（`call_wndproc`）と、Delphi の `MakeObjectInstance` のスロットの解釈。
- `wtsapi32.dll`（N = 37）と `uxtheme.dll`（N = 38）の仮想モジュール化。

**実装していないもの**（呼ばれれば名前付き停止、または未実装のまま）:

- **メッセージキュー**: `PostMessageW`、`GetMessageW`、`PeekMessageW`、`WaitMessage`、`MsgWaitForMultipleObjects`、`DispatchMessageW`、`TranslateMessage`、`PostQuitMessage` は実装していない（`tests/wall_fr6.rs` のソース検査が `fle_win.c` にこれらの語が無いことを固定）。**この作業の実行はこれらに届かなかった**ので、メッセージキューの FIFO・空のときの扱いは設計も実装もしていない。
- 窓の可視性・位置・大きさの API: `ShowWindow`、`SetWindowPos`、`MoveWindow`、`GetWindowRect`、`GetClientRect`、`AdjustWindowRectEx`、`IsWindowVisible`、`UpdateWindow`（Learn のページは取得したが、ゲストの実行が届かなかったので実装していない）。
- `WM_GETMINMAXINFO`、`WM_SIZE`、`WM_MOVE`、`WM_SHOWWINDOW`、`WM_WINDOWPOS*`、`WM_GETICON`、`WM_GETTEXT`、`WM_SYSCOMMAND`、`WM_INITMENU` の送出と既定の処理。
- `HWND_BROADCAST`、親子窓、オーナー窓、メッセージ専用窓、メニューバー、`SetParent`、`DestroyWindow`、`SetTimer`、`KillTimer`。
- 描画（`BeginPaint`、`EndPaint`、`BeginBufferedPaint`、`EndBufferedPaint`）、画面 DC の属性（壁 18）。
- 窓の実際の矩形の計算（枠・キャプションの NC 領域の寸法）: `WM_NCCALCSIZE` の RECT は提案矩形のまま返し、クライアント矩形を計算しない。作成時の矩形をそのまま保存する（実機が 0×0 を最小トラッキングサイズにクランプするかは Learn に書かれていない = 未確認）。

## 実装した場所

- 変更: `lift/runtime/fle_win.c`（+421／-15 行）、`fle_win.h`（+76／-1）、`fle_class.c`（`fle_class_set_hicon`、+18）、`fle_class.h`（+5）、`fle_modules.c`（N = 37、38、+12）、`fle_load.c`（absent の一覧から 2 名を除く）、`fle_sysinfo.c`（PF 2・3、コメント）、`lift/crates/fe-lift/src/imports_table.rs`（末尾に 8 行）。runtime に新しい `.c`／`.h` は作っていない。
- 新規: `lift/crates/fe-lift/tests/wall_fr6.rs`（33 件）、`docs/decisions/0119-window-model-and-fruity-walls.md`。
- 既存関数の本体で変えたもの（指示により理由を記す）: `fle_imp_SetWindowLongPtrW`（D3。GWLP_WNDPROC の分岐の戻り値・保存は不変。窓表に無い hWnd の扱いと、GWL_EXSTYLE・GWLP_USERDATA を足した）、`fle_imp_DefWindowProcW`（D4。NCCREATE、CREATE、NCCALCSIZE の分岐は不変）、`fle_imp_CreateWindowExW`（D1、D2。第 1 の形の検査・メッセージの順序・LRESULT の扱いは不変。第 2 の形と `call_wndproc` への置き換え）、`fle_imp_IsProcessorFeaturePresent`（D6。PF 2・3）、`fle_load.c` の absent の一覧（D7）、`fle_win_view` の項目の追加（検査用）。それ以外の既存の関数は変えていない。
- 応答表（`imports_table.rs`）は**末尾追記のみ**。追記した 8 行（決定 0015: 擬似 VA は並びで決まる）: `user32!SendMessageW`、`user32!SetClassLongPtrW`、`wtsapi32!WTSRegisterSessionNotification`、`uxtheme!BufferedPaintInit`、`user32!GetSystemMenu`、`user32!DeleteMenu`、`user32!GetWindowLongPtrW`、`user32!SetWindowTextW`。`IsProcessorFeaturePresent` には行を足していない（決定 0078 と `tests/sysinfo.rs` が「GetProcAddress が見つけない」ことを固定している）。
- 既存のテストの更新（期待値の更新漏れ。理由を記す）:
  - 隣接 API を「答えない」と固定していたもの（D9〜D14 の API を一覧から外す。旧: 表に無いと表明 → 新: `wall_fr6.rs` が表に 1 回あることを表明）: `window.rs`、`class.rs`（`SetClassLongPtrW`）、`cursor.rs`（`SetClassLongPtrW`）、`gdi_bulk.rs`（`SendMessageW`）。
  - `window.rs` の個別の更新: `SetWindowLongPtrW` が GWL_EXSTYLE・GWLP_USERDATA を答える、窓表に無い hWnd（10 種と、作成に失敗した窓）に 0 と 1400、`DefWindowProcW` の「他のメッセージ」から 0x80 を外し `WM_GETICON`（0x7F）を足す、`fle_win.c` が `fle_set_last_error` を使わないという検査を「1 か所だけ（定数 1400）」に、`fle_call_guest` の呼び出し箇所の数 3 → 2（`call_wndproc` に集約）、メニュー領域を領域の重なり検査に足す。
  - PF 2・3（D6）: `k32_dsp.rs`（`is_processor_feature_present_agrees_with_the_cpuid_answer` の期待値 0 → 1）。
  - 仮想モジュール表が 36 → 38（D7）: `modules.rs`、`load.rs`、`deferred.rs`、`msctf.rs`、`wall_s.rs`、`wall_fr5.rs`（「表の末尾の次の位置」36 → 38、`wtsapi32`／`uxtheme` を「無い名前」の例から外し、`the_seven_lacking_names…` → `the_five_lacking_names…`）。
  - 応答表の末尾からの位置（後ろに行が足された）: `wall_fr4.rs`（後ろの行の数 1 → 9）、`wall_fr5.rs`（1 → 8 の LATER_ROWS を足す）。
  - `vcall_thunk_dll.rs`: `unsupported_total` 379 → 371（理由: 答えを持つ thunk が 8 つ増えた。生成 C の関数単位の比較で、本体が変わったのは `f_424940`、`f_424ed0`、`f_425380`、`f_425390`、`f_4253b0`、`f_425530`、`f_425970`、`f_4259a0` の 8 つと実測）。
- 既定の挙動: FRUITY 無効の既定ビルドの挙動は、`IsProcessorFeaturePresent`（D6。dsp_ippv2 が問うのは PF 23 だけで、FLEngine は import していない（実測））、`wtsapi32.dll`／`uxtheme.dll` の仮想モジュール化（FRUITY 無効の a4 はこれらを読み込まない）、窓表の拡張を除いて変わらない（実測: a4 942/942、895/895、`returned from entry`、終了 0、functions 5,308、entries 1,007,947）。

## FRUITY 無効の既定ビルドの native の SHA-256（関数単位の理由）

基準 `27cdf764f10ffabc08bba2d4afb45815441cdb4e35c05221b102ce5908d23f6d`（HEAD 9013ad5、FRUITY 無効）→ **`49ce46e505089c118e4fbaef9b05b56db6b2fabdfbea8799016dc86a8d9f1d03`**（この差分、FRUITY 無効）。a4 の数字は不変（942/942、895/895、functions 5,308、entries 1,007,947、`returned from entry`、終了 0）。理由（実測）:

1. **FLEngine の生成 C**（SHA-256 `e7669cdc…ccbb7d` → `b92c4b12…df25d`）: 関数 59,380 個のうち**本体が変わったものは 8**（上の 8 thunk）。5 モジュールの生成 C は m5（ILTools）で 8、m1〜m4 は 0（`logs/gencmp-functions.txt`）。行の追加 8 行ずつ（宣言・dispatch 表・応答表）と個数の定数（`fle_dispatch_count` 59,826 → 59,834、`fle_resp_count` 446 → 454）。**FRUITY 無効の a4 の経路に、変わった 8 関数は入らない**（実測: functions／entries が不変）。
2. **runtime のオブジェクト**: 50 個（`fle_*.c` 全部 + `image`、`a4`、`main_two`（`-DFLE_FRUITY` なし）、`trace_native`）を 2 つの木で同じフラグでコンパイルして比べると、違うのは **`fle_class.o`、`fle_load.o`、`fle_modules.o`、`fle_sysinfo.o`、`fle_win.o` の 5 つ**だけ（`logs/gencmp-runtime-objects.txt`）。
3. `common.c`: 応答表と dispatch 表に 8 行ずつ。
4. **PM の決定 2 の確認**: PF 2・3 を変えた単独の測定（`logs/d2off1`、他の変更を入れた木）で functions 5,308、entries 1,007,947 が不変、native SHA だけが変わった。FLEngine は `IsProcessorFeaturePresent` を import していない（pefile で通常 import と遅延 import のどちらにも無い）。したがって **entries／functions が動いた場合の「経路の変化」は無く、SHA の変化は `fle_sysinfo.o` などのオブジェクトの変化として説明できる**。

## Learn／一次資料の取得記録（2026-10-30。取得は `urllib` で、タグを除いたテキストを `learn/` に保存し、成果物に入れる（420 KB）。各ファイルの先頭 3 行に URL・取得時刻・HTML の SHA-256）

| ページ | URL | 使った箇所 |
|---|---|---|
| SendMessageW | https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-sendmessagew | D9。「The return value specifies the result of the message processing; it depends on the message sent」「If the specified window was created by the calling thread, the window procedure is called immediately as a subroutine」。NULL・無効の結果は書かれていない |
| PostMessageW | …/nf-winuser-postmessagew | 「NULL: The function behaves like a call to PostThreadMessage」。PM の決定 1 の前提が成り立たない（実装していない） |
| SetClassLongPtrW | …/nf-winuser-setclasslongptrw | D10。「GCLP_HICON -14 Replaces a handle to the icon associated with the class」「the return value is the previous value of the specified offset. If this was not previously set, the return value is zero」 |
| GetWindowLongPtrW、SetWindowLongPtrW | …/nf-winuser-getwindowlongptrw、…setwindowlongptrw | D3、D14。GWL_EXSTYLE -20、GWL_STYLE -16、GWLP_HINSTANCE -6、GWLP_HWNDPARENT -8、GWLP_USERDATA -21「Its value is initially zero」、GWLP_WNDPROC -4 |
| SetWindowTextW、WM_SETTEXT | …/nf-winuser-setwindowtextw、…/winmsg/wm-settext | D4、D14。「If the target window is owned by the current process, SetWindowText causes a WM_SETTEXT message to be sent」「The DefWindowProc function sets and displays the window text」「The return value is TRUE if the text is set」 |
| WM_SETICON | …/winmsg/wm-seticon | D4。ICON_BIG 1／ICON_SMALL 0、「The return value is a handle to the previous large or small icon … NULL if the window previously had no icon」 |
| CreateWindowExW、Window Styles、Window Features、WM_NCCALCSIZE、WM_GETMINMAXINFO | …/nf-winuser-createwindowexw、…/winmsg/window-styles、…window-features、…wm-nccalcsize、…wm-getminmaxinfo | D1。dwStyle の値の和、「A window with the WS_THICKFRAME style receives this message during the window-creation process」、「On entry, the structure contains the proposed window rectangle」 |
| GetSystemMenu、DeleteMenu、About Menus、WM_SYSCOMMAND | …/nf-winuser-getsystemmenu、…deletemenu、…/menurc/about-menus、…wm-syscommand | D13。「The copy is initially identical to the window menu, but it can be modified」「The window menu initially contains items with various identifier values, such as SC_CLOSE, SC_MOVE, and SC_SIZE」（**完全な一覧は書かれていない**）、SC_ の値（SC_SIZE 0xF000 ほか） |
| WTSRegisterSessionNotification | …/api/wtsapi32/nf-wtsapi32-wtsregistersessionnotification | D11。「If the window handle passed in this function is already registered, the value of the dwFlags parameter is ignored」。NOTIFY_FOR_THIS_SESSION／NOTIFY_FOR_ALL_SESSIONS の**数値は書かれていない** |
| BufferedPaintInit | …/api/uxtheme/nf-uxtheme-bufferedpaintinit | D12。「If this function succeeds, it returns S_OK. Otherwise, it returns an HRESULT error code」「An application may call this API multiple times, as long as each call to BufferedPaintInit is balanced with a call to BufferedPaintUnInit」 |
| IsProcessorFeaturePresent | …/api/processthreadsapi/nf-processthreadsapi-isprocessorfeaturepresent | D6。PF 2、3、6、9、10、14、23 の行 |
| System Error Codes（1300–1699） | …/debug/system-error-codes--1300-1699- | 「ERROR_INVALID_WINDOW_HANDLE 1400 (0x578) Invalid window handle.」 |
| Mapping Modes、LPtoDP、GetDeviceCaps | …/gdi/mapping-modes-and-translations、…nf-wingdi-lptodp、…nf-wingdi-getdevicecaps | 壁 18 の選択肢 Y2。MM_LOMETRIC「Each unit in page space is mapped to 0.1 millimeter in device space」、HORZSIZE「Width, in millimeters, of the physical screen」 |

Learn が**書いていない**こと（出典なしの選択の根拠）: NULL・無効な hWnd への `SendMessageW`／`SetClassLongPtrW`／`GetWindowLongPtrW`／`SetWindowLongPtrW` の結果の last error、標準のウィンドウメニューの完全な項目一覧、`WTSRegisterSessionNotification` の dwFlags の数値、`BufferedPaintInit` が S_OK 以外を返す条件、`SetWindowTextW` の NULL の lpString の意味、PF_COMPARE_EXCHANGE_DOUBLE と CPUID の CMPXCHG8B ビットの対応。

## 出典なしの選択の一覧

| # | 場所 | 選択 | 区分 |
|---|---|---|---|
| S1 | D5、D3、D9、D10、D14 | **窓表に無い hWnd（NULL、未発行、領域の外、4 の倍数でない、上位 32 ビットだけ違う、符号拡張した値、作成に失敗した窓）は 0 と last error 1400（ERROR_INVALID_WINDOW_HANDLE）**。`SendMessageW`、`SetClassLongPtrW`、`GetWindowLongPtrW`、`SetWindowLongPtrW`、`SetWindowTextW` に適用 | PM の決定 1（出典なしの選択。Learn はこれらの API の NULL・無効な hWnd の結果を書かない。1400 は System Error Codes にあるが API を名指ししない）。**撤回条件: 実機 Windows の測定（Issue #1 の再生ツールの流儀）で値が違うと分かったとき** |
| S2 | D1 | TApplication の窓の矩形は「作成時に渡された値そのもの」（960, 540, 0×0）を保存する。実機が 0×0 を最小トラッキングサイズにクランプするか、クライアント矩形をどう計算するかは Learn に無い（**未確認**）。窓名は 256 ユニットまで（Learn WNDCLASSW の lpszClassName の上限 256 に倣った） | 作業者の選択（読み） |
| S3 | D2 | `MakeObjectInstance` のスロットの形の解釈（`E8 rel32` → `pop r11; jmp [rip+0]` → 8 バイトのジャンプ先）。ページの実行可否の検査は、ページ保護が 0x10／0x20／0x40／0x80（EXECUTE 系）のとき | 設計（ゲストの 3 命令を命令どおりに解釈する。形が違えば停止。決定 0071 の調査 §3.3 の予告に従う） |
| S4 | D4 | `WM_SETICON` を窓に結び付けた hIcon が実在のアイコンかを検査しない。`WM_GETICON` は答えない | 作業者の選択 |
| S5 | D10 | `SetClassLongPtrW(GCLP_HICON)` の新しい hIcon が実在のアイコンかを検査しない。GCLP_HICONSM（-34）は停止（Learn の Set のページは「Retrieves …」と書く書き間違いに見える） | 作業者の選択 |
| S6 | D13 | HMENU の値（`0x7F980000 + 4 × (窓の添字 + 1)`、領域 64 KiB。窓の領域 0x7F990000 の直前）。システムメニューの項目の集合 = 6 つの SC_ コマンド（実機の標準メニューの項目名と一致すると読むが、完全な一覧は Learn に無い。順序・区切りは持たない）。メニューは窓ごとに 1 つ、初回の `GetSystemMenu(…, FALSE)` で作る。`bRevert = TRUE` は 6 項目に戻して NULL | 出典なしの選択（ページは「such as SC_CLOSE, SC_MOVE, and SC_SIZE」と書くだけ） |
| S7 | D11 | `WTSRegisterSessionNotification` は TRUE。WM_WTSSESSION_CHANGE は決して送らない（単一セッション。ページは「RPC_S_INVALID_BINDING が返ることがある」とも書くが、仮想環境で Remote Desktop Services の起動待ちは起きない）。dwFlags の 0 と 1（NOTIFY_FOR_THIS_SESSION、NOTIFY_FOR_ALL_SESSIONS）は SDK ヘッダの値と読む（この作業は wtsapi32.h を読んでいない = **未確認**。ゲストの呼び出しは dwFlags = 0） | 出典なしの選択 |
| S8 | D12 | `BufferedPaintInit` は S_OK（失敗条件はページに無い。実機が S_OK 以外を返す条件は**未確認**）。`BeginBufferedPaint` ほか 4 つは行なし（停止） | 出典なしの選択 |
| S9 | D14 | `SetWindowTextW` の戻り値 = WM_SETTEXT を送った結果（ページは「nonzero on success」と書き、メッセージの結果をそのまま返すとは書かない）。`lpString = NULL` は空文字列（ページは `[in, optional]` と書き、NULL の意味を書かない）。`GetWindowLongPtrW(GWLP_HWNDPARENT)` は親なしで 0。`SetWindowLongPtrW(GWL_EXSTYLE)` は書いた値を即座に `GetWindowLongPtrW` が返す（ページは「SetWindowPos まで効かない」と書く「キャッシュ」を持たない選択） | 作業者の選択 |
| S10 | D6 | PF 2・3 = 1（Learn の表の文言「The atomic compare and exchange operation (cmpxchg) is available」「The MMX instruction set is available」。PF 2 と CPUID EDX bit 8 の対応は読み） | PM の決定 2。**PF 2 の対応は Learn だけでは導けない** |
| S11 | D7 | wtsapi32.dll の N = 37、uxtheme.dll の N = 38（ハンドル `0x7FF800250000`、`0x7FF800260000`。式は決定 0015 のとおり） | 設計（決定 0099、0118 の先例） |

## テスト

新規 `tests/wall_fr6.rs`（**33 件**。`window.rs` と同じ流儀: スタブの窓プロシージャを持つ置換 dispatch 表、実 `fle_win.c`／`fle_class.c`／`fle_callback.c`／`fle_imports.c`／`fle_env.c`、テスト側で定義した `fle_sys_region_at`）と、更新（上の「実装した場所」）:

- 生成器: 8 行が末尾の 8 行で、決定 0118 の `UuidCreate` の行の直後にこの順で並ぶこと（擬似 VA が詰まっていること）、各 thunk が自分の DLL の下でだけ答えられること、`IsProcessorFeaturePresent` に行が無いこと。
- 窓表に無い hWnd: 16 通り（NULL、未発行、領域の外、4 の倍数でない、上位 32 ビットだけ違う、符号拡張、DC のハンドル、GDI のハンドル）× `SendMessageW`／`SetClassLongPtrW` が 0 と 1400、`GetWindowLongPtrW`・`SetWindowTextW`・`SetWindowLongPtrW` も 0 と 1400。作成に失敗した窓も同じ。`DefWindowProcW` は停止のまま。
- `SendMessageW`: 窓プロシージャへの引数（Msg は下位 32 ビット、wParam／lParam は 64 ビット）と戻り値の全 64 ビット、HWND_BROADCAST は停止、関数入口でない窓プロシージャは `fle_dispatch_miss` の名前付き停止。
- TApplication 形の `CreateWindowExW`: 3 メッセージの順序、WM_NCCALCSIZE の RECT = (960, 540, 960, 540)、窓表の項目（スタイル、矩形、窓名の長さ）、第 1 の形の RECT = (0, 0, 0, 0) が不変、CW_USEDEFAULT・サイズ・親・メニュー・lpParam・hInstance・拡張スタイル・スタイルの違い・257 ユニットの窓名は停止。
- `WM_SETICON`（前のハンドルを wParam ごとに返す、NULL で外す、別の wParam は停止）、`WM_SETTEXT`／`SetWindowTextW`（NULL は空）、`SetClassLongPtrW(GCLP_HICON)`（前の値を返し `GetClassInfoW` に反映、他の index 10 種は停止）、`GetWindowLongPtrW`／`SetWindowLongPtrW`（6 つの index、前の値、GWL_STYLE の符号拡張、他の index は停止）。
- システムメニュー: 窓ごとに安定したハンドル、ゲストの 3 回の `DeleteMenu` が成功して項目が減る（0x3f → 0x37 → 0x36 → 0x34）、`bRevert = TRUE` で NULL と 6 項目、別の窓は別のハンドル、WS_SYSMENU の無い窓・別のハンドル・MF_BYPOSITION・無い項目（2 回目の削除を含む）は停止。
- `WTSRegisterSessionNotification`（最初の dwFlags を保つ、メッセージを送らない、dwFlags 2・NULL・未発行は停止）、`BufferedPaintInit`（S_OK、回数）、`wtsapi32.dll`／`uxtheme.dll` が 3 つの綴りで `LoadLibraryW`／`GetModuleHandleW` が同じハンドルを返し、各 DLL は自分の行だけを持ち、残る 5 名の absent は 126。
- `MakeObjectInstance` のスロット: 形が正しいスロットは `r11 = slot + 5` で共通スタブが呼ばれ（`SendMessageW`、`SetWindowTextW`、クラスの窓プロシージャ経由の `CreateWindowExW` の 3 メッセージ）、形が違う 8 通り（先頭が E8 でない、テンプレートの 2 バイト、テンプレートのジャンプ、非実行ページ 2 通り、領域の外へ飛ぶ call 2 通り、領域の末尾のスロット）と、ジャンプ先が関数入口でない場合は名前付き停止、領域の外のアドレスは従来どおり。
- 第 2 スレッドは 8 つの新 API すべてで停止。ソース検査: `fle_set_last_error` は 1 か所（1400）、`invalid_window();` の呼び出しは 5 か所、ホストの状態・メッセージキューの関数を使わない、`fle_sys_region_at` は weak 参照。
- 実 DLL のバイト列: 壁 9〜17 の呼び出し点（SendMessageW の WM_SETICON の引数、SetClassLongPtrW の -14、CreateWindowExW の style 0x84ca0000 と X・Y の計算、WTS と uxtheme の遅延ロード stub と DLL 名、`MakeObjectInstance` のテンプレート `41 5B FF 25 00 00 00 00` と共通スタブ、`GetSystemMenu` と 3 つの `DeleteMenu` の id、`GetWindowLongPtrW`／`SetWindowLongPtrW`／`SetWindowTextW` の呼び出し、`TApplication.WndProc` の `DefWindowProcW` への転送）。

## 検証（実測）

- **全体テスト**（`logs/fulltest/cargo-test.txt`）: 1 回の通し（2026-10-30T09:23:33Z〜10:09:45Z、46 分 12 秒、終了 101）で **120 test result・1,680 passed・1 failed・ignored 0**。基準 119・1,647・0 failed。内訳: +1 バイナリ（`wall_fr6`）、+33 件（`wall_fr6`）。**失敗の 1 件**: `wall_fr5::the_entry_is_chosen_by_the_rules_of_lookup_icon_id_from_directory_ex`（`FLENGINE_PE` を設定すると、256×256 の `MAINICON` の RT_ICON 6 が **PNG 圧縮**（先頭 8 バイト `89504e470d0a1a0a`、pefile で実測）で、`fle_icon_group_pick` が「the chosen RT_ICON is PNG-compressed: not read」で止まり、テストは 256×256 の期待値を持つため落ちる）。**HEAD の clean archive（`FLENGINE_PE` 設定あり）でも同じ失敗**（実測）。`FLENGINE_PE` が未設定でワークツリーに `binaries` が無いときは、このテストは SKIP して通る。この差分は `fle_res.c` を変えていない（`cmp` で同一）。決定 0118 の「1,647 passed」は、環境変数の与え方が違った（このテストが SKIP だった）可能性があるが、前作業は与え方を記録していない（**未確認**）。この作業はこのテストを直していない（範囲外）。
- **全体テストの経過（正直な記録）**: 通しは 3 回行い、数えるのは最後の 1 回。(1) 1 回目（07:21〜08:07Z、46 分 13 秒）は **120・1,648 passed・32 failed**。失敗 32 件を名前で数えて原因を分けた（`logs/fulltest-run1/`）: (a) ワークツリーに `tools/` が無かった（`git archive HEAD lift docs` だけで作ったため。`authenticode_check` の **10 件**。`git archive HEAD tools` を足して解消）、(b) `BLEND2D_PE` に原本（`lift/out-blend2d/Blend2D.dll`）を渡していた（テストが期待するのはリベース済み 0x4700000 の像。名前に `blend2d` を含む **18 件**が `VaOutOfImage` などで落ちた。`fe-pe rebase` で作った像（`out-seed58` のファイルとバイト一致を `cmp` で確認）を渡して解消）、(c) 実装の副作用の更新漏れ **3 件**（`cursor`、`gdi_bulk`、`vcall_thunk_dll`）、(d) 既存の失敗 **1 件**（上）。計 10 + 18 + 3 + 1 = 32。**1 回目の失敗の 31 件はワークツリーの準備・指示の値・更新漏れで、実装の欠陥ではなかった。** 
(2) 2 回目（08:13〜08:59Z、46 分 1 秒）は 120・1,679・1 failed。この通しの後に、PM の決定 1 が `SetWindowLongPtrW` にも及ぶこと（最初の実装は決定 0071 のまま窓表に無い hWnd を停止にしていた）に気づいて直した。(3) 3 回目（09:23〜10:09Z）は 120・**1,680**・1 failed。直したあとの木で、これが最終。最終の通しの後に変えたのは `fle_win.h` のコメント 1 行（「1 MiB」→「64 KiB」。オブジェクトは同一を `cmp` で確認）だけで、`wall_fr6` を個別に再実行して 33 件通った。
- **変異検査**（木のコピー `mut/` で 1 件ずつ変更 → 該当テストを実行 → 落ちる → 元に戻す。`logs/mutation-run.txt`、`tools/mutate119.py`）: 対照 M0（変更なし）は通る。M1 NULL の窓の last error を 1400 でなく 1 に、M2 NULL の窓の戻り値を 0 でなく 1 に、M3 WM_NCCALCSIZE の矩形を (0, 0, 0, 0) に戻す、M4 TApplication 形が CW_USEDEFAULT を受理、M5 WM_SETICON が新しいハンドルを返す、M6 ICON_SMALL を大アイコンの枠に、M7 `SetClassLongPtrW` が新しい値を返す、M8 メニューが 5 項目、M9 消した項目の 2 回目の `DeleteMenu` を受理、M10 MF_BYPOSITION を受理、M11 `GetSystemMenu(TRUE)` がハンドルを返す、M12 GWL_STYLE を符号拡張しない、M13 `SetWindowLongPtrW(GWL_EXSTYLE)` が新しい値を返す、M14 WTS が 2 回目の dwFlags で上書き、M15 `BufferedPaintInit` が数えない、M16 スロットの呼び出しで r11 = 0、M17 テンプレートの検査を外す、M18 非実行ページを呼ぶ、M19 wtsapi32 を absent に戻す、M20 応答表の行の順序を入れ替え、M21 PF 2 = 0、M22 PF 3 = 0、M23 PF 14 = 1、M24 `SetWindowTextW` の戻り値を TRUE に置き換え: **24 件すべて検出**（依頼は 5 件以上）。**M24 を検出するテストは、スタブの窓プロシージャの答え 0x5151 を期待している**（仕様から導いた値ではなく、S9 の選択を固定している）。
- **測定**（`logs/`。6 モジュール構成。ビルド gen 約 15 s + build 約 45 s、合計 約 1:00、最大 RSS 約 2.7 GB）:

| 構成 | native SHA-256 | 結果 |
|---|---|---|
| FRUITY 無効・HEAD（`base1`、`base2`） | `27cdf764f10ffabc08bba2d4afb45815441cdb4e35c05221b102ce5908d23f6d` | a4 942/942、895/895、functions 5,308、entries 1,007,947、終了 0（独立 2 回で同一。指定の基準と一致） |
| FRUITY 有効・HEAD（`fr0`） | `48a81021c332e2271ac02546e435c97240a7eae26a4437f03597a87c7aea4259` | `SendMessageW`（0x425390）、38,913／379、134（決定 0118 と一致） |
| 壁 9 を直した（`w9`） | `3ac11279…6b12` | `CreateWindowExW` の dwStyle 0x84ca0000、38,928／384 |
| 壁 10（`w10`） | `ad80919c…ca7e` | `RaiseException 0xc06d007e`（wtsapi32）、38,937／387 |
| 壁 11（`w11`） | `7f4d36ea…b44e` | `RaiseException 0xc06d007e`（uxtheme）、38,943／388 |
| 壁 12（`w12`） | `a9d0b6b5…78f0` | `fle_dispatch_miss`（スロット）、38,948／386 |
| 壁 13（`w13`） | `b180b217…1c89` | `DefWindowProcW` の WM_SETICON、38,953／390 |
| 壁 14（`w14`） | `9862816e…ac5f` | `SetClassLongPtrW`、38,957／390 |
| 壁 15（`w15`） | `eb31a249…ca1d` | `GetSystemMenu`、38,958／391 |
| 壁 16（`w16`） | `fa543fd7…c89f` | `GetWindowLongPtrW`、38,964／394 |
| 壁 17（`w17`） | `b65110f7…2a11` | `SetMapMode`（壁 18）、40,512／485 |
| 実験 X18（`x18`。差分に入れない） | `e916ed60…f5d0` | `GetClassInfoW(0x1d00000, "TSharedAnimTimerWindow")`、41,150／559 |
| **FRUITY 無効・最終**（`fin-off1`、`fin-off2`） | **`49ce46e505089c118e4fbaef9b05b56db6b2fabdfbea8799016dc86a8d9f1d03`** | a4 942/942、895/895、functions **5,308**、entries **1,007,947**、`returned from entry`、終了 0。別の OUT・別の cargo target、fe-lift／fe-pe の SHA-256 同一（`bb0c20fc…fa14d`、`743e2faf…70b0`） |
| **FRUITY 有効・最終**（`fin1`、`fin2`） | **`029bb30e4bb47d237cd0b38f8a404aaf4fd3540eba895778fb4890a49da65763`** | `SetMapMode`（壁 18）、**40,512／485**、134。`fin2` は別の OUT・別の cargo target。`fruity:` の出力は同一（経過時間と PIE のポインタ値を除く） |

  壁 9〜17 の途中経過の SHA（`w9`〜`w17`）は、その壁の時点の木のものである（最終の木では `SetWindowLongPtrW` の扱いなどが変わったので、最終の SHA とは別）。
- **停止点の詳細**（壁 18。`logs/stop18/`）: シャドウスタック、レジスタ、直前の import thunk の列、C スタック（シンボル化）、ゲストが作った窓とクラスの一覧（下の「到達点の状態」）。
- **差分の当たり方**: `patch -p1 --dry-run` を HEAD 9013ad5 の clean な archive に当てて確認する（`logs/patch-verify.txt`、`MERGE-NOTES.md`）。

## 到達点の状態（`CreateFruityInstance` の停止時点。メッセージループではない）

(a)（戻る）にも (b)（ブロッキングのメッセージ待ち）にも到達していないので、依頼が (b) のために求めた「API・引数・タイムアウト」は**無い**。代わりに、停止時点（壁 18）の状態を記録する（実測。`logs/stop18/`。測定専用の出力（窓表とクラス表のダンプ）は差分に入れず、同じ最終構成の runtime に出力だけを足した木で取った。native の挙動は同じで、停止点・入口数・窓表の値は最終の測定と一致）。

ゲストが作った窓（仮想窓表。5 窓）:

| HWND | クラス（アトム、hInstance） | style／exstyle | 矩形（x, y, w × h）、親 | 窓プロシージャ（保存値） | その他 |
|---|---|---|---|---|---|
| 0x7f990004 | `TPUtilWindow`（0xc000、0x400000） | 0x80000000／0x80 | (0, 0, 0 × 0)、親なし | `MakeObjectInstance` のスロット | — |
| 0x7f990008 | `TPUtilWindow`（0xc001、0x6000000 = ILTools） | 0x80000000／0x80 | (0, 0, 0 × 0)、親なし | スロット | — |
| 0x7f99000c | `TPUtilWindow`（0xc000、0x400000） | 0x80000000／0x80 | (0, 0, 0 × 0)、親なし | スロット | — |
| 0x7f990010 | `TPUtilWindow`（0xc000、0x400000） | 0x80000000／0x80 | (0, 0, 0 × 0)、親なし | スロット | — |
| 0x7f990014 | `TApplication`（0xc002、0x400000） | **0x84ca0000**／0x80 | (960, 540, 0 × 0)、親なし | スロット | 窓名 `FL Studio`、icon_big 0x7fba0624、システムメニュー HMENU 0x7f980014（項目 0x34 = SC_MINIMIZE、SC_CLOSE、SC_RESTORE）、WTS 登録済み（dwFlags 0） |

クラス表（4 クラス）: `TPUtilWindow`（0xc000、0x400000、lpfnWndProc 0x424920 = `DefWindowProcW` の thunk）、`TPUtilWindow`（0xc001、0x6000000 = ILTools の自前の登録。lpfnWndProc 0x601de90）、`TApplication`（0xc002、lpfnWndProc 0x424920、hIcon 0x7fba0624）、`TLightweightLayeredControl`（0xc003、lpfnWndProc 0x71dd10）。ゲストが作った窓は**どれも可視でない**（WS_VISIBLE なし、`ShowWindow` は呼ばれていない）。

## 解釈が分かれた点

1. **壁 18 で止める判断**: 依頼は「導けない値は出典なしの選択として列挙して進む」と「PM の判断が要る値、(i) 描画・GDI の大きなモデルは止める」の両方を書く。壁 18 の答え（画面 DC の `LPtoDP`）は、物理寸法という決定 0053 が PM の判断に残した値を要り、しかも先はスプラッシュ画面のフォームの作成（PNG・TBitmap）で、(a)(b) に近づく見込みを示せない。**止める側**に倒した。進める側に倒すなら実験 X18 の差分がそのまま使える。
2. **`PostMessageW` を PM の決定 1 の対象にしなかったこと**: Learn は `PostMessageW` の NULL の hWnd の結果を明記する（PostThreadMessage と同じ）。決定 1 は「同系」として `PostMessageW` を挙げるが、前提（Learn が書いていない）が成り立たないので、実装しなかった（実行も届かなかった）。実装するときは PM の決定からの逸脱になる。
3. **`SetWindowTextW` を決定 1 の「同系」に含めたこと**: 決定 1 は `PostMessageW`/`GetWindowLongPtrW`/`SetWindowLongPtrW` を挙げる。`SetWindowTextW` は挙げていないが、同じ種類（hWnd を窓表から引く API）なので同じ規則（0 と 1400）にした。範囲を広げた判断なので、PM が戻すと言えば戻せる。
4. **`SetWindowLongPtrW` の扱い（途中で訂正）**: 最初の実装は決定 0071 のままで、窓表に無い hWnd を名前付き停止にしていた。決定 1 が `SetWindowLongPtrW` を挙げているので、最終工程の途中で 0 と 1400 に直し、測定・全体テスト・変異検査をやり直した。
5. **PF 2 と CPUID EDX bit 8 の対応**: Learn は PF 2 を「The atomic compare and exchange operation (cmpxchg) is available」とだけ書く。PM の決定 2（CPUID と食い違わない値）に従い、PF 2 を CX8（CMPXCHG8B）に当てて 1 にした。cmpxchg16b は別の PF 14（0 のまま。CPUID は報告しない）。対応は Intel SDM の EDX[8] = CMPXCHG8B（決定 0118 が取得）との**読み**で、Learn 単独では導けない。
6. **`BLEND2D_PE` の与え方**: 依頼の指示は `BLEND2D_PE` を「実ファイル」へ、基準 119・1,647 を挙げる。実ファイルとして原本 `lift/out-blend2d/Blend2D.dll` を最初に渡したら、`msvc_switch` が 18 件落ちた（テストはリベース済み 0x4700000 の像を期待する）。リベース済みの像（`fe-pe rebase` が作ったもの）を渡して解消した。決定 0118 が何を渡したかは記録が無い（**未確認**）。
7. **全体テストの数**: 基準 119・1,647 に対して 120・1,680（+1 バイナリ、+33 件）。1 失敗は既存の失敗（上）で、基準の 1,647 はその 1 件が通っていた（SKIP だった）可能性がある。基準と数をそのまま比べると、1,647 + 33 = 1,680 が通った件数に一致する。

## PM の判断が要る点

1. **壁 18 の扱い**（Y1 止まる／Y2 画面 DC の MM_LOMETRIC を 96 DPI から逆算／Y3 スプラッシュを避けるレジストリの種）。推奨は Y1 か Y3。Y2 は物理寸法という出典なしの値を足すが、次の壁（QFC の窓クラス）も描画前の準備段階。
2. **メッセージキューの設計**: この作業の実行はキュー（`PostMessageW`、`GetMessageW`、`PeekMessageW`、`WaitMessage`、`MsgWaitForMultipleObjects`）に**届かなかった**。(b) に到達するには、壁 18 の先（スプラッシュ、`CreateFruityInstance` の本体、その後のメインフォーム）を越える必要がある。キューを実装する前に、どの壁を越えればエンジンが最初のブロッキング待ちに入るのかは未測定。
3. **`PostMessageW` の NULL**: Learn は NULL を PostThreadMessage と同じと書く。PM の決定 1 の 1400 とは違う。実装時にどちらにするか。
4. **システムメニューの項目の集合と HMENU の値**（S6）、**`SetWindowTextW` を決定 1 の範囲に含めたこと**（解釈 3）、**`IsProcessorFeaturePresent` PF 2 の CPUID との対応**（解釈 5）。
5. **決定 0118 から持ち越し**: FL64.exe のパス（実機値）、`General\SplashImage`・`MIDIForm\SplashBox` などのレジストリの種（壁 18 と関係）、時計と署名の有効期間（決定 0117）。
6. **docs の索引**: `docs/README.md` の 0119 の行は `docs/README-0119-line.txt`。注記の件数（決定ファイル 119 → 120、番号の種類、索引の行、仮の決定）は、足す側の確認を要する（この作業では `docs/README.md`、`docs/progress.md` を編集していない）。
7. **wasm の再ビルド**: runtime が変わるので wasm の SHA が変わる（別の作業）。

## 撤回条件

- **S1（0 と 1400）**: 実機 Windows の測定（Issue #1 の再生ツールの流儀で依頼できる）で値が違うと分かったとき。ゲストがその last error を読むと分かったとき。Learn が NULL・無効な hWnd の結果を書いたとき。
- **D1（TApplication 形）**: 実機で窓の矩形が作成時の値と違うとき（最小トラッキングサイズのクランプ、NC 領域）。ゲストの窓プロシージャが WM_GETMINMAXINFO・WM_SIZE・WM_MOVE に反応し、それが `CreateWindowExW` から実機で送られると分かったとき（決定 0071 の撤回条件と同じ）。
- **D2（スロットの解釈）**: PM が「生成コードの解釈は別の決定」と判断したとき（戻すには `call_wndproc` の VirtualAlloc の分岐を外す。壁 13 の停止に戻る）。ゲストが別の形の生成コードを窓プロシージャにしたとき（形が違えば停止する）。
- **D3、D4、D10、D14（窓表の API）**: 実機で GWL_EXSTYLE のキャッシュ（SetWindowPos まで効かない）がゲストの読み出しで観測されると分かったとき。`WM_SETICON` の戻り値や `SetClassLongPtrW` の前の値が実機と違うとき。
- **D6（PF 2・3）**: 実機で `IsProcessorFeaturePresent(2)` が CPUID の CX8 と別の値と分かったとき。CPUID と切り離して実機と同じ値にする決定（決定 0078 の撤回条件）。
- **D7（wtsapi32.dll・uxtheme.dll の仮想モジュール化）**: 実機でこの 2 つの扱いが別だと分かったとき。他の `fle_load_absent[]` の 5 名は、呼ばれたときに（決定 0099 の流儀で）決める。
- **D11、D12、D13**: 実機の戻り値・項目の集合が違うと分かったとき。
- 壁 18: PM が Y2／Y3 を決めたとき。

## 限界

- 実機 Windows との照合は 0 件。wasm・ブラウザは未測定（native のみ）。
- 目的 (a)（`CreateFruityInstance` が戻る）にも (b)（ブロッキングのメッセージ待ち）にも**到達していない**。停止は壁 18（停止条件 (i)(iii)）。壁 18 の先は実験 X18 で 41,150 入口まで見ただけ（次は QFC の窓クラスで、メッセージループではない）。
- **メッセージキューを実装していない**（実行が届かなかった）。ブロッキング待ちの API（`GetMessageW`、`PeekMessageW`、`WaitMessage`、`MsgWaitForMultipleObjects` ほか）に至る壁を、1 つも測っていない。
- 窓の実際の矩形・可視性・Z 順・フォーカス・アクティブ窓のモデルは持たない（`GetActiveWindow` ほかは決定 0117 の停止のまま）。
- `MakeObjectInstance` のスロットの解釈は、決定 0071 の調査が読んだ 1 つの形（`E8 rel32`、テンプレート、8 バイトのジャンプ先、スロット 0x15 バイト）に限る。ほかの生成コードは停止する。
- 時間と RAM: 1 回の 6 モジュールのビルド（gen + build）が約 1:00、最大 RSS 約 2.7 GB。実行は 0.06 秒の呼び出し + 約 1 秒のハーネス。全体テストは約 46 分。
