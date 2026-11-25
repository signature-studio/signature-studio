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
- 場面 (iii) の A/B(同じ run の中の比)。変更前と変更後を、同じ時間帯に、交互に 3 回ずつ測る(DPR 1.00 と 1.325。各 n=3。DPR ごとに、変更前・変更後の順を n = 1, 2, 3 で繰り返す)。変更前は `origin/main` の `app/`(この Issue の変更を含まない main の版。作業の commit を含む HEAD は使わない)を `npm ci`・`npm run build` して、`tools/engine-cost/measure.mjs --app` で測る。変更後は `app/dist` を、既定の `measure.mjs` で測る。engine の bundle は両方で同じ物を使う。
- 12 run の全ての `error` が null。
- DPR ごとに、変更後の busyMs の中央値 ÷ 変更前の busyMs の中央値 が 0.80 以下。区分 E+F の中央値の比も、0.80 以下。
- 0.80 の理由: 変更前と変更後は同じ時間帯に同じ runner で測るので、runner の重さの揺れは比で打ち消される。固定の基準との比より、効果を正確に測れる。前回の交互の測定の比は、busy 0.63〜0.65、E+F 0.67〜0.69 だった。0.80 は、その余裕。
- 固定の基準(`pm-work/findings/engine-cost-attribution.json`)との比は、判定に使わない。`lift/out-app/engine-cost/ab/summary.json` の `vsFixedBaseline` に参考として出す。
- `measure-live.mjs`(入力 1000 Hz・再生中): `error` が null、`playing` が true、`underrunDelta` と `last60sUnderruns` が 0、`inputRateOk` が true。
- `pointerErrPx`: `measure-live.mjs` の出力には無い。`tools/feat-c1-hidpi/measure-size.mjs`(DPR 1)の出力の `pointerErrPx` が、数値の行が 1 件以上あり、全て 1 以下。
- `inputToPresentMsP95`(`tools/p2-frames/measure.mjs`、DPR 1、負荷 a): `error` が null、`inputRateOk` が true、値が 219 ms 以下。基準は `pm-work/findings/p2-frames-1350.md` の DPR 1・負荷 a の 3 回の中央値 182.7 ms の 1.2 倍。1 回の値は 82〜185 ms に散るので、219 ms を超えたら 3 回測り、3 回の値を報告に書く。
- 流れ A〜D(`tools/smoke/welcome-flow.mjs`): 4 行が全て、`error` が null・`state` が engine-idle・`standIn` が false・`trapped` が false・`hostErrors` が 0・`opened` が true。流れ B と D は `welcomeClosed` が true。各行の `elapsedMs` が 12,500 以下、`preBootBytes` が 314,368,711 B 以下。

場面 (iii) の 12 run ごとの busyMs・区分 E・区分 F、中央値の比、`vsFixedBaseline` を、報告に全て書く。

## 受け入れ

```accept
rm -rf .tmp-base && mkdir -p .tmp-base && git archive origin/main app | tar -x -C .tmp-base && npm ci --prefix .tmp-base/app && npm run build --prefix .tmp-base/app && npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && rm -rf lift/out-app/engine-cost/ab && node -e 'const {spawnSync}=require("child_process");const fs=require("fs");const P=process.cwd();const ok=(f)=>{try{return JSON.parse(fs.readFileSync(f,"utf8")).error===null}catch(e){return false}};for(const n of [1,2,3])for(const d of ["1.00","1.325"])for(const v of ["base","after"]){const out=`${P}/lift/out-app/engine-cost/ab/${v}-iii-dpr${d}-n${n}.json`;const a=["tools/engine-cost/measure.mjs","--bundle",`${P}/lift/out-app/bundle/raw/fl-Oz`,"--names",`${P}/lift/out-app/gen/wasm-func-names.tsv`,"--scene","iii","--dpr",d,"--out",out];if(v==="base")a.push("--app",`${P}/.tmp-base/app`);for(let t=0;t<2&&!ok(out);t++)spawnSync("node",a,{stdio:["ignore","ignore","inherit"]});if(!ok(out))process.exit(1)}' && node -e 'const fs=require("fs");const D="lift/out-app/engine-cost/ab";const b=require("./pm-work/findings/engine-cost-attribution.json");const rd=(v,d,n)=>JSON.parse(fs.readFileSync(`${D}/${v}-iii-dpr${d}-n${n}.json`,"utf8"));const med=(a)=>[...a].sort((x,y)=>x-y)[1];const ef=(r)=>r.classesMs.E+r.classesMs.F;const all=[];for(const d of ["1.00","1.325"])for(const n of [1,2,3])for(const v of ["base","after"])all.push(rd(v,d,n));if(!all.every((r)=>r.error===null))process.exit(1);const S={};let ok=true;for(const [d,k] of [["1.00",1],["1.325",1.325]]){const B=[1,2,3].map((n)=>rd("base",d,n));const A=[1,2,3].map((n)=>rd("after",d,n));const busy=med(A.map((r)=>r.busyMs))/med(B.map((r)=>r.busyMs));const e=med(A.map(ef))/med(B.map(ef));const bl=b.runs.filter((r)=>r.scene==="iii"&&r.dpr===k);const fb=bl.reduce((x,r)=>x+r.busyMs,0)/bl.length;const fe=bl.reduce((x,r)=>x+ef(r),0)/bl.length;S[d]={busyRatio:busy,efRatio:e,vsFixedBaseline:{busy:med(A.map((r)=>r.busyMs))/fb,ef:med(A.map(ef))/fe}};if(!(busy<=0.8&&e<=0.8))ok=false}fs.writeFileSync(`${D}/summary.json`,JSON.stringify(S)+"\n");console.log(JSON.stringify(S));process.exit(ok?0:1)' && node tools/p3-audio-probe/measure-live.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" --play 1 --hz 1000 --mouse 60 > .tmp-153-live.json && node -e 'const r=JSON.parse(require("fs").readFileSync(".tmp-153-live.json","utf8"));process.exit(r.error===null&&r.playing===true&&r.underrunDelta===0&&r.last60sUnderruns===0&&r.inputRateOk===true?0:1)' && node tools/feat-c1-hidpi/measure-size.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" --dprs 1 > .tmp-153-size.jsonl && node -e 'const L=require("fs").readFileSync(".tmp-153-size.jsonl","utf8").trim().split("\n").map((l)=>JSON.parse(l));const p=L.filter((r)=>typeof r.pointerErrPx==="number");process.exit(p.length>0&&p.every((r)=>r.pointerErrPx<=1)?0:1)' && node tools/p2-frames/measure.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" --dpr 1 --load a > .tmp-153-p2.json && node -e 'const r=JSON.parse(require("fs").readFileSync(".tmp-153-p2.json","utf8"));process.exit(r.error===null&&r.inputRateOk===true&&r.inputToPresentMsP95<=219?0:1)' && node tools/smoke/welcome-flow.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > .tmp-153-flow.jsonl && node -e 'const L=require("fs").readFileSync(".tmp-153-flow.jsonl","utf8").trim().split("\n").map((l)=>JSON.parse(l));const F=(f)=>L.find((r)=>r.flow===f);const ok=(r)=>!!r&&r.error===null&&r.state==="engine-idle"&&r.standIn===false&&r.trapped===false&&r.hostErrors===0&&r.opened===true&&r.elapsedMs<=12500&&r.preBootBytes<=314368711;process.exit(L.length===4&&["A","B","C","D"].every((f)=>ok(F(f)))&&F("B").welcomeClosed===true&&F("D").welcomeClosed===true?0:1)'
```

## 作業の進め方

- engine のビルドは約 35 分。フォアグラウンドで、シェルツールの `timeout` 引数に大きい値(例 3600000。上限 5400000)を渡して、直接実行する。背景で起動して、終わるのを待たない。
- 受け入れのコマンドは、段ごと(ビルド・変更前と変更後の場面 (iii)・計測の道具)に分けて実行してよい。最後の 1 回は、コマンド全体を通す。場面 (iii) の run は、起動の停止で失敗したら、受け入れの中で、その run だけ 1 回やり直す(run ごとに 2 回まで)。
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

## 前回の試行の結果(2 回目の前に)

前回の試行は、判定が、外部プラグインのインストーラの検査で落ちた(修正済み)。

**結果の要点: 入力 1 件ごとの wake の `postMessage` をやめ、フレームごと 1 回にし(`Atomics.waitAsync` で Worker が待つ)、E+F は、基準の DPR 1.00 で 91%、DPR 1.325 で 86%、busy は 83%・76% に下がった。目標(E+F ≤ 60%、busy ≤ 75%)には届いていない**。残りの大部分は F(ブラウザ内部)で、**入力の配送そのもの**。

**注意: 測定は、CDP の `Input.dispatchMouseEvent` を 1200 Hz で送る。CDP の注入の費用は、ブラウザのプロセスと、ページの主スレッドに掛かる(測定の道具の費用)が、Worker の F(`(program)`)の中身が、何か(入力のメッセージの受信・`structuredClone`・`Atomics`・native)を、Worker の trace(`Tracing`)のイベントの自己時間で特定することが、次の仕事。測定の道具の費用を、製品の費用と数えない**(Worker のプロファイルだけを使う)。

## 前回の試行の結果(4 回目の前に)

前回の run(`failed`)は、場面 (iii) の判定の段で落ちた。ほかの項目は、起動の停止が出た箇所を除いて通った(下の 7)。前回の作業の差分は、次の試行が取り込んで続ける。

1. **変更の内容**: 入力 1 件ごとの `wake` をやめ、入力のあったフレームに 1 回だけ `Atomics.notify` する。Worker は idle の間 `Atomics.waitAsync` で待ち、期限の待ちも `setTimeout` から `waitAsync` にした。主スレッドの `performance` の読み取りを減らし、窓の文字列は、バイトが変わった窓だけ decode する。`run.sh --scene`・`measure.mjs --app`・`trace.mjs` を足した(既定の動作は変えていない)。
2. **効果は出ている**: 同じ時間帯に、変更前(`origin/main` の版)と変更後を交互に 3 回ずつ測ると、変更後の比は busy 0.63〜0.65、E+F 0.67〜0.69(DPR 1.00 と 1.325)。60 秒のトレースで、Worker の時間は入力 1 件あたり 95 µs から 38 µs になった。
3. **落ちた理由**: 判定は固定の基準(busy 12,388 ms、E+F の平均)との比だった。この runner では、変更前の版そのものが基準より重く(busy 14,459 ms)、上限(E+F は 60% 以下、busy は 75% 以下)に届かなかった。最後の build の値は、DPR 1.00 で E+F 85.7%・busy 75.5%、DPR 1.325 で E+F 85.2%・busy 75.0%。全ての run の `error` は null。runner で合否が変わるので、判定は同じ run の中の比にした(「判定の基準」)。
4. **残りの費用**: 変更前の 20 秒のトレースで、主スレッドの `(program)` は約 3,045 ms。その最大は、ポインタのリスナの中(約 1,068 ms)。リスナの JS 本体は小さく、残りは区分 F(ブラウザ内部)の処理と見られる(1 入力あたり約 35 µs。原因は未特定)。Worker の `(program)` は idle の回数に比例し(1 回あたり約 32〜35 µs。相関だけ)、`getCoalescedEvents` は使っていない。残りの費用は、アプリの JS だけでは大きくは減らせない見込み。
5. **起動の停止**: `state running` のまま 180 秒の停止が、前回の木の測定で約 1 割出た(変更前にも変更後にも)。この run は #1393 が main に入る前の木で動いていた。#1393 の後の main では、198 回の起動で止まりは 0 件。
6. **内蔵プラグインの名前の不整合**: 古い main の run は、内蔵プラグインの DLL の名前の不整合で `fe-build engine` が落ちた。main で直った(4d757064)。
7. **他の項目**: `measure-live` は通った(`inputRateOk` が true、`underrunDelta` が 0)。`p2-frames` の `inputToPresentMsP95` は 72.97 ms と 77.34 ms(上限 219 ms。通った)。`measure-size` は、DPR 1 の行の `pointerErrPx` が 0 で通ったが、別の DPR の行が起動の停止で error になった。流れ A〜D は、A・D が通り、B か C が起動の停止で 1 回ずつ落ちた。
8. **判定の変更**: 固定の基準との比は、runner の速さで合否が決まるので判定から外し、同じ時間帯の変更前との比にした。他の項目は変えていない。

