---
draft: "028"
title: "OTT を、起動後の取得で届け、既定の経路で読み込む(配信と LoadLibrary の経路)"
type: Task
milestone: "M6 外部プラグイン"
parent: "024"
blocked_by: ["040", "025", "026", "015", "124"]
labels: ["plugin-external"]
size: L
priority: Medium
project: signature-studio/3
---

## 目的

課題 C-5: OTT を存在させる。#{{123}}・#{{124}} の VST3 ホストで、OTT が 1 ブロックの音声を処理できた後に、(1) OTT の wasm を、1 プラグイン 1 wasm で、起動後の取得で届け、(2) 既定の経路(オプトイン無し)で、FL のプラグインの検索と guest の LoadLibrary からその wasm を使えるようにする。

## 背景

- 入力: `lift/out-app/external/` の OTT のインストーラ(`ci/artifacts.lock` の 3 行目。8,397,928 B)。VST3 本体は 3,244,032 B(`inputs/external-plugins.tsv` の OTT の行。SHA-256 の先頭 8965c711…)。
- 前回の試行(run 45253143948。報告 agent-1337)の結果: 差し戻し。
  - 受け入れの入力 `$RUNNER_TEMP/external/ott.exe` は存在しない。作業の段が `$RUNNER_TEMP/external/*` を `lift/out-app/external/` へ移すので、その dir は空になる(`.github/workflows/agent.yml` の 569〜575 行)。受け入れは、`lift/out-app/external/` の入力を `build.sh` が取り出す形に直した。
  - ビルドと起動確認は通った: engine-idle、error null、standIn false、elapsedMs 8,112(判定の段の実行。runner の値)。
  - OTT の harness が無く、wasm は作れなかった。道具 `tools/plugin-run/measure.mjs` も無い(#{{123}} で作る)。
  - 配信の経路は在る: `app/scripts/pack-assets.mjs --lazy-file SRC=PATH`(#{{015}}。manifest の `lazy` に `{name, path, …}` を書く。名前は PATH の末尾)。
- 既存の型: #{{102}} の仕組み(`fe-wasmgen hybrid --separate`、`lift/harness/hyb/hyb-args.tsv`、`app/src/engine/xmod.ts`)は、engine の中の DLL を別の wasm にし、guest の LoadLibrary の時に instantiate する。プラグインの Worker(`app/src/plugins/`)とは別の型。どちらを既定の経路にするかは、この Issue で決め、理由を報告に書く。
  - 案 A: guest の LoadLibrary の経路から、プラグインの Worker(`app/src/plugins/plugin-client.ts`)を起こす。製品の画面からは、まだ呼ばれていない。
  - 案 B: #{{102}} と同じ型(xmod)で、VST3 の wasm を instantiate する。
- 注意: 起動前に取る量は増やさない。増える場合は、増えたバイト数を報告に書く。

## 読む物

- #{{123}} と #{{124}} の成果物(`lift/harness/web/mt1/plugin-worker/build.sh`・`main_vst_ext.c`・`tools/plugin-run/measure.mjs`)
- `app/scripts/pack-assets.mjs`(`--lazy-file`。39 行目以降)、`app/scripts/pack-seed.mjs`(`--lazy`)、`lift/harness/web/pack.args`、`lift/crates/fe-build/tests/plan/bundle.txt`
- `app/src/engine/lazy-file.ts`(起動後の取得の待ち)、`app/src/engine/xmod.ts`、`lift/harness/two/xmod_glue.c`、`lift/harness/two/main_two.c`(`fle_xmod_load`)
- `app/src/plugins/plugin-client.ts`、`app/src/plugins/plugin-worker.ts`
- `lift/harness/two/wrp1_vst3.c`(`fle_realmod_pre_attach` と `wrp1_after_image_load`。#{{026}} の hook)

## やること

1. #{{123}} と #{{124}} が閉じていることを確かめる。閉じていなければ、報告に書いて止まる。
2. OTT の wasm を、`lift/out-app/plugins/ott/vst-ext.wasm` から、起動後の取得に入れる。`lift/harness/web/pack.args` に `--lazy-file lift/out-app/plugins/ott/vst-ext.wasm=plugins/ott.wasm` を足し、`lift/crates/fe-build/tests/plan/bundle.txt` を新しい引数に合わせる(#{{015}} の先例)。名前は `ott.wasm`。製品の build の一部として既定で入る。オプトインは作らない。
3. 既定の経路を作る(案 A か B)。guest が OTT の VST3 を読む時に、この wasm を使う。wasm は起動後に届くので、guest の読みは、届くまで待つ形にする(`lazy-file.ts` の待ち)。どちらの案にしたか、理由、変えたパスを報告に書く。
4. 到着時刻を測る: 起動から、`ott.wasm` が届くまでの時間(engine のログの行で)。起動前に取る量(`fe-build bundle` の出力の、起動前の合計)を、変更の前後で報告に書く。
5. `tools/plugin-run/measure.mjs ott`(#{{124}} の契約)で、1 ブロックの音声を処理する。
6. boot の後に、FL のプラグインの検索から OTT を読み込んだ時の engine のログ(`image-read` などの行)を、報告に貼る。貼れない場合は「未確認」と書く。

## 受け入れ

```accept
bash lift/harness/web/mt1/plugin-worker/build.sh ott "$PWD/lift/out-app/external" "$PWD/lift/out-app/plugins/ott" && npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false?0:1)' "$RUNNER_TEMP/boot.json" && node -e 'const m=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(Array.isArray(m.lazy)&&m.lazy.some((f)=>f.name==="ott.wasm")?0:1)' "$PWD/lift/out-app/bundle/raw/fl-Oz/manifest.json" && node tools/plugin-run/measure.mjs "$PWD/lift/out-app/plugins/ott" ott > "$RUNNER_TEMP/ott.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8").trim());process.exit(r.error===null&&r.instantiated===true&&r.blockFrames>0&&r.nonSilent===true?0:1)' "$RUNNER_TEMP/ott.json"
```

判定の基準: `ott.json` の 1 行の生の値で、`error` が null、`instantiated` が true、`blockFrames` が 0 より大きい、`nonSilent` が true。加えて、`boot.json` が engine-idle・error null・standIn false で、`manifest.json` の lazy に `ott.wasm` が在る。報告には、到着時刻と、起動前に取る量の変化、既定の経路の案(A か B)を書く。

## 作業の進め方

- ビルド(engine と wasm)は約 35 分。フォアグラウンドで、シェルツールの `timeout` 引数に 5400000 を渡して待つ。`sleep`・`timeout` をコマンドの先頭に書かない。背景のビルドを `tail --pid`・`wait` などで待たない。
- 1 回の試行で終わらないことが多い。最初に `date -u` で開始を記録し、4 時間 30 分を過ぎたら、受け入れが通らなくても作業ツリーを commit し、報告を書いて終える。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には「runner の値。この機械とは別枠」と書き、nproc と loadavg を添える。
- 実測と推定を分ける。「達成」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。
- 報告に書く: 既定の経路の案(A か B)と理由、wasm の大きさ、到着時刻、起動前に取る量の変化、measure の JSON の行(そのまま貼る)、変えたパス(lift/・app/ を含む)。

## 制約

- 製品に新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足さない。オプトインを作らない。
- Python を製品と本番の経路に入れない。
- ベンダー名と Cloudflare の識別子を書かない。インストーラの名前に含まれる会社名を、そのまま書かない。
- 外部のインストーラを実行しない。
- 一時ファイルは `.tmp-*` に置き、終わったら消す。sleep・timeout・nohup・末尾の `&` は使わない。
- 製品の作業は Actions で行う。この機械で製品のビルドや測定をしない。

## 他のタスクとの関係

- 前提: #{{123}}(基盤 1a)、#{{124}}(基盤 1b)。#{{040}}(音声への圧迫の解消)、#{{025}}(入力)、#{{026}}(既定の link)、#{{015}}(1 プラグイン 1 wasm の型)。
- #{{027}}・#{{029}} と同じ `lift/harness/web/pack.args` を使う。衝突したら、作業の最後に `git pull --no-rebase --no-edit` で main を取り込み、衝突を直し、受け入れをもう一度通す。

## 前回の試行の記録(下書きの履歴)

- run 45253143948(2026-11-23 19:52Z)で落ちた。`failed` を外し、`blocked_by` に #{{124}} を足して、基盤の完了まで起動しない形にした(PM の決定)。
