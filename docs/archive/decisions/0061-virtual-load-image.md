# 0061: 仮想 LoadImageW（ゲストが使う 1 つの引数形だけに固定のハンドルで答え、ほかは名前付きで止める）

状態: 作業者（サブエージェント）が決定。方針（ホストの実カーソル・実テーマをゲストに見せない。ハンドル値は仮の決定として固定する）は依頼者（PM）。**ハンドルの値は仮の決定**。「一次」は Microsoft Learn（2026-10-22 取得）、「非公式」はそれ以外。実装は `lift/runtime/fle_loadimage.h`、`fle_loadimage.c`（新規）、`fle_cursor.h`、`fle_cursor.c`（12 個の IDC_* と検査関数の追加）、`fle_res.h`、`fle_res.c`（`fle_res_cursor_group_ok` の追加）、`lift/crates/fe-lift/src/imports_table.rs`（1 行追記）。テストは `lift/crates/fe-lift/tests/loadimage.rs`。調査と実行記録は `docs/investigations/loadimage-caller.md`。

## 背景

harness は `fle: unsupported instruction at 0x425100`（`user32.dll!LoadImageW` の thunk、IAT スロット 0x1971090）で止まっていた。呼び出し点は 0x865a55 の 1 か所（f_8659f0 の 21 回ループ）。形は `LoadImageW(hInst, name, IMAGE_CURSOR, 32, 32, LR_SHARED)`。hInst は 14 回が NULL（13 種の標準 IDC_*）、7 回が `[0x15ec298]`（= 0x400000、DLL 自身）で名前は 32649 と 32762〜32767（`loadcursor-caller.md` §4、`loadimage-caller.md`）。

## 原則（0059 と同じ）

1. ホストの実カーソル・実テーマ・実マウス設定は見せない。値は仮想で、この決定で固定する。
2. ページから決まらない値は、選択肢・採用理由・撤回条件とともに書く。非公式の出典に基づくものは「仮の決定」と明記する。
3. ゲストが使わない形は `fle_stop` で止める。値は作らない（失敗の NULL も、ページが GetLastError の値を書かない限り作らない）。
4. カーソルのピクセルとホットスポットは読まない。必要になったら名前付き停止にする。

## 一次資料で確認した規則（LoadImageW ページ。`updated_at` 2026-05-15）

| 項目 | ページの文面（引用） |
|---|---|
| 宣言 | `HANDLE LoadImageW([in, optional] HINSTANCE hInst, [in] LPCWSTR name, [in] UINT type, [in] int cx, [in] int cy, [in] UINT fuLoad)` |
| hInst | 「To load a predefined image or a standalone resource (icon, cursor, or bitmap file), set this parameter to NULL.」 |
| name | 「If the hInst parameter is non-NULL and the fuLoad parameter omits LR_LOADFROMFILE, name specifies the image resource in the hInst module.」序数は MAKEINTRESOURCE。「If the hInst parameter is NULL and the fuLoad parameter omits the LR_LOADFROMFILE value and includes the LR_SHARED, the name specifies the predefined image to load.」接頭辞表に **IDC_（Standard cursors）**。Remarks: IS_INTRESOURCE(name) が TRUE なら整数 ID、でなければ文字列へのポインタ。 |
| type | IMAGE_BITMAP、IMAGE_CURSOR、IMAGE_ICON の 3 つ。**数値は LoadImageW のページに書かれていない**（表は名前と意味だけ）。CopyImage のページの表が IMAGE_BITMAP 0、IMAGE_ICON 1、IMAGE_CURSOR 2 を書く（一次）。mingw-w64 の `winuser.h`（4181〜4183 行）も同じ（照合のみ）。 |
| cx / cy | 「The width, in pixels, of the icon or cursor. If this parameter is zero and the fuLoad parameter is LR_DEFAULTSIZE, the function uses the SM_CXICON or SM_CXCURSOR system metric value」。0 で LR_DEFAULTSIZE なしなら実リソースの幅。 |
| fuLoad: LR_SHARED | 0x00008000。「Shares the image handle if the image is loaded multiple times. If LR_SHARED is not set, a second call to LoadImage for the same resource will load the image again and return a different handle.」「When you use this flag, the system will destroy the resource when it is no longer needed.」「Do not use LR_SHARED for images that have non-standard sizes, that may change after loading, or that are loaded from a file.」「When loading a system icon or cursor, you must use LR_SHARED or the function will fail to load the resource.」**「This function finds the first image in the cache with the requested resource name, regardless of the size requested.」** |
| 戻り値 | 成功は「the handle of the newly loaded image」。失敗は NULL、「To get extended error information, call GetLastError.」**GetLastError の値はページに書かれていない。** |
| 同一性 | 同じ資源を LR_SHARED で 2 回読むと**同じハンドル**（上の「Shares the image handle」）。「the function returns the handle of the already loaded resource」という文言そのものは**ページに無い**（上の文面が最も近い）。 |

ほかのページ: LoadCursorW（0059 に記録。「loads the cursor resource only if it has not been loaded; otherwise, it retrieves the handle to the existing resource」、LoadImage（LR_DEFAULTSIZE \| LR_SHARED）に取って代わられた）。About Cursors（「LoadCursor always uses LR_SHARED internally regardless of the instance parameter」「both return shared cursors that must not be passed to DestroyCursor」「Use LoadCursor or LoadImage to obtain an HCURSOR handle」「HCURSOR and HICON both refer to the same underlying object type」）。Resource File Formats、NEWHEADER、RESDIR（RT_GROUP_CURSOR = NEWHEADER{Reserved=0, ResType=2, ResCount} + ResCount 個の 14 バイトの RESDIR、最後の WORD が RT_CURSOR の id。RT_CURSOR の先頭 4 バイトはホットスポット）。Resource Types（RT_CURSOR 1、RT_GROUP_CURSOR = RT_CURSOR + 11 = 12）。x64 calling convention（5 番目以降はスタック、32 ビット引数の上位ビットは無視してよい）。

## 値と出典（項目ごと）

| 項目 | 値 | 出典 | 状態 |
|---|---|---|---|
| IMAGE_CURSOR | 2 | 一次: CopyImage ページ（LoadImageW ページは数値を書かない） | 確認済み |
| LR_SHARED | 0x8000 | 一次: LoadImageW ページ | 確認済み |
| 標準カーソルの識別子 13 個 | 32512、32513、32514、32515、32516、32642〜32646、32648、32650、32651 | 一次: About Cursors の表（ゲストの表 `0x1409038` の 21 個の数値と一致） | 確認済み |
| ゲストのサイズ 32×32 | 32 | ゲストの即値（`r9d = 0x20`、`[rsp+0x20] = 0x20`）。ページは 32 を特別扱いしない | 受け付ける形の選択 |
| DLL の RT_GROUP_CURSOR の id | 32762〜32767（32761 も存在、32649 は無い） | **DLL の実測**（fe-pe と pefile）。ページの値ではない | 実測 |
| 標準カーソルのハンドル | `0x7FCA0000 + 4 * (idc - 32512 + 1)`（0059 と同じ式。IDC_ARROW は 0x7FCA0004） | なし（どのページも値を書かない） | **仮の決定** |
| DLL のカーソルのハンドル | `0x7FDA0000 + 4 * (id - 32762 + 1)`（32762 は 0x7FDA0004、32767 は 0x7FDA0018）。領域 1 MiB | なし | **仮の決定**。0x80000000 未満（64 ビットのページの切り詰め・符号拡張）、0059 の標準カーソル領域（0x7FCA0000 + 1 MiB = 〜0x7FDA0000）、0054 の GDI 領域（0x7FBA0000）、0053 の DC 領域（0x7FA90000）と重ならず、イメージ（〜0x18C0000）より上 |
| LoadImageW(NULL, IDC_ARROW) と LoadCursorW(NULL, IDC_ARROW) の同一性 | **同じハンドル**（0x7FCA0004） | 下の「決定 1」 | 同一性は一次からの**読み**（2 つの API のキャッシュが同じだとは、どのページも明言しない）、値は仮 |
| GetLastError | 触れない | ページは成功時を書かない | 保守的な選択（0057〜0059 と同じ） |
| 失敗（NULL）の GetLastError | **ページに無い** | — | 作らない。停止 |

## 決定

1. **LoadImageW(NULL, IDC_X) と LoadCursorW(NULL, IDC_X) は同じハンドルを返す**（0059 の撤回条件に挙げられていた論点）。理由（一次の文面からの読み）: LoadImageW の LR_SHARED は「Shares the image handle if the image is loaded multiple times」「finds the first image in the cache with the requested resource name, regardless of the size requested」で、キャッシュのキーは資源名でサイズではない（「regardless of the size requested」）。About Cursors は「LoadCursor always uses LR_SHARED internally」で、LoadCursor が LR_SHARED で読んだ画像と LoadImage が LR_SHARED で読む画像は同じ共有物と読める。ゲストが 0x865a80 で LoadCursorW(NULL, IDC_ARROW) の結果を `[obj+0x154]` と比べて等しければ DestroyCursor を避ける作りで、同じ IDC_ARROW から出るハンドルは 1 つという前提を支持する。**限界**: ページは「LoadCursor の共有キャッシュと LoadImage の共有キャッシュが同一」とは書かない。サイズ 32 が既定のカーソルサイズ（About Cursors の 32×32、SM_CXCURSOR）と同じかも書かれていない。別のハンドルにする案も成り立つが、その場合は「LR_SHARED で同じ資源を読めば同じハンドル」という文面と矛盾する形が生じる。
2. **ハンドルの式は 1 つ**（0059 の `std_handle`）。LoadCursorW は引き続き IDC_ARROW だけに答える（`fle_cursor_handle_of` は変えていない。ゲストの LoadCursorW の静的な呼び出しは IDC_ARROW のみ）。LoadImageW 側の 13 種は `fle_cursor_image_handle_of`（新規）が同じ式で答える。IDC_HAND（32649）は NULL hInst の 13 種に含めない（ゲストは hInst = モジュールでしか使わない）。
3. **形 (a)**: type == IMAGE_CURSOR、cx == cy == 32、fuLoad が**ちょうど** LR_SHARED、name が整数、hInst が NULL、名前が 13 種のどれか → 標準カーソルのハンドル。
4. **形 (b)**: 同じ type、サイズ、fuLoad、hInst が 0x400000（`FLE_EXE_HANDLE`）、名前が 32762〜32767 → DLL に RT_GROUP_CURSOR（NEWHEADER の Reserved 0、ResType 2、ResCount ≧ 1、各 RESDIR が指す RT_CURSOR がすべて存在）が実在することを `fle_res_cursor_group_ok`（fle_res.c。LoadStringW の実装と同じ資源ディレクトリの読み取り）で確かめ、`0x7FDA0000 + 4 * (id - 32762 + 1)` を返す。ピクセル・ホットスポットは読まない。
5. **停止する形**（`fle_stop`、「decision 0061」を含む理由文。検査の順は type → fuLoad → サイズ → 名前の形 → hInst → 識別子）:
   - type が IMAGE_CURSOR 以外（IMAGE_BITMAP、IMAGE_ICON、その他）。
   - fuLoad が LR_SHARED と等しくない（LR_SHARED に別のフラグが付く形、LR_SHARED 無しを含む）。
   - cx / cy が 32 でない（0 を含む）。
   - 名前が文字列へのポインタ（上位 48 ビットが 0 でない）。
   - hInst が NULL でも 0x400000 でもない。
   - hInst が NULL で名前が 13 種以外（IDC_HAND、PIN、PERSON、追加カーソル、DLL の id を含む）。
   - hInst が 0x400000 で、DLL にその RT_GROUP_CURSOR が無い（**32649**）。実 Windows は NULL で失敗するはずだが、ページは GetLastError の値を書かないので値を作らない。
   - hInst が 0x400000 で、DLL に存在するがゲストが読まない id（32761）。
   - type、cx、cy、fuLoad は 32 ビットの引数なので下位 32 ビットだけを見る（x64 calling convention ページ）。hInst と名前は 64 ビット全体。
6. ホストのカーソル・テーマ・マウス設定・画面・ファイル・時計は読まない（テスト `source_uses_no_host_…`）。読むのは DLL 自身の資源ディレクトリだけ。
7. 既存ファイルの変更は追記のみ: `fle_cursor.c/.h`（12 個の定数、13 個の表、検査関数 2 つ。`fle_cursor_handle_of` と `LoadCursorW` の挙動は変えない）、`fle_res.c/.h`（関数 1 つ）、`imports_table.rs`（1 行）。

## 選択肢

- 同一性: A. LoadCursorW と同じ（採用）。B. LoadImageW だけ別領域のハンドル。**不採用**: 「Shares the image handle」と矛盾しうる。撤回条件に回す。
- DLL のカーソルのハンドル: A. 固定の式（採用。同じ引数なら同じ値、順序に依存しない）。B. 呼ぶ順に採番。**不採用**: 0059 と同じ理由。
- 32649（モジュール）: A. 停止（採用）。B. NULL を返し last error を触らない。C. NULL を返し 1814（`ERROR_RESOURCE_NAME_NOT_FOUND`、0025 の FindResourceW と同じ対応づけ）。**B、C は不採用**: ページは GetLastError の値を書かない。ゲストは NULL を保存でき（`loadimage-caller.md` §4）、SetCursor(NULL) を後で呼ぶので、黙って続けると見た目の挙動が変わる。PM が B か C を選べば 1 行で変えられる。**【追記】0062（`0062-load-image-missing-resource.md`）で B 案に決定した（NULL を返し last error には触れない。仮の決定）。この節の「A（採用）」「B、C は不採用」は決定時点の記録として残す。**
- 受け付ける範囲: A. ゲストの形だけ（採用）。B. LoadImageW の全引数形。**不採用**: 使われないコードは検証できない（0058、0059 と同じ判断）。

## 撤回条件

- 別の一次資料が LoadCursorW と LoadImageW（LR_SHARED）のキャッシュが別だと示す。決定 1 を逆にする。
- ゲストが HCURSOR を 32 ビットに切り詰める、ポインタとして検査する、GetIconInfo や DrawIcon 系にハンドルを渡す。領域と種類を見直す。
- 32649 の扱い（PM の未決）。実 Windows で失敗時の GetLastError が確認できる、または PM が NULL を許可する。
- ゲストが DestroyCursor に標準カーソル・DLL のカーソルを渡して止まる。共有物の破棄の規則（About Cursors は LoadCursor 由来の共有カーソルに DestroyCursor を呼ばないと書く。LoadImage の LR_SHARED は「the system will destroy the resource when it is no longer needed」）を決める。
- ゲストがカーソルの画像・ホットスポットを読む API（GetIconInfo、DrawIconEx、CopyImage の LR_COPYFROMRESOURCE など）に届く。RT_CURSOR のデータ（実測した形: 308 バイト、32×64 の 1bpp、ホットスポットあり）を読む。

## 確認できていないこと

- 実 Windows が LoadImageW で返すハンドルの値、失敗時の GetLastError、LoadCursorW と同じハンドルを返すかの実測（比較していない）。
- 32649（hInst = モジュール）をゲストが意図して呼ぶのか。DLL に 32649 の資源が無いので、実 Windows では NULL が返り、ゲストは `idx = -21` に NULL を保存する（`loadimage-caller.md` §3）。
- DLL のカーソル 32762〜32767 の使われ方（0xa00cf1 などで SetCursor、破棄時に DestroyCursor に渡る）。
