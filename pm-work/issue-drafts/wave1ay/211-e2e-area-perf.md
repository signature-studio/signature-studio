---
draft: "211"
title: "E2E の領域 perf: 起動の時間・描画の fps と遅延・アイドルの CPU・メモリ・マウス 1000 Hz の下の音声の途切れを、基準との比の粗い上限で門にするシナリオを足す"
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

この領域(perf)は、性能の機能を、期待する結果つきのシナリオで確かめる。絶対値ではなく、基準との比の粗い上限で判定する(runner の揺れを考慮する)。最小のシナリオ数は 30 本(`tools/e2e/features/perf.min` と同じ)。目標は次の 2 つ。
1. 起動・描画・アイドル・メモリ・音声の途切れを、基準との比で判定するシナリオで覆うこと。
2. 変更の前後の値を、同じ道具(A/B)で比べられること。

## 背景

- #{{199}}(E0)が作る物: `defineScenario`・`run.mjs --area`・`--list`・`--shard`・`--budget`・`--repeat`(起動の 3 回の中央値)・見本 3 本(`tools/e2e/scenarios/perf/`)。E0 が閉じてから始める。
- 性能の基準(ベースライン)は、この領域の中に置く: `tools/e2e/golden/perf/baseline.tsv`(項目・値・測った時の loadavg・nproc)。基準は、Actions の runner で、同じ道具で測った値から作る。基準の更新は、報告に理由を書いた時だけ。
- 既存の道具: `tools/boot-check/boot-bundle.mjs`(起動の時間)・`tools/p2-frames/measure.mjs`(描画の fps と遅延)・`tools/p3-audio-probe/`(音声の途切れ)・`tools/perf-beacons/`(実機の値の読み)・`app/src/perf/beacon.ts`(ビーコンの項目)。A/B の方式は #1397 の結果(`pm-work/findings/perf-windows-browser/` の下)。
- runner の揺れが大きい(GitHub の runner は GPU が無く、4K の起動は約 24 秒)。判定は、絶対値ではなく、基準との比(上限は粗い。例: 1.5 倍)で行う。

## 読む物

- `tools/e2e/`(E0 の後の姿)
- `tools/boot-check/boot-bundle.mjs`・`tools/boot-check/cdp.mjs`
- `tools/p2-frames/measure.mjs`・`pm-work/findings/p2-render-1351.md`・`pm-work/findings/p2-measure-1308.md`
- `tools/p3-audio-probe/measure.mjs`・`pm-work/findings/p3-probe-1335.patch`
- `tools/perf-beacons/`・`app/src/perf/beacon.ts`(ビーコンの項目。実機の値の名前)
- `pm-work/findings/perf-windows-browser/`(#1397 の A/B の形)・`pm-work/findings/startup-breakdown-1329.md`
- `tools/e2e/features/generated/perf.tsv`(E0 が作る一覧。無い時は、この領域の表で補う)・`pm-work/findings/e2e-program.md`

## やること

1. **機能の表** `tools/e2e/features/perf.tsv`(手で書く。見出し行は E0 が作る)。機能ごとに 1 行。列は `id`・`kind`・`name`・`expected`(基準との比の上限)・`how`。
2. **シナリオ**(`tools/e2e/scenarios/perf/`。1 本 1 ファイル)。次の全てを、機能ごとに 1 本以上。
   - **起動の時間**: 起動(navigation から `engine-idle` まで)を 3 回測り、中央値を取る(`--repeat 3`)。期待: 中央値が、基準の値の 1.5 倍以下(粗い上限)。
   - **描画の 1 秒窓の最小 fps**: 操作の無い時と、マウスを動かした時の、rAF の 1 秒窓の最小。期待: 基準の比で、低下が 1/1.5 以上(粗い下限)。
   - **入力から提示までの遅延**: クリックから、画素が変わるまでの時間(中央値)。期待: 基準の比で 1.5 倍以下。
   - **アイドルの CPU**(`busyPct`): 操作の無い 60 秒の平均。期待: 基準の比で 1.5 倍以下。
   - **メモリ**: 起動の後と、10 分の後(アイドル)の使用量。期待: 増え続けない(10 分後が、起動の後の 1.3 倍以下)。
   - **マウス 1000 Hz の下の音声の途切れ**: 音が出る状態で、マウスを 1000 Hz の相当の間隔で動かし続けた時の、途切れ(underrun)。期待: 0。
   - **変更前後の A/B**(#1397 の形): 同じ道具で、2 つの bundle(基準の版と、今の版)を測り、比を出す。期待: 比が上限の内。基準の版は、報告の run の出力から作ったファイルを使う。
3. **基準の作り方**: 初回の実行で、基準(`tools/e2e/golden/perf/baseline.tsv`)を作る。測った値・nproc・loadavg(測る前と後)を書く。基準の更新は、報告に理由を書いた時だけ。
4. **判定**: 各シナリオは `ok` と、測った値(`results`: 中央値・最小・比・nproc・loadavg)を返す。`notes` に、判定の根拠を 1 行で書く。
5. **表示**: 既定は `['4k-dpr1']`。描画の遅延だけ、乱数の DPR を足す。

## 受け入れ

```accept
npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false?0:1)' "$RUNNER_TEMP/boot.json" && node tools/smoke/welcome-flow.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/flow.jsonl" && node -e 'const fs=require("fs");const L=fs.readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const F=(f)=>L.find((r)=>r.flow===f);const ok=(r)=>!!r&&r.error===null&&r.state==="engine-idle"&&r.standIn===false&&r.trapped===false&&r.hostErrors===0&&r.opened===true;process.exit(L.length===4&&["A","B","C","D"].every((f)=>ok(F(f)))&&F("B").welcomeClosed===true&&F("D").welcomeClosed===true?0:1)' "$RUNNER_TEMP/flow.jsonl" && npm test --prefix app && (cd tools/e2e && npm ci && npx playwright install chromium) && node tools/e2e/run.mjs --area perf --list > "$RUNNER_TEMP/list.jsonl" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8").trim().split("\n")[0]);process.exit(r.area==="perf"&&r.scenarios>=30?0:1)' "$RUNNER_TEMP/list.jsonl" && node tools/e2e/run.mjs --area perf --shard 1/8 --displays 4k-dpr1 --budget 40m > "$RUNNER_TEMP/e2e.jsonl" && node -e 'const L=require("fs").readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));process.exit(L.length===1&&L.every((r)=>r.error===null&&r.servedHeadersMatch===true&&r.scenarios.length>0&&r.scenarios.every((s)=>s.skipped!==true))?0:1)' "$RUNNER_TEMP/e2e.jsonl"
```

判定の基準:
- 受け入れの式が通る(流れ A〜D・`npm test --prefix app`・`--list` の perf の scenarios が 30 以上・1 シャード(1/8)の実行が終了コード 0。`skipped` が無い)。
- 上の 7 項目の全てに、1 本以上のシナリオがある。
- 基準(`baseline.tsv`)が在り、測った値が、基準の比の上限の内(起動・遅延・CPU・メモリ・fps)。上限を超えた項目は、印 `expectFail: 'new'` を付け、値を報告に書く。
- 音声の途切れ(マウス 1000 Hz の下)は 0。
- 網羅率: 全シャードを合わせて、この領域の機能の 90% 以上が、`ok` かつ `expectFail` 無しのシナリオで覆われる(`judge-coverage.jq`。PM が判定する。報告に覆った数を書く)。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には「runner の値。この機械とは別枠」と書き、nproc と loadavg(測る前と後)を添える。値の揺れ(3 回の値)も書く。
- 「達成」「直った」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。出力の行を付ける。
- 報告に、(a) 追加したシナリオの ID と、各 `covers`、(b) 基準の値(`baseline.tsv`)と、その作り方、(c) 測った値の中央値と比、(d) A/B の比、(e) 印 `expectFail: 'new'` の一覧、(f) 通らなかった物と理由。
- 比の上限(1.5 倍など)は、runner の揺れの見積もりを報告に書く(同じ道具で 2 回測った差)。

## 制約

- 製品(`app/`・`lift/`)は変えない(読むだけ)。製品の欠陥は、シナリオに `expectFail: 'new'` を付けて記録する。直すのは PM が起票する別の Issue。
- 製品に、利用者向けの新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足さない。テストのための分岐を製品に入れない。計測のための分岐も入れない。
- 領域の作業で作るファイルは、`tools/e2e/scenarios/perf/`・`tools/e2e/features/perf.tsv`・`tools/e2e/golden/perf/`・`tools/e2e/lib/perf.mjs`(必要な時だけ)に限る。
- 基準の値は、この機械で測らない。runner(Actions)で測った値から作る。
- Python を使わない。OCR を使わない。
- ベンダー名・Cloudflare の識別子(アカウント・バケット・ホスト名・アクセス制御の値)を、ファイルに書かない。
- 期待値に、絶対値(ミリ秒・fps・バイト)を書かない。比で書く。基準は、報告の run から作る。
- sleep・timeout のコマンド・nohup・末尾の `&` は使わない。待つ時は条件で待つ。一時ファイルは `.tmp-*` に置き、終わったら消す。
- 製品の作業は Actions で行う。この機械で製品のビルドや測定をしない。GitHub に、Issue・ラベル・コメントを書かない。

## 作業の進め方

- 開始の時刻を、最初に `date -u` で記録する。4 時間 30 分を過ぎたら、受け入れが通らなくても、作業ツリーを commit し、報告を書いて終える。
- ビルドは約 35 分。フォアグラウンドで直接、シェルツールの `timeout` 引数に 5400000 を渡して待つ。背景で起動して待たない。
- blend2d の取得(GitHub)が 403 で落ちたら一時的。受け入れのコマンドは `lift/out-app/b2d-src` を消してから作り直すので、そのまま再試行する。
- 起動の時間を、先に(3 回の中央値)。次に描画・遅延・アイドル。A/B と 10 分のメモリは最後に足す。再試行は前回の差分の上で続け、作業ツリーを消さない。
- 作業の最後に、受け入れが通らなくても、作業ツリーを commit する。

## 他のタスクとの関係

- blocked_by: #{{199}}(E0)。E0 が閉じるまで起動しない。
- 起動や配信を変える Issue(`app/worker/`・起動の経路・bundle の大きさ)が閉じると、基準の値が変わりうる。基準の更新は、その Issue の報告の値で、理由を添えて行う。
- 他の領域(#{{200}}〜#{{210}}・#{{212}})と、`coverage.json` の母数を共有するが、ファイルは共有しない。
