# A5 native オラクルの再実行 その 3（HEAD 0d58b0f、FLEngine の生成 C SHA feb5b908…）

日付: 2026-10-25。対象は `git archive HEAD` のスナップショット（依頼時の HEAD は 0d58b0f。測定の途中で dc27f4d が載ったが、`git diff 0d58b0f HEAD -- lift` は差分なしで、`lift/` は d618380 から不変）。測定のみで、修正はしていない。決めない。設計は `docs/decisions/0035-a5-oracle.md`、`0036-a5-oracle-v2.md`、`0038-mxcsr-flags.md`。手順の正本は `docs/investigations/a5-oracle-rerun-2.md`（前回、HEAD b21d051）と `a5-oracle-rerun.md`（その前、d6c24be）。

凡例: **実測** = この作業でツールを走らせて得た出力。**読み** = 実測からの解釈で、直接は確かめていないもの。**未確認** = 確かめていない。

## 1. 結論

1. **実測**: 生成 C の SHA-256 は **`feb5b9087f8d7a8a1282900212d7bd1f5b6071b0326e7149642f98c168e8080f`**（依頼の期待値と一致）。同じバイナリで 2 回生成して同じ SHA（決定的）。
2. **実測**: seed 1（前回と同じ手順・パラメータ）のカテゴリ件数は前回（b21d051）と**すべて一致**: bit_exact 13,535、mismatch 0、lift_unsupported 0、native_fault 2,465。ネガコンも同じ（出力差 123/203、native fault 込み 187/203、未検出 16）。`a5-cases.json`、`a5-cases.csv`、`a5-functions.json`、`a5-mismatches.json`、`a5-negctl.json`、`a5-summary.json`、`pure.json`、`argptr.json`、`exclude-stats.json` は前回の同名ファイルと **`cmp` で全バイト一致**。一致しなかったのは `a5-native-faults.json` だけで、差は 2,465 件中 10 件の `rip`（すべて `timeout_2s(loop)`、2 秒のアラームがループのどこで落ちたかの違い）のみ。
3. **実測**: seed 2 も同様: bit_exact 13,616、mismatch 0、native_fault 2,384、ネガコン 123/187/16。`a5-cases.json` ほか 8 ファイルが全バイト一致。不一致は `a5-native-faults.json`（10 件の `rip` のみ）と `a5-summary.json`（`trace_consistency` の 2 項目 ±1。§4.3）。
4. **実測**: 前回の生成 C との `diff` は **35 行**。変更は応答表の追加 8 行（宣言 8、`fle_dispatch_table` 8、`fle_resp_table` 8、件数 2）だけで、**関数本体の変更は 0 件**（全 59,297 関数の本体を比較）。純計算 9,901 件、seed 1/2 の選択各 1,000 件も本体は同一。
5. **実測**: seed 3（新規 1,000 関数 × 16 ケース、seed 1・2 と**重ならない**ように選んだもの）: bit_exact 13,482、**mismatch 0**、lift_unsupported 0、native_fault 2,518（15.7%）。
6. **実測（要注意）**: もう 1 本、素直に実行した seed 3（`--pure` を絞らない。seed 1・2 との重なり 204 関数）では **mismatch_core が 4 件**（2 関数、全部 `lift_timeout`）。原因の切り分け（§6）: 4 件とも、**2 秒のアラームを延ばすと lift 側が最後まで走って native と `bit_exact`**（アラーム 1,200 秒で 114〜487 秒かかり、全 GPR・XMM・メモリ・フラグが一致）。lift の値の誤りではなく、**lift 側が native より約 2〜3 桁遅いために 2 秒/10 秒の上限を超えた**ケース。**読み**: オラクルの限界（時間の上限）で、lift の欠陥の証拠ではない。ただし「長い実行で lift が遅い」こと自体は測定値として残す。**修正はしていない**。
7. 範囲: 言えるのは「選んだ関数のうち native が戻ったケースで、HEAD の lift が GPR・XMM・メモリ・EFLAGS（定義済みフラグ）・MXCSR・x87 の制御状態まで native と一致した」ことだけ。純計算関数に限る。import・間接 call・x87・GS を持つ関数は対象外（§9）。**今回の再実行は、決定 0075〜0079・0087・0089〜0094 の変更を検査していない**（FLEngine の関数本体がこれらで 1 つも変わっていないため。§9）。

## 2. 対象と環境

| 項目 | 内容 | 区分 |
|---|---|---|
| HEAD | 依頼時 `0d58b0f019b232c7fba56b346287a8b658075e24`。測定中に `dc27f4d`（`docs/investigations/a3-wasm-recheck-2.md` の追加のみ、222 行）が載った。`git diff --quiet 0d58b0f HEAD -- lift` と `git diff --quiet d618380 HEAD -- lift` はともに終了コード 0（差分なし） | 実測 |
| スナップショット | `git archive HEAD \| tar -x -C lift/out-a5-head/src/`（展開は 0d58b0f の時点。10 MB、`binaries/` は含まれない）。ワークツリーの `lift/` は読んでいない。ワークツリーの未コミット変更（fe-decode の `jumptab.rs` 等、`docs/decisions/0095-*`）はスナップショットに入っていない | 実測 |
| PE | `/home/ubuntu/work/signature-studio-develop/binaries/win64/FLEngine_x64.dll`、SHA-256 `b2c64da40921cf262c15c5b25c2b0422e4d1851e953c87e0bc38ed2c19f9496d`（確認済み）を `FLENGINE_PE` で絶対パス指定 | 実測 |
| 生成 C | `fe-lift lift … --pdata`: 215,709,232 B、2,885,436 行、functions 59,297、closure_instructions 2,528,030、bad_nodes 0、unsupported_total 525（`Jmp: indirect through IAT slot` 524、`Jmp: indirect target unresolved` 1）、SHA-256 **`feb5b908…8080f`**（期待値と一致）。2 回目の生成も同じ SHA | 実測 |
| 前回の生成 C | `lift/out-a5-m85/src/lift/out-oracle/gen/a5m85-pdata.c`: 215,707,512 B、2,885,412 行、SHA-256 `43985d62321a90f562ce1b8144253e00af13df795e4190d26b6b8ce2537137a1` | 実測 |
| 環境 | AMD Ryzen 9 9950X（16 CPU）、Linux 7.0.0-38-generic、clang 21.1.8、rustc 1.99.0、capstone 5.0.7、python 3.14.8（uv の cpython）。前回と同じ機械 | 実測 |
| 並行作業 | 別の subagent が同じ機械で動いていた（census、deferred、swapped の各作業ディレクトリがある）。負荷は測っていない | 未確認 |

## 3. 実行コマンド

作業ディレクトリ `R=/home/ubuntu/work/signature-studio-develop/lift/out-a5-head`。cwd は `$R/src/lift/out-oracle`（`H=../harness/oracle`）。全 python に `PYTHONDONTWRITEBYTECODE=1`、`TMPDIR=$R/tmp`、`CARGO_TARGET_DIR=$R/target`、`FLENGINE_PE=<上の絶対パス>`。

```
# 0. スナップショット
mkdir -p $R/src $R/tmp $R/target
git archive HEAD | tar -x -C $R/src/

# 1. lifter のビルド（3.2 秒）と生成 C（2.4 秒）。SHA を先に確認
cd $R/src/lift && cargo build --release -p fe-lift
cd $R && ./target/release/fe-lift lift $FLENGINE_PE src/lift/out-oracle/gen/a5head-pdata.c --pdata
sha256sum src/lift/out-oracle/gen/a5head-pdata.c    # feb5b908…8080f（一致）

# 2. 分割→clang→ドライバのリンク（19.7 秒、出力は $R/build.txt。警告・エラーの行は 0）
sh $R/a5head-build.sh     # a5m85-build.sh のパスだけを lift/out-a5-head に替えたもの。split_c.py 3700（17 分割）、
                          # clang のフラグ・順序は前回と同じ

# 3. 選別・ポインタ判定・除外集計（それぞれ数秒）
python3 $H/select_pure.py $FLENGINE_PE gen/new-part-00.c results/pure.json
python3 $H/argptr.py $FLENGINE_PE results/pure.json results/rerun/argptr.json
python3 $H/exclude_stats.py $FLENGINE_PE gen/new-part-00.c results/rerun/exclude-stats.json > results/rerun/exclude-stats.txt

# 4. seed 1（4 分 43.7 秒 real）
python3 $H/run_diff2.py --pe $FLENGINE_PE --pure results/pure.json --argptr results/rerun/argptr.json \
    --exclude results/rerun/exclude-stats.json --out results/rerun --count 1000 --n 16 --seed 1 --negctl 200 --jobs 16

# 5. seed 2（3 分 3.5 秒）。argptr / exclude は seed 1 のコピー
python3 $H/run_diff2.py ... --out results/seed2 --count 1000 --n 16 --seed 2 --negctl 200 --jobs 16

# 6. seed 3（重ならない選び方、2 分 33.4 秒）。--pure だけ別のファイル
python3 $R/mk_seed3_pool.py results results/seed3/pure-excl12.json     # seed 1・2 の選択を除いた pure.json を新規に作る
python3 $H/run_diff2.py ... --pure results/seed3/pure-excl12.json --out results/seed3 --count 1000 --n 16 --seed 3 --negctl 200 --jobs 16

# 7. seed 3（素直、--pure を絞らない。3 分 51.9 秒）
python3 $H/run_diff2.py ... --pure results/pure.json --out results/seed3plain --count 1000 --n 16 --seed 3 --negctl 200 --jobs 16
```

ドライバの終了コードは 4 本とも native 0、lift 0、native-trace 0。emcc/wasm は使っていない。`run_diff2.py`、`select_pure.py`、`argptr.py`、`exclude_stats.py`、`oracle2_common.c`、`native_drv2.c`、`lift_drv2.c`、`ms_call2.S`、`image.c`、`split_c.py` は前回のスナップショットと **`cmp` で同一**（実測）。**`run_diff2.py` は改変していない**。

### 3.1 前回との手順の差

- 前回の `a5m85-build.sh` と同じ内容で、パスと生成 C のファイル名（`a5head-pdata.c`）だけが違う（実測）。
- seed 3 の「重ならない選び方」は `run_diff2.py` の引数では指定できないので、**`--pure` に渡す JSON を絞る**方法を取った（§7.1）。`run_diff2.py` 自体は変えていない。

## 4. seed 1 と seed 2: 前回との比較

### 4.1 カテゴリ（実測、各 16,000 ケース）

| 判定 | seed 1 前回（b21d051） | seed 1 今回 | seed 2 前回 | seed 2 今回 |
|---|---|---|---|---|
| bit_exact | 13,535 | **13,535** | 13,616 | **13,616** |
| mismatch_core / aux | 0 / 0 | **0 / 0** | 0 / 0 | **0 / 0** |
| lift_unsupported / lift_stop_other | 0 / 0 | **0 / 0** | 0 / 0 | **0 / 0** |
| native_fault | 2,465 | **2,465**（15.4%） | 2,384 | **2,384**（14.9%） |

関数単位: seed 1 は bit_exact_all_comparable 963、native_fault_all 37。seed 2 は 964 / 36（前回と同じ）。ポインタ入力別・v1 と同じ 200 関数・native_fault の原因の内訳・`native_fault_same_fault_addr`（2,415 / 2,344）・MXCSR の統計（729 / 859）・EFLAGS 定義済みフラグの差 0・x87 の差 0・実行された命令の割合（91.4% / 91.5%）・`bit_exact_cases_with_image_or_scratch_writes`（4,820 / 5,061）も、前回の記録と同じ（`a5-summary.json` の seed 1 は全バイト一致、seed 2 は §4.3 の 2 項目を除いて一致）。

### 4.2 結果ファイルの `cmp`（実測）

前回（`lift/out-a5-m85/src/lift/out-oracle/results/`）と今回（`lift/out-a5-head/src/lift/out-oracle/results/`）で比べた。

| ファイル | seed 1 | seed 2 |
|---|---|---|
| `pure.json`（9,901 件）、`argptr.json`、`exclude-stats.json`、`exclude-stats.txt` | 全バイト一致 | `argptr.json`、`exclude-stats.json` は一致（seed 1 のコピー） |
| `a5-cases.json`、`a5-cases.csv`（16,000 ケースの判定と detail） | 一致 | 一致 |
| `a5-functions.json`、`a5-mismatches.json`、`a5-negctl.json`（203 件の個別結果） | 一致 | 一致 |
| `a5-summary.json` | 一致 | **差あり**（§4.3） |
| `a5-native-faults.json`（2,465 / 2,384 件） | **差あり**（10 件、`rip` のみ） | **差あり**（10 件、`rip` のみ） |
| `run.txt` | 一致 | 一致 |

`a5-native-faults.json` の差は、record の ID 集合は同じで、違う項目は `rip` だけ、原因はすべて `timeout_2s(loop)`。
- seed 1: `5f0240.10`、`5f0240.13`、`ad7640.7`、`ad7640.9`、`ad7640.10`、`ad7640.11`、`ad7710.2`、`ad7710.12`、`ad7710.13`、`ad7710.15`（ループ中のどこで 2 秒のアラームが落ちたかの違い）。
- seed 2: `54f810.0`、`54f810.8`、`5f0460.0`、`5f0460.8`、`5f0460.14`、`ad7770.2`、`ad7770.7`、`ad7770.10`、`ad7770.12`、`ad7770.13`。

**読み**: 同じ無限（または極めて長い）ループの中の、アラームが落ちた位置の違い。判定（native_fault）は変わらない。

### 4.3 `a5-summary.json`（seed 2）の差（実測）

2 項目だけ: `trace_consistency.both_fault` 2,350 → 2,349、`trace_consistency.status_differs:fault->timeout` 23 → 24。1 ケース（`44af80.15`）の**単一ステップ（trace）実行**の結果が、前回は native 通常実行と同じ `fault`（SIGSEGV、`fault_addr 0x20100000`、`fault_rip 0x44afb8`）、今回は `timeout`（10 秒の上限、16 並列の壁時計時間による）。native の**通常実行**は前回も今回も同じ fault なので、`bit_exact` の判定とカテゴリには影響しない。trace はネガコンの対象選びとカバレッジの参考値にだけ使う。

### 4.4 生ログ（`work/out-*.jsonl`）の差（実測）

`work/` を case 単位で前回と比べた（`lift/out-a5-head/cmp_work.py`、読み取りのみ）。

- ケースの集合は全種類で同一（only-old / only-new が 0）。status の件数は seed 1 の lift・native・trace・再実行すべてで同じ（seed 1: lift ok 13,534 / fault 2,415 / abort 33 / timeout 18、native ok 13,535 / fault 2,448 / timeout 17、trace ok 13,528 / fault 2,425 / timeout 47）。ケース単位でも、seed 1 は status が変わったケースが 0 件。
- 比べた相手は前回（b21d051）の結果。rerun-2 §6.2 にある「lift の first-pass timeout が 1 件減る」「trace が timeout → fault になる 13 件」は、その前（d6c24be）との比較で出た差で、今回（b21d051 との比較）の seed 1 には出ていない。
- seed 1 の native は、ok 13,535 件と fault 2,448 件が byte 単位で一致、差は timeout 17 件の `fault_rip` / `fgpr` だけ。trace の ok 13,528 件も一致。lift の ok 13,534 件も一致。差があるのは次の項目だけ:
  - lift の fault 2,415 件は `fgpr`（fault 時のホストのレジスタ）だけが違う。
  - lift の abort 33 件（sigfpe）は `fault_addr` と `fgpr`。
  - timeout は `fault_rip`、`fgpr`。
- seed 2 も同様。**status が変わったケースは 1 件だけ**: 上の `44af80.15` の trace（fault → timeout）。

**読み**: `fgpr` と abort の `fault_addr` は、ホストのスタックのアドレスやプロセス ID に由来する項目と見える（前回の記述と同じ）。比較の対象ではない。個別の値の由来は今回も確認していない。timeout の位置の差は壁時計時間の差。

### 4.5 ネガティブコントロール（実測）

| 結果 | seed 1 前回 | seed 1 今回 | seed 2 前回 | seed 2 今回 |
|---|---|---|---|---|
| detected_core | 113 | **113** | 116 | **116** |
| detected_aux_only | 10 | 10 | 7 | 7 |
| detected_by_native_fault_only | 64 | 64 | 64 | 64 |
| undetected_state_changed_but_masked | 4 | 4 | 10 | 10 |
| undetected_equivalent_no_state_change | 12 | 12 | 6 | 6 |
| 出力の差で検出 | 123/203 = 60.6% | 同じ | 123/203 | 同じ |
| native fault 込み | 187/203 = 92.1% | 同じ | 187/203 | 同じ |
| 未検出 | 16 | 16 | 16 | 16 |

`a5-negctl.json`（個別の 203 件）が前回と全バイト一致なので、未検出 16 件の関数・命令も前回の一覧（rerun-2 §5.3、§7.3）と同じ。定数返却関数の imm 3 件（seed 1: 0x410030、0x4536d0、0x453900）はすべて detected_core。

## 5. 前回からの生成 C と runtime の差（実測）

### 5.1 生成 C（b21d051 の `a5m85-pdata.c` → 今回の `a5head-pdata.c`）

`diff` の出力は 35 行（`lift/out-a5-head/gen-diff-m85-vs-head.txt`）。全 59,297 関数の本体（`void f_XXXX(void)` から次の同形の行まで）の SHA-256 を、前回作られた `lift/out-a5-m85/cmp_bodies.py`（読み取りのみ）で比べた（`lift/out-a5-head/cmp-bodies-m85-vs-head.txt`）。

| 項目 | 結果 |
|---|---|
| 関数の集合 | 前回・今回とも 59,297、片側だけ 0 |
| 本体が同一 | **59,297** |
| 本体が変わった | **0** |
| 純計算 9,901 件のうち本体が変わったもの | **0** |
| seed 1 の選択 1,000 / seed 2 の選択 1,000 のうち本体が同一 | 1,000 / 1,000 と 1,000 / 1,000 |

**変わった関数はない**（列挙するものがない）。diff の中身は、応答表の追加だけ:

- `fle_imp_*` の宣言 8 行: `GetModuleHandleA`、`AcquireSRWLockExclusive`、`ReleaseSRWLockExclusive`、`AcquireSRWLockShared`、`ReleaseSRWLockShared`、`WakeConditionVariable`、`WakeAllConditionVariable`、`SleepConditionVariableSRW`
- `fle_dispatch_table` の行 8（擬似 VA 0x7ff900001050〜0x7ff9000010c0）、`fle_resp_table` の行 8（すべて kernel32.dll）
- `fle_dispatch_count` 59,557 → 59,565、`fle_resp_count` 260 → 268

**読み**: 決定 0091（GetModuleHandleA）と 0092（SRW ロックと条件変数）の応答表の追加と一致する（コミットログに両決定が b21d051..HEAD にある。決定文書の中身までは今回読んでいない）。決定 0089・0090・0093・0094（MSVC switch の表の解決）は、FLEngine では Blend2D 側の件数が変わるだけで、FLEngine の生成 C は変わらない（今回の実測で関数本体 0 件）。`unsupported_total` は前回と同じ 525。

### 5.2 runtime と lifter のソース（スナップショット同士）

`lift/runtime/` は b21d051 のスナップショットと比べて**変更が 7 ファイル**: `fle_load.c`、`fle_locale.{c,h}`、`fle_modules.{c,h}`、`fle_sys.{c,h}`（`diff -rq`）。追加は `runtime/test/` の 7 ファイル（データと生成スクリプト）のみ。`fle_runtime.h`（`fle_ld`/`fle_st` の定義）、`fle_flags.h`、`fle_sse*.h`、`fle_vex.h`、`fle_mxcsr*`、`fle_string.c`、`fle_x87*`、`fle_env.c`、`fle_imports.c`、`fle_proc.c`、`fle_reg.c` は**同一**（`cmp`）。`lift/crates` は fe-decode（`jumptab.rs`、`lib.rs`、テスト）と fe-lift（`imports_table.rs` +12 行、`jumptab.rs`、`lib.rs`、`main.rs`、テスト）が変わっている。オラクルのハーネス（`harness/`）は**全体が同一**（`diff -rq` で差 0）。

**読み**: 変わった runtime 7 ファイルはロケール・モジュール表・システム情報の応答で、純計算関数の生成 C（`fle_ld`/`fle_st`/`fle_op_*`/SSE）が呼ぶ範囲ではない。生成 C の本体が同一で、純計算が使う主要ファイルが同一なので、純計算の挙動が変わる経路は見当たらない。ただし、変更 7 ファイルの差分を 1 行ずつ読んではいない（**未確認**）。この結果（seed 1・2 の全 ok ケースが前回と全項目一致）が、読みの裏づけになる。

## 6. seed 3 の結果

### 6.1 選び方（実測）

`run_diff2.py` の選択は `Random(seed).sample(pool, 200)` ＋ `Random(seed+1).shuffle(rest)`（482〜487 行）で、重ならないようにする引数はない。そこで 2 通り走らせた。

| 実行 | 選び方 | seed 1 との重なり | seed 2 との重なり | seed 1 または 2 との重なり | 新規 |
|---|---|---|---|---|---|
| **seed 3（重ならない）** | `--pure` を、純計算 9,901 件から seed 1・2 の選択（和集合 1,908）を除いた **7,993 件**の JSON に替える（`mk_seed3_pool.py`。新規ファイルとして書く）。選択のアルゴリズムは同じ | **0** | **0** | **0** | 1,000 |
| **seed 3（素直）** | `--pure results/pure.json`、`--seed 3` | 89 | 124 | **204**（3 つとも重なるのが 9） | 796 |

理由: seed 3 の素直な実行は、独立な 3 つ目のサンプルとして使えるが、seed 1・2 の 1,908 関数と 204 件重なる（期待値は 1,000 × 1,908 / 9,901 ≈ 193。統計的な検定はしていない）。重なりを除くと、**新しい関数の範囲を測れる**。ただし、`pure.json` を絞ると、プールが違う（7,993 件）ので、`Random(seed).sample` の結果は素直な seed 3 とは別になる（重なりは 90 関数、約 9%）。**重ならない実行を主、素直な実行を補**とした。

注意（実測）: `mk_seed3_pool.py` は seed 1・2 の選択を `a5-functions.json` から読み、`pure.json` から同じアルゴリズムで再現した選択と一致する（seed 1、2 とも一致）ことを確認してから絞った。また絞った `pure.json` は、**定数返却関数のネガコン 3 件の選び方にも影響する**（`run_diff2.py` の 611 行以降は `funcs` の先頭から条件に合う 3 件を選ぶ）。seed 1、seed 2、素直な seed 3 の 3 件は 0x410030、0x4536d0、0x453900。seed 3（重ならない）では、0x410030（seed 1 の選択に入っている）と 0x453900（seed 2 の選択に入っている）がプールから外れたので、0x4536d0、0x457310、0x457330 になった（0x4536d0 は他の seed と共通、0x457310 と 0x457330 は新規）。

### 6.2 seed 3（重ならない）: カテゴリ（実測、16,000 ケース）

| 判定 | 件数 |
|---|---|
| bit_exact | **13,482** |
| mismatch_core / aux | **0 / 0** |
| lift_unsupported / lift_stop_other | **0 / 0** |
| native_fault | 2,518（15.7%） |
| 関数: bit_exact_all_comparable / native_fault_all | 952 / 48 |
| ptr_input（671 関数）/ no_ptr（329 関数） | bit_exact 8,418 + native_fault 2,318 / bit_exact 5,064 + native_fault 200 |
| v1 と同じ流儀の 200 関数（ベース） | bit_exact 2,694、native_fault 506 |

mismatch は 0 件（`a5-mismatches.json` は `[]`）。列挙する関数 VA・入力・差分はない。

native_fault の原因: null_deref 1,243、wild_pointer 649、gp_noncanonical_address 375、scratch_range_exceeded 234、sigfpe(divide) 9、gp_other 4、stack_range_exceeded 2、write_to_readonly_image 1、timeout_2s(loop) 1。`native_fault_same_fault_addr` 2,507 / 2,518。lift 側の挙動は、fault 2,507、abort 9（sigfpe）、timeout 2（native の timeout_2s 1 件と、native が null_deref の `43c450.10` 1 件、下記）。

- **native は null_deref（`0xc`）で fault、lift は 2 秒の上限で timeout になった 1 件**: `43c450.10`（native の fault は `0x43d9be`、`fault_addr 0xc`）。アラームを 1,200 秒にして単独で走らせると、**lift も同じ `fault_addr 0xc` で fault**（31.7 秒かかった）。native は 0.22 秒。これも lift が遅いだけで、fault の位置は同じ（実測）。カテゴリは native_fault なので判定には影響しない。

補助の指標: 全 6 つの EFLAGS 定義済みフラグの差 0（CF 13,466、PF/ZF/SF 13,420、AF 10,696、OF 13,388 を比較。AF 未定義 2,786）、x87 の差 0、`mxcsr_lift_ne_reset` / `mxcsr_native_ne_reset` 664 / 664、MXCSR 状態ビットだけの mismatch_aux 0、実行された命令の割合 平均 90.7%・中央値 100%・全命令実行 816・ゼロ 48、`bit_exact_cases_with_image_or_scratch_writes` 5,021、timeout 再実行 2 件（native: fault 1 + timeout 1）。native が全 case fault の関数 48 件は、seed 1・2 の native_fault_all（37・36 件）と重ならない（seed 1・2 の選択を除いたので当然）。

### 6.3 seed 3（素直）: カテゴリ（実測、16,000 ケース）

| 判定 | 件数 |
|---|---|
| bit_exact | 13,594 |
| **mismatch_core** | **4**（2 関数。全部 `lift_timeout`） |
| mismatch_aux / lift_unsupported / lift_stop_other | 0 / 0 / 0 |
| native_fault | 2,402（15.0%） |
| 関数 | bit_exact_all_comparable 958、native_fault_all 40、has_mismatch_core 2 |

補助の指標（EFLAGS 差 0、x87 差 0、MXCSR 728 / 728 等）は seed 3（重ならない）と同様。ネガコンは §6.6。

### 6.4 mismatch 4 件の列挙と切り分け（実測）

`a5-mismatches.json` の 4 件（いずれも `fields: ["lift_timeout"]`、`lift_msg` は空、`lift_fault_addr` は 0x0）:

| 関数 VA | case | 入力（ABI の引数レジスタ、スタック引数） | native（2 秒） | lift（2 秒、10 秒再実行） |
|---|---|---|---|---|
| 0x6a72b0 | 6a72b0.0 | rcx=0x20020870、rdx=0x20059ba0、r8=0、r9=0、スタック引数 0,0,0,0。fill 2、seed 0x78f549f5fe7e | timeout（再実行 10 秒で ok） | timeout / timeout |
| 0x6a72b0 | 6a72b0.7 | rcx=0x2003c020、rdx=0x20050cf0、r8=0、r9=0x10ad4a873a6a9246（r32 = 980,062,790）、スタック 0x2373abf7,0x1e,0x22,0x29。fill 6、seed 0xe0f91b536b29 | ok | timeout / timeout |
| 0x6a72b0 | 6a72b0.12 | rcx=rdx=0x2005f720、r8=0、r9=0x6206a53e（r32 = 1,644,602,686）、スタック 0xe,0x10,0x25,0x22。fill 6、seed 0xc348caf31c14 | ok | timeout / timeout |
| 0x6c9100 | 6c9100.9 | rcx=0x20045810、rdx=0x20043de0、r8=0x31、r9=0xd70baa5e231fcb11（r32 = 589,286,161）、スタック 0x2e,0x28f74bee,0x23,0x30。fill 0、seed 0x84b954123656。poke 2 件 | ok | timeout / timeout |

`6a72b0.0` では native も 2 秒の通常実行で timeout になり、10 秒の再実行で ok（native 単独の実行は 1.9 秒で、16 並列の負荷で 2 秒を超えた）。

**再現手順と結果**（`lift/out-a5-head/repro_case.py`、`decode_case.py`。cwd は `lift/out-a5-head/src/lift/out-oracle`。`ORACLE_TIMEOUT` は `oracle2_common.c` 272 行のアラーム秒数の環境変数）:

```
# 4 件の case 行は seed3plain の work/cases-nat-*.txt から取り出して repro/case-<id>.txt に置いた
ORACLE_TIMEOUT=<秒> ./bin/native_drv2 repro/case-<id>.txt OUT.jsonl $FLENGINE_PE
ORACLE_TIMEOUT=<秒> ./bin/lift_drv2   repro/case-<id>.txt OUT.jsonl $FLENGINE_PE
python3 ../../../repro_case.py results/seed3plain/repro results/seed3plain/repro/case-<id>.txt 1200   # 上の 2 つを走らせ、run_diff2.compare で比較
```

| case | native（アラーム 1,200 秒） | lift 2 / 10 / 60 秒 | lift 1,200 秒 | `run_diff2.compare` |
|---|---|---|---|---|
| 6a72b0.0 | ok、1.90 秒 | timeout / timeout / timeout | **ok、486.5 秒** | core=[] aux=[] → **bit_exact** |
| 6a72b0.7 | ok、0.45 秒 | timeout ×3 | **ok、113.9 秒** | **bit_exact** |
| 6a72b0.12 | ok、0.73 秒 | timeout ×3 | **ok、189.7 秒** | **bit_exact** |
| 6c9100.9 | ok、0.47 秒 | timeout ×3 | **ok、154.4 秒** | **bit_exact** |

つまり 4 件とも、**2 秒、10 秒、60 秒では lift が timeout、1,200 秒のアラームでは lift が完走し、native と GPR・XMM・メモリ（image、scratch、stack）・EFLAGS 定義済みフラグ・MXCSR・x87 まで一致した**（`compare` の core / aux が空）。native は 0.45〜1.9 秒で完走する。

**ループの規模**（読み。命令列 `gen/new-part-03.c` 133508 行〜と入力のレジスタ値から）: 0x6a72b0 は `edx`（= r9d）回の外側ループ（`dec edx; jne`）で、各回に `ecx`（= r8d）を 2 で割って 8 を掛けた長さのバイト列を `movq` で 8 バイトずつ処理する内側ループがある。3 件（.0、.7、.12）とも **r8d = 0** なので内側ループは飛ばされ、実質は外側の回数だけ回る。r9d は .7 が 980,062,790、.12 が 1,644,602,686、.0 が 0（`dec edx` で 0xFFFFFFFF に回り込むので 2^32 = 4,294,967,296 回）。lift の所要時間を回数で割ると、.7 が 116 ns/回、.12 が 115 ns/回、.0 が 113 ns/回で、3 件がほぼ同じ（native は同じ回数を 0.45〜1.9 秒、約 0.4〜0.5 ns/回）。**読み**: 4 件のうち 3 件は「外側ループが 10 億〜43 億回」で、lift は 1 回あたりの命令が C の関数呼び出しを経る（生成 C は `-O0 -finstrument-functions`）ので native の 250 倍ほど遅い。0x6c9100（case 9）は `[rcx+0x84]`・`[rcx+0x80]` から読んだ値を上限にした二重ループ（同 162656 行〜）で、この 2 つの値の中身と回数は**確認していない**（`6c9100.9` の native 終了時のレジスタ値 r8d=770,995,107 が外側のカウンタが大きいことと矛盾しないことだけ見た）。

**回数を変えた実験**（実測）: `6a72b0.7` の r9 だけを 1,000,000（0xf4240）と 5,000,000（0x4c4b40）に書き換えて再実行（`repro_case.py` に `r9=…` を渡した。別 ID `6a72b0.7.m`、アラーム 120 秒）。native 0.02 秒 / lift 0.13 秒（r9=1,000,000）、native 0.02 秒 / lift 0.57 秒（r9=5,000,000）で、どちらも **bit_exact**。lift の時間は r9 におおむね比例して伸びる（5 倍の回数で 4.4 倍。起動時間を含む）。

**原因の切り分け（読み）**: lift の欠陥（値の誤り）ではなく、**オラクルの限界（`OR_TIMEOUT_S = 2`、再実行でも 10 秒）**。lift は native より、今回の 4 件で **253〜329 倍**遅い（native 0.45 秒に対し lift 113.9 秒 = 253 倍、native 1.90 秒に対し 486.5 秒 = 256 倍、0.73 秒に対し 189.7 秒 = 260 倍、0.47 秒に対し 154.4 秒 = 329 倍）。そのため、native が 2 秒以内に戻る入力のうち、ループが長いものは lift が 2 秒にも 10 秒にも届かない。

**未確認**: 1,200 秒のアラームで 4 件が完走することは確かめたが、**素直な seed 3 の他の 2 件の timeout 判定**（`5f03d0.11`、`5f03d0.13`。native も timeout の `timeout_2s(loop)` で native_fault に分類）は、アラームを延ばして完走するかを見ていない（分類は native_fault で比較不能として扱われる）。`ad7640` 等の seed 1・2 の native timeout 17・11 件も同様に延長は試していない。

**過去の seed での同種の現象**: seed 1・2 でも lift の first-pass の timeout は 18・14 件あったが、10 秒の再実行で ok に戻るか、native も timeout の native_fault（17・11 件）に分類されて、mismatch にはならなかった。素直な seed 3 では、**native が 2 秒以内に ok で戻り、lift が 10 秒の再実行でも戻らない**ケースが初めて 4 件出た（seed 1、2、重ならない seed 3 では 0 件）。

**修正はしていない**。今回の 4 件を mismatch から外す手段（上限の延長、lift の最適化ビルド）は決めない。

### 6.5 seed 3（素直）で lift が遅いほか、native が fault で lift が timeout になったもの（実測）

`6a72b0.11`: native は `0x6a7305` の `movq` で SIGSEGV（`fault_addr 0x20100000`、scratch_range_exceeded）、lift は 2 秒で timeout。アラームを 1,200 秒にして単独で走らせると、**lift も `fault_addr 0x20100000` で fault**（17.5 秒）。カテゴリは native_fault（比較不能）なので、判定には影響しない。同じ種類の現象が seed 3（重ならない）の `43c450.10`（§6.2）にもある。

### 6.6 seed 3 のネガティブコントロール（実測）

| 結果 | seed 3（重ならない） | seed 3（素直） | （参考）seed 1 / seed 2 |
|---|---|---|---|
| detected_core | 118 | 105 | 113 / 116 |
| detected_aux_only | 5 | 10 | 10 / 7 |
| detected_by_native_fault_only | 54 | 71 | 64 / 64 |
| undetected_state_changed_but_masked | 11 | 6 | 4 / 10 |
| undetected_equivalent_no_state_change | 15 | 11 | 12 / 6 |
| 出力の差で検出 | 123/203 = 60.6% | 115/203 = 56.7% | 123/203 / 123/203 |
| native fault 込み | 177/203 = 87.2% | 186/203 = 91.6% | 187/203 / 187/203 |
| 未検出 | 26 | 17 | 16 / 16 |

定数返却関数の imm 3 件はどちらもすべて detected_core（健全性の確認は通った）。未検出の（関数, 命令）は、seed 1・2・3・素直 3 のどの 2 つの間でも重ならない（0 件）。検出率は seed によって 56.7〜60.6%（出力の差）、87.2〜92.1%（fault 込み）と動く。seed 3（重ならない）の未検出 26 件は、`mov rsp, rbp`、`test rax, rax`、`je`/`jne` の imm、`lea rax,[rax-1]` 等で、これまでの seed と同じ種類（個別の追跡はしていない）。**読み**: 出力の差の検出率は、選ばれた関数の性質（等価な変異が多い関数が多く入るか）で動くと見える。確認はしていない。

## 7. 全 seed のまとめ（実測）

| | seed 1 | seed 2 | seed 3（重ならない） | seed 3（素直） |
|---|---|---|---|---|
| 選ばれた関数 | 1,000 | 1,000 | 1,000 | 1,000 |
| bit_exact | 13,535 | 13,616 | 13,482 | 13,594 |
| mismatch（core / aux） | 0 / 0 | 0 / 0 | 0 / 0 | **4 / 0**（lift_timeout、§6.4） |
| lift_unsupported | 0 | 0 | 0 | 0 |
| native_fault | 2,465（15.4%） | 2,384（14.9%） | 2,518（15.7%） | 2,402（15.0%） |
| 出力の差で検出（ネガコン） | 123/203 | 123/203 | 123/203 | 115/203 |
| native fault 込み | 187/203 | 187/203 | 177/203 | 186/203 |
| 未検出 | 16 | 16 | 26 | 17 |

seed 1・2・3（重ならない）の合計（実測）: 48,000 ケース、bit_exact 40,633、native_fault 7,367（15.3%）、mismatch 0。関数は重なりなしで 2,908 件（純計算 9,901 件の 29.4%、全 59,297 関数の 4.9%）。そのうち bit_exact の case が 1 件以上ある関数は 2,790、どの実行でも bit_exact の case が 0 の関数（全 case が native_fault）は 118。seed 1・2 だけの和集合は 1,908 件（19.3%、3.2%）、素直な seed 3 まで入れた 4 つの和集合は 3,614 件（36.5%、6.1%、bit_exact の case が 1 件以上ある関数 3,469）。

## 8. 前回との差のまとめ

| 項目 | 前回（b21d051）との差 |
|---|---|
| 生成 C の SHA | 43985d62… → feb5b908…（応答表 8 行の追加。関数本体の変化 0 件） |
| seed 1 のカテゴリ件数・ネガコン | すべて一致 |
| seed 1 の結果ファイル | `a5-native-faults.json` の 10 件の `rip` を除き全バイト一致 |
| seed 2 のカテゴリ件数・ネガコン | すべて一致 |
| seed 2 の結果ファイル | `a5-native-faults.json` の 10 件の `rip`、`a5-summary.json` の trace 2 項目（±1）を除き全バイト一致 |
| 純計算関数の生成 C の本体 | 変化 0 件 |
| 新しい観測 | seed 3（重ならない）は mismatch 0、素直な seed 3 は mismatch_core 4（時間の上限による。§6.4） |

## 9. 限界

- **対象は純計算 9,901 件**（候補 57,816 の 17.1%、全 59,297 関数の 16.7%）**に限る**。選別から外れた関数は、IAT 経由の import 呼び出し、間接 call/jmp、x87、FS/GS、cpuid など、またはそれらを呼び先に持つ関数で、**オラクルの対象外**（0035）。`fle_imp_*` の応答表と同じ応答を native 側に用意していないため、import を呼ぶ関数は native で動かせない。除外の内訳は前回と同じ（`exclude-stats.json` が全バイト一致）。x87 の演算と TEB アクセスは検査していない。
- **サンプルは小さい**: 重ならない 3 seed で 2,908 関数（全体の 4.9%、純計算の 29.4%）。全体の等価性は主張できない。`bit_exact` は「その入力で一致した」であって、関数全体の等価性ではない。1 関数 16 ケース、入力は乱数と境界値で、通す経路は一部。全命令を実行した関数は seed 1 で 808、seed 2 で 821、seed 3（重ならない）で 816（各 1,000 関数中）。
- **native_fault は約 15%**（seed 1 15.4%、seed 2 14.9%、seed 3 15.7%）。native が戻らないケースは比較不能で、何も主張しない。関数単位でも各 seed で 36〜48 関数は全 case が native_fault（この関数については一致も不一致も言えない）。原因の大半は null_deref と wild_pointer（`argptr.py` の静的な近似。0036 §7）。
- **時間の上限**: アラームは 2 秒（再実行で 10 秒）。lift は native より桁違いに遅く（今回の 4 件で 253〜329 倍）、ループが長い入力は lift が上限に届かず mismatch_core（`lift_timeout`）と分類される。素直な seed 3 の 4 件は、延長すると bit_exact になった。上限を超える入力が他にあれば、同様に見かけの mismatch になる。逆に、native が 2 秒で timeout になるループは native_fault として比較から外れる（延長して比べてはいない）。
- **今回の再実行は、HEAD で追加された変更の検査になっていない**。FLEngine の生成 C の関数本体が、決定 0075（SSE/SSE2 26 ニーモニック）、0076（`gs:[0x10]`）、0077（`call [IAT]`）、0079（CPU 振り分けスタブ）、0087（ロケール）、0089〜0094（MSVC switch の表の解決）で 1 つも変わっていない（前回の b21d051 の時点でも 0075〜0079 は変わっていなかった）。変わったのは応答表の 8 行（0091、0092）だけで、純計算関数はそれらを呼ばない。これらを A5 で検査するには、対象の関数が FLEngine の外（dsp_ippv2 や Blend2D 等）にあり、native 側の用意が要る（そのための基盤がこのオラクルにあるかは**未確認**）。
- 比較しないもの: 未定義の EFLAGS（seed 1 で AF 2,842 case、seed 3 で AF 2,786）、GS/TEB、MM レジスタの値、x87 の 80 ビットレジスタの内容、MXCSR の RC/FTZ/DAZ が非既定の状態（native は 0x1F80 で入る。0038 §7）。
- 確認は AMD Ryzen 9 9950X の 1 台のみ。**Intel 実機との比較はしていない**（未確認）。MXCSR の状態ビットはホストの命令が立てたものを写しているので、CPU が違うと結果が違う可能性がある（0038 §7）。
- seed は 1、2、3 の 3 つ（3 は 2 通り）。run5 の個別ケースとの case 単位の突き合わせは、前回の再実行（b21d051）の個別ファイルとの突き合わせまでで、run5 そのものとはしていない。
- ネガコンの検出は、反転が出力に出る場合に限る。未検出は seed によって 16〜26 件で、出力が変わらない等価な変異を含む。検出の下限を測るもので、全件検出は期待していない。
- 生ログの `fgpr`、abort の `fault_addr` 等の差の由来（ホストのスタックと PID と読んだ）は、全件は照合していない。

## 10. 置き場と注意

- 生成物はすべて `lift/out-a5-head/`（`.gitignore` の `lift/out*/` に該当。`git check-ignore -v` で確認）。約 1.9 GB（`du -sh`。2 回目の生成 C の確認用コピー `tmp/gen-second.c` 215 MB を含む）で、500 MB を超えるファイルはない（最大は `lift_drv2` 258 MB と生成 C 215 MB）。
  - 生成 C とビルド: `src/lift/out-oracle/gen/`（`a5head-pdata.c`、`new-part-00〜16.c`）、`parts/`、`rt/`、`obj/`、`bin/`、`lift/out-a5-head/a5head-build.sh`、`build.txt`。
  - 結果: `src/lift/out-oracle/results/rerun/`（seed 1）、`seed2/`、`seed3/`（重ならない。`pure-excl12.json` を含む。`repro/` に 1 件）、`seed3plain/`（素直。`repro/` に mismatch 4 件の再現ファイル）。
  - 補助スクリプト（`lift/out-a5-head/` 直下、新規）: `cmp_work.py`（生ログの比較）、`mk_seed3_pool.py`（seed 3 のプール）、`decode_case.py`、`repro_case.py`、`analyze_head.py`、`analyze_union.py`、`gen-diff-m85-vs-head.txt`、`cmp-bodies-m85-vs-head.txt`、`fe-lift-stdout.txt`。いずれも読み取りか、`lift/out-a5-head/` 内への新規出力のみ。
- 前回の結果（`lift/out-a5-m85/`）は読み取りだけ。変更していない。
- 規則: `rm`、既存ファイルの全体上書き、git の書き込み系操作、`/tmp`、sleep、nohup は使っていない。TMPDIR は `lift/out-a5-head/tmp`（ドライバは `src/lift/out-oracle/tmp`）。新規ファイルは `docs/investigations/a5-oracle-rerun-3.md` と `lift/out-a5-head/` の下だけ。権限拒否はなかった。`run_diff2.py` など既存のハーネスは改変していない（`mk_seed3_pool.py` は絞った `pure.json` を新規ファイルとして書く）。
- **規則に対する自己申告（文言に触れる可能性があるもの）**:
  1. **「コマンド文字列に timeout を書かない」**: 6.4 の再現で、アラーム秒数を指定する環境変数 `ORACLE_TIMEOUT=…`（`oracle2_common.c` 272 行の実在の名前）をコマンド文字列に書いた。集計用の python と grep のパターンにも `timeout`（status の値）、`lift_timeout`、`timeout_2s` を書いた。`timeout` コマンドは使っていない（長い実行は shell ツールの timeout パラメータで待った）。
  2. **「末尾の & を書かない」**: 6.4・6.5 の再現で、`( python3 … > A 2>&1 & python3 … > B 2>&1 ; wait )` の形を 1 回使った。`&` は**末尾ではなく途中**で、`wait` で完了までフォアグラウンドで待った。`xargs -P 4` も 1 回使った。文言に触れるかの判断は仰ぎたい。
  3. **「python に `PYTHONDONTWRITEBYTECODE=1` を付ける」**: シェルの呼び出しごとに環境が引き継がれるかを確かめていないので、`export` を書かなかった呼び出しでは付いていなかったおそれがある。具体的には、`pure.json` の構造を見た `python3 -c`、seed 3（素直）の mismatch の中身を読んだ `python3 -c`、4 件の case の status を並べた `python3 -c`、再現用ディレクトリのファイルを確認した heredoc、環境確認の `python3 -c "import capstone,os; …"`。いずれも標準ライブラリと capstone を import しただけ。作業の最後に `find -newermt` で確認して、`lift/out-a5-head/`、リポジトリ内、uv の python 標準ライブラリのディレクトリ、`~/.local/lib/python3.14` のどこにも今日の `.pyc` は増えていない（0 件、下の最終確認）。
  4. 再現用の 1 行ループに、使っていない変数 `rm_unused=` を書いた（変数への空代入。`rm` コマンドは実行していない）。
  5. **既存ファイルの上書き（自分で作った新規ファイルだが、規則の文言には反する）**: §6.4 の「回数を変えた実験」で、`repro_case.py` の出力ファイル名は「ID + アラーム秒数」だけで決まる（`out-native-6a72b0.7.m-a120.jsonl`、`out-lift-6a72b0.7.m-a120.jsonl`）。r9=1,000,000 の 1 回目の出力ファイルを、r9=5,000,000 の 2 回目が同じ名前で**上書きした**（ドライバが OUT を書き込みで開く。タイムスタンプで確認）。残っているのは 2 回目（r9=5,000,000）の出力だけ。1 回目の結果（native 0.02 秒 / lift 0.13 秒、bit_exact）は、そのときの端末出力から本文に書いたもので、ファイルでは裏づけられない。case 行のファイル（`case-6a72b0.7-mod-1000000.txt`、`case-6a72b0.7-mod-5000000.txt`）は別名で両方残っている。上書きされたのは今回の作業で作ったファイルだけで、前回の結果や他の作業のファイルではない。
- ワークツリーの `git status --short`（最終確認時、14 行）のうち、私が作ったのは `?? docs/investigations/a5-oracle-rerun-3.md` の 1 行だけ（`lift/out-a5-head/` は `.gitignore` の `lift/out*/` で出ない）。残りは別の subagent などの作業で、私のものではない: `M docs/README.md`、`M lift/crates/fe-decode/src/{jumptab,lib}.rs`、`M lift/crates/fe-decode/tests/{jt_cpu.rs,msvc_common/mod.rs,msvc_switch.rs}`、`M lift/crates/fe-lift/tests/{jt_cpu.rs,msvc_switch.rs}`、`?? docs/decisions/0095-msvc-switch-swapped-form.md`、`?? docs/investigations/blend2d-switch-swapped.md`、`?? lift/harness/wasm/trace.c.new`、`?? lift/harness/wasm/trace_native.c.new`、`?? lift/iat-unsup-closure.txt`。作業の開始時は M が 5 ファイルだったが、終了時は 8 ファイルに増えている（`fe-decode/tests/jt_cpu.rs`、`fe-lift/tests/jt_cpu.rs`、`fe-lift/tests/msvc_switch.rs` が増えた）。私はこれらを触っていない。スナップショットは `git archive HEAD`（コミット済みの木）なので、この未コミット変更は今回の測定に入っていない。
