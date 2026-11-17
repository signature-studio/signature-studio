# ci1 → PM 決定 0260 報告2: File メニュー ×100 の取りこぼしの実測 — 原因は「入力の順序」ではなく、前の周期の Escape がエンジンに届いていないこと

## 先頭の事実（実エンジンで観測。代役・再生・推測なし）
- 記録器入りの実エンジン（c9 = 4f054ecd のビルド + 記録器 2 オブジェクト、wasm sha256 `410db7f1692b…`）を Chrome 156 で動かし、File メニュー ×60 周期のセッションを hold 0 / 50 / 300 ms で各 3 回以上（合計 **540 周期**、負荷 11〜29）。**取りこぼし 3 件**（hold50 の run 3、周期 10・22・53、いずれも負荷 27〜30）を記録器付きで捕まえた。他の 537 周期は取りこぼし 0。
- **3 件とも、直前の周期（9・21・52）にエンジン内の Escape の注入記録（`REC injk`）が 1 行も無い**。他の全周期には Escape の DOWN/UP が必ずある。つまりその周期の Escape は `fle_input_host_key` に届いていない → ポップアップが開いたまま → 次の周期の File クリックはポップアップを**閉じる**操作になる（`TPopupMenu` の通常の切り替え）→ ドライバは「6 s 以内にポップアップが見えない」と数える。周期 9→10 の間が 7.4 s（= Escape 後の閉じ待ち 6 s + 隙間）で、これとも一致。前報告の「窓表にメイン窓だけ・エンジンは生きている・次のクリックは 7/7 で開く・入力キューは落としていない」とも矛盾しない（閉じた後の状態）。
- 取りこぼし周期の DOWN/UP 自体は正常（`inj`→`tr-begin`→`target`→`push`→`get`、DOWN→UP）。UP 変換時の `cap=0`（成功周期は `cap=1`）は「開いているポップアップへのクリック」の結果で、原因ではない。
- 疑わしい点 1・2・4（同じバッチの DOWN/UP、`fle_input_synth` の再入、ダブルクリック化）は実エンジンで**観測されなかった**: 全 ~1,000 回の注入で `evq=0 outq=0`（キューに溜まった状態の注入は 0 件）、`NESTED-synth` 0 件、`push`/`get` の順は全周期 DOWN→UP、`msg=0203`(DBLCLK) 0 件、`mouseactivate` の戻りは File で常に 3、`target` 窓は常に同じ。hold 0 / 50 / 300 ms で `MSG.time` の差（注入と取り出しで同値）: 0 / 51 / 300 ms 前後。
- したがって『約 1%』の取りこぼしは**ランタイムの時刻・順序の欠陥ではない**。Escape を失う層は未特定: (1) テストツールの CDP のキー送出、(2) ページの `input/pointer.ts` の `onKey`（`inputEnabled()` が偽、またはキーが `#desktop` に届かない＝フォーカス）、(3) ドライバ/エンジンの受付。**本番ページ側の欠陥（負荷下でキーを落とす）かツール側の欠陥かは、まだ判定できていない**。次回の周期から Escape の前後で `document.activeElement` / `hasFocus` / `inputEvents` / `driverStats` を記録する（`tools5/sdrive-a1.mjs` に追加済み）。

## (c) hold 時間（実測）
| hold | セッション | 周期 | 取りこぼし | `MSG.time` DOWN→UP 中央値 |
|---|---|---|---|---|
| 0 ms | 3 | 180 | 0 | ~1 ms |
| 50 ms | 3 | 180 | 3（Escape 欠落が原因） | ~51 ms |
| 300 ms | 3 | 180 | 0 | ~300 ms |
（継続中。10 回ずつは 23:30 ごろ。負荷は他の作業者のまま、取りこぼし周期は 27〜30。）

## ci1-input.diff（再入の修正案。ただし上の取りこぼしの原因ではない）
- 内容: `fle_input_synth` が生イベントを変換している最中（`translating`）は、再入した呼び出しが `evq` を消費しない（`outq` に既にある物を返すか 0）。`fle_input_has_event` も同じ間は生イベントを「無し」とする（忙しい待ちを避ける）。`-` 行は 2 行だけ（`if (!ev_n) return 0;` → `if (!ev_n || translating) return 0;`、`if (!ev_n || guard++ > EV_MAX)` → `if (!ev_n || translating || guard++ > EV_MAX)`）。`translate_one` の後は例外経路なしで必ず `translating--`（`translate_mouse` の `return` は `translate_one` に戻る）。新しいオプション・クラス名・番地の分岐なし。
- テスト（C の harness `lift/harness/api2/input_reenter.c` = win_act.c の骨組み、MOUSEACTIVATE のハンドラが PeekMessage で 1 回吸う窓手続き。`build.sh reenter` と `fe-lift/tests/input_reenter.rs`）:
  - **直す前（HEAD）**: `[0] MOUSEACTIVATE` → `[1] LBUTTONUP`（入れ子の中で先に UP）→ `[0] LBUTTONDOWN`、最後に `GetKeyState(VK_LBUTTON)=0x8000`（押下のまま）。checks 4 failed **2**。
  - **直した後**: DOWN → UP とも外側 `[0]`、`GetKeyState` は DOWN で 0x8000・UP の後 0。checks 4 failed **0**。
  - `cargo test -p fe-lift`（HEAD + diff の clean な複製 c10）: input_reenter ok、さらに fle_input.c を使う既存の 10 ファイル（alpha1_glass, alpha3_updregion, alpha5_erase_dc, alpha7_showwin, api2, kbd, sweep1, wall_fr19, win2_act、計 46 テスト）**すべて ok**。fle_input.c を HEAD に戻すと input_reenter は FAIL（負の確認）。win_act / win_seq / win_upd のネイティブ出力は直す前後でバイト一致。
  - dry-run: HEAD の `git archive` に `git apply --check` ok、`git apply` ok、`patch -p1 -R --dry-run` ok。
- **推奨**: モデルとしては正しい修正（Windows の入力順序、GetKeyState の整合）だが、実エンジンの観測では再入が起きておらず（`NESTED-synth` 0 件）、File メニューの取りこぼしを直す物ではない。23:10 のビルドに載せるかは未決（PM の決定が要る。載せても File メニューの挙動は変わらない見込み。ネスト待ちで止まる経路は、観測された実エンジンでは通らない）。

## 次
1. Escape を失う層の特定（上記の前後記録）。ページ側なら本番の欠陥として診断を出す。
2. 修正入りの記録器ビルド（リンク中）は、上の理由で優先度を下げる。必要なら A/B を後で。
3. hold 0/50/300 の 10 回ずつ（続行中）。

## 成果物（`lift/out-git/ci1-results/0260/`）
`ci1-input.diff`、`harness/before-HEAD.txt`・`after-fix.txt`、`rec/misses-hold50-run3.json`（取りこぼし 3 件と前後周期の記録器行）、`tools/`（`rec-analyze.py`、`scn-hold.json`、`hold-runs.sh`、`reenter.sh`、`native-same.sh`、`apply-fix.py`、`sdrive-a1-tools5.mjs`）、`ci1-recorder-NOT-FOR-PRODUCT-fle_input.diff`（記録器 + 修正。製品の木に当てない）、`ci1-recorder-NOT-FOR-PRODUCT-fle_win.diff`。

## 自己申告の逸脱
- 報告1で「同じバッチの DOWN/UP」を負荷で起きる候補として挙げたが、実エンジンでは 0 件だった（読みの推測を実測で否定）。
- 仮説 2（再入）を PM と共に有力として扱い、診断より先に修正案を作った。モデルでは再現するが、実エンジンの原因ではなかった。
- 1 回目のリンクの空費（報告1に記載）。`( … ) &` の形で背景起動した（報告1に記載）。
- 周期テストツールの終了待ち `waitWindow(..., false, 6000)` の結果を無視していた（Escape が効かなくても次の周期に進む）。ツールの欠陥で、取りこぼしの「見え方」を作っていた。ツール側の修正（Escape 失敗の明示）は次回の版に入れる。
