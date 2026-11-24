---
draft: "154"
title: "FL を英語以外の言語で起動できる: ILSvg_x64.dll を real module にし、言語のファイル(.moe・.svg)を seed にする"
type: Task
milestone: "M4 FL に必要な物の配布"
parent: "012"
blocked_by: ["125","142"]
labels: ["resources","verify"]
size: L
priority: High
project: signature-studio/3
---

## 目的

依頼者の指摘(2026-11-24。原文): 「言語を切り替えると再起動を求められますが(これ自体は正常な挙動)、再起動しても設定が反映されていません。」

#{{142}} で、言語の選択(レジストリの値)は、ページの再読込をまたいで保持されるようになった。しかし、英語以外の言語で起動すると、engine が止まる。この Issue の目標は 2 つ。
1. 英語以外の言語(配信の 7 言語: de・es・fr・ja・ko・vi・zh)で起動して、engine が止まらず `engine-idle` に届くこと。
2. 選んだ言語で、画面の言語の欄が英語の時と異なる画素になること(言語が画面に反映されること)。

## 背景(#{{142}} の実測。run 45364912379)

- 言語の選択の保存は動く。保存先は `HKCU\Software\<ベンダー>\Shared\Language` の値 `Program language`(REG_SZ)。再読込の後の起動で、`RegQueryValueExW "Program Language" -> found type=1 size=6 value="es?"` が読まれた。
- 英語以外の言語で起動すると、engine が止まる。`es`・`vi`・`fr`・`ja` の 4 回とも同じ止まり方: `missing-path: LoadLibraryW 126 "ILSvg_x64.dll"` の後に `fle: indirect call to 0 is not a known function entry`。
- 原因(実測): この bundle は `ILSvg_x64.dll` を lift していない。インストーラの一覧(`inputs/fl-installer-files.tsv`)には `Shared\ILSvg_x64.dll`(1,504,256 B。SHA-256 は一覧の値)と、各言語の `System\Languages\<code>.moe`・`.svg` がある(en は `.svg` のみ)。
- 言語のファイルの seed: リポジトリの seed 表(`lift/harness/seeds/vfs-seed-idle1.tsv`)には、`System\Languages\` の 7 言語の `.moe`・`.svg` の行が在る(#1345。ホスト側は `resources/System/Languages/` の行)。#{{142}} の実行の記録では、bundle の seed に入ったのは英語の `en.svg` だけだった。なぜ入らなかったか(`resources` の復元と、seed の作り)は、この Issue で確かめる。
- #{{142}} の安全策(暫定): エンジンが異常に終わった回は、保存された木を捨てる(`app/src/engine/worker.ts` の `closeRegState`)。この Issue が英語以外の言語の起動を直せば、安全策が要るかを報告に書く。

## 読む物

- `inputs/fl-installer-files.tsv`(`Shared\ILSvg_x64.dll`・`System\Languages\` の行)
- `ci/resources-prefixes.txt`(`System\Languages\` の接頭辞)、`.github/workflows/agent.yml` の resources の段(言語の取り出しは、`resources` のラベルの作業だけで行われる)
- `lift/harness/seeds/vfs-seed-idle1.tsv`(`System\Languages\` の行)
- `lift/plugins/`(#{{125}} の形。`lift/plugins/<名前>/plugin.tsv` を 1 つ足すと、生成の段が本体の wasm と配置を作る)
- `app/src/bundle/manifest.ts`(lazy の配列の `maxItems`)
- `tools/persist/measure.mjs`(#{{142}} の道具)、`tools/smoke/welcome-flow.mjs`

## やること

0. 作業ツリーの確認。`.prior-work.md` があれば最初に読む。
1. `ILSvg_x64.dll` を real module にする。`lift/plugins/ilsvg/plugin.tsv` を 1 つ足す(#{{125}} の形)。列は `key`・`guest`・`role`・`sha256`・`size`・`hook`・`args`。`guest` は `Shared\ILSvg_x64.dll`、`role` は `本体`、`sha256` と `size` は `inputs/fl-installer-files.tsv` の値(`size` は 1504256)、`hook` は `none`、`args` は空。`key` は、#{{125}} の規則(`<種別>.<フォルダ>.<DLL の名>` の小文字)に従う。共有の表(`modules.tsv`・`pack.args` など)は手で編集しない。
2. 言語のファイル(配信の 7 言語 de・es・fr・ja・ko・vi・zh の `.moe`・`.svg`)を seed にする。起動の後の取得(lazy)にする。lazy の上限は `app/src/bundle/manifest.ts` の `maxItems`(128)。今の lazy の件数は #1366 の記録で 75 件。14 件を足して上限内に収まるかを数える。上限を超えるなら、`app/` の上限を上げるか、言語のファイルを 1 つのパック(`pack-assets`)にまとめる。実測で選び、報告に書く。言語のファイルは、インストーラの一覧(`System\Languages\`)から生成し、コードに言語の名前を定数として書かない。
3. なぜ bundle の seed に言語のファイルが入らなかったか(#{{142}} の記録の `en.svg` だけ)を、生成した seed の表で確かめ、直す。
4. 報告に、`engine.wasm` の大きさの増分、`ILSvg_x64.dll` の wasm の大きさ、lazy の件数、選んだ案と理由を書く。

## 受け入れ

```accept
npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false?0:1)' "$RUNNER_TEMP/boot.json" && node tools/persist/measure.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/persist.jsonl" && node -e 'const L=require("fs").readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l)).filter((x)=>"language" in x);const want=["de","es","fr","ja","ko","vi","zh"];const ok=(r)=>r.error===null&&r.restartPrompted===true&&r.state==="engine-idle"&&r.standIn===false&&r.languageKept===true&&r.elapsedMs<=12500&&r.preBootBytes<=314368711;process.exit(L.length===7&&want.every((c)=>L.some((r)=>r.language===c))&&L.every(ok)?0:1)' "$RUNNER_TEMP/persist.jsonl" && node tools/smoke/welcome-flow.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/flow.jsonl" && node -e 'const fs=require("fs");const L=fs.readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const F=(f)=>L.find((r)=>r.flow===f);const ok=(r)=>!!r&&r.error===null&&r.state==="engine-idle"&&r.standIn===false&&r.trapped===false&&r.hostErrors===0&&r.opened===true;process.exit(L.length===4&&["A","B","C","D"].every((f)=>ok(F(f)))&&F("B").welcomeClosed===true&&F("D").welcomeClosed===true?0:1)' "$RUNNER_TEMP/flow.jsonl" && npm test --prefix app
```

判定の基準:
- `tools/persist/measure.mjs` の出力は 7 行(言語ごとに 1 行。`language` は de・es・fr・ja・ko・vi・zh の全て。各 1 回。DPR 1)。
- 各行で、`error` が null、`restartPrompted` が true(言語を変えると、再起動の案内が出る)、`state` が `engine-idle`(再読込の後に止まらない)、`languageKept` が true(画面の言語の欄の画素が、英語の時と異なる。前回の画素の比べ方と同じ)、`standIn` が false。
- `elapsedMs` が 12,500 以下。`preBootBytes` が 314,368,711 以下(言語のファイルは起動の後の取得。起動の前に取る量を増やさない)。
- 流れ A〜D(`tools/smoke/welcome-flow.mjs`)が通る。単体テスト(`npm test --prefix app`)が通る。
- 報告に書く: `engine.wasm` の大きさの増分、`ILSvg_x64.dll` の wasm の大きさ、lazy の件数、seed に言語のファイルが入らなかった原因(生成した seed の表の確認の行)、選んだ案と理由。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には「runner の値。この機械とは別枠」と書き、nproc と loadavg を添える。
- 実測と推定を分ける。「達成」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。出力の行を付ける。
- 報告に、安全策(`closeRegState`)を残すか外すかを書く。

## 制約

- 製品に、利用者向けの新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足さない。
- 言語の名前や値を、runtime・app・テストに、定数・表・分岐として入れない。言語のファイルの一覧は、インストーラの一覧から生成する。測定の道具の入力(7 言語の一覧)は例外。
- ベンダー名と Cloudflare の識別子を書かない。
- sleep・timeout・nohup・末尾の `&` は使わない。一時ファイルは `.tmp-*` に置き、終わったら消す。
- Python は製品に使わない(調査と検証にだけ使う)。
- 製品の作業は Actions で行う。この機械で製品のビルドや測定をしない。

## 作業の進め方

- 最初に `date -u` で開始の時刻を記録する。4 時間 30 分を過ぎたら、受け入れが通らなくても、作業ツリーを commit し、報告を書いて終える(ジョブの上限は 5 時間 50 分)。
- ビルドは約 35 分。フォアグラウンドで直接実行し、シェルツールの `timeout` 引数は最大の 5400000 にする。背景で起動して待たない。
- 作業の最後に、受け入れが通らなくても、作業ツリーを commit する。

## 他のタスクとの関係

- blocked_by: #{{125}}(プラグインの 1 ディレクトリの形)と #{{142}}(言語の選択の保存)。両方が閉じた後に起動する。
- #{{142}} の安全策(異常終了の回は保存を捨てる)は、この Issue の結果で要否を決める。
- #1366(lazy の上限を 128 にした)の件数に影響する。
