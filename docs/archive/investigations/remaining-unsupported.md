# 未対応 615 件の全件集計と次の波の提案

日付: 2026-10-20。読み取りと集計のみ。コード、runtime、既存の生成物は変更していない。
集計ツールと出力はすべて `lift/out-remaining/`（新規）にある。

## 0. 入力と再現

| 入力 | 内容 |
|---|---|
| `lift/out-x87ctl/out/after-x87ctl-896.c` | 生成 C（896 seed、x87 制御対応後） |
| `lift/out-x87ctl/gen-x87ctl-896.txt` | 生成器の報告 |
| `lift/out-x87ctl/run-x87ctl-896.txt` | 実行ログ（停止点 0x407040、関数エントリ 66 件） |
| `lift/out-imports/imports.txt` | IAT スロット → dll!name（normal 738、delay 57） |
| `lift/objdump-text.txt`、`lift/out-usage/extra-objdump-0x1375000.txt` | `jmp [rip+slot]` のスロット引き当て |
| `lift/crates/fe-lift/src/imports_table.rs` | 対応表（読み取りのみ。**状態は作業中に変わった**。下の注を参照） |
| `lift/runtime/*.h`、`*.c` | `fle_imp_*` の宣言と実装 |

注（状態の時刻）: 615 件は、固定の生成 C（`out-x87ctl`、15:24 生成）の件数で、集計中に変わらない。一方、「応答がある API」は他のサブエージェントの作業で増えた。最初に読んだとき（約 15:30）の対応表は 39 行で、`fle_vfs.c`、`fle_reg.*` はなかった。最後に確認したとき（15:41 JST）は、対応表が 83 行（kernel32 70、advapi32 12、winmm 1）、`fle_vfs.c`（25 関数）、`fle_reg.c`（12 関数）、`fle_env.c`（5 関数）が `lift/runtime/` にあり、`fle_imp_*` の宣言 83 件、実装 83 件が対応表と一致した（`state-v3.txt`）。§3 の「応答の有無」は、この 15:41 の状態で数えている。生成 C は 39 行の表で作られているので、**今の C は新しい 44 行を使っていない**。再生成するまで、下の「応答あり」の thunk は C の上では未対応のままである。

再現: `cd lift/out-remaining && python3 aggregate_remaining.py && python3 aggregate_v2.py`（TMPDIR は `lift/out-remaining/tmp`）。スクリプトは既存ファイルがあると上書きせずに終了する。cargo は使っていない。

出力ファイル（すべて `lift/out-remaining/`）:

| ファイル | 内容 |
|---|---|
| `summary-counts.tsv` | 件数の突き合わせ（gen 報告、C の grep、行のパース） |
| `unsupported-all.tsv` | 615 行の全件（アドレス、ニーモニック、関数、種類、dll、API、関数が実行済みか、静的な呼び出し深さ） |
| `jmp-unresolved.tsv` | 解決できない間接 jmp 20 行 |
| `iat-by-api.tsv`、`iat-by-api-v2.tsv` | API 別（356 API）。v2 が採用した系統分類 |
| `iat-by-dll.tsv`、`iat-by-family-v2.tsv` | DLL 別、系統別 |
| `answered-but-unwired.tsv` | 宣言はあるが対応表にない API（30 件） |
| `no-answer-apis.tsv` | 応答のない API。`iat-by-api.tsv` と同じ旧系統の列で並べてある。系統は v2 を参照 |
| `insn-by-mnemonic.tsv`、`insn-detail.tsv` | 未実装命令のニーモニック別、アドレスごとの逆アセンブル |
| `executed.tsv` | 実行済み関数にある未対応 |
| `run.log`、`run-v2.log` | ツールの実行ログ |

## 1. 615 件の種類別（実測）

| 種類 | 件数 |
|---|---|
| (a) IAT 経由 jmp（`Jmp: indirect through IAT slot`） | 378 |
| (b) 解決できない間接 jmp（`Jmp: indirect target unresolved`） | 20 |
| (c) 未実装命令（`mnemonic outside minimal set` など） | 217 |
| 合計 | 615 |

(c) の内訳: 46 種のニーモニック。うち `Jecxz` 2 件（`no condition code`）と、`Lea`/`Mov` の FS/GS セグメント各 1 件を含む。

### 件数の突き合わせ

3 通りの数え方で一致した。

| 数え方 | 結果 |
|---|---|
| gen 報告の `unsupported_total` | 615 |
| gen 報告の種類別の和 | 615（Jmp IAT 378 + Jmp 解決不能 20 + 命令 217） |
| 生成 C の `fle_unsupported(UINT64_C(` の行数（grep） | 615 |
| 生成 C のラベル行のパース | 615（IAT 378、解決不能 20、命令 217） |
| 非 Jmp のニーモニック別の件数の C と gen の差 | 0 |

食い違いは、この 2 つの入力の間にはなかった。ただし、次の 2 点は注意が要る。

- アドレスの重複: 615 行のアドレスは 607 種。重複は解決不能 jmp の 3 アドレス（0x42316f が 6 関数、0x42474f が 3 関数、0x80f9ef が 2 関数で共有）。関数は 20 種で、重複していない。同じ命令バイト列を複数の関数が共有している（後述 §1.2）。
- `lift/out-usage/` の v3 との差: v3 は `unsupported_jmp_iat` 435、`unsupported_iat_slot_distinct` 390 で、今回の 378 と違う。理由は入力の C が古いこと。v3 の入力は `out-cpuid-run/out/after-cpuid-896.c` で、gen 報告の `unsupported_total` は 1617、Jmp IAT は 435 だった（`out-cpuid-run/gen-896.txt`）。その後、locale、sys などの対応表が増え、今回の gen 報告では `jmp_imp:*` が合計 67 件ある。435 − 378 = 57、67 − 10（v3 の `handled_stub_jmps_in_closure`）= 57 で、差は説明できる（算術の一致。v3 の再実行はしていない）。

### 1.1 (a) IAT 経由 jmp 378 件の DLL 別

すべて normal スロットで、delay スロットは 0 件（実測）。API は 356 種で、重複する API は 22 件分（SendMessageW と SystemParametersInfoW が 3 件など）。

| DLL | API 数 | thunk 件数 |
|---|---|---|
| user32.dll | 133 | 143 |
| kernel32.dll | 95 | 104 |
| gdi32.dll | 54 | 54 |
| advapi32.dll | 14 | 17 |
| oleaut32.dll | 11 | 11 |
| winmm.dll | 10 | 10 |
| ole32.dll | 7 | 7 |
| crypt32.dll | 7 | 7 |
| shell32.dll | 6 | 6 |
| wininet.dll | 6 | 6 |
| comctl32.dll | 4 | 4 |
| version.dll | 3 | 3 |
| dsound.dll | 2 | 2 |
| wintrust.dll、imagehlp.dll、shfolder.dll、msacm32.dll | 各 1 | 各 1 |
| 計 | 356 | 378 |

API 別の全件は `iat-by-api-v2.tsv`（356 行）。

### 1.2 (b) 解決できない間接 jmp 20 件

アドレスで 12 種、関数で 20 種（`jmp-unresolved.tsv`、逆アセンブルは `objdump-text.txt`）。

| 種類 | 行数 | 命令 | 観察（実測は命令列まで。意味づけは推測と書く） |
|---|---|---|---|
| `jmp rax`（直前が `pop r8; pop rcx; pop rdx`） | 18（10 アドレス） | 0x4071bf、0x4072ff、0x42304f、0x42316f、0x42474f、0x42572f、0x43916f、0x5dc61f、0x5e103f、0x80f9ef | 直前に `call 0x41cfc0`（`call [0x13a9398]`）がある。**推測**: delay import のヘルパー末尾（遅延ロードの解決後に解決先へ飛ぶ形） |
| `jmp [r10+rdx*8+0x40]` | 1 | 0x40d253（関数 0x40d210） | バイトを 8 個複製して詰める形（`movabs 0x0101…; imul`）の関数にある、8 バイト以下の長さ用の表引き。表は `lea r10,[rip+5]` の直後（0x40d258 から）にあり、0x40d258 から 8 バイトずつ、`.text` 内の 0x40d2xx を指す。**推測**: memset 系 |
| `jmp [rcx]` | 1 | 0x412752（関数 0x4126f0） | 直前に `mov rsp,[rcx+8]` と r12〜r15 の復元がある。**推測**: longjmp 系 |

delay import との対応（実測）: 閉包内で delay スロットへ `jmp [rip+slot]` する thunk は 18 件ある。生成 C ではどれも `goto L_xxxx;`（関数の中のラベルへの飛び）になり、`fle_unsupported` ではない。この 18 と、`jmp rax` の 18 行は数が一致する（`jmp rax` の行は、18 の thunk 関数それぞれの中にある）。18 件の API は §1.4 に書く。`jmp rax` が delay-load ヘルパーの末尾だという点は、命令列（`pop r8; pop rcx; pop rdx; jmp rax`）からの推測で、ヘルパーの中身は読み切っていない。

- 0x40d253 の関数 0x40d210 は、実行ログで実行済み（entry 40 の `f_40d210`）。この jmp 自体は実行されていない（停止はこの手前の 0x407040 だった）。長さが 8 以下の呼び出しで止まる（**推測**: 表引きの条件 `sub rdx,8; jg` が偽のとき）。
- 0x412752 の関数 0x4126f0 は、静的な呼び出し深さ 3（0x412770 から 0x4127bf で呼ばれる）。実行は未確認。
- 0x4126f0 の先頭 1 バイト（`92` = `xchg edx,eax`）は、`Xchg` の未対応 1 件として (c) にも数えられている。関数の先頭にこの命令が来る理由は未確認。

### 1.3 (c) 未実装命令 217 件（系統別。詳細は §4）

| 系統 | 件数 |
|---|---|
| SSE（xmm 整数パック 91、浮動小数点のシャッフルとロード 26） | 117 |
| MMX（mm レジスタ 37、`Emms` 3） | 40 |
| AVX/VEX | 18 |
| 整数 | 34 |
| システム | 8 |
| 合計 | 217 |

### 1.4 delay import の thunk 18 件（615 件には数えられていない）

delay スロットへの thunk は 18 件、API は 18 種（実測。`jmp-unresolved.tsv` の 20 関数のうち 18 関数が delay thunk の関数で、残る 2 関数は 0x40d210 と 0x4126f0）。生成 C ではこの 18 件は `goto` で、`fle_unsupported` の行ではない。したがって、今の C で実行がこれらの delay thunk に到達すると、delay-load ヘルパーを通って §1.2 の `jmp rax`（解決不能）で止まる、という経路になる（推測。実行して確かめていない）。

user32: MessageBoxA、GetPhysicalCursorPos、SetPhysicalCursorPos、IsProcessDPIAware。
kernel32: GetLogicalProcessorInformation、GetNativeSystemInfo、GetSystemDefaultLocaleName、GetUserPreferredUILanguages、LocaleNameToLCID、GetFinalPathNameByHandleW、NormalizeString。
advapi32: RegDeleteKeyExW。wtsapi32: WTSRegisterSessionNotification。rpcrt4: UuidCreate。uxtheme: BufferedPaintInit。DWMAPI: DwmIsCompositionEnabled。Shcore: GetProcessDpiAwareness、GetDpiForMonitor。

delay import は 57 スロットあり、閉包で thunk になっているのは上の 18 件（実測）。delay の扱い方（ヘルパーを実行するか、API ごとに応答するか）は、設計の判断が要る（§5 の D5）。

## 2. IAT 経由 jmp 378 件の系統分類（実測。分類の境界は私の判断）

分類は `aggregate_v2.py` の明示的な表（API 名 → 系統）で、結果は `iat-by-api-v2.tsv`（356 行）。「応答あり」は §0 の注の 15:41 の状態（対応表にある API）。

| 系統 | API 数 | thunk 件数 | うち応答あり（API / thunk） |
|---|---|---|---|
| user32: ウィンドウ・メッセージ・入力・メニュー・描画 | 108 | 115 | 0 / 0 |
| gdi32: GDI | 54 | 54 | 0 / 0 |
| comctl32: フラットスクロールバー | 4 | 4 | 0 / 0 |
| user32: システム設定・テーブル・リソース | 10 | 12 | 0 / 0 |
| user32: ディスプレイ・モニタ | 6 | 6 | 0 / 0 |
| user32: 文字列（窓なし） | 5 | 6 | 0 / 0 |
| user32: RECT 計算（窓なし） | 4 | 4 | 0 / 0 |
| **user32 の小計** | **133** | **143** | 0 / 0 |
| advapi32: レジストリ | 12 | 15 | 12 / 15 |
| advapi32: トークン | 2 | 2 | 0 / 0 |
| oleaut32: BSTR・VARIANT・SAFEARRAY | 11 | 11 | 0 / 0 |
| ole32: COM | 7 | 7 | 0 / 0 |
| シェル（shell32、shfolder） | 7 | 7 | 0 / 0 |
| version | 3 | 3 | 0 / 0 |
| winmm: MIDI | 7 | 7 | 0 / 0 |
| winmm: タイマ | 3 | 3 | 0 / 0 |
| kernel32: ファイル・パス・I/O | 32 | 38 | 25 / 31 |
| kernel32: ファイルマッピング | 3 | 3 | 0 / 0 |
| kernel32: プロセス・スレッド・同期・例外 | 20 | 20 | 0 / 0 |
| kernel32: 環境・コマンドライン | 5 | 6 | 5 / 6 |
| kernel32: ライブラリロード・リソース | 9 | 11 | 0 / 0 |
| kernel32: メモリ | 6 | 6 | 0 / 0 |
| kernel32: アトム | 5 | 5 | 0 / 0 |
| kernel32: 時刻・日付 | 9 | 9 | 0 / 0 |
| kernel32: ロケール・文字列・メッセージ | 6 | 6 | 0 / 0 |
| ネットワーク（wininet） | 6 | 6 | 0 / 0 |
| 暗号・信頼（crypt32、wintrust） | 8 | 8 | 0 / 0 |
| オーディオ（dsound、msacm32） | 3 | 3 | 0 / 0 |
| imagehlp | 1 | 1 | 0 / 0 |
| **合計** | **356** | **378** | **42 / 52** |

kernel32 の小計は 95 API・104 thunk（上の kernel32 の 8 行の和: 32+3+20+5+9+6+5+9+6 = 95 API、38+3+20+6+11+6+5+9+6 = 104 thunk）。DLL 別の表（§1.1）と一致する。`SHGetFolderPathW` は shell32 と shfolder に別スロットがあり、2 API として数えている。

利用者が挙げた系統との対応: ウィンドウ・メッセージ（user32）143 thunk、GDI 54、コモンコントロール 4、レジストリ 15、COM/OLE 18、シェル 7、バージョン 3、マルチメディア（winmm）10、ファイル・パス（32+3 API）41 thunk、プロセス・スレッド 20（環境 6 を含めると 26）、ライブラリロード 11、文字列・ロケール・時刻など（kernel32 のロケール 6、時刻 9、アトム 5、メモリ 6）26、その他（ネットワーク 6、暗号 8、オーディオ 3、imagehlp 1、advapi32 トークン 2）20。

実行経路上の位置（thunk）: この閉包で、実行済みの 44 関数にある未対応の IAT thunk は 0x407040（GetCommandLineW、停止点そのもの）の 1 件だけ（実測、`executed.tsv`）。生成 C の直接の呼び出し・tail jmp だけをたどった静的な深さ（`aggregate_remaining.py` の BFS。間接 dispatch はたどらない。到達 41 関数）が分かる thunk は 7 件で、他の 371 件は「分からない」。

| 深さ | VA | API |
|---|---|---|
| 2 | 0x406fc0 | kernel32!ExitProcess |
| 2 | 0x407020 | kernel32!FreeLibrary |
| 3 | 0x406e70 | kernel32!GetStdHandle |
| 3 | 0x406ee0 | kernel32!WriteFile（応答あり） |
| 6 | 0x41cf10 | kernel32!FreeLibrary |
| 6 | 0x41cf90 | kernel32!LoadLibraryA |
| 6 | 0x41cfa0 | kernel32!RaiseException |

この 7 件の位置は実行ログには出ていない（実行はまだ 0x407040 で止まる）。

## 3. 応答のない系統の一覧（15:41 の状態。実測）

15:41 の対応表（83 行）と `fle_imp_*` の実装（83 件、一致）を、未対応の IAT thunk に当てた結果は `state-v3.txt`、`answers-v3.tsv`、`family-v3.tsv`、`no-answer-v3.tsv`。

- 応答あり: 42 API・52 thunk（レジストリ 12/15、ファイル・パス 25/31、環境 5/6）。
- 応答なし: 314 API・326 thunk。
- 「宣言だけで対応表にない」API は 0 件（宣言 83 = 実装 83 = 表 83）。
- この 52 thunk は、今の生成 C では未対応のままで、生成器の再実行で `fle_unsupported` が減るはず。期待値は 615 − 52 = 563（算術。再生成は未実施で、未確認）。

応答のない系統（thunk の多い順。API 名は `no-answer-v3.tsv`）:

| 系統 | API 数 | thunk 件数 | API 名（小さい系統のみ） |
|---|---|---|---|
| user32: ウィンドウ・メッセージ・入力・メニュー・描画 | 108 | 115 | （一覧は `iat-by-api-v2.tsv`） |
| gdi32: GDI | 54 | 54 | （同上） |
| kernel32: プロセス・スレッド・同期・例外 | 20 | 20 | CreateEventW、CreateThread、ExitProcess、GetCurrentProcess、GetCurrentProcessId、GetCurrentThread、GetExitCodeThread、OutputDebugStringW、RaiseException、ResetEvent、ResumeThread、SetErrorMode、SetEvent、SetLastError、SetPriorityClass、SetThreadErrorMode、SetThreadPriority、SwitchToThread、TerminateProcess、WaitForSingleObject |
| user32: システム設定・テーブル・リソース | 10 | 12 | GetClipboardFormatNameW、GetKeyNameTextW、GetKeyboardLayout、GetSysColor、GetSystemMetrics、LoadStringW、MapVirtualKeyW、RegisterClipboardFormatW、RegisterWindowMessageW、SystemParametersInfoW（3 件） |
| kernel32: ライブラリロード・リソース | 9 | 11 | FindResourceW、FreeLibrary（3 件）、LoadLibraryA、LoadLibraryExW、LoadLibraryW、LoadResource、LockResource、SetDllDirectoryW、SizeofResource |
| oleaut32 | 11 | 11 | SafeArrayCreate、SafeArrayGetLBound、SafeArrayGetUBound、SafeArrayPtrOfIndex、SysAllocStringLen、SysFreeString、SysReAllocStringLen、VariantClear、VariantCopy、VariantCopyInd、VariantInit |
| kernel32: 時刻・日付 | 9 | 9 | DosDateTimeToFileTime、FileTimeToDosDateTime、FileTimeToLocalFileTime、FileTimeToSystemTime、GetDateFormatW、GetLocalTime、GetTimeZoneInformation、LocalFileTimeToFileTime、SystemTimeToTzSpecificLocalTime |
| 暗号・信頼（crypt32、wintrust） | 8 | 8 | CertCloseStore、CertFindCertificateInStore、CertFreeCertificateContext、CertGetNameStringW、CryptMsgClose、CryptMsgGetParam、CryptQueryObject、WinVerifyTrust |
| kernel32: ファイル・パス・I/O の残り | 7 | 7 | DuplicateHandle、GetDriveTypeW、GetLogicalDriveStringsW、GetLogicalDrives、GetStdHandle、GetVolumeInformationW、QueryDosDeviceW |
| ole32 | 7 | 7 | CLSIDFromString、CoCreateInstance、CoInitialize、CoTaskMemFree、IsEqualGUID、OleInitialize、StringFromCLSID |
| シェル | 7 | 7 | PathCleanupSpec、SHAppBarMessage、SHGetFolderPathW（shell32、shfolder の 2 スロット）、SHGetMalloc、ShellExecuteExW、ShellExecuteW |
| winmm: MIDI | 7 | 7 | midiInGetDevCapsW、midiInGetNumDevs、midiOutGetDevCapsW、midiOutGetErrorTextW、midiOutGetNumDevs、midiOutOpen、midiOutShortMsg |
| kernel32: ロケール・文字列・メッセージ | 6 | 6 | EnumCalendarInfoW、EnumSystemLocalesW、FormatMessageW、GetComputerNameExW、MulDiv、lstrlenW |
| kernel32: メモリ | 6 | 6 | GlobalAlloc、GlobalFree、GlobalLock、GlobalUnlock、VirtualProtect、VirtualQuery |
| user32: 文字列（窓なし） | 5 | 6 | CharLowerBuffW、CharLowerW、CharNextW（2 件）、CharUpperBuffW、CharUpperW |
| user32: ディスプレイ・モニタ | 6 | 6 | EnumDisplayMonitors、EnumDisplaySettingsW、GetMonitorInfoW、MonitorFromPoint、MonitorFromRect、MonitorFromWindow |
| ネットワーク（wininet） | 6 | 6 | InternetCloseHandle、InternetGetConnectedState、InternetGetLastResponseInfoW、InternetOpenUrlW、InternetOpenW、InternetReadFile |
| kernel32: アトム | 5 | 5 | AddAtomW、DeleteAtom、FindAtomW、GlobalAddAtomW、GlobalFindAtomW |
| comctl32: フラットスクロールバー | 4 | 4 | FlatSB_GetScrollPos、FlatSB_SetScrollInfo、FlatSB_SetScrollPos、FlatSB_SetScrollProp |
| user32: RECT 計算（窓なし） | 4 | 4 | IntersectRect、OffsetRect、SetRect、UnionRect |
| オーディオ（dsound、msacm32） | 3 | 3 | DirectSoundCreate、DirectSoundEnumerateA、acmMetrics |
| kernel32: ファイルマッピング | 3 | 3 | CreateFileMappingW、MapViewOfFile、UnmapViewOfFile |
| version | 3 | 3 | GetFileVersionInfoSizeW、GetFileVersionInfoW、VerQueryValueW |
| winmm: タイマ | 3 | 3 | timeBeginPeriod、timeEndPeriod、timeGetDevCaps |
| advapi32: トークン | 2 | 2 | GetTokenInformation、OpenProcessToken |
| imagehlp | 1 | 1 | ImageNtHeader |
| **応答なしの合計** | **314** | **326** | |

（応答なしの系統は、レジストリ、環境・コマンドライン以外のすべて。ファイル・パスは 7 API が残る。）

`docs/decisions/0018-virtual-fs-reg-env.md` は、`fle_env.h`、`fle_vfs.h` が出典として挙げているが、15:41 の時点で `docs/decisions/` にまだなかった（`ls` で 0 件。他の担当が書く途中と推測）。

## 4. (c) 未実装命令 217 件（系統・ニーモニック・件数・位置）

形（mm / xmm / ymm）は `insn-detail.tsv` の逆アセンブルから判定した（`insn_table.py`、結果 `insn-forms.tsv`、`insn-forms.md`）。全 217 件の命令バイト列と逆アセンブルは `insn-detail.tsv`。

### 実行経路上の位置（実測と、分からないこと）

- 実測: 217 件はどれも、実行済みの 44 関数の中にない（`executed.tsv`）。したがって「既知の停止点（0x407040）より前に実行された命令」には、未実装命令は 0 件。
- 実測: 直接の呼び出しだけをたどった静的な深さが分かるのは 3 件。`Xchg` 0x4126f0（深さ 3）、`Mov`（GS）0x41d14b（深さ 3）、`Xchg` 0x41dbe1（深さ 5）。それ以外の 214 件は、深さが分からない（直接の呼び出しで到達しない。間接 dispatch はたどっていない）。
- 実行の「停止点より後」にあるかどうかは、実行がまだ 0x407040 で止まっているので、**分からない**。以下の「位置」欄に書くのは、命令列から言える範囲だけ。

### SSE（xmm）117 件

| ニーモニック | 件数 | 関数数 | 位置 |
|---|---|---|---|
| Pshuflw | 16 | 13 | 分からない |
| Shufps | 13 | 10 | 分からない |
| Psllw | 12 | 5 | 分からない |
| Packssdw | 10 | 5 | 分からない |
| Movlps | 8 | 4 | 分からない |
| Psrlw | 8 | 2 | 分からない |
| Paddusb | 6 | 2 | 分からない |
| Pmaxsw | 6 | 1 | 分からない |
| Pminsw | 6 | 1 | 分からない |
| Pmulhw | 6 | 4 | 分からない |
| Pmullw | 6 | 2 | 分からない |
| Psubsw | 4 | 3 | 分からない |
| Movhlps | 3 | 2 | 分からない |
| Psubusb | 3 | 1 | 分からない |
| Movlhps | 2 | 1 | 分からない |
| Pshufhw | 2 | 2 | 分からない |
| Psrad | 2 | 2 | 分からない |
| Paddq、Paddsw、Psraw、Psrld | 各 1 | 各 1 | 分からない |

内訳: 整数パック 91 件（17 種）、浮動小数点のシャッフル・半分移動 26 件（Shufps、Movlps、Movhlps、Movlhps）。

### MMX（mm レジスタ）40 件

| ニーモニック | 件数 | 関数数 |
|---|---|---|
| Movd | 10 | 2 |
| Punpcklbw | 5 | 2 |
| Packuswb、Paddw、Pxor | 各 4 | 各 2 |
| Emms | 3 | 3 |
| Pmulhuw | 3 | 1 |
| Movq、Psllw | 各 2 | 各 1 |
| Pmulhw、Pshufw、Psubsw | 各 1 | 各 1 |

関数は 0x660c40、0x6a7990（mm レジスタを使う）、0x63a660（`Emms` だけ）の 3 つ（実測）。位置は分からない。

### AVX/VEX 18 件

Vmovups 12、Vmovaps 2、Vmovntps 2、Vzeroupper 2（すべて関数 0x409c80 の中）。0x409c80 は、`movups` で 16〜32 バイトをコピーする分岐を持つ関数（メモリコピー系。**推測**）。

- 実測: 0x409c80 は、直接の呼び出し元が `objdump-text.txt` で 12 件以上ある（0x4079e2、0x407a2c、0x4080a2 など）。実行ログにはまだ出ていない。
- 実測: AVX 版へ入る条件は `test DWORD PTR [0x15e60b0],1`（0x409d1d、0x409e1a）。この値は 0x409c0f で、`0x15e69c0 & 0x10000000` から作られる（関数 0x409bf0。実行済み）。
- 推測: `0x10000000` は CPUID 葉 1 の ECX の bit 28（AVX）に当たる（SDM の定義）。`fle_cpuid.c` は葉 1 の ECX を 0 で返す（読んで確認）。ただし 0x15e69c0 に何が書かれるかは未確認なので、AVX 版が実行されないとは断定できない。

### 整数 34 件

| ニーモニック | 件数 | 関数数 | 備考（実測の命令列） |
|---|---|---|---|
| Xchg | 11 | 10 | `lock xchg`（メモリ）9 件、`xchg [r13+r12],rax`（暗黙の LOCK）1 件、`xchg edx,eax`（0x4126f0 の先頭 1 バイト）1 件 |
| Bsf | 5 | 1 | 関数 0x407a50 |
| Bts | 5 | 4 | `bts [mem],reg`（レジスタのビット位置） |
| Rol | 4 | 2 | `rol eax,cl` など |
| Ror | 3 | 1 | `ror eax,0x10` |
| Btr | 2 | 1 | `btr [mem],reg` |
| Jecxz | 2 | 2 | 0x6a1ffb、0x6a2292。生成器は `no condition code (loop or jrcxz)` で未対応にしている |
| Lea | 1 | 1 | 0x409dda `lea rax,fs:[rax+riz*1+0]`（FS 接頭辞つき。実質 nop 形） |
| Mov | 1 | 1 | 0x41d14b `mov rax,gs:0x58`（関数 0x41d110） |

位置: `Xchg` 0x4126f0 と 0x41dbe1、`Mov`（GS）0x41d14b は、深さ 3〜5（実測）。他は分からない。

0x41d110 の補足（実測と推測）: 0x41d114 の `cmp BYTE [0x15ec2d8],0` が 0 なら 0x41d14b（GS 経由）へ進む。0x15ec2d8 には、実行済みの関数 0x41d8f0 の中の 0x41d912 が 1 を書く（`mov BYTE [rip+..],1`）。この命令が実行されたかは未確認。実行されたなら、GS 経由の分岐は通らない、というのが推測。

### システム 8 件

| ニーモニック | 件数 | 関数数 | 備考 |
|---|---|---|---|
| Pause | 3 | 3 | 0x407b39、0x407ec4、0x4104e4（スピン待ちの位置。**推測**） |
| Cmpxchg16b | 2 | 2 | 0x41baa1、0x4a5bb7（どちらも `lock cmpxchg16b`） |
| Sfence | 2 | 1 | 0x409e10、0x409ee0（AVX 版コピーの末尾） |
| Xgetbv | 1 | 1 | 0x639a03。直前に `and ecx,0x18000000; cmp ecx,0x18000000; jne` がある。`fle_cpuid.c` の葉 1 の ECX は 0 なので、この `jne` で飛び越える（命令列と `fle_cpuid.c` から確定。実行はしていない） |

### 系統ごとの件数（再掲）

SSE 117、MMX 40、AVX/VEX 18、整数 34、システム 8。計 217。

## 5. 次の並列の波の提案

前提と制約:

- 書き込みは新規ファイルのみ。`imports_table.rs` へは行の追加のみ。
- 例外 1（lib.rs）: 命令の単位は、`lift/crates/fe-lift/src/lib.rs` に `mod` 1 行と `if let` 4 行を足す（既存の 12 モジュール（cpuid、x87ctl など）と同じ形）。これは「新規ファイルのみ」の例外で、複数の命令の単位を同時に起動すると、lib.rs で衝突する。集約者が最後に 1 回で入れるか、先に 1 単位が登録口を作ってから始める必要がある。
- 例外 2（imports_table.rs）: TABLE は 1 つの定数なので、行の追加は 1 か所に集中する。複数の単位が同時に同じ場所へ追記すると衝突する。各単位は行を自分の新規ファイル（例: `docs/` の提案メモ）に書いて出し、集約者が 1 回で追記するのが安全。
- 生成 C のビルド（`build.sh` の `EXTRA_RT`）は、出力ディレクトリごとにある。単位ごとに自分の出力ディレクトリを使う。
- 仕様確認の難しさの区分: 「公式で決まる」＝ Microsoft Learn の API ページ（API）、Intel SDM（命令）で値が決まる。「判断が必要」＝ 仕様が決めない値の選択がある。命令の仕様は、これまでの波では felixcloutier の転載（非公式）を読み、公式 PDF は未取得だった（`x87ctl-run.md` §8）。

### 5.1 API の単位（runtime は新規 `fle_<名前>.c/.h`、行の追加は `imports_table.rs`）

| ID | 対象 | API 数 / thunk | 触るファイル | 仕様確認 |
|---|---|---|---|---|
| A1 | kernel32 プロセス・スレッドの単純なもの: GetCurrentProcess、GetCurrentProcessId、GetCurrentThread、SetLastError、SetErrorMode、SetThreadErrorMode、SetPriorityClass、SetThreadPriority、SwitchToThread、OutputDebugStringW、ExitProcess、TerminateProcess、RaiseException | 13 / 13 | 新規 `fle_proc.c/.h`、表に 13 行 | 大半は公式で決まる（擬似ハンドル -1、-2 など）。ExitProcess、TerminateProcess、RaiseException は、ホストを止めるか、`fle_stop` で止めるかの判断が要る。ExitProcess（深さ 2）は CRT の終了経路にある可能性（推測） |
| A2 | kernel32 リソース: FindResourceW、LoadResource、LockResource、SizeofResource（+ user32 LoadStringW） | 5 / 5 | 新規 `fle_res.c/.h`、表に 5 行 | 公式で決まる（PE のリソースディレクトリ形式は公式の PE 仕様、LoadStringW のテーブル形式は Microsoft Learn と PE 仕様）。イメージはメモリ上に等位置で載っているので、読み取りは実装できる見込み（推測）。入力は 0x400000 の `.rsrc` |
| A3 | kernel32 グローバルメモリ・アトム・文字列: GlobalAlloc、GlobalFree、GlobalLock、GlobalUnlock、AddAtomW、DeleteAtom、FindAtomW、GlobalAddAtomW、GlobalFindAtomW、MulDiv、lstrlenW | 11 / 11 | 新規 `fle_glob.c/.h`、表に 11 行 | 主に公式で決まる（MulDiv の丸めと -1 の返り方、アトムの範囲 0xC000 以上）。GMEM_MOVEABLE のハンドルの表現は判断が要る（ハンドル値の選び方）。アトムの大文字小文字の扱いは、公式ページで要確認 |
| A4 | kernel32 時刻・日付: DosDateTimeToFileTime、FileTimeToDosDateTime、FileTimeToSystemTime（変換だけ。公式で決まる）、FileTimeToLocalFileTime、LocalFileTimeToFileTime、SystemTimeToTzSpecificLocalTime、GetTimeZoneInformation、GetLocalTime、GetDateFormatW | 9 / 9 | 新規 `fle_time.c/.h`、表に 9 行 | 変換 3 件は公式で決まる。タイムゾーン、現在時刻の源（`fle_sys` の仮想クロック、`fle_vfs` の仮想時計のどちらか）、日付の書式は判断が要る |
| A5 | kernel32 ファイル・ボリュームの残り: GetStdHandle、GetDriveTypeW、GetLogicalDrives、GetLogicalDriveStringsW、GetVolumeInformationW、QueryDosDeviceW、DuplicateHandle | 7 / 7 | 新規 `fle_vol.c/.h`、表に 7 行 | 戻り値の形式は公式で決まる。ドライブ構成（`C:\` だけ）は `fle_vfs.h` の「空の `C:\`」に合わせる判断。DuplicateHandle、GetStdHandle は `fle_vfs.c` のハンドル表（static）に触れる可能性があり、`fle_vfs.c` を触らずに済むかは未確認 |
| A6 | kernel32 ライブラリロード: LoadLibraryA、LoadLibraryW、LoadLibraryExW、FreeLibrary、SetDllDirectoryW | 5 / 7 | 新規 `fle_load.c/.h`、表に 5 行 | 戻り値は公式で決まるが、どの DLL を「ある」とするかは判断が要る。`fle_modules.c` の仮想モジュール表（決定 0015）と整合させる必要がある（読んでの確認は部分的） |
| A7 | oleaut32: SysAllocStringLen、SysFreeString、SysReAllocStringLen、VariantInit、VariantClear、VariantCopy、VariantCopyInd、SafeArrayCreate、SafeArrayGetLBound、SafeArrayGetUBound、SafeArrayPtrOfIndex | 11 / 11 | 新規 `fle_oleaut.c/.h`、表に 11 行 | BSTR（長さの前置）と VARIANT、SAFEARRAY の配置は公式に定義がある。VariantClear が COM オブジェクトの Release を呼ぶ形（ゲストの関数ポインタの呼び出し）は、ゲスト関数を runtime から呼ぶ仕組みが要る。**ゲスト呼び出しの設計が要る** |
| A8 | ole32: CoInitialize、OleInitialize、CoTaskMemFree、IsEqualGUID、CLSIDFromString、StringFromCLSID | 6 / 6 | 新規 `fle_ole.c/.h`、表に 6 行 | 公式で決まる（GUID の文字列形式）。CoTaskMemFree は CoTaskMemAlloc がこの閉包にないので、解放対象の由来を確認する必要がある（未確認） |
| A9 | user32 の窓なし部分: CharLowerW、CharLowerBuffW、CharUpperW、CharUpperBuffW、CharNextW、SetRect、IntersectRect、OffsetRect、UnionRect、RegisterWindowMessageW、RegisterClipboardFormatW、GetClipboardFormatNameW | 12 / 13 | 新規 `fle_user_nowin.c/.h`、表に 12 行 | RECT 4 件は公式で決まる。登録系 3 件は 0xC000 以上の値の割り当て（公式で範囲が決まる）。CharXxx の ASCII 外の変換は、Windows のロケール依存で、`fle_env.h` は ASCII のみの折り畳みと明記している。ASCII 外をどうするかは判断が要る |
| A10 | winmm: midiInGetNumDevs、midiOutGetNumDevs、midiInGetDevCapsW、midiOutGetDevCapsW、midiOutGetErrorTextW、midiOutOpen、midiOutShortMsg、timeBeginPeriod、timeEndPeriod、timeGetDevCaps | 10 / 10 | 新規 `fle_winmm.c/.h`、表に 10 行 | 戻り値の名前（MMSYSERR_*）は公式。MIDI デバイスが 0 台、timeGetDevCaps の最小・最大値は判断が要る |
| A11 | 「仮想環境が何を主張するか」の判断の束: shell（SHGetFolderPathW ×2 スロット、SHGetMalloc、PathCleanupSpec、ShellExecuteW、ShellExecuteExW）、version（3）、advapi32 トークン（2）、wininet（6）、crypt32・wintrust（8）、dsound・msacm32（3）、ロケール系の残り（EnumCalendarInfoW、EnumSystemLocalesW、FormatMessageW、GetComputerNameExW） | 32 / 32（SHGetFolderPathW が 2 スロットで 2 API 分） | 新規ファイル群（系統ごと）、表に行 | 戻り値の形は公式。ただし、「ネットワークなし」「デバイスなし」「署名検証の結果」「フォルダのパス」などの中身は、すべて判断が要る。**PM の判断が先に要る**。EnumCalendarInfoW、EnumSystemLocalesW はコールバックがあり、A7 と同じゲスト呼び出しの仕組みが要る |
| A12 | imagehlp!ImageNtHeader | 1 / 1 | 新規 `fle_imagehlp.c`、表に 1 行 | 公式で決まる（ベースから IMAGE_NT_HEADERS へのポインタ。e_lfanew を読む） |
| A13 | kernel32 メモリ保護・ファイルマッピング: VirtualProtect、VirtualQuery、CreateFileMappingW、MapViewOfFile、UnmapViewOfFile | 5 / 5 | 新規 `fle_vmap.c/.h`、表に 5 行 | 戻り値と構造体（MEMORY_BASIC_INFORMATION）は公式で決まる。`fle_sys.c` の VirtualAlloc の領域表（static の `fle_region`）を読む必要があり、`fle_sys.c` を触らずに済むかは未確認。マッピング対象のファイルは `fle_vfs.c` のハンドル（static）に依存する可能性があり、**他の単位と共有状態の整合が要る** |

検算（`check_units.py`、結果 `units-v3.tsv`）: A1〜A13 と D1〜D4、D6、D7 の割り当てで、応答なしの 314 API・326 thunk を、漏れも重複もなく分けた（D5 の delay 18 件は 378 件の外なので、この 314 には入らない）。A 単位の合計は 127 API・130 thunk、D 単位（D5 を除く）の合計は 187 API・196 thunk。127 + 187 = 314、130 + 196 = 326。

### 5.2 実体の設計が要る（別に分ける。波には入れない）

| ID | 対象 | API 数 / thunk | 理由 |
|---|---|---|---|
| D1 | user32 ウィンドウ・メッセージ・入力・メニュー・描画 | 108 / 115 | **実体の設計が要る**。ウィンドウ、メッセージキュー、フォーカス、キャプチャ、タイマ、フックなどの状態を持つモデルが要る。API 単体では決まらない |
| D2 | gdi32 | 54 / 54 | **実体の設計が要る**。デバイスコンテキスト、ビットマップ、リージョン、フォント、パレットのモデルが要る。描画の出力先（実画面か仮想サーフェスか）の決定が先 |
| D3 | comctl32 フラットスクロールバー | 4 / 4 | **実体の設計が要る**。ウィンドウに属するので D1 に依存 |
| D4 | user32 システム設定・ディスプレイ: GetSystemMetrics、SystemParametersInfoW（3 thunk）、GetSysColor、GetKeyboardLayout、GetKeyNameTextW、MapVirtualKeyW、EnumDisplayMonitors、EnumDisplaySettingsW、GetMonitorInfoW、MonitorFromPoint、MonitorFromRect、MonitorFromWindow、shell32!SHAppBarMessage | 13 / 15 | **実体の設計が要る**。仮想ディスプレイの構成（解像度、モニタ数、DPI）、キーボード配列を先に決める必要がある |
| D5 | delay import 18 thunk と、`jmp rax`（delay-load ヘルパーの末尾。推測）18 行 | 18 API / 18 thunk | **設計が要る**。ヘルパー（0x41cfc0 が `call [0x13a9398]`、その初期値は 0x41d9a0 と読めた。バイナリを読んだ実測）を実行して解決するか、18 API ごとに応答するかを決める。前者はゲスト内の解決処理を通すので、`GetProcAddress` の応答（`fle_modules.c`）と整合させる必要がある |
| D6 | スレッド・イベント・待機: CreateThread、ResumeThread、WaitForSingleObject、CreateEventW、SetEvent、ResetEvent、GetExitCodeThread | 7 / 7 | **実体の設計が要る**。単一のゲストスレッドのモデルで、スレッド本体をいつ実行するか、待機でどう進めるかを決める（`fle_sys.h` は「Single guest thread only」「待ち続けるケースは止める」としている） |
| D7 | ole32!CoCreateInstance | 1 / 1 | **実体の設計が要る**。どの CLSID に何を返すか（COM クラスの登録、オブジェクトの実体、vtable）が決まっていない。A7、A8 とは独立に、COM のオブジェクトモデルの設計が先 |

### 5.3 命令の単位（`fe-lift/src/<名前>.rs`、`runtime/fle_<名前>.c/.h`、テストは新規 `tests/<名前>.rs`。lib.rs は上記の例外）

| ID | 対象 | 件数 | 仕様確認 |
|---|---|---|---|
| V1 | SSE2 の xmm 整数パック: Pshuflw 16、Psllw 12、Packssdw 10、Psrlw 8、Paddusb 6、Pmaxsw 6、Pminsw 6、Pmulhw 6、Pmullw 6、Psubsw 4、Psubusb 3、Pshufhw 2、Psrad 2、Paddq 1、Paddsw 1、Psraw 1、Psrld 1 | 91（17 種） | SDM の擬似コードで決まる（飽和、シフト幅が大きい場合の結果は SDM に定義がある）。既存の `sse_more.rs`、`sse_pack.rs` と同じ形で、`fle_xmm` を使える |
| V2 | SSE 浮動小数点のシャッフル・半分移動: Shufps 13、Movlps 8、Movhlps 3、Movlhps 2 | 26 | SDM で決まる。メモリ形（Movlps）はアラインメント規則が `sse_pack.rs` と異なるかの確認が要る（Movlps のメモリ形は 8 バイト） |
| V3 | MMX: Movd 10、Punpcklbw 5、Packuswb 4、Paddw 4、Pxor 4、Emms 3、Pmulhuw 3、Movq 2、Psllw 2、Pmulhw、Pshufw、Psubsw 各 1 | 40（12 種、関数 3） | 命令の意味は SDM で決まる。mm レジスタの状態（8 本の 64 ビット）を、x87 のスタックと別の配列にするか、重ねるかは判断が要る。この閉包に x87 の演算命令は 0 件（`x87ctl-run.md` §1）なので、別配列でも観測できる差は出ない見込み（推測） |
| V4 | 整数: Xchg 11、Bsf 5、Bts 5、Rol 4、Ror 3、Btr 2、Lea（FS 接頭辞）1 | 31 | SDM で決まる（Rol/Ror のカウントのマスクと、カウント 0 のときフラグ不変、Bt* のレジスタ形のビット位置がメモリのオペランド範囲外を指す場合、Xchg の暗黙の LOCK）。単一スレッドなので LOCK はそのまま実装できる。Lea の FS 接頭辞はアドレス計算に影響しない（SDM の LEA の定義。要確認） |
| V5 | システム: Pause 3、Sfence 2、Cmpxchg16b 2 | 7 | Pause、Sfence は単一スレッドで何もしない（SDM の定義）。Cmpxchg16b は SDM で決まる（16 バイトの境界に無いときの扱いは `fle_stop` にする判断が要る） |
| V6 | 制御・セグメント・解決不能: Jecxz 2、`mov rax,gs:0x58` 1、Xgetbv 1、解決不能 jmp 20（`jmp rax` 18、`jmp [r10+rdx*8+0x40]` 1、`jmp [rcx]` 1） | 24 | **lib.rs と fe-decode の変更が要る**（新規ファイルのみでは済まない）。Jecxz は Ctl の扱い（条件分岐）に関わる。`gs:0x58` は TEB の配置に依存し、公式の API ページでは決まらない（判断が要る）。`jmp [r10+rdx*8+0x40]` は、`.text` 内の表（8 個の要素 0x40d260〜0x40d298、飛び先は 0x40d2b6〜0x40d2ce の 8 つ。バイナリを読んで計算）なので解決できる見込み。`jmp [rcx]` は、`longjmp` 相当の形（推測）で、飛び先が実行時にしか決まらない |
| V7 | AVX/VEX: Vmovups 12、Vmovaps 2、Vmovntps 2、Vzeroupper 2 | 18 | SDM で決まるが、ymm の上位半分を持つレジスタモデルが要る。上の §4 のとおり、CPUID が AVX を主張しないので、実行されない見込みがある（推測。0x15e69c0 の出所が未確認）。**着手を後にすることを提案** |

### 5.4 起動順の提案（推測を含む）

1. 先に、生成器を再実行して、15:41 の対応表（83 行）で C を作り直す。期待は未対応 615 → 563（算術）。実行を進めて、次の停止点を測る。これがないと、A/V の優先順位は実測に基づかない。
2. A1、A2、A3、A4、A9、A10、A12、V1、V2、V4、V5 は、互いに独立に起動できる。A5、A6、A13 は他の runtime の共有状態（`fle_vfs.c`、`fle_modules.c`、`fle_sys.c`）に触れる可能性があるので、読んで確認してから起動する（新規ファイルのみ。表と lib.rs への追記が集約点）。
3. A11 は、PM の判断が先に要る。D1〜D6 は、設計文書が先に要る。V3、V6、V7 は、判断または別の変更が要る。

## 6. 失敗・拒否された操作と、作業の限界

- 権限の拒否は 0 件。
- 私が書いた最初の版の §1.4 は、「17 種」と書きながら 18 個を並べる誤りがあった。実測で 18 API・18 thunk と確かめ、その節を書き直した（同じ新規ファイル内の修正）。
- 集計ツールの 1 回目の系統分類（`iat-by-family.tsv`）は、正規表現による大まかなもので、一部の API が不自然な系統に入った（例: GetLocalTime がメモリ）。`aggregate_v2.py` で明示的な表に直した。1 回目の出力ファイル（`iat-by-family.tsv`、`no-answer-apis.tsv`）は残してあるが、採用は v2 と v3 の出力。
- 作業中に、他のサブエージェントが `lift/runtime/`、`imports_table.rs` を更新した（対応表が 39 → 83 行、`fle_vfs.c`、`fle_reg.*` の出現）。§0 の注に時刻つきで書いた。
- 読んだだけで確認していないもの: 生成器の再実行、生成 C のビルド、実行（cargo は使っていない）。公式の Intel SDM と Microsoft Learn の取得もしていない（§5 の「公式で決まる」は、私の理解と既存の決定文書の記述にもとづく見込みで、各単位の着手時に確認が要る）。
- 目的であった「FLEngine のフル lift」は、この作業では達成していない。この文書は、残りの未対応の集計と、次の波の提案までである。実行の停止点は、まだ 0x407040（GetCommandLineW）。
