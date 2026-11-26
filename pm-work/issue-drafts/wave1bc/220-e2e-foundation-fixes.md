---
draft: "220"
title: "E2E の基盤を直す: `--displays` の宣言・印つきの失敗の停止の判定・予算の分け方・所要時間と失敗の記録"
type: Task
milestone: "M2 HiDPI と P2 の測定(P2)"
parent: "133"
blocked_by: []
labels: ["verify", "render"]
size: L
priority: High
project: signature-studio/3
---

## 目的

E2E の基盤(`tools/e2e/`)の、走らせ方と判定の欠陥を直す。全シナリオ(985 本)を回した初回の結果で分かった 6 つの欠陥(下の「背景」)を直し、後の領域の Issue(#{{221}}・#{{222}}・#{{223}})が、表示の宣言と予算を正しく使える状態にする。製品(`app/`・`lift/`)は変えない。

この Issue の目標は次の 2 つ。
1. 6 つの欠陥を直し、試験(`tools/e2e/test/`)と、短い実行の出力で、直したことを示すこと。
2. 直した後の挙動(表示の宣言を守る・印つきの停止を見る・予算を行ごとに分ける・結果に宣言と所要時間とログを出す)を、報告に書くこと。

## 背景

- 全シナリオを GitHub Actions で回した初回の run(番号 45854456293。commit 8b3bdeb9)。1 シナリオは、表示 2 本(4k-dpr1 と 4k-dpr2)で走らせた。結果は 1,970 件。内訳は、未印の合格 976、印つきの合格 13、印つきの失敗 651、未印の失敗 261(うち error 25)、skip 69。両方の表示で未印の合格だったシナリオは 361 本(36.6 %)。
- この run の出力は、作業者は持たない。必要な数字と場所は、この本文に書いた。
- 欠陥 1(表示の宣言): `tools/e2e/run.mjs` の `rowsOf` は、`--displays` が付くと、シナリオの宣言(`displays`。既定は `4k-dpr1`・`4k-random`)を見ず、選んだ全てのシナリオを、その表示で走らせる。宣言に `4k-dpr2` を含むシナリオは、旧い形の集合(smoke・browser-face)を除くと 0 本なのに、run では多くが dpr2 で走った。
- 欠陥 2(印つきの停止): `tools/e2e/lib/plan.mjs` の `rowPasses` は、印つき(`expectFail`)のシナリオで、ページの例外・console のエラー・失敗した要求だけを見て、エンジンの停止(`engineStops`・`engineStuckRunning`)を見ない。`settings/language-*`(7 本。dpr1)は、言語の切替でエンジンが止まった(exit 134)が、印 `new` のため合格になった。
- 欠陥 3(宣言と違う DPR): `tools/e2e/scenarios/page/crisp-*.mjs`・`zoom-*.mjs` は、宣言の DPR と違う DPR で走っても失敗しない。`crisp-dpr-3`(宣言 `dpr:3`)は DPR 1 と 2 でも合格し、網羅が過大になった。`crisp-screen-pixels`(宣言 `dpr:1`)は、DPR 2 で 7,954 / 8,285 の画素が違った。
- 欠陥 4(予算): `--budget` の時計が、表示の行をまたいで 1 つだけ(`main` で作り、全ての行で共有する)。dpr1 の行が予算を使い切ると、dpr2 の行の `resilience` の 42 本が全て skip された。
- 欠陥 5(所要時間): 結果の JSON に、シナリオごとの所要時間が無い。予算の見積もりができない。
- 欠陥 6(失敗の記録): 失敗したシナリオの、エンジンのログが JSON に無い(`--engine-logs` は任意の出力で、run では使っていない)。

## 読む物

- `tools/e2e/run.mjs`(`rowsOf`・`runRow`・`main`・`newResult`・`runShared`・`runLegacy`)
- `tools/e2e/lib/plan.mjs`(`displayRows`・`rowPasses`・`runPasses`・`planScenarios`・`budgetClock`)
- `tools/e2e/lib/scenario.mjs`(`defineScenario`・`DEFAULT_DISPLAYS`・`checkMark`・`legacyScenario`)
- `tools/e2e/lib/stop.mjs`(エンジンの停止の報告)
- `tools/e2e/scenarios/page/crisp-*.mjs`・`zoom-*.mjs`・`tools/e2e/scenarios/settings/language-*.mjs`
- `tools/e2e/test/*.test.mjs`(既存の試験の書き方。`node:test`。Playwright は使わない)
- `tools/e2e/README.md`(引数の表と、判定の基準)

## やること

0. 作業の最初に `date -u` を記録する。`.prior-work.md` が在れば、最初に読む。
1. **試験を先に書く**: `tools/e2e/test/` に、欠陥 1〜5 の純粋な部分の試験を書く。ブラウザを使わず、`lib/plan.mjs` などの関数を、ページ無しで試せる形にする(必要なら、純粋な関数を `lib/plan.mjs` に分けて出す)。直す前に、試験が落ちることを確かめる。
2. **表示の宣言を守る**(欠陥 1): `rowsOf` を直す。`--displays` は、行(表示)の選択だけにする。シナリオは、宣言の `displays` にその表示が在る物だけ動かす。`--displays` を付けない時の動きは変えない。`--scenarios smoke`・`browser-face` は宣言が 3 表示なので、結果が変わらないことを確かめる。
3. **印つきの停止を見る**(欠陥 2): `rowPasses` を直す。印つきのシナリオも、エンジンが止まった(`engineStops` が 0 より大きい、または `engineStuckRunning` が真)なら失敗とする。ただし、既知の失敗として停止を宣言しているシナリオだけは、停止を許す(宣言の書式は作業者が決め、理由を報告に書く)。この変更で落ちる今の印つきシナリオは、一覧(ID・表示・停止の理由)を報告に書く。`--report-only` の行の判定には影響させない。
4. **DPR の宣言を守る**(欠陥 3): `page/crisp-*.mjs`・`zoom-*.mjs` は、ページの DPR が宣言と違えば失敗にする。(2) の後は宣言外の表示では走らないが、この検査も残す。
5. **予算を行ごとに分ける**(欠陥 4): `--budget` の時計を、表示の行ごとに作る。シナリオには、その行の残りの予算を渡す(`resilience/wait-idle-30min` が使う。残りが 30 分より少なければ始めない)。
6. **所要時間と失敗の記録**(欠陥 5・6): 各シナリオの結果に `ms`(開始から終了までのミリ秒)を足す。未印で ok が偽のシナリオと、印つきで停止したシナリオには、エンジンのログの、そのシナリオの開始から後の末尾 200 行を `failLog` として足す。画像は足さない。
7. **宣言を JSON に出す**: 各シナリオの結果に、宣言の `displays` を足す(受け入れの式が見る)。
8. `tools/e2e/README.md` の `--displays` の説明(「`--displays` を付けると、選んだシナリオは全てその表示で動く」)を、新しい動きに直す。予算の説明も、行ごとに直す。
9. 受け入れを通す。通らなければ、作業ツリーを commit して、報告を書いて終える。

## 受け入れ

```accept
npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false?0:1)' "$RUNNER_TEMP/boot.json" && node tools/smoke/welcome-flow.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/flow.jsonl" && node -e 'const fs=require("fs");const L=fs.readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const F=(f)=>L.find((r)=>r.flow===f);const ok=(r)=>!!r&&r.error===null&&r.state==="engine-idle"&&r.standIn===false&&r.trapped===false&&r.hostErrors===0&&r.opened===true;process.exit(L.length===4&&["A","B","C","D"].every((f)=>ok(F(f)))&&F("B").welcomeClosed===true&&F("D").welcomeClosed===true?0:1)' "$RUNNER_TEMP/flow.jsonl" && npm test --prefix app && (cd tools/e2e && npm ci && npx playwright install chromium) && node --test tools/e2e/test/*.test.mjs && node tools/e2e/run.mjs --list > "$RUNNER_TEMP/list.jsonl" && node -e 'const L=require("fs").readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const B={page:84,input:3,file:3,menus:3,windows:251,audio:3,visual:3,settings:150,plugins:302,topbar:63,resilience:42,perf:31,delivery:47};process.exit(L.length===13&&L.every((r)=>r.scenarios>=B[r.area])?0:1)' "$RUNNER_TEMP/list.jsonl" && node tools/e2e/run.mjs --area page --shard 1/3 --displays 4k-dpr1 --budget 40m --report-only > "$RUNNER_TEMP/page13.jsonl" && node -e 'const L=require("fs").readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const S=L.flatMap((r)=>r.scenarios);process.exit(L.length===1&&S.length>0&&L.every((r)=>r.error===null&&r.servedHeadersMatch===true)&&S.every((s)=>s.skipped===true||(Array.isArray(s.displays)&&s.displays.includes("4k-dpr1")&&typeof s.ms==="number"&&(s.ok===true||s.expectFail!==null||Array.isArray(s.failLog))))?0:1)' "$RUNNER_TEMP/page13.jsonl"
```

判定の基準:
- 受け入れの式が、終了コード 0 で終わる(流れ A〜D・`npm test --prefix app`・`node --test`・`--list`・`--area page --shard 1/3 --displays 4k-dpr1 --report-only` の行の構造)。
- `--list` の各領域の件数が、下の基準以上。基準(この Issue の作業の前の件数): page 84・input 3・file 3・menus 3・windows 251・audio 3・visual 3・settings 150・plugins 302・topbar 63・resilience 42・perf 31・delivery 47。他の領域の Issue が件数を増やすので、基準は下限(減らしてはいけない)。
- 試験(`node --test tools/e2e/test/*.test.mjs`)が全て通る。直す前に落ちていたことを、報告に書く。
- 報告に、(a) 手順 3 の規則(印つきの停止)で落ちる今の印つきシナリオの一覧(ID・表示・停止の理由)、(b) 停止を許す宣言の書式と理由、(c) 行ごとの予算の下での、全体の所要の見積もり(job の上限を決めるため)、(d) 受け入れの式の出力の行、を書く。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には、runner の値であることと、nproc と loadavg を添える。
- 「達成」「直った」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。出力の行を付ける。
- 確認できなかった物は「確認できない」と書く。
- 段ごとに、やること 0〜9 のどこまで進んだか(完了・途中・未着手)を書く。

## 制約

- 製品(`app/`・`lift/`)は変えない(読むだけ)。`.github/`・`spec/`・`ci/` も変えない。
- 製品に、利用者向けの新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足さない。オプトインを作らない。テストのための分岐を製品に入れない。
- Python を使わない(調査と検証にだけ使う)。OCR を使わない。
- 禁止されているのは、AI のベンダー名と Cloudflare の識別子だけ。FL の製品名・インストール先の名前・変数名は、書いてよい。
- sleep・nohup・末尾の `&`・先頭の `timeout` は使わない。待つ時は条件で待つ。
- 一時ファイルは `.tmp-*` に置き、終わったら消す。
- commit・PR に、AI が書いたことを示す行を付けない。commit の名義は、環境の設定のまま。
- GitHub に、Issue・ラベル・コメントを書かない。

## 作業の進め方

- 作業の最初に `date -u` を記録する。4 時間 30 分を過ぎたら、受け入れが通らなくても、作業ツリーを commit し、報告を書いて終える(ジョブの上限は 5 時間 50 分)。
- この作業の job(GitHub Actions の runner)で、受け入れの式の全てを実行する。ビルドと E2E は、この job の中で行う。
- 受け入れの式は、ジョブの判定が 1 本で実行する。作業者は、式の段ごとに、シェルの呼び出しを分けて確かめてよい。
- ビルドや E2E の長いコマンドは、フォアグラウンドで直接実行し、シェルツールの `timeout` 引数に 5400000 を渡す。背景で起動して待たない。
- ビルドは 1 回にする。tools だけを直した後は、同じ bundle(`lift/out-app/bundle/raw/fl-Oz`)で E2E を走らせて直す。
- 1 回で終わらない前提で、段ごとに commit する。再試行は前回の差分の上で続け、作業ツリーを消さない。
- 受け入れが通らなくても、作業の最後に作業ツリーを commit する(未 commit の変更が残ると、判定は失敗する)。

## 他のタスクとの関係

- blocked_by: なし。#{{221}}・#{{222}}・#{{223}} は、この Issue が閉じてから始まる(宣言・予算・JSON の変更に依る)。#{{224}} は独立。
- `.github/workflows/e2e-full.yml`(全シナリオの job)は、この Issue の範囲外。`--displays` の付け方と、予算の値は、その側で決める。
