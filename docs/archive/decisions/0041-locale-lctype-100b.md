# 0041: GetLocaleInfoW LCType 0x100b（LOCALE_IOPTIONALCALENDAR）に en-US で "0" を答える（仮の決定。非公式の出典）

状態: 作業者の判断。**仮**。値は MSDN のページに書かれておらず、非公式の出典（Wine）による。実機 Windows は未確認。実機の値が "0" 以外と分かれば撤回する。調査と実行記録は `docs/investigations/locale-100b-run.md`。実装は `lift/runtime/fle_locale.c`（応答表に 1 行追加）、テストは `lift/crates/fe-lift/tests/locale.rs`。0016（ロケールの方針）、0007（fail-soft な既定値を入れない）に従う。

## 決定

- LCType 0x100b は `LOCALE_IOPTIONALCALENDAR`（§1）。en-US（0x0409。0x400、0x800 の既定値も同じロケールに解決する）で、文字列 `"0"`（整数型。`LOCALE_RETURN_NUMBER` と組み合わせると DWORD 0、戻り値 2）を返す。
- ほかのロケールは従来どおり `fle_locale_stop` で止まる（`fle_require_emulated`）。ほかの LCType（隣の 0x1009 `LOCALE_ICALENDARTYPE`、0x100c `LOCALE_IFIRSTDAYOFWEEK` など）は応答表に加えていない。abort のまま。

## 根拠

### 1. 定数の同定（公式ではない出典。MSDN は名前の一覧だけ）

- MSDN の「Locale Information Constants」（https://learn.microsoft.com/en-us/windows/win32/intl/locale-information-constants、取得 2026-10-21）は、LOCALE_* の名前を並べるだけで、**数値は書かれていない**。`LOCALE_IOPTIONALCALENDAR` のページ（…/locale-ioptionalcalendar）にも数値はない。
- 数値 0x100b は、非公式の第三者ヘッダ 2 つが一致する: Wine の `include/winnls.h`（`#define LOCALE_IOPTIONALCALENDAR 0x100B`）と mingw-w64 の `winnls.h`（`#define LOCALE_IOPTIONALCALENDAR 0x0000100b`）。なお、Microsoft の `winnls.h` と称するコピーが、前の作業のツール出力ファイルとして作業環境に残っており（`#define LOCALE_IOPTIONALCALENDAR 0x0000100B   // additional calendar types specifier, eg CAL_GREGORIAN_US`）、同じ値だった。ただし取得元の URL は今回の作業では確認していないので、出典としては数えない。
- 0x100b は 0x1000 | 0x000b の分解ではない: 0x1000 は LOCALE_NOUSEROVERRIDE（実際は 0x80000000）ではなく、LCType の下位 16 ビットの値の一部。`LOCALE_NOUSEROVERRIDE` は 0x80000000、`LOCALE_RETURN_NUMBER` は 0x20000000（mingw-w64、Wine のヘッダ。`fle_locale.c` の既存定数と一致）。`fle_locale.c` は `lctype & ~(…)` で上位ビットを外してから応答表を引くので、0x100b はそのまま `FLE_LC_IOPTIONALCALENDAR`。

### 2. MSDN が定めること、定めないこと

- 定める（GetLocaleInfoEx のページの LCType の説明）: `LOCALE_IOPTIONALCALENDAR` のとき、関数は「最初の代替カレンダーだけ」を返す。全部は EnumCalendarInfoEx で取る。`LOCALE_IOPTIONALCALENDAR` のページは、「そのロケールで使える任意のカレンダーの種類」とだけ書く（値の形式、en-US の値は書かない）。カレンダー識別子は「Calendar Identifiers」のページ（1 = CAL_GREGORIAN … 23 = CAL_UMALQURA）。
- **定めない**: 文字列の形式（整数型 `LOCALE_I*` で、`LOCALE_RETURN_NUMBER` と組み合わせられるので、数値の十進文字列と読める。これは GetLocaleInfoEx のページの Remarks「それ以外の数値情報は十進数のテキスト」に基づく）、そして**en-US の値**。つまり「仕様から値が決まらない」ケース。

### 3. 値 "0" の出典（すべて非公式）

- Wine の en-US ロケールのリソース（https://raw.githubusercontent.com/wine-mirror/wine/wine-5.0/dlls/kernel32/nls/enu.nls、wine-5.0 のタグ。中身は STRINGTABLE のリソース記述）: `LOCALE_IOPTIONALCALENDAR "0"`、`LOCALE_ICALENDARTYPE "1"`。
- Wine の現行 `dlls/kernelbase/locale.c`（取得 2026-10-21）: `case LOCALE_IOPTIONALCALENDAR: return locale_return_number( locale_strings[locale->scalendartype + 2], … )`。locale.nls の calendar type 配列（先頭がカレンダー数、次が ICALENDARTYPE、その次が最初の代替）の 3 番目を数値で返す実装。en-US の配列は 1 要素（Gregorian）で、代替がなければ 0 になる（推測。locale.nls は取得できなかったので、配列の中身は未確認）。
- Wine のテスト `dlls/kernel32/tests/locale.c` の `invariant_list`: LOCALE_INVARIANT で `LOCALE_ICALENDARTYPE` "1"、`LOCALE_IOPTIONALCALENDAR` "0"。**これは en-US ではなく invariant の値**で、出典の補強にとどまる。Wine のテストは実機 Windows でも走らせる前提で書かれている（一般知識。確認していない）。
- .NET の書籍（flylib.com の「.NET Internationalization」。非公式、古い）: en-US の `CultureInfo.Calendar` は GregorianCalendar、`OptionalCalendars[0]` が既定のカレンダー。ar-SA は Hijri が既定で、Gregorian の各種（USEnglish など）が代替。en-US の `OptionalCalendars` の一覧は書かれていない。
- CLDR（`supplementalData.xml`、取得 2026-10-21）の `calendarPreference` の 001（世界）は gregorian。CLDR は Windows の値ではないので、en-US に代替カレンダーがないことの裏付けにはならない（参考）。

つまり、**en-US の値 "0" を直接定めるのは Wine の en-US データ（およびその現行の読み出し実装）だけ**で、MSDN にも CLDR にも en-US の値はない。

### 4. ゲストの使い方（静的、objdump の読み）

`f_447860`（VA 0x447860。ロケール初期化。前任の記録の呼び出し鎖の 1 つ上）は、0x44790x で `f_446370`（VA 0x446370。`GetLocaleInfoW` のラッパー。r8d = LCType = 0x100b、edx = LCID）を呼んで文字列を得て、`f_43aae0`（VA 0x43aae0。`call 0x40e3c0` で文字列を整数に変え、変換に失敗したら edx（ここでは 1）を返す形。StrToIntDef(文字列, 1) に見える。推測）にかける。結果 eax は `cmp eax,7; ja` の後、`1 << eax` を 2 つのバイトのビット集合と `test` する:

- VA 0x447dd0 のバイトは `0x38`（ビット 3、4、5）。0x447dd1 のバイトは `0x06`（ビット 1、2）。つまり結果が {3, 4, 5} か {1, 2} かで分岐する。MSDN のカレンダー識別子では 3 = CAL_JAPAN、4 = CAL_TAIWAN、5 = CAL_KOREA、1 = CAL_GREGORIAN、2 = CAL_GREGORIAN_US（コードの意図は読みであり、確認していない）。
- {3, 4, 5} の枝は `EnumCalendarInfoW`（VA 0x4232f0 の IAT jmp。0x447961、0x447a01 の 2 回）、{1, 2} の枝も同じ API（0x447a52、0x447add の 2 回）を呼ぶ（`import-locale-spec.md` §4 にある 4 回の呼び出しと一致）。**この API は runtime に未実装**で、到達すると `fle_unsupported` で止まる。
- どちらの集合にも入らない値（"0" の 0 を含む）は、`EnumCalendarInfoW` を呼ばずに 0x447c17 へ進む（オブジェクトの +0x170 のコピー、ロケール初期化の締め）。

したがって、"0" を答えることは、**`EnumCalendarInfoW`（未実装）に到達する枝を通らない**という実行上の帰結を持つ。これは値を選んだ理由ではなく（値は出典で決めた）、副次的な結果として記録する。値が "1" や "2" だったなら、この API の実装が次に必要になっていた。

## 限界、撤回の条件

- 値は仮。実機の en-US Windows で `GetLocaleInfoW(0x409, LOCALE_IOPTIONALCALENDAR, …)` が "0" 以外を返すと分かれば、`fle_locale.c` の 1 行を直す。Windows のバージョンによって値が違う可能性（en-US に代替カレンダーが追加されるなど）は確認していない。
- LCID 0x409 の `LOCALE_NOUSEROVERRIDE` 付きも同じ値を返す（環境にユーザー上書きがないため。0016 と同じ扱い）。
- 隣の LCType（`LOCALE_ICALENDARTYPE` = 0x1009 の "1"、`LOCALE_IFIRSTDAYOFWEEK` = 0x100c など）は、同じ Wine の資料に値があっても、必要になるまで加えない（範囲外）。テストに、この 2 つが abort のままであることの行を加えた。
