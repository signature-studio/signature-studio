# #1355 WebView2 の呼び出しの記録(FL が WebView2 に渡す内容)

実エンジン: この runner で `fe-build engine`・`fe-build bundle` を作り、`tools/boot-check/boot-bundle.mjs`・`tools/browser-panel/measure.mjs`・`tools/smoke/welcome-flow.mjs`・`tools/browser-panel/probe-navigate.mjs` を走らせ、エンジンのログ(`window.__app.engineLog()`)の `webview2:` 行と、窓の操作の結果を観測した。ビルドは 5 回(履歴は「8」)。

runner の値(この機械とは別枠。nproc 4。loadavg は測定の終わりに出た値)。

## 0. 受け入れの判定の項目

| 項目 | 結果 | 出力 |
| --- | --- | --- |
| 起動(boot-bundle) | 受理 | `state` engine-idle・`error` null・`standIn` false・`elapsedMs` 9143 |
| `webview2Lines` ≥ 10 | 受理 | 3 回とも 63 |
| `webview2Methods` に `CreateCoreWebView2EnvironmentWithOptions` | 受理 | 3 回とも在る |
| `webview2Fired` に `NavigationCompleted` | 受理 | `["NavigationStarting","SourceChanged","ContentLoading","NavigationCompleted"]` |
| `elapsedMs` の最大 ≤ 12500 | 受理 | 8678・8691・8759 |
| 本ファイルが在る | 受理 | (このファイル) |
| 流れ A(welcome-flow) | 受理 | A・B・C・D の全て `error` null・`engine-idle`・`trapped` false・`hostErrors` 0・`opened` true |
| `npm test --prefix app` | 受理 | Test Files 68 passed・1 skipped。Tests 1335 passed・10 skipped |
| `blackShare`(判定に入れない) | 未判定 | 0.9291581108829569(基準 0.929158 と同じ。描画は次の Issue) |
| `navigateCalls`(判定に入れない) | 未判定 | 起動の中は 0(下の 5 で、タブを押すと 1 回出ることを観測) |

## 1. 測定(measure.mjs。3 回)

| 回 | state | error | standIn | blackShare | browserComponents | elapsedMs | webview2Lines | navigateCalls |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 1 | engine-idle | null | false | 0.9291581108829569 | 1 | 8678 | 63 | 0 |
| 2 | engine-idle | null | false | 0.9291581108829569 | 1 | 8691 | 63 | 0 |
| 3 | engine-idle | null | false | 0.9291581108829569 | 1 | 8759 | 63 | 0 |

基準(変更の前): elapsedMs 7751。今回は絶対値で判定する(基準の相対値は使わない)。

## 2. 呼び出しの順序(起動。先頭 50 行。全 63 行のうち)

```
CreateCoreWebView2EnvironmentWithOptions (WebView2Loader) browser="" userData="C:\Users\user\AppData\Roaming\Image-Line\Edge\" options=0 handler=…
AddRef (ICoreWebView2Environment) refs=2
AddRef (ICoreWebView2Environment) refs=3
CreateCoreWebView2Controller (ICoreWebView2Environment) hwnd=0x7f990118 handler=…
AddRef (ICoreWebView2Controller) refs=2
get_CoreWebView2 (ICoreWebView2Controller) -> ICoreWebView2 0x…
get_Settings (ICoreWebView2) -> ICoreWebView2Settings 0x…
put_AreDevToolsEnabled (ICoreWebView2Settings) value=0
put_AreDefaultContextMenusEnabled (ICoreWebView2Settings) value=4294967295
QueryInterface (ICoreWebView2Settings) iid={fdb5ab74-…} -> E_NOINTERFACE
QueryInterface (ICoreWebView2Controller) iid={c979903e-…} -> E_NOINTERFACE
QueryInterface (ICoreWebView2Controller) iid={f9614724-…} -> E_NOINTERFACE   (3 回)
QueryInterface (ICoreWebView2Controller) iid={97d418d5-…} -> E_NOINTERFACE
AddHostObjectToScript (ICoreWebView2)
put_AreDevToolsEnabled (ICoreWebView2Settings) value=0
put_AreDefaultContextMenusEnabled (ICoreWebView2Settings) value=0
QueryInterface (ICoreWebView2Settings) iid={11cb3acd-…} -> E_NOINTERFACE
put_IsStatusBarEnabled (ICoreWebView2Settings) value=0
add_NavigationStarting (ICoreWebView2) handler=… token=1
add_NavigationCompleted (ICoreWebView2) handler=… token=2
add_SourceChanged (ICoreWebView2) handler=… token=3
add_HistoryChanged (ICoreWebView2) handler=… token=4
add_ContentLoading (ICoreWebView2) handler=… token=5
add_DocumentTitleChanged (ICoreWebView2) handler=… token=6
add_NewWindowRequested (ICoreWebView2) handler=… token=7
add_FrameNavigationStarting (ICoreWebView2) handler=… token=8
add_FrameNavigationCompleted (ICoreWebView2) handler=… token=9
add_WebResourceRequested (ICoreWebView2) handler=… token=10
add_ScriptDialogOpening (ICoreWebView2) handler=… token=11
add_PermissionRequested (ICoreWebView2) handler=… token=12
add_ProcessFailed (ICoreWebView2) handler=… token=13
add_WebMessageReceived (ICoreWebView2) handler=… token=14
add_ContainsFullScreenElementChanged (ICoreWebView2) handler=… token=15
add_WindowCloseRequested (ICoreWebView2) handler=… token=16
add_ZoomFactorChanged (ICoreWebView2Controller) handler=… token=17
add_MoveFocusRequested (ICoreWebView2Controller) handler=… token=18
add_AcceleratorKeyPressed (ICoreWebView2Controller) handler=… token=19
add_GotFocus (ICoreWebView2Controller) handler=… token=20
add_LostFocus (ICoreWebView2Controller) handler=… token=21
put_Bounds (ICoreWebView2Controller) bounds=0,0,212,871
fire NavigationStarting (ICoreWebView2) uri="about:blank"
AddRef (ICoreWebView2NavigationStartingEventArgs) refs=2
get_Uri (ICoreWebView2NavigationStartingEventArgs) -> "about:blank"
get_RequestHeaders (ICoreWebView2NavigationStartingEventArgs) -> ICoreWebView2HttpRequestHeaders 0x…
GetIterator (ICoreWebView2HttpRequestHeaders) -> ICoreWebView2HttpHeadersCollectionIterator 0x…
MoveNext (ICoreWebView2HttpHeadersCollectionIterator) -> 0
Release (ICoreWebView2HttpHeadersCollectionIterator) refs=0
Release (ICoreWebView2HttpRequestHeaders) refs=0
Release (ICoreWebView2NavigationStartingEventArgs) refs=1
fire SourceChanged (ICoreWebView2) isNewDocument=1
fire ContentLoading (ICoreWebView2) isErrorPage=0
fire NavigationCompleted (ICoreWebView2) isSuccess=1 webErrorStatus=0
get_IsSuccess (ICoreWebView2NavigationCompletedEventArgs) -> 1
get_WebErrorStatus (ICoreWebView2NavigationCompletedEventArgs) -> 0
get_Source (ICoreWebView2) -> "about:blank"
put_Bounds (ICoreWebView2Controller) bounds=0,0,364,871
put_Bounds (ICoreWebView2Controller) bounds=0,0,364,905
put_Bounds (ICoreWebView2Controller) bounds=0,0,364,917
put_Bounds (ICoreWebView2Controller) bounds=0,0,364,905
put_IsVisible (ICoreWebView2Controller) isVisible=1
put_IsVisible (ICoreWebView2Controller) isVisible=1
```

(`…` は値の省略。全文は測定の標準エラーに出る。)

`webview2Methods`(初出順): `CreateCoreWebView2EnvironmentWithOptions, AddRef, CreateCoreWebView2Controller, get_CoreWebView2, get_Settings, put_AreDevToolsEnabled, put_AreDefaultContextMenusEnabled, QueryInterface, AddHostObjectToScript, put_IsStatusBarEnabled, add_NavigationStarting, add_NavigationCompleted, add_SourceChanged, add_HistoryChanged, add_ContentLoading, add_DocumentTitleChanged, add_NewWindowRequested, add_FrameNavigationStarting, add_FrameNavigationCompleted, add_WebResourceRequested, add_ScriptDialogOpening, add_PermissionRequested, add_ProcessFailed, add_WebMessageReceived, add_ContainsFullScreenElementChanged, add_WindowCloseRequested, add_ZoomFactorChanged, add_MoveFocusRequested, add_AcceleratorKeyPressed, add_GotFocus, add_LostFocus, put_Bounds, get_Uri, get_RequestHeaders, GetIterator, MoveNext, Release, get_IsSuccess, get_WebErrorStatus, get_Source, put_IsVisible`。

## 3. 完了のハンドラと発火の順序(モデルの仕様。実測は「呼ばれた順」)

- 環境の完了のハンドラ(`Invoke(hr, environment)`)の中で `CreateCoreWebView2Controller(hwnd)` を呼ぶ。コントローラーの完了のハンドラ(`Invoke(hr, controller)`)の中で、`get_CoreWebView2`・`get_Settings`・設定の `put_*`・受け口の `add_*`(21 件)・`put_Bounds` を呼ぶ。`AddHostObjectToScript` は 1 回。
- コントローラーの完了のハンドラが戻ったあと、モデルが `about:blank` の 4 つのイベントを、登録された受け口へ、この順で呼ぶ: `NavigationStarting` → `SourceChanged` → `ContentLoading` → `NavigationCompleted`(IsSuccess TRUE)。FL の `NavigationStarting` の受け口は、引数の `get_Uri`・`get_RequestHeaders`(空の反復子)を呼んで戻る。`NavigationCompleted` の受け口は `get_IsSuccess`・`get_WebErrorStatus`・`get_Source` を呼ぶ。
- 「about:blank の最初の移動に、この 4 つのイベントが当たるか」は、公開の仕様では確かめられない(未確認。仕様の順序だけを使った)。

## 4. URL・HTML・スクリプト・メッセージの出所

- URL: 起動の経路では `Navigate` は呼ばれない。FL が WebView2 に渡す文字列は、起動では環境の `userData`(`C:\Users\user\AppData\Roaming\Image-Line\Edge\`。FL が組み立てる。組み立ての手続きは未確認)と、`browser=""`(空)だけ。
- 窓のタブを押すと、`Navigate` が呼ばれる(下の 5)。URL は `file://C:\Program Files\Image-Line\FL Studio 21\System\Internet\error-offline.html`(82 単位)。FL のインストール先の中のファイルで、`inputs/fl-installer-files.tsv` の 26934 行に在る(中身は未取得)。URL は FL が自分のインストール先から組み立てている(組み立ての手続きは未確認)。
- HTML: `NavigateToString` は、どの操作でも呼ばれなかった。
- スクリプト: `ExecuteScript` と `AddScriptToExecuteOnDocumentCreated` は、どの操作でも呼ばれなかった。
- メッセージ: `PostWebMessageAsJson`・`PostWebMessageAsString` は呼ばれない。`add_WebMessageReceived` で受け口を登録しただけ。FL → ページ、ページ → FL のメッセージは、観測されていない。
- ホストのオブジェクト: `AddHostObjectToScript` で、FL の Delphi のオブジェクトを 1 つ渡す(起動の 1 回)。ページのスクリプトが使う経路だが、ページのスクリプトは観測されていない。

## 5. FL 側の契機(tools/browser-panel/probe-navigate.mjs。試した操作と結果)

起動の後、次の操作を順に行い、各段の `webview2:` 行を数えた(最後の回。全段 `engine-idle`):

| 段 | 操作 | webview2 行 | navigate | 観測 |
| --- | --- | --- | --- | --- |
| idle30 | 30 秒、入力なし(タイマー・ニュースの取得) | 0 | 0 | 変化なし |
| escape | Escape(Welcome 窓を閉じる。流れ B と同じ) | 0 | 0 | 「Problems loading the project」の窓が出た(既定のプロジェクトの一部が無いという通知) |
| dismiss | Escape(その窓を閉じる) | 0 | 0 | 窓が消えた。Browser の領域が見える |
| tab 35,125 | 最初のタブのアイコン(波形) | 1 | 0 | `put_IsVisible isVisible=0`(WebView2 を隠す) |
| tab 91,125 | 2 番目のタブ | 0 | 0 | 変化なし |
| tab 151,125 | 3 番目のタブ | 0 | 0 | 変化なし |
| tab 208,125 | 4 番目のタブ | 0 | 0 | 変化なし |
| tab 267,125 | クラウドのタブ(起動の時点で選ばれている。画面の黒い面) | 5 | **1** | 下の列 |
| tab 325,125 | 星のタブ | 1 | 0 | `put_IsVisible isVisible=0` |
| f8 | F8 | 0 | 0 | 画面が暗くなり、「Generators」の表示が出た(F8 の意味は未確認) |

クラウドのタブの段の `webview2:` 行(全文):

```
put_IsVisible (ICoreWebView2Controller) isVisible=1
put_Bounds (ICoreWebView2Controller) bounds=0,0,365,299
get_Source (ICoreWebView2) -> "about:blank"
Navigate (ICoreWebView2) uri="file://C:\\Program Files\\Image-Line\\FL Studio 21\\System\\Internet\\error-offline.html" units=82
put_Bounds (ICoreWebView2Controller) bounds=0,0,365,329
```

観測: クラウドのタブは WebView2 を表示し、`Navigate` で FL の `error-offline.html` を指す(この runner は通信できないので、オンラインの内容が読まれたかは未確認)。タブを押しただけでは、`NavigationStarting` 以降のイベントは、この版のモデルでは返さない(下の 6)。

## 6. 障害の記録と、モデルの引っ込め(CHOICE)

- 最初の版(`Navigate` にも `about:blank` と同じ 4 つのイベントを返す版)で、クラウドのタブを押すと、`fire NavigationStarting` の後に次の行が出て、エンジンが終わった(`exited`)。

```
webview2: fire NavigationStarting (ICoreWebView2) uri="file://…error-offline.html"
fle: indirect call: the window procedure 0x15979340 lies in a page of a VirtualAlloc region that is not marked for running code: not called (decision 0119)
```

  その直後に、FL が窓のプロシージャ(Delphi の `MakeObjectInstance` の thunk)を呼ぼうとして、そのページはランタイムが実行を許さないため止められた(`lift/runtime/fle_win.c` の `thunk_target`。決定 0119)。どの受け口が呼んだかは未確認(行の順からは `NavigationStarting` の後)。
- 変更の前は、Browser の窓は WebView2 を持たず、タブを押しても何も起きなかった。最初の版のままだと、利用者がクラウドのタブを押すとエンジンが終わるので、出さない。
- 引っ込めた範囲: `Navigate`・`NavigateToString` の後は、イベントを返さない(URL・HTML を記録するだけ。画面は変わらない)。起動の `about:blank` の 4 つのイベントは残す(受け入れが要求する `NavigationCompleted` の発火)。この扱いは CHOICE(作業者の判断)。次の Issue で、thunk の実行の扱い(ランタイムの VirtualAlloc の保護)を先に直すか、イベントを返さない設計を続けるかを決める。
- 引っ込めたあとの確認: 同じ探索(5 の全段)で、全段 `engine-idle`・最後の状態 `engine-idle`・`error` null。流れ A〜D も通る。

## 7. WebView2 が無い時の FL の振る舞い(記録。決めない)

- `lift/runtime/fle_load.c` の `fle_load_absent`(decision 0126 wall 86、S-webview2-absent): `webview2loader.dll` を「無い」と扱っていた。FL の `f_be3520` は `LoadLibraryW(L"WebView2Loader.dll")` の NULL を受けて 0 を返す(決定 0126 の記録)。その後 `TEdgeBrowser` は WebView2 の関数を持たないまま生成され、窓は何も描かない(`browser-panel-1355.md` の 1〜3)。
- 今回の変更で、`webview2loader.dll` は仮想のモジュール(N = 42)になり、`fle_load_absent` から外れた。

## 8. ビルドの履歴(engine の作り直し。この runner)

1. 前回の版(起動の確認が `error`): `get_RequestHeaders` が `E_NOTIMPL` → FL が止まる。
2. ヘッダーの `get_RequestHeaders` を `S_OK` にした版: `GetIterator` が `E_NOTIMPL` → 止まる。
3. 反復子(空)を足した版: `get_Source` の文字列を `malloc` で作っていた → FL の `CoTaskMemFree` が「ランタイムの割り当てではない」と止める(`exited`)。
4. 文字列を `CoTaskMemAlloc`(`fle_ole.c` の stub)で作る版: 起動は `engine-idle`。
5. 最終版(`Navigate`・`NavigateToString` の後はイベントを返さない): 起動は `engine-idle`。測定 3 回と流れ A〜D は、この版で採った。

(途中で `fe-build engine` が、既存の `lift/out-app/b2d-src` で失敗した回が 1 回ある。その時は古いエンジンで bundle を作ったので、その回の起動の結果は採らない。)

## 9. 次の Issue(描画)の根拠(推定を含む)

- 起動の経路で、FL が WebView2 に渡すのは「環境、コントローラー、設定、受け口、領域、表示、ホストのオブジェクト」。HTML・スクリプト・メッセージは渡されない(実測)。
- 黒い面は、選ばれた状態のクラウドのタブの WebView2 の面である(実測: 起動の時点で選ばれ、`Navigate` は起動では呼ばれない)。タブを押すと、FL は `file://…System\Internet\error-offline.html` へ `Navigate` する(実測)。この HTML の中身は未取得。`inputs/fl-installer-files.tsv` に在るので、取得して読める(推定)。
- 次の Issue で「ページ側の iframe に写す」設計にするなら、(a) `error-offline.html` の中身と、それが読む資源(相対の参照)、(b) クラウドの窓の内容が、オンラインで何を読むか(この runner では通信できないので未確認)、(c) `Navigate` の後のイベントを返すための thunk の扱い(decision 0119)、の 3 点を先に決める必要がある。
- 他のタブ(左から 1 番目・星。2〜4 番目は呼び出しが無い)を押すと、クラウドの WebView2 は隠れる(`put_IsVisible 0`)。その中身(「Packs」「Current project」などの一覧)は、WebView2 を通らない FL の描画か、この run では見えていない(推定。未確認)。

## 10. 未確認

- `about:blank` の最初の移動に、4 つのイベントが当たるか(仕様の順序だけを使った)。
- `ICoreWebView2NavigationStartingEventArgs`・`ICoreWebView2SourceChangedEventArgs`・`ICoreWebView2ContentLoadingEventArgs` の vtable の並び(IDL の順と、作者の記憶。公開のソースでは照合していない)。
- `ICoreWebView2HttpRequestHeaders`・`ICoreWebView2HttpHeadersCollectionIterator` の vtable は、wailsapp/go-webview2(`pkg/edge`)の定義に従う(公開の参照の型と一致)。
- `Navigate` 後にイベントを返すと、どの受け口が thunk を呼ぶか(最初の版では `NavigationStarting` の受け口で止まった。他の受け口は試していない)。
- `error-offline.html` の中身と、オンラインの時の内容。
- F8 の意味(「Generators」の表示)。
- `elapsedMs` の基準からの増加(7751 → 8678〜8759 ms)の原因。
- `[js] fle thread 13 failed: HostFilesUnsupported ...`(前回の試行の末尾に出た行)は、最終版の測定 1 回と探索 2 回のログには出なかった(「HostFilesUnsupported」の文字列で検索)。#1357 の後の版での再現の有無は、これだけでは分からない。
