---
draft: "029"
title: "Vital を既定の経路で読み込み、1 ブロックの音声を処理する"
type: task
milestone: "M6 外部プラグイン"
parent: "024"
blocked_by: ["040", "025", "026", "015"]
labels: ["task", "plugin-external", "priority-p1"]
size: 6
---

## 目的

課題 C-5: Vital を存在させる(Serum 2・OTT と合わせた 3 件のうちの 1 件として)。既定の経路で読み込み、1 ブロックの音声を処理する。1 プラグイン 1 wasm で、起動後に届ける。

## 規模

規模: 大(想定 6 時間。ビルドを約 5 回回す規模)

## 背景

- 入力: 外部の 3 件のうちの Vital のインストーラ(子 025 が取る。大きさ 41,808,960 B。`ci/artifacts.lock` の 4 行目)。
- 未確認: Vital の harness の有無、Vital のインストーラの中身(VST3 のバンドルの場所)。リポジトリにも、この機械にも、Vital の harness は無い(pm-work/findings/survey-c5-external-plugins.md は Vital を入力の一覧に挙げるのみ)。
- 対象に含める(依頼者の指示で、外部プラグインは Serum 2・OTT・Vital の 3 件を全て対象にする)。

## 読む物

- `ci/artifacts.lock`(4 行目)
- `lift/harness/vst3a/main_vst3a.c`(1〜6 行: Serum 2 の native harness。VST3 の読みの参考)
- `lift/harness/two/wrp1_vst3.c`(1〜4 行)
- `app/src/plugins/plugin-client.ts`、`app/src/plugins/plugin-worker.ts`
- 子 014 の成果物、子 026 の成果物

## やること

1. 入力(子 025)から、Vital のインストーラの中身を確かめる。
2. 入力(子 025)から、Vital の VST3 の本体を取り出す経路を確かめ、報告に書く。
3. harness の作り方を決める(027 の Serum 2 と同じ形を使えるかを確かめる)。
4. 1 プラグイン 1 wasm で既定の build に入れ、起動後の取得で届ける。
5. 道具 `tools/plugin-run/measure.mjs` で、Vital を読み込み、1 ブロックの音声を処理する。結果を報告に書く。

## 受け入れ

```accept
npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false?0:1)' "$RUNNER_TEMP/boot.json" && node tools/plugin-run/measure.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" vital "$RUNNER_TEMP/external/vital.exe" > "$RUNNER_TEMP/vital.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8").trim());process.exit(r.error===null&&r.instantiated===true&&r.blockFrames>0?0:1)' "$RUNNER_TEMP/vital.json"
```

判定の基準: 道具の出力の生の値で、読み込みが成功し、1 ブロックの音声が処理され、`error` が null。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には「runner の値。この機械とは別枠」と書き、nproc と loadavg を添える。
- 実測と推定を分ける。「達成」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。
- 入力が無い、または対象外の場合は、「実エンジン: 実行なし(理由)」と書く。

## 制約

- 製品に新しい環境変数・#ifdef・URL パラメータ・ビルド変数を足さない。オプトインを作らない。
- 作業者に依頼者の個人の資格を置かない。
- ベンダー名と Cloudflare の識別子を書かない。
- sleep・timeout・nohup・末尾の `&` は使わない。一時ファイルは `.tmp-*` に置き、終わったら消す。

## 他のタスクとの関係

- blocked_by: 040(音声への圧迫の解消)、025、026、015。
- 027・028 と同じ仕組みを使う。
