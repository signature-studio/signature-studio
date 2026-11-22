---
draft: "110"
title: "P2 を正しく測る: 画面に出た枚数と、入力から画面までの遅れを、決まった条件で測る道具に直す"
type: Task
milestone: "M2 HiDPI と P2 の測定(P2)"
parent: "005"
blocked_by: []
labels: ["perf", "verify"]
size: L
priority: High
project: signature-studio/3
---

## 目的

P2(UI が 120 fps で動き、入力から画面までの p95 が 17 ms 以下)を、正しく測れる道具に直す。#{{007}} の道具と #{{008}} の実測は、画面に出た枚数を数えておらず、入力の遅れも負荷の道具と同じ main スレッドの中で測っていた。そのため、120 fps も入力の遅れも判定できず、差し戻しになった。この Issue は、道具を直し、依頼者の実機の条件(DPR 1.7647。物理画素の画面)で数字を出せるようにする。120 fps と 17 ms の判定は、この Issue では行わない(#{{111}} が行う)。

## 背景

- #{{008}} の結果(`pm-work/findings/p2-measure-1308.md`。runner の値。差し戻し)。
  - 120 fps: `rafFps` 10886.28 は、表示の上限を外した状態で数えた rAF の呼び出しの回数で、画面に出た枚数ではない。画面の 120 fps は測れていない(未確認)。
  - 入力から画面の p95: `inputLatencyP95Ms` 176.225。rAF が main スレッドを使い続ける負荷の道具の下で測ったため、engine の応答の遅れと、負荷の道具の影響が分けられない。
  - 提示 1 回の所要(n = 1。累計 ÷ 提示回数): 拡大率 1(1.14 Mpx)で compose 1.9 ms・put 2.2 ms、拡大率 1.5(2.57 Mpx)で compose 3.2 ms・put 2.4 ms、拡大率 2(4.57 Mpx)で compose 4.9 ms・put 5.0 ms。拡大率 2 では、提示 1 回の compose と put の合計が約 10 ms で、120 fps の予算 8.33 ms を超える。
- 依頼者の実機の画面は 3240×1863 物理画素(6.04 Mpx。DPR 1.7647。部屋は 1836×1056 CSS px)で、拡大率 2 より重い。
- #{{007}} の道具(`tools/p2-frames/measure.mjs`)の欠陥は 3 点ある。
  1. 数えているのは rAF の回数で、提示された枚数ではない。
  2. 入力の遅れを、負荷の道具と同じ main スレッドの中で測っている。
  3. 条件が実機と違う。窓 1920×1080・DPR 1 だけで、入力は File メニューのクリックと Escape の繰り返し。1000 Hz のポインタの入力ではない。
- 依頼者のマウスは 8000 Hz で、互換性のため 1000 Hz まで落として使う。測定のポインタは最低 1000 Hz で動かす(#{{109}} と同じ)。
- runner には GPU が無い(ソフトウェアの WebGL。推定)。そのため、画面に出る 120 fps は、runner では直接測れない可能性がある(推定)。

## 読む物

- `pm-work/findings/p2-measure-1308.md`(#{{008}} の結果と、旧の値)
- `pm-work/findings/survey-c2-worker-split.md`(render Worker の分け方と、旧の測り方)
- `tools/p2-frames/measure.mjs`(#{{007}} の道具。土台にする)
- `tools/feat-c1-hidpi/measure.mjs`(compose と put の所要の測り方)、`tools/feat-c2-render-worker/measure.mjs`
- `tools/boot-check/boot-bundle.mjs`・`tools/boot-check/cdp.mjs`(起動の手順と、Chrome の起動の引数)
- `app/src/engine/desk.ts`(`DeskPresentStats`: presents・rects・pixels・composeMs・putMs・whole)、`app/src/engine/desk-gl.ts`(compose と put の時間の取り方。uploads・uploadBytes・draws)、`app/src/engine/render-worker.ts`(`desk` の統計を送る所、提示の時刻 `presentedAt`、`answers`)
- `app/src/perf/beacon.ts`・`app/src/perf/stats.ts`(`input.eventToRaf`。P2 の入力の数字の定義)、`app/src/perf/display.ts`
- `app/src/input/pointer.ts`・`app/src/engine/client.ts`(入力の送り方)
- `app/src` の描画・入力・提示に当たる物(`git ls-files app/src | grep -i 'render\|input\|perf\|present'` で一覧が出る)

## やること

1. 変更の前に、#{{007}} の道具の各項目が何を数えているかを、コードで確かめ、報告に表で書く(rAF の回数・提示の記録・入力の時刻のどれか)。
2. 画面に出た枚数を数える方式を選ぶ。候補は次の 3 つ。作業者が runner で試し、使える物を選ぶ。選んだ理由と、試して使えなかった理由を報告に書く。
   - a. フレームの制御: Chrome の `HeadlessExperimental.beginFrame`(起動の引数 `--enable-begin-frame-control`)で、仮想の 120 Hz として 1 フレームずつ進め、各フレームの main スレッドと render Worker の所要を数える。
   - b. トレース: `Tracing` で、`DrawFrame`・`Display::DrawAndSwap` などの提示の印を集める。
   - c. engine の提示の記録: `desk` の統計の `presents`(`app/src/engine/render-worker.ts`)と、1 提示あたりの CPU 時間(main スレッドと render Worker)。
3. runner に GPU が無いため、画面に出る 120 fps を直接測れない場合は、代理の指標を出す。1 提示あたりの CPU 時間(compose・put・main スレッドの処理・入力 1 件あたりの engine の周の時間の合計)の分布(p50・p95)を、120 fps の予算 8.33 ms に対する比(`budgetRatioP95`)で出す。代理の指標の意味と限界(実機の GPU では put が軽くなり、compose の一部は GPU 側になる、など)を報告に書く。
4. 入力から画面までの遅れを測る。ポインタを 1000 Hz 以上で送り、各入力が画面の提示に答えられるまでの時間の p50・p95 を出す(提示が答えた入力の印 `answers` などを使う。取り方は作業者が決め、報告に書く)。rAF が main スレッドを使い続ける負荷の道具は使わない(engine の応答の遅れと分けるため)。
5. 測定の条件を固定する。
   - 部屋(表示の領域)が 1836×1056 CSS px になるように窓の大きさを決める。DPR は 1 と 1.7647。
   - スタートの Default のプロジェクトを開いた状態で始める。
   - 負荷 a: Playlist のスクロールとズームに当たる入力を続けて送る。負荷 b: 再生を続け、Mixer のメーターが動いている状態にする(再生を始める方法は、道具が確かめる)。
   - 1 条件 30 秒、n = 3。
   - 引数: `node tools/p2-frames/measure.mjs <bundle の絶対パス> [--dpr <値>] [--load a|b]`。既定は `--dpr 1.7647 --load a`。
6. 出力は、stdout に JSON の 1 行。形は `{"error","dpr","load","presentedPerSec","cpuPerPresentMsP50","cpuPerPresentMsP95","budgetRatioP95","inputToPresentMsP50","inputToPresentMsP95","inputRateOk","valid","notes"}`。`valid` は、次の全てを満たす時だけ true: 提示の枚数を、rAF の回数でなく提示の記録で数えている、1000 Hz 以上の入力が窓の全ての 1 秒で届いた(`inputRateOk`)、提示が 1 件以上。`notes` に、選んだ方式と、入力の届いた頻度を書く。
7. 計測のための追加が要る場合は、既存の perf の仕組み(`app/src/perf/`)に、軽い物だけを足す。

## 受け入れ

```accept
npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false?0:1)' "$RUNNER_TEMP/boot.json" && mkdir -p .tmp-110 && for d in 1 1.7647; do for l in a b; do for i in 1 2 3; do node tools/p2-frames/measure.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" --dpr $d --load $l > ".tmp-110/$d-$l-$i.json" || exit 1; done; done; done && node -e 'const fs=require("fs");const L=[];for(const d of ["1","1.7647"])for(const l of ["a","b"])for(const i of [1,2,3])L.push(JSON.parse(fs.readFileSync(".tmp-110/"+d+"-"+l+"-"+i+".json","utf8").trim()));const num=(x)=>typeof x==="number"&&Number.isFinite(x);process.exit(L.length===12&&L.every((r)=>r.error===null&&r.valid===true&&num(r.presentedPerSec)&&num(r.cpuPerPresentMsP95)&&num(r.inputToPresentMsP95))?0:1)' && rm -rf .tmp-110
```

## 判定の基準

上の node -e が、道具の出力の生の値を読む。DPR 1・1.7647 × 負荷 a・b × n=3 の 12 回の全てで、`error` が null、`valid` が true、`presentedPerSec`・`cpuPerPresentMsP95`・`inputToPresentMsP95` が数字で出ること。120 fps・17 ms・予算の比の目標は、この判定に入れない(#{{111}} が行う)。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には「runner の値。この機械とは別枠」と書き、nproc と loadavg を添える。
- 実測と推定を分ける。「達成」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。出力の行を付ける。
- 報告に書く: 枚数を数えた方式と選んだ理由(使えなかった方式を含む)、道具の各項目が何を数えるかの一覧、代理の指標の意味と限界、DPR 1・1.7647 × 負荷 a・b の表(`presentedPerSec`・`cpuPerPresentMsP50`・`cpuPerPresentMsP95`・`budgetRatioP95`・`inputToPresentMsP50`・`inputToPresentMsP95`・`valid`。n=3 の値を全て)、#{{008}} の値との差(同じ定義の物だけ)、変えたパス(app/・lift/・tools/ を含む。`app/` と `lift/` を変えると main の deploy の仕組みが動く)。

## 制約

- 製品に新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足さない。オプトインを作らない。
- 製品の既定の動作を、測定のためだけに変えない。
- ベンダー名と Cloudflare の識別子を書かない。
- sleep・timeout・nohup・末尾の `&` は使わない。一時ファイルは `.tmp-*` に置き、終わったら消す。
- 製品の作業は Actions で行う。この機械で製品のビルドや計測をしない。

## 他のタスクとの関係

- #{{111}}(描画の所要を変化した面積に比例させる)は、この道具の値で判定する。
- #{{107}}(物理画素の画面)・#{{109}}(入力の合成)と、同じ描画・入力の経路の一部を読む。道具が主なので、同じファイルを変える可能性は低い。衝突したら、作業の最後に `git pull --no-rebase --no-edit` で main を取り込み、衝突を直し、受け入れをもう一度通す。
- ポインタの 1000 Hz の送り方は、#{{109}} の道具と揃えてよい(道具は別に作る)。
