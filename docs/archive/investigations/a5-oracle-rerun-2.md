# A5 native オラクルの再実行 その 2（HEAD b21d051、FLEngine の生成 C SHA 43985d62…）

日付: 2026-10-24。対象は `git archive HEAD`（b21d051）のスナップショット。測定のみで、修正はしていない。設計は `docs/decisions/0035-a5-oracle.md`、`0036-a5-oracle-v2.md`、`0038-mxcsr-flags.md`。前回は `docs/investigations/a5-oracle-rerun.md`（コミット dc98e49、対象 HEAD d6c24be）。

凡例: **実測** = この作業でツールを走らせて得た出力。**読み** = 実測からの解釈で、直接は確かめていないもの。**未確認** = 確かめていない。

## 1. 結論

1. **実測**: seed 1（前回と同じ手順・同じパラメータ）の結果は、前回（HEAD d6c24be の再実行）と**カテゴリ件数がすべて一致した**。bit_exact 13,535、mismatch_core 0、mismatch_aux 0、lift_unsupported 0、lift_stop_other 0、native_fault 2,465。ネガコンも 203 件の出力が前回と同じ（検出 123/203 = 60.6%、native fault 込み 187/203 = 92.1%、未検出 16）。
2. **実測**: 一致は件数だけではない。`pure.json`（純計算 9,901 件）、`argptr.json`、`exclude-stats.json`、`a5-cases.json`（16,000 case の判定と detail）、`a5-cases.csv`、`a5-negctl.json`（203 件の個別結果）は、前回の再実行の同名ファイルと **`cmp` で全バイト一致**した。前回は個別ケースの突き合わせができていなかったので、ここは前回より強い確認になる。
3. **実測**: HEAD の生成 C は前回（d6c24be）から変わっている（215,700,245 B → 215,707,512 B）が、変わったのは**応答表の 36 行と、関数本体 9 件**（いずれも `jmp [IAT]` の thunk が `fle_unsupported` から `fle_imp_*` 呼び出しに変わっただけ）で、**純計算 9,901 件の関数本体は 0 件変わっていない**。選んだ 1,000 関数（seed 1）も 1,000 関数（seed 2）も、本体は前回と同一。
4. **実測**: seed 2（今回の追加）、1,000 関数 × 16 ケース: bit_exact 13,616、mismatch 0（core、aux とも）、lift_unsupported 0、native_fault 2,384（14.9%）。seed 1 の選択との重なりは **92 関数**。
5. 範囲: この結果が言えるのは「選んだ関数のうち native が戻ったケースで、HEAD の lift が GPR・XMM・メモリ・EFLAGS（定義済みフラグ）・MXCSR・x87 の制御状態まで native と一致した」ことだけ。純計算関数に限る。import・間接 call・x87・GS を持つ関数は対象外（§8）。

## 2. 対象と環境

| 項目 | 内容 | 区分 |
|---|---|---|
| HEAD | `b21d05119ae40486104abfff4cf87a44275a654d`（決定 0085 の直後に調査 1 件のコミットが載った状態） | 実測 |
| スナップショット | `git archive HEAD \| tar -x -C lift/out-a5-m85/src/`（8.2 MB。`binaries/` は含まれない）。ワークツリーの `lift/` は読んでいない | 実測 |
| PE | `/home/ubuntu/work/signature-studio-develop/binaries/win64/FLEngine_x64.dll`（22,227,016 B、SHA-256 `b2c64da40921cf262c15c5b25c2b0422e4d1851e953c87e0bc38ed2c19f9496d`）を `FLENGINE_PE` で絶対パス指定 | 実測 |
| FLEngine の生成 C | `fe-lift lift … --pdata` の出力 215,707,512 B、SHA-256 **`43985d62321a90f562ce1b8144253e00af13df795e4190d26b6b8ce2537137a1`**（依頼の期待値と**一致**）。functions 59,297、closure_instructions 2,528,030、bad_nodes 0、unsupported_total 525（`Jmp: indirect through IAT slot` 524、`Jmp: indirect target unresolved` 1） | 実測 |
| 前回の生成 C（d6c24be） | 215,700,245 B、SHA-256 `47e0b1d2e83ece97a0d8f8787459cc477b6e99d4de1e562615b7afaef78b7bfc`、unsupported_total 534（`lift/out-oracle-rerun/stage/lift/out-oracle/gen/` を読んで取った） | 実測 |
| 環境 | AMD Ryzen 9 9950X（16 CPU）、Linux 7.0.0-38-generic、clang 21.1.8、rustc 1.99.0、capstone 5.0.7、python は uv の cpython 3.14.8 | 実測 |
| 並行作業 | ctype 調査が並行して走っていたはずだが、負荷は測っていない | 未確認 |

## 3. 実行コマンド

前回（`a5-oracle-rerun.md` §2）と同じ手順。差は §3.1。`R=/home/ubuntu/work/signature-studio-develop/lift/out-a5-m85`、`H=../harness/oracle`、作業ディレクトリ（cwd）は `$R/src/lift/out-oracle`。全 python に `PYTHONDONTWRITEBYTECODE=1`、`TMPDIR=$R/tmp`。

```
# 0. スナップショット
mkdir -p $R/src $R/tmp $R/target
git archive HEAD | tar -x -C $R/src/

# 1. lifter のビルドと生成 C（SHA を最初に確認）
export CARGO_TARGET_DIR=$R/target TMPDIR=$R/tmp FLENGINE_PE=<上の絶対パス>
cd $R/src/lift && cargo build --release -p fe-lift                  # 2.7 秒
cd $R && ./target/release/fe-lift lift $FLENGINE_PE src/lift/out-oracle/gen/a5m85-pdata.c --pdata   # 2.4 秒
sha256sum src/lift/out-oracle/gen/a5m85-pdata.c                      # 43985d62…37137a1（一致）

# 2. 分割→clang→ドライバのリンク（$R/a5m85-build.sh。17.0 秒、出力は $R/build.txt。警告・エラーの行は 0）
sh $R/a5m85-build.sh
#   split_c.py a5m85-pdata.c gen/new-part 3700                     (59,297 関数、17 分割)
#   clang -std=gnu11 -O0 -g -w -fno-builtin -ffp-contract=off -fPIE -finstrument-functions   (生成 C。build5.sh と同じ)
#   runtime/*.c を -O0 -g -Wall -Wextra -fPIE、oracle2_common/native_drv2/lift_drv2 を -O1、image.c を -O1
#   native_drv2（image.o + ms_call2.S）と lift_drv2（生成 C 17 本 + runtime 全部）をリンク

# 3. 選別・ポインタ判定・除外集計（$R/src/lift/out-oracle で。それぞれ 2.1 / 0.6 / 2.0 秒）
python3 $H/select_pure.py $FLENGINE_PE gen/new-part-00.c results/pure.json
python3 $H/argptr.py $FLENGINE_PE results/pure.json results/rerun/argptr.json
python3 $H/exclude_stats.py $FLENGINE_PE gen/new-part-00.c results/rerun/exclude-stats.json > results/rerun/exclude-stats.txt

# 4. seed 1（前回と同じ引数）。4 分 28.5 秒（real）
python3 $H/run_diff2.py --pe $FLENGINE_PE --pure results/pure.json --argptr results/rerun/argptr.json \
    --exclude results/rerun/exclude-stats.json --out results/rerun --count 1000 --n 16 --seed 1 --negctl 200 --jobs 16

# 5. seed 2（追加。再ビルドなし、argptr/exclude は seed 1 のコピー）。2 分 54.9 秒（real）
mkdir -p results/seed2 && cp results/rerun/argptr.json results/rerun/exclude-stats.json results/seed2/
python3 $H/run_diff2.py --pe $FLENGINE_PE --pure results/pure.json --argptr results/seed2/argptr.json \
    --exclude results/seed2/exclude-stats.json --out results/seed2 --count 1000 --n 16 --seed 2 --negctl 200 --jobs 16
```

ドライバの終了コードは両 seed とも native 0、lift 0、native-trace 0。emcc/wasm は使っていない。

### 3.1 前回との手順の差（実測）

- 前回は `rerun-build.sh` の中で `fe-lift lift` まで行った。今回は SHA を先に確認するため `fe-lift lift` を単独で先に実行し、`a5m85-build.sh`（`rerun-build.sh` から `fe-lift` の行を除き、パスを `lift/out-a5-m85` に替えただけ）は `split_c.py` から始めた。clang のフラグ、分割数（3,700）、リンクの順序は前回と同じ。
- 生成 C のファイル名は `a5m85-pdata.c`（前回は `rerun-pdata.c`）。
- 並列数 16、`--count 1000 --n 16 --seed 1 --negctl 200` は前回と同じ。
- `run_diff2.py`、`select_pure.py`、`argptr.py`、`exclude_stats.py`、`oracle2_common.c`、`native_drv2.c`、`lift_drv2.c`、`ms_call2.S`、`image.c`、`split_c.py` は、前回のスナップショットと HEAD のスナップショットで**差がない**（`diff -rq` で `__pycache__`（前回側のみ）以外に差分なし）。

## 4. 生成 C と runtime の前回との差（実測）

### 4.1 生成 C（d6c24be → b21d051）

関数本体（`void f_XXXX(void)` から次の同形の行まで）の SHA-256 を、`lift/out-a5-m85/cmp_bodies.py` で全 59,297 関数について比べた。

| 項目 | 件数 |
|---|---|
| 関数の集合 | 前回・今回とも 59,297、片側だけの関数 0 |
| 本体が同一 | **59,288** |
| 本体が変わった | **9** |
| 純計算 9,901 件のうち本体が変わったもの | **0** |
| seed 1 の選択 1,000 / seed 2 の選択 1,000 のうち本体が同一 | 1,000 / 1,000 と 1,000 / 1,000 |

変わった 9 件はすべて 5 行の thunk で、`fle_unsupported(UINT64_C(0x…))` が `fle_imp_<API>()` の呼び出しに変わっただけ:

| 関数 VA | API | 区分 |
|---|---|---|
| 0x406ea0 | GetFileType | kernel32 |
| 0x406ff0 | RtlUnwindEx | kernel32 |
| 0x407000 | UnhandledExceptionFilter | kernel32 |
| 0x4070c0 | GetConsoleOutputCP | kernel32 |
| 0x4235c0 | GetOEMCP | kernel32 |
| 0x423aa0 | IsDebuggerPresent | kernel32 |
| 0x424920 | DefWindowProcW | user32 |
| 0x425780 | CreateWindowExW | user32 |
| 0x425980 | SetWindowLongPtrW | user32 |

プロローグ（表の部分）は 119,286 → 119,394 行（+108）。差分は 5 か所で、すべて追加: `fle_imp_*` の宣言 36、`fle_dispatch_table` の行 36（擬似 VA 0x7ff900000e10 以降の末尾）、`fle_resp_table` の行 36、`fle_dispatch_count` 59,521 → 59,557、`fle_resp_count` 224 → 260。既存の行の変更はない。unsupported_total は 534 → 525（−9。上の 9 件と一致）。

**読み**: user32 の 3 件は `imports_table.rs` の「Virtual windows (decision 0071)」の行、kernel32 の 6 件は「decision 0078」の行に対応する（`git diff d6c24be HEAD -- lift/crates/fe-lift/src/imports_table.rs` を読んだ。表は 36 行増え、上の +36 と一致）。**未確認**: 中間コミット（0071 だけ、0074〜0077 の時点の SHA `67827424…`）の生成 C は作り直していないので、SHA の変遷（47e0b1d2… → 67827424… → 43985d62…）の各段階の差が上の 3 行と 33 行の分かどうかは、決定文書の記述に依拠している。

決定 0075（sse_legacy.rs）、0076（gs:[0x10]）、0077（call [IAT]）、0079（CPU 振り分けスタブ）は、`d6c24be..HEAD` で `fe-lift/src` と `fe-decode/src` に差分を持つが、**FLEngine の生成 C の関数本体を 1 つも変えていない**（上の 9 件は `jmp [IAT]` の thunk で、0077 の `call [IAT]` の形ではない）。コミットメッセージと決定文書の「FLEngine 不変」と整合する。

### 4.2 runtime（スナップショット同士）

`lift/runtime/` は d6c24be のスナップショットと比べて、22 ファイルが変更、`fle_realmod.{c,h}`、`fle_win.{c,h}`、`test/` が追加。変更されたもの: `fle_class.{c,h}`、`fle_env.{c,h}`、`fle_imports.{c,h}`、`fle_load.c`、`fle_locale.{c,h}`、`fle_modules.{c,h}`、`fle_proc.{c,h}`、`fle_reg.c`、`fle_sse_int2.h`、`fle_sys.{c,h}`、`fle_sysinfo.{c,h}`、`fle_vex.h`、`fle_vol.{c,h}`。

- `fle_sse_int2.h` は +643 行（追加のみ、0075 の `fle_lg_*`）。`fle_vex.h` は +42/−3 行（`GS:[0x10]` と `fle_teb_set_stack`、0076）。
- 純計算関数の生成 C が使う SSE・フラグ・MXCSR・x87 の主要ファイルは**同一**: `fle_flags.h`、`fle_sse.h`、`fle_sse_more.h`、`fle_mxcsr_flags.h`、`fle_mxcsr.c`、`fle_string.c`、`fle_x87ctl.c`、`fle_x87.h`（`cmp` で確認）。
- 生成 C（純計算の 9,901 件の本体）が同一で、これらが同一なので、**読み**として純計算関数の挙動が変わる経路は見当たらない。ただし、runtime の変更ファイルすべての差分を 1 行ずつ読んだわけではない（`fle_sse_int2.h` と `fle_vex.h` のみ差分を読んだ）。

## 5. seed 1 の結果（前回との比較）

### 5.1 カテゴリ（16,000 ケース）

| 判定 | run5（前回記録） | 前回の再実行（d6c24be） | 今回（b21d051） | 差 |
|---|---|---|---|---|
| bit_exact | 13,535 | 13,535 | **13,535** | 0 |
| mismatch_core | 0 | 0 | **0** | 0 |
| mismatch_aux | 0 | 0 | **0** | 0 |
| lift_unsupported | 0 | 0 | **0** | 0 |
| lift_stop_other | 0 | 0 | **0** | 0 |
| native_fault | 2,465 | 2,465 | **2,465**（15.4%） | 0 |

関数単位（1,000）: bit_exact_all_comparable 963、native_fault_all 37。has_mismatch_* 0、lift_not_executable 0。ポインタ入力別: ptr_input（651 関数）10,416 case = bit_exact 8,065 + native_fault 2,351（22.6%）、no_ptr（349 関数）5,584 case = 5,470 + 114（2.0%）。v1 と同じ 200 関数: bit_exact 2,741、native_fault 459。いずれも前回と同じ。

native_fault の原因（今回 = 前回）: null_deref 1,223、wild_pointer 650、gp_noncanonical_address 289、scratch_range_exceeded 243、sigfpe(divide) 33、timeout_2s(loop) 17、stack_range_exceeded 7、gp_other 2、write_to_readonly_image 1。lift 側の挙動の内訳（`native_fault_vs_lift`）と `native_fault_same_fault_addr` 2,415 も同じ。

### 5.2 補助の指標

| 指標 | 前回 | 今回 |
|---|---|---|
| 純計算関数（`select_pure.py`）/ 候補 | 9,901 / 57,816 | 9,901 / 57,816 |
| 除外の内訳（`exclude-stats.json`） | own:iat_call 3,407、own:indirect_call 15,097、own:indirect_jmp 2、own:x87 74、own:special 4、callee_only 25,928 / 3,325 / 67 / 2 / 9 | 同じ（ファイルが全バイト一致） |
| `mxcsr_lift_ne_reset` / `mxcsr_native_ne_reset` | 729 / 729 | 729 / 729 |
| EFLAGS 定義済みフラグの差 | 0（CF 13,528、PF/ZF/SF 13,470、AF 10,693、OF 13,475 を比較。AF 未定義 2,842） | 同じ |
| x87（cw/sw/tw）の差 | 0 | 0 |
| 実行された命令の割合（平均 / 中央値 / 全命令実行 / ゼロ） | 91.4% / 100% / 808 / 37 | 91.4%（0.91397…）/ 100% / 808 / 37 |
| `bit_exact_cases_with_image_or_scratch_writes` | 4,820 | 4,820 |

### 5.3 ネガティブコントロール（203 件）

結果は前回と同じ（`a5-negctl.json` の 203 件の個別レコードが全バイト一致）: detected_core 113、detected_aux_only 10、detected_by_native_fault_only 64、undetected_state_changed_but_masked 4、undetected_equivalent_no_state_change 12。出力の差で検出 123/203 = 60.6%、native fault 込み 187/203 = 92.1%、core のみ 113/203 = 55.7%。定数返却関数の imm 3 件（0x410030、0x4536d0、0x453900）はすべて detected_core。

未検出 16 件の関数・命令は前回の一覧（`a5-oracle-rerun.md` §4）と同じ（0x6351b0/0x6351c8、0xf7b3d0/0xf7b3d4、0x6bebd0/0x6bebd9、0x409c80/0x409cc2、0xc40c00/0xc40c04、0x10e47c0/0x10e47ea、0x41c770/0x41c774、0x12dea10/0x12dea39、0x442770/0x442835、0x1312f70/0x1312f78、0x728ef0/0x728ef0、0x4146c0/0x41467a、0x478a30/0x439799、0x11db3b0/0x11db3cb、0x10bb130/0x10bb141、0x52a700/0x52a705）。前回の「run5 と未検出の関数・命令が完全に同じかは未確認」は、**この再実行と前回の再実行が同一であることまでは確認できた**が、run5 そのものの個別一覧との比較は今回もしていない（**未確認**）。

## 6. 数が違った点の切り分け

### 6.1 カテゴリ件数の差

**なし**（§5.1）。(a) 関数の選択: 純計算 9,901 件の集合・順序・各関数の `end`/`insns`/`calls` は前回と同一（`pure.json` が `cmp` 一致）、選ばれた 1,000 関数も同一。(b) ケースの出力: 16,000 ケースの判定と detail（`wrote_blocks`、`wrote_stack_blocks`）が `a5-cases.json` の全バイト一致。**関数 VA を列挙すべき差はない**（mismatch 0 件）。

### 6.2 生ログ（`work/out-*.jsonl`）の差（実測）

カテゴリには影響しないが、生ログには差がある。前回の `work/` と今回の `work/` を case 単位で比べた。

| 比べたもの | 結果 |
|---|---|
| native 通常実行 16,000 件 | ok 13,535 件は、fault 系の項目（`fgpr`、`fault_rip`、`fault_addr`）以外の**全項目が一致**。fault 2,448 件も status/sig/code/fault_rip/fault_addr が一致。timeout 17 件は status が同じで、14 件の記録で `fgpr`（13 件）か `fault_rip`（6 件）が違う |
| lift 通常実行 | ok 13,533 件は fault 系項目以外が全項目一致。fault 2,415 件、abort 33 件、timeout 18 件の status は同じ。差は後述の 1 件（ad7640.14）と `fault_addr`/`fgpr` |
| native 単一ステップ（trace）16,000 件 | ok 13,528 件は一致。**13 件が timeout → fault に変わった**（下） |

個別の差（実測）:

1. **lift の first-pass timeout が 1 件減った**: `0xad7640` の case 14（`ad7640.14`）。前回は lift の最初の実行（16 並列、2 秒）で timeout になり、10 秒の再実行で ok。今回は最初の実行で ok。**ok の内容（再実行後の前回と今回の最初の実行）は全項目一致**（native も同様）。このため `timeout_retry.retried` が 19 → 18、`lift_before.timeout` 19 → 18。最終の判定は同じ。
2. **単一ステップの trace が timeout → fault になった 13 件**（全部 trace 側で、再実行はない）: 0x54d300（case 4）、0x587740（5、10）、0x6a56d0（12、15）、0xc64110（14）、0xca9280（6、7、10、14）、0xd09bb0（11）、0xe34080（8、13）。このため `trace_consistency` が both_fault 2,412 → 2,425、fault→timeout 36 → 23。通常実行の判定には使われない（trace は、ネガコンの対象選びと `coverage_own_insns_executed` と一致の参考値に使う。後者 2 つは前回と同一）。
3. **native の timeout 記録 14 件で `fgpr`（13 件）か `fault_rip`（6 件）が違う**。再実行後の `a5-native-faults.json`（2,465 件）では、前回と違うのは 11 件で、違う項目は `rip` だけ（`0x5f0240` の case 10・13、`0xad7640` の 6・7・9・11、`0xad7710` の 2・5・10・12・13。すべて timeout_2s(loop)）。**読み**: ループ中のどこで 2 秒のアラームが落ちたかの違い。
4. **lift の abort 33 件（sigfpe）で `fault_addr` が違う**: 例 `928880.0` は `0x3e800047d42` → `0x3e800275fe5`。abort のメッセージは全件同じ。**読み**: 上位 32 ビットの 0x3e8 = 1000 は uid で、下位は pid と見え、`SIGABRT` の `si_addr` は `si_pid`/`si_uid` と同じ領域を使うので、プロセス ID が違うだけ。pid の値自体は照合していない。
5. `fgpr`（fault 時のレジスタ）の差: 2,467 件の lift 記録で違う。例 `4819d0.0` は `0x7fffffffae38` → `0x7fffffffa948` と、ホストのスタックのアドレスで、実行ごとに違う値。**読み**: host 側のアドレス（PIE と環境変数・引数のサイズ）の差で、比較の対象ではない。全件は照合していない。

**読み**: 1〜3 は壁時計時間に依存する項目（2 秒のアラーム、16 並列）で、今回の実行は前回より速かった（run_diff2.py が 6 分 46 秒 → 4 分 28 秒、ビルドが 52.9 秒 → 17.0 秒）。負荷の差によるもので、lifter の変更とは関係しないと見ている。負荷の差の原因は**未確認**。

### 6.3 lifter の退行か、オラクルの限界か、環境差か（読み）

- **lifter の退行**: 見当たらない。選んだ 2,000 関数（seed 1、2）の生成 C が前回と同一で、13,535 件の ok ケースの出力が前回と全項目同一であることは、「この範囲では lift の出力が変わっていない」ことの直接の証拠。native が戻ったケースでは native と bit 単位で一致する。
- **オラクルの限界**: 今回の測定は、HEAD で追加された命令の lift（0075 の SSE/SSE2 26 ニーモニック、0076 の `gs:[0x10]`、0077 の `call [IAT]`、0079 のスタブ）を**検査できていない**。FLEngine の生成 C がこれらで 1 関数も変わらなかった（§4.1）ので、検査する対象がそもそもない。これらを A5 で検査するには、dsp_ippv2 のような別 DLL を対象にして native の側を用意する必要がある（そのための基盤がこのオラクルにあるかは**未確認**）。
- **環境差**: 同一の機械（AMD Ryzen 9 9950X）で、同一の PE、同一の clang。生ログの差は §6.2 の壁時計時間と PID/ASLR の項目だけ。

## 7. seed 2 の測定（追加）

同じ手順で seed だけ 2 にした。`gen_cases` の乱数は `Random((seed << 32) ^ fn_va ^ 0x5A5A)`（`run_diff2.py` 110 行）、選択は `Random(seed).sample(pool, 200)` ＋ `Random(seed+1).shuffle(rest)`（同 482〜487 行）なので、seed 2 は関数の選択もケースの入力も seed 1 とは別。

### 7.1 カテゴリ（実測、16,000 ケース）

| 判定 | seed 1 | seed 2 |
|---|---|---|
| bit_exact | 13,535 | **13,616** |
| mismatch_core | 0 | **0** |
| mismatch_aux | 0 | **0** |
| lift_unsupported | 0 | **0** |
| lift_stop_other | 0 | **0** |
| native_fault | 2,465（15.4%） | **2,384（14.9%）** |
| 関数: bit_exact_all_comparable / native_fault_all | 963 / 37 | 964 / 36 |
| ptr_input の関数 / case / native_fault | 651 / 10,416 / 2,351（22.6%） | 642 / 10,272 / 2,161（21.0%） |
| no_ptr の関数 / case / native_fault | 349 / 5,584 / 114（2.0%） | 358 / 5,728 / 223（3.9%） |
| v1 と同じ 200 関数（ベース）: bit_exact / native_fault | 2,741 / 459 | 2,709 / 491 |

**mismatch は 0 件（core、aux とも）。列挙する関数 VA・入力・差分はない**（`a5-mismatches.json` は `[]`、`mismatch_fields`、`lift_unsupported_messages`、`lift_stop_other_messages` は空）。

### 7.2 補助の指標と native_fault の内訳

| 指標 | seed 1 | seed 2 |
|---|---|---|
| `mxcsr_lift_ne_reset` / `mxcsr_native_ne_reset` | 729 / 729 | 859 / 859 |
| EFLAGS 定義済みの差 | 0（CF 13,528、PF/ZF/SF 13,470、AF 10,693、OF 13,475） | 0（CF 13,560、PF/ZF/SF 13,524、AF 10,950、OF 13,497。AF 未定義 2,666） |
| x87 の差、MXCSR 状態ビットだけの mismatch_aux | 0、0 | 0、0 |
| 実行された命令の割合（平均 / 中央値 / 全命令実行 / ゼロ） | 91.4% / 100% / 808 / 37 | 91.5% / 100% / 821 / 36 |
| timeout 再実行 | 18 件 | 14 件 |
| `native_fault_same_fault_addr` | 2,415 / 2,465（差は sigfpe 33 + timeout 17） | 2,344 / 2,384（差は sigfpe 29 + timeout 11） |
| `bit_exact_cases_with_image_or_scratch_writes` | 4,820 | 5,061 |

native_fault の原因: seed 2 は null_deref 1,366、wild_pointer 552、gp_noncanonical_address 216、scratch_range_exceeded 207、sigfpe(divide) 29、timeout_2s(loop) 11、gp_other 2、stack_range_exceeded 1（seed 1 は 1,223 / 650 / 289 / 243 / 33 / 17 / 2 / 7、write_to_readonly_image 1）。lift 側の挙動は fault 2,344、abort 29、timeout 11。

全 case が native_fault の関数（seed 2、36 件）: 0x10049b0、0x1005730、0x1039d20、0x1042070、0x107e5d0、0x1101af0、0x11e83a0、0x11e8440、0x11f70f0、0x1296620、0x12a75e0、0x12d8880、0x1310300、0x4c0690、0x54f810、0x565820、0x609c20、0x6bbf80、0x8223f0、0x823c30、0xa953c0、0xc84d80、0xcdb230、0xd2d360、0xd89760、0xd89a20、0xd89b50、0xdafe10、0xe02a20、0xe16520、0xe44ef0、0xeba970、0xf11480、0xf34f90、0xfed8c0、0xff9880。seed 1 の 37 件と共通なのは 0xe16520、0xe44ef0、0xff9880 の 3 件。

native の timeout 11 件は 3 関数（0x54f810 の case 0/4/8、0x5f0460 の 0/8/14、0xad7770 の 2/7/10/12/13）。

**no_ptr の native_fault が seed 2 で 223 件（seed 1 は 114 件）**: 全 case が fault の no_ptr 関数が seed 2 では 13 件（208 case）、seed 1 では 4 件（64 case）。**読み**: `argptr.py` の静的な判定が、ポインタとして使われる引数を取りこぼしている関数が、seed 2 の選択に多く入った。個別の関数は追跡していない（**未確認**）。

### 7.3 ネガティブコントロール（実測、203 件）

| 結果 | seed 1 | seed 2 |
|---|---|---|
| detected_core | 113 | 116 |
| detected_aux_only | 10 | 7 |
| detected_by_native_fault_only | 64 | 64 |
| undetected_state_changed_but_masked | 4 | 10 |
| undetected_equivalent_no_state_change | 12 | 6 |
| 出力の差で検出（core + aux） | 123/203 = 60.6% | 123/203 = 60.6% |
| native fault も含めて検出 | 187/203 = 92.1% | 187/203 = 92.1% |
| core のみ | 113/203 = 55.7% | 116/203 = 57.1% |
| 未検出 | 16（実行 case 1: 6、2 以上: 10） | 16（実行 case 1: 1、2 以上: 15） |

定数返却関数の imm 3 件は seed 2 でもすべて detected_core。出力で検出した 123 と全体の 187 が両 seed で同数になったのは、実測した数がたまたま一致したもの（他の内訳は違う）。反転の対象にした命令は、seed 2 では別の関数・命令（同じ（関数、命令）が seed 1 と重なるのは 4 件）。

seed 2 の未検出 16 件:

| 関数 | 命令 VA | 役割 | 元の命令 | 実行 case | 結果 |
|---|---|---|---|---|---|
| 0x79b640 | 0x79b64f | disp | `mov eax, dword ptr [rbx + 0x5bc]` | 16 | masked |
| 0x808f40 | 0x808f46 | imm | `test eax, 0x200` | 16 | equivalent |
| 0x4145f0 | 0x414663 | imm | `call 0x414350` | 15 | masked |
| 0xc6cb60 | 0xc6cb74 | imm | `jne 0xc6cb98` | 16 | masked |
| 0x10d79e0 | 0x10d79e4 | opcode | `mov rsp, rbp` | 16 | equivalent |
| 0x408630 | 0x408638 | opcode | `test rcx, rcx` | 9 | masked |
| 0xa05c30 | 0xa05c30 | opcode | `test rdx, rdx` | 16 | masked |
| 0x650fe0 | 0x6510e2 | disp | `lea rax, [rax - 1]` | 7 | masked |
| 0x79bec0 | 0x79c04e | disp | `cmp dword ptr [r9 + 0x528], r8d` | 15 | masked |
| 0xb55710 | 0xb63605 | opcode | `mov rcx, qword ptr [rsp + 8]` | 16 | equivalent |
| 0x10644e0 | 0x1064535 | imm | `cmp dword ptr [rbx + 0x18], -1` | 6 | masked |
| 0x67aa60 | 0x67aaa2 | opcode | `sub r11d, r10d` | 1 | equivalent |
| 0x4794c0 | 0x4794f4 | opcode | `test eax, eax` | 15 | masked |
| 0x605900 | 0x605908 | imm | `jmp 0x60590e` | 14 | masked |
| 0x7f5cd0 | 0x7f5da7 | imm | `je 0x7f6338` | 16 | equivalent |
| 0xb981f0 | 0xb98268 | opcode | `ucomisd xmm0, qword ptr [rbx + 0xb8]` | 5 | equivalent |

seed 1 の未検出 16 件と重なるものはない。seed 2 では masked が増えている（4 → 10）。個々の変異が出力に出ない理由は命令ごとには追跡していない（**未確認**）。

### 7.4 seed 1 と seed 2 の選択の重なり（実測）

| 項目 | 件数 |
|---|---|
| 各 seed の選択 | 1,000 関数 |
| **重なり（両方に入った関数）** | **92** |
| 和集合 | 1,908（純計算 9,901 件の 19.3%、全 59,297 関数の 3.2%） |
| 和集合のうち bit_exact の case が 1 件以上ある関数 | 1,838 |
| ベース 200 関数（`Random(seed).sample`）の重なり | 6 |
| 重なった 92 関数の判定 | 89 が両 seed とも bit_exact_all_comparable、3 が両 seed とも native_fault_all（0xe16520、0xe44ef0、0xff9880） |
| bit_exact の case 合計（2 seed） | 27,151 / 32,000 |

**読み**: 2 つの選択が独立なら重なりの期待値は 1,000 × 1,000 / 9,901 ≈ 101。実測の 92 はその付近だが、統計的な検定はしていない。

重なった 92 関数: 0x4175e0、0x44ebb0、0x4747a0、0x475f70、0x4781e0、0x4782a0、0x4b4000、0x520440、0x528970、0x528d50、0x529980、0x5338b0、0x533c10、0x538c20、0x53a8b0、0x578ae0、0x59b8c0、0x5c5e10、0x60a3c0、0x650fe0、0x660d30、0x67b480、0x6da8e0、0x707630、0x707a30、0x71c790、0x72c7a0、0x75d2a0、0x7a0b10、0x7e1510、0x804a80、0x83c780、0x879170、0x88b690、0x89dfd0、0x8c7d40、0x8f6990、0x920020、0x928880、0x92e920、0x95c330、0x986ab0、0x988830、0x98f4d0、0x995bf0、0x99aa10、0x9c9030、0xa3e110、0xa4bf70、0xabd2c0、0xad1fd0、0xade4c0、0xb35cf0、0xb554b0、0xbc84e0、0xbcf140、0xbec750、0xbf1da0、0xbf1f10、0xbf3fd0、0xc96460、0xc96f10、0xc99e10、0xd26430、0xd7a750、0xd8c560、0xd928a0、0xdd1a50、0xe16520、0xe20710、0xe39a90、0xe44ef0、0xe84e90、0xe920a0、0xed8800、0xf7b730、0xf8f480、0xfc0d90、0xfc0da0、0xff9880、0x1006cb0、0x101e950、0x102c120、0x10c5510、0x10d6600、0x11488e0、0x120c120、0x12755c0、0x1277cb0、0x12c74d0、0x12c8720、0x12ef1a0。

## 8. 限界

- **対象は純計算 9,901 件**（候補 57,816 の 17.1%、全 59,297 関数の 16.7%）**に限る**。選別から外れた 47,915 件は、IAT 経由の import 呼び出し、間接 call/jmp、x87、FS/GS、cpuid など、またはそれらを呼び先に持つ関数で、**オラクルの対象外**（0035）。`fle_imp_*` の応答表と同じ応答を native 側に用意していないため、import を呼ぶ関数は native で動かせない。今回の除外の内訳は前回と同じ（GS/FS を自身が持つのは 1 件だが、届く関数が 36,913 件、x87 を自身が持つのは 74 件で、呼び先経由を含めて 2,893 件）。x87 の演算と TEB アクセスは検査していない。
- **サンプルは小さい**: seed 1 の 1,000 関数は全 59,297 関数の **約 1.7%**（1,000/59,297 = 1.69%）、純計算の約 10.1%。seed 1 と seed 2 の和集合 1,908 関数でも、全体の 3.2%、純計算の 19.3%。全体の等価性は主張できない。`bit_exact` は「その入力で一致した」であって、関数全体の等価性ではない。1 関数 16 ケース、入力は乱数と境界値で、通す経路は一部。全命令を実行した関数は seed 1 で 808/1,000、seed 2 で 821/1,000。
- **native_fault は約 15%**（seed 1 15.4%、seed 2 14.9%）。native が戻らないケースは比較不能で、何も主張しない。関数単位でも seed 1 で 37、seed 2 で 36 関数は全 case が native_fault（この関数については一致も不一致も言えない）。原因の大半は null_deref と wild_pointer（`argptr.py` の静的な近似で、構造体を 3 段以上辿る関数や、文字列長・ベクタ長を引数が決める関数は fault しやすい。0036 §7）。fault は lift 側でも同種の fault か abort/timeout になる（同じ fault アドレスは seed 1 で 2,415/2,465、seed 2 で 2,344/2,384）が、これは一致の証拠にはならない。
- **今回の再実行は、HEAD で追加された命令の lift（0075 の SSE/SSE2 26 ニーモニック、0076 の gs:[0x10]、0077 の call [IAT]、0079 のスタブ）を検査していない**。FLEngine の生成 C がこれらで変わっていないため（§4.1）。runtime の変更（0078、0083、0084、0085 を含む 22 ファイル）も、純計算関数が呼ばない範囲を検査していない。
- 比較しないもの: 未定義の EFLAGS（seed 1 で AF 2,842 case、seed 2 で 2,666）、GS/TEB、MM レジスタの値、x87 の 80 ビットレジスタの内容、MXCSR の RC/FTZ/DAZ が非既定の状態（native は 0x1F80 で入る。0038 §7）。
- 確認は AMD Ryzen 9 9950X の 1 台のみ。**Intel 実機との比較はしていない**（未確認）。MXCSR の状態ビットはホストの命令が立てたものを写しているので、CPU が違うと結果が違う可能性がある（0038 §7）。
- seed は 1 と 2 の 2 つだけ。3 つ目以降は走らせていない。run5 の個別ケース（`results/run5/a5-cases.json` など）との case 単位の突き合わせは今回もしていない。比較対象は前回の再実行（d6c24be）の個別ファイルと、前回記録の run5 の数値。
- ネガコンの検出は反転が出力に出る場合に限る（未検出 16 件は seed 1、seed 2 とも。出力が変わらない等価な変異が含まれる）。検出の下限を測るもので、全件検出は期待していない。

## 9. 置き場と注意

- 生成物はすべて `lift/out-a5-m85/`（`.gitignore` の `lift/out*/` に該当。`git check-ignore -v` で確認）。主な置き場:
  - 生成 C とビルド: `src/lift/out-oracle/gen/`（`a5m85-pdata.c` 約 215 MB、`new-part-00〜16.c`）、`parts/`、`rt/`、`obj/`、`bin/`（`lift_drv2` 258 MB、`native_drv2`）。`lift/out-a5-m85/` 全体は約 1.4 GB で、2 GB を超えるファイルはない。
  - seed 1 の結果: `src/lift/out-oracle/results/rerun/`（`a5-summary.json`、`a5-cases.json/.csv`、`a5-functions.json`、`a5-negctl.json`、`a5-native-faults.json`、`a5-mismatches.json`、`run.txt`、`run.err`、`work/`）。seed 2: 同 `results/seed2/`。
  - `lift/out-a5-m85/a5m85-build.sh`（ビルド）、`lift/out-a5-m85/cmp_bodies.py`（関数本体の比較。読み取り専用）、`build.txt`。
- 前回の結果（`lift/out-oracle-rerun/`）は読み取りだけ。変更していない。
- 規則: `rm`、既存ファイルの全体上書き、git の書き込み系操作、`/tmp`、末尾の `&` は使っていない。TMPDIR は `lift/out-a5-m85/tmp`（ドライバは `src/lift/out-oracle/tmp`）。新規ファイルは `docs/investigations/a5-oracle-rerun-2.md` と `lift/out-a5-m85/` の下だけ。権限拒否はなかった。
- **規則に対する自己申告（2 点）**: (1) `python3 -c` の短い確認コマンドを 2 回、`PYTHONDONTWRITEBYTECODE=1` を付けずに実行した（最初の環境確認の `import capstone; print(capstone.__version__)` と、pyc の置き場を調べた `import capstone,os; print(os.path.dirname(capstone.__file__))`）。どちらも capstone を import しただけのコマンドで、capstone のディレクトリに今日の日付の `.pyc` が増えていないことは `find -newermt` で確認した。`lift/out-a5-m85/` には `__pycache__` がない。一方、同じ確認で uv の python 標準ライブラリ内に今日の日付の `.pyc` が 1 つ見つかった（`…/cpython-3.14.8-linux-x86_64-gnu/lib/python3.14/encodings/__pycache__/rot_13.cpython-314.pyc`）。これが私の 2 回の実行で作られたものか、他の作業のものかは**未確認**（capstone の import が `rot_13` を読む理由は確かめていない）。私は削除していない（rm 禁止）。リポジトリ内の他の場所（`lift/out-regstop/tools/__pycache__/rdis.cpython-314.pyc`、`lift/out-loadlib/__pycache__/llpe.cpython-314.pyc`）に今日の日付の `.pyc` があるが、私が実行したものではない（並行作業のもの。未確認だが、私はそのディレクトリを触っていない）。(2) 集計用のインライン python（here-document）の中に、`'timeout'`（status の値の文字列）や `timeout_retry`（summary のキー名）という文字列を書いた。`timeout` コマンドは使っていない（長い実行は shell ツールの timeout パラメータで待った）が、「コマンド文字列に timeout を書かない」という規則の文言には触れている。
- ワークツリーの未追跡ファイル `lift/harness/wasm/trace.c.new`、`lift/harness/wasm/trace_native.c.new`、`lift/iat-unsup-closure.txt` は、私の作業の前から存在した（作業開始時の `git status --short` に出ていた）。私のものではない。
