# システムフォントの呼び出し元（SPI_GETICONTITLELOGFONT、SPI_GETNONCLIENTMETRICS、CreateFontIndirectW）

実行日: 2026-10-22。作業ディレクトリ `signature-studio-develop`、HEAD d660dc5 に決定 0066 の変更を載せた状態。決定は `docs/decisions/0066-virtual-system-fonts.md`。前の作業者の調査 `next-stop-spi-iconmetrics.md` と重複する事実（f_8663b0 の流れ、停止点 1）は参照だけにし、ここでは**確認し直した点と食い違った点**を書く。「一次」は Microsoft Learn（2026-10-22 取得）、「実測」はコマンドの出力、「読み」は `lift/objdump-text.txt` の静的な読み、「非公式」は Learn 以外、「仮」は作業者の選択。成果物は `lift/out-sf/`（gitignore 済み）。

## 1. Learn の取得結果（取得日 2026-10-22）

| ページ（URL） | `updated_at` | 確認した文言（引用） |
|---|---|---|
| [NONCLIENTMETRICSW](https://learn.microsoft.com/en-us/windows/win32/api/winuser/ns-winuser-nonclientmetricsw) | 2024-11-20 | メンバーの並び（cbSize、iBorderWidth、iScrollWidth、iScrollHeight、iCaptionWidth、iCaptionHeight、lfCaptionFont、iSmCaptionWidth、iSmCaptionHeight、lfSmCaptionFont、iMenuWidth、iMenuHeight、lfMenuFont、lfStatusFont、lfMessageFont、iPaddedBorderWidth）。「The caller must set this to sizeof(NONCLIENTMETRICS).」「iBorderWidth: … The default is 1 pixel.」「iPaddedBorderWidth: … The default value is 4 pixels.」Remarks: 「If the iPaddedBorderWidth member … is present, this structure is 4 bytes larger」。lfStatusFont は「status bars and tooltips」、lfMessageFont は「message boxes」、lfMenuFont は「menu bars」。**フォントの値と、iBorderWidth、iPaddedBorderWidth 以外の寸法の既定値は書かれていない。** |
| [LOGFONTW](https://learn.microsoft.com/en-us/windows/win32/api/wingdi/ns-wingdi-logfontw) | 2025-08-15 | `LONG × 5; BYTE × 8; WCHAR lfFaceName[LF_FACESIZE]`。「lfHeight = -MulDiv(PointSize, GetDeviceCaps(hDC, LOGPIXELSY), 72);」。lfWeight: 「400 is normal and 700 is bold」。lfFaceName: 「must not exceed 32 TCHAR values, including the terminating NULL」。**システムフォントの値は書かれていない。** |
| [SystemParametersInfoW](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-systemparametersinfow) | 2025-07-01 | 0x001F: 「The uiParam parameter specifies the size of a LOGFONT structure, and the pvParam parameter must point to the LOGFONT structure to fill in.」0x0029: 「The pvParam parameter must point to a NONCLIENTMETRICS structure … Set the cbSize member of this structure and the uiParam parameter to sizeof(NONCLIENTMETRICS).」戻り値: 「If the function fails, the return value is zero. To get extended error information, call GetLastError.」**どの形が失敗かは書かれていない。uiParam = 0 を NCM に渡した場合の結果も書かれていない。** |
| [CreateFontIndirectW](https://learn.microsoft.com/en-us/windows/win32/api/wingdi/nf-wingdi-createfontindirectw) | 2024-11-20 | 「If the function succeeds, the return value is a handle to a logical font. If the function fails, the return value is NULL.」GetLastError の値は書かれていない。「When you no longer need the font, call the DeleteObject function」。 |
| [DeleteObject](https://learn.microsoft.com/en-us/windows/win32/api/wingdi/nf-wingdi-deleteobject) | 2024-02-22 | 「deletes a logical pen, brush, font, bitmap, region, or palette」。「If the specified handle is not valid or is currently selected into a DC, the return value is zero.」 |
| [Fonts（UX guide）](https://learn.microsoft.com/en-us/windows/win32/uxguide/vis-fonts) | 2025-04-15（`ms.date` 2020-10-20） | 冒頭の注: 「This design guide was created for Windows 7 and has not been updated for newer versions of Windows.」本文: 「Segoe UI … is the Windows system font. The standard font size has been increased to 9 point.」表: 「The following fonts and colors are defaults for Windows Vista and Windows 7.」`CaptionFont` = 「9 pt. black (#000000) Segoe UI」、`BodyText` = 「9 pt. black (#000000) Segoe UI」、「For Segoe UI, use a 9 point font size or larger.」 |
| [Typography in Windows](https://learn.microsoft.com/en-us/windows/apps/design/signature-experiences/typography) | 2026-07-14 | 「Segoe UI Variable is the new system font for Windows.」「Windows 11 uses Segoe UI Variable …」。Windows 10 の Win32 の LOGFONT の記述は無い。 |
| [Fonts and text metrics](https://learn.microsoft.com/en-us/windows/win32/menurc/fonts-and-font-metrics) | 2025-03-11 | 「Here's a list of the UI fonts shipped with Windows 8.1」の表: Latin = Segoe UI。LOGFONT の値は無い。 |
| [GetObjectW](https://learn.microsoft.com/en-us/windows/win32/api/wingdi/nf-wingdi-getobjectw)（次の停止点の確認だけ） | 2024-11-20 | 「If hgdiobj is a handle to a font, the LOGFONT that is returned is the LOGFONT used to create the font.」 |

**結論（確定できたこと）**:

- 構造体の寸法: LOGFONTW = 5×4 + 8×1 + 32×2 = **92 = 0x5c**。NONCLIENTMETRICSW = 4 + 5×4 + 92 + 2×4 + 92 + 2×4 + 3×92 + 4 = **504 = 0x1f8**（iPaddedBorderWidth なしは **500 = 0x1f4**）。オフセット: lfCaptionFont **0x18**、lfSmCaptionFont **0x7c**、lfMenuFont **0xe0**、lfStatusFont **0x13c**、lfMessageFont **0x198**、iPaddedBorderWidth **0x1f4**（Python で再計算して前の作業者の値と一致を確認した。ページには 504 / 500 の数値は無く、ゲストの 0x1f8 / 0x1f4 と合うことは計算から）。
- **Learn にシステムフォントの値は書かれていない**（Windows 10 について。NCM の 4 フォントも、アイコンタイトルのフォントも）。一般的な記述として Learn が書くのは Windows Vista / 7 の既定（Segoe UI 9 pt、UX guide。Windows 7 向けと明記）、Windows 8.1 の Latin の UI フォント（Segoe UI）、Windows 11 のシステムフォント（Segoe UI Variable）。Windows 10 の LOGFONT のフィールド（lfWeight、lfCharSet、lfOutPrecision、lfClipPrecision、lfQuality、lfPitchAndFamily の厳密な値、lfFaceName の綴り）はどのページにも無い。これらは決定 0066 で**仮**として分けて書いた。

## 2. ゲストの引数の確認（uiParam）

3 つの NCM 呼び出しはどれも `uiParam = 0`（`xor edx,edx`）。`tests/spi.rs::the_guests_font_calls_are_the_forms_the_runtime_accepts` が DLL のバイト列で固定する:

| 呼び出し | 関数 | cbSize の出どころ | 命令（バイト列） |
|---|---|---|---|
| 0x866493 | f_8663b0 | `f_425a50`（`GetVersionExW` の major ≥ 6 なら 0x1f8、そうでなければ 0x1f4。決定 0014 は Windows 10 なので 0x1f8） | 0x866485: `b9 29 00 00 00`（ecx = 0x29）、`33 d2`（edx = 0）、`4c 8d 45 34`（r8 = rbp+0x34）、`4d 33 c9`（r9 = 0） |
| 0x840045 | f_840020 | `f_425a50`（同上） | 0x840037: 同じ形、r8 = rbp+0x28 |
| 0x991d3d | f_991cc0 | イミディエート 0x1f8（0x991d28: `c7 45 40 f8 01 00 00`） | 0x991d2f: 同じ形、r8 = rbp+0x40 |

アイコンタイトルの呼び出し（0x866424）は `uiParam = [rbp+0x288]`（0x866405 で `c7 85 88 02 00 00 5c 00 00 00` = 0x5c を格納）、r8 = rbp+0x22c、fWinIni = 0。ページの規則（「uiParam は LOGFONT のサイズ」）に合う。

**食い違い**: 前の作業者の記述（`SPI(0x29, 0, &ncm, 0)`）は正しい。ページの規則（「uiParam を sizeof(NONCLIENTMETRICS) にせよ」）と**ゲストは食い違う**（ゲストは 0 を渡す）。ページは uiParam = 0 が成功か失敗かを書かない（§1）。決定 0066 は「ゲストの形（uiParam = 0）とページの形（uiParam = cbSize）を受け、それ以外は停止」とした。この選択は、ページが 0 を許すと言うものではなく、3 か所すべてが 0 を渡すので停止にするとゲストが先へ進めないための選択で、決定に理由を書いた。

## 3. ncm の読み手の静的走査

方法: ncm のバッファを SPI に渡す 3 つの関数について、`[rbp+X]` のオフセットを全て抜き出し（`grep -o "rbp+0x[0-9a-f]*" | sort | uniq -c`）、ncm の範囲に入るものの読み書きの命令を読んだ。ncm のアドレスを他の関数へ渡す形（`lea` の結果を `call` へ渡す）の有無も見た。

### 3.1 f_8663b0（0x8663b0〜0x86660e。呼び元は 0x865374 と 0x868a37）

ncm = `[rbp+0x34]`（cbSize を 0x866482 で書く）。ncm の範囲（0x34〜0x22c）の `[rbp+...]` は次の 6 つだけ（実測）: `0x34`（cbSize の書き込みと SPI へ渡す lea）、`0x4c`、`0x114`、`0x170`、`0x1cc`（4 つとも `lea rcx,[...]` で `CreateFontIndirectW` に渡す）、`0x22c`（アイコンタイトルの LOGFONT。SPI に渡す lea と CreateFontIndirectW に渡す lea）。

| 引数 | ncm でのオフセット | フォント |
|---|---|---|
| `[rbp+0x4c]` | 0x18 | lfCaptionFont |
| `[rbp+0x114]` | 0xe0 | lfMenuFont |
| `[rbp+0x170]` | 0x13c | lfStatusFont |
| `[rbp+0x1cc]` | 0x198 | lfMessageFont |

**この関数は寸法、lfSmCaptionFont、iPaddedBorderWidth を読まない**（前の作業者の読みを確認した）。

### 3.2 f_991cc0（0x991cc0〜0x991f6f。呼び元 0x9c89fd）

ncm = `[rbp+0x40]`（cbSize = 0x1f8 を 0x991d28 で書く）。ncm の範囲の `[rbp+...]`: `0x40`（store と SPI の lea）、`0x1d8`（= ncm+0x198 = **lfMessageFont**。`lea rcx` → `CreateFontIndirectW` → `[rcx+0xb4]` へ `f_5c1360`）だけ。lfMessageFont 以外は読まない（読み）。

### 3.3 f_840020（0x840020〜0x840075。呼ぶのは 0x8400e0 の f_840080）

ncm = `[rbp+0x28]`（cbSize を 0x840034 で書く）。ncm の範囲の `[rbp+...]`: `0x28`、`0x100`、`0x104`。**`mov ecx,[rbp+0x100]`（0x840055）と `mov ecx,[rbp+0x104]`（0x840064）で ncm+0xd8 = `iMenuWidth` と ncm+0xdc = `iMenuHeight` を読み**、`[rbx+0xb0]` と `[rbx+0xb8]` が指す DWORD に書く。**フォントは読まない。**

**前の作業者の読みと食い違う**: 「ゲストが 0x29 の寸法を読まない」は f_8663b0 だけの読みで、**f_840020 は iMenuWidth と iMenuHeight を読む**。ただし f_840020 は実行にまだ届いていない（f_840080 は直接 call が無い: そのアドレスは `.text` 内の 2 か所の表（VA 0x725b10 と 0x838cc8 の、ファイルオフセット 0x324f10 と 0x4380c8）にだけ現れる（実測。DLL のバイト列の探索）。gdb（`out-sf/gdb-840080.cmd`、`out-sf/gdb-840080.txt`）で f_840080、f_840020、f_991cc0 にブレークを置いて実行したが、GetObjectW の停止まで一度も当たらなかった）。

### 3.4 ncm を別の関数へ渡す形

3 つとも、ncm のアドレスを `lea` するのは SPI の引数（`lea r8`）だけで、他の関数の引数（`lea rcx|rdx|r8|r9` + `call`）として渡す形は無い（上の `[rbp+...]` の一覧に、SPI 以外で ncm の先頭付近の `lea` は無い）。ncm を保存するポインタ（`[rbp+0x28]` などへ `mov` する）は f_8663b0 の `[rbp+0x28] = rbp` と f_991cc0 の `[rbp+0x28] = rbp` があるが、これはフレームの自己ポインタ（`mov [rbp+0x28], rbp` はこのコンパイラの標準の形）で ncm を指さない。

### 3.5 まとめと限界

| ncm のフィールド | 読む関数 | 状態 |
|---|---|---|
| lfCaptionFont、lfMenuFont、lfStatusFont | f_8663b0 | 決めた値を書く（今回の停止の先で読まれる） |
| lfMessageFont | f_8663b0、f_991cc0 | 同上 |
| **iMenuWidth、iMenuHeight** | **f_840020** | **0 で埋めている。静的には読まれる。実行にまだ届いていない** |
| iScrollWidth、iScrollHeight、iCaptionWidth、iCaptionHeight、iSmCaptionWidth/Height、lfSmCaptionFont | 見つけていない | 0 で埋めている |
| iBorderWidth、iPaddedBorderWidth | 見つけていない | 一次の既定値（1、4）を書く |

限界: (a) 走査は 3 つの関数の `[rbp+...]` だけ。f_8663b0 が `f_5c1360` などを呼ぶ先でフォントのコピーを読むのは ncm ではなく CreateFontIndirectW が返したハンドル。(b) 0 で埋めた領域をゲストが読む証拠が出ても止める手段がない（決定 0066 の限界）。f_840020 が実行に届いたとき、iMenuWidth と iMenuHeight は 0 になる。(c) 間接呼び出し先（vtable）は追っていない。

## 4. CreateFontIndirectW の確認（作業内容 3）

- IAT スロット 0x19719d0 = gdi32.dll!CreateFontIndirectW（DLL のインポート表を Python で読んで確認。同じ DLL の `CreateFontW` はスロット 0x1971ab8）。thunk 0x423fc0（`ff 25 0a da 54 01` = `jmp [rip+0x154da0a]` = 0x19719d0。実測）。`call 0x423fc0` は 7 か所（0x5c1296、0x866434、0x8664a7、0x8664c9、0x8664eb、0x86650a、0x991d4d）。
- ハンドル領域は 0054 の `0x7FBA0000`（1 MiB）。各領域（gdc 0x7FA90000、gdi 0x7FBA0000、cursor 0x7FCA0000、loadimage 0x7FDA0000、monitor 0x7FEA0000。いずれも 1 MiB）は互いに素で、全部 0x80000000 未満（`the_font_and_the_other_tables_use_regions_that_do_not_overlap`）。新しい領域は作っていない。

## 5. 決めた値の表と出典の区別

決定 0066 の表を参照（重複を避ける）。要約:

| 項目 | 値 | 区別 |
|---|---|---|
| 構造体の寸法・オフセット | 0x5c、0x1f8、0x1f4、0x18 / 0x7c / 0xe0 / 0x13c / 0x198 / 0x1f4 | 一次（型）+ 計算 |
| iBorderWidth、iPaddedBorderWidth | 1、4 | **一次**（既定値） |
| lfHeight | -12 | **導出**（一次の式 + 決定 0053 の DPI 96 + 9 pt） |
| ポイント数 9、書体名 Segoe UI | 9 pt、`Segoe UI` | Windows Vista / 7 は**一次**、Windows 10 に当てはめるのは**仮**。非公式の裏付け: Chromium のレイアウトテストが `-12` と `L"Segoe UI"` を `lfStatusFont`、`lfMenuFont`、`lfSmCaptionFont` に要求（`content/shell/app/blink_test_platform_support_win.cc`。実機の記述ではなくテストの前提） |
| lfWeight | 400 | 数値は一次、当てはめは**仮** |
| lfCharSet | 1（DEFAULT_CHARSET） | 名前は一次、数値は**非公式**（mingw-w64 `wingdi.h`）。Wine がシステムフォントの文字セットを DEFAULT_CHARSET にしている（非公式。Wine の挙動） |
| lfOutPrecision、lfClipPrecision、lfQuality、lfPitchAndFamily | 0、0、0、0 | **仮**（「情報を持たない値」）。実 Windows 10 で 5（CLEARTYPE_QUALITY）や 0x22 になるという記憶は出典なし |
| lfWidth、lfEscapement、lfOrientation、lfItalic、lfUnderline、lfStrikeOut | 0 | **仮** |
| 決めない寸法、lfSmCaptionFont | 0 で埋める | 決めていない値の置き場所（値の決定ではない） |
| アイコンタイトルフォント | 他の 4 つと同じ LOGFONTW | **仮** |

実測（再実行: `tests/spi.rs` の `the_font_constants_are_the_decisions` が表を固定）。

## 6. 実測（テストと再実行）

- `tests/spi.rs` 32 件、`tests/gdi.rs` 32 件（`cargo test --offline -p fe-lift --test spi|gdi`）。変異: spi 44 件、gdi 19 件、すべて検出（`out-sf/mut/run_mut.py`、`out-sf/mut/summary-spi.txt`、`summary-gdi.txt`。同値変異は作っていない: 各変異は 1 か所の文字列置換で、置換が 0 件または複数件に当たる場合は ERROR として数えず、生き残りは 0）。
- ワークスペース全体（`cargo test --workspace --no-fail-fast --offline`、`out-sf/test-ws.txt`）: **83 バイナリ、918 件、失敗 0**（直前の基準 83 バイナリ・896 件。差 +22 は spi の +11（HEAD の 21 件 → 32 件）と gdi の +11（HEAD の 21 件 → 32 件）。`#[test]` の数は `git show HEAD:…` と現在のファイルで数えた。集計は `awk '/^test result:/'`）。
- a4-survey（`OUT=lift/out-sf WASM=1 bash lift/harness/a4-survey.sh`、ログ `out-sf/a4/20261022T013823Z-*`）: init_count 115 / 942、最後の unit index 114（proc 0x623840、in_progress）、started 111 / completed 110、関数実行数 606（distinct）、入口数 13545。**native と wasm は rsp= の行（`calling entry … rsp=` の 1 行）を除いて完全一致**（`diff <(grep -v rsp= native) <(grep -v rsp= node)` が空）。
- 次の停止点: `fle: unsupported instruction at 0x424280`（gdi32!GetObjectW の thunk、IAT スロット 0x19719f0）。スタック: f_424280 ← f_5c0bc0 ← f_5c1360 ← f_8663b0（0x865379）← f_8651a0 ← f_620cd0 ← f_623840 ← …。判断材料は `next-stop-getobjectw.md`（実装していない）。
- clang `-Werror` と emcc `-Werror` は `fle_spi.c`、`fle_gdi.c` で警告なし。全 runtime の emcc ビルド（`out-sf/log-base/rt-summary.txt`）で `fle_proc.c` に警告が 1 件（`fle_thread_prio` が set but not used。既存の未変更ファイル）。

## 7. 限界

- Learn にシステムフォントの値は書かれていない。決定の値は仮。
- 0 で埋めた領域の読みを検出する手段が無い（§3.5）。
- f_840020 が iMenuWidth / iMenuHeight を読む。実行がそこに届くまで影響は分からない。
- ゲスト側の `f_5c1360`（フォント情報オブジェクトの作成）の先は、f_5c0bc0 の GetObjectW までしか読んでいない。
- テストのうち、`tests/spi.rs`、`gdi.rs` は既存の流儀どおり `lift/out-spi/test`、`lift/out-gdi/test` に作業ファイルを置く（gitignore 済み）。
