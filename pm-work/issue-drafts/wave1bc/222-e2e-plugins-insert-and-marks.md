---
draft: "222"
title: "E2E の plugins を、挿入の手順と観測を直して書く"
type: Task
milestone: "M2 HiDPI と P2 の測定(P2)"
parent: "133"
blocked_by: ["220"]
labels: ["verify", "render"]
size: XL
priority: High
project: signature-studio/3
---

## 目的

`plugins` 領域(302 本のシナリオ)の初回の結果は、表示 2 本で 604 件のうち、合格が 8 本だけだった。窓・パラメータ・音声・止まらないの 4 段に進んだ結果は 0 件。原因は、テストの手順と観測の作りにある。挿入をしないで窓を探す、一覧をログ全体から読む、閉じた Issue の番号を印にしている、の 3 つ。この Issue は、この 3 つを直し、印を実態に合わせる。製品(`app/`・`lift/`)は変えない。

## 背景

- 内訳: 内蔵プラグインの 142 本(`builtin-*.mjs`)、外部プラグインの 3 本(`external-*.mjs`: Serum 2・OTT・Vital)、窓の位置の 145 本(`position-*.mjs`。内蔵と外部)、手で書いた 12 本。
- 位置の 145 本(失敗 277 件): プラグインを挿入せずに窓を探す(`no window named …`)。構造上、通らない。`tools/e2e/lib/plugins.mjs` の `positionSource` は、Plugin database を開いた後に、窓を探すだけ。
- 内蔵・外部の 145 本: Plugin database の一覧に、項目の名前が無い(`the Plugin database has no entry for …`)で止まる。初回の run の一覧は、dpr2 では毎回 4 行、dpr1 は各 shard の 1 件目だけ 4 行で、以後は 0 行。名前の行は 1 件も無かった。`enum-dir` の件数は 2。
- 前提(PM の確認。この Issue の作業で確かめる): 一覧に出るのは、`lift/plugins/<名前>/plugin.tsv` が在る内蔵プラグインだけ(今は Fruity Balance と Sytrus の 2 本)。他の内蔵プラグインは、開いている束の Issue(#1434 〜 #1477 など。`pm-work/created-issues.tsv` の題名で対応を引ける)が、一覧に出す作業をする。
- 通った 8 本: `balance-lazy-entry`・`sytrus-lazy-entry`・`manifest-lists-plugins`(マニフェストを読む)、`picker-opens`・`picker-position-random`(picker)、`plugin-database-browser-opens`・`plugin-database-count`・`plugin-database-item-select`(Plugin database の画面と、一覧の読み)。
- 手順の欠陥:
  - `tools/e2e/lib/plugins.mjs` の `pluginSteps` は、Plugin database の項目を挿入する段(項目の二度押し)を「未確認」と注釈したまま、挿入をしていない。窓を探す前に、挿入が要る。
  - `databaseLines(guest, from = 0)` は、開始位置が既定の 0(ログ全体)。`plugin-database-count`・`plugin-database-browser-opens`・`preset-fruity-balance-load`・`preset-serum-2-load`・`preset-sytrus-load` は、開始位置を渡さずに呼んでいる。
- 印の欠陥: `tools/e2e/scenarios/plugins/_generate.mjs` は、Fruity Balance・Fruity Filter・Fruity Delay 2・Sytrus の 4 本に `#1484`(閉じている)を付け、外部の 3 本に `#1336`・`#1337`・`#1338`(閉じている)を付けている。`#1319`(閉じている)は、Fruity Filter と Fruity Delay 2 の束の番号として、表から引かれる。手で書いた `preset-*.mjs` にも、閉じた番号(`#1315`・`#1336`・`#1400`)の印が 3 本ある。閉じた番号の印は、`plugins/` の中に 17 本ある(生成された分と、手で書いた分)。
- 印の実態: 開いている束の番号の印(#1434 〜 #1477 の 44 本の Issue は開いている)は、正しい。

## 読む物

- `tools/e2e/lib/plugins.mjs`(`pluginSteps`・`openPluginDatabase`・`openSlotPicker`・`databaseLines`・`databaseHas`・`pluginWindow`・`positionSource`・`builtinSource`・`markFor`・`issueFor`)
- `tools/e2e/scenarios/plugins/_generate.mjs`(生成器。`lift/plugins/` を読むように直す)・生成された `builtin-*.mjs`・`external-*.mjs`・`position-*.mjs`(手で編集しない。生成し直す)
- `tools/e2e/scenarios/plugins/` の手で書いた 12 本(`preset-*.mjs`・`plugin-database-*.mjs`・`picker-*.mjs`・`balance-lazy-entry.mjs`・`sytrus-lazy-entry.mjs`・`manifest-lists-plugins.mjs`・`plugin-manager-opens.mjs`)
- `tools/e2e/features/generated/plugins.tsv`・`tools/e2e/README.md`
- `lift/plugins/fruity-balance/plugin.tsv`・`lift/plugins/sytrus/plugin.tsv`(内蔵プラグインの形)
- `pm-work/created-issues.tsv`(生成器が読む。Issue の番号と題名の表)

## やること

0. 作業の最初に `date -u` を記録する。`.prior-work.md` が在れば、最初に読む。
1. **試験を先に書く**: 生成器の印の決め方(次の 5)を、`tools/e2e/test/` で試せる純粋な関数にして、試験を書く。直す前に、試験が落ちることを確かめる。
2. **挿入の段を入れる**: `position-*` と、内蔵・外部の窓の段の前に、Mixer の挿入スロットの picker から、Plugin database の項目を挿入する段を入れる。挿入の操作(項目の二度押しか、ドラッグか、確かめた操作)は、実際に動かして決める。`pluginSteps` の「未確認」の注釈を、確かめた事実に書き換える。挿入の後に、窓を探す。挿入の結果(挿入スロットの名前や、窓の表の変化)を、結果の `results` に書く。
3. **一覧の観測**: `databaseLines` は、必ず開始位置(シナリオの開始時のログの長さ)を渡す。上の 5 本の呼び出しを直す。結果に、一覧の件数と、名前の有無を出す(`results.count`・`results.named`)。`notes` の「the Installed folder is empty in this build」の文言は、計数と名前の有無から言える事実だけにする。
4. **前提の確認**: 一覧に出るのは `lift/plugins/<名前>/plugin.tsv` が在る内蔵プラグインだけ、の前提を、この Issue の run の engine のログで確かめる。確かめた結果を報告に書く。前提が違うなら、報告に書いて、印の付け方を、確かめた事実に合わせる。
5. **印の付け方(生成器)**: `_generate.mjs` を直す。
   - (a) 内蔵プラグインは、名前を小文字・ハイフンにした名前(Fruity Balance → `fruity-balance`)の `lift/plugins/<名前>/plugin.tsv` が在る時、印なし(`expectFail: null`)。
   - (b) 無い時は、表(`pm-work/created-issues.tsv`)から、その名前を含む束の Issue の番号を引く(今の `markFor`)。ただし、閉じている番号は印にしない。閉じた番号の一覧(今は #1315・#1319・#1336・#1337・#1338・#1400・#1484)を、生成器の中に書く。
   - (c) 開いた束が無い物は、`new`。
   - (d) 外部(OTT・Serum 2・Vital)は、印 `#1336`・`#1337`・`#1338` を外す。run で ok になるなら印なし。ok にならないなら `new`。
   - (e) 手で書いた `preset-*.mjs` の閉じた番号の印(`#1315`・`#1336`・`#1400`)も、同じ規則で直す。
   - (f) 生成のやり方(`node tools/e2e/scenarios/plugins/_generate.mjs`)を `tools/e2e/README.md` に書く。
6. 生成し直す(`node tools/e2e/scenarios/plugins/_generate.mjs`)。生成された `builtin-*`・`external-*`・`position-*` の件数を、報告に書く。
7. 受け入れを通す。通らない ID は、落ちた ID・表示・出力の 1 行を報告に書く。

## 受け入れ

```accept
npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false?0:1)' "$RUNNER_TEMP/boot.json" && node tools/smoke/welcome-flow.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/flow.jsonl" && node -e 'const fs=require("fs");const L=fs.readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const F=(f)=>L.find((r)=>r.flow===f);const ok=(r)=>!!r&&r.error===null&&r.state==="engine-idle"&&r.standIn===false&&r.trapped===false&&r.hostErrors===0&&r.opened===true;process.exit(L.length===4&&["A","B","C","D"].every((f)=>ok(F(f)))&&F("B").welcomeClosed===true&&F("D").welcomeClosed===true?0:1)' "$RUNNER_TEMP/flow.jsonl" && npm test --prefix app && (cd tools/e2e && npm ci && npx playwright install chromium) && node --test tools/e2e/test/*.test.mjs && node -e 'const fs=require("fs");const d="tools/e2e/scenarios/plugins";const bad=fs.readdirSync(d).filter((f)=>f.endsWith(".mjs")).filter((f)=>/expectFail: .#(1315|1319|1336|1337|1338|1400|1484)./.test(fs.readFileSync(d+"/"+f,"utf8")));process.exit(bad.length===0?0:1)' && node tools/e2e/run.mjs --area plugins --shard 1/11 --displays 4k-dpr1 --budget 40m --report-only > "$RUNNER_TEMP/p1.jsonl" && node -e 'const L=require("fs").readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const S=L.flatMap((r)=>r.scenarios);const u=S.filter((s)=>s.expectFail===null&&s.ok!==true&&s.skipped!==true).length;process.exit(L.length>=1&&S.length>0&&L.every((r)=>r.error===null&&r.servedHeadersMatch===true)&&u<=3?0:1)' "$RUNNER_TEMP/p1.jsonl" && node tools/e2e/run.mjs --area plugins --shard 6/11 --displays 4k-dpr1 --budget 40m --report-only > "$RUNNER_TEMP/p6.jsonl" && node -e 'const L=require("fs").readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const S=L.flatMap((r)=>r.scenarios);const u=S.filter((s)=>s.expectFail===null&&s.ok!==true&&s.skipped!==true).length;process.exit(L.length>=1&&S.length>0&&L.every((r)=>r.error===null&&r.servedHeadersMatch===true)&&u<=3?0:1)' "$RUNNER_TEMP/p6.jsonl" && node tools/e2e/run.mjs --area plugins --only plugins/position-effects-fruity-balance-fruity-balance-x64,plugins/position-generators-sytrus-sytrus-x64 --displays 4k-dpr1 --budget 40m > "$RUNNER_TEMP/pos.jsonl" && node -e 'const L=require("fs").readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const S=L.flatMap((r)=>r.scenarios);process.exit(L.length===1&&S.length===2&&S.every((s)=>s.ok===true&&s.expectFail===null&&s.skipped!==true)?0:1)' "$RUNNER_TEMP/pos.jsonl"
```

判定の基準:
- 受け入れの式が、終了コード 0 で終わる。
- `plugins/position-effects-fruity-balance-fruity-balance-x64` と `plugins/position-generators-sytrus-sytrus-x64` が、両方とも ok で、印なし。この 2 本は、shard 1/11 と 6/11 には入らないので、`--only` で別に走らせる。
- shard 1/11 と 6/11 の、未印で ok が偽(skipped を除く)のシナリオが、それぞれ 3 本以下。
- `plugins/` の中に、閉じた番号の印が無い(受け入れの式が見る)。
- 報告に: (a) 前提(一覧に出るのは `plugin.tsv` が在る内蔵プラグインだけ)の確認の結果と、engine のログの行、(b) 挿入の操作(どの操作で、どのスロットに入ったか)、(c) 生成器の印の付け方の結果(印なし・開いた束の番号・`new` の件数)、(d) 生成された `builtin-*`・`external-*`・`position-*` の件数、(e) 外部 3 本の印の結果、(f) 2 つの shard の出力の行。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には、runner の値であることと、nproc と loadavg を添える。
- 「達成」「直った」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。出力の行を付ける。
- 確認できなかった物は「確認できない」と書く。
- 段ごとに、やること 0〜7 のどこまで進んだか(完了・途中・未着手)を書く。

## 制約

- 製品(`app/`・`lift/`)は変えない(読むだけ)。`.github/`・`spec/`・`ci/` も変えない。
- 生成された `builtin-*.mjs`・`external-*.mjs`・`position-*.mjs` は手で編集しない。`_generate.mjs` を直し、生成し直す。
- 製品に、利用者向けの新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足さない。オプトインを作らない。テストのための分岐を製品に入れない。
- Python を使わない(調査と検証にだけ使う)。OCR を使わない。文字は、ゲストのメモリの文字列・窓の表・画素のハッシュで読む。
- 禁止されているのは、AI のベンダー名と Cloudflare の識別子だけ。FL の製品名・インストール先の名前・変数名は、書いてよい。
- sleep・nohup・末尾の `&`・先頭の `timeout` は使わない。待つ時は条件で待つ。
- 一時ファイルは `.tmp-*` に置き、終わったら消す。
- commit・PR に、AI が書いたことを示す行を付けない。commit の名義は、環境の設定のまま。
- GitHub に、Issue・ラベル・コメントを書かない。

## 作業の進め方

- 作業の最初に `date -u` を記録する。4 時間 30 分を過ぎたら、受け入れが通らなくても、作業ツリーを commit し、報告を書いて終える(ジョブの上限は 5 時間 50 分)。
- この作業の job(GitHub Actions の runner)で、受け入れの式の全てを実行する。ビルドと E2E は、この job の中で行う。
- 受け入れの式は、ジョブの判定が 1 本で実行する。作業者は、式の段ごとに、シェルの呼び出しを分けて確かめてよい。
- ビルドや E2E の長いコマンドは、フォアグラウンドで直接実行し、シェルツールの `timeout` 引数に 5400000 を渡す。背景で起動して待たない。
- ビルドは 1 回にする。tools だけを直した後は、同じ bundle(`lift/out-app/bundle/raw/fl-Oz`)で E2E を走らせて直す。
- 1 回で終わらない前提で、段ごとに commit する。再試行は前回の差分の上で続け、作業ツリーを消さない。
- 受け入れが通らなくても、作業の最後に作業ツリーを commit する(未 commit の変更が残ると、判定は失敗する)。

## 他のタスクとの関係

- blocked_by: #{{220}}(表示の宣言を守る・行ごとの予算・JSON の `displays`)。この Issue は、#{{220}} が閉じてから始まる。
- #{{221}}(windows・settings)と、ファイルは重ならない。#{{223}}(topbar・page・resilience・perf)とも、ファイルは重ならない。
- 閉じた番号の印は、この Issue だけが直す(#{{223}} は plugins を触らない)。
