# Blend2D.dll を 3 つ目の実在モジュールにした native 実行: 次の停止はどこか（測定のみ）

実行日: 2026-10-24。基準の HEAD は作業開始時 `2e94df28325173e4e18583303a490881be735785`（決定 0088 の暫定版）で、`git archive` はこのコミットから作った（`lift/out-three/src/` の `lift/runtime`、`lift/harness`、`lift/crates` の全 324 ファイルが `git show 2e94df2:<path>` と SHA-256 で一致することを確認した。実測）。**作業中に別の作業が 2 コミット進めた**（`8072399` は docs/progress.md、`a31a267` は docs と `lift/runtime/fle_load.c` のコメントだけの変更。`git diff 2e94df2 a31a267 -- lift/runtime/fle_load.c` はコメント行のみで、コードは変わっていない。実測。現在の HEAD は `a31a267bd25d60cdf963460cdb1249367dc98ba3`）。測定は `2e94df2` の木で行ったので、測定結果は `a31a267` にも当てはまる（コメントだけの差。読み）。**この文書は何も決めない。** 基底 0x4700000 などの値は「測定用の値」で、決定ではない（決定 0086 は方向だけを決め、値は決めていない）。runtime の応答を足していない。gdb で値を入れていない。**新しい停止は直していない**（範囲外）。

作業物は `lift/out-three/`（gitignore 対象の `lift/out*/`）。

## 0. 凡例

- **実測** = このセッションでコマンドを実行して得た値。ログは `lift/out-three/` の下。
- **読み** = ソース、生成 C、別の文書を読んだ結果。実行して確かめていない。
- **未確認** = そのとおり。

## 1. 要約

| # | 項目 | 結果 | 区分 |
|---|---|---|---|
| 1 | 2 モジュールの再現（新しい OUT `lift/out-three/two2/`。一般化したスクリプト、モジュール 1 つ） | **基準と一致**: init_count 150/942、last unit 149（proc 0x664260）、functions_distinct 943、function_entries 26,585、停止 `LoadLibraryW "blend2d.dll"`。FLEngine の C は `43985d62…37137a1`、リベース済み dsp_ippv2 は `4fd25502…efc5`、`common.c` と分割 C は `lift/out-avrt88/two/` と一致。2 回で結果同一 | 実測（§3） |
| 2 | 3 モジュールの構成（測定用の値。**決定ではない**） | Blend2D を基底 **0x4700000**（終端 0x4a00000）にリベース（`24e01ec9…b4cb`。往復で元ファイルと一致）。`--pdata` のみ、縮小なし、`--reloc-seeds` なし。関数 2,018、`unsupported_total 662`、`bad_nodes 0`。3 つの dispatch 表を統合（69,605 行。**`f_<VA>` の衝突 0**）、強い定義の重複 7 個は 1 つに。`two-native` 612,184,672 B。コンパイル警告 0 | 実測（§4） |
| 3 | 領域の衝突 | 3 つの像は重ならない（FLEngine 0x400000〜0x1cc0000、dsp 0x2000000〜0x461f000、Blend2D 0x4700000〜0x4a00000。空きは 3.25 MiB と 0.88 MiB）。Blend2D の終端から `GLOBAL_BASE`（0x10000000）まで 182.00 MiB。仮想ハンドル域・トークン域・応答表の擬似 VA とも重ならない。像の mmap は 3 つとも成功 | 導出、実測（mmap の成功）（§4.3） |
| 4 | `fle_mem_set_image` | 1 範囲（`fle_host.c:16-20`）。読むのは wasm 専用の `FLE_CHECKED_MEM` だけで、native では呼ばれず、読まれない。**native の 3 モジュールでは runtime に 3 つ目の範囲を知らせる必要はない**（読み）。wasm 用の拡張は一般化したコピーに書いたが、emcc を使っていないので**未ビルド・未実行** | 読み（§4.2） |
| 5 | **a4 の行（3 モジュール、gdb なし 2 回）** | init_count = **150 / 942**（変わらず）、last unit = index **149**（run_rank 137/895、proc `0x664260`、in_progress）、started 137 / completed 136、functions_distinct = **960**（2 モジュール 943 → +17）、function_entries = **26,604**（26,585 → +19）、all_units_done = no。2 回とも同一 | 実測（§5） |
| 6 | **次の停止** | **`fle: indirect call to 0x48a57f8 is not a known function entry`**（`fle_dispatch_miss`。top of stack `f_48b5b90`）。**Blend2D の DllMain(ATTACH) には入った（`rcx=0x4700000 rdx=1 r8=0`）が戻っていない**（eax なし。`fle_realmod.c:144` に未到達）。**`LoadLibraryW` は 0x4700000 を返していない**（ATTACH の途中で停止） | 実測（§5.1） |
| 7 | 停止の原因（読みと実測の突き合わせ） | ATTACH の中で、CFG の踏み台（`f_48b5b90` = `jmp rax`。Load Config の `GuardCFDispatchFunctionPointer`）経由で、`.rdata` の 16 バイト刻みの表（`0x48b8a40`〜`0x48b8b40`）の先頭のポインタを呼ぶループ（`f_48ab984`）が、最初のエントリ `0x48a57f8`（`.pdata` に無い葉関数。調査の `reloc-not-function.txt` の 58 個の 1 つ）を呼んだ。**生成 C の dispatch 表に無い**（先頭が非 0 の 12 個のうち 3 個が無い） | 実測（表との照合）、読み（ループの役割）（§5.2） |
| 8 | 停止点の分類 | **(v) その他**（生成 C に無い静的な葉関数への間接呼び。調査 B5）。(i) 未応答の import、(ii) TLS、(iii) JIT、(iv) ロケールの**どれでもない**（まだ届いていない）。行なし 15 名は 1 回も呼ばれていない | 実測（§6） |
| 9 | 調査の「ATTACH は完走しない見込み」 | **完走しなかった**（実測）。ただし理由は調査が挙げた 3 つ（`GetModuleHandleA`、ロケール、`_tls_index`）ではなく、**手前の B5（`.pdata` に無い葉関数）** | 実測（§6.2 の 6） |
| 10 | 一般化したスクリプトの差分 | `lift/out-three/harness-two.diff`（`diff -u`。HEAD の `lift/harness/two/` との差。5 ファイル: `gen_two.sh`、`build_two_native.sh`、`main_two.c`、`two-native-run.sh`、新規 `merge_tables_n.py`）。`harness-two-p1.diff` は `patch -p1` 用の同内容（HEAD のコピーに当てて `harness-two/` と一致することを確認） | 実測（§7） |

つまり、**3 モジュールの機構は動き、Blend2D の `DllMain(ATTACH)` に入り、13 回の import（CRT の cookie 初期化、`api-ms-win-core-synch` と `fibers` の仮想モジュール、`InitializeCriticalSectionAndSpinCount`、`TlsAlloc`、`TlsSetValue`）は通ったあと、CRT の初期化の表の先頭の関数（`.pdata` に無い葉関数）が生成 C に無く、`fle_dispatch_miss` で止まった**。次の停止は未応答の import ではなく、生成の種の定義（調査 B5）に関わる。それを直したあとの停止点（`GetModuleHandleA`、ロケール、TLS、JIT のどれが先か）は**未測定**。

## 2. 手順（実測。実行した順）

`lift/` から実行。`TMPDIR=lift/out-three/tmp`（`/tmp` は使っていない）、`PYTHONDONTWRITEBYTECODE=1`。`<W>` は `/home/ubuntu/work/signature-studio-develop/lift`。

```
git archive HEAD | tar -x -C lift/out-three/src                                   # HEAD 2e94df28325173e4e18583303a490881be735785
cd lift/out-three/src/lift && CARGO_TARGET_DIR=<W>/out-three/target cargo build --release --offline -p fe-pe -p fe-lift
cp -r lift/harness/two/. lift/out-three/harness-two/                              # 一般化したスクリプトはこのコピーを編集
# 2 モジュール（新しい OUT。一般化したスクリプト、モジュール 1 つ）
LIFT=<W>/out-three/src/lift OUT=<W>/out-three/two2 FEBIN=<W>/out-three/target/release TAG=tbl2 FLE=…/binaries/win64/FLEngine_x64.dll \
  SEEDS=<W>/out-three/two2/dll/seeds-idx0-plus-initterm2.txt REAL=<W>/out-ext/dsp_ippv2_x64.dll bash out-three/harness-two/gen_two.sh rebase gen split merge
LIFT=<W>/out-three/src/lift TAG=tbl2 OUT=<W>/out-three/two2 JOBS=4 bash out-three/harness-two/build_two_native.sh
DLL=…/FLEngine_x64.dll OUT=<W>/out-three/out-two2 NATIVE_BIN=<W>/out-three/harness-two/two-native-run.sh TWO_NATIVE=…/two2/v-tbl2/native/two-native \
  TWO_REAL=…/two2/dll/dsp_ippv2_x64.2000000.dll FEPE=…/target/release/fe-pe bash out-three/src/lift/harness/a4-survey.sh        # 2 回（06:28:53Z、06:28:59Z）
# 3 モジュール
out-three/target/release/fe-pe rebase out-blend2d/Blend2D.dll 0x4700000 out-three/three/dll/Blend2D.4700000.dll
MODULES="dll|<W>/out-ext/dsp_ippv2_x64.dll|0x2000000|--seeds <W>/out-three/three/dll/seeds-idx0-plus-initterm2.txt|497dd4f7…c352|800;blend2d|<W>/out-blend2d/Blend2D.dll|0x4700000|--pdata|66634d24…66c7|800" \
  LIFT=… OUT=<W>/out-three/three FEBIN=… TAG=b2d FLE=… bash out-three/harness-two/gen_two.sh rebase gen split merge
LIFT=… TAG=b2d OUT=<W>/out-three/three JOBS=4 bash out-three/harness-two/build_two_native.sh                      # two-native 612,184,672 B
… TWO_REAL=<W>/out-three/three/dll/dsp_ippv2_x64.2000000.dll:<W>/out-three/three/dll/Blend2D.4700000.dll … bash out-three/src/lift/harness/a4-survey.sh   # 2 回（06:31:15Z、06:31:23Z）
GDBI_BIN=… GDBI_REAL=<dsp>:<blend2d> GDBI_FLE=… GDBI_OUT=…/run/roN-imports.txt GDBI_STDOUT=… GDBI_STDERR=… gdb -q -batch -x out-three/gdb/ro_imports3.py   # 読み取りだけ。2 回
python3 out-three/analysis/initializer_table.py … ; python3 out-three/analysis/regions.py                            # 静的な照合と算術
```

- 「手順の 3 モジュールの `gen_two.sh` の `rebase` 段」: `dll`（dsp_ippv2）は「exists」と出ず `fe-pe rebase` で作り直した（出力が無かったため。SHA-256 は `lift/out-avrt88/two/dll/` のものと一致）。`blend2d` は先に `fe-pe rebase` を手で実行してあったので「exists」と出て飛ばした（同じコマンド）。
- a4-survey は `lift/out-three/src/lift/harness/a4-survey.sh`（HEAD の `git archive`。作業ツリーの同名ファイルと `cmp` で同一）。a4-survey 自体は変えていない（`"$DLL"` だけを渡す。複数ファイルは `two-native-run.sh` の `TWO_REAL` の `:` 区切りで渡す）。
- 同じ秒に続けて走らせていない（秒単位のログ名の衝突を避けた。2 回の間は 6〜8 秒）。
- ビルドは新しい OUT で runtime のオブジェクトも含めて最初から作った（`build_two_native.sh` は作ったオブジェクトを飛ばす仕様。`lift/out-three/two2/native/` の FLEngine、runtime、harness のオブジェクトは 3 モジュールの OUT にコピーして共有した。ソースは同じ HEAD の `git archive`。コピーしたのは `native/fle`、`native/rt`、`native/h` の `.o` で、`realmod_table` と `common`、各モジュールのオブジェクトは TAG ごとに作り直した）。
- `gdb` は読み取りだけ（§5.1）。値を入れていない。runtime の応答も足していない。`lift/runtime`、`lift/crates`、`lift/harness` は編集していない（§8 の `git status`）。

## 3. 2 モジュール構成の再現（実測。新しい OUT `lift/out-three/two2/`）

HEAD の `git archive` を `lift/out-three/src/` に展開し、そこから（runtime のオブジェクトも含めて最初から）ビルドした。一般化したスクリプト（`lift/out-three/harness-two/`）を、モジュール 1 つ（従来と同じ指定）で使った。

| 項目 | 値 | 区分 |
|---|---|---|
| `git archive HEAD` | `lift/out-three/src/`（8,314,880 B のアーカイブ。作業ツリーの runtime、crates は読んでいない。`harness/a4-survey.sh` は作業ツリーと `cmp` で同一） | 実測 |
| fe-pe / fe-lift | `CARGO_TARGET_DIR=lift/out-three/target`、`cargo build --release --offline -p fe-pe -p fe-lift` | 実測 |
| FLEngine の生成 C（`fe-lift lift … --pdata`） | SHA-256 `43985d62321a90f562ce1b8144253e00af13df795e4190d26b6b8ce2537137a1`（期待値と一致。`gen_two.sh` が検査） | 実測 |
| リベース済み dsp_ippv2（基底 0x2000000） | SHA-256 `4fd255027525f2a2e56a93fe1cc7f95442861f788179a23ac8b0401b8f4fefc5`（`lift/out-avrt88/two/dll/` のものと一致。この OUT で `fe-pe rebase` から作り直した。DIR64 15,602、ABSOLUTE 22） | 実測 |
| 縮小 DLL の C（種 452 個） | SHA-256 `6035da1de5bde73f4a4c8f0226947622c3e0f51e06c767867eed5ac45e279dbf`（`two-module-after-0088.md` の値と一致）。関数 8,030、`bad_nodes 0` | 実測 |
| `common.c` | SHA-256 `33779308b032761247df4b431d6522820273929e2bcf726b860a350f88487d5f`（`lift/out-avrt88/two/` のものと一致）。分割 C（`fle-00..16.c`、`dll-00..10.c`）も `lift/out-avrt88/two/` と全ファイル `cmp` で一致 | 実測 |
| 重複 | 表の行 59,557 + 8,290、統合 67,587、両方に在る行 260。**`f_<VA>` の衝突 0**（`merge_tables_n.py` が検査）。両方に在る 260 行は、各表が持つ擬似 VA（`0x7FF900000000` 以上）の 260 行と一致する（2 モジュールの表では FLEngine 側 59,557 = `f_` 59,297 + 擬似 VA 260、dsp 側 8,290 = 8,030 + 260。3 モジュールの表で測った集計は §4.1。`f_` 以外で擬似 VA 未満の行は 0） | 実測（FLEngine と dsp の行数は §3 のビルドの表、擬似 VA の内訳は §4.1 と同じ表の集計） |
| 生成した `realmod_table`（1 モジュール） | `{ "dsp_ippv2_x64.dll", UINT64_C(0x2000000) }`、count 1 | 実測 |
| `two-native` | 571,231,232 B（`lift/out-avrt88/two` のビルドは 571,229,216 B。差は 2,016 B。原因は調べていない。ビルドパスの文字列（`-g`）の違いかもしれない（推測）） | 実測（サイズ）、推測（差の原因） |

a4-survey（一般化した `two-native-run.sh` 経由。gdb なし。2 回: 06:28:53Z と 06:28:59Z）:

```
stop   : fle: LoadLibraryW: module "blend2d.dll" is not a virtual module and not a name this environment is defined to lack (external modules are M2; no handle and no failure is invented for them)
         top of stack: 0x423990 f_423990
depth  : init_count=150 / 942   last unit: index=149 run_rank=137/895 proc=0x664260 in_progress
         started=137 completed=136 in_progress=1 of 895 runnable   all_units_done=no
funcs  : executed (distinct)=943  entries=26585
```

- **基準（`two-module-after-0088.md`）と一致**: init_count 150/942、last unit 149（proc 0x664260）、functions_distinct 943、function_entries 26,585、停止 `LoadLibraryW "blend2d.dll"`（実測）。
- 2 回の stdout の差は `calling entry … rsp=` の 1 行だけ（アドレス。ASLR）。stderr は完全に一致（実測。`diff`、`cmp`）。
- つまり、**一般化したスクリプトは 1 モジュールの場合に従来と同じ結果を出した**（実測）。

## 4. 3 モジュール構成の作り方と領域の衝突

**基底 0x4700000（終端 0x4a00000）は測定用の値で、決定ではない。** 決定 0086 は Blend2D.dll を実在モジュールに載せる方向だけを決めた。基底、表の上限、縮小の種、wasm の locals 上限などは決めていない。この文書の値は、1 回の測定を成立させるために置いたものである。

### 4.1 作り方（実測。`lift/out-three/three/`、TAG `b2d`）

| 項目 | 値 |
|---|---|
| 入力 | Blend2D.dll（`lift/out-blend2d/Blend2D.dll`）SHA-256 `66634d24412084b7541b4b27a11be8b0f4cb90d844c87e1ee5e5c3ecd66066c7`、2,065,920 B（期待値と一致。`gen_two.sh` が検査） |
| リベース | `fe-pe rebase … 0x4700000`（DIR64 1,763、ABSOLUTE 5、ブロック 18）。出力 `Blend2D.4700000.dll` SHA-256 **`24e01ec9295870db531e87637aca0aa6cb5c397409346e64a2928d706589b4cb`**、2,065,920 B。**往復（0x4700000 → 0x180000000）で抽出ファイルと全バイト一致**（`cmp`。調査 §8 の記録を再確認） |
| lift | `fe-lift lift Blend2D.4700000.dll … --pdata`（縮小しない。`--seeds` なし。**`--reloc-seeds` なし**）。関数 2,018、命令 398,879、`bad_nodes 0`、`unsupported_total 662`、生成 C 34,691,060 B、SHA-256 `f7022c97ae4efc914dd4e54f4d4c6adf4a0a6aeaed46030531a4a34ed1eb3a18`（調査 §3.2 と §8 の数と一致。調査のリベース後の `c_bytes` 34,691,060 と同じ） |
| 分割 | `split_mod.py` で `blend2d-00..02.c`（800 関数ずつ。合計 34,678,393 B）と `blend2d-tables.txt` |
| dsp_ippv2 | 2 モジュールのときと同じ（縮小。種 452。基底 0x2000000）。FLEngine の C は §3 と同じ |
| 表の統合 | `merge_tables_n.py`（`merge_tables.py` の N 個版）。FLEngine 59,557 行 + dsp 8,290 行 + Blend2D 2,278 行 → **統合 69,605 行**。**`f_<VA>` の衝突 0**（3 組の積集合がすべて空。`f_` の行は FLEngine 59,297、dsp 8,030、Blend2D 2,018。VA の範囲は FLEngine `0x403579`〜`0x13768a0`、dsp `0x2001000`〜`0x3ebc010`、Blend2D `0x4701000`〜`0x48b61e0`。擬似 VA（`0x7FF900000000` 以上）の行は各表 260 行で同一。`f_` 以外で擬似 VA 未満の行は 0）。`common.c` SHA-256 `c2f9bbb81a03fab1e0071ce13390e69595f031494815d0e90ded7b91370cd02e` |
| 強い定義の重複 | **解消した**。最終の `two-native` の中で、`R`、`X`、`FL`、`fle_dispatch_table`、`fle_dispatch_count`、`fle_resp_table`、`fle_resp_count`（と `fle_realmod_table`、`fle_realmod_count`）は、それぞれ**オブジェクト 1 個だけ**が強い定義を持つ（`nm`）。方法は dsp_ippv2 のときと同じ: `split_mod.py` が全部の分割 C で `R`/`FL`/`X` を `extern` にし、表を別ファイルに出し、`merge_tables_n.py` が `common.c` に 1 つの表と `R`/`FL`/`X` の定義を作る。リンクは `multiple definition` なしで通った |
| realmod の表 | `build_two_native.sh` が `modules.txt` から生成: `{ "dsp_ippv2_x64.dll", 0x2000000 }`、`{ "blend2d.dll", 0x4700000 }`、count 2。**runtime の `.c` は編集していない**（`lift/out-three/src/` は `git archive` のまま。表は `lift/out-three/three/v-b2d/native/realmod_table_gen.c`） |
| ビルド | コンパイルの警告 0（`log/three-build.txt`）。`two-native` **612,184,672 B**（2 GB 未満。2 モジュールの 571,231,232 B より 40,953,440 B 大きい）。最大の単一ファイルは `two-native` |

### 4.2 runtime が「実行ファイルの像」として扱う範囲（読み。ソースを読んだ。native では実測の裏づけあり）

- `fle_mem_set_image(base, size)` は `lift/runtime/fle_host.c:16-20` の 1 範囲（`fle_mem_img_lo`、`fle_mem_img_size`。既定は FLEngine の 0x400000 / 0x18c0000。`fle_host.c:13-14`）。**読む側は `fle_runtime.h:47-51` の `FLE_CHECKED_MEM`（opt-in、wasm 専用。`fle_runtime.h:30-32` の `#error` が native での定義を拒む）だけ**。
- `fle_mem_set_image` を呼ぶ箇所: `lift/harness/wasm/main.c:36` と `lift/harness/two/main_two.c:62`（`#ifdef __EMSCRIPTEN__` の中）。**native のビルドでは呼ばれず、`fle_mem_img_*` は読まれない**（読み。native の `fle_addr` は `fle_runtime.h:62` の恒等のマクロ）。
- 結論（読み）: native の測定では、3 つ目の範囲を **runtime に知らせる必要はない**。観察（実測）: 2 つ目の範囲（dsp_ippv2）も同じ扱いで動いており、Blend2D の DllMain の実行（呼び出しの連鎖と 13 回の import）も、`fle_mem_set_image` を呼ばずに進んだ。これは「読まれない」ことの観察で、範囲の検査が働かなかったことの直接の証明ではない（native に検査のコードが入っていないという読みと整合する）。
- wasm（`FLE_CHECKED_MEM` つき）では 1 範囲しか持てないので、3 つのイメージを許すには**範囲の拡張**（3 範囲化、または 3 つを覆う 1 範囲）が要る。一般化したコピー（`harness-two/main_two.c`）は、`#ifdef __EMSCRIPTEN__` の中で、**3 つのイメージを覆う 1 範囲**（最小の基底から最大の終端まで）を渡すよう書いた。**この分岐は emcc で作っていないので、ビルドも実行もしていない**（未確認）。覆う範囲の隙間（FLEngine と dsp の間 3.25 MiB など）も許してしまう点が 1 範囲の近似の限界（読み）。`a4_set_image` は FLEngine の像だけを渡す（a4 は FLEngine の InitContext だけを読む。`main_two.c` の既存のコメント）。

### 4.3 領域の衝突の算術（実測の値と導出）

定数は `lift/out-three/src/lift/`（HEAD の `git archive`）の行番号。MiB は 2^20 B。

| 領域 | 範囲 | 出典 | 区分 |
|---|---|---|---|
| FLEngine のイメージ | 0x400000〜0x1cc0000（0x18c0000 = 24.75 MiB） | `image: base=0x400000 size=0x18c0000`（実行ログ）。`fle_host.c:13-14` の既定値と同じ | 実測 |
| dsp_ippv2 のイメージ（縮小の C をリンクするが、PE は全体を置く） | 0x2000000〜0x461f000（0x261f000 = 38.12 MiB） | `real module image: base=0x2000000 size=0x261f000`（実行ログ） | 実測 |
| **Blend2D のイメージ（測定用の値）** | **0x4700000〜0x4a00000**（0x300000 = 3.00 MiB） | `real module image: base=0x4700000 size=0x300000`（実行ログ） | 実測 |
| FLEngine 終端 → dsp 基底 | 0x340000 = 3.25 MiB の空き | 算術 | 導出 |
| dsp 終端 → Blend2D 基底 | 0xe1000 = 921,600 B = 0.88 MiB の空き | 算術 | 導出 |
| 3 つの像の対ごとの重なり | **なし**（FLEngine∩dsp、dsp∩Blend2D、FLEngine∩Blend2D のどれも偽） | 算術（`lift/out-three/analysis/regions.py`、出力 `log/regions.txt`） | 導出 |
| 3 つの SizeOfImage の合計 | 0x41df000 = 65.87 MiB | 算術 | 導出 |
| wasm の `GLOBAL_BASE` | 0x10000000（268,435,456。256 MiB） | `lift/harness/wasm/build.sh:35` | 読み |
| Blend2D 終端 → `GLOBAL_BASE` | 0xb600000 = **182.00 MiB の空き**。Blend2D の終端は `GLOBAL_BASE` 以下（native では使わない値。wasm は未実行） | 算術。`harness/image.c:95-96`（wasm 分岐: `ibase + size > __global_base` なら失敗） | 導出（wasm の部分は読み） |
| Blend2D の終端は 4 GiB 未満 | 0x4a00000 < 0x100000000 | 算術 | 導出（wasm32 の `a >> 32` 検査 `fle_runtime.h:43-46` に掛からない読み） |
| 基底の検査（`fle_realmod.c:57`） | 64 KiB の倍数（0x4700000 & 0xFFFF = 0）、0x400000 でない、`FLE_MODULE_HANDLE_BASE`（0x7FF800000000。`fle_modules.h:14`）未満。**通った**（`table_check` が停止しなかった。実行が `LoadLibraryW "blend2d.dll"` を解決して DllMain に入った） | `fle_realmod.c:15`（`RM_MAX 4u`）、`:42-67` | 実測（検査の通過）、読み（条件の中身） |
| 仮想モジュールのハンドル域 | 0x7FF800000000 + 0x10000 × N（`fle_modules.h:14-15`）。N は 32 まで使用済み（avrt = 31、PowrProf = 32。`two-module-after-0088.md`）→ 最大 0x7FF800200000。実在モジュールのハンドルは基底そのもの（0x2000000、0x4700000）なので、**この域とは 3 桁以上離れていて重ならない** | 算術 | 導出 |
| 応答表の擬似 VA | 0x7FF900000000 + 0x10 × (k+1)（k は 260 行。`imports_table.rs:323`）。生成 C の `fle_dispatch_table` に載る擬似 VA 行は各モジュールの C で 260 行ずつ同一（統合後 260 行） | `lift/crates/fe-lift/src/imports_table.rs:323`、統合の集計 | 読み、実測（行数） |
| プロセスヒープのトークン | 0x7FF700000000（`fle_sys.h:37`） | 読み | 読み |
| その他のトークン域 | 0x7F990000〜0x7FF00000 台（ウィンドウ、DC、GDI、カーソル、モニタ。`fle_win.h:25`、`fle_gdc.h:23`、`fle_gdi.h:23`、`fle_cursor.h:28`、`fle_loadimage.h:44`、`fle_monitor.h:24`）と、0x7FFA60000000〜0x7FFA80000000+（`fle_vol.h:24`、`fle_vmap.h:15`、`fle_event.h:17`）。**3 つの像（最大 0x4a00000）より 0x7F990000 − 0x4a00000 = 0x7af90000 = 1.92 GiB 上**で重ならない | 読み、算術 | 導出 |
| 仮想メモリ（native） | `VirtualAlloc` に `want` なしは `mmap(NULL, …)`（`fle_sys.c:527`）で、カーネルが選ぶ高位のアドレス。`want` ありは `MAP_FIXED_NOREPLACE`（`fle_sys.c:518`）なので 3 つの像の範囲を要求すると失敗する（使用中）。像の mmap 自体も `MAP_FIXED_NOREPLACE`（`harness/image.c:101-102`）で、**Blend2D の像の mmap は成功した**（実行ログ。`mmap at ImageBase failed` なし） | 読み、実測（像の mmap の成功） | 実測 |
| アリーナ（wasm の `FLE_VMEM_ARENA`） | 512 MiB（`fle_sys.c:373`）。`aligned_alloc(65536, 512 MiB)`（`:393`）で `GLOBAL_BASE` より上のホストヒープに取る。固定のアドレスではない。native では使わない | 読み | 読み（wasm は未実行） |
| スクラッチ | **runtime にスクラッチ領域は無い**（`lift/runtime`、`lift/harness` の `scratch` の検索で一致するのは `harness/oracle/run_diff2.py:44` の `SCRATCH = 0x20000000` だけ。これは別のオラクルのハーネスで、この構成では使わない）。依頼文の「スクラッチ」に当たる runtime の定数は見つからなかった | 実測（検索） | 実測 |
| ハーネスのスタック | 16 MiB（`harness/boot.h:14`）。native は `mmap(NULL, …)`（`main_two.c`）で、固定のアドレスではない。実行ログの `rsp=0x78a8dd9fffc0`、`0x77bff45fffc0`（高位） | 実測 | 実測 |
| アドレス空間の上限 | `RLIMIT_AS` 4 GiB（`harness/boot.h:13`、`main_two.c` の `setrlimit`）。像 3 つで 65.87 MiB | 読み、算術 | 導出 |
| `vm.mmap_min_addr` | 65,536（ホスト。`/proc/sys/vm/mmap_min_addr`）。3 つの基底はすべてこれより上 | 実測 | 実測 |

読み: 3 つのイメージ、トークン域、仮想ハンドル域、`GLOBAL_BASE` のどれとも重ならない（導出）。native の実行でも像の mmap は 3 つとも成功した（実測）。

## 5. 3 モジュール構成の a4 の行と停止

a4-survey（gdb なし、仮の値なし、runtime の応答追加なし。2 回: 06:31:15Z と 06:31:23Z。`lift/out-three/out-three/a4/`）:

```
stop   : fle: indirect call to 0x48a57f8 is not a known function entry
         top of stack: 0x48b5b90 f_48b5b90
depth  : init_count=150 / 942   last unit: index=149 run_rank=137/895 proc=0x664260 in_progress
         started=137 completed=136 in_progress=1 of 895 runnable   all_units_done=no
         entry check: started_but_not_entered=0 runnable_entered_beyond_init_count=0
funcs  : executed (distinct)=960  entries=26604
```

- **2 回の結果は同一**（stdout の差は `calling entry … rsp=` の 1 行だけ、stderr は `cmp` で一致。実測）。
- 2 モジュールのとき（§3）から変わったもの: 停止が `LoadLibraryW "blend2d.dll"` から **`fle_dispatch_miss`（`indirect call to 0x48a57f8 is not a known function entry`）** に移った。init_count 150/942、last unit 149（proc 0x664260）、started 137、completed 136 は同じ。functions_distinct 943 → **960**（+17）、function_entries 26,585 → **26,604**（+19）（実測）。
- ハーネスの出力（`abort kind: fle_dispatch_miss`、`dispatch target: 0x48a57f8`）。C のシャドウスタック（`call depth at stop: 17`。先頭 10 行。`lift/out-three/out-three/a4/20261024T063115Z-native.txt`）:

```
#0 0x48b5b90 f_48b5b90 guest-return=0x48ab9af
#1 0x48ab984 f_48ab984 guest-return=0x489e084
#2 0x48a5928 f_48a5928 guest-return=0x489e084
#3 0x489e050 f_489e050 guest-return=0x489e2d6
#4 0x489e2b4 f_489e2b4 guest-return=0x489e4c4
#5 0x489e264 f_489e264 guest-return=0x489e4c4
#6 0x489e450 f_489e450 guest-return=0xcb000000
#7 0x489e584 f_489e584 guest-return=0xcb000000
#8 0x423990 f_423990 guest-return=0x44db70
#9 0x44db40 f_44db40 guest-return=0x653a56
（以下 #10〜#16 は 2 モジュールの停止と同じ f_6539e0、f_664190、f_664260、f_4120c0、f_412160、f_41d8f0、f_1376820 と読む。この表の先頭 10 行しか報告に出ない）
```

- 基底 0x4700000 の VA から、Blend2D の元の VA（基底 0x180000000）は `VA − 0x4700000 + 0x180000000`: `0x489e584` = `0x18019e584`（エントリ = `_DllMainCRTStartup`）、`0x48b5b90` = `0x1801b5b90`、`0x48a57f8` = `0x1801a57f8`（導出）。
- `f_489e584` の戻り先は `0xcb000000`（`FLE_CB_RETURN_ADDRESS`。`fle_call_guest` の番兵。`fle_realmod.c:117` の `call_dllmain` が呼ぶ）、その下に FLEngine の LoadLibraryW のラッパー `f_423990` ← `f_44db40`（2 モジュールのときの停止点と同じ連鎖）。**Blend2D の DllMain(ATTACH) には入った。戻っていない**（スタックに `f_489e584` が残ったまま停止。実測）。

### 5.1 読み取りだけの gdb（実測。`lift/out-three/run/ro1-*`、`ro2-*`）

`lift/out-ctype87/gdb/ro_imports.py` を `lift/out-three/gdb/ro_imports3.py` にコピーして使った。変更は 3 点だけ: (1) `GDBI_REAL` を `:` で区切った複数ファイルとして `two-native` の argv[2] にそのまま渡す（`main_two.c` のコピーが分ける）、(2) Blend2D の DllMain の入口 `f_489e584` に印を出す breakpoint を足した、(3) `call_dllmain` の `idx` を記録する。**値を書く機構は無い**（`return`、`set var`、`call` を使っていない。`gdb.execute` は元のコピーと同じ集合で、`signal SIGABRT` を停止時に 1 回送るだけ）。同じ環境変数（`GDBI_BIN`、`GDBI_REAL`、`GDBI_FLE`、`GDBI_OUT`（存在しないこと）、`GDBI_STDOUT`、`GDBI_STDERR`）で 2 回実行（`ro1`、`ro2`）。**import のログは 2 回で `cmp` により全バイト一致**。

| 項目 | 値 | 区分 |
|---|---|---|
| dsp_ippv2 の `call_dllmain(idx=0, 1)` | 戻り値 **0x1**（import 160 回。2 モジュールのときと同じ）。`LoadLibraryW` の戻り **0x2000000** | 実測 |
| 通し番号 1525（`LoadLibraryW`、引数の名前 `Blend2D.dll`） | 呼び出し元は `f_423990` ← `f_44db40`（`ret=0x44db70`）。直前の 1523 `SetDllDirectoryW`、1524 `SetErrorMode(0x8000)` | 実測 |
| **`call_dllmain(idx=1, reason=1)`（Blend2D の ATTACH）** | **入った**（`call_dllmain entered idx=1 reason=1`。`f_489e584` の入口 breakpoint が `rcx=0x4700000 rdx=1 r8=0` で発火 = hinstDLL は基底、reason 1、lpvReserved NULL）。**戻らなかった**（FinishBreakpoint は out of scope。`DLLMAIN-RETURN … did not return normally`）。**eax は得られていない**（戻っていない）。`fle_realmod.c:144`（`ST_LOADED`）には**到達していない** | 実測 |
| **LoadLibraryW が 0x4700000 を返したか** | **返していない**（ATTACH の途中で停止。`LOADLIBRARY-RETURN`（`fle_load.c:153` の直後）は dsp_ippv2 の 1 回だけ。`#1525 fle_imp_LoadLibraryW: out of scope`）。したがって FLEngine の `f_664190` はハンドルを受け取っていない | 実測 |
| Blend2D の ATTACH の中で起きた import | **13 回**（通し番号 1526〜1538）。`GetSystemTimeAsFileTime`、`QueryPerformanceCounter`、`GetCurrentThreadId`、`GetCurrentProcessId`、`QueryPerformanceCounter`（ここまでが `__security_init_cookie`）、`LoadLibraryExW("api-ms-win-core-synch-l1-2-0")` → `0x7ff8001b0000`（仮想）、`GetProcAddress(…, "InitializeCriticalSectionEx")` → 0、`InitializeCriticalSectionAndSpinCount(0x48f1618, 0xfa0)`、`LoadLibraryExW("api-ms-win-core-fibers-l1-1-1")` → `0x7ff8001c0000`、`GetProcAddress(…, "FlsAlloc")` → 0、`TlsAlloc`、`GetProcAddress(…, "FlsSetValue")` → 0、`TlsSetValue(2, 0x48f1590)`。**全部が名前付き停止なしで通った**。その後、import を呼ばずに `fle_dispatch_miss` | 実測 |
| 停止の C のスタック | `fle_dispatch_miss(va=76175352 = 0x48a57f8)` ← `fle_dispatch` ← `f_48b5b90`（`blend2d-02.c:24611`）← `f_48ab984`（`:14368`）← `f_48a5928`（`:7020`）← `f_489e050`（`blend2d-01.c:248255`）← `f_489e2b4`（`:248465`）← `f_489e264`（`:248448`）← `f_489e450`（`:248607`）← `f_489e584`（`:248689`）← `fle_dispatch(va=76146052 = 0x489e584)` ← `fle_call_guest`（`fle_callback.c:54`）← `call_dllmain(idx=1, reason=1)`（`fle_realmod.c:117`）← `fle_realmod_load(idx=1)`（`:138`）← `load_by_name`（`fle_load.c:152`）← `fle_imp_LoadLibraryW`（`:184`）← `f_423990` ← `f_44db40` ← `f_6539e0` ← `f_664190` ← `f_664260` ← `fle_dispatch(va=6701664)` ← `f_4120c0` ← `f_412160` ← `f_41d8f0` | 実測 |
| 停止時のゲストのレジスタ | `rcx=0x48b8a40`、`rdx=0x48b8b40`、`r8=0x48b84b8`、`r9=0x48b84c0`、`rsp=0x7ffff7bff9e8`、`rax=0x48a57f8` | 実測 |

### 5.2 停止した呼び出しの読み（読み。リベース済みファイルと生成 C を読んだ。静的）

- `f_48b5b90` は 1 命令だけ: `jmp rax`（`0x48b5b90`。元の VA `0x1801b5b90`）。生成 C は `fle_dispatch(R[0]); return;`（`blend2d-02.c:24611`）。調査 §3.4 は、元の `0x1801b5b90` の `jmp rax` を「`_pRawDllMain` に当たる `jmp rax`で、スロット `0x1801b7308` から指される」と書いている（元の `0x1801b7308` = 今回の `0x48b7308`）。一方、今回のリベース済みファイルの Load Config の **`GuardCFDispatchFunctionPointer` は `0x48b7308`**（実測。`GuardCFCheckFunctionPointer` は `0x48b72f8`、`GuardFlags 0x100`）で、**調査が `_pRawDllMain` のスロットと呼んだスロットと同じアドレスを指す**。スロットの中身は `0x48b5b90`（実測。リベース済みファイルの値）。つまりこのスロットは、Load Config が **MSVC の CFG（Control Flow Guard）の間接呼びの踏み台** を入れる場所として名指すものと一致する（読み。ポインタの一致を見ただけで、PE Format の仕様書での確認はしていない）。調査 §6 は「`_pRawDllMain` のスロットは `jmp rax` で、`[0x1801b7478]`（今回 `0x48b7478`）が NULL なので呼ばれない」と読んだ。`0x48b7478` が 0 であることは実測で一致する（リベース済みファイルから読んだ値。`0x48b7478 -> 0x0`）。ただし**実測では、この `jmp rax` に ATTACH の中の別の呼び出し（`f_48ab984` の表のループ）が入って止まった**。調査の「呼ばれない」は `_pRawDllMain` 経由の経路についての読みで、同じ踏み台を使う別の呼び出し点（生成 C に 61 か所）には当てはまらなかった、と読める（推測。`_pRawDllMain` を呼ぶ経路自体は今回測っていない）。この踏み台を通る呼び出しは生成 C に **61 か所**（うち 47 か所が `R[4] += 8` つきの `call`、残りは `jmp`。`grep`）。
- 呼び出し点は `f_48ab984`（`0x48ab984`）の `0x48ab9a9: call qword ptr [rip + 0xb959]`（スロット `0x48b7308` = 踏み台。ファイル上の値は `0x48b5b90`）。ループ: `rbx = rcx`（開始）から `rsi = rdx`（終端）まで 16 バイトずつ、`rax = [rbx]` が非 0 なら `call [踏み台]`（rax = 先頭 8 バイトの値）し、`al` が 0 でなければ次へ進む（読み）。呼び出し元 `f_48a5928`（`lea rdx,[…]; lea rcx,[…]; jmp 0x48ab984`）が `rcx = 0x48b8a40`、`rdx = 0x48b8b40` を渡す（実測: 停止時のレジスタと一致）。`f_489e050`（`0x489e050`）が `f_48a5928` を呼ぶ。`0x48b8a40`〜`0x48b8b40` は 16 エントリ × 16 バイトの表（`.rdata`）で、**CRT の初期化の表を走る形**に読める（調査 §6 の `.xi` の 4 項目 + `.xc` の 1 項目とは別の表。**どの CRT の表か、各エントリが何かは読み切っていない**）。根拠はループの形だけ: 先頭の 8 バイトが非 0 なら呼び、戻り `al` が 0 のとき（`test al,al; je 0x48ab9bc`）ループを抜ける。抜けた後（`0x48ab9c6` 以降）は、逆アセンブルで `rbx` を 16 バイト刻みで戻しながら、**2 列目**（`[rbx]` を `rbx -= 8` した位置から読む形）が非 0 なら `xor ecx,ecx` で `call [踏み台]` する形が見える（`0x48ab9d1〜0x48ab9db`）。つまり「初期化関数が偽を返したら、ここまでの終了関数を逆順に呼ぶ」組（{初期化、終了} の 16 バイト）に読める（読み。このループの読みは `0x48ab9a1〜0x48ab9db` の 20 命令ほどだけを根拠にしており、残りは読んでいない）。
- 先頭の列の値（12 個が非 0）と、生成 C の dispatch 表（Blend2D の `f_` 行 2,018）との突き合わせ（実測。`lift/out-three/analysis/initializer_table.py`、出力 `log/initializer-table.txt`）:

| 表のスロット | 先頭の値 | dispatch 表にあるか | `.pdata` の関数開始か | 調査の `reloc-not-function.txt`（58 個）にあるか |
|---|---|---|---|---|
| `0x48b8a40`（**停止したエントリ**） | **`0x48a57f8`**（元 `0x1801a57f8`） | **無い** | **いいえ** | **はい** |
| `0x48b8a50` | `0x48a5844` | ある | はい | いいえ |
| `0x48b8a60` | `0x48a9cf4` | ある | はい | いいえ |
| `0x48b8a70` | `0x48a5830`（元 `0x1801a5830`） | **無い** | いいえ | はい |
| `0x48b8a80`〜`0x48b8ae0`（6 件） | `0x48a9ef0`、`0x48ab95c`、`0x48a6cb0`、`0x48a7ef0`、`0x48ab68c`、`0x48ab2c8` | ある | はい | いいえ |
| `0x48b8b20` | `0x48a5830` | **無い** | いいえ | はい |
| `0x48b8b30` | `0x48a580c` | ある | はい | いいえ |
| 先頭が 0 の 4 件（`0x48b8aa0`、`0x48b8af0`、`0x48b8b00`、`0x48b8b10`） | 0 | （ループが飛ばす） | — | — |

  先頭が非 0 の 12 個のうち **9 個は dispatch 表にあり、3 個（`0x48a57f8`、`0x48a5830` が 2 つ）は無い**。この 3 個は、調査 §3.2・§10.3 の **B5**（`.pdata` に無い葉関数 58 個）の中の値と一致する（実測: 3 個とも `reloc-not-function.txt` の元の VA を 0x4700000 に直した値と一致。`reloc-not-function` 58 個のうち生成 C の dispatch 表にあるのは 10 個）。`0x48a57f8` の命令は `lea rax,[rip+0x4a971]; mov [rip+0x4c0b2], rax; mov al,1; ret`（実測: 逆アセンブル）、`0x48a5830` は `mov al,1; ret`（同）で、どちらも `.pdata` を持たない短い葉関数。**入口として正しいかの確認ではない**（命令の形を見ただけ）。
- つまり停止は、`fe-lift lift … --pdata` が `.pdata` の関数開始と export、エントリだけを種にし、`--reloc-seeds` を付けなかったために、**ATTACH のループが CFG の踏み台経由で呼ぶ関数が生成 C に無かった**ことによる（読み。種の定義と停止の対応は、dispatch 表の突き合わせ（実測）とループの読みから）。調査 §10.3 の B5 が「入れないと vtable の 26 エントリが生成 C に無く、呼ばれると `fle_dispatch_miss`」と予想した形が、vtable ではなく **CRT の初期化の表**で先に起きた。
- ここで 1 つ目の `fle_dispatch_miss` が出た。ループは先頭のエントリで止まったので、**残りのエントリ（`0x48a5830` など）がどうなるかは未測定**。

## 6. 停止点の分類と、次に必要な作業（事実）

### 6.1 分類（依頼の (i)〜(v)）

停止は `fle_dispatch_miss`（`indirect call to 0x48a57f8 is not a known function entry`）。

| 分類 | 該当するか | 根拠 | 区分 |
|---|---|---|---|
| (i) 未応答の import（名前付き停止。調査 B8 の行なし 15 名のどれか） | **いいえ** | 停止メッセージは API 名を持たない `fle_dispatch_miss`。Blend2D の ATTACH の中の import 13 回はすべて通った（`GetModuleHandleA`、SRWLock、条件変数、`CreateThread`、`IsProcessorFeaturePresent`、`TlsFree` などの行なし 15 名は**まだ 1 回も呼ばれていない**。ログの 1526 以降にこれらの名前は無い） | 実測 |
| (ii) TLS（B6。`_tls_index`、`gs:[0x58]`） | **いいえ（まだ届いていない）** | 停止の前の 13 回の import に `gs:[0x58]` を読む経路は出ない（`gs:[0x58]` は import ではなく生成 C の `fle_gs_ld8` で、ログに出ない）。停止した呼び出しは CRT の初期化の表を走るループ（`f_48ab984`）で、`fle_gs_ld8` を使う関数ではない（生成 C の `f_48ab984`、`f_48b5b90`、`f_489e050` に `fle_gs` は出ない（`awk` と `grep`）。生成 C 全体の `fle_gs_ld8` は 29 か所で、内訳は `gs:[0x58]` が 26、`gs:[0x30]` が 2、`gs:[0x10]` が 1（`grep -o`）。調査 §6 は「`gs:[0x58]` の読みが 13 関数 29 か所」と書いており、**生成 C の `gs:[0x58]` 26 か所とは数が合わない**（29 は `fle_gs_ld8` 全体の数と一致する。数え方の違いか、生成 C に出ない箇所があるのかは調べていない）。`TlsAlloc` は 1 回（通し番号 1536）と `TlsSetValue(2, …)`（1538）が通った（`FlsAlloc` が NULL を返した直後に `TlsAlloc` が呼ばれた順。CRT の FLS 代替と読むが、読みで、暗黙の TLS ではない）。**暗黙の TLS が実際に踏まれるかは、この停止より先で、未測定** | 実測（import の列、`fle_gs_ld8` の個数）、読み（`fle_gs_ld8` の位置） |
| (iii) JIT（`VirtualAlloc` の flAllocationType 0x20003000、`fle_dispatch_miss`） | **形は `fle_dispatch_miss` だが、JIT ではない** | 停止の `fle_dispatch_miss` の対象 `0x48a57f8` は**リベース済み PE の `.text` の中**（`0x4701000`〜`0x48b6205`）にあり、`.rdata` の初期化の表に載った**静的に存在する関数**（`lea rax,[…]; mov [..],rax; mov al,1; ret` の 4 命令）。実行時に作られた機械語ではない。`VirtualAlloc`、`GetLargePageMinimum` は Blend2D の ATTACH の中で 1 回も呼ばれていない（ログの 1526 以降）。JIT のメモリ確保に届く前に止まった | 実測（対象の位置、import の列）、読み（命令の形） |
| (iv) ロケール（範囲 D の外） | **いいえ（まだ届いていない）** | `GetStringTypeW`、`MultiByteToWideChar`、`LCMapStringW` は Blend2D の ATTACH で 1 回も呼ばれていない（`GetStringTypeW` 1 回と `MultiByteToWideChar` 6 回はログの dsp_ippv2 の DllMain の中だけ） | 実測 |
| **(v) その他** | **これ**: **生成 C に無い静的な関数への間接呼び**（`.pdata` に無い葉関数。調査 B5） | §5.2: ATTACH のループが CFG の踏み台（`f_48b5b90` = `jmp rax`）経由で 16 バイト刻みの表の先頭のポインタを呼び、その先頭エントリ（`0x48b8a40`）の `0x48a57f8` が (a) `.pdata` の関数開始でなく、(b) 調査の `reloc-not-function.txt` の 58 個の 1 つで、(c) `--pdata` だけの生成 C の dispatch 表に無い。**これは調査 B5 の「`--pdata` だけでは `.pdata` に無い葉関数が生成 C に入らない」が、Agg2D の vtable より前の、ATTACH の CRT の初期化の表で現れた** | 実測（dispatch 表との照合、レジスタ、スタック）、読み（表の役割） |

### 6.2 次に必要な作業（事実の並べ。決めない）

以下は、この測定が示した事実から言えることだけを並べる。順序や採否は決めない。

1. **生成 C に入っていない葉関数への間接呼びが、Blend2D の ATTACH の最初の初期化表で出た。** 3 エントリ（`0x48a57f8`、`0x48a5830` が 2 つ）が dispatch 表に無い。調査 §3.2 の測定では、`--pdata` に reloc の開始でない 58 個を足すと「関数 2,066、`bad_nodes 0`、`unsupported_total 665`」になる（今回はこの足し方で作っていない。**足して再ビルドすると停止がどこへ移るかは未測定**）。この 58 個を種にすることは、決定 0074 が「採らない」とした「開始でない値を seed にする」に当たる（調査 B5）。
2. **今回の停止で未実行のものは多い。** Blend2D の ATTACH が `.xi`（4 項目）と `.xc`（1 項目）に届く前に止まったため、調査 §6、§7 が「完走しない見込み」の理由とした項目（`.xc` の `GetModuleHandleA`（行なし）、ロケール、TLS の `_tls_index`、SRWLock、条件変数、`CreateThread`）は、**このビルドでは 1 つも測れていない**（上の分類で「まだ届いていない」とした項目は、停止が先へ動いた後の測定でないと判定できない）。
3. **dsp_ippv2 と同じ形で通った部分**: CRT の `__security_init_cookie`（5 回の import）、`api-ms-win-core-synch-l1-2-0` と `api-ms-win-core-fibers-l1-1-1`（決定 0081 の仮想モジュール N = 27、28 に当たる値。ログの戻り `0x7ff8001b0000`、`0x7ff8001c0000`）に対する `LoadLibraryExW` と `GetProcAddress`（NULL）、`InitializeCriticalSectionAndSpinCount`、`TlsAlloc`、`TlsSetValue`（実測）。
4. **3 モジュールの機構そのものは動いた**: 3 つの像の mmap、`fle_realmod` の表（count 2 の検査）、`LoadLibraryW("Blend2D.dll")` → `blend2d.dll` の名前解決 → `fle_realmod_load` → `call_dllmain(idx=1)` → Blend2D のエントリ `f_489e584`（実測）。生成 C の dispatch 表の統合（69,605 行）と強い定義の重複の解消も通った。
5. **FLEngine の側の経路は、停止の前後で変わっていない**: 2 モジュールの停止と同じ連鎖（`f_423990` ← `f_44db40` ← `f_6539e0` ← `f_664190` ← `f_664260`）で `LoadLibraryW` に入り、ATTACH の途中で止まった。`f_664190` が `AggCreateInstance` を引く行（`GetProcAddress`）には**届いていない**。
6. 調査 §10.2 の 7（「ATTACH は完走しない見込み」）の**実測としての確認**: ATTACH は完走しなかった（実測）。ただし**理由は調査が挙げた 3 つ（`GetModuleHandleA`、ロケール、`_tls_index`）のどれでもなく、4 つ目**（B5 の葉関数。CRT の初期化の表の先頭）だった。調査の 3 つの理由が実際に起きるかは、この停止が動いた後でないと分からない（未測定）。
7. この測定の値（基底 0x4700000、縮小なしの `--pdata`、`--reloc-seeds` なし）は**測定用**。決定ではない。種の定義（B5）、`--reloc-seeds` を使うか、`--pdata` に 58 個を足すか、は別の判断で、この文書は決めない。

## 7. 一般化したスクリプトの差分（実測）

`lift/harness`、`lift/runtime`、`lift/crates` は編集していない。一般化したスクリプトは `lift/out-three/harness-two/`（HEAD の `lift/harness/two/` の全ファイルのコピー + 編集 + 新規 1 ファイル）。差分:

- **`lift/out-three/harness-two.diff`**（`diff -ruN src/lift/harness/two harness-two`。410 行。5 ファイル。後で PM が取り込みを判断する）。
- `lift/out-three/harness-two-p1.diff`: 同じ内容を `a/`、`b/` の接頭辞で出したもの。HEAD の `lift/harness/two/` のコピーに `patch -p1` を当てると `harness-two/` と `diff -r` で一致することを確認した（実測）。

| ファイル | 変更 |
|---|---|
| `gen_two.sh` | 環境変数 `MODULES`（`label\|file\|base\|lift-args\|expect-sha256\|funcs-per-part` を `;` で並べる）で任意個の実在モジュールを扱う。**`MODULES` を省略すると従来の 1 モジュール（`REAL`、`BASE`、`SEEDS`）と同じ指定になり、§3 の 2 モジュールの再現で、分割 C と `common.c` が従来の出力と一致した**。ラベル `dll` は従来のファイル名（`gen/real.c`、`src/dll-NN.c`、`seeds.txt`）、それ以外の `L` は `gen/L.c`、`src/L-NN.c`、`seeds-L.txt`。モジュール一覧を `v-TAG/modules.txt` に書く。`LIFT` を環境変数で変えられる（`git archive` の木から作る）。`set -o pipefail` を追加。表の統合は `merge_tables_n.py` |
| `merge_tables_n.py`（新規） | `merge_tables.py` の N 個版。2 個のときの `common.c` は従来と同一（§3 で SHA-256 一致）。同じ VA が複数の表にあれば `f_<VA>` の衝突として数えて表示し、失敗する（擬似 VA の領域は除く） |
| `build_two_native.sh` | モジュールを `modules.txt` から読む。`$V/src/*-NN.c` のすべてをコンパイルしてリンク。`realmod_table` を `modules.txt` から**生成**する（名前 = ファイル名の小文字、基底 = そのエントリの基底。`dll` は従来どおり `dsp_ippv2_x64.dll`）。`realmod_table.o` は TAG ごと（`obj-h/`）。`LIFT` を環境変数で変えられる。`BASE` の環境変数は不要になった |
| `main_two.c` | `argv[2]` を `:` 区切りの複数ファイルとして受け取る（最大 4。`RM_MAX` と同じ）。ファイルが 1 つのときの出力は従来と同じ。各ファイルの基底が `fle_realmod_table` のどれかに一致すること、表の個数がファイルの個数と等しいことを検査する。wasm 分岐（`#ifdef __EMSCRIPTEN__`）で、3 つの像を覆う 1 範囲を `fle_mem_set_image` に渡す（**未ビルド・未実行**） |
| `two-native-run.sh` | コメントだけ追加（`TWO_REAL` に `:` 区切り）。実行内容は同じ |

`a4-survey.sh`、`image.c`、`a4.c`、`trace_native.c` は変えていない（`a4-survey.sh` は `"$DLL"` だけを渡し、実在モジュールのファイルは `two-native-run.sh` が足すので、複数ファイルに一般化するのは `two-native-run.sh` と `main_two.c` で足りた）。`build_two_wasm.sh`、`chain.sh`、`run_node_two.cjs`、`gdb_*.py`、`patches_*.py` などは**一般化していない**（コピーのまま。wasm は使っていないので `build_two_wasm.sh` は 1 モジュール前提のまま）。

## 8. 限界、未確認

- **基底 0x4700000、`--pdata` のみ、縮小なし、`--reloc-seeds` なしは測定用の値と構成で、決定ではない。** 決定 0086 は方向だけを決めた。別の基底、別の種の定義にすれば停止点は変わりうる。
- **新しい停止は直していない**（範囲外）。停止は `fle_dispatch_miss`（`0x48a57f8`）。`--pdata` に `reloc-not-function` の 58 個を足したとき、またはほかの種の足し方で、停止がどこへ移るかは**測っていない**（足したビルドを作っていない）。
- Blend2D の ATTACH は 13 回の import しか実行していない。調査が「完走しない見込み」の理由とした `GetModuleHandleA`、SRWLock、条件変数、`CreateThread`、ロケール（`GetStringTypeW` など）、`_tls_index`、JIT（`VirtualAlloc`）は**この構成では 1 つも実行されていない**（未測定）。
- 停止の呼び出しが「CRT の初期化の表」を走るループという読みは、リベース済み PE のバイト列（capstone）と生成 C を読んだもの（読み）。**どの CRT の表か、表の各エントリが何か**は読み切っていない。`0x48a57f8` の 4 命令の関数の意味（`lea rax,[…]; mov […],rax` は表のスロットにアドレスを書くだけ）は逆アセンブルを見ただけ。
- CFG の踏み台という読みは、Load Config のポインタ（`GuardCFDispatchFunctionPointer`）と一致する事実に基づく。PE Format の仕様書では確認していない。
- 調査 §6 は `gs:[0x58]` の読みを「13 関数 29 か所」と書くが、生成 C では `fle_gs_ld8` 29 か所のうち `gs:[0x58]` は 26 か所（§6.1 の (ii)）。数が合わない理由は調べていない。
- wasm は使っていない（`emcc` を使っていない。native のみ）。`main_two.c` の wasm 分岐、`FLE_CHECKED_MEM`、`GLOBAL_BASE` に対する 3 つの像の収まり（182.00 MiB の余裕）は、算術と読みだけで、wasm では**未実行**。wasm の locals の上限（Blend2D は調査で 4 関数が 50,000 を超える近似値）はこの作業で見ていない。
- 実機 Windows での挙動は未確認（ホストは Linux）。ゲストが実機と違う経路を通るかどうかは、この測定の範囲外。
- ビルドの再現性: 2 モジュールの `two-native` は `lift/out-avrt88/two` のものと 2,016 B 違う（原因は調べていない）。生成 C、分割 C、`common.c` は一致した。
- 領域の表の「スクラッチ」: `lift/runtime`、`lift/harness` にスクラッチ領域の定義は見つからなかった（`harness/oracle/run_diff2.py` の `SCRATCH` は別のオラクル用）。依頼の「スクラッチ」が指すものを別の場所で想定していたなら、この表には入っていない。
- 同じ構成での run は gdb なし 2 回、gdb の読み取り 2 回。どれも同じ結果。別のホストや別のカーネルでの再現は未確認。ASLR のため `calling entry … rsp=` と一部のポインタは run ごとに変わる（比較から除いた）。
- gdb の読み取りの run（`run/ro1-stdout.txt`、`ro2-stdout.txt`）でも、停止メッセージ、`call depth at stop: 17`、`init_count=150 of 942`、`functions_distinct=960`、`function_entries=26604` が gdb なしの run と同じだった（実測。`grep`）。
- 3 モジュールの OUT（`three/`）の FLEngine 側（`gen/flengine.c`、`src/fle-*.c`、`src/fle-tables.txt`）は、`two2/` からコピーした（SHA-256 `43985d62…37137a1` の C。`gen_two.sh` が「flengine.c exists」の後に SHA-256 を検査して一致を確認した）。

### 8.1 作ったファイル（`lift/out-three/`。gitignore 対象の `lift/out*/`）

- `src/`（`git archive HEAD`）、`target/`（fe-pe、fe-lift の release。28 MB）、`harness-two/`（一般化したスクリプト）、`harness-two.diff`、`harness-two-p1.diff`。
- `two2/`（2 モジュールの再現。`dll/`、`gen/`、`src/`、`v-tbl2/`（`native/two-native` を含む））、`out-two2/a4/`（a4-survey のログ 2 回）。
- `three/`（3 モジュール。`dll/Blend2D.4700000.dll`、`dll/dsp_ippv2_x64.2000000.dll`、`v-b2d/`（`gen/real.c`、`gen/blend2d.c`、`src/`、`modules.txt`、`native/two-native`、`native/realmod_table_gen.c`、`log/`）、`native/`（共有オブジェクト））、`out-three/a4/`（a4-survey のログ 2 回）。
- `run/ro1-*`、`ro2-*`（読み取りの gdb のログ）、`gdb/ro_imports3.py`、`analysis/`（`initializer_table.py`、`regions.py`、`dis_b2d.py`。読み取り専用）、`log/`（ビルド、生成、a4-survey の標準出力、`initializer-table.txt`、`regions.txt`）、`tmp/`（`TMPDIR`。`dis_b2d.py`、`tbl_b2d.py`、`b2d-pdata-seeds.txt`、`Blend2D.roundtrip.dll`、`applytest/` ほか）、`cmp/`（差分を作るためのシンボリックリンク 2 本）。
- **削除はしていない**（`rm` なし。`__pycache__` なし）。2 GB 級のファイルなし（最大は `two-native` の 612,184,672 B）。
