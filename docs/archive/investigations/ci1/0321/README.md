# ci1-results/0321（2026-11-07）
- `OWNER-OBS-SCENARIOS.md`: 依頼者の観察 O1〜O15 の検証 scenario と受け入れ基準（再現手順書。native の目標値 ↔ 我々の指標の対応表つき）。
- `FIREFOX-P3-PV7S-PROCEDURE.md`: Firefox 158 の p3（P 単独）対 pv7s（P + V7）×2 の手順書（ABBA の 4 run、判定の事前の規則）。
- ツール（`lift/out-ci1/kit/tools/`、`0277/kit/tools/` に複製）: `sdriveg2-export.mjs`（`--dpr`、op `tClear`・`meterRate`・`wheelSeq` を追加）、`scn-lint.mjs`（scenario の parse だけの検査）、`run-e14-e2e.sh`（E14 の e2e を 1 コマンドで。`--echo` は何も起動しない）。
- **実エンジンの実行は 0**（読みと設計、`node --check`、`bash -n`、`--echo` の dry-run、scenario の parse だけ）。
