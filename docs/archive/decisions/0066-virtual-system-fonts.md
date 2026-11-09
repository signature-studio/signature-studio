# 0066: 仮想システムフォント（SPI_GETICONTITLELOGFONT、SPI_GETNONCLIENTMETRICS、CreateFontIndirectW）を仮の値で答える

状態: 作業者（サブエージェント）が決定。方針（選択肢 A: 仮想のシステムフォントを**仮の決定**として決めて答える。B 案「FALSE を返してゲストに `GetStockObject(13)` の経路を通らせる」は採らない）は依頼者（PM）。**フォントの値（書体名、ポイント数、太さ、文字セット、精度、品質、ピッチとファミリ）は Learn の Windows 10 の記述に無く、仮の決定**。出典を値ごとに分けた（下の表）。実装は `lift/runtime/fle_spi.h`、`fle_spi.c`（SPI の 2 つの uiAction）、`fle_gdi.h`、`fle_gdi.c`（仮想 GDI オブジェクト表 0054 に「フォント」種別を追加）、`lift/crates/fe-lift/src/imports_table.rs`（1 行追記）、テストは `lift/crates/fe-lift/tests/spi.rs`、`gdi.rs`、調査は `docs/investigations/system-fonts-caller.md`。「一次」は Microsoft Learn（2026-10-22 取得）、「非公式」はそれ以外。

## 背景

harness は `fle: SystemParametersInfoW: uiAction 0x1f (SPI_GETICONTITLELOGFONT) is used by the guest but no value is decided (decision 0058)` で止まっていた（`next-stop-spi-iconmetrics.md`）。呼び出し元は f_8663b0（0x865379 から。`docs/investigations/system-fonts-caller.md` §2）。0058 は 0x1f と 0x29 を「フォントとスクロールバー寸法をまとめて決める決定が要る」ものとして止めていた。この決定は、ゲストが実行で読む範囲（フォント 4 つ + 1）だけを決め、読まない寸法は決めない。

## 原則

1. ホストの実フォント・実設定・実レジストリを見せない。値は仮想で、この決定で固定する。
2. 一次資料に無い値は、事実としては書かない。「仮の決定、出典（非公式を含む）の区別、作業者の選択」と明記する。
3. **決めていない値を、決めた値として見せない。** 決めていないフィールドは 0 で埋めるが、これは「0 という値を決めた」のではなく「決めていない値を書かずに置く場所を 0 にした」である（理由と限界は下の「限界」）。
4. ゲストが使わない uiAction、ページが定義しない引数の形は、従来どおり名前付きで停止する（0058 の停止の表は 0x1f、0x29 を除いて保つ）。

## 一次資料で確認した規則（2026-10-22 取得）

| ページ | 確認した内容 |
|---|---|
| [SystemParametersInfoW](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-systemparametersinfow)（`updated_at` 2025-07-01） | **SPI_GETICONTITLELOGFONT 0x001F**: 「Retrieves the logical font information for the current icon-title font. The *uiParam* parameter specifies the size of a LOGFONT structure, and the *pvParam* parameter must point to the **LOGFONT** structure to fill in.」**SPI_GETNONCLIENTMETRICS 0x0029**: 「Retrieves the metrics associated with the nonclient area of nonminimized windows. The *pvParam* parameter must point to a NONCLIENTMETRICS structure that receives the information. Set the **cbSize** member of this structure and the *uiParam* parameter to `sizeof(NONCLIENTMETRICS)`.」戻り値: 成功は非 0、失敗は 0 で GetLastError。どの形が失敗かは書かれない。 |
| [NONCLIENTMETRICSW](https://learn.microsoft.com/en-us/windows/win32/api/winuser/ns-winuser-nonclientmetricsw)（`updated_at` 2024-11-20） | メンバーの並び: `UINT cbSize; int iBorderWidth, iScrollWidth, iScrollHeight, iCaptionWidth, iCaptionHeight; LOGFONTW lfCaptionFont; int iSmCaptionWidth, iSmCaptionHeight; LOGFONTW lfSmCaptionFont; int iMenuWidth, iMenuHeight; LOGFONTW lfMenuFont, lfStatusFont, lfMessageFont; int iPaddedBorderWidth`。cbSize: 「The caller must set this to sizeof(NONCLIENTMETRICS)」。iBorderWidth: 「The default is 1 pixel.」iPaddedBorderWidth: 「The default value is 4 pixels.」Remarks: iPaddedBorderWidth があると「this structure is 4 bytes larger」。lfStatusFont: 「the font used in status bars and tooltips」、lfMessageFont: 「message boxes」、lfMenuFont: 「menu bars」。**フォントの値、他の寸法の既定値は書かれない。** |
| [LOGFONTW](https://learn.microsoft.com/en-us/windows/win32/api/wingdi/ns-wingdi-logfontw)（`updated_at` 2025-08-15） | `LONG lfHeight, lfWidth, lfEscapement, lfOrientation, lfWeight; BYTE lfItalic, lfUnderline, lfStrikeOut, lfCharSet, lfOutPrecision, lfClipPrecision, lfQuality, lfPitchAndFamily; WCHAR lfFaceName[LF_FACESIZE]`（LF_FACESIZE = 32、「must not exceed 32 TCHAR values, including the terminating NULL」）。lfHeight: 「`lfHeight = -MulDiv(PointSize, GetDeviceCaps(hDC, LOGPIXELSY), 72);`」。lfWeight: 「400 is normal and 700 is bold」（FW_NORMAL 400）。lfCharSet の表に DEFAULT_CHARSET。**どのシステムフォントの値も書かれない。** |
| [CreateFontIndirectW](https://learn.microsoft.com/en-us/windows/win32/api/wingdi/nf-wingdi-createfontindirectw)（`updated_at` 2024-11-20） | `HFONT CreateFontIndirectW(const LOGFONTW *lplf)`。「If the function succeeds, the return value is a handle to a logical font. If the function fails, the return value is NULL.」**GetLastError の値は書かれない。** 「When you no longer need the font, call the DeleteObject function to delete it.」NULL の lplf、値の範囲について書かれない。フォントマッパは SelectObject で働く。 |
| [DeleteObject](https://learn.microsoft.com/en-us/windows/win32/api/wingdi/nf-wingdi-deleteobject)（`updated_at` 2024-02-22） | 「deletes a logical pen, brush, **font**, bitmap, region, or palette」。成功は非 0、「If the specified handle is not valid or is currently selected into a DC, the return value is zero.」 |
| [Fonts（UX guide）](https://learn.microsoft.com/en-us/windows/win32/uxguide/vis-fonts)（`ms.date` 2020-10-20、`updated_at` 2025-04-15） | 冒頭の注: 「This design guide was created for Windows 7 and has not been updated for newer versions of Windows.」本文: 「Segoe UI … is the Windows system font. The standard font size has been increased to 9 point.」表の見出し: 「The following fonts and colors are defaults for Windows Vista and Windows 7.」行: `CaptionFont` = 「9 pt. black (#000000) Segoe UI」、`BodyText` = 「9 pt. black Segoe UI」。「For Segoe UI, use a 9 point font size or larger.」 |
| [Typography in Windows](https://learn.microsoft.com/en-us/windows/apps/design/signature-experiences/typography)（`updated_at` 2026-07-14） | 「Segoe UI Variable is the new system font for Windows.」「Windows 11 uses Segoe UI Variable …」。**Windows 10 の記述は無い**（WinUI/XAML のアプリの話で、Win32 の NONCLIENTMETRICS のフォントではない）。 |
| [Fonts and text metrics](https://learn.microsoft.com/en-us/windows/win32/menurc/fonts-and-font-metrics) | Windows 8.1 の UI フォント表で「Latin | Segoe UI」。LOGFONT の値は書かれない。 |

**まとめ**: Learn が決めるのは、構造体の並びと大きさ、lfHeight の式、iBorderWidth = 1、iPaddedBorderWidth = 4、lfWeight の数値、そして **Windows Vista と Windows 7 の既定が Segoe UI 9 pt**（UX guide。「CaptionFont」の行と「BodyText」の行）。**Windows 10 について、Segoe UI 9 pt が NONCLIENTMETRICS の 4 フォントやアイコンタイトルフォントの既定だと書いたページは見つかっていない**（UX guide は Windows 7 向けと明記）。5 つのフォントの LOGFONTW の各フィールドの厳密な値（lfWeight、lfCharSet、lfOutPrecision、lfClipPrecision、lfQuality、lfPitchAndFamily、lfFaceName の綴り）は、どの Learn のページにも書かれていない。

## 仮想 OS と DPI との整合

- 仮想 OS は Windows 10 22H2（決定 0014）。5 つのフォントのどれについても、Windows 10 の既定値を Learn は書かないので、**Windows 7 の既定（Segoe UI 9 pt）を Windows 10 に当てはめるのは作業者の選択**（非公式の裏付けは下の「非公式」）。
- DPI は 96（決定 0053 の `FLE_GDC_LOGPIXELSY`）。lfHeight は Learn の式から導く: `-MulDiv(9, 96, 72)` = **-12**。ヘッダでは `FLE_SPI_FONT_HEIGHT` を `-(FLE_SPI_FONT_POINTS * FLE_GDC_LOGPIXELSY / 72)` と書き、-12 の第 2 のコピーを持たない。DPI が変われば高さは追従する。
- ページは「SPI_GETNONCLIENTMETRICS は DPI を考慮しない」（SystemParametersInfoW の Remarks「This API is not DPI aware」）。ゲストが DPI 依存の値を別の経路で受け取るかは追っていない。

## 5 つのフォントの値（仮の決定）

5 つのフォント（icon title、caption、status、menu、message）は**同じ LOGFONTW** にする。出典の区別: **一次**＝Learn のページが値を書く / **導出**＝Learn の式と決定 0053 から計算 / **非公式**＝Learn 以外（実 Windows の値の記述）/ **仮**＝作業者の選択（出典なし）。

| フィールド | 値 | 出典 | 区別 |
|---|---|---|---|
| lfHeight | -12 | LOGFONTW の式 `-MulDiv(PointSize, LOGPIXELSY, 72)` に PointSize = 9、LOGPIXELSY = 96（決定 0053）を代入 | **導出**（式と DPI は一次。9 pt は下の行） |
| （ポイント数） | 9 pt | UX guide: Windows Vista / Windows 7 の既定（「CaptionFont」「BodyText」の 9 pt）。**Windows 10 の記述は無い** | Windows 7 まで**一次**、Windows 10 に当てはめるのは**仮**。非公式の裏付け: Chromium の `content/shell/app/blink_test_platform_support_win.cc` が「`required_font_size = -12`」と「`L"Segoe UI"`」を `lfStatusFont`、`lfMenuFont`、`lfSmCaptionFont` に要求する（Windows の既定のテーマでの値。実 Windows 10 の記述ではなく、Chromium のレイアウトテストの前提） |
| lfWidth、lfEscapement、lfOrientation | 0 | LOGFONTW: lfWidth 0 は「the aspect ratio of the device is matched」（既定の動作）、角度 0 は回転なし | **仮**（書かれた既定値ではない。0 は「指定なし」の値としてページに意味がある） |
| lfWeight | 400（FW_NORMAL） | LOGFONTW の表: 「400 is normal」。どのフォントが 400 かは書かれない（caption は Windows 10 の実機で 400 だが、これは記憶で、出典なし） | 数値は**一次**、5 つに 400 を当てるのは**仮** |
| lfItalic、lfUnderline、lfStrikeOut | 0 | 5 つのフォントが斜体・下線・取り消し線でないとはどのページも書かない | **仮** |
| lfCharSet | 1（DEFAULT_CHARSET） | LOGFONTW の表に名前。**数値 1 は Learn のページに無い**（mingw-w64 の `wingdi.h` 1160 行 `#define DEFAULT_CHARSET 1`、非公式）。Wine の user32 は「Set font charset to DEFAULT_CHARSET to support locale change」とシステムフォントの文字セットを DEFAULT_CHARSET にしている（非公式。Wine の挙動で、実 Windows の値ではない） | 名前は一次、数値は**非公式**、選択は**仮**。ページは「To ensure consistent results when creating a font, do not specify OEM_CHARSET or DEFAULT_CHARSET」と書くが、これは新しくフォントを作るアプリへの助言で、システムの値の記述ではない |
| lfOutPrecision | 0（OUT_DEFAULT_PRECIS） | 名前は LOGFONTW の表。数値 0 は mingw `wingdi.h` 1122 行（非公式） | **仮** |
| lfClipPrecision | 0（CLIP_DEFAULT_PRECIS） | 名前は表。数値 0 は mingw `wingdi.h` 1134 行（非公式） | **仮** |
| lfQuality | 0（DEFAULT_QUALITY） | 名前は表。数値 0 は mingw `wingdi.h` 1145 行（非公式）。**実 Windows 10 では 5（CLEARTYPE_QUALITY）などの値が返ることがあるが、これは記憶で、出典なし**。品質はゲストが SelectObject でフォントを使うときの描画に効くが、この環境はまだ描画しない | **仮**（0 は「appearance does not matter」で、最も情報を持たない値） |
| lfPitchAndFamily | 0（DEFAULT_PITCH \| FF_DONTCARE） | 名前は LOGFONTW の本文。数値は mingw `wingdi.h` 1154、1198 行（非公式）。実 Windows 10 の Segoe UI は VARIABLE_PITCH \| FF_SWISS（0x22）と記憶するが、出典なし | **仮**（0 は「Use default font」で、情報を持たない値） |
| lfFaceName | `Segoe UI` + NUL（32 文字の残りは 0） | UX guide: 「Segoe UI … is the Windows system font」（Windows 7 向け）、Fonts and text metrics: Windows 8.1 の Latin の UI フォントは Segoe UI。Typography in Windows は Windows 11 で「Segoe UI Variable」を「the new system font」と書く。**Windows 10 の NONCLIENTMETRICS の書体名は書かれない** | 綴りは Windows 7 / 8.1 で**一次**、Windows 10 に当てはめるのは**仮**（Windows 11 の「Segoe UI Variable」にはしない: 仮想 OS は Windows 10 22H2） |

- **0067 で更新（2026-10-22 追記。上の本文は消さない）**: lfQuality は 0 から **5（CLEARTYPE_QUALITY）** に更新した（非公式、仮。出典は 0067 の表: 実 Windows 10 のレジストリ export 6 件が一致）。lfPitchAndFamily は **0 のまま**（PM の仮説 0x22 は裏付けが見つからず、同じ export のバイト列は 0）。lfCharSet 1、lfOutPrecision 0、lfClipPrecision 0、9 pt（-12）、`Segoe UI`、lfWeight 400 には Windows 10 の export という非公式の裏付けが付いた。上の表の lfQuality の行と lfPitchAndFamily の行の「記憶で、出典なし」は、0067 で出典が付いた（lfPitchAndFamily の「0x22 と記憶」は**誤りの可能性が高い**）。実装は `FLE_SPI_FONT_QUALITY` = 5。
- 5 つを同じにする理由と、実 Windows との違い: 実 Windows では caption、menu、status、message、icon title が同じとは限らない（例: ユーザーの設定。実 Windows 10 の caption が太字でないこと、message が 9 pt であることなどは記憶で出典なし）。ゲストが 5 つの違いを読むかは動的には追っていない。**違いを区別する根拠が無いので同じにした**（仮）。

### 他のフィールド

| フィールド | 値 | 出典・区別 |
|---|---|---|
| iBorderWidth（+0x04） | 1 | **一次**（「The default is 1 pixel」）。ゲストは読まない（読み）が、一次の値があるので書く |
| iPaddedBorderWidth（+0x1f4） | 4（cbSize が 0x1f8 のときだけ書く。0x1f4 のときは触らない） | **一次**（「The default value is 4 pixels」）。0x1f4 は「iPaddedBorderWidth を持たない構造体」（Remarks の「4 bytes larger」と、`NONCLIENTMETRICS` の定義の差）なので、その 4 バイトは構造体の外 |
| iScrollWidth、iScrollHeight、iCaptionWidth、iCaptionHeight（+0x08..+0x17）、iSmCaptionWidth/Height（+0x74..+0x7b）、iMenuWidth/Height（+0xd8..+0xdf）、lfSmCaptionFont（+0x7c..+0xd7） | **決めない。0 で埋める** | 一次の値が無い。ゲストが読まない（読み。f_8663b0 の範囲では読まれず、f_840020 は +0xd8、+0xdc の `iMenuWidth`、`iMenuHeight` だけを読む。下の「ゲストの読み手」）。**0 は「0 という値を決めた」ではなく、書かずに置く場所を 0 にしたもの**（スタックの残骸を読ませないため） |

## 引数の形（決定）

### SPI_GETICONTITLELOGFONT（0x1f）

- ページ: 「The uiParam parameter specifies the size of a LOGFONT structure, and the pvParam parameter must point to the LOGFONT structure to fill in.」ゲストは `uiParam = 0x5c`（`mov edx,[rbp+0x288]`、0x866405 で `0x5c` を格納）、`pvParam = &lf`、`fWinIni = 0`（`xor r9,r9`）。
- **受ける形**: `uiParam == 0x5c`（= sizeof(LOGFONTW) = 92。LOGFONTW の定義から計算）、`pvParam != NULL`、`fWinIni == 0`。戻り値 1。last error に触れない。92 バイトを書く（上の表のフォント）。
- **停止**: `uiParam != 0x5c`（`SPI_GETICONTITLELOGFONT with uiParam not sizeof(LOGFONTW) = 0x5c`）、`pvParam == NULL`（`… pvParam NULL`）、`fWinIni != 0`（`… fWinIni not zero`）。ページは「どの形が失敗か」を書かないので、FALSE を返さず停止にする（0058 の規則）。

### SPI_GETNONCLIENTMETRICS（0x29）

- ページ: 「Set the cbSize member of this structure and the uiParam parameter to sizeof(NONCLIENTMETRICS)」。
- **ゲストの実際の引数（読みと、`tests/spi.rs` のバイト列固定）**: 3 か所の呼び出しは**どれも `xor edx,edx`（uiParam = 0）**: 0x866485（f_8663b0。cbSize は `f_425a50` の戻り値 0x1f8 / 0x1f4）、0x840037（f_840020。同じ）、0x991d2f（f_991cc0。cbSize はイミディエート 0x1f8）。つまり**ゲストはページの規則（uiParam = sizeof）に従っておらず、uiParam = 0 を渡す**。
- **ページは uiParam = 0 が失敗か成功かを書かない**（Learn で確認した文言: 「Set the cbSize member … and the uiParam parameter to sizeof(NONCLIENTMETRICS)」だけ。0 を渡した場合の結果は書かれない）。実機で 0 が成功するかは出典が無い（非公式の記述では、ゲスト以外の多くのコードが `uiParam = 0` で呼ぶ、と読めるが、確認していない）。
- **選択**: 「**ゲストの形（uiParam = 0）をそのまま受け付け、ページの形（uiParam = cbSize）も受け付ける。それ以外は停止**」。理由: 3 か所すべてが 0 を渡すので、停止にするとゲストが先へ進めない。この「0 を受ける」は**ゲストの形だけを名前付きで受ける**もので、ページが 0 を許すとは言わない。0 と cbSize 以外は、ページもゲストも使わない形なので停止する。
- **受ける形**: `pvParam != NULL`、`fWinIni == 0`、`pvParam->cbSize ∈ {0x1f8, 0x1f4}`、`uiParam ∈ {0, cbSize}`。戻り値 1。last error に触れない。cbSize は書き換えない（ゲストが設定した値のまま）。
- **停止**: `pvParam == NULL`、`fWinIni != 0`、cbSize が 0x1f8 / 0x1f4 以外（ページは cbSize を sizeof(NONCLIENTMETRICS) にせよと書き、それ以外の値の結果を書かない。Windows XP / 2000 向けの 0x1f4 は Remarks が書く）、uiParam が 0 でも cbSize でもない。
- **書くもの**: iBorderWidth = 1、4 つのフォント（lfCaptionFont +0x18、lfMenuFont +0xe0、lfStatusFont +0x13c、lfMessageFont +0x198）、cbSize が 0x1f8 のときだけ iPaddedBorderWidth = 4。決めない領域は 0 で埋める。cbSize の 4 バイトと、構造体の外（0x1f4 のとき +0x1f4 以降）は書かない。
- 0x1f8 と 0x1f4 の寸法: 4 + 5×4 + 92 + 2×4 + 92 + 2×4 + 3×92 = 500、iPaddedBorderWidth を足して 504（ページの定義から計算）。

## CreateFontIndirectW（仮想 GDI オブジェクト表 0054 の拡張）

- IAT スロット 0x19719d0（gdi32.dll!CreateFontIndirectW。DLL のインポート表で確認）。thunk は 0x423fc0（`jmp [rip+0x154da0a]` = 0x19719d0）。呼び出し点は 7 か所。
- 表（`fle_gdi.c`）に種別「フォント」（`FLE_GDI_KIND_FONT` = 6）を追加。作成したフォントは**パレットと同じ「作成された slot」の列**（33 から、再利用しない）を使い、ハンドルは 0054 の式 `FLE_GDI_HANDLE_BASE + 4 * (slot + 1)`（0x7FBA0000 の領域、1 MiB）。**新しい領域は作らない**ので、0053/0054/0059/0061/0065 の領域と重ならないことは、領域の一覧（gdc、gdi、cursor、loadimage、monitor）が互いに素であることのテスト（`the_font_and_the_other_tables_use_regions_that_do_not_overlap`）で固定した。
- LOGFONTW（92 バイト）を表に**複製**して保持する（後で GetObjectW、SelectObject が来たときのため。今は使わない）。検査用に `fle_gdi_font_live_count` と `fle_gdi_font_logfont`（テストと道具用。状態を動かさない）。
- 呼ぶたびに新しいハンドル（Learn は同一性を書かないが、0054 の CreatePalette と同じ選択）。last error に触れない（失敗しない呼び出しなので）。
- `lplf == NULL`: 名前付き停止（`lplf is NULL: the page does not define this call`）。Learn は「失敗は NULL」と書くが、NULL の lplf が失敗かどうか、GetLastError の値は書かない。
- 内容の検査はしない（空の書体名、終端なし、高さ 0、未知の書体名など）。ページはこれらを拒否するとは書かず、フォントマッパは SelectObject で働く（この環境は選択しない）。
- **DeleteObject（0054 を拡張）**: 生きているフォント → 削除して 1（ページがフォントを列挙し、成功は非 0）。削除済みは 0（「no longer valid」）。DC に選択中は 0 だが、選択は未実装なので起こらない。stock のフォント（`GetStockObject(13)`）は従来どおり停止（ページが stock の戻り値を書かない）。作成したフォントと stock のフォントは種別が違う（`FLE_GDI_KIND_FONT` と `FLE_GDI_KIND_STOCK_FONT`）。
- スレッド規則（最初の呼び出しのスレッドだけ）は 0054 と同じ。
- **0054 の本文は変更しない**。0054 の該当節に「0066 でフォントの種別を追加」と追記する。

## ゲストの読み手（NONCLIENTMETRICS のどのフィールドが読まれるか）

詳細は `system-fonts-caller.md` §3。要点: ncm を SPI に渡す関数は 3 つで、それぞれの読みは次のとおり。

| 関数 | cbSize | 読むフィールド（静的な読み） |
|---|---|---|
| f_8663b0（0x8663b0〜0x86660e。呼ぶのは 0x865374 と 0x868a37） | `f_425a50`（0x1f8 / 0x1f4） | lfStatusFont（`[rbp+0x170]`）、lfMenuFont（`[rbp+0x114]`）、lfMessageFont（`[rbp+0x1cc]`）、lfCaptionFont（`[rbp+0x4c]`）。ほかは読まない |
| f_840020（0x840020〜0x840075。`f_840080` が 0x8400e0 で呼ぶ。`f_840080` のアドレスは .text 内の 2 か所の表（VA 0x725b10 と 0x838cc8。ファイルオフセット 0x324f10 と 0x4380c8）にだけ現れ、直接 call は無い） | `f_425a50` | **iMenuWidth（`[rbp+0x100]` = ncm+0xd8）、iMenuHeight（`[rbp+0x104]` = ncm+0xdc）**。フォントは読まない |
| f_991cc0（0x991cc0〜0x991f6f。呼び元 0x9c89fd） | 0x1f8 | lfMessageFont（`[rbp+0x1d8]` = ncm+0x198）だけ |

**前の作業者の読み（「ゲストが読むのはフォント 4 つだけで寸法は読まない」）は、f_8663b0 だけの読みなら正しいが、f_840020 は iMenuWidth と iMenuHeight を読む**（0x840055 `mov ecx,[rbp+0x100]`、0x840064 `mov ecx,[rbp+0x104]`。ncm は `[rbp+0x28]` から始まるので +0xd8、+0xdc）。f_840020 はまだ実行に届いていない（呼び元の f_840080 は vtable 経由。gdb で f_840080、f_840020、f_991cc0 にブレークを置いたが、停止の時点まで一度も当たらなかった）。**この 2 つのフィールドを 0 のまま渡すと、実行がそこに届いたとき、ゲストは iMenuWidth = iMenuHeight = 0 を読む。止める手段がない**（下の「限界」）。

## 選択肢（全体）

- **A. フォントを仮の値で答える（採用。PM の決定）**。
- B. FALSE を返してゲストを `GetStockObject(13)` の経路に進ませる。**不採用（PM）**: 実 Windows でこの 2 つの SPI は失敗しないので、実機で通らない経路をゲストに通らせる。
- C. 停止のまま。不採用（A を PM が選んだ）。
- フォント名: A. `Segoe UI`（採用。Windows Vista / 7 / 8.1 の Learn の記述、仮想 OS は Windows 10）。B. `Segoe UI Variable`（Windows 11 の Learn の記述。仮想 OS と合わない）。C. `MS Shell Dlg` / `Tahoma`（Windows XP / 2000 向けの記述）。
- 5 つのフォントの区別: A. 全部同じ（採用）。B. caption だけ太字などにする（実 Windows の記憶。出典なし）。
- 決めない寸法: A. 0 で埋める（採用。決めない値の置き場所）。B. 0057 の GetSystemMetrics の値と合わせる（SM_CXVSCROLL = 17 などは 0057 が停止にしている 22 index で、まだ決まっていない）。C. 停止（ゲストが先へ進めない）。
- NCM の uiParam: A. 0 と cbSize を受ける（採用）。B. cbSize だけ（ゲストの 3 か所すべてが 0 なので先へ進めない）。C. 何でも受ける（ページもゲストも使わない形を黙って通す）。

## 撤回条件

- ゲストが iScrollWidth、iCaptionHeight、iMenuWidth、iMenuHeight など、0 で埋めた領域を読む箇所に届いた（f_840020 の iMenuWidth / iMenuHeight は読むと静的に分かっている）。そのときは 0057 の SM_* と一体で寸法を決める。
- ゲストが lfWeight、lfQuality、lfPitchAndFamily、書体名の違いに依存する（GetObjectW でフォントの内容を読んで分岐する、フォントの太さで幅を決めるなど）。次の停止点は GetObjectW（`next-stop-getobjectw.md`）で、LOGFONTW の内容を読む。
- Microsoft の文書が、Windows 10 のシステムフォントの LOGFONT の値を示した。
- 決定 0053 の DPI が 96 から変わった（lfHeight が追従する）。
- 実 Windows の NONCLIENTMETRICS との比較ができるようになった。

## 限界

- フォントの値は**どれも仮**（Windows 10 の記述は無い）。実 Windows 10 の値と同じかは確認していない。
- **0 で埋めた領域をゲストが読む証拠が出ても止める手段がない**: 構造体のフィールドは普通のメモリで、読み出しは検査できない。0 で埋めた領域をゲストが読んだかは、実行が分岐の結果として異なる経路に進むまで分からない。f_840020 の iMenuWidth / iMenuHeight は静的に読まれると分かっている。
- NONCLIENTMETRICS の sizeof（504 / 500）は Learn の構造体定義からの計算で、ページには数値が無い（ゲストの 0x1f8 / 0x1f4 と一致する）。
- ncm の読み手の走査は、`[rbp+...]` のオフセットを持つ関数 3 つの静的な読みで、ncm のアドレスを別の関数へ渡す形（`lea` の後の `call`）は見つけていない。間接呼び出しの先は見ていない。
- 0x1f と 0x29 の uiParam / cbSize の規則は、ゲストの 3 + 1 か所の形しか見ていない。ほかの呼び出し元（f_8663b0 を呼ぶ 0x868a37 など）は同じ関数なので同じ形。
- ゲストの `f_5c1360`、`f_5c14d0`、`f_5c1030`、`f_86b240` の中身は、GetObjectW を呼ぶ f_5c0bc0（f_5c1360 から。次の停止点）以外は追っていない。
