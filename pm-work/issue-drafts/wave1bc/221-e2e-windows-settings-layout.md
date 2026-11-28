---
draft: "221"
title: "E2E の windows と settings を、表示の倍率(DPR)に依らない位置で書く"
type: Task
milestone: "M2 HiDPI と P2 の測定(P2)"
parent: "133"
blocked_by: ["220"]
labels: ["verify", "render"]
size: XL
priority: High
project: signature-studio/3
---

## 目的

`windows`(251 本)と `settings`(150 本)の E2E のシナリオは、DPR 1 の guest 座標の固定値で、部品を押したり、読んだりしている。DPR 2 では FL の UI が約 2 倍に描かれるので、固定の座標は別の部品に当たる。この Issue は、部品の位置を窓の表の矩形から求める共通の関数を作り、2 領域のシナリオが使うようにする。DPR 1 と DPR 2 の両方で、同じ判定が通ることを目指す。製品(`app/`・`lift/`)は変えない。

## 背景

- 全シナリオを回した初回の run(番号 45854456293。commit 8b3bdeb9)の結果:
  - `settings`: 未印の失敗は 115 件で、全て DPR 2(`no panel "…" under …`・`no Presets grid`)。原因は、`tools/e2e/lib/settings.mjs` の `TAB_X`(DPR 1 で測った guest px の定数)で、タブを押すこと。DPR 2 では別のタブに当たる。
  - `windows`: 未印の失敗は 122 件。うち DPR 2 の固定座標が 98 件(`tools/e2e/lib/windows.mjs` の `RACK_LAYOUT`(Channel rack)と、`mx.x + 127 + 44k`(Mixer)が DPR 1 の値)。残りの 24 件は、`windows/plugin-f8` の後の連鎖(Channel rack が `vis false` のまま。両方の DPR で 12 件ずつ)。連鎖の後始末は、この Issue で足す。
  - 画の大きさ(3840 x 2160)は DPR によらず同じ。一方、UI の要素は DPR 2 で約 2 倍になる(guest px。DPR 1 → DPR 2 で、メニュー欄 208x298 → 402x604、メニューバー 293x33 → 586x66、picker 176x113 → 325x227)。`windows.mjs` の冒頭の「画の大きさは DPR によらない」という前提は、画の大きさについてだけ正しく、座標の定数には当てはまらない。
- 黒い区画: `page/frame-no-black-in-windows` と `page/reload-picture-after` の計数は、窓の中の区画の数を、画面の全区画(`lib/page.mjs` の `blackInWindows` の `total`)で割っている。そのため、DPR 2 で 15.1 %、DPR 1 で 7.4 %になる(run の値)。分母を、窓の中の区画にする。
- 印 `new` の settings の 8 本: `settings/language-*`(7 本。言語の切替で、エンジンが止まる。停止は、#{{220}} の規則で落ちる)と `settings/choice-pick-colorful-waveforms`。
- 宣言の DPR: 今は、`windows` の 251 本は既定(`4k-dpr1`・`4k-random`)、`settings` の 111 本は既定、39 本は `4k-dpr1` だけを宣言している。`4k-dpr2` を宣言するシナリオは、旧い形の集合を除くと 0 本。dpr2 で走らせる(#{{220}} の後)ためには、この 2 領域の宣言に `4k-dpr2` を足す必要がある。
- 位置の方式(この Issue で決める): 窓の表(`tools/e2e/lib/common.mjs` の `windowTable`・`tools/e2e/lib/dump.mjs` の `absoluteRect`)の、窓の左上・幅・高さから、部品の位置を、窓の幅と高さに対する比で求める。DPR 1 で測った比を、DPR 2 で当てはまるかを確かめる。

## 読む物

- `tools/e2e/lib/windows.mjs`(`RACK_LAYOUT`・`rackPoint`・`rowStepsRect`・`mx.x + 127 + 44k` を使う関数の全て)
- `tools/e2e/lib/settings.mjs`(`TAB_X`・`goTab`・`languageRun`・`withSettings`・`panelOnPage`)
- `tools/e2e/lib/page.mjs`(`blackInWindows`)・`tools/e2e/lib/picture.mjs`(`blackTiles`)・`tools/e2e/lib/common.mjs`(`noBlackRegions`・`windowTable`)
- `tools/e2e/lib/dump.mjs`(`absoluteRect`・`childrenOfClass`)・`tools/e2e/lib/guest.mjs`(操作)
- `tools/e2e/scenarios/windows/*.mjs`・`tools/e2e/scenarios/settings/*.mjs`(宣言と、固定座標の使い方)
- `tools/e2e/scenarios/windows/plugin-f8.mjs`・`zz-f12-plugin.mjs`

## やること

0. 作業の最初に `date -u` を記録する。`.prior-work.md` が在れば、最初に読む。
1. **試験を先に書く**: `tools/e2e/test/` に、比から部品の位置を求める関数(次の (2))の試験を書く。直す前に、試験が落ちることを確かめる。
2. **位置の共通の関数**: `tools/e2e/lib/layout.mjs` を新しく作る。窓の表の矩形(窓の左上・幅・高さ)と、部品の比(窓の左上からの距離を、窓の幅と高さで割った値)から、部品の guest px の座標を求める。DPR の値を決め打たない(実行時の窓の大きさから求める)。
3. **windows を直す**: `lib/windows.mjs` の `RACK_LAYOUT` と `mx.x + 127 + 44k` を、(2) の関数に置き換える。`rackPoint`・`rowStepsRect` など、同じ定数を使う関数も直す。
4. **settings を直す**: `lib/settings.mjs` の `TAB_X` を、(2) の関数に置き換える(選択の欄の矩形の幅に対する比)。`goTab` と、頁の中の部品の位置も、同じ方式にする。
5. **宣言に 4k-dpr2 を足す**: `windows` の全シナリオ(251 本)と、`settings` の全シナリオ(150 本)の宣言の `displays` に、`'4k-dpr2'` を足す。宣言が `['4k-dpr1']` だけの 39 本は、dpr2 でも判定が成り立つ物にだけ足し、足さない物は理由を報告に書く。
6. **連鎖の後始末**: `windows/plugin-f8.mjs`・`zz-f12-plugin.mjs` に後始末を足す(Plugin list を閉じ、Channel rack を出す)。後始末の後、Channel rack の可視が真であることを、窓の表で確かめる。(判定の原因は #{{224}} が確かめる。この Issue は後始末だけを足す。)
7. **黒い区画の分母**: `lib/page.mjs` の `blackInWindows` の分母を、窓の中の区画の数にする(区画の中心が、窓の表の窓の中に在る物)。`lib/picture.mjs` の `blackTiles`(`lib/common.mjs` の `noBlackRegions` が使う)も、窓の中に限るかを決め、理由を報告に書く。
8. **language の判定**: `lib/settings.mjs` の `languageRun`: エンジンが止まった時は、表示の判定を飛ばし、停止として結果に書く(`results` に停止の有無)。停止の無い時だけ、表示の変化を判定する。`settings/choice-pick-colorful-waveforms` の印 `new` は、(a) この Issue の修正で ok になるなら印を外す、(b) ならないなら残す、のどちらかを報告に書く。
9. 受け入れを通す。通らない shard は、落ちた ID・表示・出力の 1 行を報告に書く。

## 受け入れ

```accept
npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false?0:1)' "$RUNNER_TEMP/boot.json" && node tools/smoke/welcome-flow.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/flow.jsonl" && node -e 'const fs=require("fs");const L=fs.readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const F=(f)=>L.find((r)=>r.flow===f);const ok=(r)=>!!r&&r.error===null&&r.state==="engine-idle"&&r.standIn===false&&r.trapped===false&&r.hostErrors===0&&r.opened===true;process.exit(L.length===4&&["A","B","C","D"].every((f)=>ok(F(f)))&&F("B").welcomeClosed===true&&F("D").welcomeClosed===true?0:1)' "$RUNNER_TEMP/flow.jsonl" && npm test --prefix app && (cd tools/e2e && npm ci && npx playwright install chromium) && node --test tools/e2e/test/*.test.mjs && node tools/e2e/run.mjs --area windows --shard 1/9 --displays 4k-dpr2 --budget 40m --report-only > "$RUNNER_TEMP/w19d2.jsonl" && node -e 'const L=require("fs").readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const S=L.flatMap((r)=>r.scenarios);const u=S.filter((s)=>s.expectFail===null&&s.ok!==true&&s.skipped!==true).length;process.exit(L.length>=1&&S.length>0&&L.every((r)=>r.error===null&&r.servedHeadersMatch===true)&&u<=3?0:1)' "$RUNNER_TEMP/w19d2.jsonl" && node tools/e2e/run.mjs --area windows --shard 5/9 --displays 4k-dpr2 --budget 40m --report-only > "$RUNNER_TEMP/w59d2.jsonl" && node -e 'const L=require("fs").readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const S=L.flatMap((r)=>r.scenarios);const u=S.filter((s)=>s.expectFail===null&&s.ok!==true&&s.skipped!==true).length;process.exit(L.length>=1&&S.length>0&&L.every((r)=>r.error===null&&r.servedHeadersMatch===true)&&u<=3?0:1)' "$RUNNER_TEMP/w59d2.jsonl" && node tools/e2e/run.mjs --area settings --shard 1/5 --displays 4k-dpr2 --budget 40m --report-only > "$RUNNER_TEMP/s15d2.jsonl" && node -e 'const L=require("fs").readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const S=L.flatMap((r)=>r.scenarios);const u=S.filter((s)=>s.expectFail===null&&s.ok!==true&&s.skipped!==true).length;process.exit(L.length>=1&&S.length>0&&L.every((r)=>r.error===null&&r.servedHeadersMatch===true)&&u<=3?0:1)' "$RUNNER_TEMP/s15d2.jsonl" && node tools/e2e/run.mjs --area settings --shard 3/5 --displays 4k-dpr2 --budget 40m --report-only > "$RUNNER_TEMP/s35d2.jsonl" && node -e 'const L=require("fs").readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const S=L.flatMap((r)=>r.scenarios);const u=S.filter((s)=>s.expectFail===null&&s.ok!==true&&s.skipped!==true).length;process.exit(L.length>=1&&S.length>0&&L.every((r)=>r.error===null&&r.servedHeadersMatch===true)&&u<=3?0:1)' "$RUNNER_TEMP/s35d2.jsonl" && node tools/e2e/run.mjs --area windows --shard 1/9 --displays 4k-dpr1 --budget 40m --report-only > "$RUNNER_TEMP/w19d1.jsonl" && node -e 'const L=require("fs").readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const S=L.flatMap((r)=>r.scenarios);const u=S.filter((s)=>s.expectFail===null&&s.ok!==true&&s.skipped!==true).length;process.exit(L.length>=1&&S.length>0&&L.every((r)=>r.error===null&&r.servedHeadersMatch===true)&&u<=3?0:1)' "$RUNNER_TEMP/w19d1.jsonl"
```

判定の基準:
- 受け入れの式が、終了コード 0 で終わる。5 本の run(windows の 1/9 と 5/9、settings の 1/5 と 3/5 を 4k-dpr2、windows の 1/9 を 4k-dpr1)のそれぞれで、未印で ok が偽(skipped を除く)のシナリオが 3 本以下。
- `windows` と `settings` の宣言に `4k-dpr2` が在る。宣言が欠けていると、dpr2 の run は 0 本になり、受け入れの式が失敗する。
- 試験(`node --test tools/e2e/test/*.test.mjs`)が全て通る。
- 報告に: (a) 変えた関数の一覧と、比の値(DPR 1 で測った値と、DPR 2 で当てはまる確認の出力の行)、(b) 黒い区画の分母を変えた後の値(DPR 1・DPR 2 の数)、(c) 5 本の run の出力の行(各 run の未印の失敗の ID)、(d) 印 `new` の settings の 8 本の扱い、(e) 宣言に `4k-dpr2` を足さなかった物と理由。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には、runner の値であることと、nproc と loadavg を添える。
- 「達成」「直った」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。出力の行を付ける。
- 確認できなかった物は「確認できない」と書く。
- 段ごとに、やること 0〜9 のどこまで進んだか(完了・途中・未着手)を書く。

## 制約

- 製品(`app/`・`lift/`)は変えない(読むだけ)。`.github/`・`spec/`・`ci/` も変えない。
- 製品に、利用者向けの新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足さない。オプトインを作らない。テストのための分岐を製品に入れない。
- Python を使わない(調査と検証にだけ使う)。OCR を使わない。文字は、ゲストのメモリの文字列・窓の表・画素のハッシュで読む。
- 禁止されているのは、AI のベンダー名と Cloudflare の識別子だけ。FL の製品名・インストール先の名前・変数名は、書いてよい。
- 期待値に、実行時の値(DPR・DPI・窓の位置)を定数で書かない。比で書き、実行時に求める。
- sleep・nohup・末尾の `&`・先頭の `timeout` は使わない。待つ時は条件で待つ。
- 一時ファイルは `.tmp-*` に置き、終わったら消す。
- commit・PR に、AI が書いたことを示す行を付けない。commit の名義は、環境の設定のまま。
- GitHub に、Issue・ラベル・コメントを書かない。

## 作業の進め方

- 作業の最初に `date -u` を記録する。4 時間 30 分を過ぎたら、受け入れが通らなくても、作業ツリーを commit し、報告を書いて終える(ジョブの上限は 5 時間 50 分)。
- この作業の job(GitHub Actions の runner)で、受け入れの式の全てを実行する。ビルドと E2E は、この job の中で行う。
- 受け入れの式は、ジョブの判定が 1 本で実行する。作業者は、式の段ごとに、シェルの呼び出しを分けて確かめてよい(5 本の run は、1 本ずつ、フォアグラウンドで実行する)。
- ビルドや E2E の長いコマンドは、フォアグラウンドで直接実行し、シェルツールの `timeout` 引数に 5400000 を渡す。背景で起動して待たない。
- ビルドは 1 回にする。tools だけを直した後は、同じ bundle(`lift/out-app/bundle/raw/fl-Oz`)で E2E を走らせて直す。
- 1 回で終わらない前提で、段ごとに commit する。再試行は前回の差分の上で続け、作業ツリーを消さない。
- 受け入れが通らなくても、作業の最後に作業ツリーを commit する(未 commit の変更が残ると、判定は失敗する)。

## 他のタスクとの関係

- blocked_by: #{{220}}(表示の宣言を守る・行ごとの予算・JSON の `displays`)。この Issue は、#{{220}} が閉じてから始まる。
- #{{223}}(topbar・page・resilience・perf)は、この Issue が作る `tools/e2e/lib/layout.mjs` を使う。`lib/page.mjs` と `lib/common.mjs` は、#{{223}} と重なる。#{{222}}(plugins)とは、ファイルが重ならない。衝突したら、作業の最後に `git pull --no-rebase --no-edit` で main を取り込み、衝突を直し、受け入れをもう一度通す。
- #{{224}}(F8 の判定)は、`windows/plugin-f8.mjs` と `zz-f12-plugin.mjs` の判定を決める。この Issue は後始末だけを足す。衝突したら、作業の最後に main を取り込み、直す。
