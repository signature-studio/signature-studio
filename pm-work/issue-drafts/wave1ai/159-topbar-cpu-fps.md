---
draft: "159"
title: "上部バーの CPU Load と Frame Rate が、実際の値を出す(段階 2)"
type: Task
milestone: "M3 トップバーのコンテキストメニュー(P2)"
parent: "009"
blocked_by: ["140"]
labels: ["topbar", "verify"]
size: L
priority: High
project: signature-studio/3
---

## 目的

依頼者の指示(原文): 「トップバーの Memory Usage と CPU Load はずっと 0MB のまま、Frame Rate は 120 で固定されていますが、どう見ても 30fps すら満たしていません。(CPU Load History グラフはそれっぽい値が出ています)実際の数値を反映するようにしてください。」

この Issue は**段階 2(CPU Load と Frame Rate)**。段階 1(Memory Usage。#{{140}})が閉じた後に起動する(blocked by)。段階 1 の判定(Memory)は、この Issue の判定にも入る(回帰の確認)。

この Issue の目標は次の 2 つ。
1. CPU Load が、エンジンの Worker が実際に計算していた割合(%)を出す。無操作の時と、再生の時で値が違い、Worker の実測と ±10 ポイント以内。
2. Frame Rate が、FL が実際に画面を提示した回数(実時間あたり)を出す。120 の固定ではなく、エンジンの提示の増分 ÷ 実時間と ±15% で一致する。

PM の決定(定義): 数値は、実際の値を出す。値を良く見せるために、数字を作らない。低い値が本当なら、そのまま出す(低い理由は #1351 の描画の費用で扱う)。

## 前回の試行で分かったこと

- 前回の試行は、段階 1(#{{140}})の中で行った(run 45469639595。判定は `failed`)。段階 2 の作業は、まだしていない。以下は、その測定の結果。
- CPU(Worker の実測。runner の値): 無操作 0.565%・再生 0.456%。Worker の計測は、idle の時刻が絶対時刻なのに、窓の境界が `performance.now()` だったため、窓の中の記録が 0 件だった。両方を絶対時刻に揃え、取れるようになった。
- Frame(エンジンの提示): 無操作 0.13/s・再生 11.4/s(DPR 1 で 11.6/s、DPR 2.7 で 11.1/s)。提示の数は、engine の DeskCounters の増分 ÷ 実時間。
- 表示の値の出どころ(未確定): 下段右の CPU「0」(単位なし)と、Frame「120」は、ゲストのメモリの走査で読めていない(周りに数字の文字列が在るだけ)。
- 呼び出しの記録(FL 本体): 時間の読みは `GetTickCount`・`QueryPerformanceCounter`・`timeGetTime`。QPC の 2 つの差を積んで比を取る関数(FL 側の計時。`f_1253840`)が在り、CPU の値の候補。表示との接続は未確認。
- `DwmFlush` を呼ぶのは本体の外のモジュール(QuickFontCache の TTimerThread)。仮想の DWM は 60 Hz で、描画は 2 倍(120 回/秒。決定 0166)。表示の「120」がこれかは未確認。
- 仮想の時計: 時計の読み 1 回で 10 ticks 進み、`DwmFlush` は仮想の枠の境界まで進める(決定 0122・0166)。実時間には連動しない。
- 道具の状態: `tools/topbar-metrics/measure.mjs` の `cpu.idlePct`・`cpu.playingPct`・`fps.samples` は、前回の版では null のまま。この段階で埋める。

## 背景

- 事実 1(時計が仮想): `lift/runtime/fle_sys.h` の 9〜20 行: 時計は 100 ns の目盛りの仮想の時計。時間が進むのは、Sleep、時計の読み、DwmFlush の 3 つだけ。実時間には、そのままでは連動しない。
- 事実 2(画面の周期が仮想): `lift/runtime/fle_thread.h` の FLE_DWM_HZ の既定は 60(環境変数で変えられる)。`lift/runtime/fle_thread.c` の `fle_imp_DwmFlush` は、仮想の時計を次の枠の境界まで進める(スレッドが 1 つの時)。決定 0166 の注釈(`fle_thread.h` の 72 行付近): 「the guest's UI cycle follows DwmFlush, measured 2.00x the cycles and the draws at 120」。
- 事実 3(仮想の時計と実時間の差): `app/src/engine/worker.ts` の `lagCut`(S-clk-maxlag)は、仮想の時計が実時間より遅れすぎた時、その遅れを残す(仮想の時間は飛ばさない)と記録する。
- 事実 4(忙しい時間): `app/src/engine/worker.ts` の driver の `idle(n, now, next, busyMs, at)` は、1 回の idle の busyMs を受け取る。上部バーには届いていない(推定)。
- 事実 5(提示の数): engine の提示の回数(presents)は、`app/src/perf/beacon.ts` の DeskCounters と、終了の記録(`app/src/main.ts` の exit)に在る。
- 事実 6(ページの表示): `app/src/perf/display.ts` の表示の計測は、既定では rAF を連続で回さない(burst。決定 0202)。
- 事実 7(文字の読み): 上部バーの値の文字は、ゲストのメモリの中の文字列として読める(`app/src/engine/worker.ts` の `guestText`。#1358 と #1376 の方式。OCR は使わない)。

## 決定(PM)

- 依頼者の指示(上部バーの数字は実際の値を出す)が、決定 0122・0166(仮想の時計・仮想の DWM)に優先する。FL が見る時計(`QueryPerformanceCounter`・`QueryPerformanceFrequency`・`GetTickCount`(64)・`timeGetTime`・`GetSystemTimeAsFileTime`・`GetThreadTimes`・`GetProcessTimes`・`DwmFlush` の待ち)を、実時間(Worker の `performance.now()` に相当)に結ぶ。
- 仮想の時計が音声のスレッドの同期・テストの決定性のために在る場合は、影響を測る(決定 0122 の理由を、`docs/` か `pm-work/` の決定の記録で読む)。P3・起動・流れ・止まりが悪化するなら、時計の結び方を段階に分ける(例: `GetTickCount`・`timeGetTime` だけ実時間にする)。
- `DwmFlush` は、ページの実際の描画の周期(`requestAnimationFrame`)に結ぶ。Worker が、ページの rAF の通知を待つ。
- FL が表示する Frame Rate は、そのまま実際の値になる。低くても、数字を作らない。
- 判定 ③(CPU): FL の表示値(ゲストのメモリの走査で読む)が、無操作の時と、再生中で異なり(差が 1 ポイント以上)、Worker の忙しい時間の実測(%)と ±10 ポイント以内。
- 判定 ④(Frame): 表示値が、同じ区間のエンジンの提示(`presents`)の増分/秒と ±15% 以内。

## 読む物

- `lift/runtime/fle_sys.h`・`lift/runtime/fle_sys.c`・`lift/runtime/fle_thread.h`・`lift/runtime/fle_thread.c`(時計・DwmFlush・`fle_display_settings_hz`)
- `lift/runtime/fle_monitor.c`(EnumDisplaySettingsW)
- `app/src/engine/worker.ts`(`lagCut`・driver の `idle`・`guestText`)・`app/src/engine/client.ts`・`app/src/engine/protocol.ts`・`app/src/main.ts`
- `app/src/perf/beacon.ts`・`app/src/perf/display.ts`
- `tools/topbar-metrics/measure.mjs`(段階 1 の道具)・`tools/p2-frames/measure.mjs`・`tools/p3-audio-probe/measure.mjs`(`--boot`)
- `pm-work/findings/topbar-metrics-1391.md`(前回の記録)・`pm-work/findings/p2-frames-1350.md`

## やること

0. 段階 1 が main に入っていることを、`git log` で確かめる。在れば、その上で続ける。
1. **時計の読みの経路を確かめる**: FL が CPU の値と Frame の値を作る経路を、呼び出しの記録で確かめる(`GetTickCount`・`QueryPerformanceCounter`・`timeGetTime`・`DwmFlush` の戻り番地)。どの読みが表示に入るかを、報告に書く。確かめられない物は「未確認」と書く。
2. **実時間に結ぶ**: 「決定」のとおり、FL が見る時計を実時間に結ぶ。結んだ後の値を、変更の前後で報告に書く。
3. **音声と起動を守る**: 変更の前後で `measure.mjs --boot`(3 回)・`boot-bundle`(elapsedMs)・流れ A〜D を測り、悪化しないことを確かめる。悪化したら、結び方を段階に分ける。
4. **CPU**: `tools/topbar-metrics/measure.mjs` で、無操作の時と、Playlist の再生の時の、上部バーの CPU の値(`cpu.idlePct`・`cpu.playingPct`)と、Worker の忙しい時間(`cpu.busyPct`)を出す。
5. **Frame**: 同じ道具で、DPR 1 と乱数の DPR(2 点)の、上部バーの Frame Rate(`fps.samples[].shown`)と、エンジンの提示の増分 ÷ 秒(`presentsPerSec`)を出す。
6. **止まりの道具**(`tools/boot-hang/measure.mjs`。#1393 の道具): main に在れば、同じ判定(止まり 0)で確かめる。無ければ、報告に「確認できない」と書く。
7. **受け入れ**: 下の「受け入れ」のコマンドを、そのまま通す。

## 受け入れ

```accept
npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false&&r.elapsedMs<=12500?0:1)' "$RUNNER_TEMP/boot.json" && node tools/topbar-metrics/measure.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/metrics.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));const m=r.memory,c=r.cpu,s=r.fps.samples;process.exit(r.error===null&&((()=>{const h=m.hash,hs=h.series,hf=h.floors,fl=Math.floor(m.k32Bytes[m.k32Bytes.length-1]/1048576),near=[...new Set(m.candidates.filter((q)=>Math.abs(q-fl)<=1))],ex=near.includes(fl),fc=[],hc=[];for(let i=1;i<hs.length;i++){if(hf[i]!==hf[i-1])fc.push(i);if(hs[i]!==hs[i-1])hc.push(i);}return m.k32Bytes.length>=1&&m.k32Bytes.every((v)=>Number.isInteger(v))&&near.length>=1&&(ex?m.shownMb===fl:(near.length===1&&m.shownMb===near[0]))&&m.zeroMbNew===0&&hs.length>=40&&hs.length===hf.length&&(h.method==="band"||(h.method==="rect"&&fc.length>=1&&hc.length>=fc.length&&hc.every((x)=>fc.some((j)=>j>=x-2&&j<=x))));})())&&Number.isFinite(c.idlePct)&&Number.isFinite(c.playingPct)&&Number.isFinite(c.busyPct)&&c.playingPct>c.idlePct+1&&Math.abs(c.playingPct-c.busyPct)<=10&&s.length===2&&s.every((x)=>Number.isFinite(x.shown)&&Number.isFinite(x.presentsPerSec)&&x.presentsPerSec>0&&Math.abs(x.shown-x.presentsPerSec)<=0.15*x.presentsPerSec)&&!s.every((x)=>x.shown===120)?0:1)' "$RUNNER_TEMP/metrics.json" && for i in 1 2 3; do node tools/p3-audio-probe/measure.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" --boot > "$RUNNER_TEMP/boot-$i.json" || exit 1; done && node -e 'const fs=require("fs");const L=[1,2,3].map((i)=>JSON.parse(fs.readFileSync(process.argv[1]+"/boot-"+i+".json","utf8").trim()));const a=L.map((r)=>r.bootLast60sAt10s).filter((v)=>typeof v==="number").sort((x,y)=>x-y);process.exit(L.length===3&&a.length===3&&L.every((r)=>r.error===null&&r.state==="engine-idle"&&r.standIn===false&&r.ctxState==="running"&&r.quantaDelta>=19000&&r.bootLast60sAt70s===0&&r.bootLast60sAt90s===0)&&a[1]<=30&&a[2]<=60?0:1)' "$RUNNER_TEMP" && node tools/smoke/welcome-flow.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/flow.jsonl" && node -e 'const fs=require("fs");const L=fs.readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const F=(f)=>L.find((r)=>r.flow===f);const ok=(r)=>!!r&&r.error===null&&r.state==="engine-idle"&&r.standIn===false&&r.trapped===false&&r.hostErrors===0&&r.opened===true;process.exit(L.length===4&&["A","B","C","D"].every((f)=>ok(F(f)))&&F("B").welcomeClosed===true&&F("D").welcomeClosed===true&&Math.max(...L.map((r)=>r.elapsedMs))<=12500&&Math.max(...L.map((r)=>r.preBootBytes))<=314385095?0:1)' "$RUNNER_TEMP/flow.jsonl" && npm test --prefix app
```

判定の基準:
- 段階 1 の判定(`memory` の (i) と (ii)。#{{140}} の「判定の基準」と同じ式)が、そのまま通る(回帰)。
- `error`: null。`cpu.idlePct`・`cpu.playingPct`・`cpu.busyPct` が数値。判定 ③: `playingPct` が `idlePct` より 1 ポイント以上大きい、かつ `|playingPct − busyPct|` が 10 ポイント以下。
- `fps.samples`: 2 点(DPR 1 と乱数の DPR)。各点で `shown` と `presentsPerSec` が数値、`presentsPerSec` が 0 より大きい、`|shown − presentsPerSec|` が `presentsPerSec` の 15% 以内(判定 ④)。2 点とも 120 なら不合格(120 の固定の回帰を防ぐ)。
- 起動(`boot.json`): engine-idle・standIn false・elapsedMs が 12,500 以下。
- 音声(`measure.mjs --boot` の 3 回): error null・engine-idle・ctxState running・quantaDelta が 19,000 以上・`bootLast60sAt70s` と `bootLast60sAt90s` が 0・`bootLast60sAt10s` の 3 回の中央値が 30 以下・最大が 60 以下。
- 流れ A〜D・`npm test --prefix app`。
- 変更の前後の比較(報告に書く): 起動の elapsedMs、P3 の 3 回、流れ A〜D の elapsedMs、止まりの道具の結果(main に在る時)。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には「runner の値。この機械とは別枠」と書き、nproc と loadavg を添える。
- 「達成」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。出力の行を付ける。
- 報告に書く: 表示の値の出どころ(FL が呼んだ API と戻り値)、仮想の時計を変えたか(変えたなら、起動と音声への影響を、変更の前後で)、3 つの値(CPU・Frame・Memory)の直す前と後、低い値が出た時は、その理由(描画の費用・#1351 との関係)。

## 制約

- 製品に、利用者向けの新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足さない。オプトインを作らない。既存の FLE_DWM_HZ を読むのは可。診断は一時的にし、commit しない。
- 数字を作らない。値を良く見せない。低い値が本当なら、そのまま出す。
- ゲストのメモリには書かない(読むだけ)。
- 新しい周期の処理を増やさない(数えるのは、既存の情報: busyMs・presents)。
- Python・OCR は使わない。
- ベンダー名と Cloudflare の識別子を、ファイルに書かない。
- sleep・timeout コマンド・nohup・末尾の `&` は使わない。一時ファイルは `.tmp-*` に置き、終わったら消す。
- 製品の作業は Actions で行う。この機械で製品のビルドや測定をしない。

## 作業の進め方

- 開始の時刻を、最初に `date -u` で記録する。4 時間 30 分を過ぎたら、受け入れが通らなくても、作業ツリーを commit し、報告を書いて終える。
- engine の作り直しは約 35 分。ビルドは、フォアグラウンドで、シェルツールの `timeout` 引数に 5400000 を渡して待つ。背景で起動して待たない。
- 作業の最後に、受け入れが通らなくても、作業ツリーを commit する。

## 他のタスクとの関係

- blocked_by: #{{140}}(段階 1)。
- #{{138}}・#{{144}}: 同じ engine の経路の作業。直列の順で扱う。
- #{{149}}(起動の間欠の停止): 流れ A〜D・起動確認が落ちたら、報告に書いて、commit して終える。再試行で続く。この Issue の依存には足さない。
- #1351(P2 の描画の費用。Frame Rate が低いと出た時の原因の側。この Issue では扱わない)。

## 他の Issue との重なり

#{{139}}(マウスカーソル段 1)と、engine から page への通知のメッセージ型を足す変更が、`app/src/engine/protocol.ts` と `app/src/engine/worker.ts` で重なりうる。**足す変更どうしなので、`main` への push で衝突したら、両方の型・handler を残して解く**(片方を消さない)。解いた後、`npm test --prefix app` と受け入れを通す。
