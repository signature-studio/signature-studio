---
draft: "111"
title: "描画の所要を、変化した面積に比例させ、物理画素の画面(6 Mpx)で 120 fps の予算に収める"
type: Task
milestone: "M2 HiDPI と P2 の測定(P2)"
parent: "005"
blocked_by: ["110", "107", "109"]
labels: ["render", "perf"]
size: XL
priority: High
project: signature-studio/3
---

## 目的

依頼者の実機の画面(3240×1863 物理画素。DPR 1.7647。6.04 Mpx)で、P2 の描画の部分を満たす。提示 1 回の所要は、いまは変化した面積ではなく、画面全体の画素数に近い形で増えていると推定する(未確認。実測で確かめる)。この Issue は、#{{110}} の道具で増える所を特定し、転送と描画の量を変化した面積に比例させる。

## 背景

- #{{008}} の実測(`pm-work/findings/p2-measure-1308.md`。runner の値。n = 1): 拡大率 2(4.57 Mpx)で、提示 1 回の compose が 4.9 ms、put が 5.0 ms。合計約 10 ms で、120 fps の予算 8.33 ms を超える。依頼者の実機は 6.04 Mpx で、これより重い。
- 画素数に比例しているかは、まだ測っていない。推定: 所要は、変化した面積ではなく、画面全体の画素数に近い形で増えている。
- 表示方式 D(`app/src/engine/render-worker.ts` の 129 行付近の注釈): `display method D: the runtime describes the desktop (windows, positions, written rectangles) and the render Worker draws it with WebGL2, one texture per window`。runtime が書いた矩形を通知し、render Worker が窓ごとのテクスチャを WebGL2 で描く。`app/src/engine/desk-gl.ts` の冒頭の注釈は、書かれた矩形ごとに `texSubImage2D` で転送すると書いている。
- 入力: マウスは 1000 Hz 以上で動く(#{{109}})。入力の処理は、#{{109}} で入る合成(最新の位置だけを使う)を前提にする。
- 物理画素の画面は #{{107}} で入る。#{{107}} は render Worker と canvas の大きさを変えるので、その後に行う。

## 読む物

- `tools/p2-frames/measure.mjs`(#{{110}} が直す道具)、`tools/feat-c1-hidpi/measure-size.mjs`(#{{107}} が作る道具。縁の遷移を測る)
- `pm-work/findings/p2-measure-1308.md`、`pm-work/findings/survey-c2-worker-split.md`
- `app/src/engine/render-worker.ts`(129 行付近の注釈。フレームの要求、提示の時刻 `presentedAt`、`desk` の統計)
- `app/src/engine/desk.ts`(`DeskPresentStats`: `rects`・`pixels`・`whole`・`composeMs`・`putMs`)、`app/src/engine/desk-gl.ts`(テクスチャの更新、`texSubImage2D`、書かれた矩形。`stats` の `uploads`・`uploadBytes`・`draws`・`drawMs`・`texAllocs`)
- `app/src/engine/protocol.ts`(87〜89 行付近: `desk` の統計の型)、`app/src/engine/worker.ts`(375 行付近: `frame` の送信)
- `app/test/unit/desk-gl.test.ts`(書かれた矩形と、変化の無い時に GL を呼ばないことの単体テスト)
- `lift/runtime/fle_desk.c`(501 行以降: 書かれた矩形の記録と、ブラウザが持つ内容との比較。変わらない書き込みは通知しない)
- `app/src/engine/client.ts`(入力のリングへの書き込み)

## やること

1. 変更の前に、#{{110}} の道具で DPR 1.7647 の内訳を取り、報告に表で書く。提示 1 回あたり、負荷 a・b、n = 3 の全てで次の項目を出す。(a) 書かれた矩形の数と面積(`rects`・`pixels`)、(b) テクスチャへの転送の回数と量(`uploads`・`uploadBytes`)、(c) 描画の回数と時間(`draws`・`drawMs`)と compose の時間、(d) put の時間、(e) main スレッドの処理の時間(入力の処理・フレームの要求)。どの項目が画面全体の画素数に比例しているかを特定し、結論を書く。
2. 直す。
   - 変化した矩形だけを、テクスチャに転送する(矩形ごとの `texSubImage2D`。共有メモリからの直接の転送。画素の並びは変えない)。
   - compose は、変化した窓の面積だけを描き直す(スクラッチの再利用・窓ごとの dirty の追跡・変化の無い提示の省略)。
   - 転送と描画の量を、変化した面積に比例させる。
   - 入力の処理は、#{{109}} の合成(最新の位置だけ)を前提にする。
   - 画質を落として速くしない。窓の縁の見た目(#{{107}} の `edgeWidth`)と色は変えない。
3. 直した後、#{{110}} の道具で DPR 1.7647 の全条件(負荷 a・b、n = 3)を測り、1 の表と同じ項目を、変更の前後で並べて報告に書く。DPR 1 の値も比較として出す(判定には入れない)。
4. 判定に使う値は次の 2 つ。`cpuPerPresentMsP95` が 4.2 ms 以下(120 fps の予算 8.33 ms の半分。runner の CPU の値で、実機の GPU の余裕を見込んだ代理)、`inputToPresentMsP95` が 17 ms 以下。到達できなければ、到達した値と、残る要因を報告に書く。

## 受け入れ

```accept
npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false?0:1)' "$RUNNER_TEMP/boot.json" && mkdir -p .tmp-111 && for l in a b; do for i in 1 2 3; do node tools/p2-frames/measure.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" --dpr 1.7647 --load $l > ".tmp-111/$l-$i.json" || exit 1; done; done && node -e 'const fs=require("fs");const L=[];for(const l of ["a","b"])for(const i of [1,2,3])L.push(JSON.parse(fs.readFileSync(".tmp-111/"+l+"-"+i+".json","utf8").trim()));const num=(x)=>typeof x==="number"&&Number.isFinite(x);process.exit(L.length===6&&L.every((r)=>r.error===null&&r.valid===true&&Math.abs(r.dpr-1.7647)<0.0001&&num(r.cpuPerPresentMsP95)&&r.cpuPerPresentMsP95<=4.2&&num(r.inputToPresentMsP95)&&r.inputToPresentMsP95<=17)?0:1)' && node tools/feat-c1-hidpi/measure-size.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/size.jsonl" && node -e 'const L=require("fs").readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const r=L.find((x)=>Math.abs(x.dpr-1.7647)<0.0001);process.exit(r!==undefined&&r.error===null&&r.state==="engine-idle"&&r.standIn===false&&r.edgeWidth<=2?0:1)' "$RUNNER_TEMP/size.jsonl" && rm -rf .tmp-111
```

## 判定の基準

DPR 1.7647・負荷 a・b・n=3 の 6 回の全てで、上の node -e が次を読む: `error` が null、`valid` が true、`cpuPerPresentMsP95` が 4.2 以下、`inputToPresentMsP95` が 17 以下。さらに、#{{107}} の道具の DPR 1.7647 の行で、`state` が engine-idle、`standIn` が false、`error` が null、`edgeWidth` が 2 以下。到達できなければ、受け入れは通らない(判定は PM が見直す)。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には「runner の値。この機械とは別枠」と書き、nproc と loadavg を添える。
- 実測と推定を分ける。「達成」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。出力の行を付ける。
- 報告に書く: 変更の前の内訳の表(1 の項目。DPR 1.7647 の n = 3 の全て)、どこが画素数に比例していたかの結論、変更の後の同じ表、DPR 1 の比較、`cpuPerPresentMsP95` と `inputToPresentMsP95` の値(n = 3 の全て)、縁の遷移の値(`edgeWidth`)、変えたパス(app/・lift/・tools/ を含む。`app/` と `lift/` を変えると main の deploy の仕組みが動く)。

## 制約

- 製品に新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足さない。既定の動作として直す。オプトインを作らない。
- 測定のための一時的な変更は、受け入れの前に取り除く(製品に残さない)。
- 画質を落として速くしない。
- ベンダー名と Cloudflare の識別子を書かない。
- sleep・timeout・nohup・末尾の `&` は使わない。一時ファイルは `.tmp-*` に置き、終わったら消す。
- 製品の作業は Actions で行う。この機械で製品のビルドや計測をしない。

## 他のタスクとの関係

- blocked_by: #{{110}}(測定の道具)、#{{107}}(物理画素の画面。render Worker と canvas の大きさ)、#{{109}}(入力の合成)。
- #{{040}}(描画が音声を圧迫する経路)と #{{108}}(動的な変化と全ての拡大率)は、同じ描画の経路(`app/src/engine/desk.ts`・`desk-gl.ts`・`render-worker.ts`)を変えうる。衝突したら、作業の最後に `git pull --no-rebase --no-edit` で main を取り込み、衝突を直し、受け入れをもう一度通す。
- 受け入れは、#{{110}} の道具(`tools/p2-frames/measure.mjs`)と #{{107}} の道具(`tools/feat-c1-hidpi/measure-size.mjs`)の引数に合わせて書いてある。どちらかの引数を変えたら、この受け入れも直す。
