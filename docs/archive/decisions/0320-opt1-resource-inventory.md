# 0320(仮): 原本の FL Studio 2026 の install tree の資源の棚卸し(Data・Plugins ほか。読み取り専用)

状態: **仮の決定**(調査の記録)。作業者 `opt1`、2026-11-07 JST 07:00〜(`date` の時計)。依頼: PM(依頼者の観察 O6/O7/O10〜O12 への資源の棚卸し)。この文書は (1)(2)(5)。(3)(4) は 08:15 までに追記する。

> **実際に実行した確認(読み取り専用、Wine は起動していない)**: 原本の FL Studio 2026(Wine の prefix の中のコピー `~/work/opt1-wine/prefix/drive_c/Program Files/Image-Line/FL Studio 2026/`、**Wine 用に 9 個のプラグインのフォルダに `.off-0307` の接尾辞が付いている**ので、表では接尾辞を外した正規のパスで数えた)を、`os.walk`/`lstat`(`nice -n 19`、`ionice -c3`)で全部数えた(27,822 ファイル、シンボリックリンク 0)。ファイルを開いたのは、(a) seed との sha256 照合(218 ファイル)、(b) 圧縮率の標本(`compress-est.mjs`、読むだけ)。install tree には一切書いていない。依頼者の機械の値は 0 件で、**ここの数は我々の原本のコピーの実測**(依頼者の Data 792 MB・Plugins 867 MB に対し、こちらは Data 785.0 MiB、Plugins 867.7 MiB で、Plugins は一致、Data は 7 MiB〔0.9%〕少ない。依頼者の install との差は未確認)。

## 0. 先頭の点

1. **install tree 全体は 2,053.3 MiB(= 十進で 2.15 GB、27,822 ファイル)で、Data 785.0 MiB + Plugins 867.7 MiB = 1,652.7 MiB(PM の『1.62 GB』と同じ量)は、その 80.5% で、残り 400.6 MiB(19.5%)は Data・Plugins の外にある**。ほかに `Shared`(205.8 MiB、DLL 123 個)、`System`(128.9 MiB、864 ファイル)、インストール先の直下のファイル 14 個(43.4 MiB: FLEngine_x64.dll 21.2 MiB ほか)、`Artwork`(19.7 MiB)、`Help`(2.8 MiB)がある。
2. **依頼者が挙げた『テーマ・言語・テンプレート』は Data の中にあるのはテンプレートだけ**: テーマ = `Artwork\Themes`(39 ファイル、17.1 MiB、`.flstheme` 13 個)、言語 = `System\Languages`(15 ファイル、6.0 MiB、`.moe` 7 個ほか)、テンプレート = `Data\Templates`(70 ファイル、29.4 MiB)。したがって **Data と Plugins だけを遅延 VFS に載せても、テーマと言語は出ない**(Artwork と System も対象に入れる必要がある)。
3. **`Data\Patches\Packs\Drums (ModeAudio)\Kicks` の `*.wv` は 349 本(6.9 MiB)**(依頼者の数と一致)。`Drums (ModeAudio)` 全体は 1650 ファイル(58.0 MiB)、Data 全体の `.wv` は 1954 ファイル(117.8 MiB)。
4. **現在の seed(`seed.f7726113ae34.flpack`、CI の bundle `fl-engine-ci-7a9b0800`)は 225 エントリ・7,205,523 B**(brotli で 3,929,989 B)で、その中身は install tree からの **218 ファイル(5,638,150 B)すべてがバイト一致(sha256)** + Windows のフォント 3 個(arial・tahoma・lucon、1,478 KiB)+ 一覧の TSV + レジストリの種など。**install tree の 27,822 ファイル・2,053.3 MiB のうち、seed が持つのは 218 ファイル(0.78%)・5.38 MiB(0.26%)**。PM の覚え書きの『222 ファイル、約 11 MB』は、実測では 225 エントリ・7.2 MB(未圧縮)。
5. **seed の中の『インストール先』は `C:\Program Files\Image-Line\FL Studio 21\`**(guest の仮想ボリュームの名前)で、原本の実際のフォルダ名は `FL Studio 2026`。資源を足すときの根は guest 側の名前に合わせる(下の 2 の注)。
6. **Plugins は `Plugins\Fruity` だけ**(`Effects` 90 フォルダ、`Generators` 51 フォルダ、合計 141 フォルダ・6,414 ファイル)。VST・VST3・Serum 2 などは install tree の中に無い(サードパーティは別の場所に入る物で、棚卸しの対象外)。Plugins の 78.0%(676.9 MiB)は `.dll` 182 個で、そのうち各プラグインの本体 `*_x64.dll` が 175 個・640.0 MiB。

## 1. 構成と数(全体)

| 場所(`FL Studio 2026\` の下) | ファイル数 | 合計 | バイト |
|---|---|---|---|
| Plugins | 6414 | 867.7 MiB | 909835243 |
| Data | 20272 | 785.0 MiB | 823103474 |
| Shared | 123 | 205.8 MiB | 215848086 |
| System | 864 | 128.9 MiB | 135140557 |
| (インストール先の直下のファイル) | 14 | 43.4 MiB | 45529129 |
| Artwork | 80 | 19.7 MiB | 20619818 |
| Help | 55 | 2.8 MiB | 2925770 |
| **合計** | **27822** | **2053.3 MiB** | **2153002077** |

ファイルの大きさの分布(全体): 4 KiB 未満 15,223 個(10.3 MiB)、64 KiB 未満 9,236 個(194.9 MiB)、1 MiB 未満 3,088 個(585.0 MiB)、10 MiB 未満 263 個(958.3 MiB)、10 MiB 以上 12 個(304.9 MiB)。ディレクトリ 1,354 個、パスの平均 72.0 B(パスの合計 2.0 MB)。ファイル木のインデックス(パス・サイズ・オフセット)は、パス 2.0 MB + 1 件あたり約 12 B で約 2.3 MB(未圧縮)の見込み。

### 1.1 Data(20,272 ファイル、785.0 MiB)

| Data/… | ファイル数 | 合計 |
|---|---|---|
| Data/Patches | 18198 | 634.5 MiB |
| Data/System | 1955 | 74.2 MiB |
| Data/Demo projects | 46 | 46.8 MiB |
| Data/Templates | 70 | 29.4 MiB |
| Data/(直下のファイル) | 3 | 0.0 MiB |

`Data/Patches` の内訳:

| Data/Patches/… | ファイル数 | 合計 |
|---|---|---|
| Data/Patches/Packs | 5190 | 346.2 MiB |
| Data/Patches/Plugin presets | 8992 | 156.6 MiB |
| Data/Patches/Misc | 631 | 79.1 MiB |
| Data/Patches/Impulses | 268 | 41.3 MiB |
| Data/Patches/Plugin database | 419 | 7.9 MiB |
| Data/Patches/Soundfonts | 1 | 2.2 MiB |
| Data/Patches/Scores | 2334 | 0.7 MiB |
| Data/Patches/Channel presets | 246 | 0.4 MiB |
| Data/Patches/Mixer presets | 18 | 0.0 MiB |
| Data/Patches/Speech | 71 | 0.0 MiB |
| Data/Patches/Envelopes | 16 | 0.0 MiB |
| Data/Patches/(直下のファイル) | 12 | 0.0 MiB |

`Data/Patches/Packs` の内訳:

| Data/Patches/Packs/… | ファイル数 | 合計 |
|---|---|---|
| Data/Patches/Packs/FLEX | 16 | 89.9 MiB |
| Data/Patches/Packs/Drums | 1916 | 65.9 MiB |
| Data/Patches/Packs/Legacy | 942 | 59.5 MiB |
| Data/Patches/Packs/Drums (ModeAudio) | 1650 | 58.0 MiB |
| Data/Patches/Packs/Instruments | 489 | 47.7 MiB |
| Data/Patches/Packs/Risers | 25 | 16.6 MiB |
| Data/Patches/Packs/SFX | 42 | 4.5 MiB |
| Data/Patches/Packs/Loops | 15 | 2.8 MiB |
| Data/Patches/Packs/Vocals | 47 | 0.8 MiB |
| Data/Patches/Packs/Shapes | 41 | 0.6 MiB |
| Data/Patches/Packs/(直下のファイル) | 7 | 0.0 MiB |

`Data/System` の内訳(深さ 2):

| Data/System/… | ファイル数 | 合計 |
|---|---|---|
| Data/System/FL Mobile/Instruments | 328 | 40.7 MiB |
| Data/System/Plugin databases/Plugin database (full) | 427 | 8.0 MiB |
| Data/System/Plugin databases/Plugin database (simple) | 419 | 7.9 MiB |
| Data/System/Plugin databases/Plugin database (alphabetical) | 413 | 7.9 MiB |
| Data/System/Render tests | 82 | 7.3 MiB |
| Data/System/Plugin databases/New | 51 | 1.0 MiB |
| Data/System/(直下のファイル) | 57 | 0.8 MiB |
| Data/System/MIDI import | 137 | 0.4 MiB |
| Data/System/FL Mobile | 3 | 0.1 MiB |
| Data/System/FL Mobile/Effects | 10 | 0.1 MiB |
| Data/System/Riff machine | 8 | 0.0 MiB |
| Data/System/Default/Installed plugins | 13 | 0.0 MiB |

`Data/Templates` の内訳(深さ 2):

| Data/Templates/… | ファイル数 | 合計 |
|---|---|---|
| Data/Templates/Advanced/Electronic | 18 | 15.2 MiB |
| Data/Templates/Advanced/Instrumental | 8 | 11.4 MiB |
| Data/Templates/Utility/Create a chord progression | 2 | 0.9 MiB |
| Data/Templates/Performance/Touchscreen | 2 | 0.3 MiB |
| Data/Templates/Performance/8x8 | 2 | 0.3 MiB |
| Data/Templates/Performance/4x4 | 2 | 0.2 MiB |
| Data/Templates/Power User/Cheese's recording setup | 2 | 0.1 MiB |
| Data/Templates/Other/Patch bank | 2 | 0.1 MiB |
| Data/Templates/Other/Video chopping | 2 | 0.1 MiB |
| Data/Templates/Utility/Drum mix | 2 | 0.1 MiB |

`Data/Demo projects` の内訳:

| Data/Demo projects/… | ファイル数 | 合計 |
|---|---|---|
| Data/Demo projects/Demo songs | 20 | 27.9 MiB |
| Data/Demo projects/Product demos | 10 | 10.2 MiB |
| Data/Demo projects/Visual | 9 | 6.3 MiB |
| Data/Demo projects/Performance mode | 6 | 2.0 MiB |
| Data/Demo projects/(直下のファイル) | 1 | 0.3 MiB |

Data の拡張子別:

| 拡張子 | ファイル数 | 合計 | 割合 |
|---|---|---|---|
| .wav | 3504 | 197.3 MiB | 25.1% |
| .fst | 8573 | 164.5 MiB | 21.0% |
| .wv | 1954 | 117.8 MiB | 15.0% |
| .flp | 165 | 83.6 MiB | 10.7% |
| .flexpack | 6 | 83.1 MiB | 10.6% |
| .mrp | 300 | 35.0 MiB | 4.5% |
| .ogg | 278 | 30.6 MiB | 3.9% |
| .png | 584 | 27.6 MiB | 3.5% |
| .flac | 227 | 20.6 MiB | 2.6% |
| .tbio | 790 | 7.5 MiB | 1.0% |
| .flex2pack | 2 | 6.8 MiB | 0.9% |
| .zgemesh | 16 | 2.6 MiB | 0.3% |
| .sf2 | 1 | 2.2 MiB | 0.3% |
| .gmsynth | 306 | 1.4 MiB | 0.2% |
| .jpg | 3 | 1.1 MiB | 0.1% |
| .dwp | 26 | 1.0 MiB | 0.1% |
| .mp3 | 2 | 0.9 MiB | 0.1% |
| .mid | 1718 | 0.6 MiB | 0.1% |
| .raw | 6 | 0.4 MiB | 0.0% |
| .ini | 174 | 0.2 MiB | 0.0% |
| .fsc | 609 | 0.1 MiB | 0.0% |
| .ico | 1 | 0.1 MiB | 0.0% |
| その他 9 種 | 1027 | 0.2 MiB | 0.0% |

Data の最大のファイル 8 個: `Data/Patches/Packs/FLEX/Packs/General Midi Library.flexpack` 40.3 MiB、`Melodic Techno.flexpack` 22.7 MiB、`Data/Patches/Plugin presets/Generators/Patcher/X WiseLabs/QDR.fst` 20.9 MiB、`Olbaid Compendium.flexpack` 9.1 MiB、`Data/Demo projects/Demo songs/Mehdi Ebrahiminejad - Never Out.flp` 6.8 MiB、`Mobile Synth Pluck.flexpack` 6.3 MiB、`Core Synths.flex2pack` 4.3 MiB、`Product demos/Square a Saw - Oxygen….flp` 4.2 MiB。ファイルの大きさの中央値 2.8 KiB、p90 78.2 KiB、p99 434.8 KiB、4 KiB 未満が 10,501 個(52%)、10 MiB 以上は 3 個だけ。

### 1.2 インストール先のそのほか

`System`(864 ファイル、128.9 MiB): `System/Installers` 29.7 MiB(1 ファイル)、`System/Tools/LilyPond` 24.1 MiB(335)、`System/Tools/Chord progression` 11.7 MiB(59、コード進行ツール)、`System/Tools/Bridge` 9.8 MiB、`System/Plugin/VSTi` 6.9 MiB、`System/Tools/Plugin Manager` 6.8 MiB、`System/Config/Thumbnails` 6.6 MiB(163)、`System/Languages` 6.0 MiB(15)、拡張子別では `.exe` 22 個 72.5 MiB・`.dll` 33 個 24.3 MiB・`.pyc` 73 個 10.5 MiB・`.png` 167 個 6.6 MiB・`.moe` 7 個 6.0 MiB。
`Shared`(123 ファイル、205.8 MiB): `avcodec-58.dll` 44.9 MiB、`dsp_ippv2_x64.dll` 38.1 MiB、`dsp_ippv2.dll` 26.9 MiB、`avformat-58.dll` 11.9 MiB、`avfilter-7.dll` 11.0 MiB、`python312.dll` 6.7 MiB、`SG_x64.dll` 5.6 MiB、`elastique_x64.dll` 4.8 MiB ほか。
`Artwork`(80 ファイル、19.7 MiB): `Themes` 39、`Wallpapers` 29、`Skins` 7、`Fonts` 5。

## 2. 現在の seed との対応

seed の実体: `lift/out-ci1/kit/bundles/fl-engine-ci-7a9b0800/seed.f7726113ae34.flpack`(`FLSEEDP1`、225 エントリ、7,205,523 B)。一覧 `vfs-seed-idle1.tsv`(決定 0129 の『guest が実際に開いた installed product のファイル』+ フォント)は 221 ファイル行 + 19 個の空ディレクトリ行。**218 ファイル(`C:\Program Files\Image-Line\FL Studio 21\` の下)の sha256 を原本の install tree と照合: 218/218 が同一バイト、パスの欠けも内容の違いも 0 件**。

| 場所 | install tree のファイル数 | 合計 | seed にあるファイル数 | seed の合計 | ファイル数の割合 | バイトの割合 |
|---|---|---|---|---|---|---|
| Data | 20272 | 785.0 MiB | 136 | 3.15 MiB | 0.67% | 0.40% |
| Plugins | 6414 | 867.7 MiB | 1 | 0.00 MiB | 0.02% | 0.00% |
| Shared | 123 | 205.8 MiB | 11 | 1.60 MiB | 8.94% | 0.78% |
| System | 864 | 128.9 MiB | 64 | 0.44 MiB | 7.41% | 0.34% |
| Artwork | 80 | 19.7 MiB | 4 | 0.18 MiB | 5.00% | 0.93% |
| Help | 55 | 2.8 MiB | 2 | 0.01 MiB | 3.64% | 0.37% |

seed が持つ 218 ファイルの内訳(install tree の先頭 2 階層ごと):

| 場所 | ファイル数 | バイト |
|---|---|---|
| Data/Patches | 130 | 2937873 |
| Shared/Artwork | 11 | 1674816 |
| System/Internet | 6 | 434552 |
| Data/Demo projects | 1 | 313356 |
| Artwork/Skins | 2 | 177047 |
| Data/System | 4 | 51827 |
| System/Config | 57 | 20730 |
| Artwork/Wallpapers | 2 | 14645 |
| Help/HelpAliases.scr | 1 | 8682 |
| Help/HelpPlugins.scr | 1 | 2267 |
| Plugins/Fruity | 1 | 1434 |
| System/Languages | 1 | 899 |
| Data/Demo projects.nfo | 1 | 22 |

- **seed にある物**: `Data/Patches/…` の 130 ファイル(各フォルダの `.nfo` と `Impulses/Default.wav` など = ブラウザのフォルダ説明)、`Shared/Artwork` の 11 ファイル(フォント、1.6 MiB)、`System/Config` の 57 ファイル、`System/Internet` の 6 ファイル、`Artwork/Skins`・`Wallpapers` 各 2、`Help` 2、`Data/System` 4、`Data/Demo projects/NewStuff.flp` 1(306 KiB)、`System/Languages` 1(0.9 KiB の 1 ファイルだけ)、`Plugins/…/MIDI Out/Data/GM CCs.txt` 1。空ディレクトリ 19 個(`Data/Patches/*`、`Data/Templates`、`System/Config/*` など)。
- **seed に無い物**: 上記以外の全部。依頼者が挙げた物では、**テーマ(`Artwork/Themes` の 39 ファイル)は 0 個、言語(`System/Languages` の 15 ファイルのうち翻訳の `.moe` 7 個)は `.moe` が 0 個で、`en.svg`(0.9 KiB、言語の旗の絵)の 1 ファイルだけ、テンプレート(`Data/Templates` の 70 ファイル)は 0 個(ディレクトリだけ)、`Packs\Drums (ModeAudio)\Kicks` の `*.wv` 349 本は 0 個**。`Plugins` は 1 ファイルだけ(MIDI Out のテキスト)で、プラグインの DLL は seed に無く、別の `dll.flpack`(77.4 MB、brotli 15.95 MB)に入っているのは FLEngine・IPP・Elastique などの本体側の DLL 6〜7 個。
- 注: seed のゲスト側のパスは `FL Studio 21`。原本の実体のフォルダ名は `FL Studio 2026`。資源の仮想 FS を足すときは、guest が見る根(`C:\Program Files\Image-Line\FL Studio 21\`)の下に同じ相対パスで置く(seed の 218 ファイルが原本と同じ相対パス・同じバイトであることは確認済み)。

## 3. 起動時・ブラウザ・Settings で FL が開くファイル: Wine の file trace の手順書(**未実行**。実行は PM の合図)

**今あるデータ(実行済みの確認)**: (a) 現在の seed の元の一覧 `vfs-seed-idle1.tsv`(決定 0129: guest が **idle まで**に実際に開いた installed product のファイル 218 + 空ディレクトリ 19。`tables/0320-seed-list-vfs-seed-idle1.tsv`)。上の 2 のとおり、`Data/Patches` の `.nfo`、`Shared/Artwork` のフォント、`System/Config`・`Internet` など。(b) Wine の古い +file の記録(`logs/fl-file.log`、1.8 MB)は起動の最初で終わっていて、install tree の中で開いたのは FLEngine_x64.dll だけ(`tools/file-trace-summary.py` で確認)で、Data・Plugins の情報は無い。**したがってブラウザ(左パネル)・Settings→General・Settings→Debug で開くファイルは未測定**。

**手順書**(Wine は Chrome と重ねない。前に `ps`・`free -m | sed -n 2,3p`・`vmstat 1 3 | tail -2` を記録、swap の入出力 ≠ 0 か MemAvailable < 8 GB なら始めない。所要の見込み 8〜12 分):
1. 構成: **prefix-e2 の中で** 隠してある 8 個のプラグインのフォルダを全部戻す(`bash tools/e2-unhide-all.sh unhide`。改名だけ。元の prefix は触らない。一覧は `e2/unhidden.list`)。終わったら `bash tools/e2-unhide-all.sh rehide` で隠した状態に戻す(Fruity Balance は G9 のために見えたまま)。cpuid の書き換えは今回の目的(ファイルの観測)には影響しない。
2. 起動(project なし): `OPT1_ENVF=$HOME/work/opt1-wine/env-e2.sh OPT1_WINEDEBUG='-all,+file,+timestamp' OPT1_NOPROJECT=1 nice -n 19 bash tools/run-fl.sh > exports/trace/fl-file-0320.log 2>&1`(バックグラウンド。`tools/run-fl.sh` に `OPT1_NOPROJECT=1` を足した: project を渡さない)。事前に私設の PipeWire・WirePlumber・Xvfb :88 が要る(今までと同じ)。`+file` は Wine の NtCreateFile・CreateFileW・GetFileAttributesW・FindFirstFileExW・FindNextFileW・NtReadFile(長さ)を記録する(offset は記録されない)。
3. 段階の切り方: ログは書かれ続けるので、**各段階の境界で `wc -l exports/trace/fl-file-0320.log` を控える**(`file-trace-summary.py --from-line N --to-line M` で段階ごとに集計できる)。段階〔P0〜P4〕: **P0 起動〜idle**(メインウィンドウが出て 30 秒静かになるまで。スクリーンショット `ffmpeg -f x11grab` で確認)、**P1 ブラウザ(左パネル)**(タブ ALL・PROJECT・PLUGINS・PRESETS・SOUNDS・STARRED を順に、各 3 秒待つ。フォルダの展開: SOUNDS で `Packs`→`Drums (ModeAudio)`→`Kicks` を開く、PLUGINS で `Effects`・`Generators` を開く)、**P2 Settings→General**(F10 または OPTIONS メニュー。開いてスクロール)、**P3 Settings→Debug**(Debug のタブ。ログの文字をスクリーンショットで残す = 依頼者の O8 と比べる材料。prefix-e2 は cpuid が SSE2 の答えなので『SSE3 no』などの行は製品と同じ形になる)。**P4 Settings→About**(PM の追加。gpu1 の観測: 製品の About は描かれるが、ロゴに当たる `TLightweightLayeredControl` が 4×3 で 1 枚だけ、スタッフロールが無い。FL がロゴ・スタッフロールのために**開くファイル**を `CreateFileW` で見る。install tree の `Artwork` の資源が seed にある 4/80 だけであること、`System/Internet/About` が `.url` 5 個だけ〔ロゴ・スタッフロールの資源は無い〕であることとの関係を確かめる: ロゴが `FL64.exe`・`FLEngine_x64.dll` の埋め込み資源〔`FindResource` 系〕なら、ファイルは開かれず、トレースに出ない〔その場合は別の観測が要る〕)。座標は 1 段階ごとにスクリーンショットを見て決める(`tools/xt.py click X Y` / `key NAME`。Settings の座標は未知)。
4. 後片付け: `/usr/lib/x86_64-linux-gnu/wine/wineserver -k`(prefix-e2 の env で)、`rehide`、FL64 0 本の確認、ログのサイズ(予想 数十〜数百 MB)。
5. 集計: `python3 tools/file-trace-summary.py exports/trace/fl-file-0320.log --from-line a --to-line b`(段階ごと)。出力 = 開いたファイルの数・属性だけ問い合わせたパス・列挙したディレクトリ(FindFirstFileExW の 1 回 + FindNextFileW の回数)・`NtReadFile` のバイト数(ファイルごと、上位 20)・install tree の先頭階層別の内訳。これで『どのファイルを全部読むか(= パックの先読み対象)』『ディレクトリの列挙だけで中身を開かないか(= 索引だけで足りる)』が分かる。
6. 限界: `+file` は offset を記録しないので『ファイルの何バイト目から読むか』は分からない(長さの合計だけ)。メモリマップ(MapViewOfFile)で読む物は NtReadFile に出ない(別に `+section` が要る)。Wine の kernel32 を通らない直接の syscall は無い前提(FL は通常の Win32)。

## 4. 配布単位の提案(索引 + HTTP range + OPFS キャッシュ。起動を待たせない = P1 を保つ)

**前提(PM の決定)**: 起動時に読むのは『ディレクトリの索引と seed』だけ。1.62 GB を OPFS に先に入れない。FindFirstFile 系と GetFileAttributes は索引だけで全域に効く。中身は要求時に R2 から HTTP range で取り、OPFS に保存、背景で先読みする。`.wv` はそのまま(中身は FL のデコーダが読む)。

**4.1 索引**: install tree 全体で 27,822 ファイル・1,354 ディレクトリ。パスを前方符号化(直前のパスとの共通部分を省く)した `パス + サイズ` の文字列が 511,546 B で、**brotli q11 で 151,881 B(実測)**。これに pack の番号・offset・フラグ(各 約 8 B)を足しても **約 0.25 MB(brotli 後の見込み)**。起動の経路(wasm 約 20 MB、seed 3.9 MB の brotli)に対して無視できる大きさで、P1 に影響しない。NTFS の `FindFirstFile` は**大文字小文字を無視した並び**で返すので、索引はその順に並べ、検索は大文字化したパスで行う(契約の実装。順序が違うと FL のブラウザの並びが変わる可能性がある〔読み〕)。属性は ARCHIVE(0x20)/DIRECTORY(0x10)、時刻は固定値。

**4.2 パックの分け方と概算サイズ**(brotli q9、1 MiB ブロックを独立に圧縮した標本からの外挿。`tables/0320-compress-q9-by-class.tsv`。`.wv`・`.flac`・`.ogg`・`.mp3` は圧縮率 1.0 として足した。q11 ならあと 3〜5% 小さい見込み〔未測定〕):

| グループ | 層 | ファイル数 | 展開後 MiB | brotli 概算 MiB |
|---|---|---|---|---|
| 索引 + seed(今ある物) | **層 0: 起動の経路** | 218 + 索引 | 5.4 + 約 0.5 | 3.9 + 0.25 |
| G1 Help+Artwork の小物 / G1a System の設定・言語・インターネット / G1b Artwork(テーマ・壁紙・スキン・フォント) | 層 1: 最初のフレームの後に背景で | 55 / 438 / 80 | 2.8 / 15.0 / 19.7 | 1.2 / 7.7 / 18.9 |
| G2a Data/Templates / G2b Data/System | 層 1 | 70 / 1,956 | 29.4 / 74.2 | 8.4 / 9.0 |
| G3a Data/Patches/Plugin presets(.fst) | 層 2: 要求時(ブロック単位) | 8,993 | 156.6 | 41.5 |
| G3b Data/Patches/Packs(.wv・.wav・.flexpack ほか) | 層 2 | 5,191 | 346.2 | 237.0 |
| G3c Data/Patches/Misc(デモ曲の音声)/ G3d Data/Demo projects / G3e Impulses・Plugin database・Scores ほか | 層 2 | 632 / 47 / 3,383 | 79.1 / 46.8 / 52.6 | 75.3 / 12.2 / 45.7 |
| G4a Plugins の DLL(182 個) | 層 3: プラグイン単位(lift 済みだけ) | 182 | 676.9 | 206.0 |
| G4b Plugins の DLL 以外(画像・音声・プリセット・ZGE) | 層 3: プラグイン単位 | 6,232 | 190.8 | 105.8 |
| G5 System/Tools・Installers・Plugin(実行ファイル) / G6 Shared / インストール先の直下 | 層 4: 実行しない物、または DLL pack 側 | 426 / 123 / 14 | 113.9 / 205.8 / 43.4 | 63.7 / 68.8 / 12.5 |
| **合計** | | **27,822** | **2,053.3** | **913.6** |

- **ブロックの形**: パックは 1 MiB(展開後)のブロックを独立に圧縮して並べる。ブロック表(圧縮後の offset・長さ・保存/圧縮の旗)は索引に入れる。**既に圧縮されたファイル(`.wv`・`.flac`・`.ogg`・`.mp3`、圧縮率 0.98)と大きな `.png`/`.jpg`(0.70)は再圧縮せず、パックの中に連続した 1 本の範囲として置き**、読む側は range で必要な範囲だけ取って復号なしで渡す(`.wv` は FL のデコーダが読むので、そのままのバイトを渡す)。小さなファイル(中央値 Data 2.8 KiB・Plugins 0.9 KiB、4 KiB 未満が 15,223 個)は同じディレクトリを同じブロックに詰め、ディレクトリを列挙したときにそのブロックを先読みする(要求の数を減らす)。
- **パック分け**: 層 1(約 45 MiB br)は 1〜2 本にまとめ、最初のフレームの後に背景で取る。層 2 は意味のまとまりで分ける: Plugin presets、Packs を 3〜4 本(`Drums` 65.9 MiB・`Drums (ModeAudio)` 58.0 MiB・`Instruments`+`Legacy`+`Risers` ほか・`FLEX` 89.9 MiB)、Misc、Demo projects、そのほか。各パックは 64〜128 MiB 以下(CDN のキャッシュが効く大きさ)。層 3 は**プラグインのフォルダ単位**(141 本。小さいものは複数まとめて 1 本にしてもよい): ZGameEditor Visualizer 87.0 MiB(1,859 ファイル)、FL Studio Mobile 41.1 MiB、Morphine 39.2 MiB が大きい。**DLL(676.9 MiB、brotli 206.0 MiB)は、その DLL が lift 済みのプラグインのものだけ中身を置く**(lift していない DLL は開かれても『could not be opened』になるので、中身の配信は無駄。索引にサイズと属性だけがあればブラウザの一覧と `GetFileAttributes` は成り立つ)。現時点で lift 済みのプラグインは Fruity Balance(3.5 MiB)だけなので、**層 3 の DLL を含めると 914 MiB、DLL の中身を lift 済みに絞ると約 708 MiB**(R2 の保管量の目安)。
- **R2 の置き方**: 中身のハッシュを名前に入れた不変のオブジェクト(`res/v1/<group>.<sha12>.pack`、`index.<sha12>.br`、`Cache-Control: public, max-age=31536000, immutable`)、`Accept-Ranges: bytes`。オブジェクト数は 層 0〜2 で約 15〜25、層 3 で 141 以下、層 4 で数本。同じ配信元(CDN 経由)から取り、Range の応答がキャッシュされること(サイズの上限、`Vary`)は実機で確認が要る(未確認)。更新はパック単位(中身が変わったパックだけハッシュが変わる)。
- **ブラウザ側(読み。実装は mt3/alpha1 の設計)**: Worker が索引を読んで仮想 FS の木を作る(エンジンの開始はここまで)。`CreateFileW`/`ReadFile` が未取得のブロックに当たったら、そのブロックの range を取って(JSPI などで待つ)OPFS の同期アクセスハンドルに書く。ブロックが入ったかは存在ビットマップで管理し、`navigator.storage.estimate()` の範囲で LRU に追い出す。ディレクトリを列挙したら、その直下のファイルの先頭ブロックを背景で先読みする。**1.62 GB を最初に入れる方式は採らない**(P1)。
- **未測定・未確認**: (a) FL が起動・ブラウザ・Settings で実際に読むバイト数(3 の file trace が要る)、(b) q11 の圧縮率、(c) R2/CDN の Range のキャッシュの挙動、(d) OPFS の容量(ブラウザの上限は端末で違う)、(e) 同期の読み出しの待ち(JSPI)が P2/P3 に与える影響、(f) 依頼者の install との Data の差 7 MiB。

## 5. Plugins の内訳(867.7 MiB、6,414 ファイル)

`Plugins\Fruity` の下に `Effects` 90 フォルダ(2,560 ファイル、499.6 MiB)と `Generators` 51 フォルダ(3,854 ファイル、368.0 MiB)。VST・VST3 のフォルダは無い。

Plugins の拡張子別:

| 拡張子 | ファイル数 | 合計 | 割合 |
|---|---|---|---|
| .dll | 182 | 676.9 MiB | 78.0% |
| .tga | 148 | 53.7 MiB | 6.2% |
| .wav | 497 | 47.6 MiB | 5.5% |
| .jpg | 255 | 14.3 MiB | 1.6% |
| .zgeproj | 329 | 10.5 MiB | 1.2% |
| .png | 62 | 8.6 MiB | 1.0% |
| .zfo | 41 | 8.0 MiB | 0.9% |
| .ttf | 47 | 6.2 MiB | 0.7% |
| .bmp | 134 | 5.6 MiB | 0.7% |
| .mp3 | 7 | 5.2 MiB | 0.6% |
| .flac | 42 | 5.0 MiB | 0.6% |
| .zgemesh | 29 | 4.2 MiB | 0.5% |
| .dat | 55 | 3.2 MiB | 0.4% |
| .flm | 11 | 2.9 MiB | 0.3% |
| .fst | 557 | 2.7 MiB | 0.3% |
| .ilv | 512 | 2.5 MiB | 0.3% |
| .sf2 | 2 | 2.2 MiB | 0.3% |
| .psd | 2 | 1.9 MiB | 0.2% |
| その他 35 種 | 3502 | 6.5 MiB | 0.7% |

- **種類別の合計(867.7 MiB)**: DLL 676.9 MiB(78.0%)+ GUI の画像(`.tga`・`.jpg`・`.png`・`.bmp`・`.psd`)84.5 MiB + 音声(`.wav`・`.flac`・`.mp3`・`.wv`・`.sf2`)60.9 MiB + ZGameEditor の資源(`.zgeproj`・`.zfo`・`.zgemesh`)22.7 MiB + プリセット(`.fst`・`.flmpst`・`.fnv`)3.5 MiB + フォント 6.2 MiB + その他 13.0 MiB。
- **DLL**: 182 個。各プラグインの本体 `*_x64.dll` が 175 個・640.0 MiB(1 フォルダ 1 個のものが多い)。最大は `ZGameEditor Visualizer` の `Processing.NDI.Lib.x64.dll` 25.9 MiB(NDI の同梱)、`FL Studio Mobile` 11.7 MiB、`FL Studio Mobile Rack` 7.9 MiB、`FL Studio Mobile Rack FX` 7.8 MiB、`FLEX/engineV2_x64.dll` 7.7 MiB、`ZGameEditor Visualizer_x64.dll` 7.2 MiB、`Patcher_x64.dll` 6.7 MiB(Effects と Generators に各 1 個)など。**プラグイン 1 個は 3〜8 MiB の DLL が普通**(Fruity Balance 3.5 MiB、Fruity Filter 3.6 MiB)。
- **プラグイン別の上位 40 件**(フォルダ、ファイル数、合計、種類別の MiB): 次の表。

| folder (kind/name) | files | total MiB | dll | gui-art | audio | preset | zge | font | other |
|---|---|---|---|---|---|---|---|---|---|
| Effects/ZGameEditor Visualizer | 1859 | 87.0 | 36.1 | 16.7 | 0.0 | 2.5 | 22.7 | 6.1 | 2.9 |
| Generators/FL Studio Mobile | 1414 | 41.1 | 11.7 | 0.2 | 24.1 | 0.6 | 0.0 | 0.1 | 4.4 |
| Generators/Morphine | 204 | 39.2 | 1.7 | 8.2 | 29.3 | 0.0 | 0.0 | 0.0 | 0.0 |
| Effects/Pitcher | 23 | 22.1 | 0.4 | 21.7 | 0.0 | 0.0 | 0.0 | 0.0 | 0.0 |
| Generators/FLEX | 11 | 18.4 | 18.1 | 0.0 | 0.0 | 0.0 | 0.0 | 0.0 | 0.2 |
| Generators/Harmor | 201 | 14.2 | 6.2 | 7.7 | 0.0 | 0.3 | 0.0 | 0.0 | 0.0 |
| Generators/Minisynth | 24 | 13.6 | 0.5 | 11.5 | 0.0 | 0.0 | 0.0 | 0.0 | 1.6 |
| Effects/Vocodex | 59 | 12.2 | 11.7 | 0.1 | 0.4 | 0.1 | 0.0 | 0.0 | 0.0 |
| Generators/FL Keys | 48 | 10.2 | 4.0 | 0.0 | 4.2 | 0.0 | 0.0 | 0.0 | 1.9 |
| Effects/Newtone | 4 | 9.6 | 9.6 | 0.1 | 0.0 | 0.0 | 0.0 | 0.0 | 0.0 |
| Generators/Fruity Slicer 2 | 41 | 8.3 | 8.3 | 0.0 | 0.0 | 0.0 | 0.0 | 0.0 | 0.0 |
| Effects/Distructor | 58 | 8.2 | 7.7 | 0.0 | 0.5 | 0.0 | 0.0 | 0.0 | 0.0 |
| Generators/FL Studio Mobile Rack | 2 | 7.9 | 7.9 | 0.0 | 0.0 | 0.0 | 0.0 | 0.0 | 0.0 |
| Effects/FL Studio Mobile Rack FX | 2 | 7.8 | 7.8 | 0.0 | 0.0 | 0.0 | 0.0 | 0.0 | 0.0 |
| Effects/Frequency Splitter | 4 | 7.4 | 7.4 | 0.0 | 0.0 | 0.0 | 0.0 | 0.0 | 0.0 |
| Generators/Sytrus | 112 | 7.2 | 6.0 | 0.0 | 0.0 | 0.0 | 0.0 | 0.0 | 1.2 |
| Generators/DirectWave | 28 | 7.0 | 7.0 | 0.0 | 0.0 | 0.0 | 0.0 | 0.0 | 0.0 |
| Generators/Kepler Exo | 3 | 6.8 | 6.8 | 0.0 | 0.0 | 0.0 | 0.0 | 0.0 | 0.0 |
| Effects/Patcher | 3 | 6.7 | 6.7 | 0.0 | 0.0 | 0.0 | 0.0 | 0.0 | 0.0 |
| Generators/Patcher | 3 | 6.7 | 6.7 | 0.0 | 0.0 | 0.0 | 0.0 | 0.0 | 0.0 |
| Generators/Autogun | 35 | 6.7 | 6.0 | 0.7 | 0.0 | 0.0 | 0.0 | 0.0 | 0.1 |
| Effects/Multiband Delay | 3 | 6.7 | 6.7 | 0.0 | 0.0 | 0.0 | 0.0 | 0.0 | 0.0 |
| Effects/Edison | 31 | 6.5 | 6.0 | 0.0 | 0.5 | 0.0 | 0.0 | 0.0 | 0.0 |
| Generators/Kepler | 3 | 6.3 | 6.3 | 0.0 | 0.0 | 0.0 | 0.0 | 0.0 | 0.0 |
| Generators/Ogun | 47 | 6.1 | 5.9 | 0.2 | 0.0 | 0.0 | 0.0 | 0.0 | 0.1 |
| Effects/Soundgoodizer | 6 | 6.1 | 6.1 | 0.0 | 0.0 | 0.0 | 0.0 | 0.0 | 0.0 |
| Effects/Fruity Love Philter | 29 | 6.1 | 5.7 | 0.4 | 0.0 | 0.0 | 0.0 | 0.0 | 0.0 |
| Generators/Slicex | 42 | 6.1 | 6.0 | 0.1 | 0.0 | 0.0 | 0.0 | 0.0 | 0.0 |
| Generators/Fruity Envelope Controller | 27 | 6.0 | 6.0 | 0.0 | 0.0 | 0.0 | 0.0 | 0.0 | 0.0 |
| Generators/FL Slayer | 13 | 5.9 | 5.6 | 0.3 | 0.0 | 0.0 | 0.0 | 0.0 | 0.0 |
| Effects/Fruity Convolver | 30 | 5.9 | 5.9 | 0.0 | 0.0 | 0.0 | 0.0 | 0.0 | 0.0 |
| Generators/FPC | 23 | 5.8 | 5.8 | 0.0 | 0.0 | 0.0 | 0.0 | 0.0 | 0.0 |
| Effects/Maximus | 23 | 5.8 | 5.6 | 0.1 | 0.0 | 0.0 | 0.0 | 0.0 | 0.0 |
| Generators/Fruity Wrapper | 3 | 5.7 | 5.7 | 0.0 | 0.0 | 0.0 | 0.0 | 0.0 | 0.0 |
| Effects/Fruity Wrapper | 3 | 5.7 | 5.7 | 0.0 | 0.0 | 0.0 | 0.0 | 0.0 | 0.0 |
| Effects/Gross Beat | 9 | 5.7 | 5.6 | 0.1 | 0.0 | 0.0 | 0.0 | 0.0 | 0.0 |
| Generators/Fruity Dance | 7 | 5.6 | 4.0 | 1.6 | 0.0 | 0.0 | 0.0 | 0.0 | 0.0 |
| Generators/Transistor Bass | 4 | 5.5 | 5.4 | 0.1 | 0.0 | 0.0 | 0.0 | 0.0 | 0.0 |
| Effects/Fruity X-Y-Z Controller | 2 | 5.5 | 5.5 | 0.0 | 0.0 | 0.0 | 0.0 | 0.0 | 0.0 |
| Generators/Dashboard | 91 | 5.5 | 4.8 | 0.6 | 0.0 | 0.0 | 0.0 | 0.0 | 0.1 |

- **特徴(読み)**: 大きなフォルダは (a) `ZGameEditor Visualizer`(87.0 MiB、1,859 ファイル。DLL 36.1 + ZGameEditor の資源 22.7 + GUI 画像 16.7 + フォント 6.1)、(b) 音声を同梱する `FL Studio Mobile`(41.1 MiB、1,414 ファイル、音声 24.1 MiB)・`Morphine`(39.2 MiB、音声 29.3 MiB)・`FL Keys`(10.2 MiB)、(c) GUI 画像が大半の `Pitcher`(22.1 MiB のうち `.tga` 21.7 MiB。`KNB Speed.tga`・`KNB Tune.tga` が各 7.8 MiB)・`Minisynth`(11.5 MiB)、(d) DLL だけの小さなプラグイン(多数)。ファイルの大きさの中央値は 0.9 KiB(4 KiB 未満が 4,313 個)、p99 4.0 MiB。**プラグイン DLL は `LoadLibrary` された時だけ中身が要り、製品では lift したプラグインだけがその対象**(G9: Fruity Balance が最初)。lift していない DLL は、存在(一覧と属性)が見えれば足りる場面が多い(ブラウザの一覧、プラグインデータベース)が、開かれたら今は『could not be opened』になる。

## 6. 索引の試作(読み取りのみ。`tools/ntfs_index.py`・`check-index.py`・`index-size.py`。製品のコードには触れていない)

**実際に実行した確認**: 原本の install tree を読み取り専用で歩き(`os.walk`/`lstat`)、`27,822` ファイル + `1,522` ディレクトリの木をメモリに作り、(a) NTFS の列挙順で並べ、(b) FindFirstFileW・FindNextFileW・GetFileAttributesW の契約(Microsoft Learn の本文: `FindFirstFileW`・`FindFirstFileExW`・`FindNextFileW`・`WIN32_FIND_DATAW`・`GetFileAttributesW`)に対する検査 24 項目を実行し、(c) 前方符号化した二進形式に書き出して brotli を測った。Windows 実機の NTFS では確認していない(実機の値は 0 件)ので、Learn に書いていない挙動は『既知の挙動(未検証)』と明記した。

- **生成器**: `ntfs_index.py` の `build(root)` が木を作り、正規のパスにする(Wine 用の `.off-0307` の接尾辞を外す)。`ntfs_key(name)` = 名前を 1 文字ずつ単純な大文字化($UpCase の代わりに Python の `str.upper()` で 1 文字になる物だけ)して UTF-16 のコード単位の列にして順序比較。**ディレクトリごとにこの順に並べる**。`.` と `..` は、親のあるディレクトリ(マウントの根 `FL Studio 21` を含む)の列挙の先頭に合成する。属性は ARCHIVE(0x20)/DIRECTORY(0x10)、時刻は『ファイル = 原本の mtime(インストーラが残した 2026-09 の時刻)、ディレクトリと作成時刻 = 固定のインストール時刻』。
- **この木の事実(実測)**: 名前の長さの最大 69、パスの最長 158 文字(+ マウントの根 41 文字 < 260)。ケースだけが違う名前の衝突 0、末尾のピリオド・空白 0、予約デバイス名 0、不正文字 0。非 ASCII の名前 19 個(`Noël` など、3 ディレクトリ)。**1,347 ディレクトリ(2 件以上)のうち 27 では『小文字化して並べる』と NTFS の順序が違い、251 では単純なコードポイント順とも違う**(例: `PL_RV_04…` と `PLRAD_02…` — `_`(0x5F)は大文字化後の `R`(0x52)より後ろ、小文字化では `r`(0x72)より前)。したがって**索引を NTFS の順で並べることは契約の実装に必要**(Learn は順序を保証しないが、NTFS では通常アルファベット順に返ると書いてあり、FL のブラウザの並びがこれに依存する可能性がある〔読み〕)。
- **索引の大きさ(実測)**: 前方符号化した名前 + 種別 + サイズ + 更新時刻(インストール時刻からの差の秒)+ pack 番号(310 本の例: 層の分け方の第 1 案)で、**二進 614,628 B → brotli q11 177,610 B(0.178 MB)、q9 196,022 B**。pack 内の offset は『同じ pack のファイルを木の順に連続して置く』約束で 0 バイト(pack の作り手が守る)。pack の表(310 本 × URL のハッシュとサイズ)を足しても約 0.2 MB。§4 の見積もり(0.25 MB)より小さい。
- **契約の検査の表**(`tables/0320-index-contract-checks.md`。21 ok、0 FAIL、3 OPEN):

| # | 項目 | Learn の契約 | 期待 | 試作の結果 | 判定 |
|---|---|---|---|---|---|
| T1 | enumeration order | Learn: FindFirstFile does no sorting, the order is not guaranteed and depends on the file system; NTFS usually returns alphabetical order (FindNextFileW remarks) | NTFS order: upper-cased UTF-16 code units, ordinal | 1347 directories with >= 2 entries; order differs from sort-by-lower in 27, from plain code-point sort in 251 | ok |
| T1b | example where the order matters | (same) | NTFS order differs from lower-case order | in "Data/Patches/Misc/Used by demo projects/Astes - Bien Duro": lower-case order 'PL_RV_04_Kick_Sample_777152.ogg' vs NTFS order 'PLRAD_02_Snare_Sample_904404.ogg' | ok |
| T2 | '.' and '..' | not mentioned on the FindFirstFileW page; NTFS (known behaviour, not Learn): a non-root directory listing starts with "." and ".." | '.' , '..' first, then entries | first 4: ['.', '..', 'Advanced', 'Empty'] | ok |
| T2b | mount root | (same; the mount root has a parent, "Image-Line") | '.' , '..' first | first 3: ['.', '..', 'Artwork'] | ok |
| T3 | directory name without wildcard | Learn: "an argument of C:\Windows returns information about the directory C:\Windows, not about a directory or file in it" | one entry: the directory itself, attribute DIRECTORY | 'Templates', attrs 0x10 | ok |
| T4 | trailing backslash | Learn: "An attempt to open a search with a trailing backslash always fails" (the error code is not specified) | fails | handle=None, error=123 | ok |
| T5a | no match in an existing directory | Learn: ERROR_FILE_NOT_FOUND | 2 | 2 | ok |
| T5b | directory does not exist | Learn FindFirstFile page names ERROR_FILE_NOT_FOUND for "no matching files"; the path error for a missing directory is the usual Windows behaviour (not on the page) | 3 (ERROR_PATH_NOT_FOUND) | 3 | ok |
| T5c | missing file without wildcard | Learn: ERROR_FILE_NOT_FOUND | 2 | 2 | ok |
| T6 | end of enumeration | Learn FindNextFileW: ERROR_NO_MORE_FILES | 18 | 18 | ok |
| T7 | "*.*" matches names without a period | not stated on the Learn pages; known Win32 behaviour of FindFirstFile | "*.*" == "*" | directory 'Plugins/Fruity/Generators/BassDrum': 7 vs 7 entries | ok |
| T8 | "*.wv" in Kicks | Learn: wildcards * and ?; the search is on the name only | 349 files, no "." / ".." | 349 entries | ok |
| T9 | case-insensitive search | Naming Files page (not fetched): names are case-insensitive on NTFS by default (known behaviour) | same 349 entries | 349 entries | ok |
| T10 | separators, "..", trailing dot | Win32 path normalisation (GetFullPathName; known behaviour) | same listing as Data\Templates\* | 9 entries | ok |
| T11 | "?" wildcard | Learn: "?" in the file name | any name with >= 1 character, plus "." and ".." (they match "?*"?) | 9 entries: ['.', '..', 'Advanced'] (whether Windows matches "." and ".." with "?*" is not verified) | OPEN |
| T12 | GetFileAttributesW Data\Templates | Learn: attributes of the file or directory; INVALID_FILE_ATTRIBUTES (0xFFFFFFFF) on failure with GetLastError | 0x10 | 0x10, error None | ok |
| T12 | GetFileAttributesW FL64.exe | Learn: attributes of the file or directory; INVALID_FILE_ATTRIBUTES (0xFFFFFFFF) on failure with GetLastError | 0x20 | 0x20, error None | ok |
| T12 | GetFileAttributesW Data\Nope | Learn: attributes of the file or directory; INVALID_FILE_ATTRIBUTES (0xFFFFFFFF) on failure with GetLastError | 0xffffffff + error 2 or 3 | 0xffffffff, error 2 | ok |
| T12 | GetFileAttributesW Nope\x | Learn: attributes of the file or directory; INVALID_FILE_ATTRIBUTES (0xFFFFFFFF) on failure with GetLastError | 0xffffffff + error 2 or 3 | 0xffffffff, error 3 | ok |
| T13 | WIN32_FIND_DATA fields of a file | Learn: dwFileAttributes, ftCreationTime (zero if unsupported), ftLastAccessTime, ftLastWriteTime, nFileSizeHigh/Low, dwReserved0 only for reparse points, cFileName, cAlternateFileName | size split 32/32, write time = file time, reserved0 0 | size 0:597064, write FILETIME 134351659080000000, create FILETIME 134351659080000000, reserved0 0, alt '' | ok |
| T14 | creation / last-access time | Learn: creation time of the file; the real install time is NOT known from the copy | a fixed install time (the newest mtime of FL64.exe) is used | creation = access = 134351659080000000 | OPEN |
| T15 | non-ASCII names | ordering by $UpCase (known NTFS behaviour; Learn silent) | upper-cased code units; "ë" == "Ë" | 3 directories with non-ASCII names; in "Data/Demo projects/Demo songs": ... ['DEDZII - NIGHT TERROR.flp', 'Edlan & Ella Noël - Song For You.flp', 'Electroconductor - Inner Days - Original.flp'] | ok |
| T16 | short (8.3) names | Learn: the search includes the long and short file names; cAlternateFileName holds the 8.3 name (empty if none, always empty with FindExInfoBasic) | prototype returns an empty alternate name | 29274 of 29344 names are not 8.3 and would get a short name on a volume with 8.3 creation; extension collisions through the short name: ".html" files match "*.htm" too (1 real .htm files + 25 .html files), ".flms" files match "*.flm" too (11 real .flm files + 239 .flms files), ".data" files match "*.dat" too (55 real .dat files + 2 .data files), ".flmpst" files match "*.flm" too (11 real .flm files + 581 .flmpst files), ".flmrack" files match "*.flm" too (11 real .flm files + 13 .flmrack files), ".flmapping" files match "*.flm" too (11 real .flm files + 11 .flmapping files) | OPEN |
| T17 | path length | Learn: MAX_PATH (260) unless \\?\ or the long-path opt-in | all paths below the limit | longest relative path 158 chars (+41 root) | ok |

21 ok, 0 FAIL, 3 OPEN


- **OPEN の 3 項目〔T11 の `?*` が `.` と `..` に当たるか・T14 の作成時刻・T16 の短い名前〕と、そこから出る未確認の点**: (1) **短い名前(8.3)**。Learn は『検索は長い名前と短い名前の両方を対象にする』『cAlternateFileName に 8.3 の名前が入る(無ければ空。`FindExInfoBasic` では常に空)』と書いている。この木の名前の 29,274/29,344 個(99.8%)は 8.3 形式でないので、8.3 名の生成が有効なボリュームでは全部に短い名前が付く。**短い名前で別の拡張子と衝突する例(実測)**: `.flms` 239 ファイル・`.flmpst` 581 ファイルは短い名前が `.FLM` になるので、実機では `*.flm`(`.flm` は 11 ファイル)の検索にも当たる。`.html`(25)は `*.htm` に、`.data`(2)は `*.dat`(55)に当たる。FL が実際にどんなパターンで列挙するか(`*.flm` を使うか)は file trace の `FindFirstFileExW` の引数で分かる(§3)。依頼者の実機で 8.3 名が有効かは未確認(実機の値は 0 件)。試作は alternate 名を空にしている。(2) **作成時刻・最終アクセス時刻**: 原本のコピーは mtime しか持たない(インストール時刻は不明)ので固定値にした。
- **このあとの確認(試作で未実行)**: Wine の `FindFirstFileExW` が返す実際の順序と、この試作の順序の比較(NTFS 実機ではなく Wine の ext4 上の層。Wine は Windows の NTFS を再現する物ではないので、基準にはしない)。Windows 実機の NTFS での列挙順・`.`/`..` の扱いは手元に無い。

## 7. V0.1: 配布物の生成器(mt3 0313 §19 の形式)と実測(07:30〜07:45。`tools/vfs-build.py` ほか。製品のコードには触れていない)

**実際に実行した確認(読み取り専用)**: 原本の install tree(Wine の prefix の写し、`.off-0307` の接尾辞は外した正規のパス)を読み、ローカルの `~/work/opt1-wine/vfs-out/run-a/` と `run-b/`(各 約 940 MiB。`/` に 590 GB の空き)に **索引 1 本・chunk 885 個・manifest** を作った(nice 19、ionice -c3、単一スレッド、1 回 2 分 20〜37 秒)。R2 へは上げず、リポジトリにも入れていない(manifest と表だけ `tables/` に入れた)。作った物を (a) **往復で検証**(索引を復号し、全 chunk を取り出し、全ファイル 27,822 個と全ディレクトリ 1,522 個を原本とバイト・サイズ・mtime・属性・名前で比較。`tools/vfs-verify.py`)、(b) **2 回作って比較**した。

**形式**: mt3 の `0313-g9a-balance-module.md` §19(規範)。V0 の最初の案(`info 07:24`)から 3 点変更・5 点追加を反映した: chunk の名前・索引の SHA-256 は**復号後の bytes**、ファイルごとの SHA-256 は**入れない**(欄は残し 0)、ディレクトリの記録を入れる、マウント点 `C:\Program Files\Image-Line\FL Studio 21` を頭に、名前は UTF-8・`/`・相対、流れの名前(`vfs/<名前>`)、`base_ft` + zigzag 秒、属性 bit(ファイルは ARCHIVE、ディレクトリは kind から)。**種別の組は 8 に絞った**(`bin`・`audio-compressed`・`image-compressed`・`pcm`・`image-raw`・`data`〔preset + project〕・`pack`・`other`〔text・font・不明〕)ので流れは **16**(読み手の上限 16 に収まる)。パラメータ: 小/大の閾値 256 KiB、chunk 小 1 MiB・大 4 MiB、identity は `audio-compressed` と `image-compressed` だけ、brotli は q9・窓 22・generic・size hint = chunk 長で固定。並びは (組, パス)、パスは NTFS の列挙順(深さ優先、ディレクトリが先)。流れの中の offset は直前のファイルの終わり(索引に書かない)。決定的(時計の値をどこにも書かない)。

**実測(run-a。run-b と全バイト一致)**:

- **索引**: 29,344 エントリ(ディレクトリ 1,522 + ファイル 27,822)、`index.bin` 667,223 B → **brotli q11 215,576 B(0.206 MiB)**(ファイルごとの SHA-256 無し)。付けた場合: 1,557,527 B → **1,055,298 B(1.01 MiB)**(+0.84 MB、縮まない)。名前 `index.9fde21c39f5d.br`(`index.bin` の SHA-256 の先頭 12 桁。全体は manifest)。V0 の第 1 案(全部の chunk の保存長入り)の 614,628 B → 177,610 B よりやや大きいのは、ディレクトリの記録、mtime、chunk の SHA-256(885 × 32 B = 27 KB)を入れたため。
- **chunk**: **885 個(小の流れ 488、大の流れ 397)、不変オブジェクト 885 個(重複 0)**、保存 982,747,629 B(937.2 MiB)= 論理 2,053.3 MiB の **0.4565**。保存したオブジェクトの大きさ: 最小 3,372 B、中央値 1 MiB、最大 4 MiB。**配布物の総数 = chunk 885 + 索引 1 + manifest 1 = 887**。
- **小ファイル(4 KiB 未満 15,223 個、10.28 MiB)**: 9 本の小さな流れの **115 chunk** に収まる(保存 62.6 MB)。すべての小さい流れ(256 KiB 未満の 26,895 個、論理 483.1 MiB)で 488 chunk(保存 341.0 MiB)。1 つのファイルが複数 chunk にまたがるのは 773/27,822 個、最大 13 chunk(`FLEX` などの大きな `.flexpack`)。
- **再現性**: 同じ入力から 2 回作り(run-a 2 分 20 秒、run-b 2 分 37 秒)、**`index.bin`・`index.<sha12>.br`・`manifest.json` がバイト一致、chunk のオブジェクト名 885 個が一致、885 個全部をバイト比較して一致**。ファイル一覧(888 ファイル)も一致。
- **往復**: 索引を復号 → 16 の流れを chunk から再構成(各オブジェクトの名前 = 復号後の bytes の SHA-256 を確認)→ **27,822 ファイル(2,153,002,077 B)と 1,522 ディレクトリが原本と一致、名前の抜けも余りも 0、問題 0**(約 11 秒)。

**流れの表**
| id | 流れ | 符号化 | chunk | ファイル数 | 論理 MiB | 保存 MiB | 率 | chunk 数 |
|---|---|---|---|---|---|---|---|---|
| 0 | large-audio-compressed | identity | 4 MiB | 130 | 72.6 | 72.6 | 1.000 | 19 |
| 1 | large-bin | brotli | 4 MiB | 245 | 1014.8 | 316.3 | 0.312 | 254 |
| 2 | large-data | brotli | 4 MiB | 278 | 184.8 | 34.9 | 0.189 | 47 |
| 3 | large-image-compressed | identity | 4 MiB | 31 | 31.3 | 31.3 | 1.000 | 8 |
| 4 | large-image-raw | brotli | 4 MiB | 45 | 53.9 | 7.9 | 0.147 | 14 |
| 5 | large-other | brotli | 4 MiB | 36 | 24.1 | 12.2 | 0.506 | 7 |
| 6 | large-pack | brotli | 4 MiB | 31 | 104.7 | 47.9 | 0.458 | 27 |
| 7 | large-pcm | brotli | 4 MiB | 131 | 83.8 | 73.0 | 0.871 | 21 |
| 8 | small-audio-compressed | identity | 1 MiB | 2450 | 108.3 | 108.3 | 1.000 | 109 |
| 9 | small-bin | brotli | 1 MiB | 141 | 8.4 | 2.9 | 0.347 | 9 |
| 10 | small-data | brotli | 1 MiB | 12718 | 114.5 | 27.6 | 0.241 | 115 |
| 11 | small-image-compressed | identity | 1 MiB | 1121 | 47.0 | 47.0 | 1.000 | 48 |
| 12 | small-image-raw | brotli | 1 MiB | 242 | 7.4 | 1.8 | 0.250 | 8 |
| 13 | small-other | brotli | 1 MiB | 5952 | 21.2 | 4.9 | 0.232 | 22 |
| 14 | small-pack | brotli | 1 MiB | 392 | 10.4 | 3.8 | 0.361 | 11 |
| 15 | small-pcm | brotli | 1 MiB | 3879 | 165.9 | 144.6 | 0.872 | 166 |


**種別の組ごと**
| 種別の組 | 符号化 | ファイル数 | 論理 MiB | 保存 MiB | 率 | chunk 数 |
|---|---|---|---|---|---|---|
| bin | brotli | 386 | 1023.3 | 319.3 | 0.312 | 263 |
| data | brotli | 12996 | 299.3 | 62.5 | 0.209 | 162 |
| pcm | brotli | 4010 | 249.7 | 217.6 | 0.871 | 187 |
| audio-compressed | identity | 2580 | 180.9 | 180.9 | 1.000 | 128 |
| pack | brotli | 423 | 115.1 | 51.7 | 0.449 | 38 |
| image-compressed | identity | 1152 | 78.4 | 78.4 | 1.000 | 56 |
| image-raw | brotli | 287 | 61.3 | 9.8 | 0.159 | 22 |
| other | brotli | 5988 | 45.2 | 17.1 | 0.378 | 29 |
| **合計** | | 27822 | 2053.3 | 937.2 | 0.4565 | 885 |

small streams: files 26895, chunks 488, logical 483.1 MiB, stored 341.0 MiB; large: files 927, chunks 397, logical 1570.1 MiB, stored 596.2 MiB
{"bytes": {"logical": 2153002077, "stored_all_chunks": 982747629, "stored_unique_objects": 982747629}, "counts": {"chunks_total": 885, "dirs": 1522, "entries": 29344, "files": 27822, "objects_unique": 885, "streams": 16}, "files_lt_4KiB": {"bytes": 10781929, "chunks_touched": 115, "files": 15223, "stored_bytes_of_those_chunks": 62624836}, "files_zero_length": 0, "index": {"br_q11": 215576, "name": "index.9fde21c39f5d.br", "raw": 667223, "sha256_of_index_bin": "9fde21c39f5de145a75d109328acd5c20ae6d0df678bbd1537e4f7c27a625826", "variant_with_file_sha_br_q11": 1055298, "variant_with_file_sha_raw": 1557527}}


**代表的なディレクトリを読むときの取得量**(触れる chunk の数と、その保存 bytes。chunk は他のディレクトリのファイルも含むので論理の合計より大きいことがある)
| Data/Patches/Packs/Drums (ModeAudio)/Kicks/ | 349 | 6.9 MiB | 7 | 7.0 MiB |
| Artwork/Themes/ | 39 | 17.1 MiB | 8 | 22.2 MiB |
| System/Languages/ | 15 | 6.0 MiB | 3 | 4.0 MiB |
| Data/Templates/ | 70 | 29.4 MiB | 11 | 6.5 MiB |
| Plugins/Fruity/Effects/Fruity Balance/ | 2 | 3.5 MiB | 3 | 2.5 MiB |
| Plugins/Fruity/Effects/Fruity Filter/ | 2 | 3.6 MiB | 2 | 1.4 MiB |
| Data/Patches/Packs/FLEX/ | 16 | 89.9 MiB | 24 | 42.6 MiB |
| Data/Patches/Plugin presets/Generators/ | 7022 | 134.3 MiB | 99 | 45.7 MiB |
| (現在の seed の 218 ファイル) | 218 | 5.4 MiB | 22 | 19.4 MiB |
files spanning more than one chunk: 773 of 27822
max chunks of one file: 13
stored object size: min 3372 median 1048576 max 4194320


- **読み**: (1) 保存の総量の標本の概算(913.6 MiB、§4)に対し実測は 937.2 MiB(+2.6%)。差は PCM の `.wav` を brotli にしても率 0.87 であること、`.png`/`.jpg` を identity にしたこと、chunk 単位の独立圧縮による損、の合計と読む(未分解)。(2) **アーティスト向けの塊の取り出し**: `Kicks` の `.wv` 349 本(6.9 MiB)は identity の 7 chunk(7.0 MiB)で、1 往復あたり 1 MiB。`Templates`(70 ファイル、29.4 MiB)は 11 chunk、保存 6.5 MiB。`Artwork/Themes`(39 ファイル、17.1 MiB)は画像が identity で、8 chunk・22.2 MiB(隣のディレクトリの画像が chunk に混じる)。**現在の seed の 218 ファイル(5.4 MiB)は 22 chunk・19.4 MiB に散る**ので、seed は chunk に置き換えず eager のまま(mt3 の L4(i) のとおり)。(3) **Plugins のプラグイン 1 個**: Fruity Balance(2 ファイル、3.5 MiB)= 3 chunk・2.5 MiB、Filter = 2 chunk・1.4 MiB。

**テスト用の木(V1 の vitest 用)**: `vfs-out/testtree-src/`(21 ファイル、空ディレクトリ 2 個、6 MiB の `.wav`〔4 MiB の chunk 2 個にまたがる〕、日本語のディレクトリ名とファイル名、`Noël`、0 バイトのファイル、NTFS の順で `a_b.txt` が `ab.txt` の後ろ、1.5 MiB の DLL、`.wv` 2 個)から作った索引と chunk = `vfs-out/testtree-out/`(`index.bbec64e4bd14.br`、chunk 10 個、manifest)。往復の検証は問題 0。mt3 に場所を連絡済み。

**OPEN / 未確認**: (a) 属性: 原本の写しは Wine の prefix なので READONLY・HIDDEN・SYSTEM は取れず、ファイルは ARCHIVE のみで出した(実機の値は 0 件)。(b) 時刻: ファイルは原本の mtime(2026-09 のインストーラ由来)、作成・アクセスは mtime と同じ、ディレクトリは `FL64.exe` の mtime(実機の install 時刻は不明)。(c) 短い名前(8.3): 生成しない(§6)。(d) R2 に上げたときの `Content-Encoding: br` の扱い・CDN のキャッシュ・Worker の経由(R2 の 4 MiB のオブジェクトを 1 往復で取る費用)は未測定。(e) brotli q9 を q11 にしたときの保存量(3〜5% 小さい見込み〔未測定〕)と生成時間(q11 は q9 の 20〜50 倍遅い見込み)。(f) chunk の SHA-256 が復号後なので、`identity` と `brotli` の流れに同じ論理 bytes が出ると名前が衝突する(生成器は検出して止まる。今回は 0 件)。

## 8. Wine の run 待ち行列の計画書: 内蔵プラグインを cumulative に加える参照 Export(PM の宿題、07:4x〜07:55。計画の時点では Wine は起動していない。**実行の結果は §8.6〜8.7**)

**目的**: lift の進み(G9/G10)より先に、実機(Wine 上の原版 FL。入力 PCM と CPUID を揃えた構成)の参照 Export を揃えて、判定を待たせない。**実際に実行した確認(Wine なし)**: (a) prefix-e2 と原本の prefix の中身をファイルの一覧(種別・大きさ・mtime)で比べた、(b) 残り 7 フォルダ(10 個の DLL)の cpuid/xgetbv の位置を alpha1 の `a22-cpuid-sites.py` で全部抽出した(`tables/0320-queue-plugins-cpuid-sites.tsv`)、(c) **prefix-e3 を作り**(下)、そこで第 1 段(Delay 2)を `prep` → `undo` して、前後の一覧が一致し、prefix-e2 と原本の prefix が無傷なことを確かめた。

### 8.1 構成の計画(prefix-e3 で順に足す。Problems 窓は『隠れている内蔵プラグインの種類の数』で、2 つの `Fruity Wrapper` は 1 と数える)

| run | tag(案) | prefix | 見えるようにする物(改名する `.off-0307`) | cpuid を UD2 にする DLL(cpuid / xgetbv の箇所) | extra-mods に足す名前 | Problems 窓の見込み |
|---|---|---|---|---|---|---|
| 済 | elog14 / elog15b | prefix-e2 | なし(Balance は elog15b で追加) | Balance 5/1 | `fruity balance_x64.dll` | 8 → 7 |
| **R2** | **elog16** | prefix-e2(構成 A、準備済み) | Fruity Filter(Effects) | `Fruity Filter_x64.dll` 5/1 | `fruity filter_x64.dll` | 6 |
| R3 | elog17 | prefix-e3 | Fruity Delay 2(Effects) | `Fruity Delay 2_x64.dll` 5/1 | `fruity delay 2_x64.dll` | 5 |
| R4 | elog18 | prefix-e3 | Fruity Parametric EQ 2(Effects) | `Fruity Parametric EQ 2_x64.dll` 5/1 | `fruity parametric eq 2_x64.dll` | 4 |
| R5 | elog19 | prefix-e3 | Frequency Shifter(Effects) | `Frequency Shifter_x64.dll` 5/1、`Engine_x64.dll` 4/2 | `frequency shifter_x64.dll`、`engine_x64.dll` | 3 |
| R6 | elog20 | prefix-e3 | Soundgoodizer(Effects)+ FLEX(Generators) | `Soundgoodizer_x64.dll` 5/1、`FLEX_x64.dll` 5/1、`engine_x64.dll` 48/5、`engineV2_x64.dll` 47/4 | `soundgoodizer_x64.dll`、`flex_x64.dll`、`enginev2_x64.dll`(`engine_x64.dll` は R5 で既に) | 1 |
| R7 | elog21(完全版) | prefix-e3 | Fruity Wrapper(Effects と Generators の両方) | `Fruity Wrapper_x64.dll` 6/1(Effects)、6/1(Generators) | `fruity wrapper_x64.dll` | 0(全部有効 = 原本と同じ。最初の run で使った `problems.png` の窓の形になる見込み) |

- cpuid の箇所の合計は、R3〜R7 で追加 DLL 10 個(`Fruity Wrapper` は 2 つのファイル)、cpuid 約 130 箇所(`engine_x64.dll` 48 + `engineV2_x64.dll` 47 が大半)。全部 verified。`engine_x64.dll` は Frequency Shifter の `Engine_x64.dll`(4/2、別ファイル)と FLEX の `engine_x64.dll`(48/5、別ファイル)の 2 つが同じ名前だが、shim は UD2 を踏んだ module の名前だけで判断し、書き換えていない同名の DLL では UD2 が起きないので問題ない(`Frequency Splitter` の `engine_x64.dll` は見えているが未 patch のまま、実 CPUID で動く)。
- **各 run の比較**: Wine の新しい WAV と、(a) 前の段の WAV(= その段で足したプラグインだけの効果の大きさ。`tools/step-effect.py`。Balance は RMS 3,986.6)、(b) その時点の lift を含む wasm の Export(PM/ci1 が取る)。入力は mp3wav-all(同一 PCM)、CPUID は SSE2、フック付き(a97930x の h2 が wasm と一致するか、Init の h が一致するかも同じ run で見る)。

### 8.2 prefix-e3 を作る方法(複製の大きさの問題への答え)

- prefix-e2 は 4.2 GB(`du`。`drive_c/windows` 1.2 GB、`Program Files (x86)` 868 MB〔WebView2〕、`Program Files/Image-Line` 2.1 GB)。ext4 なので reflink は使えない。**全部を複製(`cp -a`)すると 4.2 GB・約 1 分の I/O**。
- **提案(採用して作成済み): hard link の複製 + 書かれる物だけ実体の複製**(`tools/mk-prefix-e3.sh hybrid`)。根拠(実測): prefix-e2 を作った後の約 20 回の run で変わったファイルは **81 個だけ**(`find -newer`): 3 つの `.reg`(system.reg 3.7 MB ほか)、`drive_c/users/ubuntu/AppData`(WebView2 のキャッシュ ほか 73 個)、`drive_c/ProgramData`(1)、FL のフォルダの中は私が書き換えた `wintrust.dll` と patch した DLL 2 つだけ。したがって `cp -al`(hard link、データを写さない、0.02 秒)で作り、`users`・`ProgramData`・`dosdevices`・3 つの `.reg`・`.update-timestamp`・FL のフォルダの `wintrust.dll`/`.stub`/`wintrust_real.dll` だけを実体で複製(合計 約 17 MB)。**patch する DLL は `cp --remove-destination` で新しい inode にしてから書く**(`g9-prep.sh`)ので、prefix-e2 や原本の DLL には届かない(テストで、e3 側の inode が別になり、e2 側と原本の DLL の sha が不変なことを確認)。
- **作成の結果(07:48)**: ディスクの増加は約 17 MB(空き 584 → 583 GB)、prefix-e3 と prefix-e2 のファイル一覧(種別・大きさ・mtime)の差 0、書き込み対象の 225 ファイルは link 数 1(別 inode)、残り 30,902 ファイルは hard link。`tools/guard.sh`(prefix と prefix-e2 の一覧。link 数を除いて比較)で、複製の前後に**原本の prefix と prefix-e2 に変化なし**を確認。
- **リスクと守り**: (1) hard link の共有ファイルが e3 の run で書き換えられると e2/原本も変わる。守り = run の前後に `guard.sh snap/diff` を自動実行(`queue-run.sh`)し、サイズか mtime が変わったら止まる。(2) 上の『書かれる物』の一覧が漏れていた場合も、同じ守りで検出される。(3) 心配なら `mk-prefix-e3.sh full`(実体の複製、4.2 GB)に切り替えられる。

### 8.3 1 run の所要時間の実績(`exports/runs/*.events.txt`)と見積もり

| run | 起動〜停止 | 内訳 |
|---|---|---|
| hide1・hideT1・e2b・elog8・elog13・elog14(窓を認識) | 87.1・97.6・99.1・94.0・92.6・106.9 s(中央値 約 97 s) | Problems 窓 31.6〜39.5 s、Start まで +19〜21 s、Export 27.0〜41.0 s |
| elog12(読み込みが遅い日) | 165.8 s | Problems 窓 100.2 s |
| elog15b(窓を認識できず fallback) | 221.2 s | fallback まで 150 s の待ち、Start まで +23 s、Export 36.0 s |

- **新しい窓の件数の run の見積もり**: 参照が無い件数(R3〜R6 は 5・4・3・1)では窓を認識できない。**待ちを短くする変更を入れた(未テスト)**: driver が hook のログ(`e3/elog.txt.<pid>`)の `ELOG a97930x`(project の読み込みの終わり)を見て、**その 15 秒後に参照に合わなければ fallback(Return)**にする(`OPT1_PROB_ELOG_DIR`、`tools/wine-export-run.py`、`elog-run-pfx.sh` が自動で設定)。pad の終わりから Problems 窓までは elog14 で約 8 s なので、15 s は余裕がある(読み込みが極端に遅い日に窓が 15 s 以上遅れる場合だけ、窓が出る前に Return を押す恐れ〔その場合は FL のメインに Return が入るだけで害は未確認〕)。見積もり: 読み込み 30〜45 s + 15 s + Start まで +23 s + Export 27〜41 s + 停止 5 s = **約 2〜2.5 分**(遅い日は 3 分)。ELOG の変更が効かなければ 150 s の待ちで **約 3.7 分**。
- **1 段あたりの前後作業**(prep〔rename + cpuid の位置の抽出 + patch〕+ 一覧 4 回 + guard 2 回): 約 1 分(Wine なし)。R3〜R7 の 5 段で **Wine の時間 約 10〜12 分 + 前後 約 5 分 = 約 15〜17 分**(ELOG の変更が効かないと約 23 分)。R2(elog16)は別に 約 2.5〜4 分。

### 8.4 窓の要求と段取り

- **要求**: Chrome・Firefox が 0 本、重い job(build、node の Export)が 0 本、MemAvailable ≥ 8 GB、swap の入出力 0、load ≤ 3(`queue-run.sh` が自動で確認して止まる)。窓の間 Wine は 1 本ずつ(直列)。
- **分け方の案**: 窓 1 = R2(elog16)のみ(約 4 分、PM が合図済みの順序)。窓 2 = R3〜R4(約 7 分)、窓 3 = R5〜R7(約 12 分)。1 本の連続した 20 分の窓があれば R3〜R7 を続けて実行できる。各段は `tools/queue-run.sh STEP TAG`(STEP = 3〜7)で独立に実行でき、途中で止まっても次の段は prefix-e3 の今の状態から続けられる(`prep` は冪等でないので、止まったら `e2-state.sh` の一覧で状態を見てから)。
- **注意(事実)**: 07:47 に、私の起動していない Wine(`wineserver` と `winedevice.exe` 2 つ、約 1 分で消えた)が動いているのを私の `ps` の確認で見た。誰の物かは不明(私は 05:51 以後 Wine を起動していない)。Wine は私だけという規則の確認を。

### 8.5 準備済みの物(Wine なし)

- `tools/mk-prefix-e3.sh`(実行済み、prefix-e3 あり)、`env-e3.sh`、`tools/g9-prep.sh`(複数の DLL に対応、`PFX=prefix-e3`、patch の失敗で止まる、`prep`/`undo` の往復を e3 でテスト済み: 一覧とシャが元に戻る)、`tools/e2-patch-one.py`(prefix-e3 も許す)、`tools/e2-state.sh`(全プラグインの DLL の sha を含む。`PFX` 対応)、`tools/guard.sh`、`tools/elog-run-pfx.sh`(prefix 汎用の ELOG run。ELOG 起点の fallback つき)、`tools/queue-run.sh STEP TAG [prep-only]`(段ごとの前後の確認 + 実行 + 後の確認)、`tools/step-effect.py`(段の効果の大きさ)。`elog16-run.sh` も `elog-run-pfx.sh` に切り替えた(prefix-e2 の構成 A のまま)。
- **未実行・未確認**: 各段の Wine の実行そのもの、ELOG 起点の fallback の動作、新しい窓の件数での Return の効き方(7 件では確認済み)、FLEX(約 5 MB の DLL 3 個)・Fruity Wrapper を戻したときの project の読み込み時間、全部有効の完全版の Problems 窓の形。

### 8.6 08:00 の窓の実行結果(実測。11/07 07:59〜08:35)

**実際に実行した物**: Wine の Export を **12 本**(下の表)。すべて mp3wav-all(同じ PCM)・CPUID は SSE2・ELOG hook あり。原本の prefix は一度も起動していない。窓は 07:59:56(FL64.exe の最初の起動)から 08:34:31(R7 の終了)で、Wine は 08:35 に 0 本。道具は `tools/busy-check.sh`(新設: Chrome/Firefox/FL64/wineserver の確認。cwd が `/usr/src/app` の chrome は除外)、`elog16-run.sh`・`queue-run.sh`・`elog16-variant-run.sh`(新設: 変種 FLP を差し替えて config A で 1 本)、`queue-chain.sh`(新設)、`run-record.py`・`window-table.py`・`band-compare.py`・`lag-compare.py`(新設: WAV の事実)、`modload-cpuid-check.py`(新設: 各 run で読み込まれた FL 側の DLL に未 patch の cpuid が残っていないか)、`e3-fsplit-patch.sh`・`e3-patch-file.sh`(新設: prefix-e3 だけで依存 DLL を UD2 化)。

| タグ | 構成(prefix) | 実時間(FL64 起動〜WAV 確定) | フレーム | RMS | 基準 | 基準との差の RMS(max\|d\|、差のあるサンプル) |
|---|---|---|---|---|---|---|
| elog16 | config A: +Balance +Filter(e2) | 07:59:56〜08:02:32 | 5,715,911 | 20412.1782 | wasm G9-a | **0.0004**(1、2 サンプル)。elog15b との差 0.0005(1、3 サンプル) |
| elog17(R3) | +Delay 2(e3) | 08:03:11〜08:05:08 | 5,715,911 | 20433.0423 | elog16 | 1040.5284(15009、7,452,097) |
| elog18(R4・**clean でない**) | +Parametric EQ 2 | 08:05:41〜08:07:45 | 5,716,049(+138) | 20409.4673 | elog17 | 8058.7842(65534、8,128,428) |
| elog19(R5・**clean でない**) | +Frequency Shifter | 08:08:36〜08:10:50 | 5,728,590(+12,679) | 20453.5782 | elog18 | 21867.6854(65534、10,107,847) |
| **elog22(R4・clean)** | +Parametric EQ 2、`Frequency Splitter\engine_x64.dll` も UD2 | 08:12:01〜08:14:07 | 5,716,049 | 20409.4673 | elog17 / elog18 | 8058.7842(65534、8,128,427)/ 0.0069(1、547 サンプル) |
| **elog23(R5・clean)** | +Frequency Shifter | 08:14:31〜08:16:41 | 5,728,590 | 20453.5719 | elog22 / elog19 | 21867.6813(65534、10,107,931)/ 27.5303(339、8,249,811) |
| elog20(R6) | +Soundgoodizer +FLEX(依存 DLL 5 個も UD2) | 08:22:48〜08:25:14 | 5,728,866 | 21042.0019 | elog23 | 8913.5922(65534、8,762,809)。Problems 窓は『2 plugin(s)』(plan の期待は 1。**理由は未確認**) |
| elog21(R7・完全版) | +Fruity Wrapper(Effects・Generators) | 08:31:36〜08:34:22 | 5,728,866 | 21042.0228 | elog20 | 29.2794(380、8,153,420)。Problems 窓は、原本の全部入りの参照画像(`problems.png`)に**画素差 0.00 で一致** |
| elog16-flp-lp | config A + alpha1 の filter-lp.flp | 08:17:07〜08:19:13 | 5,715,911 | 20412.1782 | elog16 | 0.0005(1、3 サンプル)= **帯 45.3〜81.6 s の差 0.0000、効果なし** |
| elog16-flp-bp | config A + filter-bp.flp | 08:19:51〜08:22:01 | 5,715,911 | 20412.1782 | elog16 | 0.0006(1、4 サンプル)= **帯の差 0.0000、効果なし** |
| elog16-ins1-lp | config A + alpha1 の ins1-lp.flp(Insert 1 に Filter を追加) | 08:25:42〜08:28:19 | **5,729,968(+14,057)** | 18649.4690 | elog16 | 12697.6647(65534、8,890,025)。帯 45.3〜81.6 s の差の RMS 10793.92 = **効果あり** |
| elog16-solo12 | config A + solo12-wine.flp(§11) | 08:28:52〜08:31:08 | 5,715,911 | 16256.2513 | elog16 | 13684.1705(65534、9,489,434)= **Solo の効果は WAV に大きく出る** |

(sha256 の頭 16 桁: elog16 f102311b4db54e57、elog17 f33c9c4880146ce8、elog18 5ba9029e41f3df61、elog19 09cc21650682243b、elog22 3b81a9de298a001f、elog23 46cc2de1ae30df24、elog20 2e8f318973c2f4e4、elog21 5ec9d68f3ad883bb、flp-lp 4448173d189a5abb、flp-bp 7d9b8629238b4850、ins1-lp f1c1012d5a21a6c2、solo12 9172ab0ad9224608。全部 `exports/runs/TAG.wav`、フックのログは `exports/elog/TAG.*`。)

**事実の読み取り**
1. **Wine の cumulative な基準**: R3(Delay 2)・R4(Parametric EQ 2)・R5(Frequency Shifter)・R6(Soundgoodizer + FLEX)・R7(完全版)の clean な WAV が揃った(elog17・elog22・elog23・elog20・elog21)。wasm の側の同じ構成との差が lift の検証の対象。段ごとの効果の大きさ(前の段との差の RMS): Delay 2 1040.5、Parametric EQ 2 8058.8(長さ +138 フレーム)、Frequency Shifter 21867.7(長さ +12,679 フレーム。相関が低い = 周波数が動く効果)、Soundgoodizer + FLEX 8913.6(長さ +276 フレーム)、Fruity Wrapper 29.3。ラグ補正(`lag-compare.py`)ではラグ 0(R4・R6・R7)で、差は位置のずれでなく信号の差。
2. **Fruity Filter の効果**: config A の project では、Filter を足しても(elog16)、alpha1 の filter-lp・filter-bp(静的設定 3/2 バイトの変更)でも、Export に出ない(帯の差 0)。Insert 1 に Filter を足した ins1-lp は強く出る(RMS 12697.66。長さも +14,057 フレーム)。
3. **見落とし(自己申告)**: `Fruity Parametric EQ 2` は、隠していない `Effects\Frequency Splitter\engine_x64.dll`(Intel IPP、cpuid 46 + xgetbv 4)を読み込む(elog18 の modload で判明)。私の g9-prep は PEQ2 本体だけを patch していたので、elog18・elog19 は『実 CPU の cpuid』が混じった run だった。影響の大きさ: R4 では 0.0069(547 サンプル ±1 LSB)、R5 では 27.53(0.13%)。以後、依存 DLL を静的に調べて(Soundgoodizer・FLEX・Wrapper の文字列から AudioRestore.dll・iZAudioRestore.dll・DirectWaveCtEngine・EasyFLAC・ILVideo を事前に UD2 化)、**run ごとに `modload-cpuid-check.py` で点検**(結果: elog17・22・23・20・21 とも、`WebView2Loader.dll` の 4 箇所〔全 run 共通、音の経路でない〕以外は残り 0)。
4. **ELOG 起点の fallback は動いた**(全 run で『project の読み込み終了』から 17 秒後に Return。Problems 窓は 6・5・(4)・3・2・R7 は原本全部入りの参照と一致)。Ok ボタンの y は件数ごとに 8 px 違う(8 件 683、7 件 675、6 件 667、5 件 659、3 件 655〔elog23〕、2 件 655〔elog20〕。4 件以下は同じ位置に収まると読めるが、4 件は画像を見ていない)。
5. 完全版(elog21)を、昔の全部入りの Wine run(`wineA`、実 CPUID・Wine の mp3 デコード)と比べると、フレーム数は同じ(5,728,866)で RMS 差 6826.4。mp3 の差(§2k〜§2q)と CPUID の差が重なった値と読めるが、**この値は分析していない**。

**guard(原本・e2・e3 の前後の一覧、窓の前〔07:57:51〕と後〔08:35:35〕)**: 原本 prefix は差 0(30,909 行)。prefix-e2 の差は、wintrust.dll の mtime(run 後に戻した shim の書き込み。sha は fe4d813e13ec2fc7 で窓の前と同じ)と `system.reg`・`user.reg`・`userdef.reg` の 3 つだけ(Wine が書く物)。**prefix-e3 は窓の終わりに rehide 済み**: 段の prep(Delay 2・Parametric EQ 2・Frequency Shifter・Soundgoodizer・FLEX・Fruity Wrapper の 2 フォルダ)を `g9-prep.sh undo` で戻し、依存 DLL 6 個を原本から戻した。結果、e3 の一覧(状態)の sha は窓の前(elog17 の前)と**同一**(`state-before-elog17.txt` = `state-window-end-e3.txt`、listing sha b07cf6d2265ece73)、`Frequency Splitter\engine_x64.dll` の sha は e2・e3・原本で 193848aab512232c。proj の FLP は d6783e08886b4b48(`variants/original.flp`)、プロジェクト用の WAV の残りは 0、FL64/wineserver は 0。

**自己申告(この窓)**: (1) 窓の開始より前に Wine を起動した(FL64.exe 07:59:56、開始は 08:00)。(2) 07:55 に存在しない `bash` ツール名を 1 回呼んだ。(3) 依存 DLL(Frequency Splitter の engine)の見落とし(上の 3)。elog18・elog19 は clean でない run だったので、段の基準には使わない。(4) `queue-chain.sh` の WAV 長さの確認が厳しすぎて R4 で止まった(R4 は +138 フレームが正しい長さ)。(5) 08:10 に連鎖の停止のため `pgrep -f` で PID を取り、harness の `bash -c` の wrapper に SIGTERM を送ってしまった(連鎖の本体は動き続け、次の呼び出しで正しい PID を止めた。R5 の run 自体は影響なし)。(6) PM の『filter-bp は走らせない』の info(08:20)の前の 08:19:47 に filter-bp を起動済みだった(窓の中、約 2.5 分の余分)。(7) 08:41 の Settings の確認で、`sleep 0` を含むコマンドを 1 回送って拒否された(実行されていない)。

### 8.7 Wine の FL の Settings(F10)の確認(alpha1 の質問、PM の指示。Wine の run 2 本、08:39〜08:49、Chrome 0 本の間)

**実際に実行した物**: `tools/settings-fl.sh`(prefix-e2・config A、project なし = `OPT1_NOPROJECT=1`、`nice 19`)で FL を起動し、`xt.py` で操作・`shot.sh` でスクリーンショット。1 本目(08:39〜08:44)は Settings を開いて見て `wineserver -k` で止めたため、FL の設定が user.reg に**書かれなかった**(FL は終了時に書く)。2 本目(08:45〜08:49)は FL を右上の × で**正常終了**させ、user.reg が書かれた。run 後に `prefix-e2/user.reg` を run 前の物(sha 743af7446d3532ea)へ戻し、e2 の状態(プラグインのフォルダ・DLL の sha)が config A のままであることを確認(`state-after-settings.txt` = `state-after-elog16-prep.txt`)。

- `HKCU\Software\Image-Line\FL Studio 26\General\MIDIForm`(76 行。全文 `exports/trace/settings2-MIDIForm.txt`): **`MultithreadFXBox="1"`、`MultithreadGenBox="1"`**、`SmartDisBox="1"`、`PriorityBox="2"`、`HardBufBox="1"`、`SafeOverloadBox="1"`、`AlignTickLengthBox="0"`、`ASIOMixInBSBox="0"`、`AsioTripleBufBox="0"`、`PBTrackingModeBox="2"`、`ThreadedSampleLoadBox="0"`、`DetachPlugsBox="0"`、`AutoSaveBox="2"`、`UndoCapacitySelect="100"` ほか。`Devices\Audio output`: `Device name="1FL Studio ASIO"`、`Sample rate="44100"`、`Buffer length (DS)="4096"`。user.reg の前後の差の全部は `exports/trace/settings2-user.reg.diff`(557 行。Edge・Wine のフォント一覧などの時刻の更新を含む)。
- スクリーンショット(`exports/runs/settings1-audio.png`): Settings→Audio の CPU 欄 = **Multithreaded generator processing 有効、Multithreaded mixer processing 有効、Smart disable 有効、Align tick lengths 無効**。出力 = 『FL Studio ASIO』44100 Hz、Float32LSB、1x2 出力・1x2 入力、バッファ 512 smp(12 ms)、遅延 68 ms、Priority Highest、Safe overloads 有効、Playback tracking = Mixer、『2 underruns』。Mixer 欄: Resampling quality = 24-point sinc。
- Settings→General(`settings1-general.png`): Main GUI scaling = System(96 PPI)、**Language = English (US) (en)**(『要再起動』、『Update languages』ボタンあり)、Note names = English。Debug タブ(`settings1-debug.png`)は白い空の欄(ログ表示)。
- **読み取り**: Wine の既定でも mixer・generator の multithreaded は**有効**。それでも §10 の表では Export の間に 0.2 コア以上のスレッドは最大 3 本、1 本が 0.96〜0.99 コアで、ミキサーが複数スレッドに割れている様子は出ていない(FL が見たプロセッサ数は未測定)。
- 起動時に出るダイアログ: usage data の同意(『I'm in』/『No thanks』— 1 本目で『No thanks』を押したため `LogStatisticsEnabled="-1"` が書かれた〔run 後に user.reg を戻したので現在の e2 には無い〕)、Welcome 画面(demo project の一覧)。Welcome 画面の × は 1 回では閉じず、押した操作で demo project が読み込まれた(『1 plugin(s) could not be opened』の Problems 窓が出た)。この run の結果(MIDIForm の値)には影響しない。

## 9. 日本語が『豆腐』になる件(O16)— 実測の計画書と、製品用フォント候補の棚卸し

**実際に実行した物**: Wine の起動は無し(この節の調べは全部、読み取りと静的解析)。実行したのは、(1) prefix の `drive_c/windows/Fonts` の一覧と `system.reg`・`user.reg` の読み、(2) `FLEngine_x64.dll`・`Shared/QuickFontCache_x64.dll`・`Shared/freetype_x64.dll` の import と文字列の読み(`objdump -p`・`strings`・バイト列の探索)、(3) reference project の FLP の日本語文字列の探索(FLP のイベントの再解析)、(4) `fc-match`・`fc-list`、(5) `tools/font-subset-sizes.py`(新設、fontTools でフォントを切り出して大きさを測る。OTF・WOFF2・brotli-11)。**FL(Wine)が日本語をどのフォントで描くかの実測は、まだしていない**(下の 9.3 が計画。Wine の窓が要る)。

### 9.1 分かったこと(事実と、読みの区別)

**(a) Wine の側の環境(読んだ値)**
- この prefix の `C:\Windows\Fonts` は**空**。フォントは Wine が host の fontconfig から見せている: `HKLM\Software\Microsoft\Windows NT\CurrentVersion\Fonts` に **531 項目**(`Z:\usr\share\fonts\opentype\noto\NotoSansCJK-*.ttc`〔Noto Sans CJK HK/JP/KR/SC/TC の各ウェイト〕、`Z:\usr\share\fonts\truetype\droid\DroidSansFallbackFull.ttf`、Liberation、DejaVu など)。`HKCU\Software\Wine\Fonts\External Fonts` も同じ内容。
- `HKCU\Software\Wine\Fonts\Codepages="932,932"`: この prefix の ANSI/OEM コードページが **932(日本語)**。(= 参照の Wine は日本語ロケールで動いている。製品の仮想 Windows のロケール・`GetACP` がどうなるかは別の決め。FLEngine は `GetACP` を import している。)
- `...\CurrentVersion\FontLink\SystemLink` は **Wine の既定**(`Lucida Sans Unicode`・`Tahoma`・`MS UI Gothic`・`Yu Gothic UI`・`Meiryo UI`・`MS Gothic` 等に、`MSGOTHIC.TTC`・`YUGOTHM.TTC`・`MEIRYO.TTC`・`MSJH.TTC`・`SEGOEUI.TTF` などのファイル名を並べた値)。これらのファイルは prefix に無い。`FontSubstitutes`: `MS Shell Dlg`=MS UI Gothic、`MS Shell Dlg 2`=Tahoma ほか。
- host の fontconfig では、`Segoe UI`・`Tahoma`・`Open Sans`・`Yu Gothic UI`・`Meiryo UI`・`MS UI Gothic` を `fc-match` すると**すべて Noto Sans CJK JP**に解決される(host の既定の sans が Noto Sans CJK JP のため)。`U+3077(ぷ)` を持つ host のフォントは Droid Sans Fallback と Noto Sans CJK 全部。= Wine の側では、日本語の字形は**host のフォントの偶然の既定**で出る可能性が高い(未確認)。

**(b) FL の側の構造(静的解析。『読み』)**
- `FLEngine_x64.dll` の gdi32 import に `CreateFontIndirectW`・`CreateFontW`・`EnumFontsW`・`EnumFontFamiliesExW`・`TextOutW`・`ExtTextOutW`・`GetTextExtentPoint32W`・`GetTextExtentPointW`・`GetTextMetricsW`(`GetFontData`・`GetGlyphIndices`・`GetCharacterPlacement` は無い。DirectWrite・GDI+・Uniscribe の import も無い)。
- FL は**自前の文字描画を持つ**: `Shared\QuickFontCache_x64.dll`(2.07 MB、Delphi、AGG + FreeType。`AggFontFreetype`・`FontFileContent`・`LoadFont`・`GetFontSignature`・`CalculateGlyphIdx`・`GlyphIndex`、export は `FontCache_AcquireFont`・`FontCache_CleanUp`・`FontCache_GetStats`)と `Shared\freetype_x64.dll`(607 KB、本物の FreeType。`FT_New_Face`・`FT_New_Memory_Face`・`FT_Load_Glyph`・`FT_Get_Char_Index` を export、cff・truetype・sfnt・psnames のモジュールを含む)。**実 run の hook の modload ログで、この 2 本が読み込まれている**(elog16 ほか)。QuickFontCache は gdi32 の `CreateFontIndirectW`・`GetTextMetricsW`・`GetTextExtentPoint32W`・`ExtTextOutW` も import している = **GDI の経路も併せ持つ**。
- FLEngine のフォントの『役割表』(file offset 0x26bf9c〜0x26c340。文字列の並び): `P_SmallBold`=`segoeui`(.ttf)、`P_Small`・`P_Tiny`・`P_Tiny2`・`P_Normal`=`OpenSans-Regular`・`tahoma`、`P_NormalLight`=`segoeuisl`(.ttf)、`P_NormalBold`・`P_Large`・`P_LargeBold`・`P_TitleBar`・`P_LargeTitleBar`・`P_DigitWheel`(`OpenSans-CondLight`)、`P_Monospaced`=`consola`・`lucon`、`P_ILGlyphs`=`ILGlyphsEx.ilfont`、`P_WebSymbols`=`WebSymbols-Regular.otf`、`P_DescriptionLarge`=`PT_Sans-Web-Regular.ttf`。(名前と役割の組の順序は未確認。)`segoeui`・`tahoma`・`consola`・`lucon` は Windows の `Fonts` から探す名前、`OpenSans-*`・`PT_Sans`・`WebSymbols` は install の `Shared\Artwork\Fonts`(seed の 11 ファイル、1.6 MiB)に実体がある。**CJK を持つフォントの名前は役割表に無い**。ヘルプ文に『`Image-Line\Shared\Artwork\Fonts` の中のフォント、またはシステムフォント』とある。
- VCL(Delphi)のフォームは DFM に `Font.Name = Segoe UI`/`Tahoma`、`Font.Charset = DEFAULT_CHARSET` を持ち、起動時に `...\CurrentVersion\FontSubstitutes` を読む(Delphi の標準)。
- 言語: `System\Languages\ja.moe`(941,838 B、圧縮された形式で中の文字は静的に読めない)と `ja.svg`(旗)。Settings→General に **Language(English (US) (en))と『Update languages』**があり、『要再起動』(Wine の run で見た。`settings1-general.png`)。
- reference project の日本語の文字列(FLP の再解析): イベント 203 の `Balance - Insert 1 - ボリューム`、イベント 203・204 の `[fadr.com] Vocals - Still_in_my_heart_feat_ぷにぷに電…`(チャンネル/サンプル名)。他の文字列に日本語は無い。

**(c) 読みの結論と、未確認の点**: FL の文字は、(1) 自前の FreeType(QuickFontCache、フォントファイルの中身を読む)か、(2) GDI(`ExtTextOutW` など。VCL の部品と、QuickFontCache の GDI の経路)のどちらかで描かれる。日本語は(1)の役割表のどのフォントにも無いはずなので、**どこかで(2)の GDI の fallback(FontLink・フォントマッパー)に落ちる**と読めるが、**確認していない**。(2)に落ちる場合、native の Windows では GDI が `FontLink\SystemLink` と日本語ロケールの既定フォント(Yu Gothic UI / Meiryo UI / MS UI Gothic)で字形を出す。製品の仮想 Windows では、その GDI の挙動(契約)を実装した側が決める。

### 9.2 製品用フォントの候補(棚卸し。大きさは `tools/font-subset-sizes.py` の実測)

native の Windows の既定(Yu Gothic UI・Meiryo UI・MS UI Gothic)は**再配布できない**ので使わない。この machine には、`fonts-noto-cjk` と `fonts-droid-fallback` がある。ライセンス: Noto Sans CJK = SIL OFL 1.1(**フォント自身の name table の著作権表示は `(c) 2014-2021 Adobe`、`Noto is a trademark of Google Inc.`、Reserved Font Name の宣言なし**。上流 `notofonts/noto-cjk` の `Sans/LICENSE` も OFL 1.1 の全文で RFN の指定なし。Debian の copyright ファイルの『Copyright 2010–2012 Google』は Debian の包の一般的な記述で、このフォントの表示でないので、当初ここに書いた物を訂正した)、Droid Sans Fallback = Apache-2.0(Debian の copyright ファイル)。

| 候補 | ライセンス | 全体の大きさ | 日本語の字形・範囲 | 備考 |
|---|---|---|---|---|
| Noto Sans CJK JP(Regular/Bold 等 7 ウェイト、`NotoSansCJK-*.ttc`) | **SIL OFL 1.1**(再配布・サブセット化・埋め込み可。(c) 2014-2021 Adobe。**Reserved Font Name の宣言は無い**ので名前を変える必要はない。条件 2〔著作権表示と全文を各コピーに添える〕は §12 で扱う) | Regular の TTC = **19,484,784 B**(JP・KR・SC・TC・HK を 1 ファイルに共有。CFF/OTF)、Bold = 20,050,760 B | 日本語の字形(JP の既定)。cmap 44,810 点。JIS X 0208 の全部 7,203 点のうち 99.6% を持つ(残りは私の切り出し集合に含めた未割当の符号) | 本命 |
| Droid Sans Fallback Full | Apache-2.0 | 4,033,420 B(TrueType) | 漢字は中国語寄りの字形。cmap 28,601 点。JIS X 0208 の 93.5% | 小さいが、かな・記号・ASCII の範囲が不完全(S1 の 45.2%)で、日本語向けの字形でない |
| Noto Sans JP(Google Fonts の可変フォント。この machine には無い) | SIL OFL 1.1 | 未測定(公開物は約 5〜6 MB 級の WOFF2 だが、**この machine で確認していない**) | 日本語専用 | 必要なら取得して測る |
| M+ / Sawarabi / BIZ UDGothic など | OFL(各配布元で確認が要る) | 未測定 | 日本語 | この machine に無く、未確認 |

**サブセット化の実測**(`fontTools` の `pyftsubset` 相当、hinting 無し、layout feature 無し、`.notdef` 保持。切り出し集合は S1 = ASCII + NBSP + CJK 記号・約物 U+3000–30FF〔かな全部を含む〕+ 全角形 U+FF01–FFEE + JIS X 0208 の第 1〜8 区の記号・ギリシャ・キリル = 848 点、S2 = S1 + JIS 第一水準漢字 2,965 字 = 3,813 点、S3 = S2 + JIS 第二水準漢字 3,390 字 = 7,203 点):

| フォント | 集合 | グリフ数 | OTF/TTF | WOFF2 | OTF を brotli-11 |
|---|---|---|---|---|---|
| Noto Sans CJK JP Regular | S1 かな・記号・ASCII | 816 | 69,484 B | **53,740 B** | 53,794 B |
| | S2 + 第一水準 | 3,781 | 710,604 B | **538,872 B** | 538,347 B |
| | S3 + 第二水準(JIS X 0208 全部) | 7,171 | 1,515,892 B | **1,135,112 B** | 1,134,508 B |
| Noto Sans CJK JP Bold | S1 | 816 | 70,288 B | 53,796 B | 53,945 B |
| | S2 | 3,781 | 711,004 B | 545,992 B | 545,244 B |
| | S3 | 7,171 | 1,519,148 B | 1,154,076 B | 1,153,867 B |
| Droid Sans Fallback Full | S1 | 386 | 32,052 B | 14,468 B | 18,481 B |
| | S2 | 6,250 | 538,176 B | 218,696 B | 250,799 B |
| | S3 | 12,434 | 1,075,492 B | 421,264 B | 476,627 B |

読み取り: (1) **かな + 記号 + ASCII(S1)なら Noto Sans CJK JP Regular で 54 KB(WOFF2)**。FL の日本語 UI の文字(ja.moe は中身が読めないので未確認だが、メニュー・ダイアログの日本語)の大半はかな + 基本漢字。(2) JIS 第一水準まで(S2)で約 0.53 MB、JIS X 0208 の全部(S3)で約 1.1 MB(WOFF2)。(3) 全体の TTC(19.5 MB)をそのまま配るのは重い。(4) **ja.moe の中身(実際に使う文字)を実測すれば、S2 か S3 のどちらが要るか決まる**: ja.moe は圧縮形式で静的には読めないので、Wine で言語を日本語にした画面(9.3 の J2)の文字を OCR 無しで列挙する手段は無く、FL が `ExtTextOutW` に渡す文字列(Wine の `+text` ログ)で集める。(5) 上の切り出しは JIS X 0208 のみで、project の中のファイル名・サンプル名(例: `ぷにぷに電…`)は JIS X 0208 の外の字(旧字体・絵文字・環境依存文字)も入り得る。それは**別の fallback(全部入りの TTC を使う、または未収録は豆腐のまま)**になる。

**どこに置くか(遅延 VFS の層)**: 起動に取得を足さない = フォントは**日本語の文字が要った時にだけ取得**する。0313 の `FLVFSIX1` の規格に合わせるなら、仮想 `C:\Windows\Fonts` に 1 ファイル(例: 切り出し済みの Noto Sans CJK JP Regular、チャンクは SHA-256 名の不変オブジェクト)を置き、**その中身は GDI がグリフを要求した最初の時にだけ**読む。注意: Windows/Wine の GDI は `Fonts` ディレクトリの全ファイルを起動時に開いて名前を読む実装が多い。仮想 Windows の GDI は、フォントの**名前・文字集合の表を静的な目録(レジストリの値と同じ内容)から答え、ファイルの中身は触らない**契約にする必要がある(`EnumFontFamiliesExW` は FLEngine が import している。起動時に呼ぶかは file trace で確認)。`QuickFontCache` が『フォントのファイル名で `FontFileContent` を読む』経路を日本語で使う場合は、その名前の読み出しが取得を起こす = 取得は『日本語が要った時』になるので問題ない。

### 9.3 実測の計画(Wine の窓が要る。Chrome 0 本の時だけ、PM の合図で)

目的: (c) FL(Wine)が日本語の文字を **どの API で、どのフォントで**描いているかを、実際の呼び出しとファイルの open で確かめる。

**準備(Wine なし、済みか短時間)**: (P1) 日本語の名前のファイルを 1 つ、install の外に置く(例: `proj/テスト/ぷにぷに.wav` のコピー。FL の Browser でそのフォルダを開く)。(P2) `trace-fl.sh` に『フォント関連の open だけ』のフィルタを足す(`.ttf|.ttc|.otf|.ilfont|.fon|fontconfig|/usr/share/fonts`)。(P3) `WINEDEBUG` の組み合わせ: `+font`(win32u の `CreateFontIndirectW`・フォント選択・fallback の選択の TRACE)と `+text`(`ExtTextOutW`・`GetTextExtentPoint` の文字列と結果)。出力は大きいので、日本語を出す段だけ(約 20 秒)に絞る(起動後に `WINEDEBUG` を変えられないため、段ごとに別の run にする、または run 全体で取って後で切る。ファイルは `~/work/opt1-wine/exports/trace/` に、/tmp には置かない)。

**手順(各 run 約 2〜3 分、config A、Chrome 0 本)**:
- **J0(英語 UI・日本語を含む project)**: `strace -f -e trace=openat,open,stat,newfstatat,readlink` + `WINEDEBUG=+font,+text` で FL を起動 → reference project(`mp3wav-all` ではなく原本の FLP でよい)を開く → Channel rack でチャンネル `[fadr.com] Vocals - Still_in_my_heart_feat_ぷにぷに電…` の行を出す(スクロール)→ Playlist でそのクリップの名前を出す → スクリーンショット。取る物: (i) 日本語の字形が出ている/豆腐の画像、(ii) `+text` ログの `ExtTextOutW` の文字列に `ぷ` を含む呼び出しと、その時の font(face 名・charset)、(iii) `+font` ログの fallback の選択(どの face にどう決まったか)、(iv) strace の open: `NotoSansCJK-*.ttc`・`DroidSansFallbackFull.ttf` を FL(Wine)が開いたか、`QuickFontCache` が FreeType で開いたフォントファイル(`Shared/Artwork/Fonts/*.ttf` か、`Z:\usr\share\fonts\...`)の一覧。
- **J1(Browser の日本語名)**: P1 のフォルダを Browser で開く。(ii)〜(iv) を同様に。
- **J2(言語を日本語に)**: Settings(F10)→ General → Language を『日本語(ja)』にして閉じる → FL を終了(user.reg を書かせる)→ 再起動 → メニュー(FILE、OPTIONS)・Settings を開く → スクリーンショット + (ii)〜(iv)。言語を変えた user.reg は run 後に元へ戻す(復元と sha の確認)。**ja.moe が使う文字の集合**もここで `ExtTextOutW` の文字列から集める(S2 か S3 かを決める根拠)。
- **J4(契約の検証。§12.4)**: prefix-e3 の `drive_c\windows\Fonts` に `msgothic.ttc` を置く/置かない、の 2 本。さらに P0(起動〜idle)の file trace に msgothic.ttc を置いた状態を 1 本足し、FL が起動時にそのファイルを読むか(何バイトか)を見る(§12.3 注意 1)。
- **J3(対照: フォントを隠した場合)**: 同じ J0 を、host の CJK フォント(`/usr/share/fonts/opentype/noto`・`truetype/droid`)を fontconfig から見えなくした状態(`FONTCONFIG_FILE` を CJK を除いた設定に向ける。host のファイルには触れない)で行い、豆腐になるか、どの経路で豆腐になるかを見る。製品の仮想 Windows(日本語のフォントが無い)の症状と同じ形かの確認。
- 結果の表(各 run): 日本語の描画の有無、FL が `CreateFontIndirectW` に渡した face 名・charset・高さ、Wine が選んだ実フォント、開かれたフォントファイル、QuickFontCache(FreeType)と GDI のどちらを通ったか。

**成功の基準**: (1) 日本語の文字ごとに『呼び出された API』と『最終的に字形を出したフォントファイル』が 1 つ決まる。(2) 製品の側が実装すべき契約(GDI の fallback の規則、FontLink の値、`EnumFontFamiliesExW` が返す一覧、FreeType 経路が開くファイル名)が列挙できる。

### 9.4 (b) 外部 API の契約として製品が持つべき物(仮説。9.3 の実測で確定する)

1. 仮想 `C:\Windows\Fonts` と `HKLM\...\CurrentVersion\Fonts`: FL が名前で探す `segoeui.ttf`・`segoeuisl.ttf`・`tahoma.ttf`・`consola.ttf`・`lucon.ttf`(現状は Liberation/DejaVu などで代替されている、と alpha1 の読み)。日本語の字形を持つ OFL フォント(Noto Sans CJK JP のサブセット)を 1 つ登録する。
2. `FontLink\SystemLink`: FL が要求する face(`Segoe UI`・`Tahoma`・`Open Sans`・`MS Shell Dlg` の行き先 `MS UI Gothic`・`Yu Gothic UI` など)に、上のフォントを**文字単位の fallback の先**として登録する(native の Windows と同じ構造、中身は OFL のフォント)。
3. GDI の `CreateFontIndirectW`(charset = SHIFTJIS_CHARSET / DEFAULT_CHARSET の扱い)・`ExtTextOutW`・`GetTextExtentPoint32W`・`GetTextMetricsW`・`EnumFontFamiliesExW`: face が持たない字形のとき、SystemLink → 日本語の既定フォント、の順で fallback する契約。
4. `GetACP`/ロケール: 参照の Wine は 932。製品の既定 ACP をどうするか(Settings の Language の初期値と、`DEFAULT_CHARSET` の解釈に効く)。
5. 起動時に取得を足さない: 目録(名前・文字集合・metrics)は静的、ファイルは最初の glyph 要求で取得。
どの項目も『症状ごとの対症』でなく、GDI/レジストリの契約の不足(b)として入れる。FL の文字の判定・窓の種類による分岐は入れない。

### 9.5 未確認・依頼

- 9.1(c)の『GDI の fallback に落ちる』は読みで未確認。9.3 の J0〜J3 で確定する(Wine の窓、Chrome 0 本、PM の合図)。
- alpha1 への質問(info 済み): 仮想 Windows の `Fonts`・FontLink・FontSubstitutes の実際の中身、`CreateFontIndirectW`/`ExtTextOutW` の fallback の実装、QuickFontCache の GDI の経路の呼び出し元、`EnumFontFamiliesExW` の用途(起動時か設定画面か)。
- ja.moe の使う文字の集合(S2/S3 の決定)と、Noto Sans JP(可変フォント)の実測は未実施。

## 10. FL のミキサーは Wine で複数スレッドに仕事を割っているか(mt3 の質問・既存の `*.threads.tsv` からの表)

**実際に実行した物**: 新しい Wine の run は無し。既存の run が記録した 1 秒刻みのスレッド別 CPU(`exports/runs/TAG.threads.tsv` = FL64.exe の Wine プロセスの各スレッドの dutime+dstime〔1/100 秒の刻み〕を 1 秒ごと)と run の時刻(`TAG.events.txt`)から、`tools/thread-table.py TAG`(新設、読み取りだけ)で『Export の間』(`clicked Start` から `WAV stable`)を切り出した。再生は `hideP1`(config hidden-8 の再生 60 秒。`PHASE_FROM='key space (play)' PHASE_TO='key space (stop)'`)だけが既存。機械は 16 コア(`nproc` 16)。

### 10.1 表(Export の間、同じ project mp3wav-all、すべて 1 本の FL64.exe プロセス)

| run(構成) | Export の長さ(標本数) | プロセス全体の CPU(平均コア数) | 0.2 コア以上のスレッド数(最小/中央/最大) | 最上位のスレッド | 2 位 | 3 位 | 4〜8 位(1 本あたり) |
|---|---|---|---|---|---|---|---|
| elog13(hidden-8・mp3wav) | 37 s | 82.5 core-s(2.23) | 2 / 3 / 3 | 35.5 s(0.96) | 16.3 s(0.44) | 11.1 s(0.30) | 1.3 s(0.03) |
| elog14(Balance なし・SSE2) | 45 s | 101.7(2.26) | 2 / 3 / 3 | 43.3 s(0.96) | 20.4 s(0.45) | 13.3 s(0.30) | 1.6 s(0.04) |
| elog16(+Balance +Filter) | 42 s | 95.8(2.28) | 2 / 3 / 3 | 40.4 s(0.96) | 18.8 s(0.45) | 13.4 s(0.32) | 1.5 s(0.04) |
| elog17(R3 +Delay 2) | 43 s | 99.7(2.32) | 2 / 3 / 3 | 42.4 s(0.99) | 19.2 s(0.45) | 13.8 s(0.32) | 1.6 s(0.04) |
| elog22(R4 +Parametric EQ 2) | 53 s | 118.8(2.24) | 2 / 3 / 3 | 50.8 s(0.96) | 21.2 s(0.40) | 13.6 s(0.26) | 2.2 s(0.04) |
| elog23(R5 +Frequency Shifter) | 61 s | 139.3(2.28) | 2 / 3 / 3 | 60.0 s(0.98) | 24.1 s(0.39) | 17.3 s(0.28) | 2.5 s(0.04) |
| elog20(R6 +Soundgoodizer +FLEX) | 73 s | 164.9(2.26) | 2 / 3 / 3 | 71.6 s(0.98) | 25.1 s(0.34) | 17.6 s(0.24) | 3.3〜3.4 s(0.05) |
| elog21(R7 完全版) | 86 s | 190.7(2.22) | 2 / 3 / 3 | 83.9 s(0.98) | 24.8 s(0.29) | 18.0 s(0.21) | 4.2 s(0.05) |
| elog16-ins1-lp | 48 s | 112.7(2.35) | 2 / 3 / 3 | 46.4 s(0.97) | 23.2 s(0.48) | 15.3 s(0.32) | 1.8 s(0.04) |
| **再生** hideP1(hidden-8、60 s) | 60 s | 110.6(1.84) | 2 / 2 / 2 | 58.3 s(0.97) | 33.0 s(0.55) | 6.8 s(0.11) | 1.7 s 以下(0.03 以下) |

(括弧内 = その段階の実時間に対する割合 = 平均コア数。『最上位』は 1 本のスレッドの CPU 秒。スレッドは tid の数字だけで名前は記録していない。run 全体で見えたスレッドは 43〜57 本。)

### 10.2 読み取り(事実と限界)

- **事実 1**: Export の間、**1 本のスレッドが 0.96〜0.99 コア(ほぼ 100%)で走り続けている**(elog13〜elog21 の全部で同じ)。ミキサーを複数スレッドで割ったなら 2 位以下にも同程度の CPU が出るはずだが、2 位は 0.21〜0.48(main のスレッド = FL64.exe の pid と同じ tid の物が多い)、3 位は 0.21〜0.32。**0.2 コア以上を使うスレッドは最大 3 本(中央値 3、最小 2)で、0.5 コア以上は最大 2 本(中央値 1)**。残りの約 50 本は 1 本あたり平均 0.03〜0.05 コア(同じ値が揃っている = 一定周期の待ち・起床の類と読めるが、役割は未確認)。
- **事実 2**: 16 コアの機械で、プロセス全体は平均 2.2〜2.35 コア(約 14%)しか使っていない。段階(プラグインを足す)が増えても平均のコア数は変わらず(2.2〜2.35)、Export の長さだけが 37 s → 86 s と伸びる = **処理が増えた分は最上位の 1 本のスレッドの時間(35.5 s → 83.9 s)にそのまま乗っている**。
- **事実 3(再生)**: 既存の再生の run(hideP1)でも、1 本が 0.97 コア、2 本目が 0.55 コアで、0.2 コア以上は 2 本。Export より 1 本少ない。
- **読み取り(推測を含む。未確認)**: この Wine 環境で FL が混ぜる(Export の)処理は、**実質 1 本のスレッドに載っている**。FL の『Multithreaded mixer processing』の設定は**有効**だった(§8.7)が、FL が見るプロセッサ数(`GetSystemInfo`・`NUMBER_OF_PROCESSORS`・CPU のトポロジー)が何だったかは、この run のデータには無い(設定は §8.7 の Settings の run で確認した: multithreaded の mixer・generator は有効)。mt3/alpha1 の『B1 では FL は processors = 1 と見る』との整合は、FL が見たプロセッサ数を測っていないので未確認。
- **限界**: (1) Export は再生と別の経路(Export は実時間でなく可能な限り速く回る)。(2) 再生の run が 1 本だけで、hidden-8 の構成。再生の実時間の余裕は、この表からは分からない(音声出力は winepulse 経由で、1 コア 0.97 は『実時間の余裕が無い』ことを意味しない — Export と同様に、待たずに回しているだけの可能性がある)。(3) 他の job の負荷(load1 5〜13)が同時にあった(CPU 秒は走った時間だけの計上なので、待たされた時間は出ない)。elog16-solo12 では、Export の中ほど 25 秒ほど全体で 0.1〜0.3 コアまで下がった(load1 が 13 に上がった時間帯。原因は未確認で、この表には入れていない)。(4) スレッドの役割(ミキサー・UI・書き出し)は名前を記録しておらず、上の『ミキサーのスレッド』は CPU の振る舞いからの読み(最上位 = 重い処理)で、確定ではない。
- 同じ表を取り直すなら: `thrsample.py` に各スレッドの `comm`(`/proc/PID/task/TID/comm`)と `wchan` を足す、`NUMBER_OF_PROCESSORS` と FL の Settings→Audio の設定を同じ run で記録する、再生の run を project 付き・構成 A で 1 本足す。いずれも Wine の窓が要る。

## 11. solo12.flp — 『Track 1-5, 7, 13, 15-17, 19, 20 だけを Solo』の変種 FLP(PM の指示、O17)

**実際に実行した物**: (1) FLP の読み(`tools/make-solo12.py`、新設)と再解析での証明。(2) Wine の Export 1 本(`elog16-solo12` = config A、mp3wav-all 由来の同じ変更の FLP `solo12-wine.flp`)で、Solo の効果が WAV に出ることを確認。(3) ci1 向けの zip の作成。

### 11.1 FL の Playlist の Solo は何としてファイルに入るか

- FL の公式マニュアル(Playlist・Mixer の節)の記述: **Solo/Unsolo = トラックの Mute アイコンの Ctrl+左クリックか右クリック**。『General Settings の「Restore previous state after solo」は、Solo の前の mute/unmute の並びを残す』。= **Solo は『他のトラックを mute する』操作**で、状態は各トラックの mute(LED)のフラグ。『Solo の前の状態』は記憶の中の物と読める(FLP に別のフィールドがあるかは未確認)。
- FLP の Playlist のトラックの情報: イベント 238(TrackData、70 バイト、500 個)。PyFLP の `TrackEvent` の構造(読み): `iid u32 | color u32 | icon u32 | enabled u8 | height f32 | locked_height i32 | content_locked u8 | motion u32 | press u32 | trigger_sync u32 | queued u32 | tolerant u32 | position_sync u32 | grouped u8 | locked u8 | …`。この project の 500 個で確認: オフセット 0 = 1〜500 の通し番号、12 = `enabled`(全部 1)、30 = `trigger_sync` 5、38 = `tolerant` 1、46 = `grouped`(トラック 2・3・9・24・26・31 が 1)、47 = `locked`(全部 0)。**`enabled` が Playlist の LED(mute)で、Solo 用の別フィールドは見当たらない**(500 個のレコードで、固定値でない位置は 13〜17〔height と locked_height〕と 46 だけ)。
- したがって FL の Solo と等価な FLP は、**Solo にしない全トラックの `enabled` を 1 → 0 にした物**。ただし『FL が実際にこう保存する』ことは、Wine の FL で Solo を押して保存した FLP との比較では確認していない(今回は読み + Export の効果の確認まで)。

### 11.2 作った物(`~/work/opt1-wine/variants/solo12/` と `variants/solo12*.flp`)

| ファイル | 元 | 変更 | sha256 | 用途 |
|---|---|---|---|---|
| `variants/solo12.flp`(= `variants/solo12/serina1-serum2.flp`) | `variants/original.flp`(d6783e08886b4b48 = 配布 zip の中の FLP) | 12 本以外の **488 トラック**の `enabled` 1→0(488 バイト) | 8bfab4682fb21967… | **ci1 の Chrome の P3 計測**(正本) |
| `variants/solo12/serina1-serum2-solo12.zip` | `~/ダウンロード/serina1-serum2.zip` | zip 内の `serina1-serum2.flp` だけを solo12.flp に差し替え(他の 48 エントリのバイトは同一、49 エントリ) | be40c28c106fe4a1… | 同上(zip ごと) |
| `variants/solo12-content.flp` | original.flp | クリップのあるトラックのうち 12 本以外の **18 トラック**(6, 8-12, 14, 18, 21-23, 25-31)だけ mute(18 バイト) | 985655a13f877752… | 音は solo12 と同じのはず(空の 470 トラックの LED だけ違う)。変更が最小の版 |
| `variants/solo12-wine.flp` / `solo12-content-wine.flp` | `variants/mp3wav-all.flp`(ca4ad59f…、Wine の run 用) | 同じ変更 | 2c111d79345ca40a… / 073db7bb4b9d0a99… | Wine の検証 run 用(mp3 を WAV に差し替えた版) |
| `variants/solo12-changes.tsv` | - | 488 行: トラック番号・ファイルオフセット・変更前後(1→0)・solo12-content に含まれるか | - | 変更したバイトの表 |

- 変更したバイトの表(要約): `cmp -l original.flp solo12.flp` = **488 行**(`solo12-content.flp` は 18 行)。オフセットは各トラックイベントの本体の 12 バイト目(最初の変更 = トラック 6: ファイルオフセット 1128895、次 = トラック 8: 1129043 …、イベント間隔 74 バイト)。全行は `variants/solo12-changes.tsv`。
- **再解析での証明**(`make-solo12.py` が assert。`flp_model.read_events` で読み直し): イベント数 7,021 が元と同じ、ファイルサイズ 1,322,619 B が元と同じ、**元と違うイベントは track のイベント(238)だけ**(solo12 で 488 個、solo12-content で 18 個)で、そのイベントも**オフセット 12(`enabled`)の 1 バイトだけ**が違う、Solo の 12 本は `enabled` = 1 のまま、変更先は『12 本以外』と完全に一致。
- 参考(Playlist のクリップ): 3,565 個のクリップのうち、Solo の 12 本の上は 1,663 個、mute される 18 トラックの上は 1,902 個。Solo にした 12 本には、すべてクリップがある。
- **Wine での確認(1 本)**: elog16-solo12 = 5,715,911 フレーム(元と同じ長さ)、RMS 16256.2513(原本の Export の elog16 は 20412.1782)、elog16 との差の RMS 13684.17、max|d| 65534、差のあるサンプル 9,489,434(元の 13.66% のフレームだけが同一)。= **Solo の効果は WAV に大きく出ている**(Export が Solo を反映する)。その間の CPU: プロセス全体は平均 0.92 コアで、通常の Export の 2.3 コアより低い(処理が減った分 + 同時刻に他の job の負荷があったため。内訳は分けていない)。

### 11.3 ci1 の P3 計測での使い方(1 段落)

配布 zip `~/ダウンロード/serina1-serum2.zip` の中の `serina1-serum2.flp`(sha256 d6783e08886b4b48…)を、**`variants/solo12/serina1-serum2.flp`(sha256 8bfab4682fb21967…)で同じ名前のまま差し替える**。zip ごと使うなら、そのまま差し替え済みの `variants/solo12/serina1-serum2-solo12.zip`(be40c28c106fe4a1…)を使う(zip の中の他の 48 エントリは元のバイトのまま、FLP の名前も `serina1-serum2.flp` のまま)。計測側のパス・名前・読み込み手順は元と同じで、FLP のバイトだけが違う。確認の手順: 差し替え後の FLP の sha256 が 8bfab468… であること、FL(または lift)で開いて Playlist の Track 1-5, 7, 13, 15-17, 19, 20 の LED だけが点灯していること(他は消灯)。Solo の状態は FLP の読みでの等価物で、**FL で保存した FLP との比較は未確認**。

## 12. 仮想 `C:\Windows\Fonts\msgothic.ttc` の素材と、V0 生成器への取り込み(PM の指示 09:0x、O16 の契約案。期限 09:40)

**実際に実行した物**: Wine・Chrome は使っていない(CPU は nice 19)。(1) フォントの切り出しと TrueType(glyf)への変換(`tools/make-msgothic.py`、新設、fontTools 4.68)。(2) FreeType での検証(`tools/verify-msgothic.py`、新設、freetype-py 2.5.1 = FreeType 2.13.2 と `fc-query`)。(3) OFL の全文と著作権表示の作成(`fonts-pkg/OFL.txt`・`COPYRIGHT.txt`)。(4) V0.1 の生成器 `tools/vfs-build.py` に `--mount`・`--install-time` を足し(8 行の差)、(a) 既存の run-a と同じ入力・同じ引数での再生成 run-c が**1 バイトも変わらない**ことと、(b) フォント 1 個の root を `C:\Windows\Fonts` の索引にした run-f1・run-f2 が再現し、往復の検証で問題 0 であることを確認。契約の背景は alpha1 の 0319 §12 と PM の判断(契約案: `msgothic.ttc` の名前で日本語を持つ face を置く。字形は実機の MS Gothic でなく**代用**)。実装(runtime の変更)はしていない。

### 12.1 素材(`~/work/opt1-wine/fonts-pkg/`、SHA256SUMS つき)

| ファイル | 大きさ | 内容 |
|---|---|---|
| `msgothic.ttc` | **2,123,288 B**(sha256 daa1948fcd6cddc9549249d79ef075f6001c3693bf2a61ad8717679d7fb80b2f) | **本物の TTC(face 1 個)**。中身は Noto Sans CJK JP Regular のサブセットの TrueType(glyf)。**これが配る物** |
| `msgothic.ttf` | 2,123,272 B(sha256 0e00be96e2602261578f…) | 同じ face の素の TTF(FreeType は拡張子でなく内容で開くので、これでも足りるはず。どちらも開くことを確認した) |
| `OFL.txt` | 4,645 B | SIL OFL 1.1 の全文(上流 `Sans/LICENSE` のままの全文)+ フォントの著作権表示 |
| `COPYRIGHT.txt` | - | 出所・著作権・変更点・OFL の条件ごとの満たし方(英文) |
| `msgothic-sample.png` | - | この face を FreeType で描いた確認画像(`あいうえお アイウエオ 日本語 「、。」`/`ぷにぷに電機 ボリューム クリスマスセリナ`/`ＡＢＣ１２３ ABC abc 123 [fadr.com] Vocals`) |

- **サイズ**: 生 2,123,288 B → **brotli -q11 で 1,135,949 B**(生成器の既定の品質 9 では 1,188,919 B)、woff2(TTF から)960,796 B は参考。CFF の OTF のサブセット(§9.2 の S3、1,515,892 B)より TrueType(二次曲線)は約 40% 大きい。(PM の指定は glyf 形式。)
- **切り出し**: ASCII(U+0020–007E)+ NBSP + CJK 記号・ひらがな・カタカナ(U+3000–30FF)+ 全角・半角形(U+FF01–FFEE)+ 一般約物と矢印(U+2010–2027、U+2190–219F)+ **JIS X 0208 の全部**(記号・ギリシャ・キリルの第 1〜8 区、第一水準 2,965 字、第二水準 3,390 字 = 6,879 点)。要求 7,203 点のうち 7,175 点がソースにあり(28 点はソースに無い: 未割当の符号 U+3040・3097–3098・FFBF–FFDF の一部 など。JIS X 0208 のうちソースに無いのは **U+2252(≒)の 1 点だけ**)、7,171 グリフ。hinting・GSUB/GPOS/GDEF/BASE・縦書き表(VORG/vhea/vmtx)は落とした。CFF → glyf は fontTools の cu2qu(最大誤差 1/1000 em)。
- **FreeType での確認**(`tools/verify-msgothic.py` の出力は `fonts-pkg/verify-freetype.txt`): `FT_New_Face` は `msgothic.ttc`(face 0)と `msgothic.ttf` の両方で成功、`num_faces` 1(face 1 は例外 = 想定どおり)、family 'Noto Sans CJK JP'、style 'Regular'、グリフ 7,171、units_per_EM 1000、Unicode の cmap あり。**あ U+3042(gid 404)・ア U+30A2・日 U+65E5・本 U+672C・語 U+8A9E・「 U+300C・」 U+300D・、 U+3001・。 U+3002・ぷ・電・機・ボ・リ・ュ・ー・ム・全角英数 Ａ U+FF21・ａ U+FF41・０ U+FF10・＆ U+FF06・全角空白 U+3000・ASCII の A・z・0 のすべてが cmap にあり**、16 px の描画でインクのあるビットマップが出る(全角空白だけは空 = 想定どおり)。かな・全角の送り幅は 16 px(全角)、ASCII は 8〜10 px。JIS X 0208 は 6,879 点中 6,878 点が cmap にある(U+2252 のみ無し)。`fc-query` は TrueType・family 'Noto Sans CJK JP'・lang に ja を含む、と報告。
- **この確認の限界**: FL が実際に使うのは install の `Shared\freetype_x64.dll`(607 KB、版は不明 — version リソースが無い)で、host の FreeType 2.13.2 ではない。hinting 無しの TrueType は古い FreeType でも開けるはずだが、**FL の FreeType での表示は Wine での確認(J4: 下の 12.4)まで未確認**。

### 12.2 ライセンス(OFL 1.1)— 条件と、この素材の扱い

- 著作権表示: フォント自身の name table に `(c) 2014-2021 Adobe (http://www.adobe.com/).`(nameID 0)、`Noto is a trademark of Google Inc.`(nameID 7)、ライセンス(nameID 13・14)が入っている。**切り出した `msgothic.ttc` でも name table をそのまま残した**(確認済み: nameID 0〜14 が残る)。上流 `Sans/LICENSE` にも著作権の行も Reserved Font Name の指定も無い。
- **Reserved Font Name は宣言されていない** → 条件 3(改変版は RFN を使わない)は課されず、**名前の変更は不要**。家族名は『Noto Sans CJK JP』のまま。ただし OFL は商標を許諾しない(Noto は Google の商標)ので、『Noto 公式の物』と誤認される書き方はしない。保守的にするなら家族名(nameID 1・4・6・16)を中立の名前にする選択肢があり、他に依存は無い(COPYRIGHT.txt に書いた)。
- この素材は OFL の定義で『Modified Version』(形式の変更と構成要素の削除)。条件 2(各コピーに著作権表示と全文を添える)の満たし方: (a) font 内の name table(機械可読のメタデータ、利用者が見られる)、(b) `OFL.txt` と `COPYRIGHT.txt` を配布物の置き場(索引の隣、例: `licenses/`)に同梱し、About 等から辿れるようにする。**仮想 `C:\Windows\Fonts` の中には置かない**(OS のフォント置き場に txt を置く必要も、GDI に見せる必要も無い)。条件 1(単体で売らない)、条件 5(OFL のまま配る)は満たす。法務の判断ではなく作業メモ。

### 12.3 V0 の生成器への取り込み案(索引の別 root)

三案を比べた(生成器 `tools/vfs-build.py` の実コードを読んで): 索引は『mount を 1 つ持ち、全エントリが mount からの相対パス』(FLVFSIX1 v1、mt3 0313 §19 が規範)。

| 案 | 内容 | 索引の形式 | run-a への影響 | 評価 |
|---|---|---|---|---|
| **A(推奨)** | フォント 1 個だけの root `C:\Windows\Fonts` に**もう 1 つの索引**を作る(同じ生成器、同じチャンクの置き場 `vfs/<SHA-256>.chk`) | v1 のまま | **無し**(既存の索引と全オブジェクトは 1 バイトも変わらない) | 最小の変更。クライアントが『root の一覧』(mount と索引の名前の組)を持つ必要がある(下の封筒) |
| B | mount を `C:\` にして、`Program Files/Image-Line/FL Studio 21/…` と `Windows/Fonts/msgothic.ttc` を 1 つの索引に入れる | v1 のまま | **有り**(全エントリのパスに接頭辞が付くので索引が変わる。チャンクは同じ) | 索引は 1 つで済むが、`C:\` の仮想ディレクトリ(`Program Files`・`Windows`)が seed や OS 側の層と衝突しないか(mt3 V1 の層の合成)を決める必要がある |
| C | seed(alpha1 の `os-fonts/`、eager)に入れる | - | 無し | **P1(起動 10 s)を壊す**: 1.1 MB が起動の取得に乗る。遅延にならない。不採用 |

- 案 A の実装(**入れた**): `vfs-build.py` に `--mount GUESTDIR`(既定 = 従来の `C:\Program Files\Image-Line\FL Studio 21`)と `--install-time UNIXTIME`(既定 = 従来の規則: FL64.exe の mtime、無ければ最新のファイル)を追加(`mount` の使用 3 箇所と、引数 2 つ。差分は `diff scratch/vfs-build.py.orig tools/vfs-build.py`)。フォントの root は `vfs-out/fonts-root/msgothic.ttc`(mtime = FL64.exe の mtime 1790692308 に固定)。コマンド: `vfs-build.py --root vfs-out/fonts-root --out vfs-out/run-f1 --mount 'C:\Windows\Fonts' --install-time 1790692308`。
- **結果**(run-f1・run-f2、同じコマンドを 2 回): 索引 `index.a3786a47df98.br`(**128 B**、生は 124 B、エントリ 1 個〔ファイル 1、ディレクトリ 0〕、ストリーム 1 個 `large-other`)、チャンク 1 個 `vfs/daa1948fcd6cddc9549249d79ef075f6001c3693bf2a61ad8717679d7fb80b2f.chk`(1,188,919 B = 生 2,123,288 B を brotli q9。名前 = 論理バイトの SHA-256 = `msgothic.ttc` の sha256)。`vfs-verify.py --same run-f1 run-f2` = 同一、`vfs-verify.py run-f1 fonts-root`(往復)= **問題 0**(mount `C:\Windows\Fonts`、ファイル 1、2,123,288 B)。
- **run-a の不変の証明**: 変更後の生成器で、run-a と同じ入力(`prefix/drive_c/Program Files/Image-Line/FL Studio 2026`)・同じ既定の引数で run-c を作り、`vfs-verify.py --same run-a run-c`:  **ファイル一覧(名前と大きさ)が同一〔888 ファイル〕、チャンク以外のファイル(`index.9fde21c39f5d.br`・`index.bin`・`manifest.json`)はバイト単位で同一、チャンクのオブジェクトの名前 885 個が同一、さらに `diff -rq vfs-out/run-a vfs-out/run-c` で差 0〔888 ファイル全部がバイト同一〕**。(run-c は証明の後に削除した。2 分 29 秒で再生成できる。)
- **封筒(root の一覧)の案 — mt3 V1 の領分**: `roots.json` = `{"roots":[{"mount":"C:\\Program Files\\Image-Line\\FL Studio 21","index":"index.9fde21c39f5d.br"},{"mount":"C:\\Windows\\Fonts","index":"index.a3786a47df98.br"}]}`。クライアントは起動時に `roots.json` と各索引(計 215,576 + 128 B)を取り、各 root の名前空間を mount に重ねる。フォントのチャンク(1.14〜1.19 MB)は**取りに行かない**(索引だけ)。実行時のパスの見せ方(仮想 Windows のどこに、seed の `os-fonts` の arial/tahoma/lucon との合成を含めて)は mt3 の V1 と alpha1 の領分。
- 注意 1(**起動時の取得になる — §13.5 の p0-font で確認した**: FL のメインスレッドが起動の約 1.1 s 後に `msgothic.ttc` を開いて全体 2,123,288 B を 1 回で読む): QuickFontCache は固定の 10 名を `CreateFileW` で調べて一覧を作る(alpha1)。存在の確認だけなら索引の引きで済み、取得は無い。ただし、(i) 一覧に入った face を QFC が起動時に開いて中身(cmap)を読むなら、`msgothic.ttc` は 10 名のうち 6 番目で、arial/tahoma より前に見られるため、**ラテン文字の代用が要る最初の描画**(主 face が持たない字、例: `…`・`×` など)で全体(約 1.14 MB)の取得が起きる。(ii) 英語 UI だけで、主 face が全部持つなら取得は起きない。判定は file trace の P0(起動〜idle)に msgothic.ttc を置いた状態で『FL が msgothic.ttc を読むか、何バイト読むか』を見る(§9.3 に項目として足す)。読まれるなら、起動から切り離すために(a) 取得を P1 の予算内の最初の 1 チャンクに収める(現状は 1 チャンク)、(b) 取得が走る間の描画を止めない(非同期の取得は VFS 層の領分)、のどちらかが要る。
- 注意 2(副作用の見込み): 一覧の順は msjh.ttf・msjh.ttc・msyh.ttc・malgun.ttf・batang.ttc・msgothic.ttc → arial.ttf → tahoma.ttf → seguiemj.ttf → segoeui.ttf(alpha1 の読み。私も QuickFontCache の文字列の並び〔file offset 0x17ea48〜0x17eb84 に連続〕で同じ 10 名・同じ順を確認した)。主 face に無い字は msgothic(Noto)が先に答える。Noto のラテン・記号は arial/tahoma の代用(Liberation/DejaVu)とは字形が違うが、『主 face が持つ字は主 face』なので通常の表示は変わらない読み(alpha1 の見込みと同じ。未測定)。

### 12.4 検証の計画(Wine の窓、Chrome 0 本、PM の合図で。各 run 2〜3 分)

- **J4(契約の検証)**: prefix-e3(config A)の `drive_c\windows\Fonts` に `msgothic.ttc` を**置いた/置かない**の 2 本。同じ project・同じ画面(日本語が出る箇所: Playlist の `ぷにぷに電機` を含むクリップ名、Channel rack、`ボリューム`)のスクリーンショットを比べる。予測: 置かない = 豆腐(この prefix の `C:\Windows\Fonts` は空で、QFC の一覧が空になる)、置く = 日本語の字形。これが当たれば alpha1 の (c) 資材の不足の読みと、この素材が FL の FreeType で開けることの両方が確認できる。予測が外れたら、`+font`/`+text` ログと strace(§9.3)へ。
- 日本語が出る場所の探し方(FLP の再解析): `Balance - Insert 1 - ボリューム`(チャンネル 5 のオートメーション、Playlist の Track 4・5・7・17 に 256 個のクリップ)、`[fadr.com] Vocals - Still_in_my_heart_feat_ぷにぷに電機 - 172bpm - Cmin - Panning`(チャンネル 59 のオートメーション、Track 13・19・27 に 5 個)、サンプル名 `[fadr.com] Vocals - Still_in_my_heart_feat_ぷにぷに電機 - 172bpm - Cmin` と `[fadr.com] Vocals - クリスマスセリナ　メモリアルロビー　高画質 - ぶるぶるあーかいぶ - 115bpm - Cmin`(Playlist のクリップ名・Mixer のトラック名に出るはず)。
- 起動時の取得(注意 1)は P0 の file trace に msgothic.ttc を置いた状態を 1 本足して確かめる。

## 13. Wine の窓が出た時の手順書: J4(msgothic.ttc の有無)・P0(起動時の file trace)・J0〜J3(準備のみ。Wine は起動していない)

**実際に実行した物**: Wine は起動していない。作った道具の Wine を使わないテストだけ: (1) `tools/make-ja-names-flp.py`(日本語のチャンネル名の FLP を作り、再解析で『違うイベントは 2 つだけ』を assert)、(2) `tools/e3-font.sh put/rm/status` の往復(prefix-e3 の `C:\windows\Fonts` に `msgothic.ttc` を置いて sha を確認 → 取り除いて空に戻す)、(3) 条件の判定式(MemAvailable・load1・swap)を実測値で、(4) 構文の確認(`bash -n`、`ast.parse`)、(5) host の fontconfig から CJK を外した設定(`scratch/fontconfig-nocjk/fonts.conf`)で `fc-list ':charset=3042'`(あ)と `:charset=65e5`(日)の保持者が 0 になること、(6) `tools/ja-compare.py` を既存の 2 枚のスクリーンショットでテスト。**run は 1 本も実行していない**。PM の条件(Wine 可の info の後。開始時 MemAvailable ≥ 12 GB、load ≤ 12、swap の入出力 0。Chrome との重なりは可。Export 級の長い run は重ねない)は `ja-run.sh` が自動で確認して止まる。

### 13.1 道具(全部 `~/work/opt1-wine/tools/` とその周辺)

| 物 | 役目 |
|---|---|
| `variants/ja-names.flp`(sha256 eb30c2e107cb2564…、1,322,633 B) | mp3wav-all.flp から、チャンネル名 2 つだけを日本語にした FLP(`Sampler` → `サンプラー　クリスマスセリナ`、`snare1 (consolidated)` → `ぷにぷに電機　日本語ボリューム（Ｓｏｌｏ）`)。1 つ目は Channel rack の 1 行目、2 つ目は Playlist の見出し(`Playlist - Arrangement > …`)に最初の画面から出る。+14 B、イベント 7,021 個のうち違うのはこの 2 個 |
| `tools/e3-font.sh put\|rm\|status` | prefix-e3 の `C:\windows\Fonts` に `fonts-pkg/msgothic.ttc`(sha daa1948f…)を置く/取り除く。e3 の外(`readlink -f`)を拒否、Wine が走っていれば拒否 |
| `tools/wine-ja-run.py` / `tools/elog-run-ja.sh` | 短い非計時の Wine セッション。project あり: ELOG の『読み込み終了』の 15 秒後に Return → 6 秒後(`-main-a.png`)と 26 秒後(`-main-b.png`)の最初の画面のスクリーンショット。idle(`JA_MODE=idle`、project なし): 30・60・90 秒のスクリーンショット。どちらも Export はしない |
| `tools/ja-run.sh TAG none\|msgothic [project\|idle] [WINEDEBUG]` | 上を束ねる。**事前の確認**(Wine が無い、MemAvailable ≥ 12288 MB、load1 ≤ 12、swap 入出力 0、e3 の状態が config A = `state-before-elog17.txt` と同一)→ e3 の 3 つの .reg を退避 → 状態・guard の一覧 → font の置く/外す → run → **後始末**(Wine を止める、font を外す、.reg を戻す、状態と guard を比べる、proj の FLP の sha、WAV の残り、Wine の process 数) |
| `tools/ja-compare.py A.png B.png [PREFIX]` | 2 枚の最初の画面の `CH`(Channel rack 1 行目)・`HD`(Playlist 見出し)を切り出して 4 倍に拡大して保存し、差のある画素数とインクの画素数を出す |
| `tools/trace-fl.sh`(既存、環境を `TRACE_ENVF` で選べるようにする案)と `tools/file-trace-summary.py` | 従来の attended な file trace(P1〜P4 の『ブラウザ・Settings・Debug・About』)。idle の P0 は `ja-run.sh ... idle` が `WINEDEBUG=-all,+file,+timestamp` で取り、`file-trace-summary.py exports/runs/TAG.fl.log` で集計 |
| `scratch/fontconfig-nocjk/fonts.conf` | host の CJK フォント(`/usr/share/fonts/opentype/noto`・`truetype/droid`・`X11/misc` の kana を持つ bitmap)を外した fontconfig の設定。`FONTCONFIG_FILE=` で Wine に渡す(host のファイルには触れない) |

### 13.2 run の表(各 2〜3 分、config A の prefix-e3。Wine 可の後、上から順に)

| # | コマンド | 取る物 | 判定(予測) |
|---|---|---|---|
| 1 | `bash tools/ja-run.sh j4-none none project` | `exports/runs/j4-none-main-a.png`・`-main-b.png`、`j4-none.fl.log`、hook のログ `exports/elog/j4-none.elog`(modload に QuickFontCache・freetype) | **msgothic.ttc 無し**: 日本語が出ない(豆腐か空白)。この prefix の `C:\windows\Fonts` は空で、QuickFontCache の一覧が空になる、という alpha1 の読みの予測 |
| 2 | `bash tools/ja-run.sh j4-font msgothic project` | `j4-font-main-a.png`・`-main-b.png` | **msgothic.ttc 有り**: `サンプラー…`・`ぷにぷに電機　日本語ボリューム…` の字形が出る。= 契約案(c 資材の不足)の実機での裏付け + FL の FreeType で当素材が開く確認 |
| 3 | `python3 tools/ja-compare.py exports/runs/j4-none-main-a.png exports/runs/j4-font-main-a.png exports/runs/j4` | `j4-CH-A/B.png`・`j4-HD-A/B.png`(4 倍) | 差のある画素数とインク画素数(B が A より明らかに多ければ字形が出ている)。画像は目で見て判定 |
| 4 | `bash tools/ja-run.sh p0-none none idle` | `p0-none.fl.log`(+file,+timestamp)、`p0-none-idle-30/60/90.png` | **起動〜idle の間の `C:\windows\Fonts` へのアクセス**: 固定 10 名(`msjh.ttf` … `segoeui.ttf`)の `CreateFileW` の試行が出るか(alpha1 の逆アセンブルの確認)。`grep -a -i 'Fonts' p0-none.fl.log \| head -40` と `file-trace-summary.py` |
| 5 | `bash tools/ja-run.sh p0-font msgothic idle` | `p0-font.fl.log` | **起動時に msgothic.ttc の中身を読むか、何バイトか**(`CreateFileW ... msgothic.ttc` の後の `ReadFile` の回数・バイト数)。読まないなら 1.14 MB の取得は起動に乗らない。読むなら §12.3 注意 1 の対策が要る |
| 6(任意) | `bash tools/ja-run.sh j0-api none project '-all,+font,+text'` | `j0-api.fl.log`(`CreateFontIndirectW` の face 名・charset、`ExtTextOutW` の文字列) | 日本語が GDI を通るか(QuickFontCache の FreeType の経路だけなら `ExtTextOutW` に日本語は出ない)。ログが大きいので、`exports/runs/` に出る(/tmp は使わない) |
| 7(任意) | `FONTCONFIG_FILE=$PWD/scratch/fontconfig-nocjk/fonts.conf bash tools/ja-run.sh j3-nocjk none project` | `j3-nocjk-main-a.png` | host に日本語のフォントが無い状態(製品の仮想 Windows に近い): #1 と同じ豆腐になるか。#1 が『日本語が出る』だった場合(= Wine は host の CJK を使っている)の対照として必須 |
| 8(任意) | 7 と同じ設定で msgothic を置く: `FONTCONFIG_FILE=… bash tools/ja-run.sh j3-nocjk-font msgothic project` | `j3-nocjk-font-main-a.png` | host に CJK が無くても msgothic.ttc だけで日本語が出る = 製品と同じ条件での契約の確認 |

- **#1 の予測が外れた場合**(msgothic.ttc が無くても日本語が出る): Wine は host の CJK フォントを GDI の fallback 経由で使っている = 契約は『QuickFontCache の一覧』だけではない可能性。その時は #6(`+font,+text`)を先に取り、#7(host の CJK を隠す)で『host のフォントが効いているか』を確かめる。J1(Browser のファイル名)・J2(Language を Japanese)は #1〜#8 の後(attended な操作が要る: Settings→General→Language は 12.1 の `settings1-general.png` の座標、再起動が要る)。
- 時間の見積もり: #1・#2・#7・#8 は約 2.5〜3 分(project の読み込み 35〜50 秒 + 26 秒)、#4・#5 は約 2 分、#6 は +ログの大きさ。8 本で約 20 分。非計時なので重なる負荷の影響は無い(Export の WAV は作らない)。

### 13.3 状態の記録と元に戻す手順(rehide)

`ja-run.sh` が各 run で自動で行う。run のたびに出る行と、期待する値:

1. **開始前**: `prefix-e3/drive_c/windows/Fonts: (0 entries)`、e2 と原本の Fonts も 0 entries。`prefix-e3 state` が `state-before-elog17.txt` と同一(config A)。`guard-before-TAG` を作る(原本と e2 の一覧)。e3 の `system.reg`・`user.reg`・`userdef.reg` を `scratch/regbak-TAG/` に退避。
2. **run の後**: `wineserver -k`、`e3-font.sh rm`(Fonts が再び 0 entries)、3 つの .reg を退避から戻す(FL が書いた設定・実行回数・フォント一覧は消える)、`state-after-TAG.txt` が `state-before-TAG.txt` と同一であること(プラグインのフォルダ・DLL の sha は run で変わらない)、`guard diff before after`(**原本 prefix は差 0、prefix-e2 も差 0**。e3 は guard の対象外だが、hard link の共有を壊していないことは e2 の差 0 が示す)、`proj/serina1-serum2.flp` が `d6783e08886b4b48`、variants の WAV 残り 0、FL64/wineserver 0。
3. **異常時**: run が途中で止まった/時間切れ → `wineserver -k` → `bash tools/e3-font.sh rm` → `scratch/regbak-TAG/` から .reg を戻す → `variants/original.flp` を `proj/serina1-serum2.flp` に戻して sha を確認(`wine-mp3wav.sh` は run の後に自動で戻すが、強制終了されたら手で)。
4. **FONTCONFIG_FILE を使った run(#7・#8)**: Wine が host に無い font の一覧を `system.reg`/`user.reg` に書く。.reg の復元で戻る。host の fontconfig のキャッシュは `scratch/fontconfig-nocjk/cache` に出る(`~/.cache/fontconfig` は汚さない)。
5. 全部が済んだ後に残る物: `exports/runs/` の PNG とログ(`.fl.log` は大きいことがある)、`exports/elog/`。

### 13.4 注意

- この手順は Export の WAV を作らないので、Wine の『参照 WAV』の基準には影響しない。prefix-e2(config A の基準)には触れない。
- `ja-run.sh` は `PFX=prefix-e3` 固定(e2 では動かさない)。e3 の `wintrust.dll` は hook 付きの物に一時的に差し替わり、run の後に `elog-run-pfx.sh` と同じ仕組みで戻る。
- 日本語の描画の判定は目で見る(`ja-compare.py` の拡大画像を `read` で開く)。画素数は補助。
- 手順書は、PM の『Wine 可』の前に実行しないこと。

### 13.5 実行結果(09:19〜09:31、PM の『Wine 可』の後に 4 本。1 本ずつ)

**実際に実行した物**: `ja-run.sh` で Wine の短い非計時の run を 4 本(prefix-e3・config A、Export なし)。j4-none(09:19:46〜09:21:03)、j4-font(09:23:08〜09:24:24)、p0-none(09:26:39〜09:28:19)、p0-font(09:29:54〜09:31:33)。**始めずに止めた事前確認が 3 回**: j4-font(09:22:03)と p0-none(09:25:22、09:26:07)は、swap の入出力が 2 秒窓で 4(最大 140 KB/s の 1 サンプル)と出て自動で停止した(PM の条件どおり)。原因は他の job(wasm-opt・Chrome)のメモリ圧と読む。確認を『2 秒窓を最大 6 回試して 0 になったら開始』に直し(`ja-run.sh`)、j4-font は同じ条件で 1 分後に再試行、p0-none・p0-font は 6 回の内の 1 回目で通った。各 run の後: Wine の process 0、msgothic.ttc を外して Fonts 空、3 つの .reg を戻した、**prefix-e3 の状態の sha は run の前後とも b07cf6d2265ece73(config A、`state-before-elog17` と同一)**、guard(原本・prefix-e2)の前後差 0、FLP は d6783e08886b4b48、variants の WAV 残り 0。

**J4(日本語が出るか。`variants/ja-names.flp` を読み込み、Problems 窓を Return で閉じた最初の画面)**
- **j4-none(msgothic.ttc 無し)**: Channel rack の 1 行目も、Playlist の見出し(`Playlist - Arrangement › …`)も、日本語の字形でなく**豆腐(□)**。1 行目は `▯▯▯▯▯▯‥▯▯▯▯▯`(FL が長い名前を『‥』で省略した形)。画像: `exports/runs/j4-none-crop-ch.png`・`j4-none-crop-hd.png`(2 倍に拡大した切り出し)、全体は `j4-none-main-a.png`。
- **j4-font(msgothic.ttc を `C:\windows\Fonts` に置く)**: Channel rack の 1 行目 = `サンプ‥リナ`(『サンプラー　クリスマスセリナ』を FL が省略した形)、Playlist の見出し = **`ぷにぷに電機　日本語ボリューム（Ｓｏｌｏ）`が完全に読める字形で出た**(全角の `Ｓｏｌｏ` も)。画像: `j4-font-crop-ch.png`・`j4-font-crop-hd.png`・`j4-font-main-a.png`。`ja-compare.py` の画素数: CH 472/3,520 が違う(インク 767 → 613)、HD 1,174/7,700 が違う(インク 1,373 → 1,378)。
- **読み取り(事実)**: (1) **契約案(資材の不足)は当たった**: 同じ project・同じ Wine で、`msgothic.ttc` の有無だけで、豆腐 ↔ 日本語になる。(2) **FL 自身の FreeType(`Shared\freetype_x64.dll`)が、当素材〔TrueType glyf・本物の TTC〕を開いて、日本語・全角英数・全角括弧を描けた** — 12.1 の『FL の FreeType では未確認』が確認になった。(3) j4-none の時、host の fontconfig には Noto Sans CJK JP があり、Wine の GDI からは見えていた(`HKLM\...\Fonts` に 531 項目)のに、**FL の文字は豆腐のまま** = FL の自前の文字描画(QuickFontCache)は Wine の GDI の fallback を使わない(alpha1 の静的な読みと一致)。J3(host の CJK を隠す対照)は、この結果から必須でなくなった(任意のまま)。

**P0(project なし、起動〜90 秒の `WINEDEBUG=+file,+timestamp`、各ログ約 100 MB は `exports/runs/p0-*.fl.log`)**
- **p0-none**: FL のメインスレッド(tid 0x24)が、起動の直後(FL64.exe の最初の記録から約 0.9 s)に `GetFileAttributesW` で `C:\windows\Fonts`(ディレクトリ)→ **`msyh.ttc`・`malgun.ttf`・`batang.ttc`・`msgothic.ttc`・`arial.ttf`・`tahoma.ttf`・`seguiemj.ttf`・`segoeui.ttf` の順に 8 個**を 1 つずつ確認し、すべて『見つからない』(Fonts は空)。**`msjh.ttf`・`msjh.ttc` は確認されなかった**(QuickFontCache の文字列には 10 個あるが、この run では 8 個。この Wine の ACP は 932。ロケール等による分岐かもしれないが未確認)。
- 名前で探す UI のフォント(`OpenSans-Regular.ttf`・`OpenSans-CondLight.ttf`・`ILGlyphsEx.ilfont`・`WebSymbols-Regular.otf`・`PT_Sans-Web-Regular.ttf`・`Barlow-Medium.ttf`・`Share-Regular.ttf` ほか)は、**まず `C:\windows\Fonts\<名前>`、無ければ `C:\Program Files\Image-Line\FL Studio 2026\Shared\Artwork\Fonts\<名前>`** の順に `GetFileAttributesW` で確認して、見つかった方を `CreateFileW` で開く(Artwork 側のファイルが開かれた)。`segoeui.ttf`・`segoeuisl.ttf`・`tahoma.ttf`・`lucon.ttf`・`arial.ttf`・`Arial.ttf`・`MS Sans Serif.ttf` は両方に無く(= 仮想 Fonts の代用 3 ファイルが無い Wine では)、`OpenSans-Regular` などで代わる。
- **p0-font(msgothic.ttc 有り)**: 上の 8 個の確認で `msgothic.ttc` が見つかり、**FL のメインスレッドが `CreateFileW` で開いて、`ReadFile` 1 回で全体 2,123,288 バイトを読んだ**(FL64.exe の最初の記録から約 1.1 s 後、最初の画面が出る前。ログ上の時刻 90919.334)。= **起動の最初に、ファイル全体が読まれる**。(Wine 自身の win32u も別のスレッドで `C:\windows\fonts` を走査しているが、Wine の GDI の初期化で、製品の仮想 Windows とは無関係。)
- **示唆(P1 に効く)**: 製品で `msgothic.ttc` を遅延 VFS の層に置くと、**起動の最初に 1 チャンク(2,123,288 B 論理、brotli-11 で 1,135,949 B、生成器の既定 q9 で 1,188,919 B)の取得が起き、その間 FL の `ReadFile` は待つ**(メインスレッドで同期的に)。= 『日本語が要った時だけ取得』にはならない(QuickFontCache は起動時に、存在した face の全体を読む)。対策の選択肢(PM の判断): (a) seed(eager)に入れる: 初回の転送が +1.14 MB。起動の待ちは増えないが、P1 の取得量が増える。(b) 遅延のままページの読み込み時に先読み(エンジンの取得と並行): 起動の前に取得が済んでいれば待ちは 0。(c) 小さい層: **S1(かな + 記号 + ASCII)= TTC 104,620 B・brotli-11 58,022 B、S2(+JIS 第一水準)= 972,984 B・542,797 B、S3(JIS X 0208 全部、当素材)= 2,123,288 B・1,136,304 B**。UI の日本語に要る字(`ja.moe` の字の集合は未測定)で層を決める。S1 は漢字が豆腐になる。(d) 日本語のロケールの時だけ置く(リソースの選択。FL のクラスや窓による分岐ではないが、PM の方針に照らして判断が要る)。
- 注意: `head.modified`(ビルド時刻)が出力に入るため、出荷した `fonts-pkg/msgothic.ttc`(sha daa1948f…)は再生成するとバイトが変わる(他の 10 表は同一を確認)。`make-msgothic.py` に `recalcTimestamp = False` を足し、再現できる版を作った(`scratch/fonts-det-1/msgothic.ttc`、sha256 1ab00fd8ffe15553c16e…、2 回作って同一)。出荷物の差し替えは PM の判断(チャンク名が変わる以外の影響は無い)。

## 14. font-seed: `C:\Windows\Fonts\msgothic.ttc` を eager な seed の行にする素材(PM の判断 O16 (a)、09:3x。適用はしていない)

**実際に実行した物**(Wine 以外): (1) 再現できる版のフォント(`recalcTimestamp=False`、2 回作って同一)の最終確認(FreeType 2.13.2 で開く・字・JIS X 0208 6,878/6,879)と、一語のチャンク/索引の生成(`vfs-build.py --mount 'C:\Windows\Fonts'`)。(2) 素材一式の作成 `lift/out-git/opt2-results/0320/font-seed/`。(3) 2 つの diff を、ワークツリーに `patch --dry-run -p1`(クリーン)、2 ファイルの一時コピーに本当に `patch -p1`、コピー後のフォントの SHA-256 を新しい行の値と照合(一致)。(4) seed の機構(`ci/lib/extract_fl.py`・`lift/harness/web/final0129/pack_seed.py`・`ci/plan.sh`)の読み。リポジトリには何も書いていない。

**素材(`font-seed/`)** — コピー先は PM が新しい `git archive HEAD` に当てる。バイナリは diff に含めない。

| ここのファイル | コピー先(リポジトリの根から) | 大きさ | SHA-256 |
|---|---|---|---|
| `NotoSansCJKjp-subset-msgothic.ttc`(**再現版**) | `lift/harness/seeds/os-fonts/NotoSansCJKjp-subset-msgothic.ttc` | 2,123,288 B | `1ab00fd8ffe15553c16ef441b6a63e539159d560491c70fbc598d26f470eea69` |
| `COPYRIGHT-fonts-noto-cjk-jp.txt` | `lift/harness/seeds/os-fonts/COPYRIGHT-fonts-noto-cjk-jp.txt`(既存 2 本と同じ Debian copyright 形式、OFL 1.1 の全文入り) | 6,022 B | - |
| `os-fonts-README.md.diff` | `lift/harness/seeds/os-fonts/README.md` に当てる(行の追加・差し替え、既存の README の書式〔表の列・文体〕) | - | - |
| `vfs-seed-idle1.tsv.diff` | `lift/harness/seeds/vfs-seed-idle1.tsv` の末尾に 1 行: `C:\Windows\Fonts\msgothic.ttc<TAB>lift/harness/seeds/os-fonts/NotoSansCJKjp-subset-msgothic.ttc<TAB>1ab00fd8…eea69` | - | - |

README の追記の中身: 代用であること(MS Gothic ではない)、由来(`fonts-noto-cjk 1:20240730+repack1-1build1` の `NotoSansCJK-Regular.ttc` の face 0)、サブセットの範囲(ASCII・NBSP・U+2010-2027・U+2190-219F・U+3000-30FF・U+FF01-FFEE・JIS X 0208 全部 = 6,879 点〔ソースに U+2252 だけ無い〕、7,171 グリフ)、TrueType 化・hinting/layout 表の除去、OFL 1.1 で Reserved Font Name の宣言なし、**FL が起動時に全体を読む**こと(Wine の測定、§13.5)、作り方(`tools/make-msgothic.py`、fonttools 4.68.0・brotli 1.2.1、`MSGOTHIC_TIER`、再現できること)。

**seed の機構に必要な変更: 無い**(読んだ範囲)
- `ci/lib/extract_fl.py`: `REPO_ROWS = "lift/harness/seeds/os-fonts/"` — host path がこの下の行は、リポジトリの中の実体を `HOST_ROOT/<host path>` にコピーして行の SHA-256 と照合(不一致で exit 2)。新しい行は同じ形なのでそのまま通る。
- `pack_seed.py`: host file のある行を `seed/<host path>` に詰め、相対パス(`..` なし)と行の SHA-256 を確認(不一致で停止)。拡張子・大きさの制限は無い(`.ttc` はただのバイト列)。pack は 2,123,288 B 増える。
- `ci/plan.sh`: `H_SEED` が `pack_seed.py` と `lift/harness/seeds` の全体を hash する → フォントと行を足すと **seed の key が変わる**(seed の pack とそれに依存する物が 1 回作り直される。想定どおり)。`H_EXTRACT_TOOLS` も seed の一覧を hash する。
- `ci/`・`lift/harness/*.sh`・`lift/harness/web/final0129/` に `os-fonts` を参照する他の箇所は無い。

**大きさと起動への効果**: 生 2,123,288 B、brotli-11 で 1,136,148 B、生成器のチャンク(q9)で 1,188,870 B。seed の取得は +約 1.14 MB(PM が許容と判断)。FL は起動の約 1.1 s 後に全体を同期で読む(§13.5)ので、seed に入れれば起動の待ちは増えない。小さい層(S1 104,620 B / brotli 58,022 B、S2 972,984 B / 542,797 B)への縮小は、J2 で ja.moe の使う字の集合が分かってから。

## 15. r2・r3(ins1-lp の再実行)が『Save dialog TIMEOUT』になった原因 — Xvfb :88 に残った Chrome がキーボードフォーカスを持っていた(09:46〜09:59)

**実際に実行した物**(Wine なし): r2・r3 の events と画像の読み(`after-return` と `problems-unrecognized` の窓の領域の画素差の数値化)、`xwininfo -root -tree`・`xprop _NET_WM_PID`・`XGetInputFocus`(ctypes)での :88 の状態の確認、Chrome の閉鎖、スクリプトの修正。Wine の run は起動していない(PM の合図待ち)。

- **事実**: (1) r2・r3 とも events は `Return pressed` → `Ctrl+R` → `Save dialog TIMEOUT` → 固定位置の OK クリック → `Ctrl+R (second try)` → `TIMEOUT` → 終了。chain の終了コードは 4(3 本目の ins1-bp は swap の事前確認で始まらなかった)。(2) `after-return` の画像は `problems-unrecognized` と同一(窓の領域〔610,385,700x60〕で、基準の『窓が消えた』画像との差分画素 18,5xx = 窓が残っている。成功した elog16・elog16-ins1-lp・elog23 は差 0)。= **Return で Problems 窓が閉じていない**。(3) r3 の画像の窓は『6 plugin(s) could not be opened』の通常の窓(691x308、Ok は y=667)。『sample(s) could not be located』の項は無く、PM が挙げた ci1 の wasm の run のように大きくならない(Wine では WAV を差し替えて揃えているため)。固定 y の 683・723・703 は 6 件の窓の Ok(667)に当たらないが、これは二次的で、最初の Return が効けば要らなかった。(4) **:88 に Chrome の窓**: `"Are you enjoying your FL Studio Trial? [EN] - Google Chrome"` 1402x1060+0+0(pid 851868、cwd = 私の prefix-e2 の FL のフォルダ、環境の DISPLAY=:88、起動 08:48 台)。`XGetInputFocus` = その窓(0xC00002、revert_to PointerRoot)。(5) 08:49:03 は、Settings の 2 本目で FL を右上の × で**正常終了**させた時刻。
- **読み**: FL は試用版を正常終了すると『アンケート』の URL を開く(Wine の winebrowser → ホストの `google-chrome-stable`)。その Chrome が :88 に残ってフォーカスを持ち、WM の無い Xvfb ではキー入力がその窓に行った。FL の Problems 窓(モーダル)への Return、メイン窓への Ctrl+R は FL に届かなかった。08:49 以後の run(j4-none・j4-font・p0-none・p0-font・r2・r3)はすべてこの状態。Export をしない 4 本(J4・P0)は、日本語の描画と file trace の結果に影響しない(Return が届いたかは未確認)。08:00〜08:34 の Export 12 本は Chrome が出る前で影響なし。**『窓が大きい/Ok の位置』の説は、この Wine の run には当たらない**。
- **対処(実施)**: (1) pid 851868 とその子(私の Wine が開いた Chrome)を `kill` で閉じた(09:57)。:88 の窓 0、`XGetInputFocus` = PointerRoot。gpu1/ci1 の headless Chrome には触れていない。(2) `tools/x88-check.sh`(新設): Wine が走っていない時に :88 に窓があれば停止。`elog16-run.sh`・`elog16-variant-run.sh`・`queue-run.sh`・`ja-run.sh`・`trace3-begin.sh` の事前確認に入れた。(3) 以後 FL は `wineserver -k` でしか終わらせない(正常終了させると Chrome が開く)。(4) ドライバ `wine-export-run.py`・`wine-ja-run.py`: Return の前にポインタを窓の中央(960,556)へ動かす、Return の後に窓が残っていれば `xwininfo` で窓の位置を取って OK を相対位置(窓の左から 369、下から 28。6 件の窓で (983,665)、ci1 の 423 高の窓で +395 に一致)でクリックする。**この変更は Wine で未テスト**。(5) `ALLOW_CHROME=1`(ci1 の Chrome Export と Wine の Export の重なりを許す PM の指示のため、`busy-check.sh` が chrome を無視する)は、:88 の Chrome を見逃した原因の一つ。x88-check は ALLOW_CHROME の影響を受けない。
- **自己申告**: 08:49 に Settings の run を正常終了させた後、:88 の窓を確認しなかった。r2・r3 を始める前の事前確認にも :88 を入れていなかった。
- **付記(製品に効くかもしれない)**: FL は試用版の終了で URL を開く。製品(ブラウザ)で FL の窓を閉じる操作が起きると同じ呼び出しが起きる。呼び出しの API(ShellExecuteW など)は今回の log には無く未確認。

## 16. P1〜P4 の file trace の手順書(Wine は PM の合図の後。ins1-lp ×2 + ins1-bp ×1 が先)

**実際に実行した物**: Wine は起動していない。道具の作成と、既存の `p0-font.fl.log` でのテスト: `trace3-begin.sh`・`trace3-run-fl.sh`・`trace3-mark.sh`・`trace3-end.sh`・`e3-unhide-all.sh`・`e3-font.sh putall`(begin → 取り消しの往復は 09:45 に 1 回実行し、状態が config A に戻ることを確認: `state-after-p4` = `state-before-elog17`、guard 差 0、Fonts 空)、`trace3-phase.py`(フェーズごとの集計)のテスト。**P0 の既存ログ(p0-none・p0-font)の集計**を `exports/trace/p0-font-summary.txt` に出した(下)。

- **構成**: prefix-e3、プラグインは全部見える(`e3-unhide-all.sh unhide`、config A に戻すのは `rehide`)、Fonts は製品の想定(arial・tahoma・lucon の代用 3 つ + seed の msgothic.ttc)。project なし。`WINEDEBUG=-all,+file,+timestamp`。ログは `exports/trace/tr3-TAG.log`(/tmp ではない)。終了は `wineserver -k` のみ。
- **手順**(attended。1 コマンドずつ、各段で `trace3-mark.sh p4 NAME` を打つ):
  1. `bash tools/trace3-begin.sh p4`(事前確認〔MemAvailable ≥ 12 GB、load ≤ 12、swap 0、:88 の窓 0、Wine なし、e3 が config A〕、.reg の退避、unhide、Fonts)→ 別呼び出しで `bash tools/trace3-run-fl.sh p4`(background)。
  2. 約 60〜75 秒待つ(usage data の同意・Welcome のダイアログが出る。同意は『No thanks』〔1036,561〕)→ `trace3-mark.sh p4 P0-idle`。
  3. **P4 (About、最優先)**: `xt move 1500 900; xt key F10`(Settings)→ mark `P4-begin` → About タブ(1222,138)をクリック → ロゴ/スタッフロールの動き出しまで待ち、スクリーンショット(`shot.sh p4-about-a/b/c`、3 s・12 s・25 s)→ mark `P4-end`。
  4. **P3 (Debug)**: Debug タブ(1089,138)→ mark `P3-begin` → 8 秒 → mark `P3-end`。**P2 (General)**: General タブ(771,138)→ mark `P2-begin` → 8 秒 → mark `P2-end`。Settings を閉じる(1278,106)。
  5. **P1 (Browser)**: 左パネルの ALL タブ(30,123)→ mark `P1-begin` → スクリーンショットで Packs の位置を見て展開 → mark `P1-end`。
  6. `bash tools/trace3-end.sh p4`(Wine を kill、Fonts を外す、rehide、.reg を戻す、状態・guard の前後を比べる)。
- **集計**: `python3 tools/trace3-phase.py exports/trace/tr3-p4.log --marks exports/trace/tr3-p4.marks --phase P4-begin`(その段の開いたファイル・読んだバイト・列挙したパターン・無かった名前)。About で開くファイル(画像・ビットマップ・リソース)が O3(ロゴが出ない)の手がかり。
- **ログの限界**: `+file` に CloseHandle が無いので、ReadFile のバイト数を『開いたファイル』に結び付けるのは同じスレッドの直近の open での推定。要求サイズでなく `NtReadFile = SUCCESS (n)` の n を使う。開いたファイルの一覧(CreateFileW)は確か。

**P0(project なし、起動〜90 秒)の既存ログの集計**(`p0-font`、e3 config A、msgothic.ttc 有り): FL64.exe の起動から約 90 秒の間に **CreateFileW で開いた異なるファイル 1,184 個**、FindFirstFileExW のパターン 1,439 個、『見つからなかった』名前 12,468 個(`NtQueryAttributesFile`)。Wine の起動(win32u・Mesa 等の host のライブラリ)のファイルも含むので、FL の分は `C:\Program Files\...` 以下とそれ以外を分けて読む必要がある。目立つもの: 起動の最初に `FLEngine_x64.ja-JP`・`.JPN`・`.JP`、`Shared\QuickFontCache_x64.ja-JP/.JPN/.JP`、`Shared\ILTools_x64.ja-JP/.JPN/.JP` を FindFirstFileExW で列挙している(言語別のリソース DLL の探索。無い)。msgothic.ttc の読み(2,123,288 B)と OpenSans-Regular.ttf(217,360 B)・ILCursors.ilfont(7,892 B)はその直後。詳細は `exports/trace/p0-font-summary.txt`。

**P0(project なし、起動〜90 秒)と seed の突き合わせ**(`tools/trace3-vs-seed.py`、`exports/trace/p0-font-vs-seed.txt`; p0-font のログ。FL の install 配下を CreateFileW で開いた異なるファイル **87 個**のうち、`vfs-seed-idle1.tsv`(FL 配下の 237 行)に**ある 44 個、無い 43 個**): 無い 43 個は、(1) `FLEngine_x64.dll`(22,227,016 B 中 24,134 B を読む)と `FL64.exe`(597,064 B 中 12,067 B を読む)— PE のヘッダ等; (2) **`System/Config/Thumbnails/edition-{allplugins,producer,signature,fruity}.png`(合計 566,592 B、全体を読む)**と `System/Hardware specific/Forward CCs to current plugin port 10/device_Forward CCs to current plugin port 10.py`(1,029 B); (3) **`Data/Templates/**/*.nfo` の 36 個(各 68〜162 B、合計 4,063 B)**。(2)(3) は project なしで出る Welcome 画面(新規 project の一覧・エディションの選択)が読む物と読める。seed を作った時の idle(project を読み込んだ状態)では開かれていないので seed に無い。製品が project を指定して開く経路では、Welcome 画面は出ない/出る時期が違うかもしれない(未確認)。出るなら、これらは遅延取得になり、4 本の PNG(566 KB)が一番大きい。

## 17. G9 の Filter の判別: プロセッサ数の実験(E1・E2)と ins1-lp の再現・ins1-bp(PM 10:1x の指示。11-07 10:19〜10:29)

**実際に実行した物**: Wine の Export を 5 本(prefix-e2、config A、mp3wav-all、SSE2 の cpuid、`variants/ins1-lp.flp`〔alpha1、sha e7ab292d…〕/`ins1-bp.flp`〔ffbc924d…〕): E1 `elog16-ins1-lp-cpu1`(10:19:30〜10:21:40)、E2 `elog16-ins1-lp-cpu4`(10:22:11〜10:24:01)、再現 `elog16-ins1-lp-r2`(10:24:19〜)、`elog16-ins1-bp`(〜10:29:02)。それぞれの前に小さい exe `stub/sysinfo.exe` で Wine が返すプロセッサ数を確認した。FL は `wineserver -k` で終了。各 run の後、FLP は d6783e08886b4b48、:88 の窓 0、guard は従来どおり。

- **Wine のプロセッサ数の見え方(実測)**: (1) `taskset -c 0 wine sysinfo.exe` → `GetSystemInfo.dwNumberOfProcessors`=**16**(`GetProcessAffinityMask` の process=0x1、system=0xffff)。(2) `WINE_CPU_TOPOLOGY=1:0 wine sysinfo.exe` → 16。**この Wine 10.0(Ubuntu 10.0~repack-12ubuntu1)の中に `WINE_CPU_TOPOLOGY` という文字列がどのファイルにも無い**(`grep -rl`、0 件)。→ 0307 §2c の hideT1 は無効な実験だった(0307 に訂正を追記)。(3) Wine の ntdll.so は `sysconf` と `/sys/devices/system/cpu/{present,online}`(`fopen`)を読む(import 表)。そこで `LD_PRELOAD=stub/libfakecpu.so FAKE_NCPU=N`(sysconf と上の 2 つの sysfs の読みを N に差し替える共有ライブラリ、16,064 B)を **wine・wineserver・FL にだけ**掛けた(`run-fl.sh` の `OPT1_WINEENV`、`OPT1_TASKSET`)。`FAKE_NCPU=1` + `taskset -c 0` → dwNumberOfProcessors=**1**、%NUMBER_OF_PROCESSORS%=1、GetLogicalProcessorInformation 7 項目・1 コア(`GetActiveProcessorCount(ALL)`=2 = 同じコアの HT の兄弟)。`FAKE_NCPU=4` + `taskset -c 0-3` → 4・4・11 項目・2 コア。
- **WAV の事実**(`tools/repeat-table.py`):

| タグ | 条件 | フレーム | sha256(16) | RMS |
|---|---|---|---|---|
| elog16-ins1-lp | 16 CPU(08:28) | 5,729,968 | f1c1012d5a21a6c2 | 18649.4690 |
| elog16-ins1-lp-cpu1 | 1 CPU(taskset 0 + FAKE_NCPU=1) | 5,729,968 | 22a57da5f99721ed | 18649.4690 |
| elog16-ins1-lp-cpu4 | 4 CPU | 5,729,968 | bece5a04a504424a | 18649.4690 |
| elog16-ins1-lp-r2 | 16 CPU(再現) | 5,729,968 | 8456b34273fd0274 | 18649.4690 |
| elog16-ins1-bp | 16 CPU | 5,716,325 | 682dfa8ce01db575 | 18708.9374 |

 ins1-lp の 4 本の対ごとの差: RMS 0.0005〜0.0009、差のあるサンプル 3〜9 個、**最大差 1(±1 LSB)**(最初に違うフレームは 10.583 s・86.325 s・120.766 s 等、効果の区間とは無関係)。= **決定的**。ins1-bp は elog16 より +414 フレーム、elog16 との差 RMS 11124.5984。
- **Filter の素通し/効果の始まり**(alpha1 の `a22-filter-structure.py` と `tools/e1-summary.py`): 全部の ins1-lp で、効果の最初のフレームは **211,681(4.8000 s)**(wasm は 210,723)、R1(frame 210,723〜211,680)は基準(elog16)と **100.0% 一致・無音 0.0%**(素通し)、R2(100.47〜100.80 s)は基準と 98.7% 一致・無音 0.0%、wasm cand1 ins1-lp との差 RMS 5325.6。ins1-bp は、効果の最初のフレーム 211,681、R1・R2 とも基準と 100.0% 一致、wasm cand1 ins1-bp との差 RMS 5154.9。出力: `exports/trace/filter-structure-elog16-ins1-lp-cpu1.txt`・`-ins1-bp.txt`。
- **読み**: Wine の Filter の素通し/効果の始まりは、プロセッサ数(Wine が返す値・affinity)にも、再実行にも依らない。wasm の『4.778 s からの無音・100.58〜100.80 s の無音』は、Wine の側ではプロセッサ数を 1 にしても出ない。= プロセッサ数の仮説は Wine 側では否定された(cand1 が GetSystemInfo=1 を返すことの影響は、この実験では測れていない — Wine 側を 1 にしても Wine の出力が変わらなかったことだけが事実)。
- **ドライバの修正の確認**: 再現 r2 と bp は、Return の前にポインタを窓の中央へ動かす修正で、Return が効いて Save dialog に進んだ(events: 『Return pressed (the pointer was moved over the dialog first …)』)。

## 18. RT-A の first idle(P1)が E13 より遅い理由の切り分け — 読みのみ(PM の指示 11:2x、期限 11:50。11-07 11:1x〜11:4x)

**実際に実行した物(読みのみ)**: Wine 0、Chrome 0、build 0、wasm の実行 0(**私は何も実行していない**)。読んだ物: ci1 が実 Chrome 156・この機械・headless で実エンジンを走らせた記録 = `lift/out-ci1/kit/out/perf-*.jsonl` の first-idle の beacon(Chrome・network 由来・cold の run だけ。Firefox と storage-cache は除いた)、`out/*/report.json` の timeline、`out/*/engine-log.log`、`out/idle-chrome-e13-vs-rta.tsv`、`out/msg-rta-idle.md`、各 bundle の manifest。mt3 の `tree-rt1`(`build-engine-bundle.sh`、`link-web.sh`、`runtime/fle_gil.h`)と各 bundle の wasm の import section(shared memory の flag だけ)。数値は私の python での集計で、**測定は ci1・mt3 の物**(私は測っていない)。

### 18.1 表: 段階ごとの中央値 [最小..最大](ms。beacon の timeline。Chrome・network 由来)
| bundle(wasm の sha、wasm の大きさ) | n | download_done | compile_done | main_start | idle(first idle) | main_start→idle |
|---|---|---|---|---|---|---|
| E13(`58ad4a6a`、143.1 MB、7 module) | 44 | 1,114 | 956 | 1,241 | **4,754** [4,413..7,576] | 3,519 [3,243..5,229] |
| cand1(`0e9aeaca`、167.0 MB、9 module、協調、`FLE_DIAG_*`=1) | 4 | 1,366 | 1,219 | 1,502 | **7,015** [6,445..7,843] | 5,526 [5,057..6,136] |
| RT-A(`09078938`、158.4 MB、9 module、実スレッド) | 16 | 1,367 | 1,221 | 1,510 | **8,180** [7,673..8,847] | 6,679 [6,283..7,185] |
| RT-A + seed2b(seed +2.3 MB) | 5 | 1,384 | 1,176 | 1,518 | 7,809 [7,752..8,892] | 6,387 [6,234..6,775] |
- ci1 の交互の対(5 組、10:34〜10:57): E13 5,312 対 RT-A 7,813、差の中央値 +2,515(5 組すべて RT-A が遅い)。上の beacon 全体では +3,426(8,180 − 4,754)。E13 の最小(4,413)と RT-A の最小(7,673)は重ならない = **差は雑音ではない**。
- 同じ page の beacon で、`wasm_instantiated→main_start` は 3 群とも 13〜16 ms、`audio.lastResume.ms`(約 1,500)も 3 群で同じ = **audio の resume 待ちは E13 にもあり、差の原因ではない**。

### 18.2 差の内訳(RT-A − E13 = +3.4 s。beacon)
| 区間 | RT-A − E13 | 割合 | 読み(事実と仮説を分ける) |
|---|---|---|---|
| main_start まで | +269 ms | 8% | 事実: download_done +253、compile_done +265、その後(instantiate〜main_start)は同じ。原因は取得量: 3 資産の wire は E13 43.7 MB 対 RT-A 81.7 MB(wasm の br 19.8 対 39.5 MB、DLL pack 19.4 対 33.9 MB、seed 4.4 対 8.2 MB)。loopback で +0.25 s。**実網では +38 MB が効く**。仮説: テスト bundle の brotli の品質が低い(未確認) |
| main_start→idle | +3,160 ms | 92% | 下の 2 段 |
| 〃 ① E13 → cand1(協調のまま) | +2,007 ms | 約 63% | **RT でない部分が過半**。cand1 は 7→9 module の bundle |
| 〃 ② cand1 → RT-A | +1,153 ms | 約 37% | RT 固有。cand1 の 4 本が 5,057〜6,136 と広く、report.json の組(下)では +0.5〜+1.7 s の幅 |
- report.json の階段(m→i。シナリオ run なので beacon より E13 が高い): 7 module(export-ref 4,106 / alpha7 3,955)→ **8 module の g9a 3,980(Balance を足しても +0)** → cand1 5,968 / 6,137 / 5,083 / 5,058 → RT-A 6,750(smoke)/ 6,633(Export N=8)/ 6,724(n=2)。song(11:14〜11:29): E13 4,221、RT-A 7,186、RT-A の Workers=2 6,969。
- ①は『Filter を足したから』と読み切れない。`tree-g9a` と `tree-g9f-cand1` の `lift/runtime` は **21 ファイルが違う**(g9a と `tree-g9f` は 5 ファイル、g9f と cand1 は 20 ファイル)。cand1 には窓などの他の修正が入っており、Filter 単独の費用は**未分離**。

### 18.3 原因の候補(順位は読み。全て未測定の仮説)
RT 固有の ②(+1.2 s)について:
1. **GIL の wrapper と TLS(`-DFLE_GIL -DFLE_TLS_MORE`、RT-A だけ)** — 本命。`fle_imp_*` を呼ぶたびに CAS(取得)+ SEQ_CST の exchange(解放)+ owner の書き込み + `fle_worker_id`(thread-local)の読み。MXCSR・x87・landing・SEH も TLS の block 経由。Worker が 1 本の起動中は競合しないが、**呼び出しの回数に比例**して全部の main_start→idle に掛かる。根拠: 設計の記述(`fle_gil.h`: 『guest が runtime を密に呼ぶ』)と、下の 3・4 を除いた残り。呼び出し回数と 1 回の費用は**未測定**。
2. **0312 の stack の他の部分**(fix17、D3a、S1 stage C の audio の `ensure_init` が page へ往復する)— 中。cand1 との差は runtime で 43 ファイル。個別の費用は未測定。
3. pthread の on-demand の生成(`PTHREAD_POOL_SIZE=0`。N=8 で『pool is exhausted』7 回、N=2 で 1 回)— **主因ではない**。N=2(`FLE_NUM_CPU=2`)の idle は 8,323(m→i 6,724)対 N=8 の 8,065(6,633)、song は 8,699 対 8,848 = 差 ±0.3 s 以内(各 n=1 なので小さい差は読めない)。
4. S3a の `GetSystemInfo`=hardwareConcurrency(16)— **主因ではない**: 3 と同じ n=2 の run が同じ時間。
5. shared memory・atomics・JSPI・pthread の libc — **差ではない(事実)**: E13・g9a・cand1・RT-A の 4 つの wasm はどれも `env.memory` を **shared(flag 3、初期 10,240 ページ = 640 MiB、最大 32,768 ページ)**で import している。
RT でない ①(+2.0 s)について: (a) Filter module の init / plugin scan での実行(Balance は +0 なので『module を 1 つ読む費用』ではない読み。Filter 固有の仕事の可能性)、(b) cand1 の `FLE_DIAG_WIN/SCROLL/VFS_MISS=1`(engine log に `fl-diag` 1,650 行 + `g9-diag` 103 行。RT-A には無いので、cand1 を押し上げ ② を過小にしている可能性)、(c) g9f→cand1 の runtime 20 ファイル。いずれも未測定。

### 18.4 切り分けの実験(Wine なし。安い順。実行は ci1 / mt3 の Chrome の窓)
1. **既存 bundle の交互の対**(build なし): g9a(`fl-engine-test-g9a-28f6f631`)・g9f(`fl-engine-test-g9f-7b2e3014`)・cand1・RT-A を同じ page・同じ条件で、各 5 回、順番を回して first idle。→ ①の Filter 単独(g9a→g9f)と、他の 20 ファイル(g9f→cand1)と、②(cand1→RT-A)を同じ負荷で分ける。
2. **cand1 の manifest から `FLE_DIAG_*` を外す**(manifest の env だけ。build なし)→ (b)の大きさ。
3. **RT-A を `FLE_WORKERS=1`(または 0)で**(manifest の env だけ。build なし。`tree-rt1/lift/runtime/fle_thread.c` の 350〜362 行を読んだ事実: `FLE_WORKERS` が最優先で、0 か 1 だと `rt_mode=false` = 協調の scheduler。GIL の wrapper と TLS は build に入ったまま、`fle_gil.h` も『Worker が 1 本でも engine 全体が lock を取って動く』と書く): → ②が GIL・TLS の定常費用なら cand1 との差(+1.2 s)がそのまま残り、Worker の生成・実スレッド固有の費用なら消える。**1 本で ②の過半が切り分かれる最も安い実験**。
4. **Chrome の CPU profile**(CDP の Profiler。engine Worker を auto-attach)を E13 と RT-A で main_start→idle の間: wasm の関数は名前が無いので link map(`link.map`)で引く。`fle_gil_*`・`fle_imp_*` の合計割合が出る。
5. **GIL の取得回数の counter**(mt3 の build が要る。`FLE_STAT_ACQ()` は空の macro): first idle までの `fle_imp_*` の呼び出し回数 → 1 と合わせて『回数 × 費用』を数にする。
6. (診断だけ)Chrome の `--js-flags=--no-wasm-dynamic-tiering` など: TurboFan への tier-up の待ちが全 build に共通で効いているか(owner は flag を渡せないので製品の対策ではない)。

### 18.5 緩和案(提案だけ。lift の忠実性の方針に従い、局所の特例でなく『通信・変換・待ちを減らす』形)
- **M1 GIL の定常費用を減らす**(1 が確定したら): 『Worker が 1 本のあいだ lock を取らない』近道は、lock の不変条件(Worker をまたぐ共有状態の保護)を mt3 が確認してからでないと入れない。代案: wrapper の粒度を粗く(密な呼び出しの連続を 1 回の取得にまとめる)。
- **M2 pthread の pool の前もっての生成**(`-sPTHREAD_POOL_SIZE=N-1`、または page が download / compile の 1.2 s の待ちの間に Worker を作る): 生成を first idle の経路から外す。3 の通り今の効果は小さい見込みだが、『pool is exhausted』(deadlock の警告)の危険を除く。
- **M3 テスト bundle の br の品質を製品と同じに**: wire 81.7 → 約 44 MB(実網の P1 に効く。loopback で約 −0.25 s)。engine でなく build の道具の話。
- **M4 module 数に対する費用の確認**: 製品は module がこの先も増える。Balance +0、Filter(+他)+約 1〜2 s という段差が module ごとに出るなら P1 の 10 s を割る(RT-A の最大 8,847 で余裕は約 1.2 s)。実験 1 の結果で、module 1 つあたりの費用を先に数にする。
- **M5 wasm の compile の共有**(compiled Module / code cache の共有): compile_done は 1.0〜1.2 s で download と並行なので first idle の主因ではない(主因は main_start 以後)。優先度は低い。

### 18.6 読みの限界
cand1 は n=4(5,057〜6,136 と広い)。E13 の beacon(n=44)と RT-A(n=16〜21)は 5053〜5106 の複数の port・時刻にまたがり、負荷・同時に走った job(cargo など)は run ごとに違う(交互の対の表には開始時の load がある: E13 側 4.8〜8.0、RT-A 側 2.5〜7.5 で、E13 側のほうが高く、RT-A が遅い向きとは逆)。①と②の割合は ±1 s の幅で読む。Filter 単独・GIL 単独の費用は**どれも測っていない**。

## 19. E4: alpha1 の minimal `.reg`(wasm 相当のレジストリ)を入れた Wine の ins1-lp の Export(PM の GO、11-07 11:41〜11:46)

**実際に実行した物**: 実 Wine の FL Studio(prefix-e3 = 設定 A のクローン)で ins1-lp の Export を 2 本、完走(`tools/e4-run.sh`。Xvfb :88、`wineserver -k` で終了、Chrome は gpu1 の非計時の物だけ重なった)。`e4-ctl` = `.reg` なし(対照)、`e4-reg` = `lift/out-git/alpha22-results/tables/0319-wasm-like-registry-minimal.reg`(sha 8257d296740354c1)を `wine regedit /S` で入れた後。適用後・FL 起動前の user.reg は `Image-Line` の section が 24 → 2(ASIO・FL Studio 26\Devices・Shared\General・Shared\Plugins は 0)、差は 136 行。実行後に e3 の 3 つの .reg を戻し、plugin 木・状態は前後で一致。

| run | WAV | sha256(先頭 16) | 対照との差 | 08:28 の `elog16-ins1-lp` との差 | [210723,211680) | 99.77〜100.8 s |
|---|---|---|---|---|---|---|
| e4-ctl | 5,729,968 フレーム(22,920,056 B) | 99ee1147fd242831 | — | 3 フレーム ±1 LSB | 0 でない(Σ\|x\| 2,028,392) | 無音でない(max\|x\| 32,767、0 のフレーム 0.3%) |
| e4-reg | 同じ | 5f5cae9d403bca36 | 5 フレーム ±1 LSB | 4 フレーム ±1 LSB | 0 でない(同じ) | 無音でない(同じ) |
- `a22-filter-structure.py`: 2 本とも Wine の最初の効果フレーム 211,681(4.8000 s)、wasm の変種が厳密に 0 になる 2 区間は Wine では 0 にならない。**minimal の registry を wasm 相当にしても、Wine の Filter の pass-through と 100.5〜100.8 s の無音は再現しない**(この minimal の範囲、各 n=1)。
- 限界: full の `.reg` と原 FLP の基準は未実行。FL が実行中に registry へ書いた内容は見ていない。後始末: FLP sha d6783e08886b4b48、Wine 0、:88 に残った窓 0、prefix-e3 は設定 A に復元。証拠: `exports/runs/e4-ctl.*`・`e4-reg.*`、`exports/trace/filter-structure-e4-*.txt`、`e4-e4-reg-user.reg.applied`(opt1-wine 側)。


## PM の決定と取り込み(2026-11-07 08:10、docs のみ。製品の差分なし)

取り込み: opt1 の 0320(原本の install tree の棚卸し、索引の契約検査、V0 の生成器、Wine の run 待ち行列)と表・道具。**PM の確認**: 表 `tables/0320-install-tree-files.tsv.gz` は 27,822 行・合計 2,053.26 MiB。V0 の出力 `~/work/opt1-wine/vfs-out/run-a` は 888 ファイル(chunk 885 + index.bin + index.br + manifest)・938.1 MiB、index.br 215,576 B、run-a と run-b で manifest がバイト一致・全ファイルのハッシュの総和が一致。elog16(Wine、Balance+Filter 有効)は 5,715,911 フレーム、elog15b との差 3 サンプル(±1)・wasm G9-a の Export との差 2 サンプル(±1) = この参照プロジェクトでは Fruity Filter の効果が Export にほぼ現れない(変種 FLP が要る)。判断: 対象は install tree 全域、lift していない DLL の中身は配らない(暫定)、配布は chunk の不変オブジェクト(mt3 の 0313 §18・§19)。依頼者の機械は 0 件。


### PM の追記(2026-11-07 08:58、再取り込み: §8.6〜§11。docs のみ。Wine の実行は opt1 の報告)

- **PM の確認(numpy、opt1 の WAV を直接読む)**: `elog16.wav` 5,715,911 フレーム・RMS 20412.1782・sha 先頭 f102311b4db54e57。`elog16-solo12.wav` 5,715,911 フレーム・RMS 16256.2513(sha 先頭 9172ab0ad9224608)、elog16 との差 RMS 13684.17(= Solo は WAV に大きく出る)。`elog16-ins1-lp.wav` 5,729,968 フレーム・RMS 18649.469(sha 先頭 f1c1012d5a21a6c2)、elog16 との差 RMS 12697.6647(先頭 4.8 s は同一)。`variants/original.flp` sha 先頭 d6783e08886b4b48、`variants/solo12.flp` 先頭 8bfab4682fb21967、どちらも 1,322,619 B。
- **訂正の採用**: Parametric EQ 2 は隠していない `Effects\\Frequency Splitter\\engine_x64.dll`(Intel IPP、cpuid あり)を読み込むため、elog18・elog19 は実 CPU の cpuid が混じった run だった。**基準は elog22(PEQ2 clean)・elog23(Frequency Shifter clean)を使う**。以後、run ごとに modload の DLL を静的に点検する(`modload-cpuid-check.py`)。『proj/serina1-serum2.flp の sha が ca4ad59f…』は run の間だけ差し替わる mp3wav-all.flp の sha で、**真の原本は variants/original.flp**。
- **FL のミキサーのスレッド(§10。既存の threads.tsv の表。新しい Wine の run なし)**: Export の間、1 本が 0.96〜0.99 コア、2 位が 0.21〜0.48、3 位が 0.21〜0.32、平均 2.2〜2.35 コア(16 コア中)。再生(60 s、1 本の run)は 1 本 0.97 + 2 本目 0.55、平均 1.84 コア。**Settings では Multithreaded generator/mixer processing は有効(`MultithreadFXBox="1"`、`MultithreadGenBox="1"`)だが、複数の重いスレッドは出ていない**(FL が見たプロセッサ数は未測定)。**PM の読み**: 原本の FL でもこのプロジェクトの重い仕事はほぼ 1 本 + 軽い 2 本目で、ネイティブが 1.84 コア分の仕事をしているのに対し、協調の単一 Worker は全部を 1 本に載せている。実スレッド版(RT-A)で 2 本目以降を別 Worker に出すことの意味の根拠だが、効果は P3 の測定(S3a)で確かめる。限界: Export は再生と別経路、再生の run は 1 本、他の job の負荷あり、スレッド名なし。
- **解決済みでない物**: R6(Soundgoodizer + FLEX)の Problems 窓が『2 plugin(s)』(期待は 1)の理由、最終の片付け(variants/mp3wav の削除、prefix-e2/e3 の扱い)、O16 の J0〜J3(Wine の +font/+text ログ)、ファイル trace(P0〜P4)は未実施。Wine は Chrome 0 本と PM の合図が揃うまで起動しない。


### PM の追記(2026-11-07 11:55、docs のみ)
- **E4 は否定**(0319 の PM の追記を参照): Wine の ins1-lp は minimal `.reg` を入れても対照と同じ。opt1 の WAV 2 本(e4-ctl 99ee1147…、e4-reg 5f5cae9d…)を PM が numpy で確認。
- **§18(RT-A の first idle が E13 より遅い理由)**: E13(n=44)の first idle 中央値 4,754 ms、cand1(協調)7,015 ms(n=4)、RT-A 8,180 ms(n=16)。差のうち main_start までが +0.27 s(wire +38 MB)、main_start→idle が +3.16 s で、その約 6 割は cand1(まだ協調)の段階で出ており、実スレッド固有はその残り約 4 割(仮説: GIL の wrapper と TLS)。pthread の on-demand 生成は主因でない。全て未測定の仮説で、切り分けの実験(RT-A の FLE_WORKERS=1、4 bundle の交互、cand1 の診断なし)を ci1 の手順書(`ci1-results/0332/IDLE4-PROCEDURE.md`)に入れた。
