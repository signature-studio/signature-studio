# crash-hunt 2026-11-25 (Issue #1405)

実エンジン: `tools/crash-hunt/run.sh` で、fe-build の bundle(raw/fl-Oz)の engine を、headless Chrome で起動し、FL の画面の操作(メニュー・キー・右クリック)を 1 件ずつ踏んだ。runner の値。この機械とは別枠。

## 概要

- probe: 234 件(DPR 別: 1: 213, 1.7: 21。グループ別: a: 210, c: 9, b: 15。グループは a メニュー・b キー・c 右クリック)
- 結果: ok: 230, stopped: 4
- 予算: 50 分。経過: 50.1 分。完了: いいえ(remaining を見る)
- DPR: 1, 1.7。停止の後の起動: 22 回(停止 4、窓の残り 17)。起動し直しの失敗: 0。CDP の Runtime.evaluate の再試行: 0。
- menuItemsTotal(DPR 1 の走査): 202。DPR 別: 1 → 202, 1.7 → 202

## 指紋ごとの表

| 指紋(`stop.lastFle` の先頭 80 字) | 件数 | 結果 | DPR | 代表の経路(3 件) |
|---|---|---|---|---|
| `fle: GetFileVersionInfoSizeW: a file name with no directory part: the LoadLibrar` | 1 | stopped | 1 | `File > #9 > #11` |
| `fle: RegOpenKeyExW: an empty component (leading, trailing or doubled backslash) ` | 1 | stopped | 1 | `Add > #14` |
| `fle: indirect call to # is not a known function entry` | 1 | stopped | 1 | `View > #11 > #1` |
| `(fle 無し)` | 1 | stopped | 1 | `File > #10` |

## 残り

- 経路の残り: 169 件(先頭 10 件: `dpr 1.7: File > #9 > #7`, `dpr 1.7: File > #9 > #8`, `dpr 1.7: File > #9 > #9`, `dpr 1.7: File > #9 > #10`, `dpr 1.7: File > #9 > #11`, `dpr 1.7: File > #9 > #12`, `dpr 1.7: File > #9 > #13`, `dpr 1.7: Edit > #1`, `dpr 1.7: Edit > #2`, `dpr 1.7: Add > #1`)
- (d) not run: F10 Settings: the tabs and their choices, the Language and Theme switches (not implemented; F10 itself is a key probe)
- (e) not run: Browser tabs (Sounds, Plugin database, Packs ...)
- (f) not run: plugins: the Effects (112) and the Generators (70) of inputs/plugin-classes.tsv (not implemented: #1371 tool not in the repository)
- (c) not run: top bar parts (Tempo, Song Position, Pattern Selector) and the Mixer inserts
