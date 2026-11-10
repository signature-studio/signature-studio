# A2a 残りの命令 43 件の実装（実行記録）

決定記録: `docs/decisions/0047-a2a-remaining.md`。実測は 2026-10-21、ホストは AMD Ryzen 9 9950X（Zen 5）、clang 21.1.8、HEAD 5e7d7e6 の上。作業用の出力は `lift/out-a2a/`（`.gitignore` の扱いは未確認）。

## 1. 命令の一覧（実測）

入力: `lift/out-doc/out/gen-0046.c`（HEAD の lift、all-pdata の生成 C、`fle_unsupported` 601 行）。IAT 経由 jmp 557 と解決不能 jmp 1 を除く 43 行の VA を、`lift/objdump-text.txt` の逆アセンブルに当てた（`out-a2a/insn-stubs.txt`）。**0046 の数字と一致した（43、内訳も同じ）。**

| 命令 | 件数 | 形（オペランド） | VA |
|---|---|---|---|
| Psubw | 10 | 全て `psubw xmm,xmm`（メモリ形 0、mm 形 0）。`xmm0,xmm1` が 6（0x6bbe72 0x6bbefe 0x6bbf5f 0x9f2257 0x9f22e2 0x9f2399）、`xmm0,xmm4` が 3（0x9f2268 0x9f22f3 0x9f23aa）、`xmm2,xmm3` が 1（0x9f2400） | 0x6bbe72 0x6bbefe 0x6bbf5f / 0x9f2257 0x9f2268 0x9f22e2 0x9f22f3 0x9f2399 0x9f23aa 0x9f2400 |
| Pcmpgtd | 8 | 全て `pcmpgtd xmm,xmm`（メモリ形 0、mm 形 0）。`xmm5,xmm4`（0x5e4ca3 0x5e4dc3）、`xmm3,xmm5`（0x5e4cbe 0x5e4dde）、`xmm7,xmm6`（0x5e5640 0x5e5760）、`xmm3,xmm7`（0x5e565f 0x5e577f） | 0x5e4ca3 0x5e4cbe 0x5e4dc3 0x5e4dde 0x5e5640 0x5e565f 0x5e5760 0x5e577f |
| Rcpps | 8 | 全て `rcpps xmm7,xmm4`（レジスタ形のみ） | 0x679490 0x679950 0x679e10 0x67a2b0 0x67ba60 0x67c090 0x67c6c0 0x67ccd0 |
| Bswap | 7 | `bswap eax` 3（0x5e916d 0x6ae122 0x6ae132）、`bswap ebx` 1（0x5ea020）、`bswap r9d` 1（0x9f2404）、`bswap rax` 1（0x478506）、`bswap rcx` 1（0x478509）。r32 5、r64 2 | 0x478506 0x478509 0x5e916d 0x5ea020 0x6ae122 0x6ae132 0x9f2404 |
| Test（high-byte） | 7 | `test ah,2`（0x5e8f8b 0x5eac8c 0x5ead4c）、`test ah,4`（0x5eabd8 0x5eace0）、`test bh,0xe0`（0x5ea243）、`test bh,2`（0x5ea960）。全て `r/m8, imm8` のレジスタ形 | 左記 |
| And（high-byte） | 1 | `and ah,0x41`（直前が `fnstsw ax`） | 0x6a044d |
| Andnpd | 1 | `andnpd xmm2,xmm0` | 0x40b17d |
| Xgetbv | 1 | `xgetbv`（`0f 01 d0`） | 0x639a03（関数 f_6399e0） |

Psubw と Pcmpgtd は、xmm 形（`66 0F F9`、`66 0F 66`）だけだった。mm 形は 0。内訳は `objdump-text.txt` の該当 VA の行を読んで書いた。

## 2. 実装

| 命令 | 形 | 実装場所 |
|---|---|---|
| Psubw | `xmm, xmm/m128`（16 バイト境界のメモリ形を含む） | `sse_int2.rs` に 1 行、`fle_sse_int2.h` に `fle_i2_psubw` |
| Pcmpgtd | `xmm, xmm/m128` | 同上 `fle_i2_pcmpgtd`（符号付き） |
| Andnpd | `xmm, xmm/m128` | `sse_pack.rs` に 1 行（既存の `fle_pandn`） |
| Rcpps | `xmm, xmm/m128` | `sse_int2.rs` に 1 行、新規 `fle_rcp.h`（`fle_sse_int2.h` の末尾で include）、新規 `fle_rcp_table.h`（4096 エントリ、生成物） |
| Bswap | r32、r64（r16 は SDM が「undefined」なので Err のまま） | `int_more2.rs` の `bswap` |
| high-byte | `lib.rs` の `reg_read` / `reg_write` が AH/CH/DH/BH を扱う（共通ヘルパ `is_high_byte`）。`reg_num` の拒否を外した | `lib.rs` |
| Xgetbv | 名前付き停止 `fle_xgetbv_stop(va)` | `cpuid.rs`、`fle_cpuid.c` |

実装した形の数: Psubw 1 形（xmm,xmm/m128 の reg と mem）、Pcmpgtd 1、Andnpd 1、Rcpps 1、Bswap 2（r32、r64）、8 ビットレジスタ（high-byte 形を含む）の全 ALU 系、Xgetbv 1（停止）。

## 3. Rcpps の調査と全数突合

SDM Vol. 2（325383-093US、`out-int2/spec/sdm-vol2.txt` の 58963 行以降）の RCPPS: 「`|Relative Error| ≤ 1.5 * 2^-12`」、「The RCPPS instruction is not affected by the rounding control bits in the MXCSR register」、0.0 には符号付き ∞、denormal は 0.0 扱い、tiny は符号付き 0 に flush、SNaN は QNaN に変換、QNaN はそのまま。擬似コードは `DEST[31:0] := APPROXIMATE(1/SRC[31:0])` で、APPROXIMATE の定義は無い。SIMD 例外は「None」。

調査（`out-a2a/rcp/`、`probe1.c`、`exhaust.c`）:

1. ホストの結果は仮数上位 12 ビット（`m >> 11`）だけで決まる。指数 127 と 100、符号 0 と 1 で、2^23 個の仮数全部について、同じ上位 12 ビットのときの結果が一致（`probe1`）。
2. 表 T[4096]（指数 127 の入力から測定、結果は指数 126、結果の下位 11 ビットは 0）を作り、**2^32 個の全ビットパターンについて、モデル（`T`、指数 253−e、特殊値の規則）とホストの `rcpps`（4 レーン）を比較して不一致 0**（`out-a2a/rcp/exhaust-default.txt`）。MXCSR を 1f80、9fc0（FTZ+DAZ）、5f80（RC=10）、3f80、7f80、8000、ffc0、1f00 にして繰り返し、全て不一致 0（`exhaust-mxcsr.txt`）。RC、FTZ、DAZ、例外マスクは結果に影響しなかった。MXCSR の状態ビットは `after-all` が開始値と同じで、フラグは立たない（`exhaust` の出力）。
3. 表の閉形式は見つからなかった。`1/(1+(i+h)/4096)` の floor と round を h = 0, 0.25, 0.5, 0.75, 1 で試し、最良でも 1,060/4,096 が不一致。`8192*4096/(4096+i+h)` の切り捨て、四捨五入、切り上げを h = 0〜2（1/8 刻み）で試した最良も 1,060 不一致（h=0.25、四捨五入）。h=0.5 の理想値との残差は −1.23〜+1.46（最下位ビット単位）。単純な丸めでは再現できない。このため表をそのまま埋め込む。（この探索は一時的な python の実行で、スクリプトは保存していない。結果の数字だけが記録。）
4. `tests/rcp/host_dump.c`（ホストの `rcpps` から表を出力）→ `tests/rcp/gen_rcp_table.py`（`runtime/fle_rcp_table.h` を生成）で再現できる。`dump.txt` の SHA-256 `f4dbc928…ee2a`、ヘッダの SHA-256 `6ae87484…1758`。
5. runtime は純 C（`fle_rcp.h`）。ホストの FP ユニットも asm も使わない。ホスト依存は表の測定だけで、測定結果は C 配列としてコミットする。

テスト（`tests/a2a_remaining.rs`）: `rcpps_matches_host`（11,326 行 × 7 形、特殊値と全 4096 索引と全指数）、`rcpps_all_2e32_inputs_match_host`（lifted C の `fle_i2_rcpps` とホストを 2^32 入力で比較。`out-a2a/test/rcp_exhaustive` の単体実行でも `inputs 4294967296 mismatches 0`、約 7 秒）、`rcp_table_header_matches_host_dump`（ヘッダの表がホストの測定と同一）。

## 4. ホスト突合（`tests/a2a_remaining.rs`、13 件）

方式は `tests/sse_int2.rs`、`tests/int_more2.rs` と同じ（ホストは実機械語を mmap して実行、C は `lift_body` を clang -O0）。比較: xmm0〜15、rax rcx rdx rbx rsi rdi r8 r9 r10、メモリ 64 バイト、6 つのフラグ。

| グループ | ベクタ | ケース | 不一致 | フラグ比較 |
|---|---|---|---|---|
| psubw | 7（reg 3 形、mem 4 形） | 4,900 | 0 | 29,400 |
| pcmpgtd | 7 | 4,900 | 0 | 29,400 |
| andnpd | 7 | 4,900 | 0 | 29,400 |
| rcpps | 7 | 11,326 | 0 | 67,956 |
| bswap | 18（r32、r64 × 9 レジスタ） | 1,440 | 0 | 8,640 |
| high-byte（対象: test と and） | 378 | 15,120 | 0 | 75,600 |
| high-byte（監査: 全 8 ビット命令） | 2,525 | 35,350 | 0 | 198,164 |
| **計** | **2,949** | **77,936** | **0** | |

加えて Rcpps の 2^32 入力の全数突合（不一致 0）。入力はレーンの辺の値と乱数が半々で、`d == s` の形を含み、メモリ形は 16 バイト境界上。各テストで「ホストが状態を変えたケース」が 80% 以上であることを assert（psubw などは 100%、bswap 1,413/1,440、high-byte 対象 7,582/15,120 は TEST が状態を変えないため。TEST 以外は 29,003/35,350）。

AF の扱い: TEST と AND は AF が未定義（SDM）。C は `FLE_UNDEF`。`required`（対象グループ）で、全ケースで AF だけが未定義であることを確認し（15,120 件）、CF=OF=0 のままであること、PF、ZF、SF が両方の値を取ることを assert。

メモリ形の境界外: SDM Table 2-21 の Type 4（`#GP(0)` Legacy SSE: Memory operand is not 16-byte aligned）。ホスト実測（`out-a2a/probe_align.c`）で psubw、pcmpgtd、andnpd、rcpps の境界外（オフセット 1、8）は SIGSEGV（11）、境界内（0、16）は fault しない。C 側は `fle_ldxa` で abort（`misaligned_memory_operand_aborts`、4 命令、`misaligned 16-byte access`）。

MMX 形、VEX 形、bswap の 16 ビット形は未対応のまま（`bswap_16bit_and_other_forms_stay_unsupported`）。

## 5. high-byte の監査

`lib.rs` の `reg_num` は AH/CH/DH/BH を `Err("high-byte register")` で拒否していた。このため、`lib.rs` の汎用ハンドラ（mov 以外の ALU、cmp、test、inc、dec、neg、not、シフト、setcc、cmov、push、pop など）は、**拒否で止まる**形で、誤って別のレジスタを読み書きすることは無かった（`reg_read` を通る全部が `reg_num` を先に呼ぶため）。他モジュールの扱い:

| モジュール | high-byte の扱い | 判定 |
|---|---|---|
| `insn_int.rs` | `mov`（高位形）、`movzx` を `R[n] >> 8` で正しく扱う | 正しい（既存テストあり） |
| `int_ext.rs`、`int_more.rs` | `reg_rd` が `is_high` を見る（movsx、idiv、div、xadd、cmpxchg の読み出し）。書き込みは `reg_wr` | `int_more.rs` は書き込みも `is_high` を処理。`int_ext.rs` の `reg_wr` は 8 ビット宛先を `Err`（高位でも）。誤用なし |
| `int_more2.rs` | `lib.rs` の `reg_read` / `reg_write` を使う（rol、ror、xchg など）。以前は `reg_num` の `Err` で拒否 | 今回の変更で正しい高位形になった（突合で確認） |
| `mmx.rs` | `need(!matches!(r, AH|CH|DH|BH), "high-byte register")` で拒否 | 誤用なし（拒否） |
| `sse_*.rs`、`vex.rs`、`sse_fp_flags.rs` | GPR は 32/64 ビットだけ（`r.size()` が 4 か 8）か `is_gpr64` | 誤用なし |
| `mul`（`lib.rs`）8 ビット形 | `R[0]` の AX を直接使い、ソースは `op_read`（高位形対応） | 正しい（監査ベクタで確認） |

監査の実測: all-pdata の閉包で高位レジスタをレジスタオペランドに持つ命令は **54 件**（`out-a2a/audit-highbyte.txt`）。`mov` 35、`lock`（cmpxchg など）6、`movzx` 5、`test` 7、`and` 1。旧生成 C との比較: `mov` 35、`lock` 6、`movzx` 5 は旧も新も同じ高位形のコード（`>> 8` か `0xff00`）で、**誤っていたものは無かった**。`test` 7 と `and` 1 は旧 unsupported から新で高位形になった。**既存の実装が高位を誤って別のレジスタとして扱っていた事例は、実測では 0 件**（修正の必要なし）。

加えて、ホスト突合で、8 ビット命令の全形を高位レジスタの全組合せで確認した（監査グループ、2,525 ベクタ、35,350 ケース、不一致 0）: add、or、adc、sbb、and、sub、xor、cmp、test（reg,reg / reg,imm / mem,reg / reg,mem / mem,imm）、inc、dec、not、neg、mul、imul（1 オペランド）、rol、ror、shl、shr、sar（1、CL、imm8）、mov、xchg、xadd、cmpxchg、movzx、movsx、setcc 16 種、div、idiv（除数が BH、CH、DH）。

**CL シフトのカウントは 0〜8 に制限した**（`ShiftCl`）。8 ビットで masked count が 9〜31 のシフトは SDM が結果を未定義にしており、lift は `fle_bad_count` で止まるため（`fle_flags.h`）、これは既存の仕様で、本作業の変更ではない。

変異テスト: 読み出しから `>> 8` を外す、書き込みの `~0xff00` を `~0xff` に変える、の 2 変異で、対象と監査の 2 テストが失敗（`mut-hb-read.txt`、`mut-hb-write.txt`）。

## 6. Xgetbv の到達性

`0x639a03` の周辺（`objdump-text.txt`）:

```
6399e0: mov r10,rbx        6399e3: xor eax,eax      6399e5: cpuid
6399e7: cmp eax,1          6399ea: jb 0x639a15
6399ec: mov eax,1          6399f1: cpuid
6399f3: and ecx,0x18000000 6399f9: cmp ecx,0x18000000   6399ff: jne 0x639a15
639a01: xor ecx,ecx        639a03: xgetbv           639a06: and eax,6 ; cmp eax,6 ; jne 0x639a15
639a0e: mov eax,1 ; jmp 0x639a17          639a15: xor eax,eax          639a17: mov rbx,r10 ; ret
```

- `xgetbv` に到達する条件は、CPUID 葉 1 の ECX の OSXSAVE（ビット 27）と AVX（ビット 28）が両方立っていること（`jne 0x639a15` を通らない）。
- ランタイムの CPUID（`fle_cpuid.c`）は、葉 1 の ECX を 0、EDX を SSE と SSE2 のビットだけ返す。葉 0 の EAX は 1、`cmp eax,1; jb` は取られず葉 1 まで進み、`and ecx,0x18000000` が 0、`cmp` が不一致、`jne` が取られて 0x639a15 へ飛ぶ。**xgetbv は実行されない**。テスト `runtime_cpuid_never_reports_osxsave_and_avx` が、CPUID 入力 10 通り（葉 0、1、2、7、0xd、0x7fffffff、0x80000000、0x80000001、0xffffffff、0x1_0000_0001）× ECX 4 通りで、ECX が `0x18000000` を満たさず、XSAVE（ビット 26）も立たないことを確認した。
- 生成した `f_6399e0` を単体で実行（`out-a2a/xgetbv_run`）: `eax=0` で戻り、`rbx` は復元され、`fle_xgetbv_stop` は呼ばれない。
- 静的な到達性: 生成 C の呼び出しグラフ（直接の call と tail call）で、`f_6399e0`（呼び元 0x639beb は `f_639a20` 内）← `f_639a20`（呼び元 0x63a38c は `f_63a330` 内）← `f_63a330` ← `f_63a5e0`。祖先は 3 関数で、`f_63a5e0` には生成 C に直接の呼び元がない（`out-a2a/xgetbv-reach.txt`）。`f_63a5e0` の VA は、DLL の 8 バイトのポインタ表（VA 0x1377140。近傍の並びは 0x6398a0/0x639890、0x63a5e0/0x63a5d0、0x63aae0/0x63aa90 の組で、初期化と終了の対に見える。これは推測）と `.pdata` にある。間接呼び出し（`fle_dispatch_table` を経由する）で到達する可能性は排除していない。したがって、`xgetbv` を含む関数に実行で到達する経路はありうるが、そこでは上の CPUID の分岐で `xgetbv` の手前を通らずに抜ける。この経路が実際に実行されるかは、今回の harness の実行では確認していない。
- `fle_xgetbv_stop` は、到達すれば `fle: xgetbv at 0x639a03: XCR0 is not defined ...` を出して abort する名前付きの停止。`fle_unsupported` ではない（`fle: unsupported instruction` を出さない）。unsupported の計数からは外れる。

仕様の根拠（XCR0 の値）: SDM は `EDX:EAX := XCR[ECX]` とだけ書き、XCR0 の値は OS が決める（`#UD` の条件に `CPUID.01H:ECX.XSAVE[26] = 0` がある）。実装できる根拠が無いので、XCR0 の値は実装していない。

## 7. 再生成と計数

`fe-lift lift binaries/win64/FLEngine_x64.dll OUT.c --pdata`（`out-a2a/target/release/fe-lift`、`out-a2a/gen-final.txt`）:

| 項目 | 前（`out-doc/gen-0046.txt`） | 後 |
|---|---|---|
| 関数 | 58,695 | 58,695 |
| `c_bytes` | 215,539,555 | 215,539,950 |
| `unsupported_total` | **601** | **558** |
| 命令 stub | **43** | **0** |
| 残り（`unsupported_total` の内訳） | IAT 経由 jmp 557、解決不能 jmp 1、命令 43 | IAT 経由 jmp 557、解決不能 jmp 1（`f_4126f0` の `jmp [rcx]`、0x412752） |
| `grep -c "fle_unsupported(UINT64_C"` | 601 | 558 |
| `bad_nodes` | 0 | 0 |

新たに処理された行: `andnpd` 1、`bswap` 7、`pcmpgtd` 8、`psubw` 10、`rcpps` 8、`test` +7、`and` +1、`xgetbv_stop` 1（計 43）。旧生成 C と新生成 C の差分は、この 43 行だけ（旧の行はすべて `fle_unsupported`、新の行 43 本、`diff` で確認）。

**A2a の判定は PM が決める。** 数字: 命令 stub は 0。`unsupported_total` から IAT 経由 jmp を除くと 1（解決不能 jmp）。Xgetbv は stub ではなく、到達時に名前付きで停止する命令として生成している（上の 6）。

コンパイルの確認: 新機能を含む 24 関数を抜き出した C（`out-a2a/new-funcs.c`、63,657 行）を、`harness/wasm/build.sh` の `CFLAGS_GEN` と同じフラグで clang と emcc の両方でコンパイルできた（どちらも rc=0）。`fle_cpuid.c` も emcc `-Wall -Wextra` で警告なし。**全体の wasm ビルドとリンクは再実行していない**。

## 8. テストの実測

- `cargo test --offline -p fe-lift --test a2a_remaining`: 13 件、全通過（`out-a2a/test-a2a-3.txt`）。
- ワークスペース全体（`cargo test --offline --workspace`、`out-a2a/test-ws.txt`）: 66 バイナリ、**629 passed、0 failed**、0 ignored（他担当の未コミット変更がワークツリーに残る状態。HEAD は 5e7d7e6）。0046 の記録（HEAD 31d0134）は 65 バイナリ 615 passed。差は +1 バイナリ、+14 件。本作業の `a2a_remaining` が 1 バイナリ 13 件で、残りの +1 件の出所は確認していない（HEAD が 31d0134 から 5e7d7e6 に進んだ分の可能性。推測）。失敗 0 なので、他担当の未コミット変更との切り分け（自分の変更を HEAD に重ねたコピーでの実行）は行っていない。
- 既存テストの編集: `tests/int_more2.rs` の `unsupported_stay_unsupported` から、xgetbv と high-byte の「未対応のまま」を外した（xgetbv は名前付き停止、`rol ah,cl` は対応済み）。`git diff` で +4/-4 行。

## 9. 変異テスト（`out-a2a/mut-*.txt`）

| 変異 | 結果 |
|---|---|
| psubw の `p - q` を `p + q` | `psubw_matches_host` 失敗 |
| pcmpgtd を符号なし比較（`u32`） | **通過（検出されず）**。下の注 |
| pcmpgtd を `(uint32_t)p > (uint32_t)q` | `pcmpgtd_matches_host` 失敗 |
| rcpps の tiny 境界を `e >= 254` | `rcpps_matches_host`、`rcpps_all_2e32_inputs_match_host` が失敗 |
| rcpps の denormal を 0 のときだけ ∞ | `rcpps_matches_host` 失敗 |
| rcpps の SNaN の quiet 化を外す | `rcpps_matches_host` 失敗 |
| rcpps の表を 1 入力（m == 0x3a5a5a）だけ別の添字 | `rcpps_matches_host` は**通過**、全数 `rcpps_all_2e32_inputs_match_host` が失敗（全数テストが要る根拠） |
| bswap の 32 ビット書き込みの上位ゼロ拡張を外す | `bswap_matches_host` 失敗 |
| high-byte の読み出しから `>> 8` を外す | 対象と監査の 2 テストが失敗 |
| high-byte の書き込みの `~0xff00` を `~0xff` | 対象と監査の 2 テストが失敗 |
| andnpd の `fle_pandn` を `fle_pand` | `andnpd_matches_host` 失敗 |

**注（検出されなかった変異）**: 「pcmpgtd のレーン読みを `s32` から `u32` に変える」変異は通過した。`FLE_I2_LANES` は `int32_t p = x.IN[k], q = ...`（マクロ内の宣言）で値を取るため、`u32` で読んでも `int32_t` への代入で同じビット列が符号付きに戻り、比較結果が変わらない。**等価変異**と読んでいる（コードからの推論）。テストの感度が足りない例ではない。実際に比較を符号なしにする変異（`(uint32_t)p > (uint32_t)q`）は失敗した。全変異を元に戻したことは grep で確認し、通常のテストが 13 件通過（`test-a2a-3.txt`）。

## 10. 触ったファイル

変更（既存）:
- `lift/crates/fe-lift/src/lib.rs`（`reg_num` の拒否を外し、`is_high_byte`、`reg_read`、`reg_write` の高位形）
- `lift/crates/fe-lift/src/int_more2.rs`（`bswap`、先頭コメント）
- `lift/crates/fe-lift/src/cpuid.rs`（`xgetbv`）
- `lift/crates/fe-lift/src/sse_int2.rs`（Psubw、Pcmpgtd、Rcpps の 3 行）
- `lift/crates/fe-lift/src/sse_pack.rs`（Andnpd の 1 行）
- `lift/crates/fe-lift/tests/int_more2.rs`（`unsupported_stay_unsupported`）
- `lift/runtime/fle_sse_int2.h`（`fle_i2_psubw`、`fle_i2_pcmpgtd`、末尾の `#include "fle_rcp.h"`）
- `lift/runtime/fle_cpuid.c`（`fle_xgetbv_stop`）

新規:
- `lift/runtime/fle_rcp.h`、`lift/runtime/fle_rcp_table.h`
- `lift/crates/fe-lift/tests/a2a_remaining.rs`
- `lift/crates/fe-lift/tests/rcp/host_dump.c`、`gen_rcp_table.py`、`rcp_exhaustive.c`
- `docs/decisions/0047-a2a-remaining.md`、`docs/investigations/a2a-remaining-run.md`
- `lift/out-a2a/` 以下（ログ、スクリプト `audit_highbyte.py`、`extract_new_funcs.py`、`xgetbv_reach.py`、`xgetbv_run_main.c`、`probe_align.c`、`rcp/*`、`test/*`、`target/`、生成 C 2 本（各 215 MB）、`tmp/`）

触っていない: `fle_sse.h`、`fle_runtime.h`、`fle_x87.c`、他担当の runtime の `fle_*.c`（`fle_cpuid.c` と新規 2 ヘッダ以外）、`docs/decisions/0046-m1-acceptance.md`、`lib.rs` 以外の `lift_insn` の分岐順序。
