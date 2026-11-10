# A3 の再確認（2 回目）: HEAD の木で FLEngine 単独の wasm（node）と native の a4 を比べ直す

測定のみ。決めない。修正はしていない。fe-lift、fe-decode、fe-pe、lift/runtime、lift/harness、lift/crates のテスト、docs/decisions、docs/README.md、docs/progress.md、既存の docs は編集していない。新規ファイルは本書と `lift/out-a3-head/` の下だけ。

凡例: **実測** = このセッションで実行したコマンドの出力。**読み** = ソースや出力を読んでの解釈（確かめていない）。**未確認** = 測っていない、または確かめられない。

前回: `docs/investigations/a3-wasm-recheck.md`（コミット 3d359a0、HEAD b515966 の木）。手順、フラグ、スクリプトは前回と同じで、変えたのは木（HEAD 0d58b0f）、作業ディレクトリ、emcc の並列度（前回 JOBS=1、今回 JOBS=2）だけ。

## 0. 結論

- **実測**: HEAD 0d58b0f の木から作った生成 C の SHA-256 は **feb5b9087f8d7a8a1282900212d7bd1f5b6071b0326e7149642f98c168e8080f**（依頼の基準と一致）。
- **実測**: FLEngine 単独の native と wasm（node）の a4 の行 6 本は、native 5 回と node 7 回の全部で同一。基準（init_count 145/942、last unit index 144 run_rank 132/895 proc 0x65c540 in_progress、started 132 / completed 131、functions_distinct 821、function_entries 15,685、停止 `LoadLibraryW "dsp_ippv2_x64.dll"`）とも一致する。3d359a0 の値とも同じ。
- **実測**: native と wasm の標準出力（281 行）と標準エラー（1 行、194 B）を全文 diff した結果、**違いは `calling entry … rsp=` の 1 行だけ**。それ以外の差はない（§5）。
- **実測**: wasm（node）の実行時間は 7 回で **1:48〜2:17（108〜137 秒）**。native は 0.05〜0.06 秒。3d359a0 の約 105 秒より長い側に寄っているが、測定中に同じマシンで他のプロセスが動いていて負荷が一定でなく（§6）、差が HEAD の変更によるものかは分けられない。
- 3d359a0 との差で、出力に関するものはない。変わったのは生成 C の SHA（43985d62… → feb5b908…）とサイズ、wasm の `rsp=` の値、実行時間のばらつき（§7）。

## 1. 対象

| 項目 | 値 |
|---|---|
| HEAD | `0d58b0f019b232c7fba56b346287a8b658075e24`（`git rev-parse HEAD`）。`git archive HEAD` を `lift/out-a3-head/src/` に展開し、そこからだけビルド。ワークツリーの lift/ は読んでいない |
| lift/ の不変 | `git diff --stat d618380 HEAD -- lift` の出力が空（依頼の「lift/ 配下は d618380 から不変」と一致） |
| DLL | `/home/ubuntu/work/signature-studio-develop/binaries/win64/FLEngine_x64.dll`、22,227,016 B、SHA-256 b2c64da40921cf262c15c5b25c2b0422e4d1851e953c87e0bc38ed2c19f9496d（依頼の値と一致） |
| 生成 C（**最初に確認**） | `out/wasm-pdata.c`、215,709,232 B、SHA-256 **feb5b9087f8d7a8a1282900212d7bd1f5b6071b0326e7149642f98c168e8080f**。`functions 59297`、`bad_nodes 0`、`unsupported_total 525`（`Jmp: indirect through IAT slot (not lifted)` 524、`Jmp: indirect target unresolved` 1）、`register_jmp_sites 20`、`msvc_tables 0`、`msvc_rejected 0`。`gen-pdata.err` 0 B |
| ツール | emcc 6.0.12、clang 21.1.8（Ubuntu）、rustc 1.99.0、node v24.19.0 |
| マシン | AMD Ryzen 9 9950X（`lscpu`: 8 コア × 2 スレッド、論理 16 CPU）、RAM 約 13 GiB（`free -g` の total） |
| 同時の emcc | 各ステージの前に `ps` で確認して 0 本。ビルドは JOBS=2（`xargs -P 2`）なので、自分の emcc は最大 2 本 |
| 他の作業ディレクトリ | `CARGO_TARGET_DIR`、`TMPDIR`、`EM_CACHE`（`build.sh` が `$OUT/emcache` に設定）は全部 `lift/out-a3-head/` の中。emsdk の既定キャッシュ（`/opt/emsdk/upstream/emscripten/cache`）の `sanity.txt`、`js_output` の更新時刻は 10-09、10-19 のままで、今回の作業では更新されていない |

## 2. 実行したコマンド

```
mkdir -p lift/out-a3-head/{src,tmp,em-cache}
git archive HEAD | tar -x -C lift/out-a3-head/src/

# 1. fe-lift、fe-pe（新しい target）
export OUT=<abs>/lift/out-a3-head TMPDIR=$OUT/tmp PATH=/opt/emsdk:/opt/emsdk/upstream/emscripten:$PATH
cd $OUT/src/lift && CARGO_TARGET_DIR=$OUT/target cargo build --release --offline -p fe-lift -p fe-pe

# 2. 生成 C（SHA-256 を確認）と分割
export DLL=<abs>/binaries/win64/FLEngine_x64.dll
cd $OUT
bash src/lift/harness/wasm/build.sh gen            # fe-lift lift $DLL out/wasm-pdata.c --pdata
sha256sum out/wasm-pdata.c                         # feb5b908… 一致
PYTHONDONTWRITEBYTECODE=1 bash src/lift/harness/wasm/build.sh split   # split_c.py ... 3700 -> 17 ファイル

# 3. native（JOBS=2）
JOBS=2 bash src/lift/harness/wasm/build_native.sh

# 4. wasm（JOBS=2）。各段を /usr/bin/time -v で包んだ
JOBS=2 bash src/lift/harness/wasm/build.sh rt
JOBS=2 bash src/lift/harness/wasm/build.sh gen-c
JOBS=2 bash src/lift/harness/wasm/build.sh harness
JOBS=2 bash src/lift/harness/wasm/build.sh link

# 5. a4-survey（native 1 回 + node 1 回を含む。2 回）
WASM=1 bash src/lift/harness/a4-survey.sh          # runs/survey-{1,2}.out、ログは a4/

# 6. 追加の実行（各回 /usr/bin/time -v）
./native/flengine-native $DLL                      # 3 回（runs/native-{1,2,3}）
FLE_WASM_DIR=$OUT/bin-base node src/lift/harness/wasm/run_node.cjs $DLL   # 5 回（runs/node-{3,4,5,6,7}）
```

- `DLL` は絶対パスで渡した。`git archive` には `binaries/`（gitignore）が無いので、`build.sh` と `a4-survey.sh` の既定の `DLL`（`$LIFT/../binaries/…`）は展開先では存在しない。
- `build.sh` は `TMPDIR=$OUT/tmp`、`EM_CACHE=$OUT/emcache` を自分で上書きする（`build.sh` の 39 行目）。キャッシュは `lift/out-a3-head/emcache/` に作られ、先に作った `lift/out-a3-head/em-cache/` は空のまま残った（rm 禁止のため消していない）。
- 前回の手順に書かれていない手順は要らなかった。

## 3. ビルド

| 段 | 所要（wall） | ピーク RSS | 結果 |
|---|---|---|---|
| cargo（fe-lift、fe-pe。新しい target） | 3.06 s | 399 MB | 成功 |
| `gen`（fe-lift lift --pdata） | 2.55 s | 321 MB | 成功 |
| `split`（split_c.py） | 1.60 s | 585 MB | `functions 59297 parts 17`（part-00〜16） |
| native（clang -O0 -g。生成 C 17 + runtime 46 + ハーネス 4。JOBS=2） | 1:35.89 | 1.17 GB | 成功。`flengine-native` 258,209,848 B |
| wasm `rt`（runtime の `fle_*.c` 46 本） | 3.42 s | — | 全部 exit 0、errors 0、**warnings 1** |
| wasm `gen-c`（生成 C 17 本、`-O0 -finstrument-functions`、JOBS=2） | 45.63 s（各 TU の合計 87.17 s） | 最大 1,754,596 KB（part-14） | 全部 exit 0、errors 0 |
| wasm `harness`（main、trace、image、a4） | 0.38 s | — | 4 本とも exit 0、errors 0、warnings 0 |
| wasm `link` | 5.44 s | 608 MB | 成功。`link.log` に `warning` 0 件 |
| wasm の emcc 合計（rt + gen-c + harness + link） | **約 55 s**（3.42 + 45.63 + 0.38 + 5.44 = 54.87） | | |

- **wasm のサイズ**: `flengine.wasm` **115,926,771 B**（約 110.6 MiB）、`flengine.js` 174,922 B。
- **警告 1 件（実測）**: `lift/runtime/fle_proc.c:105:16: warning: variable 'fle_thread_prio' set but not used [-Wunused-but-set-global]`（emcc の `rt` 段）。3d359a0 と同じ 1 件。
- **実測**: native の clang ビルド（`build_native.sh`。標準出力と標準エラーを `log-base/native-build.out` に保存）の出力は `built …` の 1 行だけ。`warning:` と `error:` は 0 件。runtime とハーネスは `-Wall -Wextra` 付きで、emcc の `rt` 段に出た `fle_thread_prio` の警告は native の出力には出なかった。**実測**: 同じ `fle_proc.c` を `-std=gnu11 -O0 -g -Wall -Wextra -fPIE -fsyntax-only` で Ubuntu clang 21.1.8 に通しても警告は出ず（出力なし）、emcc 同梱の clang（`/opt/emsdk/upstream/bin/clang --version` は 24.0.0git）は同じ警告を出す。警告の有無の差は clang のバージョンの違いと整合するが、`-Wunused-but-set-global` がいつ入ったかは調べていない（**未確認**）。
- **注意（実測、`build.sh` の読み）**: 生成 C のコンパイルは `-w` なので、`gen-c` の `warnings=0` は「警告が無い」ではなく「警告を出さない設定」。errors=0 と exit 0 は有効。
- 3d359a0 の emcc 合計（約 80.5 s、JOBS=1）とは並列度が違う（本書は JOBS=2）ので、所要時間は比べられない。

## 4. a4 の行（native と wasm）

基準は依頼文のとおり。a4 の行は、native 5 回（a4-survey 2 回 + 単独 3 回）と node 7 回（a4-survey 2 回 + 単独 5 回）の**全部が同一**（`diff <(grep '^a4:' …)` と SHA-256 で確認、**実測**）。

```
a4: init_count=145 of 942, last_unit_proc=0x65c540
a4: last_unit index=144 run_rank=132 of 895 proc=0x65c540 state=in_progress
a4: runnable_started=132 completed=131 in_progress=1 of 895 runnable
a4: unit[init_count-1] proc=0x65c540; units_outside_image=0
a4: entry_check started_but_not_entered=0 runnable_entered_beyond_init_count=0
a4: all_units_done=no functions_distinct=821 function_entries=15685
```

| 項目 | 基準 | native（5 回） | wasm（node、7 回） |
|---|---|---|---|
| init_count | 145 / 942、last_unit_proc 0x65c540 | 145 of 942、0x65c540 | 同じ |
| last unit | index 144、run_rank 132 of 895、proc 0x65c540、in_progress | 同じ | 同じ |
| started / completed / in_progress | 132 / 131 / 1 of 895 | 132 / 131 / 1 of 895 | 同じ |
| functions_distinct / function_entries | 821 / 15,685 | 821 / 15,685 | 同じ |
| entry_check | — | started_but_not_entered 0、runnable_entered_beyond_init_count 0 | 同じ |
| 停止 | `LoadLibraryW "dsp_ippv2_x64.dll"` | 下の全文（stderr、194 B） | 同じ |
| 呼び出し深さ（stop 時） | — | 9（#0 f_423990 … #8 f_1376820。#8 の guest-return=0xdead0000） | 同じ |
| 終了コード | — | 134（`trace.c` が `_exit(134)`） | 134（`run_node.cjs` の `quit`） |
| 静的な unit 表（`fe-pe pe-init-table`） | — | units 942、runnable 895、empty 46、noproc 1（header 0x13768b0、units 0x13768e0） | （同じ出力） |

停止メッセージ全文（native と node の標準エラー。保存した 12 回の実行で同一）:

```
fle: LoadLibraryW: module "dsp_ippv2_x64.dll" is not a virtual module and not a name this environment is defined to lack (external modules are M2; no handle and no failure is invented for them)
```

標準出力側では `== stop: SIGABRT (runtime abort, see runtime message)`、`abort kind: (not a runtime message)`、`functions executed (distinct): 821`、`function entries (calls): 15685` も全回同一。

`a4-survey.sh` の `WASM=1` は 2 回とも「same a4 lines as native (exit 134)」と判定し、終了コード 0 だった。

## 5. 全文 diff

対象: native と wasm の標準出力（各 281 行）と標準エラー。

```
diff a4/20261025T120947Z-native.txt a4/20261025T120947Z-node.txt
2c2
< calling entry (no export name) rcx=0x400000 rdx=1 r8=0 rsp=0x78ae25dfffc0
---
> calling entry (no export name) rcx=0x400000 rdx=1 r8=0 rsp=0x11d65fc0

diff a4/20261025T121155Z-native.txt a4/20261025T121155Z-node.txt
2c2
< calling entry (no export name) rcx=0x400000 rdx=1 r8=0 rsp=0x79765fffffc0
---
> calling entry (no export name) rcx=0x400000 rdx=1 r8=0 rsp=0x11d65fc0
```

- 標準エラー（`.err`）: 保存した 12 回（native 5、node 7）の SHA-256 がすべて `d93a986c06f7042c…` で同一。native と node の diff は 0 行。
- **`rsp=` 以外の標準出力**: 12 回すべてで、`rsp=` の行を除いた stdout の SHA-256（先頭 16 桁）が `6d0b0aeccc3c00e4` で同じ（値の種類 1 つ）。`runs/native-1.txt` との `diff` でも、`rsp=` を除いた差は 0 行（残り 11 回）。3d359a0 の記録の値（`6d0b0aec…`）とも同じ。
- **新しい差はない**。列挙できる差は `rsp=` の 1 行のみ。

### `rsp=` の値

| 実行 | `rsp=` |
|---|---|
| native（5 回） | 0x78ae25dfffc0、0x79765fffffc0、0x7a3f321fffc0、0x7254a9bfffc0、0x7f81a89fffc0（毎回違う） |
| wasm（node、7 回） | 0x11d65fc0（7 回とも同じ） |

- **実測**: 3d359a0 の wasm の値は 0x11c64fc0、今回は 0x11d65fc0。差は 0x101000（1,052,672 B）。
- **読み**: `harness/main.c`（native）はスタックを `mmap` で取り、`harness/wasm/main.c`（wasm）は `aligned_alloc(4096, …)` で取って、どちらも上端付近を `R[4]` にして出力する。native はプロセスごとに違う実アドレス、wasm は線形メモリ内の決定的な値。決定 0045、0049 の頃からの既知の差と同じ形。
- **未確認**: wasm の値が 0x101000 動いた理由（リンクした静的データやヒープの配置が変わったためと思われるが、確かめていない）。native の値が毎回違う理由が ASLR かも、ASLR を切って確かめていない。

## 6. 実行時間とピークメモリ

`/usr/bin/time -v` が使えた。

| 実行 | wall | user | ピーク RSS | 実行前の load average（1 分） |
|---|---|---|---|---|
| native 1 | 0:00.05 | 0.03 s | 51,400 KB | 13.55 |
| native 2 | 0:00.06 | 0.04 s | 51,996 KB | 13.55 |
| native 3 | 0:00.05 | 0.03 s | 51,812 KB | 13.55 |
| survey 1（`a4-survey.sh`: native + `fe-pe` + node の合計） | 2:01.27 | — | 396,536 KB | 11.54 |
| survey 2（同上） | 2:16.02 | — | 396,068 KB | 6.78 |
| node 3 | 1:59.39 | 118.28 s | 396,564 KB | 13.27 |
| node 4 | 2:17.37 | 135.11 s | 396,440 KB | 7.59 |
| node 5 | 2:16.83 | 135.72 s | 396,512 KB | 12.54 |
| node 6 | 1:48.42 | 108.29 s | 396,432 KB | 6.96 |
| node 7 | 1:50.86 | 110.73 s | 396,684 KB | 1.45 |

（native 1〜3 の load average は 3 回を 1 つのコマンドで続けて取ったため、値が 1 つ共通）

- **実測**: native は 0.05〜0.06 秒、ピーク RSS 約 51〜52 MB。node は単独 5 回で 1:48.42〜2:17.37（user 108.29〜135.72 s、system 0.52〜0.56 s）、ピーク RSS は survey を含む 7 回で 396,068〜396,684 KB（約 387 MiB）。3d359a0 は node 396,604 KB、native 53.7〜54.4 MB で、node はほぼ同じ。
- **実測（時間のばらつき）**: node の wall の最短と最長の比は約 1.27。実行前の load average は 1.45〜13.55 と一定でなく、`ps` で見えた CPU の多いプロセスは本書の測定に由来しないもの（名前: `native_drv2`、`adjustor-*`、`call_iat_dll-*`、`jt_cpu-*`、`lift_drv2`、`apport`）だった。誰のプロセスかは確認していない（他の作業のものと思われるが**未確認**）。load が高い時（13 前後）に長く（node 3: 1:59、node 5: 2:16）、低い時（1.45、6.96）に短い（node 7: 1:50、node 6: 1:48）傾向があるが、node 4（load 7.59、2:17）は傾向に合わない。**読み**: 同時の負荷が時間に効いている可能性があるが、因果は確かめていない。
- 3d359a0 の node は約 105 秒（1:45.07〜1:46.94）だった。本書の最短（1:48.42）はこれより約 3 秒長く、最長（2:17.37）は約 32 秒長い。**未確認**: この差が HEAD の runtime や生成 C の変更によるのか、マシンの負荷の違いによるのかは、本書の実測では分けられない（3d359a0 の木を同じ条件で動かしていない）。
- **範囲外（未確認）**: wasm が遅い原因。3d359a0 は `-finstrument-functions` のフックが呼ぶ `_emscripten_return_address` が原因という読みを書いている。本書では**再検証していない**（依頼で範囲外）。フックを外した版は作っていない。プロファイルも呼び出し回数の計測も取っていない。したがって、今回の実行時間について原因を述べることはできない。

## 7. 3d359a0 との比較

| 項目 | 3d359a0（HEAD b515966） | 本書（HEAD 0d58b0f） | 差 |
|---|---|---|---|
| 生成 C の SHA-256 | 43985d62321a90f5… | feb5b9087f8d7a82… | 変わった（下の読み） |
| 生成 C のサイズ | 215,707,512 B | 215,709,232 B | +1,720 B |
| `functions` / `unsupported_total` / `bad_nodes` | 59297 / 525 / 0 | 59297 / 525 / 0 | 同じ |
| 分割数 | 17 | 17 | 同じ |
| wasm サイズ | 115,917,209 B | 115,926,771 B | +9,562 B |
| native サイズ | 258,194,992 B | 258,209,848 B | +14,856 B |
| emcc の警告 | 1（`fle_thread_prio`） | 1（同じ） | 同じ |
| a4 の行 6 本 | init_count 145/942、last unit 144（0x65c540、in_progress）、132/131、functions 821、entries 15,685 | 同じ | 同じ |
| 停止 | `LoadLibraryW "dsp_ippv2_x64.dll"` | 同じ | 同じ |
| stdout 281 行、stderr 194 B | `rsp=` の 1 行だけが差 | 同じ | 同じ |
| `rsp=`（wasm） | 0x11c64fc0 | 0x11d65fc0 | +0x101000 |
| node の wall | 約 105 秒（1:45.07〜1:46.94） | 1:48.42〜2:17.37 | 最短で約 +3 秒、最長で約 +32 秒。負荷が違う（§6） |
| node のピーク RSS | 396,604 KB | 396,068〜396,684 KB | ほぼ同じ |
| native の wall | 0.04〜0.09 s | 0.05〜0.06 s | 同じ範囲 |
| native のピーク RSS | 53.7〜54.4 MB | 51.4〜52.0 MB | 約 2 MB 小さい |
| 繰り返し | native 4 回、wasm 3 回 | native 5 回、node 7 回 | — |
| emcc の並列度 | JOBS=1 | JOBS=2 | — |

- **読み**: 生成 C の SHA が変わったのは、b515966 以降の runtime と応答表の変更による。`git log b515966..HEAD -- lift/runtime` に 0087（2441cde）、0088（2e94df2）、0091（a64a41c）、0092（38df911）のコミットがあり、0091 と 0092 のコミットメッセージは「response table only」（0091: 5 places, 7 lines。0092: 5 places, 32 lines。どちらも 0 functions changed）と書く。`functions 59297` が変わらないこととは整合する。ただし、3d359a0 の生成 C と今回の生成 C を直接 diff していない（他の作業ディレクトリは読まない指示のため）ので、差が応答表だけかは**未確認**。
- **実測**: `git diff --stat b515966 HEAD -- lift/runtime` で変わっているのは fle_load.c、fle_locale.c、fle_locale.h、fle_modules.c、fle_modules.h、fle_sys.c、fle_sys.h と runtime/test/ の下のファイル（依頼の「runtime が変わった」と整合）。
- **読み**: これらの変更（GetModuleHandleA、SRW ロック、avrt/powrprof、ctype/LCMap/cp1252 など）は、単独構成の停止点までの経路の a4 の行を変えなかった。経路上でこれらの API が呼ばれたかは個別には測っていない。

## 8. 限界

- **ブラウザは未検証**。ビルドは node 限定（`-sENVIRONMENT=node`、`-sNODERAWFS=1`）。ブラウザ用の構成は無い。
- **wasm のページ保護は fail-open**（決定 0045）: ホストが保護を強制しないので、予約のみ、デコミット後、読み取り専用、PAGE_NOACCESS のページへのゲストのアクセスは、native では SIGSEGV で止まるが、wasm では成功する。今回の停止点までの経路でこの差が現れたかは測っていない（出力は一致）。
- **停止点より先は未測定**。停止（`LoadLibraryW "dsp_ippv2_x64.dll"`）は外部モジュール（M2）の扱いで、両者がここで止まることは確かめたが、その先の実行の一致は測っていない。**SSE/x87 の等価性も、停止点より先は未測定**。停止点までの経路で SSE/x87 の演算が何回実行されたかも測っていないので、a4 の行が一致したことは「その経路で SSE/x87 が一致した」ことを意味しない。
- **2 モジュール構成は対象外**。FLEngine 単独のみ。Blend2D を含む構成は測っていない。
- **最適化レベル**: native（clang 21.1.8、`-O0 -g -fPIE`）と wasm（emcc 6.0.12、`-O0`）のみ。`-O1` 以上は未確認。
- **スレッド ID**: wasm の `GetCurrentThreadId` は 1 固定（決定 0045）。native は実際の tid。
- **繰り返しの回数**: native 5 回、node 7 回。非決定的な要因が現れたのは native の `rsp=` の値だけだった。回数が少ないので、稀な非決定性は排除できない。
- **実行時間は共有マシンでの測定**: 同じマシンで他のプロセスが動いていた（§6）。時間の絶対値と 3d359a0 との差は、負荷が統制されていない測定としてしか扱えない。
- **`-finstrument-functions` が前提**: a4 の測定は入口フックが前提（`a4-survey-tool.md`）。フックを外した版は本書の対象外で、作っていない。
- 3d359a0 の生成 C との直接比較はしていない（§7）。

## 9. 成果物

- 本書: `docs/investigations/a3-wasm-recheck-2.md`（新規）。
- `lift/out-a3-head/`（`.gitignore` の `lift/out*/` に該当し、`git status` には出ない）: `src/`（HEAD の git archive、10 MB）、`target/`、`out/`（生成 C。17 分割 + 全体、431 MB）、`gen/`、`obj-base/`、`bin-base/`（`flengine.wasm`、`flengine.js`、`run_node.cjs`）、`native/`（756 MB）、`log-base/`（ビルドログと時間）、`a4/`（survey の出力）、`runs/`（survey と追加の実行の出力と時間）、`tmp/`、`emcache/`、空の `em-cache/`。合計約 1.5 GB。500 MB 超の単一ファイルは無い（最大は native バイナリ 258 MB）。
