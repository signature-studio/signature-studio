---
draft: "040"
title: "描画(guest の UI 描画を含む)が音声を圧迫する経路を取り除く"
type: Task
milestone: "M1 描画と音声の分離(P3)"
parent: "001"
blocked_by: ["002"]
labels: ["render", "audio"]
size: L
priority: Urgent
project: signature-studio/3
---

## 目的

課題 C-2 の残り: 画面の描画が音声の処理を圧迫しない状態にする。子 002 の実測で圧迫が残ると分かった経路を、取り除く。完了は、画面の更新を続ける負荷をかけても、音声の underrun が 0 であること。

## 規模

規模: 大(想定 8 時間。ビルドを約 5 回回す規模)

## 背景

- 子 002 の成果物: `pm-work/findings/c2-split-measure.md`(分離の前後の音声の余裕。残る要因の候補)。この Issue は、その候補を実測で確かめてから直す。
- 分離(commit 43a6eb88)は、描画の合成と提示を別の Worker に移した。しかし本番に出ていないため、分離の後の版で圧迫が消えたかは、子 002 まで不明である。
- survey の読み(pm-work/findings/survey-c2-worker-split.md。推定を含む): ゲストの複数のスレッドは、1 つのホストのスレッドで協調して動く(`lift/runtime/fle_thread.h` の 1〜6 行)。音声の書き込み(`lift/runtime/fle_audio_transport.c` の 54 行 `fle_js_publish`)と音声の実時間の待ち(同 108 行付近 `fle_js_wait`)は、engine と同じ fl-engine の Worker で動く。
- 仮説(推定。未確認): FL 本体の UI 描画(guest の GDI・Blend2D による描画)が、音声を作る guest のスレッドと同じホストのスレッドで動くため、描画の合成を別の Worker に移しても、guest の描画そのものが音声を圧迫する。この仮説を、子 002 の測定と、この Issue の最初の計測で確かめる。
- 方針の案(設計は作業者が決める): 音声を作る側を、描画とは別の Worker に分ける(共有メモリと、wasm のスレッドの仕組みを使う)。ただし、どの経路を分けるかは、計測の結果で決める。
- 製品の既定の動作は変えない。オプトインは作らない。

## 読む物

- `pm-work/findings/c2-split-measure.md`(子 002 の成果物)、`pm-work/findings/survey-c2-worker-split.md`(手順 4〜8 の読み)
- `lift/runtime/fle_thread.h`(1〜6 行: 協調のスレッド)、`lift/runtime/fle_audio_transport.c`(54 行・108 行付近)、`lift/runtime/fle_dsound.c`(416〜471 行付近: 音声の混合と書き込み)、`lift/runtime/fle_audio_ring.c`(148〜176 行)
- `app/src/engine/client.ts`(75・90・97 行)、`app/src/engine/worker.ts`、`app/src/engine/render-worker.ts`、`app/src/engine/desk.ts`
- `app/src/audio/page-audio.ts`、`app/src/audio/soft-sink.ts`
- `tools/p3-audio-probe/measure.mjs`(子 003 が作る道具)

## やること

1. 変更の前に、子 002 の結果を読み、残る要因の候補を 1 つに絞る。絞るための計測を、道具の中で行う(例: 描画の合成を止めた時と動かした時の underrun の差。計測の切り替えは、道具の中だけで行い、製品に入れない)。結果を報告に書く。
2. 候補を取り除く設計と実装。設計の理由を報告に書く。製品の既定の動作は変えない。
3. 同じ道具(`--load ui`)で、取り除いた後の underrun を n=3 で測る。
4. 起動が通ること(engine-idle・standIn が false・error が null)を確かめる。

## 受け入れ

```accept
npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false?0:1)' "$RUNNER_TEMP/boot.json" && for i in 1 2 3; do node tools/p3-audio-probe/measure.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" --load ui > "$RUNNER_TEMP/c2-fix-$i.json" || exit 1; done && node -e 'const fs=require("fs");const L=[1,2,3].map((i)=>JSON.parse(fs.readFileSync(process.env.RUNNER_TEMP+"/c2-fix-"+i+".json","utf8").trim()));process.exit(L.every((r)=>r.error===null&&r.state==="engine-idle"&&r.standIn===false&&r.presentsDelta>0&&r.quantaWhilePlaying>0&&r.underrunQuanta===0)?0:1)'
```

判定の基準: 道具の 3 回すべてで、`error` が null、`state` が engine-idle、`standIn` が false、`presentsDelta` が 0 より大きい(負荷が効いている)、再生の quantum が 1 以上、`underrunQuanta` が 0。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には「runner の値。この機械とは別枠」と書き、nproc と loadavg を添える。
- 実測と推定を分ける。「達成」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。出力の行を付ける。
- 報告に書く: 残る要因の候補と、絞り込みの計測の結果、取り除いた経路、変えたパス(app/・lift/ を含む。`app/` と `lift/` を変えると main の deploy の仕組みが動く)、変更の前後の underrun と余裕の値。
- 耳の確認は行わない。「音が途切れない」とは書かない。

## 制約

- 製品に新しい環境変数・#ifdef・URL パラメータ・ビルド変数を足さない。オプトインを作らない(計測の切り替えも含む)。
- 製品の既定の動作を、測定のためだけに変えない。
- ベンダー名と Cloudflare の識別子を書かない。
- sleep・timeout・nohup・末尾の `&` は使わない。一時ファイルは `.tmp-*` に置き、終わったら消す。
- 製品の作業は Actions で行う。この機械で製品のビルドや計測をしない。

## 他のタスクとの関係

- blocked_by: 002(分離の前後の測定と、残る要因の候補)。
- 004(所定のプロジェクトの実測)と 027〜029(外部プラグインの処理)は、この Issue の完了を待つ。音声の経路が決まってから、プラグインの処理を入れるため。
- 描画の Worker を変えるので、同じ経路(`app/src/engine/client.ts`・`desk.ts`・`render-worker.ts`)を変える 006(HiDPI)と 010(メニュー)と衝突しうる。衝突したら、作業の最後に `git pull --no-rebase --no-edit` で main を取り込み、衝突を直し、受け入れをもう一度通す。
