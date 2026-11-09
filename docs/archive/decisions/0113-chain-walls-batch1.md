# 0113: 6 モジュール構成（M1: フォント配置あり）の連鎖の壁 バッチ 1 — HeapReAlloc、freetype の種、LDDQU、CPU 振り分けスタブの添字 −1（決定 0114）、GetObjectW(DIBSECTION)、StretchDIBits、SelectPalette(NULL)、DestroyCursor(共有カーソル)、a4 の最後の読み。すべて仮の決定

状態: **仮の決定（すべての応答と選択）**。依頼は PM（2026-10-28。「壁ごとに直す→再測定」を 1 本の作業の中で連続して回す。番号 0113 は PM の仮の割り当て）。決定と実装は作業者（サブエージェント C1）が行い、書いた。**HEAD 5a39f6b に E1（決定 0112）と、この決定の差分を当てた木で、コンパイル（警告・エラーなし）、全体テスト（115 test result・1,575 passed・0 failed。グループ分けの合算で、1 回の通しではない）、生成 C の関数単位の比較、変異検査 12 件（12 件検出）、6 モジュール構成の a4 の測定（M1 を独立な 2 回のビルドで、M0 を 1 回）まで実施した。実機 Windows との照合は無い。wasm は未確認。**

**測定用の基底・種・註釈（dsp 0x2000000、Blend2D 0x4700000、QFC 0x1d00000、freetype 0x4b00000、ILTools 0x6000000）と、仮の応答を含む構成での値である。** 決定 0101 の表記に従えば「A4 完走（仮の応答 N 件を含む）」の形で報告する値で、達成の判定は PM が行う（「PM の判断が要る点」1）。

区分の凡例: **実測**＝コマンドの出力を見た事実。**読み**＝そこからの解釈。**Learn**＝Microsoft Learn のページに書かれていること（取得日 2026-10-28）。**出典なしの選択**＝ページにも実測にも根拠が無く、作業者が選んだ値。**未確認**。

## 先に結論

| 項目 | 内容 |
|---|---|
| **6 モジュール構成・M1（フォント配置あり）の a4** | **`init_count=942 of 942`**、`last_unit index=940 run_rank=895 of 895 proc=0x13751d0 state=returned`、`runnable_started=895 completed=895 in_progress=0 of 895`、`units_outside_image=0`、`entry_check started_but_not_entered=0 runnable_entered_beyond_init_count=0`、**`all_units_done=yes`**、functions **5,306**、entries **1,007,942**、`== stop: returned from entry`（停止メッセージなし）、**終了コード 0**。独立な 2 回のビルドで同一（native の SHA-256 `9c76bc6a…a23785` も同じ）。変更前の基準: 246/942、functions 3,386、entries 134,274、停止 `HeapReAlloc`、終了コード 134 |
| M0（配置なし） | **246 / 942 のまま**。停止 `fle: StretchDIBits: pixel transfer from a DIB (StretchDIBits rop 0xcc0020, iUsage 0, 1 x 1 to (0, 0) from 1 x 1 at (0, 0)): the bitmap of hdc is a device-dependent bitmap (or the 1-by-1 placeholder): it keeps no pixels (decision 0106) (decision 0111)`、functions 3,411、entries 134,085、depth 32、終了コード 134（s3 の記録と functions／entries／depth が同じ）。**A4 の完走は配置（決定 0110）に依存する**（「PM の判断が要る点」2） |
| 直した壁 | **8**（下の表）+ 測定の道具の欠陥 1（a4 の最後の読みが 0 になる） |
| ループを止めた理由 | `init_count` が 246 を超えて 10 unit 以上進んだ（246 → 942）。壁の数は 8（< 10）、PM の決定が要る壁・設計が大きい壁には当たらなかった（壁 4 は認識器の変更だが局所。決定 0114） |
| 全体テスト | **115 test result・1,575 passed・0 failed**（基準 113・1,526。+49 = E1 の `wall_e1` 24 + `wall_c1` 12 + 他の 13） |
| FLEngine の生成 C（註釈なし、ファイルの SHA-256） | `508b76fb…263b`（HEAD）→ **`f6d3a9715ddb92d734577fc1e87a381142c722aa17562a22a72c565599cbb08c`**（215,773,171 B、functions 59,380、bad_nodes 0、`unsupported_total` 394 → **390**）。本体が変わった関数は **4**（E1 の thunk `f_423420`、`f_423430`、`f_423df0`、`f_423e10`）だけ。**この決定の壁は FLEngine の C を変えない** |

## 背景

- 前回の統合（S3）の 6 モジュール構成の M1 は、`fle: HeapReAlloc: …`（kernel32、freetype の `ft_mem_qrealloc` 経由）で止まっていた（246/942）。依頼は、壁ごとに「原因の特定 → 最小の実装 → 再ビルド → 同じ構成で再測定」を連続して回し、停止点が進むたびに記録すること。
- 方針（PM）: ふり（fail-soft）で進めない。仕様から導ける応答だけを実装し、導けないものは名前付きの停止。値を創作するときは仮の決定として列挙し、Learn の出典か「出典なしの選択」かを区別する。応答表は末尾追記のみ。新しい `.c`／`.h` を runtime に作らない。認識器（fe-decode）を変えるのは最後の手段で、変えるなら別の決定にする。
- 土台: HEAD 5a39f6b（GDI 一括化の後）に、E1 の差分（`lift/out-git/e1-results/e1-env-claims.diff`）を当てた。`imports_table.rs` の末尾の hunk だけ HEAD の GDI の 96 行と衝突したので、E1 の 7 行を GDI の最後の行（`LoadBitmapW`）の後に手で足した。ほかの 7 ファイルは `patch -p1` で衝突なく当たった。
- 測定の手順（RAM `/tmp/opencode/c1/`）: `a4-multi.sh` の複製で 6 モジュール（FLEngine + dsp_ippv2、Blend2D、QFC、freetype、ILTools）、`VFS_SEED=lift/harness/seeds/vfs-seed-fonts.tsv`、`VFS_SEED_ROOT`＝リポジトリのルート。runtime だけが変わる壁は、生成 C を作り直さず、変わったランタイム／ハーネスのオブジェクトだけを作り直して再リンクした（約 6 秒）。生成 C を作り直したのは、種・fe-lift・fe-decode が変わった 3 回（gen 12〜17 秒＋build 38〜64 秒）。

## 壁の表

各壁の記録（停止メッセージ全文、C スタック、根拠のページと文言、出典なしの選択、測定）は `wall-log.md`。

| # | 停止メッセージ（要旨） | 原因（区分） | 修正（場所） | 再測定 |
|---|---|---|---|---|
| 1 | `fle: HeapReAlloc: the requested size of a process-heap block is not recorded by HeapAlloc (not answered)` | 決定 0102 W11 が HeapAlloc に要求サイズの表を作ったのに、HeapReAlloc の本体は決定 0078 の停止のままだった（読み） | `HeapReAlloc` を実装（host の `realloc`、表の要求サイズを移す、`HEAP_ZERO_MEMORY`、`HEAP_REALLOC_IN_PLACE_ONLY`、失敗は NULL で `SetLastError` を呼ばない）。`fle_sys.c`／`.h` | functions 3,465、entries 134,577、depth 39。次: 壁 2 |
| 2 | `fle: indirect call to 0x4b40e10 is not a known function entry` | freetype の TT_Face の関数ポインタ（`access_glyph_frame` ほか）の指す先 `TT_Load_Glyph_Header` が `.pdata` に無い葉関数で、それを指す `lea` が、同じく `.pdata` に無い葉関数 0x4b41500 の中にあった。決定 0102 の種 160 個の作り方（`lea_code.py`）は `.pdata` の関数の中だけを掃く（実測） | 測定用の種を作り直した（新規 `harness/seeds/ft-seeds-4b00000-c1.txt` 170 行 = 旧 160 + 10、作り方は新規 `harness/seeds/lea_leaf_candidates.py`）。**認識器は変えない**。旧ファイルは変えない（テストが 160 行を固定） | functions 3,695、entries 152,333、depth 34。次: 壁 3 |
| 3 | `fle: unsupported instruction at 0x38b76eb` | dsp の `ippsCopy` 系 memcpy の `lddqu xmm0,[rdi]`（SSE3、`F2 0F F0 /r`）。非整列の元の経路（実測。dsp に 975 か所） | `LDDQU`（メモリ形）を lift（`fe-lift/src/sse_pack.rs`。MOVDQU の読み込みと同じ）。CPUID は変えない | functions 3,739、entries 156,601、depth 36。次: 壁 4 |
| 4 | `fle: jump table at 0x3363e4e: selector 0x3363df0 is outside the table` | CPU 振り分けスタブの選択値の変数が初期値 −1 のまま。決定 0079 が「停止にした」添字 −1（スタブ自身の遅延初期化の `call`）の経路 | **決定 0114**: 4 条件がそろったときだけ添字 −1 のスロットを case にする（`fe-decode/src/jumptab.rs`） | functions 3,781、entries 165,636、depth 33。次: 壁 5 |
| 5 | `fle: GetObjectW: handle is a DIB section and c asks for a DIBSECTION: …(decision 0108)` | ILTools の Delphi の `CopyBitmap` 相当が `GetObjectW(hbm, 0x68, &ds)` を呼ぶ（実測）。決定 0108 の PM の判断 7「到達するまで停止のまま」の到達 | `GetObjectW(DIBSECTION)`（c = 104）を実装。dsBmih はゲストが渡したヘッダ（biSizeImage は `|biHeight|×stride`）、dsBitfields、0 の部分は**出典なしの選択**。24 ビット BI_RGB は名前付き停止（`fle_gdi.c`） | functions 3,829、entries 165,939、depth 32。次: 壁 6 |
| 6 | `fle: StretchDIBits: this pixel transfer or read-back is not one of FillRect, PatBlt and BitBlt(SRCCOPY) between DIB sections: …(decision 0111)` | `StretchDIBits(hdc, 0,0,32,32, 0,0,32,32, lpBits, lpbmi, DIB_RGB_COLORS, SRCCOPY)`（実測。ILTools の `TBitmap` 描画）。転送先は DIB セクション | **指示の許す形だけ**: 転送先が DIB セクションで、同形式（24／32 ビット RGB）の 1:1 SRCCOPY（`fle_gdi.c`）。ほかは引数つきの名前付き停止 | functions 3,830、entries 165,953、depth 32。次: 壁 7 |
| 7 | `fle: SelectPalette: hPal is not a live palette of this runtime (NULL, …)` | `SelectPalette(hdc, 0, FALSE)`（StretchDIBits の後の「保存したパレットを戻す」。保存値が 0。戻り値は使わない。実測と読み） | メモリ DC で hPal が NULL のとき NULL を返し何も変えない（**出典なしの選択**）（`fle_gdi.c`） | functions 3,860、entries 495,411、depth 31。次: 壁 8 |
| 8 | `fle: DestroyCursor: hIcon was not issued by this runtime's GDI table (a shared cursor, …)` | Delphi の `TScreen.DeleteCursor` が `DestroyCursor` を呼ぶ（戻り値は使わない）。ハンドルは 2 種: 0x7fca021c（標準カーソル IDC_SIZEALL）と 0x7fda5008（ILTools の RT_GROUP_CURSOR 32763 を `LoadImageW(LR_SHARED)` で読んだもの） | 共有カーソル（標準カーソルと `LoadImageW` の形のハンドル）への `DestroyCursor` は 1 を返して何もしない（**出典なしの選択**）。判定の純関数 `fle_cursor_is_shared_handle` を `fle_cursor.c`／`.h` に追加（`fle_gdi.c` は弱参照） | 1 つ目のハンドルのみ: functions 3,860、entries 617,967、同じ停止（ハンドルが 2 つ目に変わった）。両方: **停止なし**。functions 5,306、entries 1,007,942、終了コード 0 |
| T1 | （壁ではない）a4: `init_count=0 of ?, InitContext+0 is 0 (InitUnits not reached)` | 完走後に FreePascal の `Halt0` がユニットを finalize し、`InitContext+0` と `InitCount` を 0 に戻す。gdb のウォッチポイントで InitCount は 1, 2, …, 942 と連続して増えて 0 に戻ったことを確認（実測。a4 の報告のコードとは独立） | ハーネス: `a4_sample()`（`a4.c`、`a4.h`）を関数入口のフック（`wasm/trace_native.c`、弱参照）から呼び、`InitContext+0` が 0 でないときの最後の表の番地と最大の `InitCount` を覚える。報告時にヘッダが 0 でサンプルがあるときだけそれを使い、先頭に `a4: note: …` を 1 行足す。他の場合の出力は変えない | a4 = 942/942（上） |

## 実装した場所

- 生成器・認識器: `lift/crates/fe-lift/src/sse_pack.rs`（LDDQU）、`lift/crates/fe-decode/src/jumptab.rs`（決定 0114）。応答表（`imports_table.rs`）は E1 の 7 行以外に足していない。
- runtime（新しい `.c`／`.h` は作っていない）: `fle_sys.c`／`.h`（HeapReAlloc）、`fle_gdi.c`（GetObjectW の DIBSECTION、StretchDIBits、SelectPalette、DestroyCursor、`gdi_dib` に 4 つのヘッダ値）、`fle_cursor.c`／`.h`（`fle_cursor_is_shared_handle`）。E1 の `fle_env.c`／`.h`、`fle_vfs.c`／`.h`、`fle_locale.c`／`.h`。
- ハーネス: `a4.c`／`a4.h`／`wasm/trace_native.c`（T1）、`a4-multi.sh`（既定 `EXPECT_FLE_SHA` を `f6d3a971…` に）、`seeds/ft-seeds-4b00000-c1.txt`、`seeds/lea_leaf_candidates.py`（新規。測定の入力）。
- テスト: 新規 `tests/wall_c1.rs`（12 件）、`tests/wall_e1.rs`（E1 の 24 件）。追加・変更は下。
- 既存の関数の本体で変えたもの（指示により理由を記す）: `fle_imp_HeapReAlloc`（壁 1。停止 → 実装）、`fle_imp_StretchDIBits`（壁 6。`GDI_STOP` マクロ → 実装）、`get_bitmap` の DIB の分岐（壁 5）、`fle_imp_SelectPalette`（壁 7。NULL の分岐を足した）、`destroy_icon_or_cursor`（壁 8。分岐を足した）、`resolve_stub`（決定 0114）、`a4_report`（T1）。応答の「値」で既存のものを変えた箇所は、停止だった呼び出しを答えにしたものだけ。

## 出典なしの選択の一覧

| # | 場所 | 選択 | 区分 |
|---|---|---|---|
| S1 | HeapReAlloc | `dwBytes 0` は 1 バイトで再確保（HeapAlloc と同じ）。要求サイズは 0 を記録 | 出典なしの選択（HeapAlloc に合わせた） |
| S2 | HeapReAlloc | `HEAP_REALLOC_IN_PLACE_ONLY`: 同サイズ・縮小は同じ番地で成功、拡大は名前付き停止 | 出典なしの選択（ページは「動かさずにリサイズできないとき失敗」とだけ書く。縮小は動かす必要が無いという読み。拡大の可否はページに無いので停止） |
| S3 | HeapReAlloc | `lpMem` が NULL・表に無い・解放済み・別の heap は名前付き停止 | 設計（HeapSize に合わせた。ページは HeapAlloc／HeapReAlloc の返したポインタだけを挙げる） |
| S4 | freetype の種 | `lea` の指す先で、実行セクション内・`.pdata` の関数の外・16 バイト境界・直前が `int3` のもの | 出典なしの選択（測定の入力。決定 0074 の扱い）。旧 160 行に 10 個足した |
| S5 | GetObjectW(DIBSECTION) | dsBmih はゲストが渡したヘッダ（biHeight は符号つき、biCompression、解像度、biClrUsed、biClrImportant をそのまま）。biSizeImage は `|biHeight|×stride` | 出典なしの選択（Learn は dsBmih を「information about the color format of the DIB」とだけ書く） |
| S6 | GetObjectW(DIBSECTION) | dsBitfields: BI_BITFIELDS はゲストのマスク。BI_RGB 16 ビットは 0x7C00／0x03E0／0x001F（ページの「always RGB 555」から導いた）。**BI_RGB 32 ビットは 0x00FF0000／0x0000FF00／0x000000FF**（ページに無い）。8 ビット以下は 0。24 ビット BI_RGB は名前付き停止 | 出典なしの選択（32 ビットの値）／Learn からの導出（16 ビット） |
| S7 | GetObjectW(DIBSECTION) | dshSection = 0（Learn: hSection が NULL なら NULL）、dsOffset = 0（Learn: 「has no meaning」）、構造体の詰め物 8 バイト = 0 | Learn（dshSection）／出典なしの選択（0 の値、詰め物） |
| S8 | StretchDIBits | 32 ビットの未使用バイトも転送先へコピー（BitBlt の決定 0111 と同じ） | 出典なしの選択 |
| S9 | StretchDIBits | 転送先の矩形が転送先ビットマップから出る場合は停止、戻り値は DestHeight、ボトムアップの元の部分矩形は停止 | 出典なしの選択（保守的。Learn の戻り値は「number of scan lines copied」で、クリップしたときの値は無い。ySrc の基準の読みを避けた） |
| S10 | SelectPalette | メモリ DC で hPal NULL は NULL を返し何も変えない（最終エラーも触らない） | 出典なしの選択（Learn は NULL を失敗値として挙げ、NULL の hPal の扱いを書かない） |
| S11 | DestroyCursor | 共有カーソルへは 1 を返して何もしない | 出典なしの選択（Learn の DestroyCursor は「Do not use this function to destroy a shared cursor. A shared cursor is valid as long as the module from which it was loaded remains in memory」と書き、`LoadImage` の `LR_SHARED` を共有カーソルを得る関数に挙げるが、戻り値を定義しない。「破棄しない」は読み） |
| S12 | 決定 0114 | 添字 −1 のスロットを case にする 4 条件 | 実バイトの確認（434 スタブ） |
| S13 | a4 のサンプル | 関数入口ごとに読み、ヘッダが 0 でない間の最大の InitCount を使う | 測定の道具の設計（ゲストから見えない） |

LDDQU（壁 3）は SDM に従う（出典なしの選択ではない）。**CPUID は SSE3 を名乗らないまま、ゲストは SSE3 の `lddqu`（と決定 0106 の `haddpd`）を実行する**（「PM の判断が要る点」5）。

## テスト

新規・追加（すべて通過）:

- `tests/wall_c1.rs`（新規、12 件）: HeapReAlloc（内容・整列・要求サイズ、`HEAP_ZERO_MEMORY`、0 バイト、表の追従、移動した古い番地、`IN_PLACE_ONLY`、失敗（NULL・元のブロック・最終エラー・`GENERATE_EXCEPTIONS` の停止 2 通り）、停止の 9 ケース、フラグの組み合わせ、第 2 スレッド、応答表の位置の固定（HeapReAlloc の行の前に 257 行 = 0 始まりの添字 257）と noreturn が外れたこと）と、共有カーソルのハンドルの形（`fle_loadimage.h` の定数との一致を含む）。
- `tests/sse_pack.rs` +2（`lddqu_matches_host` 320 ケース、`lddqu_other_forms`）。
- `fe-decode/tests/jt_cpu.rs` +3（決定 0114）。
- `tests/gdi.rs` +3（`get_object_of_a_dib_section_gives_a_dibsection`（DIBSECTION の全フィールドと 8／16／16(BITFIELDS)／1 ビットの形の 2 つの確認を含む）、`get_object_of_a_24_bit_dib_section_asks_for_a_dibsection_stops`、`select_palette_null_is_the_failure_value_and_changes_nothing`）。
- `tests/gdi_bulk.rs` +4（`stretch_dibits_*` 3 件、`destroy_cursor_of_a_shared_standard_cursor_returns_1_and_keeps_it`）。K マクロを 13 引数に拡張（既存の呼び出しは不変）。
- `tests/a4_report.rs` +1。

変えた既存テストの期待値（理由つき。実装の欠陥と思われるものは無い）:

| ファイル | 変更 | 理由 |
|---|---|---|
| `tests/vcall_thunk_dll.rs`（2 か所） | 394 → 390 | E1 の 4 つの thunk が答えになった（実測 390。389 ではない） |
| `tests/sse_legacy.rs` `other_forms_stay_unsupported` | `lddqu xmm0,[rdi]` の 1 件を外した（配列 12 → 11） | その命令を lift した（壁 3） |
| `fe-decode/tests/jt_cpu.rs` `dsp_ippv2_has_434_stubs` | 表の長さの分布 `{6:376,7:57,9:1}` → `{7:376,8:57,10:1}` | 全 434 スタブで添字 −1 の case が 1 つ増えた（決定 0114） |
| `fe-lift/tests/jt_cpu.rs`（実 DLL の 2 件） | 「1 表につき case 数 = スロット数」→ +1、−1 を停止の一覧から外して「native と lifted が S − 16 に着く」の確認へ。−2 を停止の一覧に | 同上 |
| `tests/gdi.rs` `get_object_of_a_dib_section_…_stops_for_a_dibsection` | `dibsection`（c = 104）の停止を外し、`bigger` の語を "above sizeof(DIBSECTION)" に、`one-past`（105）を追加 | 壁 5 |
| `tests/gdi.rs` `select_palette_of_what_the_page_leaves_open_stops_by_name` | メモリ DC の `null-palette` を `null-palette-on-a-screen-dc` に置き換え | 壁 7 |
| `tests/gdi_bulk.rs` `NAMED_STOPS_0111` の StretchDIBits の語 | "pixel transfer or read-back" → "pixel transfer" | ゼロ引数では `rop` 検査の停止になる（StretchBlt と同じ扱い） |
| `tests/gdi_bulk.rs` `the_source_of_decision_0111_pins_where_the_pixel_bytes_are_written` | `GDI_STOP(` の個数 69 → 68、`memcpy(dib_row(` がちょうど 1 つ | 壁 6（画素を書く箇所が 1 つ増えた。その固定） |
| `tests/gdi_bulk.rs` `icon_and_cursor_calls_stop_…` | DestroyCursor の `shared-cursor` を `never-issued-cursor` に | 壁 8 |
| `tests/k32_dsp.rs` | `HeapReAlloc` を `STOPPED` から `ANSWERED` へ（配列 18 → 17、15 → 16） | 壁 1 |

## 検証

### 全体テスト（グループ分け。1 回の通しではない）

| グループ | test result | passed | failed |
|---|--:|--:|--:|
| g01 fe-pe | 10 | 83 | 0 |
| g02 fe-decode（SKIPPED 2 は再配置版 dsp の指定が無いため。指定して `msvc_switch` を再実行し 95 passed・SKIPPED 0） | 15 | 160 | 0 |
| g03 fe-lift の lib／bin | 2 | 14 | 0 |
| g04 fe-lift の doc | 1 | 0 | 0 |
| g05 a2a_remaining … cursor | 11 | 139 | 0 |
| g06 deferred … gs_stacklimit（gdi、gdi_bulk を含む） | 12 | 236 | 0 |
| g07 hook_points … loadimage（jt_cpu、k32_dsp を含む） | 11 | 134 | 0 |
| g08 locale … osver | 11 | 195 | 0 |
| g09 proc … sse_pack | 11 | 208 | 0 |
| g10 sse_scalar … vcall_thunk_dll | 11 | 126 | 0 |
| g11 ver … wall_r | 11 | 140 | 0 |
| g12 wall_r2 … x87ctl、wall_e1、wall_c1 | 9 | 140 | 0 |
| **計** | **115** | **1,575** | **0** |

基準（S3）の 113・1,526 に対する増減: fe-decode `jt_cpu` +3、`a4_report` +1、`gdi` +3、`gdi_bulk` +4、`sse_pack` +2、`wall_c1` +12、`wall_e1` +24 = **+49**。`fe-lift/tests/*.rs` 87 ファイルはすべてグループに入っている。最初の通し（共有カーソルの判定を `fle_loadimage` に置いた版）では `gdi.rs` の source test（`fle_load` の語を禁じる）が 1 件落ち、`fle_cursor.c` に移した後の 2 回目の通しで 0 failed になった（実装の欠陥ではなく、置き場所の誤り。その時の記録は `logs/tests-prelim/`）。

### 生成 C（HEAD の fe-lift と C1 の fe-lift。関数単位のダイジェストの比較。1 つずつ作って消した）

| モジュール | 関数 HEAD → 新 | 本体が変わった | 追加 | `unsupported_total` |
|---|---|--:|--:|---|
| FLEngine（註釈なし） | 59,380 → 59,380 | **4**（E1 の thunk） | 0 | 394 → 390 |
| dsp_ippv2（縮小、`--seeds`） | 8,030 → 8,479 | 680 | 449 | 390,454 → 389,479（−975 = LDDQU の箇所） |
| Blend2D | 2,067 → 2,067 | 0 | 0 | 585 → 585 |
| QFC | 6,723 → 6,723 | 0 | 0 | 41 → 41 |
| freetype（旧い種） | 1,643 → 1,643 | 0 | 0 | 49 → 49 |
| freetype（新しい種） | 1,643 → 1,653 | 0 | 10 | 49 → 49 |
| ILTools | 12,931 → 12,931 | 2（E1 の version.dll の thunk） | 0 | 613 → 611 |

dsp の +449 の内訳は読み（434 の `S − 16` の入口と、4 つの init 呼びの先の 15）。

### 6 モジュール構成の a4（M1。停止点ごとの測定）

| 段階 | init_count | functions | entries | depth | 停止／終わり |
|---|--:|--:|--:|--:|---|
| 基準（E1 を当てた木） | 246 | 3,386 | 134,274 | 45 | HeapReAlloc |
| 壁 1 の後 | 246 | 3,465 | 134,577 | 39 | `indirect call to 0x4b40e10 …` |
| 壁 2 の後 | 246 | 3,695 | 152,333 | 34 | `unsupported instruction at 0x38b76eb` |
| 壁 3 の後 | 246 | 3,739 | 156,601 | 36 | `jump table at 0x3363e4e …` |
| 壁 4 の後 | 246 | 3,781 | 165,636 | 33 | GetObjectW(DIBSECTION) |
| 壁 5 の後 | 246 | 3,829 | 165,939 | 32 | StretchDIBits |
| 壁 6 の後 | 246 | 3,830 | 165,953 | 32 | SelectPalette(NULL) |
| 壁 7 の後 | 246 | 3,860 | 495,411 | 31 | DestroyCursor（0x7fca021c） |
| 壁 8 の前半の後 | 246 | 3,860 | 617,967 | 31 | DestroyCursor（0x7fda5008） |
| 壁 8 の後 | （報告は 0 of ?） | 5,306 | 1,007,942 | 12 | `returned from entry`、終了コード 0 |
| T1 の後（最終。独立な 2 回のビルド） | **942 / 942** | 5,306 | 1,007,942 | 12 | `returned from entry`、終了コード 0、`all_units_done=yes` |

- 完走の確認は a4 の報告に頼っていない: gdb のウォッチポイント（`InitCount` の VA 0x15EAC98）が 1, 2, …, 942 と連続して増え（943 回の停止 = 942 の増加 + 0 への戻り）、`InitContext+0`（0x15EAC90）が 0x13768b0 から 0 に戻った（`logs/gdb-wc-final.txt`。最終の native）。longjmp は 3 回で、QFC、ILTools、FLEngine の `Halt0`（FreePascal のライブラリの DllMain の正常な終わり。`f_1d0f9c0`／`f_600e090`／`f_4126f0`）。ゲストの例外ではない（読み）。depth 12 は longjmp で捨てられたフレームの残り。
- E1 の API の呼び出し（gdb、最終の native）: `GetComputerNameExW` 2 回、`GetComputerNameW`／`GetFileVersionInfoW`／`VerQueryValueW`／`LocaleNameToLCID`／`GetUserNameW` は 0 回。
- `EXPECT_FLE_SHA`（註釈あり）= `e047beeb44f16aa406a6ec254cc6603be192044206195c62ea2d459ec88bfdd2` を渡して gen の SHA 検査を通した。

### 変異検査（12 件。木のコピー上で 1 件ずつ変更し、テストが落ちること。元の木は変えていない）

| # | 変異 | 落ちたテスト |
|---|---|---|
| M01 | HeapReAlloc の `HEAP_ZERO_MEMORY` のゼロ埋めを外す | `heap_realloc_zero_memory_zeroes_the_grown_part_only` |
| M02 | 同じ番地のときに表の要求サイズを更新しない | `wall_c1` の 4 件（keeps_the_contents…、moves_the_table_entry…、zero_bytes…、accepts_no_serialize…） |
| M03 | `IN_PLACE_ONLY` の拡大の停止を外す | `heap_realloc_in_place_only_…` |
| M04 | LDDQU を整列した読み込み（`fle_ldxa`）にする | `lddqu_matches_host`、`lddqu_other_forms` |
| M05 | 添字 −1 の受理から「値 = S − 16」の検査を外す | `the_init_slot_is_not_a_case_when_a_condition_fails` |
| M06 | DIBSECTION の biSizeImage を 0 にする | `get_object_of_a_dib_section_gives_a_dibsection` |
| M07 | StretchDIBits のボトムアップの行の対応を外す | `stretch_dibits_reads_a_bottom_up_source_…` |
| M08 | SelectPalette(NULL) が前のパレットを返す | `select_palette_null_is_the_failure_value_and_changes_nothing` |
| M09 | 共有カーソルへの DestroyCursor が 0 を返す | `destroy_cursor_of_a_shared_standard_cursor_returns_1_and_keeps_it` |
| M10 | a4 のサンプルが最大でなく最後の値を使う | `a_header_zeroed_by_the_exit_path_is_replaced_by_the_sampled_one` |
| M11 | StretchDIBits の「同形式」の検査を外す | `stretch_dibits_forms_the_pages_do_not_fix_stop_by_name` |
| M12 | 共有カーソルの形の k の上限を 8 → 9 | `the_shared_cursor_handle_forms_are_recognised` |

12 件とも検出。検出されなかった変異は無い。無効な変異（コンパイルできない等）も無い。

## 解釈が分かれた点

1. **「壁」の数え方**: 壁 8 は 2 つのハンドル（標準カーソルと `LoadImageW` の形）で 2 回測定したが、1 つの壁（1 つの API の 1 つの停止文言）と数えた。a4 の最後の読み（T1）は壁に数えていない。壁 2（種）を壁に数えた（停止文言が壁だった）。数え方を変えれば 8 は 7〜9。
2. **壁 2 を「種」で直したこと**: 認識器（`lea` の指す先を自動で種にする）にする案は設計が大きく（決定 0102 以来の「種は測定用の入力」の扱いを変える）、指示は認識器を最後の手段とした。種の追加で直した。自動化の判断は PM（下 7）。
3. **壁 3 の LDDQU**: CPUID が SSE3 を名乗らない（決定 0075）のに実装した。決定 0106 が HADDPD を CPUID を変えずに実装した前例に従った。
4. **壁 4 は認識器の変更**で、指示は「最後の手段。設計が大きくなりそうなら止めて報告」。約 40 行・1 ファイル・実バイトの 4 条件・実行での証明が可能、として進めた（決定 0114）。
5. **壁 5 の DIBSECTION を答えにしたこと**: 決定 0108 の PM の判断 7 は「到達するまで停止のまま」で、今回到達した。ページに無い値は S5〜S7 の「出典なしの選択」として答えにした。停止のまま報告する選択肢もあった（その場合は壁 5 でループが止まる）。
6. **壁 7、8 の戻り値**: ページが結果を定義しない呼び出し（NULL の hPal、共有カーソルの DestroyCursor）に、ゲストが戻り値を使わないことを理由に答えを選んだ。
7. **T1（a4 の測定の道具）を直したこと**: 完走後に 0 と報告されるのはハーネスの欠陥で、gdb で独立に確認した。報告のコードに「サンプル」を足した（ヘッダが 0 でないときの出力は変わらない）。
8. **`a4-multi.sh` の複製**: 実際の `a4-multi.sh` ではなく、RAM の複製（`a4-multi-c1.sh`）で測定した。差は `HERE` の固定、`RELINK`／`SHARE_FLE`、生成 C の削除、clang のラッパー（`-g` を外し、分割した生成 C をコンパイル後に消す）。最終の 2 回のビルドは gen／build を通しで実行した。
9. **変更した既存テストの期待値**（上の表）。理由はすべて実装の変更で、実装の欠陥と思われるものは無い。

## PM の判断が要る点

1. **これを「A4 完走（仮の応答 N 件を含む）」と数えるか**（決定 0101 の表記）。判定コマンドの数値条件（`completed 895 / 895`、`all_units_done=yes`、`returned from entry` で終了コード 0）は native で満たした。N は README の「仮」の行の数: HEAD で 43、この作業の 0112、0113、0114 を足すと **46**。ただし (a) モジュールの基底・種・註釈は測定用の値（決定 0101 は基底を決定にしていない）、(b) 決定 0046 の判定は native と wasm の両方を求めるが、wasm は未確認、(c) 応答に実機 Windows との照合が無い（この作業の S1〜S13 を含む）。
2. **決定 0110（決定 0018 の緩和。製品データ 2 ファイルの種付け）の承認**。M0（配置なし）は 246/942 のまま StretchDIBits で止まる。完走は配置に依存する。承認しない場合、フォントの無い `TBitmap`（1×1 の DDB）への StretchDIBits をどうするかが次の壁（選択肢: DDB への書き込みを受けて捨てる（決定 0108 の PM の判断 4 は採らなかった）／ゲストが読まない経路かを調べる／名前付き停止のまま）。**推奨**: 0110 を承認し、M1 を A4 の判定構成にする。
3. **S5〜S11 の出典なしの選択の確認**（特に S6 の BI_RGB 32 ビットのマスク、S10、S11）。実機との照合は 0 件。連鎖でゲストがこれらの値を使ったか（`CopyBitmap` が dsBmih を読んで何をするか）は追っていない（読み）。
4. **決定 0114（認識器の変更）を入れてよいか**。
5. **CPUID は SSE3 を名乗らないのに、ゲストが SSE3 の命令（`lddqu`、`haddpd`）を実行する**。IPP の CPU 振り分けがなぜ SSE3 の経路を選ぶのか（読み: 振り分けの変数と CPUID 以外の入力）は調べていない。CPUID に SSE3 を足すか、実装のままにするか。
6. **wasm の測定**（決定 0046 の判定は native と wasm の両方）。この作業の変更は wasm のビルドで確認していない（runtime と生成器の C の変更だけで、新しい `.c` は無い）。
7. **freetype の葉関数の種の自動化**（`lea` の指す先を認識器が種にする）を別の決定として設計するか。dsp、Blend2D、QFC、ILTools にも同じ形の取りこぼしがありうる（未調査）。
8. **census（dsp 構成）**を C1 では取っていない。E1 の census での予測（決定 0112）は未検証。

## 限界

- 実機 Windows との照合は 0 件。wasm は未確認（native のみ）。
- 全体テストは 1 回の通しではなくグループの合算。変異検査は 12 件（D の案の全部ではない）。E1 の変異検査（M1〜M15）は実行していない。
- 完走は測定用の基底・種・註釈と仮の応答を含む構成での値。完走した後のこと（InitUnits の後のエンジンの動作）は測っていない。
- 連鎖の中で S5〜S11 の応答がゲストにどう使われたかは、`GetObjectW` の呼び元（`CopyBitmap` 相当）を読んだ範囲。dsBmih の使われ方の全体は追っていない。
- 壁 2 の種は、実行で見つかった 1 個（0x4b40e10）から、同じ形の 10 個を一括で足した。残りの 9 個がゲストに使われたかは未確認。
- 同時に走らせた測定は 1 構成だけ。`/tmp/opencode` の使用量は最大で約 2.7 GB に達した瞬間がある（「規則違反の自己申告」は MERGE-NOTES）。

## 撤回条件

- 壁ごとの応答（S1〜S11）は、実機 Windows の挙動が示されて食い違ったとき、または Learn の更新で結果が定義されたときに見直す。
- 決定 0114 は、その決定の撤回条件による。
- T1（a4 のサンプル）は、InitContext を 0 に戻さない構成にしたとき、またはハーネスが InitUnits の完了を別の方法（エントリの戻りの直前のスナップショット）で記録するようになったときに不要になる。

## 訂正（2026-10-28、独立検証 `docs/investigations/a4-six-module-completion-verification.md` による）

本書の T1 の行（41 行）と、a4.c のコメント／`a4: note:` の文言にある次の 2 点は不正確である。測定値には影響しない。

1. **コンパイラ**: 「FreePascal の `Halt0`」と書いたが、`FLEngine_x64.dll` には `Embarcadero Delphi for Win64 compiler version 36.0` の文字列があり、Delphi 製である（読み。RTL のソースとの照合はしていない）。
2. **完走後の InitCount**: 「ユニットを finalize し」と書いたが、独立検証の gdb の観測では、InitCount は 1 回の書き込みで 0 になる（942→941→… の下降は無い）。FinalizeUnits 相当の関数は InitCount が 0 の状態で呼ばれ、ループは回らない。各 unit の finalizer（896 個）は 1 つも呼ばれていない。`f_4128b0` が InitCount を 0 にし、`f_412770` が InitContext+0 を 0 にし、最後にゲスト自身の longjmp（決定 0100）で DllMain に戻る。

a4.c のコメントと `a4: note:` の文言（`tests/a4_report.rs` の `NOTE` が同じ文字列を検査している）は、直す作業の際に合わせて直す（この訂正では直していない）。
