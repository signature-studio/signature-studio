# Blend2D の生成 C に「.pdata に無い葉関数 58 個」を種として足したとき、DllMain(ATTACH) の停止はどこへ移るか（測定のみ）

実行日: 2026-10-24。基準の HEAD は作業開始時 `92506ae95f46a0cea3b7cb2a9f0a12dbbecec630`。`git archive` はこのコミットから作った。**作業中に別の作業が 1 コミット進めた**（`925c522` は `docs/investigations/implicit-tls-design.md` の 1 ファイル追加だけ。`git diff --stat 92506ae HEAD` で確認。`lift/` の差分は 0。`lift/runtime` 114 ファイル、`lift/harness` 58 ファイル、`lift/crates` 152 ファイルは、`git show 92506ae:<path>` とワークツリーで SHA-256 が全部一致。実測）。測定は `92506ae` の木で行ったので、結果は `925c522` にも当てはまる（`lift/` が同一。読み）。**この文書は何も決めない。** 決定 0074（pdata の開始と一致する reloc 値だけを seed にする。開始でない値は種にしない）を変える決定ではない。今回の 58 個は 0074 が種にしないとした値に当たる。この測定は「停止がどこへ移るか」を見るためだけのもの。基底 0x4700000 は測定用の値で、決定ではない（決定 0086 は方向だけを決め、値は決めていない）。種 58 個も決定ではない。

作業物は `lift/out-seed58/`（gitignore 対象の `lift/out*/`）。

## 0. 凡例

- **実測** = このセッションでコマンドを実行して得た値。ログは `lift/out-seed58/` の下。
- **読み** = ソース、生成 C、別の文書を読んだ結果。実行して確かめていない。
- **未確認** = そのとおり。

## 1. 要約

| # | 項目 | 結果 | 区分 |
|---|---|---|---|
| 1 | 58 個のリスト | 調査の `reloc-not-function.txt`（58 行）を `fe-decode reloc-seeds --not-function` で出し直して全バイト一致。577 − 519 = 58。リベース後の VA への変換（元 − 0x180000000 + 0x4700000）を 3 通りで検算し一致（例 0x1801a57f8 → 0x48a57f8）。`fe-lift --seeds` は 1 行 1 VA の 16 進（`0x` 可）で、`fe-lift` を編集せずに使えた | 実測（§3） |
| 2 | 対照（種なし。新しい OUT `lift/out-seed58/ctl/`） | **three-module 文書と一致**: a4 = init_count 150/942、last unit 149（proc 0x664260）、functions_distinct 960、function_entries 26,604、停止 `indirect call to 0x48a57f8`。FLEngine の C `43985d62…37137a1`、リベース済み dsp `4fd25502…efc5`、Blend2D `24e01ec9…b4cb`。分割 C、表、`common.c` も `out-three` と一致。2 回同一 | 実測（§4） |
| 3 | 種 58 個を足したビルド（Blend2D の生成 C だけ。FLEngine、dsp には使わない） | 関数 **2,066**（+48）、命令 399,508、**`bad_nodes 0`**、**`unsupported_total 665`**（調査 §3.2 と一致）。58 個のうち 10 個は既に表にあり、新しく入ったのは 48 個。`f_<VA>` の衝突 0。統合表 69,653 行。`two-native` 612,248,328 B | 実測（§5） |
| 4 | **a4 の行（種 58 個あり。gdb なし 2 回）** | init_count = **150 / 942**（変わらず）、last unit 149（proc 0x664260、in_progress）、started 137 / completed 136、functions_distinct = **1,058**（960 → +98）、function_entries = **27,316**（26,604 → +712）。2 回同一 | 実測（§6.1） |
| 5 | **新しい停止** | **`fle: indirect call to 0x470a8a0 is not a known function entry`**（`fle_dispatch_miss`。top of stack `f_4709ea0`）。0x470a8a0（元 0x18000a8a0）は **58 個に無く、`.pdata` の関数開始でもない**。`f_4709ea0`（CPU 情報の関数。`.pdata` の `[0x4709ea0, 0x470abac)`）の **`jmp rcx`（switch 表の形）の移動先の 0 番**。調査 B3（`jmp reg` の switch 表）の形 | 実測（§6.1、§6.4） |
| 6 | Blend2D の DllMain(ATTACH) | **入ったが戻っていない**（eax なし。`fle_realmod.c:144` に未到達）。**LoadLibraryW は 0x4700000 を返していない**。FLEngine の `f_664190` は `GetProcAddress("AggCreateInstance")` に届いていない（import のログに 0 件） | 実測（§6.3） |
| 7 | 停止が移った先までの進み方 | 対照では ATTACH の import 13 回で止まったのが、**172 回**に増えた。CRT の表（16 バイト刻みの 12 エントリ）はすべて通り、`.xi` の 4 項目は全部戻り値 0 で戻り、**`.xc` の初期化子 `f_4701000` に入って**、その中の CPU 情報の関数で止まった。名前付き停止の import はなし。行なし 15 名は 1 回も実行されていない | 実測（§6.3、§6.5） |
| 8 | 種 58 個のうち実行されたもの | **2 個だけ**（`f_48a57f8` 1 回、`f_48a5830` 2 回。どちらも停止点だった値）。Agg2D の vtable の 34 個を含む残り 56 個は、停止までに 1 回も入られていない | 実測（§6.5） |
| 9 | 暗黙の TLS | `.xc` の中で `gs:[0x58]` と `_tls_index`（0）を使う読みが**停止の前に実行された**。`fle_tls_values[0]` に別の値が入っていたため落ちずに通った（偶然と読む）。調査 B6 は未解決 | 実測、推測（§6.6） |
| 10 | 停止点の分類 | **(v) その他**（調査 B3: `jmp reg` の switch 表の移動先が生成 C に無い）。(i)〜(iv) のどれでもない。unit 158（QFC）には届いていない | 実測（§7） |

つまり、**58 個を種に足すと、停止は「CRT の初期化表の葉関数（B5）」から、ATTACH の `.xc` の初期化子の中の「switch 表の移動先（B3）」へ移る**。それより前の部分（CRT の表、`.xi` 4 項目）は 58 個を足すだけで通った。その先（`GetModuleHandleA`、ロケール、JIT、スレッド）は未測定。**この文書は何も決めない**（決定 0074 との関係は §8）。

## 2. 手順（実測。実行した順）

`lift/` から実行。`TMPDIR=lift/out-seed58/tmp`（`/tmp` は使っていない）、`PYTHONDONTWRITEBYTECODE=1`。`<W>` は `/home/ubuntu/work/signature-studio-develop/lift`。

```
git archive HEAD | tar -x -C lift/out-seed58/src                                      # HEAD 92506ae95f46a0cea3b7cb2a9f0a12dbbecec630
cd lift/out-seed58/src/lift && CARGO_TARGET_DIR=<W>/out-seed58/target cargo build --release --offline -p fe-pe -p fe-lift
CARGO_TARGET_DIR=<W>/out-seed58/target cargo build --release --offline -p fe-decode   # fe-decode（reloc-seeds 用。同じ木）
# 58 個
out-seed58/target/release/fe-decode reloc-seeds out-blend2d/Blend2D.dll --not-function > out-seed58/seeds/reloc-not-function.regen.txt
python3（変換）→ out-seed58/seeds/seeds58-4700000.txt ; bash の算術で検算 ; fe-decode reloc-seeds --not-function をリベース済み DLL に実行して照合
# 対照（種なし）
cp -r out-three/harness-two/. out-seed58/harness-two/
MODULES="dll|<W>/out-ext/dsp_ippv2_x64.dll|0x2000000|--seeds <W>/out-seed58/ctl/dll/seeds-idx0-plus-initterm2.txt|497dd4f7…c352|800;blend2d|<W>/out-blend2d/Blend2D.dll|0x4700000|--pdata|66634d24…66c7|800" \
  LIFT=<W>/out-seed58/src/lift OUT=<W>/out-seed58/ctl FEBIN=<W>/out-seed58/target/release TAG=b2d FLE=…/binaries/win64/FLEngine_x64.dll bash out-seed58/harness-two/gen_two.sh rebase gen split merge
LIFT=… TAG=b2d OUT=<W>/out-seed58/ctl JOBS=8 bash out-seed58/harness-two/build_two_native.sh
DLL=…/FLEngine_x64.dll OUT=<W>/out-seed58/out-ctl NATIVE_BIN=<W>/out-seed58/harness-two/two-native-run.sh TWO_NATIVE=…/ctl/v-b2d/native/two-native \
  TWO_REAL=…/ctl/dll/dsp_ippv2_x64.2000000.dll:…/ctl/dll/Blend2D.4700000.dll FEPE=…/target/release/fe-pe bash out-seed58/src/lift/harness/a4-survey.sh        # 2 回（07:01:15Z、07:01:20Z）
# 種 58 個あり（Blend2D の lift-args だけ変える。TAG b2d58。FLEngine の生成 C、分割 C、runtime のオブジェクトは ctl/ からコピー）
MODULES="dll|…|--seeds …/s58/dll/seeds-idx0-plus-initterm2.txt|…;blend2d|…Blend2D.dll|0x4700000|--pdata --seeds <W>/out-seed58/seeds/seeds58-4700000.txt|66634d24…66c7|800" \
  LIFT=… OUT=<W>/out-seed58/s58 FEBIN=… TAG=b2d58 FLE=… bash out-seed58/harness-two/gen_two.sh rebase gen split merge
LIFT=… TAG=b2d58 OUT=<W>/out-seed58/s58 JOBS=8 bash out-seed58/harness-two/build_two_native.sh                       # two-native 612,248,328 B
… TWO_NATIVE=…/s58/v-b2d58/native/two-native TWO_REAL=…/s58/dll/dsp_ippv2_x64.2000000.dll:…/s58/dll/Blend2D.4700000.dll … bash …/a4-survey.sh   # 2 回（07:03:59Z、07:04:05Z）
GDBI_BIN=… GDBI_REAL=<dsp>:<blend2d> GDBI_FLE=… GDBI_OUT=…/run/roN-imports.txt … gdb -q -batch -x out-seed58/gdb/ro_imports_s58[b].py       # 読み取りだけ
（同様に gdb/ro_entries_s58.py、ro_tls_s58.py、ro_seedhits_s58.py。いずれも読み取りだけ）
python3 out-seed58/analysis/dis_b2d.py <rebased dll> VA N                                                            # 逆アセンブル（capstone。読み取り）
```

- `gen_two.sh` の `rebase` 段は、`blend2d`、`dll` ともに `fe-pe rebase` で作った（出力が無かったため。SHA-256 は期待値と一致）。種 58 個ありの OUT（`s58/`）には `ctl/` のリベース済み DLL をコピーしたので「exists」と出て飛ばした（同じファイル）。
- `s58/` の FLEngine の生成 C（`gen/flengine.c`）と分割 C（`src/fle-*.c`、`fle-tables.txt`）、`native/fle`、`native/rt`、`native/h` の `.o` は `ctl/` からコピーした（同じ HEAD の `git archive` から作ったもの。`gen_two.sh` が SHA-256 `43985d62…37137a1` を検査して一致を確認）。`realmod_table`、`common`、各モジュールのオブジェクトは TAG ごとに作り直した。
- a4-survey は `lift/out-seed58/src/lift/harness/a4-survey.sh`（HEAD の `git archive`。`lift/out-three/src/lift/harness/a4-survey.sh` と `cmp` で同一）。a4-survey 自体は変えていない。同じ秒に続けて走らせていない（2 回の間は 5〜6 秒）。
- **gdb なし、仮の値なし、runtime の応答追加なし**。gdb の run は読み取りだけ（§6.3）。値を入れていない。`lift/runtime`、`lift/crates`、`lift/harness` は編集していない（ワークツリーの `git status` は §8）。

## 3. 58 個のリストと変換の検算（実測）

| 項目 | 値 | 区分 |
|---|---|---|
| 調査のリスト | `lift/out-blend2d/reloc-not-function.txt`（58 行。元の VA、基底 0x180000000）。SHA-256 `ab10b381eae50a52285b2c055b039eeac24b8d56d6dddd9905aa448e5fa7199e` | 実測 |
| 自分で出し直した 58 個 | `fe-decode reloc-seeds lift/out-blend2d/Blend2D.dll --not-function`（HEAD の `git archive` から作った `lift/out-seed58/target/release/fe-decode`）。標準エラー: `reloc_slots 1763`、`reloc_unreadable 0`、`reloc_in_code 577`、`reloc_seeds 519`、`reloc_not_function 58`。出力 `lift/out-seed58/seeds/reloc-not-function.regen.txt` は調査のリストと**全バイト一致**（`cmp`。SHA-256 同じ）。数は **58 で合う**（577 − 519 = 58）。distinct 58 | 実測 |
| 519 個の seed 側 | `fe-decode reloc-seeds`（`--not-function` なし）の出力も調査の `reloc-seeds.txt` と全バイト一致（519 行） | 実測 |
| 変換 | 新 VA = 元の VA − 0x180000000 + 0x4700000。例 **0x1801a57f8 → 0x48a57f8**、0x1801a5830 → 0x48a5830（`fe-pe rebase` の差分 `0xfffffffe84700000` と同じ。three-module 文書 §5 が停止点として記録した値と一致） | 実測（算術） |
| 検算 1 | python の整数演算で 58 個を変換 → `lift/out-seed58/seeds/seeds58-4700000.txt`（0x 付き 16 進、1 行 1 VA）。範囲 `0x4706180`〜`0x48ab978` | 実測 |
| 検算 2 | bash の算術（`$((v - 0x180000000 + 0x4700000))`）で別に変換した結果が python の出力と**全バイト一致** | 実測 |
| 検算 3 | **リベース済み DLL**（`Blend2D.4700000.dll`）に対して `fe-decode reloc-seeds --not-function` を実行した出力（58 行）が、変換したリストと**全バイト一致**。つまり、変換後の 58 個は、リベース後の DLL から見ても「実行範囲内の DIR64 値のうち `.pdata` の関数開始に当たらないもの」の全部 | 実測 |
| 停止点の値との照合 | 変換後のリストに `0x48a57f8`（55 行目）と `0x48a5830`（56 行目）が入っている（three-module 文書 §5.2 の「dispatch 表に無い 3 個」のうち 2 種類） | 実測 |
| `--seeds` の書式 | `lift/crates/fe-lift/src/main.rs:21-26`: 1 行 1 VA、空行は無視、前後の空白は除去、`0x` の接頭辞を外して**16 進**で読む（`u64::from_str_radix(l.trim_start_matches("0x"), 16)`）。VA は**リベース済みの DLL の VA**（`fe-lift lift` に渡すファイルの ImageBase 基準）。`--pdata` と併用でき、種は和になる（`extra_seeds` が `--pdata` の種に追加する）。`fe-lift` を編集せずに使える | 読み（ソース。動作は §5 の実行で確認） |

## 4. 対照: 種 58 個なしの 3 モジュールを新しい OUT で作り直した（実測）

新しい OUT は `lift/out-seed58/ctl/`。`git archive HEAD`（`92506ae`）を `lift/out-seed58/src/` に展開し、`CARGO_TARGET_DIR=lift/out-seed58/target`（**他の作業の target は使っていない**）で `fe-pe`、`fe-lift`、`fe-decode` を `cargo build --release --offline` した。一般化スクリプトは `lift/out-three/harness-two/` を `lift/out-seed58/harness-two/` にコピーした（`diff -r` で同一）。`TMPDIR=lift/out-seed58/tmp`、`PYTHONDONTWRITEBYTECODE=1`。

| 項目 | 値 | 区分 |
|---|---|---|
| FLEngine の生成 C | SHA-256 `43985d62321a90f562ce1b8144253e00af13df795e4190d26b6b8ce2537137a1`（`gen_two.sh` の検査が一致と表示） | 実測 |
| 入力 DLL | `FLEngine_x64.dll` SHA-256 `b2c64da40921cf262c15c5b25c2b0422e4d1851e953c87e0bc38ed2c19f9496d`（参考）、`dsp_ippv2_x64.dll`（元）`497dd4f7…c352`、`Blend2D.dll`（元）`66634d24…66c7`（どちらも `gen_two.sh` の検査が通った） | 実測 |
| リベース済み dsp_ippv2（0x2000000） | 本セッションで `fe-pe rebase` から作り直した `ctl/dll/dsp_ippv2_x64.2000000.dll` が `lift/out-three/three/dll/` のものと同じ SHA-256 `4fd255027525f2a2e56a93fe1cc7f95442861f788179a23ac8b0401b8f4fefc5`（期待値 `4fd25502…efc5` と一致） | 実測 |
| リベース済み Blend2D（0x4700000） | `ctl/dll/Blend2D.4700000.dll` を `fe-pe rebase` から作り直し。`lift/out-three/three/dll/Blend2D.4700000.dll` と同じ SHA-256 `24e01ec9295870db531e87637aca0aa6cb5c397409346e64a2928d706589b4cb`（期待値と一致。DIR64 1,763、ABSOLUTE 5、ブロック 18） | 実測 |
| dsp_ippv2 の生成 C（縮小。種 452） | 関数 8,030、`bad_nodes 0`、SHA-256 `6035da1de5bde73f4a4c8f0226947622c3e0f51e06c767867eed5ac45e279dbf`（three-module 文書 §3 と同じ） | 実測 |
| Blend2D の生成 C（`--pdata` のみ。種なし） | 関数 2,018、命令 398,879、`bad_nodes 0`、SHA-256 `f7022c97ae4efc914dd4e54f4d4c6adf4a0a6aeaed46030531a4a34ed1eb3a18`（three-module 文書 §4.1 と同じ） | 実測 |
| 表の統合 | 59,557 + 8,290 + 2,278 → 統合 69,605 行、複数の表に在る行 260（擬似 VA）、**`f_<VA>` の衝突 0**。`common.c` SHA-256 `c2f9bbb81a03fab1e0071ce13390e69595f031494815d0e90ded7b91370cd02e`（three-module 文書と同じ） | 実測 |
| 分割 C、表 | `ctl/v-b2d/src/` の 32 個の `.c`（`blend2d-00..02`、`dll-00..10`、`common.c`、`fle-00..16` は `ctl/src/`）が `lift/out-three/three/` の対応ファイルと全部 `cmp` 一致。3 つの `*-tables.txt` も一致。realmod の表は `{ "dsp_ippv2_x64.dll", 0x2000000 }`、`{ "blend2d.dll", 0x4700000 }`、count 2（生成物は `out-three` のものと一致） | 実測 |
| ビルド | `build_two_native.sh`（`JOBS=8`）、コンパイルの警告 0（`log/ctl-build.txt`）。`two-native` 612,184,736 B（`out-three` のビルドは 612,184,672 B。**差 64 B、原因は調べていない**。ビルドパスの文字列の長さの違い（`-g`）かもしれない（推測）） | 実測（サイズ）、推測（差の原因） |

a4-survey（gdb なし、仮の値なし、runtime の応答追加なし。2 回: 07:01:15Z と 07:01:20Z。同じ秒ではない）:

```
stop   : fle: indirect call to 0x48a57f8 is not a known function entry
         top of stack: 0x48b5b90 f_48b5b90
depth  : init_count=150 / 942   last unit: index=149 run_rank=137/895 proc=0x664260 in_progress
         started=137 completed=136 in_progress=1 of 895 runnable   all_units_done=no
         entry check: started_but_not_entered=0 runnable_entered_beyond_init_count=0
funcs  : executed (distinct)=960  entries=26604
```

- **three-module 文書（§5）と一致**: init_count 150/942、last unit 149（proc 0x664260）、functions_distinct 960、function_entries 26,604、停止 `indirect call to 0x48a57f8 is not a known function entry`、top of stack `f_48b5b90`（実測）。
- 2 回の stderr は `cmp` で一致。1 回目の stdout は `lift/out-three/out-three/a4/20261024T063115Z-native.txt` と、`rsp=` の値を除いて `diff` で差なし（実測）。
- **一般化スクリプトのビルド手順は再現できた**（実測）。以降の「種 58 個あり」はこの対照との差分で読む。

## 5. 種 58 個を足したビルド（実測。`lift/out-seed58/s58/`、TAG `b2d58`）

**Blend2D の生成 C だけ**を `fe-lift lift Blend2D.4700000.dll … --pdata --seeds lift/out-seed58/seeds/seeds58-4700000.txt` で作り直した（`gen_two.sh` の `MODULES` の lift-args を `--pdata --seeds <file>` にした）。FLEngine の生成 C、dsp_ippv2 の縮小 C（種 452）、リベース済みの 2 つの DLL は対照と同じものを使った（`ctl/` からコピー。**FLEngine と dsp_ippv2 には 58 個の種を使っていない**）。FLEngine の生成 C は変えていない（SHA-256 `43985d62…37137a1` のまま。`gen_two.sh` が検査）。

| 項目 | 対照（種なし） | **種 58 個あり** | 区分 |
|---|---|---|---|
| Blend2D の関数 | 2,018 | **2,066**（+48） | 実測 |
| 命令（閉包） | 398,879 | **399,508** | 実測 |
| `emitted_instructions` | 398,886 | 399,515 | 実測 |
| `bad_nodes` | 0 | **0** | 実測 |
| `unsupported_total` | 662 | **665** | 実測 |
| `tail_calls` / `gotos` | 152 / 31,879 | 174 / 31,952 | 実測 |
| `register_jmp_sites` | 99 | 100 | 実測 |
| 生成 C | 34,691,060 B、411,794 行、SHA-256 `f7022c97…3a18` | 34,746,495 B、412,711 行、SHA-256 **`3dccba707434cda8bf9fe9e91c7347295f9d0f90e4b388ec82f969cb6aedd5cd`** | 実測 |
| 表の統合 | 69,605 行 | **69,653 行**（Blend2D の表 2,278 → 2,326）。複数の表に在る行 260。**`f_<VA>` の衝突 0** | 実測 |
| `common.c` SHA-256 | `c2f9bbb8…cd02e` | **`41602c7b2bd052d3984b0a510914bd1a37c923a834a58ea8b7edc3a5635a4eb2`** | 実測 |
| `two-native` | 612,184,736 B | **612,248,328 B**（SHA-256 `13e075e6360d6bf905548e93c1e54ea0b39f0681150d13ace26de3cf780a6a7a`）。コンパイルの警告 0（`log/s58-build.txt`）。2 GB 未満 | 実測 |
| realmod の表 | count 2（dsp_ippv2 0x2000000、blend2d 0x4700000） | 同じ | 実測 |

- **調査 §3.2 の「関数 2,066、`bad_nodes 0`、`unsupported_total 665`」と一致**した（命令 399,508 も一致。実測）。
- 58 個の種のうち、対照の dispatch 表に既にあったのは **10 個**、新しく入ったのは **48 個**で、合計 +48 関数（追加された `f_` 行は全部 58 個のどれかで、58 個以外の関数は増えていない。種 58 個の全部が新しい表の `f_` 行にある。実測。python で行を照合）。**停止点だった `f_48a57f8` と `f_48a5830` は新しい表に入った**（`blend2d-tables.txt` の 1750、1752 行目）。
- 58 個の中の 10 個は、58 個を足さなくても `.pdata` からの閉包で生成 C に入っていた（three-module 文書 §5.2 の「reloc-not-function 58 個のうち生成 C の dispatch 表にあるのは 10 個」と一致。実測）。
- `register_jmp_sites` が +1（99 → 100）、`unsupported_total` が +3（662 → 665）。増えた 1 か所の `jmp reg` と 3 つの未対応命令が、新しい 48 関数のどれにあるかは調べていない。

## 6. a4 の行・停止・スタック・import の列（種 58 個あり）

### 6.1 a4 の行（実測。gdb なし、仮の値なし、runtime の応答追加なし。2 回: 07:03:59Z と 07:04:05Z。`lift/out-seed58/out-s58/a4/`）

```
stop   : fle: indirect call to 0x470a8a0 is not a known function entry
         top of stack: 0x4709ea0 f_4709ea0
depth  : init_count=150 / 942   last unit: index=149 run_rank=137/895 proc=0x664260 in_progress
         started=137 completed=136 in_progress=1 of 895 runnable   all_units_done=no
         entry check: started_but_not_entered=0 runnable_entered_beyond_init_count=0
funcs  : executed (distinct)=1058  entries=27316
```

- **2 回の結果は同一**（stdout の差は `calling entry … rsp=` の 1 行だけ、stderr は `cmp` で一致）。
- 対照（種なし）から変わったもの: 停止が `0x48a57f8` から **`0x470a8a0`**（`fle_dispatch_miss`）に移った。top of stack は `f_48b5b90` から **`f_4709ea0`**。functions_distinct 960 → **1,058**（+98）、function_entries 26,604 → **27,316**（+712）。init_count 150/942、last unit 149（proc 0x664260、in_progress）、started 137、completed 136 は同じ（実測）。
- ハーネスの出力: `abort kind: fle_dispatch_miss`、`dispatch target: 0x470a8a0`、`call depth at stop: 19`。

### 6.2 C のスタック（実測。ハーネスが出す先頭 10 行。`call depth at stop: 19`）

```
#0 0x4709ea0 f_4709ea0 guest-return=0x470acab
#1 0x470abb0 f_470abb0 guest-return=0x4899dd3
#2 0x4899d90 f_4899d90 guest-return=0x4701009
#3 0x4701000 f_4701000 guest-return=0x48a59eb
#4 0x48b5b90 f_48b5b90 guest-return=0x48a59eb
#5 0x48a59a8 f_48a59a8 guest-return=0x489e34e
#6 0x489e2b4 f_489e2b4 guest-return=0x489e4c4
#7 0x489e264 f_489e264 guest-return=0x489e4c4
#8 0x489e450 f_489e450 guest-return=0xcb000000
#9 0x489e584 f_489e584 guest-return=0xcb000000
```

- `f_489e584`（Blend2D のエントリ = `_DllMainCRTStartup`）の戻り先は `0xcb000000`（`fle_call_guest` の番兵）で、スタックに残っている。つまり **Blend2D の DllMain(ATTACH) には入った。戻っていない**（実測。スタックに `f_489e584` が残ったまま停止）。
- 対照の停止（`f_48ab984` → `f_48b5b90` → `0x48a57f8`）と比べると、初期化表のループ `f_48ab984` ではなく、**`f_48a59a8`**（`0x48a59a8`）が CFG の踏み台 `f_48b5b90` 経由で `f_4701000` を呼ぶ形に変わっている。ループが先頭エントリ（`0x48a57f8`）を通り過ぎて、別の呼び出しに進んだと読める（読み。§6.5 で確認した）。
- ハーネスは先頭 10 行しか出さない（深さ 19）。全体は読み取りだけの gdb の `bt`（§6.3）で取った（実測）。

### 6.3 読み取りだけの gdb（実測。`lift/out-seed58/run/`、`lift/out-seed58/gdb/`。値を書いていない）

`lift/out-three/gdb/ro_imports3.py` を `gdb/ro_imports_s58.py` にコピーして使った（変更なし。`cmp` 一致。`return`、`set var`、`call` は無い。`gdb.execute` は `set pagination`、`set confirm`、`set print`、`file`、`set args`、`info functions`、`run`、`bt 30`、停止時の `signal SIGABRT` だけ）。環境変数は three-module 文書と同じ（`GDBI_BIN`、`GDBI_REAL=<dsp>:<blend2d>`、`GDBI_FLE`、`GDBI_OUT`（存在しないこと）、`GDBI_STDOUT`、`GDBI_STDERR`）。さらに同じ型の読み取り専用スクリプトを 3 つ書いた（`gdb/ro_entries_s58.py`、`gdb/ro_tls_s58.py`、`gdb/ro_seedhits_s58.py`。どれも breakpoint で値を読んで記録するだけで、ゲストにも `R[]` にもプロセスにも書かない）。各スクリプトは 2 回ずつ実行し、ログは `cmp` で全バイト一致した（`ro1`=`ro2`、`ro3`=`ro4`、`en1`=`en2`、`tl1`=`tl2`、`sh1`=`sh2`）。

**注意（自分の誤りの訂正）**: `ro_imports3.py` の `LoadRet("fle_load.c:153")` は、three-module 文書の時点（`2e94df2`）の行番号で、HEAD（`92506ae`）の `fle_load.c` は 2 行ずれており、153 行は `if (rm >= 0) {` に当たる（154 行が `R[0] = fle_realmod_load(api, rm);`、155 行が `return;`）。`ro1`、`ro2` はこのずれたままの行で実行したので、`LOADLIBRARY-RETURN` の行は実在モジュール以外の LoadLibrary の戻りも拾った（意味のない値が出る）。そこで **`LoadRet` を 155 行にしたコピー `gdb/ro_imports_s58b.py`（差は 1 行）で `ro3`、`ro4` を取り直した**。`ro1` と `ro3` は `LOADLIBRARY-RETURN` の行を除いて全行同一（`diff`）。以下の (c) は `ro3`、`ro4` の値。

**(b) 停止（gdb の `bt`。`ro1-imports.txt`。`ro3` も同じ）。** 停止時のゲストのレジスタ: `rcx=0x470a8a0 rdx=0x0 r8=0x8000001f r9=0x80000000 rsp=0x7ffff7bff810 rax=0x0`。C のスタック（全体。上が内側）:

```
fle_dispatch_miss(va=74492064 = 0x470a8a0)   fle_dispatch.h:22
fle_dispatch(va=0x470a8a0)                    fle_dispatch.h:44
f_4709ea0   blend2d-00.c:12454   (jmp rcx)
f_470abb0   blend2d-00.c:12547   (call 0x4709ea0)
f_4899d90   blend2d-01.c:254889
f_4701000   blend2d-00.c:2342    (.xc の初期化子。orig 0x180001000)
fle_dispatch(va=74452992 = 0x4701000)
f_48b5b90   blend2d-02.c:27288   (CFG の踏み台 jmp rax)
f_48a59a8   blend2d-02.c:9759    (8 バイト刻みの表を走るループ。_initterm に当たる形)
f_489e2b4 ← f_489e264 ← f_489e450 ← f_489e584 (Blend2D のエントリ)
fle_dispatch(va=0x489e584) ← fle_call_guest ← call_dllmain(idx=1, reason=1) fle_realmod.c:117
← fle_realmod_load(idx=1) :138 ← load_by_name fle_load.c:154 ← fle_imp_LoadLibraryW :186
← f_423990 ← f_44db40 ← f_6539e0 ← f_664190 ← f_664260 ← fle_dispatch(6701664 = 0x664260)
```

**(c) Blend2D の DllMain(ATTACH) は戻ったか。LoadLibraryW は 0x4700000 を返したか。**

| 項目 | 値 | 区分 |
|---|---|---|
| `call_dllmain(idx=1, reason=1)` | 入った（`rcx=0x4700000 rdx=1 r8=0`。それまでに 1,525 回の import） | 実測 |
| 戻りと eax | **戻っていない**（`DLLMAIN-RETURN … out of scope (did not return normally)`。スタックに `f_489e584` が残る）。**eax は得られていない** | 実測 |
| `fle_realmod.c:144`（`ST_LOADED`） | Blend2D では**到達していない**（到達の記録は dsp_ippv2 の 1 回だけ。`call_dllmain(idx=0)` は 0x1 を返した） | 実測 |
| **LoadLibraryW が 0x4700000 を返したか** | **返していない**。`LOADLIBRARY-RETURN`（155 行の `return;`。`ro3`、`ro4`）は 1 回だけで、`R[0]=0x2000000`（dsp_ippv2）。`0x4700000` を返した記録はない。FLEngine の `f_664190` はハンドルを受け取っていない | 実測 |

**(d) Blend2D の ATTACH の中で呼ばれた import（通し番号 1526〜1697、172 回。`ro1-imports.txt`、`ro3-imports.txt`）。** 対照では 13 回で止まっていたのが、172 回に増えた。呼び出しの順（連続は `x回`）:

```
GetSystemTimeAsFileTime, QueryPerformanceCounter, GetCurrentThreadId, GetCurrentProcessId, QueryPerformanceCounter   (#1526-1530。__security_init_cookie)
LoadLibraryExW("api-ms-win-core-synch-l1-2-0")→0x7ff8001b0000, GetProcAddress("InitializeCriticalSectionEx")→0,
InitializeCriticalSectionAndSpinCount, LoadLibraryExW("api-ms-win-core-fibers-l1-1-1")→0x7ff8001c0000, GetProcAddress("FlsAlloc")→0,
TlsAlloc, GetProcAddress("FlsSetValue")→0, TlsSetValue(2, 0x48f1590)                                              (#1531-1538)
LoadLibraryExW(synch)→0x7ff8001b0000, GetProcAddress("InitializeCriticalSectionEx")→0, InitializeCriticalSectionAndSpinCount x14 (#1541-1554), GetProcessHeap,
LoadLibraryExW(fibers), GetProcAddress("FlsAlloc")→0, TlsAlloc, GetLastError, GetProcAddress("FlsGetValue")→0, TlsGetValue(3),
GetProcAddress("FlsSetValue")→0, TlsSetValue(3, -1), HeapAlloc, TlsSetValue(3, <ptr>), Enter/LeaveCriticalSection, SetLastError,
EnterCriticalSection x2, HeapAlloc, InitializeCriticalSectionAndSpinCount x64 (#1574-1637), LeaveCriticalSection             (#1539-1638)
GetStartupInfoW, GetStdHandle x3, LeaveCriticalSection, GetCommandLineA, GetCommandLineW, TlsGetValue(3), Enter/LeaveCriticalSection (#1639-1648)
GetACP, HeapAlloc, IsValidCodePage(0x4e4), GetCPInfo x2, MultiByteToWideChar x2, GetStringTypeW, MultiByteToWideChar x2,
LoadLibraryExW("api-ms-win-core-localization-l1-2-1")→0x7ff8001d0000, GetProcAddress("LCMapStringEx")→0, GetProcAddress("LocaleNameToLCID")→0,
LCMapStringW x2, WideCharToMultiByte, MultiByteToWideChar x2, LCMapStringW x2, WideCharToMultiByte                                (#1649-1669)
Enter/LeaveCriticalSection, InitializeSListHead, InitializeCriticalSectionAndSpinCount,
GetModuleHandleW(synch)→0x7ff8001b0000, GetProcAddress("SleepConditionVariableCS")→0, GetProcAddress("WakeAllConditionVariable")→0, CreateEventW (#1670-1677)
Enter/LeaveCriticalSection, HeapAlloc x2, InitializeCriticalSectionAndSpinCount x3, GetModuleFileNameW(NULL, …),
LoadLibraryExW("kernel32")→0x7ff800090000, GetProcAddress("AreFileApisANSI")→0, WideCharToMultiByte x2, HeapAlloc,
GetEnvironmentStringsW, WideCharToMultiByte, HeapAlloc, WideCharToMultiByte, FreeEnvironmentStringsW, HeapAlloc, HeapFree         (#1678-1697)
```

- 名前ごとの回数（172 回）: `InitializeCriticalSectionAndSpinCount` 83、`GetProcAddress` 12、`HeapAlloc` 8、`EnterCriticalSection` 7、`LeaveCriticalSection` 7、`WideCharToMultiByte` 6、`MultiByteToWideChar` 6、`LoadLibraryExW` 6、`LCMapStringW` 4、`TlsSetValue` 3、`GetStdHandle` 3、`TlsGetValue` 2、`TlsAlloc` 2、`QueryPerformanceCounter` 2、`GetCPInfo` 2、ほか 1 回ずつ（`SetLastError`、`IsValidCodePage`、`InitializeSListHead`、`HeapFree`、`GetSystemTimeAsFileTime`、`GetStringTypeW`、`GetStartupInfoW`、`GetProcessHeap`、`GetModuleHandleW`、`GetModuleFileNameW`、`GetLastError`、`GetEnvironmentStringsW`、`GetCurrentThreadId`、`GetCurrentProcessId`、`GetCommandLineW`、`GetCommandLineA`、`GetACP`、`FreeEnvironmentStringsW`、`CreateEventW`）（実測）。
- **172 回のどれも名前付き停止になっていない**（実測）。`GetProcAddress` の戻りは 12 回とも 0（api-ms の仮想モジュールと `kernel32` への `GetProcAddress` は、決定 0081 のとおり NULL。CRT は FLS を使わず `TlsAlloc`、`CreateEventW` の代替に落ちる形に見える（読み））。
- **行なし 15 名の import は 1 回も実行されていない**。根拠は 2 つ: (1) 行なしの import は呼ばれると名前付き停止（`fle_unsupported`）になるが、停止は API 名を持たない `fle_dispatch_miss` である。(2) ATTACH の区間のログに `GetModuleHandleA`、SRWLock 系、条件変数、`CreateThread`、`ExitThread`、`TlsFree`、`GetFileSizeEx`、`WaitForSingleObject*`、`IsProcessorFeaturePresent`、`OutputDebugStringA`、`GetLargePageMinimum`、`VirtualAlloc` の `fle_imp_` は 0 件（`ro3-imports.txt` を `grep`）。どちらも実測。`GetProcAddress` の名前に SRWLock などが出る行は、ATTACH より前の FLEngine 側の呼び出し（avrt、kernel32 の SRWLock の取得。通し番号 1454〜1491）。
- `GetModuleHandleW` は 1 回（`api-ms-win-core-synch-l1-2-0` → `0x7ff8001b0000`）で、`GetModuleHandleA` ではない（実測）。

**(e) unit 149 の後で、ゲストは `GetProcAddress("AggCreateInstance")` を呼んだか。** **呼んでいない**。ATTACH が戻っていないので `f_664190` の `GetProcAddress` の行に届いていない（import のログに `AggCreateInstance` は 0 件。実測）。

### 6.4 停止した値 0x470a8a0 の性質（実測と読み）

| 項目 | 値 | 区分 |
|---|---|---|
| 元の VA | 0x470a8a0 − 0x4700000 + 0x180000000 = **0x18000a8a0** | 導出 |
| 種 58 個にあるか | **無い**（`seeds58-4700000.txt`、調査の `reloc-not-function.txt`（元の VA）、`reloc-seeds.txt` のどれにも無い） | 実測 |
| `.pdata` の関数開始か | **いいえ**（リベース済み DLL の `fe-pe pe-pdata-seeds` 1,843 件に無い）。**`.pdata` の RUNTIME_FUNCTION `[0x4709ea0, 0x470abac)` の中**（`f_4709ea0` の本体の途中。pefile で確認） | 実測 |
| 生成 C の dispatch 表にあるか | 無い（対照にも、種 58 個ありにも無い） | 実測 |
| 呼び出し元 | **`f_4709ea0` の `0x470a89e: jmp rcx`**（生成 C は `fle_dispatch(R[1]); return;`。`blend2d-00.c:12454`）。直前の命令は `movzx eax, byte [rax+r13+0xab8c]; mov ecx, [r13+rax*4+0xab74]; add rcx, r13`（`r13` = 0x4700000）で、**MSVC の switch 表の形**（索引のバイト表 `0x470ab8c`（32 バイト）と 32 ビット RVA の表 `0x470ab74`（6 エントリ: `0x470a8a0`、`0x470a8af`、`0x470aa40`、`0x470aa6b`、`0x470aab1`、`0x470aafc`）。索引 = `r9d − 0x80000000`、32 を超えると `ja 0x470aafc`） | 実測（バイト列と表の中身）、読み（switch 表と読むこと） |
| 状況 | 停止時 `r9 = 0x80000000`、`r8 = 0x8000001f`、`rcx = 0x470a8a0` = 表の 0 番（索引バイト 0 → 表の 0 番）。この関数は `cpuid` を 8 か所に持つ AsmJit の CPU 情報の関数（`f_180009ea0` に当たる。調査 §3.5、§5.2 の読み）で、拡張葉 `0x80000000` 以降を数える形（読み） | 実測（レジスタ）、読み（関数の役割） |
| 調査との対応 | 調査の `lift/out-blend2d/jmpreg.txt` が `0x18000a89e rva-switch 6 6 0`（6 エントリ、distinct 6、`.pdata` 開始 0）と記録した 1 か所。**調査 §3.4 / §10.3 の B3**（`jmp reg` の switch 表。移動先がブロックの途中で、生成 C に入らない）が予想した形 | 実測（`jmpreg.txt` の行）、読み（対応） |
| 生成 C の中 | この 6 つの移動先のうち、`0x470a8a0`、`0x470a8af`、`0x470aa40`、`0x470aa6b`、`0x470aab1` の 5 つは、生成 C に `L_<VA>:` のラベルが**無い**（`grep`。各 0 件）。`0x470aafc`（`ja` の直接の移動先でもある）は有る | 実測 |

**種 58 個とは別の種類の停止**である。停止した値は「`.pdata` に無い葉関数への間接呼び」ではなく、**関数の途中にある switch の分岐先**（B3）。依頼の手順 6 の規則に従い、**この値は追加の種に足していない**（足して先へ進めていない。測定は 58 個の 1 回だけ）。

### 6.5 種 58 個が実行されたか、ATTACH の初期化がどこまで進んだか（実測。`run/en1-entries.txt`、`run/sh1-seedhits.txt`）

- 停止の前に実行された種 58 個のうちの関数は **2 つだけ**: `f_48a57f8`（1 回）と `f_48a5830`（2 回）。残りの 56 個（Agg2D の vtable の 34 個を含む）は、停止までに 1 回も入られていない（実測。`f_<VA>` の入口に breakpoint を 58 個置いて数えた）。
- 対照で止まった、CFG の踏み台経由の 16 バイト刻みの表のループ `f_48ab984`（`rcx=0x48b8a40`、`rdx=0x48b8b40`）は、**先頭が非 0 の 12 エントリの入口にすべて入り、ループ自体が `al=1` で戻った**（入った順: `f_48a57f8`、`f_48a5844`、`f_48a9cf4`、`f_48a5830`、`f_48a9ef0`、`f_48ab95c`、`f_48a6cb0`、`f_48a7ef0`、`f_48ab68c`、`f_48ab2c8`、`f_48a5830`（2 回目）、`f_48a580c`。表の並びと同じ順。実測）。各エントリの戻り値は個別には測っていない。three-module 文書 §5.2 の読み（`al` が 0 ならループを抜ける）と合わせると、12 個とも非 0 の `al` を返したと読める（読み）。three-module 文書 §5.2 の「ループを抜けた後の残りのエントリがどうなるかは未測定」に対する測定。
- ループの後、`.xi` の 4 項目（`f_489db00`、`f_48a4400`、`f_48af6e4`、`f_48b0e00`。調査 §6 の `.xi` 4 項目の元の VA 0x18019db00、0x1801a4400、0x1801af6e4、0x1801b0e00 に当たる）が **すべて入られ、すべて戻り値 0 で戻った**（`RETURN … R[0]=0x0`）。そのあと `f_48a59a8`（`.xc` の範囲 `0x48b7320`〜`0x48b7330` を走るループ）が `f_4701000`（`.xc` の初期化子。調査 §6 の `0x180001000`）を CFG の踏み台経由で呼び、`f_4701000` → `f_4899d90`（調査 §5.2 の `f_180199d90`）→ `f_470abb0`（CPU 情報の初期化。調査 §5.2 の `f_18000abb0`）→ `f_4709ea0` に入って、`jmp rcx` で止まった。**`.xc` の初期化子に入ったのはこれが初めて**（対照では `.xi` より前で止まっていた）。
- `f_4701000` は入られたが戻っていない（`out of scope`）。調査 §5.2 の読みでは、`f_180199d90` は CPU 情報の初期化のあとで `GetSystemInfo`、`GetModuleHandleA(…)`（行なし）を呼ぶ順なので、**停止は `GetModuleHandleA` より前**（読み。順序は調査の読み。実測では `GetModuleHandleA` は呼ばれていない）。

### 6.6 暗黙の TLS の読みは停止の前に実行された（実測。`run/tl1-tls.txt`）

停止の原因ではないが、停止の途中で `f_470abb0`（`mov rax, gs:[0x58]; mov ecx,[_tls_index]; mov rcx,[rax+rcx*8]; mov eax,[rdx+rcx]` の形。`rdx` = 8。調査 §6 の形）が実行された。停止時に読み取りだけで確かめた値:

| 項目 | 値 |
|---|---|
| ゲストの `_tls_index` のスロット（`0x48f1560`。元の `0x1801f1560`） | **0**（ファイル上も 0。誰も書いていない） |
| `fle_tls_values[0]` | ホストのヒープのポインタ（プロセスで最初の `TlsAlloc` が 0 を返し、続く `TlsSetValue(0, …)` が入れた値。ATTACH より前の呼び出しで、FLEngine のものと読む）。停止までの `TlsGetValue` 73 回のうち 71 回が index 0（実測） |
| `[fle_tls_values[0] + 8]`（`mov eax,[rdx+rcx]` が読む 4 バイト） | **0**（停止時点の値） |
| `cmp [0x49f4b88], eax` の左 | **0**。したがって `jle 0x470ac5e` が成り立ち、`__Init_thread_header` に当たる呼び出し `f_489dc58` へは進まない（`f_489dc58` に breakpoint を置いた 1 回の実行 `run/it1-initthread.txt` で、入られていないことを確認。実測。同じ実行に置いたもう 1 つの `f_489dcb4` は私の推測で置いた名前で、生成 C に関数として無いので、結果に使わない） |
| Blend2D の `TlsAlloc` | 2 回（戻り 2 と 3）。`TlsSetValue(2, 0x48f1590)`、`TlsSetValue(3, …)`。プロセス全体の `TlsAlloc` は 4 回（戻り 0、1、2、3）で、0 は FLEngine、1 は dsp_ippv2 の DllMain の中の呼び出しと読む |

つまり**暗黙の TLS の読みは、スロット 0 に FLEngine の値が入っていたため、クラッシュせず 0 を読んで通った**（実測）。暗黙の TLS をモデル化したから通ったのではない（`_tls_index` を書く処理は runtime に無い。読み。調査 §6）。偶然通った（推測）。調査 B6 は**未解決のまま**で、この通過を「TLS が動いた」とは言えない。

## 7. 停止点の分類と、次に必要な作業（事実）

### 7.1 分類（three-module 文書 §6.1 と同じ (i)〜(v)）

停止は `fle_dispatch_miss`（`indirect call to 0x470a8a0 is not a known function entry`）。

| 分類 | 該当するか | 根拠 | 区分 |
|---|---|---|---|
| (i) 未応答の import（行なし 15 名のどれか） | **いいえ** | 停止メッセージは API 名を持たない。ATTACH の 172 回の import に名前付き停止は無い。行なし 15 名は 1 回も実行されていない（§6.3 (d)） | 実測 |
| (ii) TLS（B6。`_tls_index`、`gs:[0x58]`） | **いいえ（停止の原因ではない）。ただし踏んだ** | 停止の途中で `f_470abb0` が `gs:[0x58]` と `_tls_index`（0）を使う読みを**実行した**が、`fle_tls_values[0]` に別のポインタが入っていて、読んだ値が 0 になり、落ちずに進んだ。停止の `jmp rcx` は TLS の読みの後（`f_470abb0` から `f_4709ea0` を呼んだ先）にある。B6 は未解決で、通ったのは偶然と読む（§6.6） | 実測、推測 |
| (iii) JIT（`VirtualAlloc` 0x20003000、実行時に作られた機械語への間接呼び） | **いいえ** | 対象 `0x470a8a0` はリベース済み PE の `.text` の中（`.pdata` の `[0x4709ea0, 0x470abac)` の内側）にある静的なコード。`VirtualAlloc`、`GetLargePageMinimum` は ATTACH で 1 回も呼ばれていない | 実測 |
| (iv) ロケール（範囲 D の外） | **いいえ（範囲 D の検査による名前付き停止は出なかった）** | Blend2D の ATTACH の中で `GetStringTypeW` 1 回、`MultiByteToWideChar` 6 回、`LCMapStringW` 4 回、`WideCharToMultiByte` 6 回、`GetCPInfo` 2 回、`IsValidCodePage` 1 回が実行され、どれも名前付き停止にならずに戻った（cp 1252 = 0x4e4）。**戻した値が実機と一致するかは確認していない**（決定 0087 は仮の決定で、実機で未確認） | 実測（呼び出しと停止の有無） |
| **(v) その他** | **これ**: 生成 C に無い**関数の途中の分岐先**（`jmp rcx` の switch 表。調査 §3.4、§10.3 の **B3**） | §6.4: `f_4709ea0` の `0x470a89e: jmp rcx`。`MSVC の switch 表の形`（索引バイト表 `0x470ab8c` + 32 ビット RVA 表 `0x470ab74`、6 エントリ）。移動先 6 つのうち 5 つは生成 C に `L_<VA>:` のラベルが無い。`fe-lift` の `jumptabs` は 0（Blend2D の `jmp reg` の `register_jmp_sites` は 100）。**種 58 個の問題（B5）ではなく、B3** | 実測 |

### 7.2 次に必要な作業（事実の並べ。決めない）

1. **種 58 個を足すことで、B5 が原因の停止（CRT の表の先頭 `0x48a57f8`）は解消し、停止は B3（switch 表）に移った。** 足したのは Blend2D の生成 C だけで、FLEngine の生成 C は変えていない（SHA-256 `43985d62…37137a1`）。CRT の初期化表の 12 エントリのうち 3 個が生成 C に無かった問題は、58 個の種で解消した（12 エントリすべての入口に入れた。§6.5）。
2. **今回の停止は 58 個の種では直らない種類の停止**（B3）。停止した値 `0x470a8a0` は 58 個にも `.pdata` の関数開始にも無く、`.pdata` の 1 つの RUNTIME_FUNCTION の内側のブロックの先頭（switch の分岐先）。調査の `jmpreg.txt` では、こうした `rva-switch` が **95 か所**（distinct な移動先 1,116、うち `.pdata` 開始は 1）あり、今回止まったのはその 1 つ（`0x18000a89e`。6 エントリ、distinct 6、`.pdata` 開始 0）。残りの 94 か所が ATTACH の途中や `AggCreateInstance` 以降で実行されるかは**未測定**。
3. **生成器の現状（読み。ソースの該当部分を読んだ）**: `lift/crates/fe-decode/src/jumptab.rs` の jump table の解決は 2 通りだけ — 手書きの `Spec`（決定 0036。FLEngine の 4 か所）と、選択子スタブ `movsxd; lea; jmp [B+I*8]`（決定 0079）。今回の `movzx r, byte [..]; mov r, dword [..]; add r, base; jmp r`（バイト索引 + RVA 表）の形を認識する箇所は、読んだ範囲には無い（`jumptabs 0`。実測）。`jmp reg` は `fle_dispatch(R[x])` になり、移動先がブロックの途中だと `fle_dispatch_miss`（実測。今回の停止）。
4. **停止の先で、調査が挙げた項目はまだ測れていない**: `.xc` の初期化子 `f_4899d90`（調査の `f_180199d90`）は CPU 情報の関数の後で `GetSystemInfo`、`GetModuleHandleA`（行なし）を呼ぶ順に読める（調査 §5.2 の読み。実測では `GetModuleHandleA` は未呼出）。したがって `GetModuleHandleA`（B8）、`CreateThread`（B4）、JIT（B1）、CPUID の分岐（B11）、`.xc` の完走、`AggCreateInstance` の引き（`f_664190`）、unit 149 の後の FLEngine の経路は、**どれも停止より先で、未測定**。unit 158（QFC の LoadLibraryW）には届いていない。
5. **暗黙の TLS（B6）は停止の前に踏まれた**（§6.6）。スロット 0 に別の値が入っていたため通ったが、`_tls_index` を書く処理は runtime に無い（読み。調査 §6）。停止が先へ動くと、`gs:[0x58]` の読みは 26 か所（生成 C の `fle_gs_ld8` のうち）あるので、通る・通らないが偶然に左右されうる（推測）。
6. **今回の構成（基底 0x4700000、dsp は縮小、Blend2D は `--pdata` + 58 個）で、機構は動いた**: 3 つの像、`fle_realmod` の表（count 2）、統合した 69,653 行の表、172 回の import（行なし 15 名なし、名前付き停止なし）、CRT の初期化の表、`.xi` 4 項目、`.xc` の初期化子への進入。
7. この測定の値（基底 0x4700000、58 個の種、縮小の種 452、`--pdata`）はすべて**測定用**で決定ではない。種の定義（B5）、`--reloc-seeds`、58 個を足すか、B3 をどうするかは別の判断で、この文書は決めない。

## 8. 限界、未確認

- **決定 0074 との関係**: 0074 は「`.pdata` の関数開始と一致する reloc 値だけを seed にする。開始でない値は種にしない」と決めた。今回の 58 個は、0074 が種にしないとした値（実行範囲内の DIR64 値のうち `.pdata` の関数開始に当たらないもの）である。**この測定は 0074 の方針を変える決定ではなく**、停止がどこへ移るかを見るための測定。58 個が**入口として正しいか**の確認はしていない（調査 §10.4 の「58 個が全部関数の入口であること」は未確認のまま。今回の実行で入られた 2 個 `0x48a57f8`、`0x48a5830` は、CRT の初期化表のエントリとして呼ばれ、ループは 12 エントリを通って戻った。これは**入口として使われたこと**の観測で、関数の意味の確認ではない）。残り 56 個は実行されていない。
- **基底 0x4700000、縮小（dsp の種 452）、`--pdata`（Blend2D）は測定用の値と構成で、決定ではない。**決定 0086 は方向だけを決めた。別の基底、別の種の定義にすれば停止点は変わりうる。**種 58 個も決定ではない。**
- **58 個を FLEngine と dsp_ippv2 には使っていない**（Blend2D の生成 C だけ。`docs/investigations/dll-nonpdata-reloc-seeds.md` が記録した「FLEngine に pdata に入らない reloc 値を無条件に種にすると `bad_nodes` が 0 → 113,485」の話は、今回の構成には当てはまらない。今回の Blend2D は `bad_nodes 0` のまま）。
- **新しい停止は直していない**（範囲外）。停止は `fle_dispatch_miss`（`0x470a8a0`）。**追加で種に足して先へ進めていない**。測定は 58 個の 1 回だけ。
- **動作の意味は検証していない**。172 回の import の戻り値が実機と一致するかは確認していない（ホストは Linux。実機 Windows での挙動は未確認）。`GetProcAddress` が api-ms の仮想モジュールや `kernel32` に対して NULL を返す挙動は決定 0081 に従うもので、実機の挙動と一致するかは未確認。ロケール関数の戻り値（決定 0087 は仮の決定）も同様。
- **停止の読みの限界**: `0x470a89e` の `jmp rcx` を MSVC の switch 表と読んだのは、バイト列（索引バイト表と 32 ビット RVA 表）と調査の `jmpreg.txt` の分類による。switch の索引の範囲（`ja 0x470aafc` の前の `cmp eax, 0x1f`）と表の 6 エントリの意味（拡張 CPUID 葉の分岐と読めること）は、逆アセンブルの 20 命令ほどを根拠にした読みで、AsmJit のソースとの照合はしていない。
- **`.xi` の戻り値 0** は `R[0]` と `al` の値を読み取っただけ。`_initterm_e` の「0 = 成功」の意味づけは CRT の慣例による読み。12 個の初期化エントリの各戻り値は個別には測っておらず、ループが `al=1` で戻ったことだけを測った。
- **2 回の gdb の run のうち `ro1`、`ro2` は `LoadRet` の行番号が 2 行ずれていた**（§6.3）。数値の報告には `ro3`、`ro4`（155 行）を使った。その他のスクリプト（`ro_entries_s58.py`、`ro_tls_s58.py`、`ro_seedhits_s58.py`）は `LoadRet` を持たない。`it1`（`f_489dc58` と推測で置いた `f_489dcb4`）の実行は 1 回だけで、`f_489dcb4` は生成 C に関数として無かったため、この実行は `f_489dc58` が入られていないことの確認にだけ使った。
- **ビルドの再現性**: 対照の `two-native` は `out-three` のものと 64 B 違う（612,184,736 B と 612,184,672 B。原因は調べていない）。生成 C、分割 C、表、`common.c` は全部一致した。
- 同じ構成での run は、gdb なしが対照 2 回 + 種 58 個あり 2 回、gdb 読み取りが ro 4 回 + en 2 回 + tl 2 回 + sh 2 回 + it 1 回。それぞれの 2 回は同じ結果。別のホストや別のカーネルでの再現は未確認。ASLR のため `rsp=` と一部のポインタ値は run ごとに変わる（比較から除いた）。
- wasm は使っていない（`emcc` を使っていない。native のみ）。wasm の locals の上限（Blend2D は調査で 4 関数が 50,000 を超える近似値。種 58 個ありの locals は測っていない。調査 §3.6 は 58 個を足した版の最大を 15,618 と記録している）、`FLE_CHECKED_MEM` への影響は、この作業で見ていない。
- 調査 §6 は `gs:[0x58]` の読みを「13 関数 29 か所」と書くが、生成 C では `fle_gs_ld8` 29 か所のうち `gs:[0x58]` は 26 か所（three-module 文書 §6.1 (ii) が記録した不一致。この作業で、種 58 個ありの生成 C と対照の生成 C のどちらも `fle_gs_ld8` 29、うち `gs:[0x58]` 26 であることを `grep -o` で再確認した（実測）。不一致の理由は調べていない）。

### 8.1 作ったファイル（`lift/out-seed58/`。gitignore 対象の `lift/out*/`）

- `src/`（`git archive HEAD`）、`target/`（fe-pe、fe-lift、fe-decode の release）、`harness-two/`（`lift/out-three/harness-two/` のコピー。変更なし）。
- `seeds/`: `reloc-not-function.regen.txt`（58 個。元の VA）、`seeds58-4700000.txt`（変換後。`fe-lift --seeds` に渡したファイル）、`seeds58.bash.txt`、`not-function.on-rebased.txt`（検算用）、`reloc-seeds.regen.txt`（519 個）、`probe-init-thread.txt`（`it1` 用）。
- `ctl/`（対照。種 58 個なし。`dll/`、`gen/`、`src/`、`v-b2d/`（`native/two-native` を含む））、`out-ctl/a4/`（対照の a4-survey のログ 2 回）。
- `s58/`（種 58 個あり。`dll/`、`gen/`、`src/`、`v-b2d58/`（`gen/blend2d.c`、`src/`、`log/`、`native/two-native`）、`native/`（`ctl/` からコピーした共有オブジェクト））、`out-s58/a4/`（a4-survey のログ 2 回）。
- `gdb/`（`ro_imports_s58.py`（`ro_imports3.py` の同一コピー）、`ro_imports_s58b.py`（`LoadRet` の行だけ 155）、`ro_entries_s58.py`、`ro_tls_s58.py`、`ro_seedhits_s58.py`。すべて読み取り専用）、`run/`（gdb のログ）、`analysis/dis_b2d.py`（`out-three` のコピー）、`log/`（生成、ビルド、a4-survey の標準出力）、`tmp/`（`TMPDIR`）。
- **削除はしていない**（`rm` なし）。2 GB 級のファイルなし（最大は `two-native` の約 612 MB。2 つ）。
