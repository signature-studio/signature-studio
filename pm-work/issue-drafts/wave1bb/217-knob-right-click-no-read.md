---
draft: "217"
title: "ツマミの右クリックのメニューを開くと、許可済みのクリップボードを読んでしまう: メニューを開く時は読まず、貼り付けを選んだ時だけ読む"
type: Task
milestone: "M3 トップバーのコンテキストメニュー(P2)"
parent: "138"
blocked_by: ["165"]
labels: ["clipboard", "verify"]
size: M
priority: Medium
project: signature-studio/3
---

## 目的

#1406 で決めた規則: クリップボードの問い合わせは、読み取り・権限の要求・表示を起こさない。貼り付けの操作(FL のメニューの「貼り付け」や Ctrl+V)の時だけ読む。

この規則が、ツマミ(プラグインのノブ)の右クリックで守られていないことを、E2E が見つけた。この Issue の目標は次の 3 つ。
1. ツマミの右クリックでメニューを開く時、ページは `navigator.clipboard.read()` も `navigator.clipboard.readText()` も呼ばない(権限が `prompt`・`denied`・`granted` のどれでも)。
2. FL の「貼り付け」の有効・無効の問い合わせ(形式の有無)に、クリップボードの読みを使わない。
3. メニューの「貼り付け」を選んだ時だけ読む。読むのは、そのクリックの中で、権限に従って(`prompt` は許可の確認を出す。`denied` は読まず、表示は 1 件)。

## 背景

- #1408 の E2E(regression の `clipboard-permission-knob`。2 表示)の実測: `prompt`・`denied` の右クリックでは読みは 0 回。`granted` の右クリックでは、`navigator.clipboard.read()` が 1 回呼ばれ、`readText()` は 0 回だった(権限の表示は出ない)。
- ただし、この実測は #1482(commit `0ae6a852`。クリップボードを Clipboard API だけで行う変更)より前の main で取った。現在の `app/src/engine/clipboard.ts` には、`navigator.clipboard.read()` を呼ぶ箇所が無い。この版で、右クリックの間に読みが起きるかは、まだ測っていない。最初に測る(下の「やること」の 0)。
- 現在の版のコード上の読みの経路(`app/src/engine/clipboard.ts`。コードから読んだ。実測は未確認):
  - 起動の時(`start()`): 権限が `prompt` か `granted` の時、`readText()` を呼ぶ。
  - 最初の押下・キー(`firstAction()`): 起動の要求が拒否されていた(`askAgain`)時、権限が `prompt` なら、もう一度 `readText()` を呼ぶ。押下は右でも左でもありうる。右クリックでも読みが起きうる経路。
  - ウィンドウの focus(`onFocus()`): 権限が `granted` の時、`readText()` を呼ぶ。
  - 形式の問い合わせ(`probe()`): 読まない(知っている文字列と権限の状態だけで答える)。
  - FL の `GetClipboardData` が右クリックの直後に来た時(`isQuestion()`): 読まず、知っている文字列か、数値の占め字(`PLACEHOLDER`)を返す。`denied` では何も返さない。
- 権限の状態は、`app/src/engine/clipboard.ts` の `permissionState()`(`navigator.permissions.query`)で得る。この問い合わせは読みではない。
- FL の「貼り付け」の有効・無効は、右クリックの時に、クリップボードの文字列が数値として解釈できるかで決まる(#1406 の記録。文字列の内容は数字にする)。
- 表示(トースト)は #1482 で無くした。表示が出ないことは、`clipboard-no-toast`(smoke)と `clipboard-permission-knob`(regression)で見る。
- 既知の限界(#1406 の記録): `granted` の時の起動・focus の読みは、規則の文字どおりには「貼り付けの操作の時だけ」ではない。右クリックの間に起きないなら、その扱いは依頼者の判断に残し、報告に書く。

## 読む物

- `app/src/engine/clipboard.ts`(全体。`attach`・`start`・`firstAction`・`onFocus`・`probe`・`read`・`isQuestion`・`readKnown`)
- `app/src/main.ts`(`clipboardRead`・`clipboardProbe` の handler と、`pageClipboard` の初期化)
- `app/src/engine/client.ts`・`app/src/engine/worker.ts`・`app/src/engine/protocol.ts`(問い合わせの経路)
- `lift/runtime/fle_clip.c`・`lift/runtime/fle_clip.h`・`lift/harness/web/clip/fle_clip_lib.js`(`IsClipboardFormatAvailable`・`CountClipboardFormats`・`EnumClipboardFormats`・`GetClipboardData` の、今の応答と、ページへの問い合わせの経路)
- `pm-work/findings/clipboard-1406.md`(#1406 の記録)
- `tools/clipboard/measure.mjs`・`tools/e2e/lib/detect.mjs`(`clipboardReads`・`countClipboardReads`・`permissionStates`・`toastState`)
- `tools/e2e/scenarios/smoke/knob-right-click-permissions.mjs`・`tools/e2e/scenarios/smoke/clipboard-no-toast.mjs`
- `tools/e2e/scenarios/regression/clipboard-permission-knob`(#1408 が作る版。`r06-clipboard-permission-knob.mjs`)

## やること

0. 作業の最初に `date -u` を記録する。`.prior-work.md` が在れば最初に読む。最初の受け入れの run の JSON で、`smoke` の `knob-right-click-permissions` と `regression` の `clipboard-permission-knob` の、権限ごとの読みの回数(`read()`・`readText()` を分けた数)を見る。
1. **読みを起こす箇所を特定する**: 上の 4 つの経路(`start`・`firstAction`・`onFocus`・`read` の問い合わせ側)のどれが、右クリックの間の読みを起こすかを、記録で確かめる。推定で直さない。
2. **直す**:
   - 右クリックの押下では、読み取りも権限の要求も行わない。`firstAction` の「もう一度」の要求は、右の押下では行わない(左の押下かキー入力の時に限る)。
   - FL の形式の問い合わせ(`probe`・`isQuestion` の経路)は、読まない。既存の動きを保つ。
   - メニューの「貼り付け」を選んだ時(`GetClipboardData` で、右クリックの問い合わせでない時)だけ、`readText()` を呼ぶ。`prompt` は許可の確認が出る。`denied` は読まず、理由の表示を 1 件出す。`granted` は読む。
   - Ctrl+V の貼り付けは、既存の経路(`paste` イベントの内容を使う)を変えない。
3. **E2E**: smoke の `knob-right-click-permissions` と、regression の `clipboard-permission-knob` が、印なしで ok になること。smoke の `knob-right-click-permissions` は main で印が無い(#1482 で外した)。その ok を、この Issue の作業で確かめる。ok にならない時は、この Issue の範囲で直す。
4. 受け入れを通す。通らなければ、作業ツリーを commit して、報告を書いて終える。

## 受け入れ

```accept
npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false&&r.elapsedMs<=12500?0:1)' "$RUNNER_TEMP/boot.json" && node tools/smoke/welcome-flow.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/flow.jsonl" && node -e 'const fs=require("fs");const L=fs.readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const F=(f)=>L.find((r)=>r.flow===f);const ok=(r)=>!!r&&r.error===null&&r.state==="engine-idle"&&r.standIn===false&&r.trapped===false&&r.hostErrors===0&&r.opened===true;process.exit(L.length===4&&["A","B","C","D"].every((f)=>ok(F(f)))&&F("B").welcomeClosed===true&&F("D").welcomeClosed===true?0:1)' "$RUNNER_TEMP/flow.jsonl" && npm test --prefix app && (cd tools/e2e && npm ci && npx playwright install chromium) && (node tools/e2e/run.mjs --scenarios smoke --only knob-right-click-permissions --displays 4k-dpr1,4k-dpr2 > "$RUNNER_TEMP/e2e-smoke.jsonl") && (node tools/e2e/run.mjs --scenarios regression --only clipboard-permission-knob --displays 4k-dpr1,4k-dpr2 > "$RUNNER_TEMP/e2e-reg.jsonl") && node -e 'const fs=require("fs");const L=[];for(const f of process.argv.slice(1))for(const l of fs.readFileSync(f,"utf8").trim().split("\n"))L.push(JSON.parse(l));const good=(r,n)=>{const s=r.scenarios.find((x)=>x.name===n);return r.servedHeadersMatch===true&&r.error===null&&!!s&&s.expectFail===null&&s.ok===true&&s.pageErrors===0&&s.consoleErrors===0&&s.failedRequests===0&&s.stuckToasts===0;};process.exit(L.length===4&&L.slice(0,2).every((r)=>good(r,"knob-right-click-permissions"))&&L.slice(2).every((r)=>good(r,"clipboard-permission-knob"))?0:1)' "$RUNNER_TEMP/e2e-smoke.jsonl" "$RUNNER_TEMP/e2e-reg.jsonl" && bash guard/scan.sh
```

## 判定の基準

- 起動確認(`boot.json`)が engine-idle・standIn が false・elapsedMs が 12,500 以下。
- 流れ A〜D の 4 行が、`error` が null・engine-idle・`standIn` が false・`trapped` が false・`hostErrors` が 0・`opened` が true。B と D は `welcomeClosed` が true。
- `npm test --prefix app` が通る。
- smoke の `knob-right-click-permissions` と regression の `clipboard-permission-knob` が、2 表示(4k-dpr1・4k-dpr2)の全ての行で、印なし(`expectFail` が null)・`ok` が真・`pageErrors`・`consoleErrors`・`failedRequests`・`stuckToasts` が 0。
- `bash guard/scan.sh` が 0 件。
- 報告に、権限ごと(`prompt`・`denied`・`granted`)の右クリックの読みの回数(`read()`・`readText()` を分けて)、読みを起こしていた経路の特定(上の「やること」の 1)、`granted` の起動・focus の読みを残したかの判断を書く。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には「runner の値。この機械とは別枠」と書き、nproc と loadavg を添える。
- 実測と推定を分ける。「達成」「直った」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。出力の行を付ける。
- 報告に書く: 最初の run の読みの回数、読みの経路の特定、直した箇所、流れ A〜D の 4 行、smoke と regression の 4 行。

## 制約

- 製品に、利用者向けの新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足さない。オプトインを作らない。テストのための分岐を製品に入れない。
- 表示(トースト)を、問い合わせの時に出さない。貼り付けの結果としてだけ出す(#1406 の規則)。
- `app/src/files/select.ts`(開く・保存の窓の橋)と、Copy の経路(`writeText`。#1389)の動作を変えない。
- `tools/e2e/judge.jq` と `tools/e2e/run.mjs` の `rowPasses` は変えない。
- Python と OCR を使わない。禁止されているのは、AI のベンダー名と Cloudflare の識別子だけ。FL の製品名・インストール先の名前・変数名は、書いてよい。
- sleep・timeout のコマンド・nohup・末尾の `&` は使わない。一時ファイルは `.tmp-*` に置き、終わったら消す。
- 製品の作業(ビルド・測定)は Actions で行う。この機械では行わない。

## 作業の進め方

- 作業の最初に `date -u` を記録する。4 時間 30 分を過ぎたら、受け入れが通らなくても、作業ツリーを commit し、報告を書いて終える(ジョブの上限は 5 時間 50 分)。
- ビルドは約 35 分。フォアグラウンドで直接、シェルツールの `timeout` 引数は最大 5400000(90 分)にする。背景で起動して待たない。
- 受け入れが通らなくても、作業の最後に作業ツリーを commit する。未 commit の変更が残ると、判定は失敗する。

## 他のタスクとの関係

- blocked_by: #1408(regression の `clipboard-permission-knob` を作る。main に在る必要がある)。
- #1406(親の規則。閉じている)の続き。#1482(Clipboard API だけで行う変更。閉じている)の後の版を土台にする。
- #1389(Copy の経路)を壊さない。
