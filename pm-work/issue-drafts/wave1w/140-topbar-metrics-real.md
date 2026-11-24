---
draft: "140"
title: "FL の上部バーの Memory Usage・CPU Load・Frame Rate が、実際の値を出す"
type: Task
milestone: "M3 トップバーのコンテキストメニュー(P2)"
parent: "009"
blocked_by: []
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
3. Frame Rate が、FL が実際に画面を提示した回数(実時間あたり)を出す(120 の固定ではない。engine の提示の増分 ÷ 実時間と ±15% で一致する)。

PM の決定(定義): 数値は、実際の値を出す。値を良く見せるために、数字を作らない。低い値が本当なら、そのまま出す(低い理由は #1351 の描画の費用で扱う)。

## 前回の試行で分かったこと

- 前回: run 45366377208(09:34〜14:38 UTC。判定は `failed`)。**前回の差分は、作業ツリーに未 commit で載っている**。`.prior-work.md` に判定の理由が書いてある。最初に読む。前回の commit(`3175bc18`・`1f93f136`)は main に無い。
- 前回の差分の中身:
  - `lift/runtime/fle_sysinfo.c`: `fle_imp_K32GetProcessMemoryInfo`(GetCurrentProcess の擬似ハンドル以外は止める。使用量は `fle_sys_in_use_bytes()`)。
  - `lift/runtime/fle_sys.c`・`fle_sys.h`: `fle_sys_in_use_bytes()`(コミットした VirtualAlloc のページ + 生きている HeapAlloc のブロックの要求サイズ。読むだけ)。
  - `lift/crates/fe-lift/src/imports_table.rs`: `TABLE` の末尾に、`kernel32.dll` の `K32GetProcessMemoryInfo` の 1 行。
  - `app/src/engine/worker.ts`: `guestText` の上限を、文字コードごとに数える(NEAR_PLACES 24 → 400)。ASCII の候補が上限を使い切り、UTF-16 の上部バーの文字に届かなかった。
  - `tools/topbar-metrics/measure.mjs`(新規)・`pm-work/findings/topbar-metrics-1391.md`(新規。前回の記録)。
- 受け入れの結果: 起動確認(`boot-bundle`)は通った(`engine-idle`・`standIn: false`・11492 ms)。計測(`tools/topbar-metrics/measure.mjs`)の段で、エンジンが trap した: `RangeError: Maximum call stack size exceeded (other)`(engine worker、running)。`welcome-flow` と `npm test` は実行されていない。

呼び出しの記録(直す前。前回の一時のトレース。既定プロジェクトを開いて再生するまで。製品には残していない):

| 値 | FL が呼んだ API と戻り値 | 画面の表示 |
|---|---|---|
| Memory Usage | `GetProcAddress(kernel32.dll, "K32GetProcessMemoryInfo")` が 21 回。戻り値 0(無い) | 上段右「0 MB」 |
| CPU Load | `GetSystemTimes`・`GetProcessTimes`・`GetThreadTimes` は、名前で引かれていない | 下段右「0」(単位なし) |
| Frame Rate | `DwmFlush` が仮想の 60 Hz で約 60 回/秒(1,800 回で仮想時計 30.07 秒)。`GetTickCount`・`QueryPerformanceCounter` は数万回 | 「120」は画面に見えなかった(未確認) |

- ページ側(DPR 1。同じ実行): 無操作は rAF 59.98/秒・提示 0.6/秒。再生中(Space とメトロノーム。音あり)は rAF 59.97/秒・提示 10.87/秒。**エンジンが実際に提示した回数は、再生中でも約 11/秒**。
- 上部バーの位置(1836×1056 の部屋。CSS px): 上段左「0」・上段右「0 MB」・下段右「0」は、おおむね (953〜1087, 40〜68) に在る。
- K32 の行を足した版: 既定プロジェクトを開いた直後にエンジンが trap する(3 回とも同じ)。行を足さない版は trap しない。**診断版では `fle_imp_K32GetProcessMemoryInfo` の先頭に一度も入らなかった**。つまり、K32 の処理の中ではなく、「GetProcAddress が 0 でなく擬似 VA を返した後」の FL の経路で起きている(未確認)。
- 前回の道具の注意: `measure.mjs` の `panelMb` は「MB」の文字の候補の最頻値。候補には設定の窓の一覧(0MB・1GB・1.5GB)も入るので、上部バーの領域で絞る必要がある(未確認)。`cpu.idlePct`・`cpu.playingPct` は null のまま(未実装)。
- 静的な読み(作業環境の `binaries/win64/FLEngine_x64.dll`。git の管理外。ImageBase 0x400000):
  - SetTimer の 500 ms の処理は VA 0x866f90。直接呼ぶのは 0x86bbf0(`[0x15f7190]` が 0 でない時)か、0x86a870 と 0x411fc0。
  - GetTickCount の IAT は 0x1971550。その橋渡し(0x407240・0x423750)を直接呼ぶ関数は、0x86xxxx の範囲に無い。仮想呼び出し(`call *0x288(%rax)` など)の先は未追跡。
  - `DwmFlush` の文字列は UTF-16 で 1 件(ASCII は無い)。遅延 import の表(DWMAPI)には無い。名前で引いている。
  - `K32GetProcessMemoryInfo` の文字列は、`binaries/` の DLL のどれにも無い(ASCII・UTF-16 とも)。`GetProcessMemoryInfo` と `PSAPI` は UTF-16 で各 1 件。FL は名前を組み立てている可能性がある(未確認)。
- 関係する仕組み:
  - `fle_dispatch`(`lift/runtime/fle_dispatch.h`)は、擬似 VA の関数を `fn()` で直接呼ぶ。K32 の行が効けば、FL の呼び出しは `fle_imp_K32GetProcessMemoryInfo` に入るはず。入らなかった理由は未確認。
  - #1354(`pm-work/findings/dpi240-stop-1354.md`): バイト表の飛び(`jmp rax` を `fle_dispatch(R[0]); return;` にした形)が、入れ子の呼び出しの閉路になり、JS のスタックを超えた。直し方は表の注釈(`DSP_IPPV2_2000000`)と、種ファイルの 7 件の削除。同じ記録の §9 の末尾に、未確認の候補が 1 つ残る(`lift/harness/seeds/dsp-ippv2-2000000-seeds-idle1.txt` の `0x27afb9a`〜`0x27afc99` の 8 件)。
  - 前回の trap も、同じ型(閉路の入れ子)かもしれない(未確認。次の「やること」の 2〜3 で確かめる)。

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
- `app/src/engine/worker.ts`(`lagCut`・driver の `idle`・`guestText`・`startFruity` の trap の送り方)・`app/src/engine/client.ts`・`app/src/engine/protocol.ts`・`app/src/main.ts`(`EngineClient`・exit)
- `app/src/perf/beacon.ts`・`app/src/perf/display.ts`・`app/src/perf/display-host.ts`
- `tools/p2-frames/measure.mjs`(提示の数と、ページの実測の方式)・`tools/smoke/welcome-flow.mjs`(起動と測り方)・`tools/p3-audio-probe/measure.mjs`(Playlist の再生の始め方)
- `pm-work/findings/p2-frames-1350.md`(提示の数の測り方)・`pm-work/findings/hidpi-fl-scaling.md`(ゲストのメモリの走査)
- `pm-work/findings/real-user-beacons-2026-11-24.md`(実機の記録)
- 追加: `pm-work/findings/topbar-metrics-1391.md`(前回の記録。作業ツリーに在る)・`pm-work/findings/dpi240-stop-1354.md`(閉路の型。§2・§5・§7・§9)・`lift/crates/fe-decode/src/jumptab.rs`(`DSP_IPPV2_2000000`)・`lift/crates/fe-lift/src/main.rs`(`gen-all` の注釈の連結)・`lift/runtime/fle_dispatch.h`(`fle_dispatch`)

## 数値の定義(PM の決定)

- Memory Usage = エンジンが実際に確保しているゲストのメモリの量(MB)。runtime の VirtualAlloc・HeapAlloc・像の読み込みの積算。ページ側の実測(`WebAssembly.Memory.buffer.byteLength` の和。`performance.measureUserAgentSpecificMemory` が使える時はその値)と比べ、前者が嘘でないことを確かめる。
- CPU Load = エンジンの Worker(音声・計算の thread)が、実時間のうち、実際に計算していた割合(%)。`busyMs` ÷ 経過した実時間。Worker ごとに計り、FL が読んでいる API の形(GetSystemTimes の差など)に合わせる。
- Frame Rate = FL が実際に画面を提示した回数 ÷ 実時間(秒)。実時間は、仮想の時計ではなく、実際の時刻(`performance.now` の相当)で数える。どの形で FL の表示に入れるかは、FL の呼び出しの記録を見て決める。 真の値は、engine の提示(presents)の増分 ÷ 実時間(PM の決定)。ページの rAF(約 60 /秒)は表示の更新の上限(参考値)で、FL の値の基準にしない。

## やること

0. 呼び出しの記録(直す前): 前回の試行で済み。結果は「前回の試行で分かったこと」に書いた。
1. **trap の切り分け**(既定プロジェクトを開いた直後の trap を比べる。版ごとにビルドは 1 回):
   - 版 1(基準): `TABLE` の行を消した版。前回の、行を足さない版(一時のトレースを含む)では trap しなかった。必要なら作り直す。
   - 版 2: 行は残し、`fle_imp_K32GetProcessMemoryInfo` の中身を `R[0] = 0; return;` にした版(FL は呼ぶが、値は無い)。
   - 版 3: 行を残し、実装を入れた版(前回の差分)。②の診断は、この版で取る。
   - 読み: 版 2 も trap する → 値ではなく、擬似 VA を受けた後の FL の経路が原因。版 2 は trap せず、版 3 だけ trap する → 関数の中の処理(計上の値や、呼び出し先)が原因。
2. **スタックの溢れの場所を特定する**(版 3 で。**製品に新しい環境変数・`#ifdef` を足さない**。診断は一時的で、commit しない):
   - trap の時の JS のスタックを、枠の先頭 40 枠と末尾 40 枠で出す。`Error.stackTraceLimit` を大きくして取る(方法は `pm-work/findings/dpi240-stop-1354.md` の §7)。`app/src/engine/worker.ts` の `startFruity` は `result.message` を 1200 字で切って送るので、診断では全部を出す。
   - wasm の枠(`wasm-function[N]`)は `lift/out-app/gen/wasm-func-names.tsv` の index 列で名前を引く(#1354 と同じ)。C の枠(`fle_dispatch` など)はそのまま読む。
   - 繰り返す閉路(周期と回数)を特定し、その閉路が次のどれかで分ける:
     (i) `fle_dispatch` と、`fle_dispatch(R[0]); return;` の飛び元の関数(`f_…`)→ バイト表の閉路(#1354 と同じ型)。
     (ii) `fle_progress_slice`・`fle_loop_slice`・スレッドの切り替えを含む → 切り替えの経路(#{{149}} の領域)。
     (iii) `fle_dispatch` と `fle_imp_*` や擬似 VA の応答だけ → 擬似 VA の呼び出しの再帰。
3. **同じ型の閉路が、他に無いか**: `lift/out-app/gen/src/*.c` で `fle_dispatch(R[0]); return;` の数を数える(注釈の無い `jmp reg` の飛び)。注釈された表は今 `DSP_IPPV2_2000000` の 1 件だけ。種ファイルの `0x27afb9a`〜`0x27afc99`(8 件)は、別の表の候補(未確認)。
4. **直す**: 閉路を断つ。製品の既定の動作として入れる(新しい環境変数・オプトインは作らない)。#1354 と同じ方式(表の注釈と、`goto` による飛び先の切り替え。種ファイルの該当件を消す)か、閉路をループにする。**閉路を隠す回避(深さの上限、要素数の抑え、関数の置き換え)は、制約により採らない**(`pm-work/findings/dpi240-stop-1354.md` の §5)。
5. **Memory Usage**(元の ④): trap が無くなった後、`memory.panelMb` が 0 より大きく、ページ側の実測(`WebAssembly.Memory` の大きさの和)の ±50% に入る(`memory.ratio` が 0.5〜1.5)。候補は、上部バーの領域に絞って読む。
6. **CPU Load と Frame Rate**(元の ⑤。④の後)。式はまだ確定していない。確かめた事実は「前回の試行で分かったこと」にある。次に確かめること:
   - CPU: FL が名前で引く経路は見えていない。「0」の近くの値が、時計(GetTickCount・QueryPerformanceCounter の差)のどれと結び付くかを、トレースで確かめる。
   - Frame Rate: 「120」の出どころ(DwmFlush の回数の 2 倍か)を、DwmFlush の呼び出しの戻り番地から確かめる。定義は「提示の回数 ÷ 実時間」なので、`presentsPerSec` と比べる。
   - 500 ms の処理(0x866f90)の仮想呼び出しの先を、トレースで確かめる。
   - 式と、FL がどの値をどう計算するかを、確かめた順に本文に書き足す。確かめられない物は「未確認」と書く。
   - 仮想の時計(決定 0122)を変える時は、起動と音声への影響を測り、報告に書く。表示のための数字だけを、実時間で割る形もある(時計の挙動は変えない)。どちらを選ぶかは、測定の結果で決める。
7. 数える処理は、音声を止めない(P3)。数えるのは、既存の情報(busyMs・presents)だけ。新しい周期の処理を増やさない。
8. 測定の道具 `tools/topbar-metrics/measure.mjs`(前回の差分に在る。作業ツリーで確かめる)。Playlist を再生する版と、無操作の版の 2 つを取る。Frame Rate は、DPR 1 と、乱数の DPR(1.01〜3.00)の 2 点で取る。ページの rAF の数は、道具の中だけで連続に数えてよい(製品は、既定の burst のまま。決定 0202)。stdout に JSON を 1 行(`rafPerSec` を含むが、判定には入れない)。`panelMb`・`shown` の読みは、まだ確定していない(前回の試行で分かったこと、を見る)。

## 受け入れ

```accept
npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false?0:1)' "$RUNNER_TEMP/boot.json" && node tools/topbar-metrics/measure.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/metrics.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));const s=r.fps.samples;process.exit(r.error===null&&r.memory.panelMb>0&&r.memory.ratio>=0.5&&r.memory.ratio<=1.5&&r.cpu.playingPct>r.cpu.idlePct+1&&Math.abs(r.cpu.playingPct-r.cpu.busyPct)<=10&&s.length===2&&s.every((x)=>typeof x.shown==="number"&&typeof x.presentsPerSec==="number"&&x.presentsPerSec>0&&Math.abs(x.shown-x.presentsPerSec)<=0.15*x.presentsPerSec)&&!s.every((x)=>x.shown===120)?0:1)' "$RUNNER_TEMP/metrics.json" && node tools/smoke/welcome-flow.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/flow.jsonl" && node -e 'const fs=require("fs");const L=fs.readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const F=(f)=>L.find((r)=>r.flow===f);const ok=(r)=>!!r&&r.error===null&&r.state==="engine-idle"&&r.standIn===false&&r.trapped===false&&r.hostErrors===0&&r.opened===true;const pb=Math.max(...L.map((r)=>r.preBootBytes));const el=Math.max(...L.map((r)=>r.elapsedMs));process.exit(L.length===4&&["A","B","C","D"].every((f)=>ok(F(f)))&&F("B").welcomeClosed===true&&F("D").welcomeClosed===true&&el<=12500&&pb<=314385095?0:1)' "$RUNNER_TEMP/flow.jsonl" && npm test --prefix app
```

判定の基準(`tools/topbar-metrics/measure.mjs` の JSON の項目):
- `error`: null。
- `memory.panelMb`: 上部バーの Memory Usage の値(MB。0 より大きい)。`memory.pageBytes`: ページ側の実測(`WebAssembly.Memory` の大きさの和)。`memory.ratio` = `panelMb` × 1,048,576 ÷ `pageBytes`。0.5 以上 1.5 以下。
- `cpu.idlePct`: 無操作の時の CPU Load(%)。`cpu.playingPct`: Playlist を再生している時の CPU Load(%)。`cpu.busyPct`: 再生の時の Worker の忙しい時間の実測(%。Worker の計測。FL の表示とは別の経路)。判定: `playingPct` が `idlePct` より 1 ポイント以上大きい、かつ `|playingPct − busyPct|` が 10 ポイント以下。
- `fps.samples`: 2 点(DPR 1 と乱数の DPR)。各点: `dpr`・`shown`(上部バーの Frame Rate の値)・`rafPerSec`(ページの rAF の数 ÷ 秒。同じ区間。JSON に出すが、判定には入れない)・`presentsPerSec`(engine の提示の増分 ÷ 秒。同じ区間)。判定: 各点で、`shown` が数値、`presentsPerSec` が 0 より大きい数値で、`|shown − presentsPerSec|` が `presentsPerSec` の 15% 以内。2 点とも 120 ではない、という条件は満たさなくてもよいが、2 点とも 120 なら不合格(`!s.every((x)=>x.shown===120)`)。
- `cpu.idlePct`・`cpu.playingPct` が null のままなら、判定は落ちる(未実装のまま通さない)。
- 判定の決定(PM): FL の Frame Rate の真の値は、engine の提示(presents)の増分 ÷ 実時間(FL 自身が画面を更新した回数)。ページの rAF(約 60 /秒)は表示の更新の上限(参考値)で、FL の値の基準にしない。判定は「FL の表示値(`shown`)が、同じ区間の `presentsPerSec` と ±15% で一致」。FL の値が 120 の固定でないこと(DPR 1 と乱数の DPR の 2 点、再生中と無操作で、値が変わりうる)も保つ。前回の実測では、再生中の提示は約 11 /秒。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には「runner の値。この機械とは別枠」と書き、nproc と loadavg を添える。
- 「達成」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。出力の行を付ける。
- 報告に書く: 直す前の値の出どころの表(FL が呼んだ API と戻り値)、直した後の定義と経路、仮想の時計を変えたか(変えたなら起動と音声への影響)、3 つの値の直す前と後、低い値が出た時は、その理由(描画の費用・#1351 との関係)。
- trap が出た時は、「やること」の 1 の版ごとの結果と、2 の閉路(周期・回数・構成の関数)を報告に書く。

## 制約

- 製品に、利用者向けの新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足さない。オプトインを作らない。既存の FLE_DWM_HZ を読むのは可。診断(trap の枠の出力など)は一時的にし、commit しない。
- 数字を作らない。値を良く見せない。低い値が本当なら、そのまま出す。
- ゲストのメモリには書かない(読むだけ)。
- Python は使わない(製品にも、調査にも)。OCR は使わない。
- ベンダー名と Cloudflare の識別子を、ファイルに書かない。
- sleep・timeout コマンド・nohup・末尾の `&` は使わない。一時ファイルは `.tmp-*` に置き、終わったら消す。
- 製品の作業は Actions で行う。この機械で製品のビルドや測定をしない。

## 作業の進め方

- 開始の時刻を、最初に `date -u` で記録する。4 時間 30 分を過ぎたら、受け入れが通らなくても、作業ツリーを commit し、報告を書いて終える。
- 作業ツリーの `.prior-work.md` を最初に読む。前回の差分は未 commit で載っている。その残りを行い、受け入れを通して commit する。
- engine の作り直しは約 35 分。版 2 と版 3 で各 1 回、直した後に 1〜2 回。呼び出しの記録は、1 回のビルドにまとめて取る。
- ビルドは、フォアグラウンドで、シェルツールの `timeout` 引数に 5400000 を渡して待つ。背景で起動して待たない。
- 作業の最後に、受け入れが通らなくても、作業ツリーを commit する。

## 他のタスクとの関係

- blocked_by: #{{139}}(マウスカーソル段 1。同じ engine から page への通知の経路を使うので、直列にする)。
- #1351(P2 の描画の費用。Frame Rate が低いと出た時の原因の側。この Issue では扱わない)。
- #{{138}}・#{{144}} とは、直列の順(138 → 139 → 144・140)。
- 起動の間欠の停止(#{{149}})で、流れ A〜D・起動確認が落ちたら、報告に書いて、commit して終える。再試行で続く。この Issue の依存には足さない。

## 他の Issue との重なり

#{{139}}(マウスカーソル 段 1)と #{{138}}(クリップボード)が、同じ時期に、`app/src/engine/protocol.ts` と `app/src/engine/worker.ts` に、engine から page への通知のメッセージ型を足す。**足す変更どうしなので、`main` への push で衝突したら、両方の型・handler を残して解く**(片方を消さない)。解いた後、`npm test --prefix app` と受け入れを通す。
