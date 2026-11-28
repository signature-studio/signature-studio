---
draft: "270"
title: "PR #1555 の全体を、P3・P2 の A/B と smoke で測る(測定のみ。main には何も入れず、報告だけ commit する)"
type: "Task"
milestone: "M2 HiDPI と P2 の測定(P2)"
parent: "005"
blocked_by: []
labels: ["hidpi", "verify"]
size: "L"
priority: "High"
project: "signature-studio/3"
---

## 目的

パフォーマンス最適化の PR #1555(`perf/optimize-001`。基準は `332cbd66`)を、**PR の全体のまま**測る。main に入れる前に、次の 3 つを確かめる。

1. P3(音声の途切れ)の A/B で、main と比べて悪化が無い。
2. 1000 Hz の入力の下の最低の fps が、main より悪化しない。
3. main の e2e の gate(smoke)が、PR を入れた木で通る見込みがある。

加えて、計時器の枠(`app/src/engine/worker.ts` の `DEADLINE_SLOTS`)の取り消しの問題を、試験で確かめる。

この Issue は**測定のみ**。main には何も入れない。結果は所見 `pm-work/findings/pr-1555-measure.md` に書き、その所見だけを commit する。PR #1555 は閉じない・触らない・マージしない。

## 背景

- PR の変更は 61 ファイル。音声・協調スレッドの切り替え・計時器・起床の通知・描画・ビルドの最適化を含む。PR の検証は、起動・流れ A〜D・単体試験だけ。**音声の途切れ(P3)と 1000 Hz の入力の下の fps は、PR の検証に無い**。そのため、この測定で見る。
- main の製品の木(`app/` `lift/` `ci/` `tools/` `guard/` `.github/` `spec/`)は、PR の基準と同じ。base は、取り込みの前の製品(下の 2)。
- PR は、表示 Hz の上限(`app/src/engine/hz.ts`)を 120 から 300 に上げる変更を含む。**この変更は取り除かず、PR のまま測る。** 上限を上げる条件は「ミキサーが追いついたら」だが、測定で P3 が悪化しなければ、その条件を満たしたと見なす。判定は所見に書く。
- 計時器の枠の疑い(`pm-work/findings/pr-1555-review.md` の 2 節)。PR の版の `deadlineWait` は、16 個の枠を作成の順に循環させる。`cancelDeadlineWait` は、同じ枠の `Atomics.notify` で、生きた待ちを `'ok'` で起こす。PR の版は `'ok'` では発火しない。node の `Atomics.waitAsync` では、取り消しの後に生きた待ちも `"ok"` で解決することが、所見で実測されている。
- smoke の gate は `.github/workflows/e2e.yml` の matrix の 3 行(`4k-dpr1`・`4k-dpr2`・`4k-random`)。各行の判定は `tools/e2e/judge.jq`。
- この所見は、#1556(PR の取り込み)の入力になる。

## 読む物

- `pm-work/findings/pr-1555-review.md`(2 節: 計時器の枠。3 節: A/B の道具と時間の見積もり)。
- `tools/ab/run.mjs`・`tools/ab/gates.mjs`・`tools/ab/base.mjs`(冒頭の注釈)。
- `tools/p2-frames/measure.mjs`・`tools/p3-audio-probe/measure-live.mjs`(冒頭の注釈)。
- `tools/boot-check/boot-bundle.mjs`(起動の `elapsedMs`)。
- `tools/e2e/run.mjs`(冒頭の注釈)・`tools/e2e/judge.jq`・`.github/workflows/e2e.yml`(build の段と e2e の段)。
- `app/src/engine/worker.ts`(`deadlineWait`・`cancelDeadlineWait`・`DEADLINE_SLOTS`)。

## やること

### 0. 準備

- 作業の最初に `date -u` を記録する。`.prior-work.md` が在れば、最初に読む。
- `git fetch origin pull/1555/head` の直後に `git rev-parse FETCH_HEAD` で SHA を控える(以下 `PRHEAD`)。以後は SHA で参照する(`FETCH_HEAD` は他の fetch で上書きされる)。
- 取り込みの前の main の SHA を控える(`MAINSHA=$(git rev-parse HEAD)`)。base の ref は `MERGEBASE=$(git merge-base HEAD "$PRHEAD")`(`332cbd66` のはず)。

### 1. 作業の木を PR の内容にする

- `git merge --no-commit --no-ff "$PRHEAD"`。**commit しない。** 衝突が出たら、PR の版を採る(`git checkout --theirs -- <path>` の後に `git add -- <path>`)。
- `git diff --cached --name-only` が、PR の 61 件と一致することを確かめる。

### 2. base と changed の bundle を作る

- base(取り込みの前の製品): `node tools/ab/base.mjs "$MERGEBASE" .tmp-base` の後、`(cd .tmp-base && npm ci --prefix app && npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle)`。終わったら `node tools/ab/base.mjs --verify .tmp-base` が 0 で終わること。(推定: 約 100 分)
- changed(PR の木): 作業の木で `npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle`。smoke に使う build は、`.github/workflows/e2e.yml` の build の段の順に従う(`lift/target/release/fe-build endpoint` を含む)。(推定: 30 分台)
- 起動: `node tools/boot-check/boot-bundle.mjs "$PWD/.tmp-base/lift/out-app/bundle/raw/fl-Oz"` と、changed の bundle の両方の `elapsedMs`・`state`・`standIn`・`error` を所見の `bootMs` に書く(参考の数値。関門ではない)。

### 3. P3 の A/B(音声の途切れ)

```
node tools/ab/run.mjs p3 "$PWD/.tmp-base/lift/out-app/bundle/raw/fl-Oz" "$PWD/.tmp-base/app" "$PWD/lift/out-app/bundle/raw/fl-Oz" "$PWD/app" 3 > "$RUNNER_TEMP/ab-p3.json"
```

- 関門(道具の判定): `pass` が true。changed の各 run の `error` が null・`playing` が true・`inputRateOk` が true・`nonSilentDelta` が 0 より大きい・`pcmRepeat8192` が null か 0.01 未満。`underrunDelta`・`last60sUnderruns` の changed の中央値が、base の最大 + 4 以下。
- 所見の `underrunDelta`・`last60sUnderruns` に、base と changed の各 run の値、中央値、base の最大を書く。

### 4. P2 の A/B(1000 Hz 入力の下の最低の fps)

```
node tools/ab/run.mjs p2 "$PWD/.tmp-base/lift/out-app/bundle/raw/fl-Oz" "$PWD/.tmp-base/app" "$PWD/lift/out-app/bundle/raw/fl-Oz" "$PWD/app" 3 > "$RUNNER_TEMP/ab-p2.json"
```

- この測定の判定: `minWindowFps`(1000 Hz 以上の入力の下の最低の fps)の **changed の中央値 ≥ base の中央値**。悪化していないこと。所見の `minFps` に、base と changed の各 run の値と中央値を書く。
- 道具の関門も併記する: `minWindowFps` の changed の中央値 ≥ base の中央値 × 0.90、`inputToPresentMsP95` の changed の中央値 ≤ base の中央値 × 1.20、`pass`。

### 5. 計時器の枠の試験

- 試験のために `deadlineWait`・`cancelDeadlineWait` を別のモジュール(例: `app/src/engine/deadline-wait.ts`)に出してよい。振る舞いは変えない。製品に、テスト用の分岐・旗・環境変数を入れない。
- 試験(`app/test/unit/` の vitest。Node は `Atomics.waitAsync` を持つ): 同じ枠に 2 つの待ちを置く。枠は作成の順に 16 で循環するので、1 件目と 17 件目が同じ枠になる(間に 15 件を作る)。1 件目を取り消し、17 件目の期限(数十ミリ秒)に、発火されることを確かめる。
- 手順: (i) PR の版で、試験が**赤**になることを確かめる(赤にならなければ、その事実と理由を所見に書く)。(ii) 最小の直しを書く(例: `'ok'` で起きたときも、期限を過ぎていれば発火し、過ぎていなければ待ち直す。または、枠を待ちの数だけ用意する)。(iii) 緑にする。
- **試験と直しは commit しない。** 試験のファイルと直しの `git diff` を、所見に貼る。

### 6. smoke(main の e2e の gate)

- 準備: `.github/workflows/e2e.yml` の e2e の段と同じ(`(cd tools/e2e && npm ci && npx playwright install chromium)`)。demo の曲のフォルダは、作業ジョブで既に在る物を使う。無ければ、所見に書く。
- 3 行を、それぞれ別の実行で:
  ```
  node tools/e2e/run.mjs --scenarios smoke --displays 4k-dpr1 > "$RUNNER_TEMP/e2e-dpr1.jsonl" && jq -e -s -f tools/e2e/judge.jq "$RUNNER_TEMP/e2e-dpr1.jsonl"
  ```
  `4k-dpr2`・`4k-random` も同じ形で行う。
- 判定: 3 行とも `judge.jq` が 0 で終わる。所見の `smoke` に、行ごとの結果(合否。落ちた件数と ID)を書く。

### 7. 関連の試験

- `npm test --prefix app`(全件)。
- `cargo test --release --locked --manifest-path lift/Cargo.toml -p fe-lift`(全件。時間が足りなければ、`--test gdi --test gdi_bulk --test screen1 --test raster --test flags --test int_ext --test mxcsr_flags --test size1_deadflags --test alpha2_changed --test alpha3_updregion --test alpha7_showwin --test alpha8_hidden_move --test desk_scene --test desk1_compose --test wall_fr3` だけを走らせ、その旨を所見に書く)。
- `cargo test --locked --manifest-path lift/Cargo.toml -p fe-build`(main で既に落ちている 2 件は #1540 の範囲。ここでは直さず、結果だけを所見に書く)。

### 8. 所見を書く

`pm-work/findings/pr-1555-measure.md` に、次の順で書く。**下の語は、そのまま書く**(受け入れがこの語を探す): `PRHEAD`・`underrunDelta`・`last60sUnderruns`・`minFps`・`bootMs`・`smoke`・`recommend:`。

1. 対象: `PRHEAD` の 40 桁の SHA、`MAINSHA`、`MERGEBASE`、測った時刻(`date -u`)。
2. 関門の表: 関門ごとに、合・否・確認できない。(P3 の A/B の各関門 / P2 の最低 fps の「悪化しない」 / P2 の道具の関門 / 計時器の試験(赤から緑へ) / smoke の 3 行 / `fe-lift`・`fe-build` の試験 / 起動)。
3. 数値: `underrunDelta`・`last60sUnderruns`(base と changed の各 run、中央値、base の最大)、`minFps`(base と changed の各 run、中央値)、`bootMs`(起動の `elapsedMs`。base と changed)、`smoke`(行ごとの判定)。
4. 計時器: 試験の内容、赤の出力、直しの `git diff`(commit しない)。
5. 表示 Hz の上限(300)の扱い: P3 が悪化したか。
6. 確認できなかった物(理由つき。時間切れの物を含む)。
7. `recommend:` の行: 次の 3 つのどれか 1 つ。「マージしてよい」「条件つき(条件を書く)」「マージしない(理由を書く)」。

### 9. 所見だけを commit する

- 所見を書き終えたら、試験と直しの `git diff` を所見に貼ってから、`git reset --hard HEAD` で PR の差分と直しを捨てる。作った未追跡の試験のファイルは消す。所見は未追跡のまま残す(`reset` の前に `git add` しない)。
- `git status --porcelain` が、`?? pm-work/findings/pr-1555-measure.md` の 1 行だけになることを確かめる。
- `git add pm-work/findings/pr-1555-measure.md` の後に commit する(日本語の題名。AI の行・`[skip ci]` を付けない)。push は手で行わない(判定の段が、通ったときだけ push する)。

## 受け入れ

所見が、測定の語と推奨を含むことを確かめ、規則の検査を通す(1 行のコマンド)。

```accept
test -f pm-work/findings/pr-1555-measure.md || exit 1 && node -e 'const s=require("fs").readFileSync("pm-work/findings/pr-1555-measure.md","utf8");const k=["underrunDelta","last60sUnderruns","minFps","bootMs","smoke","recommend:"];process.exit(/\b[0-9a-f]{40}\b/.test(s)&&k.every((w)=>s.includes(w))&&/recommend:\s*(マージしてよい|条件つき|マージしない)/.test(s)?0:1)' || exit 1 && bash guard/scan.sh || exit 1
```

## 作業の進め方

- 作業の最初に `date -u` を記録する。**4 時間 30 分を過ぎたら**、結果が揃わなくても、取れた物と取れなかった物(理由つき)を所見に書き、9 の手順で commit して終える。ジョブの上限は 5 時間 50 分で、時間切れでは報告が書けない。
- 長いコマンド(ビルド・A/B・smoke・試験)は、フォアグラウンドで直接実行し、シェルツールの `timeout` 引数に 5400000 以下を渡す。背景で起動して、終わりを待たない。`sleep`・`&`・先頭の `timeout` は使わない。
- 時間の見積もり(推定。実測ではない): base の build 約 100 分、changed の build 30 分台、P3 の A/B 約 20 分、P2 の A/B 約 10 分。smoke の 3 行は見積もりが無い(確認できない)。
- 順序の目安: 5(計時器の試験)は、ビルドを要しないので先に行ってよい。次に 2(build)・3・4(A/B)・6(smoke)・7(試験)。所見は、結果が出るたびに追記する。

## 制約

- 製品に、新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数・旗を足さない。計時器の試験のための分岐も、製品に入れない。
- Python を使わない(調査と検証にだけ使う)。OCR を使わない。
- main へは、所見だけを commit する。PR の差分・試験・直しは commit しない。
- `git stash` を使わない。`git add` は、所見のパスだけ。ブランチを作らない。
- commit の名義は、環境の設定のまま。commit・所見に AI の行を付けない。`[skip ci]` を書かない。
- PR #1555 は閉じない・触らない・ラベルを付けない・マージしない。
- `gh workflow run` を打たない。
- 一時ファイルは `.tmp-*` に置き、終わったら自分の物だけ消す。
- 内部の台帳の ID・URL・ベンダーの名前・クラウドの識別子を、ファイルと所見に書かない。

## 他のタスクとの関係

- `blocked_by` は無い(起動は即時)。
- この所見は、#1556(PR の取り込み)の入力になる。この Issue は #1556 を変えない。
- #1540(main で既に落ちている `fe-build` の 2 件)は、ここでは直さない。結果だけを所見に書く。
