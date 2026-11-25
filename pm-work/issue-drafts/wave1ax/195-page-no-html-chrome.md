---
draft: "195"
title: "ページから、HTML のヘッダー・サイドバー・状態の表示を無くし、FL の画面だけを、ウィンドウいっぱいに出す"
type: Task
milestone: "M2 HiDPI と P2 の測定(P2)"
parent: "005"
blocked_by: []
labels: ["perf-beacons", "verify"]
size: M
priority: Urgent
project: signature-studio/3
---

## 目的

依頼者の指摘(2026-11-25。原文): 「マウスカーソルの Base64 を全て表示しないでください。というか、私はあなたにテレメトリに関する一切の情報をあなたに連絡しません。そのため、ページには HTML で構成されたヘッダーもサイドバーも不要です。」

ページは、FL の画面(`canvas`)だけを、ウィンドウいっぱいに出す。ヘッダー・サイドバー・状態の表示(HTML)は、見える形では出さない。

この Issue の目標は次の 3 つ。
1. 見える HTML が、FL の画面(`canvas`)以外に無いこと。
2. 必要な機能(ファイルの選択・音声の開始・最小化した窓の復元・サイズ変更への追従・テレメトリの送信)が、画面の HTML 無しで動くこと。
3. テスト・測定の道具が、見えない状態の API で読め、道具が壊れないこと。

## 背景

- 依頼者の画面の読み(PM の確認): ページの上端に HTML の帯(`FL Studio web private build, F2`・エンジン名・`音: 動作中 44.1 kHz・出力遅延 32 ms・直近 60 秒の途切れ 21` の表示・`Files` ボタン)。右端に HTML のサイドバー(`DISPLAY AND ENGINE`・`send Tab to the engine`・`End session`・`Engine: computing…`・入力の件数・`pointer: … cursor: image-set(url("data:image/png;base64,…` の長い Base64)。依頼者の実機で、アイドルの 60 秒に途切れ 21 件と出ていた。
- `app/index.html` に在る HTML(コードの読み): `#display-box`(`Monitor` の切り替え)・`#bundle-info`・`#audio-status`(`role=status`)・`#status`(`role=status`)・`#engine-line`・`#pointer-info`・`#cursor-info`・`#toolbar`(`send-tab` のチェックボックス・`#canvas-info`・`#stop-btn`)・`#resize-note`(`role=status`。再読み込みの案内)・`#restart-note`・`#taskbar`(`role=status`)・`#check`・`#phases`・`#progress`・`#saved-files`・`#windows-panel`・`#threads-panel`・`#engine-log-panel`・`#perf-panel`・`#log-panel`。`#error`(`role=alert`)・`#exit`。`#filedlg`(FL のファイルの選択の窓)。
- `Files` ボタンは `app/src/files/mount.ts` が作り、`app/src/files/explorer.ts` がページの Files の管理の窓を出す。
- テレメトリの送信(ビーコン。`app/src/perf/beacon.ts`・`app/src/perf/failure-beacon.ts`)は、画面に出さず、そのまま送る(サーバー側の記録)。
- 既存の道具が読む見えない状態: `window.__app`(`app/src/main.ts`)と `role=status` の要素。`tools/e2e/lib/`・`tools/p3-audio-probe/`・`tools/boot-check/`・`tools/smoke/welcome-flow.mjs` などが読む。どの道具がどの要素を読むかは、`grep` で確かめる。
- #1376・#1419 は、Browser の面の iframe を扱う。この Issue は、Browser の面の表示(FL の窓との前後関係)を変えない。

## 読む物

- `app/index.html`・`app/src/main.ts`(全体)・`app/src/ui/taskbar.ts`・`app/src/ui/engine-status.ts`・`app/src/ui/stop-message.ts`・`app/src/ui/screen-fit.ts`(サイズの変更)・`app/src/ui/filedlg.ts`・`app/src/engine/filedlg.ts`
- `app/src/files/mount.ts`・`app/src/files/explorer.ts`
- `app/src/perf/beacon.ts`・`app/src/perf/failure-beacon.ts`(テレメトリの送信)
- `tools/e2e/lib/detect.mjs`・`tools/e2e/lib/guest.mjs`・`tools/p3-audio-probe/measure.mjs`・`tools/boot-check/boot-bundle.mjs`・`tools/smoke/welcome-flow.mjs`(見えない状態の読み方)
- #1389(ファイルの選択)・#1348・#1354(サイズの変更)の本文(下書きの番号 138・108・114。`pm-work/issue-drafts/` の中で、`draft: "138"` などの行を探す)

## やること

0. 作業の最初に `date -u` を記録する。`.prior-work.md` が在れば、最初に読む。
1. 洗い出し: `app/index.html`・`app/src/main.ts`・`app/src/ui/`・`app/src/files/`・`app/src/perf/` の、HTML の表示を作る箇所を表にする(要素・作る箇所・読む道具)。表は報告に書く。
2. 見えない化: 全ての HTML の要素を、`hidden`(`display: none`)にするか、作らないにする。ページには `canvas`(FL の画面)だけが、ウィンドウいっぱいに出る。
   - 道具が読む要素は、`hidden` の DOM のまま残すか、`window.__app` の読み出しに移す。道具の読み方を直す時は、`tools/` の変更も、この Issue の範囲。
3. 機能の移し替え(画面に出さずに残す):
   - (a) ファイルの選択: `Files` ボタンを無くす。FL の Open ダイアログ(ゲストの `GetOpenFileName` など)が、ブラウザのファイル選択(`<input type=file>`・`showOpenFilePicker`)を開く。`app/src/ui/filedlg.ts` と `app/src/engine/filedlg.ts` の経路を使う(#1389 の経路)。フォルダの取り込み(OPFS へ)と複数ファイルも、FL のダイアログから行う。無くなる機能は、報告に一覧で書く。`Files` の管理の窓を、FL の中に作らない。
   - (b) 音声の開始: ブラウザの規則で、ユーザーの操作の後に AudioContext を `resume` する。画面に出さず、最初のクリック・キーで `resume` する。
   - (c) 最小化した窓の復元(`#taskbar`): FL の中で完結しているか確かめる。ページの帯が要るなら、最小化した窓がある時だけ、小さく出す(常には出さない)。
   - (d) サイズの変更(`#resize-note`): 再読み込みの案内を出さない。表示サイズが変わった時は、自動で反映する(#1348・#1354 の経路)。反映できない時は、無視する。
   - (e) テレメトリの送信(ビーコン)は、そのまま。
   - (f) 停止の表示(`#error`・`#exit`): engine が止まった時だけ、1 つの欄に出す。止まっていない時は見えない。
4. E2E のシナリオ `page-chrome`(`tools/e2e/scenarios/smoke/` に新規): 起動して first idle の後に、次を確かめる。(i) `canvas` が、ビューポートの 100% を占める(幅・高さの差が 1 CSS 画素以下)。(ii) `document.body` の中で、`canvas` 以外の見える要素(`getBoundingClientRect` が 0 でなく、`display`・`visibility` が可視)が 0 個(`script`・`style` を除く)。(iii) ページの本文に `data:image` の文字列が無い。各表示の行で判定する。
5. 受け入れを通す。通らなければ、作業ツリーを commit して、報告を書いて終える。
6. 1 回の試行で終わらない時: 段階 1 = 見える物を隠す(`display: none`)・道具が動く、段階 2 = 不要な DOM と処理の削除、に分けてよい(報告に書く)。

## 受け入れ

```accept
npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false&&r.elapsedMs<=12500?0:1)' "$RUNNER_TEMP/boot.json" && node tools/smoke/welcome-flow.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/flow.jsonl" && node -e 'const fs=require("fs");const L=fs.readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const F=(f)=>L.find((r)=>r.flow===f);const ok=(r)=>!!r&&r.error===null&&r.state==="engine-idle"&&r.standIn===false&&r.trapped===false&&r.hostErrors===0&&r.opened===true;process.exit(L.length===4&&["A","B","C","D"].every((f)=>ok(F(f)))&&F("B").welcomeClosed===true&&F("D").welcomeClosed===true?0:1)' "$RUNNER_TEMP/flow.jsonl" && npm test --prefix app && (cd tools/e2e && npm ci && npx playwright install chromium) && node tools/e2e/run.mjs --scenarios smoke --displays 4k-dpr1,4k-dpr2,4k-random > "$RUNNER_TEMP/e2e.jsonl" && node tools/p3-audio-probe/measure.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" --boot > "$RUNNER_TEMP/p3.json" && bash guard/scan.sh
```

判定の基準:
- 起動確認(`boot.json`)が engine-idle・standIn が false・elapsedMs が 12,500 以下。
- 流れ A〜D の 4 行が、`error` が null・engine-idle・`standIn` が false・`trapped` が false・`hostErrors` が 0・`opened` が true。B と D は `welcomeClosed` が true。
- `npm test --prefix app` が通る。
- `node tools/e2e/run.mjs --scenarios smoke --displays 4k-dpr1,4k-dpr2,4k-random` が終了コード 0(`page-chrome` を含む全シナリオが、全行で ok)。
- `tools/p3-audio-probe/measure.mjs --boot` が終了コード 0(`error` が無い)。`tools/boot-check/boot-bundle.mjs` が壊れない。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には「runner の値。この機械とは別枠」と書き、nproc と loadavg を添える。
- 「達成」「直った」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。出力の行を付ける。
- 報告に書く: 表(洗い出し)、無くした HTML の一覧、移し替えた機能(a)〜(f)の方法、無くなった機能の一覧、道具の読み方の変更、流れ A〜D の 4 行。

## 制約

- 製品に、利用者向けの新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足さない。オプトインを作らない。
- マウスカーソルの Base64 を、ページ・ログ・JSON・報告に出さない。長さと種類だけを出す。
- テレメトリの送信(ビーコン)を止めない。画面に出さないだけにする。
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
- #{{194}}(カーソルの大きさ)は、`app/src/main.ts` の `#cursor-info`・`#pointer-info` の表示を読む。この Issue で、その表示を見えなくするので、衝突したら、作業の最後に `git pull --no-rebase --no-edit` で main を取り込み、衝突を直し、受け入れをもう一度通す。
- #{{196}}(クリップボード)は、`app/src/main.ts` と `app/index.html` を触る(`#clipboard-status` の表示)。#{{196}} は、この Issue が main に入った後に作業するので、衝突は #{{196}} の側で直す。この Issue では、`#clipboard-status` を、表示の対象の一覧に入れる。
- `tools/e2e/scenarios/smoke/index.mjs` に `page-chrome` を足す。#{{194}}・#{{196}}・#{{197}}・#{{198}} も同じ並びに足すので、取り込みの時に並びを直す。
