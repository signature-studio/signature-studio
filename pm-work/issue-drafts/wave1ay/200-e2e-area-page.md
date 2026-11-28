---
draft: "200"
title: "E2E の領域 page: ページとブラウザ(表示の鮮明さ・サイズ変更・DPR の変化・再読み込み・WebGL の喪失・console・配信のヘッダー・ビーコン)の、期待する結果のシナリオを機能ごとに足す"
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

この領域(page)は、ページとブラウザの機能を、期待する結果つきのシナリオで確かめる。最小のシナリオ数は 80 本(`tools/e2e/features/page.min` と同じ)。目標は次の 2 つ。
1. 下の一覧の機能を、1 つずつ、期待する結果を持つシナリオで覆うこと。
2. 覆った機能の数と、覆えなかった機能の理由を、報告に書くこと。

## 背景

- #{{199}}(E0)が作る物: `defineScenario`(`tools/e2e/lib/scenario.mjs`)・`run.mjs --area`・`--list`・`--shard`・`--budget`・`dpr:<数>` の表示・機能の一覧 `tools/e2e/features/generated/page.tsv`(ページの機能。`app/src/` のモジュールから)・`coverage.mjs`・見本 3 本(`tools/e2e/scenarios/page/`)。E0 が閉じてから始める。
- 依頼者の指示の意味は、「クラッシュを探す走査」ではなく、存在する機能ごとに、期待する結果を確かめること。
- 今の基盤は、DPR 1 と 4K の画面で動く(#1407)。この領域は、DPR を 1・1.25・1.5・1.75・2・2.5・3 と、乱数の 2 点で動かす。
- HTML の見える要素の数は、#1377 の後の姿で確かめる(実機のビーコンの親 Issue)。

## 読む物

- `tools/e2e/`(E0 の後の姿。README.md・run.mjs・lib/browser.mjs・lib/delivery.mjs・lib/served.mjs・lib/detect.mjs・scenarios/smoke/)
- `app/worker/index.ts`(応答・`baseHeaders`・`/assets/*`)・`app/shared/headers.ts`(`ISOLATION_HEADERS`・`PRIVACY_HEADERS`・`CSP`・`cacheControlFor`)
- `app/src/perf/beacon.ts`(ビーコンの項目)と、ビーコンの形の検証の関数(`beaconProblems`。場所は作業で確かめる)
- `app/src/main.ts`(`window.__app`・canvas の大きさの扱い・WebGL)
- `tools/e2e/features/generated/page.tsv`(E0 が作る一覧)
- `pm-work/findings/e2e-program.md`(方針)

## やること

1. **機能の表** `tools/e2e/features/page.tsv`(手で書く。見出し行は E0 が作る)。機能ごとに 1 行。列は `id`・`kind`・`name`・`expected`(期待する結果)・`how`(判定の方法)。
2. **シナリオ**(`tools/e2e/scenarios/page/`。1 本 1 ファイル)。次の全てを、機能ごとに 1 本以上。
   - `canvas` がビューポートを占める(窓の大きさと、canvas の大きさが一致する)。
   - DPR ごとの、画素の鮮明さ(縁の幅。ぼやけない)。DPR は 1・1.25・1.5・1.75・2・2.5・3 と、乱数の 2 点。期待値は、DPR と canvas の大きさから計算する(定数にしない)。
   - ウィンドウのサイズ変更(幅と高さを変える。canvas の大きさが追従する。窓の表の位置も追従する)。
   - DPR の実行中の変化(別のモニターへ移動した模擬: 途中で DPR を変える)。変えた後の鮮明さと、描画の継続。
   - ズーム(ページのズーム。50%〜300%)。
   - 全画面(要求と解除。解除の後に、描画が戻る)。
   - タブの非表示と表示(`visibilitychange`)。表示に戻った後に、描画が続く。
   - フォーカスの喪失(`blur`)と復帰。
   - 再読み込み(20 回の繰り返し。毎回、起動が完了し、窓の表が取れる)。
   - 戻る・進む(履歴の操作で、ページが壊れない)。
   - WebGL コンテキストの喪失と復元(`webglcontextlost`・`webglcontextrestored`)。喪失の後に、描画が戻る。
   - ページの console が空(エラー 0・警告の上限。上限は、報告の初回の値を基準に、警告の数の増加を見る)。
   - HTML の見える要素が 0(#1377 の後の姿)。見える要素は、許す物の一覧(`#status`・`#audio-status`・`#taskbar` などの、既存の常設の物)と照らす。
   - 初回の描画の完全性(黒い領域 0。`noBlackRegions`)。
   - `Cross-Origin-*`・CSP のヘッダー(`app/shared/headers.ts` の値と一致)。配信の応答で確かめる(`lib/served.mjs`)。
   - ビーコンの送信(要求の形。個人情報が無い。`beaconProblems` の結果が空)。
3. **判定**: 各シナリオは `ok` と、測った値(`results`)を返す。`notes` に、判定の根拠(画素の値・窓の表の行・要求の形)を 1 行で書く。「止まらなかった」だけで終わらない。
4. **表示**: `displays` は `['dpr:1', 'dpr:1.25', 'dpr:1.5', 'dpr:1.75', 'dpr:2', 'dpr:2.5', 'dpr:3', 'random']` の内の必要な物。重い物(再読み込みを 20 回する物)は DPR 1 だけにしてよい(理由を `notes` に書く)。

## 受け入れ

```accept
npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false?0:1)' "$RUNNER_TEMP/boot.json" && node tools/smoke/welcome-flow.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/flow.jsonl" && node -e 'const fs=require("fs");const L=fs.readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const F=(f)=>L.find((r)=>r.flow===f);const ok=(r)=>!!r&&r.error===null&&r.state==="engine-idle"&&r.standIn===false&&r.trapped===false&&r.hostErrors===0&&r.opened===true;process.exit(L.length===4&&["A","B","C","D"].every((f)=>ok(F(f)))&&F("B").welcomeClosed===true&&F("D").welcomeClosed===true?0:1)' "$RUNNER_TEMP/flow.jsonl" && npm test --prefix app && (cd tools/e2e && npm ci && npx playwright install chromium) && node tools/e2e/run.mjs --area page --list > "$RUNNER_TEMP/list.jsonl" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8").trim().split("\n")[0]);process.exit(r.area==="page"&&r.scenarios>=80?0:1)' "$RUNNER_TEMP/list.jsonl" && node tools/e2e/run.mjs --area page --shard 1/8 --displays 4k-dpr1 --budget 40m > "$RUNNER_TEMP/e2e.jsonl" && node -e 'const L=require("fs").readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));process.exit(L.length===1&&L.every((r)=>r.error===null&&r.servedHeadersMatch===true&&r.scenarios.length>0&&r.scenarios.every((s)=>s.skipped!==true))?0:1)' "$RUNNER_TEMP/e2e.jsonl"
```

判定の基準:
- 受け入れの式が通る(流れ A〜D・`npm test --prefix app`・`--list` の page の scenarios が 80 以上・1 シャード(1/8)の実行が終了コード 0。`skipped` が無い)。
- 追加したシナリオの全てが、`covers` に機能の ID を持ち、その ID が `tools/e2e/features/page.tsv` か `generated/page.tsv` に在る。
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
- 領域の作業で作るファイルは、`tools/e2e/scenarios/page/`・`tools/e2e/features/page.tsv`・`tools/e2e/lib/page.mjs`(必要な時だけ)に限る。`run.mjs`・`lib/` の既存の物・`judge.jq`・`judge-coverage.jq`・`coverage.mjs` は、最小の差分にとどめ、変えた理由を報告に書く。
- Python を使わない。OCR を使わない。文字は、ゲストのメモリの文字列・窓の表・画素のハッシュで読む。
- ベンダー名・Cloudflare の識別子(アカウント・バケット・ホスト名・アクセス制御の値)を、ファイルに書かない。
- 期待値に、実行時の値(DPR・DPI・窓の位置)を定数で書かない。
- sleep・timeout のコマンド・nohup・末尾の `&` は使わない。待つ時は条件で待つ。一時ファイルは `.tmp-*` に置き、終わったら消す。
- 製品の作業は Actions で行う。この機械で製品のビルドや測定をしない。GitHub に、Issue・ラベル・コメントを書かない。

## 作業の進め方

- 開始の時刻を、最初に `date -u` で記録する。4 時間 30 分を過ぎたら、受け入れが通らなくても、作業ツリーを commit し、報告を書いて終える。
- ビルドは約 35 分。フォアグラウンドで直接、シェルツールの `timeout` 引数に 5400000 を渡して待つ。背景で起動して待たない。
- blend2d の取得(GitHub)が 403 で落ちたら一時的。受け入れのコマンドは `lift/out-app/b2d-src` を消してから作り直すので、そのまま再試行する。
- この領域は大きい。1 回で終わらない前提で、価値の高い順に足す(再読み込み・描画の鮮明さ・WebGL・ヘッダーを先に、ビーコンを後に)。再試行は前回の差分の上で続け、作業ツリーを消さない。
- 作業の最後に、受け入れが通らなくても、作業ツリーを commit する。

## 他のタスクとの関係

- blocked_by: #{{199}}(E0)。E0 が閉じるまで起動しない。
- 他の領域(#{{201}}〜#{{212}})と、`coverage.json` の母数を共有するが、ファイルは共有しない。
- 製品の修正の Issue が後で来た時は、その Issue に対応する領域のシナリオを 1 本以上足す(この領域の範囲の修正)。
