---
draft: "210"
title: "E2E の領域 visual: 窓の種類・テーマ・言語・DPR ごとの画素のゴールデン(ハッシュ)と、縁の鮮明さ・黒い領域 0 の、見た目のシナリオを足す"
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

この領域(visual)は、見た目の機能を、画素で確かめるシナリオにする。窓の種類・テーマ・言語・DPR ごとに、画素のハッシュ(ゴールデン)を作り、以後の退行を見る。最小のシナリオ数は 120 本(`tools/e2e/features/visual.min` と同じ)。目標は次の 2 つ。
1. 窓の種類ごと・テーマごと・言語ごと・DPR ごとに、画素の期待値(ゴールデン)を持つシナリオで覆うこと。
2. 画素の期待値が、退行を見分けられる許容誤差を持つこと。ゴールデンの作り方と、更新の手順を、報告に書くこと。

## 背景

- #{{199}}(E0)が作る物: `defineScenario`・`run.mjs --area`・`--list`・`--shard`・`--budget`・`lib/common.mjs` の `pixelHashRegion`・`noBlackRegions`・見本 3 本(`tools/e2e/scenarios/visual/`)。E0 が閉じてから始める。
- ゴールデンは、この領域の中に置く: `tools/e2e/golden/visual/` の下に、窓の種類・テーマ・言語・DPR の組ごとの、ハッシュの表(TSV)を置く。初回の実行で作り、以後は比べる。更新は、報告に理由を書いた時だけ。
- 既存の道具: `tools/p2-frames/measure.mjs`(描画の計測)・`pm-work/findings/p2-render-1351.md`(描画の基準)・`tools/cursor/`(カーソルの画像)。
- 画素のハッシュは、SHA-256 の 16 進を使う。ハッシュの値は、報告に書いてよいが、ファイル(TSV)では、ゴールデンの表のキーと値だけを書く。

## 読む物

- `tools/e2e/`(E0 の後の姿。lib/common.mjs の `pixelHashRegion`・`noBlackRegions`)
- `tools/p2-frames/measure.mjs`・`pm-work/findings/p2-render-1351.md`・`pm-work/findings/p2-measure-1308.md`
- `tools/cursor/measure.mjs`・`tools/cursor/resource-cursor.mjs`
- `tools/e2e/features/generated/visual.tsv` と `windows.tsv`・`resources.tsv`(E0 が作る一覧。窓の種類・テーマ・言語の一覧)
- `pm-work/findings/e2e-program.md`

## やること

1. **機能の表** `tools/e2e/features/visual.tsv`(手で書く。見出し行は E0 が作る)。機能ごとに 1 行。列は `id`・`kind`・`name`・`expected`・`how`(ゴールデンの名前か、判定の式)。
2. **シナリオ**(`tools/e2e/scenarios/visual/`。1 本 1 ファイル)。次の全てを、機能ごとに 1 本以上。
   - **窓の種類ごとの画素のゴールデン**: 窓の一覧(`generated/windows.tsv`)の窓を、1 つずつ開き、その窓の矩形の画素のハッシュを取る。ゴールデンと比べる(許容誤差: 画素の差の割合が基準の上限を超えない)。
   - **テーマごとの画素のゴールデン**: テーマ 13 の全て。メイン画面の矩形のハッシュ。
   - **言語ごとの画素のゴールデン**: 言語 7 の全て。上部バーと、設定の窓の矩形のハッシュ。
   - **DPR ごとの画素のゴールデン**: DPR 1・1.25・1.5・1.75・2・2.5・3 と、乱数の 2 点。上部バーの矩形のハッシュ(DPR の値を、キーに含める)。
   - **縁の鮮明さ**: 窓の縁を、画素の列で読む。縁の幅が、DPR に応じた値(物理の画素で 1 か 2 の幅)に在る。ぼやけない(縁の両側の画素の差が、0 か、閾値を超える)。
   - **黒い領域 0**: 描画の全体で、全ての画素が 0 の矩形の数が 0。窓が開いた後・テーマを変えた後・言語を変えた後の、それぞれで確かめる。
   - **文字の描画の健全性**: 上部バーの文字の領域の画素が、単色でない(文字が描かれている)。
   - **カーソルの画像の大きさ**: 窓の端・テキスト欄のカーソルの画像の大きさが、DPR に合う(`tools/cursor/` の方式)。
3. **ゴールデン**: `tools/e2e/golden/visual/` の TSV(`キー<TAB>ハッシュ<TAB>許容誤差`)。初回の実行で作る。更新は、報告に理由を書いた時だけ。ゴールデンの値が変わった時、その差(画素の割合)を報告に書く。
4. **判定**: 各シナリオは `ok` と、測った値(`results`: ハッシュ・画素の差の割合・黒い矩形の数・縁の幅)を返す。`notes` に、判定の根拠を 1 行で書く。
5. **表示**: 上の DPR の組ごとに 1 行。全件を回すと時間が長い物は、DPR 1 だけにしてよい(理由を `notes` に書く)。

## 受け入れ

```accept
npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false?0:1)' "$RUNNER_TEMP/boot.json" && node tools/smoke/welcome-flow.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/flow.jsonl" && node -e 'const fs=require("fs");const L=fs.readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const F=(f)=>L.find((r)=>r.flow===f);const ok=(r)=>!!r&&r.error===null&&r.state==="engine-idle"&&r.standIn===false&&r.trapped===false&&r.hostErrors===0&&r.opened===true;process.exit(L.length===4&&["A","B","C","D"].every((f)=>ok(F(f)))&&F("B").welcomeClosed===true&&F("D").welcomeClosed===true?0:1)' "$RUNNER_TEMP/flow.jsonl" && npm test --prefix app && (cd tools/e2e && npm ci && npx playwright install chromium) && node tools/e2e/run.mjs --area visual --list > "$RUNNER_TEMP/list.jsonl" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8").trim().split("\n")[0]);process.exit(r.area==="visual"&&r.scenarios>=120?0:1)' "$RUNNER_TEMP/list.jsonl" && node tools/e2e/run.mjs --area visual --shard 1/8 --displays 4k-dpr1 --budget 40m > "$RUNNER_TEMP/e2e.jsonl" && node -e 'const L=require("fs").readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));process.exit(L.length===1&&L.every((r)=>r.error===null&&r.servedHeadersMatch===true&&r.scenarios.length>0&&r.scenarios.every((s)=>s.skipped!==true))?0:1)' "$RUNNER_TEMP/e2e.jsonl"
```

判定の基準:
- 受け入れの式が通る(流れ A〜D・`npm test --prefix app`・`--list` の visual の scenarios が 120 以上・1 シャード(1/8)の実行が終了コード 0。`skipped` が無い)。
- 窓の一覧の全ての窓、テーマ 13 の全て、言語 7 の全てに、ゴールデンがある(件数を報告に書く)。
- 黒い領域の数が、全てのシナリオで 0。
- ゴールデンの差の割合が、許容誤差の内に在る(初回の実行でゴールデンを作った物は、2 回目の実行で差が 0 か、許容誤差の内)。2 回目の実行の出力の行を、報告に付ける。
- 網羅率: 全シャードを合わせて、この領域の機能の 90% 以上が、`ok` かつ `expectFail` 無しのシナリオで覆われる(`judge-coverage.jq`。PM が判定する。報告に覆った数を書く)。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には「runner の値。この機械とは別枠」と書き、nproc と loadavg を添える。
- 「達成」「直った」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。出力の行を付ける。
- 報告に、(a) 追加したシナリオの ID と、各 `covers`、(b) ゴールデンの件数(窓・テーマ・言語・DPR の組ごと)、(c) 許容誤差の値と決め方、(d) 2 回目の実行の差の割合、(e) 印 `expectFail: 'new'` の一覧(シナリオの ID・症状・停止の理由・run の出力の行)、(f) 通らなかった物と理由。

## 制約

- 製品(`app/`・`lift/`)は変えない(読むだけ)。製品の欠陥は、シナリオに `expectFail: 'new'` を付けて記録する。直すのは PM が起票する別の Issue。
- 製品に、利用者向けの新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足さない。テストのための分岐を製品に入れない。
- 領域の作業で作るファイルは、`tools/e2e/scenarios/visual/`・`tools/e2e/features/visual.tsv`・`tools/e2e/golden/visual/`・`tools/e2e/lib/visual.mjs`(必要な時だけ)に限る。
- ゴールデンは、この機械で作らない。runner(Actions)で初回の実行の結果から作る(報告の run の出力を、ファイルに写す)。
- Python を使わない。OCR を使わない。
- ベンダー名・Cloudflare の識別子(アカウント・バケット・ホスト名・アクセス制御の値)を、ファイルに書かない。
- 期待値に、実行時の値(DPR・DPI・窓の位置)を定数で書かない。ゴールデンのキーには、DPR の値を含める。
- sleep・timeout のコマンド・nohup・末尾の `&` は使わない。待つ時は条件で待つ。一時ファイルは `.tmp-*` に置き、終わったら消す。
- 製品の作業は Actions で行う。この機械で製品のビルドや測定をしない。GitHub に、Issue・ラベル・コメントを書かない。

## 作業の進め方

- 開始の時刻を、最初に `date -u` で記録する。4 時間 30 分を過ぎたら、受け入れが通らなくても、作業ツリーを commit し、報告を書いて終える。
- ビルドは約 35 分。フォアグラウンドで直接、シェルツールの `timeout` 引数に 5400000 を渡して待つ。背景で起動して待たない。
- blend2d の取得(GitHub)が 403 で落ちたら一時的。受け入れのコマンドは `lift/out-app/b2d-src` を消してから作り直すので、そのまま再試行する。
- この領域は大きい。1 回で終わらない前提で、窓の種類(一覧の全て)を先に、テーマと言語を後に、DPR の組を最後に足す。再試行は前回の差分の上で続け、作業ツリーを消さない。
- 作業の最後に、受け入れが通らなくても、作業ツリーを commit する。

## 他のタスクとの関係

- blocked_by: #{{199}}(E0)。E0 が閉じるまで起動しない。
- 他の領域(#{{200}}〜#{{209}}・#{{211}}・#{{212}})と、`coverage.json` の母数を共有するが、ファイルは共有しない。
- 描画の修正の Issue が後で来た時は、その Issue に対応するゴールデンを、報告の値で更新する(更新の理由を書く)。
