---
draft: "002"
title: "C-2 の描画の分離が、音声への圧迫を減らしたかを、本番相当の版で実測する"
type: task
milestone: "M1 描画と音声の分離(P3)"
parent: "001"
blocked_by: ["003"]
labels: ["task", "render", "audio", "verify", "priority-p3"]
size: 6
---

## 目的

課題 C-2(画面の描画処理が音声処理を圧迫する)に対して、描画の分離(commit 43a6eb88)が効いたかを、数字で示す。分離の前の版(親の commit)と後の版(main)を、同じ道具で、同じ負荷(画面の更新を続ける操作)をかけて測り、音声の余裕の差を報告する。この Issue は直さない。残る圧迫は子 040 で直す。

## 規模

規模: 大(想定 6 時間。ビルドを 2 回回す規模: 分離の後の版と前の版)

## 背景

- 分離の commit 43a6eb88(「C-2: 描画の合成と提示を engine の Worker から描画の Worker へ分ける」)は main に在る。描画の Worker(`app/src/engine/render-worker.ts`)を新設し、engine は合成と変化の通知だけを行う。
- しかし本番に出ていない。最後に成功した deploy は run 44869716216(head 7a0c75ef。2026-11-21 05:50Z に完了)で、43a6eb88 を含まない。その後の deploy run 44872421132 は `Set up job` で失敗し、他は取り消された。依頼者が本番で見た圧迫は、分離の前の版のものである。
- 分離の後の版で圧迫が消えたかは、誰も測っていない。過去の確認は、Worker の数と合成の回数を見ただけで、音声への影響は測っていない。
- 分離の前の版の値(runner の値。pm-work/findings/survey-c2-worker-split.md): 提示が止まった後も compose が 36〜48 回/秒走る。音声の underrun は、AudioContext が suspended のため測れていなかった(子 003 で測れるようにする)。
- 音声の余裕は、`window.__app.audio.detail()` の既存の項目で読む(`app/src/audio/page-audio.ts` の 42〜43 行: `quantaWhilePlaying`・`underrunQuanta`・`engineGapFrames`)。書き込みの遅延の分布は、既存の項目で取れる物だけを報告する。取れない物は「確認できない」と書く。

## 読む物

- `tools/p3-audio-probe/measure.mjs`(子 003 が作る道具。`--load` の引数)
- `app/src/audio/page-audio.ts`(42〜43 行: 音声の項目。249 行: 取得)
- `app/src/engine/render-worker.ts`、`app/src/engine/worker.ts`(deskTick の呼び出し元)、`app/src/engine/client.ts`(75・90・97 行)
- `pm-work/findings/survey-c2-worker-split.md`(分離の前の版の値と手順)

## やること

1. 負荷の定義: 測定の間、UI の更新を続ける操作を行う。子 003 の道具の `--load ui` を使う(窓の中の点へのマウス移動を、一定間隔で送り続ける)。作業者は、測定の間に `presentsDelta`(提示の回数の増分)が 0 より大きいことを確かめ、この操作を報告に書く。0 のときは、更新を起こす別の操作を選び、理由を書く。
2. 分離の後の版(main)で、boot-bundle で起動した後、道具を `--load ui` で 3 回実行する。
3. 分離の前の版を `.tmp-c2-before` に detached の worktree として取る(`git worktree add --detach .tmp-c2-before 43a6eb88~1`。ブランチは作らない)。そこで同じ手順で build し、同じ道具で 3 回測る。測り終えたら `git worktree remove --force .tmp-c2-before` で消す。
4. 結果を `pm-work/findings/c2-split-measure.md` に書く。前後それぞれの 3 回について、JSON の 1 行、`underrunQuanta`、`engineGapFrames` の増分、`presentsDelta`、`quantaWhilePlaying` の表にする。
5. 判定は「改善が見えた」「改善が見えない」「未確認」のいずれかで書く。圧迫が残っていれば、残る要因の候補を、実測で絞って報告する(子 040 へ渡す)。

## 受け入れ

```accept
npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false?0:1)' "$RUNNER_TEMP/boot.json" && for i in 1 2 3; do node tools/p3-audio-probe/measure.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" --load ui > "$RUNNER_TEMP/c2-after-$i.json" || exit 1; done && git worktree add --detach .tmp-c2-before 43a6eb88~1 && (cd .tmp-c2-before && npm ci --prefix app && npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle) && for i in 1 2 3; do node tools/p3-audio-probe/measure.mjs "$PWD/.tmp-c2-before/lift/out-app/bundle/raw/fl-Oz" --load ui > "$RUNNER_TEMP/c2-before-$i.json" || exit 1; done && node -e 'const fs=require("fs");const L=["after","before"].flatMap((w)=>[1,2,3].map((i)=>JSON.parse(fs.readFileSync(process.env.RUNNER_TEMP+"/c2-"+w+"-"+i+".json","utf8").trim())));process.exit(L.length===6&&L.every((r)=>r.error===null&&r.ctxState==="running"&&r.quantaWhilePlaying>0&&typeof r.underrunQuanta==="number"&&r.presentsDelta>0)?0:1)' "$RUNNER_TEMP" && test -s pm-work/findings/c2-split-measure.md
```

判定の基準: 前後それぞれ 3 回の道具の出力が取れ、いずれも `error` が null、AudioContext が `running`、再生の quantum が 1 以上、`presentsDelta` が 0 より大きい。判定の結論(改善が見えた/見えない)は、報告と `pm-work/findings/c2-split-measure.md` の表で書く。この Issue は、結論が「改善が見えない」でも、測定が取れていれば通る。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には「runner の値。この機械とは別枠」と書き、nproc と loadavg を添える。
- 「受理」「差し戻し」とは書かない。「改善が見えた」「改善が見えない」「未確認」のいずれかで書く。
- 耳の確認は行わない。「音が途切れない」とは書かない。underrun の数と余裕の値で書く。
- 報告に書く: 負荷の操作と `presentsDelta` の値、分離の前後の値の表、残る要因の候補(実測で絞ったもの)、取れなかった項目と理由。

## 制約

- 製品に新しい環境変数・#ifdef・URL パラメータ・ビルド変数を足さない。
- 製品の既定の動作を、測定のためだけに変えない。この Issue は製品のコードを変えない(変えるのは `pm-work/findings/` と、子 003 の道具の引数だけ)。
- ベンダー名と Cloudflare の識別子を、ファイルにも報告にも書かない。
- ブランチを作らない。commit の名義は環境の git の設定のまま。Claude の行を付けない。
- sleep・timeout・nohup・末尾の `&` は使わない。一時ファイルは `.tmp-*` に置き、終わったら消す。

## 他のタスクとの関係

- blocked_by: 003(測定の道具)。
- 040(圧迫を取り除く)は、この Issue の結果を使う。
- 008(HiDPI と描画の分離の後の実測)は、この Issue の結果を待つ。
