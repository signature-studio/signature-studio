# 0361: 右クリック(WM_RBUTTONUP → WM_CONTEXTMENU)と仮想ポインタ(Tempo のドラッグ)を取り込む(0319 の O20 の差分のうち virtual-pointer のみ)

分類: (b)。契約の実装。DefWindowProc が WM_RBUTTONUP・WM_NCRBUTTONUP から WM_CONTEXTMENU を生成する、という Win32 の契約を fle_input.c に実装する。SetCursorPos の後の入力が、動かした位置からの相対で届く、という契約を fle_win.c の仮想ポインタに実装する。新しい処理ではなく、契約の不足を埋める物。(a) の除去は含まない。
撤回条件: 取り込み後に、P1 の起動・`cargo test` の新しい失敗・P3 の音の書き込みのいずれかが悪化したら、`patches/alpha22/impl/0319-o20-virtual-pointer-contextmenu.diff` と `-tests.diff` の 2 本の適用(lift/ の commit)を戻す。

日付: 2026-11-14   ROADMAP: R2-3   一覧: -   コミット: `evidence/R2-3.md` の「commit と push」節

## 決定
決定: `patches/alpha22/impl/0319-o20-virtual-pointer-contextmenu.diff` と `patches/alpha22/impl/0319-o20-virtual-pointer-contextmenu-tests.diff` を取り込む(`lift/runtime/fle_win.c`・`fle_input.c`・`fle_input.h`、テスト `lift/crates/fe-lift/tests/wall_fr11.rs`、`sweep1.rs`・`harness/sweep1/sweep1_cases.c`。行数は `evidence/R2-3.md` の `git diff --stat`)。`0319-o20-delta-pointer` と `0319-o20-delta-rte-to-rtf` は当てない(下の「取り込まなかった差分」)。根拠: 下の契約の読みの要点(分類 (b))。

## 契約の読みの要点
パッチの注釈と旧記録の読みによる。Learn の原文は本記録では再確認していない(未確認)。
- WM_CONTEXTMENU: 注釈の引用「DefWindowProc generates the WM_CONTEXTMENU message when it processes the WM_RBUTTONUP or WM_NCRBUTTONUP message」。wParam は右ボタンが離された窓(子窓のことがある。その場合メッセージは親へ送られるが、wParam は子のまま)。lParam は WM_RBUTTONUP ではクライアント座標なので窓の原点を足して画面座標にし、WM_NCRBUTTONUP では既に画面座標。戻り値は 0。
- 仮想ポインタ: SetCursorPos・SetPhysicalCursorPos は点を格納し、ホストの最新の絶対位置をその点に再アンカーして offset を 0 にする。以後のホストの絶対移動は、新しい位置からの相対で届く: ゲストの位置 = clamp(ホストの位置 + offset)。画面の大きさが変わったときは、現在のゲストの位置から offset を計算し直す(`fle_win_pointer_reclamp`)。
- 旧記録: `docs/archive/decisions/0319-owner-observations-contract-reading.md` の 26.5.1(WM_CONTEXTMENU の wParam の修正、SetCursorPos の再アンカーの修正案)と §27(O20 の delta の節)が、この差分の内容を書く。どちらも静的な読みで、実エンジンでの確認ではない。

## 出典
- `docs/archive/feedback/2026-11-07-owner-observations.md` の O20 の節(127〜136 行)。O20-1(Tempo・Song Position などで右クリックが出ない)と、O20-2(Tempo のドラッグで 522 に張り付く)。いずれも依頼者の観測。旧記録は原因を未特定と書く。
- `docs/archive/decisions/0319-owner-observations-contract-reading.md` の 26.5.1 と §27。

## 追加の処理・分岐の分類
- `set_pointer` の clamp の式を `clamp_x`・`clamp_y` に括り出し、`host_ptr_x/y`・`virt_off_x/y`(static 4 つ)を足す。分類 (b)。clamp の式そのものは既存の物と同じ。
- `fle_win_host_pointer_set(x, y)`: ホストの移動の記録。ゲストの位置 = clamp(ホストの位置 + offset)。分類 (b)。
- `fle_win_pointer_reclamp()`: 画面の大きさが変わったときの再計算。分類 (b)。`fle_input.c` の display change の呼び出し 2 行(旧: `fle_win_pointer_get` → `fle_win_pointer_set`)を置き換える。
- `fle_input.c` の `WM_RBUTTONUP`・`WM_NCRBUTTONUP` の case: 既定処理で WM_CONTEXTMENU を送る。分類 (b)。
- `fle_input.h` の `#define` 3 つ(FLE_WM_CONTEXTMENU 0x007B・FLE_WM_RBUTTONUP 0x0205・FLE_WM_NCRBUTTONUP 0x00A5): 定数。`lift/` の定義は fle_input.h:96〜98 の 1 箇所(HEAD には無い)。
- 環境変数・`#ifdef`・ビルド変数・URL パラメータは増えない(下の検査)。

## パッチの `+` 行の検査
`grep -c -e '^[+].*getenv[(]' -e '^[+]#ifdef FLE_' patches/alpha22/impl/0319-o20-virtual-pointer-contextmenu.diff patches/alpha22/impl/0319-o20-virtual-pointer-contextmenu-tests.diff` → 0 と 0。`+` 行の `#if`・`#ifdef`・`#ifndef`・`#elif`・`getenv`・`URLSearchParams` は 0 件。`+` 行の `FLE_` で始まる名前は、既存の `FLE_GDC_HORZRES`・`FLE_GDC_VERTRES`(`fle_gdc.h:27-28`)と、新規の定数 3 つ(上の分類)だけ。ガードも新しい `#ifdef` も無い。

## fle_input.c の 0x007B の扱い(適用後の grep)
```
lift/runtime/fle_input.c:96:    [0x70] = 0x3B, ... [0x7B] = 0x58,   (仮想キーの表の値。WM_CONTEXTMENU ではない)
lift/runtime/fle_input.c:975:        /* Learn WM_CONTEXTMENU: "DefWindowProc generates the WM_CONTEXTMENU message when it processes the
lift/runtime/fle_input.c:992:        (void)fle_win_input_send(api, target, FLE_WM_CONTEXTMENU, hwnd, lparam_xy(sx, sy));
lift/runtime/fle_input.c:1002:  ... || msg == 0x007Bu || ...   (HEAD の 980 行の既存の行。パッチは変えていない)
lift/runtime/fle_input.h:96:#define FLE_WM_CONTEXTMENU    0x007Bu
```
扱い: 1002 行は既存の「既定処理の結果 0 で返す」集合の 1 つ。パッチが新たに送るのは 992 行の 1 箇所。`fle_win.c` には 0x007B の参照は無い。

## weak 宣言・NULL の扱い
- パッチの `+` 行に `__attribute__`・`weak`・`NULL` は 0 件(`git diff lift` で確認)。weak 宣言は足さない。
- `fle_input.c` が新しく呼ぶ `fle_win_*` は 2 つ: `fle_win_host_pointer_set`・`fle_win_pointer_reclamp`。定義は `lift/runtime/fle_win.c:5922`・`:5929`、宣言は `fle_input.h:92-93`。
- 製品の link への入り方: runtime の `fle_*.c` は一括でコンパイルされる(`lift/harness/wasm/build.sh` のコメント、`lift/harness/vst3a/build_vst3a_wasm.sh:37`)。製品の `link-web.sh:28` と、HEAD の製品ビルドでのリンクは未確認。

## テストの追従(パッチの結果への追従)
- `lift/crates/fe-lift/tests/sweep1.rs:160` と `lift/harness/sweep1/sweep1_cases.c:34` に、新しい 2 つの関数(`fle_win_host_pointer_set`・`fle_win_pointer_reclamp`)の空のスタブが入る。出典: テスト側のパッチ `0319-o20-virtual-pointer-contextmenu-tests.diff`。新しい処理ではない。`sweep1_cases.c` の hunk は dry-run で fuzz 2 で当たった(内容は一致)。
- 新しい `cargo test` の失敗は 0(`evidence/R2-3.md`)。

## 取り込まなかった差分
- `patches/alpha22/impl/0319-o20-delta-pointer.diff` と `-tests.diff`: ROADMAP R2-3 の指示により当てない。ROADMAP は「README が『置換する』と書き、組合せ未検証」と書く。この README の文言は本記録では確認できなかった(`patches/alpha22/impl/` に README は無く、`find patches -iname 'README*'` で alpha22・pointer・o20 に該当が無い)。ROADMAP の根拠は未確認。当てない判断は変わらない。
- `patches/alpha22/impl/0319-o20-delta-rte-to-rtf.diff` と `-tests.diff`: 同じく ROADMAP の指示により当てない。内容の確認は未実施。

## O20 について
O20 は、取り込みが効くかを実エンジンで確かめる別の項目(ROADMAP R2-4)。この取り込みが O20 に効くかは未確認。本記録は O20 の原因を書かない。

## 未実施・未確認
- 実エンジンでの確認は未実施。wasm のビルドも未実施。push 後に GitHub の engine-bundle のビルドが走るが、結果は未確認。
- P1〜P3 は未確認。依頼者機の値は 0 件。
- テストは C の runtime を Rust のテストから呼ぶ native の検査で、実エンジン(FL の窓)での右クリックと Tempo のドラッグは未確認。
- Learn の原文は再確認していない(未確認)。
- `delta-pointer` の根拠の README は未確認(上の節)。
