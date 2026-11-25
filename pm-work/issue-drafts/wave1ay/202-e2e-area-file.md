---
draft: "202"
title: "E2E の領域 file: New・テンプレート・Open・Save・Export・Import・demo の曲・ドラッグ&ドロップ・再読み込みをまたぐ保持の、期待する結果のシナリオを機能ごとに足す"
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

この領域(file)は、ファイルとプロジェクトの機能を、期待する結果つきのシナリオで確かめる。最小のシナリオ数は 100 本(`tools/e2e/features/file.min` と同じ)。目標は次の 2 つ。
1. 下の一覧の機能を、1 つずつ、期待する結果を持つシナリオで覆うこと。
2. 覆った機能の数と、覆えなかった機能の理由を、報告に書くこと。

## 背景

- #{{199}}(E0)が作る物: `defineScenario`・`run.mjs --area`・`--list`・`--shard`・`--budget`・機能の一覧(`tools/e2e/features/generated/file.tsv`: テンプレート・demo の一覧)・`coverage.mjs`・見本 3 本(`tools/e2e/scenarios/file/`)・`lib/common.mjs`(`menuPath`・`persist`)。E0 が閉じてから始める。
- ファイルの選択(Open・Import)は、ページのファイル選択(アップロード)を使う。ファイルは、リポジトリの `inputs/` の物を使う(`.tmp-*` へ写す場合は、終わったら消す)。
- 保持は、#1385(Recent・Presets・Projects などの FL のユーザーのファイル)と #1409(Recent の書き先)の作業の結果に依る。この領域は、確かめるだけ。
- Export の出力は、非空と、長さの妥当さ(音声の長さ。#{{205}} が音の中身を見る)で確かめる。

## 読む物

- `tools/e2e/`(E0 の後の姿)
- `tools/persist/measure.mjs`・`pm-work/findings/persist-files-1385.md`・`pm-work/findings/persist-registry.md`(保持の方式)
- `tools/e2e/features/generated/file.tsv`(テンプレートと demo の一覧。E0 が作る)
- `inputs/fl-installer-files.tsv`(テンプレート `Data\Templates` と demo の行)
- `pm-work/findings/demo-assets-inventory.tsv`・`pm-work/findings/p3-demo-scan-1401.md`(demo の曲の状態)
- `pm-work/findings/e2e-program.md`

## やること

1. **機能の表** `tools/e2e/features/file.tsv`(手で書く。見出し行は E0 が作る)。機能ごとに 1 行。列は `id`・`kind`・`name`・`expected`・`how`。
2. **シナリオ**(`tools/e2e/scenarios/file/`。1 本 1 ファイル)。次の全てを、機能ごとに 1 本以上。
   - New(新しいプロジェクト。空の曲。窓の表の変化で確かめる)。
   - New from template(全テンプレートを 1 本ずつ。`Data\Templates` の全て。開いた後、プロジェクトの題名が変わり、engine が止まらない)。
   - Open: ファイル選択でのアップロード、複数のファイルを選ぶ、フォルダを選ぶ(フォルダの読み込みの扱いを、窓の表と engine のログで確かめる)。
   - Open recent: 直前に開いた物が、一覧に出て、開ける。
   - Save・Save as・Save new version: 保存の結果が、ダウンロードの要求(ファイルの名前と大きさ)で確かめられる。
   - Export: 各形式(wav・mp3・ogg・flac・MIDI など、FL が出す形式の全て)。出力が非空で、長さが妥当(`audioProbe` の結果と照らす)。
   - Import: 各形式の取り込み(MIDI・音声・他のプロジェクト)。
   - Project info・Project data files: 窓が開き、内容が出る。
   - demo の曲を全て開く(`inputs/` の demo の全て。1 本ずつ。開けて、engine が止まらず、窓の表に曲の題名が出る)。
   - ドラッグ&ドロップ: ファイルを窓へ落とす(落とした物が開く。ドロップの扱いは、窓の表と engine のログで確かめる)。
   - 再読み込みをまたぐ保持: Recent・Presets・Projects など(#1385・#1409)。保存した物が、再読み込みの後も残る(`persist(page, guest)` を使う)。
3. **判定**: 各シナリオは `ok` と、測った値(`results`)を返す。`notes` に、判定の根拠(ダウンロードの大きさ・窓の表の行・engine のログの行)を 1 行で書く。「開いた」だけで終わらない。
4. **表示**: 既定は `['4k-dpr1', '4k-random']`。demo を全て開く物は、DPR 1 だけにしてよい(理由を `notes` に書く)。

## 受け入れ

```accept
npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false?0:1)' "$RUNNER_TEMP/boot.json" && node tools/smoke/welcome-flow.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/flow.jsonl" && node -e 'const fs=require("fs");const L=fs.readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const F=(f)=>L.find((r)=>r.flow===f);const ok=(r)=>!!r&&r.error===null&&r.state==="engine-idle"&&r.standIn===false&&r.trapped===false&&r.hostErrors===0&&r.opened===true;process.exit(L.length===4&&["A","B","C","D"].every((f)=>ok(F(f)))&&F("B").welcomeClosed===true&&F("D").welcomeClosed===true?0:1)' "$RUNNER_TEMP/flow.jsonl" && npm test --prefix app && (cd tools/e2e && npm ci && npx playwright install chromium) && node tools/e2e/run.mjs --area file --list > "$RUNNER_TEMP/list.jsonl" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8").trim().split("\n")[0]);process.exit(r.area==="file"&&r.scenarios>=100?0:1)' "$RUNNER_TEMP/list.jsonl" && node tools/e2e/run.mjs --area file --shard 1/8 --displays 4k-dpr1 --budget 40m > "$RUNNER_TEMP/e2e.jsonl" && node -e 'const L=require("fs").readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));process.exit(L.length===1&&L.every((r)=>r.error===null&&r.servedHeadersMatch===true&&r.scenarios.length>0&&r.scenarios.every((s)=>s.skipped!==true))?0:1)' "$RUNNER_TEMP/e2e.jsonl"
```

判定の基準:
- 受け入れの式が通る(流れ A〜D・`npm test --prefix app`・`--list` の file の scenarios が 100 以上・1 シャード(1/8)の実行が終了コード 0。`skipped` が無い)。
- 追加したシナリオの全てが、`covers` に機能の ID を持ち、その ID が `tools/e2e/features/file.tsv` か `generated/file.tsv` に在る。
- テンプレートと demo は、一覧の全ての行に 1 本以上のシナリオがある(数の一致を報告に書く)。
- 網羅率: 全シャードを合わせて、この領域の機能の 90% 以上が、`ok` かつ `expectFail` 無しのシナリオで覆われる(`judge-coverage.jq`。PM が判定する。報告に覆った数を書く)。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には「runner の値。この機械とは別枠」と書き、nproc と loadavg を添える。
- 「達成」「直った」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。出力の行を付ける。
- 報告に、(a) 追加したシナリオの ID と、各 `covers`、(b) 印 `expectFail: 'new'` の一覧(シナリオの ID・症状・停止の理由・run の出力の行)、(c) 通らなかった物と理由、(d) 覆った機能の数、(e) テンプレートと demo の数(一覧と照らした値)。

## 制約

- 製品(`app/`・`lift/`)は変えない(読むだけ)。製品の欠陥は、シナリオに `expectFail: 'new'` を付けて記録する。直すのは PM が起票する別の Issue。
- 製品に、利用者向けの新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足さない。テストのための分岐を製品に入れない。
- 領域の作業で作るファイルは、`tools/e2e/scenarios/file/`・`tools/e2e/features/file.tsv`・`tools/e2e/lib/file.mjs`(必要な時だけ)に限る。共有の部品の変更は、最小の差分にとどめ、変えた理由を報告に書く。
- Python を使わない。OCR を使わない。文字は、ゲストのメモリの文字列・窓の表・画素のハッシュで読む。
- ベンダー名・Cloudflare の識別子(アカウント・バケット・ホスト名・アクセス制御の値)を、ファイルに書かない。
- 期待値に、実行時の値(DPR・DPI・窓の位置)を定数で書かない。
- sleep・timeout のコマンド・nohup・末尾の `&` は使わない。待つ時は条件で待つ。一時ファイルは `.tmp-*` に置き、終わったら消す。
- 製品の作業は Actions で行う。この機械で製品のビルドや測定をしない。GitHub に、Issue・ラベル・コメントを書かない。

## 作業の進め方

- 開始の時刻を、最初に `date -u` で記録する。4 時間 30 分を過ぎたら、受け入れが通らなくても、作業ツリーを commit し、報告を書いて終える。
- ビルドは約 35 分。フォアグラウンドで直接、シェルツールの `timeout` 引数に 5400000 を渡して待つ。背景で起動して待たない。
- blend2d の取得(GitHub)が 403 で落ちたら一時的。受け入れのコマンドは `lift/out-app/b2d-src` を消してから作り直すので、そのまま再試行する。
- この領域は大きい。1 回で終わらない前提で、価値の高い順に足す(New・Open・Save・保持を先に、Export の形式と demo の全件を後に)。再試行は前回の差分の上で続け、作業ツリーを消さない。
- 作業の最後に、受け入れが通らなくても、作業ツリーを commit する。

## 他のタスクとの関係

- blocked_by: #{{199}}(E0)。E0 が閉じるまで起動しない。
- 他の領域(#{{200}}〜#{{201}}・#{{203}}〜#{{212}})と、`coverage.json` の母数を共有するが、ファイルは共有しない。
- 保持(#1385・#1409)の修正の Issue が後で来た時は、その Issue に対応するシナリオを 1 本以上足す。
