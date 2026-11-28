---
draft: "262"
title: "起動の間のスプラッシュの提示が約 2 fps しか出ない: 毎秒の提示の数を測り、60 fps 以上に上げる"
type: "Task"
milestone: "M2 HiDPI と P2 の測定(P2)"
parent: "005"
blocked_by: []
labels: ["hidpi", "verify"]
size: "L"
priority: "High"
project: "signature-studio/3"
---

## 目的

起動の間(モーダルを消した後から最初の idle まで)の、FL のスプラッシュ画像の提示が、毎秒 2 回程度しか出ていない。依頼者の指摘: 「起動時/読み込み時のスプラッシュ画像がいつも 2fps くらいでしか動いてない」。

この Issue の目的は 2 つ。
1. 起動の間の提示の毎秒の数を E2E で測り、直す前の値を記録して、受け入れに入れる。
2. 起動の間の提示を、通常の UI の更新並み(60 fps 以上。目標 120)に上げる。スプラッシュの画像は、ページが動かす・描く形にしない(FL の表示のコードを、そのまま走らせる)。

## 背景

- 前回の #1520(起動の順序。閉じた)は、「最初の idle の前に、スプラッシュの画素が canvas に出る」だけを受け入れた。提示の頻度は測っていなかった。
- 起動の間は、画面の合成と提示を、エンジンのスケジューラの切り替え点(`app/src/engine/worker.ts` の `inputPoll` → `deskPoll`)から行う。描画の担当(render Worker)の問いは、エンジンが idle になるまで答えられない。
- 読みの結論(推定。実測は無い。詳しくは `pm-work/findings/splash-fps.md` の 1 節と 5 節):
  - H3: `deskPoll` の間隔が `DESK_POLL_MS = 50`(`app/src/engine/worker.ts` 546 行)で、提示は最大 20 回/秒。受け入れの 30 fps は、この上限のままでは満たせない。
  - H2: エンジンの切り替え点は、間接呼び出し 4096 回ごと・後退辺 1024 回ごとで、時間では決まらない(`lift/runtime/fle_dispatch.h` 34・40〜41 行、`lift/runtime/fle_thread.h` 93・98 行)。呼び出しの深さ 2 以上(窓の手続きの中)では、切り替えも仮想時計の前進も無い(`lift/runtime/fle_thread.c` 878〜880 行)。
  - H1(最有力): 仮想時計は、起動の間(最初の idle の前)は、間接呼び出しの進み(4096 回ごとに 0.25 ms)でしか進まない(`lift/runtime/fle_thread.c` 885 行、`lift/runtime/fle_sys.c` の仮想時計)。壁時計に合わせるのは idle の処理だけで、その anchor は最初の idle で作られる(`app/src/engine/idle-driver.ts` 163 行)。スプラッシュの窓の更新の周期が仮想時計の上にあるなら、更新は壁時計で遅くなる。
  - 確認できない: FL のスプラッシュの描画の周期(`DwmFlush`・`Sleep`・タイマーのどれか。逆アセンブルは、この機械には無い)。

## 読む物

- `app/src/engine/worker.ts`: `inputPoll`(233 行)、`answerInline`(281 行)、`onEngineIdle`(301 行)、`sendDeskReady`(456 行)、`onRender`(465 行)、`deskTick`(491 行。`gate.busy` は 494 行)、`sceneOn`(538 行)、`DESK_POLL_MS` と `deskPoll`(546〜559 行)、`hzWanted`(172・205 行)。
- `app/src/engine/render-worker.ts`: `ask`(55 行)、`frame`(67 行)、`startDesk`(138 行)、`onFrame`(147 行。`presenter.present()` 155〜163 行、`gate.ack` 167 行)、`flushDesk`(174 行)。`app/src/engine/desk-link.ts`(`busy`・`presenter`)、`app/src/engine/desk.ts` 112 行と `desk-gl.ts` 271 行(`presents` の数え方)。
- `app/src/engine/idle-driver.ts`: `onIdle`(158 行。anchor は 163 行)、`target`(181 行)、`arm`(193 行)、ヘッダの注釈(壁時計の規則)。
- `app/src/main.ts`: `window.__app = app`(220 行)、`desk(m)`(689〜691 行。`deskStats`)、`app.idles.push`(723 行)、`timeline`(67 行の注釈。`performance.now()` 基準)。
- `lift/runtime/fle_thread.c`: `schedule_ex`(687 行。`fle_input_ring_poll` は 694 行)、`fle_thread_sleep`(843 行)、`fle_progress_slice`(870〜888 行。深さの判定 878 行)、`fle_loop_slice`(893 行)、`preempt_armed = 1`(1010 行)、`FLE_DWM_HZ` の既定(1034〜1042 行)、`fle_thread_next_frame_tick`(1081〜1087 行)、`fle_imp_DwmFlush`(1089 行)。
- `lift/runtime/fle_thread.h`: 84〜98 行(切り替えの規則と定数)。`lift/runtime/fle_sys.c`: 仮想時計(48〜66 行)、`Sleep`(86〜99 行)。`lift/runtime/fle_sys.h`: `FLE_CLOCK_READ_TICKS`・`FLE_CLOCK_START_MS`(21〜22 行)。`lift/runtime/fle_dispatch.h`: 34・40〜41・226・234〜235 行。
- `lift/crates/fe-lift/tests/dwm_timing.rs`(仮想時計の試験。直す前後で比べる)。
- `pm-work/findings/splash-fps.md`(この Issue の読み。実測と推定を分けてある)、`pm-work/findings/startup-splash-1520.md`(前回の読みと受け入れ)。
- `tools/e2e/scenarios/page/startup-sequence.mjs`(`RECORDER` と、起動の順序の判定。型を流用する)、`tools/p2-frames/measure.mjs`(`window.__app.deskStats.presents` を 1 秒ごとに読む `minWindowFps` の数え方)。`tools/e2e/lib/browser.mjs` の rAF 計数は、ページの描画の数で、提示の数ではない(使わない)。

## やること

0. 作業の最初に `date -u` を記録する。`.prior-work.md` が在れば、最初に読む。

1. **測る(先に赤)**。新しい E2E `page/startup-splash-fps`(`tools/e2e/scenarios/page/startup-splash-fps.mjs`)を書く。
   - 宣言: `covers: ['page/start/splash-presents']`(`tools/e2e/features/page.tsv` に、この feature の行を足す)、`displays: ['4k-dpr1', '4k-dpr2', '4k-random']`。
   - 記録: ページで rAF ごとに `(performance.now(), window.__app.deskStats.presents)` を溜める(`startup-sequence.mjs` の `RECORDER` と同じ型)。`deskStats` は描画の担当が 100 ms に 1 回送る統計なので、1 秒の窓の値は ±1 報告(約 100 ms)の誤差を持つ。
   - 窓: `window.__app.timeline.first_pixel`(起動の最初の提示)から `timeline.idle`(最初の idle)まで、1 秒ごとに区切る。各窓の値 = その窓の提示の増分 ÷ 窓の長さ(秒)。
   - 結果(`results` に入れる): `median`(窓の中央値)、`min`(最小の窓)、`windows`(窓の数)、`windowSeconds`(窓の全長)。窓の全長が 3 秒未満なら、測れないので `ok` を false にする。`ok` は、起動が engine-idle に届き、窓が 3 以上あり、`median` が 30 以上、`min` が 15 以上のとき true。
   - 読み取る値(`results` に入れる): `window.__app.idles[0]` の `now`(仮想 ticks)と `at`、`dwm`(その時点までの DwmFlush の数)、`timeline.engine_start`。仮想時計と壁時計の比(起動の間)は、`(idles[0].now / 10000 − 3600000)` ms と、`idles[0].at − performance.timeOrigin − engine_start` ms の比で出す(基準を揃える)。
   - 赤の値を報告に書く(直す前の、4k-dpr1 と 4k-dpr2 の `median`・`min`・`windows`・`windowSeconds`・比・`dwm`)。

2. **決める(直す前に)**。赤の値から、読みの H1〜H4(`pm-work/findings/splash-fps.md` の 5 節)のどれが効いているかを決め、表で報告に書く。
   - 仮想/壁の比が 1 より十分小さい(目安 0.1 未満)なら H1。
   - 比が高いのに提示が少ないなら、切り替えの数(`deskPoll` の呼び出し数)と compose の数(変化が無かった数)を見る。数が足りないなら H2。数が足りるなら、H4(スプラッシュ自身の周期)の疑い。
   - 提示が 20 回/秒の近くに張り付くなら H3。
   - 数が要る時は、エンジン側の数(`deskPoll` の呼び出し・compose・変化があった compose)を、既存の統計の経路で描画の担当の統計に足す(オプションにしない。既定で入れる)。

3. **直す**(決めた仮説に合わせる。複数を組み合わせてよい)。
   - (a) `DESK_POLL_MS` の 50 ms を、時間の上限へ下げる(目安 8 ms)。変化の無い compose は提示しない(既存の `deskTick` の規則のまま)。
   - (b) 起動の間も仮想時計を壁時計に合わせる(H1 の時)。anchor を、起動の始め(エンジンが走り出した時)に作る。切り替え点(間接呼び出しの切り替えと後退辺の切り替えの両方)で、壁時計の目標まで仮想時計を進める。期限を越えない。仮想時計を戻さない。idle の `target` の上限・下限(maxLag・maxLead)と同じ規則を使う。
   - (c) 切り替え点を時間でも保つ(H2 の時)。後退辺の切り替え(1024 回)に加え、一定の壁時計の経過ごとにも切り替える。
   - 呼び出しの深さ 2 以上の規則(切り替えも時計も止める)は、変えない。深さ 2 以上の区間が起動の間に長く続くなら、その長さを報告に書く。
   - スプラッシュの窓の手続きを、ページ側で描かない。画像を読まない・描かない・動かさない(依頼者の明示の禁止)。
   - P1: 起動の `elapsedMs` が、直す前より 5 % を超えて増えない(起動の確認の `boot.json` の値を、直す前後で報告に書く)。
   - P3: 仮想時計の進め方を変えるので、音の E2E(`tools/e2e/scenarios/audio/gap-free-playing-5s.mjs` と `fill-margin-playing-8s.mjs`)を、直す前後で走らせ、`underrun` の数が増えないこと(数を報告に書く)。
   - 仮想時計の試験(`lift/crates/fe-lift/tests/dwm_timing.rs`)を、直す前に走らせて、落ちているものを報告に書く。直した後に、増えた失敗が無いこと。

4. **受け入れ**。下の式を通す。

## 受け入れ

```accept
npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false&&r.elapsedMs<=20000?0:1)' "$RUNNER_TEMP/boot.json" && node tools/smoke/welcome-flow.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/flow.jsonl" && node -e 'const fs=require("fs");const L=fs.readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const F=(f)=>L.find((r)=>r.flow===f);const ok=(r)=>!!r&&r.error===null&&r.state==="engine-idle"&&r.standIn===false&&r.trapped===false&&r.hostErrors===0&&r.opened===true;process.exit(L.length===4&&["A","B","C","D"].every((f)=>ok(F(f)))&&F("B").welcomeClosed===true&&F("D").welcomeClosed===true?0:1)' "$RUNNER_TEMP/flow.jsonl" && npm test --prefix app && (cd tools/e2e && npm ci && npx playwright install chromium) && node --test tools/e2e/test/*.test.mjs && node tools/e2e/run.mjs --only page/startup-splash-fps --displays 4k-dpr1,4k-dpr2 --budget 40m --report-only > "$RUNNER_TEMP/splash.jsonl" && node -e 'const L=require("fs").readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const S=L.flatMap((r)=>r.scenarios);process.exit(L.length===2&&L.every((r)=>r.error===null)&&S.length===2&&S.every((s)=>s.id==="page/startup-splash-fps"&&s.ok===true&&s.error===null&&s.skipped!==true&&s.results&&s.results.median>=30&&s.results.min>=15)?0:1)' "$RUNNER_TEMP/splash.jsonl" && bash guard/scan.sh
```

判定の基準:
- 起動確認(`boot.json`)が engine-idle・`standIn` が false・`elapsedMs` が 20,000 以下。
- 流れ A〜D の 4 行が、`error` が null・engine-idle・`standIn` が false・`trapped` が false・`hostErrors` が 0・`opened` が true。B と D は `welcomeClosed` が true。
- `npm test --prefix app` と `node --test tools/e2e/test/*.test.mjs` が通る。
- `page/startup-splash-fps` が、2 行(4k-dpr1・4k-dpr2)すべてで、`ok` が true・`error` が null・`results.median` が 30 以上・`results.min` が 15 以上。
- `bash guard/scan.sh` の指摘が 0 件。

目標は 60 fps 以上(120 に近いほど良い)。受け入れは最低限の値。直した後の `median` と `min` の値(両方の表示)を、報告に必ず書く。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 報告に書く: (a) 直す前の値(`median`・`min`・`windows`・`windowSeconds`、2 つの表示)。(b) 決めた仮説(H1〜H4)と、その根拠の値(仮想/壁の比・`deskPoll` の数・compose の数・変化の数・`dwm`)。(c) 直した所と、その理由。(d) 直した後の値。(e) 起動の `elapsedMs` の前後。(f) 流れ A〜D の 4 行。(g) P3 の音の `underrun` の前後。(h) `dwm_timing` の前後。(i) 受け入れの各判定の出力の行。(j) `bash guard/scan.sh` の結果。
- 「達成」「直った」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。出力の行を付ける。確認できなかった物は「確認できない」と書く。実測と推定を分けて書く。

## 制約

- 製品に、利用者向けの新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足さない。テストのための分岐を製品に入れない。計測の数は既存の統計の経路に足し、オプションにしない。
- Python を使わない(調査と検証にだけ使う)。OCR を使わない。
- sleep・nohup・末尾の `&`・先頭の `timeout` は使わない。待つ時は条件で待つ。
- 一時ファイルは `.tmp-*` に置き、終わったら消す。
- `git add` は、変えたファイルを 1 つずつ指定する(`git add -A`・`git add .` は使わない)。
- commit・PR に、AI が書いたことを示す行を付けない。commit の名義は、環境の設定のまま。
- AI のベンダー名、クラウドの管理の識別子、アカウント ID、ホスト名、鍵を、ファイル・commit・報告に書かない。
- 直しを隠す形(例: 測る値を弱める、期待値を下げる、窓を狭める、停止や遅れを握りつぶす)はしない。

## 作業の進め方

- 作業の最初に `date -u` を記録する。3 時間 15 分を過ぎたら、受け入れが通らなくても、作業ツリーを commit し、報告を書いて終える(ジョブの上限は 5 時間 50 分)。
- ビルドは 1 回にして、同じ bundle で繰り返し確かめて直す。長いコマンドはフォアグラウンドで直接、シェルツールの `timeout` 引数は最大 5400000(90 分)にする。背景で起動して待たない。
- 受け入れが通らなくても、作業の最後に作業ツリーを commit する。未 commit の変更が残ると、判定は失敗する。

## 他のタスクとの関係

- `blocked_by` は無い。
- 関連(blocked_by ではない): #1520(起動の順序とスプラッシュの経路。閉じた)、#1542(起動の最中の入力。同じ起動の段で、仮想時計と切り替えに触れる可能性がある)、#1541(DPR の変更の停止)。衝突したら、作業の最後に `git pull --no-rebase --no-edit` で main を取り込み、衝突を直し、受け入れをもう一度通す。
