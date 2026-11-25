---
draft: "203"
title: "E2E の領域 menus: File・Edit・Add・Patterns・View・Options・Tools・Help の全ての項目(全階層)の効果を、期待する結果つきのシナリオで足す"
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

この領域(menus)は、FL のメニューの全ての項目(全階層)の効果を、期待する結果つきのシナリオで確かめる。最小のシナリオ数は 400 本(`tools/e2e/features/menus.min` と同じ)。目標は次の 2 つ。
1. メニューの全ての項目を、1 つずつ、効果を確かめるシナリオで覆うこと。
2. 覆った項目の数と、覆えなかった項目の理由(名前が取れない・効果が無い・未実装など)を、報告に書くこと。

## 背景

- #{{199}}(E0)が作る物: 項目の一覧 `tools/e2e/features/generated/menus.tsv`(項目の名前は、ゲストのメモリの文字列から取った物。取れない物は `File > #9` の番号で、`nameSource` が `index`)・`menuPath`(名前で選ぶ。無ければ番号)・`run.mjs --area`・`--shard`・見本 3 本(`tools/e2e/scenarios/menus/`)。E0 が閉じてから始める。
- メニューの文字は画素からは読めない(OCR は禁止)。名前は、E0 の一覧(ゲストのメモリの文字列)から取る。名前が取れない項目は、番号で選ぶ。
- 項目の効果は、次の 4 つの内の 1 つ以上で確かめる: 窓が出る(窓の表のクラス)・ダイアログが出る(窓の表)・状態が変わる(ゲストのメモリの文字列・窓の表の題名)・サブメニューが開く(新しい最上位の窓)。画素の変化だけで判定しない。
- 既存の方式: `tools/feat-c6-popup-menus/measure.mjs`(メニューの操作)・`tools/crash-hunt/hunt.mjs`(項目の番号で選ぶ。停止だけを見る。残す)。

## 読む物

- `tools/e2e/`(E0 の後の姿。lib/guest.mjs のクリック・キー、lib/common.mjs の `menuPath`)
- `tools/feat-c6-popup-menus/measure.mjs`・`tools/crash-hunt/hunt.mjs`・`tools/crash-hunt/run.sh`
- `tools/e2e/features/generated/menus.tsv`(E0 が作る一覧)
- `pm-work/findings/survey-c6-topbar-menus.md`(メニューの調査)・`pm-work/findings/hidpi-fl-scaling.md` の節 10(ゲストのメモリの文字列)
- `pm-work/findings/e2e-program.md`

## やること

1. **機能の表** `tools/e2e/features/menus.tsv`(手で書く。見出し行は E0 が作る)。項目ごとに 1 行。列は `id`・`kind`・`name`・`expected`・`how`。項目の ID は、E0 の一覧の ID と同じにする。
2. **シナリオ**(`tools/e2e/scenarios/menus/`。1 本 1 ファイル)。次の全てを、項目ごとに 1 本以上。
   - **File・Edit・Add・Patterns・View・Options・Tools・Help の全ての項目(全階層)**。サブメニューを開いた項目は、その中の項目も 1 本ずつ。項目を選んだ後の効果を、上の 4 つの見方の内で確かめる。
   - ダイアログの OK・Cancel・Esc: ダイアログが出る項目は、3 通り(OK で閉じる・Cancel で閉じる・Esc で閉じる)の結果を確かめる。閉じた後に、窓の表から消えている。
   - ショートカットと、メニューの項目が同じ効果: 割り当てのある項目は、ショートカット(キー)とメニューの項目の、2 つの経路の結果を比べる。
   - 項目が無効(灰色)の時は、選んでも効果が無い。無効の状態は、窓の表と、メモリの文字列の表示で確かめる(取れない時は `notes` に書く)。
3. **判定**: 各シナリオは `ok` と、測った値(`results`: 出た窓のクラス・題名、状態の文字、サブメニューの項目数)を返す。`notes` に、判定の根拠を 1 行で書く。「選べた」だけで終わらない。
4. **表示**: 既定は DPR 1 と、1 超 2 未満の乱数の 1 点(`displays` は `['4k-dpr1', '4k-random']`)。項目の数が多いので、重い物は DPR 1 だけにしてよい(理由を `notes` に書く)。
5. **名前が取れない項目**: 番号(`File > #9` の形)で選び、`notes` に「名前は取れていない(nameSource: index)」と書く。名前の取得を、この領域では直さない(E0 の一覧の作り方の問題として報告する)。

## 受け入れ

```accept
npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false?0:1)' "$RUNNER_TEMP/boot.json" && node tools/smoke/welcome-flow.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/flow.jsonl" && node -e 'const fs=require("fs");const L=fs.readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const F=(f)=>L.find((r)=>r.flow===f);const ok=(r)=>!!r&&r.error===null&&r.state==="engine-idle"&&r.standIn===false&&r.trapped===false&&r.hostErrors===0&&r.opened===true;process.exit(L.length===4&&["A","B","C","D"].every((f)=>ok(F(f)))&&F("B").welcomeClosed===true&&F("D").welcomeClosed===true?0:1)' "$RUNNER_TEMP/flow.jsonl" && npm test --prefix app && (cd tools/e2e && npm ci && npx playwright install chromium) && node tools/e2e/run.mjs --area menus --list > "$RUNNER_TEMP/list.jsonl" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8").trim().split("\n")[0]);process.exit(r.area==="menus"&&r.scenarios>=400?0:1)' "$RUNNER_TEMP/list.jsonl" && node tools/e2e/run.mjs --area menus --shard 1/8 --displays 4k-dpr1 --budget 40m > "$RUNNER_TEMP/e2e.jsonl" && node -e 'const L=require("fs").readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));process.exit(L.length===1&&L.every((r)=>r.error===null&&r.servedHeadersMatch===true&&r.scenarios.length>0&&r.scenarios.every((s)=>s.skipped!==true))?0:1)' "$RUNNER_TEMP/e2e.jsonl"
```

判定の基準:
- 受け入れの式が通る(流れ A〜D・`npm test --prefix app`・`--list` の menus の scenarios が 400 以上・1 シャード(1/8)の実行が終了コード 0。`skipped` が無い)。
- 一覧の全ての項目(`generated/menus.tsv` の行)に、1 本以上のシナリオがある。覆えなかった項目は、理由(無効・名前が無い・効果が無い)を報告に書く。
- 追加したシナリオの全てが、`covers` に項目の ID を持ち、その ID が一覧に在る。
- ショートカットを持つ項目は、ショートカットとメニューの両方の経路を、それぞれ 1 本以上で確かめる。
- 網羅率: 全シャードを合わせて、この領域の機能の 90% 以上が、`ok` かつ `expectFail` 無しのシナリオで覆われる(`judge-coverage.jq`。PM が判定する。報告に覆った数を書く)。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には「runner の値。この機械とは別枠」と書き、nproc と loadavg を添える。
- 「達成」「直った」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。出力の行を付ける。
- 報告に、(a) 追加したシナリオの数と、覆った項目の数、(b) 名前が取れなかった項目の数と、その一覧、(c) 印 `expectFail: 'new'` の一覧(シナリオの ID・症状・停止の理由・run の出力の行)、(d) 通らなかった物と理由。

## 制約

- 製品(`app/`・`lift/`)は変えない(読むだけ)。製品の欠陥は、シナリオに `expectFail: 'new'` を付けて記録する。直すのは PM が起票する別の Issue。
- 製品に、利用者向けの新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足さない。テストのための分岐を製品に入れない。
- 領域の作業で作るファイルは、`tools/e2e/scenarios/menus/`・`tools/e2e/features/menus.tsv`・`tools/e2e/lib/menus.mjs`(必要な時だけ)に限る。共有の部品の変更は、最小の差分にとどめ、変えた理由を報告に書く。
- Python を使わない。OCR を使わない。メニューの文字は、ゲストのメモリの文字列(一覧の名前)・窓の表・番号で扱う。
- ベンダー名・Cloudflare の識別子(アカウント・バケット・ホスト名・アクセス制御の値)を、ファイルに書かない。
- 期待値に、実行時の値(DPR・DPI・窓の位置)を定数で書かない。
- sleep・timeout のコマンド・nohup・末尾の `&` は使わない。待つ時は条件で待つ。一時ファイルは `.tmp-*` に置き、終わったら消す。
- 製品の作業は Actions で行う。この機械で製品のビルドや測定をしない。GitHub に、Issue・ラベル・コメントを書かない。

## 作業の進め方

- 開始の時刻を、最初に `date -u` で記録する。4 時間 30 分を過ぎたら、受け入れが通らなくても、作業ツリーを commit し、報告を書いて終える。
- ビルドは約 35 分。フォアグラウンドで直接、シェルツールの `timeout` 引数に 5400000 を渡して待つ。背景で起動して待たない。
- blend2d の取得(GitHub)が 403 で落ちたら一時的。受け入れのコマンドは `lift/out-app/b2d-src` を消してから作り直すので、そのまま再試行する。
- この領域は大きい。1 回で終わらない前提で、価値の高い順に足す(File と Edit を先に、Help を後に)。再試行は前回の差分の上で続け、作業ツリーを消さない。
- 作業の最後に、受け入れが通らなくても、作業ツリーを commit する。

## 他のタスクとの関係

- blocked_by: #{{199}}(E0)。E0 が閉じるまで起動しない。
- 他の領域(#{{200}}〜#{{202}}・#{{204}}〜#{{212}})と、`coverage.json` の母数を共有するが、ファイルは共有しない。
- 項目の名前が取れない問題は、E0 の一覧の作り方の問題。この領域では直さず、報告に書く。
