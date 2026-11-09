# 0312k: runtime の RCPPS/RSQRTPS の C 実装を、実命令と全 2^32 の float32 ビットパターンで比べた結果(mt3。PM 指示 00:50。**native のみ、実エンジン・Chrome は 0**。実測)

**結果: 不一致 0**。
- (1) opt1 の Wine での実測(`~/work/opt1-wine/exports/rcpsqrt-wine-zen.txt`、65,024 入力)に runtime の `fle_rcp_f32`(`runtime/fle_rcp.h`、`fle_rcp_table.h` の表)と `fle_lg_rsqrt_f32`(`runtime/fle_sse_int2.h:817`)を当てた: **rcp の不一致 0、rsqrt の不一致 0**。全出力の FNV-1a 64(opt1 の `rcpsqrt.c` と同じ計算: 入力ごとに rcp・rsqrt の 8 byte)= **`936c3c2755dd54da`**(Wine の値と runtime の値の両方で一致。PM の値と同じ)。
- (2) **全 2^32 個の float32 のビットパターン**(正規数・非正規数・0・inf・SNaN・QNaN)で、この機械の実命令(`rcpps`・`rsqrtps`。インライン asm)と runtime の C 実装を比べた: **MXCSR の 4 通り(DAZ 0/1 × FTZ 0/1)× 2 関数 = 8 通りすべて 不一致 0 / 4,294,967,296**(クラス別の不一致も 0。最初の例は無い)。CPU = **AMD Ryzen 9 9950X(Zen 5)**。1 スレッド(`taskset -c 15`、nice 19)、wall **85 s**、RSS 8 MB。
- **MXCSR が実際に効いていたことの probe**: 同じ位置で `mulps 1.0` を実行し、非正規数 16,777,214 個が DAZ 0/FTZ 0 では 0 にならず(0 個)、他の 3 通りでは全部 0 になった(DAZ/FTZ が効いている。DAZ と FTZ の区別はこの probe ではつかない)。

## 方法と最初の版の誤り(自己申告)
- 最初の版は `_mm_setcsr` と `_mm_rcp_ps` の組込み関数を使ったが、probe に使った `_mm_mul_ps(v, 1.0f)` を**コンパイラが恒等として畳み込み**、MXCSR が効いたことの確認にならなかった(『DAZ in force: no』と出た)。インライン asm(`ldmxcsr`・`rcpps`・`rsqrtps`・`mulps`、`volatile` + `memory`)に直して再実行した。結果(不一致 0)は変わらない。
- 比べているのは**要素ごとの関数**(`fle_rcp_f32` と `fle_lg_rsqrt_f32`)。`rsqrtss` は同じ関数を使う(`sse_legacy.rs:133`)ので同じ。**`rcpss` と VEX 形(`vrcpps`/`vrsqrtps`)は fe-lift が lift していない**(`grep` で `rcpss`・`vrcpps`・`vrsqrtps` が `crates/fe-lift/src` に無い)ので、この試験の対象外。AVX-512 の `vrcp14ps`・`vrsqrt14ps` は別の命令。

## 含意(読み)
- **『近似命令(RCPPS/RSQRTPS)』は、この機械(Zen 5)で runtime の C 実装と全入力で bit 一致する**ので、**同じ機械の Wine の出力と wasm の出力の差(Elastique の残差)の原因ではない**(この 2 命令に限る)。Intel の CPU では未検証(`fle_rcp.h`・`fle_sse_int2.h` のコメントのとおり表は AMD 1 機種の測定)。依頼者の機械が Intel なら表が違う可能性は残る(依頼者の機械は 0 件)。

## 再現
`tools/rcp/rcpsqrt_full.c`(`clang -O2 -msse2 -I lift/runtime rcpsqrt_full.c -o rcpsqrt_full`、`./rcpsqrt_full ~/work/opt1-wine/exports/rcpsqrt-wine-zen.txt`)。出力: `tables/0312k/run3.txt`。


## PM の判断と取り込み(2026-11-07 01:04、docs のみ。製品の差分なし)

取り込み: mt3 の 0312k(runtime の RCPPS/RSQRTPS の C 実装を、実命令と全 2^32 の float32 ビットパターンで比べた結果)。**native のみ(clang -O2 -msse2、1 スレッド)、実エンジン・Chrome・依頼者の機械は 0 件**。

事実として受け取る(mt3 の実測、run3.txt):
1. opt1 の Wine での実測 65,024 入力に runtime の fle_rcp_f32 と fle_lg_rsqrt_f32 を当てて不一致 0、全出力の FNV-1a 64 = 936c3c2755dd54da(Wine と runtime で一致)。
2. 全 2^32 個のビットパターンで、この機械(AMD Ryzen 9 9950X、Zen 5)の実命令と runtime の C 実装を比べた: MXCSR 4 通り(DAZ 0/1 × FTZ 0/1)× 2 関数で、全て不一致 0 / 4,294,967,296(1 スレッド、85 s)。MXCSR が効いていることは mulps の probe で確認(DAZ または FTZ が立つと非正規数 16,777,214 個が全部 0 になる)。
3. 自己申告: 最初の版は組込み関数の probe をコンパイラに畳み込まれて確認にならなかったので、インライン asm に直して再実行(結果は同じ)。

判断:
1. **近似命令 RCPPS/RSQRTPS は Elastique の残差(Wine 対 wasm、RMS 2,031)の原因から外す**。同じ機械(Zen 5)の Wine と wasm の差は、この 2 命令では説明できない。これで棄却済みの原因は、初期 FPU 状態(x87 CW・MXCSR)、数学関数の import(0 件)、cpuid の分岐(SSE2 に揃えた Wine で RMS 16.3 しか動かない)、近似命令、block 長(b284e60 の n が 7,270/7,270 一致)。
2. 検証の範囲外(mt3 の注記どおり): rcpss と VEX 形(vrcpps/vrsqrtps)は fe-lift が lift していないので対象外、AVX-512 の vrcp14ps は別命令、Intel の CPU は未検証(表は AMD 1 機種の測定)。依頼者の機械が Intel なら表が違う可能性は残る(依頼者の機械は 0 件)。この点は G9 以降の「実機との照合」の項目に残す。
3. 残る候補は、(a) 入力の違い(Elastique に渡る MP3 の復号 PCM = 外部 API の実装差)と、(b) lift した Elastique の命令の意味の差。alpha1 の elog5(Init の mode 別の内訳と clip の疎ハッシュ)と opt1 の Wine elog10 で (a) か (b) かを切り分ける。
