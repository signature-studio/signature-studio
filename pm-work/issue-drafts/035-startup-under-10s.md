---
draft: "035"
title: "起動の中央値を 10 秒以内にする(起動前に取る量と、wasm のコンパイルを見直す)"
type: Task
milestone: "M8 起動 10 秒以内とサイズ(P1)"
parent: "033"
blocked_by: ["034", "014"]
labels: ["startup"]
size: L
priority: Medium
project: signature-studio/3
---

## 目的

P1: 起動(034 で確定する測り方)の中央値を、runner で 10 秒以内にする。起動前に取る量を減らし、取得とコンパイルの順序を見直す。

## 規模

規模: 大(想定 8 時間。ビルドを約 5 回回す規模)

## 背景

- 034 の測り方と、段ごとの中央値の結果による。
- 014 の仕組み(起動前と起動後の区分)を使い、起動前の取得から、起動に要らない物を外す。どの物を外すかは、034 の結果と 014 の区分で決める。
- 起動前の取得の大きさ(runner の値): engine wasm 167,414,828 B、seed pack 20,780,949 B、DLL pack 88,675,800 B(11 ファイル)。
- 候補(案。作業者が決める): wasm のストリーミングのコンパイル(`compileStreaming` は、既に `app/src/bundle/loader.ts` 197 行で使う)、取得の並行化、起動に要らない pack の遅延、DLL pack のうち起動に要らない物の起動後への移動。
- サイズの方針: engine wasm の大きさの目標は無い。単一の wasm ファイルの上限は、ブラウザが読み込める約 1GB。超える場合は分割する。起動前に取る量は、P1 の実測で決める。この Issue の受け入れは時間だけを見る。

## 読む物

- `pm-work/findings/`(034 の成果物。段の時刻)
- `app/src/main.ts`(714〜724 行)、`app/src/bundle/loader.ts`(185・189・197・219 行)
- 014 の成果物(区分の仕組み)、`tools/startup-breakdown/measure.mjs`(034 が作る道具)
- `app/src/engine/fruity.ts`(133 行: instantiate)

## やること

1. 034 の段ごとの中央値を読み、最も大きい段を決める。
2. 最も大きい段から順に直す。直した後の中央値を、同じ道具で測る。
3. 直した内容(起動前から外した物、順序、並行化)と、変更の前後の中央値を、報告に書く。
4. 10 秒を超える場合は、残りの段を報告に書き、「差し戻し」と書く。

## 受け入れ

```accept
npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && for i in 1 2 3; do node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/p1-$i.json" || exit 1; done && node -e 'const fs=require("fs");const L=[1,2,3].map((i)=>JSON.parse(fs.readFileSync(process.env.RUNNER_TEMP+"/p1-"+i+".json","utf8").trim()));const t=L.map((r)=>r.timelineIdle).sort((a,b)=>a-b);process.exit(L.every((r)=>r.error===null&&r.state==="engine-idle"&&r.standIn===false)&&t[1]<=10000?0:1)'
```

判定の基準: 3 回とも engine-idle、standIn が false、error が null で、`timelineIdle` の中央値が 10000 以下。`timelineIdle` は、旧 ci タスク 0003 の P1 の値(最初の idle の ms)。034 で測り方を直した場合は、その測り方に合わせて、報告に両方を書く。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には「runner の値。この機械とは別枠」と書き、nproc と loadavg を添える。
- 実測と推定を分ける。「達成」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。出力の行を付ける。
- 初回とキャッシュ後の値を分ける。キャッシュ後の測り方が無い場合は、「未確認(キャッシュ後の測り方が無い)」と書く。
- 報告に書く: 変更の前後の中央値、直した段、起動前の取得の大きさの変化、変えたパス(app/・lift/ を含む)。

## 制約

- 製品に新しい環境変数・#ifdef・URL パラメータ・ビルド変数を足さない。
- 製品の既定の起動の動作を、測定のためだけに変えない。
- 作業者に依頼者の個人の資格を置かない。
- ベンダー名と Cloudflare の識別子を書かない。
- sleep・timeout・nohup・末尾の `&` は使わない。

## 他のタスクとの関係

- blocked_by: 034(測り方)、014(起動前と起動後の区分)。
- 032(直接 wasm 化)と同じ engine の build を使う。衝突したら、作業の最後に `git pull --no-rebase --no-edit` で main を取り込み、受け入れをもう一度通す。
