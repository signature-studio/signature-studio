# MXCSR 状態ビットと PREFETCHh の実行記録

日付: 2026-10-21。設計と判断は `docs/decisions/0038-mxcsr-flags.md`。「実測」はツールの出力、「推測」は未検証の解釈。

## 1. 変更したファイル

新規: `lift/runtime/fle_mxcsr_flags.h`、`lift/crates/fe-lift/src/sse_fp_flags.rs`、`lift/crates/fe-lift/src/prefetch.rs`、`lift/crates/fe-lift/tests/mxcsr_flags.rs`、`lift/harness/oracle/build5.sh`、`lift/harness/oracle/run5.sh`、`docs/decisions/0038-mxcsr-flags.md`、本ファイル。生成物は `lift/out-mxcsr/` 配下（`out/`、`pd/`、`pdbase/`、`oracle/`、`ctl/`、`rt-base/`、`bench/`、`bin/fe-lift-base`、`test/`、`target/`、`target-ctl/`、`tmp/`、`test-ws.txt`、`test-ctl-int.txt`、`build5.txt`、`run5.txt`）と `lift/out-oracle/results/run5/`。

局所変更（既存ファイル、編集ツール）: `lift/crates/fe-lift/src/sse_more.rs`（`#[path]` で 2 モジュールを宣言、`lift` の先頭で 2 つを呼ぶ。+12 行）、`lift/runtime/fle_sse.h`（末尾に `#include "fle_mxcsr_flags.h"`。+3 行）、`lift/harness/oracle/run_diff2.py`（`BIN` を環境変数 `ORACLE_BIN` で切り替え可能に。1 行）。`lib.rs`、`fle_dispatch.h`、`imports_table.rs` は触っていない。run4 は上書きしていない。

## 2. 実測

### 2.1 生成 C（全 pdata、`fe-lift lift --pdata`、現在のワークツリー）

| | 変更前 | 変更後 |
|---|---|---|
| `c_bytes` | 217,968,523 | **215,539,533**（−2,428,990、−1.1%） |
| `c_lines` | 2,880,335 | 2,880,419 |
| `unsupported_total` | 698 | **603**（−95） |
| 分割 C（16 分割、3,700 関数/ファイル）合計 | 236,713,829 | 234,300,124 |
| 分割 .o（clang -O0 -g、16 並列）合計 | 526,770,560 | **531,380,416**（+4,609,856、+0.9%） |

依頼文の「215MB」は、他の担当の変更が入る前の `out-pdata` の値（215,649,724）。変更前の値は、今のワークツリーのまま MXCSR の変更だけを外して再生成した（`out-mxcsr/out/base.c`）ものを使った。生成 C に `fle_mx_` を含む行は 49,997 行、4,245,954 バイト。減ったのは、旧ハンドラの C 式（`fle_addsd(X[0].u64[0], fle_ld(…))` の形）より新しい呼び出しの方が短いため（推測。行ごとの比較はしていない）。

未対応の減少（`unsupported`）: Mulpd 25、Addpd 13、Cvtdq2ps 9、Subpd 8、Prefetcht0 5、Cvtpd2ps 2、Cvtps2pd 2（計 64）と、IAT の `Jmp` が 588 → 557（−31、他の担当の変更の分で、今回の作業とは無関係）。ハンドルされた命令数（生成 C の `handled` 表）で今回の対象に当たるのは 50,002 件。

### 2.2 実行時間（ミクロ、`out-mxcsr/bench/bench2.txt`、-O0、実機）

| | ns/op |
|---|---|
| addsd 旧（正確な 1+0） / 新 | 5.08 / 16.42 |
| addsd 旧（不正確 1+1/3） / 新 | 5.29 / 18.40 |
| divsd 旧 / 新 | 6.76 / 23.49 |
| ucomisd 旧 / 新 | 4.68 / 18.57 |
| cvtsi2sd（int64）旧 / 新 | 1.28 / 14.01 |

プログラム全体の実行時間への影響は測っていない（FLEngine はまだ最後まで走らない）。オラクル 16,000 case の実行時間は 5 分 38 秒（run4）→ 5 分 40 秒（run5）で、差は誤差の範囲（ビルドの並列度と他の作業の負荷が違うので厳密な比較ではない）。

### 2.3 テスト

`lift/crates/fe-lift/tests/mxcsr_flags.rs`: **9 件、すべて pass**（単独で 22.7 秒）。ホストとの突合は 6 件のテストで合計 **127,104 case**（596 エンコーディング）: arith 42,672（112 エンコーディング。四則・min・max・sqrt の ss/sd/ps/pd の 4 つのレジスタ/メモリ形）、cmp 15,360（128: 述語 0..7）、comi 11,872（16）、cvt_fp 16,000（40）、cvt_int 15,600（44）、round 25,600（256: imm8 0..15）。差は **0**。どのグループも、ホストが立てた状態ビットの和集合は 0x3F（IE DE ZE OE UE PE の全部。テストの出力 `host_raised_union`）。テスト内の `assert` は、arith が 6 つ全部、cmp と comi が IE と DE、cvt_fp が IE DE OE UE PE、cvt_int が IE DE PE、round が IE PE。非既定の制御ビット（RC≠0、FTZ、DAZ のいずれか）の case は 89,288 件（arith 16,758、cmp 14,406、comi 4,517、cvt_fp 15,015、cvt_int 14,578、round 24,014）。新しい状態ビットを立てた case は arith 22,831、cmp 4,059、comi 2,998、cvt_fp 9,682、cvt_int 8,302、round 8,122。

`flags_match_sdm_expectations`: 61 件、SDM と差 0（最初の実行で 5 件が期待値の誤りだったので、テストの側を直した。mulsd の tiny × tiny が 2^-511 では 2^-1022 に届かず UE にならなかった（入力を 2^-767 に変更）、cvtdq2ps/cvtps2dq/cvttps2dq は上位レーンも変換されるので入力の上位 64 ビットを 0 にした、cvtss2si の非正規化数入力は DE が立たず PE のみ（ホストの結果。SDM の節は Denormal を含むが、cvtss2si の非正規化数→整数で DE が立たない実機の挙動に合わせた））。

`unmasked_raised_flag_is_a_named_abort`、`prefetch_never_faults_and_changes_nothing`（32 エンコーディング、C 側 288 case と、ホストの同じ 288 回）: pass。

ワークスペース全体（`cargo test --release --workspace --no-fail-fast`、フォアグラウンド、`out-mxcsr/test-ws.txt`、22 分 2 秒）: **pass 593、fail 6、ignored 0**。失敗の 6 件は `int_ext`（`movsx_matches_host`、`idiv_imul_matches_host`）と `int_more`（`xadd_matches_host`、`cmpxchg_matches_host`、`div_matches_host`、`bt_matches_host`）。いずれもメモリオペランドを含む整数命令の突合で、SSE とは無関係。私の変更を外した複製（`out-mxcsr/ctl/`: `sse_more.rs` から呼び出しを除き、`runtime/` を変更前のコピー `rt-base` にしたもの）でも同じ 6 件が同じ名前で失敗した（`out-mxcsr/test-ctl-int.txt`）。原因は調べていない（git の差分では `tests/int_ext.rs`、`int_more.rs` を含む多数のテストが他の担当により変更中。内容は確認していない）。

### 2.4 オラクル v2 の再実行（run5、`lift/out-oracle/results/run5/`）

再現: `lift/harness/oracle/build5.sh`（lift 側のみ再ビルド: `fe-lift lift --pdata` → `out-pdata/split_c.py` → clang。19 秒）、`lift/harness/oracle/run5.sh`（`ORACLE_BIN=lift/out-mxcsr/oracle/bin` を設定して `run_diff2.py`）。run2.sh は `out-oracle/bin` を使い、lift 側は `out-pdata/parts/*.o`（古い生成 C）を使うので、そのままでは新しい runtime とレクサを使わない。run5 の native 側は run4 と同じバイナリ（コピー）、選別・seed・入力・比較は run4 と同じ（1,000 関数、16 case、seed 1、ネガコン 200）。

| 判定 | run4 | **run5** |
|---|---|---|
| bit_exact | 12,805 | **13,535** |
| mismatch_core | 0 | 0 |
| mismatch_aux | 717 | **0** |
| lift_unsupported | 13 | **0** |
| lift_stop_other | 0 | 0 |
| native_fault | 2,465 | 2,465 |

差分: bit_exact は +730 = 717（MXCSR 状態ビットだけの差が消えた）+ 13（`Prefetcht0` で止まっていた 0x69d8b0 の case）。関数単位は bit_exact_all_comparable 900 → 963、has_mismatch_aux_only 62 → 0、lift_not_executable 1 → 0、native_fault_all 37 → 37。`mxcsr_lift_ne_reset`（lift の MXCSR が 0x1F80 以外）は 0 → **729**、`mxcsr_native_ne_reset` は 717 → 729（同数）。EFLAGS の定義済みフラグの差は 0、MXCSR 制御ビットの差 0、x87 の差 0。native_fault の内訳は run4 と同じだが、`scratch_range_exceeded` の lift 側が abort 3 → fault（3 件が native と同じ fault になった。この 3 件は 0x69d8b0 の case で、`Prefetcht0` で止まっていたため（推測。case ごとの確認はしていない））。

ネガコン（203 件）: 出力の差で検出 127 → 123、native fault も含めて 183 → 187、未検出 20 → 16（`detected_core` 117 → 113（定数返却 3 件を含む）、`detected_aux_only` 10 → 10、`detected_by_native_fault_only` 56 → 64、未検出 20 → 16（masked 7 → 4、等価 13 → 12））。run4 は `mxcsr_sticky` だけの差を「検出」に数えない扱い（0036 §3）で、run5 には状態ビットの差自体が無い。差の原因は、反転対象の命令に SSE 浮動小数・prefetch を含む関数の挙動が変わったこと（推測。個別には確認していない）。

## 3. FLEngine の MXCSR 書き込み（静的確認）

`0038` §3 に表と結論を書いた。要点（事実）: `ldmxcsr` 6 か所（0x40a36c、0x40a39d、0x40eeab、0x40f530、0x475c8a、0x63a6d0）。`.data` の 0x13e4d50 は初期値 0x9F80（FTZ が立つ）で、0x63a6d0 がこれを読み込む（65 か所から call）。DAZ は 0x63ab2b で立つ経路がある。RC を非ゼロにする経路は見つからなかった。FTZ/DAZ は実機の命令にそのまま反映するので abort にはしていない。0x63ab34 の `0xFFFFFFFF` の書き込みは、`fle_mxcsr_load` が予約ビットで abort する（既存の挙動）。これらが FLEngine の実行で実際に通るかは、今回確認していない。

## 4. 試行錯誤

- 状態ビットの取得方式の予備ベンチ（`out-mxcsr/bench/bench.c`）: 毎演算 `ldmxcsr`+`stmxcsr`（sync）、ホストを parked にして立った時だけ `ldmxcsr`（parked）、`stmxcsr` で比較してから `ldmxcsr`（check）、何もしない（bare）の 4 方式が 12.1〜12.8 ns/op で、差が判別できなかった（-O0 の関数呼び出しが支配的）。実際のヘルパを測った `bench2.c`（§2.2）では 11〜13 ns の増加。簡単な毎演算 sync を採用した。
- テストの期待値の誤り（§2.3 の 5 件）は、最初の実行で見つかり、期待値を SDM とホストの結果に合わせて直した。
- `ctl/` の複製（`cp -r crates`）はテストが書く出力先（`ctl/out-int-ext`、`ctl/out-more`）を作った。

## 5. 制約の遵守と残置物

`rm`、`>` による既存ファイルの上書き、git 操作、`/tmp`、sleep/timeout は使っていない（`time` は計測のみ）。TMPDIR は `lift/out-mxcsr/tmp`（oracle は `out-mxcsr/oracle/tmp`、`out-oracle/tmp`）。権限拒否はなし。cargo は `CARGO_TARGET_DIR=lift/out-mxcsr/target` で、ワークスペースの Cargo.toml のまま動いた。後始末できていないもの（削除禁止）: `out-mxcsr/` 配下の生成物（`oracle` 754 MB、`pdbase` 729 MB、`out` 451 MB、`pd` 224 MB、`target` 127 MB、`target-ctl` 29 MB、`ctl` 2.8 MB。合計約 2.3 GB。単独で 2 GB を超えるファイルはなく、`out-pdata`・`out-oracle` の大きなファイルの複製もない。ただし `out-mxcsr/oracle/parts`（507 MB）と `pdbase/parts` は新規に生成した .o）、`out-mxcsr/pd/`・`pdbase/` の分割 C。
