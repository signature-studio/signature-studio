# 0098: 壁の応答 第 1 バッチ（census が見つけた dsp 構成の壁のうち、未応答 import 6 つと、値が決まっていなかった 2 つ）。すべて仮の決定

状態: **仮の決定（すべての応答）**。依頼は PM（2026-10-27。census 0097 の壁表の担当割り）。決定は作業者（サブエージェント）が行い、書いた。**実機 Windows との照合は無い**（ホストは Linux）ので、どの応答も「Microsoft Learn と、これまでの決定から導いた選択」で、API ごとに撤回条件を書いた。ホストの画面・ファイル・レジストリ・ロケール・フォント・音声デバイス・時計・スレッドは**一切読まない**（0053、0057、0058、0065 と同じ方針）。

記入状況: すべて記入済み（2026-10-27 の実測。決定・調査の節は実装の前に骨子を保存し、実装・測定の後に書き足した）。

事実の出典:

- 決定 0097（census。壁の表 `lift/out-census/docs-new/wall-census-2.tsv`、`wall-census-2-notes.md`）。**census の数は A4 の進捗ではない**（0046、0060、0097）。
- 決定 0015（応答表は末尾に足す。擬似 VA は動かさない）、0018（仮想ファイルシステム。初期状態は空）、0053（仮想画面 DC。1920×1080、32 bpp、96 DPI）、0057（GetSystemMetrics）、0058（SystemParametersInfoW）、0064（`fle_call_guest`）、0065（仮想モニタ。rcWork = rcMonitor。SPI_GETWORKAREA を答えるときの条件）、0066（システムフォント "Segoe UI"、重み 400）、0072（CompareStringW の ASCII の枠）。
- ゲストの逆アセンブル（`objdump -d -M intel`。`lift/out-wallR/dis.sh`）と、census を gdb で動かして停止点の引数を読んだ記録（`lift/out-wallR/gdb-args.py`、`runs/gdb1/gdb.out`。読み取りのみ）。
- Microsoft Learn（2026-10-27 に取得。各 API の節に URL）。本文の「Learn」は取得したページ、「SDK ヘッダ」は mingw-w64 のヘッダ（`/usr/share/mingw-w64/include/`。非公式）。

## 決定（全体）

| # | 項目 | 決定 |
|---|---|---|
| 1 | 応答の方針 | 8 件のすべてに仮の値で答える（下の API ごとの節）。**ただし SM_DIGITIZER と同じ関数が続けて問う SM_TABLETPC（86）、SM_CONVERTIBLESLATEMODE（0x2003）、SM_SYSTEMDOCKED（0x2004）と隣の SM_MAXIMUMTOUCHES（95）は決めず、名前付き停止のまま**にする（値の根拠がなく、0x2003 は PM の判断が要る。8 節）。値は仮想で固定し、ホスト依存の応答は無い |
| 2 | 実装の場所 | 新しい `.c`、`.h` は作らない。既存の `fle_winmm.c`（acmMetrics）、`fle_ole.c`（SHGetMalloc と IMalloc）、`fle_monitor.c`（EnumDisplaySettingsW）、`fle_cursor.c`（GetClipCursor）、`fle_gdi.c`（EnumFontFamiliesExW）、`fle_vfs.c`（GetFileVersionInfoSizeW）、`fle_spi.c`（SPI_GETWORKAREA）、`fle_sysmetrics.c`（SM_DIGITIZER） |
| 3 | 応答表 | `imports_table.rs` の `TABLE` の**末尾に 10 行**を足す（6 つの API と、IMalloc の vtable の 4 行）。既存の擬似 VA は動かない（0015） |
| 4 | 答える形の範囲 | ゲストが使う形だけに答える。ページが結果を定めない形、ゲストが使わない形は、**名前付きで停止**する（`fle_stop`。値を作らない） |
| 5 | last error | ページが書かない成功では触れない（`GetFileVersionInfoSizeW` の失敗だけは、仮想ファイルシステムの 2 または 3 を設定） |
| 6 | 数値の出典の区別 | 下の表。「Learn」= ページが書く。「SDK ヘッダ」= 非公式（Learn は名前だけで数値を書かない）。「選択」= 出典なし |

## 値の出典の区別

| 項目 | 値 | 出典 |
|---|---|---|
| ACM_METRIC_MAX_SIZE_FORMAT の番号 | 50 | SDK ヘッダ（`msacm.h`）。Learn は名前だけ。ゲストの `mov edx,0x32` と一致 |
| acmMetrics が書く最大の WAVEFORMATEX | 18 | **選択**。構造体の大きさ 2+2+4+4+2+2+2 = 18 は Learn（WAVEFORMATEX）の型から計算 |
| IMalloc の vtable の順 | QueryInterface, AddRef, Release, Alloc, Realloc, Free, GetSize, DidAlloc, HeapMinimize | SDK ヘッダの順（Learn の IMalloc ページはメソッドをアルファベット順に並べ、順序を書かない）。ゲストの呼び出し先 +0x10、+0x28、+0x38 が Release、Free、DidAlloc と一致（読み） |
| ENUM_CURRENT_SETTINGS | (DWORD)-1 | SDK ヘッダ（`winuser.h`）。ゲストの変数の初期値 0xFFFFFFFF（DLL のバイト）と一致 |
| DM_BITSPERPEL ほか 5 つの dmFields のビット | 0x40000、0x80000、0x100000、0x200000、0x400000 | SDK ヘッダ（`wingdi.h`）。Learn は名前を書く |
| DEVMODEW のオフセット | dmSize 68、dmFields 72、dmBitsPerPel 168、dmPelsWidth 172、dmPelsHeight 176、dmDisplayFlags 180、dmDisplayFrequency 184、全体 220 | Learn の DEVMODEW のメンバの順と型から計算（和は作業者）。ゲストが dmSize = 0xDC を +0x44 に書き、周波数を +0xB8 から読むのと一致（読み） |
| dmDisplayFrequency | 60 | **選択**（出典なし）。ゲストが API の失敗時に使う既定値（-60）と同じ |
| dmBitsPerPel、dmPelsWidth、dmPelsHeight | 32、1920、1080 | 決定 0053（`FLE_GDC_BITSPIXEL`、`FLE_GDC_HORZRES`、`FLE_GDC_VERTRES` の名前で結ぶ） |
| TRUETYPE_FONTTYPE | 4 | SDK ヘッダ（`wingdi.h`）。Learn は名前だけ |
| ENUMLOGFONTEXW の大きさ | 348 | Learn の型から計算（LOGFONTW 92 + WCHAR[64] + WCHAR[32] + WCHAR[32]） |
| NEWTEXTMETRICEXW の大きさ | 100 | Learn の型から計算（NEWTEXTMETRICW 76 + FONTSIGNATURE 24）。ゲストは読まない |
| 仮想フォント | "Segoe UI"、重み 400 | 決定 0066 の仮の値 |
| SPI_GETWORKAREA の番号 | 0x0030 | Learn の uiAction の表 |
| SPI_GETWORKAREA の矩形 | (0, 0, 1920, 1080) | 決定 0065（rcWork と同じ。`FLE_MONITOR_WORK_*` の名前で結ぶ）。Learn は値を書かない |
| SM_DIGITIZER | 94（Learn）、値 0 | 番号は Learn。値の 0 は選択（Learn は「ビットマスク」と書き、定義した場合以外は 0） |

## 1. msacm32!acmMetrics（unit 170）

| 項目 | 内容 |
|---|---|
| 呼び出し元の読み | 3 か所（0x67261c、0x6726eb、0x674066。thunk 0x6708d0、IAT 0x1972160）。どれも `hao = NULL`（`xor ecx,ecx`）、`uMetric = 0x32`、`pMetric = DWORD` へのポインタ。**戻り値（MMRESULT）はどこでも見ない**。DWORD は呼ぶ前に 0 を書く。census が止めたのは 0x67261c（`f_672600`。unit proc 0x675290 の下）。gdb で停止時の引数 (0, 0x32, 0x15f47e8) を確認。DWORD は `malloc(size)`（0x409410）とバッファの 0 埋めに使われ、`f_672600` だけが `size >= 0x1a` のとき 26 バイトの WAVEFORMATEX（wFormatTag は DLL のデータ 0x13e94a8 = 0x674F、2ch、44100 Hz、cbSize 8）を書く。0x674082 は確保したバッファの先頭 2 バイトに 1 を書く |
| 仕様（Learn） | [acmMetrics](https://learn.microsoft.com/en-us/windows/win32/api/msacm/nf-msacm-acmmetrics)（`updated_at` 2024-02-22）: `ACM_METRIC_MAX_SIZE_FORMAT`「Returned value is the size of the largest WAVEFORMATEX structure. If hao is NULL, the return value is the largest WAVEFORMATEX structure in the system」、pMetric は DWORD のバッファ。戻り値は「Returns zero if successful or an error otherwise」（ACMERR_NOTPOSSIBLE、MMSYSERR_INVALHANDLE、MMSYSERR_INVALPARAM、MMSYSERR_NOTSUPPORTED）。**ページは「システムの最大」の値を書かず、どの形がどのエラーかも書かない** |
| 決定 | `(hao = NULL, uMetric = 50, pMetric 非 NULL)` だけに答える: DWORD に **18**（`sizeof(WAVEFORMATEX)`）を書き、**0（MMSYSERR_NOERROR）**を返す。この環境に ACM ドライバは無い（「MIDI デバイスなし」0030 と同じ種類の、環境の設定）ので、最大の WAVEFORMATEX は構造体そのもの。last error は触れない。uMetric は UINT なので rdx の下位 32 ビットだけ見る。**他の uMetric、hao 非 NULL（64 ビット全体で比べる）、pMetric NULL は名前付き停止**（ページがどのエラーかを書かない） |
| 帰結（読み） | `f_672600` は `size >= 0x1a` が偽なので 26 バイトの書き込みを飛ばす。実機で ACM のコーデックがあれば（ADPCM など）真になる分岐。確保は 18 バイトで、書き込みは 2 バイト以内（0x674082）なので安全 |
| 撤回条件 | (1) 実機 Windows 10 の `acmMetrics(NULL, ACM_METRIC_MAX_SIZE_FORMAT)` の値が示され、ゲストの分岐（`>= 0x1a`）で動作が変わる、(2) 他の ACM API（`acmFormatSuggest` など。thunk 0x670930 は同じ領域にある）に実行が届き、「ドライバなし」と整合しない、(3) 「この環境にコーデックがある」という方針が決まる |
| テスト | `wall_r.rs`: `acm_metrics_*` 3 件（DWORD に 18 と近傍の不変、UINT の上位ビット、hao の 64 ビット、他の形 4 種の停止と無書き込み） |

## 2. shell32!SHGetMalloc と、返す IMalloc（unit 235）

| 項目 | 内容 |
|---|---|
| 呼び出し元の読み | 2 か所。(a) 0x7082af（unit 235 の proc `f_708260`）: `f_41a770(&[0x15f6100])`（スロットが非 NULL なら 0 にして vtable +0x10 = Release を呼ぶ。続きは逆アセンブルを最後まで読んでいないが、結果を `rcx` に渡す形からスロットのアドレスを返すと読める）の結果を渡して呼ぶ。**HRESULT も得られるポインタも、直後は使わない**。(b) 0x81cba1（`f_81cb60`）: 局所スロットに対して同じ形。`test eax,0x80000000; jne`（失敗なら戻る）、成功なら `[obj+0]` にある PIDL に `vtable +0x38`（DidAlloc）、正なら `vtable +0x28`（Free）を呼び、最後にスロットを Release。gdb で停止時の rcx = 0x15f6100（中身は 0）を確認 |
| 仕様（Learn） | [SHGetMalloc](https://learn.microsoft.com/en-us/windows/win32/api/shlobj_core/nf-shlobj_core-shgetmalloc)（`updated_at` 2024-02-22）: 「Retrieves a pointer to the Shell's IMalloc interface」、「If this function succeeds, it returns S_OK. Otherwise, it returns an HRESULT error code.」。Remarks: Windows 2000 以降は不要で、「the equivalent (and easier to use) CoTaskMemAlloc and CoTaskMemFree」。[IMalloc::Free](https://learn.microsoft.com/en-us/windows/win32/api/objidl/nf-objidl-imalloc-free): 「If this parameter is NULL, this method has no effect」。[IMalloc::DidAlloc](https://learn.microsoft.com/en-us/windows/win32/api/objidl/nf-objidl-imalloc-didalloc): 1 = この allocator が確保した、0 = していない、-1 = 判定できない、「If this parameter is a NULL pointer, -1 is returned」。[IUnknown::Release](https://learn.microsoft.com/en-us/windows/win32/api/unknwn/nf-unknwn-iunknown-release): 「The method returns the new reference count」 |
| 決定 | **S_OK を返し、`*ppMalloc` に 1 つのオブジェクトへのポインタを書く**（失敗を作らない）。オブジェクトは `{ vtable へのポインタ, スロット 9 つ }` をホストの `calloc` で 1 回だけ作る（解放しない。CoTaskMemAlloc の領域と同じ流儀）。スロットは応答表の 4 行（`shell32.dll` の `IMalloc::Release`、`IMalloc::Free`、`IMalloc::DidAlloc`、`IMalloc::unimplemented`）の擬似 VA で、**`fle_resp_table` から名前で探して**埋める（生成器が書いた表と同じ値）。Release = スロット 2、Free = 5、DidAlloc = 7。**残りの 6 スロットは 1 つの停止関数を共有する**（ゲストの呼び出し点が使わない）。確保・解放の実体は `CoTaskMemAlloc` と同じ表（`tasks`）: `Free(NULL)` は何もしない、生きているブロックは解放、そうでない値は名前付き停止（`CoTaskMemFree` と同じ）。`DidAlloc(NULL)` = -1（0xFFFFFFFF）、生きているブロック = 1、ほかのポインタ = 0。`SHGetMalloc` を呼ぶたびに参照を 1 数え、`Release` は減らした後の値を返す（数えていないときの Release は名前付き停止）。`this` がこのオブジェクトでなければ名前付き停止。ppMalloc が NULL なら名前付き停止 |
| 帰結（読み） | unit 235 は SHGetMalloc の先へ進み **ok になった**（census）。`f_81cb60` の PIDL の経路は、PIDL を作る API（SHGetFolderPathW など）がまだ答えないので、実行されていない |
| 選択肢 | A. 採用（上）。B. `E_OUTOFMEMORY` などの失敗を返す: **不採用**（実機では常に成功。失敗値を作ることになる）。C. 名前付き停止のまま: **不採用**（値が決められる）。D. IMalloc の 9 メソッドをすべて実装: **不採用**（ゲストが使うのは 3 つだけで、使われないコードは検証できない。0058 と同じ流儀） |
| 撤回条件 | (1) ゲストが QueryInterface、AddRef、Alloc、Realloc、GetSize、HeapMinimize を呼ぶ（停止する。実装して vtable の行を足す）、(2) vtable の順が SDK ヘッダと違うと分かった、(3) Release が 0 になったときにオブジェクトを解放すべき（Learn は「the implementation must free the object」）と判断された（今は解放せず、次の SHGetMalloc が同じオブジェクトを返す）、(4) 「IMalloc の実体が CoTaskMem と別」であるべき実行が見つかった |
| テスト | `wall_r.rs`: `sh_get_malloc_*`、`imalloc_free_and_didalloc_*`、`every_sh_get_malloc_counts_*`、`the_imalloc_object_stops_*`、`the_vtable_rows_reach_the_runtime_through_the_generated_tables`（生成器の C と全 runtime をリンクし、vtable の擬似 VA を `fle_dispatch` で呼ぶ）、`the_six_thunks_*`（10 行の位置と VA） |

## 3. user32!EnumDisplaySettingsW（unit 246）

| 項目 | 内容 |
|---|---|
| 呼び出し元の読み | 2 か所（0x666d16、0x666dee。thunk 0x424ae0）。`lpszDeviceName = NULL`、`iModeNum = [0x13e86b0]`、`[0x13e86b4]`（DLL のバイトで**どちらも 0xFFFFFFFF = ENUM_CURRENT_SETTINGS**）、DEVMODEW を局所に置き、`dmSize = 0xDC` を +0x44 に書いてから呼ぶ。戻り値が非 0 なら **`dmDisplayFrequency`（+0xB8）だけ**を読み、`>= 0x32`（50）のときだけ使う。0x666d16 を含む関数（0x666cc0 付近）は、周波数 ≥ 50 なら `-周波数`、そうでなければ（API の失敗でも）**既定値 0xffffffc4（-60）**を返す（逆アセンブルで確認）。0x666dee の `f_666d50` は、周波数 ≥ 50 なら周波数から丸めた値の符号を反転して使い、そうでなければ別の経路（ebx から計算）に進む（式は 0x666e04 以降。実行して確かめていない）。ほかのメンバは読まない |
| 仕様（Learn） | [EnumDisplaySettingsW](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-enumdisplaysettingsw)（`updated_at` 2025-07-01）: lpszDeviceName は NULL か DISPLAY_DEVICE.DeviceName で、「A NULL value specifies the current display device」。iModeNum は ENUM_CURRENT_SETTINGS（「Retrieve the current settings」）、ENUM_REGISTRY_SETTINGS、またはモード番号。「set the dmSize member to sizeof(DEVMODE)」。「The EnumDisplaySettings function sets values for the following five DEVMODE members: dmBitsPerPel, dmPelsWidth, dmPelsHeight, dmDisplayFlags, dmDisplayFrequency」。成功は非 0、失敗は 0（「fails if iModeNum is greater than the index of the display device's last graphics mode」）。[DEVMODEW](https://learn.microsoft.com/en-us/windows/win32/api/wingdi/ns-wingdi-devmodew)（dmDisplayFrequency: 「For displays, specifies the frequency, in hertz, of a display device in its current mode」。**今のページには「0 または 1 は既定のリフレッシュレート」という記述は無い**。古いページにあったと記憶するが、確認できていない） |
| 決定 | `(NULL, ENUM_CURRENT_SETTINGS, dmSize が 0xDC の DEVMODEW)` だけに答える。構造体の 0xDC バイトを **0 で埋め**（`dmSize` と `dmDriverExtra` は呼び出し側の入力なので残す）、`dmFields` = 5 つの DM_ ビット（0x7C0000）、`dmBitsPerPel` = 32、`dmPelsWidth` = 1920、`dmPelsHeight` = 1080（決定 0053 の名前）、`dmDisplayFlags` = 0（Learn: 2 つのフラグのどちらも立てない）、**`dmDisplayFrequency` = 60（選択）**。dmDeviceName、dmPosition ほかは決めず 0。1 を返す。last error は触れない。iModeNum は DWORD（rdx の下位 32 ビット）。**停止**: デバイス名が非 NULL、ENUM_CURRENT_SETTINGS 以外の iModeNum（ENUM_REGISTRY_SETTINGS とモード番号 0、1 … を含む。ゲストは使わない）、lpDevMode NULL、dmSize が 0xDC 以外（ページは失敗値を書かない） |
| 周波数の選択 | 60 Hz は出典のない選択。0x666d16 を含む関数では、**60 を答えると返す値は API が失敗したときの既定値と同じ -60**（読み）。0x666dee の `f_666d50` では周波数から計算する経路に入る（実行で確かめていない） |
| 撤回条件 | (1) 実機の値（解像度、リフレッシュレート）が示されて整合しない、(2) 0053 が解像度、色深度を変える（名前で結んであるので自動で追従）、(3) ゲストが ENUM_REGISTRY_SETTINGS、モード番号、デバイス名で呼ぶ、(4) dmPosition、dmDeviceName などを読む実行が現れる |
| テスト | `wall_r.rs`: `enum_display_settings_*` 4 件（構造体の全バイトを期待値と比べる、入力の保持、近傍の不変、停止 8 形で何も書かない、定数） |

## 4. user32!GetClipCursor（unit 272）

| 項目 | 内容 |
|---|---|
| 呼び出し元の読み | 3 か所（0x7f4d8a、0x7f4db0、0x7f5388。thunk 0x424bd0）。0x7f5388（unit 272 の proc `f_7f5350`）は 0x15f6e2c の RECT に受け、**戻り値を見ずに**、すぐ left += 32、top += 32、right -= 32、bottom -= 32 する（画面の内側 32 ピクセルの余白）。0x7f4d8a は局所 RECT に受け、非 0 なら `IntersectRect` で現在の矩形と交差させる。ゲストは `ClipCursor` を import していない（文字列検索） |
| 仕様（Learn） | [GetClipCursor](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-getclipcursor)（`updated_at` 2025-07-01）: 「The structure receives the dimensions of the screen if the cursor is not confined to a rectangle」。成功は非 0、失敗は 0（GetLastError）。「The calling process must have WINSTA_READATTRIBUTES access」 |
| 決定 | この環境にカーソルを閉じ込めるものは無い（ClipCursor は答えず、ゲストも import しない）ので、RECT は**画面の寸法** = (0, 0, 1920, 1080)（決定 0057 の原点、0053 の大きさ、0065 のモニタ矩形と同じ名前）。RECT の 16 バイトだけを書き、1 を返す。last error は触れない。lpRect NULL は名前付き停止 |
| 撤回条件 | (1) `ClipCursor` に答える決定が入る（そのとき GetClipCursor は設定した矩形を返す）、(2) 実機の値（複数モニタ、仮想画面）が示されて整合しない、(3) 0053、0065 が画面を変える |
| テスト | `wall_r.rs`: `get_clip_cursor_*` 2 件 |

## 5. gdi32!EnumFontFamiliesExW（unit 283）

| 項目 | 内容 |
|---|---|
| 呼び出し元の読み | 1 か所（0x866e25。`f_866d80`、thunk 0x4240e0）。hdc = `GetDC(NULL)`（0x866dc3）、`lpLogfont` = 92 バイトを 0 で埋め **lfCharSet = 1（DEFAULT_CHARSET）だけ**を立てたもの（lfFaceName は空）、`lpProc` = 0x8650c0、`lParam` = `[obj+0x78]`（文字列リスト）、`dwFlags` = 0（`[rsp+0x20]`）。呼ぶ前にリストに "Default" を足す（0x866de8）。戻り値は**使わない**。コールバック 0x8650c0 は `rcx`（ENUMLOGFONTEXW）の +0x1c（lfFaceName、32 文字）を局所文字列に写し、リストが空でなく、最後の要素と違えば（`f_4399e0` = 文字列比較）リストに足し、**常に 1 を返す**。ENUMLOGFONTEXW の他のメンバ、NEWTEXTMETRIC、FontType は読まない。gdb で停止時の引数 (0x7fa90010, …, 0x8650c0, …) を確認（hdc は 0053 の 4 番目の DC） |
| 仕様（Learn） | [EnumFontFamiliesExW](https://learn.microsoft.com/en-us/windows/win32/api/wingdi/nf-wingdi-enumfontfamiliesexw)（2023-02-09）: lfCharSet が DEFAULT_CHARSET なら「enumerates all uniquely-named fonts in all character sets」、lfFaceName が空なら「one font in each available typeface name」、有効な名前ならその名前の全フォント。lfPitchAndFamily は「Must be set to zero」。dwFlags は「not used and must be zero」。戻り値は「the last value returned by the callback function」。[EnumFontFamExProc](https://learn.microsoft.com/en-us/previous-versions/dd162621(v=vs.85))（旧ページ）: `int CALLBACK (ENUMLOGFONT *lpelf, NEWTEXTMETRIC *lpntm, DWORD FontType, LPARAM lParam)`、「The return value must be a nonzero value to continue enumeration; to stop enumeration, it must return zero.」。[ENUMLOGFONTEXW](https://learn.microsoft.com/en-us/windows/win32/api/wingdi/ns-wingdi-enumlogfontexw): `{ LOGFONTW elfLogFont; WCHAR elfFullName[64]; WCHAR elfStyle[32]; WCHAR elfScript[32] }`。**ページはどのフォントが存在するかを書かない**（システムのフォントの集合は環境の事実） |
| 決定 | この環境の**フォントは 1 つの family だけ**: 決定 0066 のシステムフォント "Segoe UI"（重み 400）。hdc が決定 0053 の生きている DC（`GetDC(NULL)` が返し、まだ解放していないもの）で、`lfCharSet` = DEFAULT_CHARSET かつ lfFaceName が空、またはその family 名（ASCII の大文字小文字を畳む）のとき、**コールバックを 1 回**、`fle_call_guest`（0064）で呼ぶ: `rcx` = ENUMLOGFONTEXW（348 バイト。ゲストのスタックの API のフレームより下、16 バイト境界）、`rdx` = NEWTEXTMETRICEXW（100 バイト、全部 0）、`r8` = FontType = **TRUETYPE_FONTTYPE（4）**、`r9` = lParam（64 ビットそのまま）。ENUMLOGFONTEXW は 0 で埋め、**lfWeight = 400、lfFaceName = elfFullName = "Segoe UI"** だけを書く（elfStyle、elfScript、lfCharSet ほかは決めず 0）。戻り値は**コールバックが返した値の下位 32 ビット**（int）。last error は触れない。**停止**: hdc が生きている DC でない、lpLogfont か lpProc が NULL、dwFlags ≠ 0、lfPitchAndFamily ≠ 0、lfCharSet が DEFAULT_CHARSET でない、family 名が空でも "Segoe UI" でもない |
| 帰結（実測） | unit 283 はコールバックまで進む。コールバックの `f_4399e0` が "Default" と "Segoe UI" を比べ、**`CompareStringW` が決定 0072 の枠（ASCII の英数字だけ。"Segoe UI" の空白で枠の外）で名前付き停止**。つまり次の壁は 0072 の CompareStringW の ASCII の枠で、本決定の外 |
| 選択肢 | A. 採用（1 family）。B. 0 family（コールバックを呼ばない）: **不採用**（0066 が "Segoe UI" をシステムフォントと決めており、それと矛盾する。戻り値の意味もページが書かない）。C. 実機の一般的な一覧（Arial、Tahoma …）: **不採用**（出典がなく、ホストの一覧に似せることになる）。D. 名前付き停止のまま: **不採用**（0066 から値が決められる） |
| 撤回条件 | (1) ゲストがフォントの一覧を前提にして破綻する（例: 一覧から "Arial" などを選ぶ）、(2) 0066 のシステムフォントが変わる（`FLE_SPI_FONT_FACE` と `FLE_SPI_FONT_WEIGHT` の名前で結んであるので自動で追従）、(3) ゲストが family 名や文字セットを指定して呼ぶ、(4) 0072 の CompareStringW が空白を含む名前を扱うようになった後に、一覧の中身が別の判断を要する |
| テスト | `wall_r.rs`: `enum_font_families_*` 5 件（全 family、名前指定の大文字小文字、戻り値の int 化、構造体の全バイト、停止 11 形でコールバックなし、スタックとレジスタの保存） |

## 6. version!GetFileVersionInfoSizeW（unit 880）

| 項目 | 内容 |
|---|---|
| 呼び出し元の読み | 3 か所（0x651d2d、0x44ade6、0xa5fd26。thunk 0x423e00）。0x651d2d（`f_651ce0`。`f_6520b0` の下）は**サイズ 0 なら `jbe` で飛ばして終わる**（バージョン情報なし）。0x44ade6（`f_44ada0`）は 0 なら既定値 0xffffffff（「バージョン不明」）を返す。つまりゲストは 0 を許容する。gdb で停止時の `lptstrFilename` = `C:\Program Files\Image-Line\FL Studio 21\FLEngine_x64.dll`（`GetModuleFileNameW` の値、決定 0018）、`lpdwHandle` = スタックの変数を確認 |
| 仕様（Learn） | [GetFileVersionInfoSizeW](https://learn.microsoft.com/en-us/windows/win32/api/winver/nf-winver-getfileversioninfosizew)（`updated_at` 2024-11-20）: lptstrFilename は「The function uses the search sequence specified by the LoadLibrary function」。lpdwHandle は「A pointer to a variable that the function sets to zero」（省略可）。成功はバージョン情報のバイト数、失敗は 0（GetLastError）。**失敗の条件とエラー番号をページは書かない** |
| 決定 | 仮想ファイルシステム（0018）は**ゲストが作ったものだけ**を持つので、そこにないパスにバージョン情報はない: **0 を返し、last error に仮想ファイルシステムが返す番号（`GetFileAttributesW` と同じ 2 または 3）を設定する**。ゲスト自身の DLL のパスも仮想ファイルシステムにはない（`GetModuleFileNameW` はその名前を返すが、`CreateFileW` は開けない）ので `ERROR_PATH_NOT_FOUND`（3。`C:\Program Files` が無い）。**`*lpdwHandle` は（非 NULL なら）失敗でも 0 にする**（ページは「sets to zero」と書き、失敗時かどうかを書かない。推測）。**停止**: lptstrFilename NULL、**ディレクトリ部のない名前**（LoadLibrary の検索順序を模していない）、パスが**仮想ファイルシステムに存在する**（ゲストが作ったファイルまたはディレクトリ。仮想ファイルのバージョン資源は無い） |
| 帰結（実測） | unit 880 は GetFileVersionInfoSizeW の先へ進み、**次の壁は別の場所**: unit の proc `f_1263540` が呼ぶ `f_125da90` が `LoadLibraryW(L"SHFolder.dll")`（仮想モジュール）→ `GetProcAddress(…, L"SHGetFolderPathW")`（応答表に行がなく NULL）→ **その NULL を呼ぶ**（CSIDL 0x1a = APPDATA。`indirect call to 0`）。census は「二次の疑い」と印を付けたが、**読みでは一次**（SHGetFolderPathW の未応答。DLL の文字列とコードを読んだ）。本決定の外 |
| 撤回条件 | (1) ゲスト自身の DLL を仮想ファイルシステムに置く決定（`CreateFileW` が開けるようになる）が入り、バージョン資源（PE の RT_VERSION。`fle_res.c` が自分のリソースを持つ）を返すべきになった、(2) 実機の失敗番号（2 か 3 か）が示され、食い違う、(3) 名前だけのパス（検索順序）を呼ぶ実行が現れる |
| テスト | `wall_r.rs`: `version_info_size_*` 4 件（自分の DLL のパス = 0 と 3、存在しない名前 = 2 と 3、`lpdwHandle` NULL、存在するパスの停止、名前だけの停止） |

## 7. SystemParametersInfoW の uiAction 0x30 = SPI_GETWORKAREA（unit 275）

| 項目 | 内容 |
|---|---|
| 呼び出し元の読み | 4 か所（0x8190a6、0x866d1a、0x8673a5、0x11c3cd8。決定 0058）。census が止めたのは 0x8673a5（`f_867390`）。gdb で停止時 (0x30, 0, 局所のポインタ, 0) を確認（`xor edx,edx`、`xor r9,r9`。0x68 と同じ形） |
| 仕様（Learn） | [SystemParametersInfoW](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-systemparametersinfow) の SPI_GETWORKAREA（0x0030）: 「Retrieves the size of the work area on the primary display monitor. The work area is the portion of the screen not obscured by the system taskbar or by application desktop toolbars. The pvParam parameter must point to a RECT structure that receives the coordinates of the work area, expressed in physical pixel size」。値は書かない |
| 決定 | **決定 0065 が書いた条件のとおり**、RECT は `GetMonitorInfoW` の rcWork と同じ 4 つの数 = (0, 0, 1920, 1080)（`FLE_MONITOR_WORK_*` の名前。タスクバーがない）。RECT の 16 バイトだけを書き、1 を返す。last error は触れない。uiParam ≠ 0、fWinIni ≠ 0、pvParam NULL は 0x68 と同じく名前付き停止。引数は UINT（下位 32 ビット）。**0058 の本文は書き換えない**（この節が追記）。テスト `spi_getworkarea_is_the_same_rectangle_as_rcwork_of_getmonitorinfow` が 0065 の条件（一方を変えるなら両方）を固定する |
| 撤回条件 | (1) タスクバーを導入する決定（0065 の条件 2: rcWork、SPI_GETWORKAREA、SM_CXMAXIMIZED を一緒に決める）、(2) 実機の値が示されて整合しない |
| テスト | `wall_r.rs`: `spi_getworkarea_*` 4 件。既存の `spi.rs` の `LISTED` から 0x30 を外した |

## 8. GetSystemMetrics の index 94 = SM_DIGITIZER（unit 276）

| 項目 | 内容 |
|---|---|
| 呼び出し元の読み | `f_80e360` の 0x80e377（census が止めた呼び出し）。戻り値が 0 ならデジタイザの 2 つの bool（bit 0 = NID_INTEGRATED_TOUCH、bits 2、3 = ペン）を立てずに、非 0 なら立てて保存する。どちらの場合も続けて **SM_TABLETPC（86）、SM_CONVERTIBLESLATEMODE（0x2003）、SM_SYSTEMDOCKED（0x2004）**を問う（0057 が決めていない index） |
| 仕様（Learn） | [GetSystemMetrics](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-getsystemmetrics) の SM_DIGITIZER（94）: 「Nonzero if the current operating system is Windows 7 or Windows Server 2008 R2 and the Tablet PC Input service is started; otherwise, 0. The return value is a bitmask that specifies the type of digitizer input supported by the device」。Remarks の表（Learn が値を書く）: NID_INTEGRATED_TOUCH 0x01、NID_EXTERNAL_TOUCH 0x02、NID_INTEGRATED_PEN 0x04、NID_EXTERNAL_PEN 0x08、NID_MULTI_INPUT 0x40、NID_READY 0x80 |
| 決定 | **0**。この環境にはマウス（0057 の SM_MOUSEPRESENT = 1）だけで、タッチ・ペンのデジタイザはない（ビットが 1 つも立たない）。ページの文言（Windows 7 + Tablet PC Input service 以外は 0）にも、この環境の OS（Windows 10。0014）にも当てはまる読み。last error は触れない。rcx の下位 32 ビットが index。**同じ関数の 86、0x2003、0x2004 と隣の 95（SM_MAXIMUMTOUCHES）は決めず、名前付き停止のまま** |
| 帰結（実測） | unit 276 は 94 を通り、**次の壁は SM_TABLETPC（86）**。86 はページの文言（Windows XP Tablet PC edition、または Vista/7 + サービス、それ以外は 0）で 0 になると読めるが、その先の **0x2003（0 のときゲストはスレートモードの経路へ入る）と 0x2004（ドッキング）は値の根拠がなく PM の判断が要る**ので、86 だけ決めても unit は ok にならない。そのため本決定では 86 以降に触れない |
| 撤回条件 | (1) タッチ・ペンのデジタイザを持つ環境にする決定、(2) 実機の値が示されて整合しない、(3) 95 や 86 に実行が届き、94 = 0 との整合が要る（95 = 0 が自然） |
| テスト | `wall_r.rs`: `sm_digitizer_*`、`the_other_metrics_of_the_same_guest_function_*`。既存の `sysmetrics.rs` の `ANSWERED`（14 → 15）、`UNDECIDED`（22 → 21）を更新 |

## 実装した場所

- `lift/crates/fe-lift/src/imports_table.rs`（+23）: `TABLE` の末尾に 10 行（`acmMetrics`、`SHGetMalloc`、IMalloc の 4 行、`EnumDisplaySettingsW`、`GetClipCursor`、`EnumFontFamiliesExW`、`GetFileVersionInfoSizeW`）。IMalloc の 4 行は import ではなく vtable の行（`shell32.dll` の `IMalloc::Release`、`IMalloc::Free`、`IMalloc::DidAlloc`、`IMalloc::unimplemented`）。import の thunk からは名前を引けず、`fle_dispatch` が vtable の擬似 VA を関数に解くためだけの行（`GetProcAddress` で同じ名前を問えば見つかるが、ゲストは問わない）。
- `lift/runtime/fle_winmm.c/.h`（`acmMetrics`）、`fle_ole.c/.h`（`SHGetMalloc`、`fle_imp_IMalloc_*`、`fle_ole_shell_malloc_refs`）、`fle_monitor.c/.h`（`EnumDisplaySettingsW` と DEVMODEW の定数）、`fle_cursor.c/.h`（`GetClipCursor`）、`fle_gdi.c/.h`（`EnumFontFamiliesExW`。`fle_call_guest` は**弱参照**で宣言し、`fle_gdi.c` だけをリンクする既存のテストを壊さない）、`fle_vfs.c/.h`（`GetFileVersionInfoSizeW`）、`fle_spi.c/.h`（SPI_GETWORKAREA）、`fle_sysmetrics.c/.h`（SM_DIGITIZER）。**新しい `.c`、`.h` は無い**。
- 生成 C は、応答表の箇所と、6 つの import thunk の 1 命令（`fle_unsupported` → `fle_imp_*(); return;`）だけが変わる（下の「検証」）。

## テスト

新規 `lift/crates/fe-lift/tests/wall_r.rs` 31 件（期待値は決定と Learn から書いた）。1 つの C のドライバに必要な runtime をリンクして実行し、停止は SIGABRT のハンドラが「コールバック回数とウォッチしたバッファ」を出して検査する。内容: 各 API の戻り値、出力バッファのバイト列（前後の番兵つき）、last error、UINT 引数の上位ビット、異常入力の名前付き停止と無書き込み、IMalloc の vtable をたどる呼び出し（`fle_dispatch`）、参照カウント、`EnumFontFamiliesExW` の構造体 348 + 100 バイトの全バイトとスタック・レジスタの保存、合成 PE の生成器の出力（thunk と 10 行の位置・VA・dispatch 表）、生成 C と全 runtime をリンクした実行、ホストを読まないことの検査。

**変異検査**（実施。ソースを元に戻して `cmp` で一致を確認）: 13 件を入れ、13 件とも 1 件以上のテストが落ちた（acmMetrics の値 +1／metric 検査の除去、DidAlloc(NULL) の値、`*lpdwHandle` の 0 書き込みの除去、周波数 60 → 0、SPI の bottom −40、GetClipCursor の right −1、EnumFontFamiliesExW の int 化の除去／大文字小文字畳みの除去、Release の減算の除去、GetFileVersionInfoSizeW の error 番号の固定、EnumDisplaySettingsW の dmDriverExtra の保持の除去、SM_DIGITIZER = 1）。ログは `lift/out-wallR/logs/mut-*.txt`。

**既存テストの更新**（固定数・不在の主張が、この決定で偽になったもの。意味は変えず、理由を書いた）:

| ファイル | 行 | 理由 |
|---|---|---|
| `monitor.rs` | 不在の主張から `EnumDisplaySettingsW` を外した | 本決定が答える |
| `monitor.rs` | ソース検査: `DISPLAY` を `"DISPLAY"`（X11 の環境変数の文字列）に変更、`fle_stop(` 8 → 12、`R[0] =` 2 → 3 | `DM_DISPLAYFREQUENCY` などの SDK の名前が "DISPLAY" を含む。EnumDisplaySettingsW の停止 4 つと戻り値 1 つ |
| `spi.rs` | `LISTED` 8 → 7（0x30 を外す）、`WORKAREA` を追加、`fle_st(` の数 5 → 9 | SPI_GETWORKAREA を答える（RECT の 4 つの書き込み） |
| `sysmetrics.rs` | `ANSWERED` 14 → 15（94 → 0）、`UNDECIDED` 22 → 21 | SM_DIGITIZER を答える |
| `cursor.rs` | ソース検査「ゲストのメモリを読み書きしない」を、「`fle_ld(` なし、`fle_st(` はちょうど 4（GetClipCursor の RECT）」に変更 | GetClipCursor が RECT を書く |

「GetModuleHandleA が最後の行」を固定していたテストは、決定 0092 の時点で相対位置の検査に直っており、今回は壊れなかった。

## 検証

実測値と手順は `lift/out-wallR/docs-new/wall-r-measurement.md`、生の出力は `lift/out-wallR/`（`runs/`、`logs/`、`test-ws*.txt`）。

| 項目 | 結果 |
|---|---|
| FLEngine の生成 C の SHA-256 | **`feb5b9087f8d7a8a1282900212d7bd1f5b6071b0326e7149642f98c168e8080f` → `9d237ca0291c679d1cbdcebfb99b61539d955f937d6ad14ac4e73c24b14a35ca`**（215,709,232 → 215,711,386 バイト。functions 59,297、bad_nodes 0 のまま。unsupported_total 525 → 519）。旧 C は HEAD の `fe-lift` で作り、SHA が一致することを確認した上で `diff`（`lift/out-wallR/gen-diff.txt`）: 変わった箇所は 11 か所で、(1) 10 個の関数宣言、(2) `fle_dispatch_table` の 10 行と個数（59,565 → 59,575）、(3) `fle_resp_table` の 10 行と個数（268 → 278）、(4) **6 つの import thunk の 1 命令**（`fle_unsupported` → `fle_imp_*(); return;`）。**この 6 つの thunk 以外に関数本体の変化は 0** |
| census（dsp 構成。前 → 後） | ok **876 → 881**、stop 19 → 14、fault 0、hang 0。壁の種類 16 → 11。unit 170、235、246、272、275 が ok。276 → 次の壁 SM_TABLETPC（86）、283 → 次の壁 CompareStringW の ASCII の枠（0072）、880 → 次の壁 SHGetFolderPathW の NULL 呼び（読み）。「後」は 2 回測って経過時間を除き同一 |
| census（単独。前 → 後） | ok 873 → 877、stop 22 → 18（235、246、272、275 が ok。170 は unit 144 の放棄の二次の疑いで止まる） |
| census 無効時の a4 | 単独 **145/942**（functions 821、entries 15,685）、dsp 構成 **150/942**（functions 943、entries 26,585）。基準と**完全に一致** |
| `cargo test --workspace --no-fail-fast`（`CARGO_TARGET_DIR=lift/out-wallR/target`、`FLENGINE_PE` 指定、census 適用後の木） | 基準: HEAD 単体 98 binaries / 1,228 passed / 0 failed、census.diff で +1 binary（census.rs）/ +9 件 = 99 binaries / 1,237 passed。**この木で最後まで走った回: 100 binaries / 1,265 passed / 3 failed**（`test-ws-run2-3failed.txt`。基準 +1 binary = wall_r.rs、wall_r 31 件。3 件の失敗は `cursor.rs`、`gdi.rs`、`spi.rs` のソース検査で、本決定が足したコードで固定数・禁止語（`fle_st(` の数、"environ" を含む停止メッセージの文言）に当たったもの）。3 件を直し（`gdi`: 停止メッセージの語を変更、`spi`、`cursor`: 数の更新と理由）、`gdi` 49、`cursor` 25、`spi` 32、`monitor` 25、`wall_r` 31 を個別に再実行して**すべて 0 failed**（`logs/test-touched-3.txt`、`test-touched-2.txt`）。直した後の木（以降ソース無変更）の全体実行は、**ディスクが 100% になり（`/` の Avail 0、`ENOSPC`）、1 回では最後まで走れなかった**（`test-ws-run3-interrupted-disk-full.txt`、`test-ws-run4-interrupted-disk-full.txt`。mmx、modules、reg の失敗は `No space left on device` による）。そこで、空きを作った後に**同じ木で 2 つに分けて**実行した: (前半) `cargo test --workspace --no-fail-fast`（`test-ws.txt`。realmod までの **57 binaries / 708 passed / 0 failed**。この実行は reg で `ENOSPC` になり、それ以降の出力は残っていない）、(後半) fe-lift の `reg` から `x87ctl` までの 30 ターゲット、`fe-pe`、fe-decode と fe-lift のドキュメントテスト（`test-ws-tail.txt`。**43 binaries / 560 passed / 0 failed**、exit 0）。前半 + 後半 = **100 binaries / 1,268 passed / 0 failed / 0 ignored**（run2 の 1,265 passed + 3 failed = 1,268 と一致）。**1 回の `cargo test --workspace` で通したものではない**（2 分割） |
| 変異検査 | 13 件を入れ 13 件とも 1 件以上のテストが落ちた（ソースは `cmp` で元に戻した） |
| 応答表 | 末尾に 10 行。既存の行の擬似 VA は動かない（`the_six_thunks_*` が位置と VA の関係を固定） |
| 適用の確認 | `lift/out-wallR/patches/wallR.diff`（20 ファイル、1,794 行）は、HEAD **f1bcba7**（作業中に 5002d98 から進んだ）の `git archive` に census.diff を当てた木に `patch -p1 --dry-run` で当たる（`logs/verify-wallR-dry.txt`）。census が HEAD に入った後は、census.diff の分を除いて当たる |

## 解釈が分かれた点

1. **SHGetMalloc の成功・失敗**: 失敗（HRESULT エラー）で済ませる案は実装が小さいが、ページは常に成功しうる API として書き、実機では失敗しない。**成功 + IMalloc の実体**を採った。実体のために応答表に vtable の 4 行を足す（API 名でない行が表に入る）のが、この決定の最大の設計上の持ち出し。
2. **EnumFontFamiliesExW の family**: 1 family に決めたことで、ゲストがフォントの一覧を使う次の段階（リストの並び替え、検索）が実行される。family 名に空白があることで 0072 の枠に当たった（次の壁）。名前を変える選択はしなかった（0066 と矛盾する）。
3. **GetFileVersionInfoSizeW の失敗番号**: 2 か 3 かはページに無い。仮想ファイルシステムの検索の結果（ディレクトリが無ければ 3、あれば 2）にした。
4. **SM_TABLETPC（86）を一緒に決めなかった理由**: 86 は 0 と読めるが、同じ関数の 0x2003 と 0x2004 が決まらないので unit は ok にならない（PM の判断が要る）。値を増やさないために 94 だけにした。
5. **dmDisplayFrequency = 60**: 古い Learn の「0 または 1 は既定」という記憶は、今のページで確認できなかったので使わない。

## 限界

- **実機 Windows との照合は無い**。8 件の応答はすべて仮の決定で、特に acmMetrics の 18、dmDisplayFrequency の 60、TRUETYPE_FONTTYPE の扱い、SM_DIGITIZER の 0 は出典のない選択。
- 各 unit で見えるのは最初の壁だけ。ok になった 5 unit（170、235、246、272、275）は「戻った」だけで、**unit の中で fail-soft に進んだかを見ていない**（0060）。**census の数は A4 の進捗ではない**。
- `SHGetMalloc` のオブジェクトは Release が 0 になっても解放せず、次の SHGetMalloc が同じものを返す（Learn: 参照が 0 になれば実装は解放する）。古いポインタが生き続ける。IMalloc の QueryInterface、AddRef、Alloc、Realloc、GetSize、HeapMinimize は停止（ゲストの呼び出し点に無い）。
- `GetFileVersionInfoSizeW` は、仮想ファイルシステムに存在するパスでは停止する（ゲストが PE を置く経路は無い）。ゲスト自身の DLL を仮想ファイルとして置くと、この応答は撤回される。
- `EnumFontFamiliesExW` の ENUMLOGFONTEXW は lfFaceName、elfFullName、lfWeight 以外を 0 にしている（実機は elfStyle、elfScript、lfCharSet などを持つ）。ゲストのコールバックが読むのは lfFaceName だけ（読み）。
- 応答表に API 名でない行（`IMalloc::*`）を入れた。`GetProcAddress` で `shell32.dll` の `IMalloc::Release` を問えば見つかる。ゲストは問わない。
- 並行する他の担当（レーン S、L、遅延モジュール）も `TABLE` の末尾に行を足すため、**マージ時に `imports_table.rs` の末尾で衝突する**。行の順は結果に関係しない（擬似 VA は位置で決まるので、先にマージしたものから順に決まる。テストは位置の相対関係だけを見る）。
- wasm 版はビルドしていない（native のみ）。`fle_gdi.c` の弱参照の関数ポインタは wasm のリンクでは未確認。
- **作業環境の事実**（結果の信頼性に関わるので書く）: 作業の後半で `/` が 100% になった（`df`: 953G 使用、Avail 0。複数の担当が同時に大きなビルドを置いたため）。ファイルを削除せずに空きを作るため、(1) 自分の古いビルド（`lift/out-wallR/bld/src-pre-gdi-string/single`、1.2 GB。作り直し済み）を `/dev/shm/wallR-stale-single` へ `mv` し、(2) 自分の古いビルド（`bld/src-pre-gdi-string`、`bld/base` の 1 MB 超のファイル 158 個）を `gzip -1` で可逆に圧縮した（`logs/stale-gz-list.txt`、`logs/base-gz-list.txt`。元に戻すには `gunzip`）。census の「前」の再測定に使ったビルドは `bld/base`（圧縮済み）で、再実行するには `gunzip` が要る。ディスクが満杯の間に走ったテストの失敗（mmx、modules、reg）は `ENOSPC` による偽の失敗で、空きを作った後の実行で 0 failed を確認した。
