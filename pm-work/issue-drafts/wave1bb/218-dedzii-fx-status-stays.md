---
draft: "218"
title: "DEDZII を開いた後、ファイル管理の状態表示(fx-status)が 10 秒を超えて残る: 残る条件を特定して直す"
type: Task
milestone: "M5 内蔵のエフェクトとインストゥルメント"
parent: "018"
blocked_by: ["165"]
labels: ["verify"]
size: S
priority: Medium
project: signature-studio/3
---

## 目的

E2E のシナリオ `file-open-demo`(smoke。File > Open で評価用の曲 DEDZII を開き、engine が止まらず、プロジェクトが開くことを見る)が、表示によって失敗する。失敗の中身は、ファイル管理の窓の状態表示 `fx-status`(`role="status"`)が、10 秒を超えて文字を残したまま変わらないこと(E2E の `stuckToasts` が 1)。

この Issue の目標は次の 3 つ。
1. 状態表示の文字を作る箇所と、消える条件(タイマー・完了の通知)を特定する。
2. DEDZII を開いた後に、状態表示が古い文字を残さないように直す。エラーの表示(`data-error="1"`)は、今のまま残す。
3. `file-open-demo` が、3 表示(4k-dpr1・4k-dpr2・4k-random)の全ての行で、印なしに ok で、`stuckToasts` が 0 になる。

## 背景

- `file-open-demo` の印は、main で既に無い(#1421 が閉じた後に外した。`tools/e2e/scenarios/smoke/file-open-demo.mjs`)。その状態で、`stuckToasts` が 1 の行が出ている。
- 実測(#1483 の作業の run の smoke の JSON。runner の値): `file-open-demo` の `stuckToasts` が、4k-dpr1(DPR 1)で 1、4k-dpr2(DPR 2)で 1、4k-random(DPR 1.69)で 1。4k-random(DPR 2.47)では 0。別の run(#1419 の作業)の 4k-dpr1 では 0。表示の DPR だけでは決まらず、時間で残る表示があるらしい(推定。未確認)。
- 検出の定義(`tools/e2e/lib/detect.mjs`): `stuckToasts` は、`role="status"`・`role="alert"`・`aria-live` の要素のうち、可視で文字を持ち、10 秒を超えて変わらない物の数。
- 状態表示の要素: `app/src/files/explorer.ts` の `fx-status`(`id="fx-status"`・`role="status"`。`app/src/files/files.css` の `.fx-status`・`[data-error="1"]`)。この要素が、`stuckToasts` の数えられた物だと推定する(未確認。最初に確かめる)。
- `file-open-demo` の手順: Welcome を閉じて既定の曲を開く → 既定の曲の「Problems loading」の箱を閉じる → File > Open(Ctrl+O)でファイル管理の窓へ DEDZII をアップロードして選ぶ → Open を押す。`tools/e2e/lib/flow.mjs` の `openProjectFile`。
- 過去の関連: #1421(DEDZII の読み込みで engine が止まる。閉じている)。止まる件は直っている。この Issue は、止まらない後の表示の件。

## 読む物

- `app/src/files/explorer.ts`(`fx-status` の作り方・書き方・消す箇所。`el('div', { class: 'fx-status', id: 'fx-status', role: 'status' })` の付近)、`app/src/files/files.css`
- `app/src/files/select.ts`(開く・保存の窓の橋)、`app/src/files/engine-dialog.ts`(DEDZII を開く経路で、状態を書く箇所)、`app/src/ui/filedlg.ts`(`QUICK_CANCEL_MS`)
- `tools/e2e/lib/detect.mjs`(`toastState`・`installToastWatch`・`stuckToasts` の定義)、`tools/e2e/lib/flow.mjs`(`openProjectFile`)
- `tools/e2e/scenarios/smoke/file-open-demo.mjs`(この Issue の対象)

## やること

0. 作業の最初に `date -u` を記録する。`.prior-work.md` が在れば最初に読む。
1. **表示の特定**: `file-open-demo` を 3 表示で走らせ、`stuckToasts` が 1 の行の、残っている要素の id・文字・`data-error`・残り始めた時刻を、JSON か trace で記録する。残っているのが `fx-status` かどうかを、この記録で確かめる。
2. **原因の特定**: 文字を書く箇所(開く・読み込みの途中の表示)と、消す条件(タイマー・完了の通知・窓を閉じる操作)を、コードと記録で読む。DEDZII を開いた後に消えない経路を、記録で確かめる(推定で直さない)。
3. **直す**: プロジェクトが開いた後(または読み込みが終わった後)に、古い状態の文字を消す。エラーの表示(`data-error="1"`)は残す。ファイル管理の窓の動作(開く・保存・エラーの表示)を変えない。
4. **E2E**: `file-open-demo` の 3 表示の全ての行で、`ok` が真・`stuckToasts` が 0。印は付けない。

## 受け入れ

```accept
npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false&&r.elapsedMs<=12500?0:1)' "$RUNNER_TEMP/boot.json" && node tools/smoke/welcome-flow.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/flow.jsonl" && node -e 'const fs=require("fs");const L=fs.readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const F=(f)=>L.find((r)=>r.flow===f);const ok=(r)=>!!r&&r.error===null&&r.state==="engine-idle"&&r.standIn===false&&r.trapped===false&&r.hostErrors===0&&r.opened===true;process.exit(L.length===4&&["A","B","C","D"].every((f)=>ok(F(f)))&&F("B").welcomeClosed===true&&F("D").welcomeClosed===true?0:1)' "$RUNNER_TEMP/flow.jsonl" && npm test --prefix app && (cd tools/e2e && npm ci && npx playwright install chromium) && (node tools/e2e/run.mjs --scenarios smoke --only file-open-demo --displays 4k-dpr1,4k-dpr2,4k-random > "$RUNNER_TEMP/e2e.jsonl") && node -e 'const fs=require("fs");const L=fs.readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const good=(r)=>{const s=r.scenarios.find((x)=>x.name==="file-open-demo");return r.servedHeadersMatch===true&&r.error===null&&!!s&&s.expectFail===null&&s.ok===true&&s.pageErrors===0&&s.consoleErrors===0&&s.failedRequests===0&&s.stuckToasts===0;};process.exit(L.length===4&&L.every(good)?0:1)' "$RUNNER_TEMP/e2e.jsonl" && bash guard/scan.sh
```

## 判定の基準

- 起動確認(`boot.json`)が engine-idle・standIn が false・elapsedMs が 12,500 以下。
- 流れ A〜D の 4 行が、`error` が null・engine-idle・`standIn` が false・`trapped` が false・`hostErrors` が 0・`opened` が true。B と D は `welcomeClosed` が true。
- `npm test --prefix app` が通る。
- `file-open-demo` が、3 表示の全ての行で、印なし・`ok` が真・`stuckToasts` が 0・`pageErrors`・`consoleErrors`・`failedRequests` が 0。行の数は 4(4k-dpr1 が 1、4k-dpr2 が 1、4k-random が 2)。
- `bash guard/scan.sh` が 0 件。
- 報告に、残っていた要素の特定(id・文字・時刻)、原因の記録、直した箇所、`file-open-demo` の 4 行の JSON を書く。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には「runner の値。この機械とは別枠」と書き、nproc と loadavg を添える。
- 実測と推定を分ける。「達成」「直った」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。出力の行を付ける。

## 制約

- 製品に、利用者向けの新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足さない。オプトインを作らない。テストのための分岐を製品に入れない。
- 判定を緩めない(`stuckToasts` の数え方を変えない。表示を隠して 0 にしない)。
- `app/src/files/select.ts`(開く・保存の窓の橋)の動作を変えない。
- `tools/e2e/judge.jq` と `tools/e2e/run.mjs` の `rowPasses` は変えない。
- Python と OCR を使わない。禁止されているのは、AI のベンダー名と Cloudflare の識別子だけ。FL の製品名・インストール先の名前・変数名は、書いてよい。
- sleep・timeout のコマンド・nohup・末尾の `&` は使わない。待つ時は条件で待つ。一時ファイルは `.tmp-*` に置き、終わったら消す。
- 製品の作業(ビルド・測定)は Actions で行う。この機械では行わない。

## 作業の進め方

- 作業の最初に `date -u` を記録する。4 時間 30 分を過ぎたら、受け入れが通らなくても、作業ツリーを commit し、報告を書いて終える(ジョブの上限は 5 時間 50 分)。
- ビルドは約 35 分。フォアグラウンドで直接、シェルツールの `timeout` 引数は最大 5400000(90 分)にする。背景で起動して待たない。
- 受け入れが通らなくても、作業の最後に作業ツリーを commit する。未 commit の変更が残ると、判定は失敗する。

## 他のタスクとの関係

- blocked_by: #1408(E2E のシナリオ集。E2E の印の規則と、smoke の印の一覧を #1408 が変える。その版の上で、この Issue の受け入れを通す)。
- #1421(DEDZII の読み込みで engine が止まる。閉じている)の後の表示の件。
