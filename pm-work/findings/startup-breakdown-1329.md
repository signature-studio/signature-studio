実エンジン: lift/out-app の engine と bundle を fe-build engine・fe-build bundle で作り(bundle は raw/fl-Oz。engine wasm 168,996,523 B)、tools/startup-breakdown/measure.mjs で headless Chrome(Google Chrome 157.0.8103.78)を 3 回起動した。出力の JSON 3 行は、state が engine-idle、standIn が false、error が null、phases の 5 段がすべて数値。受け入れコマンドは 0 で終わった。

# startup-breakdown-1329: 起動の段ごとの実測と、P1 の測り方の確定

- 種別: 測定。製品(app/・lift/・ci/)は変えていない。変えたのは tools/startup-breakdown/measure.mjs と、この文書。
- 判定: **受理**。3 回とも engine-idle に達し、段の時刻がすべて出た。値の目標(10 秒)は判定に入れていない(#1330)。
- 数字は、すべて runner の値。この機械とは別枠。nproc = 4。loadavg(1 分・5 分・15 分)は各回の終わりの値: 1 回目 1.34 1.54 2.33、2 回目 1.59 1.59 2.33、3 回目 1.90 1.65 2.35。同じジョブで直前に engine をビルドしたため、1 分の値は高め。

## 1. P1 の測り方(確定)

- 始まり: `Page.navigate` を送る直前(Node の `Date.now()`)。
- 終わり: Worker の最初の `idle` の印(`window.__app.timeline.idle`。engine が最初にアイドルに入った時点)。
- 値: `navToIdleMs`(= `performance.timeOrigin + timeline.idle` − 始まり)。

理由:
1. 始まりは navigate(Issue の案どおり)。ページ内の navigation start との差は 5.0〜12.3 ms(3 回)で、始まりの選び方で値はほぼ変わらない。
2. 終わりは state の確認(poll)ではなく印。poll は 1 秒おきで、旧の測り方はこの間隔を含んでいた。この実行では、poll の読みは印より 554〜736 ms 遅かった(`pollSeenMs`)。印と state の変化の差は数 ms: 印の 3〜5 ms 後に `host_files_released` の印があり、その後に state が engine-idle になる(fruity.ts の fleIdle、worker.ts の idle の message、main.ts の idle())。
3. 終わりは「最初の idle」。起動の処理が終わり、入力を待つ最初の時点。first_pixel は、この後 352〜362 ms(3 回)。first_pixel を P1 の終わりに含めるかは、この Issue では決めず、#1330 に渡す。

## 2. 段の時刻(3 回の中央値と範囲。実測)

| 項目 | 定義 | 中央値 (ms) | 範囲 (ms) |
|---|---|---|---|
| navToIdleMs | navigate → 最初の idle の印 | 7410.1 | 7369.3〜7565.9 |
| timelineIdle | ページの navigation start → 最初の idle の印 | 7402.6 | 7364.3〜7553.6 |
| fetchWasm | engine wasm の転送(resource timing の responseEnd − startTime) | 1358.3 | 1301.3〜1413.1 |
| compileWasm | compile_done − engine wasm の responseEnd(転送の後に残った分) | 372.3 | 281.8〜376.8 |
| fetchPacks | seed pack と DLL pack(早い開始 〜 遅い終わり) | 1134.8 | 1097.5〜1270.2 |
| instantiate | engine_start → wasm_instantiated(Worker の WebAssembly.instantiate) | 40.0 | 35.1〜45.8 |
| idle | wasm_instantiated → 最初の idle(ゲストの起動) | 5444.5 | 5425.8〜5539.0 |

compileWasm の中身: compileStreaming は転送と重なって進むので、compile_done − responseEnd は「転送の後に残った分」。この中に SHA-256 の検証(WebCrypto)が入る。assets report の hash は 3 回とも 172〜174 ms(stderr の wasmSplit)。残りのコンパイルの末尾は 110〜203 ms(計算)。

フェーズの外の区間(ページの印から。中央値)。これらは phases に入れていない:

| 区間 | 中央値 (ms) |
|---|---|
| navigation start → page_script(HTML と JS の読み込み) | 102.2 |
| page_script → manifest_loaded | 13.0 |
| manifest_loaded → download_done(4 つの取得が並列。上の fetch と compile を含む) | 1709.2 |
| download_done → engine_start(Worker の生成と glue の読み込み。client.init) | 91.9 |
| idle → first_pixel(参考) | 356.4 |

区間の中央値の和は 7400.8 ms で、timelineIdle の中央値 7402.6 ms と 2 ms 違う(中央値は加法的でない)。

読み: 最も大きいのは wasm_instantiated → 最初の idle(約 5.4 s、timelineIdle の約 74%)。その中で何が時間を使うかは、この Issue では測っていない。取得の窓は約 1.7 s で、engine wasm(約 1.36 s)と packs(約 1.13 s)が並列に走る。

## 3. 旧の値との対応

| 測り方 | 旧の値(出典) | この Issue の対応する値(中央値) | 注 |
|---|---|---|---|
| A. boot-bundle の timelineIdle | 9.2〜9.4 s(survey-c2: 9288.7・9260.8・9401.5 ms。予備 10082 ms) | timelineIdle 7402.6 ms(7364.3〜7553.6) | 同じ定義。約 1.9 s 小さい。原因は未確認。Chrome は survey-c6 と同じ 157.0.8103.78(c2 の版は記録が無い)。engine wasm は旧 167.4 MB → 今 169.0 MB。runner の時期と負荷も違う |
| B. navigate から engine-idle の確認まで(poll を含む) | 10.2〜10.6 s(survey-c1: 10205・10211・10584 ms。追加 2 回 10231・10404 ms) | pollSeenMs 8120 ms(8078〜8146) | 同じ 1 秒おきの poll の読みに当たる。約 2.1 s 小さい。原因は A と同じく未確認 |
| C. probe を含む | 12.3〜12.8 s(survey-c6: probe 12330、3 回 12807・12849・12760 ms) | 対応する値なし | survey-c6 の同じ実行の timelineIdle は 9036〜9124 ms。その経過時間は、それより 3.6〜3.8 s 長い(最初の idle の後の撮影と入力の手順とみられる。推定)。この Issue の測り方は、idle の後の処理を含まない |

新しい P1 の値(navToIdleMs の中央値 7410.1 ms)は、旧の値と同じ定義ではない。上の表のとおり、対応は測り方の対応だけで付けている。

## 4. 取れなかった段と理由

- 取れなかった段: なし。3 回とも 5 段の値が数字で出た。
- 取得の経路: 毎回新しいプロファイルで起動したので、OPFS の複製は無く、3 回とも network の経路(wasmSplit の origin が network)。
- 段に入れていないが時間の出ているもの: OPFS への保存(store)は 2138〜2208 ms。検証の後に背景で書かれ、起動の段は待たない。CPU と I/O を取って起動を遅らせるかどうかは未確認。glue の取得は、取得の並列の中にあり、時間は測っていない。
- 測っていないもの: wasm_instantiated → idle の内訳。warm の状態(OPFS に複製がある場合)の起動。

## 5. 比べるときの条件

- 毎回の起動は新しいプロファイル(OPFS と HTTP キャッシュが空)。
- 3 回だけの測定。navToIdleMs の 3 回の範囲は 196 ms。
- 直前に同じジョブで engine をビルドした(loadavg が高い)。
- measure.mjs は、boot-bundle.mjs と同じ起動の引数(ISOLATION)を使う。poll は 1 秒おき。

## 6. 次の手順(推定。#1330 へ)

- 最も大きい wasm_instantiated → 最初の idle(約 5.4 s)の内訳を測る(推定: ゲストの起動の処理と、ホストのファイルの読み)。
- 取得の窓(約 1.7 s)は、engine wasm(約 1.36 s)と packs(約 1.13 s)が並列で、どちらも長い。compile の末尾(約 0.37 s)の SHA-256 は約 0.17 s(推定の対象)。
- P1 の終わりに first_pixel を含めるかを #1330 で決める。

## 7. 実行の出力(startup.jsonl。3 行)

```json
{"state":"engine-idle","standIn":false,"error":null,"navToIdleMs":7565.9,"timelineIdle":7553.6,"phases":{"fetchWasm":1358.3,"compileWasm":372.3,"fetchPacks":1270.2,"instantiate":45.8,"idle":5539}}
{"state":"engine-idle","standIn":false,"error":null,"navToIdleMs":7410.1,"timelineIdle":7402.6,"phases":{"fetchWasm":1301.3,"compileWasm":376.8,"fetchPacks":1134.8,"instantiate":40,"idle":5444.5}}
{"state":"engine-idle","standIn":false,"error":null,"navToIdleMs":7369.3,"timelineIdle":7364.3,"phases":{"fetchWasm":1413.1,"compileWasm":281.8,"fetchPacks":1097.5,"instantiate":35.1,"idle":5425.8}}
```

## 未確認

- 旧の値との差(A で約 1.9 s、B で約 2.1 s)の原因。runner の違いか、bundle の違いか、測り方の細部か。確かめていない。
- wasm_instantiated → 最初の idle の中身。
- first_pixel を P1 の終わりに含めた場合の値(参考の 356 ms は、この 3 回の差)。
- 見た目(画面の表示の速さ)と、音(P3)への影響。
