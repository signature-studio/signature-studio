# RT-E (fl-engine-ci-87af8f55) O20 full-song C — ci1, 2026-11-07

## 実際に実行した物

- 実エンジン(E13=協調版, rte=RT-E fl-engine-ci-87af8f55)を実 Chrome 156 headless で実行。4 run: E13 shown 17:32:32–17:38:32、RT-E shown 17:38:40–17:44:42、E13 allhidden 17:45:05–17:51:15、RT-E allhidden 17:51:24–17:57:37。scenario は `tools/scn-o19c.json`(O19 C のまま)。各 run の load/free/vmstat は out/steps.log と各 run の gate.txt に記録。swap in/out は開始条件で 0/0、MemAvailable は 20–26 GB、他 Chrome 0、Wine 0。mt3 cargo/wasm-opt は開始前後 0。run script `out/run-o19c-rte.sh` で実行。
- リスト表は `tools/o19c-table.py` を 4 run に対して実行した `o19c-table.md`。数値は play 後 window の値のみ。

## 結果 (post-play window)

| run | windows underrun>0 / windows | underrun quanta | lag cuts | engine gap frames | present/s (mean of 10s windows) | silent runs >=100 ms (A) / longest | Worker CPU (cores) / N | load start/end |
|---|---|---|---|---|---|---|---|---|
| E13 shown | 0/30 | 0 | 1611 | 0 | 22.2 | 2 / 75 posts | 0.80 / 1 | 1.01/3.49 |
| RT-E shown | 0/30 | 0 | 0 | 0 | 36.2 | 2 / 74 | 1.76 / 8 | 2.95/4.33 |
| E13 allhidden | 0/30 | 0 | 0 | 0 | 20.1 | 2 / 74 | 0.60 / 1 | 2.93/3.54 |
| RT-E allhidden | 1/30 | 6 | 0 | 682 | 46.8 | 2 / 74 | 1.55 / 8 | 3.00/5.80 |

10 秒窓の内訳は `o19c-table.md`。RT-E allhidden の 1 window は 70-80 の 6 quanta だけ。E13 shown の lag cuts は全窓に分散ている（合計 1611）。

## 前回 C との比較

- 前回 C: E13 shown だけが underrun 413 quanta（230-240:4, 240-250:139, 250-260:174, 260-270:96）、lag cuts 4287、engine gap 47962、present/s 14.4。RT-A shown/allhidden は 0/0/0。A の無音連続 >=100 ms は 4 run 全部で 2 回（最長 74–75 posts）。
- 今回: E13 shown 413 は再現せず（0）。RT-E shown も 0（lag 0, gap 0）。RT-E の hidden は allhidden だけ 1 window に 6 quanta / gap 682 の小さい非ゼロだが、他 3 条件は 0。
- silent runs >=100 ms は今回も 4 run 全部で 2 回（最長 74–75 posts）。前回と同じで、song 後半の 2 箇所の長無音が構造的なまま残っている。
- present/s: RT-E shown 36.2 / allhidden 46.8。E13 shown 22.2 / allhidden 20.1。協調より低下はない。

## lift の忠実性との関連

- run script と分析はページ/ランタイム側の窓挙動の上書きはしていない（描画は FL 側、present は presenter の無変換 putImageData の件数）。今回の非ゼロは RT-E allhidden の 1 window だけで、描画条件に依存し得る。報告は測った値のみで、目撃によらない推測は入れていない。

## 保存先

- `lift/out-ci1/kit/out/o19-songfull-shown-1-e13/`、`o19-songfull-shown-1-rte/`、`o19-songfull-allhidden-1-e13/`、`o19-songfull-allhidden-1-rte/`。
- 4 run のコピー、`o19c-table.md`、`REPORT.md`、scenario/tools は `lift/out-git/ci1-results/0339/`。サーバは 5121 (rte, page-rt1) を維持。
