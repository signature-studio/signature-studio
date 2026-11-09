# 2 モジュール構成の wasm（15 関数だけ -O1、別々の翻訳単位）を node で動かした測定

測定のみ。修正・決定はしない。作業中の文書（節ごとに追記して保存する）。

## 0. 凡例

- **実測** = このセッションでコマンドを実行して得た値。
- **読み** = ソース、ログ、別の文書を読んだ結果。実行して確かめていない。
- **未確認** = 確認できていない。

## 1. 対象と環境

- 作業の基準: ブランチ main、HEAD `c1eefb5b63ec687a7fb607c44a2b03444b07ed88`（`git rev-parse HEAD`。実測）。`git archive HEAD` を `lift/out-wasm2/src/` に展開し、そこからだけビルドした。作業ツリーの `lift/runtime`、`lift/crates` は読んでいない（並行の 0088 の作業は含まない）。
- 環境: emcc 6.0.12（`/opt/emsdk`）、node v24.19.0、`wasm-objdump`/`wasm-validate`（wabt）。16 コア、RAM 約 15 GB（`free -m` で total 15,047 MB）、swap 4 GB。
- 作業物: `lift/out-wasm2/o1two/`（gitignore 対象 `lift/out*/`）。

## 1.1 15 関数の確定と -O0/-O1 の locals（実測）

- 生成 C: FLEngine 17 ファイル（`fle-00.c`〜`fle-16.c`）+ 縮小 DLL 11 ファイル（`dll-00.c`〜`dll-10.c`）+ `common.c`。`lift/out-ctype87/two/` の同名ファイルと `cmp` で全部一致（`common.c`、`dll-*.c`、`fle-*.c`、種ファイル）。DLL 側 `functions 8030`、`closure_instructions 3,038,781`、`bad_nodes 0`、`c_lines 3,224,242`。
- 全 `.o`（-O0、`-finstrument-functions` つき = 現行の `CFLAGS_GEN`。FLEngine 17、DLL 12、runtime 46、ハーネス 5）の関数ごとの locals を `locals_by_name.py`（`lift/out-locals/tools/` のコピー）で数えた（`o1two/logs/o0-locals-all.tsv`、70,875 行）。**50,000 超は 15 関数**（`o1two/logs/o0-over50000.tsv`）。最大は `f_3d04cf0`・`f_3d3a420` の 389,948。50,000 以下の最大は `f_30589a0` の 41,126。FLEngine の最大は `f_7cf9b0` の 20,136。調査 §3.1 の 15 関数と **関数名・-O0 の locals が 15 個とも一致**。（`fle_string.o` と `realmod_table.o` は関数の定義を持たず、コード節が無いので数えていない。）
- 分け方: 15 関数を **1 関数ずつ別の翻訳単位**（`fn-<va>.c`。前置きは元の `dll-NN.c` と同じ、本体は元と同一）。`dll-05.c` から 10 個、`dll-09.c` から 5 個を外し、残り（`dll-05-rest.c` 790 関数、`dll-09-rest.c` 795 関数）は `-O0`。**元の `dll-05.c`、`dll-09.c` の関数 800 個ずつと、rest + fn の本体を SHA-256 で比べて同一**（`split_o1.py`。生成 C の文は変えていない）。15 関数だけ `-O1`（`-finstrument-functions` つき。他のフラグは `CFLAGS_GEN` と同じ）。

| 関数 | 元の翻訳単位 | -O0 locals | -O1 locals | -O1 秒 / RSS | V8（`.o`） |
|---|---|---:|---:|---|---|
| `f_3057340` | dll-05 | 70,635 | 11 | 4.62 s / 207 MB | 受理 |
| `f_3057360` | dll-05 | 50,926 | 10 | 0.40 s / 95 MB | 受理 |
| `f_3057400` | dll-05 | 71,434 | 11 | 4.64 s / 208 MB | 受理 |
| `f_3057420` | dll-05 | 71,434 | 11 | 4.65 s / 208 MB | 受理 |
| `f_3057580` | dll-05 | 52,644 | 10 | 1.51 s / 158 MB | 受理 |
| `f_30575a0` | dll-05 | 52,644 | 10 | 1.50 s / 158 MB | 受理 |
| `f_3059220` | dll-05 | 70,218 | 11 | 4.67 s / 202 MB | 受理 |
| `f_30592e0` | dll-05 | 71,213 | 11 | 4.65 s / 207 MB | 受理 |
| `f_3059440` | dll-05 | 52,215 | 10 | 1.45 s / 157 MB | 受理 |
| `f_3059460` | dll-05 | 51,658 | 10 | 0.98 s / 145 MB | 受理 |
| `f_3d04cf0` | dll-09 | 389,948 | 8 | 12.05 s / 678 MB | 受理 |
| `f_3d3a420` | dll-09 | 389,948 | 8 | 12.01 s / 678 MB | 受理 |
| `f_3d70550` | dll-09 | 90,005 | 7 | 1.54 s / 199 MB | 受理 |
| `f_3d76760` | dll-09 | 97,613 | 7 | 1.44 s / 187 MB | 受理 |
| `f_3d7d970` | dll-09 | 90,005 | 7 | 1.65 s / 201 MB | 受理 |
| **合計 / 最大** | | | 最大 11 | **合計 57.76 s**（表の 15 値の和）、最大 12.05 s / 678 MB | 15 個とも受理 |

- 17 個の `-O1`/`rest` ステージの `.o` の関数の locals の最大: `-O1` の 15 個の `.o` は **18**（`.o` 内の全関数の最大。15 関数本体は上の表の 7〜11。18 を持つ関数は、`static inline` の補助関数の定義が `.o` に残っているものと読んだが、どの関数かは見ていない）、`dll-05-rest.o` は 41,126、`dll-09-rest.o` は 6,751。**50,000 超は 17 個すべてで 0 個**（`wasm_locals.py`。実測）。
- `-O1` の時間・RSS は調査 §3.1 の値（合計 56.5 s、最大 `f_3d04cf0` 11.8 s / 676 MB）と同程度（実測の比較。並走の emcc は無かった。`ps` で 0 本を確認）。
- `dll-05-rest.c` 13.32 s / 4.06 GB、`dll-09-rest.c` 13.31 s / 3.20 GB（-O0）。調査 §7 の「`dll-05.c`、`dll-09.c` 13〜14 秒」は stage23 の `tbl2s`（15 関数を停止に差し替えたコピー）の値で、本測定の元の `dll-05.c`（15 関数を含む -O0）は 16.69 s / 4.76 GB、`dll-09.c` は 24.31 s / 4.27 GB。
- -O0 の全ステージ（rt 46 + fle 17 + dll 12 + harness 5）の emcc `-c` の合計の wall は 196.58 s（スクリプト全体。最大 RSS は `dll-00.c` の 9.20 GB。`ulimit -v` 12 GiB 内）。

## 1.2 リンクした wasm（実測）

- リンク: `build_o1.sh link`（`build_two_wasm.sh` の `stage_link` と同じ `emcc -O0 -sALLOW_MEMORY_GROWTH=1 -sSTACK_SIZE=8MB -sGLOBAL_BASE=268435456 -sENVIRONMENT=node -sMODULARIZE=1 -sEXIT_RUNTIME=1 -sNODERAWFS=1 -Wl,--initial-memory=0x28000000 -Wl,--max-memory=0x80000000 -Wl,--wrap=fprintf -Wl,--wrap=fputs -Wl,--wrap=abort … -lm`。オブジェクトの一覧だけ違う: 元の `dll-05.o`、`dll-09.o` の代わりに `dll-05-rest.o`、`dll-09-rest.o` と `fn-*.o` 15 個）。**終了コード 0、`link.log` は 0 バイト（警告なし）**。
- リンク時間 **1.93 s**、最大 RSS **1,415,272 KB（1.35 GB）**（`/usr/bin/time`）。stage23 §7 の記録（8.32 s / 1.4 GB）より短い。この差の原因は調べていない（未確認）。
- `two.wasm` **329,590,522 B**（314.3 MiB）、`two.js` 174,917 B。stage23 §7 の `tbl2`（`-O0` のみ、HEAD 87bc74d）の 340,905,484 B より 11,314,962 B 小さい。15 関数の本体のバイト数は `.o` の Code 節で `-O0` 合計 25,200,127 B → `-O1` 合計 13,603,987 B（差 11,596,140 B。実測。調査 §3.1 の「合計 25,200,127 B → 13,603,987 B」と同じ値）で、wasm のサイズ差とほぼ同じ大きさ。残りの差 281,178 B（11,596,140 − 11,314,962）の内訳は調べていない（HEAD の違いによる runtime や生成 C の変化を含みうるが、確かめていない。未確認）。
- `wasm_locals.py`: **functions 71,803、最大 locals 41,126、50,000 超 0 個**（最大は `f_30589a0` = func #4768）。`wasm-validate` 終了コード 0。V8: `new WebAssembly.Module`（`v8_validate.cjs`）で **受理**（228 ms。node v24.19.0）。

## 2. node での実行と native との比較（実測）

実行: `FLE_WASM_DIR=<bin-o1> node <bin-o1>/run_node.cjs binaries/win64/FLEngine_x64.dll <リベース済み DLL>`（`run_node.cjs` は `lift/harness/two/run_node_two.cjs` のコピー。`/usr/bin/time -v`）。native は同じ HEAD の archive から `build_two_native.sh`（clang。JOBS=2）で作った `two-native`（571,231,184 B）を同じ 2 つの引数で実行した（`a4-survey.sh` は使わず、ハーネスが出す `a4:` の行をそのまま比べた）。PM の native のログ `lift/out-git/a4two87/a4/20261024T042713Z-native.{txt,err}`（読み取りのみ）とも比べた。

### 2.1 a4 の行と停止（実測）

| 項目 | 基準（two-module-after-0087.md） | native（PM のログ / 自分のビルド 2 回） | wasm `-O1`（node 2 回） |
|---|---|---|---|
| `a4: init_count` | 146 of 942、last_unit_proc 0x65c9f0 | 同じ | 同じ |
| `a4: last_unit` | index 145、run_rank 133 of 895、proc 0x65c9f0、in_progress | 同じ | 同じ |
| `a4: runnable_started … completed … in_progress` | 133 / 132 / 1 of 895 | 同じ | 同じ |
| `a4: unit[init_count-1]` | proc 0x65c9f0、units_outside_image 0 | 同じ | 同じ |
| `a4: entry_check` | started_but_not_entered 0、runnable_entered_beyond_init_count 0 | 同じ | 同じ |
| `a4: all_units_done … functions_distinct … function_entries` | no / **935** / **25,215** | 同じ | 同じ |
| 停止（stderr、1 行、185 B） | `fle: LoadLibraryW: module "avrt.dll" is not a virtual module and not a name this environment is defined to lack (external modules are M2; no handle and no failure is invented for them)` | 同じ（SHA-256 先頭 16 桁 `48ab3ecc9b0c6df5`） | **同じ**（`48ab3ecc9b0c6df5`） |
| 呼び出し深さ（停止時） | 6（`#0 f_423990` … `#5 f_1376820`） | 同じ | 同じ |
| 終了コード | — | 134 | 134 |

### 2.2 stdout の全文 diff（実測）

- stdout は native（PM のログ、自分のビルド 2 回）も wasm も **280 行**。
- `diff`（PM の native ログ と node-1）: **差は 4 行目の `calling entry (no export name) rcx=0x400000 rdx=1 r8=0 rsp=…` の 1 行だけ**（native `rsp=0x77544b7fffc0`、自分の native-1 は `0x713fbe5fffc0`、wasm は `0x11c9afc0`）。`rsp=` を含む行を除くと **差 0 行**。除いた stdout の SHA-256（先頭 16 桁）は、PM の native ログ、native-1、native-2、node-1 の 4 つとも `e552b26533ee801a`。node-2 は node-1 と `cmp` でバイト一致（`rsp=` も 0x11c9afc0 で同じ）。stderr は 4 つとも同じ。
- `rsp=` の差は、`a3-wasm-recheck.md` §5 が単独構成で記録した既知の差と同じ形（native は実アドレス、wasm は線形メモリ内の値）。**読み**: 原因の説明も同文書の読みと同じで、ここでは確かめていない。

### 2.3 実行時間と最大 RSS（実測）

| 実行 | wall | user | 最大 RSS |
|---|---|---|---|
| node 1（`-O1` の 15 関数を含む wasm） | 2:05.76（125.76 s） | 125.60 s | 1,023,452 KB（約 999 MiB） |
| node 2 | 2:00.31（120.31 s） | 120.01 s | 1,023,840 KB |
| native 1、2（自分のビルド） | 0.07 s | — | 97,520 KB、97,496 KB |

- 参考（条件が違うので比較にならない）: stage23 §7 の `tbl2s`（15 関数を名前付き停止に差し替えたコピー、HEAD 87bc74d、停止は `GetStartupInfoW`、入口 15,909）は node 117.02 s、最大 RSS 982,932 KB。今回の停止は DllMain の後（入口 25,215）なので、同じ時間として並べられない。
- node は 2 回だけ。実行の間の機械の負荷は制御していない（emcc、wasm-ld、wasm-opt が 0 本であることは各ステージの前に `ps` で確認した。他のユーザーのプロセスは見ていない）。時間の差 5.5 秒（2 回）の原因は調べていない（未確認）。

### 2.4 15 関数は実行されたか（実測 + 読み）

- **実測（native）**: 自分のビルドの `two-native` を gdb（`tools/gdb_count15.py`。入口に breakpoint を置いて数えるだけで、値は書かない）で実行し、15 関数の入口の回数を数えた: **15 関数すべて 0 回**。対照として `f_65c9f0`（unit 145）1 回、`f_37caca8`（DllMain）1 回（`lift/out-wasm2/o1two/runs/gdb-count15.tsv`）。gdb の実行は SIGABRT で gdb が止まった時点で終わり、ハーネスの停止レポートは出ていない（stdout は 4 行で、`rsp=` の行を除いた先頭 3 行は node-1 と同じ。レポートが出ない理由は、gdb が SIGABRT を先に捕まえるためと読んだが、確かめていない）。したがってこの gdb の実行は、15 関数の回数を数えるためだけのもので、a4 の行の比較には使っていない。`f_30589a0`（locals 最大の非超過関数）と `f_35b3830`、`f_30559e0`（15 関数に至るスタブ）も 0 回。
- **読み**: wasm の a4 の `functions_distinct 935`、`function_entries 25,215` が native と同じなので、wasm でも 15 関数は入られていない。wasm 側の 15 関数の入口の回数は直接は数えていない。
- したがって **この実行は、`-O1` の 15 関数の本体を 1 回も動かしていない**（読み。stage23 §7 の「連鎖の範囲で 1 回も入られない」と同じ性質）。この測定が示すのは、ビルド、リンク、V8 の受理と、**15 関数を `-O1` の別の翻訳単位にした wasm が、それ以外の経路で `-O0` のときと同じ出力を出すこと**まで。`-O1` の 15 関数の計算が native と一致するかは測っていない。

## 3. 結果の読み

- **(a) 一致**: 2 モジュール構成（FLEngine + 縮小 dsp_ippv2 の 8,030 関数）で、15 関数だけ `-O1` の別々の翻訳単位にした wasm は、V8（node v24.19.0）が受理し、node で動き、a4 の行 6 本、停止メッセージ、stdout の全文が native と（`rsp=` の 1 行を除いて）一致した（実測）。基準（two-module-after-0087.md の init_count 146/942、last unit index 145 proc 0x65c9f0、functions_distinct 935、function_entries 25,215、`LoadLibraryW "avrt.dll"` の停止）とも一致する。
- 不一致は無かったので、「どの値が違うか」「`-O1` が原因か」の切り分け（`-O0` の名前付き停止に差し替えたコピーとの比較）は行っていない（任意の項目。必要が無かった）。
- 読み（意見ではなく範囲の限定）: 一致は **15 関数が実行されない経路**での一致である（§2.4）。`-O1` が 15 関数の計算を変えない保証は、この測定からは出ない。

## 4. 限界（未測定、未確認）

- **15 関数を動かしていない**（§2.4）。`-O1` の 15 関数の出力（SSE の `fle_mx_*` を含む）が `-O0` や native と一致するかは未測定。調査 §7 の「`-O1` の浮動小数点の native との一致」は未確認のまま。
- **`-O1` のコンパイラが生成 C の未定義動作を最適化で変える可能性**は、a4 の範囲（15 関数が実行されない経路）でしか確かめていない。
- **`-finstrument-functions` つきの `-O1` の関数がフックを呼ぶ回数と、`trace.c` のフックとの相互作用**は、15 関数が入られないので未測定。
- **停止点（`LoadLibraryW "avrt.dll"`）より先**の SSE/x87 の等価性は未測定。この停止点までに SSE/x87 が何回実行されたかも数えていない。
- **ブラウザは未検証**（`-sENVIRONMENT=node`、`-sNODERAWFS=1`）。node v24.19.0（V8）だけ。Chrome、Firefox の 1 関数 locals の上限は確認していない（調査 §10 と同じ）。
- **wasm のページ保護は fail-open**（決定 0045 c。`a3-wasm-recheck.md` §8）。この停止点までの経路で差が現れたかは測っていない（出力は一致）。
- 繰り返しは node 2 回、native 2 回（+ PM のログ 1 回）。`rsp=` 以外の非決定性は見つからなかったが、回数が少ない。
- 実行時間の約 120 秒の内訳は調べていない（`a3-wasm-recheck.md` §6 は単独構成の約 105 秒の主因を `-finstrument-functions` のフックが呼ぶ `_emscripten_return_address` の JS スタック解析と読んだ。今回の 2 モジュールでプロファイルは取っていない）。
- 1 台のマシン（16 コア、RAM 約 15 GB）。ブラウザ、他の OS、他の emcc のバージョンは未測定。
- 15 関数を **1 つの翻訳単位にした** 場合のビルドは今回やっていない（調査 §3.1 の 61.91 s / 1.96 GB の値のまま。今回の別々の合計 57.76 s、最大 678 MB と比べられる形の実測は無い）。
- 並行の 0088 の変更（`lift/runtime/fle_modules.c` ほか）は含まない。HEAD の archive だけを使った。0088 を含めた wasm は未測定。

## 5. 実行したコマンド（実測。`W=/home/ubuntu/work/signature-studio-develop/lift/out-wasm2`、`ROOT=/home/ubuntu/work/signature-studio-develop`）

環境の変数: `TMPDIR=$W/tmp`、`EM_CACHE=$W/emcache`、`PYTHONDONTWRITEBYTECODE=1`（`$W/emcache` と `$W/tmp` は以前の調査が作ったもので、emscripten のキャッシュ（`sanity.txt` は 6.0.12）を再利用した）。

```
git archive HEAD | tar -x -C $W/src/                                   # HEAD c1eefb5
cp lift/out-ctype87/two/dll/* $W/o1two/dll/                              # 種 3 + リベース済み DLL（SHA-256 4fd25502…efc5 を確認）
cp lift/out-locals/tools/{wasm_locals.py,locals_by_name.py,extract_fn.py,v8_validate.cjs} $W/tools/
cd $W/src/lift && CARGO_TARGET_DIR=$W/target cargo build --release --offline -p fe-lift -p fe-pe   # 1.48 s
OUT=$W/o1two TAG=tbl2 SEEDS=$W/o1two/dll/seeds-idx0-plus-initterm2.txt FEBIN=$W/target/release \
  FLE=$ROOT/binaries/win64/FLEngine_x64.dll REAL=$ROOT/lift/out-ext/dsp_ippv2_x64.dll \
  bash $W/src/lift/harness/two/gen_two.sh rebase gen split merge       # 7.51 s。FLEngine C の SHA-256 一致
bash $W/tools/build_o1.sh rt fle dll harness                            # -O0、現行の CFLAGS_GEN。196.58 s
for f in $W/o1two/v-tbl2/wasm/obj/*.o $W/o1two/wasm/{fle,rt,h}/*.o; do python3 $W/tools/locals_by_name.py $f; done   # 15 関数を確定
python3 $W/tools/split_o1.py $W/o1two/v-tbl2/src $W/o1two/v-tbl2/src-o1 $W/o1two/logs/o0-over50000.tsv
bash $W/tools/build_o1.sh o1                                            # dll-05-rest / dll-09-rest（-O0）+ fn-*.c 15 本（-O1）。84.67 s
bash $W/tools/build_o1.sh link                                          # 1.93 s
FLE_WASM_DIR=$W/o1two/v-tbl2/wasm/bin-o1 node $W/o1two/v-tbl2/wasm/bin-o1/run_node.cjs binaries/win64/FLEngine_x64.dll $W/o1two/dll/dsp_ippv2_x64.2000000.dll   # 2 回
OUT=$W/o1two TAG=tbl2 JOBS=2 bash $W/src/lift/harness/two/build_two_native.sh   # native（clang）。137.42 s
$W/o1two/v-tbl2/native/two-native binaries/win64/FLEngine_x64.dll $W/o1two/dll/dsp_ippv2_x64.2000000.dll   # 2 回
gdb -q -batch -x $W/tools/gdb_count15.py                                 # 15 関数の入口の回数（読み取りのみ）
```

- `build_o1.sh` は `lift/harness/two/build_two_wasm.sh`（HEAD の archive のもの）のコピーで、パスの固定、`o1` の段、`link` の段のオブジェクトの一覧（`dll-05.o`、`dll-09.o` の代わりに `-rest.o` と `fn-*.o`）だけが違う。`rt`、`fle`、`dll`、`harness` の段とフラグ（`CFLAGS_RT`、`CFLAGS_GEN`、`CFLAGS_H`）、リンクのコマンドは同じ。`-O1` の 15 本のフラグは `CFLAGS_GEN` の `-O0` を `-O1` にしただけ（`-finstrument-functions` つき）。**`lift/harness` の既存スクリプトは編集していない**。
- emcc は常に 1 本（各ステージの前に `ps` で emcc、wasm-ld、wasm-opt が 0 本であることを確認）。各 emcc は `ulimit -v 12582912`（12 GiB）の下。native のビルドは clang（JOBS=2）で、emcc ではない。

## 6. ビルドの時間（実測）

`/usr/bin/time` の各 `emcc -c` の wall の合計（`o1two/*/log/*.time`）:

| 内訳 | 秒 |
|---|---:|
| runtime（`fle_*.c` 46 本、`-O0`） | 3.26 |
| FLEngine（17 本、`-O0 -finstrument-functions`） | 69.25 |
| ハーネス（5 本、`-O1`） | 0.41 |
| `common.c`（`-O0`） | 0.61 |
| `dll-*.c` のうちリンクに入れた 9 本（`dll-05.c`、`dll-09.c` を除く 11 本のうちの 9 本。`-O0`） | 80.76 |
| `dll-05-rest.c`、`dll-09-rest.c`（`-O0`） | 26.63 |
| 15 関数（`-O1`、別々） | **57.76**（最大 12.05 s / 678 MB） |
| **リンクした `.o` の `emcc -c` の合計** | **238.68** |
| リンク（`emcc -O0`） | 1.93（1.35 GB） |
| 合計 | **約 240.6** |

- 参考: 15 関数を `-O0` のまま（元の `dll-05.c`、`dll-09.c`）にした全 `-O0` の `emcc -c` の合計は 195.29 s（rt 3.26 + fle 69.25 + h 0.41 + common 0.61 + dll 全 11 本 121.76）。`-O1` にしたことによる増加は **+43.4 s**（= rest 26.63 + `-O1` 15 本 57.76 − 元の `dll-05.c` 16.69 − `dll-09.c` 24.31）。元の `dll-05.o`、`dll-09.o` は今回の wasm にリンクしていない（`-O0` では V8 が拒否する関数を含むため、`.o` のまま残してある）。
- **emcc の最大 RSS**: 全体では `dll-00.c`（`-O0`）の 9.20 GB。**`-O1` にした 15 本の最大は `f_3d04cf0`、`f_3d3a420` の 678 MB**（約 676〜678 MB。調査 §3.1 の 676 MB と同程度）。`-O1` の段の最大は `dll-05-rest.c`（`-O0`）の 4.06 GB。
- stage23 §7 の記録（`tbl2` の `-c` の合計 202.8 s、リンク 8.32 s / 1.4 GB）とは、HEAD が違う（87bc74d と c1eefb5）ので並べない。リンクの 1.93 s と 8.32 s の差の原因は調べていない（未確認）。

## 7. 作ったファイル

すべて `lift/out-wasm2/`（gitignore 対象 `lift/out*/`。以前の調査の物が入っているディレクトリで、既存のファイルは触っていない。新規は `src/`、`o1two/`、`tools/`、`target/` の中身のうち今回のビルドの分）。

- `src/`: `git archive HEAD` の展開（新規。`lift/out-wasm2/src/` は存在しなかった）。
- **既存の成果物への影響（要確認の点）**: `lift/out-wasm2/` は 2026-10-21 の別の調査（決定 0045/0046 の wasm 移植）の作業ディレクトリで、`target/release/fe-lift`（10-21 03:38、1,468,536 B）、`emcache/`、`tmp/` が既にあった。依頼に従って `CARGO_TARGET_DIR=$W/target` で `cargo build` したため、**この `target/release/fe-lift`（と `fe-pe`）は今回の HEAD の archive からのビルド（10-24 13:36、1,598,848 B）に置き換わった**（cargo の通常のビルド出力の更新。`target/release/` と `target/release/deps/` を `ls` で見て、10-21 の `fe_lift-*` は残っていないことを確認した。実測）。`docs/decisions/0046-m1-acceptance.md` 60 行目は「`lift/out-wasm2/target/release/fe-lift`（2026-10-21 03:38 ビルド）で再生成した」と書いているので、その時点のバイナリを後から再現したい場合、この置き換えが影響する。`lift/out-wasm2/out/`（10-21 の生成 C）、`bin-base/`、`obj-base/` は触っていない。`emcache/` は `sanity.txt` が 6.0.12 で今回の emcc と同じなので再利用した（emcc がキャッシュに追加で書いた可能性はあるが、見ていない）。
- `o1two/`: `dll/`（種、リベース済み DLL）、`gen/`、`src/`（FLEngine の 17 分割）、`v-tbl2/src/`（DLL の 11 分割 + `common.c`）、`v-tbl2/src-o1/`（`dll-05-rest.c`、`dll-09-rest.c`、`fn-<va>.c` 15 本）、`wasm/`（rt、fle、h の `.o` とログ）、`v-tbl2/wasm/obj/`（`-O0` の `.o`）、`v-tbl2/wasm/o1/`（`-O1` と rest の `.o`）、`v-tbl2/wasm/bin-o1/`（`two.wasm` 329,590,522 B、`two.js`、`run_node.cjs`）、`v-tbl2/native/two-native`、`runs/`（node-1、node-2、native-1、native-2、gdb-count15 の出力と `.time`）、`logs/`（`o0-locals-all.tsv`、`o0-over50000.tsv`、`o1-locals.tsv`、ビルドのログ）。
- `tools/`: `build_o1.sh`、`split_o1.py`、`gdb_count15.py`（新規）、`wasm_locals.py`、`locals_by_name.py`、`extract_fn.py`（使っていない）、`v8_validate.cjs`（`lift/out-locals/tools/` のコピー）。
- `o1two/` の合計は約 3.5 GB。1 GB 超の単一ファイルは無い（最大は `two-native` 571 MB と `two.wasm` 329 MB、`dll-00.o` 90 MB）。
- 規則: `rm`、`/tmp`、git の変更系操作は使っていない。`__pycache__` は作っていない（`PYTHONDONTWRITEBYTECODE=1`。`find` で確認）。
