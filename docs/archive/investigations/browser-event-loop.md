# ブラウザ上のメッセージループ・入力・描画・音声の載せ方 — JS の小さな試作と測定（FL Studio のコードは使っていない）

実行日: 2026-10-29。基準: ブランチ main、HEAD acf69cf。**この文書は何も決めない**（推奨は書くが、決めるのは PM）。作業物は `lift/harness/web/eventloop/`（試作、測定スクリプト、`results/` の生データ、README）。バイナリは置かず、再生成できる（`lift/out-web2/` に出る）。`lift/harness/web/` の既存ファイル、`lift/runtime`、生成器、決定文書は編集していない。

## 0. 要約

| 項目 | 結果（区分は §1 の凡例） |
|---|---|
| 環境 | Chrome 156.0.8078.47（V8 15.6.75.19）、headless、専用プロファイル、Linux 7.0、AMD Ryzen 9 9950X（16 コア）。**実デバイス（GPU、音声出力、実マウス・実キーボード）は無い**（§1） |
| `crossOriginIsolated` | **可**: ローカルサーバが `Cross-Origin-Opener-Policy: same-origin` と `Cross-Origin-Embedder-Policy: require-corp`（同一オリジン資源には `Cross-Origin-Resource-Policy: same-origin`）を付けたページで `true`、`SharedArrayBuffer` が使え、`performance.now()` の刻みは 5 µs。ヘッダ無しでは `false`、`SharedArrayBuffer` は `undefined`、刻みは 100 µs（実測） |
| JSPI | **有**: `typeof WebAssembly.Suspending` と `WebAssembly.promising` はどちらも `"function"`（実測。`crossOriginIsolated` の有無に関係なく）。emcc 6.0.12 の `-sJSPI` でビルドした GetMessage ループが動いた（実測） |
| 往復遅延（µs、p50 / p95 / p99、各 10,000 回） | **M-A**（ページ → SAB リング → Worker の `Atomics.wait` → SAB の返信を `Atomics.waitAsync`）: **75–90 / 105–130 / 140–230**（4 回の測定）。返信を `postMessage` にしても **75–105 / 110–155 / 150–255**。**M-B**（ページ → `postMessage` → 中断中の wasm が再開 → `postMessage`、`-O2`）: Asyncify **95–110 / 145–165 / 195–350**、JSPI **95–110 / 130–160 / 180–260**（スタック 0 段と 40 段で差なし）。比較用: wasm の無い、`postMessage` だけの非ブロッキング Worker は 90–105 / 125–160 / 175–285。Worker 同士の `Atomics` の往復は 60–75 / 85–105 / 110–175。最大値は 0.3–5 ms の外れ値がある（§3、§4） |
| Asyncify のコード膨張率 | **合成コード**（FL のコードではない。lift 済み C と同じ形）で `-O0` は **×1.85**（関数 500 / 2,000 / 4,000 / 8,000 個のどれでも 1.85〜1.86）、`-O2` は **×1.23〜1.29**（間接 call を無視する指定で 1.20〜1.26）。JSPI は ×1.0000（+91 B）。389,439,333 B の実 wasm への**外挿（読み）**: `-O0` で約 **722 MB**（V8 の上限 1 GiB の 67 %）、リンクのメモリは約 15〜19 GB、時間は約 1〜2 分（§4.4）。実際には作っていない |
| OffscreenCanvas の描画速度 | Worker 内で 1920×1080×4 B の画素バッファ（Worker の wasm メモリ）から `putImageData`: **ループ速度 4,232 fps（0.22 ms / 回、CPU 側のみ）**、dirty rect 480×270 で 62,794 fps。表示されたフレームは `raf` / `setTimeout` 駆動で **58〜60 fps**（screencast の数）。**Worker が `Atomics.wait` で戻らないループ（M-A の形）のときは `transferControlToOffscreen` の canvas が更新されない**（表示は白のまま、screencast 1〜3 フレーム）。同じ形でも `transferToImageBitmap()` + `postMessage` でページの canvas に出すと **約 50 fps**（Worker のペース 16 ms 待ち + 4 ms 由来）で表示された（§7） |
| AudioWorklet | **可**: 実時間の `AudioContext`（OfflineAudioContext ではない）が headless で `running`、時計は実時間の 0.999〜1.006 倍、`audioWorklet.addModule` と `AudioWorkletNode` が動き、Worker → SAB リング → Worklet で 48 kHz と 44.1 kHz の 128 フレームのブロックを供給し続けた（**定常状態のアンダーラン 0**: リング 1024 フレーム以上で生産側の停止 20 ms まで、4096 フレームで 50 ms まで）。リング 256 フレームでは約 45 % のブロックがアンダーラン。出力は実デバイスに出ていない（§8） |
| 描画の要点 | 「戻らないループ」の Worker では `OffscreenCanvas` の自動表示が効かない（§7）ので、M-A なら `ImageBitmap` を `postMessage` する経路が要る。M-B（中断の度にイベントループへ戻る）なら通常の `raf` / `setTimeout` 駆動と同じに表示された |
| 比較表 | §6 |
| 含意 | §10。`CreateFruityInstance` は戻らない前提で、ブロッキングする Win32 API（`GetMessageW` 1 地点、`WaitMessage` 2、`MsgWaitForMultipleObjects(Ex)` 4、`MessageBox` 10、`TrackPopupMenu` 2、`DoDragDrop` 2、`Sleep` 33 ほか）をどう「待つ」かが論点（引用）。M-C は単独では成立しない（§5） |
| 未決の点 | §12 |

## 1. 凡例、手順、環境

- **実測** = 今回のコマンドの出力（`results/` の JSON とそれを `summarize.mjs` で表にした値）。**読み** = コードと既存文書を読んだ推論、または実測値からの外挿（実行していない）。**未確認** = 調べていない。**引用** = 既存文書の記述（再測定していない）。
- 手順: `lift/harness/web/eventloop/README.md`。1 ケース = 1 つの node プロセス（`lib.mjs`）が、localhost のみのサーバ（`/coi/` は COOP/COEP 付き、`/plain/` は無し）を立て、`/usr/bin/google-chrome` を `--headless=new --no-sandbox --disable-gpu --disable-dev-shm-usage --mute-audio` と専用プロファイル（`lift/out-web2/chrome-profile/<名前>`、終了後に削除）で起動し、CDP で操作する。音声のケースだけ `--autoplay-policy=no-user-gesture-required` を足した。全ケースで終了時に自分の Chrome のプロセスが残っていないことを確認した（各 JSON の `chrome_pids_left` = 0）。ネットワークからの取得は無し。
- 機械: AMD Ryzen 9 9950X 16 コア、Linux 7.0.0-38、node v24.19.0、emsdk 6.0.12（`EM_FROZEN_CACHE=1` で `/opt/emsdk` のキャッシュは読むだけ）。測定中、他の作業（別の作業者のコンパイルなど）が動いており、loadavg は各 JSON に開始前と終了後の値を記録した（開始前 0.8〜4.0）。**絶対値は機械と負荷に依存する。run 間の振れは M-A の p50 で 15 µs 前後、p99 で 2 倍ほど**（§3.1 の 4 回）。
- **GPU は無い**。`--disable-gpu` を付けた回も付けない回も、`WEBGL_debug_renderer_info` は `ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero)), SwiftShader driver)` で、描画の数値は CPU ラスタライズ（SwiftShader）のもの（`results/draw-coi.json`、`draw-gpuflag.json`）。**実 GPU での速度、vsync への張り付き、複数モニターは未測定**。
- **音声出力デバイスは無い**（`/dev/snd` に PCM 無し）。Chrome の `AudioContext` は動いたが、出力が実デバイスに届いたか、実デバイスの遅延は未確認（`outputLatency` は開始時 0、数秒後に 0.032 s と報告されたが、これは headless の代替出力のもので意味づけできない）。
- **入力は CDP の合成**（`Input.dispatchMouseEvent` など）。実マウス・実キーボードのイベントの時刻、粒度、coalescing は未測定。
- 作業中の訂正: (1) 最初の `ma` の全体実行は、リング消費側の停止フラグの検査が無く、積み残したイベントを全部処理しようとして 10 分で止まった（修正して再実行）。(2) 描画の最初の「表示の確認」は、フレーム番号を埋めた画素が 1 行しかなく縮小で消えて白に見えた（64×64 の塊に変更）。(3) 入力の最初の実行は、合成した F5 キーでページが再読み込みされ集計が消えた（F9 に変更）。(4) 音声の最初の実行は起動時の 4〜8 ブロックのアンダーランを混ぜていた（定常状態と分けて再実行）。(5) 合成コードの run-time ビルドは、呼び出しの深さの上限が無く wasm のスタックが溢れた（深さ 60 の上限を付けた。§4.5）。

### 1.1 試作の一覧（それぞれ README に表がある）

| 試作 | 内容 |
|---|---|
| M-A | `ring.js`: SharedArrayBuffer 上の 1 生産者・1 消費者リング（64 B のスロット、満杯ならページ側は**ブロックせず**捨てて数える）+ 「最新のマウス位置」の 1 スロット（seqlock。WM_MOUSEMOVE の合体）。`ma-worker.js`: `Atomics.wait` で待つ C 風のループ（`GetMessage` 相当。決して自分のイベントループへ戻らない） |
| M-B | `mb/mb.c`: `host_get_message` が JS の Promise を待つ C のループ。同じソースを、同期 import（基準）、`-sASYNCIFY`、`-sJSPI` の 3 通りに `-O0` と `-O2` でビルド（`mb/build.sh`）。呼び出しの深さ（`deep_get(depth)`）、各段のメモリ上のローカル 256 B、各段に 24 個のスカラのローカル（`wide`） |
| 合成コード | `mb/synth.mjs`: lift 済み C と同じ形の合成コード（`void f_N(void)`、ゲストのレジスタはグローバル `R[16]`、`call` は「戻りアドレスを積む + C の呼び出し + 戻す」、間接 call は VA の表を二分探索する `fle_dispatch`）。実 FLEngine の C（`lift/out-deferred/gen/fle-new.c`、215,712,039 B）を数えた値（命令 2,528,830、call 266,954 = 10.6 %、うち `fle_dispatch` 40,803 = 15.3 %、関数 59,297 = 1 関数 43 命令）に合わせた設定（`MEANOPS=35 CALLRATE=0.106 DISP=0.153`）と、既定の設定（1 関数 68 命令、call 8 %、間接 15 %）の両方で測り、膨張率は同じだった（§4.4） |
| M-C' | 同じ合成コードに、**生成器が**「call の後の巻き戻し検査 + 関数先頭の再開 switch」を足した版（§5.2）。M-C そのもの（`longjmp` で戻る）の試作ではない |

## 2. 前提（引用）

- ホストは `CreateFruityInstance(0xC0, 0xEDE95A, host, &instance)` を呼ぶだけで、`Flags = 0` ではエンジンがウィンドウを作り、`TApplication.Run` 相当（0x86a660 が `HandleMessage` 0x86a250 を `Terminated` まで回す。`ProcessMessage` 0x86a0f0 が `PeekMessageW` / `TranslateMessage` / `DispatchMessageW`、メッセージが無ければ `Idle` 0x86b440）を**戻らずに**回す（`host-engine-boundary.md` §6.4）。
- ブロッキングに関わる Win32 API の呼び出し地点（`lift/out-sync/msgloop_counts.txt`、`sync-thread-survey.md`）: `GetMessageW` 1、`PeekMessageW` 21（18 関数）、`WaitMessage` 2（`Idle` 0x86b440 の中の 0x86b5a7 と、0x72ad50 の中）、`MsgWaitForMultipleObjects` 3 + `Ex` 1、`DispatchMessageW` 14（**10 関数**）、`SendMessageW` 216、`PostMessageW` 139、`Sleep` 33（runtime は仮想時計で即座に進める）、`MessageBoxW/A` 7 + 3、`TrackPopupMenu` 2、`DoDragDrop` 2、`SetTimer` 13、`KillTimer` 4。
- lift した C は、ゲストの `call` を C の呼び出し `f_XXXX();` に 1 対 1 で写し、`ret` は `return;`。ゲストの状態（`R[16]`、`X[16]`、`FL`、…）はグローバル、ゲストのスタックは線形メモリ（`R[4]`）で、C 関数にはほぼローカルが無い（`fle_landing`、決定 0100 だけが C の `setjmp` を使い、wasm では既定で無効 = 名前付き停止）。`fle_call_guest`（決定 0064）が runtime の手書き C からゲストの関数を呼ぶ。仮想ウィンドウ表は決定 0071。
- スレッド: エンジンは TThread 派生 33 種（`TMixerThread_DS/ASIO`、`TRenderThread`、`TWAVStreamingThread` ほか）を持ち、InitTable の範囲で `CreateThread` に静的に届くのは 2 unit（895、898）。スレッド案は `sync-thread-survey.md` §3.3 の A1（JSPI の協調スケジューラ）、A2（Asyncify）、C（fail-closed）、D（pthreads）。lift/runtime は 2 本目のスレッド作成を名前付き停止にしている。**音声スレッドが必要かは未確認**。
- wasm: 389,439,333 B、V8 の `--wasm-max-module-size` = 1,073,741,824 B、関数 98,392、1 関数の locals 最大 41,126（上限 50,000）。現在の wasm は `-O0`（`lift/harness/web/build/link.sh`）（`browser-feasibility.md`）。

## 3. M-A: Worker + `Atomics.wait`（SharedArrayBuffer のイベントキュー）

エンジン相当の Worker は、`GetMessage` 相当（`ring.get(out, timeout)`）を `Atomics.wait` で待つ。ページは DOM のイベントをリングに書き `Atomics.notify` する。ページは `Atomics.wait` を使えない（読み）ので、返信は `Atomics.waitAsync` または `postMessage`。

### 3.1 往復遅延（実測。µs、p50 / p95 / p99 / max、各 10,000 回。`ma-coi` と `ma-rtt-r1..r3` の 4 回）

| 経路 | 結果（4 回） |
|---|---|
| ページ → リング → Worker（`Atomics.wait`）→ 返信は SAB の語（ページは `Atomics.waitAsync`） | p50 **75〜90**（75、75、75、90）、p95 105〜130、p99 140〜230、max 600〜3,295 |
| 同上、返信は Worker の `postMessage` | p50 75〜105、p95 110〜155、p99 150〜255、max 815〜1,435 |
| 同上、4 ms 間隔で 1 回ずつ（Worker が眠っている状態から起こす。n = 2,500） | p50 90〜115、p95 165〜235、p99 215〜315、max 515〜1,935 |
| Worker ↔ Worker（ページを介さない `Atomics.wait` / `notify` の往復） | p50 60〜75、p95 85〜105、p99 110〜175、max 335〜520 |
| 比較: wasm 無し、`postMessage` だけの非ブロッキング Worker（M-B の輸送路） | p50 90〜105、p95 125〜160、p99 175〜285、max 1,965〜5,190 |
| 比較: 同じ非ブロッキング Worker を `crossOriginIsolated = false` のページで（刻み 100 µs） | 100 / 200 / 200 / 3,900（刻みが粗い） |

- 往復はすべて**約 0.1 ms**で、1 フレーム（16.7 ms）の 100 分の 1 以下。M-A と M-B の差（p50 で 0〜30 µs）は run 間の振れと同程度で、**有意な差とは言えない**。外れ値（max）は 0.3〜5 ms で、どの方式にもある。
- リング投入の `post()` は 1 回 0.036 µs（満杯で捨てる分を含む。2 秒で 6.17 M 件を受理し Worker が全部消費、約 3.1 M 件/s）。ハンドラのコストはほぼリング投入の分だけ（§9 の実測で 35 µs / 回は `getCoalescedEvents` などの DOM 側を含む）。

### 3.2 `Atomics.wait` のタイムアウトの精度（`SetTimer` / `WM_TIMER` 相当。実測、遅れ = 実際の経過 − 要求値、ms）

| 要求 | 回数 | p50 / p95 / p99 / max |
|---|---:|---|
| 0.5 | 400 | 0.09 / 0.12 / 0.16 / 0.27 |
| 1 | 400 | 0.10 / 0.15 / 0.18 / 0.21 |
| 2 | 400 | 0.13 / 0.17 / 0.21 / 0.27 |
| 5 | 400 | 0.15 / 0.21 / 0.23 / 0.25 |
| 10 | 300 | 0.17 / 0.24 / 0.31 / 0.52 |
| 16 | 188 | 0.18 / 0.23 / 0.30 / 0.42 |
| 20 | 150 | 0.17 / 0.23 / 0.31 / 0.88 |
| 33 | 91 | 0.18 / 0.25 / 0.30 / 0.30 |
| 50 | 60 | 0.19 / 0.30 / 0.53 / 0.53 |
| 100 | 30 | 0.17 / 0.20 / 0.23 / 0.23 |

- 絶対期限（`next += period`）の `WM_TIMER` 相当: 10 ms 周期 500 回、遅れ p50 0.20 / p99 0.46 / max 0.59 ms。1 ms 周期 2,000 回、p50 0.12 / p99 0.20 / max 0.53 ms。リングのイベントを毎秒 250 件読みながらでも p99 0.26 ms（max 5.19 ms が 1 回）。**1 イベントの処理に 2 ms かかる**と、tick の遅れは p95 1.86 ms（= 処理 1 つ分。ハンドラは先取りされない）、イベントの待ち時間は p50 4.0 ms。
- 参考（M-B の `SetTimer` の作り方）: Worker 内の連鎖した `setTimeout(T)` は T < 5 ms でクランプされる（T = 0 → 4.15 ms、1 → 4.2、2 → 4.15 ms かかる。`settimeout-worker.json`）。T ≥ 5 ms は遅れ p50 0.16〜0.26 ms で `Atomics.wait` と同程度。`Atomics.waitAsync(…, T)` は 0.5〜5 ms でクランプされず、遅れ p50 0.10〜0.19 ms（SAB が要る = 隔離が要る）。`setTimeout` は小数の ms を切り捨てるので、絶対期限で組むと最大 1 ms 早く起きる（tick の p50 = −0.28 ms）。Windows の `SetTimer` の下限は 10 ms 級（引用なし。読み）なので、`WM_TIMER` だけなら問題にならないが、`timeSetEvent` や `Sleep(1)` の 1 ms 級は M-B ではクランプに当たる。

### 3.3 Worker が忙しいとき（実測。`busy`: 3 秒、250 件/s のマウス移動 + 約 8 件の押下を 16 ms ごとの 4 件の束で。1 イベントに `work` ms の CPU を使う）

| 1 イベントの処理 | リング | マウス移動の扱い | 遅延 p50 / p99 / max (ms) | 消費 / 送信 | 捨てた数 | 押下の取りこぼし |
|---|---|---|---|---|---|---|
| 0 ms | 64 / 4096 | そのまま / 合体 | 0.1 / 0.2〜0.3 / 0.3〜0.5 | 746 / 746（合体では 192 / 746） | 0 | 0 / 6 |
| 5 ms（処理能力 200/s < 供給 250/s） | 64 | そのまま | 306 / 322 / 322 | 597 / 746 | **88** | **4 / 6** |
| 同上 | 4096 | そのまま | 305 / 597 / 606 | 598 / 746 | 0（積み残し 148） | 0 / 6 |
| 同上 | 64 or 4096 | **合体** | 0.1 / 5.1 / 5.1 | 191 / 746 | 0 | 0 / 6 |
| 20 ms | 64 | そのまま | 1,211 / 1,283 / 1,284 | 151 / 746 | **532** | **6 / 6** |
| 同上 | 4096 | そのまま | 1,210 / 2,384 / 2,404 | 151 / 746 | 0（積み残し 595） | 0 / 6 |
| 同上 | 64 or 4096 | **合体** | 8.4 / 16 / 20 | 151 / 746 | 0 | 0 / 6 |
| 100 ms | 64 | そのまま | 1,453 / 2,888 / 2,888 | 31 / 746 | **652** | **6 / 6** |
| 同上 | 4096 | 合体 | 9.4 / 95 / 95 | 31 / 746 | 0 | 0 / 6 |

- **「満杯なら新しい方を捨てる」リングでは、押下も捨てられる**（リング 64 で 4/6〜6/6 が消えた）。入力の取りこぼしを避けるには、(a) 移動を別スロットに合体して通常のリングには入れない（本試作の方式。押下・キー・ホイールは通常のリングに残り、0 件の取りこぼし）、(b) 大きなリング、(c) 押下・キー用の別リングが要る（読み）。リングを大きくしても**遅延は消えない**（積み残しが処理を待つだけ）。
- ゲストが忙しい間、ページ側で起きていること（実測）: ページのスレッドは止まらず、イベントはリングに溜まる（または合体）。Windows の「メッセージキューが溢れる」「入力が `GetMessage` までたまる」と同じ。

### 3.4 ページ（メインスレッド）が忙しいとき（実測。ページが 100 ms を 250 ms ごとに、または 400 ms を 600 ms ごとに占有）

| ページの占有 | Worker 側の 10 ms tick の遅れ p50 / p99 / max | ページが受け取る Worker の tick の間隔 p50 / p99 / max (ms) |
|---|---|---|
| 無し | 0.16 / 0.24 / 0.77 | 10.0 / 10.4 / 11.4 |
| 100 ms | 0.16 / 0.25 / 0.26 | 10.0 / 109.7 / 110.0 |
| 400 ms | 0.16 / 0.24 / 0.26 | 9.9 / 408.5 / 409.0 |

- **Worker のタイマーとブロッキングの待ちはページの負荷に影響されない**。影響されるのはページ側の DOM イベントの配送と、Worker → ページの `postMessage` の受け取り（ページが空くまで遅れる）。`Atomics.wait` を使う側（エンジン）が止まらないので、**音声を Worker + AudioWorklet に置けばページが忙しくても途切れない**（§8 でも確認）。

### 3.5 M-A の構造的な制約（読み）

- Worker が `Atomics.wait` の中にいる間、その Worker は**自分のイベントループに戻らない**: `fetch` / `createImageBitmap` / `decodeAudioData` / `WebCodecs` / `Promise` / `setTimeout` / `onmessage` の継続が一切走らない（仕様）。非同期のブラウザ API を使うには、**別の Worker に頼み、結果を SAB で受ける**（`Atomics.wait` で待つ）形が要る。同期的に使えるもの: `Atomics`、`FileSystemSyncAccessHandle`（OPFS）、Worker 内の同期 XHR（読み。使うなら別途測定）。
- **描画**: §7 の実測どおり、戻らない Worker では `OffscreenCanvas`（`transferControlToOffscreen`）の自動表示が効かない。`ImageBitmap` を `postMessage` する経路は効く。
- **スレッドを増やす拡張**: ゲストの協調スレッド（A1）は「待ち」でスタックを切り替える仕組みが要り、`Atomics.wait` は全部を止める。M-A のまま別スレッドを実行するには、本物のスレッド（Worker + 共有 `WebAssembly.Memory`。`sync-thread-survey.md` §3.3 の D）しかない。D は全オブジェクトの `-pthread` 再ビルドと、`R`、`X`、`FL` などの thread-local 化、決定論的な仮想時計の放棄を伴う（引用）。
- `crossOriginIsolated`（COOP + COEP）が**必須**（実測: ヘッダ無しでは `SharedArrayBuffer` が `undefined`）。

## 4. M-B: スタックを退避して非同期に待つ（Asyncify / JSPI）

`mb.c` の `loop_echo(depth)` は、`depth` 段の C 関数の底で `host_get_message(&msg)`（JS の Promise を待つ）を呼び、PING なら `host_reply(seq)` する。ページは `postMessage` でイベントを送る。

### 4.1 往復遅延（実測。µs、p50 / p95 / p99 / max、10,000 回。`-O2`。`mb-coi`、`mb-rtt-r1`、`mb-rtt-r2`）

| 方式 | スタック 0 段 | スタック 40 段 |
|---|---|---|
| Asyncify | 95〜110 / 145〜165 / 195〜350 / 2,120〜3,925 | 105 / 150〜160 / 220〜245 / 715〜1,810 |
| JSPI | 95〜105 / 135〜155 / 195〜260 / 1,310〜1,505 | 95〜110 / 130〜160 / 180〜230 / 865〜2,010 |
| `-O0`（`mb-coi` の 1 回） | Asyncify 115 / 175 / 260 / 3,875、JSPI 110 / 165 / 260 / 1,485 | Asyncify 115 / 170 / 250 / 3,330、JSPI 105 / 160 / 270 / 1,220 |
| `crossOriginIsolated = false`（`mb-plain`、刻み 100 µs） | 100 / 200 / 200〜300 / 1,000〜1,900 | 同程度 |

- **M-B は COOP/COEP 無しで動く**（`mb-plain`: `SharedArrayBuffer` は `undefined`）。往復は、wasm 無しの `postMessage` の往復（§3.1）と同程度で、**中断・再開のコストは往復の中に埋もれる**（§4.2 の数値は 0.1〜2.4 µs）。スタックの深さの差も見えない。

### 4.2 中断と再開のコスト（実測。`mb-bench-coi`。C の呼び出し 1 回あたりの ns。マイクロタスク 1 回待ちの値から、同じ深さで待たない import を呼んだ値を引いた正味）

| 方式 | 深さ 0 | 10 | 40 | 160 |
|---|---:|---:|---:|---:|
| Asyncify `-O2` | 299 | 265 | 465 | **1,228** |
| Asyncify `-O0` | 468 | 539 | 902 | **2,412** |
| JSPI `-O2` | 155 | 115 | 161 | **195** |
| JSPI `-O0` | 126 | 122 | 128 | （測定の揺れで −18。実質 約 0） |

- **JSPI の中断・再開は深さにほぼ依存しない**（約 0.1〜0.2 µs）。Asyncify は深さに比例して増える（1 段あたり 約 6〜12 ns）。各段に 24 個のスカラのローカルを置いても（`wide`）、`-O2` の深さ 160 で 1,228 → 1,528 ns 程度の増加。マクロタスク（`MessageChannel`）1 回待ちは Asyncify も JSPI も 約 3〜5 µs で、輸送路が支配する。
- **待たない通常の経路の遅さ**（実測。`kind=0`、深さ 160 の再帰を 1 回呼ぶ ns）: 基準（同期 import）`-O2` 458、Asyncify 775（+69 %）、JSPI 504（+10 %）。`-O0` は 629、1,028（+63 %）、775（+23 %）。24 ローカル版は Asyncify が `-O0` で +18 %、`-O2` で +76 %、JSPI は −4〜+8 %。**Asyncify は、中断しない呼び出しにも常に計装のコストを払う**。
- 再帰の深さの上限（実測、`mb-depth`。ローカルが小さい関数の再帰、`RangeError` が出る手前）: 普通の呼び出し 7,041（`-O0`）/ 7,956（`-O2`）、Asyncify 6,369 / 7,956、**JSPI の `promising` 経由 13,388 / 15,097（約 1.9 倍深い）**。ゲストの呼び出しの深さの実コードでの上限は未測定（`sync-thread-survey.md` §3.2 も同じ指摘）。

### 4.3 JSPI の使い方の制約（読み。公開された API の性質）

- `promising` で包んだ export から `Suspending` で包んだ import までの間に **JS のフレームがあってはならない**（あれば `SuspendError`）。lift の wasm は C → C の呼び出しだけで JS が間に入らないので条件は満たす（`fle_call_guest` も C の中）。runtime の JS import（`Suspending`）は `GetMessageW`、`WaitMessage`、`MsgWaitForMultipleObjects`、`Sleep` などブロッキングする API の実装に限る。
- 中断中のワーカーは自分のイベントループを回す。JS のイベントハンドラが wasm の export を呼ぶと、**中断中のゲストの途中の状態（`R[]` などのグローバル）の上で動いてしまう**ので、ハンドラはキューへの書き込みだけに限る（`sync-thread-survey.md` §3.2 の probe が `R[3]` の上書きを実測している。引用）。
- `crossOriginIsolated` は要らない（実測）。

### 4.4 Asyncify のコード膨張（合成コード。実測 + 外挿は読み）

`synth.mjs` で作った関数 500 / 2,000 / 4,000 / 8,000 個（合計 3.4 万〜54 万命令）を `emcc` でビルドし、同じ .o から基準・JSPI・Asyncify・Asyncify（`-sASYNCIFY_IGNORE_INDIRECT`）を作った。

| 関数数（命令数） | `-O` | 基準 wasm (B) | JSPI | Asyncify | Asyncify（間接 call を無視） | Asyncify のリンク時間 / ピーク RSS（基準は 0.5〜1.1 s / 90〜190 MB） |
|---|---|---:|---:|---:|---:|---|
| 500（3.4 万） | O0 | 1,647,707 | ×1.0000 | 3,056,344（**×1.855**） | ×1.828 | 0.72 s / 278 MB |
| 2,000（13.6 万） | O0 | 6,548,272 | ×1.0000 | 12,130,218（**×1.852**） | ×1.823 | 2.70 s / 509 MB |
| 4,000（17.1 万。実 FLEngine の形の設定） | O0 | 8,289,797 | ×1.0000 | 15,376,489（**×1.855**） | ×1.816 | 1.67 s / 489 MB |
| 8,000（54.5 万） | O0 | 26,147,746 | ×1.0000 | 48,475,479（**×1.854**） | ×1.825 | 4.68 s / 1,253 MB |
| 500 | O2 | 757,110 | ×1.0000 | ×1.246 | ×1.216 | 1.06 s / 127 MB |
| 2,000 | O2 | 3,116,195 | ×1.0000 | ×1.232 | ×1.203 | 2.13 s / 275 MB |
| 4,000（実 FLEngine の形の設定） | O2 | 4,336,325 | ×1.0000 | ×1.271 | ×1.231 | 2.30 s / 349 MB |
| 8,000 | O2 | 12,239,254 | ×1.0000 | ×1.238 | ×1.208 | 6.13 s / 868 MB |

- **`-O0` で ×1.85 は、関数数・形の設定を変えても変わらない**。`-O2` は ×1.21〜1.29。間接 call を無視する指定は 2〜3 ポイントしか下げない（lift 済みのコードは間接 call が 15 % あるが、ほぼ全関数が `fle_dispatch` を通りうるため、無視しても影響する範囲が広い）。
- **外挿（読み。実際には作っていない）**: 現在の wasm は `-O0` でビルドされている（引用）。389,439,333 B × 1.855 = 約 **722 MB**（V8 のモジュール上限 1 GiB の 67 %。現在は 36 %）。リンク時のピーク RSS は、基準のサイズに対し 38〜48 MB/MB で伸びた（8,000 関数で 1,253 MB。MB は 10^6）ので、**約 15〜19 GB**（この機械は 22 GB）。Asyncify のパスの時間は 0.15〜0.25 s/MB で、**約 1〜2 分**。線形の外挿で、実コードの巨大な関数（locals 最大 41,126）での振る舞い、`-O0` の実 wasm に Asyncify を掛けたときの locals 上限（50,000）との関係は**未確認**。`-O2` の実 wasm（215 MB の C を `-O2` でコンパイルする）は試していない。
- **実行時のコスト**（実測、`synthrun`。3,000,000 回の関数入口。ns/入口、最小 / 中央値。中断はしない）: `-O0` 基準 592〜617 / 685〜725、JSPI 553〜597 / 660〜712、Asyncify **732〜808 / 850〜967（+19〜37 %）**、Asyncify（間接無視）736〜771 / 815〜934。`-O2` 基準 87〜97 / 98〜106、JSPI 84〜100 / 98〜112、Asyncify **171〜177 / 182〜238（+85〜100 %）**。3 回の測定、`-O0` で ±10 % ほどの揺れ。**JSPI は基準と区別できない**。

### 4.5 合成コードの限界

実コードの形（Delphi のコード、巨大な関数、`rep movs` などの runtime 呼び出し、`-O0` の実 wasm での locals の数）とは違う。呼び出しの割合（8 % / 10.6 %）、間接 call の割合（15 %）、1 関数の命令数（68 / 43）は実 FLEngine に合わせたが、**膨張率が実コードで同じになる保証は無い**。run-time 版は、呼び出しの深さを 60 に制限し、全体の関数入口数に上限を付けた（`fle_budget`）別ビルドで、サイズの表と同じソースではない（生成器の設定が同じで、上限の検査だけが違う）。

## 5. M-C: 協調的に分割（メッセージループの 1 反復ごとに JS へ戻る）— 読み

### 5.1 M-C そのもの（`setjmp` / `longjmp` で JS に戻り、次のイベントで再開）の制約

| # | 制約 | 根拠（読み） |
|---|---|---|
| 1 | `longjmp` は**生きている祖先のフレームへ戻る**ものにすぎない。ブロッキング API から JS へ戻ると、それより内側の C フレーム（`Idle` → `HandleMessage` → `Run` → `CreateFruityInstance` ほか）は**捨てられ**、「`WaitMessage` の次の命令から続ける」ことは**できない** | 決定 0100: `fle_landing` は呼び出し元の C フレームを生かしたまま `_setjmp` を置き、`fle_dispatch(L)` がそこへ戻る。途中のフレームは捨てる。「捨てたフレームの副作用は戻らない」と決定にも書かれている |
| 2 | 再開は「**ゲストのスタック（`R[4]` と戻りアドレス）から C の呼び出し鎖を作り直す**」ことになる。それには、すべての call の戻りアドレスを関数入口（landing）にする必要がある。戻りアドレスごとの landing は決定 0100 では「setjmp の呼び出しの戻り」1 か所（FLEngine で 0x1376840）のみ | 266,954 個の call（FLEngine の C を数えた値）すべてに入口を作るのは、§5.2 の変換と同じ規模になる |
| 3 | **入れ子のメッセージループ**: `DispatchMessageW` は 10 関数にある。モーダルなフォーム、ドラッグ、メニュー、`MessageBox`、`DoDragDrop` などは `WndProc` の呼び出しの**途中**でさらに `HandleMessage` を回す。外側の `Run` の反復境界で戻る案は、**一番外側のループにしか効かない** | `msgloop_counts.txt`: `DispatchMessageW` 14 地点 / 10 関数、`PeekMessageW` 21 / 18 関数、`MessageBox` 10、`TrackPopupMenu` 2、`DoDragDrop` 2 |
| 4 | runtime の手書き C（`fle_call_guest` を使う `fle_win.c`、`fle_gdi.c`、`fle_callback.c` など）は、ゲストの関数を呼ぶ**間**に C のローカルを持つ。そこを抜けて戻るには、runtime の関数ごとに再開の書き方を書き直す必要がある | `fle_call_guest` に触れる runtime のファイルは 5 つ（コメントを含め 14 か所。今後増える） |
| 5 | `FLE_LANDING_NOJUMP`（wasm の既定）では landing に当たると**名前付きで停止**。emcc の `setjmp` / `longjmp` は `-sSUPPORT_LONGJMP` と例外処理の設定が要り、このツリーのスクリプトには無い。wasm での動作は**未確認**（決定 0100 §5） | 決定 0100 |
| 6 | ネイティブ側（clang の harness）にも同じ API が要る。検証用の native と wasm で道が分かれる | 決定 0100 の検討 E |

→ **M-C 単独では、外側の 1 本のループ（`Application.Run`）の反復だけを JS に返す形にしかならない**。モーダル、メニュー、`MessageBox`、`Sleep` や `WaitFor` などの内側のブロッキングは別の仕組みが要り、結局、スタックの中断（M-A のブロック、M-B）か、次の M-C' に戻る。

### 5.2 M-C'（生成器が「巻き戻し」を作る）— 合成コードで測定（実測 + 設計は読み）

lift 済みの関数は、C のローカルが call をまたいで生きていない（ゲストの状態は全部グローバル）ので、Asyncify のような「ローカルの退避」が要らない。その性質を使った変換を合成コードに施した（`synth.mjs` の `cps` モード、約 40 行）:

- 各 call の後に `if (fle_mode == 1) { 記録を積む; return; }`（巻き戻しで C スタックを `return` で畳む。間接 call はその VA も記録）。
- 各関数の先頭に `if (fle_mode == 2) switch (記録を取り出す) { case k: goto K_k; }`（再開。call の**直前**のラベルへ飛ぶ。`R[4]` の push は再実行しない）。
- ブロッキングの 1 地点（合成コードでは `fle_wait()` が 64 回に 1 回 `fle_mode = 1` にする）。ドライバは `entry()` が戻ったら `fle_mode == 1` を見て待ち、`fle_mode = 2` にして `entry()` を呼び直す。

| 項目 | 結果（実測。合成コード） |
|---|---|
| **正しさ** | 基準（待ちを数えるだけ）と変換後で、最終のゲスト状態のチェックサムが**一致**（関数 200、2,000、8,000 個。2,000 個で 87 回の中断・再開、8,000 個で 20 回）。マイクロタスク待ちとマクロタスク待ちのどちらでも一致 |
| wasm の大きさ | `-O0`: 関数 2,000 個 7,477,033 → 8,977,886 B（**+20.1 %**）、8,000 個 29,902,971 → 35,984,802 B（**+20.3 %**）。`-O2` 2,000 個: 3,756,577 → 4,798,354 B（**+27.7 %**）。リンク時間・メモリは基準とほぼ同じ（wasm-opt のパスが無い。8,000 個で 0.77 s / 244 MB） |
| 実行時（3,000,000 回の関数入口、4 回の測定） | `-O0`: 最小値で +10〜11 %（3 回）と +24 %（1 回）、中央値で +6〜17 %。`-O2`: 最小値で +6〜20 %。関数 8,000 個の `-O0` は +15 % |
| 比較（同じ合成コード） | Asyncify: `-O0` で 大きさ **+85 %**、実行時 +19〜37 %、リンクは基準の 6 倍の時間と 6 倍のメモリ。`-O2` で 大きさ +21〜29 %、実行時 +85〜100 % |

- M-C' は**今の実測では、`-O0`（現在の wasm のビルド）で Asyncify の約 1/4 の膨張（+20 % 対 +85 %）、約 1/3 の実行時コスト**で、リンク時の巨大なメモリも要らない。**`-O2` では大きさは同程度**（+28 % 対 +23〜29 %）で、実行時だけが軽い（+6〜20 % 対 +85〜100 %）。この数値は次の条件つき:
  1. 合成コード（実コードとの差は §4.5 と同じ）。
  2. **runtime の手書き C のフレーム（`fle_call_guest` の呼び出し元など）には自動では効かない**（§5.1 の 4）。今は 5 ファイルに 14 か所（コメントを含む言及）だが、メッセージ配送（`DispatchMessageW`、`SendMessageW`、`SetTimer` のコールバックなど）が入ると増える。
  3. 生成器の変更が要る（`fe-lift`。call 266,954 か所に検査、全関数に switch。生成 C の大きさと `-O0` の locals 数への影響は**未測定**）。ゲストの `longjmp`（決定 0100 の landing）との整合は**未確認**。
  4. `fle_dispatch` や `fle_imp_*` の経由で JS が間に入る呼び出し（コールバックの再入）の扱いは**未検討**。
- この項は依頼された M-C の範囲外の追加で、**設計の候補の 1 つとしての数値**（採用の提案ではない）。

## 6. 比較表（3 方式 + M-C'）

| 観点 | M-A: Worker + `Atomics.wait` | M-B: JSPI | M-B: Asyncify | M-C（単独） | M-C'（生成器の巻き戻し） |
|---|---|---|---|---|---|
| 往復遅延（実測） | p50 75〜90 µs、p99 140〜230 µs | p50 95〜105、p99 180〜260 | p50 95〜110、p99 195〜350 | （試作せず） | （試作せず。輸送路は M-B と同じ） |
| ブロッキング API の実現 | 素直（`Atomics.wait`）。タイマーも `Atomics.wait` のタイムアウト（遅れ p50 0.1〜0.2 ms） | 素直（`Suspending` の import が Promise を返す） | 素直（同左） | 外側のループだけ（§5.1） | 全ブロッキング API（ただし runtime の手書き C は別途） |
| 実装量の見積もり（読み） | リング + ページ側のイベント変換 + 非同期 API 用の補助 Worker（`fetch`、デコード等）+ `OffscreenCanvas` の代わりに `ImageBitmap` の送信。生成器・ビルドは変更なし | `-sJSPI` + ブロッキング API の import を `Suspending` にする + ページのイベントを runtime のキューに書く。生成器の変更なし | JSPI と同じ + ビルド系の変更（巨大な wasm-opt）。ビルドは実現性が問題（下） | 生成器 + runtime の `setjmp` 設定。範囲が狭い | 生成器の変更が大きい + runtime の手書き C の再開コード |
| 実 wasm（389 MB）への適用 | **可**（変換なし）。ただし戻らないループなので描画・非同期 API の経路が要る | **可**（変換なし。サイズ増 0、実行時の影響は見えない）。実 wasm では**未実測**（スタックの深さの上限、JSPI のスタックの総メモリ） | **疑問**: 外挿で wasm 約 722 MB（上限の 67 %）、リンクで 15〜19 GB、`-O0` の実行時 +19〜37 %。実際には作っていない | 単独では成立しない | 外挿していない（`-O0` で +20 % なら約 467 MB と読めるが、実コードで測っていない）。生成器の変更後の再 lift が要る |
| COOP / COEP | **必須**（`SharedArrayBuffer`） | 不要（実測）。音声を SAB のリングにするなら必要（§8） | 不要（同左） | 不要 | 不要（同左） |
| 配信の制約 | COEP `require-corp` のページは、読み込む全ての外部資源に CORP / CORS が要る。COOP `same-origin` は他オリジンのポップアップとの連携（`window.opener`）を切る（読み） | なし（ブラウザの JSPI の対応が条件） | なし | なし | なし |
| デバッグ性（読み） | 普通のコールスタックが見える。ブロック中は DevTools で停止できる | 中断中のスタックの見え方は**未確認** | 巻き戻し・再開でスタックが壊れて見える。計装後のコードを読むのは難しい | 難（戻る場所が散る） | 生成された C に検査が見える。普通の C のスタック |
| Safari / Firefox | **未確認**（`SharedArrayBuffer` の隔離は一般的な機能だが、本調査では未測定） | **未確認**（`sync-thread-survey.md` は Chrome/Edge 137、Firefox 153、Safari 27 と書くが、二次情報で未検証） | 未確認（Wasm 標準機能のみで動く = 古いブラウザでも動くはず） | 未確認 | 未確認 |
| スレッド（ゲストの `CreateThread`）を増やしたときの拡張 | ブロックが全体を止めるので、**協調スレッド（A1）は作れない**。増やすなら本物のスレッド（D: pthreads）+ 全体の再ビルド | **同じ機構で A1（協調スケジューラ）に拡張できる**（`sync-thread-survey.md` の推奨と一致。ゲストのグローバル状態のスレッド別化は別途必要） | A2。計装のサイズが問題 | 不可 | A1 相当の拡張は可能（読み。記録を積むスタックをスレッドごとに分ける） |
| 描画との相性（実測） | 戻らない Worker では `transferControlToOffscreen` が表示されない → `ImageBitmap` の `postMessage`（約 50 fps で表示を確認） | 中断の度にイベントループへ戻る → `raf` / `setTimeout` 駆動と同じ（58〜60 fps）。JSPI の中断中の確認は**読み**（`raf` の Worker 駆動の実測から） | 同左 | 同左 | 同左 |
| 入力の取りこぼし | リングが満杯になると捨てる → 合体 + 押下用の余裕が要る（実測 §3.3） | runtime のキューの設計次第（同じ問題） | 同左 | 同左 | 同左 |

### 推奨（書くが決めない）

実測と既存の調査（`sync-thread-survey.md`）を合わせて読むと、**M-B の JSPI が最も変更が少なく、COOP/COEP を要求せず、スレッドの案 A1 と同じ機構で伸ばせる**ので、第一候補と読める。ただし (1) 実 wasm（`-O0`、389 MB、巨大な関数）での JSPI のスタックの上限・メモリ・ブロッキング API を通した振る舞いは**未測定**、(2) Firefox / Safari での JSPI の対応は**未確認**。M-A は変換が要らず遅延も最小だが、**ゲストのスレッドを協調で増やせず**、COOP/COEP を要求し、描画と非同期 API の経路が増える。Asyncify は、実 wasm（`-O0`）への外挿で現実的でない。M-C' は Asyncify より軽いという合成コードの数値があるが、生成器の大きな変更で、runtime の手書き C に別の手当てが要る。JSPI が使えないブラウザへの保険として M-A（または Asyncify の縮小版）を持つかどうかは未決（§12）。

## 7. 描画（実測。`draw-coi.json`。CPU ラスタライズ、実 GPU は未測定）

Worker のメモリ（`WebAssembly.Memory` 16 MiB、共有ではない）上の 1920×1080×4 B = 8,294,400 B の画素バッファを、エンジンの DIB セクション（決定 0108 / 0111）の代わりに使った。毎フレーム 16 KB の領域だけ書き換える（エンジンの描画の代役。測定に含めない）。

### 7.1 ブラウザ側の部品の速度

| 項目 | 結果（ms / 回の p50 / p95 / p99 / max、n = 200） |
|---|---|
| `new ImageData(view, 1920, 1080)`（線形メモリの上のビュー） | 0.00 / 0.00 / 0.01 / 0.05（**コピー無し**: ビューへの書き込みが `ImageData.data` に見える。実測） |
| ImageData を **SharedArrayBuffer のビュー**から作る | **`TypeError: The provided Uint8ClampedArray value must not be shared.`**（実測）。共有メモリの wasm（pthreads の D 案）では画素を非共有の配列にコピーしてから ImageData にする: 8.3 MB のコピーは 0.15 / 0.40 / 4.16 ms |
| Worker の `OffscreenCanvas`（非表示）に全面 `putImageData` | **0.22 / 0.44 / 0.53 / 0.59（ループ速度 4,232 fps）**。直後に 1×1 の `getImageData` で強制的に完了させても 0.22 / 0.44 / 0.54 / 0.68 |
| dirty rect を指定した `putImageData` | 960×540: 0.05 / 0.07 / 0.08（17,953 fps）、480×270: 0.01 / 0.02 / 0.03（62,794 fps）、256×256: 約 0.00 / 0.01 / 0.05、64×64 と 16×16: 0.00（285,714 / 338,983 fps） |
| 全面 `putImageData` + `transferToImageBitmap()`（ページへ送る形） | 3.32 / 4.28 / 5.28 / 9.81 ms |
| `createImageBitmap(ImageData)` | 4.82 / 5.65 / 6.97 / 6.97 ms |
| ページ（メインスレッド）の canvas に全面 `putImageData` | 0.22 / 0.27 / 0.41 / 0.46 ms（4,441 fps）。dirty rect 480×270 で 0.01 ms |
| ページの canvas に `createImageBitmap` + `bitmaprenderer` | 6.17 / 8.44 / 9.22 ms |
| ページの `requestAnimationFrame` + 全面 `putImageData` | 間隔 p50 16.66 ms（60 Hz）、max 717 ms（最初の 1 回） |
| Worker からページへ 8.3 MB を渡す（Worker 側で `slice()` でコピー → 転送 → ページで `putImageData`） | Worker のコピー 2.76 / 3.20 / 3.89 ms、送信から受信 0.14 / 5.17 / 27.79 ms、ページの `putImageData` 0.56 / 0.75 / 0.88 ms、受信と描画で 141.7 fps |

- 数値は CPU 側の呼び出しの時間で、**画面への反映（GPU へのアップロード、合成、vsync）は含まない**。表示されたフレーム数は §7.2。`putImageData` が 8.3 MB を 0.22 ms（約 38 GB/s）で処理するのは、この機械のキャッシュの効きかたと、呼び出しの記録だけで実ラスタライズが後になる実装による可能性がある（`getImageData` の強制後も同じ値だったので、少なくともその時点で完了している）。
- 共有メモリの wasm では全面コピー（0.15 ms 前後）が毎フレーム要る。非共有メモリなら**コピー無し**（`ImageData` は線形メモリを直接指す）。lift した wasm は現在、共有メモリではない（引用）。

### 7.2 実際に表示されるか — Worker のループの形による違い（実測。3.5 秒の描画中、400 ms ごとの 16×2 画素の PNG スクリーンショットで 64×64 の目印の R 値（= フレーム番号の下位 8 ビット）を読み、別の回で `Page.startScreencast` のフレーム数を数えた）

| Worker のループ | Worker が描いたフレーム | 表示されたフレーム（スクリーンショットの R 値の推移 / screencast fps） |
|---|---:|---|
| `requestAnimationFrame`（Worker 内。`transferControlToOffscreen` の canvas） | 212 | 25, 52, 79, 106, 133, 160, 187, 211（**進む**）/ 59.8 |
| `setTimeout(16)` | 208 | 23, 49, 75, … 203（進む）/ 58.4 |
| **`Atomics.wait(16 ms)` で戻らないループ**（M-A の形。`transferControlToOffscreen` の canvas に `putImageData`） | 208 | **255 の白のまま（表示されない）**/ 0.8（最初の 1〜3 フレームだけ） |
| 戻らないループ + `putImageData` → `transferToImageBitmap()` → `postMessage`（ページが `bitmaprenderer` に出す） | 176 | 21, 41, 64, … 173（進む）/ 49.9（Worker のペースが 16 ms 待ち + 約 4 ms） |
| ページ自身が `raf` で `putImageData`（比較） | 211 | 26, 53, … 210（進む）/ 60.0 |

- **戻らない Worker の `OffscreenCanvas` は表示されない**（`ctx.commit()` は Chrome 156 に存在しない = `typeof` が `"undefined"`。実測）。原因（フレームの提出がイベントループの 1 ターンの終わりに結びついている）は読みで、Chrome の内部実装は確認していない。`ImageBitmap` の `postMessage` はブロック中の Worker からでも届く（実測）。
- **dirty rect の効果**: CPU 側の `putImageData` の時間は面積にほぼ比例して減る（全面 0.22 ms、1/4 面積 0.05、1/16 面積 0.01、256×256 以下は測定の下限）。ただし、表示のコスト（GPU へのアップロード）は未測定なので、**dirty rect が実際の表示でどれだけ効くかは未確認**。
- `OffscreenCanvas`（Worker）とメインスレッドの canvas の比較: CPU 側の `putImageData` は同程度（0.22 ms）。Worker で描けば、ページのスレッドの負荷（§3.4）に影響されない（読み。表示側の合成は別）。

## 8. 音声（実測。`audio-coi.json`。実時間の `AudioContext`。出力デバイス無し）

- `new AudioContext({sampleRate: 48000 | 44100, latencyHint: 'interactive'})` は `running`（`--autoplay-policy=no-user-gesture-required`）、要求したサンプルレートがそのまま `ctx.sampleRate` になった。`baseLatency` = 0.0107 s（= 512 フレーム）。`audioWorklet.addModule` と `AudioWorkletNode` が動いた。`currentTime` の進みは実時間の **0.999〜1.006 倍**、処理されたブロック数は `currentTime × sampleRate / 128` と一致（1,132/1,132 など）。
- 構成: Worker（`Atomics.wait` のブロッキングのループ）が 440 Hz の正弦波を SAB のリング（インターリーブの float32 ステレオ）に書く。Worklet は 128 フレームごとに読み、足りなければ無音にしてアンダーランを数え、生産側を `Atomics.notify` で起こす。アンダーランは**起動直後（リングが半分埋まるまで）を除いた定常状態**で数えた。

| サンプルレート | リング（フレーム）/ 1 回の生成 | 生産側の停止（500 ms ごと） | 定常のアンダーラン（ブロック / 3 秒） | リングの最小〜最大の充填 |
|---|---|---|---|---|
| 48 kHz（5 秒） | 2048 / 512 | 無し | **0** | 1664〜2048 |
| 44.1 kHz（5 秒） | 2048 / 512 | 無し | **0** | 1280〜2048 |
| 48 kHz | 256 / 128 | 無し〜5 ms | **約 500（約 45 %）**（停止 50 ms で 552） | 0〜256 |
| 48 kHz | 1024 / 512 | 無し / 5 / 20 ms | **0** | 640〜1024 / 640〜1024 / 128〜1024 |
| 48 kHz | 1024 / 512 | 50 ms | 60（= 1 停止あたり 約 10 ブロック） | 0〜1024 |
| 48 kHz | 4096 / 512 | 無し / 5 / 20 / 50 ms | **0** | 3712〜4096 / 3712〜4096 / 3200〜4096 / 1664〜4096 |
| 48 kHz | 1024 / 256 | ページが 100 ms を 250 ms ごとに占有 | **0** | 640〜1024 |
| 48 kHz | 1024 / 256、1 回の生成に 3 ms（256 フレーム = 5.3 ms の 56 %） | 無し | **0** | 640〜1024 |
| 48 kHz | 1024 / 256、1 回の生成に 6 ms（113 %） | 無し | 168（追い付けない） | 0〜512 |

- **128 フレームのブロックを供給し続けられる**（リング 1024 フレーム = 21 ms 以上、生産側の停止が `リング − 約 1 回分` 未満のとき）。**ページのスレッドの占有は影響しない**（M-A の Worker + Worklet 構成）。リング 256 フレームでの大量のアンダーランは、`baseLatency` が 512 フレームであること（= 描画が 512 フレームの単位で呼ばれ、128 フレームのブロックが 4 つ続けて要求される可能性）と整合するが、**この原因は読み（確認していない）**。ブロックの要求間隔の測定はしていない。
- **できなかったこと / 未確認**: 実デバイスへの出力、実機の遅延（`outputLatency`）、デバイスのサンプルレートとの差（要求した 44.1 kHz がそのまま通ったが、デバイスの再サンプリングは見えない）、AudioWorklet 内で wasm の DSP を直接動かす構成、メインスレッドの音声 API を使わない（`MessagePort` だけ、SAB 無し）場合のジッタ、タブがバックグラウンドのときの挙動、Bluetooth 等の遅延の大きい出力。

### 8.1 Win32 の音声 API とブラウザ API の写像（読み。エンジンの import と呼び出し地点は `host-engine-boundary.md` §4.3、`unsupported-inventory-pdata.md`、`sync-thread-survey.md` §2.3 の引用）

| エンジンが使うもの（引用） | ブラウザの対応先（読み） | 注意 |
|---|---|---|
| DirectSound: `DirectSoundCreate` 1 地点、`DirectSoundEnumerateA` 1。バッファの操作（`CreateSoundBuffer`、`Lock` / `Unlock`、`GetCurrentPosition`、`Play`）は COM の vtable 呼び出しで、どのメソッドが使われるかは**未確認**。`TMixerThread_DS` は `WaitForMultipleObjects(2, …, INFINITE)` で待つスレッド | SAB のリング + AudioWorklet。`GetCurrentPosition` = Worklet の読み出し位置（`Atomics.load`）、通知イベント = Worklet からの `Atomics.notify` を、runtime のイベント表（決定 0050）の `SetEvent` に結ぶ | 本物のスレッドが要るか、協調スレッド（A1）で足りるかは**未確認**（エンジンの音声スレッドが CPU を使い続けるか） |
| ASIO: エンジンの文字列に `ASIO`（ASCII 61、UTF-16 23）、`CoCreateInstance` 14 地点。ASIO ドライバ（`ILWASAPI2ASIO_x64.dll` など）は COM のサーバー。`TMixerThread_ASIO` は `WaitForSingleObject(INFINITE)` | ASIO はドライバが `bufferSwitch` を音声スレッドから呼ぶ形。ブラウザには「ドライバ」が無く、`AudioContext` + AudioWorklet が唯一の出口。ASIO の COM インターフェースを runtime が JS 側で**代替の仮想ドライバ**として実装し、`bufferSwitch` に相当する呼び出しをエンジンの音声スレッドから行う形 | `CoCreateInstance` の応答（`ASIO` のクラス ID の列挙、`getBufferSize` / `getSampleRate` / `getLatencies` / `canSampleRate`）をどう返すかは設計項目 |
| WASAPI | エンジンの文字列に `WASAPI` は 0 件（`host-engine-boundary.md` §7）。別プロセスの COM コンポーネント `ILWASAPI2ASIO` が WASAPI を ASIO に包む（読み） | ブラウザ版では不要 |
| waveOut | import 一覧に見当たらない（`unsupported-inventory-pdata.md` の winmm の未対応 13 名は `midiIn*` / `midiOut*` のみ） | — |
| MIDI: `midiInOpen` / `Start` / `Stop` / `AddBuffer` / `PrepareHeader` ほか 13 名 | Web MIDI（`navigator.requestMIDIAccess`: Chrome 156 に有り。許可の表示が要る。headless では**未実測**）。`MIDIInput.onmidimessage` のタイムスタンプ（`DOMHighResTimeStamp`）をコールバックの時刻へ | `midiIn` のコールバックは別スレッドで呼ばれる（読み）。SysEx は許可が別に要る |
| msacm32（`acmStream*` ほか 14 名） | ブラウザに ACM は無い。`WebCodecs` の `AudioDecoder`、またはエンジン同梱のコーデック DLL（`oggio`、`wavpackdll`、`LAMEenc`）の lift | 同期的な呼び出し → 非同期の `AudioDecoder` は中断が要る |
| タイマー（`timeGetTime` ほか） | `performance.now()`（隔離で 5 µs）+ 仮想時計（決定 0017） | — |

## 9. 入力（実測 + 読み。`input-coi.json`。CDP による合成入力、headless）

### 9.1 `PointerEvent` → リング → Worker の経路（実測、マウス移動 2,000 件 + クリック 106 + ホイール 50 + キー 10 組）

| 項目 | 結果 |
|---|---|
| 配送 | `pointermove` 2,000/2,000、`pointerdown` / `pointerup` 107/107、`wheel` 50、`keydown` / `keyup` 10 組、`contextmenu` 3、ドロップ 1（取りこぼし無し） |
| ハンドラ（リング投入を含む）のコスト | 35 / 65 / 95 / 285 µs（p50 / p95 / p99 / max） |
| ハンドラの時刻 − `event.timeStamp` | p50 **15.4 ms**、p95 15.7 ms、p99 32.2 ms（= **フレーム単位の配送**。CDP が 1 件ずつ応答を待つので、各イベントが次のフレームの先頭で配送された。実マウス・`pointerrawupdate`（API は有る）は**未測定**） |
| ハンドラ → リング → Worker（`Atomics.wait` から起きて受け取るまで） | 0.075 / 0.125 / 0.185 / 1.7 ms（p50 / p95 / p99 / max）。同じ時計の取り方は §3.3 の「遅延 0.1 ms」と整合 |
| `getCoalescedEvents()` | 常に 1 件（CDP の合成のため。実マウスは**未確認**） |

### 9.2 ブラウザの機能と制約の表

| Windows の機能（エンジンの使用箇所は引用） | ブラウザ | 実測 | 制約 |
|---|---|---|---|
| `WM_MOUSEMOVE` / `LBUTTONDOWN` ほか | `PointerEvent`（`pointermove` / `pointerdown` / `pointerup`、`pointerType`、`pressure`） | 上表。`pointerType: "mouse"`、`button` / `buttons` が届く | 移動はフレームごとに配送される（実測の p50 15.4 ms）。`pointerrawupdate` は存在（`'onpointerrawupdate' in window` = true）するが遅延は**未測定** |
| `SetCapture`（7 地点）/ `ReleaseCapture`（26）/ `GetCapture`（54） | `Element.setPointerCapture` | API の存在のみ（`"function"`） | 呼び出しは同期、キャプチャの解除はイベントで通知される |
| `GetCursorPos`（72 地点）/ `SetCursorPos`（10）/ `ClipCursor` / `ShowCursor`（2） | 位置は**イベントからの最後の値**しか無い（取得 API は無い）。`SetCursorPos`（ワープ）は**できない**。相対移動はポインタロック | ポインタロックは `requestPointerLock()` が `pointerlockchange`、`locked = true`。`unadjustedMovement: true` は `NotSupportedError`（この環境） | ロックはユーザー操作（クリック）の直後だけ。`Esc` で解除される。ブラウザのウィンドウの外の座標は取れない |
| `SetCursor`（31 地点）/ カーソル形状 | CSS `cursor`（`crosshair`、`ew-resize`、`not-allowed`、`none`、`url(data:…) x y, fallback`） | 設定して `getComputedStyle` で読み戻せた。**画面に出たカーソルの形は headless では見えない** | カスタム画像の大きさの上限（Chrome は 128×128 と読むが**未確認**）。ウィンドウの外では変わらない |
| `WM_MOUSEWHEEL` | `wheel`（`deltaMode` 0 = ピクセル、`deltaY` 100 / ノッチ（CDP の値）） | 50 件 | 刻み、方向、加速は OS・ブラウザ・デバイス依存（`WHEEL_DELTA` = 120 への換算の規則は**未確認**）。`passive: false` で `preventDefault` |
| `WM_KEYDOWN` / `KEYUP` | `KeyboardEvent`（`code` = 物理キー、`key` = 論理、`keyCode` = 非推奨だが Windows の VK を返す、`location`、`repeat`） | `code` / `key` / `keyCode` が届く（キーと `keyCode` は CDP の入力に依存する値で独立の検証ではない）。`vk-map.js` に `code` → VK の表（133 個）を置いた | **表は公開されている VK の一覧から書いたもので、実機・実キーボードで照合していない**。VK_OEM_* は配列依存（`key` が必要）。`Meta`、`ContextMenu`、メディアキーは OS が先に処理する。`F5`、`Ctrl+W`、`Ctrl+T` などはブラウザが先に処理する（実測: 合成した `F5` でページが再読み込みされた）。`navigator.keyboard.lock()` は存在し解決したが、効果は未確認 |
| `WM_CHAR` / `ToUnicode`（1 地点） | `beforeinput` / `input`、隠した `textarea`、`EditContext` | `EditContext` は存在 | 文字入力は `key` ではなく入力イベントで取る |
| IME（`WM_IME_*`、imm32 の遅延 import 7） | `compositionstart` / `compositionupdate` / `compositionend`（`textarea`）、`EditContext` | CDP の `Input.imeSetComposition` + `insertText` で、`compositionstart`、`compositionupdate` ×3、`compositionend` と `beforeinput` / `input`（`insertCompositionText`）が届いた | 候補ウィンドウの位置は `textarea` / `EditContext` の位置に依存し、キャンバス描画の UI では位置を合わせる工夫が要る（未確認） |
| `GetKeyState`（12）/ `GetAsyncKeyState`（7）/ `GetKeyboardState`（6） | **取得 API は無い**。イベントから状態表を維持する（フォーカスが外れたら全解除） | — | `CapsLock` などは `getModifierState` |
| クリップボード（`OpenClipboard` 2、`GetClipboardData` 7） | 非同期の `navigator.clipboard`（`readText` / `writeText`） | 許可を付与した状態で書いて読み戻せた（`"FLclip"`） | 通常は許可・ユーザー操作が要る。形式はテキスト・HTML・PNG が基本。**同期 API をエンジンが期待する**ので、読み出しは中断（M-B）か、事前にキャッシュしておく必要がある |
| ファイルのドラッグ＆ドロップ（`DragAcceptFiles` / `DoDragDrop` 2） | HTML5 DnD（`drop` の `dataTransfer.files`） | ファイル 1 つのドロップで `name` / `size` / `type` が届き、`arrayBuffer()` で内容が読めた（12 B で 1.4 ms） | **ファイルシステムのパスは得られない**（名前だけ）。Windows のパス（`C:\…`）は存在しないので、ファイルは MEMFS / OPFS にコピーする。ブラウザからのドラッグの**開始**（`DoDragDrop` の発信側）は HTML5 のドラッグ開始 + `DataTransfer`。ファイルを外へ出す方法は**未確認** |
| フルスクリーン | `requestFullscreen()` | クリックの直後に成功（`fullscreenElement` = 対象） | ユーザー操作が要る。`Esc` で抜ける |
| リサイズ（`WM_SIZE`）/ DPI | `ResizeObserver`、`devicePixelRatio`、`visualViewport` | API の存在 | キャンバスの実ピクセルの大きさは `devicePixelRatio` に合わせる |
| メニュー（`TrackPopupMenu` 2）/ モーダル（`MessageBox` 10） | 仮想ウィンドウ上の自前の描画、または DOM | — | ブロッキング（呼び出しは選択が終わるまで戻らない）。M-B / M-A の待ちが要る |
| 複数ポインタ（タッチ・ペン） | `PointerEvent` の `pointerType`、`pressure` | 未測定 | — |

## 10. FL Studio のエンジンに適用するときの含意

1. **`CreateFruityInstance` は戻らない**。ブラウザでは、ゲストの `GetMessageW` / `WaitMessage` / `MsgWaitForMultipleObjects(Ex)` / `Sleep` / `WaitForSingleObject` / `MessageBox` / `TrackPopupMenu` / `DoDragDrop` などのブロッキング API が「待つ」場所になる。M-A はその場で `Atomics.wait`、M-B はスタックを中断して Promise を待つ。どちらでも、`TApplication.Run` が `PeekMessageW` を空振りして `Idle` → `WaitMessage` で寝る構造（引用）はそのまま使える。**ホスト層（runtime）の仕事**は、(a) メッセージキュー（`PostMessageW` 139 地点、`SendMessageW` 216、`PeekMessageW` 21、`GetMessageW` 1、`DispatchMessageW` 14）の実装、(b) ページの DOM イベントをその型（`WM_*`）へ変換してキューに積む層、(c) 仮想時計とタイマー（`SetTimer` 13、`KillTimer` 4）、(d) 待ちの API の実装。決定 0071 の仮想ウィンドウ表の上に載る。
2. **配信の要件**: M-A は COOP `same-origin` + COEP `require-corp` を**必須**とし、他オリジンの資源（フォント、CDN など）にも CORP / CORS が要る。M-B は不要。ただし**どの方式でも、音声を SharedArrayBuffer のリングで Worker から Worklet へ渡すなら COOP/COEP が要る**。SAB 無し（`MessagePort` 経由のメッセージ）での音声のジッタは**未測定**。
3. **描画**: エンジンの GDI（決定 0108 / 0111）は DIB セクションの画素を線形メモリに書くので、**線形メモリがそのまま `ImageData` になる**（非共有メモリならコピー無し。共有メモリなら 1 フレーム 0.15 ms 前後のコピー）。表示は、M-B では Worker の `OffscreenCanvas` を `raf` 駆動で、M-A では `ImageBitmap` の `postMessage` で。dirty rect を渡す API（`BitBlt` / `StretchDIBits` の範囲）が runtime にあれば `putImageData` の 7 引数版で面積比例の節約になる（CPU 側のみ確認）。
4. **音声**: 音声スレッドの要否が**未確認**。Worker（エンジン）→ SAB リング → AudioWorklet の構成は 48 kHz / 44.1 kHz の 128 フレームのブロックを供給できた（1024 フレーム以上のリング）。ASIO / DirectSound の COM の応答を runtime が仮想ドライバとして返す設計と、`TMixerThread_*` の待ち（`INFINITE`）の扱いが論点。
5. **入力**: `GetKeyState` / `GetAsyncKeyState` / `GetCursorPos` は取得 API が無く、イベントの状態表に置き換える。`SetCursorPos` はできない（相対移動はポインタロック）。クリップボードとファイルのドロップは非同期。`F5` や `Ctrl+W` などブラウザが先に処理するキーがある。
6. **スレッド**: InitTable の範囲で `CreateThread` に届くのは 2 unit（引用）。M-B（JSPI）なら協調スレッド A1 と同じ機構、M-A なら本物のスレッド（D）が必要。いずれも**音声スレッド・レンダースレッドが実際に必要になる時点**で決まる。
7. **ビルド**: 今の `-O0` の 389 MB の wasm に Asyncify を掛けるのは、外挿で現実的でない（§4.4）。JSPI は変換なしで掛かる。

## 11. 限界

- **FL Studio のコードは使っていない**。wasm 389 MB も使っていない。実 wasm のスタックの深さ、`fle_call_guest` を通した JS の呼び出し、実際のメッセージの量、巨大な関数での Asyncify・JSPI の振る舞いは**未測定**。合成コードの形は §4.5。
- Chrome 156 の headless だけ。**Firefox / Safari は未測定**。実デバイス（GPU、音声出力、マウス、キーボード、タッチ、IME、複数モニター、DPI の違い）は未測定。入力はすべて CDP の合成。
- 数値は 1 台の機械（16 コアの空いた状態に近いが、他の作業が動いていた）の値。run 間の振れは §1。バックグラウンドのタブのタイマー・Worker の絞り込み（throttling）、省電力、メモリ圧、`Page Visibility` は未測定。
- 描画は CPU ラスタライズ（SwiftShader）で、**表示まで含めたフレーム時間ではない**（呼び出しの時間と、表示された数）。画質（拡大縮小、色空間、`devicePixelRatio` ≠ 1）は未測定。
- 音声は出力デバイス無し。アンダーランは Worklet の側で数えたもので、実際に聞こえるかは別。
- M-C は読みのみ（依頼どおり）。M-C' は合成コードの範囲で正しさ（チェックサムの一致）まで確認したが、実コード・runtime の手書き C・ゲストの `longjmp`（決定 0100）との整合は未確認。
- JSPI の中断中の DevTools の見え方、中断中の Worker のメモリ（スタックごとの割り当て）、多数の中断（数千）は、`sync-thread-survey.md` の probe（node。引用）以外は未測定。

## 12. 未決の点

1. **JSPI（M-B）を第一候補にするか**。実 wasm（`-O0`、389 MB）での JSPI の試作（`CreateFruityInstance` までのどこかのブロッキング API を `Suspending` にして中断・再開できるか、スタックの深さの上限）を次の測定にするか。Firefox / Safari での JSPI の対応を調べるか（`sync-thread-survey.md` の二次情報では出荷済み、未検証）。
2. **COOP/COEP の配信を許容するか**（M-A、または音声の SAB リングを使う場合）。許容できないなら、音声を `MessagePort` で渡す方式の遅延・ジッタを測る必要がある。
3. **JSPI が使えないブラウザへの保険**を持つか（M-A の縮小版、または生成器による M-C'）。持たないなら対応ブラウザを JSPI のあるものに限る。
4. **M-C'（生成器による巻き戻し）を設計の候補に残すか**。合成コードでは Asyncify の約 1/4 の膨張（`-O0`）で正しく動いたが、生成器と runtime の手書き C の変更が大きい。
5. **音声スレッドの扱い**: `TMixerThread_DS/ASIO`、`TRenderThread` が実際に動く経路があるか（実行して確かめる必要がある）、協調スレッド（A1）で足りるか本物のスレッド（D）が要るか。
6. **入力の取りこぼしの方針**: 移動の合体 + 押下・キーの別リング（実測の結果はこの方向）を runtime のメッセージキューの設計に入れるか。
7. **描画の表示の経路**: M-A を採るなら `ImageBitmap` の送信、M-B なら `OffscreenCanvas` の `raf` 駆動。実 GPU でのフレーム時間と dirty rect の効果の測定を、どの段階で行うか。

## 13. 成果物

- `lift/harness/web/eventloop/`（74 ファイル、合計 約 272 KB）: README、試作（`ring.js`、`ma-*`、`mb/`、`mb-*`、`synthrun-*`、`cpsrun-*`、`st-*`、`draw-*`、`audio-*`、`input-*`、`vk-map.js`）、測定スクリプト（`lib.mjs`、`run-*.mjs`、`summarize.mjs`）、`results/`（生データ。JSON / TSV / ログ）。
- 大きな中間物（wasm、glue）は `lift/out-web2/` に出る（git 管理外。約 150 MB）。
