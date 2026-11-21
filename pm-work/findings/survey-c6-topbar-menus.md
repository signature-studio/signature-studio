実エンジン: fe-build engine と fe-build bundle を実行して bundle を作り、headless Chrome(Chrome/157.0.8103.78)で engine-idle まで起動して、画面の撮影と右クリック(参考点 2 点)を行った。probe 1 回と、手順 6 の 3 回。出力は下の「実測」の JSON 行。

# survey-c6-topbar-menus: C-6 トップバーのコンテキストメニューの測定

- 種別: 測定のみ。製品(app/・lift/・ci/・tools/・patches/・.github/)は変えていない。変更は、この file だけ。
- 判定: **未確認**。部品(Tempo・Song Position・Pattern Selector)は、起動後の画面に位置が読めず、右クリックを測れなかった。
- 数字は、すべて runner の値。この機械とは別枠。runner の種類: `nproc` = 4。loadavg(1 分・5 分・15 分)は各 JSON 行の `loadavg`(1 分平均は 0.9〜1.0)。

## 実測

### 1. 版と抽出物
- `grep '^EMSDK_VERSION=' ci/versions.env` の結果: `EMSDK_VERSION=7.0.0`
- `find lift/out-app/host -type f | wc -l` の結果: `238`

### 2. ビルド(queue/kinds/lower.md の accept と同じ段。この runner の中で実行)

| 段 | 終了コード | 出力の行(抜粋) |
|---|---|---|
| `npm ci --prefix app` | 0 | `added 74 packages, and audited 75 packages in 3s`(install-scripts の警告 2 件: esbuild、workerd) |
| `npm run build --prefix app` | 0 | `✓ built in 137ms`(型検査 4 本も通過) |
| `cargo build --release --locked --manifest-path lift/Cargo.toml` | 0 | ``Finished `release` profile [optimized] target(s) in 0.01s``(既存の target を使った。警告 7 件は fe-wasmgen) |
| `lift/target/release/fe-build engine` | 0 | `hybrid: 167415786 bytes, 8 direct modules` |
| `lift/target/release/fe-build bundle` | 0 | `engine wasm  167415786 B  sha256 4872c07b…`(bundle 名は fl-engine-Oz。ディレクトリは `raw/fl-Oz`) |

### 3. 読みの記録(実行していない)
`grep -rn 'Tempo\|Song Position\|Pattern Selector\|songpos\|contextmenu' app/src app/index.html --include=*.ts --include=*.html` の結果は 1 件:
```
app/src/input/pointer.ts:104: ... ['contextmenu', onContext as unknown as AnyListener],
```
- ページに、Tempo・Song Position・Pattern Selector・songpos は**無い**(0 件)。
- contextmenu は pointer.ts:104 の 1 件だけ(登録)。

### 4. 起動(手順 4)
- tools/boot-check/boot-bundle.mjs の main を写した一時ファイル tools/boot-check/.tmp-survey-c6.mjs で起動した。4 回すべて `state` が `engine-idle`、`standIn` が false、`error` が null。
- navigate から engine-idle までの `elapsedMs`: probe 12330、run1 12807、run2 12849、run3 12760(ms。P1 の判定とは測り方が違うので、判定には使わない)。

### 5. 画面(手順 5)
- a. canvas(probe の JSON 行の `canvas`): `left` 0、`top` 36、`w` 502、`h` 309(CSS px)、`canvasW` 780、`canvasH` 480。viewport は 780×493、dpr 1。canvas 画素から CSS へは、×502/780 と ×309/480。
- b. 画面の読み(probe の画像を read で開いた): canvas に見えているのは、起動時の "Welcome to FL Studio - in trial mode" の窓と、その上の "Help us improve FL Studio by sending usage data" の確認ダイアログ。**Tempo・Song Position・Pattern Selector は見えず、位置は未確認(位置が読めない)。** 確認ダイアログのボタン(I'm in / No thanks)は押していない。同意の選択は依頼者の判断で、手順にも無いため。
- c. 右クリックの前に、window の bubble 段で contextmenu を記録する listener を入れた(`window.__cmSeen` と `window.__cmPrevented`)。
- d. 部品 3 件は位置が読めないため、**送っていない**。代わりに、画面に見える 2 点を「参考点」として右クリックした。参考点は部品ではない。
  - 参考点 1: Welcome 窓の「Start a new project」欄。canvas px (233, 92) = CSS (149.96, 95.23)。
  - 参考点 2: ゲスト画面の下の灰色の所。canvas px (621, 433) = CSS (399.67, 314.74)。
  - 右クリックは、`Input.dispatchMouseEvent` の mousePressed(button right、buttons 2、clickCount 1)と mouseReleased(button right、buttons 0、clickCount 1)。`buttons` は DOM の pointer の buttons マスクに合わせて付けた。
  - 先に desk へ focus を移す評価は行っていない。pointerdown で #desktop が focus を得る(app/src/input/pointer.ts:52)ため、不要と判断した。
  - 送った後 1 秒待って撮影。撮影は、ページ全体(780×493)と、canvas の範囲(502×309)の 2 種類。
  - 対照: 入力を送らずに 1 秒置いた画面も、同じ 2 種類で撮って比べた。

### 6. 3 回の結果

判定の表(出典は、下の JSON 行の `parts[id]` と `control`。画素数は、下の差分の JSON 行の `changedPixels`)。

| 部品 | 位置(canvas px) | 右クリック | 画面の変化 | contextmenu(届いた数・既定の動作を止めたか) | エンジンのログ(右クリック後に増えた行・メニューの語) | 判定 |
|---|---|---|---|---|---|---|
| Tempo | 読めない | 送っていない | 測っていない | 測っていない | 測っていない | 未確認 |
| Song Position | 読めない | 送っていない | 測っていない | 測っていない | 測っていない | 未確認 |
| Pattern Selector | 読めない | 送っていない | 測っていない | 測っていない | 測っていない | 未確認 |
| 参考点 1(Start a new project 欄)※部品ではない | (233, 92) | 3 回送った | canvas 範囲: 違う。変わった画素 run1 10273 / run2 10278 / run3 10275。対照(入力なし)は 10976 / 10974 / 10976。全画面の PNG は 3 回とも違う(対照も違う) | 1 / 1 / 1。既定の動作は止まった(defaultPrevented true)3 回 | 増えた行 0 / 0 / 0(322 → 322)。メニューの語 0 | 未確認(部品の判定ではない) |
| 参考点 2(灰色の所)※部品ではない | (621, 433) | 3 回送った | canvas 範囲: 違う。変わった画素 run1 437 / run2 518 / run3 435。範囲は canvas の x 4〜148、y 6〜12(クリック位置から離れた左上) | 1 / 1 / 1。既定の動作は止まった(defaultPrevented true)3 回 | 増えた行 0 / 0 / 0(322 → 322)。メニューの語 0 | 未確認(部品の判定ではない) |

- 参考点のページの入力欄(`#engine-line`)には、3 回とも `last input: button R up at screen (232, 91) (pointerup)`(参考点 1)、`last input: button R up at screen (620, 432) (pointerup)`(参考点 2)と出た。右ボタンの離す操作がエンジンに届いたことの読み。
- ログの末尾 10 行(`tail10`)の内容: 起動時の出力。窓の一覧の行、描画の警告 1 件(`first drawing call discarded because its window is not visible (decision 0126, S-draw-hidden)`)、ネット取得の失敗 2 件(エラー 12007)。右クリックの後に増えた行は、どの回も 0 件。ネットの行にはベンダー名を含むホスト名があるため、この file には貼らず件数だけ書く。
- 参考点の画像(run1 の右クリック 1 秒後の画面、ref1 の after)の読み: canvas に、メニューらしい物は見えなかった。ページの欄(入力と、エンジンの状態の文字)だけが変わっていた。

差分の数(`tools/boot-check/.tmp-survey-c6-diff.mjs` で数えた。一時の道具。canvas の画素の比較):
```json
{"run":"run1","control":{"size":"502x309","changedPixels":10976,"bbox":{"x0":0,"y0":0,"x1":501,"y1":308}},"parts":[{"id":"tempo","result":"not clicked"},{"id":"songpos","result":"not clicked"},{"id":"patsel","result":"not clicked"},{"id":"ref1","diff":{"size":"502x309","changedPixels":10273,"bbox":{"x0":1,"y0":0,"x1":501,"y1":308}}},{"id":"ref2","diff":{"size":"502x309","changedPixels":437,"bbox":{"x0":4,"y0":6,"x1":148,"y1":12}}}]}
{"run":"run2","control":{"size":"502x309","changedPixels":10974,"bbox":{"x0":0,"y0":0,"x1":501,"y1":308}},"parts":[{"id":"tempo","result":"not clicked"},{"id":"songpos","result":"not clicked"},{"id":"patsel","result":"not clicked"},{"id":"ref1","diff":{"size":"502x309","changedPixels":10278,"bbox":{"x0":1,"y0":0,"x1":501,"y1":308}}},{"id":"ref2","diff":{"size":"502x309","changedPixels":518,"bbox":{"x0":4,"y0":6,"x1":148,"y1":12}}}]}
{"run":"run3","control":{"size":"502x309","changedPixels":10976,"bbox":{"x0":0,"y0":0,"x1":501,"y1":308}},"parts":[{"id":"tempo","result":"not clicked"},{"id":"songpos","result":"not clicked"},{"id":"patsel","result":"not clicked"},{"id":"ref1","diff":{"size":"502x309","changedPixels":10275,"bbox":{"x0":1,"y0":0,"x1":501,"y1":308}}},{"id":"ref2","diff":{"size":"502x309","changedPixels":435,"bbox":{"x0":4,"y0":6,"x1":148,"y1":12}}}]}
```

各回の結果(JSON の行)。ログの行(`engineLog30`、`tail10`、`addedLines`)は外し、sha256 は先頭 12 桁にした。ベンダー名を含むログを貼らないための処理で、他の値は実行の出力のまま。

- probe(部品の位置を読むために 1 回。手順 6 の 3 回には数えない):
```json
{"startedAt":"2026-11-20T18:26:57.787Z","mode":"probe","state":"engine-idle","engineKind":"emscripten-fruity-v1","timelineIdle":10276.184814453125,"standIn":false,"documentTitle":"Welcome to FL Studio - in ?trial mode?","viewport":{"innerW":780,"innerH":493,"dpr":1},"canvas":{"left":0,"top":36,"w":502,"h":309,"dpr":1,"canvasW":780,"canvasH":480},"shots":[{"name":"control-0","file":".tmp-survey-c6/control-0.png","bytes":56941,"w":780,"h":493,"sha256":"fc96bac439f6"},{"name":"control-1","file":".tmp-survey-c6/control-1.png","bytes":57071,"w":780,"h":493,"sha256":"63cf8815fc7b"},{"name":"probe","file":".tmp-survey-c6/probe.png","bytes":57055,"w":780,"h":493,"sha256":"c1ea34905733"}],"control":{"before":"fc96bac439f6","after":"63cf8815fc7b","same":false},"parts":[],"elapsedMs":12330,"loadavg":[1,1,1.67],"cpus":4,"chromeVersion":"Chrome/157.0.8103.78","error":null}
```
- run1:
```json
{"startedAt":"2026-11-20T18:30:43.493Z","mode":"parts","state":"engine-idle","engineKind":"emscripten-fruity-v1","timelineIdle":9036.31982421875,"standIn":false,"documentTitle":"Welcome to FL Studio - in ?trial mode?","viewport":{"innerW":780,"innerH":493,"dpr":1},"canvas":{"left":0,"top":36,"w":502,"h":309,"dpr":1,"canvasW":780,"canvasH":480},"shots":[{"name":"control-0","file":".tmp-survey-c6/run1-control-0.png","bytes":55911,"w":780,"h":493,"sha256":"c6c362e44616"},{"name":"control-desk-0","file":".tmp-survey-c6/run1-control-desk-0.png","bytes":21062,"w":502,"h":309,"sha256":"ff86a6297fcd"},{"name":"control-1","file":".tmp-survey-c6/run1-control-1.png","bytes":57162,"w":780,"h":493,"sha256":"7a2f8de4f160"},{"name":"control-desk-1","file":".tmp-survey-c6/run1-control-desk-1.png","bytes":21199,"w":502,"h":309,"sha256":"6cc96a9bc224"},{"name":"ref1-before","file":".tmp-survey-c6/run1-ref1-before.png","bytes":57069,"w":780,"h":493,"sha256":"6851815f03ce"},{"name":"ref1-before-desk","file":".tmp-survey-c6/run1-ref1-before-desk.png","bytes":21199,"w":502,"h":309,"sha256":"6cc96a9bc224"},{"name":"ref1-after","file":".tmp-survey-c6/run1-ref1-after.png","bytes":64143,"w":780,"h":493,"sha256":"5d65bb85b220"},{"name":"ref1-after-desk","file":".tmp-survey-c6/run1-ref1-after-desk.png","bytes":21196,"w":502,"h":309,"sha256":"8c67b6f20e3e"},{"name":"ref2-before","file":".tmp-survey-c6/run1-ref2-before.png","bytes":63993,"w":780,"h":493,"sha256":"2f67cb7415c2"},{"name":"ref2-before-desk","file":".tmp-survey-c6/run1-ref2-before-desk.png","bytes":21196,"w":502,"h":309,"sha256":"8c67b6f20e3e"},{"name":"ref2-after","file":".tmp-survey-c6/run1-ref2-after.png","bytes":64053,"w":780,"h":493,"sha256":"09bb983ce39a"},{"name":"ref2-after-desk","file":".tmp-survey-c6/run1-ref2-after-desk.png","bytes":21208,"w":502,"h":309,"sha256":"04313fd40ced"}],"control":{"before":"c6c362e44616","after":"7a2f8de4f160","same":false},"deskControl":{"before":"ff86a6297fcd","after":"6cc96a9bc224","same":false},"parts":[{"id":"tempo","name":"Tempo","canvasPx":null,"result":"not clicked: the position cannot be read in the screenshot","menuLinesCount":0,"addedLinesCount":0,"tail10Count":0},{"id":"songpos","name":"Song Position","canvasPx":null,"result":"not clicked: the position cannot be read in the screenshot","menuLinesCount":0,"addedLinesCount":0,"tail10Count":0},{"id":"patsel","name":"Pattern Selector","canvasPx":null,"result":"not clicked: the position cannot be read in the screenshot","menuLinesCount":0,"addedLinesCount":0,"tail10Count":0},{"id":"ref1","name":"参考点 1: Welcome 窓の Start a new project 欄(部品ではない)","canvasPx":{"x":233,"y":92},"cssPx":{"x":149.96,"y":95.23},"pageInputLine":"Engine: computing (0.0 s) - input waits for its next idle input: 3 sent, 3 processed, 0 pending · engine cycle 0.02 s (47.9/s) · last input processed 0.05 s after the event, drawn 0.05 s after it · last input: button R up at screen (232, 91) (pointerup)","pagePointerInfo":"pointer: -","screenBefore":"6851815f03ce","screenAfter":"5d65bb85b220","screenSame":false,"deskBefore":"6cc96a9bc224","deskAfter":"8c67b6f20e3e","deskSame":false,"contextmenuSeen":1,"defaultPrevented":true,"engineLogCountBefore":322,"engineLogCountAfter":322,"engineLogAdded":0,"menuAdded":0,"menuLinesCount":0,"addedLinesCount":0,"tail10Count":10},{"id":"ref2","name":"参考点 2: ゲスト画面の下の灰色の所(部品ではない)","canvasPx":{"x":621,"y":433},"cssPx":{"x":399.67,"y":314.74},"pageInputLine":"Engine: computing (0.1 s) - input waits for its next idle input: 6 sent, 6 processed, 0 pending · engine cycle 0.02 s (52.0/s) · last input processed 0.02 s after the event, drawn 0.02 s after it · last input: button R up at screen (620, 432) (pointerup)","pagePointerInfo":"pointer: -","screenBefore":"2f67cb7415c2","screenAfter":"09bb983ce39a","screenSame":false,"deskBefore":"8c67b6f20e3e","deskAfter":"04313fd40ced","deskSame":false,"contextmenuSeen":1,"defaultPrevented":true,"engineLogCountBefore":322,"engineLogCountAfter":322,"engineLogAdded":0,"menuAdded":0,"menuLinesCount":0,"addedLinesCount":0,"tail10Count":10}],"elapsedMs":12807,"loadavg":[0.9,0.81,1.44],"cpus":4,"chromeVersion":"Chrome/157.0.8103.78","error":null}
```
- run2:
```json
{"startedAt":"2026-11-20T18:31:18.551Z","mode":"parts","state":"engine-idle","engineKind":"emscripten-fruity-v1","timelineIdle":9124.619873046875,"standIn":false,"documentTitle":"Welcome to FL Studio - in ?trial mode?","viewport":{"innerW":780,"innerH":493,"dpr":1},"canvas":{"left":0,"top":36,"w":502,"h":309,"dpr":1,"canvasW":780,"canvasH":480},"shots":[{"name":"control-0","file":".tmp-survey-c6/run2-control-0.png","bytes":55193,"w":780,"h":493,"sha256":"40728955c9ee"},{"name":"control-desk-0","file":".tmp-survey-c6/run2-control-desk-0.png","bytes":21062,"w":502,"h":309,"sha256":"ff86a6297fcd"},{"name":"control-1","file":".tmp-survey-c6/run2-control-1.png","bytes":57153,"w":780,"h":493,"sha256":"452e71b36256"},{"name":"control-desk-1","file":".tmp-survey-c6/run2-control-desk-1.png","bytes":21208,"w":502,"h":309,"sha256":"ea72d74ff8c7"},{"name":"ref1-before","file":".tmp-survey-c6/run2-ref1-before.png","bytes":57186,"w":780,"h":493,"sha256":"91a98c92b55d"},{"name":"ref1-before-desk","file":".tmp-survey-c6/run2-ref1-before-desk.png","bytes":21241,"w":502,"h":309,"sha256":"92b2a793eb2b"},{"name":"ref1-after","file":".tmp-survey-c6/run2-ref1-after.png","bytes":64011,"w":780,"h":493,"sha256":"40bd1345f784"},{"name":"ref1-after-desk","file":".tmp-survey-c6/run2-ref1-after-desk.png","bytes":21209,"w":502,"h":309,"sha256":"c7ec90de43cd"},{"name":"ref2-before","file":".tmp-survey-c6/run2-ref2-before.png","bytes":64072,"w":780,"h":493,"sha256":"8c522a1747e1"},{"name":"ref2-before-desk","file":".tmp-survey-c6/run2-ref2-before-desk.png","bytes":21209,"w":502,"h":309,"sha256":"c7ec90de43cd"},{"name":"ref2-after","file":".tmp-survey-c6/run2-ref2-after.png","bytes":62478,"w":780,"h":493,"sha256":"bdb93b657657"},{"name":"ref2-after-desk","file":".tmp-survey-c6/run2-ref2-after-desk.png","bytes":21208,"w":502,"h":309,"sha256":"04313fd40ced"}],"control":{"before":"40728955c9ee","after":"452e71b36256","same":false},"deskControl":{"before":"ff86a6297fcd","after":"ea72d74ff8c7","same":false},"parts":[{"id":"tempo","name":"Tempo","canvasPx":null,"result":"not clicked: the position cannot be read in the screenshot","menuLinesCount":0,"addedLinesCount":0,"tail10Count":0},{"id":"songpos","name":"Song Position","canvasPx":null,"result":"not clicked: the position cannot be read in the screenshot","menuLinesCount":0,"addedLinesCount":0,"tail10Count":0},{"id":"patsel","name":"Pattern Selector","canvasPx":null,"result":"not clicked: the position cannot be read in the screenshot","menuLinesCount":0,"addedLinesCount":0,"tail10Count":0},{"id":"ref1","name":"参考点 1: Welcome 窓の Start a new project 欄(部品ではない)","canvasPx":{"x":233,"y":92},"cssPx":{"x":149.96,"y":95.23},"pageInputLine":"Engine: computing (0.1 s) - input waits for its next idle input: 3 sent, 3 processed, 0 pending · engine cycle 0.02 s (54.9/s) · last input processed 0.09 s after the event, drawn 0.09 s after it · last input: button R up at screen (232, 91) (pointerup)","pagePointerInfo":"pointer: -","screenBefore":"91a98c92b55d","screenAfter":"40bd1345f784","screenSame":false,"deskBefore":"92b2a793eb2b","deskAfter":"c7ec90de43cd","deskSame":false,"contextmenuSeen":1,"defaultPrevented":true,"engineLogCountBefore":322,"engineLogCountAfter":322,"engineLogAdded":0,"menuAdded":0,"menuLinesCount":0,"addedLinesCount":0,"tail10Count":10},{"id":"ref2","name":"参考点 2: ゲスト画面の下の灰色の所(部品ではない)","canvasPx":{"x":621,"y":433},"cssPx":{"x":399.67,"y":314.74},"pageInputLine":"Engine: computing (0.1 s) - input waits for its next idle input: 6 sent, 6 processed, 0 pending · engine cycle 0.02 s (53.0/s) · last input processed 0.03 s after the event · last input: button R up at screen (620, 432) (pointerup)","pagePointerInfo":"pointer: -","screenBefore":"8c522a1747e1","screenAfter":"bdb93b657657","screenSame":false,"deskBefore":"c7ec90de43cd","deskAfter":"04313fd40ced","deskSame":false,"contextmenuSeen":1,"defaultPrevented":true,"engineLogCountBefore":322,"engineLogCountAfter":322,"engineLogAdded":0,"menuAdded":0,"menuLinesCount":0,"addedLinesCount":0,"tail10Count":10}],"elapsedMs":12849,"loadavg":[0.94,0.82,1.42],"cpus":4,"chromeVersion":"Chrome/157.0.8103.78","error":null}
```
- run3:
```json
{"startedAt":"2026-11-20T18:31:40.022Z","mode":"parts","state":"engine-idle","engineKind":"emscripten-fruity-v1","timelineIdle":9114.205078125,"standIn":false,"documentTitle":"Welcome to FL Studio - in ?trial mode?","viewport":{"innerW":780,"innerH":493,"dpr":1},"canvas":{"left":0,"top":36,"w":502,"h":309,"dpr":1,"canvasW":780,"canvasH":480},"shots":[{"name":"control-0","file":".tmp-survey-c6/run3-control-0.png","bytes":55158,"w":780,"h":493,"sha256":"091aa12d18a1"},{"name":"control-desk-0","file":".tmp-survey-c6/run3-control-desk-0.png","bytes":21062,"w":502,"h":309,"sha256":"ff86a6297fcd"},{"name":"control-1","file":".tmp-survey-c6/run3-control-1.png","bytes":57056,"w":780,"h":493,"sha256":"e6ec6db28bce"},{"name":"control-desk-1","file":".tmp-survey-c6/run3-control-desk-1.png","bytes":21199,"w":502,"h":309,"sha256":"6cc96a9bc224"},{"name":"ref1-before","file":".tmp-survey-c6/run3-ref1-before.png","bytes":56915,"w":780,"h":493,"sha256":"e57d4fb1a7c4"},{"name":"ref1-before-desk","file":".tmp-survey-c6/run3-ref1-before-desk.png","bytes":21199,"w":502,"h":309,"sha256":"6cc96a9bc224"},{"name":"ref1-after","file":".tmp-survey-c6/run3-ref1-after.png","bytes":61418,"w":780,"h":493,"sha256":"410608b87cf0"},{"name":"ref1-after-desk","file":".tmp-survey-c6/run3-ref1-after-desk.png","bytes":21198,"w":502,"h":309,"sha256":"86047bbf7366"},{"name":"ref2-before","file":".tmp-survey-c6/run3-ref2-before.png","bytes":63780,"w":780,"h":493,"sha256":"a6168350d38f"},{"name":"ref2-before-desk","file":".tmp-survey-c6/run3-ref2-before-desk.png","bytes":21196,"w":502,"h":309,"sha256":"8c67b6f20e3e"},{"name":"ref2-after","file":".tmp-survey-c6/run3-ref2-after.png","bytes":64030,"w":780,"h":493,"sha256":"be98f2ccb0a0"},{"name":"ref2-after-desk","file":".tmp-survey-c6/run3-ref2-after-desk.png","bytes":21209,"w":502,"h":309,"sha256":"d217e6adf6ef"}],"control":{"before":"091aa12d18a1","after":"e6ec6db28bce","same":false},"deskControl":{"before":"ff86a6297fcd","after":"6cc96a9bc224","same":false},"parts":[{"id":"tempo","name":"Tempo","canvasPx":null,"result":"not clicked: the position cannot be read in the screenshot","menuLinesCount":0,"addedLinesCount":0,"tail10Count":0},{"id":"songpos","name":"Song Position","canvasPx":null,"result":"not clicked: the position cannot be read in the screenshot","menuLinesCount":0,"addedLinesCount":0,"tail10Count":0},{"id":"patsel","name":"Pattern Selector","canvasPx":null,"result":"not clicked: the position cannot be read in the screenshot","menuLinesCount":0,"addedLinesCount":0,"tail10Count":0},{"id":"ref1","name":"参考点 1: Welcome 窓の Start a new project 欄(部品ではない)","canvasPx":{"x":233,"y":92},"cssPx":{"x":149.96,"y":95.23},"pageInputLine":"Engine: waiting for input (idle) input: 3 sent, 3 processed, 0 pending · engine cycle 0.02 s (54.4/s) · last input processed 0.01 s after the event, drawn 0.01 s after it · last input: button R up at screen (232, 91) (pointerup)","pagePointerInfo":"pointer: -","screenBefore":"e57d4fb1a7c4","screenAfter":"410608b87cf0","screenSame":false,"deskBefore":"6cc96a9bc224","deskAfter":"86047bbf7366","deskSame":false,"contextmenuSeen":1,"defaultPrevented":true,"engineLogCountBefore":322,"engineLogCountAfter":322,"engineLogAdded":0,"menuAdded":0,"menuLinesCount":0,"addedLinesCount":0,"tail10Count":10},{"id":"ref2","name":"参考点 2: ゲスト画面の下の灰色の所(部品ではない)","canvasPx":{"x":621,"y":433},"cssPx":{"x":399.67,"y":314.74},"pageInputLine":"Engine: computing (0.1 s) - input waits for its next idle input: 6 sent, 6 processed, 0 pending · engine cycle 0.02 s (52.0/s) · last input processed 0.09 s after the event, drawn 0.09 s after it · last input: button R up at screen (620, 432) (pointerup)","pagePointerInfo":"pointer: -","screenBefore":"a6168350d38f","screenAfter":"be98f2ccb0a0","screenSame":false,"deskBefore":"8c67b6f20e3e","deskAfter":"d217e6adf6ef","deskSame":false,"contextmenuSeen":1,"defaultPrevented":true,"engineLogCountBefore":322,"engineLogCountAfter":322,"engineLogAdded":0,"menuAdded":0,"menuLinesCount":0,"addedLinesCount":0,"tail10Count":10}],"elapsedMs":12760,"loadavg":[0.99,0.83,1.41],"cpus":4,"chromeVersion":"Chrome/157.0.8103.78","error":null}
```

## コードの読み

### 読んだ物と要点(パスと行番号つき。実行はしていない)
1. app/index.html:23〜27(#toolbar): 3 つの要素だけ。send Tab のチェックボックス(24 行)、canvas-info(25 行)、Stop engine のボタン(26 行)。Tempo・Song Position・Pattern Selector は無い。
2. app/src/input/pointer.ts:68 `onContext`: `if (opts.enabled()) e.preventDefault();` だけ。:104 で `contextmenu` を #desktop に登録。
3. app/src/input/pointer.ts:48〜59(onDown・onUp)。右ボタンも同じ経路で、preventDefault、setPointerCapture、focus を行う。:39〜47(onPointer)が tracker.pointer に渡す。
4. app/src/input/tracker.ts:23(MASK_BUTTONS の `[2, BTN.R]`)、:46〜52(BDOWN・BUP を RawInput として出す)。app/src/input/events.ts:3(IN.BDOWN = 2、IN.BUP = 3)。
5. lift/runtime/fle_input.c:133 `fle_input_host_mouse`(入口)→ :538 `translate_mouse` → :602〜604(BDOWN・BUP を、ボタンが R なら WM_RBUTTONDOWN・WM_RBUTTONUP にする)。:453 の `btn_down` は {0x0201, 0x0204, 0x0207}。lift/runtime/fle_input.h:97 で FLE_WM_RBUTTONUP = 0x0205。
6. lift/runtime/fle_input.c:893 `fle_input_defproc`(既定の処理)→ :973〜994 で WM_RBUTTONUP を受けると、WM_CONTEXTMENU(0x007B)を送る。宛先は親があれば親(:983)、送信は :992(lParam は画面座標)。:975〜976 の注: 「DefWindowProc generates the WM_CONTEXTMENU message when it processes the WM_RBUTTONUP message」。
7. lift/runtime/fle_crash1_menu.c:602 `fle_imp_TrackPopupMenu`: :608 でメニューの確認(`get_menu`。無ければ戻り値 0)、:612 で窓の確認(無ければ fle_set_last_error し、戻り値 0)、:617 `R[0] = (flags & 0x100u) ? 0u : 1u;`(TPM_RETURNCMD = 0x100 のとき 0、それ以外は 1)。:600〜601 の注: 何も表示せず、選ばれず、窓へ何も送らない(S-crash1-menu-track)。
8. lift/runtime/fle_crash1_menu.h:5: 「Nothing is displayed: a menu is a list of items. TrackPopupMenu shows nothing and the menu is dismissed without a choice」。

### 右クリックの経路(ページ → エンジン → TrackPopupMenu)
1. ページ: 右ボタンの press と release → pointer.ts:48〜59 → tracker.ts(BTN.R の BDOWN・BUP)→ エンジンへの入力。`contextmenu` は pointer.ts:104 → :68 で既定の動作を止める。**測定で確かめた**: 入力欄の `button R up`、`defaultPrevented` が true。
2. エンジン(C): fle_input.c:133 → :538 → :602〜604 で WM_RBUTTONDOWN・WM_RBUTTONUP。
3. エンジン(C、既定の処理): fle_input.c:893 → :973 の WM_RBUTTONUP で WM_CONTEXTMENU を窓(または親)へ送る(:983・:992)。
4. ゲスト(FL 本体の窓の処理。この読みでは未読): WM_CONTEXTMENU を受けて、TrackPopupMenu を呼ぶ(**推定**)。
5. TrackPopupMenu: fle_crash1_menu.c:602〜618。表示せず、窓へ何も送らず、戻り値を 0 か 1 にするだけ(:617)。

測定との対応: 1 は、入力欄と defaultPrevented で確かめた。2 と 3 は、ログに行が出ず、この測定では確かめていない。4 と 5 は、ログに行が無く、測れていない。

## 未確認
1. **Tempo・Song Position・Pattern Selector の位置**: engine-idle の画面に出ていない(欄 5-b)。部品の右クリックは送っていない。位置が読めないため。
   - 部品を出すには、確認ダイアログと Welcome 窓を閉じるか、プロジェクトを開く必要があると推定。どちらも画面のボタン(同意の選択を含む)を押すことになり、この測定の範囲外として行っていない。
2. WM_CONTEXTMENU が FL 本体の窓の処理へ届いたか: ログに行が無いため、測れていない。
3. TrackPopupMenu が呼ばれたか: fle_imp_TrackPopupMenu(fle_crash1_menu.c:602〜618)はログを出さない。測れていない。
4. メニューの有無: 参考点の画像の読みでは、メニューらしい物は見えなかった。ただし canvas の範囲は、入力を送らない 1 秒でも変わる(対照。10974〜10976 画素)。画素の差だけでは、メニューかどうかを判定できない。
5. 測れなかった項目の理由: 部品の位置が読めないこと(欄 5-b)。ページ全体の PNG は、ページの別の欄(メトリクス)が動くため、差の判定に使えない(対照で違いが出た)。
6. 起動時間: navigate から engine-idle まで 12.3〜12.8 秒。P1(10 秒)の判定には、この測り方を使えない(定義が違う)。

## 次の手順の案(推定)
1. 部品を画面に出す経路を、依頼者と決める。確認ダイアログの扱い(同意の選択を、測定でどうするか)は依頼者の判断事項。どの経路でも、製品に新しい環境変数・URL パラメータ・#ifdef は足さない(台帳 L-002)。
2. C-6 の実装の最初の段(推定): lift/runtime/fle_crash1_menu.c の fle_imp_TrackPopupMenu で、メニューの項目(fle_menu の items)を画面の窓として描く。項目を選んだら、WM_COMMAND(0x0111)を hwnd へ送る(wParam の下位 16 ビットが項目の ID)。TPM_RETURNCMD(0x100)のときは WM_COMMAND を送らず、選んだ ID を戻り値にする(選ばなければ 0)。今の :600〜601 の「何も送らない」を変える。
3. 最初の段の受け入れ(推定): 右クリックの後、canvas の範囲で、クリック位置の近くに変化が出る。項目を選ぶと、WM_COMMAND が届いたことが、既存のログの仕組みで分かる。
4. 部品の測定は、起動後の画面を読んで位置を決めてから行う。参考点は、部品の代わりにしない。

判定: **未確認**(部品の右クリックを測れていないため)。
