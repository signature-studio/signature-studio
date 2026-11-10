# 実 6 モジュール wasm(flengine6.wasm)での JSPI による中断・再開の測定

作業日: 2026-10-29〜30。基準: ブランチ main、`git archive HEAD` の展開は **159a42e**(作業中に別の作業者が c7d65bf を commit した。`jspi.diff` は 159a42e と c7d65bf の両方の `lift/harness` に `patch -p1 --dry-run` で当たることを確認したが、c7d65bf では wasm を作り直していない)。**この文書は何も決めない**(決めるのは PM)。リポジトリの追跡ファイルは編集していない。新規は `lift/harness/web/jspi/`、この文書、`lift/out-git/jspi-results/`(gitignore 対象)だけ。作業場所は `lift/out-jspi/`(終了時に大きなものを削除。§11)。

凡例: **実測**＝このセッションで実行したコマンドの出力。**読み**＝実測や文書・コードからの推論(確かめていない)。**未確認**＝調べていない。

## 0. 結論(先に)

| 項目 | 結果 |
|---|---|
| 実 wasm で JSPI のビルド | **成功**。`-sJSPI` + **`-sSUPPORT_LONGJMP=wasm`**(setjmp/longjmp を JS の `invoke_*` から wasm の例外処理命令へ)の組み合わせ。`flengine6.wasm` 389,438,311 B(JSPI なしの基準 389,440,863 B)。**emscripten 既定の setjmp/longjmp のままでは使えない**(§6 F1: 中断を 1 回も入れなくても `SuspendError: trying to suspend JS frames` で止まる) |
| 中断なしの a4 | **基準と一致**(実測)。`init_count=942 of 942`、895/895、functions 5,306、entries 1,007,942、深さ 12、終了コード 0、stderr 0 B、stdout は native と `calling entry … rsp=` の 1 行以外同一。node 24.19.0 と headless Chrome 156 の両方 |
| 中断あり(代表 API)の a4 | **一致**(実測)。6 種(`EnterCriticalSection` 681 回、`WaitForSingleObject` 8、`AcquireSRWLockExclusive` 1、`LoadLibraryW` 34、`ReadFile` 3、`CreateFileW` 3)のそれぞれを「1 回目だけ」(微小タスク)と「全呼び出し」(微小タスク / `setTimeout(0)` / `MessageChannel` 往復の 3 通り)で中断。さらに **lifted 関数の全入口 1,007,942 回で中断**しても一致。未改造ドライバー(32:53.87)で 730 回の API 中断を入れた完走も一致 |
| 1 回の中断・再開のコスト(node、p50) | 微小タスク(`await Promise.resolve()`): **0.22 µs**(全入口 1,007,942 回。同期 import の対照は 0.05 µs)。同一スレッドの `MessageChannel` 往復: 1.05 µs(100,795 回)〜3.7 µs(API 681 回)。`setTimeout(0)`: **1,103 µs**(タイマーの下限)。p95/p99 は §5.3。**プロセス最初の中断だけ 86〜145 µs**。Chrome 156(COOP/COEP、時計の刻み 5 µs): `MessageChannel` 往復 p50 5 µs、p99 25 µs(100,795 回) |
| setjmp 着地の内側で中断 | **可**(SJLJ=wasm)。全入口の 1,007,939/1,007,942 回は生きている着地(C の `setjmp` フレーム)の内側、800,398 回は 2 重の内側。ゲスト自身の `longjmp` 3 回(QFC、ILTools、FLEngine の終了。決定 0115 / verification の読み。wasm 側で個別には観測していない)は、全入口で中断した実行でも a4 が一致したので通ったと**読む**。**不可**: SJLJ=emscripten(JS の `invoke_*` が JS フレームになる)。着地の内側の中断は全部失敗、外側(`CreateFileW`/`ReadFile` の 1〜2 回目)だけ成功(§6 F2) |
| 時間・サイズ・RSS | 同じ `.o` から `-sJSPI` の有無だけ変えて再リンクした 2 つの wasm は**バイト同一**(SHA-256 `881cc2e0…`。js グルーだけ 183,273 → 186,730 B)。SJLJ を JS 方式から wasm 方式にすると 389,440,863 → 389,434,341 B(−6,522 B)。V8 の `WebAssembly.compile` は 328〜371 ms(A、C0、C1、C2、C のどれも。3 回ずつ)で JSPI の前後に差なし。exnref 符号化の D だけ 386〜391 ms(2 回)。**ただし実行が約 +46 %**(node 2.4 s → 3.6 s): 原因は JSPI の中断ではなく、`-finstrument-functions` のフックが呼ぶ JS の import `emscripten_return_address`(235,794,450 回)が JSPI 構成で 1 回あたり約 +4.8 ns(差 1.13 s ÷ 回数)になること(§5.5。トイの測定では JS import 1 回 +約 6〜7 ns)。この import を wasm 内の関数にすると JSPI の有無で差がなくなる。最大 RSS は node で 1.1〜1.25 GB のまま |
| JSPI のスタック上限 | V8 の既定 984 kB に対し、a4 の経路は **150 kB で通り**、145 kB(全入口で中断した実行)と 140 kB(中断なしの実行)は `RangeError: Maximum call stack size exceeded`。JSPI なしの基準も、メインスタックを `--stack-size` で絞ると 160 kB 可 / 155 kB 不可で、必要量は同程度。Chrome 156 でも同じ(150 kB 可、140 kB 不可)。a4 の経路の lifted 関数の入口での最大深さは wasm フレーム 68(JS を含め 73)、ガストのスタック使用は最大 28,592 B。実エンジンのメッセージループの深さは**未測定** |
| Chrome(headless、1 構成) | 26 回(JSPI なし 3、JSPI 23)。**JSPI のスタックを 140 kB / 100 kB に絞った 2 回(意図した失敗)以外の 24 回は a4 一致**、stdout は node の基準と `vfs seed:` のパス 3 行と `rsp=` の 1 行(計 4 行)以外同一。JSPI + SJLJ=wasm(legacy)と exnref の両方、中断なし / API 730 回 / 全入口 1,007,942 回を含む(§7) |
| 実際にブロックする API | **a4 の完走には無い**(171 種のトレースに `Sleep`、`GetMessageW`、`PeekMessageW`、`WaitMessage`、`MsgWaitForMultipleObjects`、`SwitchToThread` などは 0 種)。呼ばれる待ちは、すべて即座に返る `WaitForSingleObject(INFINITE)` 8 回(信号済みのイベント)と、単一スレッドで競合しない `EnterCriticalSection` 681 回。**実際のメッセージループとの接続は未確認** |

## 1. 環境・入力・手順

| 項目 | 値(実測) |
|---|---|
| ツール | emcc 6.0.12(`/opt/emsdk`。`EM_CACHE` は `lift/out-jspi/emcache`、`/opt/emsdk` のキャッシュには書いていない)、node v24.19.0(V8 13.6.233.17-node.51)、clang 21.1.8、wabt 1.0.36、python 3.14.8、Chrome 156.0.8078.47(V8 15.6.75.19) |
| マシン | 16 論理 CPU。`free` の total は作業中に 12,991〜26,465 MB と変わった。他の作業者のビルドや cargo が同時に動いていた(`ps` で他の emcc が見えた時がある)。時間の絶対値は負荷に依存する |
| 入力 | FLEngine `binaries/win64/FLEngine_x64.dll`(SHA-256 `b2c64da4…f9496d`)、dsp `lift/out-ext/dsp_ippv2_x64.dll`(`497dd4f7…`)、Blend2D(`66634d24…`)、QFC(`afc5a0ed…`)、freetype(`6ea6c106…`)、ILTools(`20409dcb…`)、フォント 2 ファイル(`588c4bf4…`、`578b46df…`)。**再起動の後に、これらが存在し SHA-256 が上と一致することを確認してから続けた**。種 `seeds-idx0-plus-initterm2.txt`(`cf664439…`)、`seeds58-4700000.txt`(`ca6439b6…`)も存在 |
| fe-lift / fe-pe | `cargo build --release --offline --locked -p fe-lift -p fe-pe`(`lift/out-jspi/target`)。fe-lift `e93f5982…`、fe-pe `743e2faf…`(fe-pe は以前の検証の値と同じ。fe-lift は HEAD が違うので別) |
| 生成 C | FLEngine の生成 C の SHA-256 `040bc98867fb473944964dd346a2fe896cabc48f53c67a8fd0bd879ad31b6a42`(決定 0116 に書かれた値と一致。`EXPECT_FLE_SHA=skip` で作った)。分割 60 単位 + `common.c`。wasm 用の生成・分割・結合は 14.4 s、最大 RSS 711 MB |
| 基準の native | `a4-multi.sh`(無改造、`JOBS=2`)を新しい OUT で一から: gen 16 s + build 201 s(全体 3:37.77、最大 RSS 4.9 GB)。a4 の 7 行は決定 0113 の値と同じ。直接実行 2 回: 終了コード 0、stdout 297 行、stderr 0 B、0.57 s、RSS 187 MB、2 回は `rsp=` の行だけ違う。**native の SHA-256 は比べていない**: 私のビルドは clang ラッパー(`-g` を外す)を使わず別ディレクトリなので、`two-native` の SHA-256 `bcb660c5…` は以前の記録の値と比べられない |
| 基準の wasm(A) | HEAD の `build_nmod_wasm.sh` を**無改造**で `VAR=force`: `flengine6.wasm` **389,440,863 B**、SHA-256 `2580930ff5b8b723bc75b353716cb18b761bc603c97b326aa5654a28f9096bcf`、js 186,067 B(`8cb4aeec…`)。`wasm6`/`verification` の記録(389,439,333 B、`6e52a849…`)とは違う。決定 0116 などで FLEngine の thunk が変わった後のビルドなので差は想定内(**読み**)。`verification.md` より新しい SHA の記録は見つからなかった(`grep` で確認)ので、比べる対象が無い |
| 基準の node(A) | 高速ドライバー(`FLE_WASM_FAST_RETADDR=1`)2 回: `init_count=942 of 942`、`all_units_done=yes`、895/895、`returned from entry`、functions 5,306、entries 1,007,942、深さ 12、`rax=0x1`、終了コード 0、stderr 0 B、3.27 s / 3.11 s、最大 RSS 1.24 / 1.25 GB。stdout は native と `rsp=` の 1 行(wasm は `0x11f7afc0`)以外同一。未改造ドライバーの基準(JSPI なし)は**今回は走らせていない**(JSPI ありの未改造は 1 回走らせた。§5.1) |
| locals 超過 | `o1: 19 functions over 50000 locals in 3 units`(dsp 15 + Blend2D 4。決定 0115 項 4 と同じ。`-O1` の別単位。この 19 関数は a4 の経路で呼ばれない) |
| ビルド時間(A) | `rt` 6 s、`harness` 1 s、`units` 218 s(最大 RSS 8.83 GB = `m1-00`、dsp は 1 本ずつ)、`o1` 383 s(最大 RSS 3.6 GB)、`link` 8 s(キャッシュ生成を含む)。emcc は JOBS=2。**最初のビルドはサーバー再起動で `o1` の途中で中断した**ので、`o1` の作りかけ(`o1/src`、`o1/obj`)を消して `o1` と `link` だけやり直した(`units` までの `.o` は完成していた) |

### 1.1 ビルド構成(A〜D)

| 名前 | 構成 | wasm のバイト数 | SHA-256(先頭) |
|---|---|---:|---|
| A | JSPI なし、SJLJ=emscripten(HEAD の既定。無改造スクリプト) | 389,440,863 | `2580930f` |
| B | `-sJSPI`、SJLJ=emscripten、`jspi_hooks.c` あり | 389,444,985 | `d6f6ff40` |
| C | `-sJSPI`、**SJLJ=wasm**(legacy 符号化。emcc 6.0.12 の既定)、`jspi_hooks.c` あり | 389,438,311 | `52ec0419` |
| D | `-sJSPI`、SJLJ=wasm の **exnref 符号化**(`-sWASM_LEGACY_EXCEPTIONS=0`)、hooks あり | 389,438,402 | `fa3fcaa6` |
| C0 | C の `.o` を JSPI なし・hooks なしで再リンク | 389,434,341 | `881cc2e0` |
| C2 | C の `.o` を `-sJSPI`・hooks なしで再リンク | 389,434,341 | `881cc2e0`(**C0 と同一**) |
| C1 | C の `.o` を `-sJSPI` + hooks で再リンク | 389,438,340 | `9c0b5e60` |

C と D は `SJLJ`/`JSPI` の環境変数を足して全単位をコンパイルし直した。B は A の `.o`(同じ SJLJ=emscripten なので同じ翻訳)のハードリンクに hooks を足して再リンクしただけ。C と D は各 218 s(`units`)+ 約 400 s(`o1`)+ 3 s(`link`)、最大 RSS は A と同じ 8.83 GB / 3.64 GB。`.o` から再リンクするだけの C0/C1/C2 は 2.4〜2.6 s。

## 2. JSPI の適用可否の調査(先に。実測 + 読み)

依頼の (a)(b)(c) への答え。

**(a) `-sJSPI`(emcc 6.0.12 での指定)**: `src/settings.js` の `JSPI = 0`(リンク時の設定)。`-sJSPI` を付けると、グルーが `WebAssembly.promising` で `main` を包み(`JSPI_EXPORTS` の既定は `main`)、名前が `^(invoke_.*|__asyncjs__.*)$` に合う import を `WebAssembly.Suspending` で包む(実測: 生成された `flengine6.js` の `Asyncify.instrumentWasmImports` に書かれている)。`ASYNCIFY`(Binaryen のパス)は使わない。node 24(V8 13.6)は `--experimental-wasm-jspi` が要る(フラグ無しでは `typeof WebAssembly.Suspending` が `undefined`、フラグありで `function`。実測)。Chrome 156 はフラグ無しで動く(今回の Chrome の実行は JSPI のフラグ無し。`browser-event-loop.md` の実測と同じ)。

**(b) setjmp/longjmp の方式**: 現在の wasm(HEAD の既定、`-DFLE_LANDING_FORCE`)は `SUPPORT_LONGJMP=emscripten`(`-enable-emscripten-sjlj`)で、`invoke_vii` などの JS 関数を経由して wasm の呼び出しを `try/catch` で包む(実測: A の import に `invoke_vii`、`invoke_vjij`、`invoke_jijji`、`invoke_v`、`invoke_vijj`、`invoke_vi`、`_emscripten_throw_longjmp`)。`invoke_*` は JS のフレームなので、**その下で JSPI の中断はできない**(F1/F2、§6)。`-sSUPPORT_LONGJMP=wasm`(`-wasm-enable-sjlj`。Wasm EH の命令。`-fwasm-exceptions` を足さなくてよい: 実測、足さずにビルドできた)に変えると、`invoke_*` の import が無くなり(hooks なしの C0/C2 で import 17 個、タグ 1 個)、JSPI が通る。最小の例(`toy/t1.c`、`t3.c`、`t7.c`。すべて `-O0`):

| 構成 | 結果(実測) |
|---|---|
| `-sJSPI`(setjmp は emscripten 方式) + `setjmp` の内側で `await` する JS を呼ぶ | `SuspendError: trying to suspend JS frames` |
| `-sJSPI -sSUPPORT_LONGJMP=wasm` + 同じ(`longjmp` で着地まで) | 成功 |
| `-sJSPI`(emscripten 方式)、**中断を 1 回も要求しない**、`setjmp` を使う関数が入れ子(`t7.c`: `via1` の `setjmp` の内側で `via2` が別の `setjmp`) | `inner1`、`inner2` と出力した後 **`SuspendError`** |
| `-sSUPPORT_LONGJMP=wasm -sJSPI`、同じ入れ子 | 成功(`done` まで) |
| JSPI なし、emscripten 方式、同じ入れ子 | 成功 |

3 行目が重要: **emscripten 方式の SJLJ + JSPI は、中断を 1 回も要求しなくても壊れうる**。実エンジンの B 構成(SJLJ=emscripten + JSPI + hooks)でも、中断を要求しない記録だけの実行(`mode=0`)が `SuspendError` で止まった(F1。a4 の 27,862 番目の入口、生きている着地 1、`fle_call_guest` 深さ 1)。**原因は読み**: グルーが `invoke_*` をすべて `Suspending` で包む(上のパターン)ので、`invoke_*` の中(JS フレーム)からさらに入れ子の `invoke_*` を呼ぶと「JS フレームをまたいで中断しようとした」ことになる、と読める。直接の確認はしていない(`Suspending` の中の呼び出しが常に中断する仕様なのかは、このセッションでは確かめていない)。

**決定 0100/0115 の landing 方式を変えるか**: 変えていない。`FLE_LANDING_FORCE` のまま、`fle_dispatch.h` などの C コードは**一切変更していない**。変えたのは emcc のフラグ(`-sSUPPORT_LONGJMP=wasm`)だけで、全翻訳単位(`-c`)とリンクに付ける必要がある(`build_nmod_wasm.sh` の `SJLJ=wasm`)。範囲は lifted C 60 単位 + runtime 46 + harness。**決定 0115 項 1 の「追加のフラグは不要」は JSPI 構成では成り立たない**(JSPI なしでは引き続き成り立つ。A は無改造で動いた)。PM の判断(§10)。

**(c) `emscripten_return_address`**: `-finstrument-functions` のフックが `__builtin_return_address` を使うたびに、clang が JS の import `emscripten_return_address`(`new Error().stack` の解析。5.5 µs。`run_node_nmod.cjs` の説明)を呼ぶ。この import は同期の JS 関数で、`Suspending` で包まれない(パターンに合わない。実測: C の `flengine6.js`)。呼ぶ間は JS フレームができるが、すぐ戻るので中断の途中には残らない(実測: 全入口で中断した実行が一致)。**中断の邪魔にはならない**。ただし**性能に影響する**: JSPI 構成だと JS の import 1 回が約 +6〜7 ns になり(トイ `t4.c`、`t6.c`)、a4 の完走は 235,794,450 回呼ぶので完走が約 +1.1 s(§5.5)。この import を wasm 内の関数(`tools/ra0.c`、`return 0`)にすると JSPI の有無で差が出ない(R 表)。ただし高速ドライバーの `() => 0` と同じく、フックの `call_site` を捨てる(`trace.c` は使わない。決定 0115 項 3)。

## 3. 中断点の選定(トレースの読み + 実測)

### 3.1 実機でブロックしうる API と、a4 の完走での呼び出し(`a4-six-module-trace` の 13,572 件、171 種。`api-overview.tsv` と `trace.tsv.gz` の集計。実測)

| API | 呼び出し回数 | 呼び出し元のモジュール | InitCount | 備考 |
|---|---:|---|---|---|
| `EnterCriticalSection` | **681** | ILTools 575、FLEngine 86、Blend2D 8、dsp 6、freetype 4、QFC 2 | 13〜896(13 種) | 実機では競合で待ちうる。runtime は再帰カウントを増やすだけで常に即座に返す(`fle_sys.c`) |
| `WaitForSingleObject` | **8** | ILTools 8(すべて InitCount 246) | 246 | 毎回、同じイベント(`0x7ffa80000054`)を `INFINITE` で待ち、信号済みなので即座に 0。各回の後に別のイベント(`…58`)を `SetEvent` する |
| `AcquireSRWLockExclusive` | **1** | Blend2D 1 | 150 | 競合なし |
| `LoadLibraryW` | 34 | FLEngine 15、ILTools 12、QFC 7 | 104〜881 | ブロッキングではないが、DllMain を呼ぶ**ゲストの再入**(`fle_call_guest` 深さ 1)がある。「着地の内側で、再入の中」の例として選んだ |
| `ReadFile` / `CreateFileW` | 各 3 | ハーネス 2(エントリ前)、QFC 1 | 246(エントリ前 2) | I/O。エントリ前の 2 回は `setjmp` 着地の**外側**(§3.2)。「外側」の例として選んだ |
| `SetEvent`、`CreateEventW` | 8、36 | | | 待たないので中断点にしていない |
| `Sleep`、`SleepEx`、`GetMessageW`、`PeekMessageW`、`WaitMessage`、`MsgWaitForMultipleObjects(Ex)`、`WaitForMultipleObjects`、`WaitForSingleObjectEx`、`SwitchToThread`、`CoWaitForMultipleHandles`、`TryEnterCriticalSection`、`DispatchMessageW`、`SendMessageW`、`PostMessageW` | **0**(この 15 候補のどれも、171 種のトレースに現れない) | | | **メッセージループの API は a4 の完走では呼ばれない**(`CreateFruityInstance` の先の話。`browser-event-loop.md` §2) |

依頼の「呼ばれない場合は、呼ばれる API から代表を選ぶ」に従い、**多数回: `EnterCriticalSection`(681)、少数回: `WaitForSingleObject`(8)と `AcquireSRWLockExclusive`(1)、setjmp 着地の内側: 全部**(§3.2)を選び、加えて **`LoadLibraryW`(34、再入)**と **`ReadFile`/`CreateFileW`(3、うち 2 回は着地の外側)**を中断点にした。さらに「どの関数入口でも中断できるか」を見るため、**lifted 関数の入口 1,007,942 回すべて**(`-finstrument-functions` のフック `__cyg_profile_func_enter` を `--wrap` で包んだ)を中断点にした。

中断の中身は依頼の指定どおり(値は変えない): `await Promise.resolve()`(微小タスク)、`await new Promise(r => setTimeout(r, 0))`、`MessageChannel` の 1 往復。対照: 同期の JS import を 1 回呼ぶだけ(中断しない)。

### 3.2 各 API の呼び出しの位置(実測。記録だけの実行 `mode=0` + `new Error().stack` の解析。`rawprobe-api*`、`depth-all-m0`)

| API | 回数 | 呼び出し時の wasm フレーム数(フックの 3 段を含む)min / p50 / max | 生きている setjmp 着地 | `fle_call_guest` の深さ | 呼び出し元(ゲストの戻り番地)のモジュール |
|---|---:|---|---|---|---|
| `EnterCriticalSection` | 681 | 15 / 43 / 49 | 1 または 2(160 / 521 回) | 0〜2(146 / 531 / 4 回) | ILTools 575、FLEngine 86、Blend2D 8、dsp 6、freetype 4、QFC 2 |
| `WaitForSingleObject` | 8 | 42 / 42 / 42 | 2 | 1 | ILTools 8 |
| `AcquireSRWLockExclusive` | 1 | 32 / 32 / 32 | 1 | 1 | Blend2D 1 |
| `LoadLibraryW` | 34 | 11 / 30 / 36 | 1 または 2(15 / 19 回) | 0 または 1(15 / 19 回) | FLEngine 15、QFC 7、ILTools 12 |
| `ReadFile` | 3 | 3 / 3 / 52 | **0(2 回、エントリ前のハーネス発)**または 2 | 0 または 1 | ハーネス(戻り番地 `0xdead5eed`)2、QFC 1 |
| `CreateFileW` | 3 | 3 / 3 / 48 | 0(2 回)または 2 | 0 または 1 | ハーネス 2、QFC 1 |
| lifted 関数の入口(1,007,942 回。`depth-all-m0`) | | 4 / 45 / 56(p95)/ 68 | **0(3 回)、1(207,541 回)、2(800,398 回)** | 0(206,047)、1(801,439)、2(456) | |

- ブロックしうる 3 API(`EnterCriticalSection`、`WaitForSingleObject`、`AcquireSRWLockExclusive`)は**すべて着地の内側(生きている着地 ≥ 1)**で呼ばれる。「生きている着地」は `fle_landing_top` の連鎖の長さ(C の `setjmp` フレームが生きている数)。着地の外側は、エントリ前のハーネスの `ReadFile`/`CreateFileW`(各 2 回)と、関数入口の最初の 3 回だけ。
- 「スタックの深さ」は V8 のエラースタックのフレーム数(`Error.stackTraceLimit` を大きくして取った)。全体の最大は 73、wasm だけの最大は 68(JS フレームは ≤ 5: `callMain`、`run`、`Module`、プローブ用の JS)。
- ゲストのスタック(`R[4]`)の使用は、全入口で最低 `0x11f75008`、最高 `0x11f7bfb8`(使用最大 28,592 B)。

## 4. 実装(最小差分)

差分は `jspi.diff`(3 ファイル、347 行。HEAD に `patch -p1`。159a42e と c7d65bf の `lift/harness` に `--dry-run` で当たる。当てた木の 3 ファイルは測定に使ったファイルと `cmp` 一致)。**追跡ファイルの編集は `build_nmod_wasm.sh` の 1 つだけ**で、残り 2 つは新規。runtime、生成器(fe-lift)、生成 C、`trace.c`、`fle_dispatch.h`、決定文書は変えていない。

| ファイル | 内容 |
|---|---|
| `lift/harness/two/build_nmod_wasm.sh`(編集) | 環境変数を 3 つ足した: `SJLJ=emscripten`(既定。**変数なしの出力は HEAD と同じ**)、`SJLJ=wasm`(全 `emcc -c` とリンクに `-sSUPPORT_LONGJMP=wasm`)、`SJLJ=wasm-exnref`(さらに `-sWASM_LEGACY_EXCEPTIONS=0`)、`JSPI=1`(`jspi_hooks.o` をコンパイルし、リンクに `-sJSPI` と `-Wl,--wrap=…` を足す)、`LINK_ENV`(リンクの環境フラグの上書き。既定は HEAD と同じ node + NODERAWFS)。`JSPI` と `SJLJ` を指定しなければ、HEAD のスクリプトと同じコマンドを出す(A はこれで作り、ビルド出力も HEAD の記録と同種)。 |
| `lift/harness/wasm/jspi_hooks.c`(新規) | 測定用のフック。runtime の `fle_imp_EnterCriticalSection`、`fle_imp_WaitForSingleObject`、`fle_imp_AcquireSRWLockExclusive`、`fle_imp_LoadLibraryW`、`fle_imp_ReadFile`、`fle_imp_CreateFileW` と、`trace.c` の `__cyg_profile_func_enter` を、リンカーの `--wrap` で包む。設定(`globalThis.JSPI_CFG`。モード、種類ごとの「N 回に 1 回」「何番目から何番目まで」)に応じて、記録だけ(`mode=0`)/ `Promise.resolve()` を待つ(1)/ `setTimeout(0)` を待つ(2)/ `MessageChannel` の往復を待つ(3)/ 同期の JS import を 1 回呼ぶだけの対照(4)を行い、その後に**本物の関数を呼ぶ**。レジスタ、ゲストのメモリ、引数、標準出力には触れない。中断は `EM_ASYNC_JS`(JSPI では `__asyncjs__jspi_suspend` という名前の import になり、グルーが `Suspending` で包む)。記録: 種類、通し番号、ゲストの戻り番地、`R[4]`、生きている着地の数、`fle_call_guest` の深さ、中断の所要時間(`emscripten_get_now`)、JS 内の所要時間、`probe=1` のときは `new Error().stack` のフレーム数。 |
| `lift/harness/two/run_node_jspi.cjs`(新規) | `run_node_nmod.cjs` と同じ引数・環境変数・終了コード(`FLE_WASM_DIR`、`FLE_WASM_BIN`、`FLE_WASM_FAST_RETADDR=1`)。違い: wasm は常にこのファイルが `WebAssembly.compile` → `instantiate` し、時間を測る。`FLE_JSPI_CFG`(JSON)を `globalThis.JSPI_CFG` にする。`FLE_JSPI_OUT`/`FLE_JSPI_RAW` に集計と生の記録を書く。`FLE_JSPI_COUNT_RA=1` で `emscripten_return_address` の呼び出しを数え、`FLE_JSPI_TICK=<ms>` で中断中に別の JS タスクが動くか数える。JSPI 構成は `node --experimental-wasm-jspi` で動かす(JSPI なしの wasm も動かせる)。 |

ブラウザ用(`lift/harness/web/jspi/`。既存の `lift/harness/web/*` は**変更なし**で、`cdp.mjs`、`serve.mjs` は `git show HEAD:` と `cmp` 一致): `fle-worker-jspi.js`(`../fle-worker.js` のコピー + `JSPI_CFG` の設定 + 集計。`emscripten_return_address` は常に `() => 0`)、`index-jspi.html`、`run-jspi.mjs`(`measure.mjs` を縮めたもの。`--password-store=basic` と `--js-flags=` を足して Chrome を起動)。`tools/`: 実測に使ったシェルスクリプト(環境変数の `env.sh`、ビルドの `wb.sh`/`stages.sh`、再リンクの `relink.sh`/`relink-x.sh`、実行の `susprun.sh`/`runnode.sh`、集計の `sumjson.py`/`allruns.py`)、`ra0.c`(`emscripten_return_address` を wasm 内の関数にする 3 行)。`toy/`: 最小の例の C(`t1.c`、`t3.c`〜`t7.c`、`t9.c`)。

**手順の要約**(全部 `lift/harness/web/jspi/tools/` にある。パスは `lift/out-jspi` を前提):
1. `git archive HEAD lift docs` を `lift/out-jspi/src/` に展開、fe-lift/fe-pe をビルド。
2. `tools/run6n.sh <OUT>`: native の基準(`a4-multi.sh` 無改造)。出来た `dll/`、`realmod_table_gen.c`、`modules.txt` を `res/` に退避。
3. `tools/gen_w.sh rebase gen split merge`: wasm 用の生成 C。
4. `tools/stages.sh <logdir> force rt harness units o1 link`: A は無変更のスクリプトで、B/C/D は `OBJ=… JSPI=1 SJLJ=…` を付けて(`wb.sh` が `build_nmod_wasm.sh` を呼ぶ)。dsp の単位は 1 本ずつ、emcc は JOBS=2。
5. `tools/susprun.sh <bin> <名前> '<JSON>' [node のフラグ]` で 1 回ずつ実行して `runs/` に残す。`tools/allruns.py` が全実行の a4 と stdout を native と比べて `logs/all-runs.tsv` を作る。
6. Chrome: `relink.sh` で `-sENVIRONMENT=web,worker …` に再リンクし、`dist/` に wasm と入力ファイルを置いて `node run-jspi.mjs name=… cfg='{…}'`。

## 5. 測定(node。すべて実測)

### 5.1 (a) 中断を入れない実行

`T/*`(各 3 回。同じ `susprun.sh`、`FLE_WASM_FAST_RETADDR=1`、`cfg={}`。a4 は全部 942/942、895/895、5,306、1,007,942、終了コード 0、stderr 0 B)。時間は ms:

| 構成 | `WebAssembly.compile` | `instantiate` | 実行(`main` の開始から終了) | wall(`/usr/bin/time`) | 最大 RSS |
|---|---|---|---|---|---|
| A(JSPI なし、SJLJ=emscripten、基準) | 342 / 349 / 371 | 12 / 11 / 11 | 2,435 / 2,422 / 2,555 | 3.09 / 3.06 / 3.16 s | 1,220 / 1,205 / 1,119 MB |
| C0(SJLJ=wasm、JSPI なし、hooks なし) | 331 / 335 / 335 | 13 / 11 / 12 | 2,447 / 2,479 / 2,416 | 3.07 / 3.11 / 3.04 s | 1,218 / 1,204 / 1,204 MB |
| C2(SJLJ=wasm、`-sJSPI`、hooks なし) | 334 / 334 / 333 | 13 / 11 / 11 | **3,578 / 3,569 / 3,591** | 4.20 / 4.19 / 4.17 s | 1,212 / 1,218 / 1,119 MB |
| C1(C2 + hooks、設定なし) | 328 / 335 / 338 | 11 / 12 / 12 | 3,654 / 3,637 / 3,521 | 4.26 / 4.21 / 4.14 s | 1,216 / 1,119 / 1,217 MB |
| C(C1 と同じ構成のビルド出力そのもの) | 345 / 332 / 351 | 12 / 12 / 13 | 3,632 / 3,756 / 3,589 | 4.27 / 4.34 / 4.24 s | 1,219 / 1,119 / 1,215 MB |

- C0 と C2 の wasm は**バイト同一**(SHA-256 `881cc2e0726ec9f5…`。同じ `.o` を `-sJSPI` の有無だけ変えて再リンク)。JSPI は wasm のコードを変えない(`wasm-opt` を通していない: `EMCC_DEBUG=1` の再リンクの記録に `wasm-ld` の呼び出しだけがあり、`wasm-opt` の呼び出しは無かった)。変わるのは js グルー(183,273 → 186,730 B)だけ。
- **SJLJ を wasm 方式にしても実行時間は変わらない**(A 対 C0: 2,422〜2,555 対 2,416〜2,479 ms)。サイズは −6,522 B(`invoke_*` の import が消えるぶん)。
- **JSPI のリンクで実行が約 +1.1 s(+46 %)**。§5.5 で原因を調べた。
- 未改造ドライバー(`FLE_WASM_FAST_RETADDR` 無し)の JSPI なしの基準は**走らせていない**(約 30 分。`verification.md` の 29:34 と 30:02、`wasm6-report.md` の 32:54 を引用するだけ)。JSPI あり(C)で API を 730 回中断した完走は 1 回: **32:53.87**、user 1,973.13 s、最大 RSS 1,377,160 KB、a4 一致(`Stock/full-apiall-m3`)。引用値と同じ程度で、JSPI の有無の差は区別できない。

### 5.2 (b) 中断を入れた実行(`F/*`、`C/*`、`Stock/*`。ビルド C。すべて a4 一致・stdout は native と `rsp=` の行以外同一・終了コード 0・stderr 0 B)

| 中断点 | 「1 回目だけ」(微小タスク) | 全呼び出しで 微小タスク | 全呼び出しで `setTimeout(0)` | 全呼び出しで `MessageChannel` 往復 |
|---|---|---|---|---|
| `EnterCriticalSection`(681 回) | 一致 | 一致 | 一致 | 一致 |
| `WaitForSingleObject`(8 回) | 一致 | 一致 | 一致 | 一致 |
| `AcquireSRWLockExclusive`(1 回) | 一致 | 一致 | 一致 | 一致 |
| `LoadLibraryW`(34 回) | 一致 | 一致 | 一致 | 一致 |
| `ReadFile`(3 回)/ `CreateFileW`(3 回) | 一致 | 一致 | 一致 | 一致 |
| 6 種すべて同時(730 回) | | 一致 | | 一致(高速ドライバーの C、`emscripten_return_address` を wasm 内にした構成 R1(C + hooks)、**未改造ドライバー(32:53.87)**の C) |
| lifted 関数の入口(1,007,942 回) | | **全部で中断して一致**(3 回) | 100 回に 1 回(10,080 回、一致) | 10 回に 1 回(100,795 回、一致) |
| 対照: 同期の JS import だけ | | 全入口(1,007,942 回)一致 | | |

- ビルド `D`(exnref 符号化の SJLJ=wasm)でも、中断なし 2 回、API 6 種 730 回(微小タスク)、全入口 1,007,942 回(微小タスク)、全入口の同期 import、100 入口に 1 回の `setTimeout(0)`(10,080 回): すべて一致(`D/*`)。`R/ra0-C2-*` は hooks を含まない C2 の実行で、中断は入っていない(`R/ra0-C2-apiall-m3` は設定を渡しても記録 0 件)。
- **中断中に別の JS タスクが動いたか**(`FLE_JSPI_TICK=1`: 1 ms の `setInterval`): 中断なし 191 回、全入口 1,000 回に 1 回の `setTimeout(0)`(1,008 回の中断)で 1,208 回、API 730 回の `setTimeout(0)` で 929 回、全入口 1,000 回に 1 回の微小タスクで 206 回(微小タスクはタイマーに順番を回さない)。**`setTimeout(0)` で中断している間に、イベントループが別のタスクを動かした**(実測)。
- 全実行の一覧(198 件。うち意図した失敗・失敗の調査が 30 件、残り 168 件は a4 と stdout が基準と一致、終了コード 0): `logs/all-runs.tsv`。

### 5.3 (c) 1 回の中断・再開のコスト(µs。`total` は C の `emscripten_get_now` で `jspi_suspend` の呼び出し前後を測った値。中断 + 再開 + 待った JS の中身を含む。ビルド C、高速ドライバー、node)

| 中断の中身 | 中断点 | 回数 | p50 | p95 | p99 | max |
|---|---|---:|---:|---:|---:|---:|
| 同期 import の対照(中断しない) | 全入口 | 1,007,942 | 0.05 | 0.06 | 0.13 | 924 |
| 微小タスク | 全入口 | 1,007,942 | **0.22** | 0.44 | 1.28 | 1,666 |
| 微小タスク | `EnterCriticalSection` | 681 | 1.8 | 23.2 | 79.6 | 371 |
| 微小タスク | `WaitForSingleObject` | 8 | 12.4 | 101 | | 101 |
| 微小タスク | `LoadLibraryW` | 34 | 18.2 | 104 | 130 | 130 |
| 微小タスク | `ReadFile` / `CreateFileW` | 3 / 3 | 18.9 / 20.3 | 86 / 124 | | 86 / 124 |
| 微小タスク | `AcquireSRWLockExclusive` | 1 | 92.6 | | | |
| `MessageChannel` 往復 | 10 入口に 1 回 | 100,795 | 1.05 | 6.86 | 10.8 | 28,916 |
| `MessageChannel` 往復 | `EnterCriticalSection` | 681 | 3.7 | 81.7 | 136 | 32,276 |
| `MessageChannel` 往復 | `LoadLibraryW` | 34 | 78.6 | 187 | 29,312 | 29,312 |
| `setTimeout(0)` | 100 入口に 1 回 | 10,080 | **1,103** | 1,134 | 1,180 | 1,736 |
| `setTimeout(0)` | `EnterCriticalSection` | 681 | 1,120 | 1,217 | 1,305 | 1,799 |

- 微小タスクの p50 は、**呼び出しの回数が多く温まった経路で 0.22 µs、681 回の経路で 1.8 µs、数回の経路で 12〜93 µs**と大きく違う。**同じ中断の仕組みのコストが回数で違って見える**(原因は未確認。V8 の JIT の段階、キャッシュ、`jspi_rec` の記録の初回確保などが考えられるが、分けて測っていない)。**プロセスで最初の 1 回は 86〜145 µs**(6 つの API 別の「1 回目だけ」の実行、`F/api*-first-m1`)。
- 微小タスクの値は「中断 + 再開」の最小値に近いと読めるが、**実際のメッセージループでは別のタスク(入力イベント、描画)が間に入る**ので、この値はそのコストではない。`setTimeout(0)` は node では 1 ms の下限(`browser-event-loop.md` §3.2 が書く Worker の 4 ms 前後とは別。Chrome は §7)。
- `MessageChannel` 往復の max の約 29〜36 ms は、**その実行で最初の中断**に付く(10 入口に 1 回の実行の生の記録で、5 ms を超えた記録は通し番号 0 の 1 件だけ、34.7 ms: `X/mc-e10-raw`。他の実行は「最初の中断が最大値」と読むが、生の記録は取っていない)。`MessageChannel` の生成と最初の配信のコストと読む。
- 未改造ドライバー(`new Error().stack` の解析が 2.36 億回)での、同じ 730 回の中断: p50 74〜91 µs(`Stock/full-apiall-m3`。`MessageChannel` 往復、中身は同じ)。高速ドライバーの p50 3.7〜78 µs より大きい。原因は未確認(大量の `Error` オブジェクトの生成と GC が間に入る影響と読むが、確かめていない)。
- **全体の実行時間の増加**(`main` の開始から終了の ms。3 回ずつ。中断 1 回あたりではなく実行全体):

| 構成 | 中断なし(hooks あり、設定なし) | 記録だけ(`mode=0`、全入口) | 同期 import(全入口) | **微小タスクで全入口(1,007,942 回)を中断** |
|---|---|---|---|---|
| C(JS の `emscripten_return_address`) | 3,615 / 3,767 / 3,615 | 3,693 / 3,739 / 3,711 | 3,828 / 3,914 / 3,925 | **4,091 / 4,312 / 5,401** |
| R1(`emscripten_return_address` を wasm 内に) | 2,396 / 2,152 / 2,119 | 2,283 / 2,400 / 2,095 | 2,300 / 2,619 / 2,246 | **2,671 / 3,003 / 2,755** |

  - 全入口を微小タスクで中断すると、C で中断なしより **約 +0.5〜0.7 s**(最小値同士 4,091 対 3,615)、R1 で **約 +0.5〜0.6 s**(最小値同士 2,671 対 2,119。揺れが大きい)。1,007,942 回で割ると 0.5〜0.6 µs / 回。**フックの記録(`mode=0`)と同期 import を差し引いた、中断・再開そのものの増分**は、C で約 +0.3〜0.4 s(4,091 対 3,828)、R1 で約 +0.3〜0.4 s(2,671 対 2,300)で、0.3〜0.4 µs / 回。これは p50 の 0.22 µs(再開の最短値)と p95 の 0.44〜0.5 µs の間。
  - API 730 回の中断(微小タスク、`MessageChannel`)の実行時間は 3,687 / 3,684 ms で、中断なし(3,615〜3,767)の揺れの内。
  - `setTimeout(0)` を 100 入口に 1 回(10,080 回): 実行 14,966 ms(中断なし約 3,650 ms に対し約 +11.3 s。10,080 × 1.1 ms = 11.1 s とほぼ一致)。`MessageChannel` 往復を 10 入口に 1 回(100,795 回): 3,884 ms(+約 0.2 s)。

### 5.4 (d) 最大 RSS

node の `/usr/bin/time -v` の最大 RSS: A 1,119〜1,249 MB、C 系の中断なし 1,119〜1,218 MB、API 中断 1,146〜1,262 MB、**全入口の記録(1,007,942 件 × 96 B の `Float64Array`)付きの実行は 1,463〜1,583 MB**(記録の配列のぶん。中断の仕組みの増分ではない)。JSPI の中断で RSS が増えた形跡は無い。JSPI の中断で追加で使うスタックは、全入口の中断で 1 本(中断中のスタックは 1 つだけ)。

### 5.5 JSPI 構成で実行が +46 % になる原因(実測 + 読み)

| 実験 | 結果(実測) |
|---|---|
| `emscripten_return_address` の呼び出しの数(`FLE_JSPI_COUNT_RA=1`、C2) | **235,794,450 回**(`wasm6-report.md` の 235,794,590 回と 140 回違う。別のビルド) |
| C0 と C2 の実行時間の差 | 約 1,130 ms ÷ 235.8 M ≒ **4.8 ns / 回** |
| トイ `t4.c`/`t6.c`(`EM_JS` の同期 import を 2000 万〜1 億回呼ぶ。`-O1`) | JSPI なし 4.0〜4.7 ns/回、JSPI あり 10.5〜11.5 ns/回(**+約 6〜7 ns**)。深さ 0 と 60 で同じ |
| `--stress-wasm-stack-switching`(wasm を常に別スタックで動かす V8 のフラグ)で C0(JSPI なしの wasm)を実行 | 実行 3,837 / 3,824 ms(通常 2,416〜2,479 ms)。**JSPI なしの wasm でも、別スタックで動かすと +約 55 %** |
| **`emscripten_return_address` を wasm 内の関数(`ra0.c`、`return 0`)にして**再リンク(R。C0 と C2 を交互に各 3 回) | C0: 1,651 / 1,736 / 1,794 ms、C2: 1,774 / 1,660 / 1,600 ms(平均 1,727 対 1,678)。**差は無い**(測定の揺れの内) |
| 同じ構成 + hooks(R1: 中断なし / 同期 import の全入口 / 微小タスクの全入口 を各 3 回) | 中断なし 2,396 / 2,152 / 2,119 ms、記録だけ 2,283 / 2,400 / 2,095、同期 import 2,300 / 2,619 / 2,246、**微小タスク 2,671 / 3,003 / 2,755** |

読み: JSPI でリンクした wasm から JS の import を呼ぶと、1 回ごとに別スタックから呼び出し元のスタックへ切り替わるぶんが乗る(約 5〜7 ns)。a4 の完走では、`-finstrument-functions` のフックが JS の import を **2.36 億回**呼ぶので、これが +1.1 s の主因。**JSPI の中断そのものではない**(実測: 中断を入れた実行の増分は別に測れている)。ただしこの 2.36 億回は、(ア)測定用の `-finstrument-functions` に由来し、本番のビルドには無い、(イ)ブラウザ化した実エンジンでは、Win32 API の import が JS に出るたびに同じ切り替えが乗る(Win32 の import が毎秒何回になるかは**未測定**)。

## 6. 失敗の詳細と原因の仮説

### F1: emscripten 既定の SJLJ(B)+ JSPI は、中断を 1 回も要求しなくても `SuspendError` で止まる

- 構成 B(`-sJSPI`、SJLJ=emscripten、hooks あり)、`cfg={}`(記録も中断もしない)と、記録だけ(`mode=0`)の 2 通りとも: `SuspendError: trying to suspend JS frames`、`wasm trap/exception` の終了コード 99、stdout は 16 行(`calling entry` まで)で止まる。`mode=0` で 27,862 番目の関数入口の記録の後。そのときの生きている着地は 1、`fle_call_guest` の深さは 1。スタックの最初の 12 フレームは全部 wasm(JS フレームは見えない)。
- 原因の読み(§2): グルーが `invoke_*` を `Suspending` で包む。`invoke_*` は JS 関数で、中でさらに wasm を呼び、その wasm が(入れ子の)`invoke_*` を呼ぶと、同期の JS 関数でも JSPI は「JS フレームをまたいで中断する」とみなして失敗する。**直接の確認はしていない**。トイ `t7.c`(中断も `await` も無い、`setjmp` を使う関数の入れ子)が同じ `SuspendError` になり、SJLJ=wasm では通ることが傍証。

### F2: `invoke_*` を `Suspending` で包まない場合(Bp)、`setjmp` を含む関数の下では中断できない

グルーの正規表現から `invoke_.*` を外した版(`Bp`: B と同じ wasm、グルーだけ違う)で、中断(微小タスク)を 1 回ずつ要求した:

| 中断点 | 結果 | 備考 |
|---|---|---|
| `CreateFileW` の 1、2 回目 / `ReadFile` の 1、2 回目(エントリ前のハーネス発。生きている着地 0) | **成功**(a4 一致) | wasm の下に JS フレームが無い |
| `CreateFileW` の 3 回目 / `ReadFile` の 3 回目(QFC の中。着地 2) | **失敗**(`SuspendError`、終了コード 99、stdout 16 行) | |
| `EnterCriticalSection` の 1 回目 / `WaitForSingleObject` の 1 回目 / `AcquireSRWLockExclusive` の 1 回目 | **失敗** | 既定の 10 行のスタックには `invoke_*` が映らない。`EnterCriticalSection` の 1 回目は `--stack-trace-limit=300` の別の実行(`X/Bp-api0-first-full`、25 フレーム)で `invoke_v` が 1 つ見える |
| 関数入口の 1〜5 番目(1〜3 番目は生きている着地 0、4、5 番目は 1。`X/depth-all-m0` の記録) | **失敗** | エラースタック(既定の 10 行)に `invoke_vii`(1 番目)、`invoke_v`(2〜5 番目)の JS フレームが入っている |

読み: トイ(`toy/t9.c`: `setjmp` を含む関数 `via` が、`setjmp` の**前**に `other()`、後に `leaf()` を呼ぶ)を `-g` 付きで `-sJSPI`(emscripten SJLJ)でビルドして `wasm-objdump -d` で見ると、`via` の本体に `call <invoke_v>` が 2 回と `call <invoke_iii>` が 1 回あり、`via` から `other` や `leaf` への直接の `call` は 0 個だった(実測)。`invoke_v` に渡された引数が `other` か `leaf` かは、逆アセンブルから読み取っていない(**未確認**)。それでも、`setjmp` を含む関数から出る呼び出しが、直接の `call` ではなく `invoke_*` になる、とは言える。実エンジンの wasm は関数名の節が無いので同じ確認はしていないが、FLEngine、QFC、ILTools の各 1 関数(生成 C の `f_1376820` など。`fle_landing` の形)が `setjmp` を含むので、その関数が呼ぶもの(`f_412760()` も `f_1376840()` も)は JS フレームの下になる、と**読む**(エラースタックの `invoke_*` の位置と合う)。`f_1376840()` は残りの実行の全体(InitUnits を含む)を呼ぶので、**ほぼすべての中断点が JS フレームの下**になる。生きている着地が 0 の入口 1〜3 番目も、`f_1376820` が呼んだ `f_412760()` の中なので失敗する。**成功したのは `main` の中の、`fn()` を呼ぶ前のハーネスのコード(ファイルの種付け)だけ**。結論: emscripten 方式の SJLJ では、ゲストのエントリの呼び出しの下で中断する用途に使えない。

### F3: Chrome の `Page.navigate` が完了しない(JSPI とは無関係。環境)

`measure.mjs`/自作の `run-jspi.mjs` で、Chrome を起動して `Page.navigate` すると、ローカルのサーバーが 1 件もリクエストを受けないまま応答が返らなかった(1 回目は 10 分でタイムアウト、以降は 15〜60 秒で自作のタイムアウト)。`about:blank`、`data:` URL、CDP の接続は動く。ローカルの TCP 接続自体は張られた(netlog で確認)。`--no-proxy-server`、`--proxy-bypass-list=*`、`/tmp/opencode/` 内のプロファイルでは直らず、**`--password-store=basic` を足すと直った**(`Page.navigate` が 11 ms)。原因は特定していない(仮説: デスクトップのキーリングへの問い合わせ。**未確認**)。`browser-event-loop.md` の Chrome の実行は(再起動の前に)動いていたので、再起動後のこの機械の状態に依存する可能性がある(読み。起動のコードは同一ではないので、同じ条件の比較ではない)。`run-jspi.mjs` は `--password-store=basic` を足して Chrome を起動する(`cdp.mjs` は無変更)。

### F4: JSPI のスタックが小さいと `RangeError`(意図した失敗)

§8。

### F5: wasm-objdump が exnref 符号化の wasm を読めない

D(`-sWASM_LEGACY_EXCEPTIONS=0`)の wasm を `wasm-objdump -h` に渡すと `error: expected valid local type`(wabt 1.0.36 が exnref 型を知らない、と読む)。V8(node 24 と Chrome 156)は実行できた。D の functions/Code の数などは wabt で数えられない。

### 失敗ではないが記録する: 作業の中断

最初の `o1` 段の途中でサーバーが再起動し、ビルドが途中で切れた。`o1/src` と `o1/obj` の作りかけを消して `o1` と `link` をやり直し、`units` までの `.o` はそのまま使った(再リンク後の A は無改造スクリプトで作った 389,440,863 B。再現性の確認(2 回作って SHA が一致するか)は**していない**)。

## 7. (h) headless Chrome(`lift/harness/web/jspi/`。1 構成)

Chrome 156.0.8078.47(V8 15.6.75.19)、`--headless=new --no-sandbox --disable-gpu --disable-dev-shm-usage --mute-audio --password-store=basic`、`--user-data-dir=lift/out-jspi/chrome-profile/<名前>`(実行ごとに作って終了後に削除)、`localhost` のサーバーのみ、ネットワーク取得なし。`--js-flags` を付けたのは JSPI のスタックを絞る 5 回だけ。Web Worker 内で `MODULARIZE` の web 用グルー + `FS`(MEMFS)に入力 9 ファイルを `fetch` して書く(`../fle-worker.js` と同じ流儀)。`emscripten_return_address` は `() => 0`(高速ドライバーと同じ)。

| 実行(26 回) | 構成 | 結果 |
|---|---|---|
| 中断なし ×3(`W/nosusp1〜3`) | C(JSPI + SJLJ=wasm + hooks) | a4 一致、終了コード 0。`main` の実行 3,755 / 3,782 / 4,099 ms、最大 renderer RSS 1,366〜1,421 MB、プロセス木 2,545〜2,637 MB |
| 中断なし ×3(`WC0/nosusp1〜3`) | **C0(JSPI なし、SJLJ=wasm)** | a4 一致。2,899 / 2,946 / 3,118 ms、renderer 1,246〜1,290 MB。**node と同じく、JSPI リンクで約 +0.8 s(+30 %)**(JS の import `emscripten_return_address` の切り替えが原因と**読む**。Chrome で §5.5 の実験は繰り返していない) |
| 中断なし ×1(`WD/nosusp1`) | D(exnref 符号化) | a4 一致、4,106 ms |
| API 6 種 730 回 × 微小タスク / `setTimeout(0)` / `MessageChannel`(`W/apiall-m1/m2/m3`) | C | すべて a4 一致。`setTimeout(0)` は 1 回 約 4.1〜4.3 ms(**Chrome の入れ子の `setTimeout` の 4 ms クランプ**。node の 1.1 ms と違う)、`main` が 6,766 ms |
| 上と同じ × 隔離あり(COOP same-origin + COEP require-corp。`W/coi-apiall-m1/m3`) | C | すべて a4 一致。時計の刻みが 5 µs になる |
| 全入口 1,007,942 回 × 微小タスク / 同期 import(`W/entry-all-m1`、`entry-all-m4`) | C | a4 一致。4,606 / 4,512 ms(中断なし 3,755〜4,099) |
| `MessageChannel` 往復を 10 入口に 1 回(100,795 回)、`setTimeout(0)` を 100 入口に 1 回(10,080 回)(`W/entry-e10-m3`、`entry-e100-m2`、隔離あり `W/coi-entry-*`) | C | すべて a4 一致 |
| 記録 + `new Error().stack`(`W/probe-entry-e101-m1`) | C | 一致。wasm フレーム数 min 4 / p50 45 / max 67(node の 4 / 45 / 68 とほぼ同じ) |
| API 730 回、全入口 1,007,942 回(`WD/apiall-m1`、`WD/entry-all-m1`) | D(exnref) | 一致 |
| JSPI のスタック: `--js-flags=--wasm-stack-switching-stack-size=` 984 / 200 / 150 kB(`W/stack984`、`stack200`、`stack150`) | C | **一致**(a4 942/942) |
| 同 140 / 100 kB(`W/stack140`、`stack100`) | C | **`RangeError: Maximum call stack size exceeded`**、`main` が終わらない(意図した失敗)。node と同じ下限(150 kB 可、140 kB 不可) |

- 26 回のうち、a4 と stdout が node の基準と一致しなかったのは**意図したスタックの絞りの 2 回だけ**(16 行で止まった)。他の 24 回は、`calling entry` の行(`rsp=` の値。node の基準 C は `0x11f7bfc0`、Chrome の C は `0x11f7afc0`)と、`vfs seed:` の行のうちホストのパスを含む 3 行(`list` の行と 2 ファイルの行。node は `/home/ubuntu/…`、Chrome は `/data/seed/…`)だけが違い、他の全行が同一。**比較の誤りの記録**: 最初に書いた比較スクリプトが 26 件すべてを「不一致」と出した。`diff` で行を見て、違いが上の 4 行だけであることを確かめてから、その 4 行を除いて比べ直した(24 件一致、2 件が 16 行で停止)。`vfs seed:` の 2 ファイルの行のバイト数と SHA-256(7,892 B `588c4bf4…`、102,712 B `578b46df…`)は、`diff` の出力を目で読んで node と Chrome で同じであることを確認した(スクリプトでの照合はしていない)。
- **Chrome での `MessageChannel` 往復のコスト(隔離あり、時計の刻み 5 µs。`W/coi-*`)**: 10 入口に 1 回(100,795 回)p50 5 µs / p95 10 µs / p99 25 µs / max 76,640 µs。API の 681 回(`EnterCriticalSection`)p50 5 / p95 95 / p99 175 / max 1,405 µs。`setTimeout(0)`(10,080 回)p50 4,175 / p95 4,250 / p99 4,335 µs。微小タスク(API 681 回)p50 0 / p95 20 / p99 40 µs(刻み以下)。**隔離なし(刻み 100 µs)の値は p50 が 0 または 100 になるので、コストとしては使えない**(`W/apiall-*` の値は a4 の一致の確認にだけ使う)。
- Chrome の `MessageChannel` 往復・`setTimeout(0)` の実行の max は 54〜79 ms(node は 29〜36 ms)。node と同様に最初の中断に付くものと**読む**(Chrome の生の記録は取っていない)。
- 実行中のプロセス木のピーク RSS は 2.5〜2.7 GB(renderer 1.3〜1.4 GB を含む)。`WebAssembly.instantiate` を含む wasm の準備(`wasm_start` → `wasm_instantiated`)は 1,172〜1,345 ms(`fetch` したバイト列のコンパイル。node の `WebAssembly.compile` 330〜390 ms より長い。JSPI の有無で差なし: C0 1,263〜1,308、C 1,198〜1,331)。

**Chrome の構成は 1 つだけ**(headless、1 マシン、Linux)。Firefox、Safari、headed の Chrome、Edge は**未確認**。

## 8. (e) スタックの深さと JSPI のスタック上限

| 実験 | 結果(実測) |
|---|---|
| V8 の JSPI スタックの既定 | `--wasm-stack-switching-stack-size` = **984 kB**(node 24 の `--v8-options`、Chrome 156 の `--js-flags=--help` で同じ) |
| a4 の経路の最大の深さ(全入口で記録) | wasm フレーム 68、全体 73(JS フレーム ≤ 5)。ゲストのスタック使用 28,592 B |
| JSPI のスタックを絞る(`--wasm-stack-switching-stack-size`、node、ビルド C、中断なし) | 984、700、500、300、200、**150 kB: 成功**。**140、130、120、110、100、64 kB: `RangeError: Maximum call stack size exceeded`**(終了コード 99、stdout 16 行) |
| 同、全入口で微小タスクの中断 | 150 kB 成功、**145 kB と 140 kB 失敗**(中断の仕組みの分で、中断なしより下限が 5 kB ほど上がった。150 kB と 145 kB の間) |
| exnref 符号化の D | 200、150 kB 成功、140 kB 失敗 |
| JSPI なしの基準(A)で、メインスタックを `--stack-size` で絞る | 700〜160 kB 成功、**155、152、150、100 kB 失敗**。**必要量は JSPI の別スタック(約 150 kB)とほぼ同じ**(メインスタックは約 160 kB) |
| Chrome 156(`--js-flags=--wasm-stack-switching-stack-size=`) | 984、200、150 kB 成功、140、100 kB 失敗 |
| 深い位置で中断できるか(トイ `t5.c`: 再帰 `deep(n)` の底で `await`、`-O0`、各段 64 B のローカル) | JSPI のスタック 984 kB 既定で n = 10,000 〜 12,000 まで成功、**14,000 以上は `RangeError`**。JSPI なしの基準(メインスタック)も 12,000 成功、14,000 失敗(**ほぼ同じ**。メインスタックの上限は node の既定)。`--wasm-stack-switching-stack-size=16000 --stack-size=60000` なら n = 100,000 成功、984 kB / 4,000 kB では失敗。`--experimental-wasm-growable-stacks` を付けても n = 100,000 は失敗(その場合の挙動は未調査) |

**読み**: a4 の経路では、JSPI のスタックの既定(984 kB)は、下限の約 150 kB の約 6.5 倍(下限は a4 の経路の最大の深さ、wasm フレーム 68 での値)。**実エンジンの本番のメッセージループ(`CreateFruityInstance` の先、`HandleMessage` → `DispatchMessageW` → `WndProc` → 入れ子のメッセージループ)の深さは測っていない**ので、984 kB で足りるかは**未確認**。

## 9. (g) wasm のサイズと V8 のコンパイル時間(`wasm-opt` を通していない)

- `emcc` のリンクは `wasm-ld` だけ(`EMCC_DEBUG=1` の記録に `wasm-opt` の呼び出し無し。`using binaryen` と `block "binaryen" took 0.000 seconds` の行は出る)。
- 同じ `.o` から `-sJSPI` の有無だけ変えると **wasm は 389,434,341 B で SHA-256 同一**(C0 と C2)。js グルーは 183,273 → 186,730 B(+3,457 B)。hooks を足すと +4 KB(389,438,340 B)。
- SJLJ を JS 方式から wasm 方式に変えると、wasm が **−6,522 B**(A 389,440,863 → C0 389,434,341)。exnref 符号化の D は +91 B(389,438,402。C の 389,438,311)。節の構成(`wasm-objdump -h`): A は import 24、function 98,395。C は import 25(hooks の 8 + 17)、function 98,405、Tag 1 個。C0 は import 17、function 98,391、Tag 1 個。locals 超過の関数は A も C も同じ 19 個(`o1`)。
- V8 の `WebAssembly.compile`(node。`T/*`、各 3 回): **A 342 / 349 / 371 ms、C0 331 / 335 / 335、C2 334 / 334 / 333、C1 328 / 335 / 338、C 345 / 332 / 351**。`WebAssembly.instantiate` は 11〜13 ms。JSPI の有無で差なし。D(exnref)は 386〜391 ms(`D/nosusp-*`。2 回。+約 55 ms。符号化の差と読む)。
- `V8 の遅延コンパイル`(`--wasm-lazy-compilation` が既定で有効)の影響は、`compile` の時間が 0.33 s と短いことと合う(読み。無効にした場合は測っていない)。
- ビルド時間は §1(A、C、D とも units 約 218 s、o1 約 383〜401 s)。SJLJ=wasm や JSPI でコンパイル時間・RSS は変わらない(emcc の `units` 段の最大 RSS 8.83 GB、`o1` 3.64 GB)。

## 10. PM の判断が要る点

1. **ブラウザ化の setjmp 方式**: JSPI を使うなら emscripten 既定の SJLJ(JS の `invoke_*`)は使えない(F1、F2)。`-sSUPPORT_LONGJMP=wasm` に変える必要があり、これは `build_nmod_wasm.sh` の全 `emcc` 呼び出しのフラグになる(C コードの変更は無し)。決定 0115 項 1(「追加のフラグは不要」)の更新が要るか。**legacy 符号化(emcc 6.0.12 の既定)と exnref 符号化のどちらを使うか**: 両方とも node と Chrome で通った。exnref は標準化された形だが、wabt 1.0.36 が読めない(F5)。
2. **JSPI を採るか**: 実 wasm で a4 の経路(681 回の `EnterCriticalSection`、8 回の `WaitForSingleObject`、`LoadLibraryW` の再入、全 1,007,942 入口)の中断・再開が成り立つ(実測)。ただし**実際のメッセージループの経路は未測定**(§12)。JSPI を採らない場合の選択肢(`browser-event-loop.md` の M-A、M-C')との比較は、この測定の範囲外。
3. **`-finstrument-functions` の JS import**: 測定用のビルドが 2.36 億回呼ぶ JS の import(`emscripten_return_address`)は、JSPI 構成で約 +1.1 s(+46 %)。実エンジンのビルドが `-finstrument-functions` を持たなければ関係しない。トレースを残すなら、`ra0.c` のように wasm 内に置く(`call_site` は捨てる)。決定 0115 項 3(標準ドライバーの選び方)に関わる。
4. **Win32 の import の JS 化**: ブラウザ化した実エンジンで、Win32 API の import を JS に実装すると、1 回ごとに JSPI の切り替えが約 +5〜7 ns 乗る(トイと実 wasm の両方で)。毎秒の呼び出し回数は**未測定**なので、これが問題になるかは分からない。
5. **JSPI のスタックの既定(984 kB)の余裕**: a4 の経路は約 150 kB。メッセージループの深さを測る前に、984 kB を前提にしてよいか。

## 11. 規則違反の自己申告と作業の記録

- **`lift/out-jspi/` の大きさ(作業中)**: 最大 **約 8.7 GB**(`ow/` 8.4 GB。A、B、C、D の `.o` と wasm、生成 C)。使用量の上限 20 GiB 以内。終了時に大きなものを削除した(§13)。
- **正体の分からないディレクトリ**: 片付けの最中に、`lift/out-jspi/undefined/`(6.9 MB、372 ファイル、Chrome のプロファイルの中身。作成時刻 10:56)が見つかった。**作った経緯は特定できていない**(`--user-data-dir=undefined` で Chrome を起動した呼び出しがあった、と推測する。`v8flags.mjs` に引数を渡し損ねた可能性。確認していない)。`ps` でそれを使うプロセスが無いことを確かめ、`lift/out-jspi/` の中だけで削除した。ユーザーのプロファイルではない(`--user-data-dir` の指定先がこの作業ディレクトリの相対パスだったため)。
- **Chrome の残骸**: 1 回、`Page.navigate` が返らない問題(F3)の調査中に、タイムアウトした自分の `run-jspi.mjs` が起動した Chrome のプロセスが 1 組残った(PID 99890 の木。`--user-data-dir=lift/out-jspi/chrome-profile/dbg` を持つもの)。`ps` の `--user-data-dir` と起動からの経過時間で自分のものと判断して、その木(メイン、zygote、gpu、network、storage、renderer 各種)と、同じ経過時間の crashpad handler 2 つに `kill -9` を送った(crashpad handler は `--user-data-dir` を持たず、**経過時間が同じであることだけで自分のものと判断した**)。他のブラウザのプロセス(`playwright` の `chrome-headless-shell` 1 組は別の作業者のもの)には触れていない。以後の実行で、`ps` に自分のプロファイルのプロセスは残っていない。
- `/home/ubuntu/.config/google-chrome/Crash Reports/settings.dat` が作業中に更新された(Chrome のクラッシュ報告の設定。`--user-data-dir` を指定しても、crashpad のデータベースの場所は `~/.config/google-chrome/Crash Reports` を指す。起動のたびに触れている)。ユーザーのプロファイル(Cookie、履歴、拡張)には触れていない。**ただし、自分で `Crash Reports` に書いたわけではなく、Chrome が書いたものだが、ユーザーのディレクトリへの書き込みは回避できていない**(`--crash-dumps-dir` などで避ける手当てをしていない)。
- 規則の `sleep`/`timeout`/`nohup`/末尾の `&` はコマンド文字列に書いていない(`/usr/bin/time` と shell ツールの `timeout` パラメータは使用)。`run-jspi.mjs` の中の JS の `setTimeout` による待ちとサーバー側の待ちはツールのコード内で、シェルのコマンドではない。`tools/susprun.sh` などに `sleep` は無い。**1 回、フォアグラウンドで待つはずの最後の測定(未改造ドライバーの 33 分の完走)を、ツール側の操作でバックグラウンドに移された**(私が `background` を指定したのではない)。完了通知を受けてから結果を使い、他の測定を並行して走らせた(この間のタイミング測定は、33 分の完走と CPU・メモリを共有している: `R/*` と `X/*` の一部。時間の絶対値は他の測定より揺れうる。**同じ期間に走らせた他の時間の測定(`R/*`、`X/tick-*`)は、同じ構成の隣の実行と相対値で比べた**)。
- git の変更系操作(add、commit、reset、stash、checkout、clean、restore)は**1 回、誤って `git -C … checkout` を引数なしで実行した**(`toy/` の作業中。引数なしなので何も変更しないが、「このブランチは origin/main よりも 18 コミット進んでいます」と出力した)。`git status` と `git reflog` で、HEAD が 159a42e のまま別の作業者の c7d65bf の commit が積まれたこと以外に差が無いことを確認した。これは**規則違反**(禁止されている `checkout`)。副作用は確認できていない。
- python は `PYTHONDONTWRITEBYTECODE=1`(`env.sh` で設定。`find lift/out-jspi -name __pycache__` は 0 件)。python の標準ライブラリと衝突する名前のファイルは作っていない(`summarize.py`、`allruns.py`、`sumjson.py`)。`tools/win-replay/__pycache__/` は私のものではない(`git status` の未追跡に最初からある別の作業者の物)。
- 他の `lift/out-*`(特に `out-fruity4`、`out-web2`)は**触っていない**。読んだのは入力に指定されたもの(`out-ext`、`out-blend2d`、`out-qfc`、`out-wallQ/freetype`、`out-inventory/extracted`、`out-three`、`out-seed58`)だけで、SHA-256 の確認と DLL・種のコピー(`lift/out-jspi/res/`)に使った。`ps` に別の作業者の `out-fruity5` のビルドが見えた(私のものではない)。
- 資格情報、ユーザーのブラウザのプロファイルには触れていない。
- 作業はすべて自分で行った(AGENTS.md の「task agent を通じて」は、私がサブエージェントなので守れない)。emcc は同時に最大 2 本(dsp の単位 `m1` は 1 本ずつ)。
- `lift/harness/web/jspi/` をワークツリーの外(リポジトリのワークツリー)に置いた。`git status` の未追跡に `lift/harness/web/jspi/` が出る(commit は PM)。

## 12. 未確認・できなかったこと

- **実際のメッセージループとの接続**: `GetMessageW`、`PeekMessageW`、`WaitMessage`、`MsgWaitForMultipleObjects`、`Sleep` は a4 の完走で呼ばれない(0 種)ので、実際にブロックする API を JSPI で中断して再開したことは**無い**。測ったのは「呼ばれる API の呼び出しの手前で JS を `await` する」ことだけで、**入力イベントが届く、描画する、といった実際の再開の条件は無い**。`CreateFruityInstance` の先は 0116〜0117 の途中で、まだ動かない。
- **Firefox、Safari、Edge、headed の Chrome**。Chrome は headless 1 構成のみ。実音声、実 GPU、実入力は無し(`browser-event-loop.md` と同じ)。
- **未改造ドライバー(`emscripten_return_address` が JS のスタック解析)での JSPI なしの基準**(約 30 分)は走らせていない。JSPI ありの未改造の完走は 1 回(32:53.87)。
- **本番のビルド(`-finstrument-functions` 無し)**での JSPI のコスト。測ったのは測定用のビルド(フックあり)だけ。
- **JSPI のスタックの大きさ**が足りるかは、a4 の経路(約 150 kB)だけで、メッセージループの深さは未測定。`--experimental-wasm-growable-stacks` の挙動は未調査。
- **JSPI の中断中に別の wasm の export を呼ぶ**(イベントハンドラが `R[]` などの状態に触れる、`browser-event-loop.md` §4.3)ことはしていない。中断中に動いた JS は `setInterval` の空のカウンタだけ。
- **-O1 の 19 関数、ページ保護 fail-open、レジスタ・メモリの値の比較**は、`verification.md` の「未測定」のまま(今回の範囲外)。
- **SJLJ=wasm の setjmp/longjmp の意味が native と同じか**(別のフレームへの `longjmp`、スタックの巻き戻しの副作用)は、a4 の出力が一致したこと以外に確かめていない。ゲストの `longjmp` 3 回を wasm 側で個別には観測していない。
- **ビルドの再現性**(2 回作って SHA が一致するか)は測っていない(`verification.md` は A に相当する構成で 3 回一致を確認している)。
- **c7d65bf(別の作業者の最新の commit。`lift/runtime` と `main_two.c` が変わっている)での再ビルド**はしていない。`jspi.diff` が当たることだけ確認した。
- exnref 符号化の wasm を wabt で検査すること(F5)。`wasm-validate` は今回も実行していない。V8 が受理した(node と Chrome で実行できた)ことだけを確認した。
- 同じ測定を別の機械・別の emsdk で行うこと。

## 13. 成果物

- この文書: `docs/investigations/jspi-real-wasm.md`(コピーを `lift/out-git/jspi-results/` にも置く)。
- `lift/harness/web/jspi/`: `jspi.diff`(HEAD に `patch -p1`)、`fle-worker-jspi.js`、`index-jspi.html`、`run-jspi.mjs`、`tools/`(測定に使ったスクリプト)、`toy/`(最小の例の C)。
- `lift/out-git/jspi-results/`: 文書、`jspi.diff`、小さなログ(`all-runs.tsv`、`kind-summary.tsv`、ビルドの段ごとの時間、native の基準の a4 の出力、各実行の JSON の集計)。10 MB 以内。
- `lift/out-jspi/`(終了時): `jspi.diff`、`logs/`、`ref/`(native の基準の stdout)、`tools/`、`toy/`(C のソースだけ)。**削除したもの**: `target/`、`ow/`(生成 C、`.o`、wasm 7 種)、`emcache*`、`chrome-profile/`、`dist*/`、`res/`(再配置した DLL のコピー)、`src/`(`git archive` の展開。`jspi.diff` と `lift/harness/web/jspi/` で再現できる)、**`runs/`(各実行の生の出力 71 MB。全入口の記録 52 MB を含む)**。`runs/` の集計 JSON(222 件。`/` を `__` に置き換えた名前)と、代表の stdout・`/usr/bin/time -v` の出力は `lift/out-git/jspi-results/runs-summary/`、`stdout-samples/`、`time/` に残した。**本文の `F/…`、`X/…` などの名前は `runs/` の下のパスで、今は `runs-summary/F__….json` として読む**。全入口の生の記録(`X/depth-all-m0.raw.json`、`F/probe-entry-e13-m1.raw.json`)は残していない: §3.2 と §8 の「フレーム数の分布」「ゲストのスタック使用 28,592 B」「着地の数の内訳」はその記録から数えた値で、**再計算はできない**(再実行は `tools/susprun.sh` で約 40 秒)。
- 最終の大きさ: `lift/out-jspi/` は 1.4 MB(`du -sh`)、`lift/out-git/jspi-results/` は 1.7 MB、`lift/harness/web/jspi/` は 144 KB。作業中の最大は約 8.7 GB(`ow/` 8.4 GB)。
