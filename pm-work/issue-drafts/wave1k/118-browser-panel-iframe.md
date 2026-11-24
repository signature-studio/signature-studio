---
draft: "118"
title: "Browser のパネル: WebView2 の窓の面を、ページ側の iframe に写し、オフラインのページ(error-offline.html)を出す"
type: Task
milestone: "M4 FL に必要な物の配布"
parent: "012"
blocked_by: []
labels: ["resources", "render"]
size: XL
priority: High
project: signature-studio/3
---

## 目的

依頼者の目標は、Browser のパネルを FL と同じにすること。依頼者の画面(本番)で、左の Browser のパネルの下の領域(y 約 145〜1049、x 0〜363)が黒いまま。

Browser のパネルは、WebView2(Web のエンジンそのもの)の窓である。ブラウザ自体が Web のエンジンなので、この窓の面は、ページ側の `iframe` に写す(PM の決定)。依頼者の画面では、既定で選ばれているクラウドのタブ(オレンジの印)の中身が WebView2 に出るはずだが、その中身は黒いまま(推定。実測で確かめる)。

この Issue の目標は次の 3 つ。
1. Browser の窓の面を、ページ側の `iframe` に写し、`put_Bounds`(位置・大きさ)と `put_IsVisible`(表示)に従わせる。
2. FL が起動の直後に、クラウドのタブの中身(オフラインのページ `error-offline.html`)を出す。タブを押す操作の後だけに出るのでは足りない。
3. マウス・キーの入力は `iframe` が受け、FL へ流さない。

## 背景

以下は #{{115}}(#1355。完了)の実測。次の作業者は記録を読めないので、要点をここに書く。詳細は `pm-work/findings/webview2-calls-1355.md`。

- 黒い面の正体(実測): Browser の窓(`TWPControl`)の中の `TVectorPanel`(364×905)の子の `TEdgeBrowser`(Delphi の WebView2 のラッパー)。黒い面は x 0〜363、y 145〜1049(905 行)。ヘッダーとタブ(y 76〜144)は黒くない。
- runtime の WebView2 のモデル: `lift/runtime/fle_webview2.c`・`fle_webview2.h`(#1355)。`webview2loader.dll` の 4 つの export と、COM の環境・コントローラー・`ICoreWebView2`・設定を持つ。呼ばれたメソッドは、engine のログに `webview2: <名前> (<IF>) <引数>` の行で出る(上限 600 行)。モデルは何も描かない。
- 起動の中で FL が WebView2 に渡すのは、環境の作成、コントローラーの作成、設定、イベントの受け口(`add_*` 19 件)、領域(`put_Bounds`。最後の値は 0,0,364,905)、表示(`put_IsVisible` 1)、環境の userData。URL・HTML・スクリプト・メッセージは、起動の中では渡されない。`Navigate` も、起動の中では呼ばれない。
- 起動では、モデルが `about:blank` の 4 つのイベント(`NavigationStarting` → `SourceChanged` → `ContentLoading` → `NavigationCompleted`。IsSuccess TRUE)を、FL が登録した受け口へ呼ぶ(#1355)。この 4 つが実物の最初の `about:blank` に当たるかは未確認。
- **クラウドのタブ(起動の時点で選ばれている。guest 267,125 付近)を押すと、`Navigate` が 1 回呼ばれる**: `uri="file://<インストール先>\System\Internet\error-offline.html"`(82 単位)。その後、`put_Bounds` が 0,0,365,299 → 0,0,365,329 に変わる。`error-offline.html` は、インストーラの `System\Internet\error-offline.html`(`inputs/fl-installer-files.tsv` の 26934 行。中身は記録に無い)。**オフラインの時に出す、FL 自身のページ**。
- 他のタブ(波形・星など)を押すと、`put_IsVisible 0`(WebView2 を隠す)。`NavigateToString` は、どの操作でも呼ばれなかった。
- **最初の版の障害(#1355)**: `Navigate` にも `about:blank` と同じ 4 つのイベントを返す版では、クラウドのタブを押すと、`fire NavigationStarting` の後に次の行が出て、エンジンが終わった(`exited`)。`fle: indirect call: the window procedure 0x… lies in a page of a VirtualAlloc region that is not marked for running code: not called (decision 0119)`。FL の `NavigationStarting` の受け口が、Delphi の MakeObjectInstance の thunk(窓のプロシージャ)を呼び、その thunk のページが実行可能として記録されていなかった(`lift/runtime/fle_win.c` の `thunk_target`)。今の版は、このため `Navigate` の後のイベントを返さない(CHOICE)。この Issue は、イベントを返すので、この障害を先に直す(下の「やること」の 2)。
- 基準(変更の前。runner の値): `blackShare` 0.929158(guest の画面の黒の割合。`iframe` の無い版の値)、`elapsedMs` 9034〜9064(#1355 の版。#1355 の前の基準は 7751)、`preBootBytes` 314,326,602(#1355 より前の値。受け入れの基準は下の判定の基準の値)。受け入れの判定は、下の値で行う。
- **この runner(と、依頼者のブラウザ)は、FL のサーバーに通信できない**(名前解決の失敗 12007)。オンラインの内容は対象にしない。オフラインのページ(`error-offline.html`)だけを出す。
- **前提(PM が確認)**: resources の取り出し(`.github/workflows/agent.yml` の extract の行。`node tools/fl-installer-extract.mjs ... 'System\Languages\' 'Artwork\' 'Data\Templates\' 'Data\Demo projects\' 'Data\System\' 'Data\Projects\'`)の接頭辞に、`System\Internet\` は無い。作業者は `.github/` を変えない。`lift/out-app/host-extra/resources/` の下に `System/Internet/error-offline.html` が無い時は、無いことを報告に書いて終える(PM が接頭辞を足して再試行する)。

## 前回の試行で分かったこと

1 回目の試行(開始から約 3 時間)は `failed` で終わった。受け入れで落ちた理由は 1 つ(`preBootBytes`。下の 5)で、それ以外は全て通った。作業の差分は保存されていて、次の作業の開始時に作業ツリーに載っている(取り込みの結果は `.prior-work.md`)。載っていなければ、報告に書いて終える。

1. 測定(判定用の 3 回。runner の値)
   - `blackShare`(ページの画面): 基準 0.929158 → 0(3 回とも)。`guestBlackShare` は 0.929158 のまま(面は黒の上に載るので、guest の画面の黒は残る。判定は `blackShare`)。
   - `iframeCount` 1。`iframeRect` = `boundsRect` = (0,145,364,905)。`boundsMatch` true。
   - `elapsedMs` 11131・10689・11183(上限 12500 以内)。
   - 流れ A〜D: 4 行とも `error` null・`state` engine-idle・`standIn` false・`trapped` false・`hostErrors` 0・`opened` true。B と D は `welcomeClosed` true。
   - `npm test --prefix app`: 69 ファイル合格(1 件スキップ)、1344 件合格。
   - 目視(`shot4-after.png`): iframe の中に「You're offline」のボックスが CSS つきで出る。フォントは CSP の `font-src` で止まり、フォールバックの書体になる。
2. 起動の障害(#1355 の版の障害。前回の試行で直した)
   - 原因(推定。再現で確かめた): モデルが、FL が `add_*` で渡したハンドラに参照(`AddRef`)を足していなかった。FL が参照を手放すとハンドラが解放され、その thunk の領域が `VirtualFree` される。古い thunk を呼ぶと、`fle_call_guest: the callback address is 0` や `indirect call to 0xffff… is not a known function entry` で止まった。
   - 直し方: `lift/runtime/fle_webview2.c` の `hold_handler`。`add_*` の時に IUnknown の `AddRef` を呼ぶ。`remove_*` では戻さない(CHOICE)。直すと、`NavigationStarting`・`SourceChanged`・`ContentLoading`・`NavigationCompleted` が出て、エンジンは終わらない。`thunk_target` の判定は変えていない。解放の経路そのものは trace していない。
3. 写し方(前回の差分。やること 3 の「橋」は、この方式に替える)
   - 位置: `put_Bounds` の矩形に、窓の client の画面上の位置(`origin=X,Y`。`fle_win_client_origin`)を足し、CSS 画素にする。実測は origin 0,145・bounds 0,0,364,905 → iframe (0,145,364,905)。一覧の窓の origin は負(-5,139 など)のまま置き、`#desktop` で切る。
   - 押下の後の `put_Bounds`: 一時的に `origin=366,145` を経て、`Navigate` の後に `0,0,364,905 origin=0,145` へ戻る。面は最後の値に従い、判定は一致した。
   - CSP: アプリの CSP(`script-src 'self'`・`style-src 'self'`)が、`srcdoc` の中のインラインの `<style>`・`<script>` を止める。そのため、読み込み前に注入するスクリプトは動かない。iframe は `sandbox="allow-same-origin"`(スクリプトは動かさない)にし、親ページが読み込み後に、`<style>` を CSSOM(`adoptedStyleSheets`)に入れ、背景(#142028)を設定し、`window.chrome.webview` を `contentWindow` に作る。CSSOM の操作は CSP で止まらない。
   - `error-offline.html`: 44,729 B。自己完結で、相対の参照(`src`・`href`・`url()`)は無い。`blob:` への書き換えは要らなかった。前提の資源(`lift/out-app/host-extra/resources/System/Internet/error-offline.html`)は、前回の試行で在った。`fe-build` の pack は、ファイルが在る時だけ lazy 資源として足す(前回の差分)。
4. 押し方(測定の道具)
   - 起動の時点で選ばれているクラウドのタブ(267,125)を、そのまま押しても `Navigate` は出ない。別のタブ(35,125)を押してから 267,125 を押すと、`Navigate` が 1 回出る。測定の道具は、この順で押す。押す前に、Welcome 窓を Escape で閉じ、「Problems loading」を閉じる(前面に窓があると、押下が Browser に届かない)。
   - 押下の後の `webview2Fired` は 4 つのイベント。
5. 未確認・残り
   - `preBootBytes` の超過: 314,352,327(基準 314,330,698 を 21,629 B 超過)。内訳は engine.wasm 169,038,721・seed 66,702,132・DLL 77,444,112・glue 1,167,362。この変更が増やしたのは約 2.0 KB(wasm)と 717 B(glue)。残りの約 19 KB は、基準の後に main に入った変更によるもの(推定)。基準は下の判定の基準で直す(314,368,711 B)。
   - 目標 2(起動の中で `Navigate` が出ない理由)は、FL の起動の手続きを読んでいないので未確認。
   - 目標 3(入力の遮断)は、送って確かめていない。面の上の操作が `#desktop` に届かない、は仕組み上の推定。
   - 単一のコントローラーを前提にしている(`wv_view` は最初の view)。`PostWebMessageAsJson` は面に通知するだけで、`ExecuteScript` は実行しない。ページ → FL の `WebMessageReceived` は、モデルに無いので渡していない。

## 残りのやること

「やること」の 0〜6 は、前回の試行でほぼ済んでいる(上の節)。残りは次の順で行う。
1. 作業ツリーの前回の差分を確かめる(`git status`・`git diff --stat`。前回の記録は `pm-work/findings/browser-panel-118.md`。差分に載っている)。
2. 受け入れを通す(下の `accept` の 1 行を、そのまま実行する)。再ビルドと測定が要る(engine のビルドは約 35 分)。
3. 通っても通らなくても、作業ツリーを commit する(報告の規則どおり)。
4. 余裕があれば、目標 2・3 を確かめる。必須ではない。受け入れには入れない。確かめた結果を報告に書く。目標 2 は、FL の起動の手続きで `Navigate` を呼ぶ箇所を読む(材料は「未確認・残り」の節)。目標 3 は、面の上でクリックとキーを送って確かめる。

## 読む物

- `pm-work/findings/webview2-calls-1355.md`(#1355 の記録。全部)
- `lift/runtime/fle_webview2.c`・`fle_webview2.h`(モデル。`call_slot` の `K_NAVIGATE`・`K_NAV_HTML`・`K_PUT_RECT`・`K_PUT_VISIBLE`・`K_POST`・`K_EXEC`、`fire_navigation`、`complete`)
- `lift/runtime/fle_win.c`(`thunk_target`・`fle_win_try_thunk`・`call_wndproc`。decision 0119)、`lift/runtime/fle_sys.c`(`fle_win_prot`・`fle_sys_set_protection`・`fle_commit`。ページの保護の記録)、`lift/runtime/fle_sys.h`
- `app/src/engine/worker.ts`(stderr の `webview2:` 行の扱い。`onInput` と `case` の列)、`app/src/engine/protocol.ts`(`FromWorker`・`ToWorker`)、`app/src/engine/client.ts`、`app/src/main.ts`(`window.__app`・`engineLog()`)
- `app/src/ui/desktop.ts`(`DesktopView`。canvas の位置と大きさ。`scale` は CSS px per guest px)、`app/src/ui/screen-fit.ts`(#1347 の変換の規則: 1 guest px = 1 / devicePixelRatio CSS px)
- `app/src/engine/lazy-file.ts`・`app/src/engine/lazy-tree.ts`(起動の後の取得)、`app/src/engine/hostfiles.ts`(ホストのファイルの解放)
- `tools/browser-panel/measure.mjs`(今の測定の項目)、`tools/browser-panel/probe-navigate.mjs`(クラウドのタブの押し方と位置)、`tools/smoke/welcome-flow.mjs`(流れ A〜D)
- `inputs/fl-installer-files.tsv` の `System\Internet\` の行(26928〜26936 行付近。`error-offline.html` は 26934 行)

## やること

0. 作業ツリーの確認。`.prior-work.md` が在れば、最初に読む。`lift/out-app/host-extra/resources/` の下の `System/Internet/error-offline.html` が在るかを確かめ、報告に書く。無ければ、製品の変更は作らず、報告に書いて終える。
1. `error-offline.html` の中身を読む。相対の参照(画像・CSS・JS・リンク)、外部の参照(http・https)、スクリプトが使う API(`window.chrome.webview` など)を調べる。要点を `pm-work/findings/browser-panel-118.md` に書く(全文は載せない)。
2. engine の側(WebView2 のモデル → ページ、ページ → モデル)。
   - `Navigate`・`NavigateToString`・`put_Bounds`・`put_IsVisible`・`PostWebMessageAsJson`・`ExecuteScript` の呼び出しを、ページへ通知する。既存の engine のメッセージの仕組み(`app/src/engine/protocol.ts` の `FromWorker`)に足す。stderr の `webview2:` 行を Worker が読んで構造化する形か、モデルから直接送る形かは作業者が選び、理由を報告に書く。
   - ページの `iframe` の `load`・`postMessage` は、Worker へ返し、engine が idle の間に、モデルの受け口(`fire_navigation` 相当)を guest の関数として呼ぶ(`fle_call_guest`)。入力(`input`)の経路と同じ作りでよい。
   - 呼ぶ順序は実物の順序: `NavigationStarting` → `SourceChanged` → `ContentLoading` → `NavigationCompleted`(IsSuccess TRUE)。FL が登録している受け口だけを呼ぶ。
   - **thunk の障害を直す**: 上の背景の障害(`Navigate` の後の `NavigationStarting` の受け口が、窓のプロシージャを呼び、その thunk のページが実行可能でないと記録される)。そのページの保護の記録(`fle_win_prot`・`fle_sys_set_protection`・`fle_commit`)を調べ、実物の Windows の意味に合わせて直す。推定: 実物では、この thunk の領域は実行可能な保護で置かれる(確かめる)。直し方と理由を報告に書く。イベントを捨てて止めることはしない。
3. ページの側(`iframe`)。
   - 窓の面を、canvas の上に `iframe` で重ねる。位置・大きさは、`put_Bounds` の矩形を、コントローラーの窓(`CreateCoreWebView2Controller` の第 1 引数の `hwnd`。ログの `hwnd=`)の画面上の位置に足し、guest の画素から CSS 画素へ直したもの(× 1 / devicePixelRatio。`app/src/ui/desktop.ts` の `scale`)。窓の画面上の位置が取れなければ、runtime で取れる値を作る。
   - 表示は `put_IsVisible` に従う(0 なら隠す)。`put_Bounds` が変わったら、`iframe` を動かす(クラウドのタブを押すと、矩形が 365×299 → 365×329 に変わった)。
   - 中身: `error-offline.html` を、インストーラの実物(resources の木。資源は起動後の取得)から読み、`srcdoc` にする。ページは `file://` を直接読めないので、`srcdoc` の中の相対の参照(画像・CSS・JS)は、資源ごとの `blob:` URL に書き換える(`srcdoc` の相対 URL は、ページの URL に対して解決されるため、そのままでは読めない)。外部の参照(http・https)は読みに行かない(通信を増やさない)。
   - 橋: `window.chrome.webview.postMessage`(ページ → FL)と `PostWebMessageAsJson`(FL → ページ)を、`iframe` の中に、読み込み前に注入する小さなスクリプトで作る(`srcdoc` を組み立てる時に先頭へ足す)。
   - 入力: `iframe` の中のクリック・スクロール・キーは、`iframe` が受け、FL へ流さない。
   - 資源は、起動後の取得(`@lazy-file:` の仕組み)に入れる。起動前の取得を増やさない。取得の完了までは、`iframe` の `load` を遅らせる。
4. 既定のタブ。起動の中で `Navigate` が呼ばれない理由を調べ、FL が起動の直後に、クラウドのタブの内容を出すようにする。材料は次のとおり。
   - `about:blank` の 4 つのイベントの順序と、`NavigationCompleted` の `IsSuccess`・`WebErrorStatus`・`NavigationId`。
   - `QueryInterface` の応答。起動で FL が問い合わせた IID のうち、`E_NOINTERFACE` を返した物(記録の 44〜51 行の IID)。後の版のインターフェース(`ICoreWebView2_2` など)を FL が使うか。
   - タブの選択が、どの時点で `Navigate` を要求するか(FL の手続きの呼び出し元。engine のログと、必要なら trace)。
   - 公開の仕様(WebView2 の reference の、`ICoreWebView2` の各イベントの説明)を根拠に使う。仕様が決めない点は「未確認」と書く。
   - 原因が分からなければ、報告に「未確認」と書き、タブを押す操作を含めて受け入れを通す(下の受け入れの注意)。
5. 測定の道具 `tools/browser-panel/measure.mjs` を広げる(既存の項目は残す)。
   - 判定の前に、クラウドのタブ(guest 267,125。`probe-navigate.mjs` と同じ押し方)を 1 回押してから測る。起動の中で出ていれば、押しても害は無い。
   - `iframeCount`: ページの DOM の、WebView2 の面の `iframe` の数(この Issue が付ける目印の属性で数える)。
   - `iframeRect`: その 1 つ目の矩形(CSS 画素。ページの部屋の原点から。`{x, y, w, h}`)。無ければ null。
   - `boundsRect`: 最後の `put_Bounds` の矩形を、同じ座標系に直したもの(CSS 画素)。窓の左上が画面の外に出る場合(#1355 の記録では、一覧の窓が x -5 に在る)の扱いを、報告に書く。
   - `boundsMatch`: `iframeRect` と `boundsRect` の 4 つの値が、±2 CSS 画素以内で一致する時 true(`iframeCount` が 0 なら false)。
   - `blackShare`: **ページの実際の画面**(CDP の `Page.captureScreenshot` で、Browser の矩形 x 0・y 76・364×974 CSS 画素を切り出したもの)の画素で測る。黒の定義(R・G・B すべて 16 以下)と割合の定義は、既存の道具のまま。guest の画面の割合は `guestBlackShare` として残す(判定に入れない)。
   - `navigateCalls`・`webview2Fired`・`webview2Lines` は、既存のまま残す。
6. 測定の道具 `tools/smoke/welcome-flow.mjs` で、流れ A〜D を測る(この Issue の変更の後の版で)。
7. 受け入れを通し、作業ツリーを commit する(受け入れが通らなくても)。

## 受け入れ

```accept
npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false?0:1)' "$RUNNER_TEMP/boot.json" && mkdir -p .tmp-118 && B="$PWD/lift/out-app/bundle/raw/fl-Oz" && for i in 1 2 3; do node tools/browser-panel/measure.mjs "$B" > ".tmp-118/m$i.json" || exit 1; done && node -e 'const fs=require("fs");const R=[1,2,3].map((i)=>JSON.parse(fs.readFileSync(".tmp-118/m"+i+".json","utf8").trim()));const ok=(r)=>r.error===null&&r.state==="engine-idle"&&r.standIn===false&&r.dpr===1&&r.iframeCount>=1&&r.boundsMatch===true&&r.blackShare<=0.5&&r.elapsedMs<=12500&&r.preBootBytes<=314368711;process.exit(R.length===3&&R.every(ok)?0:1)' && node tools/smoke/welcome-flow.mjs "$B" > ".tmp-118/flow.jsonl" && node -e 'const fs=require("fs");const L=fs.readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const F=(f)=>L.find((r)=>r.flow===f);const ok=(r)=>!!r&&r.error===null&&r.state==="engine-idle"&&r.standIn===false&&r.trapped===false&&r.hostErrors===0&&r.opened===true;process.exit(L.length===4&&["A","B","C","D"].every((f)=>ok(F(f)))&&F("B").welcomeClosed===true&&F("D").welcomeClosed===true?0:1)' ".tmp-118/flow.jsonl" && rm -rf .tmp-118 && npm test --prefix app
```

判定の基準(DPR 1。測定の 3 回の全て):
- 起動が正常(`error` が null・`state` が engine-idle・`standIn` が false)。
- `iframeCount` が 1 以上。`boundsMatch` が true(`iframeRect` と `boundsRect` が ±2 CSS 画素以内)。
- `blackShare` が 0.5 以下(ページの実際の画面で測る。基準は 0.929158)。
- `elapsedMs` の 3 回の最大が 12500 以下。
- `preBootBytes` が 314,368,711 以下(前回の試行の実測 314,352,327 B に、余裕 16,384 B を足した値。起動前の取得を増やさない)。
- 流れ A・B・C・D(`tools/smoke/welcome-flow.mjs`): 各行が、`error` が null・`state` が engine-idle・`standIn` が false・`trapped` が false・`hostErrors` が 0・`opened` が true。流れ B と D は `welcomeClosed` が true。
- 単体テスト(`npm test --prefix app`)が通る。
- 足した物に、利用者向けの新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を書かない。

受け入れの注意: 既定のタブで `Navigate` が起動の中で呼ばれない(背景の 4)場合でも、測定の道具がクラウドのタブを押してから測るので、受け入れは通ってよい。ただし、その場合は依頼者の画面で、既定のタブを押す操作が要る、と報告に書く。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には「runner の値。この機械とは別枠」と書き、nproc と loadavg を添える。
- 実測と推定を分ける。「達成」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。出力の行を付ける。
- 報告に書く: (1) `error-offline.html` の有無と、中身の要点、(2) 起動の中で `Navigate` が出ない理由(材料と結論。分からなければ「未確認」)、(3) thunk の障害の原因と直し方、(4) 写し方(位置の求め方・`blob:` への書き換え・橋・入力)、(5) 測定の 3 回の `iframeCount`・`iframeRect`・`boundsRect`・`boundsMatch`・`blackShare`・`elapsedMs`・`preBootBytes`、(6) 流れ A〜D の JSON の 4 行。
- 「黒い面が消えた」「一覧が出た」とは書かない。見た目は、`blackShare` と `iframe` の値で言う。

## 制約

- 製品に、利用者向けの新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足さない。写す経路は既定の動作として入れる。
- 特定の拡大率・DPI・DPR の値を、runtime・app・設定・テストに、分岐や表として入れない。変換は式で書く。
- オンラインの内容(http・https の取得)を、製品の既定の動作にしない。通信を増やさない。
- `.github/`・`inputs/`・`guard/` は変えない(読むだけ)。`tools/browser-panel/measure.mjs` と `tools/smoke/welcome-flow.mjs` は、この Issue の測定として変えてよい(判定の基準は変えない)。
- ベンダー名と Cloudflare の識別子を書かない。
- sleep・timeout・nohup・末尾の `&` は使わない。一時ファイルは `.tmp-*` に置き、終わったら消す。
- Python は製品に使わない(調査と検証にだけ使う)。
- 製品の作業は Actions で行う。この機械で製品のビルドや測定をしない。

## 作業の進め方

- 最初に `date -u` で開始の時刻を記録する。4 時間 30 分を過ぎたら、受け入れが通っていなくても、作業ツリーを commit し、報告を書いて終える(ジョブの上限は 5 時間 50 分)。
- engine の作り直しは約 35 分。ビルドは、フォアグラウンドで、シェルツールの `timeout` 引数に大きい値(例 3600000。上限 5400000)を渡して待つ。背景で起動して待たない。
- 1 回の試行で、ビルドは数回まで。モデルの通知・ページの `iframe`・thunk の調査・測定の道具の変更は、まとめて 1 回のビルドで入れる。
- blend2d の取得(GitHub)が 403 で落ちたら一時的。受け入れのコマンドは `lift/out-app/b2d-src` を消してから作り直すので、そのまま再試行する。
- 作業の最後に、受け入れが通らなくても、作業ツリーを commit する。

## 他のタスクとの関係

- blocked_by: なし。
- `#{{115}}`(#1355。完了): WebView2 のモデルと記録。この Issue の土台。次の Issue は、その記録の後の描画(この Issue)。
- `#{{116}}`(#1357。完了): Welcome 窓を閉じる時の trap。流れ A〜D は、その修正の後の版で測る。
- `#{{105}}`(#1345): 言語・テーマ・テンプレート・デモの配信。`System\Internet\` の接頭辞は、その取り出しの一部(PM が `.github/workflows/agent.yml` に足す。前提の節を参照)。
- `#{{117}}`(#1358。F10 の General settings の測定)は、`app/src/engine/worker.ts` と `app/src/main.ts`(`window.__app`)の要求の経路を変える。衝突したら、作業の最後に `git pull --no-rebase --no-edit` で main を取り込み、衝突を直し、受け入れをもう一度通す。
