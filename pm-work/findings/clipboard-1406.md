# #1406 クリップボードの問い合わせ: 読み取り・権限の要求・表示を起こさない(貼り付けの操作の時だけ読む)

実エンジン: この runner で `lift/target/release/fe-build engine` を 1 回(約 40 分以内)、`fe-build bundle` を作り、`tools/boot-check/boot-bundle.mjs` が `error` null・`state` engine-idle・`standIn` false(elapsedMs 12709)、`tools/clipboard/measure.mjs` が `copied` true・`match` true・`runs` 12 件・`copyFailure` を出し(08:41〜08:52 UTC)、`tools/smoke/welcome-flow.mjs` の流れ A〜D が全て判定を通すのを観測した(08:54 UTC)。受け入れの判定式を clip.json に当てた結果は `ALL true`。`npm test --prefix app` は 1423 passed・10 skipped。

runner の値(この機械とは別枠。nproc 4。loadavg は測定の終わりに 2.76 2.35 2.18)。

## 0. 呼び出しの記録

### 0a. 直す前(前回の試行の記録。DPR 1)

- ノブ(Channel rack の 1 番目の Volume)の右クリック(prompt): `IsClipboardFormatAvailable fmt=1 -> 0` が 2 回。ページの `readText` が 2 回(`knobReadCalls` 2)。
- denied: `IsClipboardFormatAvailable fmt=1 -> 0` が 2 回。読みは 0 回。ただし状態行(`#clipboard-status`)が出て、消えなかった。
- granted: `IsClipboardFormatAvailable fmt=1 -> 1`、`OpenClipboard`、`GetClipboardData fmt=13 -> … (os text)`(右クリックの間に呼ばれた)、`CloseClipboard`、を 2 回。読み(`readText`)は 4 回。
- Ctrl+V: `paste` イベントは **0 回**。原因は `app/src/input/keymap.ts` の `keyPolicy`: Ctrl+V の keydown を既定の動作ごと止めていた(keydown を止めると、ブラウザは paste のコマンドを実行せず、paste イベントも出ない)。

### 0b. 直した後(この試行。`measure.mjs` の runs の `knobCalls`・`ctrlVCalls`・`menuPasteCalls`。DPR 1 と 1.92)

- ノブの右クリック(prompt): `IsClipboardFormatAvailable fmt=1 -> 1` → `OpenClipboard` → `GetClipboardData fmt=13 -> … (os text) bytes=4 text="0"` → `CloseClipboard`、を 2 回。**右クリックの間に `GetClipboardData` が呼ばれる**(2 回。どちらも読まずに `"0"` を返す。`readText` は 0 回)。
- ノブの右クリック(granted): 同じ並び(`GetClipboardData` が 2 回、text="0")。形式の一覧のための `navigator.clipboard.read()` が 1 回(内容は読まず、確認も出ない)。
- ノブの右クリック(denied): `IsClipboardFormatAvailable fmt=1 -> 0` が 2 回のみ。`GetClipboardData` は呼ばれない(FL は「Paste value」を無効に描く)。
- Ctrl+V(prompt / denied / granted の全件): `IsClipboardFormatAvailable fmt=1 -> 1` → `OpenClipboard` → `GetClipboardData fmt=13 -> … (os text) text="<測定の数字>"` → `CloseClipboard`。読み 0 回。
- `paste` イベント: **1 回**(全 12 件。target は `desktop`、中身は測定の数字と一致)。`keyPolicy` が Ctrl+V の既定の動作を止めないようにしたため。
- メニューの「Paste value」: prompt の 1 回目は `GetClipboardData fmt=13 -> 0`(headless では `readText` が失敗し、理由の表示が 1 回出る)、2 回目(granted にした後)は文字列が一致。denied は `GetClipboardData fmt=13 -> 0` と理由の表示。granted は 1 回目で一致。

## 1. 採った方式と理由

- 問い合わせ(`IsClipboardFormatAvailable`・`CountClipboardFormats`・`EnumClipboardFormats`)は、新しい非同期の import `fle_clip_host_probe`(JSPI。`lift/harness/web/clip/fle_clip_lib.js`)で、ページに「文字列があるか」だけを聞く。内容は読まない。答え(`app/src/engine/clipboard.ts` の `probe()`):
  - (a) 既に知っている文字列(直前の貼り付け・Copy・読みの結果)があれば、それに従う。
  - (b) `denied`: 無い。
  - (c) `granted`: `navigator.clipboard.read()` の形式の一覧(確認は出ない。1 秒の短いキャッシュ)。
  - (d) `prompt` で既知が無い: 在る(読まず、確認も出さない)。
- ノブの右クリックで FL が呼ぶ `GetClipboardData`(値の確かめ。直前の右ボタンの離れから 1.5 秒以内の読み)は、読まずに答える: 既知の文字列があればそれ、無ければ `"0"`(数値。FL の確かめを通すため)。`denied` で既知が無ければ空。
- 貼り付けの経路は 2 つ:
  - メニューの「Paste value」(左ボタンの離れの後の読み): `readText()`。`prompt` なら許可の確認が出る(クリックの後に呼ぶ)。`granted` は読む。`denied` は読まず、理由の表示を 1 回出し、空を返す(エンジンは止めない)。
  - Ctrl+V: `keyPolicy` が Ctrl+V(KeyV、Ctrl あり、Alt・Meta なし)の既定の動作を止めないようにした。ブラウザが貼り付け、フォーカス中のデスクトップで `paste` イベントが出る。ページは `paste` の `clipboardData` の `text/plain` を「次の 1 回の読み」の文字列として受け取る(権限の確認も読みも無い)。次の読みで使い切り、マウスのボタンを押した時にも捨てる。
  - タイミング: keydown と paste は同じ task で起きる。エンジンの問い合わせは Worker から main への要求(別の task)なので、paste より先には届かない。
- 表示(トースト): 貼り付けの結果(メニュー)と Copy の結果としてだけ出る。問い合わせでは出ない。同時に 1 つ(`#clipboard-status`)。5 秒で消える。閉じるボタン(×)がある。出ている間の同じ・別の理由は、同じ要素の文字と消える時刻を更新する(重ねない)。
- ページが作る独自の貼り付けメニューは作っていない(FL のメニューだけ)。
- 位置(測定の道具 `tools/clipboard/measure.mjs`): DPR ごとに実測する(`calibrate`。1 回失敗したら 1 回やり直す)。
  - Problems の箱の Ok: 窓の表の子窓 `Ok` は無かったので、式(幅の比)。閉じる操作の後に箱が消えることを、各件で確かめる。
  - ノブ: 1 目盛りの wheel の前後の画素の差分で、変わった矩形の中心を採る。4 DPR とも差分の中心が採られた(式の位置は使われなかった)。
  - メニューの「Paste value」の行: 権限 `denied`(灰色)と `prompt`(有効)の 2 回のメニューの画素の差分。差分の帯の中心を、メニューの左上からの dy で持つ。
  - 名前の欄(F2): 式の候補(幅の比)を先に試し、F2 で名前の窓が開けば採る(窓の表で確かめる)。4 DPR とも式の候補で開いた。
  - 箱の「FLEX (FL)」の行は、DPR 1 の copyCheck と copyFailure でだけ使う(式。DPR 1 の画素で求めた値)。
- 製品に新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数は足していない。テストのための分岐も入れていない。
- `plugins/ott-host.ts` の stub に、新しい import `fle_clip_host_probe` を足した(プラグインの wasm が fle_clip を含むため)。

## 2. 使い勝手の帰結

- 権限 `prompt`: FL の「Paste value」は有効(問い合わせの答えは「在る」)。押すと、ブラウザの許可の確認が出る(headless では確認は出ず、読みは失敗して空になる。実機で確認が要る)。
- 権限 `denied`: メニューの「Paste value」は無効になる。ただし、Ctrl+V の後は既知の文字列があるため、その後の右クリックでは有効になる(既知の文字列の (a))。その時に押すと、理由の表示が出る。
- 権限 `granted`: 有効で、押すと読む。
- Ctrl+V は、`denied` でも、`paste` イベントの内容で貼り付く。
- 知っている状態が無い時の答えは、Windows の実際の値とは違う(`prompt` と `granted` の右クリックの `"0"` は、読まずに答えるための数値)。
- 既知の文字列(直前の貼り付け・Copy)は、他のアプリでクリップボードを変えた後も、問い合わせの答えに使われる(仕様の (a))。

## 3. 測定の結果(`runs` 12 件)

測定: `node tools/clipboard/measure.mjs <bundle> > clip.json`(08:41〜08:52 UTC)。DPR は 1 と乱数の 3 点: 1.92・2.53・1.13。

| 件 | DPR | knobMenuShown | knobReadCalls | permissionAfterKnob | knobVisibleAlerts | pasteEnabledOnKnob | ctrlVMatch | ctrlVReadCalls | pasteEventOnCtrlV | permissionAfterCtrlV | menuPasteReadCalls | menuPasteMatch | toastOnMenuPaste | toastCount | toastHiddenAfter5s | toastHasClose | engineState |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| prompt | 1 | true | 0 | prompt | 0 | true | true | 0 | 1 | prompt | 2 | true | false | 1 | true | true | engine-idle |
| denied | 1 | true | 0 | denied | 0 | false | true | 0 | 1 | denied | 0 | null | true | 1 | true | true | engine-idle |
| granted | 1 | true | 1 | granted | 0 | true | true | 0 | 1 | granted | 1 | true | false | 0 | true | true | engine-idle |
| prompt | 1.92 | true | 0 | prompt | 0 | true | true | 0 | 1 | prompt | 2 | true | false | 1 | true | true | engine-idle |
| denied | 1.92 | true | 0 | denied | 0 | false | true | 0 | 1 | denied | 0 | null | true | 1 | true | true | engine-idle |
| granted | 1.92 | true | 1 | granted | 0 | true | true | 0 | 1 | granted | 1 | true | false | 0 | true | true | engine-idle |
| prompt | 2.53 | true | 0 | prompt | 0 | true | true | 0 | 1 | prompt | 2 | true | false | 1 | true | true | engine-idle |
| denied | 2.53 | true | 0 | denied | 0 | false | true | 0 | 1 | denied | 0 | null | true | 1 | true | true | engine-idle |
| granted | 2.53 | true | 1 | granted | 0 | true | true | 0 | 1 | granted | 1 | true | false | 0 | true | true | engine-idle |
| prompt | 1.13 | true | 0 | prompt | 0 | true | true | 0 | 1 | prompt | 2 | true | false | 1 | true | true | engine-idle |
| denied | 1.13 | true | 0 | denied | 0 | false | true | 0 | 1 | denied | 0 | null | true | 1 | true | true | engine-idle |
| granted | 1.13 | true | 1 | granted | 0 | true | true | 0 | 1 | granted | 1 | true | false | 0 | true | true | engine-idle |

- `prompt` の `toastCount` 1 は、1 回目のメニューの貼り付け(headless で `readText` が失敗)の理由の表示。2 回目(`Browser.setPermission` で granted にした後)は表示なし(`toastOnMenuPaste` false)。
- `granted` の `knobReadCalls` 1 は、形式の一覧の `navigator.clipboard.read()`(内容の読みではない。確認は出ない)。
- `denied` の `toastEpisodes` は 1 回(5000 ms で消えた)。`prompt` の 1 回目の表示は 4981 ms、2 回目以降の表示は無い。
- 位置の校正(`calibrations`): DPR 1(scale 1)・1.92(scale 2)・2.53(scale 2.4993)・1.13(scale 1)。ノブは 4 件とも差分(wheel)で、(407,147)・(833,305)・(1034,365)・(407,147)。メニューの行の dy は 277・558・690・277。名前の欄は 4 件とも式の候補で F2 が開いた。Ok は式。

## 4. `copyFailure`(`clipboard-write` を denied にして、Problems の箱の「FLEX (FL)」の Copy)

- `toastShown` true・`toastHiddenAfter5s` true(4999 ms)・`toastHasClose` true・`toastCount` 1。
- 表示の文: `The browser does not allow this page to write the clipboard (clipboard-write: denied). Copy did not reach the system clipboard. Allow it in the browser's site settings.`

## 5. Copy(DPR 1)と流れ A〜D

- Copy: `copied` true・`match` true(`text` は "FLEX (FL)")。`copyCalls`: `OpenClipboard`・`EmptyClipboard`・`SetClipboardData fmt=13 … text="FLEX (FL)"`・`CloseClipboard`。
- 流れ A〜D(`tools/smoke/welcome-flow.mjs`、4 行): 各行 `error` null・`state` engine-idle・`standIn` false・`trapped` false・`hostErrors` 0・`opened` true。B と D は `welcomeClosed` true。
  - A: `{"flow":"A","error":null,"state":"engine-idle","standIn":false,"trapped":false,"hostErrors":0,"welcomeClosed":true,"opened":true,"elapsedMs":12016,"preBootBytes":312194414}`
  - B: `{"flow":"B",…,"welcomeClosed":true,"opened":true,"elapsedMs":11757,"preBootBytes":312194414}`
  - C: `{"flow":"C",…,"welcomeClosed":true,"opened":true,"elapsedMs":11733,"preBootBytes":312194414}`
  - D: `{"flow":"D",…,"welcomeClosed":true,"opened":true,"elapsedMs":11855,"preBootBytes":312194414}`
  - 最大 elapsedMs 12016(上限 12500)。最大 preBootBytes 312194414(上限 314368711)。
- 起動(boot-bundle): `error` null・`state` engine-idle・`standIn` false・`elapsedMs` 12709(P1 の 10 秒には届かない。この試行の runner の値。前回は 9224)。

## 6. 未確認・注意・既知の限界

- Ctrl+V の `paste` は、この runner の headless Chrome(CDP の rawKeyDown)で確かめた。実機の Windows の Chrome と Edge(WebView2 ではない)での挙動は未確認。
- prompt の `readText` は headless では許可の確認が出ず、失敗する。実際の許可の確認(ユーザーが「許可」を押す)は、実機での確認が要る。
- 「Paste value」は、FL が数値を求める。知っている状態が無い時の右クリックの答えは `"0"`(読まずに有効にするための値)。その後の貼り付けは実際の文字列を読むが、数値でなければ FL が貼り付けを拒む。
- キーボードでメニューを開いた直後の貼り付けは、右クリックの問い合わせと誤認しうる(右ボタンを離してから 1.5 秒以内、という手がかりによる)。誤認の結果は、その時だけ形式の問い合わせの答えが既知の状態になるだけで、害は小さい。
- `granted` の右クリックでは、形式の一覧のための `read()` が 1 回走る(内容は読まず、確認も出ない)。
- Copy が失敗した時の表示は残す(依頼者の指示は「貼り付けの結果としてだけ」だが、Copy の表示は同じ規則で消える)。広げすぎかどうかは、依頼者が判断する。
- 位置は、拡大率の段の境界では外れうる。この試行の乱数の 3 点(1.92・2.53・1.13)は、ノブと行を差分で求めたので、段が変わっても測り直す。名前の欄と Ok は式なので、段の境界で外れうる(名前の欄は F2 で確かめ、外れた時は件が失敗する)。
- ノブの位置(wheel の差分)は、DPR 2.53 で差分の箱が 62 × 47 px、DPR 1.13 で 16 × 19 px と大小があった(原因は確かめていない。新しく出た窓の矩形は除いてある)。中心は、その後のメニューの右クリックで確かめた(`knobMenuShown` true)。
- 製品の変更は `app/` と `lift/`(C・JS の import の追加)。`main` の deploy(`app/`・`lift/`・`ci/`)を動かすので、報告に書く。

## 7. 3 回目の試行: 校正の失敗の原因と直し(この節が、1 の「位置」の方式を置き換える)

実エンジン: この runner で `lift/target/release/fe-build engine` と `fe-build bundle` を 1 回(10:31〜11:16 UTC)、`tools/boot-check/boot-bundle.mjs` が error null・engine-idle・standIn false(elapsedMs 13471)を返すのを観測した。校正の走査 `tools/clipboard/measure.mjs <bundle> --calibration`(51 点。12:29 UTC)、`tools/clipboard/measure.mjs <bundle>`(12 件と copyFailure。12:31〜12:47 UTC)、`tools/smoke/welcome-flow.mjs`(流れ A〜D。12:49 UTC)、`npm test --prefix app`(1444 passed・10 skipped)を観測した。受け入れの判定式は、校正の 51 点・clip.json の 12 件と copyFailure・流れ A〜D の各段で、すべて真。

runner の値(この機械とは別枠)。nproc 4。loadavg は校正の終わりに 3.41 3.62 3.41、測定の終わりに 2.66 2.59 2.85、流れの終わりに 3.43 2.71 2.85。

### 7.1 校正が落ちた原因(直す前の版で測った)

- 落ちた DPR は 1.44 と 1.56(FL の段は 150%。scale 1.5007)と、1.50 の 1 回目(段は同じ 150%。scale 1.4993)。
- ノブのホイールの試しの箱が 49×60 の大きな塊(guest 座標 (601..650, 204..264))になった。ノブ(Volume)の周りと、隣のパン(Pan)の周りの両方が変わっていた。その中心 (626,234) は右クリックが何も開かず、推定 (610〜611,216) の右クリックも、1.44 と 1.56 では何も開かなかった。
- 同じ DPR でも、試行によって結果が違った。1.5 の 1 回目は 49×60 で失敗し、2 回目は (613,219)・24×29 で成功した。
- 原因: 変化の撮影(a)の時、マウスはまだノブの上に無く、ホイールの後の撮影(b)では、マウスがノブの上に在った。ノブは、マウスが乗ると見た目(ホバー)が変わり、その変化が、ノブの値の変化と一緒に差分に入った。b では、隣のパンの見た目も変わった。画素の写し(4 倍。`cal-1.56-2-wheel-a.png` と `cal-1.56-2-wheel-b.png`)で確かめた。
- 右クリックの推定 (610,216) が 1.5 で開かなかった理由は確かめていない。直した後の試行では (611,216) が 1.44 で開いた。

### 7.2 直した方式(`tools/clipboard/measure.mjs` の校正。製品は変えていない)

1. ノブの探し方: マウスをノブの推定の上に置き、そのまま a を撮る(`hover`)。ホイールの 1 目盛りは、マウスを動かさずに送る(`notch`)。b を撮り、1 目盛りを戻す。a と b の差分(窓が開いた分は除く)の中で、推定に最も近い連結な変化の部品(`nearestPart`)を採り、その中心をノブとする。部品の中心が推定から 8 px 以内の時だけ採る(`VOLUME_REACH`。Pan は推定から約 25 px 右)。
2. 候補の順: ホイールの中心 → 推定 → 推定の上下左右 4 px の 4 点(`NEAR_KNOB`)。最初に、右クリックでノブのメニュー(「Paste value」の行が、権限の違いで変わる)が開いたものを採る。
3. 名前の欄(F2)と Ok は、直す前と同じ方式(名前は式の候補を先に試し、F2 で名前の窓が開けば採る)。メニューの行は、直す前と同じく、画素の差分の帯の中央(dy)。
4. 試行は 2 回まで(`CALIB_TRIES`)。2 回とも失敗した DPR は、ok が偽のエントリになり、試行の記録(`trace`: 変化の部品・メニューの矩形・変化の画素の行)を持つ。
5. 道具の走査: `--calibration` で 1.00 から 3.00 まで 0.04 刻みの 51 点(`SWEEP_DPRS`)。`--dprs a,b,...` で一部だけ走らせられる(診断用)。

### 7.3 校正の走査(51 点。直した後の版。`--calibration`)

- 51 点すべて ok(1 回目の試行で通った)。ノブの採り方は、ホイール 29 点、推定 22 点。近傍 4 点は使われなかった。
- 段ごとの範囲: scale 1.00(DPR 1.00〜1.12)・1.25(1.16〜1.36)・1.50(1.40〜1.60)・1.75(1.64〜1.84)・2.00(1.88〜2.24)・2.50(2.28〜2.72)・3.00(2.76〜3.00)。
- 段の境界の DPR 1.36(scale 1.2504)と 1.40(scale 1.4993)は、どちらも ok。境界の 1.44・1.56 も ok(7.1 の直す前の失敗の点)。
- 表(51 行。`ok` が偽の行は無い):

| DPR | 段(scale) | 採った候補 | ノブ(guest px) | メニューの行の dy | 名前(F2) | 結果 |
| --- | --- | --- | --- | --- | --- | --- |
| 1.00 | 1.000 | wheel | (408,142) | 277 | formula (520,144) | ok |
| 1.04 | 1.000 | wheel | (407,147) | 277 | formula (520,144) | ok |
| 1.08 | 1.000 | wheel | (407,147) | 277 | formula (520,144) | ok |
| 1.12 | 1.000 | wheel | (407,147) | 277 | formula (520,144) | ok |
| 1.16 | 1.250 | wheel | (509,183) | 322 | formula (650,180) | ok |
| 1.20 | 1.250 | wheel | (509,183) | 322 | formula (650,180) | ok |
| 1.24 | 1.250 | wheel | (509,183) | 322 | formula (650,180) | ok |
| 1.28 | 1.250 | wheel | (509,183) | 322 | formula (650,180) | ok |
| 1.32 | 1.249 | wheel | (509,183) | 322 | formula (649,180) | ok |
| 1.36 | 1.250 | wheel | (509,183) | 322 | formula (650,180) | ok |
| 1.40 | 1.499 | wheel | (613,218) | 400 | formula (780,216) | ok |
| 1.44 | 1.501 | guess | (611,216) | 400 | formula (780,216) | ok |
| 1.48 | 1.499 | guess | (610,216) | 400 | formula (780,216) | ok |
| 1.52 | 1.501 | guess | (611,216) | 400 | formula (780,216) | ok |
| 1.56 | 1.501 | wheel | (605,224) | 400 | formula (780,216) | ok |
| 1.60 | 1.499 | wheel | (613,219) | 400 | formula (780,216) | ok |
| 1.64 | 1.750 | guess | (712,252) | 445 | formula (910,252) | ok |
| 1.68 | 1.750 | guess | (712,252) | 445 | formula (910,252) | ok |
| 1.72 | 1.750 | guess | (712,252) | 445 | formula (910,252) | ok |
| 1.76 | 1.750 | guess | (712,252) | 445 | formula (910,252) | ok |
| 1.80 | 1.751 | guess | (713,252) | 445 | formula (911,252) | ok |
| 1.84 | 1.751 | guess | (713,252) | 445 | formula (911,252) | ok |
| 1.88 | 2.000 | guess | (814,288) | 558 | formula (1040,288) | ok |
| 1.92 | 2.000 | guess | (814,288) | 558 | formula (1040,288) | ok |
| 1.96 | 2.000 | guess | (814,288) | 558 | formula (1040,288) | ok |
| 2.00 | 2.000 | wheel | (819,292) | 558 | formula (1040,288) | ok |
| 2.04 | 2.000 | guess | (814,288) | 558 | formula (1040,288) | ok |
| 2.08 | 2.000 | wheel | (819,292) | 558 | formula (1040,288) | ok |
| 2.12 | 2.000 | guess | (814,288) | 558 | formula (1040,288) | ok |
| 2.16 | 2.000 | wheel | (819,293) | 558 | formula (1040,288) | ok |
| 2.20 | 2.000 | wheel | (810,281) | 558 | formula (1040,288) | ok |
| 2.24 | 2.000 | guess | (814,288) | 558 | formula (1040,288) | ok |
| 2.28 | 2.499 | wheel | (1023,365) | 690 | formula (1300,360) | ok |
| 2.32 | 2.501 | wheel | (1023,354) | 690 | formula (1300,360) | ok |
| 2.36 | 2.499 | guess | (1017,360) | 690 | formula (1300,360) | ok |
| 2.40 | 2.499 | guess | (1017,360) | 690 | formula (1300,360) | ok |
| 2.44 | 2.501 | wheel | (1023,354) | 690 | formula (1300,360) | ok |
| 2.48 | 2.499 | wheel | (1023,365) | 690 | formula (1300,360) | ok |
| 2.52 | 2.501 | guess | (1018,360) | 690 | formula (1300,360) | ok |
| 2.56 | 2.501 | guess | (1018,360) | 690 | formula (1300,360) | ok |
| 2.60 | 2.499 | wheel | (1023,365) | 690 | formula (1300,360) | ok |
| 2.64 | 2.501 | wheel | (1023,354) | 690 | formula (1300,360) | ok |
| 2.68 | 2.501 | wheel | (1023,365) | 690 | formula (1300,360) | ok |
| 2.72 | 2.499 | wheel | (1023,365) | 690 | formula (1300,360) | ok |
| 2.76 | 3.000 | wheel | (1229,439) | 822 | formula (1560,432) | ok |
| 2.80 | 3.000 | guess | (1221,432) | 822 | formula (1560,432) | ok |
| 2.84 | 3.000 | wheel | (1228,439) | 822 | formula (1560,432) | ok |
| 2.88 | 3.000 | guess | (1221,432) | 822 | formula (1560,432) | ok |
| 2.92 | 3.000 | wheel | (1228,439) | 822 | formula (1560,432) | ok |
| 2.96 | 3.000 | wheel | (1228,439) | 822 | formula (1560,432) | ok |
| 3.00 | 3.000 | guess | (1221,432) | 822 | formula (1560,432) | ok |

### 7.4 測定(12 件)

- 乱数の DPR は 3・2.16・1.92(DPR 1 と合わせて 4 点)。ノブは各 DPR の校正の結果(DPR 3 は推定、2.16 はホイール、1.92 は推定)。
- 表(`measure.mjs` の runs の 12 件。権限は `prompt`・`denied`・`granted` の各 4 件):

| 件 | DPR | knobMenuShown | knobReadCalls | permissionAfterKnob | knobVisibleAlerts | pasteEnabledOnKnob | ctrlVMatch | ctrlVReadCalls | pasteEventOnCtrlV | permissionAfterCtrlV | menuPasteReadCalls | menuPasteMatch | toastOnMenuPaste | toastCount | toastHiddenAfter5s | toastHasClose | engineState |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| prompt | 1 | true | 0 | prompt | 0 | true | true | 0 | 1 | prompt | 2 | true | false | 1 | true | true | engine-idle |
| denied | 1 | true | 0 | denied | 0 | false | true | 0 | 1 | denied | 0 | null | true | 1 | true | true | engine-idle |
| granted | 1 | true | 1 | granted | 0 | true | true | 0 | 1 | granted | 1 | true | false | 0 | true | true | engine-idle |
| prompt | 3 | true | 0 | prompt | 0 | true | true | 0 | 1 | prompt | 2 | true | false | 1 | true | true | engine-idle |
| denied | 3 | true | 0 | denied | 0 | false | true | 0 | 1 | denied | 0 | null | true | 1 | true | true | engine-idle |
| granted | 3 | true | 1 | granted | 0 | true | true | 0 | 1 | granted | 1 | true | false | 0 | true | true | engine-idle |
| prompt | 2.16 | true | 0 | prompt | 0 | true | true | 0 | 1 | prompt | 2 | true | false | 1 | true | true | engine-idle |
| denied | 2.16 | true | 0 | denied | 0 | false | true | 0 | 1 | denied | 0 | null | true | 1 | true | true | engine-idle |
| granted | 2.16 | true | 1 | granted | 0 | true | true | 0 | 1 | granted | 1 | true | false | 0 | true | true | engine-idle |
| prompt | 1.92 | true | 0 | prompt | 0 | true | true | 0 | 1 | prompt | 2 | true | false | 1 | true | true | engine-idle |
| denied | 1.92 | true | 0 | denied | 0 | false | true | 0 | 1 | denied | 0 | null | true | 1 | true | true | engine-idle |
| granted | 1.92 | true | 1 | granted | 0 | true | true | 0 | 1 | granted | 1 | true | false | 0 | true | true | engine-idle |

- 右クリックの窓の呼び出し(DPR 1 の件。`window-API` の行を、件ごとに取った。右クリックの間に呼ばれた順):
  - `prompt`: `IsClipboardFormatAvailable fmt=1 -> 1` → `OpenClipboard` → `GetClipboardData fmt=13 -> … text="0"` → `CloseClipboard`、を 2 回。読み(`readText`・`read`)は 0 回。`GetClipboardData` は右クリックの間に呼ばれる。
  - `denied`: `IsClipboardFormatAvailable fmt=1 -> 0` が 2 回だけ。`GetClipboardData` は呼ばれず、FL は「Paste value」を無効に描く。
  - `granted`: `prompt` と同じ並び(text="0" を 2 回)。形式の一覧の `navigator.clipboard.read()` が 1 回(内容は読まず、確認も出ない)。
  - Ctrl+V(3 件とも): `IsClipboardFormatAvailable fmt=1 -> 1` → `OpenClipboard` → `GetClipboardData fmt=13 -> … text="<測定の数字>"` → `CloseClipboard`。読み 0 回。`paste` イベントが 1 回(target `desktop`・中身が一致)。
  - メニューの「Paste value」: `prompt` の 1 回目は `GetClipboardData -> 0`(headless では `readText` が失敗し、理由の表示が 1 回出る)。granted にした後の 2 回目は文字列が一致。`denied` は `GetClipboardData -> 0` と理由の表示。`granted` は 1 回目で一致。
- `copyFailure`(DPR 1。`clipboard-write` を denied にして、Problems 箱の「FLEX (FL)」の Copy): `toastShown` true・`toastHiddenAfter5s` true(4999 ms)・`toastHasClose` true・`toastCount` 1。文: "The browser does not allow this page to write the clipboard (clipboard-write: denied). Copy did not reach the system clipboard. Allow it in the browser's site settings."

### 7.5 Copy・流れ A〜D・時間

- Copy(DPR 1): `copied` true・`match` true・`text` は "FLEX (FL)"。
- 流れ A〜D(`tools/smoke/welcome-flow.mjs`。4 行。各行の `error` null・`state` engine-idle・`standIn` false・`trapped` false・`hostErrors` 0・`opened` true。B と D は `welcomeClosed` true):
- {"flow":"A","error":null,"state":"engine-idle","standIn":false,"trapped":false,"hostErrors":0,"welcomeClosed":true,"opened":true,"elapsedMs":12048,"preBootBytes":312223225,"startStalls":0,"hostErrorText":null,"imageReadsAfterIdle":[],"steps":["first idle at 12041 ms (title \"Welcome to FL Studio - in ?trial mode?\")","Welcome window at (481, 241) 874x573","No thanks at guest (993, 548)","Default row at guest (716, 358), 2 click(s)","end: state engine-idle, title \"Problems loading the project\", welcome window closed"]}
- {"flow":"B","error":null,"state":"engine-idle","standIn":false,"trapped":false,"hostErrors":0,"welcomeClosed":true,"opened":true,"elapsedMs":11943,"preBootBytes":312223225,"startStalls":0,"hostErrorText":null,"imageReadsAfterIdle":[],"steps":["first idle at 11940 ms (title \"Welcome to FL Studio - in ?trial mode?\")","Welcome window at (481, 241) 874x573","Escape","end: state engine-idle, title \"Problems loading the project\", welcome window closed"]}
- {"flow":"C","error":null,"state":"engine-idle","standIn":false,"trapped":false,"hostErrors":0,"welcomeClosed":true,"opened":true,"elapsedMs":11893,"preBootBytes":312223225,"startStalls":0,"hostErrorText":null,"imageReadsAfterIdle":[],"steps":["first idle at 11886 ms (title \"Welcome to FL Studio - in ?trial mode?\")","Welcome window at (481, 241) 874x573","No thanks at guest (993, 548)","Default row at guest (716, 358), 1 click(s)","end: state engine-idle, title \"Problems loading the project\", welcome window closed"]}
- {"flow":"D","error":null,"state":"engine-idle","standIn":false,"trapped":false,"hostErrors":0,"welcomeClosed":true,"opened":true,"elapsedMs":12018,"preBootBytes":312223225,"startStalls":0,"hostErrorText":null,"imageReadsAfterIdle":[],"steps":["first idle at 12009 ms (title \"Welcome to FL Studio - in ?trial mode?\")","Welcome window at (481, 241) 874x573","I'm in at guest (841, 548)","after I'm in the Welcome window is still listed","Default row at guest (716, 358), 2 click(s)","end: state engine-idle, title \"Problems loading the project\", welcome window closed"]}
- 4 流れの `elapsedMs` の最大は 12048(上限 12500)。`preBootBytes` の最大は 312223225(上限 314368711)。
- 起動(boot-check): error null・engine-idle・standIn false・`elapsedMs` 13471(この runner の値。P1 の 10 秒には届いていない)。

### 7.6 この試行の既知の限界

- 校正の点は 51 点すべて ok だったが、ノブの右クリックの当たりは数 px と小さい。推定の右クリックは、直す前の版では 1.44 と 1.56 で開かず、直した後は開いた。原因は確かめていない。候補を 4 点足したのはその保険で、この試行では使われなかった。
- ホイールの部品の中心は、回転の弧の中心なので、ノブの中心から数 px ずれうる。右クリックの当たりで確かめるので、採った点がノブでなければ、メニューの行が見つからずに次の候補へ進む。
- 乱数の DPR は実行ごとに変わる。51 点の走査が全部通ったので、段の境界の失敗は観測していないが、どの DPR で落ちるかは、実行の前には分からない。
- 製品(`app/`・`lift/`)は、この試行では変えていない(前回の差分のまま。engine は 1 回ビルドした)。`main` の deploy(`app/`・`lift/`・`ci/`)に関わる変更は、前回の差分の C・JS と `app/` の変更にある。
- headless では `readText` の許可の確認は出ない(7.4 の「prompt の 1 回目」の表示は、その結果)。許可の確認は実機で確かめる。
