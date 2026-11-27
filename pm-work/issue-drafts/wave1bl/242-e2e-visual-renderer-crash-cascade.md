---
draft: "242"
title: "E2E の visual: 描画の停止(Target crashed)の後、残りの 49 本が 1 ms で error になる(ページの作り直しと、error を受理しない判定)"
type: Task
milestone: "M2 HiDPI と P2 の測定(P2)"
parent: "133"
blocked_by: []
labels: ["verify", "render"]
size: M
priority: High
project: signature-studio/3
---

## 目的

`visual`(155 本)の shard 3/3 で、1 本の描画プロセスの停止(`page.evaluate: Target crashed`)の後、残り 49 本が、すべて 1 ms ほどで `Target crashed` の error になった。さらに、この 49 本のうち 15 本は印 `new` の付いた本で、error のまま受理された(判定が error を見ていない)。この Issue は、(1) 停止の後にページを作り直して、後続の本を正しく走らせる、(2) error を持つ結果を、印の有無によらず受理しない、(3) 停止の原因(製品か実行環境か)を切り分ける。製品(`app/`・`lift/`)は、原因が製品と分かるまで変えない。

## 背景

- 全シナリオの E2E(`e2e-full` の最新の run。commit 24aeee99 の版)の結果(実測):
  - visual 3/3 の shard は 51 本。最初の 2 本(`visual/black-dpr-1-5`・`visual/black-dpr-2-5`)は合格。3 本目の `visual/black-dpr-random-2`(DPR を 2.06 にしてから黒い区画を数える。約 65 秒)で、描画プロセスが落ちた(`page.evaluate: Target crashed`)。
  - その後の 49 本(`visual/cursor-window-edge`・`visual/dpr-1-5-edge`・`visual/dpr-2-topbar`・`visual/language-de-settings` など)は、すべて `Target crashed` の error で、ms は 0〜1。ページが死んだまま、次の本が走った。
  - 内訳: 印の無い失敗 34 本、印 `new` の失敗 15 本(error のまま受理された)。
- 受理の穴(実測・コード): `tools/e2e/lib/plan.mjs` の `scenarioPasses` は、印つきの結果について、`pageErrors`・`consoleErrors`・`failedRequests` と停止しか見ず、`error` を見ない。そのため、error を持つ印つきの本が、row の受理を通ってしまう。
- 推定: DPR を変える操作(`Emulation.setDeviceMetricsOverride`)の後の読み(`picture`・`windowTable`)で、描画プロセスが落ちた。製品の欠陥か、実行環境の資源の不足かは、確認できない(この run では画像もエンジンのログも取っていない)。
- 本ではない 12 本の visual の失敗(`visual/theme-*-golden`・`visual/*-edge`・`visual/picture-start` など)は、この Issue の対象にしない。

## 読む物

- `tools/e2e/lib/plan.mjs`(`scenarioPasses`・`rowPasses`・`needsFailLog`)と `tools/e2e/test/`(対応する試験の置き場)
- `tools/e2e/run.mjs`(`runRow`・ページの開始と、本ごとの `resetProject`)・`tools/e2e/lib/common.mjs`(`startProject`・`resetProject`)
- `tools/e2e/lib/visual.mjs`(`blackDpr`・`setDpr`・`restoreDpr`・`blackReport`)・`tools/e2e/lib/picture.mjs`
- `tools/e2e/scenarios/visual/black-dpr-random-2.mjs`・`visual/cursor-window-edge.mjs`
- `tools/e2e/lib/browser.mjs`(`launchBrowser`・ページの作成)

## やること

0. 作業の最初に `date -u` を記録する。`.prior-work.md` が在れば、最初に読む。
1. 受理の穴を直す: `scenarioPasses` が、`error` のある結果を、印の有無によらず失敗にする。試験(`tools/e2e/test/`)を先に書き、直す前に、印つきの error が通ることを確かめてから直す。
2. 停止の後の作り直し: `page.evaluate` の `Target crashed`(描画プロセスの停止)を、その本の `error` として記録し、ページを作り直して、次の本を続ける。作り直しの後に、`resetProject` の前提(新しい曲)が満たされることを確かめる。残りの本を 1 ms で error にしない。
3. 原因の切り分け: `node tools/e2e/run.mjs --only visual/black-dpr-random-2 --displays 4k-dpr1 --engine-logs <DIR> --budget 20m` を 3 回、フォアグラウンドで走らせる。落ちる時は、落ちた操作(`setDpr`・`picture`・`windowTable` のどれか)を特定し、その時のエンジンのログを保存する。製品の欠陥と分かったら、製品の Issue は起票せず、報告に書く。実行環境の資源が原因なら、報告に書く。
4. 2.06 の DPR で再現しない時は、同じ操作を 2 と 3 の DPR(例 2.5・1.5)で試し、どの DPR で落ちるかを報告に書く。
5. 受け入れを通す。通らない時は、落ちた ID・表示・出力の 1 行を報告に書く。

## 受け入れ

```accept
npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false?0:1)' "$RUNNER_TEMP/boot.json" && (cd tools/e2e && npm ci && npx playwright install chromium) && node --test tools/e2e/test/*.test.mjs && node tools/e2e/run.mjs --area visual --shard 3/3 --budget 35m --displays 4k-dpr1 --report-only > "$RUNNER_TEMP/v33.jsonl" && node -e 'const L=require("fs").readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const S=L.flatMap((r)=>r.scenarios);const e=S.filter((s)=>s.error!==null).length;const sk=S.filter((s)=>s.skipped===true).length;process.exit(L.every((r)=>r.error===null)&&e<=1&&sk===0&&S.length>0?0:1)' "$RUNNER_TEMP/v33.jsonl"
```

判定の基準:
- 受け入れの式が、終了コード 0 で終わる(visual 3/3 の run で、error を持つ本が 1 本以下、skip が 0)。
- `tools/e2e/test/` に、印つきで error を持つ結果が落ちる試験が在り、直す前に通らないことを確かめてから直した(報告に、直す前の試験の失敗の行を書く)。
- 報告に: (a) 受理の穴の直し方と、試験の行、(b) 作り直しの後に、次の本が ok で走った件数、(c) 停止の切り分けの結果(製品か、実行環境か、確認できないか。3 回の結果の行)、(d) DPR を変えて試した結果、(e) 残りの visual の失敗 12 本は、この Issue の対象外と書いたこと。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には、runner の値であることと、nproc と loadavg を添える。
- 「達成」「直った」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。出力の行を付ける。
- 確認できなかった物は「確認できない」と書く。
- 段ごとに、やること 0〜5 のどこまで進んだか(完了・途中・未着手)を書く。

## 制約

- 製品(`app/`・`lift/`)は変えない(読むだけ)。製品の欠陥と分かった場合は、報告に書く。`.github/`・`spec/` は、直す物がある時だけ変える。
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
- 1 回で終わらない前提で、段ごとに commit する。再試行は前回の差分の上で続け、作業ツリーを消さない。
- 受け入れが通らなくても、作業の最後に作業ツリーを commit する(未 commit の変更が残ると、判定は失敗する)。`git add` は 1 つずつ、自分が変えたパスだけを指定する。

## 他のタスクとの関係

- #1509(topbar・page・resilience・perf)と #1507(windows・settings)は、`tools/e2e/lib/plan.mjs` の判定や `lib/common.mjs` の `resetProject` に触れる可能性がある。衝突したら、作業の最後に `git pull --no-rebase --no-edit` で main を取り込み、衝突を直し、受け入れをもう一度通す。
