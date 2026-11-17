# 0360: ShowWindow・SWP_SHOWWINDOW で可視になる窓の、可視な子孫にも全面の更新領域を与える(0319 の 5 本目・最後)

分類: (b)。契約の実装。窓を可視にしたとき、その窓と、可視になった子孫のすべてに、クライアント領域全体の更新領域(WM_PAINT の対象)が生じる、という契約を fle_win.c の表示処理に実装する。新しい処理ではない(新しい状態は増えず、既存の `upd_set_full`・`upd_erase`・`siblings`・`is_visible` を使う)。
撤回条件: 取り込み後に、P1 の起動・`cargo test` の新しい失敗・P3 の音の書き込みのいずれかが悪化したら、この 5 本目(`lift/runtime/fle_win.c` の `upd_show_descendants` の 20 行、`apply_pos` の SWP_SHOWWINDOW の 7 行、`fle_imp_ShowWindow` の 1 行)とテストの 38 行(`lift/harness/alpha3/upd_region.c`)を外す。
日付: 2026-11-14   ROADMAP: R2-2(5 本目)   一覧: -   コミット: 取り込みは `lift:` で始まる commit(下の「コミット」節)。文書と記録は次の commit。

## 決定
決定: `patches/alpha22/impl/0319-show-descendants-paint.diff` と `patches/alpha22/impl/0319-show-descendants-paint-tests.diff` を取り込む(`lift/runtime/fle_win.c` +28、`lift/harness/alpha3/upd_region.c` +38)。根拠: Win32 の表示の契約(下の出典)を実装することそのもの。分類 (b)。O9 の原因を直す物ではない(下の「O9 について」)。

契約の読みの要点(パッチの注釈による。Learn の文の引用は注釈が元):
- 窓を可視にすると、その窓のクライアント領域全体が描画の対象になる(Learn BeginPaint: 更新領域は「sizing, moving, creating, scrolling, or any other operation that affects the client area」の後に system が設定する)。窓の種類・クラスによる例外は書かれていない。
- 表示される窓の可視な子孫(WS_VISIBLE・非 iconic・大きさあり)も、親の WS_CLIPCHILDREN の有無に依らず、クライアント領域全体の更新領域を持つ。注釈は Wine 10.0 の `server/window.c` の `set_window_pos`(露出した領域を `redraw_window(…, RDW_ALLCHILDREN)` に渡す)を引く。Wine 10.0 は UNOFFICIAL で、本記録では再確認していない(未確認)。
- 不可視の窓の子で WS_VISIBLE が立っている物は、更新領域を持たない(不可視の窓は更新領域を持たない)。親が表示されたとき、その子孫は初めて描画の対象になる。
- `SetWindowPos(SWP_SHOWWINDOW)` で、呼ぶ前に不可視だった窓を可視にした場合も、`ShowWindow` と同じ扱いにする(`SWP_NOREDRAW` が無いとき)。

出典:
- `docs/archive/decisions/0319-owner-observations-contract-reading.md` の 23 節(580 行〜)。23.3 の 1〜3 項が、この差分の 3 か所の内容(`upd_show_descendants`・`apply_pos` の SWP_SHOWWINDOW・`ShowWindow` の子孫への伝播)を書く。23 節は、この直しを RT-B のログ(Balance・Settings のスクロール)との対応で書いた節。
- `docs/archive/feedback/2026-11-07-owner-observations.md:143`(旧記録)。窓 174〜177 は WS_VISIBLE なしで作られ、ShowWindow なしに可視になり WM_PAINT が届かない、という alpha1 の読みを書く。同じ行は、この差分を「O9 は直さない(別の穴)」と書く。
- パッチ `0319-show-descendants-paint.diff` の 1〜11 行目の注釈(Learn の BeginPaint の引用と Wine 10.0 の参照)。Learn の原文は本記録では再確認していない(未確認)。

## 追加の処理・分岐の分類
- `upd_show_descendants(i)` は、窓 i の可視な子孫を再帰で走査し、それぞれに `upd_set_full` と `upd_erase = 1` を与える(分類 (b))。走査は既存の `siblings(i, idx, SIB_MAX)`(`SIB_MAX` は 4096 の既存の定数)を使う。新しい状態は増えない。
- `apply_pos` の SWP_SHOWWINDOW の 7 行は、不可視だった窓が可視になったときだけ、その窓と子孫に更新領域を与える。条件は `vis0 == 0`・`!(nflags & FLE_SWP_NOREDRAW)`・`is_visible(i)`・大きさが正(既存の関数・定数)。分類 (b)。
- `fle_imp_ShowWindow` の 1 行は、既存の「表示時に全面を与える」処理の後に `upd_show_descendants(i)` を足す(分類 (b))。
- 定数・環境変数・ビルド変数・`#ifdef` は新しく増えない。

## パッチの `+` 行の検査
`grep -c -e '^[+].*getenv[(]' -e '^[+]#ifdef FLE_' patches/alpha22/impl/0319-show-descendants-paint.diff patches/alpha22/impl/0319-show-descendants-paint-tests.diff` → 0 と 0。
`+` 行の `#if`・`#ifdef`・`#ifndef`・`#elif`・`getenv`・`URLSearchParams` は 0 件。`+` 行の `FLE_` で始まる名前は、既存の定数 `FLE_WS_VISIBLE`(`fle_win.h:214`)と `FLE_SWP_SHOWWINDOW`(`fle_win.h:228`)だけ。新しい定数も無い。分類の追加は無い。

## weak 宣言と NULL の扱い
5 本目は weak 宣言を足さない(`+` 行に `__attribute__`・`weak` は 0 件)。呼び出し側の NULL の確認も要らない(新しく呼ぶ外部関数は無い。`upd_show_descendants` は同じ翻訳単位の static 関数)。
`cargo test` の新しい失敗は無かったので、リンクの追従の宣言は要らなかった(`evidence/R2-2g.md`)。

## O9 について
O9 は未特定(`docs/archive/feedback/2026-11-07-owner-observations.md:76`)。旧記録 `:143` は、この差分を「O9 は直さない(別の穴)」と書く。この取り込みが O9 に効くかは未確認。前後の画は別の記録で撮る。

## 未実施・未確認
- 実エンジンでの確認は未実施。wasm のビルドも未実施。push 後に GitHub の engine-bundle のビルドが走るが、結果は未確認。
- P2(120 fps)・P3(音切れ)は未確認。依頼者機の値は 0 件。
- 子孫の WM_PAINT が FL の画面の O9 に効くかは未確認(上の O9 の節)。
- Wine 10.0 の参照・Learn の原文の再確認は未確認(上の出典の節)。
- 取り込みの前後の native の面 R の結果は archive の 23.4 節(検証)の記録であり、本記録では再実行していない(未確認)。全 215 本の結果は 23.6 節。
- `cargo test` の `alpha3_updregion` の通過は、`upd_region.c` の追加の検査を含む(`lift/crates/fe-lift/tests/alpha3_updregion.rs` が C 側を compile して FAIL 行が無いことを確かめる)。
- R2-2 の 5 本はすべて取り込んだ。ROADMAP の R2-2 の状態は `done` にしない(reviewer の検査と、実エンジンでの確認の後)。
