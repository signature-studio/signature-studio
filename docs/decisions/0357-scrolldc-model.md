# 0357: ScrollDC を fle_gdi.c の DC モデルの上で動かす(0319 の 2 本目)と、その呼び出しに要る 0315 の weak 宣言 1 行

分類: (b)。契約の実装。Win32 の ScrollDC が、DC の可視領域(クリップ)の中のビットを (dx, dy) だけ動かし、動いた先の外で空いた領域を hrgnUpdate・lprcUpdate で返す(無効化はしない)、という契約を fle_gdi.c の DC モデルの上に実装する。新しい処理ではない。
撤回条件: 取り込み後に、P1 の起動・`cargo test` の新しい失敗・P3 の音の書き込みのいずれかが悪化したら、この 2 本目(`lift/runtime/fle_gdi.c` の `fle_imp_ScrollDC`・`fle_gdi.h` の宣言・`ship6-imports.txt` の 2 行)と、0315 の weak 宣言 1 行を外す。
日付: 2026-11-14   ROADMAP: R2-2(2 本目)   一覧: -   コミット: 取り込みは `lift:` で始まる commit(下の「コミット」節)。文書と記録は次の commit。

## 決定
決定: `patches/alpha22/impl/0319-scrolldc-model.diff` を取り込む(`lift/runtime/fle_gdi.c` +118 行(0319 の分)・`fle_gdi.h` +1 行・`lift/harness/seeds/ship6-imports.txt` +2 行。ワークツリーの `git diff --numstat` は、0319 の `+` 行数と、下の 0315 の weak 宣言 1 行の合計と一致: fle_gdi.c 119 = 118 + 1)。根拠: Win32 の ScrollDC の契約(Learn の ScrollDC のページ。パッチの注釈が `lprcScroll`・`lprcClip`・`hrgnUpdate`・`lprcUpdate` の意味を引く)を実装することそのもの。分類 (b)。O9 の原因を直す物ではない(下の「O9 について」)。

契約の読みの要点(パッチの注釈による。Learn の文の引用は注釈が元): lprcScroll の中で、lprcClip の中に残るビットだけが動く。動いた後に lprcClip の中に残る画素だけが描かれる。hrgnUpdate は ScrollDC が定める空いた領域で、矩形とは限らない。lprcUpdate は空いた領域の外接矩形(クライアント座標)。NULL の lprcScroll・lprcClip はクライアント領域全体。どちらも NULL の hrgnUpdate・lprcUpdate は計算しない。
- 画面 DC の窓は、窓のクリップ(`t.clip`)を kext とする。メモリ DC はビットマップ全体を kext とする。
- 可視でない窓は、動くビットが無い(空いた領域も空)。
- 1 バイトに収まらない画素形式(モノクロ・パレット)は、名前を書いて停止する(未モデル化)。

## 0315 の weak 宣言の節
決定: 0315 の `patches/mt3w/0315/impl/0315-gdi-visrgn-subtract-weak.diff`(`lift/runtime/fle_gdi.c` の weak 宣言 1 行)を、2 本目と一緒に取り込む。分類 (b)。根拠: (i) 差分のコメントが「the scroll of a DC (0319-scrolldc-model) takes the uncovered area with it」と、2 本目のための宣言だと書く。(ii) `lift/runtime/fle_gdi.c:2293` が `fle_visrgn_subtract` を呼ぶ(2 本目の呼び出し)。(iii) 定義は `lift/runtime/fle_visrgn.c:50`、同じ形の weak 宣言が `lift/runtime/fle_win.c:394` にある。(iv) 無いと `cargo test` が通過 2143・失敗 275(`fle_visrgn_subtract` の未定義参照 594 件)。入れると失敗は既知の 1 件(`getmodulehandleexw_returns_in_every_build`)だけ。
追加は宣言 1 行で、新しい処理・環境変数・`#ifdef` を足さない。0315 の他の hunk(`0315-rtb-test-expectations.diff` など)は別の決定に属するので取り込まない。

NULL の扱い(read で確かめた結果): `fle_imp_ScrollDC` は `fle_visrgn_subtract`(2293 行)を呼ぶ前に NULL の確認をしていない。同じ weak 宣言の `fle_visrgn_intersect_rect`・`fle_visrgn_intersect`・`fle_visrgn_bounds` の呼び出し(2286〜2292 行)も確認していない。`fle_win.c` は同じ種類の呼び出しを `upd_rgn_ok()`(2718 行)や `if (!fle_visrgn_set_rect || ...) return NULL`(2954 行・3581 行・4997 行)で守っている。既存の `fle_gdi.c` の 1747・1755・1758 行の呼び出しが、どの確認の内側にあるかは未確認(1738 行の `if` が 1758 行まで覆うかは読んでいない)。
未確認: 本番の engine ビルドが `fle_visrgn.c` をリンクするかは、`app/scripts/build-engine-bundle.sh` の中に一覧が見つからず確認していない。リンクされるなら NULL にならない。リンクされないなら、ScrollDC が呼ばれた時点で NULL の関数を呼ぶ。実エンジンでの確認は未実施。

## O9 について
O9 は未特定(`docs/archive/feedback/2026-11-07-owner-observations.md:76`)。この取り込みが O9 に効くかは未確認。前後の画は別の記録で撮る。

## 検査
- `+` 行の検査: `grep -c -e '^[+].*getenv[(]' -e '^[+]#ifdef FLE_' patches/alpha22/impl/0319-scrolldc-model.diff` → 0。`+` 行の `#if`・`#ifdef`・`#ifndef`・`getenv`・`URLSearchParams` は 0 件(`grep -o`)。`+` 行の `FLE_` で始まる名前は 2 つ: `FLE_GDC_K_SCREEN`(画面 DC の種類)・`FLE_VISRGN_MAX`(領域の矩形の上限)。どちらも定数で、環境変数・ビルド変数ではない(定義の所在は未確認)。新しい `#ifdef` は無いので、分類の追加は無い。

## 未実施・未確認
- 実エンジンでの確認は未実施。wasm のビルドも未実施。push 後に GitHub の engine-bundle のビルドが走るが、結果は未確認。
- O9 の確認は未実施。
- P2(120 fps)・P3(音切れ)は未確認。依頼者機の値は 0 件。
- NULL の確認(上記)は未確認。本番の engine ビルドが `fle_visrgn.c` をリンクするかは未確認。
- パッチの注釈が引く「`docs/archive/decisions/0319-owner-observations-contract-reading.md` の section 6.6」(ScrollDC の Wine 10 の測定)は、同ファイルに見つからなかった。§6.1〜6.5 と §9 がある。§9 の native 検証の表は ScrollWindowEx の行(U1〜K1)で、ScrollDC の行は読んだ範囲に無い。未確認。
- 3〜5 本目は未適用(dry-run だけ。`evidence/R2-2c.md` の 3 節): 3 本目 `0319-dcx-lockwindowupdate.diff` rc 0(offset -55)・4 本目 `0319-imagelist-drag.diff` rc 0・5 本目 `0319-show-descendants-paint.diff` rc 0。

## コミット
取り込み: `lift:` で始まる commit(`lift/runtime/fle_gdi.c`・`lift/runtime/fle_gdi.h`・`lift/harness/seeds/ship6-imports.txt`)。この文書・記録: 次の commit。
