---
draft: "197"
title: "ページ読み込み時の描画が、ランダムに不完全になる(画面の左側が真っ黒のまま残る)"
type: Task
milestone: "M2 HiDPI と P2 の測定(P2)"
parent: "005"
blocked_by: []
labels: ["render", "perf", "verify"]
size: M
priority: Urgent
project: signature-studio/3
---

## 目的

依頼者の指摘(2026-11-25。画像つき): ページを読み込むと、FL の画面の描画が不完全になることがある。ゲストの画面(3240×1863 物理画素)の、左側の広い領域(Browser の面。WebView2 を載せる領域)と、左上の帯の周りが、黒いまま残る。Welcome の窓が、描かれていない領域の上に出る。出る回と出ない回がある(ランダム)。

この Issue の目標は次の 3 つ。
1. 起動の後(first idle の後 3 秒)、FL の画面に、完全な黒の大きな領域が無いこと(DPR 1 と、1.01 から 3.00 の乱数で、5 回続けて)。
2. 原因を特定し、既定の動作として直すこと。
3. 再現の測定と、E2E のシナリオ `first-paint-complete` を入れること。

## 背景

- 依頼者の画像の読み(PM の確認): FL のデスクトップのうち、左側の広い領域(x が 0 から約 333 CSS 画素、y が約 165 から 1004 の付近)が黒い。左上の帯(「(Trial) Failed downloading news. Error 4.」の周り)も黒い。Welcome の窓は、描かれていない領域の上に出ている。
- 依頼者の画面の DPR は 1 超 2 未満。
- 疑う点(推定。確かめる):
  - (a) #1351(描画の所要を、変化した面積に比例させる)の、描画の差分(矩形)の管理が、初回の全面の描画を取りこぼす。初回は全面を転送する必要がある(`app/src/engine/desk-gl.ts`)。
  - (b) Browser の面は、WebView2 の代わりに、ページの `iframe`(#1362 の面)を重ねている。iframe が、読み込みの前に黒で重なる。
  - (c) 描画のタイミング(最初の idle の前に、Welcome の窓が先に出る)・WebGL の `canvas` の初回のテクスチャの転送の失敗(大きいキャンバス)。
- 関連する記録: `pm-work/findings/p2-render-1351.md`(#1351 の測定の記録)・`pm-work/findings/browser-panel-1355.md`(Browser の面の記録)。

## 読む物

- `app/src/engine/desk-gl.ts`(全体。描画の矩形・初回の転送)
- `app/src/ui/desktop.ts`・`app/src/ui/webview2-surface.ts`(Browser の面の iframe)
- `app/src/engine/` の描画の経路(`grep -rn 'dirty\|damage\|rect' app/src/engine` などで探す)
- `pm-work/findings/p2-render-1351.md`・`pm-work/findings/browser-panel-1355.md`
- `tools/e2e/lib/browser.mjs`(スクリーンショット・計測)・`tools/e2e/lib/detect.mjs`
- `tools/browser-panel/`(Browser の面の道具)

## やること

0. 作業の最初に `date -u` を記録する。`.prior-work.md` が在れば、最初に読む。
1. 再現: E2E の道具で、起動して first idle の後 3 秒の、FL の画面(`canvas`。`canvas.screenshot()`・または WebGL の `readPixels`)を読み、完全な黒(`rgb(0,0,0)`)の画素の連結成分で、幅・高さが 100 画素以上のものの数を数える。FL のデスクトップの背景は暗い青灰色で、完全な黒の広い領域は無いと推定する(最初の実測で確かめる)。DPR 1・1.01 から 3.00 の乱数(2 点)・DPR 2 で、各 5 回起動し、黒い領域が出た回の率を表にする(起動の時刻・DPR・黒い領域の矩形)。
2. 原因の特定: 上の (a)(b)(c) のどれかを、trace(描画の矩形の数・iframe の読み込み・初回の転送の時刻)で確かめる。実測と推定を分けて報告に書く。
3. 直す: 既定の動作として直す(オプトインを作らない)。
   - 初回は、画面の全体を描画する(矩形の管理が、初回の全面を取りこぼさないこと)。
   - iframe が黒で重なるなら、読み込みの前に黒い背景を出さない。背景は、FL のデスクトップの描画と同じ色にする。
4. 道具: `tools/e2e/run.mjs` に `--repeat N` を足す(同じシナリオを N 回、起動し直して繰り返す。全部が ok でなければ、終了コード 1)。
5. E2E のシナリオ `first-paint-complete`(`tools/e2e/scenarios/smoke/` に新規。`index.mjs` の並びに足す): 起動して first idle の後 3 秒で、完全な黒の連結成分(幅・高さ 100 画素以上)が 0 個。各表示の行で判定する。
6. 受け入れを通す。通らなければ、作業ツリーを commit して、報告を書いて終える。

## 受け入れ

```accept
npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false&&r.elapsedMs<=12500?0:1)' "$RUNNER_TEMP/boot.json" && node tools/smoke/welcome-flow.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/flow.jsonl" && node -e 'const fs=require("fs");const L=fs.readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const F=(f)=>L.find((r)=>r.flow===f);const ok=(r)=>!!r&&r.error===null&&r.state==="engine-idle"&&r.standIn===false&&r.trapped===false&&r.hostErrors===0&&r.opened===true;process.exit(L.length===4&&["A","B","C","D"].every((f)=>ok(F(f)))&&F("B").welcomeClosed===true&&F("D").welcomeClosed===true?0:1)' "$RUNNER_TEMP/flow.jsonl" && npm test --prefix app && (cd tools/e2e && npm ci && npx playwright install chromium) && node tools/e2e/run.mjs --scenarios smoke --displays 4k-dpr1,4k-dpr2,4k-random > "$RUNNER_TEMP/e2e.jsonl" && node tools/e2e/run.mjs --only first-paint-complete --displays 4k-dpr1,4k-dpr2,4k-random --repeat 5 > "$RUNNER_TEMP/fp.jsonl" && bash guard/scan.sh
```

判定の基準:
- 起動確認(`boot.json`)が engine-idle・standIn が false・elapsedMs が 12,500 以下。
- 流れ A〜D の 4 行が、`error` が null・engine-idle・`standIn` が false・`trapped` が false・`hostErrors` が 0・`opened` が true。B と D は `welcomeClosed` が true。
- `npm test --prefix app` が通る。
- `node tools/e2e/run.mjs --scenarios smoke --displays 4k-dpr1,4k-dpr2,4k-random` が終了コード 0。
- `first-paint-complete` が、3 表示の各々で 5 回とも ok(`fp.jsonl` の終了コード 0 で確かめる)。
- 報告に、黒い領域が出た率の表(修正の前後)を書く。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には「runner の値。この機械とは別枠」と書き、nproc と loadavg を添える。
- 「達成」「直った」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。出力の行を付ける。
- 報告に書く: 再現の表(DPR・起動の時刻・黒い領域の矩形)、原因(実測と推定を分ける)、直した箇所、修正の前後の率、流れ A〜D の 4 行。

## 制約

- 製品に、利用者向けの新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足さない。オプトインを作らない。テストのための分岐を製品に入れない。
- DPR の値を、コードや試験の定数として書かない。実行時の `devicePixelRatio` から求める。
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
- `app/src/engine/desk-gl.ts` は、#1351 の描画の経路と同じ物。#1351 の変更を取り込んでから直す。
- #{{194}}(カーソル)・#{{195}}(ページの HTML)・#{{196}}(クリップボード)は、`app/src/main.ts` と `tools/e2e/scenarios/smoke/index.mjs` を触る。衝突したら、作業の最後に `git pull --no-rebase --no-edit` で main を取り込み、衝突を直し、受け入れをもう一度通す。
