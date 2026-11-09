# 0309(仮)G9 の入口 = FL 内蔵 plugin を鳴らす最小の道の見積り(alpha1 = alpha21)

作業者 `alpha1`(成果物名 alpha21)、2026-11-06 JST 18:40〜。依頼: PM(決定 0309 仮、第 1 報 20:00、最終 20:45、out `lift/out-git/alpha21-results/`、Chrome 無し、読みと static/node、heavy ≤ 1)。基準: main a977401d(E12)。区分: **実測**=本書の道具の出力・ファイルの内容、**読み**=解釈・見積り、**未確認**。**この機械での静的な解析と文書の読みで、新しい wasm/Chrome の run は 0 本。依頼者の機械の値は 0 件。**

**実エンジンについて(先頭)**: 本書は**何も実行していない**(実エンジンの実行を主張しない)。入力は (a) 実 DLL(Wine のインストール先 `Plugins/Fruity/{Effects,Generators}/…`、opt1 が 0307 で `*.off-0307` に退避した物を**読み取りだけ**。FLEngine_x64.dll は `binaries/win64` と sha256 一致)の PE の静的な解析、(b) `lift/runtime`・`lift/crates/fe-lift`・`app/scripts/build-engine-bundle.sh`・`ci/` のソースの読み、(c) 既存の決定・調査文書(0183 wrp1、0236、0230、plugins-distribution.md、m2-inventory.md、g8-project-load.md、performance-targets.md)の数。代役・再生・期待値なし。

## 0. 結論(先頭)

1. **『8 種のうち 1 つを LoadLibraryW で開く』のために足す物は、runtime の機構としては既にほぼある(実モジュール + パス表 = 決定 0183 の wrp1)。足りないのは (i) モジュールの追加の手続き(CI の抽出行・モジュール表・基底・フック点・RM_MAX の拡張)、(ii) volume 上のファイル、(iii) lift した plugin が実行時に踏む壁(未実装の import 行 74 など)。ホスト側(TFruityPlugHost の vtable)と音声のブロック処理の呼び出しは FLEngine のゲストコードどうしの呼び出しで、runtime の実装は要らない**(読み。§2)。
2. **最小の候補は Fruity Balance / Filter / Delay 2 の 3 つ**(**今の fe-lift で実際に lift した実測つき**: 5 種〔Balance・Filter・Delay 2・PEQ2・Frequency Shifter〕とも 1.1〜1.2 s・RSS 112〜124 MB で通り、**bad_nodes 0・命令レベルの未対応 0・行なしの import の thunk は 5 種とも同一の 44**〔comctl32 の ImageList 22・user32 9・kernel32 5・ole32 4・shell32 3 ほか〕。§1.3): 3 つとも (a) インポート 543(うち行あり 469、行なし 74 は**3 つで完全に同じ集合**で、すべて FLEngine の import の部分集合)、(b) `ildsp_x64.dll` への参照が無い(PEQ2・Frequency Shifter・Soundgoodizer・FLEX は文字列に有る)、(c) DLL 3.7〜3.8 MB。**Wrapper を要さない**。§1。
3. **追加の wasm の大きさ(Balance 1 つ)**: 共有なし(今すぐできる形)で wasm + データ **14.6 MB**(DLL の 3.9 倍)、FLEngine との共有(B-FLE。生成器の変更が要り未着手)なら **3.3 MB**(0.88 倍)。6 種(Balance・Filter・Delay 2・PEQ2・Frequency Shifter〔+ Engine〕・Soundgoodizer)で共有なし **+101 MB**(S1 の全体比 2.37 倍 = 2 倍を超える)、共有あり **+29 MB**(1.67 倍)。FLEX は MSVC の engine(8.1 MB の DLL、OpenGL、AVX/AVX-512 必須)が別に 44.5 MB 級で、7 種全部で共有なし 2.66 倍・共有あり 1.91 倍。**ただし Frequency Shifter の Engine と FLEX の engine は fe-lift の VEX/EVEX 対応が無いと動かないので、この 2 つを含む合計は大きさの見積りだけで、当面は載せられない**(§1.3)。§3。
4. **S1・P1・転送量**: 共有なしで入れてよいのは Balance **1 つだけ**(全体比 1.94 倍。2 つ目で 2.03 倍 = 2 倍超)。P1(起動 10 s 以内)への影響は、静的に 1 つの wasm へ結合する今の方式では**起動のたびに全 plugin 分を取得・コンパイルする**ので、Balance 1 つで +0.14 s(コンパイル。9.4 ms/MB の旧実測で比例換算)、6 種で +0.95 s、7 種全部で +1.57 s(first idle 5.1〜6.1 s の上に。10 s 以内には収まるが余裕を使う)。転送量は brotli で Balance +1.4 MB(今 約 15.5 MB の +9%)、7 種全部で +15.7 MB(2 倍)。§3。
5. **段階表と人日の見積り(読み、根拠は wrp1 の実績。確度は低い)**: G9-0 準備 1 d、G9-a(Problems 窓の行が 1 つ減る)**2〜4 d**、G9-b(音が出る)+1〜3 d、G9-c(GUI が出る)+2〜5 d、同系の次の plugin(Filter、Delay 2)は各 +0.5〜1.5 d、`ildsp` が要る 4 種(PEQ2・Frequency Shifter・Soundgoodizer・FLEX)は ildsp の lift の可否で +3〜8 d の不確実性。**Frequency Shifter は別の DLL `Engine_x64.dll` が VEX のスカラー倍精度と FMA(692 箇所、vmulsd・vaddsd・vfmadd213sd…)を持ち、fe-lift の VEX 対応(移動 4 命令のみ)の拡張が先に要る(+2〜4 d の読み)。FLEX の engine は AVX の 256 ビット float(engine_x64: 18,772 箇所)と AVX-512(engineV2: 190,302 箇所)で、VEX/EVEX の lift の事業(0308 §9)が前提 = 当面は不可**(OpenGL の橋も別に要る)。最小の 1 つで音まで **約 1 週間(4〜8 d)**、GUI まで 約 1.5 週間。§4。

## 1. (1) 8 種の追加 wasm の大きさと、runtime の未実装 import(plugin ごと)

### 1.1 大きさ(`tables/0309-plugin-sizes.tsv`。倍率は plugins-distribution.md の実測の倍率を使った**読み**)

| plugin | DLL(B) | `.pdata` 関数 | 共有なしの追加 wasm + データ(読み) | brotli(読み) | B-FLE(FLEngine と共有。読み) | 倍率(共有なし / B-FLE) | import(行あり / 行なし) |
|---|---:|---:|---:|---:|---:|---|---|
| Fruity Balance | 3,718,656 | 11,736 | 14.59 MB | 1.43 MB | 3.28 MB | 3.92 / 0.88 | 469 / 74 |
| Fruity Filter | 3,780,608 | 11,737 | 14.93 MB | 1.46 MB | 3.63 MB | 3.95 / 0.96 | 469 / 74 |
| Fruity Delay 2 | 3,731,968 | 11,760 | 14.68 MB | 1.44 MB | 3.32 MB | 3.93 / 0.89 | 469 / 74 |
| Fruity Parametric EQ 2 | 4,036,608 | 12,317 | 16.29 MB | 1.58 MB | 4.62 MB | 4.04 / 1.14 | 474 / 74 |
| Frequency Shifter(+ Engine_x64.dll 263,168 B) | 3,876,864 | 12,148 | 15.46 MB(+ Engine 約 1 MB の推定) | 1.51 MB | 3.77 MB(+ Engine) | 3.99 / 0.97 | 474 / 74(Engine: 73 / 0) |
| Soundgoodizer | 6,407,680 | 17,386 | 24.39 MB | 2.47 MB | 9.27 MB | 3.81 / 1.45 | 531 / 107 |
| FLEX(+ engine_x64 5.67 MB / engineV2 8.12 MB、MSVC) | 5,194,752 | 16,125 | 21.32 MB(+ engineV2 44.5 MB) | 2.05 MB(+ 3.6 MB) | 6.67 MB(+ engineV2 約 44.5 MB) | 4.10 / 1.28 | 530 / 88(engine: 233 / 109) |
| Fruity Wrapper(第三者 VST の入れ物。後回し) | 5,997,056 | 19,816 | 23.16 MB | 2.29 MB | 5.58 MB | 3.86 / 0.93 | 519 / 112 |

- 共有なし = `wasm Oz = 0.368 × C`、`C = 21.0 × コード B`(Delphi)、wasm = 7.74 × コード B、+ 非ゼロのデータ(plugins-distribution.md §4.3)。brotli = 0.0717 × wasm + 0.30 × データ。**plugin のコードで同じとは限らない(推測)。実際に wasm にした物は 0 件**(plug1 が `fe-lift --pdata` を 20 個の plugin で走らせて生成 C の大きさを実測しただけ。wasm は未ビルド)。現行の直 wasm 生成(fe-wasmgen、混成)を使えば小さくなりうる(E12 の全体は 143 MB でネイティブ 8 モジュール合計 77.4 MB の 1.85 倍。plugin にも同じ圧縮が効くかは未測定)。
- B-FLE = FLEngine に同一の関数を共有し、FLEngine に無い物だけを足す(plug1 の `vs-engine-per-dll.tsv` の残り)。**生成器の変更(関数のデータ参照を表経由にする)が要り、未着手**(plugins-distribution.md §5.2 B-FLE。作業の大きさは私は見積もっていない)。既存の 3 つの Delphi 製モジュール(FLEngine・ILTools・QFC)との共有の程度は、m2-inventory.md §4.3: 小さなエフェクトで関数の 97〜99%・コードのバイトの 88〜96%、大きなシンセ(FLEX など)で関数の 89〜93%・バイトの 67〜82%。
- 『共有ありの現実的な追加サイズ』は、**今すぐできる形は共有なし**(上の『共有なし』の列)。共有ありは生成器の変更の後の見込み。

### 1.2 runtime が未実装の import(行なし。`tables/0309-imports-summary.tsv`、`0309-imports-none.tsv`)

- 『行』= fe-lift の `TABLE`(609 行)+ `ship5-imports.txt`(出荷の F2xH5、209 行の追加)= 818 の (DLL, API)。『行なし』の呼びは生成 C で `unsupported instruction` の停止になる(0210)。
- **Delphi の 3 つの小さな plugin(Balance・Filter・Delay 2)は行なし 74 で、集合が同一**(`beyond Balance = 0`)。PEQ2 と Frequency Shifter も同じ 74(インポート総数が 548 と 5 多いが、増分は行あり)。Soundgoodizer は 107(Balance + 33)、FLEX は 88(+ 14)、Wrapper は 112(+ 38。うち FLEngine に無い物は `CreateJobObjectW`・`SetInformationJobObject` の 2 つ)。
- **8 つの Delphi plugin のインポートはすべて FLEngine のインポートの部分集合**(Wrapper の 2 つを除く): FLEngine の lift が既に同じ行なしの集合 74〜107 を抱えている = 実行されなければ停止しない。**Balance の 74 のうち、plugin を開く経路で実行される物は未確認**(実行しないと分からない)。DLL 別(Balance): user32 24、comctl32 22、imm32 7、kernel32 6、ole32 4、uxtheme 4、shell32 3、msimg32 1、windowscodecs 1、wtsapi32 1、api-ms-win-crt-string 1。comctl32 の 22 は ImageList 系(0210: ImageList 23 API が行なし)。
- **MSVC の engine**(FLEX の engine_x64 / engineV2: インポート 338 / 342、行なし 107 / 109 で、うち FLEngine に無い物が 95): `opengl32` 36(`glBegin`・`glBindTexture`・`glClear`…と `wglCreateContext` 系)、`d3d9!Direct3DCreate9`、`dbghelp` 6(`StackWalk64`・`SymInitialize`…)、user32 27〜28、kernel32 14 ほか。= **FLEX は OpenGL(WebGL への橋)と dbghelp の行が要る別の仕事**。Frequency Shifter の Engine_x64.dll(263 KB)は 73 すべて行あり。
- **ildsp_x64.dll(4.2 MB、`Limiter`・`PSOLA`・`Shimmer`・`StereoEnhancer` 等の DSP のクラス)**: PEQ2・Frequency Shifter・Soundgoodizer・FLEX の DLL の文字列に `ildsp` が有り、Balance・Filter・Delay 2・Wrapper には無い(UTF-16 の文字列検索。**呼ばれるかは未確認**)。ildsp は未 lift(g8-project-load.md §5: x87 944・AVX-512 23,929 命令、`LoadLibraryW("ildsp")` を NULL 許容の分岐で呼ぶ 〔決定 0113〕)。= **PEQ2 以降は ildsp の扱い(NULL のまま通るか、lift するか)が分岐点**。
- **lift そのものは軽い(plug1 の census、11-03、HEAD eca8c0b。実測)**: `fe-lift lift --pdata` で Balance は **12,541 関数・bad_nodes 0・unsupported 97 箇所(種類 1 = import の thunk)・wall 2.1 s・RSS 60 MB・生成 C 35.3 MB**、Soundgoodizer は 18,372 関数・unsupported 147・3.6 s。= **命令レベルの未対応(VEX・x87 の例外など)は Balance の closure に 0**(unsupported はすべて行なしの import の thunk)。今の fe-lift での再実行はしていない(HEAD が進んでいる)。
- フック点(Delphi の共通): どの Delphi モジュールも静的初期化で 3 つの RTL 関数に inline hook を掛け、`VirtualProtect(target, 5, PAGE_EXECUTE_READWRITE)` を呼ぶ(0183 W1。FLEngine・QFC・ILTools と同じ unit)。**モジュールごとに 3 つの RVA が違う**(Wrapper は 0x1b700・0x39630・0x39650。Balance の同じ RVA の中身は別の関数 = RTL の置き場がモジュールごとに違う、実測)ので、`--hook-points` の RVA 表をモジュールごとに作る必要がある(Wrapper では『`f_5e91f0` の唯一の呼び元のバイト走査』で見つけた。自動化できる形)。

### 1.3 今の fe-lift での lift の実測(`lift/target/release/fe-lift`〔11-06 11:02 のビルド〕、`--pdata --extra-imports ship5-imports.txt`、出力は /tmp/opencode/a22 に書いて削除。`logs/0309-lift-census.txt`、`tables/0309-*-unsupported.tsv`、`tools/a23-unsupported-{thunks,mnemonics}.py`)

| モジュール | 関数 | 命令 | bad_nodes | 行なしの import の thunk(distinct) | 命令レベルの未対応 | 時間 / RSS | 生成 C |
|---|---:|---:|---:|---:|---:|---|---:|
| Fruity Balance | 12,549 | 412,943 | 0 | 44(44) | **0** | 1.13 s / 112 MB | 35.3 MB |
| Fruity Filter | 12,550 | 421,023 | 0 | 44 | 0 | 1.08 s | 36.0 MB |
| Fruity Delay 2 | 12,573 | 415,834 | 0 | 44 | 0 | 1.09 s | 35.6 MB |
| Fruity Parametric EQ 2 | 13,135 | 460,320 | 0 | 44 | 0 | 1.23 s | 39.4 MB |
| Frequency Shifter | 12,966 | 440,344 | 0 | 44 | 0 | 1.10 s | 37.7 MB |
| Frequency Shifter の `Engine_x64.dll`(MSVC) | 501 | 27,513 | 0 | 0 | **692**(VEX スカラー倍精度・FMA・VEX 移動: vmulsd 63、vmovdqa 59、vmovsd 56、vaddsd 52、vfmadd213sd 48、vsubsd 41、int3 40 …) | 0.06 s | 2.7 MB |
| Soundgoodizer | 18,380 | 669,406 | 0 | 72(71) | 0 | 1.89 s / 179 MB | 57.1 MB |
| FLEX | 17,251 | 607,856 | 0 | 54 | 0 | 1.75 s | 51.9 MB |
| FLEX の `engine_x64.dll`(MSVC) | 11,100 | 1,010,364 | 0 | 1 | **18,772**(VEX 256 ビット float: vaddps 2,844・vsubps 2,712・vmulps 2,655・vextractf128 1,486 …) | 2.84 s / 258 MB | 89.1 MB |
| FLEX の `engineV2_x64.dll`(MSVC) | 12,651 | 1,314,118 | 0 | 1 | **190,302**(EVEX の AVX-512: vmovups 17,968・vpshufd 9,630・vaddps 8,532 …と VEX、FMA) | 3.89 s / 336 MB | 115.8 MB |
| Fruity Wrapper(後回し) | 20,780 | 661,601 | 0 | 73(72) | 0 | 1.84 s | 56.4 MB |

- **Delphi 製の plugin 本体は命令レベルの未対応が 0**: 5 種とも行なしの thunk が**同一の 44**(comctl32: ImageList_Add/BeginDrag/Create/Destroy/DragEnter/DragLeave/DragMove/DragShowNolock/Draw/DrawEx/EndDrag/GetBkColor/GetDragImage/GetIcon/GetIconSize/GetImageCount/Read/Remove/SetBkColor/SetIconSize/SetImageCount/Write の 22、user32: CharToOemBuffW・DefFrameProcW・FindWindowExW・GetClassLongPtrW・IsDialogMessageA/W・MsgWaitForMultipleObjectsEx・ScrollWindow・TranslateMDISysAccel、kernel32: EnumCalendarInfoW・GetCPInfoExW・GlobalDeleteAtom・GlobalSize・HeapDestroy、ole32: CreateILockBytesOnHGlobal・CreateStreamOnHGlobal・ReleaseStgMedium・StgCreateDocfileOnILockBytes、shell32: SHAppBarMessage・SHGetDesktopFolder・Shell_NotifyIconW、memset〔api-ms-win-crt-string〕)。これは `--pdata` で全関数を根にした**上限**(`jmp [IAT]` の thunk だけ数える。`call [IAT]` の行なしは実行時の停止になり、この数に入らない = §1.2 の import 単位の 74 が上端)。Soundgoodizer は +28(印刷系 gdi32・winspool・CreateProcessW・OleGetClipboard 等)。
- **FLEX の 2 つの engine と Frequency Shifter の Engine は命令レベルの未対応が大量**: これらは `/arch:AVX` 以上でコンパイルされた MSVC 製で、**SSE2 に落ちる経路が無い**(CPUID の分岐でなく常に VEX)。= CPUID を SSE2 基準で答える現在の方針でも、これらの DLL を載せると必ず VEX/EVEX を実行する。**0308 §9 の VEX/EVEX の lift が FLEX と Frequency Shifter の前提条件**(Frequency Shifter の Engine は 692 箇所・約 14 種類の VEX スカラー + FMA で、境界のある小さな仕事)。
- lift そのものの時間・メモリは軽い(ビルドの `objects`/`link` は C の大きさに比例: §3)。

## 2. (2) FL が plugin を開く経路と、最小の 1 つに足す物(by-kind)

### 2.1 経路(読み。出典つき)

1. プロジェクトの FLP を解析してチャンネル/ミキサーのスロットに plugin を置く段で、`TPluginFormatFL`(RTTI 0xabe917、仮想メソッド)の `f_d969d0` → `f_d91280` → `f_d950d0` → **`f_d8cab0`** が『プラグイン名 + `_x64.dll`』を組み、**`LoadLibraryW` → `GetProcAddress("CreatePlugInstance")` → 呼び出し → 失敗なら `FreeLibrary`**(g8-project-load.md §4.7)。呼び元は仮想メソッドで、FLP の解析からの直接 call の経路は無い。
2. その前に FL は `GetFileAttributesW("…\Plugins\Fruity\Effects\<名>\<名>_x64.dll")` で存在を見る(Chrome の trace で、単一インスタンスの plugin は 2 回、Soundgoodizer は 24 回、PEQ2 は 16 回。いずれも結果 3 = path not found。`docs/investigations/ref3/tables/plugins-not-opened.tsv`)。**存在しなければ Problems 窓の行になる**(8 plugin(s))。
3. ホスト側の API は **`TFruityPlugHost`(RTTI 0x7b6947、インスタンス 0x108 B)で、FLEngine のゲストコード**。plugin は `CreatePlugInstance(Host, Tag)` で受け取ったホストの vtable を**ゲストコードどうしで**呼ぶ。= runtime の実装は要らない(読み)。plugin 側が FLEngine を呼ぶ道は host-engine-boundary.md §4 でも未調査と書いてあり、私も読んでいない。
4. 音声: ミキサー thread(`f_12aeec0` TMixerThread_DS.Execute → ミキサー本体 `f_d0ece0`)が plugin の処理を vtable 越しに呼ぶ = ゲストどうしの呼び出し。
5. **先例(決定 0183、wrp1)**: Fruity Wrapper(Generators)を実エンジンの**実モジュール**にして、参照プロジェクトを開く操作から `LoadLibraryW(full path)` → DllMain(完走)→ `CreatePlugInstance`(戻る)まで進めた(native -O0 + fork プローブ)。壁は W0〜W3 の 4 つで `CreatePlugInstance` が戻った: **W0** `LoadLibraryW` のフルパス(→ パス表 `fle_realmod_find_path` とボリューム上のファイルの確認 `volume_file`、決定済み・実装済み)、**W1** Delphi の hook の `VirtualProtect`(→ `--hook-points`)、**W2** `CreateJobObjectW`・`SetInformationJobObject`(**Wrapper 固有**)と `formats.txt`・`GM CCs.txt`・フォントの存在確認、**W3** `SHGetKnownFolderPath(FOLDERID_ProgramFilesX64)`(`FL Cloud Plugins\LicenceEngine.dll` を探す。Wrapper の物と読む)。W4 以降は VST3 の側(Serum 2)。
6. wasm での先例は無い(plugin DLL の lift を CI に入れた物は 0 件。goal-ladder.md の G9 は『未測定』、『native で Fruity Wrapper の load のみ』)。

### 2.2 『最小の 1 つ(Fruity Balance)を LoadLibraryW で開ける』ために足す物(by-kind)

| 種類 | 足す物 | 既にある物 | 備考(読み) |
|---|---|---|---|
| **module slot** | `RM_MAX` 8 → 16 以上(`fle_realmod.c`、`fle_loadimage.h` の cursor handle `base + 0x1000 × (index+1) + …` は 1 MiB の領域に収まる。32 個でも 0x20000 台) | 現在の実モジュールは **7 つ**(dsp_ippv2 0x2000000、blend2d 0x4700000、QFC 0x1d00000、freetype 0x4b00000、ILTools 0x6000000、oggio 0xc000000、elastique 0xb000000)で、空きは 1 つ | 基底: 0x400000 の Delphi は別の基底へ再配置が要る。空き 0x6800000〜0xafffff の 64 MB に 8 MB 刻みで 8 個(plugin の image は 3.8〜6.5 MB)。**PM の『実モジュール 5』は私の読みの 7 と食い違う**(`realmod_table_gen.c` の出力の行数を数えた。FLEngine を除く) |
| **path table** | 行 `{"plugins\\fruity\\effects\\fruity balance\\fruity balance_x64.dll", "fruity balance_x64.dll"}`(`fle_realmod_path_table`)を生成物に足す。**出荷の生成器 `lift/harness/b2d2/gen-direct.sh` は名前→基底の表(`realmod_table_gen.c`)しか出さず、パス表を出していない**(awk に数行足す) | `fle_realmod_find_path`(suffix 一致、決定 0183)。出荷の既存 7 モジュールは**素の名前**(例: `oggio`)で `LoadLibraryW` されるのでパス表が要らなかった。plugin は**フルパス**で呼ばれる(0183 W0) | wrp1 の W0 の機構。`gen-direct.sh` の `specs` に 9 本目の行 `"fruity balance_x64.dll|<PE>|<基底>|--pdata --hook-points <表>"` を足す(既存 7 本と同じ形。`--seeds` は不要) |
| **volume のファイル** | `C:\Program Files\Image-Line\FL Studio 21\Plugins\Fruity\Effects\Fruity Balance\Fruity Balance_x64.dll` を volume(seed pack の `vfs-seed-idle1.tsv` の行)に置く。`fle_load.c` の `volume_file` は `fle_vfs_peek_file` で**ファイルの存在だけ**を見る(無ければ NULL + 126)。**DLL の image そのものは別に、モジュールとして bundle の `--dll-dir` に入り、基底に載る**(`build-engine-bundle.sh` の `mods="/data/<dll>:…"`)ので、volume のファイルは存在確認用の小さな stub(数 KB)でよい可能性が高い | 出荷の seed pack は 221 ファイル 7.2 MB(DLL は含まず) | **FL が volume のファイルの中身を読むか(署名検査、PE ヘッダー)は未確認** — 読むなら stub でなく実体(+3.7 MB)か、モジュールの image へのリンクが要る。`Plugin.nfo`(24 B)も置く可能性(未確認) |
| **import の行** | 74 の行なしのうち、plugin を開く経路で実行される物(未確認)。Wrapper の `CreateJobObjectW` 等は Balance に無い | 469 の行あり。74 は FLEngine の import の部分集合 | 最悪 74 行(user32 24・comctl32 22 …)で、既存の 0210 の見積り(12,000〜30,000 行の全体の一部)。実際に踏む数は G9-a の試験で分かる |
| **host vtable** | なし(FLEngine のゲストコード) | `TFruityPlugHost`、インスタンス 0x108 B | 読み。plugin → ホストの呼びは vtable 越しのゲスト間 |
| **フック点** | Balance 用の 3 つの RVA(`--hook-points`)。バイト走査で自動化できる(0183 W1 の手順) | wrapper-hook-points.txt(Wrapper 用) | `VirtualProtect` を実モジュールの image に対して答える機構 `fle_vmap.c` は既に有る(W1 の修正) |
| **CI の抽出と lift** | `ci/extract-dlls.tsv` に 1 行(インストーラの NSIS の名前、出力先、SHA-256)、`build-engine-bundle.sh` の `EXTRA_MODULES`(NEXTRA)と `hyb_modargs` の case(PE、`--pdata`、seeds なし)、`HYB_MODS`(混成の対象にするなら `hyb-U-<m>.txt`)、モジュール表(基底) | 8 モジュール構成(dsp・b2d・qfc・ft・ilt・oggio・elastique + FLEngine) | seeds: `.pdata` の 11,736 関数を全部根にする(FLEngine・IPP と同じ `--pdata`)。仮想関数(RTTI の vtable)は `.pdata` に入っている |
| **UI の描画経路** | plugin の編集窓(Delphi の VCL)の作成・描画・メッセージ | FL 自身の窓(G4〜G6 の経路)。plugin の DLL も同じ VCL を持つ(コードの 70% が FLEngine と同一、plug1) | plugin 固有の描画(つまみの画像、Blend2D・ILSvg の呼び)は初めて走る。壁は G9-c で |
| **音声のブロック処理の呼び出し** | なし(ゲスト間) | ミキサー thread が plugin を呼ぶ | CPU 費用が増える(§0 の 0308 §10 の枠) |

## 3. (3) 起動時間(P1)・転送量・S1 への影響(読み、`tables/0309-s1-sets.tsv`)

基準(E12): wasm **143,085,877 B**、ネイティブ 8 モジュール合計 **77,442,944 B**(S1 の全体比 **1.848 倍**)、brotli 約 15.5 MB(623aba04 の値)、first idle **5.1〜6.1 s**(私の 0304 の Chrome 8 本、この機械)、コンパイル **9.4 ms/MB**(performance-targets.md §4。320 MB 構成の旧実測 3.0 s を比例換算 — **現行の混成・直 wasm の構成では未測定**。node は 0.9 ms/MB)。

| 追加する plugin | ネイティブ(追加) | 共有なしの追加(wasm + データ) | S1 全体比(共有なし) | brotli の追加 | コンパイルの追加(9.4 ms/MB) | B-FLE の追加 | S1 全体比(B-FLE) |
|---|---:|---:|---:|---:|---:|---:|---:|
| Balance のみ | 3.7 MB | 14.6 MB | **1.94 倍** | +1.4 MB(+9%) | +0.14 s | 3.3 MB | 1.80 倍 |
| Balance + Filter | 7.5 MB | 29.5 MB | 2.03 倍 | +2.9 MB | +0.28 s | 6.9 MB | 1.77 倍 |
| + Delay 2 | 11.2 MB | 44.2 MB | 2.11 倍 | +4.3 MB | +0.42 s | 10.2 MB | 1.73 倍 |
| 6 種(Balance・Filter・Delay 2・PEQ2・Frequency Shifter+Engine・Soundgoodizer) | 25.8 MB | 101.3 MB | **2.37 倍** | +10.0 MB | +0.95 s | 28.9 MB | 1.67 倍 |
| 7 種(+ FLEX と engineV2) | 39.1 MB | 167.2 MB | **2.66 倍** | +15.7 MB(2 倍) | +1.57 s | 80.1 MB | 1.91 倍 |

- **P1**: 今の方式(全部を 1 つの wasm に静的に結合)では、plugin を使わないプロジェクトでも**起動のたびに全 plugin 分を取得・コンパイルする**。first idle への加算の上限は『コンパイル時間がすべて first idle に載る』と置いた値(上表)。ダウンロードは同じ回線の速さで +brotli 分。**動的にモジュールを足す方式(LoadLibraryW の中で fetch → instantiate、JSPI で待てる〔jspi-results〕)にすれば P1 は変わらず、プロジェクトを開くとき(使う plugin だけ)に払う**が、wasm の動的リンク(共有 import・複数モジュール)の設計が要り**未検証**(plugins-distribution.md §5.2 A の (4))。
- **S1**(wasm ≤ ネイティブ × 2): **共有なしで入れてよいのは Balance 1 つだけ**(全体 1.94 倍。2 つ目で 2.03 倍)。モジュール単位は共有なしだと 3.8〜4.1 倍で満たさない(plugins-distribution.md §5.2: Delphi 3.3〜3.7 倍、MSVC 2.4〜5.9 倍)。**B-FLE が入れば全体 1.67〜1.91 倍**で収まる(FLEX の engine〔MSVC、共有できない〕が最大の項)。= **S1 を守るなら、2 つ目の plugin の前に B-FLE(生成器の共有化)が要る**。
- **転送量**: brotli の追加は共有なしで Balance +1.4 MB、6 種 +10.0 MB、7 種 +15.7 MB(今の約 15.5 MB の約 2 倍)。B-FLE なら 7 種で +約 7 MB の見込み(plugins-distribution.md の参照プロジェクト 6.1 MB に整合)。
- **メモリ**: PE の image(Balance 3.8 MB。8 種で 3.8〜6.5 MB)が guest の線形メモリに載る(再配置 reloc 147 KB)。wasm32 の 4 GiB 内。コードのメモリ(Liftoff/TurboFan の機械語)は wasm の大きさに比例(未測定)。
- **CI のビルド時間(読み)**: 生成 C はコード 1 B あたり 21 B(Balance 35 MB)。FLEngine の C は 約 215 MB なので、Balance 1 つで +16%、6 種で +約 100%(冷間 18 分 → 約 35 分、`objects` 323 s と `link` 702 s がほぼ比例)。

## 4. (4) 『8 種のうち 1 つを鳴らす』検証の段階表と人日(読み)

**見積りの根拠**: (a) wrp1(0183)は Fruity Wrapper を実モジュールにして、W0〜W3 の 4 つの壁で `CreatePlugInstance` が戻るところまで、4 時間の予算の 6 サイクルで進めた(native -O0、plugin は Wrapper で、Balance より複雑)。(b) wasm の経路は CI の冷間 18 分、node の試験ビルドの増分 約 10 分で、1 サイクルが native より重い。(c) 壁ごとの修正は行 1 本〜数十行(0210: 1 行あたり 12,000〜30,000 行の全体の一部)。**確度は低い**(実行していない)。

| 段階 | 合格の判定(測れる形) | 内容 | 人日 |
|---|---|---|---|
| **G9-0 準備** | ビルドが通る(Balance を 9 つ目の実モジュールとして lift・link・bundle) | extract 行(SHA-256)、モジュール表・基底・パス表、`RM_MAX` の拡張、`--hook-points` の走査(3 RVA)、volume のファイル、`build-engine-bundle.sh` の対応、node の試験ビルドの手順 | 1 |
| **G9-a** | Problems 窓の行が 1 つ減る(『7 plugin(s) could not be opened』、Balance の行が消える)。node の G3 の窓表で確認。`CreatePlugInstance` が戻る | W0 の相当(パス表・volume)、W1(フック点)、行なし 74 のうち踏む物の実装(コメント: 壁が 3〜10 本と置く)、DllMain と `CreatePlugInstance` の完走 | **2〜4** |
| **G9-b** | 音が出る: Balance 1 つだけの構成で Export した WAV が、Wine で Balance だけを有効にした構成(opt1 の退避フレームワークで再有効化)の WAV と**ほぼ一致**(規格化相関 ≈1、Elastique の残差を除く) | ミキサー thread が plugin の process を呼ぶ経路で踏む壁、パン/ボリュームの結果の検証 | +1〜3 |
| **G9-c** | GUI が出る: エフェクトスロットをダブルクリックして plugin の窓が出る(画素が native と一致、または Chrome で目視) | VCL の窓の作成・描画・入力。comctl32 の 22 行(ImageList 等)と user32/gdi32/uxtheme の行。plugin 固有の画像(Blend2D・ILSvg) | +2〜5 |
| **G9-d1** | Filter、Delay 2 が同じ手順で鳴る(行なし 74 は同一集合なので新しい壁は少ない見込み) | 各 plugin ごとにモジュール表・フック点・検証。**S1 のため B-FLE が先に要る(§3)** | 各 +0.5〜1.5 |
| **G9-d2** | PEQ2、Soundgoodizer(Frequency Shifter は下の G9-d3) | `ildsp` の扱い(NULL で通るか、lift するか。lift なら x87 944・AVX-512 23,929 命令の対応が要り、AVX-512 は fe-lift が未対応 = 0308 §9)、Soundgoodizer は行なし +33 | +3〜8(ildsp の可否で幅) |
| **G9-d3 Frequency Shifter** | Frequency Shifter が鳴る | `Engine_x64.dll` の VEX スカラー倍精度・FMA(692 箇所、約 14 種類)の lift(fe-lift の VEX 対応は移動 4 命令のみ、0032)+ `fe-wasmgen` の対応 + ildsp の扱い。FMA は融合の丸めの扱いを決める必要(0308 §9) | +4〜8 |
| **G9-e FLEX** | FLEX が鳴る | **前提: VEX/EVEX の lift**(engine_x64: VEX 256 ビット float 18,772 箇所、engineV2: EVEX の AVX-512 190,302 箇所)+ MSVC の engine(8.1 MB)+ OpenGL 36 行(WebGL への橋)+ dbghelp 6 行 + d3d9 | **見積もれない**(VEX/EVEX の lift の事業 + 橋。+30 d 以上の読み) |
| **B-FLE(共有化)** | 2 つ目以降の plugin で S1 の全体比が 2 倍以内 | 生成器の変更(データ参照を表経由に。plugins-distribution.md §5.2) | 見積もっていない(別の調査が要る。大) |

- **最小の道**: G9-0 → G9-a → G9-b で **約 1 週間(4〜8 d)**、Balance の音まで。GUI は +2〜5 d。Delphi 6 種を鳴らすまで 約 3〜5 週間(B-FLE を除く)。FLEX と Wrapper(第三者 VST)を除く。
- **最初の 1 つに Balance ではなく Filter や Delay 2 を選ぶ理由は無い**(3 つは行なしの集合が同一)。Balance が最も単純(パンとボリューム、DLL が最小)で、**Wine の再有効化の比較も 1 つだけ入れる形で素直**。
- **リスク(確度の低い順)**: (1) volume のファイルの中身を FL が読む(署名検査など)と、+3.7 MB と claims 表の行が要る。(2) 行なし 74 のうち plugin の編集窓が踏む物(comctl32 の ImageList 22 行)。(3) plugin が FLEngine を呼ぶ道(`TFruityPlugHost` の逆方向)で、wasm の hybrid(直 wasm)の関数表に影響が出る(混成の対象にしなければ無関係)。(4) 全 plugin を 1 つの wasm に静的に結合する方式の V8 の 1 GiB 上限(7 種共有なしで 309 MB なので当面は問題ない)。

## 4b. volume・pack の変更点の一覧(PM 19:10 の依頼。mt3 の bundle の仕組みと合わせるため。Balance 1 つの場合)

| # | 場所 | 変更 | 備考 |
|---|---|---|---|
| 1 | `lift/harness/b2d2/gen-direct.sh` | `specs` に 1 行 `"fruity balance_x64.dll|<PE>|<基底>|--pdata --hook-points <表>"`(既存 7 本と同じ形、`--seeds` なし)。末尾の awk に**パス表 `fle_realmod_path_table`** の出力(suffix → 名前)を足す | 現状は名前→基底の表だけ。出荷の 7 モジュールは素の名前で呼ばれる |
| 2 | `lift/harness/hooks/` | `fruity-balance-hook-points.txt` 新規(Delphi の inline hook の 3 つの RVA。Balance の静的初期化から、`f_5e91f0` 相当の唯一の呼び元をバイト走査して求める) | Wrapper は 0x1b700・0x39630・0x39650 で、Balance の同じ RVA は別の関数(実測) |
| 3 | `app/scripts/build-engine-bundle.sh` | `EXTRA_MODULES`・`NEXTRA`(今 2)を 3 に。名前の照合 `${x}_x64.*.dll` は『Fruity Balance』に空白があるので case を足す。`seq 1 $((5 + NEXTRA))`(m1〜m8 の分割)、`$((6 + NEXTRA))`(in/dll の個数の検査)、`mods=` の並び(modules.txt の順)、description の `modules=8`(表記) | `HYB_MODS` は変えない(混成の対象にしない = Balance は全部 C 部) |
| 4 | `ci/extract-dlls.tsv` | 1 行(インストーラの NSIS の名前の suffix = `Plugins\Fruity\Effects\Fruity Balance\Fruity Balance_x64.dll`、出力先、SHA-256 = Wine のインストール先の物 `a399cbe0…8064`、インストーラの物と一致するかは未確認) | extract 段のキーが変わり、以降が再実行される |
| 5 | `lift/harness/seeds/vfs-seed-idle1.tsv` | volume のファイルの行 1 つ(guest path → host path → SHA-256)。stub でよいか、実体かは §2.2 | seed pack の key が変わる(`ci/plan.sh` は `lift/harness/seeds` 全体のハッシュを含む) |
| 6 | `ci/objkeys.sh` と `ci/stage-lift.sh` | m8 の object のグループ(コメントと key の組み立て)。`ci/test-*.sh` のうち『8 モジュール』を数える物があれば更新 | `test-hybrid.sh` は混成の 8 モジュールのリストを数える(Balance を混成に入れなければ変更なし) |
| 7 | `lift/runtime/fle_realmod.c` | `RM_MAX` は 8 のままで収まる(表は 7 + 1 = 8)。2 つ目の plugin から 16 以上 | 実モジュール 7 つの空きは 1(私の読み) |
| 8 | `app/scripts/pack-assets.mjs`・`app/src/engine` | **変更なし**(`--dll-dir` を `walk` するだけ、モジュール名の固定は app/src に 0 件) | bundle の manifest の `args` が増える |
| 9 | bundle の大きさ | wasm +約 13 MB(共有なし、読み)、DLL の image(`--dll-dir`)+3.7 MB(brotli 約 1.1 MB)、volume の stub(数 KB) | §3 の表。S1 は 1.94 倍 |

## 5. 未確認と、確かめる最も安い測定

1. Balance を開く経路で実行される行なしの import の数(**node で FLEngine に Balance を実モジュールとして足した試験ビルドを 1 本走らせれば分かる**。G9-0 の一部)。
2. FL が plugin の DLL ファイルの中身を読むか(署名検査・PE の読み): FLEngine の `WinVerifyTrust` は **1 関数 `f_1375990`(4 箇所、`CryptQueryObject`・`CertFindCertificateInStore` も同じ関数)だけで、直接の呼び元は 0**(仮想メソッド/関数ポインタ経由。バイト走査の実測)。0118 の gdb では FL64.exe と FLEngine の path で呼ばれた。plugin の path で呼ばれるかは未確認。Wine の trace(opt1)か、wasm の trace で `GetFileAttributesW` の後に `CreateFileW` が同じ path に来るかで分かる。
3. `ildsp` が PEQ2 などで実行されるか(NULL 許容の分岐)。
4. 実 wasm の大きさ(Balance を直 wasm 生成・hybrid ありで作った実測)。上の表の『共有なし』は plug1 の倍率(Delphi で DLL の約 3.9 倍)の読みで、E12 の全体(8 モジュール)が実測で 1.85 倍に収まっている(直 wasm 生成・混成の圧縮)ことから、plugin でも倍率が下がる余地がある(下げ幅は未測定。見積りの幅は 1.85〜3.9 倍)。

## 6. 逸脱・自己申告

- 18:2x の `bash` ツール名の呼び出し(0308 §11 に記載済み)。
- 0304 の後片付けは PM の了解(18:31)の後。0308 の `/tmp/opencode/a21` は PM の了解待ち(最終の取り込み後に消す)。


## PM の判断と取り込み(2026-11-06 19:50、docs のみ。製品の差分なし)

取り込み: alpha1 の最終(19:47)の文書・表・道具を `docs/investigations/alpha21/` に置いた(生ログは入れない)。実行は fe-lift による census(11 モジュール、実エンジン・実ブラウザは動かしていない)と静的な読み。**wasm のサイズ・起動時間・人日はすべて読み(外挿)**。依頼者の機械は 0 件。

事実として受け取る:
1. 現行の fe-lift は Fruity Balance / Filter / Delay 2 / Parametric EQ 2 / Frequency Shifter を各 1.1〜1.2 s で lift でき、bad_nodes 0、命令レベルの未対応 0、行の無い import の thunk は同一の 44(comctl32 の ImageList 22、user32 9、kernel32 5、ole32 4、shell32 3、memset)。Soundgoodizer・FLEX・Wrapper も未対応命令 0 の Delphi 本体を持つ。**例外: Frequency Shifter の Engine_x64.dll(MSVC、VEX スカラー double + FMA 692 箇所)、FLEX の engine_x64.dll(VEX 256 bit 18,772)と engineV2_x64.dll(EVEX を含む 190,302)**は、/arch:AVX 以上で常に VEX を実行する(SSE2 の代替が無い)ため、VEX/FMA の lift が要る(Frequency Shifter +4〜8 人日、FLEX は 30 人日以上で今は見積もれない)。
2. 開く経路の機構は既にある(real module + path table、決定 0183 の wrp1)。足りないのは**モジュール追加の手順**(CI の抽出行、module table、base、path table、フック 3 RVA〔Delphi 向けは `find-hook-points.py` で自動化〕、volume のファイル、bundle の追加)で、1 つ目は 9 行の変更(§4b)。`RM_MAX 8` は実モジュール 7 + Balance で足りる。TFruityPlugHost の vtable と音声ブロックの呼びは FLEngine の guest コードで、runtime の実装は不要(読み)。
3. wasm の増分(読み): 共有なし Balance 14.6 MB(DLL の 3.9 倍)・6 種で +101 MB、FLEngine との共有(B-FLE、生成器の変更は未着手)あり Balance 3.3 MB・6 種 +29 MB。**S1(wasm ≤ native×2)の全体比は現在 1.848 倍で、Balance 単独 1.94 倍、2 つ目で 2.03 倍(超過)、6 種で 2.37 倍(B-FLE ありなら 1.67 倍)**。= **B-FLE が 2 つ目のプラグインの前提**。P1: Balance +0.14 s、6 種 +0.95 s(静的リンクなので毎回の起動で全部取得・コンパイル)。転送(brotli): Balance +1.4 MB、6 種 +10 MB。
4. 段階(読み、低い確度): G9-0 準備 1 日 → G9-a(Problems 窓が『7 plugin(s)』)2〜4 日 → G9-b(音が出る。Wine の Balance のみの Export と比べる)+1〜3 日 → G9-c(GUI)+2〜5 日。1 つが鳴るまで約 1 週間(4〜8 人日)。Filter / Delay 2 は各 +0.5〜1.5 日、PEQ2 / Soundgoodizer は ildsp で +3〜8 日。

判断:
1. **G9 の入口は開く。G9-a(Fruity Balance を 1 つ開く)は alpha1 の次の担当(決定 0310 仮)**。ただし変種 FLP の Export(両側)と Split mixer tracks で残差の原因を切り分ける(決定 0308)のを先にする(約 1 時間)。G9 は残差の原因に依らず必要だが、G9-b の音の比較(Balance のみの Export)に残差の情報が使えるため。
2. **2 つ目のプラグインの前に B-FLE(FLEngine との関数共有の生成器)を入れる**(S1 が 2.03 倍で超過するため)。G9-a / G9-b は 1 つだけなので B-FLE なしで進める(wasm +13 MB、DLL +3.7 MB、brotli 1.1 MB)。
3. Frequency Shifter / FLEX は VEX/EVEX の lift が前提なので G9 の最初の範囲から外す(Delphi の Balance → Filter → Delay 2 → PEQ2 → Soundgoodizer の順)。
4. 未確認: Balance の開く経路で実際に実行される行の無い import の数、FL がプラグイン DLL の中身を読むか(WinVerifyTrust)、実 wasm サイズ(1.85〜3.9 倍の幅)、ildsp が実行されるか。人日は外挿で低い確度。
