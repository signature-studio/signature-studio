# 同梱 DLL の所在と入手経路（ILTools_x64.dll が「見つからない」件）

実行日: 2026-10-27。HEAD a951ecc（ワークツリーの追跡ファイルは HEAD と一致。`git status` の未追跡 5 件は他の作業のもの）。**調査のみ。この文書は何も決めない**（決定、runtime、生成器、既存文書、`lift/` の既存ファイルは編集していない）。作業物は `lift/out-inventory/`（新規。`.gitignore` の `lift/out*/` で無視される）。

## 0. 要約

1. **ILTools_x64.dll の PE 実体は、このマシンの上に「単体のファイル」としては無かったが、手元のインストーラの中にある**（実測）。`/home/ubuntu/ダウンロード/flstudio_win64_26.1.7.5653.exe`（SHA-256 が既存の記録と一致）の NSIS データにエントリ 56466、3,662,336 B、SHA-256 `20409dcbc5eb5055c476ed663d6e944984281155f15c6b3a5a2961ba061a0f95` が 1 件。今回、`tools/extract_nsis2.py --write` で `lift/out-inventory/extracted/ILTools_x64.dll` に 1 本だけ書き出した（`--hash` と `sha256sum` が一致）。
2. 他の同梱 DLL 10 名（webview2loader、beat_tracker、ilsvg_x64、ildsp_x64、oggio_x64、wavpackdll_x64、lameenc_x64、python312、ilvideo_x64、speaker_x64）も**同じインストーラに全部ある**（実測。サイズと SHA-256 は §4）。手元に抽出済みなのは dsp_ippv2、Blend2D、QFC（と FLEngine）だけ。
3. **入手経路の記録はある**: 3 本とも「インストーラから `tools/extract_nsis2.py --write` で 1 本だけ書き出した」と各調査文書に手順が書かれている（§3）。インストーラのパスは QFC 調査の §2 に書かれ、旧プロジェクトの引き継ぎ資料にも同じパスがある。**記録が無いのは、インストーラを誰がいつどこから置いたか**（ダウンロード元）。
4. **「ILTools_x64.dll を lift するために実体の入手が要る」わけではない**（実測）。既存の `fe-pe`、`fe-lift` が変更なしで通る（functions 12,526、`bad_nodes` 0、`unsupported_total` 352）。足りないのは実体ではなく、**実在モジュールとして載せるための決定と作業**（基底、`RM_MAX`、0086 に ILTools の個別の方向が無い点、応答など）で、§8 に事実と読みを分けて書く。
5. 0101 と他の文書の間に、**数値や名前の直接の食い違いは見つからなかった**（§6.3）。表現上の注意が 2 点ある。

## 1. 凡例

- **実測** = 今回（2026-10-27）のコマンド出力。保存先は `lift/out-inventory/` のファイル名で示す。
- **引用** = 既存文書の記述。今回は再測定していない。元の区分を括弧で添える。
- **読み** = 静的な読みと推論。実行で確かめていない。
- **未確認** = 確認していない。
- 「unit N」は InitTable の 0 始まりの index（init_count = index + 1。決定 0060）。
- census（決定 0097 の草案）と遅延モジュール（0096 の草案）の内容は、`lift/out-census/`、`lift/out-deferred/` が他の作業者の編集中なので**読んでいない**。census についての記述は 0101 の引用に限る。

## 2. 手元にある DLL（項目 1 の前半）

実測: `lift/out-inventory/pe-inventory-held.txt`（`pe_inventory.py`）。署名の欄は「データディレクトリ 4（Authenticode）の有無」で、**署名の検証はしていない**（検証できる道具を使っていない）。

| ファイル | サイズ | SHA-256 | Machine | ImageBase | SizeOfImage | Entry RVA | TimeDateStamp（UTC） | 署名ディレクトリ |
|---|---:|---|---|---|---|---|---|---|
| `lift/out-ext/dsp_ippv2_x64.dll` | 39,942,656 | `497dd4f77e859df9e86d999b3811be3c72526b41f86c38b0c3c6e395f691c352` | 0x8664 | 0x180000000 | 0x261f000 | 0x17caca8 | 0x6abbb88e（2026-09-29 13:09:34） | なし |
| `lift/out-blend2d/Blend2D.dll` | 2,065,920 | `66634d24412084b7541b4b27a11be8b0f4cb90d844c87e1ee5e5c3ecd66066c7` | 0x8664 | 0x180000000 | 0x300000 | 0x19e584 | 0x665d89e4（2024-06-03 09:16:20） | なし |
| `lift/out-qfc/quickfontcache_x64.dll` | 2,067,968 | `afc5a0ed70eb7e4af3d0f4baf771bd6f53eb014e237f857f23023471047736d2` | 0x8664 | 0x400000 | 0x20e000 | 0x19c1f0 | 0x6abbb8ff（2026-09-29 13:11:27） | なし |
| （参考）`binaries/win64/FLEngine_x64.dll` | 22,227,016 | `b2c64da40921cf262c15c5b25c2b0422e4d1851e953c87e0bc38ed2c19f9496d` | 0x8664 | 0x400000 | 0x18c0000 | 0xf76820 | 0x6abbb7e3（2026-09-29 13:06:43） | **あり**（オフセット 0x1530000、長さ 0x2848。末尾がファイル末尾） |

- 3 本とも PE32+、Characteristics 0x2022（DLL、LARGE_ADDRESS_AWARE）。いずれも `win64-extraction.md`、`external-modules-usage.md` §1.1、`blend2d-lift-feasibility.md` §2、`quickfontcache-lift-feasibility.md` §1 の既存の値と一致（実測）。
- 署名ディレクトリがあるのは FLEngine だけ（実測）。インストーラ由来の同梱 DLL に署名ディレクトリが無いことは、今回測った 4 本（ILTools を含む）の事実で、他の同梱 DLL は PE ヘッダを測っていない。
- `binaries/binaries/win64/FLEngine_x64.dll` にも同一内容のコピーがある（SHA-256 一致。`same-size-sha256.txt`）。
- 原本とバイト一致するコピー（実測。`same-size-sha256.txt`。原本を含めた本数）: dsp は `lift/out-p0/work/ident.dll`、`roundtrip.dll`、`lift/out-deferred/src/lift/out-ext/`（計 4 本）、Blend2D は `lift/out-blend2d/Blend2D-roundtrip.dll`、`lift/out-three/tmp/Blend2D.roundtrip.dll`、`lift/out-deferred/src/lift/out-blend2d/`（計 4 本）、QFC は `lift/out-deferred/src/lift/out-qfc/`（計 2 本）、FLEngine は `binaries/binaries/win64/`（計 2 本）。合計 12 本。同じサイズで SHA-256 が違うファイル（リベース済みの `*.2000000.dll`、`Blend2D.4700000.dll`、`qfc.0x1d00000.dll` など）は派生物で、原本ではない。

## 3. 入手経路の記録（項目 1 の後半）

### 3.1 文書にある記録（引用。インストーラ側は今回再測定）

| 対象 | 記録された手順 | 記録場所（git の初出） |
|---|---|---|
| インストーラ | `flstudio_win64_26.1.7.5653.exe`、1,106,033,456 B、SHA-256 `04393e7946c7369aaa5cd9ce1c755803e922146289e1ae3263629a44f4e5fc02`、NSIS | `win64-extraction.md` §1（a7791f8、2026-10-20）。`0046-m1-acceptance.md` 46 行。**今回の `sha256sum` と一致**（`installer-sha256.txt`） |
| FLEngine_x64.dll | `tools/extract_nsis2.py INSTALLER.exe --write FLEngine_x64.dll binaries/win64/FLEngine_x64.dll` | `win64-extraction.md` §2.3、`README.md` 51〜54 行 |
| dsp_ippv2_x64.dll | `--list dsp_ippv2`、`--write`（新規ファイルのみ）→ `lift/out-ext/dsp_ippv2_x64.dll`。エントリ 56484 | `external-modules-usage.md` §1.1（218312e、2026-10-23） |
| Blend2D.dll | `--list blend2d`、`--hash Blend2D.dll`、`--write Blend2D.dll lift/out-blend2d/Blend2D.dll`。エントリ 56450 | `blend2d-lift-feasibility.md` §2（47118ac、2026-10-24） |
| quickfontcache_x64.dll | `--list quickfont`、`--hash`、`--write QuickFontCache_x64.dll lift/out-qfc/quickfontcache_x64.dll`。エントリ 56472。**入力パス `/home/ubuntu/ダウンロード/flstudio_win64_26.1.7.5653.exe`（読み取りのみ）が書かれているのはこの文書だけ** | `quickfontcache-lift-feasibility.md` §2（9b65b66、2026-10-24） |
| 抽出ツール | `tools/extract_nsis2.py`（標準ライブラリのみ。`--list`、`--hash`、`--write`。`--write` は既存ファイルを上書きしない `xb`）。`7z l -tNsis` はこのインストーラを開けない | a7791f8。`loadlibrary-names-classification.md` §0.2、§5.2 |

`git log --all --grep` の該当（subject）: 218312e（external modules usage）、8b2fc8e（決定 0086）、f8b5e6d（unit 153）、0e9b703（決定 0101）、a7791f8（最初の抽出）。`-S'flstudio_win64'` は a7791f8、e84e99d、218312e、9b65b66。**git のコミットメッセージに、ダウンロード元やインストーラを置いた人の記録は無い**（実測。`git log -S'ダウンロード'` の該当は調査文書の本文）。

### 3.2 インストーラ自体の入手元

- 実測: `stat` で `/home/ubuntu/ダウンロード/flstudio_win64_26.1.7.5653.exe` の更新日時は 2026-10-09 14:43:15 +0900、1,106,033,456 B（`installer-stat.txt`）。同じ `ダウンロード` に Mac 版 `flstudio_mac_26.1.7.5419.dmg` と他社のプラグインのインストーラがある。
- 引用（旧プロジェクトの引き継ぎ資料 `~/work/signature-studio-dev-handover/docs/06-restart-guide.md` §6.1、2026-10-18 時点）: 「`~/ダウンロード/flstudio_win64_26.1.7.5653.exe`（約 1.1GB）— Win64。NSIS 形式」。「これらは『原資材』であり、利用条件は各社ライセンスと依頼者の判断に従うこと」。
- 引用（旧リポジトリの shallow clone `lift/out-oracle/old-ref/repo/docs/REFERENCES.md`）: `[DOWNLOADS]` = `~/ダウンロード/`「元インストーラ・プラグイン素材」。`fl-win64` の bundle（R2 のクラウドストレージ上の tar.zst）には installer と `extracted/` が入っていた。旧プロジェクトの抽出物 `/home/ubuntu/work/analysis/win64/extracted/block*_PE.bin` は**このマシンに無い**（実測: `/home/ubuntu/work/analysis` が存在しない。引き継ぎ資料 §6.1 も「Windows 抽出物一式」を「失われたもの」に挙げる）。R2 は認証情報が要るので見ていない。
- **文書に書かれていないこと**: インストーラのダウンロード元（URL）、誰が置いたか。分かるのは更新日時と、依頼者のホームの `ダウンロード` にあるという事実まで。

### 3.3 入手元に ILTools と他の同梱 DLL があるか

**ある**（§4 の表。実測）。`tools/extract_nsis2.py --list ""` の全 27,575 行は、過去の `lift/out-qfc/nsis-list-all.txt` と**バイト一致**（`cmp`。`nsis-list-all-recheck.txt`）。

## 4. インストーラの中の同梱 DLL（実測）

`tools/extract_nsis2.py INSTALLER --hash NAME` の出力（`installer-hashes.txt`。1 名 1 件に確定したものだけ。曖昧なら `ambiguous` と出る）。エントリとデータオフセットは `--list` の出力（`nsis-hits.txt`）。配置先: `Shared`（エントリ 56448 の `CreateDir` が `<NSIS 変数>\Shared`。56450〜56500 は途中でディレクトリ変更が無い並び。`lift/out-qfc/nsis-ctx-qfc-wide.txt`。NSIS の変数がどれかは**未確認**）。

| DLL（インストーラ内の綴り） | エントリ | データオフセット | サイズ | SHA-256 | 手元に抽出済みか |
|---|---|---|---:|---|---|
| **ILTools_x64.dll** | 56466 | 0x2f3b08ee | 3,662,336 | `20409dcbc5eb5055c476ed663d6e944984281155f15c6b3a5a2961ba061a0f95` | 今回 `lift/out-inventory/extracted/` に 1 本 |
| ILSvg_x64.dll | 56464 | 0x2f3326a2 | 1,504,256 | `92f76fefa88a8a70446492dc755324d2338bf382e11518e6f17d798f11342ba0` | いいえ |
| ildsp_x64.dll | 56492 | 0x30376105 | 4,197,376 | `f4b23b0f93ee9119ca21ad5255c2e239f9bb78d6664289e9888ddcab082faea2` | いいえ |
| oggio_x64.dll | 56494 | 0x304297cc | 1,872,384 | `b26c544f5d3987331bbc946d4e5ec3084004a5fff5739cafb9f602c66e6466f9` | いいえ |
| wavpackdll_x64.dll | 56500 | 0x304d1c93 | 271,672 | `3bd37cc93bb9e7bda5da35ed1784324b3d7f71c818642ed189ec306c51431bad` | いいえ |
| LAMEenc_x64.dll | 56468 | 0x2f493217 | 465,720 | `0703adcd734071c74e938d9e6dd5e075594823ff376ca5d96df7dd8a1e880843` | いいえ |
| beat_tracker.dll | 56480 | 0x2f748519 | 983,552 | `43942be8711e977db2a45f506b7565fac315bc7852bed8d8c6ff1ff351fff4e0` | いいえ |
| WebView2Loader.dll | 954、57956（同じオフセット） | 0xa56492 | 137,120 | `f4839ab5bd42f5d72aceb1399550ce5e8f2be2c721dffdd366c648232eb15bb2` | いいえ |
| speaker_x64.dll | 960 | 0xb77443 | 3,263,488 | `fe745134c283ffdda20af9f2dca730be28330aaea404e3c2d1095bef08826f81` | いいえ |
| python312.dll | 56692（`python312._pth` 56690、`python312.zip` 56694） | 0x3084f62a | 7,003,928 | `0cc4e557972488eb99ea4aeb3d29f3ade974ef3bcd47c211911489a189a0b6f2` | いいえ |
| ILVideo_x64.dll | 56724 | 0x30ecf2b3 | 1,329,152 | `82ab852157998e8809633c8f8997c398a34acf6dfdacc4c483852acbee719c55` | いいえ |
| Blend2D.dll | 56450 | 0x2efc7ec5 | 2,065,920 | `66634d24…66c7`（§2 と一致） | はい |
| QuickFontCache_x64.dll | 56472 | 0x2f523c1c | 2,067,968 | `afc5a0ed…36d2`（一致） | はい |
| dsp_ippv2_x64.dll | 56484 | 0x2fbd71d6 | 39,942,656 | `497dd4f7…c352`（一致） | はい |
| FLEngine_x64.dll | 950、59704、59725（同じオフセット） | 0x2726ae | 22,227,016 | `b2c64da4…496d`（一致） | はい |

- `ILVideo_x64.dll` はインストーラの一覧には**ファイル名だけ**が出る。ゲストが渡す名前は `ffmpeg\x64\ilvideo_x64.dll`（パス付き）。ディレクトリの設定（エントリ 56724 の前の `CreateDir`）は**読んでいない**（未確認）。
- 他の名前（実測。`nsis-list-all-recheck.txt`）: 32 ビット版 `dsp_ippv2.dll`（56482）と `QuickFontCache.dll`（56470）、`freetype.dll`（56488）、`freetype_x64.dll`（56490）、`elastique_x64.dll`（56486）、`REX Shared Library_x64.dll`（56474）、`DelZip64.dll`（56452）、`ILRemoteServer_x64.dll`（56462）、`FL Studio VSTi.dll`（57716。別に 58925 がある）、`vcruntime140.dll`、`vcruntime140_1.dll`。サイズと SHA-256 は測っていない。
- インストーラに**無い**名前（実測。部分一致 0 件）: declipper、stemextract、ds2wav、ss2wav（FL の DLL と読める名前）、`ilsvg.dll`、`ildsp.dll`、`oggio.dll`、`iltools.dll`、`wavpackdll.dll`、`lameenc.dll`（`_x64` なし）、avrt、powrprof（Windows の DLL）。EXTRACTFILE は 27,574 件、うち `.dll` で終わるのは 406 件（`loadlibrary-names-classification.md` §5.2 と一致）。

## 5. このマシン上のファイル検索（項目 2）

### 5.1 範囲と除外

- 検索範囲（`find-roots.txt`）: `/home/ubuntu`、`/opt`、`/mnt`、`/media`、`/srv`、`/usr/local`、`/usr/share/wine`、`/usr/lib/x86_64-linux-gnu/wine`（`/usr/lib/wine` は存在せず、`/media`、`/srv`、`/mnt/data` は空）。
- wine のプレフィックスの典型の場所は、存在の有無だけを `test -d` で見た: `~/.wine`、`~/.local/share/wineprefixes`、`bottles`、`lutris`、`~/.var`、`~/.cache/winetricks`、`Steam` はいずれも**無い**。`/usr/bin/wine`、`wine64`、`wineboot` は入っている。
- 方法: `find` の `-iname` で `iltools|ilsvg|ildsp|oggio|wavpack|lameenc|beat_tracker|webview2loader|speaker_x64|ilvideo|python312` を含む名前（`find-names-home.txt`、`find-names-other.txt`）、同梱 DLL のバイトサイズの完全一致（15 種。`find-sizes-home.txt`、`find-sizes-other.txt`。名前に依存しない確認。旧プロジェクトは PE を `block24347_PE.bin` のような名前で持っていたため）、インストーラ類（`*.exe|msi|cab|zip|7z|dmg|pkg|iso` の 1 MB 超。`find-installers-*.txt`）、`drive_c`、`system.reg`、`FL Studio*`、`Image-Line*`、`FLEngine*` など（`find-wine-fl-*.txt`）。`lift/out-*/` も同じ `find` で名前とサイズだけを見た（中身は読んでいない）。

### 5.2 結果（実測）

| 項目 | 結果 |
|---|---|
| 名前に該当（ホーム配下） | 580 件。内訳: `lift/out-a6/test/` と、過去の HEAD のコピー `lift/out-*/src/lift/out-a6/test/`、`lift/out-git/stage-*/lift/out-a6/test/` の `ext-ILTools-x64-dll-*`、`ext-WebView2Loader-dll-*`、`ext-beat-tracker-dll-*` など計 532 件（**ゲストの `LoadLibraryW` に名前を渡すテストの C とその実行ファイル。DLL の実体ではない**。`lift/out-a6/test/` だけで 400 件）、`lift/out-oracle/old-ref/`（旧リポジトリの `tools/iltools/`、`iltools_native_probe.c`、wavpack の法務メモ。23 件。**スクリプトとメモだけで DLL は無い**）、`lift/out-loadlib/` の読み取りメモ 2 件、今回の `lift/out-inventory/` 13 件（検索時点）、`~/fl-browser` 7 件（下の Mac の dylib）、`work/hsrlog/.cache` の `Unity.CecilTools`（ディレクトリ名の偶然の一致、無関係）3 件 |
| 名前に該当（`/opt`、`/mnt`、`/media`、`/srv`、`/usr/local`、wine） | **0 件** |
| **ILTools_x64.dll と同じサイズ（3,662,336 B）のファイル** | **今回書き出した `lift/out-inventory/extracted/ILTools_x64.dll` だけ**（それ以前から存在する Windows 用の ILTools はこのマシンに無い） |
| ほかの 10 DLL のサイズ（1,504,256 / 4,197,376 / 1,872,384 / 271,672 / 465,720 / 983,552 / 137,120 / 3,263,488 / 7,003,928 / 1,329,152 B）の一致 | **0 件** |
| 手元の 3 本 + FLEngine と同サイズ | 120 件（Blend2D 40、QFC 15、dsp 59、FLEngine 6）。原本と SHA-256 が同じなのは §2 の 12 本、残りは派生（リベース等）で SHA-256 が別 |
| 大きいインストーラ・アーカイブ（ホーム配下） | `ダウンロード/` の `flstudio_win64_26.1.7.5653.exe`、`flstudio_mac_26.1.7.5419.dmg`、Xfer Serum2・OTT、Native Instruments Massive X、Kilohearts、Fresh Air、Vital のインストーラ。`~/Persistent.7z`（7,118,885,611 B） |
| `~/Persistent.7z` | `7z l` でヘッダだけを読んだ（Method = Copy、260 ファイル、31 フォルダ）。`.dll`、`.exe`、対象名に**該当 0 件**（`persistent7z-matches.txt`）。先頭 3,000 B の一覧は `persistent7z-head.txt`。抽出も中身の読み取りもしていない |
| wine のプレフィックス（`drive_c`、`system.reg`） | **無し** |
| FL Studio のインストール先らしきディレクトリ | Mac 版の展開物 `~/fl-browser/mac/pkg_extract/payload_out/Applications/FL Studio 2026.app` だけ。**Windows の FL Studio のインストール先は無い** |

### 5.3 Mac 版の同名ライブラリ（Windows の DLL ではない）

`~/fl-browser/mac/pkg_extract/payload_out/Applications/FL Studio 2026.app/Contents/Resources/FL/Shared/` に Mach-O の dylib がある（実測。`file` は ILTools で「Mach-O universal binary with 2 architectures: x86_64 …」。`mac-dylibs-sha256.txt`）:

| ファイル | サイズ | SHA-256 |
|---|---:|---|
| ILTools_x64.dylib | 4,591,968 | `edbbae1b2c54af37fcaa75b391aadcb9edc735055917ed94303fc323114d0856` |
| ILSvg_x64.dylib | 3,297,952 | `f9c13d8c7083c273b9d07a14f8012a4c450d045bb5253732bf524006c55a92d0` |
| ildsp_x64.dylib | 847,504 | `4e0d252d3a99351028f609fa009a4c733e36f2d553daad7c43869bb7907be2ec` |
| liboggio_x64.dylib | 2,490,896 | `eaa996e4a36473ae1fcfdc15156c0b335c8809029ca407ad3a334150fb5f4a97` |
| beat_tracker.dylib | 2,565,392 | `3a80c5968adf3f97a2f2b3eff67201ee35b07fb46c320099433b8f561d2fd1e0` |
| wavpack_x64.dylib | 534,224 | `3de8f19bbd8b16ffaa0f5b6eb9ff203658bbda672e35a0c95e40a8ec41ba7799` |
| ffmpeg/osx64/libILVideo_x64.dylib | 938,336 | `d30e10008eb404284fa1ec21f1fc8c9cb3852c727b9a8fba94a68112e5014fd5` |

FLEngine_x64.dll が `LoadLibraryW` するのは PE なので、これは**その代わりにならない**（形式が違う。読み）。`ILTools_x64.dylib` には `CreateCursorFromFont`、`CreatePathManagerInstance`、`CreatePluginPathManagerInstance`、`DestroyPathManager` の名前が文字列としてある（`ShowEngineChoice` は無い。実測の文字列数）。**Windows 版と同じソースかどうかは未確認**。

## 6. FLEngine_x64.dll に現れる同梱 DLL 名（項目 3）

### 6.1 import 表（実測）

- 通常 import は **24 DLL、738 名**（`win64-extraction.md` §3 と `imports-vs-table.txt` の DLL 一覧が一致）。全部 Windows の DLL（`api-ms-win-crt-string-l1-1-0.dll` を含む）。**同梱 DLL は 0**。
- 遅延 import は 17 記述子、DLL 名は重複を除いて **14**（kernel32、user32、wtsapi32、msimg32、advapi32、rpcrt4、propsys、ole32、shell32、windowscodecs、uxtheme、imm32、DWMAPI、Shcore。`flengine-delay-strings.txt`）。**同梱 DLL は 0**。`loadlibrary-names-classification.md` §1 の 8（遅延 import の 14 DLL）と一致。
- したがって、同梱 DLL は **`LoadLibrary` 系（文字列の名前を実行時に渡す）でだけ**現れる。

### 6.2 `LoadLibrary` に現れる同梱 DLL 14 名（引用。出典は `loadlibrary-names-classification.md` §2.2、§6.2、§7.2 と 0101 の表）

「InitUnits の内外」は、**確認済み** = 実行（gdb、native、census）で、その unit が `LoadLibraryW` で止まったことが観測されたもの（出典の区分を引用）。**未確認** = 静的な読みだけで、実行で確かめていないもの。静的に届く unit は直接 call と jmp の辺（下限）と、`lea` を含む辺（上限）。vtable、コールバックの辺は無いので、「unit が無い」は「実行されない」を意味しない。

| # | DLL（ゲストの綴り → インストーラの名前） | 呼び出し点 | 静的に届く unit | InitUnits の内外 | 区分 |
|--:|---|---|---|---|---|
| 1 | `dsp_ippv2.dll`（dl=1）→ `dsp_ippv2_x64.dll` | 0x653f65 `f_6539e0` ← `f_653f40` ← `f_65c540` | **144**（ほかの呼び出し元 0x680648、0x11b285c） | **内** | **確認済み**（単独構成の停止。実測） |
| 2 | `Blend2D.dll`（dl=0） | 0x6641b9 `f_6539e0` ← `f_664190` ← `f_664260` | **149**、190（0x6cf2a0 経由） | **内** | 149 は**確認済み**（gdb。仮の値つき構成と、現行の構成の停止）。190 は読みと census 草案の実測（0101 の引用） |
| 3 | `QuickFontCache`（dl=1）→ `quickfontcache_x64.dll` | 0x665bed `f_6539e0` ← `f_665bd0` | **158**、245、279、647 | **内** | 158 は**確認済み**（仮の値つき構成の実測 `registry-unit153.md` §6.1。census 草案も 158 を示すと 0101 が引用） |
| 4 | `ILTools`（dl=1）→ **`iltools_x64.dll`** | 0x815e33 `f_6539e0` ← `f_815e10` | 245、279、**280**、647、**659** | **内** | **census 草案の実測**（unit 280 と 659。0101 の引用。**私は草案を読んでいない**）。最終の `LoadLibraryW` 文字列 `iltools_x64.dll` は依頼文と 0101 による。静的な読みは 5 unit |
| 5 | `ildsp`（dl=1）→ `ildsp_x64.dll` | 0x66ea2a `f_6539e0` ← `f_66ea10` | 直接 895、898。`lea` 込み 614、745、880、895、898 | 読みでは**範囲内**（index 895、898 は 942 の表の中） | **未確認**（実行していない） |
| 6 | `Python312`（`r8` = `Python\`、dl=0）→ `python312.dll` | 0xc395c5 `f_6539e0` ← `f_c395a0` | 直接 895、898。`lea` 込み 614 以降 | 読みでは範囲内 | **未確認** |
| 7 | `ffmpeg\x64\ilvideo_x64.dll`（パス付き） | 0x66d255 `f_6539e0` ← `f_66d230` | `lea` 込みのみ 614、745、880、895、898 | 読みでは範囲内 | **未確認**（パス付きなので runtime は別の理由でも止まりうる。読み） |
| 8 | `oggio`（dl=1）→ `oggio_x64.dll` | 0x675334 `f_6539e0` ← `f_675310` | `lea` 込みのみ 614 以降 | 読みでは範囲内 | **未確認** |
| 9 | `WavpackDLL_x64.dll` | 0xa58bbd `f_6539e0` ← `f_a58ba0` | `lea` 込みのみ 614 以降 | 読みでは範囲内 | **未確認** |
| 10 | `ILSvg`（dl=1）→ `ilsvg_x64.dll` | 0x66dc2a `f_6539e0` ← `f_66dc10` ← `f_66dcf0` | なし（根 1） | 不明 | **未確認** |
| 11 | `LAMEEnc`（dl=1）→ `lameenc_x64.dll` | 0xa5d914 `f_6539e0` ← `f_a5d8f0` | なし（根 5） | 不明 | **未確認** |
| 12 | `Speaker_x64.dll` | 0xa65b24 `f_44db40` ← `f_a65ac0` | なし（データ参照の祖先 52） | 不明 | **未確認** |
| 13 | `WebView2Loader.dll` | 0xbe3539 LoadLibraryW ← `f_be3520` | なし（根 18） | 不明 | **未確認** |
| 14 | `beat_tracker.dll` | 0x11336a4 LoadLibraryW ← `f_1133660` | なし（根 1。vtable の可能性。未確認） | 不明 | **未確認** |

- 手元にある 3 名（1〜3）と ILTools（4）が、確認済みの 4 つ（0101 の決定 4 の表と一致）。
- 14 名すべてが §4 の表のとおりインストーラにある（実測）。
- 文字列としてだけ FLEngine にある名前（実測: `flengine-delay-strings.txt`）: `REX Shared Library_x64.dll`、`DelZip64.dll`、`AudioRestore.dll`、`CloudMastering.dll`、`ILRemoteServer.dll`、`FL Studio VSTi.dll`、`licence-engine\LicenceEngine.dll`、`LAMEEnc.dll` ほか。`LoadLibrary` の引数との結び付けは既存文書に無い（名前が静的に決まらない 22 点の一部と読めるが、**未確認**）。`DelZip64.dll` は呼び元が名前を渡す形（`loadlibrary-names-classification.md` §9）。

### 6.3 0101 との照合（食い違いの有無）

0101 の次の記述を、`loadlibrary-names-classification.md`、`external-modules-usage.md`、`0086` と突き合わせた（引用の照合。census の草案は読んでいない）。

| 0101 の記述 | 照合結果 |
|---|---|
| 54 行: `blend2d.dll` は unit 149、190 | 一致（分類 §2.2 の #4、§7.2） |
| 55 行: `quickfontcache_x64.dll` は unit 158 | 一致（0086 の 27 行、分類 §7.2） |
| 56 行: `iltools_x64.dll` の静的な読みは分類 75 行（行 6）で unit 245、279、280、647、659 | 一致（75 行が #6 ILTools。§2.2 の表、§7.2 の 245 の行） |
| 58 行: 他の 10 名（0086 の 68 行の 11 名から iltools を除く） | 一致（0086 の 68 行の 11 名と同じ名前） |
| 58 行: ildsp_x64、python312 は 895、898 に直接。oggio、wavpack、ilvideo は 614 以降（`lea` 込み）。ilsvg、lameenc、speaker、webview2loader、beat_tracker は静的な unit なし | 一致（分類 §2.2 の列、§7.2 の表） |
| 22 行（決定 4）: InitUnits 内に出る同梱 DLL は 4 つ | 「出る」= 実測で止まったもの。**矛盾はない**が、読みでは 5 名（ildsp、python312、oggio、wavpack、ilvideo）が index 614〜898 に静的に届く（0101 自身が 58 行で「未確認」と書く） |
| 56 行: ILTools は 0086 の対象 2 名に入っていない | 一致（0086 の範囲の節。0086 は ILTools を「実行が届いて調査した時点で決める」11 名に入れる） |

- **注意 1（表現）**: 決定 4 の「4 つ」は実測で出た名前の数で、InitUnits の中の同梱 DLL が 4 つだけという意味ではない（0101 はそう書いていないが、読み違えやすい）。
- **注意 2（時点）**: `docs/progress.md` 157 行は、ILTools を含む 11 名を「0086 が実行が届いて調査した時点で決めるとする」と書き、0101 以前の記述のまま（0101 の「関連する決定への影響」が progress.md の書き換えを別作業としている）。
- **census の unit 280、659 以外の 3 unit（245、279、647）で ILTools の `LoadLibraryW` に止まらなかった理由**は、文書にも私の測定にも無い（**未確認**）。

## 7. ILTools_x64.dll の事実（実測。`lift/out-inventory/`）

入力は `lift/out-inventory/extracted/ILTools_x64.dll`（SHA-256 `20409dcb…0f95`。`--hash` と一致）。

### 7.1 PE

| 項目 | 値 |
|---|---|
| 形式 | PE32+ x86-64 DLL、Characteristics 0x2022、10 セクション |
| ImageBase / SizeOfImage / Entry RVA | **0x400000** / 0x394000 / 0x2b35f0 |
| TimeDateStamp | 0x6abbb8e9（2026-09-29 13:11:05 UTC。FLEngine、QFC と同じ日のビルド） |
| 署名ディレクトリ | なし |
| 例外ディレクトリ（`.pdata`） | 11,747 エントリ（chained 0）、function_starts 11,747 |
| リロケーション | DIR64 72,082、absolute 232、432 ブロック（`fe-pe rebase` の出力） |
| export（7 件、すべて名前付き） | `dbkFCallWrapperAddr`（RVA 0x3082b0）、`__dbk_fcall_wrapper`、`DestroyPathManager`、`CreatePluginPathManagerInstance`、`CreatePathManagerInstance`、**`CreateCursorFromFont`**、`ShowEngineChoice` |
| 通常 import | 14 DLL、520 名、序数 0（winmm 4、comctl32 29、shell32 12、user32 199、version 3、oleaut32 16、advapi32 7、IMAGEHLP 1、kernel32 130、SHFolder 1、ole32 12、msacm32 1、api-ms-win-crt-string 1、gdi32 104）。**同梱 DLL への import は無い** |
| 遅延 import | 13 記述子（kernel32、user32、wtsapi32、msimg32、windowscodecs、shell32、imm32、uxtheme、DWMAPI、Shcore ほか）。すべて Windows の DLL |
| 自前の起動表 | `fe-pe pe-init-table`: unit_count 283、runnable 231、empty 51、noproc 1、init_context 0x15eac90（FLEngine と同じ Delphi の InitUnits の形。`init-table-iltools.tsv`） |
| kernel32 の動的ロード用 import | `LoadLibraryA`、`LoadLibraryW`、`LoadLibraryExW`、`GetProcAddress`、`GetModuleHandleW`、`GetModuleFileNameW`、`CreateThread`、`TlsAlloc` を import する |

- FLEngine が `GetProcAddress` する `CreateCursorFromFont`（FLEngine に UTF-16 の文字列が 1 つ。`Warning: shared cursor library could not be loaded.` も 1 つ）は ILTools の export にある。ほかの 4 つの export 名（`CreatePathManagerInstance`、`CreatePluginPathManagerInstance`、`DestroyPathManager`、`ShowEngineChoice`）は FLEngine に文字列として**無い**（実測の文字列数）。
- 旧リポジトリの記録（引用）と PE の構造が一致する: SizeOfImage 0x394000、Entry 0x2b35f0、`.pdata` 11,747 関数（`out-oracle/old-ref/repo/tools/iltools/REPORT.md`、`lift_measure.sh`）。同じ版のビルドとみられる（読み。旧ファイル `block24347_PE.bin` は無いのでバイト比較はできない）。

### 7.2 既存ツールを変更なしで通した結果

| 測定 | 結果 |
|---|---|
| `fe-lift lift FILE OUT.c --pdata`（HEAD a951ecc のソースを `lift/out-inventory/target` にビルドした `fe-lift`。`cargo-build.log`） | functions **12,526**、closure_instructions 400,735、emitted 401,255、**bad_nodes 0**、tail_calls 164、gotos 38,093、C 477,235 行・34,174,348 B、**unsupported_total 352**（`Jmp: indirect through IAT slot (not lifted)` 350、`Jmp: indirect target unresolved` 2）、jumptabs 0、msvc_tables 0、msvc_rejected 0（`lift-stats-iltools.txt`） |
| 比較（同じ `fe-lift`） | QFC: functions 6,712、unsupported 104（QFC 調査 §1 の値を再現）。Blend2D: functions 2,019、unsupported 596、jumptabs 31（`lift-stats-qfc.txt`、`lift-stats-blend2d.txt`） |
| 生成 C の最大関数 | `f_648290` 3,658 行、`f_5a5930` 3,409 行、`f_5c9e50` 2,525 行。1,000 行以上は 8 関数、5,000 行以上は 0（`gen/fn-sizes.txt`）。**locals 数は未測定**（QFC の最大は 13,559 行、Blend2D の 4 関数は 11,000 行前後で locals 約 10 万。0086 の未決項目 5） |
| `fe-pe rebase FILE 0x4a00000 OUT` | 通る（432 ブロック、DIR64 72,082）。`0x400000` へ戻した往復で**原本と SHA-256 が一致**。リベース後の `fe-lift` 統計は不変（functions 12,526、unsupported 352）。**0x4a00000 は測定用の値で、決定ではない**（`rebase-4a00000.txt`、`rebase-roundtrip.txt`、`lift-stats-iltools-rebased-4a00000.txt`） |
| import と現行の応答表（`imports_table.rs` の 268 行）の突き合わせ | 520 名のうち**行あり 174、行なし 346**（user32 172、gdi32 98、comctl32 29、kernel32 24、shell32 12 ほか）。比較: QFC 130 / 87、Blend2D 85 / 9、dsp_ippv2 70 / 2、FLEngine 224 / 514（`imports-vs-table.txt`）。行が無い import は、呼ばれたときに未対応の経路に入る（`imports_table.rs` 冒頭のコメント）。**実行で呼ばれるかは未測定** |

生成 C は `lift/out-inventory/gen/iltools.c` に書いたが、**コンパイル、リンク、実行はしていない**（生成した事実だけ）。

### 7.3 ILTools 自身が `LoadLibrary` する名前（静的な読み。`loadsites-iltools-2.txt`）

`pe_loadsites.py`（capstone で `.text` を線形に走査し、`jmp [IAT]` の thunk への直接 call と、`lea` で結んだ文字列を拾う）の結果。**実行はしていない**。

- Windows の DLL: `dwmapi`、`avrt`、`advapi32`、`PowrProf.dll`、`SHCore.dll`（`f_5ae320`。FLEngine の unit 145 の `f_65c9f0` と同形のローダー）、`User32.dll`、`shell32.dll`、`kernel32.dll`、`Msctf.dll`、`imm32.dll`、`oleaut32.dll`、`Keyboard Layouts` のレジストリ値 `Layout File` から取る動的な名前。
- **同梱 DLL**: 汎用ローダー `f_5a5690`（`SetDllDirectoryW` を呼び、`f_440f90`（SetErrorMode + LoadLibraryW）に渡す。FLEngine の `f_6539e0` に相当する形）の呼び出し元が、`dsp_ippv2.dll`（`f_5a5930`）、`Blend2D.dll`（`f_5c6b90`）、`QuickFontCache.dll`（`f_5c7e10`）、`ffmpeg\x64\ilvideo_x64.dll`（`f_5ce350`）を渡す。
- 読み: ILTools は FLEngine と同じ共有コード（Delphi）を持ち、QFC と同じく**他の同梱 DLL を読む形を含む**。ILTools の DllMain / InitUnits で**これらが実行されるかは未確認**（QFC 調査 §1 の 11 も、QFC の unit 88、129 が dsp_ippv2 を読む形を「読み」としている）。

## 8. ILTools_x64.dll を lift するために足りないもの

### 8.1 足りているもの（実測）

| 項目 | 状態 |
|---|---|
| PE の実体 | **足りている**。インストーラ（手元、SHA-256 一致）に 1 件。抽出手順は `tools/extract_nsis2.py INSTALLER --write ILTools_x64.dll OUTPUT`（今回 exit 0、SHA-256 が `--hash` と一致） |
| 入手経路の再現性 | 他の 3 本と同じ。ただしインストーラの入手元（ダウンロード元）は記録が無い（§3.2） |
| 既存ツールの適用 | `fe-pe`、`fe-lift` が変更なしで通る。`bad_nodes` 0。`unsupported_total` 352 は 99% が `jmp [IAT]` の行なし（350）で、SSE/AVX の未対応命令の集計行が無い（QFC の SSSE3 6/4/4 や Blend2D の VEX 命令のような項目が無い）。実測 |
| リベース | `fe-pe rebase`（決定 0080 の方式）が通り、往復が一致する |

### 8.2 足りないもの（事実と読みを分ける）

| # | 項目 | 区分 | 内容 |
|--:|---|---|---|
| 1 | ILTools 個別の方向の決定 | 事実（文書） | 決定 0086 は Blend2D と QFC だけを「実在モジュールとして載せる」と断定し、ILTools を含む 11 名は「実行が届いて調査した時点で決める」とする（0086 範囲の節）。0101 の決定 1 は「InitUnits の中で `LoadLibrary` され、影響する同梱 DLL」を実在モジュール構成に入れ、背景の表（決定 4 が指す表）に ILTools を**集合の一員として載せている**が、ILTools に 0086 相当の個別の方向決定は無い（0101 の 56 行は「0086 の対象 2 名には入っていない」と書く）。0101 の「範囲外」には、影響しないと判断された DLL への答えも決まっていないとある |
| 2 | 基底アドレス | 事実（算術と文書） | ImageBase 0x400000 は FLEngine（0x400000〜0x1cc0000）と**全面的に重なる**。SizeOfImage 0x394000 は、FLEngine の終端と dsp_ippv2 の基底（0x2000000）の隙間 0x340000 B より**大きい**（QFC の 0x20e000 は入る）。基底は 0086 の未決項目 1。dsp（0x2000000〜0x461f000）、Blend2D（測定用 0x4700000〜0x4a00000）、QFC（測定用 0x1d00000）と ILTools を同時に置く割り付けは決まっていない |
| 3 | 実在モジュールの表の上限 | 事実（コードと文書） | `lift/runtime/fle_realmod.c` の `RM_MAX` は **4**。dsp、Blend2D、QFC に ILTools を足すとちょうど 4 件。0086 の未決項目 2 は「4 件を超える場合は別の決定」「`gen_two.sh` は実在モジュールを 1 つしか取らず、3 つ目以降で `merge_tables.py` が通るかは未確認」と書く。ILTools と ILSvg 等の追加で 5 件以上になるなら別の決定になる（読み） |
| 4 | `GetProcAddress(NULL, …)` | 事実（文書）と読み | ゲストは `LoadLibraryW` が NULL でも `GetProcAddress(hModule, "CreateCursorFromFont")` を呼び、現行 runtime は hModule が仮想ハンドルでないと abort する（`loadlibrary-names-classification.md` §2.2 #6 注 1、`fle_modules.c:199-205`）。読み: ILTools を実在モジュールにすれば、この abort を通らずに `GetProcAddress` が export を引く |
| 5 | 応答 | 事実（実測）と読み | import 346 名に応答表の行が無い。FLEngine 自身も 514 名が行なしで 145 unit を通っている（実測。行なし = 呼ばれたときの停止）ので、**どれが ILTools の起動で呼ばれるかは未測定**。ILTools の DllMain と InitUnits（283 unit）を通すのに要る応答は未測定 |
| 6 | ILTools が読む他の同梱 DLL | 読み | §7.3。ILTools の InitUnits が `dsp_ippv2.dll`、`Blend2D.dll`、`QuickFontCache.dll`、`ilvideo_x64.dll` を読むなら、dsp、Blend2D、QFC が先に実在モジュールとして動いている必要がある（QFC に対する 0086 の制約と同形）。`ilvideo_x64.dll` は手元に無い（インストーラにはある）。**実行されるかは未確認** |
| 7 | DllMain の副作用 | 未確認 | ILTools の entry は 0x2b35f0。DllMain(ATTACH) を実行した記録は無い。TLS ディレクトリは無い。`CreateThread`、`TlsAlloc` を import する（スレッドや TLS を使う可能性。読み） |
| 8 | 生成 C のビルドと wasm | 未確認 | 生成 C をコンパイルしていない。wasm の locals 上限（50,000）は未測定（最大関数 3,658 行。QFC の 13,559 行より小さいが、locals は別） |
| 9 | 3 unit（245、279、647）で止まらない理由 | 未確認 | §6.3 |

### 8.3 別の手段があるか（事実の並べ替え。どれも採否を決めない）

| 手段 | 事実 | 現行の決定との関係（文書の事実） |
|---|---|---|
| (a) 実体を lift して実在モジュールにする | 実体は手元のインストーラにあり、ツールは通る（§7.2） | 0086 の方向（Blend2D、QFC について）。ILTools に当てはめる決定は無い（§8.2 の 1） |
| (b) 擬似モジュール（旧プロジェクトの「mode 3」） | 旧リポジトリの記録（引用。旧系列のコードで、今回は再現していない）: ILTools の分岐入力を非 0 にすると `units_run` 209 → 851（`old-ref/repo/tools/flelift/boot/REPORT_APPENDIX_672.md` §B）。引き継ぎ資料も 209 → 895 の突破要因に挙げる。ただし同じ記録が、native の読み込み成否は「未実測」「推定のまま」と書く | 0073 と 0086 は、実在する DLL を「無い」と定義することも、偽の応答表を持つ仮想モジュールにすることも採らない。ILTools についてこの扱いを許すかは決まっていない |
| (c) 停止のまま | 現状（census が unit 280、659 で止まる） | 0086 の「却下ではなく、単に未着手」 |
| (d) Mac の `ILTools_x64.dylib` | Mach-O。PE の代わりにならない（読み）。export 名は一部一致（§5.3） | 該当する決定は無い |
| (e) wine で native 読み込み（比較の基準にする） | 旧リポジトリの記録（引用）: `ILTools_x64.dll` を wine64 9.0 で読むと、ファイル名のままでは err=126、`ILTools.dll` に改名すると err=1114（`ERROR_DLL_INIT_FAILED`。DllMain の GUI 依存）で、実測できなかった（`REPORT_APPENDIX_4783.md` D-3） | native の比較対象にできるかは未確認。**今回は wine を実行していない** |

旧記録との違い: 旧 `REPORT_APPENDIX_4783.md` D-3 は `LoadLibraryW(L"ILTools")` を前提に「拡張子補完で `ILTools.dll` を探すので見つからない」と書く。現行の読み（`f_6539e0` が dl=1 で `_x64.dll` を付ける）と census の `iltools_x64.dll` では、ゲストが渡す名前は `ILTools_x64.dll` の形になる（引用の食い違い。現行の側が新しい）。

## 9. 未確認（まとめ）

- インストーラのダウンロード元（URL）と、置いた人。R2 の `fl-win64` bundle の有無（認証情報が要るので見ていない）。
- census 草案（`wall-census-2.*`）の中身。unit 280、659 の `LoadLibraryW "iltools_x64.dll"` を私は自分で再現していない（0101 とご依頼の文面の引用）。
- ILTools_x64.dll の DllMain、InitUnits の実行、ILTools が読む他の同梱 DLL が実行されるか、生成 C のコンパイル、wasm の locals、必要な応答の範囲。
- ILVideo_x64.dll の配置先ディレクトリ（`ffmpeg\x64\`）が NSIS の命令でどう設定されるか。
- ほかの 10 の同梱 DLL（ILTools 以外）の PE ヘッダ、署名ディレクトリ、export、import（サイズと SHA-256 だけ測った）。
- 署名ディレクトリの内容の検証（FLEngine の署名が有効か）。
- 隠しディレクトリ（`.config`、`.local`、`.cache` など）と `/tmp` の中。名前に該当するファイルがあるかは見ていない（意図して入っていない）。
- `/opt/containerd`（許可なし）。
- Mac 版 `ILTools_x64.dylib` が Windows 版と同じソースか。

## 10. 作ったファイル（`lift/out-inventory/`。すべて新規）

- スクリプト（読み取り専用）: `pe_inventory.py`（PE ヘッダと SHA-256）、`pe_delay_and_strings.py`（遅延 import と名前の文字列）、`pe_imports_vs_table.py`（import と `imports_table.rs`）、`pe_loadsites.py`（`LoadLibrary` 呼び出し点）。
- 出力: `pe-inventory-held.txt`、`pe-inventory-iltools.txt`、`installer-*.txt`、`nsis-list-all-recheck.txt`、`nsis-hits.txt`、`find-*.txt`（`.err` を含む）、`same-size-sha256.txt`、`mac-dylibs-sha256.txt`、`persistent7z-*.txt`、`fe-pe-*-iltools.txt`、`init-table-iltools.tsv`、`iltools-objdump-p.txt`、`iltools-delay-strings.txt`、`flengine-delay-strings.txt`、`loadsites-iltools*.txt`、`imports-vs-table.txt`、`lift-stats-*.txt`、`rebase-*.txt`、`cargo-build.log`。
- 抽出物と生成物: `extracted/ILTools_x64.dll`（3,662,336 B）、`rebased/iltools.4a00000.dll`、`rebased/iltools.roundtrip.dll`、`gen/iltools.c`（34 MB）、`gen/fn-sizes.txt`、`target/`（`fe-lift` のビルド出力。既存の `lift/target/` は使っていない）。
- 編集していないもの: `docs/README.md`、`docs/decisions/`、既存の調査文書、`lift/` の既存ファイル全部、`lift/out-census`、`lift/out-deferred`、`lift/out-harness-int`、`lift/out-a5dsp`、`lift/out-wall*`（名前の検索で見ただけ）。
- 規則: `rm` なし。既存ファイルの全体上書きなし（`>` は新規ファイルだけ。`--write` は `xb`）。git の変更系操作なし（`git log`、`git show`、`git status` のみ）。`/tmp` は使っていない（`cargo` とツールの実行では `TMPDIR=lift/out-inventory/tmp`。`find`、`sha256sum`、`objdump`、`7z l` は `TMPDIR` を設定せずに実行したが、一時ファイルを作らない読み取りの使い方で、`/tmp` を確認はしていない）。コマンド文字列に `sleep`、`nohup`、末尾の `&` は書いていない。`cargo build` は `--offline --locked` で、`Cargo.lock` は変わっていない。
