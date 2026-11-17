# 0234h(mt3): B′ 第 2 段スパイクの記録(進行中。実エンジンの 2 Worker 結果はまだ無い)

状態: **進行中の記録**。更新: 07:58 JST。数字は『この機械・負荷 6〜29 の他の作業者あり』の実測だけ。読み(推測)は『読み』と書く。

## 1. 測ったこと(実エンジン、1 Worker)
| 構成 | 内容 | 結果 |
|---|---|---|
| n0 | F2xH7 NO_FS=0 非ハイブリッド、共有メモリ + R/FL/X の TLS(第 1 段) | cmp 84 行 / ref 55 行 = 基準表と全行一致 |
| ng | n0 + GIL(`-DFLE_GIL`: 全 `fle_imp_*` 約 831 本を `--wrap` で包み、`fle_call_guest`・`switch_to`・idle 待ちで離す。1 Worker では競合なし) | cmp 84 / ref 55 全行一致。`fle_gil` の名前つき停止 0。CPU は下 |
| nr | ng + 残りの文脈の TLS(YH、MM、x87 レジスタ/制御、MXCSR、setjmp 着地/SEH の top・cur、TEB/TLS ポインタ、コールバック深さ、dispatch キャッシュ)+ 2 Worker の仕組み(1 Worker で動かす限り使われない) | cmp 84 行 全行一致(1 Worker) |

GIL のエンジン単位の費用(ng と n0、参照プロジェクト script、5 組、各バンドルを別の物理コアに固定、load 14.7〜21.9): user CPU 全体 88.82 → 88.73 秒(−0.1%、t=−0.2)、再生区間 2.823 → 2.799 CPU 秒/仮想秒(−0.9%、t=−0.6)。検出できる差なし。表 = tables/gil-engine-cost-n0-vs-ng.txt。微小測定は取得+解放 8.0 ns(tools/gilbench.c)。

## 2. 2 Worker の仕組み(nr に入れた物。FLE_WORKERS=2 の足場)
- 第 2 Worker = emscripten の pthread(`pthread_create`、`-sPTHREAD_POOL_SIZE=0`)。JSPI ビルドでは pthread の開始ルーチンが JSPI スタック上で走る(玩具で確認)ので、既存の `fle_thread_lib.js` の切替がそのまま使える。開始ルーチン = スケジューラの idle 文脈 `BOOT`(T[FLE_THREAD_MAX])。
- スレッド表は共有。`T[i].worker`(0 = 最初のスレッド、1 = 他すべて)、`cur` と `pending_free` は TLS、`pick_other` は自分の Worker のスレッドだけ。READY にした相手の Worker の wake word を bump + notify(`try_wake`、CreateThread、ResumeThread)。idle の待ちは Worker 0 が `Atomics.waitAsync`(JSPI の中断。イベントループが回る)、Worker 1 が `memory.atomic.wait32`。
- 時計は 2 Worker のモードでは実時計(`performance.timeOrigin + performance.now()`、Worker 間は共有の最大値で単調)。`advance_to` と読みの+10 tick は止める。
- node 駆動: `lift/harness/mt2/node-bundle-run-rt.mjs`(glue の `new Worker` を worker_threads に写す薄いクラス)+ `rt-worker-shim.mjs`。

## 3. 止まり(未解決)
FLE_WORKERS=2 で cmp-short を走らせると最初の CreateThread(suspended で 2 本)と SetThreadPriority までは進み、Worker 1 は 18 ms で起動メッセージ(cmd 1 load、cmd 2 run)を受ける。その後、プロセス全体が CPU 0% で眠る(2 回、70〜80 秒)。コードを読んで見つけた漏れ: ResumeThread が他 Worker のスレッドを起こさない(修正済み、nr2 以降)。第 1 Worker が GIL を待って同期 wait に入り、第 2 Worker が GIL を持ったまま第 1 Worker のイベントループを要る呼び出し(pthread から proxy される stdio など)を待つ組み合わせも読みの候補(未確認)。nr2 の診断行(`__fleDbg`)で確かめる。
