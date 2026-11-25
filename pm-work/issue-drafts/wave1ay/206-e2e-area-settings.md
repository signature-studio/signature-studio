---
draft: "206"
title: "E2E の領域 settings: F10 の設定の全 16 頁、言語 7、テーマ 13 の、効果と再読み込みをまたぐ保持のシナリオを機能ごとに足す"
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

この領域(settings)は、設定の窓(F10)の全ての頁と選択欄、言語 7 の全て、テーマ 13 の全てを、期待する結果つきのシナリオで確かめる。最小のシナリオ数は 150 本(`tools/e2e/features/settings.min` と同じ)。目標は次の 2 つ。
1. 下の一覧の機能を、1 つずつ、効果(値・文字・画素の変化)と保持で確かめるシナリオで覆うこと。
2. 覆った機能の数と、覆えなかった機能の理由を、報告に書くこと。

## 背景

- #{{199}}(E0)が作る物: `defineScenario`・`run.mjs --area`・`--list`・`--shard`・`--budget`・設定の一覧 `tools/e2e/features/generated/settings.tsv`(F10 の 16 頁と、各頁の選択欄)・`resources.tsv`(言語 7・テーマ 13)・`lib/common.mjs` の `persist`・見本 3 本(`tools/e2e/scenarios/settings/`)。E0 が閉じてから始める。
- 設定の値の保持は、再読み込みの後に、同じ値で出ること(`persist`)。言語とテーマは、変えた後に、UI の文字と画素が変わり、engine が止まらないこと。
- 既存の方式: #1358(F10 の General settings。`pm-work/findings/hidpi-fl-scaling.md` の節 10: ゲストのメモリから Debug ログ・係数を読めた)・`tools/settings-scroll/measure.mjs`(設定の窓のスクロール。`pm-work/findings/settings-scroll-1384.md`)。

## 読む物

- `tools/e2e/`(E0 の後の姿。lib/guest.mjs・lib/common.mjs の `persist`)
- `tools/settings-scroll/measure.mjs`・`pm-work/findings/settings-scroll-1384.md`・`pm-work/findings/settings-scroll-1384-dpr1-p50-*.png`(画面の例)
- `pm-work/findings/hidpi-fl-scaling.md`(節 10。F10 の値の読み方)・`pm-work/findings/switch-tables.md`(言語とテーマの切り替え)
- `tools/persist/measure.mjs`・`pm-work/findings/persist-registry.md`(保持の方式)
- `inputs/fl-installer-files.tsv`(`Languages\` と `Artwork\Themes\` の行)
- `tools/e2e/features/generated/settings.tsv`・`resources.tsv`(E0 が作る一覧)・`pm-work/findings/e2e-program.md`

## やること

1. **機能の表** `tools/e2e/features/settings.tsv`(手で書く。見出し行は E0 が作る)。機能ごとに 1 行。列は `id`・`kind`・`name`・`expected`・`how`。
2. **シナリオ**(`tools/e2e/scenarios/settings/`。1 本 1 ファイル)。次の全てを、機能ごとに 1 本以上。
   - **F10 の Settings の全 16 頁**: 頁ごとに、全ての選択欄・チェック・ドロップダウン・スライダー・ボタンを 1 つずつ操作し、効果を確かめる(値が変わる・窓の中の文字が変わる・画素が変わる)。各項目の後、再読み込みをまたいで、値が残ること(`persist`)。
   - **言語 7 の全て**: 言語を切り替える。UI の文字が変わる(画素の差・ゲストのメモリの文字列)。再読み込みの後も、選んだ言語のまま。engine が止まらない。
   - **テーマ 13 の全て**: テーマを切り替える。画素が変わる(テーマごとに、窓の背景の色が期待する範囲に入る)。再読み込みの後も保持。engine が止まらない。
   - **拡大率(UI のスケール)の選択**: 選んだ値の効果(窓の大きさ・文字の大きさが変わる。DPR から計算した期待値と比べる)。
   - **自動保存**: 間隔の設定の効果(保存の要求が、設定した間隔で出る)。
   - **バックアップ**: 有効・無効の効果(バックアップの要求が出る・出ない)。
   - **ファイルのパス**(データ・プロジェクトの既定の場所): 値を変えた後、そのパスに保存される(保存の要求の名前とパスで確かめる)。
   - **MIDI の設定**(入力と出力の選択): 選択の保持。
   - **音声の設定**(出力のデバイス・バッファ): 選択の保持。
   - **Debug と Info**: 窓の文字が、ゲストのメモリの文字列として読める(#1358 の方式)。
3. **判定**: 各シナリオは `ok` と、測った値(`results`: 選択された値・文字・画素の差・保持の結果)を返す。`notes` に、判定の根拠を 1 行で書く。「開いて閉じた」だけで終わらない。
4. **表示**: 既定は `['4k-dpr1', '4k-random']`。言語 7 と テーマ 13 の全件を、DPR 1 だけで回すのは可(理由を `notes` に書く)。拡大率の物は、乱数の DPR を含める。

## 受け入れ

```accept
npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false?0:1)' "$RUNNER_TEMP/boot.json" && node tools/smoke/welcome-flow.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/flow.jsonl" && node -e 'const fs=require("fs");const L=fs.readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const F=(f)=>L.find((r)=>r.flow===f);const ok=(r)=>!!r&&r.error===null&&r.state==="engine-idle"&&r.standIn===false&&r.trapped===false&&r.hostErrors===0&&r.opened===true;process.exit(L.length===4&&["A","B","C","D"].every((f)=>ok(F(f)))&&F("B").welcomeClosed===true&&F("D").welcomeClosed===true?0:1)' "$RUNNER_TEMP/flow.jsonl" && npm test --prefix app && (cd tools/e2e && npm ci && npx playwright install chromium) && node tools/e2e/run.mjs --area settings --list > "$RUNNER_TEMP/list.jsonl" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8").trim().split("\n")[0]);process.exit(r.area==="settings"&&r.scenarios>=150?0:1)' "$RUNNER_TEMP/list.jsonl" && node tools/e2e/run.mjs --area settings --shard 1/8 --displays 4k-dpr1 --budget 40m > "$RUNNER_TEMP/e2e.jsonl" && node -e 'const L=require("fs").readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));process.exit(L.length===1&&L.every((r)=>r.error===null&&r.servedHeadersMatch===true&&r.scenarios.length>0&&r.scenarios.every((s)=>s.skipped!==true))?0:1)' "$RUNNER_TEMP/e2e.jsonl"
```

判定の基準:
- 受け入れの式が通る(流れ A〜D・`npm test --prefix app`・`--list` の settings の scenarios が 150 以上・1 シャード(1/8)の実行が終了コード 0。`skipped` が無い)。
- 設定の 16 頁の全ての頁に、1 本以上のシナリオがある。言語 7・テーマ 13 は、全件に 1 本以上ある(数を報告に書く)。
- 追加したシナリオの全てが、`covers` に機能の ID を持ち、その ID が一覧か `features/settings.tsv` に在る。
- 保持(再読み込みの後に同じ値)は、保持を確かめるシナリオの全てで、値が一致する。
- 網羅率: 全シャードを合わせて、この領域の機能の 90% 以上が、`ok` かつ `expectFail` 無しのシナリオで覆われる(`judge-coverage.jq`。PM が判定する。報告に覆った数を書く)。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には「runner の値。この機械とは別枠」と書き、nproc と loadavg を添える。
- 「達成」「直った」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。出力の行を付ける。
- 報告に、(a) 追加したシナリオの ID と、各 `covers`、(b) 印 `expectFail: 'new'` の一覧(シナリオの ID・症状・停止の理由・run の出力の行)、(c) 通らなかった物と理由、(d) 覆った機能の数、(e) 言語とテーマの件数(一覧と照らした値)。

## 制約

- 製品(`app/`・`lift/`)は変えない(読むだけ)。製品の欠陥は、シナリオに `expectFail: 'new'` を付けて記録する。直すのは PM が起票する別の Issue。
- 製品に、利用者向けの新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足さない。テストのための分岐を製品に入れない。
- 領域の作業で作るファイルは、`tools/e2e/scenarios/settings/`・`tools/e2e/features/settings.tsv`・`tools/e2e/lib/settings.mjs`(必要な時だけ)に限る。共有の部品の変更は、最小の差分にとどめ、変えた理由を報告に書く。
- Python を使わない。OCR を使わない。文字は、ゲストのメモリの文字列・窓の表・画素のハッシュで読む。
- ベンダー名・Cloudflare の識別子(アカウント・バケット・ホスト名・アクセス制御の値)を、ファイルに書かない。
- 期待値に、実行時の値(DPR・DPI・窓の位置)を定数で書かない。
- sleep・timeout のコマンド・nohup・末尾の `&` は使わない。待つ時は条件で待つ。一時ファイルは `.tmp-*` に置き、終わったら消す。
- 製品の作業は Actions で行う。この機械で製品のビルドや測定をしない。GitHub に、Issue・ラベル・コメントを書かない。

## 作業の進め方

- 開始の時刻を、最初に `date -u` で記録する。4 時間 30 分を過ぎたら、受け入れが通らなくても、作業ツリーを commit し、報告を書いて終える。
- ビルドは約 35 分。フォアグラウンドで直接、シェルツールの `timeout` 引数に 5400000 を渡して待つ。背景で起動して待たない。
- blend2d の取得(GitHub)が 403 で落ちたら一時的。受け入れのコマンドは `lift/out-app/b2d-src` を消してから作り直すので、そのまま再試行する。
- この領域は大きい。1 回で終わらない前提で、価値の高い順に足す(言語とテーマの切り替えと保持を先に、Debug と Info を後に)。再試行は前回の差分の上で続け、作業ツリーを消さない。
- 作業の最後に、受け入れが通らなくても、作業ツリーを commit する。

## 他のタスクとの関係

- blocked_by: #{{199}}(E0)。E0 が閉じるまで起動しない。
- 他の領域(#{{200}}〜#{{205}}・#{{207}}〜#{{212}})と、`coverage.json` の母数を共有するが、ファイルは共有しない。
- F10 の窓が開かない・設定が保持されない、の修正の Issue が後で来た時は、その Issue に対応するシナリオを 1 本以上足す。
