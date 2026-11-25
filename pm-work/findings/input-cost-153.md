# 入力の受け渡しの費用と、入力 1 件あたりの費用の削減(Issue #1397)

基準は `pm-work/findings/engine-cost-attribution.json`(#1388。場面 (iii)、製品の -Oz、各 DPR の 3 run の平均)。基準の JSON と `engine-cost-attribution.md` は書き換えていない。

## 1. 結論

- **判定: 差し戻し**(受け入れのコマンド全体は、場面 (iii) の判定の段で exit 1。8 節)。場面 (iii) の区分 E+F は、基準の 84〜87%(上限 60%)で、2 回の実行とも平均が上限を大きく超えた。busy は DPR 1.00 で 73.7〜75.4%、DPR 1.325 で 75.0〜76.5%(上限 75%)。
- 前回の報告の「受理」(busy 5,386 ms、E+F 4,281 ms)は、今回の runner では再現しなかった。同じ版でも、runner の時間帯で絶対値が約 1.7 倍動く。同じ時間帯どうしの比は前回と今回で一致する(4 節)。
- 変えたこと(製品の既定の動作。環境変数・`#ifdef`・URL パラメータ・ビルド変数は足していない):
  1. 入力 1 件ごとの `wake` の `postMessage` をやめた。page は、入力のあったフレームに 1 回だけ、リングの head に `Atomics.notify` する(`app/src/engine/client.ts` の `sendInput` と `wakeOnFrame`)。
  2. Worker は idle のとき、リングの head を `Atomics.waitAsync` で待つ(`app/src/engine/worker.ts` の `armInputWait`、`app/src/input/ring.ts` の `waitForWrite`)。
  3. Worker の期限の待ちを `setTimeout` から、タイムアウト付きの `Atomics.waitAsync` にした(`app/src/engine/worker.ts` の `deadlineWait` と `cancelDeadlineWait`)。4 ms の丸めに頼らない。
  4. 主スレッドの入力 1 件あたりの読み取りを減らした。`performance.timeOrigin` は 1 回だけ読む(`app/src/input/pointer.ts:29`、`app/src/main.ts:398`)。入力の `tPage` は、リスナが始まった時刻(`meta.tListener`)を使う(`app/src/main.ts:406`)。
  5. 窓の一覧の読み取りで、クラス名とタイトル(96 バイト)が変わった窓だけ decode する(`app/src/engine/appview.ts:39` の `textCache`)。
- 同じ runner・同じ時間帯・交互に測った変更前(HEAD)との比: busy は 63〜65%、E+F は 67〜69%(DPR 1.00 と 1.325)。Worker の忙しい時間は約 40%(−60%)、主スレッドは約 87%(−13%)。
- 残りの大きな費用: 主スレッドの pointer リスナ(`pointer.ts` の `withMeta`)の中の、ブラウザ内部の処理(`(program)`)が 1 入力あたり約 35 µs。原因は未特定(7 節)。

選んだ方式の理由(Issue の候補 (i)(ii) のどちらか): 変更前の主スレッドの `wake` の `postMessage` は、入力 1 件ごとに 1 回で、3 節の表で 235 ms(20 秒)と、Worker の idle を 11,160 回起こしていた。候補 (ii)(1 ミリ秒に 1 回の `wake`)は、主スレッドに 1 ms の周期の仕事を増やす。候補 (i) だけ(Worker の同期の `Atomics.wait`)は、Worker の event loop を止めるので、メッセージと JSPI の処理が遅れる。そこで、page が入力を書いたフレームに 1 回だけ `Atomics.notify` し(フレームの周期で、1 秒に最大約 60〜120 回)、Worker は `Atomics.waitAsync` で待つ。フレームの周期に合わせるので、入力の遅れは最大で 1 フレーム増える(`inputToPresentMsP95` は 47〜77 ms で、基準の 182.7 ms より小さい)。

## 2. 計測の方法

- 場面と計測: `tools/engine-cost/measure.mjs`(場面 (iii): welcome を閉じ、t0+30 s から 60 s、Playlist の上を 1200 Hz)。`busyMs` は **主スレッドと Worker(engine・render)の CPU プロファイルの自己時間の合計**(基準の JSON も同じ定義)。Issue の「Worker の忙しい時間」は、正確には主スレッドを含む。
- トレース: `tools/engine-cost/trace.mjs`(場面 (iii)、20 秒、DPR 1)。変更前と変更後を、行番号の付く非圧縮ビルドで取った(HEAD の app を `git archive` で別に作り、`--app` で指定。作業ツリーと index は動かしていない)。トレースは観測の費用を含むので、数字は内訳の比に使い、受け入れの数字は `measure.mjs` の値を使う。
- 同じ runner の比較: HEAD と変更後を、交互に測った(`lift/out-app/engine-cost/ab/`。git で無視される出力)。
- 出典: `lift/out-app/engine-cost/trace/head-iii-dpr1.json`・`cur-iii-dpr1.json`(トレース)、`lift/out-app/engine-cost/ab/`(A/B)、`lift/out-app/engine-cost/runs/Oz/`(受け入れの run)。

## 3. 内訳(変更前 = HEAD、トレース 20 秒、場面 (iii)、DPR 1)

### 3.1 スレッドの忙しい時間と入力の数

| 項目 | 変更前(HEAD) | 変更後 |
| --- | --- | --- |
| Worker(engine の thread)の CPU | 2,758 ms | 1,089 ms |
| 主スレッド(CrRendererMain)の CPU | 3,861 ms | 2,960 ms |
| 送った pointer 件数 | 23,999 | 24,000 |
| engine の idle の回数 | 11,160 | 3,646 |
| engine に注入された入力(`injected`) | 11,941 | 4,434 |

idle の回数は、入力ごとの `wake` が無くなったことで約 1/3 になった。

### 3.2 区分 F の `(program)` の中身(いちばん内側のトレースイベント別)

Worker(engine の thread):

| 内側のイベント | 変更前 ms(20 秒) | 変更後 ms |
| --- | --- | --- |
| `v8.callFunction`(JS を native から呼ぶ境界) | 322.2 | −15.6(帰属の残差) |
| 名前の無い関数(blob の JS。URL 名) | 27.4 | 21.0 |
| `self.onmessage`・`HandlePostMessage`・他 | 13.0 | 1.4 |
| (トレースの内側のイベントが無い) | 0 | 122.5 |
| 合計 | 362.5 | 129.3 |

`v8.callFunction` の中の `(program)` が変更前の 89% を占める。変更前は idle 1 回あたり約 32 µs、変更後は約 35 µs で、idle の回数に比例している。JSPI の積み替えや、Promise の解決に伴う native の処理が候補。ただし、これは相関の観測で、原因は確定していない。

主スレッド(CrRendererMain)の `(program)`:

| 内側のイベント(行) | 変更前 ms | 変更後 ms |
| --- | --- | --- |
| pointer リスナ `withMeta`(`app/src/input/pointer.ts:98-101`) | 1,068.3 | 851.0 |
| `w.onmessage`(`app/src/engine/client.ts:96`。Worker からの message) | 678.8 | 266.0 |
| 窓の `watch`(`app/src/audio/page-audio.ts:129`。Worker からの全 message) | 331.0 | 207.1 |
| 入力のフレーム wake(`client.ts` の `wakeOnFrame`、変更後だけ) | 0 | 84.1 |
| RunTask(ブラウザの入力の配送) | 396.7 | 480.4 |
| `v8.callFunction` | 152.4 | 185.5 |
| (トレースの内側のイベントが無い) | 101.9 | 114.2 |
| 合計 | 3,045.1 | 2,520.6 |

### 3.3 区分 E の内訳(行番号は変更前 = HEAD の行)

| # | 何か | 変更前 ms(20 秒) | 変更後 ms | 行(HEAD) |
| --- | --- | --- | --- | --- |
| a | 入力 1 件ごとの `wake` の `postMessage`(主スレッド) | 234.8 | 0 | `app/src/engine/client.ts:141`(`sendInput` は 137) |
| b | Worker の `postMessage`(`post`: frame・input-done・idle の送り) | 130(`post`) + 198(呼び元が (root)) | 62 + 88 | `app/src/engine/worker.ts:27`(`post`)。(root) の 198 ms は呼び元が特定できない。候補は、MessageChannel の `turns.port2.postMessage(0)`(`worker.ts:159`) |
| c | `decode`(`TextDecoder('latin1').decode`) | 168 | 上位 12 に入らない(12 ms 未満) | `app/src/engine/appview.ts:47`(`text`)。呼び元は `app/src/engine/fruity.ts:305`(`readAppView`)と `app/src/engine/worker.ts:443`(`deskTick`) |
| d | `setTimeout` / `clearTimeout`(idle の期限の待ち) | 105 / 77 | 0 / 0 | `app/src/engine/worker.ts:157-158`(`setTimer`・`clearTimer`)。呼び元は `app/src/engine/idle-driver.ts:205` |
| e | 矩形の読み取り(`getBoundingClientRect`) | 83.7(主) | 69.5(主) | `app/src/ui/desktop.ts:43`(`rect`)。呼び元は `app/src/input/pointer.ts:36`(`at`) |

(d) は `decode` の答え: 窓の一覧の読み取りで、クラス名(32 バイト)とタイトル(64 バイト)を、`TextDecoder` で文字列にしていた。入力のたびに窓の状態(seq)が変わるので、毎回読み直していた。変更後は、96 バイトが変わった窓だけ decode する。

## 4. 入力 1 件あたりの費用

送った件数(60 秒)は 72,000 件。DPR 1.00、同じ runner・交互に測った HEAD と変更後の 1 回ずつ(`ab/`)。

| 項目 | 変更前(HEAD) | 変更後 |
| --- | --- | --- |
| Worker の忙しい時間 ÷ 件数 | 6,850 ms ÷ 72,000 = 95 µs | 2,745 ms ÷ 72,000 = 38 µs |
| 主スレッドの忙しい時間 ÷ 件数 | 7,572 ms ÷ 72,000 = 105 µs | 6,608 ms ÷ 72,000 = 92 µs |

## 5. 計測結果

### 5.1 受け入れの run(`run.sh --scene iii`、各 DPR の n=3。`error` は全て null)

| 実行 | DPR | busy(平均) | 基準 | busy の比(上限 75%) | E+F(平均) | 基準 | E+F の比(上限 60%) |
| --- | --- | --- | --- | --- | --- | --- | --- |
| 1 回目(段ごと) | 1.00 | 9,131.7 ms | 12,387.9 | 73.7% ✓ | 7,472.0 ms | 8,878.2 | 84.2% ✗ |
| 1 回目(段ごと) | 1.325 | 9,328.8 ms | 12,202.0 | 76.5% ✗ | 7,649.3 ms | 8,792.6 | 87.0% ✗ |
| 最後の全体の実行 | 1.00 | 9,345.1 ms | 12,387.9 | 75.4% ✗ | 7,607.9 ms | 8,878.2 | 85.7% ✗ |
| 最後の全体の実行 | 1.325 | 9,155.8 ms | 12,202.0 | 75.0% ✗ | 7,493.2 ms | 8,792.6 | 85.2% ✗ |

最後の全体の実行では、DPR 1.325 の n=3 の 1 回目が「起動の停止」で error になり、2 回目で通った(`run.sh` の再試行)。6 run の `error` は全て null。最後の実行の busy の各 run は、DPR 1.00 が 9,313・9,244・9,479、DPR 1.325 が 8,951・8,949・9,568(ms)。

### 5.2 同じ runner の A/B(60 秒。交互。`ab/`)

| DPR | 組 | n | busy | E | F | E+F |
| --- | --- | --- | --- | --- | --- | --- |
| 1.00 | 変更前(HEAD) | 1, 2 | 14,459.5, 14,641.7 | 4,047.3, 4,127.3 | 6,949.5, 7,041.9 | 10,996.8, 11,169.2 |
| 1.00 | 変更後 | 1, 2 | 9,392.3, 9,404.9 | 1,556.9, 1,537.1 | 6,042.4, 6,144.4 | 7,599.3, 7,681.5 |
| 1.325 | 変更前(HEAD) | 1–3 | 14,862.0, 14,991.7, 15,180.5 | 4,214.4, 4,237.5, 4,339.4 | 6,990.6, 7,149.8, 7,171.5 | 11,205.0, 11,387.3, 11,510.9 |
| 1.325 | 変更後 | 1–3 | 9,581.3, 9,158.5, 9,509.2 | 1,579.3, 1,491.8, 1,577.1 | 6,179.8, 6,003.2, 6,096.3 | 7,759.1, 7,495.0, 7,673.4 |

平均の比(変更後 ÷ 変更前): DPR 1.00 は busy 64.6%・E+F 68.9%、DPR 1.325 は busy 62.7%・E+F 67.2%。

変更前(HEAD)は、同じ runner では基準の JSON に対して busy 117〜123%、E+F 125〜129% になる。この runner は、基準を取った時より遅い。判定の基準は絶対値なので、この時点では、変更前の版も判定に届かない。

DPR 1.00 の n=3 は、両方の組で「起動の停止」(`no engine-idle within 180000 ms`)になり、欠けている。

### 5.3 前回との対応

前回の報告の同じ時間帯の比: busy 8,604 → 5,143(60%)、E+F 6,332 → 4,104(65%)。今回の比は、busy 63〜65%、E+F 67〜69%。どちらも同じ傾向で、絶対値が違う(前回の runner は約 1.7 倍速い)。

## 6. 他の受け入れの項目

- `measure-live.mjs`(入力 1000 Hz・再生中): `{"error":null,"playing":true,"underrunDelta":0,"last60sUnderruns":0,"inputRateOk":true,"inputSentPerSec":751.9,"inputProcessedPerSec":750.6,"pointerSent":72000,"pointerErrors":0}`。**通った**。
- `measure-live.mjs`(最後の実行): `{"error":null,"playing":true,"underrunDelta":0,"last60sUnderruns":0,"inputRateOk":true,"inputSentPerSec":761.4,"inputProcessedPerSec":760.2}`。**通った**。
- `measure-size.mjs`(DPR 1): 2 回の試行。1 回目は行が `{"dpr":1,"pointerErrPx":0,...}`、`{"dpr":1.79,"pointerErrPx":0,...}` の 2 行で、判定の検査は通ったが、DPR 1.93 の行が「起動の停止」で error になり、道具は終了コード 1 を返した。最後の実行では、校正(viewport 2176x1092)が起動の停止で止まり、行は出なかった(終了コード 1)。**通っていない**。
- `p2-frames`(DPR 1、負荷 a): 1 回目 `{"error":null,"inputRateOk":true,"inputToPresentMsP95":77.34,"valid":true}`、最後の実行 `{"error":null,"inputRateOk":true,"inputToPresentMsP95":72.97,"inputToPresentMsP50":42.17,"valid":true}`。219 ms 以下。**通った**(3 回の測り直しは不要)。
- 流れ A〜D(`welcome-flow.mjs`): 1 回目は A・C・D が通り、B が起動の停止(`no engine-idle within 180000 ms (state running)`、start の 3 回が 40 秒で止まった)。最後の実行は A・B・D が通り、C が起動の停止で失敗。通ったものの `elapsedMs` は 11,101〜11,524 ms、`preBootBytes` は 311,123,285 B(上限内)。**通っていない**(起動の停止)。

## 7. 残った費用と、次の一手

1. **主スレッドの pointer リスナの `(program)`**(変更後 851 ms/20 秒、1 入力あたり約 35 µs)。最大の残りの項目。リスナの中で呼ぶ DOM の属性の読み取りと、`getBoundingClientRect` は、JS 本体の時間(約 200 ms/20 秒)より大きい。原因の切り分けは、リスナの本体を外した版で測るのが速い。次の作業。
2. **Worker から page へのメッセージ**(idle ごとに 1 件、`input-done`、`frame`)。主スレッドの `w.onmessage` と `watch`、`HandlePostMessage` で約 19%(変更後の busy に対して)。idle の報告をまとめると減るが、状態の表示と計測の意味が変わるので、別の作業にする。
3. **起動の停止**(`state running` のまま 180 秒): 6 節と 8 節の通り、変更前にも変更後にも出た。前回の報告にも既知として書いてある(#1393 の系)。この変更の経路(最初の idle を報告した後に入る `armInputWait`)より前で止まっているので、この変更が原因とは見ていない。確かめてはいない。
4. **判定の基準の問題**: 基準の JSON は別の runner・別の時間で取られている。同じ版でも busy は runner の時間帯で 1.7 倍動く(前回と今回)。絶対値の判定は、runner で合否が決まる。同じ runner の変更前との比で判定する案を、依頼者と決めたい(Issue の「境界に近いときは同じ runner で測り直す」を、判定の本体にする)。
5. 基準の定義: `busyMs` は主スレッドを含む(Issue の「Worker の忙しい時間」は違う)。判定の定義は変えていない。

## 8. 受け入れの判定(この版)

受け入れのコマンド全体(`npm run build` から流れ A〜D まで、この版 `8902f28d` の作業ツリー)は、場面 (iii) の判定の段(`node -e` の 1 段目)で exit 1 で止まった。その段の値は 5.1 の最後の行。後ろの段(live・size・p2・流れ)は、同じ build で個別に走らせた(6 節)。live と p2-frames は通った。size と流れは、起動の停止で通らなかった。

起動の停止(`state running` のまま 180 秒、engine-idle にならない)は、この session で、変更前(HEAD)の run にも、変更後の run にも出た(A/B の HEAD DPR 1.00 n=3、流れ B・C、size の DPR 1.93 と校正)。試行のおよそ 1 割。この変更の経路では、最初の idle が報告された後に `armInputWait` が入るので、起動の停止の前段には入らない。原因は確かめていない(#1393 の系)。
