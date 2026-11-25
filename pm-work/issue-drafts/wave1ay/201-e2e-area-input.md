---
draft: "201"
title: "E2E の領域 input: マウス・キーボード・タッチ・カーソル・クリップボード・コンテキストメニュー・ツールチップの、期待する結果のシナリオを機能ごとに足す"
type: Task
milestone: "M2 HiDPI と P2 の測定(P2)"
parent: "133"
blocked_by: ["199"]
labels: ["verify", "render"]
size: XL
priority: Urgent
project: signature-studio/3
---

## 目的

依頼者の指示(2026-11-26。原文): 「そうではなく、存在する機能について、考えうる限り全ての E2E テストを追加してください。」

この領域(input)は、入力の経路の機能を、期待する結果つきのシナリオで確かめる。最小のシナリオ数は 120 本(`tools/e2e/features/input.min` と同じ)。目標は次の 2 つ。
1. 下の一覧の機能を、1 つずつ、期待する結果を持つシナリオで覆うこと。
2. 覆った機能の数と、覆えなかった機能の理由を、報告に書くこと。

## 背景

- #{{199}}(E0)が作る物: `defineScenario`・`run.mjs --area`・`--list`・`--shard`・`--budget`・`dpr:<数>` の表示・機能の一覧・`coverage.mjs`・見本 3 本(`tools/e2e/scenarios/input/`)・共通の部品(`lib/common.mjs` の `menuPath`・`waitWindow`・`guestText`)。E0 が閉じてから始める。
- 入力は、実際のマウス・キーのイベントで送る(`Input.dispatchMouseEvent`・`Input.dispatchKeyEvent`)。ページの関数を直接呼ばない。
- カーソルの大きさは DPR に合う(物理の画素と CSS の画素の比)。クリップボードは、ページの Clipboard API と権限の 3 通りで試す(`lib/detect.mjs` の `permissionStates`・`clipboardReads`)。

## 読む物

- `tools/e2e/`(E0 の後の姿。lib/guest.mjs の操作・lib/detect.mjs の権限と検出)
- `tools/cursor/measure.mjs`・`tools/cursor/resource-cursor.mjs`・`tools/clipboard/measure.mjs`(既存の計測の方式)
- `tools/piano-roll-tools/measure.mjs`(ピアノロールの鉛筆)
- `tools/e2e/features/generated/input.tsv`(E0 が作る一覧)・`pm-work/findings/e2e-program.md`
- `app/src/` の入力・カーソル・クリップボードの扱い(`ls app/src`。モジュールの場所は作業で確かめる)

## やること

1. **機能の表** `tools/e2e/features/input.tsv`(手で書く。見出し行は E0 が作る)。機能ごとに 1 行。列は `id`・`kind`・`name`・`expected`・`how`。
2. **シナリオ**(`tools/e2e/scenarios/input/`。1 本 1 ファイル)。次の全てを、機能ごとに 1 本以上。
   - マウス: 左・中・右のクリック、ダブルクリック、ドラッグ(開始・途中・終了)、ホイールの縦と横、移動(1000 Hz の相当の間隔で、音声の途切れが無いことは #{{205}} で確かめる。ここでは入力の到達を確かめる)。
   - キーボード: 英数字、F1〜F12、修飾キーの組み合わせ(Ctrl・Shift・Alt)、矢印、Tab、Escape、Enter、Space、キーの繰り返し(押しっぱなし)、日本語の IME の合成(確定・変換・取り消し)。
   - タッチ: タップ、長押し、ドラッグ、2 本指のピンチ(窓の表で、受けた入力の結果を見る)。
   - カーソル: 窓の端・テキスト欄・ピアノロールの鉛筆・手のひら・リサイズの各カーソル。カーソルの画像の大きさが DPR に合う(期待値は DPR から計算する)。
   - クリップボード: コピー・貼り付けを、FL の全てのテキスト欄で。権限は 3 通り(`prompt`・`granted`・`denied`)。権限の変化の前後で、`navigator.clipboard` の呼び出しの数と、窓の表の結果を見る。
   - コンテキストメニュー: 右クリックで、FL の文脈メニューが出る。ブラウザの標準のメニューは出ない(`contextmenu` の既定の動作が止められている)。
   - ツールチップ: マウスを止めた時、ヒントの文字が出る(上部バーのヒントバーの文字を、ゲストのメモリの文字列で読む)。
3. **判定**: 各シナリオは `ok` と、測った値(`results`)を返す。`notes` に、判定の根拠(窓の表の行・クリップボードの呼び出し数・メモリの文字列)を 1 行で書く。「入力が送られた」だけで終わらない。結果(窓・値・文字)の変化で確かめる。
4. **表示**: 既定は `['4k-dpr1', '4k-random']`。カーソルの大きさは、DPR 1 と乱数の 2 点で確かめる。

## 受け入れ

```accept
npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false?0:1)' "$RUNNER_TEMP/boot.json" && node tools/smoke/welcome-flow.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/flow.jsonl" && node -e 'const fs=require("fs");const L=fs.readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const F=(f)=>L.find((r)=>r.flow===f);const ok=(r)=>!!r&&r.error===null&&r.state==="engine-idle"&&r.standIn===false&&r.trapped===false&&r.hostErrors===0&&r.opened===true;process.exit(L.length===4&&["A","B","C","D"].every((f)=>ok(F(f)))&&F("B").welcomeClosed===true&&F("D").welcomeClosed===true?0:1)' "$RUNNER_TEMP/flow.jsonl" && npm test --prefix app && (cd tools/e2e && npm ci && npx playwright install chromium) && node tools/e2e/run.mjs --area input --list > "$RUNNER_TEMP/list.jsonl" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8").trim().split("\n")[0]);process.exit(r.area==="input"&&r.scenarios>=120?0:1)' "$RUNNER_TEMP/list.jsonl" && node tools/e2e/run.mjs --area input --shard 1/8 --displays 4k-dpr1 --budget 40m > "$RUNNER_TEMP/e2e.jsonl" && node -e 'const L=require("fs").readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));process.exit(L.length===1&&L.every((r)=>r.error===null&&r.servedHeadersMatch===true&&r.scenarios.length>0&&r.scenarios.every((s)=>s.skipped!==true))?0:1)' "$RUNNER_TEMP/e2e.jsonl"
```

判定の基準:
- 受け入れの式が通る(流れ A〜D・`npm test --prefix app`・`--list` の input の scenarios が 120 以上・1 シャード(1/8)の実行が終了コード 0。`skipped` が無い)。
- 追加したシナリオの全てが、`covers` に機能の ID を持ち、その ID が `tools/e2e/features/input.tsv` か `generated/input.tsv` に在る。
- 上の一覧の各項目に、1 本以上のシナリオがある(覆えなかった項目は、理由を報告に書く)。
- 網羅率: 全シャードを合わせて、この領域の機能の 90% 以上が、`ok` かつ `expectFail` 無しのシナリオで覆われる(`judge-coverage.jq`。PM が判定する。報告に覆った数を書く)。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には「runner の値。この機械とは別枠」と書き、nproc と loadavg を添える。
- 「達成」「直った」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。出力の行を付ける。
- 報告に、(a) 追加したシナリオの ID と、各 `covers`、(b) 印 `expectFail: 'new'` の一覧(シナリオの ID・症状・停止の理由・run の出力の行)、(c) 通らなかった物と理由、(d) 覆った機能の数。

## 制約

- 製品(`app/`・`lift/`)は変えない(読むだけ)。製品の欠陥は、シナリオに `expectFail: 'new'` を付けて記録する。直すのは PM が起票する別の Issue。
- 製品に、利用者向けの新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足さない。テストのための分岐を製品に入れない。
- 領域の作業で作るファイルは、`tools/e2e/scenarios/input/`・`tools/e2e/features/input.tsv`・`tools/e2e/lib/input.mjs`(必要な時だけ)に限る。共有の部品の変更は、最小の差分にとどめ、変えた理由を報告に書く。
- Python を使わない。OCR を使わない。文字は、ゲストのメモリの文字列・窓の表・画素のハッシュで読む。
- ベンダー名・Cloudflare の識別子(アカウント・バケット・ホスト名・アクセス制御の値)を、ファイルに書かない。
- 期待値に、実行時の値(DPR・DPI・窓の位置)を定数で書かない。
- sleep・timeout のコマンド・nohup・末尾の `&` は使わない。待つ時は条件で待つ。一時ファイルは `.tmp-*` に置き、終わったら消す。
- 製品の作業は Actions で行う。この機械で製品のビルドや測定をしない。GitHub に、Issue・ラベル・コメントを書かない。

## 作業の進め方

- 開始の時刻を、最初に `date -u` で記録する。4 時間 30 分を過ぎたら、受け入れが通らなくても、作業ツリーを commit し、報告を書いて終える。
- ビルドは約 35 分。フォアグラウンドで直接、シェルツールの `timeout` 引数に 5400000 を渡して待つ。背景で起動して待たない。
- blend2d の取得(GitHub)が 403 で落ちたら一時的。受け入れのコマンドは `lift/out-app/b2d-src` を消してから作り直すので、そのまま再試行する。
- この領域は大きい。1 回で終わらない前提で、価値の高い順に足す(マウスとキーを先に、クリップボードと IME を後に)。再試行は前回の差分の上で続け、作業ツリーを消さない。
- 作業の最後に、受け入れが通らなくても、作業ツリーを commit する。

## 他のタスクとの関係

- blocked_by: #{{199}}(E0)。E0 が閉じるまで起動しない。
- 他の領域(#{{200}}・#{{202}}〜#{{212}})と、`coverage.json` の母数を共有するが、ファイルは共有しない。
- クリップボードの修正の Issue が後で来た時は、その Issue に対応するシナリオを 1 本以上足す(この領域の範囲)。
