---
draft: "028"
title: "OTT を既定の経路で読み込み、1 ブロックの音声を処理する(harness の source を作る)"
type: Task
milestone: "M6 外部プラグイン"
parent: "024"
blocked_by: ["040", "025", "026", "015"]
labels: ["plugin-external"]
size: L
priority: Medium
project: signature-studio/3
---

## 目的

課題 C-5: OTT を存在させる。既定の経路で読み込み、1 ブロックの音声を処理する。1 プラグイン 1 wasm で、起動後に届ける。

## 背景

- 入力: 外部の 3 件のうちの OTT のインストーラ(#{{025}} が取る。大きさ 8,397,928 B。`ci/artifacts.lock` の 3 行目)。
- 欠けている物: OTT の harness の source と build は、リポジトリに無い。`app/src/plugins/ott-host.ts` の 1〜3 行が指す `lift/harness/vst`(decision 0156・0164)と、`lift/harness/web/mt1/plugin-worker/main_vst_ext.c`(decision 0173)は、どちらも無い。`app/test/unit/plugins-ott.test.ts` の 2 行が指す `lift/harness/vst/vfs-seed-vst.tsv` も無い(pm-work/findings/survey-c5-external-plugins.md の 2 節)。
- したがって、この Issue は、harness の source の作り直しから始める。作り直しの元は、この機械に無い。作業者は、OTT の入力を読んで、harness の形を新しく決める。
- 既存の先例: `app/src/plugins/ott-host.ts`(1 インスタンス 1 Worker。テストからだけ使われる)、`app/src/plugins/plugin-client.ts`。

## 読む物

- `app/src/plugins/ott-host.ts`(1〜4 行)、`app/src/plugins/plugin-client.ts`、`app/src/plugins/plugin-worker.ts`
- `app/test/unit/plugins-ott.test.ts`(2 行と、15 行の `have` の判定)
- `lift/harness/vst3a/main_vst3a.c`(Serum 2 の native harness。OTT の harness の形の参考)
- `lift/harness/two/wrp1_vst3.c`(VST3 の読みの経路)
- #{{014}} の成果物(起動後の取得)、#{{026}} の成果物(既定の link)

## やること

1. 入力(#{{025}})が取れていることを確かめる。取れていなければ、報告に書いて止まる。
2. OTT の harness の source を作る(`lift/harness/` の下に新しく置く)。作った場所と形を、報告に書く。
3. 1 プラグイン 1 wasm の build を、既定の build の一部として作る。起動後の取得で届ける(#{{014}})。
4. 道具 `tools/plugin-run/measure.mjs` で、OTT を読み込み、1 ブロックの音声を処理する。JSON の 1 行を出す: `{"plugin","instantiated","blockFrames","nonSilent","error"}`。
5. 結果を報告に書く。耳の確認はしない。

## 受け入れ

```accept
npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false?0:1)' "$RUNNER_TEMP/boot.json" && node tools/plugin-run/measure.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" ott "$RUNNER_TEMP/external/ott.exe" > "$RUNNER_TEMP/ott.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8").trim());process.exit(r.error===null&&r.instantiated===true&&r.blockFrames>0?0:1)' "$RUNNER_TEMP/ott.json"
```

判定の基準: 道具の出力の生の値で、読み込みが成功し、1 ブロックの音声が処理され、`error` が null。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には「runner の値。この機械とは別枠」と書き、nproc と loadavg を添える。
- 実測と推定を分ける。「達成」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。
- 報告に書く: harness の source の作り方と場所、wasm の大きさ、起動後の取得の到着時刻、読み込みと 1 ブロックの結果、変えたパス。入力が無い場合は「実エンジン: 実行なし(入力が無い)」と書く。

## 制約

- 製品に新しい環境変数・#ifdef・URL パラメータ・ビルド変数を足さない。オプトインを作らない。
- 作業者に依頼者の個人の資格を置かない。
- ベンダー名と Cloudflare の識別子を書かない。
- Python を製品に使わない。
- sleep・timeout・nohup・末尾の `&` は使わない。一時ファイルは `.tmp-*` に置き、終わったら消す。

## 他のタスクとの関係

- blocked_by: #{{040}}(音声への圧迫の解消。プラグインの処理は、音声の経路が決まった後に入れる)、#{{025}}(入力)、#{{026}}(既定の link)、#{{015}}(1 プラグイン 1 wasm の型)。
- #{{027}}・#{{029}} と同じ仕組みを使う。
