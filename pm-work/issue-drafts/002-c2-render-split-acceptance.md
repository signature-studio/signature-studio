---
draft: "002"
title: "C-2 描画の別 Worker の受理を確かめ、残りを直す"
type: task
milestone: "M1 描画と音声の分離(P3)"
parent: "001"
blocked_by: []
labels: ["task", "render", "priority-p3"]
size: 4
needs_owner: false
---

## 目的

台帳 L-033 の (2): 画面の描画処理が音声処理を圧迫しないようにする(Worker 分離を完了する)。描画の合成と提示を、engine と音声の書き込みを行う Worker から別の Worker へ移す。この Issue は、移した後の実装の受理を判定し、受理できない所を直す。

## 規模

規模: 大(想定 4 時間。ビルドを約 5 回回す規模)

## 背景

- 実装は main に入っている: commit `43a6eb88`(「C-2: 描画の合成と提示を engine の Worker から描画の Worker へ分ける」)。内容は次のとおり(commit の本文による)。
  - `app/src/engine/render-worker.ts` を新設。canvas を受け取り、engine の共有メモリから変化した矩形を読んで描く(putImageData と WebGL2 の両方)。描画のループは描画の Worker の requestAnimationFrame。変化が無い間は約 8 回/秒だけ engine に問い合わせる(`DESK_WAKE_MS = 125`。render-worker.ts:25)。
  - `app/src/engine/worker.ts` は canvas を持たず、compose だけを行い、変化を MessagePort で描画の Worker に知らせる。idle の 20 ms の合成は止めた。
  - `app/src/engine/client.ts` は描画の Worker(`fl-render`)を作り(client.ts:90)、canvas を `transferControlToOffscreen` で渡す(client.ts:97)。engine とは MessageChannel でつなぐ。
  - `app/src/engine/desk-link.ts` を新設(engine と描画の Worker の制御ブロック GEN・ACK)。
  - 計測の道具 `tools/feat-c2-render-worker/measure.mjs` が main に在る。
- 分離の前の実測(pm-work/findings/survey-c2-worker-split.md。runner の値。変更の前の版): 起動後 t=0〜10 秒で、提示 presents が 42〜45 回。compose は提示 1 回あたり 0.41〜0.49 ms、put は 0.23〜0.25 ms。提示が止まった後も、idleTicks が 36〜48 回/秒増えた(t=6〜10 秒の区間)。
- 受理の記録は main に無い(queue/done や findings に、分離の後の測定は無い)。このため、受理は未確認である。

## 読む物

- `app/src/engine/render-worker.ts`(25 行: DESK_WAKE_MS。描画の loop と統計)
- `app/src/engine/worker.ts`(compose と変化の通知。deskTick の呼び出し元。idle の経路)
- `app/src/engine/client.ts`(75 行: engine の Worker。90 行: 描画の Worker。97 行: canvas の受け渡し)
- `app/src/engine/desk-link.ts`(制御ブロック)、`app/src/engine/desk.ts`(compose の呼び出しと世代番号)、`app/src/engine/desk-gl.ts`
- `app/src/engine/protocol.ts`(desk の統計の型。ticks・idleTicks・composeMs・putMs)
- `tools/feat-c2-render-worker/measure.mjs`(起動後 t=0〜10 秒の deskStats を 1 行の JSON で出す道具)
- `tools/boot-check/boot-bundle.mjs`、`tools/boot-check/cdp.mjs`(起動の手順)
- `pm-work/findings/survey-c2-worker-split.md`(分離の前の実測。出典)

## やること

1. 変更の前の値を、今の main で 1 回測る(受理の判定の基準にするため)。boot-bundle の起動で、t=0〜10 秒の deskStats を貼る。
2. 受け入れのコマンドを実行する。終了コードと、道具の JSON の 1 行を報告に貼る。
3. 差し戻しの場合(受け入れが 1 で終わる場合)は、原因を直す。直す範囲は描画の分離に限る。直したら、受け入れを最初から通す。
4. 受理なら、作業の結果を報告に書く。commit は受理の後にだけ作る。
5. 所定のプロジェクトでの underrun は、この Issue では測らない(子 004 で測る)。報告に「未確認」と書く。

## 受け入れ

```accept
npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false?0:1)' "$RUNNER_TEMP/boot.json" && node tools/feat-c2-render-worker/measure.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/feat-c2.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8").trim());const at=(t)=>r.samples.find((x)=>x.t===t);const c=(t)=>at(t).ticks+at(t).idleTicks;const ok=r.state==="engine-idle"&&r.standIn===false&&r.error===null&&r.renderWorkerSeen===true&&at(10).presents>at(0).presents&&(c(10)-c(6))/4<=9;process.exit(ok?0:1)' "$RUNNER_TEMP/feat-c2.json"
```

判定の基準: 道具の出力の生の値を、上の node -e が読む。`renderWorkerSeen` が true(描画の Worker のスクリプトが読み込まれた)、t=10 の presents が t=0 より多い、画面が動かない間の compose が 9 回/秒以下(t=6〜10 秒の増分 ÷ 4)。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には「runner の値。この機械とは別枠」と書き、nproc と loadavg を添える。
- 実測と推定を分ける。「達成」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。出力の行を付ける。
- 報告に書く: 変更の前後の presents と compose の回数、変えたパス(app/・lift/ を含む)、P3 の underrun が「未確認」である理由(所定のプロジェクトが無い)。
- 出力が取れない物は「確認できない」と書く。動いていない物を動いていると書かない(台帳 L-036)。

## 制約

- 製品に新しい環境変数・#ifdef・URL パラメータ・ビルド変数を足さない。オプトインを作らない(L-002)。
- ベンダー名と Cloudflare の識別子を、ファイルにも報告にも書かない(L-005・L-006)。
- commit の名義は環境の git の設定のまま。Claude の行を付けない。ブランチを作らない(L-023・L-024)。
- 製品の作業は Actions で行う。この機械で製品のビルドや計測をしない(L-009)。
- 製品の既定の動作を、測定のためだけに変えない。
- sleep・timeout・nohup・末尾の `&` は使わない。一時ファイルは `.tmp-*` に置き、終わったら消す(L-022)。

## 他のタスクとの関係

- 同じ Worker の周辺(`app/src/engine/client.ts`・`desk.ts`)を変えうる: 006(HiDPI)。006 は描画の大きさを変えるので、衝突したら、作業の最後に `git pull --no-rebase --no-edit` で main を取り込み、衝突を直し、受け入れをもう一度通す。
- 後続: 004(P3 の実測)と 008(HiDPI の後の実測)は、この Issue の受理を待つ。
