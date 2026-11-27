---
draft: "247"
title: "E2E の visual: 印の無い失敗 12 本(テーマ・窓の縁の画素の比較)を画像で切り分ける(道具の誤り・製品の見た目の変化・確認できない)"
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

`visual`(155 本)の shard 1/3 と 2/3 で、印の無い失敗が合計 12 本ある。いずれも error は無く、画素の比較(ゴールデン)か、窓の縁の判定(edge)の失敗である(実測)。画像もエンジンのログも取っていないので、原因は確認できない。この Issue は、12 本を画像で切り分ける。製品の見た目が変わったのか、道具の窓の位置や判定の式がずれたのか、どちらか分からないのか。道具の誤りは直し、製品の変化は報告に書く。基準(ゴールデン)の書き換えは、理由を報告に書いた時だけにする。製品(`app/`・`lift/`)は変えない。

## 背景

- 全シナリオの E2E(`e2e-full` の最新の run。commit 24aeee99 の版)の結果(実測): visual 1/3 の印の無い失敗が 5 件、2/3 が 7 件。合計 12 件。error は無い。
- 内訳の例(実測の本の名前): `visual/theme-*-golden` が 6 件(テーマの画素の比較)。残りは `visual/theme-*-edge`・`visual/window-*-edge`・`visual/window-tedgebrowser-golden`・`visual/picture-start` などの中から。12 件の ID は、run の出力で確かめる。
- 一部の notes は `display 4k-dpr1 only (the golden keys carry the ratio...)` と書く(実測)。
- 仕組み(`tools/e2e/README.md` の「見た目(visual)のゴールデン」): 基準は `golden/visual/<kind>.tsv`(キー・ハッシュ・許容誤差)と、同じ場所の `<kind>.cells.tsv`(矩形を 16 x 16 の区画に分けた、区画ごとの平均の色)。ハッシュが違う時は区画の判定を使う。平均の色が 8(255 中)を超えて動いた区画の割合が、許容誤差(既定 0.02。256 区画のうち 5 区画まで)以内なら `within`、超えたら `differs`。縁の判定は、窓の矩形の 4 辺の対比・縁の幅・柔らかい帯(`lib/visual.mjs`)。
- 見立て(推定): (a) テーマの基準が、今の製品の既定の描画と違う(製品の変更の結果)。(b) 窓の矩形の取り方(DPR 1 の固定の座標)がずれている(道具)。(c) 窓の縁の判定が、今の枠(影・角)と合わない(道具)。どれかは、確認できない。
- #1528 は、visual 3/3 の描画の停止と、error の受理を扱う。この 12 件は、#1528 の本文が「この Issue の対象にしない」と書いている範囲で、#1528 とは別に扱う。

## 読む物

- `tools/e2e/lib/visual.mjs`(`judgeGolden`・縁の判定・`recover`)、`tools/e2e/lib/picture.mjs`(`regionHash`・区画)
- `tools/e2e/golden/visual/`(`window`・`theme`・`language`・`dpr`・`start` の基準の表と、区画の表)
- `tools/e2e/scenarios/visual/`(12 件の宣言。`theme-*-golden`・`*-edge`・`picture-start`)
- `tools/e2e/README.md` の「見た目(visual)のゴールデン」の節(更新の規則)
- 製品: 読むだけ。テーマの色と窓の縁の描画に関わる箇所は、見立てのために読んでよい。

## やること

0. 作業の最初に `date -u` を記録する。`.prior-work.md` が在れば、最初に読む。
1. 12 件を特定する: visual 1/3 と 2/3 を、`--shots <DIR1>` と `--engine-logs <DIR1>` などを付けて、`--report-only` で走らせる(例: `node tools/e2e/run.mjs --area visual --shard 1/3 --budget 35m --displays 4k-dpr1 --shots <DIR1> --engine-logs <DIR1> --report-only`。`<DIR>` は `.tmp-*`)。失敗の行の ID を一覧にする。
2. 画像を見て分類する。本ごとに、(a) 製品の見た目の変化(画像で差が明らか。製品の欠陥か、意図した変更か)、(b) 道具の誤り(窓の矩形・判定の式・基準の取り方)、(c) 確認できない、のどれかにする。根拠は、画像のファイル名・区画の割合・判定の行。
3. (b) の道具の誤りを直す(`tools/e2e/lib`・`scenarios`)。直した後、同じ本を同じ bundle で走らせ、結果を確かめる。
4. 基準の書き換えは、(a) の本で、製品の変化が意図した変更だと確かめた時だけ。理由と区画の割合を報告に書き、該当の行(`<kind>.tsv` と `<kind>.cells.tsv` の両方)を消して、同じ DPR の実行で書き直す(README の規則)。画像を見ずに消さない。
5. (a) で製品の欠陥と分かった本は、印 `new` を付け、画像の場所(`.tmp-*` の名前)と区画の割合を報告に書く。製品の Issue は起票しない。
6. 受け入れを通す。通らない時は、落ちた ID・表示・出力の 1 行を報告に書く。

## 受け入れ

```accept
npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false&&r.elapsedMs!=null&&r.elapsedMs<=20000?0:1)' "$RUNNER_TEMP/boot.json" && (cd tools/e2e && npm ci && npx playwright install chromium) && node --test tools/e2e/test/*.test.mjs && node tools/e2e/run.mjs --area visual --shard 1/3 --budget 35m --displays 4k-dpr1 --report-only > "$RUNNER_TEMP/v13.jsonl" && node tools/e2e/run.mjs --area visual --shard 2/3 --budget 35m --displays 4k-dpr1 --report-only > "$RUNNER_TEMP/v23.jsonl" && node -e 'const fs=require("fs");let n=0,rows=0,bad=0;for(const p of process.argv.slice(1)){for(const r of fs.readFileSync(p,"utf8").trim().split("\n").map((l)=>JSON.parse(l))){rows++;if(r.error!==null)bad++;for(const s of r.scenarios){if(s.error!==null)bad++;if(s.expectFail===null&&s.ok!==true)n++}}}process.exit(bad===0&&rows>0&&n<=3?0:1)' "$RUNNER_TEMP/v13.jsonl" "$RUNNER_TEMP/v23.jsonl"
```

判定の基準:
- 受け入れの式が、終了コード 0 で終わる。
- visual 1/3 と 2/3 の run で、印の無い失敗(skipped を含む)が合計 3 本以下。row と scenario に error が無い。
- 基準(ゴールデン)の行を書き換えた本は、理由と区画の割合を報告に書いた本だけ。
- 判定の閾値(許容誤差 0.02・縁の式の値)を、この 12 本を通すために変えない。変える必要があるなら、理由と、ほかの本への影響を報告に書く。
- 報告に: (a) 12 件の一覧と分類(製品の変化・道具の誤り・確認できない)と、根拠(画像のファイル名・区画の割合・判定の行)、(b) 直した道具の要点(差分の範囲と理由)、(c) 基準を書き換えた本と理由、(d) 印 `new` を付けた本と理由、(e) 2 つの shard の出力の行(各 run の未印の失敗の ID・ok・notes)。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には、runner の値であることと、nproc と loadavg を添える。
- 「達成」「直った」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。出力の行を付ける。
- 確認できなかった物は「確認できない」と書く。
- 段ごとに、やること 0〜6 のどこまで進んだか(完了・途中・未着手)を書く。

## 制約

- 製品(`app/`・`lift/`)は変えない(読むだけ)。製品の欠陥と分かった場合は、報告に書く。`.github/`・`spec/` は、直す物がある時だけ変える。`tools/e2e/` は直してよい。
- 基準(ゴールデン)を、製品の出力に合わせるためだけに書き換えない。画像で、製品の変化が正しいと確かめた時だけ書き換える。
- 判定の閾値(許容誤差・縁の式)を、この 12 件を通すためだけに緩めない。
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

- #1528(visual 3/3 の描画の停止と、error の受理)とは、本の範囲が重ならない。ただし `tools/e2e/lib/plan.mjs` の受理の判定は #1528 が直すので、衝突したら、作業の最後に `git pull --no-rebase --no-edit` で main を取り込み、衝突を直し、受け入れをもう一度通す。
- #1507(windows・settings)が、窓の位置の共通の道具を直す可能性がある。衝突したら、同じ手順で取り込む。
