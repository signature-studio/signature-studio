---
draft: "115"
title: "Browser のパネル(左の黒い領域)を FL と同じにする: 黒いのが描かれていないのか空の一覧なのかを切り分け、一覧に要るフォルダの階層を seed に入れ、中身は起動後に取る"
type: Task
milestone: "M4 FL に必要な物の配布"
parent: "012"
blocked_by: []
labels: ["resources", "render"]
size: L
priority: High
project: signature-studio/3
---

## 目的

依頼者の画面(本番)で、左の Browser のパネルが、ヘッダーの下から画面の下まで黒いまま。Packs・Current project・Plugin database・Channel presets などの一覧が出ない。FL の Browser は、起動後にフォルダを列挙して一覧を作ると推定される。

この Issue の目標は、DPR 1 の画面で、Browser のパネルの黒い割合を変更の前の半分以下にし、一覧の行が描かれること。併せて、起動前に取る量を増やさない。

## 前回までの試行で分かったこと

- 依頼者の実機の観察(2026-11-23。画面は物理画素 3240×1863、DPR 1.7647、部屋は 1836×1056 CSS px): 左の Browser のパネルの下の領域(ブラウザの一覧)が黒いまま。Channel rack・Playlist・Mixer の配置が崩れ、Mixer が左下に小さく出る。
- #{{107}} の前回の試行(run 45089194964)の測定: DPR 1 の `blackShare`(Browser の矩形の中で RGB が全て 16 以下の画素の割合)は 0.910。DPR 1.25・1.5・2 でも 0.912・0.912・0.914、乱数 1.77 で 0.903。DPR によらず同じ。DPR 1(DPI 96。前回の変更の影響が無い条件)でも黒いので、DPI の変更の前から在る。
- 前回の道具の矩形は、部屋の原点から x 0・y 580・400×470 CSS px(未確認。窓の表の Browser の窓と比べて確かめる)。
- Browser の一覧は、FL が `Data\Patches\`(17,782 件。626.6 MB)・`Data\Projects\`・ユーザーのデータのフォルダを列挙して作ると推定される(未確認)。seed に `Data\Patches\` が無いので、一覧が空の可能性がある。
- #{{105}}(言語パック・テーマ・テンプレート・デモの配信。閉じている)は `Data\Patches\` を入れていない。起動前に取る合計は 314,326,602 B。
- 黒いのが「描かれていない」のか「空の一覧を描いている」のかは、まだ切り分けていない。

## 読む物

- `lift/harness/seeds/`(seed の表・ファイルの seed。起動前の取得と `@lazy-file:` の仕組み)
- 起動後の取得の仕組み(`grep -rn "lazy-file" lift app/src` で探す)
- `lift/runtime/fle_win.c`・`fle_gdc.c`(窓の作成と描画)、ファイル列挙の runtime(`grep -rn "FindFirstFileW\|GetFileAttributes" lift/runtime` で探す)
- `app/src/engine/render-worker.ts`・`app/src/engine/desk.ts`(窓ごとのテクスチャ)
- `tools/boot-check/cdp.mjs`・`tools/boot-check/boot-bundle.mjs`
- `pm-work/findings/c2-split-measure.md`(過去の測定の手順)
- `git log --grep 1345`(#1345 の配信の変更)

## やること

0. 基準を測る(何かを変える前に行う)。
   - 道具 `tools/browser-panel/measure.mjs`(新規)を作る。引数は bundle の絶対パス。DPR 1・部屋 1836×1056 CSS px で起動し、1 行の JSON を出す: `{"dpr":1,"state","standIn","error","blackShare","browserComponents","elapsedMs","preBootBytes","browserWindow"}`。
   - `blackShare`: Browser の窓の矩形の中で、RGB が全て 16 以下の画素の割合(0〜1)。矩形は、窓の表(`window.__app` の `Guest top-level windows`)の Browser の窓の位置。窓の表に無ければ、部屋の原点から x 0・y 580・400×470 CSS px を使い、理由を報告に書く。
   - `browserComponents`: 同じ矩形の中で、背景色(矩形の四隅の色)でない画素の 8 近傍の連結成分のうち、16 画素以上のものの数(一覧の行に当たる)。
   - `elapsedMs`: ページの読み込みの開始から、state が最初に engine-idle になるまでの時間(ms)。
   - `preBootBytes`: engine-idle までに取得したファイルの応答の大きさの合計(B)。
   - 道具で測った値を `pm-work/findings/browser-baseline-115.json` に commit する(`{"blackShare","browserComponents","elapsedMs","preBootBytes"}`)。この値が「変更の前の値」になる。
1. 切り分け(最初の作業)。Browser の窓(窓の表で特定する)が作られているか・描かれているか・何を要求しているかを、窓の API の記録で調べる(`FindFirstFileW`・`GetFileAttributes`・`CreateWindowExW`・描画の呼び出し)。main の既存の記録は `window.__app.windowApiTrace(true)` で、起動の途中から有効になる。列挙が起動の前に起きる場合は、起動の前から有効にする仕組みを、この Issue で作る(既定では出ない形にする。`#{{107}}` の作業ツリーに同じ方法の例がある)。**黒いのが「描かれていない」のか「空の一覧を描いている」のかを、最初に切り分ける。**
2. 原因に応じて直す。
   - 列挙に要るフォルダの階層(ディレクトリの行。ファイルの本体は要らない)が無いことが原因なら、その階層を seed に入れる。件数と大きさは小さい。
   - フォルダの中身(プリセット等。626 MB)は、起動後の取得(`@lazy-file:` の仕組み)にする。起動前に取る合計を増やさない。
   - 描画の欠陥なら、runtime(`fle_gdc.c`・`fle_win.c`)か render Worker を直す。
   - 切り分けの結果を、どの原因で直したかと一緒に報告に書く。
3. 矩形の定義を確かめる。道具の矩形が Browser の窓の外を含んでいないか、窓の表の矩形と比べる。妥当でなければ、道具の定義を直し、直した内容を報告に書く。
4. 変更の後に、同じ道具で DPR 1 の値を測る。

## 受け入れ

```accept
npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false?0:1)' "$RUNNER_TEMP/boot.json" && node tools/browser-panel/measure.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/browser.json" && node -e 'const fs=require("fs");const r=JSON.parse(fs.readFileSync(process.argv[1],"utf8"));const b=JSON.parse(fs.readFileSync("pm-work/findings/browser-baseline-115.json","utf8"));const ok=r.error===null&&r.state==="engine-idle"&&r.standIn===false&&r.dpr===1&&Number.isFinite(r.blackShare)&&r.blackShare<=b.blackShare/2&&r.browserComponents>=10&&r.elapsedMs<=b.elapsedMs+500&&r.preBootBytes<=b.preBootBytes+1048576;process.exit(ok?0:1)' "$RUNNER_TEMP/browser.json" && npm test --prefix app
```

判定の基準(DPR 1 の 1 行。部屋 1836×1056 CSS px):
- 起動が正常(`error` が null・`state` が engine-idle・`standIn` が false)。
- `blackShare` が、`pm-work/findings/browser-baseline-115.json` の `blackShare` の半分以下。
- `browserComponents` が 10 以上(一覧の行が描かれている)。
- `elapsedMs` が、基準の `elapsedMs` に 500 ms を足した値以下。
- `preBootBytes` が、基準の `preBootBytes` に 1 MB(1,048,576 B)を足した値以下。
- 単体テスト(`npm test --prefix app`)が通る。

判定に使う基準の値は、この Issue の最初の commit(基準の commit)の値。変更の後の値は、同じ道具で測り直す。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には「runner の値。この機械とは別枠」と書き、nproc と loadavg を添える。
- 実測と推定を分ける。「達成」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。出力の行を付ける。
- 報告に書く: 基準の値(`blackShare`・`browserComponents`・`elapsedMs`・`preBootBytes`)と、変更の後の値、切り分けの結果(描かれていない/空の一覧を描いている)と、その根拠の窓の API の記録の行(`FindFirstFileW` の対象と戻り値)、seed に足した階層の一覧(件数と大きさ)、起動後の取得に移したフォルダ、矩形の定義を確かめた結果(窓の表との比較)。耳の確認は行わない。「黒くない」とは書かず、割合と連結成分の数で書く。

## 制約

- 製品に、利用者向けの新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足さない。
- 特定の拡大率・DPI・DPR の値を入れない(この Issue は DPR 1 で測る)。
- 製品の既定の動作を、測定のためだけに変えない。
- 起動前に取る合計を、基準に 1 MB を足した値より大きくしない。大きいフォルダの中身は、起動後の取得にする。
- リポジトリの外に、ファイルを作らない。
- ベンダー名と Cloudflare の識別子を書かない。
- sleep・timeout・nohup・末尾の `&` は使わない。一時ファイルは `.tmp-*` に置き、終わったら消す。
- Python は製品に使わない(調査と検証にだけ使う)。

## 作業の進め方

- engine の作り直しは約 30 分。ビルドは、フォアグラウンドで、シェルツールの `timeout` 引数に大きい値(例 3600000)を渡して待つ。
- 作業の最後に、受け入れが通らなくても、作業ツリーを commit する。基準の値(`browser-baseline-115.json`)は、最初の commit に入れる。

## 他のタスクとの関係

- blocked_by: なし。`#{{105}}`(閉じている)の後の main の版で行う。
- `#{{107}}`(物理画素の画面)と、`app/src` の描画の部分で、触る場所が重ならない形にする。重なれば、後に入る方が pull して直す。
- `#{{113}}`・`#{{114}}`・`#{{108}}` と、描画の経路(`app/src/engine/`)が重なりうる。衝突したら、作業の最後に `git pull --no-rebase --no-edit` で main を取り込み、衝突を直し、受け入れをもう一度通す。
- 前回の道具(`tools/feat-c1-hidpi/measure-size.mjs`)は main に無い(`#{{107}}` の作業ツリーにある)。この Issue の道具は `tools/browser-panel/` に新しく作る。
