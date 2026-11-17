# 0067: システムフォントの lfQuality を 0 から 5 に更新する（仮の決定。非公式の出典）。lfPitchAndFamily ほかは 0066 のまま

状態: 作業者（サブエージェント）が決定。依頼者（PM）の指示は「Learn に値が無ければ非公式の裏付けを探し、実 Windows の値が複数の独立した出典で一致するなら、仮の決定として更新してよい。更新するフィールドごとに出典を書く」。**この決定は 0066 の「仮の値」の一部を、非公式の出典で置き換えるもの。Learn（一次）に値は無いまま**。実装は `lift/runtime/fle_spi.h`（`FLE_SPI_FONT_QUALITY`）、テストは `lift/crates/fe-lift/tests/spi.rs`。調査の全文と取得した出典は `docs/investigations/getobjectw-caller.md` §3。取得日はすべて 2026-10-22。

## 一次資料に値があるか（再確認。2026-10-22 取得）

- [LOGFONTW](https://learn.microsoft.com/en-us/windows/win32/api/wingdi/ns-wingdi-logfontw)（`updated_at` 2025-08-15）: lfPitchAndFamily は「The two low-order bits specify the pitch … Bits 4 through 7 … specify the font family」と、DEFAULT_PITCH / FIXED_PITCH / VARIABLE_PITCH と FF_* の**名前と意味**だけを書く（FF_SWISS: 「Fonts with variable stroke width (proportional) and without serifs. MS Sans Serif is an example.」、FF_DONTCARE: 「Use default font.」、lfQuality の DEFAULT_QUALITY: 「Appearance of the font does not matter.」）。**数値は無く、どのシステムフォントの値も書かれない。** 数値は mingw-w64 `wingdi.h`（非公式。`DEFAULT_PITCH 0`、`FIXED_PITCH 1`、`VARIABLE_PITCH 2`、`FF_DONTCARE (0<<4)`、`FF_SWISS (2<<4)`、`DEFAULT_QUALITY 0`、`CLEARTYPE_QUALITY 5`）。
- **一次資料は無い**（SPI_GETICONTITLELOGFONT、SPI_GETNONCLIENTMETRICS、NONCLIENTMETRICSW、LOGFONTW のどのページも、5 つのフォントのこれらのフィールドの値を書かない）。

## 非公式の出典（実 Windows の値。どれも「レジストリに保存された LOGFONT」であり、SPI の戻りの直接のダンプではない）

`HKCU\Control Panel\Desktop\WindowMetrics` の `CaptionFont`、`IconFont`、`MenuFont`、`MessageFont`、`StatusFont`、`SmCaptionFont` は、LOGFONTW の 92 バイトそのもの（`REG_BINARY`）。SPI_GETICONTITLELOGFONT / SPI_GETNONCLIENTMETRICS はこの値を読んで返す（Wine の `sysparams.c` の `get_font_entry` が同じ構造で実装している。Wine の実装であって Windows の実装の証拠ではない）。バイト列の並び（先頭から）: `lfHeight`(4) `lfWidth`(4) `lfEscapement`(4) `lfOrientation`(4) `lfWeight`(4) `lfItalic lfUnderline lfStrikeOut lfCharSet lfOutPrecision lfClipPrecision lfQuality lfPitchAndFamily`(各 1) `lfFaceName`(64、UTF-16)。

| # | 出典（取得日 2026-10-22） | どの OS | 引用（バイト列。`...05,00,53,00,65,00,67,00,6f,00,65,00,20,00,55,00,49,00` は `lfQuality=5, lfPitchAndFamily=0`、`"Segoe UI"`） |
|---|---|---|---|
| 1 | Super User「Default Registry Values for WindowMetrics in Windows 10」質問 954408（2015-08-09）、Wayback 2024-04-15 の取得 `https://web.archive.org/web/20240415071914/https://superuser.com/questions/954408/default-registry-values-for-windowmetrics-in-windows-10` の回答 1（Moab: 「On another W10 pc export the WindowMetrics key using regedit」）。AppliedDPI 0x78（120 DPI）の PC の export | **Windows 10**（質問の題と本文） | `"MessageFont"=hex:f1,ff,ff,ff,00,00,00,00,00,00,00,00,00,00,00,00,90,01,00,00,00,00,00,01,00,00,05,00,53,00,65,00,67,00,6f,00,65,00,20,00,55,00,49,00,00,...`（lfHeight 0xfffffff1 = -15 = -MulDiv(9,120,72)、lfWeight 0x190 = 400、lfCharSet 1、lfOutPrecision 0、lfClipPrecision 0、lfQuality 5、lfPitchAndFamily 0） |
| 2 | 同じ質問の回答 2（starwarswii: 「The default values … from this reddit comment」）。同じ Wayback 取得 | **Windows 10**（96 DPI の既定。CaptionHeight -330、ScrollWidth -255） | `"CaptionFont"=hex:f4,ff,ff,ff,00,00,00,00,00,00,00,00,00,00,00,00,90,01,00,00,00,00,00,01,00,00,05,00,53,00,65,00,...`（lfHeight -12） |
| 3 | PC Astuces フォーラム「[Mal aux YEUX] Les secrets de l'affichage net du text ?」2019-03-04 14:01 の投稿「Sauvegarde des précédents paramètres」`https://forum.pcastuces.com/mal_aux_yeux_les_secrets_de_affichage_net_du_text_-f13s13567.htm`（スレッドは「Forum Windows 10」）。投稿者自身の `CaptionFont`、`IconFont`、`MenuFont`、`MessageFont`、`SmCaptionFont`、`StatusFont` の export（AppliedDPI 0x60 = 96） | **Windows 10**（フォーラムの区分と本文「Windows 10 = Segoe」） | 6 つとも `f4,ff,ff,ff,00,00,00,00,00,00,00,00,00,00,00,00,90,01,00,00,00,00,00,01,00,00,05,00,53,00,65,00,67,00,6f,00,65,00,20,00,55,00,49,00,...`（-12、400、charset 1、quality 5、pitch 0。6 つの LOGFONT は同じ） |
| 4 | AskVG「[Tip] How to Change Fonts of Menus, Dialog Box and Other Text in Windows 8 and Later」コメント（J.Stanley、2016-07-14）、Wayback 2025-01-02 の取得 `https://web.archive.org/web/20250102085827/https://www.askvg.com/tip-how-to-change-fonts-of-menus-dialog-box-and-other-text-in-windows-8-and-later/`。「to change back to the default “Segoe UI”, I apply the following .reg files」 | **Windows 8 / 8.1 / 10**（記事の題。コメントは特定の版を言わない） | `CaptionFont` ほか 6 つが `...90,01,00,00,00,00,00,01,00,00,05,00,53,00,65,00,67,00,6f,00,65,00,20,00,55,00,49,00,...`（height は f1 または f4 で、DPI に従う） |
| 5 | Windows 11 Forum「Registry and Winaero question about text size」2026-03-03 `https://www.elevenforum.com/t/registry-and-winaero-question-about-text-size.45024/`（投稿者は「Windows 11 Pro 25H2」と書く） | **Windows 11** | `ee ff ff ff 00 00 00 00 00 00 00 00 00 00 00 00 90 01 00 00 00 00 00 01 00 00 05 00 53 00 65 00 67 00 6f 00 65 00 20 00 55 00 49 00 ...`（同じ 8 バイト `00 00 00 01 00 00 05 00`） |
| 6 | PTT「Re: [問題] 請問關於Vista字體大小」2009-07-03 `https://www.ptt.cc/bbs/Windows/M.1246633596.A.BA4.html`（`.reg` で Segoe UI 12pt を設定する例。コメントは Vista の設定画面の話） | **Windows Vista**（スレッドの題） | `...90,01,00,00,00,00,00,01,00,00,05,00,53,00,65,00,67,00,6f,00,65,00,...`（同じ 8 バイト） |

- 4 つの Windows 10 / 8 / 11 / Vista の出典で、`lfWeight` 400、`lfItalic/Underline/StrikeOut` 0、`lfCharSet` 1、`lfOutPrecision` 0、`lfClipPrecision` 0、`lfQuality` 5、`lfPitchAndFamily` 0、`lfFaceName` `Segoe UI`、`lfHeight` = -MulDiv(9, DPI, 72) が**すべて一致**した。6 つのフォントも同じ（#3）。
- **一致しないものは見つからなかった**。ただし #2 と #4 は特定の PC の export ではなく、貼られた「既定値」で、**独立の程度は証明できない**（出典 #1 の export の写しの可能性がある）。独立と言えるのは、#1（別の PC の export）、#3（投稿者の PC の export）、#5（Windows 11 の別の投稿者）。
- **反対の出典**: Tatsh の gist `font-entry.py`（`https://gist.github.com/Tatsh/1d90ab3736f1eae6987015a2fcfe8c98`）は `pitch_and_family: int = 0b00100010  # (FF_SWISS << 4) | VARIABLE_PITCH`、`quality: ... = 0` を**既定の引数**にする。これは「ユーザーが別のフォント（例: Noto Sans）を設定する .reg を作るスクリプト」の既定で、**OS の既定の値のダンプではない**（作者の選択）。実 Windows の値の証拠としては採らない。
- **Wine のテスト**（実 Windows で通ることを前提にした Wine のテストスイート。`dlls/user32/tests/sysparams.c` の `test_LOGFONT_charset`、`https://raw.githubusercontent.com/wine-mirror/wine/master/dlls/user32/tests/sysparams.c`）: `SystemParametersInfoA(SPI_GETICONTITLELOGFONT, ...)` の後 `ok(lf.lfCharSet == DEFAULT_CHARSET, ...)`、`SPI_GETNONCLIENTMETRICS` の後 `ncm.lfCaptionFont.lfCharSet == DEFAULT_CHARSET`、`ncm.lfSmCaptionFont.lfCharSet == DEFAULT_CHARSET`。**lfCharSet = 1 の独立した裏付け**（SPI の戻りの値についての表明）。lfQuality、lfPitchAndFamily についての表明は、このテストにも Wine の `test_reg_font`（高さ、幅、太さ、書体名だけを比べる）にも無い。

## 決定（フィールドごと）

| フィールド | 0066 の値 | 新しい値 | 変更 | 出典と区別 |
|---|---|---|---|---|
| lfQuality | 0（DEFAULT_QUALITY） | **5（CLEARTYPE_QUALITY）** | **更新** | 上の表の 6 出典が一致（レジストリに保存された LOGFONT のバイト）。数値 5 は mingw-w64 `wingdi.h`（非公式）と Michkap のブログ（2006）の `CLEARTYPE_QUALITY = 5` の一覧。**非公式、仮**。Learn に値は無い |
| lfPitchAndFamily | 0（DEFAULT_PITCH \| FF_DONTCARE） | 0（変更なし） | **変更しない** | PM の仮説（0x22 = VARIABLE_PITCH \| FF_SWISS）の裏付けは**見つからなかった**。逆に 6 出典のバイト列は **0**。0x22 を示すのは Tatsh のスクリプトの既定（OS のダンプではない）だけ。0066 の値のまま。出典が 0 を支える形になったので、区別は「仮」から「非公式の裏付けあり（仮）」に上がる |
| lfCharSet | 1（DEFAULT_CHARSET） | 1（変更なし） | 変更しない（出典を追加） | 6 出典のバイト列 + Wine のテスト（上）が一致。非公式の裏付けあり（仮） |
| lfOutPrecision | 0 | 0（変更なし） | 変更しない（出典を追加） | 6 出典のバイト列が 0。非公式の裏付けあり（仮） |
| lfClipPrecision | 0 | 0（変更なし） | 変更しない（出典を追加） | 6 出典のバイト列が 0。非公式の裏付けあり（仮） |
| lfHeight、lfWeight、lfItalic、lfUnderline、lfStrikeOut、lfFaceName | -12、400、0、0、0、`Segoe UI` | 同じ | 変更しない | 6 出典が一致（height は -MulDiv(9, DPI, 72)。96 DPI で -12 = 0xfffffff4）。これまで「Windows 10 に当てはめるのは仮」としていた 9 pt と `Segoe UI` に、Windows 10 の export という**非公式の裏付け**が付く（Learn の記述は Windows 7 / 8.1 / 11 のまま） |

- lfWidth、lfEscapement、lfOrientation は 0（#1〜#6 のバイト列も 0。変更なし）。
- **5 つのフォントを同じにした 0066 の選択**は、#3（6 つの LOGFONT が同じ）で支えられた。ただし #3 は自分の PC の export で、全員の既定とは限らない。

## ゲストの分岐への影響

f_5c0bc0（`docs/investigations/getobjectw-caller.md` §2）は lfQuality を**読まない**（バイト列で固定した）。lfPitchAndFamily の低位 4 ビットは読み、0 は `[obj+0x10] = 0` になる（変更なし）。つまり、この更新は**ゲストの今の分岐を変えない**。将来、フォントを SelectObject で使う描画（CreateFontIndirectW に渡した LOGFONT の lfQuality）が来たときに効く。

## 限界

- すべて**レジストリに保存された値**で、SPI の戻りのダンプではない。実 Windows の user32 が SPI の戻りで lfQuality、lfPitchAndFamily を書き換えない保証は、Learn にも Wine のテストにも無い（Wine は lfCharSet だけを DEFAULT_CHARSET にし、ほかを保存値のまま返す。Wine の実装）。
- #2、#4 は export の写しかもしれない。#5 は Windows 11、#6 は Vista で、仮想 OS（Windows 10 22H2、決定 0014）ではない。Windows 10 の直接の出典は #1、#2、#3、#4。
- 既定の値とは「ユーザーが変更していない」値。#1 は 120 DPI の PC。
- 最初の 4 つの出典は Web のフォーラムの投稿で、取得したページを保存していない（`docs/investigations/getobjectw-caller.md` に URL、引用、取得日を書いた）。

## 撤回条件

- Microsoft の文書が、システムフォントの lfQuality、lfPitchAndFamily を示した。
- 実 Windows 10 の SPI_GETNONCLIENTMETRICS の戻りのダンプが、5 と異なる lfQuality（または 0 と異なる lfPitchAndFamily）を示した。
- ゲストが lfQuality を読む分岐に届いた（f_5c0bc0 は読まない。SelectObject 以降）。そのときは再確認する。
- 0066 の撤回条件のうち、Windows 10 の LOGFONT の値が示されたとき。

## 0066 との関係

0066 の本文は消さない。0066 の「5 つのフォントの値」の lfQuality の行と lfPitchAndFamily の行に「0067 で更新」「0067 で確認」を追記した。`fle_spi.h` の `FLE_SPI_FONT_QUALITY` を 0 から 5 に変えた。`tests/spi.rs` の期待値（`the_font()`、`the_font_constants_are_the_decisions`）を更新した。`tests/gdi.rs` の期待値はハンドルと LOGFONT の複製の同一性だけで、lfQuality の値に依らない（変更なし）。
