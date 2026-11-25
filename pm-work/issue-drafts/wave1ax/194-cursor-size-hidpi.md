---
draft: "194"
title: "マウスカーソルを、ゲストの画素 1 つ = 物理画素 1 つの大きさで出す: 高 DPR で、大きくならない"
type: Task
milestone: "M3 トップバーのコンテキストメニュー(P2)"
parent: "009"
blocked_by: []
labels: ["cursor", "fl-exe", "verify"]
size: S
priority: Urgent
project: signature-studio/3
---

## 目的

依頼者の実機で、マウスカーソルが FL の画面に比べて大きすぎる。依頼者の指摘(原文): 「マウスカーソルがデカすぎる、本当に HiDPI を考慮してるか?」。カーソルの画像の 1 画素を、物理画素 1 つで出す。DPR が 1 より大きい時も、カーソルを大きくしない。

この Issue の目標は次の 3 つ。
1. DPR が 1 超 2 未満の実機で、ゲストが設定したカーソルの画像の 1 画素が、物理画素 1 つになること(CSS の大きさ × DPR が、画像の幅・高さに等しい)。
2. DPR が変わった時(ウィンドウを別のモニターへ移す・ブラウザのズーム)に、カーソルを作り直すこと。
3. 画像が大きすぎてブラウザが無視する時の扱いを決め、コードの注に書くこと。

## 背景

- 現在の実装(`app/src/ui/cursor.ts`。コードの読み。実測ではない)。
  - DPR が 1 より大きい時、`pictured` は、画像の複製(1x・2x・3x。`picture.images`)を `image-set` で渡す。
  - 複製の宣言の倍率(`2x` など)は整数で、実行時の DPR(小数)と一致しない。ブラウザは DPR に近い複製を選ぶ。
  - カーソルの CSS の大きさは「画像の画素 ÷ 宣言の倍率」。複製の画素数と宣言の倍率は比例するので、どの複製でも CSS の大きさは元の画像の幅(画素)と同じになる。物理画素では「元の幅 × DPR」になる。DPR 1.5 なら、32 画素の絵が 48 物理画素で出る。
  - 直し方の見込み: 宣言の倍率を、実行時の `window.devicePixelRatio` にする(`image-set(url(…) <DPR>x)`。小数でよい)。すると CSS の大きさは「画像の幅 ÷ DPR」になり、物理画素では画像の幅に等しい。
  - 見込みは、Chromium の実測で確かめる。ホットスポットの座標の単位(画像の画素か CSS 画素か)も、同じ実測で確かめる。
- 前の #1392(完了)は、FL 自身のカーソル(資源の画像)を、ページ側の画像として出す形にした。この Issue は、その表示の大きさを直す。
- 大きさの上限: `CURSOR_IMAGE_MAX`(128)は、物理画素の上限として書かれている。Chromium がカーソルを無視する上限の単位(CSS 画素か物理画素か)を実測で確かめ、単位を合わせる。上限を超えるカーソルの扱いは、実測で決め、コードの注と報告に書く。

## 読む物

- `app/src/ui/cursor.ts`(`cursorCss`・`pictured`・`CURSOR_IMAGE_MAX`)
- `app/src/main.ts`(`cursorCss` の呼び出し(約 671 行)・DPR の監視(`dprQuery`・`matchMedia` の `resolution`))
- `app/src/ui/desktop.ts`(`setCursor` が `#desktop` の `style.cursor` を書く)
- `app/src/engine/cursor-picture.ts`・`app/src/ui/png.ts`(画像の作り。画素は資源の画素と同じ)
- `tools/cursor/measure.mjs`(カーソルの測定の道具。既存)
- `pm-work/issue-drafts/wave1w/144-cursor-fl-resources.md`(#1392 の本文)

## やること

0. 作業の最初に `date -u` を記録する。`.prior-work.md` が在れば、最初に読む。
1. 再現(Chromium の実測): DPR 1・1.5 付近・2・乱数(1.01 から 3.00)で、資源のカーソル(FL 自身のカーソル。Piano roll の鉛筆など)を出し、`#desktop` の計算済みの `cursor`(`getComputedStyle`)の `image-set` の宣言の倍率と、`tools/cursor/measure.mjs` の値から、画像の 1 画素が何物理画素で出るかを表にする。
2. 直す: 宣言の倍率を、実行時の `window.devicePixelRatio` にする。DPR が変わる時は、`matchMedia('(resolution: …dppx)')` の change で、カーソルを作り直す(既存の `dprQuery` の経路を使う)。ホットスポットを、画像の画素の単位で換算する。
3. 上限を超えるカーソルの扱いを、実測で決める(背景の最後の項目)。
4. E2E のシナリオ `cursor-size`(`tools/e2e/scenarios/smoke/` に新規。`index.mjs` の並びに足す): 各表示の行で、`#desktop` の計算済みの `cursor` から `image-set` の宣言の倍率 N を読み、`|N − window.devicePixelRatio| ≤ 0.01` を確かめる。判定の前提は、資源のカーソル(`image-set` になる物)がゲストから出ていること。出せない時は、その旨を JSON に出して失敗にし、報告に書く。カーソルの Base64 の文字列は、ログ・JSON・ページに出さない(長さと N だけ出す)。
5. 受け入れを通す。通らなければ、作業ツリーを commit して、報告を書いて終える。

## 受け入れ

```accept
npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false&&r.elapsedMs<=12500?0:1)' "$RUNNER_TEMP/boot.json" && node tools/smoke/welcome-flow.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/flow.jsonl" && node -e 'const fs=require("fs");const L=fs.readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const F=(f)=>L.find((r)=>r.flow===f);const ok=(r)=>!!r&&r.error===null&&r.state==="engine-idle"&&r.standIn===false&&r.trapped===false&&r.hostErrors===0&&r.opened===true;process.exit(L.length===4&&["A","B","C","D"].every((f)=>ok(F(f)))&&F("B").welcomeClosed===true&&F("D").welcomeClosed===true?0:1)' "$RUNNER_TEMP/flow.jsonl" && npm test --prefix app && (cd tools/e2e && npm ci && npx playwright install chromium) && node tools/e2e/run.mjs --scenarios smoke --displays 4k-dpr1,4k-dpr2,4k-random > "$RUNNER_TEMP/e2e.jsonl" && bash guard/scan.sh
```

判定の基準:
- 起動確認(`boot.json`)が engine-idle・standIn が false・elapsedMs が 12,500 以下。
- 流れ A〜D の 4 行が、`error` が null・engine-idle・`standIn` が false・`trapped` が false・`hostErrors` が 0・`opened` が true。B と D は `welcomeClosed` が true。
- `npm test --prefix app` が通る。
- `node tools/e2e/run.mjs --scenarios smoke --displays 4k-dpr1,4k-dpr2,4k-random` が終了コード 0(`cursor-size` を含む全シナリオが、全行で ok)。
- DPR の値を、コードや試験の定数として書いていないこと(報告に `grep` の結果を書く)。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には「runner の値。この機械とは別枠」と書き、nproc と loadavg を添える。
- 「達成」「直った」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。出力の行を付ける。
- 報告に書く: DPR ごとの表(画像の 1 画素が何物理画素で出たか)、直した箇所、ホットスポットの単位の実測、上限の扱い、流れ A〜D の 4 行。

## 制約

- 製品に、利用者向けの新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足さない。オプトインを作らない。
- DPR の値を、コードや試験の定数として書かない。実行時の `devicePixelRatio` から求める。
- 画像に補間(ぼかし)を入れない。DPR が整数倍でなくても、カーソルの描画はブラウザに任せる。
- Python を使わない(調査と検証にだけ使う)。OCR を使わない。
- 禁止されているのは、AI のベンダー名と Cloudflare の識別子だけ。FL の製品名・インストール先の名前・変数名は、書いてよい。
- sleep・timeout のコマンド・nohup・末尾の `&` は使わない。待つ時は条件で待つ。一時ファイルは `.tmp-*` に置き、終わったら消す。
- 製品の作業は Actions で行う。この機械で製品のビルドや測定をしない。

## 作業の進め方

- 作業の最初に `date -u` を記録する。4 時間 30 分を過ぎたら、受け入れが通らなくても、作業ツリーを commit し、報告を書いて終える(ジョブの上限は 5 時間 50 分)。
- ビルドは約 35 分。フォアグラウンドで直接、シェルツールの `timeout` 引数は最大 5400000(90 分)にする。背景で起動して待たない。
- 受け入れが通らなくても、作業の最後に作業ツリーを commit する。未 commit の変更が残ると、判定は失敗する。

## 他のタスクとの関係

- blocked_by: なし。
- `app/src/main.ts` と `app/src/ui/desktop.ts` は、#{{195}}(ページの HTML を無くす)と #{{196}}(クリップボード)も触る。衝突したら、作業の最後に `git pull --no-rebase --no-edit` で main を取り込み、衝突を直し、受け入れをもう一度通す。
- `tools/e2e/scenarios/smoke/index.mjs` に、シナリオを 1 つ足す。#{{195}}・#{{196}}・#{{197}}・#{{198}} も同じ並びに足すので、取り込みの時に並びを直す。
