---
draft: "238"
title: "`tools/plugin-run` の場面の選び方を直す: 名前の部分一致で別の曲を選ぶ・名前を持つ曲が無い(場面 (a) を足す)"
type: "Task"
milestone: "M5 内蔵のエフェクトとインストゥルメント"
parent: "021"
blocked_by: []
labels: ["plugin-builtin"]
size: "L"
priority: "High"
project: "signature-studio/3"
---

## 目的

`tools/plugin-run/run.mjs` は、内蔵のエフェクトの DLL を、曲(.flp)の場面で guest に要求させ、動くかを確かめる道具。今の場面 (c)(名前を含む .flp を開く)は、曲の選び方が悪いため、DLL は動くのに「guest がこの plug-in を要求しなかった」か「名前を持つ曲が無い」で失敗する。この Issue は、場面 (c) の選び方を直し、場面 (a)(ミキサーの FX スロットに名前でプラグインを足す)を道具に加える。

この Issue の目標は 2 つ。
1. 場面 (c) は、名前の部分一致で別の曲を選ばない。候補を順に試し、guest が要求した最初の曲で止まる。
2. 場面 (a) で、Fruity Balance と Fruity Filter がミキサーの挿入から `instantiated`・`processed` になる(場面 (a) の操作が、この build で通ることを確かめる)。

## 背景

- 実測(判定の実行の runner の値。この機械とは別枠):
  - Fruity Delay: 場面 (c) は `Data/Templates/Minimal/Delay and reverb/Delay and reverb.flp` を選んだ。この曲は Fruity Delay Bank を読み、Fruity Delay は要求されず、`the guest did not ask for Fruity Delay`(`instantiated` が false)。同じ道具で、場面の曲だけを `Product demos/Electric Loop - The Ogun Dream.flp` に差し替えると、Fruity Delay は `instantiated: true`・`processed: true` になった。DLL は動き、場面の選び方が悪い。Fast LP と Parametric EQ も、同じ型で失敗した(`scene error`)。
  - Fruity Big Clock・Fruity Center・Fruity LSD: `scene (c) for Fruity Big Clock: no .flp names it (UTF-16LE)`。名前を含む曲が 1 つも無い。
- 道具の場面 (c): `tools/plugin-run/run.mjs` の `projectFor`(111〜120 行)は、名前の UTF-16LE を部分文字列として含む最小の .flp を、根の順に最初に見つかった根から 1 本だけ選ぶ。名前の境界を見ず、外れた時に別の曲を試さない。
- 名前の境界: 名前の直後が英数字の曲、または空白 1 字の後ろが英数字の曲(`Fruity Delay Bank`・`Fruity Delay 2`)は、その名前の曲ではない。
- 場面 (a): 道具には無い。E2E の `tools/e2e/lib/plugins.mjs` の `pluginSteps`(Mixer の挿入 1 のスロット 1 から picker を開き、Plugin database の項目を選ぶ)が、同じ操作を持つ。その注釈は、Plugin database の項目が無い場合と、項目を挿す操作を「この build では未確認」と書いている。この Issue で、場面 (a) が通るかを確かめる。

## 読む物

- `tools/plugin-run/run.mjs`(`effectRows`、`projectFor`、`runScene`、`measureOnce`、`main`)
- `spec/plugin-run.test.mjs`(既存の試験。8) の `projectFor` の試験は、新しい規則でも通る)
- `tools/e2e/lib/plugins.mjs`(`pluginSteps`、`openSlotPicker`、`openPluginDatabase`、`databaseHas`)・`tools/e2e/lib/common.mjs`
- `lift/plugins/*/plugin.tsv`(guest の列。フォルダ名は guest の後ろから 2 番目)
- `pm-work/issue-drafts/wave1o/127-plugin-run-tool.md`(道具の最初の設計)

## やること

0. 作業の最初に `date -u` を記録する。`.prior-work.md` が在れば最初に読む。`git status` で作業ツリーを確かめる。
1. 試験を先に足して、赤を確かめる(`spec/plugin-run.test.mjs` に (e) の節を足す。Chrome は使わない)。
   - 部分一致の除外: `Fruity Delay` の候補から、`Fruity Delay Bank` だけを含む曲と、`Fruity Delay 2` だけを含む曲を除く。`Fruity Delay` だけを含む曲は候補に残す。
   - 候補の順序: 根の順(host-extra が先)、同じ根では小さい順。最大 5 本。
   - 判定の式: 1 本の結果の行が通るかを、`run.mjs` が export する関数で、合成データ 6 件で確かめる。通す 2 件: `sceneKind` が `c` で、`instantiated`・`processed` が true、`error` が null / `sceneKind` が `a` で、同じ条件。落とす 4 件: `instantiated` が false / `processed` が false / `error` が文字列 / `sceneKind` が null。
2. 場面 (c) の選び方を直す(`tools/plugin-run/run.mjs`)。
   - 候補は、UTF-16LE の名前を含み、名前の前後が境界である .flp の一覧。境界: 名前の直前と直後の 1 字が英数字でないこと。直後が空白 1 字で、その次が英数字の時は境界でない。
   - 候補を根の順・小さい順に並べ、最大 5 本を順に試す。1 本ごとに、今の `measureOnce` と同じく Chrome を起動して場面 (c) を行う。guest がその plug-in の wasm を `requested=1` で読んだ最初の曲で止める。要求されなければ次の曲へ進む。
   - 候補が 0 本の時は、今と同じく「名前を持つ曲が無い」の理由で止まる。
   - 既存の `projectFor` の試験(8)は、新しい規則でも通ることを確かめる。
3. 場面 (a) を足す(`tools/plugin-run/run.mjs`)。
   - Welcome を閉じ、Mixer を出し、Insert 1 のスロット 1 から picker を開き、Plugin database の項目(名前の一致)を選ぶ。項目が挿さると、窓が出る。その後、再生を今の `play()` と同じ形で測る。
   - 操作の部品は、`tools/e2e/lib/plugins.mjs` と `common.mjs` を読んで使ってもよいし、必要な分を `tools/plugin-run/` に写してもよい。どちらでも、E2E の試験の動きは変えない。
   - 場面 (a) の成否は、`instantiated`(wasm が `requested=1` で読まれたこと)で見る。
   - 挿せない時は、止まった段(Mixer・picker・Plugin database の項目・窓)を JSON の `error` に入れる。
4. 引数と出力を足す。
   - `--scene c|a|auto` を、名前の後ろに足す(道具の引数。製品の旗ではない)。既定は `auto`(場面 (c) を候補の順に試し、guest が要求しなければ場面 (a))。不明な値は終了コード 64(使い方)。
   - JSON の 1 行に、`sceneKind`(`c`・`a`。要求されなければ null)・`sceneFile`(場面 (c) の .flp の、リポジトリからの相対パス。場面 (a) と無い時は null)・`candidatesTried`(場面 (c) で試した曲の数。0〜5)を足す。既存の key は変えない。
   - 冒頭のコメントと usage も、新しい引数に合わせる。
5. 試験を通す。受け入れのコマンドで確かめる。

## 受け入れ

```accept
npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false&&r.elapsedMs<=20000?0:1)' "$RUNNER_TEMP/boot.json" && node tools/smoke/welcome-flow.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/flow.jsonl" && node -e 'const fs=require("fs");const L=fs.readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const F=(f)=>L.find((r)=>r.flow===f);const ok=(r)=>!!r&&r.error===null&&r.state==="engine-idle"&&r.standIn===false&&r.trapped===false&&r.hostErrors===0&&r.opened===true;process.exit(L.length===4&&["A","B","C","D"].every((f)=>ok(F(f)))&&F("B").welcomeClosed===true&&F("D").welcomeClosed===true?0:1)' "$RUNNER_TEMP/flow.jsonl" && npm test --prefix app && node tools/plugin-run/run.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" "Fruity Balance" "Fruity Filter" --scene a > "$RUNNER_TEMP/pr-a.jsonl" && node -e 'const L=require("fs").readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const ok=(r)=>r.error===null&&r.instantiated===true&&r.processed===true&&r.sceneKind==="a";process.exit(L.length===2&&["Fruity Balance","Fruity Filter"].every((p)=>L.some((r)=>r.plugin===p&&ok(r)))?0:1)' "$RUNNER_TEMP/pr-a.jsonl" && node tools/plugin-run/run.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" "Fruity Balance" "Fruity Filter" --scene auto > "$RUNNER_TEMP/pr-auto.jsonl" && node -e 'const L=require("fs").readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const ok=(r)=>r.error===null&&r.instantiated===true&&r.processed===true&&(r.sceneKind==="c"||r.sceneKind==="a");process.exit(L.length===2&&["Fruity Balance","Fruity Filter"].every((p)=>L.some((r)=>r.plugin===p&&ok(r)))?0:1)' "$RUNNER_TEMP/pr-auto.jsonl" && node spec/plugin-run.test.mjs && bash guard/scan.sh
```

判定の基準:
- 起動確認(`boot.json`)が engine-idle・standIn が false・elapsedMs が 20,000 以下。
- 流れ A〜D の 4 行が、`error` が null・engine-idle・`standIn` が false・`trapped` が false・`hostErrors` が 0・`opened` が true。B と D は `welcomeClosed` が true。
- `npm test --prefix app` が通る。
- `--scene a` の 2 行(Fruity Balance と Fruity Filter)が、`error` が null・`instantiated` と `processed` が true・`sceneKind` が `a`。
- `--scene auto` の 2 行が、`error` が null・`instantiated` と `processed` が true・`sceneKind` が `c` か `a`。
- `node spec/plugin-run.test.mjs` が終了コード 0。
- `bash guard/scan.sh` の指摘が 0 件。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には「runner の値。この機械とは別枠」と書く。
- 報告に書く: (a) 直す前の `spec/plugin-run.test.mjs` の赤の行、(b) `--scene a` の JSON の 2 行(`sceneKind`・`sceneFile`・`candidatesTried`・`instantiated`・`processed`)、(c) `--scene auto` の JSON の 2 行、(d) Fruity Delay の候補の一覧と、選んだ曲(部分一致の除外の結果)、(e) `node spec/plugin-run.test.mjs` の最後の行、(f) 起動確認の `elapsedMs` と、流れ A〜D の 4 行、(g) 場面 (a) が通らなかった時は、止まった段、(h) `bash guard/scan.sh` の結果。
- 「達成」「直った」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。出力の行を付ける。

## 制約

- 製品に、利用者向けの新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足さない。`--scene` は道具(`tools/plugin-run`)の引数で、製品の旗ではない。
- Python を使わない(調査と検証にだけ使う)。OCR を使わない。
- 禁止されているのは、AI のベンダー名と Cloudflare の識別子だけ。FL の製品名・変数名は、書いてよい。
- sleep・timeout のコマンド・nohup・末尾の `&` は使わない。一時ファイルは `.tmp-*` に置き、終わったら消す。

## 作業の進め方

- 作業の最初に `date -u` を記録する。4 時間 30 分を過ぎたら、受け入れが通らなくても、作業ツリーを commit し、報告を書いて終える(ジョブの上限は 5 時間 50 分)。
- ビルドは 1 回にして、同じ bundle で繰り返し確かめて直す。候補の曲は、1 本ずつ Chrome を起動する(1 本に数十秒)。長いコマンドはフォアグラウンドで直接、シェルツールの `timeout` 引数は最大 5400000(90 分)にする。背景で起動して待たない。
- 受け入れが通らなくても、作業の最後に作業ツリーを commit する。未 commit の変更が残ると、判定は失敗する。

## 他のタスクとの関係

- blocked_by: なし。
- 内蔵のエフェクトの残りの束(#1434〜#1448)の判定は、この道具の場面を使う。それらは、この Issue が閉じるまで起動しない。
- 主に変えるのは `tools/plugin-run/` と `spec/plugin-run.test.mjs`。
