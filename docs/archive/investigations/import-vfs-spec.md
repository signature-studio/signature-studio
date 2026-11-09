# ファイル・レジストリ・環境 API の仕様と実装状況（担当 API の一覧）

作業: 作業者（M1）。日付: 2026-10-20。判断は `docs/decisions/0018-virtual-fs-reg-env.md`。実行記録は `docs/investigations/vfs-run.md`。
仕様の本文は learn.microsoft.com から取得し、`lift/out-vfs/spec/*.txt`（HTML を平文にしたもの。1 行目が URL）に残した。取得は `lift/out-vfs/fetch_spec.py` ほか。

## 1. 担当 API の一覧（閉包で呼ばれるもの）

数え方: 閉包 896 seed の「IAT 経由の未対応 jmp」（`lift/out-usage/iat-jmp-unsupported-all-v3.tsv`、`iat-slot-counts-v3.tsv`）の件数と、呼び出し元の関数の数（`import-callers-v3.tsv`。直接 call、IAT への call、tail jmp を数え、レジスタ経由の間接 call は数えていない。推測を含む）。**46 個**が閉包で使われる担当 API（実測。`lift/out-vfs/scope.tsv`）。jmp 件数の合計は 56、うち実装した 42 個が 52 件・呼び出し元 159（のべ）。

| API | DLL | jmp 件数 | 呼び出し元 | 実装 |
|---|---|---|---|---|
| CreateFileW | kernel32 | 1 | 5 | ○ |
| ReadFile | kernel32 | 1 | 1 | ○ |
| WriteFile | kernel32 | 2 | 2 | ○ |
| CloseHandle | kernel32 | 1 | 13 | ○ |
| SetFilePointer | kernel32 | 1 | 1 | ○ |
| GetFileSize | kernel32 | 1 | 1 | ○ |
| SetFileTime | kernel32 | 1 | 1 | ○ |
| FindFirstFileW | kernel32 | 2 | 6 | ○ |
| FindNextFileW | kernel32 | 1 | 3 | ○ |
| FindClose | kernel32 | 2 | 6 | ○ |
| GetFileAttributesW | kernel32 | 1 | 6 | ○ |
| GetFileAttributesExW | kernel32 | 1 | 4 | ○ |
| SetFileAttributesW | kernel32 | 1 | 3 | ○ |
| CreateDirectoryW | kernel32 | 1 | 1 | ○ |
| RemoveDirectoryW | kernel32 | 1 | 2 | ○ |
| DeleteFileW | kernel32 | 1 | 2 | ○ |
| CopyFileW | kernel32 | 1 | 6 | ○ |
| MoveFileW | kernel32 | 1 | 1 | ○ |
| GetFullPathNameW | kernel32 | 1 | 1 | ○ |
| GetCurrentDirectoryW | kernel32 | 1 | 1 | ○ |
| SetCurrentDirectoryW | kernel32 | 3 | 3 | ○ |
| GetTempPathW | kernel32 | 1 | 2 | ○ |
| GetModuleFileNameW | kernel32 | 2 | 12 | ○ |
| GetWindowsDirectoryW | kernel32 | 1 | 1 | ○ |
| GetSystemDirectoryW | kernel32 | 1 | 1 | ○ |
| RegOpenKeyExW | advapi32 | 2 | 6 | ○ |
| RegOpenKeyExA | advapi32 | 1 | 7 | ○ |
| RegQueryValueExW | advapi32 | 2 | 12 | ○ |
| RegCloseKey | advapi32 | 2 | 28 | ○ |
| RegCreateKeyExW | advapi32 | 1 | 2 | ○ |
| RegSetValueExW | advapi32 | 1 | 1 | ○ |
| RegEnumKeyExW | advapi32 | 1 | 3 | ○ |
| RegEnumValueW | advapi32 | 1 | 2 | ○ |
| RegDeleteKeyW | advapi32 | 1 | 1 | ○ |
| RegDeleteValueW | advapi32 | 1 | 1 | ○ |
| RegQueryInfoKeyW | advapi32 | 1 | 1 | ○ |
| RegFlushKey | advapi32 | 1 | 1 | ○ |
| GetEnvironmentVariableW | kernel32 | 2 | 2 | ○ |
| SetEnvironmentVariableW | kernel32 | 1 | 1 | ○ |
| ExpandEnvironmentStringsW | kernel32 | 1 | 2 | ○ |
| GetCommandLineW | kernel32 | 1 | 3 | ○ |
| GetStartupInfoW | kernel32 | 1 | 1 | ○ |
| CreateFileMappingW | kernel32 | 1 | 2 | ×（メモリマップ） |
| MapViewOfFile | kernel32 | 1 | 3 | × |
| UnmapViewOfFile | kernel32 | 1 | 9 | × |
| SetDllDirectoryW | kernel32 | 1 | 1 | ×（担当の例にない） |

担当リストにあるが閉包に出ないもの（`scope.tsv` で 0 件）: なし（依頼の例示はすべて閉包にある）。閉包に出るが担当外とした近縁: `SHGetFolderPathW`、`GetFileVersionInfo*`（version.dll）、`PathCleanupSpec`（shell）、`InternetReadFile`（wininet）、`FileTimeTo*` 系（時間系の担当）。

## 2. 仕様の出典と確認状況

出典はすべて `https://learn.microsoft.com/en-us/windows/win32/` 以下。「確認」はページ本文を取得して読んだこと。「確認できない」はページに書かれていないことで、コードのコメントに「inference」（推測）と書いた。

| API | ページ | 確認した主な点 | ページに書かれていない（推測または abort） |
|---|---|---|---|
| CreateFileW | `api/fileapi/nf-fileapi-createfilew` | 5 つの disposition と last error（CREATE_ALWAYS／OPEN_ALWAYS が既存で 183、CREATE_NEW 既存で 80、OPEN_EXISTING と TRUNCATE 欠落で 2）、`/` と `\`、共有モードの衝突は 32、DELETE_ON_CLOSE、ディレクトリは BACKUP_SEMANTICS が要る、CREATE_ALWAYS で HIDDEN/SYSTEM は 5 | READONLY／削除待ちのときの 5（削除待ちは DeleteFile のページ）、ディレクトリを BACKUP_SEMANTICS なしで開くときの 5、非 NULL の security attributes、テンプレート、overlapped などは abort |
| ReadFile | `api/fileapi/nf-fileapi-readfile` | 読み取り権が要る、書き込みの前にバイト数を 0 にする、末尾以降は ERROR_HANDLE_EOF（38）、末尾をまたぐ読みは成功 | 権限なし・無効ハンドルの番号、0 バイト読み、NULL のカウンタ（Windows 7 以外の仕様）、非同期は abort |
| WriteFile | `api/fileapi/nf-fileapi-writefile` | 書き込み権、0 バイトは「null write」でファイルを変えない、カウンタを先に 0 | 同上 |
| CloseHandle | `api/handleapi/nf-handleapi-closehandle` | 成功は非 0、FindFirstFile のハンドルは FindClose で閉じる（誤用は例外を投げることがある） | 無効ハンドルの番号 6（推測） |
| SetFilePointer | `api/fileapi/nf-fileapi-setfilepointer` | 64 ビット距離、負の結果は ERROR_NEGATIVE_SEEK（131）で位置不変、末尾を超えてよい、書き込みで 0 埋め、`INVALID_SET_FILE_POINTER` の曖昧さ | 32 ビット溢れの番号（87、推測）、0xFFFFFFFF ちょうどは abort |
| GetFileSize | `api/fileapi/nf-fileapi-getfilesize` | 上位ワード、成功時は NO_ERROR | 無効ハンドルの番号 |
| SetFileTime | `api/fileapi/nf-fileapi-setfiletime` | NULL または 0/0 は変更なし、FILE_WRITE_ATTRIBUTES が要る | 0xFFFFFFFF は abort、権限不足の番号（5、推測） |
| FindFirstFileW | `api/fileapi/nf-fileapi-findfirstfilew`、`api/minwinbase/ns-minwinbase-win32_find_dataw` | ワイルドカード、ディレクトリ名そのものは自身の情報、末尾バックスラッシュは常に失敗、該当なしは INVALID_HANDLE_VALUE と ERROR_FILE_NOT_FOUND、順序は未定義、短い名前は存在しないことがある、構造体のメンバー | `.` と `..` の返却（Windows の既知の挙動で、読んだページには書かれていない）、`*.*` を `*` として扱うこと（推測）、末尾バックスラッシュの番号 123（推測）、ルートのディレクトリ属性 |
| FindNextFileW | `api/fileapi/nf-fileapi-findnextfilew` | 尽きたら ERROR_NO_MORE_FILES（18） | 無効ハンドルの番号 |
| FindClose | `api/fileapi/nf-fileapi-findclose` | 閉じたハンドルは以降使えない | 二重 close の番号 |
| GetFileAttributesW / ExW | `api/fileapi/nf-fileapi-getfileattributesw`、`...exw`、`api/fileapi/ns-fileapi-win32_file_attribute_data`、`fileio/file-attribute-constants` | 失敗は INVALID_FILE_ATTRIBUTES、構造体、属性値、GetFileExInfoStandard | 欠落時の 2 と 3 の使い分け（推測） |
| SetFileAttributesW | `api/fileapi/nf-fileapi-setfileattributesw` | 設定できる値の一覧、NORMAL は単独のみ、ディレクトリに READONLY は効かない | 欠落時の番号 |
| CreateDirectoryW | `api/fileapi/nf-fileapi-createdirectoryw` | 既存は 183、中間の欠落は 3 | — |
| RemoveDirectoryW | `api/fileapi/nf-fileapi-removedirectoryw` | 空のディレクトリのみ | 空でないときの番号 145、カレントの番号 32、ファイルに対する番号（推測） |
| DeleteFileW | `api/fileapi/nf-fileapi-deletefilew` | 欠落は 2、READONLY は 5、他のハンドルが共有削除を許さなければ失敗、閉じるまで削除待ち、以降の CreateFile は 5 | ディレクトリに対する番号（推測） |
| CopyFileW | `api/winbase/nf-winbase-copyfilew` | bFailIfExists、属性をコピー、宛先が HIDDEN/READONLY なら 5、欠落は 2 | bFailIfExists の失敗番号 80（推測）、共有モード（推測）、ディレクトリは abort |
| MoveFileW | `api/winbase/nf-winbase-movefilew` | 新しい名前は存在してはならない、ディレクトリも動かせる（同じボリューム） | 宛先存在の番号 183、開いているファイルの番号 32（推測） |
| GetFullPathNameW | `api/fileapi/nf-fileapi-getfullpathnamew` | カレントを結合、存在を確かめない、長さ、lpFilePart、小さいバッファは必要サイズ | 小さいバッファの lpFilePart（仕様不明、書かない） |
| GetCurrentDirectoryW | `api/winbase/nf-winbase-getcurrentdirectory`（processenv 版のページは取得できなかった） | 長さ、小さいバッファは必要サイズ（NUL を含む）、NULL と 0 でサイズ問い合わせ | 初期ディレクトリ（`C:\`、決定 0018） |
| SetCurrentDirectoryW | `api/winbase/nf-winbase-setcurrentdirectory` | 相対も可、完全パスを計算して保存、プロセス全体で 1 つ、使用中は削除できない | 欠落・ファイルのときの番号（推測） |
| GetTempPathW | `api/fileapi/nf-fileapi-gettemppathw` | TMP、TEMP、USERPROFILE、Windows ディレクトリの順、末尾 `\`、存在確認なし、最大 261 | 仮想環境が空なので初期値が Windows ディレクトリ（決定 0018） |
| GetModuleFileNameW | `api/libloaderapi/nf-libloaderapi-getmodulefilenamew` | NULL は実行ファイルのパス、足りなければ切り詰め・nSize を返し ERROR_INSUFFICIENT_BUFFER、nSize 0 は 0 で last error 不変 | パスの値は仕様ではない（PM の仮の値）。他のモジュールのハンドルは abort |
| GetWindowsDirectoryW / GetSystemDirectoryW | `api/sysinfoapi/nf-sysinfoapi-getwindowsdirectoryw`、`...getsystemdirectoryw` | 末尾に `\` なし、長さ、不足時の必要サイズ | 値は仮（決定 0018）。Windows 側のページは不足時の NUL の数え方が曖昧（本実装は NUL を含める） |
| RegOpenKeyExW / A | `api/winreg/nf-winreg-regopenkeyexw`、`...exa` | キー名は大文字小文字を区別しない、NULL または空の subkey と定義済みキーは同じハンドル、欠落は 2、ulOptions は 0 か REG_OPTION_OPEN_LINK | 無効ハンドルの番号、ANSI の非 ASCII（abort） |
| RegQueryValueExW | `api/winreg/nf-winreg-regqueryvalueexw` | 名前なしの既定値、lpData が NULL ならサイズだけ、小さければ ERROR_MORE_DATA と必要サイズ、欠落は 2、文字列は終端が保証されない | アクセス権不足の番号 5（推測） |
| RegCloseKey | `api/winreg/nf-winreg-regclosekey` | 閉じたハンドルは使えない | 定義済みキーを閉じて成功（推測）、二重 close の番号 |
| RegCreateKeyExW | `api/winreg/nf-winreg-regcreatekeyexw` | 欠落をすべて作る（32 階層まで）、disposition 1 と 2、空 subkey は hKey の新しいハンドル、lpClass は無視してよい、オプション | HKLM／HKU 直下の作成禁止（番号なし、abort）、セキュリティ属性（abort） |
| RegSetValueExW | `api/winreg/nf-winreg-regsetvalueexw` | 名前なしの値、KEY_SET_VALUE、文字列は終端を含む cbData、MULTI_SZ は NUL 2 つ | DWORD と QWORD の cbData の不一致（abort）、型番号（`registry-value-types` の名前のみ。数値は winnt.h の値で未確認） |
| RegEnumKeyExW / RegEnumValueW | `api/winreg/nf-winreg-regenumkeyexw`、`...enumvaluew` | 0 から増やす、尽きたら ERROR_NO_MORE_ITEMS（259）、小さければ ERROR_MORE_DATA | ERROR_MORE_DATA のときの lpcchName の値（書かれていない。変更しない） |
| RegQueryInfoKeyW | `api/winreg/nf-winreg-regqueryinfokeyw` | 各カウントの意味と単位、lpClass だけで lpcchClass なしは 87 | セキュリティ記述子（abort） |
| RegDeleteKeyW | `api/winreg/nf-winreg-regdeletekeyw` | 子を持つキーは削除できない、NULL 不可、大文字小文字を区別しない、最後のハンドルが閉じるまで残る | 子ありの番号（5、推測）、開いているハンドルがある場合（abort） |
| RegDeleteValueW | `api/winreg/nf-winreg-regdeletevaluew` | 名前なしの値、KEY_SET_VALUE | 欠落の番号（2、推測） |
| RegFlushKey | `api/winreg/nf-winreg-regflushkey` | 何も書かない環境では成功 | — |
| 定義済みキー、型、アクセス権 | `sysinfo/predefined-keys`、`sysinfo/registry-value-types`、`sysinfo/registry-key-security-and-access-rights`、`sysinfo/registry-element-size-limits` | 名前、KEY_* のアクセス権の値、サイズ上限（キー名 255、値名 16,383、値 1 MiB、深さ 512、1 回 32 階層） | **定義済みキーの数値、REG_* の数値はページにない**（`fle_reg.h` に winnt.h の値から入れ、未確認と書いた） |
| GetEnvironmentVariableW | `api/processenv/nf-processenv-getenvironmentvariablew` | 欠落は 0 と ERROR_ENVVAR_NOT_FOUND（203）、不足時は NUL を含む必要サイズ、最大 32,767 | 値が空の変数は 0（推測）、名前の大文字小文字の扱い、NULL の名前（abort） |
| SetEnvironmentVariableW | `api/winbase/nf-winbase-setenvironmentvariable` | 作成・置換、NULL の値で削除 | 存在しない変数の削除（abort）、名前に `=`（`procthread/environment-variables` は `=` を含められないと書く。番号は無いので abort） |
| ExpandEnvironmentStringsW | `api/processenv/nf-processenv-expandenvironmentstringsw` | `%名前%` を展開、大文字小文字は無視、見つからなければそのまま、戻り値は NUL を含む、不足時は必要サイズ、lpDst と lpSrc は同じにできない | 閉じない `%`、空 `%%`（abort） |
| GetCommandLineW | `api/processenv/nf-processenv-getcommandlinew` | システムが管理するポインタ、変更不可 | 値は決定 0018 |
| GetStartupInfoW | `api/processthreadsapi/nf-processthreadsapi-getstartupinfow`、`...ns-processthreadsapi-startupinfow` | 作成元が指定した構造体を返す、メンバー一覧（x64 での大きさは 0x68） | 作成元がないのですべて 0（決定 0018） |
| エラー番号 | `debug/system-error-codes--0-499-` | 2、3、5、6、18、32、38、80、87、122、123、131、145、183、203、234、259、267 の名前と値 | — |

取得できなかったページ（404）: `fileapi/nf-fileapi-getcurrentdirectory`、`.../setcurrentdirectory`（winbase 側で取得）、`processenv/nf-processenv-setenvironmentvariable`（winbase 側で取得）、`processthreadsapi/...getstartupinfow`（別 URL で取得）、`minwinbase/ns-minwinbase-win32_file_attribute_data`（fileapi 側で取得）。これらは別 URL に移っていて、取得できたページの URL を表に書いた。

## 3. 実装した API と、しなかった API

- 実装した（42 個）: 表の「○」。ランタイムは `fle_vfs.c`（25 個）、`fle_reg.c`（12 個）、`fle_env.c`（5 個）。`imports_table.rs` に 42 行を追加した。
- 実装しなかった（4 個）:
  - CreateFileMappingW、MapViewOfFile、UnmapViewOfFile: ゲストにファイルの内容を実メモリとして見せる設計（ファイルの `data` をゲストのアドレス空間に置く、書き戻す、共有モードとの整合）が要る。仮想ファイルが `realloc` で動く現在の作りと合わない。呼び出し元は計 14 関数（のべ）。使われ方（読み取り専用か、どのファイルか）は未調査。
  - SetDllDirectoryW: 担当の例示になく、DLL 検索（LoadLibrary）の設計に属する。
- 実装済みでも、ページが番号を定めない失敗は推測（コードのコメントに「inference」と書いた。決定 0018 の §5）。
- 仕様が確認できずに abort にするもの: テスト `undefined_input_stops_the_run`（vfs 38 件、reg 34 件、env 16 件のケース）で、すべて SIGABRT と `fle:` のメッセージを確認した。
