# 0062: LoadImageW(hInst = DLL, MAKEINTRESOURCE(32649), IMAGE_CURSOR, 32, 32, LR_SHARED) は NULL を返し、last error には触れない（仮の決定）

状態: **仮の決定**。PM が B 案を仮の決定として採用し、作業者（サブエージェント）が実装した。0061 は変更しない（0061 の「選択肢」節に「0062 で B 案に決定」と 1 行追記しただけ。本文は消していない）。「一次」は Microsoft Learn（2026-10-22 取得）、「実測」は DLL のバイト列とコマンド出力、「読み」は `lift/objdump-text.txt` の静的な読み、「仮」は根拠となるページが無い選択。実装は `lift/runtime/fle_loadimage.c`、`fle_loadimage.h`（`FLE_LI_MODULE_MISSING` の追加）。テストは `lift/crates/fe-lift/tests/loadimage.rs`。

## 背景

0061 の実装後、harness は f_8659f0 の 21 回ループの idx = -21（`LoadImageW(hInst = 0x400000, MAKEINTRESOURCE(32649), IMAGE_CURSOR, 32, 32, LR_SHARED)`）で止まっていた。DLL の RT_GROUP_CURSOR に 32649 は無い。実 Windows では LoadImageW は失敗して NULL を返すはずだが、Learn は失敗時の GetLastError の値を書かない。0061 は値を作らず停止を選んだ。

## 決定

1. 上の形（type = IMAGE_CURSOR、cx = cy = 32、fuLoad がちょうど LR_SHARED、名前が整数 32649、hInst が 0x400000）で、DLL の RT_GROUP_CURSOR に 32649（またはその RT_CURSOR コンポーネント）が無いとき、**戻り値 NULL（`R[0] = 0`）を返す。last error には触れない**（呼び出し前の値のまま）。戻り番地（`[rsp]`）は壊さない。
2. 変更はこの 1 つの形だけ。ほかはすべて 0061 のまま止まる: type・fuLoad・サイズの違い、文字列名、hInst が NULL でも 0x400000 でもない値、hInst NULL で 32649（標準 13 種ではない）、DLL に無い 32649 以外の id（32648、32650 など）、DLL に有るがゲストが読まない 32761。
3. DLL が 32649 を**持つ**場合は NULL を返さない（名前は 32762〜32767 ではないので、0061 の「ゲストが読まない id」として止まる）。NULL を返すのは「DLL に無いことを `fle_res_cursor_group_ok` が確かめた」ときだけ。

## 値と出典の区別

| 項目 | 内容 | 出典 | 状態 |
|---|---|---|---|
| 失敗時の戻り値が NULL | LoadImageW ページ「If the function fails, the return value is NULL」 | **一次**（Learn） | 確認済み |
| DLL に RT_GROUP_CURSOR 32649 が無い | 数値名の RT_GROUP_CURSOR は 32761〜32767 の 7 件だけ。RT_CURSOR は id 1〜60 | **実測**（fe-pe と pefile。テスト `the_dll_has_the_rt_group_cursors_32761_to_32767_and_not_32649`）。環境値ではない | 確定 |
| last error に触れない | 失敗時の GetLastError の値を Learn は書かない。値は作らない（1814 は採らない） | **仮**（ページに根拠が無い。「触れない」は「値を創作しない」ための選択） | **仮の決定** |
| 呼び出し直後の経路が GetLastError を読まない | 下の「静的な確認」 | 読み（静的）と実行時の確認（gdb） | 確認済み |

## 選択肢

- A. 停止のまま（0061 の状態）。**不採用**: 実機で起きる経路（ゲストは NULL を検査せずに保存し、後で `SetCursor(NULL)` に渡す）の先へ進めない。
- B. NULL を返し、last error に触れない（**採用**）。値を創作しない。
- C. NULL を返し 1814（`ERROR_RESOURCE_NAME_NOT_FOUND`、0025 の FindResourceW と同じ対応づけ）。**不採用**: 1814 は LoadImageW のページに書かれていない（FindResourceW の値からの類推）。ゲストが last error を読まないので観測差は出ないが、根拠のない値を入れない。

## 採用条件の静的な確認（GetLastError を読まないこと）

条件: idx = -21 の LoadImageW の直後から結果がリストに入るまでの経路で、GetLastError（kernel32 の thunk 0x407010、0x41cf80、0x423540）と SetLastError（0x423bd0）を読まない。

- 経路: f_8659f0（ループ。0x865a55 の `call 0x425100` の直後 0x865a5a〜0x865a71）→ f_865c50（`mov r8, rax` の結果を 20 バイトのノードに入れて `[obj+0x14c]` の先頭に追加。0x865c50〜0x865c88）。f_865c50 は `0x409410`（確保。`test rcx,rcx; jg` で 20 > 0 なので `[0x13a5c90]` の関数を呼ぶ。失敗時は `0x409730`）だけを呼ぶ。
- 直接 call の閉包（`lift/out-nc/work/closure.py`。.pdata の関数境界で、各関数の直接 call と関数をまたぐ jmp をたどる。IAT の thunk は jmp [IAT] の形で IAT スロットを解く）: f_8659f0 と f_865c50 から 40 関数、thunk 11 個（ExitProcess、FreeLibrary、TlsGetValue、TlsSetValue、LocalAlloc、GetCurrentThreadId、WriteFile、GetStdHandle、遅延 import の MessageBoxA、LoadCursorW、LoadImageW。**GetLastError、SetLastError は無い**）。間接呼び出しの先の候補（0x407a50 の 10 関数、0x406fd0、0x406fe0）を足した閉包（52 関数、thunk 15 個）にも、GetLastError、SetLastError は無い（足した thunk は Sleep、VirtualAlloc、RaiseException、RtlUnwind、遅延 import の GetLogicalProcessorInformation）。
- 追えない間接呼び出し（閉包の中の 15 か所）: `0x13a5c90` の関数ポインタ（.data の初期値 0x407a50、10 関数の閉包は thunk 2 個（Sleep、VirtualAlloc）で GetLastError なし）、`0x15e6018`、`0x15e6030`（0x406fd0 = RaiseException の thunk と書かれる。0x4121f4 と 0x41ce5e の `lea` → 書き込み）、`0x15e6040`、`0x15e6048`、`0x15e6068`、`0x15e63d8`、`0x15eaca8`、`0x15e60a0`（.bss。書き込み元は上の 2 か所以外に見つからない。0x15e6018、0x15e6040、0x15e6048、0x15e6068、0x15e63d8、0x15eaca8 は `call` と `cmp` だけで書き込みの命令が無い = 実行時にも未設定のまま）、`call rbx`（0x412929 = 0x15e60a0）、`call [rbp+0x38]`（0x412082）、`call [rbx-0x20]`（0x40fab5 = 仮想デストラクタ）、`call [rbx+0x8]`（0x41a607）、`jmp [rcx]`（0x412752）。**これらは静的には先を追えない**。ただし f_865c50 の閉包の静的な呼び出しはすべて確保とその失敗の報告（abort 系）の経路で、確保は成功する（確保が失敗しない限り下りない）。
- 追加の確認（実行時。`gdb` で native の `flengine-native` に `fle_imp_GetLastError`、`fle_imp_SetLastError`、`fle_imp_LoadImageW` のブレークポイント。`lift/out-nc/gdb-lasterror.txt`）: LoadImageW の 21 回の呼び出し（idx -22〜-2）の間にも直後にも GetLastError、SetLastError は一度も呼ばれない。GetLastError が呼ばれるのは LoadImageW より前の 2 回だけ。
- 結果の読み出し側（f_866060）は 1 関数、直接 call も間接 call も持たない（`xor eax,eax` から `[node+0xc]` の読み出しだけ）。f_865a80、f_865b20（破棄側）の閉包（136 関数）は GetLastError の thunk 0x407010 に届くが、経路は `f_8661d0 → 0x418210 → … → 0x41bb30 → 0x410650 → f_4104f0`（0x410536 の `call 0x407010`）で、f_4104f0 は GetModuleHandleW → GetProcAddress → 遅延 import の GetLogicalProcessorInformation を呼んだあとに GetLastError を読み、`cmp eax, 0x7a`（ERROR_INSUFFICIENT_BUFFER）で分岐する。呼び出し元の 0x410650 は `[0x15e6000]` が 0 のときだけ f_4104f0 を呼ぶ（`lock xchg` で保存する）初期化の形（読み）。これは破棄側の `f_8661d0`（`[obj+0x15c]` の表の検索。引数は idx だけで、LoadImageW が返した値は渡らない）の先にある確保器側の初期化で、**LoadImageW の NULL の有無とは無関係**（この読みの条件に cursor の値は入らない）。ただし破棄側が動くのは停止点より先。破棄側は idx = -21 のノードを NULL で検査して DestroyCursor を避ける（`cmp [rsi+0xc], 0`）。

結論: **idx = -21 の LoadImageW の直後から、その結果がリストに入るまでの経路は GetLastError を読まない**（読みと実行時の確認が一致）。B 案の条件を満たす。

限界: 追えない間接呼び出し 15 か所は、静的には先を決められない。.bss のポインタ 6 つは書き込みが見つからないので未設定、残りは仮想デストラクタとコールバックの実行時の値。実行時の確認は 1 回の実行（この harness の現在の停止点まで）に限る。停止点より先で、NULL を持つノードの経路がどこかで GetLastError を読む可能性は、読みでは除けない（撤回条件に回す）。

## 影響（読み。0061 §3 と同じ）

ゲストは NULL を検査せずに保存し、idx = -21 を引く呼び出し（`f_866060(obj, 0xffffffeb)` → SetCursor）は `SetCursor(NULL)` になる。Learn は「If this parameter is NULL, the cursor is removed from the screen」と書く。破棄側は NULL を検査するので DestroyCursor に NULL は渡らない（0x865abb、0x865b56）。0x814f2c のループは検査せずに DestroyCursor を呼ぶが、その要素がこのリストかは未確認（0061 と同じ）。

## 撤回条件

- 実 Windows で LoadImageW の失敗時の GetLastError が確認でき、ゲストがそれを読むと分かったとき（C 案への変更。1 行）。
- Learn が失敗時の値を書いたとき（その値にする）。
- ゲストが NULL の cursor を持つノードの経路で GetLastError を読むと分かったとき（A 案に戻す）。
- DLL が 32649 を持つ版に変わったとき（0062 の前提の実測が崩れる）。

## 確認できていないこと

- 実 Windows の LoadImageW が 32649 で本当に NULL を返すか、last error がどうなるか（実測していない。Learn は失敗時の NULL までしか書かない）。
- 停止点より先の実行で NULL のカーソルが SetCursor 以外に渡る先（0061 から変わらない。仮想呼び出し `[rbx+0x4a0]` の先は未追跡）。
