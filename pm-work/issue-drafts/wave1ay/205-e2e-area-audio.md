---
draft: "205"
title: "E2E の領域 audio: 再生・停止・録音・ループ・メトロノーム・テンポ・カウントイン・音量・ミュート・ソロ・レンダリングの、音声の期待する結果のシナリオを機能ごとに足す"
type: Task
milestone: "M2 HiDPI と P2 の測定(P2)"
parent: "133"
blocked_by: ["199"]
labels: ["verify", "render"]
size: XL
priority: Urgent
project: signature-studio/3
---

## 目的

依頼者の指示(2026-11-26。原文): 「そうではなく、存在する機能について、考えうる限り全ての E2E テストを追加してください。」

この領域(audio)は、再生と音声の機能を、期待する結果つきのシナリオで確かめる。音が出ること・途切れないこと・操作で音が変わることを、PCM で測る。最小のシナリオ数は 80 本(`tools/e2e/features/audio.min` と同じ)。目標は次の 2 つ。
1. 下の一覧の機能を、1 つずつ、音声の結果で確かめるシナリオで覆うこと。
2. 覆った機能の数と、覆えなかった機能の理由を、報告に書くこと。

## 背景

- #{{199}}(E0)が作る物: `defineScenario`・`run.mjs --area`・`--list`・`--shard`・`--budget`・見本 3 本(`tools/e2e/scenarios/audio/`)・`lib/common.mjs` の `audioProbe`。E0 が閉じてから始める。
- 音の中身を見る道具は、既に在る(`tools/p3-audio-probe/`: `measure.mjs`・`measure-live.mjs`・`gap-report.mjs`・`burst-report.mjs`・`trace-stats.mjs`・`waveform.mjs`・`profile-names.mjs`)。E2E の中では、これを重複させず、`audioProbe` から使う。
- 音が出る入力は、#{{158}}(#1401)が決める demo を使う(インストーラの素材だけで、非無音の量子が 0 でない物)。Default のプロジェクトにステップを入れて Play する経路も使える(#1401 の結果による)。
- 途切れ(underrun)は、Song Mode で音が途切れない(P3)の条件。E2E の中では、再生中と、マウスの移動 1000 Hz の下で、途切れ 0 を確かめる。

## 読む物

- `tools/p3-audio-probe/`(全件。特に `measure.mjs`・`measure-live.mjs`・`gap-report.mjs` の出力の項目名。`pcmRepeat8192` などの名前は、ここの出力に合わせる)
- `pm-work/findings/p3-demo-scan-1401.md`・`pm-work/findings/p3-blocks-1335.md`・`pm-work/findings/input-cost-153.md`(音が出る demo と入力の影響)
- `pm-work/findings/p3-probe-1335.patch`・`pm-work/findings/c2-split-measure.md`(音声の測定の経緯)
- `tools/e2e/features/generated/audio.tsv`(E0 が作る一覧。音の機能の一覧は、この領域の表で補う)
- `pm-work/findings/e2e-program.md`

## やること

1. **機能の表** `tools/e2e/features/audio.tsv`(手で書く。見出し行は E0 が作る)。機能ごとに 1 行。列は `id`・`kind`・`name`・`expected`・`how`(判定の値の名前)。
2. **シナリオ**(`tools/e2e/scenarios/audio/`。1 本 1 ファイル)。次の全てを、機能ごとに 1 本以上。
   - 再生の操作: Play・Stop・Pause・Record・Loop・Metronome(オンとオフ)・Tempo の変更(値が変わり、再生の速さが変わる)・Pattern モードと Song モード・拍子の変更・カウントイン(録音の前の拍)。
   - 音が出る入力(demo か Default のステップ。#1401 の結果による)で Play: 非無音の量子が 0 より大きい(`nonSilentQuanta` > 0)。
   - 途切れ 0: アイドル・再生中・マウス移動 1000 Hz の下の、それぞれで、途切れ(underrun)が 0。
   - PCM の反復: 同じ音の繰り返し(`pcmRepeat8192` などの項目)が、期待する範囲にある。
   - 出力の遅延: 入力から音までの遅延が、報告の基準の範囲に在る(基準の値は、初回の実測で決めて報告に書く)。
   - マスターの音量とパン: 変えた後に、PCM の振幅と左右の比が変わる。
   - ミュートとソロ(Channel Rack・Mixer): ミュートで音が 0 になる。ソロで、他のチャンネルの音が消える。
   - Mixer のルーティング: 出力先を変えた時に、音の経路が変わる(音の量の変化で確かめる)。
   - レンダリング(Export の wav): 出力が非無音で、長さが再生の長さと合う。
   - サンプルレート: 設定の値と、出力の値が一致する(窓の表の値とメモリの文字列で読む)。
3. **判定**: 各シナリオは `ok` と、測った値(`results`: `nonSilentQuanta`・途切れの数・`pcmRepeat8192`・振幅・遅延)を返す。`notes` に、判定の根拠を 1 行で書く。音が「鳴った」だけで終わらない。操作の前後で値が変わったことを確かめる。
4. **表示**: 音の測定は、DPR によらないので、既定は `['4k-dpr1']` だけにしてよい(理由を `notes` に書く)。マウスの移動 1000 Hz の物だけ、乱数の DPR を足す(入力の座標を DPR で換算する)。

## 受け入れ

```accept
npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false?0:1)' "$RUNNER_TEMP/boot.json" && node tools/smoke/welcome-flow.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/flow.jsonl" && node -e 'const fs=require("fs");const L=fs.readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const F=(f)=>L.find((r)=>r.flow===f);const ok=(r)=>!!r&&r.error===null&&r.state==="engine-idle"&&r.standIn===false&&r.trapped===false&&r.hostErrors===0&&r.opened===true;process.exit(L.length===4&&["A","B","C","D"].every((f)=>ok(F(f)))&&F("B").welcomeClosed===true&&F("D").welcomeClosed===true?0:1)' "$RUNNER_TEMP/flow.jsonl" && npm test --prefix app && (cd tools/e2e && npm ci && npx playwright install chromium) && node tools/e2e/run.mjs --area audio --list > "$RUNNER_TEMP/list.jsonl" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8").trim().split("\n")[0]);process.exit(r.area==="audio"&&r.scenarios>=80?0:1)' "$RUNNER_TEMP/list.jsonl" && node tools/e2e/run.mjs --area audio --shard 1/8 --displays 4k-dpr1 --budget 40m > "$RUNNER_TEMP/e2e.jsonl" && node -e 'const L=require("fs").readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));process.exit(L.length===1&&L.every((r)=>r.error===null&&r.servedHeadersMatch===true&&r.scenarios.length>0&&r.scenarios.every((s)=>s.skipped!==true))?0:1)' "$RUNNER_TEMP/e2e.jsonl"
```

判定の基準:
- 受け入れの式が通る(流れ A〜D・`npm test --prefix app`・`--list` の audio の scenarios が 80 以上・1 シャード(1/8)の実行が終了コード 0。`skipped` が無い)。
- 追加したシナリオの全てが、`covers` に機能の ID を持ち、その ID が `tools/e2e/features/audio.tsv` か `generated/audio.tsv` に在る。
- 途切れ(underrun)は、再生中とマウス移動 1000 Hz の下で 0。この値は、報告に出力の行を付ける。
- 網羅率: 全シャードを合わせて、この領域の機能の 90% 以上が、`ok` かつ `expectFail` 無しのシナリオで覆われる(`judge-coverage.jq`。PM が判定する。報告に覆った数を書く)。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には「runner の値。この機械とは別枠」と書き、nproc と loadavg を添える。音の測定の値は、取った時間の長さも書く。
- 「達成」「直った」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。出力の行を付ける。
- 報告に、(a) 追加したシナリオの ID と、各 `covers`、(b) 印 `expectFail: 'new'` の一覧(シナリオの ID・症状・停止の理由・run の出力の行)、(c) 通らなかった物と理由、(d) 覆った機能の数、(e) 出力の遅延の基準の値と、その決め方。

## 制約

- 製品(`app/`・`lift/`)は変えない(読むだけ)。製品の欠陥は、シナリオに `expectFail: 'new'` を付けて記録する。直すのは PM が起票する別の Issue。
- 製品に、利用者向けの新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足さない。テストのための分岐を製品に入れない。
- 領域の作業で作るファイルは、`tools/e2e/scenarios/audio/`・`tools/e2e/features/audio.tsv`・`tools/e2e/lib/audio.mjs`(必要な時だけ)に限る。`tools/p3-audio-probe/` は読むだけ(変えない)。
- Python を使わない。OCR を使わない。
- ベンダー名・Cloudflare の識別子(アカウント・バケット・ホスト名・アクセス制御の値)を、ファイルに書かない。
- 期待値に、実行時の値(DPR・DPI・窓の位置)を定数で書かない。音の基準は、実測で決めて、決め方を報告に書く。
- sleep・timeout のコマンド・nohup・末尾の `&` は使わない。待つ時は条件で待つ。一時ファイルは `.tmp-*` に置き、終わったら消す。
- 製品の作業は Actions で行う。この機械で製品のビルドや測定をしない。GitHub に、Issue・ラベル・コメントを書かない。

## 作業の進め方

- 開始の時刻を、最初に `date -u` で記録する。4 時間 30 分を過ぎたら、受け入れが通らなくても、作業ツリーを commit し、報告を書いて終える。
- ビルドは約 35 分。フォアグラウンドで直接、シェルツールの `timeout` 引数に 5400000 を渡して待つ。背景で起動して待たない。
- blend2d の取得(GitHub)が 403 で落ちたら一時的。受け入れのコマンドは `lift/out-app/b2d-src` を消してから作り直すので、そのまま再試行する。
- この領域は大きい。1 回で終わらない前提で、価値の高い順に足す(音が出る・途切れ 0 を先に、ルーティングと Export を後に)。再試行は前回の差分の上で続け、作業ツリーを消さない。
- 作業の最後に、受け入れが通らなくても、作業ツリーを commit する。

## 他のタスクとの関係

- blocked_by: #{{199}}(E0)。E0 が閉じるまで起動しない。
- #{{158}}(#1401)が音が出る demo を決める。この領域は、その結果を使う。#1401 が閉じていない時は、Default のステップ入力の経路を先に使う。
- P3(Song Mode の途切れ)の修正の Issue が後で来た時は、その Issue に対応するシナリオを 1 本以上足す。
