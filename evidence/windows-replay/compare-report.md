# compare-report

observed: `observed.tsv`, cases: `cases.tsv`

## Environment of the observed run
- replay-version: 1
- tag: plain
- utc: 2026-10-29T23:08:39Z
- compiler: MSVC 1944
- RtlGetVersion: 10.0.26300 platform=2 sp=0.0 suite=0x100 product=1
- native-arch: 9
- ACP: 932
- OEMCP: 932
- mask-private: 1

## Totals

- cases in cases.tsv: 2144 (run: 2144, not run: 0); with a runtime answer to compare (trace_n>0): 569
- row status: ok=2127, skip=17
- verdicts: match=352, mismatch=161, reference=55, real-only=1559, skipped=17
- match / mismatch / reference are counted per case; one case stands for one distinct argument shape (not for every call). trace calls weigh in the ranking below

## Per API

| API | class | match | mismatch | reference | real-only | skipped |
|---|---|--:|--:|--:|--:|--:|
| @box | H | 0 | 0 | 0 | 2 | 0 |
| @bstr | H | 0 | 0 | 0 | 4 | 0 |
| @bstrp | H | 0 | 0 | 0 | 1 | 0 |
| @eq | H | 0 | 0 | 0 | 5 | 0 |
| @peek | H | 0 | 0 | 0 | 5 | 0 |
| @peekp | H | 0 | 0 | 0 | 5 | 0 |
| @poke | H | 0 | 0 | 0 | 2 | 0 |
| @selfcode | H | 0 | 0 | 0 | 1 | 0 |
| AcquireSRWLockExclusive | M | 1 | 0 | 0 | 0 | 0 |
| CharLowerBuffW | M | 6 | 0 | 0 | 4 | 0 |
| CharLowerW | M | 1 | 0 | 0 | 2 | 0 |
| CharNextW | M | 1 | 0 | 0 | 3 | 0 |
| CharUpperBuffW | M | 2 | 0 | 0 | 6 | 0 |
| CompareStringW | M | 7 | 0 | 0 | 60 | 0 |
| CreateBitmap | M | 1 | 0 | 0 | 2 | 0 |
| CreateBrushIndirect | M | 1 | 0 | 0 | 1 | 0 |
| CreateCompatibleDC | M | 4 | 0 | 0 | 1 | 0 |
| CreateDIBSection | M | 1 | 0 | 0 | 1 | 0 |
| CreateFontIndirectW | M | 1 | 0 | 0 | 1 | 0 |
| CreateIconIndirect | M | 1 | 0 | 0 | 0 | 0 |
| CreatePalette | M | 1 | 0 | 0 | 2 | 0 |
| CreatePenIndirect | M | 1 | 0 | 0 | 0 | 0 |
| DecodePointer | M | 1 | 0 | 0 | 2 | 0 |
| DeleteCriticalSection | M | 1 | 0 | 0 | 1 | 0 |
| DeleteDC | M | 4 | 0 | 0 | 1 | 0 |
| DeleteObject | M | 3 | 0 | 0 | 7 | 1 |
| DestroyCursor | M | 13 | 0 | 0 | 0 | 0 |
| DestroyIcon | H | 0 | 0 | 0 | 1 | 0 |
| EncodePointer | M | 2 | 0 | 0 | 0 | 0 |
| EnterCriticalSection | M | 1 | 0 | 0 | 1 | 0 |
| EnumDisplayMonitors | R | 1 | 0 | 0 | 0 | 0 |
| EnumDisplaySettingsW | R | 0 | 0 | 1 | 0 | 0 |
| EnumFontFamiliesExW | R | 1 | 0 | 0 | 7 | 0 |
| FindFirstFileW | M | 0 | 12 | 0 | 2 | 0 |
| FlsAlloc | M | 0 | 1 | 0 | 0 | 0 |
| FlsFree | H | 0 | 0 | 0 | 1 | 0 |
| FlsGetValue | M | 1 | 0 | 0 | 2 | 0 |
| FlsSetValue | M | 0 | 0 | 0 | 1 | 0 |
| FreeEnvironmentStringsW | M | 1 | 0 | 0 | 0 | 0 |
| FreeLibrary | M | 0 | 0 | 0 | 2 | 1 |
| GdiFlush | M | 1 | 0 | 0 | 0 | 0 |
| GetACP | L | 0 | 0 | 1 | 0 | 0 |
| GetBkColor | H | 0 | 0 | 0 | 1 | 0 |
| GetBkMode | H | 0 | 0 | 0 | 1 | 0 |
| GetCPInfo | M | 1 | 0 | 1 | 18 | 0 |
| GetClassInfoW | M | 0 | 1 | 0 | 1 | 0 |
| GetClipCursor | R | 0 | 0 | 1 | 0 | 0 |
| GetClipboardFormatNameW | M | 0 | 1 | 0 | 1 | 0 |
| GetCommandLineA | R | 1 | 0 | 0 | 0 | 0 |
| GetCommandLineW | R | 1 | 0 | 0 | 0 | 0 |
| GetComputerNameExW | R | 1 | 0 | 0 | 11 | 0 |
| GetCurrentObject | H | 0 | 0 | 0 | 5 | 0 |
| GetCurrentPositionEx | M | 1 | 0 | 0 | 0 | 0 |
| GetCurrentProcess | M | 1 | 0 | 0 | 0 | 0 |
| GetCurrentProcessId | R | 0 | 0 | 1 | 0 | 0 |
| GetCurrentThread | M | 1 | 0 | 0 | 0 | 0 |
| GetCurrentThreadId | R | 0 | 0 | 1 | 0 | 0 |
| GetDC | R | 2 | 0 | 0 | 0 | 0 |
| GetDIBColorTable | M | 0 | 1 | 0 | 0 | 0 |
| GetDeviceCaps | R | 4 | 0 | 0 | 238 | 0 |
| GetEnvironmentStringsW | R | 1 | 0 | 0 | 0 | 0 |
| GetEnvironmentVariableW | R | 0 | 0 | 2 | 9 | 0 |
| GetErrorMode | H | 0 | 0 | 0 | 1 | 0 |
| GetFileAttributesExW | M | 0 | 1 | 0 | 1 | 0 |
| GetFileAttributesW | M | 15 | 37 | 0 | 21 | 0 |
| GetFileVersionInfoSizeW | R | 1 | 0 | 0 | 1 | 0 |
| GetKeyboardLayout | R | 1 | 0 | 0 | 0 | 0 |
| GetLastError | M | 0 | 0 | 0 | 1 | 0 |
| GetLocalTime | R | 1 | 0 | 0 | 0 | 0 |
| GetLocaleInfoW | M | 57 | 1 | 2 | 460 | 0 |
| GetLogicalProcessorInformation | R | 1 | 0 | 1 | 0 | 0 |
| GetMapMode | H | 0 | 0 | 0 | 1 | 0 |
| GetModuleFileNameA | R | 0 | 0 | 1 | 1 | 0 |
| GetModuleFileNameW | R | 0 | 0 | 6 | 10 | 0 |
| GetModuleHandleA | R | 3 | 0 | 0 | 1 | 1 |
| GetModuleHandleW | R | 8 | 0 | 4 | 10 | 1 |
| GetMonitorInfoW | R | 0 | 0 | 1 | 0 | 0 |
| GetNativeSystemInfo | M | 0 | 0 | 1 | 0 | 0 |
| GetOEMCP | H | 0 | 0 | 0 | 1 | 0 |
| GetObjectType | H | 0 | 0 | 0 | 4 | 0 |
| GetObjectW | M | 2 | 0 | 0 | 4 | 0 |
| GetPolyFillMode | H | 0 | 0 | 0 | 1 | 0 |
| GetProcAddress | M | 25 | 89 | 0 | 0 | 0 |
| GetProcessDpiAwareness | R | 1 | 0 | 0 | 1 | 0 |
| GetProcessHeap | R | 1 | 0 | 0 | 0 | 0 |
| GetROP2 | H | 0 | 0 | 0 | 1 | 0 |
| GetStartupInfoA | R | 0 | 0 | 1 | 0 | 0 |
| GetStartupInfoW | R | 0 | 0 | 1 | 0 | 0 |
| GetStdHandle | R | 0 | 0 | 3 | 1 | 0 |
| GetStockObject | M | 3 | 0 | 0 | 19 | 0 |
| GetStretchBltMode | H | 0 | 0 | 0 | 1 | 0 |
| GetStringTypeW | M | 2 | 0 | 0 | 6 | 0 |
| GetSysColor | R | 2 | 0 | 0 | 30 | 0 |
| GetSystemDefaultLCID | H | 0 | 0 | 0 | 1 | 0 |
| GetSystemDefaultLocaleName | R | 0 | 0 | 1 | 1 | 0 |
| GetSystemInfo | M | 0 | 0 | 1 | 0 | 0 |
| GetSystemMetrics | R | 7 | 0 | 3 | 138 | 0 |
| GetSystemTimeAsFileTime | R | 1 | 0 | 0 | 0 | 0 |
| GetTempPathW | R | 0 | 0 | 1 | 1 | 0 |
| GetTextColor | H | 0 | 0 | 0 | 1 | 0 |
| GetThreadLocale | R | 0 | 0 | 1 | 3 | 0 |
| GetThreadPreferredUILanguages | L | 0 | 0 | 2 | 0 | 0 |
| GetThreadUILanguage | R | 0 | 0 | 1 | 0 | 0 |
| GetTickCount | R | 0 | 0 | 1 | 0 | 0 |
| GetTimeZoneInformation | R | 0 | 0 | 1 | 0 | 0 |
| GetUserDefaultLCID | H | 0 | 0 | 0 | 1 | 0 |
| GetUserDefaultLocaleName | H | 0 | 0 | 0 | 1 | 0 |
| GetUserDefaultUILanguage | R | 0 | 0 | 1 | 0 | 0 |
| GetVersion | M | 0 | 1 | 0 | 0 | 0 |
| GetVersionExA | M | 0 | 1 | 0 | 0 | 0 |
| GetVersionExW | M | 0 | 1 | 0 | 1 | 0 |
| HeapAlloc | M | 8 | 0 | 0 | 4 | 0 |
| HeapCreate | M | 1 | 0 | 0 | 0 | 0 |
| HeapFree | M | 0 | 0 | 0 | 3 | 0 |
| HeapReAlloc | M | 0 | 0 | 0 | 4 | 0 |
| HeapSetInformation | M | 0 | 0 | 0 | 1 | 0 |
| HeapSize | M | 0 | 0 | 0 | 12 | 0 |
| InitializeCriticalSection | M | 1 | 0 | 0 | 0 | 0 |
| InitializeCriticalSectionAndSpinCount | M | 1 | 0 | 0 | 0 | 0 |
| InitializeSListHead | M | 1 | 0 | 0 | 0 | 0 |
| IntersectRect | M | 1 | 0 | 0 | 6 | 0 |
| IsValidCodePage | M | 1 | 0 | 0 | 19 | 0 |
| IsValidLocale | M | 2 | 0 | 0 | 48 | 0 |
| LCMapStringW | M | 7 | 0 | 0 | 18 | 0 |
| LeaveCriticalSection | M | 1 | 0 | 0 | 1 | 0 |
| LoadCursorW | M | 1 | 0 | 0 | 0 | 0 |
| LoadIconW | M | 1 | 1 | 0 | 0 | 0 |
| LoadImageW | M | 13 | 0 | 0 | 0 | 0 |
| LoadLibraryA | M | 2 | 0 | 0 | 0 | 0 |
| LoadLibraryExW | M | 4 | 0 | 0 | 0 | 0 |
| LoadLibraryW | M | 8 | 2 | 0 | 14 | 0 |
| LoadStringW | M | 0 | 0 | 0 | 2 | 0 |
| LocalAlloc | M | 0 | 0 | 0 | 4 | 0 |
| LocalFree | H | 0 | 0 | 0 | 1 | 0 |
| MonitorFromPoint | H | 0 | 0 | 0 | 1 | 0 |
| MoveToEx | M | 1 | 0 | 0 | 0 | 0 |
| MulDiv | M | 1 | 0 | 0 | 12 | 0 |
| MultiByteToWideChar | M | 14 | 0 | 0 | 32 | 0 |
| OleInitialize | M | 1 | 1 | 0 | 0 | 0 |
| OleUninitialize | H | 0 | 0 | 0 | 2 | 0 |
| PatBlt | M | 1 | 0 | 0 | 2 | 0 |
| QueryPerformanceCounter | R | 0 | 0 | 1 | 0 | 0 |
| QueryPerformanceFrequency | R | 1 | 0 | 0 | 0 | 0 |
| RegCloseKey | M | 0 | 0 | 0 | 6 | 13 |
| RegOpenKeyExA | M | 0 | 1 | 0 | 0 | 0 |
| RegOpenKeyExW | M | 14 | 3 | 0 | 1 | 0 |
| RegQueryInfoKeyW | R | 0 | 0 | 0 | 1 | 0 |
| RegQueryValueExW | R | 0 | 0 | 5 | 13 | 0 |
| ReleaseDC | M | 2 | 0 | 0 | 0 | 0 |
| ReleaseSRWLockExclusive | M | 1 | 0 | 0 | 0 | 0 |
| RtlGetVersion | H | 0 | 0 | 0 | 1 | 0 |
| SHGetFolderPathW | R | 0 | 0 | 2 | 22 | 0 |
| SHGetMalloc | M | 1 | 0 | 0 | 0 | 0 |
| SelectObject | M | 6 | 0 | 0 | 4 | 0 |
| SelectPalette | M | 1 | 0 | 0 | 2 | 0 |
| SetBkColor | M | 1 | 0 | 0 | 2 | 0 |
| SetBkMode | M | 1 | 0 | 0 | 2 | 0 |
| SetDllDirectoryW | M | 1 | 0 | 0 | 1 | 0 |
| SetErrorMode | M | 1 | 1 | 0 | 1 | 0 |
| SetHandleCount | M | 1 | 0 | 0 | 0 | 0 |
| SetLastError | M | 2 | 0 | 0 | 1 | 0 |
| SetROP2 | M | 1 | 0 | 0 | 2 | 0 |
| SetStretchBltMode | M | 2 | 0 | 0 | 0 | 0 |
| SetTextColor | M | 1 | 0 | 0 | 2 | 0 |
| SetThreadErrorMode | M | 2 | 0 | 0 | 0 | 0 |
| SetThreadLocale | M | 1 | 0 | 0 | 1 | 0 |
| StretchDIBits | M | 0 | 1 | 0 | 0 | 0 |
| SysAllocStringLen | M | 2 | 0 | 0 | 2 | 0 |
| SysFreeString | M | 1 | 0 | 0 | 2 | 0 |
| SysReAllocStringLen | M | 1 | 0 | 0 | 0 | 0 |
| SystemParametersInfoW | R | 3 | 0 | 5 | 13 | 0 |
| TlsAlloc | M | 0 | 1 | 0 | 0 | 0 |
| TlsFree | H | 0 | 0 | 0 | 1 | 0 |
| TlsGetValue | M | 1 | 0 | 0 | 2 | 0 |
| TlsSetValue | M | 0 | 0 | 0 | 2 | 0 |
| UnrealizeObject | M | 1 | 0 | 0 | 0 | 0 |
| VerSetConditionMask | M | 2 | 2 | 0 | 73 | 0 |
| VerifyVersionInfoW | M | 2 | 0 | 0 | 12 | 0 |
| VirtualAlloc | M | 4 | 0 | 0 | 0 | 0 |
| VirtualProtect | M | 0 | 0 | 0 | 1 | 0 |
| WideCharToMultiByte | M | 20 | 0 | 0 | 57 | 0 |
| acmMetrics | M | 0 | 1 | 0 | 0 | 0 |

## Top 20 mismatches (spec fields; ranked by the number of calls the trace made with this argument shape)

| # | calls | API | args | difference |
|--:|--:|---|---|---|
| 1 | 34 | GetDIBColorTable `C02031` | `$mdc2/d:0x0/d:0x100/O:1024` | last error: runtime=unchanged(0)×34 real=set 6 |
| 2 | 24 | StretchDIBits `C02023` | `$mdc1/d:0x0/d:0x0/d:0x20/d:0x20/d:0x0/d:0x0/d:0x20/d:0x20/Z:4096:0=33221100665544ff/Z:44:0` | return: runtime=0x20 real=0x0; last error: runtime=unchanged(0)×24 real=set 87 |
| 3 | 23 | SetErrorMode `C00630` | `d:0x8000` | return: runtime=0x0/0x8000 real=0x8001 |
| 4 | 20 | GetFileAttributesW `C01364` | `W:C:\Windows\Fonts\OpenSans-Regular.ttf` | last error: runtime=unchanged(3)×15,set 3×5 real=set 2 |
| 5 | 10 | GetVersionExW `C00901` | `Z:284:0=1c010000` | output: OSVERSIONINFO.major: runtime=10 real=6; OSVERSIONINFO.minor: runtime=0 real=2 |
| 6 | 9 | RegOpenKeyExW `C01460` | `i:0xffffffff80000002/W:SOFTWARE\Microsoft\Windows NT\CurrentVersion\FontSubstitutes/d:0x0/` | return: runtime=0x2 real=0x0 |
| 7 | 8 | GetVersion `C00900` | `` | return: runtime=0x4a65000a real=0x23f00206 |
| 8 | 6 | TlsAlloc `C01290` | `` | return: runtime=0x0/0x1/0x2/0x3/0x4/0x5 real=0x9 |
| 9 | 6 | RegOpenKeyExW `C01468` | `i:0xffffffff80000001/W:Software\Wow6432Node/d:0x0/d:0x20019/O:8` | return: runtime=0x2 real=0x0 |
| 10 | 6 | RegOpenKeyExW `C01472` | `i:0xffffffff80000001/W:SOFTWARE\Microsoft\Windows\Shell\Associations\UrlAssociations\https` | return: runtime=0x2 real=0x0 |
| 11 | 5 | GetFileAttributesW `C01359` | `W:C:\Windows\Fonts\tahoma.ttf` | return: runtime=0xffffffff real=0x20 |
| 12 | 5 | GetFileAttributesW `C01377` | `W:C:\Program Files\Image-Line\FL Studio 21\Shared\Artwork\Fonts\OpenSans-Regular.ttf` | last error: runtime=set 2×5 real=set 3 |
| 13 | 4 | GetLocaleInfoW `C00004` | `d:0x409/d:0x100b/O:512/d:0x100` | output: out2: runtime="0" real="2" |
| 14 | 4 | GetProcAddress `C01132` | `M:api-ms-win-core-synch-l1-2-0/A:InitializeCriticalSectionEx` | return: runtime=0x0 real=OPAQUE |
| 15 | 4 | GetFileAttributesW `C01366` | `W:C:\Windows\Fonts\OpenSans-CondLight.ttf` | last error: runtime=unchanged(3)×3,set 3×1 real=set 2 |
| 16 | 4 | GetFileAttributesW `C01367` | `W:C:\Windows\Fonts\lucon.ttf` | return: runtime=0xffffffff real=0x20 |
| 17 | 4 | GetFileAttributesW `C01369` | `W:C:\Windows\Fonts\WebSymbols-Regular.otf` | last error: runtime=unchanged(3)×3,set 3×1 real=set 2 |
| 18 | 4 | GetFileAttributesW `C01370` | `W:C:\Windows\Fonts\PT_Sans-Web-Regular.ttf` | last error: runtime=unchanged(3)×3,set 3×1 real=set 2 |
| 19 | 3 | VerSetConditionMask `C00905` | `i:0x0/d:0x2/d:0x3` | return: runtime=0x18 real=OPAQUE |
| 20 | 3 | VerSetConditionMask `C00906` | `$m1/d:0x1/d:0x3` | return: runtime=0x1b real=OPAQUE |

## All mismatches by API

### GetProcAddress: 89 cases, 226 trace calls

- `C01132` ×4 `M:api-ms-win-core-synch-l1-2-0|A:InitializeCriticalSectionEx` — return: runtime=0x0 real=OPAQUE
- `C01126` ×3 `M:advapi32|A:CheckTokenMembership` — return: runtime=0x0 real=OPAQUE
- `C01127` ×3 `M:api-ms-win-core-fibers-l1-1-1|A:FlsAlloc` — return: runtime=0x0 real=OPAQUE
- `C01129` ×3 `M:api-ms-win-core-fibers-l1-1-1|A:FlsSetValue` — return: runtime=0x0 real=OPAQUE
- `C01138` ×3 `M:avrt|A:AvQuerySystemResponsiveness` — return: runtime=0x0 real=OPAQUE
- `C01139` ×3 `M:avrt|A:AvRevertMmThreadCharacteristics` — return: runtime=0x0 real=OPAQUE
- `C01140` ×3 `M:avrt|A:AvRtCreateThreadOrderingGroup` — return: runtime=0x0 real=OPAQUE
- `C01142` ×3 `M:avrt|A:AvRtDeleteThreadOrderingGroup` — return: runtime=0x0 real=OPAQUE
- `C01143` ×3 `M:avrt|A:AvRtJoinThreadOrderingGroup` — return: runtime=0x0 real=OPAQUE
- `C01144` ×3 `M:avrt|A:AvRtLeaveThreadOrderingGroup` — return: runtime=0x0 real=OPAQUE
- `C01145` ×3 `M:avrt|A:AvRtWaitOnThreadOrderingGroup` — return: runtime=0x0 real=OPAQUE
- `C01146` ×3 `M:avrt|A:AvSetMmMaxThreadCharacteristicsW` — return: runtime=0x0 real=OPAQUE
- `C01147` ×3 `M:avrt|A:AvSetMmThreadCharacteristicsW` — return: runtime=0x0 real=OPAQUE
- `C01148` ×3 `M:avrt|A:AvSetMmThreadPriority` — return: runtime=0x0 real=OPAQUE
- `C01162` ×3 `M:kernel32.dll|A:CompareStringOrdinal` — return: runtime=0x0 real=OPAQUE
- `C01165` ×3 `M:kernel32.dll|A:InitializeConditionVariable` — return: runtime=0x0 real=OPAQUE
- `C01166` ×3 `M:kernel32.dll|A:InitializeSRWLock` — return: runtime=0x0 real=OPAQUE
- `C01167` ×3 `M:kernel32.dll|A:RegCopyTreeW` — return: runtime=0x0 real=OPAQUE
- `C01168` ×3 `M:kernel32.dll|A:SetThreadPreferredUILanguages` — return: runtime=0x0 real=OPAQUE
- `C01169` ×3 `M:kernel32.dll|A:TryAcquireSRWLockExclusive` — return: runtime=0x0 real=OPAQUE
- `C01170` ×3 `M:kernel32.dll|A:TryAcquireSRWLockShared` — return: runtime=0x0 real=OPAQUE
- `C01171` ×3 `M:ole32.dll|A:CoAddRefServerProcess` — return: runtime=0x0 real=OPAQUE
- `C01172` ×3 `M:ole32.dll|A:CoCreateInstanceEx` — return: runtime=0x0 real=OPAQUE
- `C01173` ×3 `M:ole32.dll|A:CoInitializeEx` — return: runtime=0x0 real=OPAQUE
- `C01174` ×3 `M:ole32.dll|A:CoReleaseServerProcess` — return: runtime=0x0 real=OPAQUE
- `C01175` ×3 `M:ole32.dll|A:CoResumeClassObjects` — return: runtime=0x0 real=OPAQUE
- `C01176` ×3 `M:ole32.dll|A:CoSuspendClassObjects` — return: runtime=0x0 real=OPAQUE
- `C01181` ×3 `M:oleaut32.dll|A:VarAdd` — return: runtime=0x0 real=OPAQUE
- `C01182` ×3 `M:oleaut32.dll|A:VarAnd` — return: runtime=0x0 real=OPAQUE
- `C01183` ×3 `M:oleaut32.dll|A:VarBoolFromStr` — return: runtime=0x0 real=OPAQUE
- `C01184` ×3 `M:oleaut32.dll|A:VarBstrFromBool` — return: runtime=0x0 real=OPAQUE
- `C01185` ×3 `M:oleaut32.dll|A:VarBstrFromCy` — return: runtime=0x0 real=OPAQUE
- `C01186` ×3 `M:oleaut32.dll|A:VarBstrFromDate` — return: runtime=0x0 real=OPAQUE
- `C01187` ×3 `M:oleaut32.dll|A:VarCmp` — return: runtime=0x0 real=OPAQUE
- `C01188` ×3 `M:oleaut32.dll|A:VarCyFromStr` — return: runtime=0x0 real=OPAQUE
- `C01189` ×3 `M:oleaut32.dll|A:VarDateFromStr` — return: runtime=0x0 real=OPAQUE
- `C01190` ×3 `M:oleaut32.dll|A:VarDiv` — return: runtime=0x0 real=OPAQUE
- `C01191` ×3 `M:oleaut32.dll|A:VarI4FromStr` — return: runtime=0x0 real=OPAQUE
- `C01192` ×3 `M:oleaut32.dll|A:VarIdiv` — return: runtime=0x0 real=OPAQUE
- `C01193` ×3 `M:oleaut32.dll|A:VarMod` — return: runtime=0x0 real=OPAQUE
- ... 49 more

### GetFileAttributesW: 37 cases, 88 trace calls

- `C01364` ×20 `W:C:\Windows\Fonts\OpenSans-Regular.ttf` — last error: runtime=unchanged(3)×15,set 3×5 real=set 2
- `C01359` ×5 `W:C:\Windows\Fonts\tahoma.ttf` — return: runtime=0xffffffff real=0x20
- `C01377` ×5 `W:C:\Program Files\Image-Line\FL Studio 21\Shared\Artwork\Fonts\OpenSans-Regular.ttf` — last error: runtime=set 2×5 real=set 3
- `C01366` ×4 `W:C:\Windows\Fonts\OpenSans-CondLight.ttf` — last error: runtime=unchanged(3)×3,set 3×1 real=set 2
- `C01367` ×4 `W:C:\Windows\Fonts\lucon.ttf` — return: runtime=0xffffffff real=0x20
- `C01369` ×4 `W:C:\Windows\Fonts\WebSymbols-Regular.otf` — last error: runtime=unchanged(3)×3,set 3×1 real=set 2
- `C01370` ×4 `W:C:\Windows\Fonts\PT_Sans-Web-Regular.ttf` — last error: runtime=unchanged(3)×3,set 3×1 real=set 2
- `C01361` ×3 `W:C:\Windows\Fonts\segoeui.ttf` — return: runtime=0xffffffff real=0x20
- `C01368` ×3 `W:C:\Windows\Fonts\ILGlyphsEx.ilfont` — last error: runtime=unchanged(3)×3 real=set 2
- `C01337` ×2 `W:C:\Program Files` — return: runtime=0x10/0xffffffff real=0x31
- `C01339` ×2 `W:C:\Program Files\Image-Line\FL Studio 21` — last error: runtime=unchanged(2)×2 real=set 3
- `C01340` ×2 `W:C:\Program Files\Image-Line\FL Studio 21\Shared` — last error: runtime=unchanged(2)×2 real=set 3
- `C01341` ×2 `W:C:\Program Files\Image-Line\FL Studio 21\Shared\Artwork` — last error: runtime=unchanged(2)×2 real=set 3
- `C01342` ×2 `W:C:\Program Files\Image-Line\FL Studio 21\Shared\Artwork\Fonts` — last error: runtime=unchanged(2)×2 real=set 3
- `C01365` ×2 `W:C:\Windows\Fonts\segoeuisl.ttf` — return: runtime=0xffffffff real=0x20
- `C01376` ×2 `W:C:\Program Files\Image-Line\FL Studio 21\Shared\Artwork\Fonts\ILGlyphsEx.ilfont` — return: runtime=0x80 real=0xffffffff
- `C01387` ×2 `W:c:\windows\fonts\opensans-regular.ttf` — last error: runtime=set 3×1,unchanged(3)×1 real=set 2
- `C01343` ×1 `W:C:\Program Files\Image-Line\FL Studio 21\dsp_ippv2_x64.dll` — last error: runtime=set 2×1 real=set 3
- `C01348` ×1 `W:C:\Program Files\Image-Line\FL Studio 21\Blend2D.dll` — last error: runtime=set 2×1 real=set 3
- `C01351` ×1 `W:C:\Program Files\Image-Line\FL Studio 21\QuickFontCache_x64.dll` — last error: runtime=set 2×1 real=set 3
- `C01354` ×1 `W:C:\Windows\Fonts\msyh.ttc` — return: runtime=0xffffffff real=0x20
- `C01355` ×1 `W:C:\Windows\Fonts\malgun.ttf` — return: runtime=0xffffffff real=0x20
- `C01356` ×1 `W:C:\Windows\Fonts\batang.ttc` — last error: runtime=unchanged(3)×1 real=set 2
- `C01357` ×1 `W:C:\Windows\Fonts\msgothic.ttc` — return: runtime=0xffffffff real=0x20
- `C01358` ×1 `W:C:\Windows\Fonts\arial.ttf` — return: runtime=0xffffffff real=0x20
- `C01360` ×1 `W:C:\Windows\Fonts\seguiemj.ttf` — return: runtime=0xffffffff real=0x20
- `C01371` ×1 `W:C:\Program Files\Image-Line\FL Studio 21\ILTools_x64.dll` — last error: runtime=set 2×1 real=set 3
- `C01374` ×1 `W:C:\Program Files\Image-Line\FL Studio 21\Shared\Blend2D.dll` — last error: runtime=set 2×1 real=set 3
- `C01375` ×1 `W:C:\Program Files\Image-Line\FL Studio 21\Shared\QuickFontCache_x64.dll` — last error: runtime=unchanged(2)×1 real=set 3
- `C01378` ×1 `W:C:\Program Files\Image-Line\FL Studio 21\Shared\Artwork\Fonts\tahoma.ttf` — last error: runtime=set 2×1 real=set 3
- `C01379` ×1 `W:C:\Program Files\Image-Line\FL Studio 21\Shared\Artwork\Fonts\OpenSans-CondLight.ttf` — last error: runtime=set 2×1 real=set 3
- `C01380` ×1 `W:C:\Program Files\Image-Line\FL Studio 21\Shared\Artwork\Fonts\lucon.ttf` — last error: runtime=set 2×1 real=set 3
- `C01381` ×1 `W:C:\Program Files\Image-Line\FL Studio 21\Shared\Artwork\Fonts\WebSymbols-Regular.otf` — last error: runtime=set 2×1 real=set 3
- `C01382` ×1 `W:C:\Program Files\Image-Line\FL Studio 21\Shared\Artwork\Fonts\PT_Sans-Web-Regular.ttf` — last error: runtime=set 2×1 real=set 3
- `C01384` ×1 `W:C:\Windows\Fonts\ILCursors.ilfont` — last error: runtime=set 3×1 real=set 2
- `C01385` ×1 `W:C:\Program Files\Image-Line\FL Studio 21\Shared\Artwork\Fonts\ILCursors.ilfont` — return: runtime=0x80 real=0xffffffff
- `C01386` ×1 `W:c:\program files\image-line\fl studio 21\shared\artwork\fonts\ilcursors.ilfont` — return: runtime=0x80 real=0xffffffff

### GetDIBColorTable: 1 cases, 34 trace calls

- `C02031` ×34 `$mdc2|d:0x0|d:0x100|O:1024` — last error: runtime=unchanged(0)×34 real=set 6

### StretchDIBits: 1 cases, 24 trace calls

- `C02023` ×24 `$mdc1|d:0x0|d:0x0|d:0x20|d:0x20|d:0x0|d:0x0|d:0x20|d:0x20|Z:4096:0=33221100665544ff|Z:44:0=280000002` — return: runtime=0x20 real=0x0; last error: runtime=unchanged(0)×24 real=set 87

### SetErrorMode: 1 cases, 23 trace calls

- `C00630` ×23 `d:0x8000` — return: runtime=0x0/0x8000 real=0x8001

### RegOpenKeyExW: 3 cases, 21 trace calls

- `C01460` ×9 `i:0xffffffff80000002|W:SOFTWARE\Microsoft\Windows NT\CurrentVersion\FontSubstitutes|d:0x0|d:0x20019|` — return: runtime=0x2 real=0x0
- `C01468` ×6 `i:0xffffffff80000001|W:Software\Wow6432Node|d:0x0|d:0x20019|O:8` — return: runtime=0x2 real=0x0
- `C01472` ×6 `i:0xffffffff80000001|W:SOFTWARE\Microsoft\Windows\Shell\Associations\UrlAssociations\https\UserChoic` — return: runtime=0x2 real=0x0

### FindFirstFileW: 12 cases, 12 trace calls

- `C01412` ×1 `W:C:\Program Files\Image-Line\FL Studio 21\FLEngine_x64.en-US|O:592` — last error: runtime=set 2×1 real=set 3
- `C01413` ×1 `W:C:\Program Files\Image-Line\FL Studio 21\FLEngine_x64.en|O:592` — last error: runtime=unchanged(2)×1 real=set 3
- `C01414` ×1 `W:C:\Program Files\Image-Line\FL Studio 21\FLEngine_x64.ENU|O:592` — last error: runtime=unchanged(2)×1 real=set 3
- `C01415` ×1 `W:C:\Program Files\Image-Line\FL Studio 21\FLEngine_x64.EN|O:592` — last error: runtime=unchanged(2)×1 real=set 3
- `C01416` ×1 `W:C:\Program Files\Image-Line\FL Studio 21\Shared\QuickFontCache_x64.en-US|O:592` — last error: runtime=set 2×1 real=set 3
- `C01417` ×1 `W:C:\Program Files\Image-Line\FL Studio 21\Shared\QuickFontCache_x64.en|O:592` — last error: runtime=unchanged(2)×1 real=set 3
- `C01418` ×1 `W:C:\Program Files\Image-Line\FL Studio 21\Shared\QuickFontCache_x64.ENU|O:592` — last error: runtime=unchanged(2)×1 real=set 3
- `C01419` ×1 `W:C:\Program Files\Image-Line\FL Studio 21\Shared\QuickFontCache_x64.EN|O:592` — last error: runtime=unchanged(2)×1 real=set 3
- `C01420` ×1 `W:C:\Program Files\Image-Line\FL Studio 21\Shared\ILTools_x64.en-US|O:592` — last error: runtime=set 2×1 real=set 3
- `C01421` ×1 `W:C:\Program Files\Image-Line\FL Studio 21\Shared\ILTools_x64.en|O:592` — last error: runtime=unchanged(2)×1 real=set 3
- `C01422` ×1 `W:C:\Program Files\Image-Line\FL Studio 21\Shared\ILTools_x64.ENU|O:592` — last error: runtime=unchanged(2)×1 real=set 3
- `C01423` ×1 `W:C:\Program Files\Image-Line\FL Studio 21\Shared\ILTools_x64.EN|O:592` — last error: runtime=unchanged(2)×1 real=set 3

### GetVersionExW: 1 cases, 10 trace calls

- `C00901` ×10 `Z:284:0=1c010000` — output: OSVERSIONINFO.major: runtime=10 real=6; OSVERSIONINFO.minor: runtime=0 real=2

### GetVersion: 1 cases, 8 trace calls

- `C00900` ×8 `` — return: runtime=0x4a65000a real=0x23f00206

### VerSetConditionMask: 2 cases, 6 trace calls

- `C00905` ×3 `i:0x0|d:0x2|d:0x3` — return: runtime=0x18 real=OPAQUE
- `C00906` ×3 `$m1|d:0x1|d:0x3` — return: runtime=0x1b real=OPAQUE

### TlsAlloc: 1 cases, 6 trace calls

- `C01290` ×6 `` — return: runtime=0x0/0x1/0x2/0x3/0x4/0x5 real=0x9

### LoadLibraryW: 2 cases, 5 trace calls

- `C01093` ×3 `W:dwmapi` — return: runtime=0x0 real=OPAQUE
- `C01092` ×2 `W:imm32.dll` — return: runtime=0x0 real=OPAQUE

### GetLocaleInfoW: 1 cases, 4 trace calls

- `C00004` ×4 `d:0x409|d:0x100b|O:512|d:0x100` — output: out2: runtime="0" real="2"

### FlsAlloc: 1 cases, 2 trace calls

- `C01297` ×2 `F:none` — return: runtime=0x0/0x1 real=0x9

### RegOpenKeyExA: 1 cases, 2 trace calls

- `C01476` ×2 `i:0xffffffff80000002|A:SOFTWARE\Microsoft\Cryptography|d:0x0|d:0x20019|O:8` — return: runtime=0x2 real=0x0

### OleInitialize: 1 cases, 2 trace calls

- `C01510` ×2 `NULL` — last error: runtime=unchanged(127)×2 real=set 0

### acmMetrics: 1 cases, 2 trace calls

- `C01974` ×2 `NULL|d:0x32|O:4` — last error: runtime=unchanged(3)×2 real=set 0; output: value: runtime=18 real=50

### LoadIconW: 1 cases, 2 trace calls

- `C02003` ×2 `NULL|W:MAINICON` — last error: runtime=unchanged(0)×2 real=set 1813

### GetClassInfoW: 1 cases, 2 trace calls

- `C02007` ×2 `M:self|W:TPUtilWindow|O:72` — last error: runtime=unchanged(0)×1,unchanged(2)×1 real=set 1411

### GetClipboardFormatNameW: 1 cases, 2 trace calls

- `C02009` ×2 `d:0x0|O:510|d:0xff` — output: out1: runtime="" real="췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍"

### GetVersionExA: 1 cases, 1 trace calls

- `C00902` ×1 `Z:156:0=94000000` — output: OSVERSIONINFO.major: runtime=10 real=6; OSVERSIONINFO.minor: runtime=0 real=2

### GetFileAttributesExW: 1 cases, 1 trace calls

- `C01410` ×1 `W:C:\Program Files\Image-Line\FL Studio 21\FLEngine_x64.dll|d:0x0|O:36` — last error: runtime=set 2×1 real=set 3

## Reference differences (machine / user / time / screen dependent; not spec facts)

- **EnumDisplaySettingsW** (1 cases, 2 trace calls): output: DEVMODEW.w: runtime=1920 real=3840; DEVMODEW.h: runtime=1080 real=2160; DEVMODEW.freq: runtime=60 real=32
- **GetACP** (1 cases, 9 trace calls): return: runtime=0x4e4 real=0x3a4
- **GetCPInfo** (1 cases, 3 trace calls): output: CPINFO.maxcharsize: runtime=1 real=2; CPINFO.leadbytes_nonzero: runtime=False real=True
- **GetClipCursor** (1 cases, 2 trace calls): output: RECT: runtime=(0,0,1920,1080) real=(0,0,2194,1234)
- **GetCurrentProcessId** (1 cases, 5 trace calls): return: runtime=0x1000 real=0x1ee8
- **GetCurrentThreadId** (1 cases, 1684 trace calls): return: runtime=0x1001 real=0x2f98
- **GetEnvironmentVariableW** (2 cases, 6 trace calls): output: out1: runtime="" real="췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍췍
- **GetLocaleInfoW** (2 cases, 6 trace calls): output: out2: runtime="en" real="ja"
- **GetLogicalProcessorInformation** (1 cases, 6 trace calls): output: ReturnedLength: runtime=224 real=1120; entries: runtime=[core{mask=1} numa{mask=1} cache{mask=1 L1 type=2 assoc=8 line=64 size=32768} cache{mask=1 L1 type=1 assoc=8 line=64 size=32768} cache{mask=1 L2 type=0 assoc=8 line=64 size=262144} cache{mask=1 L3
- **GetModuleFileNameA** (1 cases, 1 trace calls): return: runtime=0x39 real=0x47
- **GetModuleFileNameW** (6 cases, 8 trace calls): return: runtime=0x39 real=0x47
- **GetModuleHandleW** (4 cases, 11 trace calls): return: runtime=OPAQUE real=0x0; last error: runtime=unchanged(122)×3 real=set 126
- **GetMonitorInfoW** (1 cases, 2 trace calls): output: MONITORINFO.rcMonitor: runtime=(0, 0, 1920, 1080) real=(0, 0, 2194, 1234); MONITORINFO.rcWork: runtime=(0, 0, 1920, 1080) real=(0, 0, 2194, 1186)
- **GetNativeSystemInfo** (1 cases, 3 trace calls): output: SYSTEM_INFO.mask: runtime=1 real=65535; SYSTEM_INFO.nproc: runtime=1 real=16; SYSTEM_INFO.level: runtime=0 real=26; SYSTEM_INFO.revision: runtime=0 real=17408
- **GetStartupInfoA** (1 cases, 1 trace calls): output: STARTUPINFO.nonzero_bytes_in_first_0x68: runtime=1 real=26
- **GetStartupInfoW** (1 cases, 5 trace calls): output: STARTUPINFO.nonzero_bytes_in_first_0x68: runtime=1 real=26
- **GetStdHandle** (3 cases, 9 trace calls): return: runtime=0x0 real=0x4f0
- **GetSystemDefaultLocaleName** (1 cases, 2 trace calls): output: out0: runtime="en-US" real="ja-JP"
- **GetSystemInfo** (1 cases, 14 trace calls): output: SYSTEM_INFO.mask: runtime=1 real=65535; SYSTEM_INFO.nproc: runtime=1 real=16; SYSTEM_INFO.level: runtime=0 real=26; SYSTEM_INFO.revision: runtime=0 real=17408
- **GetSystemMetrics** (3 cases, 6 trace calls): return: runtime=0x1 real=0x0
- **GetTempPathW** (1 cases, 1 trace calls): return: runtime=0xb real=0x22
- **GetThreadLocale** (1 cases, 4 trace calls): return: runtime=0x409 real=0x411
- **GetThreadPreferredUILanguages** (2 cases, 6 trace calls): output: count: runtime=2 real=4
- **GetThreadUILanguage** (1 cases, 3 trace calls): return: runtime=0x409 real=0x411
- **GetTickCount** (1 cases, 104 trace calls): return: runtime=0x36ee80 real=0x2f9239
- **GetTimeZoneInformation** (1 cases, 1 trace calls): output: TIME_ZONE_INFORMATION.bias: runtime=0 real=-540; TIME_ZONE_INFORMATION.standard: runtime="UTC" real="東京 (標準時)"; TIME_ZONE_INFORMATION.daylight: runtime="" real="東京 (夏時間)"
- **GetUserDefaultUILanguage** (1 cases, 6 trace calls): return: runtime=0x409 real=0x411
- **QueryPerformanceCounter** (1 cases, 19 trace calls): output: value: runtime=36000000000 real=31176540750
- **RegQueryValueExW** (5 cases, 10 trace calls): return: runtime=0x2 real=0x0
- **SHGetFolderPathW** (2 cases, 5 trace calls): output: out4: runtime="C:\Windows\Fonts" real="C:\WINDOWS\Fonts"
- **SystemParametersInfoW** (5 cases, 8 trace calls): output: LOGFONTW.face: runtime="Segoe UI" real="Yu Gothic UI"

## Last error on the success path (informational; Microsoft documents no last-error value after success)

LoadLibraryW=4, GetModuleFileNameW=2, GetComputerNameExW=1, GetModuleFileNameA=1, FlsGetValue=1, SysAllocStringLen=1, SysReAllocStringLen=1, GetFileAttributesW=1

## Effect of a Windows 10 compatibility manifest (second run)

observed-manifest tag: win10-manifest; RtlGetVersion: 10.0.26300 platform=2 sp=0.0 suite=0x100 product=1

- **GetStdHandle**: 3 cases differ, e.g. `C01059`: ret 0x4f0 -> 0x25c; decoded `` -> ``
- **GetVersion**: 1 cases differ, e.g. `C00900`: ret 0x23f00206 -> 0x66bc000a; decoded `` -> ``
- **GetVersionExA**: 1 cases differ, e.g. `C00902`: ret 0x1 -> 0x1; decoded `OSVERSIONINFO{size=148 major=6 minor=2 build=9200 platform=2 csd=""} EX{spmajor=` -> `OSVERSIONINFO{size=148 major=10 minor=0 build=26300 platform=2 csd=""} EX{spmajo`
- **GetVersionExW**: 2 cases differ, e.g. `C00901`: ret 0x1 -> 0x1; decoded `OSVERSIONINFO{size=284 major=6 minor=2 build=9200 platform=2 csd=""} EX{spmajor=` -> `OSVERSIONINFO{size=284 major=10 minor=0 build=26300 platform=2 csd=""} EX{spmajo`
- **VerifyVersionInfoW**: 2 cases differ, e.g. `C00972`: ret 0x0 -> 0x1; decoded `` -> ``

## Cases the tool skipped or Windows could not run

- DeleteObject: skip:policy ×1
- FreeLibrary: skip:policy ×1
- GetModuleHandleA: skip:policy ×1
- GetModuleHandleW: skip:policy ×1
- RegCloseKey: skip:policy ×13

