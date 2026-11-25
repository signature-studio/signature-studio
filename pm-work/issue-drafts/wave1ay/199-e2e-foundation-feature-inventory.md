---
draft: "199"
title: "E2E の基盤を、機能の一覧を軸にした形に拡げる: シナリオの登録・機能のインベントリ・網羅率(`coverage.json`)・シャード・DPR の指定"
type: Task
milestone: "M2 HiDPI と P2 の測定(P2)"
parent: "133"
blocked_by: []
labels: ["verify", "render"]
size: L
priority: Urgent
project: signature-studio/3
---

## 目的

依頼者の指示(2026-11-26。原文): 「そうではなく、存在する機能について、考えうる限り全ての E2E テストを追加してください。」

この Issue は、E2E の基盤を、「存在する機能の一覧」を軸にした形に広げる。この Issue が閉じると、#{{200}}〜#{{212}}(13 領域)が、この基盤の上に、機能ごとのシナリオを足せる。目標は次の 3 つ。
1. 機能の一覧(インベントリ)が、実行中の FL とリポジトリから自動で作られ、件数が出ること。
2. シナリオを、領域のディレクトリに 1 本ずつ足せること(登録の表を編集しない)。
3. 一覧に対する網羅率(`coverage.json`)が出ること。

## 背景

- 基盤は #{{164}}(#1407)と #{{165}}(#1408。回帰の集合。作業中)。今の構成: `tools/e2e/run.mjs`(`--scenarios`・`--only`・`--displays`・`--shots`・`--engine-logs`・`--clock`)・`lib/`(配信・ブラウザ・操作・検出・起動の手順・停止の理由)・`scenarios/smoke/`(3 本。`index.mjs` が並べる)・`judge.jq`(deploy の門)。
- 依頼者の指示の意味は、「クラッシュを探す走査」ではなく、存在する機能ごとに、期待する結果を確かめるシナリオ。`tools/crash-hunt/`(停止だけを見る走査。562 件)は残す。
- 4K の起動は、この runner で約 24 秒(`tools/e2e/README.md`)。シナリオごとに再読み込みすると、千件を超える時間が足りない。1 シャードの中では起動を 1 回にし、シナリオの間は、File > New などで初期化する。
- メニューの項目の名前は、画素からは読めない(OCR は禁止)。ゲストのメモリの文字列から取る方式は、#1358 の F10 の文字(`pm-work/findings/hidpi-fl-scaling.md` の節 10)と、#1391 の上部バー(`pm-work/findings/topbar-metrics-1391.md`・`tools/topbar-metrics/measure.mjs`)にある。取れない項目は、項目の番号(`File > #9` の形)で書き、名前は取れた物だけに付ける。
- 既存の部品: `lib/guest.mjs`(窓の表 `windows()`・`findWindow`・`click`・`rightClick`・`dblclick`・`wheel`・`drag`・`key`・`ctrl`・`ev`・`state`)・`lib/flow.mjs`(起動・Welcome・Problems の箱)・`lib/detect.mjs`(例外・消えない表示・失敗した要求・クリップボードと権限・エンジンの停止)・`lib/stop.mjs`。

## 読む物

- `tools/e2e/`(README.md・run.mjs・run.sh・judge.jq・package.json・lib/*.mjs・scenarios/smoke/*.mjs)
- `app/src/main.ts`(`window.__app` の API: `deskDumpBase64`・`engineLog`・`assetsReport`)
- `tools/feat-c6-popup-menus/measure.mjs`・`tools/crash-hunt/hunt.mjs`(メニューの操作と走査)・`tools/topbar-metrics/measure.mjs`・`tools/persist/measure.mjs`(ゲストのメモリの文字列の読み)
- `inputs/plugin-classes.tsv`・`inputs/external-plugins.tsv`・`inputs/fl-installer-files.tsv`(テンプレート・demo・言語・テーマの行)・`lift/plugins/`
- `app/src/` の各モジュール(`ls app/src`)。bundle の `manifest.json`(lazy の一覧。場所は作業で確かめる)
- `pm-work/findings/guest-layout.md`・`pm-work/findings/e2e-program.md`(この計画の全体)

## やること

1. **シナリオの登録**(`tools/e2e/lib/scenario.mjs`。新規)。
   - `defineScenario({ id, area, covers, displays, expectFail, run(ctx) })` を export する。`id` は `<領域>/<名前>`。`covers` は機能の ID の配列。`displays` は表示の名前の配列(既定は `['4k-dpr1', '4k-random']`)。`expectFail` は既定 null。
   - `run(ctx)` は `{ ok, notes, results }` を返す(今の smoke と同じ形。`ctx` は今の `runScenario` が渡す物と同じ)。
   - `tools/e2e/scenarios/<領域>/*.mjs` を `run.mjs` が自動で見つける(登録の表を編集しない)。`scenarios/smoke/` と `index.mjs` の既存の形は、そのまま動く。
2. **`run.mjs` の引数を足す**(既存の引数の意味は変えない)。
   - `--area <領域>`: `scenarios/<領域>/` の全てを動かす。領域の名前は次の 13 個: `page`・`input`・`file`・`menus`・`windows`・`audio`・`settings`・`plugins`・`topbar`・`resilience`・`visual`・`perf`・`delivery`。
   - `--list`: 起動しない。領域ごとに 1 行の JSON(`{"area","scenarios","covers","features"}`)。`scenarios` はシナリオの数、`covers` は `covers` に現れる異なる機能の数、`features` は `tools/e2e/features/` の一覧の機能の数。`--area` を付けると、その領域の 1 行だけを出す。
   - `--shard i/n`: シナリオの ID を昇順に並べ、`index % n === i - 1` の物を動かす(決定的)。
   - `--repeat n`: 選んだ各シナリオを n 回動かす(行に `repeat` の番号)。
   - `--only <ID>`: 領域の ID(`page/xxx`)でも指定できる。
   - `--budget 40m`(単位 `s`・`m`・`h`)。時間を過ぎたら、新しいシナリオを始めない。始めなかった物は `skipped: true` と JSON に出す。
   - `--displays` に `dpr:<数>`(例 `dpr:1.25`)を足せる。行の画素は、CSS の大きさを 3840 ÷ DPR・2160 ÷ DPR とする(今の `displayRows` と同じ式)。
3. **機能の一覧(実行中の FL から)**: `tools/e2e/inventory.mjs --out <DIR>`(新規)。本番と同じ配信で FL を起動し、次を列挙して `<DIR>/<領域>.tsv` に書く。
   - (a) `menus.tsv`(メニューの全階層): 上部の 8 つの見出し(窓の表のバーの位置)と、各項目。項目の名前は、メニューを開いた後の新しい最上位の窓(`TQuickPopupMenuWindow`)の矩形と、ゲストのメモリの `UnicodeString` の列を対応づけて取る。取れない項目は `File > #9` の形で書き、`nameSource` を `index` にする。取れた物は `memory`。
   - (b) `windows.tsv`(窓のクラス): F キー(F5〜F12・Alt+F8)・メニュー・ボタンで開く窓の、窓の表のクラスと題名。右クリックの文脈メニュー(チャンネルラック・ミキサー・プレイリスト・ブラウザ)を含む。ブラウザのタブ(8 の枠)と、ミキサーのインサート、ピアノロールの子窓も、ここに書く。
   - (c) `topbar.tsv`: 窓の表の `TWPControl` の題名の全て(`PositionToolbar`・`TimePanel`・`PatternPanel`・`MonitorPanel`・`CPUPanel`・`MenuPanel`・`DownloaderBtn`・`VolumePanel`・`PitchPanel`・`MetronomeBtn`・… の全て)。
   - (d) `settings.tsv`: F10 の 16 頁の全て。各頁の節・選択欄・チェック・ドロップダウン・スライダー・ボタン(窓の表と画素から)。
   - 各行の列: `id`(`<領域>/<種類>/<名前の slug>`)・`kind`・`name`・`source`(見つけた経路)・`how`(DPR 1 での位置の求め方)・`nameSource`(`memory` か `index`)。
4. **リポジトリから作る一覧**(同じ `<DIR>` に、同じ列の TSV を書く):
   - `plugins.tsv`: `inputs/plugin-classes.tsv` の本体(Effects・Generators)。`inputs/external-plugins.tsv` の外部(OTT・Serum 2・Vital)。Plugin database と Plugin manager の 2 件。
   - `file.tsv`: テンプレート(`Data\Templates` の全て)・demo の曲(`inputs/fl-installer-files.tsv` の demo の行)。File メニューの項目の名前は `menus.tsv` の値を使う。
   - `resources.tsv`: 言語 7(`Languages\`)・テーマ 13(`Artwork\Themes\`)。
   - `page.tsv`: `app/src/` のモジュールの機能(クリップボード・カーソル・ファイル選択・永続・音声の開始・サイズ変更・ビーコン・配信)。
   - `delivery.tsv`: bundle の `manifest.json` の lazy の全て。
   - `summary.json`: 総数と、領域ごとの件数(`{"total": n, "perArea": {…}}`)。
5. **機能の集計**: `tools/e2e/coverage.mjs`(新規)。`--features tools/e2e/features`(手で書いた `<領域>.tsv` と `generated/<領域>.tsv` の両方)と `--run <run の出力 JSON Lines>`(複数可)を読み、`--out <file>` に `coverage.json` を書く。形は `{ total, covered, perArea: { <領域>: { features, covered, ratio } } }`。「覆われた」は、`covers` に持ち、`ok` が真で、`expectFail` が null の結果が 1 つ以上ある機能。`expectFail` の結果は数えない。
   - `tools/e2e/judge-coverage.jq`(新規): 入力は `coverage.json`。領域ごとの `tools/e2e/features/<領域>.min` を下回ったら失敗。`.min` は TSV の 2 行: `scenarios<TAB>最小のシナリオ数` と `coverage<TAB>最小の網羅率(0 から 1)`。網羅率は、全シャードの結果を合わせて判定する(1 シャードでは判定しない)。
   - `tools/e2e/judge.jq` は変えない。`judge-regression.jq` は #{{165}} が作る。この Issue では、衝突しない形に置く(取り込んでから)。
6. **共通の部品**(`tools/e2e/lib/common.mjs`。新規。既存の lib は、必要最小限の差分で変える):
   - `menuPath(guest, ['File', 'Open'])`: `menus.tsv` の名前で項目を選ぶ。名前が無い時は番号で選ぶ。戻り値は選んだ項目の ID。
   - `waitWindow(guest, cls, ms)`: 窓の表に、そのクラスの窓が出るまで待つ(条件で待つ。上限は `ms`)。
   - `pixelHashRegion(guest, rect)`: 画素の一部を取り、SHA-256 のハッシュを返す(画素が変わったかの判定)。
   - `noBlackRegions(guest)`: 描画の全体の中の、全ての画素が 0 の矩形の数。
   - `guestText(guest)`: ゲストのメモリの `UnicodeString` の列。
   - `audioProbe(page, ms)`: 音声の PCM の要約(非無音の標本の数・途切れ・反復)。`tools/p3-audio-probe/` の計測を使う(重複させない)。
   - `persist(page, guest)`: 再読み込みの後に、保持の状態を確かめる(`tools/persist/measure.mjs` の方式)。
7. **領域の枠**: `tools/e2e/scenarios/<領域>/` を 13 個作り、各領域に見本のシナリオを 3 本ずつ置く(計 39 本。どれも通る物)。見本は、領域の Issue が同じ形で足せることを示す物(例: `page/canvas-fills-viewport`・`input/menu-open-click`・`file/new-project-empty`)。`tools/e2e/features/<領域>.tsv`(手で書く物。見出し行だけ)と `tools/e2e/features/<領域>.min` を 13 個作る。`.min` の値は次の通り(coverage は全領域 0.9)。
   - page 80・input 120・file 100・menus 400・windows 250・audio 80・settings 150・plugins 200・topbar 60・resilience 40・visual 120・perf 30・delivery 40。
8. **文書**: `tools/e2e/README.md` に、シナリオの書き方(上の 1〜7)と、印の規則(`expectFail` は `#<Issue の番号>` か `new`)を書く。

## 受け入れ

```accept
npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false?0:1)' "$RUNNER_TEMP/boot.json" && node tools/smoke/welcome-flow.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/flow.jsonl" && node -e 'const fs=require("fs");const L=fs.readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const F=(f)=>L.find((r)=>r.flow===f);const ok=(r)=>!!r&&r.error===null&&r.state==="engine-idle"&&r.standIn===false&&r.trapped===false&&r.hostErrors===0&&r.opened===true;process.exit(L.length===4&&["A","B","C","D"].every((f)=>ok(F(f)))&&F("B").welcomeClosed===true&&F("D").welcomeClosed===true?0:1)' "$RUNNER_TEMP/flow.jsonl" && npm test --prefix app && node --test tools/e2e/test/*.test.mjs && (cd tools/e2e && npm ci && npx playwright install chromium) && node tools/e2e/inventory.mjs --out tools/e2e/features/generated > "$RUNNER_TEMP/inventory.log" && node -e 'process.exit(JSON.parse(require("fs").readFileSync("tools/e2e/features/generated/summary.json","utf8")).total>=600?0:1)' && node tools/e2e/run.mjs --list > "$RUNNER_TEMP/list.jsonl" && node -e 'const L=require("fs").readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));process.exit(L.length===13&&L.every((r)=>r.scenarios>=3)?0:1)' "$RUNNER_TEMP/list.jsonl" && node tools/e2e/run.mjs --area page --shard 1/2 --displays 4k-dpr1 --budget 40m > "$RUNNER_TEMP/e2e.jsonl" && node -e 'const L=require("fs").readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));process.exit(L.length===1&&L.every((r)=>r.error===null&&r.servedHeadersMatch===true&&r.scenarios.length>0&&r.scenarios.every((s)=>s.skipped!==true))?0:1)' "$RUNNER_TEMP/e2e.jsonl" && node tools/e2e/coverage.mjs --features tools/e2e/features --run "$RUNNER_TEMP/e2e.jsonl" --out "$RUNNER_TEMP/coverage.json" && node -e 'process.exit(require("fs").existsSync(process.argv[1])?0:1)' "$RUNNER_TEMP/coverage.json"
```

判定の基準:
- 流れ A〜D(`welcome-flow.mjs` の 4 行)が、受け入れの式のとおり通る。`npm test --prefix app` と `node --test tools/e2e/test/*.test.mjs` が通る。
- `inventory.mjs --out tools/e2e/features/generated` が終了コード 0 で、`summary.json` の `total` が 600 以上。
- `run.mjs --list` が 13 行で、どの領域も見本 3 本以上。
- `run.mjs --area page --shard 1/2 --displays 4k-dpr1` が終了コード 0(印の無いシナリオは全て `ok`、各カウンタ 0、`skipped` が無い)。
- `coverage.json` が `$RUNNER_TEMP` に出る。
- `features/generated/` の機能の ID に重複が無い(重複は `coverage.mjs` が落とす)。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には「runner の値。この機械とは別枠」と書き、nproc と loadavg を添える。
- 「達成」「直った」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。出力の行を付ける。
- 報告に、機能の総数と、領域ごとの件数(`summary.json` の `perArea`)を書く。
- 一覧で名前が取れなかった項目の数と、その理由(例: メモリの文字列が見つからない)を書く。

## 制約

- 製品(`app/`・`lift/`)は変えない(読むだけ)。必要な窓の表・計測が既存の API に無い時は、報告に書いて止める。
- 製品に、利用者向けの新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足さない。テストのための分岐を製品に入れない。
- `.github/`・`tools/crash-hunt/`・`judge.jq` は変えない。
- Python を使わない。OCR を使わない(文字は、ゲストのメモリの文字列・窓の表・画素のハッシュで読む)。
- ベンダー名・Cloudflare の識別子(アカウント・バケット・ホスト名・アクセス制御の値)を、ファイルに書かない。
- sleep・timeout のコマンド・nohup・末尾の `&` は使わない。待つ時は条件で待つ。一時ファイルは `.tmp-*` に置き、終わったら消す(生成物の `features/generated/` は commit する)。
- 製品の作業は Actions で行う。この機械で製品のビルドや測定をしない。

## 作業の進め方

- 開始の時刻を、最初に `date -u` で記録する。4 時間 30 分を過ぎたら、受け入れが通らなくても、作業ツリーを commit し、報告を書いて終える。
- ビルドは約 35 分。フォアグラウンドで直接、シェルツールの `timeout` 引数に 5400000 を渡して待つ。背景で起動して待たない。
- blend2d の取得(GitHub)が 403 で落ちたら一時的。受け入れのコマンドは `lift/out-app/b2d-src` を消してから作り直すので、そのまま再試行する。
- 順序: (1) `run.mjs` の引数と `scenario.mjs`(見本 1 本で動かす)。(2) インベントリ(実機の FL の起動を含む)。(3) 集計と `.min`。(4) 13 領域の枠と見本 39 本。
- 作業の最後に、受け入れが通らなくても、作業ツリーを commit する。

## 他のタスクとの関係

- blocked_by: なし。
- #{{200}}〜#{{212}}(13 領域)は、この Issue が閉じるまで起動しない。それぞれ、この基盤の上にシナリオを足す。
- #{{165}}(回帰の集合)は並行に入る。衝突したら、`git pull --no-rebase --no-edit` で取り込み、`run.mjs` の衝突を最小の差分で直す。
- `tools/crash-hunt/`(停止だけを見る走査)は残す。この Issue は、それを置き換えない。
