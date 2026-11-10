# LoadCursorW の呼び出し元の調査と、仮想標準カーソルの実装後の harness の再実行の記録（決定 0059）

実行日: 2026-10-21〜22。作業ディレクトリ `signature-studio-develop`、HEAD c2fc046。DLL は `binaries/win64/FLEngine_x64.dll`。決定は `docs/decisions/0059-virtual-cursors.md`。「実測」はコマンドの出力、「読み」は `lift/objdump-text.txt` の逆アセンブルまたは `.text` のバイト走査の静的な読み、「推測」は未検証。成果物は `lift/out-cur/`（gitignore 済み。`TMPDIR=lift/out-cur/tmp`、`EM_CACHE=lift/out-cur/emcache`、`CARGO_TARGET_DIR=lift/out-cur/target`）。補助スクリプトは `lift/out-cur/work/`（`sites.py` = 呼び出し点と直前のレジスタ書き込み、`bytescan.py` = `.text` 全体のバイト走査、`dis.py` = 範囲の逆アセンブル、`mkmut.py`、`runmut.sh`）。

注意: `lift/objdump-text.txt` は 0x11fb4af で終わる（`.text` は 0x13768a0 まで）。`sites.py` が objdump を読むので、0x11fb4af より後の呼び出し点を落とす。`bytescan.py`（`.text` 全体の `call rel32` と `jmp [rip+disp]` の走査）で数え直したところ、LoadCursorW は 7（変わらず）、SetCursor は 37 → **39**、LoadIconW は 5 → **6** だった（落ちていた 3 か所は 0x121f3ab、0x1220392、0x13764e6）。この記録の数は `bytescan.py`（`bytescan.txt`）の値を使う。

## 1. 停止点の確認（実測）

開始時点: `fle: unsupported instruction at 0x4250e0`。0x4250e0 は `ff 25 8a bd 54 01`（`jmp [rip+0x154bd8a]`）で、IAT スロットは 0x1970e70、pefile の import 表で `user32.dll!LoadCursorW`。呼び出しスタックは依頼文のとおり。基準: 関数実行数 578、入口数 13,245、深さ 9、unsupported_total 543。`LoadCursorW` を指す thunk は 0x4250e0 の 1 つだけ、`LoadCursorA`、`LoadCursorFromFile*` は import に無い（実測: pefile）。

## 2. LoadCursorW の全呼び出し点（実測・読み）

thunk を指すのは `call rel32` が 7 か所だけで、`jmp`／条件ジャンプは 0。thunk の VA、スロットの VA（4／8 バイトの絶対値と 4 バイトの RVA）はファイルのどこにも無い（実測。テスト `nothing_else_in_the_file_points_at_the_thunk_or_the_slot`）。間接呼び出しは追えない。

| 呼び出し点 | 関数（生成 C） | hInstance | lpCursorName | 型 | 戻り値の行き先（読み） |
|---|---|---|---|---|---|
| 0x6117b5 | f_611670 | NULL（`xor ecx,ecx`） | 0x7F00（`mov edx,0x7f00`） | **共有カーソル、IDC_ARROW** | `[rcx+0x58]`（`rcx = [rbp+0x30]`。同じ構造体の +0x30 に 0xb、+0x48 に `[0x15ec298]`（モジュール）、+0x60 に 0 を書く。WNDCLASSW を +0x30 から置いたとすると +0x58 は hCursor（WNDCLASSW: style 0、hInstance 24、hCursor 40）。**推測**: 以降 f_611670 は +0x78 に別の値、`GetClassInfoW`／`RegisterClassW`（0x6119f4）を呼ぶ別の WNDCLASS を使う。この構造体が RegisterClassW に渡る証拠は追えていない） |
| 0x865a01 | f_8659f0 | NULL | 0x7F00 | 共有カーソル、IDC_ARROW | `[obj+0x154]`（obj = rcx）。**今回の停止点** |
| 0x865ae6 | f_865a80 | NULL | 0x7F00 | 共有カーソル、IDC_ARROW | `[obj+0x154]` と `cmp rax,[rbx+0x154]` で比較。等しければ何もしない。違えば `DestroyCursor([obj+0x154])`（0x865b0c。ただし f_8661d0(obj, -2) が偽のとき） |
| 0x86618f | f_866170 | NULL | 0x7F00 | 共有カーソル、IDC_ARROW | `[obj+0x154]`（f_866170(obj, idx=0, cursor=NULL) のとき。cursor が非 NULL なら `[obj+0x154] = cursor`） |
| 0x9f2927 | f_9f2870 | `[0x15ec298]`（モジュール） | 文字列名へのポインタ（`f_414850` が返す。`lea rcx,[rip+..]` = 0x9f2b20、UTF-16 `VT_HEADERSPLIT`） | **リソースのロード** | `f_866170(*[0x15e4c80], 0x3f, cursor)` |
| 0x9f2959 | f_9f2870 | 同上 | 0x9f2b4c の `VT_VERTSPLIT` | リソースのロード | `f_866170(..., 0x3e, cursor)` |
| 0xa0e3db | f_a0e1a0 | 同上 | 0xa0e578 の `VT_MOVEN` か 0xa0e598 の `VT_MOVES`（条件分岐の 2 択） | リソースのロード | `[obj+0x7d4]` と比べ、違えば古い方を `0x424090`（gdi32!DeleteObject）に渡し、新しい方を保存して `SetCursor([obj+0x7d4])`（0xa0e40d）。同じなら新しい方を DeleteObject |

- 静的に確定できた (hInstance, lpCursorName) は **4 か所が (NULL, MAKEINTRESOURCE(0x7F00))** で、すべて同じ形（バイト列 `33 c9 ba 00 7f 00 00` が call の直前。テスト `four_sites_pass_null_and_idc_arrow_and_…`）。**3 か所は動的**: hInstance は `[0x15ec298]`、名前はポインタ。
- `[0x15ec298]` の書き手: `mov [0x15ec298],reg` は objdump 中 1 か所だけ（0x41d919、`f_41d8f0` が `rdi = r8`（第 3 引数、エントリの rcx = 0x400000）を保存）。即値の書き込みは探していない（読み）。したがって hInstance は 0x400000（モジュール）と読める。
- 動的な名前は DLL 自身の `RT_GROUP_CURSOR` 57 件の名前（`VT_HEADERSPLIT`、`VT_VERTSPLIT`、`VT_MOVEN`、`VT_MOVES` ほか。実測: pefile。RT_CURSOR 60、RT_GROUP_CURSOR 57、RT_ICON 13、RT_GROUP_ICON 1）に含まれる。つまり **LoadCursorW の hInstance 非 NULL の呼び出しは、ゲスト自身のリソースを読む**。決定 0059 はこれを実装せず停止する。
- WNDCLASS として静的に埋め込まれた 6 つの表（0x13be810、0x13f0648、0x13f5f20、0x1409cb8、0x14d6b48、0x151c970）はファイル上で hCursor（+0x28）が 0（実測）。実行時に書かれる。

## 3. 戻り値（HCURSOR）の使われ方（読める範囲）

- `[obj+0x154]` は「既定カーソル」。f_8659f0（obj の初期化。f_8651a0 から。今回の停止点）が LoadCursorW(NULL, IDC_ARROW) の結果を入れ、続けて 21 個の LoadImageW（§4）の結果を `f_865c50(obj, idx, cursor)` でリスト `[obj+0x14c]` に追加する（ノードは 20 バイト: next、idx、cursor）。
- f_866060(obj, idx)（読み取り）: idx が -1 なら 0、リストに idx があればその cursor、なければ `[obj+0x154]` を返す。f_866090(obj, short idx)（カーソルの切り替え）: 現在値 `[obj+0x9c]` と同じなら何もせず、idx == 0 かつ条件が整えば別の経路（0x425390、0x425380 = ウィンドウへのメッセージ系と読める。**推測**）、そうでなければ `SetCursor(f_866060(obj, idx))`（0x86614e）。
- f_866170(obj, idx, cursor)（設定）: idx == 0 なら cursor が NULL のとき LoadCursorW(NULL, IDC_ARROW)、そうでなければ cursor を `[obj+0x154]` へ。idx が -1 以外なら f_865b20（idx の項目を削除し、その cursor を DestroyCursor。ただし f_8661d0 が真なら破棄しない）→ cursor が非 NULL なら f_865c50 で追加。
- f_865a80（破棄）: リストを辿り、idx が -17〜-12、-21、または正で、かつ f_8661d0（`[obj+0x15c]` の表に idx があるか）が偽で cursor 非 NULL のとき `DestroyCursor(cursor)`。その後 **LoadCursorW(NULL, IDC_ARROW) を呼び直して `[obj+0x154]` と比べる**。等しければ破棄しない（共有カーソルと見ている）。→ **同じ IDC_ARROW が毎回同じハンドルを返す性質をゲストが前提にしている**（決定 0059 の「毎回同じ」を支持する読み。テスト `the_result_is_stored_compared_and_passed_on_the_way_the_investigation_reads_it` がバイト列で固定）。
- SetCursor への流れ: 戻り値は f_866060 経由でほとんどの SetCursor（33 か所）の引数になる。比較は上の 1 か所。WNDCLASS.hCursor は 0x6117b5 の 1 か所（推測、§2）。**IS 系（GetIconInfo など）への受け渡しは見つからなかった**。

## 4. 関連 API の呼び出し点（応答は作らない。数と引数だけ）

`bytescan.py` の値（`.text` 全体）。thunk は 1 API につき 1 つ。

| API | thunk | 呼び出し点の数 | 引数（読めた範囲） |
|---|---|---|---|
| LoadCursorW | 0x4250e0 | 7 | §2 |
| SetCursor | 0x425400 | **39**（objdump の範囲内の 37 と、範囲外の 0x121f3ab、0x1220392） | 33 か所が `f_866060(obj, idx)` の結果、ほかに `[0x15f3270]`（GetCursor の結果の保存先。0x606b94）、`[0x15f70a0]`（同じく GetCursor の結果。0x81bffe）、`[rbp+0xb0]`、`[obj+0x7d4]`（LoadCursorW の結果）。NULL を渡す形（`xor ecx,ecx`）は 37 か所の中に見つからなかった |
| GetCursor | 0x424c00 | 3 | 引数なし。結果は `[0x15f3270]`（0x606b94）、`[0x15f70a0]`（0x81bffe）に保存、または 0x86bd64 で `f_424d30`（GetIconInfo を指す thunk ではなく GetMenu の thunk 0x424d30 で、読み違いの可能性。**未確認**）へ |
| DestroyCursor | 0x424960 | 4 | 0x814f2c（リストの要素を全部）、0x865ac6、0x865b0c、0x865b61（いずれも `[node+0xc]` か `[obj+0x154]`）。§3 のとおり共有カーソルの破棄を避ける検査がある |
| ShowCursor | 0x425580 | 2 | 0x7f4cf1: `ShowCursor(FALSE)`（`xor ecx,ecx`）、0x7f4e70: `ShowCursor(TRUE)`（`or ecx,-1` は TRUE ではなく -1。**読み違いに注意**: bShow は BOOL だが、ゲストは -1 を渡す。非 0 は TRUE として扱われるはずだが、ページは書いていない） |
| LoadImageW | 0x425100 | **1**（0x865a55、f_8659f0 の 21 回ループ） | `LoadImageW(hInst, name, IMAGE_CURSOR = 2, cx = 32, cy = 32, LR_SHARED = 0x8000)`（`r8d = 2`、`r9d = 0x20`、`[rsp+0x20] = 0x20`、`[rsp+0x28] = 0x8000`。バイト列で固定）。hInst は 14 回が NULL、7 回が `[0x15ec298]`。名前は `0x1409038` の 8 バイト表の 21 個（下） |
| CopyImage | 0x424880 | 1（0x9f7ab4） | `CopyImage(h, IMAGE_BITMAP = 0, cx, cy, LR_COPYRETURNORG... ではなく flags 4）`: `xor edx,edx`（type 0 = IMAGE_BITMAP）、`[rsp+0x20] = 4`（= LR_COPYRETURNORG、Learn の値 0x4）。ビットマップの複製で、カーソルではない |
| SetCursorPos | 0x425410 | 11（objdump 内 7） | 座標。本件の範囲外（数だけ） |
| GetCursorInfo | 0x424c10 | 1 | `lea rcx,[rsp+0x28]`（0x81bfea）。f_81bfe0 付近で GetCursor と組 |
| SetClassLongPtrW / GetClassLongPtrW | 0x4259a0 / 0x425990 | 3 / 1 | index -26（0xffffffe6）= GCL_STYLE、-14（0xfffffff2）= GCLP_HICON（Learn の値は SetClassLongPtrW のページ）。GCLP_HCURSOR（-12）の呼び出しは見つからなかった |
| RegisterClassW / GetClassInfoW | 0x425260 / 0x424ba0 | 8 / 11 | WNDCLASS の hCursor が非 NULL でここに渡る経路は、静的な 6 つの表では 0（§2）。動的な表（0x6119f4 の `[rbp+0xd8]`、0x81cfab の `[rbp+0x68]`）は未追跡 |

LoadImageW の 21 個の識別子（`esi = -22 … -2`、表 `0x1409038`、Learn の数値）: 32646（SIZEALL）、**32649**（HAND）、32651（HELP）、32650（APPSTARTING）、32648（NO）、32762、32763、32764、32765、32767、32766、32514（WAIT）、32516（UPARROW）、32644（SIZEWE）、32642（SIZENWSE）、32645（SIZENS）、32643（SIZENESW）、32646（SIZEALL。重複）、32513（IBEAM）、32515（CROSS）、32512（ARROW）。hInst がモジュールになるのは `esi = -21`（32649）と `-17 … -12`（32762〜32767）の 7 つ。32762〜32767 は DLL の `RT_GROUP_CURSOR` の数値 id（実測: 32761〜32767 が存在）。**`esi = -21` の 32649（IDC_HAND）をモジュールで読む形は、モジュール側の id に存在しない**（実測）。意図は未確認。

## 5. 一次資料（Microsoft Learn。2026-10-21 取得）

決定 0059 の「一次資料で確認した規則」に書いた。要点: LoadCursorW のページは hInstance NULL で IDC_ の定義済みカーソルを読むと書く。About Cursors は **IDC_ARROW = MAKEINTRESOURCE(32512)**（0x7F00）を一次で書き、LoadCursor が内部で LR_SHARED を使うので共有ハンドルで DestroyCursor しない、HCURSOR と HICON は同じ種類のオブジェクトと書く。**0x7F00 = IDC_ARROW は一次で確認した**（`spi-caller.md` §8 の未確認事項の解消）。非公式の照合（値の根拠にはしない）: mingw-w64 の `winuser.h` の `IDC_ARROW MAKEINTRESOURCE(32512)` など 8 個（IBEAM 32513、SIZEALL 32646、NO 32648、HAND 32649、APPSTARTING 32650、HELP 32651、PIN 32671、PERSON 32672）が一致。ハンドルの値（0x7FCA0004）は**どのページにも無い仮の決定**。

## 6. 実装（実測）

- `lift/runtime/fle_cursor.h`、`fle_cursor.c`（新規）。`lift/out-cur/work/` で完成（clang `-Wall -Wextra -Werror` と emcc `-Wall -Wextra -Werror` の両方で警告なし）させてから、ヘッダ、`.c` の順に **1 回で** `lift/runtime/` に置いた。置いた後に編集していない。`cmp` で一致（`fle_cursor.h`、`fle_cursor.c`）。
- 0054 の表（`fle_gdi.c`）に同居させなかった（決定 0059 の決定 5）。状態を持たない純関数で、`fle_gdi.c` の既存の挙動に触れない。
- `lift/crates/fe-lift/src/imports_table.rs`: 末尾に 1 行（コメント付きで 2 行）追記（編集の直前に読み直した）。`lib.rs` は変更なし。
- `tests/spi.rs`、`tests/gdi.rs`: 「LoadCursorW は答えない」という表明（リストから `LoadCursorW` を外した。理由をコメントに書いた）。ほかのテストの表明（`sysmetrics.rs`、`gdc.rs` など）に LoadCursorW は無かった（実測: grep）。
- `lift/crates/fe-lift/tests/cursor.rs`（新規、25 件）。

## 7. テスト（実測）

- `cargo test -p fe-lift --test cursor`: **25 件すべて ok**（`lift/out-cur/test-cursor.txt`。66 秒）。内訳: 行と宣言、IDC_ARROW の数値（Learn の表から本文に書いた 16 個 + 追加 13 個。重複なし）、ハンドル領域（0x80000000 未満、0053 と 0054 の領域と重ならない）、答え（0x7FCA0004）、毎回同じ・他の呼び出しに依存しない、状態を持たない・last error に触れない・戻り番地を壊さない・r8/r9 に依存しない、検査関数、停止（hInstance 非 NULL 5 種、ポインタの名前 5 種 + 組み合わせ、ほかの標準カーソル 15 個、追加カーソルとその他 26 個）、ソースがホストを読まない・状態を持たない、ヘッダが plain C、カーソルのハンドルは DeleteObject と ReleaseDC で「発行していない値」として止まる、生成器が thunk を応答に置き換える、DLL 名と API 名が一致の一部、実 DLL（thunk とスロット、7 か所のバイト列、`[0x15ec298]` の書き手、`VT_*` の名前、戻り値の使われ方のバイト列、LoadImageW の次の停止点と 21 個の識別子）。
- 変異テスト（`lift/out-cur/work/mkmut.py` が `lift/out-cur/mut/` に runtime の複製を作り、`FLE_CUR_SRC_DIR` で指す。`runmut.sh`、`mut/summary.txt`）: **21 変異すべてがテストの失敗で検出された（生き残り 0）**。失敗件数 1〜9 件。別に m07（hInstance の検査を `if (0)` にした）は**コンパイルエラー**（未使用変数 `-Werror`）で落ちただけなので検出に数えず、コンパイルする m07b に置き換えた（3 件失敗）。内訳: m01 識別子が 32513（9）、m02 基底を動かす（5）、m03 基底を GDI 領域に置く（5）、m04 基底が 2 GiB 以上（5）、m05 刻み 8（4）、m06 +1 を外す（4）、m07b hInstance を調べない（3）、m08 hInstance の下位 32 ビットだけ（1）、m09 ポインタ名を調べない（1）、m10 名前の下位 16 ビットだけ（1）、m11 名前の下位 32 ビットだけ（1）、m12 ほかの識別子が NULL（2）、m13 ほかの識別子が ARROW のハンドル（2）、m14 NULL を返す（5）、m15 検査関数が識別子を問わない（1）、m16 検査関数が領域内なら何でも（1）、m17 last error を 0 にする（2）、m18 戻り番地を壊す（2）、m19 hInstance の理由文（3）、m20 名前の理由文（1）、m21 領域サイズが小さい（1）。**同値変異は作っていない**（各変異は観測できる値・分岐・書き込み・理由文を変える）。
- `cargo test --workspace --no-fail-fast`（`lift/out-cur/test-ws.txt`、終了コード 0）: `test result:` の行 **79 個、passed 802、failed 0、ignored 0**。直前の基準は 76 バイナリ・752 件。増分 +3 バイナリ・+50 件は、`cursor.rs` 25 件（本作業）と、同時に作業中の A4 survey ツール担当の `a4_report.rs` 15 件・`fe-pe/tests/inittable.rs` 10 件（ワークツリーに存在。本作業のものではない）。`spi.rs` 21 件、`gdi.rs` 21 件は変わらず ok。

## 8. harness の再実行（実測）

生成 C は変わった（thunk が応答になった）ので `lift/out-cur/out` に新規生成した（`OUT=lift/out-cur bash lift/harness/wasm/build.sh gen split`、次いで `rt gen-c harness link`、native は `build_native.sh`）。`functions 59297`、`bad_nodes 0`、`unsupported_total` **543 → 542**（`jmp_imp:LoadCursorW 1`）。runtime、生成 C の 17 パーツ、harness が `errors=0 warnings=0`（`build-wasm.txt`）。ただし `fle_proc.c` に**既存の**警告 1 件（前段の記録と同じ）。`fle_cursor.c` は 0 件。

| | native | wasm（node） |
|---|---|---|
| 停止 | `fle: unsupported instruction at 0x425100`（SIGABRT、`abort kind: fle_unsupported`、終了コード 134） | 同じ（134） |
| 関数実行数（distinct） | **579**（前 578） | 579 |
| 入口数（calls） | **13,246**（前 13,245） | 13,246 |
| 呼び出し深さ | 9（前 9） | 9 |
| unsupported_total | **542**（前 543） | 542 |

- `diff <(grep -v '^calling' run-native.txt) <(grep -v '^calling' run-node.txt)`: **差なし**。stderr も差なし。全体の diff は `calling entry … rsp=` の 1 行だけ（native `0x70173affffc0`、wasm `0x11c4f590`）。
- 0x425100 は **`user32!LoadImageW` の thunk**（スロット 0x1971090）。呼び出しスタック（native と wasm で同じ）: `f_425100`（guest-return 0x865a5a）← `f_8659f0` ← `f_8651a0`（0x86521b）← `f_620cd0`（0x620dea）← `f_623840` ← `f_4120c0` ← `f_412160` ← `f_41d8f0` ← `f_1376820`。呼び出し点は 0x865a55 の 1 か所（f_8659f0 の 21 回ループの 1 回目: `LoadImageW(NULL, MAKEINTRESOURCE(32646 = IDC_SIZEALL), IMAGE_CURSOR, 32, 32, LR_SHARED)`）。LoadCursorW の 1 回目（0x865a01）は通過した（停止点が先へ進んだことと整合）。

## 9. その先の見通し

- 次は LoadImageW。21 回の呼び出し: hInst NULL の 14 個は標準の IDC_*（SIZEALL が 2 回あるので 13 種: SIZEALL、HELP、APPSTARTING、NO、WAIT、UPARROW、SIZEWE、SIZENWSE、SIZENS、SIZENESW、IBEAM、CROSS、ARROW。HAND はモジュール側）、7 個は hInst = モジュールで DLL 自身のカーソル（32762〜32767 と 32649）。**NULL 側は共有カーソルとして仮想ハンドルで答えられる**（LR_SHARED が前提。ページ: 「When loading a system icon or cursor, you must use LR_SHARED」）。モジュール側は `RT_GROUP_CURSOR`/`RT_CURSOR` のリソース読み取り（DLL のリソース、`fle_res` 系の決定）が要る。
- 決めるべき点: LoadImageW(NULL, IDC_ARROW, IMAGE_CURSOR, 32, 32, LR_SHARED) が、LoadCursorW(NULL, IDC_ARROW) と同じハンドルを返すか（決定 0059 の撤回条件に書いた）。サイズ 32 は決定 0057 の SM_CXICON 32 と同じ（SM_CXCURSOR は 0057 で決めていない）。ゲストは破棄時に LoadCursorW(NULL, IDC_ARROW) と `[obj+0x154]` を比べるので、LoadImageW で別のハンドルを返しても破棄のロジックは通る（読み）。
- そのあと: SetCursor（39 か所、結果の保存先と NULL/無効なハンドルの扱い）、DestroyCursor（4 か所。共有カーソルへは呼ばない形と読めるが、モジュールのカーソルは破棄の対象になる）、GetCursor（3 か所）、ShowCursor（2 か所）、0xa0e3db の DeleteObject（gdi32 の 0054 は「発行していない値」で停止するので、動的な LoadCursorW を答えるなら DeleteObject の規則を決める必要がある）。

## 10. 未確認、限界

- 実 Windows との比較はしていない。LoadCursorW が実機で返すハンドルの値は見ていない。
- 間接呼び出しの先、`[0x15ec298]` への即値書き込みは追えていない。
- 0x86bd64 の GetCursor の戻り値の行き先（`f_424d30` が GetMenu の thunk であることまでは確認したが、読み違いの可能性がある）。
- 0x6117b5 の構造体が WNDCLASS として RegisterClassW に渡るか（推測）。
- `esi = -21` の LoadImageW（hInst = モジュール、32649）の意図。
- ShowCursor の `or ecx,-1` の扱い（今回は答えない）。

## 11. 触ったファイルと制約

- 変更: `lift/crates/fe-lift/src/imports_table.rs`（2 行追記）、`lift/crates/fe-lift/tests/spi.rs`、`tests/gdi.rs`（表明の更新）。
- 新規: `lift/runtime/fle_cursor.h`、`fle_cursor.c`、`lift/crates/fe-lift/tests/cursor.rs`、`docs/decisions/0059-virtual-cursors.md`、本書、`lift/out-cur/`（gitignore 済み）。
- `lift/harness/` 配下は変更していない（ワークツリーの `harness/` の差分は A4 survey ツール担当のもの）。`lift/iat-unsup-closure.txt` には触っていない。
- `rm`、既存ファイルの `>` 上書き、git の変更操作、`/tmp`、コマンド文字列の `sleep`/`timeout`/`nohup` は使っていない。emcc は `JOBS=2`。
