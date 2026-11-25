# 182: 保留中の連続したマウスの移動を 1 件にまとめる(fle_input.c)の測定(#1424)

判定: 捨てる

測定: `node tools/ab/run.mjs p2 <基準> <変更後> 3`(1200 Hz の入力、dpr 1.75、load a)。基準は main の版(f6560609)、変更後は `pm-work/findings/perf-windows-browser/patches/05-input-pending-move-coalesce.patch` の版(待ち行列の最後が移動で、新しい入力も移動の時は、最後の件の座標と時刻を置き換える)。基準と変更後を交互に 3 組。6 回すべて error は null、inputRateOk は true。

値は前回の試行の run のログの値で、この Issue では測り直していない。変更は `lift/` と `app/` に入れていない。

## 中央値(`medians`)

| 指標 | 基準 | 変更後 | 読み |
|---|---|---|---|
| inputDropped | 58 | 78 | 増えた(減らない)。残す条件(変更後が基準より小さい)を満たさない |
| inputToPresentMsP95 (ms) | 84.385 | 66.955 | 変わらない範囲と判断(run 間のばらつきの中) |
| minWindowFps | 14.46 | 15.92 | 変わらない範囲と判断(run 間のばらつきの中) |

## 6 回の run

| 版 | run | inputDropped | inputToPresentMsP95 (ms) | minWindowFps |
|---|---|---|---|---|
| 基準 | 1 | 58 | 84.385 | 14.46 |
| 基準 | 2 | 69 | 62.835 | 14.96 |
| 基準 | 3 | 49 | 106.255 | 14.02 |
| 変更後 | 1 | 79 | 78.5 | 10.99 |
| 変更後 | 2 | 78 | 66.955 | 17.02 |
| 変更後 | 3 | 65 | 47.595 | 15.92 |

inputDropped の範囲は、基準 49〜69、変更後 65〜79。重なっている。

## inputDropped の中身

`tools/p2-frames/measure.mjs` の inputDropped は、次の両方を数える。

- エンジンの待ち行列(512 件)が満杯で捨てた入力(`the input queue is full`・エンジンが拒んだ分)。
- ページ側(`app/src/engine/idle-driver.ts`)が、後の移動に統合した入力(`merged into a later move`)。

この測定は両者を分けていない。ページ側の統合は変更の影響を受けないので、エンジン側の統合で減る分は、この数からは読めない。

## 判定

取り消す(判定: 捨てる)。受け入れの基準「`inputDropped` が 0 なら取り消す」には当たらない(基準は 58)が、残す条件「変更後の `inputDropped` が基準より小さい」を満たさない。中央値は増え、範囲も重なっている。`inputToPresentMsP95` と `minWindowFps` の差は、run 間のばらつきの中で、効果と判断できない。

変更は入れない。`lift/` と `app/` に差分を作らない。
