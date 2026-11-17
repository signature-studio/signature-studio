# 0075: dsp_ippv2_x64.dll のレガシー SSE / SSE2 未対応形を実装する（W8 の一部）

状態: 作業者（サブエージェント）が決定。依頼者（PM）の判断は含まない。実測は `docs/investigations/dsp-ippv2-sse-unsupported.md`。方向は決定 0073（dsp_ippv2_x64.dll を 2 つ目のモジュールとして lift する）。実装は `lift/crates/fe-lift/src/sse_legacy.rs`（新規。`sse_more.rs` から `#[path]` で宣言し、`lib.rs` は触っていない）、runtime は `lift/runtime/fle_sse_int2.h`（既存ファイルへの追記のみ。新しい runtime ファイルは作っていない）、テストは `lift/crates/fe-lift/tests/sse_legacy.rs` と `tests/rsqrt/rsqrt_exhaustive.c`（どちらも新規）。

## 1. 対象と ISA 世代の判断

- 対象: 現行 CPUID 応答の系統（`table-seeds-idx0.txt`、調査文書 §5.5 の添字 0）に出るレガシー命令の未対応 4,074 箇所（35 キー）のうち、**現行の CPUID 応答が名乗る世代**の命令。
- 現行の CPUID 応答（決定 0009、0047。`lift/runtime/fle_cpuid.c` を読んだ）: 葉 1 の EDX は bit 25（SSE）と bit 26（SSE2）のみ、ECX は 0。したがって **SSE3、SSSE3、SSE4.1、SSE4.2、AVX は名乗らない**。決定 0009 の理由は「lift が対応していない機能を主張しない」。この作業は CPUID を変えていない（`tests/cpuid.rs` の `runtime_cpuid_never_reports_osxsave_and_avx` も通る）。
- 世代の判定は SDM の各命令の「CPUID Feature Flag」列（felixcloutier の転載。§6）による。SSE と SSE2 の命令だけを実装した。**SSE3 の命令（Lddqu、Movddup、Addsubps、Addsubpd、Movshdup、Movsldup、Haddpd。678 箇所）は、CPUID が SSE3 を名乗っていないので実装しなかった**（§5）。

## 2. 実装した形（25 ニーモニック。実装前の箇所 3,369）

| ニーモニック | 形（実装した範囲） | 符号化 | 世代 | 実装前の箇所 |
|---|---|---|---|---:|
| Pmuludq | xmm, xmm/m128 | 66 0F F4 | SSE2 | 952 |
| Orps | xmm, xmm/m128 | 0F 56 | SSE | 773 |
| Psrldq | xmm, imm8 | 66 0F 73 /3 | SSE2 | 415 |
| Andnps | xmm, xmm/m128 | 0F 55 | SSE | 366 |
| Movmskps | r32/r64, xmm | 0F 50 | SSE | 155 |
| Pmovmskb | r32/r64, xmm | 66 0F D7 | SSE2 | 129 |
| Pextrw | r32/r64, xmm, imm8 | 66 0F C5 | SSE2 | 111 |
| Pcmpeqb | xmm, xmm/m128 | 66 0F 74 | SSE2 | 110 |
| Rsqrtps | xmm, xmm/m128 | 0F 52 | SSE | 75 |
| Pslldq | xmm, imm8 | 66 0F 73 /7 | SSE2 | 49 |
| Movntps | m128, xmm | 0F 2B | SSE | 46 |
| Movntdq | m128, xmm | 66 0F E7 | SSE2 | 45 |
| Movmskpd | r32/r64, xmm | 66 0F 50 | SSE2 | 31 |
| Movntpd | m128, xmm | 66 0F 2B | SSE2 | 26 |
| Pinsrw | xmm, r32/m16, imm8 | 66 0F C4 | SSE2 | 22 |
| Psubq | xmm, xmm/m128 | 66 0F FB | SSE2 | 16 |
| Pcmpgtw | xmm, xmm/m128 | 66 0F 65 | SSE2 | 11 |
| Psubb | xmm, xmm/m128 | 66 0F F8 | SSE2 | 8 |
| Pmaxub | xmm, xmm/m128 | 66 0F DE | SSE2 | 6 |
| Pminub | xmm, xmm/m128 | 66 0F DA | SSE2 | 6 |
| Movnti | m32, r32 / m64, r64 | 0F C3 | SSE2 | 4 |
| Pcmpeqw | xmm, xmm/m128 | 66 0F 75 | SSE2 | 4 |
| Pcmpgtb | xmm, xmm/m128 | 66 0F 64 | SSE2 | 4 |
| Pmaddwd | xmm, xmm/m128 | 66 0F F5 | SSE2 | 4 |
| Rsqrtss | xmm, xmm/m32 | F3 0F 52 | SSE | 1 |

実測の形は調査文書 §3.1。表に無い **Paddb も実装した**（添字 0 の閉包には出ないが、全体の閉包（`merged-text.dll` の `--pdata`）に `paddb xmm, xmm` の未対応が 15 箇所あった。PSUBB と対称。テストは 969 + 4,096 件）。したがって実装は 26 ニーモニックで、「25 ニーモニック」は添字 0 の閉包に出ていたものの数（3,369 箇所）。実装後の添字 0 の閉包は、レガシーの未対応 705（SSE3 の 678、`Int3` 21、`Int` 5、`Fxsave` 1）。`unsupported_total` は 4,938 → 1,569。全体の閉包（`merged-text.dll` の `--pdata`）は 1,011,928 → 974,004。

- 実行形: 生成 C は `X[n] = fle_lg_xxx(X[n], <src>);` のように `fle_sse_int2.h` の `static inline` を呼ぶ。Orps は `fle_por`、Andnps は `fle_pandn`（ビット演算で `orpd`、`andnpd` と同じ。既存）を使う。GPR への結果は `R[n] = ...;`（32 ビット形も 64 ビット形も同じ値。マスクは最大 16 ビットなので、32 ビット書き込みのゼロ拡張と一致）。フラグは変えない。上位 128 ビット（MAXVL−1:128）はこのモデルに無い。
- 実装しなかった形（同じニーモニックの別形）: MMX 形（`mm` レジスタ。`sse_int2.rs` 等と同じく `None` を返す）、VEX/EVEX 形、PEXTRW の `66 0F 3A 15`（reg/m16 形。SSE4.1）。後者は `Code` で区別して明示的に拒否する（テスト `other_forms_stay_unsupported`）。

## 3. 仕様の出典と強さ

出典は felixcloutier.com/x86/<命令>（Intel SDM の**非公式の転載**。各ページに「No longer updated → use Intel documentation」「UNOFFICIAL, mechanically-separated, non-verified」とある。2026-10-23 取得。取得した本文は `lift/out-w8/spec/` に残してある）。公式 PDF は未取得で、転載の内容が公式と一致するかは**未確認**。

| 命令 | 読んだ内容 | 強さ |
|---|---|---|
| ORPS、ANDNPS | 128-bit Legacy SSE の記述: 第 2 ソースは xmm か 128 ビットメモリ、上位 MAXVL-1:128 は不変。ANDNPS の演算は `(NOT SRC1) AND SRC2`（EVEX の擬似コードで確認。Legacy の擬似コードは転載に無く、説明文が「bitwise logical AND NOT」）。例外: Table 2-21（Type 4）。 | 仕様（非公式の転載） |
| PCMPEQB/W、PCMPGTB/W | 同値（符号付きの大小）で全 1、他は 0。例外 Type 4。 | 仕様（非公式の転載） |
| PADDB、PSUBB、PSUBQ | 各レーンの wrap する加減算。例外 Type 4。 | 仕様（非公式の転載） |
| PMAXUB、PMINUB | レーンごとの符号なし最大、最小。例外 Type 4。 | 仕様（非公式の転載） |
| PMADDWD | `DEST[31:0] := DEST[15:0]*SRC[15:0] + DEST[31:16]*SRC[31:16]` と 4 レーン（符号付き）。例外 Type 4。 | 仕様（非公式の転載） |
| PMULUDQ | `DEST[63:0] := DEST[31:0]*SRC[31:0]; DEST[127:64] := DEST[95:64]*SRC[95:64]`（符号なし）。例外 Type 4。 | 仕様（非公式の転載） |
| PSRLDQ、PSLLDQ | `TEMP := COUNT; IF (TEMP > 15) THEN TEMP := 16; DEST := DEST >> (TEMP*8)`（左シフトは `<<`）。count は imm8 のバイト数。例外 Type 7。 | 仕様（非公式の転載） |
| PMOVMSKB、MOVMSKPS、MOVMSKPD | PMOVMSKB: `r32[k] := SRC[8k+7]`（k = 0..15）、`r32[31:16] := 0`。MOVMSKPS: `DEST[k] := SRC[32k+31]`（4 ビット）。MOVMSKPD: `DEST[k] := SRC[64k+63]`（2 ビット）。上位ビットは 0。例外 Type 7。 | 仕様（非公式の転載） |
| PEXTRW | `66 0F C5`: `SEL := COUNT[2:0]; TEMP := (SRC >> SEL*16) AND FFFFH; r32[15:0] := TEMP; r32[31:16] := ZERO_FILL`（64 ビット GPR でも上位は 0）。`66 0F 3A 15`（reg/m16）は SSE4.1。 | 仕様（非公式の転載） |
| PINSRW | `SEL := imm8[2:0]; DEST.word[SEL] := src.word[0]`。ソースは r32 の下位 16 ビットか m16。他のワードは不変。 | 仕様（非公式の転載） |
| MOVNTPS、MOVNTPD、MOVNTDQ | m128 への 128 ビット非一時ストア。値は通常のストアと同じ（ヒントは値に影響しない）。例外 Type 1（アライメント #GP を含む）。 | 仕様（非公式の転載）。「ヒントは値に影響しない」は一般知識（転載の説明は「using non-temporal hint」） |
| MOVNTI | `DEST := SRC`（m32 ← r32、REX.W で m64 ← r64）。例外に #AC(0)（アライメント検査が有効で CPL 3 のとき）だけ。アライメント #GP は無い。 | 仕様（非公式の転載） |
| RSQRTPS、RSQRTSS | 精度（`|Relative Error| ≤ 1.5 * 2^−12`）と特殊値の規則だけ。`APPROXIMATE` の定義は書かれていない。**結果のビットは仕様から決まらない（実装依存）**。§4。 | 仕様は特殊値と精度の上限まで。値はホストの測定（§4） |

アライメント（SDM の「Other Exceptions」が指す表）: Type 4（Table 2-21）と Type 1（Table 2-18）には「#GP(0) Legacy SSE: Memory operand is not 16-byte aligned」がある。したがって ORPS、ANDNPS、PCMPEQx、PCMPGTx、PADDB、PSUBx、PMAXUB、PMINUB、PMADDWD、PMULUDQ、RSQRTPS の m128 ソースは `fle_ldxa`、MOVNTPS、MOVNTPD、MOVNTDQ の m128 デスティネーションは `fle_stxa`（境界外は abort。`fle_misaligned`）。PINSRW の m16、RSQRTSS の m32、MOVNTI は境界の規則が無いので `fle_ld` / `fle_st`（任意のアドレス）。
**ホスト実測で上と一致した**（`lift/out-w8/probe_align.c`。16 バイト境界のバッファに対し、オフセット 0、1、4、8、15 で実行し、SIGSEGV を捕まえた）: m128 の 19 形は、オフセット 0 だけが成功し 1、4、8、15 が fault。`rsqrtss`、`pinsrw`（m16）、`movnti`（32/64）はどのオフセットでも成功。結果は `lift/out-w8/probe_align.txt`。RSQRTPS の転載も例外の表を Table 2-21（Type 4）とする。つまりアライメントの扱いは、SDM の文面（転載）とホスト実測の両方が根拠。

## 4. RSQRTPS、RSQRTSS（近似命令）の扱い

SDM は結果の下限（精度）と特殊値だけを規定する（RCPPS と同じ。決定 0047 §2）。そこで決定 0047 と同じ方針をとる: **ホスト（AMD Ryzen 9 9950X、Zen 5）の `rsqrtps` にビットを合わせる**。これは SDM の規定ではなく、作業者の決定。

- 測定（`lift/out-w8/rsqrt_probe.c`）: 正の正規化数について、結果の仮数（下位 11 ビットは 0）は「入力の指数部 e の偶奇」と「入力の仮数の上位 12 ビット」だけで決まる（指数 1〜254 × 4,096 通りで、仮数の下位 11 ビットを 0 と 1 にしたものが一致。依存 0 件）。結果の指数部は `189 − (e − e0)/2`（e0 は e が奇数なら 1、偶数なら 2）。閉形式の候補（`1/sqrt(1 + (i+h)/4096)` を floor と round、h を 33 通り）は最良でも 963/4,096 が不一致で、見つからなかった。このため 2 × 4,096 の表（`fle_rsqrt_tab`。`runtime/fle_sse_int2.h` に直接置いた。ホストの `rsqrtps` の出力から生成）をそのまま埋め込む。
- 特殊値（ホスト実測が SDM の記述と一致）: 0 と非正規化数は、符号付きの ∞（符号は入力の符号。DAZ に関係なく）。+∞ は +0。負の非ゼロ（負の正規化数と −∞）は QNaN の不定値 `0xFFC00000`。SNaN は QNaN（`x | 0x400000`）、QNaN はそのまま。
- MXCSR: 結果は RC、FTZ、DAZ、マスクに影響されず、状態ビット（IE、DE、ZE、OE、UE、PE）を立てない（SDM の「SIMD Floating-Point Exceptions: None」）。ホストで 6 通りの MXCSR（0x1f80、0x9fc0、0x7f80、0x3f80、0x5f80、0x0000）、全 2^32 入力で確認した。**したがって、決定 0044 の MXCSR 状態ビット（`fle_mxcsr` への OR）は、RSQRTPS と RSQRTSS では操作しない**（RCPPS と同じ。SDM が例外なしとし、ホストも立てないため）。
- 実装は純 C（整数）。ホストの FP ユニット、asm は使わない。
- 未確認: **Intel の CPU との違い**。Intel 実機で同じ表になる保証はない（SDM は精度の上限だけを規定する）。表は 1 台の AMD CPU の測定値で、公式の資料ではない。FL Studio や IPP の結果が Intel 実機と bit 一致するかは、この決定では保証しない。

## 5. 実装しなかった形と理由

| 形 | 箇所（添字 0 の閉包） | 理由 |
|---|---:|---|
| Lddqu、Movddup、Addsubps、Addsubpd、Movshdup、Movsldup、Haddpd | 678 | SSE3。**CPUID が名乗らない**（葉 1 の ECX = 0）。CPUID に SSE3 を足すか、これらを実装するかは PM が決めること（SSE3 を名乗ると、IPP の CPU 振り分けの選択値（調査文書 §5.2）が 0 から変わる可能性がある。仮説、未確認。**選択値の機能マスクの bit 3 が SSE3**（読み））。実装する場合は、命令の仕様（SDM）と突合の方法は本作業と同じで足りる。 |
| Pshufb、Palignr、Phaddd、Pabsw、Pabsd（SSSE3） | 全体の閉包にのみ出る（添字 0 の閉包では 0） | SSSE3。CPUID が名乗らない。 |
| Insertps、Pmovzx/sx*、Pminud、Pmaxud、Pminsd、Pmaxsd、Pmulld、Pinsrb/d、Pextrb/d、Packusdw、Blend*、Dpps、Ptest、Movntdqa、Extractps（SSE4.1） | 全体の閉包にのみ出る | SSE4.1。CPUID が名乗らない。 |
| Popcnt、Shlx、Shrx、Sarx、Andn、Blsr、Lzcnt、Mulx、Rorx（POPCNT、BMI、LZCNT） | 全体の閉包にのみ出る | CPUID が名乗らない世代（葉 1 の ECX と葉 7 は 0）。 |
| PEXTRW `66 0F 3A 15`（reg/m16） | 全体の閉包に 1 | SSE4.1。 |
| Fxsave | 1 | CPUID が名乗る世代に属するかは**未確認**（SDM のページを読んでいない）。FXSAVE は 512 バイトの領域に x87 の状態、MXCSR、XMM0〜15 を書く命令（一般知識。仕様は未確認）で、現行の `fle_x87*` と `fle_mxcsr` の状態から領域を作る必要があり、この作業の範囲を超えると判断した。1 箇所。未実装のまま残した。 |
| Int3（21）、Int（5） | 26 | パディングまたは致命エラーの経路。決定 0073 の W 外。 |
| 間接 jmp 目標が未解決（464）、IAT 経由の call/jmp（122）、`gs:`（3） | 589 | W3、W5、W7 の範囲。この作業の対象外。 |
| VEX / EVEX（`V*`） | 275 | CPUID が AVX を名乗らない（決定 0047 の XGETBV の条件）。 |

## 6. ホスト突合の結果（件数）

テストの流儀は決定 0026 と同じ（`tests/sse_legacy.rs`）: 生成 C（clang -O0、ヘッダのみの runtime）と、ホストの実機語（mmap した実行ページ。組み込み関数は使わない）に同じ状態を与え、16 本の XMM、rax・rcx・rdx・rbx・rsi、64 バイトのメモリがビット単位で一致することを確認する。テストが「命令が実際に何かを変えた」ことも確認する（80% 以上。PMAXUB、PMINUB の網羅は 50%）。

| 命令 | ケース数 | 不一致 | 入力の作り方 |
|---|---:|---:|---|
| orps / andnps | 1,241 ずつ | 0 | レーンの端の値 + 乱数、レジスタ形とメモリ形（disp −16/0/16/32）、dst = src |
| pcmpeqb / pcmpgtb | 969 ずつ + 4,096 ずつ（全 65,536 組のバイトの対） | 0 | 同上。全バイト対は 4,096 ケースの各 16 レーンに 1 回ずつ |
| pcmpeqw / pcmpgtw | 1,241 ずつ | 0 | 同上 |
| paddb / psubb | 969 ずつ + 4,096 ずつ（全バイト対） | 0 | 同上 |
| psubq | 1,241 | 0 | 同上 |
| pmaxub / pminub | 969 ずつ + 4,096 ずつ（全バイト対） | 0 | 同上 |
| pmaddwd | 1,241 + 64（0x8000 × 0x8000 の隅） | 0 | 同上 |
| pmuludq | 1,241 | 0 | 同上 |
| psrldq / pslldq | 512 ずつ | 0 | imm8 を 0〜255 の全て（count > 15 を含む）、各 2 回 |
| pmovmskb | 1,276 | 0 | 16 ビットのマスク: 全ビット単独、全ビット以外、0x5555/0xaaaa/0x00ff/0xff00 と乱数 600、r32 と r64（REX.W） |
| movmskps / movmskpd | 640 / 320 | 0 | 4 ビット（16 通り）と 2 ビット（4 通り）の全パターンを 20 回 / 40 回 × r32、r64 |
| pextrw | 512 | 0 | imm8 を 0〜255 の全て × r32、r64 |
| pinsrw | 768 | 0 | imm8 を 0〜255 の全て × （r32、r64（REX.W）、m16 の任意アドレス） |
| movntps / movntpd / movntdq | 300 ずつ | 0 | 16 バイト境界のアドレス、4 通りの disp |
| movnti | 400 | 0 | m32 と m64、任意のアドレス（境界外を含む） |
| rsqrtps | 3,256 | 0 | 特殊値と端の値、指数 0〜255 の全て、乱数、レジスタ形とメモリ形 |
| rsqrtss | 3,000 | 0 | 同上。m32 は任意のアドレス |
| **rsqrtps の全数** | **6 通りの MXCSR × 2^32 = 25,769,803,776 入力** | **0**（状態ビットの変化 0） | `tests/rsqrt/rsqrt_exhaustive.c`（`-O2`、約 1 分） |

- 命令単位のケースの合計（上の表の整数）は **51,237**（`cargo test --test sse_legacy` が出す `cases N failures 0` の合計。33 グループ）。全て不一致 0。テストは 37 件（33 のホスト突合、アライメント abort、拒否する形、ケース集合の網羅、rsqrtps の全数）。
- **アライメント abort**: 13 形（orps、andnps、pcmpeqb、pcmpgtw、paddb、psubq、pmaxub、pmaddwd、pmuludq、rsqrtps、movntps、movntpd、movntdq）の境界外オペランドが、生成 C で `misaligned 16-byte access` の abort になることを確認（ホストは #GP になるので C 側だけ実行）。
- **拒否する形**（`other_forms_stay_unsupported`、12 件）: MMX 形 5、PEXTRW の `66 0F 3A 15` 2、VEX 3、Lddqu（SSE3）、Pshufb（SSSE3）が `lift_body` でエラーのまま。
- 変異検査: していない（実施していない旨を明記する）。ただし rsqrtps の全数テストは、表の 1 エントリの変異も検出する（全入力を比べるため）。

## 7. 回帰の確認

- FLEngine（`binaries/win64/FLEngine_x64.dll` を `fe-lift lift ... --pdata`）: `unsupported_total` **531**（基準は 531 以下）、命令 stub 0、`bad_nodes` 0、`functions` 59,297、`emitted_instructions` 2,528,830。**生成 C は実装前の `lift/out-ipp/flengine-pdata.c` とバイト単位で同一**（`cmp`）。FLEngine にこの 25 ニーモニックは出現しないため。
- 全体テスト: `cd lift && CARGO_TARGET_DIR=.../lift/out-w8/target FLENGINE_PE=.../binaries/win64/FLEngine_x64.dll cargo test --workspace --no-fail-fast`: **`test result` 行 87、1,040 件成功、失敗 0**（`lift/out-w8/test-all.txt`。基準は 85 バイナリ・996 件）。増分は +2 行、+44 件。本作業の `sse_legacy` が 1 行・37 件。残りの +1 行・+7 件は他の作業者の追加（ワークツリーに `fe-decode/tests/multisec.rs` が未追跡で存在する。内訳は未確認）。
- 実行の確認: `lift/out-w8/after/ipp-idx0.c`（42.9 MB）を `clang -fsyntax-only -std=gnu11 -I runtime` に通し、エラー 0。

## 8. 限界と未確認

- 公式の SDM は未取得。仕様の根拠は非公式の転載（§3）。
- 実行（`dsp_ippv2_x64.dll` をロードして関数を呼ぶこと）は確認していない。DLL の関数の結果が正しいかは、この決定では検証できない（調査文書 W10）。
- RSQRTPS、RSQRTSS の表は 1 台の AMD CPU（Zen 5）の測定。Intel との一致は未確認。ホスト CPU が変われば表を作り直す必要がありうる（`rsqrtps_all_2e32_inputs_match_host` が失敗して知らせる）。
- Pextrw、Pinsrw の r64 / REX.W 形: SDM は 64 ビット GPR への書き込みで上位をゼロにすると読めるので同じ値を書く。ホストで r32 と r64 の両方を確認した。
- 添字 0 以外の系統（添字 1〜5。SSE3 以降の命令を使う）は未対応のまま。`--seeds` で絞った閉包の数（調査文書 §5.5）を基準にした。
- 数値の基準: 添字 0 の閉包は `lift/out-ipp/table-seeds-idx0.txt` の VA（実在の DLL ではなく、改変コピー `merged-text.dll` に対するもの）。実在の DLL に `--pdata` で通した数ではない（`.text` と `IPPCODE` が別セクションで、他の作業者が `fe-decode` の複数セクション対応を進めている。`lift/crates/fe-decode/tests/multisec.rs`）。
