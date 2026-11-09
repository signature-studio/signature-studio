# サードパーティ VST3/VST2 プラグインを lift して動かせるか(P3 の実現可能性): 静的解析と見積り

作成: 2026-11-03、基準 HEAD **af6ec7c**(ブランチ main)。担当: 調査 subagent `vst1`(PM セッション ses_eafd6e97fffeXSloTmS42jhMt4 から起動)。

**静的解析で、プラグインも FL Studio も実行していない。進捗ではない。G 段階表の到達にも数えない。**
(例外が 1 つある: §4.2 の微小ベンチは、**自作の合成カーネル**〔プラグインのコードでも FL のコードでもない数十命令の機械語〕を lift して測った。プラグインのバイナリ・インストーラは一度も実行していない〔Wine を含む〕。)

## 凡例

- **実測** = 自分でコマンドを実行して数えた/測った(再現は `tools/` と `README.txt`)。
- **読み** = 解釈(文字列・名前・直接 call・公開資料から)。実行して確かめていない。
- **未確認** = 調べていない、または静的には決められない。
- **推測** = 根拠が弱い見積り(前提を書く)。
- 数字は自分で数えた/測った物だけ。FLEngine 側の過去の数字は出典(決定番号)を付けた引用。

## 0. 結論(先頭)

### 0.1 取り出せたもの(実測)と欠品

参照プロジェクトの 21 インスタンス(VST3 ×20、VST2 ×1)のうち、**17 インスタンス分(製品 9 種 + Kilohearts の共有 DLL)の Windows x64 バイナリを、インストーラを実行せずに取り出した**(`lift/out-vst1/bin/`、git 対象外)。ini の記録サイズと全件一致。

| 製品 | FLP の数 | 取り出した物(サイズ B) | 命令セットの要点(実測、§2.3) |
|---|---:|---|---|
| Serum 2 2.1.5 | 6 | `Serum2.vst3` **19,236,864** | 2.72M 命令。**ほぼ SSE/SSE2 基準線**。AVX 系を含む関数は 75 個(AVX)+15(AVX2)+37(FMA)、関数バイトの約 0.4% |
| Vital 1.6.4 | 1 | `Vital.vst3` 15,571,240(ほか `.clap`/VST2/スタンドアロンも) | 2.45M 命令。AVX 系(AVX/AVX2/FMA/AVX-512)を含む関数が関数バイトの約 22%+9%。SSE/SSE2 版の経路も別にある |
| OTT 1.37 | 2 | `OTT.vst3` 3,244,032 | 0.58M 命令。AVX を含む関数 16 個、FMA 15 個 |
| Kilohearts(Filter ×2、Gain、Haas ×2、Pitch Shifter、Tape Stop、Disperser = 8 インスタンス、6 製品) | 8 | 殻 `*.vst3` 247,296 ×6 種(6 種の差は 400〜3,550 B。`.rdata`/`.rsrc` の ID・名前が主で、`.text` の差は最大 4 B)+ **共有 DSP `HeartCore.core_64` 76,642,304** | 殻 4.5 万命令。HeartCore 5.5M 命令。AVX 系 1.2%(AVX)+0.65%(AVX2)+0.35%(AVX-512)の関数バイト、SSE3 系 3.2% |
| **欠品** Spectral Gate(Stepwise) | 1 | **無し**(ini: 5,261,824 B) | 依頼者側の供給が必要 |
| **欠品** Ozone Imager 2(iZotope) | 2 | **無し**(ini: VST3 2,194,432 B。同じ PC に VST2 の 5,837,312 B と `iZOzoneImager2.dll` 75,692,872 B もある) | 同上。VST3 が薄い殻で本体が別 DLL の可能性(未確認) |
| **欠品** GClip(GVST、VST2) | 1 | **無し**(ini: 843,776 B) | 同上 |

- **訂正(重要)**: `docs/investigations/reference-project.md` などが使う「Serum 2 = 39,121,408 B」は、ini の **Generators\VST3\Serum 2 の記録**(2024-11-28 走査 = 古い版)のサイズ。ini には同じパスの記録がもう 1 つ **Effects\VST3\Serum 2 FX = 19,236,864 B**(2026-07-07 19:48 走査)があり、これがインストーラ内の 2.1.5 の実ファイル(更新日時も同じ)と一致する。現行の Serum 2 本体は **19.2 MB**(39 MB の半分以下)。
- Vital の Windows インストーラは **Inno Setup 6.7.0**(loader revision 2)で、手元の `innoextract` 1.9(≤ 6.0.5)では開けない。自作の読み取り専用ツール(`tools/inno67_carve.py`)で取り出し、**署名付き 3 ファイル(Vital.vst3 / .clap / スタンドアロン)の Authenticode SHA-256 を、戻したバイト列から再計算して署名内のダイジェストと一致**させた(実測。Vital.vst3 `27581398…`)。取り出しが 1 バイトも違わない証拠である。

### 0.2 P3 への実現可能性(A〜D の比較の要約)

| 案 | 内容 | 届くか | 根拠(要約) |
|---|---|---|---|
| **A** 全部 lift(現行 lifter の忠実度のまま) | 上の 9 製品を fe-lift で lift し、現行の 6 モジュールに足す | **届かない見込み(推測。外挿)** | 大きさ: 新規の生成 C 約 **892 MB**(実測)= 現行 6 モジュールの C 872 MB とほぼ同じ。wasm は -O2 で **約 +683 MB → 単一 wasm 1,350 MB で 1 GiB を超える**、-Oz なら +328 MB → 649 MB(60%)で入る(推測)。速さ: 合成カーネルでの実測が、同じ機械語の直接実行に対し wasm -O2 で **整数 5.5〜6.2 倍、scalar double 17〜20 倍、packed SSE 145〜160 倍**(新フラグ。旧フラグは 15 倍・29〜32 倍・205〜215 倍)。-Oz(配信中の 320 MB と同じ)は 33〜37 / 44〜45 / 230〜245 倍。実時間に 1 スレッドで届くには、プラグイン群のネイティブでの負荷が 1 コアの数 % 以下でなければならない(§4.2) |
| **B** プラグイン無しで再生 | FL が警告を出して無視する(エンジンの文字列。reference-project.md §4.3) | **P3 の定義を満たさない** | 楽器 8 のうち Serum 2 ×6 と Vital の 7 つが鳴らず、マスターのエフェクトも欠ける = 「このプロジェクトの音」ではない。**この案で P3 を達成と数えてよいかは依頼者の判断が要る(事実のみ書く)** |
| **C** レンダリング済み音声を使う | 実プラグインで事前に作った音声を再生 | **P3 を満たしたことには数えない** | 実エンジン/実プラグインの実行ではない(AGENTS.md の「進捗の誠実性」)。ここでは比較のために書くだけ |
| **D** その他 | D1 忠実度を落とした「音声用の lowering」(別のコード生成)/ D2 先行(オフライン)レンダリング+バッファ再生 / D3 複数スレッド(ワーカー分割)/ D4 ユーザー PC 上のネイティブのブリッジ | D1: **不明(有望な兆候あり)**。D2: 「途切れない」だけは満たすが待ち時間が長い(推測)。D3: 大改修。D4: 「ブラウザ上で動作」に反する可能性(事実のみ) | D1 の兆候: 同じ合成カーネルで SSE 演算を「既定 MXCSR 仮定の素の C」に替えた what-if の実測が、wasm -O2 で packed 145〜160 倍 → **9〜11 倍**(-msimd128)、scalar 17〜20 倍 → **約 3 倍**。ただし実プラグインでの効果は未検証で、MXCSR/NaN/denormal のビット厳密さを捨てる(実機との照合手段は無い) |

**この表の速さの数字は、PM 指示(決定 0151)の後の新フラグ(builtin 有効)を主にしている。旧フラグの数字は §4.2 の表に並べた(参考)。新フラグで下がったのは整数・フラグ系で、SSE の浮動小数の倍率は 10 倍台に下がらない。**

**判断が分かれる点(PM が決める)**: P3 を「実プラグインの音を、実時間で、途切れずに」と読むなら、A の現状の lifter では届かない見込みが強く、D1(新しい lowering)が事実上の前提になる。D1 は、FLEngine 自身の P2(UI 120 fps)にも同じ種類の改善(レジスタをメモリに置く方式・フラグ・SSE の厳密さ)が要るので、**プラグイン専用ではなくエンジン全体のコード生成の問題**である。推奨は §5.6。

## 1. 入手可能なバイナリの棚卸し(実測)

詳細表: `tables/inventory.tsv`(製品、インストーラ、取り出し方、サイズ、SHA-256、ini のサイズとの一致)、`tables/ini_records.tsv`(ini の該当記録)、`tables/authenticode_check.tsv`。

### 1.1 インストーラの形式と取り出し方(インストーラは実行していない)

| ファイル | 形式(ヘッダで識別) | 道具 | 結果 |
|---|---|---|---|
| `Install_Xfer_Serum2_2.1.5.exe`(1,313,084,056 B) | PE + **NSIS-3 Unicode**(`NullsoftInst` が 72196 にある。ファイル別 deflate、非 solid) | `7z x`(必要な 4 ファイルだけ) | `Serum2.vst3\Contents\x86_64-win\Serum2.vst3`(19,236,864)、`moduleinfo.json`(1,748)、`patch.zip`。インストーラ全体の展開後は **1,413,375,641 B**(ほぼ全部が音色・波形・FLAC の内容データ。7,370 ファイル。`Tables` だけで 44.5 MB) |
| `Install_Xfer_OTT_137.exe`(8,397,928 B) | PE + NSIS-3 Unicode | `7z x` | `OTT.vst3`(3,244,032、**単一ファイルの .vst3**)、`OTT_x64.dll`(VST2 3,146,752)、`OTT.dll`(x86 VST2) |
| `VitalInstaller.exe`(41,808,960 B) | PE + **Inno Setup 6.7.0**(`Inno Setup Setup Data (6.7.0)`、loader table version 2)。`innoextract` 1.9: 『Unexpected setup loader revision: 2』で失敗 | 自作 `tools/inno67_carve.py`(読み取りのみ。下) | `Vital.vst3` 15,571,240、`Vital.vst2.dll` 15,426,560、`Vital.clap` 15,482,656、スタンドアロン 15,509,280 |
| `Kilohearts Offline Installer.zip`(3,833,108,300 B、stored) | zip の中の `payload.dat`(3,817,512,248 B)は **zip ローカルヘッダの連なり(中央ディレクトリ無し)**。179 エントリ、うち `windows.zip/1ccb1df8c8f10745`(55,114,108 B)が Windows 側の本体で、他は presets/samples/wavetables の bank(~3.7 GB) | 自作 `tools/khs_payload_scan.py` + `unzip` | 殻 `kHs *.vst3_64`(各 247,296)と `HeartCore.core_64`(76,642,304、PE DLL)、`product_index.json`(17,357) |
| `Xfer Serum2 v2.1.5.pkg`、`*.dmg`、`NI_Massive_X_*`、`Fresh Air.*` | mac / 対象外 | 開いていない | — |

- **Inno Setup 6.7.0 の取り出し(実測)**: 『rDlPtS』のオフセット表(version 2)から Offset0 = 40,327,466(設定ヘッダ)と Offset1 = 1,101,824(ファイルデータ)を読む。Offset1 は `zlb\x1a` + 1 バイトの辞書指定(0x16)+ 生の **LZMA2**(python の `lzma`)で、**1 つの solid チャンク 87,567,245 B** が Offset0 でちょうど終わる。チャンクの中に実行ファイルが 5 個(Vital.exe、.vst3、VST2 .dll、.clap、WiX の小さな PE)。コンパイラは実行ファイルに **E8/E9(call/jmp)の『呼び出し命令最適化』フィルタ**をかけて格納している(戻す前は PE ヘッダの section の値が壊れている)。戻し方は、公開されている innoextract のソース(`src/stream/exefilter.hpp`、`inno_exe_decoder_5200` の flip_high_byte 版。zlib ライセンス)の仕様(64 KiB ブロック、ブロックをまたぐ命令は対象外、24 ビット相対化、結果の bit 23 が立てば最上位バイトを反転)で実装した。**正しさの検証**: 署名付き 3 ファイルの Authenticode SHA-256 が一致(上)。サイズも ini(15,571,240 / 15,426,560 / 15,482,656)と一致。
- 取り出した全ファイルの PE 検査: Machine 0x8664、ImageBase 0x180000000(スタンドアロンだけ 0x140000000)、ASLR 有り、Serum 2/OTT/Kilohearts は **署名無し**、Vital の 3 ファイルは署名有り(Authenticode)。再配置あり(`.reloc`)、`.pdata` あり(§2)。

### 1.2 欠品(依頼者側の供給が必要な物)

| 物 | 使用数 | 事実 |
|---|---:|---|
| Spectral Gate(Stepwise。VST3 バンドル) | 1(マスター) | `~/ダウンロード/` に無い。ini: `...\VST3\Spectral Gate.vst3\Contents\x86_64-win\Spectral Gate.vst3` 5,261,824 B、2025-04-19 |
| Ozone Imager 2(iZotope) | 2 | 無い。ini: VST3 2,194,432 B(薄い殻の可能性。同じ iZotope の Ozone 11 の VST3 も 2.19 MB で、殻に見える。読み)、別に VST2 5,837,312 B と `iZOzoneImager2.dll` 75,692,872 B(用途は未確認) |
| GClip(GVST、VST2) | 1(マスター) | 無い。ini: `...\VstPlugins\GClip.dll` 843,776 B |
| Vital が動的に import する Windows の再頒布ランタイム | — | Vital だけが `VCRUNTIME140.dll`/`VCRUNTIME140_1.dll`/`MSVCP140.dll` と UCRT の `api-ms-win-crt-*`(計 181 API)を import(Serum 2/OTT/HeartCore は CRT を静的リンク)。これらの DLL は Vital のインストーラにも ~/ダウンロード にも無い(FL のインストーラの `Shared\Python` に `vcruntime140*.dll` があるが、msvcp140・ucrt は未確認) |
| Serum 2/Vital/Kilohearts の内容データ | — | Serum 2 の音色・波形は上の 1.41 GB(インストーラの中)。FLP の Serum 2 の状態(18〜378 KB。XferJson)が工場出荷の波形を**名前で参照するか、波形を内蔵するか**は未確認。Vital の工場データはインストーラのチャンクに入っているかを未確認(取り出したのは実行ファイル 4 つ)。Kilohearts の bank 約 3.7 GB |

## 2. 各バイナリの静的な事実(実測)

出力: `tables/pe_headers.tsv`(ヘッダ、セクション、ディレクトリ、署名、バージョン)、`tables/pe_imports.tsv`・`pe_import_apis.tsv`・`import_categories.tsv`(import)、`tables/pe_exports.tsv`、`tables/isa_*.tsv`、`tables/felift_*.tsv`。道具: `pefile` 2024.8.26、`capstone` 5.0.7(`iced-x86` の Python 版は無い)。

### 2.1 PE ヘッダと構成

| | Serum2.vst3 | Vital.vst3 | OTT.vst3 | HeartCore.core_64 | kHs 殻(各) |
|---|---:|---:|---:|---:|---:|
| ファイル B | 19,236,864 | 15,571,240 | 3,244,032 | 76,642,304 | 247,296 |
| SizeOfImage(MiB) | 33.3 | 17.5 | 3.1 | 98.4 | 0.26 |
| コード(実行セクションの生 B) | 10.6 MB | 9.7 MB(.text 9.1 + IPPCODE 0.6) | 2.0 MB | 20.6 MB | 0.16 MB |
| `.pdata` の関数数 | 33,791 | 27,178 | 7,951 | 111,666 | 818 |
| `.rdata` | 7.4 MB | 4.9 MB | 0.53 MB | 21.8 MB | 0.07 MB |
| `.rsrc` | 0.13 MB | 0.0005 MB | 0.51 MB | **30.3 MB** | 0.001 MB |
| `.data` の仮想サイズ | **16.3 MB**(生は 0.6 MB。ほぼ BSS) | 3.2 MB | 0.07 MB | 28.3 MB | 0.01 MB |
| TLS(実装 TLS) | あり、コールバック 1 | あり、0 | あり、0 | あり、**4** | あり、0 |
| 遅延 import | 0 | 0 | 0 | 0 | 0 |
| 署名(Authenticode) | 無し | 有り(15,656 B) | 無し | 無し | 無し |
| export | `GetPluginFactory`、`InitDll`、`ExitDll`(3 個のみ) | 5 個(VST3 の 3 つのほか) | 3 | 144(`ACFRegisterPlugin_k***` が製品ごとに、`ACFGetClassFactory` ほか) | 14(`GetPluginFactory`/`InitDll`/`ExitDll`/`VSTPluginMain`/`KiloheartsSnapinMain`/`ACF*`/`KHSAudioUnit*`) |
| PDB 名(デバッグディレクトリ) | Serum2.pdb | Vital.pdb | 無し | HeartCore.pdb | `<製品名>.pdb` |
| ビルド時刻(PE) | 2026-07-07 | 2026-04-30 | 2023-03-24 | 2026-03-10 | 2026-03-10 |

- **Kilohearts の構造(読み)**: 殻は 6 製品とも**同じサイズで、ほぼ同じコード**(実測: Disperser との差は 400〜3,550 B で、`.rdata`/`.rsrc` の ID・名前が主、`.text` の差は Pitch Shifter の 4 B だけ。SHA-256 は全部違う)。`.core_64` という拡張子の文字列を持ち、`SHGetKnownFolderPath`/`GetFullPathNameW`/`LoadLibraryW` を import する。`HeartCore.core_64` は `ACFRegisterPlugin_kclp`、`_kdsp`、`_kfat`… と製品ごとの登録関数を持つ共有エンジン(製品 ID は殻の `KiloheartsSnapinMain_xxxx` と `product_index.json` の `id` で対応)。つまり 8 インスタンスは **76.6 MB の 1 つの DLL を共有**する。
- Serum 2 の `.data` 仮想サイズ 16 MB は静的配列(BSS)。lift しても線形メモリに載る大きさ(イメージ 33 MB)は変わらない。

### 2.2 import の分類(`tables/import_categories.tsv`)

| | KERNEL32 等の DLL 数 | API 総数 | memory | thread | sync | file | registry | com | gdi | user32 系 | d3d/dxgi | winsock | http | crypto | CRT | process/module | time | string/locale | exception | その他 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| Serum 2 | 13 | 332 | 21 | 15 | 21 | 42 | 3 | 12 | 12 | 90 | 5 | 13 | 6 | 0 | 0(静的) | 37 | 12 | 12 | 9 | 22 |
| Vital | 26 | 501 | 8 | 7 | 15 | 30 | 5 | 12 | 30 | 137 | 0 | 18 | 13 | 0 | **181(動的 CRT)** | 22 | 4 | 2 | 6 | 11 |
| OTT | 8 | 380 | 10 | 12 | 20 | 21 | 0 | 9 | **125(gdiplus 96)** | 111 | 0 | 0 | 0 | 0 | 0(静的) | 34 | 6 | 10 | 9 | 13 |
| HeartCore | 17 | 404 | 16 | 15 | 30 | 41 | 4 | 12 | 9 | 114 | 0 | 29 | 0 | **19** | 0(静的) | 43 | 9 | 20 | 9 | 34 |
| kHs 殻 | 4 | 103 | 5 | 9 | 9 | 20 | 0 | 1 | 0 | 2 | 0 | 0 | 0 | 0 | 0 | 22 | 4 | 10 | 9 | 12 |

- OpenGL(`opengl32`)の **import は 0 件**(4 製品とも)。Vital は文字列に `opengl32.dll`、`wglCreateContext`、`d3d11.dll`、bgfx のレンダラがあり、**実行時の LoadLibrary** で GL/D3D11 を使う(読み)。Serum 2 は `d3d11`/`d2d1`/`DWrite`/`dwmapi`(Direct2D 系)を import し、文字列に `D3D11CreateDevice`。OTT は GDI+(`gdiplus.dll` 96 API)。Kilohearts の殻は GUI を import しない(GUI は HeartCore 側)。
- ネットワーク: Serum 2 は WinINet(`InternetOpenUrlA`/`InternetReadFile`/`HttpQueryInfoW`)と WS2_32(`socket`/`bind`/`listen`/`accept`/`recv`/`send`/`WSAPoll`)、Vital は WinINet 13 + WS2_32 18(`getaddrinfo` ほか)、HeartCore は WS2_32 29 + CRYPT32 16 + WinVerifyTrust。OTT は無し。
- 例外: Serum 2/OTT/HeartCore は **`RaiseException`・`RtlUnwind`・`RtlUnwindEx`・`RtlVirtualUnwind`・`RtlLookupFunctionEntry`・`RtlPcToFileHeader`** を import(静的 CRT の x64 C++ 例外処理)。Vital は動的 CRT の `_CxxThrowException`/`__CxxFrameHandler4`。**既存の lift は Delphi の SEH だけを扱う**(決定 0132)。MSVC の C++ 例外(throw/catch)は未対応(§3.4)。
- スレッド: Serum 2 は `CreateThread` と **スレッドプール**(`CreateThreadpoolWork`/`SubmitThreadpoolWork`)、FLS/TLS。協調スレッド(決定 0122/0124)の上にスレッドプール API は無い。
- **既存のランタイム表との突き合わせ**(`tables/import_coverage.tsv`、`import_not_answered.tsv`): `lift/crates/fe-lift/src/imports_table.rs` の応答行(604 行 = 実装済みの `fle_imp_*`)に載っていない API は、Serum 2 で **129/332**、OTT 163/380、HeartCore 157/404、Vital 125/501(+動的 CRT の 181)、kHs 殻 14/103。種類別(5 製品の和集合 581 件): GUI 系(gdi 108、user32 104)が最大、次いで CRT 181(Vital のみ)、WinSock 33、file 24、sync 17、http 13、crypto 12 ほか。『応答行がある』は『実装が仕様どおり』を意味しない(読み)。

### 2.3 命令セットの census(`tables/isa_census.tsv`、`isa_functions.tsv`、`isa_mnemonics.tsv`)

方法: `.pdata` の全関数と、それ以外の実行セクションのバイト(パディング・leaf 関数)を capstone で線形に逆アセンブルし(無効バイト列 Serum 2 で 5、HeartCore 1,836、Vital 2,861)、capstone の拡張グループと VEX/EVEX で分類。

| 命令数(クラス別) | Serum 2 | Vital | OTT | HeartCore | kHs 殻 |
|---|---:|---:|---:|---:|---:|
| 合計(掃引) | 2,722,127 | 2,452,639 | 577,417 | 5,537,388 | 44,939 |
| 基本(整数・制御) | 2,256,927 | 1,589,258 | 464,307 | 4,590,309 | 40,830 |
| SSE | 144,359 | 253,754 | 27,169 | 353,595 | 682 |
| SSE2 | 146,766 | 83,142 | 17,529 | 98,279 | 450 |
| SSE3 | 0 | 176 | 0 | 2,034 | 0 |
| SSSE3 | 7 | 25 | 0 | 138 | 0 |
| SSE4.1 / 4.2 | 53 / 27 | 544 / 9 | 36 / 4 | 203 / 7 | 0 / 2 |
| **AVX** | **2,175** | 119,581 | 889 | 19,527 | 315 |
| **FMA3** | 334 | 30,136 | 164 | 1,850 | 9 |
| **AVX2** | 226 | 1,649 | 6 | 7,767 | 4 |
| **AVX-512** | 0 | **59,183**(zmm 45,132) | 0 | 9,096(zmm 8,644) | 0 |
| BMI1 / BMI2 | 558 / 4 | 14 / 400 | 0 / 0 | 6 / 0 | 0 |
| POPCNT / LZCNT・TZCNT | 1 / 4 | 24 / 4 | 0 / 0 | 0 / 0 | 0 |
| AES-NI / PCLMUL / SHA | 0 | 0 | 0 | 0 | 0 |
| x87 | 7 | 1,367 | 84 | 104 | 0 |
| MMX | 0 | 17 | 0 | 5 | 0 |

- **Serum 2 は、DSP を含めてほぼ SSE/SSE2 基準線で書かれている**(読み)。AVX 系を含む関数は 75(AVX)、15(AVX2)、37(FMA)、BMI1(tzcnt)は 102 関数。関数バイトでは 0.4%+0.07%+0.2%。**『Serum 2 は AVX2 を要する』というメーカー発言(Linux ベータの記事。reference-project.md §4.4)と、Windows 版のコードの量は合わない**。ただし起動時の機能チェックが AVX2 を要求して拒否するか(CPUID/XGETBV を使う関数は 4 つ)は**静的には未確認**。4 関数の読み: 661 B の 1 つは CPUID 葉 0/1/7/0x24 を引き、OSXSAVE(ECX bit 27)・AVX(bit 28)・XCR0 の SSE|AVX 状態(`and 6` / `cmp 6`)・AVX2(葉 7 の EBX bit 5)を調べる形で、Vital にも同じ大きさの関数がある(CRT の `__isa_available` の初期化と読める)。178 B の 1 つ(直接の呼び出し元 0 = 関数ポインタ経由)も同じ AVX/AVX2 の判定の形で、**Serum 2 自身の AVX2 検出の可能性**。696 B と 347 B の 2 つは CPUID 葉 0 だけ(ベンダー文字列と読める。ビットの検査なし)。
- **Vital** は AVX 系が大量(`vaddps`/`vmulps`/FMA/`vmovups` と AVX-512 の zmm)で、IPP(`IPPCODE` セクション)の分も入る。SSE/SSE2 版の経路も別にあるが、CPUID で選ぶのか、ビルドで固定かは未確認。
- **HeartCore** は SSE3 系(`movddup` 948、`movshdup` 588、`haddps` 230)が 3.2% の関数バイトにあり、CPUID が SSE3 を名乗らない現行のランタイム(決定 0009/0075)とは噛み合わない可能性。
- AVX 系を含む関数の分布: `tables/isa_functions.tsv`(例: Vital の AVX を含む関数は 1,176 個で関数バイトの 21.7%、うち AVX-512 は 421 個で 9.0%)。
- 間接 call/jmp(`tables/isa_control.tsv`): 静的に、Serum 2 は `call [reg+disp]`(vtable 形)26,793、`call reg` 1,058、`jmp reg` 1,419、ジャンプ表の可能性のある `jmp [idx*8+...]` 275;HeartCore は vtable 形の call 78,630、`jmp reg` 2,964。多くはゲストの vtable 呼び出しで、`fle_dispatch`(関数入口表への実行時引き当て。表に無い宛先は名前付きの停止)を通る。

### 2.4 fe-lift の census(`fe-lift lift FILE /dev/null --pdata` を各 DLL に実行した結果。`tables/felift_census.tsv`、`felift_unsupported_by_class.tsv`、`felift_unsupported_top40.tsv`)

lifter は静的ツールで、プラグインのコードは走らない。`CARGO_TARGET_DIR=lift/out-vst1/target cargo build --offline --release` で 9 秒、各 DLL で 0.07〜10 秒、RSS 最大 0.68 GB。**bad_nodes**: Serum 2/Vital/OTT/殻は 0、HeartCore は 11。

| | closure 命令 | 関数 | 生成 C | C B/命令 | 未対応サイト合計 | うち int3(パディング) | **int3 を除く** | 同 %(closure 比) | レジスタ jmp(`jmp reg_dispatch`/`msvc_table`) | MSVC ジャンプ表を導出できず | tail calls | goto |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---|---:|---:|---:|
| Serum 2 | 2,391,923 | 34,809 | **215.1 MB** | 89.9 | 13,453 | 10,273 | **3,180** | 0.13% | 1,141 / 9 | 16 | 2,475 | 238,351 |
| Vital | 2,024,240 | 21,592 | 178.9 MB | 88.4 | 199,165 | 4,024 | **195,141** | 9.6% | 208 / 123(+reg_const 12) | 40 | 5,624 | 157,853 |
| HeartCore | 4,929,364 | 110,995 | **449.9 MB** | 91.3 | 65,506 | 47,204 | **18,302** | 0.37% | 1,332 / 223(+reg_table 4、reg_const 1) | 55 | 3,869 | 377,934 |
| OTT.vst3 | 491,027 | 6,676 | 44.6 MB | 90.8 | 2,256 | 992 | 1,264 | 0.26% | 40 / 180 | 23 | 894 | 58,584 |
| kHs 殻 | 41,880 | 940 | 4.0 MB | 94.7 | 411 | 178 | 233 | 0.56% | 23 / 8 | 13 | 112 | 5,499 |

未対応サイトの種類(`felift_unsupported_by_class.tsv`。サイト数。『サイト』= closure 内の命令の個数で、実行回数ではない):

| | AVX/AVX2(VEX) | FMA3(VEX) | AVX-512(EVEX)+opmask | BMI/POPCNT/LZCNT | SSE3 | SSSE3 | SSE4.x | IAT 経由の間接 call/jmp(応答行なし) | x87 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| Serum 2 | 2,041 | 334 | 0 | 567 | 0 | 7 | 79 | 144 | 1 |
| Vital | 133,889 | 27,947 | 21,215+1,926 | 442 | 156 | 41 | 554 | 8,970 | 1 |
| HeartCore | 13,632 | 1,390 | 388+53 | 5 | 1,894 | 154 | 206 | 524 | 2 |
| OTT.vst3 | 845 | 164 | 0 | 0 | 0 | 0 | 40 | 204 | 1 |

- 読み: **Serum 2・HeartCore・OTT・kHs は、現行のランタイムが名乗る SSE2 までで走る経路が大部分で、未対応サイトは 0.1〜0.6%**。これらは CPUID でゲートされた AVX/SSE4 の別経路と読める(実行しないと確認できない)。ゲートされていない場合、その経路に入った時点で `fle_unsupported`(名前付き停止)になる。Vital は 9.6% が AVX/FMA/AVX-512 で、IPP と自前のカーネル。
- 『int3』は `.pdata` の関数に続くパディングを closure が含めた分(関数の外)。実行されない。
- 未対応の命令を実装するコスト: VEX/EVEX の全形はレーンごとの MXCSR 厳密処理(§4.2)が要る。128 ビットの VEX(`vaddss` 等)は SSE の非 VEX 形と同じ意味で足せる(読み)。256/512 ビットは 2/4 レーンに分けて実装する必要がある。
- ジャンプ表(`msvc_rejected`): Serum 2 で 16 件導出できず(『`ja` が表の前に無い』形が 12 件)、HeartCore 55、Vital 40。これらは `jmp reg` として実行時の dispatch に回る(読み。実行しないので、表の宛先が関数入口表にあるかは未確認)。

### 2.5 GUI の割合(`tables/gui_share_lower_bound.tsv`)

- GUI の API(user32/gdi32/gdiplus/comctl32/shell32/imm32/comdlg32/d2d1/dwrite/d3d11/dwmapi)を**直接呼ぶ関数**は、関数バイトの Serum 2 0.47%、Vital 0.89%、OTT 3.09%、HeartCore 0.23%、kHs 殻 3.08%(**下限**。GUI フレームワークの薄いラッパー、ゲームエンジン風の描画ライブラリ、仮想関数経由のビュー、フォント処理は数えていない)。
- RTTI のクラス名(`.?AV...@@`)を GUI らしい語(View/Editor/Slider/Font/Image/…)で粗く分けると、Serum 2 は 1,460 クラス中 885(61%)、Vital 1,136 中 365(32%)、OTT 504 中 297(59%)、HeartCore 4,464 中 1,376(31%)。**語の粗い一致で、バイト数ではない**(DSP のクラス名に Color/Text/Point を含むものも混ざる)。
- 結論(未確認): **GUI のコードが何割かは静的には決められない**。ただし (i) lifter は `.pdata` の全関数を lift するので、プラグインの窓を一度も開かなくても **GUI 分も生成 C と wasm の大きさに入る**(§4.1 の表はその前提)、(ii) P2 の UI は FL の主窓なのでプラグインの窓を開かない運用なら描画経路は不要だが、`createView`・window class・タイマーが生成時に呼ばれるかは未確認。
- 『GUI を切り落とす』選択肢: lift の seed を `GetPluginFactory`/`InitDll`/`ExitDll` と processor/controller 系 vtable に絞ると、GUI 専用コードの一部は closure から外れる可能性があるが、vtable の全メソッドを seed に出す仕組みが要る(未検討)。

## 3. FL が VST3 をホストする方法(FL 側の読み)

### 3.1 FL 側の部品と、プラグインへ渡す機能(読み: RTTI のクラス名・IID・文字列から。実行はしていない)

出典: `binaries/win64/FLEngine_x64.dll`(SHA-256 `b2c64da4…`)と `lift/out-qfc/quickfontcache_x64.dll`(`afc5a0ed…`)の RTTI/文字列、公開 SDK(下)。

- **形式の入口**(FLEngine、Delphi): `TPluginFormatVST3`(`Plugin.Formats.VST3`)、`TPluginFormatVST2`、`TPluginFormatCLAP`。VST2 は名前表(`VSTPluginMain` の大文字小文字違い 9 通りの UTF-16 文字列)で GetProcAddress(`TVSTPlugin` が `AEffect`/`TDAEffectX` を扱う)。VST3 のプラグインは `TVST3Plugin`(`.Create`、`.LoadPlugin`、`.Destroy`)。
- **ホストが実装してプラグインへ渡すクラス**(FLEngine の RTTI): `TVST3HostInstance`(= `IHostApplication`)、`TVST3ComponentHandler`/`2`/`3`(= `IComponentHandler`/`2`/`3`)、`TVST3EditorFrame`(= `IPlugFrame`)、`TVST3EventList`(= `IEventList`)、`TVST3ParameterChanges` + `TVST3ParamValueQueue`(= `IParameterChanges`/`IParamValueQueue`)、`TVST3BStream`(= `IBStream`。`setState`/`getState`)、`TVST3AttributeList` + `TVST3Message`(= `IAttributeList`/`IMessage`。component と controller の連絡)、`TVST3UnitHandler`(= `IUnitHandler`)、`TVST3ContextMenu` + `TVST3ContextMenuTarget`(= `IContextMenu`/`IContextMenuTarget`)。FL 独自の拡張: `il.VST3Extensions`、`VST3_IPSLHostCommands`、`VST3_IPSLContextInfo`、`VST3_IPSLViewScaling`(名前は PreSonus 系の拡張に似る。読み)。
- **ホストが呼ぶプラグイン側のインタフェース**(FLEngine が 16 バイトの IID 定数として持つ物。`tables/vst3_iids.tsv`。公開 SDK の IID を `vst3_iid_scan.py` で Windows の COM 互換バイト順に変換して検索。**実測**): `FUnknown`、`IPluginFactory2`、`IComponent`、`IAudioProcessor`、`IEditController`、`IEditController2`、`IConnectionPoint`、`IUnitInfo`、`IProgramListData`、`IMidiMapping`、`IAudioPresentationLatency`、`IInfoListener`、`IPlugViewContentScaleSupport` と、ホストが実装する側の `IHostApplication`、`IComponentHandler`/`2`/`3`、`IPlugFrame`、`IEventList`、`IParameterChanges`、`IParamValueQueue`、`IBStream`、`IMessage`、`IUnitHandler`、`IContextMenuTarget`(計 26)。**含まれない物**: `IPluginFactory3`(`setHostContext`)、`IPlugInterfaceSupport`、`IProcessContextRequirements`、`INoteExpressionController`、`IKeyswitchController`、`IXmlRepresentationController`、`IComponentHandlerBusActivation`(FL は問い合わせないと読める)。`IPlugView` は `createView` の戻りとして使うので IID 定数は不要。
- **プラグイン側のモジュール入口**(Windows。公開仕様): バンドル `X.vst3\Contents\x86_64-win\X.vst3`(単一ファイル形式は 3.6.10 で非推奨。OTT は単一ファイル)、`Contents\Resources\moduleinfo.json`(FLEngine は `FindModuleInfoFile`/`TVST3ModuleInfo`/`TVST3ModulePluginInfo` でこれを読み、DLL を読む前にクラス一覧を得る。Serum 2 の `moduleinfo.json`: 4 クラス〔Serum 2 / Serum 2 FX の Audio Module と Component Controller〕、CID `56534558…`、Cardinality 2147483647)。DLL のエクスポート `InitDll`、`GetPluginFactory`、`ExitDll`。
- **QFC(`QuickFontCache_x64.dll`、Delphi)の役割(読み。逆アセンブルの call 列から。実行はしていない)**: export の `CreateVST3Instance(path)`(VA 0x58a200)は、プロセス内の辞書(`TDictionary<string, TVST3Instance>`、グローバル `0x5da740`)にパスで引き、無ければ `TVST3Instance` を作る。その `Load`(VA 0x58a3d0)は **`LoadLibraryW(path)` → `GetProcAddress(h, "InitDll")` を呼ぶ(戻りが 1 でなければ `FreeLibrary` して失敗)→ 補助関数(0x587390)でファクトリを得る**。補助関数は、別のモジュール(名前は変数 `0x5c97a0` が持つ。変数への書き込みは QFC 内に見つからず、**正体は未確認**)から `GetProcAddress("CreateVST3Dummy")` を引き、プラグインのモジュールハンドルを渡して呼ぶ(戻りは `TVST3DummyIntf` と読める)。その後、得た `IPluginFactory`(オブジェクトの +8)に対し、vtable の slot 0(`queryInterface`)で `IPluginFactory2`(IID 定数が QFC にある)を問い合わせて +0x10 に保存する。FLEngine の `TVST3Plugin.LoadPlugin`(`f_b4da00`)は `CreateVST3Instance` の結果を『InstanceInfo』として受け、『InstanceInfo not assigned - EXIT』『InstanceInfo.Factory not assigned - EXIT』のメッセージを持ち、`IsEqualGUID` でクラスの CID を比べ、ファクトリの vtable の 6 番目(`createInstance`)と 3 番目(`getFactoryInfo`)を呼ぶ(読み)。**したがって `docs/investigations/m2-inventory.md` の『VST3 のホスト実装は QFC の中』は、『モジュールの読み込みとファクトリの取得』については当たっているが、インタフェース単位のホスト(上のクラス群)は FLEngine 本体にある**(読み。訂正案)。
- **公開資料**(取得 2026-11-03、HTTP 200): (1) https://steinbergmedia.github.io/vst3_dev_portal/pages/Technical+Documentation/Workflow+Diagrams/Audio+Processor+Call+Sequence.html — 状態遷移 Factory →(createInstance)Created →(initialize)Initialized →(setupProcessing)Setup Done →(setActive(true))Activated →(setProcessing(true))Processing、`IAudioProcessor->process` は Audio Thread、その他は UI Thread。『`setProcessing` … may be called from real-time Audio Thread (must be lock-free and without memory allocation!)』。(2) https://steinbergmedia.github.io/vst3_dev_portal/pages/Technical+Documentation/Locations+Format/Plugin+Format.html — Windows のバンドル構造 `MyPlugin.vst3/Contents/x86_64-win/MyPlugin.vst3`、`Contents/Resources/moduleinfo.json`、『the VST 3 plug-in was defined as a single dll file … deprecated since VST 3.6.10』。(3) SDK のヘッダ(https://github.com/steinbergmedia/vst3_pluginterfaces、raw で 27 ファイル取得)の `DECLARE_CLASS_IID`/`DEF_CLASS_IID` 59 個(IID の出典。SDK のソースの長い引用はしない)。

### 3.2 『プラグイン側を別の wasm モジュールとして lift し、ホストが関数ポインタ/vtable でプラグインを呼ぶ』形の妥当性(読み)

- 既存の枠組みに**合う**: FLEngine は dsp_ippv2、Blend2D、QFC、FreeType、ILTools を『実在モジュール』として載せている(決定 0073/0082/0086/0103/0104/0106。生成 C を別の ImageBase にリベースして同じ wasm にリンク。`LoadLibraryW` は実在モジュールの名前に一致すれば成功し、`GetProcAddress` は lift した関数の疑似 VA を返し、呼び出しは `fle_dispatch`〔関数入口表〕が受ける)。プラグインの DLL も同じ形(x86-64 の PE DLL。`DllMain`、実装 TLS、`GetProcAddress` でエクスポートを引く)なので、同じ機構で載せられる、と読める。vtable の呼び出しは `fle_dispatch` を通る(ホストが `call [rax+0x30]` で `createInstance` を呼ぶと、lift した C は関数入口表で引き当てる)。
- 既存の枠組みで**足りない**物(§3.4)。特に: 実在モジュールの表は `RM_MAX = 8`(決定 0086。基底は測定用)、9 製品の DLL は計 **約 12〜20 個**(Serum 2、Vital、OTT、HeartCore、kHs 殻 6 種〔コード同一。殻を 1 モジュール + 種ごとの差分にするか、6 個別にするか〕、+欠品 3 つ)。
- 複数の wasm *インスタンス*に分けるのは別の設計(共有線形メモリ + 共有関数表)で、V8 の 1 モジュール 1 GiB の制限(Chrome 156 で実測、決定 0130)を回避する唯一の道だが、未検討(`browser-feasibility.md`)。-Oz なら単一 wasm に入る(§4.1)ので、**大きさだけを理由には分割は要らない**。

### 3.3 FL → プラグインの呼び出しの順序(公開資料 + 読み)

`LoadLibraryW` → `InitDll` → `GetPluginFactory` → `IPluginFactory2::getClassInfo2` ほか(または `moduleinfo.json`)→ `createInstance(Audio Module Class, IComponent)` → `IComponent::initialize(host)` →(controller が別クラスなら)`createInstance(Controller)` + `IConnectionPoint::connect` → `setBusArrangements`/`setupProcessing` → `setActive(true)` → `setProcessing(true)` → オーディオスレッドで `IAudioProcessor::process(ProcessData)`(AudioBusBuffers、`IParameterChanges`、`IEventList`)。状態は FLP の `PluginParams`(event 213)に VST3 のラッパーとして入り、`IBStream` で `setState`。

### 3.4 追加機構の一覧(lift 側で足りない物。読みとランタイム表の突き合わせ)

1. **実在モジュールの一般化**: 表の上限(`RM_MAX`)、パス→モジュールの対応(`...\Serum2.vst3\Contents\x86_64-win\Serum2.vst3` を実在モジュールに結びつける)、VFS にバンドルのフォルダと `moduleinfo.json` を置く、FL のプラグイン DB(`FL_STUDIO_.Plugins.ini`/`VerifiedIDs.nfo`)に相当する記録の供給。
2. **`GetModuleFileNameW` の実在モジュール**: 決定 0082 の D6 は『今は決めない。名前付き停止』。5 製品すべてが `GetModuleFileNameW`/`GetModuleHandleExW` を import している(データフォルダの探索に使う、読み)。D6 の決定が要る。
3. **C++ 例外**(MSVC x64 の `RaiseException`・`RtlLookupFunctionEntry`・`RtlVirtualUnwind`・`RtlUnwindEx`・`__CxxFrameHandler`): 『`.pdata` の UNWIND_INFO を読んでゲストのスタックを巻き戻し、catch の funclet に着地する』仕組みが要る。既存は Delphi の SEH の着地(決定 0132)だけ。throw/catch が起動や読み込みの中で実際に使われるかは未確認。
4. **スレッド**: `CreateThread`、スレッドプール(`CreateThreadpoolWork`)、`InitOnce*`、SRW/条件変数(決定 0092 に有り)、FLS。協調スレッド(決定 0122/0124)が前提。プラグインがオーディオと別のスレッドで走る設計(Serum の音色読み込みなど)は、仮想時計(決定 0147)との相性が未確認。
5. **未対応の命令**(§2.4): 最小は Serum 2 の 3,180 サイト(VEX/BMI/SSE4)、OTT 1,264、kHs 233、HeartCore 18,302、Vital 195,141。
6. **CPUID**: 現行は SSE2 まで。プラグインは AVX2/AVX-512/SSE3 を前提にしている可能性(HeartCore の SSE3、Vital の AVX)。
7. **OS の窓・GDI/D2D/D3D・OpenGL**: プラグインの窓を開く場合のみ。開かない運用でも、生成時のタイマー・window class 登録(`CreateWindowExW`、`RegisterClass*`、`SetTimer` を 4 製品とも import)は通る。
8. **ネットワーク・認証**(§4.4): WinINet/WinSock/CRYPT32/`WinVerifyTrust`/レジストリを、ブラウザでは**失敗として返す**必要がある。失敗時に待ち続けないか(タイムアウト、リトライ、スレッドのブロック)は未確認。
9. **ファイルとデータ**: Serum 2 は SQLite のデータベースを作る/開く(文字列に `CREATE TABLE IF NOT EXISTS Presets`、`Could not open Serum 2 database`)、FLAC を読む(multisample)、`SHGetKnownFolderPath`、内容フォルダの走査。書き込み可能な VFS(OPFS)と、1.41 GB の内容データの配信/持ち込みの方針が要る。
10. **CRT**: Vital は動的 CRT(`msvcp140`/`vcruntime140`/UCRT)を使うので、それらの DLL を lift するか仮想化する必要(供給も)。

## 4. 規模と制約の見積り

### 4.1 (a) 生成 C と wasm の大きさ(`tables/size_estimates.tsv`、推測を含む)

| | closure 命令 | 生成 C(実測) | wasm -O2(推測) | wasm -Oz(推測) |
|---|---:|---:|---:|---:|
| Serum 2 2.1.5(19.2 MB) | 2.39M | 215.1 MB | 164 MB | 79 MB |
| Vital | 2.02M | 178.9 MB | 137 MB | 66 MB |
| HeartCore(Kilohearts の 8 インスタンス分) | 4.93M | 449.9 MB | 344 MB | 165 MB |
| OTT | 0.49M | 44.6 MB | 34 MB | 16 MB |
| kHs 殻(1 モジュール) | 0.04M | 4.0 MB | 3 MB | 1 MB |
| **取り出した製品の計** | 9.9M | **892 MB** | **683 MB** | **328 MB** |
| 現行の 6 モジュール(決定 0134/0137 の実測) | — | 872.2 MB | 667 MB(実測) | 321 MB(実測) |
| 現行 + 取り出した製品、単一 wasm | | 1,765 MB | **1,350 MB(1 GiB の 126%)** | 649 MB(60%) |
| 現行 + Serum 2 だけ | | 1,087 MB | 832 MB(77%) | 400 MB(37%) |
| (仮定) Serum 2 が ini の古い記録の 39 MB だったら | | 437 MB | 335 MB → 現行と合わせて 1,002 MB(93%) | 161 MB → 482 MB(45%) |
| 欠品 3 つ(上限。全バイトがコードと仮定) | | Spectral Gate ≤ 116、GClip ≤ 19、Ozone Imager 2 の VST3 ≤ 48 MB | ≤ 89 / 14 / 37 | ≤ 43 / 7 / 18 |

比の根拠: 現行 6 モジュールの C 872.2 MB(FLEngine 215.8、dsp_ippv2 491.1、Blend2D 101.4、QFC 18.2、FreeType 10.0、ILTools 35.7。決定 0134)に対し wasm は -O2 で 667,115,949 B(0.765)、-Oz のバンドルで 320,731,834 B(0.368。決定 0137)。プラグインのコードが同じ比で膨らむ**とは限らない**(推測)。1 命令あたりの C は 88〜95 B(6 DLL で一定、実測)。**Serum 2 単体の C は FLEngine 本体と同じ桁**(215 MB 対 215.7 MB)。

- **答え(Serum 2 を足すと V8 の単一 wasm 上限を超えるか)**: 現行の Serum 2 本体(19.2 MB)なら、**-O2 でも +164 MB で 832 MB(77%)、-Oz なら 400 MB(37%)で超えない**(推測)。仮に 39 MB だったとしても -O2 で 93%、-Oz で 45%。**超えるのは、9 製品全部を -O2 で足したとき**(1,350 MB)。-Oz なら 649 MB(60%)で入るが、-Oz は速さを捨てる(§4.2)。
- 関数の大きさ(`tables/felift_c_function_sizes.tsv`、生成 C を走査して削除済み): 最大の関数は Serum 2 が C 1.32 MB(15,521 行)、HeartCore 0.82 MB、Vital 0.49 MB。FLEngine(最大 4.2 MB が 2 本)より小さい。V8 の関数あたりの locals 50,000 の上限(決定 0115 の 19 関数のような扱い)に当たるかは未確認(コンパイルしていない)。
- **ビルド時間・メモリ(推測。決定 0134/0130 の実測を比例)**: C2(-O2)のビルドは 872 MB の C で 3,922 s(65 分、JOBS=2、ピーク 5.1 GB、最大プロセス 3.1 GB)、-Oz は 433 s(7 分、ピーク 2.8 GB)。892 MB を足すと **+67 分(-O2)/+7 分(-Oz)**、1 単位(10 MB の C)あたりの clang の RSS は同じ(-O2 で最大約 3.1 GB、-Oz で約 1.3 GB)。fe-lift 自体は 4〜10 秒。-O0 の『units 段』は dsp の 88 MB の 1 本が RSS 8.8 GB になった過去があるので、単位を 10 MB に切る既存の流儀(`split_bytes.py`)が前提。**Serum 2 の 215 MB(単体で FLEngine と同桁)は、22 単位**になる。
- 配信量(推測): brotli q11 は生の 4〜6%(決定 0130 の実測 4.2〜5.7%)。+328〜683 MB の wasm で **+14〜39 MB**(現行の wasm の brotli は 26.5 MB〔-O2。決定 0134〕/13.6 MB〔-Oz。決定 0130〕に対し)。P1(起動 10 秒)に影響するのは、プラグインを起動のバンドルに入れる場合だけ(FLP が使うときに遅延読み込みなら影響しない)。
- 線形メモリ(推測): イメージの合計 Serum 33 + Vital 17.5 + HeartCore 98 + OTT 3 + 殻 6×0.26 = 約 153 MB が現行の 947 MB に加わる。**プラグインのインスタンスごとの実メモリ(Serum 2 のサンプル・ウェーブテーブルの展開、6 インスタンス)は静的には出せない**(未確認)。wasm32 の 4 GiB と、現行が設定する上限 2 GiB に対し、危険は否定できない。

### 4.2 (b) 実時間の計算: lift したコードの速さ

**既存の記録**: FLEngine は空プロジェクトで native -O2 でも実時間の約 1/20(決定 0144)、ウィザード表示中は 1/8〜1/9(決定 0146 の S1)。ロゴ描画が支配で、プラグインの DSP ではない。プラグインの DSP の速さの実測は無い。

**今回の微小ベンチ(実測。`tables/microbench_lift_slowdown.tsv`、`tools/microbench/`)**: プラグインでも FL でもない**自作の合成カーネル 3 本**(K1 = packed SSE の `out[i] += in[i]*g`、K2 = scalar double の biquad、K3 = 整数の xorshift+表引き+rotate)の機械語を `as` で作り、合成 PE に入れ、`fe-lift lift --pdata` で lift し、プロジェクトの runtime ヘッダで native と wasm(node 24.19、emcc 6.0.12)にビルド。同じ機械語を CPU で直接実行した時間と比べた。1 ブロック 1,024 サンプル、L1 に収まる。**共有機(ロード平均 15〜20)で ±25%**。

| 倍率(lift 版 ÷ 直接実行) | K1 packed SSE | K2 scalar double | K3 整数 |
|---|---:|---:|---:|
| native -O2、旧フラグ(`-fno-builtin`) | 100〜130 | 19.5〜21.5 | 6.9〜7.2 |
| native -O2、新フラグ(`-fno-builtin-fprintf -fno-builtin-fputs`。決定 0151、hle1) | 80〜108 | 19.8 | 2.6〜2.8 |
| wasm -O2、旧 | 205〜215 | 29〜32 | 15 |
| **wasm -O2、新** | **145〜160** | **17〜20** | **5.5〜6.2** |
| wasm -Oz、旧(配信中のバンドルの最適化) | 265〜300 | 45〜48 | 37〜39 |
| **wasm -Oz、新** | **230〜245** | **44〜45** | **33〜37** |
| what-if(SSE 演算を既定 MXCSR 仮定の素の C に替えた。wasm -O2、SIMD 無し / `-msimd128`) | 24〜26 / **9〜11** | **2.7〜3.2** / 8.8 | 6.1 / 6.1〜7 |

- 直接実行の基準(ns/サンプル): K1 0.063〜0.084、K2 1.93〜2.11、K3 1.19〜1.34。lift 版(wasm -O2、新): K1 10.2〜11.3、K2 34〜40、K3 7.0〜7.9。
- 読み方: (i) 新フラグ(builtin 有効)は整数・フラグ系を大きく下げる(K3 は native で −60%)が、**SSE の浮動小数(`fle_mx_*`: レーンごとに MXCSR・NaN・denormal を厳密に処理)が支配的で、倍率は 10 倍台に下がらない**。(ii) runtime オブジェクトの最適化(-O0 か -O2)は、これらのカーネルに影響しない(`fle_ld`/`fle_st`/`fle_mx_*` は header の static inline で、生成 C 側でコンパイルされるため)。(iii) -Oz は新フラグでもほとんど改善しない。(iv) K2 の native/`-msimd128` の what-if が遅いのは、8 バイトの部分書き込みの直後に 16 バイトの `X[]` コピーが走る store-forwarding の停滞(SIMD 無しの wasm はコピーが 2×i64 なので無い)。
- 限界: 3 本の合成カーネルで、実プラグインの動的な命令混合ではない。静的な混合比は参考にならない(`tables/instruction_mix.tsv`: FP 演算は静的に Serum 2 で 2.3%、OTT 2.5%、Vital 8.6%、HeartCore 1.3%。ホットループは静的な比率から決まらない)。

**外挿(推測。前提を全部書く)**:
1. 実プラグインの時間配分 = 整数/メモリ系 `w_i`、scalar FP `w_s`、packed FP `w_p`(ネイティブ時間の割合、和 1)とすると、wasm -O2・新フラグでの倍率 S ≈ 5.8·w_i + 18.5·w_s + 152·w_p。例: (0.7, 0.25, 0.05) → **約 16 倍**、(0.4, 0.4, 0.2) → **約 40 倍**、(0.2, 0.2, 0.6) → **約 96 倍**。-Oz なら 1.5〜5 倍悪い。
2. 実時間 1 スレッド(AudioWorklet に音を渡すワーカー)で、FL エンジン自身(ミキサー、ネイティブ 25 インスタンス、UI)を除いて**プラグイン群だけで**予算を使い切ると仮定すると、プラグイン群のネイティブでの CPU 負荷 X(1 コアに対する割合)が **X ≤ 1/S** でなければならない。S=16 → X ≤ 6%、S=40 → 2.5%、S=96 → 1%。
3. X は**この機械では決められない**(プラグインを実行しない)。参考(第三者の逸話で、環境は不明。websearch の検索結果の抜粋だけで、ページ本体は取得していない): Serum 2 の利用者報告に、軽い patch で『ほぼ 4%』(https://vi-control.net/community/threads/serum-2-s1-patches-compared-in-s2-lower-volume-less-cpu-hit.161632 、2025-03)、重い patch で『1 打鍵ごとに 10%』(https://www.reddit.com/r/serum/comments/1jekg1r/serum_2_cpu_going_crazy)、AMD で Serum 1 の 2〜3 倍(KVR 公式スレッド、2025-03)という書き込みがある。1 インスタンス 4〜10% を Serum 2 が最大 4 つ同時に鳴る区間に単純に足すと **Serum 2 だけで 16〜40%**(推測)。S=16 でも許容 X ≤ 6% を 3〜7 倍超える。このプロジェクトは 21 インスタンス(Serum 2 が最大 4 つ同時に鳴る区間が 31 秒、同時に有効なサードパーティのエフェクトが最大 11。reference-project.md §2.5)で、X が 1〜6% に収まるとは考えにくい(推測)。FL の Windows 実機の CPU メーター値(依頼者の PC)があれば X の桁が決まる(依頼者に質問できないので、PM の判断で別の手段が要る)。
4. したがって **A(現行の忠実度のまま全部 lift)は届かない側に強く傾く(推測)**。what-if(忠実度を落とした素の C)が実プラグインでも効くなら S は 3〜10 倍程度になり、X ≤ 10〜30% まで許せる(この範囲なら届く可能性が出る。未検証)。
5. ブラウザの音は AudioWorklet のリングバッファ経由(決定 0139。出力のリード既定 60 ms。ワーカーで先に作り、リングの枯渇回数 0 が P3)。決定 0139 の当時の実エンジンは仮想時計が実時間の約 0.5% しか進まずアンダーランが 98%(旧状態。決定 0147 が仮想時計を是正)。プラグインの `process` が 1 回でもリードを超えて遅れれば途切れる(hog 100 ms で 7.8% の枠が欠落、同決定の試験)ので、**平均だけでなく最悪の 1 ブロックの時間**が要る(未測定)。

**命令セット(AVX/AVX2/FMA/AVX-512)を wasm SIMD(128 ビット)へ対応づけるコスト(census からの見積り。`tables/wasm_simd128_expansion.tsv`)**:
- 数え方: xmm 形 = wasm SIMD128 の 1 演算、ymm = 2、zmm = 4(最小の見積り。AVX-512 のマスクレジスタ・gather・permute、AVX2 の可変シフト・gather・`vpermd` は 1:1 でなく数演算〜スカラー化になり、上の数字は下限)。
- 結果(post-SSE2 のベクトル命令の個数 → SIMD128 演算数、倍率): **Serum 2 2,735 → 3,357(1.23 倍)**、OTT 1,059 → 1,133(1.07 倍)、kHs 殻 328 → 565(1.72 倍)、HeartCore 38,282 → 86,950(2.27 倍。AVX-512 の zmm が 3.87 倍)、Vital 210,549 → 455,676(2.16 倍。AVX-512 が 3.36 倍、FMA が 2.37 倍)。
- つまり **Serum 2・OTT は AVX 系の量が少ないので、移植コストの倍率は小さい(数 % の命令)**。Vital と HeartCore は AVX-512/AVX2 の量が大きく、全部を実装するなら 2〜4 倍の演算数になるが、**CPUID を SSE2 のままにして、プラグインが SSE2/SSE4 の経路を選ぶなら、これらの経路には入らない**(その経路を持つ前提。Vital・HeartCore・Serum 2 とも SSE/SSE2 の経路は静的に存在する〔§2.3〕が、CPUID でゲートされていることは未確認)。
- **FMA の落とし穴**: wasm の SIMD128 には融合積和が無い(`relaxed-simd` の `madd` は融合するかが実装依存で、結果が環境で変わりうる。公開仕様の理解で、今回は取得して確認していない)。FMA を `mul`+`add` に分けると丸めが変わり、ビット厳密には元と一致しない。ビット厳密にするならソフトウェアの fma(1 命令あたり数十演算)が要る。FMA を含む関数は Serum 2 で 37 個(334 サイト)、Vital で 625 個(30,136 サイト)。この理由でも、CPUID は FMA を名乗らない現行の方針(決定 0009)が安全側である。
- 決定 0075/0009 の方針(CPUID は SSE2 まで)を保つと、AVX 系の実装は不要になるが、**そのかわりプラグインが AVX2 を必須とする場合に起動で拒否される**(Serum 2 についてのメーカー発言。未確認)。

**HLE が使えない理由**: HLE(重い関数を、結果ビット同一のホスト実装に置き換える。決定 0151 の方針)の前提は『元のアルゴリズムが分かること』。Serum 2/Vital/Kilohearts のアルゴリズムは非公開(ソース無し、仕様書無し)で、状態も独自形式(Serum の XferJson + zstd、Kilohearts の独自 preset)。同じ音を出す別実装(ウェーブテーブル合成・ユニゾン・フィルター・エフェクトの再実装)は**別の製品を作ること**で、『このプロジェクトの音』にならない(P3 の定義に反する)。リバースエンジニアリング自体もライセンス上の問題がある(未確認)。一方、**プラグインの内部の小さなカーネル**(FFT・リサンプラ・IPP の関数。Vital の `IPPCODE` は Intel IPP で、公開仕様がある)に限った HLE は、dsp_ippv2 と同じ流儀で成立しうる(読み)。

### 4.3 (c) GUI

- P2 の UI は FL の主窓(プラグインの窓を開かない運用なら、プラグインの GUI コード〔Direct2D/D3D11/GDI+/bgfx/VSTGUI 風〕を動かさない)。**プラグインの編集画面が必要かどうかは P2/P3 の定義の外**(プラグインの GUI を動かすのは別の目標)。
- GUI のコードの割合: §2.5(直接の GUI API 呼び出しは関数バイトの 0.2〜3%、RTTI のクラス名で粗く 31〜61%。**上限は未確認**)。lifter は全関数を lift するので、サイズには GUI 分も入る。
- 窓を開く場合の追加要件: Serum 2 は Direct2D+D3D11(`D3D11CreateDevice`)、Vital は bgfx(OpenGL/D3D11 を LoadLibrary)、OTT は GDI+ — いずれも既存のランタイムの GDI 仮想化(決定 0053/0054/0108/0111)の範囲外。

### 4.4 (d) 認証・ライセンスの形(読み。回避しない。文字列は短い断片)

| 製品 | 見えるもの |
|---|---|
| Serum 2 | import: `WININET`(`InternetOpenUrlA`/`InternetReadFile`/`HttpQueryInfoW`)、`WS2_32`(`socket`/`bind`/`listen`/`accept`/`send`/`recv`)、`ADVAPI32` の `CryptAcquireContextA`/`CryptCreateHash`/`CryptHashData`/`CryptGetHashParam`、`RegOpenKeyExA`/`RegQueryValueExW`。文字列断片: 『authorization denied』『authorizer malfunction』『not authorized to use function: %s』『Locate License』『Update Available from Your Account on xferrecords.com』、URL は `https://xferrecords.com`。ライセンスが無いときの挙動(デモ/無音/拒否)は**静的には未確認** |
| Vital | アカウント: 『Log In』『Sign in』『Forgot Password』『<CREDENTIAL-NAME>』『PASSWORD_LOGIN_DISABLED』、URL `https://account.vital.audio`、WinINet 13 + WS2_32 18(`getaddrinfo` ほか)。無料版の動作は未確認 |
| OTT | 認証・ネットワーク import 無し(文字列に license 系無し) |
| Kilohearts(HeartCore) | **PACE iLok**: URL `https://activation.paceap.com`、`activation.ilokcloud.com`、文字列『License Support installer, which can be downloaded from iLok.com』『unlicensed snapin』『trial remaining』『`--license`』『"LicenseD " is not at the beginning of the cert's BusinessCategory』。`CRYPT32` の証明書チェーン検証、`WinVerifyTrust`。使う 6 製品(Filter/Gain/Haas/Pitch Shifter/Tape Stop/Disperser)が無料の枠かは**未確認** |

ブラウザでは、ネットワークは(同一オリジン以外)通れず、レジストリ・iLok のサービスは無い。**これらの呼び出しが『失敗』を返したときにプラグインが動くか、止まるか、無音になるかは、実行しないと分からない(未確認)**。ライセンスの回避・無効化は行わない(依頼者の私的な利用の範囲でも、提供者の認証を迂回する実装は作らない)。オンライン認証が通らないと動かない仕様なら、その製品は P3 の対象から外れる(= 依頼者の判断)。

## 5. 結論と選択肢(決めない。推奨は §5.6)

### 5.1 案 A: 全部 lift(現行の忠実度)

- **作業の種類と順序**: (1) OTT で最小の実験(§5.5)→ (2) kHs 殻 + HeartCore(98 MB のイメージ、PACE、TLS コールバック 4)→ (3) Serum 2(最大の依頼。C++ 例外、スレッドプール、SQLite のファイル、ネットワークの失敗)→ (4) Vital(動的 CRT の供給、AVX-512)→ (5) 欠品 3 つ。各段で未対応命令(§2.4)と import の応答(§2.2)を足す。大きさは -Oz で単一 wasm(649 MB)に入る。
- **詰まる所**: 速さ(§4.2。合成カーネルの実測で wasm -Oz 33〜245 倍、-O2 5.5〜160 倍)。単一スレッドの協調モデル。ビルド時間(+67 分)。配信と持ち込みの量(Serum 2 の内容 1.41 GB)。
- **失敗の兆候**: 初期化(`InitDll`/`createInstance`)だけで数十秒かかる/`process` 1 ブロックの実時間比が 1 を大きく超える/ネットワーク失敗で待ち続ける/名前付き停止(`fle_unsupported`、`RaiseException`)が出る。

### 5.2 案 B: プラグイン無しで再生

- 事実: 参照プロジェクトは楽器チャンネル 8 のうち **7 つ(Serum 2 ×6、Vital)が鳴らない**(FL は『Missing plugin』の警告を出して無視する。reference-project.md §4.3)。FLEX(FL 内蔵)と音声クリップ 48 個、ネイティブのエフェクトは鳴る。マスターの Ozone Imager 2/GClip/Spectral Gate/OTT も欠ける。
- **P3 の定義(所定のプロジェクトの Song Mode の再生で音が途切れない)を満たしたことにしてよいかは、依頼者の判断が要る**。このプロジェクトの目的が『サードパーティのプラグインを使う一般的な規模のプロジェクトで十分な性能』である点(`performance-targets.md` §1)から、満たさない側と読めるが、決めるのは PM/依頼者。
- 必要な作業: ほぼ無し(FLP の読み込みと、プラグインの欠品の警告ダイアログの処理)。供給物: なし。

### 5.3 案 C: レンダリング済み音声(**実エンジン/実プラグインの実行ではない。P3 を満たしたことには数えない**)

- 例: 依頼者の PC の FL で各プラグインのトラックを書き出し、ブラウザでは音声クリップとして再生。作業: 書き出しの手順、FLP の差し替え。**進捗として報告してはならない**(AGENTS.md の『進捗の誠実性』)。必要なら『代役』と明示して別の入口に置く。

### 5.4 案 D: その他(事実と作業の種類のみ)

- **D1 忠実度を落とした『音声用の lowering』**: 実測した what-if(§4.2 の末行)の延長で、(a) SSE/SSE2 を wasm SIMD128 に 1:1(`f32x4.mul` など。IEEE の丸めは同じ。MXCSR の丸めモード・FTZ/DAZ・NaN ペイロードは捨てる)、(b) `R[]`/`X[]` をメモリでなく wasm のローカルにレジスタ割り付け(関数内で)、(c) 使われないフラグの計算を省く(死活解析)、(d) 128 ビット VEX は 1:1、256/512 ビットは 2/4 分割。**期待**: 合成カーネルで packed 145〜160 → 9〜11 倍、scalar 17〜20 → 約 3 倍(what-if。レジスタ割り付けとフラグ省略を含まない値)。**リスク**: 実プラグインでの効果は未検証。丸めモード/FTZ を実プラグインが変更しているか(多くの音声プラグインは FTZ/DAZ を立てる)で、denormal の扱いが変わり音がわずかに変わる(可聴ではないと読むが、照合手段が無い)。この案は FLEngine 本体の P2(UI)にも効く(同じコード生成)。
- **D2 先行(オフライン)レンダリング + バッファ再生**: 実プラグインを**実際に実行**して曲全体(129.6 s)を非実時間で先に音声にし、十分たまってから再生する。『途切れない』は満たせるが、再生開始までの待ち時間が `129.6 s × X × S`(X = プラグイン群のネイティブ負荷、S = 倍率。例 X=15%、S=16〜96 → 5〜31 分、推測)。FL のエンジンに『先行レンダリング』のモードを足す必要(FL の既存の『書き出し』機能〔WAV/MP3 書き出し = G10〕に近い)。**P3 の趣旨(実用的な性能)を満たすかは依頼者の判断**。実プラグインの実行なので『代役』ではない。
- **D3 複数スレッドへの分割**: ミキサーのチャンネル/プラグインごとにワーカー(SharedArrayBuffer)へ。現行は協調スレッド(THREADS=1、決定 0124)で、各スレッドが同じ線形メモリを前提にする設計のため大改修。
- **D4 ユーザー PC 上のネイティブのブリッジ**(ローカルプロセスでプラグインを動かし、音声だけブラウザへ): 『ブラウザ上で FL Studio を動作させる』というゴールに反する可能性。事実のみ。

### 5.5 最初の小さな実験(案 A/D1 の最初の段。この調査では実行していない)

**OTT を lift して、`InitDll` → `GetPluginFactory` から 1 ブロック処理するネイティブの実験**(`lift/harness/two/` の流儀で、OTT.vst3 を実在モジュールとして載せる最小のハーネス):

1. `fe-lift lift OTT.vst3 --pdata`(実測 0.9 秒、C 44.6 MB、未対応 1,264 サイト、import は応答行あり 217/380)。
2. ハーネス(native): ゲストの `InitDll()`(戻り 1 を確認)→ `GetPluginFactory()` → vtable の `countClasses`/`getClassInfo`(CID が FLP の OTT と一致するか。`56535458-6654-546F-7474-000000000000` と読める)→ `createInstance` → `initialize(IHostApplication)` → `setBusArrangements`(ステレオ)→ `setupProcessing`(44.1 kHz、512 フレーム)→ `setActive(1)` → `setProcessing(1)` → `process` を N 回。
3. 測る物: (i) 名前付き停止の有無と種類(未対応命令、未応答 API)、(ii) 1 ブロックの実時間比と、数えたゲスト命令数/秒、(iii) 無音入力→無音出力、正弦波入力に対する出力の決定性(同じ入力で 2 回同一)。**正しさの oracle は無い**(実機で動かした結果を持っていない)。
4. 失敗の兆候: AVX/FMA の経路に入る(CPUID の応答を変えると回避できるか)、`RaiseException`、GUI の生成(`CreateWindowExW`)で止まる、`GetModuleFileNameW`(D6)で止まる。
5. この実験の価値: OTT は小さい(44.6 MB の C)ので 1 日以内に回る見込みで、(a) VST3 ホストの手順がゲストの vtable 呼び出しで通るか、(b) 『プラグイン 1 個あたりの実測の倍率 S』の 1 点を実プラグインで得られる。

### 5.6 推奨(PM が決める)

1. **まず §5.5 の OTT の実験**(native → wasm)。結果の S と停止の種類で A/D1 の判断をする。
2. 並行して**依頼者側の供給**を確認する: Spectral Gate/Ozone Imager 2/GClip のインストーラ、Vital 用の Windows ランタイム(`msvcp140.dll`/`vcruntime140*.dll`/UCRT)、(P3 に必要なら)Serum 2 の内容データのうち FLP が参照する分。
3. **性能の本命は D1(エンジン全体のコード生成の見直し)**。プラグインの P3 も FLEngine の P2 も同じ原因(レジスタ・フラグ・SSE の厳密さ)で遅い。what-if で桁が下がる見込みがあるが、忠実度の方針(MXCSR のビット厳密さを捨てるか)を PM が決める必要がある。
4. 案 B を P3 の達成と数えるか、案 D2 を許すかは**依頼者の判断**(事実は §5.2/§5.4)。案 C は P3 に数えない。
5. 案 A をそのまま実装するのは、速さの見込みが 1〜2 桁足りないので推奨しない(推測)。

## 6. 確認できなかったこと(未確認)と注意

- プラグインの動的な命令混合・実際の CPU 負荷 X・初期化の時間・メモリ使用量(**実行しないと決まらない**)。
- Serum 2 が AVX2 を要求して起動を拒否するか、ライセンス失敗時の挙動(Serum 2/Vital/Kilohearts)。
- Serum 2 の工場出荷データを FLP が参照するか。Vital の工場データの取り出し(実行ファイル 4 つだけ取り出した)。
- QFC の `CreateVST3Dummy` を引くモジュールの正体、`TVST3Plugin` が実際に呼ぶ vtable のスロット一覧(IID の存在と RTTI からの読みに留まる)。
- 欠品 3 つのバイナリ(サイズは ini の値のみ)。Ozone Imager 2 の VST3 と 75.7 MB の DLL の関係。
- wasm のビルド(プラグインの C を clang/emcc に通していない。大きさ・ビルド時間・locals 超過関数は比例からの推測)。
- 生成 C の動作の正しさ(プラグインのコードは実行していない)。
- 微小ベンチは 1 台・共有機・3 本の合成カーネル(±25%)。K2 の native −msimd128 の遅さの原因(store-forwarding)は読み。

## 7. 規則違反の自己申告

- 存在しない `bash` ツールを 1 回呼んだ(`echo test` のみ。ハーネスが shell として実行した。副作用なし。報告 1 で申告済み)。
- sleep/nohup/disown/systemd-run/`&`/while 待ちは使っていない。バックグラウンドは `cargo build`(shell ツールの `background: true`)の 1 回だけ。
- 製品の機械語の引用は 0 行、文字列は短い断片のみ(逆アセンブルは Delphi 製の QFC/FLEngine〔FL 自身〕の関数の call 列の要約で、プラグインの機械語は引用していない)。
- インストーラ(.exe)・プラグインは実行していない(Wine を含む)。`7z`/`unzip`/自作の読み取りツール/`pefile`/`capstone`/`openssl asn1parse`(署名の読み取りのみ)だけ。
- 外部ネットワーク: 公開資料の取得のみ(websearch 1 回〔Serum 2 の CPU 負荷の逸話。結果の抜粋だけ〕、Steinberg の dev portal 2 ページと SDK ヘッダ 27 ファイル。ほかに innoextract の `exefilter.hpp` を 1 件取得し、同じ `exefilter.cpp` は 404)。取得した物に依頼者でない指示は含まれていなかった。
- 書き込み先は `lift/out-vst1/`、`lift/out-git/vst1-results/`、`/tmp/opencode/vst1/` のみ。tracked ファイルは編集していない。git の変更系操作はしていない。

## 8. 成果物

`lift/out-git/vst1-results/`: `vst3-hosting.md`(本文)、`README.txt`(再現手順)、`tools/`(自作スクリプトと微小ベンチ)、`tables/`(上で参照した TSV)。`lift/out-vst1/`: `bin/`(取り出した `.vst3` など 約 165 MB)、`census/`(fe-lift の出力)。インストーラの中身・プラグインのバイナリは成果物に入れていない(git に入れない)。
