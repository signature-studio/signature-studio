# 決定 0260 (ci1): File メニュー ×100 の取りこぼしの調査

- `REPORT-0260-1-to-PM.md` 入力時刻の付け方（コードの読み）と記録器、`-2-` 実測（Escape がエンジンに届かない）、`-3-` 層の特定（フォーカス移動）・ページ修正・ランタイム修正案 v2・数。
- `ci1-page-focus.diff`: app/src/input/pointer.ts + 単体テスト（旗なし）。`ci1-input-v2.diff`: lift/runtime/fle_input.c の再入ガード v2 + C テスト + cargo テスト（保管）。`ci1-input.diff`: v1（非局所脱出で入力が止まる。使わない）。
- `harness/`: C テストの出力（直す前 HEAD / 直した後 v1 / v2、case 3 の v1 FAIL）。
- `rec/`: 取りこぼし 3 件と前後周期の記録器の行。`ci1-recorder-NOT-FOR-PRODUCT-*.diff`: テスト専用の記録器（`#define FLE_TEST_RECORDER` を足した複製でのみ有効。製品の木に当てない。fle_input の分は v1 の修正を含む）。
- `tools/`: 周期ツール（`sdrive-a1-tools5.mjs` = menuCycles に hold・REC 収集・Escape 前後の probe・フォーカス記録）、`hold-runs.sh`、`fix-runs.sh`、`final-stats.py`、`focus-stats.py`、`pair-stats.py`、`rec-analyze.py`、`reenter.sh`、`scn-*.json`。`tools-all/` は c5 の全ツール。
- 生の report.json（周期ごとの記録）は大きいので置かない（再実行は tools/ と c9 の手順）。
