---
draft: "204"
title: "E2E の領域 windows: Channel Rack・Playlist・Piano roll・Mixer・Browser・Step sequencer・Event editor・Plugin picker・Project picker と、窓の操作の、期待する結果のシナリオを機能ごとに足す"
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

この領域(windows)は、FL の窓とパネルの機能を、期待する結果つきのシナリオで確かめる。最小のシナリオ数は 250 本(`tools/e2e/features/windows.min` と同じ)。目標は次の 2 つ。
1. 下の一覧の機能を、1 つずつ、期待する結果を持つシナリオで覆うこと。
2. 覆った機能の数と、覆えなかった機能の理由を、報告に書くこと。

## 背景

- #{{199}}(E0)が作る物: `defineScenario`・`run.mjs --area`・`--list`・`--shard`・`--budget`・窓の一覧 `tools/e2e/features/generated/windows.tsv`(窓の表のクラス・題名。F キー・メニュー・ボタンで開く物。ブラウザのタブ・ミキサーのインサート・ピアノロールの子窓を含む)・`lib/common.mjs` の `waitWindow`・見本 3 本(`tools/e2e/scenarios/windows/`)。E0 が閉じてから始める。
- 窓の確認は、窓の表(`guest.windows()`・`findWindow`)で行う。窓の中の値は、ゲストのメモリの文字列か、窓の表の題名で読む。
- 操作は、窓の座標を窓の表から取り、実際のマウス・キーで行う(座標は定数にしない)。

## 読む物

- `tools/e2e/`(E0 の後の姿。lib/guest.mjs の `windows`・`findWindow`・`click`・`drag`・`wheel`)
- `tools/piano-roll-tools/measure.mjs`・`tools/settings-scroll/measure.mjs`・`tools/persist/measure.mjs`(既存の窓の操作)
- `tools/e2e/features/generated/windows.tsv`(E0 が作る一覧)
- `pm-work/findings/hidpi-fl-scaling.md`・`pm-work/findings/guest-layout.md`(窓の表と座標)
- `pm-work/findings/settings-scroll-1384.md`(設定の窓の操作)
- `pm-work/findings/e2e-program.md`

## やること

1. **機能の表** `tools/e2e/features/windows.tsv`(手で書く。見出し行は E0 が作る)。機能ごとに 1 行。列は `id`・`kind`・`name`・`expected`・`how`。
2. **シナリオ**(`tools/e2e/scenarios/windows/`。1 本 1 ファイル)。次の全てを、機能ごとに 1 本以上。
   - **Channel Rack**: チャンネルの追加・削除、ステップの入力(ステップが点く。窓の画素の変化と、メモリの値で確かめる)、ミュート・ソロ(状態が変わる)、ノブ(ドラッグで値が変わる)、グラフエディタ(開く・値を変える)、右クリックのメニュー(項目が出る)。
   - **Playlist**: ツール(選ぶ・描く・消す)、トラック、クリップ(置く・動かす・消す)、ズーム、スナップ、マーカー、範囲の選択。
   - **Piano roll**: ツール(鉛筆・選択など)、ノートの追加・移動・削除、コード・リフの機能、スナップ、ズーム、Undo と Redo(Undo で戻り、Redo で進む)。
   - **Mixer**: インサート(開く・閉じる)、ルーティング(変える)、FX スロット(プラグインを選ぶ・外す)、フェーダー(値が変わる)、メーター(動く)。
   - **Browser**: 全てのタブ(窓の表のタブの 8 の枠)、検索(文字を入れると一覧が絞られる)、プレビュー(音が鳴る物の選択で、プレビューが始まる。音の中身は #{{205}} で確かめる)。
   - **Step sequencer**・**Event editor**・**Plugin picker**・**Project picker**: 開く・値を変える・閉じる。
   - **窓の操作**(全ての窓について): 移動(ドラッグで位置が変わる)・サイズ変更(端のドラッグで大きさが変わる)・最小化・最大化・閉じる・F12(全て閉じる)。窓の表で確かめる。
3. **判定**: 各シナリオは `ok` と、測った値(`results`: 窓の位置・大きさ・題名・値・状態)を返す。`notes` に、判定の根拠を 1 行で書く。「操作できた」だけで終わらない。変化の前後の値を比べる。
4. **表示**: 既定は `['4k-dpr1', '4k-random']`。窓の移動・サイズ変更の物は、DPR 1 と乱数の 2 点で確かめる(期待値は、DPR から計算する)。

## 受け入れ

```accept
npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false?0:1)' "$RUNNER_TEMP/boot.json" && node tools/smoke/welcome-flow.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/flow.jsonl" && node -e 'const fs=require("fs");const L=fs.readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const F=(f)=>L.find((r)=>r.flow===f);const ok=(r)=>!!r&&r.error===null&&r.state==="engine-idle"&&r.standIn===false&&r.trapped===false&&r.hostErrors===0&&r.opened===true;process.exit(L.length===4&&["A","B","C","D"].every((f)=>ok(F(f)))&&F("B").welcomeClosed===true&&F("D").welcomeClosed===true?0:1)' "$RUNNER_TEMP/flow.jsonl" && npm test --prefix app && (cd tools/e2e && npm ci && npx playwright install chromium) && node tools/e2e/run.mjs --area windows --list > "$RUNNER_TEMP/list.jsonl" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8").trim().split("\n")[0]);process.exit(r.area==="windows"&&r.scenarios>=250?0:1)' "$RUNNER_TEMP/list.jsonl" && node tools/e2e/run.mjs --area windows --shard 1/8 --displays 4k-dpr1 --budget 40m > "$RUNNER_TEMP/e2e.jsonl" && node -e 'const L=require("fs").readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));process.exit(L.length===1&&L.every((r)=>r.error===null&&r.servedHeadersMatch===true&&r.scenarios.length>0&&r.scenarios.every((s)=>s.skipped!==true))?0:1)' "$RUNNER_TEMP/e2e.jsonl"
```

判定の基準:
- 受け入れの式が通る(流れ A〜D・`npm test --prefix app`・`--list` の windows の scenarios が 250 以上・1 シャード(1/8)の実行が終了コード 0。`skipped` が無い)。
- 一覧の全ての窓(`generated/windows.tsv` の行)に、1 本以上のシナリオがある。覆えなかった窓は、理由を報告に書く。
- 追加したシナリオの全てが、`covers` に機能の ID を持ち、その ID が一覧か `features/windows.tsv` に在る。
- 窓の操作(移動・サイズ変更・最小化・最大化・閉じる・F12)は、窓の全ての種類について、それぞれ 1 本以上で確かめる。
- 網羅率: 全シャードを合わせて、この領域の機能の 90% 以上が、`ok` かつ `expectFail` 無しのシナリオで覆われる(`judge-coverage.jq`。PM が判定する。報告に覆った数を書く)。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には「runner の値。この機械とは別枠」と書き、nproc と loadavg を添える。
- 「達成」「直った」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。出力の行を付ける。
- 報告に、(a) 追加したシナリオの ID と、各 `covers`、(b) 印 `expectFail: 'new'` の一覧(シナリオの ID・症状・停止の理由・run の出力の行)、(c) 通らなかった物と理由、(d) 覆った機能の数。

## 制約

- 製品(`app/`・`lift/`)は変えない(読むだけ)。製品の欠陥は、シナリオに `expectFail: 'new'` を付けて記録する。直すのは PM が起票する別の Issue。
- 製品に、利用者向けの新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足さない。テストのための分岐を製品に入れない。
- 領域の作業で作るファイルは、`tools/e2e/scenarios/windows/`・`tools/e2e/features/windows.tsv`・`tools/e2e/lib/windows.mjs`(必要な時だけ)に限る。共有の部品の変更は、最小の差分にとどめ、変えた理由を報告に書く。
- Python を使わない。OCR を使わない。文字は、ゲストのメモリの文字列・窓の表・画素のハッシュで読む。
- ベンダー名・Cloudflare の識別子(アカウント・バケット・ホスト名・アクセス制御の値)を、ファイルに書かない。
- 期待値に、実行時の値(DPR・DPI・窓の位置)を定数で書かない。窓の表から取る。
- sleep・timeout のコマンド・nohup・末尾の `&` は使わない。待つ時は条件で待つ。一時ファイルは `.tmp-*` に置き、終わったら消す。
- 製品の作業は Actions で行う。この機械で製品のビルドや測定をしない。GitHub に、Issue・ラベル・コメントを書かない。

## 作業の進め方

- 開始の時刻を、最初に `date -u` で記録する。4 時間 30 分を過ぎたら、受け入れが通らなくても、作業ツリーを commit し、報告を書いて終える。
- ビルドは約 35 分。フォアグラウンドで直接、シェルツールの `timeout` 引数に 5400000 を渡して待つ。背景で起動して待たない。
- blend2d の取得(GitHub)が 403 で落ちたら一時的。受け入れのコマンドは `lift/out-app/b2d-src` を消してから作り直すので、そのまま再試行する。
- この領域は大きい。1 回で終わらない前提で、価値の高い順に足す(Channel Rack・Playlist・Piano roll・窓の操作を先に、Browser の全タブを後に)。再試行は前回の差分の上で続け、作業ツリーを消さない。
- 作業の最後に、受け入れが通らなくても、作業ツリーを commit する。

## 他のタスクとの関係

- blocked_by: #{{199}}(E0)。E0 が閉じるまで起動しない。
- 他の領域(#{{200}}〜#{{203}}・#{{205}}〜#{{212}})と、`coverage.json` の母数を共有するが、ファイルは共有しない。
- ピアノロール・チャンネルラックの修正の Issue が後で来た時は、その Issue に対応するシナリオを 1 本以上足す(この領域の範囲)。
