---
draft: "208"
title: "E2E の領域 topbar: 上部バーの約 40 個の部品の、左クリック・右クリック・ホイール・ドラッグの効果(表示の値を含む)のシナリオを機能ごとに足す"
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

この領域(topbar)は、上部バーの部品の全てを、操作の効果つきのシナリオで確かめる。最小のシナリオ数は 60 本(`tools/e2e/features/topbar.min` と同じ)。目標は次の 2 つ。
1. 窓の表の `TWPControl` の全ての部品について、左クリック・右クリック・ホイール・ドラッグの効果を、1 本以上で確かめること。
2. 表示される値(CPU・メモリ・フレームレート・テンポ・位置)が、実際の値を出すことを、期待する形で確かめること。

## 背景

- #{{199}}(E0)が作る物: `defineScenario`・`run.mjs --area`・`--list`・`--shard`・`--budget`・上部バーの一覧 `tools/e2e/features/generated/topbar.tsv`(窓の表の `TWPControl` の題名の全て)・見本 3 本(`tools/e2e/scenarios/topbar/`)。E0 が閉じてから始める。
- 上部バーの値は、FL が呼ぶ API の戻り値から作られる。CPU Load と Frame Rate は、#1402(段階 2)の方式(FL に渡す入力の連鎖)。Memory Usage は #1391 の方式(`pm-work/findings/topbar-metrics-1391.md`)。
- ヒントバー(マウスを止めた時の文字)は、ゲストのメモリの文字列で読む(OCR は禁止)。

## 読む物

- `tools/e2e/`(E0 の後の姿)
- `tools/e2e/features/generated/topbar.tsv`(E0 が作る一覧)
- `tools/topbar-parts/`・`tools/topbar-metrics/measure.mjs`・`pm-work/findings/topbar-parts-1311.md`(部品の操作と値の読み方)
- `pm-work/findings/topbar-metrics-1391.md`・`pm-work/findings/topbar-cpu-fps-1402.md`(値の出どころ)
- `pm-work/findings/survey-c6-topbar-menus.md`・`tools/feat-c6-popup-menus/measure.mjs`(右クリックのメニュー)
- `pm-work/findings/e2e-program.md`

## やること

1. **機能の表** `tools/e2e/features/topbar.tsv`(手で書く。見出し行は E0 が作る)。部品ごとに、操作の数だけ行(左クリック・右クリック・ホイール・ドラッグ)。列は `id`・`kind`・`name`・`expected`・`how`。
2. **シナリオ**(`tools/e2e/scenarios/topbar/`。1 本 1 ファイル)。窓の表の `TWPControl` の全ての部品(約 40 個)について、次の操作の内で効果があるものを、1 本以上。
   - 左クリック: 窓が開く・状態が変わる(ボタンの点灯)・値が変わる。
   - 右クリック: 文脈メニューが出る(窓の表のクラス)。メニューの項目の効果は、`menus` の領域に任せ、ここでは開くことを確かめる。
   - ホイール: 値が増減する(テンポ・音量・位置など)。
   - ドラッグ: 値が変わる(テンポの編集・Song position・ノブの類)。
   - 主な部品の個別の効果:
     - テンポ(BPM)の編集: 値を入れて、表示が変わる。
     - Song position: 位置が変わり、表示が変わる。
     - Pattern selector: パターンが変わる。
     - Metronome・Precount・Loop record・Blend record・Start on input: 状態が変わる(点灯)。
     - 各 View ボタン(PL・PR・SS・FX・SB・ProjPicker・PlugPicker・TT・Kb など): 対応する窓が開く・閉じる。
     - Undo: 直前の操作が戻る(窓の表か画素で確かめる)。
     - Save as: 保存の窓が開く。
     - Render: レンダリングの窓が開く。
     - Edit audio: 音声の編集の窓が開く。
     - CPU・Memory・FPS の表示: 値が出る(数値。0 のまま固定ではない。#1402・#1391 の結果)。
     - ヒントバーの文字: マウスを部品に止めた時、その部品の説明が出る(ゲストのメモリの文字列で読む)。
3. **判定**: 各シナリオは `ok` と、測った値(`results`: 部品の題名・状態・値の前後・開いた窓のクラス・文字)を返す。`notes` に、判定の根拠を 1 行で書く。値の表示が「0」「空」のままでないことを、数値の形で確かめる。
4. **表示**: 既定は `['4k-dpr1', '4k-random']`。値の表示の物(CPU・Memory・FPS)は、DPR 1 と乱数の 2 点で確かめる。

## 受け入れ

```accept
npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false?0:1)' "$RUNNER_TEMP/boot.json" && node tools/smoke/welcome-flow.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/flow.jsonl" && node -e 'const fs=require("fs");const L=fs.readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const F=(f)=>L.find((r)=>r.flow===f);const ok=(r)=>!!r&&r.error===null&&r.state==="engine-idle"&&r.standIn===false&&r.trapped===false&&r.hostErrors===0&&r.opened===true;process.exit(L.length===4&&["A","B","C","D"].every((f)=>ok(F(f)))&&F("B").welcomeClosed===true&&F("D").welcomeClosed===true?0:1)' "$RUNNER_TEMP/flow.jsonl" && npm test --prefix app && (cd tools/e2e && npm ci && npx playwright install chromium) && node tools/e2e/run.mjs --area topbar --list > "$RUNNER_TEMP/list.jsonl" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8").trim().split("\n")[0]);process.exit(r.area==="topbar"&&r.scenarios>=60?0:1)' "$RUNNER_TEMP/list.jsonl" && node tools/e2e/run.mjs --area topbar --shard 1/8 --displays 4k-dpr1 --budget 40m > "$RUNNER_TEMP/e2e.jsonl" && node -e 'const L=require("fs").readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));process.exit(L.length===1&&L.every((r)=>r.error===null&&r.servedHeadersMatch===true&&r.scenarios.length>0&&r.scenarios.every((s)=>s.skipped!==true))?0:1)' "$RUNNER_TEMP/e2e.jsonl"
```

判定の基準:
- 受け入れの式が通る(流れ A〜D・`npm test --prefix app`・`--list` の topbar の scenarios が 60 以上・1 シャード(1/8)の実行が終了コード 0。`skipped` が無い)。
- 窓の表の `TWPControl` の全ての部品に、1 本以上のシナリオがある(一覧の件数と、覆った件数を報告に書く)。
- CPU・Memory・FPS の表示の物は、値が数値として出る(0 の固定でない)ことを、測った値で示す。
- 追加したシナリオの全てが、`covers` に機能の ID を持ち、その ID が一覧か `features/topbar.tsv` に在る。
- 網羅率: 全シャードを合わせて、この領域の機能の 90% 以上が、`ok` かつ `expectFail` 無しのシナリオで覆われる(`judge-coverage.jq`。PM が判定する。報告に覆った数を書く)。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には「runner の値。この機械とは別枠」と書き、nproc と loadavg を添える。
- 「達成」「直った」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。出力の行を付ける。
- 報告に、(a) 追加したシナリオの ID と、各 `covers`、(b) 部品の件数と、覆った件数、(c) 印 `expectFail: 'new'` の一覧(シナリオの ID・症状・停止の理由・run の出力の行)、(d) CPU・Memory・FPS の測った値の例、(e) 通らなかった物と理由。

## 制約

- 製品(`app/`・`lift/`)は変えない(読むだけ)。製品の欠陥は、シナリオに `expectFail: 'new'` を付けて記録する。直すのは PM が起票する別の Issue。
- 製品に、利用者向けの新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足さない。テストのための分岐を製品に入れない。
- 領域の作業で作るファイルは、`tools/e2e/scenarios/topbar/`・`tools/e2e/features/topbar.tsv`・`tools/e2e/lib/topbar.mjs`(必要な時だけ)に限る。`tools/topbar-metrics/`・`tools/topbar-parts/` は読むだけ。
- Python を使わない。OCR を使わない。文字は、ゲストのメモリの文字列・窓の表・画素のハッシュで読む。
- ベンダー名・Cloudflare の識別子(アカウント・バケット・ホスト名・アクセス制御の値)を、ファイルに書かない。
- 期待値に、実行時の値(DPR・DPI・窓の位置)を定数で書かない。
- sleep・timeout のコマンド・nohup・末尾の `&` は使わない。待つ時は条件で待つ。一時ファイルは `.tmp-*` に置き、終わったら消す。
- 製品の作業は Actions で行う。この機械で製品のビルドや測定をしない。GitHub に、Issue・ラベル・コメントを書かない。

## 作業の進め方

- 開始の時刻を、最初に `date -u` で記録する。4 時間 30 分を過ぎたら、受け入れが通らなくても、作業ツリーを commit し、報告を書いて終える。
- ビルドは約 35 分。フォアグラウンドで直接、シェルツールの `timeout` 引数に 5400000 を渡して待つ。背景で起動して待たない。
- blend2d の取得(GitHub)が 403 で落ちたら一時的。受け入れのコマンドは `lift/out-app/b2d-src` を消してから作り直すので、そのまま再試行する。
- この領域は大きい。1 回で終わらない前提で、値の表示(CPU・Memory・FPS)と、主な部品(テンポ・View ボタン・Undo)を先に、残りの部品を後に足す。再試行は前回の差分の上で続け、作業ツリーを消さない。
- 作業の最後に、受け入れが通らなくても、作業ツリーを commit する。

## 他のタスクとの関係

- blocked_by: #{{199}}(E0)。E0 が閉じるまで起動しない。
- 他の領域(#{{200}}〜#{{207}}・#{{209}}〜#{{212}})と、`coverage.json` の母数を共有するが、ファイルは共有しない。
- 上部バーの値(#1391・#1402)の修正の Issue が後で来た時は、その Issue に対応するシナリオを 1 本以上足す。
