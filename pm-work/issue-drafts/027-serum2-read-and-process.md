---
draft: "027"
title: "Serum 2: VST3 ホストの上に、固有の対応を足し、1 ブロックの音声を処理する(wasm の大きさを先に見積もる)"
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

課題 C-5: Serum 2 を存在させる。#{{123}}・#{{124}} の VST3 ホスト(OTT で 1 ブロックの音声を処理できるもの)の上に、Serum 2 の固有の対応を足し、既定の経路(オプトイン無し)で、1 ブロックの音声を処理する。1 プラグイン 1 wasm で、起動後に届ける。

## 背景

- 入力: `lift/out-app/external/` の Serum 2 のインストーラ(`ci/artifacts.lock` の 2 行目。1,313,084,056 B)。VST3 本体は 19,236,864 B(`inputs/external-plugins.tsv` の Serum 2 の行。SHA-256 の先頭 501e7bb3…)。
- 前回の試行(run 45253143948。報告 agent-1336)の結果: 差し戻し(受理ではない)。
  - 受け入れの第 3 引数 `$RUNNER_TEMP/external/serum2.exe` は存在しない。作業の段は `$RUNNER_TEMP/external/*` を `lift/out-app/external/` へ移すので、その dir は空になる(`.github/workflows/agent.yml` の 569〜575 行)。受け入れは、`lift/out-app/external/` の入力を `build.sh` が取り出す形に直した。
  - ビルドと起動確認は通った: engine-idle、error null、standIn false。elapsedMs は 11,503(判定の段の実行)と 12,602(作業者の実行)。runner の値。P1(10 秒)を超える。この Issue の範囲外。
  - Serum 2 の wasm は作れず、読み込みも 1 ブロックの処理も観測していない。
  - 止まった理由: 基盤の VST3 ホストの header が無い(`vst_host.h`)。生成の Python(`mk_stubs.py`・`patch_*.py`)が無い。1 プラグイン 1 wasm の段が `fe-build` に無い。道具 `tools/plugin-run/measure.mjs` が無い。
  - 型は在る: 起動後の取得(`pack-assets.mjs --lazy-file`。#{{015}})と、1 プラグイン 1 wasm の分割(`hybrid --separate`。#{{102}})。
- 既存の先例: `lift/harness/vst3a/main_vst3a.c`(Serum 2 の harness。決定 0165)。`lift/harness/two/wrp1_vst3.c`(#{{026}} で既定の build に入った。TLS と IAT の hook。ImageBase 0x8000000 の module に作用する)。`lift/harness/seeds/wrp1-vst3-imports.txt`(Serum 2 の import の表。153 行)。
- 固有の論点(前回の試行で見えたこと。未確認の物は未確認と書く):
  - import は 13 DLL。`d3d11`・`d2d1`・`dwrite`・`dwmapi` など、描画の import が含まれる。
  - `wrp1_vst3.c` の注釈: Serum 2 の ordinal の import 17 件は `fle_unsupported` のまま(呼ばれたら名前のある停止)。
  - wasm の大きさは未測定。1 つの wasm の上限は 1 GB(#{{033}})。

## 読む物

- `lift/harness/vst3a/main_vst3a.c`、`lift/harness/two/wrp1_vst3.c`(1〜12 行)、`lift/harness/seeds/wrp1-vst3-imports.txt`
- #{{123}} と #{{124}} の成果物(基盤。`lift/harness/web/mt1/plugin-worker/build.sh`・`main_vst_ext.c`・`tools/plugin-run/measure.mjs`)
- #{{028}} の成果物(起動後の取得の型)

## やること

1. 最初に、wasm の大きさを見積もる(1 GB の上限)。Serum 2 の VST3 本体(19 MB の PE)から lift した C の大きさ(`fe-lift`)と、emcc の出力の大きさを見積もり、報告の最初に数字で書く。1 GB を超えるなら、分割の方式(1 つの wasm を複数に分ける、または、一部の関数を別の wasm にする)を決めてから作る。決めた方式と理由を報告に書く。
2. #{{123}} の harness を Serum 2 に当てる。`main_vst3a.c` の Serum 2 固有の部分(別の controller、MIDI の note、TLS callback)を、plugin-worker の口に合わせて使う。
3. import の stub: `wrp1-vst3-imports.txt` の行を使う。答えの無い import は名前のある停止にする。ordinal の 17 件は、呼ばれたら止まる形のままにし、呼ばれたかどうかを報告に書く。
4. 1 プラグイン 1 wasm: `lift/harness/web/mt1/plugin-worker/build.sh serum2 <dir> <出力>` で作る(#{{123}} の契約)。出力は `lift/out-app/plugins/serum2/`。
5. 既定の経路: 起動後の取得で届ける。`lift/harness/web/pack.args` に `--lazy-file lift/out-app/plugins/serum2/vst-ext.wasm=plugins/serum2.wasm` を足し、`lift/crates/fe-build/tests/plan/bundle.txt`(fe-build の計画の記録)を新しい引数に合わせる(#{{015}} の先例)。起動前に取る量は増やさない。
6. 道具 `tools/plugin-run/measure.mjs serum2` で、1 ブロックの音声を処理する(#{{124}} の契約)。

## 受け入れ

```accept
bash lift/harness/web/mt1/plugin-worker/build.sh serum2 "$PWD/lift/out-app/external" "$PWD/lift/out-app/plugins/serum2" && npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false?0:1)' "$RUNNER_TEMP/boot.json" && node -e 'const m=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(Array.isArray(m.lazy)&&m.lazy.some((f)=>f.name==="serum2.wasm")?0:1)' "$PWD/lift/out-app/bundle/raw/fl-Oz/manifest.json" && node tools/plugin-run/measure.mjs "$PWD/lift/out-app/plugins/serum2" serum2 > "$RUNNER_TEMP/serum2.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8").trim());process.exit(r.error===null&&r.instantiated===true&&r.blockFrames>0&&r.nonSilent===true?0:1)' "$RUNNER_TEMP/serum2.json"
```

判定の基準: `serum2.json` の 1 行の生の値で、`error` が null、`instantiated` が true、`blockFrames` が 0 より大きい、`nonSilent` が true。加えて、`boot.json` が engine-idle・error null・standIn false で、`manifest.json` の lazy に `serum2.wasm` が在る。

## 作業の進め方

- ビルド(engine と wasm)は約 35 分。フォアグラウンドで、シェルツールの `timeout` 引数に 5400000 を渡して待つ。`sleep`・`timeout` をコマンドの先頭に書かない。背景のビルドを `tail --pid`・`wait` などで待たない。
- 1 回の試行で終わらないことが多い。最初に `date -u` で開始を記録し、4 時間 30 分を過ぎたら、受け入れが通らなくても作業ツリーを commit し、報告を書いて終える。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には「runner の値。この機械とは別枠」と書き、nproc と loadavg を添える。
- 実測と推定を分ける。「達成」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。
- 報告に書く: wasm の大きさの見積もり(数字)、分割の方式(要るなら)、ordinal の import が呼ばれたか、起動後の取得での到着時刻、measure の JSON の行(そのまま貼る)、変えたパス(lift/・app/ を含む)。

## 制約

- 製品に新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足さない。オプトインを作らない。
- Python を製品と本番の経路に入れない。
- ベンダー名と Cloudflare の識別子を書かない。インストーラの名前に含まれる会社名を、そのまま書かない。
- 外部のインストーラを実行しない。
- 一時ファイルは `.tmp-*` に置き、終わったら消す。sleep・timeout・nohup・末尾の `&` は使わない。

## 他のタスクとの関係

- 前提: #{{123}}(基盤 1a)、#{{124}}(基盤 1b)。#{{040}}(音声への圧迫の解消)、#{{025}}(入力)、#{{026}}(既定の link)、#{{015}}(1 プラグイン 1 wasm の型)。
- #{{028}}・#{{029}} と同じ基盤と、同じ `lift/harness/web/pack.args` を使う。衝突したら、作業の最後に `git pull --no-rebase --no-edit` で main を取り込み、衝突を直し、受け入れをもう一度通す。

## 前回の試行の記録(下書きの履歴)

- run 45253143948(2026-11-23 19:52Z)で落ちた。`failed` を外し、`blocked_by` に #{{124}} を足して、基盤の完了まで起動しない形にした(PM の決定)。
