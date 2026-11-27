---
draft: "234"
title: "起動の順序を直す: 黒い canvas に HTML のモーダルでダウンロードの進捗を出し、起動の直前に消してから、FL のネイティブのスプラッシュを出す(スプラッシュは FL の表示のコードを lift して出す)"
type: "Task"
milestone: "M4 FL に必要な物の配布"
parent: "012"
blocked_by: []
labels: ["delivery", "verify"]
size: "XL"
priority: "High"
project: "signature-studio/3"
---

## 目的

依頼者の指示(2026-11-27。原文): 「ネイティブのローディングスプラッシュを維持してください。順序としては、1. 黒背景（何も動いていない状態のCanvas）に、画面中央にHTML製モーダルで起動に必要なリソースのダウンロード状況を進捗バーで表示、ダウンロード完了し起動開始直前にモーダルを削除 2. ネイティブのスプラッシュを表示 - ネイティブスプラッシュは、決してネイティブスプラッシュで使用している画像を適当に動かすだけではいけません。ネイティブスプラッシュを表示しているコードをそのままliftする必要があります。」

起動の順序を、次の 3 段にする。
1. ページの読み込みの直後、黒い背景(何も動いていない canvas)の上に、画面の中央の HTML のモーダルを出し、起動に必要なリソースのダウンロード状況を進捗バー(`role="progressbar"`)で示す。
2. ダウンロードが終わり、起動を始める直前に、モーダルを DOM から消す。
3. その後、FL のネイティブのスプラッシュを出す。スプラッシュは、FL がスプラッシュを表示するコード(窓の生成と描画)を lift した C/wasm のまま走らせ、その描画の呼び出しが canvas に出る形にする。

**スプラッシュの画像を、ページが読んで動かす・描くことは禁止**。依頼者が明示的に禁止した。

## 背景

- 読み(`pm-work/findings/fb-2026-11-27.md` の 2 節): 今の `app/index.html` は `<header hidden>` と `<aside hidden>` で、起動の段(`#phases`)と進捗(`#progress`)を隠している(#1481 `c409b2db`)。進捗の行は `app/src/main.ts` の `progress.update` が作るが、画面には出ない。今、ダウンロードの間に見えるのは、黒い画面だけ。
- canvas の既定の背景は黒(`app/src/style.css` の `#desktop canvas`)。
- 起動前に取る物は 4 項目(`app/src/main.ts` の `expected`: engine wasm・seed pack・DLL pack・engine glue)。`manifest.lazy` は起動の後に取る。大きさの記録は `pm-work/findings/c5-ott-1337.md:75`(合計 311,109,738 B、br で 64,102,815 B。その時点の bundle)。
- ネイティブのスプラッシュ: リポジトリに、スプラッシュの窓の名前・描画の関数・記録が無い(`git grep -il splash` は seed の画像名と入力の表だけ)。今のエンジンでスプラッシュの窓が出るかは確認できない。起動の最初の idle で記録されている窓は `TWelcomeWizard`。`app/src` に、スプラッシュの画像を読む・描く・動かす箇所は無い。

## 読む物

- `pm-work/findings/fb-2026-11-27.md`(2 節)・`pm-work/findings/first-paint-1483.md`(黒い領域の記録)
- `app/index.html`・`app/src/style.css`・`app/src/main.ts`(`boot` の段 853-895 行付近。`progress`・`renderPhases`・`client.init`・`client.start`・`startLazy`)
- `app/src/ui/progress.ts`・`app/src/ui/phases.ts`・`app/src/bundle/loader.ts`(進捗の値 `Progress`)・`app/src/bundle/manifest.ts`
- `lift/harness/fruity.c`・`lift/harness/web/fruity/`・`lift/harness/web/pack.args`(起動の引数)・`lift/runtime/fle_win.c`・`lift/runtime/fle_raster.c`(窓の描画の経路)
- FL の起動の段のコード(スプラッシュを作る・消す箇所)。読んだ範囲と、読めなかった範囲を報告に書く。

## やること

0. 作業の最初に `date -u` を記録する。`.prior-work.md` が在れば最初に読む。`git status` で作業ツリーを確かめる。
1. 赤を先に確かめる。E2E のシナリオ `tools/e2e/scenarios/page/startup-sequence.mjs` を書く(下の判定)。今の本番で、モーダルが無いので赤になることを確かめる。
2. HTML のモーダルを足す(`app/index.html` と `app/src/ui/`)。中央に置き、ダウンロードの 4 項目の名前・大きさ・進捗を、`<progress role="progressbar">` で出す(既存の `ProgressView` の形を使う)。モーダルは、ダウンロードの開始の時に出し、`client.start()` の直前に DOM から取り除く(`hidden` ではなく削除)。
3. 起動の直後の描画: canvas は黒のまま。窓は出さない。
4. ネイティブのスプラッシュ: FL がスプラッシュを出すコードを探し、lift したものをそのまま走らせる。もし抑止されているなら、抑止を外す(harness の段。`lift/harness/fruity.c` などを読む)。製品に、新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足さない。スプラッシュが今、出ているなら、消さない。
5. 画像を読んで動かす・描く実装は入れない。`app/` にスプラッシュの画像の参照を足さない(`git grep` で 0 件を確かめる)。
6. 確認できない所(FL のスプラッシュのコードの場所、今の出力の有無)は、無理に作らず、報告に「確認できない」と書く。その時は、結果を「差し戻し」にする。
7. 試験: `app/` の単体試験で、モーダルの進捗の値(0〜1)と、削除の後に残らないことを確かめる。
8. 報告する(下の「報告の規則」)。

## E2E の判定(`tools/e2e/scenarios/page/startup-sequence.mjs`)

ページの読み込みを開始し、短い間隔で画面と DOM を読む。DPR 1・DPR 2・乱数の表示で動く。
- (a) 最初の描画で、canvas が黒(画素の大半が 0)、HTML のモーダルが `role="progressbar"` を持ち、画面の中央にある(窓の中央からの距離が、窓の幅の 10% 以下)。
- (b) 進捗の値が増え、4 項目が全て 100% になる。
- (c) モーダルが DOM から消えた時刻が、エンジンの起動(`window.__app.timeline` の `engine_start` 等の計時)より前で、エンジンの最初の描画より前。
- (d) エンジンの窓の表に、スプラッシュの窓が `first idle` より前に出る。その窓の class 名を報告に書く。
- (e) スプラッシュの描画が、エンジンの描画の呼び出しから来ている証拠を、エンジンのログの行で示す(窓への描画の行。ページの `<img>`・`background-image`・data URL の画像が、スプラッシュを出していないことを、DOM の検査で確かめる)。
- 判定の notes に、各段の時刻と、窓の class と、ログの行を書く。

## 受け入れ

```accept
npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false&&r.elapsedMs<=20000?0:1)' "$RUNNER_TEMP/boot.json" && node tools/smoke/welcome-flow.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/flow.jsonl" && node -e 'const fs=require("fs");const L=fs.readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const F=(f)=>L.find((r)=>r.flow===f);const ok=(r)=>!!r&&r.error===null&&r.state==="engine-idle"&&r.standIn===false&&r.trapped===false&&r.hostErrors===0&&r.opened===true;process.exit(L.length===4&&["A","B","C","D"].every((f)=>ok(F(f)))&&F("B").welcomeClosed===true&&F("D").welcomeClosed===true?0:1)' "$RUNNER_TEMP/flow.jsonl" && npm test --prefix app && (cd tools/e2e && npm ci && npx playwright install chromium) && node tools/e2e/run.mjs --only page/startup-sequence --displays 4k-dpr1 --budget 30m > "$RUNNER_TEMP/e2e-1.jsonl" && node tools/e2e/run.mjs --only page/startup-sequence --displays 4k-dpr2 --budget 30m > "$RUNNER_TEMP/e2e-2.jsonl" && node tools/e2e/run.mjs --only page/startup-sequence --displays 4k-random --budget 30m > "$RUNNER_TEMP/e2e-3.jsonl" && node -e 'const fs=require("fs");const L=process.argv.slice(1).flatMap((f)=>fs.readFileSync(f,"utf8").trim().split("\n")).map((l)=>JSON.parse(l));process.exit(L.length===3&&L.every((r)=>r.error===null&&r.scenarios.length>0&&r.scenarios.every((s)=>s.ok===true&&s.skipped!==true))?0:1)' "$RUNNER_TEMP/e2e-1.jsonl" "$RUNNER_TEMP/e2e-2.jsonl" "$RUNNER_TEMP/e2e-3.jsonl" && bash guard/scan.sh
```

判定の基準:
- 起動確認(`boot.json`)が engine-idle・standIn が false・elapsedMs が 20,000 以下。
- 流れ A〜D の 4 行が、`error` が null・engine-idle・`standIn` が false・`trapped` が false・`hostErrors` が 0・`opened` が true。B と D は `welcomeClosed` が true。
- `npm test --prefix app` が通る(手順 7 の試験を含む)。
- E2E の 3 行(DPR 1・DPR 2・乱数)が全て `ok`。判定の (a)〜(e) の全て。
- `bash guard/scan.sh` の指摘が 0 件。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には「runner の値。この機械とは別枠」と書く。
- 報告に書く: (a) 直す前の `page/startup-sequence` の赤の出力の行、(b) モーダルの位置・大きさ・進捗の値の推移、(c) モーダルを消した時刻と `engine_start` の時刻、(d) スプラッシュの窓の class 名と、エンジンのログの描画の行、(e) FL のスプラッシュのコードの場所と、抑止の有無(読んだ範囲)、(f) 流れ A〜D の 4 行、(g) `bash guard/scan.sh` の結果、(h) 確認できなかった点。
- 「達成」「直った」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。出力の行を付ける。

## 制約

- 製品に、利用者向けの新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足さない。
- スプラッシュの画像を、ページが読んで動かす・描くことは、依頼者が禁止した。`app/` に、スプラッシュの画像の参照を足さない。
- Python を使わない(調査と検証にだけ使う)。OCR を使わない。
- 禁止されているのは、AI のベンダー名と Cloudflare の識別子だけ。FL の製品名・インストール先の名前・変数名は、書いてよい。
- sleep・timeout のコマンド・nohup・末尾の `&` は使わない。一時ファイルは `.tmp-*` に置き、終わったら消す。

## 作業の進め方

- 作業の最初に `date -u` を記録する。4 時間 30 分を過ぎたら、受け入れが通らなくても、作業ツリーを commit し、報告を書いて終える(ジョブの上限は 5 時間 50 分)。
- ビルドは 1 回にして、同じ bundle で繰り返し確かめて直す。長いコマンドはフォアグラウンドで直接、シェルツールの `timeout` 引数は最大 5400000(90 分)にする。背景で起動して待たない。
- 受け入れが通らなくても、作業の最後に作業ツリーを commit する。未 commit の変更が残ると、判定は失敗する。

## 他のタスクとの関係

- blocked_by: なし。
- #{{233}}(File→Open の窓)とは独立。
