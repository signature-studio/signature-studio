---
draft: "249"
title: "Plugin database の Installed に、プラグインの行が出ない: 分類データ(Data\\Patches\\Plugin database の .nfo)を VFS に入れ、E2E の『行がある』判定を画素の読みに直す"
type: Task
milestone: "M4 FL に必要な物の配布"
parent: "012"
blocked_by: []
labels: ["delivery", "plugin-builtin", "verify"]
size: L
priority: Urgent
project: signature-studio/3
---

## 目的

依頼者の指示(100 を超える内蔵のエフェクトと楽器を、Plugin database から選んで挿入できること)の入口。Plugin database の Installed の一覧に、内蔵プラグインの行が描かれ、行を選べるまでにする。plugins の E2E(302 本)は、挿入の段で全て止まっている。原因を、製品と測り方に分けて特定し、直す。

## 背景

- #1484(閉じている)で、Installed の項目(`Presets\Plugin database\Installed\<名前>.nfo`)を VFS に置いた。エンジンのログでは、Installed の件数は 3 件(`enum-dir: 3`)。それでも一覧に行が描かれない。#1508 の探索では、Installed は選んでも展開されず(矢印・キーでも)、ピッカーの「More plugins…」の一覧も 0 件だった。
- **確定(E2E の測り方)**: `tools/e2e/lib/plugins.mjs` の `databaseHas` は、プラグインの名前がエンジンのログの行に出ることを求める。ログに出るのは Installed の件数(`enum-dir`)だけで、名前は出ない。さらに、成功した読みはログに出ない。よって、e2e-full の挿入の段の「no entry」(146 本)は、製品の状態の証拠にならない。
- **最有力の製品の候補(推定)**: FL の Plugin database の木は、`Data\Patches\Plugin database\` の分類データ(`Installed.nfo`・`Effects.nfo`・`Generators.nfo`)を読む。`pm-work/findings/missing-paths.md`(#1316。#1484 より前の 3 回の起動)では、FL が `Data\Patches\Plugin database\` の `Installed.nfo`・`Effects.nfo`・`Generators.nfo` を、3 回とも探して無かった。インストーラには在る(大きさ 67・71・61 B)が、seed の行が無いので抽出されず、bundle に入らない。`lift/harness/seeds/vfs-seed-idle1.tsv` にあるのは `Data\Patches\Plugin database.nfo` の 1 行だけ。抽出は seed の行で決まる(`app/scripts/extract-fl.mjs`)。
- **項目の形式(推定)**: 製品の Installed の項目は `Tip=<名前>` だけ(`lift/crates/fe-build/src/plugins.rs` の `plugin_db_rows`)。インストーラの実物の `Data\System\Plugin databases\Plugin database (alphabetical)\…` の `.nfo` は、名前の長さ + 11 B(136 件すべて)で、`.fst` と `.png` も持つ。製品は `.nfo` だけ。
- 確認できないこと: 木が Installed の子を作るかどうか。FL が `.fst`・`.png` を要るかどうか。`.nfo` の中身の形式。`.Plugins.ini` の役割。一覧が WebView2 の面の下で隠れていないか。読む時機。現在の main で、分類データが今も無いかどうか(上の記録は #1484 より前)。
- 位置のシナリオ(`plugins/position-*`)は、挿入をしないので、挿入の段(#1508)が main に入るまで ok にならない。この Issue の受け入れは、新しい smoke のシナリオ(一覧の行が描かれる)で行う。位置のシナリオの印の外しは、#1508 の後に行う。
- 読みの根拠は `pm-work/findings/plugin-database-rows.md` にある(file:line と、実測・推定の区別)。

## 読む物

- `pm-work/findings/plugin-database-rows.md`(この Issue の根拠)
- `tools/e2e/lib/plugins.mjs`(`databaseLines`・`databaseHas`・`pluginSteps`)・`tools/e2e/lib/common.mjs`(`pixelHashRegion`・`windowTable`)
- `tools/e2e/scenarios/smoke/plugin-database-lists-builtins.mjs`・`tools/e2e/scenarios/smoke/index.mjs`
- `tools/e2e/run.mjs`・`tools/e2e/lib/scenario.mjs`(宣言した表示。宣言していない表示では走らない)
- `lift/crates/fe-build/src/plugins.rs`(`plugin_db_names`・`plugin_db_rows`・`generate`)
- `lift/harness/seeds/vfs-seed-idle1.tsv`(seed の行の形。`Data\Patches\Plugin database.nfo` の行が見本)・`app/scripts/extract-fl.mjs`(seed の行の抽出と SHA-256 の検査)
- `inputs/fl-installer-files.tsv`(`Data\Patches\Plugin database` の 3 本と、`Data\System\Plugin databases\Plugin database (alphabetical)` の行。大きさと SHA-256 はこの表が正)
- `lift/runtime/fle_vfs.c`(`note_listing`・`fle_imp_FindFirstFileW`・`note_missing`)
- `pm-work/findings/missing-paths.md`(FL が探して無かったパスの記録。`Data\Patches\Plugin database\` と `Presets\Plugin database\Installed\.Plugins.ini`)・`pm-work/findings/crash-hunt-2026-11-25.json`(同じ系統の記録)

## やること

0. 作業の最初に `date -u` を記録する。`.prior-work.md` が在れば、最初に読む。
1. **原因の特定(実測。現在の main の bundle で)**:
   a. Mixer の挿入スロットの picker から Plugin database を開き、Installed を開く(矢印と Enter)。画面を `--shots` で保存し、`read` で画像を直接見て、Installed の横に展開の矢印が在るか、子の行が在るかを報告に書く。OCR は使わない。
   b. エンジンのログ(`--engine-logs`)で、`Plugin database` を含む行を全部集める。特に `missing-path` の行(`Data\Patches\Plugin database\Installed.nfo`・`Effects.nfo`・`Generators.nfo`、`Presets\Plugin database\Installed\.Plugins.ini`、および `.fst`・`.png` を探す行)を、報告に貼る。
   c. 判定(実測で 1 つ選び、根拠の行を貼る):
      - (a) 分類データの欠落: b の分類データが missing で、Installed に展開の矢印が無い → 2 の (a) を行う。
      - (c) 測り方: 行が画面に在る(画像で見える)のに、判定が取れない → 2 の (c) を行う。
      - (b) 被り: 行の領域に WebView2 の面が重なる(`webview2:` の行と窓の表で確かめる) → 報告に書き、直し方を決めて 2 で行う。
      - (d) 時機: 少し待つと行が出る → 待つ条件を書いて 2 で行う。
      - いずれでもない: 「確認できない」と書き、2 の (c) だけを行う。
2. **直す**:
   - (a) 分類データ: `lift/harness/seeds/vfs-seed-idle1.tsv` に、`Data\Patches\Plugin database.nfo` の行の隣へ、`Data\Patches\Plugin database\Effects.nfo`・`Generators.nfo`・`Installed.nfo` の 3 行を足す(形式は既存の行と同じ。SHA-256 と大きさは `inputs/fl-installer-files.tsv` の行の値)。抽出と検査は `app/scripts/extract-fl.mjs` が行う。
   - (a) 項目の形式: `plugin_db_rows` が書く中身を、実物の `Plugin database (alphabetical)` の `.nfo` の形に合わせる案を比べる。実物の中身は抽出した物を読んで決める。製品のコードに中身の値を書き並べない。`.fst`・`.png` の要否は、1 の b の `missing-path` の行で決める。決めた理由を報告に書く。
   - (c) E2E の測り方: `tools/e2e/lib/plugins.mjs` の `databaseHas` を、画素の読みに変える。Installed を開く前と後で、一覧の領域の画素のハッシュ(`pixelHashRegion`)が変わったことを、行が描かれたこととする。名前の一致は、画像で見て確かめ、報告に書く。`databaseLines` の件数は、件数として残す。
   - 新しい smoke のシナリオ `tools/e2e/scenarios/smoke/plugin-database-rows-drawn.mjs` を足す(`smoke/index.mjs` に並べる)。宣言する表示は `4k-dpr1` と `4k-dpr2`。判定: Installed を開いた後に一覧の画素が変わる・engine-idle・エラーが 0。
   - `tools/e2e/lib/plugins.mjs` と `tools/e2e/scenarios/plugins/` は #1508 と重なる。衝突したら、作業の最後に `git pull --no-rebase --no-edit` で main を取り込み、衝突を直す。
   - 製品のコードに、内蔵プラグインの名前を書き並べない。Installed の項目は `lift/plugins/<名前>/plugin.tsv` から作る(今の `plugin_db_names`)。
3. 受け入れを通す。通らなければ、落ちた段と、出力の 1 行を報告に書く。

## 受け入れ

```accept
npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false&&r.elapsedMs<=20000?0:1)' "$RUNNER_TEMP/boot.json" && node tools/smoke/welcome-flow.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/flow.jsonl" && node -e 'const fs=require("fs");const L=fs.readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const F=(f)=>L.find((r)=>r.flow===f);const ok=(r)=>!!r&&r.error===null&&r.state==="engine-idle"&&r.standIn===false&&r.trapped===false&&r.hostErrors===0&&r.opened===true;process.exit(L.length===4&&["A","B","C","D"].every((f)=>ok(F(f)))&&F("B").welcomeClosed===true&&F("D").welcomeClosed===true?0:1)' "$RUNNER_TEMP/flow.jsonl" && npm test --prefix app && (cd tools/e2e && npm ci && npx playwright install chromium) && node --test tools/e2e/test/*.test.mjs && node tools/e2e/run.mjs --scenarios smoke --displays 4k-dpr1,4k-dpr2 > "$RUNNER_TEMP/e2e.jsonl" && node -e 'const L=require("fs").readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));process.exit(L.length===2&&L.every((r)=>r.error===null&&r.servedHeadersMatch===true&&r.scenarios.some((s)=>s.name==="plugin-database-rows-drawn"&&s.ok===true&&s.expectFail===null&&s.skipped!==true&&s.engineStops===0))?0:1)' "$RUNNER_TEMP/e2e.jsonl" && bash guard/scan.sh
```

判定の基準:
- 起動確認(`boot.json`): engine-idle・standIn が false・error が null・elapsedMs が 20,000 以下。
- 流れ A〜D の 4 行が、error が null・engine-idle・standIn が false・trapped が false・hostErrors が 0・opened が true。B と D は welcomeClosed が true。
- `npm test --prefix app` と `node --test tools/e2e/test/*.test.mjs` が通る。
- `node tools/e2e/run.mjs --scenarios smoke --displays 4k-dpr1,4k-dpr2` が終了コード 0。両方の表示の行で、`plugin-database-rows-drawn` が ok が真・expectFail が null・skipped でない・engineStops が 0。
- `bash guard/scan.sh` が 0 件。

報告に書く:
- (a) 1 の a〜c の観測: 画像で見た事実、`missing-path` の行(貼る)、判定した候補と理由。
- (b) 直した箇所: seed の 3 行の SHA-256 と大きさ、項目の形式の決めた理由、`databaseHas` の変更。
- (c) `plugin-database-rows-drawn` の出力の行(両方の表示)。
- (d) やること 0〜3 のどこまで進んだか(完了・途中・未着手)。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には、runner の値であることと、nproc と loadavg を添える。
- 「達成」「直った」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。出力の行を付ける。
- 確認できなかった物は「確認できない」と書く。実測と推定を分ける。

## 制約

- 製品に、利用者向けの新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足さない。オプトインを作らない。テストのための分岐を製品に入れない。
- 内蔵プラグインの名前の一覧を、製品のコードに書き並べない。
- Python を使わない(調査と検証にだけ使う)。OCR を使わない。画像は `read` で直接見る。
- 禁止されているのは、AI のベンダー名と Cloudflare の識別子だけ。FL の製品名・インストール先の名前・変数名は、書いてよい。
- sleep・timeout のコマンド・nohup・末尾の `&` は使わない。待つ時は条件で待つ。一時ファイルは `.tmp-*` に置き、終わったら消す。
- commit・PR に、AI が書いたことを示す行を付けない。commit の名義は、環境の設定のまま。
- GitHub に、Issue・ラベル・コメントを書かない。

## 作業の進め方

- 作業の最初に `date -u` を記録する。3 時間 15 分を過ぎたら、受け入れが通らなくても、作業ツリーを commit し、報告を書いて終える(判定に約 2 時間かかり、ジョブの上限は 5 時間 50 分)。
- この作業の job(GitHub Actions の runner)で、受け入れの式の全てを実行する。ビルドと E2E は、この job の中で行う。
- ビルドは 1 回にする。長いコマンドは、フォアグラウンドで直接、シェルツールの `timeout` 引数に 5400000 を渡す。背景で起動して待たない。
- 段ごとに commit する。`git add` は 1 つずつ、自分が変えたパスだけを指定する。
- 受け入れが通らなくても、作業の最後に作業ツリーを commit する(未 commit の変更が残ると、判定は失敗する)。

## 他のタスクとの関係

- #1508(plugins の E2E の手順と、挿入の段。main に未マージ)は、`tools/e2e/lib/plugins.mjs` と `tools/e2e/scenarios/plugins/` を触る。衝突しうる。この Issue が変えるのは、`databaseHas`・smoke のシナリオ・seed の行・項目の形式だけ。
- #1484(閉じている。Installed の項目を VFS に置いた)。
- #1524(`tools/plugin-run` の場面の選び方)とは、ファイルが重ならない。
- 位置のシナリオ(`plugins/position-*`)の印の外しは、#1508 の挿入が main に入った後に行う。この Issue では触らない。
