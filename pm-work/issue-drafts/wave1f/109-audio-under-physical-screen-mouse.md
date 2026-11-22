---
draft: "109"
title: "起動直後(再生もマウス操作も無い状態)から音が途切れる現象を取り除き、再生しながらマウスを 1000 Hz で動かしても途切れないようにする"
type: Task
milestone: "M1 描画と音声の分離(P3)"
parent: "001"
blocked_by: ["040"]
labels: ["render", "audio"]
size: L
priority: Urgent
project: signature-studio/3
---

## 目的

依頼者の指摘(2026-11-23): 起動して約 10 秒後、一度も再生ボタンを押さず、マウスも動かしていない状態で、ヘッダーの `直近 60 秒の途切れ` が 343 ある。使い物にならないレベルで、パフォーマンスが最悪である。

P3 の目標は、Song Mode で音が途切れないこと。依頼者の是正(2026-11-23)の三つ目: 描画処理はまだ音声処理をブロックしている。特に、マウスを動かすだけで音が途切れる。

この Issue の目標は 0。まず、**無音で、何も操作していない状態の途切れを 0 にする**。そのうえで、依頼者の実機の条件(物理画素の画面で、再生しながら、マウスを 1000 Hz 以上で動かし続ける)でも途切れを 0 にする。

## 背景

- 依頼者の実機の観察(2026-11-23。画面は物理画素 3240×1863、DPR 1.7647、部屋は 1836×1056 CSS px)。
  - ヘッダー: `音: 動作中 44.1 kHz · 出力遅延 32 ms · 直近 60 秒の途切れ 343`。**起動して約 10 秒後の値。再生もマウス操作も無い**(前回の貼り付けでは 420)。
  - 343 回/60 秒 ≈ 5.7 回/秒(約 175 ms に 1 回)。周期性は未確認。
  - ページのログ(画面 2 回とも): `[warn] the virtual clock fell 84 ms further behind the wall clock than the cap allows (S-clk-maxlag); it stays behind (no virtual time is skipped)`(別の画面では 94 ms)。engine の仮想の時計が、壁の時計に対して、起動の時点で 80〜90 ms 遅れたまま戻らない。
  - 起動: `Download … at 2.0 s`、`Compile … at 2.0 s`、`Engine running … ran 2.5 s (main() began at 4.0 s)`、`First pixel at 6.9 s`、`Idle at 6.4 s`(2 回目。保存した複製からの起動)。
  - engine(マウスを動かした後の値): `Engine: computing (0.0 s) - input waits for its next idle`、`input: 837 sent, 835 processed`、`engine cycle 0.00 s (1058.1/s)`。
  - 前回の貼り付け(マウスを動かした状態): `input: 228 sent, 225 processed, 0 pending, 3 dropped/merged · engine cycle 0.00 s (1115.6/s)`、`last input: move at screen (3238,143) (pointermove)`。マウスの 1 イベントごとに engine が 1 周している(1115.6 周/s)。
  - 画面: `guest screen 3240x1863 device px in a room of 1836x1056 CSS px (device pixel ratio 1.7647, 6.04 Mpx)`。以前の論理の画面の約 3 倍の画素数。マウスの 1 動作ごとの guest の描画・合成の量が増える。
- 途切れの数え方(`app/src` の読み): ヘッダーの値は `underrunQuantaLast60s`(`app/src/audio/status.ts`、`app/src/audio/page-audio.ts` の `detail()`)。数えるのは、出力が PLAYING の間に、engine が次の 128 フレームを書き終えていなかった quantum(`app/src/audio/fle-out.worklet.js` の冒頭の注釈)。PLAYING は、engine が起動時に開始する出力の状態で、再生ボタンとは別(`app/src/audio/page-audio.ts` の `detail()` の注釈)。**したがって、再生を押していなくても、無音の出力の間に途切れが数えられる**(推定。段 0 で確かめる)。
- 以前の目標の誤り(PM の誤り): #1304 の測定では、**再生していない(`nonSilentQuanta` が 0)状態で、10 秒あたり約 200〜300 の途切れ**が出ていた。これを「圧迫の基準」として扱い、目標を #1334 では「UI の負荷ありで 10 秒あたり 20 以下」、この Issue の旧版では「60 秒で 5 以下」に置いた。どちらも緩すぎた。無音で、何も操作していない状態の途切れは、0 でなければならない。目標を 0 に直す。
- 以前の測定が現象を捉えられなかった理由(推定): 以前の道具(`tools/p3-audio-probe/measure.mjs` の `UI_STEP_MS`。50 ms の間隔で、半径 30 px の円を動かす)は、ポインタを 50 ms ごとに動かし、再生もしていなかった。依頼者の実機では、マウスを動かさない状態でも途切れるので、マウスの頻度だけが原因ではない。マウスを 1000 Hz で動かした時に途切れが増えるかは、段 1 と段 3 で実測する。
- 依頼者の言葉(2026-11-23): 依頼者のマウスは 8000 Hz で、互換性のため 1000 Hz まで落として使っている。したがって、測定のマウスは最低 1000 Hz で動かす。
- 画面: 本番の現在の画面(物理画素 3240×1863)で、そのまま測る。
- 過去の測定の結果は `pm-work/findings/c2-split-measure.md`(#1304)。underrun は 10 秒あたり 200〜300 で、前後・負荷の有無に差が無かった。

## 読む物

- `pm-work/findings/c2-split-measure.md`(過去の測定の結果と手順)
- `app/src/audio/status.ts`(ヘッダーの文言)、`app/src/audio/page-audio.ts`(`detail()`・`underruns()`・`engineLoss()`)、`app/src/audio/fle-out.worklet.js`(途切れの数え方。冒頭の注釈)、`app/src/audio/ring-layout.ts`
- `app/src/engine/idle-driver.ts`(65 行付近: `S-clk-maxlag`)、`app/src/engine/worker.ts`(137 行付近: 仮想の時計の警告)
- `tools/p3-audio-probe/measure.mjs`(#1303 の道具。`UI_STEP_MS` は 50 ms。土台にしてよいが、頻度は上げる)
- `tools/p2-frames/measure.mjs`(P2 の道具)
- `lift/runtime/fle_input.c`(入力の入口。`input: N sent, N processed, dropped/merged` の統合と、ポインタの送り方)
- `lift/runtime/fle_audio_transport.c`(54 行 `fle_js_publish`・108 行付近 `fle_js_wait`)、`lift/runtime/fle_thread.h`(1〜6 行: 協調のスレッド)、`lift/runtime/fle_dsound.c`(416〜471 行付近: 音声の混合と書き込み)、`lift/runtime/fle_audio_ring.c`(148〜176 行)
- `lift/runtime/fle_gdc.c`(GDI の描画)
- `lift/harness` の入力の待ち(engine の周の回し方。`grep -rn "cycle" lift/harness` で探す)
- `app/src/input/pointer.ts`、`app/src/engine/client.ts`・`worker.ts`・`render-worker.ts`
- `git log --grep 1334`(#1334 の結果)

## やること

0. 段階的な測定(この Issue の最初の作業)。全て、画面 CSS 1836×1056・DPR 1.7647、n=3、各 60 秒。ページの読み込みの後 10 秒待ってから窓を始める。測る値は、ヘッダーの `直近 60 秒の途切れ` と同じ定義の値。窓の終わりに `window.__app.audio.detail().underrunQuantaLast60s` を読み、そのまま出す(`app/src` の定義は変えない)。
   - 段 0: 無音・無操作(`--play 0 --hz 0 --mouse 0`)。再生も、マウスの動きも無い。AudioContext を開くための、測定の道具の起動の手順(ブラウザの中で 1 回、押して離す)は、窓の前に行ってよい。それ以外の入力は無い。
   - 段 1: 無音・マウス 1000 Hz(`--play 0 --hz 1000 --mouse 60`)。
   - 段 2: 再生しながら・マウス無し(`--play 1 --hz 0 --mouse 0`)。再生は窓の始まりに押す。
   - 段 3: 再生しながら・マウス 1000 Hz(`--play 1 --hz 1000 --mouse 60`)。再生は窓の始まりに押す。
   
   **段 0 の原因を最初に特定する**。段 0 の計測で、次を表にする。
   (a) 途切れの間隔の分布: 途切れ(underrun quantum)の時刻の列を取り、間隔の中央値・最小・最大と分布。約 175 ms の周期があるか。
   (b) 仮想の時計と壁の時計の差の推移: 差の最大値・平均と、警告 `S-clk-maxlag` が出る条件。`app/src/engine/idle-driver.ts` と `app/src/engine/worker.ts` を読んで、取り方を決める。
   (c) engine の周(入力待ちの周と、音声を作る周)の間隔の分布と、1 周あたりの所要。
   (d) 音声のリングバッファの残量(fill)の推移と、AudioWorklet が読む間隔(50 ms ごとの投稿)・engine が書く間隔の比較。
   (e) 協調スケジューラ(`lift/runtime/fle_thread.h`)が、音を作る guest のスレッドを動かす間隔と、その間に他のスレッド(描画・入力・待ち)が動いた時間。
   段 0 の原因が描画・入力と無関係なら、この Issue の 6 の指針(入力の合成)は効かない。その場合は、段 0 の原因に合わせて直す。
   
   計測の道具と計測は、一時的な変更として入れ、受け入れの前に取り除く(製品に残さない。切り替えも作らない)。

1. 測定の道具 `tools/p3-audio-probe/measure-live.mjs` を新しく作る(`measure.mjs` を土台にする。`UI_STEP_MS` の 50 ms の動きは、段 1・3 で 1000 Hz に直す)。引数は次のとおり。
   - bundle の絶対パス
   - `--dpr <値>`(既定 1.7647。部屋は 1836×1056 CSS px になるように窓の大きさを決める)
   - `--play <0|1>`(既定 1。1 なら窓の始まりに再生を始める(Space)。0 なら再生を押さない。出力は、engine が起動時に開始したままにする)
   - `--hz <値>`(ポインタの移動の頻度。既定 1000。0 なら動かさない)
   - `--mouse <秒>`(ポインタを動かし続ける秒数。既定 60。0 なら動かさない)
   - `--project <.flp の絶対パス>`(省略時は下の既定)
   プロジェクトの既定: 所定のプロジェクト(#{{004}} が決める。`inputs/p3-project.tsv` が在ればその 1 件。#{{105}} が配信に入れる予定の demo)を開く。無ければ、スタートの Default のプロジェクトで、Sampler のチャンネルのパターンを再生する。再生を始める方法は、道具が確かめる(`nonSilentQuanta` が増えること)。
2. ポインタの送り方(この Issue の要)。
   - 経路: 画面の広い範囲を通る円(半径 400 CSS px。部屋の中央)を、1 秒に 1〜2 周する。1 イベントあたり約 1〜5 物理画素になる速さ。ボタンは押さない(mouseMoved だけ)。
   - 頻度: `--hz` の間隔で、`--mouse` の秒数の間、連続して送る。CDP の `Input.dispatchMouseEvent` を、応答を待たずに連続して(パイプラインで)送る、またはバッチにする。送り方の選択と理由は報告に書く。`setTimeout` の間隔に頼らない(タイマーの解像度で頻度が落ちる)。
   - 届いたかの確認: ページ側で、`pointermove` の件数を 1 秒ごとに数える(`getCoalescedEvents()` の件数も記録する)。60 秒の窓の全ての 1 秒で、件数が `--hz` 以上でなければ、`inputRateOk` を false にする。届かない時は、送り方を直す(複数の接続、`Input.dispatchMouseEvent` の `timestamp`、ページ内で `PointerEvent` を合成する方法など。合成する時は、アプリの入力の処理が `isTrusted` を見ていないかを確かめる)。
3. 出力: stdout に 1 行の JSON: `{"error","state","standIn","dpr","play","hz","mouseSec","screenDevicePx","playing","nonSilentQuanta","last60sUnderruns","underrunDelta","inputRateOk","pointerEventsPerSecMin","inputSentPerSec","inputProcessedPerSec","engineCyclePerSec","guestMouseMovePerSec","quantaDelta","engineGapFrames","engineGapFramesDelta","presentsDelta"}`。
   - `last60sUnderruns`: 窓の終わりの `underrunQuantaLast60s`(ヘッダーの値と同じ)。
   - `underrunDelta`: 窓の中の `underrunQuanta` の増分。
   - `play`・`hz`・`mouseSec`: 引数の値。`inputRateOk`: `--hz` が 0 なら null。
   - `screenDevicePx`: guest の画面の物理画素(例 "3240x1863")。
   - `inputSentPerSec`・`inputProcessedPerSec`・`engineCyclePerSec`: `input: N sent`・`engine cycle` の増分を、窓の長さで割る。
   - `guestMouseMovePerSec`: guest が見た `WM_MOUSEMOVE` の数(engine の記録)。
4. 段 0〜3 の後、参考として、同じ道具で、再生しながら(`--play 1`)の 125 Hz(`--hz 125`)と 8000 Hz(`--hz 8000`。届かない時は、届いた頻度を書く)、DPR 1 で 1000 Hz(`--dpr 1 --play 1 --hz 1000`)を n=3 で測る。判定には入れない。表にして報告に書く。
5. 原因を特定する(段 0 の後)。段 1 と段 3 で、`engineCyclePerSec` が入力の数に比例して増えるかを、実測で確かめる(入力 1 件ごとに engine が 1 周しているか)。増えるなら、入力 1 件ごとに、guest の描画(GDI のラスタ・合成・提示)と協調スケジューラの切り替えが走っていないかを、時間の内訳で調べる。音声を作る側の時間を食う経路を、表にする。
6. 直し方の指針(Windows と同じ振る舞いにする)。Windows では、`WM_MOUSEMOVE` は低い優先度の合成メッセージで、キューに未処理の物が在る間は、新しい移動は 1 つに合成される。アプリが次のメッセージを取りに来た時に、最新の位置の `WM_MOUSEMOVE` が 1 つ出る(高頻度の履歴は `GetMouseMovePointsEx` で取れるが、通常の描画は最新の位置だけを使う)。これを再現する。
   - ページから engine へは、入力を時刻と位置の列として共有メモリに積む(1000〜8000 Hz でも、ページ側の処理は軽くする)。
   - engine 側では、guest が `GetMessage`・`PeekMessage` で取りに来た時に、最新の位置の `WM_MOUSEMOVE` を 1 つだけ作る。
   - 音声の書き込みの締め切りの前に、描画・入力の処理が時間の上限を超えて走らないようにする。協調スケジューラは、音を作る guest のスレッドを、描画・入力のスレッドより先に走らせる。
   - 製品に新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足さない。既定の動作として直す。オプトインを作らない。
   - 計測は一時的な変更として入れ、受け入れの前に取り除く(製品に残さない)。
7. 直した後に、上の 0(段 0〜3)と 4(参考)を同じ道具で行い、表で比べる。

## 受け入れ

```accept
npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false?0:1)' "$RUNNER_TEMP/boot.json" && mkdir -p .tmp-109 && B="$PWD/lift/out-app/bundle/raw/fl-Oz" && for i in 1 2 3; do node tools/p3-audio-probe/measure-live.mjs "$B" --dpr 1.7647 --play 0 --hz 0 --mouse 0 > ".tmp-109/s0-$i.json" || exit 1; node tools/p3-audio-probe/measure-live.mjs "$B" --dpr 1.7647 --play 0 --hz 1000 --mouse 60 > ".tmp-109/s1-$i.json" || exit 1; node tools/p3-audio-probe/measure-live.mjs "$B" --dpr 1.7647 --play 1 --hz 0 --mouse 0 > ".tmp-109/s2-$i.json" || exit 1; node tools/p3-audio-probe/measure-live.mjs "$B" --dpr 1.7647 --play 1 --hz 1000 --mouse 60 > ".tmp-109/s3-$i.json" || exit 1; done && node -e 'const fs=require("fs");const R=(s,i)=>JSON.parse(fs.readFileSync(".tmp-109/s"+s+"-"+i+".json","utf8").trim());const ok=(r)=>r.error===null&&r.state==="engine-idle"&&r.standIn===false&&Math.abs(r.dpr-1.7647)<0.0001;const all=(s,f)=>[1,2,3].every((i)=>f(R(s,i)));process.exit(all(0,(r)=>ok(r)&&r.play===0&&r.hz===0&&r.last60sUnderruns===0)&&all(1,(r)=>ok(r)&&r.play===0&&r.hz===1000&&r.mouseSec===60&&r.inputRateOk===true&&r.last60sUnderruns===0)&&all(2,(r)=>ok(r)&&r.play===1&&r.hz===0&&r.nonSilentQuanta>0&&r.last60sUnderruns<=1)&&all(3,(r)=>ok(r)&&r.play===1&&r.hz===1000&&r.mouseSec===60&&r.inputRateOk===true&&r.nonSilentQuanta>0&&r.last60sUnderruns<=1)?0:1)' && rm -rf .tmp-109
```

判定の基準(DPR 1.7647、画面は物理画素の 1836×1056 CSS px の部屋。各段 n=3 の全て):
- 段 0(無音・無操作): `last60sUnderruns` が 0。
- 段 1(無音・マウス 1000 Hz): `last60sUnderruns` が 0 で、`inputRateOk` が true。
- 段 2(再生・マウス無し): `nonSilentQuanta` が 0 より大きく、`last60sUnderruns` が 1 以下。
- 段 3(再生・マウス 1000 Hz): `nonSilentQuanta` が 0 より大きく、`last60sUnderruns` が 1 以下で、`inputRateOk` が true。

段 0 と段 1 は、0 のまま緩めない。段 2 と段 3 の 1 以下は、runner の共有 CPU(4 vCPU)の揺れを見込んだ値。報告に、実際の値と揺れの理由を書く。0 にできなければ、到達した値と残る要因を報告に書く。その場合、受け入れは通らないので、判定は PM が見直す。参考の測定(4)は、受け入れには入れない。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には「runner の値。この機械とは別枠」と書き、nproc と loadavg を添える。
- 実測と推定を分ける。「達成」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。出力の行を付ける。
- 報告に書く: 段 0〜3 の表(n=3 の各回の `last60sUnderruns`・`underrunDelta`・`inputRateOk`・`nonSilentQuanta`・`screenDevicePx`)、段 0 の (a)〜(e) の表、参考の表(4)、入力の周数の比例の確認、ポインタの送り方の選択と理由、取り除いた経路、変えたパス(app/・lift/・tools/ を含む。`app/` と `lift/` を変えると main の deploy の仕組みが動く)、変更の前後の値。
- 耳の確認は行わない。「音が途切れない」とは書かず、途切れ(underrun)の数で書く。

## 制約

- 製品に新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足さない。オプトインを作らない(計測の切り替えも含む)。
- 製品の既定の動作を、測定のためだけに変えない。
- ベンダー名と Cloudflare の識別子を書かない。
- sleep・timeout・nohup・末尾の `&` は使わない。一時ファイルは `.tmp-*` に置き、終わったら消す。
- 製品の作業は Actions で行う。この機械で製品のビルドや計測をしない。

## 他のタスクとの関係

- blocked_by: #{{040}}(音声の圧迫。#1334)。#1334 が閉じるまで起動しない。
- #{{107}}(Fractional Scaling)は、描画の画面の大きさを変えるが、この Issue の段 0・1・2 に影響しない。段 3 の測定は、#{{107}} の前後で値が変わりうるので、報告に、画面の大きさ(guest の画面の画素数。JSON の `screenDevicePx`)を書く。
- #{{108}}(動的な変化と全ての拡大率)とは、画面の大きさの扱いを共有する。
- #{{004}} と #{{105}} の demo は、配信に入ってから使う。無い時は上の既定を使う。
- 描画の経路(`app/src/engine/client.ts`・`desk.ts`・`render-worker.ts`)と入力の経路(`lift/runtime/fle_input.c`)は #{{040}} と同じ。衝突したら、作業の最後に `git pull --no-rebase --no-edit` で main を取り込み、衝突を直し、受け入れをもう一度通す。
