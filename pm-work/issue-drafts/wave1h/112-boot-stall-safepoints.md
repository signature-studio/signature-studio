---
draft: "112"
title: "起動直後の途切れの内訳を特定し、起動の最中の長い計算に切り替え点の第 1 手を入れる(段 A。途切れを 0 にするのは段 B)"
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

この Issue は**段 A**(段階に分けた 1 段目)。段 A の目標は次の 2 つ。
1. 起動直後の途切れの内訳を、関数・スレッド・連続の長さで特定し、表にする(`pm-work/findings/boot-burst-1352.md`)。
2. その特定に基づき、切り替え点の第 1 手を入れ、`bootLast60sAt10s` を変更の前の値の 70% 以下にする(n=3)。

起動の途切れを 0 にすること(`bootLast60sAt10s` と `bootLast60sAt70s` が 0。`maxRefillStallMs` が 60 以下)は段 B の目標。この Issue では判定に入れない。段 B は、段 A の結果を見て PM が起票する。

## 背景

- 測定の前提(#{{040}} が入れた道具): `tools/p3-audio-probe/measure.mjs --boot`(起動の窓。idle の +10 秒と +70 秒に読む。`--profile FILE` で CDP の CPU profile を取る)、`tools/p3-audio-probe/profile-names.mjs`(wasm-function[N] を名前にする)、関数名の表(`lift/out-app/gen/wasm-func-names.tsv`。製品のビルドで書き出す。配信の対象に入れない)、`pm-work/findings/boot-stall-1334.md`(#{{040}} の 3 回の試行の実測と、計数の検証の結果)。
- #{{040}} の 3 回の試行(runner の値。n=3): `bootLast60sAt70s` は 3 回とも 0。`bootLast60sAt10s` は 878・1,112・978(目標 0)。`maxRefillStallMs` は 722.0・710.1・700.9 ms(目標 60 以下)。連続 underrun の最長は 63・69・60 quanta。
- #{{040}} の報告(PM の記録): 関数は `f_344fcf0`(467 ms)・`f_27ad5f8`(160 ms)・`fle_mx_addsd`(SSE の演算の呼び出し)が上位。根は `fle_wasm_thread_entry`(guest のスレッド)。
- 起動直後: メインのスレッドが `DispatchMessageW → call_wndproc` の中に 367 ms 連続で在った(idle の前。約 8.5 秒まで。推定: 起動直後の連続の由来)。コールバック内では切り替えが飛ばされ(`lift/runtime/fle_thread.c` の `fle_progress_slice`。深さが `FLE_PREEMPT_MAX_DEPTH` を超えたとき)、その間は音声の補充も止まる。#{{040}} で、コールバック内の切り替えの飛ばしのとき、外部の音声の消費者があるかぎり音声の補充を行うようにした(`fle_audio_poll_in_callback`)。それでも 0 にはならない。
- `FLE_PREEMPT_MAX_DEPTH` を 1 → 2 にした実験(1 回): `bootLast60sAt10s` 959 → 764 だが、`maxRefillStallMs` 681 → 1,215 ms に悪化。ウィンドウ手続きの中で切り替えを許すだけでは、停止は減らない。原因は未確認。
- 切り替えの計数は、間接呼び出しでしか進まない(`lift/runtime/fle_dispatch.h` の `++fle_progress_count`)。スライスは 4,096 回の間接呼び出しごと(`lift/runtime/fle_thread.h` の `FLE_PREEMPT_SLICE_DISPATCHES`)。したがって、間接呼び出しの無い長いループは、切り替えを通らずに走りうる(コードの読み。前回の試行で確かめた)。
- 最近の測定(runner。今の HEAD): 起動直後の `bootLast60sAt10s` は 2,294〜2,315(前回の試行の基準の範囲)、無操作の段 0 の +10 秒は 2,475〜2,569(#{{109}} の試行の 9 回)。#{{040}} の時点より大きい。HEAD が進んで増えた(原因は未確認)。
- 物理画素の画面(`#{{107}}` の後)では、起動直後の山が約 13.8 秒まで続く(runner の実測)。目標(0)は変えない。

## 前回の試行で分かったこと

run 45253143948(2026-11-23 19:52Z 起動)の作業。時間切れ(5 時間 50 分。`cancelled`)で、作業は失われた(push も差分も無い)。報告の原文は `pm-work/findings/boot-stall-112-attempt1.md`。runner の値(nproc 4)。

1. 時間の使い方: 1 回のエンジンのビルドは約 46 分(HEAD)。前回は、基準・wasm 側の試し 3 回・C 側の作りかけで、ビルドを 5 回ほど回して時間が尽きた。段 A は**ビルドを 3 回以内**に抑える(基準の版、第 1 手の版、必要なら修正の 1 回)。
2. 基準(HEAD。作業ツリーは clean): `bootLast60sAt10s` 2,294〜2,315。engine-idle までの起動は約 12.3 秒。
3. 罠(wasm 側の v1): ループ先頭の切り替えの計数を、間接呼び出しと同じ `fle_progress_count` に入れると、切り替えのたびに仮想時計が 0.25 ms 進む。ループは桁違いに多いので、engine-idle まで 139.7 秒かかった。**ループ用の計数は別に持ち、仮想時計を進めない**。
4. v2(wasm 側のループ先頭。ループ用の計数と上限を別に持つ。仮想時計は進めない。4,096 単位): 起動 11.2 秒、`bootLast60sAt10s` 2,018〜2,084(基準の 88〜90%)。0 にはならない。再生中の busy の割合は基準 4.0%、v2 3.6%(無音・無操作の窓。再生ではない)。
5. v3(ループ用の粒度を 512 に細かくした): 補充の間隔は約 36〜45 ms に 1 回で、v2 と変わらなかった。粒度は原因ではない。
6. **停止の主因は wasm の外**: `f_344fcf0` と `f_27ad5f8` は、`wasm-func-names.tsv` で「not selected for the new backend (hybrid)」(wasm に載せていない)。fe-lift が出した C のコード。wasm 生成器に入れた切り替え点は、これらに届かない。
7. 補充の間隔: 補充は平均 49 ms に 1 回。先読みは 60 ms(2,646 frames)なので、55 ms 前後で使い切る。0.6〜14 秒の間に underrun が 3,048 quanta(v2 のトレース)。worklet 側では、1〜14 秒で約 70% が underrun。
8. 次に試す(前回は途中で止まった。未確認): fe-lift の C 出力の**後ろ向きの `goto`**(アドレスが現在の命令以下の分岐。ループの後方辺)の前に、ループ用の切り替えを置く。切り替えは、`fle_dispatch.h` の既存の切り替え(間接呼び出しの箇所)と同じ呼び出し規則で行い、計数は別に持つ(項 3)。前回は fe-lift の出力部を直し、`FLE_LOOP_TICK();` を前置する形まで作った。
9. fe-lift のテスト: HEAD でも失敗 55 件(前回の報告の値。内訳: authenticode 9・int_more 4・int_more2 1・noreturn_audit 5・realmod 16 など)。この変更の前後で同じ件数だった。段 A の受け入れには入れない。増えないことを確かめる。
10. 切り替え後のグローバルの扱い: wasm 側では、切り替えの後に `mx_guard_refresh()`(既存)を呼ぶ必要があった。C 側でも、切り替えの後に同じ状態を読み直す箇所が要るかを、既存の間接呼び出しの箇所と比べて確かめる。

## 読む物

- `pm-work/findings/boot-stall-112-attempt1.md`(前回の報告の原文)、`pm-work/findings/boot-stall-1334.md`(#{{040}} の 3 回の試行。関数の表。計数の検証)
- `lift/runtime/fle_thread.c`(`fle_progress_slice` のコールバックの分岐と、`fle_audio_poll_in_callback` の呼び出し)、`lift/runtime/fle_thread.h`(協調のスレッド。切り替えの規則・`FLE_PREEMPT_SLICE_DISPATCHES`・`FLE_PREEMPT_MAX_DEPTH`)、`lift/runtime/fle_dispatch.h`(切り替えの計数。177 行付近)
- `lift/crates/fe-lift`(fe-lift の C 出力。後ろ向きの `goto` を出す箇所)、`lift/crates/fe-wasmgen`(wasm 側。hybrid の対象の判定)
- `lift/runtime/fle_dsound.c`(`audio_sync`・`fle_audio_poll_in_callback`)、`lift/runtime/fle_audio_transport.c`(`fle_js_publish`・`fleAudioWorkerState`)、`lift/runtime/fle_audio_ring.h`(`FLE_AUDIO_LEAD_MS`)、`lift/runtime/fle_audio_ring.c`
- `lift/harness/fruity_idle.c`(`fle_thread_idle_hook`: アイドルの待ち)
- `app/src/audio/page-audio.ts`(`detail()`・`underrunQuantaLast60s`)、`app/src/audio/fle-out.worklet.js`、`app/src/audio/status.ts`
- `app/src/engine/idle-driver.ts`(`S-clk-maxlag`)、`app/src/engine/worker.ts`
- `tools/p3-audio-probe/measure.mjs`(`--boot`・`--profile`)、`tools/p3-audio-probe/profile-names.mjs`、`tools/boot-check/boot-bundle.mjs`・`cdp.mjs`・`stage-assets.mjs`

## やること

0. 作業の最初に `date -u` を記録する。
1. **基準を取る(変更の前)**: 作業ツリーを変える前の HEAD でエンジンをビルドし、`tools/p3-audio-probe/measure.mjs <bundle> --boot` を n=3 で行う。3 回の JSON を `pm-work/findings/boot-baseline-112.json` に次の形で書いて commit する: `{"head":"<SHA>","runs":[{…},{…},{…}]}`(各 run は道具の JSON 1 行。`longestRunQuanta`(連続 underrun の最長)を含む)。
2. **内訳を特定する(段 A の本体)**: `measure.mjs --boot --profile FILE` の CDP の CPU profile と、`profile-names.mjs` の関数名で、engine の Worker とメインのスレッドの自己時間を、関数別・スレッド別に出す。切り替え点を通らずに続いた区間(呼び出しの鎖に切り替えの関数が無い連続の時間)を、長い順に 10 件出す。結果を `pm-work/findings/boot-burst-1352.md` に書く。
   - (a) 上位 20 関数の表(列: 関数名・スレッド・自己時間 ms・切り替え点を通らずに続いた最長 ms・wasm か C か(`wasm-func-names.tsv` の印))。
   - (b) 最長の連続区間 10 件(開始からの ms・長さ・根の関数・鎖の関数)。
   - (c) 取り方(道具の版・プロファイルの時間窓・スレッドの分け方)。
3. **第 1 手**: 表で上位の 1〜2 関数(#{{040}} の報告の `f_344fcf0`・`f_27ad5f8`、または表の上位)の後ろ向きの分岐に、ループ用の切り替え点を入れる。方式は前回の結論(「前回の試行で分かったこと」の 3〜10)に従う。計数は間接呼び出しと別に持ち、仮想時計を進めない。対象は生成器の入力で絞る。全関数に入れる場合は、再生中の性能への影響を報告する。
   - 音声を作る guest のスレッドが、締め切り(先読み 60 ms。この Issue では変えない)の前に必ず走ることを保つ。
   - 製品に新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足さない。既定の動作として直す。
4. 変更の後に、`measure.mjs --boot` を n=3 で行い、`bootLast60sAt10s` を記録する。
5. 計測のための変更は、受け入れの前に取り除く(製品の本体に計測の切り替えを残さない。道具は `tools/` に残してよい)。
6. 再生中の性能: `measure.mjs` の既存の窓で、再生中の engine の 1 周あたりの時間を、変更の前後で比べる(取り方は作業者が決め、報告に書く)。
7. 作業ツリーを commit する(受け入れが通らなくても、作業の最後に必ず commit する)。

## 受け入れ

```accept
npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false?0:1)' "$RUNNER_TEMP/boot.json" && test -s pm-work/findings/boot-burst-1352.md && test -s pm-work/findings/boot-baseline-112.json && mkdir -p .tmp-112 && for i in 1 2 3; do node tools/p3-audio-probe/measure.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" --boot > ".tmp-112/boot-$i.json" || exit 1; done && node -e 'const fs=require("fs");const L=[1,2,3].map((i)=>JSON.parse(fs.readFileSync(".tmp-112/boot-"+i+".json","utf8").trim()));const B=JSON.parse(fs.readFileSync("pm-work/findings/boot-baseline-112.json","utf8")).runs;const base=B.map((r)=>r.bootLast60sAt10s).sort((a,b)=>a-b);const med=base[1];const rows=fs.readFileSync("pm-work/findings/boot-burst-1352.md","utf8").split("\n").filter((l)=>l.startsWith("|")).length;const px=(r)=>{const [w,h]=r.screenDevicePx.split("x").map(Number);return Math.abs(w-Math.round(1836*r.dpr))<=1&&Math.abs(h-Math.round(1056*r.dpr))<=1;};process.exit(B.length===3&&base.length===3&&rows>=22&&L.every((r)=>r.error===null&&r.state==="engine-idle"&&r.standIn===false&&r.ctxState==="running"&&r.quantaDelta>=19000&&typeof r.bootLast60sAt10s==="number"&&r.bootLast60sAt10s<=0.7*med&&px(r))?0:1)' && rm -rf .tmp-112
```

判定の基準(道具の 3 回すべて):
- `error` が null、`state` が engine-idle、`standIn` が false、`ctxState` が running、`quantaDelta` が 19,000 以上(窓の 60 秒で音声が動いていたことの確認)。
- `screenDevicePx` の幅と高さが、道具の `dpr` から求めた `Math.round(1836 × dpr)` と `Math.round(1056 × dpr)` の ±1 以内(既定の DPR では 3240x1864)。
- `bootLast60sAt10s` が、`pm-work/findings/boot-baseline-112.json` の 3 回の中央値の **70% 以下**。
- `pm-work/findings/boot-burst-1352.md` に、表の行が 20 件以上ある(前提: 上位 20 関数の表)。`pm-work/findings/boot-baseline-112.json` が 3 回分ある。

`bootLast60sAt70s`・`maxRefillStallMs` は判定に入れない(段 B)。報告には書く。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には「runner の値。この機械とは別枠」と書き、nproc と loadavg を添える。
- 実測と推定を分ける。「達成」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。出力の行を付ける。
- 報告に書く: 基準と変更後の表(`bootLast60sAt10s`・`bootLast60sAt70s`・`maxRefillStallMs`・`longestRunQuanta`。n=3)、特定した関数の表(上位 20 関数と、切り替え点を通らずに続いた最長の時間)、切り替え点の場所と方式と理由、再生中の性能(engine の 1 周あたりの時間が変わったか)、変えたパス(`app/`・`lift/`・`tools/` を含む。`app/` と `lift/` を変えると main の deploy が動く)。
- 耳の確認は行わない。「音が途切れない」とは書かず、途切れ(underrun)の数で書く。

## 制約

- 製品に新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足さない。オプトインを作らない(計測の切り替えも含む)。
- 製品の既定の動作を、測定のためだけに変えない。
- 音声の先読み(`FLE_AUDIO_LEAD_MS`)は変えない(依頼者の決定の対象)。
- 関数名の表(`lift/out-app/gen/`)は配信の対象に入れない。
- ベンダー名と Cloudflare の識別子を書かない。
- sleep・timeout・nohup・末尾の `&` は使わない。一時ファイルは `.tmp-*` に置き、終わったら消す。
- 製品の作業は Actions で行う。この機械で製品のビルドや計測をしない。

## 作業の進め方

- 作業の最初に `date -u` を記録する。4 時間 30 分を過ぎたら、受け入れが通っていなくても、作業ツリーを commit し、報告を書いて終える(ジョブの上限は 5 時間 50 分)。
- 受け入れが通らなくても、作業の最後に作業ツリーを commit する。未 commit の変更が残ると、判定は失敗する。
- エンジンのビルドは 1 回約 46 分。ビルドは 3 回以内に抑える(前回は 5 回ほどで時間が尽きた)。プロファイルと測定は、同じビルドで行う。

## 他のタスクとの関係

- blocked_by: #{{040}}(起動直後の音声の補充の停止を測る道具と、関数名の表、コールバック内の補充の修正)。#{{040}} は閉じている。
- 段 B(起動の途切れを 0 にする。`bootLast60sAt10s`・`bootLast60sAt70s` が 0 と、`maxRefillStallMs` が 60 以下)は、段 A の結果を見て PM が起票する。この Issue では、段 B の判定を入れない。
- #{{109}}(実機の条件の段階的な測定。段 1〜3: マウス 1000 Hz と再生)と並行してよい。段 0(無音・無操作)は、この Issue と段 B が直す。同じ `fle_thread.c` を変える場合は、後に入る方が pull して直す。
- 描画の経路(`app/src/engine/client.ts`・`desk.ts`・`render-worker.ts`)は変えない。
