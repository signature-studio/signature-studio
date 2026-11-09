# 0172: 丸めモード RC≠0(下・上・0 方向)と FTZ/DAZ の SSE 浮動小数を、結果ビットと旗を変えずに速くした(runtime の `fle_mxcsr_rc.h`)。Serum 2 の実 process(状態 3、1 音、4096 フレーム)が native -O2 で 約 195 → 約 34 ms/ブロック。すべて仮の決定

状態: **仮の決定(すべての選択)**。依頼は PM(番号 0172 は PM の割り当て、仮)。調査・実装・測定・文書は作業者(`rz1`、2026-11-03 10:39〜)。HEAD **b07659a** の `git archive`(`lift docs`)に対する差分 `rz1.diff`(runtime、テスト、道具)と `rz1-wasmgen.diff`(fe-wasmgen。新ファイル中心)。**生成 C は変わらない**(`lift/crates/fe-lift` は無変更。FLEngine の生成 C の SHA-256 は `cd9107cef4cbef14f154645749957b7067be7ed7c2b93f5bbe1f72fd47029e50`(215,811,678 B))。**実機 Windows との照合は 0 件**(この作業が示すのは『ホスト CPU の実命令と同じ』と『従来の整数実装と同じ』で、ゲストの正しさではない)。

区分の凡例は決定 0143・0154 と同じ。**実測**＝このセッションのコマンドの出力を見た事実(この機械の値。16 コア、他の担当 6〜7 本と自分の差分試験・ビルドが稼働していて load は 12〜46 だった)。**計算**。**読み**＝確かめていない解釈。**未確認**。**出典なしの選択**＝名前つき(`S-rz1-…`、§9 に撤回条件つき)。

## 0. 先に書くこと(進捗の誠実性)

**(1) 実エンジンが実行された事実(実測)**: lift した **Serum2.vst3**(サードパーティの VST3、`binaries` の PE を fe-lift で C にして clang -O2 で native 実行する vst3a のハーネス。決定 0165)を、この決定の runtime でビルドし直して実際に実行した。状態 3(FLP から取り出した実パッチ)、保持した C4 1 音、4096 フレーム/ブロック × 40 ブロック。Serum 2 は process の頭で MXCSR=0xffc0(FTZ|DAZ|RC=3|全例外マスク)にして終わりで 0x1f80 に戻す。この条件で新旧の `out_hash` が一致(ビット同一)。**FLEngine_x64.dll(FL Studio 本体)の 16 スナップ**: 実エンジンを実行した: FLEngine の native 16 スナップ(`cmp-short.g3`、FRUITY 有効、全 92 ユニット -O2 + この決定の runtime)の面 SHA・窓表・クロック・進捗 143 行が基準の digest と**完全に一致**(§4)。
**(2) 代役・再生・推測が含まれる範囲**: Serum 2 の実行は vst3a の代役(Direct2D/DirectWrite の失敗応答、音色データの一部の仮想ボリュームなど。決定 0165 §5)の上での実行で、**進捗の到達ではない測定**。差分試験は、ホスト CPU の実命令との比較(乱数と端の値。ゲストの実演算ではない)と、Serum 2 の遅い経路に落ちた演算のオペランドの再生(§3 の `replay128`。値の再生で画面・音の再生ではない)。wasm の数字は node(V8)のもの。

## 先に結論(実測)

| 項目 | 結果 |
|---|---|
| **ホスト CPU(Ryzen 9 9950X)の実命令との一致** | `mxfp_diff rc`: 46 演算(算術 20、変換 22、比較 4)× RC=1,2,3 × FTZ/DAZ 4 通り。合計 **25,200,000,000 件、不一致 0**。**10^8 件/組(12 組 = 1.2×10^9 件/演算)の演算: addps・addsd・addss・cvtsd2ss・divsd・mulps・mulsd・mulss・subpd・subsd・subss**、その他は 2.5×10^7 件/組(10^8 件/演算・RC)。`mxall`(1,024 の MXCSR 状態)129,024,000 件、不一致 0。従来の rand/unmasked/grid32 も不変。建て方の行列(-O0〜-O3、FMA あり、LANES、SLOWBYVAL、厳格な守り、STATIC、DIRECT)と変異検査(27 件中 25 検出、2 件は結果が同じ等価な変異)も通過 |
| wasm(node)での一致 | `emcc -O2` と `-Oz` の `rc` 各 1.38×10^9 件(従来の整数実装と)で不一致 0。`-msimd128` ありも 0。wasm の補助関数 4 つ(`rcgen.rs`)はホスト CPU と 1.2×10^7 件で不一致 0 |
| **Serum 2 の実 process(状態 3、1 音、4096 フレーム/ブロック、native -O2、同じ木で `-DFLE_MX_NORC` の有無だけ)** | cpu_ms_min: **旧 188.4〜201.9 → 新 33.1〜35.2**(約 5.7 倍。8 往復)。状態 0・1・4・5 は 5.1・3.7・6.0・3.7 倍。**5 状態すべてで out_hash が新旧同一で、vst3a の基準と一致**。RC を 0 に落とす診断(出力は不忠実)は 14.3 ms なので、RC=3 のままの新はその 2.3 倍。状態 3 の 4 音和音: 741 → 130.5 ms(5.7 倍、hash `617c196bcd9e0af1` 一致。実時間 92.88 ms は超える) |
| 命令単位(命令数) | RC=3 新 / RC=0 速い経路 = **2.0〜2.7 倍**(addsd 2.04、addps 2.10、mulsd 2.33、mulps 2.27、divsd 2.73)。旧(整数)は addsd 427 命令 = 新の 4.7 倍 |
| 命令単位(時間、native、load 27) | RC=3 / RC=0: scalar の add/sub/mul 1.5〜1.9 倍(div 2.2〜3.5、sqrt 5〜6 倍。ホストの divsd/sqrtsd が 1.3〜1.5 ns で速いため。Serum では div 4・sqrt 0 回/ブロック)、packed 2.1〜3.7 倍。旧(整数)に対し addsd 7.3 倍、mulps 11.7 倍、sqrtsd 31 倍速い。**目標『2 倍以内』は scalar の add/sub/mul と命令数では達成、packed の時間では未達**(packed の RC=0 は 1 ns 前後と極端に速い) |
| 命令単位(wasm、node、load 25) | RC=3 / RC=0: scalar の add/sub/mul 約 3〜4.5 倍(div 約 2 倍)、packed(simd128 なし)約 2.2〜3.5 倍。旧(整数)に対し scalar 6〜7 倍、packed 5〜11 倍速い |
| 大きさ | native -O2 の Serum の 1 ユニット text −5.8%、リンク後の text +70 KB(+0.08%)。wasm -Oz の同ユニットは lifted 関数 +37 B(既定を wasm では展開に)+ weak の補助関数 23.6 KB が 1 回。生成 C は不変 |
| テスト・不変 | 全体テスト results=154・passed=2,121・failed=0(基準 153/2,119/0 + 新規 `rc_helpers`)。a4 FRUITY 無効 942/942・895/895・5,310・1,009,731。FLEngine の生成 C の SHA-256 `cd9107ce…` 不変。fe-lift の `mxcsr_flags` 11 件通過。FLEngine の native 16 スナップ(`cmp-short.g3`、FRUITY 有効、全 92 ユニット -O2 + この決定の runtime)の面 SHA・窓表・クロック・進捗 143 行が基準の digest と**完全に一致** |
| 直接 wasm 生成部 | 補助関数 4 つ(スカラの add/sub/mul、f32・f64)を作り、node でホスト CPU と一致を確認、ループ内 1 回 3.4〜4.0 ns(`f64.add` だけのループ 1.0 ns との差で 約 2.5〜3 ns。C 経路の wasm は約 5.6〜6.0 ns)。`lower.rs` へは未接続(§8) |
| **確認できなかったこと** | 実機 Windows(0 件)、Serum 2 の wasm 実行、6 インスタンス、状態 2 の Serum 2、Chrome、非 x86 ホスト |

## 1. 何を直したか

### 1.1 問題(vst3a の実測、決定 0165)

runtime の MXCSR 厳密な浮動小数(決定 0143・0154)の速い経路は **RC=0(最近接偶数)限定**で、ホスト(C・SSE・wasm)の演算をそのまま使う。Serum 2(サードパーティ VST3)は process の頭で MXCSR=0xffc0(FTZ|DAZ|RC=3〔0 方向〕|全例外マスク|旗クリア)にし、終わりで 0x1f80 に戻す。RC=3 の間は全 FP 命令が整数実装(`fle_mx_pack` / `fle_mx_arith` / `fle_mx_add` …)に落ち、状態 3 の 1 ブロック(4096 フレーム)で守りの外れ 3,844,998 命令(addps 1.43M、mulps 1.33M、addsd 0.30M、mulss 0.23M …)。process の CPU の 75〜87% がそこだった。診断として RC を 0 に落とすと 207 ms → 16.6 ms。wasm は動的な丸めモードを持たない(f32/f64 の演算は常に最近接偶数)。

### 1.2 方式(表は `tables/design-methods.tsv`)

窓の内側(両オペランドがゼロか、指数が限界から十分に離れた正規数)では、結果が 0 か正規数なので IE・DE・ZE・OE・UE は起こらず、FTZ・DAZ は無関係で、旗は PE だけが変わり得る。守り(PE が立っていて PM がマスク)のとき PE も変わらない。したがって**ホストの最近接偶数の結果 s と、誤差項 e(= 厳密値 − s、誤差なし変換または厳密な余り)の符号**だけで RD/RU/RZ の結果が決まる: 厳密値は s の隣り合う 2 つの浮動小数の間にあり(丸め前の s が最近接なので)、

- RZ(0 方向): `e` と `s` の符号が逆(= 最近接が 0 から遠い側へ丸めた)なら、s のビット列から 1 を引く(符号付き絶対値表現なので、-1 が 0 に近づく向き)。
- RD(下): `e < 0` なら下へ 1 ulp(s が正ならビット −1、負ならビット +1)。RU(上)は `e > 0` で上へ。
- 例外: 正確なゼロ和は −0 になる(RD で、符号の逆な 2 オペランドの和だけ)。積・商・平方根のゼロの符号はホストの結果のまま。

誤差項は命令ごとに次の式(すべて窓の内側で厳密。ホストは最近接偶数、FMA 融合なし)。

| 命令 | 誤差項(sign(厳密 − s)) |
|---|---|
| add / sub(f32・f64) | Knuth の TwoSum: `s=a+b, v=s-a, e=(a-(s-v))+(b-v)` |
| mul f64 | Dekker/Veltkamp の TwoProduct(分割定数 2^27+1)。部分積が非正規になると誤差項が壊れる(`exp(a)+exp(b) ≥ -970` が要る)ので、**窓を 2^-400..2^400 に絞った**(add/sub は従来の 2^±510) |
| mul f32 | double の積(24+24=48 ビットで厳密)、`e = d - (double)(float)d` |
| div f64 | `q=a/b`、余り `r=(a-p)-e`(`(p,e)=TwoProduct(q,b)`。余りは正確に表現できる)、符号は `sign(r)·sign(b)` |
| div f32 | double で余り `r=(double)a-(double)q·(double)b`(厳密)、符号は `sign(r)·sign(b)` |
| sqrt | 余り `r=b-q²`(f64 は TwoProduct、f32 は double)、符号は `sign(r)` |
| cvtsd2ss cvtpd2ps | `s=(float)x, e=x-(double)s`(厳密) |
| cvtsi2ss / cvtsi2sd64(\|i\|>2^24 / 2^53) | `s=(float)i`、`err=i-(int64)s`(整数、\|i\|<2^62) |
| cvt(s\|d\|ps\|pd)2si・cvtps2dq・cvtpd2dq(RC に従う丸め) | 切り捨て `t=(int64)x` と比較で `floor=t-(x<t)`、`ceil=t+(x>t)` |
| cvtt*(切り捨て固定) | **値は変えない**。結果が RC に依らないので、守りから RC を外すだけ(窓の下限も正規数全体に) |
| ucomis* comis* | 値は変えない。旗が RC に依らないので、`fle_mx_comi` の守りから RC を外すだけ |
| min max cmp | 既に RC 非依存(決定 0154) |

packed(`ps` 4 レーン、`pd` 2 レーン)はclang のベクトル拡張(`fle_v2d`/`fle_v4f`。x86 では SSE2、wasm の `-msimd128` なしではレーンごと)で同じ式を一度に。`mulps/divps/sqrtps` は double 半分ずつ(`__builtin_convertvector`)。

### 1.3 守りと窓(RC の他の条件)

- 守り: `(MXCSR & 0x1020) == 0x1020`(PE が立ち、PM がマスク)かつ `RC ≠ 0`。決定 0154 の守り『RC=0・PE・PM』から RC の条件だけを外した(RC=0 の経路は従来のまま。RC≠0 の経路が表から呼ばれる)。FTZ・DAZ・他のマスクは守りに入れない(窓が非正規と tiny を除くので結果にも旗にも効かない。`-DFLE_MX_FAST_CTL=0xE040u` の厳格な守りでは RC 経路も FTZ・DAZ を尊重する)。
- 窓: add/sub/mul(f32)/div(f32)/sqrt(f32)は決定 0154 と同じ(f32 は 0 か 2^-62..2^62、f64 add/sub は 0 か 2^-510..2^510)。f64 の mul/div/sqrt は 0 か 2^-400..2^400。変換は決定 0154 の窓(cvtsd2ss は 0 か 2^-100..2^101 など)、丸め変換は正規数全体で \|x\|<2^31−1(32 ビット出力)・2^51(64 ビット出力)。窓の外・守りの外は決定 0143 のコードに**そのまま**落ちる(`fle_mx_do2_slow_id_h`。関数ポインタで渡す末尾呼び出し)。
- **MXCSR の 1,024 状態の確認**(§3): RC(4)× FTZ × DAZ × 5 つのマスク(IM DM ZM OM UM)× PM × PE の既立ち、すべてで新と従来の整数実装が同じ結果・同じ abort。ホスト CPU の実命令との比較は『全マスク』の状態(ホストが全マスクで走るため)。

### 1.4 呼び出しの形(呼び出し点のコードは増えない)

呼び出し点は従来どおり `fle_mx_do2_slow_p(id, &d, &s, &r)`(引数は 6 → 4 個に減り、わずかに小さい)を呼ぶ。この関数は数命令のトランポリンになり、`fle_mxrc_tab[RC][id]` を引いて(RC=0 の行は空)関数に飛ぶ。表の各関数は『命令 × RC』ごとに RC が定数で、窓を検査して結果を書く(窓の外なら従来の整数実装を末尾呼び出し)。関数と表は weak 定義でリンク後に 1 つ(wasm で +23 KB)。変換の `*_slow` は、native の RC ビルドでは**各呼び出し点に展開されていた従来の整数実装を翻訳単位ごとの 1 関数にした**(Serum の 1 ユニットで text −5.8%)。wasm では従来どおり展開する(out of line にすると wasm -Oz の lifted 関数が +1.0% になったため。`-DFLE_MXC_OUTLINE` / `-DFLE_MXC_INLINE` で選べる)。

## 2. 設計の選択肢と選んだ理由

(表 `tables/design-methods.tsv`。要点)
- add/sub は TwoSum を選んだ。Fast2Sum は大小の判定が要り、f32 を double に持ち上げる方法は『24 ビットのオペランドが 147 ビット離れると 53 ビットに収まらない』ので厳密でない。
- f64 の mul は FMA が使えない(wasm の標準に決定的な fma がない。libm の `fma()` は呼び出し)ので Dekker。整数(`ma·mb` の下位 64 ビットと `ms<<sh` の差の符号)は同程度の命令数で、ベクトル化できず指数の場合分けが要る。
- f32 の mul/div/sqrt は double の積・余りが厳密なので、最も軽い。
- 表引き+RC ごとの関数にしたのは、RC を定数にして 1 ulp のステップを 1 つの式にするため(RZ は『符号が逆なら −1』の 3 命令)。呼び出し点への展開(`FLE_MXR_DIRECT`)は、トランポリンの約 10 命令を減らす代わりに呼び出し点ごとに約 15 命令増える(opt-in)。

## 3. 一致(核): ホスト CPU の実命令との差分試験

道具: `lift/harness/fp1/mxfp_diff.c`(決定 0143・0154 の差分試験)に副コマンドを足した。ホスト CPU の実命令は `crates/fe-lift/tests/ref/fle_mxcsr_flags_host.h`(`ldmxcsr` で RC・FTZ・DAZ を設定し、全例外をマスクして実行、`stmxcsr` で旗を読む)。比べるのは結果の XMM 128 ビット・汎用レジスタ・MXCSR 全体・COMIS の旗 6 つ。

| 試験 | 内容 | 結果 |
|---|---|---|
| `rc`(核) | 全演算 × RC=1,2,3 × FTZ/DAZ 4 通り = 12 組。MXCSR は全マスク、PE は 90% で既立ち、旗は 1/8 でランダム。オペランドは窓の縁・端の値・丸め境界・2^24/2^53 超・非正規・Inf・NaN を含む(既存の生成器)。ホスト CPU の実命令と比較 | 46 演算 × RC 3 × FTZ/DAZ 4 = 12 組、合計 **25,200,000,000 件、不一致 0**(`tables/difftest-native-rc.tsv`、組ごとの件数は `logs/difftest/rc-*.txt`、`logs/difftest3/rc-*.txt`)。**10^8 件/組(= 1.2×10^9 件/演算)の演算: addps・addsd・addss・cvtsd2ss・divsd・mulps・mulsd・mulss・subpd・subsd・subss**。その他は 2.5×10^7 件/組(= 10^8 件/演算・RC)以上 |
| `mxall` | RC(4)× FTZ × DAZ × 5 つのマスク × PM × PE の既立ち = 1,024 状態の MXCSR。新 と 従来の整数実装で『同じ結果・同じ abort』。全マスクの状態ではホスト CPU も | `TOTAL mxall cases 129024000 host_cases 4032000 path_hits 35023751 aborts 48632516 mismatches 0`(各演算 × 1,024 状態 × 3,000 件、新と従来の整数実装で 同じ結果・同じ abort。ホスト CPU は全マスクの状態のみ) |
| `rand`(既存) | 全演算、MXCSR は既定 35%・FTZ/DAZ 20%・RC≠0 16%… の混合、新 / nofast / 従来 / ホスト。ガード違反の計数(ベクトル経路・RC 経路は PE と PM が要る) | `TOTAL rand cases 91800000 mismatches 0 guard_violations 0` |
| `unmasked`(既存) | マスクをランダムにして、旗が立つときの abort が従来と同じ | `TOTAL unmasked cases 26100000 aborts 5550497 mismatches 0` |
| `grid32`(既存) | float の符号 × 指数 256 × 仮数 24 クラスの全組み合わせ(stride 4) | `TOTAL grid32 cases 415358976 mismatches 0` |
| wasm(node、`emcc -O2` と `-Oz`)の `rc` | 同じ試験を wasm にして、従来の整数実装(ホスト CPU の実命令は wasm にない)と比較。`-msimd128` ありも | O2: `TOTAL rc cases 1380000000 mismatches 0 low_coverage 0`、Oz: `TOTAL rc cases 1380000000 mismatches 0 low_coverage 0`(2.5×10^6 件/組 = 10^7 件/演算・RC、従来の整数実装と比較)。`-msimd128` ありの O2: `TOTAL rc cases 1380000000 mismatches 0 low_coverage 0`。最終既定(wasm は変換の slow を展開)の Oz 再試験: 1.656×10^8 件で不一致 0 |
| 建て方の行列 | `-O0 -O2 -Oz -O3`、`-O3 -march=native -ffp-contract=fast`(FMA あり)、`-DFLE_MXV_LANES=1`、`-DFLE_MXV_SLOWBYVAL=1`、`-DFLE_MX_FAST_CTL=0xE040u`(厳格な守り)、`-DFLE_MXR_STATIC`、`-DFLE_MXR_DIRECT`、gcc(packed は整数経路に落ちる)。各 `rc` 全演算 × 12 組 × 2 万件 + `rand` | 不一致 0(`logs/matrix.txt`) |
| Serum 2 のオペランドの再生(`replay128`) | process 1 ブロックで遅い経路に落ちた演算(算術・cvtsd2ss・cvttsd2si64・cvttss2si32・cvtsd2si32)の実オペランドを gdb で 29 回に 1 回採って、記録した MXCSR のまま(と PE 既立ちで)新・従来・ホスト CPU に流す | `TOTAL serum cases 109200 mismatches 0`(54,600 件の実オペランド〔addps 17,716、mulps 16,442、cvtsd2ss 3,975、addsd 3,805、mulss 2,904、mulsd 2,455、addss 1,887、subsd 1,837、cvttsd2si64 1,724、cvttss2si32 865、subss 613、subps 301、subpd 71、divsd 4、divss 1〕を、記録した MXCSR〔0xffe4 = RC=3・FTZ・DAZ・PE|ZE 既立ち〕の旗を消した状態と PE 既立ちの状態の 2 通りで)。**PE 既立ちの状態では 全演算が RC 経路・ベクトル経路に入った(窓の外れ 0)** |
| wasm の補助関数(`rc_helpers`) | `rcgen.rs` の 4 関数を node で、ホスト CPU の実命令(add/sub/mul、f32・f64、RC 3 通り、FTZ/DAZ ランダム)と比較(各 300 万件) | 不一致 0、窓の判定の不一致 0(`tables/wasm-helpers.txt`) |
| 変異検査(`mutate.py`、27 件) | step の向き、ゼロの符号、TwoSum/TwoProduct の項、除数の符号、split 定数、窓の拡大、変換の誤差の符号、f2i の floor/ceil、守りの PE/PM を外す … | `MUTANTS built 27 caught 25`(未検出は窓の縁を変える 2 件:結果が同じで経路だけが変わる等価な変異。ゼロを窓に入れない) |

**RC 経路に入る率**(`rc` の rc_hit。窓の外・守りの外を含む乱数なので、実走行の率ではない): 算術 20〜68%、変換 10〜93%。窓の外のケースは従来の整数実装で同じ結果になる(同じ試験でそれも比べている)。

## 4. 既存のテストと不変の確認

| 項目 | 結果 |
|---|---|
| 全体テスト(`harness/test.sh full --jobs 2`、この木、`FLE_INPUT_ROOT` は主チェックアウト) | `harness/test.sh full --jobs 2`(この木、キャッシュなし)**results=154、passed=2,121、failed=0**(基準 153 / 2,119 / 0〔決定 0164 後〕に、新規の `fe-wasmgen/rc_helpers` 1 results・2 件が加わった)。SKIPPED-lines=15 は作業木に `lift/out-*` の入力が無かったため(基準は 1)。入力を `harness/test-env.sh --link` でリンクし、skip した 10 の実行ファイルと runtime を直接建てる `mxcsr_flags`・`wall_fr18`・`fe-wasmgen` を `--fresh` で再実行: 18 実行ファイル、results=22、passed=209、failed=0、**SKIPPED-lines=0**(`logs/test-full.txt`、`logs/test-linked.txt`)。所要は 708 s(負荷 25〜40) |
| fe-lift の `mxcsr_flags`(ホスト CPU との比較。runtime のヘッダを直接建てる) | 11 件通過 |
| a4 FRUITY 無効(`harness/a4-multi.sh`、5 モジュール、この木の runtime) | init_count **942/942**、run_rank **895/895**、functions **5,310**、entries **1,009,731**、`returned from entry`(`logs/a4-off-rz1.txt`) |
| FLEngine の生成 C の SHA-256(この木で建てた `fe-lift lift FLEngine_x64.dll --pdata`) | `cd9107cef4cbef14f154645749957b7067be7ed7c2b93f5bbe1f72fd47029e50`、215,811,678 B(期待値と同じ) |
| FRUITY 有効の最終構成の生成 C(`gen-final.sh`) | flengine.c `6d39d04f…`、m1 `2c880576…`、m2 `9600514f…`(決定 0154・0160 と同じ) |
| FLEngine の native 16 スナップ(`cmp-short.g3`)の面 SHA・窓表・クロック・進捗 143 行 | **実施した**(`tools/eng_link_run.sh`、`logs/eng-*.txt`、`logs/cmp1.digest`): この決定の runtime(frozen-F = 最終の木と native の既定コードが同じ。差は wasm の変換 slow の既定とコメントだけ)で、FLEngine の FRUITY 有効の最終構成(`gen-final.sh`、SEH 有効)の native -O2 全 92 ユニット(clang、JOBS=2〜3)+ runtime を建て、`cmp-short.g3`(最初の idle → 収束 → Enter → 閉じるアニメ。16 スナップ)を実行した。出力の決定的な行(g3-snap 16・g3-progress・g3-surface・g3-layered・g3-wintable・g3-win・timers)143 行が、fplow1(決定 0154)の旧 runtime の run の digest(`docs/investigations/fplow1/logs/engine/old-cmp-1.digest`)と**1 行も違わず一致**(`cmp` で同一)。面 SHA(idle、収束、Enter 後を含む)・クロック・進捗カウンタが同じ。エンジンの終了コード 134 は旧の run と同じ(g3 スクリプトの最後に harness が abort する)。実行は 6.2 s wall(旧の run は 8.7 s)。基準は『同じ木の -DFLE_MX_NORC の run』ではなく fplow1 の run の digest なので、HEAD の他の決定(0155〜0165)の影響があれば不一致になるはずだが、一致した。 |

## 5. Serum 2 の実 process(核)

**測定の対**: 同じ木(b07659a の runtime + 本パッチ)で、`-DFLE_MX_NORC` の有無だけを変えた 2 つの native -O2 ビルド(vst3a の `build_vst3a_native.sh` を `LIFT=この木` で。生成 C は vst3a の `src/s2g-*.c`、clang -O2、JOBS=1)。旧 = `-DFLE_MX_NORC`(決定 0154 の runtime と同じ守りと窓・変換・比較。違いは `fle_mx_do2_slow_p` の引数が (op, f64, scalar, …) から id 1 つになっただけで、中身は従来の整数実装)。ハーネスは vst3a の `--blocks 40 --note --state N`(保持した C4 1 音、4096 フレーム/ブロック)。交互に走らせ(状態 3 は 5 往復 + 3 往復、他は 2 往復)、`cpu_ms_min`(median)。この機械、load 21〜35(他の担当が稼働)。**測ったビルドは最終の木の一つ前の凍結版**で、最終との差は、コメント、`fle_mxcsr_vec.h` 内の共有ブロックの位置、守りを `FLE_MXV_NP0X` で書き直した箇所(既定の `FLE_MX_FAST_CTL` では同値)、wasm で変換の slow を展開する既定、opt-in の `FLE_MXR_DIRECT` だけ(native の既定のコードは同じ。最終の木でも差分試験・全体テスト・a4 は通した)。

| 状態 | 旧 min(median) ms | 新 min(median) ms | 比(min) | out_hash(新旧同一・vst3a の基準と一致) |
|---|---|---|---|---|
| 3 | 188.4〜201.9(203〜223) | 33.1〜48.4(34〜61) | 5.69 | 9ef6044e59ca83cd |
| 0 | 202.1〜204.7(219〜220) | 39.8〜40.0(42〜44) | 5.08 | 492563b1734d9b80 |
| 1 | 120.0〜123.2(128〜134) | 32.7〜34.8(35〜38) | 3.67 | a8b003644d03494d |
| 4 | 184.7〜188.6(201〜205) | 30.9〜31.2(32〜35) | 5.98 | ccde2b9483f61ce2 |
| 5 | 247.8〜252.9(263〜267) | 67.1〜69.0(74〜74) | 3.69 | 5d70321b39a22988 |

- 状態 3 の和音(`--notes 60,64,67,72`、4 音、40 ブロック、1 往復、`logs/serum-chord4.txt`): 旧 min 741(median 786)→ 新 min 130.5(median 139.8) ms(5.7 倍)、out_hash は新旧とも `617c196bcd9e0af1`(vst3a の基準と一致)。**4 音では新でも 1 ブロック 130 ms で実時間 92.88 ms を超える**(native の 1 スレッド)。
- 状態 3 の新は 2 往復目だけ min 48.4(median 60.7)に外れた。他の 7 往復は min 33.1〜35.2。その時の load は 27 で、同時に別のジョブが走っていた(測定の揺れとして残した)。
- 診断(新ビルドで、ldmxcsr のあとにゲストの RC を 0 に落とす `VST3A_DIAG_GUEST_RC0=1`。出力は忠実でない): 状態 3 で min 14.3 / median 15.9 ms。RC=3 のままの新(33.1〜35.2 ms)はその約 2.3 倍。
- 状態 3 の 1 ブロックは 92.88 ms(4096 フレーム/44.1 kHz)の実時間に対し、新で min 33.1 ms(約 36%)。1 インスタンス。**6 インスタンスなら約 2.2 倍で、native でも 1 スレッドでは足りない**(PM の計算と同じ。ここから先は lift した機械語そのものの速さと並列化)。
- 大きさ: 同じ Serum のユニット `s2g-03.c`(5 MB、`fle_mx_` を 4,230 箇所)の native -O2 の text は旧 1,784,958 B → 新 1,681,657 B(−5.8%)。リンク後の `vst-native` 全体の text は 90,529,603 → 90,600,035 B(+70 KB、+0.08%)。wasm `-Oz` の同じユニットは、補助関数の weak 定義 +23.6 KB が 1 回で、lifted 関数は旧と +37 B(`FLE_MXC_INLINE` 相当の既定。変換の slow を翻訳単位に 1 つにすると wasm では lifted 関数が +1.0% 増えたので、wasm は従来どおり呼び出し点に展開する既定にした)。`-DFLE_MXR_DIRECT` は同じユニットの native text が +10.1%(+169 KB)なので既定に入れない。

## 6. 命令単位の速さ

(表: `tables/bench-native-ns.tsv`、`tables/bench-wasm-*-ns.tsv`、`tables/icount-native.tsv`)

**native -O2(clang)**。ns/命令はループとオペランドのコピー(nop)を差し引いた CPU 時間の最小(3 回)。表は 1 回目(load 27)。2 回目(load 13〜22、`tables/bench-native-ns-b.tsv`)の RC=0 → RC=3 新は addsd 1.67 → 2.73、mulsd 1.74 → 3.24、addps 1.41 → 2.92、mulps 1.24 → 3.65、addpd 1.17 → 2.91、subpd 1.21 → 2.85 ns。回ごとに ±30%。

| 命令 | RC=0 速い経路 | RC=3 新 | RC=1 新 | RC=2 新 | RC=3 旧(整数) |
|---|---:|---:|---:|---:|---:|
| addsd | 1.58 | 2.63 | 3.44 | 2.55 | 19.19 |
| subsd | 1.57 | 2.69 | 3.26 | 2.51 | 20.25 |
| mulsd | 1.61 | 3.08 | 3.30 | 2.97 | 14.57 |
| divsd | 1.49 | 5.20 | 4.17 | 3.84 | 18.93 |
| sqrtsd | 1.28 | 7.87 | 7.91 | 7.58 | 242.87 |
| addss | 3.62 | 3.09 | 3.42 | 2.66 | 19.03 |
| mulss | 3.59 | 2.91 | 3.30 | 3.02 | 14.47 |
| addpd | 1.35 | 3.70 | 3.72 | 2.59 | 37.47 |
| subpd | 1.03 | 3.22 | 3.76 | 2.62 | 37.19 |
| mulpd | 1.15 | 3.79 | 4.07 | 3.41 | 25.68 |
| addps | 1.12 | 3.23 | 3.88 | 2.68 | 72.69 |
| subps | 1.20 | 3.11 | 3.57 | 2.99 | 69.31 |
| mulps | 1.06 | 3.93 | 3.79 | 3.15 | 46.07 |
| cvtsd2ss | 0.86 | 7.24 | 7.11 | 7.29 | 7.93 |
| cvttsd2si32 | 0.65 | 0.66 | 0.66 | 0.43 | 6.17 |
| cvtsd2si32 | 0.81 | 2.08 | 2.13 | 2.10 | 5.17 |

**命令数**(ptrace の単一ステップ。呼び出し点の窓の検査・トランポリン・関数・戻りまで。nop を差し引いた 1 命令あたり。負荷に依らない):

| 命令 | RC=0 速い経路 | RC=3 新 | 比 | RC=1 新 | RC=3 旧(整数) |
|---|---:|---:|---:|---:|---:|
| addsd | 44.3 | 90.3 | 2.04 | 103.3 | 427.0 |
| subsd | 45.1 | 92.1 | 2.04 | 105.1 | 433.9 |
| mulsd | 44.3 | 103.3 | 2.33 | 106.3 | 350.4 |
| divsd | 41.7 | 113.7 | 2.73 | 116.7 | 372.1 |
| addps | 47.3 | 99.3 | 2.10 | — | — |
| mulps | 47.2 | 107.3 | 2.27 | — | — |
| addpd | 45.3 | 99.3 | 2.19 | — | — |

`-DFLE_MXR_DIRECT` のビルド(トランポリンを介さず呼び出し点から表を呼ぶ。`tables/icount-direct.tsv`)の RC=0 → RC=3 は addsd 47.3 → 82.2(1.74 倍)、addps 49.3 → 90.3、mulps 49.3 → 98.2、addpd 47.3 → 90.3 で、既定より約 9% 少ない。

RC=3 / RC=0 の速い経路は、**命令数で 2.0〜2.7 倍**(addsd 2.04、addps 2.10、mulsd 2.33、mulps 2.27、divsd 2.73)。時間では scalar の add/sub/mul が 1.5〜1.9 倍、div 2.2〜3.5 倍、sqrt 5〜6 倍(ホストの `divsd`/`sqrtsd` が 1.3〜1.5 ns と速く、新は 5〜8 ns)、packed 2.1〜3.7 倍(packed の RC=0 は 1 ns 前後と極端に速い)。**目標『2 倍以内』は scalar の add/sub/mul と命令数で達成、packed の時間では未達**。旧(整数)に対しては addsd 19.2 → 2.6 ns(7.3 倍)、mulps 46 → 3.9(11.7 倍)、sqrtsd 243 → 7.9(31 倍)。

**wasm(node 24、V8。`emcc -O2`、`-Oz`、`-O2 -msimd128`)**。RC=0 の速い経路 → RC=3 新 → RC=3 旧(整数)、ns/命令(nop 差引、load 25):

**wasm -O2**

| 命令 | RC=0 速い経路 | RC=3 新 | RC=1 新 | RC=3 旧(整数) |
|---|---:|---:|---:|---:|
| addsd | 1.87 | 5.55 | 6.63 | 40.22 |
| subsd | 1.87 | 7.09 | 5.91 | 42.49 |
| mulsd | 1.80 | 5.97 | 7.03 | 25.94 |
| divsd | 3.63 | 6.66 | 6.85 | 37.00 |
| addpd | 3.47 | 9.51 | 9.03 | 78.28 |
| subpd | 3.26 | 9.63 | 8.96 | 79.91 |
| mulpd | 4.06 | 8.79 | 8.23 | 43.47 |
| addps | 6.13 | 17.71 | 15.04 | 162.57 |
| subps | 5.85 | 14.90 | 16.19 | 154.66 |
| mulps | 5.75 | 15.65 | 14.96 | 82.83 |

**wasm -Oz**

| 命令 | RC=0 速い経路 | RC=3 新 | RC=1 新 | RC=3 旧(整数) |
|---|---:|---:|---:|---:|
| addsd | 1.50 | 6.37 | 6.80 | 44.57 |
| subsd | 1.54 | 5.83 | 6.21 | 40.41 |
| mulsd | 1.48 | 6.61 | 6.53 | 28.72 |
| divsd | 1.57 | 7.24 | 7.52 | 43.04 |
| addpd | 2.93 | 9.10 | 8.88 | 80.58 |
| subpd | 2.48 | 9.02 | 9.09 | 79.08 |
| mulpd | 2.66 | 9.17 | 8.42 | 51.53 |
| addps | 5.91 | 15.25 | 14.52 | 164.81 |
| subps | 6.56 | 16.62 | 14.63 | 160.74 |
| mulps | 5.75 | 19.85 | 13.62 | 99.57 |

**wasm -O2 -msimd128**

| 命令 | RC=0 速い経路 | RC=3 新 | RC=1 新 | RC=3 旧(整数) |
|---|---:|---:|---:|---:|
| addsd | 3.01 | 9.11 | 11.33 | 61.96 |
| subsd | 3.90 | 7.96 | 8.66 | 54.47 |
| mulsd | 3.37 | 8.91 | 8.37 | 38.02 |
| divsd | 1.18 | 9.90 | 9.65 | 44.13 |
| addpd | 1.77 | 6.60 | 6.43 | 75.29 |
| subpd | 1.87 | 7.21 | 8.40 | 79.01 |
| mulpd | 1.98 | 7.75 | 8.68 | 40.42 |
| addps | 2.56 | 5.85 | 8.11 | 153.36 |
| subps | 2.57 | 5.80 | 4.90 | 158.80 |
| mulps | 0.64 | 9.35 | 9.86 | 88.10 |

wasm の RC=3 / RC=0 は scalar の add/sub/mul で約 3〜4.5 倍(div 約 2 倍)、packed(-msimd128 なし)で約 2.2〜3.5 倍。旧との比は addsd で 6〜7 倍、packed 5〜11 倍速い。`-msimd128` ありの packed は RC=0 が 0.6〜2.6 ns と速く、比は大きい(3〜15 倍)が絶対値は 6〜10 ns。

## 7. 新ビルドの process の内訳(サンプリング)と、次に支配する物

`VST3A_SAMPLE`(ハーネス自身の 1 ms ごとの CPU 時間のサンプラー。top frame と、runtime の補助関数は最も近い lifted 関数に帰属)。状態 3、150 ブロック(新 5,792 サンプル ≒ 38.6 ms/ブロック〔負荷下〕、旧 32,774 ≒ 218 ms/ブロック)。

| top frame | 旧 | 新 |
|---|---:|---:|
| lifted 関数の本体(f_*) | 10.0% | 41.4% |
| 整数実装(`fle_mx_pack` 25.0 + `fle_mx_arith` 24.5 + `fle_mx_do2_slow_p` 20.8 + `fle_mx_add` 16.3) | 86.5% | 0 |
| RC 経路 `fle_mxrc_mulps_3` | — | 18.4% |
| `fle_mxrc_addps_3` | — | 15.9% |
| トランポリン `fle_mx_do2_slow_p`(新) | — | 6.1% |
| cvtsd2ss: `fle_mxrc_cvtsd2ss` 3.8 + `fle_mx_cvtsd2ss_slow` 3.0 | (整数) | 6.8% |
| `fle_mxrc_mulss_3` | — | 3.4% |
| その他の `fle_mxrc_*`(subsd_3 1.9、addss_3 1.6、addsd_3 1.3、mulsd_3 1.2 ほか) | — | 7.1% |
| その他の runtime(旧: `fle_mx_comi_slow` 2.1、`fle_mx_f2f` 0.5、`fle_mx_f2i` 0.3 …) | 3.5% | 0.8% |

- **RC の遅い経路は消えた**(旧は runtime の整数実装 90%)。残りの runtime は RC 経路 約 58%(= 約 22 ms/ブロック)と lifted 関数の本体 約 41%(≒ 15.9 ms)。後者は診断(RC=0)の 14.3 ms とほぼ同じ。
- **次に支配する物**: (1) **packed の `mulps/addps` の RC 経路**(34% + トランポリン 6% = 約 15 ms/ブロック)。命令数で RC=0 の 2.1〜2.3 倍まで来ており、さらに削るには窓の検査(約 20 命令)と誤差項の計算(約 25 命令)の削減が要る。呼び出し点への展開(`DIRECT`)は 9% しか減らさず text が +10%。(2) **1 つの lifted 関数 `f_1803d0980`**: 自分の分(top frame)が 21%、呼ぶ RC 経路込みで 61.7%(約 24 ms/ブロック)。Serum 2 の 6 インスタンスの P3 には、この関数の HLE または直接 wasm 生成(packed も)と、インスタンスごとのスレッド化が次の手。(3) dispatch・HLE の項目は上位に現れない(`fle_dispatch` は top 40 に無い)。
- **他の状態のサンプリング**(新ビルド、`logs/profile-new-state*.txt`): 状態 0(3,471 サンプル)は lifted 本体 43.9%・RC 経路 56.1%(`f_1803d0980` 13.0%、`mulps_3` 10.5%、`addps_3` 10.1%、トランポリン 6.4%、`mulsd_3` 5.9%、`mulss_3` 4.6%)。状態 1(2,894 サンプル)は本体 45.5%・RC 経路 54.5%、**scalar が支配**(`mulss_3` 10.9%、`addss_3` 8.1%、`mulsd_3` 7.4%、トランポリン 6.6%、`addsd_3` 3.9%)。状態 4(2,612 サンプル)は本体 39.4%・RC 経路 60.6%、packed が支配(`f_1803d0980` 20.7%、`mulps_3` 19.1%、`addps_3` 16.7%)。**どの状態でも RC 経路は process の 54〜61% を占め、残りの本体は RC=0 の診断とほぼ同じ**。状態 0・3・4 は packed(`f_1803d0980`)、状態 1・5 は scalar が支配。
- **状態 5**(新で 67〜69 ms、診断〔RC=0〕は 41.8 ms): 別の混合。サンプリング(60 ブロック、4,681 サンプル ≒ 78 ms/ブロック〔負荷下〕、`logs/profile-new-state5.txt`)は lifted 関数の本体 44.3%、RC 経路 55.7%。**scalar が支配**: `fle_mxrc_mulss_3` 11.6%、`addss_3` 7.9%、トランポリン 6.6%、`mulsd_3` 5.6%、`subss_3` 4.4%、`addsd_3` 3.9%、packed は `addps_3` 2.7%・`mulps_3` 2.3%、`cvtsd2ss` 2.7%。特定の 1 関数は支配せず(RC 経路込みで最大 f_180362f44 の 9.5%)、状態 3 の f_1803d0980 のような HLE の標的はない。この状態では scalar の RC 経路(トランポリン+窓+TwoSum/Dekker)の命令数を減らすか、呼び出し点への展開(`DIRECT`)が効く。

## 8. wasm 側(直接 wasm 生成部 fe-wasmgen。`rz1-wasmgen.diff`、作れた範囲)

**C 経路の wasm**(本決定の runtime。`emcc -O2`/`-Oz`、`-msimd128` なし〔出荷の構成〕と あり)は、上の差分試験(node)と §6 のマイクロベンチで測った。**直接 wasm 生成部(`mx_form` の窓)は RC≠0 に対応していない**(窓の守りが RC=0・PE・PM で、外れると C 経路の関数 `fle_hyb_mx` を呼ぶ)。したがって RC≠0 のとき、スカラの add/sub/mul/div/sqrt の直接 wasm の関数も、**窓を外れた C 呼び出しの先で本決定の C 経路が速くなる**だけ。packed の形は元々直接 wasm 生成部の対象外(`mx_form` が『MXCSR の窓の規則がない』として関数を C 経路に残す)なので、Serum 2 の支配的な packed `addps/mulps`(RC≠0 の命令の約 70%)は C 経路の本決定の経路で走る。

試作として `crates/fe-wasmgen/src/rcgen.rs`(新規)に、スカラの RC≠0 用の wasm 補助関数 4 つを作った(`rc_addsub64`、`rc_mul64`、`rc_addsub32`、`rc_mul32`。引数 `(a, b, rc)`、結果 `(ビット列, 窓の内か)` の多値)。本体は C の `fle_mxrc_*` と同じ式(TwoSum/Dekker/二重積)を wasm の命令で出したもの。`tests/rc_helpers.rs` が node で実行し、ホスト CPU の実命令と比べる: 各 300 万件(RC 3 通り、FTZ/DAZ ランダム、add と sub、窓の内外)で不一致 0・窓の判定の不一致 0(`tables/wasm-helpers.txt`)。補助関数 1 回の費用(node、wasm のループで `f64.add` 1 命令の同じループと比べる): `f64.add` 1 命令だけのループ 1.0 ns に対し、補助関数(窓の検査込み)の add は RC=1 で 4.0 ns・RC=3 で 3.8 ns、mul は 3.6・3.4 ns(3 回の実行で ±0.05 ns。ループは operand の生成を含むので補助関数の正味は約 2.5〜3 ns。C 経路の wasm -O2 の RC=3 は addsd 5.6 ns・mulsd 6.0 ns〔§6、nop 差引〕で、約 2 倍)。`lower.rs` へは繋いでいない(hyb1 が同じファイルを編集中)。繋ぐ手順:
1. `lib.rs` の補助関数の追加箇所(`mulhu` と同じ所)で `rcgen::add_rc_helpers(&mut module)` を呼び、返った関数インデックスを `Ctx`(または `static` の `OnceLock`)に持たせる。
2. `lower.rs` の `mx_guard_refresh` に、守り『RC≠0 かつ PE が立ち PM がマスク』(`(mx & 0x1020) == 0x1020 && (mx & 0x6000) != 0`)の局所変数 `mx_rcg` と、RC の値の局所変数を足す。
3. `mx_form` の `W::Arith`(addss/addsd/subss/subsd/mulss/mulsd)の『else』側(窓の外か守りの外で `mx_slow` を呼ぶ前)で、`mx_rcg` のとき補助関数を呼ぶ(sub は第 2 オペランドの符号を反転して渡す)。結果が 1 なら `d` の下位レーンを書き、0 なら `mx_slow`。div と sqrt は補助関数を作っていない(C 経路)。
4. 試験: OTT のパイロット(決定 0164)の `--mx-force-slow` と同様に、RC を 3 にした MXCSR で既存のハッシュ(4 シナリオ)が C 経路と一致することを確かめる。
これで呼び出し 1 回あたり、C 呼び出し(`fle_hyb_mx` の往復+本決定の C 経路)から補助関数 1 回(下の費用)になる。**繋いだ版の測定はない**。

## 9. 出典なしの選択(撤回条件つき)

| 名前 | 内容 | 理由 | 撤回条件 |
|---|---|---|---|
| S-rz1-default | 既定で on(`-DFLE_MX_NORC` で戻す) | PM の判断(結果がビット同一な速度だけの変更) | 差分試験の不一致、生成コードの大きさが問題になったとき |
| S-rz1-guard | 守りから RC を外した(PE が立ち PM がマスク)。FTZ・DAZ・他のマスクは見ない | 窓が非正規・tiny を除く。1,024 状態の確認(§3) | 窓が非正規を除かない形に変わったとき |
| S-rz1-win64 | f64 の mul/div/sqrt の窓は 2^-400..2^400(add/sub は 2^±510) | Dekker の部分積が非正規になると誤差項が壊れる(`exp(a)+exp(b) ≥ -970`)。変異 M16 が窓を 2^520 にすると検出される | Serum 2 のオペランドに 2^-510..2^-400 が多いと分かったとき(窓の外れの計数 `fle_mx_rcstat[2]` で見る) |
| S-rz1-table | トランポリン+表+命令×RC ごとの関数 | 呼び出し点のコードを増やさずに、RC を定数にできる | `FLE_MXR_DIRECT`(呼び出し点から表を呼ぶ。コード増) |
| S-rz1-weak | 補助関数と表は weak(ヘッダに定義) | ヘッダだけで建てる利用者(テスト)が変わらない。リンク後は 1 つ | `-DFLE_MXR_STATIC`(翻訳単位ごと) |
| S-rz1-outline | 変換の `*_slow` を native の RC ビルドでは out-of-line に、wasm では従来どおり展開(`-DFLE_MXC_INLINE`/`-DFLE_MXC_OUTLINE`) | RC 試行を呼び出し点に展開しないため。Serum の 1 ユニットで native text −5.8%、wasm -Oz は out-of-line だと lifted 関数 +1.0% | 全体の wasm の大きさを測って out-of-line が小さいとき |
| S-rz1-win-cvt | 丸め変換(cvt(s\|d)2si 等)の RC 経路の窓は正規数全体 \|x\|<2^31−1(32 ビット出力)/ 2^51(64 ビット出力) | 切り捨て・床・天井は微小な正規数でも厳密 | — |
| S-rz1-cvtt | 切り捨て変換は守りから RC を外し、窓の下限を正規数全体に | 結果が RC に依らない。RC=0 の既定の動作にも効く(速くなるだけ) | `-DFLE_MX_NORC` |
| S-rz1-comi | COMIS/UCOMIS の守りから RC を外した | 比較の旗が RC に依らない | `-DFLE_MX_NORC` |
| S-rz1-vec | packed の RC 経路はベクトル拡張と `__builtin_elementwise_sqrt` が要る | clang/emcc で足りる | gcc 対応が要るとき(スカラの 4 レーンで書く) |
| S-rz1-bench | ns/命令はループと operand のコピー(nop)を差し引いた CPU 時間の最小(load 15〜35) | 負荷の高い共有機械。命令数(`icount.c`)を併記 | — |
| S-rz1-replay | Serum 2 のオペランドは gdb で 29 呼び出しに 1 回採った | 全部だと遅い(1 ブロック 380 万) | — |

## 10. PM が決めること

1. **既定 on の最終確認**(PM の条件: ≥10^8 件/組の本走行で mismatch 0〔§3〕、全体テスト・a4 FRUITY 無効・FLEngine の生成 C の SHA-256 不変、16 スナップの面 SHA 不変〔native〕〔§4〕)。**すべて満たした**: 16 スナップ 143 行が基準の digest と完全一致(FLEngine の全 92 ユニットを native -O2 で建て直して実行)。RC=0 の既定の動作が変わるのは『切り捨て変換の窓が広がる』『COMIS の守りから RC が外れる』『呼び出しの形がトランポリン経由になる』の 3 点で、差分試験・a4・16 スナップのいずれも同一。
2. **packed のインライン化(`-DFLE_MXR_DIRECT`)を採るか**。命令数(`icount`)で RC=3/RC=0 は scalar 約 2.0 倍、packed 約 2.1 倍にまで来ており(§6)、残りのコストは窓の検査(約 20 命令)と TwoSum/ステップ(約 15 命令)で、呼び出しの 3 段ではない。DIRECT は 1 ユニット(s2g-03、5 MB)の native -O2 の text が 1,681,657 → 1,851,089 B(+10.1%、+169 KB)で、命令数は約 9% 減(addsd 90.3 → 82.2)。採らないことを勧める(Serum 2 の次に支配する物は §7)。ただし scalar が支配する状態 1・5 ではトランポリンが 6.6% を占めるので、その状態を重視するなら再検討の余地がある。
3. **fe-wasmgen の補助関数を `lower.rs` に繋ぐか**(§8)。hyb1 が `lower.rs` を編集中なので、取り込みの後に別の作業で。Serum 2 の RC≠0 の命令の約 70% は packed で、直接 wasm 生成部の対象外(C 経路)。繋いで効くのはスカラ約 30%(状態 3 の 1 ブロックで addsd 0.30M・mulss 0.23M・mulsd 0.19M・addss 0.15M・subsd 0.15M ほか 約 106 万命令)で、1 命令あたり 約 3 ns(補助関数の正味 約 2.8 ns と C 経路の wasm の約 5.6〜6.0 ns の差)≒ 約 3 ms/ブロック(推定、繋いだ版は未測定)。scalar が支配する状態 1・5 ではもっと効く。
4. **次の P3 の手**: §7 の内訳。


## 11. 限界(確認できなかったこと)

- **実機 Windows との照合は 0 件**。ホスト CPU は AMD Ryzen 9 9950X(Zen 5)。『ホスト CPU の実命令と同じ』は、x86 の SSE が IEEE 754 の丸めモード(MXCSR の RC)に従うことの確認で、ゲストの実機(Intel/AMD の別の世代)の NaN ペイロード等の差は見ていない。窓の内側では NaN が演算に入らない。
- FLEngine の 16 スナップの基準は fplow1(決定 0154)の旧 runtime の run の digest で、同じ木の `-DFLE_MX_NORC` の run との比較ではない(HEAD の他の決定の影響があれば不一致になるはずだが一致した)。FLEngine の wasm 実行・Chrome は未実施。
- **Serum 2 の測定は native のみ**(wasm の Serum 2 は走らせていない)。状態は 0・1・3・4・5(2 は 800 ms 超で除外)、1 インスタンス。6 インスタンスは未測定。vst3a の代役(Direct2D/DirectWrite の失敗応答など)の上。
- 速さはこの機械(16 コア、load 15〜35、他の担当 6〜7 本が稼働)の値で、CPU 時間の最小/中央値。ns/命令は ±30%。命令数(ptrace の単一ステップ)は負荷に依らない。依頼者の機械の値ではない。
- wasm は node(V8)のみ。Chrome・Firefox・Safari は未測定。`-msimd128` ありの wasm は差分試験と micro のみ(Serum は走らせていない)。
- gcc では packed の RC 経路が無効(整数実装に落ちる。結果は同じ)。ARM などの非 x86 ホストは未確認(IEEE 754 最近接偶数の host 演算に依存。x86-64 SSE と wasm で確認)。
- 窓の外の f64 mul/div/sqrt(2^±400 の外、2^±510 の内)は RC≠0 では整数実装のまま(Serum 2 の実オペランド 54,600 件では、PE 既立ちの状態で窓の外れ 0〔§3〕)。
- wasm の補助関数は `lower.rs` に繋いでいない(§8)。
- 例外の unmasked(abort)の差分は native のみ(emscripten の abort はシグナルでない)。

## PM 追記(取り込み時、2026-11-03)

**取り込んだ範囲**: `rz1.diff`(runtime の MXCSR 経路、差分試験、道具。21 ファイル)と `rz1-wasmgen.diff`(fe-wasmgen の新ファイル `rcgen.rs`・`tests/rc_helpers.rs` と `lib.rs` の 1 行)。PM の検証: 両方の差分は HEAD 61aa1a4 に fuzz・offset・reject なしで当たる。適用後の `fe-lift` で FLEngine の生成 C を再生成した SHA-256 は `cd9107ce…`(215,811,678 B)で不変。全体テスト(`test.sh full --jobs 4`、81.7 秒): **155 results・2,131 passed・0 failed**(基準 154/2,129/0 + `fe-wasmgen/rc_helpers` 1 実行ファイル・2 テスト)、SKIPPED 1 行(wall_fr9t の既存)。a4 FRUITY 無効: init_count 942/942、run_rank 895/895、functions 5,310、entries 1,009,731(不変)。

**決めたこと**:
1. **既定 on を承認**。条件(1 組あたり 10^8 件以上の差分試験で不一致 0、全体テスト、a4、生成 C の SHA、16 スナップの面 SHA)は担当が満たし、PM がテストと a4 と SHA を再実測した。巻き戻しは `-DFLE_MX_NORC`(RC≠0 経路と、RC 非依存の 2 変更〔切り捨て変換の守り・COMIS の守り〕をまとめて切る)。
2. **`FLE_MXR_DIRECT` は採らない**(命令数 −9% の代わりに Serum の 1 ユニットで text +10.1%)。scalar が支配する状態 1・5 を重視する場合に再検討。
3. **差分試験の基準はホスト CPU(Ryzen 9 9950X)の実命令**で、実機 Windows の命令との照合は 0 件。丸めと FTZ/DAZ の挙動が Intel と AMD で異なる箇所が無いことは、IEEE 754 と Intel SDM の範囲での読み。別の CPU での差分試験は未実施。
4. **P3 の見方**: Serum 2 の状態 3 は 1 インスタンスで process 1 ブロック 約 34 ms(実時間 92.88 ms の約 36%)。ただし 4 音和音は 130.5 ms で実時間を超え、**native -O2 の値**。wasm の Serum 2 は未測定で、C 経路の wasm は native の 2〜3 倍以上かかると読む(OTT の C 経路 wasm -O2 は native -O2 builtin 9.5 ms に対し 9.92 ms と測っている〔決定 0164〕が、Serum の packed 経路では未測定)。次の測定: Serum 2 の C 経路 wasm を node と Chrome Worker で実測する。
5. **fe-wasmgen の補助関数(`rcgen.rs`)の `lower.rs` への接続**は hyb1(決定 0169)の取り込み後に別作業とする(Serum の RC≠0 命令の約 70% が packed で、直接 wasm 生成の対象外のため効果は限定的〔担当の見積りで約 3 ms/ブロック、未測定〕)。
6. 担当の自己申告(同時プロセス 6〜8、作業ディレクトリ一時 6.3 GB)は、PM の負荷規則への違反として記録する。成果物には影響しない。
