# ロケール系 import の仕様の出典と、実装した／しなかったの一覧

作業者の調査。判断は `docs/decisions/0016-locale.md`。仕様は learn.microsoft.com のページ（取得日 2026-10-20）。ページの本文を読んだものだけ「確認」と書く。「読み」は、ページの文から作業者が導いたもので、ページが明示していない。

## 1. 閉包で呼ばれるロケール系 import（件数 = 閉包の `jmp [IAT]` の thunk の数）

件数は `lift/out-usage/iat-slot-counts-v3.tsv` の count と、生成器の出力（`lift/out-locale/gen-locale-896.txt` の `jmp_imp:`）が一致した。呼び出し元は `lift/out-usage/import-callers-v3.tsv`（関数単位）と objdump（thunk 単位）。

| API | 件数 | 呼び出し元（関数。thunk の VA） |
|---|---|---|
| SetThreadLocale | 1 | thunk 0x407120。call 箇所 0x41ce46（`ecx = 0x400`）。.text には同じスロットへの thunk が 0x423c00 にもあるが、閉包の件数は 1 |
| GetThreadLocale | 1 | thunk 0x423730。call 箇所 0x443188、0x44338d、0x4464d6、0x446b63 |
| IsValidLocale | 2 | thunk 0x407130（call 0x419251、0x4195ec。dwFlags は `edx = 2`）、thunk 0x423960（call 0x4464cd。`edx = 1`） |
| GetUserDefaultUILanguage | 1 | thunk 0x407100。call 0x41a0ae、0x41a2c2 |
| GetSystemDefaultUILanguage | 1 | thunk 0x407110。call 0x419640、0x41966d、0x41a306 |
| GetACP | 2 | thunk 0x4070a0（call 0x41cecf）、thunk 0x4233f0（call 0x4512be、0x45281d、0x4529bb） |
| IsDBCSLeadByteEx | 1 | thunk 0x407140。call 0x40d8e6、0x40e9a3（`ecx = [0x15e6a28]`。0x41ced4 が GetACP の結果をここへ保存する） |
| GetCPInfo | 1 | thunk 0x423400。call 0x44ba62、0x4529fb（`ecx = [rax+0x18]`） |
| GetLocaleInfoW | 3 | thunk 0x4070f0（call 0x41927a、0x419290、0x41a0c5）、0x423560（call 0x446392、0x4463e6）、0x423570（call 0x44d614）。0x81c740 と 0x8da480 の呼び出し（`ecx = 0x400`）は 0x423560、0x423570 経由 |
| LCMapStringW | 1 | thunk 0x423970。call 0x4505f4、0x4506b4（0x4505e0 で `edx = 0x1000100`） |
| CompareStringW | 3 | thunk 0x407160（call 0x40f5ef、0x40f6c3）、0x4231c0（call 0x4399c5、0x439a38、0x44ecf6、0x4558ee）、0x4231e0（call 0x439a77、0x439aba、0x439bd3、0x439c66）。`ecx = 0x400`、flags は 0 か 1 |
| MultiByteToWideChar | 1 | thunk 0x4070d0。call 0x40f675、0x40f69c（`ecx = 0xfde9` = 65001）、0x41c9af、0x41ca18。0x423a40 にも thunk があるが閉包の件数は 1 |
| WideCharToMultiByte | 2 | thunk 0x4070e0（call 0x41c8f0）、0x423d70（call 0x448bb9、0x448c06） |
| EnumCalendarInfoW | 1 | thunk 0x4232f0。call 0x447961、0x447a01、0x447a52、0x447add |
| EnumSystemLocalesW | 1 | thunk 0x423300。call 0x44d984 |
| GetDateFormatW | 1 | thunk 0x423480。call 0x4431bd、0x4433c2 |

注意: 件数は `lift/out-usage/iat-slot-counts-v3.tsv` と生成器の `jmp_imp:` の数（閉包の thunk の数）で、生成器の出力（`gen-locale-896.txt`）と一致した。thunk の VA と call 箇所は `objdump -d` の `jmp [rip+…] # 0x19…` と `call 0x4…` の grep で拾った（`lift/out-locale/tmp/text-401000-4a0000.txt`、0x401000〜0x4a0000）。0x4a0000 より後（0x81c740、0x8da480、0x988d50、0x103ba10 など）の呼び出しは、この grep の範囲外で、`import-callers-v3.tsv` の関数名だけを根拠にしている。call 箇所を全部拾えたかは確認していない。

## 2. API ごとの仕様の出典、確認状況、実装

| API | 出典 URL（learn.microsoft.com/en-us/…） | 確認 | 実装 |
|---|---|---|---|
| SetThreadLocale | windows/win32/api/winnls/nf-winnls-setthreadlocale | 確認。Syntax は BOOL、Return value は「LCID（以前のスレッドロケール）」で食い違う。BOOL を採用 | 実装 |
| GetThreadLocale | …/nf-winnls-getthreadlocale | 確認 | 実装 |
| IsValidLocale | …/nf-winnls-isvalidlocale | 確認。dwFlags は LCID_INSTALLED、LCID_SUPPORTED（0x39 は「使うな」） | 実装（他の flags は abort） |
| Get{User,System}DefaultUILanguage | …/nf-winnls-getuserdefaultuilanguage、…/nf-winnls-getsystemdefaultuilanguage | 確認。値はページにない（環境の設定） | 実装 |
| GetACP | …/nf-winnls-getacp | 確認。値はページにない（環境の設定） | 実装 |
| IsDBCSLeadByteEx | …/nf-winnls-isdbcsleadbyteex | 確認。「932、936、949、950、1361 でだけ検証する」 | 部分実装（他のコードページは 0。上の 5 つは abort。範囲がページにない） |
| GetCPInfo | …/nf-winnls-getcpinfo、…/nf-winnls-getcpinfoexa、…/ns-winnls-cpinfo | 確認。CPINFO は 20 バイト（MAX_DEFAULTCHAR 2、MAX_LEADBYTES 12 は mingw-w64 の winnls.h） | 部分実装（ANSI、OEM の 2 コードページだけ。他は abort） |
| GetLocaleInfoW | …/nf-winnls-getlocaleinfow、…/nf-winnls-getlocaleinfoex、windows/win32/intl/locale-information-constants、…/locale-ilanguage、…/locale-sabbrev-constants、…/locale-siso-constants、…/locale-sname、…/locale-idefault-constants、…/locale-scurrency、…/locale-return-constants | 確認（サイズ規則、エラーコード、RETURN_NUMBER）。値は en-US で、ページにあるもの | 部分実装（下の LCTYPE 表の 10 種。他は abort） |
| MultiByteToWideChar | windows/win32/api/stringapiset/nf-stringapiset-multibytetowidechar | 確認（-1、0、サイズ、flags、エラーコード）。UTF-8 は Unicode 4.1 準拠、とある | 部分実装（UTF-8、ASCII） |
| WideCharToMultiByte | …/nf-stringapiset-widechartomultibyte | 確認 | 部分実装（UTF-8、ASCII） |
| LCMapStringW | windows/win32/api/winnls/nf-winnls-lcmapstringw、…/nf-winnls-lcmapstringex、windows/win32/intl/handling-sorting-in-your-applications | 確認（フラグ、サイズ規則、「i は I に」） | 部分実装（ASCII の LCMAP_UPPERCASE、LCMAP_LOWERCASE） |
| CompareStringW | windows/win32/api/stringapiset/nf-stringapiset-comparestringw、…/nf-stringapiset-comparestringex、…/handling-sorting-in-your-applications | 確認（戻り値 1、2、3、負の長さ、フラグ、エラー）。en-US の照合表は**ページにない** | 部分実装（ASCII 英数字、フラグ 0 と NORM_IGNORECASE） |
| EnumCalendarInfoW | …/nf-winnls-enumcalendarinfow | 確認。コールバックにカレンダー情報を渡す。en-US の値のページ（Calendar Type Information）は未取得 | **未実装**（理由は §4） |
| EnumSystemLocalesW | …/nf-winnls-enumsystemlocalesw | 確認。ロケール ID を 1 つずつコールバックへ渡す | **未実装**（理由は §4） |
| GetDateFormatW | （未取得） | 未確認 | **未実装**（理由は §4） |
| LOCALE_USER_DEFAULT、LOCALE_SYSTEM_DEFAULT、LOCALE_INVARIANT | windows/win32/intl/locale-user-default（0x0400）、…/locale-system-default（0x0800）、…/locale-invariant（0x007f） | 確認 | 値として使う |

### GetLocaleInfoW の LCTYPE（en-US、実装した 10 種）

| LCTYPE（値） | 値 | ページの根拠 |
|---|---|---|
| LOCALE_ILANGUAGE（0x1） | "0409" | locale-ilanguage に "English (United States) has the value 0409" |
| LOCALE_SABBREVLANGNAME（0x3） | "ENU" | locale-sabbrev-constants に例 "ENU" |
| LOCALE_SISO639LANGNAME（0x59） | "en" | locale-siso-constants に例 "en" |
| LOCALE_SISO3166CTRYNAME（0x5a） | "US" | 同、例 "US" |
| LOCALE_SISO639LANGNAME2（0x67） | "eng" | 同、例 "eng" |
| LOCALE_SISO3166CTRYNAME2（0x68） | "USA" | 同、例 "USA" |
| LOCALE_SNAME（0x5c） | "en-US" | locale-names に "en-US" |
| LOCALE_SCURRENCY（0x14） | "$" | locale-scurrency に「米国では "$"」 |
| LOCALE_IDEFAULTANSICODEPAGE（0x1004） | "1252" | 環境の設定（ページは値を定めない） |
| LOCALE_IDEFAULTCODEPAGE（0xb） | "437" | 環境の設定 |

閉包の呼び出し元が即値で渡す LCTYPE（objdump の読み。ラッパー 0x446370 の `r8d` と 0x4463d0 の `edx`、および 0x419281、0x4192xx、0x41a0b6 の直接の呼び出し）: 0x14、0x1b、0x1c、0x28、0x29、0x19、0x25、0x23、0x1005、0x100b、0x1009（0x446370 経由）、0xf、0xe、0x1d、0x1e、0xc（0x4463d0 経由）、0x59、0x5a（0x419274 付近と 0x8da4c2 の呼び出し）、0x3（0x41a0b6）。ほかにレジスタやメモリから渡す呼び出し（0x447e02、0x447e5f、0x44837e など）があり、値は未確認。**このうち実装したのは 0x14、0x59、0x5a、0x3 の 4 つだけ**で、残りは実行が到達したときに `fle_locale_stop` で止まる（黙って値を作らない）。数値形式、日付形式、カレンダー関連などは、値の根拠となる en-US のページを取得していない。LOCALE_ILANGUAGE（0x1）などは、閉包の即値には見つからなかったが、ページに値があるので実装した。

## 3. 実装した規則と、そこでの読み、選択（要点）

- **サイズ規則（GetLocaleInfoW、MultiByteToWideChar、WideCharToMultiByte、LCMapStringW）**: サイズ 0 は必要サイズを返し、バッファを使わない。バッファ不足は 0 と ERROR_INSUFFICIENT_BUFFER（122）。成功は NUL を含む文字数（ページ）。バッファ不足のときバッファに何も書かない（ページは書かれる内容を定めない。選択）。
- **ERROR_INVALID_PARAMETER（87）**: NULL のバッファ付きサイズ、負のバッファサイズ、変換元と変換先が同じポインタ、変換元の NULL、入力長 0（各ページ）。
- **UTF-8**: flags は 0 か MB_ERR_INVALID_CHARS（変換）、0 か WC_ERR_INVALID_CHARS（逆変換）。他は ERROR_INVALID_FLAGS（1004）。WideCharToMultiByte の UTF-8 では lpDefaultChar と lpUsedDefaultChar が NULL でなければ ERROR_INVALID_PARAMETER（ページ）。
- **不正な UTF-8**: MB_ERR_INVALID_CHARS 付きは ERROR_NO_UNICODE_TRANSLATION（1113）。**なしは abort**（ページは「U+FFFD に置き換える」とだけ書き、1 つの不正列を何個の U+FFFD にするかを定めない）。
- **UTF-16 の孤立サロゲート**: WC_ERR_INVALID_CHARS 付きは 1113。なしは U+FFFD に置き換える（ページ。1 つの孤立単位につき 1 つ、は読み）。
- **LCMapStringW**: 「小文字の i は、LCMAP_LINGUISTIC_CASING がなければ、どのロケールでも I に写る」（ページ）ので、ASCII では結果がロケールに依存しない。LCMAP_LINGUISTIC_CASING だけ（大文字小文字フラグなし）は ERROR_INVALID_FLAGS（ページ「このフラグは LOWERCASE か UPPERCASE と使うときだけ有効」の読み）。
- **CompareStringW**: 実装した規則は、(1) 同一（NORM_IGNORECASE ありなら ASCII 畳み込み後に同一）なら CSTR_EQUAL（フラグの定義）、(2) 英数字だけの文字列で、最初に違う位置が文字同士（または数字同士）なら、その順序（「Handling Sorting」のページの例が、文字のアルファベット順を示す）。大文字小文字が主要な差に勝つかどうかは読み。

## 4. 実装しなかったもの（理由つき）

| API / 場合 | 理由 |
|---|---|
| EnumCalendarInfoW | コールバックを呼ぶ API。`fle_dispatch` でゲストのコールバックを呼ぶ実装は可能だが、渡す文字列（en-US のカレンダー情報、CAL_* の種類ごと）が、取得したページにない。呼び出し元は 0x447860 のロケール初期化で、4 回呼ぶ（0x447961、0x447a01、0x447a52、0x447add。CalType は 4 か 3）。値を作らないため未実装 |
| EnumSystemLocalesW | コールバックにロケール ID を渡す。「この環境が持つロケール」の一覧は決めたが（0x0409 だけ）、flags（LCID_INSTALLED など）ごとの一覧と順序の規則がページにない。呼び出し元は 0x44d930（0x44db27 から） |
| GetDateFormatW | 日付書式の規則（書式文字列の展開）と en-US の書式が、取得したページにない。ページ自体を取得していない |
| GetLocaleInfoW の他の LCTYPE | 上のとおり、ページに en-US の値がない |
| CompareStringW の残り | en-US の照合表、記号と句読点の順序、接頭辞、大文字小文字のタイブレーク、NORM_* のフラグはページにない |
| LCMapStringW の残り | Unicode の大文字小文字表、ソートキー、幅、かな、中国語、タイトルケース、ハッシュ、バイト反転はページにない |
| コードページ変換の 0x80 以上 | 1252、437 の表がページにない（Code Page Identifiers のページは識別子の一覧だけ） |
| IsDBCSLeadByteEx の DBCS コードページ | 先頭バイトの範囲がページにない |

## 5. テストの期待値の根拠

`lift/crates/fe-lift/tests/locale.rs`（11 件）。期待値は C のコードとは別に、Rust で書いた。UTF-8 と UTF-16 の期待値は Rust の `str` の符号化（`encode_utf16`、`from_utf16_lossy`、`as_bytes`）、大文字小文字は `to_ascii_uppercase`、`to_ascii_lowercase`、照合は `ref_cmp`、サイズ、エラーコード、CPINFO のバイト列、LCTYPE の文字列は上の表（ページ）から。abort の 31 ケースは、全て SIGABRT で止まり、戻り値を返さない。
