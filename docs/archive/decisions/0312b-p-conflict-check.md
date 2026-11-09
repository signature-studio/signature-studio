# 0312b: gpu1 の present Worker 案(0311)と S0+S1 の衝突の事前確認(mt3。PM 指示 21:55。実測 + 読み。実エンジン・Chrome は 0)

**結論(冒頭)**: 文字どおりの衝突は **0**。P の 3 本(`0311-present-worker-src.diff`・`-test.diff`・`0311-tsconfig.diff`)は main HEAD(856d72a3)・S0・S0+S1 の `app/` に **`patch --dry-run` で全部 clean**(reject 0)。ずれたのは `app/src/main.ts` の 1 hunk だけで **offset 2 行**(S0 の test aid の 2 行が手前にあるため)。**P を S0+S1 の `app/` に実際に当てた複製**(`/tmp/opencode/pcheck`、製品の木には書いていない)で、**tsc 3 構成(tsconfig・test・webworker)0 エラー**、**vitest 全体 71 files・70 passed・1 skipped・1,323 passed・7 skipped・0 failed**(S0+S1 単独の 1,315 + P の試験 8)。実エンジンでの動作は**未確認**。

## 1. 触るファイルの重なり(file:line)

| 役 | 触るファイル |
|---|---|
| P | `app/src/engine/{client,desk-gl,protocol,worker}.ts`・新規 `desk-link.ts`・`desk-worker.ts`・`app/src/main.ts`(1 hunk、`@@ -555,6 +555,7 @@`)、試験 `desk-gl-link`・`desk-link`(新規)・`worker-diag`、`app/tsconfig.json`・`tsconfig.webworker.json` |
| S0(足場) | `app/src/main.ts`:**108**(`AppApi.deskWindowsText` の宣言)・**176**(実装)、`app/src/engine/fruity.ts`:**79**〜(`importGlueFromBlob`、pthread の Worker に glue を渡す)、`app/test/e2e/browser.mjs`(env) |
| S1 | `app/src/audio/{worker-hook.ts,page-audio.ts,fle-out.worklet.js}`、`app/test/unit/audio-{wiring,default}.test.ts` |

- **重なるのは `main.ts` だけ**で、行は重ならない(S0: 108・176、P: 555 付近の 1 行追加)。`client.ts`・`worker.ts`・`protocol.ts` を S0・S1 は**触らない**(今回の問い)。`fruity.ts` は P が触らない。
- 表: `tables/0312b-p-dry-run.txt`(3 基準 × 3 diff、rc=0、offset は main.ts の 1 件だけ)。

## 2. 意味の干渉(読み。ブラウザでは未確認)

1. **Worker の名前**: `audio/install.ts` は `Worker` の constructor を包み、**名前が `'fl-engine'` の Worker にだけ** `audio-init` を送る。P の present Worker は名前 `'fl-desk'`(`0311-present-worker-src.diff` の client.ts の hunk、`new Worker(new URL('./desk-worker.ts', …), { type: 'module', name: 'fl-desk' })`)なので**包まれない**。干渉なし。
2. **page 側のメッセージ**: `client.ts` の `onMessage` は `switch (m.t)` で **default 節が無い**(未知の `t` は無視)。エンジンの音声は `{type:'audio-ring'|'audio-pulse'}`(`t` ではなく `type`)で、`PageAudio.watch()` が別の listener で処理する。P が足すのは `desk-*` の `t` だけ。干渉なし。
3. **`stopAudioReport`**: P の `worker.ts` が `../audio/worker-hook.ts` から import する(`worker.ts:18`、呼び出し `:278`)。S1 は `worker-hook.ts` の `stopAudioReport` を**そのまま export**(`worker-hook.ts:30`)。tsc の webworker 構成(`desk-worker.ts`・`desk-link.ts`・`worker.ts`・`worker-hook.ts` を含む)も 0 エラー。
4. **エンジンのメモリ**: P は `fruity.memory`(WebAssembly.Memory)を present Worker に postMessage で渡す。S0+S1 のメモリは pthread の共有メモリ(これが無いと P の handover は成立しない)で、S1 の音声リングはその**静的領域(低い番地)**にある。P の present Worker が読むのは scene の記述で、リングとは重ならない。メモリが伸びたときに P の側が `memory.buffer` を取り直す必要は、S1 の page の SAB 保持(古い SAB のまま低い番地を読む)とは独立。
5. **実スレッド**: P が足す `servicePresent`・`ensureDesk` の呼び出し(`inputPoll`・idle の `onIdle`)は、足場の `fle_thread.c` で **Worker 0 のみ**(`:800` `if (MYW == 0)` 『UI Worker だけが page の入力 ring を持つ』、`:816` idle hook)。つまり E11 の `deskTick` と**同じスレッド**。新しい前提は増えない〔読み〕。
6. **S0 の test aid**: `AppApi.deskWindowsText()`(`main.ts:176`)は `client.deskDump(true)` = `desk-peek` を使う。P は `desk-peek` を present Worker 側の `glRead('desk-dump', d.windows ?? '')` に回す。P で present が handover されているとき、`windows` の文字列が足場の期待どおり入るかは**未確認**(試験用の補助で、S4 で外すか決める)。

## 3. 結論と提案

- P を先に main に入れても、S0+S1 を先に入れても、**textual な衝突は無い**。順序は自由。S0 の `main.ts` の 2 行(test aid)は S4 で外す案(`deskWindowsText` は製品に要らない)。入れる順が P → S0 なら `main.ts` の hunk は offset 無しで当たる。
- 実エンジンでの確認は、**S0+S1+P の bundle を gpu1 が 1 本測る**ときに初めて取れる(B1 の後)。私は作らない(P の diff を適用した bundle を作る必要があれば、`tree-ci` に P を当てて `run-ci-build.sh` で再 build、冷間 約 45〜60 分〔B1 の実測: objects 53 分は停止込み、link 11 分〕)。


## PM の判断と取り込み(2026-11-06 22:32、docs のみ)

取り込み: mt3 の最終(0312 の一部)として `docs/investigations/mt3w/0312/` の表・道具・差分とともに置いた。判断は決定 0312 の PM の節にまとめた(段階 C は採る、T の基準が通るまで実スレッドは main に出さない、P を先に、S0+S1+P の bundle の測定、Worker 数の式は「区分に当たらない選択」として測定用の既定)。**ブラウザでの実行は 0 件、依頼者の機械は 0 件**。
