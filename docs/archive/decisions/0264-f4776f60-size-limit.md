# 0264(mt3z、仮): Blend2D の f_4776f60(Mixer ドラッグ 546 ms)が hybrid の大きさの上限で止まる理由の構造調査と、直 wasm にする道の判定

基準: 調査は HEAD 62ca650b + 0252(opt1-f)+ 0254 + 0259 + hyb 一覧 +40 の積(木 `z1`)と、その上に 0261 の rsp-delta・jt-copy を載せた木(`z2`)。新しい差分は **b00932ec に `patch -p1 --dry-run` が offset なしで通る**(`mt3-locshare.diff`、`mt3z-diag.diff`)。この機械(負荷 13〜20)の実測は census(`fe-wasmgen --census`、hybrid の出荷フラグ)と cargo のテストと **node 単体での wasm のコンパイル計測**だけ。**実エンジン(ブラウザ/node の FL)での走行は 0 件、依頼者の機械は 0 件、f_4776f60 の自己時間の変化は 0 件(未測定)。**

## 冒頭の 3 点
1. **測ったこと(事実)**: f_4776f60 は **8,446 命令・822 ブロック・17 本の後退辺(既約)・自然ループ 11**で、**命令の 98.6%(8,324 命令)が最外のループ 1 つ(頭 0x4777106)の中**、その中に 6,966 命令の入れ子のループ…と続く。ピクセル形式ごとの特殊化ループではなく、**倍精度の幾何計算(2,419 個の `mulsd/addsd/subsd/divsd` = 命令の 29%、`call` 132)の大きな 1 ループ**。2 つの 6 エントリの jump table の case 本体は合わせて 370 命令(4.4%)しかない。直 wasm の dry-run は **1,162,969 B・局所 892**。**1.16 MB の 89.8%(1,043,755 B)は上の 2,419 命令**(1 命令 419〜442 B = MXCSR の窓の検査〔SIMD の快速検査 + 精密検査 + 速い演算の 2 コピー + C の遅い経路〕が命令ごとに展開される)、残りの 5,000 余命令で 11%。**局所 892 の 96%(852)は『フラグ定義の保存オペランド』**(426 定義 × 2 局所。全部が関数の最後まで別々の局所を持つ)で、GPR 16・XMM 16・その他 8。上限は決定 0185 の `bytes × locals ≤ 50,000,000`(`MAX_FN_COST`)で、この関数は **1.04 G = 上限の 21 倍**。同じ形が製品全体にある: 上限で C 経路に残る関数は census で **148**(dsp 134・FLEngine 8・Elastique 4・qfc 1・ILTools 1)で、dsp の最大級は 1.3〜1.5 MB・局所 626〜760・命令の 80〜90% が `mulps/addps/mulss/addss`(410〜457 B/命令)。
2. **結論**: 『case ごとの断片に分ける』(b)は**効かない**(case は 4.4%、残りは 1 つのループで 132 の call とレジスタ 32 本が生きる)。『上限を上げる』(a)は**局所を減らさない限り不可能**(node で f_4776f60 の wasm を Liftoff だけで eager にコンパイルすると **954 MB・351 ms**、0185 の 5 関数と同じ形)。**局所の共有(今回の試作、`mt3-locshare.diff`)で局所 892 → 48〜52、node の Liftoff コンパイルは 954 MB → 44 MB、351 → 66 ms**。census で上限に当たる関数は **148 → 6**(dsp 5、f_4776f60 1)になる。ただし f_4776f60 自身は積が 60.4 M で上限の 50 M をまだ 21% 超える。残りを埋めるのが (c)『窓の検査を共有のヘルパにする』: 呼び出し側を『快速検査 + 速い演算 + ヘルパ呼び出し』にした形を実際に出して測ると **1 命令 95 B(428 B から 4.5 倍小さい)、f_4776f60 は 1.16 MB → 0.35 MB(dry-run)、積 17.8 M で上限の下**。**(d) 局所の共有だけは実装・検査まで済み、(c) は大きさの測定までで実装は 0 行**。最も有望な道は『(d) → (c)』の順で、(a) の規則変更なしに f_4776f60 を直 wasm にできる見込み(試作の範囲では (d) の後に積 60.4 M を 75 M へ上げる 1 行でも node の実測は Liftoff 44 MB・TurboFan 1.9 秒/287 MB)。
3. **未確認**: 実エンジンでの (d) の走行(局所の割り当てを全関数が受けるので、直 wasm の全関数の局所番号が変わる=bundle の wasm が変わる。opt1 の E 行〔progress/snap/lid 14 sha〕で確かめる必要がある)。f_4776f60 を直 wasm にしたときの自己時間(546 ms の C 経路に対し、上限の見積り 205〜364 ms の削減はどれも未測定)。node の V8 13(Node 24.19)のコンパイル計測がブラウザ(Chrome 156・Firefox 158)の Liftoff/TurboFan と同じかどうか。(c) の実装と、その遅い経路(C の関数呼び出し)の MXCSR の意味の同一性。

4. **追加(PM の 23:30 の指示)の結果**(10〜13 節): (i) 新たに ok になる 141 関数(+ f_48b2de0)のうち**実行されるのは 10 関数だけ**で、合計でも 18 run の実行命令の 0.5%、UI のプロファイルで ms が分かる 4 つが合計 221 ms(24.56 s の 0.9%)。Song Mode・Mixer UI の改善は見込めない。(ii) 規則 0239 を共有ありで回すと **U の差は +0/−4**: 実行される 9 関数は**すべて startup-big(≥20,000 B で起動で動く)で規則の手順 3 が落とす**(f_48b2de0 が U に無い理由も同じ)。『全部入れる』案は 141 関数で +57.3 MB(収まらない)、実行される 9 つだけなら +3.40 MB(145.8 MB で S1 の線に収まる)。(iii) **共有は既存の直 wasm 部にも効く**: 現行の fle の U を node の Liftoff で eager にコンパイルすると 74 → 40 ms・peak 32 → 7 MB(Elastique 24 → 18 ms・34 → 19 MB)。(iv) 無作為標本 160 関数・3,840 run の共有あり/なしの差分で不一致 0(検査力は弱く、正常復帰は 19%)。(v) f_4776f60 は 18 run で 2.45 G 命令・start で 62.5 M 命令の startup-big で、(d)(c) を入れても規則の手順 3 が落とす=手順 3 の閾値の見直しが必要。

## 1. f_4776f60 の構造(`tables/f_4776f60-structure.txt`、`tools/fnstruct.py`、`tools/fn_dump.rs`〔`mt3z-diag.diff`〕)
fe-decode の閉包(0261 の jt-copy を含む)の 1 関数の命令から基本ブロック・支配木・自然ループを作った。
| 項目 | 値 |
|---|---|
| x86 命令 / ネイティブ B | 8,446 / 38,860 |
| 基本ブロック / 辺 | 822 / 1,177(最大 140 命令、中央値 3) |
| 後退辺 / 既約 | 17 / 0(全部の後退辺の行き先が支配する=可約。lowering は構造化の `do_tree`) |
| 自然ループ | 11。**最外の頭 0x4777106 に 8,324 命令・807 ブロック(98.6%)**、その中の 0x4777270 に 6,966 命令・694 ブロック、0x477b210 に 1,992、0x477d8f2 に 1,343、0x477c2b0 に 970 … |
| `mulsd/addsd/subsd/divsd` | 899 / 735 / 638 / 147 = 2,419(29%)。`sqrtsd` 56、`maxsd/minsd` 64、`sqrtpd` 49 |
| `call` / `ret` | 132 / 1 |
| jump table | 0x477fe11(6 エントリ、case 本体 7・77・88・7・16・1 命令 = 196)と 0x47802d9(6 エントリ、17・56・67・10・20・4 = 174)。**case 本体の合計 370 命令 = 4.4%**。どの case も 1 ブロックを支配するだけで、残りの本体は table の前にも後にも連なる |
| 支配木 | 入口から続く長い 1 本の鎖(0, 1, 2, 10, 11, 12, … が全部 8,300 命令超を支配)。table のブロックは鎖の最後の方(822 ブロック中の 795 と 809 番目) |
**読み**: 『MSVC の switch + インライン展開されたピクセル形式の特殊化』ではない。Blend2D の**パスの幾何(offset/stroke のような倍精度の計算)の主ループ全体**が 1 関数になっている。分割できそうな所は『最外ループの中身』で、入口・出口で GPR 16 + XMM 16 + フラグが生きるので、case 単位の分割(b)は意味を持たない。

## 2. 1.16 MB と局所 892 の由来(`tables/bytes-*.txt`、census の `DIAG` 診断)
診断(`mt3z-diag.diff`、`WASMGEN_DIAG_FN=<va>`): `body()` が各 x86 命令の lowering の前後で `Code::len()` を測り、ニーモニックごとに合計する。
- **f_4776f60**(dry-run 1,162,969 B): `lower_insn` の中 1,157,231 B、外(終端・ブロック構造)5,738 B。**Mulsd 899 命令 384,987 B(428.2 B/命令)、Addsd 735 命令 314,863 B(428.4)、Subsd 638 命令 282,312 B(442.5)、Divsd 147 命令 61,593 B(419.0)で 1,043,755 B = 89.8%**。次は Movsd 1,426 命令 32 KB(22.5)、Mov 1,242 命令 22 KB(17.8)、Sqrtsd 56 命令 13.8 KB(247)、Maxsd/Minsd 14 KB(215)、Sqrtpd 49 命令 6 KB(125)。Comisd(99 命令)は 0 B(フラグ消費側で局所に積むだけ)。
- **局所 892**(i32 264・i64 611・v128 17): GPR 16、XMM 16、**フラグ定義 426 個 × 2 = 852**(`FlagInfo` の `need_save` の定義は、読む命令が別ブロックにあっても、同じブロック内でも、関数の終わりまで自分の 2 局所を持つ)、`mat` 1、`mx_guard`/`mx_rcl` 2、一時 6。
- **同じ形が dsp の最大級にもある**: dsp 0x3d76760(6,882 命令、1,268,374 B、局所 760): Mulps 1,307 命令 596,930 B(456.7 B/命令)+ Addps 1,208 命令 531,756 B(440.2)= 89%、フラグ定義 363 → 726 局所。dsp 0x20a3bd0(11,191 命令、1,489,926 B、局所 626): Mulss 1,602 命令 658 KB(410.8)+ Addss 1,370 命令 559 KB(408.1)= 82%、フラグ定義 294 → 588 局所。FLEngine 0x7cf9b0(12,110 命令、828,118 B、局所 1,039): フラグ定義 502 → 1,004 局所。
- 窓の検査が 1 命令 410〜457 B になる内訳(`mx_form` の Arith): 快速検査(SIMD の `mx_l1v`、約 60 B)、`if`、速い演算(約 20 B)の 1 つ目、`else`、精密検査(`mx_win_chk` ×2、約 72 B)、速い演算の 2 つ目(約 20 B)、C の関数を呼ぶ遅い経路(`mx_slow`)、`mx_rcl` が有効なときの RC の枝(`mx_rc_scalar`)。

## 3. 上限の意味(決定 0185 を読んだ)と、上限を外した場合の費用
- **規則**: `Config::max_fn_cost` = `MAX_FN_COST` = 50,000,000(`fe-wasmgen/src/lib.rs`)。dry-run の本体バイト × 局所数が上限を超えた関数は `over the size limit (hybrid)` で C 経路に残る。bytes だけ・locals だけの上限は `u64::MAX`(無効)。**根拠は V8 の Liftoff の Zone メモリが本体 × 局所にほぼ比例する測定**(0185 §2: 24 関数の中央値 0.81 B/単位、範囲 0.27〜2.52。node の最大 RSS が +1 GB になった原因は直接部の巨大関数 5 個を V8 がコンパイルするときの Zone〔0x7cf9b0: 765 KB × 1,030 局所 = 788 M で Liftoff 1,429 MB・381 ms、TurboFan 852 MB・1,360 ms〕)。0185 の読み: 『Liftoff が基本ブロックごとにローカルの状態を複製するため』(内部は未確認)。
- **今回 node で測った**(`tools/measure.cjs`、Node 24.19.0〔V8 13〕、`--no-wasm-lazy-compilation`、同じ f_4776f60 の wasm を局所の共有なし/あり、1 回ずつ、負荷 13〜20、`peak_malloced_memory` の差):
  | f_4776f60 の wasm(1.27 MB) | Liftoff だけ(`--liftoff-only`)の eager コンパイル | TurboFan だけ(`--no-liftoff`) |
  |---|---|---|
  | 局所の共有なし(局所 892、上限を外して出力) | **351 ms・peak 954 MB**・maxRSS 633 MB | 2,003 ms・372 MB |
  | 局所の共有あり(局所 48) | **66 ms・peak 44 MB**・maxRSS 103 MB | 1,934 ms・287 MB |
  = **局所の共有は Liftoff の爆発(Zone の 954 MB)を 22 分の 1 にする**。TurboFan は本体バイト主体で、局所を減らしても 1.9 秒・287 MB のまま(f_4776f60 は熱いので TurboFan に上がる。バックグラウンドの CPU に 1.9 秒、一時的に peak 287 MB)。
- 上限を外す/上げる費用の見積り: (i) **現状のまま(局所 892)で外す**: 上の 954 MB(Liftoff)= 0185 が禁じた状態。不可。(ii) **局所の共有の後に 50 M → 75 M**(積 60.4〜60.6 M):Liftoff 44 MB・66 ms、TurboFan 1.9 秒・287 MB。(iii) **(c) の後**(積 17.8 M〜20 M): 上限の下で規則の変更は不要、TurboFan は本体 0.35〜0.42 MB なので約 0.5〜0.6 秒と見込む(bytes にほぼ比例する読み、未測定)。Liftoff の first idle への効き(0251 の 0.53 ms/KB の読み)は、共有ありでは 66 ms/1.27 MB = 0.05 ms/KB と**読みより一桁小さい**(node、eager、1 回の測定)。この関数は start では呼ばれず、最初の Mixer ドラッグの最初の呼びでコンパイルされる。
- 上限は『ビルドの 1 箇所の規則』のまま。旗にしない。

## 4. 道の表(行数・直る自己時間・リスク)
| 案 | 規模(行) | f_4776f60 | 製品全体(census、上限で C に残る 148 のうち) | リスク |
|---|---|---|---|---|
| (a) 上限を f_4776f60 のために上げる | 1(`MAX_FN_COST`) | **局所を減らさないと不可**(Liftoff 954 MB・351 ms)。局所の共有 (d) の後なら積 60.4 M を通す 1 行で Liftoff 44 MB・66 ms、TurboFan 1.9 s・287 MB | 同じ規則がほかの巨大関数も通す(上限を 75 M にすると dsp の 5 関数〔68〜71 M〕も通る) | 規則の根拠(Zone ∝ 積)が (d) の後は成り立たない方向に外れる。TurboFan の本体主体の費用が残る |
| (b) case ごとの断片に分ける | — | **無意味**: case は 4.4%、残りは最外の 1 ループ、入口・出口で GPR 16・XMM 16・フラグが生きる。分割は呼び出し規約(ABI)を決めて保存/復元を作ることになる | — | 高(RSP・callee-saved・フラグの受け渡し、132 の call の内側、ループの出入り)。推奨しない |
| (c) 窓の検査を共有のヘルパ(gpu1 の P3 案) | 200〜300 の見込み(新ヘルパを `rcgen.rs` の流儀で、呼び出し側を `mx_form` で置換) | **1.16 MB → 0.35 MB(dry-run、呼び出し側を実際に出した測定)**。(d) と合わせて積 17.8 M(real 20.1 M) | すべての FP 重い関数(dsp 134・FLEngine 8…)の本体が約 1/3〜1/4 になる | 中〜高: 速い経路の演算と MXCSR の意味を共有のヘルパが同じに保つ必要(遅い経路は C の関数 + scratch)。ヘルパ呼び出しの費用が速くない経路に乗る(窓を外れた値) |
| (d) フラグ定義の局所の共有(**実装・検査済み**) | `locshare.rs` 約 190 + `lower.rs` 28 + `lib.rs` 1 + テスト | 局所 892 → 52(dry)/48(real)。積 60.4 M(上限の 1.21 倍)。**Liftoff 954 → 44 MB、351 → 66 ms** | **上限に当たる 148 → 6**(dsp 134 → 5、FLEngine 8 → 0、Elastique 4 → 0、qfc 1 → 0、ILTools 1 → 0、f_4776f60 は積 60.4 M でなお上)。上限に当たらなくなる 127 の dsp 関数(最大 1.4 MB)は、本体が大きいので TurboFan の費用は別に残る | 中: 全関数の局所番号が変わる。割り当ては保守的(ブロックの粒度、同じブロックの 2 定義は共有しない)で、独立した命令単位の検査器で 8 つの DLL の全関数を検査した(下) |
**最も有望なのは (d) → (c)**。(d) は小さく、検査済みで、局所の爆発という『上限の主因の 1 つ』を消す。(c) は f_4776f60 を上限の下へ入れる決め手だが、実装が重く MXCSR の意味の検証が要る。

## 5. 試作: フラグ定義の局所の共有(`mt3-locshare.diff`、`locshare.rs`)
**方式**: `need_save` の定義(または `Src::Def` で読まれる定義)ごとに、その保存オペランドの局所が**必要な範囲**を『読む命令のあるブロックから後ろ向きに閉包を取り、定義のあるブロックで止める(定義が局所を書き直すから)』ブロックの集合とする(定義より前に同じブロックにある読み手は、ループで運ばれる使用として前のブロックまで伸ばす)。範囲が交わらず、局所の型(i32/i64)が同じ定義は同じ 1 組の局所を使う(first-fit)。lowering は従来どおり『定義で書き、読み手で読む』。`def_locals(id)` が局所の組をスロット経由で返すだけの変更(`lower.rs` +28/−2、`lib.rs` +1)。テスト用の静的スイッチ `LOCSHARE_OFF`/`LOCSHARE_VERIFY`(`lower::MX_FORCE_SLOW` の前例と同じ)は製品の旗ではない。
**検査**:
1. **独立した検査器** `locshare::verify`: 割り当て(範囲の閉包)とは別に、**命令を後ろ向きに歩いて**、定義 d の各読み手から d の命令までの全経路に、同じスロットを書く別の定義の命令が無いこと、読み手自身が同じスロットの別の定義の命令でないことを確かめる。`WASMGEN_LOCSHARE_VERIFY=1` で 8 つの DLL の census(全関数の dry-run と本番の lowering)を走らせ、**違反 0**(panic なし。census の全行を dry-run と本番で lowering: dsp 25,370、FLEngine 59,305、Elastique 4,727、qfc 6,712、ILTools 12,930、freetype 1,747、oggio 2,700、Blend2D 732 関数。`tables/census-over-counts.txt`)。
2. **差分テスト** `tests/locshare.rs`(3 件、node): 共有なしと共有ありで同じ関数を lowering して同じ入力で走らせ、期待値(Rust で計算)と一致するか。(i) 比較と分岐を別ブロックに 3 組並べた関数(共有が実際に起きる: 2 つの wasm が異なることを assert)、入力 6×6×3×3 通り(符号付き/符号なしの境界を含む)、(ii) `cmp` の CF を `inc`(CF を書かない)の後の `jb` が読む関数(共有してはいけない形)、(iii) 比較と `inc` のフラグを別ブロックで読むループの和。
3. **変異**(割り当ての壊し方): (M2) 範囲を見ずに型が同じなら共有、(M5) 定義のブロックを範囲に入れない。**どちらも (ii) が落ちる**(検査器なしの node の比較だけでも落ちる=検査器に頼らない確認も済み)。
4. `cargo test -p fe-wasmgen`(0252+0254+0259 の積 + 本差分): **25 結果ok・41 通過・失敗 0・無視 1**(`locshare` の 3 件を含む)。
**census**(`tables/overlimit-before-after.tsv`、`tables/census-over-counts.txt`): 上限に当たる関数が 148 → 6。dsp 134 → 5(残りは 68〜71 M の 1.48 MB の 5 関数: 0x20a3bd0・0x2157ea0・0x220f6d0・0x22c24f0・0x2385030)、FLEngine 8 → 0(0x7cf9b0・0x7e5290・0x7f6f20・0xcbc020・0xf46490・0x1046a30・0x11b38e0・0x1280530)、Elastique 4 → 0、qfc 1 → 0、ILTools 1 → 0。f_4776f60 は 1.04 G → 60.4 M(17 分の 1)。
**リスクと取り込みの注意**: (1) 割り当ては全関数に効くので、**直 wasm の全関数の局所番号が変わる**=bundle の wasm の sha が変わる。実エンジンでの検証(opt1 の E 行〔progress/snap/lid〕)が要る。局所番号の変化は Liftoff/TurboFan の入力を変えるだけで意味は同じ、というのが検査の主張。(2) 範囲はブロックの粒度で、同じブロックに 2 つの定義があると共有しない(保守的)。(3) 定義と読み手が同じ命令(`adc` の運ばれ使用)は共有しない。(4) 共有で上限に当たらなくなった 127 の dsp 関数などは、hyb 一覧 U に入れるかは PM/opt1 の判断で、本体が大きい(最大 1.4 MB)ので TurboFan の費用が別に残る(下の (c) で小さくなる)。

## 6. (c) の大きさの測定(実装は 0 行)
`mx_form` の `Arith` の枝を、『快速検査 → 速い演算、そうでなければ `local.get d; local.get sv; local.get mx_guard; local.get mx_rcl; call H; local.set d`』の形に置き換えたコードを**測定専用に**出した(ワークツリー、`WASMGEN_DIAG_SHARE_EST=1`、差分に入れない。H は `call 0` の代用で、wasm は検証に通らない=大きさだけ):
- Mulsd/Addsd/Subsd/Divsd の呼び出し側は **1 命令 95.2/95.4/97.5/93.0 B**(428〜442 B から)。
- f_4776f60: dry-run **349,703 B・局所 51**(積 17.8 M)、本番の lowering 419,135 B・局所 48(積 20.1 M)。(d) なしでは 349,703 × 878 = 307 M で上限の外のまま=**(c) だけでは足りず、(d) と組み合わせる**。
- 実装に要る物: 共有のヘルパ(v128 の d、v128 の sv、guard、rcl を受けて v128 を返す)の中身は、現在の `else` の枝(精密検査 → 速い演算、または `mx_rc_scalar` の RC の枝、または `mx_slow` の C の関数呼び出し)そのまま。`rcgen.rs` が RC の枝のヘルパを共有で作る前例がある。同じ形が packed(`mx_packed`)、scalar float にも要る。

## 7. 副次: Mixer ドラッグの Blend2D の C 経路の関数で今 ok にできる物
Mixer ドラッグ(e)の Blend2D の『RSP の 17 関数』は 754 ms(8.8%)で、f_4776f60(546 ms)+ f_4758e40(201 ms)で約 747 ms、残りの 15 関数は合計約 7 ms(`docs/investigations/gpu2/tables/c-path-by-reason.txt`)。同じ画面の Blend2D のほかの大きい C 経路の関数(prof-e の上位 30)を、積(opt1-f + 0252 + 0254 + 0259)の census の状態に当てた:
| 関数 | e(ミキサードラッグ) ms | 積(rsp-delta なし) | rsp-delta + jt-copy を足す | hyb 一覧 U | 見積り |
|---|---|---|---|---|---|
| f_4776f60 | 546(b 320、d 104) | RSP | 上限(積 1.04 G) | なし | (d) で 60.4 M、(d)+(c) で 17.8〜20 M → 直 wasm 化。自己時間の削減は未測定(上限の見積り 205〜364 ms) |
| f_4758e40 | 201(b 23、d 27) | RSP(`__chkstk`) | **ok(7,848 B)** | なし | 0261 の rsp-delta が要る。37.5% の短縮で約 75 ms(未測定) |
| f_48b2de0 | 55(d 14) | **ok(62,855 B。opt1 の積で `Cvtpd2dq` は lowering 済み)** | ok(61,427 B) | **なし** | 一覧に足すだけ。短縮で約 20 ms(未測定) |
| f_476e000 | 89(d 13) | atomic RMW(`fle_cas/xchg/xadd`)で C 経路 | 同じ | なし | 直す物ではない(lowering の方針) |
| f_4776030 | 56(d 13) | atomic RMW で C 経路 | 同じ | なし | 同上(f_4776f60 を呼ぶ) |
これらを hyb 一覧に入れたときの見積り: f_4758e40 7.8 KB + f_48b2de0 61 KB の直 wasm で、first idle への Liftoff の費用は 0251 の 0.53 ms/KB の読みで +0.04 s、f_4776f60(real 419 KB〔(c) の後〕〜1.27 MB〔(d) のみ〕)は start で呼ばれないので first idle に乗らない(最初のドラッグで初めてコンパイルされる)。

## 8. 差分と取り込みの順序(基準 b00932ec。いずれも `patch -p1 --dry-run` が offset なしで通る)
- `mt3-locshare.diff`(570 行: `locshare.rs` 新規 + `tests/locshare.rs` 新規 + `lower.rs` +28/−2 + `lib.rs` +1)。**単独で取り込める**(rsp-delta・jt-copy に依存しない)。
- `mt3z-diag.diff`(95 行): census の診断(`WASMGEN_DIAG_FN`)と `examples/fn_dump.rs`。製品の差分ではない(道具)。
- 0261 の `mt3-rsp-delta-on-e4.diff` は b00932ec に offset なしで通ることを確認した(f_4758e40 のために必要なら)。`mt3-jt-copy.diff` も同じ。
- 環境変数の診断(`WASMGEN_LOCSHARE_VERIFY` 等)はワークツリーだけにある。製品の差分は静的スイッチのテスト用口だけ。
- 適用の順序: (d) →(e 行での確認)→ (c)(未実装)→ 必要なら rsp-delta・jt-copy。

## 9. 未確認・限界(正直に)
- 実エンジンでの走行 0 件。(d) の局所番号の変更が実エンジンで意味を変えないことは、検査器・差分テスト・census の全関数の検査〔8 モジュール〕が支えるだけで、実機の画面・音では未確認。
- node の計測は 1 回ずつ、負荷 13〜20 の共有機械、node 24.19(V8 13)。ブラウザ(Chrome 156、Firefox 158)の Liftoff/TurboFan は未測定。
- 『1.16 MB の窓の検査』の内訳(約 60 + 72 + 20×2 + 遅い経路 + RC の枝)の各部は `Code::len()` を部品ごとに測ったのではなく、コードから数えた概算。確かな数は『1 命令 428 B』と『呼び出し側を共有にすると 95 B』の 2 つ(実際に出して測った)。
- (c) の実装・MXCSR の意味の検査は 0 行。f_4776f60 の自己時間の削減の見積り(205〜364 ms)は、C 経路 546 ms のうち own 242 ms を 37.5% 短縮、helpers+emulation 304 ms を 0〜90% 短縮するという仮定で、**未測定**。
- この作業の途中で `lift/out-ci1/c6` の DLL の抽出が消えた(別の担当が片付けた)。`out-ci1/c7` の同名の DLL を `lift/out-mt3y/dll` にコピーして続けた(Blend2D は sha1 が `out-deferred/deferred-cfg/dll` のものと同じ)。c6 の census の数字と c7 でやり直した数字が同じ(f_4776f60 は 1,162,969 B・892 局所)ことは確かめた。他のモジュールの census は c6 の DLL で取った数字(c7 でやり直していない)。
- 自己申告(0264): 存在しない `bash` ツールを 2 回目に呼んだ(echo のみ)。それ以外の禁止事項の違反はない(sleep・末尾 &・pkill/pgrep -f なし)。

## 10. PM の追加指示(23:30)への回答: 新たに ok になる関数のうち実行される物と、規則 0239 の hyb リストの差
入力は、決定 0231 の census(`docs/investigations/hyb6/tables/E-all.tsv.gz` = 18 run で実行された 15,412 関数の命令数)と、`tools/0252/regen-hyb-u.py`(規則 0239 そのまま)。fe-wasmgen の report は、局所の共有なしの積(`ctz`)と共有ありの積(`ctz3` = 0252+0254+0259+hyb +40 + 本差分)の 2 通りを、実行された 15,412 関数すべてを `--hybrid-only` にして取った(`tools/runcs.sh`、`run-census-status.sh` のフラグ)。
1. **新たに ok になる関数 141(census 全体)のうち、18 run で実行された物は 9 だけ**(dsp 0x2120cb0、fle 0x1046a30・0x7cf9b0・0x7f6f20・0x7e5290・0x1280530・0xf46490・0x11b38e0、qfc 0x1e22e50。残りの 132 は 18 run のどれでも実行されない)。f_48b2de0(Blend2D、積で既に ok)を足した 10 関数の表が `tables/newly-ok-executed.tsv`(18 run の合計命令数、起動〔start + par-boot〕、Song Mode〔pl・pl3240・ref1-perf・par-ui-pl〕、Mixer UI〔par-ui-mx・par-ui-mxd〕、その他の UI の命令数、UI の V8 プロファイル〔gpu5 の `prof-b-ui-top400.md`、24.56 s〕の C 経路の自己 ms、命令数 × 2.66 ns の見積り)。V8 プロファイルに ms があるのは f_7f6f20 73 ms、f_7e5290 70 ms、f_7cf9b0 57 ms、f_48b2de0 21 ms の 4 つ(合計 221 ms = その 24.56 s の 0.9%)。ほかの 6 つはプロファイルの上位 400 に現れず、命令数からの見積り(C 経路 2.66 ns/命令、0185 の steady の C 部 0.79 s ÷ 297 M 命令の読み。未確認)で 18 run 合計 21〜1,795 ms。Song Mode(4 場面の合計)は 0x2120cb0 150 M・0x1046a30 91 M・0x7e5290 53 M … 命令で、見積りでは場面あたり 100 ms 以下(0x2120cb0 は各 run の最初に 1 回 37.5 M 命令を走らせる関数で場面あたり 100 ms、0x1046a30 は 61 ms、残りは 30 ms 未満。0x2120cb0 は P1 側の費用)。**Song Mode(P3)にも Mixer UI(P2)にも、この 10 関数は効かない**(合計でも 18 run の実行命令の 0.5%)。
2. **規則 0239 を locshare 入りで回した結果: 差は +0 / −4**(`tables/regen-base-vs-committed.tsv`、`regen-locshare-vs-committed.tsv`)。理由: 新たに ok になった 9 つの実行された関数は **すべて『本体 ≥ 20,000 B で起動のスクリプトで動く』= 規則の手順 3(startup-big)で除かれる**(260〜1,030 KB。合計 3.40 MB)。−4(dsp 0x2743970、fle 0x50e830・0x6ca920・0x806e10、どれも 38.6 K 命令)は、lowerable な実行命令の分母が 1.7 G 増えて 99.99% の切り口が動き、末尾の 4 関数が外れた人工物。**規則の出力は実質変わらない**。f_48b2de0 が U に無い理由もこれで説明できる(62,393 B・起動で 8 M 命令を実行=startup-big)。
3. 同じ手順で私の再現が、コミット済みの U と合わないこと: 局所の共有なしの積に規則を回すと 4,056 関数(コミット済み 3,853 に対し +210/−7)。fle +198 が主因で、DLL の抽出・フラグの差で opt1 の再現と同一でない(opt1 の入力 `v-tb2d2` の DLL を持たない)。**だから絶対数は参考で、共有なし/ありの差(4,056 → 4,052)だけが同じ入力どうしの比較**。
4. **wasm サイズ(S1 の線 154,885,888 B、E5 142,380,705 B)**: 規則どおりなら −4 関数(約 −14 KB)で収まる。『実行される 9 関数を startup-big を無視して全部入れる』案は +3,398,484 B(raw、-Oz 前)= E5 に足して 145.8 MB(S1 の線まで 9.1 MB 余裕)で収まる。『ok になった 141 関数を全部』は dsp の 127 関数だけで +51.3 MB(raw)、fle 8 が +3.37 MB、Elastique 4・qfc 1・ILTools 1 が +2.6 MB、計約 +57.3 MB で 142.4 + 57.3 = 199.7 MB > 154.9 MB で**収まらない**(しかも 132 関数は実行されない=意味がない)。-Oz 後の関数ごとの大きさは未測定(raw の dry-run/本番の lowering の大きさ)。
5. **startup-big を外すかは別の判断で、測定が要る**: 規則の手順 3 の根拠は first idle の Liftoff の費用(0251: 0.53 ms/KB)。局所の共有があると、node の eager コンパイル(Liftoff だけ)は次のとおり: 上の 8 つの fle 関数(3.37 MB)が **48 ms・peak 40 MB**(0.53 ms/KB の読みでは 1.79 s)、f_4776f60(1.27 MB)が 66 ms。**読みの 1/30〜1/40**(node、eager、1 回、負荷 13〜20)。ブラウザの first idle の実測は未測定。f_4776f60 自身も 18 run で **2.45 G 命令・start で 62.5 M 命令を実行する startup-big**(本体 419 KB〔(c) 後〕〜1.27 MB)で、規則 0239 の手順 3 は、(d)(c) を入れても **f_4776f60 を落とす**。C 経路の start の 62.5 M 命令は、2.66 ns/命令の見積りで約 166 ms、直 wasm 化の Liftoff は 66 ms(node の実測)。= 手順 3 の閾値の見直し(例: 局所の共有の後は 20,000 B → 引き上げ)は E7 の first idle で決める価値がある。

## 11. 既存の直 wasm 部への効き(局所の共有の副作用。node の eager コンパイル)
コミット済みの hyb 一覧 U(積 = 0252 +40 を含む)の直 wasm だけのモジュール(`--hybrid-only` U、同じバイナリで共有あり/なし)を `--no-wasm-lazy-compilation` で node にコンパイルさせた(`tools/measure.cjs`、各 1 回、負荷 13〜20、`tables/ufull-liftoff.txt`)。
| モジュール | wasm | Liftoff だけ: なし → あり | TurboFan だけ: なし → あり |
|---|---|---|---|
| fle U | 6.07 → 6.06 MB | **74 → 40 ms、peak 32 → 7 MB** | 892 → 859 ms、97 → 85 MB |
| Elastique U | 1.96 MB | 24 → 18 ms、peak 34 → 19 MB | 347 → 432 ms(雑音)、152 → 143 MB |
| dsp U | 0.38 MB | 8 → 8 ms、2 → 1 MB | 38 → 55 ms(雑音)、18 → 18 MB |
= **Liftoff の費用は 4〜5 分の 1(メモリ)、時間は 1/2〜2/3**。first idle の Liftoff(起動で動く関数の lazy コンパイル)への効きは**数十 ms の桁の見込み(未測定)**。TurboFan は局所数に依らず本体に比例して変わらない。これは E7 の first idle で確かめる物。

## 12. 無作為標本の差分テスト(`tables/random-sample-differential.tsv`、`tools/driver.py`・`diffrun.cjs`)
census の ok 関数(命令数 25〜4,000)から**各モジュール 20 関数を無作為に**(seed 264)取り、局所の共有あり/なしの 2 つの wasm(同じバイナリ、`WASMGEN_LOCSHARE_OFF`)を作り、**同じ無作為な状態(16 レジスタ 40% 小さい整数/40% 配列の指す値/…、XMM に倍精度、MXCSR 0x1f80 または 0x1fa0、配列の内容)で 24 回ずつ**実行して、結果(正常復帰か trap の種類、import 呼び出しの列と引数、レジスタ・XMM・配列の FNV ハッシュ、MXCSR)が一致するかを調べた。**8 モジュール 160 関数・3,840 run で不一致 0**。ただし**検査力は弱い**: 正常に復帰したのは 720 run(30 関数)だけで、残り 3,120 run は両方の build で同じ trap(範囲外のメモリ参照 約 8 割、`unreachable`)。ホストの x86 との比較は**していない**(共有あり/なしの比較だけ)。ゲストのメモリを DLL の静的データなしのゼロで走らせているので、静的データを読む関数は意味のある経路に入らない。
追加の保証は 5 節の独立した検査器(census の全関数、違反 0)と、3 件の差分テスト・変異の検出。

## 13. 残る f_4776f60: (c) か規則の見直しか
| 案 | f_4776f60 | 費用(測定) | 副作用 | 推奨 |
|---|---|---|---|---|
| 規則 62 M(PM の案) | dry-run の積 60,416,564(real 60.6 M)< 62 M で通る。**余裕 2.6%**(opt1 の次の lowering の変更で外れうる)。75 M なら余裕 24%、通る関数は f_4776f60 と dsp の 5 関数(68〜71 M、18 run で実行されない=無関係)だけ | Liftoff 44 MB・66 ms、TurboFan 1.9 s・287 MB(node の eager、1.27 MB の本体) | 規則の根拠(Zone ∝ 積)は局所の共有の後は弱くなる(Liftoff 954 → 44 MB)。TurboFan の費用は本体に比例して残る。**手順 3(startup-big)が f_4776f60 を落とす**ので、規則を変えるだけでは U に入らない(10 節 5) | 1 行で E7 に載せて**実測**する。余裕のため 75 M を勧める |
| (c) 窓の検査を共有のヘルパ | 積 17.8 M〔real 20.1 M〕で規則の変更が要らない。本体 1.16 MB → 0.35 MB(測定専用の呼び出し側) | TurboFan も約 1/3.3 の本体になる(見込み 0.6 s、未測定) | 実装 200〜300 行(MXCSR の遅い経路の語義を共有のヘルパで同一に保つ)。dsp など FP の重い関数すべての本体が小さくなる=wasm サイズが減る(E5 に効く) | E7 の結果を見てから。実装は別の担当で 1 日の規模 |
| 両方 | 17.8 M、規則変更なし、本体 0.35 MB | | | 最終形 |
**表のまとめ**: P2 の本命(f_4776f60)を最短で試すには『(d) + 規則 75 M + 手順 3 の閾値(または f_4776f60 の例外でなく**閾値の引き上げ**)』を E7 に載せて実測。(c) はその結果で決める。

## 14. 追加の未確認・自己申告(第 2 報分)
- 10 節 1 の ms は V8 プロファイル(gpu5 の 24.56 s の 1 本)にある 4 関数だけ実測。ほかは命令数 × 2.66 ns の見積り(未確認)。-Oz 後の大きさは未測定。
- 11 節・10 節 5 の node の計測は各 1 回、負荷 13〜20、eager コンパイルで、ブラウザの first idle ではない。
- 12 節の差分テストの検査力は弱い(正常復帰 19%)。
- 規則 0239 の再現(10 節 3)が opt1 のコミット済みの U と同一でない(+210/−7)。
- 自己申告: ファイルの置き場 `lift/out-ci1/c6` の DLL が消えたので `out-ci1/c7` から `lift/out-mt3y/dll` にコピーした(PM の指示どおり今後は自分用の抽出を持つ)。`bash` ツールの誤呼び出しは 2 回(echo のみ)。

## 15. E7b: 差分列(PM の 00:08 の指示。`e7/`、基準 HEAD 91cbd9b0 = コードは b00932ec と同じ)
E7a = 0001(locshare、リスト変更なし。opt1 が 23:58 に着手)。E7b = E7a の上に 0002〜0005(必要なら 0005b)。HEAD の `git archive` に 0001 → 0005 を**実際に順に当てて** offset/fuzz なし・rc=0(`e7/series-dry-run.txt`)。0005b は 0005 の上に offset なしで当たる。
| # | ファイル | 行(+/−) | 内容 | `-` 行の説明 |
|---|---|---|---|---|
| 0001 | `0001-mt3-locshare.diff` | +526/−2、4 ファイル | `locshare.rs`(新規)、`lower.rs`(`def_locals` が組のスロットを返す、L に 2 欄)、`lib.rs`(mod 1 行)、テスト `locshare.rs`(新規) | `lower.rs` の `new_local` 2 行(`def_locals` が新しい局所を作る箇所をスロット経由に置換) |
| 0002 | `0002-mt3-rsp-delta.diff` | +340/−18、4 ファイル | `Sig::rsp`(戻り時の RSP − 入口の RSP = k)、k の関数間の不動点、caller の `pending_rsp += k`、tail jmp は callee の k を足す、wrapper は R[4] = 入口 + k、`mov r,imm` の定数追跡、テスト `rsp_delta.rs`(6 件) | `pres::analyze`/`step` の引数に `rsp_of` を足すための旧シグネチャ行(5 行)、`solve` の旧い更新・`sigs` の作成の旧い行(6 行)、`Sig { inr, ins, out }` の旧い作成、`lower.rs` の『ret with a pop count』の検査 6 行(`ret imm16` を k に数えるので不要) |
| 0003 | `0003-mt3-jt-copy.diff` | +43/−5、2 ファイル | fe-decode `jumptab.rs` の `sext_source` に 32 ビットの `mov eD,eS` | doc 3 行(『movsxd だけ』の説明)と、既存 2 テストの期待(`mov eax,edx` を挟む型は拒否のまま、理由の文言が『is not a sign extension』→『are last written by』) |
| 0004 | `0004-fe-wasmgen-max-fn-cost-75M.diff` | +1/−2、1 ファイル | `MAX_FN_COST` 50,000,000 → 75,000,000(ビルドの 1 箇所の定数、旗にしない)と doc | 旧い doc 行と定数 |
| 0005 | `0005-hyb-u-e7b-B150-and-regen-step3b.diff` | +68/−2、6 ファイル | `regen-hyb-u.py` に手順 3b(`--startup-ms`・`--startup-budget-ms B`、既定 off)、`startup-big-ms.tsv`、`hyb-U-{fle,dsp,b2d,elastique,…}.txt` の末尾に **18 関数を追加(何も削らない)** | `regen-hyb-u.py` の usage 行と最後の print 行(手順 3b の表示を足すための置換) |
| 0005b | `0005b-alternative-hyb-u-B1100-with-f_4776f60.diff` | +14/−4、4 ファイル | 0005 の追加ブロックを予算 1100 ms の 32 関数に差し替え(f_4776f60 を含む) | 0005 の追加ブロックの見出し行(4 ファイル) |
**検査**: `cargo test -p fe-wasmgen -p fe-decode` を**各段の木で別々の target で**実行(0001 まで: **41 結果ok・181 通過・失敗 0**、0002 まで: 42・187・0〔`rsp_delta` 6 件〕、0003 まで: 42・188・0〔fe-decode の新しいテスト 1 件〕、0004 まで: 42・**188**・0・無視 1)。0005 は docs とリストだけでコードは同じ。(同じ target を共有すると古い成果物が使われうるのに気づいて、各段を別の target でやり直した。最初の共有 target の 182/188/188 は捨てた。)0001〜0003 の census(8 モジュール、全関数、`tables/e7-census-series-0001-0003.tsv`): ok **101,935 → 103,394(+1,459 = +1,465 −6)**。失われた関数は oggio の 6 だけ(0002 の健全性の修正、16 節)で、ほかの 7 モジュールは 0。0004(75 M)を足すと、Blend2D の ok は 631(+1、f_4776f60 が 1,249,045 B で ok)、dsp の ok は 16,988 → 16,993(+5、68〜71 M の 5 関数で、18 run のどれにも実行されない)で、**上限に当たる関数は 8 モジュールで 0 になる**(`WASMGEN_LOCSHARE_VERIFY` ありで違反 0)。`WASMGEN_LOCSHARE_VERIFY` の独立した検査器は 8 モジュールの全関数で違反 0。
**生成 C(fe-lift、同じ DLL・同じ引数)**: FLEngine は 0001〜0004 の前後で **sha256 が同一**(af31b7854e06…、215,741,681 B)。Blend2D は 4,251,520 → 4,269,181 B(sha 12e7e3325321… → 12adb182dfcf…、f_4776f60 の jump table の case 5 つが C に入る)、Elastique は 64,394,773 → 64,401,065 B(77007849778f… → 0c2e6cd6e7f0…、f_b2bc5b0 の 9 case)。= **変わる C は Blend2D と Elastique の 2 モジュールだけ**で、CI の m2 / m5〜m7 の C のキャッシュの鍵が変わる。FLEngine・dsp・qfc・freetype・ILTools・oggio の C は変わらない(0003 の fe-decode の閉包の命令数・関数数・表の数が 6 モジュールで前後一致、`tables/jt-census-before-after.txt`。fe-lift の実走は FLEngine で確かめた)。
**wasm の増分(S1 の線 154,885,888 B)**: 0001〜0004 は新しい関数をリストに入れないので、リストを変えない限り本番の wasm(142,380,926 B)は局所番号が変わるだけでほぼ同じ大きさ(fle U の直 wasm は 6,070,644 → 6,061,048 B、Elastique U 1,962,025 → 1,960,365 B。共有で局所番号が小さくなり LEB が縮む)。0005 は +18 関数・raw +344,428 B(→ 142.73 MB)、0005b は +32 関数・+2,180,007 B(→ 144.56 MB)。どちらも S1 の線まで 10 MB 以上の余裕。直 wasm には -Oz が掛からず(`ci/stage-wasm.sh` の見出し: 『wasm-opt -Oz of the C part, the direct wasm of fe-wasmgen merged into it』と私は読んだ。未確認)raw で数えた。

## 16. rsp-delta(0002)の tail-jmp k の健全性〔R[4] と k について。wrapper の callee-saved は §21 で訂正〕と、oggio の 6 関数が ok → C に戻るのが正しい理由
lifted C の呼び出し規約は『呼び出し側が `R[4] -= 8` して戻り番地を置き、`f(); R[4] += 8`。`ret` は pop しない(`ret imm16` は `R[4] += imm`)』。関数の C 版が戻るときの R[4] は入口の値 + k(k = 戻り時の RSP − 入口の RSP)で、`f_<va>` の wrapper は『C 側の R[4]』をその値にして返さなければならない。**HEAD の fe-wasmgen は `jmp g`(tail)で g の k を足さず、親の出口の RSP を『jmp の時点の相対値』だけで見ていた**。そのため『相対値 0 で共有エピローグ g(`add rsp,N; pop…; ret`、k = N)へ jmp する』道は 0 とみなされ、同じ関数の別の出口(`jmp rcx` の TailIndirect、相対値 0)と『一致して釣り合っている』と判定され、wrapper は R[4] を入口の値に戻した。実際にはその道では g が RSP を N だけ進める(C 版は入口 + N で戻る)ので、wrapper が書く R[4] は C 版より N だけ低く、呼び出し側の以後のスタック参照が N ずれる。oggio の 6 関数(0xc01a69a・0xc01a6b4・0xc01a707・0xc01a721・0xc01a738・0xc01a790)は 2 つの出口が 0 と 264(g の k)で食い違い、**どの 1 つの R[4] も両方の道に対して正しくない**ので、HEAD の『ok』は不健全だった。0002 は tail jmp に g の k を足し、出口が食い違う関数を unbalanced にして C 経路に残す(C 経路は lifted C そのもので、規約を 1 つも間違えない)。k の関数間の不動点は callee の k を呼び出し側の追跡と `pending_rsp` にも足すので、同じ誤りが call の側にもない。6 関数は `hyb-U-oggio.txt` に無く、本番では元々 C 経路で走っている(HEAD の census の ok が不健全だったのであって、出荷された挙動は変わらない)。その道が実行時に踏まれたかは未確認。テスト: 共有エピローグの組(P → E)、`ret 16` の callee の caller の RSP と呼び出し後の slot の番地、変数・食い違いは C 経路、`__chkstk` の型、`ret` と `jmp E` が混在する型(`tests/rsp_delta.rs` 6 件、変異 6 つを検出)。
**訂正(2026-11-06、mt3)**: 上の『同じ誤りが call の側にもない』『C 経路は lifted C そのもので、規約を 1 つも間違えない』は **R[4] の値についてだけ正しい**。C 経路の親から `jmp` で入る k ≠ 0 の継続入口(共有エピローグ)を直 wasm にすると、`f_<va>` の wrapper が callee-saved レジスタ(rbx・rbp・rsi・rdi・r12〜r15)を『入口の値で戻す』ため、**継続入口が pop して復元した値を消す**(合成テストで E の後の rbx が 0x1111、期待 0x2222)。0002 のテストはこれを見ていなかった。E7b(0001〜0005)の実エンジン確認 x8 が起動直後に止まった原因はこれと読む(実エンジンでの確認は x13 待ち)。`0002b` で直した。k ≠ 0 の関数が U に何本あり、どこで実行されるかも含め §21 以降。

## 17. startup-big の見直し(PM の 00:08 の追加項目)
**対象**: 規則 0239 の手順 3 は『本体 ≥ 20,000 B で、起動のスクリプト(start・par-boot)で動き、hyb-sse2-F2x でない』関数を落とす。E7 の積(0001〜0004)で regen の手順 1〜3 を実際に回すと、**切り口(99.99%)の中の startup-big は 92 関数・raw 8.29 MB**(`tables/e7-startup-big-ranked.tsv`)。そのうち C 経路 ms が 5 ms 以上の 29 関数を `tables/e7-startup-big-table.tsv`(関数、モジュール、本体バイト〔raw と -Oz 概算〕、start+par-boot の命令数、ミキサースレッドの ms、Song Mode 再生中の UI スレッドの ms、場面 b 全体の ms、Mixer ドラッグの ms、node の Liftoff ms と Zone MB、first idle の見積り、S1 の増分)に並べた。
**スレッドの区別(訂正)**: 場面 b の ms は gpu2 の w0 全体(`b ms`)で、ミキサースレッド(P3 の音)と UI スレッドが混ざる。**ミキサースレッドで最大なのは dsp f_310fb40(自己 232 + 補助 528 = 760 ms、gpu3 の prof-b-mixer-thread)**。fle **f_e95750 は UI スレッド側**(gpu5 の ui-charge-b-HEAD: Song Mode 再生中の UI スレット busy 17,120 ms のうち 2,208 ms〔自己 952 + 補助 1,256〕= UI の C 経路 3,512 ms の 63%)で、P3 でなく UI(P2 寄り)の費用。f_4776f60 は Mixer ドラッグ(e)の費用。
**node の Liftoff(1 関数ずつ、V8 の `--trace-wasm-compilation-times`、eager、1 回)**: f_e95750 2 ms/Zone 2.1 MB、f_310fb40 6 ms/3.9 MB、f_e7e470 4 ms/2.4 MB、f_7f6f20 18 ms/9.6 MB、f_7e5290 40 ms/19 MB、f_7cf9b0 55 ms/27 MB、f_4776f60 73 ms/43 MB。0251 の 0.53 ms/KB の読み(f_e95750 38 ms、f_310fb40 81 ms、f_4776f60 646 ms)の **1/9〜1/20**。ブラウザの first idle は未測定(E7b で測る)。
**1 つの値で選ぶ案**(`tables/e7-startup-big-options.txt`。first idle の見積りは悲観側の 0.53 ms/KB):
| 案 | 値 | 関数 | raw | first idle(0.53 ms/KB) | Song Mode b(whole w0) | Mixer ドラッグ e | f_e95750 / f_310fb40 / f_4776f60 |
|---|---|---|---|---|---|---|---|
| (1) 本体バイトの閾値 T | 160,000 B | 82 | 3.59 MB | +1,859 ms | 3,408 ms | 149 ms | 入る / 入る / 入らない |
| (1) | 1,300,000 B | 92 | 8.29 MB | +4,291 ms | 3,988 ms | 738 ms | 全部入る |
| **(2) first idle の予算 B(C 経路 ms〔b+e〕/ first idle の大きい順)** | **150 ms** | **4** | **0.29 MB** | **+149 ms** | **2,705 ms** | 0 | **入る / 入る / 入らない** |
| (2) | 300 ms | 12 | 0.57 MB | +295 ms | 3,082 ms | 67 ms | 入る / 入る / 入らない |
| (2) | 1100 ms | 18 | 2.12 MB | +1,099 ms | 3,651 ms | 624 ms | 全部入る |
| (3) 比 ρ(C 経路 ms ≥ ρ × first idle の見積り) | 3 | 7 | 0.41 MB | +212 ms | 2,935 ms | 11 ms | 入る / 入る / 入らない |
| (3) | 1 | 17 | 2.10 MB | +1,086 ms | 3,642 ms | 624 ms | 全部入る |
(1) の本体バイトの閾値は、効かない小さい関数も大量に通すので不利(T=40,000 で 51 関数・+738 ms が Song Mode 421 ms 分)。(2) と (3) は同じ順位づけ(利得/費用)で、(2) の予算は『first idle をどこまで払うか』という 1 つの値で表せる。**E7b の主: (2) B = 150 ms = 0005**(P1 を悪くしない側:悲観の見積りで +149 ms、node の実測の Liftoff なら 4 関数で合計 11 ms〔Zone の合計 7 MB〕)。f_4776f60 を入れる実験用: (2) B = 1100 ms = 0005b(悲観の見積りで +1.1 s、node の実測で 18 関数の Liftoff の合計は 101 ms〔Zone の合計 64 MB〕)。
**regen への反映**: `regen-hyb-u.py` の手順 3b(0005)。`--startup-budget-ms B` が既定 0 で off。入力の `startup-big-ms.tsv`(35 行)は C 経路 ms の出典つき。関数は名指しされない。リストは『コミット済み U + (規則の再実行 − 共有なしの再実行)の追加分』で、減る分(99.99% の切り口の移動で外れる 24 関数、113 KB)は人工物なので反映しない(追加のみ)。+14(rsp-delta 等で lowerable になった f_4758e40〔18 run で 5.1 G 命令〕、Elastique f_b034096・f_b04a921 ほか)+ 4(再許可: dsp f_310fb40、fle f_e95750・f_de2280・f_e9d8a0)= +18。

## 18. opt1 への E7b の指示書
**ビルド**: E7a(HEAD + 0001)の上に `lift/out-git/mt3z-results/e7/` の 0002 → 0003 → 0004 → 0005 を `patch -p1` で順に当てる(各 offset 0、rc=0)。f_4776f60 の実験用ビルド(E7b-2)は 0005 の後に 0005b も当てる(E7b-1 = 0005 まで)。ファイルの置き換え: `lift/harness/seeds/hyb-U-{fle,dsp,b2d,elastique}.txt`(0005 が末尾に追加するブロックだけ。ほかの hyb-U は変わらない)。fe-decode が変わる(0003)ので Blend2D(m2)と Elastique(m5〜m7)の lift の C を作り直す(FLEngine・dsp・qfc・freetype・ILTools・oggio の C は不変=キャッシュが効く)。直 wasm は全関数の局所番号が変わる(0001)。
**見る物**(E7a / E6 に対して):
1. node の 84 / 55 / 87 行(progress / snap / lid の 14 sha のみが想定内、停止・g3-win・timers・wintable の変化 0)。E7a で見えている物と E7b の差は、リスト(+18 / +32)で直 wasm になる関数が増えた分だけのはず。差が出たら export の sha を関数ごとに照らす。
2. 自己時間(この機械、Chrome 156、場面 b = Song Mode 再生、Mixer ドラッグ): f_4776f60(E7b-2 のみ)・f_4758e40(両方)の C 経路の ms(Mixer ドラッグ 546 / 201 ms)が direct の自己 ms に移るか。dsp f_310fb40(ミキサースレッド 760 ms)・fle f_e95750(Song Mode の UI スレッド 2,208 ms)の ms。**見積り(未測定)**: 37.5% の短縮なら f_310fb40 は約 −285 ms、f_e95750 は約 −830 ms、f_4776f60 は −205〜−364 ms、f_4758e40 は約 −75 ms。
3. first idle(Chrome の median、Firefox。P1 ≤ 10 s、Chrome median 7.3 s が現状): E7a と E7b-1 と E7b-2 の差。悲観の見積り +149 ms(E7b-1)/ +1.1 s(E7b-2)に対して、node の Liftoff の合計の実測は 11 ms / 101 ms。**判定: E7b-1 が E7a に対して first idle +100 ms 以内なら採る。E7b-2 は f_4776f60 の自己時間が明確に下がり(Mixer ドラッグの CPU/移動の減)かつ first idle が +300 ms 以内のときだけ**。悪化したら B を下げる(予算は 1 つの値)。
4. メモリ(RSS、`peak_malloced_memory`): 0185 の問題(巨大関数 5 個で +1 GB)が再発しないこと。locshare の node の実測では f_4776f60 の Liftoff が 954 → 44 MB なので、E7b-2 でも増えないはず。
5. 波及: bundle の wasm のサイズ(本番 142,380,926 B に対し +0.34 MB / +2.18 MB、S1 の線 154,885,888 B)、CI のビルド時間・メモリ peak(約 7.9 GB が 0003 の C の再生成で変わるか)。
**戻し方**: 0005b → 0005 → 0004 → 0003 → 0002 の順に外す。0001 は E7a のまま。

## 19. 追加の未確認・自己申告(最終報告分)
- 実エンジンでの走行は 0 件。E7b は opt1 の仕事。
- startup-big の ms は profile 表からの転記(gpu2/gpu3/gpu5 の別の run)で、0005 の順位づけは gpu2 の `b+e`(whole w0)。ミキサースレッド / UI の分離は欄を分けたが、順位づけには使っていない。
- node の Liftoff の実測は 1 関数ずつ(全体の中の 1 関数)、eager、1 回。ブラウザの first idle は未測定。-Oz 後の関数別の大きさは、f_310fb40 の 1 モジュールで −9.5%(161,917 → 146,521 B)を測っただけ(概算 ×0.905)。直 wasm は C 部の -Oz の後に結合される読み。
- 自己申告: (1) 関数ごとの測定ループに `while read` を 1 回使った(待ちのループではないが規則の文面には反する)。実行の直後に気づき、数分後に `kill` で止めて for ループ + 一括の実行に直した。(2) `bash` ツールの誤呼び出しは合計 2 回。(3) 先の info の『Song Mode の最大は f_e95750』は UI スレッドとミキサースレッドを混同した誤りで、00:23 に訂正を送った。

## PM の決定と取り込み(第 1 回、2026-11-05 23:55、docs のみ。下の第 2 回で積みを採った)(2026-11-06 00:05、docs のみ。差分は製品の木に当てていない)

取り込み: mt3 の最終報告(23:57)の文書・表・道具を `docs/investigations/mt3z/` に、差分 2 本(`mt3-locshare.diff`、`mt3z-diag.diff`)を `docs/investigations/mt3z/patches/` に置いた。`mt3-locshare.diff`(24,380 B、`fe-wasmgen` の `src/lib.rs`・`src/lower.rs`・新規 `src/locshare.rs`・新規 `tests/locshare.rs`)は HEAD(b00932ec 相当)の `git archive` に `patch --batch -p1 --dry-run` が rc=0・offset 0 で通る。`-` 行は `lower.rs` の `new_local` 2 行(`let a = self.new_local(t);`、`let b = self.new_local(t);`)だけで、読んだ。測定は mt3 の census・cargo・node 単体の wasm コンパイル計測・共有あり/なしの無作為標本の差分(実エンジンの走行は 0 件、ホスト x86 との比較は無し)で、PM は再現していない。

判断:
1. **locshare は単独では出荷しない。** 直 wasm の全関数の局所番号を変えるので、bundle の wasm が全面的に変わり、実エンジンの確認(node の 84/55/87 行、参照プロジェクトを開く、Export の sha256、first idle)が要る。単独の利益は『既存の直 wasm の Liftoff のメモリが 4〜5 分の 1(fle U: 32 → 7 MB、74 → 40 ms)』という node 単体の計測だけで、ブラウザの first idle への効きは未測定。
2. **hyb リストは今回の規則のままで変えない。** locshare 入りで規則 0239 を回した差は +0 / −4 で、新たに ok になる実行される 9 関数はすべて startup-big で規則の手順 3 が落とす。−4(fle 3 + dsp 1)は分母が増えて 99.99% の切り口が動いた人工物(計約 −14 KB)で入れる意味が無い。
3. **新たに ok になる 141 関数のうち実行されるのは 10 関数だけ、18 run の実行命令の 0.48%** で、Song Mode(P3)にも Mixer UI(P2)にも効かない。リストへの全追加(+57.3 MB)は S1 の線を超える。
4. **f_4776f60(Mixer ドラッグ 546 ms の C 経路)を直 wasm にするテストを E7 で行う。** 規則の見直し(積の上限 75 M)と startup-big の手順 3 の閾値の引き上げ(f_4776f60 の例外ではなく閾値)が要り、f_4776f60 は start で 62.5 M 命令を走らせる(C 経路で約 166 ms、直 wasm の Liftoff は node で 66 ms)。同時に f_4758e40(Mixer ドラッグ 201 ms、rsp-delta で ok、startup-big ではない)も入る見込み。ただし Blend2D の C 経路は Mixer ドラッグの約 8.8%(0.75 s)なので、直 wasm 化で見込めるのは数 % で、P2(8〜10.5 ms/移動 → 120 fps には少なくとも約 10〜25% の削減が要る)を単独で満たす梃子ではない。E7 の判定は f_4776f60 の自己時間、Mixer ドラッグの 1 移動あたりの CPU、first idle の実測で行う。
5. 未確認: 実エンジンの全て(自己時間・first idle)、ブラウザの Liftoff/TurboFan、-Oz 後の関数別サイズ、V8 プロファイル外の 6 関数の ms、(c)(呼び出し側 95 B/命令の共有窓検査)の実装、手順 3 の閾値の効果。無作為標本の差分は検査力が弱い(正常復帰 720 run、残りは両 build で同じ trap)。


## PM の決定と取り込み(第 2 回、2026-11-06 00:50。E7b の積み)

取り込み: mt3 の最終(00:35)の文書(15〜19 節)・表・道具・差分列 `e7/` を `docs/investigations/mt3z/` に置いた。差分列を HEAD 91cbd9b0 の `git archive` に順に `patch --batch -p1 --dry-run` で確かめ(0001〜0005 すべて rc=0、offset・fuzz なし)、PM の木に当てた。`-` 行を読んだ: `0001`(lower.rs の `new_local` 2 行)、`0002`(`lib.rs` の `Sig` の作成・`solve` の更新・`ret imm16` の検査 18 行、`pres.rs` の `step` / `analyze` の引数)、`0003`(`jumptab.rs` の `sext_source` の注釈 3 行と、`fe-decode/tests/msvc_switch.rs` の既存テスト 2 件の期待〔拒否のまま、理由の文言だけ〕)、`0004`(`MAX_FN_COST` の定数と注釈 2 行)、`0005`(`regen-hyb-u.py` の usage と出力の 2 行)。FL のクラス名・窓名・アドレスの分岐、新しいオプション、旗は無い。hyb 一覧への追加は 18 行(fle 8、dsp 2、b2d 7、Elastique 6〔重なる行を除く〕、何も削らない。規則 0239 の手順 3b = first idle の予算 B=150 ms で選んだ関数)。

PM が測った値(この機械、`lift/out-pmchk-e7/`、HEAD 91cbd9b0 + 0001〜0005):
- cargo build rc=0。FLEngine の既定の生成 C の sha256 **`34a376dbf23a30c62df5ce6d04c07ed9beb5b651dd82d608cf5bcc59af78062a`**(215,740,414 B)不変。`cargo metadata --locked --offline` rc=0。shellcheck と `ci/test-{needs,lifecycle,wrangler-config,b2d-key}.sh`・`test-real-deploy.py` rc=0。`ci/test-hybrid.sh` は件数の期待が古くなるので **PM が直した**(hyb-U の合計 3,853 → **3,871**、elastique 155 → **161**、oggio 46、`MAX_FN_COST` 50,000,000 → **75,000,000**)。
- `test.sh full`: **209 results / 2,373 passed / 1 failed / 1 ignored**(基準 207 に mt3 の新規 2 本 `locshare`・`rsp_delta` が加わった)。**落ちた 1 件は `fe-lift/msvc_switch` の `real_blend2d_site_is_resolved_by_default`**: 決定 0094 / 0109 の pin `(msvc_tables, jumptabs) = (33, 33)` が **(41, 41)** になった。原因は `0003`(`sext_source` が 32 ビットの `mov eD,eS` を『検査した値を保つ段』として認める)で、閉包は広がらず(2009 関数のまま、命令 405,129 → 405,734)、新たに解ける表が 8 つ(Blend2D の 0x4701a97〔f_4701a70〕、0x4722fcd / 0x4723038 / 0x4723086 / 0x4723238 / 0x4723436〔f_4722f50〕、0x4771e23〔f_4771dd0〕、0x477fe11〔f_4776f60〕。いずれも `cmp S,N; ja default; mov eax,S; lea B,[ImageBase]; … jmp` の形で、以前は『RAX is written at … (`Mov`) on the way into the idiom』で拒否され `fle_dispatch(R[1]); return;` に落ちていた)。生成 C は 51,748 B 増え `clang -fsyntax-only` を通る。mt3 の census の『8 モジュールで新たに解ける表は 2 つ』は fe-wasmgen の census(RSP の理由で C 経路に残る関数に限る)の数で、fe-lift の閉包全体の数ではない — 食い違いは数え方の違い。期待は黙って直さず、mt3 の `0006-fe-lift-msvc-switch-pin-33-to-41.diff`(コメントに +8 の内訳を書き、8 サイトの存在を `contains` で確かめる)を当て、そのテストを再走: **25 passed / 0 failed**。これで 209 results がすべて通る(落ちた 1 件が直った)。
- a4(FRUITY 無効): **init_count 942/942、run_rank 895/895、functions_distinct 5,310、function_entries 1,009,737、started_but_not_entered 0**(不変。FLEngine の C は不変なので予想どおり)。

判断:
1. **`0001`〜`0006` を採り、1 回の冷間ビルドで出す。ただし push は opt1 の実エンジン確認(E7b = x8。node 84/55/87 を E6 の行と、参照プロジェクトが開く・30 s 再生、Export の sha256、wasm の大きさ)が済んでから。** `0005b`(f_4776f60 を含む B=1100)は今回入れない。
2. **`0003` を採る理由**: 性能目的ではない(mt3 の推奨も同じ)。lifter の正しさの補完 — MSVC の switch の表で、添字の検査の後に 32 ビットの `mov eax,S` を挟む形を解く。以前は表の行き先へ入る経路が `fle_dispatch`(関数の入口でない行き先では dispatch-miss で止まる)に落ちていた。x86 の命令の意味の実装であって FL の挙動の特別扱いではない。範囲外の添字は決定 0089 の `fle_jumptab_miss`(不変)。FLEngine の C は変えない。Blend2D の 8 サイトが実エンジンで踏まれても出力が変わらないことは、x8 の node 行で確かめる(行が変われば 0003 を外す)。
3. **`0004`(MAX_FN_COST 50M → 75M)** は 1 箇所の定数で、特定の関数のための例外ではない。`0001`(局所の共有)で関数のコスト(コードのバイト × 局所数)が下がるので、この積みの 0005 の一覧では上限に当たる関数は 0。
4. **`0002` の発見**: oggio の 6 関数(0xc01a69a / 6b4 / 707 / 721 / 738 / 790)は HEAD の fe-wasmgen で、tail jmp の callee の k=264 を足さず ok と見ていた(不健全)。0002 で C 経路に戻る。いずれも hyb-U-oggio.txt に無く、本番では元々 C 経路なので出荷される挙動は変わらない。
5. **効果は未測定**: 一覧への +18 は f_310fb40(ミキサースレッドで最大の C 経路の関数、自己 232 + 補助 528 ms)と f_e95750(UI スレッド側 2,208 ms)ほか。見積り(mt3、37.5% 短縮の上限)は f_310fb40 −285 ms、f_e95750 −830 ms で、**実測ではない**。first idle の増分(悲観の見積り +149 ms、node の Liftoff 実測では 11 ms / 7 MB)は E7b の実測で確かめ、E7b-1 は first idle が E7a に対し +100 ms 以内なら採る(mt3 の判定基準)。S1: +344,428 B(raw)で、線 154,885,888 B まで 10 MB 以上の余裕。

## PM の判断(第 3 回、2026-11-06 00:55): `0003`(jt-copy)と `0006` は今回の冷間ビルドから外す

mt3 の回答(00:46、`docs/0264` 20 節): 「新たに解ける表は 2 つ」(census)と「+8」(fe-lift の pin)は閉包が違うだけ — census は製品の閉包(`--pdata` なし)、pin は `pdata_seeds` 全部の閉包。+8 の 8 サイトはどれも同じ copy 型で、ja の fall-through で S≤N、32 ビット mov は零拡張、2 段の表は全エントリが関数内で decode できることを検査済み、閉包は広がらない(2,009 関数のまま)、生成 C は `clang -fsyntax-only` を通る。製品の閉包に入るのは 0x477fe11(f_4776f60)だけ。**`0003` は fe-decode を変える唯一の差分で、性能に直接効かない**(f_4776f60 は `0005b` の実験のときだけ要る)。

判断: **`0003` と、その pin の更新 `0006` は今回入れない**(`fe-decode/src/jumptab.rs`、`fe-decode/tests/msvc_switch.rs`、`fe-lift/tests/msvc_switch.rs` を 91cbd9b0 の版に戻した)。理由は決定 0260 の `ci1-input.diff` と同じ原則 — 観測された不具合を直さず、性能にも効かない変更を、実エンジンでの確認が薄いまま冷間ビルドに足さない。外すと **全モジュールの生成 C が不変**(Blend2D と Elastique の C のキャッシュも効く)で、fe-lift の pin は 33 のまま通る。`0005` の +18 関数は `0003` 無しでも ok(mt3 が `0003` を逆に当てた木で確認、f_4776f60 だけが RSP で ok でなくなり、それは `0005b` のみ)。PM の確認(戻した木): `cargo test -p fe-lift --test msvc_switch` 25 passed / 0 failed、`cargo test -p fe-decode --test msvc_switch` 72 passed / 0 failed。`0003`・`0006` の差分は `docs/investigations/mt3z/patches/e7/` に保管し、f_4776f60 の実験(`0005b`)のときに別の決定として入れる(lifter の正しさの補完としては有効 — 以前は 8 サイトの行き先が `fle_dispatch(R[1])` に落ちていた)。

実エンジンの確認の扱い: opt1 の x8 は `0003` 入り(Blend2D と Elastique の C が違う)なので、出荷する版とは C 部が違う。出荷する版は C 部が本番(層 e の確認済み)と同一で、直 wasm 部は x8 と同じ。差の影響は小さい(x8 の node 行・Export・再生は直 wasm の確認として有効)と読み、**push は x8 の結果が出てから**、出荷した bundle そのものは ci1 がデプロイ後にブラウザで確認する(`0003` 無しの確認ビルド x9 は opt1 に依頼、出荷を待たせない)。


---

# 追記(mt3、2026-11-06 08:23): E7b(x8)の起動停止の原因と `0002b`(§21〜§26)

(以下は mt3 の 0264 続き。基準 HEAD d6926b4d 以降のコード。実エンジンの確認は opt1 の x13。)

## 21. 冒頭の 3 点
1. **測ったこと(事実)**: `f_<va>` の wrapper(C 経路から直 wasm への入口。`lib.rs`)は、呼び出しの前に callee-saved レジスタ(rbx・rbp・rsi・rdi・r12〜r15)を R[] から読み、関数の戻り値を R[] に書いた**後に、読んでおいた入口の値を R[] へ書き戻す**。普通の関数は callee-saved を保存するので、これは『関数の中の stub 呼び出しが R[] に書く一時値から C 側の呼び出し元を守る』だけの働きをする。しかし **k≠0 の継続入口**(別の関数が `jmp` で入る共有エピローグ。`add rsp,N; pop rbx; pop rsi; …; ret`)は、親のフレームから callee-saved を pop して復元するのが仕事で、wrapper はその結果を書いた直後に入口の値(親が使っていた一時値)で**上書きして消す**。HEAD では k≠0 の関数は C 経路に残っていたので表に出ず、0002 が初めて直 wasm にした。node の合成テスト `tests/rsp_boundary.rs`(C 経路の親が `jmp E`、E = `mov rcx,[rsp+8]; add rax,rcx; add rsp,0x20; pop rbx; ret`)で、**修正前は E の後の rbx が入口の値 0x1111、期待は pop した 0x2222**(E2 の `pop rdi; pop rsi; pop r12; pop r13; pop rbp` も同じ)。E7b の hyb-U(0005 まで)で k≠0 の関数はちょうど 7 本(fe-wasmgen の solve の k を全 U で出した): b2d 0x4770ed2(k=184)・0x47748e5(k=344)、dsp 0x275c35d(k=248)、Elastique 0xb034096・0xb0345a2・0xb04a921・0xb04addc(k=120)。18 run の実行: dsp 0x275c35d は 621 回(start 5.4 万・par-boot 31.6 万命令)、b2d 0x4770ed2 は 53 万回、0x47748e5 は 43 万回、Elastique の 4 本は start・par-boot で 0。
2. **結論**: 修正(`0002b`)は wrapper の『入口の値で戻す』対象から、その関数自身が結果(`res`)として変更する callee-saved レジスタを除く 1 行。普通の関数は callee-saved を `res` に持たない(pres が保存と見る)ので挙動は変わらない。テストは修正前に落ち、修正後に通る。`cargo test -p fe-wasmgen`(0001〜0005 + 0002b): 27 結果・51 通過・失敗 0。x8 の起動中の abort は、この壊れた callee-saved を受け取った C 経路(dsp の CPU 判定初期化付近)で起きた、というのが私の仮説(opt1 の JS スタックの f_2001040)。
3. **未確認**: 実エンジンで abort が消えるか(opt1 の x13 = x9 の木 + 新 0002b で確かめる。PM の木の検証: 生成 C の sha 不変、ci テスト rc=0、test.sh full 210 結果・2,377 通過・失敗 0・無視 1、a4 942/942・895/895)。abort の直接の機構(どの検査が `abort()` を呼んだか)は見ていない。0261 の報告で『rsp-delta は健全』と書いた一部が誤りだった(C ↔ 直 wasm の境界で『k≠0 の継続入口が callee-saved を復元する』場合のテストが無く、テストのコメントに『rbx は検査しない』と書いて飛ばしていた)。

## 22. 差分
`e7/0002b-wrapper-keeps-the-registers-a-continuation-pops.diff`(PM が採った差し替え版は 15,806 B・b724cb7e。初版は 201 行、最終は 281 行。`lib.rs` の wrapper の 1 行 + コメント、`tests/rsp_boundary.rs` 新規。`-` は 1 行: `let cs = …` の旧い定義)。現 HEAD に `patch -p1 --dry-run` が通る(lib.rs に offset −13、fuzz なし)。0002 の直後に当てる。

## 23. 修正の追加(0002b の 2 つ目の変更)
最初の修正(wrapper が `res` に入る callee-saved を『入口の値で戻す』対象から除く)だけでは、**モジュールの中の C 経路の関数から `jmp` で入られない継続入口**(`called_from_c` に入らない=dispatch か、モジュールに現れない jump で入られる物)は `res` に callee-saved が入らず、pop した値が R[] に出ない。そこで **k≠0 の関数は `want`(結果として返すレジスタ)に ALL_REGS を入れる**(`rdelta[k] != Some(0)`、`lib.rs` の `out` の計算に 1 項)。k≠0 の関数は常に別のフレームの継続入口で、誰が入っても C の版は pop した値を R[] に残すから。普通の関数(k = 0)は変わらない。

## 24. 監査(PM の 01:12 の依頼 1): wrapper の『入口の値で戻す』『入口で読む』対象
- **入口で読む**: `sg.params()`(= `inr`)のレジスタだけを R[] から読む。callee-saved に限らない。継続入口が親から引き継ぐレジスタ(r8・r11・rbx…)は `inr` に入る。穴なし。
- **戻りで書く**: `res`(= `out`)のレジスタを R[] へ、R[4] は入口 + k。`out = (変更したレジスタ ∩ want) ∪ …`、`want` は直 wasm の caller の需要 + rax(export)+ **C 経路の関数から呼ばれる/jmp される関数は全レジスタ**(`called_from_c`、`call_targets` は call と tail jmp の両方を含む)+(今回)k≠0 の関数は全レジスタ。**戻りで callee-saved を入口の値に戻す対象は、`res` に入らない callee-saved だけ**(今回の修正)。rax・rdx・r8〜r11 などの volatile は『戻りで入口の値に戻す』処理が無い(`res` に入った分だけ書く)ので、同種の穴はない。
- **XMM(xmm6〜15 が callee-saved)**: wrapper は触らない。`XPlan` が、関数が書いた xmm(`own`)を**どの `ret` でも X[] へ書く**ので、継続入口が `movaps xmm6,[r11-0x10]` で復元した値は X[] に出る(b2d 0x4770ed2 の出口がこの形)。入口の値で戻す処理は無い。穴なし。
- **フラグ**: 直 wasm は C の `FL` を持たない。継続入口が親のフラグを読む場合は『reads a flag that is live into the function』で lowering しない(無言の誤りは無い)。
- **直 wasm どうしの tail jmp**(C を経由しない): `Term::Tail(g)` は g の結果をそのまま返し、親の `out` に g が変えるレジスタが入る(親の `pres` は g の `pres` を畳む)。親が push した callee-saved を g が pop して復元する形は 0261 のテスト(P → E: rbx が親の書いた slot から戻る)が通る。0001(locshare)はフラグの局所だけに関わり、レジスタの受け渡しには関係しない。
- **インデックスの出ない経路**: 間接 call/jmp 用のアダプタ(`ia_<va>`)は `!c_mode`(非 hybrid)だけで、製品の hybrid では作られない。C → 直 wasm の入口は `f_<va>` の wrapper だけ。

## 25. テスト(PM の依頼 2)
`tests/rsp_boundary.rs` を 1 本から 4 本にした(いずれも node、修正前に落ち、修正後に通る。k≠0 の 7 本の形に近い合成): (1) 共有エピローグ `add rsp,0x20; pop rbx; ret` と `add rsp,0x30; pop rdi; pop rsi; pop r12; pop r13; pop rbp; ret`(C 経路の親の `jmp` から)、(2) E3: 6 つ pop(r15・r14・r13・r12・rdi・rbx)+ rsi を `lea` で書くが pop しない + rbp は触らない + R[4] が +0x68(= b2d 0x4770ed2 のような `pop` と上書きの混在)、(3) E4: 出口の pop 量が違う継続は C 経路に残る、(4) 親がモジュールに無い継続(dispatch で入る物)が pop した rbx を返す(2 つ目の修正のテスト)。`cargo test -p fe-wasmgen`(0001〜0005 + 0002b): **27 結果・51 通過・失敗 0**。実関数 7 本(`tables/k-nonzero-in-U.tsv`)を host x86 と比較できない理由: どれも別のフレームの継続入口で、親のフレーム・レジスタ・DLL の静的データ(node のテスト環境は DLL を読み込まない)が要るため、関数単体を実行できない。同じ理由で fe-fruity の x87_hybrid/small_ops_hybrid の流儀(1 関数を合成で走らせる)の対象にならない。wrapper の意味(レジスタの受け渡し)だけを合成で調べた。

## 26. 訂正文(PM の依頼 3): 0261 / 0264 の docs の『rsp-delta は健全』(本文側の訂正は `docs-0261-0264-retraction.diff`)
- **0261 §2 の最後の項**『k は solve の中で他の署名と同時に決まり、呼び出し側は callee の署名で RSP を足すだけで、レジスタを書かない』は **RSP の値についてだけ正しい**。wrapper(C → 直 wasm の入口)は callee-saved を『入口の値で戻す』処理を持ち、k≠0 の継続入口が pop した callee-saved を消す。この節の『HEAD の ok が不健全だった』という主張(oggio の 6 関数)は変わらないが、『0002 は健全』と読める記述(0261 §2・§3 と、0264 §16 の最後の段落)は**撤回する**。
- **誤りの原因**: 0261 のテスト `tests/rsp_delta.rs` は『C 経路から呼ばれた k≠0 の関数が戻った後の R[4]』を見たが、**同じ呼び出しで callee-saved レジスタが pop した値になるか**を見ていなかった(テストのコメントに『rbx はここでは検査しない』と書いて飛ばした)。census と cargo のテストは直 wasm どうし・R[4]・rax だけを見た。k≠0 の関数は HEAD では C 経路にあり、wrapper のこの処理を通ったことが無かった(= 0002 が初めて通した)。
- **E7b で表に出た経路**: x8(0003 入り)の停止は、起動で動く dsp 0x275c35d(621 回)と Blend2D の 0x4770ed2・0x47748e5 が、C 経路の親から jmp されて戻るたびに callee-saved を壊したため、と読む(実エンジンでの確認は x13)。

## PM の判断(第 4 回、2026-11-06 08:50): E7b の起動停止は 0002(rsp-delta)の wrapper の誤りだった。`0002b` を足して出す。上の第 2 回の「0002 は健全」の読みは撤回する

事実(実エンジンで観測): opt1 の x8(91cbd9b0 + 0001〜0005、`0003` 入り)と x9(`0003` 無し、`0002b` 無し)は、製品形の bundle でも FS 付きテストビルドでも起動直後に止まる(node は SIGABRT、exit 134、84/55/87 行が 2/2/0 行で終わる。ブラウザは `first non-background pixel … UpdateLayeredWindowIndirect … lid 8` の旧証拠文つきで停止)。JS スタックは C の `abort()`(`--wrap=abort`)で、経路は f_7cf7f0 の直 wasm 本体 → `fle_hyb_dispatch` → f_7ce9b2(C)→ dsp f_2001040(IPP の CPU 判定)付近。E7a(0001 のみ)・E6・E5 は同じ手順で止まらない。x9 でも止まるので `0003` は原因ではない。

原因(mt3 の静的解析と node の合成テスト。§21〜§26): `0002` が、C 経路の親が `jmp` で入る k ≠ 0 の継続入口(共有エピローグ)を初めて直 wasm にした。`f_<va>` の wrapper は callee-saved(rbx・rbp・rsi・rdi・r12〜r15)を「入口の値で戻す」ので、継続入口が親のフレームから pop して復元した値を消す。k ≠ 0 の関数は U に 7 本(b2d 0x4770ed2・0x47748e5、dsp 0x275c35d〔起動中に 621 回〕、Elastique 0xb034096・0xb0345a2・0xb04a921・0xb04addc)。`0002b` は wrapper の「入口の値で戻す」対象から、その関数自身が結果として変えた callee-saved を除き、k ≠ 0 の関数は全レジスタを出力させる(lib.rs、`-` 2 行。新規 `tests/rsp_boundary.rs` 4 件が修正前に落ち、修正後に通る)。**撤回**: 0261 §2・§3 と 0264 §16 の「0002 は健全」は R[4] と呼び出し規約の k についてだけ正しく、C ↔ 直 wasm の境界の callee-saved については誤りだった(本文側に訂正を入れた: `docs-0261-0264-retraction.diff`)。0261 のテストは callee-saved を検査しておらず、PM も第 2 回の確認で test.sh full・a4(どちらも FLEngine の C 経路を通らない)を根拠にしたが、**実エンジンの確認(x8)が出荷前に止めた**。「実エンジンの確認を push の前提にする」という手順(第 2 回の判断 1)が効いた例として記録する。

PM が測った値(この機械、PM の木 = e7b-wip d1697a30 = d6926b4d + 0001 + 0002 + 0002b + 0004 + 0005〔0003・0006 なし〕):
- cargo build rc=0。FLEngine の既定の生成 C の sha256 **`34a376dbf23a30c62df5ce6d04c07ed9beb5b651dd82d608cf5bcc59af78062a`**(215,740,414 B)不変。shellcheck と `ci/test-*.sh`(`test-hybrid` は 3,871 / elastique 161 / oggio 46 / `MAX_FN_COST` 75,000,000)・`test-real-deploy.py` rc=0。
- `test.sh full`: **210 results / 2,377 passed / 0 failed / 1 ignored**(基準 207 に `locshare`・`rsp_delta`・`rsp_boundary` が加わった)。a4: **942/942、895/895、5,310、1,009,737、started_but_not_entered 0**。
- opt1 の x13(d1697a30 の木、x6 からの増分、7 単位を再コンパイル): 製品形 bundle の engine wasm 142,587,221 B(線 154,885,888 B の内、本番 ff5ee429 に対し +206,295 B)。FS 付きテストビルドの node 84 / 55 / 87 行は **E6 の行と差 0 行**(PM が `diff` で確認: `g3-lines-0265-e6-*` と `g3-lines-0265-e7b-fixed-*` が 84/84・55/55・87/87 行で一致)。x8 の abort は消えた。
- 未確認(opt1 が取り終えるまで): 参照プロジェクトを開いて 30 s 再生、Export の sha256(期待 `381cdb5acf891a160e0286aab6b4822d08f661bcfd4f44f4f7184eda00d1f979`)、f_310fb40 / f_e95750 / f_4758e40 の直 wasm 本体の自己時間、書き込み率の交互 A/B。
判断:
1. `0001`・`0002`・`0002b`・`0004`・`0005` を採り、**opt1 の Export・30 s 再生が上の期待どおりなら**、これを 1 回の冷間ビルドで出す(`0003` と `0006` は第 3 回のとおり外す)。出荷する版の C 部は本番と同一(fe-decode 不変)。
2. 効果は未測定のまま(f_310fb40 −285 ms、f_e95750 −830 ms は 37.5% 短縮を当てた上限の見積り)。P3 の達成とは言わない。デプロイ後に低 load で書き込み率・first idle・入力遅延を測る。
3. 同じ種類の穴の再発防止: 直 wasm 化の対象が増える変更(新しい関数の種類を直にする差分)には、C 経路 ↔ 直 wasm の境界の全レジスタ(callee-saved・xmm6〜15・フラグ・R[4])を戻りの後で検査するテストを必須にする(`rsp_boundary.rs` は rsp-delta の分だけ)。次に直 wasm 化の種類を増やす決定(0270 の Elastique ② flags など)の完了条件に入れる。
