# 0027: SSE 浮動小数のシャッフルと半分移動（sse_shuf）

状態: 作業者（サブエージェント）が決定。依頼者（PM）の判断は含まない。

実装は `lift/crates/fe-lift/src/sse_shuf.rs`（新規）と `lift/runtime/fle_sse_shuf.h`（新規、header only）。
`lib.rs` の変更は `mod sse_shuf;` 1 行と、`lift_insn` の `sse_int2::lift` の直後に `if let` 4 行の追加のみ（編集の直前に `lib.rs` を読み直した。`git diff --stat` で +5 行）。`runtime/fle_sse.h` は末尾に `#include "fle_sse_shuf.h"` を 1 行追加（+1 行）。`imports_table.rs` は触っていない。
テストは `lift/crates/fe-lift/tests/sse_shuf.rs`（新規、17 件）。

## 対象

xmm 形（レガシー SSE エンコード）だけ。VEX/EVEX 形は iced の別 mnemonic（`Vshufps`、`Vmovlps` など）なので拾わない（テスト `vex_forms_stay_unsupported`）。

| mnemonic | 形 | 閉包（896 seed）の件数 |
|---|---|---|
| Shufps | `xmm, xmm/m128, imm8` | 13（すべて `xmm_n, xmm_n, 0x0` または `0x1` のレジスタ形） |
| Movlps | `xmm, m64`（ロード）、`m64, xmm`（ストア） | 8（ロード 7、ストア 1。すべてメモリ形） |
| Movhlps | `xmm, xmm` | 3 |
| Movlhps | `xmm, xmm` | 2 |
| 計 | | 26 |

閉包に出ない近縁で、同じ形で SDM の擬似コードから確実に書ける 8 種も同時に実装した: Shufpd、Unpcklps、Unpckhps、Unpcklpd、Unpckhpd、Movhps、Movlpd、Movhpd。実装前に `sse_pack.rs`、`sse_more.rs`、`sse_scalar.rs`、`sse_int2.rs` を grep して、これらの mnemonic（Movhps など `Mov*p*` を含む）が既に実装されていないことを確認した（該当なし）。重複はない。合計 12 mnemonic。

## 仕様の出典

Intel SDM Vol. 2（Order Number 325383-093US）。取得は決定 0023 で済んでいて、`lift/out-int2/spec/sdm-vol2.txt`（`pdftotext -layout`）を読んだ（新たな取得はしていない）。読んだのは各命令の「128-bit Legacy SSE」の擬似コードと、Other Exceptions の節、Table 2-21（Type 4）、Table 2-22（Type 5）。図は見ていない。

| 命令 | 根拠（SDM のテキスト） |
|---|---|
| SHUFPS | `Select4(SRC, control)`。`DEST[31:0] := Select4(SRC1, imm8[1:0])`、`[63:32]` は `imm8[3:2]`、`[95:64] := Select4(SRC2, imm8[5:4])`、`[127:96]` は `imm8[7:6]`。SRC1 は DEST |
| SHUFPD | `IMM0[0]` で SRC1 の低/高 64 ビットを DEST[63:0] へ、`IMM0[1]` で SRC2 の低/高を DEST[127:64] へ。「imm8[7:2) are ignored」 |
| UNPCKLPS | `DEST[31:0] := SRC1[31:0]`、`[63:32] := SRC2[31:0]`、`[95:64] := SRC1[63:32]`、`[127:96] := SRC2[63:32]` |
| UNPCKHPS | `DEST[31:0] := SRC1[95:64]`、`[63:32] := SRC2[95:64]`、`[95:64] := SRC1[127:96]`、`[127:96] := SRC2[127:96]` |
| UNPCKLPD / UNPCKHPD | `DEST[63:0] := SRC1[63:0]`（HPD は `[127:64]`）、`DEST[127:64] := SRC2[63:0]`（HPD は `[127:64]`） |
| MOVHLPS | `DEST[63:0] := SRC[127:64]`、上位は未変更。「ModRM.MOD = 011B required」（レジスタ形のみ） |
| MOVLHPS | `DEST[127:64] := SRC[63:0]`、下位は未変更。レジスタ形のみ |
| MOVLPS / MOVLPD ロード | `DEST[63:0] := SRC[63:0]`（SRC は m64）、`DEST[MAXVL-1:64]` は未変更 |
| MOVHPS / MOVHPD ロード | `DEST[63:0]` 未変更、`DEST[127:64] := SRC[63:0]` |
| MOVLPS / MOVLPD ストア | `DEST[63:0] := SRC[63:0]`（DEST は m64） |
| MOVHPS / MOVHPD ストア | `DEST[63:0] := SRC[127:64]` |

## 判断

1. **メモリ形の境界（決定が要った点。SDM で確認した）**。
   - SHUFPS、SHUFPD、UNPCKLPS、UNPCKHPS、UNPCKLPD、UNPCKHPD の m128 形: Other Exceptions は「Type 4 Class Exception Conditions」（Table 2-21）を指す。Table 2-21 に `General Protection, #GP(0): Legacy SSE: Memory operand is not 16-byte aligned` があり、脚注 1 で除外されるのは LDDQU、MOVUPD、MOVUPS、PCMPESTRI/M、PCMPISTRI/M だけで、これらは含まれない。実装は `fle_ldxa`（16 バイト境界でなければ `fle_misaligned` で stderr に出して abort）。`sse_int2.rs` と同じ。
   - MOVLPS、MOVHPS、MOVLPD、MOVHPD の m64 形: Other Exceptions は「Type 5 Class Exception Conditions」（Table 2-22、見出し「<16 Byte Mem Arg and No FP Exceptions」）を指す。Table 2-22 には、16 バイト境界の #GP の行がない（#GP(0) は、セグメント、非カノニカルアドレス、実効アドレス空間の外だけ）。#AC(0) は「2, 4, or 8 byte memory access if alignment checking is enabled ... CPL 3」で、AC フラグが無効なら出ない。したがって任意のアドレスで動くとして、`fle_ld(a, 8)` / `fle_st(a, 8, v)` で、境界を確認しない（#AC はゲストの EFLAGS.AC と CR0.AM を持たないため再現しない。この点は下の「確認できていないこと」）。
   - MOVHLPS、MOVLHPS: メモリオペランドがない（レジスタ形のみ）。メモリ形のエンコード（`0F 12 /r` の mod != 11 は MOVLPS、`0F 16 /r` の mod != 11 は MOVHPS）は iced が別の mnemonic（Movlps、Movhps）にデコードするので、混ざらない。`regreg` はソースがレジスタでない場合にエラーにする。
2. **フラグ**: どれも変えない（SDM に Flags Affected の記載なし。SIMD Floating-Point Exceptions: None）。`FL` に触らない。
3. **浮動小数点演算をしない**: すべてビットのコピー。NaN（signaling を含む）のペイロード、符号、±0、非正規化数はそのまま残る。MXCSR を参照しない。
4. **半分だけ書く命令の他方の半分**: MOVLPS/MOVLPD ロードと MOVHLPS は `X[d].u64[0]` だけ、MOVHPS/MOVHPD ロードと MOVLHPS は `X[d].u64[1]` だけを書く（SDM の「Unmodified」）。`sse_pack.rs` の MOVQ のようにゼロ拡張しない。
5. **レジスタ形で `d == s`**（閉包の Shufps 13 件すべて、Movlhps の 1 件など）: `X[n]` を値で渡すので SDM のとおり（DEST と SRC が同じ値）。テストの入力に `d == s` を含む。
6. 上位ビット（MAXVL-1:128）は未変更（このレジスタモデルは xmm の 128 ビットのみ）。
7. 一部の mnemonic の実装を、`sse_pack.rs` の `unpack`（整数）には混ぜず、別ファイルにした。依頼の指定どおり。

## 実装しなかったもの（fail-closed）

- VEX/EVEX 形（Vshufps、Vmovlps、Vmovhlps、Vunpcklps など）: 別作業。
- 確認できず実装しなかった命令は **なし**（12 mnemonic すべて SDM の擬似コードを読み、ホスト突合で一致した）。
- MOVMSKPS、MOVNTPS などの近縁は今回の一覧にない。実装していない。

## テスト（ホスト突合）

`cargo test --offline -p fe-lift --test sse_shuf`: 17 件すべて通過（ログ `lift/out-shuf/test-sse_shuf-2.txt`）。内訳: 突合 13（12 命令 + 閉包のレジスタ形エンコード 9 種）、ケース集合の範囲確認 1、m128 境界外が abort する 1（6 命令）、m64 の境界外が abort しない 1（4 命令 × ロード/ストア）、VEX 形が未対応のまま 1。

方式は `tests/sse_int2.rs` と同じ（ホストは実機械語を mmap して実行、C は `lift_body` の本文を clang -O0 でコンパイル）。比較は xmm0〜15 全部、rax/rcx/rdx/rbx/rsi、メモリ 64 バイト。ホストの命令は intrinsics ではなく、命令バイト列を直接エンコードして実行する。

- SHUFPS、SHUFPD: imm8 の 256 通り全部 × レジスタ形・16 バイト境界上のメモリ形 + ランダムな imm8 の追加 300 ラウンド = 各 1,112 ケース。
- UNPCK 4 命令: 400 ラウンド × （レジスタ形、メモリ形）= 各 800 ケース。
- MOVHLPS、MOVLHPS: 各 600 ケース（7 回に 1 回は `d == s`）。
- MOVLPS、MOVHPS、MOVLPD、MOVHPD: ディスプレースメント -16〜+40 の全 57 通りを 3 周（171 ラウンド）× （ロード、ストア）= 各 342 ケース。16 バイト境界上と境界外の両方を含み、ホストと一致（m64 は境界要求なしの裏づけ）。
- 閉包の逆アセンブルにあるレジスタ形のバイト列 9 種（Movlhps、Movhlps、Shufps）を 40 ケースずつ = 360 ケース。メモリ形（`[rcx]`、`[rbx+8]` など）は rcx/rbx/rdx が突合のハーネスでランダム値のため、同じアドレス生成コードを通る `[rdi+disp]` の形で突合した。
- 入力: 辺の値（±0、±1、±inf、quiet/signaling NaN とペイロード、非正規化数、最大値、π）、乱数、全ダブルワードが互いに異なる値（レーン違いが出力に出る）。
- 各テストで「ホストが xmm かメモリを変えたケース」が 80% 以上であることを assert（命令が実際に実行されたことの確認）。実測は全命令で 97% 以上（最小は Shufpd の 1,085/1,112 = 97.6%。他は 99.7% 以上）。
- ケース数の合計: 8,352（`test-sse_shuf-2.txt` の `cases N failures 0` の和）。失敗 0。
- 変異テスト: `fle_shufps` の `(imm >> 4)` を `(imm >> 5)` に変えて shufps のテストを実行すると、1,112 ケース中 813 件が失敗した（`test-sse_shuf-mutant.txt`、EXIT=101。検出できる）。その後に元へ戻して全 17 件通過を再確認した（`test-sse_shuf-2.txt`）。
  - 1 回目の変異の実行は「通過」した（`test-sse_shuf-mutant.txt` は 2 回目で上書きされ、残っていない）。原因は**推測**: 変異の取り消しの編集を、テストの実行と同じ並列ブロックで発行したので、clang がヘッダを読む前に取り消しが反映された可能性が高い（実行順の誤り）。確認はしていない。変異を入れ直し、取り消しを別の手順にした 2 回目で 813 件の失敗を確認した。1 回目の結果は検出能力の根拠に使っていない。

## 確認できていないこと

- Intel の CPU での挙動（ホストは AMD Ryzen 9 9950X。SDM の擬似コードと一致したことは確認）。
- m64 形の #AC（EFLAGS.AC=1 かつ CR0.AM=1 のユーザーモード）。ゲストの EFLAGS.AC を持たないので再現しない。
- ページ境界をまたぐメモリ形の #PF の扱い（`fle_ld`/`fle_ldx` は memcpy で、ゲストのページ表を持たない）。
- 実行経路上で、これらの命令が実際に実行されるか（停止点は 0x407030 のまま。実行記録 `docs/investigations/sse-shuf-run.md` を参照）。
