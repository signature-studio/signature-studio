# 0054: 仮想 GDI オブジェクト表（CreatePalette、DeleteObject、GetStockObject 3 index、LoadIconW(IDI_APPLICATION)）を固定の仕様で答える

状態: 作業者（サブエージェント）が決定。依頼者（PM）の判断は含まない。**ハンドルの値、stock の index の数値、palVersion の値は仮の決定**（下の「出典の確認状況」の表で、Learn が決めるものと決めないものを分けた）。実装は `lift/runtime/fle_gdi.h`、`fle_gdi.c`、`lift/crates/fe-lift/src/imports_table.rs`（5 行追記）、テストは `lift/crates/fe-lift/tests/gdi.rs`（21 件）、調査と実行記録は `docs/investigations/gdi-objects-caller.md`。「一次」は Microsoft Learn（2026-10-21 取得）、「非公式」はそれ以外。

## 背景

harness は `fle: unsupported instruction at 0x423ff0`（`gdi32.dll!CreatePalette` の thunk）で止まっていた（`getdc-caller.md` §6）。呼び出し元 f_5c4dc0 は、静的な 16 色の表（0x13d8468、64 バイト）をスタック上の LOGPALETTE（palVersion 0x300、palNumEntries 0x10）に写して `CreatePalette` を呼び、戻り値を f_5d19e0 がグローバル 0x15f2f68 に保存する。同じ初期化 unit（f_5d2680）はこの直後に `GetStockObject(7)`、`GetStockObject(5)`、`GetStockObject(0xd)`、`LoadIconW(NULL, 0x7f00)` を呼び、戻り値をグローバル 0x15f2f78、0x15f2f80、0x15f2f88、0x15f2f90 に保存する（`gdi-objects-caller.md` §2、§3）。

## 原則

1. ホストの実画面・実リソース・実環境値は見せない（再現性）。値は仮想で、この決定で固定する。
2. ページから決まらない値は、選択肢・採用理由・撤回条件とともにここに書く。非公式の出典に基づくものは「仮の決定」と明記する。
3. ゲストが使わない stock index、LOGPALETTE の別形、icon の別の識別子、HINSTANCE が非 NULL の LoadIconW、ページが規則を書かない DeleteObject の対象は、名前付きの停止（`fle_stop`）にする。値は作らない。
4. 「ゲストが次の停止点までで使う範囲」だけを作る。SelectObject、GetTextMetricsW、GetSystemPaletteEntries、GetSysColor、DestroyIcon、CreateFont 系などは作らない（実行がまだそこに届いていない。`gdi-objects-caller.md` §3、§5）。

## 一次資料で確認した規則

| ページ | 確認した内容 |
|---|---|
| [CreatePalette](https://learn.microsoft.com/en-us/windows/win32/api/wingdi/nf-wingdi-createpalette)（`updated_at` 2024-02-22） | 引数は `const LOGPALETTE *plpal`。成功なら論理パレットのハンドル、失敗は NULL。使い終わったら `DeleteObject` で削除する。NULL、palVersion、0 個のエントリについての記述は**無い**。 |
| [LOGPALETTE](https://learn.microsoft.com/en-us/windows/win32/api/wingdi/ns-wingdi-logpalette)（2024-02-22） | `{ WORD palVersion; WORD palNumEntries; PALETTEENTRY palPalEntry[1]; }`。palVersion は「The version number of the system」（値は書かれない）。palNumEntries はエントリ数。 |
| [PALETTEENTRY](https://learn.microsoft.com/en-us/previous-versions/dd162769(v=vs.85))（アーカイブ、2021-12-02） | `{ BYTE peRed; BYTE peGreen; BYTE peBlue; BYTE peFlags; }`。peFlags は 0 か PC_EXPLICIT、PC_NOCOLLAPSE、PC_RESERVED。 |
| [DeleteObject](https://learn.microsoft.com/en-us/windows/win32/api/wingdi/nf-wingdi-deleteobject)（2024-02-22） | ペン、ブラシ、フォント、ビットマップ、リージョン、**パレット**を削除する。「After the object is deleted, the specified handle is no longer valid」。成功は非ゼロ。「If the specified handle is not valid or is currently selected into a DC, the return value is zero」。 |
| [GetStockObject](https://learn.microsoft.com/en-us/windows/win32/api/wingdi/nf-wingdi-getstockobject)（2024-02-22） | 引数 `int i` は名前の一覧（BLACK_BRUSH、NULL_BRUSH、BLACK_PEN、SYSTEM_FONT、DEFAULT_PALETTE など）で、**数値は書かれない**。成功なら論理オブジェクトのハンドル、失敗は NULL。「It is not necessary (but it is not harmful) to delete stock objects by calling DeleteObject」。同じ index を 2 回呼んだときに同じ値かは書かれない。 |
| [LoadIconW](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-loadiconw)（2025-07-01） | hInstance が NULL なら lpIconName は IDI_ で始まる定義済みシステムアイコンの識別子。成功ならアイコンのハンドル、失敗は NULL（拡張エラーは GetLastError）。「LoadIcon loads the icon resource only if it has not been loaded; otherwise, it retrieves a handle to the existing resource」。 |
| [About Icons](https://learn.microsoft.com/en-us/windows/win32/menurc/about-icons)（2026-09-21） | **IDI_APPLICATION = MAKEINTRESOURCE(32512)**（IDI_ERROR 32513 … IDI_SHIELD 32518 も）。「LoadIcon – always uses LR_SHARED internally regardless of the instance parameter」、共有ハンドルは DestroyIcon で壊さない。 |
| [Interprocess communication](https://learn.microsoft.com/en-us/windows/win32/winprog64/interprocess-communication)（2025-11-01） | 64 ビット Windows のハンドルは下位 32 ビットだけが有効で、切り詰めても符号拡張しても安全。対象に HWND と GDI オブジェクト（HBRUSH、HPEN）が挙がる（HPALETTE、HICON は挙がらない）。 |
| [OSVERSIONINFOW](https://learn.microsoft.com/en-us/windows/win32/api/winnt/ns-winnt-osversioninfow)（2024-11-20） | dwPlatformId の唯一の値は VER_PLATFORM_WIN32_NT = 2。フィールドの並びは dwOSVersionInfoSize、dwMajorVersion、dwMinorVersion、dwBuildNumber、dwPlatformId（f_5d1a60 を通らないことの根拠。今回の調査で確認した）。 |

## 出典の確認状況（値ごと）

| 項目 | 値 | 出典 | 状態 |
|---|---|---|---|
| stock index → 数値 | NULL_BRUSH 5、BLACK_PEN 7、SYSTEM_FONT 13 | Learn の GetStockObject は**名前だけ**で数値を書かない。数値は mingw-w64 の `wingdi.h`（`/usr/share/mingw-w64/include/wingdi.h` の 1441〜1462 行を読んだ: `NULL_BRUSH 5`、`BLACK_PEN 7`、`SYSTEM_FONT 13`、`STOCK_LAST 19`）だけ。今回は windows-sys、ReactOS、Wine の確認はしていない。 | **非公式。仮の決定**（独立した複数の出典での一致は未確認。前回の 0053 の index より確認が薄い） |
| IDI_APPLICATION | 32512 | **一次**: About Icons の表（MAKEINTRESOURCE(32512)）。mingw の `winuser.h` 4330 行も同じ。 | 確認済み（`getdc-caller.md` §3 は「非公式」としていた。本決定で一次に更新） |
| LOGPALETTE の配置 | palVersion 2 バイト、palNumEntries 2 バイト、エントリ 4 バイトずつ（R、G、B、flags） | 一次: LOGPALETTE と PALETTEENTRY の型定義（WORD、BYTE の並び）。 | 確認済み。パディングが無いこと（4 バイト目から始まる）は、WORD 2 つの後に BYTE 配列なので型から決まる。ゲストも `[rbp+0x3c]` に 64 バイトを写している（読み） |
| palVersion | 0x300 だけを受ける | Learn は値を書かない。ゲストが 0x300 を渡す（0x5c4dd6、読み）。 | 選択（ゲストの形に限る。**仮**） |
| palNumEntries | 16 だけを受ける | Learn は範囲を書かない。ゲストは 0x10 を渡す（0x5c4ddc、読み）。 | 選択（ゲストの形に限る。**仮**） |
| ハンドルの値 | `0x7FBA0000 + 4 * (slot + 1)`、領域 1 MiB | なし（どのページも値を書かない）。一次: 64 ビットのページ（下位 32 ビットが有効）。HPALETTE と HICON はそのページの列挙に無い。 | 選択。32 ビットに切り詰めても符号拡張しても元の値に戻る（0x80000000 未満）こと、0053 の DC 領域（`0x7FA90000` + 1 MiB）と重ならないこと、イメージ（〜0x18C0000）より上であることを条件にした。HPALETTE、HICON に同じ規則が当てはまるかは**推測** |
| stock のハンドル | slot = index（5、7、13）。同じ index は毎回同じ値 | なし。ページは同一性を書かない。 | 選択（**仮**）。ゲストは 1 回しか呼ばないので、値の同一性を必要としていない（読み） |
| icon のハンドル | slot 32。何度呼んでも同じ値 | 一次: LoadIconW「it retrieves a handle to the existing resource」、About Icons の LR_SHARED。値そのものは書かれない。 | 同一性は一次、値は選択（**仮**） |
| 作成したパレットの slot | 33 から。解放後も再利用しない | なし（ページは再利用の有無を書かない）。 | 選択（0053 の DC と同じ流儀） |
| DeleteObject(生きているパレット) | 1 | 一次: 「nonzero」。 | 確認済み。ただし「DC に選択中なら 0」を実装していない（SelectObject、SelectPalette が無いので選択される経路が無い。下の撤回条件） |
| DeleteObject(NULL) と削除済みのハンドル | 0 | 一次: 「If the specified handle is not valid … zero」。NULL が「有効でない」に当たるかはページに明記されない。 | 解釈（**仮**）。削除済みは「no longer valid」から導ける |

## 決定

1. **CreatePalette**: plpal が NULL、palVersion ≠ 0x300、palNumEntries ≠ 16 は `fle_stop`（それぞれ「plpal is NULL」、「palVersion is not 0x300」、「palNumEntries is not 16」）。通れば 16 エントリを内部へ**写す**（ゲストの LOGPALETTE はスタック上で、戻ると消える）。呼ぶたびに新しいハンドルを返す。last error は変えない（ページは変えるとは書かない）。
2. **DeleteObject**: NULL は 0。削除済みの値は 0。生きているパレットは削除して 1（ハンドルは以後無効で、再利用しない）。stock のハンドルは `fle_stop`（"stock object: the page gives no return value for deleting one"）。icon のハンドルは `fle_stop`（"icon: the pages give no rule for DeleteObject on an icon"）。この表が発行していない値（別種のハンドル、領域内の未発行の値、領域のずれた値、画面 DC のハンドル）は `fle_stop`（"handle was not issued by this runtime's GDI table"）。
3. **GetStockObject**: 5、7、13 だけ。ecx の下位 32 ビットだけを index として見る（`int`）。それ以外は `fle_stop`（"index N is not answered"）。同じ index には毎回同じ値を返す。
4. **LoadIconW**: hInstance が NULL で lpIconName が 32512（rdx の 64 ビット全体で比較。ポインタ値は名前であって識別子ではない）のときだけ。常に同じハンドル。hInstance が非 NULL は `fle_stop`（"hInstance is not NULL"）。別の識別子と名前（ポインタ）は `fle_stop`（"lpIconName is not IDI_APPLICATION"）。last error は変えない。**（0069 で追記: hInstance が NULL で lpIconName が文字列 L"MAINICON" ちょうどのときだけ NULL を返す（仮の決定）。それ以外の文字列名は従来どおり停止。上の本文は変えていない。）**
5. **スレッド**: 最初の呼び出しのスレッドだけ（0053 と同じ。`fle_host_tid()`）。
6. ホストの画面・リソース・環境・時計・ファイルは読まない（テスト `source_uses_no_host_display_resource_clock_thread_process_or_file`）。
7. 画面 DC（0053）とは別の表・別のハンドル領域。互いのハンドルは相手の API で「発行していない値」として停止する（テスト `a_gdi_handle_is_not_a_dc_for_release_dc`）。

## 選択肢

- ハンドル領域: A. `0x7FBA0000`（採用。0053 の DC 領域の直後、0x80000000 未満）。B. 他の領域と同じ `0x7FFAxxxxxxxx`。**採用理由**: 64 ビットのページに従い 32 ビット内に収める（0053 と同じ判断）。
- stock の slot: A. slot = index（採用）。B. 呼ぶ順に採番。**採用理由**: 同じ index が同じ値になる性質が構造から保証される。値からも index が読める（デバッグ）。
- icon: A. 1 つの slot（採用）。B. 呼ぶたびに新しい値。**採用理由**: LoadIconW のページが「既存のリソースのハンドルを返す」と書く。
- DeleteObject(stock): A. 停止（採用）。B. 1 を返して何もしない。C. 0 を返す。**採用理由**: ページは「削除する必要は無い、害も無い」と書くだけで戻り値を決めない。ゲストが 0x15f2f68（パレット）を削除する経路は 0x5d2622 で読んだ。stock と icon を DeleteObject に渡す経路は、135 箇所の呼び出し点のうち 2 箇所しか読んでいないので、あるかどうか分からない。分からないものに値を創作しない。
- DeleteObject(icon): A. 停止（採用）。B. 1。**採用理由**: アイコンの破棄は DestroyIcon で、共有アイコンは壊さない（About Icons）。DeleteObject への規則は無い。
- LOGPALETTE の受理範囲: A. ゲストの形（0x300、16）だけ（採用）。B. 任意の palVersion と個数。**採用理由**: 他の 8 つの呼び出し点（`gdi-objects-caller.md` §3）は個数を変数で渡す。それらが実行で届いたとき、内容を見てから決める。
- DeleteObject の「DC に選択中なら 0」: A. 実装しない（採用）。B. 選択状態を持つ。**採用理由**: SelectObject、SelectPalette を作らない（上の原則 4）ので、パレットが DC に選択される経路が無い。

**追記（決定 0066）: 0066 でフォントの種別を追加した。** `CreateFontIndirectW` を表に加え、種別「フォント」（`FLE_GDI_KIND_FONT` = 6）を足した。作成したフォントはパレットと同じ「作成された slot」の列（33 から、再利用しない）とこの決定のハンドルの式・領域（`0x7FBA0000`、1 MiB）を使い、LOGFONTW の 92 バイトを表に複製する。DeleteObject は生きているフォントを削除して 1、削除済みは 0（stock のフォントは従来どおり停止）。上の本文（原則 4 の「CreateFont 系などは作らない」を含む）は、この時点（2026-10-21）の決定として残す。値の出典と仮の度合いは 0066。

**追記（決定 0068）: 0068 で `GetObjectW` を表に加えた（作成したフォントだけ）。** 生きている作成フォント（種別 `FLE_GDI_KIND_FONT`）に `c == 0x5c` と非 NULL の `pv` で呼ぶと、複製の 92 バイトを `pv` に書いて 92 を返す（last error と戻り番地は触らない）。パレット、stock（ブラシ、ペン、フォント）、アイコン、削除済み、未発行の値、`c` が 0x5c 以外、`pv` が NULL は、形ごとの理由で `fle_stop`。上の本文（原則 4 の「SelectObject などは作らない」）はこの時点の決定として残す。`tests/gdi.rs` の「行を持たない API」の一覧にあった `GetObjectW` は 0068 で外した（理由はそのコメントに書いた）。

## 撤回条件

次のどれかが見えたら見直す。

- 別の一次資料（Windows SDK のヘッダを Learn が引用するなど）が stock index の数値を示し、5、7、13 と食い違う。
- ゲストが stock、icon、palette のハンドルを 32 ビットに切り詰めて使う、またはポインタらしい値として検査する。ハンドルの領域を変える。
- ゲストが SelectObject、SelectPalette、RealizePalette に届き、パレットや stock オブジェクトを DC に選択する（DeleteObject の「選択中は 0」を実装する必要が出る。そのときは DC 側の選択状態を設計し、この表と一緒に決める）。
- ゲストが stock、icon に DeleteObject を呼ぶ、または別の LOGPALETTE、別の IDI_、HINSTANCE 付きの LoadIconW に届く（名前付きの停止が出る。その箇所の読みに基づいて決める）。
- 実 Windows または Microsoft の別の文書が、同じ stock index・同じ IDI_ で 2 回呼んだときにハンドルが変わると示す。

## 確認できていないこと

- 実 Windows が CreatePalette、GetStockObject、LoadIconW で返すハンドルの値（比較していない）。
- stock の index の数値が、現行の Windows SDK のヘッダと同じか（mingw-w64 の 1 つのヘッダしか読んでいない）。
- palVersion の 0x300 が Windows の定める値か（ゲストが渡す値であることしか確認していない。Learn は値を書かない）。
- HPALETTE と HICON のハンドルに「下位 32 ビットだけが有効」が当てはまるか（ページは HWND、HBRUSH、HPEN を挙げる）。
- パレットを使うゲストのコード（f_5c4740、f_5c4f30 ほか）が、このパレットのハンドルをどの API に渡すか（読んでいない。`gdi-objects-caller.md` §5）。
