# A5 native オラクルの再実行（HEAD d6c24be）

日付: 2026-10-22。対象は `git archive HEAD`（d6c24be）のスナップショット。設計は `docs/decisions/0035-a5-oracle.md`、`0036-a5-oracle-v2.md`、`0038-mxcsr-flags.md`。比較対象の run5 の数値は `docs/decisions/0046-m1-acceptance.md` と `docs/investigations/mxcsr-flags-run.md` §2.4 から取った。「実測」はツールの出力、「仮説」は未検証の解釈。

## 1. 結論（実測）

run5 と同じ手順・同じパラメータ（1,000 関数 × 16 ケース、seed 1）で、HEAD から lift 側を全部作り直して実行した。**カテゴリ別の件数は run5 と完全に一致した。** mismatch は 0 件（core、aux とも）、lift_unsupported は 0 件、native_fault は 2,465 件のまま。

この結果が示すのは「選んだ 1,000 関数のうち、native が戻った 13,535 ケースでは、HEAD の lift が GPR・XMM・メモリ・EFLAGS（定義済みフラグ）・MXCSR・x87 の状態まで native と一致した」ことだけである。関数全体の等価性や、対象外の関数（§5）については何も言えない。

## 2. 実行コマンド

スナップショットは `lift/out-oracle-rerun/stage/`（`git archive HEAD | tar -x`。`binaries` は展開から除外し、`FLENGINE_PE` でワークツリーの PE を指す）。ワークツリーの `lift/` は読んでいない。

```
R=/home/ubuntu/work/signature-studio-develop/lift/out-oracle-rerun
export CARGO_TARGET_DIR=$R/target
export TMPDIR=$R/tmp                                # cargo 時。rerun-build.sh と選別・差分実行は $R/stage/lift/out-oracle/tmp を使う
export FLENGINE_PE=/home/ubuntu/work/signature-studio-develop/binaries/win64/FLEngine_x64.dll

# 1. lifter のビルド（スナップショット上、7 秒）
cd $R/stage/lift && cargo build --release -p fe-lift

# 2. 生成→分割→clang→ドライバのリンク（52.9 秒。$R/rerun-build.sh、出力は $R/build.txt）
sh $R/rerun-build.sh
#   fe-lift lift $FLENGINE_PE out-oracle/gen/rerun-pdata.c --pdata
#   python3 harness/wasm/split_c.py rerun-pdata.c gen/new-part 3700      (59,297 関数、17 分割)
#   clang -std=gnu11 -O0 -g -w -fno-builtin -ffp-contract=off -fPIE -finstrument-functions   (生成 C。build5.sh と同じフラグ)
#   runtime/*.c を -O0 -g -Wall -Wextra -fPIE、oracle2_common/native_drv2/lift_drv2 を -O1、image.c を -O1
#   native_drv2（image.o + ms_call2.S）と lift_drv2（生成 C 17 本 + runtime 全部）をリンク

# 3. 選別、ポインタ判定、除外集計、差分実行（cwd は $R/stage/lift/out-oracle、H=../harness/oracle）
python3 $H/select_pure.py $FLENGINE_PE gen/new-part-00.c results/pure.json
python3 $H/argptr.py $FLENGINE_PE results/pure.json results/rerun/argptr.json
python3 $H/exclude_stats.py $FLENGINE_PE gen/new-part-00.c results/rerun/exclude-stats.json > results/rerun/exclude-stats.txt
python3 $H/run_diff2.py --pe $FLENGINE_PE --pure results/pure.json --argptr results/rerun/argptr.json \
    --exclude results/rerun/exclude-stats.json --out results/rerun --count 1000 --n 16 --seed 1 --negctl 200 --jobs 16
```

`run_diff2.py` の所要時間は 6 分 46 秒（real、16 並列。native 通常 + lift + native 単一ステップ + ネガコン 203 件）。ドライバの終了コードは native 0、lift 0、native-trace 0。環境は AMD Ryzen 9 9950X（16 コア）、clang 21.1.8、capstone 5.0.7。

結果の置き場: `lift/out-oracle-rerun/stage/lift/out-oracle/results/rerun/`（`a5-summary.json`、`a5-cases.json`、`a5-cases.csv`、`a5-functions.json`、`a5-negctl.json`、`a5-native-faults.json`、`a5-mismatches.json`、`run.txt`）。

### run5.sh との違い

| 項目 | run5.sh（当時） | 今回 |
|---|---|---|
| 生成 C | `lift/out-mxcsr/pd/out/new-part-NN.c`（当時のワークツリーから生成） | HEAD のスナップショットから `fe-lift lift --pdata` で再生成（`rerun-build.sh`） |
| runtime | 当時のワークツリー | HEAD のスナップショット（0044 の純 C 版 `fle_mxcsr_flags.h` を含む。ヘッダ冒頭に "No asm, no <emmintrin.h>, no <fenv.h>" とある） |
| native_drv2 | run4 のバイナリをコピー | HEAD のソースから再ビルド |
| `argptr.json`、`exclude-stats.json` | run4 のものをコピー | 再生成（件数は run4 の記録と一致。ファイルのバイト比較はしていない） |
| `pure.json` | 既存を再利用 | `select_pure.py` で再生成 |

`build5.sh`/`run5.sh` はそのままでは使っていない（`out-mxcsr`、`out-oracle`、`out-pdata` のワークツリー側の生成物に依存するため）。

## 3. 結果の比較（実測）

### 3.1 カテゴリ（16,000 ケース）

| 判定 | run5 | 今回（HEAD d6c24be） | 差 |
|---|---|---|---|
| bit_exact | 13,535 | **13,535** | 0 |
| mismatch_core | 0 | **0** | 0 |
| mismatch_aux | 0 | **0** | 0 |
| lift_unsupported | 0 | **0** | 0 |
| lift_stop_other | 0 | **0** | 0 |
| native_fault | 2,465 | **2,465**（15.4%） | 0 |

`a5-summary.json` の `categories` は `{"bit_exact": 13535, "native_fault": 2465}` のみ。`mismatch_fields` は空、`a5-mismatches.json` は `[]`、`lift_unsupported_messages` と `lift_stop_other_messages` も空。

関数単位（1,000）: bit_exact_all_comparable **963**、native_fault_all **37**（run5 と同じ。has_mismatch_* は 0、lift_not_executable は 0）。

ポインタ入力別:

| 種別 | case | bit_exact | native_fault |
|---|---|---|---|
| ptr_input（651 関数） | 10,416 | 8,065 | 2,351（22.6%） |
| no_ptr（349 関数） | 5,584 | 5,470 | 114（2.0%） |

（run4 の記録では ptr_input の native_fault 2,351、no_ptr の 114 で、今回と同じ。run4→run5 の bit_exact の増分は mismatch_aux 717 と lift_unsupported 13 の解消分。）

v1 と同じ 200 関数（3,200 case）: bit_exact 2,741、native_fault 459。

### 3.2 mismatch の列挙

mismatch は 0 件のため、関数 VA・入力・差分の列挙対象はない。

### 3.3 native_fault の内訳（2,465 件）

| 原因 | 件数 | run4 の記録 |
|---|---|---|
| null_deref（実効アドレス < 0x10000） | 1,223 | 1,223 |
| wild_pointer | 650 | 650 |
| gp_noncanonical_address | 289 | 289 |
| scratch_range_exceeded | 243 | 243 |
| sigfpe(divide) | 33 | 33 |
| timeout_2s(loop) | 17 | 17 |
| stack_range_exceeded | 7 | 7 |
| gp_other | 2 | 2 |
| write_to_readonly_image | 1 | 1 |

native_fault の lift 側の挙動（`native_fault_vs_lift`）: fault と abort/timeout の別は、null_deref/fault 1,223、wild_pointer/fault 650、gp_noncanonical_address/fault 289、scratch_range_exceeded/fault 243、sigfpe/abort 33、timeout/timeout 17、stack_range_exceeded/fault 7、gp_other/fault 2、write_to_readonly_image/fault 1。`native_fault_same_fault_addr` は 2,415（run4 の記録は 2,412。差の 3 件は `scratch_range_exceeded` で lift 側が abort から fault に変わった件で、mxcsr-flags-run.md §2.4 の run5 の記述と同じ）。

native_fault の割合が変わっていないのは、選別・seed・入力生成が同じで、native 側の挙動は lift の変更に依存しないため（native_fault は native が戻らないケース）。

### 3.4 補助の指標

| 指標 | run5（記録） | 今回 |
|---|---|---|
| 純計算関数（`select_pure.py`） | 9,901 | 9,901（候補 57,816） |
| `mxcsr_lift_ne_reset` / `mxcsr_native_ne_reset` | 729 / 729 | 729 / 729 |
| mismatch_aux のうち MXCSR 状態ビットだけ | 0 | 0 |
| EFLAGS 定義済みフラグの差 | 0 | 0（CF 13,528、PF/ZF/SF 13,470、AF 10,693、OF 13,475 を比較。未定義で比較しなかった AF は 2,842） |
| x87（cw/sw/tw）の差 | 0 | 0 |
| native 通常実行と単一ステップの一致 | — | equal 13,528、両側 fault 2,412、両側 timeout 17、fault→timeout 36、ok→timeout 7 |
| タイムアウト再実行 | — | 19 件（native: timeout 17 / ok 2、lift 再実行後: timeout 17 / ok 2） |
| 実行された命令の割合（平均 / 中央値 / 全命令実行の関数） | — | 91.4% / 100% / 808（ゼロは 37 関数） |

`mxcsr_lift_ne_reset` = 729 は、native と lift の両方が MXCSR を 0x1F80 以外にした case の数で、native と同数。MXCSR 状態ビットを lift が更新していることを示す（0038）。

選別の除外分類（`exclude-stats.json`）も run4 の記録と同じ: 純計算 9,901、自身が IAT 呼び出し 3,407、自身が間接 call 15,097、自身が間接 jmp 2、自身が x87 74、自身が cpuid など 4、呼び先だけが IAT 25,928 / 間接 call 3,325 / 間接 jmp 67 / x87 2 / cpuid など 9。

## 4. ネガティブコントロール（実測、203 件）

native だけ、単一ステップで実行された命令の opcode/displacement/immediate の 1 バイトを XOR 0x01（200 件）と、定数返却関数の imm（3 件）。

| 結果 | run5（記録） | 今回 |
|---|---|---|
| detected_core | 113 | 113 |
| detected_aux_only | 10 | 10 |
| detected_by_native_fault_only | 64 | 64 |
| undetected_state_changed_but_masked | 4 | 4 |
| undetected_equivalent_no_state_change | 12 | 12 |
| **出力の差で検出**（core + aux） | 123/203 = **60.6%** | 123/203 = **60.6%** |
| native fault も含めて検出 | 187/203 = 92.1% | 187/203 = 92.1% |
| core のみ（v1 の比較対象） | 113/203 = 55.7% | 113/203 = 55.7% |
| 未検出 | 16 | 16（実行 case 数が 1 のもの 6、2 以上のもの 10） |

role 別: opcode は detected_core 65、native fault のみ 38、等価 6、途中状態は変化 2（計 111 件）。disp は core 38、aux 6、fault のみ 20、等価 4、途中状態は変化 1（計 69 件）。imm は core 10、aux 4、fault のみ 6、等価 2、途中状態は変化 1（計 23 件）。種類別では、実行された命令のバイト 200 件が core 110、aux 10、fault のみ 64、masked 4、等価 12。定数返却関数の imm 3 件はすべて detected_core（健全性の確認は通った）。

未検出 16 件（関数 / 命令 VA / 役割 / 命令 / 実行 case 数 / 結果）:

| 関数 | 命令 VA | 役割 | 元の命令 | 実行 case | 結果 |
|---|---|---|---|---|---|
| 0x6351b0 | 0x6351c8 | disp | `movzx rcx, byte ptr [rip+0x24]` | 7 | masked |
| 0xf7b3d0 | 0xf7b3d4 | opcode | `mov rsp, rbp` | 16 | equivalent |
| 0x6bebd0 | 0x6bebd9 | disp | `movups xmm2, [rip+0xf373f8]` | 16 | equivalent |
| 0x409c80 | 0x409cc2 | disp | `mov dword ptr [rdx+r8-4], ecx` | 1 | equivalent |
| 0xc40c00 | 0xc40c04 | opcode | `mov rsp, rbp` | 16 | equivalent |
| 0x10e47c0 | 0x10e47ea | opcode | `cmp qword ptr [rbp], rax` | 1 | masked |
| 0x41c770 | 0x41c774 | opcode | `mov rsp, rbp` | 16 | equivalent |
| 0x12dea10 | 0x12dea39 | opcode | `mov edx, [rip+0x6900e5]` | 2 | equivalent |
| 0x442770 | 0x442835 | imm | `mov eax, 0x51eb851f` | 1 | masked |
| 0x1312f70 | 0x1312f78 | disp | `mov eax, [rip+0x65bcda]` | 16 | equivalent |
| 0x728ef0 | 0x728ef0 | disp | `mov rax, [rip+0xecd261]` | 16 | equivalent |
| 0x4146c0 | 0x41467a | opcode | `movzx eax, word ptr [r14]` | 1 | equivalent |
| 0x478a30 | 0x439799 | imm | `jl 0x4397b1` | 16 | equivalent |
| 0x11db3b0 | 0x11db3cb | imm | `test eax, 2` | 10 | equivalent |
| 0x10bb130 | 0x10bb141 | opcode | `mov rax, [rbp+0x28]` | 1 | equivalent |
| 0x52a700 | 0x52a705 | opcode | `test rax, rax` | 1 | masked |

検出率は run5 と同一で、run4（127/203 = 62.6%）より低い状態が続いている。未検出の個別の原因は命令ごとには追跡していない。仮説: `mov rsp, rbp` の opcode 反転（`mov spl, bpl` 相当になる）や、RIP 相対の disp 反転が同じ値を読む場合など、入力で値が変わらない変異が多い（run4 の記録 `a5-oracle-run2.md` §4 の同種の例からの推測。今回の 16 件の再デコードはしていない）。

反転した 200 件の関数は、元の結果が bit_exact だった case の命令列から選ばれる。run5 と未検出の関数・命令が完全に同じかは、run5 の個別の一覧を読んでいないので未確認（件数の内訳は一致）。

## 5. 限界

- **対象は純計算 9,901 件のうち 1,000 件**（約 10%）。選別から外れた 47,915 件（候補 57,816 の 82.9%）は、IAT 経由の import 呼び出し、間接 call/jmp、x87、FS/GS、cpuid など、またはそれらを呼び先に持つ関数で、**オラクルの対象外**（0035、`a5-oracle-run2.md` §3）。lift の `fle_imp_*` 応答表と同じ応答を native 側に用意していないため、import を呼ぶ関数は native で動かせない。
- 特に GS/FS を自身が持つ関数は 1 件だけだが、そこへ届く関数が 36,913 件あり、x87 を自身が持つのは 74 件（呼び先経由を含めて 2,893 件）。x87 の演算と TEB アクセスは、このオラクルでは検査していない。MMX の値も比較していない（0036）。
- native が戻らない 2,465 ケース（15.4%）は比較不能で、何も主張しない。関数単位では 37 関数が全ケース native_fault。
- ポインタ引数は静的な近似で判定している。構造体を 3 段以上辿る関数、文字列長やベクタ長を引数が決める関数は fault しやすい（0036 §7）。
- 1 関数 16 ケース、seed は 1 のみ。全命令を実行した関数は 808/1,000。`bit_exact` は「その入力で一致した」であって、関数全体の等価性ではない。
- 純計算関数は小さい（0035 の記録で命令数の中央値 5、最大 1,784。今回は再集計していない）。
- 比較しないもの: 未定義の EFLAGS（AF が 2,842 ケース）、GS/TEB、MM レジスタの値、x87 の 80 ビットレジスタの内容、MXCSR の RC/FTZ/DAZ が非既定の状態（native は 0x1F80 で入る。0038 §7）。
- 確認は AMD Ryzen 9 9950X の 1 台のみ。Intel 実機との比較はしていない。
- **今回の再実行で確認できた範囲**: 0042（ロケール）、0043（x87 の f2xm1/fsincos）、0044（MXCSR の純 C 版）、0047、0049 などを含む HEAD の lifter と runtime で、上の対象に限って run5 と同じ結果になった。これらの変更が対象外の関数に与える影響は、このオラクルでは測れない。
- run5 の個別ケース（`results/run5/a5-cases.json` など）との case 単位の突き合わせは、ワークツリーの `lift/` を読まない条件のためしていない。比較は、決定文書と調査記録に書かれた run5 の数値との集計レベルの一致である。

## 6. 試行錯誤・注意

- 最初に `cargo build --release -p fe-lift` だけを実行した（`fe-decode`、`fe-pe` は依存として自動でビルドされた）。テストは実行していない（目的外）。
- 生成 C（`rerun-pdata.c`、約 215 MB）と分割 C、`.o` 計 1.2 GB 程度がスナップショット内の `out-oracle/` にある。2 GB を超えるファイルは作っていない。削除はしていない。
- `lift/out-oracle-rerun/` は `.gitignore` の `lift/out*/` に該当する（`git check-ignore -v` で確認）。新規ファイルは本書のみ（`docs/` 側）。
- `rm`、既存ファイルの上書き、git の書き込み操作（`git archive` の読み取りのみ）、`/tmp`、sleep/timeout/nohup、末尾の `&` は使っていない。TMPDIR は `lift/out-oracle-rerun/` 配下。権限拒否はなかった。
