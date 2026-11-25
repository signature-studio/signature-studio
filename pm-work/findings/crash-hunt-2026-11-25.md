# crash-hunt 2026-11-25 (Issue #1405, #1417)

実エンジン: `tools/crash-hunt/run.sh` で、fe-build の bundle(raw/fl-Oz)の engine を、headless Chrome で起動し、FL の画面の操作(メニュー・キー・右クリック・上部バー・Settings・Browser・Mixer のインサート)を 1 件ずつ踏んだ。runner の値。この機械とは別枠。

## 概要

- probe: 562 件(DPR 別: 1: 213, 1.7: 349)。グループ別: a: 385, c: 18, b: 30, top: 83, plugin: 22, browser: 8, settings: 16。グループは a メニュー・b キー・c 右クリック・top 上部バーと Mixer インサートの右クリック・settings Settings の頁・browser Browser の頁の帯・plugin Mixer インサートの下位のメニュー。
- 結果: ok: 554, stopped: 8。停止(期待される終了を除く): 6 件。期待される終了: 2 件(File の最後の項目 Exit)。
- 予算: 40 分。経過: 16.8 分。完了: はい
- DPR: 1, 1.7。停止の後の起動: 32 回(停止 7、期待される終了 1、窓の残り 21)。起動し直しの失敗: 0。CDP の Runtime.evaluate の再試行: 0。
- menuItemsTotal(DPR 1 の走査): 202。DPR 別: 1 → 202, 1.7 → 258

## 指紋ごとの表(期待される終了を除く)

| 指紋(`stop.lastFle` の先頭 80 字) | 件数 | 結果 | DPR | 代表の経路(3 件) |
|---|---|---|---|---|
| `fle: GetFileVersionInfoSizeW: a file name with no directory part: the LoadLibrar` | 2 | stopped | 1, 1.7 | `File > #9 > #11` |
| `fle: indirect call to # is not a known function entry` | 2 | stopped | 1, 1.7 | `View > #11 > #1` |
| `fle: RegOpenKeyExW: an empty component (leading, trailing or doubled backslash) ` | 1 | stopped | 1 | `Add > #14` |
| `fle: GetFileAttributesW: reserved character in a path component (streams are not` | 1 | stopped | 1.7 | `settings > tab 6 of 16` |

## 期待される終了

- `File > #10` (DPR 1): File の最後の項目 Exit: FL が終了する(正しい終了。停止に数えない)
- `File > #10` (DPR 1.7): File の最後の項目 Exit: FL が終了する(正しい終了。停止に数えない)

## 4 件の指紋の再走査(#1405 の停止)

| 経路 | 結果 |
|---|---|
| `File/9/11` | DPR 1: stopped / exited / `fle: GetFileVersionInfoSizeW: a file name with no directory part: the LoadLibrar`<br>DPR 1.7(再走査): stopped / exited / `fle: GetFileVersionInfoSizeW: a file name with no directory part: the LoadLibrar` |
| `Add/14` | DPR 1: stopped / exited / `fle: RegOpenKeyExW: an empty component (leading, trailing or doubled backslash) `<br>DPR 1.7(再走査): ok / engine-idle |
| `View/11/1` | DPR 1: stopped / exited / `fle: indirect call to # is not a known function entry`<br>DPR 1.7(再走査): stopped / exited / `fle: indirect call to # is not a known function entry` |
| `File/10` | DPR 1: stopped(期待される終了) / exited / `(fle 無し)`<br>DPR 1.7(再走査): stopped(期待される終了) / exited / `(fle 無し)` |

## 残り

- 経路の残り: 0 件(先頭 10 件: なし)
- (d) not run: the Language (7) and Theme (13) switches: not done (the window table has no window per language or theme; the Settings tabs and their sections are walked)
- (c) not run: the Mixer inserts other than the master insert's first slot (the rack is found in the window table; its first slot's menu is walked)
- (f) not run: the built-in plugins one by one (#1371 tool not in the repository): the Mixer insert's submenus are walked (group plugin)
