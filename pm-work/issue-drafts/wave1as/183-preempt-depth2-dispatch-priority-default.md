---
draft: "183"
title: "音声のミキサーが、ウィンドウの処理(WndProc・TIMERPROC)の中で回らない: 深度 2 の切り替えを DispatchMessage の WndProc・TIMERPROC に限って許し、スレッドの優先度を既定で有効にする"
type: Task
milestone: "M1 描画と音声の分離(P3)"
parent: "001"
blocked_by: ["180", "141", "159", "173"]
labels: ["perf", "audio"]
size: M
priority: Urgent
project: signature-studio/3
---

## 目的

P3(Song Mode で音が途切れない)。ゲストの音声のミキサーは、別のスレッドとして疑似プリエンプションで回る。ところが、ウィンドウの処理(`DispatchMessageA/W` から呼ばれる WndProc と TIMERPROC)の中は、深度 2 のコールバックのため、切り替えが抑止される。その間、ミキサーは音声の新しい PCM を作れず、古いバッファを循環することがある。この Issue で、切り替えの許可を、安全な呼び出し点だけに限り、スレッドの優先度を既定で効かせ、効果を P3 の指標で測る。

## 背景

- main の今の版(`lift/runtime/fle_thread.c` と `fle_win.c`・`fle_callback.c`):
  - `FLE_PREEMPT_MAX_DEPTH` は 1。`fle_progress_slice` は、深度が 1 を超える時、切り替えを延期する(`pst.deferred++`)。延期の経路では、公開済みの音声のポーリングは既に行う(`fle_audio_poll_in_callback`)。
  - `static int prio_on;`(既定は 0)。スレッドの優先度は、既定では順番に使われない。
  - `dispatch_message`(`fle_win.c`)は、WndProc と TIMERPROC を深度 2 で呼ぶ。深度 2 では許可は無い。
- ブランチの版(`pm-work/findings/perf-windows-browser/patches/03-preempt-depth2-dispatch-priority-default.patch` と、`patches/01`・`02` のとおり):
  - `fle_callback.c`: 許可の旗 `depth2_allowed` と、その設定・取得(`fle_callback_depth2_allow`・`fle_callback_depth2_allowed`)。
  - `fle_thread.c`: `save_ctx`・`load_ctx` で、許可の旗をスレッドごとに保存・復元する。`fle_progress_slice` は、深度が 2 で許可の旗が立つ時だけ、深度 1 と同じに切り替えを行う。`prio_on` の既定は 1(`base_prio_of` と `prio_of` に分けた)。
  - `fle_win.c`: `dispatch_message` で、深度 1 から呼ばれた時だけ、WndProc と TIMERPROC の呼び出しの間、許可を立てる(`SendMessage` や、深いネストの呼び出しには立てない)。終わったら、元の値に戻す。
- ブランチの計測(単発・探索。実機ではなくネイティブの計測の報告の表): 深度 2 を全面的に許した版では、無入力で「新規書き込み／消費」が 0.4991 → 0.9990、8192 フレーム一致が 47.17% → 1.63%。**全面的な版は採用しない**(フックの連鎖・タイマーの枠の再利用による再入の懸念)。限定版は、ブランチでも未測定。
- 関連する Issue(open): #{{141}}(テーマ切り替え)・#{{159}}(上部バーの値)・#{{173}}(null の呼び出し)・#1418(Windows の失敗の返り値)が、`fle_thread.c` や `fle_win.c` を触る。

## 読む物

- `lift/runtime/fle_thread.c`(`fle_progress_slice`・`save_ctx`・`load_ctx`・`prio_of`・`FLE_PREEMPT_MAX_DEPTH`)、`lift/runtime/fle_thread.h`(decision 0147 と 0234 の説明)
- `lift/runtime/fle_callback.c`・`fle_callback.h`(`depth`・`fle_callback_depth_set`)
- `lift/runtime/fle_win.c`(`dispatch_message`・`call_wndproc`・`fle_imp_DispatchMessageW`)
- `lift/harness/web/threads/thread_scenarios.c`(`sc_priority_yield` が `fle_thread_set_priorities(1)` を呼んでいる)
- `lift/crates/fe-lift/tests/wall_fr20.rs`(decision 0147 の試験)・`wall_fr9t.rs`・`callback.rs`
- `pm-work/findings/perf-windows-browser/patches/03-*.patch`(ブランチの版。参考)

## やること

1. 許可の旗を、`fle_callback.c` に足す(`depth2_allowed`。公開の環境変数・`#ifdef` は作らない)。`fle_thread.c` の `save_ctx`・`load_ctx` で、スレッドごとに保存・復元する。
2. `fle_progress_slice`: 深度が 1 を超える時、深度 2 で許可の旗が立っていれば、深度 1 と同じに切り替えを行う。それ以外は、今のとおり延期する。
3. `dispatch_message`: 深度 1 から呼ばれた時だけ、WndProc と TIMERPROC の呼び出しの間、許可を立てる。終わったら、前の値に戻す。`SendMessage` の経路・深いネストでは、立てない。
4. `prio_on` の既定を 1 にする。`fle_thread_set_priorities(0)` で戻せる内部の関数は残す。公開の環境変数は作らない。
5. 試験: レジスタ・SIMD・フラグの保存と復元、許可の旗が別のスレッドへ持ち越されないこと、`SendMessage` の経路では立たないこと。`wall_fr20`・`wall_fr9t`・`callback` が通ること(必要なら試験を足す)。

## 受け入れ

```accept
node tools/ab/base.mjs "$(git merge-base HEAD origin/main)" .tmp-base && (cd .tmp-base && npm ci --prefix app && npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle) && node tools/ab/base.mjs --verify .tmp-base && npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && cargo test --release --locked --manifest-path lift/Cargo.toml -p fe-lift --test wall_fr20 --test wall_fr9t --test callback && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > .tmp-183-boot.json && node -e 'const r=JSON.parse(require("fs").readFileSync(".tmp-183-boot.json","utf8"));process.exit(r.error===null&&r.state==="engine-idle"&&r.standIn===false?0:1)' && node tools/smoke/welcome-flow.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > .tmp-183-flow.jsonl && node -e 'const L=require("fs").readFileSync(".tmp-183-flow.jsonl","utf8").trim().split("\n").map((l)=>JSON.parse(l));const F=(f)=>L.find((r)=>r.flow===f);const ok=(r)=>!!r&&r.error===null&&r.state==="engine-idle"&&r.standIn===false&&r.trapped===false&&r.hostErrors===0&&r.opened===true&&r.elapsedMs<=12500&&r.preBootBytes<=314368711;process.exit(L.length===4&&["A","B","C","D"].every((f)=>ok(F(f)))&&F("B").welcomeClosed===true&&F("D").welcomeClosed===true?0:1)' && node tools/ab/run.mjs p3 "$PWD/.tmp-base/lift/out-app/bundle/raw/fl-Oz" "$PWD/.tmp-base/app" "$PWD/lift/out-app/bundle/raw/fl-Oz" "$PWD/app" 3 > .tmp-183-ab-p3.json && node -e 'const s=JSON.parse(require("fs").readFileSync(".tmp-183-ab-p3.json","utf8"));process.exit(s.pass===true?0:1)' && node tools/ab/run.mjs p2 "$PWD/.tmp-base/lift/out-app/bundle/raw/fl-Oz" "$PWD/.tmp-base/app" "$PWD/lift/out-app/bundle/raw/fl-Oz" "$PWD/app" 3 > .tmp-183-ab-p2.json && node -e 'const s=JSON.parse(require("fs").readFileSync(".tmp-183-ab-p2.json","utf8"));process.exit(s.pass===true?0:1)'
```

判定の基準:
- p3 の判定(`tools/ab/gates.mjs`): 変更後の全 run で `error` が null・`playing` が true・`last60sUnderruns` が 0・`inputRateOk` が true・`nonSilentDelta` が 0 より大きい・`pcmRepeat8192` が null か 0.01 未満。加えて、`underrunDelta` の変更後の中央値が基準の中央値以下(絶対の 0 は求めない。起動後の途切れは、別の規則で見る)。
- ネイティブの試験(`wall_fr20`・`wall_fr9t`・`callback`)が全て通る。
- 起動の確認と流れ A〜D が全て合格(受け入れのとおり)。
- `tools/ab/run.mjs` の `p3` と `p2` が、回帰なしで `pass` が true。
- 効果の判定(残す条件): `p3` の `pcmRepeat8192` の基準の中央値が数値(null でない)で、変更後の中央値が基準の 0.5 倍以下。基準が null で比べられないなら、取り消す。届かないなら、この変更(深度 2 の許可と優先度の既定の両方)を取り消し、測定の表だけを `pm-work/findings/perf-windows-browser/183-result.md` に書いて commit する。取り消した後も、受け入れのコマンドは通る。
- 優先度の既定の変更は、ほかのプログラムの切り替えの順番を変える。効果の判定とは別に、`p2` の `minWindowFps` が回帰しないことを確かめる(上の判定の中で通る)。
- 報告に書く: 6 回の run ごとの `pcmRepeat8192`・`pcmEligible`・`underrunDelta`・`nonSilentDelta`・`minWindowFps`、中央値、判定(残す・取り消す)。「達成」と書かず、「受理」か「差し戻し」を、出力の行で書く。

## 前回の試行で分かったこと

- まだ起動していない(依存の Issue が閉じるのを待っている)。
- 基準の準備: 受け入れの `node tools/ab/base.mjs "$(git merge-base HEAD origin/main)" .tmp-base` が、`git archive` の複製に読むだけの入力(`lift/out-app/host` など)をリンクで持ち込む。基準のビルドの後の `node tools/ab/base.mjs --verify .tmp-base` が、リンクした入力が書き換えられていないことを確かめる。
- `underrunDelta` は、中央値の相対で判定する(上の判定の式)。絶対の 0 は求めない。

## 作業の進め方

- engine のビルドは約 35 分。基準(変更前)と変更後の 2 回の engine のビルドが要る(受け入れのコマンドに入っている)。フォアグラウンドで、シェルツールの `timeout` 引数に大きい値(例 3600000。上限 5400000)を渡して、直接実行する。背景で起動して、終わるのを待たない。
- 全面的な深度 2 の許可は採用しない。許可は、WndProc と TIMERPROC の呼び出しの間だけ。
- 作業は開始から 4 時間 30 分以内に終える。最初に `date -u` で開始を記録する。その時刻を過ぎたら、受け入れが通っていなくても作業ツリーを commit して、報告を書いて終える。

## 制約

- 製品に、利用者向けの新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足さない。許可は既定の動作として入れる。オプトインを作らない。
- Python を使わない。OCR を使わない。
- 禁止されているのは、AI のベンダー名と Cloudflare の識別子だけ。FL の製品名・インストール先の名前・変数名は、書いてよい。
- sleep・nohup・末尾の `&` は使わない。一時ファイルは `.tmp-*` に置き、終わったら消す。
- `lift/` の変更は、push で main の `deploy.yml` を動かす。報告に書く。

## 他のタスクとの関係

- blocked_by: #{{180}}(計測)。#{{141}}・#{{159}}・#{{173}}(いずれも `fle_thread.c` か `fle_win.c` を触る。閉じるまで起動しない)。
- 衝突: #1418(Windows の失敗の返り値。`fle_win.c` と `fle_thread.c` を触る。開いていて、blocked_by にしていない)。`fle_thread.c` と `fle_win.c` は、他の Issue と重なりやすい。衝突したら、作業の最後に `git pull --no-rebase --no-edit` で main を取り込み、直して、受け入れをもう一度通す。
- #{{184}}(音声の通知)とは、`fle_dsound.c` と `fle_thread.c` の別の箇所を触る。
