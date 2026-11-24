---
draft: "127"
title: "内蔵プラグインを 1 本ずつ確かめる道具: 任意のプラグインの名前を渡すと、起動 → 要求 → instantiate → 1 ブロックの処理まで走る"
type: Task
milestone: "M4 FL に必要な物の配布"
parent: "012"
blocked_by: ["125", "126"]
labels: ["verify", "plugin-builtin"]
size: M
priority: High
project: signature-studio/3
---

## 目的

第 2 波の 45 束は、束ごとに 3 本のプラグインを確かめる。その確かめを、プラグインの名前を渡すだけで同じ方法で行える道具を作る。1 本ずつ、guest がそのプラグインを要求し(instantiate)、1 ブロックの音声を処理して、出力が有限で非無音であることを確かめる。

対象は効果プラグイン(Effects)。インストゥルメント(Generators)は、起動の後に音を出す必要があり、この Issue の範囲外。

## 背景(状況。実測)

- #{{125}} で、プラグインは `lift/plugins/<名前>/plugin.tsv` で足せる。道具は、`lift/plugins/*/plugin.tsv` の `key` とフォルダ名から、プラグインを選ぶ。
- 既存の道具: `tools/delivery-split/measure.mjs`。起動 → 最初の idle → 場面(Welcome を閉じる → Ctrl+O → `NewStuff.flp` を開く)→ `reads` の instantiate。要求させるのは Fruity Balance だけ(`NewStuff.flp` がその名前を含む)。
- 任意のプラグインを要求させる場面は、まだ無い。候補(どれが使えるかを、実測で選ぶ):
  - (a) ミキサーの FX スロットのメニューから、プラグインの名前で足す。`tools/feat-c6-popup-menus/` がメニューの操作を持つ。ただし、トップバーのメニューの回帰(#{{106}} の結果)が今の main で動くかは未確認。
  - (b) harness の試験の入口(`lift/harness/two/main_two.c`)から、指定の DLL を guest の `LoadLibrary` と同じ経路(`fle_realmod_map_image`)で読む。製品の既定の動作は変えず、環境変数も使わない。
  - (c) 名前を含む `.flp` を `lift/out-app/host-extra/resources/` から選ぶ(#{{126}} と同じ。全てのプラグインに `.flp` が在るとは限らない)。
- 1 ブロックの処理: テスト信号(サイン波)を入れ、出力が有限で、非無音であることを確かめる(#{{124}} の VST3 の道具と同じ考え。既存の道具を再利用できるかは、実測で決める)。

## やること

1. 道具 `tools/plugin-run/run.mjs`(新規)を作る。引数: バンドルのディレクトリ、プラグインのフォルダの名前(1 つ以上)。名前は `lift/plugins/<名前>/` のフォルダの名前で、大文字と小文字・空白とハイフンは区別しない(例: `Fruity Balance` と `fruity-balance` は同じ)。フォルダの中の本体の DLL を全て確かめる。
2. 各本体について: 起動 → engine-idle → プラグインを要求させる場面(上の (a)〜(c) から、実測で選んだ 1 つ)→ `reads` に、その本体の wasm の instantiate(`requested: true`)→ 1 ブロックの処理。
3. 結果は、本体ごとに 1 行の JSON: `{"plugin":"Fruity Balance","instantiated":true,"processed":true,"error":null,"elapsedMs":<数>}`。`elapsedMs` は、その起動の、navigation から最初の engine-idle までの時間(`tools/delivery-split/measure.mjs` の定義と同じ)。`processed` は、出力が全て有限で、RMS が 0 を超えるとき true。
4. 1 プラグインを 1 回の起動で測る。既定の起動の動作を、測定のためだけに変えない。
5. 場面が失敗したら、画面の写しを `.tmp-plugin-run-fail-<名前>.png` に残し、報告に書く。報告を書いた後、`.tmp-*` は消す。
6. 道具の試験(`spec/` の下)を 1 つ足す。少なくとも、名前の解決(存在しない名前は 1 で止まる)を確かめる。

## 受け入れ

```accept
npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/plugin-run/run.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" "Fruity Balance" "Fruity Filter" > "$RUNNER_TEMP/plugin-run.jsonl" && node -e 'const L=require("fs").readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const ok=L.length===2&&L.every((r)=>r.instantiated===true&&r.processed===true&&r.error===null);process.exit(ok?0:1)' "$RUNNER_TEMP/plugin-run.jsonl"
```

判定の基準: Balance と Filter の 2 行が在り、どちらも `instantiated` が true、`processed` が true、`error` が null。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には「runner の値。この機械とは別枠」と書き、nproc と loadavg を添える。
- 実測と推定を分ける。「達成」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。出力の行を付ける。
- 報告に書く: 選んだ場面(a)〜(c) と、選んだ理由(他の候補が動かなかった場合は、その出力)、1 行の JSON の出力、`processed` の判定に使った値(RMS と最大の絶対値)。

## 制約

- 製品に新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足さない。道具は `tools/` の中に置く。
- 製品の既定の起動の動作を、測定のためだけに変えない。
- ベンダー名と Cloudflare の識別子を書かない。Python は製品に使わない。
- sleep・timeout・nohup・末尾の `&` は使わない。一時ファイルは `.tmp-*` に置き、終わったら消す。
- 製品の作業は Actions で行う。この機械で製品のビルドや測定をしない。

## 他のタスクとの関係

- blocked_by: #{{125}}(プラグインのディレクトリの仕組み)と #{{126}}(Filter の場所。受け入れに Filter を使うため)。
- #{{124}}(VST3 の 1 ブロックの処理の道具)と、処理の判定の方法を揃える。
- Generators(インストゥルメント)の 1 本ずつの確かめは、この Issue の範囲外(別の Issue で、音を出す場面を作る)。
- `tools/delivery-split/measure.mjs` と `tools/feat-c6-popup-menus/` は、場面の作り方の参考。両方とも変えない(新しい道具を作る)。
