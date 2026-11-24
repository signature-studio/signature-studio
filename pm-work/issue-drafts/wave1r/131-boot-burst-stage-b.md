---
draft: "131"
title: "起動直後の途切れを 0 にする(段 B): 残りの 421 ms の区間(wasm の C++ 描画と C の関数)に切り替え点を入れ、補充の最大の間隔を窓の値にする"
type: Task
milestone: "M1 描画と音声の分離(P3)"
parent: "001"
blocked_by: []
labels: ["audio", "perf"]
size: L
priority: Urgent
project: signature-studio/3
---

## 目的

依頼者の指摘(2026-11-23): 起動して約 10 秒後、再生もマウス操作も無い状態で、ヘッダーの `直近 60 秒の途切れ` が 343(実機)。**目標は 0**。

段 A(#{{112}}。commit `0b746fe8` の作業、commit `eb118ef2`、結果は `pm-work/findings/boot-burst-1352.md`)で、受け入れの窓の途切れは 2,071 から 4・4・25 まで下がった(runner。n=3)。この段の目標は、残りを 0 にすること。

1. `bootLast60sAt10s` が n=3 の全てで 0。`bootLast60sAt70s` と `bootLast60sAt90s` も全てで 0。
2. 窓(idle の +10 秒から +90 秒)の、補充の最大の間隔 `maxRefillStallInWindowMs` が、全てで 60 ms 以下。起動からの最大の `maxRefillStallMs` は判定に使わない(段 A の定義の欠陥。下の背景)。
3. 再生中の性能を悪くせず、起動の時間を増やさない。

## 背景

- 段 A の結果(runner の値。nproc 4、loadavg 0.4〜1.7。実測。全文は `pm-work/findings/boot-burst-1352.md` の 1 節):

| 版 | 回 | bootLast60sAt10s | bootLast60sAt70s | bootLast60sAt90s | maxRefillStallMs (ms) | longestRunQuanta |
| --- | --- | --- | --- | --- | --- | --- |
| 基準(abadafd1) | 1 | 2,071 | 0 | 0 | 592.7 | 117 |
| 基準(abadafd1) | 2 | 2,105 | 0 | 0 | 592.1 | 124 |
| 基準(abadafd1) | 3 | 2,003 | 0 | 0 | 603.5 | 118 |
| 段 A(eb118ef2) | 1 | 4 | 0 | 0 | 539.9 | 4 |
| 段 A(eb118ef2) | 2 | 4 | 0 | 0 | 547.3 | 4 |
| 段 A(eb118ef2) | 3 | 25 | 0 | 0 | 549.8 | 25 |

- 段 A の受け入れのコマンドそのものの窓(3 回): `bootLast60sAt10s` は 0・1・4、`bootLast60sAt70s` は 0・0・0。
- 切り替えを通らずに続いた最長の区間(0〜90 s): 基準 1,102.2 ms → 段 A 421.0 ms。
- 段 A の上位 5 区間(長さ・開始・主な葉): 421.0 ms(開始 1,891 ms。`bl::Pipeline::Reference` 123 ms、`f_27ad5f8` 51 ms、`f_3370290` 37 ms)、374.0 ms(8,635 ms。`f_344fcf0` 126 ms、`f_27ad5f8` 74 ms、`fle_mx_addsd` 33 ms)、369.7 ms(5,375 ms。`f_344fcf0` 123 ms、`f_27ad5f8` 46 ms)、342.0 ms(7,364 ms。`f_344fcf0` 123 ms、`f_27ad5f8` 55 ms)、329.0 ms(3,410 ms。`f_27acd60` 72 ms、`f_344fcf0` 47 ms、`f_33d6e70` 28 ms)。
- **注意(実測の読み)**: 段 A の報告は「残りは C 以外(wasm の C++ 描画)」と読んだ。しかし、上位 5 区間のうち 4 件(2〜5 位)は、主な葉が C の関数(`f_344fcf0`・`f_27ad5f8`・`f_27acd60`。fe-lift の出力で、wasm に載っていない)である。区間 1 だけが C++ の描画。この段では、場所を決める前に、この食い違いを確かめる。
- 段 A 後の関数ごとの最長(切り替えを通らない連続の時間): `f_344fcf0` 53.4 ms、`f_27ad5f8` 93.5 ms、`f_27acd60` 34.1 ms、`f_33d6e70` 28.3 ms、`fle_hyb_dispatch` 198.3 ms。
- 切り替えの仕組み(コードの読み。`lift/runtime/fle_dispatch.h`・`fle_thread.c`・`fle_thread.h`):
  - 間接呼び出し: 4,096 回ごと(`FLE_PREEMPT_SLICE_DISPATCHES`)に 1 回の切り替え。仮想時計は 2,500 ticks(0.25 ms)進む(`FLE_PREEMPT_SLICE_TICKS`。決定 0147・0279)。仮想時計は間接呼び出しの数で進み、実時間とは別なので、間接呼び出しの少ない重い計算は、4,096 回に届くまで実時間で長く続きうる。
  - 後ろ向きの分岐(段 A で追加): 1,024 回ごと(`FLE_LOOP_SLICE_BACKEDGES`)に 1 回の切り替え。仮想時計は進めない。
  - 深さ 2 以上(ウィンドウ手続きの中など)では切り替えは飛ばされ、音声の補充だけ行う(`fle_audio_poll_in_callback`)。
- 仮想時計の罠(#1334 の試行、段 A の報告 3): ループの切り替えで仮想時計を進めると、起動が 139.7 秒かかった。計数の粒度を変えるときは、1 回あたりの仮想時間を比例して変える。例: 4,096 回・2,500 ticks と 1,024 回・625 ticks は、仮想時計の進む速さが同じ。
- 最大の補充の間隔の定義の欠陥: `maxRefillStallMs` は `fleAudioWorkerState().maxPubGapMs`(`lift/runtime/fle_audio_transport.c` の 27 行付近)で、起動からの最大。押下の前の間隔を含む(推定)。窓の値は無い。段 B で窓の値を足す。
- 段 A の性能(窓の +10〜+70 s。profile 付きの 1 回ずつ): 再生中の engine の 1 周あたりの busy は 88.2 µs(基準)→ 78.5 µs(段 A)。profile 無しの周あたりは未測定。
- Blend2D の C++(wasm。emcc でコンパイル)は、`lift/harness/b2d1/`(`blend2d-sources.txt`・`b2d_swap.cpp`・`b2d3-compop.patch`)と、`fe-build` の `blend2d_clone`(`lift/crates/fe-build/src/main.rs` の 171 行付近)で作られる。ゲストの Blend2D.dll の入口は、`b2d_swap.cpp` の `fle_swap_*` で差し替えている(決定 0160)。

## 読む物

- `pm-work/findings/boot-burst-1352.md`(全文。特に 2〜5 節)、`pm-work/findings/boot-baseline-112.json`、`pm-work/findings/boot-stall-1334.md`(計数の検証)
- `lift/runtime/fle_dispatch.h`(170〜200 行: 間接呼び出しの計数と `fle_loop_tick`)、`lift/runtime/fle_thread.h`(83〜94 行: 切り替えの規則と定数)、`lift/runtime/fle_thread.c`(`fle_progress_slice`・`fle_loop_slice` の 785〜820 行付近、`fle_audio_poll_in_callback` の呼び出し)、`lift/runtime/fle_dsound.c`(`audio_sync`)
- `lift/runtime/fle_audio_transport.c`(`fleAudioWorkerState`。27 行付近)、`app/src/audio/page-audio.ts`(`underrunQuantaLast60s`。ヘッダーの定義。**変えない**)
- `lift/harness/b2d1/`(`blend2d-sources.txt`・`b2d_swap.cpp`・`b2d3-compop.patch`)、`lift/crates/fe-build/src/main.rs`(`blend2d_clone`。パッチの当て方)、`lift/harness/web/cflags-b2d.args`
- `lift/runtime/fle_raster.c`(`fle_r_blit`)、`lift/runtime/fle_gdi.c`(`EACH_CLIP` の呼び出し)
- `lift/crates/fe-wasmgen`(hybrid の対象。段 A の結論どおり、生成器の入口・ループへの切り替えは C の関数に届かない)
- `tools/p3-audio-probe/measure.mjs`(`--boot`・`--profile`・`maxPubGapMs` の読み)、`tools/p3-audio-probe/burst-report.mjs`・`profile-names.mjs`(段 A で追加・既存)、`pm-work/findings/measure-live-109.patch`(#1349 の道具の差分。main に無ければ使う)
- `tools/smoke/welcome-flow.mjs`(流れ A の確認)、`tools/boot-check/boot-bundle.mjs`(`elapsedMs` は engine-idle までの時間)

## やること

0. 作業の最初に `date -u` を記録する。4 時間 30 分を過ぎたら、受け入れが通っていなくても、作業ツリーを commit し、報告を書いて終える(ジョブの上限は 5 時間 50 分)。
1. **計測の道具を足す(製品の既定の動作は変えない)**
   - `measure.mjs --boot` の JSON に `maxRefillStallInWindowMs` を足す。窓は idle の +10 秒から +90 秒。
   - 方式は作業者が決め、報告に書く。例: `fleAudioWorkerState` の近くに、読むだけの関数を足す(窓の最大を返して 0 に戻す。最後の公開の時刻は消さない。+10 秒で読んで捨て、+90 秒で読む)。
   - 起動からの `maxRefillStallMs` は残す。
   - 環境変数・`#ifdef`・URL パラメータ・ビルド変数は足さない。ヘッダーの表示(`page-audio.ts`)は変えない。
2. **区間の特定(切り替え点の入る場所を決める)**
   - 1 回目のビルド(段 A の版の上)で、`measure.mjs --boot --profile FILE` を取り、`burst-report.mjs` で、切り替えを通らない最長の区間の上位 10 件を、関数名・スレッド・鎖で出す。
   - 区間 1(`bl::Pipeline::Reference` の鎖)と、区間 2〜5(C の `f_344fcf0`・`f_27ad5f8`・`f_27acd60`)のそれぞれについて、長い理由を確かめる。例: 後ろ向きの分岐が在るか(生成された C。`lift/out-app/gen` の該当の関数)、間接呼び出しの数はどれだけか、切り替えが深さ 2 以上で飛ばされていないか。
   - 結果は、実測と推定を分けて報告に書く。
3. **切り替え点を入れる(場所は 2 の結果で決める。全関数には入れない)**
   - (a) Blend2D の C++ の、上位の区間の行・span のループに、切り替え点を入れる。`lift/harness/b2d1/` に新しいパッチを足し、`fe-build` の `blend2d_clone` で当てる。C++ からは `fle_loop_tick()` の包み(extern "C")を呼ぶ形でよい。または
   - (b) ゲストから Blend2D へ入る点(`fle_swap_*`)で、描画の 1 回の呼び出しを帯(バンド)に分け、帯ごとに切り替え点を通す。
   - (c) 区間 2〜5 の C の関数で、切り替えが届かない理由が確かめられたら、その形に限って切り替え点を足す。計数の粒度を変えるときは、仮想時計の進む速さを変えない(4,096 回・2,500 ticks を 1,024 回・625 ticks にする等)。決定 0147・0279 との関係を報告に書く。
   - 切り替えの後で、状態を読み直す必要があるか(段 A の報告 10。`mx_guard_refresh` のような既存の処理)を確かめる。
   - 音声を作るゲストのスレッドが、締め切り(先読み 60 ms。`FLE_AUDIO_LEAD_MS` は変えない)の前に必ず走ることを保つ。
   - 仮想時計(`fle_sys_clock_advance_to`)を、後ろ向きの分岐や描画の切り替えで進めない。
   - 再生中の性能への影響を、4 の計測で確かめる。
4. **ビルドは 3 回まで**(1 回約 35 分)。1 回目: 1 の計測の道具と、段 A の版の上の profile。2 回目: 3 の切り替え点。3 回目: 2 回目の結果に基づく修正の 1 回だけ。
5. **受け入れの測定**: 下の受け入れのコマンドで、`measure.mjs --boot` を n=3 で行い、判定の基準の全てを満たすか確かめる。満たせないときは、到達した値と、残る区間の上位 5 件(関数名・鎖・長さ)を報告に書く(判定は PM が見直す)。
6. **再生中の性能**: 無音・再生・マウス 1000 Hz の途切れ(`measure-live.mjs`。#1349 の道具。main に無ければ `pm-work/findings/measure-live-109.patch` を `git apply` で当てて使う。当たらなければ `measure.mjs --load ui`(50 ms 間隔)で代え、1000 Hz は未確認と書く)が 0 のまま。engine の 1 周あたりの時間(profile 無しの測定で。段 A の版は 78.5 µs)を、基準と比べて報告に書く。
7. 計測のための変更は、受け入れの前に取り除く(読むだけの関数は残してよい)。
8. 作業ツリーを commit する(受け入れが通らなくても、作業の最後に必ず commit する)。

## 受け入れ

```accept
npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false&&r.elapsedMs<=12500?0:1)' "$RUNNER_TEMP/boot.json" && test -s pm-work/findings/boot-stage-b-131.md && node tools/smoke/welcome-flow.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/flow.jsonl" && node -e 'const fs=require("fs");const L=fs.readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const A=L.find((r)=>r.flow==="A");process.exit(A&&A.error===null&&A.state==="engine-idle"&&A.standIn===false&&A.trapped===false&&A.hostErrors===0&&A.opened===true?0:1)' "$RUNNER_TEMP/flow.jsonl" && mkdir -p .tmp-131 && for i in 1 2 3; do node tools/p3-audio-probe/measure.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" --boot > ".tmp-131/boot-$i.json" || exit 1; done && node -e 'const fs=require("fs");const L=[1,2,3].map((i)=>JSON.parse(fs.readFileSync(".tmp-131/boot-"+i+".json","utf8").trim()));const px=(r)=>{const [w,h]=String(r.screenDevicePx).split("x").map(Number);return Math.abs(w-Math.round(1836*r.dpr))<=1&&Math.abs(h-Math.round(1056*r.dpr))<=1;};process.exit(L.length===3&&L.every((r)=>r.error===null&&r.state==="engine-idle"&&r.standIn===false&&r.ctxState==="running"&&r.quantaDelta>=19000&&r.bootLast60sAt10s===0&&r.bootLast60sAt70s===0&&r.bootLast60sAt90s===0&&typeof r.maxRefillStallInWindowMs==="number"&&r.maxRefillStallInWindowMs<=60&&px(r))?0:1)' && rm -rf .tmp-131
```

判定の基準(受け入れのコマンドと同じ。道具の出力で確かめる):
- `boot.json`: `error` が null、`state` が engine-idle、`standIn` が false、`elapsedMs`(engine-idle までの時間)が 12,500 以下。
- `pm-work/findings/boot-stage-b-131.md` が在る(この段の報告の原文。表と区間と読みを書く)。
- 流れ A(`tools/smoke/welcome-flow.mjs`): `error` が null、`state` が engine-idle、`standIn` が false、`trapped` が false、`hostErrors` が 0、`opened` が true。
- `measure.mjs --boot` の 3 回の全て: `error` が null、`state` が engine-idle、`standIn` が false、`ctxState` が running、`quantaDelta` が 19,000 以上、`bootLast60sAt10s`・`bootLast60sAt70s`・`bootLast60sAt90s` が 0、`maxRefillStallInWindowMs` が 60 以下、`screenDevicePx` の幅と高さが `Math.round(1836 × dpr)` と `Math.round(1056 × dpr)` の ±1 以内。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には「runner の値。この機械とは別枠」と書き、nproc と loadavg を添える。
- 実測と推定を分ける。「達成」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。出力の行を付ける。
- 報告に書く: 基準・段 A・段 B の表(`bootLast60sAt10s`・`bootLast60sAt70s`・`bootLast60sAt90s`・`maxRefillStallMs`・`maxRefillStallInWindowMs`・`longestRunQuanta`。n=3)、切り替えを通らない最長の区間の上位 10 件(段 B の版の上で)、関数の表、区間 1 と区間 2〜5 の食い違いを確かめた結果、切り替え点の場所・方式・理由、計数の粒度を変えた場合の仮想時計の扱い、再生中の性能(無音・再生・1000 Hz の途切れと、1 周あたりの時間)、`elapsedMs` の値、変えたパス(`app/`・`lift/`・`tools/` を含む。`app/` と `lift/` を変えると main の deploy が動く)。
- 耳の確認は行わない。「音が途切れない」とは書かず、途切れ(underrun)の数で書く。

## 制約

- 製品に新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足さない。オプトインを作らない(計測の切り替えも含む)。
- 製品の既定の動作を、測定のためだけに変えない。
- 音声の先読み(`FLE_AUDIO_LEAD_MS`)は変えない(依頼者の決定の対象)。
- `page-audio.ts` のヘッダーの定義は変えない。
- 関数名の表(`lift/out-app/gen/`)は配信の対象に入れない。
- ベンダー名と Cloudflare の識別子を書かない。
- sleep・timeout・nohup・末尾の `&` は使わない。一時ファイルは `.tmp-*` に置き、終わったら消す。
- 製品の作業は Actions で行う。この機械で製品のビルドや計測をしない。

## 作業の進め方

- ビルドは 1 回約 35 分。フォアグラウンドで、シェルツールの `timeout` 引数に大きい値(例 5400000。上限)を渡して待つ。
- blend2d の取得(GitHub)が 403 で落ちたら一時的。受け入れのコマンドは `lift/out-app/b2d-src` を消してから作り直すので、そのまま再試行する。
- 作業の最後に、受け入れが通らなくても、作業ツリーを commit する。未 commit の変更が残ると、判定は失敗する。

## 他のタスクとの関係

- blocked_by: なし。#{{112}}(段 A)は閉じている。
- #{{109}}(実機の条件の段階的な測定。段 1〜3: マウス 1000 Hz と再生)と並行してよい。`fle_thread.c`・`fle_dispatch.h`・`fle_thread.h` を変える場合は、後に入る方が pull して直す。
- 描画の経路(`app/src/engine/client.ts`・`desk.ts`・`render-worker.ts`)は変えない。
- 起動の時間(`elapsedMs`)は、段 A の版の値を記録してから比べる。12,500 を超えたら、どの変更で増えたかを報告に書く。
