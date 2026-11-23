---
draft: "040"
title: "起動直後の音声の補充の停止を測る道具と、関数名の表を入れ、コールバック内の補充の修正を入れる(切り替え点の実装は次の Issue)"
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

この Issue は、1 回の試行で終わる大きさに分けた(PM の決定。3 回の試行で、受け入れの全てには届かなかった)。この Issue で入れるもの:

1. 起動直後の音声の補充の停止を測る道具(`tools/p3-audio-probe/measure.mjs --boot`)と、wasm の関数の番号から guest の関数名に戻す表(製品のビルドで書き出す)。
2. 計数の定義の食い違い(`maxRefillStallMs` と連続 underrun の最長)を確かめた結果。
3. コールバック内の切り替えの飛ばしで、外部の音声の消費者があるときだけ音声の補充を行う修正(作業ツリーに在る。`fle_audio_poll_in_callback`)。

途切れを 0 にする切り替え点の実装は、`#{{112}}` に分けた。この Issue の受け入れは、途切れの目標値(0 と 60 ms 以下)を含まない。

原因の見立て(3 回の試行の実測。下の「前回の試行で分かったこと」): 起動の直後、guest の重い計算が、協調スケジューラの切り替えを通らずに続く。コールバックの中では切り替えが飛ばされ、その間は音声の補充も止まる。コールバックの中でも補充すると途切れは減るが、0 にはならない。3 回目の試行で、長い計算の上位の関数を特定した(`f_344fcf0` など。下の表)。どのスレッドの、どの連続の計算が締め切りを超えるかは、`#{{112}}` で確定する。

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

### 3 回目の試行(run 45084966641。約 4 時間 47 分。受け入れに届かず判定が落ちた。作業は `work.diff` として artifact に残った)

- 受け入れの実測(最終の版。n=3。窓は idle の +10 秒から +70 秒。`measure.mjs --boot`。runner の値):

| 回 | bootLast60sAt10s | bootLast60sAt70s | maxRefillStallMs (ms) | 連続 underrun の最長 (quanta) | quantaDelta | screenDevicePx | ctxState |
| --- | --- | --- | --- | --- | --- | --- | --- |
| 1 | 878 | 0 | 722.0 | 63 | 20672 | 3240x1863 | running |
| 2 | 1112 | 0 | 710.1 | 69 | 20672 | 3240x1863 | running |
| 3 | 978 | 0 | 700.9 | 60 | 20675 | 3240x1863 | running |

- 結果: `bootLast60sAt70s` は 3 回とも 0(定常は 0)。`bootLast60sAt10s` は 878〜1112(目標 0)。`maxRefillStallMs` は 700〜722 ms(目標 60 以下)。2 回目の試行の 3 回は 447〜556 だった。同じ構成でも値がばらつくため、1 回の比較は弱い。
- 作業ツリーに残った変更(`work.diff`。5 パス。次の作業者は、取り込まれた物を取り除かずに使う):
  - `lift/runtime/fle_thread.c`: `fle_progress_slice` のコールバックの分岐で、`fle_audio_poll_in_callback` を呼ぶ(1 行)。
  - `lift/runtime/fle_dsound.c`: `fle_audio_poll_in_callback`。外部の音声の消費者(AudioWorklet)があるときだけ `audio_sync` を呼ぶ。native の動作は変えない。
  - `lift/runtime/fle_audio_transport.c`: `fleAudioWorkerState` に `maxPubGapMs`(publish の間隔の最大。出力が両方の区間で進んだときだけ数える。計数のみ)。
  - `tools/p3-audio-probe/measure.mjs`: `--boot`(既存の窓・`--load ui`・出力の項目は変えていない)。
  - `pm-work/findings/boot-stall-1334.md`(新規。3 回の実測・関数の表・深さ 2 の実験・計数の食い違い)。
- 関数の特定(CDP の `Profiler`。engine の Worker。接続〜idle+15 秒。1 回。名前を付けた一時の版で取った。製品には残していない):
  - idle の後の自己時間(ms。括弧は、呼び出しスタックに連続して在った最長。idle の後だけ): `f_344fcf0` 1713(101)、`f_27ad5f8` 642(68)、`fle_mx_addsd` 461(1.6)、`fle_mx_subsd` 277(1.3)、`fle_mx_addss` 266(5.9)、`fle_mx_mulsd` 171(2.0)、`f_33d6e70` 133(6.7)、`fle_mx_mulss` 121(1.5)、`fle_lg_psrldq` 112(5.6)、`f_27acd60` 107(43)、`fle_hyb_dispatch` 37(163。呼び出しの鎖を含む)。名前は生成器の VA(`f_<16 進>`)。`fle_mx_*`・`fle_lg_*` は SSE 命令のランタイム(演算ごとの呼び出し)。
  - idle の前(約 8.5 秒まで): メインのスレッドが `DispatchMessageW → call_wndproc` の中に 367 ms 連続で在った(推定: 起動直後の連続はこの間の物)。`vs_sha256_hex` の連続は 131 ms。コールバック内では切り替えが飛ばされる(`fle_progress_slice`)。
  - idle の後: engine の Worker は 62% が idle(CPU は平均では飽和していない)。長い計算の根は `fle_wasm_thread_entry`(guest のスレッド)。
- 深さ 2 の実験(1 回。`FLE_PREEMPT_MAX_DEPTH` を 1 → 2。ウィンドウ手続きの中で切り替えを許す): `bootLast60sAt10s` 959 → 764、`maxRefillStallMs` 681 → 1215。停止は悪化したので採らず、戻した。
- 計数の食い違い(未確認): `maxRefillStallMs`(約 700 ms)に対し、連続 underrun の最長は 60〜69 quanta(約 175〜200 ms)。推定: `maxRefillStallMs` は publish の間隔の最大で、欠落の長さではない(先読み 60 ms を超えて ring に残っていれば、間隔が長くても欠落にならない)。定義の出典: `app/src/audio/page-audio.ts` 234 行(`winMs = 60000`)と 250 行(`underrunQuantaLast60s = c.urQuanta - base.ur`)、`app/src/audio/fle-out.worklet.js` の `UR_QUANTA`、`app/src/audio/status.ts` 18〜19 行。
- 名前の付いた版を作った経路(一時。製品には残していない): 生成器の関数名の出力は既定で無効(CLI からは有効にできない)。wasm-opt の `-g`、hybrid の wasm-merge の `-g`、link の `--profiling-funcs` が要った(`-g1` では名前が残らなかった)。C 部分の名前の節は約 2.2 MB。名前の節を `engine.wasm` に入れると大きさが増えるので、製品の名前は別のファイルに出す(下の「作業の順序」2)。
- 判定の結果: 受け入れは exit 1 で通らず、判定は push しなかった。時間の制約で、製品の追加の修正と再ビルドは行わなかった。
- ビルドの所要(参考): engine の `fe-build engine` は約 35 分、`fe-build bundle` は約 11 分、受け入れの 3 回は約 50 分。

## 作業の順序

作業の始めに、前回の作業が作業ツリーに載っている(`.prior-work.md` が指示する。衝突が在れば、その解消から始める)。前回の変更は取り除かず、残りを行う。

1. **作業ツリーの確認**: `git status`・`git diff HEAD` で、上の「3 回目の試行」の 5 パスの変更が載っていることを確かめる。違えば、違いを報告に書いてから進む。
2. **関数名の表(必須)**: 生成器(`lift/crates/fe-wasmgen`・`fe-build`)が、ビルドのたびに、wasm の関数の番号 → guest の RVA(VA)・関数名の対応を、`lift/out-app/gen/wasm-func-names.tsv` に書く(タブ区切り。1 行 1 関数。C の runtime の関数(`fle_progress_slice` など)も行に入れる)。配信の対象に入れない。`engine.wasm` の大きさを増やさない(名前の節を入れない)。新しい旗・環境変数・`#ifdef` は足さない。名前の取り方(生成器の既定で無効の関数名の出力を使う、link の map を使う、など)は作業者が選び、理由を報告に書く。
3. **表で関数名に戻す**: CDP の `Profiler` の結果を、この表で関数名に戻す手段を `tools/p3-audio-probe/` に置く(一時の解析でよい。受け入れには入れない)。
4. **計数の定義の検証**(3 回目の試行の計数の食い違い): publish ごとの `WR − RD` と、worklet の quantum ごとの `fill` を、同じ時刻で記録し、`maxRefillStallMs`(publish の間隔の最大)と連続 underrun の最長が何を測っているかを確かめる。計測の切り替えは、既存の経路(CDP で読む値)だけで行い、製品に新しい旗を足さない。結果は `pm-work/findings/boot-stall-1334.md` に書く。
5. **押下の 2 条件**: `measure.mjs --boot` の押下を、idle の +1 秒と +30 秒の 2 条件で測り、両方の結果を `pm-work/findings/boot-stall-1334.md` に表で出す(押下の条件ごとの `bootLast60sAt70s`・`maxRefillStallMs`)。
6. **受け入れと commit**: 下の受け入れを 1 回通す。作業ツリーを commit する(受け入れが通れば push される)。通らなければ、その出力を報告に貼り、commit する。

## 設計の追加(2・3 回目の試行の結果から)

- (i) 関数名の表: 上の「作業の順序」2 と 3。生成器が、wasm の関数の番号 → guest の RVA・関数名を、`lift/out-app/gen/` の下の別のファイルに書く。配信の対象に入れない。
- (ii) 計数の定義: 上の「作業の順序」4。受け入れが読む `bootLast60sAt10s`・`bootLast60sAt70s` は、アプリのヘッダーの `直近 60 秒の途切れ` と同じ定義(`app/src/audio/page-audio.ts` の `detail()`)。この Issue はその定義を変えない。
- (iii) 押下より前: 上の「作業の順序」5。AudioContext を始める押下より前は、消費者が無いので途切れとして数えない(アプリの定義どおり)。
- (iv) 作業の進め方: ビルドの完了は、フォアグラウンドで、シェルツールの `timeout` 引数に大きい値(例 3600000)を渡して待つ。バックグラウンドで起動して、完了の通知を待つ形にしない(ターンを終えると作業が止まる)。受け入れが通らなくても、作業の最後に作業ツリーを commit する。

## 受け入れ

```accept
npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false?0:1)' "$RUNNER_TEMP/boot.json" && mkdir -p .tmp-1334 && for i in 1 2 3; do node tools/p3-audio-probe/measure.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" --boot > ".tmp-1334/boot-$i.json" || exit 1; done && node -e 'const fs=require("fs");const L=[1,2,3].map((i)=>JSON.parse(fs.readFileSync(".tmp-1334/boot-"+i+".json","utf8").trim()));const px=(r)=>Math.floor(1836*r.dpr)+"x"+Math.floor(1056*r.dpr);process.exit(L.every((r)=>r.error===null&&r.state==="engine-idle"&&r.standIn===false&&r.ctxState==="running"&&r.quantaDelta>=19000&&r.bootLast60sAt70s===0&&r.screenDevicePx===px(r))?0:1)' && test "$(wc -l < lift/out-app/gen/wasm-func-names.tsv)" -ge 100000 && grep -q fle_progress_slice lift/out-app/gen/wasm-func-names.tsv && test -f pm-work/findings/boot-stall-1334.md && rm -rf .tmp-1334
```

判定の基準(道具の 3 回すべて): `error` が null、`state` が engine-idle、`standIn` が false、`ctxState` が running、`quantaDelta` が 19,000 以上(窓の 60 秒で音声が動いていたことの確認)、`bootLast60sAt70s` が 0、`screenDevicePx` が、道具の `dpr`(既定 30/17 ≈ 1.7647)から `Math.floor(1836 × dpr)` × `Math.floor(1056 × dpr)` になる。切り捨ては、アプリの `chooseGuestScreen`(`app/src/ui/screen-fit.ts`)と同じ。丸めると高さが 1864 になり、実測の 1863 と合わない。

関数名の表 `lift/out-app/gen/wasm-func-names.tsv` が在り、行数が 100,000 以上で、`fle_progress_slice` の行を含む。`pm-work/findings/boot-stall-1334.md` が在る。

`bootLast60sAt10s` と `maxRefillStallMs` の目標値は、この Issue の受け入れに入れない(`#{{112}}` が行う)。報告に 3 回分の値を書き、0 にできなかった理由(推定でよい)を書く。

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
- #{{112}}(起動して 10 秒後・無操作の途切れを 0 にする。切り替え点の実装)は、この Issue の後に行う。この Issue の道具(`measure.mjs --boot`・関数名の表・`pm-work/findings/boot-stall-1334.md`)を使う。
- #{{109}}(実機の条件の段階的な測定)は、この Issue の後に、段 1〜3(マウス 1000 Hz・再生)を直す。段 0(無音・無操作)は #{{112}} が直す。
- #{{004}}(所定のプロジェクトの実測)と #{{027}}〜#{{029}}(外部プラグインの処理)は、この Issue の完了を待つ。音声の経路が決まってから、プラグインの処理を入れるため。
- 描画の Worker を変えるので、同じ経路(`app/src/engine/client.ts`・`desk.ts`・`render-worker.ts`)を変える #{{006}}(HiDPI)と #{{010}}(メニュー)と衝突しうる。衝突したら、作業の最後に `git pull --no-rebase --no-edit` で main を取り込み、衝突を直し、受け入れをもう一度通す。
