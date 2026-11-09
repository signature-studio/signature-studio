# A3 の再確認: FLEngine 単独構成で HEAD の wasm（node）と native の a4 が一致するか

測定のみ。修正はしていない。fe-lift、fe-decode、fe-pe、lift/runtime、lift/harness、lift/crates のテスト、docs/decisions、docs/README.md、docs/progress.md、既存の docs は編集していない。新規ファイルは本書と `lift/out-a3-m86/` の下だけ。

凡例: **実測** = このセッションで実行したコマンドの出力。**読み** = ソースや出力を読んでの解釈（確かめていない）。**未確認** = 測っていない、または確かめられない。

## 0. 結論

- **実測**: HEAD b515966 の runtime と生成 C（SHA-256 が基準の 43985d62… と一致）で、FLEngine 単独の native と wasm（node）の a4 の行 6 本は一致した。基準（init_count 145/942、last unit index 144 run_rank 132/895 proc 0x65c540 in_progress、started 132 completed 131、functions_distinct 821、function_entries 15,685、停止 `LoadLibraryW "dsp_ippv2_x64.dll"`）とも一致する。
- **実測**: native と wasm の標準出力（281 行）と標準エラー（1 行、194 B）を全文 diff した結果、**違いは `calling entry … rsp=` の 1 行だけ**。決定 0045、0049 の頃の既知の差と同じで、新しい出力の差はない。
- **実測**: native は 4 回、wasm は 3 回実行した。`rsp=` の行を除いた出力は 7 回とも同一（SHA-256 が同じ）。標準エラーも 7 回とも同一。
- **実測（出力の差ではないが目立つ変化）**: wasm の実行時間は **約 105 秒**（native は 0.04〜0.09 秒）。決定 0045 の実行記録は停止まで 1.7 秒（199 関数 / 1,386 入口）だった。原因の読みは §6。
- progress.md の「A3 は定義の範囲で達成だが現 HEAD では wasm の a4 一致を未再確認」に対して、**FLEngine 単独の構成は再確認できた**（本書の実測）。2 モジュール構成、ブラウザ、停止点より先は対象外（§8）。

## 1. 対象

| 項目 | 値 |
|---|---|
| HEAD | b5159667307636c0e1fffc218e167ba7abfba143（b515966）。`git archive HEAD` を `lift/out-a3-m86/src/` に展開し、そこからだけビルド。作業ツリーの lift/runtime と lift/crates は読んでいない |
| DLL | `/home/ubuntu/work/signature-studio-develop/binaries/win64/FLEngine_x64.dll`、22,227,016 B、SHA-256 b2c64da4…9496d |
| 生成 C の SHA-256（**最初に確認**） | **43985d62321a90f562ce1b8144253e00af13df795e4190d26b6b8ce2537137a1**（基準と一致）。215,707,512 B。`functions 59297`、`unsupported_total 525`（内訳: `Jmp: indirect through IAT slot (not lifted)` 524、`Jmp: indirect target unresolved` 1）、`bad_nodes 0` |
| ツール | emcc 6.0.12、clang 21.1.8（Ubuntu）、rustc 1.99.0、node v24.19.0。マシン: 16 コア、RAM 14 GB |
| 同時の emcc | 測定中 0 本（`ps` で各ステージの前に確認）。ビルドは JOBS=1（emcc は同時に 1 本） |

## 2. 実行したコマンド

```
# 1. HEAD の展開と fe-lift のビルド
mkdir -p lift/out-a3-m86/{src,tmp,em-cache}
git archive HEAD | tar -x -C lift/out-a3-m86/src/
cd lift/out-a3-m86/src/lift
CARGO_TARGET_DIR=<abs>/lift/out-a3-m86/target TMPDIR=<abs>/lift/out-a3-m86/tmp cargo build --release --offline -p fe-lift -p fe-pe

# 2. 生成 C と分割（SHA-256 を確認）
export OUT=<abs>/lift/out-a3-m86 DLL=<abs>/binaries/win64/FLEngine_x64.dll FLENGINE_PE=$DLL TMPDIR=$OUT/tmp EM_CACHE=$OUT/em-cache
cd $OUT
bash src/lift/harness/wasm/build.sh gen            # fe-lift lift $DLL out/wasm-pdata.c --pdata
sha256sum out/wasm-pdata.c                         # 43985d62… 一致
bash src/lift/harness/wasm/build.sh split          # split_c.py ... 3700 -> 17 ファイル

# 3. native
JOBS=2 bash src/lift/harness/wasm/build_native.sh
bash src/lift/harness/a4-survey.sh                 # native のみ（1 回目）

# 4. wasm（JOBS=1）
JOBS=1 bash src/lift/harness/wasm/build.sh rt
JOBS=1 bash src/lift/harness/wasm/build.sh gen-c
JOBS=1 bash src/lift/harness/wasm/build.sh harness link
WASM=1 bash src/lift/harness/a4-survey.sh          # native 1 回 + node 1 回 + a4 行の比較

# 5. 追加の実行（各回 /usr/bin/time -v）
./native/flengine-native $DLL                      # 2 回（runs/native-2, runs/native-3）
FLE_WASM_DIR=$OUT/bin-base node src/lift/harness/wasm/run_node.cjs $DLL   # 1 回（runs/node-2）
node --cpu-prof --cpu-prof-dir=runs/prof ... run_node.cjs $DLL            # 1 回（runs/node-prof、§6）
同じ実行で flengine.js の `_emscripten_return_address` に呼び出し回数のカウンタを足したコピー（runs/bin-count/）   # 1 回（§6）
```

- `FLENGINE_PE` は export したが、`build.sh` と `a4-survey.sh` は読まない（`grep` では tests の load.rs、res.rs と harness/oracle/run*.sh にあった）。生成 C の入力は `DLL` で渡した。`git archive` には `binaries/`（gitignore）が無いので、`build.sh` の既定の `DLL`（`$LIFT/../binaries/…`）は展開先では存在しない。**`DLL` の絶対パス指定が必須**。
- `build.sh` は `TMPDIR=$OUT/tmp`、`EM_CACHE=$OUT/emcache` を自分で上書きする（`build.sh` の 39 行目）。キャッシュは `lift/out-a3-m86/emcache/` に作られ、先に作った `lift/out-a3-m86/em-cache/` は空のまま残った（rm 禁止のため消していない）。どちらも `lift/out-a3-m86/` の中。
- 手順は `a4-survey-tool.md`、`build.sh`、`build_native.sh`、`a4-survey.sh`、`run_node.cjs` に書かれたとおりで、書かれていない手順は要らなかった。

## 3. ビルド

| 段 | 所要（wall） | ピーク RSS | 結果 |
|---|---|---|---|
| cargo（fe-lift、fe-pe。新しい target） | 3.06 s | 400 MB | 成功 |
| `gen`（fe-lift lift --pdata） | 2.44 s | 321 MB | 成功。`gen-pdata.err` 0 B |
| `split`（split_c.py） | 1.39 s | 585 MB | `functions 59297 parts 17`（part-00〜16。最後の 1 本は小さい） |
| native（clang -O0 -g。生成 C 17 + runtime 46 + ハーネス 4） | 59.49 s（JOBS=2） | 1.16 GB | 成功。`flengine-native` 258,194,992 B |
| wasm `rt`（runtime の `fle_*.c` 46 本） | 4.68 s | — | 全部 exit 0、errors 0、**warnings 1** |
| wasm `gen-c`（生成 C 17 本、`-O0 -finstrument-functions`） | 70.65 s（各 TU の合計 70.36 s） | 最大 1,755,896 KB（part-14） | 全部 exit 0、errors 0 |
| wasm `harness`（main、trace、image、a4） | 合計 0.3 s | — | exit 0、errors 0、warnings 0 |
| wasm `link` | 4.91 s | 608 MB | **成功**。警告なし（`link.log` に `warning` 0 件） |
| wasm の emcc 合計（rt + gen-c + harness + link） | **約 80.5 s**（4.68 + 70.65 + 0.30 + 4.91） | | |

- **wasm のサイズ**: `flengine.wasm` **115,917,209 B**（約 110.5 MiB）、`flengine.js` 174,922 B。参考: progress.md の「10-22 の値」は 115,894,302 B で +22,907 B、決定 0045 の実行記録は 115,721,618 B。
- **警告 1 件（実測）**: `lift/runtime/fle_proc.c:105:16: warning: variable 'fle_thread_prio' set but not used [-Wunused-but-set-global]`。
- **注意（実測、`build.sh` の読み）**: 生成 C のコンパイルは `-w` なので、`gen-c` の `warnings=0` は「警告が無い」ではなく「警告を出さない設定」。errors=0 と exit 0 は有効。
- 決定 0045 は「16 分割」と書く。今回は 17 ファイル（`split_c.py` に渡した 3700 関数 / ファイルで 59,297 / 3,700 = 16.03 のため、**読み**）。
- native 側の `-Wall -Wextra` の警告は記録していない（`build_native.sh` の出力は末尾の行だけ保存）。**未確認**。

## 4. a4 の行（native と wasm）

基準は依頼文のとおり。「native」は 4 回、「wasm」は 3 回とも同じ値（**実測**）。

| 項目 | 基準 | native（4 回） | wasm（node、3 回） |
|---|---|---|---|
| `a4: init_count` | 145 of 942、last_unit_proc 0x65c540 | 145 of 942、0x65c540 | 同じ |
| `a4: last_unit` | index 144、run_rank 132 of 895、proc 0x65c540、in_progress | 同じ | 同じ |
| `a4: runnable_started … completed … in_progress` | 132 / 131 / 1 of 895 | 132 / 131 / 1 of 895 | 同じ |
| `a4: unit[init_count-1]` | — | proc 0x65c540、units_outside_image 0 | 同じ |
| `a4: entry_check` | — | started_but_not_entered 0、runnable_entered_beyond_init_count 0 | 同じ |
| `a4: all_units_done … functions_distinct … function_entries` | functions_distinct 821、function_entries 15,685 | no / 821 / 15,685 | 同じ |
| 停止 | `LoadLibraryW "dsp_ippv2_x64.dll"` | メッセージ全文（stderr、194 B）が基準と同じ | 同じ |
| 呼び出し深さ（stop 時） | — | 9（#0 f_423990 … #8 f_1376820） | 同じ |
| 終了コード | — | 134（SIGABRT 相当、`trace.c` が `_exit(134)`） | 134（`run_node.cjs` の `quit`） |
| 静的な unit 表 | — | units 942、runnable 895、empty 46、noproc 1 | （同じ `fe-pe pe-init-table` の出力） |

`a4-survey.sh` の `WASM=1` は「same a4 lines as native (exit 134)」と判定し、終了コード 0 だった。

## 5. 全文 diff

対象: native と wasm の標準出力（各 281 行）と標準エラー。

```
diff a4/20261024T013315Z-native.txt a4/20261024T013315Z-node.txt
2c2
< calling entry (no export name) rcx=0x400000 rdx=1 r8=0 rsp=0x7bf530ffffc0
---
> calling entry (no export name) rcx=0x400000 rdx=1 r8=0 rsp=0x11c64fc0
```

- 標準エラー: native と node で差なし（`fle: LoadLibraryW: module "dsp_ippv2_x64.dll" is not a virtual module and not a name this environment is defined to lack (external modules are M2; no handle and no failure is invented for them)` の 1 行）。
- 停止メッセージ全文、`abort kind: (not a runtime message)`、関数実行数、入口数、深さ、スタック 9 段、entry log 256 件、a4 の行は全部一致（上の diff に出ていない）。

### 同じ実行の繰り返し（実測）

| 実行 | `rsp=` | `rsp=` 以外の stdout の SHA-256（先頭 16 桁） | stderr |
|---|---|---|---|
| native 1（survey 1 回目） | 0x76c96dffffc0 | 6d0b0aeccc3c00e4 | d93a986c06f7042c |
| native 2（survey の WASM=1 内） | 0x7bf530ffffc0 | 同じ | 同じ |
| native 3 | 0x708683ffffc0 | 同じ | 同じ |
| native 4 | 0x7abae4dfffc0 | 同じ | 同じ |
| wasm 1（survey） | 0x11c64fc0 | 同じ | 同じ |
| wasm 2 | 0x11c64fc0 | 同じ | 同じ |
| wasm 3（`--cpu-prof` 付き） | 0x11c64fc0 | 同じ | 同じ |

- **native 同士**: 全文 diff の差は `rsp=` の 1 行だけ（毎回違う値）。**wasm 同士**: 全文一致（`rsp=` も 0x11c64fc0 で固定）。
- 上の 3 回とは別に、カウンタ入りの `flengine.js` のコピー（§6）でも 1 回実行した（`runs/node-count.txt`）。標準出力は `rsp=` 以外が同一。標準エラーは、私がコピーに足した `RAC_CALLS=19444540` の 1 行を除いて同一（その行は本来の出力ではない）。この 1 回は上の「7 回」に数えていない。

### (a) 既知の差

- `calling entry … rsp=` の 1 行。決定 0045 の実行記録（`wasm-port-run.md` §1）と `wasm-portability-fixes-run.md` が、同じ形（この 1 行だけ）で記録している。
- **読み**: `main.c` は `aligned_alloc` で取った領域の上端を `R[4]` にして出力する。native はプロセスごとに違う実アドレス（4 回とも違う値。ASLR を切って確かめてはいない）、wasm は線形メモリ内の決定的な値（3 回とも同じ）。ゲストから見える意味は変わらない。

### (b) 新しい差

- **出力の新しい差はない**（実測）。
- 出力ではない変化（実測）: wasm の実行時間（§6）、ビルドの分割数 17（§3）、wasm サイズ（§3）。

## 6. 実行時間とピークメモリ（追加）

`/usr/bin/time -v` が使えた。

| 実行 | wall | user | ピーク RSS |
|---|---|---|---|
| native（4 回） | 0.04〜0.09 s | 0.02〜0.03 s | 53.7〜54.4 MB |
| node（survey 1 回目。native 1 回と fe-pe の表を含む） | 1:45.64 | — | 396,680 KB |
| node 2 | 1:45.07 | 104.88 s | 396,604 KB（約 387 MiB） |
| node（`--cpu-prof` 付き） | 1:46.94 | — | 401,920 KB |
| node（呼び出しカウンタ入りコピー） | 1:45.23 | — | — |

- **実測**: node の実行は約 105 秒。決定 0045 の実行記録は停止まで 1.56〜1.78 秒（199 関数 / 1,386 入口の時点）。入口数は 11.3 倍（15,685 / 1,386）だが、時間は約 60 倍。
- **実測（`--cpu-prof`、`runs/prof/*.cpuprofile`）**: 自己時間の合計 106.8 秒のうち、`jsStackTrace` 54.5 %、`defaultPrepareStackTrace` 35.6 %、`_emscripten_return_address` 5.5 %（計 95.7 %）。`convertFrameToPC` と正規表現を足すと 97.5 %。wasm の関数自体の自己時間は個々に 0.1 秒未満。
- **実測**: `_emscripten_return_address` の呼び出し回数は **19,444,540 回**（`flengine.js` のコピーにカウンタを足して数えた。出力は元と同一）。
- **読み（確かめていない）**: 生成 C は `-O0 -finstrument-functions` でコンパイルされる（`build.sh` の `INSTRUMENT=1`）。emscripten は `__builtin_return_address` を `_emscripten_return_address`（JS の `new Error().stack` の解析）で実装するので、フックを呼ぶたびに 1 回の JS スタックトレースが要る。`fle_runtime.h` の `fle_ld`、`fle_st`、`fle_addr` などの `static inline` には `no_instrument_function` が付いておらず（`lift/runtime/*.h` に 0 件）、`-O0` ではインライン化されないので、生成 C のメモリアクセスごとに enter と exit のフックが呼ばれ、`trace.c` のフック内で `!is_lifted(fn)` として捨てられるまでに呼び出し側アドレスの計算が済んでいる。19.4 M という回数は関数入口 15,685 回では説明できず、この読みと整合する。ただし、フックを外した版や `no_instrument_function` を付けた版で再測していないので、**原因は未確認**。
- 影響: a4 の測定は入口フック（`-finstrument-functions`）が前提（`a4-survey-tool.md` §4）なので、フックを外すと `entry_check` が意味を失う。時間を縮める方法は本書の対象外。

## 7. 変わっていないこと / 前回との比較

- 決定 0076 の「a4 survey（native と WASM=1）」の記録（init_count 145 / 942、関数 821、a4 の行は同一）と、今回の値は同じ。決定 0078 の応答表変更（生成 C の SHA が 43985d62… に）、0083〜0085 の runtime の変更の後でも、a4 の行は変わっていない。
- 決定 0045 の時点の停止（GetLocaleInfoW、199 関数 / 1,386 入口）から、今の停止（LoadLibraryW、821 関数 / 15,685 入口）まで進んだ。この経路で native と wasm の出力が一致することを確かめた。

## 8. 限界（決定 0045、0049 を読んで書く）

- **ブラウザは未検証**。ビルドは node 限定（`-sENVIRONMENT=node`、`-sNODERAWFS=1`）。ブラウザ用の構成は無い（0045 e）。
- **停止点より先は未測定**。停止（`LoadLibraryW "dsp_ippv2_x64.dll"`）は外部モジュール（M2）の扱いで、両者がここで止まることは確かめたが、その先の実行の一致は測っていない。
- **SSE/x87 の等価性**: 0045 の時点では DllMain の経路で `fle_mx_*` と x87 の演算が 0 回で、wasm 上の MXCSR 純 C 版と x87 の等価性の証拠にならなかった。今回、現在の停止点までの経路でこれらが何回実行されたかは**測っていない**ので、停止点まで一致したことは「その経路で SSE/x87 が一致した」ことを意味しない。個別の突合（`fscale`、`fle_sqrtss/sd` など）は 0049 の実行記録の範囲（60,002 行で不一致 0）。
- **ページ保護は wasm で fail-open**（0045 c）: ホストが保護を強制しないので、予約のみ、デコミット後、読み取り専用、PAGE_NOACCESS のページへのゲストのアクセスは、native では SIGSEGV で止まるが、wasm では成功する。仮想ページ保護の状態（`VirtualProtect`、`VirtualQuery`）は runtime の表が持つので、API の結果は変わらないが、違反アクセスは検出されない。今回の停止点までの経路でこの差が現れたかは測っていない（出力は一致）。
- **アリーナの制限**（0045 c、b）: 合計 512 MiB（ファイルマッピングを含む）が上限、固定アドレス（アリーナの外）の予約は wasm で失敗する（native は成功）、同じオブジェクトの同じオフセットの 2 つ目の共有ビューは wasm で名前付き停止。
- **ゲストアドレスの検査**（0045 d、0049 a）: wasm では上位 32 ビットの検査（`FLE_CHECK_HI32`）が有効。領域チェック（`FLE_CHECKED_MEM`）は既定でオフ。`fle_ldx` 系は 0049 で検査に通した。runtime 内の直接変換のうち 71 か所は対象外（理由は `hi32-audit.md`）。ヒープ内のワイルドポインタとページ保護は検出しない。
- **スレッド ID**: wasm の `GetCurrentThreadId` は 1 固定（0045 a）。native は実際の tid。
- **最適化レベル**: wasm は `-O0` のみ。`-O1` 以上は未確認（0049 の限界）。
- **比較の性質**: native（clang 21.1.8、`-O0 -g -fPIE`）と wasm（emcc 6.0.12、`-O0`）は、同じ生成 C と同じ runtime ソースの 2 つのコンパイル。2 モジュール構成は本書の対象外（progress.md は、実 DLL 全体では node が `local count too large` で拒否すると記録している。本書では再測していない）。
- **繰り返しの回数**: native 4 回、wasm 3 回。非決定的な要因（ASLR）があるのは `rsp=` の値だけだった。回数が少ないので、稀な非決定性は排除できない。
- 本書の実測は 1 台のマシン（16 コア、RAM 14 GB）。実行時間は他の負荷の影響を受けうる（開始時の load average 0.9 程度）。

## 9. 成果物

- 本書: `docs/investigations/a3-wasm-recheck.md`（新規）。
- `lift/out-a3-m86/`: `src/`（HEAD の git archive）、`target/`、`out/`（生成 C、17 分割）、`gen/`、`obj-base/`、`bin-base/`（`flengine.wasm`、`flengine.js`、`run_node.cjs`）、`native/`、`log-base/`（ビルドログと時間）、`a4/`（survey の出力）、`runs/`（追加の実行、`prof/` のプロファイル、`bin-count/` のカウンタ入りコピー）、`tmp/`、`emcache/`、空の `em-cache/`。合計約 1.6 GB。1 GB 超の単一ファイルは無い（最大は native バイナリ 258 MB）。
- `runs/bin-count/flengine.js` は測定用に 1 行を変えたコピー（`flengine.wasm` は元ファイルへのシンボリックリンク）。元の `bin-base/flengine.js` は変えていない。
