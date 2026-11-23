---
draft: "112"
title: "起動して 10 秒後・無操作の途切れを 0 にする: 起動の最中の guest の長い計算に、切り替え点を入れる"
type: Task
milestone: "M1 描画と音声の分離(P3)"
parent: "001"
blocked_by: ["040"]
labels: ["audio", "perf", "render"]
size: XL
priority: Urgent
project: signature-studio/3
---

## 目的

P3 の目標は、Song Mode で音が途切れないこと。目標どうしがぶつかるときは P3、P2、P1、サイズの順に優先する(`AGENTS.md`)。

依頼者の指摘(2026-11-23): **起動して約 10 秒後、再生もマウス操作も無い状態で、ヘッダーの `直近 60 秒の途切れ` が 343**(実機)。使い物にならないレベルで、パフォーマンスが最悪である。

この Issue の目標: **engine の idle の +10 秒と +70 秒に読んだ `直近 60 秒の途切れ`(ヘッダーと同じ定義の値)を、n=3 の全てで 0**。併せて、音声の補充の停止の最大(`maxRefillStallMs`)を 60 ms 以下にする(音声の先読みは 60 ms。この Issue では変えない)。

## 背景

- 前提(#{{040}} が入れた道具): `tools/p3-audio-probe/measure.mjs --boot`(起動の窓。idle の +10 秒と +70 秒に読む)、関数名の表(`lift/out-app/gen/wasm-func-names.tsv`。製品のビルドで書き出す)、`pm-work/findings/boot-stall-1334.md`(3 回の試行の実測と、計数の検証の結果)。
- 3 回の試行の結果(#{{040}} の run 45084966641。runner の値。n=3): `bootLast60sAt70s` は 3 回とも 0。`bootLast60sAt10s` は 878・1112・978(目標 0)。`maxRefillStallMs` は 722.0・710.1・700.9 ms(目標 60 以下)。連続 underrun の最長は 63・69・60 quanta(約 175〜200 ms)。
- 関数の特定(3 回目の試行。CDP の `Profiler`。engine の Worker。接続〜idle+15 秒。1 回。計測のための一時の版): idle の後の自己時間の上位は `f_344fcf0` 1713 ms(呼び出しスタックに連続して在った最長 101 ms)、`f_27ad5f8` 642 ms(68 ms)、`fle_hyb_dispatch` 37 ms(連続 163 ms。呼び出しの鎖を含む)、`f_27acd60` 107 ms(43 ms)、`f_33d6e70` 133 ms(6.7 ms)。`fle_mx_*`・`fle_lg_*` は SSE 命令のランタイムで、演算ごとの呼び出し(連続 6 ms 以下)。根は `fle_wasm_thread_entry`(guest のスレッド)。
- 起動直後: メインのスレッドが `DispatchMessageW → call_wndproc` の中に 367 ms 連続で在った(idle の前。約 8.5 秒まで。推定: 起動直後の連続の由来)。コールバック内では切り替えが飛ばされ(`lift/runtime/fle_thread.c` の `fle_progress_slice`。深さが `FLE_PREEMPT_MAX_DEPTH` を超えたとき)、その間は音声の補充も止まる。
- #{{040}} で、コールバック内の切り替えの飛ばしのとき、外部の音声の消費者があるかぎり音声の補充を行うようにした(`fle_audio_poll_in_callback`)。それでも 0 にはならない。
- `FLE_PREEMPT_MAX_DEPTH` を 1 → 2 にした実験(1 回): `bootLast60sAt10s` 959 → 764 だが、`maxRefillStallMs` 681 → 1215 ms に悪化。ウィンドウ手続きの中で切り替えを許すだけでは、停止は減らない。原因は未確認(再入の安全性か、切り替え後の別のスレッドの待ち方か)。
- 切り替えの計数は、間接呼び出しでしか進まない(`lift/runtime/fle_dispatch.h` の 177 行の `++fle_progress_count`)。直接の呼び出しと、ループの後方辺では進まない。スライスは 4096 回の間接呼び出しごと(`lift/runtime/fle_thread.h` の `FLE_PREEMPT_SLICE_DISPATCHES`)。したがって、間接呼び出しの無い長いループは、切り替えを通らずに走りうる(推定。コードの読み)。
- 未確定: 締め切り(先読み 60 ms)を超えて切り替えが無い区間が、どのスレッドの、どの関数の、どのループかは確定していない。`maxRefillStallMs` の定義(publish の間隔の最大か、欠落の長さか)も、#{{040}} の計数の検証の結果を見てから使う(`pm-work/findings/boot-stall-1334.md` の計数の節)。

## 読む物

- `pm-work/findings/boot-stall-1334.md`(3 回の試行の実測。関数の表。深さ 2 の実験。計数の食い違い)
- `lift/runtime/fle_thread.c`(`fle_progress_slice` のコールバックの分岐と、`fle_audio_poll_in_callback` の呼び出し)、`lift/runtime/fle_thread.h`(協調のスレッド。切り替えの規則・`FLE_PREEMPT_SLICE_DISPATCHES`・`FLE_PREEMPT_MAX_DEPTH`)、`lift/runtime/fle_dispatch.h`(切り替えの計数。177 行)
- `lift/runtime/fle_dsound.c`(`audio_sync`・`fle_audio_poll_in_callback`)、`lift/runtime/fle_audio_transport.c`(`fle_js_publish`・`fleAudioWorkerState`)、`lift/runtime/fle_audio_ring.h`(`FLE_AUDIO_LEAD_MS`)、`lift/runtime/fle_audio_ring.c`
- `lift/harness/fruity_idle.c`(`fle_thread_idle_hook`: アイドルの待ち)
- `lift/crates/fe-lift`・`lift/crates/fe-wasmgen`・`lift/crates/fe-build`(生成器。切り替えの点を入れる場所)
- `app/src/audio/page-audio.ts`(`detail()`・`underrunQuantaLast60s`)、`app/src/audio/fle-out.worklet.js`、`app/src/audio/status.ts`
- `app/src/engine/idle-driver.ts`(`S-clk-maxlag`)、`app/src/engine/worker.ts`
- `tools/p3-audio-probe/measure.mjs`(`--boot`)、`tools/boot-check/boot-bundle.mjs`・`cdp.mjs`・`stage-assets.mjs`

## やること

0. 変更の前の値を最初に測り、`pm-work/findings/boot-baseline-112.json` に commit する。変更の前の版は、#{{040}} が main に入った後の版(作業ツリーの変更を入れる前)。`measure.mjs --boot` を n=3 で行い、JSON の 3 回分をまとめる(`bootLast60sAt10s`・`bootLast60sAt70s`・`maxRefillStallMs`・`quantaDelta`・`screenDevicePx`)。連続 underrun の最長も同じ 3 回で測る(測り方は作業者が決め、報告に書く)。
1. 停止の原因を、どのスレッドの、どの連続の計算かまで特定する。`pm-work/findings/boot-stall-1334.md` の関数の表と、#{{040}} が入れた道具(関数名の表・CDP の `Profiler`)で、スレッド別の自己時間と、切り替え点を通らずに続いた最長の時間を出す。
2. 切り替え点を入れる。方式の候補(作業者が選び、実測で比べ、理由を報告に書く):
   - (a) 生成器(`lift/crates/fe-lift`・`fe-wasmgen`)が、ループの後方辺と関数の入口に、カウンタの減算と、0 以下で協調スケジューラへ戻る呼び出しを入れる。上位の関数と、そこから呼ばれるループの多い関数に限ってよい。全関数に入れる場合は、P3 の再生中の性能への影響を実測し、許容範囲に収める。
   - (b) ウィンドウ手続きの中(コールバック内)でも、音声を作るスレッドへ切り替えられるようにする。`FLE_PREEMPT_MAX_DEPTH` の深さ 2 の実験が悪化した原因を調べ、再入の安全性(FL のメッセージ処理が再入を許す箇所と許さない箇所)を満たす形にする。
   - (c) 起動の最中の重い処理を、締め切りに合わせて分割する(特定の API の呼び出し点で切り替える)。
   どの方式でも、音声を作る guest のスレッドが、締め切り(先読み 60 ms。この Issue では変えない)の前に必ず走ることを保つ。製品に新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足さない。既定の動作として直す。
3. P3 の再生中の性能(engine の 1 周あたりの時間)が悪化していないことを測り、報告に書く。`measure.mjs` の既存の窓で、再生中の engine の 1 周あたりの時間を、変更の前後で比べる(取り方は作業者が決め、報告に書く)。
4. 計測の道具と計測は、一時的な変更として入れ、受け入れの前に取り除く(製品に残さない)。
5. 受け入れを通す。通らなければ、到達した値(`bootLast60sAt10s`・`bootLast60sAt70s`・`maxRefillStallMs` の 3 回分)と、残る関数(上位 5 件。切り替え点を通らずに続いた最長の時間を添えて)を報告に書く。
6. 作業ツリーを commit する(受け入れが通れば push される)。

## 受け入れ

```accept
npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false?0:1)' "$RUNNER_TEMP/boot.json" && mkdir -p .tmp-112 && for i in 1 2 3; do node tools/p3-audio-probe/measure.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" --boot > ".tmp-112/boot-$i.json" || exit 1; done && node -e 'const fs=require("fs");const L=[1,2,3].map((i)=>JSON.parse(fs.readFileSync(".tmp-112/boot-"+i+".json","utf8").trim()));const px=(r)=>Math.floor(1836*r.dpr)+"x"+Math.floor(1056*r.dpr);process.exit(L.every((r)=>r.error===null&&r.state==="engine-idle"&&r.standIn===false&&r.ctxState==="running"&&r.quantaDelta>=19000&&r.bootLast60sAt10s===0&&r.bootLast60sAt70s===0&&typeof r.maxRefillStallMs==="number"&&r.maxRefillStallMs<=60&&r.screenDevicePx===px(r))?0:1)' && test -f pm-work/findings/boot-baseline-112.json && rm -rf .tmp-112
```

判定の基準(道具の 3 回すべて): `error` が null、`state` が engine-idle、`standIn` が false、`ctxState` が running、`quantaDelta` が 19,000 以上(窓の 60 秒で音声が動いていたことの確認)、`bootLast60sAt10s` と `bootLast60sAt70s` が 0、`maxRefillStallMs` が 60 以下(null でない)、`screenDevicePx` が、道具の `dpr` から `Math.floor(1836 × dpr)` × `Math.floor(1056 × dpr)` になる(アプリの `chooseGuestScreen`(`app/src/ui/screen-fit.ts`)と同じ切り捨て。3240x1863)。`pm-work/findings/boot-baseline-112.json` が在る。

0 にできなければ、到達した値と残る関数(上位 5 件)を報告に書く。その場合、受け入れは通らないので、判定は PM が見直す(「差し戻し」とは書かない)。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には「runner の値。この機械とは別枠」と書き、nproc と loadavg を添える。
- 実測と推定を分ける。「達成」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。出力の行を付ける。
- 報告に書く: 変更の前後の表(`bootLast60sAt10s`・`bootLast60sAt70s`・`maxRefillStallMs`・連続 underrun の最長。n=3)、特定した関数の表(上位 20 関数と、切り替え点を通らずに続いた最長の時間)、切り替え点の方式と理由、P3 の再生中の性能(再生中の engine の 1 周あたりの時間が変わったか)、変えたパス(`app/`・`lift/`・`tools/` を含む。`app/` と `lift/` を変えると main の deploy が動く)。
- 耳の確認は行わない。「音が途切れない」とは書かず、途切れ(underrun)の数で書く。

## 制約

- 製品に新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足さない。オプトインを作らない(計測の切り替えも含む)。
- 製品の既定の動作を、測定のためだけに変えない。
- 音声の先読み(`FLE_AUDIO_LEAD_MS`)は変えない(依頼者の決定の対象)。
- 関数名の表(`lift/out-app/gen/`)は配信の対象に入れない。
- ベンダー名と Cloudflare の識別子を書かない。
- sleep・timeout・nohup・末尾の `&` は使わない。一時ファイルは `.tmp-*` に置き、終わったら消す。
- 製品の作業は Actions で行う。この機械で製品のビルドや計測をしない。

## 他のタスクとの関係

- blocked_by: #{{040}}(起動直後の音声の補充の停止を測る道具と、関数名の表、コールバック内の補充の修正)。#{{040}} が閉じるまで起動しない。
- #{{109}}(実機の条件の段階的な測定。段 1〜3: マウス 1000 Hz と再生)と並行してよい。段 0(無音・無操作)は、この Issue が直す。同じ `fle_thread.c` を変える場合は、後に入る方が pull して直す。
- 描画の経路(`app/src/engine/client.ts`・`desk.ts`・`render-worker.ts`)は変えない。
