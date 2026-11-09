# GetLocaleInfoW の LCType の静的な網羅と、en-US の値の一括決定、harness の再実行の記録

日付: 2026-10-21。決定は `docs/decisions/0042-locale-enus-values.md`（**仮の決定。値は非公式の出典（Wine、CLDR）による。実機 Windows は未確認**）。前提: `locale-100b-run.md`（停止点が LCType 0x37）、手順は `locale-100b-run.md` §6、`versioninfo-size0-run.md` §2（split ビルド）。成果物は `lift/out-loc2/`（`tmp/` は TMPDIR。cargo の target は前任の `lift/out-loc/target` を共用。新規に作ると約 1.8GB になるため）。HEAD は 46ff672。

「実測」はツールの出力、「推測」は未検証の解釈。

**混在の注意**: 作業ツリーには他担当の変更（x87 の F2XM1/FSINCOS: `fe-lift/src/x87.rs`、`runtime/fle_x87.c`、`fle_x87.h`、`tests/x87_trans*`。MXCSR: `runtime/fle_mxcsr_flags.h`。ETXTBSY 修正: `tests/` の多数）が混在し、**私の作業中にも書き換えられた**（§6）。私が触ったのは §8 のファイルだけ。

## 1. 現在の runtime が答える LCType と abort する LCType（実測、読み）

- 作業前（0041 の後）に `fle_locale_info_value` が答えるもの: 0x1（ILANGUAGE）、0x3（SABBREVLANGNAME）、0xb（IDEFAULTCODEPAGE）、0x14（SCURRENCY）、0x1004（IDEFAULTANSICODEPAGE）、0x100b（IOPTIONALCALENDAR。0041。非公式）、0x59、0x5a、0x5c、0x67、0x68（ISO 名、SNAME）。11 種。それ以外の LCType は `fle_locale_stop`（abort）。
- 他に abort する条件（変更なし）: en-US 以外のロケール（`fle_require_emulated`。0x400、0x800 は en-US に解決）、`LOCALE_RETURN_GENITIVE_NAMES`、`LOCALE_RETURN_NUMBER` と文字列型の組、`LOCALE_RETURN_NUMBER` と cchData 0。
- 決定 0016 §2: 「仕様のページで規則が確認でき、en-US の範囲で実装できるものだけ実装し、決まらないものは abort。黙って値を作らない」。0041: 非公式の出典（Wine）の 1 件を仮に採用。今回の 0042 は、**出典 2 つ以上の一致**を基準にした（0007）。

## 2. 閉包の `GetLocaleInfoW` の呼び出し箇所と LCType の静的な列挙（実測: objdump、読み）

方法: インポートスロット 0x19716f8（`GetLocaleInfoW`）への thunk は 3 つ（`objdump-text.txt` の `jmp [rip+…] # 0x19716f8`）: 0x4070f0、0x423560、0x423570。呼び出し元を全部拾った（`objdump-text.txt` は .text の 0x11fb4af までなので、残り 0x11fb4a0〜0x13a44b0 を `objdump -d --start-address=…` で `out-loc2/tail-text.txt` に作って確認した。thunk、ラッパーへの呼び出しは 0 件）。ラッパーと thunk への `call` 以外の参照（`jmp`、ポインタ、vtable）は、objdump の `jmp`/`call` 参照 0 件、ファイル内の 32/64 ビット絶対値 0 件で確認した（読み。間接 `call [reg]` で呼ぶ可能性は完全には排除していない）。

直接の thunk の呼び出し（13 箇所）とラッパー:

| 呼び出し側 | 内容 | LCType の渡し方 |
|---|---|---|
| 0x41927a、0x419290（f_419150） | `edx = 0x59`、`0x5a`（直前の `mov edx, imm`） | 即値 |
| 0x41a0c5（f_41a080） | `edx = 3` | 即値 |
| 0x446392（`f_446370`、ラッパーA） | `r8d`（引数）を `edx` に移して呼ぶ。`GetLocaleInfoW(LCID = edx, LCType = r8d, buf, 0x100)` | 引数（14 箇所の呼び出し側） |
| 0x4463e6（`f_4463d0`、ラッパーB） | `edx`（引数）をそのまま LCType として呼ぶ。`r9d = 3`（バッファ 3 文字） | 引数（5 箇所） |
| 0x44d614（`f_44d5f0`、ラッパーC） | `r8d`（引数）を `edx` に移して呼ぶ。`r9d = 0x400`（バッファ 0x400 文字） | 引数（4 箇所） |
| 0x70f57f（f_70f550）、0xa16486（f_a16460） | `edx = 0x1004` | 即値 |
| 0x81c76a、0x81c794（f_81c740） | `edx` が引数。呼び出し元は 0x11a803c の 1 箇所で `edx = 1` | 引数から即値 `1` |
| 0x8da4d1、0x8da4f4（f_8da480） | `edx = 0x59`、`0x5a`（即値）。`ecx = 0x400` | 即値 |
| 0x98eb01（f_98eaa0） | `edx = [rbp+0x2c]`（引数 r8d） | **動的**（§3） |

ラッパーAの 14 箇所の呼び出し側（`f_446370`）:

| 呼び出し箇所 | LCType（`r8d`） | 備考 |
|---|---|---|
| 0x44653e | 0x14 | |
| 0x446569 | 0x1b | |
| 0x446599 | 0x1c | |
| 0x446604 | 0x19 | |
| 0x44670a | 0x28 | |
| 0x446733 | 0x29 | |
| 0x4467f9 | 0x25 | |
| 0x446848 | 0x23 | |
| 0x44686f | 0x1005 | |
| 0x447901（f_447860） | 0x100b | 前任が処理済み |
| 0x447fa4（f_447f40） | 0x1009 | f_447f40 は 0x447e85 の 1 箇所から。結果は `f_43aae0`（文字列を整数に。既定 1）にかけ、`cmp eax,7; ja` の後、`1 << eax` を VA 0x44829c のバイト（`objdump -s` で 0x38 = ビット 3、4、5）と `test` し、真なら 0x44822a へ（読み） |
| 0x447e02（f_447df0、ラッパーD） | `r8d` 引数。4 箇所の呼び出し側（f_4473f0 の 0x447475、0x4474ac と f_447510 の 0x44756b、0x4475a2） | 下の「ループ」 |
| 0x447e5f（f_447e30、ラッパーE） | `r8d` 引数。2 箇所の呼び出し側（0x44665a、0x4466b2）。`r8d = 0x1f`、`0x20` | |
| 0x44837e（f_448350、ラッパーF） | `r8d` 引数。2 箇所の呼び出し側（0x446767、0x44679b）。`r8d = 0x1003`、`0x79` | |

ラッパーBの 5 箇所（`edx` に即値）: 0x4465c2（`edx = 0xf`）、0x4465de（0xe）、0x44662a（0x1d）、0x4466ea（0x1e）、0x44693e（0xc）。ラッパーCの 4 箇所（`r8d` に即値）: 0x44d6ff（3）、0x44d761（2）、0x44d80c（0x59）、0x44d829（0x5a）。

ループ（読み。`add r8d, imm`）:
- `f_4473f0`（0x4473f0）: ebx = 1..7、`k = (ebx + 5) % 7` で **LCType = 0x31 + k**（0x447459。短縮名 7 個）と **0x2a + k**（0x447490。正式名 7 個）。`k` は 0..6 の全部を通る。
- `f_447510`（0x447510）: ebx = 1..12（`cmp ebx, 0xd; jne` で 13 で止まる）で **LCType = 0x43 + ebx**（0x44754f の `add r8d, 0x43`。短縮名 12 個: 0x44〜0x4f）と **0x37 + ebx**（0x447586。月名 12 個: 0x38〜0x43）。13 番目の月（0x100e、0x100f）は要求しない。

静的に決まる LCType の集合（**64 種**。読み）:

| 範囲 | LCType | LOCALE_ 名 | 呼び出し箇所 | ゲストの使い方 |
|---|---|---|---|---|
| 1 | 0x1 | ILANGUAGE | f_81c740（0x81c76a、0x81c794。引数 `edx = 1` を 0x11a803c から） | サイズ問い合わせ（バッファ 0）の戻り値で文字列の長さを得る。呼び出し元の用途は未確認 |
| 2 | 0x2 | SLANGUAGE | 0x44d761（f_44d5f0 経由）。ロケール名に似た文字列を作る関数（f_44d5f0 の呼び出し元 0x44d6xx〜0x44d8xx。EnumSystemLocalesW のコールバック付近。推測） | 文字列を作って保持 |
| 3 | 0x3 | SABBREVLANGNAME | 0x41a0b6、0x4463d0（ラッパーB の `r9d = 3` はバッファ長）、0x44d6f9 | 文字列、先頭 4 文字 |
| 4 | 0x59、0x5a | SISO639LANGNAME、SISO3166CTRYNAME | 0x419268、0x419281、0x8da4c2、0x8da4e5、0x44d80c、0x44d829 | 文字列。0x8da4c2 は戻り値 3 と比較（`cmp eax,3`。"en" + NUL なら 3 になる） |
| 5 | 0xc | SLIST | 0x44693e（ラッパーB。既定の文字は `r8w = 0x2c`、`,`） | **1 文字だけ（バッファ 3 文字）取り、失敗時は既定 `,`** |
| 6 | 0xe、0xf | SDECIMAL、STHOUSAND | 0x4465de（既定 `.`）、0x4465c2（既定 `,`） | 1 文字（失敗時は既定。f_446400 のオブジェクトの +0x17a、+0x178 に格納） |
| 7 | 0x14 | SCURRENCY | 0x44653e | 文字列。オブジェクトの +0x38 前後の文字列を `f_4133f0` で保持 |
| 8 | 0x19、0x1b、0x1c | ICURRDIGITS、ICURRENCY、INEGCURR | 0x446604、0x446569、0x446599 | `f_43aae0`（既定 0。文字列を整数に）にかけ、バイトで保持（オブジェクトの +0x9 など） |
| 9 | 0x1d、0x1e | SDATE、STIME | 0x44662a（既定 `/`、`r8w = 0x2f`）、0x4466ea（既定 `:`、`r8w = 0x3a`） | 1 文字 |
| 10 | 0x1f、0x20 | SSHORTDATE、SLONGDATE | 0x44665a、0x4466b2（ラッパーE。既定は `/` 区切りの文字列を作る f_450540 の前処理） | **日付書式の文字列。ゲストが `/` と `y/M/d` などを読み替える（f_447e30 → f_447f40）** |
| 11 | 0x23、0x25 | ITIME、ITLZERO | 0x446848、0x4467f9 | 整数（0 か否かで分岐。0x446858 の `test eax,eax`） |
| 12 | 0x28、0x29 | S1159、S2359 | 0x44670a、0x446733 | 文字列（オブジェクトの +0x20、+0x28） |
| 13 | 0x2a〜0x30、0x31〜0x37 | SDAYNAME1〜7、SABBREVDAYNAME1〜7 | f_4473f0（14 回） | 文字列。配列（rdi + 0x38、rdi + 0x100）に格納 |
| 14 | 0x38〜0x43、0x44〜0x4f | SMONTHNAME1〜12、SABBREVMONTHNAME1〜12 | f_447510（24 回） | 文字列。配列（rsi + 0x60、+0x40）に格納 |
| 15 | 0x79 | SSHORTTIME | 0x44679b（ラッパーF） | **時刻書式の文字列。`f_448490` が書式の解析をする（引用符 `'`、`0xff` 以下の文字の判定）** |
| 16 | 0x1003 | STIMEFORMAT | 0x446767（ラッパーF） | 同上 |
| 17 | 0x1004 | IDEFAULTANSICODEPAGE | 0x70f57f、0xa16486 | 文字列（バッファ 7、6 文字）。コードページとして使う（用途は未確認） |
| 18 | 0x1005 | ITIMEMARKPOSN | 0x44686f | 整数（0 か否か） |
| 19 | 0x1009 | ICALENDARTYPE | 0x447fa4（f_447f40） | `f_43aae0`（既定 1）にかけ、`cmp eax,7; ja` の後、ビットマスク 0x38（{3, 4, 5}）と `test`。{3, 4, 5} か否かで分岐（読み。0x100b の判定（0041 §4）と同じ形） |
| 20 | 0x100b | IOPTIONALCALENDAR | 0x4478f4（0041 で処理済み） | `EnumCalendarInfoW`（未実装）に到達する枝の判定（0041 §4）。"0" では到達しない |

（個数: 1 + 1 + 1 + 2 + 1 + 2 + 1 + 3 + 2 + 2 + 2 + 2 + 14 + 24 + 1 + 1 + 1 + 1 + 1 + 1 = 64。上の表の「範囲」の列は便宜上の通し番号で、行数は 20。）

## 3. 静的に決まらない呼び出し（別掲）

- `f_98eaa0`（0x98eaa0）の 0x98eb01: `edx = r8d`（引数 `[rbp+0x2c]`）。呼び出し元は 0x98ec90 の 1 箇所（`f_98eb50`）で、その `r8d` は `[rbp+0x70]`。`f_98eb50` を呼ぶのは 0x98f03f の 1 箇所（`f_98f030`。`or r8d, 0xffffffff` = -1）。`f_98eb50` は `[rbp+0x70] == -1` のとき 0x98ec95 へ飛び、`f_98eaa0` を呼ばない（0x98ec7f〜0x98ec83）。したがって、**この呼び出しに実行時の LCType が渡るのは、`f_98eb50` を `r8d != -1` で呼ぶ経路があるときだけ**。そういう呼び出し元は、objdump の `call` からは見つからなかった（`f_98f030` の呼び出し元は 0x9c5917 の 1 箇所。間接呼び出しは調べていない）。この呼び出しの LCType は静的に決まらず、64 種には数えていない（到達する呼び出し元も見つからなかった。間接呼び出しは調べていない）。LCID も固定ではない（`f_98eb50` で 0x409、グローバル 0x15f97ec の値、または引数 `& 0xff` から。読み）。バッファは `r9d = 0x3ff`。
- 他に引数で渡る LCType（`f_447df0`、`f_447e30`、`f_448350` の `r8d`、`f_44d5f0` の `r8d`）は、上の表のとおり呼び出し側の即値、ループの添字で全部決まった。`f_81c740` の `edx` も 1 箇所の呼び出し元の即値（1）。

## 4. 閉包がゲストから他の NLS API で LCType を要求し得るか（実測）

- インポート表（`objdump -p`）の NLS 系: `GetACP`、`GetCPInfo`、`GetCPInfoExW`、`EnumSystemLocalesW`、`GetUserDefaultLCID`、`CompareStringW`、`LCMapStringW`、`GetUserDefaultUILanguage`、`CompareStringA`、`GetLocaleInfoW`、`GetOEMCP`、`GetDateFormatW`、`IsValidLocale`、`GetSystemDefaultUILanguage`、`EnumCalendarInfoW`、`GetThreadLocale`、`SetThreadLocale`。
- `GetLocaleInfoEx`、`GetCalendarInfo*`、`GetTimeFormat*`、`GetNumberFormat*`、`GetCurrencyFormat*`、`GetUserDefaultLocaleName`、`LCIDToLocaleName`、`GetSystemDefaultLCID`、`GetLocaleInfoA` は、インポート表にも DLL 内の文字列（ASCII、UTF-16）にも無い。
- LCType を渡すのは `GetLocaleInfoW` だけ。`EnumCalendarInfoW` は CalType（0x447961、0x447a01、0x447a52、0x447add。CalType は 4 か 3）を渡す。`GetDateFormatW`（0x4431bd、0x4433c2）は `edx = 4`（dwFlags。値 4 は Wine の `winnls.h` で `DATE_USE_ALT_CALENDAR`）と書式文字列を渡す。書式文字列の中身（0x4432f4、0x443468、0x44347c 付近。`y` が見える）は読み切っていない。**この 2 つの API はこの作業の範囲外**（§5）。

## 5. 範囲外（実装していない）で、次に必要になりうるもの

- `GetDateFormatW`（f_443110、0x4431bd と f_443390 付近、0x4433c2）: 0x443110 の関数は `GetThreadLocale()`（0x443188）の結果を LCID に、`dwFlags = 4`、書式文字列（`y` を含む）、`SYSTEMTIME`（呼び出し側から）、バッファ 0x200 文字で呼ぶ。`fle_unsupported` で止まる（thunk 0x423480 は応答表にない）。到達するかは未確認。
- `EnumCalendarInfoW`: 0x100b が "0" のとき、`f_447860` の判定（0041 §4）で到達しない（呼び出し 4 回のうち 0x447961 などが {3, 4, 5}、{1, 2} の枝）。0x1009 の判定（f_447f40 の 0x447fb7〜0x447fd8）は {3, 4, 5} なら 0x44822a へ進む（読み）。**"1" は {3, 4, 5} に入らないので、この枝には入らず、0x447fde 以降（グローバル 0x15ec3a4 によるもう一つの判定と、0x448005 からの書式文字列の処理）に進む**。その先は未解析。値を "1" と決めたこと（0042）の帰結はここまでしか確認していない。

## 6. 値の決定とテスト（実測）

- 決定は 0042。51 種を実装（`fle_locale.c` に `FLE_LC_*` と `switch` の分岐）。6 種（0xc、0x1c、0x1f、0x20、0x1003、0x79）は出典が食い違うので abort のまま。
- 出典の突合（詳細は 0042 §1、§3）: Wine 5.0 `enu.nls`（ReactOS の `enu.nls` と**バイト単位で同一**。`cmp`）、Wine 現行 `locale.nls`（`parse_wine_nls.py` で LCID 0x409 を取り出した。`wine-locale-nls-en-US.txt`）、CLDR en（`en.xml`、`supplementalData.xml`）、MSDN の各定数ページ（意味と表。en-US の値はほぼ無い）。
- `lift/crates/fe-lift/tests/locale.rs`（編集直前に該当箇所を読み直し、追記のみ。前任の 0x100b の変更と他担当の `mod etxt_common;` はそのまま）:
  - `get_locale_info_enus_0042`（新規）: 51 行 × LCID 3 通り（0x409、0x400、0x800）で、サイズ問い合わせ、バッファ、1 文字足りない、NULL バッファ、負のサイズ。`LOCALE_NOUSEROVERRIDE`。`LOCALE_RETURN_NUMBER` の整数型 6 種（0x19、0x1b、0x23、0x25、0x1005、0x1009）。
  - `undefined_cases_abort`: 0x1009 の行（ICALENDARTYPE が答え済みになったので）を削除し、`gli-0042-*` を 16 行追加（食い違い 6、答えの範囲の外 3: 0x100e、0x50、0x100f、他ロケール 5、`LOCALE_RETURN_NUMBER` と文字列型 2）。既存の `gli-lctype-ifirstdayofweek`、`gli-optcal-german` は残した。
- `cargo test --offline -p fe-lift --test locale`（`CARGO_TARGET_DIR=out-loc/target`。`out-loc2/test-locale.txt`）: **12 件すべて passed**（`undefined_cases_abort` 約 60 秒）。
- ワークスペース全体（`cargo test --offline --workspace --no-fail-fast`）を 3 回実行した（`test-ws.txt`、`test-ws2.txt`、`test-ws3.txt`。最終は `test-ws-final.txt` = `test-ws3.txt` のコピー。`test-ws.summary.txt` に集計）:
  1. `test-ws.txt`（EXIT 101）: 63 バイナリ、598 passed、2 failed（`x87_forms_not_implemented_stay_unsupported`、`x87_arithmetic_stays_unsupported`）。原因: 別担当が実行中に `x87.rs` と `fle_x87.c` に F2XM1/FSINCOS を入れ、同時に更新した `tests/x87.rs`、`x87ctl.rs` が（私の実行より後の 02:43 に）書き換えられた。私の変更とは無関係。`cargo test -p fe-lift --test x87 --test x87ctl` の再実行（`test-x87-recheck.txt`）は passed（14 + 7）。
  2. `test-ws2.txt`（EXIT 101）: 64 バイナリ、541 passed、66 failed。原因: `fle_mxcsr_flags.h:975` の `fle_mx_addss` の再定義（clang エラー）。別担当が実行中（03:21:19）にこのヘッダを書き換えていた。私は触っていない。
  3. `test-ws3.txt`（EXIT 0）: **64 バイナリ、610 passed、0 failed**、0 ignored。`Text file busy` は 0 件。
- 変異による検出力の確認は行っていない。

## 7. harness（all-pdata）の再生成・再実行（実測）

コマンド（`lift/out-loc2/` で）: `../out-loc/target/release/fe-lift lift ../../binaries/win64/FLEngine_x64.dll out/loc2-pdata.c --pdata`（`gen-pdata.txt`、`gen-pdata.err`）→ `python3 split_c.py out/loc2-pdata.c out/loc2-pdata-part 3700`（16 分割）→ `EXTRA_RT="…"（24 個。runtime の全 `fle_*.c` から `build-parts.sh` が既に持つ 7 個（fle_string、fle_imports、fle_mxcsr、fle_cpuid、fle_int_ext、fle_osver、fle_modules）を除いたもの。`extra-rt.txt`）JOBS=8 sh build-parts.sh loc2-pdata`（`build.txt`、`build.err`。EXIT 0、29.4 秒、最大 RSS 1,205,380 KB）→ `./bin/flengine-loc2-pdata ../../binaries/win64/FLEngine_x64.dll`（`run.txt`、`run.err`、`run.exit` = 134）。二重定義は出なかった。

| 項目 | 前（`out-loc/run.txt`） | 後（`out-loc2/run.txt`） |
|---|---|---|
| 停止点 | `GetLocaleInfoW` LCType **0x37**（SABBREVDAYNAME7） | `GetLocaleInfoW` LCType **0x1c**（INEGCURR） |
| 関数実行数（distinct） | 198 | **199** |
| 関数入口（calls） | 984 | **1386** |
| 呼び出し深さ | 11 | 9 |
| 生成の `unsupported_total` | 603（12 種類） | **601**（10 種類） |
| 閉包の命令 / 関数 | 2,526,826 / 58,695 | 2,526,826 / 58,695（不変） |
| `c_lines` | 2,880,419 | 2,880,419 |

- **`unsupported_total` が 603 → 601 に減ったのは、今回の変更（runtime のみ）の効果ではない**。runtime の変更は生成に影響しない。一覧から `F2xm1` 1、`Fsincos` 1 が消えた（他担当の x87.rs の変更。決定 0043。推測ではなく、`x87.rs` の diff で `F2xm1`、`Fsincos` の行が追加されている）。残り: `Jmp: indirect through IAT slot` 557、`Psubw` 10、`Pcmpgtd` 8、`Rcpps` 8、`Bswap` 7、`Test: high-byte register` 7、`And: high-byte register` 1、`Andnpd` 1、`Jmp: indirect target unresolved` 1、`Xgetbv` 1。
- gdb（`gdb-loc2.cmds`、`gdb-loc2.txt`。`fle_imp_GetLocaleInfoW` の全呼び出しを記録）の LCType の順（lcid は全部 0x409）: 0x5c、0x59、0x5c、0x59、0x3（ここまでは前回と同じ）、0x100b、**0x37、0x30、0x31、0x2a、0x32、0x2b、0x33、0x2c、0x34、0x2d、0x35、0x2e、0x36、0x2f**（曜日 14 個。読みどおり、k = (ebx+5)%7 の順、各ペアは短縮名、正式名）、**0x44、0x38、0x45、0x39、…、0x4f、0x43**（月 24 個。短縮名、正式名のペア）、**0x14、0x1b、0x1c**（ここで abort）。合計 47 回（abort の呼び出しを含む）。**曜日 14 個と月 24 個が全部通った**。
- 停止点の呼び出し鎖（`run.txt`）: `f_423560 ← f_446370（戻り番地 0x44659e = 0x446599 の呼び出しの直後）← f_446400（0x44bb9f）← f_44bb70 ← f_455790 ← f_4120c0 ← f_412160 ← f_41d8f0 ← 入口 f_1376820`。つまり前回の停止点と同じ鎖の上位（`f_446400`）で、`f_4473f0`、`f_447510` を過ぎて、`f_446400` の続きに進んだ。

## 8. 次の停止点の原因（実測）

- **`GetLocaleInfoW`、lcid = 0x409、LCType = 0x1c = `LOCALE_INEGCURR`（負の通貨の書式）、cchData = 256**。呼び出し元は `f_446400` の 0x446599（`f_446370` 経由。`r8d = 0x1c`、`r9 = 0x4469d0`。0x4469d0 は文字列 "0"（`01 00 00 00 30 00`、長さ 1）で、失敗時の既定値と読める）。結果は `f_43aae0`（0x4465a7。既定値 0 で文字列を整数に）にかけ、バイトでオブジェクトの +0x17e に格納（読み）。
- **値が決まらない理由**（0042 §3）: Wine 5.0 と現行は "0"（"($1.1)"）。CLDR の通貨書式は標準が `-¤#,##0.00`（モード 1）で、会計が `(¤#,##0.00)`（モード 0）。どちらが Windows の既定か、CLDR は言えない。**実機の新しい Windows の en-US は "1" だという記憶があるが、確認していない（推測）**。出典 2 つ以上の一致が取れないので実装していない。
- この後（`f_446400` の読み）に続く順: 0xf、0xe、0x19、0x1d、**0x1f（SSHORTDATE。食い違い）**、0x20、0x1e、0x28、0x29、0x1003、0x79、0x25、0x23、0x1005、0xc。つまり**食い違いの 6 種のうち 0x1c、0x1f が次々に止まる**見込み（0x1f、0x20、0x1003、0x79、0xc も同じ関数の中。読み）。
- 実機 Windows の値、または Microsoft が公表した en-US の値が必要になるのは、この 6 種（0xc、0x1c、0x1f、0x20、0x1003、0x79）。決めるまで harness は 0x1c で止まる。

## 9. 制約の遵守と残置物

- `rm`、`>` による既存ファイルの上書き（新規ファイルへのリダイレクトと `cp` による新規コピーは使った）、git 操作、`/tmp`、sleep/timeout は使っていない（`/usr/bin/time -v` は GNU time で、timeout ではない）。TMPDIR は `lift/out-loc2/tmp`。権限拒否はなし。2 GB 級の中間ファイルなし（最大は `bin/flengine-loc2-pdata` 257 MB、`out/loc2-pdata.c` 216 MB。`out-pdata` の大きなファイルの複製はしていない。`out-loc` の `build-parts.sh`、`split_c.py`、`trace.c` の小さなファイルだけを `out-loc2/` にコピー）。cargo の target は `out-loc/target` を共用した（新規に作らず、前任の成果物に書き足した）。
- 後始末できていないもの（削除禁止）: `lift/out-loc2/` の生成物（`out/` の 17 ファイル約 430MB、`parts/`、`bin/`、`obj/`、`gdb-loc2.*`、`src/`（取得した出典）、`tail-text.txt`（16MB）、`test-ws*.txt`）。
- 触ったファイル:
  - `lift/runtime/fle_locale.c`（M。前任の 0x100b の変更に追記）
  - `lift/crates/fe-lift/tests/locale.rs`（M。他担当の ETXTBSY 修正の変更と同じファイルに混在。私の変更は `get_locale_info_enus_0042` の追加と `undefined_cases_abort` の 1 か所）
  - `docs/decisions/0042-locale-enus-values.md`（新規）
  - `docs/investigations/locale-enus-run.md`（新規）
  - `lift/out-loc2/`（新規の生成物、上記）
