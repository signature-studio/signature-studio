# 0026: SSE2 の xmm 整数パック命令（sse_int2）

状態: 作業者（サブエージェント）が決定。依頼者（PM）の判断は含まない。

実装は `lift/crates/fe-lift/src/sse_int2.rs`（新規）と `lift/runtime/fle_sse_int2.h`（新規、header only）。
`lib.rs` の変更は `mod sse_int2;` 1 行と、`lift_insn` の `sse_pack::lift` の直後に `if let` 4 行の追加のみ（編集の直前に `lib.rs` を読み直した。差分は +5 行）。
`runtime/fle_sse.h` に、末尾で `#include "fle_sse_int2.h"` する 3 行を追加した（生成 C は `fle_sse.h` だけを include するので、runtime の `.o` を増やさず、`EXTRA_RT` と `lib.rs` のヘッダ文字列を変えずに済む）。
テストは `lift/crates/fe-lift/tests/sse_int2.rs`（新規、28 件）。`imports_table.rs` は触っていない。

## 対象

xmm 形（`66 0F ..`、`F2/F3 0F 70`）だけ。MMX（mm 形）と VEX/EVEX は対象外（`sse_int2::lift` は mm レジスタを含む命令に `None` を返し、VEX 形は iced の別 mnemonic（`Vpaddsb` など）なので拾わない）。

実装した 25 mnemonic: Paddq、Paddsb、Paddsw、Paddusb、Paddusw、Psubsb、Psubsw、Psubusb、Psubusw、Pmaxsw、Pminsw、Pmulhw、Pmullw、Packsswb、Packssdw、Psllw、Pslld、Psllq、Psrlw、Psrld、Psrlq、Psraw、Psrad（各 `xmm, xmm/m128` と `xmm, imm8`）、Pshuflw、Pshufhw。
このうち閉包（896 seed）に出るのは 17 種・91 件。Paddsb、Paddusw、Psubsb、Psubusw、Packsswb、Pslld、Psllq、Psrlq の 8 種は閉包に 0 件だが、同じ形なので同じ作業で実装した。

## 仕様の出典

Intel SDM Vol. 2（Order Number 325383-093US）の公式 PDF。取得は決定 0023 で済んでいて、`lift/out-int2/spec/sdm-vol2.txt`（`pdftotext -layout`）を読んだ（新たな取得はしていない）。読んだのは各命令の「128-bit Legacy SSE Version」の擬似コードと、Other Exceptions の節。図は見ていない。

| 命令 | 根拠（SDM のテキスト） |
|---|---|
| PACKSSWB / PACKSSDW | 「PACKSSWB Instruction (128-bit Legacy SSE Version)」「PACKSSDW ...」。DEST の語（二重語）が下位、SRC が上位。符号付き飽和（7FH/80H、7FFFH/8000H） |
| PADDQ | `DEST[63:0] := DEST[63:0] + SRC[63:0]` と上位 64 ビット（ラップ） |
| PADDSB/W、PSUBSB/W | `SaturateToSignedByte/Word (DEST ± SRC)` |
| PADDUSB/W、PSUBUSB/W | `SaturateToUnsignedByte/Word (DEST ± SRC)`（減算は 0 で飽和） |
| PMAXSW、PMINSW | 擬似コードは `IF DEST > SRC THEN DEST ELSE SRC`（MAX）、`IF DEST < SRC THEN DEST ELSE SRC`（MIN）。符号付き比較 |
| PMULHW、PMULLW | 符号付き 16×16→32 の上位 / 下位 16 ビット |
| PSLL*/PSRL* | `LOGICAL_LEFT/RIGHT_SHIFT_*`: `COUNT := COUNT_SRC[63:0]`。語は COUNT > 15、二重語は > 31、四重語は > 63 で結果は 0 |
| PSRAW / PSRAD | `ARITHMETIC_RIGHT_SHIFT_*`: COUNT > 15 なら COUNT := 16（二重語は 32）。符号拡張のシフト |
| PSHUFLW / PSHUFHW | `DEST[15:0] := (SRC >> (imm[1:0]*16))[15:0]` など。反対側の 64 ビットは SRC のコピー |

xmm/m128 のカウント形: 説明に「If the count operand is a memory address, 128 bits are loaded but the upper 64 bits are ignored」とある。カウントは下位 64 ビット全体を使うので、上位 32 ビットが立っていると「幅以上」になる（テストで `0x1_0000_0000`、`0xffff_ffff_0000_0003` などを使った）。

## 判断

1. **メモリ形の 16 バイト境界**（決定が要った点）。SDM の各命令の Other Exceptions は、非 EVEX は Table 2-21「Type 4 Class Exception Conditions」を指し、そこに `General Protection, #GP(0): Legacy SSE: Memory operand is not 16-byte aligned` がある（脚注 1 で除外されるのは LDDQU、MOVUPD、MOVUPS、PCMPESTRI/M、PCMPISTRI/M だけで、今回の命令は含まれない）。PSLL*/PSRL*/PSRA* の xmm/m128 カウント形（RM 形）も、VEX 形が Type 4 を指す。同じクラスなので、Legacy SSE 形も #GP と読んだ。
   - 実装: メモリオペランドは `fle_ldxa`（16 バイト境界でなければ `fle_misaligned` で stderr に出して abort）で読む。`sse_pack.rs`、`sse_more.rs` と同じ。
   - 実測の裏づけ: ホストで `psllw`、`psrad`、`psrlq`、`paddsw`、`pshuflw`、`psraw` のメモリ形を境界外（オフセット 1、8）で実行すると SIGSEGV（シグナル 11、#GP の結果）、境界内では fault しない（`lift/out-sse2/work/probe_align.c`、`probe_align-out.txt`）。
   - imm8 の PSHUFLW/PSHUFHW のメモリ形（RMI 形）も Type 4 で同じ扱い。
   - テスト: ホストで #GP を起こすケースは突合できないので、C 側だけを実行し、abort と stderr のメッセージ `misaligned 16-byte access`、命令が戻らないことを確認（`misaligned_memory_operand_aborts`、5 命令）。突合の入力のメモリ形は全部 16 バイト境界上。
2. **フラグ**: どの命令も変えない（SDM の Flags Affected: None）。`FL` に触らない。
3. **算術シフトのカウント**: SDM は COUNT > 15 を 16 に丸めて符号拡張でシフトする。C では 15 に丸める（`x >> 15` は符号ビットで埋めた値）。16 で 16 ビット値を符号拡張しながらシフトした結果と同じで、ホスト突合で一致。
4. **レジスタ形のソースがメモリのオペランドを兼ねる場合**: `X[n]` を値で渡すので、`pxxx xmm1, xmm1` でも結果は SDM のとおり（DEST と SRC が同じ値）。テストの入力に `d == s` を含む。
5. **PMAXSW / PMINSW**: 比較は符号付き。等しいときの選択は結果が同じ値なので差が出ない。
6. 上位ビット（MAXVL-1:128）は未変更（このレジスタモデルは xmm の 128 ビットのみ）。

## 実装しなかったもの（fail-closed）

- MMX 形（mm レジスタ）: Psllw 2、Pmulhw 1、Psubsw 1 を含む。別作業。`sse_int2::lift` は `None` を返し、既存どおり `fle_unsupported` のまま（テスト `mmx_and_vex_forms_stay_unsupported`）。
- VEX/EVEX 形: 同上。
- 確認できず実装しなかった命令は **なし**（対象の 25 mnemonic は SDM の擬似コードを読み、ホスト突合で一致した）。
- PSLLDQ / PSRLDQ（バイト単位のシフト）、PSADBW、PMADDWD、PAVGB などは今回の一覧にない。実装していない。

## テスト（ホスト突合）

`cargo test --offline -p fe-lift --test sse_int2`: 28 件（突合 25 + 範囲確認 1 + 境界外 abort 1 + mm/VEX 非対応 1）すべて通過。ログ `lift/out-sse2/test-sse_int2-3.txt`。

方式は `tests/sse_more.rs` と同じ（ホストは実機械語を mmap して実行、C は `lift_body` の本文を clang -O0 でコンパイル）。ホストの命令は intrinsics ではなく、命令バイト列を直接エンコードして実行する。比較は xmm0〜15 全部、rax/rcx/rdx/rbx/rsi、メモリ 64 バイト。

- 二項（15 命令）: ランダム（レーンごとに辺の値と乱数が半々）300 ラウンド × （レジスタ形 + 16 バイト境界上のメモリ形）= 600 + 辺の値の全組合せ（レーン幅ごとのプールの 2 乗: 8 ビット 13² = 169、16/32/64 ビット 21² = 441）。ラウンド 7 回に 1 回は `d == s`（同じレジスタ）。
- シフト（8 命令）: カウント 47 通り（固定 29 通り: 0、幅の前後、63/64/65、0x1_0000_0000 など上位ビットが立つ値、`u64::MAX`。乱数 18 通り）× 3 回 × （xmm カウント、m128 カウント）= 282 + imm8 の 256 通り全部 = 538 ケース。
- PSHUFLW / PSHUFHW: imm8 の 256 通り全部 × （レジスタ形、メモリ形）。
- 各テストで「ホストが xmm を変えたケース」が 80% 以上であることを assert（命令が実際に実行されたことの確認）。
- ケース数: 1 命令あたり 512〜1,041。合計 19,855 ケース（`test-sse_int2-3.txt` の `cases N failures 0`）。

## 確認できていないこと

- Intel の CPU での挙動（ホストは AMD Ryzen 9 9950X。SDM の擬似コードと一致したことは確認）。
- ページ境界をまたぐメモリ形の #PF の扱い（`fle_ldx` は memcpy で、ゲストのページ表を持たない）。
- 実行経路上で、これらの命令が実際に実行されるか（停止点は 0x407030 のまま。run-sse2 の記録を参照）。
