---
draft: "026"
title: "VST3 の link を既定で有効にする(オプトインを外す。台帳 L-002)"
type: task
milestone: "M6 外部プラグイン"
parent: "024"
blocked_by: []
labels: ["task", "plugin-external", "priority-p1"]
size: 6
needs_owner: false
---

## 目的

台帳 L-002: オプトイン・オプトアウトの処理を作らない。既定で全部有効。VST3 の link が、今は「opt-in」の注釈つきで、製品の build に入っていない。この Issue は、VST3 の経路を既定で有効にし、外部プラグインが読み込める状態を作る。

## 規模

規模: 大(想定 6 時間。ビルドを約 5 回回す規模)

## 背景

- 現状(旧の読み。pm-work/findings/survey-c5-external-plugins.md の 3・4 節。読みによる):
  - `lift/harness/two/wrp1_vst3.c` 1〜4 行の注釈: 「OPT-IN: linked only into the wrp1 native build; nothing else defines the two hooks」。この file は、Serum 2 を読むための TLS・IAT・TLS callback の処理。
  - `lift/runtime/fle_realmod.h` 5〜7 行: 「Without a table (the default ...) no real module exists」。85〜88 行: 「OPT-IN: a link that supplies no path table behaves as before」。
  - `lift/runtime/fle_realmod.c` 172〜177 行: `fle_realmod_pre_attach` と `fle_realmod_pre_thread` の弱い参照。定義が無ければ何もしない。
  - `lift/runtime/fle_ole.c` 171〜172 行: CSIDL の弱い参照。定義が無ければ止まる。
  - `lift/runtime/fle_modules.h` 82〜84 行: `-DFLE_GMHEX_RETURNS` の注釈(opt-in)。
- 製品の build の入力(`ci/extract-dlls.tsv`・`lift/harness/web/pack.args`・`lift/harness/hyb/hyb-args.tsv`)に、serum・vst3 の行は無い(survey-c5 の 3 節)。
- 作業の段の build は、`lift/out-app/gen/realmod.c`(real module の表)を使う(`lift/harness/two/harness-inputs.txt` の 10 行)。表の中身は未確認。

## 読む物

- `lift/harness/two/wrp1_vst3.c`(1〜4 行)
- `lift/runtime/fle_realmod.h`(5〜7 行、85〜88 行)、`lift/runtime/fle_realmod.c`(172〜177 行)
- `lift/runtime/fle_ole.c`(171〜172 行)
- `lift/runtime/fle_modules.h`(82〜84 行)
- `lift/harness/two/harness-inputs.txt`(10 行)、`lift/crates/fe-lift/src/gen_all.rs`(279〜288 行: real module の表の生成)
- `lift/crates/fe-build/src/main.rs`(gen 段と engine の build)
- `lift/harness/vst3a/main_vst3a.c`(1〜6 行。Serum 2 の native harness)

## やること

1. 現状を確かめる: 既定の build に、VST3 の link が入っているかを、生成物と起動の記録で確かめる。結果を報告に書く。
2. 既定で link されるように、build の入力(表と段)を変える。製品の `#ifdef` や環境変数は足さない。注釈の「opt-in」の語も直す。
3. 検査の道具 `tools/vst3-link/check.mjs` を作る。bundle の生成物に、VST3 の link の印が在るかを確かめ、JSON の 1 行を出す: `{"linked","hookSymbols","error"}`。
4. 起動確認が通ること(boot-bundle)。

## 受け入れ

```accept
npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false?0:1)' "$RUNNER_TEMP/boot.json" && node tools/vst3-link/check.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/vst3.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8").trim());process.exit(r.error===null&&r.linked===true?0:1)' "$RUNNER_TEMP/vst3.json"
```

判定の基準: 既定の build の生成物に VST3 の link が入っており、起動確認が通ること。`opt-in` の語が、VST3 の経路の注釈から消えていること(報告に grep の結果を貼る)。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には「runner の値。この機械とは別枠」と書き、nproc と loadavg を添える。
- 実測と推定を分ける。「達成」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。
- 報告に書く: 変更前の link の有無、変えたパス(lift/・app/ を含む)、既定の build の差、`opt-in` の語の残り(grep の結果)。

## 制約

- 製品に新しい環境変数・#ifdef・URL パラメータ・ビルド変数を足さない(L-002)。オプトインを作らない。
- ベンダー名と Cloudflare の識別子を書かない(L-005・L-006)。
- 製品の build を変えるので、main の deploy の仕組みが動く。報告に書く(AGENTS.md の運用)。
- sleep・timeout・nohup・末尾の `&` は使わない。一時ファイルは `.tmp-*` に置き、終わったら消す。

## 他のタスクとの関係

- 027・028・029 の前提。
- `lift/` の real module の表を変えるので、015(Fruity Balance)と 020(抽出の表)と同じ build の段を変えうる。衝突したら、作業の最後に `git pull --no-rebase --no-edit` で main を取り込み、衝突を直す。
