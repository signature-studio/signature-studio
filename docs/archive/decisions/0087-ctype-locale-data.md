# 0087: GetStringTypeW(CT_CTYPE1)、LCMapStringW(大文字化・小文字化。Locale=0 を含む)、WideCharToMultiByte(cp1252) を、範囲 D の文字に限って実装する（0016、0084 の範囲の拡張。仮の決定）

状態: PM の決定（2026-10-24）を、作業者（サブエージェント）が記録し、実装した。**仮の決定**（実機の Windows 10 22H2 で未確認。撤回条件は下）。事実の出典は `docs/investigations/dsp-ippv2-ctype-locale-data.md`（以下「調査」。節番号はその文書のもの）。この記録は調査に無い主張を足さない（調査に無い数値は、この作業で実測したと明記する）。決定 0016（ロケール。「表がページに無ければ停止」）と決定 0084（「cp437・WideCharToMultiByte・GetStringTypeW・LCMapStringW は変更しない」）を、下の範囲 D に限って改める。決定 0041（非公式の出典を「仮」として採った先例）の形に倣う。決定 0081（api-ms の GetProcAddress は NULL と 127）は変えない。

## 決定（PM。内容を変えない）

| # | 項目 | 決定 |
|---|---|---|
| 0 | 範囲 D（対象文字の集合） | 「U+0000〜U+00FF の 256 単位 ∪ 調査が測った ATTACH の入力に出る U+0100 以上の 27 単位 ∪ それらの大文字・小文字の写像の結果」の最小の閉包。PM が示した 27 単位（調査 §3.1。0152 0153 0160 0161 0178 017D 017E 0192 02C6 02DC 2013 2014 2018 2019 201A 201C 201D 201E 2020 2021 2022 2026 2030 2039 203A 20AC 2122）の写像が D の中に閉じるかを確認し、閉包を再定義してよい、とされた。確認の結果は下の「範囲 D の全列挙」（**284 単位**。27 単位に **U+0191** が加わる）。**D の外の文字が入力に 1 つでもあれば、名前付き停止**（どの関数のどの文字（コードポイント）で、決定 0087 の範囲外、と出す） |
| 1 | GetStringTypeW | dwInfoType = CT_CTYPE1 (1) のみ。D の各文字の C1 値は、調査 §5.1 の**非公式の Wine 10.0（en-US）の値**を採る。UPPER と LOWER のビットは UCD（Unicode 16.0.0）の `Lu` と `Lowercase` と一致（照合のみ）。それ以外のビットは「Wine のみ」（下の出典の格の表）。CT_CTYPE2 (2)、CT_CTYPE3 (4)、D の外の文字は名前付き停止。cchSrc、lpCharType、戻り値、エラーは Learn の GetStringTypeW のページのとおり。ページが規則を書かない入力は名前付き停止 |
| 2 | LCMapStringW | dwMapFlags が**ちょうど** LCMAP_LOWERCASE (0x100) または LCMAP_UPPERCASE (0x200) のとき（他のフラグが付いたもの、特に LCMAP_LINGUISTIC_CASING (0x01000000) は、**現在の runtime の挙動のまま**）。**Locale = 0 を、この 2 つのフラグのときに限り**受け付け、ユーザー既定（0x400）と同じに扱う（**Locale=0 は Learn に無い。この 2 フラグに限る仮の選択**）。`fle_locale_resolve` の他の用途（GetLocaleInfoW など）は変えない。大文字化は Microsoft の出典（MS-UCODEREF 3.1.5.3 の Windows 8 Upper Case Mapping Table）。U+00B5 は表どおり変えない。小文字化は Microsoft に表が無いので MS8 の大文字表の逆引きを使う（**逆引きを小文字化とみなすのは調査者の読みであり、Microsoft の記述ではない**）。サイズ問い合わせ、cchSrc = −1、出力バッファの不足、LastError は Learn の LCMapStringW のページのとおり。ページが書かないものは名前付き停止 |
| 3 | WideCharToMultiByte | cp = 1252（CP_ACP が 1252 のときと CP_THREAD_ACP を含む。決定 0084 と同じ扱い。cp の解決は既存の `fle_cp_resolve`）。変換は WCTABLE（決定 0084 の zip の `CodpageFiles/1252.txt`）。MS-UCODEREF 3.1.5.1.1.2 の疑似コード（WCTABLE の第 1 欄が一致する行の第 2 欄、無ければ lpDefaultChar、NULL なら CPINFO の既定文字 0x3f）。dwFlags は 0（best-fit を含む WCTABLE の行をそのまま使う）と 0x400（WC_NO_BEST_FIT_CHARS。**MBTABLE の厳密な逆引き 256 件に限る**。Learn の文章からの読み）。他の dwFlags は現在の runtime の挙動のまま（停止）。lpUsedDefaultChar（非 NULL のとき）は既定文字を使ったかを書く。サイズ問い合わせ、エラーは Learn のページのとおり。cp437・他のコードページ・UTF-8 の現在の挙動は変えない（ASCII だけを通す既存の挙動を保つ）。**MS-UCODEREF の疑似コードは dwFlags と lpUsedDefaultChar を扱わない** |
| 4 | 採らない選択 | 失敗を返す（応答を偽る）、恒等、停止のまま、UCD 全域を出典にする、GetStringTypeA／ExW・LCMapStringEx・LocaleNameToLCID の実装（これらは import されない、または決定 0081 の範囲） |
| 5 | 状態 | 仮の決定。撤回条件は下 |

## 範囲 D の全列挙

D = 284 単位（`lift/runtime/test/case_ms8_d.txt` が全 284 単位を 1 行ずつ持つ。生成スクリプトが閉包を計算して表明する）。

1. **U+0000〜U+00FF の 256 単位**（連続）。
2. **U+0100 以上の 28 単位**:
   `0152 0153 0160 0161 0178 017D 017E 0191 0192 02C6 02DC 2013 2014 2018 2019 201A 201C 201D 201E 2020 2021 2022 2026 2030 2039 203A 20AC 2122`
   - うち 27 単位は PM が示した ATTACH の入力（調査 §3.1。0153、0161、017E を含む。`tables/in-units.txt` の U+0100 以上 27 種と一致することを、この作業で機械的に確認した）。
   - **U+0191 は閉包で加わる**。MS8 の表に `0x0192 → 0x0191` の行があり、U+0192 の大文字化の結果が U+0191 になる（調査 §5.2、§5.3: 大文字側の出力に U+0191 が出る）。U+0191 の小文字化（逆引き）は U+0192 で、D の中。
3. **閉包の確認**（この作業の実測。MS8 973 行から）: D の全単位に MS8 の大文字化と逆引きを繰り返し適用しても、加わるのは U+0191 の 1 単位だけ。MS8 の 973 行のうち D に触れる行は **61 行で、すべて両端が D の中**（小文字 a〜z の 26、U+00E0〜U+00FE から U+00F7 を除く 30、U+00FF→U+0178、U+0153→U+0152、U+0161→U+0160、U+017E→U+017D、U+0192→U+0191）。U+00B5、U+00DF、U+00AA、U+00BA は表に無い（変わらない）。

D の使われ方は関数ごとに異なる:

| 関数 | D の用途 |
|---|---|
| GetStringTypeW | C1 の値を持つのは D のうち **283 単位**（U+0191 を除く）。**U+0191 は Wine で測っていない**（調査の Wine の実行は U+0000〜U+00FF と ATTACH の 256 単位だけで、U+0191 は入力に無い）ので、**U+0191 の C1 値は無く、名前付き停止**（D の外の文字と同じ扱い。「D に入るが値が無い」と出す） |
| LCMapStringW | 入力が D の単位だけのとき。出力も D の中（閉包） |
| WideCharToMultiByte | 入力が D の単位だけのとき（D の外は WCTABLE に行があっても停止。「解釈が分かれた点」の 2） |

## 値の出典の格

| 値・規則 | 出典 | 格 |
|---|---|---|
| 大文字化（D の 61 組） | MS-UCODEREF 3.1.5.3 と「Windows 8 Upper Case Mapping Table」（[MSDN-UCMT/Win8]。Download Center id=10921。973 行）。SHA-256 `76a66e29298eafc744fa1e8e11c80c3de28219c70539b3fb10efbccadb6ae335`（この作業でファイルから全文を計算） | **Microsoft 一次**（Windows 8 の表。Windows 10 に当てはめるのは仮。撤回条件 (d)） |
| WCTABLE（698 行）、MBTABLE（256 行）、CPINFO（既定文字 0x3f） | 決定 0084 と同じ zip（SHA-256 `5074e6dd253056ba61fc6c870c9a955467855129c6ad3a51761c386b301b125a`）の `CodpageFiles/1252.txt`（SHA-256 `72ea23c939c5b26fae7aded0207b327e2f3902d7d3c168d7087f5cfc38ee76a9`）。MS-UCODEREF 3.1.5.1.1.2 の疑似コード | **Microsoft 一次** |
| API の規則（cchSrc、サイズ問い合わせ、エラー、戻り値、既定文字） | Learn の GetStringTypeW、LCMapStringW、WideCharToMultiByte のページ（下の「Learn の規則」） | **Microsoft 一次**（ページが書く範囲だけ） |
| C1 の UPPER（0x1）、LOWER（0x2） | Wine 10.0（en-US）の `GetStringTypeW(CT_CTYPE1)` の出力（調査 §5.1） | **非公式**。UCD 16.0.0 の `General_Category = Lu` と派生プロパティ `Lowercase` が、D のうち値のある 283 単位で一致（この作業でスクリプトが照合。不一致 0）。**UCD は照合のみ**で、出典にしない。Wine の表が UCD から作られているかなど、2 つが独立かは確認していない |
| C1 の ALPHA（0x100） | 同上（Wine） | **Wine のみ**。調査 §5.1 は、UCD の `General_Category = L*` と 117 個すべて一致、と書く。UCD は Microsoft の規範として参照されている確認が無い |
| C1 の DIGIT（0x4）、PUNCT（0x10）、SPACE（0x8）、CNTRL（0x20）、BLANK（0x40）、XDIGIT（0x80）、DEFINED（0x200） | 同上（Wine） | **Wine のみ**（裏付けの出典なし）。調査 §5.1: DIGIT は U+00B2、U+00B3、U+00B9（UCD では `No`）を含む。PUNCT は 65 個で UCD の `P*`（30 個）より多い（Wine は記号 `S*` も PUNCT にする）。CNTRL は UCD の `Cc` 65 個に U+00AD を足した 66 個。**Microsoft の資料から確認できない** |
| 小文字化（D の 61 組） | MS8 の大文字表の逆引き（下の「小文字化の読み」） | **調査者の読み**（Microsoft に小文字の表は無い。逆引きは D の全単位で UCD の `Simple_Lowercase_Mapping` と Wine に一致。照合のみ） |
| Locale = 0 を、LOWERCASE／UPPERCASE の 2 フラグに限りユーザー既定（0x400）と同じに扱う | PM の仮の選択（MS-UCODEREF の大文字表が 1 つでロケール依存でない、という理由）。**非公式の Wine でも Locale 0 と 0x409、0x7F、0x400、0x800 の出力が同一**（調査 §5.2。255 単位） | **調査者の読み、PM の仮の選択**（**Learn にも MS-LCID にも Locale=0 の記述は無い**） |
| WC_NO_BEST_FIT_CHARS が MBTABLE の厳密な逆引き 256 件に限ること | Learn の WideCharToMultiByte の文章（下に引用）。どの文字が best-fit かの表は無く、WCTABLE と MBTABLE の逆引きの差から導く | **Learn の文章からの読み**（調査 §5.3 に引用） |
| lpUsedDefaultChar が「既定文字を使ったとき」に TRUE（best-fit の変換では FALSE）、サイズ問い合わせでも書く | Learn の文章（下に引用）。非公式の Wine は flags 0 の best-fit（U+0191）で used = 0、flags 0x400 で used = 1（調査の記録 `run/wine-enUS-out.txt`。照合のみ） | **読み**（疑似コードに無い） |

## 背景（事実。調査の節番号）

- DllMain(ATTACH) の CRT のマルチバイト初期化（`f_37d674c`。`setSBUpLow` 相当は形の類似による読み）は、決定 0084 の MultiByteToWideChar の先で、GetStringTypeW を 1 回（dwInfoType = 1、cchSrc = 256）、LCMapStringW を 4 回（Locale **0**、dwMapFlags 0x100 と 0x200、それぞれサイズ問い合わせと変換）、WideCharToMultiByte を 2 回（cp 1252、dwFlags 0、cb 256、lpDefaultChar と lpUsedDefaultChar は NULL）呼ぶ（調査 §3、実測）。入力は固定の 256 単位（異なる単位 255、U+00FF 以下 228、U+0100 以上 27。**U+0000 は渡されない**）。
- 決定 0084 の後の停止は GetStringTypeW（882 / 15,937。`two-module-after-0084.md`）。調査は、gdb の仮の値 8 通り（値 4、失敗 4）のすべてで DllMain が eax = 1 で戻ると実測した（§4.3）。ゲストが C1 のうち読むのは UPPER と LOWER だけ（§1 の 5）。
- 出典: C1 の文字ごとの値は Microsoft の資料に無い（§5.1）。大文字化は MS-UCODEREF 3.1.5.3（§5.2）。小文字化は表が無い（§5.2）。Locale=0 は Learn と MS-LCID に無い（§5.2）。WideCharToMultiByte(1252) は WCTABLE で、ATTACH の入力のうち best-fit に頼るのは U+0191 → 0x83 の 1 単位（§5.3）。
- 同じ CRT 部分は Blend2D.dll にもある（§6。決定 0086 の対象。読み）。

## Learn の規則（2026-10-24 に取得して読んだ。この作業が機械的に引用を確認）

URL と、実装が従った語。

**GetStringTypeW**（`https://learn.microsoft.com/en-us/windows/win32/api/stringapiset/nf-stringapiset-getstringtypew`。`ms.date` 2024-10-23）

- 「Returns a nonzero value if successful, or 0 otherwise.」エラー: 「ERROR_INVALID_FLAGS. The values supplied for flags were not valid.」「ERROR_INVALID_PARAMETER. Any of the parameter values was invalid.」
- 「The values of the *lpSrcStr* and *lpCharType* parameters must not be the same. If they are the same, the function fails with ERROR_INVALID_PARAMETER.」
- cchSrc: 「If the size includes a terminating null character, the function retrieves character type information for that character. If the application sets the size to any negative integer, the source string is assumed to be null-terminated and the function calculates the size automatically with an additional character for the null termination.」
- lpCharType: 「If *cchSrc* is not a negative number, *lpCharType* should be an array of words with *cchSrc* elements.」
- dwInfoType の値: CT_CTYPE1 「Retrieve character type information.」、CT_CTYPE2 「Retrieve bidirectional layout information.」、CT_CTYPE3 「Retrieve text processing information.」。Ctype 1 の名前と値（C1_UPPER 0x0001、C1_LOWER 0x0002、C1_DIGIT 0x0004、C1_SPACE 0x0008、C1_PUNCT 0x0010、C1_CNTRL 0x0020、C1_BLANK 0x0040、C1_XDIGIT 0x0080、C1_ALPHA 0x0100、C1_DEFINED 0x0200）。**ページは文字ごとの値を書かない**（調査 §5.1）。
- 先頭の Note: 「This API may have incomplete/outdated information for certain Unicode characters, particularly those in the supplementary range.」

**LCMapStringW**（`https://learn.microsoft.com/en-us/windows/win32/api/winnls/nf-winnls-lcmapstringw`。`ms.date` 2025-08-08。フラグの定義は LCMapStringEx のページ `https://learn.microsoft.com/en-us/windows/win32/api/winnls/nf-winnls-lcmapstringex`）

- Locale: 「Locale identifier that specifies the locale. You can use the MAKELCID macro to create a locale identifier or use one of the following predefined values.」（LOCALE_INVARIANT、LOCALE_SYSTEM_DEFAULT、LOCALE_USER_DEFAULT。**0 は挙がっていない**。調査 §5.2）
- lpSrcStr: 「This string cannot have a size of 0.」cchSrc: 「The application cannot set this parameter to 0.」「The application can set the parameter to any negative value to specify that the source string is null-terminated. In this case, if LCMapString is being used in its string-mapping mode, the function calculates the string length itself, and null-terminates the mapped string indicated by lpDestStr.」
- cchDest: 「The application can set *cchDest* to 0. In this case, the function does not use the *lpDestStr* parameter and returns the required buffer size for the mapped string or sort key.」
- 戻り値: 「If the function succeeds when used for string mapping, it returns the number of characters in the translated string」。エラー: 「ERROR_INSUFFICIENT_BUFFER. A supplied buffer size was not large enough, or it was incorrectly set to NULL.」「ERROR_INVALID_FLAGS. The values supplied for flags were not valid.」「ERROR_INVALID_PARAMETER. Any of the parameter values was invalid.」
- 失敗時の出力: 「If this function fails, the destination buffer might contain either partial results or no results at all. In this case, all results should be considered invalid.」
- LCMAP_LOWERCASE: 「For locales and scripts capable of handling uppercase and lowercase, map all characters to lowercase.」LCMAP_LINGUISTIC_CASING: 「This flag is valid with LCMAP_LOWERCASE or LCMAP_UPPERCASE only.」

**WideCharToMultiByte**（`https://learn.microsoft.com/en-us/windows/win32/api/stringapiset/nf-stringapiset-widechartomultibyte`。`ms.date` 2024-02-05）

- WC_NO_BEST_FIT_CHARS: 「Translate any Unicode characters that do not translate directly to multibyte equivalents to the default character specified by *lpDefaultChar*. In other words, if translating from Unicode to multibyte and back to Unicode again does not yield the same Unicode character, the function uses the default character.」
- lpDefaultChar: 「The application sets this parameter to **NULL** if the function is to use a system default value.」lpUsedDefaultChar: 「The flag is set to **TRUE** if one or more characters in the source string cannot be represented in the specified code page. Otherwise, the flag is set to **FALSE**. This parameter can be set to **NULL**.」4 通りの組み合わせの表（NULL／NULL は既定文字の検査なし。非 NULL／NULL は指定の既定文字を使うが lpUsedDefaultChar は設定しない。NULL／非 NULL はシステムの既定文字を使い、必要なら設定する。非 NULL／非 NULL は両方）。
- cchWideChar: 「If this parameter is -1, the function processes the entire input string, including the terminating null character.」「If *cchWideChar* is set to 0, the function fails.」cbMultiByte: 「If this value is 0, the function returns the required buffer size, in bytes, including any terminating null character, and makes no use of the *lpMultiByteStr* buffer.」
- 「If *cbMultiByte* is less than *cchWideChar*, this function writes the number of characters specified by *cbMultiByte* to the buffer indicated by *lpMultiByteStr*.」
- 「The *lpMultiByteStr* and *lpWideCharStr* pointers must not be the same. If they are the same, the function fails, and GetLastError returns ERROR_INVALID_PARAMETER.」
- エラー: ERROR_INSUFFICIENT_BUFFER、ERROR_INVALID_FLAGS、ERROR_INVALID_PARAMETER、ERROR_NO_UNICODE_TRANSLATION。

**MS-UCODEREF**（v20240423。決定 0084 と同じ文書）

- 3.1.5.3: 「To map a UTF-16 string to upper case, each UTF-16 code point is looked for in an upper casing table [MSDN-UCMT/Win8]. If an entry is found, the input code point is changed to the output code point.」3.1.5.3.2 UpperCaseMapping は「SELECT RECORD caseMapping FROM UpperCaseTable WHERE field 1 matches SourceCharacter … IF EXISTS … SET Result TO caseMapping field 2 … ELSE SET Result TO SourceCharacter」。
- 3.1.5.1.1.2（WideCharToMultiByteFromCodepageDataFile）: lpDefaultChar が NULL なら CPINFO の第 3 欄を読む。WCTABLE の第 1 欄が一致する行が無ければ lpDefaultChar を出力。行があれば第 2 欄を出力（256 未満なら 1 バイト）。**この疑似コードに dwFlags（WC_NO_BEST_FIT_CHARS など）と lpUsedDefaultChar は出てこない**（調査 §5.3）。

## 実装した動作

### GetStringTypeW（`fle_imp_GetStringTypeW`）

- dwInfoType が 1（CT_CTYPE1）以外は名前付き停止（CT_CTYPE2 = 2、CT_CTYPE3 = 4。ほか 0、3 など、ページが 3 つの値以外の結果を書かない値も同じ）。
- lpSrcStr または lpCharType が NULL、cchSrc = 0 は名前付き停止（ページが規則を書かない。「解釈が分かれた点」の 3）。
- lpSrcStr と lpCharType が同じポインタなら 0 と ERROR_INVALID_PARAMETER（ページの規則）。
- cchSrc が負なら NUL 終端と見なし、NUL を含めた個数（長さ + 1）を調べて NUL の分も書く（ページ）。cchSrc が正なら、ちょうど cchSrc 個（途中の NUL も 1 文字として）。
- 入力に D の外の文字、または C1 値の無い U+0191 があれば、**何も書かずに**名前付き停止（走査は書く前に全文字に対して行う）。メッセージは `GetStringTypeW: U+XXXX is outside the range D of decision 0087 …`（U+0191 は `… is in the range D but decision 0087 has no C1 value for it …`）。
- 成功は戻り値 1。最後のエラーは触らない（ページは成功時のエラーを書かない。既存の関数と同じ）。
- 値: U+0000〜U+00FF の 256 値と、U+0100 以上の 27 値（下の表）。Wine 10.0 en-US の値（`lift/runtime/test/c1_wine_10_0_enus.txt`。調査の記録から取り出した）。

  | 単位 | C1 |
  |---|---|
  | 0152、0160、0178、017D | 0x0301（UPPER、ALPHA、DEFINED） |
  | 0153、0161、017E、0192 | 0x0302（LOWER、ALPHA、DEFINED） |
  | 02C6 | 0x0300（ALPHA、DEFINED） |
  | 02DC、20AC、2122 | 0x0200（DEFINED） |
  | 2013 2014 2018 2019 201A 201C 201D 201E 2020 2021 2022 2026 2030 2039 203A | 0x0210（PUNCT、DEFINED） |

### LCMapStringW（`fle_imp_LCMapStringW`）

- dwMapFlags が**ちょうど** 0x100 または 0x200: 入力の各単位を D の写像（大文字: MS8 の 61 組。小文字: その逆引き。D の他の単位は変わらない）で変換する。入力に D の外の文字があれば、何も書かずに名前付き停止（サイズ問い合わせ cchDest = 0 でも停止。メッセージは `LCMapStringW: U+XXXX is outside the range D of decision 0087 …`）。
- **Locale = 0 は、この 2 つのフラグのときに限り**ユーザー既定（0x409 に解決）として受け付ける。0x400、0x800、0x409、0x7F は従来どおり。他のロケールは従来どおり名前付き停止。
- dwMapFlags が他の値（LCMAP_LINGUISTIC_CASING 付きの 0x01000100、0x01000200 を含む）のときは、**従来の挙動のまま**（Locale=0 は停止、ASCII だけを変換、0x80 以上の単位は従来の名前付き停止）。このため、`LCMapStringW(0x409, 0x200, "é")` は変換され、`LCMapStringW(0x409, 0x01000200, "é")` は停止する（PM の指定どおり。矛盾に見えるが、意図した境界）。
- サイズ問い合わせ（cchDest = 0）、cchSrc の負値（NUL を含めて変換し、個数に数える）、cchSrc = 0 と NULL（ERROR_INVALID_PARAMETER）、出力バッファ不足（ERROR_INSUFFICIENT_BUFFER、何も書かない。ページは部分的な結果を許す）は従来の実装のまま（ページのとおり）。写像は 1 単位を 1 単位に写すので長さは変わらない。同じバッファの上書き（ページは非推奨と書く）は動く。
- `fle_locale_resolve` と、それを使う他の関数（SetThreadLocale、IsValidLocale、GetLocaleInfoW、CompareStringW）は変えない（Locale = 0 を従来どおり扱う）。

### WideCharToMultiByte（`fle_imp_WideCharToMultiByte`。cp が 1252 のときだけ）

- dwFlags が 0 または 0x400（従来どおり）。他の dwFlags は従来どおり名前付き停止。
- 入力の各単位を次のとおり変換する（D の単位だけ。D の外は何も書かずに名前付き停止。メッセージは `WideCharToMultiByte: U+XXXX is outside the range D of decision 0087 …`）:
  - U+0000〜U+007F、U+00A0〜U+00FF、U+0081、U+008D、U+008F、U+0090、U+009D: 同じ数値のバイト（WCTABLE の行。MBTABLE の逆引きと同じ）。
  - 決定 0084 の表（`fle_cp1252_hi`）の 27 単位（U+0152 など）: その表の位置のバイト（0x80〜0x9F。WCTABLE の行と一致することをテストが全件確かめる）。
  - U+0191: WCTABLE の best-fit の行 → 0x83。**dwFlags が 0 のときだけ**（0x400 のときは厳密な逆引きに無いので既定文字）。
  - WCTABLE に行が無い D の単位（U+0080、U+0082〜U+008C、U+008E、U+0091〜U+009C、U+009E、U+009F の 27 単位）: 既定文字（lpDefaultChar が非 NULL ならそのバイト、NULL なら CPINFO の 0x3f）。
- lpUsedDefaultChar が非 NULL なら、既定文字を 1 つでも使ったときだけ 1、他は 0 を書く（best-fit の行による変換は「既定文字を使った」に数えない。読み）。サイズ問い合わせ（cbMultiByte = 0）でも書く（読み。ページはサイズ問い合わせの lpUsedDefaultChar を書かない）。
- cchWideChar の負値は NUL を含めて変換する。cbMultiByte = 0 は個数（NUL を含めた個数）を返し、出力を使わない（ページ）。
- 出力バッファ不足（cbMultiByte > 0 で必要量より小さい）は 0 と ERROR_INSUFFICIENT_BUFFER。cchWideChar が正のときは、ページの一文（「If cbMultiByte is less than cchWideChar, this function writes the number of characters specified by cbMultiByte」）に従い、先頭の cbMultiByte バイトを書く。cchWideChar が負（−1）のときは、ページがその場合の書き込みを書かないので何も書かない。
- 変えないもの: cp437（CP_OEMCP）と他のコードページ（ASCII だけ。0x80 以上の単位は従来の名前付き停止、メッセージも従来のまま）、UTF-8、dwFlags の検査、cchWideChar = 0 や NULL の扱い、同じポインタの扱い。

## 小文字化の読み

Microsoft に小文字の表は無い（調査 §5.2。MS-UCODEREF の 3.1.5.3 は大文字化だけ）。MS8 の大文字表は 973 行で、異なる大文字が 973 個（写像が重ならない）。この逆引き（大文字の単位 → 表の小文字の単位。表に無い単位は変わらない）を小文字化とみなす。**これは調査者の読みで、Microsoft の記述ではない**。D の全単位で UCD の `Simple_Lowercase_Mapping` と Wine の出力に一致する（調査 §5.2。D の 255 単位、この作業で U+0191 を含む 284 単位を `gen_case_wctable_expect.py` と UCD で機械的に照合: 一致）。BMP 全体では UCD と 205 コードポイントで違う（調査 §5.2。D の外）。

## 解釈が分かれた点、仮の点（作業者が選んだ。PM の決定の文面で決まらなかったもの）

1. **U+0191 の C1 値**: D の閉包に U+0191 が入るが、調査は Wine で U+0191 の C1 値を測っていない。値を作らず（UCD や他の単位からの類推もしない）、GetStringTypeW は名前付き停止。ATTACH は GetStringTypeW に U+0191 を渡さない（調査 §3.1。入力 256 単位に無い）。
2. **WideCharToMultiByte の範囲**: PM の文面は「D の外の文字は名前付き停止」と「WCTABLE 698 行をそのまま使う」の両方を含む。**D の外は停止、を採った**（D の外の WCTABLE の 441 行（U+0100 以上のほとんど。全角英数字など）は runtime に持たず、停止）。runtime に持つのは D の中の 257 行（229 行は U+00FF 以下、28 行は U+0100 以上）だけ。決定 0084 の「データファイルの内容を複製しない」にも合う。698 行全部を使う意図なら、表を足す小さな変更で広げられる（PM の判断）。
3. **GetStringTypeW でページが規則を書かない入力は停止**: dwInfoType が 1、2、4 以外（0、3、5 など。ページは ERROR_INVALID_FLAGS の一般文だけで、どの値が無効かを書かない）、lpSrcStr が NULL、lpCharType が NULL、cchSrc = 0（ページは「0 を除く」とも「0 は失敗」とも書かない。LCMapStringW のページは cchSrc = 0 を明示的に禁じるが、GetStringTypeW のページは書かない）。
4. **WideCharToMultiByte の出力バッファ不足の書き込み**: cchWideChar が正のときはページの一文どおり先頭の cbMultiByte バイトを書き、−1 のときは書かない。既存の UTF-8 などの経路は書かない（変更しない）。
5. **lpUsedDefaultChar**: best-fit の変換で FALSE、サイズ問い合わせでも書く、は読み（疑似コードにも Learn にも、これらを直接書く文は無い）。
6. **LCMapStringW の `LINGUISTIC_CASING` 付きは従来のまま**（ASCII だけ）。ATTACH は 0x100／0x200 だけを使う（調査 §3.2）。FLEngine の 2 か所（0x01000100、0x01000200。調査 §6）は 0x80 以上の単位で従来どおり停止する。
7. **`fle_locale.h` の `fle_imp_GetStringTypeW` の宣言から `__attribute__((noreturn))` を外した**（関数が値を返すようになったため。noreturn のままだと、呼び出し側のコンパイルで戻らないものとして扱われうる）。生成 C（`common.c`）の宣言には noreturn は無い。
8. **`k32_dsp.rs`**: `STOPPED`（決定 0078 §2。名前付き停止）から `GetStringTypeW` を外し、`ANSWERED` に移した（下の「テスト」）。

## 範囲外

- CT_CTYPE2、CT_CTYPE3。D の外の文字（U+0100 以上のほとんどと、U+0191 の C1）。LCMAP_LINGUISTIC_CASING 付き、他の LCMAP_* フラグ（ソートキー、幅、かな、中国語、タイトルケース、ハッシュ、NORM_*）。他のロケール。cp437 と他のコードページの WideCharToMultiByte（0x80 以上）、dwFlags の他の値（WC_COMPOSITECHECK など）。
- GetStringTypeA／GetStringTypeExW、LCMapStringEx、LCMapStringA、LocaleNameToLCID（import されない。決定 0081 の範囲）。決定 0081 は変えない（api-ms の GetProcAddress は NULL と 127）。
- FLEngine の WideCharToMultiByte／LCMapStringW の呼び出し点の入力（調査 §6 は測っていない）。Blend2D の 1 文字ずつの GetStringTypeW（0x1abbb4。調査 §6）の入力。D の外の文字が来れば名前付き停止する。
- 応答表（`imports_table.rs`）の行は足さない。GetStringTypeW は既に行がある（`("kernel32.dll", "GetStringTypeW", "fle_imp_GetStringTypeW", "jmp_imp:GetStringTypeW")`）。FLEngine は GetStringTypeW を import しないので、FLEngine の生成 C は変わらない。

## 撤回条件

- (a) 実機の Windows 10 22H2 で、D の文字の C1 値が違うと確認されたとき。
- (b) 実機で、Locale=0 の LCMAP_LOWERCASE／LCMAP_UPPERCASE の結果が違う、または Locale=0 が受け付けられないと確認されたとき。
- (c) 実機の小文字化が MS8 の逆引きと違うと確認されたとき。
- (d) Windows 10 の大文字表が Windows 8 の表と違うと確認されたとき（MS8 を Windows 10 に当てはめるのは仮）。
- (e) WCTABLE の best-fit の扱い（flags 0 の U+0191 → 0x83、WC_NO_BEST_FIT_CHARS の逆引き限定、lpUsedDefaultChar の意味づけ）が実機と違うと確認されたとき。
- (f) データファイル再利用の知財上の異議があったとき（撤回して従来の名前付き停止に戻す）。MS-UCODEREF の知財の通知が zip のデータと MS8 の表の再利用に及ぶかの法的な解釈は、決定 0084 と同様に**この決定では行わない（未判断）**。MS8 の表の取得元（Download Center id=10921）にライセンス文があるかは確認していない。
- Wine の C1 値（UPPER と LOWER 以外のビット）が、Microsoft の資料や実機で違うと確認されたとき。

## 実装した場所

- `lift/runtime/fle_locale.c`: `static const` の表（`fle_c1_latin1[256]`、`fle_c1_high[27]`、`fle_d_high[28]`、`fle_ms8_d[61]`）、`fle_imp_GetStringTypeW`（書き直し）、`fle_imp_LCMapStringW`（Locale=0 の受け入れと D の写像）、`fle_imp_WideCharToMultiByte`（cp1252 の経路を追加。他の cp は従来の `fle_wide_to_mb`）。出典のコメント付き。新しい .c、.h は作っていない。
- `lift/runtime/fle_locale.h`: `fle_imp_GetStringTypeW` の宣言から noreturn を外した（上の 7）。
- `lift/runtime/test/` の新規ファイル:
  - 生成スクリプト（標準ライブラリのみ。入力の SHA-256 を先に検査。出力先が既にあれば書かない。`gen_cp1252_expect.py` の流儀）: `gen_c1_wine_data.py`（調査の Wine の記録 `wine-enUS-out.txt` から C1 値を取り出す）、`gen_c1_ucd_expect.py`（UCD 16.0.0 から UPPER／LOWER を導き、Wine のデータと照合して書く。UnicodeData と DerivedCoreProperties のパスと SHA-256 は引数）、`gen_case_wctable_expect.py`（MS8 の表と zip から D の閉包、写像、WCTABLE 698 行を書く）。
  - 期待値のファイル（コミットする。テストが読む）: `c1_wine_10_0_enus.txt`（283 行。見出しに出典、測定の手順、SHA-256）、`c1_ucd16_upper_lower.txt`（283 行）、`case_ms8_d.txt`（284 行）、`wctable_1252_expect.txt`（698 行）。
  - **C1 の Wine の値は再生成できない**（Wine を実行した結果）。`c1_wine_10_0_enus.txt` は、記録（`lift/out-ctype/run/wine-enUS-out.txt`。gitignore の作業物）から `gen_c1_wine_data.py` で取り出したもので、記録の SHA-256 を検査する。再現できるのは取り出しまでで、Wine の実行そのものは調査の作業物にしかない。

## テスト

期待値は `lift/runtime/test/` のファイル（上のスクリプトが作った。C の配列のコピーではない）から読む。テストの実行にゲストの zip や UCD は要らない。`lift/crates/fe-lift/tests/locale.rs` に 12 件を足した（既存 20 件 → 32 件）:

- `range_d_data_files_agree_with_each_other_and_with_the_attach_input`: D が 284 単位（U+0000〜U+00FF と 28 単位）、ATTACH の入力 256 単位が 255 種（228 + 27）で U+0000 と U+0191 を含まず全て D の中、閉包（写像が D の中、61 行、逆引き）、U+00B5、U+00DF、U+00AA、U+00BA が不変、C1 が 283 単位（U+0191 なし）で UPPER／LOWER が UCD と全件一致、WCTABLE 698 行（厳密な逆引き 256、D の中 257、外 441、D の中の best-fit は U+0191→0x83 の 1 行、行の無い D の単位は 27）。作業物 `lift/out-ctype/tables/in-units.txt` があれば、調査の実測の入力と全単位を比べる（無ければ省略。リポジトリの外のファイル）。
- `gstw_c1_every_unit_of_d_one_by_one`: 283 単位を 1 つずつ（cchSrc 1）、期待ファイルと全件一致。次の単語は触れない。
- `gstw_c1_all_at_once_the_attach_input_and_the_page_rules`: 283 単位を一括。UPPER／LOWER を UCD のファイルと照合。**ATTACH の 256 単位の入力（cchSrc 256、dwInfoType 1）が停止せず通る**。cchSrc の負値（−1、−2、−100。NUL の分も書く）、明示の長さ（NUL を含めない／含める、途中の NUL）。lpSrcStr と lpCharType が同じポインタで 0 と ERROR_INVALID_PARAMETER。
- `gstw_stops_by_name_outside_the_decision_and_writes_nothing`: dwInfoType 2、4、0、3、5〜8 など 11 通り、NULL、cchSrc 0、U+0191、後ろの単位が D の外、が名前付き停止。**65,536 単位すべて**を `a?b` の途中に入れて、D の外 65,252 単位 + U+0191 が停止し単位の名前が出る、何も書かれない、D の 283 単位は通る、を数える。
- `lcmap_range_d_every_unit_for_locale_zero_and_the_defaults`: Locale 0、0x409、0x400、0x800、0x7F の 5 通り × 大文字・小文字: D の全 284 単位を一括（サイズ問い合わせ 284、変換、ちょうどのバッファ、1 足りないバッファで 0 と ERROR_INSUFFICIENT_BUFFER（バッファは触れない）、cchSrc 負値で NUL を含む）、1 単位ずつ（Locale 0 と 0x409）、NULL・長さ 0・負の cchDest（ERROR_INVALID_PARAMETER）、同じバッファ。Locale 0 がスレッドロケールを変えない。
- `lcmap_u00b5_is_unchanged_and_the_ascii_rows_still_work_with_locale_zero`: U+00B5、U+00DF、U+00AA、U+00BA が不変、U+00FF↔U+0178、ASCII の文字列。
- `lcmap_stops_locale_zero_for_other_flags_and_everything_else_stops_as_before`: **Locale 0 は 0x100／0x200 のときだけ**（0x01000100、0x01000200、0x300、0x400、0、1、0x101、0x201、0x40000200 ほか 13 通りは従来どおり停止）。他のロケール（0x407 ほか 9 通り）は停止。LINGUISTIC_CASING 付きは 0x80 以上で従来どおり停止。**Locale 0 を他の関数が従来どおり扱う**（GetLocaleInfoW と CompareStringW は停止、SetThreadLocale は 0 と 87、IsValidLocale は FALSE）。D の外の全 65,536 単位 × 2 フラグ × サイズ問い合わせと変換が停止し、単位の名前が出て何も書かれない。
- `wctmb_1252_every_unit_of_d_flags_0_and_no_best_fit_default_and_used_chars`: D の全 284 単位 × dwFlags 0、0x400 × lpDefaultChar NULL／`#`、lpUsedDefaultChar あり／なし（WCTABLE の期待ファイルから期待値を作る）。一括、サイズ問い合わせ、CP_ACP（0）と CP_THREAD_ACP（3）。
- `wctmb_1252_the_attach_chain_gstw_lcmap_lower_wctmb_lcmap_upper_wctmb`: 調査 §3.1、§3.2 の呼び出しの列（GetStringTypeW → LCMapStringW の小文字（サイズ問い合わせ + 変換）→ WideCharToMultiByte → 大文字の同じ列）が、**停止せず**、期待の値で通る。best-fit の 1 単位（バイト 0x83 の位置の U+0192→U+0191→0x83）を確かめる。
- `wctmb_1252_size_buffer_nul_and_error_rules`: サイズ問い合わせ（−1 で NUL を含む）、lpDefaultChar と lpUsedDefaultChar の 4 通り（Learn の表）、best-fit の変換は「既定文字を使った」にならない、全て表現できる文字列で TRUE が消える、バッファ不足（−1 は何も書かない、明示の長さは先頭の cbMultiByte バイトを書く）、cchWideChar 0、NULL、同じポインタ、cp437 と CP_OEMCP の ASCII（従来どおり）。
- `wctmb_stops_outside_d_other_flags_and_other_code_pages`: cp1252 の dwFlags 0x1、0x40、0x80、0x200 ほか 13 通りが停止、cp437・CP_OEMCP の U+00E9 と U+20AC は従来のメッセージで停止、cp1251、932、850、28591、2、42 が停止、cchWideChar −2 が停止、D の外の単位（全角 A、矢印、CJK、U+0100、サロゲート、U+FFFF）が停止、**WCTABLE に行のある D の外の 441 行を含む全 65,536 単位 × 2 フラグ × サイズ問い合わせと変換**が停止し単位の名前が出て何も書かれない。
- `decision_0087_stops_are_real_aborts`: 上の停止は、テスト内で `abort` を差し替えて 1 つのプログラムで何度も試している。ここでは本物の SIGABRT と stderr のメッセージを確かめる。

その他の変更（既存テスト）:

- `locale.rs`: `undefined_cases_abort` の行 `wctmb-non-ascii-1252`（cp1252 の U+00E9 で停止を表明）を `wctmb-non-ascii-437`（cp437 の U+00E9。停止のまま）と `wctmb-outside-d-1252`（U+4E2D）に、行 `lcmap-non-ascii`（0x409、0x200、U+00E9 で停止を表明）を `lcmap-non-ascii-linguistic`（0x01000200）と `lcmap-non-ascii-linguistic-lower`（0x01000100、U+00C9）に直した。理由: 0087 で、これらの入力が通る（cp1252 の U+00E9、フラグがちょうど 0x200 の U+00E9）か、停止の理由が変わるため。`PRE` の定義の末尾に static 変数を足した（新しいテストの出力バッファ）。
- `k32_dsp.rs`: `GetStringTypeW` を `STOPPED`（決定 0078 §2。名前付き停止。`the_names_that_stop_say_their_name_and_never_return` が呼んで停止を表明していた）から外して `ANSWERED` に足した（`ANSWERED` 13、`STOPPED` 20）。応答表の行と宣言の存在を確かめるテストは両方の集合に効くので変わらない。
- **テストの変異検査**（この作業の自己検査。テスト自体の検査で、決定の根拠ではない）: runtime を 4 通りに壊して（U+00B5 を大文字化、U+00AA の C1 から LOWER を除く、WC_NO_BEST_FIT_CHARS でも U+0191→0x83、Locale 0 を LINGUISTIC_CASING 付きでも受ける）テストが落ちることを確かめ、元に戻した。4 通りとも少なくとも 1 件が落ちた。
- 全体: `cargo test --workspace`: **95 バイナリ、1,125 件通過、0 失敗、0 無視**（基準 1,113 から +12）。

## 限界

- **実機の Windows では未確認**。ホストは Linux。C1 の値（特に UPPER と LOWER 以外のビット）は Wine の値で、Microsoft の資料にも実機の値にも当たっていない。大文字表は Windows 8 の表。小文字化の逆引き、Locale=0、WC_NO_BEST_FIT_CHARS の逆引き限定、lpUsedDefaultChar の意味づけは読み。
- UCD の版: 16.0.0 で照合。U+00AA、U+00BA の `General_Category` は UCD 6.1.0 で `Ll` から `Lo` に変わる（調査 §5.1）が、派生プロパティ `Lowercase` は 16.0.0 で true。Windows 10 22H2 がどの版を使うかは未確認。
- 知財: 上の撤回条件 (f)。法的な解釈は未判断。
- 調査 §4.4 のとおり、ATTACH でゲストが読む C1 は UPPER と LOWER だけで、DllMain の成否には効かない（仮の値 8 通りで実測）。この決定の値が効くのは、ゲストが `isalpha`、`isupper`、`tolower` などの CRT 関数を通常の実行で呼ぶ場合だが、調査は DLL 内にその呼び出しを見つけていない。
- ATTACH の外の呼び出し点（FLEngine の LCMapStringW と WideCharToMultiByte、Blend2D の 1 文字ずつの GetStringTypeW、QuickFontCache）の入力は測っていない。D の外が来れば名前付きで停止する。
