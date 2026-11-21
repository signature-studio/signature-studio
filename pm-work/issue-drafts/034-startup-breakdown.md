---
draft: "034"
title: "起動の内訳を実測し、P1 の測り方を確定する(取得・コンパイル・インスタンス化・起動)"
type: Task
milestone: "M8 起動 10 秒以内とサイズ(P1)"
parent: "033"
blocked_by: []
labels: ["startup", "verify"]
size: S
priority: Medium
project: signature-studio/3
---

## 目的

起動(ページを開いてから engine-idle まで)の内訳を、段ごとに実測する。併せて、P1 の測り方(何を始まりと終わりにするか)を確定する。測り方が違うと、旧の値(9.2〜12.8 秒)が比べられない。

## 規模

規模: 大(想定 3 時間。ビルドを約 5 回回す規模の前段)

## 背景

- 測り方の違い(旧の測定): boot-bundle の `timelineIdle`(9.2〜9.4 秒。survey-c2)、navigate から engine-idle の確認まで(10.2〜10.6 秒。survey-c1。poll を含む)、probe を含む(12.3〜12.8 秒。survey-c6)。
- 段の候補(`app/src/main.ts` 714〜724 行): `loadWasm`(engine wasm の取得と `compileStreaming`)、`loadPack`(seed pack)、`loadPack`(DLL pack)、`fetchAsset`(glue)、`client.init`(Worker への渡し)。`app/src/engine/fruity.ts` 133 行(`WebAssembly.instantiate`)。
- 旧のコード: `stamp('compile_done')` の印が、`app/src/main.ts` の起動の経路に在る(段の時刻の印)。

## 読む物

- `app/src/main.ts`(714〜724 行: 起動時の取得と client.init。`stamp` の使い方)
- `app/src/bundle/loader.ts`(185 行: `fetchAsset`。189・197 行: `loadWasm`。219 行: `loadPack`)
- `app/src/engine/fruity.ts`(120〜140 行)
- `tools/boot-check/boot-bundle.mjs`(`timelineIdle` の出し方)、`tools/boot-check/cdp.mjs`
- `pm-work/findings/survey-c1-hidpi.md`、`pm-work/findings/survey-c2-worker-split.md`、`pm-work/findings/survey-c6-topbar-menus.md`(旧の値。測り方)

## やること

1. 段の時刻を取る道具 `tools/startup-breakdown/measure.mjs` を作る。段の時刻は、ページの印(stamp)と resource timing から取る。stdout に JSON の 3 行(1 行が 1 回の起動): `{"state","standIn","error","navToIdleMs","timelineIdle","phases":{"fetchWasm","compileWasm","fetchPacks","instantiate","idle"}}`(各段は ms。取れない段は null)。
2. 3 回の起動で、段の時刻を取る。中央値を出す。
3. P1 の始まりと終わりを、この Issue の報告で確定する(案: 始まりは navigate、終わりは engine-idle の最初の印)。確定の理由を書く。
4. 旧の 3 つの測り方の値と、この Issue の値の対応を、表で書く。

## 受け入れ

```accept
npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/startup-breakdown/measure.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/startup.jsonl" && node -e 'const L=require("fs").readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));process.exit(L.length===3&&L.every((r)=>r.state==="engine-idle"&&r.standIn===false&&r.error===null&&typeof r.navToIdleMs==="number"&&typeof r.phases==="object")?0:1)' "$RUNNER_TEMP/startup.jsonl"
```

判定の基準: 3 回とも engine-idle に達し、段の時刻の項目が出ていること。値の目標(10 秒)は判定に入れない(子 035)。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には「runner の値。この機械とは別枠」と書き、nproc と loadavg を添える。
- 実測と推定を分ける。「達成」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。
- 報告に書く: 段ごとの中央値、P1 の測り方の確定とその理由、旧の値との対応表、取れなかった段と理由。

## 制約

- 製品に新しい環境変数・#ifdef・URL パラメータ・ビルド変数を足さない。計測のための切り替えを、製品に作らない。
- 製品の既定の起動の動作を、測定のためだけに変えない。
- ベンダー名と Cloudflare の識別子を書かない。
- sleep・timeout・nohup・末尾の `&` は使わない。待ちは JS のタイマーで行う。

## 他のタスクとの関係

- 035(10 秒以内にする)が、この Issue の測り方を使う。
- 014(起動後の取得の仕組み)と、同じ起動の経路を測る。
