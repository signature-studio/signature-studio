# 0358: GetDCEx の DCX_LOCKWINDOWUPDATE を、LockWindowUpdate 中の DC の可視領域に反映する(0319 の 3 本目)

分類: (b)。契約の実装。Win32 の GetDCEx で DCX_LOCKWINDOWUPDATE が立つ DC は、LockWindowUpdate が有効でもこの窓への描画を許す(tracking 中の描画に使う)、という契約を fle_win.c の可視領域の問い合わせに実装する。新しい処理ではない。
撤回条件: 取り込み後に、P1 の起動・`cargo test` の新しい失敗・P3 の音の書き込みのいずれかが悪化したら、この 3 本目(`lift/runtime/fle_win.c` の `fle_win_dc_query_ex`・`fle_win.h` の宣言・`fle_gdi.c` の ~~1715 行~~ 1729 行の呼び出しと weak 宣言 1 行)を外す。(HEAD `4f363e2` の値。2 本目の取り込み時の行番号は 3・4 本目でずれた。訂正 reviewer 指摘 2026-11-14)
日付: 2026-11-14   ROADMAP: R2-2(3 本目)   一覧: -   コミット: 取り込みは `lift:` で始まる commit(下の「コミット」節)。文書と記録は次の commit。

## 決定
決定: `patches/alpha22/impl/0319-dcx-lockwindowupdate.diff` を取り込む(`lift/runtime/fle_win.c` +12 −1・`fle_win.h` +2・`fle_gdi.c` +2 −1)。根拠: Win32 の GetDCEx の DCX_LOCKWINDOWUPDATE の契約(Learn GetDCEx の記述。パッチの注釈が引く)を実装することそのもの。分類 (b)。O9 の原因を直す物ではない(下の「O9 について」)。

契約の読みの要点(パッチの注釈による。Learn の文の引用は注釈が元):
- DCX_LOCKWINDOWUPDATE(0x400)の付いた GetDCEx の DC は、LockWindowUpdate が有効でもその窓への描画を許す。
- ロックされた窓(またはその子)の、それ以外の DC は、従来どおり空の可視領域を持つ(LockWindowUpdate のページ。0126)。
- FL は LockWindowUpdate の中で 0x402・0x412(DCX_CACHE [| DCX_CLIPSIBLINGS] | DCX_LOCKWINDOWUPDATE)で GetDCEx を呼ぶ(パッチの注釈が 7 箇所と書く)。

DCX_LOCKWINDOWUPDATE の値(0x400)は、`lift/runtime/fle_gdc.h:85` の注記どおり UNOFFICIAL(windows-sys の docs.rs の値。2026-10-31 の確認)。

出典:
- `docs/archive/decisions/0319-owner-observations-contract-reading.md` の §13(260 行の節。`grep -n 'DCX_LOCKWINDOWUPDATE' docs/archive/decisions/0319-owner-observations-contract-reading.md` で 265 行・266 行に当たる)。この節が、LockWindowUpdate 中の GetDCEx の描画が捨てられること、差分の有無で native harness の結果が変わることを書く。同じ節は `docs/archive/decisions-unmerged/` の版にもある。その表の数字は本記録では再確認していない(未確認)。
- `docs/archive/decisions/0126-pixel-model-and-walls.md`(LockWindowUpdate のページの空の可視領域の元の決定。`grep -rln 'LockWindowUpdate\|DCX_' docs/archive/decisions` の結果に出る)。
- `lift/runtime/fle_win.c` の注釈(0126 の LockWindowUpdate のページの文。0319 の節 13)。

## 追加の処理・分岐の分類
- 新しい `#define FLE_DCX_LOCKWINDOWUPDATE_ 0x00000400u` は、フラグの定数(環境変数・ビルド変数・`#ifdef` ではない)。
- `fle_win_dc_query_ex` は、`fle_win_dc_query` の dcx = 0 の形。既存の問い合わせに、DCX フラグの例外を 1 つ足しただけ(分類 (b))。
- `fle_gdi.c` の ~~1715 行~~ 1729 行(HEAD `4f363e2` の値。訂正 reviewer 指摘 2026-11-14)は、GetDCEx の DC のときだけ `gi.flags` を渡す。0142 の `gi.from_ex`・`gi.flags`(`fle_gdc.h`)が既にあり、新しい状態は増えない。

## パッチの `+` 行の検査
`grep -c -e '^[+].*getenv[(]' -e '^[+]#ifdef FLE_' patches/alpha22/impl/0319-dcx-lockwindowupdate.diff` → 0。`+` 行の `#if`・`#ifdef`・`#ifndef`・`getenv`・`URLSearchParams` は 0 件(`grep -n`)。`+` 行の `FLE_` で始まる名前は `FLE_DCX_LOCKWINDOWUPDATE_`(上の定数)だけ。新しい `#ifdef` は無いので、分類の追加は無い。

## NULL の扱い
- `fle_win_dc_query_ex` は `fle_gdi.c` で weak 宣言(1570 行)。呼び出し(~~1715 行~~ 1729 行。HEAD `4f363e2` の値。同上)は `fle_win_dc_query_ex ? ... : fle_win_dc_query(...)` の形で、NULL のときは既存の `fle_win_dc_query` に戻る。その `fle_win_dc_query` は ~~1714 行~~ 1717 行(HEAD `4f363e2` の値。同上)の `if (!fle_win_dc_query) pixel_stop(...)` で守られている。したがって、リンクされない場合も NULL の関数は呼ばれない(read で確かめた)。
- 本番の wasm にリンクされるかは未確認。`lift/harness/two/build_nmod_wasm.sh` の `stage_rt`(53〜55 行)は `fle_*.c` を全部 `$OBJ/rt/` にコンパイルすることを確かめたが、このファイル(81 行)にリンクの段階は無く、`app/scripts/build-engine-bundle.sh` は `build_nmod_wasm.sh rt harness units` を呼ぶだけ。リンクを行う箇所は未確認。実エンジンでの確認は未実施。

## O9 について
O9 は未特定(`docs/archive/feedback/2026-11-07-owner-observations.md:76`)。この取り込みが O9 に効くかは未確認。前後の画は別の記録で撮る。

## 未実施・未確認
- 実エンジンでの確認は未実施。wasm のビルドも未実施。push 後に GitHub の engine-bundle のビルドが走るが、結果は未確認。
- P2(120 fps)・P3(音切れ)は未確認。依頼者機の値は 0 件。
- DCX_LOCKWINDOWUPDATE の描画が実際にドラッグの絵を出すかは未確認(FL がこの経路を通るかは、archive の §13 も「まだ分からない点」と書く)。
- 4・5 本目は未適用(dry-run だけ。下の表)。
  - 4 本目 `0319-imagelist-drag.diff`: rc 0(`fle_imagelist.h`・`fle_imagelist.c`・`fle_gdi.c`(1673 行 offset +1・2161 行 offset +1)・`fle_gdi.h`・`ship6-imports.txt` が成功)。
  - 5 本目 `0319-show-descendants-paint.diff`: rc 0(`fle_win.c` の 2702・3509 行 offset +51、3926 行 offset +239)。
