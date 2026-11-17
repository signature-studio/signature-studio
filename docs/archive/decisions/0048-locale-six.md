# 0048: GetLocaleInfoW の en-US の 6 種（SLIST、INEGCURR、SSHORTDATE、SLONGDATE、STIMEFORMAT、SSHORTTIME）を答える（仮の決定。実機 Windows 未確認）

状態: PM の決定。**仮**。**実機の Windows では確認していない**。0042 §3 で「出典が食い違う」として abort のままにした 6 種を、調査 `docs/investigations/locale-6-official-sources.md` の結果に 0007/0042 の基準（公式、または独立した複数の出典の一致）を当てはめて実装する。実装は `lift/runtime/fle_locale.c`、テストは `lift/crates/fe-lift/tests/locale.rs`、実行記録は `docs/investigations/locale-six-run.md`。

## 決定

en-US（0x409。0x400、0x800 の既定値も同じロケールに解決する）で、次の値を返す。他のロケールは従来どおり abort。

| LCType | 名前 | 値 | 型 | 根拠の強さ |
|---|---|---|---|---|
| 0xc | SLIST | `,` | 文字列 | **強（公式）**: Microsoft Learn の Excel 記事「The comma is the default list separator for US - English Locale.」。レジストリのダンプ 2 件、Wine 2 世代も一致 |
| 0x1f | SSHORTDATE | `M/d/yyyy` | 文字列 | **強**: 公式の例（.NET の ShortDatePattern、Win32 の SSHORTDATE ページの書式例、PowerShell）＋独立出典（レジストリ 2 件、Wine 2 世代） |
| 0x1003 | STIMEFORMAT | `h:mm:ss tt` | 文字列 | **強〜中**: 公式の例（.NET の LongTimePattern、'T' の例、PowerShell）＋独立出典（同上） |
| 0x79 | SSHORTTIME | `h:mm tt` | 文字列 | **強〜中**: 公式の例（.NET の ShortTimePattern、Win32 の SSHORTTIME ページの例、't' の例、PowerShell）＋独立出典（同上） |
| 0x20 | SLONGDATE | `dddd, MMMM d, yyyy` | 文字列 | **弱い**: 公式の例が割れる（.NET は `d`、PowerShell は `dd`。Wine 5.0 は `dd`、Wine 現行とレジストリ 2 件は `d`）。`d` を採った（レジストリ 2 件と Wine 現行と .NET）。`dd` の可能性が残る |
| 0x1c | INEGCURR | `0` | **整数** | **最も弱い**: 非公式の Wine 2 世代とレジストリ 2 件、公式の例は .NET の "C3" の en-US が `($123.456)`（モード 0 と等価、ただし注意書きつき）の 1 件だけ。ICU/CLDR の en-US は `1`（dotnet/runtime issue 82178）。Windows の版で違う疑い（PM の記憶では新しい Windows は `1`）を、出典で確認も否定もできていない |

- 時刻の書式の空白は通常の空白 U+0020。U+202F ではない（U+202F は ICU/CLDR の値。Windows の値としては確認できない）。`tt` は AM/PM 2 文字の書式記号（NLS 表記。CLDR の `a` ではない）。全部 ASCII。
- 0x1c は整数型。`LOCALE_RETURN_NUMBER` は 0x1004、0x100b、0xb と同じ扱い（DWORD 1 つ = 2 文字、戻り値 2、cchData が 2 未満は ERROR_INSUFFICIENT_BUFFER、cchData 0 は abort）。残りの 5 種は文字列型で、`LOCALE_RETURN_NUMBER` を付けると従来どおり abort。
- 他の細部（サイズ問い合わせ、1 文字足りない、NULL バッファ、負のサイズ、`LOCALE_NOUSEROVERRIDE`）は既存の流儀のまま。

## 0042 との関係

- 0042 は「三つの出典が一致」したものだけを実装し、6 種を abort のままにした。0048 は、その後の調査（Microsoft Learn の公式ページの en-US の例と、独立した出典の突き合わせ）で、0042 の基準（公式、または独立した複数の出典の一致）を **満たしたものを実装する**。0042 の他の 51 種、0041 には変更なし。
- ただし 0x20 と 0x1c は、基準は満たすが**弱い**（上の表）。0x20 は公式の例が割れており、0x1c は ICU/CLDR と値が違い、Windows の版で変わる疑いが残る。この 2 つは、実機で別の値と分かれば最初に直す対象。
- 0042 §3 の「abort のまま」の記述は、この決定で置き換える（0042 のファイル自体は変更していない）。

## 撤回の条件

- 実機の Windows 10 22H2（`fle_osver.c` の `fle_emulated_os`）の en-US で `GetLocaleInfoW` が別の値を返すと分かった LCType があれば、`fle_locale.c` の該当 1 行を直す（0042 のとおり）。0x20（`dd` か）、0x1c（`1` か）を優先して確認する。
- 空白の種類（U+0020 か U+202F か）が実機で違うと分かった場合も同様。

## 影響（ゲストの使い方。調査 §5 の読み）

- 時刻の書式（0x1003、0x79）: ゲストの `f_448490` が書式をパースする。`H`/`h`/`s` の連続は小文字で連続コピー、`m` は `n`、`t` 1 個は `a/p`、`t` 2 個以上は `AMPM`、`'` で囲んだ部分はそのまま、それ以外（空白を含む）はそのままコピー。したがって `h:mm:ss tt` は `h:nn:ss AMPM`、`h:mm tt` は `h:nn AMPM`。`tt` が必須（`a` だと AM/PM の印にならない）。空白が U+0020 か U+202F かは分岐に影響せず、そのままコピーされる。
- 日付の書式（0x1f、0x20）: en-US ではパースされず、`LCMapStringW(…, LCMAP_LOWERCASE | LCMAP_LINGUISTIC_CASING)` で小文字化されて `obj+0x10`、`obj+0x18` に保存されるだけ（`M` → `m`）。`yy`/`yyyy`、`d`/`dd` の違いは後段に残るが、後段の使用箇所は追っていない（未確認）。値は ASCII だけなので `LCMapStringW`（0x80 以上は stop）で止まらない。
- SLIST（0xc）: 先頭の 1 文字だけが効く。`,` はゲストの失敗時の既定と同じ。
- INEGCURR（0x1c）: 整数として 16 要素の表（`0` → `($*)`、`1` → `-$*` …）の添字になる。値が違うと、負の通貨の整形の文字列が変わる（この経路への到達は未確認）。

## 限界

- すべて仮。Win32 の LOCALE_* ページは en-US の既定値を書いておらず、公式の値のほとんどは .NET と PowerShell のページの静的な出力例。レジストリのダンプ 2 件は版と取得元が不明。Wine の 2 世代は同じプロジェクトのデータで、完全には独立でない。
- 実機 Windows で値の測定はしていない。
