---
draft: "140"
title: "FL の上部バーの Memory Usage・CPU Load・Frame Rate が、実際の値を出す"
type: Task
milestone: "M3 トップバーのコンテキストメニュー(P2)"
parent: "009"
blocked_by: ["139"]
labels: ["topbar", "verify"]
size: L
priority: High
project: signature-studio/3
---

## 目的

依頼者の指示(原文): 「トップバーの Memory Usage と CPU Load はずっと 0MB のまま、Frame Rate は 120 で固定されていますが、どう見ても 30fps すら満たしていません。(CPU Load History グラフはそれっぽい値が出ています)実際の数値を反映するようにしてください。」

この Issue の目標は次の 3 つ。
1. Memory Usage が、エンジンが実際に使っているゲストのメモリの量を、MB で出す(0 でない。ページ側の実測と近い)。
2. CPU Load が、エンジンの Worker が実際に計算していた割合を出す(無操作の時と、再生の時で、値が違う。Worker の実測と近い)。
3. Frame Rate が、FL が実際に画面を提示した回数(実時間あたり)を出す(120 の固定ではない。ページの実測と近い)。

PM の決定(定義): 数値は、実際の値を出す。値を良く見せるために、数字を作らない。低い値が本当なら、そのまま出す(低い理由は #1351 の描画の費用で扱う)。

## 背景

依頼者の画面では、上部バーの Memory Usage と CPU Load が 0MB のまま。Frame Rate は 120 で固定。CPU Load History のグラフには、それらしい値が出ている。数値の出どころを、次の事実から確かめる。

- 事実 1(時計が仮想): `lift/runtime/fle_sys.h` の 9〜20 行: 時計は 100 ns の目盛りの仮想の時計。時間が進むのは、Sleep、時計の読み(1 回の読みで 10 ticks)、DwmFlush の 3 つだけ。実時間には、そのままでは連動しない。
- 事実 2(画面の周期が仮想): `lift/runtime/fle_thread.h` の FLE_DWM_HZ の既定は 60(環境変数で変えられる)。`lift/runtime/fle_thread.c` の `fle_imp_DwmFlush` は、仮想の時計を次の枠の境界まで進める(スレッドが 1 つの時)。決定 0166 の注釈(`fle_thread.h` の 72 行付近): 「the guest's UI cycle follows DwmFlush, measured 2.00x the cycles and the draws at 120」。この注釈の読み: FL の UI の周期は DwmFlush に従い、描画は仮想の 60 Hz の 2 倍(120)の回数で行われる(作業者が、呼び出しの記録で確かめる)。
- 事実 3(仮想の時計と実時間の差): `app/src/engine/worker.ts` の `lagCut`(S-clk-maxlag)は、仮想の時計が実時間より遅れすぎた時、その遅れを残す(仮想の時間は飛ばさない)と記録する。実際の処理が遅くても、仮想の時間の上では描画の数を保つ。
- 事実 4(EnumDisplaySettingsW): `lift/runtime/fle_monitor.c` の EnumDisplaySettingsW は、dmDisplayFrequency に `fle_display_settings_hz()` を入れる(既定 60)。
- 事実 5(Memory): `lift/runtime/fle_sysinfo.h` の 29〜35 行: GlobalMemoryStatus は定数を返す(総物理 4 GB・空き物理 2 GB・ページファイル 8 GB・空き 6 GB・負荷 50%)。実際のメモリの量は反映していない。
- 事実 6(CPU の数): `lift/runtime/fle_sys.c` の GetSystemInfo の注釈: プロセッサは 1 個(マスク 1。決定)。
- 事実 7(API の有無): `GetProcessMemoryInfo`・`K32GetProcessMemoryInfo`・`GetSystemTimes`・`GetProcessTimes`・`GetThreadTimes` は、`lift/runtime`・`lift/crates`・`app/src` に実装が無い(git grep で 0 件)。`lift/crates/fe-lift/src/imports_table.rs` の冒頭の注釈: 表に無い import は unsupported の経路に入る(`lift/crates/fe-lift/src/lib.rs` の `fle_unsupported`)。FL が GetProcAddress で名前を引く経路なら、`lift/runtime/fle_modules.c` の GetProcAddress の注釈どおり、名前が無い時は NULL と ERROR_PROC_NOT_FOUND(127)が返り、FL は 0 として表示しうる。**どの経路を FL が使っているかは未確認**。呼び出しの記録で確かめる。
- 事実 8(エンジンの忙しい時間): `app/src/engine/worker.ts` の driver の `idle(n, now, next, busyMs, at)` は、1 回の idle の busyMs を受け取る。上部バーには届いていない(推定。確かめる)。
- 事実 9(提示の数): engine の提示の回数(presents)は、`app/src/perf/beacon.ts` の DeskCounters と、終了の記録(`app/src/main.ts` の exit)に在る。
- 事実 10(ページの表示): `app/src/perf/display.ts` の表示の計測は、既定では rAF を連続で回さない(burst。決定 0202)。
- 事実 11(文字の読み): 上部バーの値の文字は、ゲストのメモリの中の文字列として読める(`app/src/engine/worker.ts` の `guestText`。#1358 と #1376 の方式。`pm-work/findings/hidpi-fl-scaling.md` の 207 行付近。OCR は使わない)。

## 読む物

- `lift/runtime/fle_sys.h`・`lift/runtime/fle_sys.c`(時計・GetSystemInfo)・`lift/runtime/fle_thread.h`(FLE_DWM_HZ と決定 0166 の注釈)・`lift/runtime/fle_thread.c`(`fle_display_settings_hz`・`fle_thread_next_frame_tick`・`fle_imp_DwmFlush`)
- `lift/runtime/fle_monitor.c`(EnumDisplaySettingsW)・`lift/runtime/fle_sysinfo.c`・`lift/runtime/fle_sysinfo.h`(GlobalMemoryStatus・定数)
- `lift/runtime/fle_modules.c`(GetProcAddress の注釈)・`lift/crates/fe-lift/src/imports_table.rs`(冒頭の注釈)・`lift/crates/fe-lift/src/lib.rs`(`fle_unsupported`)
- `app/src/engine/worker.ts`(`lagCut`・driver の `idle`・`guestText`)・`app/src/engine/client.ts`・`app/src/engine/protocol.ts`・`app/src/main.ts`(`EngineClient`・exit)
- `app/src/perf/beacon.ts`・`app/src/perf/display.ts`・`app/src/perf/display-host.ts`
- `tools/p2-frames/measure.mjs`(提示の数と、ページの実測の方式)・`tools/smoke/welcome-flow.mjs`(起動と測り方)・`tools/p3-audio-probe/measure.mjs`(Playlist の再生の始め方)
- `pm-work/findings/p2-frames-1350.md`(提示の数の測り方)・`pm-work/findings/hidpi-fl-scaling.md`(ゲストのメモリの走査)
- `pm-work/findings/real-user-beacons-2026-11-24.md`(実機の記録)

## 数値の定義(PM の決定)

- Memory Usage = エンジンが実際に確保しているゲストのメモリの量(MB)。runtime の VirtualAlloc・HeapAlloc・像の読み込みの積算。ページ側の実測(`WebAssembly.Memory.buffer.byteLength` の和。`performance.measureUserAgentSpecificMemory` が使える時はその値)と比べ、前者が嘘でないことを確かめる。
- CPU Load = エンジンの Worker(音声・計算の thread)が、実時間のうち、実際に計算していた割合(%)。`busyMs` ÷ 経過した実時間。Worker ごとに計り、FL が読んでいる API の形(GetSystemTimes の差など)に合わせる。
- Frame Rate = FL が実際に画面を提示した回数 ÷ 実時間(秒)。実時間は、仮想の時計ではなく、実際の時刻(`performance.now` の相当)で数える。どの形で FL の表示に入れるかは、FL の呼び出しの記録を見て決める。

## やること

0. 呼び出しの記録(直す前): 既定のプロジェクトを開いて、上部バーの 3 つの値を出す。FL が使う API を、呼び出しの記録で取る(GetProcessMemoryInfo・K32GetProcessMemoryInfo・GetSystemTimes・GetProcessTimes・GetThreadTimes・GetProcAddress の名前・QueryPerformanceCounter・timeGetTime・EnumDisplaySettingsW・DwmFlush・SetTimer)。上部バーの文字列(MB・%・FPS を含む)は、`guestText` で走査し、その値と、FL の呼び出しの戻り値を対応させる。報告に、値の出どころの表を書く。
1. 直す: 上の定義で値を出す経路を作る。製品の既定の動作として入れる。新しい環境変数・オプトインは作らない。仮想の時計(決定 0122)を変える時は、起動と音声への影響を測り、報告に書く。表示のための数字だけを、実時間で割る形もある(時計の挙動は変えない)。どちらを選ぶかは、測定の結果で決める。
2. 数える処理は、音声を止めない(P3)。数えるのは、既存の情報(busyMs・presents)だけ。新しい周期の処理を増やさない。
3. 測定の道具 `tools/topbar-metrics/measure.mjs`(新規)。Playlist を再生する版と、無操作の版の 2 つを取る。Frame Rate は、DPR 1 と、乱数の DPR(1.01〜3.00)の 2 点で取る。ページの rAF の数は、道具の中だけで連続に数えてよい(製品は、既定の burst のまま。決定 0202)。stdout に JSON を 1 行。

## 受け入れ

```accept
npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false?0:1)' "$RUNNER_TEMP/boot.json" && node tools/topbar-metrics/measure.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/metrics.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));const s=r.fps.samples;process.exit(r.error===null&&r.memory.panelMb>0&&r.memory.ratio>=0.5&&r.memory.ratio<=1.5&&r.cpu.playingPct>r.cpu.idlePct+1&&Math.abs(r.cpu.playingPct-r.cpu.busyPct)<=10&&s.length===2&&s.every((x)=>Math.abs(x.shown-x.rafPerSec)<=0.15*x.rafPerSec)&&!s.every((x)=>x.shown===120)?0:1)' "$RUNNER_TEMP/metrics.json" && node tools/smoke/welcome-flow.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/flow.jsonl" && node -e 'const fs=require("fs");const L=fs.readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const F=(f)=>L.find((r)=>r.flow===f);const ok=(r)=>!!r&&r.error===null&&r.state==="engine-idle"&&r.standIn===false&&r.trapped===false&&r.hostErrors===0&&r.opened===true;const pb=Math.max(...L.map((r)=>r.preBootBytes));const el=Math.max(...L.map((r)=>r.elapsedMs));process.exit(L.length===4&&["A","B","C","D"].every((f)=>ok(F(f)))&&F("B").welcomeClosed===true&&F("D").welcomeClosed===true&&el<=12500&&pb<=314385095?0:1)' "$RUNNER_TEMP/flow.jsonl" && npm test --prefix app
```

判定の基準(`tools/topbar-metrics/measure.mjs` の JSON の項目):
- `error`: null。
- `memory.panelMb`: 上部バーの Memory Usage の値(MB。0 より大きい)。`memory.pageBytes`: ページ側の実測(`WebAssembly.Memory` の大きさの和)。`memory.ratio` = `panelMb` × 1,048,576 ÷ `pageBytes`。0.5 以上 1.5 以下。
- `cpu.idlePct`: 無操作の時の CPU Load(%)。`cpu.playingPct`: Playlist を再生している時の CPU Load(%)。`cpu.busyPct`: 再生の時の Worker の忙しい時間の実測(%。Worker の計測。FL の表示とは別の経路)。判定: `playingPct` が `idlePct` より 1 ポイント以上大きい、かつ `|playingPct − busyPct|` が 10 ポイント以下。
- `fps.samples`: 2 点(DPR 1 と乱数の DPR)。各点: `dpr`・`shown`(上部バーの Frame Rate の値)・`rafPerSec`(ページの rAF の数 ÷ 秒。同じ区間)・`presentsPerSec`(engine の提示の増分 ÷ 秒。同じ区間)。判定: 各点で `|shown − rafPerSec|` が `rafPerSec` の 15% 以内。2 点とも 120 ではない、という条件は満たさなくてもよいが、2 点とも 120 なら不合格(`!s.every((x)=>x.shown===120)`)。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には「runner の値。この機械とは別枠」と書き、nproc と loadavg を添える。
- 「達成」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。出力の行を付ける。
- 報告に書く: 直す前の値の出どころの表(FL が呼んだ API と戻り値)、直した後の定義と経路、仮想の時計を変えたか(変えたなら起動と音声への影響)、3 つの値の直す前と後、低い値が出た時は、その理由(描画の費用・#1351 との関係)。

## 制約

- 製品に、利用者向けの新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足さない。オプトインを作らない。既存の FLE_DWM_HZ を読むのは可。
- 数字を作らない。値を良く見せない。低い値が本当なら、そのまま出す。
- ゲストのメモリには書かない(読むだけ)。
- Python は製品に使わない(調査と検証にだけ使う)。OCR は使わない。
- ベンダー名と Cloudflare の識別子を、ファイルに書かない。
- sleep・timeout コマンド・nohup・末尾の `&` は使わない。一時ファイルは `.tmp-*` に置き、終わったら消す。
- 製品の作業は Actions で行う。この機械で製品のビルドや測定をしない。

## 作業の進め方

- 開始の時刻を、最初に `date -u` で記録する。4 時間 30 分を過ぎたら、受け入れが通らなくても、作業ツリーを commit し、報告を書いて終える。
- engine の作り直しは約 35 分。1 回の試行で、ビルドは数回まで。呼び出しの記録は、1 回のビルドにまとめて取る。
- ビルドは、フォアグラウンドで、シェルツールの `timeout` 引数に 5400000 を渡して待つ。背景で起動して待たない。
- 作業の最後に、受け入れが通らなくても、作業ツリーを commit する。

## 他のタスクとの関係

- blocked_by: #{{139}}(マウスカーソル段 1。同じ engine から page への通知の経路を使うので、直列にする)。
- #1351(P2 の描画の費用。Frame Rate が低いと出た時の原因の側。この Issue では扱わない)。
- #{{138}}・#{{144}} とは、直列の順(138 → 139 → 144・140)。
