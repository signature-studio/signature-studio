# feat-c2-render-worker: 描画の合成と提示を、音声と engine の Worker から分ける(C-2。P3 最優先)

mode: agent
規模: 大

## 目的
依頼者の言葉: 「画面の描画処理が音声処理を圧迫している。Worker 分離は完了していないのか」(台帳 L-033 の (2))。queue/plan.md の C-2 を実装で終える。描画の合成と提示を、engine と音声の書き込みを行う fl-engine の Worker から、別の Worker に移す。優先は P3(Song Mode で音が途切れない)。

## 背景(survey の実測。出典: queue/findings/survey-c2-worker-split.md の `## 判定` と `## 実測`)
- 判定は「差し戻し」。描画の合成(compose)と提示(putImageData)も、音声の書き込み(lift/runtime/fle_audio_transport.c:54 の fle_js_publish)も、同じ fl-engine の Worker で動く。ページ(メインスレッド)と AudioWorklet(app/src/audio/page-audio.ts:159)は分かれている。
- 実測(起動後 10 秒。runner の値。3 回): 提示 presents は t=0 → t=10 で 42〜45 回。compose は提示 1 回あたり 0.41〜0.49 ms、put は 0.23〜0.25 ms。画面が動かない間も、engine の idle の経路(app/src/engine/worker.ts:226)が 20 ms ごとに deskTick を呼び、idleTicks が 36〜48 回/s 増える(t=6 → 10 の区間)。
- 合成の入口: app/src/engine/desk.ts:80(this.api.compose())、提示: desk.ts:101〜102(putImageData)。呼び出しの元は worker.ts の deskTick(381 行)。
- canvas の受け渡し: app/src/engine/client.ts:86(transferControlToOffscreen を fl-engine に渡す)。Worker は client.ts:75 の 1 つだけ(製品の経路)。
- 統計: app/src/engine/worker.ts:410〜416(flushDeskStats)が、desk の統計を page に送る。項目名は presents・ticks・idleTicks・composeMs・putMs・sleeps(app/src/engine/protocol.ts の 66 行付近)。

## 方針(設計の細部は作業者が決める。制約は下の「制約」)
- 描画の Worker(例: app/src/engine/render-worker.ts。Vite の出力は render-worker-<ハッシュ>.js になる)を新設し、canvas の transferControlToOffscreen を、fl-engine ではなく描画の Worker に渡す。
- 案 A(共有メモリ): engine の画素と、変化した矩形の報告(desk.ts:14 の readDeskState)を、描画の Worker が直接読む。compose は engine に残す。一貫性は、desk.ts:52 の lastGen で確かめる。
- 案 B(矩形のコピー): engine が変わった矩形だけを共有の ring にコピーし、描画の Worker が putImageData する。追加のコピーは、今の putMs(1 回あたり約 0.25 ms)に収まると見込む(推定。未測定)。
- readback(worker.ts:303〜314)と WebGL の経路(desk-gl.ts)も、canvas と一緒に描画の Worker へ移す。
- 画面が変わらない間は、compose を走らせない。変化の通知があった時だけ起きる形にする。
- 統計の項目名(presents・ticks・idleTicks・composeMs・putMs)は、描画の Worker からも同じ名前で page に送る(測定の道具が読むため)。

## 制約(守る。台帳の依頼者の指示)
- 製品に新しい環境変数・#ifdef・URL パラメータ・ビルド変数を足さない。オプトインを作らない(既定で有効。L-002)。
- ベンダー名・Cloudflare のアカウント ID・ホスト名・絶対パスを書かない。wrangler.jsonc を書き換える仕組みを作らない。マスクの命令を置かない。Python を使わない。
- commit は環境の git の名義のまま(git config・git -c・--author・GIT_* を使わない)。commit に Claude の行を付けない。ブランチを作らない。
- app/・lift/ を変える push は main の deploy.yml を動かす。報告に、変えたパスを書く。
- 他のタスク(feat-c1-hidpi が app/src/ui/desktop.ts・app/src/main.ts・app/src/engine/desk.ts・app/src/engine/client.ts を変えうる。feat-c6-popup-menus が lift/runtime を変えうる)と衝突しうる。作業の最後に `git pull --no-rebase --no-edit` で main を取り込み、衝突を直し、下の受け入れをもう一度通してから commit する。
- P3 の測定(所定のプロジェクトでの underrun)は、入力(依頼者の個人のファイル)が runner に無いので、この task の受け入れには入れない。報告では「未確認」と書く。

## 読む物(実在を確認済み)
- app/src/engine/worker.ts(36〜39 行: 描画の canvas。220〜232 行: idle の経路。303〜314 行: readback。338〜357 行: rAF の loop。381 行: deskTick。410〜416 行: 統計)
- app/src/engine/client.ts(75 行: Worker の作成。86 行: canvas の受け渡し。146〜172 行: メッセージの処理)
- app/src/engine/desk.ts(14 行: readDeskState。52 行: lastGen。80 行: compose。91〜102 行: 共有メモリと putImageData)、app/src/engine/desk-gl.ts(208 行)
- app/src/engine/protocol.ts(66 行付近: desk の統計の型)、app/src/input/ring.ts(入力の共有メモリ。描画の ring の作り方の先例)
- app/src/audio/page-audio.ts(159 行: AudioWorkletNode)、lift/runtime/fle_audio_transport.c(54 行: fle_js_publish、108 行: fle_js_wait)
- lift/runtime/fle_desk.h(描画の合成の注釈)、lift/runtime/fle_thread.h(1〜6 行: 協調スレッドの注釈)
- tools/boot-check/boot-bundle.mjs、tools/boot-check/cdp.mjs(起動と CDP の使い方)
- queue/findings/survey-c2-worker-split.md(`## 次の手順の案` の 1〜2)

## やること
1. 変更の前に、現状を 1 回測る(tools/feat-c2-render-worker/measure.mjs を作る前の手で。boot-bundle の起動で、t=0〜10 秒の deskStats の値を貼る)。
2. 設計と実装(上の方針。案 A か案 B を選び、理由を報告に書く)。
3. 測定の道具を作る: tools/feat-c2-render-worker/measure.mjs。boot-bundle.mjs と同じ起動で、engine-idle の後、t=0 から 10 秒まで 1 秒ごとに window.__app.deskStats と state を読み、1 行の JSON を stdout に出す: {"state","standIn","error","renderWorkerSeen","workers":[名前の一覧],"samples":[{"t","presents","ticks","idleTicks","composeMs","putMs"}]}(t は 0 から 10 の 11 件)。renderWorkerSeen は、ページのリソースの .js の名前(performance.getEntriesByType('resource')、initiatorType が other)に render-worker を含むかで決める。Target.getTargets の worker の URL は空なので使わない。
4. 測定の値の読み: 変更の前後で、presents(t=0 → 10)、(ticks + idleTicks) の増分(t=6 → 10 を 4 秒で割る)を書く。
5. 音声: 所定のプロジェクトは runner に無い。underrun の判定はしない。報告に「未確認」と書く。

## 受け入れ
下の accept の行が 0 で終わること。内容は次のとおり。
- 通常のビルド(app・cargo・fe-build engine・fe-build bundle)と、boot.json の起動確認(state が engine-idle、standIn が false、error が null)。
- tools/feat-c2-render-worker/measure.mjs の出力(1 行)を node -e が読む。state が engine-idle、standIn が false、error が null、renderWorkerSeen が true。
- 提示が続く: samples の t=10 の presents が、t=0 の presents より大きい。
- 画面が動かない間の compose の回数: (t=10 の ticks + idleTicks − t=6 の ticks + idleTicks) ÷ 4 が 9 以下。survey の値 36〜48 回/s の 1/4(9 回/s)を基準にした。

accept: npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false?0:1)' "$RUNNER_TEMP/boot.json" && node tools/feat-c2-render-worker/measure.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/feat-c2.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8").trim());const at=(t)=>r.samples.find((x)=>x.t===t);const c=(t)=>at(t).ticks+at(t).idleTicks;const ok=r.state==="engine-idle"&&r.standIn===false&&r.error===null&&r.renderWorkerSeen===true&&at(10).presents>at(0).presents&&(c(10)-c(6))/4<=9;process.exit(ok?0:1)' "$RUNNER_TEMP/feat-c2.json"

## 報告の規則
- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には「runner の値。この機械とは別枠」と書き、runner の種類(nproc と loadavg)を添える。
- 実測と推定を分ける。「達成」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。出力の行を付ける。
- 報告に書く: 案 A か B の選択と理由、変えたパス(app/・lift/ を含む)、変更の前後の presents と compose の回数、P3 の underrun が「未確認」である理由(所定のプロジェクトが無い)。
- 確認できなかった物は「確認できない」と書く。

## 禁止と注意
- 測定の値を、accept の判定のために書き換えない(道具の出力の生の値を、accept の node -e が読む)。
- 製品の既定の動作を、測定のためだけに変えない。
- sleep・timeout・nohup・disown・末尾の & は使わない(待ちは JS のタイマー)。
- 一時ファイルは .tmp-* に置き、終わったら消す。リポジトリの外(/tmp を含む)に書かない。
- ログ・Artifacts・run を削除しない。
