---
draft: "130"
title: "fe-build の blend2d のソースの取得を、一時の失敗に強くする(再試行と、取得済みの物の再利用)"
type: Task
milestone: "M4 FL に必要な物の配布"
parent: "012"
blocked_by: []
labels: ["delivery"]
size: M
priority: High
project: signature-studio/3
---

## 目的

受け入れの連鎖(Issue 本文の accept の `rm -rf lift/out-app/b2d-src && … lift/target/release/fe-build engine …`)は、ほぼ全ての Issue が通る。その中で、毎回 blend2d のソースを GitHub から取るため、取得の一時の失敗で作業の判定が落ちる。この Issue は、一時の失敗で判定が落ちないようにする。

この Issue の目標は次の 2 つ。
1. 取得の一時の失敗(接続の失敗・403 など)を、再試行で吸収し、取れれば通ること。
2. 取得した物を固定の SHA-256 で検査し、取得済みの物を再利用できること。取得元の版(commit)は変えない。

## 背景

- 事実 1(#1365 の判定。PM の記録): blend2d の GitHub への接続が 133 秒で失敗し、作業が落ちた。
- 事実 2(#1355 の作業。PM の記録): 取得が 403 で失敗した。取り直すと通った。
- 今の取得(`lift/crates/fe-build/src/main.rs` の `blend2d_clone`): `git clone` で全履歴を取り、`git -C … checkout` で固定の commit に合わせ、`patch -p1` で `lift/harness/b2d1/b2d3-compop.patch` を当て、`CheckSources` で一覧を検査する。取得の再試行は無く、1 回の失敗で `fe-build engine` が止まる。
- 固定の値: 版(commit)は `main.rs` の `BLEND2D_REV`(a7f947677e85441abe1769586d3bb935b4086a07)。同じ commit の tarball(GitHub のアーカイブ)の SHA-256 は、`lift/harness/b2d1/build_upstream.sh` の `SHA` の既定値(391a57ad2508ab2291ad04b39b0a82d78473662b6d3029dbaf18de06165ea1f0。計測は 2026-11-03)。この道具は旧の opt-in の道具で、engine の作りには使われない。その取得の形(`curl` で tarball を取り、SHA-256 を検査し、`tar -xzf` で `--strip-components=1` して展開する)を使ってよい。
- GitHub はアーカイブを作り直すことがあり、同じ commit でも SHA-256 が変わりうる(`build_upstream.sh` の注釈)。

## 読む物

- `lift/crates/fe-build/src/main.rs`: 24〜27 行付近(blend2d の定数)、171 行付近(`blend2d_clone`)、237 行付近(`plan` の blend2d の stage)、325 行付近(`exec`)、351 行付近(`run`)、448 行付近(`sha256_hex`。既存。tarball の検査に使える)、602 行付近からのテスト(`plans_match_recorded_texts`)
- `lift/crates/fe-build/tests/plan/engine.txt`(先頭の `stage blend2d` の 6 行。上のテストが比べる記録)
- `lift/harness/b2d1/build_upstream.sh`(冒頭の注釈と、`COMMIT`・`SHA`・取得の行。読むだけ)
- `lift/harness/b2d1/blend2d-sources.txt`・`lift/harness/b2d1/b2d3-compop.patch`(取得の後の処理。読むだけ)
- `.github/workflows/deploy.yml`(`fe-build engine` を呼ぶ。読むだけ)
- `.github/workflows/agent.yml` の extract ジョブ(R2 のキャッシュの作り方の例。読むだけ)

## やること

1. 取得を、固定の commit の tarball 1 つにする。`git clone` と `git -C … checkout` をやめる(git は要らない)。取得は `curl`、展開は `tar`。展開先は従来どおり `lift/out-app/b2d-src`。`patch -p1` と `CheckSources` は変えない。
2. tarball は `lift/out-app/` の下に、commit の名前で置く(例: `lift/out-app/blend2d-<commit>.tar.gz`。`.gitignore` の `lift/out*/` で除外され、コミットしない)。展開の前に、`sha256_hex` で SHA-256 を計算し、固定の値(`main.rs` の定数)と比べる。固定の値は `main.rs` に書く。環境変数にしない。
3. 再試行: 取得の 1 回の試行は、curl の失敗、または SHA-256 の不一致で失敗とする。試行は最大 4 回(初回を含む)。失敗のたびに待ち、待つ時間は 10 秒・30 秒・60 秒とする(最後の失敗の後は待たない)。待ちは `fe-build` の中の `std::thread::sleep`(シェルの sleep ではない)。回数と時間は定数にする。
   - 途中のファイルを残さない: `.part` の名前に取り、検査が通ったら本来の名前にする。
   - 失敗のたびに、標準エラーに 1 行を出す: 何回目か(`attempt 2/4` の形)、取得元(commit のアーカイブ。秘密を含まない)、エラー(curl の終了の状態と標準エラーの 1 行。SHA-256 の不一致なら、期待の値と得た値)。
   - 4 回とも失敗したら、`fe-build` は失敗で止まる(終了の状態は 0 以外)。
4. 取得済みの物の再利用: `lift/out-app/` の tarball が在り、SHA-256 が固定の値と一致すれば、取得の試行をしない。展開は毎回やり直す(`lift/out-app/b2d-src` は受け入れの `rm -rf` の対象)。一致しない tarball は捨てて、取り直す。
5. 取得の処理と待ちの処理を、関数として差し替えられる形にする(試験で、実際には取らず、実際には待たない)。
6. 記録の更新: `tests/plan/engine.txt` の blend2d の行は、新しい手順に合わせて直してよい。記録の rt の件数の不一致(既存。PM の記録では 91 と 90 で、f4cffe71 以来)は、この Issue の範囲外なので直さない。

## 受け入れ

```accept
cargo test --locked --manifest-path lift/Cargo.toml -p fe-build blend2d_fetch_ && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine
```

試験の名前は `blend2d_fetch_` で始める。次の試験を入れる(偽の取得と偽の待ちを使う。ネットワークを使わない。試験で一時ファイルを作るなら、リポジトリの中の `.tmp-*` に作り、試験の最後に消す。バイト列を受ける形にしてファイルを作らなくてもよい)。
- 2 回失敗して 3 回目に成功する → 全体が成功する。試行は 3 回。待ちは 10 秒・30 秒。
- 4 回失敗する → 全体が失敗する。試行は 4 回。待ちは 10 秒・30 秒・60 秒。
- 固定の SHA-256 と一致する tarball が在る → 取得の試行は 0 回。
- 固定の SHA-256 と一致しない tarball が在る → それを捨てて取り直す(試行が 1 回以上)。

判定の基準:
- 試験 `blend2d_fetch_` が全て通る。
- `fe-build engine` が、GitHub からの実際の取得で通る(受け入れの最後の段)。
- 取得した tarball の SHA-256 が、固定の値と一致する(`sha256sum` の出力を報告に貼る)。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 「直った」と書かず、試験の結果と取得の結果で書く。出力の行を付ける。
- 報告に書く: 変えたパス(`lift/crates/fe-build/` の中。`lift/` の変更は main への push で `deploy.yml` を動かすので、その旨)、固定した commit と SHA-256、受け入れの `fe-build engine` の取得の試行の回数と、失敗の行(`attempt n/4`)の有無、試験の名前と結果、R2 のキャッシュのための材料(`lift/out-app/` の tarball の名前と、その SHA-256。PM が extract ジョブの鍵を決める)。SHA-256 が一致しなかった場合は、得た値。

## 制約

- 製品に、新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足さない。回数と時間は定数にする。
- 版(commit)は変えない。SHA-256 が合わないときは、固定の値を書き換えない。止めて、得た値を報告に書く。
- 変更は `lift/crates/fe-build/` の中に限る(`tests/plan/engine.txt` を含む)。`lift/harness/`・`.github/`・`app/`・`ci/` は読むだけ。
- Python を使わない。ベンダー名と Cloudflare の識別子を書かない。
- シェルでの sleep・timeout・nohup・末尾の `&` は使わない(Rust の `std::thread::sleep` は使ってよい)。一時ファイルは `.tmp-*` に置き、終わったら消す。

## 作業の進め方

- engine の作り直しは約 35 分。ビルドは、フォアグラウンドで、シェルツールの `timeout` 引数に大きい値(最大 5400000)を渡して待つ。
- 試験は取得を伴わない単体試験なので速い。先に試験を通してから、engine を作る。
- 開始から 4 時間 30 分を過ぎたら、受け入れが通らなくても、作業ツリーを commit して、報告を書いて終える。
- 作業の最後に、受け入れが通らなくても、作業ツリーを commit する。

## 他のタスクとの関係

- blocked_by: なし。
- この Issue は、受け入れの連鎖を使う全ての Issue の判定の失敗を減らす。他の Issue の本文は変えない。
- R2 のキャッシュ(`agent.yml` の extract ジョブで 1 回取って保存し、作業ジョブが復元する形)は、PM が行う。この Issue の範囲外。
- `fe-build` は `deploy.yml` からも呼ばれる。`deploy.yml` の手順は変えない。
