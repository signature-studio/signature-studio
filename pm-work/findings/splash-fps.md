# 起動の間のスプラッシュの提示が約 2 fps しか出ない(依頼者の指摘。2026-11-28)

- 範囲: `app/src/engine/`(提示の経路)、`lift/runtime/`(切り替え点・仮想時計・DwmFlush)、`tools/e2e/`・`tools/p2-frames/`(測り方)。読むだけで、何も変えていない。
- 起票: #1548(下書き 262。`pm-work/issue-drafts/wave1bw/262-startup-splash-fps.md`)。直す前の実測は、その Issue の作業が行う。
- この機械では製品をビルドしていない(`lift/out-app/` が無い)。したがって、**実測は 1 つも無い**。下の「推定」は、読みから組み立てた筋道で、E2E で確かめる必要がある。
- 区分: 「読み」はファイルと行で示す事実。「推定」は読みからの推論。「確認できない」は、読みでも記録でも決められないもの。
- 前提: 依頼者の指摘は「起動時/読み込み時のスプラッシュ画像がいつも 2fps くらいでしか動いてない」(起動の間。最初の idle の前)。前回の所見は `pm-work/findings/startup-splash-1520.md`(#1520。閉じた)。その受け入れは「最初の描画が canvas に出る」だけで、提示の頻度を測っていない。

## 1. 結論(要約)

1. 起動の間に画面が提示される経路は、エンジンの切り替え点から呼ぶ `deskPoll` だけ(読み)。その上限は **50 ms に 1 回 = 最大 20 fps**(`app/src/engine/worker.ts` 546・549〜552 行。読み)。受け入れの 30 fps は、この上限だけで満たせない。
2. 切り替え点は、**間接呼び出し 4096 回ごと**と**後退辺 1024 回ごと**で、時間では決まらない(`lift/runtime/fle_dispatch.h` 34・40〜41・226・234〜235 行、`lift/runtime/fle_thread.h` 93〜98 行。読み)。窓の手続きの中(呼び出し深さ 2 以上)では、切り替えも仮想時計の前進も無い(`lift/runtime/fle_thread.h` 87〜90 行、`fle_progress_slice` 878〜880 行。読み)。
3. **仮想時計は、起動の間(最初の idle の前)は壁時計に合わせられていない**(読み)。仮想時計が進むのは、間接呼び出しの進み(4096 回ごとに 0.25 ms)・時刻の読み(1 回 10 ticks)・全部のスレッドが待つ時の期限への跳びだけ(`lift/runtime/fle_sys.c` 48〜66 行、`fle_thread.c` 729・734 行、`fle_thread.h` 91〜92・94 行)。壁時計に合わせるのは、idle の処理(`app/src/engine/idle-driver.ts` 163・181〜192 行の `anchor` と `target`)だけで、anchor は**最初の idle で作られる**(163 行)。
4. スプラッシュの窓の描画(thread 2 の `TLightweightLayeredControl`)の周期は、FL が決める。その周期が仮想時計の上にあるか(`Sleep`・タイマー・`DwmFlush`)は、逆アセンブルが無いので**確認できない**。`DwmFlush` は仮想の 60 Hz の境界を待つ(`fle_thread.c` 1080〜1087 行)。
5. **推定の筋道(最有力)**: 起動の計算は、間接呼び出しが少ない長い区間(ループ)を含む。そこでは仮想時計がほとんど進まず、仮想時計で周期が決まるスプラッシュの更新が、壁時計の 2 回/秒程度になる。切り替え点は回っていても、変化が無いので提示されない。もう一つの推定は、窓の手続きの中の長い区間で、切り替えも時計も止まる(深さの規則)。どちらかは E2E の値で決める(第 5 節)。

## 2. 提示の経路(製品の読み)

### 2-1. エンジン側(`app/src/engine/worker.ts`)

- 切り替え点の入口: `inputPoll`(233 行)が最初に `deskPoll()` を呼ぶ(234 行)。`inputPoll` は、`lift/runtime/fle_thread.c` の `schedule_ex` が、切り替えのたびに `fle_input_ring_poll` を呼んで、そこから来る(`fle_thread.c` 694 行。`__EMSCRIPTEN__` の時。推定: 製品の wasm のビルドはこの条件)。
- `deskPoll`(549〜559 行): 2 つの条件で戻る。`t - deskPollAt < DESK_POLL_MS`(50 ms)と、`t - idleAt < DESK_POLL_MS`(最後の idle から 50 ms)。その後、`sendDeskReady()`・`mode === PRESENTER_NONE` の確認・`sceneOn()`・`deskTick(false)` を行う。
- `deskTick`(491〜535 行): `gate.busy` なら戻る(494 行。描画 Worker が前の画像を読み終えていない)。`changed`(502 行)の時だけ `gate.publish`(506 行)と `frame` の送信(514 行)。変化が無ければ何も送らない。
- 描画の問い(`onRender` の `want`、465〜473 行)は、エンジンの Worker のイベントの処理で届く。その処理は、エンジンが idle になるまで動かない(推定: `pm-work/findings/startup-splash-1520.md` の 2-3(40〜41 行)と、`worker.ts` 299〜300 行の注釈が、同じことを書いている。実行時の確認は無い)。
- 上限の計算: 50 ms に 1 回の `deskPoll` で、変化が 1 回あれば提示は 1 回。**起動の間の提示の上限は 20 回/秒**。実際の提示は、これに「変化の回数」を掛けた数より多くならない。

### 2-2. 描画 Worker 側(`app/src/engine/render-worker.ts`)

- 問いの仕組み: `frame`(67〜72 行)が毎フレーム `ask()`(55〜60 行)を呼ぶ。`ask` は `waiting` が立っていれば何もしない。`waiting` は `want` を送った時に立ち、`frame` の受け取り(`onFrame` 147〜151 行)で下りる。
- 起動の間は、`want` の答えが来ない(上)ので、`waiting` は立ったまま。提示は、エンジンの `deskPoll` が送る変化の `frame` で起きる(`onFrame` が `presenter.present()`、155〜163 行、`gate.ack` 167 行)。
- 統計の報告(`flushDesk` 174〜181 行): 100 ms に 1 回(提示が続く時)、無い時は 1 秒に 1 回。`presents` の数え方は `app/src/engine/desk.ts` 112 行と `desk-gl.ts` 271 行(`present` が画素を変えた時に +1)。
- 共有の制御語(`app/src/engine/desk-link.ts`): `GEN`・`ACK`・`MODE`。`busy` は `ACK !== GEN`(desk-link.ts の `busy`)。

### 2-3. 直接の上限

| 上限 | 値 | 出所 |
|---|---|---|
| `deskPoll` の間隔 | 50 ms(20 回/秒) | `worker.ts` 546 行 |
| 描画 Worker の問い | 1 フレームに 1 回、答えを待つ | `render-worker.ts` 55〜60 行 |
| 統計の報告 | 100 ms に 1 回(提示が続く時) | `render-worker.ts` 174〜181 行 |
| 切り替え点 | 4096 間接呼び出しごと、1024 後退辺ごと(呼び出し深さ 1 の時だけ) | `fle_dispatch.h` 34・40〜41、`fle_thread.h` 93・98 |

## 3. 切り替え点と仮想時計(`lift/runtime/`)

- 切り替え点は、`fle_progress_slice`(`fle_thread.c` 870〜888 行)と `fle_loop_slice`(893〜904 行)で、どちらも `schedule_ex(1)` に入る(`schedule_ex` は 687 行。入口で `fle_input_ring_poll` を呼ぶ 694 行)。
- 両方とも、**`preempt_armed`(最初の `CreateThread` の後。1010 行)と `nthr >= 2` の時だけ**、**呼び出し深さが 1 以下の時だけ**働く(`fle_progress_slice` 874〜878 行、`fle_loop_slice` 897〜899 行)。深さ 2 以上(窓の手続き・`DispatchMessageW` の中・コールバック)では、切り替えも仮想時計の前進も無い(`fle_thread.h` 87〜90 行の注釈。読み)。
- 仮想時計の前進: `fle_progress_slice` は 1 回ごとに `slice_ticks`(2500 ticks = 0.25 ms)を進める(`fle_thread.c` 885 行)。`fle_loop_slice` は進めない(`fle_dispatch.h` 40〜41 行の注釈: 「no virtual time passes for it」)。時刻の読みは 10 ticks(`fle_sys.h` 22 行)。`Sleep` の時間は、1 スレッドの時だけそのまま進める(`fle_sys.c` 99 行)。複数のスレッドでは、全部が待つ時に限って、最も近い期限まで跳ぶ(`fle_thread.c` 729・734 行)。
- したがって、**間接呼び出しの無い計算は、後退辺の切り替えが働いても、仮想時計を進めない**(読み)。例: 大きなバッファの処理・展開・検索のループ。
- 壁時計との対応(`app/src/engine/idle-driver.ts`): 「仮想時計を壁時計に合わせる」のは idle の処理(`onIdle` 158 行)。anchor は、最初の idle で作られる(163 行)。`target` の上限(maxLag 1000 ms)と下限(maxLead 100 ms)は、その後の規則。**起動の間は anchor が無いので、合わせる仕組みが無い**(読み)。

## 4. スプラッシュの窓(thread 2)

- 前回の記録(`startup-splash-1520.md` 2-2): 窓 lid 8(class `TLightweightLayeredControl`、334 x 476)は thread 2 が持つ。描画は `UpdateLayeredWindowIndirect`(`lift/runtime/fle_layered.c`。呼び出しは `f_71e570`、番地 0x71e570。注釈による)。
- 呼び出しの周期: **確認できない**。`lift/out-app/` が無いので、`f_71e570` の周りの待ち(`Sleep`・`SetTimer`・`DwmFlush`・`WaitForSingleObject`)を読めない。`lift/runtime/fle_layered.c` は、更新の回数を `updates` で数える(241 行)が、その報告 `fle_layered_report` は、この checkout の C からは呼ばれていない(grep の読み)。つまり、更新の回数は、どのログにも出ていない。
- `DwmFlush` の周期: 仮想の 60 Hz(`fle_thread.c` 1080〜1087 行。`FLE_DWM_HZ` の既定 60。1034〜1042 行)。`fle_imp_DwmFlush`(1089〜)は、複数のスレッドの時、次の境界まで待つ(仮想時計の上)。`app/src/engine/worker.ts` の `page の Hz` は、次の idle で適用される(`hzWanted`・`setHz` 172・205 行。`idle-driver.ts` の `pendingHz`)。つまり、起動の間の仮想の Hz は既定の 60。
- `Sleep` の周期: 仮想時計の上(`fle_thread.c` 843〜857 行の `begin_wait`)。
- 推定: FL がスプラッシュの更新に `DwmFlush` か `Sleep` を使うなら、更新の周期は**仮想時計**で決まる。壁での周期は「仮想の周期 ÷ 仮想/壁の比」。2 回/秒は、比が小さい(仮想時計が遅い)時に出る。

## 5. 推定の筋道と、決めるための値(E2E で読む)

| 仮説 | 予測される値 | 決める読み |
|---|---|---|
| H1: 仮想時計が起動の間に遅い(間接呼び出しの少ない計算。壁時計に合わせていない)。スプラッシュの周期が仮想時計の上 | 起動の間の 仮想 ÷ 壁 の比が、1 より十分小さい(推定の閾値は 0.1)。直すと上がる | 最初の idle の `idles[0].now`(仮想 ticks。`/10000` で ms。起動の時の値は `FLE_CLOCK_START_MS` = 3,600,000 ms。`lift/runtime/fle_sys.h` 21 行)から、起動の時刻(`idles[0].at` − `engine_start` の壁の ms)との比を出す。**基準を揃える**: `idles[0].at` は `performance.timeOrigin` 基準、`timeline` は `performance.now()` 基準(`main.ts` 67 行の注釈)。確認できない: 比が `timeline` の基準で正しいか(実行で確かめる) |
| H2: 切り替え点が少ない(深さ 2 以上の中で長く計算する) | 比は高い(0.5 以上)のに、提示は 2 回/秒。切り替え点の数が少ない | 切り替え点の数(`deskPoll` の呼び出し数)と、compose の回数を、起動の間に数える(新しい数。既存の統計に足す) |
| H3: `deskPoll` の 50 ms の上限 | 提示が 20 回/秒を超えない(上限ぴったりに近い)。変化は十分ある | 提示の数の分布(1 秒の窓の数)。20 に張り付くか |
| H4: スプラッシュ自身の周期が 2 回/秒(FL の仕様) | 比が 1 に近く、切り替え点は多く、変化は 2 回/秒。直しても変わらない | 変化の回数(compose が変化を返した数)と、窓の `updates`(数の差) |

- H1 が成り立つなら、直し方は「起動の間も、仮想時計を壁時計へ合わせる」(第 6 節)。H2 なら切り替え点を時間で保つ。H3 なら `DESK_POLL_MS` を下げる。H4 なら、FL の周期は変えられないので、報告に書く(確認できない点の決め直し)。
- 「2 回/秒」は依頼者の観察(体感)。E2E の 1 秒窓の値で、数字を決め直す。

## 6. 直し方の候補(測ってから選ぶ)

- (a) `DESK_POLL_MS` の 50 ms を、時間の上限に変える(例: 8 ms)。変化が無ければ composes しない(既存の `deskTick(false)` の規則のまま)。
- (b) 起動の間の時間の進み: 切り替え点(`schedule_ex` の入口。間接呼び出し・後退辺の両方)で、壁時計の経過を仮想時計に加える(期限を越えない。待ちの期限は、そのまま守る)。anchor を、起動の始め(エンジンが走り出した時)に作る。仮想時計を戻さない。idle の `target` の規則(maxLag・maxLead)を、同じ形で使う。
- (c) 切り替え点を、時間で保つ: 後退辺の切り替え(1024 回)に加え、一定の壁時計の経過ごとに切り替える。
- 深さ 2 以上の長い区間は、規則を変えない(スキップの理由は `fle_thread.c` 866〜869 行の注釈: 窓の手続きの中では、ランタイムの関数の途中の位置を、ゲストが別の場所で保持しているかもしれない)。ただし、起動の間にそれが長く続くなら、報告に書く。
- (b) は仮想時計の進め方の変更なので、P1(起動が遅くならない)・P3(音)・決定的な再現(E2E の再現性)への影響を、直す前後で比べる。

## 7. E2E で測る方法(既存の道具を使う)

- 提示の数: `window.__app.deskStats.presents`(`app/src/main.ts` 689〜691 行で、描画 Worker の `desk` 統計を受けて更新。`window.__app = app` は 220 行)。`tools/p2-frames/measure.mjs` は同じ値を 1 秒ごとに読み、窓の最小(`minWindowFps`)を出している(5・374 行の数え方)。
- 起動の時刻: `window.__app.timeline`(`first_pixel`・`engine_start`・`idle`。`tools/e2e/scenarios/page/startup-sequence.mjs` の (c)・(f) が使う)。
- 録画の型: `tools/e2e/scenarios/page/startup-sequence.mjs` の `RECORDER`(ページの rAF で値を溜めて、後で読む)は、提示の数にも使える。`tools/e2e/lib/browser.mjs` の rAF 計数は、ページの描画の数で、提示の数ではない(使わない)。
- 精度: 提示の統計は、描画 Worker が 100 ms に 1 回送る(`flushDesk`)。1 秒の窓の値は、±1 報告(約 100 ms 分)の誤差。閾値 30 fps・15 fps の判定には十分(読み)。
- 追加で読める値(既存): `window.__app.idles[0]`(最初の idle。`now`・`at`・`dwm`。`app/src/main.ts` 723 行で保存)。`dwm` は、その時点までの `DwmFlush` の数(`idle` の `dwm`。`protocol.ts` 85 行)。
- `covers` に使う feature は `tools/e2e/features/page.tsv` に行を足す必要がある(`scenario.mjs` 81〜83 行の形の検査。`tools/e2e/test/coverage.test.mjs` が重複を検査)。

## 8. 確認できない点

- 起動の間の、実際の提示の数(この機械では製品を動かしていない)。
- FL のスプラッシュの周期(`f_71e570` の周りの待ち。逆アセンブルが無い)。
- 起動の間の、切り替え点の壁時計の間隔(読みでは、間接呼び出しと後退辺の数でしか決まらない)。コードの注釈に「約 2,000 回/秒」とあるが(`worker.ts` 215 行。decision 0280)、出所(測定の記録)は、このチェックアウトに無い(`docs/` が無い)。
- 起動の時間(この機械の値は無い。`pm-work/findings/startup-splash-1520.md` 2-6 の記録では、runner の `boot.json` の `timelineIdle` が 8,844 ms)。
- 深さ 2 以上の中で、起動の計算がどれだけ続くか(`pst.deferred` は stop の時の報告だけ。`fle_thread.c` 607〜612 行、`fruity.c` 754 行)。
- `app.idles` が、起動の最初の値を保つか(上限で切られていないか)。読みでは確認できない。

## 9. 受け入れの式について

- 受け入れは「起動の間の提示の中央値が 30 fps 以上、最小の 1 秒が 15 fps 以上」。`DESK_POLL_MS = 50` のままでは、中央値 30 fps は不可能(上限 20 fps。第 2 節の 2-1)。したがって、上限の変更は必須(第 6 節 (a))。
- 1 秒の窓の判定は、`first_pixel`(起動の最初の提示)から最初の idle まで。その長さが 3 秒未満なら、測定できないので失敗にする(窓の数を報告に書く)。
