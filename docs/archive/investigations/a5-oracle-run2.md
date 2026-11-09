# A5 オラクル v2 の実行手順と結果

日付: 2026-10-20。設計と判断は `docs/decisions/0036-a5-oracle-v2.md`（v1 は 0035、`a5-oracle-run.md`）。「実測」はツールの出力、「推測」は未検証の解釈。結果の置き場は `lift/out-oracle/results/run4/`。

## 1. 再現コマンド（1 本）

```
lift/harness/oracle/run2.sh
```

環境変数: `FLENGINE_PE`、`PARTS_NAME`（`after-pdata-b`）、`COUNT`（1000）、`N`（16）、`SEED`（1）、`NEGCTL`（200）、`JOBS`（16）、`OUTDIR`（`run4`）。中身は `build2.sh`（`native_drv2`、`lift_drv2`）、`argptr.py`（ポインタ引数の静的判定）、`exclude_stats.py`（除外分類の集計）、`run_diff2.py`（差分実行、ネガコン、集計）。`results/pure.json` が無いときだけ `select_pure.py` を実行する。所要時間は 5 分 38 秒（`time` の real、16 並列、native 通常 + lift + native 単一ステップ + ネガコン 203 件）。

前提は v1 と同じ（`out-pdata/parts` と `out-pdata/fle_*.o` が既にある、capstone）。

## 2. 結果の要約（実測、`results/run4/a5-summary.json`）

1,000 関数（純計算 9,901 件から。最初の 200 は v1 と同じ `Random(1).sample`）、各 16 case、**16,000 case**。

| 判定 | case 数 |
|---|---|
| bit_exact（GPR、XMM、メモリ、EFLAGS、MXCSR、x87 すべて一致） | **12,805** |
| mismatch_core（GPR/XMM/メモリの差） | **0** |
| mismatch_aux（core は一致、MXCSR/EFLAGS/x87 に差） | **717**（すべて `mxcsr_sticky`。EFLAGS の差 0、x87 の差 0、MXCSR 制御ビットの差 0） |
| lift_unsupported | **13**（関数 0x69d8b0 の `Prefetcht0` at 0x69d90c） |
| lift_stop_other | 0 |
| native_fault（比較不能） | **2,465**（15.4%。v1 は 943/2,400 = 39.3%） |

native_fault 2,465 件のうち lift 側: 同じ種別の fault 2,412（fault アドレスも一致 2,412）、abort 36（`sigfpe` 33 件は lift が abort、`scratch_range_exceeded` 3 件は lift が abort）、timeout 17（両側 timeout）。

関数単位（1,000）: bit_exact_all_comparable 900、has_mismatch_aux_only 62、native_fault_all 37、lift_not_executable 1、has_mismatch_core 0。

### ポインタ入力あり/なし別

| 種別 | case | bit_exact | mismatch_aux | lift_unsupported | native_fault |
|---|---|---|---|---|---|
| ptr_input（ポインタ引数を持つ関数、計 10,416 case = 651 関数） | 10,416 | 7,487 | 565 | 13 | 2,351（22.6%） |
| no_ptr（計 5,584 case = 349 関数） | 5,584 | 5,318 | 152 | 0 | 114（2.0%） |

（651/349 は `functions_with_ptr_args`/`functions_without_ptr_args`。`a5-summary.json`）

### v1 と同じ 200 関数（case 生成は v2）

3,200 case: bit_exact 2,640、mismatch_aux 101、native_fault 459（14.3%。v1 は同じ 200 関数で 943/2,400 = 39.3%）。関数単位は bit_exact_all_comparable 186、has_mismatch_aux_only 7、native_fault_all 7（v1 は native_fault_all 27）。追加の 800 関数分は native_fault 2,006/12,800（15.7%）。

### native_fault の原因別（2,465 件、実測）

| 原因 | 件数 |
|---|---|
| NULL 近傍の参照（実効アドレス < 0x10000） | 1,223 |
| 不正な（無効な）ポインタ（それ以外の未割り当てアドレス） | 650 |
| 非正規アドレス（#GP） | 289 |
| scratch の範囲外（scratch の前後 16 MiB 以内、1 MiB を超えた参照） | 243 |
| 0 除算（SIGFPE。lift は abort） | 33（関数 0xbae280 15、0x928880 11、0x1062ae0 5、他 2） |
| タイムアウト（再実行の 10 秒でも戻らない。両側とも戻らない） | 17（関数 0xad7710 7、0xad7640 6、0x4104e0 2、0x5f0240 2）。0xad7640 の case 5 は再実行で両側が戻り、bit_exact に入っている |
| スタック領域を超えた参照 | 7（関数 0xbb3dd0） |
| #GP（その他。`movsq` の rep なし 8 バイト） | 2（0xebeb90、0xf8f480） |
| イメージ（読み取り専用領域）への書き込み | 1（0x57b8e0 の `movss`、アドレス 0x8a91f1） |

「グローバル未初期化ポインタ」: ベースレジスタが「書き込み可能なイメージのグローバルから読んだ値」の fault は 13 件（`loaded_from_image_global(writable)`）。

fault したメモリ参照のベースレジスタの出所（2,465 件のうち、原因が参照系でないタイムアウト 17 とスタック超過 7 を含めた分類。`a5-cases.json` の `fault_extra` から集計。後方 40 命令の走査による推定）:

| 出所 | 件数 |
|---|---|
| 関数内で別のメモリ参照から読んだ値（rax などがベース） | 663 |
| 関数内に定義がない（入口で持っていた値か、call の戻り値） | 610 |
| 引数が指す構造体の内容から読んだ値 | 321 |
| コピー、演算、`lea`、0、即値の結果 | 301 |
| 整数引数として渡した値と同じ | 296 |
| ポインタとして渡した引数と同じ値（渡したポインタ + オフセットが範囲外など） | 106 |
| スタック引数と同じ値 | 95 |
| 出所なし（暗黙のメモリ参照、SIGFPE、タイムアウトなど） | 60 |
| 書き込み可能なイメージのグローバルから読んだ値 | 13 |

「整数引数として渡した値と同じ」296 件と「スタック引数と同じ値」95 件は、`argptr.py` がポインタと判定しなかった引数がポインタとして使われた件（静的判定の取りこぼし）。

fill モード別の native_fault 率: 0（乱数）9.3%、1 7.4%、2（全 0）14.6%、3（全 -1）3.8%、4（境界値）4.0%、5（ポインタ豊富）32.7%、6（小整数）11.3%。fill 5 は二重間接を持つ関数に使うので、構造体を辿る関数が多く、率が高い。

推測（未検証）: 残る fault の多くは、引数が指す構造体の内部のポインタ（3 段以上の間接、または型が決まったポインタ表）と、静的判定が拾えなかった間接参照。入力が構造体の型を知らないことが原因。

### 比較対象の拡張の結果

EFLAGS（status フラグ 6 個）: 両側が戻って比較した 13,522 case（bit_exact 12,805 + mismatch_aux 717）で、定義済みフラグの差は **0**（`flag_stats`）。

| フラグ | 定義として比較 | 差 | lift が未定義（比較せず） | そのとき native が 1 |
|---|---|---|---|---|
| CF | 13,515 | 0 | 7 | 2 |
| PF | 13,457 | 0 | 65 | 32 |
| AF | 10,680 | 0 | 2,842 | 109 |
| ZF | 13,457 | 0 | 65 | 9 |
| SF | 13,457 | 0 | 65 | 33 |
| OF | 13,462 | 0 | 60 | 3 |

AF の未定義が多いのは、logic 命令（and/or/xor/test）と shift の後に AF が `FLE_UNDEF` になるため（lift の runtime、`fle_flags.h`）。除外は命令単位でなく「lift が未定義とするフラグ」単位。

MXCSR: 制御ビットの差 0。状態ビットの差（`mxcsr_sticky`）が **717 case / 62 関数**。native が立てたビット（case 数、重複あり）: PE 637、DE 270、IE 141、UE 108、OE 52、ZE 17。lift の MXCSR は全 case で 0x1F80（`mxcsr_lift_ne_reset` = 0）。native 側で 0x1F80 以外になったのは 717 case（`mxcsr_native_ne_reset`）で、`mxcsr_sticky` の件数と一致。lift の runtime が SSE 演算で MXCSR を更新しないため（`fle_sse.h` に `fle_mxcsr` の参照が無い）。62 関数のうち、SSE の浮動小数点演算を自身が持たない関数（呼び先が持つもの）も含む。

x87（cw、sw、tw の空集合）: 差 0。native も lift も、全 case で `fninit` 後の既定値（cw 0x037F、sw 0、tw 0xFFFF）のまま（`x87_native_ne_reset` = 0、`x87_lift_ne_reset` = 0）。x87 命令を使わない関数で不変であることの確認になった。MMX 命令を持つ純計算関数は全 9,901 件で 4 件（0x660c40、0x660c80、0x6a7900、0x6a7990）。選ばれた 1,000 関数のうち 1 件が含まれる（どれかは `a5-cases.json` の関数 VA と上の 4 件の突き合わせで確認できる）。x87 tw の「生の値」の差は 0（参考値）。

### 単一ステップ（トレース）の整合

native の通常実行と単一ステップ実行の結果は、戻った 13,528 case で一致（`trace_consistency.equal`）。差があったのは、通常 fault だが単一ステップではタイムアウトになった 24 件と、通常は戻るが単一ステップではタイムアウトになった 7 件（単一ステップのオーバヘッドで 10 秒に届いたもの。このトレースはネガコンのベースにだけ使う）。

実行された命令の割合（単一ステップで取れた case の和集合、関数の命令数に対する）: 平均 91.4%、中央値 100%、全命令を実行した関数 808/1,000、ゼロ 37 関数（全 case が fault）。

## 3. 除外された関数の分類（`exclude_stats.json`、実測）

候補（.pdata の開始かつ lift 済み）57,816 件。

| 区分 | 自身が持つ（重複あり） | 呼び先を経由して届く（重複あり） |
|---|---|---|
| IAT 呼び出し（IAT サンク `jmp [rip+IAT]` への直接 call/jmp） | 3,407 | 39,129 |
| IAT スロットの直接参照 | 0 | 0 |
| 間接 call | 16,467 | 46,587 |
| 間接 jmp | 5 | 37,063（間接 call/jmp の和は 46,678） |
| x87 | 78 | 2,893 |
| GS/FS オペランド | 1 | 36,913 |
| cpuid/rdtsc/int など | 5 | 24 |
| 直接の呼び先が lift 済み関数でも IAT サンクでもない | 0 | 0 |

重複を除いた分割（優先順 IAT、間接 call、間接 jmp、x87、segment、special。自身の理由を先に、無ければ呼び先の理由）:

| 区分 | 件数 |
|---|---|
| 純計算（残った） | 9,901 |
| 自身が IAT 呼び出し | 3,407 |
| 自身が間接 call（IAT なし） | 15,097 |
| 自身が間接 jmp（IAT・間接 call なし） | 2 |
| 自身が x87（上記なし） | 74 |
| 自身が cpuid など | 4 |
| 呼び先だけが IAT 呼び出し | 25,928 |
| 呼び先だけが間接 call | 3,325 |
| 呼び先だけが間接 jmp | 67 |
| 呼び先だけが x87 | 2 |
| 呼び先だけが cpuid など | 9 |
| 合計 | 57,816 |

注: GS/FS オペランドを自身が持つ関数は 1 件だけで、その 1 関数に直接・推移的な呼び出しで届く関数が 36,913 件ある（実測。この 1 件が何の関数かは調べていない）。この区分は優先順で上位の理由に吸収されるので、分割表には現れない。v1 の `select_pure.py` の件数（間接 call 16,461、推移 29,331 など）は「最初に見つかった 1 つの理由」の件数で、本表の重複あり/なし分割とは数え方が違う。

## 4. ネガティブコントロール（実測、`a5-negctl.json`）

native のみ、**単一ステップで実際に実行された命令**の opcode、displacement、immediate のバイトを 1 つ XOR 0x01。対象関数は元の case が bit_exact（native が fault しない入力）だったもの。

| 種類 | 件数 | 検出 |
|---|---|---|
| 実行された命令のバイト（無作為の関数 200 件、1 関数 1 バイト） | 200 | 下記 |
| 定数返却関数（`mov eax, imm32; ret`）の imm（健全性確認） | 3 | 3（core） |
| 合計 | 203 | |

結果（203 件）:

| 結果 | 件数 | 意味 |
|---|---|---|
| detected_core | 117 | GPR/XMM/メモリが lift と違う（定数返却 3 件を含む） |
| detected_aux_only | 10 | EFLAGS だけが違う（PF/ZF/SF/AF/CF の組。MXCSR 状態ビットは除く） |
| detected_by_native_fault_only | 56 | 反転後に native が fault/タイムアウトし、出力の差としては観測できない |
| undetected_state_changed_but_masked | 7 | 実行の途中状態は変わったが、最終出力は同じ |
| undetected_equivalent_no_state_change | 13 | 各ステップの状態ハッシュも同じ（観測範囲では等価） |

検出率（実測）: 出力の差で検出 127/203 = 62.6%（core 117 + aux 10）。native fault も含めると 183/203 = 90.1%。v1 は 29/60 = 48.3%（v1 は fault した case を「未検出」側に数えていた。比較対象も GPR/XMM/メモリのみ）。v2 の core のみ（v1 と同じ比較対象）では 117/203 = 57.6%。

role 別（件数 = opcode/disp/imm、すべての結果の合計）: opcode 123（detected_core 75、aux 2、native fault のみ 39、masked 1、equivalent 6）、disp 45（core 21、aux 5、fault のみ 12、masked 3、equivalent 4）、imm 35（core 21、aux 3、fault のみ 5、masked 3、equivalent 3）。

未検出 20 件の内訳（実測の分類 + 命令の確認）:

反転後の命令を capstone で再デコードした結果（実測。`未検出` 20 件の全部）。「実行 case 数」はそのバイトを含む命令を実行した case の数。

| 区分 | 関数 | 元の命令 → 反転後 | 実行 case 数 |
|---|---|---|---|
| 等価 | 0xe04500 | `je 0xe04510` → `je 0xe04511` | 16 |
| 等価 | 0xa93e60 | `cmp qword [rbp], rax` → `cmp byte [rbp], al` | 1 |
| 等価 | 0x705a90 | `test dword [rcx+0x28], eax` → `test dword [rcx+0x29], eax` | 1 |
| 等価 | 0x4104e0 | `pause`（`f3 90`）→ `nop`（先頭バイトが `f2` になり、capstone は nop と読む） | 10 |
| 等価 | 0x1062aa0 | `test edx, edx` → `test dl, dl` | 6 |
| 等価 | 0x5aadb0 | `sub r10, r9` → `sub r10b, r9b` | 8 |
| 等価 | 0x127b470 | `mov dword [rax+0x10], edx` → `mov dword [rax+0x11], edx` | 14 |
| 等価 | 0xa5fc40 | `mov qword [rip+0xbfa1b5], 0` → `[rip+0xbea1b5]` | 16 |
| 等価 | 0xca0f20 | `mov rsp, rbp` → `mov spl, bpl` | 16 |
| 等価 | 0x1312f70 | `mov rax, [rip+0x65bccd]` → `[rip+0x65bdcd]` | 16 |
| 等価 | 0xcaf670 | `mov rsp, rbp` → `mov spl, bpl` | 16 |
| 等価 | 0x6da8e0 | `cmp dword [rax+0x398], 0x7fffffff` → `0x7effffff` | 16 |
| 等価 | 0x11db3b0 | `test eax, 2` → `test eax, 0x1000002` | 10 |
| 途中状態は変化 | 0x477180 | `cmp cx, 0x7f7f` → `0x7f7e` | 14 |
| 途中状態は変化 | 0x80e720 | `je 0x80e733` → `jne 0x80e733` | 16 |
| 途中状態は変化 | 0x5392b0 | `lea rax, [rax-1]` → `[rax-2]` | 9 |
| 途中状態は変化 | 0xb54d20 | `sub eax, 0xb2` → `0xb3` | 7 |
| 途中状態は変化 | 0x12dea10 | `jl 0x12dea4b` → `jl 0x12dea4c` | 16 |
| 途中状態は変化 | 0x8ccb90 | `lea rax, [rax-1]` → `[rax-2]` | 5 |
| 途中状態は変化 | 0xf34fb0 | `mov rax, [rip+0x6ad715]` → `[rip+0x6ad615]` | 16 |

読み取れること（実測の範囲）: レジスタの幅だけが変わる 4 件（`test dl, dl`、`sub r10b, r9b`、`mov spl, bpl` 2 件）は、使った入力では結果が同じだった。上位ビットが異なる入力なら出力が変わる可能性がある（推測。そのような入力は試していない）。0x127b470 の `mov [rax+0x10], edx` → `[rax+0x11]` は、native の通常実行で image と scratch の書き込みが 0 件、スタックの差分は戻りアドレススロットだけだった（反転後の書き込みがどこへ行ったか、なぜ差分に出ないかは調べていない）。他の個別の理由は命令ごとには追跡していない（推測を含む。例えば `cmp` の定数の 1 ビット違いで比較結果が全 case で同じになった可能性）。

実行 case 数が 1 のものは 2 件（0xa93e60、0x705a90）、2 以上が 18 件。反転対象を実行された命令に限ったので、「実行されない経路」が原因の未検出は 0 件。

未検出の原因に「比較対象外の状態」が含まれるか: x87、MXCSR 制御、EFLAGS は比較に入っているので、これらの差が漏れた例はない（`detected_aux_only` 10 件は EFLAGS の差で検出）。MXCSR 状態ビットは上記の理由で「検出」から除外している。メモリのうち比較外なのは、書き込み可能なイメージ（.data など）・scratch・スタック以外の領域。未検出 20 件が比較外のメモリへの書き込みだったかは確認していない。

v1 からの変化: v1 の未検出 31 件のうち 14 件は「native が fault する case が残って、比較できた case は一致」、17 件は「全 case が bit_exact のまま」で、実行されない経路に当たったバイトかの区別が付かなかった。v2 では実行された命令に限り、fault しない入力で回したので、未検出 20 件はすべて「そのバイトを含む命令は実行されたが、出力に現れなかった変化」。

注意: `opcode` の分類は、capstone が接頭辞と見なさなかった先頭バイトを含む（`pause` の `f3`、0x4104e0 の例）。検出率の比較（v1 は 29/60、v2 は 127/203 または 183/203）は、v1 と v2 で反転対象の選び方（v1: 関数本体の無作為位置、v2: 実行された命令の opcode/disp/imm）と判定（v2 は EFLAGS と native の fault も区別して数える）が違うので、同じ条件の比較ではない。

## 5. 不一致（実測）

- mismatch_core: **0 件**。
- mismatch_aux: 717 case、すべて MXCSR 状態ビット（lift の runtime が更新しない既知の未実装）。命令ハンドラの誤りは検出されていない。最小例は `a5-oracle-mismatches.md` に追記。
- lift_unsupported: 13 case、関数 0x69d8b0 の `Prefetcht0`（0x69d90c）。v1 の追加実行でも同じ（`a5-oracle-mismatches.md`）。

## 6. 試行錯誤（参考。最終結果には影響しない）

- 最初の run4 実行で、関数 0xad7640 の case 5 が「native は戻る、lift は 2 秒でタイムアウト」で `mismatch_core`（`lift_timeout`）と判定された。単独で再実行すると両側とも戻り、結果は一致（gpr/xmm/mem/mxcsr 同じ、実行時間 0.03 秒）。並列 16 と lift の起動時間の下で 2 秒のアラームが誤発火したと見ている。これを受けて、タイムアウトした case は両側を 8 並列、10 秒で再実行する処理を `run_diff2.py` に足した。最終の run4 では、再実行された 18 case のうち 1 件が両側で戻り、17 件は両側とも 10 秒でも戻らない。
- ポインタ判定は 3 段階で直した（200 関数 × 16 case の試走。`smoke2` は 20 関数、`smoke2b`、`smoke2c` は 200 関数）。初版（逆参照のみ、20 関数）は native_fault 83/320。rbp フレーム、添字（整数）、`[arg+d]` への poke を足した `smoke2b` は 492/3,200、レジスタの初期値（case 14/15）と fill 5 の変更を足した `smoke2c` は 459/3,200。最終（run4）の v1 と同じ 200 関数は 459/3,200 で `smoke2c` と同じ。
- run4 は `run2.sh` を 2 回実行している。1 回目（タイムアウト再実行の処理なし）の出力は 2 回目が上書きした（自分が作ったファイルのみ）。1 回目との違いは、関数 0xad7640 の case 5 が `mismatch_core` から `bit_exact` に変わり、bit_exact 12,804 → 12,805、mismatch_core 1 → 0 になった点だけ。

## 7. 限界（事実）

- 1 関数 16 case では分岐の全経路は通らない（関数全命令を実行した関数は 808/1,000）。
- native_fault 2,465 件（15.4%）が残る。両側が戻って比較できたのは 13,522 case（bit_exact + mismatch_aux）。
- 純計算 9,901 件のうち 1,000 件。純計算は小さい関数に偏る（v1 と同じ）。選別から外した 47,915 件（82.9%）は、IAT 呼び出し、間接 call 等を持つ関数、またはその呼び元で、今回も対象外。
- MXCSR 状態ビットは lift が更新しないので、浮動小数点の関数の MXCSR は比較しても必ず差が出る。その分は `mismatch_aux` の別カテゴリ。
- EFLAGS: 未定義フラグ（lift が `FLE_UNDEF`）は比較していない。AF は 2,842 case で未定義。
- 乱数 seed は 1 のみ。

## 8. 触ったファイル

新規: `lift/harness/oracle/oracle2_common.h`、`oracle2_common.c`、`ms_call2.S`、`native_drv2.c`、`lift_drv2.c`、`argptr.py`、`exclude_stats.py`、`run_diff2.py`、`build2.sh`、`run2.sh`、`docs/decisions/0036-a5-oracle-v2.md`、`docs/investigations/a5-oracle-run2.md`、`lift/out-oracle/` 配下の生成物（`bin/native_drv2`、`bin/lift_drv2`、`obj/*.v2.o`、`results/argptr.json`、`results/exclude-stats.json`、`results/run4/`、`results/smoke2*`、`tmp/`）。追記: `docs/investigations/a5-oracle-mismatches.md`（末尾のみ）。lifter、runtime、`imports_table.rs`、v1 の oracle ファイルは変更していない。
