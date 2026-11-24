# #1362 Browser パネルの WebView2 の面を iframe に写す(記録)

実エンジン: この runner で、受け入れのコマンドの全体を 2 回通した(`ACCEPT_RC=0` の 2 回。2 回目は戻り番地の行を足した版)。その後、同じ engine の bundle で `tools/browser-panel/measure.mjs`(3 回)・`tools/smoke/welcome-flow.mjs`(流れ A〜D)・`tools/browser-panel/probe-input.mjs`(最後の版)を走らせ、エンジンのログ(`webview2:` 行)とページの画面・要素の当たり判定を観測した。生成コード(`lift/out-app/gen/src/fle-10.c` など)で、Navigate の呼び出し元を読んだ。

runner の値(この機械とは別枠): nproc 4。loadavg は測定の間に 1.7〜2.1(1 分平均)。

## 0. 判定の項目

| 項目 | 判定 | 出力 |
| --- | --- | --- |
| 起動(state・error・standIn) | 受理 | 測定 3 回とも `engine-idle`・`error` null・`standIn` false |
| `iframeCount` ≥ 1 | 受理 | 3 回とも 1 |
| `boundsMatch` が true | 受理 | 3 回とも true(`iframeRect` = `boundsRect` = (0,145,364,905)) |
| `blackShare` ≤ 0.5(ページの画面) | 受理 | 3 回とも 0(基準 0.929158) |
| `elapsedMs` の最大 ≤ 12500 | 受理 | 6822・6866・7059 |
| `preBootBytes` ≤ 314,368,711 | 受理 | 3 回とも 310,590,056 |
| 流れ A・B・C・D | 受理 | 4 行とも `error` null・`engine-idle`・`standIn` false・`trapped` false・`hostErrors` 0・`opened` true。B と D は `welcomeClosed` true |
| `npm test --prefix app` | 受理 | Test Files 69 passed・1 skipped(70)。Tests 1351 passed・10 skipped(1361) |
| 目標 1(iframe を `put_Bounds`・`put_IsVisible` に従わせる) | 受理 | 上の `boundsMatch`・`iframeCount` と、下の 4 |
| 目標 2(起動の直後に、クラウドのタブの中身を出す) | **未確認・未達** | 起動の中では `Navigate` が出ない。タブを押すと `Navigate` が 1 回出て、`error-offline.html` を表示する(受け入れの注意の範囲)。下の 2 |
| 目標 3(入力は iframe が受け、FL へ流さない) | 受理(probe) | 面の上の押下と打鍵は iframe の文書だけが受け、`#desktop` の入力の監視と document には届かない。下の 8 |

## 1. `error-offline.html`(有無と中身)

- 有無: 在る。`lift/out-app/host-extra/resources/System/Internet/error-offline.html`(44,729 B、97 行)。`agent.yml` の取り出しの接頭辞 `System\Internet\` が効いている。`fe-build bundle` の lazy 資源として `error-offline.html`(44,729 B。起動の後の取得)に入った。
- 中身の要点: 自己完結の HTML。`<style>` に `@font-face`(フォントは `data:` URI。約 40 KB)とボックス(`.popup`)の CSS。本文は「You're offline」(見出し)、「FL Cloud needs an internet connection to browse and download sounds.」、「Browse already downloaded content」(リンク `href="#offlineData"`)。アイコンはインラインの SVG。
- 相対の参照(`src`・`href`・`url()`): 無い(`href="#offlineData"` は文書内の参照)。外部の参照(http・https): 無い。スクリプト: 無い。`window.chrome.webview` の利用: 無い。
- したがって、書き換えられる参照は無い(`rewriteRefs` は、この資源では何も書き換えない。単体テストで確かめた)。フォントの `data:` は CSP の `font-src` で止まり、フォールバックの書体になる。

## 2. 起動の中で `Navigate` が出ない理由(材料と結論)

- 観測:
  - 起動の中の `webview2:` 行は、コントローラーの作成と設定・受け口の登録・`put_Bounds`・`put_IsVisible`・`about:blank` の 4 つのイベントまで。`Navigate` は無い。
  - タブ(35,125 を押した後に 267,125 を押す)で、`Navigate (ICoreWebView2) uri="file://…\System\Internet\error-offline.html" units=82` が 1 回出る。その直後に `caller (ICoreWebView2) at=0xbe77eb stack=…` の行(下の 2 の 3)。
  - 前回の試行の記録: 267,125 を押しただけ(別のタブを先に押さない)では、`Navigate` は出なかった。
- 生成コード(`lift/out-app/gen/src/fle-10.c` など。guest の VA)で読んだ呼び出し元:
  1. `Navigate` の呼び出し: `0xbe77e8`(`call [R3+0x28]`。ICoreWebView2 の 6 番目の slot = `Navigate`。この slot の VA は、モデルの slot の範囲に入る)。関数 `f_be7750`(小さな関数: `[this+0x494]` の COM に、引数の uri を渡す)の中。
  2. `f_be7750` の呼び出し元は 2 つ: `f_bf6120`(`0xbf6152`)と `f_be6040`(`0xbe67ee`)。
  3. 今回の押下で使われたのは `f_bf6120`(`caller` 行の戻り番地 `0xbf6157` と一致)。`f_bf6120` は引数を文字列に直して `f_be7750` を呼ぶだけで、条件を持たない。その呼び出し元は `f_db0460`(`0xdb0608`)。`f_db0460` は `f_da9f80(…)` の結果が 0 のときだけ `f_bf6120` を呼ぶ(`0xdb05e3` の `jne`)。`f_da9f80` の意味は未確認。
  4. `f_be6040` は、`[obj+0x4b4]` の文字列の長さが 0 より大きいときだけ `f_be7750` を呼ぶ(`0xbe67d8`〜`0xbe67e2`)。この経路は今回は使われていない。
  5. スタックの戻り番地の列(`caller` 行の `stack=`)のうち、生成コードのラベルと合うもの: `0xbe77eb`(`f_be7750`)→ `0xbf6157`(`f_bf6120`)→ `0x4149ff`(`f_4149e0`)→ `0xdb060d`(`f_db0460`)。それより上は、ラベルと合わない値があり、解決していない。
- 結論: **未確認**。起動の中で `Navigate` が出ない理由は、FL の起動の手続きの上位(`f_db0460` を呼ぶ側と `f_da9f80` の判定)を読まないと決まらない。推定: 起動の時点では、`f_da9f80` の判定が「呼ぶ」にならない(または `f_be6040` の文字列の欄が空の)状態であり、タブを押すと、その状態が変わる。FL の起動の手続きの全体は、この Issue では読んでいない。

## 3. thunk の障害の原因と直し方

- 障害(#1355): `NavigationStarting` の受け口が、窓のプロシージャ(Delphi の `MakeObjectInstance` の thunk)を呼ぶと、そのページが実行可能として記録されておらず、`fle_win.c` の `thunk_target` が止めた(decision 0119)。
- 保護の記録の確認: `fle_sys_set_protection`・`fle_commit`・`fle_win_prot` を読んだ。VirtualAlloc の確保と保護は、ページごとの `PAGE_*` 値として記録される。`thunk_target` は 0x10・0x20・0x40・0x80(実行可能)だけを通す。判定そのものは正しい(実行できない領域の thunk を通すと、ゲストのコードが解放済みのメモリを実行することになる)。失敗の文には、領域の基底・大きさ・確保時の保護・ページの値を足した(診断)。
- 原因(推定): モデルは、`add_*` で登録されたハンドラに COM の参照(`AddRef`)を足さなかった。FL が自分の参照を手放すと、ハンドラが解放され、その thunk の領域が後の確保に再利用される。再利用された領域のページは実行可能でないので、判定が止める。解放の経路そのものは trace していない(推定)。実物の Delphi の `MakeObjectInstance` の thunk は実行可能な保護で確保される、と見られる(推定。ここでは確かめていない)。
- 直し方: `lift/runtime/fle_webview2.c` の `hold_handler`。`add_*`(`K_ADD`・`K_ADD_EV`)で、渡されたハンドラの `IUnknown::AddRef`(vtable の 1 番目)を呼ぶ。`remove_*` では戻さない(CHOICE: 実行の終わりまで保持する。漏れは小さく、解放後の呼び出しは起きない)。
- 直した後: `Navigate` の後に `NavigationStarting`・`SourceChanged`・`ContentLoading`・`NavigationCompleted` が出て、エンジンは終わらない(測定 3 回・流れ A〜D・probe の全て。`webview2: fire NavigationStarting` の後は、`GetIterator`・`MoveNext` と続く)。
- `thunk_target` の判定は変えていない。イベントを捨てて止めてはいない。

## 4. 写し方(位置・大きさ・表示・資源・文書・橋・入力)

- 位置と大きさ: `put_Bounds` の矩形に、窓の client の画面上の位置(`fle_win_client_origin` → `put_Bounds … origin=X,Y`)を足し、`DesktopView.currentScale`(CSS px per guest px)を掛ける(`app/src/ui/webview2-surface.ts` の `faceRect`)。実測: `put_Bounds 0,0,364,905 origin=0,145` → iframe `(0,145,364,905)`(`#stage` の原点から)。窓の origin が負(一覧の `origin=-5,139`)のときも式のまま置き、`#desktop` の `overflow: hidden` で切る。`boundsRect` は、この式で最後の `put_Bounds` から求めた矩形(切らない)。
- 表示: `put_IsVisible 0` で枠(`wrap`)を `display: none`。他のタブの押下で `isVisible=0` が出る経路で確認。
- 資源: `Navigate` の uri `file://…\System\Internet\error-offline.html` を、lazy 資源 `resources/System/Internet/error-offline.html` に直す(`app/src/engine/webview2-log.ts` の `resourceOf`)。資源は `fe-build bundle` の `--lazy-file`(`lift/crates/fe-build/src/main.rs` の `OFFLINE_PAGE_SRC`。在るときだけ入る)。起動の後の取得なので、起動前の取得は増えない(`preBootBytes` は下の 7)。
- 文書: `<style>` を文書から外し、`CSSStyleSheet`(`adoptedStyleSheets`)に入れる(CSP の `style-src 'self'` が、srcdoc の中のインラインの `<style>` を止めるため)。背景は `#142028`(文書の CSS の注記の色。FL の色ではない。推定)。実測: `iframeBackground` = `rgb(20, 32, 40)`、`iframeSrcdocLength` = 1,684。
- 橋: `window.chrome.webview` を親が、文書の読み込み後に iframe の window へ作る(`postMessage` は親へ、`addEventListener('message')` は `PostWebMessageAsJson` の受け口)。CSP の `script-src 'self'` が srcdoc の中のスクリプトを止めるので、文書は script を持たない(`sandbox="allow-same-origin"`)。`PostWebMessageAsJson` は、この版では呼ばれず、観測していない。
- 入力: 面(iframe)は、押下と打鍵を自分の文書で受ける。`#desktop` の入力の監視(`attachDesktopInput`)には届かない(下の 8)。
- 取得の順: 押下 → `Navigate`(保留)→ 資源を取る(完了まで待つ)→ `srcdoc` → `load` → `webview2-loaded`(Worker)→ `fle_g3_wv2_loaded` → エンジンの idle で 4 つのイベント(`fle_wv2_pump`)。

## 5. 測定(`measure.mjs`。3 回。最後の版)

| 回 | state | error | blackShare(ページ) | guestBlackShare | iframeCount | iframeRect | boundsRect | boundsMatch | elapsedMs | preBootBytes | navigateCalls |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 1 | engine-idle | null | 0 | 0.9291581108829569 | 1 | (0,145,364,905) | (0,145,364,905) | true | 6822 | 310,590,056 | 1 |
| 2 | engine-idle | null | 0 | 0.9291581108829569 | 1 | (0,145,364,905) | (0,145,364,905) | true | 6866 | 310,590,056 | 1 |
| 3 | engine-idle | null | 0 | 0.9291581108829569 | 1 | (0,145,364,905) | (0,145,364,905) | true | 7059 | 310,590,056 | 1 |

- `webview2Fired`(3 回とも): `NavigationStarting`・`SourceChanged`・`ContentLoading`・`NavigationCompleted`。
- `guestBlackShare`(コンポジタの画面の黒)は、基準と同じ 0.929158。面は黒の上に載るので変わらない(判定に入れない)。
- `browserWindow` は `fallback`(窓の表に Browser が無い。前回と同じ)。`blackShare` は、ページの画面の `Browser` の矩形(x 0・y 76・364×974 CSS px)で測る。

## 6. 流れ A〜D(`welcome-flow.mjs`。JSON の 4 行)

```
{"flow":"A","error":null,"state":"engine-idle","standIn":false,"trapped":false,"hostErrors":0,"welcomeClosed":true,"opened":true,"elapsedMs":6985,"preBootBytes":310590056,"hostErrorText":null,"imageReadsAfterIdle":[],"steps":["first idle at 6981 ms (title \"Welcome to FL Studio - in ?trial mode?\")","Welcome window at (481, 241) 874x573","No thanks at guest (993, 548)","Default row at guest (716, 358), 2 click(s)","end: state engine-idle, title \"Problems loading the project\", welcome window closed"]}
{"flow":"B","error":null,"state":"engine-idle","standIn":false,"trapped":false,"hostErrors":0,"welcomeClosed":true,"opened":true,"elapsedMs":6918,"preBootBytes":310590056,"hostErrorText":null,"imageReadsAfterIdle":[],"steps":["first idle at 6915 ms (title \"Welcome to FL Studio - in ?trial mode?\")","Welcome window at (481, 241) 874x573","Escape","end: state engine-idle, title \"Problems loading the project\", welcome window closed"]}
{"flow":"C","error":null,"state":"engine-idle","standIn":false,"trapped":false,"hostErrors":0,"welcomeClosed":true,"opened":true,"elapsedMs":6850,"preBootBytes":310590056,"hostErrorText":null,"imageReadsAfterIdle":[],"steps":["first idle at 6847 ms (title \"Welcome to FL Studio - in ?trial mode?\")","Welcome window at (481, 241) 874x573","No thanks at guest (993, 548)","Default row at guest (716, 358), 1 click(s)","end: state engine-idle, title \"Problems loading the project\", welcome window closed"]}
{"flow":"D","error":null,"state":"engine-idle","standIn":false,"trapped":false,"hostErrors":0,"welcomeClosed":true,"opened":true,"elapsedMs":6978,"preBootBytes":310590056,"hostErrorText":null,"imageReadsAfterIdle":[],"steps":["first idle at 6973 ms (title \"Welcome to FL Studio - in ?trial mode?\")","Welcome window at (481, 241) 874x573","I'm in at guest (841, 548)","after I'm in the Welcome window is still listed","Default row at guest (716, 358), 2 click(s)","end: state engine-idle, title \"Problems loading the project\", welcome window closed"]}
```

## 7. `preBootBytes`

- 3 回とも 310,590,056 B(受け入れの上限 314,368,711 B に対して、約 3.8 MB の余裕)。
- 前回の試行の 314,352,327 B からの差(約 3.76 MB)は、engine の wasm の大きさの差と見られる(このビルドの `fe-build bundle` の表示は 165,104,932 B。前回の内訳の wasm は 169,038,721 B)。この差の原因は未確認。この Issue の変更の増分は、前回の記録では約 2 KB(wasm)と 717 B(glue)。
- 資源の取得: `error-offline.html` は起動の後の取得(lazy)で、起動前の取得には入らない。

## 8. 目標 3(入力)の確認(`tools/browser-panel/probe-input.mjs`。最後の版の 1 回)

| 段 | 押したもの | `#desktop` の監視 | `document` の監視(capture) | 面(iframe の文書) |
| --- | --- | --- | --- | --- |
| control-pointer | `#desktop` の上の点(`elementFromPoint` で、`#desktop` の中・面の外を確かめた。CANVAS) | +1 | +1 | 0 |
| control-key | 打鍵(押下の後、`#desktop` が焦点) | +1 | 0 | 0 |
| face-pointer | 面の中央(IFRAME) | 0 | 0 | +1 |
| face-key | 打鍵(面が焦点) | 0 | 0 | +1 |

- 読み: 面の上の押下と打鍵は面の文書だけが受ける。`#desktop` の入力の監視(`attachDesktopInput` の `el` の `pointerdown`・`keydown`)は、これらを受けない。対照(`#desktop` の上の操作)は届く。probe の別の版(対照の点が `#desktop` の外に当たった版)では、対照も届かなかった。それは probe の点の取り方の誤りで、面の結果とは無関係(記録)。
- 注意(推定): 焦点が面へ移るとき、親の `window` の `blur` が走り、`release()` が押下中のキーと押下中のボタンの解放を FL へ送る(押していなければ何も送らない)。これは面の入力を FL へ流すのではなく、既存の動作。
- `input:` の行はエンジンの右ボタンの測定用で、入力の受け取りの指標には使えない(`fle_input.c`)。

## 9. 戻り番地の行(診断。目標 2 の材料)

- 追加: `Navigate` のたびに、`webview2: caller (ICoreWebView2) at=<戻り番地> stack=<スタックの上の値のうち、イメージの範囲(0x400000〜0x13000000)のもの>`。`lift/runtime/fle_webview2.c`。判定には使わない(`webview2Methods` に `caller` が入る)。
- 1 回目の見え方: `at=0xbe77eb`。上の 2 の 3 の材料になった。

## 10. 未確認・残り

- 目標 2(起動の直後にクラウドのタブの中身を出す): 上の 2 の結論のとおり、未確認。次の段は、`f_da9f80` の判定の意味、`f_db0460` を呼ぶ上位の関数(スタックの戻り番地の列の上位)、`f_be6040` の呼び出し元。
- 依頼者の画面: 受け入れの注意のとおり、既定のタブ(267,125)を押す操作が要る(目標 2 が未達のため)。
- `PostWebMessageAsJson`・`ExecuteScript` は、この版では呼ばれず観測していない。`ExecuteScript` は実行しない(CHOICE)。`WebMessageReceived`(ページ → FL)は、モデルに無いので渡していない。
- 単一のコントローラーを前提にしている(`wv_view` は最初の view)。
- `remove_*` で参照を戻さない(CHOICE)。
- `blob:` の URL は解放していない(資源の数だけ。この資源には無い)。
- `elapsedMs` は、前回の試行の 10,689〜11,183 ms より小さい(今回 6,822〜7,059 ms)。原因は未確認。
