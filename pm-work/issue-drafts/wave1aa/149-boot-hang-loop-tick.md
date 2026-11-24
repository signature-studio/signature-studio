---
draft: "149"
title: "起動が first idle に届かず state running のまま止まる(間欠・DPR 2.5 で 5 回中 4 回)を、再現して直す: 後ろ向きの goto の切り替え点(fle_loop_tick・fle_loop_slice)"
type: Task
milestone: "M1 描画と音声の分離(P3)"
parent: "001"
blocked_by: []
labels: ["audio", "perf", "verify"]
size: L
priority: Urgent
project: signature-studio/3
---

## 目的

起動の途中で、ページが engine の first idle に届かず、`state` が running のまま止まる障害を直す。この障害は、間欠に、特に起動前の画面の大きさ(DPR)が高い時に起きる。

この Issue の目標は次の 3 つ。
1. 再現の道具 `tools/boot-hang/measure.mjs`(この Issue で作る)で、止まりの率を測れること。
2. 止まりの原因を、切り替え点(`fle_loop_tick`・`fle_loop_slice`)の仕組みの中で特定し、直すこと。
3. #1352 段 A の効果(起動直後の音声の途切れを減らすこと)を保つこと。

## 背景

- 事実 1(ある作業者の試行。HEAD の engine。DPR 2.5 = DPI 240): 起動が 180 秒経っても engine-idle に届かなかった。5 回中 4 回が止まった。止まった時は、engine のログが 0 行、engine のメモリの値が null、ページの例外と console の出力が 0。DPR 3(DPI 288)は 9 回中 1 回が止まった。
- 事実 2(同じ試行): 起動の app の変更を HEAD に戻した版でも、DPR 2.5 は 3 回中 3 回止まった(同じ engine で、計 8 回中 7 回)。app は原因ではない。
- 事実 3(同じ試行。実験。製品には入れていない。取り消し済み): `lift/runtime/fle_thread.c` の `fle_loop_slice` を何もしない版に作り直すと、DPR 2.5 は 3 回中 3 回 engine-idle に届いた(採用段は 250%)。
- 事実 4(判定の段の 1 回。ある作業者の run): DPR 3 は 180 秒で止まった。乱数の DPR 2.4 は、CDP の `Runtime.evaluate` が 30 秒で timeout した(ページの JS が返らない。engine の状態を読めない)。DPR 2.14 の起動は通った。止まり方は 2 種類あり、どちらも起きている(engine の状態が running のまま止まる。ページの JS が返らない)。
- 事実 5(別の作業者の run): 流れ A〜D(`tools/smoke/welcome-flow.mjs`)を 2 回実行し、1 回目は 2 流れ、2 回目は 1 流れが、180 秒で `no engine-idle within 180000 ms (state running)` になった。起動の前に viewport を変える起動(`VIEW_START`)でだけ観測。viewport を変えない起動は、4 回続けて 7.1 秒で成功。
- 事実 6(別の作業者の run): 判定の `boot-bundle` が `no first idle, error or exit within 180000 ms (state running)` で落ちた。流れ A が `timeout 20000 ms: Runtime.evaluate` で 1 回落ちた(B〜D は通った)。
- 事実 7(実機): 実機のビーコンに、first idle の前に利用者がページを離れた記録が 1 件ある。
- 事実 8(#1354): DPI 240 以上で起動しない障害は、#1354 で直った(再帰の閉路を断つ、jump table の注釈。注釈は今も在る)。しかし、#1352 段 A(commit `eb118ef2`)で、この修正の効果が無効になった(事実 1 と 3 の通り)。
- 事実 9(#1352 段 A。`pm-work/findings/boot-burst-1352.md`): 後ろ向きの goto の前に `fle_loop_tick()` を置き、1024 回ごとに `fle_loop_slice()` で準備完了のスレッドへ切り替える。その結果、`bootLast60sAt10s` は、基準の 2,071・2,105・2,003 から、4・4・25 に下がった。この効果は保つ。
- 事実 10(#1375。段 B): 段 B の受け入れは、同じ切り替えの機構の上で通った(`bootLast60sAt10s` 0)。段 B は、この Issue が直った後に、もう一度 main で確かめる。
- 仕組み(コードの読み): `lift/runtime/fle_dispatch.h` の `fle_loop_tick()` は、`fle_loop_count` を増やし、`fle_loop_limit` に届くと `fle_loop_slice()` を呼ぶ。`lift/runtime/fle_thread.c` の `fle_loop_slice()` は、`preempt_armed` と `nthr` を見て、現在のスレッドが実行中(`T_RUNNING`)なら `T_READY` にして `schedule_ex(1)` を呼ぶ。深さが `FLE_PREEMPT_MAX_DEPTH` を超える時(窓の手続きの中)は、切り替えを飛ばし、音声の補充だけ行う。仮想時計は進めない。
- 切り替え点の入り方: `lift/crates/fe-lift/src/lib.rs`(後ろ向きの goto を出す所。`fle_loop_tick(); goto L_...;` の形)と `lift/crates/fe-lift/src/int_more2.rs`(同じ形の別の経路)。段 A の commit は `git show eb118ef2 --stat` で確かめる。

推定(実測で確かめる): 切り替えが、起動のある区間で、切り替えてはいけない状態(例: 共有の表を書きかけの間、ロックを持つ間、`setjmp`・`longjmp` の間、例外の処理の間、呼び出しの途中の状態を持つ間)で起きている。または、切り替えた先のスレッドが、切り替え元の状態を待つ形で、戻ってこない。DPR が高いと、同じ起動の処理でループの回数が増え、切り替えの回数が増える。これが止まりの率と、DPR の関係として見える(推定)。

## 読む物

- `lift/runtime/fle_dispatch.h`(`fle_loop_tick`・`fle_loop_count`・`fle_loop_limit`。38〜46 行と 186〜196 行)
- `lift/runtime/fle_thread.c`(`fle_progress_slice`・`fle_loop_slice`(805 行付近)・`schedule_ex`・`preempt_armed`・`fle_callback_depth`)
- `lift/runtime/fle_thread.h`(切り替えの規則と定数。90〜95 行付近)
- `lift/crates/fe-lift/src/lib.rs`(`fle_loop_tick` を出す所。1000 行付近)・`lift/crates/fe-lift/src/int_more2.rs`(280 行付近)・`lift/crates/fe-wasmgen`
- `git show eb118ef2 --stat` と `git show eb118ef2 -- lift/runtime`(段 A の差分)
- `lift/harness/web/`(起動の経路の設定)・`app/src/engine/`(起動の状態の読み)
- `pm-work/findings/boot-burst-1352.md`・`pm-work/findings/boot-stall-1334.md`(計数の検証)
- `tools/p3-audio-probe/measure.mjs`(`--boot`・`--profile`。CDP の Profiler の使い方)・`tools/p3-audio-probe/profile-names.mjs`(wasm-function の番号から名前へ)
- `tools/smoke/welcome-flow.mjs`(`VIEW_START` の使い方)・`tools/boot-check/boot-bundle.mjs`(`elapsedMs` の定義)
- 生成物 `lift/out-app/gen/`(関数名の表。ビルドで作る。配信の対象に入れない)

## やること

0. 作業の最初に `date -u` を記録する。
1. **再現の道具 `tools/boot-hang/measure.mjs`(新規。製品の既定の動作は変えない)**
   - 引数: `<bundle の絶対パス>` と `--dprs 1,2.5,3`(固定の値)と `--random LO,HI,COUNT`(乱数。繰り返し指定できる。区間の中の一様な乱数を、起動のたびに選ぶ。値は `randomDprs` に出す)と `--n N`(各 DPR の起動の回数)。
   - 各起動: 起動の前に viewport(DPR を含む)を変える起動と、変えない起動を、交互に行う(`viewStart` に出す)。first idle まで待つ。上限は 60 秒(180 秒は待たない)。
   - 止まった時は、止まった瞬間の状態を取る: ページの `engine.state`、engine の Worker の状態(`fleAudioWorkerState`)、engine のログの末尾、CDP の `Profiler` で止まった Worker の 3 秒間のプロファイル(`topFunctions`。名前は `lift/out-app/gen/` の表で引く)、そして `busy`(CPU を使い続けている)か、待っている(`Atomics.wait` などで動いていない)かの判定。ページの JS が返らない時は、その旨を `stalled` の理由に書く。
   - 出力は JSON 1 行: `{"runs":[{"dpr","randomDprs"?,"viewStart","ok","firstIdleMs","stalled","busy","topFunctions"}],"hangRate":{"total","hung","rate"}}`。`hung` は `ok` が false の起動の数。
2. **切り分け(この Issue の最初の実験)**
   - (i) 今の main の版で、`tools/boot-hang/measure.mjs` を DPR 1・2.5・3 と乱数 2 点(1.01〜2.24)・乱数 2 点(2.25〜3.00)で、各 N=10。止まりの率と、止まった時の `busy`・`topFunctions` を取る。
   - (ii) `fle_loop_slice` を何もしない版(実験用の出力。`lift/out-app/` の下の別の名前、または `fe-build` の引数で作る。**製品に新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足さない**)で、同じ測定を行う。
   - 両方の結果を、報告に表で書く。
3. **原因の特定**: 止まった起動の `topFunctions` と、切り替えの記録(切り替えの回数、切り替え元と切り替え先の番号、`fle_callback_depth`、`preempt_armed`、準備完了のスレッドの状態)を、engine のログか、その場の計数で取り出す。どの区間で、切り替えが何を壊すか(上の推定の候補の、どれか)を特定する。候補ごとに、確かめた事実と、確かめていない事実を分けて書く。
4. **直す**: 原因に合わせて、最小の変更で直す。
   - 切り替えを外すだけの修正は採らない(#1352 段 A の効果を失うため)。切り替えは残す。
   - 切り替えてはいけない区間で切り替えないようにする、切り替えた先が戻れるようにする、などの形で直す。
   - 直した後も、#1352 段 A の効果を保つ(下の 5 の `boot` の基準)。
   - 製品に新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足さない。特定の DPR や DPI の値を、runtime・app・seed・テストに、定数・表・分岐として入れない。
5. **受け入れの前に、実験の変更を取り除く**: 実験(ii)の変更は、`lift/runtime/fle_thread.c` を HEAD に戻す。計測のための読むだけの関数は残してよい。
6. 作業の最後に、受け入れが通らなくても、作業ツリーを commit する。

## 受け入れ

```accept
npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false&&r.elapsedMs<=12500?0:1)' "$RUNNER_TEMP/boot.json" && node tools/boot-hang/measure.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" --dprs 1,2.5,3 --random 2.25,3,2 --random 1.01,2.24,2 --n 5 > "$RUNNER_TEMP/hang.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));const ms=r.runs.filter((x)=>x.ok).map((x)=>x.firstIdleMs).sort((a,b)=>a-b);const p95=ms.length?ms[Math.ceil(0.95*ms.length)-1]:Infinity;process.exit(r.runs.length===35&&r.hangRate.total===35&&r.hangRate.hung===0&&r.runs.every((x)=>x.ok===true)&&p95<=12500?0:1)' "$RUNNER_TEMP/hang.json" && node tools/smoke/welcome-flow.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/flow.jsonl" && node -e 'const fs=require("fs");const L=fs.readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const F=(f)=>L.find((r)=>r.flow===f);const ok=(r)=>!!r&&r.error===null&&r.state==="engine-idle"&&r.standIn===false&&r.trapped===false&&r.hostErrors===0&&r.opened===true;process.exit(L.length===4&&["A","B","C","D"].every((f)=>ok(F(f)))&&F("B").welcomeClosed===true&&F("D").welcomeClosed===true&&Math.max(...L.map((r)=>r.elapsedMs))<=12500&&Math.max(...L.map((r)=>r.preBootBytes))<=314368711?0:1)' "$RUNNER_TEMP/flow.jsonl" && for i in 1 2 3; do node tools/p3-audio-probe/measure.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" --boot > "$RUNNER_TEMP/boot-$i.json" || exit 1; done && node -e 'const fs=require("fs");const L=[1,2,3].map((i)=>JSON.parse(fs.readFileSync(process.argv[1]+"/boot-"+i+".json","utf8").trim()));process.exit(L.length===3&&L.every((r)=>r.error===null&&r.state==="engine-idle"&&r.standIn===false&&r.ctxState==="running"&&r.quantaDelta>=19000&&r.bootLast60sAt10s!==null&&r.bootLast60sAt10s<=30&&r.bootLast60sAt70s===0&&r.bootLast60sAt90s===0)?0:1)' "$RUNNER_TEMP" && npm test --prefix app
```

判定の基準(道具の出力の生の値を、上の `node -e` が読む):
- `boot.json`: `error` が null、`state` が engine-idle、`standIn` が false、`elapsedMs` が 12,500 以下。
- `hang.json`: `runs` が 35 件(DPR 1・2.5・3 と乱数 4 点の、各 5 回)、`hangRate.total` が 35、`hangRate.hung` が 0、全ての `ok` が true。`ok` の起動の `firstIdleMs` の 95 パーセンタイル(小さい順で、`ceil(0.95 × 件数) − 1` 番目)が 12,500 以下。
- 流れ A〜D(`tools/smoke/welcome-flow.mjs`): 4 行。各行の `error` が null、`state` が engine-idle、`standIn` が false、`trapped` が false、`hostErrors` が 0、`opened` が true。流れ B と D は `welcomeClosed` が true。全行の `elapsedMs` が 12,500 以下。`preBootBytes` の最大が 314,368,711 以下。
- `measure.mjs --boot` の 3 回の全て: `error` が null、`state` が engine-idle、`standIn` が false、`ctxState` が running、`quantaDelta` が 19,000 以上、`bootLast60sAt10s` が 30 以下、`bootLast60sAt70s` と `bootLast60sAt90s` が 0。
- 単体テスト(`npm test --prefix app`)が通る。

注: 受け入れの 35 起動の `hangRate` が 0 である確かさは、N=5 では下がる(N=10 なら 70 起動。約 40 分の測定を、判定の段でもう一度走らせるため、N=5 にした)。止まりの率の確かな測定は、この Issue の 2 の (i)(ii) で N=10 を行い、報告に書く。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には「runner の値。この機械とは別枠」と書き、nproc と loadavg を添える。
- 実測と推定を分ける。「達成」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。出力の行を付ける。
- 報告に書く: (i) と (ii) の表(DPR・乱数の値・N・止まりの数・止まった時の `busy` と `topFunctions` の上位 5 件)、切り替えの記録(止まった起動で切り替えた先と、切り替えの計数)、特定した原因(実測と推定を分けて)、直した箇所と理由(切り替えを残した理由)、`boot` の値(`bootLast60sAt10s`・`bootLast60sAt70s`・`bootLast60sAt90s`。n=3)、`elapsedMs` の値、`preBootBytes` の値、流れ A〜D の 4 行、変えたパス(`app/`・`lift/`・`tools/` を含む。`app/` と `lift/` を変えると main の deploy が動く)。
- 「直った」と書かず、(i)(ii) と受け入れの結果で書く。

## 制約

- 製品に新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足さない。オプトインを作らない(計測の切り替えも含む)。
- 製品の既定の動作を、測定のためだけに変えない。
- 切り替えの計数の粒度(`FLE_LOOP_SLICE_BACKEDGES`)と、仮想時計の進め方(`FLE_PREEMPT_SLICE_TICKS`)は、変える場合に限り、仮想時計の進む速さの比を保つ。変えたら、報告に書く(決定 0147・0279 との関係も書く)。
- 音声の先読み(`FLE_AUDIO_LEAD_MS`)は変えない。
- ベンダー名と Cloudflare の識別子を書かない。R2・アカウント ID・ホスト名を出さない。
- sleep・timeout・nohup・末尾の `&` は使わない。一時ファイルは `.tmp-*` に置き、終わったら消す。
- Python は製品に使わない(調査と検証にだけ使う)。OCR を使わない。
- 製品の作業は Actions で行う。この機械で製品のビルドや測定をしない。

## 作業の進め方

- engine の作り直しは約 35 分。ビルドは、受け入れの 1 回を含めて、3 回まで(1 回目: 2 の (i) と (ii) の両方の版。2 回目: 直した版。3 回目: 2 回目の結果の修正が必要な時だけ)。
- ビルドと受け入れは、フォアグラウンドで、シェルツールの `timeout` 引数に大きい値(例 5400000。上限)を渡して直接実行する。背景で起動して待たない。
- 受け入れの 1 回は約 70〜90 分(ビルド約 35 分、`hang` の測定の最大約 35 分、`boot` の測定 3 回、流れ A〜D)。
- 開始から 2 時間 30 分を過ぎたら、新しい受け入れを始めない。開始から 4 時間 00 分を過ぎたら、受け入れが通っていなくても、作業ツリーを commit して、報告を書いて終える(ジョブの上限は 5 時間 50 分)。
- blend2d の取得(GitHub)が 403 で落ちたら一時的。受け入れのコマンドは `lift/out-app/b2d-src` を消してから作り直すので、そのまま再試行する。
- 作業の最後に、受け入れが通らなくても、作業ツリーを commit する。未 commit の変更が残ると、判定は失敗する。

## 他のタスクとの関係

- blocked_by: なし。
- #1348(DPR 2.5・3 と、動的な変化の確認)は、この Issue の後に再開する。#1348 は DPR 2.5 の起動が直るまで、DPR 2.5 の行を測れない。
- #1375(段 B。起動直後の途切れを 0 にする)は、同じ切り替えの機構の上で動いている。#1375 の変更と、この Issue の変更が同じ箇所(`fle_thread.c`・`fe-lift`・`fle_dispatch.h`)を触る場合は、先に main に入った方が勝ち、後の方は `git pull --no-rebase --no-edit` で取り込み、衝突を直し、受け入れをもう一度通す。
- #1352(段 A)の効果は、この Issue の判定で保つ(上の 5 の `boot` の基準)。
- #1354 の修正(jump table の注釈)は残す。この Issue は、その注釈を外さない。
