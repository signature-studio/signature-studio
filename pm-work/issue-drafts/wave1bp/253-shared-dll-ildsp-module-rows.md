---
draft: "253"
title: "共有の DLL の行を足す: `ildsp_x64.dll` を実モジュールにし、`Shared\\module\\` の DLL の出所を確かめる(出所が無ければ行は足さない)"
type: "Task"
milestone: "M5 内蔵のエフェクトとインストゥルメント"
parent: "021"
blocked_by: []
labels: ["plugin-builtin"]
size: "M"
priority: "High"
project: "signature-studio/3"
---

## 目的

内蔵のエフェクトの束(#1434 の Distructor、#1436 の Emphasizer と Frequency Shifter)の本体は、インストーラの `Shared\` の DLL を `LoadLibraryW` で読もうとする。木に無いので、`missing-path` で止まる。この Issue は、次の 2 つをする。

1. `Shared\ildsp_x64.dll` を実モジュールとして木に入れる行を足す。
2. `Shared\module\` の 2 本の DLL(`module_x64.dll`・`FruityPlug_x64.dll`)の出所を、インストーラの一覧で確かめる。出所が在る物だけ行を足す。出所が無ければ、行は足さず、探した範囲を報告する。

`Shared\ILSvg_x64.dll` は #1395 が扱うので、この Issue では触らない。

この Issue は `ci/` に触れる。判定が通って push されると、main の e2e が走る。報告に書く。

## 背景

- 観測(runner の値。この機械とは別枠):
  - `ildsp_x64.dll`: #1436 の Emphasizer と Frequency Shifter の本体が読む。`missing-path: LoadLibraryW 126 "ildsp_x64.dll"` のあと、呼び出し先が 0 になり trap する。
  - `Shared\module\module_x64.dll` と `Shared\module\FruityPlug_x64.dll`: #1434 の Distructor が読む。`missing-path: LoadLibraryW 126` の後、`MessageBoxW` の表示を経て、`indirect call to 0` で止まる。
  - `Shared\ILSvg_x64.dll`: #1448 の Fruity Scratcher が読む。#1395 の範囲。
- インストーラの一覧(`inputs/fl-installer-files.tsv`):
  - `Shared\ildsp_x64.dll`: 4197376 B・SHA-256 `f4b23b0f93ee9119ca21ad5255c2e239f9bb78d6664289e9888ddcab082faea2`・kind dll。`pm-work/findings/shared-dll-inventory.md` では、分類は (b)(まれ)。
  - `Shared\module\` の行は無い。`FruityPlug_x64.dll` の行も無い(この Issue の起票の時に確かめた)。`module_x64.dll` と同じ名前の行は `Plugins\Fruity\Effects\Distructor\module\Module_x64.dll` の 1 行だけで、guest の綴りが違う。同じ物と見なす根拠は無い。
- 行の形:
  - `ci/extract-dlls.tsv`: `name<TAB>destination<TAB>sha256`。destination は `lift/out-plugins/` の下。`ILSvg_x64.dll` の行(#1395)が見本。
  - `lift/plugins/<名前>/plugin.tsv`: 補助の行。`lift/plugins/reverb/plugin.tsv` が見本(`Shared\Reverb_x64.dll`・role は補助・key は bare 名・hook は none・args は `--pdata`)。この見本は、`ci/extract-dlls.tsv` の行を持たない。
  - `app/scripts/lib/plugin-rows.mjs` の `pluginRows()` は、`lift/plugins/*/plugin.tsv` の行から、抽出の表の行を作る。`ci/extract-dlls.tsv` の行と二重にならないかは、作業者が読んで確かめる。
- 抽出物の入り方(推定。`.github/workflows/agent.yml` の job の順で確かめた): 作業の job は、抽出物を、`ci/extract-dlls.tsv` と `lift/plugins/*/plugin.tsv` を含む鍵のキャッシュから取る(`fail-on-cache-miss`)。その鍵は、job の始めの HEAD の内容で決まる。この変更で足す DLL は、この job の抽出物には入らない見込み。確認できない。見込みどおりなら、`fe-build` が DLL を読めずに止まる。

## 読む物

- `inputs/fl-installer-files.tsv`(`Shared\` の行。`grep -n '^Shared' inputs/fl-installer-files.tsv`、`grep -in 'module' inputs/fl-installer-files.tsv`)
- `ci/extract-dlls.tsv`(注釈の行の形と、`ILSvg_x64.dll` の行)
- `lift/plugins/reverb/plugin.tsv`(補助の行の見本)
- `app/scripts/lib/plugin-rows.mjs`(`pluginRows()`)・`app/scripts/extract-fl.mjs`(`--dlls` の読み)
- `tools/place-plugin-dlls.mjs`・`ci/plugin-builtin-prefixes.txt`(接頭辞の外の DLL は skipped になり、`ci/extract-dlls.tsv` が扱う)
- `lift/crates/fe-build/src/plugins.rs`(補助の行の検査と、`host_file()`)
- `.github/workflows/agent.yml`(「抽出物を取る」の step の鍵)
- `pm-work/findings/shared-dll-inventory.md`(`Shared\ildsp_x64.dll` の分類の行)

## やること

1. `ildsp_x64.dll` の行を足す。
   - `ci/extract-dlls.tsv` に 1 行足す: name `ildsp_x64.dll`・destination `lift/out-plugins/ildsp_x64.dll`・SHA-256 `f4b23b0f93ee9119ca21ad5255c2e239f9bb78d6664289e9888ddcab082faea2`。日付と Issue 番号の注釈を、既存の行の形に倣って 1 行足す。
   - `lift/plugins/ildsp/plugin.tsv` を新しく作る。見出しは 8 列(`key`・`guest`・`role`・`sha256`・`size`・`hook`・`args`・`omit`)、区切りは TAB。行は 1 つ: key `ildsp_x64.dll`・guest `Shared\ildsp_x64.dll`・role `補助`・sha256 上の値・size `4197376`・hook `none`・args `--pdata`・omit `-`。注釈は `lift/plugins/reverb/plugin.tsv` に倣う。
   - `ci/extract-dlls.tsv` と `lift/plugins/ildsp/plugin.tsv` の両方が抽出の表に入ったときに、同じ DLL の行が二重にならないかを、`pluginRows()` と `extract-fl.mjs` を読んで確かめる。二重になるなら、どちらか 1 つにし、理由を報告に書く。
2. `Shared\module\` の 2 本の DLL の出所を確かめる。`inputs/fl-installer-files.tsv` の全行で、`module_x64.dll`・`FruityPlug_x64.dll` と同じ名前、および `Shared\module\` を含む綴りを探す。`Data\`・`Plugins\` の下も同じように探す。見つかれば、同じ形で行を足す(SHA-256 と大きさは一覧の値)。見つからなければ、行は足さない。探した範囲(一覧の行数・探した名前)と結果を報告に書く。
3. `Shared\ILSvg_x64.dll` の行と、`lift/plugins/ilsvg/` には触れない(#1395)。

## 受け入れ

```accept
node -e 'const fs=require("fs");const T=(f)=>fs.readFileSync(f,"utf8").split("\n").map((l)=>l.split("\t"));const H="f4b23b0f93ee9119ca21ad5255c2e239f9bb78d6664289e9888ddcab082faea2";const ci=T("ci/extract-dlls.tsv").some((c)=>c[0]==="ildsp_x64.dll"&&c[1]==="lift/out-plugins/ildsp_x64.dll"&&c[2]===H);const ins=T("inputs/fl-installer-files.tsv").some((c)=>c[0]==="Shared\\ildsp_x64.dll"&&c[1]==="4197376"&&c[2]===H);const pl=T("lift/plugins/ildsp/plugin.tsv").some((c)=>c[0]==="ildsp_x64.dll"&&c[1]==="Shared\\ildsp_x64.dll"&&c[2]==="補助"&&c[3]===H&&c[4]==="4197376");process.exit(ci&&ins&&pl?0:1)' && npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && node tools/place-plugin-dlls.mjs && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false&&r.elapsedMs<=20000?0:1)' "$RUNNER_TEMP/boot.json" && node tools/smoke/welcome-flow.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/flow.jsonl" && node -e 'const fs=require("fs");const L=fs.readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const F=(f)=>L.find((r)=>r.flow===f);const ok=(r)=>!!r&&r.error===null&&r.state==="engine-idle"&&r.standIn===false&&r.trapped===false&&r.hostErrors===0&&r.opened===true;process.exit(L.length===4&&["A","B","C","D"].every((f)=>ok(F(f)))&&F("B").welcomeClosed===true&&F("D").welcomeClosed===true?0:1)' "$RUNNER_TEMP/flow.jsonl" && npm test --prefix app && cargo test --locked --manifest-path lift/Cargo.toml -p fe-build && bash guard/scan.sh
```

判定の基準:
- `ci/extract-dlls.tsv` に `ildsp_x64.dll` の行が 1 つあり、destination が `lift/out-plugins/ildsp_x64.dll`、SHA-256 が上の値。`inputs/fl-installer-files.tsv` の `Shared\ildsp_x64.dll` の行(大きさ 4197376・同じ SHA-256)と一致する。`lift/plugins/ildsp/plugin.tsv` の行が、role 補助・size 4197376 で、同じ SHA-256。
- `place-plugin-dlls` の JSON 行(`missing`・`mismatch` の数)を報告に貼る。
- 起動確認(`boot.json`)が engine-idle・standIn が false・error が null・elapsedMs が 20,000 以下。
- 流れ A〜D の 4 行が、`error` が null・engine-idle・`standIn` が false・`trapped` が false・`hostErrors` が 0・`opened` が true。B と D は `welcomeClosed` が true。
- `npm test --prefix app` と `cargo test -p fe-build` が通る。
- `bash guard/scan.sh` の指摘が 0 件。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には「runner の値。この機械とは別枠」と書く。
- 報告に書く: (a) 足した行(`ci/extract-dlls.tsv` の 1 行と `lift/plugins/ildsp/plugin.tsv` の行)と、二重にならない形の判断、(b) `Shared\module\` の 2 本について、探した範囲と結果(行が在ったか、無かったか)、(c) `place-plugin-dlls` の JSON 行、(d) この job の木に `lift/out-plugins/ildsp_x64.dll` が在ったかどうか(`ls` の結果)、在らなければ、`fe-build engine` の出力の末尾、(e) 起動確認の `elapsedMs`、流れ A〜D の 4 行、(f) `cargo test -p fe-build` の `test result` の行、(g) `bash guard/scan.sh` の結果。
- 「達成」「直った」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。出力の行を付ける。

## 制約

- 製品に、利用者向けの新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足さない。
- Python を使わない(調査と検証にだけ使う)。OCR を使わない。
- 禁止されているのは、AI のベンダー名と Cloudflare の識別子だけ。FL の製品名・インストール先の名前・変数名は書いてよい。
- sleep・timeout のコマンド・nohup・末尾の `&` は使わない。一時ファイルは `.tmp-*` に置き、終わったら消す。
- `git add` は、変えたファイルを 1 つずつ指定する(`git add -A`・`git add .` は使わない)。
- `ci/` に触れるので、判定が通って push されると、main の e2e が走る。報告に書く。
- `Shared\ILSvg_x64.dll` と `lift/plugins/ilsvg/` に触れない(#1395)。

## 作業の進め方

- 作業の最初に `date -u` を記録する。`.prior-work.md` が在れば、最初に読む。
- ビルド(`fe-build engine`)は、1 回に数十分(観測: 約 17 分から 38 分)。ビルドは 1 回にする。長いコマンドはフォアグラウンドで直接、シェルツールの `timeout` 引数に 5400000 を渡す。背景で起動して待たない。
- 作業を始めてから 3 時間 15 分を過ぎたら、受け入れが通らなくても、作業ツリーを commit し、報告を書いて終える(判定に約 2 時間掛かる)。
- 受け入れが通らなくても、作業の最後に作業ツリーを commit する。未 commit の変更が残ると、判定は失敗する。

## 他のタスクとの関係

- blocked_by: なし。
- #1434 と #1436 は、この Issue を前提にする(#1448 は #1395 を前提にする)。
- 同じ束の #{{251}}(プラグインを足すと既存が止まる)と #{{252}}(runtime に無い import)と、同じファイルを変えることがあり得る。衝突は、後から入る方が直す。
