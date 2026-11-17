# first idle の4条件の交互比較の手順書(ci1、仮の番号 0332。PM 11:4x の依頼。**実行は PM の合図のあと。いまは Chrome 0 の窓**)

## 目的
RT-A の first idle が E13 より約 +2.5 s(中央値 7,813 対 5,312 ms、5 組すべて RT-A が遅い。ci1 の測定、n=5)遅い理由を、1 つずつ外して切り分ける。候補: (1) 実スレッド(Worker の生成・pthread)そのもの、(2) page-rt1(RT の page 差分)、(3) bundle(wasm 158 MB、strip 済み)、(4) 診断の環境変数(cand1 は FLE_DIAG_* が manifest に入っている)。

## 4条件
| 名前 | assets | ページ | ポート | 何を外すか |
|---|---|---|---|---|
| `e13` | `fl-engine-ci-0363b050`(本番) | page-d0447902(E13 の page) | 5053 | 基準(協調) |
| `rta` | RT-A `fl-engine-test-rt1-91bf78f6`、manifest 無改変 | page-rt1 | 5101 | 実スレッド(N = clamp(n−1, 2, 8) = 8)の全部 |
| `rta-w1` | 同じ bundle + `FLE_WORKERS=1`(`assets-rta-w1`) | page-rt1 | 5108 | RT の bundle・page のまま、**Worker を 1 にして協調スケジューラ**(ランタイム: 0 または 1 は協調。`fle_thread.c`)。`rta` との差 = Worker の数だけ |
| `cand1-nodiag` | cand1 `fl-engine-test-g9f-cand1-f7ebcca1` の manifest から `FLE_DIAG_WIN` / `FLE_DIAG_SCROLL` / `FLE_DIAG_VFS_MISS` を除いた物(`assets-cand1-nodiag`) | page-d0447902 | 5109 | 協調・RT の page なし・診断なし。cand1 の wasm は strip 漏れで重複入り(166.97 MB)なので『bundle の大きさの影響』も見る |
読み(実行前の仮説。結果ではない): `rta` ≈ `rta-w1` なら Worker の生成が原因ではなく bundle / page の側。`rta-w1` ≈ `e13` なら RT の page・bundle は無関係で Worker の数が原因。`cand1-nodiag` と `e13` の差は『bundle の大きさ(+23 MB)と Filter の追加』、`cand1-nodiag` と cand1 の既存の値の差は『診断』。

## 手順(`tools/idle4.sh`。Chrome 156 を 1 本ずつ、冷間・新しいプロファイル)
1. サーバーは起動済み: 5053(E13)、5101(rta)、5108(rta-w1)、5109(cand1-nodiag)。`curl` で 200 を確認。
2. gate(各 run の前に `wait-quiet.mjs 8 15`: 開始 load ≤ 8・I/O 圧 0・swap 入出力 0。満たさない run は REFERENCE と印を付ける)、他の Chrome 0・Wine 0・MemAvailable ≥ 12 GB(`free -m`・`vmstat 1 3` を run の前に `out/steps.log` に)を確かめてから: `REF=rt1 bash tools/idle4.sh 5 e13:5053:d0447902 rta:5101:rt1 rta-w1:5108:rt1 cand1-nodiag:5109:d0447902`
3. 条件の順は round ごとに 1 つずつ回す(round i は i 番目の条件から始める)ので、窓の静けさの変動が条件に偏らない。1 条件 5 回 = 計 20 回、1 回 約 15〜40 s(wait-quiet の待ち次第)。
4. 出力: `out/idle4.tsv`(browser、名前、round、first idle ms、start load、end load、WebGL renderer、desk.mode)と末尾の条件別の値・中央値。結果は条件ごとに 5 組の中央値と組ごとの差で報告し、`rta` 対 `rta-w1` と `rta-w1` 対 `e13` の差を分けて書く。

## 注意
- first idle の絶対値は、この機械・この時点の load(RT-B の build が nice 19 で並走する間は 4〜8)に依る。比較は同じ round 内の対でみる。
- `rta-w1` の動作確認(FLE_WORKERS=1 で RT-A の page-rt1 が起動するか)はまだしていない(最初の run が smoke を兼ねる。停止したらそこで報告)。
- opt1 のように Chrome を使わない担当が走らせるなら、`serve.sh` の 4 サーバーを同じ assets・page で起動すればよい。
