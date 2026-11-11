# 0019: x87 制御命令（FNSTCW、FLDCW、FNSTSW、FNCLEX、FNINIT、FWAIT）の応答

状態: 作業者が決定。依頼者の決定は含まない。実装は `lift/crates/fe-lift/src/x87ctl.rs`（振り分けは `lib.rs` の mod 宣言 1 行と `if let` 4 行）、runtime は `lift/runtime/fle_x87ctl.h` と `fle_x87ctl.c`（新規）。

## 文脈

前回（`lift/out-locale/run-locale-896.txt`）の停止点は 0x40a332 の `fnstcw WORD PTR [rsp]`。f_40a330 は `push 0; fnstcw [rsp]; pop rax; ret` で、呼び出し元 f_40f500 の 0x40f50e が呼ぶ。直後の 0x40f513 は `and ax, 0x1f3f`。未対応の種類は x87 命令（import ではない）。

閉包（896 seed）の生成 C（`lift/out-locale/out/after-locale-896.c`）の未対応 625 件のうち、x87 は次の 10 件だけ（`/* VA Mnemonic */` のコメントを集計した実測）。

| ニーモニック | 件数 | VA |
|---|---|---|
| Fldcw | 4 | 0x40a329, 0x63a662, 0x63a670, 0x63a680 |
| Fnclex | 2 | 0x40a327, 0x63a660 |
| Fnstcw | 2 | 0x40a332, 0x6398df |
| Fninit | 1 | 0x6398d3 |
| Fnstsw | 1 | 0x6398d5（`fnstsw WORD PTR [rbp+0xc]`） |

演算系（fld、fstp、fmul など）は 0 件。Fwait/Wait も 0 件。Emms は 3 件あるが、これは MMX で x87 の制御ではない（対象外、未対応のまま）。

## 仕様の出典

- felixcloutier.com（Intel SDM の非公式の転載。各ページに「UNOFFICIAL, mechanically-separated, non-verified」とある。2026-10-20 取得）
  - FLDCW https://www.felixcloutier.com/x86/fldcw : 「Loads the 16-bit source operand into the FPU control word」。Operation は `FPUControlWord := SRC`。保留中の例外フラグを新しい制御ワードが unmask すると、次の「waiting」x87 命令で例外が発生しうる。FLDCW 自体は例外なし。C0-C3 は undefined。
  - FNSTCW https://www.felixcloutier.com/x86/fstcw:fnstcw : `DEST := FPUControlWord`。FNSTCW は保留中の unmask された例外を確認しない。Floating-Point Exceptions は None。
  - FNCLEX https://www.felixcloutier.com/x86/fclex:fnclex : `FPUStatusWord[0:7] := 0; FPUStatusWord[15] := 0`。PE、UE、OE、ZE、DE、IE、ES、SF、B をクリア。C0-C3 は undefined。
  - FNINIT https://www.felixcloutier.com/x86/finit:fninit : `FPUControlWord := 037FH; FPUStatusWord := 0; FPUTagWord := FFFFH; FPUDataPointer := 0; FPUInstructionPointer := 0; FPULastInstructionOpcode := 0`。C0-C3 は 0。データレジスタは変えず、すべて empty（11B）にする。
  - FNSTSW https://www.felixcloutier.com/x86/fstsw:fnstsw : `DEST := FPUStatusWord`。宛先は m16 または AX。C0-C3 は undefined。
- SDM Vol. 1 §8.1.5「x87 FPU Control Word」（xem.github.io/minix86 の転載のページ 197、https://xem.github.io/minix86/manual/intel-x86-and-64-manual-vol1/o_7281d5ea06a5b67a-197.html ）: FINIT/FNINIT で制御ワードは 037FH（全例外 masked、最近偶数、精度 64 bit）。ビット 0-5 が例外マスク、ビット 8-9 が PC。
- 公式 PDF（Intel の SDM）は取得していない（アーカイブの全文テキストは 5 MB 超で取得できなかった）。転載の内容と公式の一致は未確認。
- 制御ワードの予約ビットの扱い（ビット 6、7、13-15 と、ビット 12 の Infinity Control）は、取得した範囲の仕様の文面からは確認できなかった。下の実測で決めた。

## 実測（ホスト AMD Ryzen 9 9950X、Linux x86_64）

`lift/out-x87ctl/work/probe.c`、`probe2.c`、`probe3.c`。

- 起動時（Linux のプロセス）の制御ワード 037F、ステータス 0。
- FLDCW を 65536 通り全部試して FNSTCW で読み戻すと、すべて `(v & 0x1F3F) | 0x0040` と一致（不一致 0）。つまり読み戻しでは、ビット 6 は常に 1、ビット 7 と 13-15 は常に 0、ビット 0-5 と 8-12 は書いた値。FLDCW が #GP や他のフォールトを起こした値はなかった。
- FNINIT で制御ワード 037F、ステータス 0。FNSTENV でタグ FFFF。
- マスクされた 0 除算の後のステータスは 0x0004（ZE）、比較の後は C0 が立って 0x0100。FNCLEX は ZE をクリアし、C0 は残す（0x0104 → 0x0100）。仕様どおり。

## 決定

1. x87 の状態は runtime の 1 つの構造体 `fle_x87`（`cw`、`sw`、`tw`、各 uint16_t）。初期値は 037FH、0、FFFFH。
2. FNSTCW m16: `cw` を m16 に 16 bit 書く。上位 16 bit の隣接メモリは変えない。
3. FLDCW m16: `cw = (v & 0x1F3F) | 0x0040`。abort しない。仕様は予約ビットで #GP を起こすと書いていない（取得した範囲で。mxcsr と違う）ので、abort しない。この値は上の実測（65536 通りの全数）に合わせた。「予約ビットの値が保持される」とは書けない。ホストでは保持されない（推測ではなく実測）。
   - 精度制御（ビット 8-9）と丸め制御（ビット 10-11）が既定から変わる値も、そのまま `cw` に保持して FNSTCW で報告する。x87 演算は未実装なので、演算には反映されない。
4. FNSTSW m16 と FNSTSW AX: `sw` を書く。AX の場合は RAX の上位 48 bit を残す。
5. FNCLEX: `sw &= ~0x80FF`（ビット 0-7 とビット 15）。C0-C3（ビット 8-10、14）と TOP（11-13）は変えない（仕様は C0-C3 を undefined としているので、変えない選択は仕様の範囲内。ホストの実測とも一致）。
6. FNINIT: `cw = 037FH; sw = 0; tw = FFFFH`。データポインタ、命令ポインタはモデルを持たない。
7. FWAIT（Wait）と FSTCW、FSTSW、FCLEX、FINIT の wait 形も同じハンドラに送る。理由: ステータスワードに例外フラグを立てる命令がない（x87 演算が未実装）ので、確認する保留例外が常にない。閉包に出現しないので、実測の対象外。単体テストは FWAIT の本文が空であることだけを確認した。

## 仕様から言えること／言えないこと

- 言える（転載の仕様）: 既定値 037FH、FNINIT の状態、FNCLEX のクリアするビット、FNSTSW の宛先。
- 言える（実測）: FLDCW の読み戻し規則、FNCLEX と FNINIT の効果。上の 65536 通りの全数とテスト。
- 言えない（未確認）:
  - 公式 PDF との一致。
  - FLDCW が保留例外を unmask した場合の例外発火（仕様にあるが、モデルに保留例外が存在しないので再現しない）。
  - AMD 以外の CPU の予約ビットの読み戻し（Intel CPU で同じ規則かどうかは未確認）。

## 仕様の外（作業者の判断）

- x87 演算（fld、fstp、fmul など）は lift しない。到達すれば既存どおり `fle_unsupported` で止まる（黙って近似しない）。閉包 896 seed では 0 件。
- FNSTENV、FNSAVE、FLDENV、FRSTOR は lift しない（テスト `x87_arithmetic_stays_unsupported` で未対応のままを確認）。
- 制御ワードの精度制御、丸め制御は、演算に反映しない。反映する演算がない。
