---
draft: "191"
title: "F10 の Settings の General タブをスクロールすると、DPR 1 で、窓の外の 683 画素が変わる(#1384 の直しが DPR 1 で不完全)"
type: Task
milestone: "M1 描画と音声の分離(P3)"
parent: "001"
blocked_by: ["165"]
labels: ["verify"]
size: S
priority: High
project: signature-studio/3
---

## 目的

F10 の Settings の General タブを開いて下へスクロールすると、DPR 1 では、窓の外の画素が 683 px 変わる。DPR 2 は通る。#1384(Settings の General のスクロールで描画が崩れる。閉じている)は、DPR 2 で確かめられた直しで、DPR 1 では不完全だった。この Issue は、窓の外が変わる原因を特定し、DPR 1 でも直す。

この Issue の目標は次の 3 つ。
1. DPR 1 で、Settings の General をスクロールしても、窓の外の画素が変わらない。
2. DPR 2 で、今の結果(窓の外が変わらない)を保つ。
3. 窓の外が変わる原因を、実測で特定し、報告に書く。

## 前回の試行で分かったこと(#1408 の run 45594361647)

数値は runner の値(nproc 4、loadavg 2.10 から 3.13)。
- R13 `settings-general-scroll` の結果: DPR 1 は窓の外が 683 px 変わる。DPR 2 は通った。
- 1 つ前の試行(run 45548619833)では、下へのスクロールで 696 px 動き、DPR 2 では 26 px が窓の外に出た。今回の DPR 2 では 0。DPR 1 は前回も今回も窓の外が変わる。
- R13 は、スクロールの前後の画素を比べ、窓の外の差を数える。窓の外の差が 0 でなければ落ちる。
- 窓の外の物として、カーソルとツールチップが考えられる。ブラウザの canvas の外の物(カーソル・ツールチップ)が画素の比較に入っていないかを、まず確かめる。
- #1392(FL 自身のカーソル。閉じている)の画像カーソルが、窓の外の画素に写っていないかも確かめる。

## 背景

- #1384(F10 の Settings の General タブをスクロールすると、描画が崩壊する。閉じている。`pm-work/findings/settings-scroll-1384.md` の記録がある)。DPR 2 では直った。DPR 1 では、窓の外の画素が変わる。
- #1408(E2E のシナリオ集)の R13 `settings-general-scroll`: F10 の Settings の General を開き、スクロールしても描画が崩れない(画素の比較で、スクロールの前後の差が、窓の外に出ない)。R13 には、この Issue の番号の印が付く。この Issue が直ったら、R13 の印を外す。

## 読む物

- `tools/e2e/scenarios/regression/`(R13。#1408 が main に入った後に読める)
- `tools/e2e/run.mjs`・`tools/e2e/lib/`(画素の比較と、検出)
- `pm-work/findings/settings-scroll-1384.md`(#1384 の記録と、その時の画像)
- #1384 の直しの変更(`git log --oneline --grep 1384` で特定する)
- `app/src/ui/`(画面の画素を出す経路。canvas・`#desktop`)
- `lift/runtime/`(窓の再描画・クリップ・矩形の計算)

## やること

0. 作業の最初に `date -u` を記録する。`.prior-work.md` が在れば、最初に読む。
1. **再現**: `bash tools/e2e/run.sh --scenarios regression --only settings-general-scroll --displays 4k-dpr1` で、窓の外の変わった画素の位置(矩形)を記録する。DPR 2 は、同じ実行で結果を見る(`--displays 4k-dpr1,4k-dpr2`)。
2. **原因の特定**: 窓の外が変わる原因を、実測で切り分ける。候補は、スクロールの描画の矩形が窓の範囲を越える・クリップの誤り・別の窓の再描画・カーソル(#1392 の画像カーソル)の重なり。カーソルとツールチップは、画素の比較の対象か、確かめる。
3. **直す**: DPR 1 でも DPR 2 でも、窓の外が変わらない形にする。既定の動作として直す(オプトインを作らない)。
4. 報告に、変わった画素の矩形(直す前と後)、原因(実測と推定を分ける)、変えた箇所を書く。

## 受け入れ

```accept
npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false&&r.elapsedMs<=12500?0:1)' "$RUNNER_TEMP/boot.json" && node tools/smoke/welcome-flow.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/flow.jsonl" && node -e 'const fs=require("fs");const L=fs.readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const F=(f)=>L.find((r)=>r.flow===f);const ok=(r)=>!!r&&r.error===null&&r.state==="engine-idle"&&r.standIn===false&&r.trapped===false&&r.hostErrors===0&&r.opened===true;process.exit(L.length===4&&["A","B","C","D"].every((f)=>ok(F(f)))&&F("B").welcomeClosed===true&&F("D").welcomeClosed===true&&Math.max(...L.map((r)=>r.preBootBytes))<=314368711?0:1)' "$RUNNER_TEMP/flow.jsonl" && npm test --prefix app && (cd tools/e2e && npm ci && npx playwright install chromium) && bash tools/e2e/run.sh --scenarios regression --only settings-general-scroll --displays 4k-dpr1,4k-dpr2 > "$RUNNER_TEMP/e2e-r13.jsonl" && node -e 'const fs=require("fs");const L=fs.readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));process.exit(L.length===2&&L.every((r)=>r.scenarios.length===1&&r.scenarios[0].ok===true&&r.scenarios[0].pageErrors===0&&r.scenarios[0].consoleErrors===0&&r.scenarios[0].failedRequests===0)?0:1)' "$RUNNER_TEMP/e2e-r13.jsonl" && bash guard/scan.sh
```

判定の基準(全て満たす):
- 起動確認(`boot.json`)が engine-idle・standIn が false・error が null・elapsedMs が 12,500 以下。
- 流れ A〜D の 4 行が、`error` が null・engine-idle・`standIn` が false・`trapped` が false・`hostErrors` が 0・`opened` が true。B と D は `welcomeClosed` が true。
- `npm test --prefix app` が通る。
- R13 が DPR 1 と DPR 2 の両方で `ok` が真(窓の外の差が 0)。R13 の印(この Issue の番号)を外してから見る。
- `bash guard/scan.sh` が 0 件。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には「runner の値。この機械とは別枠」と書き、nproc と loadavg を添える。
- 「達成」「直った」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。出力の行を付ける。
- 報告に書く: 変わった画素の矩形(直す前と後)、原因(実測と推定を分ける)、変えた箇所、流れ A〜D の 4 行、R13 の JSON の要点。

## 制約

- 禁止されているのは、AI のベンダー名と Cloudflare の識別子だけ。FL の製品名・インストール先の名前・変数名は、書いてよい。
- 製品に、新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足さない。テストのための分岐を製品に入れない。
- Python を使わない(調査と検証にだけ使う)。OCR を使わない(FL の画面の文字は、窓の表とゲストのメモリの走査で読む)。
- 印を付けるために判定を緩めない。R13 の期待(窓の外が変わらない)を変えない。
- sleep・timeout のコマンド・nohup・末尾の `&` は使わない。待つ時は条件で待つ。一時ファイルは `.tmp-*` に置き、終わったら消す。
- 製品の作業は Actions で行う。この機械で製品のビルドや測定をしない。

## 作業の進め方

- 作業の最初に `date -u` を記録する。4 時間 30 分を過ぎたら、受け入れが通らなくても、作業ツリーを commit し、報告を書いて終える(ジョブの上限は 5 時間 50 分)。
- ビルドは約 35 分。フォアグラウンドで直接、シェルツールの `timeout` 引数は最大 5400000(90 分)にする。背景で起動して待たない。
- 受け入れが通らなくても、作業の最後に作業ツリーを commit する。未 commit の変更が残ると、判定は失敗する。

## 他のタスクとの関係

- blocked_by: #1408(E2E のシナリオ集)。R13 の道具(`tools/e2e/scenarios/regression/`・`run.mjs --only`)は、#1408 が main に入るまで使えない。
- #1384(閉じている)の直しの続き。`app/src/ui/` の描画の経路を触る。
- #1392(閉じている)の画像カーソルが、窓の外の画素に写るかを確かめる。写るなら、カーソルの経路の直しは #1392 の範囲で、この Issue では窓の外の矩形だけを直す。
