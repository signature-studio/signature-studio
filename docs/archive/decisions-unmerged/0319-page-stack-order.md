# 0319 ページ側の diff の積み順(alpha1、PM の依頼 08:5x。固定)

**実際に実行した物**: 新しい `git archive HEAD`(d71efab5、`app/` のみ)に、次の 4 本を**この順に** `patch --batch -p1` で実適用し、tsc 4 設定と vitest 全体を実行した(Node 24、nice 19)。wasm・Chrome・実エンジンは実行していない。

| 順 | diff | 場所 | 内容 |
|---|---|---|---|
| 1 | `0319-page-wall-clock-env.diff` | alpha22-results/impl/ | FLE_WALL_FILETIME・FLE_TZ_BIAS_MIN・FLE_ENV_DEFAULTS(wall-clock.ts + main.ts) |
| 2 | `0319-page-num-cpu.diff` | alpha22-results/impl/ | **FLE_NUM_CPU**(num-cpu.ts + テスト 12 件 + main.ts の 3 hunk)。**1 の後ろに載る形に作り直した**(内容は同じ。文脈だけ。main.ts の import 行は hz.ts の行ではなく display.ts の行の直後、関数は applyHzParam の前、呼び出しは applyHzParam の後・applyScreenParam の前) |
| 3 | `0318-hidpi-page.diff` | gpu8-results/diffs/(gpu1 の物) | HiDPI(main.ts・screen-fit.ts) |
| 4 | `0318-hidpi-page-test.diff` | gpu8-results/diffs/(gpu1 の物) | 同上のテスト |

## 検証の数字(実測)
- `patch --batch -p1` の 4 本: 全て rc 0、**FAILED 0・`.rej` 0**。1 と 2 は offset 無し・fuzz 無し(2 は 1 を適用した木に対して作り直した)。3 は 9 hunk が offset 1〜2 行だけで適用(fuzz 無し)。`.orig` が 1 つ(`app/src/main.ts.orig`、3 の offset 適用の副産物。内容に影響なし)。
- `tsc --noEmit -p` の 4 設定(tsconfig.json、tsconfig.webworker.json、tsconfig.test.json、worker/tsconfig.json): **全て rc 0、エラー 0**。
- vitest 全体: lift が HEAD のままだと **2 failed**(`screen-fit.test.ts` の『the limits / the DPI range are the ones of lift/runtime/fle_screen.h』。gpu1 のテストは `lift/runtime/fle_screen.h` を読み、`FLE_SCREEN_DPI` の定義と DPI の範囲は `0318-hidpi-runtime.diff`〔runtime 側〕が足す物。ページ側だけを積んだ木では当然落ちる)。**gpu1 の runtime 側の diff を `lift/runtime` の複製に当てると、70 files passed・1339 tests passed・0 failed**(この確認では runtime diff の `lift/harness/hyb/c-imports-engine.txt` の hunk は複製に含めなかったので当たらなかった。テストが読む `fle_screen.h` の hunk は当たった)。1 と 2 だけ(3・4 無し)だと 70 files passed・1334 passed・0 failed。
- 1 と 2 の入れ替え(2 → 1)も当たる(2 を先にすると 1 の main.ts の hunk が offset と fuzz 2 で当たる。積み順は 1 → 2 に固定する)。
- 2 を HEAD に単独で当てても rc 0・.rej 0(offset −1〜−8 行、呼び出しの hunk だけ fuzz 2)。1 → 2 の順なら fuzz も無い。

## 注意
- `FLE_NUM_CPU` は『要求した bundle だけ』に入る: manifest の `engine.env` に `"FLE_NUM_CPU": "auto"` を書いた bundle(RT-A 側が書く)か、URL の `fle_cpu=auto`。何も無い bundle(今の協調版)には何も足さない。`fle_cpu=<n>`(1〜32)で固定、`fle_cpu=off` で外す。n = clamp(navigator.hardwareConcurrency − 1, 2, 8)(PM の判断 Y。runtime 側の `0319-num-cpu.diff` は RT-A の木〔mt3 の tree-rt1〕に当てる)。
- gpu1 の hidpi を 2 の後ろに載せるための rebase 版は不要だった(そのまま当たる)ので作っていない。
