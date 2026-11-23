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

1 回目の試行(run 45146774112。切り分けの記録のみ。判定で落ちて `failed`)。製品の変更は無く、記録と測定の道具は commit されず push されていない。内容は artifact `agent-1355` の `work.diff` に在る。取り込みで作業ツリーに載る(`tools/browser-panel/measure.mjs`・`pm-work/findings/browser-baseline-115.json`・`pm-work/findings/browser-panel-1355.md`)。probe(trace を既定で有効にした一時ビルド)は、戻してある。

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

### 2 回目の試行(run 45198372697。判定で落ちて `failed`)

製品の変更を入れた。作業は artifact `agent-1355` の `work.diff`(約 100 KB)に保存された。次の試行では、作業ツリーに取り込みで載る(作業ツリーに `.prior-work.md` が在れば、先に読む)。載っている物:

- **WebView2 のモデル**(新規 `lift/runtime/fle_webview2.c`・`fle_webview2.h`): `webview2loader.dll` の export 4 つ(`CreateCoreWebView2EnvironmentWithOptions`・`CreateCoreWebView2Environment`・`GetAvailableCoreWebView2BrowserVersionString`・`CompareBrowserVersions`)、COM のインターフェース 4 つ(環境・コントローラー・`ICoreWebView2`・設定)、`NavigationCompletedEventArgs` の vtable の仮想スロット。呼ばれたメソッドを `webview2: <メソッド> (<インターフェース>) <引数>` の 1 行で engine のログに出す(上限 600 行)。
- **仮想モジュール**: `lift/runtime/fle_modules.c` に `webview2loader.dll`(N = 42)。`fle_load.c` の `fle_load_absent` から外した。`lift/crates/fe-lift/src/imports_table.rs` に応答の 4 行。`lift/runtime/fle_win.c` にスロットのフックの 1 行。
- **`lift/runtime/fle_oleaut.c`(製品の振る舞いの変更)**: `VariantClear` が VT_UNKNOWN の中身を解放する時、従来は止まっていた。今回は、ゲストの `IUnknown::Release`(vtable の 3 番目)を `fle_call_guest` で呼ぶ。起動を通すために必要だった(1 回目のビルドは、ここで止まった)。`SafeArray` の解放は従来どおり止まる。
- **測定の道具** `tools/browser-panel/measure.mjs`: `webview2Lines`・`webview2Methods` を足した。
- **テストの期待値**: モジュール数(41 → 42)と応答テーブルの行数を、新しい状態に合わせた(`modules`・`load`・`wall_fr5`・`wall_fr6`・`wall_fr13`・`wall_fr15`・`wall_fr16`・`msctf`・`deferred`・`wall_s`)。cargo テストの既存の失敗が 3 件あり、HEAD の時点から在る(`wall_fr4` の 2 件は `archive/harness-attic/` の fixture が無いため。`wall_fr19` の 1 件は `fle_appfile.c` の `fprintf(`)。受け入れには入っていない。
- **記録**: `pm-work/findings/webview2-calls-1355.md`(呼び出しの順序・出所・未確認)、`pm-work/findings/browser-panel-1355.md`、`pm-work/findings/browser-baseline-115.json`。

実測(runner の値。nproc 4。3 回の測定。受け入れは未達):
- 起動は正常(`error` が null・`engine-idle`・`standIn` が false)。`blackShare` は 0.9291581108829569 で変わらない(判定に入れない)。
- `webview2Lines` は 48(3 回とも)。`webview2Methods` は 34 種で、先頭は `CreateCoreWebView2EnvironmentWithOptions`。
- FL が渡した内容: 環境の作成(唯一の文字列は環境の `userData`: `C:\Users\user\AppData\Roaming\Image-Line\Edge\`)、コントローラーの作成、設定の `put_*` 3 件、イベントの受け口 `add_*` 19 件、領域 `put_Bounds` 5 回、表示 `put_IsVisible` 2 回、`AddHostObjectToScript` 1 回。
- **`Navigate`・`NavigateToString`・`ExecuteScript`・`PostWebMessage*` は、起動の中で呼ばれない。** アイドルの後 5 秒の測定でも 48 行のまま。URL・HTML・スクリプト・メッセージは、起動の経路では渡されていない。
- `elapsedMs` は 10874・10903・10926 ms(基準 7751 ms)。基準の runner では、変更の無い作業でも 10〜11 秒が出た例がある(#{{102}} の基準は同じ runner で 10993〜11093 ms)。揺れが大きいので、基準の相対値は使わない。増えた原因は未確認。
- 起動の後のログの末尾に `[js] fle thread 13 failed: HostFilesUnsupported ... pread on descriptor 85` が出た。#{{116}} の修正(`91b4c973`)は、この試行の後に main に入った。
- ニュースの取得は、runner では名前解決に失敗した(12007)。
- 仮説(未確認): 実物の WebView2 は、コントローラーの作成の後に最初の `about:blank` の `NavigationCompleted` を出す。このモデルは出さない。ラッパー(`TEdgeBrowser`)がこれを待つなら、`Navigate` が来ない理由になりうる。

落ちた理由: (b) `Navigate`・`NavigateToString` が出ない。(c) `elapsedMs` の最大が上限 9251 を超えた(10926)。`guard/scan.sh` が `lift/crates/fe-lift/tests/wall_fr5.rs` の 448・449 行で 32 桁の 16 進を検出した(HEAD の時点から在る UUID の検査の文字列。この変更で、このファイルが検査の対象になった)。作業者は規則を迂回する書き換えをしなかった。

## 読む物

- `lift/runtime/fle_load.c`(`fle_load_absent` と、`webview2loader.dll` の決定の記録。60〜72 行付近)。`f_be3520` が `LoadLibraryW(L"WebView2Loader.dll")` の NULL をどう扱うか。
- `lift/runtime/fle_win.c`(`call_wndproc`・既定の処理の `WM_PAINT`・`DefWindowProcW`・`BeginPaint`。`CreateWindowExW` の class の解決)。`LoadLibraryW`・`GetProcAddress` の実装(`grep -rn "GetProcAddress" lift/runtime` で探す)。
- `tools/browser-panel/measure.mjs`(前回の道具。土台にする)、`pm-work/findings/browser-panel-1355.md`(前回の切り分けの記録)、`pm-work/findings/browser-baseline-115.json`(基準)。
- `app/src/engine/render-worker.ts`・`app/src/engine/desk.ts`・`app/src/engine/client.ts`(次の Issue の描画の経路。読むだけ)。
- `git log --grep 1345`(#1345 の配信の変更)。

## やること

2 回目の試行で、1・2(最小のモデル)・4 は済んでいる(取り込みで載る)。残りは 7 と、2 の追加。0・3・5・6 は、そのまま行う。

0. 取り込みの確認。`.prior-work.md` が在れば、最初に読む(取り込みの結果と、次にやること)。`git status`・`git diff HEAD` で、2 回目の試行の製品の変更と測定の道具(上の「2 回目の試行」の節の 1 つ目から 4 つ目)が載っているかを見る。載っていなければ、報告に「取り込み無し」と書いて終える(製品の変更は作らない。取り込みの失敗は、次の PM が直す)。
1. WebView2 が無い時の FL の振る舞いを読む。`fle_load.c` の S-webview2-absent の扱いを読み、`TEdgeBrowser` が `WebView2Loader.dll` の NULL を受けた後に何をするかを、記録で確かめる(決めずに、記録で)。
2. 最小のモデルを作る。runtime に、`WebView2Loader.dll` を、仮想のモジュールとして読めるようにする(`LoadLibraryW` が、その名前に仮想のハンドルを返す)。
   - export は、FL が `GetProcAddress` で求めた物だけを作る(記録に出た物。候補は `CreateCoreWebView2EnvironmentWithOptions`・`CreateCoreWebView2Environment`・`GetAvailableCoreWebView2BrowserVersionString`・`CompareBrowserVersions`)。
   - COM のインターフェース(`ICoreWebView2Environment`・`ICoreWebView2Controller`・`ICoreWebView2`)の vtable を、C の静的な表で作る。
   - 呼ばれた全てのメソッドを、`webview2: <メソッド名> <引数の要約>` の 1 行で engine のログに出す。要約の例: `Navigate` の URL、`NavigateToString` の HTML の大きさと先頭 300 文字、`ExecuteScript` のスクリプトの先頭、`add_WebMessageReceived`・`PostWebMessageAsJson` の内容、`put_Bounds`・`put_IsVisible` の値。
   - 既定でログに出す(新しい旗を足さない)。件数が多ければ、先頭の数百行で打ち切る。
   - 完了のハンドラ(`ICoreWebView2CreateCoreWebView2ControllerCompletedHandler` など)は呼び出して、FL を先に進める。
   - FL が WebView2 の応答を待って固まる場合は、モデルを引っ込めて、元の「無い」の振る舞い(`fle_load_absent`)に戻す。起動を壊さない。`elapsedMs` を悪化させない。引っ込めたら、報告に書く。
    - **実物の WebView2 の起動の順序に合わせる**: コントローラーを作った後、最初の `about:blank` の読み込みに当たるイベントを、受け口に**呼び出して**(`fle_call_guest`)、FL を先に進める。呼ぶのは、`add_*` で受け口を登録した物のうち次の 4 つで、登録順ではなく、この順序で呼ぶ: `NavigationStarting` → `SourceChanged` → `ContentLoading` → `NavigationCompleted`(`IsSuccess` = TRUE)。`DOMContentLoaded` は FL が登録していない(記録に無い)ので、呼ばない。根拠は公開の仕様(Microsoft の WebView2 の reference の `ICoreWebView2` の各イベントの説明。`ContentLoading` は `NavigationStarting`・`SourceChanged` の後で `NavigationCompleted` の前、`SourceChanged` は `ContentLoading` の前)。ただし、仕様は最初の `about:blank` の移動にこれらが当たるかを書いていない。その点は未確認として、呼び出す物と根拠を `pm-work/findings/webview2-calls-1355.md` に書く。それでも `Navigate` が出ない場合は、**FL の側の契機**(Browser のパネルの表示・操作・タイマー・ニュースの取得)を調べる(Browser パネルの窓をクリックする、F8 でブラウザを出し入れする、など。`tools/feat-c6-popup-menus/` の道具が、窓の操作の方法を持つ)。
3. 記録。`pm-work/findings/webview2-calls-1355.md` に、次を書く。呼び出しの順序(先頭 50 行程度)、URL と HTML の出所(ファイルの読みか、FL が組み立てるか。組み立てるなら、どの手続きか)、スクリプトとメッセージの種類と件数、完了のハンドラの呼び出し順。
4. 測定の道具。`tools/browser-panel/measure.mjs` の 1 行の JSON に、`webview2Lines`(engine のログの `webview2:` の行の数)と `webview2Methods`(現れたメソッド名の、初出の順の配列。重複なし)を足す。既存の項目(`blackShare` など)は残す。判定には入れない。
5. 次の Issue の根拠。報告に、FL が渡す内容は、ローカルのファイルか、組み立てた HTML か、どの種類のメッセージが FL → ページ・ページ → FL に流れるかを書く。描画(ページ側の iframe への写し)は次の Issue で行う。
6. 作業ツリーを commit する(受け入れが通らなくても)。
7. 残り(2 回目の試行の続き。ここから始める)。
    - (a) 0 の取り込みの確認を行う。
    - (b) `lift/crates/fe-lift/tests/wall_fr5.rs` の 448・449 行の、2 つの 32 桁の 16 進の文字列(`expected_uuid(0)`・`expected_uuid(1)` の期待値)が、`guard/scan.sh` の cfhex で検出される。元の意味を保ったまま、ダッシュ区切りの GUID の形(8-4-4-4-12。文字の並びは変えない)で書き、テストの側でダッシュを外して比べる。`guard/scan.sh` の規則は変えない・回避しない。
    - (c) 2 の追加の発火を入れる。`NavigationCompleted` などを、`webview2: fire <イベント名> (<インターフェース>) <引数>` の 1 行で、engine のログに出す。測定の道具 `tools/browser-panel/measure.mjs` の JSON に、`webview2Fired`(`fire` の行のイベント名の、初出順の配列。重複なし。判定に入る)と、`navigateCalls`(`Navigate`・`NavigateToString`・`ExecuteScript`・`PostWebMessageAsJson`・`PostWebMessageAsString` のどれかを呼んだ `webview2:` の行の件数。判定に入れない)を足す。`webview2Methods` からは `fire` の行を除く。
    - (d) 受け入れを通す。`navigateCalls` が 0 なら、`pm-work/findings/webview2-calls-1355.md` に、FL の側の契機を調べた結果(試した操作と結果)を書く。
    - (e) 作業ツリーを commit する(受け入れが通らなくても)。

## 受け入れ

```accept
npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false?0:1)' "$RUNNER_TEMP/boot.json" && mkdir -p .tmp-115 && B="$PWD/lift/out-app/bundle/raw/fl-Oz" && for i in 1 2 3; do node tools/browser-panel/measure.mjs "$B" > ".tmp-115/m$i.json" || exit 1; done && node -e 'const fs=require("fs");const R=[1,2,3].map((i)=>JSON.parse(fs.readFileSync(".tmp-115/m"+i+".json","utf8").trim()));const ok=(r)=>r.error===null&&r.state==="engine-idle"&&r.standIn===false&&r.dpr===1&&r.webview2Lines>=10&&r.webview2Methods.includes("CreateCoreWebView2EnvironmentWithOptions")&&r.webview2Fired.includes("NavigationCompleted");process.exit(R.every(ok)&&Math.max(...R.map((r)=>r.elapsedMs))<=12500&&require("fs").existsSync("pm-work/findings/webview2-calls-1355.md")?0:1)' && if [ -f tools/smoke/welcome-flow.mjs ]; then node tools/smoke/welcome-flow.mjs "$B" > ".tmp-115/flow.jsonl" && node -e 'const L=require("fs").readFileSync(".tmp-115/flow.jsonl","utf8").trim().split("\n").map((l)=>JSON.parse(l));const a=L.find((r)=>r.flow==="A");process.exit(!!a&&a.error===null&&a.state==="engine-idle"&&a.trapped===false&&a.hostErrors===0&&a.opened===true?0:1)'; fi && rm -rf .tmp-115 && npm test --prefix app
```

判定の基準(DPR 1。3 回の測定の全て):
- 起動が正常(`error` が null・`state` が engine-idle・`standIn` が false)。
- `webview2Lines` が 10 以上。`webview2Methods` に `CreateCoreWebView2EnvironmentWithOptions` が在り、`webview2Fired` に `NavigationCompleted` が在る(モデルが受け口へ呼んだ記録。engine のログの `webview2: fire NavigationCompleted` の行による)。`navigateCalls`(件数)は判定に入れない。
- `elapsedMs` の 3 回の最大が 12500 以下(絶対値。基準の相対値は使わない。runner の揺れが大きい)。
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
- `#{{116}}`(Welcome 窓を閉じる時の trap)は main に入った(`91b4c973`・`f1d2bbf2`・`5f8ba910`)。受け入れの流れ A(`tools/smoke/welcome-flow.mjs`)は、今回から効く。
- 次の Issue(描画: 窓の面を、ページ側の iframe に写す)は、この Issue の記録の後に起票する。
- `#{{113}}`(拡大率の測定)と、`lift/runtime/fle_win.c` の変更が重なりうる。衝突したら、作業の最後に `git pull --no-rebase --no-edit` で main を取り込み、衝突を直し、受け入れをもう一度通す。

