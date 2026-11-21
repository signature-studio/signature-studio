---
draft: "007"
title: "P2 の測定の道具を作る(フレーム間隔と入力の遅延を JSON で出す)"
type: Task
milestone: "M2 HiDPI と P2 の測定(P2)"
parent: "005"
blocked_by: []
labels: ["perf", "verify"]
size: M
priority: High
project: signature-studio/3
---

## 目的

完成の条件の 1 つ P2(UI が 120 fps で動く)を、runner で測る道具を作る。この Issue は道具を作って、道具が使えることを確かめる。120 fps の判定は#{{008}} で行う。

## 背景

- P2 の定義(旧 ci タスク 0004 の記述による。出典の文書 docs/ops/fidelity-and-performance.md は、リポジトリに無い): 所定のプロジェクトを開き、UI を 120 fps で動かし、入力から画面の p95 が 17 ms 以下。
- 旧の記録: この機械の表示は 60 Hz だった。120 fps を、表示の上限に縛られずに測るため、headless Chrome のフレーム数の上限を外す起動の引数を使う。引数は作業者が選び、報告に書く。
- 既存の道具: `tools/verify-kit/tools/sdriveg2-lat.mjs`(入力の遅延 T と、`presentsPerS` を出す駆動。先頭のコメントに使い方)、`tools/verify-kit/tools/scn-lat-ff.json`(P2 の台本の候補)、`tools/verify-kit/serve-once-lat-bundle.mjs`(bundle を 1 回配信する道具)。
- 製品の画面のフレーム統計: `window.__app.deskStats`(`presents`・`ticks`・`idleTicks` など。main の描画分離後も同じ名前で出る。`app/src/engine/protocol.ts` の 87 行付近)。

## 読む物

- `tools/verify-kit/tools/sdriveg2-lat.mjs`(先頭のコメントに使い方)
- `tools/verify-kit/tools/scn-lat-ff.json`(台本)
- `tools/verify-kit/serve-once-lat-bundle.mjs`(配信)
- `tools/boot-check/boot-bundle.mjs`、`tools/boot-check/cdp.mjs`(起動の手順)
- `app/src/engine/protocol.ts`(87 行付近: desk の統計の型)

## やること

1. 既存の道具の使い方を読み、boot-bundle の起動と組み合わせられるかを確かめる。組み合わせられない場合は、理由を報告に書く。
2. 道具 `tools/p2-frames/measure.mjs` を作る。引数は bundle の絶対パス。stdout に JSON の 1 行だけを出す: `{"state","standIn","error","frames","rafFps","rafIntervalP95Ms","inputLatencyP95Ms","presentsPerS","loadavg","nproc"}`。`frames` は計測した rAF の回数、`rafFps` は `frames` を計測時間で割った値。入力の遅延は、台本の入力から画面の変化までの時間(取れない場合は null)。
3. 計測の時間は、道具の中で JS のタイマーで決める(sleep を使わない)。計測は engine-idle の後に始める。
4. 道具の JSON の値を、報告に貼る。

## 受け入れ

```accept
npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false?0:1)' "$RUNNER_TEMP/boot.json" && node tools/p2-frames/measure.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/p2.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8").trim());process.exit(r.error===null&&r.state==="engine-idle"&&r.standIn===false&&r.frames>=60&&typeof r.rafFps==="number"&&typeof r.rafIntervalP95Ms==="number"?0:1)' "$RUNNER_TEMP/p2.json"
```

判定の基準: 道具の出力の生の値を、上の node -e が読む。120 fps を満たすかは判定に入れない(#{{008}} で判定する)。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には「runner の値。この機械とは別枠」と書き、nproc と loadavg を添える。
- 実測と推定を分ける。「達成」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。
- 60 Hz の表示で 120 fps が出ない場合、それは道具の判定ではなく、表示の上限である。その旨を書く。

## 制約

- 製品に新しい環境変数・#ifdef・URL パラメータ・ビルド変数を足さない。
- 製品の既定の動作を、測定のためだけに変えない。
- ベンダー名と Cloudflare の識別子を書かない。
- Python を製品に使わない。道具は JavaScript で書く。
- sleep・timeout・nohup・末尾の `&` は使わない。一時ファイルは `.tmp-*` に置き、終わったら消す。

## 他のタスクとの関係

- #{{008}}(HiDPI と描画の分離の後の実測)が、この道具を使う。
- #{{003}}(P3 の音声の道具)と、起動の手順(`tools/boot-check/`)を共有する。道具を変えるときは、両方の受け入れを通す。
