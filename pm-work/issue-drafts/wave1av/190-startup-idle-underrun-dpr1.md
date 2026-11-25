---
draft: "190"
title: "起動から first idle の後、無入力の 10 秒で、音声の途切れが 1 件出る(DPR 1)"
type: Task
milestone: "M1 描画と音声の分離(P3)"
parent: "001"
blocked_by: ["165"]
labels: ["verify"]
size: M
priority: Urgent
project: signature-studio/3
---

## 目的

起動が first idle に届いた後、何も入力しない 10 秒の間に、音声の出力が途切れる(underrun)ことがある。依頼者の規則は「アイドルの音声の途切れは 0」。#1408 の E2E の R18 `startup-no-dropout-10s`(DPR 1)で、途切れが 1 件出た。DPR 2 では出ていない。この Issue は、その 1 件の原因を特定し、直す。

この Issue の目標は次の 3 つ。
1. 起動の後の無入力の 10 秒で、途切れ(underrun)が 0 になる(DPR 1 で、5 回続けて)。
2. 途切れが出た時の、起動からの時刻と、engine が何をしていたか(描画・lazy の取得・プラグインの読み込み・GC のどれか)を、記録して原因を特定する。
3. 原因を直す(効果が出る変更を 1 つ。変更の前後を `tools/ab` で測る)。

## 前回の試行で分かったこと(#1408 の run 45594361647)

数値は runner の値(nproc 4、loadavg 2.10 から 3.13)。
- R18 の結果(DPR 1): `first idle, then 10 s with no input: underrun quanta 1; the output: status running, context running, error none`。
- この試行の全体の実行は 2 回とも DPR 1 で途切れ 1 件。単独のデバッグの 1 回は 0 件。DPR 2 は 3 回とも 0 件。
- 1 つ前の試行(run 45548619833)の R18 は、DPR 1 で underrun 0 と記録された。途切れは間欠で、回によって出たり出なかったりする。
- R17(再生中に、マウスを 1,742 Hz・1,696 Hz で動かす)は、underrun 0。途切れは、無入力の起動の後に出ている。
- 既存の道具: `tools/p3-audio-probe/measure.mjs --boot` は、起動の後 10 秒の `bootLast60sAt10s` を 3 回の中央値で見る(中央値 30 以下・最大 60 以下が粗い基準)。R18 の基準(途切れ 0)は、それより厳しい。
- 途切れの時刻(起動からの ms)と、その時の engine の状態は、前回の記録に無い。この Issue で最初に取る。

## 背景

- 依頼者の規則(「アイドルの音声の途切れは 0」)。台帳 `pm-work/ledger/instructions.tsv` を読む。
- #1375(閉じている。起動直後の途切れ 段 B。残りの 421 ms の区間(wasm の C++ 描画と C の関数)に切り替え点を入れ、補充の最大の間隔を窓の値にした)。#1352(閉じている。起動の途切れの計測)。
- #1408(E2E のシナリオ集)の R18 `startup-no-dropout-10s`: 起動して最初の idle の後、10 秒の無入力で、途切れ 0 を求める。R18 には、この Issue の番号の印が付く。この Issue が直ったら、R18 の印を外す。

## 読む物

- `tools/e2e/scenarios/regression/`(R18。#1408 が main に入った後に読める)
- `tools/e2e/run.mjs`・`tools/e2e/lib/`(実行と検出。`--only` で 1 本だけ走らせられる)
- `tools/p3-audio-probe/`(`measure.mjs`・`burst-report.mjs`)
- `tools/ab/run.mjs`(A/B の比較)
- `lift/runtime/fle_thread.c`・`lift/runtime/fle_audio_ring.c`・`lift/runtime/fle_audio_transport.c`(音声の出力の経路)
- `app/src/engine/` の音声(worklet・ring buffer)
- `pm-work/findings/boot-burst-1352.md`(#1352 の記録)

## やること

0. 作業の最初に `date -u` を記録する。`.prior-work.md` が在れば、最初に読む。
1. **再現**: `bash tools/e2e/run.sh --scenarios regression --only startup-no-dropout-10s --displays 4k-dpr1` を 5 回。途切れが出た時の、起動からの時刻(ms)と、その時の engine の状態(何をしていたか)を、表にする。engine のログは `tools/p3-audio-probe/burst-report.mjs` で読む。
2. **原因の特定**: 途切れの直前に engine が何をしていたか。その区間の長さ(ms)と、音声の補充の間隔(ms)。
3. **直す**: 効果が出る変更を 1 つ。変更の前後を、`tools/ab` の A/B で測る。既定の動作として直す(オプトインを作らない)。
4. 報告に、5 回の表(時刻・状態)、原因、変更の前後の A/B を書く。

## 受け入れ

```accept
npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false&&r.elapsedMs<=12500?0:1)' "$RUNNER_TEMP/boot.json" && node tools/smoke/welcome-flow.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/flow.jsonl" && node -e 'const fs=require("fs");const L=fs.readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const F=(f)=>L.find((r)=>r.flow===f);const ok=(r)=>!!r&&r.error===null&&r.state==="engine-idle"&&r.standIn===false&&r.trapped===false&&r.hostErrors===0&&r.opened===true;process.exit(L.length===4&&["A","B","C","D"].every((f)=>ok(F(f)))&&F("B").welcomeClosed===true&&F("D").welcomeClosed===true&&Math.max(...L.map((r)=>r.preBootBytes))<=314368711?0:1)' "$RUNNER_TEMP/flow.jsonl" && npm test --prefix app && (cd tools/e2e && npm ci && npx playwright install chromium) && for i in 1 2 3 4 5; do bash tools/e2e/run.sh --scenarios regression --only startup-no-dropout-10s --displays 4k-dpr1 > "$RUNNER_TEMP/r18-$i.jsonl" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8").trim());process.exit(r.scenarios.length===1&&r.scenarios[0].ok===true?0:1)' "$RUNNER_TEMP/r18-$i.jsonl" || exit 1; done && bash guard/scan.sh
```

判定の基準(全て満たす):
- 起動確認(`boot.json`)が engine-idle・standIn が false・error が null・elapsedMs が 12,500 以下。
- 流れ A〜D の 4 行が、`error` が null・engine-idle・`standIn` が false・`trapped` が false・`hostErrors` が 0・`opened` が true。B と D は `welcomeClosed` が true。
- `npm test --prefix app` が通る。
- R18 を 5 回続けて走らせ、5 回とも R18 の `ok` が真(無入力の 10 秒の途切れが 0)。R18 の印(この Issue の番号)を外してから見る。
- `bash guard/scan.sh` が 0 件。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には「runner の値。この機械とは別枠」と書き、nproc と loadavg を添える。
- 「達成」「直った」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。出力の行を付ける。
- 報告に書く: 5 回の表(途切れの時刻と、engine の状態)、原因(実測と推定を分ける)、変えた箇所、変更の前後の A/B、流れ A〜D の 4 行。
- 1 回の試行で受け入れが通らない時は、再現の表(5 回の途切れの時刻と状態)と、原因の特定まで書いて終える。

## 制約

- 禁止されているのは、AI のベンダー名と Cloudflare の識別子だけ。FL の製品名・インストール先の名前・変数名は、書いてよい。
- 製品に、新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足さない。テストのための分岐を製品に入れない。
- Python を使わない(調査と検証にだけ使う)。OCR を使わない。
- 印を付けるために判定を緩めない。R18 の期待(途切れ 0)を変えない。
- sleep・timeout のコマンド・nohup・末尾の `&` は使わない。待つ時は条件で待つ。一時ファイルは `.tmp-*` に置き、終わったら消す。
- 製品の作業は Actions で行う。この機械で製品のビルドや測定をしない。

## 作業の進め方

- 作業の最初に `date -u` を記録する。4 時間 30 分を過ぎたら、受け入れが通らなくても、作業ツリーを commit し、報告を書いて終える(ジョブの上限は 5 時間 50 分)。
- ビルドは約 35 分。フォアグラウンドで直接、シェルツールの `timeout` 引数は最大 5400000(90 分)にする。背景で起動して待たない。
- 受け入れが通らなくても、作業の最後に作業ツリーを commit する。未 commit の変更が残ると、判定は失敗する。

## 他のタスクとの関係

- blocked_by: #1408(E2E のシナリオ集)。R18 の道具(`tools/e2e/scenarios/regression/`・`run.mjs --only`)は、#1408 が main に入るまで使えない。
- #1375・#1352(閉じている)の直しの続き。音声の経路(`lift/runtime/fle_audio_*.c`)を、#1375 と同じ区間で触る。
- #1419(Browser の面の console)と #1421(DEDZII の停止)とは別の経路。
