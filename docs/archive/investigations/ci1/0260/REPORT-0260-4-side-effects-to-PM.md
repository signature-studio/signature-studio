# ci1 → PM 決定 0260 報告4: ページのフォーカス修正の副作用点検（app/src のフォーカス移動の全数）

## 先頭の事実（grep は HEAD ee18a690 の app/src 全体。実ブラウザの確認は Chrome 156、Worker a3fc8c43 相当のページ = 4c5d36e6）
- 奪い返される条件（修正の中身）: `#desktop` の focusout で、(1) 行き先が別コントロール → 次のタスクでまだその要素が active なら戻す、(2) 行き先なし → active が body/null なら戻す。ただし直近 1 s に『その行き先（または外側）へのポインタ押下か Tab』があった場合と、行き先が `INPUT`/`TEXTAREA`/`SELECT`/contenteditable の場合は戻さない。
- `.focus(` の呼び出しは 14 箇所、`autofocus`・`showModal`・`showPopover` は 0 件、`tabindex` は `#desktop`（index.html）だけ。
- **奪い返される形は 1 つだけ確認した**: ページの Files ボタンを**ポインタ押下なしで**（プログラムの `.click()` など。desktop が 1 s を超えて持っている状態）開くと、Files の窓は開く（`overlayHidden=false`）が、`#fx-list` への `focus()` が desktop に奪い返され、`activeElement` は desktop のまま（実ブラウザで確認、`out/files-a-5012`）。本物のマウス押下・キーボード（Tab → Enter）では起きない（ボタンが先にフォーカスを取り、desktop の focusout は押下の時点で免責される。本物の押下 3 回中 3 回で fx-list にフォーカスが入った。うち 1 回は、窓を開いた約 126 ms 後にブラウザが fx-list から panel-toggle へフォーカスを移した ← 同じ『勝手な移動』で、修正の有無に関係なく起きる〔desktop 以外から出る移動は修正の対象外〕）。

## 全数の表
| # | 場所 | 呼び出し | いつ・誰が | desktop が持っている最中に起きうるか | この修正で奪い返されるか |
|---|---|---|---|---|---|
| 1 | input/pointer.ts:52 | `el.focus()`（desktop） | ポインタ押下 | — | desktop へ（修正の対象外） |
| 2 | input/pointer.ts:89 | `el.focus()`（修正本体） | 勝手な移動の後 | — | — |
| 3 | main.ts:137 | `FileDialogView.closed` → `desktop.focus()` | エンジンのファイル窓が閉じた | はい | desktop へ戻す側なので影響なし |
| 4 | main.ts:139 | Files 窓の `onClosed` → `desktop.focus()` | Files 窓を閉じた | — | 同上 |
| 5 | main.ts:579 | body が active のとき `desktop.focus()` | エンジンのメッセージ | — | 同上 |
| 6 | ui/filedlg.ts:133 | `input[type=file].focus()`（microtask） | ゲストが『開く』を要求 → ページの代行ダイアログ | **はい**（desktop が持っている） | **いいえ**: 行き先が `INPUT`（`isTextEntry` はタグ名で判定、type=file も含む） |
| 7 | ui/filedlg.ts:172 | `input[type=text].focus()`（microtask） | ゲストが『保存』を要求 | はい | いいえ（INPUT） |
| 8 | files/engine-dialog.ts:65 | `#fx-filename.focus()` | ゲスト要求のファイル窓の最初の一覧の後 | はい | いいえ（INPUT）。直前に一覧など非テキストへ移っても、0 ms 後に active が行き先でなければ戻さない |
| 9 | files/mount.ts:50 | `#fx-list.focus()`（`await ex.navigate` の後） | Files ボタンの `click` ハンドラ | ボタンが本物の押下／Tab+Enter なら**いいえ**（ボタンが先にフォーカスを取る）。ボタンを押してもフォーカスを取らない環境、またはポインタを使わない `.click()` なら**はい** | **本物の押下・キーボード: いいえ**。**ポインタ押下なしの `.click()`・フォーカスを取らない環境: 奪い返される**（上で確認。直し方は下） |
| 10 | files/mount.ts:70, 71 | Tab の囲い込み `first/last.focus()` | Files 窓内の Tab | いいえ（窓内） | 影響なし（Tab は免責） |
| 11 | files/explorer.ts:313 | `list.focus()` | 行のクリック | いいえ（窓内） | 影響なし |
| 12 | files/explorer.ts:473, 478 | モーダルの `finish` → `list.focus()`、`(focus ?? button).focus()` | 窓内の操作の結果 | いいえ（窓内） | 影響なし |
| 13 | files/explorer.ts:503 | `input.focus()`（microtask、名前入力） | 窓内の操作 | いいえ | いいえ（INPUT） |
| 14 | index.html / main.ts の Monitor・Stop・resize-reload・send-tab・log 要素 | ユーザーの押下で focus | ポインタ押下／Tab | 押下で desktop から移る | いいえ（押下した要素、またはその内側／外側。ラベルの先の checkbox は INPUT） |
| 15 | （コード外）支援技術・ブラウザ拡張による移動 | ポインタも Tab も無い移動 | — | はい | **はい**（意図した動作と衝突: スクリーンリーダーの仮想カーソルで desktop から別のボタンへ移すと、1 s 以内の押下・Tab が無ければ desktop に戻される） |
- 『ログの入力欄』『perf パネル』『ダイアログ』は app/src に focus を移す script が無い（`engine-log` は `<pre>`、perf パネルは無い、ダイアログはファイル窓のみ）。

## 直し方（#9。旗なし。任意。未決）
- 案: `byUser` を『直近 1 s に `#desktop` の外へのポインタ押下か Tab があった』に単純化する（`pressed === null || !el.contains(pressed)`）。desktop 自身の押下は免責にしない（案3 の欠陥の再発を避ける）。これでボタンを押してもフォーカスを取らない環境でも Files の `#fx-list` は奪い返されない。`.click()`（ポインタ無し）は依然奪い返される（ポインタ押下が無いので）ため、Files の `openWindow` を呼ぶ側でフォーカスを移す前に何もしないなら、プログラム起動は desktop のままで良い（キーボード操作の入口はボタンの Enter なので問題ない）。
- 未検証: Safari、macOS の Firefox、実機の Firefox / 通常の Chrome。
