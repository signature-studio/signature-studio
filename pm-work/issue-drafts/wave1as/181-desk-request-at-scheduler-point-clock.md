---
draft: "181"
title: "連続入力の間に描画の要求と仮想時計が止まる: ゲストのスケジューラ地点で描画の要求を受け、描画の通知を 100 ms まとめず、仮想時計を実時間へ合わせる"
type: Task
milestone: "M2 HiDPI と P2 の測定(P2)"
parent: "005"
blocked_by: ["180", "159", "163", "154"]
labels: ["perf", "render"]
size: M
priority: High
project: signature-studio/3
---

## 目的

1000 Hz 級の連続入力の間、ゲストは Worker のイベントループへ戻らずに実行を続ける。そのため、描画の要求(Worker の rAF からの「画面を作って」)が届かず、画面が更新されない。また、描画の通知は 100 ms まとめて送られ、入力から画面までの遅れに通知の待ちが加わる。仮想時計も、idle に戻らない間は進まない。この Issue で、3 つを直し、効果を P2 の指標で測る。

## 背景

- 要求の経路(main の今の版): `app/src/engine/render-worker.ts` の `ask()` は、要求を `toEngine({cmd: 'want'})` で送る。エンジンの Worker(`app/src/engine/worker.ts`)は、その message を、イベントループで受けて初めて描画する。ゲストが実行を続けると、この message は処理されない。
- 通知の経路(main の今の版): `render-worker.ts` の `flushDesk` は、`now - lastReport < 100` の間は通知を送らない(描画済みの通知は最大 100 ms まとめられる)。入力から rAF までの値(`latency`)に、この待ちが加わる。
- ブランチの計測(実ブラウザ・報告の記述): 通知を速やかに送るように直すと、入力→rAF の中央値は 6〜7 ms、p95 は 177・37・36.5 ms(3 回。長い停止は残る)。小規模なプロジェクトの連続入力では描画の更新が大きく増えた(数値の記録は無い)。
- 仮想時計: 連続入力の間は idle に戻らないので、仮想時計が実時間より遅れる。ブランチは、ゲストのスケジューラ地点で、実時間に合わせて仮想時計を進めた(報告の記述では、仮想時間／実時間 ≈ 1)。
- ブランチの写し: `pm-work/findings/perf-windows-browser/patches/01-scheduler-point-requests-clock-canvas-count.patch`(`desk-link.ts`・`render-worker.ts`・`worker.ts`・`idle-driver.ts`・`fruity.ts` の変更)。取り込むのではなく、この 3 点を main の上で 1 つずつ実装し、測ってから残す。canvas の数を数える部分(`displayFrames`)は、この Issue では入れない。

## 読む物

- `app/src/engine/desk-link.ts`(`DeskGate`・`GEN`・`ACK`)、`app/src/engine/render-worker.ts`(`ask`・`frame`・`onFrame`・`flushDesk`)、`app/src/engine/worker.ts`(`inputPoll`・`onRender`・`deskTick`・`sendDeskReady`)
- `app/src/engine/idle-driver.ts`(`target`・`submit`・`started`・`anchor`)、`app/src/engine/fruity.ts`(`fleIdle` と、`ex.fle_g3_*` の受け口。`fle_g3_ticks` は `lift/harness/fruity_idle.c` に在る)
- `app/test/unit/desk-link.test.ts`・`app/test/unit/idle-driver.test.ts`
- `pm-work/findings/perf-windows-browser/patches/01-scheduler-point-requests-clock-canvas-count.patch`(ブランチの版。参考)

## やること

1. 描画の要求をスケジューラ地点で受ける。
   - `desk-link.ts` の `DeskGate` に、要求の旗(`REQUEST`)を足す。`request()` は旗を立て、`takeRequest()` は旗を読んで下ろす(`Atomics.exchange`)。
   - `render-worker.ts` の `ask()` は、`toEngine({cmd: 'want'})` と一緒に `gate.request()` を呼ぶ。
   - `worker.ts` の `inputPoll`(ゲストのスケジューラ地点から呼ばれる)で、旗が立っていれば描画を 1 回行う(`serviceDeskRequest`)。イベントループの `'want'` の受信でも、同じ `serviceDeskRequest` を呼ぶ。旗は 1 回だけ消費する。
2. 描画の通知を、変化の時に速やかに送る。`flushDesk` の 100 ms の集約をやめる。変化の無い通知は、今のとおり 1 秒ごとにする(`presentedAt` が空で `force` でない時)。
3. 仮想時計を実行中に実時間へ合わせる。
   - `idle-driver.ts` に `syncRunningClock()` を足す。idle の中でなく、かつ起動の後なら、`engine.clockTicks()` の値を読み、`target(host.now(), now)` が進んでいれば `clockTo` で進める。
   - `fruity.ts` の `EngineInput` の組み立てに、`fle_g3_ticks` があれば `clockTicks` を足す(無ければ無視する)。
   - `worker.ts` の `inputPoll` で `d.syncRunningClock()` を呼ぶ。
4. 単体の試験を足す(`app/test/unit/desk-link.test.ts`: 旗の消費は 1 回だけ。`idle-driver.test.ts`: 実行中の同期で時計が進み、idle の時は進まない)。

## 受け入れ

```accept
node tools/ab/base.mjs "$(git merge-base HEAD origin/main)" .tmp-base app && npm ci --prefix .tmp-base/app && npm run build --prefix .tmp-base/app && npm run build --prefix app && npm test --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > .tmp-181-boot.json && node -e 'const r=JSON.parse(require("fs").readFileSync(".tmp-181-boot.json","utf8"));process.exit(r.error===null&&r.state==="engine-idle"&&r.standIn===false?0:1)' && node tools/smoke/welcome-flow.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > .tmp-181-flow.jsonl && node -e 'const L=require("fs").readFileSync(".tmp-181-flow.jsonl","utf8").trim().split("\n").map((l)=>JSON.parse(l));const F=(f)=>L.find((r)=>r.flow===f);const ok=(r)=>!!r&&r.error===null&&r.state==="engine-idle"&&r.standIn===false&&r.trapped===false&&r.hostErrors===0&&r.opened===true&&r.elapsedMs<=12500&&r.preBootBytes<=314368711;process.exit(L.length===4&&["A","B","C","D"].every((f)=>ok(F(f)))&&F("B").welcomeClosed===true&&F("D").welcomeClosed===true?0:1)' && node tools/ab/run.mjs p2 "$PWD/lift/out-app/bundle/raw/fl-Oz" "$PWD/.tmp-base/app" "$PWD/lift/out-app/bundle/raw/fl-Oz" "$PWD/app" 3 > .tmp-181-ab-p2.json && node tools/ab/run.mjs p3 "$PWD/lift/out-app/bundle/raw/fl-Oz" "$PWD/.tmp-base/app" "$PWD/lift/out-app/bundle/raw/fl-Oz" "$PWD/app" 3 > .tmp-181-ab-p3.json
```

判定の基準:
- p3 の判定(`tools/ab/gates.mjs`): 変更後の全 run で `error` が null・`playing` が true・`last60sUnderruns` が 0・`inputRateOk` が true・`nonSilentDelta` が 0 より大きい・`pcmRepeat8192` が null か 0.01 未満。加えて、`underrunDelta` の変更後の中央値が基準の中央値以下(絶対の 0 は求めない。起動後の途切れは、別の規則で見る)。
- `npm test --prefix app` が全て通る(新しい試験を含む)。
- 起動の確認(`boot-bundle.mjs`)と流れ A〜D(`welcome-flow.mjs`)の 4 行が全て合格(受け入れのとおり)。
- `tools/ab/run.mjs` の `p2` と `p3` が、`pass` が true で終わる(回帰が無い)。`p2` の判定には、`minWindowFps` の中央値 ≥ 基準 × 0.90 と、`inputToPresentMsP95` の中央値 ≤ 基準 × 1.20 が入る。
- 効果の判定(この Issue の残す条件): `p2` の `minWindowFps` の変更後の中央値が、基準の 1.10 倍以上。届かないなら、この 3 点の変更を取り消し(製品の変更を残さない)、測定の表だけを `pm-work/findings/perf-windows-browser/181-result.md` に書いて commit する。受け入れのコマンドは、取り消し後も同じに通る(基準と変更後が同じになる)。
- 報告に書く: 6 回の run ごとの `minWindowFps`・`inputToPresentMsP95`・`inputRateOk`・`p3` の `pcmRepeat8192`・`underrunDelta`、中央値、判定(残す・取り消す)。「達成」と書かず、「受理」か「差し戻し」を、出力の行で書く。

## 前回の試行で分かったこと

- まだ起動していない(依存の Issue が閉じるのを待っている)。
- 基準の準備: `node tools/ab/base.mjs` で基準の複製を作る(git の追跡のファイルと、読むだけの入力 `lift/out-app/host` などのリンク)。この Issue の基準は `app` だけ。engine は変えないので、基準と変更後の bundle は同じ。
- `underrunDelta` は、中央値の相対で判定する(上の判定の式)。基準の側で 0 にならない負荷の runner でも、比べられる。

## 作業の進め方

- engine のビルドは約 35 分。フォアグラウンドで、シェルツールの `timeout` 引数に大きい値(例 3600000。上限 5400000)を渡して、直接実行する。背景で起動して、終わるのを待たない。
- A/B の run は 12 回(p2 と p3 で各 6 回)。1 回ずつ、結果を確かめてから次へ進む。
- 作業は開始から 4 時間 30 分以内に終える。最初に `date -u` で開始を記録する。その時刻を過ぎたら、受け入れが通っていなくても作業ツリーを commit して、報告を書いて終える。
- `tools/ab/run.mjs` は #{{180}} で作る。この Issue の作業の前に、#{{180}} が main に入っていること。

## 制約

- 製品に、利用者向けの新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足さない。描画の要求の旗は、既定の動作として入れる。オプトインを作らない。
- Python を使わない。OCR を使わない。
- 禁止されているのは、AI のベンダー名と Cloudflare の識別子だけ。FL の製品名・インストール先の名前・変数名は、書いてよい。
- sleep・nohup・末尾の `&` は使わない。一時ファイルは `.tmp-*` に置き、終わったら消す。
- `app/` の変更は、push で main の `deploy.yml` を動かす。報告に書く。

## 他のタスクとの関係

- blocked_by: #{{180}}(計測の道具)。#{{159}}(上部バーの Frame Rate。`idle-driver.ts` と `worker.ts` を触る)。#{{163}}(クリップボード。`worker.ts` を触る)。#{{154}}(言語の起動。`worker.ts` を触る)。
- `pm-work/findings/perf-windows-browser/README.md` の表の 1・2・3(描画要求・通知・仮想時計)。
- 衝突: `app/src/engine/worker.ts` を触るので、他の作業と衝突しうる。衝突したら、作業の最後に `git pull --no-rebase --no-edit` で main を取り込み、直して、受け入れをもう一度通す。
