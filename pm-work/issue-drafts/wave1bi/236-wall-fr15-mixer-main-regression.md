---
draft: "236"
title: "`wall_fr15` のエンジンの協調のミキサーの試験が、main で `rt_mixer: MISMATCH at 8193` で落ちる: 先読みの値(#1432)か通知位置の丸め(#1426)で、モデルと出力が食い違う"
type: "Task"
milestone: "M5 内蔵のエフェクトとインストゥルメント"
parent: "021"
blocked_by: []
labels: ["plugin-builtin"]
size: "M"
priority: "High"
project: "signature-studio/3"
---

## 目的

`lift/crates/fe-lift/tests/wall_fr15.rs` の `the_engines_polling_mixer_runs_against_the_model_in_virtual_time_without_an_underrun`(477 行付近)が、main で `rt_mixer: MISMATCH at 8193` により落ちる。不一致の件数は 16378 件。この試験は、エンジンの協調のミキサーの出力を、仮想時間のモデルとサンプルごとに比べる。音声の出力の正しさを保ったまま、試験を通す。

この Issue の目標は 2 つ。
1. `node tools/fe-lift-test.mjs --test wall_fr15` の新しい失敗を 0 にする。
2. 原因を、仕様(DirectSound の通知位置と、先読みの意味)で説明できる形で、報告に書く。

## 背景

- `fe-wasmgen` の変更を退避した main の HEAD でも、同じ不一致で落ちることを、#1517 の試行が確かめた。`fe-lift` の依存は `fe-decode`・`fe-pe` だけで、`fe-wasmgen` を含まない。つまり、main の既存の回帰。
- 疑う commit は 2 つ。
  - `dd98ce09`(#1432): 先読みの既定値を 60 ms から 300 ms にした(`lift/runtime/fle_audio_ring.h` の `FLE_AUDIO_LEAD_MS` が 300u)。不一致の位置 8193 は、区切りの値 8192 の直後で、先読みや通知位置の区切りに近い。
  - `7d6ddd5e`(#1426): DirectSound の通知位置をバイト単位で受け、サンプルへ切り上げる(`lift/runtime/fle_dsound.c`・試験 `sc_notify_bytes`)。
- どちらかが、試験のモデル(先読みの値・通知位置の丸め)を変え、試験の期待が古くなった。または、音声の出力そのものが変わった(製品の欠陥)。どちらかは、作業者が切り分けて決める。
- 試験の注釈が古い: `wall_fr15.rs` の 495 行付近は「既定の先読み(60 ms)」と書いたまま。100 行付近の注釈は既定を 300 ms と書いている。
- 試験の経路: 482 行と 491 行の `ok(...)` は、`run()`(100 行付近)が `FLE_AUDIO_LEAD_MS=15` を指定して走らせる。495〜500 行の `run_lead` の 2 通り(指定なし = 既定の 300 ms と、90 ms)は、既定の先読みを使う。落ちた assertion の行は、作業者が特定する。
- 出力の照合は `lift/harness/audio/ds_scenarios.c` の `rt_mixer` の場面(681 行付近)が行い、最初の数件を `rt_mixer: MISMATCH at <フレーム番号>` で出す。
- この試験は、音が途切れないこと(P3)に関わる。

## 読む物

- `lift/crates/fe-lift/tests/wall_fr15.rs`(100 行付近の `run`・`run_lead`、334 行付近の既定の先読みの試験、477〜500 行の試験)
- `lift/harness/audio/ds_scenarios.c`(`rt_mixer` の場面。681 行付近の照合)
- `lift/runtime/fle_audio_ring.h`(`FLE_AUDIO_LEAD_MS`)・`lift/runtime/fle_audio_ring.c`(`lead_ms`。132 行付近)・`lift/runtime/fle_dsound.c`(`DS_WRITE_CURSOR_MS`・`fire_notifications`。376 行付近)
- `git show dd98ce09` と `git show 7d6ddd5e`(差分と commit の本文)
- `pm-work/findings/startup-underrun-1432.md`(先読みの値の根拠。実測と推定の区別)・`pm-work/findings/perf-windows-browser/184-result.md`(通知位置の変更の測定)
- `tools/ab/run.mjs`(P3 の A/B。p3 の判定を使う)

## やること

0. 作業の最初に `date -u` を記録する。`.prior-work.md` が在れば最初に読む。`git status` で作業ツリーを確かめる。
1. 再現: `node tools/fe-lift-test.mjs --test wall_fr15` を走らせ、この試験が落ちることを確かめる。落ちた assertion の行と、`rt_mixer: MISMATCH` の行を控える。
2. 切り分け: `dd98ce09` の前後、`7d6ddd5e` の前後で、この試験を走らせ、どの commit で落ち始めるかを確かめる。`git bisect` の手動でよい。1 つの試験だけなら `cargo test --locked --manifest-path lift/Cargo.toml -p fe-lift --test wall_fr15 -- the_engines_polling_mixer` で足りる。
3. 原因の判定:
   - (a) 試験のモデルが、新しい先読みの値(300 ms)か、通知位置の丸め(サンプルへの切り上げ)を前提にしていない。→ 試験の注釈と、モデル(期待)を、新しい値に合わせて直す。根拠を、仕様(DirectSound の通知位置と、先読みの意味)で書く。
   - (b) 製品の出力が変わった(8193 以降のサンプルが、モデルと違う)。→ 製品の欠陥として直す。音が出る内容が変わるので、`tools/ab/run.mjs` の p3 の A/B も走らせ、結果を報告に書く。
   - 試験の期待を、製品の出力に合わせて書き換えるのは、(a) の仕様の根拠を示せる時だけ。
4. 直した後、`node tools/fe-lift-test.mjs --test wall_fr15` と `--test wall_fr14` が通ることを、受け入れのコマンドで確かめる。

## 受け入れ

```accept
npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false&&r.elapsedMs<=20000?0:1)' "$RUNNER_TEMP/boot.json" && node tools/smoke/welcome-flow.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/flow.jsonl" && node -e 'const fs=require("fs");const L=fs.readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const F=(f)=>L.find((r)=>r.flow===f);const ok=(r)=>!!r&&r.error===null&&r.state==="engine-idle"&&r.standIn===false&&r.trapped===false&&r.hostErrors===0&&r.opened===true;process.exit(L.length===4&&["A","B","C","D"].every((f)=>ok(F(f)))&&F("B").welcomeClosed===true&&F("D").welcomeClosed===true?0:1)' "$RUNNER_TEMP/flow.jsonl" && npm test --prefix app && node tools/fe-lift-test.mjs --test wall_fr15 && node tools/fe-lift-test.mjs --test wall_fr14 && bash guard/scan.sh
```

判定の基準:
- 起動確認(`boot.json`)が engine-idle・standIn が false・elapsedMs が 20,000 以下。
- 流れ A〜D の 4 行が、`error` が null・engine-idle・`standIn` が false・`trapped` が false・`hostErrors` が 0・`opened` が true。B と D は `welcomeClosed` が true。
- `npm test --prefix app` が通る。
- `node tools/fe-lift-test.mjs --test wall_fr15` と `--test wall_fr14` が終了コード 0(新しい失敗が無い)。
- `bash guard/scan.sh` の指摘が 0 件。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には「runner の値。この機械とは別枠」と書く。
- 報告に書く: (a) 直す前の赤の出力の行(落ちた assertion の行と `rt_mixer: MISMATCH` の行)、(b) `dd98ce09`・`7d6ddd5e` の前後の結果(どの commit で落ち始めたか)、(c) 原因の判定((a) か (b) か)と、仕様の根拠、(d) 直した後の `--test wall_fr15` と `--test wall_fr14` の最後の行、(e) 起動確認の `elapsedMs`、流れ A〜D の 4 行、(f) (b) の時の `tools/ab/run.mjs` の p3 の結果。
- 「達成」「直った」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。出力の行を付ける。

## 制約

- 製品に、利用者向けの新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足さない。`FLE_AUDIO_LEAD_MS` は既存の物で、新しく足さない。
- Python を使わない(調査と検証にだけ使う)。OCR を使わない。
- 禁止されているのは、AI のベンダー名と Cloudflare の識別子だけ。FL の製品名・変数名は、書いてよい。
- sleep・timeout のコマンド・nohup・末尾の `&` は使わない。一時ファイルは `.tmp-*` に置き、終わったら消す。

## 作業の進め方

- 作業の最初に `date -u` を記録する。4 時間 30 分を過ぎたら、受け入れが通らなくても、作業ツリーを commit し、報告を書いて終える(ジョブの上限は 5 時間 50 分)。
- 急ぐ: `node tools/fe-lift-test.mjs` の全体を受け入れに持つ Issue は、この試験の新しい失敗で落ちる。最初に、この試験を直す。
- ビルドは 1 回にして、同じ bundle で繰り返し確かめて直す。長いコマンドはフォアグラウンドで直接、シェルツールの `timeout` 引数は最大 5400000(90 分)にする。背景で起動して待たない。
- 受け入れが通らなくても、作業の最後に作業ツリーを commit する。未 commit の変更が残ると、判定は失敗する。

## 他のタスクとの関係

- blocked_by: なし。
- `#1517`(fe-wasmgen の movmsk の型)の試行は、この試験の不一致とは別の原因。#1517 の受け入れは `fe-lift` の全体の試験を外し、`fe-wasmgen` の試験だけを持つ。
