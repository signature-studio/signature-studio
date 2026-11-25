---
draft: "180"
title: "計測の道具に 3 つの指標を足す: P2 の 1 秒窓の最小 fps と入力の落ち数、P3 の PCM の 8192 フレーム反復。A/B の比較の道具 tools/ab を作る"
type: Task
milestone: "M2 HiDPI と P2 の測定(P2)"
parent: "005"
blocked_by: []
labels: ["verify", "perf", "audio"]
size: M
priority: High
project: signature-studio/3
---

## 目的

後続の性能の Issue(#{{181}}・#{{182}}・#{{183}}・#{{184}}・#{{185}})は、効果を同じ道具で測り、変更の前後を比べる。その道具に、次の 3 つが無い。
1. P2 の 1 秒窓の最小 fps。今の `tools/p2-frames/measure.mjs` は 1 秒ごとの `presents` を持つが、最小の窓を出さない。平均 fps は合格に使わない。
2. 入力の落ち数。エンジンの入力の待ち行列(`fle_input.c` の `EV_MAX`)が満杯の時、入力は捨てられる。その数を出す。
3. P3 の PCM の反復。音切れのカウンターが 0 でも、古いバッファを循環していれば正常ではない。8192 フレーム前との完全一致率を出す。

あわせて、変更の前後を交互に測って中央値を比べる道具 `tools/ab/run.mjs` を作る。

## 背景

- 依頼者の条件(台帳の L-074): 1000 Hz のマウス入力を続けながら評価用のプロジェクトを開いて再生し、音が途切れず、画面が最低 120 fps で更新されること。各報告で、起動時間・実測の入力 Hz・平均と 1 秒窓の最低 fps・音声の途切れの増分・PCM の異常な反復を、同じ指標で示す。
- この 3 つの指標は、ブランチ `perf/windows-browser` の計測から取る。ブランチの道具は実機の Chrome(browse)専用なので、runner で動く `tools/p2-frames/measure.mjs`・`tools/p3-audio-probe/measure-live.mjs` に移す。ブランチの写し(計測の道具と、PCM の反復の関数)は `pm-work/findings/perf-windows-browser/tools/` にある。関数の定義は `pm-work/findings/perf-windows-browser/tools/real-browser-perf/waveform.mjs` と、その試験 `pm-work/findings/perf-windows-browser/tools/spec/real-browser-waveform.test.mjs`。
- PCM の反復の判定: 無音の近傍を除いて、左右の両サンプルが 8192 フレーム前と完全一致する割合。隣の lag(4095・4096・4097・8191・8193・12288・16384・32768)は対照として出す。0.01 未満を合格にする。ループ音源は正しく反復するので、単独では判定にしない。非無音の比較が 1,000 件未満なら `null`(不合格にしない)。
- 既定のプロジェクトは、メトロノームを押すまで無音になる。`tools/p2-frames/measure.mjs` の `--load b` は、音出しのためにメトロノームのボタンを押している。同じ手順を使う。

## 読む物

- `tools/p2-frames/measure.mjs`(`seconds`・`droppedN`・`out` の組み立て・`--load b` の音出し・`stageAssets` と `appDir`)
- `tools/p3-audio-probe/measure-live.mjs`(`--play`・`--hz`・`--mouse`・出力の項目 `underrunDelta`・`nonSilentDelta`・`last60sUnderruns`・`inputRateOk`)、`tools/p3-audio-probe/measure.mjs`(出力の項目)
- `app/src/audio/page-audio.ts`(`armCapture`・`captures`・`Capture`)
- `pm-work/findings/perf-windows-browser/tools/real-browser-perf/measure.mjs`(`captureIndex`・`armCapture`・`waveform`・`minOneSecondCanvasFps` の作り方の参考)
- `lift/runtime/fle_input.c`(`EV_MAX` と `fle_input_host_mouse` の満杯の時の処理)

## やること

1. `tools/p2-frames/measure.mjs` に次を足す(既定の動作は変えない)。
   - `--app <dir>`: 測るページの app を指定する(既定はこのリポジトリの `app`)。A/B の変更前の app に使う。
   - 出力に `minWindowFps`: 30 秒の窓の、1 秒ごとの `presents` ÷ 経過秒の最小。
   - 出力に `inputDropped`: 窓の中で、エンジンが落とした入力の数(`droppedN` と同じ値を、整数で)。
2. `tools/p3-audio-probe/measure-live.mjs` と `tools/p3-audio-probe/measure.mjs` に次を足す。
   - `--app <dir>`(上と同じ)。
   - `--sound`: 再生の後にメトロノームのボタンを押す(`measure.mjs` の `--load b` と同じ座標と手順)。押した後、非無音の数が 5 秒以内に 20 増えなければ失敗にする。
   - 窓の始めに `armCapture` を呼び、窓の 8 秒分の出力を取る(`timeline` を true)。取った音から次を求め、出力に入れる: `pcmRepeat8192`(8192 フレーム前との完全一致率。取れない・非無音の比較が 1,000 未満なら `null`)、`pcmEligible`(非無音の比較の数)、`pcmNeighbors`(lag の一覧の一致率)。
   - 求め方は `tools/p3-audio-probe/waveform.mjs`(新規。`pm-work/findings/perf-windows-browser/tools/real-browser-perf/waveform.mjs` の写し)。ページの中で計算し、サンプルは外へ出さない。
3. `spec/p3-pcm-repeat.test.mjs`(新規。`pm-work/findings/perf-windows-browser/tools/spec/real-browser-waveform.test.mjs` の写し。import 先を `tools/p3-audio-probe/waveform.mjs` に直す)。
4. `tools/ab/run.mjs`(新規)。引数: `<p2|p3> <基準の bundle の raw/fl-Oz> <基準の app の dir> <変更後の bundle の raw/fl-Oz> <変更後の app の dir> [n=3]`。
   - 基準と変更後を、交互に `n` 回ずつ測る(基準 → 変更後 を 1 組とする)。`p2` は `tools/p2-frames/measure.mjs <bundle> --dpr 1.75 --load a --app <app>`。`p3` は `tools/p3-audio-probe/measure-live.mjs <bundle> --dpr 1.75 --play 1 --hz 1000 --mouse 20 --sound --app <app>`。
   - 各 run の出力は `lift/out-app/ab/` に保存する(`lift/out*/` は git の対象外)。
   - 標準出力に 1 つの JSON。`runs`・`medians`(基準と変更後の中央値。`p2` は `minWindowFps`・`inputToPresentMsP95`・`inputDropped`、`p3` は `pcmRepeat8192`・`underrunDelta`・`nonSilentDelta` の中央値)・`gates`(判定の一覧)・`pass`。`pass` が true の時だけ終了コード 0。
   - `p2` の判定: 全 run で `error` が null、全 run で `inputRateOk` が true、`minWindowFps` の変更後の中央値 ≥ 基準の中央値 × 0.90、`inputToPresentMsP95` の変更後の中央値 ≤ 基準の中央値 × 1.20。
   - `p3` の判定(変更後の全 run): `error` が null、`playing` が true、`underrunDelta` と `last60sUnderruns` が 0、`inputRateOk` が true、`nonSilentDelta` が 0 より大きい、`pcmRepeat8192` が `null` か 0.01 未満。
5. 作業の確認: `node --check` で、変えた `.mjs` の構文を確かめる。

## 受け入れ

```accept
npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node --test spec/p3-pcm-repeat.test.mjs && node --check tools/ab/run.mjs && node tools/p2-frames/measure.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" --dpr 1.75 --load a > .tmp-180-p2.json && node -e 'const r=JSON.parse(require("fs").readFileSync(".tmp-180-p2.json","utf8"));process.exit(r.error===null&&typeof r.minWindowFps==="number"&&Number.isInteger(r.inputDropped)?0:1)' && node tools/p3-audio-probe/measure-live.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" --dpr 1.75 --play 1 --hz 1000 --mouse 20 --sound > .tmp-180-live.json && node -e 'const r=JSON.parse(require("fs").readFileSync(".tmp-180-live.json","utf8"));const p=r.pcmRepeat8192;process.exit(r.error===null&&(p===null||(typeof p==="number"&&p>=0&&p<=1))&&Number.isInteger(r.pcmEligible)&&r.nonSilentDelta>0?0:1)'
```

判定の基準:
- `spec/p3-pcm-repeat.test.mjs` が通る(反復が有る信号で 8192 の一致率が 1、隣の lag は 0。無音では `null`。NaN の数が出る)。
- `tools/p2-frames/measure.mjs` の出力に、`minWindowFps`(数値)と `inputDropped`(整数)がある。`error` が null。
- `tools/p3-audio-probe/measure-live.mjs --sound` の出力に、`pcmRepeat8192`・`pcmEligible`・`pcmNeighbors` がある。`nonSilentDelta` が 0 より大きい(音が出ている)。
- 既定のプロジェクトで、この 3 つの値の一覧(実測の値)を報告に書く。

## 作業の進め方

- engine のビルドは約 35 分。フォアグラウンドで、シェルツールの `timeout` 引数に大きい値(例 3600000。上限 5400000)を渡して、直接実行する。背景で起動して、終わるのを待たない。
- 作業は開始から 4 時間 30 分以内に終える。最初に `date -u` で開始を記録する。その時刻を過ぎたら、受け入れが通っていなくても作業ツリーを commit して、報告を書いて終える。
- 計測の道具は、製品の動作を変えない。製品の既定の動作は、この Issue で変えない。

## 制約

- 製品に、新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足さない。道具の `--app` と `--sound` は、道具の引数であり、製品の設定ではない。
- Python を使わない。OCR を使わない。
- 禁止されているのは、AI のベンダー名と Cloudflare の識別子だけ。FL の製品名・インストール先の名前・変数名は、書いてよい。
- 評価用のプロジェクトは、リポジトリに入れない。音声の波形は、ブラウザの外へ出さない(集計の値だけ出す)。
- sleep・nohup・末尾の `&` は使わない。一時ファイルは `.tmp-*` に置き、終わったら消す。

## 他のタスクとの関係

- blocked_by: なし。
- #{{181}}・#{{182}}・#{{183}}・#{{184}}・#{{185}} は、この Issue の道具を使う。この Issue が閉じるまで、起動しない。
- 測定の道具の基準は、`tools/p2-frames/measure.mjs` と `tools/p3-audio-probe/measure-live.mjs` の既存の出力。既存の項目の名前と意味は変えない。
