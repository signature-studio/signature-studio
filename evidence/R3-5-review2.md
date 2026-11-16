reviewer の再検査の報告(2026-11-16、対象 9e0f3a5・9924f9d・d935286)。判定: 差し戻し(4 件)。要点の保存(原文ではない)

# R3-5: reviewer の再検査の要点と、この報告への対応

## 1 受理した点

- 停止条件: `fle_desk.c:394` の `if (ncur == FLE_DESK_MAX_WINDOWS) fle_stop(...)`。`cur` への格納は 395 行で、その直前。非表示の窓は 340 行の continue で数えない。
- 窓の一覧の大きさ: `DESK_LIST_MAX` 4096u(`fle_desk.c:324`)と `SIB_MAX` 4096u(`fle_win.c:464`)が一致。`fle_win.c:7118` の `siblings(-1, idx, SIB_MAX)` の返りは 4096 が上限。n が 4096 以上なら `fle_desk.c:332` が名前つきで停止する。範囲外の書き込みは無い。
- メモリ: static で 4096 × 8 = 32768 B(0392 の実測 5 と一致)。
- テスト (a)〜(e)(`desk1_compose.rs:403–416`。compose と scene の両方)。
- `need_full` は 3ded666 のまま(hunk は `@@ -320` と `@@ -389` の 2 つだけ)。0385 は追記のみ。
- 0392 の先頭 2・3 行に分類と撤回条件がある。分類の「名前つき停止 2 本」は `fle_desk.c:332` と `:394` と一致。
- M2・M3・M4・M5・M6 の直し。
- cargo の足し算(219 行・通過 2428・失敗 0・無視 1)。
- 範囲(9e0f3a5 は 3 ファイル、5 箇所の 64 は不変、ROADMAP は不変、新しい旗・環境変数・`#ifdef` なし)。語の検索。

## 2 差し戻し

1. 0392 の 22 行目の「例外: msgonly の窓が一覧に混じる」は事実に反する。`fle_win.c:391` の `in_group` が親 -1 の群から msgonly を除き、`siblings(-1)` は `in_group` で選ぶので、一覧に msgonly は入らない。0392 の 30 行目の未確認 3 項も同じ理由で不要。
2. `evidence/R3-5.md` §7a(109〜117 行)の出典が無い。`r35/cargo-full.log` と `r35/hang-record.txt` は `/tmp/opencode/r35/` に無い。12:22 JST・5:08・0:00・0.14〜0.25 の数字は確かめられない(reviewer の前回の指摘も直されていない)。
3. `evidence/R3-5-cargo.txt` の 24 行「`wall_wrp2` を単独で追加に 2 回実行」の出力が無い。「下の 3 の読み」の参照先が無い。
4. `evidence/R3-5-review.md` の 12 行「8 件中 7 件は visible 0」は誤り。表の行は 7 件で、visible 0 は 6 件(`engine-log-run2.txt:290〜296`)。

任意: `desk1_compose.rs` の (a) に「64 描画 + 非表示 4031」(合計 4095)の場面を足す。`SIB_MAX` と `DESK_LIST_MAX` の一致をコードで検査できる形にする(コードで検査できないなら、0392 の撤回条件の文書にだけあることを注記)。

## 3 未確認(reviewer が残したもの)

- 実エンジン(wasm bundle)での 65 窓(ROADMAP R3-5 の完了条件『窓を 65 以上開く場面を作って確認』は、native の stub では満たさない)。
- 実機の窓数の最大(実測は起動直後の 1 時点、8・9 件のみ)。
- wasm32 の構文・リンク。
- 基準の既知の失敗 `getmodulehandleexw_returns_in_every_build` が通過した原因。
- 停止の文面が依頼者の画面に出る経路。
- 旧版・新版の `run-limit.py` の出力の保存。

---

(以下は作成者の注。報告の一部ではない)この報告への対応: 差し戻し 1〜4 を直した(0392 の 22・30 行、`R3-5.md` の §7a・§7c、`R3-5-hang.txt`、`R3-5-cargo.txt` の単独の実行、`R3-5-review.md` の訂正)。任意の「64 描画 + 非表示 4031」の場面を `desk1_compose.rs` に足した。直しの後の reviewer の再検査は未実施。
