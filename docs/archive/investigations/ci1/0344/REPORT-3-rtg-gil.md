# 0344-3: RT-G の GIL 診断つき再生中 T(run-tp.sh の採り直し) — ci1, 2026-11-08

## 実際に実行した物

- 実 Chrome 156 headless(CDP)。RT-G bundle `fl-engine-ci-b083dc31`(wasm 170,941,751 B、SHA256SUMS 9/9 OK)。manifest 複写に `FLE_DIAG_ATOM=1 FLE_DIAG_GIL=1`(`mk-bundle-assets.sh` で作成。mt3 の `mk-rtg-assets.sh` は project の複写と serve の表示が付くので、同じ manifest になる手順を直接使った。bundle 本体は書き換えていない)。`assets-rtg-gil`、port 5142、page-rt1。
- 診断 ON なので **counted ではない**(非計時の診断 run)。0343 と同じ `run-tp.sh` とシナリオ(再生 → 10 s 後に CDP Tracing + v8 cpu_profiler → File メニュー開閉 12 回 → 停止)。
  | run | 時刻 | 開始前 load | MemAvailable | swap si/so | Wine | 他 Chrome | engine-log |
  |---|---|---|---|---|---|---|---|
  | tp-3-rtg-gil | 06:23:21–06:25:05 | 1.01 | 36643 MB | 0/0 | 0 | 0 | なし(`scn-tp.json` は saveLog を持たない) |
  | tp-4-rtg-gil | 06:25:25–06:27:07 | 2.86 | 28766 MB | 0/0 | 0 | 0 | あり(`scn-tp-log.json` = `scn-tp.json` + saveLog。測る部分は同一) |
  最初の起動(06:22:37)は swap si=112 で gate が止め、run していない。
- **`fl-diag: gil` 行は出た**(tp-4: 6,318 行 / engine-log 7,270 行)。PM の「出ない場合は止めて報告」には該当しない。Worker 別 W0〜W7 の周期 dump(各 87〜93 回)。

## GIL の待ち・保持(Worker 別、累積)

**注意**: engine-log に時刻が無く、dump は周期的なので、下の数値は **エンジン起動から run の終わりまでの累積**。File click の phase C だけの値ではない。0343 の phase C との対応表は作れない(下の「対応」)。

| Worker | 取得回数 | 待ち <0.1 ms | <1 | <5 | <20 | <50 | ≥50 ms | 最長の待ち ms | 保持(標本)n | 保持 ≥1 ms | 最長の保持 ms | 保持の平均 ms |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| W0 | 13,758,390 | 13,745,608 | 12,758 | 24 | 0 | 0 | 0 | 3.74 | 954,871 | 748(<5: 320, <20: 428) | 17.955 | 0.0059 |
| W1 | 182,656 | 174,823 | 7,802 | 29 | 2 | 0 | 0 | 13.38 | 47,611 | 2 | 1.585 | 0.0047 |
| W2 | 304,044 | 293,950 | 10,051 | 42 | 1 | 0 | 0 | 6.535 | 73,364 | 1 | 2.82 | 0.0048 |
| W3 | 152,300 | 144,574 | 7,710 | 16 | 0 | 0 | 0 | 3.905 | 45,358 | 1 | 2.22 | 0.0047 |
| W4 | 202,744 | 193,059 | 9,663 | 22 | 0 | 0 | 0 | 4.67 | 58,531 | 1 | 1.155 | 0.0046 |
| W5 | 1,897,652 | 1,881,827 | 15,786 | 37 | 1 | 0 | 1 | 89.095 | 220,162 | 2 | 1.63 | 0.0027 |
| W6 | 160,034 | 152,303 | 7,709 | 22 | 0 | 0 | 0 | 3.92 | 46,075 | 1 | 2.87 | 0.0047 |
| W7 | 171,220 | 163,341 | 7,841 | 36 | 2 | 0 | 0 | 14.04 | 48,022 | 1 | 1.52 | 0.0053 |

(「保持 ≥1 ms」は「保持(標本)の <1 を除く列の合計」。W0 の保持の内訳は <0.1: 953,930 / <1: 193 / <5: 320 / <20: 428 / <50: 0 / ≥50: 0。全 Worker の表は `gil-table-rtg.md`。)

- 全 Worker の取得 16,829,040 回のうち、0.1 ms 以上待ったのは 79,555 回(0.473%)。**99.5% は 0.1 ms 未満で取れている。**
- **W0**(取得回数が最多で、`WaitMessage` を持つので、ゲストの main thread = 0343 の UI Worker とみられる。**これは推定**で、W 番号と trace の Worker の対応は確認していない): 取得 13,758,390 回のうち 0.1 ms 以上の待ちは 12,782 回(0.093%)、最長の待ち 3.74 ms。5 ms 以上待った回は 0。
- 保持の「標本」は 1,493,994 件(取得の約 9%)で、0.1 ms 未満が 99.9%(1,492,790)。0.1 ms 以上の保持が 1,204 件、1 ms 以上が 757 件(W0 が 748 件)。
- **W0 の保持の長い物**: `> WaitMessage` が 686 件・最長 17.955 ms(16 ms 前後が並ぶ)。WaitMessage は、メッセージを待つ間に GIL を保持している時間として記録されている。W0 の他の API は最長 2.25 ms(`call_guest`)、2.17 ms(`> CreateDIBSection`)、0.485 ms(`> BitBlt`)で、WaitMessage 以外はすべて 2.3 ms 以下。
- W1〜W7 の保持の長い物(最長 2.87 ms): `switch_to`(954 件・最長 2.87)、`idle wait`(909 件・最長 2.82)、`> LeaveCriticalSection`(728 件・最長 1.38)、`idle wait over`(348 件)、`> EnterCriticalSection`、`> PostMessageW`、`> CreateDIBSection`。
- **W5 の取得 1,897,652 回は他の pthread Worker(15–30 万回)の約 6〜12 倍**。W5 に長い待ち 89.095 ms が 1 回ある(≥50 ms の列の 1)。何を待ったかは top-held には出ていない(top-held は保持の記録で、待ちの記録ではない)。

## 0343 の phase C との対応(測れた範囲)

- 0343 の RT-F: 再生中 File click の phase C(UI Worker の処理)は 44〜47 ms、うち **fle_gil_wake + fle_gil_acquire_slow の self が約 1 ms/入力**。
- RT-G の trace(同じ手順の run 3・4)でも phase C は 44.81 / 47.17 ms(中央値)、UI Worker の処理時間は 39.3 / 44.0 ms/入力、`fle_gil_*` の self は 0.70 / 1.33 ms/入力(RT-F の 0.97 / 0.74 と同程度)、g9_hook の包含時間は 30.02 / 31.58 ms/入力(RT-F の 34.90 / 31.82)。**GIL の診断を入れた RT-G でも、phase C の形は RT-F と同じ。**
- GIL の待ちの累積(W0)は、取得 13.8M 回のうち 0.1 ms 以上の待ちが 0.093%、最長 3.74 ms(5 ms 以上は 0 回)。**累積の分布からは、+23 ms を GIL の待ちで説明する形は見えない**(W0 の最長の待ちが 3.74 ms で、+23 ms より 1 桁小さい)。ただしこれは run 全体の累積で、phase C の窓だけの値ではない。phase C の間に W0 が何回 GIL を取り、何 ms 待ったかは **未取得**。
- (program) の中身は GIL の待ちではない(0343 の測定: (program) の祖先に fle_gil_* は 0 ms)。0343 で未取得だった「(program) が待ちか native 処理か」は、今回の GIL 診断でも**区別できていない**。

## 未取得 / 未確認

- phase C の窓だけの GIL の待ち・保持(engine-log に時刻が無い。診断の出力に時刻を付ける必要がある → mt3 に依頼する案: dump の行頭に performance.now() 相当の値)。
- W5 の 89.095 ms の待ちが何か。
- W0 の `> WaitMessage` の保持(最長 17.955 ms)が File click の T と関係するか。WaitMessage 中に GIL を保持している記録は、W0 が入力を待っている間の時間である可能性があるが、**これは推測で、確認していない**。
- ATOM 診断(`fl-diag: atom`)の行も出ている(0343 の RT-F の run と同様)。今回は解析していない。
- tp-3 の engine-log(saveLog なし)。tp-4 のみ。

## 保存先

`0344/`: この REPORT、`gil-table-rtg.md`(全 Worker の表)、`gil-table.py`、`tp-{3,4}-rtg-gil/`(lat-tp-play.json、report.json、gate.txt、sync-*.json、tp-4 は engine-log.log)、`tp{1..4}-{3,4}-rtg.md`(0343 と同じ解析の出力)、`scn-tp-log.json`、`run-tp.sh`。生 trace(各 117 MB)は `lift/out-ci1/kit/out/tp-{3,4}-rtg-gil/tp.trace.json`、複写していない。
