# ci1 → PM 決定 0260 報告3b: ページのフォーカス修正（案5）の効果の実測 — 24 セッション 1,440 周期で取りこぼし 0

## 先頭の事実（実ブラウザ Chrome 156 + 実エンジン。代役・再生なし）
- **案5 の腕: 24 セッション・1,440 周期（hold 50 ms、他の作業者の負荷 14〜25）で File メニュー取りこぼし 0 件**。同じ手順の『観測した移動に触れない版』は 46 セッション・2,760 周期で 19 件（0.69%）。割合が同じなら案5 の腕で 9.9 件の期待、0 件の確率は 約 0.005%。案4（フォーカスの行き先が別コントロールの場合だけ戻す）は 13 セッション 780 周期で 1 件（フォーカスが body へ落ちた型。案4 の対象外）→ 案5 はその型も戻す。
- 同じ腕で**ブラウザがフォーカスを `#desktop` から `panel-toggle` へ勝手に移した回数は 26 回**（1,440 周期あたり。約 1/55 周期）。全て `panel-toggle` で、全て 0 ms 後に `#desktop` へ戻され、Escape は全て届いた（無修正では同じ移動が Escape を落とす）。
- 実ブラウザでの決定的な確認（`tools/scn-focus5.json`）: 最後の #desktop へのクリックの 0.4 s 後に (a) `panel-toggle.focus()`、(b) `activeElement.blur()` → 300 ms 後の `activeElement` は、案5 のページで (a) desktop (b) desktop、案4 のページで desktop / BODY、無修正のページで panel-toggle / BODY。
- **引き金は未特定**（止めてよいとの指示に従い『未特定』）。記録した 16 件（時刻付き）: 移動は最後のポインタ上げの 82〜1,535 ms 後（中央値 749 ms）、最後のキーからは 52〜1,441 ms 後で、周期内の決まった位相ではない。直前 1.5 s の DOM 変更（MutationObserver）は `engine-log` / `log-count` / `pointer-info` / `cursor-info` / `engine-metrics` / `audio-status` の**状態表示の定期更新だけ**（`panel-toggle` や `#desktop` の属性・子の変更は 0）で、16/16 に主スレッドの long task（150〜200 ms）があるが、このページは常時 long task があり区別できない。移動先は常に `panel-toggle`（n=26）、`document.hasFocus()` は真、`visibilityState` は visible、スクリプトの `focus()` 呼び出しは `#desktop` へのものだけ。→ ブラウザ（headless Chrome）が主スレッドの混み合いの中で『フォーカスされた要素を失う → 文書内の最初のコントロールを選ぶ』ように見える（推測。未確認）。実機の Firefox / 通常の Chrome で起きるかは**未確認**。

## 修正案 `ci1-page-focus.diff`（更新版 sha256 `e000e626e5bff67b…`、app/src/input/pointer.ts + 単体試験 1 件。旗なし、`-` 行 1）
- `#desktop` の `focusout` で、(1) 行き先が別コントロール → 次のタスクでその要素がまだ active なら `#desktop` に戻す、(2) 行き先なし（body へ落ちた）→ active が body/null なら戻す。ただし直近 1 s に『その行き先（または外側のページ）へのポインタ押下』か Tab があった場合（ユーザーの移動）と、行き先がテキスト入力の場合は戻さない。`#desktop` 自身へのクリックは免責にしない（案3 の欠陥）。
- 確認: HEAD `91cbd9b0` の `git archive` に `git apply --check` ok、適用後 vitest `pointer-dom` 18 件 ok、`npm run typecheck` ok。
- 注意: 案1〜3 は効かなかった（報告3 の表 B・C・D）。案4 は身体の一部（body へ落ちる型）が残った。PM の再検証（typecheck・vitest・worker・e2e）は案5 の版で必要。

## 次
- 新本番 bundle `fl-engine-ci-ff5ee429` の確認（01:00 の第一報）を進行中（Chrome / Firefox の 11 シーン、alpha7、File ×100、Mixer ドラッグ・F9・ピッカーを df0512b0 と交互に）。案5 の腕は停止した（追加が必要なら再開する）。
- 自己申告: 案3 の免責条件の誤り、案1・2 の誤読は報告3 に記載。案5 まで 3 回の作り直しをした（各 1 回分の検証時間を費やした）。
