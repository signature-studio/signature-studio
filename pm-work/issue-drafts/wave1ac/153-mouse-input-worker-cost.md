---
draft: "153"
title: "マウス入力(1000 Hz 以上)の場面で Worker の時間の 44.5% がブラウザ内部、27.2% がアプリの JS(postMessage・decode・setTimeout): 内訳を特定し、入力 1 件あたりの費用を減らす"
type: Task
milestone: "M1 描画と音声の分離(P3)"
parent: "001"
blocked_by: []
labels: ["perf", "audio"]
size: L
priority: High
project: signature-studio/3
---

## 目的

マウスを 1000 Hz を超えて動かす場面(計測の場面 (iii))で、エンジンの Worker が忙しい時間の 71.7% は、入力の受け渡しに使われている。内訳は、ブラウザ内部(区分 F)44.5%、アプリの JS と組み込み関数(区分 E)27.2%。エンジンの計算(区分 B と D)は 24.8% にすぎない。この Issue で、内訳を特定し、入力 1 件あたりの費用を減らす。

## 背景

基準(`pm-work/findings/engine-cost-attribution.json`。#1388 の段階 0。製品の `-Oz` の engine。場面 (iii) の 3 run の平均):

| DPR | busyMs(Worker の忙しい時間) | 区分 E | 区分 F | E+F |
| --- | --- | --- | --- | --- |
| 1.00 | 12,387.9 ms | 3,371.4 ms | 5,506.8 ms | 8,878.2 ms |
| 1.325 | 12,202.0 ms | 3,407.2 ms | 5,385.4 ms | 8,792.6 ms |

- 区分の定義(`tools/engine-cost/measure.mjs` の冒頭): E はアプリの JS(127.0.0.1 の URL の関数)と、URL の無い組み込み関数(`postMessage`・`setTimeout`・`now`・`getElementById`・V8 の境界フレーム `wasm-to-js`・`js-to-wasm`)。F は `(program)`・`(garbage collector)`・`(root)`。
- 同じ道具の run どうしで、busyMs は最大 7% 動く(DPR 1.00 の 3 run は 11,927〜12,835 ms)。判定が 75% の境界に近いときは、この揺れを頭に置く。
- 入力の経路(今の製品。コードを読んで確かめた。計測で裏付ける):
  - `app/src/engine/client.ts` の `sendInput`: 入力は共有メモリのリング(`app/src/input/ring.ts`。SharedArrayBuffer。1024 件)に書く。Worker が parked(エンジンが idle で待っている)の時だけ、`wake` を `postMessage` で送る。リングに入らない時は、`input` を 1 件ずつ `postMessage` する(`ringStats.viaMessage`)。
  - Worker 側: `lift/harness/fruity_idle.c` の `fle_input_ring_poll` が、エンジンのスケジューラの点で、リングを読む。Worker の待ちは `app/src/engine/worker.ts` の `setTimer`(`setTimeout`)と MessageChannel の往復。
- 仮説(未確認。計測で確かめる): (a) E の `postMessage` は、parked の時の 1 件ごとの `wake` と、その受信(場面 (iii) は 1200 Hz で送る)。(b) E の `setTimeout` は、idle の待ちの最小の 4 ms の丸め。(c) F の `(program)` は、Atomics.wait の待ち・`postMessage` の構造化複製・wasm から JS への呼び出しの橋・GC のどれか。(d) E の `decode` が何の decode か。

## 読む物

- `pm-work/findings/engine-cost-attribution.md`・`.json`(基準)
- `tools/engine-cost/measure.mjs`(場面 (iii) の作り方と区分の規則)、`tools/engine-cost/run.sh`
- `app/src/engine/client.ts`(`sendInput`・`ringStats`・`wake`・`input`)、`app/src/input/ring.ts`、`app/src/input/events.ts`・`pointer.ts`・`tracker.ts`
- `app/src/engine/worker.ts`(`setTimer`・`wake`・`input` の受け口)、`app/src/engine/idle-driver.ts`
- `lift/harness/fruity_idle.c`(`fle_input_ring_poll`)、`lift/harness/web/fruity/fle_idle_lib.js`、`lift/runtime/fle_thread.c`
- `tools/p3-audio-probe/measure-live.mjs`(#1349 の測定。`--hz`・`--mouse`・`--play`)、`tools/p2-frames/measure.mjs`(#1350。`inputToPresentMsP95`)、`tools/feat-c1-hidpi/measure-size.mjs`(#1347。`pointerErrPx`)、`pm-work/findings/p2-frames-1350.md`
- `tools/boot-check/cdp.mjs`(CDP の部品)、`tools/smoke/welcome-flow.mjs`(流れ A〜D)

## やること

1. 内訳の特定。変える前の engine で行う。
   - CDP の `Tracing.start`(categories: `devtools.timeline,v8,v8.wasm,disabled-by-default-v8.cpu_profiler,toplevel`)で、場面 (iii) を 20 秒トレースし、区分 F の `(program)` の中身を、イベントの名前ごとの自己時間の表にする。
   - 区分 E の `postMessage`・`decode`・`setTimeout` が、どのコード(`app/src/...`・`lift/...`)の、どの呼び出しかを、行番号つきで特定する。`decode` が何の decode かも書く。
   - 結果を `pm-work/findings/input-cost-153.md` に書く(表と行番号)。入力 1 件あたりの費用(Worker の時間 ÷ 入力の件数)も書く。
   - トレースの道具は `tools/engine-cost/` に足してよい。
2. 減らす。製品の既定の動作として直す。オプトインを作らない(環境変数・`#ifdef`・URL パラメータ・ビルド変数を足さない)。
   - 入力 1 件ごとの `wake` の `postMessage` をまとめる。候補: (i) Worker を、idle の時、リングの head への `Atomics.wait`(タイムアウト付き)で待たせ、`wake` の message を無くす。(ii) page 側で、parked の時の `wake` を 1 ミリ秒に 1 回、または 1 フレームに 1 回にまとめる。どちらを選ぶかは内訳の結果で決め、理由を書く。
   - `setTimeout` の 4 ms の丸めに依存しない待ちにする。
   - 入力の届き方と遅れを悪くしない。入力は毎秒 1000 件以上届く(`measure-live.mjs` の `inputRateOk`)。入力の遅れ(`inputToPresentMsP95`)を悪くしない。ポインタの位置の誤差(`pointerErrPx`)は 1 px 以下にする。
3. 計測の道具。`tools/engine-cost/run.sh` に `--scene <i|ii|iii>` を足す。指定した場面の run だけを行い、集計の段(`aggregate`)は行わない(`pm-work/findings/engine-cost-attribution.*` を書き換えない)。既定の動作は今のままにする。

## 判定の基準

下の受け入れのコマンドが、次を全て確かめる。
- 場面 (iii) の 6 run(DPR 1.00 と 1.325。各 n=3)が全て `error` が null。
- DPR ごとに、区分 E+F の平均が、基準(上の表)の 60% 以下。
- DPR ごとに、busyMs の平均が、基準の 75% 以下。
- `measure-live.mjs`(入力 1000 Hz・再生中): `error` が null、`playing` が true、`underrunDelta` と `last60sUnderruns` が 0、`inputRateOk` が true。
- `pointerErrPx`: `measure-live.mjs` の出力には無い。`tools/feat-c1-hidpi/measure-size.mjs`(DPR 1)の出力の `pointerErrPx` が、数値の行が 1 件以上あり、全て 1 以下。
- `inputToPresentMsP95`(`tools/p2-frames/measure.mjs`、DPR 1、負荷 a): `error` が null、`inputRateOk` が true、値が 219 ms 以下。基準は `pm-work/findings/p2-frames-1350.md` の DPR 1・負荷 a の 3 回の中央値 182.7 ms の 1.2 倍。1 回の値は 82〜185 ms に散るので、219 ms を超えたら 3 回測り、3 回の値を報告に書く。
- 流れ A〜D(`tools/smoke/welcome-flow.mjs`): 4 行が全て、`error` が null・`state` が engine-idle・`standIn` が false・`trapped` が false・`hostErrors` が 0・`opened` が true。流れ B と D は `welcomeClosed` が true。各行の `elapsedMs` が 12,500 以下、`preBootBytes` が 314,368,711 B 以下。

基準は `pm-work/findings/engine-cost-attribution.json` の値を使う。判定が境界に近いときは、変更前の engine を同じ runner で測り直し、両方の値を報告に書く。

## 受け入れ

```accept
npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && rm -rf lift/out-app/engine-cost/runs && bash tools/engine-cost/run.sh --scene iii && node -e 'const fs=require("fs");const b=require("./pm-work/findings/engine-cost-attribution.json");const mean=(a)=>a.reduce((x,y)=>x+y,0)/a.length;const ef=(r)=>r.classesMs.E+r.classesMs.F;let ok=true;for(const [d,k] of [["1.00",1],["1.325",1.325]]){const now=[1,2,3].map((n)=>JSON.parse(fs.readFileSync(`lift/out-app/engine-cost/runs/Oz/iii-dpr${d}-n${n}.json`,"utf8")));const base=b.runs.filter((r)=>r.scene==="iii"&&r.dpr===k);if(!now.every((r)=>r.error===null)||mean(now.map(ef))>0.6*mean(base.map(ef))||mean(now.map((r)=>r.busyMs))>0.75*mean(base.map((r)=>r.busyMs)))ok=false;}process.exit(ok?0:1)' && node tools/p3-audio-probe/measure-live.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" --play 1 --hz 1000 --mouse 60 > .tmp-153-live.json && node -e 'const r=JSON.parse(require("fs").readFileSync(".tmp-153-live.json","utf8"));process.exit(r.error===null&&r.playing===true&&r.underrunDelta===0&&r.last60sUnderruns===0&&r.inputRateOk===true?0:1)' && node tools/feat-c1-hidpi/measure-size.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" --dprs 1 > .tmp-153-size.jsonl && node -e 'const L=require("fs").readFileSync(".tmp-153-size.jsonl","utf8").trim().split("\n").map((l)=>JSON.parse(l));const p=L.filter((r)=>typeof r.pointerErrPx==="number");process.exit(p.length>0&&p.every((r)=>r.pointerErrPx<=1)?0:1)' && node tools/p2-frames/measure.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" --dpr 1 --load a > .tmp-153-p2.json && node -e 'const r=JSON.parse(require("fs").readFileSync(".tmp-153-p2.json","utf8"));process.exit(r.error===null&&r.inputRateOk===true&&r.inputToPresentMsP95<=219?0:1)' && node tools/smoke/welcome-flow.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > .tmp-153-flow.jsonl && node -e 'const L=require("fs").readFileSync(".tmp-153-flow.jsonl","utf8").trim().split("\n").map((l)=>JSON.parse(l));const F=(f)=>L.find((r)=>r.flow===f);const ok=(r)=>!!r&&r.error===null&&r.state==="engine-idle"&&r.standIn===false&&r.trapped===false&&r.hostErrors===0&&r.opened===true&&r.elapsedMs<=12500&&r.preBootBytes<=314368711;process.exit(L.length===4&&["A","B","C","D"].every((f)=>ok(F(f)))&&F("B").welcomeClosed===true&&F("D").welcomeClosed===true?0:1)'
```

## 作業の進め方

- engine のビルドは約 35 分。フォアグラウンドで、シェルツールの `timeout` 引数に大きい値(例 3600000。上限 5400000)を渡して、直接実行する。背景で起動して、終わるのを待たない。
- 受け入れのコマンドは、段ごと(ビルド・`run.sh --scene iii`・計測の道具)に分けて実行してよい。最後の 1 回は、コマンド全体を通す。
- 内訳の特定は、変える前の engine で行い、方向を決めてから直す。
- 作業は開始から 5 時間以内に終える。最初に `date -u` で開始を記録する。4 時間 30 分を過ぎたら、受け入れが通っていなくても、作業ツリーを commit して、報告を書いて終える。
- 受け入れが通らなくても、作業の最後に作業ツリーを commit する(commit しないと判定が落ちる)。

## 報告

- 内訳の表(`pm-work/findings/input-cost-153.md` の要点)と、特定した行番号。
- 変更の前後の E・F・busyMs(DPR 1.00 と 1.325。各 n=3)。
- `measure-live.mjs`・`measure-size.mjs`・`p2-frames`・流れ A〜D の出力の行を貼る。
- commit の SHA。「達成」と書かず、「受理」か「差し戻し」かを、出力の行で書く。

## 制約

- 製品に、利用者向けの新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足さない。
- Python を使わない。OCR を使わない。
- ベンダー名・Cloudflare の識別子・内部の ID・URL を、ファイルと報告に書かない。
- sleep・timeout コマンド・nohup・末尾の `&` を使わない。一時ファイルは `.tmp-*` に置き、終わったら消す。
- `pm-work/findings/engine-cost-attribution.*` と `pm-work/findings/p2-frames-1350.md` は書き換えない(基準)。
- `app/` と `lift/` の変更は、push で main の `deploy.yml` を動かす。報告に書く。

## 他のタスクとの関係

- blocked_by: なし。
- 計測の道具と基準は `#1388`(段階 0)の `tools/engine-cost/` と `pm-work/findings/engine-cost-attribution.*`。
