# 0010: 符号拡張、IDIV、1 オペランドの IMUL

状態: 作業者（M1 作業者）が決定。依頼者（PM）の判断は含まない。実装は `lift/crates/fe-lift/src/int_ext.rs`（新規）と `lift/crates/fe-lift/src/lib.rs`（mod 宣言 1 行と振り分け 3 行）。runtime は `lift/runtime/fle_int_ext.h` と `fle_int_ext.c`（新規）。テストは `lift/crates/fe-lift/tests/int_ext.rs`（新規）。

## 文脈

前回（cpuid-run）の未対応の上位は、Jmp の IAT 経由（435 件）、Movsx（219）、Idiv（157）、Cdq（128）、Cqo（26）、1 オペランドの Imul（132）。今回は、この中の符号拡張と符号付きの除算・乗算を対象にした。

## 仕様の出典

以下はすべて felixcloutier.com の転載（非公式。ページに「No longer updated → use Intel documentation」とある。2026-10-20 取得）。Intel SDM 本体は未取得。

- IDIV: https://www.felixcloutier.com/x86/idiv （読んだ箇所: Operation の `IF SRC = 0 THEN #DE`、商の範囲の判定（8/16/32/64 bit）、剰余の符号は被除数に従う（"The remainder is always less than the divisor in magnitude" と "truncated (chopped) towards 0"）、Flags Affected「CF, OF, SF, ZF, AF, and PF flags are undefined」）
- MOVSX/MOVSXD: https://www.felixcloutier.com/x86/movsx:movsxd （読んだ箇所: `DEST := SignExtend(SRC)`、0F BE（8→16/32/64）、0F BF（16→32/64）、Flags Affected「None」）
- IMUL: https://www.felixcloutier.com/x86/imul （読んだ箇所: 1 オペランド形の Operation（8/16/32/64 bit の TMP_XP と CF/OF の規則）、「The SF, ZF, AF, and PF flags are undefined」）
- CWD/CDQ/CQO: https://www.felixcloutier.com/x86/cwd:cdq:cqo （読んだ箇所: `EDX := SignExtend(EAX)`、`RDX := SignExtend(RAX)`、Flags Affected「None」）
- CBW/CWDE/CDQE: https://www.felixcloutier.com/x86/cbw:cwde:cdqe （読んだ箇所: `AX := SignExtend(AL)`、`EAX := SignExtend(AX)`、`RAX := SignExtend(EAX)`、Flags Affected「None」。REX.W + 98 は CDQE）

## 実装した命令と判断

1. movsx（0F BE / 0F BF、REX.W あり・なし）: 8→16/32/64、16→32/64。宛先の幅の書き方は既存の `reg_write` と同じ（32 bit はゼロ拡張、16 bit は上位を保つ）。フラグは変えない（SDM「None」）。
2. cbw、cwde、cdqe、cdq、cqo: 符号拡張。フラグは変えない（SDM「None」）。cdq は `R[2] = (uint32_t)(0 - (EAX >> 31))`。SDM の「EDX := SignExtend(EAX)」の 32 bit 書き込みの規則（ゼロ拡張）に合わせた。
3. idiv（F6 /7、F7 /7、REX.W 付き）: 被除数は AX、DX:AX、EDX:EAX、RDX:RAX。商と剰余は SDM の表どおり。剰余の符号は被除数に従う（C の `%` と同じ、ホスト突合で確認）。
   - 除数が 0: #DE。
   - 商が宛先の範囲（8/16/32/64 bit の符号付き）を外れる: #DE。
   - 最小値を -1 で割る（w = 64 の 128 bit 型の範囲外になるため）: #DE として扱う。商は範囲外なので SDM でも #DE。
   - #DE の扱い: `fle_div_fault()`（runtime の `fle_int_ext.c`）が stderr に "fle: divide error (#DE)" を出して abort する。abort の前にレジスタは書かない。
   - フラグは全部 `FLE_UNDEF`（SDM「CF, OF, SF, ZF, AF, PF undefined」）。読まれたら既存の仕組みで abort する。
4. imul（1 オペランド、F6 /5、F7 /5、REX.W 付き）: RDX:RAX（EDX:EAX、DX:AX、AX）に積を書く。CF = OF = 上位半分が下位の符号拡張と違うとき 1（SDM の TMP_XP の規則どおり）。SF、ZF、AF、PF は `FLE_UNDEF`。
5. 変えないもの: mul、div（1 オペランド、既存の扱い）、2 と 3 オペランドの imul（`insn_int`/`lib.rs` の既存の扱い）。

## 判断（仕様から言えないこと）

- idiv の 8 bit 形の AH の書き方: 剰余は AH（bit 8〜15）に入る。SDM の表どおり。既存の mov の高バイト扱いとは別の書き込み。
- 0 除算と商の範囲外を abort にすること: 依頼どおり。SDM は #DE という例外を定めるが、FLEngine が #DE を受けるための処理（SEH など）は未確認で、ここでは実行を止めることだけを決めた。
- idiv の 32 bit 形と 16 bit 形の宛先の書き込み: 既存の `lib.rs` の mul/div 相当の書き方（32 bit はゼロ拡張）に合わせた。SDM の Operation は 32 bit の宛先の上位を明記しない。ゼロ拡張は Intel 64 の一般規則（SDM Vol.1、3.4.1.1）からの推測。

## テスト（ホスト突合）

テストファイル `lift/crates/fe-lift/tests/int_ext.rs`。ホストの CPU で同じ命令を実行し、生成 C（clang -O0）の結果と比べる。比べる対象はレジスタの 6 本とバッファ（64 byte）。フラグは比べない（SDM の未定義と、CF/OF だけ定まる規則があるため）。

- movsx: 16 件（境界値、0x80、0x7f、0xff、8/16 bit の宛先、AH/BH の源、メモリ源）。
- 符号拡張（cbw、cwde、cdqe、cdq、cqo）: 14 件（最小値、-1、0、上位ビットの扱い）。
- idiv と 1 オペランド imul: 定義 22 件のうち #DE を含む 1 件を除く 21 件がホスト突合。#DE の 3 件（0 除算、最小値 / -1、8 bit の商の範囲外）は別のテスト（`divide_error_aborts`）で C 側だけを検査。ホストでは SIGFPE になるため。
- 結果: `cargo test --offline -p fe-lift --test int_ext` で 4 件すべて通過（`lift/out-int-ext/test-int-ext-1.txt`）。`cargo test --offline --workspace` は全件通過（`lift/out-int-ext/test-workspace-1.txt`）。

## 実測として言えないこと

- Intel SDM 本体の文言との一致は未確認（転載のみ）。
- 実際の FLEngine のどの分岐が、この実装の結果に依存するかは未確認（今回の実行は 0x407360 で止まり、これより後は読めていない）。
