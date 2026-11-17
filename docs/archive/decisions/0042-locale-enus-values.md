# 0042: GetLocaleInfoW の en-US の値を一括で決める（仮の決定。非公式の出典。実機 Windows 未確認）

状態: 作業者の判断。**仮**。値は MSDN のページに書かれておらず（書かれているものは 0016 で実装済み）、Wine と CLDR による。**実機の Windows では確認していない**。実装は `lift/runtime/fle_locale.c`、テストは `lift/crates/fe-lift/tests/locale.rs`。調査と実行記録は `docs/investigations/locale-enus-run.md`。0016（ロケールの方針）、0041（0x100b）、0007（fail-soft な既定値を入れない）に従う。

## 決定

- 閉包（all-pdata）がゲストから `GetLocaleInfoW` で要求する LCType は、静的に **64 種**（実行記録 §2）。このうち 7 種は 0016 と 0041 で答え済み。
- **51 種を新たに en-US（0x409。0x400、0x800 の既定値も同じロケールに解決する）で答える**（§1）。三つの出典（Wine 5.0 の `enu.nls`、Wine 現行の `locale.nls`、CLDR の en）が一致したものだけ。
- **6 種は出典が食い違うので実装せず、abort のまま**（§3）: 0xc、0x1c、0x1f、0x20、0x1003、0x79。
- 他のロケールは従来どおり `fle_require_emulated` で止まる。`LOCALE_RETURN_NUMBER` は整数型（`LOCALE_I*`）にだけ使える（従来どおり。文字列型では abort）。0x100e（SMONTHNAME13）、0x100f（SABBREVMONTHNAME13）は閉包が要求しないので答えず、abort のまま（Wine は ""。決めていない）。

## 1. 実装した 51 種の値

| LCType | 名前 | en-US の値 | 型 | Wine 5.0 `enu.nls` | Wine 現行 `locale.nls` | CLDR en | 備考 |
|---|---|---|---|---|---|---|---|
| 0x2 | SLANGUAGE | "English (United States)" | 文字列 | 同じ | 同じ | `{0} ({1})` に English と United States で同じ | MSDN ページは「Windows 7 以降は非推奨。完全なローカライズ名」と書くだけ |
| 0xe | SDECIMAL | "." | 文字列 | 同じ | 同じ | symbols decimal "." | MSDN の例（"3.14"） |
| 0xf | STHOUSAND | "," | 文字列 | 同じ | 同じ | symbols group "," | |
| 0x19 | ICURRDIGITS | 2 | 整数 | "2" | 2 | 通貨パターン `¤#,##0.00`、ISO 4217 の既定 digits=2 | MSDN の例（200.00 は 2） |
| 0x1b | ICURRENCY | 0 | 整数 | "0" | 0 | 通貨パターン `¤#,##0.00`（前置、空白なし） | MSDN の表で 0 = 前置、空白なし（"$1.1"）。CLDR からの値は表との照合で導いた（読み） |
| 0x1d | SDATE | "/" | 文字列 | "/" | 短い日付 `M/d/yyyy` の区切り | 短い日付 `M/d/yy` の区切り | MSDN は Vista 以降は非推奨と書く |
| 0x1e | STIME | ":" | 文字列 | ":" | 時刻書式 `h:mm:ss tt` の区切り | 時刻 `h:mm:ss a` の区切り | MSDN は Vista 以降は非推奨と書く |
| 0x23 | ITIME | 0 | 整数 | "0" | 0（書式が `h`） | 書式が `h`（12 時間） | MSDN の表で 0 = 12 時間（AM/PM） |
| 0x25 | ITLZERO | 0 | 整数 | "0" | 0（`h` は 1 桁） | `h` は 1 桁 | MSDN の表で 0 = 時の先頭ゼロなし |
| 0x28 | S1159 | "AM" | 文字列 | "AM" | "AM" | dayPeriod am "AM" | CLDR には別表記 "am"（variant）もある。標準の表記を採った |
| 0x29 | S2359 | "PM" | 文字列 | "PM" | "PM" | dayPeriod pm "PM" | 同上 |
| 0x2a〜0x30 | SDAYNAME1〜7 | Monday〜Sunday | 文字列 | 同じ | 同じ | 同じ（format wide） | MSDN のページ: SDAYNAME1 は "Native long name for Monday"…7 は Sunday |
| 0x31〜0x37 | SABBREVDAYNAME1〜7 | Mon〜Sun | 文字列 | 同じ | 同じ | 同じ（format abbreviated） | 0x37 = Sun（前任の記録の停止点） |
| 0x38〜0x43 | SMONTHNAME1〜12 | January〜December | 文字列 | 同じ | 同じ | 同じ（wide。単独形と format 形が en では同じ） | MSDN の注: SMONTHNAME* は単独形（主格） |
| 0x44〜0x4f | SABBREVMONTHNAME1〜12 | Jan〜Dec | 文字列 | 同じ | 同じ | 同じ | |
| 0x1005 | ITIMEMARKPOSN | 0 | 整数 | "0" | 0（`tt` が末尾） | `a` が末尾 | MSDN の表で 0 = 接尾 |
| 0x1009 | ICALENDARTYPE | 1 | 整数 | "1" | 1 | calendarPreference 001 は gregorian | **この行が最も弱い**: CLDR は「グレゴリオ暦」までしか言えず、MSDN の Calendar Identifiers の 1（CAL_GREGORIAN、ローカライズ）と 2（CAL_GREGORIAN_US、常に英語の文字列）を区別できない。1 は Wine の二つの世代が一致 |

テストは `get_locale_info_enus_0042`（51 行 × LCID 3 通り。サイズ問い合わせ、バッファ、1 文字足りない、NULL バッファ、負のサイズ、RETURN_NUMBER の整数型 6 種）。

### 突合の方法（実測）

- Wine 5.0 `dlls/kernel32/nls/enu.nls`（取得 2026-10-21、STRINGTABLE。ReactOS の `dll/win32/kernel32/winnls/nls/enu.nls` は**バイト単位で同一**。`cmp` で確認。したがって ReactOS は独立した出典ではなく、Wine 5.0 の複製）。
- Wine 現行 `nls/locale.nls`（バイナリ。取得 2026-10-21）。`include/winternl.h` の `NLS_LOCALE_DATA` の配置と、`dlls/kernelbase/locale.c` の `get_locale_info` の読み出し（曜日は `(type - SDAYNAME1 + 1) % 7` で、配列は日曜始まり。`IFIRSTDAYOFWEEK` は `(raw + 6) % 7`）に従って、`lift/out-loc2/parse_wine_nls.py` で LCID 0x409 の値を取り出した。結果は `lift/out-loc2/wine-locale-nls-en-US.txt`。
- CLDR `common/main/en.xml`（取得 2026-10-21。`en_US.xml` は空の継承だけ）と `supplementalData.xml`。CLDR は Windows の値ではない（Windows の en-US と食い違うことがある）ので、値が書式パターンなどから間接的にしか決まらないもの（上の「読み」）は、MSDN の表との照合で導いた。
- 曜日と月の短縮名は、Wine の別の表（`sabbrevdayname`、`sabbrevmonthname`）の値を全部読み、**長い名前の最初の 3 文字と一致する**ことを確認した（テストはこの性質で期待値を作る）。

## 2. MSDN が定めること、定めないこと

- 定める: 各定数の意味（曜日の番号づけ: 1 = 月曜…7 = 日曜。ICURRENCY、ITIME、ITLZERO、ITIMEMARKPOSN の値の表。ICURRDIGITS の例。SDECIMAL の例）、カレンダー識別子の番号（1 = CAL_GREGORIAN…）。
- 定めない: en-US の曜日名、月名、AM/PM、区切り、カレンダー種類の具体的な値（ページにあるのは例や意味だけ）。したがって**すべての値が非公式の出典に依存する**。

## 3. 食い違って実装しなかった 6 種（abort のまま）

| LCType | 名前 | Wine 5.0（= ReactOS） | Wine 現行 | CLDR en | 食い違いの内容 |
|---|---|---|---|---|---|
| 0xc | SLIST | "," | "," | symbols list **";"** | CLDR の list は Windows の SLIST とは別の概念（リスト区切り）である可能性が高いが、確認していない（推測）。MSDN は「リスト項目の区切り。多くのロケールでカンマ」とだけ書く。Wine の二つの世代は一致するが CLDR と食い違うので、二つ以上の一致の基準に従い実装しない |
| 0x1c | INEGCURR | "0" | 0 | 標準の通貨パターン `-¤#,##0.00`（= モード 1）、会計パターン `(¤#,##0.00)`（= モード 0） | CLDR は二つのパターンを持ち、どちらが Windows の既定かを言えない。Wine は 0（"($1.1)"）。**実機 Windows の en-US は新しい版で "-$1.1"（モード 1）である、という記憶があるが、確認していない（推測）**。決まらないので実装しない。**harness の次の停止点** |
| 0x1f | SSHORTDATE | "M/d/yyyy" | "M/d/yyyy" | "M/d/yy" | 年の桁数が食い違う |
| 0x20 | SLONGDATE | "dddd, MMMM dd, yyyy" | "dddd, MMMM d, yyyy" | "EEEE, MMMM d, y" | Wine の二つの世代の間でも日が `dd` と `d` で食い違う。CLDR は別の書式記法 |
| 0x1003 | STIMEFORMAT | "h:mm:ss tt" | "h:mm:ss tt"（先頭。配列の 2 番目は "HH:mm:ss"） | "h:mm:ss a"（最近の CLDR では `a` の前が U+202F） | 書式記法が違い（tt と a）、区切りの空白が U+202F か通常の空白かで食い違う可能性がある（Windows での扱いは未確認） |
| 0x79 | SSHORTTIME | "h:mm tt" | "h:mm tt" | "h:mm a"（U+202F） | 同上 |

- これらは日付書式（SSHORTDATE、SLONGDATE、STIMEFORMAT、SSHORTTIME）と SLIST、INEGCURR。**値が決まるまで `fle_locale_stop` で止まる**（テスト: `undefined_cases_abort` の `gli-0042-*` 6 行）。
- 値を決めるのに必要なもの: 実機 Windows（en-US）での `GetLocaleInfoW` の実測、または Microsoft が公表した en-US の値。

## 4. 閉包が `GetLocaleInfoW` 以外で要求し得るもの

- インポート表にあるのは `GetLocaleInfoW`、`GetDateFormatW`、`EnumCalendarInfoW`（ロケール系）。`GetLocaleInfoEx`、`GetCalendarInfo*`、`GetTimeFormat*`、`GetNumberFormat*`、`GetCurrencyFormat*`、`GetUserDefaultLocaleName`、`LCIDToLocaleName`、`GetSystemDefaultLCID`、`GetLocaleInfoA` は、インポート表にも DLL 内の文字列（ASCII と UTF-16）にも無い。`GetProcAddress` で名前から解決する経路は、名前の文字列が無いことだけを根拠にしており、`GetProcAddress` の呼び出し自体は調べていない（文字列を組み立てて作る場合は見つからない。推測）。
- `GetDateFormatW`（0x4431bd、0x4433c2）と `EnumCalendarInfoW`（0x447961、0x447a01、0x447a52、0x447add）は、この決定の範囲外（未実装のまま。`fle_unsupported`）。実行記録 §5 に条件を記録した。

## 限界、撤回の条件

- 値はすべて仮。実機の en-US Windows で値が違うと分かった LCType があれば、`fle_locale.c` の該当 1 行を直し、この決定の表を更新する。
- 特に ICALENDARTYPE（1 か 2 か。CLDR が区別できない）、S1159/S2359（"AM"/"PM" の表記）を最初に確認する価値がある（推測）。
- Windows の版による違い（0x1c のように版で変わる値）は確認していない。
- 出典の独立性: Wine の二つの世代は同じ project のデータ（同じ人が保守）で、完全に独立ではない（推測）。ReactOS は Wine 5.0 の複製。独立した出典は CLDR だけで、CLDR は Windows の値ではない。「二つ以上が一致」は、この意味で弱い基準であることを承知の上で採った（依頼の基準）。
