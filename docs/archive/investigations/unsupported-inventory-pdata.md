# all-pdata seed の lift に残る未対応の棚卸し（IAT 経由 jmp ほか）

日付: 2026-10-20。読み取り専用の調査（ソース、runtime、`imports_table.rs` は変更していない）。数値の根拠は `lift/out-inv/` のスクリプトとログ（§9 に一覧）。

表記: 「**実測**」はスクリプト／ツールの出力、「**推測**」は未検証の解釈。API の分類（§3）は API 名からの判断で、仕様書は読んでいない（すべて**推測**）。

## 0. 前提と注意（実測）

- 対象の生成 C: `lift/out-pdata/out/after-pdata-b-part-00..15.c`（16 分割、`after-pdata-b.c` と同内容。生成 19:21、`gen-pdata-b.txt`）。関数 58,695、`unsupported_total` **2,991**（54 種類）= `Jmp` **680**（IAT 経由 618 + 解決不能 62）+ それ以外 **2,311**（52 種類のニーモニック）。
- **生成 C は古い**: 生成の後で `imports_table.rs` の TABLE は増えている（棚卸し時点で 173 行、sha256 先頭 `c89be6e39aab`。0034 sysinfo の `GetNativeSystemInfo` 等が入っている）。HEAD も依頼時の a9bec91 から 7757e07（0032: VEX、FS lea、GS:[0x58]）へ進んでいた。生成 C は再生成していない（巨大ファイルを複製しないため）。したがって「生成 C に残る未対応」と「今の TABLE が答える API」は別に数えた（§1、§2.4）。
- 「直接呼び出し元」= 生成 C の `f_X();`（call と tail jmp）の呼び出し元関数の異なる数。**間接 call（`fle_dispatch(t)`、全体で 40,725 か所）とデータポインタ（vtable、ウィンドウプロシージャ、InitTable 等）は数えていない**。したがって数値は下限。
- 閉包内に `ILTools`、`dsp_ippv2` の IAT / delay-load 経由 thunk は **0 件**（実測）。これらは文字列としてだけ存在し（`dsp_ippv2.dll` は UTF-16 の文字列、`ILToolsCreateCursorFromFontW` は API 名の一部）、LoadLibrary の動的な呼び出しで使われる。この棚卸しの対象外（0029 の記述と同じ）。
- 依頼の「ADR-0010」「M2」に当たる文書はリポジトリに無い（0029 §9 にも同じ記録。`docs/decisions/0010-int-ext.md` は符号拡張の決定）。(e) の扱いは依頼文と 0029 の記述（外部 DLL は M2、未知は fail-closed）に従った。

## 1. IAT 経由 jmp の未対応: DLL ごとの集計（実測）

生成 C の `Jmp: indirect through IAT slot (not lifted)` **618 行**（thunk 618 関数。各 thunk は `jmp [IAT]` 1 命令）。23 DLL、**590 API**（`jmp_inventory.log`、`final_counts.log`）。

そのうち **27 thunk（23 API）は、棚卸し時点の TABLE に既に行がある**（`GlobalAlloc` など 0024 の kernel32 の 23 API。`gen-pdata-b.txt` に `jmp_imp:GlobalAlloc` 等が無い＝生成後に追加された行）。それを除くと **591 thunk、567 API**（`jmp_thunks.tsv` の `answered=0`）。現在の TABLE（173 行）はさらに `GetProcessAffinityMask`、`GlobalMemoryStatus`、`SetProcessAffinityMask` の IAT 3 thunk に答えるので、**588 thunk** が未対応（`recount_now.txt`）。

| DLL | thunk（生成 C） | うち TABLE に行あり | 未対応 thunk | 未対応 API |
|---|---:|---:|---:|---:|
| user32.dll | 221 | 0 | 221 | 209 |
| gdi32.dll | 129 | 0 | 129 | 126 |
| kernel32.dll | 88 | 27 | 61 | 58 |
| ole32.dll | 31 | 0 | 31 | 29 |
| comctl32.dll | 31 | 0 | 31 | 30 |
| shell32.dll | 24 | 0 | 24 | 22 |
| oleaut32.dll | 20 | 0 | 20 | 19 |
| wininet.dll | 15 | 0 | 15 | 15 |
| msacm32.dll | 14 | 0 | 14 | 14 |
| winmm.dll | 13 | 0 | 13 | 13 |
| crypt32.dll | 7 | 0 | 7 | 7 |
| winspool.drv | 5 | 0 | 5 | 5 |
| mpr.dll | 4 | 0 | 4 | 4 |
| version.dll | 3 | 0 | 3 | 3 |
| advapi32.dll | 2 | 0 | 2 | 2 |
| dsound.dll | 2 | 0 | 2 | 2 |
| imagehlp.dll | 2 | 0 | 2 | 2 |
| opengl32.dll | 2 | 0 | 2 | 2 |
| wintrust.dll、shfolder.dll、shlwapi.dll、oleacc.dll、api-ms-win-crt-string-l1-1-0.dll | 各 1 | 0 | 各 1 | 各 1 |
| **合計** | **618** | **27** | **591** | **567**（TABLE 行ありを含む全体で 590） |

PE の IAT は 738 import（24 DLL）。生成 C に未対応の thunk として残るのは 590 API。**`call [IAT スロット]` の直接呼び出しは .text に 0 件**（実測: `objdump -d` の `call QWORD PTR [rip+..]` 787 件のうち、IAT／遅延 IAT の範囲 0x196xxxx〜0x197xxxx を指すものは 0。`Call: indirect through IAT slot` も `gen-pdata-b.txt` に無い）。IAT 経由の呼び出しはすべて thunk（`jmp [IAT]`）を通る。thunk が閉包に無い import（738 − 590 − 生成時点で応答済み）は、閉包から呼ばれない、または `fle_dispatch` の先で呼ばれる（後者は未確認）。

### 1.1 `Jmp: indirect target unresolved` 62 行の内訳（実測、`jmp_other.tsv`、`delay_map.log`）

| 種類 | 行数 | 内容 |
|---|---:|---|
| 遅延 import ヘルパの末尾 `jmp rax` | **57** | 17 か所の `jmp rax`（0x4071bf、0x4072ff、0x42304f、0x42316f、0x423e6f、0x42474f、0x42572f、0x43916f、0x4692af、0x46932f、0x5933cf、0x59396f、0x5dc61f、0x5dcacf、0x5e103f、0x741fdf、0x80f9ef）が、57 個の thunk 関数に重複してインライン展開された。57 個は PE の遅延 import 57 スロットと 1 対 1 |
| `jmp [r10+rdx*8+0x40]`（f_40d210 内、0x40d253） | 1 | memset 風の小サイズ分岐の 8 エントリの表（0x40d2c1 など。`jmp_table_40d210.txt`） |
| `jmp [rcx]`（f_4126f0 内、0x412752） | 1 | longjmp 風（rbx、rbp、rsi、rdi、r12〜r15、xmm6〜15、rsp を [rcx] から復元して [rcx] へ jmp）。対になる f_412680 が setjmp 風 |
| `movsxd; add; jmp rcx/rax`（switch の相対表） | 3 | 0x5e5eb6、0x5e9ae8、0x5f1499 |

## 2. 分類ごとの数（実測の数＋推測の分類）

分類規則は §3 の注記。**thunk** = 未対応の thunk 関数の数、**API** = 異なる (DLL, API) の数、**呼び出し元** = 直接呼び出し元関数の異なる数（分類内の和集合、`class_summary.txt`）。IAT の数字は生成 C 時点で TABLE に行が無い 591 thunk 分。

### 2.1 IAT 経由 jmp（591 thunk）

| 分類 | thunk | API | 直接呼び出し元 |
|---|---:|---:|---:|
| (a) 状態を持たない純粋関数・定数 | 52 | 47 | 251 |
| (a′) 「デバイス／サービス無し」型（wininet、winspool、mpr、msacm32、dsound、crypt32、wintrust、winmm の MIDI 残り）。環境についての主張を伴う | 61 | 61 | 36 |
| (b) 仮想 OS 状態 | 87 | 82 | 215 |
| (c) ウィンドウ・メッセージ・GDI・コモンコントロール・入力 | 370 | 356 | 1,306 |
| (d) COM/OLE | 21 | 21 | 72 |
| (e) 外部 DLL | 0 | 0 | 0 |
| 合計 | 591 | 567 | 1,816（和集合） |

### 2.2 遅延 import thunk（57 関数。`jmp rax` が解決不能）

| 分類 | thunk | API | 直接呼び出し元 |
|---|---:|---:|---:|
| (a) 純粋・定数（kernel32 の 8、shell32 の SHGetKnownFolderPath、user32 の DPI／タッチ 5） | 14 | 14 | 23 |
| (b) 仮想 OS 状態（RegDeleteKeyExW、CreateSymbolicLinkW、GetFinalPathNameByHandleW） | 3 | 3 | 3 |
| (c) ウィンドウ系（user32 の 10、shell32 の SHOpenFolderAndSelectItems。MessageBoxA を含む） | 11 | 11 | 27 |
| (d) COM/OLE（PropVariantClear、SHCreateItemFromParsingName） | 2 | 2 | 11 |
| (e) 25 名の仮想モジュール表に無い DLL（dwmapi 2、imm32 7、msimg32 2、propsys 5、rpcrt4 1、shcore 2、uxtheme 5、windowscodecs 1、wtsapi32 2） | 27 | 27 | 31 |
| 合計 | 57 | 57 | 88（和集合） |

### 2.3 (f) 解決不能 jmp（delay-load の `jmp rax` 以外）

| 種類 | 箇所 | 関数 | 直接呼び出し元関数 |
|---|---:|---|---:|
| delay-load ヘルパ末尾 `jmp rax`（§2.2 と同じもの） | 17 か所（57 関数に展開） | 57 thunk | 88（thunk の呼び出し元） |
| f_40d210 の表 jmp（memset 風） | 1 | 1 | **337** |
| f_4126f0 の `jmp [rcx]`（longjmp 風） | 1 | 1 | 1（f_412770。その先 f_4128b0 の呼び出し元は 5） |
| switch の相対表 | 3 | 3 | 10 |

### 2.4 現在の TABLE（173 行）との差（実測、`recount_now.txt`）

- IAT 未対応: A 51、A′ 61、B 85、C 370、D 21、E 0 = **588 thunk**。
- delay-load の `GetLogicalProcessorInformation`、`GetNativeSystemInfo`、`IsWow64Process` には TABLE 行がある（0034）。ただし **行があるだけでは通らない**: これらは遅延ロードのヘルパが `GetProcAddress` で引き、戻り値 rax へ `jmp rax` する（0x42316f）。生成 C ではこの `jmp rax` が `fle_unsupported`（実測、`L_42316f`）。GetProcAddress が擬似 VA（0x7FF9...）を返すようになると、**次の停止点は 0x42316f の `jmp rax` になる**（生成 C の読解に基づく**推測**。再生成して実行はしていない）。§6 の W1 がそれを解く。

### 2.5 閉包内の他の未対応（IAT 以外、参考）

`recount_now.txt`、`insn_table.tsv`: 2,311 行、52 ニーモニック。
- x87（F*）: 32 ニーモニック、**2,193 行**、73 関数。
- SSE/SSE2 の packed（Mulpd、Addpd、Subpd、Cvtdq2ps、Rcpps、Pcmpgtd、Psubw、Cvtpd2ps、Cvtps2pd、Andnpd）: 86 行、25 関数。
- その他: Bswap 5、Prefetcht0 5、And（high-byte）1、Xgetbv 1、Vmovups 12、Vmovaps 2、Vmovntps 2、Vzeroupper 2、Lea（FS）1、Mov（GS）1。VEX の 5 種と Lea／Mov は 7757e07（0032）で対応済み（生成 C は古いので残っている）。Xgetbv は 0032 のコミットメッセージで「CPUID ECX=0 で守られている」とある（未検証）。
- これらの関数は、**InitTable の 895 件とエントリ f_1376820 の直接呼び出し閉包（和集合 U、6,297 関数）に、x87 と SSE packed は 0 関数**（実測）。U に入る非 Jmp は VEX の 5 種、Lea、Mov、Xgetbv だけ。

## 3. API 一覧と分類

分類は API 名からの**推測**。規則:
(a) kernel32 の状態なしの関数と環境定数、純粋な矩形計算、DPI／キーボードレイアウト等の環境値、`IsEqualGUID`、`VariantInit`、`GetErrorInfo`、`memset`。FormatMessageW は純粋ではない（メッセージ表が要る）が、状態表を持たないので (a) に置いた。
(b) ハンドル、スレッド、同期、ファイル、ヒープ、BSTR／Variant／SafeArray の割り当て、CoTaskMem、CoInitialize 系、プロセス、レジストリ。
(c) user32 の大半、gdi32 全部（GdiFlush を除く）、comctl32、opengl32、ドラッグ、通知領域、クリップボード、メニュー、キャレット、カーソル。
(d) CoCreateInstance、Ole のクリップボード／ドラッグ、IStream／ILockBytes、SHGetMalloc／SHGetDesktopFolder、ReleaseStgMedium、LresultFromObject、GetActiveObject。
(a′) 外部サービスのない仮想環境で「存在しない／失敗」を返す類。(a) の部分集合だが、「環境が何を持つか」という主張になるので PM の決定が要る（0029 §「外部 DLL」と同じ扱い）。
`*` = ゲストの関数ポインタを取る／呼び返す可能性がある（コールバック機構が要る。**36 API、40 thunk**、`class_summary.txt`）。
書式 `API(thunk 数/直接呼び出し元数,u=その API を閉包に含む InitTable unit の数)`（u は 0 のときは省略。§5 の静的推定）。

#### (a) 状態なし・定数 / IAT 経由 jmp: API 47、thunk 52
- `api-ms-win-crt-string-l1-1-0.dll` 1: memset(1/6)
- `gdi32.dll` 1: GdiFlush(1/7,u=2)
- `imagehlp.dll` 2: ImageDirectoryEntryToData(1/1) ImageRvaToVa(1/1)
- `kernel32.dll` 16: CompareFileTime(1/1) CompareStringA(1/1) EnumCalendarInfoW*(1/1,u=2) EnumSystemLocalesW*(1/1,u=1) FormatMessageW(2/5,u=22) FreeResource(1/1) GetCPInfoExW(1/1) GetComputerNameExW(1/25,u=4) GetComputerNameW(1/1) GetConsoleCP(1/1) GetConsoleOutputCP(1/1) GetOEMCP(1/1) GetUserDefaultLCID(1/2) GlobalMemoryStatus(1/1) IsDebuggerPresent(1/1) TzSpecificLocalTimeToSystemTime(1/1)
- `ole32.dll` 2: CLSIDFromString(1/1) IsEqualGUID(3/42)
- `oleaut32.dll` 2: GetErrorInfo(1/1) VariantInit(1/39,u=3)
- `shell32.dll` 3: PathCleanupSpec(1/1,u=2) SHAddToRecentDocs(1/1) SHGetFolderPathW(1/1,u=4)
- `shfolder.dll` 1: SHGetFolderPathW(1/3,u=2)
- `shlwapi.dll` 1: PathUnExpandEnvStringsW(1/1)
- `user32.dll` 18: CharNextExA(1/1) CharToOemBuffW(1/1) EqualRect(1/2) GetDoubleClickTime(1/1) GetGuiResources(1/2) GetKeyNameTextW(1/1,u=2) GetKeyboardLayout(1/3,u=1) GetKeyboardLayoutList(1/1) GetKeyboardLayoutNameW(1/1) GetMessageExtraInfo(1/1) GetSysColor(1/1,u=7) GetSystemMetrics(1/35,u=11) InflateRect(1/12) MapVirtualKeyW(1/7,u=2) MessageBeep(1/5) PtInRect(1/13) SubtractRect(1/1) SystemParametersInfoW(3/19,u=9)

#### (a) 状態なし・定数 / 遅延 import（`jmp rax` 末尾）: API 14、thunk 14
- `kernel32.dll` 8: GetLogicalProcessorInformation(1/2,u=84) GetNativeSystemInfo(1/2,u=1) GetSystemDefaultLocaleName(1/1,u=1) GetTimeZoneInformationForYear(1/1) GetUserPreferredUILanguages(1/1) IsWow64Process(1/1) LocaleNameToLCID(1/1,u=1) NormalizeString(1/1)
- `shell32.dll` 1: SHGetKnownFolderPath(1/1)
- `user32.dll` 5: EvaluateProximityToRect(1/6) IsProcessDPIAware(1/1,u=3) PackTouchHitTestingProximityEvaluation(1/4) SetThreadDpiAwarenessContext(1/4) SetThreadDpiHostingBehavior(1/2)

#### (a′) 「デバイス/サービス無し」型（環境の主張を伴う） / IAT 経由 jmp: API 61、thunk 61
- `crypt32.dll` 7: CertCloseStore(1/1) CertFindCertificateInStore(1/1) CertFreeCertificateContext(1/1) CertGetNameStringW(1/1) CryptMsgClose(1/1) CryptMsgGetParam(1/1) CryptQueryObject(1/1)
- `dsound.dll` 2: DirectSoundCreate(1/1,u=2) DirectSoundEnumerateA(1/1)
- `mpr.dll` 4: WNetCloseEnum(1/1) WNetEnumResourceW(1/1) WNetGetUniversalNameW(1/1) WNetOpenEnumW(1/1)
- `msacm32.dll` 14: acmDriverClose(1/2) acmDriverDetailsA(1/1) acmDriverEnum*(1/1) acmDriverOpen(1/1) acmFormatChooseW*(1/1) acmFormatSuggest(1/1) acmMetrics(1/3,u=1) acmStreamClose(1/2) acmStreamConvert(1/1) acmStreamMessage(1/1) acmStreamOpen(1/1) acmStreamPrepareHeader(1/1) acmStreamSize(1/1) acmStreamUnprepareHeader(1/1)
- `wininet.dll` 15: HttpAddRequestHeadersW(1/1) HttpEndRequestW(1/1) HttpOpenRequestW(1/1) HttpQueryInfoA(1/2) HttpQueryInfoW(1/1) HttpSendRequestExW(1/1) InternetCloseHandle(1/3) InternetConnectW(1/1) InternetGetConnectedState(1/1,u=2) InternetGetLastResponseInfoW(1/2) InternetOpenUrlW(1/2) InternetOpenW(1/3) InternetReadFile(1/3) InternetSetOptionW(1/1) InternetWriteFile(1/1)
- `winmm.dll` 13: midiInAddBuffer(1/2) midiInClose(1/1) midiInGetErrorTextW(1/1) midiInOpen*(1/1) midiInPrepareHeader(1/2) midiInReset(1/1) midiInStart(1/1) midiInStop(1/1) midiInUnprepareHeader(1/1) midiOutClose(1/1) midiOutLongMsg(1/1) midiOutPrepareHeader(1/1) midiOutUnprepareHeader(1/1)
- `winspool.drv` 5: ClosePrinter(1/2) DocumentPropertiesW(1/1) EnumPrintersW(1/2) GetDefaultPrinterW(1/1) OpenPrinterW(1/1)
- `wintrust.dll` 1: WinVerifyTrust(1/1)

#### (b) 仮想 OS 状態 / IAT 経由 jmp: API 82、thunk 87
- `advapi32.dll` 2: GetTokenInformation(1/1) OpenProcessToken(1/1)
- `kernel32.dll` 42: AddVectoredExceptionHandler*(1/1) AssignProcessToJobObject(1/1) ChangeTimerQueueTimer(1/1) CreateEventW(1/14,u=11) CreateProcessW(1/2) CreateThread*(2/3,u=13) CreateTimerQueueTimer*(1/1) DeleteTimerQueueTimer(1/1) ExitThread(1/1) GetDiskFreeSpaceW(1/2) GetExitCodeProcess(1/3) GetExitCodeThread(1/1,u=2) GetFileInformationByHandle(1/2) GetFileType(1/1) GetPrivateProfileStringW(1/3) GetProcessAffinityMask(1/3) GetTempFileNameW(1/3) GetThreadPriority(1/2) GlobalDeleteAtom(1/2) GlobalSize(1/7) HeapCreate(1/1) HeapDestroy(1/2) IsBadWritePtr(1/2) OpenFileMappingW(1/1) OpenProcess(1/1) ResetEvent(1/7,u=8) ResumeThread(1/2,u=6) RtlUnwindEx*(1/1) SetEndOfFile(2/3) SetEvent(1/17,u=4) SetProcessAffinityMask(1/6) SetThreadExecutionState(1/4) SetVolumeLabelW(1/1) SignalObjectAndWait(1/1) SuspendThread(1/1) TlsFree(1/1) UnhandledExceptionFilter*(1/1) VirtualQueryEx(1/1) WaitForMultipleObjects(1/1) WaitForMultipleObjectsEx(1/3) WaitForSingleObject(1/16,u=3) WritePrivateProfileStringW(1/4)
- `ole32.dll` 8: CoInitialize(1/4,u=2) CoTaskMemAlloc(1/2) CoTaskMemFree(1/26,u=2) CoUninitialize(1/7) OleInitialize(1/4,u=2) OleUninitialize(1/5) ProgIDFromCLSID(1/1) StringFromCLSID(1/1)
- `oleaut32.dll` 16: SafeArrayAccessData(1/1) SafeArrayCreate(1/2,u=3) SafeArrayGetElement(1/2) SafeArrayGetElemsize(1/1) SafeArrayGetLBound(1/3,u=3) SafeArrayGetUBound(1/3,u=3) SafeArrayPtrOfIndex(1/3,u=3) SafeArrayPutElement(1/2) SafeArrayUnaccessData(1/1) SysAllocStringLen(1/3,u=6) SysFreeString(2/6,u=85) SysReAllocStringLen(1/1,u=83) VariantChangeType(1/1) VariantClear*(1/2,u=3) VariantCopy*(1/2,u=3) VariantCopyInd*(1/1,u=2)
- `shell32.dll` 11: ILCreateFromPathW(1/2) ILFree(1/4) SHChangeNotifyDeregister(1/1) SHChangeNotifyRegister*(1/1) SHFileOperationW(2/3) SHGetFileInfoW(1/2) SHGetPathFromIDListW(2/3) SHGetSpecialFolderLocation(1/2) ShellExecuteA(1/1) ShellExecuteExW(1/3) ShellExecuteW(1/17,u=2)
- `version.dll` 3: GetFileVersionInfoSizeW(1/3,u=3) GetFileVersionInfoW(1/3,u=3) VerQueryValueW(1/3,u=3)

#### (b) 仮想 OS 状態 / 遅延 import（`jmp rax` 末尾）: API 3、thunk 3
- `advapi32.dll` 1: RegDeleteKeyExW(1/1)
- `kernel32.dll` 2: CreateSymbolicLinkW(1/1) GetFinalPathNameByHandleW(1/1,u=2)

#### (c) ウィンドウ・メッセージ・GDI・コモンコントロール・入力 / IAT 経由 jmp: API 356、thunk 370
- `comctl32.dll` 30: FlatSB_GetScrollInfo(1/1) FlatSB_GetScrollPos(1/1) FlatSB_SetScrollInfo(1/1) FlatSB_SetScrollPos(1/1) FlatSB_SetScrollProp(1/1) ImageList_Add(1/2) ImageList_BeginDrag(1/2) ImageList_Create(1/1) ImageList_Destroy(1/1) ImageList_DragEnter(1/4) ImageList_DragLeave(1/4) ImageList_DragMove(1/2) ImageList_DragShowNolock(1/4) ImageList_Draw(1/2) ImageList_DrawEx(1/3) ImageList_EndDrag(1/3) ImageList_GetBkColor(1/2) ImageList_GetDragImage(2/2) ImageList_GetIcon(1/1) ImageList_GetIconSize(1/1) ImageList_GetImageCount(1/3) ImageList_Read(1/1) ImageList_Remove(1/1) ImageList_Replace(1/1) ImageList_SetBkColor(1/1) ImageList_SetIconSize(1/4) ImageList_SetImageCount(1/1) ImageList_Write(1/1) InitializeFlatSB(1/1) _TrackMouseEvent(1/1)
- `gdi32.dll` 125: AngleArc(1/1) Arc(1/1) ArcTo(1/1) BeginPath(1/1) BitBlt(1/21,u=7) Chord(1/1) CloseEnhMetaFile(1/1) CombineRgn(1/12,u=2) CopyEnhMetaFileW(1/3) CreateBitmap(1/7,u=6) CreateBrushIndirect(1/1,u=5) CreateCompatibleBitmap(1/7,u=6) CreateCompatibleDC(1/23,u=8) CreateDCW(1/2) CreateDIBSection(1/6,u=7) CreateDIBitmap(1/3,u=1) CreateEnhMetaFileW(1/1) CreateFontIndirectW(1/3,u=1) CreateFontW(1/1) CreateHalftonePalette(1/3,u=5) CreatePalette(1/9,u=6) CreatePatternBrush(1/1) CreatePen(1/1) CreatePenIndirect(1/1) CreatePolygonRgn(1/1) CreateRectRgn(1/10,u=2) CreateRectRgnIndirect(1/12,u=2) CreateSolidBrush(1/5,u=2) DeleteDC(1/41,u=6) DeleteEnhMetaFile(1/3) DeleteObject(1/80,u=8) Ellipse(1/1) EndDoc(1/1) EndPage(1/2) EndPath(1/1) EnumFontFamiliesExW*(1/1,u=1) EnumFontsW*(1/1) EqualRgn(1/3,u=2) ExcludeClipRect(1/8,u=2) ExtFloodFill(1/1) ExtSelectClipRgn(1/3,u=2) ExtTextOutW(2/6) FrameRgn(1/1) GetBitmapBits(1/1,u=1) GetBkColor(1/2) GetBrushOrgEx(1/2) GetClipBox(1/5,u=2) GetClipRgn(1/2,u=2) GetCurrentObject(1/1) GetCurrentPositionEx(1/1) GetDCOrgEx(1/1,u=2) GetDIBColorTable(1/3,u=5) GetDIBits(1/6,u=5) GetDeviceCaps(1/19,u=9) GetEnhMetaFileBits(1/1) GetEnhMetaFileDescriptionW(1/2) GetEnhMetaFileHeader(1/8) GetEnhMetaFilePaletteEntries(1/1) GetNearestPaletteIndex(1/1) GetObjectW(1/21,u=6) GetPaletteEntries(1/7,u=5) GetPixel(1/1) GetRandomRgn(2/2,u=2) GetRegionData(1/1,u=2) GetRgnBox(1/3,u=2) GetStockObject(1/5,u=2) GetStretchBltMode(1/1) GetSystemPaletteEntries(1/1,u=1) GetTextAlign(1/1) GetTextColor(1/2) GetTextExtentPoint32W(2/9) GetTextExtentPointW(1/2) GetTextMetricsW(1/9,u=3) GetWinMetaFileBits(1/1) GetWindowOrgEx(1/4,u=2) IntersectClipRect(1/7,u=2) LPtoDP(1/3,u=2) LineTo(1/2) MaskBlt(1/2) MoveToEx(1/2) OffsetRgn(1/9,u=2) PatBlt(1/4,u=5) Pie(1/1) PlayEnhMetaFile(1/2) PolyBezier(1/1) PolyBezierTo(1/1) Polygon(1/1) Polyline(1/1) RealizePalette(1/14,u=5) RectInRegion(1/2,u=2) RectVisible(1/1) Rectangle(1/1) ResizePalette(1/1) RestoreDC(1/18) RoundRect(1/1) SaveDC(1/9) SelectClipPath(1/1) SelectClipRgn(1/5,u=2) SelectObject(1/47,u=10) SelectPalette(1/22,u=5) SetAbortProc*(1/1) SetBkColor(1/24,u=5) SetBkMode(1/5) SetBrushOrgEx(1/3) SetDIBColorTable(1/3,u=7) SetDIBits(1/2) SetDIBitsToDevice(1/1,u=2) SetEnhMetaFileBits(1/1) SetMapMode(1/3) SetPixel(1/1) SetPixelV(1/4) SetROP2(1/1) SetRectRgn(1/3,u=2) SetStretchBltMode(1/7) SetTextAlign(1/1) SetTextColor(1/13,u=5) SetViewportOrgEx(1/1) SetWinMetaFileBits(1/1) SetWindowOrgEx(1/10,u=2) StartDocW(1/1) StartPage(1/2) StretchBlt(1/7,u=1) StretchDIBits(1/3) TextOutW(1/1) UnrealizeObject(1/1)
- `ole32.dll` 2: IsAccelerator(1/1) OleSetMenuDescriptor(1/1)
- `opengl32.dll` 2: wglGetProcAddress(1/1) wglUseFontBitmapsW(1/1)
- `shell32.dll` 6: DragAcceptFiles(1/2) DragFinish(1/1) DragQueryFileW(1/1) DragQueryPoint(1/1) SHAppBarMessage(1/1) Shell_NotifyIconW(1/1)
- `user32.dll` 191: ActivateKeyboardLayout(1/4) AdjustWindowRectEx(1/2) AttachThreadInput(1/2) BeginDeferWindowPos(1/3,u=2) BeginPaint(1/10,u=2) CallNextHookEx*(1/7) CallWindowProcW*(1/12) CheckMenuItem(1/1,u=2) ClientToScreen(1/7) CloseClipboard(1/3) CopyImage(1/1) CountClipboardFormats(1/1) CreateCaret(1/1) CreateIcon(1/1,u=1) CreateIconFromResource(1/1) CreateIconIndirect(1/1,u=5) CreateMenu(1/1) CreatePopupMenu(1/2) CreateWindowExW*(1/8,u=7) DefFrameProcW*(1/1) DefWindowProcW*(1/19) DeferWindowPos(1/3,u=2) DeleteMenu(1/2,u=1) DestroyCaret(1/2) DestroyCursor(1/3,u=5) DestroyIcon(1/1) DestroyMenu(1/3) DestroyWindow(1/8,u=6) DispatchMessageA*(1/1,u=2) DispatchMessageW*(2/10,u=2) DrawEdge(1/11) DrawFocusRect(1/3) DrawFrameControl(1/3) DrawIcon(1/2) DrawIconEx(1/2) DrawMenuBar(1/4,u=2) DrawTextExW(1/1) DrawTextW(2/17) EmptyClipboard(1/2) EnableMenuItem(1/2) EnableWindow(1/3,u=2) EndDeferWindowPos(1/3,u=2) EndPaint(1/15,u=2) EnumChildWindows*(1/3,u=1) EnumClipboardFormats(1/2) EnumDisplayMonitors*(1/2,u=4) EnumDisplaySettingsW(1/2,u=1) EnumThreadWindows*(1/5,u=2) EnumWindows*(1/4) FillRect(1/18,u=5) FindWindowExW(1/1) FindWindowW(1/6,u=2) FrameRect(1/5) GetActiveWindow(1/7,u=2) GetAncestor(1/2) GetAsyncKeyState(1/7) GetCapture(1/54,u=2) GetClassInfoW(1/8,u=7) GetClassLongPtrW(1/1) GetClassNameW(1/1) GetClientRect(1/10,u=2) GetClipCursor(1/2,u=1) GetClipboardData(1/7) GetCursor(1/3,u=2) GetCursorInfo(1/1,u=2) GetCursorPos(1/72,u=2) GetDC(1/36,u=10) GetDCEx(1/6) GetDesktopWindow(1/5) GetDlgItem(1/3) GetFocus(1/12,u=2) GetForegroundWindow(1/5) GetIconInfo(1/5,u=1) GetKeyState(1/12,u=2) GetKeyboardState(1/6) GetLastActivePopup(1/5,u=2) GetMenu(1/2) GetMenuItemCount(1/4) GetMenuItemID(1/2) GetMenuItemInfoW(1/1,u=2) GetMenuState(1/4) GetMenuStringW(1/1) GetMessagePos(1/2) GetMessageTime(1/1) GetMessageW(1/1) GetMonitorInfoW(1/6,u=3) GetParent(1/18,u=2) GetPropW(1/3,u=3) GetScrollInfo(1/5) GetScrollPos(1/2) GetSubMenu(1/5) GetSystemMenu(1/3,u=1) GetTopWindow(1/2) GetUpdateRect(1/2) GetUpdateRgn(1/2,u=2) GetWindow(2/10) GetWindowDC(1/7) GetWindowLongPtrW(1/46,u=6) GetWindowPlacement(2/16,u=2) GetWindowRect(1/27,u=2) GetWindowTextW(1/1) GetWindowThreadProcessId(2/11,u=2) HideCaret(1/2) InsertMenuItemW(1/1) InsertMenuW(1/1) InvalidateRect(2/15,u=2) InvalidateRgn(1/1,u=2) IsChild(1/4) IsClipboardFormatAvailable(1/1) IsDialogMessageA(1/1,u=2) IsDialogMessageW(1/1,u=2) IsIconic(1/8,u=2) IsWindow(1/6,u=2) IsWindowEnabled(1/11,u=2) IsWindowUnicode(1/4,u=2) IsWindowVisible(1/31,u=2) IsZoomed(1/2) KillTimer(1/4,u=2) LoadBitmapW(1/3) LoadCursorW(1/6,u=6) LoadIconW(1/6,u=2) LoadImageW(1/1,u=1) LoadKeyboardLayoutW(1/2) LockWindowUpdate(1/7) MapWindowPoints(1/17,u=2) MessageBoxA(1/1) MessageBoxW(2/7,u=6) MonitorFromPoint(1/1) MonitorFromRect(1/2) MonitorFromWindow(1/4,u=2) MsgWaitForMultipleObjects(1/3,u=2) MsgWaitForMultipleObjectsEx(1/1) NotifyWinEvent(1/5) OpenClipboard(1/2) PeekMessageW(1/18,u=2) PostMessageW*(1/122,u=6) PostQuitMessage(1/6,u=2) PostThreadMessageW(1/1) RedrawWindow(1/16,u=2) RegisterClassW*(1/8,u=7) ReleaseCapture(1/26,u=2) ReleaseDC(1/82,u=11) RemoveMenu(1/2) RemovePropW(1/3,u=1) ScreenToClient(1/2) ScrollDC(1/4) ScrollWindow(1/2) ScrollWindowEx(1/1,u=2) SendInput(1/1) SendMessageA*(1/1,u=2) SendMessageW*(3/160,u=3) SetActiveWindow(1/18,u=2) SetCapture(1/7) SetCaretPos(1/3) SetClassLongPtrW(1/3,u=1) SetClipboardData(1/5,u=3) SetCursor(1/31,u=2) SetCursorPos(1/10) SetFocus(1/28,u=2) SetForegroundWindow(1/4) SetKeyboardState(1/1) SetMenu(1/2) SetMenuItemInfoW(1/1,u=2) SetParent(1/2) SetPropW(1/4,u=1) SetScrollInfo(1/2) SetTimer*(1/13,u=2) SetWindowLongPtrW(1/30,u=7) SetWindowPlacement(2/6,u=2) SetWindowPos(1/46,u=2) SetWindowRgn(1/4) SetWindowTextA(1/1) SetWindowTextW(1/4) SetWindowsHookExW*(1/5,u=2) ShowCaret(1/3) ShowCursor(1/2) ShowOwnedPopups(1/2) ShowScrollBar(1/1) ShowWindow(1/18,u=2) ToAscii(1/1) ToUnicode(1/1) TrackPopupMenu(1/2) TranslateMDISysAccel(1/1,u=2) TranslateMessage(1/6,u=2) UnhookWindowsHookEx(1/5,u=2) UnregisterClassW(1/10,u=7) UpdateWindow(1/2) ValidateRect(1/3,u=2) ValidateRgn(1/1,u=2) WaitMessage(1/2,u=2) WindowFromPoint(1/4,u=2)

#### (c) ウィンドウ・メッセージ・GDI・コモンコントロール・入力 / 遅延 import（`jmp rax` 末尾）: API 11、thunk 11
- `shell32.dll` 1: SHOpenFolderAndSelectItems(1/1)
- `user32.dll` 10: CloseGestureInfoHandle(1/2) GetGestureInfo(1/1) GetPhysicalCursorPos(1/4,u=2) LogicalToPhysicalPoint(1/3) MessageBoxA(1/2,u=128) PhysicalToLogicalPoint(1/5) RegisterTouchHitTestingWindow(1/2) RegisterTouchWindow(1/1) SetGestureConfig(1/3) SetPhysicalCursorPos(1/11,u=2)

#### (d) COM/OLE / IAT 経由 jmp: API 21、thunk 21
- `ole32.dll` 17: CoCreateInstance(1/14,u=2) CoGetClassObject(1/1) CoGetInterfaceAndReleaseStream(1/2) CoMarshalInterThreadInterfaceInStream(1/2) CreateDataAdviseHolder(1/1) CreateILockBytesOnHGlobal(1/2) CreateStreamOnHGlobal(1/5) DoDragDrop(1/2) OleDraw(1/1) OleFlushClipboard(1/1) OleGetClipboard(1/3) OleIsCurrentClipboard(1/1) OleSetClipboard(1/5) RegisterDragDrop(1/5) ReleaseStgMedium*(1/18) RevokeDragDrop(1/6) StgCreateDocfileOnILockBytes(1/2)
- `oleacc.dll` 1: LresultFromObject(1/1)
- `oleaut32.dll` 1: GetActiveObject(1/1)
- `shell32.dll` 2: SHGetDesktopFolder(1/4) SHGetMalloc(1/2,u=1)

#### (d) COM/OLE / 遅延 import（`jmp rax` 末尾）: API 2、thunk 2
- `ole32.dll` 1: PropVariantClear*(1/9)
- `shell32.dll` 1: SHCreateItemFromParsingName(1/2)

#### (e) 25 名の仮想モジュール表に無い DLL（遅延 import のみ） / 遅延 import（`jmp rax` 末尾）: API 27、thunk 27
- `dwmapi.dll` 2: DwmExtendFrameIntoClientArea(1/1) DwmIsCompositionEnabled(1/1)
- `imm32.dll` 7: ImmAssociateContextEx(1/2) ImmGetContext(1/1) ImmGetConversionStatus(1/1) ImmIsIME(1/1) ImmReleaseContext(1/1) ImmSetConversionStatus(1/1) ImmSetOpenStatus(1/1)
- `msimg32.dll` 2: AlphaBlend(1/4) GradientFill(1/1)
- `propsys.dll` 5: PropVariantToBooleanWithDefault(1/1) PropVariantToFileTime(1/1) PropVariantToStringWithDefault(1/2) PropVariantToVariant(1/2) VariantToPropVariant(1/1)
- `rpcrt4.dll` 1: UuidCreate(1/1)
- `shcore.dll` 2: GetDpiForMonitor(1/1,u=2) GetProcessDpiAwareness(1/1,u=3)
- `uxtheme.dll` 5: BeginBufferedPaint(1/4) BufferedPaintInit(1/1,u=1) BufferedPaintSetAlpha(1/4) BufferedPaintUnInit(1/1) EndBufferedPaint(1/8)
- `windowscodecs.dll` 1: WICConvertBitmapSource(1/1)
- `wtsapi32.dll` 2: WTSRegisterSessionNotification(1/1,u=1) WTSUnRegisterSessionNotification(1/1)

## 4. 現在の停止点の先: DllMain/InitUnits の静的推定（推測を含む）

### 4.1 方法（実測のスクリプトと、その限界）

- 表は構造体 0x13768b0（`+0` = 件数 942、`+8` = 要素 0x13768e0。DLL のバイト列から読んだ。`index-0x412104.md` と一致）。f_4120c0 のループ（0x4120c0〜0x41214b、objdump）は、各要素の `+0` が 0 でなく、その先頭 8 バイトが 0 でないときに `call` する。**895 件が実行対象**（proc≠0 かつ先頭 8 バイト≠0。46 件は 0x13a9390（データ）で先頭 8 バイトが 0 のため飛ばされる、1 件は 0）。895 件はすべて閉包内（`units.tsv`）。
- 各 proc から、生成 C の**直接 call（と tail jmp）だけ**の BFS／DFS で到達する関数を集め、その中の未対応の項目（IAT thunk、delay-load thunk、`jmp:other`、未対応ニーモニック）を列挙した。
- **間接 call（`fle_dispatch(t)`、40,725 か所）は追えない**。閉包に間接 call を含まない 763 unit のうち 26 unit に間接 call が合計 72 か所ある（`extra_checks.log`）。それらの先で踏む API はここに出ない。
- **上限（過大見積もり）**: 到達できる関数をすべて実行するとみなす。分岐の向きは見ない。実際には通らない枝（エラー経路など）の API が混ざる。**実測の反証例**: unit 12（0x42d870）と 15（0x44a570）の閉包には `MessageBoxA`（delay）、`SysReAllocStringLen` 等が入っているが、実行は両方を通過して unit 16 に達した（`run-pdata-b.txt`）。
- 静的閉包の精度の目安（実測、`overapprox.log`）: 実行ログの先頭 256 エントリの異なる callee 115 のうち、113 は「エントリ＋ unit 0〜16 の直接閉包」（305 関数）に入り、2（0x41cf40、0x41d050）は外（間接 call かデータポインタ経由と見られる。**推測**）。実測の実行関数数は 140 で、静的閉包 305 の約半分。

### 4.2 結果（実測）

- 895 unit のうち、**直接閉包に未対応が 0 件の unit は 763**（85.3%）。うち 706 は閉包が 1 関数（`extra_checks.log`）。残り **132 unit** は何かしらの未対応に到達する。index ≥ 16（これから走る分）は 879 件中 129 件が未対応に到達する（`units_first_api.tsv`）。
- 現在の停止 unit は index 16（proc 0x454090、`GetNativeSystemInfo` の thunk は f_4236f0）。その閉包の未対応は: delay の GetLogicalProcessorInformation、GetNativeSystemInfo、MessageBoxA、`jmp:other`、oleaut32 の SysFreeString、SysReAllocStringLen、user32 の GetSystemMetrics、と VEX／Lea／Mov（対応済み）。
- エントリ f_1376820 の直接閉包は 42 関数で、未対応は 4 行だけ: 0x406e70（GetStdHandle。TABLE に行がある）、0x4071bf（delay MessageBoxA）、0x412752（longjmp 風）、0x41d14b（`mov rax, gs:[0x58]`、0032 で対応済み）。
- **unit ごとの「最初に要る未対応」（BFS、index ≥ 16、129 件）**: 

| 最初の未対応（BFS の深さ最小。同深さは呼び出し地点の VA 順） | unit 数 |
|---|---:|
| `jmp:other`（f_40d210 の表 jmp が 59 件。f_4126f0 の longjmp は BFS の先頭には出なかった） | 59 |
| delay `user32.dll!MessageBoxA` | 32 |
| oleaut32 `SysReAllocStringLen` | 6 |
| user32 `GetSystemMetrics` | 6 |
| oleaut32 `SysFreeString` | 5 |
| kernel32 `CreateEventW` | 4 |
| user32 `PostMessageW` | 3 |
| kernel32 `GetComputerNameExW` | 2 |
| 各 1: gdi32 GetStockObject、user32 GetKeyboardLayout、MessageBoxW、FindWindowW、EnumDisplaySettingsW、GetClipCursor、SystemParametersInfoW、GetDC、msacm32 acmMetrics、ole32 OleInitialize、delay kernel32 LocaleNameToLCID、delay GetLogicalProcessorInformation | 12 |

  全 unit の一覧は `units.tsv`（BFS／DFS の両方）、`units_first_api.tsv`。DFS（呼び出し順に近い。分岐は無視）では、113 unit が最初に delay `MessageBoxA` に当たる。

- 閉包に含む unit が多い項目（`api_unit_reach.tsv`、全 895 unit 中）: `jmp:other` 132、delay MessageBoxA 128、SysFreeString 85、delay GetLogicalProcessorInformation 84、SysReAllocStringLen 83、FormatMessageW 22、CreateThread 13、GetSystemMetrics 11、CreateEventW 11、ReleaseDC 11、GetDC 10、SelectObject 10。
- `jmp:other` の中身（`jmp_split.log`）: 132 unit のうち f_40d210（memset 風）のみ 4、f_4126f0（longjmp 風）のみ 41、両方 87。f_40d210 だけが原因で他に未対応の無い unit は 2。
- **index 順に走るとして、過去の unit が要らなかった新しい未対応が初めて現れる unit**（`seq_frontier.tsv`。過大見積もりの閉包で、24 unit）。先頭: 16（上記）、17（EnumCalendarInfoW、FormatMessageW、SysAllocStringLen）、41（CreateEventW）、95（GDI: CreatePalette、GetDeviceCaps、GetStockObject、GetSystemPaletteEntries、GetTextMetricsW、SelectObject、GetDC、LoadIconW、ReleaseDC）、114（uxtheme BufferedPaintInit、wtsapi32 WTSRegisterSessionNotification と GDI 多数）、134（Xgetbv）、158（SHGetFolderPathW、MessageBoxW）、170（acmMetrics）、235（SHGetMalloc）、236（EnumChildWindows、Get/Set/RemovePropW）、245（shcore GetProcessDpiAwareness、IsProcessDPIAware と GDI 多数）、246（EnumDisplaySettingsW）、265（delay GetSystemDefaultLocaleName）、272（GetClipCursor）、275（GetMonitorInfoW）、283（EnumFontFamiliesExW）、314（ResetEvent、SetEvent、WaitForSingleObject）、365（ResumeThread、DestroyWindow、GetWindowLongPtrW、PostMessageW）、492（delay LocaleNameToLCID、EnumSystemLocalesW）、604（SafeArray*、Variant*）、745（SetClipboardData）、830（GetComputerNameExW）、880（version.dll 3 API）、895（残りの GDI／リージョン、DirectSoundCreate、delay の物理カーソル等）。
- **貪欲法**（`greedy_units.tsv`、`greedy_units2.py`。「その項目を実装すると何 unit が未対応 0 になるか」。他の項目と共に塞がれている unit は数えない）: API だけ見ると、実装済みにする順に `jmp:other` +3、MessageBoxA(delay) +37、SysFreeString +2、delay GetLogicalProcessorInformation +2、SysReAllocStringLen +51、FormatMessageW +2、CreateEventW +3、…で 763 → 873 unit（16 項目）。その先は 1 項目 1 unit 程度で伸びが鈍い（895 unit すべてを未対応 0 にするまで 204 項目。命令も含めると 211 項目。`greedy_units.tsv` の末尾）。
- 提案する実装単位（§6）を積み上げた場合の「未対応 0 の unit 数」（`batches2.tsv`。VEX 等の命令は対応済みとして数える）: 

| 段 | 追加する単位 | 項目数 | 累計の未対応 0 unit（895 中） |
|---|---|---:|---:|
| 0 | （現状。0032 まで） | 0 | 763 |
| 1 | G1 tail jmp（単独では 0 項目） | 0 | 763 |
| 2 | G2 表 jmp（f_40d210） | 1 | 765 |
| 3 | K-delay（kernel32 の delay 5） | 5 | 766 |
| 4 | OA（oleaut32 の BSTR／Variant／SafeArray） | 11 | 766 |
| 5 | U-pure（user32 の環境値・表示） | 13 | 766 |
| 6 | K-pure（kernel32 の純粋・状態表なし） | 4 | 766 |
| 7 | OLE-mem（CoInitialize、CoTaskMemFree、OleInitialize） | 3 | 766 |
| 8 | K-sync（イベント／スレッド） | 7 | 766 |
| 9 | SHELL/VER | 7 | 766 |
| 10 | U-msgbox-dpi（MessageBoxA/W、IsProcessDPIAware） | 3 | **876** |
| 11 | EXT（shcore、uxtheme、wtsapi32 の 4） | 4 | 876 |
| 12 | ABSENT（DirectSoundCreate、acmMetrics、InternetGetConnectedState） | 3 | 877 |
| 13 | WIN（window／GDI の 141 項目） | 141 | 891 |
| 14 | COM（CoCreateInstance、SHGetMalloc） | 2 | 894 |
| 15 | INSN（Xgetbv） | 1 | 895 |

  読み方: 項目の大半は 1 つ 1 つでは unit を解放せず、**MessageBoxA の段（+110）で一度に解ける**（それまでの項目が揃っている場合）。各単位を単独で足しても 0〜+2 unit で、効果は積み上げでしか出ない（`batches2.log`）。これは過大見積もりの閉包に基づく（§4.1）。MessageBoxA は `f_4071e0`（呼び出し元は f_408400 と f_4127d0 の 2 つだけ）に集中している。f_4127d0 は `GetStdHandle(-11)` の後に文字列を書く関数で（objdump）、**エラー報告経路に見える**（**推測**）。もしそうなら実行時には踏まれず、MessageBoxA は急がなくてよい。
- 静的推定の限界の再掲: 間接 call、データポインタ、分岐の向き、ループ回数は見ていない。「次に実装すべき API の優先順」は、実測の停止点（unit 16、`GetNativeSystemInfo`）から順に、**①delay の `jmp rax`（W1）、②f_40d210 の表 jmp（W2）、③ BSTR（W4）の順**で、③以降は「その unit に当たったら止まる」の列挙として読むのがよい。

## 5. 実測のまとめ（数字だけ）

| 項目 | 値 |
|---|---:|
| 生成 C の `Jmp` 未対応 | 680（IAT 618、解決不能 62） |
| IAT thunk の DLL／API | 23／590（TABLE 行あり 27 thunk／23 API を除くと 591 thunk／567 API） |
| 現在の TABLE（173 行）で更に答える IAT thunk | 3（GetProcessAffinityMask、GlobalMemoryStatus、SetProcessAffinityMask） |
| delay-load thunk（57 関数、17 か所の `jmp rax`） | 57 API、9 DLL（外部 27 API）、TABLE 行あり 3 API |
| コールバックが要る API | 36 API／40 thunk |
| InitTable の実行対象 unit | 895（先頭 8 バイト 0 の 46 と終端 0 の 1 を除く） |
| 直接閉包に未対応が無い unit | 763（うち閉包 1 関数が 706） |
| エントリ＋全 unit の直接閉包 U | 6,297 関数（58,695 の 10.7%）。U 内の IAT thunk 196（現 TABLE 後）、delay thunk 13、x87 0 |

## 6. 実装単位の提案（優先順位つき）

各単位は「新規ファイルを主に作り、共有ファイルへの変更を最小の追記にする」形にした。**共有ファイル**: `lift/crates/fe-lift/src/imports_table.rs`（TABLE への行追加。全 API 単位が触る）、`lift/crates/fe-lift/src/lib.rs`（モジュール宣言と `Jmp` の分岐）、`lift/runtime/fle_modules.c`（外部 DLL の応答）、ビルドスクリプトの `EXTRA_RT`。難しさは**推測**。

### W0（前提、小）TABLE を行単位で衝突しない形にする

- `imports_table.rs` の TABLE を DLL／単位ごとのスライス（新規ファイル `imports_table/*.rs` または複数の `const` を連結）に分け、単位ごとに自分のファイルだけに追記できるようにする。以降の単位が同じファイルの同じ行域を触らなくなる。
- API 数 0。難しさ: 低。変更ファイル: `imports_table.rs`（構造の変更）。**他の単位の前に 1 人がやる**（並行編集の衝突を避ける）。

### W1（最優先）`jmp reg` の tail dispatch（遅延ロードヘルパの `jmp rax`）

- 対象: §1.1 の 57 行（17 か所の `jmp rax`）。`GetNativeSystemInfo`（停止 unit 16 の次の壁）以降の delay-load 57 API すべての前提（応答行だけでは通らない。§2.4）。
- 内容案: 解決不能な `jmp r64` を `fle_dispatch(R[reg]); return;` に落とす（ゲストの戻りアドレスはスタックに残す、`fle_imp_*` の規約と同じ）。対象は `ff e0`／`ff e1` の形で、switch の表 jmp（W2）とは分ける。
- 変更ファイル案: 新規 `lift/crates/fe-lift/src/tailjmp.rs`、`lib.rs` に 1 行の分岐。テスト: 新規。
- API 数 0（機構）。想定される難しさ: 低〜中。仕様確認は不要（x86 の意味論だけ）。注意: delay スロットは `jmp [slot]` が初期値の静的解決で `goto` になっている（f_4236f0: `goto L_4236e0`）ため、スロットがヘルパに書き換えられても生成 C は読み直さず、毎回ヘルパ経由になる（生成 C の読解。性能の問題で、正しさの問題ではない。**推測**）。

### W2 表を引く jmp（in-function の jump table）

- 対象: f_40d210（memset 風）の `jmp [r10+rdx*8+0x40]`（0x40d253。表 8 エントリ、rdx は −7〜0。`jmp_table_40d210.txt`）、switch 3 か所（0x5e5eb6、0x5e9ae8、0x5f1499。いずれも `movsxd; add; jmp`）。f_40d210 は **337 関数が直接 call**、91 unit が到達する（`jmp_split.log`）。
- 内容案: 表の位置と添字の範囲を手書き（seed 風の注釈ファイル）で与え、`switch` に落とす。範囲外は abort。
- 変更ファイル案: 新規 `lift/crates/fe-lift/src/jumptab.rs`（と注釈データ）、`lib.rs` に分岐 1 行。API 数 0。難しさ: 中（添字の範囲の根拠を objdump で確認する必要がある。f_40d210 は `cmp rdx,0x20; jg`、`sub rdx,8; jg` の後なので、rdx は −7〜0 と読める。表の 8 エントリはすべて f_40d210 内の ret 付きの断片を指す。いずれも objdump の読み）。

### W3（後回し）setjmp/longjmp 風の組（f_412680、f_4126f0）

- 対象: f_4126f0 の `jmp [rcx]`（0x412752）。呼び出し元は f_412770 のみ、その先 f_4128b0 を呼ぶのはエントリ f_1376820 を含む 5 関数。f_412680（保存側）はエントリの実行で呼ばれた（実測、entry 2）。
- 内容案: 非局所ジャンプ。生成 C の関数呼び出しスタックへの復帰が要る。C の `setjmp`/`longjmp` に写すか、「到達したら止める」の明示的な停止にするかの設計が要る。
- 難しさ: 高。ただし実行されるのは終了／エラー経路に見える（**推測**: 呼び出し元 f_4127d0 が前述のエラー報告関数）。実際に踏まれるまで保留でよい。

### W4 oleaut32／ole32 の割り当て系（BSTR、Variant、SafeArray、CoTaskMem、CoInitialize）

- API: oleaut32 の A/B 18（SysAllocStringLen、SysFreeString、SysReAllocStringLen、VariantInit、VariantClear\*、VariantCopy\*、VariantCopyInd\*、VariantChangeType、SafeArray* 9、GetErrorInfo）と ole32 の A/B 10（CoInitialize、CoUninitialize、OleInitialize、OleUninitialize、CoTaskMemAlloc、CoTaskMemFree、StringFromCLSID、ProgIDFromCLSID、CLSIDFromString、IsEqualGUID）。thunk は oleaut32 19、ole32 12。
- 根拠: SysFreeString は **85 unit** が閉包に含み、呼び出し元の thunk 側（f_412e00 の直接呼び出し元 194）。SysReAllocStringLen は 83 unit。静的推定の「最初に要る未対応」で上位（§4.2）。
- 変更ファイル案: 新規 `runtime/fle_oleaut.c`、`fle_oleaut.h`、TABLE 行（W0 後は自分のファイル）。難しさ: 中（BSTR の長さ接頭辞とアロケータの仕様確認、VARIANT の型表。VariantClear 等のコールバック\*は COM オブジェクトが無ければ起きない。**推測**）。

### W5 kernel32 の純粋・状態表なし

- API: FormatMessageW、GetComputerNameExW、GetComputerNameW、GetUserDefaultLCID、GetOEMCP、GetConsoleCP、GetConsoleOutputCP、GetCPInfoExW、CompareStringA、CompareFileTime、IsDebuggerPresent、TzSpecificLocalTimeToSystemTime、FreeResource、EnumCalendarInfoW\*、EnumSystemLocalesW\*（16。GlobalMemoryStatus は現 TABLE で対応済み）と、delay の GetSystemDefaultLocaleName、LocaleNameToLCID、GetTimeZoneInformationForYear、GetUserPreferredUILanguages、NormalizeString（5。W1 が前提）。
- 根拠: FormatMessageW 22 unit、GetComputerNameExW（呼び出し元 25、4 unit）。\* の 2 つはゲストのコールバックを呼ぶので、W10 のコールバック機構が前提（それまでは対象外にできる）。
- 変更ファイル案: 新規 `runtime/fle_k32misc.c`。難しさ: 低〜中（FormatMessageW はメッセージ表が要り、仕様確認が重い。まず `FORMAT_MESSAGE_FROM_SYSTEM` を使う呼び出しの有無を調べる）。

### W6 kernel32 の同期・スレッド・プロセス・ファイル状態

- API: 42（IAT）＋ delay 3。主に CreateEventW、SetEvent、ResetEvent、WaitForSingleObject、WaitForMultipleObjects(Ex)、CreateThread\*、ResumeThread、SuspendThread、ExitThread、GetExitCodeThread、GetThreadPriority、SignalObjectAndWait、HeapCreate/HeapDestroy、TlsFree、GetFileType、SetEndOfFile、GetFileInformationByHandle、GetDiskFreeSpaceW、GetTempFileNameW、Get/WritePrivateProfileStringW、タイマーキュー 3、OpenProcess、CreateProcessW、…（`names_block.md` の (b)）。
- 根拠: CreateEventW（11 unit、最初の未対応が 4 unit）、ResetEvent 8、SetEvent 4、ResumeThread 6、CreateThread 13。
- 変更ファイル案: 新規 `runtime/fle_sync.c`（イベント、待機）、`fle_thread.c`（スレッド）、`fle_file2.c`（残りのファイル）に分割可能。難しさ: 中〜高。CreateThread\* は仮想スレッドの設計（協調的に同期実行するか、実スレッドか）が要り、現在の「単一スレッドのゲスト」前提を変える（決定が要る）。イベントと待機は、単一スレッドなら単純な状態機械で足りる（**推測**）。

### W7 user32 の「ウィンドウ無し」環境値と表示

- API: GetSystemMetrics、GetSysColor、SystemParametersInfoW、GetKeyboardLayout 等 4、MapVirtualKeyW、GetKeyNameTextW、GetDoubleClickTime、GetMessageExtraInfo、MessageBeep、EqualRect/PtInRect/InflateRect/SubtractRect、GetGuiResources、CharNextExA、CharToOemBuffW（A: 18）、GetClipCursor、EnumDisplaySettingsW、GetMonitorInfoW、MonitorFrom*、EnumDisplayMonitors\*、GetCursorPos、GetCursorInfo、GetDesktopWindow（c の一部）と delay の DPI 5 と shcore の 2。
- 根拠: GetSystemMetrics（呼び出し元 35、11 unit）、SystemParametersInfoW（呼び出し元 19、9 unit）。
- 変更ファイル案: 新規 `runtime/fle_user_env.c`。難しさ: 中。**決定が要る**: 仮想ディスプレイの寸法、DPI、システム色、メトリクスの値（環境の主張）。どの値を返すか PM の決定が先。

### W8 「デバイス／サービス無し」型 61 API（wininet 15、msacm32 14、winmm 13、crypt32 7、winspool 5、mpr 4、dsound 2、wintrust 1 ほか）

- 根拠: 閉包に含む unit は少ない（DirectSoundCreate 2、InternetGetConnectedState 2、acmMetrics 1）。
- 変更ファイル案: 新規 `runtime/fle_absent.c`（API ごとの失敗コードの表）。難しさ: 低（1 件ずつ）が、API ごとに仕様確認（失敗時の戻り値と last error）が要る。**PM の決定**（0029 の「外部 DLL」と同じ「環境が何を持たないか」の主張）。優先は低い。

### W9 shell32／version／shfolder／imagehlp／shlwapi／crt

- API: version.dll 3（GetFileVersionInfoSizeW、GetFileVersionInfoW、VerQueryValueW。3 unit）、shell32 の A/B 14（SHGetFolderPathW 4 unit、ShellExecuteW、ShellExecuteExW、PathCleanupSpec、SHFileOperationW ほか）、shfolder SHGetFolderPathW、imagehlp の 2、shlwapi の 1、`memset`（crt の 1。6 関数が呼ぶ）。
- 変更ファイル案: 新規 `runtime/fle_shell.c`、`fle_version.c`。難しさ: 中（version.dll は仮想ファイルのバージョン資源の解釈が要る。SHGetFolderPathW は仮想ファイルシステムの標準フォルダの表が要る）。

### W10（設計が先）user32／gdi32／comctl32 の実体と、コールバック機構

- API: (c) の IAT 356（user32 191、gdi32 125、comctl32 30、shell32 6、ole32 2、opengl32 2）＋ delay 11。
- 先に要るのは **ゲスト関数の呼び返し機構**（C から `f_XXXX` を Windows x64 ABI で呼ぶ。引数を R[1]、R[2]、R[8]、R[9] とスタックに置き、戻りを R[0] から取る）。\* の 36 API（CreateWindowExW、RegisterClassW、SendMessageW、DispatchMessageW、SetTimer、SetWindowsHookExW、EnumWindows、CreateThread ほか）が前提とする。(d) の COM もこれを使う。
- 次に要るのは **仮想 DC／GDI オブジェクト表**（GetDC、CreateCompatibleDC、SelectObject、GetDeviceCaps、GetStockObject、DeleteObject、CreateBitmap、CreateDIBSection、パレット）。静的推定で最初に出る GDI は unit 95、114、245 で、約 45 API。
- 変更ファイル案: 新規 `runtime/fle_callback.c`、`fle_gdi.c`、`fle_win.c`。難しさ: 高（実体設計）。ウィンドウ・メッセージ（CreateWindowExW、DefWindowProcW、SendMessageW、PostMessageW、GetMessageW ほか）はさらに設計が要る。
- 現実的な範囲の目安（**推測**）: DC／ビットマップ表と GDI の描画なし版（描画は no-op、状態だけ保持）から始め、描画の出力が要る段階で再評価する。

### W11 COM/OLE（d）

- API: IAT 21、delay 2（CoCreateInstance 14 呼び出し元、2 unit、ReleaseStgMedium 18、RegisterDragDrop 5、OleSetClipboard 5、CreateStreamOnHGlobal 5、…）。
- 前提: W10 のコールバック機構（IUnknown::Release 等の vtable 呼び出し）。難しさ: 高。静的推定では 2 unit にしか当たらない。優先は低い。

### W12 外部 DLL（e）: delay-load の 27 API（9 DLL）

- 状況: 25 名の仮想モジュール表に無い DLL（dwmapi、imm32、msimg32、propsys、rpcrt4、shcore、uxtheme、windowscodecs、wtsapi32）。遅延ヘルパは `LoadLibrary` と `GetProcAddress` を呼び、NULL なら `RaiseException` する（実測、停止点 0x41cfa0 の機構）。**推測**: ゲストが先に利用可否を確認する経路（`f_4099c0` のような NULL 許容）を持つ API だけは「無い」で済むが、遅延ヘルパを直接踏む経路は答えが無いと例外になる。
- 静的推定で当たる項目: shcore の GetProcessDpiAwareness（3 unit）、GetDpiForMonitor（2 unit）、uxtheme の BufferedPaintInit（1 unit）、wtsapi32 の WTSRegisterSessionNotification（1 unit）。
- 変更ファイル案: `runtime/fle_modules.c` の仮想モジュール表への追加（末尾のみ。決定 0015）と応答。**PM の決定が先**（モジュールを「存在する」にするか）。難しさ: 低〜中。

### 優先順位（まとめ。推測を含む）

1. **W1**（`jmp reg` tail）と、別 subagent が対応中の GetNativeSystemInfo 等（0034）をセットで。W1 が無いと応答行が効かない。
2. **W2**（f_40d210 の表 jmp。337 関数が呼ぶ、91 unit が到達）。f_40d210 は今回の実行で 1 回呼ばれて通過した（実測、`run-pdata-b.txt` の entry 41、guest-return 0x409b56）が、表 jmp は呼び出しの引数（サイズ 1〜8）次第なので、いずれ踏まれる（**推測**）。W0 は W1 と並行して 1 人。
3. **W4**（BSTR 系）、**W5**（FormatMessageW 等）。静的推定で最初に出る項目の上位。
4. **W6** の同期（CreateEventW 他）と **W7**（環境値。決定が要る）。
5. **W10** の設計（コールバック機構と DC 表）。unit 95 以降（約 45 の GDI／user32 API）に先立って。
6. W9、W3、W8、W11、W12 は実際に踏まれた時点で。
- 各単位の API 数（`names_block.md` の分類から手で拾った概数。スクリプトでは集計していない。W7 と W10 は user32 の一部の置き場所で境界が動く）: W4 28、W5 20、W6 約 45（kernel32 の B 40＋delay 3＋advapi32 2）、W7 約 30、W8 61、W9 約 22、W10 約 350〜360、W11 23、W12 27。

## 7. 推測と実測の区別（要点）

- 実測: 生成 C の未対応行（2,991。Jmp 680、IAT 618、解決不能 62）、DLL ごとの thunk 数、57 の delay-load と 17 か所の `jmp rax`、f_40d210／f_4126f0 の表と直接呼び出し元数、InitTable の 895 件と各 unit の直接閉包（U = 6,297 関数）、実行ログとの突き合わせ（静的閉包 305 に対し実行 140 関数）、現 TABLE との差。
- 推測: API の分類（名前からの判断）、CB（コールバック）の判定、MessageBoxA がエラー経路であること、longjmp 風が終了／エラー経路であること、各単位の難しさ、次の停止点が `jmp rax`（0x42316f）になること、W6 のスレッド設計、W7／W8／W12 の値の決定の要否。
- 未確認: 再生成した C での挙動（生成 C は古い）。間接 call の先。実行順序。仕様（Microsoft Learn 等）は読んでいない。

## 8. 制約の遵守

`rm`、`>` による既存ファイルの上書き、git 操作、`/tmp`、sleep／timeout は使っていない。TMPDIR は `lift/out-inv/tmp`。権限拒否はなし。ソース、runtime、`imports_table.rs`、`lift/out-pdata/` は読み取りのみ。外部通信なし。2 GB 級の中間ファイルなし（`funcs.json` 9.3 MB、`lift/out-inv/tmp/text.dis` は DLL 全体の objdump 出力 270 MB、残りは約 10 MB）。`tmp/text.dis` は削除禁止のため残した。

## 9. 数値の根拠（`lift/out-inv/`、すべて新規）

実行順（`lift/out-inv` で。入力は `lift/out-pdata/out/after-pdata-b-part-*.c`、DLL、`imports_table.rs`、`out-idx/table-elements.txt`、`out-pdata/run-pdata-b.txt`）:

1. `python3 parse_c.py funcs.json`（`parse_c.log`）: 16 ファイル、3,759,382 行、58,695 関数、`fle_unsupported` 2,991、直接 call の辺 226,187、`fle_dispatch` 40,725。
2. `python3 jmp_inventory.py`（`jmp_inventory.log`、`jmp_thunks.tsv`、`jmp_other.tsv`、`api_callers.tsv`）: 680 = IAT 618 + その他 62。
3. `python3 delay_map.py`（`delay_map.log`、`delay_thunks.tsv`、`jmp_other_class.tsv`）。
4. `python3 class_report.py`（`class_report.log`、`class_summary.txt`、`class_apis.tsv`）と `apiclass.py`（分類表。推測）。
5. `python3 lists.py`、`names_block.py`（`class_lists.txt`、`dll_table.txt`、`names_block.md`）、`final_counts.py`（`final_counts.log`）。
6. `python3 bfs_units.py`（`bfs_units.log`、`units.tsv`、`units_first_api.tsv`、`api_unit_reach.tsv`）、`greedy_units2.py`（`greedy_units2.log`、`unit_blockers.tsv`、`greedy_units.tsv`。`greedy_units.py` は放棄した下書き）、`seq_units.py`（`seq_units.log`、`seq_frontier.tsv`、`f_class.txt`）、`extra_checks.py`、`overapprox.py`、`jmp_split.py`（`jmp_table_40d210.txt`）、`batches.py`、`batches2.py`（`batches2.tsv`、`batch_members.tsv`）、`recount_insn.py`（`recount_now.txt`、`insn_table.tsv`）。
7. 手作業の確認（objdump -d -M intel）: 0x40d210〜0x40d2d1、0x4126f0〜0x412754、0x423120〜0x423171、0x4127d0〜0x41282b、0x41d110〜0x41d15e、0x409c80〜0x409e00、0x6399e0〜0x639a03。

再実行する場合、出力は新規ファイルのみなので、別ディレクトリで実行するか、古い出力を退避する必要がある（削除は禁止のため）。

