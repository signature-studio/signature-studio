---
draft: "148"
title: "重さの原因が emscripten のランタイム・グルーかを、実測の内訳で決め、その上で zig cc と自前のグルーで置き換えられるかを決める"
type: Task
milestone: "M4 FL に必要な物の配布"
parent: "012"
blocked_by: []
labels: ["delivery"]
size: L
priority: Urgent
project: signature-studio/3
---

## 目的

依頼者は、次の問題提起をした(2026-11-24。原文)。

> 一応言っておくと、**使い物にならないレベルで重すぎるのは emscripten(もしくは emscripten が吐き出すグルーコード)が原因だろ、という問題提起です**

依頼者は、emscripten の採用を取りやめ、zig cc の採用を検討していたと述べた。進捗を質問された。この Issue の動機は、この問題提起にある。目的は性能であり、P3(Song Mode で音が途切れない)・P2(UI が 120 fps で動く)・P1(10 秒以内に起動する)の順に優先する。サイズは最後。

仮説(未確認): 重さの主因は、emscripten のランタイム(JS のグルー・JSPI・pthread・ヒープの管理)である。この Issue は、まずこの仮説を実測で確かめる。仮説が外れたなら、重さの主因を、wasm の質・描画・runtime の C・ページ側の JS から探す。

この Issue は次の順で進む。

1. **段階 0(この Issue の受け入れ対象)**: 重さの内訳を実測する。重さが emscripten のランタイム・グルーに由来するのか、それ以外(lift した C の wasm の質・最適化の水準・Blend2D の描画・runtime の C・ページ側の JS)に由来するのかを、区分ごとの時間で決める。あわせて、サイズ優先の最適化の水準を速度優先に替えたときの差を測る。成果物は `pm-work/findings/engine-cost-attribution.md` と `.json`。
2. **判定**: 段階 0 の結果で、emscripten のグルーを zig cc と自前のグルーに替える価値があるかを、下の「0-7 判定の規則」で決める。
3. **段階 1(判定で「置き換える価値がある」場合だけ。この Issue の受け入れには含めない)**: emscripten を使わず、zig cc で、生成した C の compile と link ができるかを実測する(下の「段階 1」の節)。判定が「価値がない」なら行わない。

この Issue は本番のビルドを変えない。段階 0 は、測定と判定だけを行う。

## 既知の実測(数字と出典)

以下は PM が `pm-work/findings/` と下書きで確かめた値。作業者は各出典を読み直し、食い違いがあれば報告に書く。

1. **起動直後の途切れ**: `bootLast60sAt10s`(起動の 10 秒時点の直前 60 秒の underrun の数)。切り替え点を入れる前の基準は 2,071(n=3 の値は 2,071・2,105・2,003)。切り替え点を入れた版(main に入っている。#1352 の段 A)は 4・4・25(中央値 4)。切り替えを通らない最長の区間は、基準で 1,102 ms(主因は `f_344fcf0` など。wasm に載らない C)、切り替え点を入れた版で 421 ms(根の直下の `f_1375990` が 266 ms、主な葉の `bl::Pipeline::Reference`(Blend2D の塗り。wasm の C++)が 123 ms)。出典: `pm-work/findings/boot-burst-1352.md` の 0・1・4 節。
2. **マウス 1000 Hz**: Worker の忙しい時間が、無入力の 1.49 秒/60 秒から 8.9 秒/60 秒に増えた。増えた上位は `(program)`・`f_863b50`・`postMessage`・`decode`・`setTimeout`・`fle_win_input_view`・`poll_waiters`。出典は下書き 109 の 7 番(`pm-work/issue-drafts/wave1f/109-audio-under-physical-screen-mouse.md`)。以前の記録にあった測定ファイル名(`measure-109-attempt2.md`)は `pm-work/findings/` に無い(未確認)。
3. **起動の所要時間(`elapsedMs`)**: 6.8〜11.2 秒。10 秒を超える run がある(10,716 ms・10,870〜11,237 ms)。出典: `pm-work/findings/browser-panel-118.md`・`browser-panel-1355.md`・`blocks-samples-1366.md`。
4. **最適化の水準(事実)**: 製品の wasm の最適化は、サイズ優先の `-Oz`。
   - `lift/harness/web/wasm-opt-c.cmd:1`(`wasm-opt -Oz`。`lift/crates/fe-build/src/main.rs:314-316` の wasm-opt 段が使う)。
   - `lift/harness/web/wasm-opt.args:1`(`-Oz`。`lift/harness/hyb/hybrid.cmd:20-21` が `--wasm-opt-args` として渡す。hybrid 段)。
   - `lift/harness/web/cflags-u.args:1` と `lift/harness/web/cflags-ai.args:3`(gen の C の `-Oz`。`main.rs:274-278` が、`plain-units.txt` と `ai-units.txt` の単位に使う)。
   - 製品の名前も `fl-Oz`・`fl-engine-Oz`(`lift/harness/web/pack.args:2,4`。`main.rs:512`)。
   - 速度寄りの旗: runtime の C は `lift/harness/web/cflags-rt.args:1` の `-O2`、harness の C は `cflags-h.args:1` の `-O1`、b2d の C++ は `cflags-b2d.args:2` の `-O2`。Blend2D の em++ 旗(`main.rs:30-35`)は `-O2`・`-O3`・`-O2` の並び。後の旗が有効になると推定(未確認)。
   - リンクは `lift/harness/web/link.rsp:1` の `-O0`。

## 段階 0: 重さの内訳と最適化の水準

### 0-1 道具

- 新しく作る: `tools/engine-cost/measure.mjs`(測定)と `tools/engine-cost/run.sh`(ビルド・測定・成果物の作成。受け入れの入口)。
- 再利用する: `tools/p3-audio-probe/measure.mjs`(`--profile` の CPU プロファイル。engine の Worker に対して `Profiler.enable`・`Profiler.start`・`Profiler.stop` を使う。#1352 の使い方)、`tools/p3-audio-probe/profile-names.mjs`(`wasm-function[N]` を名前に直す)、`tools/p2-frames/measure.mjs`(マウスを 1200 Hz で送る)。
- 場面 (ii) の再生の方式は、#1349 の測定の道具(`measure-live.mjs`)にある。この道具は main に無く、`pm-work/findings/measure-live-109.patch` に在る。参考にしてよい。
- 製品を変えない。計測と変種のための変更は `tools/engine-cost/` の中だけに置く。`app/`・`lift/`・`ci/` に触れる push は、本番のデプロイ(`.github/workflows/deploy.yml` の `on.push.paths`)を動かす。

### 0-2 場面

各場面は 60 秒の窓。`engine-idle` の時点を t0 とする。DPR は 2 点とする。1.00 と、1.01〜3.00 の範囲から無作為に選んだ 1 点(選んだ値を結果に書く)。各場面・各 DPR で n=3。

- (i) 起動後・無操作・再生なし: t0+30 秒〜t0+90 秒。
- (ii) 無操作の再生中: Playlist を再生し、再生が始まってから 60 秒。音が出なくてよい。
- (iii) マウス 1000 Hz 以上(1200 Hz で送る)を 60 秒間。

合計 18 run(3 場面 × 2 DPR × n=3)。

### 0-3 区分(CPU プロファイルの自己時間)

Worker の全て(engine のスレッドを持つ Worker。選び方は `tools/p3-audio-probe/measure.mjs` の `engineWorker` の通り)の、窓の中の自己時間を、次の区分に分ける。

- **A. emscripten のランタイム・グルー**: 関数の所在(callFrame の url)が、emscripten の生成した glue(`engine-glue` の JS。`--js-library` の 3 件を含む)のもの。加えて、名前が `_emscripten_*`・`invoke_*`・`__syscall_*`・`emscripten_futex_wait`・`updateMemoryViews`・`HEAP*`・`Atomics.wait`、JSPI の包み、pthread の Worker の起動。
- **B. lift した C 由来の wasm 関数**: 名前が `f_<16 進>`(`lift/out-app/gen/wasm-func-names.tsv` で引く)。
- **C. C++ の描画**: 名前が `bl::`(Blend2D)で始まるもの。
- **D. runtime の C**: `lift/runtime/` の関数(`fle_*`・`poll_waiters` など)。
- **E. ページ側と Worker 側の JS**: app のビルド物の関数(url で判定)と、ブラウザ API の呼び出し(`postMessage`・`decode`・`setTimeout`・`structuredClone` など)。
- **F. ブラウザの内部**: `(program)`・`(garbage collector)`・`(root)`。
- `(idle)` は別に出す。忙しい時間には入れない。
- 区分に入らないものは `unclassified` に出す。

忙しい時間 = A + B + C + D + E + F + unclassified。`classifiedShare` = (A + B + C + D + E + F) ÷ 忙しい時間。受け入れは、全 run で 0.95 以上を求める。

### 0-4 境界・JSPI・メモリ・待ち

測れない項目は `null` と理由を書く。測定の道具の側で数え、製品のコードは変えない。

- 境界の呼び出し(JS と wasm の間。秒あたり)。lift の import(`fle_imp_*`)と、emscripten の import を分けて数える。
- JSPI の suspend と resume の回数と、合計の時間。
- `memory.grow` の回数と時間。
- 共有メモリ(pthread)の `Atomics.wait` の時間。

### 0-5 対照実験(境界の費用)

同じ wasm の、熱い import 1 本を、100 万回呼ぶ微小な測定を 2 通りで行う。(1) 今の emscripten の包み(JS のグルー経由)、(2) 包み無しの直接の import。1 回あたりの差(ns)を出す。微小な wasm の作り方は作業者が決めてよい。道具は `tools/engine-cost/` の中に置く。

### 0-6 最適化の水準

- 基準は、現行の `-Oz`(製品と同じ出力)。同じ 60 秒の窓で測る。
- 速度寄りの変種 O2 と O3 を作る。変えるのは、gen の C(`cflags-u.args`・`cflags-ai.args` の `-Oz`)と、wasm-opt(`wasm-opt-c.cmd`・`wasm-opt.args` の `-Oz`)だけ。runtime(`-O2`)・harness(`-O1`)・b2d は現行のまま。
- 旗は `tools/engine-cost/` の中の別ファイルで与える。`lift/` は変えない。出力は `lift/out-app/` の下の別の名前にし、本番の `fl-Oz` の出力を上書きしない。
- 比べる: 場面 (i) と (iii)、DPR 1、n=3。忙しい時間・underrun の数(`underrunQuanta`)・`elapsedMs`・wasm のバイト数(`wasmBytes`)・起動前の取得の合計(`preBootBytes`。P1 に効く)。

### 0-7 判定の規則

この規則は、測る前に決めた。数字を変えてよいが、変えた理由を報告に書く。

- **emscripten のグルーを替える価値がある**: 区分 A が、忙しい時間の **15% 以上**(場面 (iii) の平均)である。または、境界の呼び出し 1 回あたりの差が **50 ns 以上**で、呼び出しが秒あたり **10 万回以上**である。この場合、段階 1 を次の Issue の案として報告に書く(起票は PM)。
- **価値がない**: 重さの主因は B・C・D(wasm の質・描画・runtime)で、emscripten の置き換えは主因に効かない。段階 1 は行わず、「判定により、行わない」と報告に書く。
- **最適化の水準を先に変える**: O2 か O3 が、Oz より忙しい時間を **20% 以上**減らす。この場合、最適化の水準を変える Issue を先に起票する(報告に書く。起票は PM)。

判定の結果は、場面 (iii) と、場面 (i)・(ii) の値を、区分ごとに並べて書く。

### 0-8 成果物

- `pm-work/findings/engine-cost-attribution.md`: 場面と DPR ごとの区分の表(A〜F・unclassified・idle の ms と割合)、境界・JSPI・`memory.grow`・`Atomics.wait`、対照実験、最適化の水準の比較、出力のバイト数、判定、測れなかった項目と理由。
- `pm-work/findings/engine-cost-attribution.json`。形は次のとおり。

```
{
  "nproc": number, "loadavg": [number, number, number],
  "runs": [18 件: {"scene": "i"|"ii"|"iii", "dpr": number, "n": 1|2|3, "busyMs": number, "idleMs": number,
            "classesMs": {"A": number, "B": number, "C": number, "D": number, "E": number, "F": number, "unclassified": number},
            "classifiedShare": number, "boundaryCallsPerSec": number|null, "jspi": {"suspends": number, "resumes": number, "ms": number}|null,
            "memoryGrow": {"count": number, "ms": number}|null, "atomicsWaitMs": number|null,
            "underrunQuanta": number|null, "elapsedMs": number|null, "notes": string}],
  "contrast": {"calls": 1000000, "nsPerCallEmscripten": number|null, "nsPerCallDirect": number|null, "notes": string},
  "optLevels": [{"variant": "Oz"|"O2"|"O3", "busyMs": number|null, "underrunQuanta": number|null, "elapsedMs": number|null,
                 "wasmBytes": number|null, "preBootBytes": number|null, "notes": string}]
}
```

`busyMs` と `classesMs` は、60 秒の窓の値(ms)。

### 0-9 作業の進め方

- 最初に `date -u` で開始の時刻を記録する。**4 時間 30 分を過ぎたら**、受け入れが通っていなくても、作業ツリーを commit し、報告を書いて終える。
- ビルドは約 35 分。現行の `-Oz`、変種 O2、変種 O3 の 3 回。フォアグラウンドで直接実行し、シェルツールの `timeout` 引数に 5400000 を渡す。背景で起動して待たない。
- sleep・timeout コマンド・nohup・disown・末尾の `&` は使わない。
- 時間が足りないときは、(c) の場面を (iii) の 1 つに絞る(n=3 は保つ)。測れなかった項目は `null` と理由で書く。
- 報告には、場面と DPR ごとの区分 A〜F の割合、判定、判定の数字を変えたか、最適化の水準の比較、測れなかった項目と理由、開始と終了の時刻を書く。
- 段階 0 の成果物は、受け入れが通らなくても commit する。

### 0-10 受け入れ(段階 0)

```accept
bash tools/engine-cost/run.sh && node -e 'const r=JSON.parse(require("fs").readFileSync("pm-work/findings/engine-cost-attribution.json","utf8"));const K=["A","B","C","D","E","F","unclassified"];const runs=Array.isArray(r.runs)&&r.runs.length===18&&r.runs.every((x)=>x&&typeof x.classifiedShare==="number"&&x.classifiedShare>=0.95&&x.classesMs&&K.every((k)=>typeof x.classesMs[k]==="number"));const opt=Array.isArray(r.optLevels)&&["Oz","O2","O3"].every((v)=>r.optLevels.some((x)=>x&&x.variant===v&&(typeof x.busyMs==="number"||(typeof x.notes==="string"&&x.notes.length>0))));const con=!!r.contrast&&(typeof r.contrast.nsPerCallEmscripten==="number"||(typeof r.contrast.notes==="string"&&r.contrast.notes.length>0));process.exit(runs&&opt&&con?0:1)' && test -s pm-work/findings/engine-cost-attribution.md && bash guard/scan.sh pm-work/findings/engine-cost-attribution.md pm-work/findings/engine-cost-attribution.json tools/engine-cost/*
```

判定の基準:
- `bash tools/engine-cost/run.sh` が 0 で終わる。測れなかった項目は `null` と理由で書けば、0 で終わる(ビルドの失敗は 1)。
- `pm-work/findings/engine-cost-attribution.json` の `runs` が 18 件で、各 run の `classifiedShare` が 0.95 以上、`classesMs` に A〜F と `unclassified` の数値がある。`optLevels` に Oz・O2・O3 があり、`busyMs` の数値か `notes` の理由がある。`contrast` がある。
- `pm-work/findings/engine-cost-attribution.md` が空でない。
- `guard/scan.sh` が 0 件。
- `git diff --name-only` に `app/`・`lift/`・`ci/`・`.github/`・`inputs/` が含まれない(commit の前に確かめる)。

## 段階 1 の本文(判定で「置き換える価値がある」場合に、次の Issue として行う)

この Issue の受け入れには含めない。以下は、段階 1 の本文として残す。

## 背景

以下は PM が 2026-11-24 に作業ツリーを読んで確かめた事実。件数は `git grep` の出力による。作業者は同じ確認をやり直し、食い違いがあれば報告に書く。

1. 今のビルドは emscripten で動いている。
   - `ci/versions.env`: `EMSDK_VERSION=7.0.0`・`EMSDK_REPO`・`EMSDK_REV`(emsdk のリビジョン)・`EMSDK_DIR=${EMSDK_DIR:-/opt/emsdk}`。そのコメントは、emcc 7.0.0 の emsdk を挙げ、同梱の wasm-opt が binaryen 133 だと述べる。
   - `.github/workflows/deploy.yml`(52〜54 行): `emscripten-core/setup-emsdk@v16` を `version: 7.0.0` で直書き。`.github/workflows/agent.yml`(495〜499 行): 同じ action を、版の指定に env の `EMSDK_VERSION` を渡して呼ぶ。その値は 473〜478 行で `ci/versions.env` から読む。`.github/workflows/actions.lock` が、この action を固定している。
2. 本番の段は `lift/crates/fe-build/src/main.rs` の `fe-build engine` が行う。段と件数は `lift/crates/fe-build/tests/plan/engine.txt`(計画の記録)による。
   - stage `blend2d`(6 step。git の取得・checkout・patch・一覧の検査)。
   - stage `blend2d-compile`(159 step)。`em++`。旗は `BLEND2D_CXX_FLAGS` の 24 件。対象は `lift/harness/b2d1/blend2d-sources.txt` の 159 行。
   - stage `blend2d-archive`(3 step)。`emar qc`(1)・`emranlib`(1)・既存の `.a` の削除。
   - stage `compile`(171 step。rt 90 + h 14 + gen 66 + b2d 1)。170 件が `emcc -c @<cflags>`、1 件が `emcc @lift/harness/web/cflags-b2d.args`(`b2d_swap.cpp`)。`cflags-rt.args`・`cflags-h.args`・`cflags-u.args`・`cflags-ai.args` は `-sSUPPORT_LONGJMP=wasm` を含む。PM の概算は「172 段・約 434 秒」。出典は未確認。計画の記録は 171 段。
   - stage `link`(1 step)。`emcc @lift/harness/web/link.rsp @lift/out-app/exports.rsp <objects>`。`link.rsp` は 51 行。`-s` の旗 17、`-Wl,` の旗 17、`--js-library` 3 件(`lift/harness/web/threads/fle_thread_lib.js`・`lift/harness/web/fruity/fle_idle_lib.js`・`lift/harness/web/filedlg/fle_filedlg_lib.js`)。主な旗: `-sJSPI`・`-pthread`・`-sPTHREAD_POOL_SIZE=0`・`-sFILESYSTEM=0`・`-sMODULARIZE=1`・`-sEXPORT_ES6=1`・`-sINITIAL_MEMORY=671088640`・`-sMAXIMUM_MEMORY=2147483648`・`-sGLOBAL_BASE=268435456`・`-Wl,-Map,lift/out-app/web/link.map`。
   - stage `wasm-opt`(1 step)。`wasm-opt -Oz …`(`lift/harness/web/wasm-opt-c.cmd`)。
   - stage `hybrid`(3 step)。`fe-wasmgen hybrid` が `--wasm-opt wasm-opt`・`--wasm-merge wasm-merge`(binaryen)を呼び、`--link-map lift/out-app/web/link.map` を読む。続けて `glue()`(`main.rs` の 138〜159 行)が `flengine6.js` を `engine-glue.js` に書き換える。この書き換えは emcc の出力の書式に依存する。`function _f_<16 進>(...args` で始まる範囲を `stub = true;` の行まで削除し、`f_<16 進>: _f_<16 進>,` の行を削除する。
   - `toolchain_bin()`(`main.rs` の 408〜414 行)は、emcc のあるディレクトリの `../bin` を PATH の先頭に置く。wasm-opt と wasm-merge を emsdk の版にするため。zig には binaryen が含まれない。
   - 版の食い違い: `lift/harness/web/pack.args` は `--host-files emscripten-6.0.12` を渡す。`app/scripts/pack-assets.mjs` は、その文字列を manifest の `hostFilesAbi` に書く。`app/src/engine/hostfiles.ts` の `SUPPORTED_EMSCRIPTEN` は `['6.0.12']`。一方、ビルドの emsdk は 7.0.0。ABI の文字列は、link した glue から取っていない(未確認)。
3. JS(`app/`)の依存。
   - `app/src/engine/fruity.ts`: glue を ES module として import し(`default` の関数。`importGlueFromBlob`)、Module に `locateFile`・`arguments`・`print`・`printErr`・`onExit`・`onAbort`・`onRuntimeInitialized`・`fleIdle`・`fleThreadFailed`・`fleXmodLoad`・`fleInputPoll`・`fleInputDone`・`fleFileDlg`・`fleFileDlgProgress`・`preRun`・`instantiateWasm` を渡す。`preRun` は `opts.ENV` を使う(glue に `-sEXPORTED_RUNTIME_METHODS=ENV,wasmExports,stackRestore` が要る)。
   - `app/src/engine/hostfiles.ts`: glue の libc の system call を差し替える。`__syscall_openat`・`__syscall_stat64`・`__syscall_lstat64`・`__syscall_newfstatat`・`__syscall_fstat64`・`fd_read`・`fd_pread`・`fd_seek`・`fd_close`。`-sFILESYSTEM=0` のため、glue に FS は無い。
   - `app/src/engine/features.ts`: JSPI(`WebAssembly.Suspending`・`WebAssembly.promising`)と、emcc 6.0.12 が出す legacy 形式の wasm 例外を要求する。
   - `app/src/plugins/ott-host.ts`: 外部プラグインの harness の wasm に、emscripten の import(`_emscripten_throw_longjmp`・`emscripten_resize_heap`)を自前で与え、`_emscripten_stack_alloc` を呼ぶ。
   - `app/src/bundle/manifest.ts`・`app/shared/manifest-guard.ts`・`app/shared/perf-summary.mjs`: engine の種類の文字列 `emscripten-fruity-v1` と、`hostFilesAbi` の形式 `^emscripten-…`。
4. runtime C(`lift/runtime/`。`third_party/` を除く)の依存。
   - `#include <emscripten.h>`: 7 ファイル(`fle_appfile.c`・`fle_appview.c`・`fle_audio_ring.c`・`fle_audio_transport.c`・`fle_desk.c`・`fle_filedlg.c`・`fle_thread_wasm.c`)。`#include <emscripten/stack.h>`: `fle_thread_wasm.c`。
   - `EMSCRIPTEN_KEEPALIVE`(export の印): 6 ファイル(上の 7 から `fle_audio_transport.c` を除く。`fle_audio_ring.c` は `FLE_AUDIO_EXPORT` の定義で使う)。
   - `EM_JS` 8 件と `EM_ASYNC_JS` 1 件(`fle_js_wait`)が `fle_audio_transport.c` にある(計 9)。音声の ring の読み書きを JS で行う。
   - `emscripten_stack_get_current`(4 件)と `emscripten_stack_get_free`(1 件)が `fle_thread_wasm.c`。
   - `emscripten_builtin_memalign`・`emscripten_builtin_free`(`fle_malloc16.c` に 8 行。注釈を含む)。`malloc`・`free`・`calloc`・`realloc` を、emscripten の組み込みの関数で差し替えている(16 バイト整列)。
   - `#ifdef __EMSCRIPTEN__` で囲まれた行: 35 行、15 ファイル。
   - `lift/harness/wasm/jspi_hooks.c`(harness): `EM_JS` 5 件、`EM_ASYNC_JS` 1 件(`jspi_suspend`)、`emscripten_get_now`。
5. `--js-library` の 3 件は、emscripten の `mergeInto(LibraryManager.library, …)` の書式で書かれている。補助の `$FleThreads`・`$FleIdle`・`$FleFileDlg`、属性 `__deps`・`__sig`・`__async: true`。使う emscripten の名前は `UTF8ToString`・`stringToUTF8`・`lengthBytesUTF8`・`malloc`・`free`・`stackRestore`・`HEAPU8`。`fle_thread_lib.js` は、2 番目以降の guest thread を `WebAssembly.promising(fle_wasm_thread_entry)` の呼び出しで走らせる(`-sPTHREAD_POOL_SIZE=0`)。
6. 周辺の依存。
   - `lift/crates/fe-lift/tests/gs_stacklimit.rs`(543〜585 行)と `ldst_switch.rs`(257〜284 行)は `/opt/emsdk/upstream/emscripten/emcc` を決め打ちし、無ければ skip する。`tools/verify-kit/tools/run-e14-e2e.sh` も `/opt/emsdk/upstream/emscripten` を決め打ちする。
   - `ci/lib/common.sh` は `app/scripts/build-engine-bundle.sh` を呼ぶが、このファイルは tree に無い(`git ls-files` に無い)。`ci/lib/common.sh` を読むのは `lift/harness-docs-tools/alpha1/mk-wasm.sh` だけ。
   - 製品の段の外で emcc・em++・emar を呼ぶ `*.sh` は 6 件: `lift/harness/wasm/build.sh`・`lift/harness/vst3a/build_vst3a_wasm.sh`・`lift/harness/fp1/build_diff_wasm.sh`・`lift/harness/web/fruity/link-web.sh`・`lift/harness/web/mt1/plugin-worker/build.sh`・`tools/verify-kit/tools/run-e14-e2e.sh`(ほかに `ci/lib/common.sh`)。
7. 取得の再試行。#1374(blend2d の取得を、curl と固定の SHA-256 で行い、再試行する計画)は、main には入っていない。`fe-build` の取得は `git clone` のまま。この Issue の zig の取得は、同じ方式を `tools/zig-vs-emcc/run.sh` の中に書く。
8. zig の版。公式の配布の索引(download の index)による、安定版の最新は 0.17.1。その `x86_64-linux` の tarball `zig-x86_64-linux-0.17.1.tar.xz` の SHA-256 は `3581863e3cbcfd0fce0efc682ce7e9e93da96668a977791d356a26d2a7e9b17a`、サイズは 57411036 バイト。PM が、索引の値と実物の `sha256sum` の一致を確かめた。
9. 受け入れの場所。受け入れの job は、`lift/out-app/host`(抽出物)をキャッシュから置き(`.github/workflows/agent.yml` の 522〜527 行)、emsdk を入れ(495 行)、`lift/target/release/fe-build` と `fe-lift` を作る。`lift/out-app/gen/` は fe-build の gen 段で作られる。その引数は `lift/harness/b2d2/gen.args`(実行ファイルは `lift/target/release/fe-lift gen-all`)。`lift/out-app/gen/` が無いときは、この引数で gen を実行してから測る。
10. 版の固定の場所。`ci/` の変更は、`.github/workflows/deploy.yml` の `on.push.paths`(`ci/**`)により、main への push で本番のデプロイを動かす。この Issue は本番を動かさないため、zig の版は `ci/versions.env` に足さず、`tools/zig-vs-emcc/zig.env` に置く。

## 読む物

- `lift/crates/fe-build/src/main.rs`(特に 102〜109・193〜208・286・303〜316・408〜414 行と、`glue`(138〜159 行))
- `lift/crates/fe-build/tests/plan/engine.txt`
- `lift/harness/web/link.rsp`・`cflags-rt.args`・`cflags-h.args`・`cflags-u.args`・`cflags-ai.args`・`cflags-b2d.args`・`wasm-opt-c.cmd`・`glue.sed`・`pack.args`
- `lift/harness/hyb/hybrid.cmd`・`lift/harness/hyb/n0.cmd`
- `lift/harness/web/threads/fle_thread_lib.js`・`lift/harness/web/fruity/fle_idle_lib.js`・`lift/harness/web/filedlg/fle_filedlg_lib.js`
- `lift/runtime/fle_thread_wasm.c`・`fle_malloc16.c`・`fle_audio_transport.c`
- `lift/harness/wasm/jspi_hooks.c`・`lift/harness/wasm/build.sh`・`lift/harness/web/fruity/link-web.sh`・`lift/harness/vst3a/build_vst3a_wasm.sh`
- `lift/harness/parity/README.md`(ネイティブ対 wasm の比較。面 SHA)・`lift/harness/parity/node-bundle-run.mjs`・`lift/harness/two/run_node_jspi.cjs`(JSPI の node の駆動)
- `lift/harness/b2d1/blend2d-sources.txt`・`lift/harness/two/plain-units.txt`・`lift/harness/two/ai-units.txt`・`lift/harness/two/main_two.c`
- `app/src/engine/fruity.ts`・`app/src/engine/hostfiles.ts`(冒頭の注釈と、`__syscall_*`・`fd_*` の差し替え)・`app/src/engine/features.ts`・`app/src/plugins/ott-host.ts`・`app/scripts/pack-assets.mjs`(`hostFilesAbi`)
- `.github/workflows/agent.yml`(473〜499 行・522〜527 行)・`.github/workflows/deploy.yml`(1〜15 行と 40〜70 行)・`.github/workflows/actions.lock`・`ci/versions.env`・`ci/lib/common.sh`

## やること

1. 使用箇所の洗い出し(成果物 `pm-work/findings/emscripten-usage.md`)。
   - `git grep -nE 'emcc|em\+\+|emar|emranlib|emsdk|emscripten|EM_ASM|EMSCRIPTEN_KEEPALIVE|emscripten_|-sJSPI|-pthread|PTHREAD|wasm-opt|binaryen' -- lift app ci .github tools` で探す(git で管理されているファイルだけ。`lift/out*/`・`node_modules`・`*.ttf` は除く)。
   - 表を次の区分で作る。区分ごとに件数とファイル名。(a) compile(生成した C・Blend2D・runtime・harness)(b) archive(c) link(d) 後処理(wasm-opt・wasm-merge・glue の書き換え・link.map)(e) JS の glue(f) runtime C の emscripten の API(g) 環境(`.github/`・`ci/`・テスト・tools)。製品の段の外の harness の sh は、別の欄にする。
   - JS の glue に依存する物を 1 件ずつ特定する: JSPI の suspend と resume、pthread の代わりの promising、ファイルシステムの system call、`ENV`・`print`・`fleIdle` などの Module のフック。
2. 版の固定と取得(`tools/zig-vs-emcc/run.sh` の中)。
   - `tools/zig-vs-emcc/zig.env` に `ZIG_VERSION=0.17.1`・`ZIG_SHA256=3581863e3cbcfd0fce0efc682ce7e9e93da96668a977791d356a26d2a7e9b17a`・取得元の変数(`ZIG_TARBALL`)を書く。`ZIG_TARBALL` の値は、公式の索引から取る(その版の `x86_64-linux` の `tarball`)。
   - 取得は `curl` で行い、curl の再試行の機能を使う(シェルの sleep を使わない)。`sha256sum` で `ZIG_SHA256` と照合する。一致しなければ、取った物を捨てて失敗にする。展開は `lift/out-app/zig/` に `tar -xJf` で行う(`lift/out*/` は git で無視される)。
3. 測定(`tools/zig-vs-emcc/` の中の `measure.mjs` か、シェルの `run.sh`。Python を使わない)。各計測は n=3。時間は `/usr/bin/time -f '%e %M'`、サイズは `wc -c` で取る。
   - (a) 生成した C の 1 単位。gen の 66 件のうち、バイト数が最小と最大の 2 件。plain-units の件は `cflags-u.args`、ai-units の件は `cflags-ai.args` を使う。`emcc -c @<flags> <src> -o <obj>` と、`zig cc -target wasm32-wasi -c <flags> <src> -o <obj>` を比べる。emcc 専用の `-s…` の旗は zig に渡さない。落とした旗を JSON に書く。`-I lift/runtime` を付ける。`wasm-objdump -x`(無ければ wasm-tools)で、関数の数・import・export を比べる。
   - (b) Blend2D の 1 単位。`blend2d-sources.txt` の最小と最大の 2 件。`em++`(`BLEND2D_CXX_FLAGS` の 24 件)と `zig c++`(同じ旗。`-s…` は除く)。ソースは `lift/out-app/b2d-src/src/` にあり、`fe-build engine` の blend2d 段で取られる。取れていなければ、(b) は「未確認」と書き、理由を書く。
   - (c) link。全体の link は重い(app の注釈は、engine の wasm が 250〜670 MB と書いている)。まず小さい部分で試す(runtime の C 1 本と `lift/harness/two/main_two.c` を含む最小の wasm)。確かめる事: (1) `-sJSPI` の import と export の包み。zig 側では、JS の glue が `WebAssembly.Suspending` と `WebAssembly.promising` を手で与える必要がある。(2) `-pthread` の共有メモリ。`wasm-ld` の `--shared-memory`・`--import-memory`・`--max-memory`・`--initial-memory`・`--global-base`・`--export`・`--features` で足りるか。(3) `__wasm_init_tls`・`__wasm_init_memory` の扱い。(4) libc の system call の import の名前(emscripten の `__syscall_*` か、WASI の `wasi_snapshot_preview1`)。全体の link が 1 段の試行の時間に収まるなら、全体も試す。収まらなければ、その旨と見積もりを書く。
   - 置き換えに要る作業を数えて書く: JS の glue の行数の見積もり、runtime C の変更の箇所数、`glue()` の書き換えの変更。
   - (d) 出力の同一性。`lift/harness/parity/README.md` の方式(面 SHA の比較)か、`lift/harness/two/run_node_jspi.cjs` の方式で、同じ生成 C から作った emcc の wasm と zig の wasm の実行の結果を比べる。浮動小数点(`-ffp-contract=off`・`-fno-fast-math`・`-msimd128`・`fle_mx_*` の SSE の模倣)が同じかを確かめる。wasm 例外の形式(`features.ts` の legacy 形式)が zig の出力で同じかも確かめる。確かめられない物は「未確認」と書く。
   - (e) 全体の見積もり。compile(emcc -c 170 件と em++ 159 件)・link・wasm-opt の時間を、(a)(b) の実測から見積もる。危険の一覧を書く: 未対応の機能、出力の違い、binaryen(wasm-opt・wasm-merge)の出どころ、`hostfiles.ts` の差し替えの対象の変化、`glue()` の書式への依存、`link.map` の書式(`fe-wasmgen hybrid` が読む)、ABI の版の食い違い(6.0.12 と 7.0.0)。
4. 判定(成果物 `pm-work/findings/zig-cc-vs-emcc.md`)。項目ごとの表(emcc の値・zig の値・差)と結論。結論は「置き換えられる(段階)」「置き換えられない(理由)」「一部だけ置き換えられる」のどれか。置き換えられるなら、段階の計画を書く(段階 1: … 段階 N: emsdk の削除)。各段階は 1 回の試行(作業の上限は 4 時間 30 分、ジョブの上限は 5 時間 50 分)に収まる大きさにする。次の Issue の案として書く(起票は PM がする)。
5. JSON(成果物 `pm-work/findings/zig-vs-emcc.json`)。形は `{"zigVersion","emccVersion","units":[{"name","emccSeconds","zigSeconds","emccBytes","zigBytes","ok","notes"}],"link":{"ok","notes"},"parity":{"checked","same"}}`。`emccVersion` は `emcc --version` の 1 行目。`units` は (a)(b) の全件で、3 件以上。emcc の値は必ず入れる。zig が compile できなかった件は `ok` を偽にし、`notes` に理由(標準エラーの 1 行)を書く。

## 変えないもの(制約)

- `lift/crates/fe-build/`・`.github/`・`app/`・`lift/runtime/`・`lift/harness/`(既存のビルドの中身)・`ci/`・`tools/`(既存の物)・`guard/`・`spec/`・`inputs/` は変えない。読むだけ。段階 0 で新しく作るのは、`tools/engine-cost/`・`pm-work/findings/engine-cost-attribution.*` だけ。段階 1 で新しく作るのは、`tools/zig-vs-emcc/`・`pm-work/findings/` の zig に関する物だけ。
- 段階 0 は、`app/`・`lift/`・`ci/`・`.github/` に触れる commit をしない。push が本番のデプロイ(`deploy.yml`)を動かすため。
- 製品に、新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足さない。測定の設定と、最適化の水準の変種の旗は `tools/engine-cost/` の中だけに置く。段階 1 の測定の設定は `tools/zig-vs-emcc/` の中だけに置く(`-D__EMSCRIPTEN__` などを付けるかどうかは、測定の設定として報告に書く)。
- ベンダー名・Cloudflare の識別子を書かない。URL を本文と `pm-work/findings/` に書かない。
- Python を使わない(調査と検証を含む。製品にも使わない)。
- 一時ファイルは `.tmp-*` に置き、終わったら消す。
- sleep・timeout コマンド・nohup・disown・末尾の `&` は使わない。
- 製品のビルドと測定は Actions で行う。この機械で製品の段を回さない。

## 作業の進め方

- 最初に `date -u` で開始の時刻を記録する。4 時間 30 分を過ぎたら、受け入れが通っていなくても、作業ツリーを commit し、報告を書いて終える。
- compile の測定は数分。フォアグラウンドで、シェルツールの `timeout` 引数に大きい値(上限 5400000)を渡して直接実行する。背景で起動して待たない。
- 全体の link を試すのは 1 回。

## 段階 1 の受け入れ(この Issue では使わない。段階 1 の Issue の本文に accept として書く)

判定の基準:
- `bash tools/zig-vs-emcc/run.sh` が 0 で終わる。
- `pm-work/findings/zig-vs-emcc.json` が JSON として読め、`units` が 3 件以上。各件に `emccSeconds`・`emccBytes` の数値が入り、`ok` が真か、`notes` が空でない。`zigVersion`・`emccVersion` が文字列。`link`・`parity` が在る。
- `pm-work/findings/emscripten-usage.md` と `pm-work/findings/zig-cc-vs-emcc.md` が空でない。
- `guard/scan.sh` が 0 件。
- `git diff --name-only` の結果に、`ci/`・`.github/`・`app/`・`lift/`・`inputs/` が含まれない(commit の前に確かめる)。

## 報告の規則

- 1 行目は「実測: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には nproc と loadavg を添える。
- 実測と推定を分ける。「達成」と書かず、「置き換えられる」「置き換えられない」「一部だけ」「未確認」のいずれかで書く。出力の行を付ける。
- 報告に書く: emscripten の使用箇所の件数(区分ごと。`emscripten-usage.md` の表の値)、zig の版と SHA-256 の照合の結果、(a)〜(e) の値、判定、危険の一覧、測定の設定(旗の落とし方)。確認できなかった物は「未確認」と書き、理由を書く。

## 他のタスクとの関係

- blocked_by: なし。
- この Issue は本番のビルドを変えない。置き換えの実装は、判定の後に PM が起票する。
- `#1374`(blend2d の取得の再試行)と、同じ方式を使う。別の Issue。`lift/crates/fe-build/src/main.rs` は `#1374` が変える。衝突したら、作業の最後に `git pull --no-rebase --no-edit` で main を取り込み、`fe-build` の変更は触らない。
