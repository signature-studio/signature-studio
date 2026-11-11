# 0108: ILTools の壁 W6（`gdi32!CreateCompatibleDC`）を越えるための GDI のメモリ DC のモデル — DC、ビットマップ（DDB と DIB セクション）、ブラシ、選択の状態を持ち、画素を読み書きする操作は名前付きで止める（HEAD d3be2a3 に統合し、コンパイル、全体テスト、6 モジュール構成の測定まで実施。実機照合なし）

状態: **仮の決定**（番号 0108 は PM の仮の割り当て）。第 1 段階（設計と実装案。作業者、2026-10-27）の後、**第 2 段階（統合・検証・測定担当、同日）が HEAD `d3be2a3`（決定 0106、0107 が入っている）に差分を統合し、RAM 上でコンパイルし、全体テストを失敗 0 まで確認し、6 モジュール構成の a4 と census を測った**（下の「第 2 段階の結果」）。0054、0057、0066、0068、0098、0100、0104、0105、0106 の本文は書き換えない。第 1 段階の基準の木は HEAD `fc940a9` に `wallI2.diff` を当てたものだったので、`w6-gdi.diff` は HEAD `d3be2a3` に直接は当たらない（R2 が入っているため 2 か所が拒否された。手で統合した。MERGE-NOTES.md の 1）。統合後の差分は **`resolved-W6.diff`**（HEAD `d3be2a3` の clean な `git archive` に `patch -p1` で当たる。7 ファイル）。

> **これは A4 の達成ではない。** この決定は壁 W6（`CreateCompatibleDC` の import thunk）の先を、実機の仕様から導ける範囲で答える設計と実装である。6 モジュール構成（測定用の基底と種）の FLEngine の **init_count は 246 / 942 のまま**（HEAD `d3be2a3` の木でも同じ）。動いたのは unit 245 の中の停止点だけで、`CreateCompatibleDC` の thunk（0x601d6d0）から次の import の thunk（`gdi32!MoveToEx`、0x601da30）になった（実測）。census の数は診断で A4 の進捗ではない（決定 0046、0060、0097）。**実機 Windows との照合は 0 件**で、応答はすべて仮。凡例が「実測」の数は第 2 段階で測った値、「読み」は逆アセンブルと Learn を読んだ結果で実行して確かめていないもの。

記入状況: 第 1 段階（設計・実装案・文書）と第 2 段階（統合、コンパイル、全体テスト、生成 C の関数単位の比較、6 モジュール構成の a4 を 2 回と gdb、census）を記入済み。**未実施**: 変異検査 36 件のうち 29 件、wasm のビルド、`cargo test --workspace` の 1 回の通し（グループ分けの合算で代えた）、実機 Windows との照合（0 件）、`MoveToEx` より先の応答。**実施した**: Microsoft Learn の取得と読み（45 ページ。本文は `w6-results/learn/` に保存）、ILTools の逆アセンブルと直接 call の閉包の計算、FLEngine の import thunk の走査、統合、C のコンパイル（clang -O0／-O1／-O2、gcc -O1）、全体テスト（112 test result・1,469 passed・0 failed）、変異検査 7 件、FLEngine と他 5 モジュールの生成 C の関数単位の比較、6 モジュール構成の a4（独立した 2 回のビルドと実行）と gdb、census（6 モジュール構成と dsp 構成、それぞれ HEAD と W6）。

区分の凡例: **Learn** = Microsoft Learn のページに書かれていること（取得日 2026-10-27）。**読み** = 逆アセンブルを読んだ結果で、実行して確かめていない。**選択** = 一次資料に書かれていない作業者の選択（「出典なしの選択」）。**推測** = 作業者の推論。**未確認** = 確認していない。

## 第 2 段階の結果（実測）

作業場所は RAM（`/tmp/opencode/m2/src/` = HEAD `d3be2a3` の `lift/` と一致することを `diff -r` で確認した前回の統合の木、ビルドは `/tmp/opencode/m2/target`）。ワークツリーと `/` には書いていない。

### 統合とコンパイル

- `w6-gdi.diff` を HEAD の木に `patch -p1`: 6 ファイル中 4 つは当たった（`tests/gdc.rs`、`tests/gdi.rs`、`runtime/fle_gdi.c`、`fle_gdi.h`。ずれ・fuzz なし）。**拒否は 2 か所**: `imports_table.rs`（末尾。I2 の CreateBitmap の行の後ろに R2 の shfolder の行が入っているため）、`tests/vcall_thunk_dll.rs`（コメントと 514 の値。HEAD は R2 込みで 513）。手で統合した（PM の順序 I2 → R2 → W6。W6 の 19 行は shfolder の行の後ろ）。
- コンパイル: `cargo build`（dev）は**警告 0・エラー 0**。テスト binary のビルドも**型・構文のエラー 0**（第 2 段階で直した Rust／C の誤りは 0 箇所）。`fle_gdi.c` は `clang -std=gnu11 -O0／-O1／-O2 -Wall -Wextra -Werror -fPIE` と `gcc -O1 -Wall -Wextra` で警告 0（第 1 段階が確認できなかった最適化依存の警告も出ない）。テスト側の警告 2 件は HEAD の既存（`tests/monitor.rs` の `INIT_HEX`、`tests/sse_shuf.rs` の不要な `mut`）。

### 全体テスト（実測。グループ分けの合算。1 回の通しではない）

**112 test result・1,469 passed・0 failed・0 ignored**（fe-pe 83、fe-decode 140、fe-lift 1,246）。前回の統合の木（112・1,428・0）に対し、増えたのは `tests/gdi.rs` の **55 → 96 = +41** だけ（binary ごとの比較で他に差がない）。環境変数 `FLENGINE_PE`／`FLENGINE_DLL`／`ILTOOLS_DLL`／`BLEND2D_PE`／`DSP_IPPV2_PE`／`DSP_IPPV2_PE_2000000` を実ファイルに向けた。ENOSPC・quota のエラーは 0 件。skip を調べるために影響を受ける 13 binary（vcall_thunk_dll、gdc、gdi、imports_table、modules、cursor、loadimage、realmod、sysmetrics、spi、deferred、census、hook_points）を `--nocapture` で再実行した: `deferred` の 2 件だけ、既定のパス（`out-blend2d/Blend2D.dll`、`out-qfc/quickfontcache_x64.dll`）が `git archive` の木に無く SKIPPED と出る（前回の統合でも同じ）。2 つの DLL を置いて再実行し 15 passed（skip 0）。

### 生成 C の確認（FLEngine、註釈なし。実測）

| 項目 | HEAD `d3be2a3` | W6 統合 |
|---|---|---|
| ファイルの SHA-256 | `794d26cb38f6aad5c92390f8fe1bcaa740d7c57f28840382cd4d56898cb0bcdd` | **`94fe0c853537cefbcbc674b65e77c79464b8659a2c250b46dc762f6bf2ddc6b6`** |
| バイト数 | 215,747,616 | 215,751,676（+4,060） |
| functions／bad_nodes | 59,380／0 | 59,380／0 |
| `unsupported_total`（`Jmp: indirect through IAT slot`） | 513 | **494**（−19） |
| `fle_resp_count`／`fle_dispatch_count` | 312／59,692 | **331／59,711** |
| 註釈あり（`--hook-points flengine-hook-points.txt`。a4-multi.sh の測定が使う形） | `6732af99fc6c7b9e7de384e8baffc9eeb4031bbe20ee7886d6c2baf48ed57028`（215,748,463。HEAD の木から再生成して一致を確認） | `ce205cfe5f70de6d6556651242466c6b4c4255769d09b81fdf9b5b3f77bf7b8a`（215,752,523） |

関数単位の比較（HEAD の値は前回の統合が保存した関数ごとの SHA の表 `new.digest.tsv`（SHA 794d26cb… の C）。W6 の C は同じ道具 `cdigest.py` で作った）: 59,385 行（関数 59,380 + 節 5）のうち**本体が変わった関数は 19 だけ**で、すべて W6 の 19 API の thunk（`fle_unsupported(…)` → `fle_imp_<API>(); return;`）。アドレスと API: f_423ee0 BitBlt、f_423f40 CreateBrushIndirect、f_423f50 CreateCompatibleBitmap、f_423f60 CreateCompatibleDC、f_423f80 CreateDIBSection、f_423fd0 CreateHalftonePalette、f_424070 DeleteDC、f_424200 GetDIBColorTable、f_424210 GetDIBits、f_424410 PatBlt、f_424480 RealizePalette、f_424520 SelectObject、f_424530 SelectPalette、f_424550 SetBkColor、f_424580 SetDIBColorTable、f_424630 SetTextColor、f_4248d0 CreateIconIndirect、f_424b20 FillRect、f_424ec0 GetSysColor（第 1 段階が `ff 25` の走査で挙げたアドレスと 19 件すべて一致）。ほかに変わったのは、プロトタイプの節（+19 行）、`fle_dispatch_table`（+19 行）、`fle_resp_table`（+19 行）、2 つの個数だけ。**予測（FLEngine が import する W6 の 19 API の thunk だけが変わる）どおり**で、`vcall_thunk_dll.rs` の予測 513 − 19 = 494 も当たった。第 1 段階が「突き止めていない」とした走査の 523 と 514 の 9 の差は、今回も説明できていない（予測は当たった）。

他の 5 モジュールの C（a4-multi.sh の引数と同じ。註釈つき・種つき）を HEAD と W6 の fe-lift で作り、関数単位で比べた: **dsp_ippv2 0、Blend2D 0、freetype 0、QFC 18、ILTools 19** の関数の本体が変わった（関数の数は両方同じ。5 モジュールとも C のファイルとしては応答表の節が変わるので SHA は変わる）。ILTools の 19 は `w6-import-closure.tsv` の `thunk_va_rebased`（19 API）と**完全に一致**（f_601d660、f_601d6b0、f_601d6c0、f_601d6d0、f_601d6e0、f_601d710、f_601d770、f_601d890、f_601d8a0、f_601da50、f_601dac0、f_601db50、f_601db60、f_601db70、f_601dba0、f_601dc40、f_601de40、f_601e080、f_601e350）。QFC の 18 は QFC が import する W6 の API の thunk（f_1d1dbf0〜f_1d1e030。アドレスと API の対応は表にしていない）。

### 6 モジュール構成の a4（測定。HEAD の木と W6 の木。いずれも註釈あり）

構成は決定 0106 の S6 と同じ引数（dsp_ippv2 0x2000000 + 種 452、Blend2D 0x4700000 + pdata + 種、QFC 0x1d00000 + pdata + 註釈、freetype 0x4b00000 + pdata + 種、ILTools 0x6000000 + pdata + 種 400 + 註釈。FLEngine も註釈つき）。素材 DLL の SHA-256 は 0106 の記録と一致。`a4-multi.sh` の複製（`HERE` の固定と、gen の後に生成 C を消す 2 行だけ違う。ディスク quota のため）と、`-g` を外し分割ソースをコンパイル後に消す `clang` のラッパーを使った。

| 木 | init_count | last unit | started／completed／in progress | functions | entries | call depth | 停止 | 終了コード |
|---|---|---|---|--:|--:|--:|---|--:|
| HEAD `d3be2a3`（この環境で再測。決定 0106 の S6 と同じ値） | 246 / 942 | 245（proc 0x733090） | 209／208／1（895 runnable） | 3,373 | 133,960 | 33 | `unsupported instruction at 0x601d6d0`（`gdi32!CreateCompatibleDC` の thunk） | 134 |
| **W6、1 回目** | **246 / 942** | 245 | 209／208／1 | **3,377** | **133,965** | **36** | **`fle: unsupported instruction at 0x601da30`**（`gdi32!MoveToEx` の thunk。abort kind `fle_unsupported`） | 134 |
| **W6、2 回目**（別のディレクトリに一から作り直し。生成物はすべて別） | 246 / 942 | 245 | 209／208／1 | 3,377 | 133,965 | 36 | 同じ | 134 |

2 回の W6 は `native.txt` が `calling entry … rsp=` の 1 行（スタックのアドレス）を除いて同一、`native.err` と `units.tsv` は完全に同一、5 モジュールの生成 C の SHA-256 も同一。HEAD の再測が 0106 の S6 の値（3,373／133,960／33／0x601d6d0）に一致したので、測定の手順（ラッパー、註釈、素材、ビルド）は 0106 の記録と同じ結果を出す（実測）。

- **停止メッセージ全文**: `fle: unsupported instruction at 0x601da30`。**分類: 未応答 import**（応答表に行が無い import の thunk）。API 名は `w6-import-closure.tsv` の `thunk_va_rebased` から `gdi32.dll!MoveToEx`（FLEngine 側の同じ import の thunk は 0x4243f0。その行は `import-table-only`、`row_after_w6` = no）。census の側でも独立に `gdi32.dll!MoveToEx` と同定された。
- **停止点の C スタック**（gdb の `bt` から `f_*`。32 フレーム。ハーネスの出力は先頭の 10 まで。call depth はハーネスの数で 36）: `f_601da30 ← f_6130690 ← f_6130f60 ← f_6131150 ← f_61374a0 ← f_61311b0 ← f_6130190 ← f_625ee30 ← f_625ef30 ← f_6262b60（export CreateCursorFromFont）← f_625ed40 ← f_625cfc0 ← f_622aa30 ← f_622ab90 ← f_600da60 ← f_600db00 ← f_60186d0 ← f_62b3610 ← f_62b35f0（ILTools の DllMain）← f_423990 ← f_44db40 ← f_6539e0 ← f_815e10 ← f_814e60 ← f_812e80 ← f_732f30 ← f_733090 ← f_4120c0 ← f_412160 ← f_41d8f0 ← f_1376840 ← f_1376820`。先頭の `f_601da30` の 4 つ後ろが W6 の呼び出し元 `f_61374a0`（第 1 段階が `TBitmapCanvas.CreateHandle` と読んだ関数）で、`CreateCompatibleDC` から戻ったあと `f_6131150 → f_6130f60 → f_6130690` と 3 段下がって `MoveToEx` を呼んでいる。
- **停止の瞬間のゲストのレジスタ**（`R[]`、実測）: `rcx` = 0x7fba00d0（直前の `CreateCompatibleDC` が返した DC）、`rdx` = 0、`r8` = 0、`r9` = 0。`MoveToEx(HDC, int x, int y, LPPOINT)` のシグネチャで読むと `MoveToEx(hdc, 0, 0, NULL)`（**読み**。この API の Learn のページは読んでいない）。VCL が描画の起点（ペン位置）を DC に設定する呼び出しと**推測**する。
- **W6 の API の呼び出し実績**（gdb。W6 の 19 API と、既存の関連 5 API（CreateBitmap、DeleteObject、GetObjectW、GetStockObject、GetDeviceCaps）の `fle_imp_*` にブレークポイント。引数は `rcx`／`rdx`／`r8`／`r9`／`[rsp+0x28]`／`[rsp+0x30]` と戻り値 `rax`。全記録は `logs/gdb/w6trace1.tsv`）: 停止までに W6 の API は **`CreateCompatibleDC` が 1 回だけ**（引数 `hdc` = NULL、呼び出し元 0x613750b、戻り値 0x7fba00d0）。**残りの 18 は 0 回**（SelectObject、SelectPalette、GetSysColor、SetBkColor、SetTextColor、DeleteDC、CreateDIBSection ほか）。既存の関連 API は GetDeviceCaps 8、GetStockObject 9、GetObjectW 10、CreateBitmap 1（`w = h = 0`、planes 1、bpp 1、lpBits NULL → 1 × 1 のモノクロ。戻り値 0x7fba00bc、決定 0106 の W5）。ハンドルの採番（実測）: CreateBitmap 0x7fba00bc → 既定の 4 つ 0x7fba00c0〜0x7fba00cc → DC 0x7fba00d0 → プレースホルダ 0x7fba00d4（決定 D3 の順序どおり。D3 の「33〜40」は新しい runtime での数字で、この連鎖では先に作られた GDI オブジェクトの分だけずれる）。
- **第 1 段階の読み（MERGE-NOTES の 3 章 7）との比較**: 予想は `CreateCompatibleDC(NULL)` → （0 × 0 ならハンドル 0 で `SelectObject` を飛ばす）→ `GetSysColor(8)` → `SetBkColor` → `SetTextColor` → `graphic.Draw` → `DeleteDC`。**実測は `CreateCompatibleDC(NULL)` の直後に `MoveToEx(hdc, 0, 0, NULL)`** で、予想の呼び出しのどれにも届いていない。`MoveToEx` は第 1 段階の「82 の import 表にあるだけの gdi32 import」の 1 つ（直接 call の閉包では届かず、キャンバスのハンドルの設定を vtable 経由で呼ぶ経路と読む）。**`GetSysColor(8)` = 黒、初期のテキスト色／背景色、DIB のゼロ埋め、DDB の `bmBits` = 0 などの「出典なしの選択」は、この測定では一度も観測されていない**（呼ばれていない）。0 × 0 の矩形（決定 0106 の解釈の点 1）は直していない（PM の決定 5）ので、`CreateBitmap(0, 0)` → 1 × 1 のまま進み、DIB セクションや色表の経路には届いていない。

### census（診断。A4 の進捗ではない。実測）

6 モジュール構成の census は**最後まで走らない**（2 つの木の両方）。census の道具（`harness/census/` の `census.c` と `main_two.c` を、a4 のビルドと同じオブジェクトにリンクした別の実行ファイル。`FLE_CENSUS` が無いときは a4 と同じ値を出すことを W6 の実行ファイルで確認した。HEAD のものは確認していない）で 1 回ずつ:

| 木 | ok | stop | fault | hang | empty | noproc | not_reached | 終わり方 |
|---|--:|--:|--:|--:|--:|--:|--:|---|
| HEAD `d3be2a3`（6 モジュール） | 208 | 1 | 0 | 0 | 46 | 1 | 686 | unit 245 が `CreateCompatibleDC` で放棄 → unit 246 を始めた（InitCount = 247）ところで SIGSEGV（fault address 0x16d10）が起き、census は unit として検出できず（246 は not_reached）、census が終了 |
| **W6（6 モジュール）** | **227** | **1** | **1** | 0 | 46 | 1 | **666** | unit 245 が **`gdi32.dll!MoveToEx`** で放棄 → unit 265 が SIGSEGV（fault address 0xe、`f_410320` ← `f_5af120` ← `f_7a6bc0`）で放棄 → unit 266 を始めたところで SIGSEGV（fault address 0xe）が起き、同様に census が終了 |
| HEAD（dsp 構成 = FLEngine + 縮小 dsp_ippv2。前回の H の構成） | 890 | 5 | 0 | 0 | 46 | 1 | 0 | 最後まで走った |
| **W6（dsp 構成）** | **890** | **5** | 0 | 0 | 46 | 1 | 0 | 同上。**HEAD と unit／proc／state／import の列が完全に一致**（W6 は dsp 構成の census に影響しない） |

- 6 モジュール構成の壁の一覧（上位 20。W6）: ① `gdi32.dll!MoveToEx`（1 unit: 245。分類 a 未応答 import）、② null ページへの SIGSEGV（`f_410320`。unit 265。二次の疑い）。TSV: `logs/census/top-walls-6mod-w6.tsv`。**この census の数（ok 227）は 942 unit の診断として使えない**: 666 unit が到達されていない。
- 打ち切りの原因（**推測**）: 6 モジュール構成では unit 245 の `LoadLibraryW "iltools_x64.dll"` が成功して ILTools の DllMain（と、その中の `InitUnits` に相当する入れ子の初期化）が走るので、unit 245 を途中で放棄すると、ゲストの初期化の大域の状態（census が読む `InitContext`、`COUNT_VA` = 0x15EAC98）が書き換わったまま残り、census の unit 検出（`InitCount - 1` の表の proc と呼び出し先の一致）が外れて以後の unit を数えられなくなる。手掛かり（実測）は、打ち切りの時のスタックが `f_6539e0 ← f_665bd0 ← f_66d050 ← f_66d1a0 ← f_4120c0` で、`f_4120c0` が呼んだ `f_66d1a0` が表の `InitCount - 1` 番目の proc（HEAD: 0x741880、W6: 0x7a7960）と違うこと。放棄のたびに戻さないものが多いという census の仕様（`census.c` の冒頭）の範囲の現象だが、原因の確認（`InitContext` の中身を読む）はしていない。**census を 6 モジュール構成で最後まで走らせるには census の道具か構成を変える必要があり、この作業ではしていない**。
- dsp 構成の壁（W6 でも HEAD でも同じ。上位 20、2 種）: ① `LoadLibraryW "iltools_x64.dll"`（3 unit: 245、280、659。分類 c）、② `kernel32.dll!GetComputerNameExW`（2 unit: 830、880。分類 a。H の census では「null 関数ポインタ呼び（二次の疑い）」だったもの。W6 とは無関係に、I2／R2 の統合で SM 86 などが答えられた結果、先に進んで現れた壁と読む。未確認）。TSV: `logs/census/top-walls-2mod-{w6,head}.tsv`。前回（HEAD + H の dsp 構成）は ok 888／stop 7（壁 4 種）、今回の HEAD は ok 890／stop 5（壁 2 種）。

### 変異検査（第 1 段階の案 36 件のうち 7 件を実行。実測）

`FLE_GDI_SRC_DIR` に変異した `fle_gdi.c`／`fle_gdi.h` を置いて `--test gdi` を実行。**7 件すべてを検出した**（変異を置いたのは別ディレクトリで、`runtime/` のファイルは変えていない）:

| # | 変異 | 落ちたテスト（実測） | 第 1 段階の予想との差 |
|--:|---|---|---|
| M1 | `DeleteObject` が `sel` を見ない | `a_halftone_palette_…`、`delete_object_of_a_dib_section_…`、`delete_object_of_a_selected_object_…`（3 件） | 予想と同じ 3 件 |
| M3 | ビットマップの二重選択を許す | `select_object_returns_the_previous_bitmap_and_a_bitmap_is_in_one_dc_at_a_time`（1 件） | 同じ |
| M4 | `SelectObject` が新しいものを返す | `delete_dc_unselects_…`、`select_object_returns_the_previous_bitmap_…`、`select_object_selects_brushes_pens_and_fonts_…`（3 件） | 「ほぼ全部」より少ないが、期待した型を検出 |
| M8 | 背景色の既定を黒にする | `a_memory_dc_starts_with_…`、`the_constants_of_decision_0108`、`the_text_and_background_colours_…`（3 件） | 同じ |
| M14 | DIB のストライドの DWORD 切り上げを外す | `a_dib_section_of_8_bits_or_less_keeps_the_colour_table_…`、`a_dib_section_gets_its_bits_a_handle_and_the_stride_of_the_page`（2 件） | `get_object_of_a_dib_section_…` は落ちなかった（予想より 1 件少ない） |
| M21 | `BitBlt` が停止せず 1 を返す | `bitblt_patblt_fillrect_getdibits_and_realizepalette_…`、`the_source_of_decision_0108_…`（2 件） | 同じ |
| M28 | `GetSysColor(8)` が白 | `only_the_window_text_system_colour_is_answered_and_it_is_a_choice`、`a_second_thread_stops_the_new_calls_too_…`（2 件） | `the_constants_of_decision_0108` は落ちなかった（予想より 1 件少ない。定数のテストは `GetSysColor` の C を見ない） |

残り 29 件（M2、M5〜M7、M9〜M13、M15〜M20、M22〜M27、M29〜M36）は**実行していない**。


## 事実の出典（読んだもの）

- 決定 0106（W5、W6 の記述、`e-6mod-final/` の C スタックと gdb の記録、`d-6mod-createbitmap-wall-createcompatibledc/`）、0105（インライン フック）、0104（ILTools の実在モジュール化）、0100（`jmp [mem]` の tail dispatch）、0098（`EnumFontFamiliesExW`）、0069（`LoadIconW(NULL, L"MAINICON")`）、0068（`GetObjectW`）、0066（`CreateFontIndirectW`）、0054（仮想 GDI オブジェクト）、0053（仮想画面 DC）、0015（応答表は末尾に足す）。`lift/runtime/fle_gdi.c/.h`、`fle_gdc.c/.h`、`fle_cursor.h`、`fle_loadimage.h`、`fle_env.h`、`fle_sys.c`（`HeapAlloc` が host の `malloc`/`calloc` を使う作り）、`tests/gdi.rs`、`tests/gdc.rs`、`tests/cursor.rs`、`tests/loadimage.rs`、`tests/vcall_thunk_dll.rs`。
- **Microsoft Learn（2026-10-27 に `curl` で取得して読んだ。`updated_at` は各ページの meta）**。URL は `https://learn.microsoft.com/en-us/windows/win32/api/` の下:
  - `wingdi/nf-wingdi-` + `createcompatibledc`、`deletedc`、`selectobject`、`createcompatiblebitmap`、`createdibsection`、`getdibits`、`getdibcolortable`、`setdibcolortable`、`selectpalette`、`realizepalette`、`getpaletteentries`、`bitblt`、`patblt`、`settextcolor`、`setbkcolor`、`getbkcolor`、`gettextcolor`、`getbkmode`、`createbrushindirect`、`getobject`、`deleteobject`、`getstockobject`、`getcurrentobject`、`getdevicecaps`（いずれも `updated_at` 2024-02-22）。`createhalftonepalette`（**2026-08-21**。「The exact color entries are an implementation detail and are not guaranteed to be identical across devices or Windows versions」の記述はこの版にある）。`getobject`（2025-11-25）。
  - `winuser/nf-winuser-` + `fillrect`、`createiconindirect`、`destroycursor`、`destroyicon`、`geticoninfo`（2025-07-01）、`getsyscolor`（2024-02-22）。
  - 構造体: `wingdi/ns-wingdi-` + `bitmap`、`dibsection`、`bitmapinfoheader`、`bitmapinfo`、`rgbquad`、`logbrush`、`paletteentry`、`winuser/ns-winuser-iconinfo`（2024-02-22）。
  - 概説: `https://learn.microsoft.com/en-us/windows/win32/gdi/` + `memory-device-contexts`、`graphic-objects`、`graphic-modes`、`device-context-types`、`about-device-contexts`、`device-contexts`（2025-03-11）。
  - **読んで見つからなかったもの（「未確認」の根拠）**: メモリ DC の**初期の描画色**（SetTextColor／SetBkColor／GetTextColor／GetBkColor のどのページにも既定値が無い）、**DC が最初に持つ既定のペン・ブラシ・フォントの正体**（graphic-objects は「the system automatically stores a set of default objects in it (there is no default bitmap or path)」とだけ書く）、**システムカラーの値**（GetSysColor は要素の名前と番号だけ）、**CreateDIBSection の初期のバイト**、**GDI が 32 ビット DIB の未使用バイトに書く値**。
- 読み（ILTools。元のファイル `lift/out-inventory/extracted/ILTools_x64.dll`、SHA-256 `20409dcb…061a0f95`、ImageBase 0x400000。**リベース済みの像は手元に無かったので、ファイルのまま RVA で読み、決定 0106 の基底 0x6000000 の VA は +0x5c00000 で対応させた**）。道具は `lift/out-git/w6-results/tools/`（`w6_common.py` ほか。実行は RAM 上の `/tmp/opencode/w6/py/`）。
- FLEngine（`binaries/win64/FLEngine_x64.dll`、読み取りのみ）の import thunk の走査（`ff 25` の rip 相対 jmp が IAT スロットを指すものを数えた）。

## 背景（事実と読み）

### 壁 W6

- 停止（決定 0106 S6、**実測**）: `fle: unsupported instruction at 0x601d6d0`。`gdi32!CreateCompatibleDC` の IAT thunk。C スタックの先頭は `f_601d6d0 ← f_61374a0 ← f_61311b0 ← f_6130190 ← f_625ee30 ← f_625ef30 ← f_6262b60（export CreateCursorFromFont）← … ← f_62b35f0（ILTools の DllMain）`。
- **読み（推測を含む）**。`f_625ee30`（ファイルの VA 0x65ee30）は引数 (graphic, w, h, hotspot) から cursor を作る関数: `CreateBitmap(w, h, 1, 1, NULL)` で 1 ビットのマスクを作り（決定 0106 W5）、クラス参照 `[0x…]` から Delphi の `TBitmap` を作り、`dl = 7`（pf32bit と読む）の `f_613b470`（`PixelFormat` の setter と読む）、vtable +0xd8（`SetSize` と読む）、`f_61398c0`（`Canvas` の getter と読む）、vtable +0x88（`TCanvas.Draw(0, 0, graphic)` と読む）を呼び、ICONINFO（fIcon 0、hot spot、hbmMask = マスク、hbmColor = `TBitmap` のハンドル）で `CreateIconIndirect` し、マスクを `DeleteObject` し、`TBitmap` を解放する。以下の VCL の名前（`TCanvas.Draw`、`TBitmapCanvas.CreateHandle`、`CopyBitmap`、`ColorToRGB`）は**関数の形が Delphi の VCL（Graphics.pas）の同名の手続きと一致すると読んだ名前で、ソースとの照合はしていない（推測）**。
  - `f_6130190`（`TCanvas.Draw`）: 引数の graphic が NULL でなく空でなければ、`RequiredState`（`f_61311b0`。ハンドルが無ければ vtable +0x138 の `CreateHandle`）を呼び、`SetBkColor(handle, ColorToRGB(Brush.Color))`、`SetTextColor(handle, ColorToRGB(Font.Color))`（どちらも戻り値を使わない）、`graphic.Draw(canvas, rect)`（vtable +0x28）。
  - `f_61374a0`（`TBitmapCanvas.CreateHandle`）: **`CreateCompatibleDC(NULL)`**（`xor ecx, ecx` の 0x6137504）、画像のハンドルが 0 でなければ `SelectObject(dc, hbitmap)`（0x613752e）、パレットが 0 でなければ `SelectPalette(dc, pal, TRUE)` と `RealizePalette(dc)`（0x6137572、0x6137586）。`f_61373d0`（`FreeContext`）は逆に `SelectObject(dc, 旧)`、`SelectPalette(dc, 旧, TRUE)`、`DeleteDC(dc)`。
  - `f_612d9a0`（`ColorToRGB`）: 色が負なら **`GetSysColor(色 & 0xff)`**、そうでなければそのまま。`TCanvas.Draw` は既定のフォントの色 `clWindowText`（= `$80000008`）をこれで変えるので、**GetSysColor(8) が CreateCompatibleDC の直後に来る**。
  - `f_6137910`（ファイルの VA 0x537910。`CopyBitmap` と読む。0x6137910〜0x6138223）: `GetObjectW(h, 0x68, buf)`（既存ハンドルのとき）、`GetDC(0)`、`CreateCompatibleDC(screen)` 2 つ、`CreateBitmap`／`CreateCompatibleBitmap(screen, w, h)`／`CreateDIBSection(screen, pbmi, 0, &bits, 0, 0)`、`SelectObject`、`GetDIBColorTable(dc, 0, 256, …)`、`GetDIBits`、`SelectPalette`＋`RealizePalette`、キャンバスが渡されたときの `FillRect`＋`SetTextColor`＋`SetBkColor`、1 ビットのときの `SetDIBColorTable(dc, 0, 2, …)`、`PatBlt(WHITENESS = 0xFF0062)`、`BitBlt(…, SRCCOPY = 0xCC0020)`、`DeleteDC`、`ReleaseDC`。
  - `f_61e9500`／`f_61e9680`（ILTools 自身の DIB バッファのクラス）: `CreateCompatibleDC(NULL)`、`CreateDIBSection(dc, pbmi, 0, &bits, 0, 0)`（`pbmi` は biBitCount 1／4／8／16／24／32、高さは**負（top-down）**、16 ビットは `BI_BITFIELDS`、行は `((w * bpp + 31) >> 5) << 2` バイト）、`SelectObject`。
- **0 × 0 の注意**: 決定 0106 の「解釈が分かれた点 1」（QFC の測定が空の矩形を返し、`CreateBitmap(0, 0, …)` が 1 × 1 になる）は**この決定では扱っていない**。この決定の連鎖は 0 × 0 でない場合にも成り立つ形で書いた。0 × 0 のとき `TBitmap` はハンドルを持たない（`f_61374a0` の `SelectObject` を飛ばす分岐）と読むが、実測していない。

### W6 の先の import（読み。直接 call だけを深さ 8 まで）

道具: `py/w6_closure.py`（`.pdata` の関数範囲の中の `call rel32`、`jmp rel32`（関数の外）、`call [rip+disp]`（IAT）を幅優先で辿る。`call reg`／`call [reg+disp]`（vtable 呼び出し）は**辿らない**）。起点は 2 つ: (E) ILTools の export `CreateCursorFromFont`（`f_6262b60`）、(W) W6 の呼び出し元 `f_61374a0`。深さ = 起点からの call の段数（import thunk を 1 段と数える）。一覧は **`w6-import-closure.tsv`**（150 行。列は下）。

| | 件数 | 内訳 |
|---|--:|---|
| 閉包（E ∪ W）の gdi32 の import | 22 | |
| 閉包の user32 の import | 11 | |
| 閉包のその他（kernel32 33、oleaut32 2） | 35 | 行が無いのは `kernel32!FormatMessageW` の 1 つ |
| ILTools の gdi32 の import 全体 | 104 | 22 が閉包、82 は import 表にあるだけ（直接 call では届かない。TextOutW 系、StretchBlt、StretchDIBits、SetDIBits、GetPixel、LineTo、Rectangle、SaveDC、SelectClipRgn、SetBkMode ほか。TSV に thunk の VA がある）。82 のうち行があるのは CreateFontIndirectW、EnumFontFamiliesExW、GetStockObject の 3 つ |
| ILTools の user32 の import 全体 | 199 | 閉包の 11 以外は数えていない（窓・メッセージ系が大半。TSV は閉包の分だけ） |

閉包の gdi32／user32 の 33 件を**最短深さ**（E と W の小さい方）で数える。「行あり」は応答表（HEAD + I2）に行があるもの、「行なし」は無いもの（= `fle_unsupported`）。実装済み（`fle_imp_*` が runtime にある）は行ありと一致する:

| 最短深さ | 件数 | 行あり（実装済み） | 行なし | 行なしの API |
|--:|--:|--:|--:|---|
| 1 | 4 | 0 | 4 | CreateCompatibleDC、RealizePalette、SelectObject、SelectPalette |
| 2 | 6 | 5 | 1 | CreateHalftonePalette（行あり: GetDeviceCaps、GetDC、GetSystemMetrics、ReleaseDC、SystemParametersInfoW） |
| 3 | 7 | 4 | 3 | DeleteDC、GetDIBColorTable、CreateIconIndirect（行あり: CreateBitmap、CreatePalette、DeleteObject、LoadCursorW） |
| 4 | 5 | 2 | 3 | CreateDIBSection、GetPaletteEntries、DestroyCursor（行あり: IntersectRect、LoadStringW） |
| 5 | 0 | — | — | （CreateCompatibleDC は E からは深さ 5、W からは 1。最短は 1） |
| 6 | 9 | 1 | 8 | BitBlt、CreateCompatibleBitmap、GetDIBits、PatBlt、SetBkColor、SetDIBColorTable、SetTextColor、FillRect（行あり: GetObjectW） |
| 7 | 2 | 0 | 2 | CreateBrushIndirect、GetSysColor |
| 8 | 0 | — | — | |
| 計 | **33** | **12** | **21** | |

起点別の深さ: (E) 33 件すべて深さ 2〜7（2: 2、3: 4、4: 8、5: 1、6: 16、7: 2）。(W) 13 件（1: 4、2: 4、3: 3、4: 2）。関数の数は (E) 269、(W) 125。**辿らなかった間接呼び出し（vtable 呼び出しなど）は (E) 92、(W) 30**: 閉包は**下限**で、`TCanvas.Draw` の `graphic.Draw`（vtable）の先、`TBitmap` の virtual メソッドの先は含まない。**決定 0106 の「行の無い約 20」は、`f_625ee30` から 8 段で、gdi32／user32 の 20（本書の 21 から DestroyCursor を除いたもの）と kernel32 の FormatMessageW**。本書は export から辿ったので `DestroyCursor`（深さ 4）が増えた。

TSV の列: `dll`、`api`、`scope`（`closure` = gdi32／user32 の閉包、`import-table-only` = 閉包にない gdi32、`closure-other-dll` = 閉包のその他）、`depth_export`、`depth_w6caller`、`depth_min`、`sites_in_closure`、`first_site_rebased`（最初の呼び出し点と関数。基底 0x6000000）、`arg_shape_first_site`（直前の命令の線形走査で読んだ引数の形。**分岐をまたぐ読みは保証しない**）、`thunk_va_rebased`（import thunk の VA。今後の `unsupported instruction at 0x601dXXX` の停止から API 名を引く表）、`flengine_thunk_va`、`row_at_I2`、`impl_at_I2`、`row_after_w6`、`w6_stage1`、`note`。

## 決定（作業者。仮）

| # | 項目 | 決定 | 区分 |
|---|---|---|---|
| D1 | 方針 | **実機の仕様（Learn）から導ける応答だけを実装し、導けないものは名前付きの停止にする**（`fle: <API>: <理由>`。書式は既存の `fle_gdi.c` と同じで、`gdi32!` の接頭辞は付けない）。画素を実際に読み書きする操作（BitBlt、PatBlt、FillRect、GetDIBits）、パレットを実体化する操作（RealizePalette）、描画を伴う操作は止める。偽の成功で先へ進まない | PM の方針 |
| D2 | メモリ DC の表現 | 既存の GDI オブジェクト表（`objs[]`、ハンドルは `0x7FBA0000 + 4 * (slot + 1)`、スロットは作成順に増え再利用しない）に**種類 `FLE_GDI_KIND_DC = 8`** を足す。新しい .c／.h は作らない。DC の状態（選択中のビットマップ・ブラシ・ペン・フォント・パレットのハンドル、テキスト色、背景色、DC 作成時の 1 × 1 モノクロのビットマップ）は `gdi_dc` に持つ。**`DeleteObject(hdc)` も DC を削除できる**（Learn「you can also call DeleteObject with the HDC to delete the DC」）。画面 DC（`GetDC(NULL)`、`fle_gdc.c`）とは別の表・別の領域のまま | 表の形は選択、DeleteObject は Learn |
| D3 | 新しい DC の初期状態 | (a) 1 × 1 モノクロのビットマップが選択済み（Learn）。(b) 4 つの既定のオブジェクト（ブラシ、ペン、フォント、パレット）が選択済み。**どのオブジェクトかは Learn に無いので、最初の DC を作るときに 4 つの隠しオブジェクト（種類 `FLE_GDI_KIND_DEFAULT = 10`）を作り、全 DC で共有する**（ストックの WHITE_BRUSH 等だとは**主張しない**）。(c) テキスト色 `0x00000000`、背景色 `0x00FFFFFF`。**(b) と (c) は出典なしの選択**。観測されるのは `SelectObject`／`SelectPalette`／`SetTextColor`／`SetBkColor` の戻り値だけ。最初の DC のスロットは 4 つの既定（33〜36）、DC（37）、プレースホルダ（38）、次の DC は 39、40 | (a) Learn、(b)(c) 選択 |
| D4 | 選択の状態 | 全オブジェクトが「選択されている DC の数」`sel` を持つ。`SelectObject` は同じ型の前のオブジェクトを返す。**ビットマップは 1 つの DC にだけ**（Learn「A single bitmap cannot be selected into more than one DC」。他の DC に選択中なら NULL）。**画面 DC にビットマップを選ぶと NULL**（Learn「Bitmaps can only be selected into memory DC's」）。`DeleteObject` は選択中なら 0 で何もしない（Learn「currently selected into a DC」）。`DeleteDC` は選択中のオブジェクトを**削除せず選択を外す** | Learn（DeleteDC が削除しないことは「SelectObject: 元のオブジェクトを選び直せ」からの読み） |
| D5 | ビットマップの 3 つの形式 | `fmt`: **MONO**（0。`CreateBitmap` の形、DC のプレースホルダ。決定 0106）、**COLOR**（1。仮想画面と同じ形式 = 1 プレーン 32 ビット（`fle_gdc.h`）の DDB）、**DIB**（2。`CreateDIBSection`）。DDB は大きさと形式だけで**画素を持たない**（決定 0106 P6 のまま）。DIB セクションだけ、ゲストが書くブロックを持つ | 選択（形式の名前）、32 ビットは決定 0053 |
| D6 | DIB セクションのメモリ | `calloc` で `stride * abs(biHeight)` バイト（stride = `((((biWidth * biBitCount) + 31) & ~31) >> 3)`。Learn の式）。**ゼロ埋め（出典なしの選択）**。ポインタは `*ppvBits` に書く。**runtime はこのブロックに一度も書かない**。`DeleteObject` で `free`（Learn「The system closes the handle to that memory when you later delete the DIB」）。2 GiB 以上は停止 | 式は Learn、ゼロ埋めは選択 |
| D7 | 画素の操作 | **BitBlt、PatBlt、FillRect、GetDIBits は常に名前付き停止**（ラスタ演算の「効果」は Learn に文で書かれているが、GDI が DIB／DDB に**書くバイト**（32 ビットの未使用バイト、形式変換、パターン）は書かれていない）。RealizePalette も停止（戻り値の「システム パレットに写った数」を導けない）。引数（rop、大きさ、座標）を停止メッセージに出す。**DDB への書き込みを黙って受ける案（読みは止める）は採っていない**（PM の判断の点 3） | 選択（保守的） |
| D8 | 色表 | ≤ 8 ビットの DIB セクションは `pbmi` の色表（`biClrUsed` 個、0 なら 2^bpp 個）を RGBQUAD として持ち、`GetDIBColorTable`／`SetDIBColorTable` は**選択中の DIB セクション**の表を読み書きする。個数は `min(cEntries, 表の個数 - iStart)`（`iStart` が個数以上なら 0）。DIB が選択されていない／> 8 ビットなら 0（失敗値） | Learn（個数の clamp は読み） |
| D9 | ブラシ | `CreateBrushIndirect` は `BS_SOLID`（色の上位バイトが 0）と `BS_HOLLOW`（= `BS_NULL`）だけ。ハッチ・パターン系は停止。作ったブラシは `DeleteObject` できる（Learn）。`lbHatch` は読まない | Learn |
| D10 | 半色調パレット | `CreateHalftonePalette` は**不透明なパレット**（エントリも個数も持たない）。選択と削除はできる。個数・エントリを問うと停止（Learn 2026-08-21 版が「exact color entries are an implementation detail」） | Learn |
| D11 | GetSysColor | **index 8（COLOR_WINDOWTEXT）= `0x00000000`（黒）だけ**。**出典なしの選択**（Learn に色の値は無い。`TCanvas.Draw` が `clWindowText` を `GetSysColor(8)` で変えるので避けられない）。他の index は停止 | 選択 |
| D12 | アイコン／カーソル | `CreateIconIndirect` は、モノクロのマスク（1 プレーン 1 ビット）と、NULL（マスクの高さが偶数）またはマスクと同じ大きさの hbmColor で、どちらも DC に選択されていないとき、新しいハンドル（種類 `FLE_GDI_KIND_ICONCUR = 11`）を返す。表は fIcon、ホットスポット、マスクの大きさ、色ビットマップの有無だけ持つ。**画素はコピーしない**（描画の API は答えていないので観測できない）。`DestroyCursor`／`DestroyIcon` は答えない（D15） | Learn |
| D13 | GetObjectW の拡張 | ビットマップに限り: DDB は c ≥ 32 で 32 バイトの BITMAP（`bmWidthBytes` = `((w * planes * bpp + 15) >> 4) << 1`（`CreateBitmap` のページの式）、`bmBits` = 0（選択））、DIB セクションは c = 32 のとき BITMAP（`bmWidthBytes` = stride、`bmBits` = ゲストが得たポインタ）。DIB セクションで c ≥ 104（DIBSECTION）と、32 より大きく 104 未満の c は停止（ページが挙げるのは sizeof(BITMAP) と sizeof(DIBSECTION) の 2 つ）。pv が 4 バイト境界でなければ 0（Learn）。パレット（c = 4）は決定 0068 のまま停止 | Learn |
| D14 | 応答表 | 19 行を**末尾に追記**（下の「応答表の行」）。最初の 14 が答え、最後の 5 が名前付き停止 | 決定 0015 |
| D15 | 作らないもの | 新しい .c／.h。`DestroyCursor`／`DestroyIcon`（`tests/cursor.rs` と `tests/loadimage.rs` が「行が無い」を固定しており、作成したカーソルと共有カーソルの区別が要る）。`GetPaletteEntries`（`GetObjectW(palette, 4, …)` が先に止まる。決定 0068）。`GetCurrentObject`、`StretchBlt`、`TextOutW` 系、`SetDIBits`、`StretchDIBits`、`CreatePen`／`CreatePenIndirect`（閉包に無い）。メモリ DC を引数にする `CreateCompatibleDC`。`CreateCompatibleBitmap` が DIB を作る形。`GetDeviceCaps` のメモリ DC（既存の「画面 DC でない」停止のまま） | 範囲 |

## 1 呼び出しごとの表: Learn の文言 → 実装の規則

引用は 2026-10-27 に取得したページの原文。「停止」は `fle_stop`（`fle: <API>: <理由>` と abort）。最後のエラーはどの呼び出しも変えない（失敗値を返す経路が少なく、Learn が GetLastError の値を名指すのは `CreateDIBSection` の `ERROR_INVALID_PARAMETER` だけで、その経路は停止にした）。

| API（引数） | Learn の文言 | 実装の規則 | 区分 |
|---|---|---|---|
| `CreateCompatibleDC(hdc)` | 「creates a memory device context (DC) compatible with the specified device」「If this handle is NULL, the function creates a memory DC compatible with the application's current screen」「its display surface is exactly one monochrome pixel wide and one monochrome pixel high」「all attributes are set to normal default values」戻り値は DC のハンドル、失敗は NULL | hdc が NULL または `GetDC(NULL)` の画面 DC: 新しいメモリ DC（D3）。メモリ DC の引数、削除済み、その他は停止（ページは失敗値を定め、失敗する場合を挙げない） | Learn、既定の中身は選択 |
| `DeleteDC(hdc)` | 「deletes the specified device context (DC)」「nonzero」「zero」「An application must not delete a DC whose handle was obtained by calling the GetDC function」 | 生きているメモリ DC: 削除（選択中のものは選択を外すだけ、プレースホルダは DC と共に消える）、1。NULL と削除済みのメモリ DC: 0（無効なハンドル）。画面 DC: 停止（「してはならない」と書くだけで結果は書かない）。その他は停止 | Learn |
| `SelectObject(hdc, h)` | 「selects an object into the specified device context (DC). The new object replaces the previous object of the same type」「Bitmaps can only be selected into memory DC's. A single bitmap cannot be selected into more than one DC at the same time」「the return value is a handle to the object being replaced」「If an error occurs and the selected object is not a region, the return value is NULL」 | メモリ DC にビットマップ・ブラシ・ペン・フォント: 選択して前のものを返す（初回はプレースホルダ／既定）。ビットマップが別の DC に選択中: NULL。画面 DC にビットマップ: NULL。画面 DC にそれ以外、パレット（ページの一覧に無い）、DC、アイコン、未発行・削除済みのハンドル、削除済み DC: 停止 | Learn |
| `SelectPalette(hdc, hPal, bForce)` | 「selects the specified logical palette into a device context」「a handle to the device context's previous logical palette」「(If the hdc parameter is a memory device context, this parameter is ignored.)」「only if device contexts are compatible」 | メモリ DC に生きているパレット（`CreatePalette`、半色調、既定）: 選択して前のものを返す。`bForce` は見ない。画面 DC と、パレットでないハンドルは停止 | Learn |
| `CreateCompatibleBitmap(hdc, cx, cy)` | 「The color format of the bitmap ... matches the color format of the device identified by the hdc parameter」「When a memory device context is created, it initially has a 1-by-1 monochrome bitmap selected into it. If this memory device context is used in CreateCompatibleBitmap, the bitmap that is created is a monochrome bitmap」「If an application sets the nWidth or nHeight parameters to zero, CreateCompatibleBitmap returns the handle to a 1-by-1 pixel, monochrome bitmap」「If a DIB section ... is selected into the device context ..., CreateCompatibleBitmap creates a DIB section」 | 画面 DC: COLOR（仮想画面の 1 プレーン 32 ビット）。メモリ DC: 選択中のビットマップの形式（MONO か COLOR）。cx または cy が 0: 1 × 1 MONO。DIB セクションが選択中、負の大きさ、2 GiB 以上（ページに規則なし）、NULL・削除済み・その他の hdc: 停止 | Learn |
| `CreateDIBSection(hdc, pbmi, usage, ppvBits, hSection, offset)` | 「creates a DIB that applications can write to directly. The function gives you a pointer to the location of the bitmap bit values」「If hSection is NULL, the system allocates memory for the DIB」「If the function fails, the return value is NULL, and *ppvBits is NULL」(ERROR_INVALID_PARAMETER)。BITMAPINFOHEADER: 「stride = ((((biWidth * biBitCount) + 31) & ~31) >> 3)」「biSizeImage ... can be set to 0 for uncompressed RGB bitmaps」「if biHeight is positive, the bitmap is a bottom-up DIB ... negative ... top-down」「If biCompression equals BI_RGB and the bitmap uses 8 bpp or less, the bitmap has a color table ... If biClrUsed is zero, ... 2^biBitCount colors」「BI_BITFIELDS ... Valid for 16-bpp and 32-bpp」「For 16-bpp bitmaps, if biCompression equals BI_RGB, the format is always RGB 555」 | D6、D8。受けるのは: hdc が生きている DC、usage = DIB_RGB_COLORS、hSection = NULL、ppvBits ≠ NULL、biSize = 40、biPlanes = 1、biWidth > 0、biHeight ≠ 0、biBitCount ∈ {1, 4, 8, 16, 24, 32}、BI_RGB（または 16・32 ビットの BI_BITFIELDS でマスクが 555／565／8-8-8）、biSizeImage が 0 か計算値、biClrUsed ≤ 2^bpp、大きさ < 2 GiB。それ以外は停止（失敗値を返さない） | Learn、受ける範囲は選択 |
| `GetDIBColorTable`／`SetDIBColorTable(hdc, iStart, cEntries, prgbq)` | 「retrieves RGB ... color values from a range of entries in the color table of the DIB section bitmap that is currently selected into a specified device context」「the number of color table entries that the function retrieves」「zero」「DIB section bitmaps with a biBitCount value greater than eight do not have a color table」 | D8。prgbq が NULL、削除済み DC、DC でないハンドルは停止 | Learn、clamp は読み |
| `SetTextColor`／`SetBkColor(hdc, color)` | 「a color reference for the previous text color as a COLORREF value」「CLR_INVALID」／「sets the current background color to the specified color value, or to the nearest physical color if the device cannot represent the specified color value」「the previous background color」 | メモリ DC で色の上位バイトが 0: 保存して前の色を返す（初回は D3 の初期色）。仮想の 32 ビット表示ではどの 24 ビット色も表せるので「最も近い色」には変えない。上位バイト ≠ 0（PALETTEINDEX／PALETTERGB）、画面 DC、DC でないハンドルは停止 | Learn、初期色は選択 |
| `CreateBrushIndirect(plb)` | 「creates a logical brush that has the specified style, color, and pattern」LOGBRUSH: 「If lbStyle is the BS_HOLLOW or BS_PATTERN style, lbColor is ignored」「If lbStyle is BS_HATCHED or BS_SOLID, lbColor is a COLORREF color value」「call the DeleteObject function」 | D9。BS_HOLLOW は色を 0 で保存（無視） | Learn |
| `CreateHalftonePalette(hdc)` | 「creates a halftone palette for the specified device context (DC)」「The exact color entries are an implementation detail and are not guaranteed to be identical across devices or Windows versions」 | D10。hdc は生きている DC（画面またはメモリ） | Learn |
| `GetSysColor(nIndex)` | 「retrieves the current color of the specified display element」（要素の名前と番号の表。値なし） | D11 | 選択 |
| `CreateIconIndirect(pii)` | 「The system copies the bitmaps in the ICONINFO structure ... the hbmMask and hbmColor ... should not already be selected into a device context」ICONINFO: 「A value of TRUE specifies an icon; FALSE specifies a cursor」「the hot spot is always in the center of the icon, and this member is ignored」「For monochrome icons, the hbmMask is twice the height of the icon ..., and hbmColor is NULL. Also, in this case the height should be an even multiple of two. For color icons, the hbmMask and hbmColor bitmaps are the same size」 | D12。fIcon は 0／1 に正規化して保存（ホットスポットは icon でも保存） | Learn |
| `DeleteObject(ho)`（拡張） | 「zero if the specified handle is not valid or is currently selected into a DC」「Do not delete a drawing object (pen or brush) while it is still selected into a DC」 | D4。DC は DeleteDC と同じ。ブラシも削除できる。プレースホルダ、既定のオブジェクト、`CreateIconIndirect` のハンドルは停止（ページに規則なし）。DIB セクションはブロックを `free` | Learn |
| `GetObjectW(h, c, pv)`（拡張） | 「HBITMAP: BITMAP」「DIBSECTION, if cbBuffer is set to sizeof (DIBSECTION), or BITMAP, if cbBuffer is set to sizeof (BITMAP)」「the bmBits member ... will contain a pointer to the bitmap's bit values」「If hgdiobj is a handle to a bitmap created by any other means, GetObject returns only the width, height, and color format information」「The address of lpvObject must be on a 4-byte boundary; otherwise, GetObject fails」 | D13 | Learn、DDB の `bmBits` は選択 |
| `BitBlt`／`PatBlt`／`FillRect`／`GetDIBits`／`RealizePalette` | 「If the color formats of the source and destination device contexts do not match, the BitBlt function converts the source color format to match the destination format」（変換は書かない）／「paints the specified rectangle using the brush that is currently selected」／「fills a rectangle by using the specified brush ... excludes the right and bottom borders」／「a color table is synthesized」（色は文で書く）／「the number of entries in the logical palette mapped to the system palette」 | D7: 常に停止。メッセージに rop、大きさ、座標、hbr、`lpvBits` の有無、start、cLines を出す | Learn が書かないものの停止 |

## 応答表の行（`imports_table.rs`。末尾に追記）

追記の前の行数は **312**（HEAD `d3be2a3`。I2 の `CreateBitmap` と R2 の `shfolder.dll!SHGetFolderPathW` が入っている。第 1 段階は 311 と書いた）。以下の 19 行をこの順に末尾（shfolder の行の後ろ。順序は **I2 → R2 → W6**、PM の決定）に足す（行 k の擬似 VA は `0x7ff900000000 + 0x10 * (k + 1)`）。**統合した木では k = 312〜330、擬似 VA は 0x7ff900001390〜0x7ff9000014b0**（算術。第 1 段階の「W6 だけの木」では k = 311〜329、0x7ff900001380〜0x7ff9000014a0）。生成 C の `fle_resp_count` は 312 → **331**、`fle_dispatch_count` は 59,692 → **59,711**（実測）。テストは相対位置だけを固定する:

`gdi32!CreateCompatibleDC`、`gdi32!DeleteDC`、`gdi32!SelectObject`、`gdi32!SelectPalette`、`gdi32!CreateCompatibleBitmap`、`gdi32!CreateDIBSection`、`gdi32!GetDIBColorTable`、`gdi32!SetDIBColorTable`、`gdi32!SetTextColor`、`gdi32!SetBkColor`、`gdi32!CreateBrushIndirect`、`gdi32!CreateHalftonePalette`、`user32!GetSysColor`、`user32!CreateIconIndirect`（ここまで 14 行が答え）、`gdi32!BitBlt`、`gdi32!PatBlt`、`user32!FillRect`、`gdi32!GetDIBits`、`gdi32!RealizePalette`（5 行が名前付き停止）。

## 範囲

- **答える**: メモリ DC の作成と削除、ビットマップ（MONO／COLOR の DDB、DIB セクション）の作成と選択と削除、ブラシ・ペン・フォント・パレットの選択、DIB セクションのポインタとストライドと色表、テキスト色と背景色、半色調パレット（不透明）、`GetSysColor(8)`、アイコン／カーソルのハンドル、`GetObjectW` の BITMAP。
- **名前付きで止める**: BitBlt、PatBlt、FillRect、GetDIBits、RealizePalette。DIBSECTION 全体の `GetObjectW`、パレットの個数・エントリ、`BS_HATCHED` ほかのブラシ、`DIB_PAL_COLORS`、RLE の DIB、`hSection` つきの DIB、`GetSysColor` の 8 以外、画面 DC への描画系の選択。
- **範囲外**: `MoveToEx` 以降の応答（6 モジュール構成の次の壁は実測で `gdi32!MoveToEx`）、`DestroyCursor`／`DestroyIcon`、`GetPaletteEntries`、TextOutW 系・StretchBlt・SetDIBits 系（閉包に無い 82 の gdi32 import）、メモリ DC どうしの互換、`SelectClipRgn` などの領域、ペンの作成、描画モード（`SetBkMode`、`SetROP2` ほか）、`GetDeviceCaps` のメモリ DC、複数スレッド、wasm、実機との照合。

## 撤回条件（仮の決定）

| # | 仮の決定 | 撤回条件 |
|---|---|---|
| P1 | 既定の 4 つのオブジェクトの正体（D3 b） | 一次資料が既定のブラシ・ペン・フォント・パレットを名指したとき。ゲストが既定のオブジェクトの内容（`GetObject`、`GetCurrentObject`）を要するとき（今は `GetObjectW` が停止） |
| P2 | 初期のテキスト色 black、背景色 white（D3 c） | 一次資料や実機の値が示されて食い違ったとき。ゲストが `SetTextColor`／`SetBkColor` の戻り値を使うと分かったとき（`TCanvas.Draw` は使わない（読み）） |
| P3 | `GetSysColor(8)` = 黒（D11） | 一次資料が値を与えたとき。他の index が要るとき（停止が知らせる）。テーマ（高コントラストなど）を考えるとき |
| P4 | DIB セクションのゼロ埋め（D6） | 一次資料が初期値を与えたとき。ゲストが初期値に依存すると分かったとき |
| P5 | 画素の操作を止める（D7） | PM が DIB 32 ビットの未使用バイトの値、DDB の書き込みの受け方、BitBlt の SRCCOPY の範囲などを決めたとき（「PM の判断が要る点」の 2、3） |
| P6 | 半色調パレットを不透明にする（D10） | 個数・エントリを問う経路が来たとき（停止が知らせる）。Learn の旧版が別の記述を持つと分かったとき |
| P7 | `GetObjectW` の DDB の `bmBits` = 0（D13） | 一次資料が値を与えたとき |
| P8 | `CreateIconIndirect` が画素を持たない（D12） | アイコンの描画・複製・`GetIconInfo`・`SetCursor` を答えるとき |
| P9 | 無効なハンドルを `SelectObject` に渡したら停止（NULL を返さない） | 一次資料がどのハンドルが「error」かを定めたとき |
| P10 | 応答表の行の順序 | 統合の都合で並べ直す必要が出たとき（擬似 VA が動く。決定 0015） |

## 実装した場所

差分は **`resolved-W6.diff`**（HEAD `d3be2a3` の `lift/` の clean な `git archive` に `patch -p1` で当たる。7 ファイル、2,605 行。ずれ・拒否・fuzz なし。当てた木はテストした木と `diff -r` で一致）。第 1 段階の `w6-gdi.diff` との違いは、`imports_table.rs` の追記位置が shfolder の行の後ろであること、`vcall_thunk_dll.rs` の期待値が 513 → 494 であること、`harness/a4-multi.sh` の既定 `EXPECT_FLE_SHA` の更新だけ:

- `lift/runtime/fle_gdi.c`（+902 −16。構造体 `gdi_dc`、`gdi_dib`、`gdi_obj` の拡張、補助関数、19 の `fle_imp_*`、`DeleteObject` と `GetObjectW` の拡張）、`fle_gdi.h`（+102。種類 8〜11、形式、定数、宣言、検査関数）。**新しい .c／.h は作っていない**。`slot_get` の新しい枠の初期化を「状態と種類だけ」から全体のゼロ化に変えた（挙動は同じ）。
- `lift/crates/fe-lift/src/imports_table.rs`（+21。末尾に 19 行とコメント）。
- `lift/crates/fe-lift/tests/gdi.rs`（+1,392 −3。節 12 を末尾に追記し、「行が無い」のリストから 7 つの名前を除いた）、`tests/gdc.rs`（+2 −1。リストから 2 つの名前を除いた）、`tests/vcall_thunk_dll.rs`（+7 −3。513 → 494（R2 込み））。
- 行数の注記: 第 1 段階の文書の「+815／+95／+1,330」は `w6-gdi.diff` の実数（`fle_gdi.c` +902、`fle_gdi.h` +102、`tests/gdi.rs` +1,392）と合わない（第 1 段階の数え違いと読む。差分の中身は変えていない）。上は `resolved-W6.diff` を数えた値（7 ファイル、+2,427 −24）。
- `lift/harness/a4-multi.sh`（既定の `EXPECT_FLE_SHA`: `794d26cb…` → `94fe0c853537cefbcbc674b65e77c79464b8659a2c250b46dc762f6bf2ddc6b6`。FLEngine の C の**ファイル**の SHA-256。`fe-lift lift DLL OUT.c --pdata` が書いた OUT.c の値で、`/dev/stdout` の流れの値ではない）。

## テスト（実行した。`tests/gdi.rs` は 96 件 = 既存 55 + 新規 41 件が通った）

`tests/gdi.rs` の節 12（新規 **41 件**）。期待値は決定と Learn から書いた（C のコードを読み返していない）。C の断片 147 個（本文 23、停止のケース 124）は `clang -fsyntax-only -Wall` を通した。 **第 2 段階: 新規 41 件はすべて初回の実行で通った（期待値を C の出力に合わせて直したテストは 0）。** 全体の結果は上の「全体テスト」。

| 区分 | 件数 | 検査すること |
|---|--:|---|
| 応答表・定数 | 2 | 19 行が古い行の後ろに決めた順で 1 つずつあり、`fle_gdi.h` に宣言がある。DestroyCursor ほか「答えない隣」に行が無い。種類・形式・構造体の大きさ（算術）・定数 |
| DC | 6 | 初期状態（プレースホルダ、既定 4、色、スロット 33〜40、生存数）。NULL／画面 DC から作れる、メモリ DC・削除済み・その他は停止。DeleteDC（選択を外す、プレースホルダが消える、2 回目は 0）。DeleteObject(hdc)。DeleteDC の停止。メモリ DC は ReleaseDC／GetDeviceCaps で受けられない |
| 選択 | 9 | 前のビットマップを返す、別の DC に選択中なら NULL、同じものを選び直すと自分を返す。ブラシ・ペン・フォントの前の値。画面 DC へのビットマップは NULL。選択中の削除は 0、外せば 1。SelectObject の停止（11 ケース）。SelectPalette（前の値、bForce 無視）とその停止（7 ケース）。プレースホルダ・既定・作成したアイコンの DeleteObject は停止。DC・ブラシ・既定の GetObjectW は停止 |
| ビットマップ | 8 | CreateCompatibleBitmap の形式（画面 → COLOR、メモリ DC → 選択中の形式、0 → 1 × 1 MONO、下位 DWORD だけ、2 GiB の境界）と停止 7 ケース。DIB セクション（ポインタ、ゼロ、書ける、ストライド 6 通り、BITFIELDS、biSizeImage、メモリ DC から）。色表の保存。DeleteObject（選択中は 0、`free`）。GetObjectW の BITMAP（DIB、DDB、4 バイト境界、DIBSECTION は停止）。CreateDIBSection の停止 32 ケース |
| 色表 | 2 | Get／Set の clamp（個数、iStart、下位 DWORD）。DIB が選択されていない・> 8 ビットは 0。NULL・削除済み DC・prgbq NULL の停止 |
| 色・ブラシ・パレット | 6 | テキスト色・背景色（初期値、前の値、DC ごと、下位 DWORD、上位バイトの停止）。GetSysColor（8 だけ。他の 13 index は停止）。ブラシ（スタイル、色、BS_HOLLOW は色を無視、削除、停止 10 ケース）。半色調パレット（不透明、選択、削除、GetObjectW は停止） |
| アイコン | 2 | 作成（cursor／icon／モノクロ icon／カラー DDB）、元のビットマップはそのまま。停止 12 ケース |
| 生存期間 | 1 | 削除した DC・ビットマップ・DIB セクション・ブラシのハンドルは再利用されず、無効のまま（削除済みのブラシ・ビットマップ・フォントを `SelectObject` に渡すと停止） |
| 画素の停止 | 1 | BitBlt／PatBlt／FillRect／GetDIBits（bits あり・なし）／RealizePalette の停止メッセージ（rop、大きさ、座標） |
| スレッド・生成器・ソース | 4 | 18 の API が第 2 スレッドで停止し、GetSysColor は止まらない。19 の thunk が runtime を呼び、DLL 名が一致の一部（他の DLL と相手の DLL で外れる）。隣の 6 つは未対応の thunk のまま。5 つの停止関数がピクセルを触らない、`calloc` が 1 か所、`fle_gdc_is_live` を使う |
| 計 | **41** | |

DDB のメモリ DC への書き込みを受ける案は採っていないので、それを検査するテストは無い。

### 変異検査の案（36 件。7 件を実行した。結果は上の「変異検査」）

「落ちるテスト」は、その変異で**少なくとも落ちるはずの**テスト（第 1 段階の読み。M1、M3、M4、M8、M14、M21、M28 は第 2 段階で実行し、結果は上の表。ほかは未実行）:

| # | 変異（ファイル） | 落ちるはずのテスト |
|--:|---|---|
| M1 | `DeleteObject` が `sel` を見ない（`fle_gdi.c`） | `delete_object_of_a_selected_object…`、`halftone`、`delete_object_of_a_dib_section…` |
| M2 | `SelectObject` が前のオブジェクトの `sel_drop` を省く | `select_object_returns_the_previous_bitmap…`（counts）、`…selects_brushes_pens_and_fonts…`、`delete_object_of_a_selected…` |
| M3 | ビットマップの二重選択を許す（`o->sel && dc->bmp != h` を外す） | `select_object_returns_the_previous_bitmap…`（「selected in another DC: 0」） |
| M4 | `SelectObject` が前のものでなく新しいものを返す | `select_object_*` ほぼ全部 |
| M5 | `DeleteDC` が選択中のオブジェクトも削除する | `delete_dc_unselects…`（bitmap kind 7） |
| M6 | `DeleteDC` がプレースホルダを消さない | `delete_dc_unselects…`（placeholder kind、live 4） |
| M7 | 新しい DC のプレースホルダの `sel` を 0 にする | `a_memory_dc_starts_with…` |
| M8 | 既定の色を入れ替える（テキスト白、背景黒） | `a_memory_dc_starts_with…`、`the_text_and_background_colours…`、`the_constants_of_decision_0108` |
| M9 | `SetTextColor` が新しい色を返す | `the_text_and_background_colours…` |
| M10 | `SetBkColor` が上位バイトを受ける | `a_colour_with_a_high_byte…` |
| M11 | 画面 DC の `CreateCompatibleBitmap` が MONO を作る | `a_compatible_bitmap_follows…` |
| M12 | プレースホルダ選択中のメモリ DC からの `CreateCompatibleBitmap` が COLOR を作る | 同上 |
| M13 | 0 × 0 の `CreateCompatibleBitmap` が 1 × 1 にしない | 同上（zero width／height） |
| M14 | DIB のストライドの DWORD 切り上げを外す | `a_dib_section_gets_its_bits…`、`…colour_table_of_its_bitmapinfo`、`get_object_of_a_dib_section…` |
| M15 | DIB を `malloc` にする（ゼロ埋めをやめる） | `the_source_of_decision_0108…`（`calloc` が 1 つ）、`a_dib_section_gets_its_bits…`（運次第） |
| M16 | `biClrUsed` = 0 のときの色数を 0 にする | `…colour_table_of_its_bitmapinfo`（4 ビット 16、8 ビット 256） |
| M17 | `CreateDIBSection` が `usage` = 1 を受ける | `create_dib_section_forms_…`（usage-pal-colors） |
| M18 | RLE8 を受ける | 同上（rle8） |
| M19 | Get／SetDIBColorTable の個数を clamp しない | `the_colour_table_of_the_dib_section…` |
| M20 | 24 ビット DIB でも色表を返す | `without_a_dib_section_of_8_bits_or_less…` |
| M21 | `BitBlt` が停止せず 1 を返す | `bitblt_patblt_…`、`the_source_of_decision_0108…`（`fle_stop(api` が無い） |
| M22 | `FillRect` が DIB に書く（`fle_st` を足す） | `the_source_of_decision_0108…` |
| M23 | `CreateBrushIndirect` が `BS_HATCHED` を受ける | `a_brush_style_the_guest_does_not_use…` |
| M24 | BS_HOLLOW の色を保存する | `a_solid_or_hollow_brush…`（info 1 1 00000000） |
| M25 | `CreateIconIndirect` が選択中のマスクを受ける | `create_icon_indirect_of_what…`（mask-selected） |
| M26 | `GetObjectW(DIB, 104)` が答える | `get_object_of_a_dib_section…`（dibsection／bigger） |
| M27 | 4 バイト境界でない pv にも書く | `get_object_of_a_device_dependent_bitmap…` |
| M28 | `GetSysColor(8)` が白 | `only_the_window_text_system_colour…`、`the_constants…` |
| M29 | 応答表の 2 行を入れ替える／古い行の途中に挿入する | `the_nineteen_rows_are_appended_in_order…` |
| M30 | 新しい API の 1 つで `fle_gdi_check_thread` を省く | `a_second_thread_stops_the_new_calls_too…` |
| M31 | `dc_destroy` が既定オブジェクトの `sel_drop` を省く | `delete_dc_unselects…`（default brush 0） |
| M32 | 既定のオブジェクトを DC ごとに作る | `a_memory_dc_starts_with…`（defaults shared、slots） |
| M33 | `DeleteObject` がプレースホルダの削除を許す | `delete_object_of_the_placeholder…` |
| M34 | `DeleteObject(hdc)` が DC を扱わない | `delete_object_on_a_memory_dc…` |
| M35 | Rust 側: `ROWS_0108` の 1 行を応答表から消す／DLL 名を変える | `the_nineteen_rows…`、`the_thunks_of_the_nineteen_apis…` |
| M36 | 新しい DC・ビットマップ・ブラシが削除済みのスロットを再利用する（`new_slot` が空きを探す） | `the_handles_of_deleted_dcs_bitmaps_sections_and_brushes_are_never_reused`、`a_memory_dc_starts_with…`（スロット） |

## 固定数テストの影響（実測した。旧 → 新と根拠）

| 場所 | HEAD `d3be2a3` | W6 統合（実測） | 式と根拠 |
|---|--:|--:|---|
| `tests/vcall_thunk_dll.rs`（`after_the_memory_form_jmp_only_the_import_thunks_remain_unsupported`）`unsupported_total` と `unsupported["Jmp: indirect through IAT slot (not lifted)"]`（2 か所） | 513 | **494** | 513 − 19。FLEngine の C を作った統計行が `unsupported_total 494`、`jmp_imp:<API> 1` が 19 行、テスト通過（実測）。第 1 段階の 495 は R2 を知らない値（514 − 19）。コメントの列に W6 の 19 のアドレスを足した |
| 応答表の行数（`resp_count()`、生成 C の `fle_resp_count`）／`fle_dispatch_count` | 312／59,692 | **331／59,711** | + 19（生成 C の表。実測）。**どのテストも行数を固定していない** |
| 応答表の擬似 VA | 行 k の VA = `0x7ff900000000 + 0x10 * (k + 1)` | 新しい行は k = 312〜330 → `0x7ff900001390`〜`0x7ff9000014b0` | 既存の行は動かない（末尾追記）。算術（個別には実測していない） |
| `tests/gdi.rs` の `ROWS`（7 行）と `rows_are_in_the_table_and_declared_in_the_header` | 7 | 7（不変） | 新しい 19 行は別の定数 `ROWS_0108`。同じテストの「行が無い」リストから 7 つの名前を除いた（第 1 段階の編集。通過） |
| `tests/gdc.rs` の「行が無い」リスト | 6 名 | 4 名 | CreateCompatibleDC、SelectObject を除いた（通過） |
| `tests/gdi.rs` の `getobjectw_of_a_bitmap_stops_with_a_reason_of_its_own` | 停止（c = 24） | 停止（不変） | 通過 |
| `tests/cursor.rs`／`tests/loadimage.rs` の「DestroyCursor に行が無い」 | 不変 | 不変 | DestroyCursor に行を足さない（通過） |
| `harness/a4-multi.sh` の既定 `EXPECT_FLE_SHA`（FLEngine の C の**ファイル**の SHA-256、註釈なし） | `794d26cb38f6aad5c92390f8fe1bcaa740d7c57f28840382cd4d56898cb0bcdd` | **`94fe0c853537cefbcbc674b65e77c79464b8659a2c250b46dc762f6bf2ddc6b6`** | 統合した木の `fe-lift lift FLE OUT.c --pdata` が書いたファイルの値（実測）。註釈あり（`FLE_HOOKS`）の値は上の表。a4-multi.sh は註釈なしの値を既定とする |
| 生成 C の大きさ・`unsupported_total`（FLEngine） | 215,747,616 バイト、513 | **215,751,676 バイト、494** | 実測 |
| 他のモジュールの生成 C | — | dsp_ippv2、Blend2D、freetype は 0 関数、QFC は 18 関数、ILTools は 19 関数の本体が変わる | 実測（上の「生成 C の確認」） |


## 検証（実施したこと。実施していないこと）

- **実施した（第 1 段階）**: (1) `clang -fsyntax-only … -Werror` の構文検査。(2) テストの C 断片 147 個の構文検査。(3) `rustfmt` による Rust の構文解析。(4) `tests/gdi.rs` の節 7 の禁止語の走査の Python での再現。(5) `patch -p1` の適用の確認。(6) 読み: ILTools の逆アセンブルと閉包の計算、FLEngine の thunk の走査。
- **実施した（第 2 段階。実測）**: (7) 統合（拒否 2 か所を手で）。(8) `cargo build`（警告 0）、`fle_gdi.c` の `clang -O0／-O1／-O2 -Wall -Wextra -Werror`、`gcc -O1 -Wall -Wextra`（警告 0）。(9) 全体テスト 112 test result・1,469 passed・0 failed（グループ分け。ENOSPC・quota 0 件）。(10) skip の確認（13 binary を `--nocapture` で再実行、`deferred` の 2 件を DLL を置いて再実行）。(11) 変異検査 7 件（7 件検出）。(12) FLEngine の生成 C のファイル SHA、統計、関数単位の比較（変わった関数 19 = W6 の thunk だけ）。他の 5 モジュールの関数単位の比較。(13) 6 モジュール構成の a4（HEAD と W6、W6 は独立に 2 回）と gdb（W6 の API の呼び出しの記録、停止点の C スタック）。(14) census（6 モジュール構成と dsp 構成、HEAD と W6）。
- **実施していない**: 変異検査の残り 29 件、wasm（`emcc`）のビルド（使わない方針）、`cargo test --workspace` の 1 回の通し、実機 Windows との照合（0 件）、ILTools のローダー呼び出し（`LoadLibrary`／`GetProcAddress`）の trace の取り直し（停止点が +5 function entries しか進まないので 0106 の記録から変わらないと読むが、W6 の測定では取っていない）、`MoveToEx` の Learn のページ、`MoveToEx` 以降。
- **既知の弱点**: 全体テストは 1 回の通しではなく、グループ分けの合算（前回の統合と同じ）。`deferred` の 2 件は既定のパスに DLL が無いと skip で通る（通ったことにはなるが何も検査しない）。W6 の API は 6 モジュール構成では `CreateCompatibleDC` の 1 回しか呼ばれていないので、他の 13 の応答（SelectObject、SetTextColor ほか）は**単体テストだけで検証されていて、連鎖の中では未使用**。


## 解釈が分かれた点

1. **既定の 4 つのオブジェクトをストックと同一視しない**（D3）。Learn は「set of default objects」としか書かない。ストック（WHITE_BRUSH、BLACK_PEN、SYSTEM_FONT、DEFAULT_PALETTE）と同一視すれば実装は小さいが、`GetStockObject(0)` を答えないままその値を見せることになる。4 つの隠しオブジェクトにした（コストは 4 スロット）。
2. **DDB の書き込みを受ける案を採らなかった**（D7）。DDB に画素を持たないので、書き込みを黙って捨てても読みが全部止まるなら観測されない、という案がある。検討の結果、BitBlt／PatBlt／FillRect を常に止めた: (a) 書き込み先が DIB セクションなら観測される（ゲストが直接読む）、(b) 読み側（`SRCCOPY` の元）が DDB のときは元を読むので止まる、(c) 実装が小さい。この案は PM の判断の点 3。
3. **32 ビット DIB の未使用バイト**。BITMAPINFOHEADER のページは 32 ビットの DIB を「blue、green、red」と書き、未使用バイトに触れない（`BitBlt`／`FillRect` が何を書くか不明）。D7 で止める根拠。
4. **`SetBkColor` の「nearest physical color」**。仮想表示は 32 ビットなのでどの色も表せると読み、保存した色をそのまま返す。メモリ DC の 1 × 1 モノクロのプレースホルダが選択されている間に「表せない」とする読みもありうる（未確認）。
5. **`DeleteDC` が選択中のオブジェクトを削除しない**（D4）。ページは書かない。`SelectObject` のページの助言（元のオブジェクトを選び直す）からの読み。
6. **メモリ DC のプレースホルダを別の DC に選ぶ**。プレースホルダは作った DC に選択中なら二重選択で NULL。その DC から外れていれば（別のビットマップが選ばれている間）他の DC に選べてしまう。ページが書かない角で、テストしていない。
7. **`GetDIBColorTable` の失敗値**。DIB が選択されていない／> 8 ビットのとき、ページは「must be selected」「do not have a color table」と書き、失敗値は 0。これを 0 の返しにした（停止にする案もあった）。
8. **`CreateDIBSection` の `hdc`**。ページは `DIB_PAL_COLORS` のときだけ使うと書く。`DIB_RGB_COLORS` で NULL や無効な hdc が通るかは書かない。生きている DC を要求した（停止）。
9. **書式**。依頼文は「`fle: gdi32!<API>: <理由>` の形」と「既存の `fle_gdi.c` の停止の書式に合わせる」の両方を述べる。既存は `fle: <API>: <理由>`（`gdi32!` なし。`tests/gdi.rs` がこの形を固定している）なので、**既存に合わせた**。
10. **統合の順序**（PM の決定）: 応答表の末尾は I2（CreateBitmap）→ R2（shfolder）→ W6（19 行）。W6 の 19 行の擬似 VA は R2 の行の分だけ第 1 段階の記述から 1 つずれる（0x…1380 から 0x…1390 から）。
11. **`EXPECT_FLE_SHA` の値**: 既定は註釈なしの FLEngine の C の**ファイル**の SHA（`94fe0c85…`）。a4 の測定は `FLE_HOOKS` つき（`ce205cfe…`）なので、`EXPECT_FLE_SHA` を註釈ありの値で渡して測った（`skip` ではない）。
12. **6 モジュール構成の census の扱い**: 打ち切られた census の数（ok 227 など）をそのまま「census の結果」と書くか、診断として使えないと書くかは解釈が分かれる。上は数と終わり方を並べ、使えないと書いた（原因は推測）。

## 未決の点(PM)

1. **次の壁 `gdi32!MoveToEx` を答えるか**。ILTools の `TBitmapCanvas.CreateHandle`（`f_61374a0`）は `CreateCompatibleDC(NULL)` の直後に `MoveToEx(hdc, 0, 0, NULL)` を呼ぶ（実測）。答えるには DC に現在の位置を持たせる必要がある（Learn のページは読んでいない。lppt が非 NULL のときの前の位置の返し方などが論点）。同じ種類の「閉包に無い 82 の gdi32 import」（SaveDC、SetBkMode、LineTo、Rectangle ほか）が続く可能性が高い（読み。測っていない）。今回の連鎖で W6 の他の 13 の応答（`SelectObject`、`GetSysColor(8)`、`SetBkColor` ほか）は一度も呼ばれていない。
2. **0 × 0 の矩形（決定 0106 の解釈の点 1）を先に直すか**。PM の決定 5 で今回は直していない。0 × 0 のまま、`CreateBitmap(0, 0)` → 1 × 1 の連鎖で進んでいる（実測）。freetype の switch とフォントを置いた後は、`TBitmap` が非 0 × 0 になり `CreateDIBSection` などの経路に入る可能性がある（読み、未測定）。
3. **DIB セクションへの画素の書き込み**（FillRect、PatBlt、BitBlt の SRCCOPY）。PM の決定 1 で名前付き停止のまま（到達していない）。許可するなら 32 ビットの未使用バイトの値と対応形式の範囲を選ぶ。`CreateCursorFromFont` の `TBitmap` は pf32bit（読み）。
4. **DDB の書き込みを受けて読みを止める案**（解釈の点 2）。PM の決定 1 で採らない。
5. **出典なしの選択の確認**（PM の決定 3 で仮のまま採用）: 初期色（黒と白）、`GetSysColor(8)` = 黒、DIB のゼロ埋め、既定オブジェクトの共有、DDB の `bmBits` = 0。**連鎖では一度も観測されていない**ので、実機との照合や一次資料の探索は、これらが観測されるまで急がなくてよい（読み）。
6. **`DestroyCursor`／`DestroyIcon`**。PM の決定 2 で到達するまで足さない。今回は到達していない（ILTools の `f_62953c0` が呼ぶ。閉包の深さ 4）。
7. **`GetObjectW(DIBSECTION)`**。PM の決定 2 で停止のまま。到達していない。
8. **6 モジュール構成の census を使える形にするか**。unit 245 の放棄が後続の unit の検出を外し、census が HEAD でも W6 でも 229〜266 unit で打ち切られる（実測。原因は推測）。選択肢: (a) 6 モジュール構成の census は取らず、dsp 構成（ok 890／stop 5）を診断に使う、(b) census の道具が、放棄の後にゲストの初期化の大域の状態を戻す（`InitContext` の保存と復元）ようにする、(c) census では ILTools の `LoadLibraryW` を拒否する構成（dsp 構成）に固定する。どれもこの作業ではしていない。
9. **`EXPECT_FLE_SHA` をコミット単位でどうするか**。I2、R2、W6 を別々にコミットするなら、各コミットで C のファイルの SHA が変わる（I2 のみ `a5832f3b…`、I2 + R2 は `794d26cb…`、I2 + R2 + W6 は `94fe0c85…`）。
10. **統合の順序**（PM の決定 4）は実施した。I2 → R2 → W6。


## 限界

- **連鎖での検証は `CreateCompatibleDC` の 1 回だけ**: 6 モジュール構成は `MoveToEx` で止まるので、W6 の他の 13 の応答と 5 の名前付き停止は、連鎖の中では一度も実行されていない（実測）。それらは単体テスト（`tests/gdi.rs` の 96 件）と、変異検査 7 件（7 件検出）でだけ検証された。変異検査の残りは未実行。
- 閉包は**直接 call だけ**（下限）。実測の停止（`MoveToEx`）は、閉包で届かない vtable 経由の呼び出しの先だった。`graphic.Draw` の先（ILTools の `TGraphic` の子クラス、QFC への呼び出し）、`TBitmap` の virtual メソッドの先は読んでいない。82 の gdi32 import（TextOutW 系、StretchBlt、SetDIBits、StretchDIBits など）が続くかは測っていない。
- **モデルは DC の状態と DIB のブロックだけ**: 画素の読み書きは一切しない。`CreateIconIndirect` は画素をコピーしない。DIB セクションのブロックはゲストだけが書く。DC の現在位置（`MoveToEx`）は持たない。
- **実機 Windows との照合は 0 件**。Learn の 2026-08-21 版（半色調パレット）と 2024-02-22 版（他）が実機の挙動と一致するかは確認していない。
- 出典なしの選択（既定オブジェクト、初期色、`GetSysColor(8)`、ゼロ埋め、DDB の `bmBits`）は、連鎖で観測されていない。観測される経路（`SetTextColor`／`SetBkColor` の戻り値）は読みでは `TCanvas.Draw` が使わないが、他の経路の有無は未確認。
- FLEngine の `unsupported` の走査（523）と、テストの値（514／513）の 9 の差は説明できていない（予測 494 は当たった）。
- 6 モジュール構成の census は打ち切られ、診断として使えない。原因は推測のまま。
- wasm は未確認（`emcc` を使わない方針）。`calloc`／`free` と `fle_host_size_fits` は既存の `HeapAlloc` と同じ作りで、`(uint64_t)(size_t)p` の変換は wasm32 でも幅が足りる（読み）。
- 全体テストは 1 回の通しではなく、グループ分けの合算。


## 測定の道具と成果物

- `lift/out-git/w6s2-results/`（第 2 段階）: `resolved-W6.diff`、`docs/0108-gdi-memory-dc.md`（本書）、`docs/README-0108-line.txt`、`MERGE-NOTES.md`、`logs/`（`tests/` = グループごとの全文のログ・要約・skip の確認・変異検査、`gen/` = 生成 C の統計・関数単位の差・19 の thunk の本体・モジュールごとの比較、`a4/` = HEAD と W6（2 回）と dsp 構成の a4 の記録、`gdb/` = W6 の API の呼び出しの記録と gdb スクリプト、`census/` = 4 つの census の生データ・要約・壁の TSV、`tools/` = 使ったスクリプトとラッパー）。
- `lift/out-git/w6-results/`（第 1 段階）: `w6-gdi.diff`、`w6-import-closure.tsv`（thunk の VA から API 名を引く表。停止の API 名の同定に使った）、`tools/`、`learn/`。
- RAM 上（`/tmp/opencode/w6s2/`）: `logs/`、`final/`、`gen/`、`census/`、`gdb/`、`mut/`（変異した C）、`bin/`（スクリプト）。ビルド出力（`o6*`）は削除した。`/tmp/opencode/m2/src/` は W6 を当てた木のまま、`/tmp/opencode/m2/target` は dev ビルドのキャッシュ。
