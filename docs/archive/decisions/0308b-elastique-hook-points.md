# 0308 付録 B: FL が Elastique に渡す引数を、Wine と wasm の両側で記録できる場所(読み、静的。alpha1)

date 21:30。実行はしていない(gpu1 の測定窓の間の読みの作業)。対象は `FLEngine_x64.dll`(sha256 `b2c64da4…496d`、Wine のインストール先の物 = `binaries/win64` と同一)と `Shared/elastique_x64.dll`(sha256 `8551e922…89c`)。関数名は `f_<VA>`(ImageBase 0x400000)。

## 1. FL → Elastique の経路(実測の静的読み)

1. **Elastique の API の取り込み**: `f_677d10`(Delphi の `ElastiqueAPI` 初期化)が、DLL(ハンドルの全域変数)から `GetProcAddress` 相当の `call 0x4235f0` で 5 つの export を引き、全域変数に保存する: `ElPro_CreateInstance` → `0x15F4860`、`ElProDirect_CreateInstance` → `0x15F4868`、`ElProV3_CreateInstance` → `0x15F4870`、`ElProV3Direct_CreateInstance` → `0x15F4878`、`ElResampleSource_CreateInstance` → `0x15F4880`(名前は UTF-16 で `.text` の 0x677e56〜0x677f1e)。**使われるのは ElPro / ElProV3 / ElResampleSource の 3 つだけ**(`Direct` の 2 つの全域変数を読む命令は FLEngine に 0 件)。失敗時のメッセージ『Elastique DLL couldn't be opened. Please reinstall』は `0xa98e00`。
2. **生成のラッパ**(引数をそのまま渡す薄い関数): `f_677f60` → `[0x15F4860]`(ElPro、v2)、`f_677fb0` → `[0x15F4870]`(ElProV3、v3、末尾に float 引数を 1 つ足す)、`f_678010` → `[0x15F4880]`(ElResampleSource)。シグネチャ(読み): `(ecx = 第 1 整数, edx = 第 2 整数, xmm2 = float, r8, r9d = 品質/モードの旗, [rsp+0x20] = float〔v3 のみ〕)`。zplane の公開 API の `CreateInstance(チャンネル数, サンプルレート, 最大ストレッチ比, 最大ピッチ比, 旗)` に当たる並びと読めるが、**引数の意味は未確認**。
3. **モード → API の選択**: `f_a97780`(呼び元 `f_a97780` が `f_677f60`〔0xa977d1〕と `f_677fb0`〔0xa97805〕の両方を呼ぶ)。入力は `[rsp+0x60]` の『モード番号』(0〜15。16 以上は v3)。`mov dx,1; shl edx,cl; test dx, [0xa9783f]` の 16 ビットのマスクは **0xC03D**(立っているビット 0・2・3・4・5・14・15)で、**そのモード番号は `ElPro_CreateInstance`(v2)、それ以外(1・6〜13 と 16 以上)は `ElProV3_CreateInstance`(v3、末尾の float 引数 = `0xa9783c` の定数 0.1f)**。`r9 = (signed char) byte table[モード番号]`(`0x14E01EA` の 16 バイト = `[−1, 0, 3, 0, −1, −1, 0, 2, 4, 5, 0, 1, 2, 6, 7, 8]`)が品質/旗として渡る。`xmm3`(double)を `cvtsd2ss` で float にして `xmm2` に渡す。結果は `this+0x10`(v2)または `this+8`(v3)に保存し、`this+0x18 = edx` の後 `f_a97730` で初期化。**FLP の mode(−2/−1/5)がこのモード番号のどれに対応するかは未確認**(UI のモード名の表〔『elastique version 3, default mode』など〕の順と読めるが、確かめていない)。
4. **ResampleSource**: `f_67d5a0` が `f_678010` を呼ぶ(`TElastiqueResampleSourceInputProvider`)。
5. 関連: `Auftakt_CreateInstance` の取り込みは `f_c67520`(呼び元 `f_c67730`)、`SolPitchAn_CreateInstance`・`SolPitchAn_CreatePitchMarks` は `f_c69c10`(呼び元 `f_c69d10`・`f_c69d60`)。UI の文字列『Elastique Pro with formant shifting』は `f_f633b0` が参照。モードの名前表(『elastique version 3, default mode』など)は `.data` の 0x146fe8a〜0x147003c 付近。

## 2. 記録に向く場所(安い順)

| # | 場所 | 取れる物 | Wine | wasm |
|---|---|---|---|---|
| 1 | **`f_a97780` の入口**(1 サンプルのストレッチ器の生成ごとに 1 回) | this、第 2 引数 `edx`、`r8d`、`xmm3`(double)、**`[rsp+0x60]` = モード番号**、どの API(v2/v3)に行ったか | `elastique_x64.dll` でなく FLEngine の関数なので DLL の shim では取れない。**winedbg の条件付きブレークポイント(`bp`+ 引数の表示)か、FLEngine の該当位置の 1 命令を `int3`/ログ呼びに書き換えた複製**。clip 数が少ない(ストレッチ付きは 13 ch)ので手動でも足りる | 生成 C の `f_a97780` の入口にログを足す(使い捨ての木で `onat/gen/flengine.c` に 1 行。再コンパイルは 1 つの unit)。または G3 harness の `FLE_FRUITY_STOP_AT` 類で止めてレジスタを読む |
| 2 | **`elastique_x64.dll` の 3 つの export**(`ElPro_CreateInstance`・`ElProV3_CreateInstance`・`ElResampleSource_CreateInstance`)の引数 | チャンネル数・レート・最大比・旗(r9)・v3 の float | **export を転送しつつログを書く shim DLL**(opt1 の cpuid の shim と同じ道具立て。引数 5 個の関数 3 本だけ)。インターフェースの vtable の関数は shim では取れない | wasm では `elastique` モジュールの export の入口にログ(`--hook-points` ではなくログ用の小さな差分) |
| 3 | **生成されたインターフェースの vtable の関数**(ストレッチ比・ピッチ比を設定する呼び、処理の呼び) | 1 ブロックごとの比、入出力のフレーム数 | shim が返すオブジェクトを包んで vtable を差し替える(`CreateInstance` の戻り値を包む。メソッド数と並びの調査が先に要る = 工数大) | 同様に生成 C の側でログ |
| 4 | **Elastique の入力と出力の PCM** | 両側の PCM を直接比べられる | shim でインターフェースを包み、処理の前後の PCM をファイルへ | 同様 |

## 3. 推奨(読み)

1. **まず #1(`f_a97780` の入口)を両側で記録**し、ch 7・35 のストレッチ器の『モード番号』『経路(v2/v3)』『整数・double の引数』を比べる。ここが違えば、残差は『FL の判定(モード→API 選択)が wasm と Wine で違う』ことで説明がつき、Elastique 自体の調査は不要。同じなら #2 の引数を比べ、さらに #3/#4(PCM)へ進む。
2. #1 は『ストレッチ付きのクリップが何個あるか』が少ない(13 チャンネル、うち長いクリップで鳴るのは 5)ので、**手動のブレークポイントで足りる**(工数 0.5 人日以下)。wasm 側は生成 C に 1 行のログ(使い捨ての木、wasm の試験 build 1 回 約 8〜10 分)。
3. (b)(`nopitch-ch7-35`・`notimemode-ch7-35`)の結果で『ピッチ』と『伸縮』のどちらの分岐かが分かれば、#1 のどの引数を見るべきかがさらに絞れる(ピッチなら `xmm3` と r9 の旗、伸縮ならモード番号)。

## 4. 限界

静的な読み。`f_a97780` の呼び元(どのクリップ・どのモードから呼ばれるか)、モード番号と FLP の mode(−2/−1/5)の対応、関数の引数の意味は未確認。vtable のメソッドの並びは調べていない。

## 5. wasm 側で Elastique の引数を記録する見積り(PM 依頼 (2)。alpha1、date 22:17。読みと過去の build の実測だけで、新しい実行も build もしていない)

**結論**: wasm 側は『生成 C の 2 つの unit(`fle-03.c`・`fle-08.c`)に `fprintf(stderr, …)` を 4〜5 行足して再 build し、Node の Export(約 2.4 分)を 1 回流す』で足りる。再 build は約 10 分、実行を足して約 13 分、重い job 1 本(nice 19、JOBS 1)。Chrome 窓の間は始めない。

**なぜ生成 C に行を足すのが最も安いか(静的に確認した事実)**
- 対象の 4 関数はどれも C 後端が出した関数で、新後端(wasm 直接生成)には選ばれていない(`n0-fle.tsv` の `f_a97780` の行が『not selected for the new backend (hybrid)』。`f_677f60`・`f_677fb0`・`f_678010` は `fle-03.c`〔L120569〜L120650〕、`f_a97780` は `fle-08.c`〔L91660〕にある)。
- 生成 C の関数は引数を全域の `R[]`(整数レジスタ、rcx=`R[1]`・rdx=`R[2]`・r8=`R[8]`・r9=`R[9]`)と `X[]`(xmm)で受け、スタック引数は `fle_ld(R[4] + n, …)` で読める。`fle_runtime.h` が `<stdio.h>` を取り込んでいるので `fprintf(stderr, …)` がそのまま書ける。
- 入口で読めるもの(関数の入口の `R[4]` = 戻り番地の位置): `f_677f60`(ElPro、v2)・`f_677fb0`(ElProV3、v3)・`f_678010`(ElResampleSource)は `ecx=R[1]`・`edx=R[2]`・`xmm2`(float)・`r8`・`r9d=R[9]`、v3 だけ `[R[4]+0x28]` の float を足す。`f_a97780` は `this=R[1]`・`edx=R[2]`・`r8d=R[8]`・`xmm3`(double、`X[3]`)と、**5 番目の引数 = `fle_ld(R[4] + 0x28, 4)` がモード番号**(関数の中で `sub rsp,0x28` と push 2 回の後に `[rsp+0x60]` を読む形と一致する。0308b §1-3 の「`[rsp+0x60]`」は関数本体の中の位置で、入口では `[rsp+0x28]`)。

**記録する行(例、各 400 行で打ち切る)**
| 場所 | 出す物 |
|---|---|
| `f_a97780` の入口(`fle-08.c`) | `this`〔ポインタなので比較から除く〕・edx・r8d・double(`X[3]` を `%.9g`)・**mode**・(`mode<=15` なら 0xC03D のビット = v2/v3) |
| `f_677f60` / `f_677fb0` の入口(`fle-03.c`) | edx・ecx・float(`X[2]`)・r8・r9d、v3 のみ末尾の float。ここが zplane の `CreateInstance` の引数そのもの |
| `f_678010` の入口 | 同上(ResampleSource を使うクリップがあるか見る) |
| (任意)`f_a97850` の入口 | 比の設定の呼び(`this+0x10`/`this+0x8` の実体に `[vtable+0x50]`〔float〕と `[vtable+0x58]`〔整数〕を呼ぶ関数)の double と r8d。1 ブロックごとなので最初の 200 回で打ち切る |

**費用の内訳(過去の build の実測)**: 2 unit の再コンパイル 計約 45 秒(`w-Oz/obj/log/fle-03.log.time` = 25.9 s、`fle-08.log.time` = 15.8 s)+ weblink 3〜4 秒 + wasm-opt(`BINARYEN_CORES=4`)198〜261 秒 + hybrid の各段(`hyb-gen-fle` 73〜81 秒、`hyb-gen-dsp` 85 秒、他は 11 秒以下、merge 11 秒)で約 4 分 = 約 9〜10 分。Node の Export が約 2.4 分。**危険**: (i) 生成 C の sha を点検する段〔`generated-c-sha256.txt`〕が編集を拒む可能性(未確認。試験の木なので、その段を外すか sha を更新する)、(ii) 出力が出る stderr の取り込み先(`g3-run.sh` が Node の stderr を `runs/<名>.log` に保存している=確認済み)。

**他の仕組みを選ばない理由**
- `fle_hook`(`fle_hook_point`/`fle_hook_target`)は『ゲストが関数の先頭 5 バイトを書き換えた時に本体を走らせるか』の印で、ログ用ではない。
- G3 の名前付き停止 `FLE_FRUITY_STOP_AT` は **import thunk(API 名)でだけ**止まる(`harness/fruity.c`)。`GetProcAddress` で止めても Elastique の `CreateInstance` の引数は見えず、`f_a97780` では止まれない。
- `FLE_FRUITY_PEEK_AT=<VA>:<reg>` は関数の入口でレジスタを出すが、`-finstrument-functions` の入口フックに乗る(`__wrap___cyg_profile_func_enter`)ので、フックを有効にした stage-1 の harness(`CreateFruityInstance` の呼びの間)にだけある。製品の bundle には入っておらず、reg も rcx/rdx/r8/r9 に限られる(スタック引数のモード番号は取れない)。使えない見込み(未確認)。

**この記録の読み方**: mode・API(v2/v3)・整数と float の引数が Wine(`winedbg` の条件付き bp か shim)と同じなら、FL 側の判定は同じで『Elastique 自体の出力の差』(PCM の比較 §2 の #4 へ)。違えば、その引数が原因の候補(例えば mode 番号の写像、または音声の出力レートの引数)。(b)(`nopitch-ch7-35`・`notimemode-ch7-35`)の結果が出ていれば、見る引数を絞れる(§3-3)。

## 6. 追加の静的読み(22:3x。**訂正あり: 23:4x に opt1 の Wine の ELOG で、ch 7/35 はこの節の経路を通らないと分かった。下の『訂正』を先に読むこと**)

**訂正(23:4x)**: opt1 の Wine の ELOG(隠した構成、Export 全体)で `f_a981f0`・`f_a97780`・`f_a97930`・`677fb0` は各 **1 回だけ**(mode = 5 = Auto の別のクリップ = atmos pad〔ch 45〕、len = 823,590 フレーム、ratio = 1.4556531906127930 = 2^(650/1200) = **pitch 比**)で、ch 7(mode −2)・ch 35(mode −1)は通らない。ch 7/35 の伸縮は **`f_678010` = `ElResampleSource_CreateInstance`(Wine で 1,254 回)の経路**(`f_67d5a0` が作る。§8)。以下の「mode −1/−2 は ElProV3・旗 0」「オフラインの事前レンダリング」「ブロック長に依らない」は **Auto(5) の pad の経路についてだけ**成り立つ読みで、ch 7/35 には**当てはまらない(ブロック長への依存は ch 7/35 について未確認に戻す)**。`f_a981f0` の ratio 引数は時間の比でなく pitch の比で、時間の比は 5 番目(入口の `[rsp+0x28]`)の double(呼び元 `f_a9b220` の `0xa9b3eb`)。

（元の節の見出し: 追加の静的読み(22:3x。`onat/src/fle-08.c`・`fle-10.c` の生成 C を読んだだけ。実行なし）

- **モード番号の決まり方**: `f_a981c0(クリップ, mode)` は『mode が 5(Auto)ならクリップの `[+0x10c]`〔配列。要素数 > 1〕で 3、それ以外で 1』『それ以外は mode をそのまま返す』。したがって FLP の mode −1・−2 は**そのまま**符号付き整数として `f_a97780` に渡り、`cmp eax,0xf; ja`(符号なし比較)で『16 以上』の側 = **`ElProV3_CreateInstance`(v3)**に行く。旗 `r9` は `table[mode]` の符号付き 1 バイト = `table[-1]`・`table[-2]` = 表の直前の 2 バイト(`0x14E01E9`・`0x14E01E8`)で、PE の `.data` の実バイトは**どちらも 0x00**(`FLEngine_x64.dll` の RVA 0x10E01E8〜、静的データなので Wine と wasm で同じ)。= ch 7(−2)・ch 35(−1)は両方 v3・旗 0 の一つの経路。(この関数に『mode −2 が Elastique でなく FL 自身の伸縮に行く』分岐は見えない。)
- **伸縮は『オフライン』の事前レンダリング**: `f_a981f0`(クリップ、新しい長さ、比〔double〕、mode)は、`f_a97780` で伸縮器を作り(edx = 0x100、r8d = チャンネル数 `[クリップ+0x74]`、xmm3 = サンプルレート `[クリップ+0x7c]`〔double〕)、クリップ全体を `f_a97930`(処理の呼び。引数の並び = this、バッファ 2 本、整数 2 つ〔フレーム数と読める〕、double の比、チャンネル数、旗。**意味は推定**)で何回かに分けて処理し、結果を `[クリップ+0x8c]`、長さを `[+0x94]` に置く。つまり**ミキサーのブロック長(オーディオデバイスの buffer)には依らず**、クリップの中身・長さ・比で決まる(読み)。`f_a97780` の呼び元は 5 つ(`0xa9830d` = この事前レンダリング、`0xc65a29`・`0xc65bb2`・`0xc7a2cf`・`0xc7be49` = fle-10 の別系統。`0xc7a3xx` の系統は pitch の比〔`f_40ae70` = SSE2 の exp の自前実装、`mulsd/addsd/cvttsd2si` だけ〕と時間の比〔`f_c78940`〕を作って `f_a97850`〔比の設定〕と `f_a97930` を呼ぶ)。
- **含意**: 『ブロック長の違い』という仮説は、この経路では支持されない(読み)。残る分け方は (i) 引数・入力 PCM が違う(MP3 の復号後・リサンプル後の PCM、長さ、比の double の最下位ビット)、(ii) 同じ入力・引数で Elastique 自体の出力が違う(lift した算術の差)。**切り分けに最適な記録点は `f_a97930` の入口と出口**: 2 本のバッファの内容の FNV-64(呼びの前と後の両方)、整数 2 つ、比の double のビット列、チャンネル数、および `f_a981f0` の入口の `(クリップ, 新しい長さ, 比のビット列, mode)`。両側で入力のハッシュが同じで出力が違えば (ii)(Elastique の lift の差分を命令単位で探す)、入力が違えば (i)(上流を探す)。§5 の表に `f_a97930` と `f_a981f0` を足せば同じ再 build(約 10 分)で取れる。Wine 側は winedbg/gdb の条件付き bp と PCM の読み出し(opt1)。

## 7. ログ行の形式と道具(22:3x、PM 22:25 の指示に沿う。道具は書いただけで、build にも実行にも使っていない)

`tools/a22-make-elog.py <onat/src>` が使い捨ての木の生成 C(`fle-03.c`・`fle-08.c`)に §5・§6 のログを足す(冪等。試験用。製品の木には適用しない)。コピーに適用して差分を確認した(構文の検査は clang をまだ走らせていない = 窓の後)。1 行 1 呼び出しの固定形式、数値は小文字の 16 進(0x なし)、float/double は生のビット列、`seq` は関数ごとに 0 から:

| 行 | フィールド |
|---|---|
| `ELOG 677f60 seq` / `677fb0` / `678010` | `ecx edx f2`(xmm2 の下位 32 ビット)`r8 r9`、`677fb0` だけ `f5`(入口の `[rsp+0x28]` の float のビット) |
| `ELOG a97780 seq` | `p`(this、比較から除く)`edx r8 d3`(xmm3 の double のビット)`mode`(入口の `[rsp+0x28]`) |
| `ELOG a981f0 seq` | `p`(クリップ、除く)`len`(rdx)`ratio`(xmm2 の double のビット)`mode`(r9d)`ch`(`[クリップ+0x74]`)`sr`(`[クリップ+0x7c]` の double のビット)`n`(`[クリップ+0x94]`) |
| `ELOG a97930 seq` | `p` `n1`(r9)`n2`(`[rsp+0x28]`)`ratio`(`[rsp+0x30]` の double のビット)`ch`(`[rsp+0x38]`)`flag`(`[rsp+0x40]` の 1 バイト)`h1 h2`(rdx と r8 のバッファの FNV-1a 64 を n×ch×4 バイトで、**呼びの前**) |
| `ELOG a97930x seq` | `h1 h2`(同じバッファを**呼びの後**で) |

どちらが入力のバッファかは呼び元からの推定なので両方を前後で取る。`a97930` と `a97930x` は `seq` で対にする。上限 = 全体で 10 万行・1 ハッシュ 1 MiB。Wine 側の detour(opt1)は同じ形式で出せば `p=` を除いて diff できる。

## 8. ch 7/35 の経路: ElResampleSource(23:4x。生成 C〔`onat/src/fle-03.c`、`wsrc/v-tb2d2/src/m7-03.c`〕と実 DLL〔`elastique_x64.dll` sha256 8551e922…、pefile + capstone〕の読み。第 2 版 ELOG の根拠)

- **FL 側**: `f_678010`(`ElResampleSource_CreateInstance` のラッパ)の呼び元は `f_67d5a0` の 1 箇所(`0x67d60d`)。`f_67d5a0` を呼ぶのは `0x67d556`(`[this+0x38] >= 1` なら作成、そうでなければ `f_67d570` = 解放)。作った物は `[this+0xf8]`。渡す引数: ecx = this(FL の『入力プロバイダ』。Elastique がここから入力を引く読み出しは `f_67d630`)、edx = r8 = チャンネル数(`[this+0xa0]` の byte)、xmm2 = float(`[クリップ+0x7c]` のサンプルレート)、**xmm3 = float(`[[this+0xb8]+8]` の整数)**、スタック 5〜8 番 = `([this+0x38]==2 ? 0 : 1)`・float 定数(`0x67d620`、Wine で 0x3dcccccd = 0.1f)・`0x2b2a`・0。r9 は未設定の残り(意味なし)。
- **パラメータの設定**: `f_67db90(this, xmm1 = pitch cents〔double〕, xmm2 = 時間比〔double、定数 0x67dd60/0x67dd68 の範囲に clamp〕, xmm3 = double)` が pitch 比 = `f_40ae70(cents × 定数)`(SSE2 の自前 exp)を `[this+0x18]` に置き、**ResampleSource の vtable+0x8 を (obj, xmm1 = 時間比, xmm2 = pitch 比, xmm3 = 第 3 の double) で呼ぶ**(`0x67dd22`)。`f_67d9b0(this, xmm1 = 合成比)`、`f_67dd90`/`f_67ddf0`(xmm1 = double)は vtable+0x18 を呼ぶ。`f_67db00` は pitch == 0 かつ比 == 1.0 なら `f_67d9b0(1.0)`、でなければ exp(cents × 定数) × 比。
- **Elastique 側(`elastique_x64.dll`、ImageBase 0x180000000、runtime では 0xb000000 + RVA)**: `ElResampleSource_CreateInstance` = RVA **0x285320**(5 番目 = 旗〔0/1 に正規化〕、6 番目 = float、7 番目 = int、8 番目 = int)。作る物のラッパの vtable = RVA **0x3E8770**: +0x00 解放 0x2852f0、+0x08 0x284700・+0x10 0x284c20・+0x18 0x284590・+0x20 0x284e60・+0x28 0x284c10 は内部実装の vtable への転送 thunk(`mov rcx,[rcx+8]; jmp [rax+off]`)、+0x38 0x2853e0、**+0x40 = RVA 0x285460 = 処理**: `(this = rcx, rdx = バッファ A, r8 = バッファ B, r9d = n)`、0x4000 サンプルごとに分割、`f_b0018a0`(A を this の内部 2 本へ)→ 内部実装の vtable[0] → `f_b001870`(内部 2 本を B へ)、戻り値 eax。ポインタは n<<2 バイトずつ進む。A/B がインターリーブかは未確認。`ElProV3_CreateInstance` の export 入口(RVA 0x285040)は Wine で 0 回(opt1)。
- **第 2 版 ELOG**(`tools/a22-make-elog2.py`、差分 `tools/a22-elog2-fle-03-m7-03.test-only.diff`、emcc の構文検査は通した): `ELOG b285460 seq inst= n= h1=`(入口)・`b285460x seq inst= n= r= h1= h2=`(出口)、`b285320`/`b285320x`(生成と戻り値 obj)、`b284700`(vtable+8 の 3 つの double)、`678010b`(xmm3 とスタック 5〜8 番)、`67dd90`・`67ddf0`(double)。各 20,000 行まで。

## 9. ElProV3 の側(23:5x。実 DLL の読み。pad〔Auto 5〕の経路の記録点)

- `ElProV3_CreateInstance` = RVA **0x285040**(forward でなく実体): 0x18 バイトのラッパを new し、vtable = RVA **0x3E8610**、`[obj+8]` に内部実装(`0x29da20(&obj+8, …)` が作る)。引数: ecx = FL の第 1 引数(0x100)、edx = チャンネル数、xmm2 = float(44100)、r9d = 品質の旗(0→0、1→1、2→2、3→3 の写像)、入口の `[rsp+0x28]` = float(0.1f)。
- ラッパの vtable(RVA 0x3E8610)のスロット: +0x00 解放 0x284fa0、+0x08 0x284fd0(常に 0 を返す)、+0x10 0x284fc0(0x29dd40 へ jmp)、+0x18 0x284700・+0x20 0x284c20・+0x28 0x284c10・+0x30 0x284590 は内部実装 vtable の +0x08・+0x18・+0x10・+0x20 への転送、**+0x38 0x285020(r9d を bool に正規化して内部 +0x28)・+0x40 0x284c60(同様に内部 +0x30)・+0x80 0x284fe0(edx と r8d を bool に正規化して内部 +0x70)**、+0x48 0x284620・+0x50 0x284740・+0x58 0x284c00・+0x60 0x284630・+0x68 0x284650・+0x78 0x284600 は純粋な転送、+0x88 0x284f50 は解放（vtable を書き換える形）。FL の `fb_a97930` が呼ぶラッパのオフセットは +0x88(2 回)・**+0x40(2 回)**・+0x80・+0x18・+0x28・+0x08・**+0x38**・+0x60。ブロックごとの処理は +0x38/+0x40/+0x80 のどれか(**未確定**)。
- 第 2 版 ELOG は 0x285040(入口と戻り値)と 0x285020・0x284c60・0x284fe0 の入口(生のレジスタ値 rdx・r8・r9・スタック 5〜6 番、ポインタは掘らない)を記録し、`a97930` の最初の 1 回の入力バッファ(rdx、n1×ch×4 バイト)と出力バッファ(r8、n2×ch×4 バイト)を `/dump/a97930-in.f32`・`a97930-out.f32` に書く。Wine 側も同じ 2 ファイルを作れば、**最初に違うサンプル**とその大きさ(ulp か、構造的か)が数値で分かる。
- ハッシュの範囲: `a97930x` の h1/h2 は先頭 **1 MiB**(262,144 個の float = インターリーブなら 131,072 フレーム、約 3 s)まで。入口の h2(出力バッファ)は両側で一致(= 初期値が同じ)、Wine は 2 回の run で出口の h2 が同じ(e3398e3c97f5b612)、wasm は WAV が run 間で bit 同一(決定的)。


## PM の判断と取り込み(2026-11-06 22:33、docs のみ)

取り込み: alpha1 の 0308b(静的な読み。実行なし)。判断は決定 0308 の追記にまとめた(Elastique の残差を lift の忠実性の候補として優先度を上げる、変種 4 本と入出力のハッシュ記録で切り分ける、単体 harness は最後)。wasm 側のログ付き build(約 10 分)と Wine 側の shim(約 2 時間)は窓の後。`f_a97930` の引数の意味は推定のままで、alpha1 と opt1 が形式を合わせてから実装する。


## PM の追記(2026-11-06 22:42、§5〜§7 の取り込み、docs のみ)

最新版(§5 見積り、§6 追加の静的読み、§7 ログ行の形式 `ELOG <fn> <seq> key=hex …`)を取り込んだ。差分 `a22-elog-fle-03-08.test-only.diff` は『コピーに適用しただけ』で、木にも build にも未適用(clang の構文検査も窓の後)。引数の意味(rdx=入力、r8=出力、r9=入力フレーム、5 番目=出力フレーム、float32)は呼び元からの推定。


## PM の追記(2026-11-07 00:45、alpha1 の §6 訂正・§8 ElResampleSource・§9 ElProV3 の取り込み、docs のみ)

最新版に差し替えた(上の PM の節は残した)。要点は 0308 の PM の追記(2026-11-07)に書いた: ch7/35 は ElProV3 ではなく ElResampleSource(`f_678010`)の経路で、process は RVA 0x284e60 → 0x2abc20。なお alpha1 は vtable の RVA の誤記を訂正した(ResampleSource のラッパ 0x3E8770、ElProV3 のラッパ 0x3E8610。スロットの関数の RVA は正しい)。
