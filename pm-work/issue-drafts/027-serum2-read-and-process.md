---
draft: "027"
title: "Serum 2 を既定の経路で読み込み、1 ブロックの音声を処理する"
type: task
milestone: "M6 外部プラグイン"
parent: "024"
blocked_by: ["040", "025", "026", "015"]
labels: ["task", "plugin-external", "priority-p1"]
size: 8
---

## 目的

課題 C-5: Serum 2 を存在させる。既定の経路(オプトイン無し)で読み込み、1 ブロックの音声を処理する。1 プラグイン 1 wasm で、起動後にバックグラウンドで届ける。

## 規模

規模: 大(想定 8 時間。ビルドを約 5 回回す規模)

## 背景

- 入力: 外部の 3 件のうちの Serum 2 のインストーラ(子 025 が取る。大きさ 1,313,084,056 B。`ci/artifacts.lock` の 2 行目)。
- 既存の harness: `lift/harness/vst3a/main_vst3a.c`(1〜6 行: Serum 2 を扱う native の harness。PE を 1 つ受け取る)。wasm の build のレシピ `lift/harness/vst3a/build_vst3a_wasm.sh` は在る(旧の確認)。
- VST3 の link は 026 で既定にする。Serum 2 の読みは、`lift/harness/two/wrp1_vst3.c`(TLS・IAT・TLS callback)の経路。
- 1 プラグイン 1 wasm の形: `app/src/plugins/plugin-client.ts`(1 インスタンス 1 Worker の口)、`app/src/plugins/plugin-worker.ts`(READY までの流れ)。製品の画面からは、まだ呼ばれていない(`app/src/plugins/` はテストだけが使う)。
- 旧の読みの論点(survey-c5 の 5 節): インストーラから VST3 の本体を取り出す経路。wasm の大きさと起動時間は未測定。

## 読む物

- `lift/harness/vst3a/main_vst3a.c`(1〜6 行)、`lift/harness/vst3a/build_vst3a_wasm.sh`
- `lift/harness/two/wrp1_vst3.c`(1〜4 行)
- `app/src/plugins/plugin-client.ts`(1〜60 行)、`app/src/plugins/plugin-worker.ts`(1〜7 行、88〜93 行)、`app/src/plugins/driver.ts`
- `app/test/unit/plugins-ott.test.ts`(テストの形。入力が無いと skip する)
- `app/test/helpers/echo-plugin-worker.ts`
- 子 014 の成果物(起動後の取得の仕組み)、子 026 の成果物(既定の link)

## やること

1. 入力(子 025)が取れていることを確かめる。取れていなければ、報告に書いて止まる。
2. Serum 2 の wasm を、既定の build の一部として作る(1 プラグイン 1 wasm)。作り方(harness の build の入力、realmod の表)を、報告に書く。
3. 起動後の取得で届ける(子 014 の仕組み)。起動前の取得には入れない。
4. 道具 `tools/plugin-run/measure.mjs` で、Serum 2 を読み込み、1 ブロックの音声を処理する。JSON の 1 行を出す: `{"plugin","instantiated","blockFrames","nonSilent","error"}`。
5. 道具の結果を、報告に書く。耳の確認はしない。

## 受け入れ

```accept
npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false?0:1)' "$RUNNER_TEMP/boot.json" && node tools/plugin-run/measure.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" serum2 "$RUNNER_TEMP/external/serum2.exe" > "$RUNNER_TEMP/serum2.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8").trim());process.exit(r.error===null&&r.instantiated===true&&r.blockFrames>0?0:1)' "$RUNNER_TEMP/serum2.json"
```

判定の基準: 道具の出力の生の値で、読み込みが成功し(`instantiated`)、1 ブロックの音声が処理された(`blockFrames` > 0)こと。`error` が null。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には「runner の値。この機械とは別枠」と書き、nproc と loadavg を添える。
- 実測と推定を分ける。「達成」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。
- 報告に書く: wasm の大きさ、起動後の取得での到着時刻、読み込みの結果、1 ブロックの処理の結果、変えたパス(lift/・app/ を含む)。入力が無い場合は「実エンジン: 実行なし(入力が無い)」と書く。

## 制約

- 製品に新しい環境変数・#ifdef・URL パラメータ・ビルド変数を足さない。オプトインを作らない。
- 作業者に依頼者の個人の資格を置かない。入力の取得は子 025 の仕組みに従う。
- ベンダー名と Cloudflare の識別子を書かない。インストーラの名前に含まれる会社名を、そのまま書かない。
- Python を製品に使わない。
- sleep・timeout・nohup・末尾の `&` は使わない。一時ファイルは `.tmp-*` に置き、終わったら消す。

## 他のタスクとの関係

- blocked_by: 040(音声への圧迫の解消。プラグインの処理は、音声の経路が決まった後に入れる)、025(入力)、026(既定の link)、015(1 プラグイン 1 wasm の型)。
- 028・029 と同じ仕組みを使う。
