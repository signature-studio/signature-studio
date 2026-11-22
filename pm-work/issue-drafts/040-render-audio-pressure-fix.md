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

課題 C-2 の残り: 画面の描画が音声の処理を圧迫しない状態にする。#{{002}} の実測で、描画の分離の後も圧迫が残ると分かった(下の「前回の測定で分かったこと」)。この Issue は、残る圧迫の主因を計測で特定し、取り除く。完了の目安は、画面の更新を続ける負荷(`--load ui`)をかけても、10 秒あたりの underrun が 20 以下であること。P3 の目標は 0 だが、実環境(runner は 4 vCPU の共有)で届く範囲は、報告で見積もる。

## 背景

- #{{002}} の成果物: `pm-work/findings/c2-split-measure.md`。この Issue の本文は、その要点と数字を「前回の測定で分かったこと」に写してある。
- 分離(commit 43a6eb88)は、描画の合成と提示を engine の Worker から描画の Worker へ移した。#{{002}} で分離の前後の版を比べると、underrun に差は見えなかった。
- survey の読み(pm-work/findings/survey-c2-worker-split.md。推定を含む): ゲストの複数のスレッドは、1 つのホストのスレッドで協調して動く(`lift/runtime/fle_thread.h` の 1〜6 行)。音声の書き込み(`lift/runtime/fle_audio_transport.c` の 54 行 `fle_js_publish`)と音声の実時間の待ち(同 108 行付近 `fle_js_wait`)は、engine と同じ fl-engine の Worker で動く。
- 旧の仮説(描画が音声を圧迫する)は、#{{002}} の測定では支持されなかった。UI 負荷を外しても underrun は同じ程度に出た。
- 主因の見立て(推定。未確認): 圧迫は描画そのものより、engine の Worker の中で音声を作る側の時間の不足(guest のミキサとスケジューラ、音声の書き込みと実時間の待ち)とみられる。この見立てを、最初の計測で確かめる。
- 方針の案(設計は作業者が決める): 計測で支配的な要因が分かってから決める。候補は、協調スケジューラの切り替えの頻度、待ちの粒度、音声を作る guest のスレッドを描画とは別の Worker に分ける仕組み(共有メモリと、wasm のスレッドの仕組み)、busy loop の抑制。
- 製品の既定の動作は変えない。オプトインは作らない。

## 前回の測定で分かったこと(#{{002}})

測定: `tools/p3-audio-probe/measure.mjs`、bundle fl-Oz、headless Chrome、窓 10 秒、各 n=3。値は runner の値で、この機械とは別枠(nproc 4、loadavg 0.96〜2.12)。UI 負荷は、デスクトップ中央で半径 30 px の円を 50 ms ごとにマウス移動する。

| 負荷 | 版 | underrunDelta(10 秒あたり。平均(範囲)) | engineGapFrames 増分(平均(範囲)) | presentsDelta(平均) |
|---|---|---|---|---|
| UI | 分離の前 | 257(199〜301) | 39,298(33,268〜46,580) | 30 |
| UI | 分離の後 | 267(238〜289) | 51,955(45,170〜55,710) | 27 |
| なし | 分離の前 | 268(247〜292) | 39,541(36,344〜44,062) | 27 |
| なし | 分離の後 | 262(239〜281) | 41,547(32,758〜58,140) | 21 |

- 圧迫は分離の後も残る。underrun は、再生の quantum(10 秒で約 3,460)の約 6〜9%。
- 前後の差は見えない。UI 負荷の有無でも差は見えない。マウスによる UI の処理は主因ではない。
- 負荷なしでも、提示は 10 秒あたり 20〜28 回起きる。
- engineGapFrames は、エンジンが無音で埋めた量(エンジン側の書き込みの遅れ)。
- 12 回すべてで nonSilentQuanta が 0。所定のプロジェクトを再生していない。P3(Song Mode)は判定していない。
- 取れていない項目(この Issue で取る): compose の回数と時間、書き込みの遅延の分布、quantum ごとのジッタ、AudioWorklet の CPU 時間。

## 読む物

- `pm-work/findings/c2-split-measure.md`(#{{002}} の成果物)、`pm-work/findings/survey-c2-worker-split.md`(手順 4〜8 の読み)
- `lift/runtime/fle_thread.h`(1〜6 行: 協調のスレッド)、`lift/runtime/fle_audio_transport.c`(54 行・108 行付近)、`lift/runtime/fle_dsound.c`(416〜471 行付近: 音声の混合と書き込み)、`lift/runtime/fle_audio_ring.c`(148〜176 行)
- `app/src/engine/client.ts`(75・90・97 行)、`app/src/engine/worker.ts`、`app/src/engine/render-worker.ts`、`app/src/engine/desk.ts`
- `app/src/audio/page-audio.ts`、`app/src/audio/soft-sink.ts`
- `tools/p3-audio-probe/measure.mjs`(#{{003}} が作った道具)

## やること

1. 最初に計測する(変更の前)。engine の Worker の中で、音声 1 quantum 分の 1 周の時間の内訳を取る。どの guest のスレッドの、どの `fle_*` の関数(`lift/runtime/fle_audio_transport.c` の書き込みと待ち、`lift/runtime/fle_thread.h` の協調スケジューラ、ミキサ)に時間がかかっているかを表にする。合わせて、書き込み 1 回の所要、書き込みの間隔の分布、描画(compose)の時間を取る。計測は一時的な変更として入れ、受け入れの前に取り除く(製品に残さない。切り替えも作らない)。結果は報告に表で出す(pm-work には書かない)。
2. 1 の結果で、支配的な要因を 1 つ決め、それを取り除く。設計の理由を報告に書く。製品の既定の動作は変えない。
3. 同じ道具(`--load ui`)で、取り除いた後の underrunDelta・engineGapFrames 増分・quantaDelta を n=3 で測る。上の表の「分離の後・UI」と並べて、報告に表で書く。
4. 起動が通ることを確かめる(engine-idle・standIn が false・error が null)。

## 受け入れ

```accept
npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false?0:1)' "$RUNNER_TEMP/boot.json" && mkdir -p .tmp-1334 && for i in 1 2 3; do node tools/p3-audio-probe/measure.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" --load ui > ".tmp-1334/ui-$i.json" || exit 1; done && node -e 'const fs=require("fs");const L=[1,2,3].map((i)=>JSON.parse(fs.readFileSync(".tmp-1334/ui-"+i+".json","utf8").trim()));process.exit(L.every((r)=>r.error===null&&r.load==="ui"&&r.state==="engine-idle"&&r.standIn===false&&r.presentsDelta>0&&r.quantaDelta>0&&r.underrunDelta<=20)?0:1)' && rm -rf .tmp-1334
```

判定の基準: 道具の 3 回すべてで、`error` が null、`load` が ui、`state` が engine-idle、`standIn` が false、`presentsDelta` が 0 より大きい(負荷が効いている)、`quantaDelta` が 0 より大きい(再生している)、`underrunDelta` が 20 以下。
受け入れが通らないときは、到達した 3 回の `underrunDelta` と残る要因を報告に書き、PM が目標と実環境の限界を見直せるようにする(この場合は「差し戻し」と書かない)。

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

- blocked_by: #{{002}}(分離の前後の測定と、残る要因の候補)。
- #{{004}}(所定のプロジェクトの実測)と #{{027}}〜#{{029}}(外部プラグインの処理)は、この Issue の完了を待つ。音声の経路が決まってから、プラグインの処理を入れるため。
- 描画の Worker を変えるので、同じ経路(`app/src/engine/client.ts`・`desk.ts`・`render-worker.ts`)を変える #{{006}}(HiDPI)と #{{010}}(メニュー)と衝突しうる。衝突したら、作業の最後に `git pull --no-rebase --no-edit` で main を取り込み、衝突を直し、受け入れをもう一度通す。

## 依頼者の実機の観察(2026-11-23)

- 依頼者の実機で、マウスを動かすだけで音が途切れる。ヘッダー: `動作中 44.1 kHz · 出力遅延 32 ms · 直近 60 秒の途切れ 420`。
- 画面: `guest screen 3240x1863 device px in a room of 1836x1056 CSS px (device pixel ratio 1.7647, 6.04 Mpx)`(物理画素の画面)。
- engine: `input: 228 sent, 225 processed, 0 pending, 3 dropped/merged · engine cycle 0.00 s (1115.6/s)`、`last input: move at screen (3238,143) (pointermove)`。マウスの 1 イベントごとに engine が 1 周している(1115.6 周/s)。
- ポインタの測定は、最低 1000 Hz で行う(依頼者の実機は 8000 Hz のマウスを、互換性のため 1000 Hz に落として使っている)。以前の測定の頻度(50 ms の間隔)では、この現象が出なかったと推定する(推定)。
- #{{109}} が、この条件(物理画素の画面・再生中・1000 Hz 以上のポインタ)の測定と修正を行う。
- **起動して約 10 秒後、再生もマウス操作も無い状態で、直近 60 秒の途切れが 343**(依頼者の実機。2026-11-23)。目標は、無音・無操作の途切れ 0。仮想の時計が壁の時計より 80〜90 ms 遅れる警告(`S-clk-maxlag`)が出ている。
- この Issue の受け入れ(UI の負荷ありで 10 秒あたり 20 以下)は、**緩すぎる**。#{{002}} の測定で、再生していない(無音の)状態でも 10 秒あたり 200〜300 の途切れが出ていたのに、それを圧迫の基準として扱ったのは PM の誤り。
- #{{109}} が、段階的な測定(段 0 は無音・無操作)と修正を行う。この Issue で、無音・無操作の途切れの原因が分かったら、報告に書く。
