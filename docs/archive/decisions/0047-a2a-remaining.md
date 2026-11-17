# 0047: A2a の残り（Psubw、Pcmpgtd、Andnpd、Rcpps、Bswap、high-byte の Test と And、Xgetbv）

状態: 作業者（サブエージェント）が決定。依頼者（PM）の判断は含まない。実測と手順は `docs/investigations/a2a-remaining-run.md`。A2a を満たすかの判定、0046 への書き戻しは PM が行う（本作業は 0046 を編集していない）。

## 対象（実測）

0046 §3 の 43 件と一致した（Psubw 10、Pcmpgtd 8、Rcpps 8、Bswap 7、Test high-byte 7、And high-byte 1、Andnpd 1、Xgetbv 1）。形は、Psubw、Pcmpgtd、Andnpd、Rcpps が xmm 形のレジスタ形のみ（mm 形 0、メモリ形 0）、Bswap が r32 5 と r64 2、Test と And が AH/BH の `r/m8, imm8`、Xgetbv が 1 件（0x639a03）。VA は実行記録 §1。

結果: all-pdata の生成 C で、`unsupported_total` 601 → 558、命令 stub 43 → 0（残りは IAT 経由 jmp 557 と解決不能 jmp 1）。Xgetbv は名前付き停止として生成される（下記）。

## 1. Psubw、Pcmpgtd、Andnpd（xmm 形）

- 流儀: 決定 0026（`sse_int2.rs`、`fle_sse_int2.h`）に Psubw と Pcmpgtd を追加。Andnpd は `sse_pack.rs`（`andps`、`andpd` と同じ場所）に追加し、既存の `fle_pandn` を使う。メモリ形の 16 バイト境界は既存どおり `fle_ldxa`（境界外は abort）。
- 仕様の根拠: Intel SDM Vol. 2 の PSUBB/PSUBW/PSUBD（lane ごとの wrap する減算）、PCMPGTB/W/D（符号付き比較、真なら全 1）、ANDNPD（`(NOT DEST) AND SRC`）。メモリ形の例外は、PSUBW と PCMPGTD の「Other Exceptions: Non-EVEX-encoded instruction, see Table 2-21, Type 4 Class」（SDM のテキストで確認）、ANDNPD は「VEX-encoded instruction, see Table 2-21」。ANDNPD の Legacy SSE 形のメモリ例外は、SDM の記述が VEX 形にだけ Table 2-21 を引くので、Legacy SSE 形が 16 バイト境界を要求するかは **SDM の文言からは直接確認できない**。ホスト実測（`out-a2a/probe_align.c`）で、andnpd のメモリ形は境界外（オフセット 1、8）が SIGSEGV、境界内が fault なし。これを根拠に、他の同種命令（0026）と同じ扱い（境界外は abort）とした。
- フラグ: 変えない。上位ビット（MAXVL−1:128）は未変更（レジスタモデルは 128 ビット）。
- MMX 形、VEX 形は対象外。`lift_body` でエラーのまま（テストで確認）。

## 2. Rcpps（近似値命令）

### 仕様が決めるもの、決めないもの（SDM の文言）

SDM Vol. 2（325383-093US、RCPPS）:

- 「The relative error for this approximation is: |Relative Error| ≤ 1.5 ∗ 2^−12」（精度の上限だけ）。
- 「The RCPPS instruction is not affected by the rounding control bits in the MXCSR register.」
- 「When a source value is a 0.0, an ∞ of the sign of the source value is returned. A denormal source value is treated as a 0.0 (of the same sign).」
- 「Tiny results … are always flushed to 0.0, with the sign of the operand. (Input values greater than or equal to |1.11111111110100000000000B∗2^125| are guaranteed to not produce tiny results; input values less than or equal to |1.00000000000110000000001B*2^126| are guaranteed to produce tiny results, which are in turn flushed to 0.0; and input values in between this range may or may not produce tiny results, depending on the implementation.)」
- 「When a source value is an SNaN or QNaN, the SNaN is converted to a QNaN or the source QNaN is returned.」
- 擬似コード: `DEST[31:0] := APPROXIMATE(1/SRC[31:0])` と各レーン。**APPROXIMATE の定義は書かれていない**。SIMD 例外は「None」。

したがって結果の bit パターンは仕様から決まらない（実装依存）。特に tiny の境界は「実装による」と明記されている。以下は、**ホスト（AMD Ryzen 9 9950X、Zen 5）の `rcpps` に bit 一致させる**決定であり、SDM の規定ではない。

### 決定

- ホストの結果は、仮数の上位 12 ビットの索引（4096 通り）と符号と指数だけで決まる（指数 127 と 100、符号 0 と 1 で全 2^23 仮数を確認）。結果の指数は 253 − e（e は入力の指数部、1〜252）、結果の仮数の上位 12 ビットは表 `fle_rcp_tab[m >> 11]`、下位 11 ビットは 0。
- 特殊値（ホストが SDM と一致するのは確認済み）: e = 255 で仮数 0 なら符号付き 0、仮数 ≠ 0 なら `x | 0x400000`（SNaN は QNaN に、QNaN はそのまま、符号とペイロードを保つ）。e = 0（0 と denormal）は符号付き ∞。e = 253、254（結果が tiny）は符号付き 0（SDM が「実装による」とする境界の中。ホスト値をそのまま採用）。
- MXCSR の RC、FTZ、DAZ、例外マスクは結果に影響しない（SDM の記述と、ホストを 8 通りの MXCSR で全数確認した結果が一致）。例外フラグは立てない（SDM 「None」、ホストの MXCSR 状態ビットが変わらないことを確認）。
- 実装は純 C（`runtime/fle_rcp.h`）。ホストの FP ユニット、asm は使わない。表は `runtime/fle_rcp_table.h`（4096 個の `uint16_t`、生成物）。生成の再現手順: `tests/rcp/host_dump.c`（ホストの `rcpps` から 4096 値を出力）→ `tests/rcp/gen_rcp_table.py`（ヘッダを生成）。テスト `rcp_table_header_matches_host_dump` が、ヘッダの表とホストの測定が同じであることを毎回確認する。
- 表の閉形式（`1/(1+(i+h)/4096)` の丸め）は、2 組の探索（`1/(1+(i+h)/4096)` の floor と round を h 5 通りで 10 通り、`8192*4096/(4096+i+h)` の 3 種の丸めを h 17 通りで 51 通り）で試したが、最良でも 1,060/4,096 が不一致で、見つからなかった（探索のスクリプトは保存していない。実行記録 §3）。このため表をそのまま埋め込む。

### 突合の結果

- **ホストと全 2^32 個の float ビットパターンで不一致 0**（ホスト側 `exhaust.c`: モデルとホスト、`rcp_exhaustive.c`: lifted C の `fle_i2_rcpps` とホスト。どちらも 4 レーン ×2^30 回）。MXCSR 8 通りでも 0（`exhaust-mxcsr.txt`）。`cargo test` の `rcpps_all_2e32_inputs_match_host` が約 7 秒でこれを再実行する。
- 命令のテスト（`rcpps_matches_host`）: reg 形 3、メモリ形 4（境界上）、11,326 ケース、不一致 0。
- 変異: tiny 境界、denormal、SNaN の quiet 化 を変えると失敗。表の 1 入力だけを変える変異は 11,326 ケースのテストでは通り、**全数テストだけが検出**した（全数テストを残す根拠）。

### 未確認

- **Intel の CPU との違いは未確認**。Intel 実機で同じ表になる保証はない（SDM は 12 ビット精度の近似とだけ規定し、実装ごとに違うことが知られている。これは一般知識で、本作業では確認していない）。FL Studio（ゲスト）が Intel 上で出す値と bit 一致するかは、この決定では保証しない。
- 表はこのホストの測定値で、公式の資料ではない。ホストの CPU が変われば、表を作り直す必要が出る可能性がある（`rcp_table_header_matches_host_dump` が失敗して知らせる）。

## 3. Bswap

- 実装: r32（`__builtin_bswap32`、32 ビット書き込みで上位 32 ビットをゼロ拡張）、r64（`__builtin_bswap64`）。フラグ不変。SDM（BSWAP、`Operation`）のとおり。**16 ビット形（`66 0F C8+r`）は SDM が「the result is undefined」と書くので実装しない**（`lift_body` はエラー）。
- 突合: r32 と r64 を、rax rcx rdx rbx rsi rdi r8 r9 r10 で、18 ベクタ、1,440 ケース、不一致 0。全 6 フラグが入力で両方の値を取り、命令後も変わらないことを確認。変異（32 ビットのゼロ拡張を外す）を検出。

## 4. high-byte（AH、CH、DH、BH）

- 実装: `lib.rs` の `reg_read`、`reg_write` が高位形（`R[n] >> 8`、`R[n] & ~0xff00 | v << 8`）を扱う。`reg_num` の拒否（`Err("high-byte register")`）を外した。他モジュール（`insn_int.rs`、`int_ext.rs`、`int_more.rs`）は従来どおり自前の高位形ヘルパを持つ。**共通ヘルパ `is_high_byte` を `lib.rs` に追加**し、`lib.rs` の汎用経路（ALU、cmp、test、inc、dec、neg、not、シフト、setcc、mul、`int_more2.rs` の rol、ror、xchg など）が使う。
- フラグ: TEST と AND は SDM どおり CF=OF=0、SF、ZF、PF は結果、AF は未定義（C は `FLE_UNDEF`）。8 ビットの高位形でも同じ `fle_op_logic(8, …)`。
- **監査の結果**: 誤りは見つからなかった（修正の必要なし）。理由: 旧 `reg_num` が高位形を拒否していたので、`lib.rs` 経由の命令は「別のレジスタとして誤って扱う」のではなく unsupported で止まっていた。他モジュールは高位形を自前で処理しており、`mmx.rs` は拒否、SSE 系は 32/64 ビット GPR だけ。all-pdata の閉包で高位形のオペランドを持つ命令 54 件（mov 35、lock cmpxchg 等 6、movzx 5、test 7、and 1）を、旧生成 C と新生成 C で比べた結果、mov、lock、movzx の 46 件は旧新とも高位形のコード、test と and の 8 件が unsupported から高位形に変わった。
- 突合: 8 ビットの命令全形を、8 つの 8 ビットレジスタ（AL CL DL BL AH CH DH BH）の組合せで、ホストと比べた。対象（test、and）378 ベクタ、15,120 ケース、監査（add、or、adc、sbb、sub、xor、cmp、inc、dec、not、neg、mul、imul、rol、ror、shl、shr、sar、mov、xchg、xadd、cmpxchg、movzx、movsx、setcc、div、idiv）2,525 ベクタ、35,350 ケース。**不一致 0**。未定義フラグは、TEST と AND は AF だけ（exact）、他の命令は SDM が未定義とする集合の内側（`allowed`）であることを確認。変異（読み出しの `>> 8` を外す、書き込みのマスクを変える）をそれぞれ検出。
- 制限: 8 ビットの CL シフトで masked count が 9〜31 は、SDM が結果を未定義とし lift が既存どおり止まる（`fle_bad_count`）。突合ではカウントを 0〜8 に制限した。本作業の変更ではない。

## 5. Xgetbv

### 根拠の確認

- SDM（XGETBV、`Operation`）は `EDX:EAX := XCR[ECX]` と書くだけで、XCR0 の値を決めない（XCR0 は OS が設定する）。`#UD` の条件に `CPUID.01H:ECX.XSAVE[26] = 0` と `CR4.OSXSAVE = 0` がある。**実装できる XCR0 の値の根拠は確認できなかったので、値を返す実装はしない**（依頼の指示どおり）。
- 0x639a03 の逆アセンブル（関数 f_6399e0、実行記録 §6）を再確認した。`cpuid(0)` で `eax < 1` なら終了、`cpuid(1)`、`and ecx,0x18000000; cmp ecx,0x18000000; jne 0x639a15`（OSXSAVE と AVX が両方立っていないと `xgetbv` に進まない）、`xor ecx,ecx`、`xgetbv`。0032 の記録と一致した。
- ランタイムの CPUID（`fle_cpuid.c`）は、どの入力でも葉 1 の ECX が `0x18000000` を満たさず、XSAVE も立てない（テストで 10 通りの EAX × 4 通りの ECX を確認）。したがって `jne` が必ず取られ、`xgetbv` は実行されない。生成した f_6399e0 を単体で実行して、eax = 0 で戻り、rbx が復元され、停止関数が呼ばれないことを確認した。
- 静的な到達性: `f_6399e0` の祖先は f_639a20、f_63a330、f_63a5e0 の 3 関数で、f_63a5e0 は生成 C に直接の呼び元がなく、DLL のポインタ表（初期化か終了の表に見える。推測）から参照される。関数への到達はありうるが、`xgetbv` の手前の分岐で守られる。この経路が実際に実行されるかは、harness の実行では確認していない。

### 決定

- `xgetbv` は「到達したら名前付きで止まる命令」として明示的に生成する。生成 C: `{ void fle_xgetbv_stop(uint64_t); fle_xgetbv_stop(UINT64_C(0x639a03)); }`。`runtime/fle_cpuid.c` の `fle_xgetbv_stop` が `fle: xgetbv at 0x639a03: XCR0 is not defined (CPUID leaf 1 ECX reports no OSXSAVE; the SDM gives no value)` を stderr に出して `abort()`。ハンドラ名は `xgetbv_stop`（`handled` の計数に出る）。
- これは `fle_unsupported`（「未実装の命令」）とは別の、**決まった停止**である。`unsupported_total` の計数には含まれない（生成 C の `grep -c fle_unsupported` が 558）。A2a の計数から外す根拠: (a) 値を返す実装を作れる仕様の根拠が無い（SDM は XCR0 の値を決めない）、(b) ランタイムの CPUID が `xgetbv` の手前の分岐を必ず通らせない（テストと単体実行で確認）、(c) 到達した場合は黙って値を作らず、命令と VA を名前に出して止まる（0016 の「決まらないものは abort」と同じ運用）。**A2a の判定にこの除外を使うかは PM が決める。**
- 条件: CPUID が OSXSAVE または AVX を主張するように変える場合は、先に XCR0 の値の決定が必要になる（0032 と同じ）。そのとき `fle_xgetbv_stop` の扱いを見直す。テスト `runtime_cpuid_never_reports_osxsave_and_avx` が、CPUID の変更でこの前提が崩れたときに失敗して知らせる。

## 実装しなかったもの

- Psubw、Pcmpgtd、Andnpd、Rcpps の MMX 形と VEX 形（`lift_body` はエラー）。閉包に出ない。
- Bswap の 16 ビット形（SDM が undefined）。
- XGETBV の値（上記）。ECX = 1 など XCR の他の番号も実装しない。

## 確認できていないこと

- Intel CPU での Rcpps（上記）。
- Zen 5 以外の AMD の CPU での Rcpps（ホストは 1 台）。
- all-pdata 全体の wasm リンクと node 実行は再実行していない。新しい 24 関数の C と新ヘッダは、clang と emcc で `CFLAGS_GEN` と同じフラグでコンパイルできた（実行記録 §7）。
- 生成 C がこれらの命令を実行経路で踏むかは未測定（A4 の停止点は別の位置）。

## 影響

- `lib.rs` の `reg_num` から高位形の拒否が無くなった。8 ビットで AH/CH/DH/BH を含む命令の `lift_body` は、これまでの `Err` から成功に変わる（`rol ah,cl` など）。`tests/int_more2.rs` の `unsupported_stay_unsupported` を、この変更に合わせて編集した。
- `lift/runtime/fle_sse_int2.h` の末尾に `#include "fle_rcp.h"` を足した（`fle_rcp.h` は 1 つの関数と表 1 つ、4096 エントリの静的配列。`fle_sse.h` を include する TU ごとに 8 KB のデータが入る。表は `static const`）。生成 C、runtime の `.c`、テストの C は `fle_sse.h` 経由でこれを取り込む。`fle_sse.h` 自体は触っていない（他担当の変更中）。
