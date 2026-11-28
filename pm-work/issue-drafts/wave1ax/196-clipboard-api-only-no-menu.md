---
draft: "196"
title: "『貼り付け』だけのコンテキストメニューを出さない。クリップボードは Clipboard API だけを使い、ページ読み込み時に権限を求める。権限が無い時のトーストは出さない"
type: Task
milestone: "M3 トップバーのコンテキストメニュー(P2)"
parent: "138"
blocked_by: ["195"]
labels: ["clipboard", "verify"]
size: M
priority: Urgent
project: signature-studio/3
---

## 目的

依頼者の指摘(2026-11-25。原文): 「出現条件は不明ですが、『貼り付け』のみのオプションのコンテキストメニューがたまに出現します。まず、このコンテキストメニューを出現させないでください。そもそも、Clipboard API は JS から読み書きできるため、それ以外のフォールバックは一切不要です。ページ読込み時に権限を求めてください。権限がない場合のトーストも不要です。」

この Issue の目標は次の 3 つ。
1. 「貼り付け」だけのブラウザのコンテキストメニューを出さない(`canvas`・見えない要素・iframe の中でも)。
2. クリップボードは、Clipboard API(`navigator.clipboard.readText`・`writeText`)だけで行う。`paste` イベントを使う経路と、他のフォールバックを無くす。
3. 権限を、ページ読み込み時に求める。権限が無い時のトーストを出さない(黙って空の答えを返す)。

## 背景

- FL の貼り付け(Ctrl+V・メニューの貼り付け)は、ゲストの `GetClipboardData` が、Clipboard API の読み取り(`navigator.clipboard.readText()`)で答える。
- 現在の実装(`app/src/engine/clipboard.ts`・#1389・#1406。コードの読み):
  - `paste` イベント(`doc.addEventListener('paste', …)`。対象は `#desktop`): Ctrl+V の貼り付けの文字を、次の `GetClipboardData` へ渡す(`pasted()`・`pending`)。
  - `probe()`(書式の問い合わせ)は、読み取りを起こさず、知っている状態で答える。この方式は、#1406 の決定で、保つ。
  - 権限の状態が `denied`・`prompt` の時、トーストを出す(`#clipboard-status`・`role=status`。5 秒、または閉じるボタン)。
  - `app/src/input/keymap.ts` は Ctrl+V を転送し、ブラウザの既定の動作を止めない(paste イベントを起こすため)。
- 出現条件が不明な「貼り付け」だけのメニュー: PM の確認(grep)では、`app/src/` と `app/index.html` に `textarea`・`contenteditable`・`<input>` の生成は無い(`app/index.html` の `#send-tab` のチェックボックス、`app/src/ui/filedlg.ts` と `app/src/files/explorer.ts` の `<input type=file>` のみ)。編集可能な要素の右クリックで、ブラウザが「貼り付け」だけを出していると推定する(推定。確かめる)。
- `app/src/input/pointer.ts` の `contextmenu` の抑止(`onContext`)は、入力が有効(`opts.enabled()`)の時だけ行う。無効な時と、ページの他の要素・iframe の中では、抑止されない。
- #1406(完了)は、右クリックの問い合わせで読まない、という決定を入れた。この Issue は、その決定を保ったまま、貼り付けの経路を Clipboard API だけに直す(依頼者の指示)。

## 読む物

- `app/src/engine/clipboard.ts`(全体)
- `app/src/main.ts`(#1389 の結線。189 行付近)
- `app/src/input/keymap.ts`(Ctrl+V の扱い)・`app/src/input/pointer.ts`(`onContext`・`enabled`)
- `app/src/engine/client.ts`・`app/src/engine/worker.ts`(`GetClipboardData` などの応答)
- `pm-work/findings/clipboard-1389.md`・`pm-work/findings/clipboard-1406.md`(#1389・#1406 の記録)
- `tools/clipboard/measure.mjs`(既存)
- `tools/e2e/scenarios/smoke/knob-right-click-permissions.mjs`(印 `expectFail: '#1406'` の付いたシナリオ)

## やること

0. 作業の最初に `date -u` を記録する。`.prior-work.md` が在れば、最初に読む。
1. 再現と原因: Chromium で、`canvas`・`#desktop`・iframe の中(Browser の面)・ツールバー・サイドバーで右クリックを行い、出た ブラウザのメニューの項目と、その時の要素(種類・編集の可否・フォーカス・iframe の中か)を記録する。「貼り付け」だけのメニューが出る条件を見つける。見つからなければ、試した条件を報告に書く。
2. 直す:
   - (a) `document` の capture 段で、`contextmenu` を、ページの全ての要素で `preventDefault()` する。iframe の中(Browser の面の `about:srcdoc`)では、iframe の window でも抑止する(ウィンドウの中の `contextmenu` は親に届かない)。FL 自身のコンテキストメニューは、ゲストが作るので、ブラウザの物は出さない。
   - (b) `paste` イベントの経路(`clipboard.ts` の `doc.addEventListener('paste')`・`pasted()`・`pending`)を無くす。Ctrl+V は、keymap で既定の動作を止め(paste イベントを起こさない)、ゲストへ転送する。ゲストの `GetClipboardData` が、`navigator.clipboard.readText()` で答える。
   - (c) 見えない入力要素と、フォールバック(`document.execCommand` など。在れば)を無くす。IME・テキスト入力に必要な入力要素が在れば、クリップボードには使わず、`contextmenu` を抑止し、見えない位置に置く。
   - (d) 権限: ページ読み込み時に `navigator.permissions.query({name:'clipboard-read'})` で状態を読む。`prompt` なら、読み込み時に `navigator.clipboard.readText()` を呼んで、読み取りの許可を求める。`clipboard-write` は、ブラウザがユーザーの操作の時に自動で許可するので、要求しない。読み込み時の要求がブラウザに拒否された時(`NotAllowedError`・ドキュメントにフォーカスが無い等)は、最初のクリック・キーの時に、同じ要求を自動でもう一度行う(ページに案内を出さない)。
   - (e) トーストを無くす: 権限の拒否・Copy の失敗・貼り付けの失敗の表示(`#clipboard-status`・`role=status`)を出さない。拒否された時は、黙って空の答え(ゲストへは「テキスト無し」)を返す。
   - (f) FL の「貼り付け」の有効・無効の問い合わせ(`IsClipboardFormatAvailable`・`CountClipboardFormats`): #1406 の方式(読まない。知っている状態から答える)を保つ。権限が `granted` の時は、読み込み時とフォーカス時に、テキストの有無を `readText()` で更新して持つ。右クリックの時には読まない。
3. E2E のシナリオ(`tools/e2e/scenarios/smoke/` に新規。`index.mjs` の並びに足す):
   - `context-menu-none`: 各表示で、`canvas`・`#desktop`・サイドバー・ツールバーの中の、ランダムな位置で右クリックを 20 回行う。ページ側の計測(`contextmenu` の capture のリスナー)で、`defaultPrevented` が 20 回とも真。
   - `clipboard-load-permission`: 権限 `prompt` の状態で(`Browser.setPermission` を使い、`context.grantPermissions` は使わない)ページを読み込み、最初のクリックの後に、`clipboard-read` の要求が 1 回以上行われる。要求の回数は、`navigator.clipboard.readText` を計測用に包んで数える。
   - `clipboard-no-toast`: 権限 `denied` で、貼り付け・コピーの操作を行い、`role=status`・`role=alert`・`aria-live` の、見える要素が 0。
   - 既存の `knob-right-click-permissions`(印 `expectFail: '#1406'`)は、印を外す(#1406 は完了。この Issue の期待で見る)。
4. 受け入れを通す。通らなければ、作業ツリーを commit して、報告を書いて終える。

## 受け入れ

```accept
npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false&&r.elapsedMs<=12500?0:1)' "$RUNNER_TEMP/boot.json" && node tools/smoke/welcome-flow.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/flow.jsonl" && node -e 'const fs=require("fs");const L=fs.readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const F=(f)=>L.find((r)=>r.flow===f);const ok=(r)=>!!r&&r.error===null&&r.state==="engine-idle"&&r.standIn===false&&r.trapped===false&&r.hostErrors===0&&r.opened===true;process.exit(L.length===4&&["A","B","C","D"].every((f)=>ok(F(f)))&&F("B").welcomeClosed===true&&F("D").welcomeClosed===true?0:1)' "$RUNNER_TEMP/flow.jsonl" && npm test --prefix app && (cd tools/e2e && npm ci && npx playwright install chromium) && node tools/e2e/run.mjs --scenarios smoke --displays 4k-dpr1,4k-dpr2,4k-random > "$RUNNER_TEMP/e2e.jsonl" && bash guard/scan.sh
```

判定の基準:
- 起動確認(`boot.json`)が engine-idle・standIn が false・elapsedMs が 12,500 以下。
- 流れ A〜D の 4 行が、`error` が null・engine-idle・`standIn` が false・`trapped` が false・`hostErrors` が 0・`opened` が true。B と D は `welcomeClosed` が true。
- `npm test --prefix app` が通る。
- `node tools/e2e/run.mjs --scenarios smoke --displays 4k-dpr1,4k-dpr2,4k-random` が終了コード 0。`context-menu-none`・`clipboard-load-permission`・`clipboard-no-toast`・`knob-right-click-permissions`(印を外した状態)が、全行で ok。
- `paste` イベントの経路が `app/src/` に残っていない(報告に `grep` の結果を書く)。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には「runner の値。この機械とは別枠」と書き、nproc と loadavg を添える。
- 「達成」「直った」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。出力の行を付ける。
- 報告に書く: 右クリックの表(要素ごとに、出た項目)、「貼り付け」だけのメニューの出現条件(見つかった物。見つからなければ、試した条件)、直した箇所、権限の要求のタイミングの実測、流れ A〜D の 4 行。

## 制約

- 製品に、利用者向けの新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足さない。オプトインを作らない。
- Clipboard API 以外の、クリップボードのためのフォールバックを作らない。
- 権限の要求は、ページ読み込み時(と、その後の最初の操作)に限る。右クリックの時に読み取り・要求を起こさない(#1406 の決定)。
- Python を使わない(調査と検証にだけ使う)。OCR を使わない。
- 禁止されているのは、AI のベンダー名と Cloudflare の識別子だけ。FL の製品名・インストール先の名前・変数名は、書いてよい。
- sleep・timeout のコマンド・nohup・末尾の `&` は使わない。待つ時は条件で待つ。一時ファイルは `.tmp-*` に置き、終わったら消す。
- 製品の作業は Actions で行う。この機械で製品のビルドや測定をしない。

## 作業の進め方

- 作業の最初に `date -u` を記録する。4 時間 30 分を過ぎたら、受け入れが通らなくても、作業ツリーを commit し、報告を書いて終える(ジョブの上限は 5 時間 50 分)。
- ビルドは約 35 分。フォアグラウンドで直接、シェルツールの `timeout` 引数は最大 5400000(90 分)にする。背景で起動して待たない。
- 受け入れが通らなくても、作業の最後に作業ツリーを commit する。未 commit の変更が残ると、判定は失敗する。

## 他のタスクとの関係

- blocked_by: #{{195}}(ページの HTML を無くす)。`app/index.html` と `app/src/main.ts` の表示の変更を取り込んでから作業する。
- #{{194}}(カーソルの大きさ)は、`app/src/main.ts` を触る。衝突したら、作業の最後に `git pull --no-rebase --no-edit` で main を取り込み、衝突を直し、受け入れをもう一度通す。
- `tools/e2e/scenarios/smoke/index.mjs` に 3 シナリオを足す。#{{194}}・#{{197}}・#{{198}} も同じ並びに足すので、取り込みの時に並びを直す。
