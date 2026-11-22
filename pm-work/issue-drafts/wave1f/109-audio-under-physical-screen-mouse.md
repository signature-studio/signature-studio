---
draft: "109"
title: "実機の条件(再生しながら、マウスを 1000 Hz 以上で動かし続ける)で音が途切れる現象を再現し、取り除く"
type: Task
milestone: "M1 描画と音声の分離(P3)"
parent: "001"
blocked_by: ["040", "107"]
labels: ["render", "audio"]
size: L
priority: Urgent
project: signature-studio/3
---

## 目的

依頼者の是正(2026-11-23)の三つ目: 描画処理はまだ音声処理をブロックしている。特に、マウスを動かすだけで音が途切れる。P3 の目標は、Song Mode で音が途切れないこと。この Issue は、依頼者の実機の条件(物理画素の画面で、再生しながら、マウスを 1000 Hz 以上で動かし続ける)で音が途切れる現象を再現し、取り除く。

## 背景

- 依頼者の実機の観察(デバッグパネルとヘッダー)。
  - ヘッダー: `動作中 44.1 kHz · 出力遅延 32 ms · 直近 60 秒の途切れ 420`
  - engine: `input: 228 sent, 225 processed, 0 pending, 3 dropped/merged · engine cycle 0.00 s (1115.6/s)`、`last input: move at screen (3238,143) (pointermove)`。マウスの 1 イベントごとに engine が 1 周している(1115.6 周/s)。
  - 画面: `guest screen 3240x1863 device px in a room of 1836x1056 CSS px (device pixel ratio 1.7647, 6.04 Mpx)`。物理画素の画面で、以前の論理の画面の約 3 倍の画素数。マウスの 1 動作ごとの guest の描画・合成の量が増える。
- 依頼者の言葉(2026-11-23): 依頼者のマウスは 8000 Hz で、互換性のため 1000 Hz まで落として使っている。したがって、測定のマウスは最低 1000 Hz で動かす。
- 以前の測定が現象を捉えられなかった理由(推定。実測で確かめる): 以前の道具(`tools/p3-audio-probe/measure.mjs` の `UI_STEP_MS`。値は 50 で、`uiLoop` は半径 30 px の円)は、ポインタを 50 ms の間隔で動かしていた。この頻度は、依頼者の 1000 Hz の入力の 1/50 に当たる。#1304 と #1334 の測定は、この頻度で、再生もしていない(`nonSilentQuanta` が 0)。headless の小さな画面・DPR 1 でもあった。
- #{{107}} で、guest の画面は Windows と同じ Fractional Scaling の物理画素になる。この Issue は、その物理画素の画面で測る。論理の小さい画面では測らない。
- 過去の測定の結果は `pm-work/findings/c2-split-measure.md`(#1304)。underrun は 10 秒あたり 200〜300 で、前後・負荷の有無に差が無かった。

## 読む物

- `pm-work/findings/c2-split-measure.md`(過去の測定の結果と手順)
- `tools/p3-audio-probe/measure.mjs`(#1303 の道具。37 行の `UI_STEP_MS`、`uiLoop` の 50 ms 間隔。低い頻度の経路。土台にしてよいが、頻度は上げる)
- `tools/p2-frames/measure.mjs`(P2 の道具)
- `lift/runtime/fle_input.c`(入力の入口。`input: N sent, N processed, dropped/merged` の統合と、ポインタの送り方)
- `lift/runtime/fle_audio_transport.c`(54 行 `fle_js_publish`・108 行付近 `fle_js_wait`)、`lift/runtime/fle_thread.h`(1〜6 行: 協調のスレッド)、`lift/runtime/fle_dsound.c`(416〜471 行付近: 音声の混合と書き込み)、`lift/runtime/fle_audio_ring.c`(148〜176 行)
- `lift/runtime/fle_gdc.c`(GDI の描画)
- `lift/harness` の入力の待ち(engine の周の回し方。`grep -rn "cycle" lift/harness` で探す)
- `app/src/input/pointer.ts`、`app/src/engine/client.ts`・`worker.ts`・`render-worker.ts`
- `git log --grep 1334`(#1334 の結果)

## やること

1. 測定の道具 `tools/p3-audio-probe/measure-live.mjs` を新しく作る(`measure.mjs` を土台にしてよい)。引数は次のとおり。
   - bundle の絶対パス
   - `--dpr <値>`(既定 1.7647。部屋は 1836×1056 CSS px になるように窓の大きさを決める)
   - `--hz <値>`(ポインタの移動の頻度。既定 1000)
   - `--mouse <秒>`(ポインタを動かし続ける秒数。0 なら動かさない。既定 60)
   - `--project <.flp の絶対パス>`(省略時は下の既定)
   プロジェクトの既定: 所定のプロジェクト(#{{004}} が決める。`inputs/p3-project.tsv` が在ればその 1 件。#{{105}} が配信に入れる予定の demo)を開く。無ければ、スタートの Default のプロジェクトで、Sampler のチャンネルのパターンを再生する。再生を始める方法は、道具が確かめる(`nonSilentQuanta` が増えること)。
2. ポインタの送り方(この Issue の要)。
   - 経路: 画面の広い範囲を通る円(半径 400 CSS px。部屋の中央)を、1 秒に 1〜2 周する。1 イベントあたり約 1〜5 物理画素になる速さ。ボタンは押さない(mouseMoved だけ)。
   - 頻度: `--hz` の間隔で、`--mouse` の秒数の間、連続して送る。CDP の `Input.dispatchMouseEvent` を、応答を待たずに連続して(パイプラインで)送る、またはバッチにする。送り方の選択と理由は報告に書く。`setTimeout` の間隔に頼らない(タイマーの解像度で頻度が落ちる)。
   - 届いたかの確認: ページ側で、`pointermove` の件数を 1 秒ごとに数える(`getCoalescedEvents()` の件数も記録する)。60 秒の窓の全ての 1 秒で、件数が `--hz` の 1000 以上(`--hz 1000` の時)でなければ、`inputRateOk` を false にする。届かない時は、送り方を直す(複数の接続、`Input.dispatchMouseEvent` の `timestamp`、ページ内で `PointerEvent` を合成する方法など。合成する時は、アプリの入力の処理が `isTrusted` を見ていないかを確かめる)。
3. 出力: stdout に 1 行の JSON: `{"error","state","standIn","dpr","load","hz","windowSec","playing","inputRateOk","pointerEventsPerSecMin","inputSentPerSec","inputProcessedPerSec","engineCyclePerSec","guestMouseMovePerSec","nonSilentQuanta","quantaDelta","underrunDelta","engineGapFrames","presentsDelta"}`。
   - `load`: `--mouse` が 0 より大きければ "mouse"、0 なら "none"。
   - `inputSentPerSec`・`inputProcessedPerSec`・`engineCyclePerSec`: `input: N sent` と `engine cycle` の増分を、窓の長さで割る。
   - `guestMouseMovePerSec`: guest が見た `WM_MOUSEMOVE` の数(engine の記録)。
4. 測る(DPR 1.7647。各 n=3)。
   - 無負荷(`--mouse 0`)
   - 125 Hz(参考)
   - 1000 Hz(受け入れ。下の判定の対象)
   - 8000 Hz(参考。1 ms に 8 件。依頼者の実機の本来の頻度。届かない時は、届いた頻度を報告に書く)
   - DPR 1 で 1000 Hz(比較。判定には入れない)
   表にして報告に書く。
5. 原因を特定する。1000 Hz の入力で、`engineCyclePerSec` が入力の数に比例して増えるかを、実測で確かめる。増えるなら、入力 1 件ごとに、guest の描画(GDI のラスタ・合成・提示)と協調スケジューラの切り替えが走っていないかを、時間の内訳で調べる。音声を作る側の時間を食う経路を、表にする。
6. 直し方の指針(Windows と同じ振る舞いにする)。Windows では、`WM_MOUSEMOVE` は低い優先度の合成メッセージで、キューに未処理の物が在る間は、新しい移動は 1 つに合成される。アプリが次のメッセージを取りに来た時に、最新の位置の `WM_MOUSEMOVE` が 1 つ出る(高頻度の履歴は `GetMouseMovePointsEx` で取れるが、通常の描画は最新の位置だけを使う)。これを再現する。
   - ページから engine へは、入力を時刻と位置の列として共有メモリに積む(1000〜8000 Hz でも、ページ側の処理は軽くする)。
   - engine 側では、guest が `GetMessage`・`PeekMessage` で取りに来た時に、最新の位置の `WM_MOUSEMOVE` を 1 つだけ作る。
   - 音声の書き込みの締め切りの前に、描画・入力の処理が時間の上限を超えて走らないようにする。協調スケジューラは、音を作る guest のスレッドを、描画・入力のスレッドより先に走らせる。
   - 製品に新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足さない。既定の動作として直す。オプトインを作らない。
   - 計測は一時的な変更として入れ、受け入れの前に取り除く(製品に残さない)。
7. 直した後に、上の 4 の測定を同じ道具で行い、表で比べる。

## 受け入れ

```accept
npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false?0:1)' "$RUNNER_TEMP/boot.json" && mkdir -p .tmp-109 && for i in 1 2 3; do node tools/p3-audio-probe/measure-live.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" --dpr 1.7647 --hz 1000 --mouse 60 > ".tmp-109/mouse-$i.json" || exit 1; done && node -e 'const fs=require("fs");const L=[1,2,3].map((i)=>JSON.parse(fs.readFileSync(".tmp-109/mouse-"+i+".json","utf8").trim()));process.exit(L.every((r)=>r.error===null&&r.state==="engine-idle"&&r.standIn===false&&Math.abs(r.dpr-1.7647)<0.0001&&r.hz===1000&&r.load==="mouse"&&r.inputRateOk===true&&r.playing===true&&r.nonSilentQuanta>0&&r.quantaDelta>0&&r.underrunDelta<=5)?0:1)' && rm -rf .tmp-109
```

判定の基準: DPR 1.7647、画面は物理画素の 1836×1056 CSS px の部屋、音を出しながら(`playing` が true、`nonSilentQuanta` が 0 より大きい)、ポインタを 1000 Hz 以上で 60 秒動かし続けた 3 回すべてで、`error` が null、`state` が engine-idle、`standIn` が false、`inputRateOk` が true(60 秒の窓の全ての 1 秒で 1000 件以上届いた)、`underrunDelta` が 5 以下。8000 Hz と 125 Hz と無負荷の結果は、報告の表に出すが、受け入れには入れない。

5 以下にできなければ、到達した 3 回の `underrunDelta` と残る要因を報告に書く。その場合、受け入れは通らないので、判定は PM が見直す。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には「runner の値。この機械とは別枠」と書き、nproc と loadavg を添える。
- 実測と推定を分ける。「達成」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。出力の行を付ける。
- 報告に書く: 頻度ごとの表(無負荷・125 Hz・1000 Hz・8000 Hz。DPR 1.7647 の n=3。`inputRateOk`・`inputSentPerSec`・`engineCyclePerSec`・`guestMouseMovePerSec`・`underrunDelta`・`engineGapFrames`・`presentsDelta`)、DPR 1 の比較、入力の周数の比例の確認、内訳の表(計測の結果)、ポインタの送り方の選択と理由、取り除いた経路、変えたパス(app/・lift/・tools/ を含む。`app/` と `lift/` を変えると main の deploy の仕組みが動く)、変更の前後の値。
- 耳の確認は行わない。「音が途切れない」とは書かず、underrun の数で書く。

## 制約

- 製品に新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足さない。オプトインを作らない(計測の切り替えも含む)。
- 製品の既定の動作を、測定のためだけに変えない。
- ベンダー名と Cloudflare の識別子を書かない。
- sleep・timeout・nohup・末尾の `&` は使わない。一時ファイルは `.tmp-*` に置き、終わったら消す。
- 製品の作業は Actions で行う。この機械で製品のビルドや計測をしない。

## 他のタスクとの関係

- blocked_by: #{{040}}(音声の圧迫。#1334)と #{{107}}(物理画素の画面に戻す。Fractional Scaling)。#{{107}} の後の物理画素の画面で測る。
- #{{108}}(動的な変化と全ての拡大率)とは、画面の大きさの扱いを共有する。
- #{{004}} と #{{105}} の demo は、配信に入ってから使う。無い時は上の既定を使う。
- 描画の経路(`app/src/engine/client.ts`・`desk.ts`・`render-worker.ts`)と入力の経路(`lift/runtime/fle_input.c`)は #{{040}} と #{{107}} と同じ。衝突したら、作業の最後に `git pull --no-rebase --no-edit` で main を取り込み、衝突を直し、受け入れをもう一度通す。
