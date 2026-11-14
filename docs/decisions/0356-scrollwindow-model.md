# 0356: ScrollWindowEx / ScrollWindow を fle_win_scroll(0214 の窓モデル)の上で動かす(0319 の 1 本目)

分類: (b)。契約の実装。Win32 の ScrollWindowEx・ScrollWindow が、可視領域(CLIPSIBLINGS・WS_CLIPCHILDREN)の中のビットを動かし、動いた先の外の領域と更新矩形を返す、という契約を窓モデルの上に実装する。新しい処理ではない。
撤回条件: 取り込み後に、P1 の起動・`cargo test` の新しい失敗・P3 の音の書き込みのいずれかが悪化したら、この 1 本(`lift/runtime/fle_win.c` の `scroll_copy_region` 周辺・`fle_wall_ia2w.c` の `scroll_run`・`fle_imp_ScrollWindowEx`・`fle_imp_ScrollWindow`)を外す。
日付: 2026-11-14   ROADMAP: R2-2(1 本目)   一覧: -   コミット: 取り込みは `git log --oneline -2` の 2 本目(`lift:` で始まる commit)。文書は 1 本目の次の commit。

## 決定
決定: `patches/alpha22/impl/0319-scrollwindow-model.diff` を取り込む。`fle_win.c` / `fle_win.h` に `fle_win_scroll`(窓モデル)を足し、`fle_wall_ia2w.c` の `ScrollWindowEx` と `ScrollWindow` をそれに載せる。~~根拠: O9(「スクロールで画面が崩れ、ホバーした部品だけが直る」)の原因は、ScrollWindowEx が画素を動かす範囲と、返す更新領域が Win32 の契約(可視領域・WS_CLIPCHILDREN・更新領域の外接矩形)と合わなかったこと。契約を実装する(分類 (b))。~~ 訂正(main の検査 2026-11-14): O9 の原因は未特定。旧記録(`docs/archive/feedback/2026-11-07-owner-observations.md:76`・`:143`)は、cand1 の ScrollWindowEx のモデル入りでも E13 の絵とビット単位で同一の崩れで、PM の仮説(子窓の移動で親の空いた領域が無効化されない)は alpha1 の harness で否定されたと書く。この決定の根拠は O9 の原因ではなく、Win32 の契約(Learn の ScrollWindowEx・ScrollWindow の記述。`fle_wall_ia2w.c:161-167` の注釈が引く)を実装することそのもの(分類 (b))。1 本目が O9 を直すかは未確認(R1-6b の前後の画は、1 本目を入れる前の bundle)。

分類の根拠(パッチの注釈と出典):
- `lift/runtime/fle_wall_ia2w.c:161-167` の注釈が、Learn の ScrollWindowEx・ScrollWindow の記述(更新領域は子窓を含む・lpRect が NULL のときだけ子窓と無効領域が動く)を引く。
- `lift/runtime/fle_win.c:3496-3510` の注釈: prcClip が NULL のときはクライアント全体(Learn ScrollDC の lprcClip の記述)。WS_CLIPCHILDREN の親は子のビットを動かさない。更新領域は 1 つの外接矩形(S-updregion)。
- 出典の文書: `docs/archive/decisions/0214-present-path.md`(窓モデルの元の決定。SetWindowPos の SWP_NOCOPYBITS・SWP_NOREDRAW の Learn 引用を含む)。
- Wine 10 を、Learn が黙る点の参照にしている(UNOFFICIAL)。Wine の答えの出典 `docs/0319` の 1.5 節と `lift/harness/scroll_contract.c` は、木に存在しないことを確認した(`find docs -iname '*0319*'` は `docs/archive/` 以下の別の決定だけ。`scroll_contract*` は 0 件)。**この 2 点は未確認**。

## パッチの `+` 行の検査
`grep -c -e '^[+].*getenv[(]' -e '^[+]#ifdef FLE_' patches/alpha22/impl/0319-scrollwindow-model.diff` → 0。`+` 行の `#if`・`#ifdef`・`#ifndef`・`getenv`・`URLSearchParams` は 0 件。`+` 行に現れる `FLE_` で始まる名前は、描画の定数だけ(`FLE_VR_CLIPSIBLINGS`・`FLE_VR_CLIPCHILDREN`・`FLE_WS_CLIPCHILDREN_`・`FLE_WM_MOVE`・`FLE_SWP_NOSIZE`・`FLE_SWP_NOZORDER`・`FLE_SWP_NOACTIVATE`・`FLE_SWP_NOREDRAW`・`FLE_SWP_NOCOPYBITS`)。環境変数・ビルド変数は無い。定数の定義の所在は未確認。新しい `#ifdef` は無いので、分類の追加は無い。

## テストの追従
(a) `lift/crates/fe-lift/tests/wall_ia2.rs` に `fle_win_scroll` のスタブを 2 行足した。
- 理由: `fle_wall_ia2w.c` が `fle_win_scroll` を呼ぶようになった。`wall_ia2.rs` は `fle_win.c` を組み込まないので、スタブが無いと link できない。スタブは「答えられない」(0 を返す)ので、`fle_wall_ia2w.c` の旧い簡単な経路が答え、テストはその経路を確かめる。
- 出典: `patches/alpha22/impl/0319-run-values-and-scroll-model-tests.diff` の同じ hunk(`wall_ia2.rs` の 26〜36 行目)。`+` 行 2 本の本文は一致を確認した。
- 同じ diff の `time.rs`・`vfs.rs` の hunk(実行時の値 `FLE_WALL_FILETIME`・`FLE_TZ_BIAS_MIN`・`FLE_ENV_DEFAULTS` を `getenv` で読むのを許す変更)は、別の決定(run values。`getenv` の分類が要る)に属するので取り込まない。

(a) `lift/crates/fe-lift/tests/dwm_timing.rs` の 1 hunk(`assert_eq!(s6.len(), s5.len() + 1)` を `>=` に、最後の行の文言を「ship5 の後の最初の行」に)。
- 理由: 1 本目が `lift/harness/seeds/ship6-imports.txt` の末尾に ScrollWindow の行を 1 行追記した(追記のみ。decision 0015)。「ship6 = ship5 + 1」の固定が成り立たなくなったので、テストを「ship5 の後の最初の行」の形にする。
- 出典: `patches/mt3w/0315/impl/0315-rtb-test-expectations.diff` の同じ hunk(21〜35 行目)。同じ diff の他の hunk(`wall_dec1.rs` の RM_MAX 8→16、`sweep1.rs`、`wall_fr9t.rs`、`sweep1_cases.c`)は別の決定に属するので取り込まない。

どちらも分類は「新しい処理ではなく、パッチの結果への追従」。テストの期待値を新しい処理に合わせて書き換えたのではない。

## 元のパッチとの違い
なし(`patch -p1` の本適用は rc 0。`.rej`・`.orig` は無し)。取り込んだ 7 ファイル: `lift/crates/fe-lift/tests/wall_ia2.rs`(+2)・`lift/crates/fe-lift/tests/dwm_timing.rs`(hunk 1)・`lift/harness/seeds/ship6-imports.txt`(+2)・`lift/runtime/fle_wall_ia2.h`(+1)・`lift/runtime/fle_wall_ia2w.c`(+81/−1)・`lift/runtime/fle_win.c`(+188)・`lift/runtime/fle_win.h`(+11)。

## 未実施・未確認
- 実エンジンでの確認は未実施。wasm のビルドも未実施。R1-6b が前後の画を撮ってあるが、1 本目を入れた bundle の画は未取得(O9 の確認は未実施)。
- P2(120 fps)・P3(音切れ)は未確認。
- 2〜5 本目は未適用(`docs/decisions/` の別の決定にする)。ship6 の連結を確かめる CI 用のスクリプトは木に無い(その差分の取り込みは見送る)。
