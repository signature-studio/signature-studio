---
draft: "214"
title: "fe-lift の試験: 試験の native な link が未定義のシンボルで落ち、約 730 件が新しく失敗する(`fle_thread.c` の `__real_pthread_mutex_unlock`・`fle_glob.c` の関数・`fle_clip_host_probe`)"
type: Task
milestone: "M4 FL に必要な物の配布"
parent: "012"
blocked_by: []
labels: ["verify"]
size: M
priority: High
project: signature-studio/3
---

## 目的

`lift/runtime/fle_thread.c` の `__wrap_pthread_mutex_unlock`(#1382)は `__real_pthread_mutex_unlock` を呼ぶ。製品の wasm の link は `lift/harness/web/link.rsp` の `-Wl,--wrap=pthread_mutex_unlock` でこの名前を解く。しかし試験の native な link には `--wrap` が無く、`__real_pthread_mutex_unlock` が未定義になって link が落ちる。同じ型の未定義が、`fle_glob.c` の関数と `fle_clip_host_probe` にもある(下の背景)。その結果、`node tools/fe-lift-test.mjs` は、link に失敗した試験を約 730 件、新しく落とす。

この Issue の目標は 2 つ。
1. 試験の link の未定義を全て解き、`node tools/fe-lift-test.mjs` の新しい失敗を 0 にすること(既知の一覧と host 依存の一覧の物は除く)。
2. 直った試験を `lift/crates/fe-lift/tests/known-failures.txt` から消し、その件数を `spec/fe-lift-test.test.mjs` に合わせること。

製品の動作は変えない。試験の link と、試験専用の定義(`tests/support/`)だけを直す。

## 背景

- 判定の再実行(main の現状。`node tools/fe-lift-test.mjs`): 「通った 1,379 件・既知の失敗 53 件・新しい失敗 729 件・実行した binary 163 件」。新しい失敗が多い binary: gdi 85・wall_fr11 49・wall_fr8 41・gdi_bulk 35・locale 33・wall_fr9 26・wall_fr14 24・wall_fr15 21・vmap 21・wall_fr17 20・wall_fr12 20・vol 19・alpha10_syscmd・alpha1_glass・alpha2_changed ほか。
- 失敗の形: 多くの試験は、テストの中で `clang` を呼び、試験のプログラムと `lift/runtime/fle_*.c` を link する。link が失敗すると、試験は `clang failed for ...: ... undefined reference to ...` で落ちる。binary の build は通るので、失敗は試験ごとに数えられる。
- 静的に確かめた未定義の 3 種類(ファイルを読んで数えた値。試験は走らせていない):
  (a) `__real_pthread_mutex_unlock`(#1382 の `lift/runtime/fle_thread.c`)。`fle_thread.c` を link する試験は約 35 ファイル。試験ごとに clang の呼び出しと link の一覧を持ち、共通の helper は無い。
  (b) `fle_glob_fixed_add`・`fle_glob_fixed_release` など(`lift/runtime/fle_imports.c` が `lift/runtime/fle_glob.c` の関数を呼ぶ)。`fle_imports.c` を一覧に持ち、`fle_glob.c` を持たない試験は 59 ファイル。`tests/apis_common/mod.rs` と `tests/fsx_common/mod.rs` は、それを使う試験が共有するので、その 1 箇所を直すと複数が直る。
  (c) `fle_clip_host_probe`(#1406 の `lift/runtime/fle_clip.c`)。`lift/crates/fe-lift/tests/support/clip_host_stub.c` に定義が無い。このファイルを使う試験は 27 ファイル(`wall_ia1.rs`・`wall_ia1w.rs` は自前の定義を持つ)。`clip_host_stub.c` の 1 箇所で直る。
- 前例: commit `6f9cfb95`(「fe-lift の試験の link と入力を直す(509 → 59)」)は、`tests/support/clip_host_stub.c` と `win_trace_off.c`(weak 定義)を、試験ごとに link へ 1 行足す形で直した。記録は `pm-work/findings/fe-lift-test-baseline.md`。
- 前回の試行(#1425)の差分 `pm-work/findings/fe-lift-link-fix-1425.patch`(main に在る)は、3 試験(callback・wall_fr20・wall_fr9t)について (a)(b)(c) を直した。内容: `tests/support/mutex_real_stub.c`(新規。`__real_pthread_mutex_unlock` を libc の `pthread_mutex_unlock` に解く)、`clip_host_stub.c` に `fle_clip_host_probe`、`wall_fr20.rs` の「0147 の行は最後の行」の試験を名前で探す形、`known-failures.txt` の wall_fr20 の 1 行を消し、`spec/fe-lift-test.test.mjs` の件数を 60 から 59 に直す。この差分は 3 試験にしか当たらない。全体は直らない。
- `tools/fe-lift-test.mjs` の判定: 失敗は known の一覧と比べ、known に無い失敗と build の失敗は終了 1。直った試験は「直った」と出すが、`--strict-known` が無ければ終了コードは変わらない。known から消すのは手で行う。

## 読む物

- `lift/runtime/fle_thread.c`(861 行〜915 行付近。`__wrap_pthread_mutex_lock`・`__wrap_pthread_mutex_unlock`)
- `lift/harness/web/link.rsp`(`--wrap=pthread_mutex_unlock` と、製品の link の一覧)
- `lift/runtime/fle_imports.c`・`lift/runtime/fle_glob.c`(`fle_glob_fixed_add`)・`lift/runtime/fle_clip.c`(`fle_clip_host_probe`)
- `lift/crates/fe-lift/tests/`: clang を呼ぶ試験(`grep -rln 'Command::new("clang")' lift/crates/fe-lift/tests`)、それぞれの link の一覧(`RT_FILES`・`FILES`・`runtime_sources` など)、`apis_common/mod.rs`・`fsx_common/mod.rs`、`support/`
- `pm-work/findings/fe-lift-link-fix-1425.patch`(前回の差分)・`pm-work/findings/fe-lift-test-baseline.md`(前例)
- `tools/fe-lift-test.mjs`・`lift/crates/fe-lift/tests/known-failures.txt`・`tools/fe-lift-host-dependent.txt`・`spec/fe-lift-test.test.mjs`(known の件数を固定している)

## やること

0. 作業ツリーの確認(`git status`)。`.prior-work.md` が在れば最初に読む。`date -u` を記録する。
1. 再現と内訳: `node tools/fe-lift-test.mjs` を全体で 1 回走らせ、標準出力を `.tmp-fe-lift-before.txt` に保存する(約 46 分。フォアグラウンドで直接実行する)。link の失敗を、試験の binary ごと・未定義のシンボルごとに数え、表にする(binary・未定義のシンボル・件数)。個別の binary を確かめる時は `node tools/fe-lift-test.mjs --test <binary>` を使う。
2. 直し方を決める: (a)(b)(c) のそれぞれについて、変更の行数が一番少なく、かつ 1 箇所の変更で多くの試験が直る形を選ぶ。(c) は `clip_host_stub.c` に定義を足すのが 1 箇所の変更。(b) は、link の一覧に `fle_glob.c` を足すか、共通の helper(`apis_common`・`fsx_common`)を直す。(a) は、試験専用の定義(`tests/support/` の C。`__real_pthread_mutex_unlock` を `pthread_mutex_unlock` に解く。weak にするかは選ぶ)を、link する試験ごとに足す。前回の差分は `git apply --check pm-work/findings/fe-lift-link-fix-1425.patch` で確かめ、当たる部分は使う。
3. 製品の意味を変えない: `lift/runtime/`・`lift/harness/` の製品の C は変えない。試験専用の定義は `lift/crates/fe-lift/tests/support/` に置き、link する試験だけが使う。`#ifdef`・環境変数・オプションを足さない。試験の期待値を、通すために書き換えない。試験の数え方を直すのは、その前提が古いと確かめた時だけ(1425 の wall_fr20 が例)。
4. 全体を走らせる: `node tools/fe-lift-test.mjs` を、標準出力を `.tmp-fe-lift-after.txt` に保存して走らせ、新しい失敗が 0 であることを確かめる。直った試験(「直った:」の行)は `known-failures.txt` から消し、`spec/fe-lift-test.test.mjs` の件数を合わせる。
5. 作業ツリーを commit する。`.tmp-*` の一時ファイルは commit せず、終わったら消す。

## 受け入れ

```accept
node tools/fe-lift-test.mjs && node --test spec/fe-lift-test.test.mjs && bash guard/scan.sh lift/crates/fe-lift/tests tools spec
```

判定の基準:
- `node tools/fe-lift-test.mjs` が終了コード 0。出力に「新しい失敗」の行(build の失敗を含む)と「問題:」の行が無い。
- `node --test spec/fe-lift-test.test.mjs` が通る(known の件数が一覧と合う)。
- `guard/scan.sh` の指摘が 0 件。

## 報告の規則

- 報告の先頭に、直す前と後の `node tools/fe-lift-test.mjs` の最後の行(「fe-lift の試験: 通った … 件・既知の失敗 … 件・…・直った … 件・新しい失敗 … 件・実行した binary … 件」)を、そのまま貼る。
- link の失敗の表(binary・未定義のシンボル・件数)を、直す前と後で並べる。
- 直した方式と理由(どの 1 箇所を変えたか、なぜそれが一番少ないか)。
- 直した試験の一覧(known から消した行)と、spec の件数の変更。
- 直せなかった失敗は、binary と試験名と、出力の 1 行を書く。
- 「達成」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。出力の行を付ける。

## 制約

- 製品に、利用者向けの新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足さない。
- `lift/runtime/`・`lift/harness/`・`lift/crates/fe-lift/src/` の製品の動作を変えない(試験の link と試験専用の定義だけを直す)。
- 禁止されているのは、AI のベンダー名と Cloudflare の識別子だけ。
- Python を使わない。OCR を使わない。
- sleep・timeout のコマンド・nohup・末尾の `&` は使わない。一時ファイルは `.tmp-*` に置き、終わったら消す。

## 作業の進め方

- 作業の最初に `date -u` を記録する。4 時間 30 分を過ぎたら、受け入れが通らなくても、作業ツリーを commit し、報告を書いて終える(ジョブの上限は 5 時間 50 分)。
- `node tools/fe-lift-test.mjs` は全体で約 46 分。フォアグラウンドで直接、シェルツールの `timeout` 引数は最大 5400000(90 分)にする。背景で起動して待たない。全体の実行は直す前と後の 2 回まで。
- 受け入れが通らなくても、作業の最後に作業ツリーを commit する。

## 他のタスクとの関係

- #1415 と #1370 は、この Issue を blocked by にする。どちらも fe-lift の全体の試験を受け入れに持つので、この Issue が閉じるまで起動しない。
- 元になった差分: #1382(`fle_thread.c` の mutex の wrapper)・#1406(`fle_clip_host_probe`)・#1425(前回の直しの試行)。
