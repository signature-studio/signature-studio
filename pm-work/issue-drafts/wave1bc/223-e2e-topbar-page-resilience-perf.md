---
draft: "223"
title: "E2E の topbar・page・resilience・perf を直す"
type: Task
milestone: "M2 HiDPI と P2 の測定(P2)"
parent: "133"
blocked_by: ["220", "221"]
labels: ["verify", "render"]
size: L
priority: High
project: signature-studio/3
---

## 目的

`topbar`(63 本)・`resilience`(42 本)・`perf`(31 本)の判定と手順を直す。`page`(84 本)の残りの失敗は、#{{220}}(DPR の宣言)と #{{221}}(黒い区画の分母)が直す。この Issue では、`page` が、その後で受け入れの式を通ることを確かめる。`plugins` は、#{{222}} が扱う。製品(`app/`・`lift/`)は変えない。

## 背景

- topbar(63 本)。初回の run:
  - 印なしの失敗が 12 件(dpr1 が 3 件、dpr2 が 9 件)。例: `topbar/wheel-tquickedittoolbar-2`・`topbar/wheel-tquickedittoolbarpanel-11`・`topbar/wheel-tquickedittoolbarpanel-3`。wheel・click・drag の効果が出ない。
  - 印 `new` の 23 本。両方の表示で `no effect (picture unchanged; opened none; menu none)`(dpr1 と dpr2 で、製品の状態は同じ)。画像が無いので、位置の誤りか、部品の機能が無いのかは、確認できない。
  - 部品の位置は、窓の表の矩形から取っている(今の読み)。
  - topbar の 63 本は、宣言が既定(`4k-dpr1`・`4k-random`)で、dpr2 では走らない。
- resilience(42 本)。初回の run:
  - `resilience/demo-open-replace-50`: error。Ctrl+O で開く箱に答えないまま、次に進んだ(履歴では 2 件が答えられていた)。
  - `resilience/wait-idle-30min`: 30 分の固定の待ち。dpr1 の行で予算が切れ、skip された。
  - `resilience/tab-hidden-repeat`: 隠す・表示の繰り返しで、描画が戻らない回が 0 回。確認できない。
  - 印 `new` の `resilience/random-ops-*` の一部(01・05・07・09 など)は、run で ok になった。印が古い可能性がある。
  - `resilience/stop-chord-tool` は、印 `#1386`(開いている。Piano roll のコード進行ツールの停止)。この印の扱いは、この Issue の印の見直し(やること 6)の対象に入らない。
- perf(31 本)。初回の run の比の超過は 5 件:
  - `perf/cpu-playing-busy`: busy 1.77 % / 基準 0.80 % = 2.2 倍(上限 1.5)。
  - `perf/latency-ab-vs-base`・`perf/latency-menu-open-options`: クリック 12.5 ms / 基準 8.2 ms(上限 1.5)。
  - runner の load は 2.4〜2.9(4 コア)。基準(`tools/e2e/golden/perf/ab-base.tsv`・`baseline.tsv`)は、別の機械の値。比の分母には使えない。
- page(84 本)。初回の run では、`page/frame-no-black-in-windows`・`page/reload-picture-after`(黒い区画。#{{221}} が直す)、`page/crisp-screen-pixels`(宣言 `dpr:1` の DPR 2 での実行。#{{220}} が直す)の 3 本が、未印で失敗した。
- 閉じた番号の印: `plugins/` の中にしか無い(#{{222}} が直す)。この Issue では、plugins 以外の印を見直す。

## 読む物

- `tools/e2e/lib/topbar.mjs`(`partOf`・`findPart`・`presentPart`・`partOperation`)・`tools/e2e/scenarios/topbar/*.mjs`(63 本)
- `tools/e2e/lib/common.mjs`(`windowTable`・`closeNew`・`guestText`)・`tools/e2e/lib/page.mjs`
- `tools/e2e/lib/resilience.mjs`・`tools/e2e/scenarios/resilience/demo-open-replace-50.mjs`・`wait-idle-30min.mjs`・`tab-hidden-repeat.mjs`・`random-ops-*.mjs`
- `tools/e2e/lib/flow.mjs`(`openProjectFile`・`dismissProblems`・`waitProjectOpen`)
- `tools/e2e/lib/perf.mjs`(`Judge`・`baseOf`・`LIMIT`・`loadNow`)・`tools/e2e/scenarios/perf/*.mjs`
- `tools/e2e/lib/plan.mjs` と #{{220}} が作る予算の仕組み(行ごとの予算。残りの予算をシナリオに渡す)
- #{{221}} が作る `tools/e2e/lib/layout.mjs`(位置の方式)

## やること

0. 作業の最初に `date -u` を記録する。`.prior-work.md` が在れば、最初に読む。
1. **topbar の宣言**: `tools/e2e/scenarios/topbar/*.mjs` の 63 本の宣言の `displays` に `'4k-dpr2'` を足す(既定の `4k-dpr1`・`4k-random` は残す)。
2. **topbar の位置**: 部品の位置が、窓の表の矩形から、#{{221}} の `layout.mjs` の方式で求められているかを確かめる。dpr2 の `no effect` が、位置の誤りか、機能が無いのかを、画素と窓の表の矩形で切り分ける。機能が無い物だけ、印 `new` のまま残す。一覧(シナリオ名・部品・症状・切り分けの根拠)を報告に書く(PM が製品の欠陥として起票する)。
3. **resilience の demo**: `resilience/demo-open-replace-50` の Ctrl+O の箱を、`lib/flow.mjs` の `openProjectFile` で答える。答えた後に、曲のタイトルが変わることを確かめる。
4. **resilience の待ち**: `resilience/wait-idle-30min` は、行の残りの予算(#{{220}} が渡す値)が 30 分より少なければ始めない(結果は skip。報告に書く)。宣言は既に `4k-dpr1` だけ。
5. **perf の比**: `tools/e2e/scenarios/perf/*.mjs` の比は、`golden/perf/ab-base.tsv` と `baseline.tsv` を使わず、同じ run の中の 2 回の測定で取る。基準の側は、同じ機械・同じ操作で、比べる条件だけを変えた 2 回目の測定(例: 負荷を掛けない回と、掛けた回)。どの条件を基準にするかは作業者が決め、理由を報告に書く。上限の倍率(`LIMIT`)の値は変えない。
6. **印 new の見直し**(plugins 以外): 対象は topbar の印 `new` の 23 本、resilience の印 `new` の 8 本、perf の印 `new` の 1 本。(a) この Issue の修正で ok になる物は、印を外す。(b) ok にならない物は、印 `new` を残すか、症状が同じ開いている Issue に付け替える(付け替えは、題名の症状が一致する物だけ)。開いている Issue は次の通り: #1386(Piano roll のコード進行ツールの停止)・#1395(英語以外の言語で起動)・#1429(リフ マシーンの呼び出しの停止)・#1499(Plugin Manager の起動)・#1502(Recent の保持)・#1503(ツマミの右クリックの読み取り)・#1504(DEDZII の fx-status)・#1505(Theme の頁のタイル)。(c) 今の run で ok になった印 `new` は、印を外す(印が古い)。どの物を外したかを、根拠の出力の行つきで報告に書く。
7. **page の確認**: `page` の失敗が、#{{220}}・#{{221}} の後に 3 本以下であることを、受け入れの式で確かめる。超えたら、残りの ID と理由を報告に書く。
8. 受け入れを通す。通らない shard は、落ちた ID・表示・出力の 1 行を報告に書く。

## 受け入れ

```accept
npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false?0:1)' "$RUNNER_TEMP/boot.json" && node tools/smoke/welcome-flow.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/flow.jsonl" && node -e 'const fs=require("fs");const L=fs.readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const F=(f)=>L.find((r)=>r.flow===f);const ok=(r)=>!!r&&r.error===null&&r.state==="engine-idle"&&r.standIn===false&&r.trapped===false&&r.hostErrors===0&&r.opened===true;process.exit(L.length===4&&["A","B","C","D"].every((f)=>ok(F(f)))&&F("B").welcomeClosed===true&&F("D").welcomeClosed===true?0:1)' "$RUNNER_TEMP/flow.jsonl" && npm test --prefix app && (cd tools/e2e && npm ci && npx playwright install chromium) && node --test tools/e2e/test/*.test.mjs && node tools/e2e/run.mjs --area topbar --shard 1/3 --displays 4k-dpr2 --budget 40m --report-only > "$RUNNER_TEMP/tb13d2.jsonl" && node -e 'const L=require("fs").readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const S=L.flatMap((r)=>r.scenarios);const u=S.filter((s)=>s.expectFail===null&&s.ok!==true&&s.skipped!==true).length;process.exit(L.length>=1&&S.length>0&&L.every((r)=>r.error===null&&r.servedHeadersMatch===true)&&u<=3?0:1)' "$RUNNER_TEMP/tb13d2.jsonl" && node tools/e2e/run.mjs --area page --shard 2/3 --budget 40m --report-only > "$RUNNER_TEMP/pg23.jsonl" && node -e 'const L=require("fs").readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const S=L.flatMap((r)=>r.scenarios);const u=S.filter((s)=>s.expectFail===null&&s.ok!==true&&s.skipped!==true).length;process.exit(L.length>=1&&S.length>0&&L.every((r)=>r.error===null&&r.servedHeadersMatch===true)&&u<=3?0:1)' "$RUNNER_TEMP/pg23.jsonl" && node tools/e2e/run.mjs --area perf --shard 1/2 --budget 40m --report-only > "$RUNNER_TEMP/pf12.jsonl" && node -e 'const L=require("fs").readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const S=L.flatMap((r)=>r.scenarios);const u=S.filter((s)=>s.expectFail===null&&s.ok!==true&&s.skipped!==true).length;process.exit(L.length>=1&&S.length>0&&L.every((r)=>r.error===null&&r.servedHeadersMatch===true)&&u<=3?0:1)' "$RUNNER_TEMP/pf12.jsonl" && node tools/e2e/run.mjs --area resilience --shard 1/2 --budget 40m --report-only > "$RUNNER_TEMP/rs12.jsonl" && node -e 'const L=require("fs").readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const S=L.flatMap((r)=>r.scenarios);const u=S.filter((s)=>s.expectFail===null&&s.ok!==true&&s.skipped!==true).length;process.exit(L.length>=1&&S.length>0&&L.every((r)=>r.error===null&&r.servedHeadersMatch===true)&&u<=3?0:1)' "$RUNNER_TEMP/rs12.jsonl"
```

判定の基準:
- 受け入れの式が、終了コード 0 で終わる。4 本の run(topbar の 1/3 を 4k-dpr2、page の 2/3、perf の 1/2、resilience の 1/2)のそれぞれで、未印で ok が偽(skipped を除く)のシナリオが 3 本以下。
- topbar の 63 本の宣言に `4k-dpr2` が在る。
- 試験(`node --test tools/e2e/test/*.test.mjs`)が全て通る。
- 報告に: (a) topbar の一覧(シナリオ名・部品・症状・切り分けの根拠。本当に効かない物だけ、印 `new` が残る)、(b) resilience の demo と待ちの結果(skip の有無と、その時の残りの予算)、(c) perf の基準の側の決め方と、比の値(同じ run の中の値)、(d) 印の見直しの一覧(外した物・付け替えた物・残した物。根拠の出力の行つき)、(e) 4 本の run の出力の行(各 run の未印の失敗の ID)、(f) page の残りの失敗(#{{220}}・#{{221}} の後に残った ID と理由)。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には、runner の値であることと、nproc と loadavg を添える。
- 「達成」「直った」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。出力の行を付ける。
- 確認できなかった物は「確認できない」と書く。
- 段ごとに、やること 0〜8 のどこまで進んだか(完了・途中・未着手)を書く。

## 制約

- 製品(`app/`・`lift/`)は変えない(読むだけ)。`.github/`・`spec/`・`ci/` も変えない。
- 製品に、利用者向けの新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足さない。オプトインを作らない。テストのための分岐を製品に入れない。
- Python を使わない(調査と検証にだけ使う)。OCR を使わない。文字は、ゲストのメモリの文字列・窓の表・画素のハッシュで読む。
- 禁止されているのは、AI のベンダー名と Cloudflare の識別子だけ。FL の製品名・インストール先の名前・変数名は、書いてよい。
- 期待値に、実行時の値(DPR・DPI・窓の位置・時間)を定数で書かない。
- sleep・nohup・末尾の `&`・先頭の `timeout` は使わない。待つ時は条件で待つ。
- 一時ファイルは `.tmp-*` に置き、終わったら消す。
- commit・PR に、AI が書いたことを示す行を付けない。commit の名義は、環境の設定のまま。
- GitHub に、Issue・ラベル・コメントを書かない。

## 作業の進め方

- 作業の最初に `date -u` を記録する。4 時間 30 分を過ぎたら、受け入れが通らなくても、作業ツリーを commit し、報告を書いて終える(ジョブの上限は 5 時間 50 分)。
- この作業の job(GitHub Actions の runner)で、受け入れの式の全てを実行する。ビルドと E2E は、この job の中で行う。
- 受け入れの式は、ジョブの判定が 1 本で実行する。作業者は、式の段ごとに、シェルの呼び出しを分けて確かめてよい(4 本の run は、1 本ずつ、フォアグラウンドで実行する)。
- ビルドや E2E の長いコマンドは、フォアグラウンドで直接実行し、シェルツールの `timeout` 引数に 5400000 を渡す。背景で起動して待たない。
- ビルドは 1 回にする。tools だけを直した後は、同じ bundle(`lift/out-app/bundle/raw/fl-Oz`)で E2E を走らせて直す。
- 1 回で終わらない前提で、段ごとに commit する。再試行は前回の差分の上で続け、作業ツリーを消さない。
- 受け入れが通らなくても、作業の最後に作業ツリーを commit する(未 commit の変更が残ると、判定は失敗する)。

## 他のタスクとの関係

- blocked_by: #{{220}}(表示の宣言・行ごとの予算・JSON の `displays`)と #{{221}}(`layout.mjs`・黒い区画の分母)。両方が閉じてから始まる。
- #{{222}}(plugins)とは、ファイルが重ならない。`lib/page.mjs` と `lib/common.mjs` は #{{221}} と重なる。衝突したら、作業の最後に `git pull --no-rebase --no-edit` で main を取り込み、衝突を直し、受け入れをもう一度通す。
