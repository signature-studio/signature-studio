---
draft: "265"
title: "E2E の plugins: 一覧(Installed)の読みが本の開始位置から始まり、共有の page では一覧が見えないため、282 本が一覧の段で止まる(印の受理の後ろに隠れている)"
type: Task
milestone: "M5 内蔵のエフェクトとインストゥルメント"
parent: "133"
blocked_by: []
labels: ["verify"]
size: S
priority: High
project: signature-studio/3
---

## 目的

plugins の本は、挿入の段の最初に、Plugin database の `Installed` の一覧(engine のログの `enum-dir` の行)を読む。この読みが、本の開始位置(`engineLog().length` の値)から始まる。しかし shard の page は多くの本で共有され、一覧はその page の開始時に 1 回しか書かれない。そのため、本の開始後のログには一覧が無く、本は一覧の段で止まる。e2e-full の最新の run では 302 本のうち **282 本** がこの理由で止まり、その大半は印(`expectFail`)で受理されて、停止が見えなくなっていた。

この Issue の目標は次の 3 つ。

1. 共有の page の本でも、page の中の一覧の行を読めること(読みの範囲を、本の開始位置ではなく、その page のログの中の一覧の行にする。試験の名前と判定を変えない)。
2. 一覧の段で止まる本が、0 本になること(shard の出力に `no listing of Installed` の注記が残らない)。
3. 一覧の段を越えた後の段(行・窓・パラメータ・音声・止まらない)の結果が、本ごとに出ること。印(`expectFail`)の付け替えは、結果を見た後に、別に行う。

## 背景

### 最新の run の結果(実測。`tools/e2e` の出力)

- plugins の結果は 302 件(1 表示)。`ok` は 7 本。
- 注記の内訳: `no listing of Installed in the engine log since the scenario started (the listing is written once per page)` が 282 本。`the row of … is not located` が 11 本。その他 9 本。
- 挿入の段を越えた結果(`insert`・`window`・`param`・`audio`・`stop` のどれかが値を持つ本)は **0 本**。
- 282 本は、ほぼ全て印つき(`expectFail` が `#1435`・`#1440`・`#1535` など。開いた Issue と閉じた Issue の両方)で、印の判定で受理されている。印の無い失敗は 1 本(`plugins/plugin-database-browser-opens`)だけ。
- 閉じた Issue #1535 は、一覧の読みを「列挙で判定する」に変えた(commit 1a46c2c3)。しかし、本の開始位置から読む部分は残っている(実測)。

### 製品ではなく道具の箇所(コードで確認)

- `tools/e2e/lib/plugins.mjs` の `insertFromDatabase`: `const from = await guest.ev('window.__app.engineLog().length')` の後で、`databaseLines(guest, from)` を呼ぶ。
- `databaseLines(guest, from)` は `window.__app.engineLog().slice(from)` を読み、`installedCount(lines)` は、その中の `enum-dir` の行の最大の件数を返す。一覧の行が無ければ `null`。
- `tools/e2e/run.mjs` の `runShared` は、shard の 1 行(表示ごと)の page を 1 回だけ開き、本を続けて実行する(その page が壊れた時だけ、読み直す)。
- 一覧は page の開始時(`startProject` の後)に 1 回だけ書かれる(本の注記による。engine の書き方は、この run のログからは確かめられない)。

### 見立て(推定)

- 一覧の行は、page の開始時のログの中に在る。本の開始位置から読むと、必ず見えない。
- 確認できない: engine が一覧を書く正確な時点(page の開始時のみか、Plugin database を開いた時か)。

## 読む物

- `tools/e2e/lib/plugins.mjs`(`insertFromDatabase`・`databaseLines`・`installedCount`・`categoryCount`・`databaseHas`)。
- `tools/e2e/lib/common.mjs`(`startProject`)と `tools/e2e/run.mjs`(`runShared`・`openShared`)。
- `tools/e2e/scenarios/plugins/` の任意の 1 本(例: `builtin-effects-fruity-delay-2-fruity-delay-2-x64.mjs`)と、`tools/e2e/features/` の plugins の表。
- `tools/e2e/scenarios/plugins/_generate.mjs` があれば、生成の元。

## やること

1. 再現: `--area plugins --shard 1/11 --displays 4k-dpr1` を走らせ、注記 `no listing of Installed` の本の数を数える(282 本の相当の数が出ることを確かめる)。
2. 読みを直す: 一覧を、本の開始位置からではなく、その page のログの中の一覧の行から読む。例: 本の開始時に、直前の `enum-dir` の行の位置を page の状態として覚えておく。page を作り直した時は、その位置を作り直す。試験の判定(件数・名前の有無)は変えない。
3. 一覧の段で止まる本が無くなったことを、shard の出力で確かめる。
4. 段を越えた結果を、本ごとに確かめる(挿入・窓・パラメータ・音声・止まらない)。印(`expectFail`)は、この Issue では変えない。結果を報告に書き、印の付け替えは別の作業にする。
5. 受け入れを通す。

## 受け入れ

```accept
npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false?0:1)' "$RUNNER_TEMP/boot.json" && (cd tools/e2e && npm ci && npx playwright install chromium) && node --test tools/e2e/test/*.test.mjs && node tools/e2e/run.mjs --area plugins --shard 1/11 --displays 4k-dpr1 --budget 35m --report-only > "$RUNNER_TEMP/p1.jsonl" && node -e 'const L=require("fs").readFileSync(process.argv[1],"utf8").trim().split("\n").map((x)=>JSON.parse(x));process.exit(L.length>=1&&L.every((r)=>r.error===null&&r.scenarios.length>=1)&&L.every((r)=>r.scenarios.every((s)=>!(s.notes||[]).join(" ").includes("no listing of Installed")))?0:1)' "$RUNNER_TEMP/p1.jsonl"
```

判定の基準:
- 受け入れの式が終了コード 0 で終わる(単体の `node --test` が通り、起動確認が engine-idle、plugins 1/11 の全本が一覧の段を越える。注記 `no listing of Installed` が 0 本)。
- 報告に: (a) 修正前の注記の本数と、修正後の本数(同じ shard)、(b) 段を越えた結果の本数と、段ごとの内訳(挿入・窓・パラメータ・音声・止まらない)、(c) 読みを直した箇所(ファイルと行)、(d) 確かめられなかった点(engine が一覧を書く時点)。
- 製品(`app/`・`lift/`)は変えない(読むだけ)。この Issue は `tools/e2e/` の道具を直す。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 「達成」「直った」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。出力の行を付ける。
- 確認できなかった物は「確認できない」と書く。
- 段ごとに、やること 1〜5 のどこまで進んだか(完了・途中・未着手)を書く。

## 制約

- 製品(`app/`・`lift/`)は変えない(読むだけ)。`tools/e2e/` を直す。
- 製品に、利用者向けの新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足さない。オプトインを作らない。
- Python を使わない(調査と検証にだけ使う)。OCR を使わない。
- 禁止されているのは、AI のベンダー名と Cloudflare の識別子だけ。
- sleep・nohup・末尾の `&`・先頭の `timeout` は使わない。待つ時は条件で待つ。
- 一時ファイルは `.tmp-*` に置き、終わったら消す。
- commit・PR に、AI が書いたことを示す行を付けない。commit の名義は、環境の設定のまま。
- GitHub に、Issue・ラベル・コメントを書かない。

## 作業の進め方

- 作業の最初に `date -u` を記録する。4 時間 30 分を過ぎたら、受け入れが通らなくても、作業ツリーを commit し、報告を書いて終える(ジョブの上限は 5 時間 50 分)。
- ビルドと E2E は、この job の中で行う。長いコマンドは、フォアグラウンドで直接実行し、シェルツールの `timeout` 引数に 5400000 を渡す。背景で起動して待たない。
- 受け入れが通らなくても、作業の最後に作業ツリーを commit する。`git add` は、自分が変えたパスだけを指定する。

## 他のタスクとの関係

- 内蔵プラグインの束(#1434〜#1477 など)は、挿入の段の先の作業。この Issue で一覧の段を越えないと、その束の判定は進まない。
- 印の付け替え(`#1440`・`#1441`・`#1445`・`#1447`・`#1535` の閉じた印)は、この Issue の結果を見た後に、別に行う。
