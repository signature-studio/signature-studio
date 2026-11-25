---
draft: "198"
title: "Plugin Database が空で、Plugin Manager が起動しない: 内蔵プラグインを FL の一覧に出し、Plugin Manager を開けるようにする"
type: Task
milestone: "M4 FL に必要な物の配布"
parent: "012"
blocked_by: []
labels: ["delivery", "plugin-builtin", "verify"]
size: M
priority: High
project: signature-studio/3
---

## 目的

依頼者の指摘(2026-11-25。原文): 「もしかしてプラグインを追加しているのですか? Plugin Database が空で、Plugin Manager も起動しません。」

この Issue の目標は次の 3 つ。
1. Plugin Manager(FL の窓)が開き、開いた後も engine が止まらないこと。
2. Plugin Database(Browser の「Plugin database」)に、今の VM で読み込める内蔵プラグイン(`lift/plugins/*/plugin.tsv` の全て)が、一覧に出ること。
3. 内蔵プラグインを足す時は、`lift/plugins/<名前>/` を足すだけで、一覧に出ること(共有のファイルは変えない)。

## 背景

- 依頼者の指摘(上)への答え: 今の VM は、内蔵プラグインを 1 プラグイン 1 wasm で少しずつ足している。第 2 波の束が起票済みで、今は途中の姿。そのため Plugin Database の項目が少ない。
- ただし、Plugin Manager が起動しない(窓が出ない・engine が止まる)のは、別の欠陥として扱う。
- Plugin Database は、`Presets\Plugin database\Installed\` の `.nfo` を読む(`pm-work/findings/missing-paths.md`。`FindFirstFileW` の `…\Presets\Plugin database\Installed\*.*` が、場所が無いため 1 件も返らない)。FL は起動の時にプラグインの DLL を走査して `.nfo` を書く、と推定する(推定。確かめる)。
- 現在の `lift/plugins/` には `fruity-balance` と `sytrus` の 2 つの `plugin.tsv` が在る。`lift/harness/seeds/vfs-seed-installed.tsv` に、`Plugin database` の行が 1 つある。
- 道具: `tools/plugin-run`(1 本ずつ、起動 → 要求 → instantiate → 1 ブロックの処理まで走る道具)・`tools/crash-hunt/hunt.mjs`(FL の画面の経路)。
- Plugin Manager を開く経路(FL の Options メニューの項目など)は、窓の表と、`tools/crash-hunt/hunt.mjs` の経路で確かめる。

## 読む物

- `lift/plugins/*/plugin.tsv`・`lift/harness/seeds/vfs-seed-installed.tsv`(`Plugin database` の行)・`lift/harness/`(VFS の seed と、`lift/plugins/` の取り込み)
- `inputs/plugin-classes.tsv`・`inputs/plugin-hooks.tsv`
- `pm-work/findings/missing-paths.md`(`Plugin database` の要求)・`pm-work/findings/crash-hunt-2026-11-25.json`(`Plugin database` の missing-path の記録)
- `tools/plugin-run/`・`tools/crash-hunt/hunt.mjs`
- `lift/runtime/`(Plugin Manager の窓を作る呼び出し。`grep -rn 'Plugin' lift/runtime` で探す)

## やること

0. 作業の最初に `date -u` を記録する。`.prior-work.md` が在れば、最初に読む。
1. 再現: E2E の道具で、(a) FL の Plugin Manager を開く項目(`tools/crash-hunt/hunt.mjs` の経路で探す)、(b) Browser の「Plugin database」の頁を開く。窓が出るか、engine が止まるか、Plugin database の項目数を表にする(起動の時刻・止まった時の `fle:` の最後の行)。
2. 原因の特定: Plugin Manager の窓が出ない・止まる理由(止めている `fle_stop`・未実装の API・間接呼び出しの 0 など)を、trace と engine のログで特定する。
3. 直す:
   - Plugin Manager が開き、止まらない。
   - Plugin Database に、`lift/plugins/*/plugin.tsv` の全てのプラグインが一覧に出る。FL が走査する場所(`Presets\Plugin database\Installed\` の `.nfo`・`Plugins\Fruity\…` の DLL)に、見える形で置く。`.nfo` が無いなら、FL が書く形(走査の結果)で、engine の起動の中で作る。
   - `lift/plugins/<名前>/plugin.tsv` を足すだけで、一覧に出る。一覧は、`lift/plugins/` の中身から作る(プラグインの名前を、製品のコードに書き並べない)。共有のファイルの変更は、この仕組みの 1 箇所だけ。
4. E2E のシナリオ(`tools/e2e/scenarios/smoke/` に新規。`index.mjs` の並びに足す):
   - `plugin-manager-opens`: Plugin Manager の窓が出て、engine が `engine-idle` のまま 10 秒。
   - `plugin-database-lists-builtins`: Plugin database の項目数が、`lift/plugins/*/plugin.tsv` のプラグインの数以上。数は、シナリオの中で、`lift/plugins/` を読んで数える(数を書き込まない)。
5. 受け入れを通す。通らなければ、作業ツリーを commit して、報告を書いて終える。

## 受け入れ

```accept
npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false&&r.elapsedMs<=12500?0:1)' "$RUNNER_TEMP/boot.json" && node tools/smoke/welcome-flow.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/flow.jsonl" && node -e 'const fs=require("fs");const L=fs.readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const F=(f)=>L.find((r)=>r.flow===f);const ok=(r)=>!!r&&r.error===null&&r.state==="engine-idle"&&r.standIn===false&&r.trapped===false&&r.hostErrors===0&&r.opened===true;process.exit(L.length===4&&["A","B","C","D"].every((f)=>ok(F(f)))&&F("B").welcomeClosed===true&&F("D").welcomeClosed===true?0:1)' "$RUNNER_TEMP/flow.jsonl" && npm test --prefix app && (cd tools/e2e && npm ci && npx playwright install chromium) && node tools/e2e/run.mjs --scenarios smoke --displays 4k-dpr1,4k-dpr2 > "$RUNNER_TEMP/e2e.jsonl" && bash guard/scan.sh
```

判定の基準:
- 起動確認(`boot.json`)が engine-idle・standIn が false・elapsedMs が 12,500 以下。
- 流れ A〜D の 4 行が、`error` が null・engine-idle・`standIn` が false・`trapped` が false・`hostErrors` が 0・`opened` が true。B と D は `welcomeClosed` が true。
- `npm test --prefix app` が通る。
- `node tools/e2e/run.mjs --scenarios smoke --displays 4k-dpr1,4k-dpr2` が終了コード 0(`plugin-manager-opens`・`plugin-database-lists-builtins` を含む全シナリオが、全行で ok)。
- 報告に、Plugin database の一覧の項目(名前の一覧)と、`lift/plugins/` のプラグインの一覧を並べて書く。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には「runner の値。この機械とは別枠」と書き、nproc と loadavg を添える。
- 「達成」「直った」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。出力の行を付ける。
- 報告に書く: 再現の表(Plugin Manager の窓・engine の状態・項目数)、原因(実測と推定を分ける)、直した箇所、`.nfo` を作る方式の理由、流れ A〜D の 4 行。

## 制約

- 製品に、利用者向けの新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足さない。オプトインを作らない。
- 内蔵プラグインの名前の一覧を、製品のコードに書き並べない。`lift/plugins/` の中身から作る。
- Python を使わない(調査と検証にだけ使う)。OCR を使わない。
- 禁止されているのは、AI のベンダー名と Cloudflare の識別子だけ。FL の製品名・インストール先の名前・変数名は、書いてよい。
- sleep・timeout のコマンド・nohup・末尾の `&` は使わない。待つ時は条件で待つ。一時ファイルは `.tmp-*` に置き、終わったら消す。
- 製品の作業は Actions で行う。この機械で製品のビルドや測定をしない。

## 作業の進め方

- 作業の最初に `date -u` を記録する。4 時間 30 分を過ぎたら、受け入れが通らなくても、作業ツリーを commit し、報告を書いて終える(ジョブの上限は 5 時間 50 分)。
- ビルドは約 35 分。フォアグラウンドで直接、シェルツールの `timeout` 引数は最大 5400000(90 分)にする。背景で起動して待たない。
- 受け入れが通らなくても、作業の最後に作業ツリーを commit する。未 commit の変更が残ると、判定は失敗する。

## 他のタスクとの関係

- blocked_by: なし。
- 第 2 波の内蔵プラグインの束は、`lift/plugins/<名前>/` を足すだけで、この Issue が作る一覧に出る。束の追加で、共有のファイルを変えない。
- #{{194}}・#{{195}}・#{{196}}・#{{197}}(カーソル・ページの HTML・クリップボード・初回の描画)は、`tools/e2e/scenarios/smoke/index.mjs` を触る。衝突したら、作業の最後に `git pull --no-rebase --no-edit` で main を取り込み、衝突を直し、受け入れをもう一度通す。
