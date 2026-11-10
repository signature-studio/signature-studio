# 0156: 最初のサードパーティ VST3 プラグイン(Xfer OTT 1.37)を lift して native と wasm で実際に実行した(DllMain から process まで)。process 1 ブロック(4096 フレーム = 92.88 ms)の CPU 時間は native -O2 で約 9.7 ms、wasm -O2 で約 11.6 ms、wasm -Oz で約 48 ms。実行された原命令は 1 ブロック 7.82M。実機との照合は無い。すべて仮の決定

状態: **仮の決定(すべての選択)**。依頼は PM(番号 0156 は PM の割り当て、仮)。調査・実装・ビルド・測定・文書は作業者(vst2)が行った(2026-11-03 03:43〜。HEAD **0991f92** の `git archive` に対する差分 `vst2.diff`。`patch -p1 --dry-run` は 0991f92 から進んだワークツリー〔HEAD 83006a9〕にも当たる)。**編集した既存ファイルは 4 つだけ**(いずれも `lift/crates/fe-lift/src/`): `imports_table.rs`・`lib.rs`・`main.rs`(opt-in の `--extra-imports ROWS.txt`。指定が無いときの生成 C は 1 バイトも変わらない)、`prefetch.rs`(PREFETCHW を他の prefetch と同じ本体で足した)。`lift/runtime/` は**無変更**。新規ファイルは `lift/harness/vst/` の下だけ。OTT のバイナリ・生成 C は成果物に入れていない。

> **これは lift した OTT の機械語を native(と wasm)で実行した最初の実測である。実機の OTT(Windows)との照合は無い**(Windows 実機なし、Wine は禁止)。出力が『正しい』ことは示していない。示したのは (i) 実行できたこと、(ii) 出力が有限・決定的・入力とパラメータに依存すること、(iii) native と wasm とコンパイル構成が違っても出力が 1 ビットも違わないこと、(iv) 時間と実行命令数。

**進捗の誠実性(`docs/policies/progress-integrity.md`)**: 結果の数字・ハッシュ・波形はすべて、lift した OTT の機械語が実際に実行された出力であり、事前生成・再生・固定データ・モックを含まない。代役に当たるのは**環境側**だけで(GDI+ のスタンドイン〔何も描かない〕、`d2d1.dll` 無し、フォントの登録を装う応答、API セットの対応づけ、プロファイルフォルダの seed、ホストのオブジェクト)、いずれも §5 に `S-vst-…` として列挙した。これらはプラグインの編集画面の経路を測るには使えない。G 段階表の到達には数えない(決定 0156 は G11 の最初の実測であって、P3 の達成ではない)。

区分の凡例: **実測**＝このセッションのコマンドの出力を見た事実。**読み**＝ファイル・公開資料・逆アセンブルの解釈(確かめていない)。**推測**＝読みからさらに推し量ったもの。**未確認**。**出典なしの選択**＝名前つき(`S-vst-…`、撤回条件つき)。時間はすべて**スレッドの CPU 時間**(`clock_gettime(CLOCK_THREAD_CPUTIME_ID)`)で、測ったときの load average を併記する。この機械(AMD Ryzen 9 9950X、16 コア・32 スレッド)は他の作業と共有で、測定中の load average は 14〜45 だった。壁時計は load が 8 以下のときしか意味が無いので、結果の表には CPU 秒だけを使う。

## 先に結論(実測)

| 項目 | 結果 |
|---|---|
| **実行された範囲** | `LoadLibraryW("ott.vst3")`(実在モジュールとして載せた OTT。`DllMain(DLL_PROCESS_ATTACH)` がその中で走り戻り値 1)→ `InitDll()` = 1 → `GetPluginFactory()` → `getFactoryInfo`(vendor "Xfer Records")→ `countClasses()` = 1 → `getClassInfo`/`getClassInfo2`(Audio Module Class、名前 "OTT"、subCategories "Fx\|Dynamics"、version "1,3,7,0"、SDK "VST 3.7.5")→ `createInstance(IComponent)` → `initialize(IHostApplication)` → QI(`IAudioProcessor`、`IEditController`。**同じオブジェクト**)→ `setComponentHandler` → パラメータ 21 個(Depth, Time, In Gain, Out Gain, Clean XOV, Thresh L/M/H, Gain L/M/H, Byp Upwd/Dnwd L/M/H, Upwd/Dnwd Strgth, Bypass, Program)→ バス(ステレオ入力 1・出力 1)→ `setBusArrangements`(ステレオ)→ `canProcessSampleSize(float32)` = ok → `setupProcessing`(realtime、float32、最大 4096、44100 Hz)→ `setActive(true)` → `setProcessing(true)` → **`process` を 4096 フレームで 40 ブロック** → `setProcessing(false)`・`setActive(false)`・`terminate`・`release` → `ExitDll()`。止まった所は無い。2 個目のインスタンス(createInstance → setActive)も作れた |
| 出力ハッシュ | 既定の入力の 40 ブロックの出力(float32 327,680 サンプル)の FNV-1a は **`8104f50e938ccf36`**。native -O0・-O2(builtin 有効)・-O2(`-fno-builtin`)・wasm -O2・wasm -Oz の**全構成で一致**。出力ファイルは native -O2 と wasm -O2 で**バイト一致**(`cmp`)。同じ構成の再実行 3 回も一致(決定的) |
| **process 1 ブロック(4096 フレーム、実時間 92.88 ms)の CPU 時間**(2 回の測定、各 3 回の実行の中央値の中央値。load 14〜37) | **native -O2・builtin 有効(hle1 の最終フラグ `-fno-builtin-fprintf -fno-builtin-fputs`)= 9.7 ms(92.88 ms の 10.4%)**、native -O2・`-fno-builtin`(HEAD の本番フラグ)= 46 ms(36〜57。49%)、native -O0 = 105 ms(113%)、**wasm -O2(node 24.19)= 11.6 ms(10.0〜14.2。12.5%)**、**wasm -Oz = 48 ms(42〜56。51%)**。ばらつきは大きい(±15〜40%。表 3) |
| 実行された原命令数 | process 1 ブロック = **7,822,247**(40 ブロックで同一、決定的)。内訳: スカラー SSE 浮動小数 4,157,787(53.2%)、その他の SSE 1,289,233(16.5%)、整数ほか 2,375,227(30.4%)。初期化(DllMain〜setActive)= **101,669,246**(DllMain だけ 152,184) |
| lift 後の 1 原命令あたり | process 1.24 ns(native -O2)、整数中心の初期化 0.43 ns |
| **倍率 S**(lift した C の時間 ÷ 同じ原命令数をネイティブで実行する推定時間) | **仮定しだいで 2〜25(表 5)**。この CPU(約 5 GHz)で IPC 2〜4(= 0.05〜0.10 ns/命令)なら **S = 12〜25**(native -O2。wasm -O2 は 15〜30)、PM の例の 0.25〜0.5 ns/命令なら 2〜5。**中心の見積りは S ≈ 12〜15、範囲 5〜30**(推測)。実機の OTT の負荷は測れない |
| **X(OTT 1 インスタンスのネイティブ負荷、1 ブロックの実時間比)の推定** | 0.4〜0.8%(IPC 2〜4)〜 2.1〜4.2%(0.25〜0.5 ns/命令)。公開情報は定量値無し(§7) |
| 最悪ブロック | 無音入力を続けると 1 ブロックの原命令が 7.8M → 10.9M に増え、native -O2 の CPU 時間が 18〜20 ms(約 2 倍、20%)に増える(最大 36 ms を load 40 で観測)。wasm -O2 では 12〜16 ms。**ホストの FTZ/DAZ ではなく、ゲストの命令の増加と、非正規数の `mulsd`(約 80 万回)が入る従来の整数の遅い経路**(§2.6) |
| 出力の健全性(実機との照合は**無し**) | 全サンプル有限(非有限 0)。入力を 0.5 倍で出力ハッシュが変わる(`0c435d2acaa363ed`)。Depth を変えると変わる。無音入力 → 無音出力(maxabs 0)。Depth = 0 では出力が入力に 2 サンプル遅れて(`getLatencySamples()` = 2 と一致、公開レビューの『latency of 2 samples』とも一致)ゲイン 0.994、残差 11% |
| 壁(停止) | **28 個**(§4、`tables/walls.tsv`)。lift の穴 4(.pdata 外の関数、MSVC ジャンプ表、prefetchw)、runtime の欠け 17、プラグインが要求する環境 6(TLS、プロファイルフォルダ、Direct2D、GDI+、フォント)、ハーネスの誤り 1 |
| wasm | emcc 6.0.12 -O2: 28.2 MB(OTT.vst3 3.24 MB の 8.7 倍。runtime とハーネスを含む)、-Oz: 13.0 MB(4.0 倍)。node 24.19 で native と**同じ出力**。process は -O2 で native の 1.0〜1.4 倍(中央値 1.2 倍)、-Oz は wasm -O2 の約 4 倍遅い |

肯定的に見せる点は限られる。(1) OTT は**軽いプラグイン**とされる(公開レビュー)。その 1 インスタンスが lift 後で 1 コアの 10〜14%(-O2)〜 51%(wasm -Oz)を使う。参照プロジェクトには VST3 が 20 インスタンスある。(2) 倍率 S は**ネイティブの CPI の仮定に比例して動く**(2〜25)。実機の OTT を測れないので、S は 1 点ではなく範囲である。(3) 最悪ブロックは平均の約 2 倍。(4) GUI は『何も描かない』スタンドインで通した(§4)。実際に編集画面を作る経路は測っていない。(5) 1 本のプラグイン、1 つの入力信号(サイン+ノイズ)の結果で、一般化はできない。

## 1. 何をしたか(方法)

### 1.1 入力と生成

- 入力: `lift/out-vst1/bin/OTT.vst3`(PE DLL、3,244,032 B、SHA-256 `8965c711…d5a`、ImageBase 0x180000000、`.pdata` 7,951、TLS ディレクトリ有り・コールバック 0)。読み取りのみ。
- `fe-lift lift OTT.vst3 gen.c --pdata --seeds SEEDS --extra-imports ROWS`(0.8 秒、RSS 67 MB)。閉包 502,243 命令、関数 8,899、生成 C 44.3 MB、`bad_nodes` 0、未対応サイト 2,258(うち int3 パディング 984、AVX/FMA 約 1,000、IAT 経由の応答なし約 200。**実行では 1 つも踏まなかった**。AVX/FMA の経路は CPUID が SSE2 までを名乗るので選ばれなかった、と読む)。
- **種(seeds)**: (a) `fe-decode reloc-seeds --not-function` の 1,892 個(どれも `.pdata` の範囲の外にある、CRT の初期化表・vtable が指すリーフ関数)、(b) `lift/harness/seeds/lea_leaf_candidates.py` の 202 個(`lea` だけで参照されるリーフ関数)、(c) fe-lift が表を導出できなかった MSVC ジャンプ表 44 サイトの宛先 456 個(`tools/rejected_seeds.py`。表の添字の上限が `cmp` から取れた 12 サイトはその値、取れなかった 32 サイトは『表の近傍で実行セクション内の連続した RVA』で打ち切り。`fe-lift` の生成を繰り返して新しい拒否が出なくなるまで 3 周)。合計 **2,547 個**。S-vst-seeds、S-vst-jt-bound。
- 生成 C を 4 MB ごとに 12 個の翻訳単位に分け(`split_bytes.py`)、`common.c`(R/FL/X、ディスパッチ表、応答表、ホストオブジェクトの 19 行)を `mk_common.py` で作る(`merge_tables_n.py` は 2 表以上が要るので、単一モジュール版を新規に書いた)。
- ビルド: native = clang 21、-O0(-g)・-O2、builtin 有効(`-fno-builtin-fprintf -fno-builtin-fputs`)と `-fno-builtin`、`-ffp-contract=off`。wasm = emcc 6.0.12 -O2 / -Oz、node 24.19(NODERAWFS)。wasm は ImageBase を 0x2000000 に作り直した PE(`fe-pe rebase`)で、種を同じ差だけずらし、閉包が同一(8,899 関数・2,258 サイト)であることを確認した。
- **生成 C は既定のものから変えていない**。命令数の計数は、生成 C を別ファイルに変換する道具(`count_instrument.py`)で作り、既定のビルドは変換前のファイルを使う。

### 1.2 ハーネス(`lift/harness/vst/`)

- `main_vst.c`: PE を ImageBase に載せ、TLS を用意し、`LoadLibraryW("ott.vst3")`(実在モジュール、決定 0082。DllMain がその中で走る)から、上の手順を VST3 の公開文書(Steinberg の dev portal「Audio Processor Call Sequence」の状態遷移)の順に、**ゲストの vtable 呼び出し(`call [vtable+8*slot]` → `fle_dispatch`)**で呼ぶ。引数は Windows x64 規約(rcx/rdx/r8/r9、5・6 番目はスタック、double は xmm)。
- ホストのオブジェクトは C で実装した(`vst_host.c`): `IHostApplication`(`getName` = "FL Studio"、`createInstance` は kNotImplemented)、`IComponentHandler`、`IParameterChanges` + `IParamValueQueue`(入力と出力)、`IEventList`(空)。メソッドの擬似 VA は `vst_hm.def` で決まり、`common.c` のディスパッチ表に行として入る。
- 公開資料: SDK のヘッダ(https://github.com/steinbergmedia/vst3_pluginterfaces の `base/funknown.h`、`base/ipluginbase.h`、`vst/ivstcomponent.h`、`vst/ivstaudioprocessor.h`、`vst/ivsteditcontroller.h`、`vst/ivsthostapplication.h`、`vst/ivstparameterchanges.h`、`vst/ivstevents.h`、`vst/ivstprocesscontext.h`、`vst/vsttypes.h`。いずれも 2026-11-03 に HTTP 200 で取得)から、IID(`INLINE_UID` の COM 互換のバイト順)、構造体の配置(`PClassInfo2` 440 B、`ParameterInfo` 792 B、`ProcessSetup` 24 B、`AudioBusBuffers` 24 B、`ProcessData` 80 B、`ProcessContext` 112 B、`BusInfo` 276 B。`_Static_assert` で確認)、vtable のスロット番号を取った。長い引用はしていない。
- 入力信号: 左 = 0.5·sin(2π·440 t) + 0.05·ノイズ、右 = 0.4·sin(2π·660 t) + 0.05·ノイズ(ノイズは固定種の xorshift32)。`ProcessContext` は再生中・テンポ 200・4/4・位置を毎ブロック進める(S-vst-ctx。参照プロジェクトのテンポ)。パラメータ入力: 既定は無し(`inputParameterChanges` は空のキュー、`inputEvents` は空)。
- 計時: `process` 呼び出しの前後のスレッド CPU 時間。ブロック 0 は初回(ページの初回アクセス、wasm では JIT の昇格)を含むので別に示す。中央値は 1〜39 番のブロック。3 回の実行の中央値の中央値を表に使う。
- 原命令数: `count_instrument.py` が生成 C の各関数を基本ブロックに分け、先頭に `FLE_CNT(n, s, p)`(n = ブロックの原命令数、s = スカラー SSE 浮動小数、p = その他の SSE)を入れる。**照合**: 命令ごとに数える版(`--per-insn`)でも、初期化 101,669,246、process 1 ブロック 7,822,247、s 4,157,787、p 1,289,233 と**完全に一致**した。ただしどちらも『生成 C の行』を数えており、**原バイナリの実行命令列との照合ではない**(`rep` 付き文字列命令は 1 実行 1 命令として数える。生成 C が原命令を過不足なく写しているという前提)。

## 2. 結果(実測)

### 2.1 実行の段階(native -O2、builtin 有効、`vst-native OTT.vst3`)

| 段階 | 戻り値/事実 | CPU 時間 | 原命令数 |
|---|---|---:|---:|
| `LoadLibraryW` → `DllMain(ATTACH)` | 戻り値 1(CRT 初期化 + C++ の静的初期化) | 1.0〜1.4 ms | 152,184 |
| `InitDll` ・ `GetPluginFactory` ・ factory の各呼び出し | ok | (下に含む) | |
| `createInstance` + `initialize` | ok | 約 35 ms | 約 101.2M |
| `setupProcessing` + `setActive(true)` ほか | ok | (含む) | |
| **DllMain〜setActive の合計** | | **37〜50 ms** | **101,669,246** |
| 2 個目のインスタンス(createInstance → setActive) | ok | 33〜38 ms | 101,383,653 |
| `process` 4096 フレーム(1 ブロック) | `kResultOk` | 8.2〜13.1 ms(中央値 9.4〜10.6) | 7,822,247 |

初期化の約 1 億命令は**インスタンスごと**(2 個目も同じ 1.01 億)。主に GUI の資源の準備(PNG のデコードと GDI+ ビットマップの組み立て、フォントの一時ファイル)と読む(推測。関数別の計数はしていない)。-O0 では 820 ms、wasm -O2 では 100〜280 ms(node。V8 の昇格を含む)、wasm -Oz では 350〜490 ms。

### 2.2 process 1 ブロックの CPU 時間(表 3。load はその実行の直前の `/proc/loadavg` の 1 分値)

| 構成 | 3 回の実行の中央値(ms) | 中央値の中央値 | 92.88 ms に対する割合 | 最小〜最大(ブロック) | load |
|---|---|---:|---:|---|---|
| native -O2、**builtin 有効**(最終フラグ) | A: 9.89 / 9.54 / 9.41(load 36〜38)、9.57 / 9.67 / 10.49(load 14)。B: 10.48 / 10.01 / 8.87(load 22) | **9.7** | **10.4%** | 7.8〜13.7 | 14〜38 |
| native -O2、`-fno-builtin`(HEAD の本番フラグ) | A: 55.2 / 56.6 / 56.3(load 32)。B: 37.8 / 35.9 / 36.2(load 22) | 45.7(36〜57。二つの測定で食い違う) | 49%(39〜61%) | 30〜63 | 22〜32 |
| native -O0 | A: 103.9 / 108.0 / 104.6(load 30〜34)。B: 106.2 / 108.0 / 104.7(load 22) | 105.4 | 113% | 91〜120 | 22〜34 |
| wasm -O2(node 24.19) | A: 10.5 / 10.4 / 10.0(load 14)。B: 14.1 / 12.8 / 14.2(load 22) | **11.6**(10.0〜14.2) | **12.5%** | 8.7〜100(ブロック 0 = 22〜33。外れ値あり) | 14〜22 |
| wasm -Oz(node 24.19) | A: 45.9 / 41.7 / 45.3(load 14)。B: 49.6 / 53.0 / 56.2(load 22) | 47.7(41.7〜56.2) | 51% | 37〜149 | 14〜22 |

- `-fno-builtin` の外し(builtin 有効)で process が **3.7〜5.9 倍**速い(測定 A で 56.3 → 9.5 ms の 5.9 倍、測定 B で 36.0 → 10.0 ms の 3.6 倍。`-fno-builtin` 側の時間は負荷で大きく動く: `memcpy` の呼び出しが SMT の相手に影響されやすい、と読む。builtin 有効側は安定)。hle1(決定 0151)の『FLEngine の起動が 2.8 倍』と同じ原因(`fle_ld`/`fle_st` の `memcpy` が組み込みにならない)で、別の物で独立に確かめられた。hle1 の最終フラグ(`-fno-builtin-fprintf -fno-builtin-fputs`)の行が『builtin 有効』の行である。
- wasm -O2 の process は native -O2 の 1.0〜1.4 倍(同じ負荷の対で、測定 A 10.4 対 約 10、測定 B 14.1 対 10.0)。-Oz は wasm -O2 の約 4 倍遅い(4.1 倍)。node(V8)でのブロック 0 は 22〜33 ms(昇格前)で、以後 9〜14 ms。測定 B の wasm には 100 ms の外れ値が 1 回あった(load 22 で他のプロセスに取られた CPU 時間と読む)。**ブラウザ(Chrome)では測っていない**(V8 は同じ系統だが、AudioWorklet の中の実行は未測定)。
- 負荷が高い(load 30 超)実行の CPU 時間は、SMT の相手スレッドの影響で膨らむ。同じ構成の再測定は load 14 で 9.4〜10.5 ms の範囲に収まった。
- 参考: 全体の表は `tables/timing_runs.tsv`(36 行)、`tables/s_estimate.tsv`。

### 2.3 実行された原命令の内訳(`RESULT … icount_*`)

| | process 1 ブロック | 初期化(DllMain〜setActive) |
|---|---:|---:|
| 全部 | 7,822,247 | 101,669,246 |
| スカラー SSE 浮動小数(`addsd`・`mulsd`・`movsd`・`cvtsi2sd`・`comisd` など) | 4,157,787(53.2%) | 101,616 |
| その他の SSE(packed 演算・`movaps`/`movups`・SSE 整数) | 1,289,233(16.5%) | 2,123,227 |
| 整数ほか | 2,375,227(30.4%) | 99,444,403 |

プラグインは**倍精度のスカラー演算が中心**(Xfer の公式フォーラムの書き込み『Clean XOV is a double-precision crossover』と、パラメータ Clean XOV(id 4)の存在と合う。読み)。1 サンプルフレームあたり 1,910 原命令。lift 後は 1 原命令あたり process 1.22 ns、初期化 0.43 ns。

### 2.4 倍率 S の推定(表 5。`tables/s_estimate.tsv`)

S = (lift した C の 1 ブロックの CPU 時間: native -O2 9.7 ms、wasm -O2 11.6 ms、wasm -Oz 47.7 ms) ÷ (7,822,247 × ネイティブの 1 命令あたりの時間 t)。**t は測れない仮定**。

| 仮定した t(ns/原命令) | 対応する native の 1 ブロック | X(92.88 ms に対する割合) | S: native -O2 | S: wasm -O2 | S: wasm -Oz |
|---:|---:|---:|---:|---:|---:|
| 0.05 | 0.39 ms | 0.42% | 25 | 30 | 122 |
| 0.083 | 0.65 ms | 0.70% | 15 | 18 | 73 |
| 0.10 | 0.78 ms | 0.84% | 12 | 15 | 61 |
| 0.167 | 1.31 ms | 1.41% | 7 | 9 | 37 |
| 0.25 | 1.96 ms | 2.11% | 5 | 6 | 24 |
| 0.50 | 3.91 ms | 4.21% | 2 | 3 | 12 |

- 仮定の根拠(推測): この機械の Zen 5 は約 5 GHz。倍精度スカラー演算中心の DSP コードで IPC 2〜4 とすると t = 0.05〜0.10 ns。PM の例の 0.25〜0.5 ns/命令は IPC 0.4〜0.8 に当たり、このコードには低すぎる、と読む(**S の下限**として併記)。**S の比は測った機械と同じ機械で考える**(lift 後の時間もこの機械の 5 GHz で測った)ので、クライアントの CPU が遅くても S は同じで、lift 後の時間と native の時間が同じ割合で伸びる。
- 中心の見積り: **S ≈ 12〜15(範囲 5〜30)**(native -O2 で 12、wasm -O2 で 15)。wasm -Oz は 37〜122 に当たる(中心 61)。
- vst1 の外挿式(S ≈ 5.8·w_i + 18.5·w_s + 152·w_p、w はネイティブ時間の割合)に、**命令の割合**(w_i 0.30、w_s 0.53、w_p 0.165)をそのまま入れると S ≈ 37 になり、今回の中心 12〜15(CPI 仮定しだいで 5〜30)より大きい。packed 命令の多くが演算ではなく移動(`movaps`/`unpck`)だったことなどが理由、と読む(推測。式の重みはネイティブの時間の割合なので、命令の割合を入れるのは粗い)。
- ネイティブ側の IPC を測る手段は無い(`perf_event_paranoid` = 4 で性能カウンタが使えない)。

### 2.5 出力の健全性(実機との照合は**無い**)

`vst-native OTT.vst3 [--scale S] [--silence] [--param-first V] [--rate HZ]`、40 ブロック × 4096 フレーム × 2 チャンネル、native -O2。表は `tables/timing_runs.tsv`。

| シナリオ | 出力ハッシュ | maxabs | RMS | 結果 |
|---|---|---:|---:|---|
| 既定(3 回、別プロセス) | `8104f50e938ccf36` ×3 | 0.2248 | 0.0638 | **決定的**(3 回一致)。非有限 0 |
| 入力 0.5 倍 | `0c435d2acaa363ed` | 0.2168 | 0.0624 | 既定と**違う**(入力の振幅に依存する。入力を半分にしても出力の RMS が 2% しか減らない = 圧縮が効いている) |
| 無音 | `96deca0f23b22325` | 0 | 0 | 無音 → 無音 |
| `Depth` = 0.25(`setParamNormalized` とパラメータ変更キューの両方) | `07f081139e0d6448` | 0.4723 | 0.2265 | 既定と違う(**パラメータに依存する**) |
| `Depth` = 0.0 | `22038d79605476c1` | 0.6002 | 0.3207 | 出力 RMS が入力(左 0.354、右 0.283)に近い。**左チャンネルの出力と入力の最良一致はラグ 2 サンプル、ゲイン 0.994、残差 11%** |
| 48 kHz | `6129402b8e8bab77` | 0.2191 | 0.0638 | 既定と違う(サンプルレートに依存する) |

- Depth 0 で『入力に 2 サンプル遅れて通る』ことは、プラグイン自身が返す `getLatencySamples()` = 2、公開レビュー(KVR の OTT のページ https://www.kvraudio.com/product/ott-by-xfer-records の利用者レビュー『latency of 2 samples』)と合う。これは**実装が実行されていることの傍証**であって、正しさの照合ではない。Depth 100%(既定)で出力 RMS が入力の 1/5 まで下がる値が実機の OTT と同じかは**未確認**。
- 『同じ入力で 2 回走らせて同一』: 同一構成の 3 回(別プロセス)と、native -O0/-O2(2 種)/wasm -O2/-Oz の構成間で、ハッシュが一致(wasm は全シナリオで native と一致: 既定・0.5 倍・無音・Depth 0.25)。
- ガード付きの割り当て(`GUARD=1`: 各ブロックを PROT_NONE ページの直前に置く)の -O0 ビルドでも、全手順が通り、ハッシュが同じ(ヒープの境界超えなし)。

### 2.6 最悪ブロック(無音入力)の切り分け

無音入力を続けると 1 ブロックの原命令が増える(ブロック 0〜3: 7,841,429 → ブロック 4: 8.6M → ブロック 5〜8: 8.87M → ブロック 9 以降: 10,925,577)。CPU 時間は native -O2 で 8.9 ms → 約 20 ms(1 命令あたり 1.1 → 1.8 ns)、wasm -O2 で約 10 ms → 12〜16 ms。

- **ゲストの MXCSR**: `fle_mxcsr` はプラグインの `ldmxcsr`(静的に 11 サイト)の結果で **0x1fa4 のまま**(例外マスクは全部立ち、FTZ(bit 15)も DAZ(bit 6)も**立っていない**。下位の 0x24 は旗の PE と ZE)。つまり **OTT は FTZ/DAZ を立てない**。生成 C の `fle_mx_*` は MXCSR を尊重するので、非正規数は IEEE どおり扱われる。
- **ホストの MXCSR**: ハーネスは変えない。診断として `VST_DIAG_HOST_FTZ=1` でホストの FTZ|DAZ を立てて走らせると、時間は変わらない(ブロック 15: 20.7 → 18.5 ms)。**ホストの非正規数の補助(microcode assist)が原因ではない**。
- **`FLE_MX_STATS` 版(決定 0143 の統計。native -O2 + 計数、`XCF=-DFLE_MX_STATS=1`)の実測**: 既定の入力では MXCSR 厳密な浮動小数演算 約 1.2 億回のうち、遅い経路に入ったのは `mulsd` 3 回と `ucomisd` 1,475,366 回(`ucomisd` は既定でも 75% が遅い経路)だけ。**無音入力では `mulsd` の遅い経路が 799,376 回**(既定では 3 回。非正規数のオペランドか結果が、`fle_mxcsr_flags.h` の従来の整数実装に回る)で、`addsd`・`subsd` は遅い経路 0 回、`ucomisd` は 1,075,960 回(既定と同程度)。つまり**最悪ブロックの余計な時間は、ホストの非正規数の補助ではなく、非正規数が出る `mulsd` の整数の遅い経路**(= lift の生成の側の性質。実機の CPU は FTZ/DAZ なしの非正規数の `mulsd` にマイクロコードの補助を使うので、ネイティブの OTT も遅くなるが、その大きさは別)。1 つの遅い経路の呼び出しの時間は測っていない。wasm で増え方が小さい理由は**未確認**。
- したがって: 命令数の増加 +39% は**プラグイン自身の命令**(無音のあいだに別の経路を通る。ゲストの結果)。1 命令あたりのコストの増加は、非正規数の `mulsd` の遅い経路(上)による、と読む。

## 3. ビルドの大きさ・時間・メモリ(実測)

| | 値 |
|---|---|
| native 実行ファイル(-O2) | 15.0 MB(runtime とハーネスを含む)。実行時 RSS 24.6 MB |
| wasm -O2 / -Oz | 28.2 MB / 13.0 MB(gzip -9: 4.50 MB / 2.39 MB。OTT.vst3 の gzip -9 は 1.69 MB)。node の RSS 156 MB |
| ビルド時間(load 14〜45、JOBS=3) | clang -O2: 12 単位で 3 分 10 秒(CPU 276 秒)。計数つきの -O2: 6 分 20 秒。clang -O0: 12 秒。emcc -O2: 単位ごとに 24〜30 秒(合計 358 CPU 秒、最大 RSS 738 MB)+ リンク 19 秒(RSS 946 MB)。emcc -Oz: 80 CPU 秒 + リンク 8 秒 |
| fe-lift | 0.8 秒、RSS 67 MB |

メモリは小さい(vst1 の見積りどおり)。最大の単位でも emcc の RSS は 0.74 GB。

## 4. 停止の分類(28 個。`tables/walls.tsv`に全文)

分類: **lift の穴**＝生成器が表・関数・命令を落とした。**runtime の欠け**＝応答が無い/名前付き停止の仮定が合わない。**プラグインが要求する環境**＝OS の状態(プロファイルフォルダ、TLS、グラフィックスの DLL)。**ハーネスの誤り**＝ハーネス側の誤り。

| # | 停止(出力どおり) | 分類 | 足した応答 | 出典なしの選択 |
|---|---|---|---|---|
| 1 | indirect call to 0x1801cec04 is not a known function entry | lift の穴 | `.pdata` 外のリーフ関数を reloc 値から種に(1,892 個。関数 6,676 → 8,243) | S-vst-seeds |
| 2 | unsupported at 0x1801b053c(jmp [IAT] `InitializeCriticalSectionEx`) | runtime の欠け | `fle_imp_InitializeCriticalSectionEx`(`InitializeCriticalSectionAndSpinCount` と同じ)。新規 `--extra-imports` で | — |
| 3 | LoadLibraryExW: module "api-ms-win-core-string-l1-1-0.dll" is not a virtual module | runtime の欠け | api-ms-win-*/ext-ms-win-* を kernel32(ntuser は user32、security は advapi32)のハンドルに(`--wrap`) | S-vst-apiset |
| 4 | indirect call to 0x1801b1af8(MSVC memcpy のジャンプ表) | lift の穴 | 拒否された表 44 サイトの宛先 456 個を種に(3 周) | S-vst-jt-bound |
| 5 | 静的初期化で [4] を読んで SIGSEGV | プラグインが要求する環境 | 実装 TLS: ハーネスが TLS ブロックを作り、`TlsAlloc` の添字を `_tls_index` に書く | S-vst-tls |
| 6 | unsupported at 0x180002cf8(`CoCreateGuid`) | runtime の欠け | `fle_imp_CoCreateGuid` = `UuidCreate` の決定的な流れ | S-vst-guid |
| 7 | GetModuleFileNameW: only the lifted DLL has a path | runtime の欠け | プラグインを実在モジュールとして登録(`fle_realmod_table`、名前 `ott.vst3`)し、ゲストの `LoadLibraryW` で載せる(決定 0082、0104) | S-vst-realmod |
| 8 | (ハーネスの誤り)`getClassInfo2` の表示が文字化け | ハーネスの誤り | `PClassInfo2` のオフセットを直した | — |
| 9 | indirect call to 0x1801acd70(CFG のディスパッチ) | lift の穴 | `lea` だけで参照されるリーフ関数 202 個を種に | S-vst-seeds |
| 10 | GetModuleHandleExW: not answered | runtime の欠け | `--wrap`: FROM_ADDRESS でプラグインの像の中なら ImageBase、名前なら GetModuleHandleW | S-vst-getmodulehandleex |
| 11 | misaligned 16-byte access(`movdqa`) | ハーネスの誤り(ビルド) | 生成単位が読む `fle_modules.h` は停止の応答を `noreturn` と宣言し、応答する wrapper の呼び出しの後ろのコードを clang が捨てた。単位だけ、属性を外した写しを include パスの先頭に | — |
| 12 | unsupported at 0x18002fe5f(`RegisterClassExW`) | runtime の欠け | `WNDCLASSEXW` → `WNDCLASSW` に組み替えて `RegisterClassW` へ(クラス表は 1 つ) | — |
| 13 | CreateWindowExW: hInstance is not the lifted DLL | runtime の欠け | 7 の実在モジュール登録で解消 | S-vst-realmod |
| 14 | CreateWindowExW: the class name is an atom | runtime の欠け | `--wrap`: atom → 登録名(`fle_class_name_of`)にして runtime の `CreateWindowExW` へ | S-vst-classatom |
| 15 | unsupported(`SHGetSpecialFolderPathW`) | runtime の欠け | `SHGetFolderPathW` の引数を並べ替えて呼ぶ | — |
| 16 | malloc(): corrupted top size | プラグインが要求する環境 | `C:\Users\user\AppData\Roaming` が無いと JUCE の `File::isDirectory` が偽になり、プラグインが未初期化のスタックバッファ(CRT の 0..255 の表)を文字列にコピーしてヒープを壊す。VFS にフォルダを置く(`FLE_VFS_SEED`) | S-vst-vfs |
| 17 | CreateFileW: lpSecurityAttributes is not NULL | runtime の欠け | `--wrap`: 記述子が NULL の SECURITY_ATTRIBUTES は NULL として渡す | S-vst-secattr |
| 18 | GetFileType: no standard handle exists | runtime の欠け | `--wrap`: 仮想ボリュームのハンドルは FILE_TYPE_DISK | S-vst-filetype |
| 19 | unsupported at 0x1801dad04(`prefetchw`) | lift の穴 | fe-lift の `prefetch.rs` に足した(ヒントで効果なし。FLEngine の C は変わらない) | — |
| 20 | SetFilePointerEx: not answered | runtime の欠け | `--wrap`: SetFilePointer を距離を 2 分割して呼ぶ | — |
| 21 | unsupported(`FindResourceA`) | runtime の欠け | `FindResourceW` に幅を広げて渡す | — |
| 22 | LockResource で SIGSEGV(exe の像 0x400000 を読む) | runtime の欠け | `--wrap`: プラグインの像の中の VA ならそのまま返す | S-vst-lockresource |
| 23 | unsupported(`InitializeConditionVariable`、`InitializeSRWLock`) | runtime の欠け | 初期値(全ゼロ)を書く | — |
| 24 | LoadLibraryA: module "d2d1.dll" is not a virtual module | プラグインが要求する環境 | `--wrap`: d2d1.dll は 126(JUCE は software renderer に) | S-vst-gfx-absent |
| 25 | unsupported(`GdiplusStartup`) | プラグインが要求する環境(GUI) | `GdiplusStartup`/`Shutdown`/`GdipAlloc`/`GdipFree`(Ok、非ゼロのトークン) | S-vst-gdip |
| 26 | unsupported(`GdipCreateBitmapFromScan0` ほか 5 つ) | プラグインが要求する環境(GUI) | ピクセルバッファだけのスタンドイン(`vst_gdip.c`)。何も描かない | S-vst-gdip |
| 27 | FormatMessageW: dwLanguageId 0x400 is not 0 | runtime の欠け | `--wrap`: 0x0400 を 0 と同じに扱う | S-vst-fmtmsg-lang |
| 28 | unsupported(`AddFontResourceExW`) | プラグインが要求する環境(GUI) | 成功値(1 / TRUE)を返すだけ。フォントは登録しない | S-vst-addfont |

- 大きい壁と書かれがちな物の扱い(gdb の入口ブレークポイントの回数で確認。-O0 の全手順 1 回): **スレッドは壁に出会わなかった**(`CreateThread` 0 回、`WaitForSingleObject` 0 回)。**C++ 例外は投げられなかった**(`RaiseException`・`RtlUnwindEx` 0 回。静的 CRT の例外処理は import にあるが、この手順では通らない)。呼ばれた物: `InitializeCriticalSectionAndSpinCount` 94 回、`EnterCriticalSection` 405 回、`CreateDirectoryW` 11 回(ハーネスの seed 10 + プラグインの `Xfer` フォルダ 1)、`CreateEventW` 2、`SetTimer` 1(JUCE の隠し窓。メッセージループを回さないので**タイマーは一度も発火しない**)、`CreateWindowExW` 1、`ReadFile` 1、`WriteFile` 1。`PostMessageW`・`SendMessageW` は 0 回。**SQLite・ネットワーク・認証は無い**(OTT に import が無い)。**GUI**は唯一の大きい壁で、グラフィックスの DLL の有無(Direct2D)と GDI+ のビットマップの最小のスタンドインで通した。編集画面(`createView`)は呼んでいない。
- 実行しなかった事: プラグインの編集画面、状態の保存・復元(`getState`/`setState`)、プログラムリスト、ノート(OTT は FX)、`IConnectionPoint`(コンポーネントとコントローラが同一)。
- 『足した応答』のうち**新規の runtime 関数**は `harness/vst/vst_imp.c`・`vst_gdip.c` に置き(runtime には置いていない。tests/ が runtime の `fle_imp_*` を走査する可能性を避ける)、fe-lift が `--extra-imports` で表に足す。既定の応答表(608 行)は変えていない。

## 5. 出典なしの選択(`S-vst-…`。すべて撤回条件つき)

| 名前 | 内容 | 撤回の条件 |
|---|---|---|
| S-vst-seeds | .pdata 外の関数の種: reloc 値 1,892 + `lea` 202(既存の道具)。誤って命令の途中を指す種は無かった(`bad_nodes` 0、実行で踏んだ未対応サイト 0) | 生成器が自分で種を求める(別の決定) |
| S-vst-jt-bound | 拒否された MSVC ジャンプ表の宛先: 添字の上限が取れた 12 サイトはその値、32 サイトは近傍打ち切り(±0x1800、実行セクション内、最大 64)。隣の表の宛先を余分に含み得る(コードの先頭ではあるが、使われない関数が増える) | fe-lift が表を導出できるようになる |
| S-vst-tls | 実装 TLS: ハーネスがローダの仕事をする(`TlsAlloc` の添字、ブロック = テンプレート + ゼロ埋め)。他のスレッドのブロックは無い | ローダのモデル(決定 0122 のスレッドに TLS ブロックを足す) |
| S-vst-apiset | `api-ms-win-*`/`ext-ms-win-*` への LoadLibrary は kernel32 等のハンドル(GetProcAddress は行があれば見つかり、無ければ NULL) | 実 Windows では各 API セットが別の転送先 |
| S-vst-realmod | プラグインを実在モジュール `ott.vst3` として載せる。`GetModuleFileNameW` は決定 0104 の値(`Shared\` + エクスポートの Name)で、FL が渡す実パスではない | FL のプラグインの読み込み(`LoadLibraryW(path)`)のモデル |
| S-vst-getmodulehandleex | `GetModuleHandleExW`: 像の中の FROM_ADDRESS → ImageBase、名前 → GetModuleHandleW、他は 126 | runtime が実装する |
| S-vst-classatom | `CreateWindowExW` の atom を登録名に直す(runtime のテストは atom で停止を要求している) | runtime が atom を受ける |
| S-vst-guid | `CoCreateGuid` は決定的な流れ(`UuidCreate` と同じ)。実 Windows は乱数 | — |
| S-vst-vfs | 仮想ボリュームに `C:\Users\user\AppData\Roaming` ほか 10 個のフォルダ(`vfs-seed-vst.tsv`)。実 Windows にある物 | 決定 0107 の VFS の初期状態 |
| S-vst-secattr | `SECURITY_ATTRIBUTES`(記述子 NULL)は無視 | — |
| S-vst-filetype | 仮想ボリュームのハンドルは FILE_TYPE_DISK | runtime が GetFileType を VFS に結ぶ |
| S-vst-lockresource | プラグインの像の中の VA の `LockResource` はその値 | runtime が exe の像を仮定しなくなる |
| S-vst-gfx-absent | `d2d1.dll` は無い(126)。実 Windows 10 にはある。JUCE はソフト描画に落ちる | 編集画面を測るなら Direct2D のモデル |
| S-vst-gdip | GDI+ はスタンドイン(`GdiplusStartup` = Ok、ビットマップ = ピクセルバッファ、描画は何もしない) | 編集画面を測るなら実装 |
| S-vst-addfont | `AddFontResourceExW` = 1、`RemoveFontResourceExW` = TRUE。フォントは登録しない | — |
| S-vst-fmtmsg-lang | `FormatMessageW` の言語 0x0400 = 0 | runtime が 0x0400 を受ける |
| S-vst-trycs | `TryEnterCriticalSection` = Enter + 1(他スレッドの所有は考えない)。**この手順では呼ばれていない**(先に行を足しただけ) | スレッドを使う実行 |
| S-vst-hostname | `IHostApplication::getName` = "FL Studio" | FL が渡す文字列を実機で見る |
| S-vst-hostmsg | `IHostApplication::createInstance` は kNotImplemented(`IMessage`・`IAttributeList` を持たない) | コンポーネントとコントローラが別クラスのプラグインを載せるとき |
| S-vst-qi | ホストのオブジェクトの `queryInterface` は FUnknown と自分の IID だけ(`IComponentHandler2`・`IPlugFrame`・`IUnitHandler` は拒否) | 同上 |
| S-vst-ctx | `ProcessContext`: 再生中、テンポ 200、4/4、位置を毎ブロック進める | FL の値 |
| S-vst-signal | 入力信号(サイン + ノイズ)、ステレオ、ブロック 4096、44100 Hz、float32、realtime | PM の指定(サンプルレート、フレーム数、float32) |

## 6. P3 への含意(推測を含む)

測った事実: OTT 1 インスタンスの `process` 4096 フレーム(実時間 92.88 ms)は、**この機械の 1 スレッド**で、native -O2 の lift 後 CPU 9.7 ms(10.4%)、wasm -O2 で 11.6 ms(12.5%)、wasm -Oz で 47.7 ms(51%)。最悪のブロック(無音の 2 秒目以降)は native -O2 で約 20 ms(21%)。

- **Y = S × X**。X(ネイティブ OTT の負荷)は測れない。公開情報は定量値を持たない: 取得して確認した文言は (1) WAVS のプラグインの一覧 https://wavs.com/plugin-directory/ott(2026-11-03 取得)『CPU load: Light』(『published reviews and user forums』の要約と注記)、(2) KVR の利用者レビュー https://www.kvraudio.com/product/ott-by-xfer-records『somewhat noticeable CPU hit』と『latency of 2 samples』、(3) Xfer の公式フォーラム https://xferrecords.com/forums/general/ott-preset(2012-09)『I'm seeing really very high CPU usage in Logic 9.1.3 and OS 10.8.1. More than a couple OTTs rolling and I'm redlining』。**X の数値は無し**(推測として 0.4〜4%、§2.4)。
- 参照プロジェクト(`docs/investigations/reference-project.md`): VST3 20 + VST2 1。OTT はそのうち 2 インスタンス。**OTT だけで 2 × 11.6 = 約 25%(wasm -O2)、wasm -Oz なら約 103%**。残り 18 インスタンス(Serum 2 ×6、Vital、Kilohearts ×8 ほか)の X は未測定。
- **外挿の前提(推測。範囲で)**: 全インスタンスが OTT と同じ S(中心 12〜15、範囲 5〜30)で、1 つのワーカーが FL エンジンを除く全プラグインを担当し、予算が 1 コアの 100% のとき、**プラグイン群のネイティブ負荷の合計 ΣX ≤ 100% / S = 3〜20%**(中心 7〜8%)が要る。vst1 に引用された逸話(Serum 2 が 1 インスタンス 4〜10%、最大 4 つ同時に鳴る区間)だけで 16〜40% があり、**この条件を 2〜10 倍超える**。OTT のように軽い(X ≈ 1%)インスタンスが 20 個でも ΣX ≈ 20% で、S = 12 のとき Σ = 240%。**したがって A(現行の忠実度のまま全部 lift)は、OTT 級に軽いプラグインでも 1 スレッドでは届かない側に強く傾く**(vst1 の結論と同じ方向。ただし OTT の S は vst1 の式の予測 37 よりは小さかった)。
- 最悪の 1 ブロックが予算(92.88 ms ではなく、出力のリード 60 ms〈決定 0139〉)を超えると途切れる。OTT 1 つなら最悪でも 20〜36 ms(load 40 で 36)で、リードの内側。20 インスタンスの**最悪ブロックの同時発生**は未測定。
- 起動(P1): プラグインの初期化は**インスタンスごとに 1 億命令**(native 35 ms、wasm -O2 100〜280 ms、-Oz 350〜490 ms)。20 インスタンスで native 0.7 s、wasm -O2 2〜5 s、wasm -Oz 7〜10 s(推測。プロジェクトを開くときに作るなら P1 の 10 秒に効く)。DllMain は小さい(1 ms 台)。
- 手段(判断は PM): (1) 『builtin 有効』を本番のフラグにする(3.7〜5.9 倍)。(2) wasm は -O2 以上が要る(-Oz は約 4 倍遅い)。(3) 忠実度を落とした音声用の lowering(D1。vst1 §5.4)は、S の中心 12 を下げる余地がある(合成カーネルの what-if では scalar 約 3 倍、packed 9〜11 倍)。今回の mix(スカラー SSE 53%)なら効く側。**未検証**。(4) 初期化の 1 億命令は、GUI の資源の準備と読むので、編集画面を使わない運用なら省く方法を探す価値がある(関数別の計数をしていないので、どの部分かは未確認)。

## 7. 限界と未確認

- **実機との照合は無い**。出力が実機の OTT と同じかは分からない。生成 C が原命令を過不足なく写す前提(命令数の計数も同じ前提)。
- 1 台・共有機(load 14〜45)・1 プラグイン・1 つの入力信号。CPU 時間は SMT の相手に影響される(±15%)。ネイティブの IPC は測れない(性能カウンタ不可)ので S は範囲。
- メッセージループを回していない: 隠し窓の `SetTimer` 1 個のタイマーと、`PostMessage` されたメッセージ(この手順では 0 件)は配送されない。FL がホストとして UI スレッドで回すループの中でプラグインが何をするかは未測定。
- ブラウザ(Chrome)では測っていない。node 24.19 のみ。AudioWorklet・SharedArrayBuffer・メモリ上限 4 GiB(wasm32)の評価はしていない。
- 編集画面・GUI・状態の保存復元・複数バス・MIDI・サンプルレートの変更(48 kHz は 1 回走らせた)・`setProcessing` を実時間スレッドから呼ぶ形は測っていない。
- 最悪ブロックの『1 つの遅い経路の呼び出しの時間』と、wasm で増え方が小さい理由は**未確認**(§2.6)。
- OTT は『ライセンス確認・ネットワーク・レジストリ』を持たない(import 無し、実行でも呼ばれなかった)。**認証は出てこなかったので、回避はしていない**。

## 8. 規則違反・逸脱の自己申告

- 存在しない `bash` ツールを 1 回呼んだ(`echo test` のみ。ハーネスが shell として実行した。副作用なし)。
- cargo の出力先を誤って `lift/out-vst2/edit/lift/target` に作った(29 MB)。成果物に入れず、削除した。
- ビルド中のスクリプトを編集して、最初の -O2 構成が 1 回失敗した(`SRCD: 未割り当ての変数`)。再実行で解消(単位の再コンパイルは不要だった)。
- 負荷の規則(PM の 04:55 の連絡)は受け取った後から守った(`nice -n 15`、同時 3〜4 本)。それ以前の 04:30〜04:50 は、`nice` なしで clang を JOBS=3〜4 で走らせた。
- sleep・nohup・disown・systemd-run・`timeout` コマンド・末尾 `&`・待機ループは使っていない。バックグラウンドは shell ツールの `background: true` だけ。
- git の変更系操作はしていない。tracked ファイルは直接編集していない(`git archive` の展開の編集 + 差分)。書き込み先は `lift/out-vst2/`、`lift/out-git/vst2-results/`、`/tmp/opencode/vst2/` のみ。

## 9. 成果物と再現

`lift/out-git/vst2-results/`: `vst2.diff`(`patch -p1`)、`docs/0156-first-vst3-plugin.md`(本文)、`docs/README-0156-line.txt`、`MERGE-NOTES.md`、`wall-log.md`、`tables/`、`tools/`、`logs/`。OTT のバイナリと生成 C は入れていない(`lift/out-vst1/bin/` から)。

再現(リポジトリのルートから。`lift/out-vst2/` は新しい名前):

1. `git archive HEAD | tar -x -C lift/out-vst2/edit` に `vst2.diff` を `patch -p1`。`CARGO_TARGET_DIR=lift/out-vst2/target cargo build --offline --release -p fe-lift -p fe-decode -p fe-pe`(`lift/` で)。
2. 種: `fe-decode reloc-seeds OTT.vst3 --not-function` → `tools/nonpdata_seeds.py`、`lea_leaf_candidates.py` → `tools/seed_fixpoint.sh`(拒否された表の固定点)。
3. 生成: `bash lift/harness/vst/mkrows.sh rows.txt`、`OUT=… TAG=ott18 FILE=… FEBIN=… LIFT_ARGS="--pdata --seeds SEEDS --extra-imports lift/harness/vst/imports-vst.txt" ROWS=rows.txt bash lift/harness/vst/gen_vst.sh`。
4. ビルド: `OUT=… FILE=… TAG=ott18 OPT=O2 BUILTIN=1 COUNT=0 bash lift/harness/vst/build_vst_native.sh`(COUNT=1 は命令数、COUNT=2 は命令ごとの照合、BUILTIN=0 は `-fno-builtin`、GUARD=1 は境界検査の割り当て)。wasm は `build_vst_wasm.sh`(PE を `fe-pe rebase … 0x2000000` で作り直す)と `run_vst_node.cjs`。
5. 実行: `FLE_VFS_SEED=lift/harness/vst/vfs-seed-vst.tsv native/…/vst-native OTT.vst3 [--blocks N] [--scale S] [--silence] [--param-first V] [--out F] [--dump-in F]`。`tools/measure.sh`・`measure_wasm.sh` が表の測定。
6. 全体テストと a4: §10。

## 10. 既定のビルドが変わらないことの確認

編集した木(`vst2.diff` を当てた木。`lift/runtime/` は無変更)で確認した。

| 確認 | 結果(実測) | 基準(HEAD) |
|---|---|---|
| 全体テスト `bash lift/harness/test.sh full --jobs 2 --threads 2 --fresh`(1 回目。入力の DLL の一部が編集木に無く SKIPPED 15 行) | results=142、passed=2,083、failed=0(622 s) | 142 / 2,083 / 0 |
| 同(2 回目。`test-env.sh --link` で入力をつないだ。SKIPPED は 1 行 = `wall_fr9t` の `FLE_FR9T_CLEAN_RT` 未設定のみ) | results=142、passed=2,083、failed=0(265 s) | 同 |
| FRUITY 無効 a4 `a4-fruity.sh off`(JOBS=2、別の OUT で独立 2 回) | off1・off2 とも init_count 942/942、run_rank 895/895(returned)、functions 5,310、entries **1,009,731**、`returned from entry`、harness 終了 0。gen 18 s / 22 s、build 345 s / 349 s | 942/942、895/895、5,310、1,009,731 |
| 生成 C の不変 | FLEngine(`fe-lift lift FLEngine_x64.dll X --pdata`)の SHA-256 は、クリーンな木の fe-lift と編集した木の fe-lift で同一 `cd9107ce…e50`。OTT の既定の生成(`--seeds` のみ)も同一(`9dc68fcb…`)。`--extra-imports` を付けない限り応答表は 608 行のまま | — |
| 変更の範囲 | `diff -rq clean edit`: 既存ファイル 4 つ(fe-lift の `imports_table.rs`・`lib.rs`・`main.rs`・`prefetch.rs`)と新規の `lift/harness/vst/` のみ | — |

全体テストは cargo の出力先を `lift/out-vst2/target-test` にして走らせ、テストが作る `edit/lift/out-*`・`edit/binaries`・`edit/lift/target` は差分に入る前に削除した(`diff -rq` で確認)。
