# 0359: comctl32 の ImageList のドラッグ(BeginDrag・DragEnter・DragMove・DragLeave・DragShowNolock・EndDrag・GetDragImage)を、fle_imagelist.c の画素モデルの上で動かす(0319 の 4 本目)

分類: (b)。契約の実装。comctl32 の ImageList が、ドラッグ用の一時の list を作り、DragEnter で窓を LockWindowUpdate して像を窓の上に重ね、DragMove で像の古い位置の画素を戻して新しい位置に描き直す、という契約を fle_imagelist.c に実装する。新しい処理ではない。
撤回条件: 取り込み後に、P1 の起動・`cargo test` の新しい失敗・P3 の音の書き込みのいずれかが悪化したら、この 4 本目(`lift/runtime/fle_imagelist.c`・`fle_imagelist.h`(未追跡の新規ファイル)、`fle_gdi.c` の `fle_gdi_bitmap_view`・`fle_gdi_dc_blend32`・`fle_gdi_dc_put32`、`fle_gdi.h` の宣言 4 行、`ship6-imports.txt` の 24 行、テストの領域登録 2 行)を外す。
日付: 2026-11-14   ROADMAP: R2-2(4 本目)   一覧: -   コミット: 取り込みは `lift:` で始まる commit(下の「コミット」節)。文書と記録は次の commit。

## 決定
決定: `patches/alpha22/impl/0319-imagelist-drag.diff` と `patches/alpha22/impl/0319-imagelist-drag-tests.diff` を取り込む(`lift/runtime/fle_imagelist.c` 439 行・`fle_imagelist.h` 54 行(新規)、`fle_gdi.c` +102、`fle_gdi.h` +4、`ship6-imports.txt` +24、`lift/crates/fe-lift/tests/gdi.rs` +2、`window.rs` +2)。根拠: Win32 の ImageList のドラッグの契約(下の出典)を実装することそのもの。分類 (b)。O9 の原因を直す物ではない(下の「O9 について」)。

契約の読みの要点(パッチの注釈による。Learn の文の引用は注釈が元):
- ImageList_BeginDrag は、一時の list を作り、像の画素を写す。himlTrack・iTrack が不正なとき、または drag が既に始まっているときは FALSE(Wine 10.0 を参照。UNOFFICIAL)。
- ImageList_DragEnter は、hwndLock の窓を LockWindowUpdate し、像を (x − dxHotspot, y − dyHotspot) に表示する。座標は窓の左上基準(Learn: 「not the client area」)。hwndLock が NULL のときはデスクトップ。
- ImageList_DragMove は、像の古い位置の画素を戻し、新しい位置の画素を保存して描く。
- ImageList_DragLeave は、像を消し、DragEnter が掛けた LockWindowUpdate を外す。
- ImageList_DragShowNolock は、既にその状態なら FALSE(Wine 10.0)。窓の更新は止めない。
- 描画は GetDCEx(DCX_WINDOW | DCX_CACHE | DCX_LOCKWINDOWUPDATE = 0x403)の DC で行う。LockWindowUpdate 中でもこの DC は描ける(0358 の契約。Learn GetDCEx の例外)。

出典:
- `docs/archive/decisions/0313-g9a-balance-module.md` の 258〜262 行(ImageList_BeginDrag・DragEnter・DragMove・DragShowNolock の API の引数と Learn の文。FL の VCL の TDragImageList の呼び出し元 `0x57d820`〜`0x57db30` を実測で挙げる)。
- `docs/archive/decisions/0319-owner-observations-contract-reading.md` の 269・299〜301 行(FLEngine が 6 個の API を import し、runtime にこの 6 行が無かったこと(api-coverage.tsv の事実)。実装の対応表)。
- パッチ `0319-imagelist-drag.diff` の 1〜79 行目の注釈(Learn の各ページの引用。Wine 10.0 を基準にした点は UNOFFICIAL と注釈が書く)。Learn の原文は本記録では再確認していない(未確認)。

## 追加の処理・分岐の分類
- `fle_imagelist.c` は、ImageList の 32 の API のうち 7 を実装する(ドラッグに要るもの。他の 7 は名前つきの停止。`il_stop` は「API 名を書いて停止する」だけで、代わりの結果を返さない)。分類 (b)。
- `fle_gdi_bitmap_view`(ビットマップの画素の読み)・`fle_gdi_dc_blend32`(DC の 32 ビットの画素を保存し、半透明で重ねる)・`fle_gdi_dc_put32`(保存した画素を戻す)は、既存の DC・DIB の画素の読み書きの上の関数。新しい状態は増えない。分類 (b)。
- `DRAG_ALPHA 192` は、Wine 10.0 の値(UNOFFICIAL)。Learn の ImageList_DragEnter の頁は透明度を定めない(パッチの注釈)。この値が FL の見た目と一致するかは未確認。
- `FLE_IL_HANDLE_BASE`・`FLE_IL_REGION_SIZE`(ハンドルの領域。0x7FB90000、64 KiB)は、定数。環境変数・ビルド変数・`#ifdef` ではない。領域の重なりは、テストの `window.rs`・`gdi.rs` の 2 行が検査する(手順 6 で通過)。

## パッチの `+` 行の検査
`grep -c -e '^[+].*getenv[(]' -e '^[+]#ifdef FLE_' patches/alpha22/impl/0319-imagelist-drag.diff patches/alpha22/impl/0319-imagelist-drag-tests.diff` → 0 と 0。
`+` 行の `#if`・`#ifdef`・`#ifndef`・`#elif`・`getenv`・`URLSearchParams` は、`fle_imagelist.h` の `#ifndef FLE_IMAGELIST_H` / `#define FLE_IMAGELIST_H`(include ガード。許可の範囲)の 2 行だけ(`grep` で 14・15 行)。`+` 行の `FLE_` で始まる名前は、定数(`FLE_IL_HANDLE_BASE`・`FLE_IL_REGION_SIZE`・`FLE_DESKTOP_HANDLE`(既存。`fle_win.h:262` の定義)・`FLE_GDI_KIND_BITMAP`・`FLE_GDI_BMP_DIB`(既存))と、テストの領域名だけ。新しい `#ifdef` は無いので、分類の追加は無い。

## weak 宣言と NULL の扱い
| 宣言 | 場所 | 呼び出し | NULL の確認 |
|---|---|---|---|
| `fle_imp_LockWindowUpdate` | `fle_imagelist.c:41` | 308 行(DragEnter)・397 行(DragLeave) | 305 行 `if (!fle_imp_LockWindowUpdate) return;`・397 行 `if (fle_imp_LockWindowUpdate) …` |
| `fle_imp_GetDCEx` | `fle_imagelist.c:42` | 284 行(`drag_dc`) | 281 行 `if (!fle_imp_GetDCEx) fle_stop(…)`(停止する) |
| `fle_imp_ReleaseDC` | `fle_imagelist.c:43` | 297 行(`drag_release`) | 294 行 `if (!fle_imp_ReleaseDC) return;` |

- 3 本とも、既存の `fle_win.c:59`・`fle_win.h:352`・`fle_gdc.h:77,100` と同じ形の宣言で、定義は `fle_gdc.c:347`・`376`、`fle_win.c:5728` にある(手元の `cargo test` のビルドで解決された)。
- 製品の link に入ること: `lift/harness/two/build_nmod_wasm.sh` の `stage_rt`(53〜55 行)は `fle_*.c` を全部 `$OBJ/rt/` にコンパイルする(新しい `fle_imagelist.c` も入る)。`lift/harness/web/fruity/link-web.sh` の rt の入れ方は、手元のビルドの map で確かめた範囲と HEAD の製品ビルドは未確認(前回 R2-2e と同じ持ち越し)。
- `fle_gdi_bitmap_view`・`fle_gdi_dc_blend32`・`fle_gdi_dc_put32` は weak でなく、`fle_gdi.c` に定義があるので NULL にならない。

## O9 について
O9 は未特定(`docs/archive/feedback/2026-11-07-owner-observations.md:76`)。この取り込みが O9 に効くかは未確認。前後の画は別の記録で撮る。

## 未実施・未確認
- 実エンジンでの確認は未実施。wasm のビルドも未実施。push 後に GitHub の engine-bundle のビルドが走るが、結果は未確認。
- P2(120 fps)・P3(音切れ)は未確認。依頼者機の値は 0 件。
- ドラッグの絵が実際に FL の画面に出るかは未確認。FL がこの経路(DragEnter・DragMove)を実際に通るかは、archive の 0319 §13 と 0313 §11 の「まだ分からない点」と同じく未確認。
- DRAG_ALPHA 192 の値、Learn の原文の再確認は未確認(上の出典の節)。
- 5 本目 `0319-show-descendants-paint.diff` は未適用(dry-run の結果だけ。`evidence/R2-2f.md`)。
