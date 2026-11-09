# 多モジュール構成の測定ハーネス（`lift/harness/a4-multi.sh`）: 実在モジュールの一覧を与えて、ビルドして a4 の行を出す（取り込み案。何も決めない）

実施日: 2026-10-27（JST。ログの時刻は 2026-10-26T20:0x〜22:xxZ。調査とスクリプトの編集は 2026-10-25 の夜（JST）に始め、ネットワーク障害で中断したあと再開した）。元にした木: `git archive HEAD`（作業開始時の HEAD は `dc27f4d`、再開時は `5002d98`。`git diff --stat dc27f4d 5002d98 -- lift` は空で、差は `docs/investigations/a5-oracle-rerun-3.md` の 1 ファイルだけ。実測）。作業物は `lift/out-harness-int/`（gitignore 対象）。HEAD の作業ツリーには書いていない。

**この文書は何も決めない。** 基底（dsp_ippv2 0x2000000、Blend2D 0x4700000）と種（452 個、58 個）は、過去の測定（`two-module-after-0088.md`、`blend2d-seed58.md`）で使った**測定用の値**で、決定ではない。スクリプトに既定値として埋めていない（引数で与える）。

## 0. 凡例

- **実測** = このセッションでコマンドを実行して得た値。ログは `lift/out-harness-int/log/` と `run-*/runs/`。
- **読み** = ソースやスクリプトを読んだ結果。実行して確かめていない。
- **未確認** = そのとおり。

## 1. 要約

| # | 項目 | 結果 | 区分 |
|---|---|---|---|
| 1 | 入口 | `OUT=<絶対パス> FLE=<FLEngine_x64.dll> bash lift/harness/a4-multi.sh [名前:DLL:基底[:種[:pdata]] ...]`。0 個 = 単独、1 個 = 2 モジュール、2 個 = 3 モジュール（4 個まで runtime が受ける。§5） | 実測（0・1・2・4・5 個を実行） |
| 2 | (a) 単独（0 個） | init_count **145 / 942**、last unit 144、functions **821**、entries **15,685**、停止 `LoadLibraryW "dsp_ippv2_x64.dll"`。**変わらない**。`a4-survey.sh` を直接 2 回走らせた結果も同じ。既存の出力（`lift/out-progress/a4build/a4/20261025T113046Z-native.txt`、281 行）と `calling entry … rsp=` の 1 行を除いて**全行一致** | 実測 |
| 3 | (b) 2 モジュール（dsp:0x2000000、種 452） | **150 / 942**、last unit **149**、functions **943**、entries **26,585**、停止 `LoadLibraryW "blend2d.dll"`。2 回同一 | 実測 |
| 4 | (c) 3 モジュール（+ Blend2D:0x4700000、種 58、pdata） | **150 / 942**、functions **1,069**、entries **27,333**、停止 `fle: indirect call to 0x489f940 is not a known function entry`。HEAD の基準と一致。2 回同一 | 実測 |
| 5 | 上限と基底の検査 | 4 個は通常どおり進む。5 個と、基底 0x7FF800000000 は runtime（`fle_realmod.c`）が名前付きで停止し、a4-multi.sh はその停止メッセージを出しただけ。スクリプトに独自の検証は足していない | 実測 |
| 6 | 変更したファイル | 新規 2（`a4-multi.sh`、`two/merge_tables_n.py`）、変更 4（`two/gen_two.sh`、`two/build_two_native.sh`、`two/main_two.c`、`two/two-native-run.sh`）。`a4-survey.sh`、`a4.c`、`a4.h`、`image.c`、`trace_native.c`、`build_native.sh`、`runtime/`、`crates/`、`docs/` は HEAD と同一（`cmp`、`diff -r`） | 実測 |
| 7 | テスト | §7 | 実測 |

## 2. 使い方

```
OUT=/abs/dir FLE=/abs/path/FLEngine_x64.dll bash lift/harness/a4-multi.sh [NAME:DLL:BASE[:SEEDS[:OPTS]] ...]
```

| 引数 | 意味 |
|---|---|
| `NAME` | ゲストの `LoadLibraryW` が `normalize()` のあとに作る綴り（小文字。例 `blend2d.dll`）。`fle_realmod_table` の名前になる。スクリプトは確かめない（`fle_realmod.c` が最初の検索で確かめる） |
| `DLL` | 元のファイルの絶対パス。`fe-pe rebase DLL BASE` で `$OUT/v-$TAG/dll/<basename>.<基底 16 進>.dll` に作る |
| `BASE` | `0x` つきの 16 進。`fe-pe rebase` と realmod の表の両方に同じ値が入る。**既定値なし** |
| `SEEDS` | 省略可。1 行 1 VA（リベース後の VA）。`fe-lift lift --seeds` に渡す |
| `OPTS` | 省略可。`pdata` だけ（`fe-lift lift --pdata`）。Blend2D の測定は `--pdata --seeds` |

パスと名前に `:` `;` `|` 空白 `"` `\` を含められない（区切りに使うため。含めると OUT を作る前に拒否する）。

| 環境変数 | 意味 |
|---|---|
| `OUT` | **必須**。絶対パス。空か、a4-multi.sh が作ったものだけ受ける（`.a4-multi` の印）。それ以外の既存ディレクトリは何も書かずに拒否（exit 2） |
| `FLE` | FLEngine の DLL。既定 `$LIFT/../binaries/win64/FLEngine_x64.dll`（`git archive` の木には `binaries/` が無いので、その場合は必須） |
| `LIFT` | `crates/ runtime/ harness/` のある木。既定はスクリプトの 1 つ上の `lift/`（`git archive` の木でよい） |
| `FEBIN` | `fe-lift` と `fe-pe` のあるディレクトリ。既定 `$OUT/target/release`（無ければ `cargo build --release --offline -p fe-pe -p fe-lift` を `CARGO_TARGET_DIR=$OUT/target` で実行） |
| `JOBS` | clang の並列数（既定 6） |
| `TAG` | 既定は、モジュール列（名前、ファイルの SHA-256、基底、OPTS、種ファイルの SHA-256）のハッシュ `t<12 桁>`。同じ列は OUT が違っても同じ TAG（実測: `t6a353495d10f` が `run-b` と `run-c` で同じ）。別の列は別の TAG なので上書きしない |
| `FLE_PER_PART` / `REAL_PER_PART` | 分割の関数数（既定 3700 / 800） |
| `EXPECT_FLE_SHA` | FLEngine の生成 C の SHA-256 の期待値（既定は HEAD の `feb5b9087f8d…e080f`）。違えば止まる。`skip` で検査しない |

標準出力: `a4-survey.sh` の要約、`a4:` の 6 行、`runtime message:`（停止メッセージ全文）と `== stop:` `abort kind:` `dispatch target:` `call depth at stop:`、functions と entries、ハーネスの終了コード、段ごとの秒数、ログの場所。進行は標準エラー。終了コードは `a4-survey.sh` のもの（0 = 調査が走った。停止点は結果で、エラーではない）。必要なものが無い、段が失敗した、は 2。

実行ごとに `$OUT/runs/<UTC 時刻>-<pid>/` を作る（`gen.txt`、`build.txt`、`survey.txt`、`a4/<時刻>-{units.tsv,native.txt,native.err}`）。何も削除せず、上書きしない。同じ OUT で同じ列をもう一度実行すると、生成とビルドは「exists」で飛ばし（実測: 2 回目は gen 0〜1 s、build 0〜1 s）、実行だけをやり直す。

### 2.1 実際に使った 3 つのコマンド（実測）

```
B=/home/ubuntu/work/signature-studio-develop
FLE=$B/binaries/win64/FLEngine_x64.dll OUT=<W>/run-a bash lift/harness/a4-multi.sh                      # (a)
DSP=dsp_ippv2_x64.dll:$B/lift/out-ext/dsp_ippv2_x64.dll:0x2000000:$B/lift/out-three/three/dll/seeds-idx0-plus-initterm2.txt
FLE=… OUT=<W>/run-b bash lift/harness/a4-multi.sh $DSP                                                  # (b)
B2D=blend2d.dll:$B/lift/out-blend2d/Blend2D.dll:0x4700000:$B/lift/out-seed58/seeds/seeds58-4700000.txt:pdata
FLE=… OUT=<W>/run-c bash lift/harness/a4-multi.sh $DSP $B2D                                             # (c)
```

`<W>` は `lift/out-harness-int`。`lift/harness/` は `lift/out-harness-int/src/lift/harness/`（`git archive` の木）。

## 3. 中で何が走るか（読み + 実測）

| 段 | 0 個（単独） | 1 個以上 |
|---|---|---|
| 準備 | `fe-lift` と `fe-pe` を `$OUT/target` に作る（`FEBIN` 未指定のとき） | 同じ |
| 生成 | `fe-lift lift FLEngine gen/flengine.c --pdata`、SHA-256 検査 | `gen_two.sh rebase gen split merge`（`MODULES=m1|…;m2|…`、`DLLDIR=v-$TAG/dll`）。FLEngine の生成 C は `$OUT/gen` で全 TAG が共有 |
| 分割 | `harness/wasm/split_c.py`（`wasm/build.sh` と同じ） | `split_mod.py`（`crates/fe-lift/tests/realmod_s1/tools`）と `merge_tables_n.py` |
| ビルド | `harness/wasm/build_native.sh`（変更なし）→ `flengine-native` | `build_two_native.sh`（`modules.txt` から realmod の表を生成。`RMNAMES` で名前を渡す）→ `two-native` |
| 実行 | `a4-survey.sh`（変更なし。`NATIVE_BIN=flengine-native`） | `a4-survey.sh`（変更なし。`NATIVE_BIN=two/two-native-run.sh`、`TWO_REAL=<リベース済みファイルを : で連ねたもの>`） |

モジュールのラベルは `m1`、`m2`、…（引数の順）。生成物は `v-$TAG/gen/m1.c`、`v-$TAG/src/m1-NN.c` など。表の名前は引数の `NAME`。

`OUT` 直下の `stamp.txt` は、ランタイム全ファイル、ハーネスのソース、分割ツール、`fe-lift` と `fe-pe` のバイナリ、FLEngine の DLL、`*_PER_PART`、`EXPECT_FLE_SHA` の SHA-256 から作る。**別の内容の OUT は拒否する**（理由: `build_two_native.sh` と `build_native.sh` は作ったオブジェクトを飛ばし、古さを検出しない）。実測した拒否は `EXPECT_FLE_SHA` を変えた場合だけ（`log/neg-stamp.txt`）。他の変更で拒否されるかは試していない。

## 4. 測定（実測）

環境: clang 21.1.8、`nproc` 16、メモリ 13 GiB。**この間、他の作業者の `cargo test` が 3〜4 本同時に走っており、秒数は負荷の下の値**（速度の評価には使えない）。`python3` は `PYTHONDONTWRITEBYTECODE=1`、`TMPDIR` は各 OUT の `tmp/`。`/tmp` は使っていない。

入力: FLEngine `b2c64da4…9496d`、dsp_ippv2 `497dd4f7…c352`、Blend2D `66634d24…066c7`（絶対パス・SHA-256 はすべて依頼のとおりで、`sha256sum` で一致を確認）。FLEngine の生成 C は `feb5b9087f8d7a8a1282900212d7bd1f5b6071b0326e7149642f98c168e8080f`（`a4-multi.sh` が検査して一致）。リベース済み dsp_ippv2（0x2000000）は先頭 `4fd255027525f2a2`、リベース済み Blend2D（0x4700000）は先頭 `24e01ec9295870db`（`run-c/v-t79f49a54425f/dll/`。過去の文書の値と先頭 16 桁が一致。全桁の比較はしていない）。dsp_ippv2 の生成 C: 関数 8,030、`bad_nodes 0`。Blend2D（`--pdata` + 58 個）: 関数 2,067、closure 405,674、`bad_nodes 0`（`progress.md` の値と一致）。統合した表: 2 モジュール 67,595 行、3 モジュール 69,662 行、`f_<VA>` の衝突 0。

### 4.1 (a) 単独

a4-multi.sh の実行 2 回と、`a4-survey.sh` を直接（同じ `flengine-native` で）2 回。4 回とも同じ:

```
stop   : fle: LoadLibraryW: module "dsp_ippv2_x64.dll" is not a virtual module and not a name this environment is defined to lack (external modules are M2; no handle and no failure is invented for them)
         top of stack: 0x423990 f_423990
depth  : init_count=145 / 942   last unit: index=144 run_rank=132/895 proc=0x65c540 in_progress
         started=132 completed=131 in_progress=1 of 895 runnable   all_units_done=no
funcs  : executed (distinct)=821  entries=15685
a4: init_count=145 of 942, last_unit_proc=0x65c540
a4: last_unit index=144 run_rank=132 of 895 proc=0x65c540 state=in_progress
a4: runnable_started=132 completed=131 in_progress=1 of 895 runnable
a4: unit[init_count-1] proc=0x65c540; units_outside_image=0
a4: entry_check started_but_not_entered=0 runnable_entered_beyond_init_count=0
a4: all_units_done=no functions_distinct=821 function_entries=15685
harness exit code: 134
```

- `native.txt`（281 行）は、既存の出力 `lift/out-progress/a4build/a4/20261025T113046Z-native.txt` と `calling entry … rsp=` の 1 行（アドレス。ASLR）を除いて全行一致（`diff` の出力 0 行。比べたのは直接の 2 回と a4-multi.sh の 1 回目の 3 本。a4-multi.sh の 2 回目は 1 回目と一致）。`native.err` は直接の 2 回で `cmp` 一致。
- 比較に使った既存の出力は別の作業者のもの（`lift/out-progress/`）で、私はそれを作っていない。単独の `flengine-native` は HEAD の `build_native.sh`（変更なし）で作った。
- 段の秒数（1 回目）: lift 4 s、split 4 s、build 62 s。ビルドの警告 0、エラー 0。

### 4.2 (b) 2 モジュール

`NAME=dsp_ippv2_x64.dll`、基底 0x2000000、種 452 個。TAG `t6a353495d10f`。`two-native` 571,255,856 B。2 回（1 回目 gen 20 s + build 278 s、2 回目は既存を再利用）:

```
stop   : fle: LoadLibraryW: module "blend2d.dll" is not a virtual module and not a name this environment is defined to lack (external modules are M2; no handle and no failure is invented for them)
         top of stack: 0x423990 f_423990
depth  : init_count=150 / 942   last unit: index=149 run_rank=137/895 proc=0x664260 in_progress
         started=137 completed=136 in_progress=1 of 895 runnable   all_units_done=no
funcs  : executed (distinct)=943  entries=26585
a4: init_count=150 of 942, last_unit_proc=0x664260
a4: last_unit index=149 run_rank=137 of 895 proc=0x664260 state=in_progress
a4: runnable_started=137 completed=136 in_progress=1 of 895 runnable
a4: unit[init_count-1] proc=0x664260; units_outside_image=0
a4: entry_check started_but_not_entered=0 runnable_entered_beyond_init_count=0
a4: all_units_done=no functions_distinct=943 function_entries=26585
harness exit code: 134
```

2 回の `native.txt` は `calling entry … rsp=` の 1 行を除いて同一、`native.err` は `cmp` 一致。呼び出し深さ 9。

### 4.3 (c) 3 モジュール

`NAME=blend2d.dll`、基底 0x4700000、`--pdata --seeds`（58 個）。TAG `t79f49a54425f`。`two-native` 612,732,928 B（この OUT で最大の単一ファイル。2 GB 未満。`find -size +300M` で確認）。2 回（1 回目 gen 21 s + build 165 s）:

```
stop   : fle: indirect call to 0x489f940 is not a known function entry
         top of stack: 0x489f7d0 f_489f7d0
depth  : init_count=150 / 942   last unit: index=149 run_rank=137/895 proc=0x664260 in_progress
         started=137 completed=136 in_progress=1 of 895 runnable   all_units_done=no
funcs  : executed (distinct)=1069  entries=27333
a4: init_count=150 of 942, last_unit_proc=0x664260
a4: last_unit index=149 run_rank=137 of 895 proc=0x664260 state=in_progress
a4: runnable_started=137 completed=136 in_progress=1 of 895 runnable
a4: unit[init_count-1] proc=0x664260; units_outside_image=0
a4: entry_check started_but_not_entered=0 runnable_entered_beyond_init_count=0
a4: all_units_done=no functions_distinct=1069 function_entries=27333
abort kind: fle_dispatch_miss
dispatch target: 0x489f940
call depth at stop: 20
harness exit code: 134
```

2 回の `native.txt` は `calling entry … rsp=` の 1 行を除いて同一、`native.err` は `cmp` 一致。基準（`progress.md`、決定 0094 の「検証」）の 150/942、1,069、27,333、停止 `0x489f940` と**一致**。差はなかった。

### 4.4 同じ OUT に別の列を入れる

`run-c`（3 モジュールを作った OUT）に dsp の 1 個だけの列を実行: TAG `t6a353495d10f`（`run-b` と同じ）の新しいディレクトリ `v-t6a353495d10f/` ができ、`v-t79f49a54425f/` は変わらない。FLEngine の生成 C とオブジェクトは再利用（gen 7 s、build 110 s。dsp の分を作り直した分）。a4 の行は `run-b` のものと同一（150/942、943、26,585、停止 `blend2d.dll`）。

### 4.5 生成の再現性

`run-b` と `run-c` は別の OUT で、`fe-lift` も別に作ったが、dsp_ippv2 の生成 C `m1.c`（SHA-256 先頭 `45690ae2c35f32ba`）、リベース済み dsp（`4fd255027525f2a2`）は同じ。`merge_tables.py`（一般化前、`crates/fe-lift/tests/realmod_s1/tools/`）を `run-b` の 2 つの表に直接実行した `common.c` と、`merge_tables_n.py` の `common.c`（4,079,756 B）は `cmp` で一致。

## 5. 負の入力（実測。`run-n1`、`run-n4`、`run-n5`。Blend2D を種・pdata なしで使った小さい構成）

| 入力 | 結果 |
|---|---|
| 基底 `0x7FF800000000`（仮想モジュールのハンドル域の先頭）1 個 | `fe-pe rebase` と画像の mmap は通り、最初の `fle_realmod_find_handle` で runtime が停止: `fle: fle_realmod_find_handle: real module "blend2d.dll": base 0x7ff800000000 is not usable (a nonzero multiple of 64 K, not 0x400000, below the virtual module handles)`。init_count 2 / 942、functions 33、entries 49。a4-multi.sh は exit 0 でこの停止を出した |
| 実在モジュール 4 個（`b1.dll`〜`b4.dll`、基底 0x4700000、0x4a00000、0x4d00000、0x5000000） | 通常どおり進み、FLEngine 単独と同じ停止（`LoadLibraryW "dsp_ippv2_x64.dll"`、145/942、821、15,685）。表に dsp が無いため |
| 実在モジュール 5 個（上の 4 個 + `b5.dll` 0x5300000） | 5 個すべてを画像として mmap（`real module table:` 5 行）したあと、runtime が停止: `fle: fle_realmod_find_handle: fle_realmod_count is larger than the 4 real modules this runtime holds`。init_count 2 / 942 |

読み: 基底が 64 KiB の倍数でない場合は `fe-pe rebase` が拒否する（`rebase.rs:163`）。基底 0x400000 は画像の mmap が `MAP_FIXED_NOREPLACE`（`image.c`）で失敗するはずだが、**試していない**（未確認）。

## 6. 一般化前との差（実測 + 読み）

「一般化前」は 3 つある。(i) HEAD の `lift/harness/two/`（コミット `e90231d`。1 モジュール）、(ii) 測定用の一般化コピー（`lift/out-srw/three/harness-two/` など 4 か所。`diff -r` で**4 か所とも全ファイル同一**。実測）、(iii) 今回の版。

| ファイル | (i) → (ii) | (ii) → (iii) |
|---|---|---|
| `gen_two.sh` | `MODULES` で N 個、`merge_tables_n.py`、`LIFT`、`modules.txt`、`pipefail` | **`EXPECT_FLE_SHA` の既定値を `43985d62…` から HEAD の `feb5b908…` に**（(ii) の既定値のままでは HEAD の生成 C と合わず `gen` が止まる。`progress.md` の 3 モジュールの測定は `EXPECT_FLE_SHA=feb5b908…` を環境変数で渡している）。`EXPECT_FLE_SHA=skip` を追加。`DLLDIR`（リベース済みファイルの置き場。a4-multi.sh が TAG ごとに指す）を追加。コメントから `lift/out-three` の記述を除いた。行数 +80 −25（HEAD との差） |
| `build_two_native.sh` | `modules.txt` から realmod の表を生成、`*-NN.c` を全部コンパイル、`BASE` を廃止、`LIFT`、`RMNAMES` | 動作は同じ。コメントと `OUT` の既定値（`OUT=${OUT:?…}` を HEAD と同じ `${OUT:-$LIFT/out-stage2/two}` に戻した）だけ。+46 −12（HEAD との差） |
| `main_two.c` | `argv[2]` を `:` で分け N 個、表との照合、`__EMSCRIPTEN__` の 1 範囲 | **`MAIN_REAL_MAX` を 4 から 16 に**。(ii) は 5 個目で exit 64（`more than 4 real module files`）だったが、(iii) は 5 個とも mmap して runtime に止めさせる（§5）。+64 −22（HEAD との差） |
| `two-native-run.sh` | コメントのみ | コメントの記述を整えた。+2 −0 |
| `merge_tables_n.py` | 新規（2 個のとき `merge_tables.py` と同じ出力） | 同一（`diff` 0 行） |
| `a4-multi.sh` | — | 新規（182 行） |

HEAD の `two/` の他のファイル（`build_two_wasm.sh`、`chain.sh`、`gdb_*.py`、`patches_*.py`、`realmod_table.c` など）は変えていない。(ii) の 4 コピーも同じ。`realmod_table.c` は `build_two_wasm.sh` が使う（`build_two_native.sh` はもう使わない）。

## 7. テスト

**HEAD の基準: 98 binaries / 1,228 passed / 0 failed。**

テストは `harness/image.c`（複数のテストが include／コンパイルする）、`harness/a4.c`（`a4_report.rs`）、`harness/a4-survey.sh`（コメントのみ）、`harness/wasm/build.sh`（コメントのみ）を読む。`harness/two/`、`a4-multi.sh`、`merge_tables_n.py` を名指すテストは無い。`realmod.rs` は `realmod_s1/tools/merge_tables.py` を使うが、これは変えていない（読み。`grep`）。変えたファイルはどれも `cargo test` の入力ではないので、影響は無いと**読む**。実測の結果は下。

TEST_RESULT_PLACEHOLDER

## 8. 限界と未確認

- **native だけ**。`emcc` は使っていない。`build_two_wasm.sh` は一般化していない（1 モジュール前提、`BASE`、`realmod_table.c`、`src/dll-NN.c` の glob）。`main_two.c` の `__EMSCRIPTEN__` の分岐（3 つの像を覆う 1 範囲）は作っていないので**未確認**。a4-multi.sh は wasm を扱わない。
- **「2 回」の意味**: 2 回目は同じ OUT の既存ビルドを再利用して実行だけをやり直した。別の OUT でビルドから 2 回やったのは §4.5 の生成物（`m1.c`、リベース済み DLL）まで。`two-native` のバイト一致は見ていない（過去の文書は、ビルドパスの違いでサイズが数十〜数千 B 違うと記録している）。
- 異なる列を同じ OUT に入れる共有は、(b) と (c) の 2 列だけ試した。OUT 間の共有（FLEngine の生成 C、オブジェクトのコピー）は実装していない。新しい OUT は FLEngine の 17 分割とランタイムを作り直す（§4 の秒数に含まれる）。
- `stamp.txt` が拒否するのは、確かめた `EXPECT_FLE_SHA` の変更だけ。ランタイムのファイルの変更などで拒否されることは**読み**（`stamp()` の対象）で、実行していない。
- OPTS は `pdata` だけ。`--no-adjustors`、`--msvc-base-*` など `fe-lift` の他のオプションは a4-multi.sh から渡せない（`gen_two.sh` の `MODULES` には書ける）。
- `gen_two.sh` の `EXPECT_REAL_SHA`（元のファイルの SHA-256 検査）は a4-multi.sh から空にしている（任意のファイルを受けるため）。入力の SHA-256 は実行の最初に標準出力へ出す。
- 基底 0x400000 の拒否、基底が 64 KiB の倍数でない入力、名前が大文字を含む入力、名前が仮想モジュールと同じ入力は実行していない（`fle_realmod.c:36-70` の `table_check` が止めると**読む**）。
- 3 モジュールの停止（`0x489f940`）は A4 の進捗ではない（`progress.md`）。この文書は停止点の意味を扱わない。
- `a4-multi.sh` の実行ビットはパッチで運べない（`bash` で起動する。取り込み側が必要なら `chmod +x`）。
- 他の作業者の `cargo test` と同時に走っていたため、秒数は参考値。

## 9. 成果物

- 差分: `lift/out-harness-int/patches/harness-int.diff`（`diff -ruN`。6 ファイル。`patch -p1` で HEAD の木に当たる）。別の clean な archive（現 HEAD `5002d98` の `git archive`、`archive-apply-check/`）で `git apply --check -p1` が exit 0、`patch -p1 --dry-run` が exit 0。実際に当てた木（`apply-real/`。`5002d98` の `lift/harness` に `patch -p1`）と `src/lift/harness` は `diff -r` で一致。
- 変更・追加したファイルの行数（HEAD との差）: `a4-multi.sh` +182（新規）、`two/merge_tables_n.py` +61（新規）、`two/gen_two.sh` +80 −25、`two/build_two_native.sh` +46 −12、`two/main_two.c` +64 −22、`two/two-native-run.sh` +2。
- 作業ディレクトリ: `src/`（`git archive dc27f4d`。`src/lift/out-*` と `src/lift/target/` は `cargo test` が作った gitignore 対象のテスト出力）、`base-dc27f4d/`、`archive-apply-check/`、`apply-real/`、`run-a`（単独）、`run-b`（2 モジュール）、`run-c`（3 + 2 モジュール）、`run-n1`、`run-n4`、`run-n5`（負の入力）、`log/`、`target/`、`tmp/`。
