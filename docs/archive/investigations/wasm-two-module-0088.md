# 2 モジュール構成（決定 0088 後の 150/942）の wasm（15 関数だけ -O1、別々の翻訳単位）を node で動かした測定

測定のみ。修正・決定はしない。作業中の文書（節ごとに追記して保存する）。

## 0. 凡例

- **実測** = このセッションでコマンドを実行して得た値。
- **読み** = ソース、ログ、別の文書を読んだ結果。実行して確かめていない。
- **未確認** = 確認できていない。

## 1. 対象と環境

- 作業の基準: ブランチ main、HEAD `925c5224ae0e28cd55a0059be5c2cf801cff2757`（作業開始時の `git rev-parse HEAD`。実測）。`git archive HEAD` を `lift/out-wasm88/src/` に展開し、そこからだけビルドした。作業ツリーの `lift/runtime`、`lift/crates` は読んでいない。
- **作業の途中で別のセッションが HEAD を `04f2eedcfc115c5273834ae0c46e2e24429f5988` に進めた**（私はコミットしていない）。`git diff --stat 925c522 HEAD` は `docs/investigations/blend2d-seed58.md`（312 行追加）だけで、`git diff --stat 925c522 HEAD -- lift` は **空**（実測）。ビルドに使った `lift/` は 2 つの HEAD で同じ。この文書の測定は 925c522 の archive から行った。
- 決定 0088 は HEAD に入っている（コミット 2e94df2）。`lift/runtime/fle_modules.c` の `fle_modules[]` に `avrt.dll`（N=31）、`powrprof.dll`（N=32）がある（実測。`out-wasm88/two/wasm/rt/fle_modules.o` は、前回 `lift/out-wasm2/o1two/wasm/rt/fle_modules.o` の 6,726 B に対し 6,849 B。前回（c1eefb5）と今回の `.o` の大きさが違うのは、この `.o` だけ。`fle_load.o` ほかの runtime、FLEngine、DLL、`-O1` の 15 個、`common.o` の `.o` の大きさは前回と全部同じ。実測）。
- 構成: **FLEngine + 縮小 dsp_ippv2（種 452 個、8,030 関数）の 2 モジュール**。Blend2D は載せない（3 モジュールは対象外）。
- 環境: emcc 6.0.12（`/opt/emsdk`）、node v24.19.0、`wasm-objdump`/`wasm-validate`（wabt）、gdb 17.1。16 コア。RAM は **測定の間に `free -m` の total が 15,129 → 16,797 → 27,989 → 15,883 → 21,611 MB と変わった**（仮想マシンのメモリが動的に変わるように見えるが、原因は調べていない。未確認）。swap 4 GB（使用 約 2.5 GB のまま）。
- 作業物: `lift/out-wasm88/`（gitignore 対象 `lift/out*/`。今回新規に作った。`lift/out-wasm2`、`lift/out-avrt88`、`lift/out-three`、`lift/out-seed58` には**書いていない**。`lift/out-wasm2/emcache`（emscripten のキャッシュ。`sanity.txt` は 6.0.12）は `lift/out-wasm88/emcache/` に**コピーして**使った）。
- 入力の確認（最初に。実測）:

| 項目 | 期待 | 実測 |
|---|---|---|
| FLEngine の生成 C（`out-wasm88/two/gen/flengine.c`、215,707,512 B）の SHA-256 | `43985d62321a90f562ce1b8144253e00af13df795e4190d26b6b8ce2537137a1` | **一致**（`gen_two.sh` の検査も通過） |
| リベース済み DLL `dsp_ippv2_x64.2000000.dll`（`lift/out-avrt88/two/dll/` からコピー） | `4fd255027525f2a2e56a93fe1cc7f95442861f788179a23ac8b0401b8f4fefc5` | **一致** |
| 縮小 DLL の C（`dll-00.c`〜`dll-10.c`）、`common.c`、FLEngine の分割（`fle-00.c`〜`fle-16.c`、`fle-tables.txt`）、`dll-tables.txt` | `lift/out-avrt88/two/` と同じ | `cmp` で**全部バイト一致**（差 0）。`common.c` の SHA-256 `33779308…487d5f`（two-module-after-0088.md §1 の値と一致） |
| 縮小 DLL の lift | — | `functions 8030`、`closure_instructions 3,038,781`、`bad_nodes 0`、`c_lines 3,224,242`（two-module-after-0088.md の `two-gen.txt` と同じ） |

## 2. ビルド（実測）

### 2.1 15 関数の確定と分割

- 全 `.o`（`-O0`、`-finstrument-functions` つき。FLEngine 17、DLL 12、runtime 46、ハーネス 5）の関数ごとの locals を `locals_by_name.py` で数えた（`out-wasm88/logs/o0-locals-all.tsv`、70,876 行）。**50,000 超は 15 関数**（`logs/o0-over50000.tsv`）。関数名と `-O0` の locals は、`wasm-two-module-o1.md` §1.1 の 15 個と**全部一致**（`f_3d04cf0`・`f_3d3a420` 389,948、`f_3d76760` 97,613、`f_3d70550`・`f_3d7d970` 90,005、`f_3057400`・`f_3057420` 71,434、`f_30592e0` 71,213、`f_3057340` 70,635、`f_3059220` 70,218、`f_3057580`・`f_30575a0` 52,644、`f_3059440` 52,215、`f_3059460` 51,658、`f_3057360` 50,926）。50,000 以下の最大は `f_30589a0` の 41,126（`dll-05.o`）。FLEngine の最大は `f_7cf9b0` の 20,136。
- 分け方（前回と同じ `split_o1.py`）: 15 関数を **1 関数ずつ別の翻訳単位**（`fn-<va>.c`）にし、`dll-05.c` から 10 個、`dll-09.c` から 5 個を外した（`dll-05-rest.c` 790 関数、`dll-09-rest.c` 795 関数）。元の `dll-05.c`、`dll-09.c` の関数 800 個ずつの本体を、rest + fn の本体と文字列で比べて**全部同一**、関数の集合も同じ（実測。`logs/split_o1.txt` と、その場の Python での検査）。生成 C の文は変えていない。
- `-O1` の 15 本は `-O1 -finstrument-functions`（他のフラグは `CFLAGS_GEN` と同じ。`-O0` を `-O1` にしただけ）。rest と残りは前回と同じ `-O0`。

### 2.2 15 関数の `-O1` の locals、時間、V8（実測）

| 関数 | 元の翻訳単位 | -O0 locals | -O1 locals（`.o`） | -O1 秒 / RSS | V8（`.o`） |
|---|---|---:|---:|---|---|
| `f_3057340` | dll-05 | 70,635 | 11 | 5.50 s / 206 MB | 受理 |
| `f_3057360` | dll-05 | 50,926 | 10 | 0.42 s / 96 MB | 受理 |
| `f_3057400` | dll-05 | 71,434 | 11 | 5.74 s / 208 MB | 受理 |
| `f_3057420` | dll-05 | 71,434 | 11 | 5.85 s / 208 MB | 受理 |
| `f_3057580` | dll-05 | 52,644 | 10 | 1.82 s / 157 MB | 受理 |
| `f_30575a0` | dll-05 | 52,644 | 10 | 1.91 s / 157 MB | 受理 |
| `f_3059220` | dll-05 | 70,218 | 11 | 5.77 s / 203 MB | 受理 |
| `f_30592e0` | dll-05 | 71,213 | 11 | 5.82 s / 207 MB | 受理 |
| `f_3059440` | dll-05 | 52,215 | 10 | 1.84 s / 157 MB | 受理 |
| `f_3059460` | dll-05 | 51,658 | 10 | 1.28 s / 144 MB | 受理 |
| `f_3d04cf0` | dll-09 | 389,948 | 8 | 17.60 s / 679 MB | 受理 |
| `f_3d3a420` | dll-09 | 389,948 | 8 | 18.73 s / 674 MB | 受理 |
| `f_3d70550` | dll-09 | 90,005 | 7 | 1.97 s / 201 MB | 受理 |
| `f_3d76760` | dll-09 | 97,613 | 7 | 1.93 s / 187 MB | 受理 |
| `f_3d7d970` | dll-09 | 90,005 | 7 | 1.89 s / 202 MB | 受理 |
| **合計 / 最大** | | | 最大 11 | **合計 78.07 s**（表の 15 値の和）、最大 18.73 s / 679 MB | 15 個とも受理 |

- 15 個の `-O1` の locals（7〜11）は `wasm-two-module-o1.md` §1.1 の表と **15 個とも同じ**。
- 17 個の `-O1`/`rest` の `.o` の関数の locals の最大: `-O1` の 15 個は **18**（前回と同じ値。`.o` 内の全関数の最大）、`dll-05-rest.o` は 41,126、`dll-09-rest.o` は 6,751（前回と同じ）。**50,000 超は 17 個すべてで 0 個**（`wasm_locals.py`。`logs/o1-wasm_locals.txt`）。17 個とも V8 が `.o` を受理（`node v8_validate.cjs`）。
- **時間は前回（合計 57.76 s、最大 12.05 s / 678 MB）より長い**: 15 本の合計 78.07 s（約 1.35 倍）、最大 18.73 s。RSS は同じ程度（最大 679 MB と 678 MB）。**-O0 の rest、dll、fle の時間も前回より長い傾向がある**（§2.4 の表）。原因は調べていない（未確認）。emcc は 1 本だけで、各ステージの前に `ps` で emcc、wasm-ld、wasm-opt が 0 本なのを確認した（実測）。生成物（locals、`.o` の大きさ）は前回と同じなので、時間の差はビルドの結果に影響していない。

### 2.3 リンクした wasm（実測）

- リンク: `build_o1.sh link`（前回と同じ `emcc -O0 -sALLOW_MEMORY_GROWTH=1 -sSTACK_SIZE=8MB -sGLOBAL_BASE=268435456 -sENVIRONMENT=node -sMODULARIZE=1 -sEXIT_RUNTIME=1 -sNODERAWFS=1 -Wl,--initial-memory=0x28000000 -Wl,--max-memory=0x80000000 -Wl,--wrap=fprintf -Wl,--wrap=fputs -Wl,--wrap=abort … -lm`。オブジェクトの一覧は、元の `dll-05.o`、`dll-09.o` の代わりに `dll-05-rest.o`、`dll-09-rest.o` と `fn-*.o` 15 個）。**終了コード 0、`link-o1.log` は 0 バイト（警告なし）**。
- リンク時間 **2.20 s**、最大 RSS **1,414,880 KB（1.35 GB）**（前回は 1.93 s / 1,415,272 KB）。
- `two.wasm` **329,590,538 B**（314.3 MiB）、`two.js` 174,917 B（前回の `two.js` と `cmp` で一致）。**前回の `two.wasm`（329,590,522 B）より 16 B だけ大きい**（実測。2 つの `two.wasm` の差の内訳は見ていない。`fle_modules[]` に 2 名（`avrt.dll`、`powrprof.dll`）が増えたことと関係するかもしれないが、文字列 2 つ + 配列の 2 要素の大きさ（約 30 B）とも 16 B は合わないので、**原因は未確認**）。SHA-256 `two.wasm` `a905ab43…a2c9`、`two.js` `2ab026c3…2a11`。
- `wasm_locals.py`: **functions 71,803、最大 locals 41,126（func #4768）、50,000 超 0 個**（前回と同じ値）。`wasm-validate` 終了コード 0（14.03 s、最大 RSS 14.76 GB。**測定の間、他に emcc は走っていなかった。この RSS は wasm-validate の値で、emcc の制限（12 GiB）の外**）。V8: `new WebAssembly.Module`（`v8_validate.cjs`）で **受理**（268 ms。node v24.19.0）。

### 2.4 ビルドの時間（実測。`/usr/bin/time` の各 `emcc -c` の wall の合計）

| 内訳 | 今回（秒） | 前回 `wasm-two-module-o1.md` §6（秒） |
|---|---:|---:|
| runtime（`fle_*.c` 46 本、`-O0`） | 3.90 | 3.26 |
| FLEngine（17 本、`-O0 -finstrument-functions`） | 76.08 | 69.25 |
| ハーネス（5 本、`-O1`） | 0.43 | 0.41 |
| `common.c`（`-O0`） | 0.65 | 0.61 |
| `dll-*.c` のうちリンクに入れた 9 本（`dll-05.c`、`dll-09.c` を除く。`-O0`） | 85.62 | 80.76 |
| `dll-05-rest.c`、`dll-09-rest.c`（`-O0`） | 30.43（15.61 + 14.82） | 26.63 |
| 15 関数（`-O1`、別々） | **78.07**（最大 18.73 s / 679 MB） | 57.76（最大 12.05 s / 678 MB） |
| **リンクした `.o` の `emcc -c` の合計** | **275.18** | 238.68 |
| リンク（`emcc -O0`） | 2.20（1.35 GB） | 1.93（1.35 GB） |
| 合計 | **約 277.4** | 約 240.6 |

- 参考: 15 関数を `-O0` のまま（元の `dll-05.c` 19.25 s、`dll-09.c` 26.64 s を含む。リンクには入れていない）の全 `-O0` の `emcc -c` の合計は 212.57 s（rt 3.90 + fle 76.08 + h 0.43 + common 0.65 + dll 全 11 本 131.51）。`-O1` にしたことによる増加は **+62.6 s**（= rest 30.43 + `-O1` 15 本 78.07 − 元の `dll-05.c`、`dll-09.c` の 45.89）。前回の増加は +43.4 s。
- emcc の最大 RSS: 全体では `dll-00.c`（`-O0`）の 9.20 GB（9,198,544 KB。`ulimit -v` 12 GiB 内。前回と同じ）。
- 今回の各ステージの wall（`time`）: `rt fle harness` 1m21.6s、`dll` 2m12.4s、`o1` 1m48.8s、`link` 2.2s。cargo のビルド（`fe-lift`、`fe-pe`）3.91 s / 402 MB、`gen_two.sh rebase gen split merge` 10.35 s / 711 MB。
- 今回の時間が前回より全体に 10〜35 % 長い理由は調べていない（未確認）。前回と今回で、機械の負荷（`uptime` の load average は今回 0.2〜0.8）、RAM の total が変わった（§1）。

## 3. node での実行と native との比較（実測）

実行: `FLE_WASM_DIR=<bin-o1> node <bin-o1>/run_node.cjs binaries/win64/FLEngine_x64.dll <リベース済み DLL>`（`run_node.cjs` は `lift/harness/two/run_node_two.cjs` と `cmp` で一致。`/usr/bin/time -v`）。node は **3 回**（依頼は 2 回。3 回目は 15 関数の入口数の測定の後に、時間のばらつきを見るために足した）。native は、自分の archive からの `build_two_native.sh`（clang、JOBS=2、`two-native` 571,231,264 B）を同じ 2 つの引数で **2 回**、`lift/out-avrt88/two/v-tbl2/native/two-native`（PM の native ビルド）を 1 回、PM のログ `lift/out-avrt88/out-tbl2/a4/20261024T052223Z-native.{txt,err}`（読み取りのみ）と比べた。`a4-survey.sh` は使わず、ハーネスが出す `a4:` の行をそのまま比べた（前回と同じ）。

### 3.1 a4 の行と停止（実測）

| 項目 | 基準（two-module-after-0088.md の native） | native（自分の 2 回 + out-avrt88 のビルド 1 回 + PM のログ） | wasm `-O1`（node 3 回） |
|---|---|---|---|
| `a4: init_count` | 150 of 942、last_unit_proc 0x664260 | 同じ | 同じ |
| `a4: last_unit` | index 149、run_rank 137 of 895、proc 0x664260、in_progress | 同じ | 同じ |
| `a4: runnable_started … completed … in_progress` | 137 / 136 / 1 of 895 | 同じ | 同じ |
| `a4: unit[init_count-1]` | proc 0x664260、units_outside_image 0 | 同じ | 同じ |
| `a4: entry_check` | started_but_not_entered 0、runnable_entered_beyond_init_count 0 | 同じ | 同じ |
| `a4: all_units_done … functions_distinct … function_entries` | no / **943** / **26,585** | 同じ | 同じ |
| 呼び出し深さ（停止時） | 9（`#0 f_423990` … `#8 f_1376820`） | 同じ | 同じ（スタック 9 段の全行が一致） |
| 終了コード | — | 134 | 134 |

- `a4:` の 6 行の SHA-256（先頭 16 桁）は、native 2 回、out-avrt88 のビルド、PM のログ、node 3 回のうち確認した 5 つ（native-1、native-2、native-avrt88、node-1、node-2。PM のログも）が全部 `5555d5a4db12a729`（実測）。node-3 は stdout 全体を node-1 と `cmp` で比べ、バイト一致（§3.2）。
- 停止（stderr。1 行、188 B）。全文は native も node も同じ:

```
fle: LoadLibraryW: module "blend2d.dll" is not a virtual module and not a name this environment is defined to lack (external modules are M2; no handle and no failure is invented for them)
```

stderr の SHA-256（先頭 16 桁）は、native 2 回、out-avrt88 のビルド、PM のログ、node 3 回で全部 `0d27deb2d6707edc`（PM のログとは `diff` で一致を確認）。ハーネスの標準出力の `runtime message:` も同じ文、`abort kind: (not a runtime message)`、`functions executed (distinct): 943`、`function entries (calls): 26585`、`call depth at stop: 9`、先頭 256 件の entry log も同じ。

- 前回の測定（`wasm-two-module-o1.md`。決定 0088 の前、基準は 146/942、935、25,215、停止 `LoadLibraryW "avrt.dll"`）から変わった値は、すべて 0088 で native が動いた分と同じだけ動いた（init_count 146 → 150、last unit 145 → 149、functions 935 → 943、entries 25,215 → 26,585、停止 avrt → blend2d、深さ 6 → 9）。**native と wasm の間の差は、前回と同じく 0**。

### 3.2 stdout と stderr の全文 diff（実測）

- stdout は native（自分の 2 回、out-avrt88 のビルド、PM のログ）も node（3 回）も **283 行**（前回は 280 行。深さが 6 → 9 で 3 行増えた分）。
- `diff native-1.txt node-1.txt`: **差は 4 行目の `calling entry (no export name) rcx=0x400000 rdx=1 r8=0 rsp=…` の 1 行だけ**（native-1 `rsp=0x75fef95fffc0`、wasm `rsp=0x11c9afc0`）。`diff` を native-avrt88 と node-1、PM のログと node-1 についても取った: どれも同じ 4 行目の 1 行だけ（native 側の値だけが毎回違う: `0x75fef95fffc0`、`0x7735f55fffc0`、`0x7218f65fffc0`、PM のログは `0x78a68f3fffc0`）。`diff native-1.txt native-2.txt` も 4 行目だけ。
- `rsp=` を含む行を除いた stdout の SHA-256（先頭 16 桁）: native-1、native-2、native-avrt88、PM のログ、node-1、node-2 とも **`627fb24871845443`**（5 つのファイルとも、PM のログは別に計算して一致）。node-1、node-2、node-3 は `cmp` でバイト一致（`rsp=0x11c9afc0` も同じ）。
- 同じ `rsp=` の差は、前回と `a3-wasm-recheck.md` §5 が記録した既知の差と同じ形（native は実アドレス、wasm は線形メモリ内の値）。**読み**: 原因の説明も同文書の読みと同じ。ここでは確かめていない。

### 3.3 実行時間と最大 RSS（実測）

| 実行 | wall | user | 最大 RSS |
|---|---|---|---|
| node 1 | 2:04.06（124.06 s） | 123.91 s | 1,023,584 KB（約 1.0 GB） |
| node 2 | 3:19.46（199.46 s） | 198.83 s | 1,023,480 KB |
| node 3（15 関数の入口数の測定の後） | 2:54.90（174.90 s） | 174.49 s | 1,024,100 KB |
| native 1、2（自分のビルド） | 0.10 s、0.09 s | 0.04 s | 96,712 KB、97,016 KB |
| native（out-avrt88 のビルド） | 0.15 s | 0.04 s | 92,584 KB |

- **node の時間は 3 回で 124 s、199 s、175 s と大きくばらついた**（実測）。前回の 125.8 s、120.3 s（`wasm-two-module-o1.md` §2.3。入口 25,215）と比べると、1 回目は同程度、2、3 回目は 1.4〜1.7 倍。3 回とも `Percent of CPU this job got: 100%`（node は 1 スレッドの CPU を使い切っていた）で、user の時間そのものが伸びている。
- 3 回目の間の機械全体の CPU 時間を `/proc/stat` の前後で取った: user 184.98 s、sys 15.84 s（全コアの合計）、idle 2,588.64 s（16 コア × 174.9 s ≒ 2,798 s のうち）。node の user 174.49 s を引くと、他のプロセスの user は約 10.5 s で、node の実行の邪魔になる量ではない（steal は 0）。1、2 回目の間は取っていない。
- **ばらつきの原因は調べていない（未確認）**。仮想マシンのホスト側の負荷かもしれないが、確かめていない（読みではなく推測。測定の間に `free -m` の total が変わったこと（§1）はこの推測の傍証にならない）。並行して別の測定（seed58 の native と gdb）が走っていた（`lift/out-seed58/` のファイルの更新時刻で見ただけで、その CPU 使用は測っていない）。
- 最大 RSS は 3 回とも約 1.02 GB で、前回（1.02 GB）と同じ。native は約 97 MB（前回 97.5 MB）。
- 前回の「`-finstrument-functions` のフックの JS スタック解析のコスト」（`a3-wasm-recheck.md` §6 の読み）が今回も効いているかは、今回プロファイルを取っていないので**未確認**。

## 4. 違い（既知の差と新しい差）

### 4.1 新しい差

- **ありません**（実測）。native と wasm の間で、a4 の行 6 本、停止メッセージ全文、stderr、stdout の全 283 行（`rsp=` の 1 行を除く）、呼び出し深さ、entry log 256 件が一致した。

### 4.2 既知の差（区別して書く）

| 差 | 今回の測定で見えたか | 出典・区分 |
|---|---|---|
| `calling entry … rsp=` の 1 行（native は実アドレス、wasm は線形メモリ内の値） | **見えた**（stdout の唯一の差） | `a3-wasm-recheck.md` §5、前回 `wasm-two-module-o1.md` §2.2。実測 |
| 実行時間が桁違い（native 約 0.1 s、node 124〜199 s）。`-finstrument-functions` のフックが呼ぶ `_emscripten_return_address`（JS のスタック解析）のコストが主因と `a3-wasm-recheck.md` §6 が読んでいる | **見えた**（時間は上の §3.3）。今回は主因を**測っていない** | 読み（同文書。今回は未確認）。実測は時間の値だけ |
| wasm のページ保護は fail-open（予約のみ、デコミット後、読み取り専用、PAGE_NOACCESS へのアクセスが wasm では成功する）。決定 0045 c、`a3-wasm-recheck.md` §8 | **この停止点までの経路で差が現れたかは測っていない**（出力は一致したので、少なくとも出力の差としては見えなかった） | 既知の限界。未測定 |
| `GetCurrentThreadId` は wasm で 1 固定、アリーナの上限 512 MiB、固定アドレスの予約は wasm で失敗、など | この経路で差が現れたかは測っていない（出力は一致） | `a3-wasm-recheck.md` §8。未測定 |
| `-O0` のみだった wasm が、今回 15 関数だけ `-O1` | 15 関数は入られない（§5）ので、**`-O1` が出力に影響したかは見えない** | 前回と同じ。実測（入口 0 回） |

### 4.3 差が出なかったので、位置や原因の読みは無い

依頼の手順 5（「違いが出たら…」）は、違いが出なかったので該当しない。`-O0` の名前付き停止に差し替えたコピーとの比較などの切り分けも行っていない（必要が無かった）。

## 5. 15 関数は入られたか（任意項目。実測 + 読み）

### 5.1 native の入口の回数（実測）

自分のビルドの `two-native`（`out-wasm88/two/v-tbl2/native/two-native`）を、読み取り専用の gdb（`tools/gdb_count15_88.py`。`lift/out-wasm2/tools/gdb_count15.py` のコピーで、対照の関数に `f_664260` を足しただけ。各関数の入口に breakpoint を置いて数え、`return` しない。ゲスト、`R[]`、メモリに何も書かない）で実行した。出力は `runs/gdb-count15.tsv`。

| 関数 | 入口の回数 |
|---|---:|
| 15 関数（`f_3057340`、`f_3057360`、`f_3057400`、`f_3057420`、`f_3057580`、`f_30575a0`、`f_3059220`、`f_30592e0`、`f_3059440`、`f_3059460`、`f_3d04cf0`、`f_3d3a420`、`f_3d70550`、`f_3d76760`、`f_3d7d970`） | **全部 0** |
| 対照: `f_65c9f0`（unit 145）、`f_664260`（unit 149。今回の last unit の proc）、`f_37caca8`（DllMain） | 1、1、1 |
| 参考: `f_30589a0`（locals 最大の非超過関数）、`f_35b3830`、`f_30559e0` | 0、0、0 |

- 15 関数を **呼ぶ側**（生成 C を `f_<va>();` の形で正規表現で探した。`dll-*.c` の中で、他の関数の本体から直接呼ばれている箇所。読み）は 1 つずつ: `f_30559e0`（`f_3057340`、`f_3057360`）、`f_3055a10`（`f_3057400`、`f_3057420`）、`f_3055aa0`（`f_3057580`、`f_30575a0`）、`f_3056250`（`f_3059220`）、`f_3056280`（`f_30592e0`）、`f_3056310`（`f_3059440`、`f_3059460`）、`f_35d17f0`（`f_3d04cf0`）、`f_35d5a00`（`f_3d3a420`）、`f_35e3880`（`f_3d70550`）、`f_35e45b0`（`f_3d76760`）、`f_35e5330`（`f_3d7d970`）。**この 11 個の呼ぶ側も gdb で数えた: 全部 0 回**（`runs/gdb-count-callers.tsv`。`f_65c9f0`、`f_664260`、`f_37caca8` は 1 回）。15 関数は dispatch 表（`common.c`）にも入っている（間接呼び出しの経路は、直接呼びの正規表現では見えない。入口の 0 回は gdb の実測なので、間接呼びでも入られていない）。
- したがって **今回の 150/942 の構成の停止点まで、15 関数は 1 回も入られず、その呼び出し元の 11 関数も入られていない**（実測、native）。前回（146/942）と同じ結果。
- wasm 側の入口の数は直接は数えていない。wasm の a4 の `functions_distinct 943`、`function_entries 26,585` が native と同じなので、wasm でも入られていない（読み）。gdb の実行は SIGABRT で止まり、ハーネスの停止レポートは出ない（stdout は 4 行、stderr は 1 行。stderr は §3 の stderr と SHA-256 が同じ `0d27deb2d6707edc`。レポートが出ない理由は前回と同じく gdb が SIGABRT を先に捕まえるためと読んだが、確かめていない）。この gdb の実行は入口を数えるためだけで、a4 の行の比較には使っていない。

### 5.2 この測定が示すこと

- この実行は、**`-O1` の 15 関数の本体を 1 回も動かしていない**。測ったのは、ビルド、リンク、V8 の受理と、**15 関数を `-O1` の別の翻訳単位にした wasm が、15 関数に入らない経路で `-O0` のときと同じ出力を出すこと**まで。**`-O1` の 15 関数の計算が native と一致するかは未測定**（前回の `wasm-two-module-o1.md` §2.4 と同じ性質）。

### 5.3 入口があった場合に、`-O1` 版と native の一致を測る方法の提案（**提案。実装していない。意見**）

入口は 0 回なので「入口があれば」の条件は今回は満たされない。連鎖の延長で入られる日を待たずに測るなら、次の方法がある（私の考え。確かめていない）:

- **15 関数を連鎖から切り離して直接呼ぶ**: `main_two.c` と別の小さなハーネスで、`R`、`X`、`FL`、`fle_mxcsr` と、関数が読み書きするメモリを、固定した乱数（種つき）で初期化し、`fle_dispatch(va)` 相当で 1 関数だけ呼び、戻った後の `R[16]`、`X[16]`、`FL`、`fle_mxcsr`、書かれたメモリ領域のハッシュを出す。これを (1) native の `-O0`（現行の native ビルド）、(2) wasm の `-O1`（今回の `fn-*.o` をリンクしたもの）で同じ入力で動かして比べる。
- 比較の相手が `-O0` の wasm では取れない（V8 が拒否する）ので、**native の `-O0`** を基準にする。`-O1` のコンパイラが生成 C の未定義動作を最適化で変える可能性の切り分けには、**native の `-O1`**（同じ `fn-*.c` を clang `-O1`）も並べると、「`-O1` による差」と「wasm と native の差」を分けられる（見込み。やっていない）。
- 関数の中身から、そのままでは動かない可能性がある点（実測。生成 C の文字列の数）: 15 関数のうち **7 関数は `fle_unsupported(...)` の行を持つ**（`f_3057360` 286、`f_3059460` 9、`f_3d04cf0`・`f_3d3a420` 各 396、`f_3d70550`・`f_3d7d970` 各 68、`f_3d76760` 276）。その行に届くと `abort()` する（`fle_runtime.h` の `fle_unsupported`）。残り 8 関数は 0 行。15 関数とも `fle_mx_*`（SSE の浮動小数点を整数演算で行う runtime の `static inline`。`fle_mxcsr_flags.h`）の呼び出しを 585〜13,978 回分持つ（例: `f_3d04cf0` の `fle_mx_mulpd` は 7,114 行）。つまり直接呼びの入力は、`fle_unsupported` に届く経路を避けるか、「どの入力で止まるか」も比較の対象にする必要がある。
- 15 関数は `switch`（jumptab）を持たず、`f_3d04cf0` 系の 5 関数は他の関数を呼ばない（前回 `wasm-locals-15-functions.md` §6.2 の記述。今回の `fn-*.c` で、`f_3057340` 系の 10 関数が `f_X();` を 6 個ずつ呼び、`f_3d04cf0` 系の 5 関数は 0 個なのを再確認した。実測）ので、直接呼びの環境の準備は小さくて済むと読めるが、`f_X()` の 6 個の呼び先が何をするかは見ていない（未確認）。
- 入力の選び方（どの `R`、メモリの値が現実の呼び出しに近いか）は決まっていない。ここで決めない。

## 6. 限界（未測定、未確認）

- **15 関数を動かしていない**（§5）。`-O1` の 15 関数の出力（SSE の `fle_mx_*` を含む）が native と一致するかは未測定。`-O1` が生成 C の未定義動作を最適化で変える可能性、`-finstrument-functions` つきの `-O1` の関数がフックを呼ぶ回数と `trace.c` のフックとの相互作用も、15 関数が入られないので未測定。
- **ブラウザは未検証**（`-sENVIRONMENT=node`、`-sNODERAWFS=1`）。node v24.19.0（V8）だけ。Chrome、Firefox の 1 関数 locals の上限は確認していない。
- **wasm のページ保護は fail-open**（決定 0045 c。`a3-wasm-recheck.md` §8）。この停止点までの経路で差が現れたかは測っていない（出力は一致）。
- **実機 Windows との照合なし**。ホストは Linux。native も wasm も、同じ生成 C と同じ runtime の 2 つのコンパイルで、実機の Windows の挙動の再現ではない。決定 0088 の runtime は avrt、PowrProf の `GetProcAddress` を NULL にし、ゲストは「機能が無い」経路を通る（two-module-after-0088.md §7）。この比較は、その経路での native と wasm の一致であり、実機との一致ではない。
- **停止点（`LoadLibraryW "blend2d.dll"`）より先**の SSE/x87 の等価性は未測定。この停止点までに SSE/x87 が何回実行されたかも数えていない。
- Blend2D を載せた 3 モジュール構成は対象外（測っていない）。
- **繰り返しの回数が少ない**: node 3 回（うち 1 回は後から足した）、native 2 回（+ out-avrt88 のビルド 1 回 + PM のログ 1 回）。`rsp=` 以外の非決定性は見つからなかったが、回数が少ない。
- **実行時間のばらつき**: node は 124 s、199 s、175 s。原因は未確認（§3.3）。前回の 120〜126 s の範囲に入ったのは 1 回だけ。時間の約 120 秒以上の内訳は調べていない（`a3-wasm-recheck.md` §6 の読みが今回も当てはまるかは未確認）。
- **ビルド時間も前回より長かった**（emcc `-c` の合計 275.18 s、前回 238.68 s）。原因は未確認。生成物（locals、`.o` の大きさ、`two.wasm` の大きさ）は前回とほぼ同じ。
- 1 台のマシン（16 コア）。RAM の total が測定の間に変わって見えた（§1。原因は未確認）。ブラウザ、他の OS、他の emcc のバージョンは未測定。
- 15 関数を **1 つの翻訳単位にした** 場合のビルドは今回やっていない。
- `two.wasm` が前回より 16 B 大きい理由は未確認（§2.3）。
- 並行して別の作業（seed58 の測定）が走っていた。その CPU 使用が node の時間に影響したかは測っていない。

## 7. 実行したコマンド（実測。`ROOT=/home/ubuntu/work/signature-studio-develop`、`W=$ROOT/lift/out-wasm88`）

環境の変数: `TMPDIR=$W/tmp`、`EM_CACHE=$W/emcache`（`lift/out-wasm2/emcache` を `cp -r` でコピーして使った。`sanity.txt` は 6.0.12）、`PYTHONDONTWRITEBYTECODE=1`。

```
git archive HEAD | tar -x -C $W/src/                                          # HEAD 925c522
cp $ROOT/lift/out-avrt88/two/dll/* $W/two/dll/                                  # リベース済み DLL（SHA-256 4fd25502…efc5）と種 3 ファイル
cp $ROOT/lift/out-locals/tools/{wasm_locals.py,locals_by_name.py,v8_validate.cjs} $W/tools/
cp $ROOT/lift/out-wasm2/tools/{build_o1.sh,split_o1.py,gdb_count15.py} $W/tools/    # 読み取り→コピー。build_o1.sh のパスだけ sed で out-wasm88 に変えた（$W/tools/build_o1.sh.orig-copy が元）
cd $W/src/lift && CARGO_TARGET_DIR=$W/target cargo build --release --offline -p fe-lift -p fe-pe   # 3.91 s
OUT=$W/two TAG=tbl2 SEEDS=$W/two/dll/seeds-idx0-plus-initterm2.txt FEBIN=$W/target/release \
  FLE=$ROOT/binaries/win64/FLEngine_x64.dll REAL=$ROOT/lift/out-ext/dsp_ippv2_x64.dll \
  bash $W/src/lift/harness/two/gen_two.sh rebase gen split merge                # 10.35 s。FLEngine C の SHA-256 一致
bash $W/tools/build_o1.sh rt fle harness                                       # -O0（harness は -O1）、現行の CFLAGS。1m21.6s
bash $W/tools/build_o1.sh dll                                                  # -O0。2m12.4s
for f in <全 .o>; do python3 $W/tools/locals_by_name.py $f; done               # 15 関数を確定（logs/o0-over50000.tsv）
python3 $W/tools/split_o1.py $W/two/v-tbl2/src $W/two/v-tbl2/src-o1 $W/logs/o0-over50000.tsv
bash $W/tools/build_o1.sh o1                                                   # dll-05-rest / dll-09-rest（-O0）+ fn-*.c 15 本（-O1）。1m48.8s
bash $W/tools/build_o1.sh link                                                 # 2.20 s
python3 $W/tools/wasm_locals.py two.wasm; wasm-validate two.wasm; node $W/tools/v8_validate.cjs two.wasm
FLE_WASM_DIR=$W/two/v-tbl2/wasm/bin-o1 /usr/bin/time -v node $W/two/v-tbl2/wasm/bin-o1/run_node.cjs \
  $ROOT/binaries/win64/FLEngine_x64.dll $W/two/dll/dsp_ippv2_x64.2000000.dll    # 3 回（runs/node-{1,2,3}）
OUT=$W/two TAG=tbl2 JOBS=2 bash $W/src/lift/harness/two/build_two_native.sh     # native（clang）。4m42s
$W/two/v-tbl2/native/two-native <FLEngine> <リベース済み DLL>                     # 2 回（runs/native-{1,2}）
$ROOT/lift/out-avrt88/two/v-tbl2/native/two-native <FLEngine> <リベース済み DLL>   # PM のビルド 1 回（runs/native-avrt88。読み取りのみ。出力は $W/runs/ へ）
gdb -q -batch -x $W/tools/gdb_count15_88.py        # 15 関数の入口の回数（GC_BIN=$W の two-native。読み取りのみ）
gdb -q -batch -x $W/tools/gdb_count_callers.py     # 15 関数を呼ぶ 11 関数の入口の回数
```

- `build_o1.sh` は `lift/harness/two/build_two_wasm.sh`（HEAD の archive のもの）のコピー（前回 `lift/out-wasm2/tools/build_o1.sh`）で、パスの固定、`o1` の段、`link` の段の違いは前回と同じ。`rt`、`fle`、`dll`、`harness` の段とフック（`CFLAGS_RT`、`CFLAGS_GEN`、`CFLAGS_H`）、リンクのコマンドは `build_two_wasm.sh` と同じ（`diff` は取っていない。前回の文書の記述と、スクリプトを読んだ範囲）。`-O1` の 15 本のフラグは `CFLAGS_GEN` の `-O0` を `-O1` にしただけ（`-finstrument-functions` つき）。**`lift/harness` の既存のスクリプトは編集していない**。
- emcc は常に 1 本（各ステージの前に `ps` で emcc、wasm-ld、wasm-opt が 0 本であることを確認）。各 emcc は `ulimit -v 12582912`（12 GiB）の下。native のビルドは clang（JOBS=2）で、emcc ではない。
- `lift/out-wasm88/target` だけを `CARGO_TARGET_DIR` に使った。`lift/out-wasm2/target/release/fe-lift`、`fe-pe` の更新時刻は今回の作業の前のまま（`2026-10-24 13:36`。前回の置き換えの時刻で、今回は触れていない。実測）。

## 8. 作ったファイル

すべて `lift/out-wasm88/`（gitignore 対象 `lift/out*/`。新規）と、この文書（新規）。

- `src/`: `git archive HEAD`（925c522）の展開。`target/`: fe-lift、fe-pe のビルド。`emcache/`、`tmp/`。
- `two/`: `dll/`（リベース済み DLL、種）、`gen/`、`src/`（FLEngine 17 分割）、`v-tbl2/src/`（DLL の 11 分割 + `common.c`）、`v-tbl2/src-o1/`（`dll-05-rest.c`、`dll-09-rest.c`、`fn-<va>.c` 15 本）、`wasm/`（rt、fle、h の `.o` とログ）、`v-tbl2/wasm/obj/`（`-O0` の `.o`）、`v-tbl2/wasm/o1/`（`-O1` と rest の `.o`）、`v-tbl2/wasm/bin-o1/`（`two.wasm` 329,590,538 B、`two.js`、`run_node.cjs`）、`v-tbl2/native/two-native`（571,231,264 B）。
- `runs/`: node-1〜3、native-1、native-2、native-avrt88 の出力（`.txt`、`.err`、`.time`）、gdb のログ（`gdb-count15.*`、`gdb-count-callers.*`）、`node-3.stat0/1`（`/proc/stat`）。
- `logs/`: `gen_two.txt`、`build-*.txt`、`o0-locals-all.tsv`（70,876 行）、`o0-over50000.tsv`、`o1-locals.tsv`、`o1-wasm_locals.txt`、`o1-15-locals.txt`、`two-wasm_locals.txt`、`two-wasm-validate.txt`、`two-v8.txt`、`two-sha.txt`、`split_o1.txt`。
- `tools/`: `build_o1.sh`（パスを変えたコピー）、`build_o1.sh.orig-copy`、`split_o1.py`、`gdb_count15.py`、`gdb_count15_88.py`、`gdb_count_callers.py`、`wasm_locals.py`、`locals_by_name.py`、`v8_validate.cjs`。
- `out-wasm88/` の合計は約 3.6 GB。1 GB 超の単一ファイルは無い（最大は `two-native` 571 MB と `two.wasm` 329 MB）。
- 規則: `rm`、`/tmp`、git の変更系操作は使っていない。`__pycache__` は作っていない（`PYTHONDONTWRITEBYTECODE=1`。`find` で 0 件を確認）。`lift/out-wasm2`、`lift/out-avrt88`、`lift/out-three`、`lift/out-seed58` には書いていない（`find -newer` で、`out-seed58` 以外に今回の作業の後に更新されたファイルが無いことを確認。`out-seed58` の更新は並行の seed58 の作業のもの）。
