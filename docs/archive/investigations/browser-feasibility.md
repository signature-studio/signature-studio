# 6 モジュール構成の wasm を実際のブラウザ（headless Chrome 156）で動かす: 実現性の測定

作業日: 2026-10-28。基準: ブランチ main、作業開始時の HEAD **e071fc0**（`git archive HEAD lift docs/investigations/a4-six-module-wasm/tools` を `/tmp/opencode/web/src/` に展開。ワークツリーは読み取りだけ）。作業の途中で HEAD は 228c78a まで進んだ（他の作業者の docs／tools の commit。`git diff --stat e071fc0 HEAD -- lift` は空なので、wasm の入力は変わっていない）。**測定のみ・commit なし**。新規ファイルは本書と `lift/harness/web/`（小さな JS／HTML／MD／TSV のみ、バイナリなし）。

凡例: **実測**＝コマンド・スクリプトの出力を自分で見た事実／**読み**＝そこからの解釈・外挿（確かめていない）／**未確認**。

## 0. 結論（先に）

**ブラウザ（Chrome 156.0.8078.47 headless、V8 15.6.75.19）での 6 モジュール wasm: 取得 成功、コンパイル・インスタンス化 成功、a4 は node と一致、V8／Chrome の上限には当たらなかった。** 実測。ただし headless Chrome 1 種（Linux x86_64、localhost）での 1 つの作業（a4 の初期化経路）の結果で、Firefox／Safari、実ネットワーク、FL Studio の実際の使用（UI・音声）は測っていない。

| 項目 | 結果（実測） |
|---|---|
| 実行した wasm | **`flengine6.wasm` 389,439,333 B、SHA-256 `6e52a849b2aca06c3f78d43454ce32006105e2b12a2dbb10e8aa9372a590dcc7`（独立再現と一致）。ブラウザ用のリンク（`-sENVIRONMENT=web,worker`、MEMFS）で作った wasm も node 用と SHA-256 がバイト同一**（変わるのは JS の接着コードだけ）。§2.2 |
| a4（Web Worker 内、高速版相当） | **`init_count=942 of 942`、`all_units_done=yes`、`runnable_started=895 completed=895 in_progress=0`、`== stop: returned from entry`、終了コード 0、functions 5,306、entries 1,007,942、深さ 12、`rax=0x1`。stdout 297 行は、最小環境の node（`env -i`）の stdout と全文バイト同一**。通常環境の node との差は 16 行目の `rsp=` の値 1 行だけ。ブラウザで完走した 38 回（圧縮・取得方式・V8 設定・回線違いを含む）の stdout は互いにバイト同一（ページの MEMFS ルートを変えた 1 回を除く。§3.1） |
| 時間（高速版、既定の設定、loopback、n=8） | wasm の取得＋コンパイル＋インスタンス化（`instantiateStreaming`）**1.21〜1.29 s**（中央 1.27 s。389 MB の loopback 転送を含む）、実行（`main`）**2.80〜3.04 s**（中央 2.85 s）、Worker 全体 4.21〜4.52 s。node は 0.55〜0.58 s で instantiate 済み、3.09〜3.10 s で終了（§3.3） |
| メモリ | 最大の renderer プロセスの RSS ピーク **1.30〜1.42 GB**（実行中の定常 約 0.73 GB）。wasm の線形メモリは初期 640 MiB から終了時 838,402,048 B（約 800 MiB）。JS ヒープは使用 6.2 MB（MEMFS の配列 70.9 MB は backing store）。`performance.memory` は Worker では取れない（§3.4） |
| wasm の圧縮後サイズ | gzip -9 **112,199,383 B**（28.8 %、36.1 s）、brotli q11 **44,269,611 B**（11.4 %、583 s）、zstd -19 **54,290,375 B**（13.9 %、162.7 s）（§3.2） |
| V8／Chrome の上限 | **当たらなかった**。モジュール 389 MB／1 GiB（フラグ値）、関数 98,392／1,000,000、1 関数の locals 最大 41,126／50,000（`-O0` のままの 19 関数は 50,926〜397,472 で、Chrome でも 50,001 locals は検証失敗を実測）、メモリは 800 MiB／4 GiB。全関数を eager に Liftoff コンパイルしても失敗しない（renderer ピーク RSS 3.7〜4.0 GB）（§3.5） |
| 未改造ドライバー相当 | **実行していない**（理由 §3.6） |
| SharedArrayBuffer | **COOP/COEP なしでは使えない**（`crossOriginIsolated=false`、`SharedArrayBuffer` が未定義）。**この wasm は SAB を使わず、COOP/COEP なしの通常の HTTP で完走した**ので不要。COOP: same-origin + COEP: require-corp を付けると SAB・共有 wasm メモリ・AudioWorklet への SAB 受け渡しが動く（§3.7） |
| AudioWorklet | **可**（OfflineAudioContext + AudioWorkletNode で描画、先頭サンプル 0.5）。`AudioContext` は生成でき `suspended`。**音は出していない**。Worker 内には `AudioContext` が無い |
| OffscreenCanvas | **可**（2D の読み戻し、`convertToBlob`、WebGL2 は ANGLE／SwiftShader のソフトウェア描画。ページと Worker の両方）。WebGPU は `navigator.gpu` はあるが `requestAdapter()` が null |
| OPFS | **可**（`getDirectory`、`createWritable`。`createSyncAccessHandle` は Worker だけで、1 MiB の書き込みを確認）。`storage.estimate().quota` 4,294,967,296 B |
| 見つかった問題 | 大きな応答の `Cache.put()` が失敗（22 MB 以上。原因 未確認）、HTTP キャッシュ経由の 2 回目の高速化は確認できず（原因 未確認）、IndexedDB に `WebAssembly.Module` は保存できない（DataCloneError）、`DecompressionStream` は gzip／deflate だけ（§3.8） |
| Chrome 以外 | **Firefox／Safari は未検証** |

## 1. 環境（実測）

| 項目 | 値 |
|---|---|
| ブラウザ | `google-chrome --version` = **Google Chrome 156.0.8078.47**。CDP の `Browser` = Chrome/156.0.8078.47、`V8-Version` = 15.6.75.19、UA = HeadlessChrome/156.0.0.0。起動: `/usr/bin/google-chrome --headless=new --no-sandbox --disable-gpu --remote-debugging-port=<空き> --user-data-dir=/tmp/opencode/web/profiles/<名前>`（専用の一時プロファイル。ケースごとに新しい Chrome、終了後に削除）に、**依頼文にない 4 つの引数** `--no-first-run --no-default-browser-check --disable-dev-shm-usage --mute-audio` を足した（`/dev/shm` の空きが 742 MB しかなく他の作業が使っていたため、音を出さないため。§7） |
| マシン | Linux 7.0.0-38、16 論理 CPU、RAM は `free -m` で 19,845〜36,909 MB と作業中に変わった（仮想マシン。他の作業者の Playwright 製 headless Chrome が別に動いていた＝負荷は一定でない。load average 0.7〜1.4） |
| node／emcc | node v24.19.0（`~/.local/bin/node`）、emcc 6.0.12（`/opt/emsdk`） |
| 素材 | `wasm6-report.md`／`a4-six-module-wasm-verification/verification.md` と同じ（FLEngine `binaries/win64/FLEngine_x64.dll` SHA-256 `b2c64da4…`、dsp、Blend2D、QFC、freetype、ILTools、フォント 2 ファイル）。生成 C の SHA-256（FLEngine `e047beeb…`、m1〜m5 `6e9f5fdb…`、`733a6a50…`、`eaf6eacf…`、`b800faab…`、`33554184…`）は記録と**一致** |

## 2. 手順（実測）

### 2.1 土台と wasm の再ビルド

1. `git archive HEAD lift docs/investigations/a4-six-module-wasm/tools | tar -x -C /tmp/opencode/web/src`。`fe-lift`／`fe-pe` は `/tmp/opencode/wasmv/target/release/` のものをコピー（SHA-256 `3a6ff3c8…782467f6`、`743e2faf…66b570b0`＝記録と一致）。
2. `build/gen_web.sh rebase gen split merge`（`gen_two.sh` の薄いラッパー。モジュールの 6 構成は `a4-multi.sh` の `modules.txt` と同じ。パスだけ `/tmp/opencode/web/src` に付け替え）: 15.3 s、最大 RSS 711 MB。`realmod_table_gen.c` は `build_two_native.sh` の生成部をそのまま再現して作り、`/tmp/opencode/wasmv/on/…/realmod_table_gen.c` と `cmp` 一致。
3. `build/build.sh force`（`build_nmod_wasm.sh` の段 `rt harness`／`units`／`o1`／`link`、`VAR=force JOBS=2 SERIAL_MODS=m1 DELETE_SRC=1`）:

| 段 | wall | 最大 RSS |
|---|--:|--:|
| rt harness | 7 s | 75 MB |
| units（`-O0`） | 227 s | **8,825,784 KB（8.42 GiB）**（m1 dsp の先頭の単位） |
| o1（19 関数の `-O1` と `-rest`） | 468 s | 3,643,828 KB |
| link | 8 s | 1,691,200 KB |
| 合計（生成を除く） | **710 s（11 分 50 秒）** | |

過去の記録の「約 8 分」より長い（`o1` の段が記録の 300 s に対して 468 s。原因 未確認。負荷が一定でない機械での値）。locals が 50,000 を超える関数は `logs/list-all.tsv` で **19 個**（dsp 15 + Blend2D 4。記録と同じ）。

4. **SHA-256 の照合**: HEAD e071fc0 のまま作ると **`flengine6.wasm` 389,439,365 B、SHA-256 `d0f2f0a190baf28aa546ca9f8ad1e7c46e26c8f9dbd701df67f9297345049853`**（js は `8cb4aeec…` で記録と一致）。記録の `6e52a849…`（389,439,333 B）と**一致しない**。差は 32 B で、原因は HEAD の `lift/harness/a4.c` が a802043 の版から **a4 の note 行の文言だけ**（`finalised the units` → `zeroed it; the unit finalizers were not called`）を変えたため（コミット d7336ad。`git diff a802043 HEAD -- lift/harness/a4.c` で確認）。**a802043 の `a4.c` を同じフラグで `emcc -c` して差し替え、再リンクすると 389,439,333 B、SHA-256 `6e52a849b2aca06c3f78d43454ce32006105e2b12a2dbb10e8aa9372a590dcc7` で一致**（`build/relink_a4_a802043.sh` に手順。スクリプトとしては実行していない: 中間物の `.o` を削除した後に書いた。実測したのは同じコマンドを直接実行した版）。以降のブラウザ測定には、この**検証済みの SHA の wasm**を使った（a4 の note 行が旧文言になる点だけが HEAD のビルドと違う）。

### 2.2 ブラウザ用のリンク（wasm は同一）

`build/link.sh <out> <h dir> web`: `build_nmod_wasm.sh` の `stage_link` と同じフラグで、`-sENVIRONMENT=node -sNODERAWFS=1` を `-sENVIRONMENT=web,worker -sFORCE_FILESYSTEM=1 -sEXPORTED_RUNTIME_METHODS=FS,ENV` に換えて、同じオブジェクトをリンク（2.7 s、最大 RSS 1.69 GB）。出力の **wasm は node 用とバイト同一（SHA-256 `6e52a849…`）**。JS の接着コードだけが違う（167,043 B、SHA-256 `840dbb4514324970d71357e734d0932f50e7fd6fa12c59b2da65894ccdf2ab6d`、node 用は 186,067 B）。つまり「ブラウザで動く wasm」は「node で検証した wasm」そのもの。wasm の import は 24 個（`env.emscripten_return_address`、`invoke_*` 6 個、`_emscripten_throw_longjmp`、`__syscall_*`、`wasi_snapshot_preview1.*`）で、ブラウザの接着コードがすべて満たした。

### 2.3 ローダー（`lift/harness/web/fle-worker.js`、`index.html`）

`run_node_nmod.cjs` は NODERAWFS でホストのファイルを直接読む。ブラウザではこれを次のように置き換えた（ランナーの引数・環境変数は同じ）:

- ページが Web Worker を起動（メインスレッドはブロックしない）。
- Worker が 9 ファイル（FLEngine 22.2 MB、再配置済みモジュール 5 本、`vfs-seed-fonts.tsv`、フォント 2 本。合計 70,685,642 B）を `fetch` で並列に取得し、MEMFS に `FS.writeFile(…, {canOwn:true})`（コピーしない）で置く。`ENV.FLE_VFS_SEED`、`ENV.FLE_VFS_SEED_ROOT` を設定。引数は `<FLEngine dll> <5 本を ':' で連結>`。ホストの他のファイル・レジストリは見えない（MEMFS の 9 ファイルだけ）。
- wasm は `instantiateWasm` で自前に取得: `WebAssembly.instantiateStreaming`（既定）、`compileStreaming` + `instantiate`、`fetch` + `compile`（時間の内訳用）、Cache Storage 経由、のいずれか。`fast=1` で `imports.env.emscripten_return_address = () => 0`（`FLE_WASM_FAST_RETADDR=1` と同じ差し替え）。
- 進捗（`mark`: glue_loaded、data_fetched、wasm_start、wasm_instantiated、memfs_ready、main_start、main_end、done）と stdout／stderr の各行を `postMessage` でページに返す。a4 の行は stdout の最後に出る（ページでの到着は 1.50 s〜4.41 s の間に 297 行が順次、a4 の 7 行は 4.41 s 付近）。
- 測定のドライバー（`measure.mjs`）は 1 つの node スクリプトで、HTTP サーバー起動→Chrome 起動→CDP（node 標準の WebSocket。puppeteer 等は使っていない）→結果取得→Chrome とサーバーの終了、までをフォアグラウンドで行う。Chrome のプロセス木の RSS を `/proc` から 100 ms ごとに読む。

## 3. 結果

### 3.1 a4 が node と一致するか（実測）

| 比較 | 結果 |
|---|---|
| ブラウザの全 38 回（既定の設定 8 回、圧縮違い、`compile`／`buffer`、eager、`--liftoff-only`、`--no-liftoff`、遅い回線、PSS 付き、キャッシュ）の stdout（MEMFS のルートを node と同じ絶対パスにした回） | 38 ファイルが互いに**バイト同一** |
| ブラウザの stdout と、**最小環境の node**（`env -i FLE_VFS_SEED=… FLE_VFS_SEED_ROOT=… FLE_WASM_DIR=… FLE_WASM_FAST_RETADDR=1 node run_node_nmod.cjs …`、同じ wasm・同じ入力・同じ絶対パス） | **297 行、全文バイト同一**（`cmp`） |
| ブラウザと**通常環境の node** | 16 行目の `calling entry … rsp=0x11f7afc0`（node）／`0x11f79fc0`（ブラウザ）の**1 行だけ**が違う。`env -i` で node の値がブラウザと同じになったので、原因は環境変数ブロックの大きさ（読み: ゲストの初期スタック位置に効く）。a4 の 7 行・functions・entries・深さ・先頭 256 件の entry log は同じ |
| ブラウザの `root=/data`（ページの既定の MEMFS ルート）と node | 4 行が違う: `vfs seed:` の 3 行（パス文字列だけ）と `rsp=` |
| stderr | ブラウザの完走した 38 回すべて 0 行（`results` の各 json の `stderrLines`） |

a4 の 7 行（node の記録と同一。note 行は検証済み SHA の版＝旧文言）:

```
a4: note: InitContext+0 is 0 now (the program's exit path finalised the units); the table 0x13768b0 and init_count=942 are the last non-zero header and the largest InitCount sampled at lifted function entries
a4: init_count=942 of 942, last_unit_proc=0x13751d0
a4: last_unit index=940 run_rank=895 of 895 proc=0x13751d0 state=returned
a4: runnable_started=895 completed=895 in_progress=0 of 895 runnable
a4: unit[init_count-1] proc=0x0; units_outside_image=0
a4: entry_check started_but_not_entered=0 runnable_entered_beyond_init_count=0
a4: all_units_done=yes functions_distinct=5306 function_entries=1007942
```

### 3.2 取得とサイズ（実測）

| 形式 | サイズ | 作成時間 | 備考 |
|---|--:|--:|---|
| 非圧縮 | 389,439,333 B | | Code 節 387,190,488 B、関数 98,392、Data 1.7 MB、Elem 0.28 MB |
| gzip -9（`gzip` コマンド、1 スレッド） | 112,199,383 B（28.8 %） | 36.1 s | |
| brotli q11、窓 16 MiB（lgwin 24。ブラウザが受ける最大。node の zlib） | **44,269,611 B（11.4 %）** | 583 s（9 分 43 秒） | 最大 RSS 467 MB |
| zstd -19、窓 8 MiB（wlog 23。ブラウザが受ける最大） | 54,290,375 B（13.9 %） | 162.7 s | 最大 RSS 225 MB |

`Content-Encoding` を付けた配信（localhost）で、`instantiateStreaming` の所要（コンパイル込み。n=3 の範囲）:

| 配信 | loopback（無制限） | 12.5 MB/s（100 Mbit/s 相当に絞ったサーバー） |
|---|--:|--:|
| 非圧縮 | 1.21〜1.29 s（n=8） | **29.9 s**（1 回） |
| gzip | 0.81〜0.84 s | 8.7 s（1 回） |
| brotli | 0.93〜0.99 s | **3.5〜3.6 s**（2 回）。125 MB/s では 0.92 s（1 回） |
| zstd | 0.72〜0.84 s | 4.3 s（1 回） |

Chrome 156 は `Content-Encoding` の gzip／br／zstd をネットワーク層で復号し、`instantiateStreaming` はそれをそのまま受けた。回線の絞りはサーバー側のトークン制限（`serve.mjs` の `rate=`）で、実ネットワーク（遅延、CDN）の値ではない。取得中にコンパイル（検証）が並行するので、非圧縮では「取得にかかる時間 ≒ 準備完了までの時間」だった（遅い回線での 29.9 s も転送時間とほぼ同じ）。

### 3.3 コンパイル・インスタンス化・実行の時間（実測）

内訳（非圧縮、loopback、1 回ずつ）: `fetch`+`arrayBuffer` 1.26 s（gzip 0.82 s、br 0.97 s）→ **`WebAssembly.compile(buffer)` 0.215〜0.235 s**（3 形式で 3 回）→ `WebAssembly.instantiate(module, imports)` 7〜10 ms。`compileStreaming` は 1.26 s（取得に重なる）。つまり**取得後に残るコンパイルは 0.2〜0.24 s**で、V8 は既定で関数を**遅延コンパイル**する（`--wasm-lazy-compilation` が既定で有効。実測した V8 フラグ一覧）。実行中に最初に呼ばれた関数が Liftoff でコンパイルされる。

CDP の Tracing（`v8.wasm`、`disabled-by-default-v8.wasm.detailed`、1 回）:

| 項目 | 値 |
|---|--:|
| 遅延コンパイル（Liftoff）された関数 | **7,614**（定義 98,392 のうち 7.7 %）。本体の合計 **9,636,662 B（Code 節の 2.49 %）**。最大の本体 566,069 B |
| 遅延コンパイルの時間の合計（全スレッド） | CompileBaseline 720 ms |
| TurboFan への tier-up | 394 関数、527 ms（全スレッドの合計） |
| ストリーミング検証 | 286 イベント、合計 904 ms（全スレッドの合計） |
| `memory.grow` | 1 回、62.5 ms（640 MiB → 約 800 MiB） |

実行の時間（`onRuntimeInitialized`〜`onExit`）は、既定の設定で 2.80〜3.04 s（n=8）。node は同じ wasm・同じ入力で、read 0.19 s、instantiate 済み 0.55〜0.58 s、終了 3.09〜3.10 s（`FLE_WASM_TIMING`、3 回）、wall 3.20〜3.23 s、最大 RSS 1.25 GB（`results/node-baseline.txt`）。差し引きの実行部分は node 約 2.5 s、ブラウザ約 2.85 s（別の形の時計の差し引きなので精密な比較ではない）。

**tiering の違い（V8 のフラグ。Web ページからは変えられない。開発者向けの測定）**:

| 設定 | 準備完了 | 実行 | renderer の CPU | renderer ピーク RSS |
|---|--:|--:|--:|--:|
| 既定（遅延 Liftoff + 動的 tier-up） | 1.27 s | 2.85 s | 5.2 s | 1.37 GB |
| `--liftoff-only` | 1.25〜1.32 s | 3.89 s | 5.8 s | 1.29 GB |
| `--no-liftoff`（TurboFan のみ、遅延） | 1.24 s | 9.07 s | 11.0 s | 1.42 GB |
| `--no-wasm-lazy-compilation`（全関数を eager に Liftoff） | **12.74 s** | 2.15 s | **58.4 s** | **3.72 GB**（br 配信 4.01 GB、12.78 s） |

動的 tier-up は実行を約 1 s 短くした（既定 2.85 s、`--liftoff-only` 3.89 s）。eager にすると準備に 12.7 s かかるが失敗はしない。

**測定そのものが結果を変えた**: `/proc/<pid>/smaps_rollup`（PSS）を renderer に対し 100 ms ごとに読むと、renderer が遅くなる。同じ設定で準備 1.9〜2.2 s（PSS なしは 1.2〜1.3 s）、eager は **77.7 s（PSS なしは 12.7 s）**。この PSS 付きの回は `runs/perturbed/` に分けて使わず、時間の表はすべて PSS なし（RSS のみ）の回。PSS は 1 回だけ 250 ms ごとに取った（次節）。

### 3.4 メモリ（実測）

最大の renderer プロセス（wasm を動かす Worker のあるプロセス）の RSS の推移（PSS 付き 250 ms の 1 回）: 開始 77 MB → 取得中に増加（1.5 s で 918 MB）→ **1.8 s にピーク 1,382 MB（匿名 PSS 1,282 MB）**→ 2.0 s に 698 MB → 実行中に最大 1,177 MB → 定常 730 MB → 終了後 750 MB。ピークは `instantiateStreaming` が終わる付近で、その後に下がる（読み: ストリーミングの受信バッファの解放。確認していない）。定常の約 0.73 GB の内訳は未分解（読み: V8 が遅延コンパイルのために保持する wasm のバイト列 389 MB、MEMFS の 70.9 MB、触れた線形メモリ、生成コード）。

| 項目 | 値 |
|---|---|
| renderer ピーク RSS（PSS なし 100 ms） | `instantiateStreaming` の遅延コンパイル設定 30 回: 1,263〜1,422 MB（既定の設定 8 回: 1,303〜1,419 MB）。`buffer` 方式は ArrayBuffer を JS 側にも持つため 1,558〜1,625 MB。Chrome の全プロセス木の RSS の合計のピークは 2.3〜2.8 GB（待機中の予備 renderer 約 100 MB が数本を含む） |
| Worker の JS ヒープ（CDP `Runtime.getHeapUsage`、実行後） | used 6,182,208 B、total 11,800,576 B、backing store 70,860,788 B（MEMFS に渡した 9 ファイル） |
| 線形メモリ | 初期 640 MiB（671,088,640 B）→ 終了時 838,402,048 B。モジュールの max は 2 GiB |
| `performance.memory` | Worker では null（取れない） |
| 取得データ（70.7 MB）の `fetch` | 0.18〜0.22 s |

### 3.5 V8／Chrome の上限（実測）

Chrome 156 の V8 のフラグ値は `--js-flags=--help`（`runs/v8-help.txt`）、locals とメモリは Chrome 内で `WebAssembly.validate`／`WebAssembly.Memory` を実行して確認した。

| 上限 | Chrome 156（V8 15.6.75.19） | この wasm | 余裕 |
|---|---|---|---|
| モジュールの大きさ | `--wasm-max-module-size` = 1,073,741,824 B | 389,439,333 B | 36.3 % |
| 関数の数 | `--max-wasm-functions` = 1,000,000 | 98,392 | 9.8 % |
| 1 関数の locals | 50,000 は検証成功、**50,001 は失敗**（合成モジュールで実測） | `-O1` 分離後の最大 41,126 | 82 %。`-O0` のままの 19 関数（50,926〜397,472）は Chrome でも通らない |
| 線形メモリ（wasm32） | 65,536 ページ（4 GiB）まで。初期 1,024／2,048／4,096 MiB の確保は成功、max 65,537 は拒否 | 初期 10,240 ページ（640 MiB）、max 32,768（2 GiB）、使用 約 800 MiB | 実使用は 4 GiB の 20 % |
| wasm のコード領域 | 1 領域 `--wasm-max-code-space-size-mb`=1024、総量 `--wasm-max-committed-code-mb`=4095 | 既定の遅延コンパイルで生成コードは小さい。eager に全関数を Liftoff しても失敗しない（renderer ピーク RSS 3.7〜4.0 GB） | eager での実際のコード量は未測定 |
| テーブルの大きさ | `--wasm-max-table-size` = 10,000,000 | Elem 節 1 本（0.28 MB） | 十分 |
| 同期コンパイル | メインスレッドでの同期 `new WebAssembly.Module` に大きさの制限があるはず（読み。未確認） | Worker で非同期に行った | 影響なし |

当たった上限は無い。当たったのは**自分で設定した評価基準**（1 回の準備と実行が数秒）に対する eager コンパイルの 12.7 s／3.7 GB だけで、既定の設定では起きない。

### 3.6 未改造ドライバー相当（実行していない）

依頼どおり実行していない。理由: node で 30 分（独立再現 29:34〜30:02、測定担当 32:54）かかる見込みで、原因は `-finstrument-functions` の各フックが呼ぶ `emscripten_return_address`（`new Error().stack` の解析）が 235,794,590 回あること（`wasm6-report.md` §5.3）。ブラウザの Chrome ではこの種の呼び出しの単価が別なので、**参考値だけ**測った: Worker 内の JS 微小ベンチ（呼び出しの深さ 12+3、20,000 回、`features.js`）は **2.69〜3.17 µs／回**（4 回）。node では同じ種類の呼び出しが 5.5 µs（`run_node_nmod.cjs` のコメント）。これに 235.8 M 回を掛けると **約 11〜12.5 分（読み）**。node では微小ベンチ 5.5 µs × 235.8 M = 1,297 s に対して実測 1,974 s（1.52 倍）だったので、同じ比が成り立てば Chrome では約 16〜19 分（読み。どちらも未実行）。実行は高速版だけ（出力は node で未改造版と同一と確認済み）。

### 3.7 機能テスト（実測。`features.html`、ページと Worker の両方、COOP/COEP なし／あり）

| 機能 | COOP/COEP なし | COOP: same-origin + COEP: require-corp |
|---|---|---|
| `crossOriginIsolated` | false | **true** |
| `SharedArrayBuffer`（ページ・Worker） | **未定義**（`new SharedArrayBuffer` は ReferenceError） | 使える |
| 共有 wasm メモリ（`shared: true`） | 作れない | 作れる |
| SAB を AudioWorklet に渡す | 不可（SAB が無い） | 可（描画中に `process()` が SAB 経由で数えられた） |
| wasm の機能の検証（`WebAssembly.validate`） | simd、threads、bulk memory、例外（legacy）、memory64、tail call、multivalue、reference types、extended-const、multi-memory、relaxed-simd: すべて true（GC と exnref は未プローブ）。JSPI（`WebAssembly.Suspending`／`promising`）あり | 同じ |
| AudioWorklet | `OfflineAudioContext` + `audioWorklet.addModule` + `AudioWorkletNode` で描画でき、先頭サンプルが 0.5。`AudioContext` は生成でき `state = suspended`、sampleRate 48,000、baseLatency 0.0107 s（`resume()` しない・音は出さない）。Worker 内に `AudioContext`／`AudioWorkletNode` は無い | 同じ |
| OffscreenCanvas | ページ・Worker の両方で 2D の描画と `getImageData` の読み戻し（255,0,0,255）、`convertToBlob`、WebGL1／2（ANGLE／SwiftShader のソフトウェア）、`transferControlToOffscreen`（ページ）。WebGPU は adapter が null | 同じ |
| OPFS | `getDirectory` 可、`createWritable` 可（5 B の往復）、`createSyncAccessHandle` は **Worker だけ**（1 MiB の書き込み・サイズ確認）。`storage.estimate()` quota 4,294,967,296 B（プロファイルが tmpfs。ディスクの空きに依存する値、読み）、persist() false | 同じ |
| IndexedDB／Cache Storage | 1 MiB の往復 可。`WebAssembly.Module` の保存は DataCloneError | 同じ |
| `DecompressionStream` | `gzip`、`deflate` は構築できる。**`brotli`、`zstd` は TypeError**（未対応） | 同じ |
| `Atomics.wait`／`waitAsync` | あり | あり |

この wasm は `-pthread` なしの単一スレッドなので、**SAB（COOP/COEP）は要らない**。実際に COOP/COEP なしで完走した（§3.1）。将来スレッドや AudioWorklet との共有リングバッファを使う場合に要る。

### 3.8 キャッシュ（実測。うまくいかなかったものを含む）

| 方法 | 結果 |
|---|---|
| HTTP キャッシュ（`Cache-Control: public, max-age=3600`、brotli 44 MB、同じプロファイルで 3 回） | **2・3 回目もサーバーに wasm の要求が来た**（サーバー側の応答時間 182 ms と 727 ms）。準備完了は 1.05／1.03／1.00 s で、**高速化は確認できなかった**。プロファイルは 51→93→94 MB に増えた（何が保存されたかは未確認）。原因 未確認 |
| Cache Storage（`cache.put(url, await fetch(…))`） | **22 MB 以上で `NetworkError: Cache.put() encountered a network error`**（8 KB〜3.6 MB は成功）。`--disable-dev-shm-usage` の有無で変わらない。原因 未確認。wasm（389／44 MB）では使えなかった |
| IndexedDB（非圧縮 389 MB の ArrayBuffer） | **成功**: put 1.35 s、get 0.56 s、get 後の `compile` 0.23 s（Blob でも同程度）。`estimate().quota` 4.68 GB。コンパイル済みコードは保存できない（`WebAssembly.Module` は DataCloneError）ので、訪問のたびに遅延コンパイルはやり直し。プロファイル 379 MB を使う |

## 4. 失敗・不具合・観測された問題（時系列、実測）

1. **ドライバーの不具合で 30 分を無駄にした**: gzip 配信のケースで、`serve.mjs` が `?enc=` を `index.html` にも適用して 404 になり、ページが起動せず 15 分のタイムアウトを 2 ケース分待った（ツールの上限 30 分で打ち切り。残った Chrome プロセスは無いことを確認）。修正後は `.wasm` にだけ適用。結果に影響なし。
2. 最初の RSS の集計は Chrome の子プロセスの種別を取り違えた（`/proc/<pid>/cmdline` が 1 本の文字列で `--type=` を見つけられなかった）。正規表現に直した。
3. 時間の測定が測定自体で変わった（§3.3）。PSS の読み取りを外した。
4. `Cache.put()` の失敗、HTTP キャッシュの無効果（§3.8）。原因を調べていない。
5. memory64 の構築テストは、プローブ側の引数（BigInt）の誤りで失敗した（検証だけが true）。`WebAssembly.Memory({address:'i64'})` の実行は未確認。
6. V8 のフラグ `--trace-wasm-lazy-compilation` は、renderer の標準出力が取れず出力なし。CDP の Tracing に切り替えた。
7. 初回の wasm ビルドの `o1` 段が記録より長い（§2.1）。

## 5. 選択肢（見積もり付き。**決めない・実装しない**）

当たった上限は無いので、観測した負担（取得の大きさ、メモリ、再訪問、初期化後の拡張）への選択肢として書く。見積もりは実測の数値からの読みで、実装した場合の値ではない。

| 選択肢 | 見積もり（根拠） |
|---|---|
| **A. 圧縮配信**（br 44.3 MB／zstd 54.3 MB／gzip 112.2 MB） | 100 Mbit/s 相当の絞りでの準備完了の実測: 非圧縮 29.9 s、gzip 8.7 s、**br 3.5〜3.6 s**、zstd 4.3 s。これは実測。実ネットワークの遅延・CDN は含まない。br q11 は 583 s（ビルド時に 1 回）。Firefox／Safari での br／zstd の扱いは未確認 |
| **B. 分割・遅延読み込み**（機能単位で wasm を分け、必要になった時に取得） | a4 の初期化経路が実際に使うのは 7,614 関数・本体 9.6 MB（Code 節の 2.5 %）。非コード部分（Data 1.7 MB、Elem 0.28 MB、Function 節 0.1 MB）を足して約 12 MB（読み）。**初期化だけなら理論上 97 % 減らせる**が、wasm には関数単位の遅延取得が無く、モジュール境界（FLEngine＋5 モジュールは別の翻訳単位）での分割と、モジュール間の呼び出し（`common.c` の 91,667 行の表、間接呼び出し）の仕組みが要る。工数は見積もれない。FL Studio の実際の使用（UI・音）が触る関数の割合は未測定 |
| **C. 再訪問の高速化**（IndexedDB、OPFS） | IndexedDB の実測: 389 MB を put 1.35 s、get 0.56 s、compile 0.23 s（ネットワーク無し）。保存に 389 MB を使う。圧縮のまま保存するなら復号が要るが、`DecompressionStream` が br／zstd 未対応なので JS／wasm の復号器が要る（未測定）。OPFS は使えることだけ確認（wasm のバイト列では未測定）。HTTP キャッシュと Cache Storage は今回の設定では効果なし／失敗（§3.8）で、原因の調査が先 |
| **D. tiering／遅延コンパイル** | 既定（遅延 Liftoff + 動的 tier-up）が最良: 準備 1.27 s、実行 2.85 s。eager は 12.7 s・58 s の CPU・3.7〜4.0 GB で不利。ページから V8 のフラグは変えられないので、選べるのは「何を一度に読み込むか」（B）だけ。Chrome の wasm コードキャッシュ（`SerializeModule` イベントが 1 回あった）の効果は、キャッシュが効かなかったため未確認 |
| **E. `-O1`／`-O2`／`-Os` でのサイズ削減** | 未測定。現行は全体 `-O0`＋19 関数だけ `-O1`。`o1` 段（`-rest` 3 単位の `-O0` と 19 関数の `-O1`）だけで 468 s かかった（`units` 段＝約 60 単位＋`common.c` の `-O0` は 227 s）ので、全単位を上げるとビルド時間は大きく増える見込み（読み。測っていない）。`-O0` では 19 関数が 50,000 locals を超えて Chrome でも通らないという制約が残る（`-O1` で解消済み） |
| **F. メモリ 64 ビット** | 不要に見える: 使用 約 800 MiB に対して wasm32 の上限は 4 GiB（Chrome で初期 4,096 MiB の確保まで成功）。memory64 は Chrome 156 で検証は通る（実行は未確認）。使うなら 1 回の `memory.grow` が約 62 ms（実測）である点と、境界検査・トラップ処理の違いの評価が要る |
| **G. 未改造ドライバーの 30 分** | 高速版（`emscripten_return_address` を 0 にする）は出力が同一（node で確認、ブラウザでは高速版のみ）。計装を伴わないビルドにする選択もあるが、a4 の計数（`function_entries` 等）が計装に依存する。工数・効果は未見積もり |
| **H. スレッド化** | COOP/COEP の付与で SAB・共有メモリ・AudioWorklet への共有が使える（実測）。この wasm 自体は単一スレッドで、スレッド化の工数は見積もれない |

## 6. 限界（未検証・未確認）

- **Firefox／Safari（および Chrome 以外の全ブラウザ）は未検証**。wasm の上限（モジュールの大きさ、関数の数、locals）、遅延コンパイルの有無、`Content-Encoding` の br／zstd、OPFS、AudioWorklet は他の実装で別の値・別の挙動になりうる。
- Chrome は **156 の headless（`--headless=new`、Linux x86_64、ソフトウェア描画）1 種類**。headful、Windows、macOS、モバイル、他のバージョンは未測定。GPU・実音声デバイスは使っていない（AudioContext は `suspended` のまま閉じた）。
- ネットワークは **localhost のみ**（遅い回線はサーバー側の転送制限で模擬）。実回線、CDN、TLS、HTTP/2 は未測定。取得の時間にはサーバーとブラウザが同じ機械の CPU を取り合う影響がある（読み）。
- 測ったのは **a4（FLEngine の初期化 942 ユニット）の経路だけ**。FL Studio の UI・音声・プロジェクト読み込みの経路は一度も動かしていない。メモリ・遅延コンパイルされる関数の数・実行時間はそれらでは大きく変わりうる。a4 は仮の応答 47 件（数えているのは仮の決定の数で、決定 0101 の N。README の `**仮**` の行数。決定 0115 を含む。**この文書の初版は 46 と書いていたが、作業の基準 HEAD e071fc0 の時点で既に 47 だった**: `git show e071fc0:docs/README.md` で 47、決定 0115 を足す前の HEAD 0408644 で 46、HEAD 9aa3615 で 47）、測定用の基底、フォント 2 ファイル、ページ保護 fail-open（決定 0045）、`-O1` の 19 関数が 1 回も実行されない、という前提の上（決定 0115）。
- 未改造ドライバーは実行していない（§3.6）。Chrome の値は微小ベンチからの読み。
- 負荷が一定でない機械（他の作業者の Chrome が同時に動いていた）の値。ケースの繰り返しは多くが 1〜3 回で、分散の評価は粗い（既定の設定だけ n=8）。`R-br-rate12.5` の 1 回は実行が 4.36 s（他の回は 2.7〜2.9 s）と長く、原因 未確認。
- COOP/COEP の有無は同一オリジン内だけ（クロスオリジンの subresource は試していない）。
- `performance.memory` は Worker で無く、V8 のヒープの内訳（wasm コードの大きさ）は直接測っていない（RSS と CDP の `getHeapUsage` のみ）。
- GC と exnref の wasm 機能はプローブしていない。memory64 は検証のみ。
- ワークツリーの `lift/out-*`（コミットされていない素材）に依存する。`wasm-validate` は実行していない（Chrome の受理と実行で代用）。

## 7. 規則違反・逸脱の自己申告

- **`timeout` コマンドをコマンド文字列に 1 回書いた**（V8 フラグの一覧を取ろうとして）。権限で拒否され実行されていない。以後は node スクリプトが自分で子プロセスを終了する形（`v8flags.mjs`）に変えた。それ以外に sleep／nohup／末尾の `&`／バックグラウンド実行は使っていない（`sleep` 関数は node スクリプトの中のタイマーとして使用）。
- **Chrome の引数を依頼文より増やした**: `--no-first-run --no-default-browser-check --disable-dev-shm-usage --mute-audio`（§1）。`--js-flags=…` は V8 の比較測定（§3.3）と V8 のフラグ一覧の取得のときだけ。キャッシュの 1 ケースだけ `--disable-dev-shm-usage` を外した（`/dev/shm` に最大 22 MB 程度。他の作業者の分を圧迫しない範囲）。
- **`ps aux | grep -i chrome` を 1 回実行**し、他の作業者の Playwright 製 `chrome-headless-shell` のコマンド行が表示された（読み取りのみ。触れていない）。自分の Chrome のプロセスの終了と、自分のプロファイル配下以外のプロセスに信号を送っていないことは確認した。依頼者の Chrome プロファイル・ログイン状態には触れていない。
- **`/tmp/opencode/wasmv/` を読み取り、`target/release/fe-lift`、`fe-pe`（約 9 MB）を自分の `web/tools/` にコピー**した。`modules.txt` と `realmod_table_gen.c` は読んで比較しただけ。削除はしていない。
- **容量**: `/tmp/opencode/web` の `du -sm` の観測最大は **1,357 MB**（wasm が 2 本あった時。枠 1.5 GB 以内）。この時点の `/tmp/opencode` 全体の `du` は取っていない（直前の読みから約 2.0 GB と推測。枠 2.3 GB 以内、**測定していない**）。実測できた全体の最大は 1,654 MB（`build.sh` の link 段）。`No space left` は 0 回。`/` への書き込みは本書と `lift/harness/web/` のみ（合計 約 170 KB）。`/` の空きは 441 MB → 433 MB（他の作業者の書き込みを含む）。
- 誤って `harness-web/*` を `dist/` にコピーし、すぐ削除した。
- 中間の `ad hoc` な `python3 - <<EOF` に `PYTHONDONTWRITEBYTECODE=1` を付けなかった回がある（標準入力のスクリプトで `.pyc` は作られない。`web/` とリポジトリに `__pycache__` が無いことを確認）。ビルドスクリプトは `export` している。
- `relink_a4_a802043.sh` は、実行した同じコマンドをまとめて書いたもの。スクリプトとしては実行していない（`.o` を削除済み）。`build/` の他のスクリプトは、実際に実行したものの写し。
- AGENTS.md の「作業は task agent を通じて」は、私がサブエージェントで task ツールを持たないため、自分で作業した。リポジトリのソースの変更、git の変更系操作、commit はない。`rm` は `/tmp/opencode/web/` の下だけ。

## 8. 成果物と再現

- 本書。`lift/harness/web/`: `README.md`、`fle-worker.js`、`index.html`（ローダー）、`serve.mjs`、`cdp.mjs`、`measure.mjs`、`suite.sh`、`summarize.mjs`、`compress.mjs`、`v8flags.mjs`、`features.html`／`features.js`、`cachetest.html`、`idbtest.html`、`build/`（`gen_web.sh`、`wb.sh`、`build.sh`、`link.sh`、`relink_a4_a802043.sh`、`wrap/python3`）、`results/`（`results-summary.tsv` 37 行、`node-baseline.txt`、`trace-summary.txt`、`compression.json`、`compression-zstd.txt`、`build-stages.txt`、`features-plain.json`、`features-coi.json`）。
- 一時物（RAM 上）: `/tmp/opencode/web/{dist,runs,logs}`。`runs/<名前>.json`（各ケースの全記録）、`.stdout`、`runs/perturbed/`（PSS の読み取りで遅くなった回）。
- 再現（依頼者の環境で）: ①`build/gen_web.sh` → `build/build.sh force logs/build1`（約 12 分、最大 RSS 8.8 GB）→ `build/relink_a4_a802043.sh <repo> dist web`（SHA を `6e52a849…` に合わせる場合）、② `dist/data/` に再配置済みの 6 ファイル＋フォント、③ `node lift/harness/web/measure.mjs run name=x fast=1 mode=instantiate enc=identity root=<dist>/data`。
