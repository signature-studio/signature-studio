# ホスト実行ファイル（FL64.exe）と FLEngine_x64.dll の境界（静的分析）

実行日: 2026-10-28。基準 HEAD e071fc0。**静的分析のみ**（インストーラから抽出した PE を pefile と capstone で読んだ。何も実行していない。lift、runtime、生成器、既存文書は編集していない）。この文書は何も決めない。作業物は `docs/investigations/host-engine-boundary/`（TSV 1 本と、使った補助スクリプト 12 本）。展開した製品ファイルは RAM 上の `/tmp/opencode/host/` に置き、git と `/` には入れていない。

## 0. 要約

1. **ホスト EXE は `FL64.exe`**（インストールルート直下。597,064 B、MSVC 製の GUI exe、署名あり）。元のファイル名は `FL.exe`（版情報）。DPI の扱いだけが違う `FL64 (scaled).exe` が同サイズで同梱される（実測。§2、§3）。
2. **ホストはとても薄い**。ホスト固有のコードは `.text`（361 KB）のうち先頭の 26 KB 前後（概算。残りは MSVC の CRT と STL と読む）で、ホストがやるのは「Media Foundation の有無確認 → エンジン DLL の場所決め → （引数なしなら）エンジン選択ダイアログ → エンジンの Authenticode 検査 → `LoadLibraryW` → `GetProcAddress("CreateFruityInstance")` → 呼ぶ → 戻ったら返り値のオブジェクトの仮想関数スロット 0 を呼ぶ → `FreeLibrary`」だけ（実測 + 読み。§5）。ウィンドウ、メッセージループ、音声、COM、プラグイン、ライセンス、更新確認はホストに無い（FL64.exe の import は 6 DLL 112 名で、USER32 は `MessageBoxA` 1 つ。§3）。
3. **FLEngine_x64.dll の export は 3 個**。ホストが使うのは `CreateFruityInstance`（序数 3、RVA 0xf75990、VA 0x1375990）**1 つだけ**。残り 2 個（`__dbk_fcall_wrapper` 序数 2、`dbkFCallWrapperAddr` 序数 1）は Delphi 系のデバッガ用フックの名前で、FL64.exe に文字列が無い（実測）。ホストはこれを静的 import せず、`GetProcAddress` で名前を引く（実測）。
4. **`CreateFruityInstance(rcx=0xC0, rdx=0xEDE95A, r8=ホストオブジェクト, r9=&インスタンス)`**。rcx と rdx はエンジン側の関数内で**読まれない**（実測）。**この呼び出しは、`Flags=0` のとき、エンジンの `TApplication.Run` 相当のループ（メッセージポンプ）を内側で回して、アプリが終わるまで戻らない**（読み。§6.4）。
5. **FLEngine → ホストの呼び出し: 有**。ただし道は「ホストが渡すオブジェクト 1 個」だけ。仮想関数 3 スロット（スロット 0 = `Dispatch(int,int,int)`、スロット 1 = 何もしない、スロット 2 = `Dispatch(intptr,intptr,intptr)`）と、データ 7 フィールド（版 = 1、フラグ = 0、`+0x10` = 0 の 8 バイト、製品名 "FL Studio"、空文字 2 つ、ベースディレクトリ）（実測。§6）。FL64.exe は export を持たず（実測）、FLEngine の import に FL64.exe 由来の名前は無い（実測）。エンジンがホストの export を引く道は無い。
6. **ブラウザ化への含意（決めない）**: ホストを JS 側で置き換えるのは、**ホストの仕事の量から見て軽い**（§9）。重いのはホストの外、`CreateFruityInstance` の内側（user32、gdi32 の描画とメッセージ、wintrust と crypt32 による自己検査、`TApplication.Run` のブロッキングループ）。FL64.exe 自体を lift する必要は薄い（読み）。判断に必要な未解明点は §10。

## 1. 凡例と手順

- **実測** = 今回のコマンドの出力（ファイル、命令、数値）。**読み** = 逆アセンブルと文字列から読んだ意味づけ（実行していない）。**未確認** = 調べていない、または決められない。**引用** = 既存文書の記述（再測定していない）。
- インストーラ: `/home/ubuntu/ダウンロード/flstudio_win64_26.1.7.5653.exe`（1,106,033,456 B、SHA-256 `04393e7946c7369aaa5cd9ce1c755803e922146289e1ae3263629a44f4e5fc02`。今回 `sha256sum` で再確認、一致）。読み取りのみ。
- 手順（`/tmp/opencode/host/` は RAM 上の作業場所。作業中の最大は約 29 MB）:
  1. `tools/extract_nsis2.py INSTALLER --list ""` → 27,574 件の extract エントリ（`list-all.txt`）。名前は**ファイル名だけ**でディレクトリは付かない。
  2. 配置先は、NSIS の命令列（`EW_CREATEDIR` で SetOutPath のもの）を `extract_by_entry.py` と同じ読み出し部で辿り、各 extract エントリの直前の SetOutPath 文字列を取った（`pe_inventory_nsis.py`）。
  3. 個別ファイルは `extract_nsis2.py --hash` でサイズと SHA-256 を見てから、エントリ番号指定で書き出した（`extract_by_entry.py write ENTRY OUT`。`AudioRestore.dll` は名前の末尾一致が `iZAudioRestore.dll` と衝突するのでエントリ番号を使った）。書き出したのは FL64.exe（948）、FL64 (scaled).exe（946）、FLMManaged.dll、AudioRestore.dll、FLMManagedBridge_x64.dll、WebView2Loader.dll、speaker_x64.dll、FL Studio VSTi.dll（57716）、Fruity Balance_x64.dll（43168）、ILTools_x64.dll（56466）。FLEngine_x64.dll は `binaries/win64/` の既存コピー（SHA-256 が installer のエントリ 950 と一致）を読んだ。
  4. 306 件の PE（NSIS のインストーラ用プラグイン 6 本を除く）を、ディスクに書かずにメモリへ展開して、SHA-256 と PE ヘッダ、export、import を採った（`pe_inventory_nsis.py` → `inventory_to_tsv.py` → `installer-pe-inventory.tsv`。cmp で再生成が一致することを確認）。
  5. FL64.exe は全 `.text` を線形に逆アセンブルして（`disasm_all.py`、約 70,100 行）、アドレス範囲で切り出して読んだ（`dump_lines.py`）。関数境界は `.pdata`（`function_extent.py`）。FLEngine は、`rip` 相対参照の索引（`rip_xref.py`、約 70 秒、5,022,299 命令）、範囲の逆アセンブル（`disasm_range.py`。IAT 名と、参照先のポインタ値を付ける）、関数内の UTF-16 文字列参照（`function_strings.py`）、Delphi の import 用 `jmp [IAT]` の薄い関数 832 個への `call` の数え上げ（`thunk_callers_scan.py`。§4.3 の表を再現する）。
  6. Authenticode の証明書は、`IMAGE_DIRECTORY_ENTRY_SECURITY` の PKCS#7 を `openssl pkcs7 -print_certs` で読んで、埋め込まれた証明書の subject と issuer だけを見た。**署名の検証（チェーン、ハッシュ）はしていない**。
- 逆アセンブルの抜粋は §5.4 に計 33 行（3 箇所、各 30 行以内）。命令の用途を示す短い要約は表に入れた。製品ファイルそのものや機械語の長い抜粋は記録に入れていない。

## 2. 構成表（実測）

PE 306 件の全一覧（名前、配置、サイズ、SHA-256、種別、機種、ImageBase、署名ディレクトリと TLS ディレクトリの有無、export 数と先頭 4 名、import 集合の ID、文字列ヒット）は `host-engine-boundary/installer-pe-inventory.tsv`（91 KB）。以下は境界に関わるものだけ。サイズ、SHA-256（先頭 8 桁…末尾 4 桁。全文は TSV）、依存 DLL（遅延 import は「遅延」）。配置の `<INSTDIR>` はインストールルート（NSIS の変数 21）、`<R>` は NSIS の変数 31（`$_OUTDIR`。インストールルートと読む。未確認）。

### 2.1 ホスト、エンジン、共有ライブラリ

| 名前（エントリ） | 配置 | サイズ B | SHA-256 | 種別 / 機種 / ImageBase | 依存 DLL | export |
|---|---|---:|---|---|---|---|
| **FL64.exe**（948） | `<INSTDIR>` | 597,064 | `4c839f24da13885fef7190d81c792fe2a2e0248dcfce9cb5f400a48ae0520c65` | EXE / x64 / 0x140000000 / GUI | KERNEL32、USER32、ADVAPI32、SHELL32、WINTRUST、CRYPT32（CRT は静的リンク） | 0 |
| FL64 (scaled).exe（946） | `<INSTDIR>` | 597,064 | `0ab91f0f1e947ec0237046b7c03e9155b2b1dc195b872d32e93b21df05a2e979` | 同上（差は §3.4） | 同上 | 0 |
| **FLEngine_x64.dll**（950） | `<INSTDIR>` | 22,227,016 | `b2c64da40921cf262c15c5b25c2b0422e4d1851e953c87e0bc38ed2c19f9496d` | DLL / x64 / 0x400000 | 通常 24 DLL（738 名）、遅延 14 DLL（§4.3） | 3 |
| ILTools_x64.dll（56466） | `<R>\Shared` | 3,662,336 | `20409dcb…0f95` | DLL / x64 / 0x400000 | winmm、comctl32、shell32、user32、version、oleaut32、advapi32、IMAGEHLP、kernel32、SHFolder、ole32、msacm32、CRT string、gdi32。遅延 10 | 7（`ShowEngineChoice` を含む。§5.3） |
| Blend2D.dll（56450） | `<R>\Shared` | 2,065,920 | `66634d24…66c7` | DLL / x64 / 0x180000000 / TLS あり | KERNEL32 | 1 |
| QuickFontCache_x64.dll（56472） | `<R>\Shared` | 2,067,968 | `afc5a0ed…36d2` | DLL / x64 / 0x400000 | winmm、kernel32、shell32、ole32、user32、oleaut32、advapi32、gdi32。遅延 4 | 10 |
| freetype_x64.dll（56490） | `<R>\Shared` | 607,032 | `6ea6c106…7cb8` | DLL / x64 / 0x180000000 / 署名あり | KERNEL32 | 271 |
| dsp_ippv2_x64.dll（56484） | `<R>\Shared` | 39,942,656 | `497dd4f7…c352` | DLL / x64 / 0x180000000 | KERNEL32 | 446 |
| ILSvg_x64.dll（56464） | `<R>\Shared` | 1,504,256 | `92f76fef…2ba0` | DLL / x64 / 0x180000000 / TLS あり | KERNEL32 | 3 |
| ildsp_x64.dll（56492） | `<R>\Shared` | 4,197,376 | `f4b23b0f…aea2` | DLL / x64 / 0x180000000 | KERNEL32 | 10 |
| oggio_x64.dll（56494） | `<R>\Shared` | 1,872,384 | `b26c544f…66f9` | DLL / x64 / 0x180000000 | KERNEL32 | 9 |
| wavpackdll_x64.dll（56500） | `<R>\Shared` | 271,672 | `3bd37cc9…1bad` | DLL / x64 / 0x180000000 / 署名あり | KERNEL32 | 48 |
| LAMEenc_x64.dll（56468） | `<R>\Shared` | 465,720 | `0703adcd…0843` | DLL / x64 / 0x180000000 / 署名あり | KERNEL32、USER32、ADVAPI32 | 9 |
| beat_tracker.dll（56480） | `<R>\Shared` | 983,552 | `43942be8…f4e0` | DLL / x64 / 0x180000000 / TLS あり | KERNEL32 | 14 |
| ILVideo_x64.dll（56724） | `<R>\Shared\ffmpeg\x64` | 1,329,152 | `82ab8521…9c55` | DLL / x64 / 0x400000 | ffmpeg 系 6 DLL（avcodec-58 ほか）、kernel32、user32、oleaut32、advapi32 | 26 |
| python312.dll（56692） | `<R>\Shared\Python` | 7,003,928 | `0cc4e557…b6f2` | DLL / x64 / 0x180000000 / 署名あり / TLS あり | VCRUNTIME140、api-ms-win-crt 系、bcrypt ほか | 1,696 |
| WebView2Loader.dll（954） | `<INSTDIR>` | 137,120 | `f4839ab5…5bb2` | DLL / x64 / 0x180000000 / 署名あり / TLS あり | KERNEL32。遅延 4 | 4 |
| FLMManaged.dll（952） | `<INSTDIR>` | 9,294,848 | `6c6cea4a…c892` | DLL / **x86 ヘッダ（0x14c）/ mscoree のみ import** = .NET のマネージド DLL と読む | mscoree.dll | 0 |
| speaker_x64.dll（960） | `<INSTDIR>` | 3,263,488 | `fe745134…6f81` | DLL / x64 / 0x400000 | winmm ほか（ILTools と同じ系統）。遅延 11 | 7（`ZXSpeak*` ほか） |
| AudioRestore.dll（942）/ iZAudioRestore.dll（958） | `<INSTDIR>` | 953,976 / 2,481,784 | `203834a8…b4a2` / `ca28db0c…37a3` | DLL / **x86（0x14c）** / 署名あり | 標準の Windows DLL | 2 / 1 |
| ILMinihostBridge64.exe（56458）、ILPluginScanner64.exe（56460） | `<R>\Shared` | 3,153,720 / 3,207,480 | `fa567939…7401` / `861c04d4…17de` | EXE / x64 / 署名あり / TLS あり | 標準の Windows DLL のみ | 0 |
| ILWASAPI2ASIO_x64.dll（26） | `<V38>`（未同定） | 531,456 | `ddaabd91…696d` | DLL / x64 / 0x180000000 | WINMM、AVRT、KERNEL32、USER32、ADVAPI32、SHELL32、ole32 | 5（`DllGetClassObject`、`DllRegisterServer` など） |

読み: `ILWASAPI2ASIO_x64.dll` は COM サーバー形式の DLL（`DllGetClassObject`、`DllRegisterServer`）で、ASIO ドライバーとして登録される部品と読める（未確認。エンジンの文字列に `ILWASAPI2ASIO` は無い）。

### 2.2 エンジンを呼ぶ他のホストと、エンジンが呼ぶもの

`CreateFruityInstance` の文字列を含む PE は、インストーラ中の 306 件のうち **FL64.exe、FL64 (scaled).exe、FLEngine_x64.dll 自身、`FL Studio VSTi.dll`（57716）、`FL Studio VSTi (Multi).dll`（57714）、`FLReWire_x64.dll`（57702）の 6 件だけ**（実測。文字列検索）。FL64.exe（と scaled 版）のほかに、VSTi 2 本と ReWire 1 本の計 3 本が、FLEngine を読み込む側と読める。VSTi と ReWire を詳しくは分析していない（§8）。

| 区分 | 件数 | 配置 | 備考（実測） |
|---|---:|---|---|
| Fruity エフェクト | 112 | `Plugins\Fruity\Effects\<名前>\` | すべて x64 DLL |
| Fruity ジェネレータ | 69 | `Plugins\Fruity\Generators\<名前>\` | x64 DLL 68、x86 DLL 1 |
| 上の 2 区分のうち `CreatePlugInstance` を export するもの | 159 | | 他の 22 は補助 DLL（`engine_x64.dll` など） |
| 上の 2 区分のうち `__dbk_fcall_wrapper` / `dbkFCallWrapperAddr` を export するもの | 147（全体） | | FLEngine と同じ系統の RTL を持つ（実測。export 名） |
| FL Studio VSTi | 2 | `System\Plugin\VSTi\x64` | `main` と dbk 2 個を export |
| ReWire | 2 | `System\Plugin\ReWire` | `FLReWire_x64.dll`（23 export）、`ReWire.dll`（68 export） |
| ツール EXE | 十数 | `System\Tools\…` | `ilbridge.exe`（32/64 ビット）、`fldiagnostic.exe`、`PluginManager.exe`、`ControlCreator.exe`、`CopyLicenseToAllUsers.exe`、`RemoveLicenses.exe` ほか |
| LilyPond（楽譜出力）一式 | 34 | `System\Tools\LilyPond\bin` | x86 の DLL と EXE |
| Python 一式 | 31 | `Shared\Python` | x64。python312.dll と `.pyd` |
| ffmpeg 一式 | 8 | `Shared\ffmpeg` | x64 |

- インストーラ中の PE の合計は 306 件、約 1.06 GB（実測）。プラグイン DLL は 3.5〜7 MB のものが多い（最小 45,056 B、最大 27,128,048 B）。**プラグインは各自が Delphi の RTL と共通コードを持つ別の DLL** で、エンジンとは `TFruityPlugHost` 系のインターフェース（エンジンが `LoadLibraryW` して `GetProcAddress("CreatePlugInstance")` で引く。エンジンの UTF-16 文字列に `CreatePlugInstance` が 2 つ、周辺は「…is missing」）でつながる（読み）。プラグイン側が FLEngine を呼ぶ道は、今回は調べていない。
- プラグインの DLL にも `FL Studio engine`、`FLEngine_x64.dll` の文字列がある（`Fruity Balance_x64.dll` で確認。レジストリ `Software\Image-Line\Shared\Paths` を読む共通コードの断片）。**プラグインがエンジンに依存しているわけではない**（読み。import に FLEngine は無い）。

## 3. FL64.exe の PE（実測）

### 3.1 ヘッダ

| 項目 | 値 |
|---|---|
| Machine / Characteristics | 0x8664 / 0x22（実行可能、LARGE_ADDRESS_AWARE） |
| TimeDateStamp | 0x6abbb846（2026-09-29 13:08:22 UTC）。FLEngine は 0x6abbb7e3（13:06:43）で、約 1 分半前 |
| Linker | 14.29（MSVC。RTTI あり、CRT 静的リンク） |
| ImageBase / SizeOfImage / EntryPoint | 0x140000000 / 0xa6000 / RVA 0x30ed4 |
| Subsystem | 2（Windows GUI） |
| DllCharacteristics | 0x8140（DYNAMIC_BASE、NX_COMPAT、TERMINAL_SERVER_AWARE）。`.reloc` あり |
| スタック / ヒープ（予約、コミット） | 0x100000、0x1000 / 0x100000、0x1000 |
| セクション | `.text`（0x58400）、`.rdata`（0x17442）、`.data`（0x13304、うち初期化済み 0x1c00）、`.pdata`（0x3450 = 1,116 関数）、`_RDATA`、`.rsrc`（0x19310）、`.reloc` |
| データディレクトリ | import（0x7076c、0x8c）、resource、exception、**security（file offset 0x8f400、0x2848 B）**、base reloc、debug、load config（0x138）、IAT。**export なし、TLS なし、delay import なし**（`dd[0]`、`dd[9]`、`dd[13]` が空） |

### 3.2 import 表（6 DLL、112 名）

| DLL | 件数 | 主な名前 |
|---|---:|---|
| KERNEL32 | 98 | `LoadLibraryW`、`GetProcAddress`、`FreeLibrary`、`GetModuleHandleW`、`GetModuleFileNameW`、`SetCurrentDirectoryW`、`GetFileAttributesW`、`LocalAlloc`、`LocalFree`、`lstrcmpA`、`GetStartupInfoW`、`LoadLibraryExW`（CRT 用）、そのほか CRT の始動、ロケール、ファイル、TLS/FLS、例外 |
| USER32 | 1 | `MessageBoxA` |
| ADVAPI32 | 3 | `RegOpenKeyExW`、`RegQueryValueExW`、`RegCloseKey` |
| SHELL32 | 1 | `ShellExecuteA` |
| WINTRUST | 1 | `WinVerifyTrust` |
| CRYPT32 | 8 | `CryptQueryObject`、`CryptMsgGetParam`、`CryptMsgClose`、`CertFindCertificateInStore`、`CertGetNameStringA`、`CertFreeCertificateContext`、`CertCloseStore`、`CryptDecodeObject` |

CreateWindow、GetMessage、waveOut、CoInitialize、CreateFile（CRT 以外）、CreateThread など、GUI アプリやオーディオアプリに普通ある API が**無い**（実測）。

### 3.3 リソース、署名

- リソース（実測）: `RT_ICON` 13、`RT_GROUP_ICON` 1（id 102。**名前 `MAINICON` ではない**）、`RT_VERSION`、`RT_MANIFEST`。版情報: CompanyName `Image-Line`、FileDescription `FL Studio`、FileVersion / ProductVersion `26.1.7.5653`、InternalName `FL`、OriginalFilename `FL.exe`、LegalCopyright `1998-2026 by Image-Line`。マニフェスト: assemblyIdentity `ImageLine.FLStudio.FLExe` 26.1.7.0 amd64、Common-Controls 6.0.0.0、`dpiAware` = `True/PM`、`longPathAware`、supportedOS 4 件。
- 一方 **FLEngine_x64.dll の `.rsrc` には `RT_GROUP_ICON` の `MAINICON` がある**（実測。`RT_ICON` 13、`RT_CURSOR` 60、`RT_BITMAP` 25、`RT_RCDATA` 128 など、約 2 MB）。つまり UI のリソースは**エンジン側**にある。これは `hinstance-caller.md` §8（限界）の未解明点「ホスト（FL64.exe）側の MAINICON の有無」への答え: **FL64.exe に `MAINICON` は無い**。
- 署名（実測。**検証はしていない**）: FL64.exe と FLEngine_x64.dll の埋め込み証明書は、どちらも subject が `C=BE, L=Gent, O=Image Line NV, CN=Image Line NV`、issuer が `DigiCert Trusted G4 Code Signing RSA4096 SHA384 2021 CA1`（その上が `DigiCert Trusted Root G4`）。

### 3.4 `FL64 (scaled).exe` との差

597,064 B で同サイズ。差のあるバイトは計 7,055（ヘッダ 6、`.rdata` 3、`.rsrc` 814、署名ブロック 6,232）。`.text`、`.data`、`.pdata`、`.reloc` は**同一**（実測）。マニフェストの差: scaled 版は `requestedExecutionLevel asInvoker` を持ち、`dpiAware` を持たない（= システム DPI のまま、OS が拡大する版と読む）。コードは同じ。エンジンの UTF-16 文字列に `FL.exe`、`FL64.exe`、`FL64 (Scaled).exe` が並ぶ（FLEngine の file offset 0x251f28 付近。用途は未確認）。

## 4. FLEngine_x64.dll の口（実測）

### 4.1 export 表（3 個、すべて名前付き）

| 序数 | 名前 | RVA | VA | 区分 | 用途の読み |
|---:|---|---|---|---|---|
| 3 | `CreateFruityInstance` | 0xf75990 | 0x1375990 | .text（関数、3,213 B。`.pdata` に境界あり） | **エンジンの本体の入口**。FL64.exe、VSTi、ReWire が `GetProcAddress` で引く（§5、§8） |
| 2 | `__dbk_fcall_wrapper` | 0x1d5a0 | 0x41d5a0 | .text（関数、826 B） | Delphi の RTL にあるデバッガ用のフック（名前から。読み）。FL64.exe は使わない |
| 1 | `dbkFCallWrapperAddr` | 0x11ec2b0 | 0x15ec2b0 | .bss（データ変数） | 同上。データなので関数としては呼べない（`closure-count.md` も同じ扱い） |

- エントリポイント（`AddressOfEntryPoint`）= RVA 0xf76820 = VA 0x1376820（115 B）。`DllMain` で、`InitUnits` を呼ぶ（`0x41d8f0`。InitTable 0x13768b0、件数 942）。**これは `LoadLibraryW` が自動で呼ぶもので、ホストが「呼ぶ」口ではない**。A4（InitUnits の完走）が測ったのはここまで。
- `HInstance` は `[0x15ec298]`（InitUnits が DllMain の hinstDLL を保存。`hinstance-caller.md` で実測済み）。`MainInstance` は `[0x15e6078]`（DLL では 0）。
- export は空ではないが、ホストから見える口は実質 `CreateFruityInstance` の 1 つ。他の口は、window class（エンジンが自分で登録する）、COM（`CoCreateInstance` はエンジンが 14 箇所で使う側）、named pipe（文字列 `\\.\pipe` が無い）、mutex（`Mutex` の文字列、`CreateMutex` の import が無い）、共有メモリ（`OpenFileMappingW`、`CreateFileMappingW` はあるが、ホストの側に無い）のいずれでも、ホストとの間では見つからなかった（実測。文字列と import の有無。エンジン同士や他プロセスとの IPC に使うかは未確認）。

### 4.2 エンジンの import に FL64.exe 由来の名前は無い

通常 import 24 DLL（738 名）に `FL64.exe` も `FL.exe` も無い（実測）。`GetModuleHandleW` は 37 箇所（31 関数）で呼ばれるが、引数が文字列定数のもの（kernel32、user32、ole32、comctl32、wininet、NTDLL など）と、`NULL` を渡す 1 箇所（`0x8eb0b9`）がある。`NULL` の箇所は Delphi の RTL にある汎用の動的リンクの補助（モジュール名と関数名のペアを引く）と読める（未確認）。**ホストの export を引く道は無い**（FL64.exe に export が無いことが先に効く）。

### 4.3 import の概要（件数は `win64-extraction.md` の 738 と一致）

通常 import: user32 227、kernel32 184、gdi32 127、comctl32 30、ole32 29、winmm 24、shell32 22、oleaut32 19、wininet 15、advapi32 14、msacm32 14、crypt32 7、winspool 5、mpr 4、version 3、IMAGEHLP 3、opengl32 2、comdlg32 2、dsound 2、shlwapi 1、wintrust 1、SHFolder 1、oleacc 1、api-ms-win-crt-string 1（計 738）。遅延 import（17 記述子、14 DLL）: kernel32、user32、wtsapi32、msimg32、advapi32、rpcrt4、propsys、ole32、shell32、windowscodecs、uxtheme、imm32、DWMAPI、Shcore。

エンジン自身が呼ぶ「ホスト的な」API（Delphi の `jmp [IAT]` の薄い関数 832 個への `call` を数えた。`jmp` で飛ぶ呼び方は数えていないので**下限**）:

| API | 呼び出し箇所 | 関数数 |
|---|---:|---:|
| `CreateWindowExW` | 9 | 8 |
| `RegisterClassW` | 8 | 8 |
| `PeekMessageW`（`HandleMessage` 経由）、`TranslateMessage`、`DispatchMessageW` | メッセージポンプ 1 箇所（0x86a0f0）で確認 | |
| `MessageBoxW` / `MessageBoxA` | 7 / 1 | 7 / 1 |
| `WinVerifyTrust` | 4 | 1（`CreateFruityInstance` のみ） |
| `LoadLibraryW` / `LoadLibraryA` / `LoadLibraryExW` | 48 / 1 / 1 | 40 / 1 / 1 |
| `CoCreateInstance` | 14 | 14 |
| `ShellExecuteW` / `ShellExecuteExW` / `ShellExecuteA` | 17 / 3 / 1 | 17 / 3 / 1 |
| `CreateProcessW` | 2 | 2 |
| `InternetOpenW` / `InternetOpenUrlW`（wininet） | 3 / 2 | 3 / 2 |
| `DirectSoundCreate` | 2 | 1 |
| midiIn / midiOut（winmm の 20 名） | 薄い関数は 0x663310〜0x663440 に連続 | |

## 5. ホストの動き（FL64.exe。実測の命令 + 読み）

### 5.1 起動順

アドレスは FL64.exe の VA（ImageBase 0x140000000）。`WinMain` = 0x140007560、エンジンを探して呼ぶ本体 `Run` = 0x1400062a0（3,573 B）。

| # | 動作 | 根拠（実測） | 区分 |
|---:|---|---|---|
| 0 | CRT のエントリ 0x140030ed4 → `__scrt_common_main_seh` → `WinMain(hInstance=0x140000000, 0, lpCmdLine, nShow)` | 0x140030e5a〜0x140030e61 | 実測 + 読み |
| 1 | `LoadLibraryW(L"Mfplat.dll")` → `GetProcAddress("MFStartup")` → `MFStartup(0x20070, 1)`（= `MF_VERSION`、`MFSTARTUP_LITE`）→ `FreeLibrary`。戻りが `E_NOTIMPL`（0x80004001）、または DLL が無ければ、`MessageBoxA`（OK / キャンセル、エラーアイコン）で「FL Studio requires the Windows Media Feature Pack. Click OK to show instructions…」→ OK なら `ShellExecuteA` でサポートの URL（`https://support.image-line.com/redirect/kb_windows_n…`）を開いて終了。**`MFShutdown` は呼ばない**（文字列が無い） | 0x140007597〜0x1400075d6、0x1400077dd〜0x14000781e | 実測。意味は Learn の `MFStartup` ページ（E_NOTIMPL = メディアコンポーネントが無い）と合う |
| 2 | ホストオブジェクト（0x38 B）を `operator new` で確保して初期化（§6.1） | 0x1400075e5〜0x140007649 | 実測 |
| 3 | コマンドライン（ANSI）を UTF-16 に変換（`std::wstring_convert<codecvt_utf8_utf16>`）して `Run` に渡す | 0x140007671〜0x14000768e | 読み（RTTI に `codecvt_utf8_utf16`） |
| 4 | **エンジン DLL の場所決め**: 既定名 `L"FLEngine_x64.dll"`（グローバルの静的 `std::wstring`。引数の「上書き名」が空なら）。`GetModuleFileNameW(NULL)` で自分の exe のパス → ディレクトリ部 + 名前。`GetFileAttributesW` で実在しない（または ディレクトリ）なら、**レジストリ `HKCU`（`KEY_READ` = 0x20019）、次に `HKLM`（0x20219 = `KEY_READ \| KEY_WOW64_32KEY`）の `Software\Image-Line\Shared\Paths` の値 `FL Studio engine`** を読み、得た文字列から作ったパスへ `SetCurrentDirectoryW` し、そのディレクトリ + 既定名を新しいエンジンのパスにする形 | 0x1400062f0〜0x140006519、`0x140005000`（レジストリ）の 0x1400050c3〜0x14000527e | 実測（命令と定数）+ 読み（値の意味） |
| 5 | エンジンのディレクトリ（末尾の区切り付き）を取って、ベースディレクトリとして取っておく（後でホストオブジェクトの +0x30 へ） | 0x1400065c4〜0x140006602、`0x140001330` = `find_last_of(L"\\/")` + `substr` | 読み |
| 6 | **コマンドラインが空のときだけ**エンジン選択: `Shared\ILTools_x64.dll` のパスを作り（`0x140005480`。exe のディレクトリ + `Shared\` + `ILTools_x64.dll`。見つからなければレジストリの値 `Shared DLLs` を使う形）、エンジンのパスのディレクトリに **`Builds\` ディレクトリがあれば** `LoadLibraryW(ILTools)` → `GetProcAddress("ShowEngineChoice")` を、32 KB のバッファ（エンジンのパスを変換した文字列）を渡して呼ぶ。真が返ればバッファの文字列を新しいエンジンのパスにする。偽（キャンセル）なら空のパスになり、`Run` は −3（0xfffffffd）を返して静かに終わる。`Builds\` が無い、または ILTools が無い場合は、パスは変わらない。ILTools の文字列に `Choose engine`、`Builds\`、`FL Studio engine`、`FLEngine_x64.dll` がある。引数が与えられていればこの段は飛ばす | 0x140006648〜0x140006757、`0x140005d80`（0x140005f55 `LoadLibraryW`、0x140005f7e `GetProcAddress`、0x140005fcd `call r15`）。早期脱出はすべて入力パスのコピーを返す（0x140006197、0x1400061ea、0x140006230） | 実測（命令の流れと文字列）。ダイアログの見た目は未確認 |
| 7 | **Authenticode 検査**: `WinVerifyTrust`（アクション GUID = `WINTRUST_ACTION_GENERIC_VERIFY_V2` {00AAC56B-CD44-11D0-8CC2-00C04FC295EE}、UI なし、ファイル指定）→ `CryptQueryObject` / `CryptMsgGetParam` で署名者情報を取り、`CertFindCertificateInStore` + `CertGetNameStringA`（名前の種類 4 = 表示名）で署名者名を取り、**`L"Image Line NV"` と比較**。失敗すれば −2（0xfffffffe）。特定の `WinVerifyTrust` のエラー値（0x80092026、0x800b0004、0x800b0100、0x800b0111）を個別に扱う分岐がある | 0x140006788〜0x140006e6a、定数 0x140069b80（`Image Line NV`） | 実測（命令と定数）+ 読み（比較の細部） |
| 8 | **ロード**（`0x140004b60`）: `GetModuleHandleW(パス)` が非 NULL（= すでに同じ DLL が読み込まれている）なら、ファイル名の拡張子の前に **`_Copy`** を挟んだ名前（`FLEngine_x64_Copy.dll`）があればそれを使い、無ければ元のパス → **`LoadLibraryW(パス)`**。この時点で `DllMain` が走る（= `InitUnits`。A4 が測ったところ） | 0x140004bc9、0x140004e4f、0x140004f33 | 実測 + 読み |
| 9 | ベースディレクトリ（UTF-8）を 32 KB のグローバルバッファ（0x1400752f0）に `strcpy_s` | 0x140006f05〜0x140006f0c | 実測 + 読み |
| 10 | **`GetProcAddress(hEngine, "CreateFruityInstance")` → `CreateFruityInstance(0xC0, 0xEDE95A, host, &instance)`** | 0x140006f11〜0x140006f37（`edx = 0xede95a`、`ecx = 0xc0`、`r8` = ホスト、`r9` = 出力先） | 実測 |
| 11 | 戻り値が 0 または 2 なら **`instance->vtable[0](instance)`**（§6.4。スロット 0 はデストラクタ相当と読む）。−2 なら `MessageBoxA`（「Verification error」/「The validity of the program could not be verified. Please reinstall it and try again.」）。−1 や −3、その他は何も表示しない | 0x140007789〜0x1400077bb | 実測 |
| 12 | `FreeLibrary(hEngine)`（DLL_PROCESS_DETACH）、ホストオブジェクトを `delete`、**`WinMain` は常に 0 を返す**（エンジンの戻り値を終了コードにしない） | 0x1400077c4〜0x140007824 | 実測 |

戻り値の意味（読み）: −1 = エンジンのロード失敗か `GetProcAddress` 失敗（初期値）、−2 = 署名検査の失敗、−3 = 選択のキャンセル。エンジン側の戻り値は §6.4。

`GetProcAddress` に渡す名前の文字列は、`MFStartup`（0x1400699e8）、`ShowEngineChoice`（0x140069b68）、`CreateFruityInstance`（0x140069ba0）の 3 つだけ（実測。FL64.exe の自前の `GetProcAddress` 呼び出しは 0x1400075b3、0x140005f7e、0x140006f1b の 3 箇所で、残る 0x14003667b は CRT 内）。`LoadLibraryW` は 0x140007597（Mfplat）、0x140005f55（ILTools）、0x140004f33（エンジン）の 3 箇所。

### 5.2 ホストの擬似コード（読み。抜粋を含まない要約）

```
WinMain(cmdline):
  if (!MFStartup 可能) { MessageBox + URL; return 0 }
  host = new THostInstance{ vtbl, version=1, flags=0, name="FL Studio", "", "", baseDir=g_buf }
  r = Run(host, &inst, &hEngine, 上書き名="", cmdline)
  if (r == 0 || r == 2) inst->vtable[0](inst)      // 片付け（デストラクタ相当と読む）
  else if (r == -2)     MessageBox("Verification error")
  FreeLibrary(hEngine); delete host; return 0

Run(...):
  path = exeDir + (上書き名 or L"FLEngine_x64.dll"); 無ければレジストリ "FL Studio engine"
  if (cmdline.empty()) path = ShowEngineChoice(ILTools, path)    // Builds\ があるとき
  if (!AuthenticodeOK(path, signer == L"Image Line NV")) return -2
  hEngine = LoadLibraryW(path)       // → DllMain → InitUnits
  host->baseDir = narrow(dirname(path))
  return GetProcAddress(hEngine,"CreateFruityInstance")(0xC0, 0xEDE95A, host, &inst)
```

### 5.3 `ShowEngineChoice` は ILTools の export（実測）

ILTools_x64.dll の export 7 個: `CreateCursorFromFont`、`CreatePathManagerInstance`、`CreatePluginPathManagerInstance`、`DestroyPathManager`、`ShowEngineChoice`、`__dbk_fcall_wrapper`、`dbkFCallWrapperAddr`。FL64.exe 以外で `ShowEngineChoice` を含む PE は ILTools 自身だけ（文字列検索）。FLEngine は ILTools を `CreateCursorFromFont` などのために別途 `LoadLibraryW` する（`bundled-dll-inventory.md` の引用）。**ILTools はホストの検査（署名の確認）の対象ではない**（`sig=N`。ホストは署名を見ずに `LoadLibraryW` する）。

### 5.4 抜粋（FL64.exe。実測。計 33 行）

エンジンの呼び出し（0x140006f11〜）:

```
140006f11: lea rdx, [rip + 0x62c88]       ; 0x140069ba0 = "CreateFruityInstance"
140006f18: mov rcx, qword ptr [rdi]       ; hEngine
140006f1b: call qword ptr [rip + 0x53147] ; GetProcAddress
140006f21: test rax, rax
140006f24: je 0x140006f3c
140006f26: mov r9, qword ptr [rbp]        ; &instance（出力先）
140006f2a: mov r8, r14                    ; ホストオブジェクト
140006f2d: mov edx, 0xede95a
140006f32: mov ecx, 0xc0
140006f37: call rax                       ; CreateFruityInstance
140006f39: mov r15d, eax
```

戻り値の振り分け（0x140007789〜）:

```
140007789: test ebx, 0xfffffffd           ; 0 か 2 か
14000778f: je 0x1400077b2
140007791: cmp ebx, -2                    ; 署名失敗
140007794: jne 0x1400077bb
1400077b2: mov rcx, qword ptr [rbp - 0x21] ; instance
1400077b6: mov rax, qword ptr [rcx]        ; vtable
1400077b9: call qword ptr [rax]            ; スロット 0
```

ホストオブジェクトの初期化（0x1400075e5〜）:

```
1400075e5: mov ecx, 0x38
1400075ea: call 0x1400309bc               ; operator new
1400075ef: mov rdi, rax
1400075fa: xor esi, esi
1400075fc: lea rax, [rip + 0x626fd]       ; 0x140069d00 = THostInstance の vftable
140007615: mov qword ptr [rdi], rax
14000761c: mov qword ptr [rdi + 8], 1     ; +8 = 1（HostVersion）、+0xc = 0（Flags）
140007624: lea rax, [rip + 0x620e5]       ; 0x140069710 = "FL Studio"
14000762b: mov qword ptr [rdi + 0x18], rax
14000762f: lea rax, [rip + 0x621ea]       ; 0x140069820 = ""
140007636: mov qword ptr [rdi + 0x20], rax
14000763a: mov qword ptr [rdi + 0x28], rax
14000763e: lea rax, [rip + 0x6dcab]       ; 0x1400752f0 = ベースディレクトリのバッファ
140007645: mov qword ptr [rdi + 0x30], rax
140007649: mov qword ptr [rdi + 0x10], rsi ; 0
```

## 6. エンジン → ホスト（実測 + 読み）

### 6.1 ホストオブジェクト `THostInstance`（FL64.exe が作り、エンジンへ渡す）

RTTI（実測）: `.?AVTHostInstance@@` の基底クラス一覧に `THostInstance`、`THostComInstance` がある（`THostInstance` が `THostComInstance` を継承。後者は FL64.exe に vftable を持たず、抽象クラスと読む）。vftable は 0x140069d00（3 スロット）。レイアウト:

| オフセット | 型 | FL64.exe が入れる値（実測） | エンジンが読む箇所（実測） |
|---:|---|---|---|
| +0x00 | vtable | 0x140069d00 | 呼び出しは §6.2 |
| +0x08 | dword（HostVersion） | 1 | `CreateFruityInstance` 内 0x1375a46 でログ出力 `HostVersion=%d Flags=%.8x`、0x13755d1（`0x13755b0`）で **1 でなければ**「This version of the %s engine doesn't match the application executable. Please reinstall %s」 |
| +0x0c | dword（Flags） | 0 | §6.3 |
| +0x10 | qword | 0 | 0x13764b4: `Flags` のビット 0 が立つ分岐で `0x869c90`（Application 側のメソッド）の引数に渡す。ウィンドウハンドルのようなものと読む（未確認） |
| +0x18 | char*（PChar） | "FL Studio" | 0x1354c66（`0x1354630`）: `Format` の引数（TVarRec 型 6 = vtPChar）。製品名と読む |
| +0x20 | char* | ""（0x140069820、ゼロ） | 0x1251850（`0x12517f0`）: 文字列に変換して `Format` へ。用途は未確認 |
| +0x28 | char* | "" | 0x117e282（`0x117cd70`）: 文字列に変換して使う。用途は未確認 |
| +0x30 | char* | ベースディレクトリ（UTF-8、末尾が区切り。0x1400752f0） | 0x126219b（`0x1262140`）。下の `GetBaseDir` |

`0x1262140`（実測 + 読み）は `GetBaseDir(var s: string)`: `s := ''; if Host <> nil and Host.Dispatch(3, 1, 0) = 1 and Host.baseDir <> nil and Host.baseDir[0] <> #0 then s := Host.baseDir; if s = '' then s := ExtractFilePath(GetModuleFileNameW(HInstance))`。**ホストが渡さなければ、エンジン自身の DLL のあるディレクトリになる**（ホストの寄与は任意）。

### 6.2 仮想関数スロット（FL64.exe の実装と、エンジンの呼び出し）

FL64.exe の vftable 0x140069d00（実測）:

| スロット | アドレス | 中身（実測の命令） | 読み |
|---:|---|---|---|
| 0 | 0x140007520 | `mov rax,[rcx]; movsxd r9,r9d; movsxd r8,r8d; movsxd rdx,edx; jmp [rax+0x10]` | `Dispatch(int id, int index, int value)` → 引数を 64 ビットに符号拡張してスロット 2 へ |
| 1 | 0x140007530 | `ret 0`（何もしない） | 何もしない通知（エンジンの呼び出しは `edx = int, r8d = int`） |
| 2 | 0x140007540 | `id == 2 → 0`、`id == 3 && index <= 1 → 1`、それ以外 0 | `Dispatch(intptr id, intptr index, intptr value)`。**FL64.exe が 1 を返すのは `id = 3` かつ `index` が 0 か 1 のときだけ** |

エンジン側でホストオブジェクトを使う箇所（セル `0x15e0638` 経由の参照 21 件のうち、設定の 1 件と、読む 20 件。実測）:

| エンジンのアドレス（関数） | 呼び出し・参照 | FL64.exe の応答 |
|---|---|---|
| 0x1375a22（`CreateFruityInstance`） | ホストポインタをグローバル（変数 0x15ce510）に保存 | — |
| 0xffb92c（0xffb600、984 B）、0x104423c（0x1042d20、5,456 B。周辺の文字列は「Linked mixer track」など） | `Host <> nil` なら `Dispatch(0, 0, 0)`（スロット 0） | 0 を返す（効果なし） |
| 0x1193db8（0x1192f80。周辺に「%d underruns」） | `Dispatch(4, 0, round(float [0x15e2b30]))` | 0 |
| 0x121f2cf（0x121f290、71 B） | `Dispatch(1, 0, byte [this+0xa9])` | 0 |
| 0x1262194（0x1262140） | `Dispatch(3, 1, 0) == 1` なら +0x30 を使う（§6.1） | **1** |
| 0x132aaeb（0x132a910） | スロット 1（`edx = int`、`r8d = int`） | 何もしない |

読み: ID 0、1、4 の意味は分からない（周辺のコードは音声エンジンの状況通知、ミキサー、状態の通知で一貫しない）。FL64.exe はすべて 0 を返し、エンジンはその 0 を使って先へ進んでいる（`id = 3, index = 1` 以外の戻り値はエンジン側で読まれていない）と読める（呼び出し元の分岐は 5 箇所のうち 4 箇所で戻り値を使っていない。実測の命令）。

### 6.3 `Flags`（+0x0c）のビット（実測の分岐 + 読み）

FL64.exe は 0（すべて落ちている）。FL Studio VSTi.dll の中には、近傍のコンストラクタが `+8 = 1`、`+0xc = 0xf`（4 ビットすべて）、`+0x10 = ポインタ`、`+0x38 = 所有者`を設定するものがある（0x6a2b70。`CreateFruityInstance` を呼ぶ関数 0x69fe50 と別。**同じオブジェクトかは未確認**）。

| ビット | エンジンでの使われ方（実測の分岐） | 読み |
|---|---|---|
| 0 | 0x1375f09: **立っていなければ**ホスト exe の署名検査（`GetModuleFileNameW(NULL)` のパス）を行う。0x1376421: グローバル `[0x15e3a78]` に保存。0x1375950（Run）: **立っていなければ** `TApplication.Run` 相当、立っていれば別のループ関数（0x11efbf0）。0x11db3fa: 関数の戻り値が `!bit0`。0x13751e4: 立っていなければ追加の初期化 | 「ホストがウィンドウとメッセージを持つ組み込みモード」の旗と読める（プラグイン版が立てる）。FL64.exe（0）がスタンドアロンの経路 |
| 1 | 0x1376444: グローバル `[0x15e01b0]` に保存 | 不明 |
| 2 | 0x1375214: 立っていなければ `0x11b3810` を呼ぶ | 不明 |
| 3 | 0x11ef87a: 立っていなければ、文字列 "Play startup sound" のログ（0x1265690）と 0x92d150 の呼び出しへ進む。関数 0x11ef180 には `ProcessFirstTimeStuff`、`CrashLoadingStartupProject` などの文字列もある | 起動時の処理（起動音など）を抑える旗と読む（未確認） |

### 6.4 `CreateFruityInstance` の中身（0x1375990、3,213 B。実測の呼び出し順 + 読み）

| 順 | アドレス | 内容 |
|---:|---|---|
| 1 | 0x13759e6〜0x13759fb | 引数を退避（rcx、rdx、r8 = ホスト、r9 = 出力先）。**rcx と rdx は退避だけで、以後一度も読まれない**（`[rbp+0x620]`、`[rbp+0x628]` の参照は 2 つの `mov` のみ） |
| 2 | 0x1375a03〜0x1375a3a | `0x663480` = `winmm!timeGetTime`（戻りを `[0x15e18d0]` に保存）。ホストポインタを保存。`0x13752f0`、`0x12658c0(1)`（内容は未読） |
| 3 | 0x1375a46〜0x1375aa8 | ログ `HostVersion=%d Flags=%.8x`、`Init module` |
| 4 | 0x1375ab5〜0x1375bf8 | **自己検査**: `GetModuleFileNameW(HInstance)` → `WinVerifyTrust` → 署名者名が `Image Line NV` か（`CryptQueryObject`、`CryptMsgGetParam`、`CertFindCertificateInStore`、`CertGetNameStringW`）。失敗なら **−2 を返す**。ホストも同じ値で検査していた（§5.1 #7）。**エンジンは自分自身を（`HInstance` = 自分の DLL）検査する** |
| 5 | 0x1375f01〜0x1376329 | `Flags` のビット 0 が 0 なら、同じ検査を **ホスト exe**（`GetModuleFileNameW(NULL)`）に対して行う。失敗なら −2 |
| 6 | 0x1376350〜0x13763b8 | ログ `Init core paths`、`0x1264050`。`0x13754b0`。`[0x15e0698]` のビット 1 なら「Fire edition」のログ |
| 7 | 0x1376409〜0x137645e | `*instance = 0`。`0x13755b0`（HostVersion が 1 でなければメッセージ。`[0x15f32e0+0x12] & 0x10` が 0 なら「This computer's CPU won't run %s」を出して 1 を返す。CPU の機能の確認と読む）。`Flags` からグローバルを設定。`0x11b32a0`（失敗なら「BasicInitialization failed.」）。**`0x11b32a0` の中でインスタンスとその相方のオブジェクトが作られ、グローバル（0x15ce518、0x15ce520）に入る** |
| 8 | 0x1376486〜0x1376545 | アプリケーション（Delphi の `TApplication` 相当 `[0x15f7190]`）の設定: `Flags` のビット 0 が立っていれば `0x86ccd0`、`0x869c90(host+0x10)`、立っていなければタイトルに `FL Studio`、**`LoadIconW(HInstance, L"MAINICON")`**（エンジン自身のリソース）、`0x867e10` ほか |
| 9 | 0x137654a〜0x137655b | **`*instance = [0x15ce518]` の中身**（= 7 で作ったインスタンス） |
| 10 | 0x1376573 | **`0x1375950`（Run）**: `Flags` のビット 0 が 0 なら `0x86a660`（`TApplication.Run` の形: `Running := True` の後、ループで `HandleMessage`（0x86a250）を呼び、`[this+0x138]`（Terminated）が真になるまで回る。`HandleMessage` は `ProcessMessage`（0x86a0f0。`PeekMessageW`、`TranslateMessage`、`DispatchMessageW`）、なければ `Idle`（0x86b440））。**ここでブロックする** |
| 11 | 0x1376586〜0x13765eb | Run の前にグローバル（セル 0x15e4790 の先）のオブジェクトの `+8` の先の `+0x40` を 0 にし、Run から戻ったらその値を戻り値にする（終了コードと読む）。同じオブジェクトの仮想関数 `+0x58` が真なら `0x125d370` を呼ぶ |

FL64.exe が呼ぶ `instance->vtable[0]`（§5.1 #11）: インスタンスのクラス（クラスセル 0x1253c80、VMT 0x1253d48）は、オフセット 0 から数えて仮想関数 20 個（実測）。スロット 0 = 0x12541b0 の動作は、(1) グローバル `[0x15ce518]`（インスタンスの変数）に 0 を書く、(2) 相方のオブジェクト（`[0x15ce520]`）が非 0 ならその `vtable[0]` を呼ぶ、(3) `dl = 1` で `[VMT−0x20]` を呼んで基底の後始末をする。相方のクラス（VMT 0x1253f08）のスロット 0（0x1254840）は逆向きに同じことをする（実測の命令）。VMT の負の側（−0x08〜−0x40）には、両クラスで同じ値のコードアドレスが 8 個並び（`−0x08` = 0x40f9d0 など）、Delphi 64 ビットの `TObject` の 8 個の仮想関数（`Destroy` が −0x08）の配置と合う。したがってスロット 0 は `TObject.Destroy` そのものではなく、**ユーザーが宣言した最初の仮想関数で、デストラクタ相当（相互に参照する 2 つのオブジェクトを片付ける）の動作**と読む（読み。名前のシンボルは無い）。FL64.exe は「走り終わったインスタンスを片付ける」ためにこれを呼ぶ、と読む。スロット 1〜19 を FL64.exe は呼ばない（実測）。VSTi/ReWire が呼ぶかは未確認。

## 7. 責務の推定（読み。根拠付き）

| 責務 | ホスト（FL64.exe） | FLEngine | 根拠 |
|---|---|---|---|
| プロセス起動、WinMain | ○ | — | FL64.exe のエントリ。ただし `hInstance` も `nShow` も使わない（`WinMain` は `r8` のコマンドラインだけ使う） |
| Media Foundation の有無確認 | ○ | — | `MFStartup` を `Mfplat.dll` から引く（§5.1 #1） |
| エンジンの場所決め、レジストリ | ○（`Software\Image-Line\Shared\Paths` の `FL Studio engine`、`Shared DLLs`） | ○（同じキー名の文字列、UTF-16 で 5 件。用途は未確認） | 命令と文字列 |
| エンジン選択のダイアログ | ○（`Builds\` があるとき、ILTools の `ShowEngineChoice`） | — | §5.1 #6 |
| 署名の検査 | ○（エンジン DLL を `Image Line NV` と照合） | ○（**自分自身**と、`Flags` のビット 0 が 0 なら**ホスト exe**） | §5.1 #7、§6.4 #4〜5 |
| ウィンドウ作成、クラス登録 | — | ○ | FL64.exe の USER32 import は `MessageBoxA` だけ。エンジンは `CreateWindowExW` 9 箇所、`RegisterClassW` 8 箇所、`MAINICON`、`TApplication` |
| メッセージループ | — | ○（`CreateFruityInstance` の内側で回る） | §6.4 #10。FL64.exe に `GetMessage`、`PeekMessage` が無い |
| 音声デバイス | — | ○（読み） | FL64.exe は winmm も dsound も import しない。エンジンは `DirectSoundCreate`（2 箇所）、winmm の midiIn/midiOut 20 名、`ASIO` の文字列（部分一致。ASCII 61、UTF-16 23）、`DirectSound`（ASCII 26、UTF-16 9）。ASIO ドライバーは COM 経由と読む（`CoCreateInstance` 14 箇所）。WASAPI の文字列は無い（0 件） |
| MIDI | — | ○ | 同上 |
| ファイル I/O | ホストは `GetFileAttributesW` のみ（CRT 以外） | ○（`CreateFileW`、`ReadFile`、`WriteFile` など） | import |
| プラグインのロード | — | ○ | `LoadLibraryW` 48 箇所、`CreatePlugInstance`（UTF-16）。プラグイン DLL は `Plugins\Fruity\…`（181 PE） |
| ライセンスの検証 | — | ○（読み。**どの関数かは未確認**） | エンジンの文字列に `license`（ASCII 79、UTF-16 58）、`License`（133、13）、`unlock`（130、35）、`Activate`（175）。ホストの文字列には無い。別ツール `CopyLicenseToAllUsers.exe`、`RemoveLicenses.exe` がある |
| 更新確認、ネットワーク | — | ○（読み） | エンジンの wininet import（`InternetOpenW`、`InternetOpenUrlW` ほか 15 名）と `image-line.com`（ASCII 11、UTF-16 37）、`Updates`、`check for update`。ホストの import に wininet は無い |
| COM / OLE の初期化 | — | ○ | ホストは ole32 を import しない |
| DPI の宣言 | ○（マニフェスト） | ○（`SetThreadDpiAwarenessContext`、`SetThreadDpiHostingBehavior` を import） | §3.3、§4.3 |
| 終了コードの伝達 | ✕（常に 0） | — | §5.1 #12 |

## 8. 他のホスト（FL Studio VSTi、ReWire）

- `FL Studio VSTi.dll`（57716）と `(Multi)` 版（57714）は、`CreateFruityInstance`（UTF-16）を `GetProcAddress` で引く関数 0x69fe50 を持つ（実測）。呼び出しは `rcx = 0, rdx = 0, r8 = [rbp+0x30], r9 = [rbp+0x38]`（0x69ffdc〜0x69ffe8）。**FL64.exe の 0xC0、0xEDE95A とは違う**（エンジンが読まないので差は効かない。§6.4 #1）。
- VSTi は `FLEngine_x64.dll` を `%s_Copy1.dll`、`%s%s_Copy%d.dll` の形で**複製して読み込む**文字列を持つ（実測。0x6a3420 の周辺）。FL64.exe の `_Copy` の処理（§5.1 #8）と同じ思想で、**エンジンは DLL 1 つにつきインスタンス 1 つ（グローバル状態を持つ）**と読める。VSTi は複数インスタンスのために DLL の複製を使うと読める。
- VSTi の `export` は `main`（VST2 の入口）と dbk 2 個。VSTi がホストオブジェクトに何を入れるか、返ったインスタンスの 20 スロットのどれを使うか、`Flags = 0xf` が実際の値か、は**未確認**。`FLReWire_x64.dll` も同様に `CreateFruityInstance` の文字列を持つが、中は読んでいない。

## 9. ブラウザ化への含意（読み。決めない）

1. **ホストを JS（または小さな wasm）で置き換えれば、エンジンを動かし始めることはできる**と読める。最低限の契約は次のとおり（FL64.exe の実装が示す）:
   - ゲストのメモリ上に 0x38 B のホストオブジェクトを置き、`+8` = 1（HostVersion）、`+0xc` = 0（Flags）を入れる。`+0x18` は "FL Studio"、`+0x20`、`+0x28` は空文字列を指す。`+0x30` は省略可能（空ならエンジンが自分の DLL のディレクトリを使う。§6.1）。
   - vtable に 3 スロット。スロット 0（`Dispatch32`）はスロット 2 へ符号拡張して渡す。スロット 2 は `id = 3`、`index <= 1` で 1、それ以外は 0。スロット 1 は何もしない。
   - `GetProcAddress` 相当で `CreateFruityInstance`（VA 0x1375990）を解決して、`(0xC0, 0xEDE95A, host, &instance)` で呼ぶ。
   - 戻り値が 0 か 2 なら、`instance->vtable[0]`（片付け）を呼ぶ。
2. **ホストの仕事の大半はブラウザでは要らない**: Media Foundation の確認、レジストリによるパス探し、`Builds\` と ILTools によるエンジン選択、`LoadLibraryW` の `_Copy` 処理、署名検査（ホスト側のもの）は、置き換えるなら省略できる。FL64.exe は `.text` のホスト固有部が 26 KB 前後で、**ホスト EXE を lift しなくても足りる**と読める。FL64.exe を lift するのは、`LoadLibraryW` → `CreateFruityInstance` の経路を**同じ runtime で再現する比較の相手**（差分の相手）としては使えるが、必須ではない（読み）。
3. **重いのはホストの外**:
   - `CreateFruityInstance` は `Flags = 0` だと `TApplication.Run` 相当の**ブロッキングのメッセージループを内側で回す**（§6.4 #10）。ブラウザのイベントループとの接続（ループを分割する、中断可能にする、など）は設計が未決。`Flags` のビット 0 を立てると別のループ関数（0x11efbf0）に入り、ホスト exe の署名検査も飛ぶ。ただしこのときエンジンがホストに何を求めるか（`+0x10` のウィンドウハンドルなど）は未確認。
   - **エンジン自身による署名検査**（§6.4 #4〜5）: `WinVerifyTrust` と crypt32 の 7 名は、runtime ではまだ「応答なし」の系統（`remaining-unsupported.md` 201 行の引用: 暗号・信頼 8 API。その文書の基準は当時の HEAD で、今の HEAD では再確認していない）。検査の失敗は −2 でエンジンを止める。自己検査（`HInstance` の DLL）は `Flags` で飛ばせない。ホストの検査は `Flags` のビット 0 で飛ぶ。`hinstance-caller.md` 133 行の引用では、runtime は `GetModuleFileNameW(NULL)` に仮想の DLL パス（決定 0018 の値）を返すので、ホスト検査の対象のパスも DLL 側と同じになる（今の HEAD で再確認していない）。
   - その先の描画と入力: 応答のない系統は user32 108 API、gdi32 54 API など（`remaining-unsupported.md` 190〜215 行の引用）。**これらは境界の外の、エンジン内部の問題**。
4. **インスタンスは DLL 1 つにつき 1 つ**（§8）。ブラウザでは wasm インスタンス 1 つにつきエンジン 1 つと読める。
5. ホストの寄与が小さいため、**「境界」は決定の対象としては小さい**。決める必要があるのは、(a) ホストオブジェクトとその vtable をゲストのどこに置き、ゲストの `call [vtable+n]` をどう受けるか（§10 の U1）、(b) 署名検査 API に何を答えるか（偽の成功を答えるか、`Flags` で一部を避けるか）、(c) ブロッキングのループをどう扱うか、の 3 点（未決。PM の決定が要る）。

## 10. 判断に必要な未解明点

| ID | 内容 | 状態 |
|---|---|---|
| U1 | ゲストが `call [rax]` でホストの関数（vtable のスロット）へ飛ぶとき、現在の lift と runtime はそれをどう受けるか。ホストが与えるアドレスはゲストのコード範囲の外になる。既存の `fle_imp_*` の仕組みで足りるか、薄い関数をゲストの image の中に置くか | **未確認**（runtime のソースを読んでいない。この調査の範囲外） |
| U2 | `Dispatch` の ID 0、1、4 の意味。FL64.exe は 0 を返してエンジンが動く（実測の命令の読み）が、エンジンが戻り値を使わない確証はない | 読み。5 箇所中 4 箇所は戻り値を捨てる |
| U3 | `Flags` のビット 1〜3 の意味。ビット 0 のときのエンジンの挙動（ホストがメッセージを回すのか、どの関数を呼ぶのか） | 未確認 |
| U4 | `+0x10`、`+0x20`、`+0x28` の用途 | 未確認 |
| U5 | `CreateFruityInstance` が最後まで走るために runtime が必要とする応答の一覧（`WinVerifyTrust`、`CryptQueryObject`、`CertFindCertificateInStore`、`CertGetNameStringW`、`GetModuleFileNameW`、`LoadIconW`、ウィンドウ API 群、…）。実行していない | 未確認（実行が必要） |
| U6 | `0x11b32a0`（インスタンス生成）以降の初期化で、InitUnits の外に必要になる外部 API の洗い出し | 未確認（`census` のような調査が別に要る） |
| U7 | VSTi、ReWire のホストオブジェクトの中身と、返ったインスタンスの 19 スロット（1〜19）の用途 | 未確認 |
| U8 | 署名検査の実際のエラー経路（`WinVerifyTrust` の戻り値で分岐が変わる箇所）。実機の値 | 未確認（実機なし） |
| U9 | ライセンス検証がどの関数で行われ、`CreateFruityInstance` の経路に入るか | 未確認 |
| U10 | 音声の経路（`DirectSoundCreate` 2 箇所、ASIO の COM、`ILWASAPI2ASIO_x64.dll`）と、ブラウザの `AudioWorklet` との対応 | 未確認 |
| U11 | `<R>` が指すインストール先（NSIS 変数 31）の確定、`<V38>` の同定 | 未確認 |

## 11. 限界

- **実行していない**。すべて静的な読み。FL64.exe と FLEngine の命令は capstone の線形走査で、Delphi 側にはデータがコードの中にあるため、`(bad)` を含む。参照の索引は「線形に読めた命令」からのもので、漏れがありうる（下限）。
- 署名は**検証していない**（埋め込み証明書の subject と issuer を読んだだけ）。
- 関数の意味づけ（`Dispatch`、`GetBaseDir`、`TApplication.Run`、`Destroy`）は、形、定数、周辺の文字列、Delphi の VMT の並びからの読み。ソースやシンボルは無い。
- VSTi、ReWire、LilyPond、Python 等は表の項目のみ。ILTools は `ShowEngineChoice` の存在と文字列だけで、中は読んでいない。
- NSIS の変数コード（`<R>` = 31、`<INSTDIR>` = 21、`<V38>`）の解釈は、Unicode の NSIS 文字列では変数が `0x03` と `0x8000 \| 変数番号` の 2 文字で符号化される、という前提による読み（`$PLUGINSDIR\System.dll` が 26 で整合する）。確認は取っていない。
- NSIS の extract エントリのうち 145 件は他のエントリと同一のデータを指す（133 件はインストーラ用プラグイン DLL の重複、12 件が製品の PE の重複。FLEngine_x64.dll は 950、59704、59725 が同じデータ）ので、表から省いた。

## 12. 補助ファイル（`docs/investigations/host-engine-boundary/`）

| ファイル | 内容 |
|---|---|
| `installer-pe-inventory.tsv` | PE 306 件の一覧（§2）と、末尾に import 集合の一覧 |
| `extract_by_entry.py` | extract エントリ番号を指定して hash / write（`tools/extract_nsis2.py` を import） |
| `pe_inventory_nsis.py`、`inventory_to_tsv.py` | NSIS の配置と PE の採取 → TSV |
| `pe_header.py`、`pe_imports_exports.py` | PE ヘッダ、import、export の表示 |
| `disasm_all.py`、`dump_lines.py` | PE の実行可能セクションの線形逆アセンブル（FL64.exe 用）と、その出力のアドレス範囲の切り出し |
| `disasm_range.py`、`function_extent.py`、`function_strings.py` | 範囲の逆アセンブル（IAT 名とポインタ値付き）、`.pdata` から関数境界、関数内の UTF-16 文字列参照 |
| `rip_xref.py`、`thunk_callers_scan.py` | `rip` 相対参照の索引、import 用の薄い関数への `call` の数え上げ |

いずれも `PYTHONDONTWRITEBYTECODE=1 python3 …` で動く。入出力のパス（`/tmp/opencode/host/` など）は実行時のものを直書きしている。
