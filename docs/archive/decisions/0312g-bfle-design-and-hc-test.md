# 0312g: (1) B-FLE の設計の読み(plugin の wasm を FLEngine とどう共有するか。G9 の前提、決定 0309 §3 の『2 つ目で 2.03 倍』の解消)、(2) S3a の clamp の node テスト(mt3。PM 指示 22:46。仮。**build・test・Chrome は 0。(1) は現在の bundle B1 の link map・wasm の section・plug1 の表からの読み、(2) は書いただけで未実行**)

## 0. 冒頭の三点

1. **推薦(1 つ)**: **plugin は今の方式のまま『FLEngine と同じ 1 つの wasm に静的に結合(RM_MAX を拡げた 9 個目以降の実モジュール)』とし、共有は『B-FLE = 同一の Delphi RTL/VCL 関数を 1 つにして、データの参照だけをモジュールごとの表経由にする生成器の変更』で行う。** 別 module 化(table/import で共有)は B-FLE の**後**に、S1 か P1 が実際に効いたときだけ検討する。Emscripten の dynamic linking は採らない。理由: (a) **runtime(fle_*)は 1 つしか持てない**(状態の表が全部 runtime の中にあり、二重に持つと Windows が 2 つできる)ので、runtime の二重化は選択肢でない。(b) **wasm の増分の大半(Balance の 14.6 MB のうち約 10.1 MB)は『FLEngine と同一の lift 済み関数』の重複**で、runtime の二重化(約 1〜3 MB)とは桁が違う。どの packaging でも効くのは B-FLE だけで、**B-FLE の関数の ABI(モジュールのデータ基底を渡す形)は packaging に依らない**ので、先に B-FLE を決め、packaging は静的で始めて後から切り出せる。(c) 静的結合は runtime・dispatch 表・GIL・per-Worker の状態を新しく作らずに済む。
2. **S1 は『基準の取り方』で結論が変わる(PM の判断が要る)**: 固定の絶対値 154,885,888 B(= 8 モジュールのネイティブ × 2)なら、残りは **11.90 MB** しかなく、**B-FLE があっても Delphi の plugin は 3 つまで**(Balance + Filter + Delay 2 = +10.2 MB で 153.2 MB。4 つ目の PEQ2 +4.6 MB で超過)。plugin のネイティブも分母に入れる比(0309 の式)なら、**B-FLE の plugin の限界比は 0.88〜1.45 倍 < 2 なので、何個入れても全体比は下がる**(Balance + Filter + Delay 2 = 1.73 倍)が、B-FLE なしでは 2 つ目で 2.03 倍。
3. **見積りの前提に 1 つ重要な訂正の候補**: 0309 の係数 7.74(wasm B / コード B)は、**現在の bundle の FLEngine の pre-opt の実測 7.69**(link map の C 部 75.83 MB + hybrid の direct wasm 6.27 MB = 82.10 MB ÷ 関数バイト 10,677,239)と合う。しかし**最終の wasm は wasm-opt -Oz と merge の後で pre-opt の 0.765 倍**(code section 140.25 MB ÷ pre-opt 183.3 MB)。7.74 が pre-opt の係数なら、**Balance の『共有なし 14.6 MB』は最終 wasm では約 11.6 MB、B-FLE の 3.28 MB は約 2.6 MB の見込み**(読み。plugin の hybrid の割合は未知。実 wasm にした plugin は 0 件)。11.6 MB なら固定の絶対値でも Balance 1 つは共有なしで残り 11.90 MB に収まる(ぎりぎり)。

## 1. 現在の bundle(B1 = E13 + 積み重ね)の実測の事実

### 1.1 最終の wasm(`engine.d396496ad64b.wasm`、142,986,125 B)の section(実測。section header を読んだ)

| section | B | 備考 |
|---|---:|---|
| code | **140,254,002** | 106,214 関数 |
| data | 2,229,452 | 4 segment(bss は含まない) |
| elem | 307,297 | 関数表(間接呼び出しの対象) |
| export | 81,764 | |
| function / type / import / global / table / tag | 106,803 / 4,942 / 1,420 / 356 / 36 / 3 | |

### 1.2 link の直後(wasm-opt の前)の入力別の大きさ(`ci-b1/engine/hyb/link.map` を集計。実測)

| 由来 | code B | 備考 |
|---|---:|---|
| FLEngine の生成 C(`w-Oz/obj/fle/*.o`) | 75,829,251 | 43.9% |
| 他の 7 モジュールの生成 C(`obj/mod/*.o`、IPP が大半) | 93,784,806 | 54.3% |
| **runtime の C(`obj/rt/*.o` 90 個)** | **697,876** | 0.4% |
| runtime の GIL ラッパ(`fle_gil_wrap.o`) | 245,898 | |
| harness(`obj/h-fruity/*.o`) | 114,549 | |
| Blend2D のライブラリ + b2d glue + emscripten の libc | 1,305,079 + 13,779 + 75,963 | |
| 合計(link の code section) | 172,067,664 | |
| hybrid の direct wasm(`hyb/direct.wasm`、8 モジュール分。FLEngine 分は `n0-fle.wasm` 6,266,759) | 10,582,930 | wasm-opt 後に merge される |

- 最終の wasm 142,986,125 B ≈ C 部 172.07 MB → wasm-opt -Oz で 131.45 MB(`w-Oz/web/flengine6.c.opt.wasm`)+ direct wasm 10.58 MB + merge で増える約 0.95 MB。**wasm-opt -Oz は C 部を 0.764 倍に縮めている**(実測)。
- runtime の data は link map 上 4,692,703 B だが、ほとんどが bss(`fle_sys.o` の静的な表 2.63 MB ほか)で wasm の file には載らない(最終の data section は 2.23 MB で、Blend2D の定数表なども含む全体)。**runtime を二重に持つときの file の増分は code 約 0.94 MB(pre-opt。-Oz 後は約 0.72 MB)+ 初期化 data(0〜2.2 MB の内)**、メモリ(線形メモリ)は bss 約 4.7 MB。

### 1.3 係数の検証(FLEngine で。実測 + plug1 の表)

- FLEngine のネイティブ: file 22,227,016 B、関数の合計 10,677,239 B(63,500 関数、`docs/investigations/plugins-distribution/tables/dupscan-modules.tsv` idx 182)。
- 現在の bundle の FLEngine の pre-opt: C 部 75,829,251 + direct 6,266,759 = 82,096,010 B → **7.69 B/B**。plug1 の係数 7.74 と 0.6% の差。
- 最終(-Oz + merge)は pre-opt 全体(172.7 + 10.6 = 183.3 MB)の **0.765 倍**。

## 2. Balance の『14.6 MB(共有なし)』の内訳 — 『runtime C 部/lift した plugin 本体/共有できる lift 済み関数』

(plug1 の `vs-engine-per-dll.tsv` の Balance の行 = 『DLL の関数を FLEngine の関数と照合した』実測と、上の係数による**読み**。**Balance を wasm にした実測は 0 件**)

| 項目 | ネイティブ側の実測 | wasm の見積り(係数 7.74、pre-opt の基準) | 最終(-Oz 後。× 0.765。読み) |
|---|---:|---:|---:|
| Balance の関数の合計 | 1,665,794 B(13,038 関数) | 12.89 MB | 9.9 MB |
| ① lift した plugin 本体(FLEngine に**無い**関数) | **361,815 B**(21.7%) | **2.80 MB** | 2.1 MB |
| ② 共有できる lift 済み関数(FLEngine と**同一**) | **1,303,979 B**(78.3%) | **10.09 MB** | 7.7 MB |
| データ(非ゼロ)の合計 | 1,694,527 B | 1.69 MB | 1.69 MB |
| ③ データのうち FLEngine に無い物 / 同一の物 | 474,694 / 1,219,833 B | 0.47 / 1.22 MB | 同左 |
| **共有なし(① + ② + データ全部)** | | **14.59 MB** | **約 11.6 MB** |
| **B-FLE(① + ③ のみ)** | | **3.28 MB** | **約 2.6 MB** |
| runtime の C 部(この表の外) | **追加 0**(静的結合では既存の 1 つを使う)。別 module が自前の runtime を持つなら +0.94 MB(pre-opt) | | +0.72 MB |

- 読み: **共有なしの 14.6 MB の約 70%(10.1 MB)は『FLEngine と同じ関数の重複』で、runtime の C 部は 0**。つまり『runtime を二重に持つ形』の増分(約 0.7〜0.9 MB/module)は主題の大きさではない。
- Balance の import は Windows の DLL だけで(469 行あり + 74 行なし、0309 §1.2)、**IPP・Blend2D は呼ばない**(IPP の `dsp_ippv2` は PEQ2・Soundgoodizer・FLEX・Frequency Shifter が `ildsp` 経由〔未 lift〕)。FLEngine の『plugin 用 API』は guest 側の host オブジェクト(TFruityPlugHost の vtable)で、plugin からは **guest の間接呼び出し(`fle_dispatch` の VA 引き)**で呼ばれる。どの packaging でも新しい仕組みは要らない。

## 3. 『plugin が FLEngine/runtime を呼ぶ形』対『runtime を二重に持つ形』

| | plugin が既存の runtime を呼ぶ(静的結合では自動、別 module では import) | plugin module が runtime をもう 1 つ持つ |
|---|---|---|
| wasm の増分 | 0(静的)/ import 表 約 15 KB(Balance の 469 行) | + code 約 0.94 MB(pre-opt)+ 初期化 data(≤ 2.2 MB)/ module |
| メモリ | 0 | + bss 約 4.7 MB / module |
| **意味** | **Windows が 1 つ**。窓・ハンドル・heap・VFS・時計・GIL が共有され、plugin の窓が FL の窓と相互作用できる | **Windows が 2 つ**(状態の表が runtime の中: `fle_realmod.c:35` `static rm_t mods[RM_MAX]`、`fle_sys.c` の SRW・CS・時計、`fle_gil.c` の `fle_gil_word` ほか)。**GIL が 2 つになり排他が成り立たない。plugin の `CreateWindowEx` が FL の窓表に載らない**。成立しない |

→ **二重化は大きさの問題でなく正しさの問題で選択肢にならない**。

## 4. 共有の方式(3 候補)の比較

| 観点 | ① 静的結合(同じ wasm に plugin を追加) | ② 別 module 化(table/import で共有) | ③ dynamic linking(Emscripten MAIN_MODULE/SIDE_MODULE) |
|---|---|---|---|
| 仕組み | `a4-multi`/`build-engine-bundle.sh` の module 一覧を拡げる(`EXTRA_MODULES`・`NEXTRA`〔`build-engine-bundle.sh:48,113,430,442`〕、`RM_MAX 8`〔`fle_realmod.c:18`〕を拡げる)。9 個目以降 | plugin だけを別に link(`--allow-undefined` + 共有メモリ・共有表の import)。`link-web.sh:23〜24` は既に `--export-memory`・`--export=__tls_base`・`--allow-undefined`。`GLOBAL_BASE` と表の base を module ごとに分ける。`LoadLibraryW` から JSPI で fetch + instantiate し、dispatch 表(VA → 関数)に**実行時に登録** | `-sMAIN_MODULE`(主 module を PIC で再 build)、plugin は SIDE_MODULE、`dlopen` |
| S1(固定の絶対値 154,885,888 B) | B-FLE があっても Delphi 3 つまで(+10.2 MB)。共有なしなら Balance 1 つ(+14.6 MB だと超過、最終 -Oz 後の見込み 11.6 MB なら収まる) | 主 module は 143 MB のまま(1.85 倍)。plugin は別の対象。『bundle の大きさ』をどこで測るかの定義が変わる | **PIC 化で主 module の code が増える**(一般に +10〜25% 〔読み。未測定〕= 14〜36 MB で、plugin 0 個の時点で上限に迫る/超える) |
| S1(plugin のネイティブも分母に入れる比) | B-FLE で全体比は下がる(Balance + Filter + Delay 2 で 1.73 倍)。共有なしは 2 つ目で 2.03 倍 | 同じ(各 module が B-FLE の残りだけ持つ) | 同じ + PIC の分 |
| P1(起動 10 s 以内。first idle は今 5.1〜6.1 s) | **毎回の起動で全 plugin 分を取得・compile**。B-FLE の Balance で +0.03 s、共有なしで +0.14 s(9.4 ms/MB、現行の構成では未測定)、brotli +0.3 / +1.4 MB。3 つで +0.1 s。6 種で +約 0.27 s(B-FLE の 28.9 MB、0309 の表) | **起動に響かない**(使う plugin だけ、project を開くとき〔FLP を解析した時点〕に取得・compile)。ただし**音声 thread の中では待てない**ので先読みが要る | 同左(dlopen で遅延) |
| No-options | 旗・環境変数は要らない | 『どの plugin をいつ取るか』の規則が要る(旗ではない)。新しい loader の挙動 | build の旗(`-sMAIN_MODULE=2` 等)は利用者の選択でない |
| 実装の新しさ | **最小**: runtime・dispatch 表・GIL・Worker ごとの状態に新しい物は無い | **大**: ① dispatch 表の動的登録(今は生成 C の静的な配列。a4 の 5 モジュールの生成 C で 62,935 行)、② 全 Worker への instantiate の配布(real thread では各 Worker が plugin module を持つ必要。`WebAssembly.Module` を postMessage して各 Worker で instantiate)、③ module ごとの data base の割当と再配置(Delphi は全部 ImageBase 0x400000)、④ import する `fle_imp_*` の export(数百〜千)、⑤ 表の index の割当(間接呼び出しの型) | Emscripten の dynamic linking は **pthread + JSPI + 主 module 143 MB の組み合わせの実績が無い**(読み)。dlopen は各 Worker で実行される |
| CI | 主 module の link に plugin が加わる(+16%/plugin の C 〔0309 §3〕)。**1 つの plugin の変更で全体を再 link** | plugin を独立に build・cache できる | 同左 + 主 module が PIC |
| V8 の限界 | 全部入れると単一 module 1 GiB を超える(181 個で 2.6 GB。0309/plug1)。7 種なら問題なし | なし | なし |

## 5. B-FLE そのものの難所(どの packaging でも共通。**ここが本当の仕事**)

(`docs/investigations/plugins-distribution.md` §5.2 B/B-FLE の読み + 今回の確認)
- 『同一』の判定は、RIP 相対の displacement などを正規化したあとの一致(`dupscan`)。**lift した C は VA を絶対値で持つ**ので、共有するには関数が**モジュールのデータ基底を引数または thread-local の『現在のモジュール』から取る**形にする必要がある。**関数の 72% がデータに触れる**(純粋な計算は 28%)。
- Delphi の RTL は DLL ごとに**自分の状態**(メモリ manager、例外のフック、スレッド変数)を持つ(各 DLL が RTL を静的に持つ)。共有した関数が FLEngine のデータを触ると、plugin の RTL が FLEngine の heap を使ってしまい**忠実性が壊れる**。だから『表経由』は必須。
- 呼び先: 共有した関数が plugin 固有の関数を呼ぶ(callback、VMT)。`call_ind:dispatch` の VA 引きがそのまま使える。複数 thread(ミキサー thread が plugin、UI thread が FLEngine)で『現在のモジュール』が競合しないよう、**基底は引数で渡す**か thread-local にする。
- FLEngine の側も参照表経由に変える必要がある可能性(再 lift。**未確認**)。性能への影響(データ参照が 1 段間接になる)は未測定。
- **最小の確かめ**(plug1 の推奨 2、未実施): Flanger と Balance(コードの 98.7% が同一)の 2 つで、データ参照を表経由にして**生成 C がモジュール間で同じになるか**を fe-lift の出力で見る(wasm にする前に、C の一致だけ。約 1〜2 人日〔私の読み〕)。

## 6. 推薦の確認と、PM の判断が要る点

1. **推薦**: 上の 0 のとおり。具体的な第一歩 = (i) 5 の最小の確かめ(Flanger + Balance の生成 C の一致)、(ii) 一致するなら生成器に『モジュールのデータ基底』の ABI を入れる(packaging 中立)、(iii) G9-a/b は 1 つだけなので B-FLE なしの静的結合(+約 11.6〜14.6 MB)で進める(0309 の結論のとおり)。
2. **PM の判断**: S1 を『固定の絶対値 154,885,888 B』とするか『plugin のネイティブも分母に入れる比』とするか。前者なら静的結合は Delphi 3 つまでで、4 つ目以降は別 module 化(②)が必須になり、その場合は ②の loader の設計(§4 の ①〜⑤)を B-FLE と並行して始める必要がある。
3. **未確認**: (a) 7.74 が pre-opt の係数か(最終の plugin の wasm は 0.765 倍になるか)、(b) plugin の hybrid(direct wasm)の割合、(c) Balance を開く経路で実行される import の数、(d) ②の loader の工数(見積もっていない)、(e) ③の PIC の増分(読み。未測定)。

## 7. (2) S3a の clamp の node テスト(書いた。**未実行**)

- 置き場: `lift/harness/sysinfo_wasm/{run.sh,hc_test.c,hc_test.mjs}`(`impl/sysinfo_wasm/` に写し)。`tp_wasm` と同じ形: `run.sh` が `fle_sys.c` の**実際の文**(`EM_JS fle_js_hardware_concurrency` から `fle_sys_processors` の終わりまで)を awk で切り出して `hc_extract.inc` にし、小さな C から emcc で wasm にして node で動かす(runtime の他の部分は要らない)。切り出しは S3a・S3b の両方で確認済み(awk のみ、emcc は未実行)。
- 検査(`hc_test.mjs`): `hardwareConcurrency` が **undefined・null・0・負(-1、-3)・NaN・Infinity・0.5 → 1**、**1・1.9 → 1**、**2・2.9 → 2**、4、**文字列 "8" → 8**、16、31、**32 → 32、33・33.9・64・1e9 → 32**、**2^31 → 1**(`|0` が負に回る。実在しない値)、**`navigator` が無い環境 → 1**。**1 回だけ読む**(最初の呼び出しの値が後の navigator の変更・消失に関わらず保たれる)、instance ごとに独立。S3a のときは `fle_cfg_processors`(FLE_PROCESSORS)の優先(4 が 8 に勝つ、16 が 2 に勝つ、64 → 32、0・負 = なし)も検査。
- `run.sh` はさらに、`fle_sysinfo.h` の `FLE_SYS_MAX_PROCESSORS` が 32 であること、`fle_sysinfo.c` の GLPI のレコード用バッファが `FLE_SYS_MAX_PROCESSORS` 個まで確保されていること(0312e で見つけた溢れの再発防止)を文字列で確認する。
- 実行は窓の後: `bash lift/harness/sysinfo_wasm/run.sh`(emcc 1 回、数秒〔読み〕)。**emcc の警告・EM_JS の書き方の誤りが残っている可能性は否定できない**(コンパイルしていない)。


## PM の決定と取り込み(2026-11-06 22:59、docs のみ)

取り込み: mt3 の 0312g(B-FLE の設計の読み、S3a の `fle_sys_processors` の clamp の node テストの下書き)を置いた。(1) は B1 の link map・wasm の section・plug1 の表からの**読み**、(2) は**書いただけで未実行**(emcc も未実行)。ビルド・Chrome・依頼者の機械は 0 件。

事実として受け取る(mt3 の集計): B1 の最終 wasm 142,986,125 B = code 140,254,002(106,214 関数)+ data 2,229,452 + elem 307,297 ほか。link 直後の code 172.07 MB は FLEngine の C 75.83 MB(43.9%)・他 7 モジュールの C 93.78 MB(54.3%)・runtime 0.70 MB(0.4%)。wasm-opt -Oz が C 部を 0.764 倍にする。**plug1 の係数 7.74 は pre-opt の値**(FLEngine の関数 10,677,239 B に対し B1 の pre-opt 82.10 MB = 7.69 倍)で、最終 wasm 基準なら Balance は共有なし 約 11.6 MB・B-FLE 約 2.6 MB(0309 の 14.6 / 3.3 より約 3 割小さい可能性、読み。Balance を wasm にした実測は 0 件)。Balance の共有なし 14.59 MB の内訳(plug1 の実測 × 係数): FLEngine に無い plugin 本体 2.80 MB、FLEngine と同一で共有できる関数 10.09 MB(78.3%)、データ 1.69 MB(固有 0.47)。runtime の C 部は静的結合なら追加 0(runtime は 1 つしか持てない: fle_realmod.c の `mods[RM_MAX]`、SRW・CS・時計、`fle_gil_word` が状態を持つため)。

判断:
1. **S1 の基準は『対応するネイティブの 2 倍以下(モジュールごと、全体で)』という比(performance-targets §4b の定義)を正式な基準とする**。154,885,888 B は現行 8 モジュールのネイティブ合計 77,442,944 B の 2 倍であって、plugin を足すと分母(plugin のネイティブ)も増える。ただし絶対値の余白(残り 11.90 MB)も併記する: B-FLE でも Delphi を足せるのは 3 つ(Balance + Filter + Delay 2 = +10.2 MB で 153.2 MB、PEQ2 で 157.8 MB)という読みは、plugin の分母を入れる比の基準では全体比を下げる(限界比 0.88〜1.45 < 2)。**4 つ目以降は絶対値の基準なら別 module 化が必須、比の基準なら不要**。実測(Balance を wasm にする)が出るまで、どちらも読みのままにする。
2. **B-FLE は 2 つ目の plugin の前に必須**(共有なしは 2 つ目で全体比 2.03 倍)。packaging は mt3 の推薦を採る: **plugin は今の方式のまま FLEngine と同じ 1 つの wasm に静的に結合**(`RM_MAX 8` を拡げる)。**Emscripten の dynamic linking は採らない**(PIC 化で主 module の code が +10〜25%〔読み・未測定〕、pthread + JSPI + 143 MB の実績が無い)。別 module 化は B-FLE の後、S1/P1 が実際に効いたときだけ(新しい仕組みが大きい: dispatch 表の動的登録、全 Worker への instantiate の配布、module ごとの data base と再配置、import の export、表 index の割当)。
3. **B-FLE の最小の確かめを採る**(mt3 の次の仕事、窓の後に fe-lift の小さい実験として。plug1 の推奨 2): Flanger と Balance の 2 つで、データ参照を表経由にした生成 C がモジュール間で同じになるか(wasm にする前に C の一致だけ、約 1〜2 人日〔読み〕)。難所は mt3 の整理どおり: 関数の 72% がデータに触れる、Delphi の RTL が DLL ごとに自分の状態(heap manager・例外のフック)を持つので共有した関数が FLEngine のデータを触ると忠実性が壊れる、FLEngine 側も表経由にする必要(再 lift の可能性)、データ参照の 1 段間接の性能。**lift の忠実性の方針に照らして、B-FLE は『FL の挙動の代行』ではなく生成物の共有(通信/変換の削減ではなく大きさの削減)**で、区分は新設せず『S1 の達成のための生成器の変更』として決定文書に書く。
4. node テスト(`lift/harness/sysinfo_wasm/`)は S3a のビルドの窓で走らせる(emcc の警告・EM_JS の書き方の誤りが残っている可能性がある)。
5. 未確認: Balance の wasm の実測の大きさ、B-FLE の C の一致、node テストの実行、依頼者の機械。
