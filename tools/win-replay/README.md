# tools/win-replay — 実機 Windows での API 応答の再生ツール（A2b 用）

決定 0046 §3 の A2b（「実行で踏まれる import 応答が全て仕様確認済み」）を、**実機の Windows の値**で確認するための道具。
6 モジュール完走の呼び出し記録（`docs/investigations/a4-six-module-trace/trace.tsv.gz`、13,572 呼び出し・171 種）から
引数の形を取り出し、**同じ API を実機で呼んで**、戻り値・`GetLastError`・出力を記録する。プロジェクトの runtime が返した値（トレース）との突き合わせは `compare.py`。

- FL Studio のコード・バイナリ・インストーラは**一切使わない**。`replay.c` は Windows の標準ヘッダと標準 DLL だけを使う独立した小さなプログラム。Windows 側に FL Studio を入れてはいけない。
- 凡例（このファイルでも他の記録でも）: **実測**＝このツールの出力そのもの／**読み**＝私の解釈／**未確認**。
- 状態: wine 10.0 + mingw-w64 での煙テストだけ実施済み（`SMOKE-TEST.md`。**wine の値は実機 Windows の値ではない**）。MSVC でのビルド・実機 Windows での実行は**未確認**。

## ファイル

| ファイル | 内容 |
|---|---|
| `replay.c` | 再生プログラム本体（単一ファイル、x64 専用）。MSVC・mingw-w64 の両方で通る書き方（mingw-w64 gcc 13 でコンパイル確認済み、MSVC は未確認） |
| `cases.tsv` | 再生するケース 2,144 件（生成物。`gen_cases.py` で再生成可。LF 改行、約 290 KB） |
| `gen_cases.py` | トレースから `cases.tsv` と `coverage.tsv` を作る（Python 3 標準ライブラリのみ。Windows 側では不要） |
| `compare.py` | `observed.tsv` とトレースの runtime 応答を突き合わせて `compare-report.md` を作る（Python 3 標準ライブラリのみ） |
| `coverage.tsv` | API ごとの「監査の分類（S/D/P/X/N）・このツールでの扱い（M/R/L/K）・ケース数」。M＝実機の値が仕様の項目になりうる（108 種）、R＝機種・ユーザー・時刻・画面・起動方法に依存し参考のみ（43 種）、L＝VM のロケールに依存（2 種）、K＝呼ばない（18 種） |
| `replay-win10.manifest` | 第 2 回の実行（Windows 10 互換 manifest あり）用。バージョン系 API が manifest で変わるかを見る |
| `layout_check.c` | `replay.c`/`gen_cases.py` が決め打ちしている構造体の大きさ・オフセット（約 75 個）のコンパイル時検査 |
| `SMOKE-TEST.md` | wine での煙テストの記録（仕様の確認ではない） |

## ビルド（管理者権限は不要）

```text
MSVC  (x64 Native Tools の端末、または vcvars64.bat 実行後):
  cd tools\win-replay
  mkdir out
  cl /nologo /O2 /W3 /MT replay.c /Foout\ /Feout\replay.exe /link /MANIFEST:NO

mingw-w64 / w64devkit の gcc:
  cd tools\win-replay
  mkdir out
  gcc -O2 -Wall -o out\replay.exe replay.c

Linux でのクロスコンパイル確認（このリポジトリで実施）:
  x86_64-w64-mingw32-gcc -O2 -Wall -o replay.exe replay.c
```

- `/MANIFEST:NO` は、`cl` が `replay.exe.manifest` を自動で生成して「manifest なし」の条件を崩すのを防ぐため。
- `-municode` などは不要（`main` を使い、引数は ASCII）。32 ビット版はビルドできない（`#error`）。
- 出力先 `out\` は `.gitignore` 済み。

## 実行

```text
out\replay.exe --cases cases.tsv --out observed.tsv --tag plain
```

| オプション | 意味 |
|---|---|
| `--cases FILE` | ケースの入力（既定 `cases.tsv`） |
| `--out FILE` | 出力（既定 `observed.tsv`）。実行中は `FILE.inflight` に今実行中のケース番号を書く |
| `--tag NAME` | 出力の先頭に記録する名前（`plain` / `win10-manifest` など） |
| `--resume` | 途中で落ちた（クラッシュ・強制終了）実行の続きから再開。落ちたケースは `crash:…` または `hang-or-killed` の行になり、次のケースから続ける |
| `--no-mask` | プライバシーのマスクを外す（既定ではコンピュータ名・ユーザー名・プロファイルパスを `<COMPUTER>` `<USER>` `<PROFILE>` に置換し、GetComputerNameExW の値は長さと文字種だけ出す） |
| `--only A,B` | 指定した API のケースだけ実行（変数を共有する場面の途中の行は失敗しうる。調べ物用） |
| `--limit N` | 先頭から N 件だけ |
| `--dpi-aware` | 起動時に `SetProcessDPIAware()` を呼ぶ（DPI 拡大が 100% でない環境で、画面系の値の違いを見る任意の第 3 回用） |
| `--list-whitelist` | ホワイトリストを出力して終了 |

終了コード: 0＝完走、2＝引数・ファイルのエラー、3＝呼んだ API がクラッシュした（`--resume` で続けられる）。
所要時間: wine で約 2 秒（実機は**未確認**。LoadLibrary で DLL を読むので数秒から数十秒の見込み＝読み）。ネットワークは使わない。

### 第 2 回（Windows 10 互換 manifest あり）

```text
copy out\replay.exe out\replay-m10.exe
copy replay-win10.manifest out\replay-m10.exe.manifest
out\replay-m10.exe --cases cases.tsv --out observed-manifest.tsv --tag win10-manifest
```

- manifest なしのプロセスでは `GetVersion`/`GetVersionExW`/`VerifyVersionInfoW` が実際の OS と違う版（Windows 8 相当）を答える、と一般に言われる（読み。この実行で実測する）。FL Studio 本体の exe の manifest は**未確認**なので、両方を測る。
- 外部 manifest は、exe に埋め込み manifest が無いときだけ使われる。コピー（別名）の exe で実行すること（同名の exe の manifest はキャッシュされることがある＝読み）。

## 出力 `observed.tsv`

先頭に `# key<TAB>value` の環境行（`RtlGetVersion`＝manifest に左右されない OS 版、`ACP`、`OEMCP`、コンパイラ、時刻 など）。続いて 1 ケース 1 行（`cases.tsv` と同じ `case_id` の順）:

`case_id  api  status  ret  last_error  le_changed  outs  decoded  extra`

- `status`: `ok`／`skip:policy`（ツールが拒否。理由は `extra`）／`skip:not-whitelisted`／`nomodule`（その DLL が実機に無い）／`noexport`（その DLL にその API の export が無い）／`err:…`／`crash:0x…`／`hang-or-killed`。
- `ret`: 戻り値（16 進）。`void` は `-`。
- `last_error` と `le_changed`: 呼ぶ直前に `SetLastError(0x5EED5EED)` を入れ、直後に `GetLastError()`。`le_changed=0` は「その API が last error に触れなかった」。
- `outs`: 出力バッファのうち、呼んだ後に内容が変わったものの hex（`a<引数番号>=…`）。`@peek` 系は `m=…`。
- `decoded`: 構造体・文字列の主要フィールド。**トレースの `decoded_outputs` と同じ書式**（突き合わせのため）。
- `extra`: ツール固有の測定（`align=`＝戻りアドレスの整列、`objtype=`＝GDI オブジェクトの種類、`modfile=`＝モジュールの実パス、`sam_orig=`＝元の samDesired など）。

## ホワイトリスト（呼ぶ API は 174 種）

`replay.exe --list-whitelist` の出力が正。**ここに無い API 名のケースは `skip:not-whitelisted` になり、呼ばれない。** 窓の作成・スレッド・ファイルへの書き込み・レジストリへの書き込み・ネットワーク・クリップボード・グローバルアトムの API は表に無い。

トレースにある API（153 種。★は引数を検査して制限しているもの）:

- kernel32（89）: AcquireSRWLockExclusive CompareStringW DecodePointer DeleteCriticalSection EncodePointer EnterCriticalSection FindFirstFileW★ FlsAlloc★ FlsGetValue FlsSetValue FreeEnvironmentStringsW FreeLibrary★ GetACP GetCPInfo GetCommandLineA GetCommandLineW GetComputerNameExW GetCurrentProcess GetCurrentProcessId GetCurrentThread GetCurrentThreadId GetEnvironmentStringsW GetEnvironmentVariableW★ GetFileAttributesExW★ GetFileAttributesW★ GetLastError GetLocalTime GetLocaleInfoW GetLogicalProcessorInformation GetModuleFileNameA★ GetModuleFileNameW★ GetModuleHandleA★ GetModuleHandleW★ GetNativeSystemInfo GetProcAddress GetProcessHeap GetStartupInfoA GetStartupInfoW GetStdHandle GetStringTypeW GetSystemDefaultLocaleName GetSystemInfo GetSystemTimeAsFileTime GetTempPathW GetThreadLocale GetThreadPreferredUILanguages GetThreadUILanguage GetTickCount GetTimeZoneInformation GetUserDefaultUILanguage GetVersion GetVersionExA GetVersionExW HeapAlloc★ HeapCreate★ HeapFree★ HeapReAlloc★ HeapSetInformation★ HeapSize★ InitializeCriticalSection InitializeCriticalSectionAndSpinCount InitializeSListHead IsValidCodePage IsValidLocale LCMapStringW LeaveCriticalSection LoadLibraryA★ LoadLibraryExW★ LoadLibraryW★ LocalAlloc★ MulDiv MultiByteToWideChar QueryPerformanceCounter QueryPerformanceFrequency ReleaseSRWLockExclusive SetDllDirectoryW★ SetErrorMode★ SetHandleCount SetLastError SetThreadErrorMode★ SetThreadLocale★ TlsAlloc TlsGetValue TlsSetValue VerSetConditionMask VerifyVersionInfoW VirtualAlloc★ VirtualProtect★ WideCharToMultiByte
- user32（23）: CharLowerBuffW CharLowerW CharNextW CharUpperBuffW CreateIconIndirect DestroyCursor★ EnumDisplayMonitors★ EnumDisplaySettingsW★ GetClassInfoW★ GetClipCursor GetClipboardFormatNameW GetDC★ GetKeyboardLayout GetMonitorInfoW GetSysColor GetSystemMetrics IntersectRect LoadCursorW★ LoadIconW★ LoadImageW★ LoadStringW★ ReleaseDC★ SystemParametersInfoW★
- gdi32（27）: CreateBitmap CreateBrushIndirect CreateCompatibleDC★ CreateDIBSection CreateFontIndirectW CreatePalette CreatePenIndirect DeleteDC★ DeleteObject★ EnumFontFamiliesExW★ GdiFlush GetCurrentPositionEx GetDIBColorTable GetDeviceCaps GetObjectW GetStockObject MoveToEx★ PatBlt★ SelectObject★ SelectPalette★ SetBkColor★ SetBkMode★ SetROP2★ SetStretchBltMode★ SetTextColor★ StretchDIBits★ UnrealizeObject★
- advapi32（5）: RegCloseKey★ RegOpenKeyExA★ RegOpenKeyExW★ RegQueryInfoKeyW★ RegQueryValueExW★
- shell32（2）: SHGetFolderPathW★（shfolder.dll 経由でも呼ぶ） SHGetMalloc
- oleaut32（3）: SysAllocStringLen★ SysFreeString★ SysReAllocStringLen★
- ole32 OleInitialize★、shcore GetProcessDpiAwareness★、msacm32 acmMetrics、version GetFileVersionInfoSizeW★

読み戻し用の補助（21 種。トレースには無い）: GetBkColor GetBkMode GetCurrentObject GetMapMode GetObjectType GetPolyFillMode GetROP2 GetStretchBltMode GetTextColor（gdi32）／FlsFree GetErrorMode GetOEMCP GetSystemDefaultLCID GetUserDefaultLCID GetUserDefaultLocaleName LocalFree TlsFree（kernel32）／RtlGetVersion（ntdll）／OleUninitialize（ole32）／DestroyIcon MonitorFromPoint（user32）。

### 検査の中身（★）

| API | 制限 |
|---|---|
| `SystemParametersInfoW` | 取得（GET）系の 20 種の番号だけ。`fWinIni=0`。SET 系は拒否 |
| `RegOpenKeyEx*` | 親は HKCU / HKLM のみ。**`samDesired` は常に `KEY_READ(0x20019)` に置換**（元の値は `extra` の `sam_orig`）。値の読み取りと閉じるのは、ここで開いたハンドルだけ |
| `SHGetFolderPathW` | `CSIDL_FLAG_CREATE(0x8000)` を拒否（フォルダを作らない）。`hwnd`/`hToken` は 0 |
| `LoadLibrary*` / `GetModuleHandle*` / `M:` トークン | **Windows のシステム DLL 名の許可リスト**（kernel32 user32 gdi32 advapi32 shell32 shfolder shcore ole32 oleaut32 comctl32 version msacm32 avrt powrprof dwmapi imm32 msctf ntdll kernelbase `api-ms-win-*`）だけ。FL Studio の DLL 名は拒否。起動時に `SetDefaultDllDirectories(LOAD_LIBRARY_SEARCH_SYSTEM32)` |
| `FreeLibrary` | このツールが `LoadLibrary` したハンドルだけ |
| `GetFileAttributes*` / `FindFirstFileW` / `GetFileVersionInfoSizeW` | `\\server\share` 形式（UNC）を拒否（ネットワークに出ない）。読み取りのみ |
| `GetEnvironmentVariableW` | 名前は 9 個の許可リスト（LANG SystemRoot windir OS PROCESSOR_ARCHITECTURE NUMBER_OF_PROCESSORS PATHEXT ComSpec とダミー）。値は最大 1024 文字を記録し、パスのユーザー名はマスク |
| `Heap*` / `LocalAlloc` / `VirtualAlloc` | このツールが作ったヒープ・ブロックだけ。サイズ上限 64 MB（VirtualAlloc は 16 MB で、確保後すぐ解放）。`HEAP_GENERATE_EXCEPTIONS` 拒否 |
| `VirtualProtect` | このプロセス自身の `.text` の 1 ページ、5 バイトだけ。呼んだ直後に元の保護に戻す |
| `GetDC` / `ReleaseDC` / `LoadImageW` / `LoadIconW` / `LoadCursorW` | `hwnd`/`hInstance` は NULL のみ（窓なし・FL Studio のリソースなし） |
| `CreateCompatibleDC` ほか DC 系 | このツールが作ったメモリ DC だけ。`PatBlt`/`StretchDIBits` はメモリ DC のビットマップにだけ描く |
| `DeleteObject` / `DestroyCursor` / `DestroyIcon` / `UnrealizeObject` | このツールが作ったオブジェクトだけ（ストックオブジェクトは拒否） |
| `EnumDisplayMonitors` / `EnumFontFamiliesExW` | コールバックは内蔵のもの（数えて文字列にするだけ） |
| `SetThreadLocale` / `SetErrorMode` / `SetDllDirectoryW` / `SetThreadErrorMode` | スレッド・プロセス内の状態だけ。値は固定のいくつか（ロケール 0x400／0x409／ここで読んだ値、エラーモードの 3 ビット、`Shared\` と NULL、0／1）に限り、直後のケースで元に戻す |
| `OleInitialize` | 引数 NULL のみ。同じ回数 `OleUninitialize` する |

### 呼ばない API（トレースにあるが実行しない 18 種）

| API | 理由 |
|---|---|
| CreateEventW SetEvent WaitForSingleObject CloseHandle | 監査では S/D で測る値が無い。WaitForSingleObject は無限待ちになりうる |
| RegisterWindowMessageW RegisterClipboardFormatW GlobalAddAtomW | セッション全体の原子表に登録する（副作用） |
| CreateDirectoryW WriteFile CreateFileW ReadFile SetFilePointer | ファイルシステムへの書き込み／ファイル操作（対象は FL Studio のファイルで、実機には無い） |
| CreateWindowExW DefWindowProcW SetWindowLongPtrW RegisterClassW | 窓・窓クラス |
| RegCreateKeyExW RegSetValueExW | レジストリへの書き込み（RegCreateKeyExW の代わりに同じキーを `RegOpenKeyExW` で読み取りのみで開き、有無を測る） |

### 副作用の自己申告（読み）

「読み取り専用」は**システムの状態を変えない**という意味で守る。プロセス内の状態（ヒープ・TLS/FLS・メモリ DC 上の GDI オブジェクト・スレッドのロケールとエラーモード・COM の初期化・DLL の読み込み）は変わるが、プロセス終了で消え、元に戻せるものは直後に戻す。ディスクへの書き込みは `--out` で指定した出力ファイルと `.inflight` だけ。

## 入力 `cases.tsv`

`case_id  kind  api  dll  args  capture  rtype  cls  trace_n  trace_seq  trace_ret  trace_le  trace_out  note`（C は先頭 6 列だけ読む）。

- `kind`: `trace`（トレースにあった引数の形）／`scn`（トレースの手順をなぞった場面の 1 手。ハンドルは `$変数`）／`probe`（トレースに無い。実機の挙動を面で測る標本）。件数: trace 513／scn 56／probe 1,575。runtime の応答と突き合わせられるのは `trace_n>0` の 569 件。
- `args`: `|` 区切りのトークン。`d:` 32 ビット整数、`i:` 64 ビット整数、`W:`/`U:` UTF-16 文字列（`U:` は生の UTF-16LE の hex）、`A:`/`X:` バイト列、`Z:n[:off=hex,...][:name]` ゼロ埋めのバッファ、`O:n[:name]` 出力用バッファ（0xCD で埋めて書き換わった所を検出）、`D:n[:name]` DWORD へのポインタ、`$name` 変数、`M:dll` システム DLL のハンドル、`F:` 内蔵コールバック。
- `@eq` `@peek` `@peekp` `@bstr` `@bstrp` `@box` `@poke` `@selfcode` は API ではなくツール内部の操作（変数の比較、確保したメモリの中身の読み出し、など）。
- **トレースに無いもの**: 入力側の構造体・バッファの中身（トレースは `ptr` としか残していない）。LOGFONTW・LOGBRUSH・BITMAPINFOHEADER・ICONINFO・LOGPALETTE・OSVERSIONINFOEX の中身は、文書化された構造体の配置と、トレースから読める寸法（32×32 など）で作った（`note` 列に「inferred」と書いた）。**元の呼び出しと同じ中身という保証は無い。**
- 各ケースの最初の `GetLastError` は `0x5EED5EED` の番兵で始める。したがって `GetLastError` のケース自体はツールが値を決める（compare では比較対象外）。

## 突き合わせ `compare.py`

```text
python compare.py --cases cases.tsv --observed observed.tsv --observed-manifest observed-manifest.tsv --report compare-report.md
```

- `match`（runtime の応答のどれかと一致）／`mismatch`（Windows が決める項目が違う）／`reference`（違うが、機種・ユーザー・時刻・画面・起動方法・VM のロケールに依存する項目）／`real-only`（runtime の応答が無い標本）／`skipped`。
- ハンドルとポインタは「NULL でない不透明値」としてだけ比べる。戻り値が失敗のときだけ last error を仕様項目として比べ、成功時の last error は参考（Microsoft は成功時の値を保証していない）。
- 機種依存の判断は、`cases.tsv` の `cls` 列（`gen_cases.py` の `CLS`）と、`compare.py` の `REF_FIELDS` / `LOCALE_APIS` / `locale_dependent()`。**この分類は私の読み**で、実機の値を見てから見直す前提。

## cases.tsv の再生成

```text
python tools/win-replay/gen_cases.py                      # cases.tsv と coverage.tsv を書く
python tools/win-replay/gen_cases.py --whitelist wl.tsv   # wl.tsv = replay.exe --list-whitelist の出力。ケースとホワイトリストの一致を確かめる
```

## このツールが測れないこと（限界）

- **機種・ユーザー依存の値**（コンピュータ名、ユーザー名、画面解像度、モニタ構成、DPI、時刻、プロセッサ数、インストール済みフォント、レジストリの利用者設定）は、値を記録するが**仕様の確認にはならない**（`reference`）。VM のロケールが en-US でないと、ロケール依存のケースは `reference` になる（runtime の「en-US / 1252」は環境の定義）。
- FL Studio 側の事情（FL Studio 本体の exe の manifest、DPI 宣言、実行時の引数、リソース DLL、フォントの配置）は測れない。
- 呼び出しの**途中の状態**（同じ API が前の呼び出しの結果に左右される場面）は、トレースの手順をなぞった `scn` の範囲だけ。
- ゲストが戻り値でどの枝に進んだか（影響の大きさ）は、このツールでは分からない。
- ケースが引数の形まで一致させているトレースの呼び出しは、13,572 回のうち 8,093 回（引数の形 7,777 回＋システム DLL への GetProcAddress 316 回）。残りは、同じ API の ASCII 文字列変換を代表例に畳んだもの（WideCharToMultiByte 3,011 回のうち 56 回だけが形まで一致。ASCII/UTF-8 の変換は代表例＋面の標本で測る）、FL Studio 自身の DLL への GetProcAddress（1,296 回）、FL Studio のリソースを読む LoadStringW（162 回）、イベントなど呼ばない API。A2b は API 単位の判定なので、API 種の数は「呼ぶ API」の節を参照。
