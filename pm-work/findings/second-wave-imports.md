# 第 2 波の DLL の import の全件(Issue #1538)

実エンジンの調査と、そこから得た表。前例は #1386(python312 の import)。この表は、内蔵のプラグインの DLL の import の全件と、runtime に無い物の分類。

## 1. 範囲と方法

- 対象: `inputs/fl-installer-files.tsv` の `Plugins\` の下の kind が dll の行 182 本(すべて `Plugins\Fruity\` の下)。DLL の実物は `lift/out-app/host-extra/plugins/Plugins/` に在る(182 本の実物を読んだ)。
- 読んだ import: 通常の import 表(IAT の枠は FirstThunk)と、遅延読み込みの import 表(IMAGE_DELAYLOAD_DESCRIPTOR)の両方。遅延の枠は `.data` に在り、fe-decode は名前を解決しない。
- 参照の有無: 各 DLL の逆アセンブル(`objdump -d`)で、IAT の枠を指す命令(RIP 相対の `# 0x<VA>`。`Slayer_Engine.dll` だけは 32 ビットで絶対アドレス)が 1 つでも在るかを見た。
- runtime の有無: `lift/crates/fe-lift/src/imports_table.rs` の TABLE と、プラグインの生成が `--extra-imports` で渡す `lift/harness/seeds/ship6-imports.txt` の行。両方に無いものを「無し」とした(ship6 は TABLE と重ならない。`set_extra` が重複を拒むため)。
- `Slayer_Engine.dll` は 32 ビット(pei-i386)。製品は x86-64 だけを lift するので、この DLL は対象の外。表には入れた(件数は下の合計に含む)。

## 2. 数

| 項目 | 数 |
|---|---|
| DLL の本数(import を持つ) | 182(import を持たない DLL は 0 本) |
| import の枠の総数 | 83688(通常 78337・遅延 5351) |
| 一意の `dll!名前` | 1218 |
| runtime に在る `dll!名前` | 695 |
| runtime に無い `dll!名前`(本表の行) | 523(枠で 8723 件:通常 6390・遅延 2333) |
| うち遅延読み込みだけの名前 | 43 |
| うち通常と遅延の両方の名前 | 4 |

分類ごと(本表の行):

| 分類 | 状態 | 件数 |
|---|---|---|
| (a) 実装する | 実装 | 13 |
| (a) 実装する | 未実装(窓・ファイル・通信・装置・COM の実体などが要る。名前つきの停止) | 497 |
| (b) 字形など、フォントの実物が要る | 名前つきの停止 | 12 |
| (c) 使われていない | 名前つきの停止 | 1 |

(a) の「未実装」は、Issue の「時間内に終わらない物」に当たる。窓・ファイル・通信などを要求するもの(実装しない)と、純粋な処理だが時間内に書かなかったもの(shlwapi の文字列、imagehlp の PE 読み、CRT の残りなど)を、理由の欄で分けてある。

## 3. 実装したもの(13 行。`lift/crates/fe-lift/src/imports_table.rs` の末尾に追加)

| dll!名前 | 実装の場所 | 仕様・注記 |
|---|---|---|
| `api-ms-win-crt-string-l1-1-0.dll!memset` | fle_imp_memset (既存の C を表に載せた) | 窓・ファイル・通信を要求しない。試験: `lift/crates/fe-lift/tests/second_wave_imports.rs` |
| `kernel32.dll!CreateMutexW` | fle_imp_CreateMutexW (既存の C を表に載せた) | 窓・ファイル・通信を要求しない。試験: `lift/crates/fe-lift/tests/second_wave_imports.rs` |
| `kernel32.dll!GetUserDefaultLangID` | fle_imp_GetUserDefaultLangID (新規。en-US の 0x0409) | 窓・ファイル・通信を要求しない。試験: `lift/crates/fe-lift/tests/second_wave_imports.rs` |
| `kernel32.dll!OpenFileMappingW` | fle_imp_OpenFileMappingW (既存の C を表に載せた。名前は見つからない) | 窓・ファイル・通信を要求しない。試験: `lift/crates/fe-lift/tests/second_wave_imports.rs` |
| `kernel32.dll!ReleaseMutex` | fle_imp_ReleaseMutex (既存の C を表に載せた) | 窓・ファイル・通信を要求しない。試験: `lift/crates/fe-lift/tests/second_wave_imports.rs` |
| `kernel32.dll!TryEnterCriticalSection` | fle_imp_TryEnterCriticalSection (新規) | 窓・ファイル・通信を要求しない。試験: `lift/crates/fe-lift/tests/second_wave_imports.rs` |
| `kernel32.dll!lstrcatW` | fle_imp_lstrcatW (新規) | 窓・ファイル・通信を要求しない。試験: `lift/crates/fe-lift/tests/second_wave_imports.rs` |
| `kernel32.dll!lstrcpyA` | fle_imp_lstrcpyA (新規) | 窓・ファイル・通信を要求しない。試験: `lift/crates/fe-lift/tests/second_wave_imports.rs` |
| `kernel32.dll!lstrcpynW` | fle_imp_lstrcpynW (新規) | 窓・ファイル・通信を要求しない。試験: `lift/crates/fe-lift/tests/second_wave_imports.rs` |
| `ole32.dll!StringFromGUID2` | fle_imp_StringFromGUID2 (新規) | 窓・ファイル・通信を要求しない。試験: `lift/crates/fe-lift/tests/second_wave_imports.rs` |
| `user32.dll!GetClassLongPtrW` | fle_imp_GetClassLongPtrW (新規。GCLP_HICON のみ。他の添字は名前つきの停止) | 窓・ファイル・通信を要求しない。試験: `lift/crates/fe-lift/tests/second_wave_imports.rs` |
| `vcruntime140.dll!memset` | fle_imp_memset (既存の C を表に載せた) | 窓・ファイル・通信を要求しない。試験: `lift/crates/fe-lift/tests/second_wave_imports.rs` |
| `winmm.dll!joyGetNumDevs` | fle_imp_joyGetNumDevs (新規。0 台) | 窓・ファイル・通信を要求しない。試験: `lift/crates/fe-lift/tests/second_wave_imports.rs` |

実装の注記:

- `GetClassLongPtrW` は GCLP_HICON(-14) だけを答える(窓の表と `fle_class.c` のクラスの hIcon。`SetClassLongPtrW` の決定 0119 の読み側)。他の添字は名前つきの停止。窓が表に無いときは 0 と 1400(`SetClassLongPtrW` と同じ選択)。試験は無効な窓だけ。有効な窓の経路は既存の窓の試験(`window.rs`・`wall_fr6.rs`)に任せる。
- `OpenFileMappingW` は、どの名前も見つからないとして 0 と ERROR_FILE_NOT_FOUND(2)。`fle_vmap.c` の既存の実装を表に載せただけ。
- `CreateMutexW`・`ReleaseMutex`・`memset`(2 つの CRT DLL)は、既存の C を表に載せただけ(`fle_event.c`・`fle_env.c`)。
- `joyGetNumDevs` は 0 台(ジョイスティックが無い仮想の機械)。`joyGetPosEx`・`joyGetDevCapsA` は未実装。

## 4. 名前つきの停止(実装しない物)

- 通常の import の枠(`call [枠]` と `jmp [枠]` の thunk)に、表の行が無いとき、生成された C は `fle_unsupported(VA)` の代わりに `fle_import_stop("dll!名前")` を出す。停止の文は `fle: import <dll>!<名前> is not modeled`(`lift/runtime/fle_runtime.h` の `fle_import_stop`)。
- 場所: `lift/crates/fe-lift/src/lib.rs` の Err の枝(`unmodelled_import`)。統計(`unsupported` の数)は変えない。
- なぜここか: `lift/crates/fe-wasmgen/src/lower.rs` の `call_indirect` と `TailImport` は、C に無い import を見ると「is not modelled by the C runtime」で関数の lift を諦め、その関数は C の経路に残る。したがって wasm 側では呼ばれず、停止は C の thunk で名前つきに出る。lower.rs は変えていない。
- 遅延読み込みの枠(本表の遅延だけの名前): fe-decode が枠の名前を解決しないので、名前つきの停止にはならない。GetProcAddress の経路で止まる(既存の挙動のまま)。このため、遅延だけの名前 43 件は、停止の名前ではなく「遅延読み込みの枠」と書いてある。
- 字形(b): `DrawTextW`(user32)と `GetTextMetricsW`(gdi32)は、表に行が在り、C の中で「not modelled (decision 0111)」と名前つきに止まる(既存。本表の外)。

## 5. 表(runtime に無い `dll!名前` の全件)

| DLL | dll!名前 | runtime の有無 | 種別 | 使う DLL(参照の在る / 全) | 分類 | 状態 | 実装の場所 または 停止の名前 | 理由 |
|---|---|---|---|---|---|---|---|---|
| advapi32.dll | `RegConnectRegistryW` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import advapi32.dll!RegConnectRegistryW is not modeled` | レジストリの A 版。仮想レジストリの W 版と同じ表で作れるが、未実装 |
| advapi32.dll | `RegCreateKeyA` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import advapi32.dll!RegCreateKeyA is not modeled` | レジストリの A 版。仮想レジストリの W 版と同じ表で作れるが、未実装 |
| advapi32.dll | `RegCreateKeyExA` | 無し | 通常 | 2/2 | (a) | 未実装 | `fle: import advapi32.dll!RegCreateKeyExA is not modeled` | レジストリの A 版。仮想レジストリの W 版と同じ表で作れるが、未実装 |
| advapi32.dll | `RegDeleteKeyA` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import advapi32.dll!RegDeleteKeyA is not modeled` | レジストリの A 版。仮想レジストリの W 版と同じ表で作れるが、未実装 |
| advapi32.dll | `RegDeleteKeyExW` | 無し | 遅延 | 22/22 | (a) | 未実装 | 遅延読み込みの枠: 名前つきの停止は未対応(GetProcAddress の経路) | レジストリの A 版。仮想レジストリの W 版と同じ表で作れるが、未実装 |
| advapi32.dll | `RegDeleteValueA` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import advapi32.dll!RegDeleteValueA is not modeled` | レジストリの A 版。仮想レジストリの W 版と同じ表で作れるが、未実装 |
| advapi32.dll | `RegGetValueA` | 無し | 通常 | 2/2 | (a) | 未実装 | `fle: import advapi32.dll!RegGetValueA is not modeled` | レジストリの A 版。仮想レジストリの W 版と同じ表で作れるが、未実装 |
| advapi32.dll | `RegLoadKeyW` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import advapi32.dll!RegLoadKeyW is not modeled` | レジストリの A 版。仮想レジストリの W 版と同じ表で作れるが、未実装 |
| advapi32.dll | `RegQueryValueExA` | 無し | 通常 | 20/20 | (a) | 未実装 | `fle: import advapi32.dll!RegQueryValueExA is not modeled` | レジストリの A 版。仮想レジストリの W 版と同じ表で作れるが、未実装 |
| advapi32.dll | `RegReplaceKeyW` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import advapi32.dll!RegReplaceKeyW is not modeled` | レジストリの A 版。仮想レジストリの W 版と同じ表で作れるが、未実装 |
| advapi32.dll | `RegRestoreKeyW` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import advapi32.dll!RegRestoreKeyW is not modeled` | レジストリの A 版。仮想レジストリの W 版と同じ表で作れるが、未実装 |
| advapi32.dll | `RegSaveKeyW` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import advapi32.dll!RegSaveKeyW is not modeled` | レジストリの A 版。仮想レジストリの W 版と同じ表で作れるが、未実装 |
| advapi32.dll | `RegSetValueExA` | 無し | 通常 | 3/3 | (a) | 未実装 | `fle: import advapi32.dll!RegSetValueExA is not modeled` | レジストリの A 版。仮想レジストリの W 版と同じ表で作れるが、未実装 |
| advapi32.dll | `RegUnLoadKeyW` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import advapi32.dll!RegUnLoadKeyW is not modeled` | レジストリの A 版。仮想レジストリの W 版と同じ表で作れるが、未実装 |
| api-ms-win-crt-heap-l1-1-0.dll | `_callnewh` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import api-ms-win-crt-heap-l1-1-0.dll!_callnewh is not modeled` | C ランタイムの残り(例外・初期化・ヒープ・終了)。未実装 |
| api-ms-win-crt-heap-l1-1-0.dll | `free` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import api-ms-win-crt-heap-l1-1-0.dll!free is not modeled` | C ランタイムの残り(例外・初期化・ヒープ・終了)。未実装 |
| api-ms-win-crt-heap-l1-1-0.dll | `malloc` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import api-ms-win-crt-heap-l1-1-0.dll!malloc is not modeled` | C ランタイムの残り(例外・初期化・ヒープ・終了)。未実装 |
| api-ms-win-crt-runtime-l1-1-0.dll | `_cexit` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import api-ms-win-crt-runtime-l1-1-0.dll!_cexit is not modeled` | C ランタイムの残り(例外・初期化・ヒープ・終了)。未実装 |
| api-ms-win-crt-runtime-l1-1-0.dll | `_configure_narrow_argv` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import api-ms-win-crt-runtime-l1-1-0.dll!_configure_narrow_argv is not modeled` | C ランタイムの残り(例外・初期化・ヒープ・終了)。未実装 |
| api-ms-win-crt-runtime-l1-1-0.dll | `_execute_onexit_table` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import api-ms-win-crt-runtime-l1-1-0.dll!_execute_onexit_table is not modeled` | C ランタイムの残り(例外・初期化・ヒープ・終了)。未実装 |
| api-ms-win-crt-runtime-l1-1-0.dll | `_initialize_narrow_environment` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import api-ms-win-crt-runtime-l1-1-0.dll!_initialize_narrow_environment is not modeled` | C ランタイムの残り(例外・初期化・ヒープ・終了)。未実装 |
| api-ms-win-crt-runtime-l1-1-0.dll | `_initialize_onexit_table` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import api-ms-win-crt-runtime-l1-1-0.dll!_initialize_onexit_table is not modeled` | C ランタイムの残り(例外・初期化・ヒープ・終了)。未実装 |
| api-ms-win-crt-runtime-l1-1-0.dll | `_initterm` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import api-ms-win-crt-runtime-l1-1-0.dll!_initterm is not modeled` | C ランタイムの残り(例外・初期化・ヒープ・終了)。未実装 |
| api-ms-win-crt-runtime-l1-1-0.dll | `_initterm_e` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import api-ms-win-crt-runtime-l1-1-0.dll!_initterm_e is not modeled` | C ランタイムの残り(例外・初期化・ヒープ・終了)。未実装 |
| api-ms-win-crt-runtime-l1-1-0.dll | `_invalid_parameter_noinfo_noreturn` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import api-ms-win-crt-runtime-l1-1-0.dll!_invalid_parameter_noinfo_noreturn is not modeled` | C ランタイムの残り(例外・初期化・ヒープ・終了)。未実装 |
| api-ms-win-crt-runtime-l1-1-0.dll | `_seh_filter_dll` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import api-ms-win-crt-runtime-l1-1-0.dll!_seh_filter_dll is not modeled` | C ランタイムの残り(例外・初期化・ヒープ・終了)。未実装 |
| api-ms-win-crt-runtime-l1-1-0.dll | `abort` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import api-ms-win-crt-runtime-l1-1-0.dll!abort is not modeled` | C ランタイムの残り(例外・初期化・ヒープ・終了)。未実装 |
| api-ms-win-crt-runtime-l1-1-0.dll | `terminate` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import api-ms-win-crt-runtime-l1-1-0.dll!terminate is not modeled` | C ランタイムの残り(例外・初期化・ヒープ・終了)。未実装 |
| api-ms-win-crt-string-l1-1-0.dll | `memset` | 無し | 通常 | 134/134 | (a) | 実装 | fle_imp_memset (既存の C を表に載せた) | 仕様どおりに実装(窓・ファイル・通信を要求しない) |
| comctl32.dll | `#345` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import comctl32.dll!#345 is not modeled` | 画像リストの描画の実物が要る |
| comctl32.dll | `ImageList_Copy` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import comctl32.dll!ImageList_Copy is not modeled` | 画像リストの描画の実物が要る |
| comctl32.dll | `ImageList_GetImageInfo` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import comctl32.dll!ImageList_GetImageInfo is not modeled` | 画像リストの描画の実物が要る |
| comctl32.dll | `ImageList_LoadImageW` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import comctl32.dll!ImageList_LoadImageW is not modeled` | 画像リストの描画の実物が要る |
| comctl32.dll | `ImageList_ReplaceIcon` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import comctl32.dll!ImageList_ReplaceIcon is not modeled` | 画像リストの描画の実物が要る |
| comctl32.dll | `ImageList_SetOverlayImage` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import comctl32.dll!ImageList_SetOverlayImage is not modeled` | 画像リストの描画の実物が要る |
| comdlg32.dll | `ChooseColorW` | 無し | 通常 | 2/2 | (a) | 未実装 | `fle: import comdlg32.dll!ChooseColorW is not modeled` | ダイアログ(窓)が要る |
| comdlg32.dll | `ChooseFontW` | 無し | 通常 | 2/2 | (a) | 未実装 | `fle: import comdlg32.dll!ChooseFontW is not modeled` | ダイアログ(窓)が要る |
| comdlg32.dll | `GetOpenFileNameW` | 無し | 通常 | 86/86 | (a) | 未実装 | `fle: import comdlg32.dll!GetOpenFileNameW is not modeled` | ダイアログ(窓)が要る |
| comdlg32.dll | `GetSaveFileNameW` | 無し | 通常 | 86/86 | (a) | 未実装 | `fle: import comdlg32.dll!GetSaveFileNameW is not modeled` | ダイアログ(窓)が要る |
| crypt32.dll | `CryptUnprotectData` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import crypt32.dll!CryptUnprotectData is not modeled` | 暗号の実体(DPAPI の鍵)が要る |
| d3d11.dll | `D3D11CreateDevice` | 無し | 通常 | 5/5 | (a) | 未実装 | `fle: import d3d11.dll!D3D11CreateDevice is not modeled` | 描画・入力の装置(Direct3D、DWM、テーマ、WIC、DirectInput)が要る |
| d3d9.dll | `Direct3DCreate9` | 無し | 通常 | 7/7 | (a) | 未実装 | `fle: import d3d9.dll!Direct3DCreate9 is not modeled` | 描画・入力の装置(Direct3D、DWM、テーマ、WIC、DirectInput)が要る |
| dbghelp.dll | `StackWalk64` | 無し | 通常 | 2/2 | (a) | 未実装 | `fle: import dbghelp.dll!StackWalk64 is not modeled` | デバッグ情報(PDB などの実物)が要る |
| dbghelp.dll | `SymFromAddr` | 無し | 通常 | 2/2 | (a) | 未実装 | `fle: import dbghelp.dll!SymFromAddr is not modeled` | デバッグ情報(PDB などの実物)が要る |
| dbghelp.dll | `SymFunctionTableAccess64` | 無し | 通常 | 2/2 | (a) | 未実装 | `fle: import dbghelp.dll!SymFunctionTableAccess64 is not modeled` | デバッグ情報(PDB などの実物)が要る |
| dbghelp.dll | `SymGetLineFromAddr64` | 無し | 通常 | 2/2 | (a) | 未実装 | `fle: import dbghelp.dll!SymGetLineFromAddr64 is not modeled` | デバッグ情報(PDB などの実物)が要る |
| dbghelp.dll | `SymGetModuleBase64` | 無し | 通常 | 2/2 | (a) | 未実装 | `fle: import dbghelp.dll!SymGetModuleBase64 is not modeled` | デバッグ情報(PDB などの実物)が要る |
| dbghelp.dll | `SymInitialize` | 無し | 通常 | 2/2 | (a) | 未実装 | `fle: import dbghelp.dll!SymInitialize is not modeled` | デバッグ情報(PDB などの実物)が要る |
| dinput8.dll | `DirectInput8Create` | 無し | 通常 | 0/2 | (c) | 名前つきの停止 | `fle: import dinput8.dll!DirectInput8Create is not modeled` | この DLL のコードに、この IAT の枠を指す命令が無い。呼ばれない |
| dxgi.dll | `CreateDXGIFactory1` | 無し | 通常 | 2/2 | (a) | 未実装 | `fle: import dxgi.dll!CreateDXGIFactory1 is not modeled` | 描画・入力の装置(Direct3D、DWM、テーマ、WIC、DirectInput)が要る |
| gdi32.dll | `AbortDoc` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import gdi32.dll!AbortDoc is not modeled` | 印刷の装置・メタファイルの実物が要る |
| gdi32.dll | `ChoosePixelFormat` | 無し | 通常 | 9/9 | (a) | 未実装 | `fle: import gdi32.dll!ChoosePixelFormat is not modeled` | OpenGL の描画面が要る |
| gdi32.dll | `CloseEnhMetaFile` | 無し | 通常 | 3/3 | (a) | 未実装 | `fle: import gdi32.dll!CloseEnhMetaFile is not modeled` | 印刷の装置・メタファイルの実物が要る |
| gdi32.dll | `CreateDCW` | 無し | 通常 | 43/43 | (a) | 未実装 | `fle: import gdi32.dll!CreateDCW is not modeled` | 印刷の装置・メタファイルの実物が要る |
| gdi32.dll | `CreateEnhMetaFileW` | 無し | 通常 | 3/3 | (a) | 未実装 | `fle: import gdi32.dll!CreateEnhMetaFileW is not modeled` | 印刷の装置・メタファイルの実物が要る |
| gdi32.dll | `CreateFontW` | 無し | 通常 | 1/1 | (b) | 名前つきの停止 | `fle: import gdi32.dll!CreateFontW is not modeled` | フォントの実物(字形・計量)が要る |
| gdi32.dll | `CreateICW` | 無し | 通常 | 43/43 | (a) | 未実装 | `fle: import gdi32.dll!CreateICW is not modeled` | 印刷の装置・メタファイルの実物が要る |
| gdi32.dll | `EndDoc` | 無し | 通常 | 43/43 | (a) | 未実装 | `fle: import gdi32.dll!EndDoc is not modeled` | 印刷の装置・メタファイルの実物が要る |
| gdi32.dll | `EndPage` | 無し | 通常 | 43/43 | (a) | 未実装 | `fle: import gdi32.dll!EndPage is not modeled` | 印刷の装置・メタファイルの実物が要る |
| gdi32.dll | `EnumFontsW` | 無し | 通常 | 43/43 | (b) | 名前つきの停止 | `fle: import gdi32.dll!EnumFontsW is not modeled` | フォントの実物(字形・計量)が要る |
| gdi32.dll | `GetBkColor` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import gdi32.dll!GetBkColor is not modeled` | 描画の DC・GDI オブジェクトの実物が要る |
| gdi32.dll | `GetCurrentObject` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import gdi32.dll!GetCurrentObject is not modeled` | 描画の DC・GDI オブジェクトの実物が要る |
| gdi32.dll | `GetGlyphIndicesW` | 無し | 通常 | 1/1 | (b) | 名前つきの停止 | `fle: import gdi32.dll!GetGlyphIndicesW is not modeled` | フォントの実物(字形・計量)が要る |
| gdi32.dll | `GetGlyphOutlineW` | 無し | 通常 | 1/1 | (b) | 名前つきの停止 | `fle: import gdi32.dll!GetGlyphOutlineW is not modeled` | フォントの実物(字形・計量)が要る |
| gdi32.dll | `GetKerningPairsW` | 無し | 通常 | 1/1 | (b) | 名前つきの停止 | `fle: import gdi32.dll!GetKerningPairsW is not modeled` | フォントの実物(字形・計量)が要る |
| gdi32.dll | `GetObjectA` | 無し | 通常 | 7/7 | (a) | 未実装 | `fle: import gdi32.dll!GetObjectA is not modeled` | 描画の DC・GDI オブジェクトの実物が要る |
| gdi32.dll | `GetOutlineTextMetricsW` | 無し | 通常 | 1/1 | (b) | 名前つきの停止 | `fle: import gdi32.dll!GetOutlineTextMetricsW is not modeled` | フォントの実物(字形・計量)が要る |
| gdi32.dll | `GetPixelFormat` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import gdi32.dll!GetPixelFormat is not modeled` | OpenGL の描画面が要る |
| gdi32.dll | `GetTextAlign` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import gdi32.dll!GetTextAlign is not modeled` | 描画の DC・GDI オブジェクトの実物が要る |
| gdi32.dll | `GetTextColor` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import gdi32.dll!GetTextColor is not modeled` | 描画の DC・GDI オブジェクトの実物が要る |
| gdi32.dll | `GetTextExtentPoint32A` | 無し | 通常 | 1/1 | (b) | 名前つきの停止 | `fle: import gdi32.dll!GetTextExtentPoint32A is not modeled` | フォントの実物(字形・計量)が要る |
| gdi32.dll | `RemoveFontMemResourceEx` | 無し | 通常 | 1/1 | (b) | 名前つきの停止 | `fle: import gdi32.dll!RemoveFontMemResourceEx is not modeled` | フォントの実物(字形・計量)が要る |
| gdi32.dll | `SetAbortProc` | 無し | 通常 | 41/41 | (a) | 未実装 | `fle: import gdi32.dll!SetAbortProc is not modeled` | 印刷の装置・メタファイルの実物が要る |
| gdi32.dll | `SetMapperFlags` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import gdi32.dll!SetMapperFlags is not modeled` | 描画の DC・GDI オブジェクトの実物が要る |
| gdi32.dll | `SetPixelFormat` | 無し | 通常 | 9/9 | (a) | 未実装 | `fle: import gdi32.dll!SetPixelFormat is not modeled` | OpenGL の描画面が要る |
| gdi32.dll | `SetTextAlign` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import gdi32.dll!SetTextAlign is not modeled` | 描画の DC・GDI オブジェクトの実物が要る |
| gdi32.dll | `StartDocW` | 無し | 通常 | 41/41 | (a) | 未実装 | `fle: import gdi32.dll!StartDocW is not modeled` | 印刷の装置・メタファイルの実物が要る |
| gdi32.dll | `StartPage` | 無し | 通常 | 41/41 | (a) | 未実装 | `fle: import gdi32.dll!StartPage is not modeled` | 印刷の装置・メタファイルの実物が要る |
| gdi32.dll | `SwapBuffers` | 無し | 通常 | 8/8 | (a) | 未実装 | `fle: import gdi32.dll!SwapBuffers is not modeled` | OpenGL の描画面が要る |
| gdi32.dll | `TextOutW` | 無し | 通常 | 73/73 | (b) | 名前つきの停止 | `fle: import gdi32.dll!TextOutW is not modeled` | フォントの実物(字形・計量)が要る |
| gdiplus.dll | `GdipAlloc` | 無し | 遅延 | 1/1 | (a) | 未実装 | 遅延読み込みの枠: 名前つきの停止は未対応(GetProcAddress の経路) | 描画の実物(GDI+)が要る |
| gdiplus.dll | `GdipBitmapLockBits` | 無し | 遅延 | 1/1 | (a) | 未実装 | 遅延読み込みの枠: 名前つきの停止は未対応(GetProcAddress の経路) | 描画の実物(GDI+)が要る |
| gdiplus.dll | `GdipBitmapUnlockBits` | 無し | 遅延 | 1/1 | (a) | 未実装 | 遅延読み込みの枠: 名前つきの停止は未対応(GetProcAddress の経路) | 描画の実物(GDI+)が要る |
| gdiplus.dll | `GdipCloneBrush` | 無し | 遅延 | 1/1 | (a) | 未実装 | 遅延読み込みの枠: 名前つきの停止は未対応(GetProcAddress の経路) | 描画の実物(GDI+)が要る |
| gdiplus.dll | `GdipCreateBitmapFromFile` | 無し | 遅延 | 1/1 | (a) | 未実装 | 遅延読み込みの枠: 名前つきの停止は未対応(GetProcAddress の経路) | 描画の実物(GDI+)が要る |
| gdiplus.dll | `GdipCreateBitmapFromFileICM` | 無し | 遅延 | 1/1 | (a) | 未実装 | 遅延読み込みの枠: 名前つきの停止は未対応(GetProcAddress の経路) | 描画の実物(GDI+)が要る |
| gdiplus.dll | `GdipDeleteBrush` | 無し | 遅延 | 1/1 | (a) | 未実装 | 遅延読み込みの枠: 名前つきの停止は未対応(GetProcAddress の経路) | 描画の実物(GDI+)が要る |
| gdiplus.dll | `GdipDeleteCachedBitmap` | 無し | 遅延 | 1/1 | (a) | 未実装 | 遅延読み込みの枠: 名前つきの停止は未対応(GetProcAddress の経路) | 描画の実物(GDI+)が要る |
| gdiplus.dll | `GdipDeleteCustomLineCap` | 無し | 遅延 | 1/1 | (a) | 未実装 | 遅延読み込みの枠: 名前つきの停止は未対応(GetProcAddress の経路) | 描画の実物(GDI+)が要る |
| gdiplus.dll | `GdipDeleteFont` | 無し | 遅延 | 1/1 | (b) | 名前つきの停止 | 遅延読み込みの枠: 名前つきの停止は未対応(GetProcAddress の経路) | フォントの実物(字形・計量)が要る |
| gdiplus.dll | `GdipDeleteFontFamily` | 無し | 遅延 | 1/1 | (b) | 名前つきの停止 | 遅延読み込みの枠: 名前つきの停止は未対応(GetProcAddress の経路) | フォントの実物(字形・計量)が要る |
| gdiplus.dll | `GdipDeleteGraphics` | 無し | 遅延 | 1/1 | (a) | 未実装 | 遅延読み込みの枠: 名前つきの停止は未対応(GetProcAddress の経路) | 描画の実物(GDI+)が要る |
| gdiplus.dll | `GdipDeleteMatrix` | 無し | 遅延 | 1/1 | (a) | 未実装 | 遅延読み込みの枠: 名前つきの停止は未対応(GetProcAddress の経路) | 描画の実物(GDI+)が要る |
| gdiplus.dll | `GdipDeletePath` | 無し | 遅延 | 1/1 | (a) | 未実装 | 遅延読み込みの枠: 名前つきの停止は未対応(GetProcAddress の経路) | 描画の実物(GDI+)が要る |
| gdiplus.dll | `GdipDeletePen` | 無し | 遅延 | 1/1 | (a) | 未実装 | 遅延読み込みの枠: 名前つきの停止は未対応(GetProcAddress の経路) | 描画の実物(GDI+)が要る |
| gdiplus.dll | `GdipDeleteRegion` | 無し | 遅延 | 1/1 | (a) | 未実装 | 遅延読み込みの枠: 名前つきの停止は未対応(GetProcAddress の経路) | 描画の実物(GDI+)が要る |
| gdiplus.dll | `GdipDeleteStringFormat` | 無し | 遅延 | 1/1 | (a) | 未実装 | 遅延読み込みの枠: 名前つきの停止は未対応(GetProcAddress の経路) | 描画の実物(GDI+)が要る |
| gdiplus.dll | `GdipDisposeImage` | 無し | 遅延 | 1/1 | (a) | 未実装 | 遅延読み込みの枠: 名前つきの停止は未対応(GetProcAddress の経路) | 描画の実物(GDI+)が要る |
| gdiplus.dll | `GdipDisposeImageAttributes` | 無し | 遅延 | 1/1 | (a) | 未実装 | 遅延読み込みの枠: 名前つきの停止は未対応(GetProcAddress の経路) | 描画の実物(GDI+)が要る |
| gdiplus.dll | `GdipFree` | 無し | 遅延 | 1/1 | (a) | 未実装 | 遅延読み込みの枠: 名前つきの停止は未対応(GetProcAddress の経路) | 描画の実物(GDI+)が要る |
| gdiplus.dll | `GdipGetImageHeight` | 無し | 遅延 | 1/1 | (a) | 未実装 | 遅延読み込みの枠: 名前つきの停止は未対応(GetProcAddress の経路) | 描画の実物(GDI+)が要る |
| gdiplus.dll | `GdipGetImageWidth` | 無し | 遅延 | 1/1 | (a) | 未実装 | 遅延読み込みの枠: 名前つきの停止は未対応(GetProcAddress の経路) | 描画の実物(GDI+)が要る |
| gdiplus.dll | `GdiplusShutdown` | 無し | 遅延 | 1/1 | (a) | 未実装 | 遅延読み込みの枠: 名前つきの停止は未対応(GetProcAddress の経路) | 描画の実物(GDI+)が要る |
| gdiplus.dll | `GdiplusStartup` | 無し | 遅延 | 1/1 | (a) | 未実装 | 遅延読み込みの枠: 名前つきの停止は未対応(GetProcAddress の経路) | 描画の実物(GDI+)が要る |
| imagehlp.dll | `ImageDirectoryEntryToData` | 無し | 通常 | 36/36 | (a) | 未実装 | `fle: import imagehlp.dll!ImageDirectoryEntryToData is not modeled` | PE 画像の読み取り関数。未実装 |
| imagehlp.dll | `ImageRvaToVa` | 無し | 通常 | 16/16 | (a) | 未実装 | `fle: import imagehlp.dll!ImageRvaToVa is not modeled` | PE 画像の読み取り関数。未実装 |
| imm32.dll | `ImmGetCompositionStringW` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import imm32.dll!ImmGetCompositionStringW is not modeled` | IME(窓の入力)が要る |
| imm32.dll | `ImmNotifyIME` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import imm32.dll!ImmNotifyIME is not modeled` | IME(窓の入力)が要る |
| imm32.dll | `ImmSetCandidateWindow` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import imm32.dll!ImmSetCandidateWindow is not modeled` | IME(窓の入力)が要る |
| iphlpapi.dll | `CancelIPChangeNotify` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import iphlpapi.dll!CancelIPChangeNotify is not modeled` | ネットワーク・セッション・資源の実物が要る(外の世界) |
| iphlpapi.dll | `ConvertLengthToIpv4Mask` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import iphlpapi.dll!ConvertLengthToIpv4Mask is not modeled` | ネットワーク・セッション・資源の実物が要る(外の世界) |
| iphlpapi.dll | `GetAdaptersAddresses` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import iphlpapi.dll!GetAdaptersAddresses is not modeled` | ネットワーク・セッション・資源の実物が要る(外の世界) |
| iphlpapi.dll | `GetAdaptersInfo` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import iphlpapi.dll!GetAdaptersInfo is not modeled` | ネットワーク・セッション・資源の実物が要る(外の世界) |
| iphlpapi.dll | `NotifyAddrChange` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import iphlpapi.dll!NotifyAddrChange is not modeled` | ネットワーク・セッション・資源の実物が要る(外の世界) |
| kernel32.dll | `AddVectoredExceptionHandler` | 無し | 通常 | 37/37 | (a) | 未実装 | `fle: import kernel32.dll!AddVectoredExceptionHandler is not modeled` | 例外・デバッグ・装置の経路が要る(未実装) |
| kernel32.dll | `AllocConsole` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import kernel32.dll!AllocConsole is not modeled` | コンソールが要る |
| kernel32.dll | `AreFileApisANSI` | 無し | 通常 | 4/4 | (a) | 未実装 | `fle: import kernel32.dll!AreFileApisANSI is not modeled` | ファイル・ボリューム・ディレクトリの実物が要る |
| kernel32.dll | `AssignProcessToJobObject` | 無し | 通常 | 24/24 | (a) | 未実装 | `fle: import kernel32.dll!AssignProcessToJobObject is not modeled` | プロセス・スレッド・待ち合わせ・タイマーの実物が要る |
| kernel32.dll | `Beep` | 無し | 通常 | 2/2 | (a) | 未実装 | `fle: import kernel32.dll!Beep is not modeled` | 例外・デバッグ・装置の経路が要る(未実装) |
| kernel32.dll | `CopyFileA` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import kernel32.dll!CopyFileA is not modeled` | ファイル・ボリューム・ディレクトリの実物が要る |
| kernel32.dll | `CreateDirectoryA` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import kernel32.dll!CreateDirectoryA is not modeled` | ファイル・ボリューム・ディレクトリの実物が要る |
| kernel32.dll | `CreateEventA` | 無し | 通常 | 2/2 | (a) | 未実装 | `fle: import kernel32.dll!CreateEventA is not modeled` | プロセス・スレッド・待ち合わせ・タイマーの実物が要る |
| kernel32.dll | `CreateEventExW` | 無し | 通常 | 3/3 | (a) | 未実装 | `fle: import kernel32.dll!CreateEventExW is not modeled` | プロセス・スレッド・待ち合わせ・タイマーの実物が要る |
| kernel32.dll | `CreateFileMappingA` | 無し | 通常 | 2/2 | (a) | 未実装 | `fle: import kernel32.dll!CreateFileMappingA is not modeled` | ファイル・ボリューム・ディレクトリの実物が要る |
| kernel32.dll | `CreateIoCompletionPort` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import kernel32.dll!CreateIoCompletionPort is not modeled` | OS の実値が要る(未実装) |
| kernel32.dll | `CreateJobObjectW` | 無し | 通常 | 2/2 | (a) | 未実装 | `fle: import kernel32.dll!CreateJobObjectW is not modeled` | プロセス・スレッド・待ち合わせ・タイマーの実物が要る |
| kernel32.dll | `CreateMutexA` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import kernel32.dll!CreateMutexA is not modeled` | プロセス・スレッド・待ち合わせ・タイマーの実物が要る |
| kernel32.dll | `CreateMutexW` | 無し | 通常 | 4/4 | (a) | 実装 | fle_imp_CreateMutexW (既存の C を表に載せた) | 仕様どおりに実装(窓・ファイル・通信を要求しない) |
| kernel32.dll | `CreateProcessW` | 無し | 通常 | 24/24 | (a) | 未実装 | `fle: import kernel32.dll!CreateProcessW is not modeled` | プロセス・スレッド・待ち合わせ・タイマーの実物が要る |
| kernel32.dll | `CreateSemaphoreA` | 無し | 通常 | 2/2 | (a) | 未実装 | `fle: import kernel32.dll!CreateSemaphoreA is not modeled` | プロセス・スレッド・待ち合わせ・タイマーの実物が要る |
| kernel32.dll | `CreateSemaphoreW` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import kernel32.dll!CreateSemaphoreW is not modeled` | プロセス・スレッド・待ち合わせ・タイマーの実物が要る |
| kernel32.dll | `CreateTimerQueue` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import kernel32.dll!CreateTimerQueue is not modeled` | プロセス・スレッド・待ち合わせ・タイマーの実物が要る |
| kernel32.dll | `CreateToolhelp32Snapshot` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import kernel32.dll!CreateToolhelp32Snapshot is not modeled` | OS の実値が要る(未実装) |
| kernel32.dll | `CreateWaitableTimerW` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import kernel32.dll!CreateWaitableTimerW is not modeled` | プロセス・スレッド・待ち合わせ・タイマーの実物が要る |
| kernel32.dll | `DeleteFileA` | 無し | 通常 | 4/4 | (a) | 未実装 | `fle: import kernel32.dll!DeleteFileA is not modeled` | ファイル・ボリューム・ディレクトリの実物が要る |
| kernel32.dll | `DeleteTimerQueueEx` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import kernel32.dll!DeleteTimerQueueEx is not modeled` | プロセス・スレッド・待ち合わせ・タイマーの実物が要る |
| kernel32.dll | `DeviceIoControl` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import kernel32.dll!DeviceIoControl is not modeled` | OS の実値が要る(未実装) |
| kernel32.dll | `DisableThreadLibraryCalls` | 無し | 通常 | 2/2 | (a) | 未実装 | `fle: import kernel32.dll!DisableThreadLibraryCalls is not modeled` | プロセス・スレッド・待ち合わせ・タイマーの実物が要る |
| kernel32.dll | `EnumCalendarInfoW` | 無し | 通常 | 134/134 | (a) | 未実装 | `fle: import kernel32.dll!EnumCalendarInfoW is not modeled` | ロケール・時刻の表が要る(未実装) |
| kernel32.dll | `FindFirstFileA` | 無し | 通常 | 11/11 | (a) | 未実装 | `fle: import kernel32.dll!FindFirstFileA is not modeled` | ファイル・ボリューム・ディレクトリの実物が要る |
| kernel32.dll | `FindFirstFileExA` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import kernel32.dll!FindFirstFileExA is not modeled` | ファイル・ボリューム・ディレクトリの実物が要る |
| kernel32.dll | `FindNextFileA` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import kernel32.dll!FindNextFileA is not modeled` | ファイル・ボリューム・ディレクトリの実物が要る |
| kernel32.dll | `FindResourceA` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import kernel32.dll!FindResourceA is not modeled` | ファイル・ボリューム・ディレクトリの実物が要る |
| kernel32.dll | `FlushViewOfFile` | 無し | 通常 | 3/3 | (a) | 未実装 | `fle: import kernel32.dll!FlushViewOfFile is not modeled` | ファイル・ボリューム・ディレクトリの実物が要る |
| kernel32.dll | `FormatMessageA` | 無し | 通常 | 6/6 | (a) | 未実装 | `fle: import kernel32.dll!FormatMessageA is not modeled` | ロケール・時刻の表が要る(未実装) |
| kernel32.dll | `FreeConsole` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import kernel32.dll!FreeConsole is not modeled` | コンソールが要る |
| kernel32.dll | `FreeLibraryAndExitThread` | 無し | 通常 | 5/5 | (a) | 未実装 | `fle: import kernel32.dll!FreeLibraryAndExitThread is not modeled` | プロセス・スレッド・待ち合わせ・タイマーの実物が要る |
| kernel32.dll | `GetCPInfoExW` | 無し | 通常 | 134/134 | (a) | 未実装 | `fle: import kernel32.dll!GetCPInfoExW is not modeled` | ロケール・時刻の表が要る(未実装) |
| kernel32.dll | `GetComputerNameA` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import kernel32.dll!GetComputerNameA is not modeled` | ファイル・ボリューム・ディレクトリの実物が要る |
| kernel32.dll | `GetConsoleWindow` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import kernel32.dll!GetConsoleWindow is not modeled` | コンソールが要る |
| kernel32.dll | `GetDiskFreeSpaceA` | 無し | 通常 | 3/3 | (a) | 未実装 | `fle: import kernel32.dll!GetDiskFreeSpaceA is not modeled` | ファイル・ボリューム・ディレクトリの実物が要る |
| kernel32.dll | `GetEnvironmentVariableA` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import kernel32.dll!GetEnvironmentVariableA is not modeled` | OS の実値が要る(未実装) |
| kernel32.dll | `GetErrorMode` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import kernel32.dll!GetErrorMode is not modeled` | 例外・デバッグ・装置の経路が要る(未実装) |
| kernel32.dll | `GetExitCodeProcess` | 無し | 通常 | 27/27 | (a) | 未実装 | `fle: import kernel32.dll!GetExitCodeProcess is not modeled` | プロセス・スレッド・待ち合わせ・タイマーの実物が要る |
| kernel32.dll | `GetFileAttributesA` | 無し | 通常 | 3/3 | (a) | 未実装 | `fle: import kernel32.dll!GetFileAttributesA is not modeled` | ファイル・ボリューム・ディレクトリの実物が要る |
| kernel32.dll | `GetFinalPathNameByHandleW` | 無し | 遅延 | 32/32 | (a) | 未実装 | 遅延読み込みの枠: 名前つきの停止は未対応(GetProcAddress の経路) | ファイル・ボリューム・ディレクトリの実物が要る |
| kernel32.dll | `GetFullPathNameA` | 無し | 通常 | 3/3 | (a) | 未実装 | `fle: import kernel32.dll!GetFullPathNameA is not modeled` | ファイル・ボリューム・ディレクトリの実物が要る |
| kernel32.dll | `GetLogicalDriveStringsA` | 無し | 通常 | 2/2 | (a) | 未実装 | `fle: import kernel32.dll!GetLogicalDriveStringsA is not modeled` | ファイル・ボリューム・ディレクトリの実物が要る |
| kernel32.dll | `GetLongPathNameW` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import kernel32.dll!GetLongPathNameW is not modeled` | ファイル・ボリューム・ディレクトリの実物が要る |
| kernel32.dll | `GetModuleHandleExA` | 無し | 通常 | 2/2 | (a) | 未実装 | `fle: import kernel32.dll!GetModuleHandleExA is not modeled` | ファイル・ボリューム・ディレクトリの実物が要る |
| kernel32.dll | `GetNumaHighestNodeNumber` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import kernel32.dll!GetNumaHighestNodeNumber is not modeled` | メモリ管理の残り(未実装) |
| kernel32.dll | `GetOverlappedResult` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import kernel32.dll!GetOverlappedResult is not modeled` | ファイル・ボリューム・ディレクトリの実物が要る |
| kernel32.dll | `GetQueuedCompletionStatus` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import kernel32.dll!GetQueuedCompletionStatus is not modeled` | プロセス・スレッド・待ち合わせ・タイマーの実物が要る |
| kernel32.dll | `GetShortPathNameW` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import kernel32.dll!GetShortPathNameW is not modeled` | ファイル・ボリューム・ディレクトリの実物が要る |
| kernel32.dll | `GetSystemPowerStatus` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import kernel32.dll!GetSystemPowerStatus is not modeled` | OS の実値が要る(未実装) |
| kernel32.dll | `GetSystemTimes` | 無し | 遅延 | 1/1 | (a) | 未実装 | 遅延読み込みの枠: 名前つきの停止は未対応(GetProcAddress の経路) | ロケール・時刻の表が要る(未実装) |
| kernel32.dll | `GetTempPathA` | 無し | 通常 | 4/4 | (a) | 未実装 | `fle: import kernel32.dll!GetTempPathA is not modeled` | ファイル・ボリューム・ディレクトリの実物が要る |
| kernel32.dll | `GetThreadTimes` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import kernel32.dll!GetThreadTimes is not modeled` | プロセス・スレッド・待ち合わせ・タイマーの実物が要る |
| kernel32.dll | `GetTimeFormatW` | 無し | 通常 | 21/21 | (a) | 未実装 | `fle: import kernel32.dll!GetTimeFormatW is not modeled` | ロケール・時刻の表が要る(未実装) |
| kernel32.dll | `GetUserDefaultLangID` | 無し | 通常 | 1/1 | (a) | 実装 | fle_imp_GetUserDefaultLangID (新規。en-US の 0x0409) | 仕様どおりに実装(窓・ファイル・通信を要求しない) |
| kernel32.dll | `GetVolumeInformationA` | 無し | 通常 | 2/2 | (a) | 未実装 | `fle: import kernel32.dll!GetVolumeInformationA is not modeled` | ファイル・ボリューム・ディレクトリの実物が要る |
| kernel32.dll | `GlobalDeleteAtom` | 無し | 通常 | 134/134 | (a) | 未実装 | `fle: import kernel32.dll!GlobalDeleteAtom is not modeled` | メモリ管理の残り(未実装) |
| kernel32.dll | `GlobalHandle` | 無し | 通常 | 2/2 | (a) | 未実装 | `fle: import kernel32.dll!GlobalHandle is not modeled` | メモリ管理の残り(未実装) |
| kernel32.dll | `GlobalMemoryStatusEx` | 無し | 通常 | 2/2 | (a) | 未実装 | `fle: import kernel32.dll!GlobalMemoryStatusEx is not modeled` | メモリ管理の残り(未実装) |
| kernel32.dll | `GlobalSize` | 無し | 通常 | 139/139 | (a) | 未実装 | `fle: import kernel32.dll!GlobalSize is not modeled` | メモリ管理の残り(未実装) |
| kernel32.dll | `HeapCompact` | 無し | 通常 | 3/3 | (a) | 未実装 | `fle: import kernel32.dll!HeapCompact is not modeled` | メモリ管理の残り(未実装) |
| kernel32.dll | `HeapDestroy` | 無し | 通常 | 138/138 | (a) | 未実装 | `fle: import kernel32.dll!HeapDestroy is not modeled` | メモリ管理の残り(未実装) |
| kernel32.dll | `HeapQueryInformation` | 無し | 通常 | 7/7 | (a) | 未実装 | `fle: import kernel32.dll!HeapQueryInformation is not modeled` | メモリ管理の残り(未実装) |
| kernel32.dll | `HeapValidate` | 無し | 通常 | 3/3 | (a) | 未実装 | `fle: import kernel32.dll!HeapValidate is not modeled` | メモリ管理の残り(未実装) |
| kernel32.dll | `InitOnceBeginInitialize` | 無し | 通常 | 3/3 | (a) | 未実装 | `fle: import kernel32.dll!InitOnceBeginInitialize is not modeled` | ファイル・ボリューム・ディレクトリの実物が要る |
| kernel32.dll | `InitOnceComplete` | 無し | 通常 | 3/3 | (a) | 未実装 | `fle: import kernel32.dll!InitOnceComplete is not modeled` | ファイル・ボリューム・ディレクトリの実物が要る |
| kernel32.dll | `InitializeConditionVariable` | 無し | 通常 | 5/5 | (a) | 未実装 | `fle: import kernel32.dll!InitializeConditionVariable is not modeled` | ファイル・ボリューム・ディレクトリの実物が要る |
| kernel32.dll | `InitializeSRWLock` | 無し | 通常 | 5/5 | (a) | 未実装 | `fle: import kernel32.dll!InitializeSRWLock is not modeled` | ファイル・ボリューム・ディレクトリの実物が要る |
| kernel32.dll | `InterlockedDecrement` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import kernel32.dll!InterlockedDecrement is not modeled` | OS の実値が要る(未実装) |
| kernel32.dll | `InterlockedIncrement` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import kernel32.dll!InterlockedIncrement is not modeled` | OS の実値が要る(未実装) |
| kernel32.dll | `IsBadWritePtr` | 無し | 通常 | 36/36 | (a) | 未実装 | `fle: import kernel32.dll!IsBadWritePtr is not modeled` | OS の実値が要る(未実装) |
| kernel32.dll | `LoadLibraryExA` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import kernel32.dll!LoadLibraryExA is not modeled` | OS の実値が要る(未実装) |
| kernel32.dll | `LockFile` | 無し | 通常 | 3/3 | (a) | 未実装 | `fle: import kernel32.dll!LockFile is not modeled` | ファイル・ボリューム・ディレクトリの実物が要る |
| kernel32.dll | `LockFileEx` | 無し | 通常 | 3/3 | (a) | 未実装 | `fle: import kernel32.dll!LockFileEx is not modeled` | ファイル・ボリューム・ディレクトリの実物が要る |
| kernel32.dll | `MoveFileExW` | 無し | 通常 | 3/3 | (a) | 未実装 | `fle: import kernel32.dll!MoveFileExW is not modeled` | ファイル・ボリューム・ディレクトリの実物が要る |
| kernel32.dll | `NormalizeString` | 無し | 遅延 | 75/75 | (a) | 未実装 | 遅延読み込みの枠: 名前つきの停止は未対応(GetProcAddress の経路) | ロケール・時刻の表が要る(未実装) |
| kernel32.dll | `OpenEventA` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import kernel32.dll!OpenEventA is not modeled` | プロセス・スレッド・待ち合わせ・タイマーの実物が要る |
| kernel32.dll | `OpenEventW` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import kernel32.dll!OpenEventW is not modeled` | プロセス・スレッド・待ち合わせ・タイマーの実物が要る |
| kernel32.dll | `OpenFileMappingA` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import kernel32.dll!OpenFileMappingA is not modeled` | ファイル・ボリューム・ディレクトリの実物が要る |
| kernel32.dll | `OpenFileMappingW` | 無し | 通常 | 26/26 | (a) | 実装 | fle_imp_OpenFileMappingW (既存の C を表に載せた。名前は見つからない) | 仕様どおりに実装(窓・ファイル・通信を要求しない) |
| kernel32.dll | `OpenMutexA` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import kernel32.dll!OpenMutexA is not modeled` | プロセス・スレッド・待ち合わせ・タイマーの実物が要る |
| kernel32.dll | `OpenProcess` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import kernel32.dll!OpenProcess is not modeled` | プロセス・スレッド・待ち合わせ・タイマーの実物が要る |
| kernel32.dll | `PeekNamedPipe` | 無し | 通常 | 16/16 | (a) | 未実装 | `fle: import kernel32.dll!PeekNamedPipe is not modeled` | ファイル・ボリューム・ディレクトリの実物が要る |
| kernel32.dll | `PostQueuedCompletionStatus` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import kernel32.dll!PostQueuedCompletionStatus is not modeled` | プロセス・スレッド・待ち合わせ・タイマーの実物が要る |
| kernel32.dll | `Process32First` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import kernel32.dll!Process32First is not modeled` | プロセス・スレッド・待ち合わせ・タイマーの実物が要る |
| kernel32.dll | `Process32Next` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import kernel32.dll!Process32Next is not modeled` | プロセス・スレッド・待ち合わせ・タイマーの実物が要る |
| kernel32.dll | `QueryDepthSList` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import kernel32.dll!QueryDepthSList is not modeled` | OS の実値が要る(未実装) |
| kernel32.dll | `QueryFullProcessImageNameA` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import kernel32.dll!QueryFullProcessImageNameA is not modeled` | プロセス・スレッド・待ち合わせ・タイマーの実物が要る |
| kernel32.dll | `QueueUserAPC` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import kernel32.dll!QueueUserAPC is not modeled` | プロセス・スレッド・待ち合わせ・タイマーの実物が要る |
| kernel32.dll | `RegisterWaitForSingleObject` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import kernel32.dll!RegisterWaitForSingleObject is not modeled` | プロセス・スレッド・待ち合わせ・タイマーの実物が要る |
| kernel32.dll | `ReleaseMutex` | 無し | 通常 | 2/2 | (a) | 実装 | fle_imp_ReleaseMutex (既存の C を表に載せた) | 仕様どおりに実装(窓・ファイル・通信を要求しない) |
| kernel32.dll | `ReleaseSemaphore` | 無し | 通常 | 3/3 | (a) | 未実装 | `fle: import kernel32.dll!ReleaseSemaphore is not modeled` | プロセス・スレッド・待ち合わせ・タイマーの実物が要る |
| kernel32.dll | `RemoveDirectoryA` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import kernel32.dll!RemoveDirectoryA is not modeled` | ファイル・ボリューム・ディレクトリの実物が要る |
| kernel32.dll | `RemoveVectoredExceptionHandler` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import kernel32.dll!RemoveVectoredExceptionHandler is not modeled` | 例外・デバッグ・装置の経路が要る(未実装) |
| kernel32.dll | `ReplaceFileW` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import kernel32.dll!ReplaceFileW is not modeled` | ファイル・ボリューム・ディレクトリの実物が要る |
| kernel32.dll | `RtlUnwind` | 無し | 通常 | 153/153 | (a) | 未実装 | `fle: import kernel32.dll!RtlUnwind is not modeled` | 例外・デバッグ・装置の経路が要る(未実装) |
| kernel32.dll | `SetConsoleCtrlHandler` | 無し | 通常 | 6/6 | (a) | 未実装 | `fle: import kernel32.dll!SetConsoleCtrlHandler is not modeled` | コンソールが要る |
| kernel32.dll | `SetConsoleTextAttribute` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import kernel32.dll!SetConsoleTextAttribute is not modeled` | コンソールが要る |
| kernel32.dll | `SetConsoleTitleA` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import kernel32.dll!SetConsoleTitleA is not modeled` | コンソールが要る |
| kernel32.dll | `SetCurrentDirectoryA` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import kernel32.dll!SetCurrentDirectoryA is not modeled` | ファイル・ボリューム・ディレクトリの実物が要る |
| kernel32.dll | `SetInformationJobObject` | 無し | 通常 | 2/2 | (a) | 未実装 | `fle: import kernel32.dll!SetInformationJobObject is not modeled` | プロセス・スレッド・待ち合わせ・タイマーの実物が要る |
| kernel32.dll | `SetThreadAffinityMask` | 無し | 通常 | 4/4 | (a) | 未実装 | `fle: import kernel32.dll!SetThreadAffinityMask is not modeled` | プロセス・スレッド・待ち合わせ・タイマーの実物が要る |
| kernel32.dll | `SetVolumeLabelW` | 無し | 通常 | 2/2 | (a) | 未実装 | `fle: import kernel32.dll!SetVolumeLabelW is not modeled` | ファイル・ボリューム・ディレクトリの実物が要る |
| kernel32.dll | `SetWaitableTimer` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import kernel32.dll!SetWaitableTimer is not modeled` | プロセス・スレッド・待ち合わせ・タイマーの実物が要る |
| kernel32.dll | `SignalObjectAndWait` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import kernel32.dll!SignalObjectAndWait is not modeled` | プロセス・スレッド・待ち合わせ・タイマーの実物が要る |
| kernel32.dll | `SleepConditionVariableCS` | 無し | 通常 | 4/4 | (a) | 未実装 | `fle: import kernel32.dll!SleepConditionVariableCS is not modeled` | プロセス・スレッド・待ち合わせ・タイマーの実物が要る |
| kernel32.dll | `SleepEx` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import kernel32.dll!SleepEx is not modeled` | プロセス・スレッド・待ち合わせ・タイマーの実物が要る |
| kernel32.dll | `TerminateThread` | 無し | 通常 | 4/4 | (a) | 未実装 | `fle: import kernel32.dll!TerminateThread is not modeled` | プロセス・スレッド・待ち合わせ・タイマーの実物が要る |
| kernel32.dll | `TryEnterCriticalSection` | 無し | 通常 | 8/8 | (a) | 実装 | fle_imp_TryEnterCriticalSection (新規) | 仕様どおりに実装(窓・ファイル・通信を要求しない) |
| kernel32.dll | `UnlockFile` | 無し | 通常 | 3/3 | (a) | 未実装 | `fle: import kernel32.dll!UnlockFile is not modeled` | ファイル・ボリューム・ディレクトリの実物が要る |
| kernel32.dll | `UnlockFileEx` | 無し | 通常 | 3/3 | (a) | 未実装 | `fle: import kernel32.dll!UnlockFileEx is not modeled` | ファイル・ボリューム・ディレクトリの実物が要る |
| kernel32.dll | `UnregisterWait` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import kernel32.dll!UnregisterWait is not modeled` | プロセス・スレッド・待ち合わせ・タイマーの実物が要る |
| kernel32.dll | `UnregisterWaitEx` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import kernel32.dll!UnregisterWaitEx is not modeled` | プロセス・スレッド・待ち合わせ・タイマーの実物が要る |
| kernel32.dll | `WaitForSingleObjectEx` | 無し | 通常 | 17/17 | (a) | 未実装 | `fle: import kernel32.dll!WaitForSingleObjectEx is not modeled` | プロセス・スレッド・待ち合わせ・タイマーの実物が要る |
| kernel32.dll | `WinExec` | 無し | 通常 | 3/3 | (a) | 未実装 | `fle: import kernel32.dll!WinExec is not modeled` | OS の実値が要る(未実装) |
| kernel32.dll | `lstrcatW` | 無し | 通常 | 1/1 | (a) | 実装 | fle_imp_lstrcatW (新規) | 仕様どおりに実装(窓・ファイル・通信を要求しない) |
| kernel32.dll | `lstrcmpW` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import kernel32.dll!lstrcmpW is not modeled` | OS の実値が要る(未実装) |
| kernel32.dll | `lstrcpyA` | 無し | 通常 | 1/1 | (a) | 実装 | fle_imp_lstrcpyA (新規) | 仕様どおりに実装(窓・ファイル・通信を要求しない) |
| kernel32.dll | `lstrcpynW` | 無し | 通常 | 1/1 | (a) | 実装 | fle_imp_lstrcpynW (新規) | 仕様どおりに実装(窓・ファイル・通信を要求しない) |
| mpr.dll | `WNetCloseEnum` | 無し | 通常 | 2/2 | (a) | 未実装 | `fle: import mpr.dll!WNetCloseEnum is not modeled` | ネットワーク・セッション・資源の実物が要る(外の世界) |
| mpr.dll | `WNetEnumResourceW` | 無し | 通常 | 2/2 | (a) | 未実装 | `fle: import mpr.dll!WNetEnumResourceW is not modeled` | ネットワーク・セッション・資源の実物が要る(外の世界) |
| mpr.dll | `WNetOpenEnumW` | 無し | 通常 | 2/2 | (a) | 未実装 | `fle: import mpr.dll!WNetOpenEnumW is not modeled` | ネットワーク・セッション・資源の実物が要る(外の世界) |
| mscoree.dll | `_CorDllMain` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import mscoree.dll!_CorDllMain is not modeled` | 未実装 |
| msimg32.dll | `AlphaBlend` | 無し | 遅延 | 134/134 | (a) | 未実装 | 遅延読み込みの枠: 名前つきの停止は未対応(GetProcAddress の経路) | 描画・入力の装置(Direct3D、DWM、テーマ、WIC、DirectInput)が要る |
| msvcp140.dll | `?_Xlength_error@std@@YAXPEBD@Z` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import msvcp140.dll!?_Xlength_error@std@@YAXPEBD@Z is not modeled` | C ランタイムの残り(例外・初期化・ヒープ・終了)。未実装 |
| msvcp140.dll | `_Thrd_hardware_concurrency` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import msvcp140.dll!_Thrd_hardware_concurrency is not modeled` | C ランタイムの残り(例外・初期化・ヒープ・終了)。未実装 |
| mswsock.dll | `AcceptEx` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import mswsock.dll!AcceptEx is not modeled` | ネットワーク・セッション・資源の実物が要る(外の世界) |
| mswsock.dll | `GetAcceptExSockaddrs` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import mswsock.dll!GetAcceptExSockaddrs is not modeled` | ネットワーク・セッション・資源の実物が要る(外の世界) |
| ole32.dll | `CoCreateGuid` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import ole32.dll!CoCreateGuid is not modeled` | COM の実体が要る |
| ole32.dll | `CoFreeUnusedLibraries` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import ole32.dll!CoFreeUnusedLibraries is not modeled` | COM の実体が要る |
| ole32.dll | `CoGetClassObject` | 無し | 通常 | 3/3 | (a) | 未実装 | `fle: import ole32.dll!CoGetClassObject is not modeled` | COM・OLE の実体(窓・クリップボード・ストレージ)が要る |
| ole32.dll | `CoGetInterfaceAndReleaseStream` | 無し | 通常 | 40/40 | (a) | 未実装 | `fle: import ole32.dll!CoGetInterfaceAndReleaseStream is not modeled` | COM・OLE の実体(窓・クリップボード・ストレージ)が要る |
| ole32.dll | `CoMarshalInterThreadInterfaceInStream` | 無し | 通常 | 40/40 | (a) | 未実装 | `fle: import ole32.dll!CoMarshalInterThreadInterfaceInStream is not modeled` | COM・OLE の実体(窓・クリップボード・ストレージ)が要る |
| ole32.dll | `CreateDataAdviseHolder` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import ole32.dll!CreateDataAdviseHolder is not modeled` | COM・OLE の実体(窓・クリップボード・ストレージ)が要る |
| ole32.dll | `CreateILockBytesOnHGlobal` | 無し | 通常 | 134/134 | (a) | 未実装 | `fle: import ole32.dll!CreateILockBytesOnHGlobal is not modeled` | COM・OLE の実体(窓・クリップボード・ストレージ)が要る |
| ole32.dll | `CreateStreamOnHGlobal` | 無し | 通常 | 141/141 | (a) | 未実装 | `fle: import ole32.dll!CreateStreamOnHGlobal is not modeled` | COM・OLE の実体(窓・クリップボード・ストレージ)が要る |
| ole32.dll | `IsAccelerator` | 無し | 通常 | 3/3 | (a) | 未実装 | `fle: import ole32.dll!IsAccelerator is not modeled` | COM の実体が要る |
| ole32.dll | `OleDraw` | 無し | 通常 | 3/3 | (a) | 未実装 | `fle: import ole32.dll!OleDraw is not modeled` | COM・OLE の実体(窓・クリップボード・ストレージ)が要る |
| ole32.dll | `OleGetClipboard` | 無し | 通常 | 41/41 | (a) | 未実装 | `fle: import ole32.dll!OleGetClipboard is not modeled` | COM・OLE の実体(窓・クリップボード・ストレージ)が要る |
| ole32.dll | `OleSetMenuDescriptor` | 無し | 通常 | 3/3 | (a) | 未実装 | `fle: import ole32.dll!OleSetMenuDescriptor is not modeled` | COM・OLE の実体(窓・クリップボード・ストレージ)が要る |
| ole32.dll | `ReleaseStgMedium` | 無し | 通常 | 141/141 | (a) | 未実装 | `fle: import ole32.dll!ReleaseStgMedium is not modeled` | COM・OLE の実体(窓・クリップボード・ストレージ)が要る |
| ole32.dll | `StgCreateDocfileOnILockBytes` | 無し | 通常 | 134/134 | (a) | 未実装 | `fle: import ole32.dll!StgCreateDocfileOnILockBytes is not modeled` | COM・OLE の実体(窓・クリップボード・ストレージ)が要る |
| ole32.dll | `StringFromGUID2` | 無し | 通常 | 1/1 | (a) | 実装 | fle_imp_StringFromGUID2 (新規) | 仕様どおりに実装(窓・ファイル・通信を要求しない) |
| oleacc.dll | `LresultFromObject` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import oleacc.dll!LresultFromObject is not modeled` | アクセシビリティの窓(オブジェクト)が要る |
| oleaut32.dll | `#149` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import oleaut32.dll!#149 is not modeled` | COM の実体が要る |
| oleaut32.dll | `#150` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import oleaut32.dll!#150 is not modeled` | COM の実体が要る |
| oleaut32.dll | `#4` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import oleaut32.dll!#4 is not modeled` | COM の実体が要る |
| oleaut32.dll | `#6` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import oleaut32.dll!#6 is not modeled` | COM の実体が要る |
| oleaut32.dll | `#9` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import oleaut32.dll!#9 is not modeled` | COM の実体が要る |
| oleaut32.dll | `GetActiveObject` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import oleaut32.dll!GetActiveObject is not modeled` | COM の実体が要る |
| opengl32.dll | `glBegin` | 無し | 通常 | 7/7 | (a) | 未実装 | `fle: import opengl32.dll!glBegin is not modeled` | 描画の実物(OpenGL の描画面とテクスチャ)が要る |
| opengl32.dll | `glBindTexture` | 無し | 通常 | 9/9 | (a) | 未実装 | `fle: import opengl32.dll!glBindTexture is not modeled` | 描画の実物(OpenGL の描画面とテクスチャ)が要る |
| opengl32.dll | `glBlendFunc` | 無し | 通常 | 7/7 | (a) | 未実装 | `fle: import opengl32.dll!glBlendFunc is not modeled` | 描画の実物(OpenGL の描画面とテクスチャ)が要る |
| opengl32.dll | `glClear` | 無し | 通常 | 7/7 | (a) | 未実装 | `fle: import opengl32.dll!glClear is not modeled` | 描画の実物(OpenGL の描画面とテクスチャ)が要る |
| opengl32.dll | `glClearColor` | 無し | 通常 | 7/7 | (a) | 未実装 | `fle: import opengl32.dll!glClearColor is not modeled` | 描画の実物(OpenGL の描画面とテクスチャ)が要る |
| opengl32.dll | `glClearDepth` | 無し | 通常 | 7/7 | (a) | 未実装 | `fle: import opengl32.dll!glClearDepth is not modeled` | 描画の実物(OpenGL の描画面とテクスチャ)が要る |
| opengl32.dll | `glColor4b` | 無し | 通常 | 7/7 | (a) | 未実装 | `fle: import opengl32.dll!glColor4b is not modeled` | 描画の実物(OpenGL の描画面とテクスチャ)が要る |
| opengl32.dll | `glColor4f` | 無し | 通常 | 7/7 | (a) | 未実装 | `fle: import opengl32.dll!glColor4f is not modeled` | 描画の実物(OpenGL の描画面とテクスチャ)が要る |
| opengl32.dll | `glColor4fv` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import opengl32.dll!glColor4fv is not modeled` | 描画の実物(OpenGL の描画面とテクスチャ)が要る |
| opengl32.dll | `glColorMask` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import opengl32.dll!glColorMask is not modeled` | 描画の実物(OpenGL の描画面とテクスチャ)が要る |
| opengl32.dll | `glCopyTexSubImage2D` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import opengl32.dll!glCopyTexSubImage2D is not modeled` | 描画の実物(OpenGL の描画面とテクスチャ)が要る |
| opengl32.dll | `glCullFace` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import opengl32.dll!glCullFace is not modeled` | 描画の実物(OpenGL の描画面とテクスチャ)が要る |
| opengl32.dll | `glDeleteTextures` | 無し | 通常 | 9/9 | (a) | 未実装 | `fle: import opengl32.dll!glDeleteTextures is not modeled` | 描画の実物(OpenGL の描画面とテクスチャ)が要る |
| opengl32.dll | `glDepthFunc` | 無し | 通常 | 7/7 | (a) | 未実装 | `fle: import opengl32.dll!glDepthFunc is not modeled` | 描画の実物(OpenGL の描画面とテクスチャ)が要る |
| opengl32.dll | `glDisable` | 無し | 通常 | 8/8 | (a) | 未実装 | `fle: import opengl32.dll!glDisable is not modeled` | 描画の実物(OpenGL の描画面とテクスチャ)が要る |
| opengl32.dll | `glDisableClientState` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import opengl32.dll!glDisableClientState is not modeled` | 描画の実物(OpenGL の描画面とテクスチャ)が要る |
| opengl32.dll | `glDrawArrays` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import opengl32.dll!glDrawArrays is not modeled` | 描画の実物(OpenGL の描画面とテクスチャ)が要る |
| opengl32.dll | `glDrawBuffer` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import opengl32.dll!glDrawBuffer is not modeled` | 描画の実物(OpenGL の描画面とテクスチャ)が要る |
| opengl32.dll | `glDrawElements` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import opengl32.dll!glDrawElements is not modeled` | 描画の実物(OpenGL の描画面とテクスチャ)が要る |
| opengl32.dll | `glEnable` | 無し | 通常 | 8/8 | (a) | 未実装 | `fle: import opengl32.dll!glEnable is not modeled` | 描画の実物(OpenGL の描画面とテクスチャ)が要る |
| opengl32.dll | `glEnableClientState` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import opengl32.dll!glEnableClientState is not modeled` | 描画の実物(OpenGL の描画面とテクスチャ)が要る |
| opengl32.dll | `glEnd` | 無し | 通常 | 7/7 | (a) | 未実装 | `fle: import opengl32.dll!glEnd is not modeled` | 描画の実物(OpenGL の描画面とテクスチャ)が要る |
| opengl32.dll | `glFinish` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import opengl32.dll!glFinish is not modeled` | 描画の実物(OpenGL の描画面とテクスチャ)が要る |
| opengl32.dll | `glFrontFace` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import opengl32.dll!glFrontFace is not modeled` | 描画の実物(OpenGL の描画面とテクスチャ)が要る |
| opengl32.dll | `glGenTextures` | 無し | 通常 | 9/9 | (a) | 未実装 | `fle: import opengl32.dll!glGenTextures is not modeled` | 描画の実物(OpenGL の描画面とテクスチャ)が要る |
| opengl32.dll | `glGetError` | 無し | 通常 | 2/2 | (a) | 未実装 | `fle: import opengl32.dll!glGetError is not modeled` | 描画の実物(OpenGL の描画面とテクスチャ)が要る |
| opengl32.dll | `glGetIntegerv` | 無し | 通常 | 9/9 | (a) | 未実装 | `fle: import opengl32.dll!glGetIntegerv is not modeled` | 描画の実物(OpenGL の描画面とテクスチャ)が要る |
| opengl32.dll | `glGetString` | 無し | 通常 | 8/8 | (a) | 未実装 | `fle: import opengl32.dll!glGetString is not modeled` | 描画の実物(OpenGL の描画面とテクスチャ)が要る |
| opengl32.dll | `glGetTexImage` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import opengl32.dll!glGetTexImage is not modeled` | 描画の実物(OpenGL の描画面とテクスチャ)が要る |
| opengl32.dll | `glGetTexLevelParameteriv` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import opengl32.dll!glGetTexLevelParameteriv is not modeled` | 描画の実物(OpenGL の描画面とテクスチャ)が要る |
| opengl32.dll | `glHint` | 無し | 通常 | 7/7 | (a) | 未実装 | `fle: import opengl32.dll!glHint is not modeled` | 描画の実物(OpenGL の描画面とテクスチャ)が要る |
| opengl32.dll | `glLineWidth` | 無し | 通常 | 7/7 | (a) | 未実装 | `fle: import opengl32.dll!glLineWidth is not modeled` | 描画の実物(OpenGL の描画面とテクスチャ)が要る |
| opengl32.dll | `glLoadIdentity` | 無し | 通常 | 7/7 | (a) | 未実装 | `fle: import opengl32.dll!glLoadIdentity is not modeled` | 描画の実物(OpenGL の描画面とテクスチャ)が要る |
| opengl32.dll | `glMaterialfv` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import opengl32.dll!glMaterialfv is not modeled` | 描画の実物(OpenGL の描画面とテクスチャ)が要る |
| opengl32.dll | `glMatrixMode` | 無し | 通常 | 7/7 | (a) | 未実装 | `fle: import opengl32.dll!glMatrixMode is not modeled` | 描画の実物(OpenGL の描画面とテクスチャ)が要る |
| opengl32.dll | `glNormalPointer` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import opengl32.dll!glNormalPointer is not modeled` | 描画の実物(OpenGL の描画面とテクスチャ)が要る |
| opengl32.dll | `glOrtho` | 無し | 通常 | 7/7 | (a) | 未実装 | `fle: import opengl32.dll!glOrtho is not modeled` | 描画の実物(OpenGL の描画面とテクスチャ)が要る |
| opengl32.dll | `glPixelStorei` | 無し | 通常 | 9/9 | (a) | 未実装 | `fle: import opengl32.dll!glPixelStorei is not modeled` | 描画の実物(OpenGL の描画面とテクスチャ)が要る |
| opengl32.dll | `glPopAttrib` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import opengl32.dll!glPopAttrib is not modeled` | 描画の実物(OpenGL の描画面とテクスチャ)が要る |
| opengl32.dll | `glPushAttrib` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import opengl32.dll!glPushAttrib is not modeled` | 描画の実物(OpenGL の描画面とテクスチャ)が要る |
| opengl32.dll | `glReadBuffer` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import opengl32.dll!glReadBuffer is not modeled` | 描画の実物(OpenGL の描画面とテクスチャ)が要る |
| opengl32.dll | `glReadPixels` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import opengl32.dll!glReadPixels is not modeled` | 描画の実物(OpenGL の描画面とテクスチャ)が要る |
| opengl32.dll | `glScissor` | 無し | 通常 | 7/7 | (a) | 未実装 | `fle: import opengl32.dll!glScissor is not modeled` | 描画の実物(OpenGL の描画面とテクスチャ)が要る |
| opengl32.dll | `glShadeModel` | 無し | 通常 | 7/7 | (a) | 未実装 | `fle: import opengl32.dll!glShadeModel is not modeled` | 描画の実物(OpenGL の描画面とテクスチャ)が要る |
| opengl32.dll | `glStencilFunc` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import opengl32.dll!glStencilFunc is not modeled` | 描画の実物(OpenGL の描画面とテクスチャ)が要る |
| opengl32.dll | `glStencilMask` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import opengl32.dll!glStencilMask is not modeled` | 描画の実物(OpenGL の描画面とテクスチャ)が要る |
| opengl32.dll | `glStencilOp` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import opengl32.dll!glStencilOp is not modeled` | 描画の実物(OpenGL の描画面とテクスチャ)が要る |
| opengl32.dll | `glTexCoord2f` | 無し | 通常 | 7/7 | (a) | 未実装 | `fle: import opengl32.dll!glTexCoord2f is not modeled` | 描画の実物(OpenGL の描画面とテクスチャ)が要る |
| opengl32.dll | `glTexCoordPointer` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import opengl32.dll!glTexCoordPointer is not modeled` | 描画の実物(OpenGL の描画面とテクスチャ)が要る |
| opengl32.dll | `glTexImage2D` | 無し | 通常 | 9/9 | (a) | 未実装 | `fle: import opengl32.dll!glTexImage2D is not modeled` | 描画の実物(OpenGL の描画面とテクスチャ)が要る |
| opengl32.dll | `glTexParameterf` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import opengl32.dll!glTexParameterf is not modeled` | 描画の実物(OpenGL の描画面とテクスチャ)が要る |
| opengl32.dll | `glTexParameteri` | 無し | 通常 | 9/9 | (a) | 未実装 | `fle: import opengl32.dll!glTexParameteri is not modeled` | 描画の実物(OpenGL の描画面とテクスチャ)が要る |
| opengl32.dll | `glTexSubImage2D` | 無し | 通常 | 9/9 | (a) | 未実装 | `fle: import opengl32.dll!glTexSubImage2D is not modeled` | 描画の実物(OpenGL の描画面とテクスチャ)が要る |
| opengl32.dll | `glVertex2f` | 無し | 通常 | 7/7 | (a) | 未実装 | `fle: import opengl32.dll!glVertex2f is not modeled` | 描画の実物(OpenGL の描画面とテクスチャ)が要る |
| opengl32.dll | `glVertex2i` | 無し | 通常 | 7/7 | (a) | 未実装 | `fle: import opengl32.dll!glVertex2i is not modeled` | 描画の実物(OpenGL の描画面とテクスチャ)が要る |
| opengl32.dll | `glVertexPointer` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import opengl32.dll!glVertexPointer is not modeled` | 描画の実物(OpenGL の描画面とテクスチャ)が要る |
| opengl32.dll | `glViewport` | 無し | 通常 | 7/7 | (a) | 未実装 | `fle: import opengl32.dll!glViewport is not modeled` | 描画の実物(OpenGL の描画面とテクスチャ)が要る |
| opengl32.dll | `wglCreateContext` | 無し | 通常 | 8/8 | (a) | 未実装 | `fle: import opengl32.dll!wglCreateContext is not modeled` | 描画の実物(OpenGL の描画面とテクスチャ)が要る |
| opengl32.dll | `wglDeleteContext` | 無し | 通常 | 8/8 | (a) | 未実装 | `fle: import opengl32.dll!wglDeleteContext is not modeled` | 描画の実物(OpenGL の描画面とテクスチャ)が要る |
| opengl32.dll | `wglGetCurrentContext` | 無し | 通常 | 8/8 | (a) | 未実装 | `fle: import opengl32.dll!wglGetCurrentContext is not modeled` | 描画の実物(OpenGL の描画面とテクスチャ)が要る |
| opengl32.dll | `wglGetCurrentDC` | 無し | 通常 | 2/2 | (a) | 未実装 | `fle: import opengl32.dll!wglGetCurrentDC is not modeled` | 描画の実物(OpenGL の描画面とテクスチャ)が要る |
| opengl32.dll | `wglGetProcAddress` | 無し | 通常 | 10/10 | (a) | 未実装 | `fle: import opengl32.dll!wglGetProcAddress is not modeled` | 描画の実物(OpenGL の描画面とテクスチャ)が要る |
| opengl32.dll | `wglMakeCurrent` | 無し | 通常 | 8/8 | (a) | 未実装 | `fle: import opengl32.dll!wglMakeCurrent is not modeled` | 描画の実物(OpenGL の描画面とテクスチャ)が要る |
| opengl32.dll | `wglUseFontBitmapsW` | 無し | 通常 | 1/1 | (b) | 名前つきの停止 | `fle: import opengl32.dll!wglUseFontBitmapsW is not modeled` | フォントの実物(字形・計量)が要る |
| shell32.dll | `#155` | 無し | 通常 | 2/2 | (a) | 未実装 | `fle: import shell32.dll!#155 is not modeled` | シェル(ファイル・アイコン・ダイアログ)が要る |
| shell32.dll | `#165` | 無し | 通常 | 12/12 | (a) | 未実装 | `fle: import shell32.dll!#165 is not modeled` | シェル(ファイル・アイコン・ダイアログ)が要る |
| shell32.dll | `#190` | 無し | 通常 | 2/2 | (a) | 未実装 | `fle: import shell32.dll!#190 is not modeled` | シェル(ファイル・アイコン・ダイアログ)が要る |
| shell32.dll | `DragQueryFileA` | 無し | 通常 | 7/7 | (a) | 未実装 | `fle: import shell32.dll!DragQueryFileA is not modeled` | シェル(ファイル・アイコン・ダイアログ)が要る |
| shell32.dll | `ExtractAssociatedIconW` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import shell32.dll!ExtractAssociatedIconW is not modeled` | シェル(ファイル・アイコン・ダイアログ)が要る |
| shell32.dll | `SHAppBarMessage` | 無し | 通常 | 134/134 | (a) | 未実装 | `fle: import shell32.dll!SHAppBarMessage is not modeled` | シェル(ファイル・アイコン・ダイアログ)が要る |
| shell32.dll | `SHBrowseForFolderW` | 無し | 通常 | 3/3 | (a) | 未実装 | `fle: import shell32.dll!SHBrowseForFolderW is not modeled` | シェル(ファイル・アイコン・ダイアログ)が要る |
| shell32.dll | `SHFileOperationW` | 無し | 通常 | 28/28 | (a) | 未実装 | `fle: import shell32.dll!SHFileOperationW is not modeled` | シェル(ファイル・アイコン・ダイアログ)が要る |
| shell32.dll | `SHGetDesktopFolder` | 無し | 通常 | 134/134 | (a) | 未実装 | `fle: import shell32.dll!SHGetDesktopFolder is not modeled` | シェル(ファイル・アイコン・ダイアログ)が要る |
| shell32.dll | `SHGetFileInfoW` | 無し | 通常 | 80/80 | (a) | 未実装 | `fle: import shell32.dll!SHGetFileInfoW is not modeled` | シェル(ファイル・アイコン・ダイアログ)が要る |
| shell32.dll | `SHGetFolderPathA` | 無し | 通常 | 7/7 | (a) | 未実装 | `fle: import shell32.dll!SHGetFolderPathA is not modeled` | シェル(ファイル・アイコン・ダイアログ)が要る |
| shell32.dll | `SHGetSpecialFolderPathW` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import shell32.dll!SHGetSpecialFolderPathW is not modeled` | シェル(ファイル・アイコン・ダイアログ)が要る |
| shell32.dll | `SHOpenFolderAndSelectItems` | 無し | 遅延+通常 | 4/4 | (a) | 未実装 | `fle: import shell32.dll!SHOpenFolderAndSelectItems is not modeled` | シェル(ファイル・アイコン・ダイアログ)が要る |
| shell32.dll | `ShellExecuteExA` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import shell32.dll!ShellExecuteExA is not modeled` | シェル(ファイル・アイコン・ダイアログ)が要る |
| shell32.dll | `Shell_NotifyIconW` | 無し | 通常 | 134/134 | (a) | 未実装 | `fle: import shell32.dll!Shell_NotifyIconW is not modeled` | シェル(ファイル・アイコン・ダイアログ)が要る |
| shlwapi.dll | `PathFileExistsW` | 無し | 通常 | 7/7 | (a) | 未実装 | `fle: import shlwapi.dll!PathFileExistsW is not modeled` | 文字列・パスの純粋な処理。未実装 |
| shlwapi.dll | `PathFindExtensionA` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import shlwapi.dll!PathFindExtensionA is not modeled` | 文字列・パスの純粋な処理。未実装 |
| shlwapi.dll | `PathFindFileNameA` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import shlwapi.dll!PathFindFileNameA is not modeled` | 文字列・パスの純粋な処理。未実装 |
| shlwapi.dll | `PathRemoveFileSpecW` | 無し | 通常 | 2/2 | (a) | 未実装 | `fle: import shlwapi.dll!PathRemoveFileSpecW is not modeled` | 文字列・パスの純粋な処理。未実装 |
| shlwapi.dll | `StrCatW` | 無し | 通常 | 2/2 | (a) | 未実装 | `fle: import shlwapi.dll!StrCatW is not modeled` | 文字列・パスの純粋な処理。未実装 |
| shlwapi.dll | `StrCpyW` | 無し | 通常 | 2/2 | (a) | 未実装 | `fle: import shlwapi.dll!StrCpyW is not modeled` | 文字列・パスの純粋な処理。未実装 |
| shlwapi.dll | `StrStrIA` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import shlwapi.dll!StrStrIA is not modeled` | 文字列・パスの純粋な処理。未実装 |
| user32.dll | `AppendMenuA` | 無し | 通常 | 5/5 | (a) | 未実装 | `fle: import user32.dll!AppendMenuA is not modeled` | メニュー(窓)が要る |
| user32.dll | `AppendMenuW` | 無し | 通常 | 2/2 | (a) | 未実装 | `fle: import user32.dll!AppendMenuW is not modeled` | メニュー(窓)が要る |
| user32.dll | `AttachThreadInput` | 無し | 通常 | 29/29 | (a) | 未実装 | `fle: import user32.dll!AttachThreadInput is not modeled` | 入力(フック・キーボード・タッチ)の窓が要る |
| user32.dll | `ChangeClipboardChain` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import user32.dll!ChangeClipboardChain is not modeled` | クリップボードの窓が要る |
| user32.dll | `CharNextExA` | 無し | 通常 | 2/2 | (a) | 未実装 | `fle: import user32.dll!CharNextExA is not modeled` | 窓(ウィンドウ・メッセージの待ち行列)が要る |
| user32.dll | `CharToOemBuffW` | 無し | 通常 | 134/134 | (a) | 未実装 | `fle: import user32.dll!CharToOemBuffW is not modeled` | 窓(ウィンドウ・メッセージの待ち行列)が要る |
| user32.dll | `CloseGestureInfoHandle` | 無し | 遅延 | 134/134 | (a) | 未実装 | 遅延読み込みの枠: 名前つきの停止は未対応(GetProcAddress の経路) | 入力(フック・キーボード・タッチ)の窓が要る |
| user32.dll | `CloseTouchInputHandle` | 無し | 通常 | 3/3 | (a) | 未実装 | `fle: import user32.dll!CloseTouchInputHandle is not modeled` | 入力(フック・キーボード・タッチ)の窓が要る |
| user32.dll | `CopyIcon` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import user32.dll!CopyIcon is not modeled` | アイコン・カーソルの実物が要る |
| user32.dll | `CopyImage` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import user32.dll!CopyImage is not modeled` | アイコン・カーソルの実物が要る |
| user32.dll | `CopyRect` | 無し | 通常 | 2/2 | (a) | 未実装 | `fle: import user32.dll!CopyRect is not modeled` | 窓(ウィンドウ・メッセージの待ち行列)が要る |
| user32.dll | `CreateAcceleratorTableW` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import user32.dll!CreateAcceleratorTableW is not modeled` | メニュー(窓)が要る |
| user32.dll | `CreateIconFromResource` | 無し | 通常 | 2/2 | (a) | 未実装 | `fle: import user32.dll!CreateIconFromResource is not modeled` | アイコン・カーソルの実物が要る |
| user32.dll | `CreateIconFromResourceEx` | 無し | 通常 | 7/7 | (a) | 未実装 | `fle: import user32.dll!CreateIconFromResourceEx is not modeled` | アイコン・カーソルの実物が要る |
| user32.dll | `CreateWindowExA` | 無し | 通常 | 18/18 | (a) | 未実装 | `fle: import user32.dll!CreateWindowExA is not modeled` | 窓(ウィンドウ・メッセージの待ち行列)が要る |
| user32.dll | `DefFrameProcW` | 無し | 通常 | 134/134 | (a) | 未実装 | `fle: import user32.dll!DefFrameProcW is not modeled` | 窓(ウィンドウ・メッセージの待ち行列)が要る |
| user32.dll | `DefMDIChildProcW` | 無し | 通常 | 134/134 | (a) | 未実装 | `fle: import user32.dll!DefMDIChildProcW is not modeled` | 窓(ウィンドウ・メッセージの待ち行列)が要る |
| user32.dll | `DefWindowProcA` | 無し | 通常 | 15/15 | (a) | 未実装 | `fle: import user32.dll!DefWindowProcA is not modeled` | 窓(ウィンドウ・メッセージの待ち行列)が要る |
| user32.dll | `DialogBoxIndirectParamA` | 無し | 通常 | 2/2 | (a) | 未実装 | `fle: import user32.dll!DialogBoxIndirectParamA is not modeled` | ダイアログ(窓)が要る |
| user32.dll | `DialogBoxParamA` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import user32.dll!DialogBoxParamA is not modeled` | ダイアログ(窓)が要る |
| user32.dll | `DialogBoxParamW` | 無し | 通常 | 3/3 | (a) | 未実装 | `fle: import user32.dll!DialogBoxParamW is not modeled` | ダイアログ(窓)が要る |
| user32.dll | `EnableScrollBar` | 無し | 通常 | 134/134 | (a) | 未実装 | `fle: import user32.dll!EnableScrollBar is not modeled` | スクロールバー(窓)が要る |
| user32.dll | `EndDialog` | 無し | 通常 | 6/6 | (a) | 未実装 | `fle: import user32.dll!EndDialog is not modeled` | ダイアログ(窓)が要る |
| user32.dll | `EndMenu` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import user32.dll!EndMenu is not modeled` | メニュー(窓)が要る |
| user32.dll | `EnumDisplaySettingsA` | 無し | 通常 | 3/3 | (a) | 未実装 | `fle: import user32.dll!EnumDisplaySettingsA is not modeled` | 窓(ウィンドウ・メッセージの待ち行列)が要る |
| user32.dll | `EvaluateProximityToRect` | 無し | 遅延 | 134/134 | (a) | 未実装 | 遅延読み込みの枠: 名前つきの停止は未対応(GetProcAddress の経路) | 入力(フック・キーボード・タッチ)の窓が要る |
| user32.dll | `FindWindowA` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import user32.dll!FindWindowA is not modeled` | 窓(ウィンドウ・メッセージの待ち行列)が要る |
| user32.dll | `FindWindowExW` | 無し | 通常 | 135/135 | (a) | 未実装 | `fle: import user32.dll!FindWindowExW is not modeled` | 窓(ウィンドウ・メッセージの待ち行列)が要る |
| user32.dll | `GetClassInfoExW` | 無し | 通常 | 7/7 | (a) | 未実装 | `fle: import user32.dll!GetClassInfoExW is not modeled` | 窓(ウィンドウ・メッセージの待ち行列)が要る |
| user32.dll | `GetClassLongPtrW` | 無し | 通常 | 134/134 | (a) | 実装 | fle_imp_GetClassLongPtrW (新規。GCLP_HICON のみ。他の添字は名前つきの停止) | 仕様どおりに実装(窓・ファイル・通信を要求しない) |
| user32.dll | `GetClassNameA` | 無し | 通常 | 5/5 | (a) | 未実装 | `fle: import user32.dll!GetClassNameA is not modeled` | 窓(ウィンドウ・メッセージの待ち行列)が要る |
| user32.dll | `GetDlgItem` | 無し | 通常 | 79/79 | (a) | 未実装 | `fle: import user32.dll!GetDlgItem is not modeled` | ダイアログ(窓)が要る |
| user32.dll | `GetDpiForWindow` | 無し | 通常 | 3/3 | (a) | 未実装 | `fle: import user32.dll!GetDpiForWindow is not modeled` | 窓(ウィンドウ・メッセージの待ち行列)が要る |
| user32.dll | `GetGestureInfo` | 無し | 遅延 | 134/134 | (a) | 未実装 | 遅延読み込みの枠: 名前つきの停止は未対応(GetProcAddress の経路) | 入力(フック・キーボード・タッチ)の窓が要る |
| user32.dll | `GetQueueStatus` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import user32.dll!GetQueueStatus is not modeled` | 窓(ウィンドウ・メッセージの待ち行列)が要る |
| user32.dll | `GetScrollPos` | 無し | 通常 | 134/134 | (a) | 未実装 | `fle: import user32.dll!GetScrollPos is not modeled` | スクロールバー(窓)が要る |
| user32.dll | `GetScrollRange` | 無し | 通常 | 134/134 | (a) | 未実装 | `fle: import user32.dll!GetScrollRange is not modeled` | スクロールバー(窓)が要る |
| user32.dll | `GetTouchInputInfo` | 無し | 通常 | 3/3 | (a) | 未実装 | `fle: import user32.dll!GetTouchInputInfo is not modeled` | 入力(フック・キーボード・タッチ)の窓が要る |
| user32.dll | `GetWindowInfo` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import user32.dll!GetWindowInfo is not modeled` | 窓(ウィンドウ・メッセージの待ち行列)が要る |
| user32.dll | `GetWindowLongA` | 無し | 通常 | 3/3 | (a) | 未実装 | `fle: import user32.dll!GetWindowLongA is not modeled` | 窓(ウィンドウ・メッセージの待ち行列)が要る |
| user32.dll | `GetWindowLongPtrA` | 無し | 通常 | 20/20 | (a) | 未実装 | `fle: import user32.dll!GetWindowLongPtrA is not modeled` | 窓(ウィンドウ・メッセージの待ち行列)が要る |
| user32.dll | `GetWindowLongW` | 無し | 通常 | 8/8 | (a) | 未実装 | `fle: import user32.dll!GetWindowLongW is not modeled` | 窓(ウィンドウ・メッセージの待ち行列)が要る |
| user32.dll | `GetWindowTextA` | 無し | 通常 | 3/3 | (a) | 未実装 | `fle: import user32.dll!GetWindowTextA is not modeled` | 窓(ウィンドウ・メッセージの待ち行列)が要る |
| user32.dll | `InsertMenuA` | 無し | 通常 | 2/2 | (a) | 未実装 | `fle: import user32.dll!InsertMenuA is not modeled` | メニュー(窓)が要る |
| user32.dll | `IsDialogMessageA` | 無し | 通常 | 134/134 | (a) | 未実装 | `fle: import user32.dll!IsDialogMessageA is not modeled` | ダイアログ(窓)が要る |
| user32.dll | `IsDialogMessageW` | 無し | 通常 | 134/134 | (a) | 未実装 | `fle: import user32.dll!IsDialogMessageW is not modeled` | ダイアログ(窓)が要る |
| user32.dll | `IsProcessDPIAware` | 無し | 遅延 | 134/134 | (a) | 未実装 | 遅延読み込みの枠: 名前つきの停止は未対応(GetProcAddress の経路) | 窓(ウィンドウ・メッセージの待ち行列)が要る |
| user32.dll | `LoadCursorA` | 無し | 通常 | 21/21 | (a) | 未実装 | `fle: import user32.dll!LoadCursorA is not modeled` | アイコン・カーソルの実物が要る |
| user32.dll | `LoadImageA` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import user32.dll!LoadImageA is not modeled` | アイコン・カーソルの実物が要る |
| user32.dll | `MessageBoxA` | 無し | 遅延+通常 | 142/142 | (a) | 未実装 | `fle: import user32.dll!MessageBoxA is not modeled` | 窓(ウィンドウ・メッセージの待ち行列)が要る |
| user32.dll | `MsgWaitForMultipleObjectsEx` | 無し | 通常 | 134/134 | (a) | 未実装 | `fle: import user32.dll!MsgWaitForMultipleObjectsEx is not modeled` | 窓(ウィンドウ・メッセージの待ち行列)が要る |
| user32.dll | `NotifyWinEvent` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import user32.dll!NotifyWinEvent is not modeled` | 窓(ウィンドウ・メッセージの待ち行列)が要る |
| user32.dll | `PackTouchHitTestingProximityEvaluation` | 無し | 遅延 | 134/134 | (a) | 未実装 | 遅延読み込みの枠: 名前つきの停止は未対応(GetProcAddress の経路) | 入力(フック・キーボード・タッチ)の窓が要る |
| user32.dll | `PeekMessageA` | 無し | 通常 | 3/3 | (a) | 未実装 | `fle: import user32.dll!PeekMessageA is not modeled` | 窓(ウィンドウ・メッセージの待ち行列)が要る |
| user32.dll | `PostMessageA` | 無し | 通常 | 9/9 | (a) | 未実装 | `fle: import user32.dll!PostMessageA is not modeled` | 窓(ウィンドウ・メッセージの待ち行列)が要る |
| user32.dll | `RegisterClassA` | 無し | 通常 | 17/17 | (a) | 未実装 | `fle: import user32.dll!RegisterClassA is not modeled` | 窓(ウィンドウ・メッセージの待ち行列)が要る |
| user32.dll | `RegisterClassExW` | 無し | 通常 | 8/8 | (a) | 未実装 | `fle: import user32.dll!RegisterClassExW is not modeled` | 窓(ウィンドウ・メッセージの待ち行列)が要る |
| user32.dll | `RegisterClipboardFormatA` | 無し | 通常 | 3/3 | (a) | 未実装 | `fle: import user32.dll!RegisterClipboardFormatA is not modeled` | クリップボードの窓が要る |
| user32.dll | `RegisterTouchHitTestingWindow` | 無し | 遅延 | 134/134 | (a) | 未実装 | 遅延読み込みの枠: 名前つきの停止は未対応(GetProcAddress の経路) | 入力(フック・キーボード・タッチ)の窓が要る |
| user32.dll | `RegisterTouchWindow` | 無し | 遅延+通常 | 137/137 | (a) | 未実装 | `fle: import user32.dll!RegisterTouchWindow is not modeled` | 入力(フック・キーボード・タッチ)の窓が要る |
| user32.dll | `SendDlgItemMessageA` | 無し | 通常 | 2/2 | (a) | 未実装 | `fle: import user32.dll!SendDlgItemMessageA is not modeled` | ダイアログ(窓)が要る |
| user32.dll | `SendDlgItemMessageW` | 無し | 通常 | 2/2 | (a) | 未実装 | `fle: import user32.dll!SendDlgItemMessageW is not modeled` | ダイアログ(窓)が要る |
| user32.dll | `SendInput` | 無し | 通常 | 4/4 | (a) | 未実装 | `fle: import user32.dll!SendInput is not modeled` | 入力(フック・キーボード・タッチ)の窓が要る |
| user32.dll | `SendNotifyMessageW` | 無し | 通常 | 4/4 | (a) | 未実装 | `fle: import user32.dll!SendNotifyMessageW is not modeled` | 窓(ウィンドウ・メッセージの待ち行列)が要る |
| user32.dll | `SetClipboardViewer` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import user32.dll!SetClipboardViewer is not modeled` | クリップボードの窓が要る |
| user32.dll | `SetGestureConfig` | 無し | 遅延 | 134/134 | (a) | 未実装 | 遅延読み込みの枠: 名前つきの停止は未対応(GetProcAddress の経路) | 入力(フック・キーボード・タッチ)の窓が要る |
| user32.dll | `SetScrollPos` | 無し | 通常 | 134/134 | (a) | 未実装 | `fle: import user32.dll!SetScrollPos is not modeled` | スクロールバー(窓)が要る |
| user32.dll | `SetScrollRange` | 無し | 通常 | 134/134 | (a) | 未実装 | `fle: import user32.dll!SetScrollRange is not modeled` | スクロールバー(窓)が要る |
| user32.dll | `SetThreadDpiAwarenessContext` | 無し | 遅延 | 16/16 | (a) | 未実装 | 遅延読み込みの枠: 名前つきの停止は未対応(GetProcAddress の経路) | 窓(ウィンドウ・メッセージの待ち行列)が要る |
| user32.dll | `SetThreadDpiHostingBehavior` | 無し | 遅延 | 16/16 | (a) | 未実装 | 遅延読み込みの枠: 名前つきの停止は未対応(GetProcAddress の経路) | 窓(ウィンドウ・メッセージの待ち行列)が要る |
| user32.dll | `SetWindowLongA` | 無し | 通常 | 2/2 | (a) | 未実装 | `fle: import user32.dll!SetWindowLongA is not modeled` | 窓(ウィンドウ・メッセージの待ち行列)が要る |
| user32.dll | `SetWindowLongPtrA` | 無し | 通常 | 19/19 | (a) | 未実装 | `fle: import user32.dll!SetWindowLongPtrA is not modeled` | 窓(ウィンドウ・メッセージの待ち行列)が要る |
| user32.dll | `SetWindowLongW` | 無し | 通常 | 8/8 | (a) | 未実装 | `fle: import user32.dll!SetWindowLongW is not modeled` | 窓(ウィンドウ・メッセージの待ち行列)が要る |
| user32.dll | `SetWindowTextA` | 無し | 通常 | 3/3 | (a) | 未実装 | `fle: import user32.dll!SetWindowTextA is not modeled` | 窓(ウィンドウ・メッセージの待ち行列)が要る |
| user32.dll | `SetWindowsHookExA` | 無し | 通常 | 7/7 | (a) | 未実装 | `fle: import user32.dll!SetWindowsHookExA is not modeled` | 入力(フック・キーボード・タッチ)の窓が要る |
| user32.dll | `SystemParametersInfoA` | 無し | 通常 | 7/7 | (a) | 未実装 | `fle: import user32.dll!SystemParametersInfoA is not modeled` | 窓(ウィンドウ・メッセージの待ち行列)が要る |
| user32.dll | `TrackMouseEvent` | 無し | 通常 | 8/8 | (a) | 未実装 | `fle: import user32.dll!TrackMouseEvent is not modeled` | 入力(フック・キーボード・タッチ)の窓が要る |
| user32.dll | `TranslateMDISysAccel` | 無し | 通常 | 134/134 | (a) | 未実装 | `fle: import user32.dll!TranslateMDISysAccel is not modeled` | 窓(ウィンドウ・メッセージの待ち行列)が要る |
| user32.dll | `UnregisterClassA` | 無し | 通常 | 17/17 | (a) | 未実装 | `fle: import user32.dll!UnregisterClassA is not modeled` | 窓(ウィンドウ・メッセージの待ち行列)が要る |
| user32.dll | `WindowFromDC` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import user32.dll!WindowFromDC is not modeled` | 窓(ウィンドウ・メッセージの待ち行列)が要る |
| user32.dll | `wsprintfW` | 無し | 通常 | 3/3 | (a) | 未実装 | `fle: import user32.dll!wsprintfW is not modeled` | 窓(ウィンドウ・メッセージの待ち行列)が要る |
| uxtheme.dll | `BeginBufferedPaint` | 無し | 遅延 | 134/134 | (a) | 未実装 | 遅延読み込みの枠: 名前つきの停止は未対応(GetProcAddress の経路) | 描画・入力の装置(Direct3D、DWM、テーマ、WIC、DirectInput)が要る |
| uxtheme.dll | `BufferedPaintSetAlpha` | 無し | 遅延 | 134/134 | (a) | 未実装 | 遅延読み込みの枠: 名前つきの停止は未対応(GetProcAddress の経路) | 描画・入力の装置(Direct3D、DWM、テーマ、WIC、DirectInput)が要る |
| uxtheme.dll | `BufferedPaintUnInit` | 無し | 遅延 | 134/134 | (a) | 未実装 | 遅延読み込みの枠: 名前つきの停止は未対応(GetProcAddress の経路) | 描画・入力の装置(Direct3D、DWM、テーマ、WIC、DirectInput)が要る |
| uxtheme.dll | `EndBufferedPaint` | 無し | 遅延 | 134/134 | (a) | 未実装 | 遅延読み込みの枠: 名前つきの停止は未対応(GetProcAddress の経路) | 描画・入力の装置(Direct3D、DWM、テーマ、WIC、DirectInput)が要る |
| vcruntime140.dll | `_CxxThrowException` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import vcruntime140.dll!_CxxThrowException is not modeled` | C ランタイムの残り(例外・初期化・ヒープ・終了)。未実装 |
| vcruntime140.dll | `__C_specific_handler` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import vcruntime140.dll!__C_specific_handler is not modeled` | C ランタイムの残り(例外・初期化・ヒープ・終了)。未実装 |
| vcruntime140.dll | `__CxxDetectRethrow` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import vcruntime140.dll!__CxxDetectRethrow is not modeled` | C ランタイムの残り(例外・初期化・ヒープ・終了)。未実装 |
| vcruntime140.dll | `__CxxExceptionFilter` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import vcruntime140.dll!__CxxExceptionFilter is not modeled` | C ランタイムの残り(例外・初期化・ヒープ・終了)。未実装 |
| vcruntime140.dll | `__CxxQueryExceptionSize` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import vcruntime140.dll!__CxxQueryExceptionSize is not modeled` | C ランタイムの残り(例外・初期化・ヒープ・終了)。未実装 |
| vcruntime140.dll | `__CxxRegisterExceptionObject` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import vcruntime140.dll!__CxxRegisterExceptionObject is not modeled` | C ランタイムの残り(例外・初期化・ヒープ・終了)。未実装 |
| vcruntime140.dll | `__CxxUnregisterExceptionObject` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import vcruntime140.dll!__CxxUnregisterExceptionObject is not modeled` | C ランタイムの残り(例外・初期化・ヒープ・終了)。未実装 |
| vcruntime140.dll | `__FrameUnwindFilter` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import vcruntime140.dll!__FrameUnwindFilter is not modeled` | C ランタイムの残り(例外・初期化・ヒープ・終了)。未実装 |
| vcruntime140.dll | `__current_exception` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import vcruntime140.dll!__current_exception is not modeled` | C ランタイムの残り(例外・初期化・ヒープ・終了)。未実装 |
| vcruntime140.dll | `__current_exception_context` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import vcruntime140.dll!__current_exception_context is not modeled` | C ランタイムの残り(例外・初期化・ヒープ・終了)。未実装 |
| vcruntime140.dll | `__std_exception_copy` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import vcruntime140.dll!__std_exception_copy is not modeled` | C ランタイムの残り(例外・初期化・ヒープ・終了)。未実装 |
| vcruntime140.dll | `__std_exception_destroy` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import vcruntime140.dll!__std_exception_destroy is not modeled` | C ランタイムの残り(例外・初期化・ヒープ・終了)。未実装 |
| vcruntime140.dll | `__std_type_info_destroy_list` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import vcruntime140.dll!__std_type_info_destroy_list is not modeled` | C ランタイムの残り(例外・初期化・ヒープ・終了)。未実装 |
| vcruntime140.dll | `memmove` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import vcruntime140.dll!memmove is not modeled` | C ランタイムの残り(例外・初期化・ヒープ・終了)。未実装 |
| vcruntime140.dll | `memset` | 無し | 通常 | 1/1 | (a) | 実装 | fle_imp_memset (既存の C を表に載せた) | 仕様どおりに実装(窓・ファイル・通信を要求しない) |
| version.dll | `GetFileVersionInfoA` | 無し | 通常 | 2/2 | (a) | 未実装 | `fle: import version.dll!GetFileVersionInfoA is not modeled` | バージョン情報を持つ実ファイルが要る |
| version.dll | `GetFileVersionInfoSizeA` | 無し | 通常 | 2/2 | (a) | 未実装 | `fle: import version.dll!GetFileVersionInfoSizeA is not modeled` | バージョン情報を持つ実ファイルが要る |
| version.dll | `VerQueryValueA` | 無し | 通常 | 3/3 | (a) | 未実装 | `fle: import version.dll!VerQueryValueA is not modeled` | バージョン情報を持つ実ファイルが要る |
| windowscodecs.dll | `WICConvertBitmapSource` | 無し | 遅延+通常 | 141/141 | (a) | 未実装 | `fle: import windowscodecs.dll!WICConvertBitmapSource is not modeled` | 描画・入力の装置(Direct3D、DWM、テーマ、WIC、DirectInput)が要る |
| winmm.dll | `joyGetDevCapsA` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import winmm.dll!joyGetDevCapsA is not modeled` | 装置(ジョイスティック)の状態が要る |
| winmm.dll | `joyGetNumDevs` | 無し | 通常 | 1/1 | (a) | 実装 | fle_imp_joyGetNumDevs (新規。0 台) | 仕様どおりに実装(窓・ファイル・通信を要求しない) |
| winmm.dll | `joyGetPosEx` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import winmm.dll!joyGetPosEx is not modeled` | 装置(ジョイスティック)の状態が要る |
| winmm.dll | `timeKillEvent` | 無し | 通常 | 8/8 | (a) | 未実装 | `fle: import winmm.dll!timeKillEvent is not modeled` | タイマーの通知(メッセージや窓)が要る |
| winmm.dll | `timeSetEvent` | 無し | 通常 | 8/8 | (a) | 未実装 | `fle: import winmm.dll!timeSetEvent is not modeled` | タイマーの通知(メッセージや窓)が要る |
| winspool.drv | `ClosePrinter` | 無し | 通常 | 43/43 | (a) | 未実装 | `fle: import winspool.drv!ClosePrinter is not modeled` | 印刷の装置(プリンタ)が要る |
| winspool.drv | `DocumentPropertiesW` | 無し | 通常 | 43/43 | (a) | 未実装 | `fle: import winspool.drv!DocumentPropertiesW is not modeled` | 印刷の装置(プリンタ)が要る |
| winspool.drv | `EnumPrintersW` | 無し | 通常 | 43/43 | (a) | 未実装 | `fle: import winspool.drv!EnumPrintersW is not modeled` | 印刷の装置(プリンタ)が要る |
| winspool.drv | `GetDefaultPrinterW` | 無し | 通常 | 43/43 | (a) | 未実装 | `fle: import winspool.drv!GetDefaultPrinterW is not modeled` | 印刷の装置(プリンタ)が要る |
| winspool.drv | `OpenPrinterW` | 無し | 通常 | 43/43 | (a) | 未実装 | `fle: import winspool.drv!OpenPrinterW is not modeled` | 印刷の装置(プリンタ)が要る |
| ws2_32.dll | `#1` | 無し | 通常 | 2/2 | (a) | 未実装 | `fle: import ws2_32.dll!#1 is not modeled` | ネットワーク・セッション・資源の実物が要る(外の世界) |
| ws2_32.dll | `#10` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import ws2_32.dll!#10 is not modeled` | ネットワーク・セッション・資源の実物が要る(外の世界) |
| ws2_32.dll | `#11` | 無し | 通常 | 2/2 | (a) | 未実装 | `fle: import ws2_32.dll!#11 is not modeled` | ネットワーク・セッション・資源の実物が要る(外の世界) |
| ws2_32.dll | `#111` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import ws2_32.dll!#111 is not modeled` | ネットワーク・セッション・資源の実物が要る(外の世界) |
| ws2_32.dll | `#112` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import ws2_32.dll!#112 is not modeled` | ネットワーク・セッション・資源の実物が要る(外の世界) |
| ws2_32.dll | `#115` | 無し | 通常 | 2/2 | (a) | 未実装 | `fle: import ws2_32.dll!#115 is not modeled` | ネットワーク・セッション・資源の実物が要る(外の世界) |
| ws2_32.dll | `#116` | 無し | 通常 | 2/2 | (a) | 未実装 | `fle: import ws2_32.dll!#116 is not modeled` | ネットワーク・セッション・資源の実物が要る(外の世界) |
| ws2_32.dll | `#13` | 無し | 通常 | 2/2 | (a) | 未実装 | `fle: import ws2_32.dll!#13 is not modeled` | ネットワーク・セッション・資源の実物が要る(外の世界) |
| ws2_32.dll | `#14` | 無し | 通常 | 2/2 | (a) | 未実装 | `fle: import ws2_32.dll!#14 is not modeled` | ネットワーク・セッション・資源の実物が要る(外の世界) |
| ws2_32.dll | `#15` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import ws2_32.dll!#15 is not modeled` | ネットワーク・セッション・資源の実物が要る(外の世界) |
| ws2_32.dll | `#151` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import ws2_32.dll!#151 is not modeled` | ネットワーク・セッション・資源の実物が要る(外の世界) |
| ws2_32.dll | `#16` | 無し | 通常 | 2/2 | (a) | 未実装 | `fle: import ws2_32.dll!#16 is not modeled` | ネットワーク・セッション・資源の実物が要る(外の世界) |
| ws2_32.dll | `#17` | 無し | 通常 | 2/2 | (a) | 未実装 | `fle: import ws2_32.dll!#17 is not modeled` | ネットワーク・セッション・資源の実物が要る(外の世界) |
| ws2_32.dll | `#18` | 無し | 通常 | 2/2 | (a) | 未実装 | `fle: import ws2_32.dll!#18 is not modeled` | ネットワーク・セッション・資源の実物が要る(外の世界) |
| ws2_32.dll | `#19` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import ws2_32.dll!#19 is not modeled` | ネットワーク・セッション・資源の実物が要る(外の世界) |
| ws2_32.dll | `#2` | 無し | 通常 | 2/2 | (a) | 未実装 | `fle: import ws2_32.dll!#2 is not modeled` | ネットワーク・セッション・資源の実物が要る(外の世界) |
| ws2_32.dll | `#20` | 無し | 通常 | 2/2 | (a) | 未実装 | `fle: import ws2_32.dll!#20 is not modeled` | ネットワーク・セッション・資源の実物が要る(外の世界) |
| ws2_32.dll | `#21` | 無し | 通常 | 2/2 | (a) | 未実装 | `fle: import ws2_32.dll!#21 is not modeled` | ネットワーク・セッション・資源の実物が要る(外の世界) |
| ws2_32.dll | `#22` | 無し | 通常 | 2/2 | (a) | 未実装 | `fle: import ws2_32.dll!#22 is not modeled` | ネットワーク・セッション・資源の実物が要る(外の世界) |
| ws2_32.dll | `#23` | 無し | 通常 | 2/2 | (a) | 未実装 | `fle: import ws2_32.dll!#23 is not modeled` | ネットワーク・セッション・資源の実物が要る(外の世界) |
| ws2_32.dll | `#3` | 無し | 通常 | 2/2 | (a) | 未実装 | `fle: import ws2_32.dll!#3 is not modeled` | ネットワーク・セッション・資源の実物が要る(外の世界) |
| ws2_32.dll | `#4` | 無し | 通常 | 2/2 | (a) | 未実装 | `fle: import ws2_32.dll!#4 is not modeled` | ネットワーク・セッション・資源の実物が要る(外の世界) |
| ws2_32.dll | `#5` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import ws2_32.dll!#5 is not modeled` | ネットワーク・セッション・資源の実物が要る(外の世界) |
| ws2_32.dll | `#52` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import ws2_32.dll!#52 is not modeled` | ネットワーク・セッション・資源の実物が要る(外の世界) |
| ws2_32.dll | `#57` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import ws2_32.dll!#57 is not modeled` | ネットワーク・セッション・資源の実物が要る(外の世界) |
| ws2_32.dll | `#6` | 無し | 通常 | 2/2 | (a) | 未実装 | `fle: import ws2_32.dll!#6 is not modeled` | ネットワーク・セッション・資源の実物が要る(外の世界) |
| ws2_32.dll | `#7` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import ws2_32.dll!#7 is not modeled` | ネットワーク・セッション・資源の実物が要る(外の世界) |
| ws2_32.dll | `#8` | 無し | 通常 | 3/3 | (a) | 未実装 | `fle: import ws2_32.dll!#8 is not modeled` | ネットワーク・セッション・資源の実物が要る(外の世界) |
| ws2_32.dll | `#9` | 無し | 通常 | 2/2 | (a) | 未実装 | `fle: import ws2_32.dll!#9 is not modeled` | ネットワーク・セッション・資源の実物が要る(外の世界) |
| ws2_32.dll | `WSAAddressToStringW` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import ws2_32.dll!WSAAddressToStringW is not modeled` | ネットワーク・セッション・資源の実物が要る(外の世界) |
| ws2_32.dll | `WSACloseEvent` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import ws2_32.dll!WSACloseEvent is not modeled` | ネットワーク・セッション・資源の実物が要る(外の世界) |
| ws2_32.dll | `WSACreateEvent` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import ws2_32.dll!WSACreateEvent is not modeled` | ネットワーク・セッション・資源の実物が要る(外の世界) |
| ws2_32.dll | `WSAEnumNetworkEvents` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import ws2_32.dll!WSAEnumNetworkEvents is not modeled` | ネットワーク・セッション・資源の実物が要る(外の世界) |
| ws2_32.dll | `WSAEventSelect` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import ws2_32.dll!WSAEventSelect is not modeled` | ネットワーク・セッション・資源の実物が要る(外の世界) |
| ws2_32.dll | `WSAGetOverlappedResult` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import ws2_32.dll!WSAGetOverlappedResult is not modeled` | ネットワーク・セッション・資源の実物が要る(外の世界) |
| ws2_32.dll | `WSAIoctl` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import ws2_32.dll!WSAIoctl is not modeled` | ネットワーク・セッション・資源の実物が要る(外の世界) |
| ws2_32.dll | `WSARecv` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import ws2_32.dll!WSARecv is not modeled` | ネットワーク・セッション・資源の実物が要る(外の世界) |
| ws2_32.dll | `WSARecvFrom` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import ws2_32.dll!WSARecvFrom is not modeled` | ネットワーク・セッション・資源の実物が要る(外の世界) |
| ws2_32.dll | `WSAResetEvent` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import ws2_32.dll!WSAResetEvent is not modeled` | ネットワーク・セッション・資源の実物が要る(外の世界) |
| ws2_32.dll | `WSASend` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import ws2_32.dll!WSASend is not modeled` | ネットワーク・セッション・資源の実物が要る(外の世界) |
| ws2_32.dll | `WSASendTo` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import ws2_32.dll!WSASendTo is not modeled` | ネットワーク・セッション・資源の実物が要る(外の世界) |
| ws2_32.dll | `WSASetEvent` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import ws2_32.dll!WSASetEvent is not modeled` | ネットワーク・セッション・資源の実物が要る(外の世界) |
| ws2_32.dll | `WSASocketW` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import ws2_32.dll!WSASocketW is not modeled` | ネットワーク・セッション・資源の実物が要る(外の世界) |
| ws2_32.dll | `WSAStringToAddressW` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import ws2_32.dll!WSAStringToAddressW is not modeled` | ネットワーク・セッション・資源の実物が要る(外の世界) |
| ws2_32.dll | `WSAWaitForMultipleEvents` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import ws2_32.dll!WSAWaitForMultipleEvents is not modeled` | ネットワーク・セッション・資源の実物が要る(外の世界) |
| ws2_32.dll | `freeaddrinfo` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import ws2_32.dll!freeaddrinfo is not modeled` | ネットワーク・セッション・資源の実物が要る(外の世界) |
| ws2_32.dll | `getaddrinfo` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import ws2_32.dll!getaddrinfo is not modeled` | ネットワーク・セッション・資源の実物が要る(外の世界) |
| ws2_32.dll | `inet_ntop` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import ws2_32.dll!inet_ntop is not modeled` | ネットワーク・セッション・資源の実物が要る(外の世界) |
| ws2_32.dll | `inet_pton` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import ws2_32.dll!inet_pton is not modeled` | ネットワーク・セッション・資源の実物が要る(外の世界) |
| wsock32.dll | `WSACleanup` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import wsock32.dll!WSACleanup is not modeled` | ネットワーク・セッション・資源の実物が要る(外の世界) |
| wsock32.dll | `WSAGetLastError` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import wsock32.dll!WSAGetLastError is not modeled` | ネットワーク・セッション・資源の実物が要る(外の世界) |
| wsock32.dll | `WSAStartup` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import wsock32.dll!WSAStartup is not modeled` | ネットワーク・セッション・資源の実物が要る(外の世界) |
| wsock32.dll | `closesocket` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import wsock32.dll!closesocket is not modeled` | ネットワーク・セッション・資源の実物が要る(外の世界) |
| wsock32.dll | `htons` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import wsock32.dll!htons is not modeled` | ネットワーク・セッション・資源の実物が要る(外の世界) |
| wsock32.dll | `inet_addr` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import wsock32.dll!inet_addr is not modeled` | ネットワーク・セッション・資源の実物が要る(外の世界) |
| wsock32.dll | `sendto` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import wsock32.dll!sendto is not modeled` | ネットワーク・セッション・資源の実物が要る(外の世界) |
| wsock32.dll | `socket` | 無し | 通常 | 1/1 | (a) | 未実装 | `fle: import wsock32.dll!socket is not modeled` | ネットワーク・セッション・資源の実物が要る(外の世界) |
| wtsapi32.dll | `WTSUnRegisterSessionNotification` | 無し | 遅延 | 134/134 | (a) | 未実装 | 遅延読み込みの枠: 名前つきの停止は未対応(GetProcAddress の経路) | ネットワーク・セッション・資源の実物が要る(外の世界) |

## 6. DLL ごと

| DLL(Plugins\ の下) | 枠の数 | runtime に無い枠の数 |
|---|---|---|
| `Plugins\Fruity\Effects\Control Surface\Control Surface_x64.dll` | 636 | 88 |
| `Plugins\Fruity\Effects\Distructor\Distructor_x64.dll` | 543 | 44 |
| `Plugins\Fruity\Effects\Distructor\module\Module_x64.dll` | 576 | 56 |
| `Plugins\Fruity\Effects\EQUO\EQUO_x64.dll` | 544 | 44 |
| `Plugins\Fruity\Effects\Edison\Data\Scripts\Examples\testdll_x64.dll` | 36 | 2 |
| `Plugins\Fruity\Effects\Edison\Edison_x64.dll` | 654 | 83 |
| `Plugins\Fruity\Effects\Effector\Effector_x64.dll` | 543 | 44 |
| `Plugins\Fruity\Effects\Effector\engine_x64.dll` | 128 | 8 |
| `Plugins\Fruity\Effects\Emphasis\Emphasis_x64.dll` | 560 | 51 |
| `Plugins\Fruity\Effects\Emphasizer\Emphasizer_x64.dll` | 548 | 45 |
| `Plugins\Fruity\Effects\FL Studio Mobile Rack FX\FL Studio Mobile Rack FX_x64.dll` | 200 | 46 |
| `Plugins\Fruity\Effects\Frequency Shifter\Engine_x64.dll` | 73 | 0 |
| `Plugins\Fruity\Effects\Frequency Shifter\Frequency Shifter_x64.dll` | 548 | 44 |
| `Plugins\Fruity\Effects\Frequency Splitter\Frequency Splitter_x64.dll` | 568 | 56 |
| `Plugins\Fruity\Effects\Frequency Splitter\engine_x64.dll` | 77 | 0 |
| `Plugins\Fruity\Effects\Fruity 7 Band EQ\Fruity 7 Band EQ_x64.dll` | 543 | 44 |
| `Plugins\Fruity\Effects\Fruity Balance\Fruity Balance_x64.dll` | 543 | 44 |
| `Plugins\Fruity\Effects\Fruity Bass Boost\Fruity Bass Boost_x64.dll` | 543 | 44 |
| `Plugins\Fruity\Effects\Fruity Big Clock\Fruity Big Clock_x64.dll` | 543 | 44 |
| `Plugins\Fruity\Effects\Fruity Blood Overdrive\Fruity Blood Overdrive_x64.dll` | 548 | 45 |
| `Plugins\Fruity\Effects\Fruity Center\Fruity Center_x64.dll` | 543 | 44 |
| `Plugins\Fruity\Effects\Fruity Chorus\Fruity Chorus_x64.dll` | 543 | 44 |
| `Plugins\Fruity\Effects\Fruity Compressor\Fruity Compressor_x64.dll` | 543 | 44 |
| `Plugins\Fruity\Effects\Fruity Convolver\Fruity Convolver_x64.dll` | 646 | 78 |
| `Plugins\Fruity\Effects\Fruity Delay 2\Fruity Delay 2_x64.dll` | 543 | 44 |
| `Plugins\Fruity\Effects\Fruity Delay 3\Fruity Delay 3_x64.dll` | 551 | 50 |
| `Plugins\Fruity\Effects\Fruity Delay 3\engine_x64.dll` | 224 | 64 |
| `Plugins\Fruity\Effects\Fruity Delay Bank\Fruity Delay Bank_x64.dll` | 543 | 44 |
| `Plugins\Fruity\Effects\Fruity Delay\Fruity Delay_x64.dll` | 543 | 44 |
| `Plugins\Fruity\Effects\Fruity Fast Dist\Fruity Fast Dist_x64.dll` | 543 | 44 |
| `Plugins\Fruity\Effects\Fruity Fast LP\Fruity Fast LP_x64.dll` | 543 | 44 |
| `Plugins\Fruity\Effects\Fruity Filter\Fruity Filter_x64.dll` | 543 | 44 |
| `Plugins\Fruity\Effects\Fruity Flanger\Fruity Flanger_x64.dll` | 543 | 44 |
| `Plugins\Fruity\Effects\Fruity Flangus\Fruity Flangus_x64.dll` | 544 | 44 |
| `Plugins\Fruity\Effects\Fruity Formula Controller\Fruity Formula Controller_x64.dll` | 548 | 45 |
| `Plugins\Fruity\Effects\Fruity Free Filter\Fruity Free Filter_x64.dll` | 543 | 44 |
| `Plugins\Fruity\Effects\Fruity HTML NoteBook\Fruity HTML NoteBook_x64.dll` | 566 | 56 |
| `Plugins\Fruity\Effects\Fruity LSD\Fruity LSD_x64.dll` | 579 | 62 |
| `Plugins\Fruity\Effects\Fruity Limiter\Fruity Limiter_x64.dll` | 543 | 44 |
| `Plugins\Fruity\Effects\Fruity Love Philter\Fruity Love Philter_x64.dll` | 639 | 76 |
| `Plugins\Fruity\Effects\Fruity Multiband Compressor\Fruity Multiband Compressor_x64.dll` | 543 | 44 |
| `Plugins\Fruity\Effects\Fruity Multiband Compressor\comp_eng_x64.dll` | 88 | 3 |
| `Plugins\Fruity\Effects\Fruity Mute 2\Fruity Mute 2_x64.dll` | 543 | 44 |
| `Plugins\Fruity\Effects\Fruity NoteBook 2\Fruity NoteBook 2_x64.dll` | 548 | 45 |
| `Plugins\Fruity\Effects\Fruity NoteBook\Fruity NoteBook_x64.dll` | 557 | 57 |
| `Plugins\Fruity\Effects\Fruity PanOMatic\Fruity PanOMatic_x64.dll` | 543 | 44 |
| `Plugins\Fruity\Effects\Fruity Parametric EQ 2\Fruity Parametric EQ 2_x64.dll` | 548 | 44 |
| `Plugins\Fruity\Effects\Fruity Parametric EQ\Fruity Parametric EQ_x64.dll` | 543 | 44 |
| `Plugins\Fruity\Effects\Fruity Peak Controller\Fruity Peak Controller_x64.dll` | 555 | 50 |
| `Plugins\Fruity\Effects\Fruity Phase Inverter\Fruity Phase Inverter_x64.dll` | 543 | 44 |
| `Plugins\Fruity\Effects\Fruity Phaser\Fruity Phaser_x64.dll` | 543 | 44 |
| `Plugins\Fruity\Effects\Fruity Reeverb 2\Fruity Reeverb 2_x64.dll` | 544 | 44 |
| `Plugins\Fruity\Effects\Fruity Reeverb\Fruity Reeverb_x64.dll` | 543 | 44 |
| `Plugins\Fruity\Effects\Fruity Scratcher\Fruity Scratcher_x64.dll` | 579 | 66 |
| `Plugins\Fruity\Effects\Fruity Scratcher\engine_x64.dll` | 72 | 0 |
| `Plugins\Fruity\Effects\Fruity Send\Fruity Send_x64.dll` | 543 | 44 |
| `Plugins\Fruity\Effects\Fruity Soft Clipper\Fruity Soft Clipper_x64.dll` | 543 | 44 |
| `Plugins\Fruity\Effects\Fruity Spectroman\Fruity Spectroman_x64.dll` | 543 | 44 |
| `Plugins\Fruity\Effects\Fruity Squeeze\Fruity Squeeze_x64.dll` | 543 | 44 |
| `Plugins\Fruity\Effects\Fruity Stereo Enhancer\Fruity Stereo Enhancer_x64.dll` | 543 | 44 |
| `Plugins\Fruity\Effects\Fruity Stereo Shaper\Fruity Stereo Shaper_x64.dll` | 543 | 44 |
| `Plugins\Fruity\Effects\Fruity Vocoder\Fruity Vocoder_x64.dll` | 543 | 44 |
| `Plugins\Fruity\Effects\Fruity WaveShaper\Fruity WaveShaper_x64.dll` | 638 | 76 |
| `Plugins\Fruity\Effects\Fruity Wrapper\Fruity Wrapper_x64.dll` | 631 | 81 |
| `Plugins\Fruity\Effects\Fruity X-Y Controller\Fruity X-Y Controller_x64.dll` | 545 | 45 |
| `Plugins\Fruity\Effects\Fruity X-Y-Z Controller\Fruity X-Y-Z Controller_x64.dll` | 639 | 77 |
| `Plugins\Fruity\Effects\Fruity dB Meter\Fruity dB meter_x64.dll` | 543 | 44 |
| `Plugins\Fruity\Effects\Gross Beat\Gross Beat_x64.dll` | 638 | 76 |
| `Plugins\Fruity\Effects\Hardcore\Hardcore_x64.dll` | 147 | 14 |
| `Plugins\Fruity\Effects\Hyper Chorus\Engine_x64.dll` | 77 | 1 |
| `Plugins\Fruity\Effects\Hyper Chorus\Hyper Chorus_x64.dll` | 548 | 44 |
| `Plugins\Fruity\Effects\Low Lifter\Low Lifter_x64.dll` | 543 | 44 |
| `Plugins\Fruity\Effects\Luxeverb\LuxeVerb_x64.dll` | 560 | 50 |
| `Plugins\Fruity\Effects\Maximus\Maximus_x64.dll` | 638 | 76 |
| `Plugins\Fruity\Effects\Multiband Delay\Multiband Delay_x64.dll` | 551 | 50 |
| `Plugins\Fruity\Effects\Multiband Delay\engine_x64.dll` | 73 | 0 |
| `Plugins\Fruity\Effects\Newtime\Newtime_x64.dll` | 641 | 79 |
| `Plugins\Fruity\Effects\Newtone\Newtone_Engine_x64.dll` | 275 | 36 |
| `Plugins\Fruity\Effects\Newtone\Newtone_x64.dll` | 641 | 79 |
| `Plugins\Fruity\Effects\Patcher\Patcher_x64.dll` | 646 | 76 |
| `Plugins\Fruity\Effects\Pitch Shifter\Pitch Shifter_x64.dll` | 560 | 50 |
| `Plugins\Fruity\Effects\Pitcher\Pitcher_x64.dll` | 133 | 9 |
| `Plugins\Fruity\Effects\Razer Chroma\Razer Chroma_x64.dll` | 558 | 50 |
| `Plugins\Fruity\Effects\Soundgoodizer\Soundgoodizer_x64.dll` | 638 | 76 |
| `Plugins\Fruity\Effects\Spreader\Spreader_x64.dll` | 548 | 45 |
| `Plugins\Fruity\Effects\Transient Processor\Transient Processor_x64.dll` | 552 | 50 |
| `Plugins\Fruity\Effects\Transient Processor\engine_x64.dll` | 73 | 0 |
| `Plugins\Fruity\Effects\Transmitter\Transmitter_x64.dll` | 560 | 51 |
| `Plugins\Fruity\Effects\Transporter\Transporter_x64.dll` | 560 | 51 |
| `Plugins\Fruity\Effects\Tuner\Tuner_x64.dll` | 551 | 50 |
| `Plugins\Fruity\Effects\Tuner\engine_x64.dll` | 73 | 0 |
| `Plugins\Fruity\Effects\VFX Color Mapper\VFX Color Mapper_x64.dll` | 555 | 50 |
| `Plugins\Fruity\Effects\VFX Envelope\VFX Envelope_x64.dll` | 638 | 76 |
| `Plugins\Fruity\Effects\VFX Key Mapper\VFX Key Mapper_x64.dll` | 555 | 50 |
| `Plugins\Fruity\Effects\VFX Keyboard Splitter\VFX Keyboard Splitter_x64.dll` | 638 | 76 |
| `Plugins\Fruity\Effects\VFX Level Scaler\VFX Level Scaler_x64.dll` | 555 | 50 |
| `Plugins\Fruity\Effects\VFX Script\VFX Script_x64.dll` | 575 | 51 |
| `Plugins\Fruity\Effects\VFX Sequencer\VFX Sequencer_x64.dll` | 555 | 50 |
| `Plugins\Fruity\Effects\VFX Sequencer\engine_x64.dll` | 88 | 2 |
| `Plugins\Fruity\Effects\Vintage Chorus\Vintage Chorus_x64.dll` | 551 | 50 |
| `Plugins\Fruity\Effects\Vintage Chorus\engine_x64.dll` | 73 | 0 |
| `Plugins\Fruity\Effects\Vintage Phaser\Vintage Phaser_x64.dll` | 551 | 50 |
| `Plugins\Fruity\Effects\Vintage Phaser\engine_x64.dll` | 73 | 0 |
| `Plugins\Fruity\Effects\Vocodex\Synthesizer\Sytrus_x64.dll` | 639 | 76 |
| `Plugins\Fruity\Effects\Vocodex\Vocodex_x64.dll` | 638 | 76 |
| `Plugins\Fruity\Effects\Wave Candy\Wave Candy_x64.dll` | 573 | 62 |
| `Plugins\Fruity\Effects\ZGameEditor Visualizer\Processing.NDI.Lib.x64.dll` | 259 | 111 |
| `Plugins\Fruity\Effects\ZGameEditor Visualizer\SpoutLibrary.dll` | 205 | 73 |
| `Plugins\Fruity\Effects\ZGameEditor Visualizer\ZGEBullet_x64.dll` | 74 | 3 |
| `Plugins\Fruity\Effects\ZGameEditor Visualizer\ZGameEditor Visualizer_x64.dll` | 682 | 120 |
| `Plugins\Fruity\Effects\ZGameEditor Visualizer\ZgeNano_x64.dll` | 102 | 24 |
| `Plugins\Fruity\Effects\ZGameEditor Visualizer\ZgeSkelet_x64.dll` | 81 | 9 |
| `Plugins\Fruity\Generators\3x Osc\3x Osc_x64.dll` | 578 | 66 |
| `Plugins\Fruity\Generators\3x Osc\engine_x64.dll` | 76 | 1 |
| `Plugins\Fruity\Generators\Autogun\Autogun_x64.dll` | 639 | 76 |
| `Plugins\Fruity\Generators\BassDrum\BassDrum_x64.dll` | 558 | 50 |
| `Plugins\Fruity\Generators\BassDrum\engine_x64.dll` | 147 | 14 |
| `Plugins\Fruity\Generators\BeepMap\BeepMap_x64.dll` | 579 | 66 |
| `Plugins\Fruity\Generators\BooBass\BooBass_Eng_x64.dll` | 95 | 4 |
| `Plugins\Fruity\Generators\BooBass\BooBass_x64.dll` | 543 | 44 |
| `Plugins\Fruity\Generators\Dashboard\Dashboard_x64.dll` | 605 | 71 |
| `Plugins\Fruity\Generators\DirectWave\DirectWave_x64.dll` | 171 | 30 |
| `Plugins\Fruity\Generators\DirectWave\editor_x64.dll` | 641 | 78 |
| `Plugins\Fruity\Generators\Drumaxx\Drumaxx_x64.dll` | 578 | 66 |
| `Plugins\Fruity\Generators\Drumaxx\engine_x64.dll` | 151 | 19 |
| `Plugins\Fruity\Generators\Drumpad\Drumpad_x64.dll` | 560 | 50 |
| `Plugins\Fruity\Generators\Drumpad\engine_x64.dll` | 143 | 13 |
| `Plugins\Fruity\Generators\FL Keys\FL Keys_x64.dll` | 566 | 50 |
| `Plugins\Fruity\Generators\FL Slayer\FL Slayer_x64.dll` | 606 | 62 |
| `Plugins\Fruity\Generators\FL Slayer\Slayer_Engine.dll` | 55 | 5 |
| `Plugins\Fruity\Generators\FL Studio Mobile Rack\FL Studio Mobile Rack_x64.dll` | 200 | 46 |
| `Plugins\Fruity\Generators\FL Studio Mobile\FL Studio Mobile_x64.dll` | 228 | 70 |
| `Plugins\Fruity\Generators\FL Studio Mobile\Installed\DLL\FLMManagedBridge_x64.dll` | 47 | 33 |
| `Plugins\Fruity\Generators\FLEX\FLEX_x64.dll` | 618 | 57 |
| `Plugins\Fruity\Generators\FLEX\engineV2_x64.dll` | 342 | 109 |
| `Plugins\Fruity\Generators\FLEX\engine_x64.dll` | 338 | 107 |
| `Plugins\Fruity\Generators\FPC\FPC_x64.dll` | 646 | 77 |
| `Plugins\Fruity\Generators\Fruit Kick\Fruit Kick_x64.dll` | 578 | 66 |
| `Plugins\Fruity\Generators\Fruity DX10\Fruity DX10_x64.dll` | 578 | 66 |
| `Plugins\Fruity\Generators\Fruity Dance\Fruity dance_x64.dll` | 579 | 67 |
| `Plugins\Fruity\Generators\Fruity DrumSynth Live\Fruity DrumSynth Live_x64.dll` | 555 | 50 |
| `Plugins\Fruity\Generators\Fruity DrumSynth Live\drumsynth_eng_x64.dll` | 98 | 5 |
| `Plugins\Fruity\Generators\Fruity Envelope Controller\Fruity Envelope Controller_x64.dll` | 638 | 76 |
| `Plugins\Fruity\Generators\Fruity Granulizer\Fruity Granulizer_x64.dll` | 590 | 67 |
| `Plugins\Fruity\Generators\Fruity Keyboard Controller\Fruity Keyboard Controller_x64.dll` | 555 | 50 |
| `Plugins\Fruity\Generators\Fruity Slicer 2\Fruity Slicer 2_x64.dll` | 643 | 77 |
| `Plugins\Fruity\Generators\Fruity Slicer 2\engine_x64.dll` | 215 | 63 |
| `Plugins\Fruity\Generators\Fruity Slicer\Fruity Slicer_x64.dll` | 590 | 67 |
| `Plugins\Fruity\Generators\Fruity Video Player\Fruity Video Player_x64.dll` | 565 | 56 |
| `Plugins\Fruity\Generators\Fruity Voltage Controller\Fruity Voltage Controller_x64.dll` | 556 | 50 |
| `Plugins\Fruity\Generators\Fruity Wrapper\Fruity Wrapper_x64.dll` | 631 | 81 |
| `Plugins\Fruity\Generators\GMS\GMS_x64.dll` | 147 | 14 |
| `Plugins\Fruity\Generators\Harmless\Harmless_x64.dll` | 593 | 67 |
| `Plugins\Fruity\Generators\Harmor\Harmor_x64.dll` | 640 | 78 |
| `Plugins\Fruity\Generators\Kepler Exo\Kepler Exo_x64.dll` | 560 | 51 |
| `Plugins\Fruity\Generators\Kepler Exo\engine_x64.dll` | 213 | 62 |
| `Plugins\Fruity\Generators\Kepler\Kepler_x64.dll` | 543 | 44 |
| `Plugins\Fruity\Generators\Kepler\engine_x64.dll` | 213 | 62 |
| `Plugins\Fruity\Generators\MIDI Out\MIDI Out_x64.dll` | 555 | 50 |
| `Plugins\Fruity\Generators\Minisynth\MiniSynth_x64.dll` | 144 | 13 |
| `Plugins\Fruity\Generators\Morphine\Morphine_x64.dll` | 148 | 15 |
| `Plugins\Fruity\Generators\Ogun\Ogun_x64.dll` | 639 | 76 |
| `Plugins\Fruity\Generators\Patcher\Patcher_x64.dll` | 646 | 76 |
| `Plugins\Fruity\Generators\Plucked!\Plucked!_x64.dll` | 543 | 44 |
| `Plugins\Fruity\Generators\PoiZone\PoiZone_x64.dll` | 548 | 44 |
| `Plugins\Fruity\Generators\PoiZone\engine_x64.dll` | 145 | 14 |
| `Plugins\Fruity\Generators\ReWired\ReWired_x64.dll` | 561 | 52 |
| `Plugins\Fruity\Generators\Sakura\Sakura_x64.dll` | 558 | 50 |
| `Plugins\Fruity\Generators\Sakura\engine_x64.dll` | 145 | 14 |
| `Plugins\Fruity\Generators\Sawer\Sawer_x64.dll` | 561 | 50 |
| `Plugins\Fruity\Generators\Sawer\engine_x64.dll` | 145 | 14 |
| `Plugins\Fruity\Generators\SimSynth\SimSynth_Eng_x64.dll` | 95 | 4 |
| `Plugins\Fruity\Generators\SimSynth\SimSynth_x64.dll` | 543 | 44 |
| `Plugins\Fruity\Generators\Slicex\Slicex_x64.dll` | 648 | 79 |
| `Plugins\Fruity\Generators\Soundfont Player\Soundfont Player_x64.dll` | 557 | 50 |
| `Plugins\Fruity\Generators\Sytrus\Sytrus_x64.dll` | 639 | 76 |
| `Plugins\Fruity\Generators\Toxic Biohazard\Toxic Biohazard_x64.dll` | 146 | 14 |
| `Plugins\Fruity\Generators\Transistor Bass\Transistor Bass_x64.dll` | 553 | 46 |
| `Plugins\Fruity\Generators\Transistor Bass\engine_x64.dll` | 219 | 63 |
| `Plugins\Fruity\Generators\Wasp XT\Wasp XT_x64.dll` | 543 | 44 |
| `Plugins\Fruity\Generators\Wasp\Wasp_x64.dll` | 543 | 44 |
| `Plugins\Fruity\Generators\Wave Traveller\Wave Traveller_x64.dll` | 574 | 65 |

