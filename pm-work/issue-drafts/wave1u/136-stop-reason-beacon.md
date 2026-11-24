---
draft: "136"
title: "停止(stopped / 134)の理由を、ビーコンに別の項目として残す: 最後の fle: の行・== stop: の行・abort の種類・開く窓の答えの種類(個人の名前・パスを除く)"
type: Task
milestone: "M2 HiDPI と P2 の測定(P2)"
parent: "005"
blocked_by: []
labels: ["perf-beacons", "audio"]
size: M
priority: Urgent
project: signature-studio/3
---

## 目的

依頼者の実機で、評価用プロジェクトを File > Open で開いた後に、engine が `stopped` / 134(abort)で止まった。記録の `engine.exit.message` には、停止の理由が読める形で残っていなかった(#1377 の集計。`pm-work/findings/real-user-exits-2026-11-24.md` の節 3.2)。

この Issue は計測の Issue。次に実機で停止した時に、理由が記録から読めるように、停止の報告の中の項目を、ビーコンに残す。停止そのものは直さない(直すのは #{{121}}(#1365)・#{{128}}(#1372)の後)。

この Issue の目標は次の 3 つ。
1. 停止の理由の項目(`engine.exit.stop` と `engine.exit.evidence`)を、ビーコンに任意の項目として足し、読み手(`tools/perf-beacons/summary.mjs --exits` の表)に出す。
2. 開く窓の答えの種類(名前・パスは入れず、数だけ)を、`files.engine` に足す。
3. ビーコンの版は上げない(`BEACON_VERSION` は 3 のまま。古い記録と古い読み手を壊さない)。

## 背景

- 実測(記録。`real-user-exits-2026-11-24.md`): 版 d7a6e6f392f2 の 7 件のうち停止は 1 件(2026-11-24 06:56Z ごろ。`stopped` / 134。DPR 1.7647)。開く窓の応答の 3.8 秒後。`engine.exit.message` は `first non-background pixel on a drawing surface (decision 0126): ... [guest return 0x680e3d2]`。`fle:` の行は記録に無い。同じ型の停止は、runner の #1335 の最初の試行(File > Open の直後に `stopped (exit code 134)`)にも出た。
- 停止の報告の作り(コードの読み。実測ではない):
  1. web 版の停止の報告は `lift/harness/fruity.c` の `report()` が出す。abort は `lift/harness/wasm/trace_fruity.c` の `__wrap_abort` が、`trace_abort_hook`(`on_abort_wasm`)を呼んで、その報告を出してから `_exit(134)`。行は `fruity: == stop: …`・`fruity: runtime message: …`・`fruity: abort kind: …`・`fruity: guest stack at rsp …`・`wf1: … linear memory now N bytes …` など。
  2. `runtime message` は、`__wrap_fprintf` と `__wrap_fputs` が最後に受けた文字列。つまり「最後に書かれた診断の文」で、停止の理由とは限らない。06:56Z の記録では、その文は decision 0126 の証拠の文(`lift/runtime/fle_raster.c` が `fputs(buf, stderr)` で一度に書く)だった。
  3. `abort kind` は、その文が `fle: ` で始まるかで決まる(`fruity.c` の `kind_of`)。`fle_stop`(`lift/runtime/fle_env.c`)で止まった時だけ `fle_stop …` になり、それ以外は `(not a runtime message)`。
  4. web 版では `functions executed` と `entry log` を数えていない。web のビルド(`lift/harness/web/cflags-*.args`)に `-finstrument-functions` が無く、関数の入り口の計測が動かない。実記録の証拠の行も `function entries since the program began: 0` と書いている(`fle_raster.c`)。この 2 つは項目にしない。
- 捕捉と捨てられる場所(どこで捕捉され、どこで捨てられているか):
  - 捕捉: `app/src/engine/fruity.ts` の `print` / `printErr` → `app/src/engine/worker.ts` の `stdout` / `stderr` → `pushLog`(100 ms ごとの束。束の中では out の行が err の行より先に送られる: `flushLogs`)→ `engine-log` → `app/src/main.ts` の `engineLog.add` → `app/src/ui/panels.ts` の `EngineLogView.lines`(最大 50,000 行)。
  - 使われる所: `app/src/engine/fruity-log.ts` の `summarizeEngineLog`(画面の表示だけ)と、`app/src/perf/exit-message.ts` の `engineExitMessage`。後者は最後の 2,000 行から 1 行だけを選ぶ(runtime message → 最後の `fle:` / `limit:` の行 → `== stop:` の行 → trap の文の順)。結果は 400 字以内で、`app/src/main.ts` の `exit(info)` が `exitMessage` に入れ、`engine.exit.message` になる。
  - 捨てられる物: この 1 行以外の全て。`abort kind`、`wf1:` のメモリの大きさ、`fle:` の行(最後の 1 つ以外)、`guest stack` の 1 語目以外、`CreateFruityInstance` の経過時間、`guest threads`、`exit code`。
- 注意: 停止の報告(stdout)と `fle:` の行(stderr)は、同じ束の中で順序が入れ替わる。だから、位置ではなく行の内容で探す。
- 開く窓の答えの種類(`files.engine`): 答えは `app/src/files/engine-dialog.ts` の `select` で `filesUsage.c.engine.answered++` して `done(a)` に渡る。答えの項目(`a.items`)は guest パス(`guestPathOfOpfs`。`OPEN_FOLDER` の下)を持ち、読んだファイルの `file` は Open と folder の答えの項目に在る(Save の答えには無い)。名前とパスは記録しない(設計どおり)。

## 読む物

- `app/src/perf/exit-message.ts`(`engineExitMessage`・`sanitizeEngineText`・`EXIT_MESSAGE_MAX`)
- `app/src/perf/beacon.ts`(`BeaconInput.exit`・`exitBlock`・`buildBeacon`・`beaconText` の縮める段・`BEACON_VERSION` の注釈)
- `app/shared/perf.ts`(`beaconProblems` の `engine.exit`・`files.engine` の検査。`PERF_MAX_BODY`)
- `app/src/main.ts`(`exitMessage`・`exit(info)`・`makeBeacon`・`panelView`)
- `app/src/engine/worker.ts`(`pushLog`・`flushLogs`)・`app/src/engine/fruity.ts`(`print` / `printErr`)
- `app/src/ui/panels.ts`(`EngineLogView`)・`app/src/engine/fruity-log.ts`(`summarizeEngineLog`)
- `app/src/files/usage.ts`(`FilesUsage`・`zero()`)・`app/src/files/engine-dialog.ts`(`select`・`answer`)・`app/src/files/select.ts`(`planOpen`)
- `app/worker/perf-summary.ts`・`app/shared/perf-summary.mjs`(`engine.exit.message` の読み)
- `tools/perf-beacons/summary.mjs`(`--exits`・`exitsReport`・`exitRow`・`scrubWords`)
- `lift/harness/fruity.c`(`report()`・`kind_of()`・`on_abort_wasm`・`wf1:` の行)
- `lift/harness/wasm/trace_fruity.c`(`__wrap_fprintf`・`__wrap_fputs`・`__wrap_abort`)
- `lift/runtime/fle_env.c`(`fle_stop`)・`lift/runtime/fle_raster.c`(証拠の文。`fputs(buf, stderr)`)
- `pm-work/findings/real-user-exits-2026-11-24.md`(節 3.2・4・5)

## やること

1. 停止の理由の項目 `engine.exit.stop`(任意)。`app/src/main.ts` の `exit(info)` で `exitMessage` を作る所の近くで、`engineLog.lines` の最後の 2,000 行から取る。停止の報告の `== stop:` の行が在る時だけ付ける(無ければ `stop` を付けない)。項目:
   - `lastFle`: 最後の `fle: ` で始まる行(`[err] ` と `fruity: ` の接頭辞を外す)。400 字以内。
   - `stopLine`: 最後の `== stop: ` の行(接頭辞を外す)。200 字以内。
   - `abortKind`: `abort kind: ` の後の語。80 字以内。
   - `memoryBytes`: `wf1:` の行の `linear memory now N bytes` の N(整数。停止の時の wasm のメモリの大きさ。H2 の確かめに使う)。
   - 文字列は全て `sanitizeEngineText` を通す(ユーザー名・パス・ファイル名・メール・URL・IP を除く。既存の試験の形のまま)。
   - 証拠の項目 `engine.exit.evidence`(`stop` の外。`engine.exit` の直下): 最後の 2,000 行の中の、`first non-background pixel` で始まる行(`[err] ` を外す)。400 字以内。停止の報告が無くても、行が在れば付ける。
   - `engine.exit.message` は変えない(今の 400 字の文と、その計算は、そのまま)。
2. 開く窓の答えの種類を `files.engine` に足す(名前・パスを入れない。数だけ)。`app/src/files/engine-dialog.ts` の `select` の `answered++` の所で、答えの `a.items` から数える:
   - `answeredUnderDemo`(数): 答えの項目の guest パスに、`Demo projects` の区分(大文字小文字を問わない)が在った答えの数。
   - `answeredFlp`(数): 答えの項目の guest パスが `.flp` で終わった答えの数。
   - `answeredBytes`(4 つの数): `file` が在る項目の大きさの合計のバケット。`k64`(64 KB 以下)・`m1`(1 MB 以下)・`m16`(16 MB 以下)・`more`(それを超える)。`file` の無い答え(Save)は数えない。
   - `FilesUsage` の型と `zero()` に足す。検査(`app/shared/perf.ts` の `files.engine`)は、新しい鍵を任意にする(古い記録に無いのは正しい。在る時は、数と、バケットの 4 つの鍵だけを許す)。
3. ビーコンの検査: `app/shared/perf.ts` の `beaconProblems` に、`engine.exit.stop` と `engine.exit.evidence` の検査を足す(既存の `engine.exit.message` と同じ検査: 制御文字・ホームのパス・メール・URL を含まない。字数の上限)。
4. 大きさ: `stop` と `evidence` の合計は 1,200 バイト以下。`app/src/perf/beacon.ts` の `beaconText` の縮める段に、最後の段として `stop` と `evidence` を捨てる段を足す(記録が `PERF_MAX_BODY`(16 KB)を超えて、サーバーに 413 で断られないこと)。
5. 読み手: `tools/perf-beacons/summary.mjs --exits` の表に、`stop` の `lastFle`・`stopLine`・`abortKind`・`memoryBytes` と `evidence` の列を足す(短い形。既存の `scrubWords` を通す)。`files.engine` の新しい数は、セッションごとの表に出す。`--exits` に `--now <ISO 時刻>` を足す(試験のため。既定は現在時刻。48 時間の窓の終わり)。
6. 試験(vitest。`app/test/unit/` の既存の流儀):
   - `exit-message.test.ts`: 合成の stderr の列で、`stop` と `evidence` が取れること。名前・パスを含む `fle:` の行で、名前とパスが出ないこと。`stop` の無い列(`== stop:` が無い)では `stop` が無いこと。
   - `perf-beacon.test.ts`: `stop` と `evidence` が任意であること。古い記録(項目が無い)を読めること。大きさの段で、記録が 16 KB を超えないこと。
   - `files-usage.test.ts`: 答えの種類(demo・flp・バケット)が数えられること。ビーコンの JSON に、答えのファイル名の文字列が含まれないこと。
   - 合成の記録: `tools/perf-beacons/fixtures/stop/` に 1 件以上(JSON。`{ t, remote, perf }` の形。時刻は 2026-11-24 の UTC。`fle: StopFixture: …` の行を含む。個人の値を含まない)。実機の記録は commit しない。
7. 実機の確かめ(この Issue の範囲): ビルドと起動確認と `tools/smoke/welcome-flow.mjs` の流れ A(起動の値が悪化しないこと)。停止の項目が実機で出ることは、この Issue では確かめられない。報告に、その旨を書く(次に停止した実機の記録で確かめる)。

## 受け入れ

```accept
npm test --prefix app && node tools/perf-beacons/summary.mjs tools/perf-beacons/fixtures/stop --exits --now 2026-11-24T08:00:00Z | grep -q 'StopFixture' && bash guard/scan.sh && npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > .tmp-boot.json && node -e 'const r=JSON.parse(require("fs").readFileSync(".tmp-boot.json","utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false?0:1)' && node tools/smoke/welcome-flow.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > .tmp-flow.jsonl && node -e 'const L=require("fs").readFileSync(".tmp-flow.jsonl","utf8").trim().split("\n").map((l)=>JSON.parse(l));const A=L.find((r)=>r.flow==="A");const ok=!!A&&A.error===null&&A.state==="engine-idle"&&A.standIn===false&&A.trapped===false&&A.hostErrors===0&&A.opened===true&&A.elapsedMs<=12500&&A.preBootBytes<=314342986;process.exit(ok?0:1)'; rc=$?; rm -f .tmp-boot.json .tmp-flow.jsonl; exit $rc
```

判定の基準:
- 受け入れの 1 行が通る(単体試験・fixture の表に `StopFixture` が出る・`guard/scan.sh` が 0 件・ビルド・起動確認・流れ A)。
- 流れ A の行: `error` が null・`state` が engine-idle・`standIn` が false・`trapped` が false・`hostErrors` が 0・`opened` が true。
- 流れ A の `elapsedMs` が 12,500 以下(絶対値)。
- 流れ A の `preBootBytes` が 314,342,986 以下。基準は 314,326,602 B(#1355 の実測。`pm-work/findings/browser-panel-1355.md`)+ 16,384 B。
- 報告に、停止の項目が実機で出たかどうかは書かない(この Issue では停止が起きない)。確かめられていないと書く。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には「runner の値」と書き、nproc と loadavg を添える。
- 実測と推定を分ける。「達成」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。出力の行を付ける。
- 報告に書く: 項目の形(`engine.exit.stop`・`engine.exit.evidence`・`files.engine` の新しい鍵)、停止の報告の中で捨てられていた行の一覧(この Issue で取り出すようにした行)、流れ A の JSON の 1 行、`elapsedMs` と `preBootBytes` の値。
- 個人を特定できる値(名前・パス・ファイル名・sid・IP・ユーザーエージェントの全文)は、報告にも、fixture にも、findings にも書かない。

## 制約

- 製品に、新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足さない。項目は任意で、既定で全部取る。
- `BEACON_VERSION` を上げない。項目は任意の追加。
- 停止そのものを直さない(計測だけ)。
- 答えのファイル名・パス・個人の名前を、どの項目にも入れない。
- ベンダー名と Cloudflare の識別子を書かない。内部の ID(`L-0` で始まる語)と URL を書かない。
- Python は製品に使わない(調査と検証にだけ使う)。
- sleep・timeout・nohup・末尾の `&` は使わない。一時ファイルは `.tmp-*` に置き、終わったら消す。
- 製品の作業は Actions で行う。この機械で製品のビルドや測定をしない。

## 作業の進め方

- 最初に `date -u` で開始の時刻を記録する。4 時間 30 分を過ぎたら、受け入れが通っていなくても、作業ツリーを commit し、報告を書いて終える(ジョブの上限は 5 時間 50 分)。
- engine のビルドは約 35 分。フォアグラウンドで直接実行し、シェルツールの `timeout` 引数は最大の 5400000 にする。背景で起動して待たない。
- `tools/smoke/welcome-flow.mjs` は流れを 4 つ(A・B・C・D)すべて走らせて 4 行を出す。判定に使うのは流れ A の行だけ。
- blend2d の取得(GitHub)が 403 で落ちたら一時的。受け入れのコマンドは `lift/out-app/b2d-src` を消してから作り直すので、そのまま再試行する。
- 作業の最後に、受け入れが通らなくても、作業ツリーを commit する。

## 他のタスクとの関係

- blocked_by: なし。
- 結果は、#{{121}}(#1365 LockResource)・#{{128}}(#1372 ExitProcess)の原因の確かめに使う。この Issue は、それらを直さない。
- #{{133}}(#1377。セッションの表の道具)の後の変更。`tools/perf-beacons/summary.mjs` の表を広げる。
- `app/src/main.ts`・`app/src/files/`・`app/src/perf/` は、ほかの作業と衝突しうる。衝突したら、作業の最後に `git pull --no-rebase --no-edit` で main を取り込み、衝突を直し、受け入れをもう一度通す。
