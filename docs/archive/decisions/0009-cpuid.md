# 0009: CPUID の応答

状態: 作業者（M1 作業者）が決定。依頼者（PM）の判断は含まない。実装は `lift/crates/fe-lift/src/cpuid.rs`（`lib.rs` の変更は mod 宣言 1 行と振り分け 4 行）、runtime は `lift/runtime/fle_cpuid.h` と `fle_cpuid.c`（新規）。テストは `lift/crates/fe-lift/tests/cpuid.rs`。

## 文脈

前回の停止点は 0x40f47b の `cpuid`（ラッパー 0x40f470。呼び出し元は 0x40f490 ← 0x40f4b0 ← 0x41ce10）。0x40f4b0 は葉 0 から 7 までを ECX=0 で順に取る（`objdump-text.txt` の 0x40f470 から 0x40f4e7 まで）。0x40f470 では EAX=EDX（入力の葉）、ECX=R8D（入力のサブ葉）、結果は [R9] から 16 バイトずつ書く。

実行時の葉（probe の結果。`lift/out-cpuid/run-probe.err`）:

| 入力 EAX | 入力 ECX | probe で取った host の EAX/EBX/ECX/EDX |
|---|---|---|
| 0 | 0 | 0000000d, 68747541, 444d4163, 69746e65 |
| 1 | 0 | 00b40f40, 0c100800, fed83203, 178bfbff |
| 2 | 0 | 0, 0, 0, 0 |
| 3 | 0 | 0, 0, 0, 0 |
| 4 | 0 | 0, 0, 0, 0 |
| 5 | 0 | 0, 0, 0, 0 |
| 6 | 0 | 0, 0, 1, 0 |
| 7 | 0 | 1, f1bf07a9, 00405fc6, 00000110 |

注: この表は probe（計測用）が実行したホストの値である。生成 C の応答には使っていない（後述）。葉 0 の EBX:EDX:ECX は `68747541 69746e65 444d4163` の並びで、ASCII にすると "AuthenticAMD" である（実測）。つまり実行ホストは AMD 系 CPU だった。ホストの型番は未確認。

## 仕様の出典

- CPUID: https://www.felixcloutier.com/x86/cpuid （非公式の転載。ページに「No longer updated → use Intel documentation」とある。2026-10-20 取得）
  - 読んだ箇所:
    - 「CPUID returns processor identification and feature information in the EAX, EBX, ECX, and EDX registers. The instruction's output is dependent on the contents of the EAX register upon execution (in some cases, ECX as well).」
    - 「If a value entered for CPUID.EAX is higher than the maximum input value for basic or extended function for that processor then the data for the highest basic information leaf is returned.」
    - 「If a value entered for CPUID.EAX is less than or equal to the maximum input value and the leaf is not supported on that processor then 0 is returned in all the registers.」
    - 「On Intel 64 processors, CPUID clears the high 32 bits of the RAX/RBX/RCX/RDX registers in all modes.」
    - Table 3-8 の葉 0（EAX=最大の基本入力値、EBX/EDX/ECX=ベンダー文字列 "Genu" "ineI" "ntel" の並び）、葉 1（EAX=版、EBX、ECX と EDX=機能ビット）。
    - 機能ビットの表（Table 3-11 相当）: EDX の 25 は「SSE. The processor supports the SSE extensions.」、26 は「SSE2. The processor supports the SSE2 extensions.」。ECX の SSE3 は「A value of 1 indicates the processor supports this technology.」（取得した同じ転載）。
- 公式 PDF（Intel SDM Vol. 2 の CPUID）は未取得。felixcloutier の転載の内容が公式と同じかは未確認。

## 確認できなかった点

- 公式 PDF の本文との一致（未取得）。
- 0x80000000 以上の拡張葉の最大値。このプロセッサの拡張の最大葉は、仕様からは決まらない（SDM は「その CPU による」とする）。
- EAX の入力が 0x80000000 以上で、基本葉の最大より大きいときの規則。felixcloutier の転載の「highest basic information leaf」の規則は、基本葉に対する規則。拡張葉の規則は未確認。
- 葉 1 の EBX（ブランド、APIC ID、CLFLUSH 行サイズ）と ECX（SSE3 以降などの機能ビット）の値を 0 にした理由は、仕様からは決まらない。下の「判断」を参照。

## 決定

1. 応答は `runtime/fle_cpuid.c` の `fle_cpuid()` が返す。入力は R[0]（EAX）、R[1]（ECX）。出力は R[0]（EAX）、R[3]（EBX）、R[1]（ECX）、R[2]（EDX）。
2. 最大の基本葉は 1。
3. 葉 0: EAX=1、EBX=ECX=EDX=0（ベンダー文字列なし）。
4. 葉 1: EAX=0、EBX=0、ECX=0、EDX=bit 25（SSE）と bit 26（SSE2）のみ。
5. 葉 2 から 0x7fffffff: 葉 1 のデータを返す（仕様の「最大の基本葉のデータ」の規則）。
6. 葉 0x80000000 以上: 全レジスタ 0。
7. 命令は `cpuid` の 0 オペランドのみを受ける（既存の振り分けの挙動は変えない。`lib.rs` の振り分けは、cpuid を mxcsr の次に 1 つ追加しただけ）。

## 判断の根拠（仕様から言えること／言えないこと）

- 仕様から言えること: 葉 0 の EAX は最大の基本葉。最大葉を超える入力は最大の基本葉のデータを返す。Intel 64 では上位 32 bit が 0。
- 仕様から言えないこと（作業者が決めた）:
  - 最大の基本葉を 1 にすること。仕様は値を強制しない。1 は作業者の選択で、根拠は「葉 1 までの機能を主張し、それ以上は主張しない」という最小の論理（作業者の推測）。FLEngine が葉 1 のどのビットを必要とするかは、今回の実行では確認していない。
  - ベンダー文字列を空にすること。ベンダーを主張しない。仕様は CPU の製造元を指定しない。ホストの値（AuthenticAMD）を使うと、特定の CPU を前提にすることになる。仕様から決まらないため、ホストの値は使わない。
  - SSE と SSE2 のみを立てること。根拠は fe-lift が SSE/SSE2 の命令を生成すること（sse_scalar.rs、sse_pack.rs）。SSE3 以降と AVX は立てない。根拠は「lift が対応していない機能を主張しない」。仕様からではない。
  - 拡張葉を 0 にすること。最大の拡張葉は仕様から決まらないため、未確認。
- 実測として言えること: FLEngine の経路は葉 0 から 7（ECX=0）を取りに来る（probe で確認）。
- 実測として言えないこと: 取った値のどのビットを使うか。今回の実行は葉 2 以降の結果を読む前（0x407360 の import thunk）で止まったため、葉 1 や葉 7 の各ビットがどの分岐に効くかは確認していない。「最小の値」は、この時点の推測であり、確認済みではない。

## ホストの値を使わない理由

probe の結果（ホストの値）は計測のための情報で、応答に使わない。理由: ホストの値は特定の CPU を前提にする（ベンダーや機能ビット）。仕様から決められない値を、ホストの値で決めると、「仕様から言えないこと」を事実として書くことになる。

## テスト

- `cpuid_body_calls_runtime`: 0f a2 の本体が `fle_cpuid()` の呼び出しであること。
- `cpuid_response_matches_table`: 生成 C を clang -O0 でコンパイルして実行し、上の決定の表（入力 10 件）と一致すること。
