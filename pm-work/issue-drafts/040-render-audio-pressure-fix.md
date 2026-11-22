---
draft: "040"
title: "起動の直後の、guest の長い計算が音声の補充を止める問題を取り除く(切り替え点の挿入。起動して 10 秒後・無操作の途切れ 0)"
type: Task
milestone: "M1 描画と音声の分離(P3)"
parent: "001"
blocked_by: ["002"]
labels: ["render", "audio"]
size: L
priority: Urgent
project: signature-studio/3
---

## 目的

P3 の目標は、Song Mode で音が途切れないこと。目標どうしがぶつかるときは P3、P2、P1、サイズの順に優先する(`AGENTS.md`)。

依頼者の指摘(2026-11-23): **起動して約 10 秒後、再生もマウス操作も無い状態で、ヘッダーの `直近 60 秒の途切れ` が 343**(実機)。使い物にならないレベルで、パフォーマンスが最悪である。

この Issue の目標: **起動の直後の途切れを 0 にする**。具体的には、engine の idle の +10 秒と +70 秒に読んだ `直近 60 秒の途切れ` が、n=3 の全てで 0。併せて、音声の補充の停止の最大を 60 ms 以下にする(音声の先読みは 60 ms。この Issue では変えない)。

原因の見立て(前回の試行の実測。下の「前回の試行で分かったこと」): 起動の直後、guest の重い計算が、協調スケジューラの切り替えを通らずに続く。コールバックの中では切り替えが飛ばされ、その間は音声の補充も止まる。前回は、コールバックの中でも補充するようにして途切れを半分にしたが、0 にはならなかった。残りは、切り替え点も音声の呼び出しも無い長い計算(推定。関数は未特定)。

## 依頼者の指摘(2026-11-23)

- 起動して約 10 秒後、再生もマウス操作も無い状態で、ヘッダー `音: 動作中 44.1 kHz · 出力遅延 32 ms · 直近 60 秒の途切れ 343`(実機。画面 2 回。343 は、約 175 ms に 1 回の割合)。
- 同じ画面のログ: `[warn] the virtual clock fell 84 ms further behind the wall clock than the cap allows (S-clk-maxlag); it stays behind (no virtual time is skipped)`(別の画面では 94 ms)。
- 起動の時刻(実機): `Download … at 2.0 s`、`Compile … at 2.0 s`、`Engine running … ran 2.5 s (main() began at 4.0 s)`、`First pixel at 6.9 s`、`Idle at 6.4 s`(2 回目。保存した複製からの起動)。
- マウスを動かした後の値(別の貼り付け): `直近 60 秒の途切れ 420`、`input: 228 sent, 225 processed, 0 pending, 3 dropped/merged · engine cycle 0.00 s (1115.6/s)`、`last input: move at screen (3238,143) (pointermove)`。マウスの 1 イベントごとに engine が 1 周している。
- 画面: `guest screen 3240x1863 device px in a room of 1836x1056 CSS px (device pixel ratio 1.7647, 6.04 Mpx)`。依頼者のマウスは 8000 Hz で、互換性のため 1000 Hz に落として使っている。
- ヘッダーの値の数え方(`app/src` の読み): `underrunQuantaLast60s`(`app/src/audio/page-audio.ts` の `detail()`)。出力が PLAYING の間に、engine が次の 128 フレームを書き終えていなかった quantum を数える(`app/src/audio/fle-out.worklet.js` の冒頭)。PLAYING は、再生ボタンとは別に、engine が起動時に開始する状態。値は「60 秒前の標本からの差」なので、起動から 60 秒以内は、起動からの累計になる。ワークレットは AudioContext が running のときだけ処理するので、その間の途切れは数えられない(推定。段 0 の測定で確かめる)。

## 前回の試行で分かったこと

前回の作業者の試行(run 45045850171)は、判定が落ちて push されず、作業は失われた。以下は、その報告の数字と結論。変更前の版で測った値(窓 10 秒。窓の開始は `tools/p3-audio-probe/measure.mjs` と同じ、Space の直後。runner の値。nproc 4、loadavg 1 分値 1.6〜2.2。この機械とは別枠)。

1. **音声の補充の停止の主因は、描画ではない**。窓の 10 秒の内訳: 音声の計算 `audio_sync` 27.5 ms(窓の 0.3%)、混合 2.6 ms、書き込み 3.3 ms(892 回、1 回約 3.7 µs)。描画(compose)は約 40 回、合計約 5 ms(1 回約 0.13 ms)。
2. **否定された候補**: 描画(compose)。音声の計算と書き込み。アイドルの待ちが補充より長い(計画された待ちは 60 ms 以下、実際の最大は 12.7 ms。窓の 79% がアイドル)。協調の実時間待ち `fle_js_wait` は 0 回。
3. **時間の偏り**(1 秒ごとの値。窓の開始からの時刻):
   - 前半(−3 s 〜 +1.2 s。起動の直後で、Space の直後): スレッド 2 が約 93% を実行(区間の最大 209 ms の連続)。アイドル 0〜3%。`audio_sync` の呼び出し 430〜1,070/s。スライス終了の約 9 割がコールバックの中で飛ばされる(47〜103/s)。書き込みの間隔が 60 ms を超える事象は 3〜9/s。underrun の連続は 50〜830 ms。
   - 後半(+1.5 s 以降): スレッド 7 の `Sleep(1)` の繰り返し。アイドル約 94%。`audio_sync` 約 5,200〜5,500/s。遅れと間隔 > 60 ms は 0。underrun は +1.7 s より後は 0。
   - 注: 窓全体の集計で「スレッド 7 が約 93%」と書かれた箇所もあった。前半と後半を分けた集計を正とする。
4. **支持された原因**: コールバックの中での切り替えの飛ばし。`lift/runtime/fle_thread.c` の `fle_progress_slice`(792 行付近: `fle_callback_depth() > FLE_PREEMPT_MAX_DEPTH` の分岐で、切り替えを飛ばして戻る)。この間、音声の補充も止まる。
5. **前回の修正と結果**(失われた。作り直してよい): 上の分岐で、切り替えと仮想時計は従来どおりにし、**音声の補充だけを行う**。`lift/runtime/fle_dsound.c` に `fle_audio_poll_in_callback` を足し、外部の音声消費者(AudioWorklet)があるときだけ `audio_sync` を呼ぶ(ネイティブの仮想時計の動作は変えない)。
   - underrunDelta(10 秒): 変更前 185・192(各 n=1)。補充を無条件に行った版 135・120・95(n=3)。外部の消費者があるときだけ行った版 108・150・98(n=3、平均 119)。
   - 補充の停止の最大: 修正前 317 ms → 修正後 185 ms(累積の最大)。
   - 窓の中の最長の連続 underrun: 修正前 約 250 ms(85 quanta) → 修正後 38〜42 quanta(約 110〜120 ms)。
   - 評価: 約半分に減ったが、0 にはならない。修正後も前半に、切り替え点も `audio_sync` の呼び出しも無い長い計算が残る(書き込みの間隔が 60 ms を超える事象が 1 秒に 3〜9 回)。**関数は未特定**。関数別の計測はしていない。
6. **先読み**: `FLE_AUDIO_LEAD_MS` は 60 ms(`lift/runtime/fle_audio_ring.h` の 23 行)。60 ms より増やせば、185 ms までの停止は吸収できるが、入力から音までの遅れが増える(`S-audio-lead`。依頼者の決定の対象)。**この Issue では先読みを変えない**。
7. **受け入れの窓の位置**: 前回の受け入れ(UI の負荷あり、窓 10 秒、10 秒あたり 20 以下)は、Space の直後の起動の山の後半に当たっていた。定常(+1.7 s より後)の途切れは 0 だった(推定)。この 10 秒窓の基準は、この Issue では使わない。
8. **取れていないもの**: 関数別の所要(`fle_*` の内訳)、AudioWorklet の CPU 時間、quantum ごとのジッタ、Song Mode の判定、耳の確認。

### 2 回目の試行(判定が落ちた。作業は `.prior-work.md` で続きから載る)

2 回目の試行(run 45072464291)は、判定が `commit していない変更がある` で落ちた。作業者が commit せずにターンを終えたため。作業ツリーの 4 ファイル(`lift/runtime/fle_audio_transport.c`・`fle_dsound.c`・`fle_thread.c`、`tools/p3-audio-probe/measure.mjs`)は、次の作業に載る。以下は、その報告の数字(runner の値。この機械とは別枠。nproc 4、loadavg 0.36〜0.57)。

- 入れた物: 前節 5 の修正の入れ直し(`fle_audio_poll_in_callback`)。補充の計数(Worker 側。`fleAudioWorkerState` に 1 項目。動作は変えない)。`tools/p3-audio-probe/measure.mjs` の `--boot`。既存の不具合を直した: `return` で終わると JSON が出なかったので、JSON の出力を `finally` に移した。
- 部屋の寸法: `--boot` の部屋は `#stage` の寸法(ビューポートから、モニターの 340 px とヘッダーの 36 px を除いたもの)。CSS 1836×1056 の部屋には、ビューポート 2176×1092 を使い、DPR は 30/17 にする(1.7647 のままでは画面が 3239×1863 になり、受け入れの `3240x1863` にならない)。
- 測定(最終の設定。窓は idle の +10 秒〜+70 秒。押下は idle を見た直後に 1 回):

| 回 | screenDevicePx | ctxState | quantaDelta | bootLast60sAt10s | bootLast60sAt70s | maxRefillStallMs | 連続 underrun の最長(quanta) |
|---|---|---|---|---|---|---|---|
| 1 | 3240x1863 | running | 20676 | 447 | 0 | 554.6 | 35 |
| 2 | 3240x1863 | running | 20675 | 516 | 0 | 566.7 | 36 |
| 3 | 3240x1863 | running | 20675 | 556 | 0 | 571.5 | 39 |

- 結果: 目標に届かない。`bootLast60sAt70s` は 3 回とも 0。`bootLast60sAt10s` は 447〜556、`maxRefillStallMs` は 555〜572 ms(目標は 0 と 60 以下)。変更の前後の表は、この試行では作っていない(修正を入れない版の bundle を作り直していない)。
- 関数の特定(未完): CDP の `Profiler` を Worker に付けた(navigation から idle の +30 秒まで。サンプリング 250 µs)。関数名は `wasm-function[N]` だけで、guest の RVA に戻せなかった。自己時間の上位は `wasm-function[19053]`(1,337 ms、3.7%)など。上位 20 件は、どれも単独では 120 ms 程度。idle は 69.2%。「切り替え点を通らずに続いた最長」は未確認(枠の名前が無く、`(root)` が 6,138 ms と出た)。`link.map` と最適化後の `engine.wasm` の番号が一致する保証は無い。
- 作業者の見立て(PM へ): (1) 関数の番号を名前に戻す手段が要る。(2) `maxRefillStallMs`(555〜572 ms)と、連続 underrun の最長(35〜39 quanta、約 100〜115 ms)が合わない。計数の定義を検証する。(3) 押下より前の停止は数えていない。依頼者の実機の 343 は押下の後の数字で、この定義で合うかは未確認。(4) 前景の待ちに約 1 時間使った(下の設計の (iv))。

## 作業の順序

1. **修正を入れ直す**: 前節 5 の修正(`lift/runtime/fle_thread.c` と `lift/runtime/fle_dsound.c`)。単独では 0 にならないことは実測済み。以降の作業の土台にする。
2. **長く走る関数を特定する**(計測。一時的な変更として入れ、受け入れの前に取り除く)。
   - 前提(コードの読みで確かめた。計測ではない): 切り替えの計数は、guest の間接呼び出しでしか進まない(`lift/runtime/fle_dispatch.h` の 177 行の `++fle_progress_count`)。直接の呼び出しと、ループの後方辺では進まない。スライスは 4096 回の間接呼び出しごと(`lift/runtime/fle_thread.h` の `FLE_PREEMPT_SLICE_DISPATCHES`)。だから、間接呼び出しの無い長いループは、切り替えを通らずに走りうる。
   - 方法: CDP の `Profiler` で、engine の Worker に接続し(`Target.setAutoAttach` で Worker の対象へ。`Profiler.start` と `Profiler.stop`)、wasm の関数の自己時間を取る。関数名が取れない場合は、`lift/` の生成物の関数の名前(`lower 0x…` など)から、guest の RVA に対応づける。取り方は作業者が決め、報告に書く。
   - 対象: 起動の直後(engine の idle の前後 0〜30 秒)。出すもの: (a) 自己時間の上位 20 関数(割合と ms)。(b) 各関数について、切り替え点を通らずに続いた最長の時間(ms)。
   - 計測の切り替えは、既存の仕組み(既存の記録の経路や、既存の引数)だけで行う。新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数は作らない。既定では無効にする。
3. **切り替え点を入れる**(特定した長い計算に)。方式は作業者が選び、実測で比べ、理由を報告に書く。
   - (a) 生成器(`lift/crates/fe-lift`・`fe-wasmgen`・`fe-build`)で、ループの後方辺と関数の入口に、カウンタの減算と、0 以下で協調スケジューラへ戻る呼び出しを入れる。全関数ではなく、特定した上位の関数と、そこから呼ばれるループの多い関数に限ってよい。全関数に入れる場合は、P3 の再生中の性能への影響を実測し、許容範囲に収める。
   - (b) 該当の guest の処理が、特定の API(`WaitForSingleObject`・`Sleep`・`SwitchToThread` など)を呼ぶ所で、切り替えを増やす。
   - どちらの方式でも、音声を作る guest のスレッドが、締め切り(先読み 60 ms)の前に必ず走ることを保つ。
   - 製品に新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足さない。既定の動作として直す。
4. **測定の道具に起動段の測定を足す**: `tools/p3-audio-probe/measure.mjs` に `--boot` を足す。既存の窓・`--load ui`・既存の出力の項目は変えない。
   - 画面: CSS 1836×1056・DPR 1.7647(依頼者の実機と同じ。guest の画面は 3240×1863 の物理画素。`Emulation.setDeviceMetricsOverride` などで作る)。
   - 0 とする時刻: `window.__app.state` が `engine-idle` を最初に観測した時刻(ポーリングは 50 ms 以下)。
   - 窓: idle の +10 秒から +70 秒。
   - 押下: AudioContext を running にするための、状態の行への押下を 1 回だけ、窓の前に行ってよい(`startAudio` と同じ。ゲストには届かない)。窓の中では、何も操作しない。Space を押さない。ゲストの同意の箱(No thanks)を押さない。マウスを動かさない。
   - JSON(stdout に 1 行): `error`・`state`・`standIn`・`screenDevicePx`(`window.__app.screen.guest` の幅×高さ。例 `3240x1863`)・`dpr`・`ctxState`(窓の終わりの AudioContext の状態)・`quantaDelta`(idle+10 から +70 の `quantaWhilePlaying` の増分。期待値は約 20,600)・`bootLast60sAt10s`(idle+10 秒に読んだ `underrunQuantaLast60s`)・`bootLast60sAt70s`(idle+70 秒に読んだ値)・`maxRefillStallMs`・`nproc`・`loadavg`。
   - `maxRefillStallMs`: 音声の補充の停止の最大(ms)。音声の出力が PLAYING になった時刻から、idle の +70 秒まで、engine が出力のリングへ補充した時刻の、連続する 2 回の差の最大。取り方(engine 側の一時的な計数か、ページ側の標本か)は作業者が決め、報告に書く。

## 設計の追加(2 回目の試行の結果から)

- (i) **関数の名前の表**: 生成器(`lift/crates/fe-wasmgen`・`fe-build`)は、ビルドのたびに、wasm の関数の番号 → guest の RVA と関数名の対応を、`lift/out-app/gen/` の下の別のファイルに書く。配信の対象に入れない(engine.wasm の大きさを増やさない)。新しい旗・環境変数は足さない。測定の道具は、CDP の `Profiler` の結果を、この表で関数の名前に戻す。
- (ii) **計数の定義の検証**: `maxRefillStallMs` と連続 underrun の最長が合わない理由(単位・起点・quantum の長さ・先読みの扱い)を、コードを読んで特定し、直す。受け入れが読む `bootLast60sAt10s`・`bootLast60sAt70s` が、アプリのヘッダーの `直近 60 秒の途切れ` と同じ定義であることを、該当の行(ファイル・行番号)を引いて報告に書く(`app/src/audio/page-audio.ts` の `detail()`、`app/src/audio/status.ts`)。
- (iii) **押下より前**: AudioContext を始める利用者の操作(押下)より前は、消費者が無いので途切れとして数えない(アプリの定義どおり)。押下の直後の数秒には、起動の直後の山が残ることがある。測定の道具は、押下を idle の直後(+1 秒)に行う条件(既定。受け入れが読む。起動の山を含むため、厳しい方)と、idle の +30 秒に行う条件の両方を測り、報告に表で出す。
- (iv) **作業の進め方**: ビルドの完了は、フォアグラウンドで、シェルツールの `timeout` 引数に大きい値(例 3600000)を渡して待つ。バックグラウンドで起動して、完了の通知を待つ形にしない(ターンを終えると作業が止まる)。受け入れが通らなくても、作業の最後に作業ツリーを commit する(判定は commit 済みを要求する。通らなければ push されず、差分が保存される)。

## 受け入れ

```accept
npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false?0:1)' "$RUNNER_TEMP/boot.json" && mkdir -p .tmp-1334 && for i in 1 2 3; do node tools/p3-audio-probe/measure.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" --boot > ".tmp-1334/boot-$i.json" || exit 1; done && node -e 'const fs=require("fs");const L=[1,2,3].map((i)=>JSON.parse(fs.readFileSync(".tmp-1334/boot-"+i+".json","utf8").trim()));process.exit(L.every((r)=>r.error===null&&r.state==="engine-idle"&&r.standIn===false&&r.screenDevicePx==="3240x1863"&&r.ctxState==="running"&&r.quantaDelta>=19000&&r.bootLast60sAt10s===0&&r.bootLast60sAt70s===0&&r.maxRefillStallMs!==null&&r.maxRefillStallMs<=60)?0:1)' && rm -rf .tmp-1334
```

判定の基準(道具の 3 回すべて): `error` が null、`state` が engine-idle、`standIn` が false、`screenDevicePx` が 3240x1863、`ctxState` が running、`quantaDelta` が 19,000 以上(窓の 60 秒で音声が動いていたことの確認)、`bootLast60sAt10s` と `bootLast60sAt70s` が 0、`maxRefillStallMs` が 60 以下。

0 にできないときは、到達した値(3 回の `bootLast60sAt10s`・`bootLast60sAt70s`・`maxRefillStallMs`)と、残る関数(上位 5 件)を報告に書く。その場合、受け入れは通らないので、PM が判定を見直す(「差し戻し」とは書かない)。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には「runner の値。この機械とは別枠」と書き、nproc と loadavg を添える。
- 実測と推定を分ける。「達成」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。出力の行を付ける。
- 報告に書く: 変更の前後の表(`bootLast60sAt10s`・`bootLast60sAt70s`・`maxRefillStallMs`・連続 underrun の最長)、特定した関数の表(上位 20 関数と、切り替え点を通らずに続いた最長の時間)、切り替え点の方式と理由、P3 の再生中の性能(再生中の engine の周あたりの時間が変わったか)、変えたパス(`app/`・`lift/`・`tools/` を含む。`app/` と `lift/` を変えると main の deploy が動く)。
- 耳の確認は行わない。「音が途切れない」とは書かず、途切れ(underrun)の数で書く。

## 制約

- 製品に新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足さない。オプトインを作らない(計測の切り替えも含む)。
- 製品の既定の動作を、測定のためだけに変えない。
- 音声の先読み(`FLE_AUDIO_LEAD_MS`)は変えない(依頼者の決定の対象)。
- ベンダー名と Cloudflare の識別子を書かない。
- sleep・timeout・nohup・末尾の `&` は使わない。一時ファイルは `.tmp-*` に置き、終わったら消す。
- 製品の作業は Actions で行う。この機械で製品のビルドや計測をしない。

## 読む物

- `lift/runtime/fle_thread.c`(780〜800 行: `fle_progress_slice` と、792 行のコールバックの分岐)、`lift/runtime/fle_thread.h`(80〜92 行: 切り替えの決まりと `FLE_PREEMPT_MAX_DEPTH`)、`lift/runtime/fle_dispatch.h`(35 行と 177 行: 計数)
- `lift/runtime/fle_dsound.c`(416 行 `audio_sync`、471 行 `fle_audio_poll`)、`lift/runtime/fle_audio_transport.c`(54 行 `fle_js_publish`、108 行付近 `fle_js_wait`)、`lift/runtime/fle_audio_ring.h`(23 行 `FLE_AUDIO_LEAD_MS`)、`lift/runtime/fle_audio_ring.c`(148〜176 行)
- `lift/harness/fruity_idle.c`(65 行 `fle_thread_idle_hook`: アイドルの待ち。JSPI で止まり、ページが次の締切まで 1 本のタイマーで待つ)
- `lift/crates/fe-lift`・`lift/crates/fe-wasmgen`・`lift/crates/fe-build`(生成器)
- `app/src/audio/page-audio.ts`(`detail()`・`underrunQuantaLast60s`)、`app/src/audio/fle-out.worklet.js`(途切れの数え方)、`app/src/audio/status.ts`(ヘッダーの文言)
- `app/src/engine/idle-driver.ts`(`S-clk-maxlag`)、`app/src/engine/worker.ts`(138 行付近: 仮想の時計の警告)
- `tools/p3-audio-probe/measure.mjs`(#{{003}} の道具。`--boot` を足す)、`tools/boot-check/boot-bundle.mjs`・`cdp.mjs`・`stage-assets.mjs`
- `pm-work/findings/c2-split-measure.md`(#{{002}} の測定)、`pm-work/findings/survey-c2-worker-split.md`

## 他のタスクとの関係

- blocked_by: #{{002}}(分離の前後の測定と、残る要因の候補)。
- #{{109}}(実機の条件の段階的な測定)は、この Issue の後に、段 1〜3(マウス 1000 Hz・再生)を直す。段 0(無音・無操作)は、この Issue で直す。
- #{{004}}(所定のプロジェクトの実測)と #{{027}}〜#{{029}}(外部プラグインの処理)は、この Issue の完了を待つ。音声の経路が決まってから、プラグインの処理を入れるため。
- 描画の Worker を変えるので、同じ経路(`app/src/engine/client.ts`・`desk.ts`・`render-worker.ts`)を変える #{{006}}(HiDPI)と #{{010}}(メニュー)と衝突しうる。衝突したら、作業の最後に `git pull --no-rebase --no-edit` で main を取り込み、衝突を直し、受け入れをもう一度通す。
