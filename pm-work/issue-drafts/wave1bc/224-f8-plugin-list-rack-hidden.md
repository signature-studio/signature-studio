---
draft: "224"
title: "F8 で Plugin list を開くと Channel rack が見えないままになり、F12 で Plugin list が残る"
type: Task
milestone: "M2 HiDPI と P2 の測定(P2)"
parent: "133"
blocked_by: []
labels: ["verify", "render"]
size: M
priority: Medium
project: signature-studio/3
---

## 目的

`windows` 領域の 2 つの不具合を確かめて、直す。(1) F8 で Plugin list(`TPlugListForm`)を開いた後、Channel rack(`TStepSeqForm`)が、窓の表で `vis false` のまま見えない。その後の、同じ shard の Channel rack のシナリオが、両方の表示で 12 件ずつ error になる。(2) F12 で、Plugin list が閉じず、窓の表の上の階層に残る。原因が製品なら、製品を直す。E2E の判定の誤りなら、判定を直す。どちらでもない時は、確かめた事実を報告に書く。

## 背景

- 全シナリオを回した初回の run(番号 45854456293。commit 8b3bdeb9)の結果:
  - `windows/plugin-f8`(F8 で Plugin list を開き、閉じる)は、両方の表示で合格。
  - 同じ shard(windows の 6 番目)の Channel rack のシナリオ(`windows/rack-knob-r3-small`・`windows/rack-menu-r2`・`windows/rack-plus` など)が、両方の表示で 12 件ずつ `error`: `Channel rack (TStepSeqForm) is not shown: vis false`。同じ shard の他の部分と、ほかの 8 shard の Channel rack のシナリオは、全て合格。
  - `windows/zz-f12-plugin`(印 `new`。F8 で開いた Plugin list に F12 を送る)は、両方の表示で失敗。`TPlugListForm` が残り、窓の表の上の階層に出る。
- `windows/zz-f12-plugin.mjs` の注釈は、F12 が Plugin list を閉じず、Channel rack・Playlist・Mixer も隠し、2 回目の F12 でも戻らない、と書いている(この注釈は未確認)。
- 関係する関数: `tools/e2e/lib/windows.mjs` の `pluginListOpen`(F8 を送り、窓の表で Plugin list を探し、`closeNew` で閉じる)と `f12CloseAll`(F8 で開いた後、F12 を送り、残った上の階層の窓を数える)。`tools/e2e/lib/common.mjs` の `closeNew`・`newWindows`・`openLids`・`windowTable`。
- #{{221}} は、`windows/plugin-f8.mjs` と `zz-f12-plugin.mjs` に、後始末(Plugin list を閉じ、Channel rack を出す)を足す。この Issue は、判定の原因を確かめる。

## 読む物

- `tools/e2e/lib/windows.mjs`(`pluginListOpen`・`f12CloseAll`・`cls_for`・`done`・`KEYS`)・`tools/e2e/lib/common.mjs`(`closeNew`・`newWindows`・`openLids`・`windowTable`)・`tools/e2e/lib/dump.mjs`
- `tools/e2e/scenarios/windows/plugin-f8.mjs`・`zz-f12-plugin.mjs`・同じ shard の Channel rack のシナリオ(`tools/e2e/scenarios/windows/rack-*.mjs`)
- 製品: `app/src/`(キーの扱い。`grep -rn` で F8・F12 の扱いを探す)と、`lift/` の窓の表示(F8・F12 の経路)。場所は作業で確かめる。
- `tools/e2e/README.md`(判定の基準)

## やること

0. 作業の最初に `date -u` を記録する。`.prior-work.md` が在れば、最初に読む。
1. **F8 の後の状態を見る**: 1 つの表示(4k-dpr1)で、F8 の前・Plugin list が開いた時・Plugin list を閉じた後、の 3 つの時点で、窓の表の `TStepSeqForm`(Channel rack)と `TPlugListForm`(Plugin list)の、`vis`・矩形・前後の順を読む。併せて、Channel rack の矩形の画素が変わったかを、ハッシュで見る。
2. **原因を切り分ける**: Channel rack が `vis false` になるのは、(a) 製品が Channel rack を隠す(窓の表から可視が外れる)か、(b) 前面の窓(Plugin list など)が残り、E2E の可視の読みが `false` を返すか、(c) 窓の表からの読みが誤っているか、のどれか。(1) の読みで、どれかを決める。
3. **F12 の後の状態を見る**: 同じく、F12 の前後で、窓の表の上の階層の窓(`TPlugListForm`・`TStepSeqForm`・`TEventEditForm`・`TFXForm`)の `vis` を読む。F12 が Plugin list を閉じないのか、他の窓も隠すのか、2 回目の F12 で戻るのかを確かめる。
4. **直す**: (a) 製品の欠陥なら、製品を直す(`app/`・`lift/` の最小の差分)。(b)・(c) の E2E の判定の誤りなら、`windows/plugin-f8.mjs`・`zz-f12-plugin.mjs`・`lib/windows.mjs` の判定を直す。根拠(窓の表の行と画素)を、報告に書く。どれでもない時は、「確認できない」と書き、確かめた事実を残す。
5. **宣言に 4k-dpr2 を足す**: `windows/plugin-f8.mjs` と `windows/zz-f12-plugin.mjs` の宣言の `displays` に `'4k-dpr2'` を足す(両方の表示で確かめる)。
6. **印**: `windows/zz-f12-plugin` が ok になったら、印 `new` を外す。ok にならないなら、残すか、症状が一致する開いている Issue に付け替える(報告に書く)。
7. 受け入れを通す。

## 受け入れ

```accept
npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false?0:1)' "$RUNNER_TEMP/boot.json" && node tools/smoke/welcome-flow.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/flow.jsonl" && node -e 'const fs=require("fs");const L=fs.readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const F=(f)=>L.find((r)=>r.flow===f);const ok=(r)=>!!r&&r.error===null&&r.state==="engine-idle"&&r.standIn===false&&r.trapped===false&&r.hostErrors===0&&r.opened===true;process.exit(L.length===4&&["A","B","C","D"].every((f)=>ok(F(f)))&&F("B").welcomeClosed===true&&F("D").welcomeClosed===true?0:1)' "$RUNNER_TEMP/flow.jsonl" && npm test --prefix app && (cd tools/e2e && npm ci && npx playwright install chromium) && node --test tools/e2e/test/*.test.mjs && node tools/e2e/run.mjs --area windows --only windows/plugin-f8,windows/zz-f12-plugin --displays 4k-dpr1,4k-dpr2 --budget 40m > "$RUNNER_TEMP/f8.jsonl" && node -e 'const L=require("fs").readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const S=L.flatMap((r)=>r.scenarios);process.exit(L.length===2&&L.every((r)=>r.scenarios.length===2)&&S.every((s)=>s.ok===true&&s.expectFail===null&&s.skipped!==true)?0:1)' "$RUNNER_TEMP/f8.jsonl"
```

判定の基準:
- 受け入れの式が、終了コード 0 で終わる。`windows/plugin-f8` と `windows/zz-f12-plugin` が、両方の表示(4k-dpr1 と 4k-dpr2)で、ok で、印なし。
- 報告に: (a) 窓の表の行(F8 の前・開いた時・閉じた後の `TStepSeqForm` と `TPlugListForm` の `vis`・矩形・前後の順。F12 の前後の上の階層の窓)、(b) 原因の切り分け(製品・判定・確認できない)と、その根拠の出力の行、(c) 直した差分の場所(製品なら `app/`・`lift/` の差分の要点)、(d) 印の扱い。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には、runner の値であることと、nproc と loadavg を添える。
- 「達成」「直った」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。出力の行を付ける。
- 確認できなかった物は「確認できない」と書く。
- 段ごとに、やること 0〜7 のどこまで進んだか(完了・途中・未着手)を書く。

## 制約

- 製品の変更は、原因が製品だと確かめた時だけ。最小の差分にする。`.github/`・`spec/`・`ci/` は変えない。
- 製品に、利用者向けの新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足さない。オプトインを作らない。テストのための分岐を製品に入れない。
- Python を使わない(調査と検証にだけ使う)。OCR を使わない。文字は、ゲストのメモリの文字列・窓の表・画素のハッシュで読む。
- 禁止されているのは、AI のベンダー名と Cloudflare の識別子だけ。FL の製品名・インストール先の名前・変数名は、書いてよい。
- 期待値に、実行時の値(DPR・DPI・窓の位置)を定数で書かない。
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

- blocked_by: なし。この Issue は、#{{221}} の `layout.mjs` に依らない(窓の表の読みだけ)。
- #{{221}} が、同じ 2 本のシナリオ(`windows/plugin-f8.mjs`・`zz-f12-plugin.mjs`)に後始末を足す。この Issue は判定の原因を決める。衝突したら、作業の最後に `git pull --no-rebase --no-edit` で main を取り込み、衝突を直し、受け入れをもう一度通す。
