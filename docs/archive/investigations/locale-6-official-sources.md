# en-US の GetLocaleInfoW 6 種（SLIST、INEGCURR、SSHORTDATE、SLONGDATE、STIMEFORMAT、SSHORTTIME）の公式出典調査

調査日: 2026-10-21（JST）。HEAD 3a66790。読み取り専用の調査で、ソースは変更していない。決定は PM が行う。
前提の文書: `docs/decisions/0007`、`0016`、`0041`、`0042`、`docs/investigations/locale-enus-run.md`。

「実測」はツールの出力、「読み」は逆アセンブルの静的な読み、「推測」は未検証の解釈。引用は取得した原文のまま。取得ツールの出力は HTML エスケープされていて、`-&gt;` は `->` に、`\$` は `$` に戻して書いた。リンクの記法（`[text](url)`）は外した。

**最初に書く限界（変えられない）**: 実機の Windows では確認していない。取得できた出典のうち、Windows 10 22H2（build 19045。`lift/runtime/fle_osver.c` の `fle_emulated_os`）で `GetLocaleInfoW(0x409, …)` を実行した出力は 1 件もない。実機に最も近いのは、第三者が載せたレジストリのダンプ（§3）だが、Windows の版は書かれていない。

## 0. 結論の要約

| LCType | 名前 | 候補の値 | 分類 | 確度 |
|---|---|---|---|---|
| 0xc | SLIST | `,` | 実装してよい（公式） | 強 |
| 0x1f | SSHORTDATE | `M/d/yyyy` | 実装してよい（公式の例 + 独立出典 2 つ以上） | 強 |
| 0x1003 | STIMEFORMAT | `h:mm:ss tt`（区切りは U+0020） | 実装してよい（公式の例 + 独立出典 2 つ以上） | 強〜中 |
| 0x79 | SSHORTTIME | `h:mm tt`（同上） | 実装してよい（公式の例 + 独立出典 2 つ以上） | 強〜中 |
| 0x20 | SLONGDATE | `dddd, MMMM d, yyyy` | 実装してよい（独立出典 2 つ以上一致、非公式） | 弱（公式 2 件が `d` と `dd` で食い違う） |
| 0x1c | INEGCURR | `0` | 実装してよい（独立出典 2 つ以上一致、非公式。公式の例 1 件が支持） | 最弱（版依存の疑いが残る。abort 維持も妥当） |

- 「公式」は Microsoft Learn のページが en-US の値を書いているもの。ただし 0xc 以外は Win32 の LOCALE_* ページではなく、.NET と PowerShell のページにある en-US の出力例で、**測定した出力ではない静的な文章**。Win32 の LOCALE_* ページは、en-US の既定値を明言していない（SSHORTDATE と SSHORTTIME のページに、en-US と名指さない書式の例があるだけ）。
- ゲストは日付の書式を**ほぼそのまま使い**、時刻の書式は**パース**する（§5）。時刻の書式は **NLS 表記の `tt` が必須**（CLDR 表記の `a` は変換されず素通しになる）。
- 全候補が ASCII だけで、runtime の `LCMapStringW`（0x80 以上は stop）を通る日付書式でも止まらない。

## 1. 取得の記録

取得日時: 2026-10-20T19:46Z〜2026-10-20T20:20Z（= 2026-10-21 04:46〜05:20 JST）の範囲。取得ツールが時刻を返さないので、ページごとの時刻は記録していない。範囲は、作業の前後でシェルの `date -u` を取って決めた（19:46:40Z、20:12:52Z、20:20:01Z）。

### 1.1 取得できたもの（ocean_webfetch。本文を読んだ）

| URL（https://learn.microsoft.com/en-us/ は略す） | 内容 |
|---|---|
| `dotnet/standard/base-types/standard-date-and-time-format-strings` | 'd'、'D'、't'、'T' の en-US の出力例 |
| `dotnet/standard/base-types/custom-date-and-time-format-strings` | "h"、"tt"、"t" の例 |
| `dotnet/standard/base-types/standard-numeric-format-strings` | "C3" の en-US の例（負の通貨） |
| `dotnet/api/system.globalization.datetimeformatinfo.shortdatepattern` | en-US `M/d/yyyy` |
| `dotnet/api/system.globalization.datetimeformatinfo.longdatepattern` | en-US `dddd, MMMM d, yyyy` |
| `dotnet/api/system.globalization.datetimeformatinfo.shorttimepattern` | en-US `h:mm tt` |
| `dotnet/api/system.globalization.datetimeformatinfo.longtimepattern` | en-US `h:mm:ss tt` |
| `dotnet/api/system.globalization.numberformatinfo.currencynegativepattern` | 値の表（0〜16）。en-US の値の記述なし |
| `dotnet/api/system.globalization.textinfo.listseparator` | 値の記述なし |
| `dotnet/api/system.globalization.cultureinfo` | en-US の書式値の記述なし |
| `dotnet/core/extensions/globalization-icu` | NLS と ICU の違い |
| `windows/win32/intl/locale-information-constants` | 定数の一覧 |
| `windows/win32/intl/locale-ineg-constants` | INEGCURR の表（0〜15） |
| `windows/win32/intl/locale-slist`、`locale-sshortdate`、`locale-slongdate`、`locale-stime-constants`、`locale-sshorttime` | 各定数の説明 |
| `windows/win32/intl/hour--minute--and-second-format-pictures`、`day--month--year--and-era-format-pictures` | NLS の書式記法 |
| `windows/win32/intl/locale-nouseroverride`、`locale-return-constants`、`locale-icurrency`、`locale-scurrency`、`locale-sabbrev-constants`、`locale-names`、`using-persistent-locale-data`、`national-language-support-constants` | 補足 |
| `windows/win32/api/winnls/nf-winnls-getlocaleinfow`、`nf-winnls-getlocaleinfoex` | 関数の説明 |
| `globalization/locale/currency-formatting`（`currency-formats` に転送）、`globalization/locale/number-formatting` | 一般論だけ。en-US の値なし |
| `powershell/module/microsoft.powershell.utility/get-culture` | en-US の `DateTimeFormat` の出力例 |
| `troubleshoot/microsoft-365-apps/excel/formula-errors` | 既定のリスト区切り |
| `openspecs/windows_protocols/ms-lcid/a9eac961-e77d-41a6-90a5-ce1a8b0cdb9c` | LCID の表だけ。書式の値なし |
| https://support.microsoft.com/en-us/excel/negative-numbers-aren-t-showing-with-parentheses-in-excel | 負の通貨の設定手順 |
| https://learn.microsoft.com/en-us/answers/questions/5878681/show-negative-numbers-in-parenthesis | Microsoft Q&A（既定値の記述なし） |
| https://github.com/dotnet/runtime/issues/102220、`/82178`、`/81853` | Microsoft の dotnet/runtime の issue（ICU の値） |
| https://raw.githubusercontent.com/dotnet/runtime/main/src/libraries/System.Private.CoreLib/src/System/Globalization/CultureData.Icu.cs | .NET の ICU 経路のコード |
| https://renenyffenegger.ch/notes/Windows/registry/tree/HKEY_CURRENT_USER/Control-Panel/International/index | 第三者。レジストリの値の表（非公式） |
| https://www.vemotech.fr/tutorials/mfa-resoudre-erreur-aucune-feuille-de-style-nest-configuree-dans-le-theme-actif-pour-la-locale-par-defaut-075852 | 第三者。`.reg` の全文（非公式） |

### 1.2 取得に失敗したもの（失敗として記録）

| URL | 結果 |
|---|---|
| https://learn.microsoft.com/en-us/windows/win32/intl/locale-ineg-curr | 404（INEGCURR は `locale-ineg-constants` にある） |
| https://learn.microsoft.com/en-us/globalization/locale/other-locale-settings | 404 |
| https://learn.microsoft.com/en-us/globalization/locale/standard-formats | 404 |
| https://raw.githubusercontent.com/dotnet/runtime/main/src/libraries/System.Globalization/tests/NumberFormatInfo/NumberFormatInfoData.cs | 404 |
| https://stackoverflow.com/questions/15760035/boolean-values-in-local-language | 403 |
| https://forums.mydigitallife.net/threads/how-do-you-optimize-your-windows-10-setup.76025/page-3 | 403 |
| https://www.askvg.com/tip-customize-date-and-time-in-windows-11-taskbar-clock/ | 403 |
| https://grabebook.files.wordpress.com/2011/12/sybex-administering-windows-server-2008-server-core.pdf | 取得ツールが PDF を扱えない（Unsupported fetched file content type: application/pdf） |

### 1.3 見つからなかったもの

- PM が候補に挙げた「HKCU\Control Panel\International の en-US の既定値を載せた Microsoft の KB、サポート文書」: 検索 4 回（`wall_websearch`）で、そのようなページは見つからなかった。検索結果に出た Microsoft のページ（Windows registry information for advanced users、Configure International Settings in Windows、unattend の International-Core など）は、既定値の一覧ではなく設定の手順なので取得していない。
- Windows 11 の新しい版で en-US の INEGCURR が `1` になった、あるいは時刻の書式の空白が U+202F になった、と書く資料は、検索でも取得したページにも見つからなかった（PM の記憶は出典にならない）。

## 2. 公式ページの引用（LCType 別）

### 2.1 SLIST（0xc）

出典 A（Win32。en-US の値なし）: https://learn.microsoft.com/en-us/windows/win32/intl/locale-slist
> Character(s) used to separate list items, for example, a comma is used in many locales. The maximum number of characters allowed for this string is four, including a terminating null character.

出典 B（**en-US の既定を明言**。Microsoft 365 Apps のトラブルシュート記事）: https://learn.microsoft.com/en-us/troubleshoot/microsoft-365-apps/excel/formula-errors
> The comma is the default list separator for US - English Locale.

同じページ:
> The list separator in Windows - Regional Settings doesn't match what is being typed for the Excel formula.
> Changing the List separator in the Windows Region settings will affect the delimiter used when opening or saving a Comma-separated value (.csv) file as Excel utilizes the Windows list separator character for the delimiter in .csv files.

読み: この記事の「list separator」は Windows の地域設定の「リスト区切り記号」で、`LOCALE_SLIST` のこと。Win32 の API のページではなく Excel の記事だが、Microsoft の公開資料が en-US の既定を `,` と書いている。

.NET の TextInfo.ListSeparator のページ（https://learn.microsoft.com/en-us/dotnet/api/system.globalization.textinfo.listseparator）は「The string that separates items in a list.」だけで、値はない。

CLDR の `symbols list ";"`（0042 §3）について: .NET の ICU 経路（Microsoft のコード）は、CLDR の値を使わず自前の表で決め、既定は `,`。
https://raw.githubusercontent.com/dotnet/runtime/main/src/libraries/System.Private.CoreLib/src/System/Globalization/CultureData.Icu.cs
```
private static string IcuGetListSeparator(string? cultureName)
...
    int separator = IcuLocaleData.GetLocaleDataNumericPart(cultureName, IcuLocaleDataParts.DigitSubstitutionOrListSeparator);
...
    return ","; // default separator
```
この表で en-US が何かまでは読んでいない（参考）。CLDR の `;` を `LOCALE_SLIST` の値と見なさない、という間接の裏付けにとどまる（推測）。

### 2.2 INEGCURR（0x1c）

出典 A（Win32。値の表。en-US の既定は書かない）: https://learn.microsoft.com/en-us/windows/win32/intl/locale-ineg-constants
> | 0 | Left parenthesis, monetary symbol, number, right parenthesis; for example, ($1.1) |
> | 1 | Negative sign, monetary symbol, number; for example, -$1.1 |

出典 B（.NET の表。既定は InvariantInfo の値だけ。en-US は書かない）: https://learn.microsoft.com/en-us/dotnet/api/system.globalization.numberformatinfo.currencynegativepattern
> The format pattern for negative currency values. The default for InvariantInfo is 0, which represents "($n)", where "$" is the CurrencySymbol and "n" is a number.

同じページのコード例の注:
> // The result shown here happens only if using ICU.
> // Apps that use NLS, for example, .NET Framework apps, might see different results.
> // Globalization data can change, which would also change these results.

出典 C（**en-US の負の通貨の例。モード 0 と等価**）: https://learn.microsoft.com/en-us/dotnet/standard/base-types/standard-numeric-format-strings
> | "C" or "c" | Currency | … | 123.456 ("C", en-US)-> $123.46 123.456 ("C", fr-FR)-> 123,46 € 123.456 ("C", ja-JP)-> ¥123 -123.456 ("C3", en-US)-> ($123.456) -123.456 ("C3", fr-FR)-> -123,456 € -123.456 ("C3", ja-JP)-> -¥123.456 |

同じページの注意書き（この例が測定値ではないことを Microsoft 自身が書いている）:
> The result of a formatted string for a specific culture might differ from the following examples. Operating system settings, user settings, environment variables, and the .NET version you're using can all affect the format. For example, starting with .NET 5, .NET tries to unify cultural formats across platforms.

論理: 例 "($123.456)" は、表の `0` の「($n)」に当たる（`-$n` なら "-$123.456"）。したがって、この例は en-US = モード 0 を示す。ただし注意書きのとおり、.NET 5 以降（ICU）では違い得る。

出典 D（ICU 側の値。Microsoft の dotnet/runtime の issue）: https://github.com/dotnet/runtime/issues/82178
> Creating a sample project in Blazor WASM, you'll see that the en-US culture will return "1" for its negative currency pattern, thus showing -$100 and not ($100).

> the .NET wasm runtime uses icu to determine the number formatting rules, like .NET also does on linux (and others https://learn.microsoft.com/en-us/dotnet/core/extensions/globalization-icu).  If you run this sample on linux … it will also output -$25.30

読み: **ICU（CLDR）の en-US は `1`、Windows NLS を使う .NET の例は `0`**。0042 の「Wine は 0、CLDR は 1（標準の通貨パターンは `-¤#,##0.00`）」という食い違いは、この「エンジンの違い」で説明がつく（推測。出典 C と D の組み合わせによる）。

出典 E（既定を書かない）: https://support.microsoft.com/en-us/excel/negative-numbers-aren-t-showing-with-parentheses-in-excel
> On the Currency tab, for Negative currency format, choose ($1.1).

この記事は「負の数が括弧で出ない」ときの手順で、既定値は書かない。括弧にしたい人がいる、ということだけが読める。

### 2.3 SSHORTDATE（0x1f）

出典 A（Win32。en-US と名指さない書式の例）: https://learn.microsoft.com/en-us/windows/win32/intl/locale-sshortdate
> Short date formatting string for the locale. The maximum number of characters allowed for this string is 80, including a terminating null character. The string can consist of a combination of day, month, year, and era format pictures. For example, "M/d/yyyy" indicates that September 3, 2004 is written 9/3/2004.

出典 B（**en-US の値**）: https://learn.microsoft.com/en-us/dotnet/api/system.globalization.datetimeformatinfo.shortdatepattern
> //          en-US            M/d/yyyy   5/1/2011
>
> //       Original Short Date Pattern:
> //          M/d/yyyy: 5/1/2011

出典 C（出力例）: https://learn.microsoft.com/en-us/dotnet/standard/base-types/standard-date-and-time-format-strings
> | "d" | Short date pattern. … | 2009-06-15T13:45:30 -> 6/15/2009 (en-US) …

> Console.WriteLine(thisDate.ToString("d"));           // Displays 3/15/2008

出典 D（Windows PowerShell の出力例）: https://learn.microsoft.com/en-us/powershell/module/microsoft.powershell.utility/get-culture
> ShortDatePattern                 : M/d/yyyy

**NLS の書式との等価の論理**: Win32 の書式記法のページ（https://learn.microsoft.com/en-us/windows/win32/intl/day--month--year--and-era-format-pictures）に
> | d | Day of the month as digits without leading zeros for single-digit days. |
> | M | Month as digits without leading zeros for single-digit months. |
> | yy | Year represented only by the last two digits. A leading zero is added for single-digit years. |
> | yyyy | Year represented by a full four or five digits, depending on the calendar used. …

出力例 `5/1/2011`（出典 B。月 5、日 1、年 2011）は、月と日が先頭ゼロなし（`M`、`d`）、年が 4 桁（`yyyy`）。`MM`、`dd`、`yy` なら `05/01/11` になる。書式 `M/d/yyyy` は .NET と Win32 で同じ文字列。

### 2.4 SLONGDATE（0x20）

出典 A（Win32。en-US の例なし）: https://learn.microsoft.com/en-us/windows/win32/intl/locale-slongdate
> Long date formatting string for the locale. The maximum number of characters allowed for this string is 80, including a terminating null character. The string can consist of a combination of day, month, year, and era format pictures and any string of characters enclosed in single quotes. Characters in single quotes remain as specified. For example, the Spanish (Spain) long date is "dddd, dd' de 'MMMM' de 'yyyy". Locales can define multiple long date formats.

出典 B（**`d`**。OS 依存の注記つき）: https://learn.microsoft.com/en-us/dotnet/api/system.globalization.datetimeformatinfo.longdatepattern
> This code produces the following output. Note that the exact output format depends on the OS, the OS version, and the native globalization library used by the OS.
>
>  CULTURE    PROPERTY VALUE
>   en-US     dddd, MMMM d, yyyy

> //    en-US   dddd, MMMM d, yyyy   Saturday, November 12, 2011

出典 C（**`dd`**）: https://learn.microsoft.com/en-us/powershell/module/microsoft.powershell.utility/get-culture
> LongDatePattern                  : dddd, MMMM dd, yyyy

同じ出力に `FullDateTimePattern : dddd, MMMM dd, yyyy h:mm:ss tt`、`MonthDayPattern : MMMM dd`、`YearMonthPattern : MMMM, yyyy` がある。レジストリのダンプ（§3）は `sYearMonth` が `MMMM yyyy`（カンマなし）なので、出典 C は古い Windows（Windows 7 や 8 の世代）の出力例と読める（推測）。

出力例（`Monday, June 15, 2009`、`Thursday, April 10, 2008`、`Saturday, November 12, 2011`）の日は全部 10 以上で、`d` と `dd` を区別できない。書式の文字列を直接書いているのは出典 B（`d`）と C（`dd`）だけで、**公式の 2 件が食い違う**。

### 2.5 STIMEFORMAT（0x1003）

出典 A（Win32。書式の記法だけ）: https://learn.microsoft.com/en-us/windows/win32/intl/locale-stime-constants
> | LOCALE_STIMEFORMAT | Time formatting strings for the locale. The maximum number of characters allowed for this string is 80, including a terminating null character. The string can consist of a combination of hour, minute, and second format pictures. |

> | LOCALE_STIME | Character(s) for the time separator. … **Windows Vista and later:** This constant is deprecated. Use LOCALE_STIMEFORMAT instead. …

出典 B（**en-US の値**）: https://learn.microsoft.com/en-us/dotnet/api/system.globalization.datetimeformatinfo.longtimepattern
> This code produces the following output.
>
>  CULTURE    PROPERTY VALUE
>   en-US     h:mm:ss tt

出典 C（出力例）: https://learn.microsoft.com/en-us/dotnet/standard/base-types/standard-date-and-time-format-strings
> | "T" | Long time pattern. … | 2009-06-15T13:45:30 -> 1:45:30 PM (en-US) …

出典 D: https://learn.microsoft.com/en-us/powershell/module/microsoft.powershell.utility/get-culture
> LongTimePattern                  : h:mm:ss tt

**NLS の書式との等価の論理**: Win32 の書式記法のページ（https://learn.microsoft.com/en-us/windows/win32/intl/hour--minute--and-second-format-pictures）に
> | h | Hours without leading zeros for single-digit hours (12-hour clock). |
> | mm | Minutes with leading zeros for single-digit minutes. |
> | ss | Seconds with leading zeros for single-digit seconds. |
> | tt | Multi-character time marker string. |

> Note: In the format types, the letters "m", "s", and "t" must be lowercase, and the letter "h" must be lowercase to denote the 12-hour clock or uppercase ("H") to denote the 24-hour clock.

13:45:30 → `1:45:30 PM`（出典 C）は、時が先頭ゼロなしの 12 時間制（`h`）、分と秒が 2 桁（`mm`、`ss`）、時刻の印が 2 文字の `PM`（`tt`。1 文字の `t` なら `P`。.NET の custom ページに「"t" … 2009-06-15T13:45:30 -> P (en-US)」「"tt" … -> PM (en-US)」）。`h:mm:ss tt` は .NET と Win32 で同じ文字列。

### 2.6 SSHORTTIME（0x79）

出典 A（Win32。**`h:mm:ss tt` の例と、そこから導かれる短い書式**）: https://learn.microsoft.com/en-us/windows/win32/intl/locale-sshorttime
> **Windows 7 and later:** Short time formatting string for the locale. Patterns are typically derived by removing the "ss" (seconds) value from the long time format pattern. For example, if the long time format is "h:mm:ss tt", the short time format is most likely "h:mm tt". This constant can specify multiple formats in a semicolon-delimited list. However, the preferred short time format should be the first value listed.

出典 B（**en-US の値**）: https://learn.microsoft.com/en-us/dotnet/api/system.globalization.datetimeformatinfo.shorttimepattern
>  CULTURE    PROPERTY VALUE
>   en-US     h:mm tt

出典 C: https://learn.microsoft.com/en-us/dotnet/standard/base-types/standard-date-and-time-format-strings
> | "t" | Short time pattern. … | 2009-06-15T13:45:30 -> 1:45 PM (en-US) …

出典 D: https://learn.microsoft.com/en-us/powershell/module/microsoft.powershell.utility/get-culture
> ShortTimePattern                 : h:mm tt

注意: 出典 A は、複数の書式を `;` で区切って返し得る、とも書く（「The preferred short time format should be the first value listed」）。en-US の実装は書式 1 つだけを返す想定。

### 2.7 空白の種類（U+0020 か U+202F か）

- 取得した .NET の Standard date and time format strings のページの本文（保存した出力）に、U+202F、U+00A0 は **0 件**（Python で数えた。時刻の例 `1:45 PM`、`1:45:30 PM` は通常の空白）。ただしこれは文書の文字で、OS の出力ではない。
- U+202F は **ICU/CLDR（.NET が macOS、Linux で ICU を使うとき）の値**で、Windows NLS の値ではない。
  - https://github.com/dotnet/runtime/issues/102220（Microsoft の dotnet/runtime）
  > In the string produced by string.Format() using either the short time pattern "t" or long time pattern "T" in en-US (CultureInfo.GetCultureInfo("en-US")), the regular space between the numeric portion of the time and AM/PM portion changed from a regular space (\u0020) to a non-breaking space (\u202f).
  > OS: MacOS 14.4 (Sonoma) (ARM64), Linux Ubuntu 20.04 (x64), whatever OS linux .NET ASP.Net containers use (x64)
  > This change originates from the Unicode standard (CLDR) and has been implemented globally to resolve the text wrapping issue.（Microsoft の担当者の返信）
  - .NET の ICU 経路のコード（CultureData.Icu.cs の `ConvertIcuTimeFormatString`）は、ICU の `a` を `tt` に変え、ASCII の英字以外の文字（U+202F を含む）はそのままコピーする:
  ```
  case 'a': // AM/PM
  ...
      result[resultPos++] = 't';
      result[resultPos++] = 't';
  ...
  default:
      // Characters that are not ASCII letters are literal texts
      if (!char.IsAsciiLetter(current))
          result[resultPos++] = current;
  ```
- Windows NLS で U+202F が出る版があるかは、**確認できない**（見つからなかった。§1.3）。Win32 の SSHORTTIME のページの例 `"h:mm:ss tt"` は、ページの表示上は通常の空白（コードポイントは未確認）。

## 3. 非公式の独立した出典（Windows の値を載せた第三者の資料）

どちらも `HKEY_CURRENT_USER\Control Panel\International` の値。**Windows の版は書かれていない**。

出典 R1: https://www.vemotech.fr/tutorials/mfa-resoudre-erreur-aucune-feuille-de-style-nest-configuree-dans-le-theme-actif-pour-la-locale-par-defaut-075852（2024-01-04。Windows Server の ADFS のサービス アカウントのプロファイルに、en-US の値を足す `.reg` の全文）
```
"Locale"="00000409"
"LocaleName"="en-US"
"sList"=","
"sLongDate"="dddd, MMMM d, yyyy"
"sShortDate"="M/d/yyyy"
"sTimeFormat"="h:mm:ss tt"
"sShortTime"="h:mm tt"
"sYearMonth"="MMMM yyyy"
"iNegCurr"="0"
"iCurrency"="0"
"iTime"="0"
"iTLZero"="0"
```
出典 R2: https://renenyffenegger.ch/notes/Windows/registry/tree/HKEY_CURRENT_USER/Control-Panel/International/index（日付なし。値の表の「Example」列）
```
sList            List separator    ,
sLongDate        Long date format  dddd, MMMM d, yyyy
sShortDate     Short date format  M/d/yyyy
sTimeFormat  Long time format  h:mm:ss tt
sShortTime  Short time format  h:mm tt
iNegCurr  Negative currency format  0
```

この 2 件は互いに独立と見なせる（別の筆者、別の目的）が、**どちらも版と取得元が不明**。レジストリの値は、GetLocaleInfoW が `LOCALE_NOUSEROVERRIDE` なしで呼ばれ、LCID がユーザーの既定のロケールのときに返る値（ユーザーの上書きを含む）でもある。
> If this constant is not specified, any values that the user has configured from the Control Panel or that an application has configured using SetLocaleInfo take precedence over the database settings for the current system default locale.（https://learn.microsoft.com/en-us/windows/win32/intl/locale-nouseroverride）

ゲストの呼び出しは LCType に `LOCALE_NOUSEROVERRIDE`（0x80000000）を足さない（読み。`r8d = 0x1c` などの即値）ので、**実機では、ユーザーの上書きがない既定のユーザーなら、レジストリの値が返る**（推測）。

Wine（5.0 の enu.nls、現行の locale.nls）と CLDR は、0042 §3 の記録による（今回は再取得していない）。Wine の 2 つの世代は同じプロジェクトのデータで、完全には独立でない（0042 の限界のとおり）。

### 3.1 出典の突き合わせ（Windows の値と ICU/CLDR の値を区別）

| LCType | Wine 5.0（Windows 由来） | Wine 現行（同左） | レジストリ R1 | レジストリ R2 | Microsoft の公式ページの例 | ICU/CLDR（Windows ではない） |
|---|---|---|---|---|---|---|
| 0xc | `,` | `,` | `,` | `,` | 「The comma is the default list separator for US - English Locale.」 | `;`（CLDR）。.NET の ICU 経路の既定は `,` |
| 0x1c | `0` | `0` | `0` | `0` | `($123.456)`（.NET、モード 0 と等価。注意書きつき） | `1`（issue 82178） |
| 0x1f | `M/d/yyyy` | `M/d/yyyy` | `M/d/yyyy` | `M/d/yyyy` | `M/d/yyyy`（.NET、PowerShell） | `M/d/yy` |
| 0x20 | `dddd, MMMM dd, yyyy` | `dddd, MMMM d, yyyy` | `d` | `d` | .NET: `d`、PowerShell: `dd` | `EEEE, MMMM d, y` |
| 0x1003 | `h:mm:ss tt` | `h:mm:ss tt` | `h:mm:ss tt` | `h:mm:ss tt` | `h:mm:ss tt`（.NET、PowerShell） | `h:mm:ss a`（新しい CLDR は `a` の前が U+202F） |
| 0x79 | `h:mm tt` | `h:mm tt` | `h:mm tt` | `h:mm tt` | `h:mm tt`（.NET、PowerShell、Win32 の例） | `h:mm a`（同上） |

読み: 食い違いは、**Windows NLS（と、それをコピーした Wine）対 ICU/CLDR** で整理できる。.NET のページ自身が「Apps that use NLS, for example, .NET Framework apps, might see different results.」と書き、globalization-icu のページに「Even when using ICU, the `CurrentCulture`, `CurrentUICulture`, and `CurrentRegion` members still use Windows operating system APIs to honor user settings.」とある。**ゲストが呼ぶのは Windows の `GetLocaleInfoW`** なので、合わせる相手は Windows 側の列（推測）。

SLONGDATE の `dd` と `d` は、Windows の版の違い（古い版が `dd`、Windows 10 世代が `d`）と読める（推測。根拠は、レジストリの 2 件が `d`、PowerShell の例が古い世代と読めること）。ただし、レジストリの 2 件の版は不明で、「Windows 10 22H2 が `d`」とは断定できない。

## 4. 論理で示す等価（まとめ）

- `M/d/yyyy`、`dddd, MMMM d, yyyy`、`h:mm:ss tt`、`h:mm tt` は、.NET の custom 書式と Win32 の書式記法で、**同じ文字の並び**（`M`、`d`、`yyyy`、`dddd`、`MMMM`、`h`、`mm`、`ss`、`tt`）。公式の出力例（`6/15/2009`、`5/1/2011`、`1:45:30 PM`、`1:45 PM`）は、この書式でだけ得られる（上の §2.3、§2.5）。
- CLDR の `M/d/yy`、`EEEE, MMMM d, y`、`h:mm:ss a` は、別の記法。.NET の標準書式 'd' の出力例が `6/15/2009`（4 桁の年）であることから、公式の en-US の例は `yy` ではない。

## 5. ゲストの使い方（逆アセンブルによる静的な確認。読み）

対象: `binaries/win64/FLEngine_x64.dll`（`objdump -d -M intel`。ファイルは作っていない）。

### 5.1 呼び出しの構造

- `f_446400`（0x446400）は、ロケールの書式オブジェクトを作る関数。呼び出し元は 0x44bb9f（`f_44bb70`）。LCID は 0x15ec3a0（`f_44bae0` が 0x409 と置き、`GetThreadLocale` の結果で更新）。まず `IsValidLocale(lcid, 1)`（0x4464c8）で確かめ、失敗なら `GetThreadLocale` を使う。
- `f_446370`（ラッパー A）: `GetLocaleInfoW(lcid, type, buf, 0x100)` が正なら buf を文字列にし、0 以下なら**呼び出し元が渡した既定の文字列**を返す。
- 順序（読み）: 0x14、0x1b、**0x1c**、0xf、0xe、0x19、0x1d、**0x1f**、**0x20**、0x1e、0x28、0x29、**0x1003**、**0x79**、（フォールバックのときだけ 0x25、0x23、0x1005）、**0xc**。`locale-enus-run.md` §8 は 0x25、0x23、0x1005 を常に呼ぶ順に書いたが、`obj+0x30`（短い時刻）と `obj+0x38`（時刻の書式）がともに空でないとき、0x4467b1〜0x4467c5 から 0x446931 に跳び、**この 3 つは呼ばれない**（読み。訂正）。

### 5.2 0x1c（INEGCURR）

- 0x446599: `r8d = 0x1c`、既定 `r9 = 0x4469d0`（文字列 "0"）。0x4465a7 で `f_43aae0(str, 0)`（文字列を整数にし、失敗なら既定 0）を呼び、バイトで `obj+0x17e` に格納。
- 使用箇所（1 つ見つけた）: 0x43fb99 で `obj+0x17e` を読み、`cmp al, 0xf; jbe` で、**16 要素の表 0x13ab360** の `[al*8]` を引く。範囲外は 0x13ab3d8 の文字列。表の内容（デコード）:
  `0:'($*)@' 1:'-$*@@' 2:'$-*@@' 3:'$*-@@' 4:'(*$)@' 5:'-*$@@' 6:'*-$@@' 7:'*$-@@' 8:'-* $@' 9:'-$ *@' 10:'* $-@' 11:'$ *-@' 12:'$ -*@' 13:'*- $@' 14:'($ *)' 15:'(* $)'`（`*` は数、`@` は末尾の詰め物と見える。`@` の意味は確認していない）。
  分岐のフラグ `[rbp+0x56]` が真のとき負の通貨の表（`obj+0x17e`）、偽のときは ICURRENCY の表（`obj+8`、0x13ab340: `'$*@@@' '*$@@@' '$ *@@' '* $@@'`）。
- 結論: **整数としてだけ効く**。`"0"` は `($*)` 型、`"1"` は `-$*` 型の書式を選ぶ（表の 0 番目と 1 番目）。値が違うと、ゲストが負の通貨を整形するときの文字列が変わる（この経路に harness が到達するかは未確認）。ゲスト自身の既定も "0"。

### 5.3 0x1f、0x20（SSHORTDATE、SLONGDATE）

- 0x44665a、0x4466b2 で `f_447e30`（ラッパー E）を呼ぶ。既定は 0x4469e0 の `"m/d/yy"`、0x4469fc の `"mmmm d, yyyy"`（小文字の `m`）。5 番目の引数は SDATE の文字（`obj+0xa`）。
- `f_447f40`（0x447f40）の動き（読み）:
  1. `GetLocaleInfoW(lcid, 0x1009)`（ICALENDARTYPE。en-US は "1"）を整数にし、`{3,4,5}`（マスク 0x38、0x44829c）なら文字ごとの変換ループ（0x448099 以降）へ。
  2. そうでなければ、グローバル 0x15ec3a4（`GetThreadLocale() & 0x3ff` = 英語は 9。0x44bb1d）が、マスク 0x00060010（言語 ID 4、0x11、0x12 = 中国語、日本語、韓国語。0x44829d）に入るか調べる。**入らなければ 0x448078 で、文字列をそのままコピー**（en-US はこちら）。
  3. 0x447ee0 で、`'` の外にある SDATE の文字を `/` に置換（en-US は SDATE が `/` なので何も変わらない）。
- そのあと、0x450540 → 0x450560 で `LCMapStringW(lcid, 0x01000100, …)` を呼ぶ。0x01000100 = `LCMAP_LOWERCASE | LCMAP_LINGUISTIC_CASING`。**書式の文字列は小文字化される**（`M` → `m`）。結果は `obj+0x10`（短い日付）、`obj+0x18`（長い日付）。
- 結論: ゲストは en-US では日付の書式を**パースせず**、大小文字（`M` か `m` か）の違いも出ない。**`yy` と `yyyy`、`d` と `dd` の違いは、そのまま後段の `obj+0x10`、`+0x18` に残る**。後段の使用箇所（`GetDateFormatW` を呼ぶ f_443110 付近など）は追っていない。書く値が ASCII だけなので、runtime の `LCMapStringW`（0x80 以上は stop）では止まらない。

### 5.4 0x1003、0x79（STIMEFORMAT、SSHORTTIME）

- 0x446767、0x44679b で `f_448350`（ラッパー F）を呼ぶ。既定は 0（空）。引数に STIME の文字（`obj+0xc`）。内部で `f_448490`（0x448490）が書式を変換する（読み。文字集合は 0x44878c、0x4487ac、0x4487cc の表を Python でデコード: `H h s`、`m`、`t`）。
- 変換規則（読み）:
  - `H`、`h`、`s` の連続 → 小文字にして連続コピー（`H` も `h` になる）。
  - `m` の連続 → `n` の連続（0x4485bf で `dx = 0x6e`）。
  - `t` の連続の個数が 1 → `"a/p"`（0x4487f8）、2 以上 → `"AMPM"`（0x44880c）。
  - `'` で囲んだ部分 → 引用符ごとそのまま。
  - STIME の文字（`:`）→ `:`。
  - それ以外の文字 → **そのままコピー**（空白の種類、`a`、U+202F を含む）。
- したがって `"h:mm:ss tt"` → `"h:nn:ss AMPM"`、`"h:mm tt"` → `"h:nn AMPM"`。CLDR の `"h:mm:ss a"` は `a` が変換されず `"h:nn:ss a"` になり、AM/PM の印が `AMPM` にならない。**`tt`（NLS 表記）が必須**。
- 空白の種類（U+0020 か U+202F か）は、この変換では制御の分岐に影響せず、そのままコピーされる。後段で空白を特別扱いするかは追っていない（推測できない）。Windows NLS の値としては U+0020 の出典だけがあるので、U+0020 を書く。
- フォールバック: `obj+0x30` か `obj+0x38` が空なら、ITLZERO、ITIME、ITIMEMARKPOSN から `"h"`/`"hh"`、`":mm"`、`":mm:ss"`、`" AMPM"`/`"AMPM "`（0x446a4c〜0x446ab4）を組み立てる。実装でこの 2 つを失敗させる（0 を返す）と、harness は別の経路（0x25、0x23、0x1005 を問い合わせ、`h:mm AMPM` 型の文字列を組む）に入る。0007 の「fail-soft な既定値を入れない」に反するので、選択肢として挙げるだけで推奨しない。

### 5.5 0xc（SLIST）

- 0x44693e: `f_4463d0`（ラッパー B）。`GetLocaleInfoW(lcid, 0xc, buf, 3)`（バッファは 3 文字）が正なら**先頭の 1 文字**（`[rbp+0x2a]`）、そうでなければ既定の `r8w = 0x2c`（`,`）を返し、`obj+0xe` に格納。
- 結論: **先頭の 1 文字だけが効く**。値が `,` なら、失敗時の既定と同じ。ゲストの既定が `,` なので、ゲストの作者も `,` を期待している（読み）。

### 5.6 推測（出典には使わない）

ゲストの既定（`"m/d/yy"`、`"mmmm d, yyyy"`、`"am"`/`"pm"`、`,`）、`f_448490` の `m` → `n`、負の通貨の 16 要素の表（`($*)` など）は、Delphi の `TFormatSettings` の流儀に似る（推測。記憶によるもので、出典ではなく、値の決定には使っていない）。

## 6. 結論の表（決定は PM）

| LCType | 書く値 | 分類 | 根拠 | ゲストへの影響と注意 |
|---|---|---|---|---|
| 0xc SLIST | `,` | **実装してよい（公式）** | Microsoft Learn の Excel のトラブルシュート記事（§2.1 B）が en-US の既定をコンマと明言。レジストリ R1、R2 と Wine 2 件も一致 | 先頭の 1 文字だけが効く。失敗時の既定と同じ。CLDR の `;` は採らない |
| 0x1f SSHORTDATE | `M/d/yyyy` | **実装してよい（公式の例 + 独立出典 2 つ以上）** | .NET の ShortDatePattern のページ、Win32 の SSHORTDATE のページの例、PowerShell。レジストリ R1、R2、Wine 2 件 | パースされず小文字化されて残る（`m/d/yyyy`）。`yy` ではなく `yyyy` |
| 0x1003 STIMEFORMAT | `h:mm:ss tt`（U+0020） | **実装してよい（公式の例 + 独立出典 2 つ以上）** | .NET の LongTimePattern のページ、Standard の 'T' の例、PowerShell。R1、R2、Wine 2 件 | `tt` 必須（`a` は素通しになる）。空白は U+0020 の出典だけ。U+202F は ICU/CLDR の値で、Windows の値としては確認できない |
| 0x79 SSHORTTIME | `h:mm tt`（U+0020） | **実装してよい（公式の例 + 独立出典 2 つ以上）** | .NET の ShortTimePattern のページ、Win32 の SSHORTTIME のページの例、Standard の 't' の例。R1、R2、Wine 2 件 | 上と同じ。単一の書式（`;` 区切りの複数は返さない） |
| 0x20 SLONGDATE | `dddd, MMMM d, yyyy` | **実装してよい（独立出典 2 つ以上一致、非公式）。弱い** | R1、R2、Wine 現行、.NET の LongDatePattern のページ（OS 依存の注記つき）。公式の PowerShell は `dd`、Wine 5.0 も `dd` | 日の先頭ゼロの有無が後段に残る（使用箇所は未確認）。Windows の版で違う疑い（§3.1）。公式の 2 件が食い違うので「公式」にはしない |
| 0x1c INEGCURR | `0` | **実装してよい（独立出典 2 つ以上一致、非公式）。最弱。abort 維持も妥当** | R1、R2、Wine 2 件。公式の例 1 件（.NET の "C3" の en-US が `($123.456)`、ただし注意書きつき）。`1` は ICU/CLDR（issue 82178）の値 | 整数の添字として表を引く（`0` → `($*)`、`1` → `-$*`）。Windows の版による違い、PM の記憶（新しい Windows は `1`）は、出典で確認も否定もできない |

- 「確認できない」に当たるもの: 0x1c の Windows 11 の新しい版での値、時刻の書式の空白が新しい Windows で U+202F かどうか。
- 実装してよい 5 種（0xc、0x1f、0x1003、0x79、0x20）と 0x1c は、**ゲスト側の分岐を壊さない**（上の §5）。ただし、値の細部（`yy`/`yyyy`、`d`/`dd`、空白の種類、0x1c の 0/1）が、後段の整形に出るかどうかは、後段の使用箇所を追っていないので判定できていない。
- 決定の選択肢（参考）: (a) 5 種だけ実装して 0x1c は abort 維持、(b) 6 種とも実装（0042 の方針どおり、非公式と明記）、(c) 0x20 と 0x1c を abort 維持。harness は 0x1c の次に 0x1f、0x20、0x1003、0x79、0xc の順に進む（§5.1）ので、(a) や (c) でも 0x1c で止まる。

## 7. 限界と残る課題

- 実機 Windows の値は未確認（冒頭のとおり）。取得した出典に、Windows 10 22H2 の測定値はない。
- 公式の en-US の値は、ほとんどが .NET と PowerShell のページの**静的な出力例**で、Win32 の LOCALE_* ページには en-US の既定がない（SLIST だけ、Excel の記事が明言）。.NET のページ自身が「OS、OS の版、使うライブラリで出力が変わる」と書く。
- 「Data is not guaranteed to be the same from computer to computer or between runs of the application.」（https://learn.microsoft.com/en-us/windows/win32/intl/using-persistent-locale-data）。Windows の更新で値が変わり得る。
- レジストリのダンプ 2 件は、版と取得元が不明。Wine の 2 世代は同じプロジェクトで、完全には独立でない。
- 後段（`obj+0x10`、`+0x18`、`+0x30`、`+0x38`、`+0x17e` の使用箇所）は、0x1c の表の引きだけを追った。日付と時刻の書式の使われ方は追っていない。
- 撤回の条件: 実機の Windows で `GetLocaleInfoW` の値が違うと分かったら、`fle_locale.c` の該当 1 行を直す（0042 のとおり）。

## 8. 制約の遵守と触ったファイル

- 削除（`rm`）、`>` による既存ファイルの上書き、git 操作、`/tmp` の使用、コマンド文字列の `sleep` と `timeout` は、なし。ソース（`lift/crates`、`lift/runtime`）は変更していない。権限拒否はなし。
- `objdump` の出力は標準出力だけで、ファイルを作っていない。Python は標準入力から渡し、スクリプトのファイルは作っていない。`lift/out-loc3/` は存在しないので、作らなかった。
- 取得ツールが自動で保存した出力（`~/.local/share/opencode/tool-output/`）は、リポジトリの外のハーネスの領域で、私が作ったファイルではない。
- 他の subagent が作業ツリーを変更中（wasm 移植）。私はそれらに触れていない。
- **触ったファイル（新規 1 つだけ）**: `docs/investigations/locale-6-official-sources.md`
