---
draft: "008"
title: "HiDPI と描画の分離の後で、描画の所要と P2 を実測する"
type: Task
milestone: "M2 HiDPI と P2 の測定(P2)"
parent: "005"
blocked_by: ["006", "002", "007"]
labels: ["hidpi", "perf", "verify"]
size: S
priority: High
project: signature-studio/3
---

## 目的

HiDPI(006)と描画の分離(002)の後で、拡大率 1 と 2 の描画の所要と、P2(UI の 120 fps。入力から画面の p95)を、同じ道具で測る。画素数が増えた影響を、推定ではなく実測で報告する。

## 規模

規模: 大(想定 3 時間。ビルドを約 5 回回す規模)

## 背景

- 006 は画素数を拡大率の 2 乗で増やす(推定: 1580×957 → 3160×1914 は 4 倍)。描画の所要が増える。P2 と P3 に効く(推定)。
- 002 は、分離の前後の音声の余裕を測る。分離は本番に出ていない(002 の背景)。
- 007 の道具 `tools/p2-frames/measure.mjs` と、006 の道具 `tools/feat-c1-hidpi/measure.mjs` を使う。
- P2 の定義と判定(旧 ci タスク 0004 の記述による): 120 fps、入力から画面の p95 が 17 ms 以下。120 fps は、007 の道具の起動の引数(表示の上限を外す)で測る。

## 読む物

- `tools/feat-c1-hidpi/measure.mjs`(006 が作る道具)
- `tools/p2-frames/measure.mjs`(007 が作る道具)
- `pm-work/findings/survey-c1-hidpi.md`、`pm-work/findings/survey-c2-worker-split.md`(旧の値。比べる基準)

## やること

1. 拡大率 1 と 2 で、006 の道具を 1 回ずつ実行し、composeMs と putMs の 1 回あたり(累計 ÷ presents)を出す。
2. 007 の道具を、拡大率 1 の起動で実行し、rAF の回数と入力の遅延を出す。
3. 結果を表にする。各値に出力の行を付ける。120 fps と p95 17 ms の判定は、表の値で「受理」「差し戻し」「未確認」のいずれかにする。
4. 旧の値(pm-work/findings/survey-c2-worker-split.md の手順 4)との差を、同じ単位で書く。

## 受け入れ

```accept
npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false?0:1)' "$RUNNER_TEMP/boot.json" && node tools/feat-c1-hidpi/measure.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/c1.jsonl" && node tools/p2-frames/measure.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/p2.json" && node -e 'const fs=require("fs");const c=fs.readFileSync(process.env.RUNNER_TEMP+"/c1.jsonl","utf8").trim().split("\n").map((l)=>JSON.parse(l));const p=JSON.parse(fs.readFileSync(process.env.RUNNER_TEMP+"/p2.json","utf8").trim());process.exit(c.length===3&&c.every((r)=>r.error===null&&typeof r.composeMs==="number")&&p.error===null&&p.frames>=60?0:1)'
```

判定の基準: 道具の出力が取れ、JSON が valid であること(上の node -e)。120 fps と p95 17 ms の判定は、報告の表で「受理」「差し戻し」「未確認」を付ける。この Issue は「受理」を目指すが、値が条件を満たさない場合は「差し戻し」と書く。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には「runner の値。この機械とは別枠」と書き、nproc と loadavg を添える。
- 実測と推定を分ける。「達成」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。
- 60 Hz の表示で 120 fps が出ない場合は、道具の不具合ではなく表示の上限として書く。

## 制約

- 製品に新しい環境変数・#ifdef・URL パラメータ・ビルド変数を足さない。
- 製品の既定の動作を、測定のためだけに変えない。
- ベンダー名と Cloudflare の識別子を書かない。
- sleep・timeout・nohup・末尾の `&` は使わない。

## 他のタスクとの関係

- blocked_by: 006(HiDPI)、002(分離の前後の測定)、007(P2 の道具)。
- 038(完成の検証)が、この Issue の値を使う。
