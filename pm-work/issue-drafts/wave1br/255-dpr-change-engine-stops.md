---
draft: "255"
title: "DPR を実行中に 1.8 から 1.75 へ変えると、engine が終了コード 5 で止まる: 再現して、一度目の停止の原因を特定して直す"
type: Task
milestone: "M2 HiDPI と P2 の測定(P2)"
parent: "005"
blocked_by: []
labels: ["hidpi", "verify"]
size: M
priority: High
project: signature-studio/3
---

## 目的

HiDPI の方針: 拡大率(DPR)は実行時に取る。ウィンドウを別のモニタへ動かす・ブラウザの拡大率を変える、などで DPR が変わっても、画面が崩れず、engine が止まらない。

この Issue の目的: E2E `page/dpr-change-keeps-drawing` の途中で、engine が止まる件を、再現して、原因の場所で直す。止まりの原因が E2E の操作の誤りだと分かった場合は、E2E の操作を直す(製品は変えない)。

## 背景

- 作業者の報告(#1509)の要旨: 基準の DPR 1.8(報告の表記)の行で、DPR を 1.75 へ変えた時(`page/dpr-change-keeps-drawing`)、engine が終了コード 5 で止まった。ログは残っていない。報告の文言は「DPR 1.8 でエンジンが exit code 5 で止まる(製品の候補、未確認)」。
- 実測(同じシナリオの別の run の判定の出力): 基準 DPR 1・1.84・2.89 から 1.75 へ変えた時は、いずれも ok(engine-idle・止まり 0)。止まったのは、報告の 1 回だけ。間欠か、基準の DPR に依るかは、確認できない。
- シナリオの操作(`tools/e2e/lib/page.mjs` の `setRatio`): CDP の `Emulation.setDeviceMetricsOverride` で、物理の窓の大きさを保ったまま、DPR を変える。
- 終了コード 5 の意味(読みで確かめた): `lift/harness/fruity.c` の `report()` の先頭 `if (in_report) _exit(5);`。停止の報告(`report()`)の最中に、もう一度 abort が起きた時の値。`lift/runtime/fle_thread.c` の mutex の wrapper は、「同じ guest スレッドが保持する libc の mutex を取った」時に `fle: pthread_mutex_lock: ...` の行を出して停止する。したがって、報告の最後の `fle:` の行は、二度目の停止のものかもしれない(推定)。一度目の原因は、ログを取らないと分からない。同じ型の停止は、DPR と無関係の `file/export-last-format` でも出た(#1529 の系統 D)。
- DPR の変化の経路(製品): `app/src/main.ts` の `onDevicePixelRatio`(357 行付近)→ `applyDisplay`。`followRoom`(391 行付近)が、guest 画面の大きさが変わった時に `sendRaw({ kind: IN.DISPLAY })` を送る → `lift/runtime/fle_input.c` の `apply_display`(777 行付近): 全トップレベル窓へ `WM_DISPLAYCHANGE` と `WM_SETTINGCHANGE` を送る。
- 再現の前提: このシナリオの宣言は既定値 `['4k-dpr1', '4k-random']`(`tools/e2e/lib/scenario.mjs` の 4 行と 86 行)。`--displays` は宣言を広げないので、`--displays dpr:1.8` は、宣言に `dpr:1.8` を足すまで走らない。
- 詳細: `pm-work/findings/dpr-change-stop.md`(読んだ行つき。実測と推定を分けてある)。

## 読む物

- `tools/e2e/scenarios/page/dpr-change-keeps-drawing.mjs`(宣言と `dprMoveCase(ctx, 1.75)`)
- `tools/e2e/lib/page.mjs`: `setRatio`(107 行付近)・`waitSettled`(126 行付近)・`menuChangesPicture`(230 行付近)・`dprMoveCase`(288 行付近)
- `tools/e2e/lib/scenario.mjs`(4 行の既定の宣言と 86 行)・`tools/e2e/lib/plan.mjs`(`displayRows`・`rowsFor`)・`tools/e2e/run.mjs`(先頭の注釈: `--displays`・`--engine-logs`・`--report-only`)
- `app/src/main.ts`(305〜404 行: `measureRoom`・`applyDisplay`・`onDevicePixelRatio`・`followRoom`)・`app/src/ui/screen-fit.ts`(`chooseGuestScreen`、47 行付近)
- `lift/runtime/fle_input.c`(`fle_input_host_display` 191 行付近・`apply_display` 777 行付近)・`lift/runtime/fle_screen.h`(`fle_screen_set`、84 行付近)
- `lift/runtime/fle_thread.c`(`__wrap_pthread_mutex_lock`、939〜980 行付近)・`lift/runtime/fle_env.c`(`fle_stop`、66 行付近)
- `lift/harness/fruity.c`(`report()`、645〜652 行付近・`on_abort_wasm`、787 行付近)
- `pm-work/findings/dpr-change-stop.md`

## やること

0. 作業の最初に `date -u` を記録する。`.prior-work.md` が在れば、最初に読む。
1. **再現(最初の段)**:
   a. `tools/e2e/scenarios/page/dpr-change-keeps-drawing.mjs` の宣言に `'dpr:1.8'` を足す(`displays: ['4k-dpr1', '4k-random', 'dpr:1.8']`)。`--displays` は宣言を広げないため、この 1 行が要る。
   b. ビルド(下の受け入れの最初の段と同じ)の後、フォアグラウンドで: `node tools/e2e/run.mjs --only page/dpr-change-keeps-drawing --displays dpr:1.8 --repeat 3 --engine-logs "$PWD/.tmp-dpr18" --budget 30m --report-only > "$PWD/.tmp-dpr18/rows.jsonl"`。
   c. 止まったら、そのログ(`.tmp-dpr18/` の中)の `fle:` の行を、順序どおりに全部、報告に貼る(一度目と二度目の停止を分ける)。止まらなければ、止まらなかった回数を書く。`4k-random` の行も、同じ要領で 3 回ほど走らせて、止まりを探す。
   d. 止まりが再現しなければ、その事実を報告に書き、「確認できない」と書いて、次の段の読みで原因を探す。
2. **原因の場所**: 一度目の停止の `fle:` の行と、その直前の呼び出し(戻り番地は `lift/out-app/gen/wasm-func-names.tsv` で引く)。停止が `pthread_mutex_lock` なら、その mutex を解放しないまま戻った経路を特定する。報告中の再入(二度目)と、一度目の原因を分けて書く。
3. **直す**: 原因の場所で直す(製品)。原因が E2E の操作だと分かったら(例: DPR の変え方が、実機の移動と違い、engine に通常届かない入力を届けている)、E2E の操作を直す(製品は変えない)。どちらの場合も、根拠を報告に書く。
4. **受け入れ**: 下の式を通す。宣言に足した `dpr:1.8` の行は残す(この行で止まりが再現した件の回帰の確認)。

## 受け入れ

```accept
npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false&&r.elapsedMs<=20000?0:1)' "$RUNNER_TEMP/boot.json" && node tools/smoke/welcome-flow.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/flow.jsonl" && node -e 'const fs=require("fs");const L=fs.readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const F=(f)=>L.find((r)=>r.flow===f);const ok=(r)=>!!r&&r.error===null&&r.state==="engine-idle"&&r.standIn===false&&r.trapped===false&&r.hostErrors===0&&r.opened===true;process.exit(L.length===4&&["A","B","C","D"].every((f)=>ok(F(f)))&&F("B").welcomeClosed===true&&F("D").welcomeClosed===true?0:1)' "$RUNNER_TEMP/flow.jsonl" && npm test --prefix app && (cd tools/e2e && npm ci && npx playwright install chromium) && node --test tools/e2e/test/*.test.mjs && node tools/e2e/run.mjs --only page/dpr-change-keeps-drawing --displays 4k-dpr1,dpr:1.8,4k-random --budget 40m --report-only > "$RUNNER_TEMP/dpr.jsonl" && node -e 'const L=require("fs").readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const S=L.flatMap((r)=>r.scenarios);process.exit(L.length===3&&L.every((r)=>r.error===null)&&S.length===3&&S.every((s)=>s.id==="page/dpr-change-keeps-drawing"&&s.ok===true&&s.error===null&&s.skipped!==true&&s.engineState==="engine-idle"&&s.engineStops===0&&s.engineStuckRunning!==true&&s.dprOk!==false)?0:1)' "$RUNNER_TEMP/dpr.jsonl" && bash guard/scan.sh
```

判定の基準:
- 起動確認(`boot.json`)が engine-idle・standIn が false・elapsedMs が 20,000 以下。
- 流れ A〜D の 4 行が、`error` が null・engine-idle・`standIn` が false・`trapped` が false・`hostErrors` が 0・`opened` が true。B と D は `welcomeClosed` が true。
- `npm test --prefix app` と `node --test tools/e2e/test/*.test.mjs` が通る。
- `page/dpr-change-keeps-drawing` が、3 行(4k-dpr1・dpr:1.8・4k-random)すべてで、`ok` が true・`error` が null・`engineState` が engine-idle・`engineStops` が 0・`engineStuckRunning` が true でない・`dprOk` が false でない。
- `bash guard/scan.sh` の指摘が 0 件。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 報告に書く: (a) 再現の結果(run の回数、止まった回数、止まった時の `fle:` の行の全部)、(b) 一度目の停止の場所(呼び出し元・戻り番地・停止の型)と、報告中の再入の有無、(c) 直した所と、その理由(製品か E2E か)、(d) 受け入れの各判定の出力の行、(e) 起動確認の `elapsedMs`。
- 「達成」「直った」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。出力の行を付ける。確認できなかった物は「確認できない」と書く。実測と推定を分けて書く。

## 制約

- 製品に、利用者向けの新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足さない。テストのための分岐を製品に入れない。
- DPR の値(拡大率の具体の数値)を、コードに固定しない。実行時に取る(依頼者の方針)。E2E の宣言の `dpr:1.8` は、E2E の行の指定であり、製品の値ではない。
- 止まりを隠す直し方(例: 例外を握りつぶす・判定を外す・停止の報告を消す)はしない。
- Python を使わない(調査と検証にだけ使う)。OCR を使わない。
- sleep・nohup・末尾の `&`・先頭の `timeout` は使わない。待つ時は条件で待つ。
- 一時ファイルは `.tmp-*` に置き、終わったら消す。
- `git add` は、変えたファイルを 1 つずつ指定する(`git add -A`・`git add .` は使わない)。
- commit・PR に、AI が書いたことを示す行を付けない。commit の名義は、環境の設定のまま。

## 作業の進め方

- 作業の最初に `date -u` を記録する。3 時間 15 分を過ぎたら、受け入れが通らなくても、作業ツリーを commit し、報告を書いて終える(ジョブの上限は 5 時間 50 分)。
- ビルドは 1 回にして、同じ bundle で繰り返し確かめて直す。長いコマンドはフォアグラウンドで直接、シェルツールの `timeout` 引数は最大 5400000(90 分)にする。背景で起動して待たない。
- 受け入れが通らなくても、作業の最後に作業ツリーを commit する。未 commit の変更が残ると、判定は失敗する。

## 他のタスクとの関係

- blocked_by: なし。
- 関連(blocked_by ではない): #1529(系統 D: `file/export-last-format` の終了 5・同じ mutex の型)。#1496(visual の DPR 2.06 の停止)。#1486(page 領域の E2E)。
