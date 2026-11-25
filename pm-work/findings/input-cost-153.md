# 入力の受け渡しの費用と、入力 1 件あたりの費用の削減(Issue #1397)

基準は `pm-work/findings/engine-cost-attribution.json`(#1388。場面 (iii)、製品の -Oz、各 DPR の 3 run の平均)。基準の JSON・`engine-cost-attribution.md`・`p2-frames-1350.md` は書き換えていない。

## 1. 結論

- **判定: 受理**。受け入れのコマンドの全段が exit 0 で通った(6 節の出力の行)。受け入れは 90 分の制限のため、2 回の shell 呼び出しに分けて行った。1 回目はビルドの段(exit 0、05:25 UTC)、2 回目は場面 (iii) の A/B・集計・live・size・p2・流れ A〜D(exit 0、06:03 UTC)。
- 場面 (iii) の同じ時間帯の A/B(変更前 = `origin/main`、変更後。交互に n=3)の中央値の比:
  - busy: DPR 1.00 は **0.653**、DPR 1.325 は **0.657**(上限 0.80)
  - E+F: DPR 1.00 は **0.701**、DPR 1.325 は **0.693**(上限 0.80)
- 入力 1 件あたり(Worker の時間 ÷ 入力の件数。トレース 20 秒、約 24,000 件): **114 µs → 32 µs**(−72%)。主スレッドを含む busyMs ÷ 入力(計測 60 秒、72,000 件、中央値、DPR 1.00): **202 µs → 132 µs**。
- 残りの大部分は区分 F(ブラウザ内部)。変更後の E+F の約 80% が F で、その中身の大部分は主スレッドの `(program)`(3.3 節)。
- 固定の基準との比(参考。判定には使わない): busy 0.767(DPR 1.00)・0.829(DPR 1.325)、E+F 0.871・0.920。

## 2. 変えたこと(製品の既定の動作。環境変数・`#ifdef`・URL パラメータ・ビルド変数は足していない)

1. 入力 1 件ごとの `wake` の `postMessage` をやめた。page は、入力を共有リングに書いたフレームに 1 回だけ、`requestAnimationFrame` で `Atomics.notify` する(`app/src/engine/client.ts:154-160` の `wakeOnFrame`。`sendInput` の `:144` が呼ぶ)。`wake` の message の型と受け口は `protocol.ts` と `worker.ts` から消した。
2. Worker は idle の間、リングの head を `Atomics.waitAsync` で待つ(`app/src/engine/worker.ts:305-309` の `armInputWait`。`onEngineIdle` の `:299` で張り直す。`app/src/input/ring.ts:54-59` の `waitForWrite`。通知は `ring.ts:42` の `notify`)。
3. idle の期限の待ちを、`setTimeout` から、タイムアウト付きの `Atomics.waitAsync` にした(`worker.ts:150-160` の `deadlineWait`・`cancelDeadlineWait`。`setTimer` は `worker.ts:174`)。4 ms の丸めに頼らない。期限は同時に 1 つだけ(`idle-driver.ts` の `timer` は `resume` で消してから、`onIdle` で置き直す)ので、取り消しの通知で他の期限が早く起きることは無い。
4. 主スレッドの 1 入力あたりの読み取りを減らした。`performance.timeOrigin` は 1 回だけ読む(`app/src/input/pointer.ts:29`・`app/src/main.ts:403`)。入力の `tPage` は、リスナが始まった時刻 `meta.tListener` を使う(`main.ts:411`)。
5. 窓の一覧の読み取りで、クラス名とタイトル(96 バイト)が変わらなかった窓は、前の文字列を使う(`app/src/engine/appview.ts:41` の `textCache`、読み取りは `:65-70`)。

方式の理由(候補 (i) と (ii) のどちらか): 候補 (i)(Worker の同期の `Atomics.wait`)は、Worker の event loop を止めて、メッセージと JSPI の処理を遅らせる。候補 (ii)(1 ミリ秒に 1 回の `wake`)は、主スレッドに 1 ms の周期の仕事を増やす。そこで、フレームごとに 1 回の `notify`(候補 (ii) の「1 フレームに 1 回」)と、Worker の `waitAsync` を組み合わせた。トレースで、`notify` のフレームの仕事は 20 秒で約 61 ms(入力 1 件あたり約 2.5 µs)。入力の遅れは最大で 1 フレーム増える(6.2 節の `inputToPresentMsP95` は 72.15 ms で、基準の 182.7 ms より小さい)。

注意: `requestAnimationFrame` は、非表示のタブでは止まる。入力は見えているタブでしか起きないので、通常の利用では遅れにならない。表示の変化(`sendInput` の `user=false`)は、タブが見えるまで届かない。依頼者の判断が要る点として 7 節に書く。

## 3. 内訳(変更前 = `origin/main`、変更後。トレース 20 秒、DPR 1)

計測の方法: `tools/engine-cost/trace.mjs`(場面 (iii)、20 秒、DPR 1)。変更前と変更後の両方を、非圧縮のビルド(`app/` の複製に `vite build --minify false`。`.tmp-unmin/` で作って消した)で取った。トレースの位置は V8 の 0 始まりの行で、コードの文字列と照合した(例: `pointer.ts` の `withMeta` の位置は、ソースの行 98 の `meta = {`)。トレースは観測の費用を含むので、数字は内訳の比に使い、受け入れの数字は 6 節の `measure.mjs` の値を使う。

### 3.1 スレッドの CPU と件数

| 項目 | 変更前 | 変更後 |
| --- | --- | --- |
| Worker(engine)の CPU | 2,743.6 ms | 764.3 ms |
| 主スレッド(CrRendererMain)の CPU | 3,879.4 ms | 2,502.4 ms |
| 送った pointer 件数 | 23,997 | 23,998 |
| engine に注入された件数(`inputs.injected`) | 12,016 | 12,336 |
| engine の idle の回数 | 11,092 | 2,592 |

idle の回数は、入力ごとの `wake` が無くなって約 1/4 になった。注入の件数が送った件数の約半分なのは、driver が受けた入力の一部だけを注入するため(理由は未確認)。

### 3.2 区分 E の内訳(変更前 → 変更後、20 秒)

| # | 何 | 変更前 ms | 変更後 ms | 場所(`origin/main` の行) | 呼び出し元 |
| --- | --- | --- | --- | --- | --- |
| a | 入力 1 件ごとの `wake` と `input` の `postMessage`(主スレッド) | 230.9 | 上位に無い | `app/src/engine/client.ts:143`(`wake`)・`:148`(`input`) | `sendInput`(`client.ts:139`) |
| b | Worker の `postMessage`(`post`。idle の報告と frame の送り) | 126.3 | 22.1 | `app/src/engine/worker.ts:27`(`post`) | `report`・`renderPost` |
| b' | Worker の `postMessage`(呼び元が無い `(root)`) | 192.3 | 55.2 | 候補は `worker.ts:160` の `turns.port2.postMessage(0)`(yieldTurn)。未確認 | (root) |
| c | `decode`(`TextDecoder('latin1').decode`。窓のクラス名とタイトルの文字列) | 149.3 | 上位に無い | `app/src/engine/appview.ts:49`(`text`) | `readAppView`(`fruity.ts:309-313`)← `worker.ts:444`(`deskTick`、`:417`) |
| d | `setTimeout`(idle の期限の待ち) | 100.1 | 上位に無い | `app/src/engine/worker.ts:158`(`setTimer`) | `idle-driver.ts:205`(期限の待ち) |
| d' | `clearTimeout` | 68.4 | 上位に無い | `app/src/engine/worker.ts:159`(`clearTimer`) | `idle-driver.ts:216`(`resume`) |
| e | `getBoundingClientRect`(主スレッド) | 74.0 | 69.0 | `app/src/ui/desktop.ts:43`(`rect`) | `app/src/input/pointer.ts:36`(`at`) |

`decode` の答え: 窓の一覧(`readAppView`)の、クラス名(32 バイト)とタイトル(64 バイト)を、`TextDecoder` で文字列にしていた。入力のたびに窓の状態が変わるので、窓の数だけ毎回読み直していた。変更後は、96 バイトが変わった窓だけ decode する。

### 3.3 区分 F の内訳(`(program)` の内側のトレースイベント別、20 秒)

Worker(engine の thread):

| 項目 | 変更前 ms | 変更後 ms |
| --- | --- | --- |
| `(program)` の合計 | 345.5 | 91.2 |
| うち、`v8.callFunction`(JS を native から呼ぶ境界)の内側 | 330.4 | 0(内側のイベントが無い区間: 82.5) |

`(program)` は idle の回数に比例する(変更前は idle 1 回あたり約 31 µs、変更後は約 35 µs)。JSPI の積み替えや Promise の解決に伴う native の処理が候補。相関で、原因は確定していない。

主スレッド(CrRendererMain):

| `(program)` の内側(行) | 変更前 ms | 変更後 ms |
| --- | --- | --- |
| 合計 | 3,077.5 | 2,041.1 |
| pointer リスナ `withMeta`(`app/src/input/pointer.ts:96-101`) | 1,134.9 | 853.7 |
| engine の Worker からの message(`app/src/engine/client.ts:98` の `w.onmessage`) | 659.9 | 144.6 |
| 音の Worker からの message(`app/src/audio/page-audio.ts:129` の `watch`) | 343.8 | 69.9 |
| フレームの `notify` の rAF(`client.ts:158`。変更後だけ) | 0 | 60.9 |
| `ThreadControllerImpl::RunTask`(ブラウザのタスクの配送) | 383.5 | 468.4 |
| `v8.callFunction` | 146.7 | 62.3 |
| (トレースの内側のイベントが無い) | 106.0 | 108.7 |

前回の報告の `w.onmessage (client.ts:96)` の 678.8 ms は、行の番号が古い(現在は `client.ts:98`)。今回の非圧縮トレースで、engine の受け口は 659.9 ms(変更前)、144.6 ms(変更後)と確定した。

### 3.4 Worker の CPU の上位(engine の wasm の関数)

| 関数 | 変更前 ms | 変更後 ms | 備考 |
| --- | --- | --- | --- |
| `wasm-function[14510]`(名前表で `f_863b50`) | 370.9 | 97.3 | idle の回数と同じ向きで減る(相関) |
| `wasm-function[137491]`(名前表で `fle_win_input_view`) | 136.3 | 51.6 | engine の計算(区分 B/D の側。用途は未確認) |

## 4. 入力 1 件あたりの費用

- トレース(20 秒、約 24,000 件): Worker 2,743.6 ms ÷ 23,997 = **114 µs** → 764.3 ms ÷ 23,998 = **32 µs**(−72%)。主スレッド: 162 µs → 104 µs。
- 計測(場面 (iii)、60 秒、72,000 件。中央値。DPR 1.00): busyMs 14,567.4 ÷ 72,000 = **202 µs** → 9,507.1 ÷ 72,000 = **132 µs**。E+F: 153 µs → 107 µs。

## 5. 計測の方法と道具

- 場面 (iii): `tools/engine-cost/measure.mjs`。welcome を閉じ、t0+30 s から 60 秒、Playlist の上を 1200 Hz で動かす。`busyMs` は、主スレッドと Worker(engine・render)の CPU プロファイルの自己時間の合計(基準と同じ定義)。
- 変更前: `--app .tmp-base/app`(`git archive origin/main app` を `npm ci`・`npm run build`)。変更後: `app/dist`(既定)。engine の bundle は両方で `lift/out-app/bundle/raw/fl-Oz`。
- 同じ時間帯の交互: (変更前, 変更後) を、DPR 1.00 と 1.325 の各々で n = 1, 2, 3 の順に測った。12 run の全てが 1 回目で成功(再試行は 0)。
- 判定: 変更後の busy 中央値 ÷ 変更前の busy 中央値 ≤ 0.80、E+F の中央値の比 ≤ 0.80(DPR ごと)。固定の基準との比は、`lift/out-app/engine-cost/ab/summary.json` の `vsFixedBaseline` に参考として出した。
- 道具: `tools/engine-cost/run.sh --scene <i|ii|iii>`(場面を指定し、集計は行わない。findings は書き換えない)、`tools/engine-cost/measure.mjs --app`、`tools/engine-cost/trace.mjs`。

## 6. 受け入れの結果(最後の run。出力の行)

### 6.1 場面 (iii) の 12 run(60 秒。全て `error` null)

| DPR | 変更 | n | busyMs | E | F | E+F | error |
| --- | --- | --- | --- | --- | --- | --- | --- |
| 1.00 | 変更前(origin/main) | 1 | 14,129.6 | 3,988.1 | 6,825.3 | 10,813.4 | null |
| 1.00 | 変更後 | 1 | 9,387.6 | 1,469.9 | 6,090.1 | 7,560.0 | null |
| 1.00 | 変更前(origin/main) | 2 | 14,567.4 | 4,080.6 | 6,939.0 | 11,019.6 | null |
| 1.00 | 変更後 | 2 | 9,507.1 | 1,526.1 | 6,204.0 | 7,730.1 | null |
| 1.00 | 変更前(origin/main) | 3 | 15,367.2 | 4,276.8 | 7,356.2 | 11,633.0 | null |
| 1.00 | 変更後 | 3 | 9,932.0 | 1,634.6 | 6,317.3 | 7,951.9 | null |
| 1.325 | 変更前(origin/main) | 1 | 14,928.8 | 4,283.5 | 6,969.5 | 11,253.0 | null |
| 1.325 | 変更後 | 1 | 9,362.4 | 1,551.9 | 6,052.4 | 7,604.3 | null |
| 1.325 | 変更前(origin/main) | 2 | 15,397.2 | 4,341.1 | 7,329.0 | 11,670.1 | null |
| 1.325 | 変更後 | 2 | 10,115.1 | 1,624.7 | 6,468.3 | 8,093.0 | null |
| 1.325 | 変更前(origin/main) | 3 | 15,686.9 | 4,416.1 | 7,385.5 | 11,801.6 | null |
| 1.325 | 変更後 | 3 | 10,267.1 | 1,708.4 | 6,440.4 | 8,148.8 | null |

中央値と比(判定の値):

| DPR | busy 中央値 変更前 → 変更後 | busy の比(上限 0.80) | E+F 中央値 変更前 → 変更後 | E+F の比(上限 0.80) | 参考: 固定の基準との比(busy / E+F) |
| --- | --- | --- | --- | --- | --- |
| 1.00 | 14,567.4 → 9,507.1 | **0.653** | 11,019.6 → 7,730.1 | **0.701** | 0.767 / 0.871 |
| 1.325 | 15,397.2 → 10,115.1 | **0.657** | 11,670.1 → 8,093.0 | **0.693** | 0.829 / 0.920 |

区分 E の中央値(変更前 → 変更後): DPR 1.00 は 4,080.6 → 1,526.1(−63%)、DPR 1.325 は 4,341.1 → 1,624.7(−63%)。区分 F は DPR 1.00 で 6,939.0 → 6,204.0(−11%)、DPR 1.325 で 7,329.0 → 6,440.4(−12%)。

`summary.json`(出力の行):
`{"1.00":{"busyRatio":0.6526284717931821,"efRatio":0.7014864423391048,"vsFixedBaseline":{"busy":0.7674525611081812,"ef":0.8706832466040415}},"1.325":{"busyRatio":0.6569441197100772,"efRatio":0.6934816325481358,"vsFixedBaseline":{"busy":0.8289729251295556,"ef":0.9204330914632759}}}`

### 6.2 他の項目(出力の行)

- `measure-live.mjs`(入力 1000 Hz・再生中): `{"error":null,"playing":true,"underrunDelta":0,"last60sUnderruns":0,"inputRateOk":true,"inputSentPerSec":776.2,"inputProcessedPerSec":775.5,"pointerSent":72000,"pointerErrors":0}`。**通った**。
- `measure-size.mjs`(`--dprs 1`): 数値の行は 3 件(`{"dpr":1,"error":null,"state":"engine-idle","pointerErrPx":0}`、`{"dpr":1.77,…,"pointerErrPx":0}`、`{"dpr":1.02,…,"pointerErrPx":0}`)。全て `error` null、`pointerErrPx` の最大は 0。`{"highDpr":{"3":"engine-idle","2.5":"engine-idle"}}`。**通った**。
- `p2-frames`(DPR 1、負荷 a): `{"error":null,"inputRateOk":true,"inputToPresentMsP95":72.15,"inputToPresentMsP50":5.155,"valid":true}`。219 ms 以下。**通った**(3 回の測り直しは不要)。
- 流れ A〜D(`welcome-flow.mjs`):
  - `{"flow":"A","error":null,"state":"engine-idle","standIn":false,"trapped":false,"hostErrors":0,"welcomeClosed":true,"opened":true,"elapsedMs":11322,"preBootBytes":311132315,"startStalls":0}`
  - `{"flow":"B","error":null,"state":"engine-idle","standIn":false,"trapped":false,"hostErrors":0,"welcomeClosed":true,"opened":true,"elapsedMs":11250,"preBootBytes":311132315,"startStalls":0}`
  - `{"flow":"C","error":null,"state":"engine-idle","standIn":false,"trapped":false,"hostErrors":0,"welcomeClosed":true,"opened":true,"elapsedMs":11405,"preBootBytes":311132315,"startStalls":0}`
  - `{"flow":"D","error":null,"state":"engine-idle","standIn":false,"trapped":false,"hostErrors":0,"welcomeClosed":true,"opened":true,"elapsedMs":11612,"preBootBytes":311132315,"startStalls":0}`
  
  4 行とも判定の条件を満たす(`elapsedMs` は 12,500 以下、`preBootBytes` は 314,368,711 以下)。**通った**。

### 6.3 起動の停止

この試行の起動(12 run・live・size・p2・流れ A〜D)では、起動の停止は 0 回だった(流れの `startStalls` は全て 0)。前回の試行の「起動の停止(`state running` のまま 180 秒)」は、この試行では出なかった。原因は未確認(#1393 の系)。

## 7. 残った費用と次の一手

1. 主スレッドの pointer リスナ(`pointer.ts:96-101`)の `(program)`: 854 ms/20 秒(約 36 µs/入力)。JS 本体の時間ではなく、ブラウザ内部の処理。原因の切り分けは、リスナの本体を外した版で測るのが速い。
2. 主スレッドの `RunTask`(入力の配送そのもの): 468 ms/20 秒。アプリの JS では減らせない見込み。
3. Worker から page への message(`post` 22 ms + `(root)` 55 ms/20 秒)。idle の報告をまとめると、さらに減る。ただし、状態の表示と計測の意味が変わるので、別の作業にする。
4. `(root)` の `postMessage` の呼び元が未特定(192 → 55 ms/20 秒)。候補は `worker.ts:160` の `turns.port2.postMessage(0)`(yieldTurn)。
5. 非表示のタブでは `requestAnimationFrame` が止まるので、入力の `notify` も止まる(2 節の注意)。依頼者が、表示の変化の遅れを許すか決める。

## 8. 前回の試行からの変更

- 判定: 固定の基準との比を判定から外し、同じ時間帯の変更前との比にした(Issue の本文どおり)。固定の基準との比は参考。
- 前回の取り込みで衝突した `app/src/engine/appview.ts` は、main 側の `cursor_made`(#1390)のヘッダの説明を残し、前回の `textCache` を入れて解消した。
- 変更の commit: `9cca3f2d`(コードと衝突の解消)。この報告は、その次の commit。
- 作業の時刻: 開始 02:57 UTC、ビルド 1 回目の完了 03:47 UTC、場面 (iii) の A/B(変更前後の試行)04:12 UTC、受け入れの最後の段の完了 06:03 UTC。
