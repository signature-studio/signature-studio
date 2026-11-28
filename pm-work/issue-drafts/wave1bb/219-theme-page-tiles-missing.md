---
draft: "219"
title: "Settings の Theme の頁に、テーマのタイルが出ず、13 件のテーマを切り替えられない: 頁の一覧が見つからない原因を特定して直す"
type: Task
milestone: "M4 FL に必要な物の配布"
parent: "012"
blocked_by: ["165"]
labels: ["resources", "verify"]
size: M
priority: High
project: signature-studio/3
---

## 目的

依頼者の報告(テーマの切り替え): 「テーマを切り替える」の操作で、全 13 件を切り替えられること。#1382 で、テーマの切り替えで engine が止まる件は直した(閉じている)。その後の E2E(regression の `theme-switch-all-13`。2 表示)で、Settings(F10)の Theme の頁に、テーマのタイルが 1 つも見つからず、1 件も切り替えられなかった。

この Issue の目標は次の 3 つ。
1. Settings の Theme の頁に、インストーラの全テーマ(13 件)の一覧が出る。
2. 13 件を、1 件ずつ選べる。選ぶたびに engine が `engine-idle` のまま、画面が変わる。
3. 原因が製品の欠陥なら製品を直す。E2E の探し方の誤りなら E2E(`tools/e2e/scenarios/regression/r10-theme-switch-all-13.mjs`)を直し、根拠を報告に書く。

## 背景

- regression の R10(`theme-switch-all-13`。#1408 の実測。2 表示。#1382 の後の main): Theme の頁の文字(窓の表の文字)は Presets・Adjustments・Options だけで、テーマの名前の文字を持つ窓が無い。R10 は、テーマの名前を窓の表の文字で探すので、タイルが 1 つも見つからず、1 件も切り替えなかった。
- #1382 の作業の記録(閉じた Issue の本文の「前回の試行で分かったこと」): タイルは描画された画素で、スクロールする一覧の中に在り、つまみのドラッグで押せた。窓の表には、タイルの文字の窓は無かった。
- つまり、タイルが画面に在るかどうか、押せるかどうかは、窓の表の文字では判定できない(推定。記録で確かめる)。製品の欠陥(一覧が描画されない・読めていない)か、E2E の探し方の誤り(文字で探している)かを、最初に決める。
- テーマの数: `inputs/fl-installer-files.tsv` の `Artwork\Themes\` の下の `.flstheme` は 13 件。直下の 8 件(Ash・Dark orange・Dark・Light cherry・Light pear・Light tangerine・Sytrus・Ultra green)と、フォルダの 5 件(`DEDZII\DEDZII Light`・`DEDZII\DEDZII`・`P.Priime\PPRIIME Dark`・`P.Priime\PPRIIME Light`・`Veela\Veela`)。`System\Themes\` の行は、インストーラの一覧に無い。
- 切り替えの経路: テーマは `Artwork\Themes\` の `.flstheme` を、FL が読み、選んだ物を `Settings\Themes\__current.flstheme` として書く(推定。#1382 の記録による)。

## 読む物

- `tools/e2e/scenarios/regression/r10-theme-switch-all-13.mjs`・`tools/e2e/scenarios/regression/common.mjs`(`settingsPage`・`under`・`table`・`clickIn`・`pictureHash`)。どちらも #1408 が main に入った後の版を読む。
- `#1382` の本文(閉じた Issue。「前回の試行で分かったこと」の節。タイルの位置と押し方の記録)
- `inputs/fl-installer-files.tsv`(`Artwork\Themes\` の行)・`tools/e2e/features/generated/resources.tsv`(テーマの一覧)
- `lift/harness/seeds/vfs-seed-idle1.tsv`(テーマの `resources/Artwork/Themes/...` の行)
- `lift/runtime/fle_win.c`(窓の表の作り方。子の窓と文字の取り方)・`lift/runtime/fle_gdi.c`(描画の経路)
- `pm-work/findings/missing-paths.md`(起動の間に要求されて無かった物の記録。テーマの `__current.flstheme`)

## やること

0. 作業の最初に `date -u` を記録する。`.prior-work.md` が在れば最初に読む。
1. **手で再現する**: F10 で Settings を開き、Theme の頁を選ぶ。その頁の一覧の画素を、窓の表の子の窓(矩形)と合わせて取る。タイルの位置を、スクロールの前後の画素の変化で求める。タイルが画面に在り、押せることを確かめる。
2. **原因を決める**: (a) 製品の欠陥: 一覧が描画されない、テーマの名前が読めていない(`.flstheme` の読み、VFS の seed の不足、窓の作り方)。(b) E2E の誤り: タイルを窓の表の文字で探している。どちらかを、記録(画素・窓の表・VFS の要求の記録)で決め、報告に書く。推定だけで決めない。
3. **直す**:
   - (a) なら、製品を直す(一覧が描画され、名前が読める形にする)。
   - (b) なら、E2E の探し方を直す。タイルの位置は画素で求め、押した後の状態(engine の状態・画素の変化)で確かめる。テーマの名前の一致は、窓の表の文字が無い時の目印にしない。
4. **13 件を順に切り替える**: 各回で、選んだ後 3 秒で、engine が `engine-idle`、画面の画素が変わる。JSON の `results.rows` に 13 件を、名前・見つかったか・状態・変わったかを書く。
5. **E2E**: `theme-switch-all-13` の印(`expectFail`)を外す。2 表示で ok。

## 受け入れ

```accept
npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false&&r.elapsedMs<=12500?0:1)' "$RUNNER_TEMP/boot.json" && node tools/smoke/welcome-flow.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/flow.jsonl" && node -e 'const fs=require("fs");const L=fs.readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const F=(f)=>L.find((r)=>r.flow===f);const ok=(r)=>!!r&&r.error===null&&r.state==="engine-idle"&&r.standIn===false&&r.trapped===false&&r.hostErrors===0&&r.opened===true;process.exit(L.length===4&&["A","B","C","D"].every((f)=>ok(F(f)))&&F("B").welcomeClosed===true&&F("D").welcomeClosed===true?0:1)' "$RUNNER_TEMP/flow.jsonl" && npm test --prefix app && (cd tools/e2e && npm ci && npx playwright install chromium) && (node tools/e2e/run.mjs --scenarios regression --only theme-switch-all-13 --displays 4k-dpr1,4k-dpr2 > "$RUNNER_TEMP/e2e.jsonl") && node -e 'const fs=require("fs");const L=fs.readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const good=(r)=>{const s=r.scenarios.find((x)=>x.name==="theme-switch-all-13");return r.servedHeadersMatch===true&&r.error===null&&!!s&&s.expectFail===null&&s.ok===true&&s.pageErrors===0&&s.consoleErrors===0&&s.failedRequests===0&&s.stuckToasts===0&&Array.isArray(s.results?.rows)&&s.results.rows.length===13&&s.results.rows.every((x)=>x.found===true&&x.state==="engine-idle"&&x.changed===true);};process.exit(L.length===2&&L.every(good)?0:1)' "$RUNNER_TEMP/e2e.jsonl" && bash guard/scan.sh
```

## 判定の基準

- 起動確認(`boot.json`)が engine-idle・standIn が false・elapsedMs が 12,500 以下。
- 流れ A〜D の 4 行が、`error` が null・engine-idle・`standIn` が false・`trapped` が false・`hostErrors` が 0・`opened` が true。B と D は `welcomeClosed` が true。
- `npm test --prefix app` が通る。
- `theme-switch-all-13` が、2 表示(4k-dpr1・4k-dpr2)の全ての行で、印なし・`ok` が真・`pageErrors`・`consoleErrors`・`failedRequests`・`stuckToasts` が 0。`results.rows` が 13 件で、全件が見つかり(`found`)、`engine-idle`、画面が変わる(`changed`)。
- `bash guard/scan.sh` が 0 件。
- 報告に、(a) 製品の欠陥か、(b) E2E の誤りかの根拠(画素・窓の表の記録)、直した箇所、13 件の切り替えの表(名前・状態・変わったか)を書く。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には「runner の値。この機械とは別枠」と書き、nproc と loadavg を添える。
- 実測と推定を分ける。「達成」「直った」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。出力の行を付ける。

## 制約

- 製品に、利用者向けの新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足さない。オプトインを作らない。テストのための分岐を製品に入れない。
- テーマの名前を、製品のコードやテストに表として書き並べない。インストーラの `Artwork\Themes\` の一覧から作る。
- 判定を緩めない。タイルを見つけられないのを、テーマの数を減らして隠さない。13 件の全てを切り替える。
- Python と OCR を使わない(テーマの名前は、窓の表と画素と、FL が読んだ物の記録で確かめる)。禁止されているのは、AI のベンダー名と Cloudflare の識別子だけ。FL の製品名・インストール先の名前・変数名は、書いてよい。
- sleep・timeout のコマンド・nohup・末尾の `&` は使わない。待つ時は条件で待つ。一時ファイルは `.tmp-*` に置き、終わったら消す。
- 製品の作業(ビルド・測定)は Actions で行う。この機械では行わない。

## 作業の進め方

- 作業の最初に `date -u` を記録する。4 時間 30 分を過ぎたら、受け入れが通らなくても、作業ツリーを commit し、報告を書いて終える(ジョブの上限は 5 時間 50 分)。
- ビルドは約 35 分。フォアグラウンドで直接、シェルツールの `timeout` 引数は最大 5400000(90 分)にする。背景で起動して待たない。
- 段 1(再現)と段 2(原因)の結果が出た時点で commit する。受け入れが通らなくても、作業の最後に作業ツリーを commit する。

## 他のタスクとの関係

- blocked_by: #1408(regression の `theme-switch-all-13` を作る。main に在る必要がある)。
- #1382(テーマの切り替えで止まる件。閉じている)の続き。止まる件は #1382 で直っている。この Issue は、タイルが見えない・選べない件。
