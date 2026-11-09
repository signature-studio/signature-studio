# A5 オラクル: 不一致の一覧

日付: 2026-10-20。設計は `docs/decisions/0035-a5-oracle.md`、実行は `docs/investigations/a5-oracle-run.md`。

## 結果（実測、`lift/out-oracle/results/run3/a5-summary.json`）

200 関数、2,400 case（各 12）のうち、**`mismatch` は 0 件**、`lift_unsupported` は 0 件、`lift_stop_other` は 0 件。

| 判定 | case 数 |
|---|---|
| bit_exact | 1,457 |
| mismatch | 0 |
| lift_unsupported | 0 |
| lift_stop_other | 0 |
| native_fault（比較不能） | 943（SIGSEGV 933、タイムアウト 10） |

このため、最小例（関数 VA、入力、差分のあるレジスタ/メモリ、該当する lifter の命令ハンドラの推定）として挙げるものはない。

## 補足（不一致ではないが、読み違えやすい点）

1. **native_fault の 943 件で、lift 側も同じ挙動だった（実測）**。933 件は両側とも SIGSEGV で、fault アドレスも同じ。10 件は両側ともタイムアウトで、すべて関数 0x4104e0 の case（2 秒内に終わらない、両側で同じ）。これは「比較不能」に分類してあり、一致の件数には入っていない。ただし「lift が native と同じ場所で落ちた」ことは、無効なポインタを読む命令のアドレス計算が両側で同じだったことを示す。
2. **ネガティブコントロールの検出は 29/60（全体）**。定数返却関数の反転 3/3 は検出（rax が変わる）。本体の乱数位置の反転 57 件のうち検出は 26、未検出は 31（内訳: 全 case が bit_exact のまま 17、native が fault する case が残って比較できた case は一致 14）。未検出 17 件のバイトが実行された命令かどうかは調べていない（推測: 実行されない経路、または結果に出ない flag だけを変えるバイトに当たった）。つまり、このオラクルは「バイトの 1 つの変化が、この case 集合のもとで出力に現れれば」検出する。出力に現れない変化は検出できない。
3. **比較していないもの**: EFLAGS、x87、MXCSR、MMX 状態、GS/TEB。これらの違いはこの実行では検出できない（`0035` の「比較しないもの」）。
4. **対象の偏り**: 選別された純計算関数は小さく（命令数の中央値 5）、200 関数中、他の関数を直接呼ぶものは 55。無作為の 200 関数には、本体に `fle_unsupported` を含む関数が **0 件**だった（実測: 選別 9,901 件のうち本体に `fle_unsupported` を持つのは 21 件で、無作為選択に入らなかった）。そのため無作為の 200 関数では `lift_unsupported` が 0 件になる。

## 追加実行: lift 側に `fle_unsupported` を持つ純計算関数 21 件（`results/run-unsupported/`）

目的は、「lift が実行不能」の分類が働くことの確認と、実行できた範囲での不一致の有無。選別 9,901 件のうち、生成 C の本体（`L_` 行）に `fle_unsupported` を含む 21 件を全部走らせた（`results/pure-unsupported.json`、各 12 case = 252 case）。

| 判定 | case 数 |
|---|---|
| bit_exact | 7 |
| mismatch | **0** |
| lift_unsupported | 42 |
| lift_stop_other | 0 |
| native_fault（比較不能） | 203（native と lift が同じ fault アドレスの SIGSEGV が 98、native は SIGSEGV で lift は先に unsupported で abort が 105） |

`lift_unsupported` の 42 case は 11 か所の命令（実測。`out-pdata/out/after-pdata-b-part-*.c` の `L_<va>` 行のニーモニック）:

| VA | 命令 | 関数が `lift_unsupported` になった case 数 |
|---|---|---|
| 0x478506 | Bswap | 4 |
| 0x69d66c、0x69d7c7 | Prefetcht0 | 1、3 |
| 0x6a03ae、0x6a03fd | Cvtdq2ps | 2、1 |
| 0x6a4915 | Addpd | 1 |
| 0x6ae122、0x6ae132 | Bswap | 12、12 |
| 0x9f2257、0x9f22e2、0x9f2399 | Psubw | 2、2、2 |

これは lifter のバグではなく、「未実装の命令（`fle_unsupported`）に当たった」という既知の状態（`docs/investigations/remaining-unsupported.md` の系統）。lift が実行できた case で、native と違った結果は 0 件。

## 追記（v2、run4）: 1,000 関数 × 16 case（`results/run4/`、`a5-oracle-run2.md`、決定 0036）

実測: 16,000 case のうち bit_exact 12,805、**mismatch_core 0**（GPR/XMM/メモリの差なし）、**mismatch_aux 717**（すべて MXCSR 状態ビットのみ）、lift_unsupported 13、native_fault 2,465。EFLAGS の定義済みフラグの差 0、x87（cw/sw/tw）の差 0、MXCSR 制御ビットの差 0。

### mismatch_aux 717 件: MXCSR の状態ビット（PE/UE/OE/ZE/DE/IE）

- 内容: native は SSE 演算で MXCSR の状態ビットを立て、lift の `fle_mxcsr` は常に 0x1F80。717 case、62 関数。native が立てたビットの case 数（重複あり）: PE 637、DE 270、IE 141、UE 108、OE 52、ZE 17。
- 推定される lifter/runtime 側の原因（推測ではなく確認した事実 + 推測）: 事実 — `lift/runtime/fle_sse.h` に `fle_mxcsr` の参照が無く、`fle_mxcsr` を更新するのは `fle_mxcsr_load`（LDMXCSR）だけ（`fle_mxcsr.c`）。決定 0005 の「仕様の外」にも「SSE は既定の MXCSR で動く」と書いてある。推測 — SSE の算術ハンドラ（`FLE_ARITH64`/`FLE_ARITH32` 系の `fle_addsd`、`fle_mulsd`、`fle_divsd`、`cvt*`、`ucomis*`）が例外フラグを立てないことが差の原因。ハンドラの誤りではなく未実装。
- 最小例（命令数が少ないもの）:
  - 関数 0xe43bb0（3 命令: `movapd xmm0, xmm1; mulsd xmm0, [rcx+0xb04]; ret`）、case `e43bb0.2`。入力は `results/run4/min-example-e43bb0.2.txt`（1 行。RCX=0x2002adc0、XMM0=0xcc5ee8ccdff078eb_40665cc407f8ca6f、XMM1=0x76ffd544c4541598_719296baf6bfd6f2、scratch の fill=4（境界値表）、rflags=0x8c4）。差分: native の MXCSR=0x1fa2、lift=0x1f80。0x1fa2 = 0x1f80 | 0x22 で、native が立てたのは DE（ビット 1）と PE（ビット 5）。GPR、XMM0-15、EFLAGS、メモリ、x87 は両側一致（xmm0=30e2718d80d25744:76ffd544c4541598）。推定ハンドラ: `Mulsd`（`sse_scalar.rs` の `arith(i, "mulsd", 8, "fle_mulsd")`、runtime の `FLE_ARITH64(fle_mulsd, *)`）。
  - 関数 0x6cef20（5 命令、`movups` と `subps` を含む）case `6cef20.2`: native 0x1f82（DE）、lift 0x1f80。推定ハンドラ: `Subps`（packed 版）。
  - 関数 0xe43c70（6 命令、`cvtsi2sd` と `mulsd`）case `e43c70.1`: native 0x1f81（IE）、lift 0x1f80。推定ハンドラ: `Cvtsi2sd` または `Mulsd`（どちらが立てたかは、この case では切り分けていない）。
  - 関数 0x476500、0xe20860、0xe20880（`call` だけの関数で、呼び先が SSE 演算を持つ。native の MXCSR はそれぞれ 0x1fa0、0x1fb0、0x1fa0）。
- 影響: `mxcsr_sticky` は GPR/XMM の結果に影響しない（これらの case の GPR/XMM/メモリ/EFLAGS は一致）。ただし、プログラムが MXCSR の状態ビットを `stmxcsr` で読んで分岐する場合は、lift と native で結果が分かれる（そのような関数が純計算 9,901 件に何件あるかは調べていない。`stmxcsr`/`ldmxcsr`/x87 制御命令を持つ関数は 8 件）。
- 判断: `mismatch_core` ではなく `mismatch_aux` として分け、bit_exact には数えない（決定 0036 §3）。lifter と runtime は変更していない。

### lift_unsupported 13 件（関数 0x69d8b0）

`fle: unsupported instruction at 0x69d90c`（`prefetcht0 byte ptr [r8 + 0x10]`）。v1 の追加実行（上の節）で既出の `Prefetcht0`（0x69d66c、0x69d7c7）と同じ種類の未実装命令。この関数の 16 case は、lift_unsupported 13、native_fault 3 で、bit_exact は 0 件（lift が止まるため、この関数の残りの経路は比較できていない）。

### mismatch_core 0 件について

run4 の初回実行では、関数 0xad7640 の case 5 が「native は戻る、lift は 2 秒でタイムアウト」として mismatch_core（`lift_timeout`）になった。単独で再実行すると両側とも戻って一致（`a5-oracle-run2.md` §6）。並列 16 のもとで 2 秒のアラームが誤って発火したもので、lift のバグではない。最終の run4 ではタイムアウトした case を両側 10 秒で再実行しており、mismatch_core は 0。

### 読み違えやすい点（v2）

1. native_fault 2,465 件（15.4%）は比較不能で、一致の件数に入っていない。lift 側は 2,412 件が同じアドレスの fault、36 件が abort（SIGFPE の 33 件と、scratch 範囲外の 3 件）、17 件が両側 timeout。
2. 「mismatch_core 0」は、比較できた 13,522 case の範囲。浮動小数点の MXCSR 状態ビットの差が常にあるため、浮動小数点を使う関数の bit_exact 判定は「MXCSR 状態ビットを除く」条件付き（717 case は bit_exact に入っていない）。
3. ネガティブコントロールは、実行された命令の opcode/disp/imm の 1 バイト反転 203 件で、出力の差による検出 127（core 117、EFLAGS のみ 10）、native が fault/timeout した検出 56、未検出 20（等価で状態も変わらない 13、途中状態は変わるが出力に出ない 7）。詳細は `a5-oracle-run2.md` §4。
