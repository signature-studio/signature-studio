---
draft: "257"
title: "共有の DLL 4: Elastique_x64.dll を実モジュールにして、既定の曲を開いた時の『Problems loading the project』の箱を消す"
type: "Task"
milestone: "M4 FL に必要な物の配布"
parent: "012"
blocked_by: ["227"]
labels: ["resources", "plugin-builtin", "verify"]
size: "M"
priority: "High"
project: "signature-studio/3"
---

## 目的

既定の曲(welcome-default)を開くと、今は「Problems loading the project」の箱が出る。engine は止まらないが、利用者には警告が出る。この Issue は、箱の原因と見られる `Shared\elastique_x64.dll`(Elastique)を実モジュールにして、箱を消す。

## 背景

- #1513(WavPack と oggio の実モジュール化)の後、既定の曲を開いた時の見出しが `Problems loading the project` になった(#1513 の報告の実測。起動後の既定の曲の行)。基準は `FL Studio 2026`(#1511 の受け入れ)。
- 同じ操作で、WavPack の要求の後に、`Shared\Elastique_x64.dll` の存在の確認(`GetFileAttributesW`)が 2 を返す(仮想ボリュームに載っていない)要求が新しく出た(#1513 の報告)。`pm-work/findings/shared-dll-inventory.md` の elastique の行も同じ。
- **因果は確かめていない。** この DLL の確認の失敗が箱の原因かは、この Issue で確かめる。確かめられなければ、箱の本文から見た欠けている物を報告に書く。
- 一覧の値(`inputs/fl-installer-files.tsv`): `Shared\elastique_x64.dll`(一覧の綴りは小文字。照合は大文字・小文字を区別しない)・4,997,632 B・依存は ws2_32.dll と kernel32.dll。SHA-256 は、この行の値を使う(本文には書かない)。
- `ci/extract-dlls.tsv` には、この DLL の行が 2 つ在る。`lift/out-sampledec/elastique_x64.dll`(b2d2 が読む。残す)と、`lift/out-plugins/elastique_x64.dll`(この Issue の plugin.tsv が main に入ったら外す。#1513 で足した)。`spec/workflow.test.sh` の r11 の検査が、この 2 行を見ている。
- 実モジュールの見本: `lift/plugins/lameenc/plugin.tsv`(補助。args は `--pdata --reloc-seeds --rva-tables` と `rva-tables.txt`。#1512・#1518)。

## 入れ方

- `lift/plugins/elastique/plugin.tsv` を 1 つ足す。見出しは 8 列(`key`・`guest`・`role`・`sha256`・`size`・`hook`・`args`・`omit`)、区切りは TAB。key は `elastique_x64.dll`、guest は `Shared\elastique_x64.dll`、role は `補助`、sha256 と size は一覧の同じ行の値。
- role を `補助` にする理由: `Shared\` の DLL は Plugin database の項目ではない。入口(export の `CreatePlugInstance`)を持たない DLL は補助。`fe-pe` で export を確かめ、報告に書く。
- hook・args・omit は手で推測しない。`fe-pe`・`fe-lift` の結果から決め、理由を報告に書く。
- 同じ commit で、`ci/extract-dlls.tsv` から `lift/out-plugins/elastique_x64.dll` の行を外す(外さないと、plugin.tsv の行と二重に抽出される)。そして `spec/workflow.test.sh` の r11 の elastique の検査を、out-sampledec 1 行・out-plugins 0 行(期待値 `1/1/0`)に直す。

## 前提

- #1513 が閉じて main に入っている(blocked_by)。WavPack の補助の行(`lift/plugins/wavpack/plugin.tsv`)が、この Issue と同じ形で通っている。

## 受け入れ

```accept
test -f lift/plugins/elastique/plugin.tsv && test "$(awk -F'\t' '$2 == "lift/out-plugins/elastique_x64.dll"' ci/extract-dlls.tsv | wc -l)" -eq 0 && node tools/place-plugin-dlls.mjs && npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false&&r.elapsedMs<=20000?0:1)' "$RUNNER_TEMP/boot.json" && node tools/smoke/welcome-flow.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/flow.jsonl" && node -e 'const L=require("fs").readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const F=(f)=>L.find((r)=>r.flow===f);const ok=(r)=>!!r&&r.error===null&&r.state==="engine-idle"&&r.standIn===false&&r.trapped===false&&r.hostErrors===0&&r.opened===true;process.exit(L.length===4&&["A","B","C","D"].every((f)=>ok(F(f)))&&F("B").welcomeClosed===true&&F("D").welcomeClosed===true&&/^FL Studio 2026/.test(F("D").title)?0:1)' "$RUNNER_TEMP/flow.jsonl" && npm test --prefix app && cargo test --locked --manifest-path lift/Cargo.toml -p fe-build && bash guard/scan.sh && test -z "$(git diff --name-only origin/main...HEAD -- . ':!lift/plugins' ':!ci/extract-dlls.tsv' ':!spec/workflow.test.sh' ':!tools/smoke/welcome-flow.mjs')"
```

判定の基準:
- 起動の確認(`boot.json`): engine-idle、`standIn` が false、`elapsedMs` が 20,000 以下。
- 流れ A〜D の 4 行: `error` が null・`state` が engine-idle・`standIn` が false・`trapped` が false・`hostErrors` が 0・`opened` が true。B と D は `welcomeClosed` が true。
- 流れ D の見出し(`title`)が `FL Studio 2026` で始まる。`Problems loading the project` ではない。`opened` は、箱の見出しでも true になる(#1513 の報告の実測)ので、見出しを見る式が要る。流れの出力に `title` の欄が無ければ、`tools/smoke/welcome-flow.mjs` の出力に、終わりの見出し(`end.title`)を `title` として足す。既定の動作で、オプションは作らない。
- `ci/extract-dlls.tsv` に `lift/out-plugins/elastique_x64.dll` の行が無く、`lift/out-sampledec/elastique_x64.dll` の行は在る。
- `npm test --prefix app` と `cargo test --locked --manifest-path lift/Cargo.toml -p fe-build` が通る。`bash guard/scan.sh` の指摘が 0 件。
- 差分: `lift/plugins/`・`ci/extract-dlls.tsv`・`spec/workflow.test.sh`・`tools/smoke/welcome-flow.mjs` の外に変更が無い。
- E2E(`tools/e2e/scenarios/smoke/start-welcome-default.mjs` の notes に `the "Problems loading the project" box was closed` が出ないこと)は、作業ジョブでは走らせない(e2e.yml で走る)。確かめられないなら「確認できない」と書く。

## やること

0. 作業の最初に `date -u` を記録する。`.prior-work.md` が在れば最初に読む。`git status` で作業ツリーを確かめる。
1. 前提を確かめる。`lift/plugins/wavpack/plugin.tsv` が main に在ること。`ci/extract-dlls.tsv` に elastique の 2 行が在ること。
2. `lift/plugins/elastique/plugin.tsv` を足す(上の「入れ方」)。
3. 同じ commit で、表の行を外し、r11 の elastique の検査を直す。
4. `node tools/place-plugin-dlls.mjs` の後で、`fe-build engine`。`indirect call to 0x… is not a known function entry` が出たら、args に `--pdata --reloc-seeds --rva-tables <file>` を足す(#1518 の仕組み。`rva-tables.txt` の作り方は `lift/plugins/lameenc/` と `lift/plugins/wavpack/` に倣う)。build は 1 回にする。
5. `tools/smoke/welcome-flow.mjs` の出力に `title` を足す(上の受け入れの注記)。
6. 流れ D の見出しが `Problems loading the project` のままなら、箱の本文を読み、足りない物を特定する。窓の子の文字は `guestText`(`tools/e2e/lib/common.mjs`)で読む。読み方は `tools/e2e/scenarios/menus/manage-files-order.mjs` に倣う。欠けた物が別の DLL なら、その名前を報告に書いて終える。
7. 受け入れを実行し、出力の行を報告に貼る。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には「runner の値」と書く。
- 報告に書く: (a) plugin.tsv の hook・args・omit と、その決め方(`fe-pe`・`fe-lift` の出力の行)、(b) `fe-build engine` の elastique の行、(c) 流れ A〜D の 4 行と、流れ D の `title`、(d) 箱が出たかどうか、(e) 因果の結論(確かめた・確かめられなかった)、(f) 受け入れの各段の結果、(g) `spec/workflow.test.sh` を作業ジョブで走らせられたか、その結果。
- 「達成」「直った」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。出力の行を付ける。

## 制約

- 製品に、利用者向けの新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足さない。オプトインを作らない。既定の動作として直す。
- Python を使わない。OCR を使わない。
- AI の製品名・ベンダー名と、クラウドの識別子を、ファイルと報告に書かない。
- sleep・timeout のコマンド・nohup・末尾の `&` は使わない。一時ファイルは `.tmp-*` に置き、終わったら消す。
- commit の名義を変えない。commit・タグ・PR に、AI の署名の行を付けない。必要の無いブランチを作らない。
- 製品の作業は Actions で行う。この機械で製品のビルドや測定をしない。

## 作業の進め方

- 作業の開始時刻を `date -u` で記録する。3 時間 15 分を過ぎたら、受け入れが通らなくても、作業ツリーを commit し、報告を書いて終える。
- ビルドは 1 回にする。build は長い(見積りは約 100 分で、確認できない)。長いコマンドはフォアグラウンドで直接、シェルツールの `timeout` 引数に 5400000 を渡して待つ。背景で起動して待たない。
- 受け入れが通らなくても、作業の最後に作業ツリーを commit する。`git add` は 1 つずつ行う。

## 他のタスクとの関係

- blocked_by: #1513(WavPack と oggio)。#1513 が閉じて main に入ってから起動する。両方が `ci/extract-dlls.tsv` と `spec/workflow.test.sh` を触るため、順に流す。
