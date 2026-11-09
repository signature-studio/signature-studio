# 0016: エミュレート ロケール（SetThreadLocale ほか NLS 系）

状態: 作業者の判断（依頼者の「判断は作業者側が行え」と、PM の指示の範囲）。仕様の出典と実装の一覧は `docs/investigations/import-locale-spec.md`。実行記録は `docs/investigations/locale-run.md`。実装は `lift/runtime/fle_locale.h`、`fle_locale.c`。応答表への追記は `lift/crates/fe-lift/src/imports_table.rs`（13 行）。

## 決定

### 1. 定数は 1 か所（`fle_emulated_locale`、`fle_locale.c`）

| 項目 | 値 | 根拠 |
|---|---|---|
| ロケール（ユーザー既定 = システム既定 = スレッドの初期ロケール） | en-US、LCID 0x0409 | この lift 環境の設定 |
| ANSI コードページ | 1252 | この lift 環境の設定 |
| OEM コードページ | 437 | この lift 環境の設定 |
| UI 言語（Get{User,System}DefaultUILanguage） | 0x0409（LCID の下位 16 ビット） | 上のロケールから導く |

**これは仕様ではない**。MSDN の GetACP、GetThreadLocale、LOCALE_USER_DEFAULT などのページは、既定のロケール ID も ANSI/OEM コードページも定めていない（システムに依存する、とだけある）。値はこの lift 環境の設定で、変えるときは `fle_emulated_locale` だけを変える。既存の runtime（`fle_osver.c` ほか）にロケール系の既定値はなく（`grep` で 0 件。docs にもロケールの決定はなかった）、合わせる対象はなかった。依頼にあった旧調査の「en-US 0x0409 / CP1252」は、リポジトリの docs と runtime からは見つからなかった（依頼文のとおりに採用した）。

制限: コードは、この 2 つのコードページが単一バイトで ASCII 互換であることを前提にしている（1252 と 437 について。一般知識で、MSDN のページの記述ではない）。DBCS のコードページに変えるときは、`fle_locale.c` の変換と GetCPInfo を直す必要がある。

### 2. 仕様から決まるものは実装し、決まらないものは abort

- 仕様のページで規則が確認でき、en-US の範囲で実装できるものだけ実装した。範囲外（他のロケール、ページにない LCTYPE、ページが結果を定めない入力、表が要るバイト値）は `fle_locale_stop`（メッセージを出して `abort()`）で止める。黙って値を作らない。
- 各 API の規則と、どこが仕様でどこが読みか、は `import-locale-spec.md`。

### 3. ロケール ID の扱い

- 「この環境が持つロケール」= en-US（0x0409）、LOCALE_USER_DEFAULT（0x0400）、LOCALE_SYSTEM_DEFAULT（0x0800）、LOCALE_INVARIANT（0x007F）。値はいずれも MSDN の各定数ページに書かれた値。2 つの既定値は 0x0409 に解決する。
- IsValidLocale は、この 4 つだけ TRUE。ドイツ語（0x0407）など、Windows が持つがこの環境が持たないロケールは、LCID_SUPPORTED でも FALSE（持たないロケールを持つと偽らない）。これは環境の選択であり、実際の Windows の挙動ではない。
- SetThreadLocale は、この 4 つのとき TRUE を返してスレッドのロケールを設定する（0x400、0x800 は 0x0409 に解決。0x7F はそのまま）。それ以外は FALSE と ERROR_INVALID_PARAMETER（87。ページはエラーコードを定めない。作業者の選択）。
- スレッドのロケールはスレッドごと（`__thread`）。新しいスレッドは、ユーザーロケールで始まる（GetThreadLocale の Remarks と SetThreadLocale の Remarks の一文）。GetThreadLocale のページの別の一文は「新しいスレッドは作成元のロケールを継承する」で、2 つのページが一致しない。runtime は前者に従う。FLEngine 閉包は 1 スレットなので、実行には影響しない（推測）。
- SetThreadLocale の戻り値: Syntax は `BOOL`、Return value の文は「LCID を返す。これは以前のスレッドロケールの LCID」で、ページ内で食い違う。`BOOL`（成功で TRUE = 1）を採った。依頼の指示もこれ。呼び出し元 0x41ce46 は、戻り値を読まない（objdump の読み）。

### 4. 他の担当のファイルとの関係

- `imports_table.rs` へは、`HeapFree` の行の後に 13 行を追記した（既存行は変えていない。別のサブエージェントが同じ表へ追記していたので、編集の直前に読み直した）。応答表は 28 → 41 行になり、行の順序が擬似 VA を決める（0015 の §5）。
- `fle_set_last_error`（`fle_imports.c`）を使う。`fle_imports.c`、`fle_imports.h` は変えていない。
- `lift/runtime/fle_env.h` は、この作業のものではない（別の担当）。

## 限界

- 実行環境が Linux で、Windows の NLS とは照合していない。値と順序は MSDN のページと、ページにある例だけが根拠。
- CompareStringW は部分集合だけ（ASCII 英数字、フラグ 0 と NORM_IGNORECASE）。接頭辞の関係、大文字小文字だけの違い（NORM_IGNORECASE なし）、記号、他のフラグは abort。en-US の照合表は MSDN のページにない。
- LCMapStringW は ASCII の大文字化、小文字化だけ。ASCII 以外の文字、ソートキーなどのフラグは abort。（**0087 の注記（2026-10-24）**: dwMapFlags がちょうど LCMAP_LOWERCASE／LCMAP_UPPERCASE のとき、範囲 D（U+0000〜U+00FF と 28 単位）の文字を MS-UCODEREF 3.1.5.3 の Windows 8 の大文字表（小文字はその逆引き）で変換し、Locale = 0 をこの 2 フラグに限り 0x400 と同じに受け付ける。LINGUISTIC_CASING 付きなど他のフラグ、D の外の文字は従来どおり abort。`fle_locale_resolve` と GetLocaleInfoW など他の関数は変えない。この項目は、本文を書き換えず、注記だけを足した。）
- MultiByteToWideChar、WideCharToMultiByte は UTF-8 と、ANSI/OEM の ASCII（0x80 未満）だけ。0x80 以上は 1252、437 の表が MSDN のページにないので abort。（**0084 の注記**: MultiByteToWideChar の cp1252 は 0084 で表（Microsoft のコードページデータファイル）に従い変換する。**0087 の注記（2026-10-24）**: WideCharToMultiByte の cp1252 も、範囲 D の文字に限り、同じデータファイルの WCTABLE（dwFlags 0 と 0x400。0x400 は MBTABLE の厳密な逆引き 256 件に限る）で変換する。「表がページに無ければ停止」の例外は、cp1252 の MB→WC（0084）と、この WC→MB の範囲 D（0087）。cp437 と他のコードページは従来どおり。**GetStringTypeW**（0078 の名前付き停止）も、0087 で CT_CTYPE1 と範囲 D に限り、非公式の Wine 10.0 の値を仮に採って答える（出典の格は 0087 の表）。本文は書き換えていない。）
- EnumCalendarInfoW、EnumSystemLocalesW、GetDateFormatW は実装していない。理由は `import-locale-spec.md`。この 3 つに達したら、`fle_unsupported`（IAT 経由の jmp、対応表になし）で止まる。
