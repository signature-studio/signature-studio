# LoadImageW の呼び出し元の調査と、仮想 LoadImageW の実装後の harness の再実行の記録（決定 0061）

実行日: 2026-10-22。作業ディレクトリ `signature-studio-develop`、HEAD aa215c5。DLL は `binaries/win64/FLEngine_x64.dll`。決定は `docs/decisions/0061-virtual-load-image.md`。「実測」はコマンドの出力、「読み」は `lift/objdump-text.txt` の逆アセンブルの静的な読み、「推測」は未検証。成果物は `lift/out-img/`（gitignore 済み。`TMPDIR=lift/out-img/tmp`、`EM_CACHE=lift/out-img/emcache`、`CARGO_TARGET_DIR=lift/out-img/target`）。補助スクリプトは `lift/out-img/work/`（`rsrc_cursor.py`、`rsrc_group.py`、`callers.py`、`dis_range.py`、`mkmut.py`、`runmut.sh`）。

`loadcursor-caller.md` と重複する事実は参照だけにする: 停止点の確認（§1）、LoadCursorW の呼び出し点（§2）、戻り値の使われ方（§3）、関連 API の呼び出し点の数と LoadImageW の 21 個の識別子（§4、§8 の呼び出しスタック）。以下は新しい事実。

## 1. 一次資料（Microsoft Learn。2026-10-22 取得）

引用は決定 0061 の表にある。要点と、**書かれていないこと**:

- 引数（hInst、name、type、cx、cy、fuLoad）の定義はページにある。hInst は「To load a predefined image or a standalone resource ... set this parameter to NULL」。hInst NULL かつ LR_LOADFROMFILE なしで LR_SHARED を含むとき name は定義済みの画像。IDC_ 接頭辞は「Standard cursors」。
- **IMAGE_CURSOR の数値は LoadImageW のページに無い**（表は名前と意味だけ）。数値は CopyImage ページの表（IMAGE_BITMAP 0、IMAGE_ICON 1、IMAGE_CURSOR 2）で一次に確認した。ゲストの `r8d = 2` と一致。mingw-w64 の `winuser.h` も同じ（照合のみ）。
- LR_SHARED は 0x00008000。「Shares the image handle if the image is loaded multiple times. If LR_SHARED is not set, a second call to LoadImage for the same resource will load the image again and return a different handle.」「When loading a system icon or cursor, you must use LR_SHARED or the function will fail to load the resource.」「Do not use LR_SHARED for images that have non-standard sizes」「This function finds the first image in the cache with the requested resource name, regardless of the size requested.」
- 戻り値は成功でハンドル、失敗で NULL、「To get extended error information, call GetLastError」。**GetLastError の値はページに無い。**
- 「the function returns the handle of the already loaded resource」に相当する文は**無い**。同一性は上の「Shares the image handle」から読む。
- ページは SM_CXICON と SM_CXCURSOR のどちらが 0 のときのサイズかを書く（LR_DEFAULTSIZE の説明）。cx = 32 が標準サイズかは書かない。About Cursors は 32×32 を「< 150% の DPI の SM_CXCURSOR」と書く。

## 2. モジュール側 7 回: DLL の資源（実測）

`rsrc_cursor.py`（pefile）と `rsrc_group.py`（RT_GROUP_CURSOR の RESDIR を読む）、テスト `the_dll_has_the_rt_group_cursors_32761_to_32767_and_not_32649`（fe-pe）で再確認した。

- RT_GROUP_CURSOR（型 12）は 57 件。数値の名前は **7 件だけ: 32761、32762、32763、32764、32765、32766、32767**。残りの 50 件は文字列名（ADDPOINT、VT_HEADERSPLIT など）。**32649 はモジュール側に存在しない。** 標準の識別子（32512 など）もモジュール側に無い。
- RT_CURSOR（型 1）は 60 件（id 1〜60）。7 個の数値グループは各 1 コンポーネントで、すべて RT_CURSOR に実在する（32761 → 1、32762 → 2、32763 → 59、32764 → 60、32765 → 5、32766 → 6、32767 → 7）。形は 32×64（AND と XOR の 2 面）の 1bpp、308 バイト。ホットスポットは 32761 が (7, 1)、32762 が (0, 0)、32763 が (0, 0)、32764 が (14, 12)、32765 が (16, 14)、32766 が (0, 0)、32767 が (2, 2)。NEWHEADER は (0, 2, 1)。言語は 1033。
- ゲストが hInst = モジュールで読む 7 つ: `esi = -17 … -12` の 32762、32763、32764、32765、32767、32766（表の順）と `esi = -21` の 32649。32761 はゲストの表に無い。
- 実 Windows が 32649 を hInst = モジュールで読むと、資源が無いので失敗して NULL のはずだが、**Learn に失敗時の GetLastError の値は書かれていない**。よってその形は名前付き停止にした（決定 0061）。

## 3. 戻り値（HCURSOR）の使われ方と NULL の扱い（読み）

- 21 回のループ（0x865a19〜0x865a71）: `r8 = rax`（LoadImageW の結果）を**検査せずに** `f_865c50(obj, esi, cursor)` に渡す（0x865a5f〜0x865a62）。f_865c50 は 20 バイトのノード（next、idx = esi、cursor）を `[obj+0x14c]` の先頭に追加する（0x865c64 の確保、0x865c70〜0x865c7a）。**NULL でも追加される。** idx は -22 … -2。
- 読み出し: `f_866060(obj, idx)`（idx が -1 なら 0、リストに idx があればその cursor、なければ `[obj+0x154]`）。結果は 33 か所で SetCursor に渡る（`loadcursor-caller.md` §4）。**NULL の cursor を持つノードがあれば、f_866060 はその NULL を返す**（`[obj+0x154]` へのフォールバックは「ノードが無い」ときだけ。0x86607d〜0x86608b）。→ idx = -21 の 32649 が NULL なら、idx = -21 を引く呼び出し（0xd890d1〜0xd890e8 の `edx = 0xffffffeb` の `f_866060` → `SetCursor`）は `SetCursor(NULL)` になる。Learn は「If this parameter is NULL, the cursor is removed from the screen」と書く。**ゲストは NULL を許容するが、見た目の挙動が変わる**（カーソルが消える）。NULL を黙って返さない理由。
- 破棄側は NULL を検査する: `f_865a80`（0x865abb: `cmp [rsi+0xc], 0; je`）、`f_865b20`（0x865b56）、いずれも `[node+0xc] != 0` のときだけ DestroyCursor。0x814f2c のループは検査なしに DestroyCursor を呼ぶ。要素は `f_50aa70(*[0x15f6f70], i)` の結果で、このノードリストかは**未確認**。
- 破棄側の条件: `f_865a80` は idx が -17〜-12、-21、または正で、`f_8661d0(obj, idx)`（`[obj+0x15c]` の表に idx があるか）が偽のときだけ DestroyCursor する。つまり**モジュール側の 7 つ（-21 と -17〜-12）は破棄の対象**で、標準カーソル側（-22 … -18、-11 … -2）は破棄しない。About Cursors と LoadImage の LR_SHARED（「the system will destroy the resource when it is no longer needed」）に照らすと、LR_SHARED で読んだカーソルを DestroyCursor に渡すのは仕様の勧めに反するが、ゲストはこの形を取る。DestroyCursor は今は答えない（4 か所の呼び出し点。次の課題）。
- ほかの流れ: `0xa00c9d`（`f_866060(obj, [..+0xd8])` → `[rbp+0xb0]`）、0xa00d15 の仮想呼び出し（`[rbx+0x4a0]`）、0xa00d27 の `f_866060(obj, 0)` との比較。結果が SetCursor 以外に渡る先は仮想呼び出しの先（**未追跡**）。

## 4. 実装の要点（実測）

- `lift/runtime/fle_loadimage.h`、`fle_loadimage.c`（新規）: 引数は rcx、rdx、r8、r9、`[rsp+0x28]`（cy）、`[rsp+0x30]`（fuLoad）。`fle_arg(5)`、`fle_arg(6)`（`fle_env.h`）が読む。作成は `lift/out-img/work/` で、clang `-Wall -Wextra -Werror` と emcc `-Wall -Wextra -Werror` の両方で警告なしを確認してから、ヘッダ、`.c` の順に `lift/runtime/` に置いた。置いた後に 2 回編集した（後述）。`diff`（`cmp` ではなく）で work と一致を確認した。
- 置いた後の編集: ① ハンドル領域の基底を 0x7FCB0000 から **0x7FDA0000** に変更。0059 の領域は 0x7FCA0000 + 1 MiB = 0x7FDA0000 までで、0x7FCB0000 はその中だった（テストが検出）。② 停止メッセージから 0x8000 と 0x400000 の数字を除いた（ソース検査がヘッダ以外の数字を禁じるため）。いずれも work に同じ編集をし、置換後に `diff` で一致を確認した。
- 既存ファイルの差分（`git diff --stat`）: `fle_cursor.c` +32 -1、`fle_cursor.h` +22、`fle_res.c` +33、`fle_res.h` +4（追記が中心。`fle_cursor_handle_of` の本体は `std_handle` に切り出して同じ式にした: 唯一の `-1` 行）。LoadCursorW の挙動は変えていない（既存の `tests/cursor.rs` 25 件はそのまま通る）。
- `lift/crates/fe-lift/src/imports_table.rs`: 末尾に 1 行追記（編集の直前に読み直した）。`lib.rs` は変更なし。
- テストの表明の更新: `tests/gdi.rs`（リストから `LoadImageW` を外し、理由をコメントに書いた）、`tests/cursor.rs`（`LoadImageW` を 2 か所から外し、`LoadImageA` に置き換えた。ソース検査の「static 変数なし」を「可変の static 変数なし」に直した: `image_ids` の定数表と static 関数が増えたため）。`tests/spi.rs` に LoadImageW はなかった。

## 5. テスト（実測）

- `lift/crates/fe-lift/tests/loadimage.rs`（新規）: **31 件すべて ok**（`lift/out-img/test-loadimage4.txt`）。内訳: 表と宣言、Learn の値、領域（0x80000000 未満、標準カーソル・GDI・DC の領域と重ならない）、13 個の標準カーソルの答え、LoadCursorW と同じ IDC_ARROW、DLL の 6 個の答え、毎回同じ・last error・戻り番地、32 ビット引数の上位ビット、停止（type、サイズ、fuLoad、文字列名、NULL で別の識別子、hInst、32649、32761 とほかの id、検査の順）、検査関数、他のハンドル表が止まること、ソース検査、ヘッダ、資源検査関数が last error に触れないこと、生成器（thunk と DLL 名・API 名）、DLL の資源（7 個の数値グループと 32649 の不在、RT_CURSOR 60）、ゲストの 21 回を再現して 20 回答えて 1 回（-21、32649）止まること、合成イメージの資源検査（NEWHEADER、ResCount、コンポーネントの欠落、型 12 なし）。
- `tests/cursor.rs` 25 件、`tests/gdi.rs`: 変更後も ok。
- 変異テスト（`lift/out-img/work/mkmut.py` が `lift/out-img/mut/` に複製を作り、`FLE_LI_SRC_DIR` で指す。`runmut.sh`、`mut/summary.txt`）: 40 変異。**最初の実行で 40 件すべてがテストの失敗で検出された**（生き残り 0、失敗件数 1〜19。コンパイルエラーで落ちたものは 0: `clang_failed_mentions=0`）。ヘッダだけを変える 7 件（m18〜m20、m23、m24、m26、m27）は、複製に無いファイルを `runtime/` から読む（`loadimage.rs` の `src`）ので、念のため 6 ファイルすべてを複製に入れた `-withc` 版 7 件を別に走らせた。いずれも失敗 3〜8 件で検出された。**同値変異は作っていない**（各変異は観測できる値・分岐・書き込み・理由文を変える）。

## 6. harness の再実行（実測）

生成 C は変わった（thunk が応答になった）ので `lift/out-img/out` に新規生成した（`OUT=$PWD/lift/out-img bash lift/harness/wasm/build.sh gen split`、次いで `rt gen-c harness link`、native は `build_native.sh`）。`functions 59297`、`bad_nodes 0`、`unsupported_total` **542 → 541**（`jmp_imp:LoadImageW 1`）。runtime、生成 C の 17 パーツ、harness が `errors=0 warnings=0`（`fle_proc.c` の既存の警告 1 件のみ。`fle_loadimage.c`、`fle_cursor.c`、`fle_res.c` は 0 件）。`OUT` は絶対パスで渡す（`a4-survey.sh` は相対パスだと fe-pe を見つけられない）。

| | native | wasm（node） |
|---|---|---|
| 停止 | `fle: LoadImageW: the lifted DLL has no cursor MAKEINTRESOURCE(32649) (RT_GROUP_CURSOR): real Windows fails with NULL, but Learn gives no GetLastError value for it, so it is not answered (decision 0061)`（SIGABRT、終了コード 134） | 同じ（134） |
| 関数実行数（distinct） | **580**（前 579） | 580 |
| 入口数（calls） | **13,250**（前 13,246） | 13,250 |
| 呼び出し深さ | 9 | 9 |
| unsupported_total | **541**（前 542） | 541 |

- `diff <(grep -v '^calling' native) <(grep -v '^calling' node)`: **差なし**。stderr も差なし。全体の diff は `calling entry … rsp=` の 1 行だけ。
- 呼び出しスタックは前回と同じ（`f_425100` ← `f_8659f0` ← `f_8651a0` ← `f_620cd0` ← `f_623840` ← `f_4120c0` ← `f_412160` ← `f_41d8f0` ← `f_1376820`）。同じ呼び出し点 0x865a55 の **ループの 2 回目**（`esi = -21`、hInst = モジュール、名前 32649）で止まった。ループの 1 回目（`esi = -22`、IDC_SIZEALL、hInst NULL）は通過した。入口数が 4 増えた内訳（1 回目の `f_865c50` とその中の確保呼び出しなど）は数えていない（推測）。
- a4（`a4-survey.sh`、`WASM=1`）: `init_count=115 / 942`、最後の unit は index 114（run_rank 111/895、proc 0x623840、in_progress）、`started=111 completed=110 in_progress=1 of 895 runnable`、`all_units_done=no`。**前回と同じ**（停止は同じ unit 114 の中）。native と wasm の a4 の 6 行は一致。

## 7. その先の見通し

- **PM の判断が要る点**: `esi = -21`（hInst = モジュール、32649）。DLL に資源が無いので実 Windows では NULL。Learn は GetLastError の値を書かない。選択肢（決定 0061）: A. 停止のまま（今の形）、B. NULL を返し last error を触らない、C. NULL を返し 1814。ゲストは NULL を許容して先へ進み、idx = -21 を引く側が `SetCursor(NULL)`（カーソルを消す）になる（§3）。B か C にすると 1 行で通る。
- そのあと: LoadImageW のループの残り（-20 … -2）は答えられる形（NULL 側 12 回、モジュール側 6 回）。次の停止は `f_865c50` の確保（0x409410 のメモリ確保。既存の応答）の後の、SetCursor（39 か所）か DestroyCursor（4 か所）、GetCursor（3 か所）、ShowCursor（2 か所）、0xa0e3db の DeleteObject（0054 は発行していない値で止まる）。**どれが先に届くかは次の実行で分かる**（32649 の判断を B か C にしたとき）。
- SetCursor は `HCURSOR SetCursor([in, optional] HCURSOR hCursor)` で、「If this parameter is NULL, the cursor is removed from the screen」「The return value is the handle to the previous cursor」（Learn）。前のカーソルを仮想の状態として持つ（スレッドごとか。決定が要る）。

## 8. 未確認、限界

- 実 Windows との比較はしていない。LoadImageW の返すハンドル、LoadCursorW との同一性、32649 の失敗コードは実測していない。
- 32649 の意図、DLL のカーソル（32762〜32767）の使われ方（SetCursor までの流れは読んだが、仮想呼び出し `[rbx+0x4a0]` の先は未追跡）。
- 0x814f2c の DestroyCursor のループの要素の出どころ（0x15f6f70 の表）。
- `f_8661d0`（`[obj+0x15c]` の表）の中身。破棄の条件の一部。

## 9. 触ったファイルと制約

- 変更: `lift/crates/fe-lift/src/imports_table.rs`（2 行追記）、`lift/runtime/fle_cursor.c`、`fle_cursor.h`、`fle_res.c`、`fle_res.h`、`lift/crates/fe-lift/tests/gdi.rs`、`tests/cursor.rs`。
- 新規: `lift/runtime/fle_loadimage.h`、`fle_loadimage.c`、`lift/crates/fe-lift/tests/loadimage.rs`、`docs/decisions/0061-virtual-load-image.md`、本書、`lift/out-img/`（gitignore 済み）。
- `lift/harness/` 配下は変更していない。`lift/iat-unsup-closure.txt`、`trace.c.new`、`trace_native.c.new` には触っていない。
- `rm`、既存ファイルの `>` 上書き、git の変更操作、`/tmp`、コマンド文字列の `sleep`/`timeout`/`nohup` は使っていない。置き換えた成果物（古い .o、wasm、native バイナリ）は `mv` で `lift/out-img/stale/` に退避した。emcc は `JOBS=2`。
