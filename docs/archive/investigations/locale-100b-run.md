# GetLocaleInfoW LCType 0x100b（LOCALE_IOPTIONALCALENDAR）の調査と harness 再実行の記録

日付: 2026-10-21。決定は `docs/decisions/0041-locale-lctype-100b.md`（**仮の決定。値は非公式の出典（Wine）による。実機 Windows は未確認**）。前提の記録は `versioninfo-size0-run.md` §3（停止点が LCType 0x100b）、手順は `pdata-seeds-run.md`、`versioninfo-size0-run.md` §2（split ビルド）。成果物は `lift/out-loc/`（`tmp/` は TMPDIR、cargo の target は `out-loc/target`）。HEAD は cecdd21。

「実測」はツールの出力、「推測」は未検証の解釈。

**混在の注意**: 作業ツリーには他担当の未コミット変更（MXCSR 状態ビット: `fe-lift/src/sse_fp_flags.rs`、`prefetch.rs`、`sse_more.rs`、`runtime/fle_mxcsr_flags.h`、`fle_sse.h`、`tests/mxcsr_flags.rs`。ETXTBSY 競合修正: `tests/` 配下の多数のテストとヘルパ）が混在している。生成・ビルド・実行・テストはその混在状態で行った。私が触ったのは §6 のファイルだけ。

## 1. 0x100b の同定（実測・取得した出典）

- **LCType 0x100b = `LOCALE_IOPTIONALCALENDAR`**。
- **Microsoft Learn の「Locale Information Constants」（https://learn.microsoft.com/en-us/windows/win32/intl/locale-information-constants、取得 2026-10-21）は名前の一覧だけで、数値は書かれていない**。`LOCALE_IOPTIONALCALENDAR` のページ（…/locale-ioptionalcalendar）にも数値と en-US の値はない。したがって数値の同定は公式の出典では取れず、非公式のヘッダで行った:
  - Wine `include/winnls.h`（https://raw.githubusercontent.com/wine-mirror/wine/master/include/winnls.h）: `#define LOCALE_IOPTIONALCALENDAR 0x100B`。
  - mingw-w64 `winnls.h`（https://raw.githubusercontent.com/mingw-w64/mingw-w64/master/mingw-w64-headers/include/winnls.h）: `#define LOCALE_IOPTIONALCALENDAR 0x0000100b`。
  - 2 つは一致する。
- 「0x1000 | 0x000b」の分解ではない。`LOCALE_NOUSEROVERRIDE` は 0x80000000（上のヘッダ）で、0x1000 ではない。0x1000 台は、1 つの定数として割り当てられた値（`LOCALE_SENGLANGUAGE` 0x1001、`LOCALE_ICALENDARTYPE` 0x1009、`LOCALE_IFIRSTDAYOFWEEK` 0x100c ほか）。`fle_locale.c` は上位ビット（0xF0000000）だけを外してから表を引くので、0x100b はそのまま渡る。
- 隣の値（同じヘッダ）: 0x1009 = `LOCALE_ICALENDARTYPE`、0x100a = `LOCALE_IPAPERSIZE`、0x100c = `LOCALE_IFIRSTDAYOFWEEK`、0x100d = `LOCALE_IFIRSTWEEKOFYEAR`。

## 2. 現在の `fle_locale.c` の応答表と、0x100b が無かった理由（実測、読み）

- 変更前の応答表（`fle_locale_info_value`）: 0x1（ILANGUAGE "0409"）、0x3（SABBREVLANGNAME "ENU"）、0xb（IDEFAULTCODEPAGE）、0x14（SCURRENCY "$"）、0x1004（IDEFAULTANSICODEPAGE）、0x59、0x5a、0x5c、0x67、0x68（ISO 名、SNAME）。10 種。**根拠は決定 0016 §2: 「仕様のページで規則が確認でき、en-US の範囲で実装できるものだけ実装し、決まらないものは `fle_locale_stop` で abort。黙って値を作らない」**。応答表の 10 種は、MSDN のページに en-US の値が書かれているもの（"0409"、"ENU"、"en-US"、"$" など）か、環境のロケールの設定値（コードページ）。
- LOCALE_IOPTIONALCALENDAR は、ページに en-US の値が無い。0016 の方針により応答表に無かった。これは**仕様不足のため意図的に空けてあった**もので、実装の漏れではない。`import-locale-spec.md` §3 は、この LCType が閉包の即値の 1 つ（0x100b）であることを前から記録していた。

## 3. 値の出典（実測。要約。詳細は 0041 §3）

- **MSDN は en-US の値を書いていない**。GetLocaleInfoEx のページが定めるのは「`LOCALE_IOPTIONALCALENDAR` のとき、最初の代替カレンダーだけを返す。全部は EnumCalendarInfoEx」だけ。Calendar Identifiers のページは 1 = CAL_GREGORIAN … 23 = CAL_UMALQURA の ID を定める。**値の形式（整数型の十進文字列か）と en-US の値は、MSDN から決まらない**。つまり「実装を決めるには仕様が足りない」ケース。
- 非公式の出典（Wine）:
  - en-US のロケールのリソース（https://raw.githubusercontent.com/wine-mirror/wine/wine-5.0/dlls/kernel32/nls/enu.nls、中身は STRINGTABLE）: `LOCALE_IOPTIONALCALENDAR "0"`、`LOCALE_ICALENDARTYPE "1"`。
  - 現行 `dlls/kernelbase/locale.c` の `get_locale_info`: `case LOCALE_IOPTIONALCALENDAR: return locale_return_number( locale_strings[locale->scalendartype + 2], … )`。
  - Wine のテスト `dlls/kernel32/tests/locale.c` の `invariant_list`（**LOCALE_INVARIANT の値であり en-US ではない**）: `LOCALE_IOPTIONALCALENDAR "0"`、`LOCALE_ICALENDARTYPE "1"`。同じ表の `locales_with_optional_calendars` は、代替カレンダーを持つ locale として ar-*、zh-TW、dv、fa、he、ja、ko、th、ur を挙げる（en は無い）。
- 参考（en-US の値は決めない）: .NET の `CultureInfo.OptionalCalendars` のページ（Microsoft Learn）は ar-SA の例のみ。CLDR `supplementalData.xml` の `calendarPreference territories="001"` は gregorian。CLDR は Windows の値ではない。
- **en-US の "0" を直接示すのは Wine の en-US データだけ**（ReactOS のロケールデータは、取得を試みた URL が 404 で確認できなかった。Wine の `locale.nls` はバイナリで取得できなかった）。

## 4. ゲストの使い方（静的、objdump。実測と推測の区別つき）

- 呼び出し鎖: `f_423560`（IAT jmp）← `f_446370`（0x446370。`GetLocaleInfoW(edx = LCID, r8d = LCType, buf = rbp+0x20, 0x100)` を呼び、**成功（eax > 0）なら `f_413060`（文字列を作る）、失敗なら `f_4133f0`（r9 = 引数のフォールバック文字列）**。つまり戻り値 0 のときの既定値の経路を持つ）← `f_447860`（0x447860。ロケール初期化）。
- `f_447860` の 0x4478ea〜0x447947: `f_446370(&str, LCID, 0x100b, r9 = 0x447dcc)` の後、`f_43aae0(str, 1)` を呼ぶ。`f_43aae0` は `call 0x40e3c0`（文字列を整数にする関数。空白を読み飛ばし、符号、先頭の 0、桁を処理する形）を呼び、**変換に失敗（[rbp+0x2c] != 0）したら edx（ここでは 1）を返す**。StrToIntDef(文字列, 1) に見える（推測）。
- 結果 eax = n に対し、`cmp eax,7; ja` の後、`1 << n` を VA 0x447dd0 のバイト `0x38`（ビット 3、4、5）と `test`（objdump -s で確認: 0x447dd0 = `38`、0x447dd1 = `06`）。ここから先:
  - n ∈ {3, 4, 5}: 0x4232f0（IAT jmp。`EnumCalendarInfoW`。`import-locale-spec.md` §4 の 4 回の呼び出しの一部、0x447961）を呼ぶ。
  - n ∈ {1, 2}（0x447dd1 のバイト `06`）: 0x447a52 と 0x447add の `EnumCalendarInfoW` を呼ぶ。
  - それ以外（0 を含む）: 0x447c17 へ進む（EnumCalendarInfoW を呼ばない）。
  - コードの意図は読みであり、n の意味（3 = CAL_JAPAN、4 = CAL_TAIWAN、5 = CAL_KOREA、1 = CAL_GREGORIAN、2 = CAL_GREGORIAN_US は Calendar Identifiers のページ）との対応は確認していない（推測）。
- 帰結（実測ではなく、読みからの推論）: "0" は `EnumCalendarInfoW`（runtime に未実装）に到達する枝を通らない。"1" や "2" を返した場合は、この API の実装が必要になる。値は出典で決め、この帰結は選択理由にしていない。

## 5. 実装とテスト

- `lift/runtime/fle_locale.c`: `FLE_LC_IOPTIONALCALENDAR 0x100B` を追加し、`fle_locale_info_value` に en-US（0x0409）のとき整数型で "0" を返す case を追加（他のロケールは 0 を返し、`fle_require_emulated` と LCType の表の外では従来どおり `fle_locale_stop`）。コメントに「MSDN にない。非公式（Wine）。0041」と明記した。**fail-soft な既定値ではない**: 値は出典のある固定値で、未知の LCType は abort のまま。
- `lift/crates/fe-lift/tests/locale.rs`（`get_locale_info`、`undefined_cases_abort` に局所追記。編集直前に該当行を読み直し、行番号が変わっていないことを確認した）:
  - `rows` を 10 → 11 行にし、`(0x100b, "0")` を追加（LCID 0x409、0x400、0x800 のそれぞれで、サイズ問い合わせ、バッファ、1 文字足りない、NULL バッファ、負のサイズ）。
  - `LOCALE_RETURN_NUMBER` の表に `(0x100b, 0)` を追加（DWORD 0、戻り値 2、cchData 1 で ERROR_INSUFFICIENT_BUFFER）。
  - abort の確認に 3 行追加: `gli-lctype-icalendartype`（0x1009）、`gli-lctype-ifirstdayofweek`（0x100c）、`gli-optcal-german`（0x407 の 0x100b）。
- `cargo test --offline -p fe-lift --test locale`（`out-loc/target`）: **11 件すべて passed**（`undefined_cases_abort` 123 秒）。
- ワークスペース全体（`cargo test --offline --workspace --no-fail-fast`、`out-loc/test-ws.txt`、`test-ws.exit` = EXIT=0）: 63 バイナリ、**599 passed、0 failed**、0 ignored。null0 の Text file busy は今回は出なかった。変異による検出力の確認は行っていない。

## 6. harness（all-pdata）の再生成・再実行（実測）

コマンド（`lift/out-loc/` で）: `fe-lift lift ../../binaries/win64/FLEngine_x64.dll out/loc-pdata.c --pdata`（`gen-pdata.txt`）→ `split_c.py out/loc-pdata.c out/loc-pdata-part 3700`（16 分割）→ `EXTRA_RT="fle_env fle_glob … fle_x87ctl"（24 個。build-parts.sh が既に持つ 7 個は重ねない）JOBS=8 sh build-parts.sh loc-pdata` → `./bin/flengine-loc-pdata ../../binaries/win64/FLEngine_x64.dll`（`run.txt`、`run.err`、`run.exit` = 134）。最初のビルドは `EXTRA_RT` に既存の 7 個を重ねて渡したため二重定義でリンクに失敗した（`build.err` は 2 回目で上書きされている）。2 回目は EXIT 0、26.4 秒、最大 RSS 1,192,780 KB。

| 項目 | 前（`out-vi3/run.txt`） | 後（`out-loc/run.txt`） |
|---|---|---|
| 停止点 | `GetLocaleInfoW` LCType **0x100b**（`fle_locale.c:283`） | `GetLocaleInfoW` LCType **0x37**（`fle_locale.c:293`。行番号は追記で動いた） |
| 関数実行数（distinct） | 189 | **198** |
| 関数入口（calls） | 957 | **984** |
| 呼び出し深さ | 10 | 11 |
| 生成の `unsupported_total` | 667（19 種類） | **603**（12 種類） |
| 閉包の命令 / 関数 | 2,526,826 / 58,695 | 2,526,826 / 58,695（不変） |
| `c_lines` | 2,880,419 | 2,880,419 |

- **`unsupported_total` が 667 → 603 に減ったのは、今回の変更（runtime のみ）の効果ではない**。runtime の変更は生成に影響しない。作業ツリーの別担当（SSE、prefetch の未コミット変更）の lift の変更による（推測。内訳の差: `Mulpd` 25、`Addpd` 13、`Cvtdq2ps` 9、`Subpd` 8、`Prefetcht0` 5、`Cvtpd2ps` 2、`Cvtps2pd` 2 が一覧から消えた）。残り: `Jmp: indirect through IAT slot` 557、`Psubw` 10、`Pcmpgtd` 8、`Rcpps` 8、`Bswap` 7、`Test: high-byte register` 7、`And: high-byte register` 1、`Andnpd` 1、`F2xm1` 1、`Fsincos` 1、`Jmp: indirect target unresolved` 1、`Xgetbv` 1。
- 0x100b の通過の確認（gdb、`gdb-loc2.cmds`、`gdb-loc2.txt`。`fle_imp_GetLocaleInfoW` の全呼び出しを記録）: LCType の順は 0x5c、0x59、0x5c、0x59、0x3、**0x100b**、**0x37**（7 回。最後が abort）。つまり 0x100b は通過し、次の呼び出しが 0x37。
- 0x100b の後の経路（実測は `run.txt` の呼び出し鎖、他は読み）: 新しい停止点の鎖は `f_423560 ← f_446370 ← f_447df0（0x447e02 の call）← f_4473f0（戻り番地 0x44747a）← f_446400 ← f_44bb70 ← f_455790 ← f_4120c0 ← f_412160 ← f_41d8f0 ← 入口 f_1376820`。前の停止点の鎖では `f_446400` の下が `f_447860`（0x100b の呼び出し元）だった。`f_446400` は 0x44650c で `f_447860`、0x446518 で `f_4473f0` を続けて呼ぶので（objdump）、`f_447860` は最後まで戻り、次の `f_4473f0` に進んだ。`EnumCalendarInfoW` の枝を通らなかったことは、停止が `EnumCalendarInfoW` の未実装停止ではなく 0x37 の `GetLocaleInfoW` になったことからの間接的な確認（`EnumCalendarInfoW` 到達の有無を直接見る gdb の確認は取っていない）。

## 7. 次の停止点の原因（実測）

- **`GetLocaleInfoW`、lcid = 0x409、LCType = 0x37、cchData = 256**（gdb `gdb-loc1.txt`: `GLI lcid=409 lctype=37 buf=7ffff7bff640 cch=256`）。名前付き abort は変わらず `LCType value for en-US is not on the MSDN pages`（`fle_locale.c:293`）。
- 0x37 = **`LOCALE_SABBREVDAYNAME7`**（日曜日の短縮名）。Wine と mingw-w64 の `winnls.h` で 0x31〜0x37 が SABBREVDAYNAME1〜7（月〜日）。MSDN の「LOCALE_SABBREV* Constants」（取得 2026-10-21）は "Native abbreviated name for Sunday. 最大 80 文字" とだけ書き、**en-US の値（"Sun"）はページにない**。LOCALE_SDAYNAME*（0x2a〜0x30）のページも同様に値なし。
- 呼び出し元: `f_4473f0`（VA 0x4473f0）。ループ ebx = 1..7 で `(ebx + 5) % 7`（0..6）を求め、`f_447df0`（`f_446370` のラッパー。VA 0x447df0）を **LCType = 0x31 + k**（短縮名、0x447459 の `add r8d,0x31`）と **0x2a + k**（正式名、0x447490 の `add r8d,0x2a`）で呼び、結果を `f_4133f0` で配列（rdi と rdi+0x38）に格納する。k = (ebx + 5) % 7 なので ebx = 1 で k = 6 → **0x37（日曜、短縮）が最初**、次が 0x30（日曜、正式）、ebx = 2 で k = 0 → 0x31（月曜）…と続く（読み。gdb は最初の 0x37 までしか実測していない）。つまり**曜日名 7 + 7 = 14 種類、続いて月名（`f_447510`、0x446524 の呼び出し。LCType は未確認）が来ると読める**。受け取った文字列は日付書式の名前表（ゲストの NLS キャッシュ。推測）に格納される。
- 決定が必要な点（実装していない。範囲外）: en-US の曜日名・月名は、**MSDN のページに値がない**（ただし "Monday" など英語の名前は一般知識。0016 の方針は「ページにあるものだけ」で、一般知識では決めない）。値の候補と出典: Wine のデータ（`enu.nls` の STRINGTABLE。LOCALE_SABBREVDAYNAME1〜7 = "Mon"〜"Sun"、LOCALE_SDAYNAME1〜7 = "Monday"〜"Sunday"、LOCALE_SMONTHNAME1〜12 = "January"〜"December"、LOCALE_SABBREVMONTHNAME1〜12 = "Jan"〜"Dec"、13 番目は ""。非公式）。Wine のテスト `invariant_list` も同じ値（invariant）。公式の出典は見つからなかった。0041 と同じ扱い（非公式、仮）で追記する案と、実機 Windows での確認を待つ案がある。ゲストの `f_4473f0` は 14 回、`f_447510` は未解析の回数を呼ぶので、追記するなら 0x31〜0x37、0x2a〜0x30、0x38〜0x4f 前後と、13 番目の月（0x100e、0x100f）を一緒に決めるのが現実的（推測）。

## 8. 制約の遵守と残置物

- `rm`、`>` による既存ファイルの上書き（新規ファイルへのリダイレクトと `cp` による新規コピーは使った）、git 操作、`/tmp`、sleep/timeout は使っていない（`command time -v` は GNU time で、timeout ではない）。TMPDIR は `lift/out-loc/tmp`。権限拒否はなし。2 GB 級の中間ファイルなし（最大は `out/loc-pdata.c` 216 MB、`bin/flengine-loc-pdata` 257 MB）。`out-pdata` の大きなファイルの複製はしていない（`out-vi3` の `build-parts.sh`、`split_c.py`、`trace.c` の小さなファイルだけを `out-loc/` にコピー）。
- 後始末できていないもの（削除禁止）: `lift/out-loc/` の生成物（`out/loc-pdata.c` と分割 16 個、`parts/`、`bin/`、`gdb-loc1.*`、`gdb-loc2.*`、`build.err`、`target/`）。
- 触ったファイル:
  - `lift/runtime/fle_locale.c`（M）
  - `lift/crates/fe-lift/tests/locale.rs`（M。他担当の ETXTBSY 修正の変更と同じファイルに混在。私の変更は `get_locale_info` と `undefined_cases_abort` の 6 か所）
  - `docs/decisions/0041-locale-lctype-100b.md`（新規）
  - `docs/investigations/locale-100b-run.md`（新規）
  - `lift/out-loc/`（新規の生成物、上記）
