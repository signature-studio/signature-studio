---
draft: "109"
title: "起動直後(再生もマウス操作も無い状態)から音が途切れる現象を取り除き、再生しながらマウスを 1000 Hz で動かしても途切れないようにする"
type: Task
milestone: "M1 描画と音声の分離(P3)"
parent: "001"
blocked_by: ["040"]
labels: ["render", "audio"]
size: L
priority: Urgent
project: signature-studio/3
---

## 目的

依頼者の指摘(2026-11-23): 起動して約 10 秒後、一度も再生ボタンを押さず、マウスも動かしていない状態で、ヘッダーの `直近 60 秒の途切れ` が 343 ある。使い物にならないレベルで、パフォーマンスが最悪である。

P3 の目標は、Song Mode で音が途切れないこと。依頼者の是正(2026-11-23)の三つ目: 描画処理はまだ音声処理をブロックしている。特に、マウスを動かすだけで音が途切れる。

この Issue の目標は、段 1〜3 の途切れを 1 以下にすること(無音・マウス 1000 Hz、再生・マウス無し、再生・マウス 1000 Hz)。段 2・3 は、再生中(`playing` が true)であることを条件に入れる。再生中の engine の補充は、音の中身に依らず、リングバッファが空になれば途切れを数えるので、無音のプロジェクトでも、この判定になる。音の中身(`nonSilentQuanta`)は判定に入れず、JSON と報告には出す。音の出るプロジェクトでの測定は #{{004}} が行う。無音・無操作の段 0 の途切れは #{{112}} が直し、この Issue の判定には入れない。依頼者の実機の条件(物理画素の画面で、再生しながら、マウスを 1000 Hz 以上で動かし続ける)は、段 3 である。

## 背景

- 段 0 の定義(PM の直し。2026-11-23): 段 0 は「起動して 10 秒後から 60 秒」ではなく、ヘッダーの `直近 60 秒の途切れ` を、engine の idle の +10 秒と +70 秒に読む(起動の最中の途切れを含む)。#{{112}} が、起動の直後の guest の長い計算に切り替え点を入れて、段 0 を直す。この Issue は、段 1〜3(マウス 1000 Hz・再生)を直す。段 1〜3 の窓は、起動の山を避けて idle の +70 秒から +130 秒(下の「やること」0)。
- 依頼者の実機の観察(2026-11-23。画面は物理画素 3240×1863、DPR 1.7647、部屋は 1836×1056 CSS px)。
  - ヘッダー: `音: 動作中 44.1 kHz · 出力遅延 32 ms · 直近 60 秒の途切れ 343`。**起動して約 10 秒後の値。再生もマウス操作も無い**(前回の貼り付けでは 420)。
  - 343 回/60 秒 ≈ 5.7 回/秒(約 175 ms に 1 回)。周期性は未確認。
  - ページのログ(画面 2 回とも): `[warn] the virtual clock fell 84 ms further behind the wall clock than the cap allows (S-clk-maxlag); it stays behind (no virtual time is skipped)`(別の画面では 94 ms)。engine の仮想の時計が、壁の時計に対して、起動の時点で 80〜90 ms 遅れたまま戻らない。
  - 起動: `Download … at 2.0 s`、`Compile … at 2.0 s`、`Engine running … ran 2.5 s (main() began at 4.0 s)`、`First pixel at 6.9 s`、`Idle at 6.4 s`(2 回目。保存した複製からの起動)。
  - engine(マウスを動かした後の値): `Engine: computing (0.0 s) - input waits for its next idle`、`input: 837 sent, 835 processed`、`engine cycle 0.00 s (1058.1/s)`。
  - 前回の貼り付け(マウスを動かした状態): `input: 228 sent, 225 processed, 0 pending, 3 dropped/merged · engine cycle 0.00 s (1115.6/s)`、`last input: move at screen (3238,143) (pointermove)`。マウスの 1 イベントごとに engine が 1 周している(1115.6 周/s)。
  - 画面: `guest screen 3240x1863 device px in a room of 1836x1056 CSS px (device pixel ratio 1.7647, 6.04 Mpx)`。以前の論理の画面の約 3 倍の画素数。マウスの 1 動作ごとの guest の描画・合成の量が増える。
- 途切れの数え方(`app/src` の読み): ヘッダーの値は `underrunQuantaLast60s`(`app/src/audio/status.ts`、`app/src/audio/page-audio.ts` の `detail()`)。数えるのは、出力が PLAYING の間に、engine が次の 128 フレームを書き終えていなかった quantum(`app/src/audio/fle-out.worklet.js` の冒頭の注釈)。PLAYING は、engine が起動時に開始する出力の状態で、再生ボタンとは別(`app/src/audio/page-audio.ts` の `detail()` の注釈)。**したがって、再生を押していなくても、無音の出力の間に途切れが数えられる**(推定。段 0 の参考の値で見える。直しは #{{112}})。
- 以前の目標の誤り(PM の誤り): #{{002}} の測定では、**再生していない(`nonSilentQuanta` が 0)状態で、10 秒あたり約 200〜300 の途切れ**が出ていた。これを「圧迫の基準」として扱い、目標を #{{040}} では「UI の負荷ありで 10 秒あたり 20 以下」、この Issue の旧版では「60 秒で 5 以下」に置いた。どちらも緩すぎた。無音で、何も操作していない状態の途切れは、0 でなければならない。目標を 0 に直す。
- 以前の測定が現象を捉えられなかった理由(推定): 以前の道具(`tools/p3-audio-probe/measure.mjs` の `UI_STEP_MS`。50 ms の間隔で、半径 30 px の円を動かす)は、ポインタを 50 ms ごとに動かし、再生もしていなかった。依頼者の実機では、マウスを動かさない状態でも途切れるので、マウスの頻度だけが原因ではない。マウスを 1000 Hz で動かした時に途切れが増えるかは、段 1 と段 3 で実測する。
- 依頼者の言葉(2026-11-23): 依頼者のマウスは 8000 Hz で、互換性のため 1000 Hz まで落として使っている。したがって、測定のマウスは最低 1000 Hz で動かす。
- 画面: 本番の現在の画面(物理画素 3240×1863)で、そのまま測る。
- 過去の測定の結果は `pm-work/findings/c2-split-measure.md`(#{{002}})。underrun は 10 秒あたり 200〜300 で、前後・負荷の有無に差が無かった。

## 前回の試行で分かったこと

### 3 回目の試行(run 45317119636)

2 回目(run 45315924778)は時間切れで取り消され、保存された作業は、この 3 回目の作業の前に取り込まれている。3 回目は約 4 時間で、判定の段が落ちた(`failed`)。作業は保存されている(`work.diff` 約 38 KB。artifact `agent-1349`)。runner の値(nproc 4、loadavg(1 分)0.14〜1.15)。画面は guest 3240×1864 device px(Issue の 1863 より 1 px 多い。丸め)。報告の全文は、取り込まれる作業の差分の `pm-work/findings/measure-109-attempt2.md` に在る。

1. **途切れの条件は受理**: 段 1〜3 の 9 回すべてで、窓(idle の +70〜+130 秒)の途切れ `last60sUnderruns` は 0、`underrunDelta` も 0。入力の届きは毎秒 1194〜1197(条件は 1000 以上)。公称は `--hz` の 1.2 倍(前回の 1.0 倍では毎秒の最小が 992 で、条件を満たさなかった)。0.95 倍の緩和は取り消した。
2. **音の条件は差し戻し**: 段 2・3 の `nonSilentQuanta` は 6 回とも 0(音が出ていない)。既定のプロジェクトは音符が無く、無音。`--project`(音の出るプロジェクトを開く)は、この枝に開く経路が無い(Ctrl+O でファイルの窓が開かない)。開く経路は #{{004}} の作業(`tools/p3-audio-probe/measure.mjs` の `openProject`)が持つが、main にはまだ入っていない(#{{129}} の完了待ち)。demo と Blocks も、#{{122}}・#{{121}}・#{{129}} の完了前は素材が無く、無音。
3. **判定の直し**: 音の条件を `playing`(再生中であること)に直した。無音でも再生中の補充は途切れを数えるので、途切れの条件は音の有無に依らず効く。
4. 段 0(参考。判定に入れない): `last60sUnderrunsAtIdle10s`(idle の +10 秒)は 2129〜2249。窓の終わり(+130 秒)は 0。起動の山は #{{112}} の領域。
5. 参考(n=3。判定に入れない):

| 条件 | 窓の途切れ | 届いた毎秒の最小 / 平均 | 周回 (/s) |
|---|---|---|---|
| 再生・125 Hz(公称 150) | 0・0・0 | 148〜150 / 150 | 1080〜1090 |
| 再生・DPR 1・1000 Hz | 0・0・0 | 1197 / 1200 | 1706〜1710 |
| 再生・8000 Hz(公称 9600) | 0・0・0 | 5419〜6422 / 6642〜8685(判定 false) | 2993〜3485 |

   8000 Hz は届かなかった。CDP の応答が追いつかず、未応答が上限 256 に張り付いた。前回は道具が Node のヒープ不足で落ちたので、未応答の上限を送りの内側にも掛けて直した(今回は 3 回とも JSON が出た)。
6. 入力と周回は比例する。周回の増分は、1000 Hz で処理 1 件あたり約 0.8 周、8000 Hz で約 0.74〜0.85 周、125 Hz で約 0.24 周。
7. プロファイル(各 1 回。CDP の負荷付きで、途切れが 4〜8 出た。内訳にだけ使う): Worker の忙しい時間が、無入力の 1.49 秒/60 秒から、マウス 1000 Hz で 8.9 秒/60 秒に増えた(約 12% of one core)。増えた上位は `(program)`・`f_863b50`(名前の無い wasm 関数)・`postMessage`・`decode`・`setTimeout`・`fle_win_input_view`・`poll_waiters`。描画の転送 `fle_r_blit` は増えていない。推定: 待機中の engine は、移動 1 件ごとに Worker へ `wake` を送るので(`app/src/engine/client.ts` の 126〜129 行)、1 件ごとに 1 周する。
8. 変えた物: 測定の道具 `tools/p3-audio-probe/measure-live.mjs`(送り 1.2 倍、毎秒の下限の判定、未応答の上限、締め切り、JSON の欄)と、報告。製品(`app/`・`lift/`・`ci/`)は変えていない。

### 1 回目の試行(run 45253143948。以下は、その時点の記録)

run 45253143948(2026-11-23 19:52Z 起動)の作業。runner の値(nproc 4)。判定は差し戻し(音の条件で落ちた。途切れの条件は 9 回すべて満たした)。報告の原文は `pm-work/findings/measure-109-attempt1.md`。前回の道具の差分は `pm-work/findings/measure-live-109.patch`(新規ファイル `tools/p3-audio-probe/measure-live.mjs`)。製品(`app/`・`lift/`・`ci/`)は変えていない。

1. 段 1〜3(受け入れと同じ条件、n=3。窓は idle の +70〜+130 秒): 窓の途切れ `last60sAt130s` は 9 回とも 0。`underrunDelta` は 9 回とも 0。`inputRateOk` は段 1・3 の 6 回とも true。engine が受け取った秒あたりの件数の最小は 992〜997。送った件数(`inputSentPerSec`)は 643〜658、処理(`inputProcessedPerSec`)は 642〜657。周回(`engineCyclePerSec`)は段 1 が 1,579〜1,644、段 2 が 1,057〜1,058、段 3 が 1,585〜1,635。
2. 音: `nonSilentQuanta` は 9 回とも 0。Default のプロジェクトは、サンプルが読めず、FLEX・Emphasizer も読めないので、音が出ない。Blocks(#{{004}} の試行)も同じ。段 2・3 の音の条件は、音の出る入力が無い限り満たせない(#{{122}}・#{{121}} の範囲)。この Issue の報告では、途切れの数と音の有無を分けて書く。
3. 段 0(参考。判定に入れない): `last60sAt10s`(idle の +10 秒)は 9 回とも 2,475〜2,569。窓の終わり(+130 秒)は 9 回とも 0。起動の山は #{{112}} の領域。
4. 参考(n=1。判定に入れない): 再生・125 Hz: 届いた 124/s、処理 108.2/s、周回 1,099.3、窓の途切れ 0。再生・8000 Hz: 失敗(CDP の `Runtime.evaluate` が 20 秒返らず。送りがページを飽和させたと推定)。再生・DPR 1・1000 Hz: 窓の途切れ 0、届いた 994/s、処理 679.2/s、周回 1,679.6/s(`@+10` の途切れは 897。起動の画面が軽い)。
5. 入力と周回の比例(実測): 入力が無い段 2 の周回は 1,057〜1,058/s。処理された入力が約 650/s のとき、周回は 1,579〜1,644/s。差 520〜590/s を入力の処理量で割ると、入力 1 件あたり約 0.8〜0.9 周。**入力 1 件ごとに、おおよそ 1 周する**。時間の内訳(描画・協調スケジューラ・音声の書き込み)は未確認(道具に CPU プロファイルを入れていない)。
6. ポインタの送り方(前回の選択): CDP `Input.dispatchMouseEvent`(`mouseMoved`、ボタン無し)。`setTimeout` の一回ごとの送りに頼らず、起きるたびに、その時刻までに due の分をまとめて送る(バッチ)。応答は待たず、未応答は最大 256 件。経路は、部屋の中央を中心にした半径 400 CSS px の円を 1 秒に 1 周(1000 Hz で 1 件あたり約 2.5 CSS px、約 4.4 物理画素)。届いたかは、ページ側の `pointermove` の件数を 1 秒ごとに数えて確かめた(段 1・3 の 60 秒の全ての秒で 992 以上)。engine が処理したのは約 650/s で、残りは統合された(推定: フレームごとの統合)。
7. **判定の式を直した(逸脱。PM が見直す)**: 最初の式「60 秒のどの 1 秒も `--hz` 以上」では、1 回だけ 2 秒が 983〜985 に落ちて false になった。送り側が遅れた分は次の秒に届くので、60 秒の合計は揃っていた。式を「60 秒の合計 ≥ 0.99 × hz × 60 かつ、どの 1 秒も ≥ 0.95 × hz」に変えた。道具の `inputRateOk` は、この式で判定する。
8. 未確認: 8000 Hz で届く頻度(失敗したため)。`guestMouseMovePerSec` は null(engine 側に WM_MOUSEMOVE の計数が無い。足すには製品の計数が要る)。描画・協調・音声書き込みの時間の内訳。
9. 次の作業者へ: 段 2・3 の音の条件を満たす入力を先に揃える(#{{122}}・#{{121}})。それまでは、途切れの数と、音が出ないことを、分けて報告する。`pm-work/findings/measure-live-109.patch` は `git apply` で当てて使ってよい(当たらなければ書き直す)。
10. 入力の判定を 0.95 倍にゆるめる変更は、依頼者の指示(最低でも 1000 Hz)に反するので、取り消した。公称を 1.2 倍にして、各秒 1000 件以上を、判定にする。

## 読む物

- `pm-work/findings/c2-split-measure.md`(過去の測定の結果と手順)
- `pm-work/findings/measure-109-attempt1.md`(前回の報告の原文)
- `app/src/audio/status.ts`(ヘッダーの文言)、`app/src/audio/page-audio.ts`(`detail()`・`underruns()`・`engineLoss()`)、`app/src/audio/fle-out.worklet.js`(途切れの数え方。冒頭の注釈)、`app/src/audio/ring-layout.ts`
- `app/src/engine/idle-driver.ts`(65 行付近: `S-clk-maxlag`)、`app/src/engine/worker.ts`(137 行付近: 仮想の時計の警告)
- `tools/p3-audio-probe/measure.mjs`(#{{003}} の道具。`UI_STEP_MS` は 50 ms。前回の差分 `pm-work/findings/measure-live-109.patch` は、この道具の窓の読みを土台にしている)
- `tools/p2-frames/measure.mjs`(P2 の道具)
- `lift/runtime/fle_input.c`(入力の入口。`input: N sent, N processed, dropped/merged` の統合と、ポインタの送り方)
- `lift/runtime/fle_audio_transport.c`(54 行 `fle_js_publish`・108 行付近 `fle_js_wait`)、`lift/runtime/fle_thread.h`(1〜6 行: 協調のスレッド)、`lift/runtime/fle_dsound.c`(416〜471 行付近: 音声の混合と書き込み)、`lift/runtime/fle_audio_ring.c`(148〜176 行)
- `lift/runtime/fle_gdc.c`(GDI の描画)
- `lift/harness` の入力の待ち(engine の周の回し方。`grep -rn "cycle" lift/harness` で探す)
- `app/src/input/pointer.ts`、`app/src/engine/client.ts`・`worker.ts`・`render-worker.ts`
- `git log --grep 1334`(#{{040}} の結果)

## やること

0. 段階的な測定(この Issue の最初の作業)。全て、画面 CSS 1836×1056・DPR 1.7647、n=3。idle は、`window.__app.state` が `engine-idle` を最初に観測した時刻。段 1〜3 の窓は、idle の +70 秒から +130 秒(起動の山を避ける。山は #{{112}} の領域)。窓の終わり(+130 秒)に読む `underrunQuantaLast60s`(ヘッダーの `直近 60 秒の途切れ` と同じ定義の値。`window.__app.audio.detail().underrunQuantaLast60s`)は、ちょうど窓の 60 秒の途切れになる。idle の +10 秒に読んだ値も、段 0 の参考として JSON に出す。読んだ値は、そのまま出す(`app/src` の定義は変えない)。
   - 段 0: 無音・無操作(`--play 0 --hz 0 --mouse 0`)。再生も、マウスの動きも無い。AudioContext を開くための、測定の道具の起動の手順(ブラウザの中で 1 回、押して離す)は、窓の前に行ってよい。それ以外の入力は無い。
   - 段 1: 無音・マウス 1000 Hz(`--play 0 --hz 1000 --mouse 60`)。
   - 段 2: 再生しながら・マウス無し(`--play 1 --hz 0 --mouse 0`)。再生は窓の始まりに押す。
   - 段 3: 再生しながら・マウス 1000 Hz(`--play 1 --hz 1000 --mouse 60`)。再生は窓の始まりに押す。
   
   **段 0 の原因は、この Issue では特定しない**(#{{112}} が行う)。段 0 の計測は、参考として 4 で行う。
   
   計測の道具と計測は、一時的な変更として入れ、受け入れの前に取り除く(製品に残さない。切り替えも作らない)。

1. 測定の道具 `tools/p3-audio-probe/measure-live.mjs` を作る(前回の差分 `pm-work/findings/measure-live-109.patch` を `git apply` で当てて土台にしてよい。当たらなければ、`measure.mjs` を土台に書く)。引数は次のとおり。
   - bundle の絶対パス
   - `--dpr <値>`(既定 1.7647。部屋は 1836×1056 CSS px になるように窓の大きさを決める)
   - `--play <0|1>`(既定 1。1 なら窓の始まりに再生を始める(Space)。0 なら再生を押さない。出力は、engine が起動時に開始したままにする)
   - `--hz <値>`(判定の基準の頻度。既定 1000。送りは公称 1.2 倍。0 なら動かさない)
   - `--mouse <秒>`(ポインタを動かし続ける秒数。既定 60。0 なら動かさない)
   - `--project <.flp の絶対パス>`(省略時は下の既定)
   プロジェクトの既定: 所定のプロジェクト(#{{004}} が決める。`inputs/p3-project.tsv` が在ればその 1 件。音の出る物)を開く。無ければ、スタートの Default のプロジェクトで、Sampler のチャンネルのパターンを再生する。再生を始める方法は、道具が確かめる(`playing` が true になること)。
2. ポインタの送り方(この Issue の要)。
   - 経路: 画面の広い範囲を通る円(半径 400 CSS px。部屋の中央)を、1 秒に 1〜2 周する。1 イベントあたり約 1〜5 物理画素になる速さ。ボタンは押さない(mouseMoved だけ)。
   - 頻度: 公称の送る頻度は **`--hz` の 1.2 倍**(`--hz 1000` なら 1200 Hz。`--hz 8000` なら 9600 Hz)。`--mouse` の秒数の間、連続して送る。CDP の `Input.dispatchMouseEvent` を、応答を待たずに送る。送りの形は #{{110}}(P2 の道具)の `tools/p2-frames/measure.mjs` の `pointerLoop` と同じにする(その時刻までに due の分をまとめて送り、`sleep(2)` で回す)。`setTimeout` の一回ごとの送りに頼らない(タイマーの解像度で頻度が落ちる)。送り方の選択と理由は報告に書く。
   - 届いたかの確認: ページ側で、`pointermove` の件数(`getCoalescedEvents()` の件数で数える)を 1 秒ごとに数える。判定は `inputRateOk`: **窓の 60 秒の全ての 1 秒で、件数が `--hz` 以上**(`--hz 1000` なら 1000 件以上)。前回の道具の `TOTAL_RATE_SHARE`・`SECOND_RATE_SHARE` と `MAX_IN_FLIGHT` の待ちは取り除き、この判定と送り方に合わせて直す。届かない時は、送り方を直す(複数の接続、`Input.dispatchMouseEvent` の `timestamp`、ページ内で `PointerEvent` を合成する方法など。合成する時は、アプリの入力の処理が `isTrusted` を見ていないかを確かめる)。
3. 出力: stdout に 1 行の JSON: `{"error","state","standIn","dpr","play","hz","mouseSec","screenDevicePx","playing","nonSilentQuanta","last60sUnderruns","last60sUnderrunsAtIdle10s","underrunDelta","inputRateOk","pointerEventsPerSecMin","pointerEventsPerSecMean","inputSentPerSec","inputProcessedPerSec","engineCyclePerSec","guestMouseMovePerSec","quantaDelta","engineGapFrames","engineGapFramesDelta","presentsDelta"}`。
   - `last60sUnderruns`: 窓の終わり(idle の +130 秒)の `underrunQuantaLast60s`(ヘッダーの値と同じ。窓の 60 秒の途切れ)。`last60sUnderrunsAtIdle10s`: idle の +10 秒の同じ値(段 0 の参考。判定に入れない)。
   - `underrunDelta`: 窓の中の `underrunQuanta` の増分。
   - `play`・`hz`・`mouseSec`: 引数の値。`inputRateOk`: `--hz` が 0 なら null。それ以外は、窓の全ての 1 秒で `pointerEventsPerSecMin` が `--hz` 以上のとき true(他の式は使わない)。`pointerEventsPerSecMin`: 窓の各 1 秒の件数の最小。`pointerEventsPerSecMean`: 窓の全体の件数を秒数で割った平均。
   - `screenDevicePx`: guest の画面の物理画素(例 "3240x1863")。
   - `inputSentPerSec`・`inputProcessedPerSec`・`engineCyclePerSec`: `input: N sent`・`engine cycle` の増分を、窓の長さで割る。
   - `guestMouseMovePerSec`: guest が見た `WM_MOUSEMOVE` の数(engine の記録)。取れなければ null と書く。
4. 参考として、同じ道具で、段 0(`--play 0 --hz 0 --mouse 0`。無音・無操作。判定に入れない。#{{112}} の領域)、再生しながら(`--play 1`)の 125 Hz(`--hz 125`。公称 150 Hz)と 8000 Hz(`--hz 8000`。公称 9600 Hz。届かない時は、届いた頻度を書く)、DPR 1 で 1000 Hz(`--dpr 1 --play 1 --hz 1000`)を n=3 で測る。判定には入れない。表にして報告に書く。
5. 原因を特定する。段 1 と段 3 で、`engineCyclePerSec` が入力の数に比例して増えるかを、実測で確かめる(入力 1 件ごとに engine が 1 周しているか)。増えるなら、入力 1 件ごとに、guest の描画(GDI のラスタ・合成・提示)と協調スケジューラの切り替えが走っていないかを、時間の内訳で調べる。音声を作る側の時間を食う経路を、表にする。
6. 直し方の指針(この Issue では実装しない。段 1〜3 の途切れが 9 回とも 0 のため、直す根拠が無い。音の出るプロジェクトで途切れが出た時に、#{{004}} の測定を受けて扱う)。候補は、待機中の wake をまとめる、描画の通知を画面が変わった周だけにする、`readAppView` の文字列の取り出しを seq が変わった時だけにする。どれも入力の遅れや挙動に関わる。以下は、Windows と同じ振る舞いへの方向。Windows では、`WM_MOUSEMOVE` は低い優先度の合成メッセージで、キューに未処理の物が在る間は、新しい移動は 1 つに合成される。アプリが次のメッセージを取りに来た時に、最新の位置の `WM_MOUSEMOVE` が 1 つ出る(高頻度の履歴は `GetMouseMovePointsEx` で取れるが、通常の描画は最新の位置だけを使う)。これを再現する。
   - ページから engine へは、入力を時刻と位置の列として共有メモリに積む(1000〜8000 Hz でも、ページ側の処理は軽くする)。
   - engine 側では、guest が `GetMessage`・`PeekMessage` で取りに来た時に、最新の位置の `WM_MOUSEMOVE` を 1 つだけ作る。
   - 音声の書き込みの締め切りの前に、描画・入力の処理が時間の上限を超えて走らないようにする。協調スケジューラは、音を作る guest のスレッドを、描画・入力のスレッドより先に走らせる。
   - 製品に新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足さない。既定の動作として直す。オプトインを作らない。
   - 計測は一時的な変更として入れ、受け入れの前に取り除く(製品に残さない)。
7. (直した場合だけ。この Issue では行わない)直した後に、上の 0(段 1〜3)と 4(参考)を同じ道具で行い、表で比べる。

### 残りのやること(3 回目の試行の後)

3 回目の作業は、次の run で `work.diff` から取り込まれる。残りは次のとおり。
1. 載った作業ツリーを確かめる(`git status`・作業ツリーの `.prior-work.md`)。取り込みが衝突していれば、衝突の印を解消する。
2. 受け入れを通す(ビルドを含む。約 35 分。`accept` のコマンドは下の「受け入れ」のとおり)。
3. 通れば commit する。通らなければ、作業ツリーを commit し、判定の出力を報告に貼る。
4. 4 時間 30 分を過ぎたら、受け入れが通らなくても、作業ツリーを commit し、報告を書いて終える(ジョブの上限は 5 時間 50 分)。

## 受け入れ

```accept
npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false?0:1)' "$RUNNER_TEMP/boot.json" && mkdir -p .tmp-109 && B="$PWD/lift/out-app/bundle/raw/fl-Oz" && for i in 1 2 3; do node tools/p3-audio-probe/measure-live.mjs "$B" --dpr 1.7647 --play 0 --hz 1000 --mouse 60 > ".tmp-109/s1-$i.json" || exit 1; node tools/p3-audio-probe/measure-live.mjs "$B" --dpr 1.7647 --play 1 --hz 0 --mouse 0 > ".tmp-109/s2-$i.json" || exit 1; node tools/p3-audio-probe/measure-live.mjs "$B" --dpr 1.7647 --play 1 --hz 1000 --mouse 60 > ".tmp-109/s3-$i.json" || exit 1; done && node -e 'const fs=require("fs");const R=(s,i)=>JSON.parse(fs.readFileSync(".tmp-109/s"+s+"-"+i+".json","utf8").trim());const ok=(r)=>r.error===null&&r.state==="engine-idle"&&r.standIn===false&&Math.abs(r.dpr-1.7647)<0.0001;const all=(s,f)=>[1,2,3].every((i)=>f(R(s,i)));process.exit(all(1,(r)=>ok(r)&&r.play===0&&r.hz===1000&&r.mouseSec===60&&r.inputRateOk===true&&r.pointerEventsPerSecMin>=1000&&r.last60sUnderruns<=1)&&all(2,(r)=>ok(r)&&r.play===1&&r.hz===0&&r.playing===true&&r.last60sUnderruns<=1)&&all(3,(r)=>ok(r)&&r.play===1&&r.hz===1000&&r.mouseSec===60&&r.inputRateOk===true&&r.pointerEventsPerSecMin>=1000&&r.playing===true&&r.last60sUnderruns<=1)?0:1)' && rm -rf .tmp-109
```

判定の基準(DPR 1.7647、画面は物理画素の 1836×1056 CSS px の部屋。段 1〜3 の各 n=3 の全て):
- 段 1(無音・マウス 1000 Hz): `last60sUnderruns` が 1 以下で、`inputRateOk` が true(窓の全ての 1 秒で、ページが受けたポインタの件数が 1000 以上)。
- 段 2(再生・マウス無し): `playing` が true で、`last60sUnderruns` が 1 以下。
- 段 3(再生・マウス 1000 Hz): `playing` が true で、`last60sUnderruns` が 1 以下で、`inputRateOk` が true(窓の全ての 1 秒で、ページが受けたポインタの件数が 1000 以上)。

段 1〜3 の窓は、idle の +70 秒から +130 秒。`last60sUnderruns` は窓の終わりに読む値で、ちょうど窓の 60 秒の途切れになる。段 0(無音・無操作)は判定に入れない。その参考の値(`last60sUnderrunsAtIdle10s`)は JSON と報告に出す(起動の山の直し `#{{112}}` の領域)。

1 以下は、runner の共有 CPU(4 vCPU)の揺れを見込んだ値。報告に、実際の値と揺れの理由を書く。0 にできなければ、到達した値と残る要因を報告に書く。その場合、受け入れは通らないので、判定は PM が見直す。参考の測定(4)は、受け入れには入れない。

段 2・3 の条件は `playing`(再生中であること)。音の中身(`nonSilentQuanta`)は判定に入れず、JSON と報告には出す。無音のプロジェクトでも、再生中の engine の補充は途切れを数えるので、途切れの判定になる。音の出るプロジェクトでの測定は #{{004}} が行う。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には「runner の値。この機械とは別枠」と書き、nproc と loadavg を添える。
- 実測と推定を分ける。「達成」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。出力の行を付ける。
- 報告に書く: 段 1〜3 の表(n=3 の各回の `last60sUnderruns`・`underrunDelta`・`inputRateOk`・`nonSilentQuanta`・`screenDevicePx`)、段 0 の参考の値(`last60sUnderrunsAtIdle10s`・`last60sUnderruns`)、参考の表(4)、入力の周数の比例の確認、ポインタの送り方の選択と理由、取り除いた経路、変えたパス(app/・lift/・tools/ を含む。`app/` と `lift/` を変えると main の deploy の仕組みが動く)、変更の前後の値。
- 耳の確認は行わない。「音が途切れない」とは書かず、途切れ(underrun)の数で書く。

## 制約

- 製品に新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足さない。オプトインを作らない(計測の切り替えも含む)。
- 製品の既定の動作を、測定のためだけに変えない。
- ベンダー名と Cloudflare の識別子を書かない。
- sleep・timeout・nohup・末尾の `&` は使わない。一時ファイルは `.tmp-*` に置き、終わったら消す。
- 製品の作業は Actions で行う。この機械で製品のビルドや計測をしない。

## 作業の進め方

- 作業の最初に `date -u` を記録する。4 時間 30 分を過ぎたら、受け入れが通っていなくても、作業ツリーを commit し、報告を書いて終える(ジョブの上限は 5 時間 50 分)。
- 受け入れが通らなくても、作業の最後に作業ツリーを commit する。未 commit の変更が残ると、判定は失敗する。
- エンジンのビルドは 1 回約 35〜46 分。測定の回数は、ビルドを増やさずに同じビルドで行う。

## 他のタスクとの関係

- blocked_by: #{{040}}(音声の圧迫。#{{040}})。#{{040}} が閉じるまで起動しない。
- #{{112}}(起動して 10 秒後・無操作の途切れを 0 にする。段 0 の切り替え点)と並行する。段 0 は #{{112}} が直す。同じ `fle_thread.c` を変える場合は、後に入る方が pull して直す。
- #{{107}}(Fractional Scaling)は、描画の画面の大きさを変えるが、この Issue の段 1・2 に影響しない。段 3 の測定は、#{{107}} の前後で値が変わりうるので、報告に、画面の大きさ(guest の画面の画素数。JSON の `screenDevicePx`)を書く。
- #{{108}}(動的な変化と全ての拡大率)とは、画面の大きさの扱いを共有する。
- #{{004}}(音の出る入力の選択)と #{{105}}(demo の配信)の入力は、配信に入ってから使う。無い時は上の既定を使う。
- #{{121}}(demo の 2 件が開けない)と #{{122}}(サンプルが読めない)は、音の出るプロジェクトでの測定(#{{004}})の前提。この Issue の判定には入れない。
- 描画の経路(`app/src/engine/client.ts`・`desk.ts`・`render-worker.ts`)と入力の経路(`lift/runtime/fle_input.c`)は #{{040}} と同じ。衝突したら、作業の最後に `git pull --no-rebase --no-edit` で main を取り込み、衝突を直し、受け入れをもう一度通す。
