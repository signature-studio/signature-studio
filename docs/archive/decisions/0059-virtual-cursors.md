# 0059: 仮想標準カーソル（LoadCursorW(NULL, IDC_ARROW) だけに固定のハンドルで答え、ほかは名前付きで止める）

状態: 作業者（サブエージェント）が決定。方針（ホストの実カーソル・実テーマをゲストに見せない。ハンドル値は仮の決定として固定し、同じ IDC_* には毎回同じハンドルを返す）は依頼者（PM）。**ハンドルの値は仮の決定**。IDC_ARROW = 32512 は一次資料が書く値。実装は `lift/runtime/fle_cursor.h`、`fle_cursor.c`、`lift/crates/fe-lift/src/imports_table.rs`（2 行追記）、テストは `lift/crates/fe-lift/tests/cursor.rs`（25 件）、調査と実行記録は `docs/investigations/loadcursor-caller.md`。「一次」は Microsoft Learn（2026-10-21 取得）、「非公式」はそれ以外。

## 背景

harness は `fle: unsupported instruction at 0x4250e0`（`user32.dll!LoadCursorW` の thunk、IAT スロット 0x1970e70）で止まっていた。呼び出し点は 7 か所。うち 4 か所（0x6117b5、0x865a01、0x865ae6、0x86618f）が `xor ecx,ecx; mov edx,0x7f00`（hInstance NULL、lpCursorName 32512）の静的な形で、残り 3 か所（0x9f2927、0x9f2959、0xa0e3db）は hInstance がモジュールハンドル `[0x15ec298]`、lpCursorName が文字列名（`VT_HEADERSPLIT`、`VT_VERTSPLIT`、`VT_MOVEN` か `VT_MOVES`）の動的な形（`loadcursor-caller.md` §2）。

## 原則

1. ホストの実カーソル・実テーマ・実マウス設定・実リソースは見せない。値は仮想で、この決定で固定する。
2. ページから決まらない値は、選択肢・採用理由・撤回条件とともにここに書く。非公式の出典に基づくものは「仮の決定」と明記する。
3. ゲストが使わない IDC_*、hInstance が非 NULL の呼び出し、文字列名は `fle_stop` で止める。値は作らない。
4. 「ゲストが次の停止点までで使う範囲」だけを作る。SetCursor、GetCursor、DestroyCursor、ShowCursor、LoadImageW、CopyImage は作らない（呼び出し点の数だけ `loadcursor-caller.md` §4 に書いた）。

## 一次資料で確認した規則

| ページ | 確認した内容 |
|---|---|
| [LoadCursorW](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-loadcursorw)（`updated_at` 2025-07-01） | `HCURSOR LoadCursorW(HINSTANCE hInstance, LPCWSTR lpCursorName)`。「To load a predefined system cursor, set this parameter to NULL」。hInstance が NULL なら lpCursorName は IDC_ で始まる定義済みシステムカーソルの識別子、非 NULL なら名前または MAKEINTRESOURCE の序数。成功ならカーソルのハンドル、失敗は NULL。「The LoadCursor function loads the cursor resource only if it has not been loaded; otherwise, it retrieves the handle to the existing resource」。ページは LoadImage（LR_DEFAULTSIZE | LR_SHARED）に取って代わられたと注記する。 |
| [About Cursors](https://learn.microsoft.com/en-us/windows/win32/menurc/about-cursors)（2026-09-21） | 標準カーソルの表: **IDC_ARROW = MAKEINTRESOURCE(32512)**、IBEAM 32513、WAIT 32514、CROSS 32515、UPARROW 32516、SIZENWSE 32642、SIZENESW 32643、SIZEWE 32644、SIZENS 32645、SIZEALL 32646、NO 32648、HAND 32649、APPSTARTING 32650、HELP 32651、PIN 32671、PERSON 32672。別の表に識別子の無い追加カーソル（32631、32652〜32663）。「Standard cursors are predefined and do not need to be created」。「LoadCursor … return shared cursors that must not be passed to DestroyCursor. LoadCursor always uses LR_SHARED internally regardless of the instance parameter」。「HCURSOR and HICON both refer to the same underlying object type」。ユーザーはマウス設定で標準カーソルの画像をシステム全体で変えられる（ホストの値を見せない根拠）。 |
| [IS_INTRESOURCE](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-is_intresource)（2025-07-09） | 「checks whether all bits except the least 16 bits are zero. When true, _r is an integer identifier for a resource. Otherwise it is typically a pointer to a string」。 |
| [Interprocess communication](https://learn.microsoft.com/en-us/windows/win32/winprog64/interprocess-communication)（2025-11-01） | 下位 32 ビットだけが有効、切り詰めても符号拡張しても安全。例示は HWND と GDI の HBRUSH、HPEN（HCURSOR は挙がらない）。 |
| [LoadImageW](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-loadimagew)（2026-05-15） | 次の停止点の API。hInst NULL かつ LR_SHARED なら name は定義済みの画像（IDC_ など）。「When loading a system icon or cursor, you must use LR_SHARED」。LR_SHARED なしだと同じ資源でも別のハンドルになる。（この決定では答えない。） |

## 出典の確認状況（値ごと）

| 項目 | 値 | 出典 | 状態 |
|---|---|---|---|
| IDC_ARROW | 32512（0x7F00） | **一次**: About Cursors の表。mingw-w64 の `winuser.h`（`/usr/share/mingw-w64/include/winuser.h` 4124 行 `#define IDC_ARROW MAKEINTRESOURCE(32512)`）も同じ（照合のみ。値の根拠には使っていない）。ゲストの即値 `mov edx,0x7f00` と一致 | 確認済み（`spi-caller.md` §8 の「Learn では未確認」を一次に更新） |
| ハンドルの値 | `0x7FCA0000 + 4 * (idc - 32512 + 1)`。IDC_ARROW は `0x7FCA0004`。領域 1 MiB | なし（どのページも値を書かない） | **仮の決定**。0x80000000 未満（64 ビットのページに従い、切り詰めと符号拡張が同じ値になる）、0054 の GDI 領域（`0x7FBA0000` + 1 MiB）と 0053 の DC 領域（`0x7FA90000` + 1 MiB）の双方と重ならず、イメージ（〜0x18C0000）より上、を条件にした。HCURSOR に「下位 32 ビットだけが有効」が当てはまるかは**推測**（ページは HWND、HBRUSH、HPEN を挙げる。HCURSOR と HICON が同じ種類のオブジェクトという記述だけが根拠） |
| 同じ IDC_* は同じハンドル | 常に同じ | **一次**: LoadCursorW「retrieves the handle to the existing resource」、About Cursors の LR_SHARED。値そのものは書かれない | 同一性は一次、値は選択（仮） |
| 呼び出し順への非依存 | ハンドルは IDC_* だけの関数（状態を持たない） | なし | 選択。0054 の slot 表のような状態を持たないので、スレッド規則も要らない |
| 非 NULL hInstance、文字列名 | 停止 | 一次: ページは「リソースのロード」とだけ書き、ホスト側に対応物が無い | 実装しない選択 |
| GetLastError | 触れない | ページは成功時を書かない | 保守的な選択（0057、0058 と同じ） |

## 決定

1. **LoadCursorW**: hInstance（rcx の 64 ビット全体）が NULL で、lpCursorName（rdx の 64 ビット全体）が 32512 のときだけ、固定のハンドル `0x7FCA0004` を返す。毎回同じ。
2. hInstance が非 NULL は `fle_stop`（"hInstance is not NULL: cursor resources of a module are not implemented (decision 0059; only IDC_ARROW is answered)"）。rcx の上位 32 ビットだけ立っていても停止する。
3. lpCursorName の上位 48 ビットが 0 でない（IS_INTRESOURCE が偽。文字列名へのポインタ）は `fle_stop`（"lpCursorName is a pointer to a name"）。hInstance の検査が先。
4. 識別子が 32512 以外（About Cursors の 15 個、追加カーソル、その他の数）は `fle_stop`（"lpCursorName MAKEINTRESOURCE(N) is not IDC_ARROW (32512): not answered"）。値は作らない。
5. 実装は別ファイル `fle_cursor.c/.h`。0054 の表（`fle_gdi.c`）に同居させない。理由: カーソルは状態（発行・削除・種類）を持たない純関数で、GDI オブジェクト表の slot と削除の規則は当てはまらない。0054 の `DeleteObject` は表が発行していない値で停止するので、カーソルのハンドルを DeleteObject に渡せば停止する（テスト `a_cursor_handle_is_not_a_gdi_object_or_a_dc`）。既存ファイルの変更は最小限（`imports_table.rs` に追記のみ）。
6. ホストのカーソル・テーマ・マウス設定・画面・リソース・時計・ファイルは読まない（テスト `source_uses_no_host_…`）。

## 静的に確定できた IDC_* の範囲（「まとめて受け付けてよい」条件）

7 か所のうち、静的に確定できた識別子は IDC_ARROW（4 か所）だけ。ほかの 3 か所は hInstance 非 NULL と文字列名で、静的にも確定できない（名前は 4 種のうちどれかを呼び出し元が選ぶ）。そのため受け付ける IDC_* は 1 つ。次の停止点の LoadImageW のループには別の IDC_*（SIZEALL、HAND、HELP、APPSTARTING、NO、WAIT、UPARROW、SIZEWE、SIZENWSE、SIZENS、SIZENESW、IBEAM、CROSS）が現れるが、それは LoadImageW の引数であって LoadCursorW ではない。

## 選択肢

- ハンドル領域: A. `0x7FCA0000`（採用。0054 の直後、0x80000000 未満）。B. 他の領域と同じ `0x7FFAxxxxxxxx`。**採用理由**: 0053、0054 と同じ 64 ビットのページへの判断。
- ハンドルの導出: A. `idc - 32512 + 1` から決める（採用。ただし答えるのは 1 つなので今は 0x7FCA0004 だけ）。B. 呼ぶ順に採番。**採用理由**: 同じ IDC_* が同じ値になる性質が呼び出し順に依存せず、将来 IDC_* を増やしたときも既存の値が動かない。
- 受理範囲: A. IDC_ARROW だけ（採用）。B. About Cursors の 16 個すべて。**不採用**: 値の根拠は同じだが、使われないコードは検証できない（0058 と同じ判断）。ゲストの静的な LoadCursorW は IDC_ARROW だけ。
- 非 NULL hInstance: A. 停止（採用）。B. NULL を返す（「失敗」）。**不採用**: ページは失敗する形を書かない。呼び出し元は NULL を保存して先へ進み、後で壊れる。
- 状態: A. 持たない（採用）。B. 発行済みの表。**採用理由**: 共有カーソルは破棄しない（About Cursors）ので、生死の状態が要らない。

## 撤回条件

- 別の一次資料が IDC_ARROW の数値を 32512 以外と示す（可能性はほぼ無い）。
- ゲストが HCURSOR を 32 ビットに切り詰めて使う、ポインタとして検査する、別の API（SetCursor、GetCursorInfo、GetIconInfo、DrawIcon 系）にハンドルの種類を問う形で渡す。領域や種類を見直す（特に GetIconInfo は HCURSOR で fIcon=FALSE を返すと About Cursors が書く）。
- ゲストが hInstance 非 NULL の LoadCursorW に届く（名前付き停止が出る）。リソース（`RT_GROUP_CURSOR` 57 件、`VT_*` など）の読み取りが要る。LoadResource 系の決定と一緒に決める。
- LoadImageW（IMAGE_CURSOR、LR_SHARED）に答えるとき。同じ IDC_* に対して LoadCursorW と LoadImageW が同じハンドルを返すかを決める必要が出る（About Cursors は LoadCursor が LR_SHARED を内部で使うと書き、LoadImage の LR_SHARED は「Shares the image handle if the image is loaded multiple times」と書く。同じハンドルになると読めるが、キャッシュのキー（サイズ）が異なる場合は書かれていない）。このときこの決定の領域と関数を拡張する。
- 実 Windows または Microsoft の別の文書が、同じ IDC_* で 2 回呼んだときにハンドルが変わると示す。

## 確認できていないこと

- 実 Windows が LoadCursorW で返すハンドルの値（比較していない）。
- HCURSOR のハンドルが「下位 32 ビットだけが有効」かどうか（ページは HCURSOR を挙げない）。
- ゲストが戻り値のハンドルを SetCursor 以外に渡す先の全部（`loadcursor-caller.md` §3 に読めた範囲を書いた）。
- 動的な 3 か所のどれが実行で先に届くか。
