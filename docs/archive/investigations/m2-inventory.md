# M2 の棚卸し: FL Studio が使うプラグイン・DLL の全体像（インストーラ 26.1.7.5653 の静的調査）

実行日: 2026-10-28。基準: ブランチ main、HEAD **e071fc0**（ワークツリーの追跡ファイルは HEAD と一致。`git status` の未追跡のうち、この調査が作ったのは `m2-inventory.md` と `m2-inventory/` だけ。ほかは他の作業のもの）。**調査のみ。何も決めない**（決定、runtime、生成器、既存文書、`lift/` は編集していない）。未決の点は §7 に列挙した。

対象: `/home/ubuntu/ダウンロード/flstudio_win64_26.1.7.5653.exe`（1,106,033,456 B、SHA-256 `04393e7946c7369aaa5cd9ce1c755803e922146289e1ae3263629a44f4e5fc02`。**読み取りのみ**。今回 `sha256sum` で再計算して一致を確認）。これは NSIS のインストーラで、`tools/extract_nsis2.py` の関数で読んだ。

## 0. 要約

1. **インストーラの中身**（実測）: EXTRACTFILE **27,574 エントリ**（データが同一のものを 1 つと数えると 25,164）。展開後の合計 **2422.9 MB**（一意 2313.4 MB。1 MB = 10^6 B）、インストーラ内の圧縮後は 1105.4 MB。**コード（PE）458 エントリ・一意 313・1333.7 MB**、データ 27,108 エントリ・1089.0 MB、NSIS 補助ファイル 8。
2. **製品として入る x64 の PE**（NSIS 補助、第三者インストーラ、アンインストーラ、TEMP のコピー、x86 を除く）は **258 個（一意）、954.3 MB**。x86 の PE は一意 55 個（NSIS 補助 6 を含む。307.9 MB。うち 201.7 MB は版情報が「Microsoft Edge Update Setup」の第三者の EXE、31.1 MB は FL Cloud Plugins Installer）。
3. **プラグイン**（`Plugins\Fruity` 配下で `CreatePlugInstance` を export する PE）: **160 エントリ（一意 159）、一意 654.4 MB**。うち **Delphi 製 132 個**（Embarcadero Delphi 36.0 の Win64。FLEngine/ILTools と同じ作り）と MSVC 製 27 個。プラグインの補助 DLL が別に 22 個（x86 の 1 個を含む）、FL Studio VSTi 2 個と ReWire 2 個。
4. **データ**: 1089.0 MB（一意 1056.1 MB）。大半は `Data\Patches` の音声と FL の状態・プリセット類。`Plugins\Fruity` 配下のデータ（サンプル・プリセット）が別に約 200.1 MB。
5. **InitUnits に必須で未実在化の同梱 DLL: 0 個**（実測）。6 モジュール構成で DllMain が `init_count=942 of 942` で完走したトレース（`a4-six-module-trace/trace.tsv.gz`、全 LoadLibraryW/A/ExW の名前）に、読まれた同梱 DLL は **dsp_ippv2、Blend2D、QuickFontCache（FLEngine から）、freetype（QFC から）、ILTools の 5 個だけ**で、ilsvg、ildsp、oggio、wavpack、lameenc、python312、ilvideo、speaker、webview2loader、beat_tracker、elastique の名前は 0 件。静的な読み（`bundled-dll-inventory.md` §6.2）で unit 895/898 に届きうるとされた ildsp、python312 なども、完走では呼ばれていない。**ただし A4 は DllMain(ATTACH) の完走であって、`CreateFruityInstance` 以降（ウィンドウ作成、プラグイン探索、音声）は実行していない**ので、それ以降に必要な DLL は未測定（§5.1）。
6. **発見（読み。§4）**: FL Studio のネイティブ・プラグインは FLEngine の中ではなく**別の DLL**（1 プラグイン 1 DLL、各 3.6〜7.6 MB（pdata 11,513〜23,540）の Delphi 製が 132 個）で、FLEngine が `LoadLibrary` + `GetProcAddress("CreatePlugInstance")` で読むと読める（xref。実行は未確認）。**外部 VST3 ホストは QuickFontCache_x64.dll の中**（`CreateVST3Instance` を export。QFC は 5 個の実在モジュールの 1 つ）、VST2 の入口は FLEngine の中の名前表（`VstPluginMain`/`VSTPluginMain`/`main` と大文字小文字の変種。Fruity Wrapper、Sytrus、Tuner にも同じ文字列がある）、CLAP は FLEngine の 2 か所（`clap_entry`）。
7. **発見（実測。§4.3）**: Delphi 製プラグインの関数は、FLEngine∪ILTools∪QFC の関数と**バイト列がほぼ同じ**（相対参照の変位を除いて一致。小さなエフェクトで関数の約 97〜99%・コードのバイト数の 88〜96%、大きなシンセ（Sytrus、Harmor、FLEX、SG）で関数の 89〜93%・バイト数の 67〜82%）。132 個の和集合でも、既存 3 モジュールに無い関数は 28,779 個・10.4 MB（コード）しかない。プラグイン 132 個の関数の延べは 1,819,636、延べコードは 266.8 MB。**同一コードを共有できるか（データのアドレスの扱い）は未検討**。
8. **見積もり（読み。§6）**: 6 モジュール wasm は 389,439,333 B（実測）。pdata が覆うコード 1 B あたりの wasm は FLEngine 10.98 倍、dsp_ippv2 11.1 倍、残り 4 モジュール 14.1 倍（実測値からの逆算）。この 11〜14 倍で、(b) の 10 DLL をすべて lift すると **89〜114 MB**、Delphi プラグイン 132 個を**共有なしで 1 個ずつ**lift すると **2929〜3756 MB**、既存 3 モジュールと共有できた場合の残りは約 114〜146 MB。ffmpeg 7 DLL は 491〜630 MB。

## 1. 凡例・方法・範囲

- **実測** = 今回のコマンドの出力。**引用** = 既存文書の記述（再測定していない）。**読み** = 静的な読みと推論（実行で確かめていない）。**未確認** = 確認していない。
- MB は 10^6 B。「一意」は NSIS のデータオフセットが同じエントリを 1 つと数えた値（同じデータを複数の場所に置く `System.dll` などがある）。
- ファイルの大きさと SHA-256 は、`tools/extract_nsis2.py` の `read_header`、`extract_files`、`stream_file`（LZMA の復号）を**同じプロセスの中で**呼んで、全 25,164 データを**メモリ上で復号**して数えた（書き出しなし。`--list` はファイルの大きさを出さないため）。復号エラー 0。PE のヘッダは `pefile` 2024.8.26 で読んだ（実行はしていない）。`--write` で書き出したファイルは**無い**。
- パスの復元（読み）: NSIS の EXTRACTFILE は葉の名前しか持たないので、直前の `SetOutPath`（EW_CREATEDIR の更新あり）を**命令の並びの順に**追って親を付けた。変数は NSIS の組み込み番号で読み（`$v21` = `$INSTDIR`、`$v25` = `$TEMP`、`$v26` = `$PLUGINSDIR`、`$v31` = 直前の基準の出力先）、`$v0`、`$v38`、`$v43` は名前を確認していない。**条件分岐（コンポーネントの選択）は評価していない**ので、パスは「命令の並び上の位置」であって、実際に全部が入るとは限らない。
- 「作り」の判定（読み）: `Embarcadero Delphi for Win64 compiler version 36.0` の文字列 → Delphi。Rich ヘッダ → MSVC。`GCC: (` → GCC/MinGW。Rich ヘッダが無く MSVC 系のリンカー版数 → 「MSVC（推定）」。
- 「関数数」= `.pdata`（例外ディレクトリ）の個数（`pdata`）。連鎖した UNWIND_INFO（フラグ 4）を除いた個数を「独立」、独立な関数の範囲の長さの合計を「被覆バイト」とした。これは `fe-pe pe-pdata-info` の「entries／function_starts」と同じ定義（既存 6 モジュールで ILTools 11,747／11,747、dsp 50,529／30,816 と一致。`bundled-dll-inventory.md` §7.1、`dll-nonpdata-reloc-seeds.md` §2 の引用と照合した）。**`.pdata` に載らない関数（thunk、CRT 初期化表の関数など）は数えていない**。
- DLL も EXE も**実行していない**。`fe-lift` も新しい DLL には実行していない（難しさは読み。§5.6）。

## 2. インストーラの全ファイルの分類（項目 1）

### 2.1 種別ごとの合計

`installer-files.tsv` に、全 27,574 エントリを（区分、種別、ディレクトリ 3 階層）で集計した 199 行を置いた。**エントリごとの 27,574 行**は約 4.7 MB になるので `m2-inventory/` に置けない（上限 200 KB）。PE（コード）は 1 つずつ `pe-inventory.tsv` にある。エントリごとの表（27,574 行・約 4.7 MB）は RAM 上の `/tmp/opencode/m2inv/entries-full.tsv` にだけ置いた（揮発）。作り方は §9。

| 区分 | 種別 | 内容 | エントリ | 一意 | 合計 MB | 一意 MB | 圧縮後 MB |
|---|---|---|---:|---:|---:|---:|---:|
| コード | `plugin-effect` | ネイティブ・エフェクト(CreatePlugInstance を export) | 94 | 94 | 394.6 | 394.6 | 100.3 |
| コード | `plugin-generator` | ネイティブ・ジェネレータ(CreatePlugInstance を export) | 66 | 65 | 263.5 | 259.9 | 69.9 |
| コード | `third-party-installer` | 第三者のインストーラ(Edge Update 系 201 MB、FL Cloud Plugins Installer 31 MB) | 2 | 2 | 232.9 | 232.9 | 232.6 |
| コード | `ffmpeg` | ffmpeg 系(ILVideo + ffmpeg 7 DLL。Shared\ffmpeg\x64) | 8 | 8 | 76.3 | 76.3 | 24.0 |
| コード | `shared-real5` | 実在モジュール化済みの同梱 DLL 5 個(Shared) | 5 | 5 | 48.3 | 48.3 | 8.6 |
| コード | `plugin-support-effect` | エフェクトの補助 DLL | 18 | 18 | 45.0 | 45.0 | 7.0 |
| コード | `engine-copy-in-temp` | FLEngine のコピー(TEMP に展開) | 2 | 1 | 44.5 | 22.2 | 5.7 |
| コード | `tool-exe-x64` | 道具の EXE(x64) | 9 | 9 | 38.2 | 38.2 | 9.1 |
| コード | `shared-x86-counterpart` | 32 ビット版の同梱 DLL(dsp_ippv2、QuickFontCache、freetype) | 3 | 3 | 29.9 | 29.9 | 5.0 |
| コード | `shared-other-x64` | その他の同梱 DLL(Shared。x64) | 16 | 16 | 27.4 | 27.4 | 7.3 |
| コード | `engine` | エンジン本体(FLEngine_x64.dll) | 1 | 1 | 22.2 | 22.2 | 5.7 |
| コード | `tool-lilypond-x86` | LilyPond 一式(x86。System\Tools) | 40 | 34 | 19.9 | 19.6 | 5.3 |
| コード | `python-runtime` | Python 実行環境の PE(Shared\Python) | 31 | 31 | 17.9 | 17.9 | 5.6 |
| コード | `nsis-helper-dll` | NSIS 補助 DLL(System.dll など。インストーラ用) | 139 | 6 | 17.2 | 1.3 | 0.4 |
| コード | `plugin-vst-wrapper` | FL Studio VSTi(FL を VST として使う側) | 4 | 2 | 14.4 | 7.2 | 1.8 |
| コード | `root-x86` | インストール先直下の x86 DLL(AudioRestore、iZAudioRestore、FLMManaged) | 3 | 3 | 12.7 | 12.7 | 2.5 |
| コード | `plugin-support-generator` | ジェネレータの補助 DLL | 4 | 4 | 6.7 | 6.7 | 1.7 |
| コード | `plugin-rewire` | ReWire(FLReWire_x64 + ReWire.dll) | 2 | 2 | 5.8 | 5.8 | 1.4 |
| コード | `tool-exe-x86` | 道具の EXE(x86) | 3 | 3 | 5.8 | 5.8 | 1.7 |
| コード | `uninstaller` | アンインストーラ | 2 | 2 | 5.1 | 5.1 | 1.0 |
| コード | `root-x64-dll` | インストール先直下の x64 DLL(speaker、WebView2Loader) | 2 | 2 | 3.4 | 3.4 | 0.9 |
| コード | `host-exe` | ホスト EXE(FL64.exe 系) | 2 | 2 | 1.2 | 1.2 | 0.5 |
| コード | `asio-driver-component` | FL Studio ASIO ドライバー(ILWASAPI2ASIO) | 2 | 2 | 0.9 | 0.9 | 0.3 |
| データ | `audio` | 音声(サンプル・ループ・曲) | 6,595 | 6,485 | 454.8 | 448.4 | 412.4 |
| データ | `fl-preset-state` | FL の状態・プリセット類(.fst .fsc .nfo ほか) | 15,657 | 14,068 | 337.2 | 332.1 | 95.7 |
| データ | `image` | 画像 | 1,162 | 819 | 132.3 | 117.5 | 56.7 |
| データ | `project` | プロジェクト(.flp。デモ曲) | 165 | 161 | 87.7 | 87.2 | 20.9 |
| データ | `config` | 設定・言語・ハードウェア対応表 | 723 | 676 | 19.4 | 19.2 | 9.0 |
| データ | `zge-asset` | ZGameEditor の素材 | 374 | 367 | 18.0 | 15.4 | 6.6 |
| データ | `font` | フォント | 84 | 63 | 11.9 | 8.6 | 3.0 |
| データ | `script` | スクリプト(.py .pyc ほか) | 42 | 42 | 10.6 | 10.6 | 2.0 |
| データ | `tool-lilypond-data` | LilyPond のデータ | 295 | 295 | 5.4 | 5.4 | 1.5 |
| データ | `python-runtime-data` | Python の zip・設定(python312.zip ほか) | 6 | 6 | 4.5 | 4.5 | 4.0 |
| データ | `doc` | ヘルプ・文書 | 214 | 180 | 4.5 | 4.4 | 1.6 |
| データ | `data-other` | その他のデータ | 71 | 71 | 1.9 | 1.9 | 0.6 |
| データ | `midi` | MIDI | 1,718 | 1,704 | 0.6 | 0.6 | 0.3 |
| データ | `license` | ライセンス文書 | 2 | 2 | 0.3 | 0.3 | 0.1 |
| installer | `nsis-helper-file` | NSIS 補助ファイル | 8 | 8 | 0.2 | 0.2 | 0.0 |
|  |  | **合計** | 27,574 | 25,164 | 2422.9 | 2313.4 | 1105.4 |

- 「圧縮後」はインストーラ内の LZMA のストリーム長（一意データの合計）。音声（wav の中身の ogg/flac、wv、flac、ogg）はほぼ縮まない。
- 区分の合計: コード 458 エントリ・1333.7 MB（一意 1284.5 MB）、データ 27,108 エントリ・1089.0 MB（一意 1056.1 MB）、installer 8 エントリ。

### 2.2 ブラウザ配布の観点: プラグインとデータで何 MB か（一意のデータ。読み）

| 区分 | 一意 MB | 圧縮後 MB |
|---|---:|---:|
| コア: ホスト EXE + FLEngine + 5 実在 DLL | 71.8 | 14.8 |
| 同梱 DLL（Shared の x64。(b)(c) 候補）+ 直下の speaker/WebView2Loader | 30.8 | 8.1 |
| Python 実行環境の PE + ffmpeg 系 | 94.2 | 29.6 |
| ネイティブ・プラグインの PE（ジェネレータ + エフェクト） | 654.4 | 170.2 |
| プラグインの補助 DLL + VSTi + ReWire | 64.7 | 12.0 |
| 道具の x64 EXE + ASIO ドライバー | 39.1 | 9.4 |
| x86 の PE（道具、32 ビット版 DLL、LilyPond） | 68.1 | 14.6 |
| 第三者インストーラ + NSIS 補助 + アンインストーラ + TEMP のコピー | 261.4 | 239.7 |
| データ: 音声 | 448.4 | 412.4 |
| データ: FL の状態・プリセット類 + プロジェクト + MIDI | 420.0 | 116.9 |
| データ: 画像 + フォント + ZGameEditor 素材 | 141.5 | 66.2 |
| データ: スクリプト + Python + 文書 + 設定 + ライセンス + その他 | 46.2 | 18.7 |

- `Plugins\Fruity` 配下は PE 709.8 MB（エントリの合計。重複込み）とデータ 200.1 MB（サンプル・プリセット・ZGameEditor 素材など）。`Data` 配下はデータ約 814.8 MB（エントリの合計。ほとんどが `Data\Patches`）。
- **ブラウザに載せる場合の下限の見積もり（読み）**: プラグインの PE（654.4 MB 一意）とデータ（1056.1 MB 一意）は、wasm 化の前の元のバイト数で、wasm にすると PE は §6 の見積もりのとおり 11〜14 倍のコード（共有なしなら数 GB）になる。データは変換不要だが、`.wv`（WavPack。2,024 個・124.5 MB）の復号に wavpackdll が、`.wav` の名前で中身が ogg の 183 個・flac の 139 個（合計 50.2 MB）の復号に oggio（または別の復号）が要る（読み: 呼び出し元の確認はしていない）。

### 2.3 トップのディレクトリ別（エントリの合計。重複込み）

| ディレクトリ | PE 数 | PE MB | データ数 | データ MB | NSIS 補助 |
|---|---:|---:|---:|---:|---:|
| `Plugins\Fruity\{Generators,Effects}`（INSTDIR） | 182 | 709.8 | 6,232 | 200.1 | 0 |
| `Data\{Patches,Demo projects,Templates,System}` | 0 | 0.0 | 19,853 | 814.8 | 0 |
| INSTDIR 直下（FL64.exe、FLEngine など。変数名 `$0` で書かれる 201.7 MB の第三者 EXE 1 個を含む） | 10 | 245.4 | 3 | 3.6 | 0 |
| `Shared`（同梱 DLL、Python、ffmpeg、Artwork） | 65 | 206.2 | 58 | 9.7 | 0 |
| `System`（Plugin、Tools、Config、Languages、Hardware specific、Installers） | 55 | 101.6 | 809 | 33.6 | 0 |
| TEMP に展開（FLEngine のコピー） | 2 | 44.5 | 0 | 0.0 | 0 |
| `Artwork` | 0 | 0.0 | 80 | 20.6 | 0 |
| `$PLUGINSDIR`（NSIS 補助） | 139 | 17.2 | 0 | 0.0 | 8 |
| `$v43`（VST の入れ先？ FL Studio VSTi ほか。未確認） | 2 | 7.2 | 1 | 0.2 | 0 |
| `$0`（変数。DirectWave のサンプル 11 個ほか） | 0 | 0.0 | 15 | 3.6 | 0 |
| `Help` | 0 | 0.0 | 55 | 2.9 | 0 |
| `$v38`（FL Studio ASIO ドライバーの構成要素） | 3 | 1.9 | 2 | 0.0 | 0 |

### 2.4 データについての注意（実測）

- `.wav` の拡張子で中身（先頭バイト）が RIFF でないものが 322 個ある（ogg 183、flac 139）。拡張子だけで音声の形式を決めると誤る。
- `.fst`（8,990 個）と `.fsc`（609 個）の先頭は FL の状態ファイルの印（`FLhd`）で、プロジェクトと同じ形式。`.ilv`（512 個）は zip の印。`.nfo`、`.tbio`、`.dmpatch`、`.flmpst` などは先頭に既知の印が無い（`other`）。中身は読んでいない。
- `Shared\Python\python312.zip`（3.8 MB）と `python312._pth`（80 B）が Python の標準ライブラリの入れ物。`.pyc` 73 個・`.py` 37 個はスクリプトの種別に入れた。

## 3. コードを含むファイル（PE）の一覧（項目 2）

全 PE 一意 313 個の一覧は **`pe-inventory.tsv`**（種別、パス、大きさ、SHA-256、機種、作り、`.pdata`、独立関数、被覆バイト、例外ハンドラ付き関数数、import DLL とその数、export の数と先頭 4 名、TLS、.NET、特殊セクション、版情報）。ここでは集計と主要なものだけを置く。

### 3.1 グループ別の集計

| 種別 | 一意 PE | エントリ | 一意 MB | 作り | pdata | 独立 | 被覆 MB | 機種 |
|---|---:|---:|---:|---|---:|---:|---:|---|
| `host-exe` | 2 | 2 | 1.2 | MSVC 2 | 2,232 | 2,144 | 0.68 | x64 |
| `engine` | 1 | 3 | 22.2 | Delphi 1 | 57,816 | 57,816 | 10.55 | x64 |
| `shared-real5` | 5 | 5 | 48.3 | MSVC 3/Delphi 2 | 73,545 | 52,000 | 23.45 | x64 |
| `shared-other-x64` | 16 | 16 | 27.4 | MSVC 15/Delphi 1 | 62,328 | 54,694 | 13.82 | x64 |
| `root-x64-dll` | 2 | 3 | 3.4 | Delphi 1/MSVC 1 | 10,860 | 10,858 | 1.41 | x64 |
| `python-runtime` | 31 | 31 | 17.9 | MSVC 31 | 38,430 | 24,444 | 5.37 | x64 |
| `ffmpeg` | 8 | 8 | 76.3 | GCC/MinGW 7/Delphi 1 | 88,257 | 88,257 | 45.23 | x64 |
| `plugin-generator` | 65 | 66 | 259.9 | Delphi 43/MSVC 22 | 737,998 | 703,602 | 124.21 | x64 |
| `plugin-effect` | 94 | 94 | 394.6 | Delphi 89/MSVC 5 | 1,215,239 | 1,208,033 | 178.94 | x64 |
| `plugin-support-generator` | 4 | 4 | 6.7 | MSVC 2/Delphi 1/.NET 1 | 18,238 | 18,217 | 2.90 | x64 |
| `plugin-support-effect` | 18 | 18 | 45.0 | MSVC 17/Delphi 1 | 41,493 | 29,804 | 10.91 | x64 |
| `plugin-vst-wrapper` | 2 | 4 | 7.2 | Delphi 2 | 23,218 | 23,218 | 3.08 | x64 |
| `plugin-rewire` | 2 | 2 | 5.8 | Delphi 1/MSVC 1 | 22,559 | 22,243 | 2.51 | x64 |
| `tool-exe-x64` | 8 | 8 | 38.0 | Delphi 6/MSVC 2 | 110,864 | 110,856 | 14.57 | x64 |
| `asio-driver-component` | 2 | 2 | 0.9 | MSVC 2 | 1,665 | 1,599 | 0.34 | 混在 |
| `tool-exe-x86` | 3 | 3 | 5.8 | MSVC 2/Delphi 1 | 0 | 0 | 0.00 | x86 |
| `tool-lilypond-x86` | 34 | 40 | 19.6 | GCC/MinGW 34 | 0 | 0 | 0.00 | x86 |
| `shared-x86-counterpart` | 3 | 3 | 29.9 | MSVC 2/Delphi 1 | 0 | 0 | 0.00 | x86 |
| `root-x86` | 3 | 3 | 12.7 | MSVC 2/.NET 1 | 0 | 0 | 0.00 | x86 |
| `third-party-installer` | 2 | 2 | 232.9 | MSVC 1/Delphi 1 | 0 | 0 | 0.00 | x86 |
| `uninstaller` | 2 | 2 | 5.1 | MSVC 1/Delphi 1 | 0 | 0 | 0.00 | x86 |
| `nsis-helper-dll` | 6 | 139 | 1.3 | MSVC 5/Delphi 1 | 0 | 0 | 0.00 | x86 |

- **x86 の PE は `.pdata` を数えていない**（x86 は別形式の例外情報。表の pdata は 0）。x86 の PE は製品の x64 の FL が読むものではない（FLEngine の LoadLibrary で読まれる名前は `_x64` 付きか Windows の DLL。32 ビット版 `dsp_ippv2.dll`、`QuickFontCache.dll`、`freetype.dll` は x86。`ilbridge.exe` の 32 ビット版が 32 ビット VST のブリッジに当たると読めるが、呼び出しは確認していない）。
- 作りの内訳（実測のヘッダ・文字列）: **Delphi 36.0（Win64）** は FLEngine、ILTools、QFC、speaker、ILVideo、SG、プラグイン本体 132、FL Studio VSTi 2、FLReWire、道具の EXE 6（ControlCreator、PluginManager、fldiagnostic、ilbridge 64、CopyLicenseToAllUsers、RemoveLicenses）、プラグイン補助の `editor_x64.dll` 1 など。**MSVC**（Rich ヘッダ。リンカー 8.0〜14.42）は dsp_ippv2、Blend2D、freetype、ildsp、oggio、ILSvg、beat_tracker、python312 ほか。**GCC/MinGW** は ffmpeg 7 DLL（avcodec だけで `GCC: (` が約 2,800 個、`msvcrt.dll` を import）と LilyPond 一式。.NET は `FLMManaged.dll`（x86、9.3 MB、`mscoree.dll` を import）。

### 3.2 誰に読まれるか（ホスト EXE／FLEngine／プラグイン）

| 読む側 | 読まれるもの | 区分 |
|---|---|---|
| ホスト EXE `FL64.exe`（597,064 B、MSVC 14.29、1,116 関数、import は kernel32 ほか 6 DLL）| `FLEngine_x64.dll`（`CreateFruityInstance`）、`ILTools_x64.dll`（`ShowEngineChoice`）、`System\Plugin\ReWire\FLReWire_x64.dll`、`mscoree.dll`、`Mfplat.dll`。レジストリ `Software\Image-Line\Shared\Paths` | 実測（`FL64.exe` の ASCII/UTF-16 文字列。呼び出しの実行は未確認）|
| FLEngine（`LoadLibrary` 系。DllMain/InitUnits の中） | dsp_ippv2（unit 144）、Blend2D（149）、QuickFontCache（158）、ILTools（245）。QFC の DllMain から freetype | 実測（6 モジュール完走のトレース）|
| FLEngine（同。呼び出し点はあるが完走では実行されず） | ilsvg、ildsp、oggio、WavpackDLL、LAMEEnc、Python312（`Python\` 前置）、`ffmpeg\x64\ilvideo_x64.dll`、Speaker、WebView2Loader、beat_tracker。ほか ILRemoteServer、DelZip64（`DZ_Exec`）の文字列 | 呼び出し点は引用（`bundled-dll-inventory.md` §6.2、`loadlibrary-names-classification.md`）。完走で実行されないのは実測 |
| FLEngine（名前が静的に決まらない `LoadLibrary`） | ネイティブ・プラグイン DLL（`f_d8cab0`、`f_132c680` が `CreatePlugInstance` を引く）、CLAP（`f_b6e950`、`f_b76250` が `clap_entry`）、ASIO ドライバー（レジストリ `software\asio\` の値）、Windows の `Keyboard Layouts` の値 ほか | 実測（xref。§4.2）。実行は未確認 |
| ILTools / QFC / プラグイン（各 DLL の中の共有ユニット） | dsp_ippv2、Blend2D、QuickFontCache、ILSvg、ildsp、ILTools（UTF-16 の文字列が Sytrus、Fruity Wrapper、Tuner、Fruity Dance にある）。QFC は freetype、elastique（`LoadSharedElastiqueDll`）、Python（`InitSharedPython`）の読み込み口を持つ | 文字列は実測、読み込み口の用途は読み |
| python312.dll | `Shared\Python` の `.pyd` 21 個ほか（`python312.dll` を import）、`libcrypto-3`/`libssl-3`/`sqlite3`/`libffi-8`/`vcruntime140` | 実測（import）。動的な読み込みは読み |
| ILVideo_x64.dll | `avcodec-58`、`avformat-58`、`avutil-56`、`swscale-5`、`swresample-3`、`avdevice-58`（avfilter-7 は avdevice が import）| 実測（import 表）|
| Windows | kernel32、user32、gdi32 ほか。**全 PE の import のうち同梱 PE の名前に当たるのは Python／ffmpeg／LilyPond の内部と ILVideo→ffmpeg だけ**で、FLEngine やプラグインから他の同梱 DLL への静的な import は無い（`FLMManagedBridge_x64.dll` の `VCRUNTIME140.dll` だけは名前が Shared\Python の同名ファイルと同じだが、別の場所） | 実測 |

### 3.3 実在モジュール化済み 6 個と(b)の 10 DLL: 個別

**(a) 実在モジュール化済み（FLEngine を含む）**

| 名前 | 大きさ B | SHA-256（先頭 16 桁） | 作り | pdata | 独立 | 被覆 KB | EH/UH 関数 | TLS | import DLL/名 | export | 特殊セクション |
|---|---:|---|---|---:|---:|---:|---|---|---|---:|---|
| `dsp_ippv2_x64.dll` | 39,942,656 | `497dd4f77e859df9…` | MSVC 14.29 | 50,529 | 30,816 | 19,205 | 2371/2385 | - | 1/72 | 446 | IPPCODE,IPPDATA |
| `FLEngine_x64.dll` | 22,227,016 | `b2c64da40921cf26…` | Delphi | 57,816 | 57,816 | 10,553 | 2816/10837 | - | 24/738 | 3 | - |
| `ILTools_x64.dll` | 3,662,336 | `20409dcbc5eb5055…` | Delphi | 11,747 | 11,747 | 1,569 | 573/1798 | - | 14/520 | 7 | - |
| `QuickFontCache_x64.dll` | 2,067,968 | `afc5a0ed70eb7e4a…` | Delphi | 6,347 | 6,347 | 837 | 361/1042 | - | 8/217 | 10 | - |
| `Blend2D.dll` | 2,065,920 | `66634d24412084b7…` | MSVC 14.29 | 2,493 | 1,843 | 1,619 | 328/386 | あり | 1/94 | 1 | - |
| `freetype_x64.dll` | 607,032 | `6ea6c106ad2695b5…` | MSVC 8.0 | 2,429 | 1,247 | 221 | 26/47 | - | 1/74 | 271 | - |

**(b) の候補 10 DLL**（依頼文の名前。インストーラ内の綴りに直した）

| 名前 | 大きさ B | SHA-256（先頭 16 桁） | 作り | pdata | 独立 | 被覆 KB | EH/UH 関数 | TLS | import DLL/名 | export | 特殊セクション |
|---|---:|---|---|---:|---:|---:|---|---|---|---:|---|
| `python312.dll` | 7,003,928 | `0cc4e557972488eb…` | MSVC 14.37 | 12,839 | 7,906 | 1,978 | 369/369 | あり | 19/361 | 1696 | PyRuntim |
| `ildsp_x64.dll` | 4,197,376 | `f4b23b0f93ee9119…` | MSVC 14.29 | 2,622 | 1,871 | 1,730 | 92/146 | - | 1/73 | 10 | IPPCODE,IPPDATA |
| `speaker_x64.dll` | 3,263,488 | `fe745134c283ffdd…` | Delphi | 10,458 | 10,458 | 1,347 | 528/1654 | - | 12/483 | 7 | - |
| `oggio_x64.dll` | 1,872,384 | `b26c544f5d398733…` | MSVC 14.29 | 2,575 | 2,137 | 880 | 229/380 | - | 1/95 | 9 | IPPCODE,IPPDATA |
| `ILSvg_x64.dll` | 1,504,256 | `92f76fefa88a8a70…` | MSVC 14.11 | 4,837 | 2,914 | 683 | 844/967 | あり | 1/83 | 3 | - |
| `ILVideo_x64.dll` | 1,329,152 | `82ab852157998e88…` | Delphi | 4,556 | 4,556 | 515 | 247/774 | - | 10/195 | 26 | - |
| `beat_tracker.dll` | 983,552 | `43942be8711e977d…` | MSVC 14.29 | 1,959 | 1,504 | 585 | 237/457 | あり | 1/93 | 14 | - |
| `LAMEenc_x64.dll` | 465,720 | `0703adcd734071c7…` | MSVC 14.0 | 1,019 | 720 | 192 | 84/113 | - | 3/71 | 9 | - |
| `wavpackdll_x64.dll` | 271,672 | `3bd37cc93bb9e7bd…` | MSVC 8.0 | 706 | 397 | 98 | 35/62 | - | 1/83 | 48 | - |
| `WebView2Loader.dll` | 137,120 | `f4839ab5bd42f5d7…` | MSVC 14.0 | 402 | 400 | 64 | 32/47 | あり | 1/80 | 4 | .tls |

- 「EH/UH 関数」は UNWIND_INFO のフラグ（EHANDLER／UHANDLER）が立つ関数の数（重なりうる）。runtime は構造化例外を持たず `RaiseException` を名前付きで止める（`loadlibrary-names-classification.md` §2.2）。ILSvg は pdata 4,837 に対し EH/UH が 844/967 で、C++ 例外を使う関数の比率が高い。
- 全 SHA-256 は `pe-inventory.tsv`。5 個の実在 DLL の SHA-256 は `bundled-dll-inventory.md` §2、§4 の値と**一致**（FLEngine `b2c64da4…`、dsp `497dd4f7…`、Blend2D `66634d24…`、QFC `afc5a0ed…`、ILTools `20409dcb…`。freetype `6ea6c106…` は決定 0102 の値と一致）。

### 3.4 その他の同梱 DLL・道具（個別の主なもの）

| 名前 | 大きさ B | 作り | pdata | 独立 | 被覆 KB | import DLL | export 数・先頭 | 版情報（会社） |
|---|---:|---|---:|---:|---:|---:|---|---|
| `PluginManager.exe` | 7,158,856 | Delphi | 20,927 | 20,927 | 2,674 | 16 | 2: __dbk_fcall_wrapper;dbkFCallWrapperAddr | Image-Line |
| `ControlCreator.exe` | 6,556,232 | Delphi | 18,467 | 18,467 | 2,468 | 17 | 2: __dbk_fcall_wrapper;dbkFCallWrapperAddr | - |
| `fldiagnostic.exe` | 6,473,800 | Delphi | 20,476 | 20,476 | 2,580 | 17 | 2: __dbk_fcall_wrapper;dbkFCallWrapperAddr | Image Line NV |
| `ilbridge.exe` | 6,200,392 | Delphi | 19,391 | 19,391 | 2,637 | 16 | 2: __dbk_fcall_wrapper;dbkFCallWrapperAddr | Image-Line |
| `SG_x64.dll` | 5,892,096 | Delphi | 17,522 | 17,522 | 2,771 | 15 | 3: CreatePlugInstance;__dbk_fcall_wrapper;dbkFCallW | Image-Line |
| `elastique_x64.dll` | 4,997,632 | MSVC | 5,475 | 3,583 | 2,466 | 2 | 15: Auftakt_CreateInstance;Auftakt_CreateInstance2;D | Image-Line |
| `DirectWaveCtEngine_x64.dll` | 4,788,736 | MSVC | 19,695 | 18,679 | 3,144 | 9 | 2: CT_Process;CT_ProgramNames | Chicken Systems, Inc. |
| `CopyLicenseToAllUsers.exe` | 3,314,760 | Delphi | 10,293 | 10,293 | 1,285 | 12 | 2: __dbk_fcall_wrapper;dbkFCallWrapperAddr | - |
| `ILPluginScanner64.exe` | 3,207,480 | MSVC | 7,206 | 7,202 | 1,115 | 11 | 0:  | - |
| `ILMinihostBridge64.exe` | 3,153,720 | MSVC | 6,932 | 6,928 | 1,084 | 10 | 0:  | - |
| `RemoveLicenses.exe` | 1,984,584 | Delphi | 7,172 | 7,172 | 726 | 4 | 2: __dbk_fcall_wrapper;dbkFCallWrapperAddr | Image-Line |
| `FL64 (scaled).exe` | 597,064 | MSVC | 1,116 | 1,072 | 342 | 6 | 0:  | Image-Line |
| `FL64.exe` | 597,064 | MSVC | 1,116 | 1,072 | 342 | 6 | 0:  | Image-Line |
| `ILWASAPI2ASIO_x64.dll` | 531,456 | MSVC | 1,665 | 1,599 | 340 | 7 | 5: DllCanUnloadNow;DllGetClassObject;DllMain;DllReg | - |
| `Reverb_x64.dll` | 492,544 | MSVC | 1,515 | 1,484 | 319 | 1 | 3: CreateReverbInstance;Process_Fruity_32FM_32FS;Pr | - |
| `sveng64.dll` | 399,160 | MSVC | 162 | 162 | 82 | 1 | 1: LoadLanguage | - |
| `EasyFLAC_x64.dll` | 394,240 | MSVC | 1,008 | 772 | 206 | 1 | 10: EasyFLAC_Create;EasyFLAC_CreateForFile;EasyFLAC_ | - |
| `DelZip64.dll` | 362,808 | MSVC | 784 | 650 | 236 | 3 | 7: DZ_Abort;DZ_Banner;DZ_Exec;DZ_Name | DelphiZip |
| `REX Shared Library_x64.dll` | 321,848 | MSVC | 1,466 | 1,395 | 174 | 2 | 14: Close;Open;REXCreate;REXDelete | Propellerhead Software A |
| `ILRemoteServer_x64.dll` | 215,040 | MSVC | 658 | 579 | 125 | 4 | 1: ILR_createServerManager | - |
| `svctl64.dll` | 211,768 | MSVC | 325 | 325 | 118 | 4 | 39: SVAbort;SVAudioCallback;SVCloseSpeech;SVCommand | - |

### 3.5 Windows 側の API の表面（読み）

- 6 実在モジュールの import の和集合（(DLL, 名前) の組）は 841。Delphi 製プラグイン 132 個のうち **115 個は、この 841 に無い import が 0**（最大でも 42。gdiplus 24、wsock32 8 などが主）。MSVC 製のプラグイン・補助 47 個は 742 組のうち 359 組が 841 に無い（opengl32 62、ws2_32 49、kernel32 104 など）。(b) の 10 DLL のうち 841 に無い import は python312 が 242、ILVideo が 75、oggio が 8、speaker が 7、beat_tracker が 6、ILSvg が 5、他は 3 以下、ildsp は 0。
- 以上は**import 表の名前の集合**であって、応答表（`imports_table.rs`）の行があるか、実行で呼ばれるかは別（未測定）。

## 4. FL Studio のプラグインの仕組み（項目 3。読み）

### 4.1 ネイティブ・ジェネレータ／エフェクトは別の DLL

- **1 プラグイン = 1 つ（以上）の DLL**。`Plugins\Fruity\Generators\<名前>\<名前>_x64.dll` と `Plugins\Fruity\Effects\<名前>\<名前>_x64.dll` に置かれ（実測の復元パス。例: `Sytrus\Sytrus_x64.dll`）、export は `CreatePlugInstance`（ほとんど唯一）。FLEngine の中には入っていない（FLEngine の export は `CreateFruityInstance` と `__dbk_fcall_wrapper`、`dbkFCallWrapperAddr` の 3 つ。実測）。
- **2 系統**（実測のヘッダ）: (i) **Delphi 製の本体** 132 個。FLEngine/ILTools と同じコンパイラ（Delphi 36.0）、同じセクション配置（`.text .data .bss .idata .didata .edata .rdata .reloc .pdata .rsrc`）、ImageBase 0x400000、`__dbk_fcall_wrapper`/`dbkFCallWrapperAddr` 付き。各 3.6〜7.6 MB、pdata 11,513〜23,540。(ii) **MSVC 製（リンカー 14.29）** 27 個。ImageBase 0x180000000、Rich ヘッダあり、`RT_CODE` セクションを持つ 18 個（多くは `FileDescription` が `… Rev2658 (x64)` の Hardcore、Pitcher、BassDrum、Drumaxx、Sakura ほか。同じ SDK で作られたと読める）、IPP 系 1 個、`FL Studio Mobile` 系 3 個。
- **補助 DLL**: `engine_x64.dll` 系（Tuner、Vintage Chorus など。補助 DLL 22 個のうち **`VSTPluginMain` を export するものが 7 個**＝VST2 の形。Delphi 側の本体が名前 `engine_x64` を UTF-16 で持つ — Tuner_x64.dll で実測）、`Fruity Delay 3 engine`（`VSTPluginMain`、`MAIN`、`?CreateEngine@@…`）、Frequency Splitter/Multiband Delay の engine（IPPCODE）、ZGameEditor Visualizer の NDI（27.1 MB）/Spout/Bullet/Nano/Skelet ほか。
- **FLEngine は DLL の中身ではなくホスト側 API を提供する**（読み）: プラグインの import は Windows の DLL だけで（実測）、FLEngine や他の同梱 DLL への import は無い。ホスト（FL）との受け渡しは `CreatePlugInstance` が返すオブジェクトの仮想関数とホスト側のコールバックで行われると読めるが、**インターフェースの中身は読んでいない**（未確認）。
- **プラグインにも共有ユニットが入っている**（実測）: Delphi 製プラグイン（Sytrus、Fruity Wrapper、Tuner、Fruity Dance で確認）の UTF-16 文字列に `QuickFontCache`、`dsp_ippv2`、`ILTools`、`Blend2D`（と一部 `ILSvg`、`ildsp`）がある。つまり**各プラグインの DllMain/InitUnits も FLEngine と同形の `LoadLibrary` による共有 DLL の取り込みを持つ**（読み）。同じプロセスでは同じハンドルが返る。

### 4.2 FLEngine にある外部プラグインの入口（実測の xref。呼び出しの実行は未確認）

`FLEngine_x64.dll`（`binaries/win64/`、SHA-256 `b2c64da4…496d`）の `.text` を 7 バイトの RIP 相対 `lea` で走査して文字列の参照元を探した。VA は ImageBase 0x400000 のもの。

| 入口 | 手がかり（FLEngine 内） | 読み |
|---|---|---|
| ネイティブ（Fruity）プラグイン | UTF-16 `CreatePlugInstance` が 2 つ（VA 0xd8ceee、0x132c7c2）。参照元 `f_d8cab0`（0xd8cb86）と `f_132c680`（0x132c711）。前者は `_x64.dll` の文字列 2 つと一緒、後者は `_X64` と `FreeLibrary` と一緒（`lift/out-loadlib/dyn-callers-recheck.txt`、`sites-flengine.tsv` の引用と一致）| 名前 + `_x64.dll` を `LoadLibraryW` し `CreatePlugInstance` を `GetProcAddress`。2 つの用途の違い（後者は `FreeLibrary` を呼ぶ）は未確認 |
| VST2 | UTF-16 の名前表（VA 0xac3278〜0xac34c0 付近、`.text` 内）: `VstPluginMain`、`VSTPluginMain`、`main` の大文字小文字 16 通り（`maiN` … `MAIN`）、`vstpluginmain`、`VSTPluginmain`、`VSTpluginmain` …。`.data` から 2 か所のポインタが `VSTPluginMain`（VA 0xac32a0）を指す（ファイルオフセット 0x10e1f48、0x10e1fd8）。`lea` の直接参照は無い。`audioMaster` の UTF-16 が 1 つ、`AEffect` の ASCII が 5 つ | 外部 DLL の候補入口名を順に `GetProcAddress` する表と読める（**呼び出しは未確認**）。VST2 のホストは Fruity Wrapper（`VSTPluginMain`、`VstPluginMain`、`audioMaster` が UTF-16 で、`AEffect` が ASCII で 5 つ）と Sytrus にも同じ文字列がある |
| VST3 | FLEngine の UTF-16 `.vst3`（2 か所。参照元 `f_b60d70`、`f_12b9e70`）、`TVST3Plugin.Create`/`.Destroy`、`CreateVST3Instance`（UTF-16、VA 0xb4766c。直前が Delphi の遅延バインド風のスタブ）、`il.VST3Extensions`、`VST3_IPSLHostCommands` | **VST3 のホスト実装は QuickFontCache_x64.dll にある**: QFC の export に `CreateVST3Instance`（決定 0086 の「FLEngine が QFC から引く 8 名」と QFC の export 8 名が一致するので、その 1 つと読む）、QFC の UTF-16 に `GetPluginFactory`、`InitDll`、`ExitDll` が 1 つずつ。つまり外部 VST3 を読む `LoadLibrary`/`GetPluginFactory` は **既に lift 済みの QFC の中**（読み。実行は未確認）|
| CLAP | UTF-16 `clap_entry`（VA 0xb6ecb8、0xb768d4）。参照元 `f_b6e950`（0xb6e9fb）、`f_b76250`（0xb7646f）| 名前が動的な `LoadLibraryW`（`f_44db40` 経由）+ `GetProcAddress("clap_entry")`（`sites-flengine.tsv` の gpa_wrapper 2 点。引用）|
| 32 ビット／別プロセスのブリッジ | UTF-16 `ilbridge` が 2 つ（参照元の `lea` は無い）。インストーラに `System\Tools\Bridge\{32bit,64bit}\ilbridge.exe`（x86 4.1 MB、x64 6.2 MB）、`Shared\ILMinihostBridge64.exe`、`Shared\ILPluginScanner64.exe`（JUCE 製。どちらにも `JUCE` と `VSTPluginMain` の文字列）| 外部プラグインの走査・ブリッジは別プロセス（読み）。ブラウザでは別プロセスが無い |
| ASIO | `software\asio\` と `clsid`（レジストリから DLL 名を取る。`f_c522f0`。`loadlibrary-names-classification.md` §9 の引用）| 音声ドライバーはレジストリ経由のユーザー環境依存 |
| ReWire／FL Studio VSTi | `System\Plugin\ReWire\FLReWire_x64.dll`、`System\Plugin\VSTi\x64\FL Studio VSTi.dll`（`main` を export する VST2 の形。**FL を他の DAW のプラグインにする側**で、FL が外部プラグインを読む入口ではない）| ホストではなくゲスト側の DLL |

同梱のプラグインに**外部 VST の本体は含まれない**（実測の範囲: iZotope の `AudioRestore.dll`（x86、VST2。`VSTPluginMain` と `main` を export）と `iZAudioRestore.dll`（x86）が INSTDIR 直下にある。これは 32 ビットなのでブリッジ経由と読める）。外部 VST／VST3／CLAP はユーザーの環境にある DLL で、インストーラの外。

### 4.3 Delphi 製プラグインが既存モジュールと共有する割合（実測。方法つき）

**方法**: 各 Delphi 製 DLL の `.pdata` の全関数（連鎖を除く）を逆アセンブル（capstone）し、**RIP 相対のメモリ変位の 4 バイト、`call/jmp` の rel32、8 バイトの即値だけを 0 にして**関数のバイト列の SHA-1（先頭 16 桁）を作る。同じ値を持つ関数を「同一」と数える。これは**同じ関数でも別の DLL ではデータやほかの関数のアドレスが違う**ことを無視した比較で、**同一の関数は同じ命令列を持つ**（参照先の違いは無視）ことを意味する。これ以外の違い（ジャンプ表の中身、定数の違いなど）があれば同一にならないので、**同一の割合は下限**（読み）。最初の試行で capstone の `disp_offset` が命令の先頭からの相対であることを見落としてマスクの位置を誤った（関数の先頭からの位置に直して再実行した。誤った版の値は使っていない）。

**対（a が b に含まれる割合）— 関数数の %／コード（被覆バイト）の %** の一部:

| a ＼ b（a の何 % が b にもあるか） | ILTools | QFC | FLEngine | Fruity Chorus | Sytrus |
|---|---:|---:|---:|---:|---:|
| Fruity Chorus（11,674 関数） | 96 / 87 | 57 / 37 | 95 / 82 | - | 99 / 95 |
| Wasp（11,549 関数） | 97 / 93 | 58 / 40 | 96 / 88 | 100 / 100 | 99 / 96 |
| Fruity Balance（11,736 関数） | 96 / 90 | 57 / 39 | 95 / 85 | 99 / 96 | 100 / 99 |
| Sytrus（18,013 関数） | 75 / 53 | 47 / 23 | 87 / 63 | 75 / 55 | - |
| Harmor（18,614 関数） | 73 / 51 | 47 / 22 | 86 / 61 | 74 / 52 | 95 / 88 |
| FLEX（16,125 関数） | 81 / 65 | 50 / 28 | 90 / 75 | 81 / 66 | 90 / 83 |
| SG_x64（17,522 関数） | 76 / 57 | 48 / 25 | 88 / 66 | 76 / 59 | 97 / 90 |
| ILTools（11,747 関数） | - | 58 / 41 | 95 / 88 | 96 / 91 | 98 / 97 |
| QFC（6,347 関数） | 85 / 70 | - | 86 / 71 | 85 / 68 | 85 / 71 |

**プラグイン全体（Delphi 製 149 個＝プラグイン 132 + 他の Delphi の DLL。`plugin-sharing.tsv`）**

| 群 | 個数 | 関数の延べ | コード延べ MB | FLEngine∪ILTools∪QFC に在るコードの割合（中央値） | 在らない関数（中央値） | 同（最大） | 在らないコード（中央値） | 同（最大） |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| ジェネレータ（Delphi） | 43 | 625,913 | 93.8 | 86% | 499 | 7,652 | 286 KB | 2,222 KB |
| エフェクト（Delphi） | 89 | 1,193,723 | 173.0 | 89% | 296 | 3,395 | 184 KB | 1,013 KB |

- Delphi 製プラグイン 132 個の**和集合**: 関数 45,685 個・コード 13.5 MB（延べ 1,819,636 関数・266.8 MB から、重複を除くと **約 5.0%**）。そのうち FLEngine∪ILTools∪QFC に無いのは **28,779 関数・10.4 MB**。FLEngine は 57,816 関数・10.6 MB なので、プラグイン 132 個の「新しい」コードの総量は FLEngine の関数数の約半分、コードのバイト数でほぼ同じ。
- 他の Delphi 製 DLL を含めた 149 個の和集合は 66,510 関数・21.0 MB、うち既存 3 モジュールに無い 31,627 関数・11.2 MB。
- MSVC 製のプラグイン 27 個: 延べ 91,999 関数・36.4 MB のうち、同じ手順の和集合は 23,899 関数・11.9 MB（33%）。補助 DLL を含めた 48 個では延べ 50.2 MB → 和集合 19.5 MB（39%）。`RT_CODE` を持つ 18 個（同じ SDK）で延べ 28.7 MB → 8.8 MB（31%）。
- **注意（読み）**: 同じ命令列の関数でも、別の DLL では**別のグローバル変数・別の関数を参照する**。lift した C の関数を DLL 間で 1 つに共有するには、参照するデータの基底を引数やモジュールごとの表で渡す必要があり、現行の生成器（モジュールごとに絶対アドレスで出力する）にはその仕組みが無い。この測定は「共有の可能性の上限の目安」で、実現性は未検討。

### 4.4 外部プラグインを wasm で動かす場合の論点（決めない。論点の整理）

1. **外部プラグインは何か**: VST2 は C の ABI（`AEffect`、dispatcher、`VSTPluginMain`）、VST3 は COM 風の vtable（`GetPluginFactory` → `IPluginFactory`）、CLAP は C の ABI（`clap_entry`）。いずれも **Windows 向けの x64 の PE（DLL）** で、Win32 API を import し、GUI は親ウィンドウ（HWND）の子ウィンドウを作る。**インストーラには入っていない**（§4.2）。
2. **機械語をどう wasm にするか**。選択肢の列挙のみ: (A) 対象を事前に決めてビルド時に lift（再配布の可否、§6 の大きさ、プラグインごとの emcc の時間とピークメモリが問題。実測: ILTools 一式の `emcc -c` は 12.4 秒（17 単位）、dsp_ippv2 の最大の単位はピーク 8.42 GiB。`wasm6-report.md` §4.1）、(B) ブラウザ上で x86-64 を実行時に変換またはエミュレーション（現行の方針＝静的 lift の外）、(C) ブラウザ側の別実装に置き換える（WebAudio Modules、CLAP の wasm ビルド、各社の wasm 版。ホストの入口 VST2/VST3/CLAP の API を JS/wasm のプラグインにつなぐアダプターが要る）、(D) 同梱のものだけ対象にして外部は対象外。
3. **VST3 のホストはすでに QFC の中**（lift 済み。§4.2）。外部 VST3 の vtable 呼び出し自体は lift 済みコードが行うので、問題は呼ばれる側（プラグイン DLL）をどう用意するかと、`LoadLibraryW`/`GetPluginFactory`/`InitDll` に対する runtime の応答（ユーザーの DLL のバイト列をどこから得るか）。
4. **GUI**: プラグインは HWND の子ウィンドウと GDI/Direct2D/OpenGL を使う。FL の窓システムを wasm 側でどこまで模倣しているか（実在モジュール構成では GDI の DC/DIB の塊が壁として出た。決定 0106、その後 0108、0111）に依存する。
5. **スレッドとリアルタイム**: `process()` は音声スレッドから呼ばれ、プラグインは自前のスレッドを作る。ブラウザでは AudioWorklet、SharedArrayBuffer、Atomics。現行の runtime のスレッド/同期の扱い（決定 0086 の未決項目に JIT・TLS・スレッドの扱いが挙がっている）に依存する。
6. **ブリッジ**: FL は 32 ビットや不安定なプラグインを別プロセスのブリッジ（`ilbridge.exe`、`ILMinihostBridge64.exe`。JUCE 製）で隔離する。ブラウザでは別 wasm インスタンス（Worker）に当たるが、メモリは別で、プラグインのウィンドウの親子関係を越える必要がある。
7. **同梱の Delphi プラグイン 132 個**: (i) 1 個ずつ別モジュール、(ii) 共通部（FLEngine/ILTools/QFC と同一の関数）を共有、(iii) 使われたときに遅延ロード、の選択がある。§4.3 の測定は (ii) の余地（既存 3 モジュールに無い関数は和集合で 28,779）を示すが、データの基底の扱いは未検討。
8. **ほかの形のもの**: `FLMManaged.dll`（.NET、x86 9.3 MB、`mscoree.dll` を import）と、FL Studio Mobile 系（MSVC 製の本体 12.1 MB とラックの 2 DLL、補助の `FLMManagedBridge_x64.dll` は `mscoree.dll` を import し `GDrive*` を export。**管理コードとの橋と読む**。中身は未確認）は、CLR の代わりが要るかもしれない。ZGameEditor Visualizer は OpenGL/DirectX 系（opengl32 62 名など）と NDI/Spout を使う。
9. **ライセンス**: 同梱 DLL には第三者製（Microsoft の WebView2Loader、Python、ffmpeg、LAME、WavPack、zplane の élastique、Intel IPP、Propellerhead の REX、SoftVoice、iZotope、Chicken Systems の DirectWave エンジン）がある。再配布や変換の条件は**未確認**。

## 5. M2 の対象の分類と見積もり（項目 4）

### 5.1 分類の定義と、依頼文の想定との差

- **(a)** 起動（InitUnits、DllMain(ATTACH)）に必須で、実在モジュール化済み。**FLEngine 本体 + 5 個**（dsp_ippv2、Blend2D、QFC、freetype、ILTools）。実測。
- **(b)** 起動に必須だが未実在化: **実測では 0 個**（§0 の 5）。依頼文の候補 10 名（ilsvg、ildsp、oggio、wavpackdll、lameenc、python312、ilvideo、speaker、webview2loader、beat_tracker）は、FLEngine の `LoadLibrary` の呼び出し点があり、静的な到達の読みでは ildsp と python312 が unit 895/898 に届くが、**完走したトレースには出ない**。理由は読み: 呼び出し点が機能（SVG 描画、DSP、ファイル形式、スクリプト、動画、読み上げ、ブラウザビュー、拍の解析）の最初の使用時にあるか、A4 の仮の応答で分岐が外れた。**確認していない**。表では「(b) 候補」として個別に出す。
- **(c)** 機能を使うときだけ必要: オーディオ I/O（ドライバー）、ファイル形式（wavpack、oggio、EasyFLAC、DelZip64、REX、LAME）、時間伸縮（elastique）、拍の解析、動画（ILVideo + ffmpeg）、SVG、Python スクリプト、読み上げ、ブラウザビュー、リモート（ILRemoteServer）、道具の EXE。
- **(d)** プラグイン: ネイティブ・プラグイン（Delphi 132、MSVC 27、補助 22）、FL Studio VSTi、ReWire、そして外部 VST/VST3/CLAP（インストーラの外）。
- **A4 の限界（引用と実測）**: エントリは `FLEngine_x64.dll` の DllMain で、A4 は「DllMain の InitUnits が 942/942 で戻る」こと（決定 0101、0113）。ホスト EXE `FL64.exe` がその後に `CreateFruityInstance` を呼ぶ（`FL64.exe` の文字列。実測）が、その先は**一度も実行していない**。(c)(d) の「いつ必要か」はすべてここから先の話で、未測定。

### 5.2 (a) 実在モジュール化済み 6 個（実測の wasm との比較）

| 名前 | 大きさ MB | pdata | 独立 | 被覆 MB | 11〜14 倍の見積もり MB |
|---|---:|---:|---:|---:|---:|
| `dsp_ippv2_x64.dll` | 39.94 | 50,529 | 30,816 | 19.21 | 211〜270 |
| `FLEngine_x64.dll` | 22.23 | 57,816 | 57,816 | 10.55 | 116〜149 |
| `ILTools_x64.dll` | 3.66 | 11,747 | 11,747 | 1.57 | 17.2〜22.1 |
| `QuickFontCache_x64.dll` | 2.07 | 6,347 | 6,347 | 0.84 | 9.2〜11.8 |
| `Blend2D.dll` | 2.07 | 2,493 | 1,843 | 1.62 | 17.8〜22.8 |
| `freetype_x64.dll` | 0.61 | 2,429 | 1,247 | 0.22 | 2.4〜3.1 |

- 6 個の合計: pdata 131,361、被覆 34.01 MB。見積もり 373〜479 MB に対し**実測は 389.4 MB**（`flengine6.wasm` の Code 節 387,190,488 B、関数 98,392。これには runtime/harness を含む）。見積もりの係数がこの実測から作った値なので、これは検算であって独立の検証ではない。
- 係数の出典（引用と計算）: FLEngine のみの wasm 115,903,130 B（`two-module-design.md` §の表）÷ 被覆 10,553,710 B = **10.98**。dsp_ippv2 を足した 2 モジュールの 329.6 MB（`wasm6-report.md` §3 の引用）から 115.9 を引いた 213.7 MB ÷ 被覆 19,205,731 B = **11.13**。6 モジュール 389,439,333 B から FLEngine と dsp の分を引いた 59.8 MB ÷ 残り 4 モジュールの被覆 4,248,456 B = **14.08**（dsp の分が 6 モジュールでも 2 モジュールの値と同じと仮定した逆算で、誤差が入る）。

### 5.3 (b) 候補 10 DLL: 個別の見積もりと難しさの見立て（読み）

| 名前 | MB | pdata | 独立 | 被覆 MB | wasm 見積もり MB（11〜14 倍） | 難しさ（読み） | 根拠（実測のヘッダ・読み） |
|---|---:|---:|---:|---:|---:|---|---|
| `python312.dll` | 7.00 | 12,839 | 7,906 | 1.98 | 21.7〜27.9 | 高 | CPython 3.12。import 361（6 モジュールに無いのは 242。DLL 19 個で、api-ms-win-crt-* 12、api-ms-win-core-path 1、VCRUNTIME140、ws2_32、bcrypt ほか）、TLS（`PyRuntim`）、1,696 export。`.pyd` 21 個と ssl/sqlite/ctypes を動的に読み、stdlib は zip と `python312._pth`。JIT は無い（3.12）が、`ctypes` の libffi は関数ポインタ呼び出しを動的に組み立てる。**lift より Pyodide などの別の Python の wasm に置き換える選択肢**もある（論点） |
| `ildsp_x64.dll` | 4.20 | 2,622 | 1,871 | 1.73 | 19.0〜24.4 | 中〜高 | Intel IPP を静的リンク（IPPCODE/IPPDATA）。dsp_ippv2 と同種の SIMD コード（SSE/AVX の未対応命令の問題は dsp で大量に出ている）。import は kernel32 のみで新しい API は 0。10 の `*_CreateInstance` |
| `speaker_x64.dll` | 3.26 | 10,458 | 10,458 | 1.35 | 14.8〜19.0 | 低〜中 | Delphi。ILTools と同形（10,458 関数）。InitUnits の形（unit 数）は未測定。FLEngine∪ILTools∪QFC に関数の 98%・コードの 93% が在る（`plugin-sharing.tsv` の speaker_x64.dll の行）。svctl64/sveng64 を使うと読める（`ZXSpeak*` の export） |
| `oggio_x64.dll` | 1.87 | 2,575 | 2,137 | 0.88 | 9.7〜12.4 | 中〜高 | IPPCODE/IPPDATA あり。Ogg/Vorbis/FLAC 読み書き。C++ の export が 1 つ。import は kernel32 のみ |
| `ILSvg_x64.dll` | 1.50 | 4,837 | 2,914 | 0.68 | 7.5〜9.6 | 中 | MSVC 14.11 の C++。EHANDLER 844・UHANDLER 967（pdata 4,837 に対して）と多い、TLS ディレクトリ（暗黙の TLS は Blend2D で先例あり）。import は kernel32 のみ（83、6 モジュールに無いのは 5）。`svgCreate/svgRender/svgDestroy` の 3 export |
| `ILVideo_x64.dll` | 1.33 | 4,556 | 4,556 | 0.52 | 5.7〜7.3 | 中（単体）／高（ffmpeg 込み） | Delphi（4,556 関数）。ffmpeg の DLL 6 本を import。ffmpeg 7 DLL は GCC/MinGW（`msvcrt.dll` 137 名、TLS コールバック 4、手書きアセンブラの SIMD。AVX2/AVX-512 を含みうる（未確認））で 75 MB・83,701 関数・被覆 44.7 MB |
| `beat_tracker.dll` | 0.98 | 1,959 | 1,504 | 0.59 | 6.4〜8.2 | 中 | MSVC 14.29 の C++、TLS ディレクトリ、条件変数/SRW ロック（`InitializeConditionVariable`、`SleepConditionVariableCS`。決定 0092 の先例）、スレッド。14 の `beatTracker*` export |
| `LAMEenc_x64.dll` | 0.47 | 1,019 | 720 | 0.19 | 2.1〜2.7 | 低〜中 | 466 KB、C。import は kernel32 + user32 1 + advapi32 1（`SystemFunction036`＝乱数）。x87/SSE の使い方は未確認 |
| `wavpackdll_x64.dll` | 0.27 | 706 | 397 | 0.10 | 1.1〜1.4 | 低 | 272 KB、C、48 の素直な export、import は kernel32 のみ。**同梱サンプル 2,024 個（124.5 MB）の復号に要る**（読み） |
| `WebView2Loader.dll` | 0.14 | 402 | 400 | 0.06 | 0.7〜0.9 | 低だが意味が無い | 137 KB、402 関数、TLS ディレクトリ。**Edge WebView2 ランタイムを探して読むだけ**で、ランタイム本体（Edge）は INSTDIR の外/第三者のインストーラ（201 MB）。ブラウザ内では iframe などへの置き換えの話になる |

- (b) 候補 10 DLL の合計: 一意 21.0 MB、pdata 41,973、被覆 8.08 MB。**すべて lift した場合の wasm の見積もりは 89〜114 MB**（python312 が約 21.7〜27.9 MB、ildsp が約 19.0〜24.4 MB）。6 モジュール wasm（389 MB）への加算になる。
- 比較の基準（引用）: Blend2D は未対応 596（VEX 命令の項目あり）、jump table 31、locals 5 万超 4 関数。QFC は 6,712 関数・SSSE3 の未対応・最大関数 13,559 行。ILTools は 12,526 関数・未対応 352（99% が `jmp [IAT]` の行なし）・bad_nodes 0。dsp_ippv2 は未対応 97 万・locals 5 万超 15 関数。**これらの数は新しい DLL には測っていない**ので、上の難しさは import・セクション・作りからの読み。

### 5.4 (c) 機能を使うときだけ必要なもの

| 群 | 一意 PE | MB | pdata | 独立 | 被覆 MB | 見積もり MB | 内容 | いつ（読み） |
|---|---:|---:|---:|---:|---:|---:|---|---|
| python312 以外の Python の PE 30 個 | 30 | 10.9 | 25,591 | 16,538 | 3.39 | 37〜48 | pyd 21、libcrypto-3（5.2 MB）、libssl-3、sqlite3、libffi-8、vcruntime140(_1)、python.exe/pythonw.exe ほか。`python312.dll` に依存 | Python を使うなら python312 とセット |
| ffmpeg 7 DLL（ILVideo は別） | 7 | 74.9 | 83,701 | 83,701 | 44.72 | 491〜630 | avcodec-58（47.1 MB、35,510 関数）、avformat-58（12.4 MB）、avfilter-7（11.5 MB）、avutil、swscale、swresample、avdevice。GCC/MinGW | 動画の読み込み・書き出し |
| その他の Shared の x64 DLL 10 個 | 10 | 18.1 | 48,610 | 45,151 | 9.65 | 106〜136 | elastique（時間伸縮、IPP）、DirectWaveCtEngine（4.8 MB、19,695 関数）、SG_x64（Delphi）、Reverb、EasyFLAC、DelZip64、REX、ILRemoteServer、svctl64、sveng64 | 機能ごと |
| 道具の x64 EXE 8 個 | 8 | 38.0 | 110,864 | 110,856 | 14.57 | 160〜205 | ILMinihostBridge64、ILPluginScanner64（JUCE）、ilbridge 64、ControlCreator、PluginManager、fldiagnostic（Delphi）、CopyLicenseToAllUsers、RemoveLicenses | 別プロセスの道具。DLL ではなく EXE |
| ホスト EXE（FL64.exe、FL64 (scaled).exe） | 2 | 1.2 | 2,232 | 2,144 | 0.68 | 7.5〜9.6 | 597 KB ×2（同一の中身ではない。SHA-256 が違う。署名ディレクトリあり） | FLEngine を読んで `CreateFruityInstance` を呼ぶ |
| FL Studio ASIO ドライバー | 1 | 0.5 | 1,665 | 1,599 | 0.34 | 3.7〜4.8 | ILWASAPI2ASIO_x64.dll 531 KB（+ x86 版）。COM 風の export、AVRT | ASIO を使う場合のドライバー。別コンポーネント |

- x86 の PE（一意 49 個。NSIS 補助 6 を除く）は lift の対象外と読む（FLEngine は x64。32 ビット版 DLL は x64 の FL から読まれない）。ただし 32 ビットの外部プラグインを使うブリッジ `ilbridge.exe`（x86）は別プロセスの道具。
- 「いつ必要か」はこれまでの調査では確認していない（A4 が DllMain までのため）。

### 5.5 (d) プラグイン

| 群 | 一意 PE | MB | pdata | 独立 | 被覆 MB | 共有なしで全部 lift した wasm の見積もり MB | 内容 |
|---|---:|---:|---:|---:|---:|---:|---|
| Delphi 製のネイティブ・プラグイン（ジェネレータ 43 + エフェクト 89） | 132 | 588.6 | 1,819,636 | 1,819,636 | 266.79 | 2929〜3756 | FLEngine/ILTools と同形。共有なしなら 1 個ずつ lift |
| MSVC 製のネイティブ・プラグイン（ジェネレータ 22 + エフェクト 5） | 27 | 65.9 | 133,601 | 91,999 | 36.36 | 399〜512 | `RT_CODE` 18、IPP 1 ほか。FL Studio Mobile 3 個（各 8〜12 MB）を含む |
| 補助 DLL（engine_x64、ZGE の NDI ほか） | 22 | 51.7 | 59,731 | 48,021 | 13.81 | 152〜194 | VST2 形（VSTPluginMain を export）の engine 7 個、NDI 27.1 MB など |
| FL Studio VSTi 2 + FLReWire + ReWire.dll | 4 | 13.0 | 45,777 | 45,461 | 5.59 | 61〜79 | Delphi 3 + MSVC（Propellerhead 製）1 |

- **共有なしの見積もりは Delphi 製だけで 2929〜3756 MB**（6 モジュール wasm 389 MB の約 7.5〜9.6 倍）。1 個あたりの中央値は被覆 1.6 MB → 約 18〜23 MB。
- **既存 3 モジュールに無い関数だけを足す場合（§4.3 の共有が成り立つと仮定）**: 和集合の「新しい」コード 10.4 MB → 約 114〜146 MB。1 個ずつなら、エフェクトの中央値 184 KB → 約 2.0〜2.6 MB、ジェネレータの中央値 286 KB → 約 3.1〜4.0 MB、最大のジェネレータで 2,222 KB → 約 24〜31 MB。**これは「共有してよい」という前提の下限で、実現性は未検討**。
- **外部プラグイン**（VST/VST3/CLAP）は分類上 (d) だが、インストーラに無く、見積もりは立てられない（§4.4）。

### 5.6 難しさの見立ての根拠（読み。全体）

| 特徴（実測のヘッダ） | 該当する DLL | 既存の経験に照らした読み |
|---|---|---|
| Delphi 36.0、ILTools と同じ形（`.pdata` 11〜24k、InitUnits 形、import が user32/gdi32/comctl32 など） | プラグイン 132、speaker、SG、VSTi、FLReWire、ILVideo | ILTools は「ツールが変更なしで通る」（bad_nodes 0）。壁は Windows API の応答（GDI の DC/DIB など）。プラグイン 115/132 は 6 モジュールに無い import が 0 なので、応答の壁は新しくは増えないと読める（応答表の行があるかは別）|
| IPP を静的リンク（IPPCODE/IPPDATA セクション） | ildsp、oggio、elastique、Frequency Splitter/Multiband Delay の engine | dsp_ippv2 で SSE/AVX の未対応命令が大量（97 万）、CPUID による分岐が壁。同じ難しさが来ると読む |
| C++ 例外・TLS ディレクトリ・条件変数 | ILSvg、beat_tracker、WebView2Loader、elastique | Blend2D の経験（暗黙の TLS、SRW ロックと条件変数、決定 0092）がある |
| 巨大な関数・巨大な DLL | python312（12.8k）、ffmpeg（avcodec 35.5k）、ILVideo | locals 5 万超の関数（dsp 15、Blend2D 4）が出る可能性。ビルドの時間とピークメモリ（emcc 8.8 GB）|
| 他の言語実行環境・外部ランタイム | python312（CPython）、FLMManaged（.NET）、WebView2（Edge）、ffmpeg | lift ではなく置き換えを検討する対象（論点）|
| CRT の api-set（api-ms-win-crt-*）・msvcrt | python312、ffmpeg、plugin の VC++ 系 | 仮想モジュール（決定 0081）の拡張が要る |

## 6. ブラウザ配布の観点（項目 5）

**見積もりと明記する。** 係数は §5.2 の 3 つの実測の点（10.98、11.13、14.08）から作った 11〜14 倍で、新しい DLL には測っていない。

| 項目 | 区分 | wasm（MB） | 備考 |
|---|---|---:|---|
| 6 モジュール wasm（FLEngine + 5） | 実測 | 389.4 | — |
| (b) 候補 10 DLL を lift | 見積もり | 89〜114 | 被覆 8.08 MB |
| Python の他の PE 30 個 | 見積もり | 37〜48 | 被覆 3.39 MB |
| ffmpeg 7 DLL | 見積もり | 491〜630 | 被覆 44.72 MB |
| その他の Shared の x64 DLL 10 個 | 見積もり | 106〜136 | 被覆 9.65 MB |
| 道具の x64 EXE 8 個 + ホスト EXE 2 + ASIO | 見積もり | 171〜220 |  |
| Delphi 製プラグイン 132（共有なし） | 見積もり | 2929〜3756 | 被覆 266.8 MB |
| Delphi 製プラグイン 132（既存 3 モジュールとの共有を仮定した「新しい」分） | 見積もり | 114〜146 | 28,779 関数・10.4 MB |
| MSVC 製プラグイン 27（共有なし） | 見積もり | 399〜512 | 被覆 36.4 MB |
| プラグイン補助 22 + VSTi/ReWire 4 | 見積もり | 213〜273 |  |

- **x64 のコードをすべて lift した場合（共有なし）の合計は約 4.8〜6.1 GB**（見積もり）。**Delphi 製プラグインの共有ができるとしても**、残りは約 2.0〜2.5 GB。単一の wasm は V8 の 1 GB 上限（`wasm6-report.md` §3 の引用）があり、現在の 389 MB は上限の約 4 割。
- コードの元のバイト数（参考）: 製品の x64 PE 一意 258 個で 954.3 MB（FLEngine 22.2 MB、dsp_ippv2 39.9 MB、ffmpeg 7 DLL 74.9 MB、Delphi 製プラグイン 588.6 MB など）。
- データ（そのまま配布する分。一意）: 1056.1 MB（音声 448.4 MB、FL の状態・プリセット・プロジェクト・MIDI 約 420.0 MB、画像・フォント・ZGE 約 141.5 MB）。圧縮後（LZMA、インストーラ内）は 614.2 MB。
- **プラグインとデータだけで**: PE（プラグイン一意 654.4 MB + 補助・VSTi・ReWire 約 64.7 MB）とデータ 1056.1 MB で、元のバイトでは約 1775 MB。

## 7. 未決の点（列挙のみ。決めない）

1. **(b) の定義**: 「起動に必須」の範囲を DllMain(ATTACH) に限るか、`CreateFruityInstance` 以降（メインウィンドウ、プラグイン探索、音声）までに広げるか。現状の実測では (b) は空で、広げるには A4 の先（M3 相当）を走らせる必要がある。
2. **同梱 DLL のうち何を lift し、何を置き換えるか**: wavpackdll（小さい。サンプル復号に必要と読める）、oggio（ogg/flac の復号。IPP）、ILSvg（UI の SVG）、ildsp（IPP）、beat_tracker、LAMEenc は lift の候補。python312、ffmpeg 系、WebView2Loader、`FLMManaged`（.NET）は置き換えの方が自然に読めるが、決めるには機能の優先順位が要る。
3. **プラグインの出し方**: 1 個ずつ別モジュール、共通部の共有、遅延ロードのどれか（§4.4 の 7）。共有を採るには、**グローバルのアドレスの扱い**（生成器の変更か、モジュールごとの表か）の決定が要る。
4. **対象プラグインの範囲**: 132 + 27 の全部か、代表（FL の標準のジェネレータ/エフェクト数個）か。1 個あたりの大きさは §5.5。
5. **外部プラグイン（VST/VST3/CLAP）の方針**: (A)〜(D) のどれか（§4.4 の 2）。ライセンス（再配布）と、ユーザーの DLL の入力経路（アップロードか）も。
6. **ライセンス**: 第三者製の DLL（§4.4 の 9）を再配布・変換してよいか。インストーラの入手元は記録が無い（`bundled-dll-inventory.md` §3.2）。
7. **実在モジュールの数の上限**: `RM_MAX` は 8 に上がっている（決定 0106）が、ここで挙げた (b)(c) を全部載せると足りない。基底アドレスの割り付けも（`bundled-dll-inventory.md` §8.2 の 2）。
8. **x86 の PE**（49 個。NSIS 補助を除く）と LilyPond、第三者インストーラ（232.9 MB）を配布の対象から外すか。
9. **A4 の先**を走らせるか（`CreateFruityInstance`、プラグイン探索 `Plugins\Fruity` の走査）。これをしないと (b)(c)(d) の「いつ必要か」は静的な読みのまま。

## 8. 未確認・限界

- 走らせていない: DLL/EXE の実行、`fe-lift`（新しい DLL への）、wasm のビルド、ブラウザ。難しさ（§5.3、§5.6）は import・セクション・作りと、既存 DLL の経験からの読み。
- **NSIS のスクリプトの全体は評価していない**: コンポーネント選択や条件分岐を無視して `SetOutPath` を線形に追ったので、パスの復元は読み。`$v0`、`$v38`、`$v43` の実体は未確認。`ILVideo_x64.dll` の配置 `Shared\ffmpeg\x64\` は復元パスから読み取った（`bundled-dll-inventory.md` §4 は未確認としていたが、今回の復元では `SetOutPath ${v31}\Shared\ffmpeg\x64` 相当に入っていた）。
- `.pdata` の個数は関数の数の目安で、`.pdata` に載らない関数がある（dsp_ippv2 では reloc が指すのに `.pdata` の範囲に入らないものが 1,544 個。`dll-nonpdata-reloc-seeds.md` §1 の引用）。他の DLL では数えていない。x86 の PE は数えていない。
- 共有の測定（§4.3）は、マスクの規則が不完全なため**下限**。Delphi 149 個と MSVC 48 個だけで、ほかの PE（ffmpeg、Python、道具）には行っていない。比べたのは `.pdata` が覆うコードだけで、データ領域（`.data`、ジャンプ表、RTTI）は比べていない。
- wasm の見積もり（§5〜6）の係数は FLEngine（Delphi）と dsp（IPP/MSVC）と残り 4 モジュール（混合）の 3 点。SIMD の多い ffmpeg、巨大関数の多い DLL では外れうる。V8 の locals 上限（5 万）に当たる関数は DLL ごとに未測定。
- FLEngine の外部プラグインの入口（§4.2）は文字列と `lea` の参照の静的な読みで、**呼び出しの実行も、VST2 の名前表を使う関数も特定していない**。QFC の VST3 ホストの中身（`GetPluginFactory` 以降）は読んでいない。
- プラグインの発見の仕組み（`Plugins\Fruity` の走査、`.fst`/`.nfo` の役割、`Presets\Plugin database`）は読んでいない。FL Studio Mobile と `FLMManaged.dll` の関係も読んでいない。
- 第三者製 DLL の版・ライセンスは版情報の会社名までしか見ていない。再配布条件は未確認。
- インストーラのダウンロード元は不明のまま（`bundled-dll-inventory.md` §3.2）。

## 9. 再現と作ったファイル

- **この文書**: `docs/investigations/m2-inventory.md`。補助 TSV は `docs/investigations/m2-inventory/` に 5 本（合計 168,601 B）:
  - `installer-files.tsv`（12,595 B）: 全 27,574 エントリを（区分、種別、ディレクトリ 3 階層）で集計
  - `kind-totals.tsv`（1,949 B）: 種別ごとの合計
  - `m2-modules.tsv`（10,457 B）: M2 の分類 (a)〜(d) と見積もり
  - `pe-inventory.tsv`（123,748 B）: PE 一意 313 個の一覧（SHA-256、機種、作り、pdata、import、export、TLS ほか）
  - `plugin-sharing.tsv`（19,852 B）: Delphi 製 149 個の、FLEngine∪ILTools∪QFC との関数の同一性
- **作り方**（RAM 上の `/tmp/opencode/m2inv/` に置いたスクリプト。リポジトリには入れていない）: (1) `read_header` で NSIS のヘッダ（3,519,116 B に展開）を得て、36 B ごとのエントリ（opcode、パラメータ 8 個）を走査。opcode 20（EXTRACTFILE）の名前（パラメータ 2）とデータオフセット（パラメータ 3）を取り、opcode 11（CREATEDIR）でパラメータ 2 が 1 のもの（`SetOutPath`）を追って親を付ける。(2) 一意のオフセット 25,164 個を 12 プロセスで `stream_file` により復号し、大きさ・SHA-256・先頭 16 B を数える（メモリ上。先頭が `MZ` のものは `pefile` で解析）。(3) `.pdata` の UNWIND_INFO（フラグ 1/2/4）を数える。(4) §4.3 は capstone 5.0.7 で関数ごとにマスクしたハッシュを作る。
- **規則の確認と判断の記録**: git の変更系操作なし、commit なし。`/` への書き込みは上の文書 1 本と TSV 5 本だけ。製品のファイルは `/` にも git にも置いていない。`/tmp/opencode/m2inv/` の使用量は最大で約 80 MB（中間の JSON が主で、製品ファイルの書き出しは無い）。依頼で許された「PE ヘッダの読み取りに必要な個別ファイルを `--write` で RAM に出す」の代わりに、**全 25,164 データをメモリ上で復号して大きさと SHA-256 を数えた**（ディスクにもRAMのファイルにも書かず、`--list` が大きさを出さないための判断）。`multiprocessing` のロックが「Disk quota exceeded」で作れなかったため、12 本のプロセスを `xargs -P` と `subprocess` で起動した（ファイルは作られていない）。

### 付録: エントリを復元する部分（RAM 上の `m2inv_load.py` から抜粋。`tools/extract_nsis2.py` を import）

```python
f = open(INST,'rb'); hdr, info = ex2.read_header(f); _, blocks = ex2.parse_blocks(hdr)
es = ex2.entry_size(blocks)            # 36
off, num = blocks[2]; so = blocks[3][0] # 命令表と文字列表
base = outdir = '?'
for i in range(num):
    e = struct.unpack_from('<9i', hdr, off + i*es)
    if e[0] == 11 and e[2] == 1:          # EW_CREATEDIR, SetOutPath
        s = fmt(e[1])                      # 3 + 1 語の変数符号を $vN に直す
        if '$v31' in s: outdir = s.replace('$v31', base)
        else: base = outdir = s
    elif e[0] == 20:                       # EW_EXTRACTFILE
        name, data_off = fmt(e[2]), e[3] & 0xFFFFFFFF
        rows.append((i, outdir, name, data_off))
```

- 変数符号: 文字列の語 `0x0003` の次の語が `(下位 7 bit) | ((上位 7 bit) << 7)` の変数番号（`0x809a` → 26 = `$PLUGINSDIR`）。
