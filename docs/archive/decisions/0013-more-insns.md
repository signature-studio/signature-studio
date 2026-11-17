# 0013: 整数（xadd、cmpxchg、bt、div）と SSE パック系（packuswb、paddw、addps、subps、mulps、minps、maxps）

状態: 作業者（M1 作業者）が決定。依頼者（PM）の判断は含まない。

実装は `lift/crates/fe-lift/src/int_more.rs` と `sse_more.rs`（新規）、`lift/crates/fe-lift/src/lib.rs`（mod 宣言 2 行と振り分け 2 ブロック）。runtime は `lift/runtime/fle_int_more.{h,c}` と `fle_sse_more.{h,c}`（新規）。テストは `lift/crates/fe-lift/tests/int_more.rs` と `sse_more.rs`（新規）。

## 文脈

前回（int-ext）の未対応の上位は、Xadd 56、Bt 53、Cmpxchg 30、Packuswb 27、Paddw 24、Mulps 23、Maxps 20、Div 18 だった。今回はこの中の、仕様を確認できるものを対象にした。

## 仕様の出典

以下はすべて felixcloutier.com の転載（非公式。ページに「No longer updated → use Intel documentation」とある。2026-10-20 取得）。Intel SDM 本体は未取得。

- XADD: https://www.felixcloutier.com/x86/xadd （読んだ箇所: 形式（0F C0/C1、REX.W）、Operation（TEMP := SRC + DEST; SRC := DEST; DEST := TEMP）、Flags Affected「CF, PF, AF, SF, ZF, OF」、LOCK の記述）
- CMPXCHG: https://www.felixcloutier.com/x86/cmpxchg （読んだ箇所: 形式（0F B0/B1）、Operation（accumulator = TEMP なら ZF := 1、DEST := SRC、そうでなければ ZF := 0、accumulator := TEMP、DEST := TEMP）、Flags「ZF」と「CF, PF, AF, SF, OF は比較の結果」）
- BT: https://www.felixcloutier.com/x86/bt （読んだ箇所: 形式（0F A3、0F BA /4 ib）、Operation（CF := Bit(BitBase, BitOffset)）、レジスタ形式の modulo 規則、メモリ形式の「Effective Address + (4 * (BitOffset DIV 32))」、Flags「CF は選択されたビット、ZF は変化なし、OF、SF、AF、PF は未定義」）
- DIV: https://www.felixcloutier.com/x86/div （読んだ箇所: 形式（F6 /6、F7 /6、REX.W + F7 /6）、Operation（IF SRC = 0 THEN #DE、商の範囲外で #DE、剰余は MOD）、Flags「CF, OF, SF, ZF, AF, PF は未定義」、Table 3-15）
- PACKUSWB: https://www.felixcloutier.com/x86/packuswb （読んだ箇所: 形式（66 0F 67）、Operation（レジスタ 128 bit の Legacy SSE 形: DEST[7:0..63:56] に符号付き word の飽和、SRC は [71:64..127:120]）、Flags「None」）
- PADDW: https://www.felixcloutier.com/x86/paddb:paddw:paddd:paddq （読んだ箇所: 形式（66 0F FD）、Operation（レーンごとの加算、オーバーフローは下位ビットを書く）、Flags「None」）
- MULPS: https://www.felixcloutier.com/x86/mulps （読んだ箇所: 形式（0F 59）、Operation（DEST[31:0] := SRC1[31:0] * SRC2[31:0] 等。Legacy SSE 形では上位は変化しない）、SIMD 例外「Overflow, Underflow, Invalid, Precision, Denormal」）
- ADDPS: https://www.felixcloutier.com/x86/addps （読んだ箇所: 形式（0F 58）、Operation（レーンごとの加算））
- SUBPS: https://www.felixcloutier.com/x86/subps （読んだ箇所: 形式（0F 5C）、Operation（レーンごとの減算。SRC1 - SRC2 の順））
- MINPS: https://www.felixcloutier.com/x86/minps （読んだ箇所: 形式（0F 5D）、MIN(SRC1, SRC2) の擬似コード（0.0 同士は SRC2、NaN は SRC2、SRC1 < SRC2 なら SRC1、それ以外 SRC2））
- MAXPS: https://www.felixcloutier.com/x86/maxps （読んだ箇所: 形式（0F 5F）、MAX(SRC1, SRC2) の擬似コード（MINPS と対称））

## 実装した命令と判断

1. xadd（0F C0、0F C1、REX.W 付き）: 宛先（レジスタ、またはメモリ）と元の値を先に読み、和と元の値を書く。フラグは `fle_op_add` が ADD と同じく全 6 本を設定する。
2. cmpxchg（0F B0、0F B1、REX.W 付き）: 比較は `fle_op_sub`（CMP と同じ）。等しければ宛先に元の source を書き、ZF は CMP の結果（1）。等しくなければアキュムレータに宛先の元の値を書き、宛先は元の値のまま書き戻す（SDM の「destination is written back」）。アキュムレータは AL、AX、EAX、RAX（サイズに従う）。
3. bt（0F A3、0F BA /4）:
   - レジスタ base: オフセットは 16、32、64 の剰余（SDM の規則）。
   - メモリ base（レジスタ offset）: オフセットは符号付き。バイト位置は EA + floor(offset / 8)、ビットは offset mod 8。SDM の「Effective Address + (4 * (BitOffset DIV 32))」は、アクセス幅の注記であり、対象ビットの位置の規則ではない。ここでは、バイトの位置を 2 の補数の算術右シフトで求める方式を取った。
   - メモリ base（imm8）: オフセットは剰余で、EA のバイトのビット。
   - フラグは CF のみ設定し、ZF は変えず、OF、SF、AF、PF は `FLE_UNDEF`。
4. div（F6 /6、F7 /6、REX.W 付き）: 被除数は AX、DX:AX、EDX:EAX、RDX:RAX、商と剰余は SDM の表どおり。フラグは全部 `FLE_UNDEF`。
   - 除数が 0: #DE。
   - 商が宛先の範囲を超える: #DE。
   - #DE の扱い: `fle_div_fault()`（runtime の `fle_int_more.c`）が "fle: divide error (#DE)" を stderr に出して abort する。abort の前にレジスタは書かない。
5. packuswb（66 0F 67、xmm と xmm/m128）: 符号付き word の飽和で unsigned byte。MMX 形（NP 0F 67、mm レジスタ）は対象外（`sse_more::lift` が mm を含むものを除外）。
6. paddw（66 0F FD）: 8 レーン、オーバーフローは下位ビット（SDM の wrap）。
7. addps、subps、mulps（0F 58、0F 5C、0F 59）: 各レーンは `sse_more` で `fle_sse.h` の `fle_addss`、`fle_subss`、`fle_mulss` を使う。NaN の扱いは既存のスカラー命令と同じ（SSE の規則）。
8. minps、maxps（0F 5D、0F 5F）: 各レーンは `fle_minss`、`fle_maxss`。SDM の擬似コードどおり、NaN と 0.0 同士は SRC2 を返す。既存のスカラー版と同じ規則。
9. LOCK の扱い: xadd と cmpxchg の LOCK プレフィックスは無視する。理由: FLEngine は単一スレッドで動く想定（lift の既存の方針と同じ。ただし、FLEngine が別スレッドを使わないことは、ここでは未確認の前提）。LOCK 付きの命令は、既存の生成でもプレフィックスを区別していない（iced の命令として同じ mnemonic）。
10. フラグの扱い: packuswb、paddw、addps、subps、mulps、minps、maxps はフラグを変えない。xadd と cmpxchg は ADD、CMP と同じ。bt と div は上記のとおり。

## 仕様を確認できず実装しなかった命令

今回の範囲（上位の一覧と、閉包の残り）で、仕様を確認していないため実装しなかったもの:

- Cmpxchg8b、Cmpxchg16b（対象外。依頼どおり）。
- Shufps、Pshuflw、Pshufhw、Pshufw、Psllw、Psrlw、Psraw、Psrld、Psrad、Pmulhw、Pmullw、Pmulhuw（既に対応済みの pmulhuw を除く）、Paddusb、Paddsw、Psubsw、Psubusb、Pminsw、Pmaxsw、Packssdw、Punpcklbw（SSE 形）、Movhlps、Movlhps、Movlps、Movd（MMX 形と SSE 形の混在）、Movq（MMX 形）、Pxor（mm 形）、Paddq、Emms、Xchg、Bts、Btr、Bsf、Rol、Ror、Vmovups、Vmovaps、Vmovntps、Vzeroupper、Sfence、Pause、Xgetbv、Fldcw、Fnstcw、Fnclex、Fninit、Fnstsw、Psrld、Jecxz、Cmpxchg16b、Minps（ただし上の 8 で実装済み）。

上記のうち Bts、Btr、Bsf、Xchg、Rol、Ror、Movhlps、Movlhps は、今回は仕様を確認していない（実装していない。次の作業で確認する対象）。Vmovups、Vmovaps、Vzeroupper は VEX 符号化で、本ワークの範囲外。Xgetbv、Fldcw、Fnstcw などは x87 とシステム命令で、扱いは未決。

## テスト（ホスト突合）

- `cargo test --offline -p fe-lift --test int_more`: 4 件（xadd、cmpxchg、bt、div のホスト突合）と、divide_error_aborts（C 側のみ、#DE の abort 検査）の計 5 件。すべて通過。
- `cargo test --offline -p fe-lift --test sse_more`: 7 件（packuswb、paddw、addps、subps、mulps、minps、maxps の各ホスト突合）。すべて通過。ホスト突合は 338 件（addps、subps、mulps、minps、maxps 各 48、packuswb と paddw 各 49）。
- フラグは比べない（SDM の未定義と、CF/OF だけ定まる規則があるため）。比べるのは、レジスタ（rax、rcx、rdx、rbx、rsi、rdi を除く）、XMM（SSE のみ）、メモリ 64 byte。

## 実測として言えないこと

- Intel SDM 本体の文言との一致は未確認（転載のみ）。
- Bt の Bit() の定義（SDM Vol. 1 の 3-11）は、検索で存在は確認したが本文は取得していない。メモリ形式の規則は、ホスト突合の結果による。
- 実際の FLEngine のどの分岐が、この実装の結果に依存するかは未確認。
