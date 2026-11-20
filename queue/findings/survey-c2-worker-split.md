実エンジン: 実行あり。headless Chrome(拡大率 1)で bundle の fl-Oz を起動し、engine-idle の後 10 秒の window.__app(deskStats・audio)と CDP の Target.getTargets を観測した(予備 1 回 + 本測定 3 回)。

# survey-c2-worker-split: C-2(描画が音声処理を圧迫しない)の Worker 分離の現状(測定のみ)

依頼: 台帳 L-033 の (2)「画面の描画処理が音声処理を圧迫している。Worker 分離は完了していないのか」。queue/plan.md の C-2 の測り方。
製品(app/・lift/・tools/・patches/・ci/・.github/)は変えていない。変えたのはこの文書だけ。耳の確認は行っていない。

注意:
- 数字はすべて runner の値。この機械とは別枠。nproc = 4。loadavg は、実行の前が `0.33 0.93 1.68`(`/proc/loadavg`)。各実行の終わりの値は、下の JSON の `loadavg` にある。
- P3 の所定のプロジェクトは runner に無い(入力が無い)。Song Mode の音の途切れは、この作業では測っていない。
- 待ちは、`boot-bundle.mjs` と同じ `cdp.mjs` の JS タイマーで行った(シェルの sleep は使っていない)。

## 判定

- C-2 (a) 描画の合成と提示が、音声の書き込みと別の Worker で動く: **差し戻し**。現状は、描画の合成(compose)と提示(putImageData)も、音声の書き込み(fle_js_publish)も、同じ fl-engine の Worker で動く。根拠は下の「コードの読み」と、Target の数(Worker は 1 つ)。
- C-2 (b) P3 の underrun が 0(所定のプロジェクト、Song Mode、n=3): **未確認**。所定のプロジェクトが無く、AudioWorklet が再生の quantum を数えていないため(下の「実測」の手順 6)。

## 実測

### 手順 1: 版と抽出物

- `grep '^EMSDK_VERSION=' ci/versions.env` → `EMSDK_VERSION=7.0.0`
- `find lift/out-app/host -type f | wc -l` → `238`

### 手順 2: ビルド(queue/kinds/lower.md の accept と同じ段)

| 段 | 終了コード | 出力の要点 |
|---|---|---|
| `npm ci --prefix app` | 0 | `added 74 packages`、`found 0 vulnerabilities`(install-scripts の警告 2 件。警告のみ) |
| `npm run build --prefix app` | 0 | 型検査の 4 段と vite build。`✓ built in 177ms` |
| `cargo build --release --locked --manifest-path lift/Cargo.toml` | 0 | `Finished release profile ... in 0.01s`(既存の target を使った。警告は fe-wasmgen の 7 件) |
| `lift/target/release/fe-build engine` | 0 | 末尾 `hybrid: 167414828 bytes, 8 direct modules` |
| `lift/target/release/fe-build bundle` | 0 | `engine wasm  167414828 B`、`seed pack 20780949 B`、`dll pack 88675800 B  11 files`、`stored as br (17717323 B)` |

engine wasm は 167,414,828 B。queue/plan.md(C-4 の論点)が書く 165,848,536 B は、計画が引いた過去の run の値で、この runner の build とは違う。

### 手順 4〜6: 起動後の deskStats・Target・音声の項目

測り方: `tools/boot-check/boot-bundle.mjs` の main を一時ファイル `tools/boot-check/.tmp-survey-c2.mjs` に写した(終わりに削除)。state が engine-idle になるまで待ち、その後 1 秒ごとに 10 回(t=0 を含めて 11 回)、`window.__app` の `state` と `deskStats` を読んだ。終わりに `Target.getTargets` を type ごとに数え、ページのリソースの `.js` の名前(ハッシュを除いた名前)と、`window.__app.audio.detail()` を読んだ。`engineLog30`(エンジンのログの先頭 30 行。ゲストの仮想パスを含む)だけは、下の JSON から外した。他は実測のまま。

| 項目 | 予備 (run 1) | run 2 | run 3 | run 4 |
|---|---|---|---|---|
| timeline.idle(ms。最初の engine-idle) | 10082.0 | 9288.7 | 9260.8 | 9401.5 |
| 提示 presents(t=0 → t=10) | 1 → 45 | 5 → 43 | 5 → 42 | 4 → 42 |
| composeMs(t=10 の累計) | 18.27 | 18.49 | 20.59 | 18.23 |
| putMs(t=10 の累計) | 10.21 | 10.55 | 10.52 | 10.04 |
| 提示 1 回あたり(計算: 累計 ÷ 提示回数)compose / put | 0.41 / 0.23 | 0.43 / 0.25 | 0.49 / 0.25 | 0.43 / 0.24 |
| 提示のない間の composeMs 増分 / 秒(t=6 → 10) | 0.63 | 0.37 | 0.46 | 0.54 |
| idleTicks 増分 / 秒(t=6 → 10) | 47 | 36 | 36 | 48 |
| sleeps(t=10) | 1 | 1 | 1 | 1 |
| 音声 status / ctxState / quantaWhilePlaying / underrunQuanta | needs-gesture / suspended / 0 / 0 | 同左 | 同左 | 同左 |
| engineGapFrames(音声の項目) | 121805 | 107977 | 106126 | 106766 |

- 提示の増分は、提示が止まった後(t=6 以降)でも composeMs と idleTicks が増えている。これは、engine の idle の経路(worker.ts:226)が約 20 ms ごとに deskTick を呼び、画面が動かなくても compose を走らせるため(コードの読みを参照)。
- deskStats は Worker が 'desk' を送った時点の値で、提示のない間は最大 1 秒に 1 回しか送らない(worker.ts:410-416)。1 秒ごとの差には、この遅れが入る(run 2 の t=7 と run 3 の t=8 で増分が 0 になっているのがそれ)。このため、増分は t=6 → 10 の 4 秒の区間で書いた。
- 「提示 1 回あたり」は実測の累計を提示回数で割った計算で、推定ではない。初回の全画面の提示を含む。

### 手順 5: スレッドの一覧(Target.getTargets)

- type の数は、4 回とも同じ: `service_worker` 4、`worker` 1、`page` 2、`browser_ui` 2。
- `worker` の名前は `worker`(ページのリソースの `.js`。initiatorType は `other`)。Target.getTargets の `worker` の URL は空だった(`name: ""`)。このため名前はリソースの一覧で取った。これは repo の app/src/engine/worker.ts(client.ts:75 で fl-engine として作られる)に当たる。
- `service_worker` の 4 件は `thunk.js` ×2、`background.js`、`service_worker.js`。app/src に service worker の登録は無い(grep で 0 件)。出どころは未確認。
- AudioWorklet(`fle-out`)は Target.getTargets に出なかった。
- chunk-io-worker は起動されていない(「コードの読み」の chunk-io の項)。

### 手順 6: 音声の項目(window.__app.audio.detail())

run 2〜4 の値: status `needs-gesture`、ctxState `suspended`、deviceState `playing`、quantaWhilePlaying `0`、underrunQuanta `0`、nonSilentQuanta `0`、engineGapFrames 106126〜107977。

判定: **underrun の 0 は測定ではない**。AudioContext が suspended のまま(ユーザーの操作が無く、ブラウザが再生を許していない)で、AudioWorklet が再生の quantum を 1 つも数えていない(quantaWhilePlaying = 0)。この間、製品内の SoftSink は、AudioWorklet が動かないときに読み位置を page のタイマー(page-audio.ts:146、10 ms)で進める作りになっている(app/src/audio/soft-sink.ts:1-6 の注釈)。この実測では softSinkFrames を取っていないので、進んだ量は未確認。engineGapFrames は、エンジンが穴を埋めた無音のフレーム数(page-audio.ts:37 の注釈どおり)で、値は run ごとに 106126〜121805 と出ている。AudioWorklet が読まなかった間に積もったものとみられる(推定)。P3 の判定には使わない。

### 手順 7: 3 回の JSON 行

本測定は run 2・3・4 の 3 回。run 1 は予備で、worker の名前を取る項目(手順 5)を足す前に同じ式で 1 回走らせたもの。JSON は 1 行ずつ、`engineLog30` だけを node で外した(他は改変していない)。

予備 (run 1):

```json
{"startedAt":"2026-11-20T18:18:47.992Z","state":"engine-idle","engineKind":"emscripten-fruity-v1","timelineIdle":10082.02001953125,"standIn":false,"documentTitle":"Welcome to FL Studio - in ?trial mode?","elapsedMs":20400,"idleToSamplesMs":10029,"samples":[{"t":0,"state":"engine-idle","deskStats":{"presents":1,"rects":2,"pixels":875202,"composeMs":1.7149999998509884,"putMs":2.5899999998509884,"whole":1,"drawMs":0.2800000002607703,"uploads":2,"draws":2,"texAllocs":2,"uploadBytes":3500808,"ringReads":0,"ringFull":0,"wakes":0,"ticks":0,"idleTicks":1,"sleeps":0}},{"t":1,"state":"engine-idle","deskStats":{"presents":6,"rects":12,"pixels":3011745,"composeMs":5.850000000093132,"putMs":6.979999999515712,"whole":1,"drawMs":0.6900000004097819,"uploads":12,"draws":12,"texAllocs":2,"uploadBytes":12046980,"ringReads":0,"ringFull":0,"wakes":0,"ticks":1,"idleTicks":10,"sleeps":0}},{"t":2,"state":"engine-idle","deskStats":{"presents":10,"rects":20,"pixels":4275815,"composeMs":8.710000000428408,"putMs":8.06499999947846,"whole":1,"drawMs":0.9150000005029142,"uploads":20,"draws":20,"texAllocs":2,"uploadBytes":17103260,"ringReads":0,"ringFull":0,"wakes":0,"ticks":1,"idleTicks":16,"sleeps":0}},{"t":3,"state":"engine-idle","deskStats":{"presents":15,"rects":30,"pixels":4660443,"composeMs":11.574999999720603,"putMs":8.549999999813735,"whole":1,"drawMs":1.2450000010430813,"uploads":30,"draws":30,"texAllocs":2,"uploadBytes":18641772,"ringReads":0,"ringFull":0,"wakes":0,"ticks":1,"idleTicks":24,"sleeps":0}},{"t":4,"state":"engine-idle","deskStats":{"presents":21,"rects":38,"pixels":5044375,"composeMs":13.330000000540167,"putMs":9.034999999217689,"whole":1,"drawMs":1.6000000005587935,"uploads":38,"draws":42,"texAllocs":2,"uploadBytes":20177500,"ringReads":0,"ringFull":0,"wakes":0,"ticks":1,"idleTicks":33,"sleeps":0}},{"t":5,"state":"engine-idle","deskStats":{"presents":36,"rects":54,"pixels":5574833,"composeMs":15.359999999403954,"putMs":9.919999998994172,"whole":1,"drawMs":2.460000001359731,"uploads":54,"draws":72,"texAllocs":2,"uploadBytes":22299332,"ringReads":0,"ringFull":0,"wakes":0,"ticks":1,"idleTicks":50,"sleeps":0}},{"t":6,"state":"engine-idle","deskStats":{"presents":45,"rects":63,"pixels":5579004,"composeMs":15.759999999310821,"putMs":10.209999999031425,"whole":1,"drawMs":2.7850000020116568,"uploads":63,"draws":90,"texAllocs":2,"uploadBytes":22316016,"ringReads":0,"ringFull":0,"wakes":0,"ticks":10,"idleTicks":59,"sleeps":0}},{"t":7,"state":"engine-idle","deskStats":{"presents":45,"rects":63,"pixels":5579004,"composeMs":16.369999998249114,"putMs":10.209999999031425,"whole":1,"drawMs":2.7850000020116568,"uploads":63,"draws":90,"texAllocs":2,"uploadBytes":22316016,"ringReads":0,"ringFull":0,"wakes":0,"ticks":14,"idleTicks":104,"sleeps":1}},{"t":8,"state":"engine-idle","deskStats":{"presents":45,"rects":63,"pixels":5579004,"composeMs":16.979999997187406,"putMs":10.209999999031425,"whole":1,"drawMs":2.7850000020116568,"uploads":63,"draws":90,"texAllocs":2,"uploadBytes":22316016,"ringReads":0,"ringFull":0,"wakes":0,"ticks":14,"idleTicks":152,"sleeps":1}},{"t":9,"state":"engine-idle","deskStats":{"presents":45,"rects":63,"pixels":5579004,"composeMs":17.66999999806285,"putMs":10.209999999031425,"whole":1,"drawMs":2.7850000020116568,"uploads":63,"draws":90,"texAllocs":2,"uploadBytes":22316016,"ringReads":0,"ringFull":0,"wakes":0,"ticks":14,"idleTicks":200,"sleeps":1}},{"t":10,"state":"engine-idle","deskStats":{"presents":45,"rects":63,"pixels":5579004,"composeMs":18.274999995715916,"putMs":10.209999999031425,"whole":1,"drawMs":2.7850000020116568,"uploads":63,"draws":90,"texAllocs":2,"uploadBytes":22316016,"ringReads":0,"ringFull":0,"wakes":0,"ticks":14,"idleTicks":248,"sleeps":1}}],"deltas":[{"t":1,"presents":5,"rects":10,"composeMs":4.14,"putMs":4.39,"ticks":1,"idleTicks":9},{"t":2,"presents":4,"rects":8,"composeMs":2.86,"putMs":1.08,"ticks":0,"idleTicks":6},{"t":3,"presents":5,"rects":10,"composeMs":2.86,"putMs":0.49,"ticks":0,"idleTicks":8},{"t":4,"presents":6,"rects":8,"composeMs":1.76,"putMs":0.48,"ticks":0,"idleTicks":9},{"t":5,"presents":15,"rects":16,"composeMs":2.03,"putMs":0.88,"ticks":0,"idleTicks":17},{"t":6,"presents":9,"rects":9,"composeMs":0.4,"putMs":0.29,"ticks":9,"idleTicks":9},{"t":7,"presents":0,"rects":0,"composeMs":0.61,"putMs":0,"ticks":4,"idleTicks":45},{"t":8,"presents":0,"rects":0,"composeMs":0.61,"putMs":0,"ticks":0,"idleTicks":48},{"t":9,"presents":0,"rects":0,"composeMs":0.69,"putMs":0,"ticks":0,"idleTicks":48},{"t":10,"presents":0,"rects":0,"composeMs":0.6,"putMs":0,"ticks":0,"idleTicks":48}],"targetsByType":{"service_worker":4,"worker":1,"page":2,"browser_ui":2},"workers":[{"type":"service_worker","name":"thunk.js"},{"type":"service_worker","name":"background.js"},{"type":"service_worker","name":"service_worker.js"},{"type":"service_worker","name":"thunk.js"},{"type":"worker","name":""},{"type":"browser_ui","name":""},{"type":"browser_ui","name":""}],"audio":{"status":"needs-gesture","ctxState":"suspended","deviceState":"playing","quantaWhilePlaying":0,"underrunQuanta":0,"underrunQuantaSinceFirstIdle":0,"nonSilentQuanta":0,"engineGapFrames":121805,"error":null},"chromeVersion":"Chrome/157.0.8103.78","loadavg":[0.59,0.91,1.64],"error":null}
```

run 2:

```json
{"startedAt":"2026-11-20T18:20:25.092Z","state":"engine-idle","engineKind":"emscripten-fruity-v1","timelineIdle":9288.704833984375,"standIn":false,"documentTitle":"Welcome to FL Studio - in ?trial mode?","elapsedMs":20326,"idleToSamplesMs":10028,"samples":[{"t":0,"state":"engine-idle","deskStats":{"presents":5,"rects":10,"pixels":2781323,"composeMs":5.724999999627471,"putMs":7.430000000167638,"whole":1,"drawMs":0.7549999998882413,"uploads":10,"draws":10,"texAllocs":2,"uploadBytes":11125292,"ringReads":0,"ringFull":0,"wakes":0,"ticks":1,"idleTicks":9,"sleeps":0}},{"t":1,"state":"engine-idle","deskStats":{"presents":10,"rects":20,"pixels":4275815,"composeMs":9.214999999850988,"putMs":8.62999999942258,"whole":1,"drawMs":0.9950000001117587,"uploads":20,"draws":20,"texAllocs":2,"uploadBytes":17103260,"ringReads":0,"ringFull":0,"wakes":0,"ticks":1,"idleTicks":16,"sleeps":0}},{"t":2,"state":"engine-idle","deskStats":{"presents":15,"rects":30,"pixels":4660443,"composeMs":12.114999999757856,"putMs":9.149999999441206,"whole":1,"drawMs":1.2850000001490116,"uploads":30,"draws":30,"texAllocs":2,"uploadBytes":18641772,"ringReads":0,"ringFull":0,"wakes":0,"ticks":1,"idleTicks":24,"sleeps":0}},{"t":3,"state":"engine-idle","deskStats":{"presents":21,"rects":38,"pixels":5044375,"composeMs":13.924999999813735,"putMs":9.639999999199063,"whole":1,"drawMs":1.6250000004656613,"uploads":38,"draws":42,"texAllocs":2,"uploadBytes":20177500,"ringReads":0,"ringFull":0,"wakes":0,"ticks":1,"idleTicks":33,"sleeps":0}},{"t":4,"state":"engine-idle","deskStats":{"presents":43,"rects":61,"pixels":5477107,"composeMs":15.789999999571592,"putMs":10.549999999813735,"whole":1,"drawMs":2.4300000006332994,"uploads":61,"draws":86,"texAllocs":2,"uploadBytes":21908428,"ringReads":0,"ringFull":0,"wakes":0,"ticks":14,"idleTicks":50,"sleeps":0}},{"t":5,"state":"engine-idle","deskStats":{"presents":43,"rects":61,"pixels":5477107,"composeMs":16.43999999994412,"putMs":10.549999999813735,"whole":1,"drawMs":2.4300000006332994,"uploads":61,"draws":86,"texAllocs":2,"uploadBytes":21908428,"ringReads":0,"ringFull":0,"wakes":0,"ticks":20,"idleTicks":94,"sleeps":1}},{"t":6,"state":"engine-idle","deskStats":{"presents":43,"rects":61,"pixels":5477107,"composeMs":17.00499999895692,"putMs":10.549999999813735,"whole":1,"drawMs":2.4300000006332994,"uploads":61,"draws":86,"texAllocs":2,"uploadBytes":21908428,"ringReads":0,"ringFull":0,"wakes":0,"ticks":20,"idleTicks":142,"sleeps":1}},{"t":7,"state":"engine-idle","deskStats":{"presents":43,"rects":61,"pixels":5477107,"composeMs":17.00499999895692,"putMs":10.549999999813735,"whole":1,"drawMs":2.4300000006332994,"uploads":61,"draws":86,"texAllocs":2,"uploadBytes":21908428,"ringReads":0,"ringFull":0,"wakes":0,"ticks":20,"idleTicks":142,"sleeps":1}},{"t":8,"state":"engine-idle","deskStats":{"presents":43,"rects":61,"pixels":5477107,"composeMs":17.53999999864027,"putMs":10.549999999813735,"whole":1,"drawMs":2.4300000006332994,"uploads":61,"draws":86,"texAllocs":2,"uploadBytes":21908428,"ringReads":0,"ringFull":0,"wakes":0,"ticks":20,"idleTicks":190,"sleeps":1}},{"t":9,"state":"engine-idle","deskStats":{"presents":43,"rects":61,"pixels":5477107,"composeMs":18.089999999385327,"putMs":10.549999999813735,"whole":1,"drawMs":2.4300000006332994,"uploads":61,"draws":86,"texAllocs":2,"uploadBytes":21908428,"ringReads":0,"ringFull":0,"wakes":0,"ticks":20,"idleTicks":238,"sleeps":1}},{"t":10,"state":"engine-idle","deskStats":{"presents":43,"rects":61,"pixels":5477107,"composeMs":18.49000000115484,"putMs":10.549999999813735,"whole":1,"drawMs":2.4300000006332994,"uploads":61,"draws":86,"texAllocs":2,"uploadBytes":21908428,"ringReads":0,"ringFull":0,"wakes":0,"ticks":20,"idleTicks":286,"sleeps":1}}],"deltas":[{"t":1,"presents":5,"rects":10,"composeMs":3.49,"putMs":1.2,"ticks":0,"idleTicks":7},{"t":2,"presents":5,"rects":10,"composeMs":2.9,"putMs":0.52,"ticks":0,"idleTicks":8},{"t":3,"presents":6,"rects":8,"composeMs":1.81,"putMs":0.49,"ticks":0,"idleTicks":9},{"t":4,"presents":22,"rects":23,"composeMs":1.86,"putMs":0.91,"ticks":13,"idleTicks":17},{"t":5,"presents":0,"rects":0,"composeMs":0.65,"putMs":0,"ticks":6,"idleTicks":44},{"t":6,"presents":0,"rects":0,"composeMs":0.56,"putMs":0,"ticks":0,"idleTicks":48},{"t":7,"presents":0,"rects":0,"composeMs":0,"putMs":0,"ticks":0,"idleTicks":0},{"t":8,"presents":0,"rects":0,"composeMs":0.53,"putMs":0,"ticks":0,"idleTicks":48},{"t":9,"presents":0,"rects":0,"composeMs":0.55,"putMs":0,"ticks":0,"idleTicks":48},{"t":10,"presents":0,"rects":0,"composeMs":0.4,"putMs":0,"ticks":0,"idleTicks":48}],"targetsByType":{"service_worker":4,"worker":1,"page":2,"browser_ui":2},"workers":[{"type":"service_worker","name":"thunk.js"},{"type":"service_worker","name":"background.js"},{"type":"service_worker","name":"service_worker.js"},{"type":"service_worker","name":"thunk.js"},{"type":"worker","name":""},{"type":"browser_ui","name":""},{"type":"browser_ui","name":""}],"audio":{"status":"needs-gesture","ctxState":"suspended","deviceState":"playing","quantaWhilePlaying":0,"underrunQuanta":0,"underrunQuantaSinceFirstIdle":0,"nonSilentQuanta":0,"engineGapFrames":107977,"error":null},"chromeVersion":"Chrome/157.0.8103.78","loadavg":[0.9,0.88,1.55],"error":null,"scriptResources":[{"name":"index","initiatorType":"script"},{"name":"engine-glue.136e260fc41e.js","initiatorType":"fetch"},{"name":"worker","initiatorType":"other"}]}
```

run 3:

```json
{"startedAt":"2026-11-20T18:21:07.458Z","state":"engine-idle","engineKind":"emscripten-fruity-v1","timelineIdle":9260.804931640625,"standIn":false,"documentTitle":"Welcome to FL Studio - in ?trial mode?","elapsedMs":20276,"idleToSamplesMs":10020,"samples":[{"t":0,"state":"engine-idle","deskStats":{"presents":5,"rects":10,"pixels":2781323,"composeMs":7.449999999720603,"putMs":7.305000000167638,"whole":1,"drawMs":0.8199999998323619,"uploads":10,"draws":10,"texAllocs":2,"uploadBytes":11125292,"ringReads":0,"ringFull":0,"wakes":0,"ticks":1,"idleTicks":9,"sleeps":0}},{"t":1,"state":"engine-idle","deskStats":{"presents":10,"rects":20,"pixels":4275815,"composeMs":10.894999999087304,"putMs":8.524999999906868,"whole":1,"drawMs":1.065000000409782,"uploads":20,"draws":20,"texAllocs":2,"uploadBytes":17103260,"ringReads":0,"ringFull":0,"wakes":0,"ticks":1,"idleTicks":16,"sleeps":0}},{"t":2,"state":"engine-idle","deskStats":{"presents":15,"rects":30,"pixels":4660443,"composeMs":13.834999999031425,"putMs":9.024999999906868,"whole":1,"drawMs":1.3700000001117587,"uploads":30,"draws":30,"texAllocs":2,"uploadBytes":18641772,"ringReads":0,"ringFull":0,"wakes":0,"ticks":1,"idleTicks":24,"sleeps":0}},{"t":3,"state":"engine-idle","deskStats":{"presents":21,"rects":38,"pixels":5044375,"composeMs":15.62999999942258,"putMs":9.550000000279397,"whole":1,"drawMs":1.7349999994039536,"uploads":38,"draws":42,"texAllocs":2,"uploadBytes":20177500,"ringReads":0,"ringFull":0,"wakes":0,"ticks":1,"idleTicks":33,"sleeps":0}},{"t":4,"state":"engine-idle","deskStats":{"presents":42,"rects":60,"pixels":5478310,"composeMs":17.449999999254942,"putMs":10.519999999087304,"whole":1,"drawMs":2.5949999997392297,"uploads":60,"draws":84,"texAllocs":2,"uploadBytes":21913240,"ringReads":0,"ringFull":0,"wakes":0,"ticks":13,"idleTicks":51,"sleeps":0}},{"t":5,"state":"engine-idle","deskStats":{"presents":42,"rects":60,"pixels":5478310,"composeMs":18.205000000074506,"putMs":10.519999999087304,"whole":1,"drawMs":2.5949999997392297,"uploads":60,"draws":84,"texAllocs":2,"uploadBytes":21913240,"ringReads":0,"ringFull":0,"wakes":0,"ticks":20,"idleTicks":93,"sleeps":1}},{"t":6,"state":"engine-idle","deskStats":{"presents":42,"rects":60,"pixels":5478310,"composeMs":18.774999998509884,"putMs":10.519999999087304,"whole":1,"drawMs":2.5949999997392297,"uploads":60,"draws":84,"texAllocs":2,"uploadBytes":21913240,"ringReads":0,"ringFull":0,"wakes":0,"ticks":20,"idleTicks":141,"sleeps":1}},{"t":7,"state":"engine-idle","deskStats":{"presents":42,"rects":60,"pixels":5478310,"composeMs":19.469999997410923,"putMs":10.519999999087304,"whole":1,"drawMs":2.5949999997392297,"uploads":60,"draws":84,"texAllocs":2,"uploadBytes":21913240,"ringReads":0,"ringFull":0,"wakes":0,"ticks":20,"idleTicks":189,"sleeps":1}},{"t":8,"state":"engine-idle","deskStats":{"presents":42,"rects":60,"pixels":5478310,"composeMs":19.469999997410923,"putMs":10.519999999087304,"whole":1,"drawMs":2.5949999997392297,"uploads":60,"draws":84,"texAllocs":2,"uploadBytes":21913240,"ringReads":0,"ringFull":0,"wakes":0,"ticks":20,"idleTicks":189,"sleeps":1}},{"t":9,"state":"engine-idle","deskStats":{"presents":42,"rects":60,"pixels":5478310,"composeMs":20.144999995362014,"putMs":10.519999999087304,"whole":1,"drawMs":2.5949999997392297,"uploads":60,"draws":84,"texAllocs":2,"uploadBytes":21913240,"ringReads":0,"ringFull":0,"wakes":0,"ticks":20,"idleTicks":237,"sleeps":1}},{"t":10,"state":"engine-idle","deskStats":{"presents":42,"rects":60,"pixels":5478310,"composeMs":20.594999993685633,"putMs":10.519999999087304,"whole":1,"drawMs":2.5949999997392297,"uploads":60,"draws":84,"texAllocs":2,"uploadBytes":21913240,"ringReads":0,"ringFull":0,"wakes":0,"ticks":20,"idleTicks":285,"sleeps":1}}],"deltas":[{"t":1,"presents":5,"rects":10,"composeMs":3.44,"putMs":1.22,"ticks":0,"idleTicks":7},{"t":2,"presents":5,"rects":10,"composeMs":2.94,"putMs":0.5,"ticks":0,"idleTicks":8},{"t":3,"presents":6,"rects":8,"composeMs":1.8,"putMs":0.53,"ticks":0,"idleTicks":9},{"t":4,"presents":21,"rects":22,"composeMs":1.82,"putMs":0.97,"ticks":12,"idleTicks":18},{"t":5,"presents":0,"rects":0,"composeMs":0.76,"putMs":0,"ticks":7,"idleTicks":42},{"t":6,"presents":0,"rects":0,"composeMs":0.57,"putMs":0,"ticks":0,"idleTicks":48},{"t":7,"presents":0,"rects":0,"composeMs":0.69,"putMs":0,"ticks":0,"idleTicks":48},{"t":8,"presents":0,"rects":0,"composeMs":0,"putMs":0,"ticks":0,"idleTicks":0},{"t":9,"presents":0,"rects":0,"composeMs":0.67,"putMs":0,"ticks":0,"idleTicks":48},{"t":10,"presents":0,"rects":0,"composeMs":0.45,"putMs":0,"ticks":0,"idleTicks":48}],"targetsByType":{"service_worker":4,"worker":1,"page":2,"browser_ui":2},"workers":[{"type":"service_worker","name":"thunk.js"},{"type":"service_worker","name":"background.js"},{"type":"service_worker","name":"service_worker.js"},{"type":"service_worker","name":"thunk.js"},{"type":"worker","name":""},{"type":"browser_ui","name":""},{"type":"browser_ui","name":""}],"audio":{"status":"needs-gesture","ctxState":"suspended","deviceState":"playing","quantaWhilePlaying":0,"underrunQuanta":0,"underrunQuantaSinceFirstIdle":0,"nonSilentQuanta":0,"engineGapFrames":106126,"error":null},"chromeVersion":"Chrome/157.0.8103.78","loadavg":[0.7,0.82,1.5],"error":null,"scriptResources":[{"name":"index","initiatorType":"script"},{"name":"engine-glue.136e260fc41e.js","initiatorType":"fetch"},{"name":"worker","initiatorType":"other"}]}
```

run 4:

```json
{"startedAt":"2026-11-20T18:21:41.627Z","state":"engine-idle","engineKind":"emscripten-fruity-v1","timelineIdle":9401.5,"standIn":false,"documentTitle":"Welcome to FL Studio - in ?trial mode?","elapsedMs":20315,"idleToSamplesMs":10031,"samples":[{"t":0,"state":"engine-idle","deskStats":{"presents":4,"rects":8,"pixels":2552773,"composeMs":4.374999999534339,"putMs":6.5750000001862645,"whole":1,"drawMs":0.6499999999068677,"uploads":8,"draws":8,"texAllocs":2,"uploadBytes":10211092,"ringReads":0,"ringFull":0,"wakes":0,"ticks":1,"idleTicks":8,"sleeps":0}},{"t":1,"state":"engine-idle","deskStats":{"presents":9,"rects":18,"pixels":4195651,"composeMs":8.06499999947846,"putMs":7.96500000031665,"whole":1,"drawMs":0.9099999996833503,"uploads":18,"draws":18,"texAllocs":2,"uploadBytes":16782604,"ringReads":0,"ringFull":0,"wakes":0,"ticks":1,"idleTicks":15,"sleeps":0}},{"t":2,"state":"engine-idle","deskStats":{"presents":15,"rects":30,"pixels":4660443,"composeMs":11.609999999869615,"putMs":8.584999999962747,"whole":1,"drawMs":1.349999999627471,"uploads":30,"draws":30,"texAllocs":2,"uploadBytes":18641772,"ringReads":0,"ringFull":0,"wakes":0,"ticks":1,"idleTicks":24,"sleeps":0}},{"t":3,"state":"engine-idle","deskStats":{"presents":20,"rects":37,"pixels":4993753,"composeMs":13.25,"putMs":9.019999999552965,"whole":1,"drawMs":1.6850000000558794,"uploads":37,"draws":40,"texAllocs":2,"uploadBytes":19975012,"ringReads":0,"ringFull":0,"wakes":0,"ticks":1,"idleTicks":32,"sleeps":0}},{"t":4,"state":"engine-idle","deskStats":{"presents":33,"rects":51,"pixels":5523166,"composeMs":15.139999999664724,"putMs":9.759999999776483,"whole":1,"drawMs":2.3499999991618097,"uploads":51,"draws":66,"texAllocs":2,"uploadBytes":22092664,"ringReads":0,"ringFull":0,"wakes":0,"ticks":4,"idleTicks":46,"sleeps":0}},{"t":5,"state":"engine-idle","deskStats":{"presents":42,"rects":60,"pixels":5525783,"composeMs":15.450000000186265,"putMs":10.03999999910593,"whole":1,"drawMs":2.669999999459833,"uploads":60,"draws":84,"texAllocs":2,"uploadBytes":22103132,"ringReads":0,"ringFull":0,"wakes":0,"ticks":12,"idleTicks":53,"sleeps":0}},{"t":6,"state":"engine-idle","deskStats":{"presents":42,"rects":60,"pixels":5525783,"composeMs":16.054999998304993,"putMs":10.03999999910593,"whole":1,"drawMs":2.669999999459833,"uploads":60,"draws":84,"texAllocs":2,"uploadBytes":22103132,"ringReads":0,"ringFull":0,"wakes":0,"ticks":18,"idleTicks":96,"sleeps":1}},{"t":7,"state":"engine-idle","deskStats":{"presents":42,"rects":60,"pixels":5525783,"composeMs":16.55999999726191,"putMs":10.03999999910593,"whole":1,"drawMs":2.669999999459833,"uploads":60,"draws":84,"texAllocs":2,"uploadBytes":22103132,"ringReads":0,"ringFull":0,"wakes":0,"ticks":18,"idleTicks":144,"sleeps":1}},{"t":8,"state":"engine-idle","deskStats":{"presents":42,"rects":60,"pixels":5525783,"composeMs":17.229999996721745,"putMs":10.03999999910593,"whole":1,"drawMs":2.669999999459833,"uploads":60,"draws":84,"texAllocs":2,"uploadBytes":22103132,"ringReads":0,"ringFull":0,"wakes":0,"ticks":18,"idleTicks":192,"sleeps":1}},{"t":9,"state":"engine-idle","deskStats":{"presents":42,"rects":60,"pixels":5525783,"composeMs":17.78999999957159,"putMs":10.03999999910593,"whole":1,"drawMs":2.669999999459833,"uploads":60,"draws":84,"texAllocs":2,"uploadBytes":22103132,"ringReads":0,"ringFull":0,"wakes":0,"ticks":18,"idleTicks":240,"sleeps":1}},{"t":10,"state":"engine-idle","deskStats":{"presents":42,"rects":60,"pixels":5525783,"composeMs":18.22999999905005,"putMs":10.03999999910593,"whole":1,"drawMs":2.669999999459833,"uploads":60,"draws":84,"texAllocs":2,"uploadBytes":22103132,"ringReads":0,"ringFull":0,"wakes":0,"ticks":18,"idleTicks":288,"sleeps":1}}],"deltas":[{"t":1,"presents":5,"rects":10,"composeMs":3.69,"putMs":1.39,"ticks":0,"idleTicks":7},{"t":2,"presents":6,"rects":12,"composeMs":3.55,"putMs":0.62,"ticks":0,"idleTicks":9},{"t":3,"presents":5,"rects":7,"composeMs":1.64,"putMs":0.43,"ticks":0,"idleTicks":8},{"t":4,"presents":13,"rects":14,"composeMs":1.89,"putMs":0.74,"ticks":3,"idleTicks":14},{"t":5,"presents":9,"rects":9,"composeMs":0.31,"putMs":0.28,"ticks":8,"idleTicks":7},{"t":6,"presents":0,"rects":0,"composeMs":0.6,"putMs":0,"ticks":6,"idleTicks":43},{"t":7,"presents":0,"rects":0,"composeMs":0.5,"putMs":0,"ticks":0,"idleTicks":48},{"t":8,"presents":0,"rects":0,"composeMs":0.67,"putMs":0,"ticks":0,"idleTicks":48},{"t":9,"presents":0,"rects":0,"composeMs":0.56,"putMs":0,"ticks":0,"idleTicks":48},{"t":10,"presents":0,"rects":0,"composeMs":0.44,"putMs":0,"ticks":0,"idleTicks":48}],"targetsByType":{"service_worker":4,"worker":1,"page":2,"browser_ui":2},"workers":[{"type":"service_worker","name":"thunk.js"},{"type":"service_worker","name":"background.js"},{"type":"service_worker","name":"service_worker.js"},{"type":"service_worker","name":"thunk.js"},{"type":"worker","name":""},{"type":"browser_ui","name":""},{"type":"browser_ui","name":""}],"audio":{"status":"needs-gesture","ctxState":"suspended","deviceState":"playing","quantaWhilePlaying":0,"underrunQuanta":0,"underrunQuantaSinceFirstIdle":0,"nonSilentQuanta":0,"engineGapFrames":106766,"error":null},"chromeVersion":"Chrome/157.0.8103.78","loadavg":[0.98,0.87,1.49],"error":null,"scriptResources":[{"name":"index","initiatorType":"script"},{"name":"engine-glue.136e260fc41e.js","initiatorType":"fetch"},{"name":"worker","initiatorType":"other"}]}
```

## コードの読み

### 手順 3: 3 つの grep の結果

1. `grep -n 'compose()\|putImageData' app/src/engine/desk.ts app/src/engine/desk-gl.ts`
   - app/src/engine/desk.ts:80 `const ptr = this.api.compose();`(DeskPresenter.present。compose の呼び出し)。33 は API の宣言。
   - app/src/engine/desk.ts:101 と 102 `this.ctx.putImageData(...)`(変化した矩形を canvas に出す)。43 は DeskContext の宣言。
   - app/src/engine/desk.ts:3 と 77 は注釈。desk-gl.ts:1 は、WebGL2 で描く経路の注釈(decision 0240)。
   - 呼び出し元: desk.ts の present() は worker.ts の deskTick(381 行)から呼ばれる。deskTick の呼び出し元は、worker.ts:116(入力を処理した後の提示)、208(answerInline)、226(engine の idle の経路。約 20 ms ごと)、280(run の終わり)、347(rAF の loop)。
   - engine の idle の経路: worker.ts:77 の `idle: onEngineIdle` は、app/src/engine/fruity.ts:118 の fleIdle(engine の idle の import)から呼ばれる。engine はその間 fleIdle で止まっている(fruity.ts:9 の注釈)。つまり、engine の idle の間の描画は、engine と同じ fl-engine のスレッドで走る。

2. `grep -rn 'fle_js_publish\|fle_tp_publish' lift/runtime/*.c`
   - lift/runtime/fle_audio_transport.c:54(`EM_JS fle_js_publish`。音声の ring への書き込み)、:129(`fle_tp_publish` が fle_js_publish を呼ぶ)、:178(native のとき何もしない)。
   - lift/runtime/fle_audio_ring.c:26(`js_publish` が fle_tp_publish を呼ぶ)、:174(`fle_audio_ring_append` が js_publish を呼ぶ)。
   - 呼び出しの連鎖: lift/runtime/fle_dsound.c:446(`audio_sync` が `fle_audio_ring_append` を呼ぶ)→ fle_audio_ring.c:148-176 → js_publish → fle_js_publish。
   - `audio_sync` を呼ぶのは、fle_dsound.c:471 の `fle_audio_poll`(スケジューラの lift/runtime/fle_thread.c:434 の poll_waiters が呼ぶ)と、DirectSound の呼び出し(fle_dsound.c:920・993・1201・1219・1270)。どれも guest のコードの中。
   - lift/runtime/fle_thread.h:3 は、guest のスレッドは 1 つのホストのスレッドで、協調して動くと書く。よって、音声の書き込みを呼ぶ側は、engine が run している fl-engine の Worker と同じスレッド。

3. `grep -n 'new Worker\|new AudioWorkletNode' app/src -r`
   - Worker を作る式は、製品の経路では 1 つ: app/src/engine/client.ts:75(`new Worker(new URL('./worker.ts', …), { name: 'fl-engine' })`)。
   - app/src/plugins/node-worker.ts:7 の Worker は、app/src のどのファイルからも import されていない(app/test だけが使う)。
   - AudioWorkletNode は 1 つ: app/src/audio/page-audio.ts:159(`new AudioWorkletNode(ctx, 'fle-out', …)`)。
   - client.ts:74、worker-diag.ts:1、chunk-io-worker.ts:1 の `new Worker` は注釈。chunk-io-worker を起動する式は app/src に無い。

### スレッドの配置

| スレッド | 何を走らせるか | 根拠(パスと行番号) |
|---|---|---|
| ページ(メインスレッド) | UI。取得(fetch)、WebCrypto の SHA-256、compileStreaming(engine の wasm の Module)、OPFS への保存。AudioContext の生成。engine の Worker の生成と入力の ring への書き込み。統計の表示。音声が動かない間は SoftSink(10 ms の setInterval) | app/src/main.ts:714-717(loadWasm・loadPack・fetchAsset)、:724(client.init)、app/src/bundle/loader.ts:109-110(fetch)、:197(compileStreaming)、:249(digest)、:327(取得の途中で compile する経路)、app/src/engine/client.ts:75-93、app/src/audio/page-audio.ts:143-148・:146、app/src/audio/soft-sink.ts:1-6 |
| fl-engine(Worker。app/src/engine/worker.ts) | WebAssembly.instantiate(app/src/engine/fruity.ts:133-136)、Module の run(worker.ts:277)、guest の全スレッド(協調。lift/runtime/fle_thread.h:3)、音声の混合と書き込み(fle_dsound.c:416-449、fle_audio_ring.c:148-176、fle_audio_transport.c:54)、音声の実時間の待ち(fle_audio_transport.c:108)、描画の合成(desk.ts:80)と提示(desk.ts:101-102)、engine の idle での描画(worker.ts:220-233)、rAF の描画の loop(worker.ts:338-357) | 上記 |
| AudioWorklet(fle-out) | 音声の出力(SharedArrayBuffer の ring を読んで出す) | app/src/audio/page-audio.ts:159 |
| I/O Worker(chunk-io-worker.ts) | 起動する式が app/src に無い。app/test の単体テストだけが使う | app/src/engine/chunk-io-worker.ts:1(注釈は「エンジンの Worker が起動する」と書く) |

読みの結論: 分離は途中にある。ページ・AudioWorklet(音声の出力)・engine の Worker は分かれている。しかし、engine の実行、描画の合成と提示、音声の書き込みは、1 つの fl-engine の Worker に残っている。実測の Target の数(Worker は 1 つ)と一致する。描画の所要(composeMs・putMs)は、この同じ Worker の中で計った値。音声の書き込みの所要は測っていないので、描画と音声の所要の比は未確認。

### 手順 8: 配信の方針(L-035)の読み(読みのみ)

(a) 起動時の取得とコンパイルを、どのスレッドが行うか:

| 処理 | 行うスレッド | 音声(fl-engine)と同じか |
|---|---|---|
| 取得(fetch) | ページ。app/src/bundle/loader.ts:109-110(fetchChecked)。fetchAsset(:185)・loadWasm(:189→:193)・loadPack(:219→:222)は、app/src/main.ts:714-717 からページで呼ばれる | 違う |
| 検証(WebCrypto の SHA-256) | ページ(loader.ts:249) | 違う |
| compileStreaming(engine の wasm の Module) | ページ(loader.ts:197 と :327)。結果の Module は client.init(main.ts:724)で Worker に渡る | 違う |
| OPFS への保存 | ページ(app/src/bundle/assetstore.ts) | 違う |
| instantiate(Module からインスタンスを作る) | fl-engine(app/src/engine/fruity.ts:133) | 同じ |

(b) 遅延の install tree の先例:
- app/src/engine/chunk-io.ts:3-4: エンジンは libc の呼び出しに同期で答え、fetch は非同期なので、fetch は 2 つ目の Worker(I/O Worker)が行い、エンジンのスレッドは Atomics.wait で待つ。
- app/src/engine/chunk-io.ts:11: I/O Worker 側の loop は、待ちの間 Atomics.wait で止まる。
- app/src/engine/chunk-io-worker.ts:1-2: I/O Worker の役割。ただし、これを起動する式は app/src に無い(grep)。chunk-io.ts の SabChunkClient と makeChunkIoShared を使うのは app/test だけで、install tree の組み立て(app/src/engine/lazy-tree.ts)も app/src のどのファイルからも import されていない。製品の経路には繋がっていない。
- 取得は別の Worker で行うが、取得の完了を待つのは fl-engine(音声を書くスレッド)。

| 処理 | 行うスレッド | 音声(fl-engine)と同じか |
|---|---|---|
| (b) chunk の取得 | I/O Worker(設計。未接続) | 違う |
| (b) chunk の取得の完了を待つ | fl-engine(chunk-io.ts:3-4、:11 の Atomics.wait) | 同じ(待ちで音声の書き込みが止まる) |

(c) 音声の書き込み(fle_js_publish)のスレッド: fl-engine(「読みの 2.」のとおり。スケジューラの poll_waiters から)。

(d) 結論(推定): 取得とコンパイルは、音声のスレッドの外に置けている(起動時はページで行っている)。今の設計で音声のスレッドに残るのは、engine の wasm の instantiate(fruity.ts:133)と、chunk-io の待ち(chunk-io.ts:3-4)。遅延で取った wasm を入れる場合、instantiate の所要が音声を止めるかどうかは、この作業では測っていない(未確認)。queue/plan.md の「配信の方針」(起動後にバックグラウンドで取る)に照らすと、chunk-io の設計は再生中の取得を音声のスレッドで待つ形なので、そのまま使うと音声の書き込みが待たされる(推定)。実測の個別の所要(起動時の fetch・compile・instantiate の時間)は、この作業では取っていない。

## 未確認

- P3(Song Mode の音の途切れ)は判定できない。理由は三つある。
  - 所定のプロジェクトが runner に無い(入力が無い)。
  - AudioWorklet が動いていない(AudioContext は suspended。status は needs-gesture。quantaWhilePlaying = 0)。
  - underrunQuanta = 0 は、AudioWorklet が再生の quantum を数えていない状態の値で、測定ではない。
- 音声の書き込み(fle_js_publish)の所要は測っていない。fle_audio_transport.c:98-103 は `audio-pulse` を Worker から page へ送るが、page の EngineClient.onMessage(client.ts:146-172)は `t` で分けるため、この `type: 'audio-pulse'` は処理されていない。このため、描画と音声の所要の比は取れていない。
- 描画の負荷が Song Mode(曲の再生)でどう変わるかは、測っていない。今回の計測は、起動直後の idle の間だけ。
- P2(UI が 120 fps)は測っていない。rAF の間隔は取っていない。
- AudioWorklet のスレッドの CPU 時間は、Target.getTargets に出ないため取れていない。
- service_worker の 4 件の出どころは未確認(repo に登録は無い)。
- P1(10 秒以内の起動)の判定は、この作業の対象外。timeline.idle は 9260.8〜9401.5 ms(run 2〜4)と取れたが、判定はしていない。
- 耳の確認は、この作業では行っていない。
- 手順 8(d)の instantiate の所要、取得とコンパイルの個別の所要は未測定。

## 次の手順の案

推定を含む。製品の変更は app/ を触るため、main の deploy.yml が動く(AGENTS.md に従い、報告に書く)。

1. 描画の合成と提示を fl-engine から分ける。
   - 案 A(共有メモリ): engine の画素(desk.ts:91-101 の SharedArrayBuffer の分岐が既に扱う)と、矩形の報告(desk.ts:14 の readDeskState、ヘッダ 64 バイト)を、描画の Worker が直接読む。compose は engine に残す。一貫性は、ヘッダの gen(desk.ts:52 の lastGen)で見る。
   - 案 B(矩形のコピー): fl-engine が変わった矩形だけを共有の ring にコピーし、描画の Worker が putImageData する。追加のコピーの所要は、今の putMs(提示 1 回あたり約 0.24 ms。累計 ÷ 提示回数)の中に収まると見込む(推定。未測定)。
   - canvas の受け渡し: client.ts:86 の transferControlToOffscreen を、fl-engine ではなく描画の Worker に渡す。readback(worker.ts:303-314)と WebGL の経路(desk-gl.ts)も同じ Worker に移す。
   - 画面が動かない間の compose: engine の idle の経路(worker.ts:226)が、変化が無い間も 36〜48 回/s の compose を走らせている(実測)。描画の Worker が「変化あり」の通知を受けたときだけ起きる形にすると、この分が減る。
2. P3 の測り方(所定のプロジェクトが用意できたとき):
   - Song Mode を再生し、n=3 で underrunQuanta を取る(app/src/audio/page-audio.ts の detail の項目)。
   - AudioContext は needs-gesture で止まる。runner では、CDP の Input.dispatchMouseEvent などの操作で、ユーザーの操作と同じ経路で開始させる(推定)。
   - 同時に、composeMs・putMs の秒あたりの増分(この作業の手順 4 と同じ)と、音声の書き込みの所要を取る。後者は、製品を変えずに測定用のスクリプトで Worker の `audio-pulse` を受ける形を見込む(推定。Page.addScriptToEvaluateOnNewDocument で Worker の生成を包む)。
   - 描画の分離の前と後で、同じ手順で比べる。
3. L-035 の取得の設計: 起動後に取る wasm の instantiate を fl-engine の外に置く案を、C-4・C-5 の前に検討する(推定)。chunk-io の Atomics.wait の設計は、再生中の取得には使わない案を先に決める。
