---
draft: "245"
title: "E2E の settings: 前回は DPR 1 で合格していた 14 本が落ちる(テーマ 11・表示 2・波形 1)。共通の準備(resetProject・Problems の箱)の変更が候補"
type: Task
milestone: "M2 HiDPI と P2 の測定(P2)"
parent: "133"
blocked_by: []
labels: ["verify"]
size: S
priority: High
project: signature-studio/3
---

## 目的

`settings`(150 本)で、印の無い失敗が 14 件ある。前回の全シナリオの E2E(commit 8b3bdeb9 の版)では、これらの本は DPR 1 で合格していた。今回(commit 24aeee99 の版)は、DPR 1 だけで走らせて落ちている。この Issue は、退行の原因を、道具の変更(`tools/e2e/lib/common.mjs`・`lib/flow.mjs` の、2 つの commit 間の変更)か製品の変更(`app/` の commit)かに切り分け、直す。本の判定(テーマが画に反映されること)は変えない。`#1507` は DPR 2 の位置を扱い、この Issue は DPR 1 の退行を扱う。

## 背景

- 落ちた 14 本(実測。`e2e-full` の最新の run。commit 24aeee99 の版): `settings/theme-ash`・`theme-dark`・`theme-dark-orange`・`theme-light-cherry`・`theme-light-pear`・`theme-light-tangerine`・`theme-ppriime-dark`・`theme-ppriime-light`・`theme-sytrus`・`theme-ultra-green`・`theme-veela`(テーマ 11 本)、`settings/check-display-force-refreshes`・`settings/check-display-transparent-menus`(表示の 2 本)、`settings/choice-open-colorful-waveforms`(波形の 1 本)。
- テーマの本の notes(実測): `engine names its file false; picture changed false; engine names the file after reload true; picture same after reload false; bands differing after reload [5,7] of 8`。テーマを選んでも、その時点では画が変わらず、再読込の後に画が変わる(帯の差)。`the page was reloaded` の文言がある。
- 表示の本の notes(実測): `/^Display/ box at 309,266: changed false; picture after reload same true (not gated: unexplained, see the report)`。波形の本: `/^Display/ at 320,320: popups 0 (none); closed by Escape true`。
- 前回の run の DPR 1 の結果(実測): settings の印の無い失敗は、すべて DPR 2 の本だった(前回の所見の表)。今回の 14 本は、DPR 1 で、前回は合格していた。
- 差分(実測): `git diff 8b3bdeb9 24aeee99 -- tools/e2e/scenarios/settings tools/e2e/lib/settings.mjs` は空。変わったのは、`tools/e2e/lib/common.mjs` の `resetProject`(New を Enter で行う。前に `clearProblems` で Problems の箱を閉じる)と `closeNew`(窓が閉じるまで待つ)、`tools/e2e/lib/flow.mjs` の変更(Problems の箱を Enter で閉じる)。commit は 4741abb8 と、その後の #1510 の変更。
- 製品の変更: 同じ期間に `app/` の commit が 8 件あり、テーマや表示の設定に関わる物は、確認した範囲では無い(`dd289209` はツマミの右クリックのクリップボード)。
- 推定: 本の始まりの状態(開いている箱・新しい曲)が、`resetProject` の変更で変わり、テーマの選択が効く前に、箱が画を覆うか、Enter が別の物に当たっている。
- 確認できない: 前回と今回の両方で、テーマが画に反映される順序の正しい仕様(製品の仕様)。

## 読む物

- `tools/e2e/lib/common.mjs`(`resetProject`・`closeNew`・`waitFewer`・`clearProblems` の呼び出し)
- `tools/e2e/lib/flow.mjs`(`clearProblems`・Problems の箱の扱い)
- `tools/e2e/lib/settings.mjs`(`themeRun`・`withSettings`・`goTab`・`languageRun`。読むだけ)
- `tools/e2e/scenarios/settings/theme-*.mjs`・`check-display-*.mjs`・`choice-open-colorful-waveforms.mjs`(宣言は `4k-dpr1`)
- `git log --oneline 8b3bdeb9..24aeee99 -- tools/e2e/lib/common.mjs tools/e2e/lib/flow.mjs`(変更の一覧)

## やること

0. 作業の最初に `date -u` を記録する。`.prior-work.md` が在れば、最初に読む。
1. 切り分け(道具か製品か): 作業ツリーの複写(`.tmp-*` の中)で、`tools/e2e/lib/common.mjs` と `tools/e2e/lib/flow.mjs` を 8b3bdeb9 の版に戻し、同じ 14 本を DPR 1 で走らせる。合格するなら、道具の変更が原因。落ちたままなら、製品の変更が原因の候補。報告に、両方の結果の行を書く。
2. 道具が原因の時: 変えた準備(`resetProject` の Enter・`clearProblems` の Enter)のうち、どれが本の画を変えるかを、1 つずつ戻して確かめる。原因の変更を、本の前提を満たす形に直す(New を Enter で行う意味を保つ。Problems の箱の扱いは、箱が残らない形にする)。
3. 製品が原因の時: 変更の commit を特定し(二分探索。`git bisect` は使わず、commit の一覧を手で絞る)、製品の欠陥として報告に書く。この Issue では製品を直さない。
4. 直した後、14 本すべてが、DPR 1 で、印の無い本として ok になることを確かめる。
5. 受け入れを通す。通らない時は、落ちた ID・出力の 1 行を報告に書く。

## 受け入れ

```accept
npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false?0:1)' "$RUNNER_TEMP/boot.json" && (cd tools/e2e && npm ci && npx playwright install chromium) && node --test tools/e2e/test/*.test.mjs && node tools/e2e/run.mjs --only settings/theme-ash,settings/theme-dark,settings/theme-dark-orange,settings/theme-light-cherry,settings/theme-light-pear,settings/theme-light-tangerine,settings/theme-ppriime-dark,settings/theme-ppriime-light,settings/theme-sytrus,settings/theme-ultra-green,settings/theme-veela,settings/check-display-force-refreshes,settings/check-display-transparent-menus,settings/choice-open-colorful-waveforms --displays 4k-dpr1 --budget 60m --report-only > "$RUNNER_TEMP/settings.jsonl" && node -e 'const L=require("fs").readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const S=L.flatMap((r)=>r.scenarios);process.exit(L.every((r)=>r.error===null)&&S.length===14&&S.every((s)=>s.skipped!==true&&s.expectFail===null&&s.ok===true&&s.pageErrors===0&&s.consoleErrors===0&&s.failedRequests===0)?0:1)' "$RUNNER_TEMP/settings.jsonl"
```

判定の基準:
- 受け入れの式が、終了コード 0 で終わる(14 本すべてが、印なしで ok)。
- 報告に: (a) 切り分けの結果(道具の変更を戻した時の 14 本の結果の行と、戻さない時の結果の行)、(b) 原因の変更(道具なら、変えた行と理由。製品なら、commit の SHA と、製品の欠陥の説明)、(c) 14 本の出力の行(ID・ok・notes の 1 行)。
- `#1507`(DPR 2 の位置)の受け入れとは別。この Issue で直した物と、`#1507` の範囲を、報告に分けて書く。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には、runner の値であることと、nproc と loadavg を添える。
- 「達成」「直った」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。出力の行を付ける。
- 確認できなかった物は「確認できない」と書く。
- 段ごとに、やること 0〜5 のどこまで進んだか(完了・途中・未着手)を書く。

## 制約

- 製品(`app/`・`lift/`)は変えない(読むだけ)。製品が原因なら、報告に書く。
- 本の判定(テーマが画に反映されること・帯の差の基準)を緩めない。判定を外して ok にしない。
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

- #1507(windows・settings の DPR 2 の位置)とは、同じ `settings` の領域だが、対象の DPR と原因が違う。ファイルの衝突は、`lib/common.mjs` の `resetProject` だけで起こりうる。衝突したら、作業の最後に main を取り込み、直し、受け入れをもう一度通す。
- `#1509`(topbar・page・resilience・perf)とも、`lib/common.mjs` を共有する。
