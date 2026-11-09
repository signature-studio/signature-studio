# 0072: CompareStringW — 最初の位置で ASCII の記号が英数字と向き合うとき、記号を前にする（案 A'。仮の決定）

状態: **仮の決定**。PM の指示（案 A'）に従い、作業者（サブエージェント）が実装した。「一次」は Microsoft Learn（2026-10-23 に原文を再取得して確認）、「実測」はコマンドの出力（gdb、a4-survey、テスト）、「読み」は PE のバイト列の静的な読み。調査は `docs/investigations/comparestringw-usage.md`、実行の記録は `docs/investigations/after-comparestringw.md`。

## 背景

unit 144（proc 0x65c540）で `CompareStringW(0x409, 0, L"_x64.dll", 8, L"ppv2.dll", 8)`（実測。`f_4231c0` ← `f_44ec40` ← `f_44f040` ← `f_653660` の 0x653797 ← `f_6539e0` ← `f_653f40` ← unit 144）が、`lift/runtime/fle_locale.c` の `fle_imp_CompareStringW` にある `fle_locale_stop`（「等しくない文字列に ASCII の英数字以外を含む」）で止まっていた。**停止点は `fle_locale.c`**（`fle_string.c` ではない。`0071-virtual-windows.md` と `window-design.md` の「既存の `fle_string.c` の停止」は誤り。`fle_string.c` に CompareStringW は無い）。

ゲストの鎖では戻り値は等しいか否かにだけ使われる（調査 §1.3。`f_44f040` の `sete al`）。しかし返す値は API の仕様として正しくなければならない（順序を使うゲストの箇所が別にある。調査 §1.6）ので、順序を作らない近道（常に LESS など）は採らない。

## 一次の根拠（これだけを根拠にする）

Microsoft Learn「Handling Sorting in Your Applications」
https://learn.microsoft.com/en-us/windows/win32/intl/handling-sorting-in-your-applications

> The functions CompareString, CompareStringEx, lstrcmp, lstrcmpi, LCMapString, LCMapStringEx, FindNLSString, and FindNLSStringEx all default to use of a "word sort" technique. For this type of sort, all punctuation marks and other nonalphanumeric characters, except for the hyphen and the apostrophe, come before any alphanumeric character.

CompareStringW の戻り値（1 = CSTR_LESS_THAN、2 = CSTR_EQUAL、3 = CSTR_GREATER_THAN）は CompareStringEx のページ（https://learn.microsoft.com/en-us/windows/win32/api/stringapiset/nf-stringapiset-comparestringex）による。既定（flags 0）が word sort であることは上の文による。NORM_IGNORECASE は大文字小文字を畳み込むだけで word sort を変えない（フラグの定義。記号と英数字の向き合いには効かない）。

## 決定

`fle_imp_CompareStringW`（`lift/runtime/fle_locale.c`）で、flags が 0 か NORM_IGNORECASE の emulated locale（0x409、0x400、0x800）のとき、文字列が等しくなく、**両方の長さが 1 以上**で、**位置 0 の単位が違い**、一方が ASCII の記号（0x21〜0x7E のうち英数字、ハイフン `-`、アポストロフィ `'` を除く）で、他方が ASCII の英字か数字なら、**記号の側を小さい**とする（記号が string1 なら 1、string2 なら 3）。

それ以外は従来どおり名前付きで停止する: 位置 1 以降で初めて違う場合、記号同士（`_` と `.` など）、ハイフン、アポストロフィ、空白（0x20）、制御文字、0x7F、非 ASCII、空文字列や接頭辞、大文字小文字だけの違い、英字と数字、flags が 0 と NORM_IGNORECASE 以外（SORT_STRINGSORT 0x1000 を含む）、emulated でない locale（LOCALE_INVARIANT を含む）。等しい場合の戻り値 2 と、英数字だけの文字列の既存の順序は変えない。

## 読み: なぜ位置 0 に限るか

- 上の一文は「記号は英数字の前」と言うが、**文字列のどの位置に適用されるか**を述べない。同じページは「Usually the sorting functions evaluate strings character by character」とも書き、複数文字の要素の例外があると書く。
- 位置 0 で単位が違う場合は、それより前に比べる単位が無く、先頭の単位の分類（記号か英数字か）の比較だけで決まる読みが最も素直である。位置 1 以降だと、先行する位置の差（大文字小文字だけの違い、記号の「無視」に当たるもの、複数文字の要素）との優先関係をページが述べないので、同じ強さで言えない。
- unit 144 の組（"_x64.dll" と "ppv2.dll"）と、調査でリテラルを解決できた他の組（"_x64.dll" と "ache.dll"）は、どちらも位置 0 である。必要な範囲に収まる。
- ハイフンとアポストロフィは、ページが「他の非英数字と別扱い」と書くだけで、英数字に対する位置を（表の例以外では）書かないので、決めない。空白（0x20）、制御文字、0x7F は、ページの「句読点とその他の非英数字」に入るかが（重みを持たない符号位置があるという別の記述もあり）読み切れないので決めない。

## 根拠にしないもの

Wine の `dlls/kernel32/tests/locale.c`（`strings_sorted` など）は**根拠にしない**。Wine のテストが期待する値であって Windows の実測ではない（調査 §2 に非公式として記録）。ReactOS、ブログ、記憶による値も使わない。

## 未確認（値を決めていない）

- 記号同士の順序（`_` と `.` など）。位置 0 でも停止のまま。
- ハイフンとアポストロフィの、英数字に対する位置（ページの表の例以外）。
- 位置 1 以降での「記号が前」の適用（"a_b" と "ab" など）。
- 英字と数字の順序、大文字小文字だけの違い（flags 0）、接頭辞（短い方が先か）。
- 空白、制御文字、非 ASCII の記号。
- 実 Windows（10 22H2）での `CompareStringW(0x409, 0, L"_x64.dll", 8, L"ppv2.dll", 8)` の実測値。ホストは Linux で、実測は 1 つも無い。

## テスト

`lift/crates/fe-lift/tests/locale.rs`:
- `compare_string_symbol_before_alnum`（新規）: unit 144 の組（両順）、"_x64.dll" と "ache.dll"、`_`/`.`/`!`/`{`/`~` と英字・数字、記号 + 複数文字（"_ab" と "c"）。lcid 0x409、0x400、0x800 × flags 0 と NORM_IGNORECASE × 長さ -1 と明示長。期待値は Learn の一文だけから（記号が前）。
- `compare_string_symbol_outside_rule_aborts`（新規）: 位置 1 以降の記号（"a_b" と "ab"、両順。"a.b" と "ac"。大文字小文字の違いの後の記号、NORM_IGNORECASE 付きも）、記号同士（両順）、ハイフン・アポストロフィ（両順、NORM_IGNORECASE 付きも）、空白、記号と空文字列、記号と接頭辞、非 ASCII（é、U+2022）、flags 0x2 と 0x1000、locale 0x407 と 0x7F が、SIGABRT と `fle:` のメッセージで止まること。
- 既存の 13 件は変えていない（`compare_string`、`undefined_cases_abort` の `cmp-punct`（"a" と "-"）を含む。いずれも通る）。

## 撤回条件

1. 実 Windows の測定（または Microsoft の別の文書）が、`CompareStringW(0x409, 0, L"_x64.dll", 8, L"ppv2.dll", 8)` を LESS 以外と示す、または位置 0 の記号と英数字の組のどれかで逆の順序を示す: 該当の規則を直すか、停止に戻す。
2. Learn が、word sort の「記号は英数字の前」の適用位置や、ハイフン・アポストロフィ・空白・制御文字の扱いを書く: 位置 0 の限定を外す、または対象の文字を広げる（別の決定で）。
3. ゲストが順序を使う箇所（`f_103ba10` の `setl`/`setg`、二分探索 `f_7890d0` など。調査 §1.6）に届き、位置 1 以降の記号や記号同士の組で止まる: その組を、その時に別の決定で決める。この決定の範囲を暗黙に広げない。
4. PM が環境の定義（0014 の Windows 10 22H2、en-US）を変える。

## 仮の決定の一覧（`docs/investigations/provisional-decisions.md`）への追記

**追記を推奨する**（作業者は編集していない。PM の判断）。理由: この決定は「仮」と書いてあり、一次の根拠は 1 文だが、位置 0 への限定が読みで、実機の値は未確認。出典の種別は「一次（Learn の 1 文）。位置 0 への適用は読み」、撤回条件は上の 1〜3 の要約になる。なお一覧の対象は 0001〜0070 で、0071 も未掲載。

## 影響範囲

`lift/runtime/fle_locale.c`（`fle_is_sort_symbol` の追加と `fle_imp_CompareStringW` の 1 分岐、コメント）と `lift/crates/fe-lift/tests/locale.rs`（2 件追加）のみ。生成 C、`imports_table.rs`、他のランタイムファイルは変えていない。
