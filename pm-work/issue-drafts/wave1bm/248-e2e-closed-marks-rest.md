---
draft: "248"
title: "E2E の印: 閉じた Issue を指す残りの印(#1499・#1419・#1429・基準の一覧の閉じた番号)を、開いている Issue か new に直し、閉じた番号の印を検査する試験を足す"
type: Task
milestone: "M2 HiDPI と P2 の測定(P2)"
parent: "133"
blocked_by: ["222", "241", "244"]
labels: ["verify"]
size: S
priority: Medium
project: signature-studio/3
---

## 目的

E2E の印(`expectFail`・`expectStop`・`OPEN_ISSUES`・`ALLOWED_MARKS`)に、閉じた Issue の番号を指す物が残っている。閉じた番号の印は、その本が今どうなっているかを表さない。この Issue は、(1) 閉じた番号の印を、その本の今の結果に合わせて直す(ok なら印を外す。まだ失敗するなら `new`。開いている Issue の症状と同じと確かめられた時だけ、その番号)、(2) 閉じた番号の印が入らないように、試験を足す。製品(`app/`・`lift/`)は変えない。

この Issue の対象は、印の残りだけ。plugins・menus・file の閉じた印は、#1508・#1527・#1530 が直す(それぞれの本文に書いてある)。この Issue は、その 3 件が閉じてから始まる(blocked_by)。

## 背景

- 閉じた番号の印の検査(2026 年 11 月 28 日の時点。`tools/e2e/scenarios/` と `tools/e2e/lib/` の中。コメントの中も含む。閉じた番号は `gh issue view` で確かめた):
  - この Issue が直す物:
    - `scenarios/plugins/plugin-manager-opens.mjs`(印 `#1499`)・`scenarios/smoke/plugin-manager-opens.mjs`(印 `#1499`)・`scenarios/smoke/index.mjs` の `ALLOWED_MARKS`(`['#1499']`)。
    - `scenarios/regression/r11-language-switch-all-7.mjs`(印 `['#1395', '#1419']`。#1419 は閉じている)。
    - `scenarios/regression/r15-riff-machine-open.mjs`(印 `#1429`。閉じている)。
    - `scenarios/regression/common.mjs` の `OPEN_ISSUES`(`['#1386', '#1395', '#1419', '#1429', '#1502']`。#1419・#1429・#1502 は閉じている)。
  - #1508 が直す: `scenarios/plugins/` の `#1315`・`#1336`・`#1337`・`#1338`・`#1400`・`#1484` の印(生成された本と、手で書いた本)。
  - #1527 が直す: `scenarios/menus/` の `#1411`・`#1415`・`#1416`(`menus/_generate.mjs` の `STOP` の表を含む)。
  - #1530 が直す: `scenarios/file/z-persist-recent-list.mjs` の `#1409`。
- e2e-full の run の 22 件の閉じた印のうち、この Issue の対象は `plugins/plugin-manager-opens` の 1 件(実測)。他の 21 件は、上の 3 つの Issue が受け持つ。
- regression の本は、`e2e-full` の run に入らない(`--scenarios regression` で別に走る)。そのため、regression の印は e2e-full の結果には出ない。
- 閉じた番号の一覧(この試験に書く。2026 年 11 月 28 日に閉じていると確かめた番号): `1315 1319 1336 1337 1338 1382 1400 1406 1409 1411 1415 1416 1419 1421 1429 1484 1499 1502 1503 1504 1505`。
- 作業者の段には GitHub の token が無い。Issue の状態は、job の中では確かめられない。そのため、試験は一覧の番号だけを見る。新しく閉じた番号は、一覧に足すまで検出されない。

## 読む物

- `tools/e2e/scenarios/smoke/index.mjs`・`smoke/plugin-manager-opens.mjs`・`plugins/plugin-manager-opens.mjs`
- `tools/e2e/scenarios/regression/r11-language-switch-all-7.mjs`・`r15-riff-machine-open.mjs`・`common.mjs`(`OPEN_ISSUES`)
- `tools/e2e/lib/scenario.mjs`(`checkMarks`・`marksOf`・`stopMark`)、`tools/e2e/run.mjs`(印の扱い。`expectFail scenario passed` の警告)
- `tools/e2e/test/scenario.test.mjs`・`stop-rule.test.mjs`(印の既存の試験の形)

## やること

0. 作業の最初に `date -u` を記録する。`.prior-work.md` が在れば、最初に読む。
1. 試験を先に書く: `tools/e2e/test/closed-marks.test.mjs`。`tools/e2e/scenarios/` の `.mjs` を全部読み、引用符の中の `'#<番号>'` を集め、上の一覧の番号が 1 つでもあれば失敗させる。直す前に、この試験が、この Issue の 4 か所(`plugin-manager-opens` 2 本・`r11`・`r15`)と `OPEN_ISSUES`・`ALLOWED_MARKS` で落ちることを確かめる(報告に、落ちた行を書く)。
2. 4 件の本を走らせ、結果ごとに直す(`--scenarios smoke --only plugin-manager-opens`・`--area plugins --only plugins/plugin-manager-opens`・`--scenarios regression --only riff-machine-open,language-switch-all-7`):
   - ok なら、印を外す(`expectFail: null`)。`run.mjs` の `warning: expectFail scenario passed` も、同じ判断の根拠になる。
   - 失敗するなら、印を `'new'` にする。停止があるなら、`expectStop: 'new'` を付ける(印の規則は `lib/scenario.mjs`)。開いている Issue の症状と同じと確かめられた時だけ、その番号にする。
3. `smoke/index.mjs` の `ALLOWED_MARKS`・`regression/common.mjs` の `OPEN_ISSUES` を、残る番号だけにする(`checkMarks` は、一覧に無い番号の印を拒否する)。
4. 試験と受け入れを通す。通らない時は、落ちた ID・表示・出力の 1 行を報告に書く。

## 受け入れ

```accept
npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false&&r.elapsedMs!=null&&r.elapsedMs<=20000?0:1)' "$RUNNER_TEMP/boot.json" && (cd tools/e2e && npm ci && npx playwright install chromium) && node --test tools/e2e/test/*.test.mjs && ! grep -rqE "'#(1315|1319|1336|1337|1338|1382|1400|1406|1409|1411|1415|1416|1419|1421|1429|1484|1499|1502|1503|1504|1505)'" tools/e2e/scenarios tools/e2e/lib && node tools/e2e/run.mjs --scenarios smoke --only plugin-manager-opens --displays 4k-dpr1 --budget 20m --report-only > "$RUNNER_TEMP/smoke.jsonl" && node tools/e2e/run.mjs --area plugins --only plugins/plugin-manager-opens --displays 4k-dpr1 --budget 20m --report-only > "$RUNNER_TEMP/pm.jsonl" && node tools/e2e/run.mjs --scenarios regression --only riff-machine-open,language-switch-all-7 --displays 4k-dpr1 --budget 30m --report-only > "$RUNNER_TEMP/reg.jsonl" && node -e 'const fs=require("fs");let rows=0,bad=0,sc=0;for(const p of process.argv.slice(1)){for(const r of fs.readFileSync(p,"utf8").trim().split("\n").map((l)=>JSON.parse(l))){rows++;if(r.error!==null)bad++;sc+=r.scenarios.length}}process.exit(bad===0&&rows>0&&sc>=3?0:1)' "$RUNNER_TEMP/smoke.jsonl" "$RUNNER_TEMP/pm.jsonl" "$RUNNER_TEMP/reg.jsonl"
```

判定の基準:
- 受け入れの式が、終了コード 0 で終わる。
- `tools/e2e/test/closed-marks.test.mjs` が在り、直す前に落ちることを確かめてから直した(報告に、直す前の失敗の行を書く)。
- 上の一覧の閉じた番号が、`tools/e2e/scenarios` と `tools/e2e/lib` に 0 件(コメントの中も含む)。
- 3 本の run で、row に error が無い。
- 報告に: (a) 4 件の印の付け方の結果(外した・`new`・番号の別。根拠の出力の行つき)、(b) `ALLOWED_MARKS`・`OPEN_ISSUES` の変更後の中身、(c) 試験の直す前の失敗の行、(d) 試験の限界(一覧の番号しか見ないこと。新しく閉じた番号は、一覧に足すまで検出されない)、(e) 3 本の run の出力の行(ID・ok・notes)。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には、runner の値であることと、nproc と loadavg を添える。
- 「達成」「直った」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。出力の行を付ける。
- 確認できなかった物は「確認できない」と書く。
- 段ごとに、やること 0〜4 のどこまで進んだか(完了・途中・未着手)を書く。

## 制約

- 製品(`app/`・`lift/`)は変えない(読むだけ)。`.github/`・`spec/` は変えない。`tools/e2e/` を直す。
- 閉じた番号の印を、番号を書き換えるだけで残さない。結果に合わせて、外すか `new` にするか、開いている Issue の症状と同じと確かめた番号にする。
- 本の中身(判定の式・期待する結果)を、印を通すために変えない。
- 製品に、利用者向けの新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足さない。オプトインを作らない。テストのための分岐を製品に入れない。
- Python を使わない(調査と検証にだけ使う)。OCR を使わない。文字は、ゲストのメモリの文字列・窓の表・画素のハッシュで読む。
- 禁止されているのは、AI のベンダー名と Cloudflare の識別子だけ。FL の製品名・インストール先の名前・変数名は、書いてよい。
- sleep・nohup・末尾の `&`・先頭の `timeout` は使わない。待つ時は条件で待つ。
- 一時ファイルは `.tmp-*` に置き、終わったら消す。
- commit・PR に、AI が書いたことを示す行を付けない。commit の名義は、環境の設定のまま。
- GitHub に、Issue・ラベル・コメントを書かない。

## 作業の進め方

- 作業の最初に `date -u` を記録する。4 時間 30 分を過ぎたら、受け入れが通らなくても、作業ツリーを commit し、報告を書いて終える(ジョブの上限は 5 時間 50 分)。
- この作業の job(GitHub Actions の runner)で、受け入れの式の全てを実行する。ビルドと E2E は、この job の中で行う。
- ビルドや E2E の長いコマンドは、フォアグラウンドで直接実行し、シェルツールの `timeout` 引数に 5400000 を渡す。背景で起動して待たない。
- ビルドは 1 回にする。tools だけを直した後は、同じ bundle(`lift/out-app/bundle/raw/fl-Oz`)で E2E を走らせて直す。
- 受け入れが通らなくても、作業の最後に作業ツリーを commit する(未 commit の変更が残ると、判定は失敗する)。`git add` は 1 つずつ、自分が変えたパスだけを指定する。

## 他のタスクとの関係

- blocked_by: #1508・#1527・#1530。この Issue は、その 3 件が閉じてから始まる。その 3 件の印の直しが済んでいないと、受け入れの grep が落ちる。
- #1509(topbar・page・resilience・perf)の本文の「開いている Issue」の一覧には、閉じた番号が 6 件ある(#1429・#1499・#1502・#1503・#1504・#1505)。#1509 の作業で、閉じた番号へ印を付け替えた時は、この試験が落ちる。その時は、この Issue の作業の中で、その印を直す。
