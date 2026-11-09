# win-replay 実機測定(PR #2、`worklog/1/`)の切り分け: mismatch 161 件、reference 55 件、real-only 1,559 件と、A2b への含意

作成: 2026-10-30(基準 HEAD c76a7c3。作業は 2026-10-30 の途中でサーバー再起動をはさみ、再起動後に `lift/out-triage/` と `docs/investigations/win-replay-triage/` の状態を確かめて続けた)。
状態: **調査(読み取りと分析のみ。何も決めない)**。runtime、生成器、テスト、`tools/win-replay/`、`worklog/1/` は変更していない。決めるのは PM。
凡例: **実測**＝ファイルの値そのもの、または `compare.py` / 集計スクリプトの出力／**読み**＝私の解釈(コードの静的な読み、Learn の文言からの導出。実行していないもの)／**未確認**。「非公式」は Learn 以外(Wine のソース)。

## 0. 結果(先に)

| mismatch 161 件の分類 | 件 | 内訳(`mismatch-161.tsv` の subclass) |
|---|--:|---|
| (a) runtime の欠陥(Learn だけから環境によらず決まるのに runtime が違う) | **66** | GetProcAddress 65、LoadLibraryW("dwmapi") 1。**いずれも、バグではなく PM の決定 0015 と Learn の食い違い** |
| (b) 環境差(OS・ファイルツリー・レジストリ・起動条件・導入物) | **59** | ファイルツリー 50(GetFileAttributesW 37、FindFirstFileW 12、GetFileAttributesExW 1)、レジストリ 4、OS バージョン/manifest 3、起動時のエラーモード 1、導入済みコーデック 1 |
| (c) 再生入力の影響 | **2** | StretchDIBits(再生ツールが 13 引数目を渡していない疑い。読み)、GetClipboardFormatNameW(出力バッファの事前充填) |
| (d) 参考値・比較規則・仕様が値を決めない | **3** | TlsAlloc、FlsAlloc(Learn が索引を決めない)、OleInitialize(`compare.py` が HRESULT 0 を失敗扱い) |
| (e) 未確認(一次資料が無い・足りない) | **31** | GetProcAddress 24(Learn が最小 OS を書かない)、VerSetConditionMask 2、GetDIBColorTable・LoadIconW・GetClassInfoW・GetLocaleInfoW(0x100b)各 1、LoadLibraryW("imm32") 1 ほか |
| 計 | **161** | A と A'(同条件の再実行)で、判定に使う戻り値・last error・decoded が揺れる mismatch は **0 件**。出力列(`outs`)まで見ると 4 件(RegOpenKeyExW 3、RegOpenKeyExA 1。`a4=` の出力ハンドル値だけが違い、判定には使われない)(実測)。A と B で変わるのは GetVersion 系 3 件、A と C で変わるのは 0 件 |

reference 55 件: ロケール 10、機種 12、画面 11、プロセス(起動条件)11、揺れる値 9、再生入力 2。**この 55 件に「仕様で決まる値の食い違い」は無い**(`reference-55.tsv`)。
real-only 1,559 件(+ skipped 17): **仕様の規則として使えるもの(U1)は 68 件、実測値としてのみ使えるもの(U2)は 618 件、環境依存で使えないもの(E)は 810 件**、揺れる値 30、ツール内部 33(`real-only-usable.tsv`)。

**A2b への含意(数字)**: 監査 171 種のうち、**実機の値が一致し、かつ値が環境に依存せず、基準を満たして「仕様確認済み」に上がる種は 2(DestroyCursor、SelectPalette。呼び出し 64 回)。条件付き 2(MultiByteToWideChar、WideCharToMultiByte。3,589 回。PM 判断)。数えない 3(同 137 回)**。P/X の 78 種(6,927 回)のうち、実機で値が一致した 22 種の大半は「一致したが環境(OS のデータ・画面・ロケール)に依存する」ため上がらない(13 種)。**「実機測定だけでは A2b は閉じない」という以前の見立て(`a2b-audit/SUMMARY.md`、`tools/win-replay/README.md`)は変わらない**(§6)。

## 1. 入力と手順

- 入力(読み取りのみ): `worklog/1/`(`compare-report.md`、`observed.tsv`(A)、`observed-repeat.tsv`(A')、`observed-manifest.tsv`(B)、`observed-dpiaware.tsv`(C)、`env.md`、`notes.md`)、`tools/win-replay/`(`replay.c`、`cases.tsv` 2,144 件、`compare.py`、`coverage.tsv`)、`lift/runtime/fle_*.c`、`lift/crates/fe-lift/src/imports_table.rs`、`docs/decisions/`、`docs/investigations/a2b-audit/`、`a4-six-module-trace/`。
- **再現**(実測): `PYTHONUTF8=1 python3 tools/win-replay/compare.py --cases cases.tsv --observed worklog/1/observed.tsv --observed-manifest worklog/1/observed-manifest.tsv` は `match=352, mismatch=161, reference=55, real-only=1559, skipped=17` を再現し、`compare-report.md`(CRLF を除く)と差が無い。同じ `compare.py` を A'、B、C にも適用した: A' は `match=352 mismatch=161 reference=55`(同一)、B 単独は `match=352 mismatch=158 reference=58`(GetVersion 系 3 件が mismatch から reference に移る。build だけ OS の差)、C 単独は `match=348 mismatch=161 reference=59`。
- **判定の再導出**(実測): `compare.py` の規則(`ret` と `last error`(失敗時のみ)と `decoded` の 3 欄の比較)を自分のスクリプトで 2,144 件に当て直し、各ケースが何を比べたか(値/不透明/比較なし)を `lift/out-triage/detail.tsv` に出した。
- **一次資料**: Microsoft Learn(`learn.microsoft.com/en-us/...`、取得 2026-10-30、`lift/out-triage/learn/`)と MicrosoftDocs/sdk-api のソース(`raw.githubusercontent.com/MicrosoftDocs/sdk-api/docs/...`、`lift/out-triage/sdkmd/`。最小 OS 欄と DLL 欄の取得に使った)。Wine は 1 回、VerSetConditionMask の bit 配置の参考に読んだだけで、根拠にしていない(非公式)。
- **作業の途中経過**: 各分類ごとに TSV を書いた(`mismatch-161.tsv` → `real-only-*.tsv` → `a2b-171.tsv` → `reference-55.tsv` → `fix-plan.md` → `next-issue-draft.md` → 本文)。

## 2. 前提: 実機は runtime の仮定の環境と違う(env.md)

| 項目 | 実機(実測。`env.md`、`observed*.tsv` の環境行) | runtime の仮定 |
|---|---|---|
| OS | Windows 11 Pro 10.0.26300(26H2)。manifest なしの `GetVersion` は 6.2 build 9200(Learn が Windows 8.1 manifest なしの規則を書く)。manifest あり(B)は 10.0 build 26300 | Windows 10.0.19045(決定 0014) |
| ロケール・CP | ja-JP(LCID 1041)、ACP/OEMCP 932、Tokyo Standard Time | en-US、1252/437、UTC |
| 画面・DPI | 2194x1234(A)/3840x2160(C = DPI 認識)。比 1.75。Hyper-V + リモート表示。GDI の DPI は 96 のまま | 1920x1080、96 DPI |
| 機種 | Hyper-V VM、プロセッサ 16 | 1 プロセッサ |
| 起動 | PowerShell の子の `replay.exe`(MSVC `/MT`、コンソール) | FL Studio のホスト exe |

**これらは「en-US / Windows 10 の値」としては使えない**。以下の分類は、**(i) Learn が環境によらず決める規則**、**(ii) 実機が環境によって変わる値**、**(iii) 実行ごとに揺れる値**を分けて行った。

## 3. mismatch 161 件の分類(`mismatch-161.tsv`)

分類基準(先に固定した):

- **(a)** Learn(または、Learn の最小 OS 欄)から**環境によらず**結果が決まり、runtime が違う値を返す。runtime の値の根拠が Learn に反することをコードの行で示せる。
- **(b)** 実機の値が OS ビルド/ロケール/CP/画面/機種/ファイルとレジストリの有無/導入物/起動条件で変わり、runtime の規則は実機と整合している。
- **(c)** 再生ツールの入力(構造体の中身が inferred、引数の個数、出力バッファの初期値)が原因で、実機が仕様どおりの入力に対する結果を返していない。
- **(d)** 仕様が値を決めない(Learn が opaque と書く)、または `compare.py` の規則の産物。
- **(e)** 一次資料が無い・足りない・矛盾。実測だけがある。

### 3.1 API 別の件数(`api-class-matrix.tsv`)

| API | (a) | (b) | (c) | (d) | (e) | 計 |
|---|--:|--:|--:|--:|--:|--:|
| GetProcAddress | 65 | | | | 24 | 89 |
| GetFileAttributesW | | 37 | | | | 37 |
| FindFirstFileW | | 12 | | | | 12 |
| RegOpenKeyExW | | 3 | | | | 3 |
| LoadLibraryW | 1 | | | | 1 | 2 |
| VerSetConditionMask | | | | | 2 | 2 |
| GetFileAttributesExW、GetVersion、GetVersionExA、GetVersionExW、RegOpenKeyExA、SetErrorMode、acmMetrics | | 7(各 1) | | | | 7 |
| GetClipboardFormatNameW、StretchDIBits | | | 2(各 1) | | | 2 |
| FlsAlloc、OleInitialize、TlsAlloc | | | | 3(各 1) | | 3 |
| GetClassInfoW、GetDIBColorTable、GetLocaleInfoW、LoadIconW | | | | | 4(各 1) | 4 |
| **計** | **66** | **59** | **2** | **3** | **31** | **161** |

### 3.2 (a) runtime の欠陥(66 件): 決定 0015 と Learn の食い違い

- **GetProcAddress 65 件**。実測: 実機は 89 名すべてで非 NULL(A=A'=B=C)。runtime は `fle_modules.c:348-357`(応答表に行が無い名前は NULL と 127)。Learn の最小クライアント: Windows 2000 が 13、XP が 4、Vista が 37、Windows 7 が 7、Windows 8 が 3、Windows 8.1 が 1、問い合わせ先の DLL が一致(読み: Windows 10 に在ると導ける)。**実装がバグなのではなく、決定 0015 の PM 決定(「応答表に無い= 無い」)が Learn と食い違う**。ゲストは GetProcAddress の NULL を 229/1,612 回受け取り(14.2%。`a4-six-module-trace/summary/getprocaddress.tsv`)、実機の Windows 10 とは違う経路を通っている(読み)。
- **LoadLibraryW("dwmapi") 1 件**。実測: 実機は成功(`modfile=C:\WINDOWS\SYSTEM32\dwmapi.DLL`)。runtime は NULL+126(`fle_load.c:56-58` の `fle_load_absent[]`、決定 0015)。Learn の DwmIsCompositionEnabled は Vista・Dwmapi.dll。
- 修正案とテスト案は `fix-plan.md`(§1、§2)。**G0(決定 0015 を維持し「環境の定義」と明記)なら (a) は 0 件になる**。決めるのは PM。

### 3.3 (b) 環境差(59 件)

- **ファイルツリー 50 件**(GetFileAttributesW 37、FindFirstFileW 12、GetFileAttributesExW 1)。実機は C:\Windows\Fonts と C:\Program Files に存在し、C:\Program Files\Image-Line 以下が無い(失敗= 3)。runtime の仮想ボリュームはハーネスが Image-Line..Fonts を作り、C:\Windows\Fonts が無い。**runtime の規則(最後の成分が無ければ 2、途中が無ければ 3。`fle_vfs.c:342-366`)を実機の木に当てると、実機の失敗 65 件中 64 件で実機の last error と一致する**(読み: 実機の成功の形から木を組み、規則を転記して評価した。runtime の C は実行していない。`lift/out-triage/vfs_model.py`。不一致 1 件は空文字列 `W:`)。(b) の一部(5 件)はトレース上の呼び出しがハーネスの種付けで、ゲストの呼び出しではない(`mismatch-161.tsv` の evidence に注記)。
- **レジストリ 4 件**(RegOpenKeyExW 3、RegOpenKeyExA 1)。実機は成功(0)、runtime は 2。キーの有無は Learn が決めない。
- **OS バージョン 3 件**(GetVersion、GetVersionExW、GetVersionExA)。manifest なし(A): 6.2 build 9200、あり(B): 10.0 build 26300。**runtime(10.0 build 19045)との差は B では build だけ**(実測: compare-report-manifest)。A の 6.2 は Learn の規則(「Targeting your application at Windows 8.1」)。FL64.exe の manifest は未確認。
- **SetErrorMode 1 件**。「前の状態を返す」規則は一致(C00631)。差は起動時の初期値 0x8001(親からの継承。Learn: 「A child process inherits the error mode of its parent process」)。
- **acmMetrics 1 件**。実機 50、runtime 18(決定 0098「出典のない選択」)。Learn は「システムの最大の WAVEFORMATEX」(導入済みコーデック依存)。last error の差は戻り 0 の失敗扱い(`compare.py`)の産物。

### 3.4 (c) 再生入力の影響(2 件)

- **StretchDIBits**: 実機は戻り 0、last error 87、画素は書かれない(C02024)。読み(静的): `replay.c:873` の `Fn12` は 12 引数、StretchDIBits は 13 引数で、`replay.c:1063` は `c.a[0]..c.a[11]` しか渡さない。第 13 引数(rop)が載らず、仕様どおりの入力ではない。runtime の戻り 0x20 との比較に使えない。**この読みは再実行で確かめていない**(replay.c は 13 トークン分を `Ctx.a[13]` に読むが、呼び出しは 12 個)。
- **GetClipboardFormatNameW**: 戻り 0 と last error 87 は一致。差は出力バッファ(実機は事前充填 0xCD が残る)だけ。

### 3.5 (d) 参考値・比較規則(3 件)

- **TlsAlloc**(実機 9、runtime 0..5)、**FlsAlloc**(9)。Learn: 「The value of the TLS index should be treated as an opaque value; do not assume that it is an index into a zero-based array.」。
- **OleInitialize**: 戻りは両方 0 で一致。`compare.py:215` が戻り 0 を失敗とみなし、実機の last error 0(set)と runtime の unchanged(127)を差とした。

### 3.6 (e) 未確認(31 件)

- **GetProcAddress 24 件**: oleaut32 の Var* 21 と VariantChangeTypeEx は sdk-api のソースで最小 OS 欄が空。RegCopyTreeW は Learn の DLL が Advapi32。UpdateLayeredWindowIndirect はページが 404。
- **VerSetConditionMask 2 件**: 実機の戻りは bit 63 が立つ(0x8000000000000018、0x800000000000001b)。runtime は 0x18、0x1b。**下位 63 ビットは、この API の 77 ケース全部で runtime の式と一致する**(実測。式は `fle_imports.c:263-277` を転記して評価)。`compare.py` が 64 ビット値を OPAQUE 扱いするため差が値として出ない。Learn は配置も bit 63 も書かない。
- **GetLocaleInfoW(LCID 0x409, 0x100b) 1 件**: 実機 "2"、runtime "0"(決定 0041 の仮。出典は Wine)。同じ API の LCID 0x409 の 57 種は一致。Learn は値を書かない。決定 0041 の撤回条件に当たるが、採ると a4 が EnumCalendarInfoW(未実装)で止まる(決定 0041 §4。読み)。
- **GetDIBColorTable / LoadIconW / GetClassInfoW**: 戻りは一致。last error だけが違う(実機 6 / 1813 / 1411)。Learn は失敗時の code を書かない(System Error Codes には値の名前がある)。決定 0069/0070 の撤回条件に当たる(ゲストが GetLastError を読まないことは決定 0069 が確認済み)。
- **LoadLibraryW("imm32")**: Learn は最小クライアントを「Windows XP、East Asian language support installed」と書く。実機は ja-JP。en-US の Windows 10 に在るかは決まらない。

### 3.7 A と A'(同条件の再実行)で揺れる項目の扱い

実測: mismatch 161 件のうち、判定に使う 3 欄(戻り値・last error・decoded)が A と A' で違うものは **0 件**。全列(出力バッファ `outs`、`extra` を含む)で A と A' が違う 92 ケース(`notes.md` の 92 と一致)の判定は match 48、real-only 32、reference 8、mismatch 4(RegOpenKeyExW/A の出力ハンドル値 `a4=` だけが違う)。3 欄だけで見ると 79 ケース(match 44、real-only 27、reference 8)。ポインタ・ハンドル・時刻・PID/TID が揺れる(`notes.md` の記述と一致)。**mismatch には、実行ごとに揺れる値(時刻・PID・ポインタ)で判定が決まる件は無い**(そうした値は `compare.py` が最初から reference または OPAQUE にするため)。揺れる値で差が出たものは reference 55 件の 9 件(`reference-55.tsv` の d-varying)と real-only に入っている。mismatch の (d) 3 件は揺れではなく「仕様が値を決めない」(TlsAlloc/FlsAlloc の 9 は A、A'、B、C で同じ)か比較規則の産物(OleInitialize)で、揺れる項目とは分けてある。

## 4. reference 55 件(`reference-55.tsv`)

| 原因 | 件 | API |
|---|--:|---|
| ロケール(ja-JP、CP 932) | 10 | GetACP、GetThreadLocale、GetUserDefaultUILanguage、GetThreadUILanguage、GetSystemDefaultLocaleName、GetThreadPreferredUILanguages×2、GetCPInfo(CP_ACP)、GetLocaleInfoW(LCID 0x400)×2 |
| 機種・ユーザー | 12 | GetSystemInfo、GetNativeSystemInfo、GetLogicalProcessorInformation、GetTimeZoneInformation、GetTempPathW、SHGetFolderPathW×2、RegQueryValueExW×5 |
| 画面・DPI | 11 | GetSystemMetrics×3、SystemParametersInfoW×5、GetClipCursor、EnumDisplaySettingsW、GetMonitorInfoW |
| プロセス(起動条件) | 11 | GetModuleFileNameW×6、GetModuleFileNameA、GetModuleHandleW×4(実機の replay.exe が oleaut32 等をロードしていない) |
| 揺れる値(A と A' で違う) | 9 | GetTickCount、QueryPerformanceCounter、GetCurrentProcessId、GetCurrentThreadId、GetStdHandle×3、GetStartupInfoW/A |
| 再生入力(出力バッファ) | 2 | GetEnvironmentVariableW×2 |

**仕様で決まる値の食い違いは 0 件**。A と C で値が変わるのは reference 15 件(画面・DPI 系 + GetStartupInfoW)。**実機の差を「en-US / Windows 10 の値」に読み替えてはいけない**。

## 5. real-only 1,559 件(+ skipped 17)の使い方(`real-only-usable.tsv`、`real-only-cases.tsv`)

real-only は「runtime がまだ行を持たない、またはトレースに無い」ケースの実機値。分類(ケース単位、`real-only-cases.tsv`):

| 区分 | 件 | 意味 |
|---|--:|---|
| U1 仕様の規則として使える | 68 | Learn の文言から計算でき、環境によらない: MulDiv 12、IntersectRect 6、HeapSize 12、GetObjectType 4、HeapFree ほか、CP_UTF8 の MultiByteToWideChar/WideCharToMultiByte など |
| U2 実測値としてのみ使える | 618 | OS のデータ表(NLS、コードページ)。Learn は表を書かない。Win11 26H2 の 1 点。GetLocaleInfoW(LCID 0x409)342、CompareStringW(0x409)60、VerSetConditionMask 73 ほか |
| E 環境依存で使えない | 810 | GetLocaleInfoW(LCID 0x400/0x800 = ja-JP)118、GetDeviceCaps 238、GetSystemMetrics 138、GetSysColor 30、IsValidLocale 48、SHGetFolderPathW 22 ほか |
| V 揺れる | 30 | ポインタ・ハンドル(HeapAlloc、SelectObject ほか)。成功/失敗の別だけが使える |
| T ツール内部 | 33 | `@peek` などのヘルパー、読み戻し補助 |
| S skipped | 17 | ツールが拒否(`skip:policy`) |

将来の期待値として使える(U1)のうち、実機が規則と一致したことを確かめたもの(実測): **MulDiv 12/13(C00896: -2147483648×1/1 の実機は -1 で、素直な丸め規則の計算値 -2147483648 と違う。runtime の式(`fle_glob.c:483`)の転記評価も -2147483648 で実機と違う。Learn の「overflow」の読みの問題)**、IntersectRect 7/7、HeapSize 12/12(要求サイズと一致。0、1 バイトを含む)。**MultiByteToWideChar(cp1252)は 256 バイト全部が、Python の cp1252 + 未定義 5 バイト(0x81、0x8d、0x8f、0x90、0x9d)の同値写像と一致(256/256。決定 0084 の選択と実機が一致)**、WideCharToMultiByte(1252)の 27 点の非 Latin-1 も一致(実測)。ただし 1252 の WC→MB で 0x80..0x9f のうち表に無い 27 バイトは "?" に落ちる(実測。best-fit なし)。

**使えない例(注意)**: 実機の `GetLocaleInfoW(0x400, ...)` は "ja"/"JP"、`CP_ACP` は 932。これらを en-US の期待値にしてはいけない。

## 6. A2b への含意(監査 171 種)

### 6.1 「仕様確認済み」に数える基準(決定 0046 §3 の定義を緩めない)

決定 0046 §3: 「閉包の IAT thunk のうち、実行で踏まれるものが全て、仕様確認済みの応答を持つ。仮の決定を含む応答は、実機または公式資料で確認するまで数えない」。ここから、**実機測定で P/X(仮・創作・非公式)の種が「確認済み」に上がる**条件を次の 4 つに固定した(結果を見る前に決めた。`a2b_calc.py` の先頭にも書いた):

1. **R1**: 実行で踏まれた引数の形(`trace_n > 0`)の再生ケースが 1 件以上あり、**そのすべてが match**(mismatch、reference、skipped を含まない)。
2. **R2**: match の少なくとも 1 件が**値を比べている**(戻り値、失敗時の last error、デコードした出力のいずれか)。ハンドル/ポインタの「NULL でない」だけの match は値の確認ではない。
3. **R3**: R2 の値が **OS ビルド、ロケール/CP/NLS データ、画面・DPI、機種、ユーザー、導入物、起動条件に依存しない**(API ごとの読みを `a2b-171.tsv` の reason 列に書いた)。
4. **R4**: 実機が **Windows 10 22H2 の en-US ではない**(Win11 26H2 の ja-JP)ことを割り引き、**OS 差を持ちうるデータ(NLS 表、コードページ表など)は数えない**(条件付きの欄に分ける)。

S/N は監査が実機なしで「仕様確認済み」に数えている(`SUMMARY.md`)。D は PM 判断。

### 6.2 数字(実測の集計。`a2b-171.tsv`)

| 区分 | 種 | 呼び出し回数 | 実機測定の結果 |
|---|--:|--:|---|
| S + N(監査が数える) | 41 | 3,072 | 実機でも一致 14、不透明/比較なし 11、比較不能 8(トレースに引数の形が無い)、K(測らない)7、**不一致 1(OleInitialize。HRESULT 0 の compare.py 規則の産物 = (d))** |
| D(PM 判断) | 52 | 3,573 | 一致 11、不透明 21、K 7、不一致 6、reference 5、比較不能 2 |
| P + X(数えられない) | 78 | 6,927 | 下の表 |
| 計 | 171 | 13,572 | |

P + X 78 種の実機測定の結果:

| 行き先 | 種 | 呼び出し | API |
|---|--:|--:|---|
| **上がる(R1〜R4 を満たす)** | **2** | **64** | DestroyCursor(戻り 1、共有カーソル 13/13、Learn「成功は非 0」)、SelectPalette(hPal=NULL の失敗形が一致、Learn「失敗は NULL」) |
| **条件付き(PM 判断)** | 2 | 3,589 | MultiByteToWideChar、WideCharToMultiByte。実行された形はほぼ明示の CP(1252: 3,402 回、UTF-8: 177 回、CP_ACP: 10 回。`codepage-conversions.tsv`)で、実機は 1252 の表が 256/256 一致(実測)。**ただし Win11 26H2 の 1 点で、Win10 22H2 との差は未測定**(R4)。CP_ACP の 10 回は実機が CP 932 で確認にならない |
| 数えない(弱い) | 3 | 137 | SetBkColor、SetTextColor(新メモリ DC の初期色。Learn が値を書かない)、GetFileVersionInfoSizeW(存在しないパスの失敗。ファイルの有無は環境の定義) |
| 一致したが環境に依存 | 13 | 225 | CompareStringW、GetStringTypeW、LCMapStringW(NLS 表)、IsValidLocale、IsValidCodePage、SetThreadLocale、VerifyVersionInfoW(OS バージョン)、GetDeviceCaps/GetSysColor/EnumDisplayMonitors/GetProcessDpiAwareness(画面・DPI)、EnumFontFamiliesExW、GetComputerNameExW |
| 一致に見えるが `compare.py` が比べない出力で実機と違う | 2 | 78 | **PatBlt**: WHITENESS 後の 32bpp の未使用バイトが実機 0xff、runtime 0(`fle_gdi.c:1459`)。**GetObjectW**: 32bpp BI_RGB の dsBitfields が実機 0,0,0、runtime 0x00FF0000..(`fle_gdi.c:519-520`)。いずれも `compare.py` は出力を比べない(trace_out が空)ので match に入っている |
| 不一致を含む | 15 | 2,300 | GetProcAddress(1,612 回)、GetFileAttributesW、LoadLibraryW、GetLocaleInfoW、GetVersion* ほか |
| reference(環境依存) | 25 | 263 | ロケール、機種、画面、起動条件(§4) |
| 値を比べていない(不透明のみ) | 10 | 231 | CreateCompatibleDC、LoadImageW、LoadLibraryA/ExW ほか。ハンドルの「NULL でない」だけ |
| 比較不能 | 2 | 20 | RegQueryInfoKeyW、VirtualProtect(再生ケースに引数の形が無い) |
| 測らない(K) | 4 | 20 | CreateFileW、CreateWindowExW、DefWindowProcW、RegCreateKeyExW |

### 6.3 A2b の見立てが変わるか

- **変わらない**: 171 種のうち、実機測定で P/X から「仕様確認済み」に**上がる種は 2(64 回)**、条件付きで 2(3,589 回)。P/X 78 種のうち、上がる 2 種と条件付き 2 種を除く 74 種は、この測定では確認できない。`SUMMARY.md` の「P と X の 78 種(回数の 51%)は仮／創作／非公式で、数えられない。A2b は、実機または公式資料での確認が無い限り満たせない」は、**実機 1 台(Win11 26H2 ja-JP)の測定を足しても結論が変わらない**。上がった 2 種の回数の合計は P/X の呼び出しの 1%(64/6,927)、条件付きを入れても 53%(3,653/6,927)。
- **実機測定が「環境の定義でしか決まらない」ことを確かめた**: 画面・ロケール・機種・ファイル・レジストリ・導入物の 25 + 13 種は、実機の値が Windows の仕様ではなく環境の値であることを(a2b 監査の予想どおり)実測で示した。これらは PM が環境の定義を固定する以外に道が無い。
- **実機測定が見つけた「runtime の値の疑い」**(A2b のためではなく忠実度のため): PatBlt の未使用バイト、GetObjectW の dsBitfields、決定 0015 の GetProcAddress/LoadLibrary、GetLocaleInfoW(0x100b)。いずれも `fix-plan.md` に整理した。
- **`compare.py` の限界**: match 352 件のうち、値を 1 つも比べていない(ハンドルの「NULL でない」だけ、または比較項目なし)ものが **126 件(36%)**。値を比べた match は 226 件。「match 352」を「352 件の応答が実機と同じ」と読んではいけない。

### 6.4 数字の根拠の弱い点

- 実機は 1 台(Win11 26H2 ja-JP、Hyper-V、175% 推定)。**Windows 10 22H2 の en-US の確認ではない**。
- R3 の「環境に依存しない」判断は API ごとの私の読み(`a2b-171.tsv` の reason 列)。DestroyCursor と SelectPalette の 2 種は、ケース数が少ない(13 と 1)。
- 実行で踏まれた引数の形は、トレースが残した形だけ(13,572 回中 8,093 回。`README.md`)。入力の構造体は inferred(LOGFONT、BITMAPINFO、ICONINFO ほか)で、GetObjectW(DIB section)の比較は入力が推定に依る。

## 7. runtime の欠陥の修正計画(`fix-plan.md`)

要約(詳細は fix-plan。実装していない):

- **実質の対象は決定 0015 の見直し 2 点**: (1) GetProcAddress で Learn が Windows 10 以前から在ると書く 65 名に応答表の行を足し、呼ばれたら名前付き停止にする(G1)。(2) `fle_load_absent[]` から dwmapi を外す。**採らないなら (a) は 0 件**(G0)。
- G1 の影響(読み): `imports_table.rs` の TABLE の**末尾**に行を足す(擬似 VA は動かない)。FLEngine の IAT にもある 10 名のうち normal slot の comctl32 FlatSB 6 名を足すと `vcall_thunk_dll.rs:179` の `unsupported_total` 379 が変わる見込み(delay import の 4 名は thunk を持たない)。ゲストが「機能あり」の経路に入り、a4 の完走が止まる可能性がある(未確認。実行していない)。
- 次の作業に渡せる項目: テストの追加案、固定数テストへの影響、実装の順序は fix-plan §1.3、§2、§4。
- (a) としなかった項目(GetDIBColorTable、StretchDIBits、SetErrorMode、GetFileAttributesW、TlsAlloc、RegOpenKeyExW、VerSetConditionMask)の理由は fix-plan §0、§3。

## 8. 実機測定の追加の要否(`next-issue-draft.md`、投稿しない)

- **現在の VM でできる**: 再生ツールの修正(StretchDIBits の 13 引数、出力バッファの充填 2 通り、実行ファイル名の固定)と `compare.py` の規則(HRESULT の失敗扱い、64 ビット値、出力構造体の比較)。ここは Linux 側の作業と、小さな再測定。
- **Windows 10 22H2 の en-US の VM が要る**: GetProcAddress の存在確認、LoadLibrary の成否、VerSetConditionMask の bit 63、標準フォント・レジストリの有無、PatBlt/GetObjectW/SetBkColor の既定値。**どれも A2b を閉じる測定ではない**(§6)。用意できるかどうかはユーザーの判断で、**この調査は必要とも不要とも決めない**。必要になった場合の Issue 下書きは next-issue-draft.md。

## 9. PM の判断が要る点

1. 決定 0015(GetProcAddress と LoadLibrary の「無い」集合)を維持するか(G0)、Learn が最小 OS を書く 65 名に行を足すか(G1)。
2. MultiByteToWideChar/WideCharToMultiByte(1252/UTF-8)を「実機 1 点で確認済み」に数えるか(条件付き 2 種、3,589 回)。基準 R4(OS 差)を緩めるかどうかの判断。
3. `compare.py` の出力が見ない項目(PatBlt の未使用バイト、GetObjectW の dsBitfields)を忠実度の問題として扱うか。
4. GetLocaleInfoW(0x409, 0x100b)= "2"、LoadIconW の last error 1813、GetClassInfoW の 1411、GetDIBColorTable の 6 を実機の値に合わせるか(決定 0041/0069/0070 の撤回条件)。
5. FL64.exe の manifest(GetVersion の 6.2 と 10.0 のどちらが実環境か)。
6. Windows 10 の en-US の VM を用意するか(§8)。

## 10. 限界・確認できなかったこと

- **実機で確認していない**: runtime を動かしていない。runtime の式(MulDiv、VerSetConditionMask、GetFileAttributesW の lookup 規則)の評価は C のソースを Python に転記して行った(**C の実行ではない**)。
- 実機は 1 台。env.md の画面スケール(175%)は測定者の推測。接続方法(拡張セッションか RDP か)は未確認。
- StretchDIBits の 12 引数/13 引数の読みは静的で、再実行で確かめていない。
- Learn: 取得 2026-10-30。最小 OS 欄が空のページ(oleaut32 の Var*)と 404(UpdateLayeredWindowIndirect)は「Windows 10 に在る」を導けない(e)。Learn の版(`ms.date`)は各ページにあるが、全ページの版の一覧は作っていない。GetDIBColorTable と StretchDIBits の Learn の ms.date は 2018-12-05、updated_at 2024-02-22。
- `compare.py` が比べない出力(PatBlt の画素、GetObjectW の構造体)は、runtime が実機と違う可能性を示す単発の事実であり、他の API にも同種のものがあるかは網羅していない(match 352 件のうち出力バッファが書かれたが比べていないのは 72 件。`detail.tsv` 参照)。
- 「実行された引数の形」の網羅性: トレースが記録した 13,572 回の引数の形に依る。ゲストが実機で踏む別の形は分からない。
- 作業はサーバー再起動をはさんだ。再起動前の中間物(`lift/out-triage/`)を再利用し、判定は再導出して再起動前の数と一致することを確かめた(match 352 / mismatch 161 / reference 55 / real-only 1,559 / skipped 17)。

## 11. 規則の確認(自己申告)

- 作業ツリーの tracked ファイル、`worklog/1/`、`tools/win-replay/`(`compare.py` を import しただけ)、`lift/runtime/`、テストは変更していない。git の変更系操作はしていない(`git status`、`git log`、`git diff --stat` の読み取りのみ)。commit していない。
- **`tools/win-replay/__pycache__/compare.cpython-314.pyc`(2026-10-30 09:24 作成、untracked)が存在する**。私が `PYTHONDONTWRITEBYTECODE=1` を付け忘れた python 呼び出し(`import compare`)で作った可能性が高い(作成時刻は私の作業時間帯。ただし同時に動く別の作業者が作った可能性は否定できない。未確認)。**編集してよい範囲の外なので削除していない**。PM が消す場合は `tools/win-replay/__pycache__/` を削除する(中身は .pyc 1 個)。以降の python 呼び出しには `PYTHONDONTWRITEBYTECODE=1` と、スクリプト内の `sys.dont_write_bytecode = True` を付けた。
- `/tmp` 直下には書いていない(ネットワークからの取得物は `lift/out-triage/learn/`、`sdkmd/`、`sdkapi-tree.json` に保存した)。外部アクセスは Microsoft Learn、`raw.githubusercontent.com`(MicrosoftDocs/sdk-api、Wine の `version.c` を 1 回)、GitHub API の tree 取得 1 回(認証なし)。資格情報・ブラウザのプロファイルには触れていない。
- コマンド文字列に `sleep`、`timeout`(コマンドとして)、`nohup`、末尾の `&` は書いていない。

## 12. 成果物

- 本文: `docs/investigations/win-replay-triage.md`
- `docs/investigations/win-replay-triage/`: `mismatch-161.tsv`(1 件 1 行。api、case id、runtime の値、実機の A/B の値、安定性、分類、根拠、修正案、確認方法)、`api-class-matrix.tsv`(API × 分類)、`reference-55.tsv`、`real-only-usable.tsv`(API 別)、`real-only-cases.tsv`(ケース別 1,576 行)、`a2b-171.tsv`(171 種の行き先)、`fix-plan.md`、`next-issue-draft.md`
- 補助物(自分用。`lift/out-triage/`): 集計スクリプト(`build_mm.py`、`build_ro.py`、`build_ref.py`、`a2b_calc.py`、`detail.py`)、Learn の本文(`learn/`、`sdkmd/`)、`detail.tsv`(2,144 ケースの判定の再導出)
