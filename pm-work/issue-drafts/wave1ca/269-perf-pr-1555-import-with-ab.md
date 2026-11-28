---
draft: "269"
title: "パフォーマンス最適化の PR #1555 を、判定を通して取り込む: P3 の A/B(音声の途切れ)で main と比べ、悪化が無い部分から入れる"
type: "Task"
milestone: "M2 HiDPI と P2 の測定(P2)"
parent: "005"
blocked_by: []
labels: ["hidpi", "verify"]
size: "XL"
priority: "High"
project: "signature-studio/3"
---

## 目的

PR #1555(パフォーマンス最適化。Waves 1-19。head `dd00c1fd42dc3fbe54c31e246ac4af5982fb04dc`)の変更を、判定(`guard/judge.sh` と、受け入れコマンド)を通して main に取り込む。依頼者は PR を直接マージしない(判定と、P3 の A/B の関門を通すため)。**PR は閉じない・触らない**(依頼者が後で閉じる)。

この Issue の 1 回の試行で行うのは、**(低) の群だけ**(下の「やること」の 1)。(中)と(高)は、後の試行で、段ごとに取り込む。

## 背景

- PR の変更は 61 ファイル・+16,303 / −1,033 行。音声(P3)・協調スレッドの切り替え・計時器・起床の通知・UI の応答・描画・ビルドの最適化を含む。
- PR の検証は、起動(warm 中央値 12.2 秒)・流れ A〜D・単体試験だけ。**音声の途切れ(P3)と、1000 Hz 入力の下の fps は、PR の検証に無い。** そのため、(高) の取り込みの前に、P3 の A/B で main と比べる。
- 規則の検査: `guard/flags.sh` は、PR の head と `origin/main` の間で 0 件(この作業で確認)。`guard/scan.sh` は PR の時点で 0 件(PM の報告)。取り込んだ後の段で、もう一度 `guard/scan.sh` を当てる。
- main は、PR の基準(`332cbd66`)から、`AGENTS.md` と台帳の変更だけ進んでいる。PR の 61 ファイルとは重ならない(衝突は無い)。ただし、取り込みの作業は、PR の全体を `git checkout` で戻さない(main の後の変更を消すため)。パスを指定して取り込む。
- 読みで見つかった、(高)の前に直す点: `app/src/engine/worker.ts` の計時器の枠(`DEADLINE_SLOTS = 16`)。取り消しの `notify` が、同じ枠の生きた待ちを `'ok'` で起こし、PR の版は `'ok'` では発火しない。その待ちの期限の発火が失われる(実測: 同じ枠の 2 つの `Atomics.waitAsync` で、取り消しの後も、生きた待ちが `"ok"` で解決した)。
- 取り込まない物: `app/src/engine/hz.ts` と `app/test/unit/hz.test.ts` の、表示 Hz の上限を 120 から 300 に上げる変更。既存の設計の決定は、上限 120 を、ミキサーが追いついたら見直すこと。その条件を満たしていないので、上限は 120 のまま残す。
- 重なる Issue: #1386(`fle_thread.c`・`fle_event.c`・`lower.rs`・`lib.rs`)、#1541(`fle_thread.c`・`fle_thread.h`・`fle_input.c`)、#1548・#1546・#1395・#1545・#1552・#1554(`worker.ts`・`desk-gl.ts`・`fle_sys.c`・`fle_gdi.c`・`fle_desk.c`・`lower.rs`・`main.rs` などで重なる見込み)。衝突したら、`git pull --no-rebase --no-edit` で main を取り込み、直してから、受け入れを通す。

## 読む物

- `pm-work/findings/pr-1555-review.md`(群の分けの根拠。ファイルと行番号。確認できなかった物の一覧)。main の作業ツリーで読む。
- PR の差分: `git fetch origin pull/1555/head` の直後に `git rev-parse FETCH_HEAD` で SHA を控え、その SHA の `git diff 332cbd66 <SHA> -- <path>`。`FETCH_HEAD` は、同じクローンで他の `git fetch` に上書きされることがあるので、SHA で参照する。
- A/B の道具: `tools/ab/run.mjs`・`tools/ab/gates.mjs`・`tools/ab/base.mjs`(冒頭の注釈)。`tools/p2-frames/measure.mjs`・`tools/p3-audio-probe/measure-live.mjs`(冒頭の注釈)。
- `app/src/engine/worker.ts`(`deadlineWait`・`cancelDeadlineWait`・`DEADLINE_SLOTS`。PR の版と main の版の両方)。
- `pm-work/issue-drafts/wave1be/229-separate-wasm-suspend-js-frames.md`(基準のビルドの見本)。

## やること

### 0. 準備

- 作業の最初に `date -u` を記録する。`.prior-work.md` が在れば、最初に読む。
- `git fetch origin pull/1555/head` の後、`git rev-parse FETCH_HEAD` で SHA を控える(以下 `PRHEAD`)。
- 取り込みは、ファイルのパスを指定して行う。方法: `git checkout "$PRHEAD" -- <path>`(ファイル単位)。混在するファイル(`lift/runtime/fle_sys.c`・`lift/runtime/fle_win.c`・`app/src/engine/worker.ts`)は、`git diff <merge-base> "$PRHEAD" -- <path>` の差分から、指定した群のハンクだけを `git apply --recount` で当てるか、同じ変更を手で書く。
- **取り込まない**: `AGENTS.md`・`pm-work/ledger/instructions.tsv`(PR の外の差分)、`app/src/engine/hz.ts`・`app/test/unit/hz.test.ts`(表示 Hz の上限。main の版のまま残す)。

### 1. (低) の群(この試行で行う)

挙動が変わらないと読める物。既存の試験で守る。以下の 20 件(混在の 1 件は、ハンクの指定あり)。

- `lift/runtime/fle_raster.c`・`lift/runtime/fle_raster.h`・`lift/runtime/fle_sse.h`・`lift/runtime/fle_mxcsr_vec.h`(描画の差分走査の SIMD 化)
- `lift/runtime/fle_gdi.c`(GDI のホットパス)
- `lift/runtime/fle_flags.h`・`lift/runtime/fle_flags_nf.h`(フラグの算術)
- `lift/runtime/fle_user_nowin.c`(RECT の読み書き)
- `lift/runtime/fle_runtime.h`(分岐の予測と、未対応の命令の分割)
- `lift/runtime/fle_sys.c`(**混在: `fle_host_protect` のハンク(926 行付近)だけ。`LeaveCriticalSection` の 412 行付近のハンクは、(高) なので取り込まない**)
- `lift/harness/web/cflags-ai.args`・`lift/harness/web/cflags-b2d.args`・`lift/harness/web/cflags-h.args`・`lift/harness/web/cflags-rt.args`・`lift/harness/web/wasm-opt.args`・`lift/harness/web/wasm-opt-c.cmd`(最適化の水準。新しい変数は無い)
- `lift/crates/fe-build/src/main.rs`・`lift/crates/fe-build/tests/plan/engine.txt`(Blend2D のフラグ。**2 件は同時に取り込む**: 計画の試験が、この 2 件を照合する)
- `lift/harness/audio/sim_transport.c`(ネイティブ試験の模擬)
- `app/src/engine/protocol.ts`(型の注釈)

受け入れ: 下の accept。この段の commit は、段の変更だけを含む(`git add` は 1 件ずつ)。

### 2. (中) の群(後の試行。段ごとに commit)

入力・描画・UI の応答時間、試験の期待の書き換え、製品の既定値、正しさを試験で確かめていない生成物。以下の 24 件(混在の 2 件は、ハンクの指定あり)。

- `app/src/engine/desk-gl.ts`・`app/test/unit/desk-gl.test.ts`(**期待の書き換え。部分描画の確認を、全画面の描画の確認に置き換えた。元の確認を消さないこと**)
- `app/src/engine/desk-link.ts`・`app/src/engine/render-worker.ts`・`app/src/engine/idle-driver.ts`・`app/test/unit/idle-driver.test.ts`
- `app/src/engine/client.ts`・`app/src/main.ts`・`app/src/ui/desktop.ts`・`app/src/engine/desk.ts`・`app/src/engine/fruity.ts`
- `lift/runtime/fle_desk.c`・`lift/runtime/fle_desk.h`・`lift/runtime/fle_input.c`・`lift/runtime/fle_x87.c`
- `lift/crates/fe-wasmgen/src/lower.rs`・`lift/crates/fe-wasmgen/src/lib.rs`・`lift/crates/fe-wasmgen/src/hybrid.rs`
- `lift/harness/b2d1/b2d4-tick.patch`・`lift/harness/b2d1/b2d_swap.cpp`
- `lift/harness/seeds/reg-seed-app.tsv`(**製品の既定値 3 行を足す**)・`lift/crates/fe-lift/tests/wall_fr3.rs`(**試験の期待を、既定値に合わせて広げた**)
- `app/src/engine/worker.ts`(**混在: 計時器の枠(160〜178 行)は、(高) まで main の版のまま。`deadlineWait` と `cancelDeadlineWait` の実装は main の版を使う**。入力の間引き(`MOVE_PACE_MS`)などは取り込む)
- `lift/runtime/fle_win.c`(**混在: `CM_ACTIONUPDATE` の掃引の間引き(1040〜1058 行付近)と、`action_note_msg` だけ。計時器の走査(`timer_hi`)は (高)**)

試験の期待を書き換えた物は、報告に 1 件ずつ書く(元の期待と、新しい期待)。期待を弱める書き換えは、そのまま残さない(元の確認を、新しい実装で保つ)。

受け入れ: 下の accept に、P2 の A/B(下の 5 の方法)を加える。

### 3. 計時器の枠(**(高) の前に、試験で赤を出してから直す**)

- 対象: `app/src/engine/worker.ts` の `deadlineWait`・`cancelDeadlineWait`(PR の版の 160〜178 行)。取り消しの `notify` が、同じ枠の生きた待ちを `'ok'` で起こすと、PR の版は `if (v === 'timed-out') fire()` のため、発火しない。
- 試験: 同じ枠の 2 つの待ちを作り(枠は、作成の順に 16 で循環する)、一方を取り消し、他方の期限に発火することを確かめる。`app/test/unit/` の vitest(Node は `Atomics.waitAsync` を持つ)。試験のために、`deadlineWait` を別のモジュール(例: `app/src/engine/deadline-wait.ts`)に出してよい。製品に、テスト用の分岐・旗・環境変数は入れない。
- 手順: PR の版で赤を確かめる → 直す(例: `'ok'` でも、期限を過ぎていれば発火し、過ぎていなければ待ち直す。または、枠を取り消しごとに分ける) → 緑にする → commit。
- 直した後も、`lift/crates/fe-lift/tests/dwm_timing.rs` などの、仮想時計の試験が落ちていないこと。

### 4. (高) の群(3 回目以降。段ごとに commit)

音声(P3)・協調スレッドの切り替え・計時器・起床の通知・固定番地への書き込み・音声プラグインの生成物。以下の 15 件と、混在の 3 件の (高) のハンク。

- `lift/runtime/fle_thread.c`・`lift/runtime/fle_thread.h`(協調スレッドの UI の順番・仮想時計の寄せ・通知型の起床・後退辺の切り替えの間隔 1024 → 4096)
- `lift/runtime/fle_audio_ring.c`・`lift/runtime/fle_audio_transport.c`・`lift/runtime/fle_audio_transport.h`・`lift/runtime/fle_dsound.c`(音声の仮想時計の対応・looping の未アンロック範囲の無音)
- `lift/runtime/fle_event.c`(`SetEvent` の起床)
- `lift/runtime/fle_sys.c`(**混在の (高): `LeaveCriticalSection` の起床(412 行付近)**)
- `lift/runtime/fle_win.c`(**混在の (高): 計時器の走査範囲 `timer_hi`**)
- `app/src/engine/worker.ts`(**混在の (高): 計時器の枠。3 の直しを入れてから**)
- `lift/harness/web/threads/fle_thread_lib.js`(JSPI の切り替えを MessageChannel で返す)
- `lift/harness/two/main_two.c`(**起動の時の優先度の設定と、`fle_host_on_idle` の固定番地への書き込み。この 2 件の目的を、PR の記述と照らして確かめ、説明が無ければ、その関数を取り込まない**)
- `lift/harness/hyb/hyb-args.tsv`・`lift/harness/hyb/lists/lowered-fruity_delay2.txt`・`lift/harness/hyb/n0.args`・`lift/harness/hyb/c-imports-engine.txt`・`lift/harness/hyb1/hybrid_glue.c`(**音声のエフェクト `fruity_delay2` を Wasm にする。生成物**)
- `lift/harness/seeds/elastique-seeds-all3.txt`(音声プラグインの seed)

受け入れ: 下の accept と、P2 の A/B と、**P3 の A/B**(下の 5)。判定の基準は、`tools/ab/gates.mjs` のとおり。

### 5. P3 の A/B(音声の途切れ)と P2 の A/B(入力から提示まで)

- 基準(base)の作り方。**base の bundle は、main の e2e の cache(`bundle-<sha>`)から取らない**(作業ジョブは、その cache を取らない)。作業ジョブの中で作る:
  `node tools/ab/base.mjs "$(git merge-base HEAD origin/main)" .tmp-base && (cd .tmp-base && npm ci --prefix app && npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle) && node tools/ab/base.mjs --verify .tmp-base`
- 変更後(changed)の作り方: 作業の木で `npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle`。
- P3: `node tools/ab/run.mjs p3 "$PWD/.tmp-base/lift/out-app/bundle/raw/fl-Oz" "$PWD/.tmp-base/app" "$PWD/lift/out-app/bundle/raw/fl-Oz" "$PWD/app" 3 > "$RUNNER_TEMP/ab-p3.json"` の出力の `pass` が true。
- P2: 同じ形で `node tools/ab/run.mjs p2 ... 3 > "$RUNNER_TEMP/ab-p2.json"` の `pass` が true。P2 の判定は、`minWindowFps` の changed の中央値 ≥ base の中央値 × 0.90、`inputToPresentMsP95` の changed の中央値 ≤ base の中央値 × 1.20(`tools/ab/gates.mjs`)。
- P3 の判定(`tools/ab/gates.mjs`): changed の各 run で `error` が null・`playing` が true・`inputRateOk` が true・`nonSilentDelta` > 0・`pcmRepeat8192` が null か 0.01 未満。`underrunDelta` と `last60sUnderruns` の changed の中央値 ≤ base の最大 + 4。
- 1000 Hz の入力の下の最低の fps は、P2 の判定(`minWindowFps`)で見る。
- 作業の後、`.tmp-base` を消す(自分の物だけ)。

## 受け入れ

(低) の段の受け入れ。1 行のコマンド。

```accept
npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false&&r.elapsedMs<=20000?0:1)' "$RUNNER_TEMP/boot.json" && node tools/smoke/welcome-flow.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/flow.jsonl" && node -e 'const fs=require("fs");const L=fs.readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const F=(f)=>L.find((r)=>r.flow===f);const ok=(r)=>!!r&&r.error===null&&r.state==="engine-idle"&&r.standIn===false&&r.trapped===false&&r.hostErrors===0&&r.opened===true;process.exit(L.length===4&&["A","B","C","D"].every((f)=>ok(F(f)))&&F("B").welcomeClosed===true&&F("D").welcomeClosed===true?0:1)' "$RUNNER_TEMP/flow.jsonl" && npm test --prefix app && cargo test --release --locked --manifest-path lift/Cargo.toml -p fe-lift --test gdi --test gdi_bulk --test screen1 --test raster --test flags --test int_ext --test mxcsr_flags --test size1_deadflags --test alpha2_changed --test alpha3_updregion --test alpha7_showwin --test alpha8_hidden_move --test desk_scene --test desk1_compose --test wall_fr3 && cargo test --locked --manifest-path lift/Cargo.toml -p fe-build -- --skip the_second_wave_is_160_dlls --skip the_second_wave_with_its_data && bash guard/scan.sh && bash guard/flags.sh origin/main HEAD
```

- `--skip` の 2 件は、main で既に落ちている試験(#1540 の範囲。この Issue では直さない)。
- 判定の基準: 起動(`boot.json`)が engine-idle・`standIn` が false・`elapsedMs` が 20,000 以下。流れ A〜D が 4 行あり、`error` が null・engine-idle・`standIn` が false・`trapped` が false・`hostErrors` が 0・`opened` が true。B と D は `welcomeClosed` が true。`npm test --prefix app` が通る。上の `cargo test` の試験の各 target が通る。`guard/scan.sh` と `guard/flags.sh origin/main HEAD` の指摘が 0 件。

## 作業の進め方

- 作業の最初に `date -u` を記録する。**3 時間 15 分を過ぎたら**、受け入れが通らなくても、作業ツリーを commit し、報告を書いて終える(ジョブの上限は 5 時間 50 分)。
- ビルドは、(低) の段で 1 回(受け入れのコマンドの中の 1 回)。長いコマンドは、フォアグラウンドで直接実行し、シェルツールの `timeout` 引数を 5400000 以下にする。背景で起動して、終わりを待たない。
- 段ごとに commit する。`git add` は、変えたファイルを 1 件ずつ指定する(`git add -A`・`git add .` を使わない)。commit のメッセージは日本語で、AI が書いたことを示す行を付けない。`[skip ci]` を書かない。
- 受け入れが通らなくても、作業の最後に作業ツリーを commit する(未 commit の変更が残ると、判定は失敗する)。
- 報告(8 行以内): (a) 取り込んだ群と、ファイル数。(b) 受け入れの各判定の出力の行。(c) 試験の期待を書き換えた物と、その元の期待。(d) 次の試行の手順(残りの群・混在のハンク・計時器の試験の状態)。(e) 確認できなかった物(「確認できない」と書く)。実測と推定を分けて書く。「達成」「直った」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。

## 制約

- 製品に、新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数・旗を足さない。計時器の試験のための分岐も、製品に入れない(試験の形を変えて対応する)。
- Python を使わない(調査と検証にだけ使う)。OCR を使わない。
- sleep・nohup・末尾の `&`・先頭の `timeout` は使わない。待つときは、条件で待つ。
- 一時ファイルは `.tmp-*` に置き、終わったら消す(自分の物だけ)。
- `git stash` を使わない。commit の名義は、環境の設定のまま。commit・タグ・PR に AI の行を付けない。
- PR #1555 は閉じない・触らない・ラベルを付けない・マージしない。
- `app/`・`lift/`・`ci/` に触れる push は、main の `deploy.yml` を動かす。報告に、push したことを書く(push は判定の段が行う)。
- 内部の台帳 ID・禁止の数値・URL を、ファイル・commit・報告に書かない。

## 他のタスクとの関係

- `blocked_by` は無い(起動は即時)。
- 関連(blocked_by ではない): #1386・#1541・#1548・#1546・#1395・#1545・#1552・#1554(上の「背景」の重なり)。衝突したら、main を取り込んで直し、受け入れを通す。
- #1540(main で既に落ちている `fe-build` の 2 件)は、この Issue の受け入れで除く。直すのは #1540。
