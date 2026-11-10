# 0168: dsp_ippv2 の、固定した CPUID では選ばれない CPU 別の IPP コード(case 1 以上とそこからしか届かない関数)を生成 C から外す(`fe-lift --prune-list`、opt-in)。判定は 3 段階(強 S / 中 M / 弱 W)、外した関数が実行されれば大きな音で止まる。S+M で dsp の生成 C は 481 MB → 114 MB(−76%)、エンジン全体の wasm は D の 233.9 MB → 155.8 MB(3.31 → 2.21 倍)、16 スナップは D と全行同一。S1 は未達。仮の決定

状態: **仮の決定(すべての選択)**。依頼は PM(番号 0168 は PM の割り当て、担当 prune1)。調査・実装・測定・文書は作業者(prune1)。作業は HEAD **fdfa1dc**(決定 0161)の `git archive` から始め(`lift/out-prune1/src/`)、作業中に HEAD は d9af868(決定 0163・0164・0165 を含む)まで進んだ。差分 `prune1.diff` は **d9af868 の `git archive` に `patch -p1 --dry-run` が rc=0・fuzz/offset/reject なしで当たる**(MERGE-NOTES)。測定は fdfa1dc の runtime(決定 0163 のキャッシュなし)で行った。

区分の凡例は決定 0152 と同じ。**実測**＝このセッションのコマンドの出力を見た事実(2026-11-03 JST)。**読み**＝コード・文書を読んだ解釈で、走らせていない。**出典なしの選択**＝根拠が無く作業者が選んだ値(`S-prune-…`、§10)。**未確認**＝確かめていない。**概算**＝別の構成の測定値から引いた数。

**冒頭の三点**

1. **何を実測したか**: dsp_ippv2(Intel IPP 2021.7)の生成 C(25,370 関数、491 MB)の CPU 振り分けの構造(選択子スタブ 433、選択変数 4、表スロット 3,091)と、固定した CPUID(SSE2 のみ)の下で到達できない関数の数(強 S 5,560、中 M 12,929、弱 W 6,546、入口に入った K 335)。それを外す opt-in の `fe-lift --prune-list` を実装し、D 構成(決定 0161)に足した S のみ(PS)と S+M(PSM)のエンジンを、**native -O0 と wasm(node)で 16 スナップ、Chrome 156 で最初の idle まで、実際に走らせた**。生成 C・wasm・brotli の大きさ、Chrome と node の CPU 秒、全体テスト、既定の生成 C の同一性、止まる事象の有無(陰性対照を含む)、同梱プラグイン 134 個と VST3 の dsp_ippv2 の使い方。
2. **結論**: dsp の生成 C は S+M で 481 MB → 114 MB(−76%)、エンジン全体の wasm は D の 233.9 MB → S 190.0 MB(2.69 倍)→ **S+M 155.8 MB(2.21 倍)**。16 スナップ(native と wasm)は D と全行同一、除いた関数が実行されて止まる事象は 0(陰性対照は VA つきで止まる)。**S1(2 倍)は未達。起動(P1)にはほぼ効かない**(Chrome の CPU −6%〔14 対すべて PSM が小さい〕、ダウンロード+展開の段 −24%)。S は環境の定義だけに基づき、M はデータ表のモデル(仮定)に基づく。**開発中に自分で欠陥を 1 件見つけて直した**(§4.3)。
3. **未確認**: 実機 Windows との照合(0)。M のモデルの検定は 16 スナップの dsp 335 関数だけ(再生・書き出し・プロジェクト・プラグインは未実施)。『IPP の全 export を引数を変えて呼ぶ』は未実施。Chrome の壁時計は load ≤ 8 の窓が無く主張できない。決定 0163 を含む runtime での再測定なし。(D の再現は確認済み: §5.1)詳細は §11。

## 0. 先に書くこと(進捗の誠実性。AGENTS.md / `docs/policies/progress-integrity.md` に従う)

**(1) 実エンジンが実行された事実(実測)**:
- D 構成(決定 0161: `--dead-flags` + `--outline-shared`〔FLEngine 以外〕+ `-DFLE_LOWREGS -DFLE_LOWREGS_X` + 起動 20 単位の always_inline + 決定 0162 の dsp HLE)に、**dsp の除去(`fe-lift --prune-list`)を足した**エンジン 3 つを実際に走らせた。(a) **native -O0**(生成 C を clang でコンパイル)で `cmp-short.g3` の 16 スナップを、D・PS(除去 S)・PSM(除去 S+M)で走らせ、g3 の 71 行が**全行同一**(仮想時計 `clock_ms` 3601776 から、面 SHA idle `895634156…`・収束 `4fa213b0…`・Enter 後 `dd416725…`、擬似プリエンプションの進捗 18,284,440、HLE の呼び出し 120/120)。(b) **wasm**(emcc -Oz → `wasm-opt -Oz`、配布する物と同じ)を node で走らせ、PSM の 16 スナップの 82 行(g3-snap 16、g3-surface 34、g3-wintable 16、g3-progress 16)が **b1 の D の node 走行と同一**(違うのは末尾の経過秒だけ)。S の wasm(wasm-opt 済み)の node 走行も 82 行が D と同一。 (c) **Chrome 156**(隔離 headless)で D と PSM を交互に開き、最初の idle に達した(§5.4)。
- 代役・モック・画像の再生・事前生成物・期待値の貼り付けは**使っていない**。既定の入口(`/assets/manifest.json`)は触っていない。バンドルは私の private な `vite preview`(127.0.0.1:4181)の `/assets/fl-engine-prune1-PSM/` だけ。

**(2) 代役・推測を含む範囲**: 無い。ただし次は**代理の測定環境**(この Linux VM、16 コア、SwiftShader、localhost、キャッシュ無効)の値で、依頼者の PC の値ではない。**実機 Windows との照合は 0 件**(実機の FL Studio が AVX の CPU で選ぶ層〔Wine で観測した選択子 6、決定 0162〕と lift の層〔0〕は違う。この差は除去の前から決定 0118 の帰結で、除去は変えない)。P2(120 fps)・P3(音)は**測っていない**。

## 1. 先に結論(実測。概算は概算と書いた)

| 項目 | 結果 |
|---|---|
| dsp の構造 | export 446(FLEngine が引くのは 420、プラグイン 134 個も同じ 420)。CPU 振り分けは**選択子スタブ 433 個**(`movsxd [sel]; lea table; jmp [table+idx*8]`)・選択変数 4 つ・表スロット 3,091(case -1 = 遅延 init、0〜5、一部 6〜8)。CPUID は SSE2 のみに固定なので、実測で**選択子は 0**(終了時の値)。case 3 以上は AVX/AVX-512 の系統。閉包 25,370 関数・生成 C 491 MB のうち、case -1/0 から届く(モデル)のは 6,881 関数(23%) |
| 判定(3 段階) | **強 S** 5,560 関数(C 183.6 MB = 37.6%): 環境の定義だけで届かない。**中 M** さらに 12,929 関数(C 190.6 MB): 根を FLEngine が引く 420 export に絞りデータ表のモデルを足すと届かない。**弱 W** 6,546 関数(C 111 MB): モデルでは届きうるが 16 スナップで入口に入らない(**除かない**)。K 335 関数: 入口に入った。S+M = 18,489 関数(C 374 MB = 76.6%)。16 スナップで入口に入った 335 関数は全部、除かれずに残る(モデルの到達集合に入る。反例 0) |
| 実装(opt-in、既定不変) | `fe-lift --prune-list FILE`(`fe-decode` の `analyze_with_prune`、`prune.rs`、`runtime/fle_prune.h`)、判定ツール `tools/prune/judge.py`(TSV を出す)、`app/scripts/build-engine-bundle.sh` の `PRUNE_LIST`。除いた関数を残ったコードが名指すと `fle_pruned(VA)` で abort する stub。**既定の生成 C はバイト一致**(dsp 491,094,508 B `2c880576…`、FLEngine 215,811,678 B `cd9107cef4cbef14…47029e50`)。**開発中の欠陥を 1 件自分で見つけて直した**(表ジャンプの受理/棄却が変わり、擬似プリエンプションの数がずれて面 SHA が D と不一致。区分 U。§4.3) |
| 大きさ(実測) | dsp の生成 C(D のオプション)481.3 MB → S 300.0 MB → **S+M 114.0 MB**。エンジン全体の wasm(wasm-opt 後): D 233.9 MB(3.314 倍) → S 190.0 MB(2.692 倍) → **S+M 155.8 MB(2.208 倍、−33.4%)**。brotli q11: D 16.57 MB → S+M 12.00 MB。**S1(2 倍 = 141.1 MB)は未達**(S+M で差 14.7 MB)。D を一から作り直した wasm は記録の D と**バイト同一**(sha256 `80a5c36f…`)なので、この比較は同じ手順同士 |
| 一致(実測) | D と、native -O0 の 16 スナップ g3 71 行が S・S+M とも**全行同一**(クロック・面 SHA・擬似プリエンプション・HLE)。S+M の wasm(wasm-opt 済み)を node で走らせた 82 行は D と同一。S の wasm(node)も同一。 |
| 止まる事象 | 正規ビルド(S・S+M)の 16 スナップ・起動で **0 件**。陰性対照(入口に入る関数を除去リストに足す)は 15,728 入口で VA つきの abort。「IPP の全 export 呼び出し」は**未実施** |
| 起動・compile(Chrome 156、この機械、D と交互) | D と PSM を交互に、set 1 の 8 対 + set 2 の 6 対 = **14 対(28 走行)**。load は全走行で 24〜44(load ≤ 8 は 0 件)なので**壁時計は主張しない**。Chrome の CPU 秒の中央値 D 17.88 → PSM 16.82(−5.9%)、対の差(D − PSM)は **14 対すべて PSM が小さい**(中央値 1.24 s)。ダウンロード+展開の段 4.46 → 3.40 s(−24%)。**起動(P1)への効きはほぼ無い**(V8 は呼ばれた関数を遅延コンパイルするので起動 CPU は実行される関数で決まる)。node の 16 スナップの CPU 秒は D 21.28、PS 19.95、PSM 19.94(ノイズ内) |
| テスト | 全体 **results=153 passed=2128 failed=0**(基準 152/2118/0 + 新しい `prune_list` 実行ファイル 6 テスト + `prune.rs` 単体 4)。入力をつないで再実行した 11 実行ファイルは SKIPPED-lines=1(基準と同じ)。FRUITY 無効の a4: **942/942、895/895、functions 5,310、entries 1,009,731 で基準と全部一致**(`a4-fruity.sh off`、パッチ済みの fe-lift とパッチ済みの木。gen は同じ OUT の再利用、build 66 s。`logs/a4off.txt`) |
| 他の根・環境の前提 | プラグイン 134 個(Fruity)は FLEngine と同じ 420 名を引くだけ(増分 0)。VST3 は dsp_ippv2 に触れない。S は『CPUID 固定』と『`ippSetCpuFeatures` 不呼び出し』に依存し、崩れれば stub が abort(§7) |
| PM の決定 | S だけ(B)か S+M(C)か(§9)。推奨(読み): 既定は B、S1 の距離を詰めるなら C を検定つきで |

## 2. dsp_ippv2_x64.dll の構造(実測)

入力は `lift/out-ext/dsp_ippv2_x64.dll`(39,942,656 B、SHA-256 `497dd4f7…c352`)を `fe-pe rebase` で 0x2000000 にした像(`4fd25502…efc5`)と、決定 0129 の種 `dsp-ippv2-2000000-seeds-idle1.txt`(16,133 行)で作った生成 C(`fe-lift lift … --seeds`、オプション無し。491,094,508 B、閉包 25,370 関数、5,706,010 命令)。

### 2.1 export(446)と FLEngine が引く名前(`tables/exports-requested.tsv`)

- export 446(すべて名前付き、転送なし)。FLEngine は `f_653f40` が **420 名**を UTF-16 のリテラルで `GetProcAddress`(423 回の呼び出し、`lift/out-ext/ipp_names.txt`)。引かない 26(`tables/unrequested-exports.txt`): `AggCreateInstance`、`ippSetCpuFeatures`、`ippsConvBiased_32f`、`ippsFIRGen*_64f` 4 つ、`ippsIIR*` 12 個、`ippsPhase_32f/64f`、`ippsTriangle_Direct_32f/32fc`、`ippsVectorRamp_32f/64f`、`ippsConvCyclic4x4_32f32fc`、`ippsConvCyclic8x8_32f`。
- 16 スナップで入口に入った export は 60(`tables/exports-requested.tsv` の class K)。
- 他の DLL: 同梱プラグイン 182 個(`lift/out-plug1/dll`)と第三者 VST3(`lift/out-vst1/bin` の OTT・Serum2・Vital・kHs 他 19 ファイル)と FLEngine を静的に走査した(`tables/importers.tsv`、`tools/prune/scan_importers.py`): **import 表に dsp_ippv2 を持つものは 0**。`dsp_ippv2` の文字列と 420 名のリテラルを持つのは **Fruity の 134 個(Effects 89、Generators 45)と FLEngine** で、135 個すべてで 420 名の集合が同一(名前の列の SHA-256 先頭 8 桁 `101c0683`)。VST3(OTT・Serum2・Vital・kHs)は dsp_ippv2 に触れない。→ **他の根(プラグインが引く export)の増分は、静的な名前では 0**(§6)。

### 2.2 CPU 振り分けの仕組み

- **選択子スタブ 433 個**(MSVC/IPP の `movsxd rax,[sel]; lea r10,[table]; jmp [r10+rax*8]`、3 命令。fe-decode が `switch` に持ち上げる = `Origin::Stub`)。選択変数は 4 つ: `0x4541000`(376 スタブ。RVA 0x2541000)、`0x455d050`(33)、`0x45410a0`(20。`ippiWarpAffineQuad` など)、`0x454f270`(4)。表のスロットは 3,091(すべて再配置された DIR64)。1 スタブは 7 case(添字 -1 = 遅延 init の呼び出し、0〜5)が 375、8 case(0〜6)が 57、10 case が 1。
- 選択変数の書き手: 主変数は `f_2001040`(遅延 init。DllMain の ATTACH と各スタブの添字 -1 から呼ばれる)と `f_2001000`(**= export `ippSetCpuFeatures`**。誰も内部から呼ばず、FLEngine も引かない)の 2 つだけ。どちらも `f_20010c0`(機能マスクから 0〜6 を作る)の戻り値を書く。マスクは `f_20013c0`(CPUID の呼び出し〔`f_37e0890`〕と AVX/AVX-512 の OS 対応の判定〔`f_37e011e`・`f_37e014e`、XGETBV〕を使う。262 命令。`f_2001320` が呼ぶ)が作り `0x4560ec0` にキャッシュする(`0x4560ec8` は初期化済みフラグ)。他の 3 変数は同じ形の関数(`f_20034d0`、`f_335fd10`、`f_35b45a0`)が 6/5/3/1/0/2/4/0 を書く。
- 環境: CPUID は `lift/runtime/fle_cpuid.c` に固定(決定 0118): 最大の基本葉 1、ベンダー 0、葉 1 の EAX/EBX/ECX = 0、EDX は FPU・CX8・CMOV・MMX・FXSR・SSE・SSE2 のみ。拡張葉は 0。**実測**(native -O0、16 スナップの終了時にゲストのメモリを読んだ。`FLE_COV_PEEK`): `0x4541000` = 0、`0x45410a0` = 0、`0x454f270` = -1、`0x455d050` = -1(この 2 つは該当するスタブが一度も呼ばれない)、マスク `0x4560ec0` = 7、初期化済みフラグ `0x4560ec8` = 1。すなわち**選択子は 0、マスクは MMX/SSE/SSE2 の 3 ビット**。
- case と命令集合(スタブの飛び先本体に含まれる最も新しい命令集合。`tables/isa-by-case.tsv`、`tools/prune/isa_by_index.py`。飛び先本体だけを数え、その先の呼び出しは数えない):

| case | 飛び先(distinct) | 命令集合が最大 SSE2 まで | SSE3 | SSSE3 | SSE4.1 | AVX | AVX-512(K 命令など) |
|---|---|---|---|---|---|---|---|
| -1(遅延 init の thunk) | 433 | 433 | | | | | |
| 0 | 427 | 416 | 11 | | | | |
| 1 | 427 | 419 | 6 | 2 | | | |
| 2 | 427 | 404 | 13 | 1 | 9 | | |
| 3 | 426 | 307 | 2 | 1 | | 113 | 3 |
| 4 | 426 | 302 | 2 | 1 | 4 | 110 | 7 |
| 5 | 426 | 275 | | | | 113 | 38 |
| 6 | 58 | 25 | | | | 33 | |
| 7・8 | 1・1 | 1・1 | | | | | |

  (case 0 は SSE2 までの実装、1・2 は SSE3/SSSE3/SSE4 の実装、3 は AVX、4 は AVX2/FMA、5 は AVX-512 の系統、という既存の読み〔`dsp-ippv2-lift-feasibility.md` §5.3〕と合う。飛び先の多くは共通のフォールバック〔`f_20018a0`〕または薄い関数なので、「SSE2 まで」の列が大きい。IPP の文書で確認した対応ではない。)

## 3. 到達可能性の判定

判定ツール `tools/prune/judge.py` が、**オプション無しの生成 C**(25,370 関数。関数ごとの本体・`f_<va>()` の呼び出し・`UINT64_C` の定数)と PE(export・DIR64 再配置・`.pdata`)と種ファイルから、関数ごとの区分を TSV に出す(`tables/functions.tsv`、`prune-S.tsv`、`prune-SM.tsv`、`summary.txt`)。再現: 生成 C を `fe-lift lift dsp… --seeds …` で作り(8 秒)、`judge.py --c … --dll … --seeds … --names lift/out-ext/ipp_names.txt --cov <16 スナップの cov.txt> --rejected-from <その fe-lift の標準出力> --out DIR`(約 1 分)。

### 3.1 区分と根拠の強さ

| 区分 | 定義(何が前提か) | 根拠の強さ | 前提が崩れたときの症状 |
|---|---|---|---|
| **S(強)** | 選択子スタブ 433 個が **case -1 と 0 だけ**を取るとき、どの根からも到達できない関数。根は最大限に保守的: entry、**446 個の export 全部**、seed のうち再配置が選択子表以外にあるもの、**選択子表以外の再配置スロットが指す関数ポインタ全部**。辺は直接 call/jmp(`f_X()`)、関数のアドレスに一致する定数(`lea`)、選択子スタブの case。 | **環境の定義から決まる**(CPUID 固定〔`fle_cpuid.c`、決定 0118〕+ 選択変数の書き手が `f_2001040` と `ippSetCpuFeatures` だけ〔誰も呼ばない〕)。実測: 選択子は 0(終了時の値)。データ表の読み方に依存しない。ただし関数アドレスの供給源は『DIR64 の再配置・コード中の定数・seed・export』と見ており、4 バイトの RVA の表など再配置の無い供給源は辺にしていない(IPP の関数アドレスが RVA 表から来る証拠は見つけていない。検定は 335 関数で反例 0)。 | 選択子が 1 以上になると、case k の飛び先が S の関数なら **stub が VA を出して abort**(`fle_pruned`)。S の関数でない(共有の)飛び先なら lift 済みのコードがそのまま走る(実 DLL と同じ関数)。黙って別の値を返す経路は無い。 |
| **M(中)** | S の前提に加えて、根を **entry と FLEngine が引く 420 export** に絞り、データ表を「**コードが参照するアドレスから、次に(どこかのコード・再配置から)参照されるアドレスの手前まで**」を 1 つの区間とみなして辿るモデルで到達できない関数。区間内の再配置スロットが指す関数は、参照した関数の辺。区間内の再配置がデータを指せば、その区間も辿る。ジャンプ表の case 本体(関数の先頭でない seed)は、それを含む `.pdata` の関数(チェーンは主 RUNTIME_FUNCTION まで辿る)にぶら下げる。 | **動的に 335 関数が入口に入り、その全部がモデルの到達集合に入る(反例 0)**。しかしモデルは仮定(イメージにオブジェクトの範囲が無い)。区間の取り方への感度は §3.4。 | モデルの辺が足りないとき、その関数は stub になり、**実行されれば VA を出して abort**。到達しない限り何も起きない。 |
| **W(弱)** | モデルでは到達しうるが、16 スナップで一度も入口に入らなかった関数。 | カバレッジだけ。 | **除去しない**(測るだけ)。 |
| **K** | 16 スナップで入口に入った(335 関数)。 | 実行された事実。 | 残す。 |
| **U** | 除去ではない。完全な閉包の lift が**拒否**した表ジャンプ 12 個(`jumptab_rejected` 1 + `msvc_rejected` 11)を、除去した閉包でも拒否のままにする(§4.3)。 | - | - |

### 3.2 結果(`tables/summary.txt`。dsp の閉包 25,370 関数)

| 区分 | 関数 | 生成 C(関数本体の合計) | 命令 | A の -Oz wasm 本体(size1 の表、概算) | AVX/AVX-512 命令を含む関数 |
|---|---|---|---|---|---|
| 全体 | 25,370 | 488,849,065 B | 5,906,476 | 174,636,728 B | 6,475 |
| **S** | 5,560(21.9%) | 183,618,208 B(37.6%) | 2,173,734 | 67,575,477 B(38.7%) | 1,749 |
| **M**(S の外) | 12,929 | 190,649,471 B | 2,351,341 | 50,782,094 B | 3,603 |
| S+M | 18,489(72.9%) | 374,267,679 B(76.6%) | 4,525,075 | 118,357,571 B(67.8%) | 5,352 |
| W | 6,546 | 111,139,796 B | 1,340,701 | 54,742,435 B | 1,123 |
| K | 335 | 3,441,590 B | 40,700 | 1,536,722 B | 0 |

(「A の -Oz wasm 本体」は size1 `prod-Oz-funcs.tsv.gz`〔決定 0155 の HEAD eca8c0b の -Oz、dead-flags などなし〕の関数ごとの本体バイトを `f_<va>` で引いた**概算**。D の wasm の実測は §5。)

### 3.3 層(case)ごとの大きさ(モデルでその case で初めて到達する関数。`summary.txt` の下半分)

| 初めて到達する case | 関数 | 生成 C | 命令 | AVX 系命令を含む関数 | A の -Oz 本体 |
|---|---|---|---|---|---|
| -1/0(=残る) | 6,881 | 114,581,386 B | 1,381,401 | 1,123 | 56,279,157 B |
| 1 | 3,777 | 74,839,594 B | 900,955 | 692 | 31,915,429 B |
| 2 | 2,220 | 61,639,706 B | 754,159 | 228 | 31,212,097 B |
| 3 | 1,732 | 50,918,285 B | 599,399 | 667 | 18,126,628 B |
| 4 | 4,049 | 98,349,049 B | 1,211,449 | 1,993 | 25,083,686 B |
| 5 | 1,138 | 36,487,133 B | 433,762 | 813 | 3,467,793 B |
| 6 | 347 | 21,044,747 B | 246,551 | 281 | 1,244,817 B |
| どの case でも到達しない | 5,226 | 30,989,165 B | 378,800 | 678 | 7,307,121 B |

(`tables/layers.tsv`。)

(層への帰属は直接辺とデータモデルによる**初到達**で、関数名のない IPP の「レイヤ」そのものではない。case 0 に残る 6,881 関数のうち 1,123 は AVX 系命令を含む関数(共有の薄い本体、CRT の分岐など)で、16 スナップでは 1 つも入口に入らない。)

### 3.4 モデルの検定と感度

- **検定**: 16 スナップ(native -O0、D 構成)の入口カバレッジ `tables/cov-D-16snapshots.txt.gz`(全体 12,298 関数。size1 の記録 12,298 と一致)の dsp 335 関数は、**全部が S と M の到達集合に入る**(反例 0。`tools/covcheck.py`)。dsp 内の呼び出し辺 401 のうち、呼び出し元の辺集合にモデルが持っていない辺が 24(`0x37de9b0`〔`_initterm` 風〕→ 初期化関数 13、`0x33e97c0`→`0x33d6e70`〔HLE の関数。ポインタが構造体経由〕など)。これらの被呼び出し側はすべて別の経路で到達集合に入っている。
- **感度**(区間の取り方を広げる。`--seg-extra-refs N` = 次の N 個の参照アドレスまで区間に含める、`--seg-back B` = B バイト手前から含める):

| モデル | M の外(除去)の関数 | 生成 C の除去量 | 検定(反例) |
|---|---|---|---|
| 既定 | 18,489 | 374.3 MB(76.6%) | 0 |
| N=1 | 16,664 | 338.7 MB(69.3%) | 0 |
| N=3 | 15,288 | 310.1 MB(63.4%) | 0 |
| B=512 | 17,132 | 326.3 MB(66.8%) | 0 |
| N=3 かつ B=4096 | 12,210 | 233.8 MB(47.8%) | 0 |

  モデルを広げても反例は出ない(出るはずのものではない: 広げるのは保守側)。**除去量はモデルの区間の取り方で 48〜77% の間で動く**。S だけなら 37.6% で動かない。
- **他の根を足したとき**(PM 依頼 1): 同梱プラグインと VST3 は dsp_ippv2 の export を FLEngine と同じ 420 名より多く引かない(§2.1)ので増分は 0。仮に **26 個の未使用 export を全部根に足した**場合は、M の外が 18,489 → 18,017(−472 関数、生成 C −3.9 MB)。`ippSetCpuFeatures` を根に足しても選択子の許容 case は変わらない(S の前提の側の話。§7)。

## 4. 実装(opt-in。既定は不変)

### 4.1 `fe-lift lift … --prune-list FILE`

- `FILE` は判定ツールが出す TSV(`tables/prune-S.tsv`、`prune-SM.tsv`)。1 行 = 1 関数(1 列目 VA または `rva:`+RVA、2 列目 = 区分 `S`/`M`、他の列は無視。`#` 行と `va` で始まる見出しは読み飛ばす)。**区分 `U` の行は関数ではなく、拒否のままにする表ジャンプの VA**(§4.3)。
- 意味(`lift/crates/fe-lift/src/prune.rs`、`fe-decode` の `analyze_with_prune`): リストの関数は (1) 種・export・`--seeds` から外す(entry を載せるとエラー)、(2) **残ったコードが直接 call/jmp・解決済みの間接呼び・選択子スタブの case で名指すときだけ**関数の入口として閉包に入る。(3) 入口に入った場合も**本体は復号せず、先も辿らない**。(4) 生成 C は stub `void f_<va>(void) { fle_pruned(UINT64_C(0x<va>)); }` だけ(`lift/runtime/fle_prune.h`: VA を出して `abort()`。`fle_prune.h` は `--prune-list` を渡したときだけ include される)。名指されない関数は何も出ず、ディスパッチ表にも入らない(間接呼びが来れば既存の `fle_dispatch_miss` が VA を出して止まる)。リストの関数だけを経由してのみ届く関数は、閉包に入らないので自然に消える。
- `--dead-flags` と `--outline-shared` と併用できる(`shared_heads` と `flaglive::analyse` に、stub を辿らない `skip` の引数を足した。`--swap-entries` の「併用不可」は変えていない)。HLE 入口・フック点がリストにあればエラー。
- **既定の生成 C は不変**: `--prune-list` なしで `include "fle_prune.h"` も出ず、`analyze_with_prune` の追加引数は空集合。実測: dsp(`--seeds` のみ)の生成 C が **491,094,508 B、SHA-256 `2c880576…`(HEAD の fe-lift と `cmp` 一致)**。FLEngine は §5。

### 4.2 判定ツール `tools/prune/judge.py`

§3 のとおり。入力は fe-lift が出した C だけ(新しいデコーダを書いていない)。TSV に関数ごとの区分・層・サイズを出す。リストの再現性: 同じ C・同じ種・同じ名前表・同じカバレッジなら同じ TSV(辞書順の出力。乱数なし)。

### 4.3 区分 U: 除去で「ディスパッチの数」を変えないための仕組み(**開発中に見つけた欠陥の直し**)

最初の版は、除去した関数を stub にしたあと**表ジャンプの受理/棄却の判定が変わってしまった**。fe-decode は閉包の後で表ごとに「飛び先が復号済みの命令の先頭であること」などを検査し、通らない表は**未解決のまま**(`fle_dispatch` ジャンプ)にする。除去した関数の本体は復号しないので、(a) 選択子スタブの飛び先(stub)が「復号済みの命令でない」として**表ごと棄却され**(受理された表が完全な閉包の 434 から **3** に減った。`logs/lift-dsp-PSM.txt` の最初の版)、(b) 逆に、完全な閉包では他の関数が重なって棄却されていた表(`0x2c227de`)が、除いたあとは通った。`fle_dispatch` は 1 回ごとに擬似プリエンプションの進捗カウンタを進める(決定 0147)ので、**native -O0 の 16 スナップで仮想時計が +19 ms ずれ、面 SHA が D と一致しなかった**(`logs/g3-lines-PSM-bad1-native.txt`、実測。クロック 3601795 対 3601776、面 SHA `8351da8f…` 対 `89563415…`、HLE の呼び出し 121 対 120、進捗 19,199,049 対 18,284,440)。直し: (1) 表の検査で、stub にした飛び先は「復号済みの命令の先頭」の検査から外す(`jumptab::verify_except`)。(2) 完全な閉包の lift が拒否した表ジャンプ 12 個(`jumptab_rejected` 1 + `msvc_rejected` 11)を、判定ツールが `--rejected-from`(プレーンな lift の標準出力)から **区分 U の行**にして TSV に足し、`fe-lift` はそのジャンプを常に未解決にする(`keep_unresolved`)。直したあと、除去ありと D の生成 C(dsp、同じオプション)を関数ごとに比べた(`tools/prune/cmp_bodies.py`): **残した関数の本体は、尾呼び → ブロックへの置き換え 7 件を除いて同一、`fle_dispatch`/`switch` の個数の違いは 0**。

**確認(PS と PSM)**: D のオプション(`--dead-flags --outline-shared --hle-entries`)の dsp の生成 C と、リスト付きの生成 C を比べた(`tools/prune/cmp_bodies2.py`、`cmp_kinds.py`、`logs/cmp-*.txt`)。PSM: 両方にある 8,952 関数のうち stub 1,998 を除き、本体が違う関数は 7(尾呼び → ブロック)、`fle_dispatch`/`switch` の個数の違いは 0。PS: 本体が違う関数 167、そのうち `fle_dispatch` の個数が違う関数が 28 あったが、これは関数の境界の移動(`--outline-shared` で、除いた関数に挟まれていた共有領域が別の関数に入る)で、**命令単位で比べると**間接遷移(`fle_dispatch` か `switch`)の文 1,172 件が D の同じ命令の文とすべて同じ種類で、違いは 0(PSM は 577 件で 0)。native と node の 16 スナップの一致と整合する。

### 4.4 その他の変更

- `lift/harness/wasm/trace_native.c`(native のトレース。**opt-in**): `FLE_COV_FILE=<path>` で関数の入口回数(`F va 回数`)と(呼び出し元, 呼び出し先)の辺(`E 元 先 回数`)を 4M 入口ごとと終了時に書く。`FLE_COV_PEEK=hex,hex,…` で終了時にゲストの 32 ビット値(`V addr 値`)も書く。未設定なら何も記録しない。
- `app/scripts/build-engine-bundle.sh`: opt-in の環境変数 `PRUNE_LIST=<絶対パス>`(dsp_ippv2 のモジュールだけ `--prune-list` を付ける包み。`SIZE_OPTS=1` の有無を問わない)。バンドルの説明に ` prune-list=<ファイル名>` が付く。無指定なら説明も手順も従来と同じ。
- テスト: `lift/crates/fe-lift/tests/prune_list.rs`(合成 PE。既定の出力一致、stub と先の消滅、名指されない関数は出ない、`--outline-shared`/`--dead-flags` との併用、エラー、生成 C のコンパイル)と `prune.rs` の単体テスト。

## 5. 測定(この機械: 16 コア、Linux VM、SwiftShader。他担当の負荷あり。**依頼者の PC の値ではない**)

### 5.1 大きさ

| | dsp の生成 C(D のオプション〔dead-flags+outline〕+ HLE) | 全体の wasm(emcc -Oz のリンク直後) | wasm-opt -Oz 後(配布する物) | ネイティブ 6 モジュール 70,572,928 B に対する倍率 | brotli q11(バンドルの .br) |
|---|---|---|---|---|---|
| **D**(決定 0161、記録) | 481,264,353 B(27,046 関数) | 292,196,358 B | **233,857,725 B** | **3.314** | 16,570,010 B(ネイティブの brotli 合計 14,494,211 B の 1.143 倍) |
| **D + 除去 S** | 299,964,424 B(23,433 関数。stub 2,080) | 235,176,410 B | 189,958,847 B(−18.8%) | 2.692 | 14,028,690 B(−15.3%。node zlib の q11・lgwin24 で測定。同じ方法で PSM を測ると、バンドルの .br の 11,995,854 B と一致する) |
| **D + 除去 S+M** | **113,981,461 B(8,952 関数。stub 1,998)** | **194,663,896 B** | **155,822,874 B(−78.0 MB、−33.4%)** | **2.208** | **11,995,854 B(−27.6%。ネイティブの brotli の 0.83 倍)** |

- 生成 C(オプション無しの dsp、`--seeds` のみ): 基準 491,094,508 B → S 307,092,435 B(−37.5%)→ S+M 115,357,076 B(−76.5%)。D のオプション付きでは S+M が 113,981,461 B。モデルの区間を広げた版(`--seg-extra-refs 3 --seg-back 4096`、`tables/judge-wide/`): 252,675,951 B(−47.5%)。
- **D の作り直し(再現の確認)**: 記録の D(決定 0161)のバンドルをそのまま「D」として比べたので、同じ手順で D を一から作り直した(`DR`: a4-multi の gen → 切り直し → dsp 49 単位 + common.c を含む全単位 → node のリンク → wasm-opt)。結果: リンク直後 292,196,358 B(記録 292,196,358 B)、wasm-opt 後 233,857,725 B(記録 233,857,725 B)。**記録の D の wasm とバイト同一(sha256 `80a5c36f6cbb6d0aa69667b1526b04bf18c13b1f0179dc74974a83e88939213b`、`cmp` 一致)**。したがって『D』と S・S+M の違いは、dsp の除去リストと、除去後の dsp の always_inline の単位の対応づけ(S-prune-ai-units)だけである。 非 dsp の .o 17 個は b1 の C の .o と大きさが全部一致(`tables/object-identity.tsv`)。
- **dsp の .o(-Oz、リンク前)の合計**: D 164,191,381 B(m1 の 49 単位。`DR`)、S 100,982,492 B(30 単位。D の −38.5%)、S+M 56,160,542 B(12 単位。D の −65.8%)。common.c の .o は D 5,286,397 B、S 5,122,555 B、S+M 4,511,132 B。
- dsp の S の分: wasm-opt 後 −43.9 MB(233.86 → 189.96)、M の分: さらに −34.1 MB(→ 155.82)。S の C は −37.5%(491.1 → 307.1 MB、オプション無し)で wasm(エンジン全体)は −18.8%、M の C は −39.0 ポイント(dsp の C 比)で wasm は −14.6 ポイント(エンジン全体の wasm 比): M の関数は C のバイトあたりの wasm が小さい(AVX-512 などの未対応命令は `fle_unsupported` の呼び出し 1 つにしかならない)。
- **S1(ネイティブの 2 倍以内 = 141.1 MB)は S+M だけでは未達**: 155.8 MB(2.208 倍)、差 14.7 MB。目標 1.2 倍にも遠い。残りの大きい部分: dsp の W(モデルで届きうる 6,546 関数、C 111 MB)・FLEngine・Blend2D(決定 0160 の置換で −約 40 MB の見込み、別担当)。
- **配布物の dll パック**(70,573,808 B → br 14,407,894 B)は**変わらない**(dsp の元の 39.9 MB の PE イメージをそのまま含む。dsp の `.text`/IPPCODE の機械語 32 MB はゲストのデータとして置いてあるだけで、その大部分は今の除去でも不要になるはずだが、IPP は定数を `.text`/IPPCODE に置くので〔`movaps xmm,[rip+…]` が読む〕、**黙って削ると壊れる**。測っていない)。

### 5.2 16 スナップの一致(再掲 §0 と内訳)

| 走行 | 行 | 結果 |
|---|---|---|
| D の native -O0(`logs/g3-lines-D-native.txt`) | g3 71 行 | b1 の A の native と全行同一(クロックと面 SHA)。 |
| PSM の native -O0(`logs/g3-lines-PSM-native.txt`) | 71 行 | D と**全行同一**(`diff` 0 行)。擬似プリエンプション: スライス 1812、進捗 18,284,440(D と同じ)。HLE 120/120。 |
| PS の native -O0(`logs/g3-lines-PS-native.txt`) | 71 行 | D と**全行同一**(`diff` 0 行)。進捗 18,284,440。 |
| PSM の wasm(node、wasm-opt 済み) | 85 行(g3-snap 16 + g3-surface 34 + g3-wintable 16 + g3-progress 16 = 82) | b1 の D の node 走行と同一(末尾行の経過秒だけ違う)。 |
| PS の wasm(node、wasm-opt 済み) | 85 行(82) | D と同一(末尾行の経過秒だけ違う) |
| 修正前の版(`logs/g3-lines-PSM-bad1-native.txt`、欠陥 §4.3) | 71 行 | **不一致**: クロック +19 ms、面 SHA 違い、進捗 19,199,049。 |

### 5.3 止まる事象(「除去した関数が実行されて止まる」)

- **0 件**: PSM と PS の native 16 スナップ(起動 → ウィザード → 主窓 → 閉じる)、PSM の wasm 16 スナップ(node)、Chrome の最初の idle までの起動(§5.4)。ログに `pruned function`・`not a known function entry` の行なし。
- **選択変数の終了時の値**(native、ゲストのメモリを読む): `0x4541000` = 0、`0x45410a0` = 0、`0x454f270` = -1、`0x455d050` = -1(後ろ 2 つは該当スタブが呼ばれない)。マスク `0x4560ec0` = 7(MMX/SSE/SSE2)。
- **陰性対照**(止まる機構の確認): 16 スナップで実際に入口に入る dsp 関数 0x37d09b0(50 回呼ばれる)を PSM のリストに足して作り直した版を native で走らせると、**15,728 入口で `fle: pruned function 0x37d09b0 was executed (prune list, decision 0168; …)` で abort**(1 秒、`logs/mutant1-*`)。
- **「IPP エクスポート全呼び出し」は実施していない**(未確認): 420 個の export を引数を変えて呼ぶ道具が無い(引数に正しい構造体・バッファが要る)。16 スナップで入口に入った export は 60。残りの 360 は W で、M の外の関数ではない(除いていない)ので、呼ばれても止まらない。

### 5.4 Chrome と node(この機械、負荷あり)

**Chrome 156(隔離 headless、使い捨てプロファイル、キャッシュ無効、localhost、私の private な `vite preview` 127.0.0.1:4181、`tools/chrome-startup.mjs`)**。D(`fl-engine-b1-D`、決定 0161 のバンドルそのもの)と PSM(`fl-engine-prune1-PSM`)を**交互に 8 回ずつ**(16 走行。これが set 1。追加の set 2 は表の後)開き、ページを開いてから最初の idle までを測った(performance-targets §2 の P1 の読み)。**16 走行すべての load は 24〜33(他担当の負荷)で、load ≤ 8 の走行は 0 件**。したがって規則どおり**壁時計の速さは主張しない**(参考値として併記)。CPU 秒と、ページ自身の timeline の段階を主にする。

| | D | PSM(S+M) | 差 |
|---|---|---|---|
| 最初の idle までの壁時計(参考。load 24〜33) 中央値〔最小〜最大〕 | 14.05 s〔12.79〜26.95〕 | 13.09 s〔9.13〜29.56〕 | 範囲が重なる(主張しない) |
| Chrome 全プロセスの CPU 秒(最初の idle の時点)中央値〔最小〜最大〕 | **17.81 s**〔17.38〜19.25〕 | **16.85 s**〔16.35〜18.78〕 | **−0.96 s(−5.4%)**。ラウンドごとの対(D − PSM)は 8 回すべて正: 0.47、0.81、1.48、1.43、1.05、0.43、1.45、0.55 s(中央値 0.93 s) |
| ダウンロード+brotli 展開+SHA-256 の段(ページの `download_done`)の中央値 | 4.27 s | **3.19 s** | −1.1 s(−25%)。wasm が 233.9 → 155.8 MB(brotli 16.6 → 12.0 MB) |
| その後の compile の尾(`download_done` → `compile_done`) | 0.17 s | 0.10 s | 小さい(V8 は呼ばれた関数を遅延コンパイルする) |
| `main_start` → 最初の idle | 9.45 s | 9.66 s | 同じ(ノイズの範囲)。起動の CPU は『実行される関数』で決まり、除いた関数は実行されない |
| エンジン種別・STAND-IN の帯 | `emscripten-fruity-v1`・無し | `emscripten-fruity-v1`・無し | 両方とも実エンジンが最初の idle に達した。除去した関数が実行されて止まった走行は 0 |

**読み**: 除去は**配布と展開の段**(約 1 s、−25%)と Chrome の CPU(約 −1 s、−5%)に効くが、**起動の本体(main() から idle まで、約 9.5 s の CPU 主体)は変わらない**。P1 のために dsp の除去を入れる意味はほぼ無い(P1 は決定 0161 の C → D の HLE と always_inline が効く)。決定 0161 の D の 7.63 s は load ≤ 8 の窓の値で、ここの 14 s は load 29 の値(同じ D でも別の条件)。load が下がったら測り直す価値がある(`tools/run-chrome.sh`)。生の値: `tables/chrome-D-PSM.tsv`。

**追加の組(set 2、同じ手順でさらに 6 ラウンド = 12 走行。上の表は set 1 の 8 ラウンド)**: set 2 も **12 走行すべて load 24〜44(load ≤ 8 は 0 件)**なので壁時計は主張しない。set 1 + set 2 の **14 対(28 走行)**: Chrome の CPU 秒の中央値 **D 17.88 s〔17.38〜24.24〕 → PSM 16.82 s〔16.18〜18.78〕(−1.06 s、−5.9%)**。ラウンドごとの対(D − PSM)は **14 対すべて正**(中央値 1.24 s、最小 0.43 s: 0.47、0.81、1.48、1.43、1.05、0.43、1.45、0.55、1.76、4.95、0.94、0.86、2.07、7.04 s)。ダウンロード+展開の段(`download_done`)の中央値 4.46 → 3.40 s(−24%)、compile の尾 0.17 → 0.10 s。壁時計(参考のみ): 最初の idle の中央値 D 15.54 s〔12.34〜77.89〕、PSM 13.44 s〔9.13〜29.56〕、対の差の中央値 2.02 s(正は 14 対中 11)で、範囲が大きく重なる。**set 2 の D の 2 走行は外れ値**: #2(最初の idle 74.05 s、`download_done` 45.95 s、load 25.8 → 35.7)は私の後始末のファイル削除(I/O)と重なった。#6(77.89 s、`download_done` 40.62 s、load 35.0 → 43.8)は私は何も走らせていない時刻で、原因は調べていない(他の担当の負荷と読む)。この 2 走行は対の差を 4.95 s と 7.04 s に押し上げているが、**除外していない**(除外した 12 対の中央値は 0.995 s、全て正)。main() から idle は set 1 では同じ(9.45 対 9.66 s)だったが、set 2 と合わせると D 10.77 s、PSM 9.83 s で、load と外れ値に左右される(主張しない)。生の値: `tables/chrome-D-PSM.tsv`(set 1)、`tables/chrome-D-PSM-2.tsv`、`tables/chrome-D-PSM-2-runs.jsonl.gz`。

**node(wasm-opt 済みの wasm、16 スナップ、`FLE_G3_PROGRESS=1`)の CPU 秒**(`tools/node-rounds.sh`、D・PS・PSM を交互に 3 ラウンド。load は 28 → 14 に下がる間。D は決定 0161 のバンドルの wasm に、今回のリンクで出た node 用グルーを組み合わせた。D の g3 行はグルーを替えても b1 の D と同一):

| | D | PS(S) | PSM(S+M) |
|---|---|---|---|
| 全体の CPU 秒(user+sys)3 回の中央値〔各回〕 | **21.28** 〔22.21、21.28、19.30〕 | **19.95** 〔22.26、19.95、19.40〕 | **19.94** 〔33.71、19.94、18.76〕 |
| 最初の idle(wiz0)までの主スレッド CPU 秒 中央値 | 6.12 | 5.39 | 5.86 |
| 最大 RSS | 1,957 MB | 1,913 MB | 1,877 MB |

PSM の 1 回目(33.71 s)は外れ値(load 28 の最初の走行。2・3 回目の 19.94・18.76 s と合わない)。最も load が低かった 3 回目(load 14〜17)では D 19.30 / PS 19.40 / PSM 18.76 s で、差は 3% 以内。**起動の CPU は除去でほとんど変わらない**(Chrome の対の差と同じ向き)。生の値: `tables/node-rounds.tsv`。

**`WebAssembly.compile` の時間(node、`tools/node-compile.mjs`、3 回ずつ交互、load 約 31)**: node の V8 でバイナリを `WebAssembly.compile`(検証 + ベースライン用の準備。関数の実コンパイルは呼ばれたときの遅延)したときの壁時計とプロセスの CPU 秒:

| | wasm(wasm-opt 後) | 壁時計 ms〔3 回〕 | CPU 秒〔3 回〕 | RSS |
|---|---|---|---|---|
| D | 233,857,725 B | 306〔306、316、306〕 | 0.65〔0.66、0.65、0.65〕 | 505 MB |
| PS(S) | 189,958,847 B | 285〔285、289、221〕 | 0.53〔0.53、0.52、0.53〕 | 421 MB |
| PSM(S+M) | 155,822,874 B | 216〔218、216、209〕 | **0.45**〔0.46、0.44、0.45〕 | 354 MB |

compile の CPU は wasm の大きさにほぼ比例して減る(PSM は D の −31%)。ただし絶対値が小さい(0.65 → 0.45 s)ので、起動全体(CPU 約 17 s)への効きは Chrome の対の差(約 −1 s)の一部。生の値: `tables/node-compile.jsonl`。

### 5.5 テスト

- **全体テスト**(`bash lift/harness/test.sh full --jobs 1 --threads 2 --no-record`、`FLE_TEST_NOCACHE=1`、cargo の target は private の `lift/out-prune1/target-test`、`FLE_INPUT_ROOT` = 本体のチェックアウト): 149 実行ファイル + doc-test → **results=153 passed=2128 failed=0**(`logs/test-full.txt`)。基準の 152 results・2,118 passed・0 failed に対し、増えたのは新しいテストの実行ファイル `fe-lift/prune_list`(合成 PE、6 テスト)と `prune.rs` の単体テスト 4 だけ(+1 result・+10 passed = 153・2,128)。**この木には `out-*` の入力が無かったので** SKIPPED-lines=15 だった(skip は pass に数えられる)。`test-env.sh --link` で入力をつないで、skip した 11 実行ファイルを再実行: **results=11 passed=204 failed=0 SKIPPED-lines=1**(`fe-lift/wall_fr9t` の 1 つ。基準と同じ。`logs/test-rerun-linked.txt`)。所要: ビルド 178 s、実行 841 s(1 並列)。
- **FRUITY 無効の a4**(942/942、895/895、functions 5,310、entries 1,009,731): **942/942、895/895、functions 5,310、entries 1,009,731 で基準と全部一致**(`a4-fruity.sh off`、パッチ済みの fe-lift とパッチ済みの木。gen は同じ OUT の再利用、build 66 s。`logs/a4off.txt`)
- **生成 C の既定の同一性**(上記テストとは別に直接確認): dsp(`--seeds` のみ)491,094,508 B・SHA-256 `2c88057609b1574d…`(最初の素のビルドと `cmp` 一致。最終の fe-lift で再確認)。FLEngine(`fe-lift lift FLEngine_x64.dll out.c --pdata`)215,811,678 B・SHA-256 `cd9107cef4cbef14f154645749957b7067be7ed7c2b93f5bbe1f72fd47029e50`(PM の基準と一致。`keep-logs/default-identity.txt`)。

## 6. 他の根(プラグイン・VST3)と、リストの前提(PM 依頼 1)

**リスト `prune-S.tsv` と `prune-SM.tsv` は、『FLEngine 単独の起動 + 16 スナップ』の根で作った物**。M の根は entry と FLEngine が引く 420 export(`tables/exports-requested.tsv`)。S の根は 446 export 全部。

| 調べたこと | 結果(実測。`tables/importers.tsv`、`tools/prune/scan_importers.py`) |
|---|---|
| 同梱プラグイン DLL 182 個 + 第三者 VST3 19 ファイル + FLEngine を、(a) import 表(遅延 import を含む)に ipp/dsp を名に持つモジュール、(b) 文字列 `dsp_ippv2`(ASCII/UTF-16)、(c) 446 の export 名(ASCII/UTF-16 の語単位)で走査 | (a) **0 個**。(b)(c) は **Fruity の 134 個(Effects 89、Generators 45)と FLEngine** だけ。135 個すべてで見つかる名前は **420 個の同一集合**(名前の列の SHA-256 先頭 8 桁 `101c0683`)。ASCII の名前は 0。VST3(OTT・Serum2・Vital・kHs)・残りの 48 個の DLL は触れない。 |
| 他の根の増分(静的な名前) | **0**(プラグインが引く名前 = FLEngine が引く 420 名)。 |
| 仮に 26 個の未使用 export を全部根に足したとき(上限の見積もり。`judge.py --extra-exports tables/unrequested-exports.txt`) | M の外が 18,489 → 18,017 関数(−472)、生成 C の除去量 −3.9 MB(374.3 → 370.4 MB)。S は変わらない(446 全部が根)。 |
| プラグインが実行時に組み立てる名前・序数での `GetProcAddress` | 静的には見えない。**未確認**。 |

プラグイン(G9/G11)を動かすと、同じ 420 名の export が**別の引数・別の経路**で呼ばれ、W の関数(360 export と、そこから届く関数)が入口に入る。それは M の外ではないので止まらないが、**M のモデルの検定は 16 スナップの 335 関数だけ**なので、プラグインを動かす段階で `judge.py --cov` を回して再検定すること(§8 R2)。

## 7. CPU 環境の定義への依存と、前提が崩れたときの振る舞い(PM 依頼 2)

S の判定は『**CPUID を SSE2 のみに固定する**』環境の定義(決定 0118、`lift/runtime/fle_cpuid.c`)と、『**ゲストが ippSetCpuFeatures を呼ばない**』に依存する。実機の FL は AVX の CPU では選択子 6(Wine で観測。決定 0162)を選ぶが、lift は 0 を選ぶ。この違いは決定 0118 の帰結で、除去によって変わらない(除去は選択子 0 の経路を一切変えない: 16 スナップの面 SHA が D と一致、§5)。

| 前提の崩れ方(将来の変更) | 何が起きるか | 検出(ガード) |
|---|---|---|
| CPUID の葉 1 の ECX/EDX に AVX や SSE4 を足す(`fle_cpuid.c` の変更) | `f_20013c0` のマスクが変わり、選択子が 1 以上になりうる。lift は VEX 命令を持ち上げられない(`fle_unsupported`)ので、そもそも AVX 以降の経路は今も止まる。 | **(1) 実行時**: 選択子スタブの case k の飛び先が S/M の関数なら、その stub が `fle_pruned: pruned function 0x… was executed (… check the CPUID answer of fle_cpuid.c and calls of ippSetCpuFeatures)` で abort(VA つき)。飛び先が S/M でない関数(case 0 と共有の関数)なら、lift 済みのコードがそのまま走る(実 DLL が選ぶ関数と同じ)。どちらも**黙って別の値を返す経路ではない**。**(2) ビルド時**: `tools/prune/check_env.py` が `fle_cpuid.c` の葉 1(ECX = 0、EDX = 7 ビット)と拡張葉(0)を静的に調べ、変わっていれば exit 1。ビルドの手順(`PRUNE_LIST`)に組み込む場合は、この検査を先に走らせる。 |
| ゲストが `ippSetCpuFeatures`(`f_2001000`)を呼ぶ | `f_20010c0` が引数のマスクから選択子を作って書く(ハードウェアの CPUID と突き合わせない)。 | S のみのリストでは `f_2001000` は残る(根が export 446 個全部)ので、書き換え後の最初の選択子スタブの飛び先が stub なら abort。S+M のリストでは `f_2001000` 自体が M(引かれない export)で除かれ、**呼んだ時点で stub が abort**。 |
| 別の DLL(プラグイン)が `ippSetCpuFeatures` などの未使用 export を引く | 同上 | 同上。静的には、同梱プラグイン 134 個の名前の列は FLEngine と同一(§2.1)で、26 個の未使用 export は誰も引かない。実行時に組み立てる名前は静的には見えない(未確認)。 |
| 選択変数 `0x4541000` などの値が -1/0 以外になった | 上のどれかが起きた結果 | 専用の実行時検査は**入れていない**。理由: (a) 選択子 k >= 1 は必ず「S/M の stub(abort)」か「case 0 とも共有される lift 済みの関数」に行き、前者は大きな音で止まり後者は実 DLL と同じコードなので、検査を足しても新しい検出は無い。(b) 検査をスタブの `jmp` に足すと 433 か所の生成物を変える(既定の動作を変えない方針に反する)。(c) 値は `FLE_COV_PEEK`(native の cov 実行)で読める。**入れる場合の最小の形**: dsp の `f_2001040`/`f_20010c0` の戻りに HLE 風のガード(`fle_hle` の仕組み)で選択子の範囲を検査する。 |

`fle_dispatch_miss`(間接呼びが除いた関数のアドレスに来た場合)も VA を出して abort する。

## 8. リスクの表と、PM が決める分岐

| # | リスク | 症状 | 可能性・根拠 | 検出 | S だけで足りるか |
|---|---|---|---|---|---|
| R1 | 除いた関数が実際は実行される(判定の誤り) | **止まる**: `fle: pruned function 0x… was executed`(直接に名指された関数)または `fle: indirect call to 0x… is not a known function entry`(間接)。黙って別の値を返す経路は無い | S: 環境の定義(選択子 -1/0)に依存、データ表に依存しない。M: データモデルの仮定に依存(§3.4: 反例は 16 スナップの 335 関数で 0。区間の取り方で除去量が 48〜77% 動く) | 実行すれば abort(VA つき)。静的には `judge.py --cov` が、新しい作業(別の FLP、別の機能)で入口に入った関数が M の外なら **MODEL FALSIFIED** を出す(§3) | S は足りる(反例の出方がない)。M は仮定つき |
| R2 | 将来のプロジェクト(依頼者の別の FLP、プラグイン G9/G11、再生 P3)で、16 スナップで入口に入らなかった関数(W)を使う | W は除いていないので影響なし。M の外の関数が使われるときだけ abort | 16 スナップは起動とウィザードだけ(dsp の入口 335 関数 = 閉包の 1.3%)。再生・書き出しで使う ipps は W に残っている(360 の export が W)。 | カバレッジを取り直す: 新しい作業を D 構成の native + `FLE_COV_FILE` で走らせ、`judge.py --cov` に渡す(反例が出ればリストを直す) | 判定が M の仮定に依る分だけ M は『作業を増やして検定する』必要がある |
| R3 | CPU 環境の定義が変わる(CPUID に AVX、`ippSetCpuFeatures`) | §7: 止まる | - | §7 の表(stub の abort、`check_env.py`) | S の前提そのもの |
| R4 | 他の DLL(プラグイン、サードパーティ)が未使用 export を引く | M(26 個の未使用 export)で外した関数に来れば abort。M の外が 18,489 → 18,017(−472 関数、C 3.9 MB)に減る | 静的には、同梱プラグイン 134 個は FLEngine と同じ 420 名のみ(§2.1)。VST3 は dsp_ippv2 に触れない。実行時に組み立てる名前は未確認。 | `scan_importers.py` を新しい DLL に回す。実行時は abort | S は影響なし(446 export を全部根にしている) |
| R5 | 表ジャンプの受理/棄却が変わり、ゲストの擬似プリエンプション(決定 0147)がずれる | 面 SHA・仮想時計が変わる(**実際に最初の版で起きた**: §4.3) | 直した(区分 U)。直したあとの確認: 残した関数の本体は D と同一(尾呼び → ブロック 7 件を除く)で `fle_dispatch`/`switch` の個数の違いは 0、16 スナップは D と一致(§5)。 | `tools/prune/cmp_bodies.py` | 両方に必要 |
| R6 | 生成物が大きくなる別の部分(Blend2D など)の置換(決定 0160)と組み合わせたときの相互作用 | 未測定 | dsp だけを触るので独立のはずだが、起動の時間は別 | - | - |
| R7 | wasm のコンパイル時間・メモリが悪化する | 悪化なし(測定済み): Chrome の compile の尾 0.17 → 0.10 s、node の最大 RSS 1,957 → 1,877 MB、wasm-opt の所要・メモリは D より小さい | - | §5.4 | - |

**PM が決めること**(2 通りの結果を §5・§9 に出した): (a) S だけを既定にするか(−37.6%。前提は環境の定義だけ)、(b) S+M にするか(−76.6%、データモデルの仮定つき。保険は abort と `judge.py --cov` の再検定)、(c) W(モデルで到達しうるが 16 スナップで入口に入らない 6,546 関数、C 111 MB)は**除かない**ことを前提にしているが、さらに削る場合はプロジェクトごとの FLP の再生まで含めたカバレッジを先に増やすこと。

## 9. PM が決めること(分岐。2 通りの結果を出した)

| 選択 | dsp の生成 C(D のオプション) | エンジン全体の wasm(wasm-opt 後) | 根拠・前提 | 保険 |
|---|---|---|---|---|
| **A: 除去しない**(既定のまま) | 481.3 MB | 233.9 MB(3.31 倍) | - | - |
| **B: S だけ** | 300.0 MB | 190.0 MB(2.692 倍) | 環境の定義(CPUID 固定、`ippSetCpuFeatures` 不呼び出し)だけ。モデルに依らない。 | stub の abort。`check_env.py`。 |
| **C: S+M** | 114.0 MB | 155.8 MB(2.21 倍) | B に、データ表のモデル(§3)と『根 = FLEngine が引く 420 export』を足す。16 スナップの 335 関数で反例 0。区間の取り方で除去量が 48〜77% 動く(§3.4)。 | 同上 + 新しい作業ごとに `judge.py --cov` で再検定(§8 R1・R2)。 |
| (D: M の保守版 = `--seg-extra-refs 3 --seg-back 4096`) | 252.7 MB(測ったのは生成 C だけ。wasm は未ビルド) | - | モデルを広げても反例 0 | 同上 |
| (E: W も除く) | (勧めない) | - | カバレッジだけが根拠。16 スナップは起動とウィザードだけ | - |

**推奨(読み)**: 既定にするなら **B**(根拠が環境の定義だけで、外れたら止まる)。S1 の距離を詰めるのが目的なら **C** を『実エンジンの検定を回しながら使う』形で(opt-in のまま)。どちらも **S1(2 倍)には届かない**(C で 2.21 倍)ので、S1 に必要な残りの約 15 MB(wasm-opt 後)は別の手段が要る: dsp の W(C 111 MB)を作業のカバレッジを増やして削る、Blend2D の置換(決定 0160)、wasm-opt を超える後処理。**起動(P1)への効き**は §5.4。

**M を既定にしてはいけない場合**: 依頼者が G9/G11(プラグイン)・再生(P3)を近く動かすなら、先に `judge.py --cov` を、その作業の native -O0 + `FLE_COV_FILE` で回す。

**HEAD に取り込む手順**: `prune1.diff` を当てる(opt-in なので既定は不変)→ 取り込んだ後に作るバンドルは `PRUNE_LIST=` を付けた `build-engine-bundle.sh`。決定 0161 の D の代わりに既定の入口へ置くかどうかは PM の判断(この作業は既定の入口に何も置いていない)。

## 10. 出典のない選択(S- の番号)

| 番号 | 選択 | 理由 | 代替 | 撤回条件 |
|---|---|---|---|---|
| S-prune-roots | M の根を『entry + FLEngine が引く 420 export』にした | 他の DLL が引かない静的な証拠がある(§2.1) | 446 export 全部(S がそう) | 実行時に組み立てた名前を引く DLL が見つかったとき(`--extra-exports`) |
| S-prune-segment | データ表の区間 = 『コードが参照するアドレス〜次に参照されるアドレスの手前』 | イメージにオブジェクト範囲が無く、保守側で最小の仮定 | `--seg-extra-refs` / `--seg-back` で広げた感度(§3.4) | 16 スナップ以外の作業でモデルの外の関数が入口に入ったとき |
| S-prune-addrconst | 関数の入口に一致する `UINT64_C` 定数はその関数が持つアドレスの辺とする | `lea` で取った関数アドレスを拾う(保守側) | 呼び出しの辺だけ | - |
| S-prune-casebody | 関数の先頭でない seed(jump table の case 本体)は、それを含む `.pdata` 関数(チェーンを主 RUNTIME_FUNCTION まで辿る)にぶら下げる | seed ファイルの由来(決定 0129: ptrtab/jt2 の走査)に対応 | 全部を根にする(S 側がそう) | - |
| S-prune-W | W(モデルでは到達しうるが 16 スナップで入口に入らない)は除かない | カバレッジだけでは根拠が弱い(PM の指示) | カバレッジを増やして除く | - |
| S-prune-class-U | 完全な閉包で棄却された表ジャンプ 12 個を未解決のまま残す | 擬似プリエンプションのカウントを D と同じにするため(§4.3) | 除去後の閉包で検査を通す(カウントが変わる) | 決定 0147 の数え方が表ジャンプに依らなくなったとき |
| S-prune-ai-units | 除去後の dsp の always_inline の単位を、D の 3 単位(m1-35・m1-02・m1-34)の関数のうち 16 スナップで入口に入った 21 個を含む単位(m1-01、m1-06)にした | 関数名で対応づける決定 0161 の方式を、起動のプロファイルを取り直さずに適用 | 起動のプロファイルを取り直す | プロファイルで別の単位が熱いとき |
| S-prune-stubmsg | stub のメッセージに『CPUID の答えと ippSetCpuFeatures を確認』の文を付けた | 原因の推定(読み) | 無し | - |
| S-prune-chrome | Chrome の測定は b1 と同じ手順(使い捨てプロファイル、キャッシュ無効、localhost、D と交互、load の記録) | 並べて比べるため | - | - |

## 11. 未確認と規則違反の自己申告

**未確認**:
- 実機 Windows との照合(0 件)。lift の IPP は選択子 0(SSE2)、実機は AVX の CPU で別の層(決定 0162)。
- M のモデルの検定は 16 スナップ(起動・ウィザード・主窓の少し)の入口 335 関数だけ。プロジェクトを開く・再生・書き出し・プラグインの経路は走らせていない(G8 の道具はあるが、この作業では回していない)。
- 『IPP の export 全部を引数を変えて呼ぶ』カバレッジは実施していない(道具がない。§5.3)。
- 決定 0163(`fle_dispatch` の直接写像キャッシュ)を含む runtime での再測定。擬似プリエンプションの数え方を変えないはずだが、この作業は fdfa1dc の runtime で行った。
- `build-engine-bundle.sh` を `gen` 段階から通していない。`PRUNE_LIST` の包みは `tools/febin/fe-lift`(同じ文)で eng.sh から使った。`weblink`・`wasmopt`・`seed`・`bundle` は実際のスクリプトで走らせた(ただし `wasmopt` は node のリンクの wasm-opt 結果を、リンクのバイト同一を `cmp` で確認した上で再利用した)。
- 除去を `--dead-flags`・`--outline-shared` 無し(`SIZE_OPTS=0`)で使ったビルドは、生成 C の検査(`verify_c.py`)までで、コンパイルも実行もしていない(測った 3 構成はすべて D のオプション付き)。
- プラグインが実行時に組み立てる名前で dsp を引く可能性。
- 速度(壁時計)の主張は load が 8 以下の窓の分だけ(§5.4)。

**規則違反の自己申告**(気づいた順。時刻は `date`):
1. 存在しない `bash` ツールを 3 回呼んだ(10:05、10:30、12:28。いずれも結果に影響なし)。
2. 実行中のスクリプト `tools/eng.sh` を 3 回書き換えた(bash は実行中のスクリプトを少しずつ読むので、その段階の終了後に構文エラーの行が出た。結果への影響なし)。
3. `sleep` を含むコマンドを 3 回出して拒否された(1 回は `sleep 1`、2 回は末尾の `&` と `sleep 0`〔11:12 と 12:28〕)。実行されていない。
4. 前景で 2 分を超えるコマンドを 1 回(単位のビルドを `background` なしで起動し、タイムアウトで打ち切られた。残った子プロセスを手で止めて、その単位の .o を消して作り直した)。
5. `pkill -f` が自分のシェルを殺した(1 回)。その後の PID 指定の停止で整理。
6. 作業ディレクトリが一時 8 GB(上限 5 GiB)を超えた。中間物を消した。終了時は 500 MB 以内に整理する。
7. 重いプロセス ≤2 の PM の警告(10:24)の前は、最大 4 本を同時に走らせた時間がある。警告の後は 2 本以内(ただしテストの cargo build は CARGO_BUILD_JOBS=2 で、その間の本数は 2〜3)。
8. 報告 1 の時刻に自分の見積もり(10:34)を書いた(実際は約 10:29)。
9. 最初の版の除去に欠陥があった(§4.3)。依頼者に出す前に自分で見つけて直した。
10. 外部要因と中止の記録: 私の `wasm-opt`(PS)が約 11:22 に他の担当から SIGTERM を受けた(私のコマンドではない。再実行して使った)。a4 を JOBS=3 で誤って起動し数秒で止めた。Chrome の追加の組(set 2)の 1 回目の試行を自分で中止した(結果は使っていない)。set 2 の D #2 は私の後始末の I/O で 74 s かかり外れ値になった(中央値と対の差で扱い、除外はしていない)。

## 12. 再現手順とファイル

```
# 木: git archive fdfa1dc(または d9af868)+ prune1.diff   cd <木>
cargo build --release --offline -p fe-pe -p fe-lift            # fe-lift に --prune-list が入る
fe-pe rebase lift/out-ext/dsp_ippv2_x64.dll 0x2000000 dsp.2000000.dll
fe-lift lift dsp.2000000.dll dsp-base.c --seeds lift/harness/seeds/dsp-ippv2-2000000-seeds-idle1.txt > dsp-base.txt   # オプション無し = 従来の生成 C(491,094,508 B)
# 16 スナップのカバレッジ: D 構成の native -O0 を作り(a4-multi.sh の FRUITY 構成、dsp は --hle-entries --dead-flags --outline-shared)、
#   FLE_COV_FILE=cov.txt FLE_COV_PEEK=4541000,45410a0,454f270,455d050,4560ec0,4560ec8 ... FLE_G3_SCRIPT=cmp-short.g3 で走らせる
python3 tools/prune/judge.py --c dsp-base.c --dll dsp.2000000.dll --seeds <同じ種> --names lift/out-ext/ipp_names.txt \
        --cov cov.txt --rejected-from dsp-base.txt --wasm-oz docs/investigations/size1/tables/prod-Oz-funcs.tsv.gz --out judge
fe-lift lift dsp.2000000.dll dsp-pruned.c --seeds <同じ種> --dead-flags --outline-shared --hle-entries lift/harness/hooks/dsp-hle-entries.txt --prune-list judge/prune-SM.tsv
python3 tools/prune/cmp_bodies.py dsp-D.c dsp-pruned.c      # 残した関数の本体は D と同一(尾呼び → ブロックを除く)
python3 tools/prune/check_env.py lift/runtime/fle_cpuid.c    # 環境の前提
# バンドル: PRUNE_LIST=<abs>/judge/prune-SM.tsv SIZE_OPTS=1 LOWREGS=2 WASM_OPT=1 FLE_SEH_HANDLERS=0x112c0 bash app/scripts/build-engine-bundle.sh
```

`lift/out-git/prune1-results/`: `prune1.diff`、`MERGE-NOTES.md`、`docs/`(この文書、README の 1 行)、`tables/`(`prune-S.tsv`・`prune-SM.tsv`〔リスト〕、`functions.tsv.gz`〔関数ごとの区分〕、`summary.txt`、`sensitivity.tsv`、`exports-requested.tsv`、`importers.tsv`、`isa-by-case.tsv`、`sizes.tsv`、`object-identity.tsv`、Chrome・node の表、カバレッジ `cov-*.txt.gz`)、`logs/`、`tools/`(eng.sh ほか、判定ツールは diff に入っている)。

## 付録 PM の判断(2026-11-03、取り込み時)

- **取り込み時の PM の確認**: `prune1.diff` は HEAD 83c9dd8(決定 0166 の取り込み後)に `patch -p1 --dry-run` が rc=0・fuzz/offset/reject なしで当たる(担当の確認は d9af868 に対して)。適用後の FLEngine の生成 C の SHA-256 は `cd9107cef4cbef14f154645749957b7067be7ed7c2b93f5bbe1f72fd47029e50`(215,811,678 B)のままで、オプション無しでは生成 C が変わらない。全体テストは results=154・passed=2,129・failed=0(取り込み前の 153・2,119 に、`prune_list` 6 件と `prune.rs` 単体 4 件、結果 1 件が加わる)。a4 FRUITY 無効は init_count 942/942・run_rank 895/895・functions 5,310・entries 1,009,731 で基準どおり。
- **既定にするのは S(根拠=環境の定義だけ)**。S+M は『データ表のモデル』を仮定に含み、検定は 16 スナップの 335 関数だけなので、**条件つきの選択肢**として残す。S+M を既定にする条件: 参照プロジェクトの再生・書き出し・プラグインのシナリオを含む `judge.py --cov` の再検定で、モデルの外に入口が出ないこと。失敗の形は名前つきの abort(`fle: pruned function 0x… was executed`)で、静かな誤動作ではない(陰性対照で確認済み)。
- **W(弱: モデルでは届きうるが一度も入口に入らない 6,546 関数、C 111 MB)は除かない**(担当の前提を維持)。
- **S1 の算術**: S+M は 155.8 MB(2.208 倍)で、基準 141.1 MB(2 倍)に約 14.7 MB 足りない。決定 0167(b2d2)の Blend2D 置換(D 比 −18.5 MB)を重ねれば、足し算では約 137 MB(1.95 倍)になる見込みだが、**重ねた構成での再測定は未実施**。b2d2 の取り込み後に、D + S(または S+M) + E を一つのバンドルにして、大きさ・16 スナップ・Chrome の最初の idle を測る担当を出す。
- **P1 には効かない**(Chrome の CPU 秒 −5.9%、起動の壁時計は load 24〜44 で主張できない)。この作業は S1(配布・展開・メモリ)の手段として扱う。
- 決定 0163(`fle_dispatch` のキャッシュ)を含む runtime での再測定は未実施(担当は 0163 の無い runtime で測った)。最終バンドルの測定で行う。
