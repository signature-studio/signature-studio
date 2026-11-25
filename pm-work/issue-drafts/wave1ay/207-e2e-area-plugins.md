---
draft: "207"
title: "E2E の領域 plugins: 内蔵プラグインの全て(窓・パラメータ・音声)と外部プラグイン(OTT・Serum 2・Vital)の、期待する結果のシナリオを一覧から生成して足す"
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

この領域(plugins)は、プラグインの機能を、期待する結果つきのシナリオで確かめる。内蔵プラグインは一覧から生成し、1 本ずつ同じ形のシナリオにする(データ駆動)。最小のシナリオ数は 200 本(`tools/e2e/features/plugins.min` と同じ)。目標は次の 2 つ。
1. 内蔵プラグインの全て(読み込める物)と、外部プラグインの全てを、窓・パラメータ・音声の結果で覆うこと。
2. 覆えなかった物を、理由つきで報告し、欠落の物は `expectFail` で記録すること。

## 背景

- #{{199}}(E0)が作る物: `defineScenario`・`run.mjs --area`・`--list`・`--shard`・`--budget`・プラグインの一覧 `tools/e2e/features/generated/plugins.tsv`(`inputs/plugin-classes.tsv` の本体と、`inputs/external-plugins.tsv` の外部)・見本 3 本(`tools/e2e/scenarios/plugins/`)。E0 が閉じてから始める。
- 内蔵プラグインは、Effects(Mixer の FX スロットに入れる)と Generators(Channel Rack に入れる)に分かれる。1 プラグイン = 1 つの wasm(`lift/plugins/` の物と、その他の物)。読み込める物は、その状態で決まる。読み込めない物は、欠落として `expectFail` で記録する。
- 1 本のシナリオは、次の 4 段で作る: 窓が開く → パラメータを 1 つ変える(画素の変化) → 音声を処理する(Generators は非無音、Effects は入力を変える)→ engine が止まらない。
- 既存の道具: `tools/plugin-run/measure.mjs`(#1371。1 本ずつ、起動から 1 ブロックの処理まで走らせる)・`tools/plugin-classes/`・`tools/plugin-layout/`・`tools/plugin-hooks/`。E2E の中では、これらを重複させず、必要な部品を `lib/plugins.mjs` に写す。

## 読む物

- `tools/e2e/`(E0 の後の姿)
- `tools/e2e/features/generated/plugins.tsv`(E0 が作る一覧)・`inputs/plugin-classes.tsv`・`inputs/external-plugins.tsv`・`inputs/plugin-hooks.tsv`・`lift/plugins/`
- `tools/plugin-run/measure.mjs`・`tools/plugin-classes/`・`tools/plugin-layout/`・`tools/plugin-hooks/`
- `pm-work/findings/survey-c4-builtin-plugins.md`・`pm-work/findings/survey-c5-external-plugins.md`・`pm-work/findings/serum2-1336.md`・`pm-work/findings/c5-ott-1337.md`・`pm-work/findings/vital-read-1338.md`
- `pm-work/created-issues.tsv`(欠落の expectFail に使う Issue の番号を、この表から取る)

## やること

1. **機能の表** `tools/e2e/features/plugins.tsv`(手で書く。見出し行は E0 が作る)。プラグインごとに、次の 4 行(窓・パラメータ・音声・止まらない)。列は `id`・`kind`・`name`・`expected`・`how`。
2. **データ駆動のシナリオ**: 内蔵プラグインは、一覧(`plugins.tsv` の本体の行)から、1 プラグイン 1 本ずつ生成する(`tools/e2e/scenarios/plugins/builtin-<slug>.mjs` の形を、生成の手順と一緒に commit する。手で 1 本ずつ書かない)。外部のプラグインも同じ形で 1 本ずつ。
   - **内蔵プラグインの全て**(`lift/plugins/*` と `inputs/plugin-classes.tsv` の、読み込める物): Effects は Mixer の FX スロットに、Generators は Channel Rack に、1 つずつ入れる。その後、次を確かめる。
     - 窓が開く(窓の表のクラス・題名。プラグインの名前を含む)。
     - パラメータを 1 つ変える(画素が変わる。変化の範囲を `notes` に書く)。
     - 音声を処理する(Generators は非無音の量子が 0 より大きい。Effects は、入力の音に対して出力が変わる)。
     - engine が止まらない(`engineStops` が 0)。
   - **欠落したプラグイン**(読み込めない・窓が出ない・止まる物): シナリオに `expectFail` を付ける。付ける値は、その欠落に当たる Issue の番号(`#` で始める)。番号は `pm-work/created-issues.tsv` の表で探す(題名で一致させる)。表に当たる Issue が無い物は、`expectFail: 'new'`。
   - **外部プラグイン**(OTT・Serum 2・Vital): 読み込み(窓が開く)・音声の処理(Generators は非無音)・パラメータの変化を、内蔵と同じ 4 段で確かめる。読み込めない物は、上と同じ規則で `expectFail` を付ける。
   - **Plugin database の一覧**(窓の表に出る一覧)と **Plugin manager**: 開く・一覧の件数が出る・項目を選ぶと、窓の表が変わる。
   - **プリセットの読み込み**: 1 つ以上のプリセットを読み込み、パラメータの値が変わる(画素の変化か、窓の表の値)。
3. **判定**: 各シナリオは `ok` と、測った値(`results`: 窓のクラス・題名・パラメータの画素の差・音声の非無音の量子の数)を返す。`notes` に、判定の根拠を 1 行で書く。
4. **表示**: 既定は `['4k-dpr1']`(プラグインの窓の画素は DPR の影響を受けるが、ここでは機能の確認が目的)。窓の位置の確認だけ、乱数の DPR を足す。

## 受け入れ

```accept
npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false?0:1)' "$RUNNER_TEMP/boot.json" && node tools/smoke/welcome-flow.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/flow.jsonl" && node -e 'const fs=require("fs");const L=fs.readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const F=(f)=>L.find((r)=>r.flow===f);const ok=(r)=>!!r&&r.error===null&&r.state==="engine-idle"&&r.standIn===false&&r.trapped===false&&r.hostErrors===0&&r.opened===true;process.exit(L.length===4&&["A","B","C","D"].every((f)=>ok(F(f)))&&F("B").welcomeClosed===true&&F("D").welcomeClosed===true?0:1)' "$RUNNER_TEMP/flow.jsonl" && npm test --prefix app && (cd tools/e2e && npm ci && npx playwright install chromium) && node tools/e2e/run.mjs --area plugins --list > "$RUNNER_TEMP/list.jsonl" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8").trim().split("\n")[0]);process.exit(r.area==="plugins"&&r.scenarios>=200?0:1)' "$RUNNER_TEMP/list.jsonl" && node tools/e2e/run.mjs --area plugins --shard 1/8 --displays 4k-dpr1 --budget 40m > "$RUNNER_TEMP/e2e.jsonl" && node -e 'const L=require("fs").readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));process.exit(L.length===1&&L.every((r)=>r.error===null&&r.servedHeadersMatch===true&&r.scenarios.length>0&&r.scenarios.every((s)=>s.skipped!==true))?0:1)' "$RUNNER_TEMP/e2e.jsonl"
```

判定の基準:
- 受け入れの式が通る(流れ A〜D・`npm test --prefix app`・`--list` の plugins の scenarios が 200 以上・1 シャード(1/8)の実行が終了コード 0。`skipped` が無い)。
- 内蔵プラグインの一覧の全ての本体(`plugins.tsv` の行)に、1 本以上のシナリオがある。読み込めた物は、4 段(窓・パラメータ・音声・止まらない)の全てを確かめる。読み込めない物は、`expectFail` の印を付けて、報告に一覧を書く。
- 外部プラグイン(3 件)の全てに、1 本以上のシナリオがある。
- 欠落の印(`expectFail`)の番号が、`created-issues.tsv` の表に在る(`new` を除く)。
- 網羅率: 全シャードを合わせて、この領域の機能の 90% 以上が、`ok` かつ `expectFail` 無しのシナリオで覆われる(`judge-coverage.jq`。PM が判定する。報告に覆った数を書く)。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には「runner の値。この機械とは別枠」と書き、nproc と loadavg を添える。
- 「達成」「直った」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。出力の行を付ける。
- 報告に、(a) 内蔵プラグインの読み込めた数と、読み込めなかった数(一覧の名前つき)、(b) 外部プラグインの 3 件の結果、(c) 印 `expectFail` の一覧(シナリオの ID・その番号・症状・停止の理由)、(d) 印 `expectFail: 'new'` の一覧、(e) 覆った機能の数、(f) 生成の手順(どの一覧から何本を生成したか)。

## 制約

- 製品(`app/`・`lift/`)は変えない(読むだけ)。製品の欠陥は、シナリオに `expectFail` を付けて記録する。直すのは PM が起票する別の Issue。
- 製品に、利用者向けの新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足さない。テストのための分岐を製品に入れない。
- 領域の作業で作るファイルは、`tools/e2e/scenarios/plugins/`・`tools/e2e/features/plugins.tsv`・`tools/e2e/lib/plugins.mjs`(必要な時だけ)に限る。`tools/plugin-run/` などの既存の道具は読むだけ。
- Python を使わない。OCR を使わない。
- ベンダー名・Cloudflare の識別子(アカウント・バケット・ホスト名・アクセス制御の値)を、ファイルに書かない。外部プラグインの名前は、一覧(`inputs/external-plugins.tsv`)の値のまま書く。
- 期待値に、実行時の値(DPR・DPI・窓の位置)を定数で書かない。
- sleep・timeout のコマンド・nohup・末尾の `&` は使わない。待つ時は条件で待つ。一時ファイルは `.tmp-*` に置き、終わったら消す。
- 製品の作業は Actions で行う。この機械で製品のビルドや測定をしない。GitHub に、Issue・ラベル・コメントを書かない。

## 作業の進め方

- 開始の時刻を、最初に `date -u` で記録する。4 時間 30 分を過ぎたら、受け入れが通らなくても、作業ツリーを commit し、報告を書いて終える。
- ビルドは約 35 分。フォアグラウンドで直接、シェルツールの `timeout` 引数に 5400000 を渡して待つ。背景で起動して待たない。
- blend2d の取得(GitHub)が 403 で落ちたら一時的。受け入れのコマンドは `lift/out-app/b2d-src` を消してから作り直すので、そのまま再試行する。
- 先に、生成の仕組み(一覧から 1 本を作る形)を 1 プラグインで確かめ、残りを生成する。1 回で終わらない前提で、読み込める物から順に足す。再試行は前回の差分の上で続け、作業ツリーを消さない。
- 作業の最後に、受け入れが通らなくても、作業ツリーを commit する。

## 他のタスクとの関係

- blocked_by: #{{199}}(E0)。E0 が閉じるまで起動しない。
- 第 2 波の束(内蔵プラグインの読み込みと Data の配信)が閉じると、欠落の印は外れる。印を外すのは、その Issue の作業。この領域では、印の番号の Issue が閉じたかを、報告に書く。
- 他の領域(#{{200}}〜#{{206}}・#{{208}}〜#{{212}})と、`coverage.json` の母数を共有するが、ファイルは共有しない。
