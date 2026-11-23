---
draft: "115"
title: "Browser のパネル: FL が WebView2(TEdgeBrowser)に渡す内容を、最小の COM のモデルで記録する"
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

依頼者の目標は、Browser のパネルを FL と同じにすること。依頼者の画面(本番)で、左の Browser のパネルの下の領域(y 約 145〜1049、x 0〜363)が黒いまま。Packs・Current project・Plugin database・Channel presets などの一覧が出ない。

前回の切り分けで、黒い面の正体は、Browser の一覧を載せる WebView2(Web の表示部品)の窓(`TEdgeBrowser`)だと分かった。WebView2 はブラウザのエンジンそのものなので、この部品の中身は、ページ側の要素(iframe など)に写すのが筋である(PM の決定)。この Issue は、その第 1 段である。FL が WebView2 に渡す内容(URL・HTML・スクリプト・メッセージ)を、最小の COM のモデルで記録する。描画(ページ側への写し)は、次の Issue で行う。

この Issue の目標は、(1) FL が WebView2 に渡す内容を、engine のログに全て出すこと、(2) 次の Issue(描画)の設計の根拠を出すこと。黒い割合(`blackShare`)の判定は、この Issue には入れない。

## 前回までの試行で分かったこと

前回の試行(run 45146774112。切り分けの記録のみ。判定で落ちて `failed`)。製品の変更は無く、記録と測定の道具は commit されず push されていない。内容は artifact `agent-1355` の `work.diff` に在る。取り込みで作業ツリーに載る(`tools/browser-panel/measure.mjs`・`pm-work/findings/browser-baseline-115.json`・`pm-work/findings/browser-panel-1355.md`)。probe(trace を既定で有効にした一時ビルド)は、戻してある。

- **黒い面の正体(実測)**: Browser の窓(lid 49 `TWPControl`)の中の `TVectorPanel`(lid 107、0x7f9901ac、364×905)の子の、**`TEdgeBrowser`(lid 108、0x7f9901b0、364×905)**。`TEdgeBrowser` は Delphi の WebView2 の wrapper。
- 黒い面の位置: `black full-width rows (x 0..363): [[145,1049]]`。905 行 = 364×905 で、黒い割合 905/974 = 0.929158 と一致する。ヘッダーとタブ(y 76〜144)は黒くない。
- **WebView2 の読み込みが失敗している**: `[err] missing-path: LoadLibraryW 126 "WebView2Loader.dll"`(`TEdgeBrowser` の生成時)。`lift/runtime/fle_load.c` の `fle_load_absent` に `webview2loader.dll` が在る(decision 0126 wall 86、S-webview2-absent。意図的に「無い」と扱っている。代案として、読み込んで COM の応答を作る案が記録に挙がっている)。
- **描画**: `TEdgeBrowser` の窓は、背景ブラシが無く、`WM_PAINT` は既定の処理(`DefWindowProcW`)で消去するだけ。`BeginPaint` は呼ばれない。一覧の窓 lid 107(`TVectorPanel`)は、`WM_PAINT` の後に、`BeginPaint` も既定の処理も呼ばない。
- **列挙は件数を返している**: `enum-dir: 18 …\Mixer presets\*`・`71 …\Speech\*`・`0 …\Plugin database\*`。前提「フォルダの列挙が一覧を作る」は当たらない。seed には `Data\Patches\` の階層が在る。
- **除外した候補**: `WM_SETREDRAW`(lid 49 は 0 の後に 1 を受けて釣り合う。lid 108 の `no_redraw` は 0)。表示(`SWP_SHOWWINDOW` と `ShowWindow` の記録が在る)。
- **FL の画面に、一覧の中身は無い**: FL の資源にある HTML は壁紙だけ。一覧の中身は、WebView2 に渡されている(推定)。その内容は、まだ分かっていない。
- **基準(変更の前。runner の値。1 回目: nproc 4、loadavg 1.48 1.21 1.67)**: `blackShare` 0.929158、`browserComponents` 1、`elapsedMs` 7751(2 回目 7797。負荷の高い回は 10716)、`preBootBytes` 314,326,602。矩形は x 0・y 76・364×974(道具の定義。窓の表に Browser は無い。子窓のため)。
- **作業者が決めなかったこと(前回)**: WebView2 の扱い。(a) runtime に WebView2 の完全なモデル(描画を含む)を作る、(b) 窓の面を、ページ側の要素で置き換える、(c) 一覧は空のまま。今回は、(b) の第 1 段として、FL が渡す内容を記録する。
- 黒い割合を背景色で塗って下げる変更はしない。一覧の行が描かれないため、割合が下がっても目標を満たさない。

## 読む物

- `lift/runtime/fle_load.c`(`fle_load_absent` と、`webview2loader.dll` の決定の記録。60〜72 行付近)。`f_be3520` が `LoadLibraryW(L"WebView2Loader.dll")` の NULL をどう扱うか。
- `lift/runtime/fle_win.c`(`call_wndproc`・既定の処理の `WM_PAINT`・`DefWindowProcW`・`BeginPaint`。`CreateWindowExW` の class の解決)。`LoadLibraryW`・`GetProcAddress` の実装(`grep -rn "GetProcAddress" lift/runtime` で探す)。
- `tools/browser-panel/measure.mjs`(前回の道具。土台にする)、`pm-work/findings/browser-panel-1355.md`(前回の切り分けの記録)、`pm-work/findings/browser-baseline-115.json`(基準)。
- `app/src/engine/render-worker.ts`・`app/src/engine/desk.ts`・`app/src/engine/client.ts`(次の Issue の描画の経路。読むだけ)。
- `git log --grep 1345`(#1345 の配信の変更)。

## やること

0. 取り込みの確認。`git status`・`git diff HEAD` で、前回の 3 ファイル(上の「前回までの試行で分かったこと」)が載っているかを見る。無ければ、基準を `tools/browser-panel/measure.mjs` で測り、基準の 1 回目の値(`blackShare` 0.929158 など)と揃えてから進める。
1. WebView2 が無い時の FL の振る舞いを読む。`fle_load.c` の S-webview2-absent の扱いを読み、`TEdgeBrowser` が `WebView2Loader.dll` の NULL を受けた後に何をするかを、記録で確かめる(決めずに、記録で)。
2. 最小のモデルを作る。runtime に、`WebView2Loader.dll` を、仮想のモジュールとして読めるようにする(`LoadLibraryW` が、その名前に仮想のハンドルを返す)。
   - export は、FL が `GetProcAddress` で求めた物だけを作る(記録に出た物。候補は `CreateCoreWebView2EnvironmentWithOptions`・`CreateCoreWebView2Environment`・`GetAvailableCoreWebView2BrowserVersionString`・`CompareBrowserVersions`)。
   - COM のインターフェース(`ICoreWebView2Environment`・`ICoreWebView2Controller`・`ICoreWebView2`)の vtable を、C の静的な表で作る。
   - 呼ばれた全てのメソッドを、`webview2: <メソッド名> <引数の要約>` の 1 行で engine のログに出す。要約の例: `Navigate` の URL、`NavigateToString` の HTML の大きさと先頭 300 文字、`ExecuteScript` のスクリプトの先頭、`add_WebMessageReceived`・`PostWebMessageAsJson` の内容、`put_Bounds`・`put_IsVisible` の値。
   - 既定でログに出す(新しい旗を足さない)。件数が多ければ、先頭の数百行で打ち切る。
   - 完了のハンドラ(`ICoreWebView2CreateCoreWebView2ControllerCompletedHandler` など)は呼び出して、FL を先に進める。
   - FL が WebView2 の応答を待って固まる場合は、モデルを引っ込めて、元の「無い」の振る舞い(`fle_load_absent`)に戻す。起動を壊さない。`elapsedMs` を悪化させない。引っ込めたら、報告に書く。
3. 記録。`pm-work/findings/webview2-calls-1355.md` に、次を書く。呼び出しの順序(先頭 50 行程度)、URL と HTML の出所(ファイルの読みか、FL が組み立てるか。組み立てるなら、どの手続きか)、スクリプトとメッセージの種類と件数、完了のハンドラの呼び出し順。
4. 測定の道具。`tools/browser-panel/measure.mjs` の 1 行の JSON に、`webview2Lines`(engine のログの `webview2:` の行の数)と `webview2Methods`(現れたメソッド名の、初出の順の配列。重複なし)を足す。既存の項目(`blackShare` など)は残す。判定には入れない。
5. 次の Issue の根拠。報告に、FL が渡す内容は、ローカルのファイルか、組み立てた HTML か、どの種類のメッセージが FL → ページ・ページ → FL に流れるかを書く。描画(ページ側の iframe への写し)は次の Issue で行う。
6. 作業ツリーを commit する(受け入れが通らなくても)。

## 受け入れ

```accept
npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false?0:1)' "$RUNNER_TEMP/boot.json" && mkdir -p .tmp-115 && B="$PWD/lift/out-app/bundle/raw/fl-Oz" && for i in 1 2 3; do node tools/browser-panel/measure.mjs "$B" > ".tmp-115/m$i.json" || exit 1; done && node -e 'const fs=require("fs");const R=[1,2,3].map((i)=>JSON.parse(fs.readFileSync(".tmp-115/m"+i+".json","utf8").trim()));const ok=(r)=>r.error===null&&r.state==="engine-idle"&&r.standIn===false&&r.dpr===1&&r.webview2Lines>=10&&r.webview2Methods.includes("CreateCoreWebView2EnvironmentWithOptions")&&(r.webview2Methods.includes("Navigate")||r.webview2Methods.includes("NavigateToString"));process.exit(R.every(ok)&&Math.max(...R.map((r)=>r.elapsedMs))<=9251&&require("fs").existsSync("pm-work/findings/webview2-calls-1355.md")?0:1)' && if [ -f tools/smoke/welcome-flow.mjs ]; then node tools/smoke/welcome-flow.mjs "$B" > ".tmp-115/flow.jsonl" && node -e 'const L=require("fs").readFileSync(".tmp-115/flow.jsonl","utf8").trim().split("\n").map((l)=>JSON.parse(l));const a=L.find((r)=>r.flow==="A");process.exit(!!a&&a.error===null&&a.state==="engine-idle"&&a.trapped===false&&a.hostErrors===0&&a.opened===true?0:1)'; fi && rm -rf .tmp-115 && npm test --prefix app
```

判定の基準(DPR 1。3 回の測定の全て):
- 起動が正常(`error` が null・`state` が engine-idle・`standIn` が false)。
- `webview2Lines` が 10 以上。`webview2Methods` に `CreateCoreWebView2EnvironmentWithOptions` が在り、`Navigate` か `NavigateToString` が在る(engine のログの `webview2:` の行による)。
- `elapsedMs` の 3 回の最大が 9251 以下(基準 7751 + 1500 ms)。
- `pm-work/findings/webview2-calls-1355.md` が在る。
- `tools/smoke/welcome-flow.mjs` が在る時だけ、その流れ A が通る(`#{{116}}` が入った後。無い時は、この項は無し。報告に、無かったことを書く)。
- 単体テスト(`npm test --prefix app`)が通る。
- `blackShare` は判定に入れない。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には「runner の値。この機械とは別枠」と書き、nproc と loadavg を添える。
- 実測と推定を分ける。「達成」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。出力の行を付ける。
- 報告に書く: `webview2:` の呼び出しの要点(メソッドの種類と順序、`webview2Lines`、`webview2Methods`)、URL・HTML・スクリプト・メッセージの出所、WebView2 の応答を引っ込めたかどうか、`elapsedMs` の 3 回の値、FL が渡す内容の結論(次の Issue の設計の根拠)。
- 「一覧が描かれた」とは書かない。この Issue は記録だけで、描画は次の Issue で行う。

## 制約

- 製品に、利用者向けの新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足さない。
- `webview2:` の記録は、既定でログに出す(新しい旗を足さない)。
- 製品の既定の動作を、測定のためだけに変えない。WebView2 のモデル自体は、既定の動作として入る(起動を壊さないこと)。
- 特定の拡大率・DPI・DPR の値を入れない。
- 黒い割合を背景色で塗って下げる変更をしない。
- ベンダー名と Cloudflare の識別子を書かない。
- sleep・timeout・nohup・末尾の `&` は使わない。一時ファイルは `.tmp-*` に置き、終わったら消す。
- Python は製品に使わない(調査と検証にだけ使う)。
- 製品の作業は Actions で行う。この機械で製品のビルドや測定をしない。

## 作業の進め方

- engine の作り直しは約 35 分。1 回の試行で、ビルドは数回まで。trace と記録は、まとめて 1 回のビルドで入れる。
- ビルドは、フォアグラウンドで、シェルツールの `timeout` 引数に大きい値(例 3600000)を渡して待つ。
- blend2d の取得(GitHub)が 403 で落ちたら一時的。受け入れのコマンドは `lift/out-app/b2d-src` を消してから作り直すので、そのまま再試行する。
- 作業の最後に、受け入れが通らなくても、作業ツリーを commit する。

## 他のタスクとの関係

- blocked_by: なし。
- `#{{116}}`(Welcome 窓を閉じる時の trap)が入った後に、受け入れの流れ A を行う(上の受け入れ)。
- 次の Issue(描画: 窓の面を、ページ側の iframe に写す)は、この Issue の記録の後に起票する。
- `#{{113}}`(拡大率の測定)と、`lift/runtime/fle_win.c` の変更が重なりうる。衝突したら、作業の最後に `git pull --no-rebase --no-edit` で main を取り込み、衝突を直し、受け入れをもう一度通す。
