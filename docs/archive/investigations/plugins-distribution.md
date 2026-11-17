# FL Studio 同梱プラグイン全体の配布設計: 静的調査(重複の共有・使う分だけ取得・実行時変換)

作成: 2026-11-03、基準 HEAD **eca8c0b**(ブランチ main)。**静的解析で、プラグインも FL Studio も実行していない。進捗ではない**(`docs/policies/progress-integrity.md`)。**何も決めない**(推奨は §9 に書く。決めるのは PM)。インストーラは実行していない(NSIS のデータを `tools/extract_nsis2.py` の流儀で読んだだけ)。wasm は**ビルドしていない**: wasm の大きさはすべて「fe-lift が出した生成 C の実測 × プロジェクトの既存の倍率」の**見積り**(§4.3)。リポジトリの tracked ファイルは編集していない。

## 凡例

- **実測** = 今回のコマンドの出力(表・ツールは `tables/`、`tools/`)。**読み** = 実測や公開資料からの解釈(確かめていない)。**未確認** = 調べていない。**推測** = 根拠が弱い見込み。
- 「コード」= 関数単位に切った機械語の合計(`.pdata` の関数+フロー追跡で見つけた `.pdata` 外の葉関数。§3.1)。「ネイティブ」= 元の DLL(バイト)。「Oz」= `-Oz` の wasm。「A〜E」= §5 の配布案。

## 0. 要約表(先頭 6 項目)

| # | 問い | 答え |
|---|---|---|
| 1 | **同梱プラグインの DLL は何個・何バイトか。依頼者の例と合うか** | **182 個、709,770,088 B(676.9 MiB)**(`Plugins\` 配下の `.dll` 全部。実測)。x64 が 181 個、x86 が 1 個(`FL Slayer\Slayer_Engine.dll` 179,512 B)。プラグインのディレクトリは 141(`ls.out` の 144 から `Fruity`・`Effects`・`Generators` の 3 を引いた数。名前は全件一致)。`CreatePlugInstance` を輸出する DLL が 160 個(Delphi 133、MSVC 27)。言語別: **Delphi 135 個・598.4 MB(84%)、MSVC 47 個・111.4 MB**。Delphi の前面 DLL(133)は最小 3.59 MB・**中央値 4.05 MB**・平均 4.45 MB・p90 6.05 MB・最大 7.60 MB。**Soundgoodizer 6,407,680 B(6.11 MiB)、Fruity Flanger 3,716,608 B(3.54 MiB)で依頼者の例と一致**。g8-project-load の「111 種・139 個・約 524 MB」はデモ曲が使う部分集合(全体は 182 個・710 MB) |
| 2 | **重複の共有で何割減るか(最重要)** | **Delphi 135 個のコード 274.8 MB のうち、ユニークは 17.4 MB(6.4%)=93.6% が他の DLL と同一**(関数単位、アドレス差を除いて照合、呼び先の同定を 1 段含む。3 段でも 7.0%)。1 つの DLL のコードの 98.7% は別の DLL にもある(Flanger↔Balance)。Delphi の `.text` の中にある RTTI/VMT/定数(`.rdata` が 69 B しか無いので全部 `.text` にある)も、内容定義チャンクで 194.6 MB → 14.9 MB(7.7%)、`.rsrc` は 36.8 → 6.0 MB(16%)、`.data` は 37.5 → 25.6 MB(68%。共有しにくい)。MSVC 46 個(x64)はコード 59.0 MB → 27.5 MB(47%)。**全 181 個の「ユニークな内容」の下限 = コード 44.9 MB + データ 70.1 MB**。独立の確認: 710 MB を連結して `zstd -10 --long=30` で 109.1 MB(15.4%)。**さらに FLEngine は Delphi 製プラグインのコードの 70% と同一の関数を既に持つ**(DLL ごとの中央値 75%、最大 84%)。ただし共有関数の 68.9% はモジュール固有のデータを RIP 相対で参照するので、**アドレスを変えずにそのまま共有はできない**(同じ RVA での一致は最良の組で 26%)。生成器の変更(モジュールごとの参照表経由)が要る |
| 3 | **サンプルの lift: 生成 C → 推定 wasm → ネイティブ比** | `fe-lift lift --pdata`(census)を 20 個に実施。**生成 C はネイティブのコード 1 B あたり Delphi 21.0 B(20.6〜21.3、11 個)、MSVC 21.8 B(19.5〜24.0、9 個)。Delphi と MSVC に実質的な差は無い**。Flanger: 3.72 MB の DLL → C 35.07 MB → Oz 推定 12.9 MB(+非ゼロのデータ 1.71 MB)= DLL の **3.9 倍**。Soundgoodizer: 6.41 MB → C 57.1 MB → Oz 21.0 MB(+3.18 MB)= **3.8 倍**。Delphi は 3.3〜3.7 倍(wasm のみ)、MSVC はコードが DLL の 29〜78% を占める(engine は 56〜78%)ので 2.4〜5.9 倍(FL Studio Mobile 5.4、FLEX engine 5.8)。**MSVC の FLEX engine・FL Studio Mobile・Morphine・Slicer 2 は AVX/AVX2/FMA/EVEX の未対応命令が 1.2 万〜19 万個あり、その分の C は入っていない(wasm の見積りは下限)** |
| 4 | **配布案 A〜E の比較(要約。表は §5)** | **全 181 個**(Oz): A(共有なし)= wasm 2.60 GB+データ 0.32 GB = ネイティブの **4.1 倍**(brotli で約 281 MB)。-O2 なら 8.1 倍。B(重複を共有する下限)= 425 MB(**0.60 倍**、brotli で約 46 MB)。**参照プロジェクト**(7 種+Engine 2 個、ネイティブ 39.1 MB):A/C = 171.9 MB(4.39 倍、brotli 16.0 MB)、B(Delphi をプラグイン間で共有)= 81.8 MB(2.09 倍)、**B-FLE(FLEngine を共有ライブラリとして使う)= 66.9 MB(1.71 倍、brotli 6.1 MB)**。Delphi 7 個だけなら B-FLE は **16.7 MB = ネイティブ 30.7 MB の 0.54 倍**。D(関数単位の遅延取得)は転送をさらに 1/4〜1/11 に。E(ネイティブをそのまま配って実行時に変換)は大きさは 1.0 倍だが、**CheerpX は 32 ビット x86 のみ、v86 は 32 ビットの PC エミュレータで、x86-64 の Windows DLL を扱う既製品が無い**。 |
| 5 | **依頼者の基準(ネイティブの 2 倍、目標 1.2 倍)を満たす案はあるか** | **基準の取り方で答えが変わる**(§6)。(a) 配布物の総量(生): A は 4.1 倍で不可、**B は 0.60 倍で可(1.2 倍も可)**。(b) brotli 後 / ネイティブの brotli: **A でも 1.33 倍(2 倍以内、1.2 倍は不可)**。(c) 参照プロジェクト 1 回の転送(生): A/C 4.39 倍 不可、B 2.09 倍(僅差で不可)、**B-FLE 1.71 倍 可(2 倍)**、D は 0.4〜0.7 倍。(d) **プラグイン 1 個を初めて取得する量(生)**: A・B は 3.8〜4.1 倍で不可(1 個目は自分のコードを全部取るしかない)、**B-FLE は Delphi 133 個の中央値 1.03 倍(p90 1.54 倍)で可**。**MSVC の engine 群は重複がほとんど無い(ユニーク 47%)ため、wasm の膨張率(コード 1 B あたり約 8 B)そのものを下げない限り 2 倍に届かない**(別の担当の領域) |
| 6 | **推奨(意見。§9)** | (1) 配布の単位を「プラグイン 1 個 = 1 モジュール(LoadLibraryW で JSPI を使って取得)」にして使う分だけ取る(C)。(2) 並行して、**Delphi 製プラグインは FLEngine を共有ライブラリとして使い、残りだけを配る(B-FLE)ための生成器の変更の可否を、Flanger と Balance の 2 個で先に試す**(関数をモジュール固有の参照表経由にして、同一の C が出るか)。(3) 起動時に走る関数の割合(D)をプラグインで測る(音声スレッドは待てないので事前取得の集合が要る)。(4) E は研究枠に留める |

## 1. 手順・出典・実行環境

- 作業の入口: `lift/out-git/plug1-results/`(本文・`tools/`・`tables/`、合計 約 0.4 MB)。作業ディレクトリ `lift/out-plug1/`(DLL 約 678 MB、重複の中間物 約 160 MB、cargo target は終了時に削除)。DLL・インストーラの中身は成果物に入れていない。
- インストーラ `~/ダウンロード/flstudio_win64_26.1.7.5653.exe`(NSIS。SHA-256 `04393e79…5fc02`)は**読み取りのみ**。`tools/nsis_all_sections.py`(`gen_install_tree.py` の走査を**全セクション**に広げたもの)で EXTRACTFILE を列挙: ファイルを出すのは「FL Studio (required)」(27,402 件)と「VSTi」(2 件)だけ。`Plugins\` 配下は 6,414 ファイル(LZMA 後 284,094,765 B。うち `.dll` 182 個が 179,783,827 B、残りは wav・jpg・fst・zgeproj 等)。`.dll` だけを `lift/out-plug1/dll/` に書き出した(SHA-256 と大きさ: `tables/dll-sha256-size.tsv`)。
- 道具(標準ライブラリ以外): pefile 2024.8.26、capstone 5.0.9(検算のみ)、numpy 2.5.3、iced-x86 1.21.0(Rust、`lift/Cargo.lock` の版。`tools/dupscan/`)、`fe-lift`(HEAD のもの。`CARGO_TARGET_DIR=lift/out-plug1/target`、`cargo build --offline --release`)、node 24.19.0 の zlib(brotli)、zstd 1.5.x(連結の確認)。
- 他の調査から取った数値(再測定していない): wasm 倍率(-Oz = C の 0.368 倍)と brotli(wasm の 7.17%)は `docs/milestones/performance-targets.md` §4b の 6 モジュール構成の実測(C 872.2 MB → -Oz 320,731,834 B → brotli 22,994,669 B)。起動で走る関数は FLEngine で約 9%(依頼文の記述)。Cache API・HTTP キャッシュ・IndexedDB の限界は決定 0130 の実測(§7)。
- ネットワーク(公開資料のみ、取得日 2026-11-03): §7 と §5.2 の E に URL と文言。ページの末尾に依頼者以外の指示は無かった。

## 2. 同梱プラグインの棚卸し(やること 1)

**実測**。全表は `tables/dll-inventory.tsv`(182 行。パス、大きさ、SHA-256、Machine、ImageBase、リンカ版、Rich ヘッダ、節の大きさ、.text/.rsrc/.pdata/.reloc/.idata/.data の大きさ、import の DLL 名、export 名、Delphi の目印、言語の推定)、`tables/size-distribution.txt`。

| 区分 | 個数 | 合計 B | 最小 | 中央値 | 平均 | p90 | 最大 |
|---|---:|---:|---:|---:|---:|---:|---:|
| `Plugins\` の全 DLL | 182 | 709,770,088 | 45,056 | 3,894,784 | 3,899,835 | 6,147,379 | 27,128,048(NDI) |
| x64 | 181 | 709,590,576 | 45,056 | 3,912,704 | 3,920,389 | 6,156,288 | 27,128,048 |
| Delphi(全) | 135 | 598,405,120 | 45,056 | 4,046,848 | 4,432,630 | 6,120,652 | 7,600,128 |
| Delphi、`CreatePlugInstance` を輸出 | 133 | 592,165,376 | 3,588,608 | 4,046,848 | 4,452,371 | 6,053,376 | 7,600,128 |
| MSVC(全) | 47 | 111,364,968 | 103,936 | 653,824 | 2,369,467 | 6,652,313 | 27,128,048 |
| MSVC、`CreatePlugInstance` を輸出 | 27 | 65,879,552 | 394,752 | 851,968 | 2,439,983 | 8,163,430 | 12,141,568 |
| `CreatePlugInstance` を輸出する DLL(= goal-ladder G9 の 160) | 160 | 658,044,928 | 394,752 | 3,951,104 | 4,112,780 | 6,159,513 | 12,141,568 |

- **言語の推定**(読み): Delphi = 節に `.didata` がある(135 個すべて。Rich ヘッダなし、リンカ 8.0、ImageBase 0x400000、`.rdata` は 69 B)。MSVC = Rich ヘッダがある、または `_RDATA`/`RT_CODE` 節がある(リンカ 14.29 が 42 個、他に 14.13/14.16/14.27/12.0/6.0 が各 1。x64 は ImageBase 0x180000000、x86 の 1 個は 0x10000000)。「その他」は 0 個。Delphi 133 + MSVC 27 = 160 は goal-ladder G9 の「Delphi 132、MSVC 27」とほぼ一致(Delphi が 1 個多い)。
- **x86(32 ビット)** は `Generators\FL Slayer\Slayer_Engine.dll` の **1 個だけ**(179,512 B、MSVC 6.0 リンカ、`main` を輸出)。lift の対象外として別に数えた。他の 181 個は x64、**181 個すべてに `.pdata` がある**(`fe-lift --pdata` が使える)。
- **Delphi 前面 DLL の中身**(135 個の合計に対する割合): `.text` 79.2%、`.rsrc` 6.3%、`.pdata` 3.7%、`.reloc` 3.8%、`.idata` 0.5%、その他のデータ 6.5%。**`.text` の中身が実は半分近くデータ**で、関数として切れるコードは `.text` の約 58%(Flanger: `.text` 2.94 MB のうちコード 1.65 MB)、DLL 全体の約 46%(274.8 / 598.4 MB)。残りは RTTI・VMT・文字列・浮動小数定数・ジャンプ表・ゼロ詰め(Flanger で `.text` の 28.0% が 0 バイト)。
- **名前が付いた Delphi DLL の例との一致**: Soundgoodizer 6,407,680 B(`.rsrc` が 1,088,000 B と大きい)、Fruity Flanger 3,716,608、Fruity Parametric EQ 2 4,036,608、Fruity Delay 2 3,731,968、Fruity Filter 3,780,608、Fruity Balance 3,718,656、Frequency Shifter 3,876,864(+ MSVC の `Engine_x64.dll` 263,168 B)、FLEX_x64 5,194,752(+ MSVC の `engine_x64.dll` 5,673,984 と `engineV2_x64.dll` 8,119,808。FLEX_x64 の中に `FLEX2Dispatcher` と `DispatchToEngine` の両方の文字列があり、どちらの engine を使うかは**未確認**)。参照プロジェクトの 7 DLL の合計は 30,747,136 B(`reference-project.md` の 30.7 MB と一致)。
- 大きさの上位 20(MSVC は NDI の `Processing.NDI.Lib.x64.dll` 27.1 MB、FL Studio Mobile 3 個 12.1/8.2/8.2 MB、FLEX engineV2 8.1 MB。Delphi は ZGameEditor Visualizer 7.6 MB、Patcher 7.06 MB ×2、Harmor 6.48 MB、Soundgoodizer 6.41 MB…)は `tables/size-distribution.txt`。
- 範囲外: `Plugins\` 配下の `.dll` 以外(wav 38.7 MB、jpg 14.7 MB、png 8.9 MB、tga 7.8 MB、zgeproj 4.5 MB など LZMA 後で約 104 MB)と `Data\` 配下(LZMA 後 483.6 MB)。プラグインが実行時に読むデータとして別に配る必要がある(**未測定**)。`System\Plugin\VSTi`(FL Studio VSTi ラッパー 2 個、918 KB)も対象外。

## 3. 重複の定量(やること 2)

### 3.1 方法(`tools/dupscan/`、`tools/analyze_dup*.py`、`tools/verify_pairs.py`)

1. 各 DLL の関数境界: `.pdata` の RUNTIME_FUNCTION(Delphi DLL では `.text` のコードの約 55% しか覆わない=葉関数は `.pdata` に無い)+ `.pdata` 外の領域を**フロー追跡**した葉関数(`.pdata` 関数の中の call/jmp(域外)/RIP 相対 lea の先、`.reloc` が指す 8 バイトの絶対ポインタの先を入口にして、ret/無条件 jmp まで追う)。追えなかった `.pdata` 外の領域は「`.text` 内データ」に回す。
2. 各関数を iced-x86 で復号し、**変位・絶対アドレスを消して**命令列をハッシュ: RIP 相対メモリの変位 4 B、関数の外に出る call/jmp/jcc の rel 変位、`.reloc` が覆うバイトを 0 にし、各命令の後ろに種別タグ(0 なし、1 域外分岐、2 IAT 経由、3/4 コード/データ参照、5 再配置)を付ける。関数内の分岐の変位は残す。ハッシュは 64 ビット(SipHash)×3 段:**h1** = 正規化した命令列だけ、**h2** = h1 + 呼び先の関数の h1(同じモジュール内の直接呼び出し)+ IAT の import 名+データ参照の節名、**h3** = h2 を 1 段深くしたもの。このレポートでは**h2 を標準**(h1 は下限、h3 は上限寄りの保守側)にした。
3. データ: `.text` 内データ、`.data`/`.rdata`/その他、`.rsrc`(リソースのデータ項目ごと)を、再配置バイトを 0 にして**内容定義チャンク**(Gear ハッシュ、64〜1024 B、平均 約 256 B)に切ってハッシュ。全ゼロのチャンクは別に数えて除外。
4. 検算(実測): h2 が等しい関数の組をランダムに 372 組取り、capstone で命令ごとに比べると、バイト列が違う命令が RIP 相対・分岐・メモリ/即値の命令だけだった組が **369/372**(残り 3 組はジャンプ表等に含まれる再配置バイトの差と読む)。独立の確認: 全 182 DLL を連結して `zstd -10 --long=30`(窓 1 GiB)で **109,107,781 B(710 MB の 15.4%)**。私の「ユニークな内容」(コード 44.9 + データ 70.1 = 115 MB)と同じ桁(zstd 側には `.pdata`/`.reloc` 等も含まれる)。
5. 限界(読み): 呼び先の同定は同じモジュール内の 1〜2 段だけ。仮想呼び出し(VMT 経由)の先は見ていない。Delphi の `.text` 内データのうち、フロー追跡で見つからなかった本物のコードはデータ側に数えている。ジャンプ表の中身を関数の命令として復号するので、関数内に不正命令が DLL あたり 1,900〜2,000 個ある(Flanger: 1,943)。いずれも「共有できる量」を**過大にも過小にもし得る**が、上の検算の範囲では 1% 程度。

### 3.2 結果(実測。`tables/dup-summary.txt`、`dup-summary2.txt`、`dup-per-dll.tsv`)

| 集合 | モジュール数 | 関数数 | コード B(重複あり) | ユニーク h1 | ユニーク h2 | ユニーク h3 | h2 のユニーク / 全体 |
|---|---:|---:|---:|---:|---:|---:|---:|
| Delphi プラグイン DLL | 135 | 2,037,110 | 274,816,307 | 14,744,159 | **17,443,385** | 19,241,861 | **6.35%** |
| MSVC(x64) | 46 | 189,421 | 58,966,041 | 23,121,684 | **27,500,468** | 29,713,078 | **46.6%** |
| 全 181 個 | 181 | 2,226,531 | 333,782,348 | 37,865,422 | **44,943,538** | 48,954,624 | **13.5%** |
| 上 + FLEngine | 182 | 2,290,031 | 344,459,587 | 44,473,785 | 51,848,170 | 56,019,692 | 15.1% |

- **Delphi の関数が何個の DLL に在るか**(h2、135 個中。`dup-summary.txt`): 1 個にしか無い 38,374 関数(10.4 MB。コードのユニーク分の 60%)、2 個 6,467(1.6 MB)、3 個 6,333(1.4 MB)、4〜10 個 5,147(1.2 MB)、11〜50 個 4,649(1.1 MB)、51〜100 個 871(0.2 MB)、101〜135 個 8,699(1.4 MB)。**ほとんどの DLL に在るコアは 1.3〜1.6 MB**(121 個以上に在る関数 1.3 MB、67 個以上〔50% 以上〕に在る関数 1.59 MB)。
- **1 つの DLL に固有のコード**(他のどの DLL にも無い): 中央値 28,287 B(その DLL のコードの 1.6%)、平均 78,900 B、p90 174,018 B、最大 1,121,335 B(Soundfont Player、コードの 52%)。
- **FLEngine との重複**(Delphi プラグイン 135 個のコードのうち FLEngine にも同じ関数〔h2〕がある量): **70.4%**(193.5 / 274.8 MB)、DLL ごとの割合は中央値 0.746、最小 0.033、最大 0.842。ただし、プラグイン全体のユニークな和(17.4 MB)のうち FLEngine に無い物は 14.7 MB(84%)=FLEngine が持つのは「ほとんどの DLL に共通の部分」で、長い尾は固有。QFC・ILTools・Blend2D・dsp_ippv2 も含めた既存 4 モジュールとの一致も同程度(Delphi プラグインのコードの 192.8 MB、70%)。`m2-inventory` の「既存 3 モジュールと 89〜99% 共有」(下限の測定)とは指標が違う(私のは関数全体が正規化後に一致する量)。
- **MSVC**: 共有はごく少ないが、**IPP を静的に抱えた engine** がある: Frequency Splitter の engine は 66%(1.72/2.62 MB)、Multiband Delay の engine は 65%、FLEX engineV2 は 34% の関数が **dsp_ippv2_x64.dll と同一**(MSVC プラグインのコード 59.0 MB のうち 9.4 MB=16% が dsp_ippv2 と一致)。FL Studio Mobile 系 3 個は互いに 3.7〜3.8 MB 共有、FLEX の engine/V2 も共有。QFC/ILTools との一致は 1 KB 未満(MSVC はそれらを抱えていない)。
- **データ**(内容定義チャンクの非ゼロ分。実測):

| 種別 | 集合 | 重複あり B | ユニーク B | ユニーク / 全体 |
|---|---|---:|---:|---:|
| `.text` 内データ(RTTI/VMT/定数/表) | Delphi 135 | 194,556,472 | 14,935,989 | **7.7%** |
| 同上 | MSVC 46 | 26,461,613 | 11,788,924 | 44.6% |
| `.data`/`.rdata`/その他 | Delphi | 37,466,395 | 25,588,103 | 68.3% |
| 同上 | MSVC | 20,678,799 | 11,136,443 | 53.9% |
| `.rsrc` | Delphi | 36,822,591 | 6,038,548 | **16.4%** |
| 同上 | MSVC | 619,247 | 604,050 | 97.6% |
| 全データ | 全 181 個 | 316,605,117 | 70,074,478 | 22.1% |

  全ゼロのチャンク(圧縮でほぼ消える)は別に 7.5 MB。`.pdata`(Delphi 22.0 MB)・`.reloc`(22.3 MB)・`.idata` はリフトの入力であって配るデータではない(`.reloc` は共有データの指す先を直す表として一部要る可能性。§5.B)。

### 3.3 共有の理想的な下限と、現実的な共有単位

- **理想的な下限**(全 181 個、1 バイトも重複させない): コード 44.9 MB(ネイティブ)+データ 70.1 MB。これを -Oz の wasm に直すとコードは約 355 MB(§4.3 の倍率)。**これは「同一の関数が 1 個だけある」という最良の場合**で、実現には §3.4 の条件が要る。
- **現実的な単位**(Delphi、コードのみ。`dup-summary.txt` のしきい値表): 「k 個以上の DLL に在る関数を 1 つの共有モジュールにし、各 DLL は残りを持つ」。

| k(共有に入れる条件) | 共有モジュール(ネイティブのコード) | 各 DLL の残り(合計) | 残りの中央値/DLL | 共有+残りの合計 |
|---|---:|---:|---:|---:|
| 2 | 6,993,988 | 10,651,663 | 28,287 | 17,645,651 |
| 5 | 3,803,094 | 18,902,018 | 37,873 | 22,705,112 |
| 10 | 2,889,509 | 24,830,787 | 58,256 | 27,720,296 |
| 34(25%) | 1,856,832 | 43,855,208 | 134,490 | 45,712,040 |
| 67(50%) | 1,586,453 | 56,098,229 | 192,479 | 57,684,682 |
| 120 | 1,300,786 | 80,671,354 | 387,434 | 81,972,140 |

  (比較: 共有なしの合計 274,816,307)。**R2 に置く総量は k によらず理想の下限に近い(k=2 で 17.6 MB)が、1 プロジェクトで取る量が k で変わる**。ランダムな 7 個の Delphi プラグイン(300 回の平均、`tables/tiers-N7.tsv`)で取るコード(ネイティブ換算): 別々のモジュール 13.05 MB、k=2 の共有モジュール 7.52 MB、k=10 で 4.02 MB、**k=34〜67 で 3.67 MB(= 和集合の下限 3.66 MB とほぼ同じ)**。つまり**プロジェクトの規模で 7 個なら「半数以上の DLL に在る関数だけを共有モジュールにする(1.6 MB、wasm Oz で約 12 MB)」で足りる**。1 個だけなら 1.88 MB(別々)と同じ(共有では減らない)、20 個なら 37.4 → 6.0 MB。
- **N 個のプラグインを使うプロジェクトで取るユニークなコードとデータ**(Delphi 前面 133 個からランダムに N 個、60 回の平均、`tables/dup-cumulative.tsv`):

| N | ネイティブのファイル | ネイティブのコード | ユニークなコード | 非ゼロのデータ | ユニークなデータ |
|---:|---:|---:|---:|---:|---:|
| 1 | 4.29 MB | 1.96 MB | 1.96 MB | 1.93 MB | 1.93 MB |
| 2 | 8.60 | 3.95 | 2.33 | 3.86 | 2.50 |
| 3 | 13.24 | 6.10 | 2.77 | 5.93 | 3.13 |
| 5 | 22.14 | 10.18 | 3.23 | 9.94 | 4.10 |
| 7 | 31.01 | 14.25 | 3.75 | 13.93 | 5.04 |
| 10 | 44.24 | 20.32 | 4.37 | 19.88 | 6.36 |
| 20 | 88.94 | 40.86 | 6.25 | 39.95 | 10.46 |
| 50 | 223.5 | 102.7 | 10.02 | 100.4 | 21.4 |
| 133 | 592.2 | 271.9 | 17.74 | 266.0 | 46.3 |

  **2 個目以降は 1 個あたりコード 0.3〜0.5 MB(ネイティブ)しか増えない**。逆に**1 個目は共有では 1 B も減らない**。

### 3.4 共有の前提条件(読み。重要)

- 共有できる関数(2 個以上の DLL に在る 6.99 MB)のうち、**RIP 相対でモジュール固有のデータを参照するのが 68.9%**(4.82 MB)、再配置された絶対アドレスを含むのが 3.7%、**参照も絶対アドレスも持たない純粋な計算だけの物は 28.1%**(1.96 MB)(`dup-summary2.txt`)。ほとんどの DLL に在るコア(67 個以上)も同じ割合(純粋 31.0%、データ参照 68.9%)。
- 同じ RVA での一致は最良の組(Flanger↔Balance)でも `.text` の 26.5%、典型は 13% だった(再配置を消した上でのバイト比較)=**DLL ごとにレイアウトが違う**。したがって共有関数を 1 個の wasm にすると、その中の定数アドレス(`image base + rva`)は DLL ごとに違う値になってしまう。共有するには、データ参照を「呼び出し側のモジュールの参照表(GOT 的)」経由にする、または関数ごとに基底を渡す必要がある(§5.B)。**現行の fe-lift はそうなっていない**(生成 C は絶対アドレスの定数を埋め込む。読み)。
- 可変なグローバル(Delphi RTL のヒープ管理、例外チェーン、`HInstance`、スレッド変数の番号、クラス登録表、unit の初期化表)は DLL ごとに別でなければならない(元は DLL ごとに RTL が静的リンクされている)。参照表経由にすればコードだけ共有され、状態は別のままで済む(読み。未検証)。

## 4. lift の実測(サンプル)(やること 3)

### 4.1 手順

`CARGO_TARGET_DIR=lift/out-plug1/target cargo build --offline --release -p fe-lift -p fe-decode -p fe-pe`(13 秒)。`fe-lift lift FILE /dev/null --pdata`(`lift/out-git/vst1-results/` の census と同じ呼び方。種・hook・site-override なし)で closure 命令数・関数数・`bad_nodes`・未対応命令・**生成 C の大きさ(c_bytes)**を取った。wasm のビルドはしていない。20 個で 1 個あたり 0.1〜7.9 秒、最大 RSS 249 MB(FL Studio Mobile)。

### 4.2 結果(実測。`tables/census-sample.tsv`)

| 名前 | 言語 | DLL B | コード B | closure 命令 | 関数 | 未対応命令 | 生成 C B | C / コード B | Oz 推定 B | Oz / DLL |
|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| Soundgoodizer | Delphi | 6,407,680 | 2,740,028 | 669,406 | 18,372 | 147 | 57,148,430 | 20.86 | 21,030,622 | 3.28 |
| Fruity Flanger | Delphi | 3,716,608 | 1,649,288 | 409,846 | 12,505 | 97 | 35,066,473 | 21.26 | 12,904,462 | 3.47 |
| Fruity Parametric EQ 2 | Delphi | 4,036,608 | 1,873,339 | 460,320 | 13,127 | 97 | 39,397,891 | 21.03 | 14,498,424 | 3.59 |
| Fruity Delay 2 | Delphi | 3,731,968 | 1,677,759 | 415,834 | 12,565 | 97 | 35,582,251 | 21.21 | 13,094,268 | 3.51 |
| Fruity Balance | Delphi | 3,718,656 | 1,665,794 | 412,943 | 12,541 | 97 | 35,331,546 | 21.21 | 13,002,009 | 3.50 |
| Fruity Filter | Delphi | 3,780,608 | 1,708,355 | 421,023 | 12,542 | 97 | 36,045,824 | 21.10 | 13,264,863 | 3.51 |
| Frequency Shifter(前面) | Delphi | 3,876,864 | 1,771,601 | 440,344 | 12,958 | 97 | 37,688,527 | 21.27 | 13,869,378 | 3.58 |
| FLEX(前面) | Delphi | 5,194,752 | 2,461,277 | 607,856 | 17,243 | 138 | 51,910,933 | 21.09 | 19,103,223 | 3.68 |
| Harmor | Delphi | 6,478,336 | 3,159,035 | 761,815 | 19,604 | 169 | 65,017,788 | 20.58 | 23,926,546 | 3.69 |
| Patcher | Delphi | 7,056,384 | 3,261,658 | 812,912 | 24,534 | 146 | 69,245,557 | 21.23 | 25,482,365 | 3.61 |
| Sytrus | Delphi | 6,269,440 | 3,012,820 | 727,088 | 19,005 | 147 | 62,112,080 | 20.62 | 22,857,245 | 3.65 |
| Frequency Shifter の Engine | MSVC | 263,168 | 110,725 | 27,513 | 500 | 698 | 2,632,341 | 23.77 | 968,701 | 3.68 |
| FLEX engine_x64 | MSVC | 5,673,984 | 4,104,199 | 1,010,364 | 11,060 | **19,579** | 89,253,270 | 21.75 | 32,845,203 | 5.79 |
| FLEX engineV2 | MSVC | 8,119,808 | 5,909,276 | 1,314,118 | 12,611 | **191,344** | 115,883,097 | 19.61 | 42,644,980 | 5.25 |
| Hardcore | MSVC | 641,536 | 358,514 | 91,267 | 1,561 | 1,047 | 8,464,458 | 23.61 | 3,114,921 | 4.86 |
| Pitcher | MSVC | 446,464 | 270,602 | 69,454 | 1,233 | 892 | 6,503,697 | 24.03 | 2,393,360 | 5.36 |
| Kepler engine | MSVC | 2,717,696 | 781,080 | 192,456 | 1,892 | 1,352 | 17,731,171 | 22.70 | 6,525,071 | 2.40 |
| Morphine | MSVC | 1,745,408 | 1,368,658 | 321,477 | 2,684 | 12,367 | 27,732,498 | 20.26 | 10,205,559 | 5.85 |
| Fruity Slicer 2 engine | MSVC | 2,738,176 | 2,140,786 | 511,171 | 3,729 | 18,801 | 44,158,972 | 20.63 | 16,250,502 | 5.93 |
| FL Studio Mobile | MSVC | 12,141,568 | 9,119,494 | 2,065,861 | 16,492 | 23,764 | 178,145,037 | 19.53 | 65,557,374 | 5.40 |

(Oz 推定 = C × 0.368。`bad_nodes` はすべて 0、FL Studio Mobile だけ 2。closure は `--pdata` なので全 `.pdata` 関数が根=FLEngine での実績と同じ方式。)

- **Delphi と MSVC の倍率**(実測): コード 1 B あたりの C は Delphi 21.04 B(11 個の平均。範囲 20.58〜21.27)、MSVC 21.77 B(9 個の平均。19.53〜24.03)。**3.5% の差で、実質同じ**。DLL 全体に対する wasm の倍率が違って見えるのは、DLL の中のコードの割合が違うため(Delphi 46%、MSVC の engine 56〜78%。Kepler は 29%、Frequency Shifter の Engine は 42%)。FLEngine の実績(C 215.8 MB / 22.2 MB の DLL = 9.7 倍)は Delphi プラグインの 8.9〜10.0 倍(C / DLL)と同じ桁。
- **Delphi はほぼ完全に lift できる**: 未対応命令は 97〜169 個で、種類はほぼ「IAT 経由の indirect jmp(tail call)」の 1 種(Harmor だけ 5 種)。
- **MSVC の大きい engine は不完全**: FLEX engine 19,579 個、engineV2 191,344 個(Vmovups EVEX 18.0k、Vaddps 17.5k、Vsubps 15.1k、Vmulps 13.2k、Vfmadd231ps…)、FL Studio Mobile 23,764 個(Int3 4.9k、Vaddps 2.9k…)、Morphine 12,367、Slicer 2 engine 18,801。これらは **AVX/FMA/EVEX の経路で、cpuid による分岐の先**と読む(実行時にどちらへ行くかは**未確認**)。未対応の分は C に入っていないので、**上の Oz 推定は下限**(`vst1-results/tables/wasm_simd128_expansion.tsv` にも SIMD の膨張の記録がある。ここでは使っていない)。
- 生成 C が最大 178 MB(FL Studio Mobile)、wasm にすると V8 の単一モジュール上限 1 GiB(決定 0130)には十分収まる(NDI を除く)。**NDI の `Processing.NDI.Lib.x64.dll`(27.1 MB、コード 26.4 MB)は census していないが、同じ倍率なら C 約 570 MB、Oz 約 210 MB**(推測)で、サードパーティの NDI ランタイムであり FL の機能(ZGameEditor Visualizer のオプション)なので優先度は低い。

### 4.3 wasm の大きさの見積り(読み。倍率は他の担当が精密化中)

- 倍率: wasm Oz = 0.368 × C、wasm O2 = 0.765 × C、brotli(Oz の wasm)= 0.0717 × wasm(`performance-targets.md` §4b の 6 モジュール構成の実測を、プラグインにそのまま当てはめた。**プラグインのコードで同じとは限らない**=推測)。データの brotli は 0.30(DLL 全体の brotli q9 の実測値 0.298 を流用=推測)。
- **コード 1 B(ネイティブ)あたりの Oz の wasm は Delphi 7.74 B、MSVC 8.01 B**(21.04×0.368、21.77×0.368)。
- 配布するデータ = 非ゼロの `.text` 内データ+`.data`+`.rdata`+`.rsrc`(コードの機械語、`.pdata`、`.reloc`、`.idata` を除く)。現在の構成が「元の DLL をそのまま」データとして読む方式(70,573,808 B)であることとは違う前提なので、元の DLL をそのまま置く場合は Delphi で約 2〜2.5 倍のデータになる点に注意(この見積りの外)。
- ビルド時間の桁(読み。決定 0130 の「6 モジュール、C 872 MB で -Oz 7 分、-O2 59 分(JOBS=2)」から比例): 全プラグインの C は 333.8 MB × 約 21 = **約 7.0 GB**(重複を除かない場合)で、-Oz で約 1 時間、-O2 で約 8 時間(JOBS=2)。

## 5. 配布の設計の選択肢(やること 4)

**決めない。推奨は §9。** 数値の出し方: 「生」= wasm(Oz 見積り)+データ。「brotli」= 上の比率。「ネイティブ」= 対応する DLL の生。`tables/estimates.tsv`(全表)、`estimates-b-fle.txt`、`single-plugin-estimates.tsv`、`vs-engine-per-dll.tsv`。

### 5.0 基準になる数

| 範囲 | ネイティブの生 | ネイティブの brotli(q9、DLL ごと) |
|---|---:|---:|
| 全 181 個(x64) | 709,590,576 | 211,421,708(0.298) |
| 参照プロジェクト(7 種+Frequency Shifter の Engine+FLEX engineV2) | 39,130,112 | 12,269,147 |
| 参照プロジェクト(FLEX が engine_x64 の場合) | 36,684,288 | 11,379,984 |
| Fruity Flanger 1 個 | 3,716,608 | 1,105,915 |

### 5.1 比較表(Oz、倍率はネイティブの生に対する。brotli の欄の括弧は「ネイティブの brotli に対する倍率」)

| 案 | (1) 配布物の総量(R2、全 181 個) | (2) 参照プロジェクト 1 回の転送(生 / brotli) | 単独の 1 個目(Flanger、生 / brotli) |
|---|---|---|---|
| **A** 現状のまま(1 DLL = 1 wasm、共有なし)。全部 | wasm 2.60 GB + データ 0.32 GB = 2.92 GB(**4.11 倍**)、brotli 281 MB(1.33)。-O2 なら 5.72 GB(8.06 倍) | 171.9 MB(**4.39 倍**)/ 16.0 MB(1.30) | 14.6 MB(3.93 倍)/ 1.44 MB(1.30) |
| **B** 重複の共有(Delphi のコア+各プラグインの残り。データも共有) | 理想の下限 355 + 70 = **425 MB(0.60 倍)**、brotli 約 46 MB(0.22) | 半数以上の DLL が持つ関数を共有 + 残り(データ共有)= 81.8 MB(**2.09 倍**)/ 7.5 MB(0.61)。全体を 1 つ(k≥2)にすると 113.7 MB(2.91 倍)で逆に大きい | 14.6 MB(3.93 倍)—**1 個目は減らない** |
| **B-FLE** FLEngine を共有ライブラリにし、プラグインは FLEngine に無い物だけ | 全 Delphi 133 個の残り 699 MB(1.18 倍。プラグイン間の共有は未適用) | **66.9 MB(1.71 倍)/ 6.1 MB(0.50)**。Delphi 7 個だけなら 16.7 MB(0.54 倍)/ 2.0 MB | Flanger の残り 3.18 MB(**0.86 倍**)。Delphi 133 個の中央値 1.03 倍、p90 1.54 倍、最大 3.05 倍 |
| **C** 使うプラグインのモジュールだけ取得(遅延ロード) | A と同じ(置く量は同じ。**取る量が使う分だけ**) | A と同じ(171.9 MB。**全部ではなく使う分**) | A と同じ |
| **D** 関数単位のチャンクの遅延ロード(B と併用) | B と同じ置く量+チャンクの索引 | 起動で走る関数が 9% / 25% なら wasm の取得量は 9% / 25%(データは別):**14.0 MB(0.36 倍)/ 2.7 MB、25.9 MB(0.66 倍)/ 3.5 MB**(**割合は推測**) | 9% で 2.9 MB(0.77 倍)、25% で 4.9 MB(1.33 倍) |
| **E** ネイティブを(圧縮して)そのまま配り、実行時にブラウザ内で x86-64 → wasm に変換 | 709.6 MB(**1.0 倍**)、brotli 211 MB(個別)/ 連結して zstd-long なら 109 MB | 39.1 MB(1.0 倍)/ 12.3 MB(1.0) | 3.72 MB(1.0 倍)/ 1.11 MB |

読み方: 「総量」は R2 に置く量。brotli の欄の A は 0.397 倍(ネイティブの**生**に対して)= 281.4 / 709.6。**依頼者の基準を「ネイティブの生に対する生」で見ると A と C は 4 倍超で不可、B は 0.60 倍、B-FLE は参照プロジェクトで 1.71 倍、D が 1 倍未満になり得る**。

### 5.2 各案の詳細

**A. 現状のまま全部**
- (3) 待ち: 参照プロジェクトを開くとき、全部を取らず 7 種+Engine を取れば C と同じ。全 181 個を取るなら 281 MB(25 Mbit/s で約 90 秒、100 Mbit/s で約 22 秒)で非現実的。V8 の単一モジュール 1 GiB を、全部を 1 つの wasm にすれば超える(2.6 GB)ので、プラグインごとに別モジュールにする前提。
- (4) 実装: `a4-multi.sh` は NAME:DLL:BASE:SEEDS:OPTS で複数の DLL を**静的に 1 つのプログラムへ結合**する仕組み(読み。現在 6 モジュール)。ランタイムの `fle_realmod.c` は `RM_MAX = 8` の実在モジュールの表(名前・基底・エクスポート)で、`LoadLibraryW`・`GetProcAddress` が仮想 DLL を返す。プラグインを別 wasm にするには、(i) 動的に読み込む wasm(Emscripten の MAIN_MODULE/SIDE_MODULE または自前の読み込み。**未検証**)、(ii) 181 個にそれぞれ重ならない基底(Delphi は全部 ImageBase 0x400000、MSVC は 0x180000000 なので再配置が要る。`lift/out-*/` にリベース済みの派生物が複数あるのは既存の手順)、(iii) `RM_MAX` の拡張が要る。
- (5) 失敗の兆候: 取得量でなく、**1 プロジェクトで読む wasm の合計(参照プロジェクトで 156 MB)のコンパイルとメモリ**(`performance-targets.md` §4 の起動の内訳: ブラウザの compile 3.0 秒/320 MB 級=9.4 ms/MB、`jspi-results` の node: 0.33 秒/389 MB=0.9 ms/MB)。

**B. 重複の共有**(読み)
- (3) 待ち: §5.1 の転送量 ÷ 回線。参照プロジェクトの B は 7.5 MB(25 Mbit/s で 2.4 秒、100 Mbit/s で 0.6 秒)。2 回目は共有モジュールが 6 MB 以下の応答に収まればキャッシュされる(§7)。
- (4) 実装(大): (i) **生成器の変更**: 共有関数のデータ参照を、モジュールごとの表(GOT 的)経由にする。純粋な計算だけの関数は 28%(§3.4)なので、残りの 72% は参照表を介す。(ii) モジュール間の呼び出し(共有モジュールの関数 → プラグイン固有の関数への callback、VMT の関数ポインタ → 関数表の番号。生成 C は `call_ind:dispatch` で VA から関数を引いている)。(iii) 関数の同一性の判定をビルドに組み込む(`dupscan` の h2 が使えるが、h2 は 1 段の呼び先まで)。(iv) データの共有: 内容定義チャンクの表+モジュールごとの再配置(Delphi の `.reloc` は DLL あたり 147〜200 KB、135 個で 22.3 MB)。
- (5) 失敗の兆候: 参照表経由にしても生成 C がモジュール間で同じにならない(= 比較に使った正規化が足りない。例: 関数内にジャンプ表の絶対アドレスがある、`lea` で取った関数のアドレスが共有側と固有側で食い違う)、グローバルの状態が混ざる(§3.4)、浮動小数の丸め(同じ C なので同じはずだが未検証)。
- 注: **1 個目は減らない**。5.1 の「1 個目 3.93 倍」は B では変わらない(自分のコードを全部取る)。

**B-FLE. FLEngine を共有ライブラリにする(B の特殊形)**
- 根拠(実測): Delphi プラグインのコードの 70%(DLL ごとの中央値 75%)は FLEngine が持つ関数と同一。FLEngine は常に読まれるので、プラグインはそれと違う物だけを持てばよい。残り(コードのうち FLEngine に無い物+データのうち FLEngine に無い物)を DLL ごとに計算すると、**参照プロジェクトの 7 個で合計 16.7 MB=ネイティブ 30.7 MB の 0.54 倍**(プラグイン間の共有も適用。FLEngine に無い物は 1.69 MB のコード=wasm 13.1 MB+データ 3.6 MB)、**1 個ずつ見ると Delphi 133 個の中央値が 1.03 倍、p90 1.54 倍、最大 3.05 倍**(`tables/vs-engine-per-dll.tsv`)。参照プロジェクトの 7 種: Flanger 0.86、Balance 0.88、Delay 2 0.89、Filter 0.96、Frequency Shifter 0.97、PEQ 2 1.14、FLEX 1.28、Soundgoodizer 1.45。
- 参照プロジェクト全体は、MSVC の engine(FLEX engineV2 が wasm 約 47.3 MB〔実測 C からは 42.6 MB〕+データ 1.9 MB と Frequency Shifter の Engine 約 0.9 MB)が残るので **66.9 MB(1.71 倍)/ brotli 6.1 MB**。FLEX engineV2 の関数の 34% は dsp_ippv2 と同一なので、さらに減らせる(未適用)。
- (4) 実装: B と同じ生成器の変更+FLEngine の側も参照表経由で呼べる形にする(FLEngine を再 lift する必要がある可能性。**未確認**)。(5) 兆候: FLEngine の RTL 関数をプラグインが呼ぶと FLEngine の状態(ヒープ等)を触る場合(§3.4 の参照表が効いていないとき)。

**C. 使うプラグインのモジュールだけ取得**
- (3) 待ち: A の「使う分」。参照プロジェクト 16.0 MB(25 Mbit/s で 5.1 秒)。ネットワーク以外に、起動後にプロジェクトを開くときにプラグインの `LoadLibraryW` で止まる時間。**JSPI で `LoadLibraryW` の中から待てる**(`lift/out-git/jspi-results/jspi-real-wasm.md` §0: 実 wasm で `LoadLibraryW` を 34 回中断して一致。1 回の中断・再開は微小タスクで 0.22 µs、`setTimeout(0)` で 1.1 ms)。fetch+compile 自体はネットワークとコンパイルの時間。**ただしエンジンの音声処理スレッドの中では待てない**ので、プロジェクトを読むとき(または FLP を解析した時点)に、使うプラグインを全部先に取得して instantiate する必要がある(推測)。
- (4) 実装(小〜中): プラグインの wasm の動的な登録(`RM_MAX` の拡張、基底の割当、`GetProcAddress` の仮想エクスポート表を実行時に追加、マニフェスト「プラグイン名 → wasm のキー・基底・データ」)、FLP から必要なプラグインを知る経路(`reference-project.md` の解析は FLP に GUID があることを示している)。Engine DLL(Frequency Shifter の Engine、FLEX の engine/V2)は前面 DLL が `LoadLibrary` するので同じ仕組みで取得。
- (5) 失敗の兆候: 取得前にエンジンがプラグインの DLL を要求する(起動時のプラグイン DB の走査〔`ILPluginScanner64.exe`、`.nfo` 141 個があるので走査は `.nfo` で済む可能性が高いが**未確認**〕、ミキサーのプラグインの UI)、`GetProcAddress` の戻りに関数ポインタが要る。
- **S1(モジュールごとに 2 倍以下)は満たさない**(Delphi 3.3〜3.7 倍、MSVC 2.4〜5.9 倍)。

**D. 関数単位のチャンクの遅延ロード**
- 前提(依頼文): 起動で実行される関数の割合は FLEngine で約 9%(`performance-targets.md` と依頼文)。**プラグインで実行される関数の割合は未測定**(プラグインの UI の作成+DSP+再生。曲の再生中に実行される割合は更に大きい)。
- (1) 置く量は B と同じ+索引。(2) 取得量は割合に比例(§5.1)。9% / 25% は仮の値。(3) 待ち: 最初に必要な関数を先に取ってからコンパイル。**音声スレッドは待てない**ので、実行される関数の集合をプラグインごとに事前に測って(ネイティブでの実行記録。既存の gdb 計測〔`gdbcount.py`〕と同種)、プラグイン作成時に取得する。
- (4) 実装(大): 関数群を小さな wasm モジュール(チャンク)に分け、未取得の関数への呼び出しを JSPI の停止スタブ経由にして fetch → compile → 関数表に登録する。生成 C の側で関数の「チャンクへの割り当て」と、チャンク間の呼び出しを間接呼び出しにする必要がある。(5) 兆候: 1 回目の呼び出しの遅延が音切れに出る(P3)、チャンク数が増えてコンパイルのオーバーヘッドが増える、V8 の関数表・モジュール数の上限。

**E. ネイティブを配って実行時に変換(JIT)**
- 大きさ(実測): 1.0 倍。全体 709.6 MB、DLL ごとの brotli q9 で 211 MB(0.298)。連結して `zstd -10 --long=30` なら 109 MB(0.154)だが、ブラウザの `DecompressionStream` は gzip/deflate だけ(決定 0130)なので、zstd/brotli の復号器を wasm で持つ必要がある(未測定)。参照プロジェクトは 39.1 MB(brotli 12.3 MB)。
- 公開資料(2026-11-03 取得):
  - CheerpX の概要(https://cheerpx.io/docs/overview): 「CheerpX is a two-tier emulator for the x86 architecture … There is both an interpreter and a sophisticated JIT compiler that is able to generate efficient WebAssembly representations for hot code.」「CheerpX enables you to run existing **32-bit** x86 native binaries and Linux distributions」「CheerpX currently focuses on user mode code, on top of a Linux-compatible system call interface」。npm のページ(https://www.npmjs.com/cheerpx、2026-10-07): 「Currently, only 32-bit x86 binaries are supported. Future updates will include support for 64-bit binaries」。→ **x86-64 の Windows DLL を扱えない**(Linux の syscall 層で、Win32 の層は無い)。
  - 速度の自己申告(https://labs.leaningtech.com/blog/cx-10): 「CheerpX is expected to run complex applications **5x-10x slower than native**, with the slowdown being as low as 2x-3x for binaries that only stress the best optimized parts of the CheerpX execution pipeline」。
  - v86(https://github.com/copy/v86): 「Machine code is translated to WebAssembly modules at runtime in order to achieve decent performance.」。32 ビットの PC エミュレータ(Pentium III 相当+SSE2。紹介記事 https://korben.info/en/x86-webassembly-emulator-old-os-browser.html)。
  - wasm の制約: Chrome は main thread の同期コンパイルの上限を 4 KB から 8 MB に拡大(https://chromestatus.com/feature/5099433642950656)。CheerpX の講演(https://www.youtube.com/watch?v=7JUs4c99-mo、自動字幕)は「we cannot just generate some webassembly code and jump into it … you need to put it into a module, then you compile the module and this is a synchronous operation」と述べ、1 関数ごとか大きなモジュールにまとめるかのトレードオフがあるとする。
- 速度・複雑さ(読み): 本プロジェクトは **AOT の lift(fe-lift が静的に命令を C に直す。ジャンプ表のデータフロー解析、closure の事前計算、Win32 のランタイムを前提)**で、実行時の変換器は無い。JIT には (i) x86-64 の命令の網羅(SSE/AVX/FMA/x87。fe-lift の census でも AVX は未対応が残る)、(ii) 基本ブロック単位の動的な発見(間接ジャンプ・ジャンプ表)、(iii) wasm のバイナリを直接出す生成器(fe-lift は C を出す)、(iv) 最初に走らせる低速の層(CheerpX は先に解釈器)が要り、**起動 10 秒(P1)・120 fps(P2)・音切れなし(P3)**との相性が最大の懸念。大きさの問題は消えるが、別の問題に置き換わる(推測)。
- (4) 既存の機構との関係: 再利用できるのは fe-decode/fe-pe の読み取り程度。`fle_realmod`・仮想 DLL・Win32 ランタイムは AOT の生成 C を前提にしている。
- (5) 失敗の兆候: warm-up の長さ、JIT したコードの破棄(wasm のコードメモリは長寿命を前提と言われる=ハッカーニュースの一コメントのみ、確かめていない)。

### 5.3 待ち時間の概算(転送 ÷ 回線 + 準備。参照プロジェクト、読み)

| 案 | 転送(brotli 見積り) | 25 Mbit/s | 100 Mbit/s | wasm の準備(生の wasm × 0.9〜9.4 ms/MB) | 2 回目 |
|---|---:|---:|---:|---:|---|
| A/C | 16.0 MB | 5.1 秒 | 1.3 秒 | 156 MB → 0.14〜1.5 秒 | プラグインごとの応答が 6 MB 以下(0.1〜5.4 MB。FL Studio Mobile は 5.4 MB で境界、NDI は約 15 MB で超える)なら HTTP キャッシュに載る(決定 0130 の閾値からの**推測**。この大きさでは未測定) |
| B | 7.5 MB | 2.4 秒 | 0.6 秒 | 74.5 MB → 0.07〜0.7 秒 | 共有モジュール(Delphi コア約 12 MB の wasm、brotli で約 0.9 MB)は 6 MB 以下 |
| B-FLE | 6.1 MB | 1.9 秒 | 0.5 秒 | 約 62 MB → 0.06〜0.6 秒 | 同上 |
| D(25%) | 3.5 MB | 1.1 秒 | 0.3 秒 | 18.6 MB → 0.02〜0.2 秒 | チャンクごとに小さいので全部載る |
| E | 12.3 MB(q9) | 3.9 秒 | 1.0 秒 | JIT の warm-up(**未測定・不明**) | 同上 |

## 6. 依頼者の基準を満たすか(2 倍、目標 1.2 倍)

| 基準の取り方 | A | B | B-FLE | C | D | E |
|---|---|---|---|---|---|---|
| (a) R2 の総量(生 / ネイティブの生、181 個) | 4.11 ✗ | **0.60 ✓✓** | (Delphi のみ 1.18 ✓) | 4.11 ✗ | B と同じ | 1.00 ✓✓ |
| (b) 総量の brotli / ネイティブの brotli | **1.33 ✓(1.2 は ✗)** | 0.22 ✓✓ | — | 1.33 | — | 1.00 ✓✓ |
| (c) 参照プロジェクト 1 回の転送(生) | 4.39 ✗ | 2.09 ✗(僅差) | **1.71 ✓** | 4.39 ✗ | 0.36〜0.66 ✓✓ | 1.00 ✓✓ |
| (d) プラグイン 1 個目(Flanger、生) | 3.93 ✗ | 3.93 ✗ | **0.86 ✓✓** | 3.93 ✗ | 0.77〜1.33 ✓ | 1.00 ✓✓ |
| (e) モジュールごと(S1 の文言どおり) | ✗(3.3〜5.9) | 共有モジュール+残りで定義が変わる | Delphi の中央値 1.03 ✓ | ✗ | 定義が変わる | ✓ |

- **MSVC の大きなエンジンは重複が少なく**(ユニーク 47%、dsp_ippv2 と共有できる IPP 部分が 16%)、wasm の膨張が直接効く: Oz で DLL の 5〜6 倍(FLEX engine 5.8、FL Studio Mobile 5.4)。FLEX engineV2 だけで B-FLE の参照プロジェクト 66.9 MB のうち約 49 MB(73%。wasm 47.3 MB+データ 1.9 MB。合計は言語平均の 21.77 を使った値で、engineV2 の実測 C からは 42.6 MB)を占める。**共有ではこの 2 倍に届かず、膨張率の改善(別の担当)か、D(関数単位の遅延ロード)が要る**。さらに AVX 系の未対応が残っており、その分が足されると膨張は更に大きい。
- (b) の見方に注意: brotli 後はネイティブの機械語(約 3.5 倍の圧縮)より lift 後の wasm(約 14 倍の圧縮)の方がよく縮む。このため**生の 4 倍超でも brotli では 1.3 倍で済む**。依頼者の基準が「生の大きさ」「転送量」のどちらかは、依頼文の「100 以上のプラグインで 3 GB 以上の転送量」が**生の合計**を指しているように読めるが、確かめていない。

## 7. R2 / Workers の制約と、プラグインの保存・配信(やること 5)

公開資料の取得日 2026-11-03(各ページの更新日は 2026-10-29/30)。

**Workers Static Assets**(https://developers.cloudflare.com/workers/platform/limits/):
- 1 ファイル **25 MiB**、1 バージョンあたりのファイル数 20,000(Free)/ 100,000(Paid)。→ 総量でなく**ファイルの大きさ**が先に効く: 現在の 6 モジュール wasm(320 MB)や、プラグインの wasm の大きいもの(生で 13〜65 MB。NDI は約 210 MB 推定)は 25 MiB を超えるので Static Assets に置けない。brotli の前圧縮の形(0.9〜5.4 MB)なら置けるが、配信側で `Content-Encoding` を付けるにはヘッダの設定が要る(`_headers` で可否は**未確認**)。
- Worker 自体は 64 MiB(非圧縮)、メモリ 128 MB(isolate)、応答本体の大きさに上限なし、CPU 時間は待ち時間を数えない。→ 大きな応答は R2 のストリーム(`r2.get(key).body`)をそのまま返す形にする(バッファに貯めない)。
- Cache API の 1 オブジェクト 512 MB、CDN キャッシュ 512 MB(Free/Pro/Business)。Free プランは 1 日 10 万リクエスト・1 リクエストの subrequest 50(R2 の呼び出しも含む)。

**R2**(https://developers.cloudflare.com/r2/platform/limits/、https://developers.cloudflare.com/r2/pricing/):
- 1 オブジェクト 4.995 TiB、単一 PUT 4.995 GiB、バケットの容量・オブジェクト数は無制限。**egress は無料**。Standard の保存は $0.015/GB-月、Class B(読み出し)$0.36/百万回、無料枠は保存 10 GB・Class A 100 万回・Class B 1,000 万回(月)。→ 全部を A の形で置いても brotli で 0.28 GB、生で 2.9 GB(月 $0.04 以下)。**費用は問題にならない**。
- Range GET は対応(Workers API の `range`、S3 の `Range`。リリースノート: 206 を返す)。ただし**`Content-Encoding: br` を付けたオブジェクトの Range は圧縮後のバイト列に対する物**で、ブラウザは途中からは復号できない。D のチャンクは**チャンクごとに別オブジェクト**(または gzip の独立ブロックで索引)にするのが無難(読み)。
- 公開は `r2.dev` がテスト用(数百 req/s で 429、帯域も制限)、本番はカスタムドメイン。Worker を介すと Free の 1 日 10 万リクエストに数えられる。D では 1 プロジェクトで数百リクエストになり得る。

**ブラウザ側の保存**(決定 0130、`browser-feasibility.md` の実測。Chrome 156、隔離プロファイル):
- HTTP キャッシュ: **1 つの応答が 6 MB までは 2 回目にキャッシュから出る、7 MB 以上は毎回サーバーへ**(5/6 MB が hit、7/8/16 MB が miss)。→ プラグインのモジュールを「brotli で 6 MB 以下」に分ければ標準のキャッシュが効く(A/C の 1 プラグインは 0.1〜5.4 MB)。
- Cache API: 復号後 237・240 MB の `put` は成功、300・512 MB は失敗(`UnknownError`)、同一 context で 160〜237 MB を続けて(累計約 1 GB)入れた後は `QuotaExceededError`。→ プラグインごとに分ける限り問題ない。IndexedDB の 1 値の上限は 133,169,152 B。`WebAssembly.Module` は IndexedDB に保存できない(DataCloneError。`browser-feasibility.md`)ので、再訪問でもコンパイルはやり直し(コード キャッシュが効くかは**未確認**)。OPFS は使える。quota は 4.3〜4.7 GB(プロファイルが tmpfs の値。ディスクに依存)。
- V8 の単一モジュール上限 1 GiB(Chrome 156 で実測)。プラグインごとの wasm は最大でも NDI を除いて 65 MB(推定)なので問題にならないが、A の全部を 1 つにすることはできない(2.6 GB)。

**現実的な保存・配信の形**(意見): R2 に**内容アドレス(SHA-256)のオブジェクト**として置き、`Cache-Control: public, max-age=31536000, immutable`、`Content-Encoding: br`(事前圧縮は q11 で 0.8 s/MB の桁。決定 0130)。マニフェスト「FL のビルド → プラグイン名 → [wasm のキー、基底、データのキー]」だけを短い TTL で配る。カスタムドメインの R2 から直接(Worker を介さない)返せば Worker の上限に当たらない。B/B-FLE ならオブジェクト数は 1 つの共有モジュール+181 個の残り+データのチャンク表で、A と同程度。

## 8. 失敗の兆候と未確認の一覧

- **wasm は 1 つも作っていない**。wasm の大きさは C の実測 × 0.368(6 モジュールの比)の見積り。size1 の担当の結果で全体が動く(例: 倍率が半分になれば A の Delphi は 2 倍弱に近づく)。brotli 率 7.17% もプラグインのコードで同じとは限らない。
- census は **全 `.pdata` 関数を根にした closure**(実際の `LoadLibrary`→`CreatePlugInstance` から到達する関数より多い可能性、または種・hook・site-override が要って少ない可能性)。MSVC の大きい engine は AVX/EVEX が未対応のまま(その分は C に無い)。NDI・ZGameEditor の補助 DLL(Bullet/Spout/Skelet/Nano)・FL Studio Mobile の一部は census していない。
- 「ユニーク」の判定は h2(呼び先 1 段)。仮想呼び出し・間接呼び出しの先、関数内のジャンプ表の中身、`.pdata` 外で見つからなかった本物の葉関数(データ側に数えた)は未考慮。372 組の検算で 369 組が整合(0.8% は疑わしい)。
- 共有の実現可能性(§3.4)は読みで、生成器で試していない。純粋な関数が 28% というのは「RIP 相対を持たない」だけの基準で、関数の呼び先に依存するデータ参照は考えていない。
- 参照プロジェクトの FLEX が使う engine(`engine_x64.dll` か `engineV2_x64.dll`)は未確認(大きい方を採った。小さい方だと A は 156.9 MB、B-FLE はおよそ 1.5 倍)。起動時のプラグイン DB の走査で全 DLL を読むか(`.nfo` 141 個で済むか)は未確認。
- D の「実行される関数の割合」(9% / 25%)はプラグインでは未測定。E は公開資料の引用だけで、x86-64 の JIT の実現は試していない。
- ネットワークの時間は転送量 ÷ 回線の算術で、実回線・CDN・TLS は未測定(決定 0130 の実測では算術と 0.3〜0.9 秒の範囲で一致)。
- `Plugins\` 配下の DLL 以外のファイル(wav・jpg 等、LZMA 後約 104 MB)と `Data\` は範囲外。
- 時刻: この環境の時計が不安定で、各作業の所要時間は記録していない。

## 9. 推奨(意見。決めるのは PM)

1. **単位を「プラグイン 1 個 = 1 モジュール」にして使う分だけ取得(C)**を土台にする。理由: 参照プロジェクトの転送が 全部を取る場合の 1/17(281 → 16 MB)になる、既存の `LoadLibraryW`/`GetProcAddress` の仮想化と JSPI(実 wasm で 34 回の中断を確認)に素直に乗る、実装が小〜中。ただし S1(モジュールごと 2 倍)は満たさない。
2. **Delphi 製(133 個)は FLEngine を共有ライブラリとして使い、FLEngine に無い物だけを配る(B-FLE)を、生成器で成立するか先に確かめる**。成立すれば 1 個目から 0.86〜1.45 倍(参照の 7 個)、全体 1.18 倍(共有前)、7 個で 0.54 倍と、依頼者の 2 倍・1.2 倍の両方に届く唯一の案。確かめ方: Flanger と Balance(コードの 98.7% が同一)の 2 個で、データ参照を参照表経由にして生成 C が一致するか、FLEngine の関数を呼ぶ形にできるかを見る。
3. **MSVC の大きい engine(FLEX・FL Studio Mobile・Morphine・Slicer 2 など)は共有で減らない**。膨張率の改善(size1)と AVX 系の対応が先。dsp_ippv2 と同一の IPP 部分(FLEX engineV2 の 34% など)は共有で減らせる。
4. D は、まず**プラグイン 1 個(Soundgoodizer)で、プラグイン作成〜再生中に実行される関数の割合を計測**してから判断する(音声スレッドは待てないので事前取得の集合が要る)。
5. E は現状では研究枠。CheerpX が 32 ビット専用である以上、x86-64 の変換器を新規に作る規模になる(推測)。
6. 総量の基準は「R2 の総量(生)」「参照プロジェクト 1 回の転送(生・brotli)」「プラグイン 1 個目」の 3 つに分けて持つ方が、設計の良し悪しを見誤りにくい(§6)。

## 10. 再現手順

```
# 抽出(インストーラは読むだけ)
python3 tools/nsis_all_sections.py INSTALLER --list > tree-all-sections.tsv
python3 tools/nsis_all_sections.py INSTALLER --extract lift/out-plug1/dll '^Plugins\\.*\.dll$' > extract-plugins.tsv
python3 tools/inventory.py lift/out-plug1/dll extract-plugins.tsv tables/dll-inventory.tsv
# 重複
cd tools/dupscan && CARGO_TARGET_DIR=lift/out-plug1/target cargo build --release --offline
lift/out-plug1/target/release/dupscan lift/out-plug1/dup/list.tsv lift/out-plug1/dup   # list.tsv: id TAB path(FLEngine を最後に)
python3 tools/analyze_dup.py DUPDIR tables/dll-inventory.tsv tables
python3 tools/analyze_dup2.py DUPDIR tables/dll-inventory.tsv tables ; python3 tools/analyze_tiers.py DUPDIR tables/dll-inventory.tsv tables/tiers-N7.tsv 7 300
python3 tools/verify_pairs.py DUPDIR list.tsv 400 7
# lift の census
CARGO_TARGET_DIR=lift/out-plug1/target cargo build --offline --release -p fe-lift -p fe-decode -p fe-pe   # lift/ で
lift/out-plug1/target/release/fe-lift lift DLL /dev/null --pdata > census/X.txt
# 見積り
python3 tools/estimates.py DUPDIR tables/dll-inventory.tsv tables/census-sample.tsv BROTLI_TSV tables
python3 tools/analyze_vs_engine.py DUPDIR tables/dll-inventory.tsv tables/vs-engine-per-dll.tsv ; python3 tools/analyze_vs_engine_project.py DUPDIR tables/dll-inventory.tsv BROTLI_TSV tables/estimates-b-fle.txt
```
(`DUPDIR` = `lift/out-plug1/dup`、`BROTLI_TSV` = DLL ごとの brotli q9(node の zlib、LGWIN 24)の `tables/brotli-q9-per-dll.tsv`。)
