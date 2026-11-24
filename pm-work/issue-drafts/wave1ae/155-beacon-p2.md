---
draft: "155"
title: "ビーコンに、実機の提示の頻度(直近 10 秒の fps)と、入力から提示までの遅延(p50・p95)を足し、読み手(summary.mjs)の表に出す"
type: Task
milestone: "M2 HiDPI と P2 の測定(P2)"
parent: "005"
blocked_by: []
labels: ["perf-beacons", "render"]
size: S
priority: High
project: signature-studio/3
---

## 目的

P2(画面の描画を 120 fps に収める)の最終の判定は、依頼者の実機で行う。runner には GPU が無く、絶対値は実機と違うため、runner の受け入れでは判定できない。実機の利用者のブラウザが送るビーコンに、提示の頻度と、入力から提示までの遅延を残し、読み手の表で見られるようにする。

## 背景

確かめた結果(`app/src/perf/beacon.ts`・`app/shared/perf.ts`・`tools/perf-beacons/summary.mjs`。既に在る物と無い物):

- 既に在る: `desk`(描画の統計)の累計の `presents`。前の記録からの区間 `since`(`presents` と `seconds`)。区間は前の記録からの時間で、直近 10 秒ではない。
- 既に在る: `input.eventToRaf` の `p50Ms`・`p95Ms`(DOM の時刻から、描いた後の次のフレームまで)。`shared/perf.ts` が値の範囲を検査する。
- 既に在る: `display.raf.recent10s`(rAF の間隔の中央値・p95。直近 10 秒。`display.ts` の `RECENT_MS`)。
- 在るが表に出ていない: `summary.mjs` の「版ごと: DPR・DPI・画面・起動・途切れ・入力・engine の状態」の表は、入力を `eventToRaf.p95Ms` の中央値 1 列だけ出す。提示の頻度・p50・rAF の間隔は出していない。
- 無い: 直近 10 秒の提示の回数と fps。

## 読む物

- `app/src/perf/beacon.ts`(`DeskBeacon`・`deskBeacon`・`engineRunBeacon`(10 秒の窓の作り方の例)・`BeaconInput`)
- `app/src/perf/display.ts`(`RECENT_MS`・`raf.recent10s`)、`app/src/perf/stats.ts`(`RECENT_WINDOW_MS`・`eventToRaf` の作り方)
- `app/shared/perf.ts`(`beaconProblems`: 項目の検査。版 3 の互換)
- `app/src/engine/render-worker.ts`・`app/src/engine/protocol.ts`(`desk` の統計の報告)
- `tools/perf-beacons/summary.mjs`(表)、`tools/perf-beacons/fixtures/stop/`(合成の記録の形)
- `app/test/unit/perf-beacon-desk.test.ts`・`app/test/unit/perf-beacon.test.ts`

## やること

1. ページが、描画の統計(`desk` の累計)の時刻付きの履歴を、直近 10 秒の分だけ持つ(上限付き)。記録の `desk` に `recent10s: { seconds, presents, presentsPerSec }` を足す。`presentsPerSec` は、直近 10 秒の提示の回数を、実際の秒数で割った値。
2. 記録の `input` に、直近の窓の `inputToPresentMsP50` と `inputToPresentMsP95` を足す。`eventToRaf` の既存の値を、読みやすい項目として出す(新しい計算を二重に持たない)。
3. 記録に `rafIntervalMsP95` を足す(`display.raf.recent10s.p95Ms` と同じ値。二重の計算はしない)。
4. 版は上げない(項目は任意。古い記録は読める)。`beaconProblems` に、足した項目の型と範囲の検査を足す。
5. `summary.mjs` の表(「版ごと: DPR・DPI・画面・起動・途切れ・入力・engine の状態」)に、「提示 fps 中央値(直近 10 秒)」「入力 p50 中央値」「入力 p95 中央値」「rAF 間隔 p95 中央値」の列を足し、記録数を出す。個人を特定する値(セッション・アドレス・パス・名前)は出さない(既存の規則)。
6. 合成の記録を `tools/perf-beacons/fixtures/fps/` に 2 件置く。`bundle.version` を `PerfFpsFixture` にする(`fixtures/stop/` の形を写す)。受け入れは、この表に `PerfFpsFixture` が出ることを見る。
7. 製品に新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足さない。

## 受け入れ

```accept
npm test --prefix app && node tools/perf-beacons/summary.mjs tools/perf-beacons/fixtures/fps --day all | grep -q 'PerfFpsFixture' && bash guard/scan.sh && npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false?0:1)' "$RUNNER_TEMP/boot.json" && node tools/smoke/welcome-flow.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/flow.jsonl" && node -e 'const fs=require("fs");const L=fs.readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const ok=(r)=>r!==undefined&&r.error===null&&r.state==="engine-idle"&&r.standIn===false&&r.trapped===false&&r.hostErrors===0&&r.opened===true&&r.elapsedMs<=12500&&r.preBootBytes<=314368711&&((r.flow!=="B"&&r.flow!=="D")||r.welcomeClosed===true);process.exit(L.length===4&&["A","B","C","D"].every((f)=>ok(L.find((r)=>r.flow===f)))?0:1)' "$RUNNER_TEMP/flow.jsonl"
```

判定の基準:
- 受け入れの 1 行が通る(単体試験・fixture の表に `PerfFpsFixture` が出る・`guard/scan.sh` が 0 件・ビルド・起動確認・流れ A〜D)。
- 流れ A〜D の各行: `error` が null、`state` が engine-idle、`standIn` が false、`trapped` が false、`hostErrors` が 0、`opened` が true。流れ B と D は `welcomeClosed` が true。
- 各行の `elapsedMs` が 12,500 以下、`preBootBytes` が 314,368,711 以下。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- fixture の表の `PerfFpsFixture` の行を、そのまま貼る。
- 実機のビーコンの値は、この Issue では得られない。報告に「実機の値は未確認」と書く。

## 制約

- 版は上げない。項目は任意で、古い記録は読めること(`beaconProblems` の互換を保つ)。
- 製品に新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足さない。
- ベンダー名と Cloudflare の識別子を書かない。
- sleep・timeout・nohup・末尾の `&` は使わない。一時ファイルは `.tmp-*` に置き、終わったら消す。
- 製品の作業は Actions で行う。この機械で製品のビルドや計測をしない。

## 作業の進め方

- 開始の時刻を `date -u` で記録する。受け入れの最初の実行は、開始から 2 時間までに終える。
- ビルドは約 35 分。長いコマンドは、フォアグラウンドで直接実行し、シェルツールの `timeout` 引数に 5400000 を渡す。
- 受け入れが通っても通らなくても、作業ツリーを commit して報告を書く。

## 他のタスクとの関係

- blocked_by: なし。
- #1351(描画の所要を、変化した面積に比例させる)の実機の判定の材料。#1391(上部バーの Frame Rate の実測)と並ぶ。
- `summary.mjs` と、ビーコンの項目は、#1380(停止の理由の項目)と同じファイルを変える。衝突したら、作業の最後に `git pull --no-rebase --no-edit` で main を取り込み、衝突を直す。
