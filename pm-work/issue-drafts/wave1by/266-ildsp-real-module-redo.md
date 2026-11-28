---
draft: "266"
title: "ildsp_x64.dll を実モジュールにし直す: ポインタ経由の入口を全て入口にし、デモの曲を開いても止まらない"
type: "Task"
milestone: "M5 内蔵のエフェクトとインストゥルメント"
parent: "021"
blocked_by: []
labels: ["plugin-builtin"]
size: "L"
priority: "High"
project: "signature-studio/3"
---

## 目的

`Shared\ildsp_x64.dll`(インストーラの `Shared` の下の DSP の DLL。4,197,376 B。SHA-256 `f4b23b0f93ee9119ca21ad5255c2e239f9bb78d6664289e9888ddcab082faea2`)を、実モジュール(木に lift して入る DLL)として入れ直す。入れた後、デモの曲を開いても engine が止まらないことを確かめる。

共有の DLL を実モジュールにする Issue は、その DLL を使う既存の plugin を含む曲(デモ)でも止まらないことを、`smoke/file-open-demo` で確かめる。

今の main には、この DLL の実モジュールは無い(前の試みを取り消した)。

## 背景

- 前の試み(#1539)は、`lift/plugins/ildsp/plugin.tsv` を足して、この DLL を `--pdata`、次に `--pdata --reloc-seeds` で lift した。どちらも、デモの曲を開くと engine が止まった。smoke の `file-open-demo` が 3 表示とも落ち、本番への deploy が止まった。commit `7c9790a1` で取り消した(その前は `5dba7bc8` と `3deea975`)。
- 止まりの観測(runner の値。この機械とは別枠。標準エラーの `fle: indirect call to <値> is not a known function entry (caller <戻り番地>, near <関数>)` の行):
  - `--pdata` で lift した木: 呼び先 `0x12c77d38`(像の基底からの RVA は `0x1b7d38`)。
  - `--pdata --reloc-seeds` で lift した木: 呼び先 `0x12c2bb50`(RVA `0x16bb50`。レジスタ rax は r9 の値)。呼び元は `0x12c6cb3d`(RVA `0x1acb3d`)。像の基底は `0x12ac0000` と見える(観測の番地と RVA の差から逆算)。
  - `--reloc-seeds` を足すと、1 つの番地は解けたが、別の番地で止まった。
- 見立て(推定。確かめる): ildsp は、関数のポインタを実行時に計算する型(DSP の部品の関数の表を、実行時に選んで呼ぶ)と見る。入口が `.pdata` にも `--reloc-seeds` の対象にも無い関数がある。`--reloc-seeds` だけでは、全ての入口は拾えない。`lift/plugins/reverb/seeds.tsv`(#1429)と同じ型の問題と見る。
- 使う側: Emphasis(UI の名は Emphasizer)が `LoadLibraryW "ildsp_x64.dll"` で読む。デモの曲は、この plugin を含む。Frequency Shifter も読むが、その folder は main に無い(#1436 の束)。
- `ci/extract-dlls.tsv` の ildsp の行は、取り消し後も残してある(抽出先 `lift/out-plugins/ildsp_x64.dll`)。

## 読む物

- `lift/plugins/reverb/seeds.tsv`: ポインタの表から拾った関数の seed の見本(#1429)。ヘッダの注釈に、表の型(16 バイトの `{関数, 0}` の表、コードポインタの表)が書いてある。
- `lift/plugins/lameenc/plugin.tsv`・`rva-tables.txt`・`seeds.tsv`: args の形(`--pdata --reloc-seeds --rva-tables <file>`)と、`--rva-tables` の表の形(#1512)。
- `lift/crates/fe-lift/src/main.rs`: `--pdata`・`--reloc-seeds`・`--rva-tables`・`--seeds` の口と、`extra_seeds`。
- `lift/crates/fe-pe/src/main.rs`: `pe-info`・`pe-exports`・`pe-pdata-seeds`(像の基底、セクション、エクスポートの RVA)。
- `lift/runtime/fle_dispatch.h`: `fle_dispatch_stop`(停止の文言と、caller・near の出し方)。
- `lift/crates/fe-build/src/plugins.rs`: `plugin.tsv` の検査、args、`host_file()`、第 2 波の試験(件数と SizeOfImage の合計)。
- `tools/plugin-run/run.mjs`(出力の鍵: `instantiated`・`processed`・`engineStop`)、`tools/e2e/run.mjs`、`tools/e2e/scenarios/smoke/file-open-demo.mjs`。
- `git show 5dba7bc8` と `git show 7c9790a1`: 前の実モジュール化の行と、その取り消し。取り消しの差分の逆が、ildsp の項の出発点。

## やること

1. `lift/plugins/ildsp/plugin.tsv` を足す(補助。見出しは 8 列の `key`・`guest`・`role`・`sha256`・`size`・`hook`・`args`・`omit`、区切りは TAB)。行は key `ildsp_x64.dll`・guest `Shared\ildsp_x64.dll`・role `補助`・sha256 上の値・size `4197376`・hook `none`・args `--pdata --reloc-seeds`・omit `-`。注釈は `lift/plugins/reverb/plugin.tsv` に倣う。
2. 二重の抽出を避けるため、`ci/extract-dlls.tsv` の ildsp の行を外す(`lift/plugins/reverb` と同じ形: plugin.tsv が抽出の行を作る)。行を外すと、`spec/workflow.test.sh` の r10(ci の行の検査)が落ちる。その検査を、plugin.tsv の行(key・guest・sha256・size)の検査に置き換える。検査の数は減らさない。
3. 取り消しで外した ildsp の項を戻す: `inputs/plugin-image-sizes.tsv` の ildsp の行、`lift/crates/fe-build/src/plugins.rs` の試験の件数(165 → 166)、SizeOfImage の合計の ildsp の項(`+ 4_521_984`)、`first_wave_folder` の一覧の ildsp(8 本)。`git show 7c9790a1` の差分の逆が、この値。
4. 止まりの番地を全部解く。1 回の lift・build で、見つけた番地を複数まとめて解く:
   a. 停止の記録に、間接呼び出しの読み元を出す診断を入れる(commit しない): 呼び先の値を作った `call [reg+off]` の `reg+off` の値(表の番地)と、その番地が属する像の場所(RVA)を、`fle_dispatch_stop` の記録に出す。
   b. 表の番地の周り(64 バイト)と、表の項目の並びを、ildsp の像のバイト列から読む。項目が 8 バイトの絶対番地か、4 バイトの RVA の配列かを決める。
   c. 表の全項目の RVA を、`lift/plugins/ildsp/seeds.tsv`(ヘッダは `key` と `rva` の 2 列、TAB 区切り。見本は `lift/plugins/reverb/seeds.tsv`)に足す。4 バイトの RVA の配列なら、`lift/plugins/ildsp/rva-tables.txt`(見本は lameenc の `rva-tables.txt`)を作り、args に `--rva-tables lift/plugins/ildsp/rva-tables.txt` を足す。
   d. GetProcAddress で得る入口は、そのエクスポートの RVA を seeds.tsv に足す(`fe-pe pe-exports` の一覧)。`--exports` の旗は無い。`fe-lift` の usage を見て確かめる。
   e. 診断の差分は、commit の前に外す。`indirect call … is not a known function entry` が、`file-open-demo` と Emphasis の plugin-run で出なくなるまで、番地を全部解く。
5. Frequency Shifter の folder は、この Issue では作らない(#1436 の束が作る)。

## 受け入れ

```accept
npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && node tools/place-plugin-dlls.mjs && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false&&r.elapsedMs<=20000?0:1)' "$RUNNER_TEMP/boot.json" && node tools/plugin-run/run.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" "Emphasis" --scene d > "$RUNNER_TEMP/plugin-run.jsonl" && node -e 'const L=require("fs").readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));process.exit(L.length===1&&L.every((r)=>r.instantiated===true&&r.error===null&&r.engineStop===null)?0:1)' "$RUNNER_TEMP/plugin-run.jsonl" && node tools/e2e/run.mjs --scenarios smoke --only file-open-demo --displays 4k-dpr1,4k-dpr2 > "$RUNNER_TEMP/e2e.jsonl" && node -e 'const L=require("fs").readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));process.exit(L.length===2&&L.every((r)=>r.error===null&&r.scenarios.length>0&&r.scenarios.every((s)=>s.ok===true&&s.engineStops===0))?0:1)' "$RUNNER_TEMP/e2e.jsonl" && node tools/smoke/welcome-flow.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/flow.jsonl" && node -e 'const fs=require("fs");const cap=Number(fs.readFileSync("pm-work/findings/wave2-round2.md","utf8").match(/^startup-cap-bytes: (\d+)$/m)[1]);const L=fs.readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const ok=(r)=>!!r&&r.error===null&&r.state==="engine-idle"&&r.standIn===false&&r.trapped===false&&r.hostErrors===0&&r.opened===true&&r.elapsedMs<=20000&&r.preBootBytes<=cap&&(!/^[BD]$/.test(r.flow)||r.welcomeClosed===true);process.exit(L.length===4&&["A","B","C","D"].every((f)=>ok(L.find((r)=>r.flow===f)))?0:1)' "$RUNNER_TEMP/flow.jsonl" && npm test --prefix app && cargo test --locked --manifest-path lift/Cargo.toml -p fe-build && bash guard/scan.sh
```

判定の基準:
- 起動確認(`boot.json`): engine-idle・standIn が false・error が null・elapsedMs が 20,000 以下。
- `plugin-run` の `Emphasis`(場面 d)の 1 行: instantiated が true(ildsp を読んだ)、error が null、engineStop が null。processed は、この Issue の判定に入れない(#1436 が確かめる)。
- `smoke/file-open-demo`(4k-dpr1・4k-dpr2)の 2 行: 各行の error が null、シナリオの ok が true、engineStops が 0。
- 流れ A〜D の 4 行: 条件が通り、preBootBytes が `pm-work/findings/wave2-round2.md` の `startup-cap-bytes` 以下。
- `npm test --prefix app` と `cargo test --locked --manifest-path lift/Cargo.toml -p fe-build` が通る。
- `bash guard/scan.sh` の指摘が 0 件。
- 上の受け入れは、直した後の木で通ること。診断の差分は、commit に入れない。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には「runner の値。この機械とは別枠」と書く。
- 報告に書く: (a) 停止の番地の表(呼び先・読み元の番地・表の場所(RVA)・項目の型・項目の数)、(b) 足した seeds.tsv の行数と rva-tables の有無、(c) ci の行を外したこと、r10 をどう置き換えたか、(d) 試験の件数の前後(165 → 166 と SizeOfImage の合計)、(e) plugin-run の Emphasis の 1 行、file-open-demo の 2 行、(f) 起動確認の elapsedMs、流れ A〜D の 4 行、preBootBytes、(g) `cargo test -p fe-build` の `test result` の行、(h) `bash guard/scan.sh` の結果。
- 原因が確定できず、番地を解けなかった時は、(a) の表と、次に確かめることを、報告の先頭に書いて終える。
- 「達成」「直った」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。出力の行を付ける。
- 報告の末尾に、判定が通って push されると `deploy.yml` が動く(`lift/` と `ci/` に触れるため)と書く。

## 制約

- 製品に、利用者向けの新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足さない。
- Python・OCR を使わない。
- sleep・timeout のコマンド・nohup・末尾の `&` を使わない。一時ファイルは `.tmp-*` に置き、終わったら消す。
- `git add` は、変えたファイルを 1 つずつ指定する(`git add -A`・`git add .` は使わない)。
- commit の名義は、環境の git の設定のまま。commit・タグ・PR に、AI の署名の行を付けない。必要の無いブランチを作らない。
- この機械で製品のビルドや測定をしない。製品の作業は Actions の作業ジョブで行う。

## 作業の進め方

- 作業の最初に `date -u` を記録する。`.prior-work.md` が在れば、最初に読む。
- ビルド(`fe-build engine`)は、1 回に数十分(観測: 17 分から 38 分)。ビルドは 1 回にし、その 1 回で、停止の番地を全て集める。長いコマンドはフォアグラウンドで直接、シェルツールの `timeout` 引数に 5400000 を渡す。背景で起動して待たない。
- 作業を始めてから 3 時間 15 分を過ぎたら、受け入れが通らなくても、診断の差分を外し、作業ツリーを commit し、報告を書いて終える(判定に約 2 時間 30 分掛かる)。
- 受け入れが通らなくても、作業の最後に作業ツリーを commit する。未 commit の変更が残ると、判定は失敗する。

## 他のタスクとの関係

- blocked_by: なし。
- #1436(第 2 波の束。Emphasizer と Frequency Shifter)は、この Issue を blocked_by に持つ(起票の後に設定する)。Frequency Shifter の folder は、この Issue の後に、#1436 が入れる。
- #1546(像の中の番地の型。`0x13447d38` は ildsp の番地)と、ildsp の番地の型が同じ。この Issue が ildsp の分を受け持つ。#1546 の受け入れの 5 の項は、この Issue が閉じた後に確かめる。
- `lift/crates/fe-build/src/plugins.rs` と `inputs/plugin-image-sizes.tsv` は、他の Issue も触るかもしれない。commit の前に `git pull --no-rebase --no-edit` で main を取り込み、衝突を直す。
