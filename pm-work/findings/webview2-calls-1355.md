# #1355 WebView2 の呼び出しの記録(FL が WebView2 に渡す内容)

実エンジン: この runner で `fe-build engine` と `fe-build bundle` を作り(ビルドは 2 回。1 回目は `VariantClear` で止まり、2 回目で起動が通った)、`tools/boot-check/boot-bundle.mjs` と `tools/browser-panel/measure.mjs` を走らせ、エンジンのログ(`window.__app.engineLog()`)の `webview2:` 行を観測した。

runner の値(この機械とは別枠。nproc 4。loadavg は測定ごと)。

| 項目 | 基準(変更の前) | 今回 1 回目 | 今回 2 回目 | 今回 3 回目 |
| --- | --- | --- | --- | --- |
| state / error / standIn | engine-idle / null / false | engine-idle / null / false | engine-idle / null / false | engine-idle / null / false |
| blackShare | 0.929158 | 0.9291581108829569 | 0.9291581108829569 | 0.9291581108829569 |
| browserComponents | 1 | 1 | 1 | 1 |
| elapsedMs | 7751 | 10874 | 10903 | 10926 |
| webview2Lines | — | 48 | 48 | 48 |
| loadavg(測定の開始時) | 1.48 1.21 1.67 | 1.44 1.34 1.95 | 0.62 1.08 1.80 | 1.42 1.22 1.82 |

`blackShare` は判定に入れない(本 Issue の対象外。描かれないので変わらない)。`elapsedMs` は 3 回とも受け入れの上限 9251 ms を超えた。原因は未確認(下の「未確認」)。

## 1. 呼び出しの順序(実測: 1 回目の起動の `webview2:` 行、全 48 行の先頭から)

```
CreateCoreWebView2EnvironmentWithOptions (WebView2Loader) browser="" userData="C:\Users\user\AppData\Roaming\Image-Line\Edge\" options=0 handler=0x159d0128
AddRef (ICoreWebView2Environment) refs=2
AddRef (ICoreWebView2Environment) refs=3
CreateCoreWebView2Controller (ICoreWebView2Environment) hwnd=0x7f990118 handler=0x159d0168
AddRef (ICoreWebView2Controller) refs=2
get_CoreWebView2 (ICoreWebView2Controller) -> ICoreWebView2 0x11d3aeb0
get_Settings (ICoreWebView2) -> ICoreWebView2Settings 0x11d3af00
put_AreDevToolsEnabled (ICoreWebView2Settings) value=0
put_AreDefaultContextMenusEnabled (ICoreWebView2Settings) value=4294967295
QueryInterface (ICoreWebView2Settings) iid={fdb5ab74-af33-4854-84f0-0a631deb5eba} -> E_NOINTERFACE
QueryInterface (ICoreWebView2Controller) iid={c979903e-d4ca-4228-92eb-47ee3fa96eab} -> E_NOINTERFACE
QueryInterface (ICoreWebView2Controller) iid={f9614724-5d2b-41dc-aef7-73d62b51543b} -> E_NOINTERFACE   (3 times)
QueryInterface (ICoreWebView2Controller) iid={97d418d5-a426-4e49-a151-e1a10f327d9e} -> E_NOINTERFACE
AddHostObjectToScript (ICoreWebView2)
put_AreDevToolsEnabled (ICoreWebView2Settings) value=0
put_AreDefaultContextMenusEnabled (ICoreWebView2Settings) value=0
QueryInterface (ICoreWebView2Settings) iid={11cb3acd-9bc8-43b8-83bf-f40753714f87} -> E_NOINTERFACE
put_IsStatusBarEnabled (ICoreWebView2Settings) value=0
add_NavigationStarting (ICoreWebView2) handler=... token=1
add_NavigationCompleted (ICoreWebView2) handler=... token=2
add_SourceChanged ... token=3
add_HistoryChanged ... token=4
add_ContentLoading ... token=5
add_DocumentTitleChanged ... token=6
add_NewWindowRequested ... token=7
add_FrameNavigationStarting ... token=8
add_FrameNavigationCompleted ... token=9
add_WebResourceRequested ... token=10
add_ScriptDialogOpening ... token=11
add_PermissionRequested ... token=12
add_ProcessFailed ... token=13
add_WebMessageReceived ... token=14
add_ContainsFullScreenElementChanged ... token=15
add_WindowCloseRequested ... token=16
add_ZoomFactorChanged (ICoreWebView2Controller) ... token=17
add_MoveFocusRequested (ICoreWebView2Controller) ... token=18
add_AcceleratorKeyPressed (ICoreWebView2Controller) ... token=19
add_GotFocus (ICoreWebView2Controller) ... token=20
add_LostFocus (ICoreWebView2Controller) ... token=21
put_Bounds (ICoreWebView2Controller) bounds=0,0,212,871
put_Bounds (ICoreWebView2Controller) bounds=0,0,364,871
put_Bounds (ICoreWebView2Controller) bounds=0,0,364,905
put_Bounds (ICoreWebView2Controller) bounds=0,0,364,917
put_Bounds (ICoreWebView2Controller) bounds=0,0,364,905
put_IsVisible (ICoreWebView2Controller) isVisible=1
put_IsVisible (ICoreWebView2Controller) isVisible=1
```

(`handler=...` は値を省いた。全文は測定の標準エラーに出る。)

- 環境の作成は、完了のハンドラを呼ぶ形で進み、コントローラーの作成も同じ(FL の完了のハンドラが次の呼び出しをする)。
- 起動の最後は `put_IsVisible`。以後、アイドルの後 5 秒の測定でも、`webview2:` 行は増えなかった(48 行のまま)。
- `webview2Methods`(初出順、34 種): `CreateCoreWebView2EnvironmentWithOptions, AddRef, CreateCoreWebView2Controller, get_CoreWebView2, get_Settings, put_AreDevToolsEnabled, put_AreDefaultContextMenusEnabled, QueryInterface, AddHostObjectToScript, put_IsStatusBarEnabled, add_NavigationStarting, add_NavigationCompleted, add_SourceChanged, add_HistoryChanged, add_ContentLoading, add_DocumentTitleChanged, add_NewWindowRequested, add_FrameNavigationStarting, add_FrameNavigationCompleted, add_WebResourceRequested, add_ScriptDialogOpening, add_PermissionRequested, add_ProcessFailed, add_WebMessageReceived, add_ContainsFullScreenElementChanged, add_WindowCloseRequested, add_ZoomFactorChanged, add_MoveFocusRequested, add_AcceleratorKeyPressed, add_GotFocus, add_LostFocus, put_Bounds, put_IsVisible`。
- `Navigate` も `NavigateToString` も、起動の中では現れなかった。受け入れの「Navigate か NavigateToString が在る」は満たさない。

## 2. URL・HTML・スクリプト・メッセージの出所(実測)

- URL: 起動の中で `Navigate` は呼ばれない。FL は WebView2 に URL を渡さない。
- HTML: `NavigateToString` は呼ばれない。HTML は渡されない。
- スクリプト: `ExecuteScript` と `AddScriptToExecuteOnDocumentCreated` は呼ばれない。
- メッセージ: `PostWebMessageAsJson`・`PostWebMessageAsString` は呼ばれない。`add_WebMessageReceived` で受け口(ハンドラ)を登録しただけ。FL → ページの メッセージは無い。ページ → FL のメッセージは、ページが動いていないので観測できない。
- 唯一の文字列の引数: 環境の `userData`(`C:\Users\user\AppData\Roaming\Image-Line\Edge\`)。FL が自分で組み立てて渡す(どの手続きかは未確認。読みは `browser=""`、空の文字列)。
- `AddHostObjectToScript`: 1 回。VARIANT の中の host オブジェクト(ゲストの Delphi のオブジェクト)を渡す。直後の `VariantClear` が `IUnknown::Release` を呼ぶ(下の「VariantClear の変更」)。

## 3. 完了のハンドラの呼び出し順(モデルは同期に呼ぶ。CHOICE S-wv2-sync)

1. 環境の完了のハンドラ(`Invoke(hr, environment)`)。その中で `CreateCoreWebView2Controller(hwnd)` を呼ぶ。
2. コントローラーの完了のハンドラ(`Invoke(hr, controller)`)。その中で `get_CoreWebView2`・`get_Settings`・設定の `put_*`・イベントの `add_*` を呼ぶ。
3. `put_Bounds`(5 回)、`put_IsVisible`(2 回)。
- `NavigationCompleted` は、`Navigate` か `NavigateToString` の後に呼ぶ作り(CHOICE)。起動では navigate が無いので、呼ばれない。

## 4. VariantClear の変更(製品の振る舞いを 1 箇所だけ変えた)

`lift/runtime/fle_oleaut.c` の `VariantClear`(VT_UNKNOWN / VT_DISPATCH で、中身が非 NULL)は、従来は「call-back が無いので止まる」だった。今回は、ゲストの `IUnknown::Release`(vtable の 3 番目)を `fle_call_guest` で呼ぶ。ページの定義(VariantClear は中身を Release する)どおり。`SafeArray` の解放(`array_release`)は、従来どおり止まる(今回の経路では使われない)。

## 5. 次の Issue(描画)の根拠(推定を含む)

- 起動の時点で、FL が WebView2 に渡すのは「環境、コントローラー、設定、イベントの受け口、領域、表示」だけ。URL・HTML・スクリプト・メッセージは渡されていない(実測)。
- したがって、「一覧の HTML を、ページ側の iframe に写す」設計(b)の材料は、この起動の経路には無い。一覧の内容が渡される契機(ブラウザ パネルの操作、タイマー、ニュースの取得)を、次の Issue で確かめる必要がある(推定)。
- ニュースの取得(`support.image-line.com` の `downloadnewsrss`)は、この runner では名前が解決せず失敗した(`InternetOpenUrlW ... -> NULL, last error 12007`)。ニュースの表示が WebView2 への navigate の契機かは未確認。
- 仮説(未確認): 実物の WebView2 は、コントローラーの作成の後に最初の `about:blank` の NavigationCompleted を出す。このモデルは出さない。ラッパーがこれを待つなら、navigate が来ない理由になりうる。次の Issue で、NavigationCompleted を 1 回出す版と比べて確かめる(推定)。

## 6. 方針の判断(作業者の決定)

- モデルは引っ込めなかった。起動は止まらず、アイドルまで進んだ。
- 黒い割合を下げる変更はしていない(描画は次の Issue)。

## 7. 未確認

- `elapsedMs` の増加(基準 7751 ms → 10874〜10926 ms)の原因。WebView2 の経路で FL がアイドルまでに行う仕事が増えたことが考えられるが、測っていない。
- 起動の後の `[js] fle thread 13 failed: HostFilesUnsupported: ... pread on descriptor 85` の行(起動の確認の後の、ログの末尾にだけ現れた)。この変更との関係は未確認。
- `QueryInterface` の IID のうち、`{c979903e-d4ca-4228-92eb-47ee3fa96eab}` だけが ICoreWebView2Controller2 だと確かめた(go-webview2 のソース)。他の IID の名前は未確認。
- 応答 vtable の順序は、go-webview2 パッケージのソース(公開)による。WebView2.idl 本体では照合していない。
