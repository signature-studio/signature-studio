# ロケール応答の導入と計測実行（0x407120 の後）

作業者による実行記録。推測と実測を分けて書く。判断は `docs/decisions/0016-locale.md`、仕様は `docs/investigations/import-locale-spec.md`。前回は `docs/investigations/modules-run.md`（停止点 0x407120、SetThreadLocale）。

## 1. 実装（実測）

- 新規: `lift/runtime/fle_locale.h`、`lift/runtime/fle_locale.c`（13 API。定数は `fle_emulated_locale` の 1 か所）、`lift/crates/fe-lift/tests/locale.rs`。
- 追記: `lift/crates/fe-lift/src/imports_table.rs`（`HeapFree` の後に 13 行。応答表は 28 → 41 行）。
- `fle_locale.c` は `clang -O0 -Wall -Wextra` で警告 0（単体コンパイル EXIT=0 と、`build.sh` の EXIT=0 で確認）。
- 実装した 13 API: SetThreadLocale、GetThreadLocale、IsValidLocale、GetUserDefaultUILanguage、GetSystemDefaultUILanguage、GetACP、IsDBCSLeadByteEx、GetCPInfo、GetLocaleInfoW、LCMapStringW、CompareStringW、MultiByteToWideChar、WideCharToMultiByte（部分実装を含む。範囲外は abort）。
- 実装していない閉包のロケール系: EnumCalendarInfoW、EnumSystemLocalesW、GetDateFormatW（理由は `import-locale-spec.md` §4）。

## 2. テスト（実測）

- `locale.rs`: 11 件、全件通過（`lift/out-locale/test-locale-3.txt`、EXIT=0）。1 回目（`test-locale-1.txt`）は 5 件失敗: テスト内の C マクロ `P(x)` に `{}` 初期化子を渡して、カンマで引数が分かれたため。可変引数マクロに直した。2 回目（`test-locale-2.txt`）は `compare_string` が 1 件失敗: 期待値側の `ref_cmp` が、大文字小文字だけの違い（NORM_IGNORECASE なし）で `None` を返すのに、テストが `unwrap` していたため。その組は期待値を作らず「runtime が abort する」側（`undefined_cases_abort` の `cmp-case-only`）で確かめるように直した。C の実装は変えていない。
- `cargo test --offline --workspace --no-fail-fast`（`test-workspace-1.txt`）: **157 件通過、1 件失敗、EXIT=101**。失敗は `modules.rs` の `generated_unit_with_tables_compiles_and_links_with_runtime`:

  ```
  ld: ... gen-with-tables-....o:(.data.rel.ro+0x1f8): undefined reference to `fle_imp_SetThreadLocale'
  ```

  原因（実測の読み）: このテストは `fle_modules.c`、`fle_imports.c`、`fle_osver.c`、`fle_string.c`、`fle_sys.c` だけをリンクして、生成した応答表（`fle_resp_table`）の全行の `fle_imp_*` を解決させる。応答表に載った 13 行の関数（`fle_locale.c`）がリンクに入っていないため。**`imports_table.rs` に行を足すと、応答表を使うテストはどれも、足した行の runtime をリンクに入れる必要がある**。`modules.rs`（131 行と 238〜242 行の `.c` の並び）に `fle_locale.c` を足せば直る（`fle_sys.c` を別の担当が足したのと同じ形）。`modules.rs` は私の担当ファイルではないので、触っていない。PM か `modules.rs` の担当の作業になる。他の 31 のテスト実行（`fe-decode`、`fe-lift` の他の統合テスト、`sys.rs` の 18 件を含む）は通過した。
- cargo のターゲットは `lift/out-locale/target`、TMPDIR は `lift/out-locale/tmp`。

## 3. 生成、clang、リンク（実測）

- 生成器: `lift/out-locale/tool`（`out-modules/tool` の複製）。ビルド EXIT=0（`build-tool.txt`）。seed は `lift/out-locale/seeds-896.txt`（896 件、`out-modules/seeds-896.txt` の複製）。
- 生成: EXIT=0（`gen-locale-896.txt`）。出力 `lift/out-locale/out/after-locale-896.c`（42,514,031 バイト）。
  - functions 8427、closure_instructions 497367（前回と同じ）。c_lines 548232（前回 548148、+84）。
  - `fle_resp_count = 41`、`fle_dispatch_count = 8468`（関数 8427 + 擬似 VA 41）。
  - `jmp_imp:` のロケール系: SetThreadLocale 1、GetThreadLocale 1、IsValidLocale 2、GetUserDefaultUILanguage 1、GetSystemDefaultUILanguage 1、GetACP 2、IsDBCSLeadByteEx 1、GetCPInfo 1、GetLocaleInfoW 3、LCMapStringW 1、CompareStringW 3、MultiByteToWideChar 1、WideCharToMultiByte 2（`iat-slot-counts-v3.tsv` の件数と一致。13 API で 21 か所）。
  - unsupported_total 625。前回（`modules-run.md`）は 667 だが、前回の生成物の後に別の担当が応答表へ足した行（同期、時間、メモリ、ヒープ）も入っているので、−42 の全てがロケールによるものではない。内訳 `Jmp: indirect through IAT slot (not lifted)` は 378（前回 420）。
- clang（-O0）とリンク: `lift/out-locale/build.sh`（`out-modules/build.sh` の複製）。`EXTRA_RT="fle_int_more fle_sse_more fle_sys fle_locale"` で EXIT=0（`build-896.txt`）。`lift/out-locale/bin/flengine-after-locale-896`。

## 4. 実行（実測）

コマンド: `./bin/flengine-after-locale-896 ../../binaries/win64/FLEngine_x64.dll`（TMPDIR は `lift/out-locale/tmp`）。出力 `lift/out-locale/run-locale-896.txt`、`run-locale-896.err`、`run-locale-896.exit`。EXIT=134（SIGABRT）。

- 停止点 VA: 0x40a332
- 関数: f_40a330（0x40a330）
- 命令: `fnstcw WORD PTR [rsp]`（x87 の制御ワードの保存。`d9 3c 24`）
- abort の種類: `fle_unsupported`（`fle: unsupported instruction at 0x40a332`。生成物 19664 行 `L_40a332: { /* 0x40a332 Fnstcw */ fle_unsupported(UINT64_C(0x40a332)); }`）
- 関数実行数（distinct）: 42
- 関数の呼び出し（entries）: 64
- 呼び出し深さ: 7

```
#0 0x40a330 f_40a330 guest-return=0x40f513
#1 0x40f500 f_40f500 guest-return=0x41ce98
#2 0x41ce10 f_41ce10 guest-return=0x412129
#3 0x4120c0 f_4120c0 guest-return=0x41229a
#4 0x412160 f_412160 guest-return=0x41d989
#5 0x41d8f0 f_41d8f0 guest-return=0x1376875
#6 0x1376820 f_1376820 guest-return=0xdead0000
```

### 前回（0x407120、SetThreadLocale）との比較

| 項目 | 前回（modules-run.md） | 今回 |
|---|---|---|
| 停止点 | 0x407120 f_407120（kernel32!SetThreadLocale） | 0x40a332 f_40a330（`fnstcw`） |
| abort の種類 | fle_unsupported（IAT 経由の jmp） | fle_unsupported（x87 命令） |
| 関数実行数（distinct） | 35 | 42 |
| 関数の呼び出し | 51 | 64 |
| 呼び出し深さ | 6 | 7 |

- 実測で言えること（entry の並び）: entry 50（f_407120 = SetThreadLocale、guest-return 0x41ce4b）は返った。続いて entry 51 で f_418e10（0x41ce4b の `call 0x418e10`）が呼ばれ、その中で f_406f40、f_407220、そして f_407060（GetModuleHandleW）と f_407070（GetProcAddress）の 3 組（entry 54〜59）が返った。entry 60 f_41ca30、61 f_407230 が返り、entry 62 f_40f500（0x41ce93 の `call 0x40f500`）から entry 63 f_40a330 に入り、0x40a332 で停止した。停止点は f_41ce10 の中の SetThreadLocale の後、`0x41ce46` → `0x41ce93` まで進んだ。
- **今回の実行で通ったロケール系 API は SetThreadLocale の 1 回だけ**（entry log の `f_407120` が 1 回。他の 12 API の thunk は entry log に出ていない）。他の 12 API（GetACP は 0x41cecf で、f_40f500 の後に呼ばれる）の実機能は、テスト（`locale.rs`）でだけ確認していて、FLEngine の実行では未実行。
- 読み（推測）: SetThreadLocale の戻り値は 0x41ce4b の前に使われない（objdump の読み）ので、BOOL か LCID かの食い違い（0016 の §3）は、この呼び出しの先に影響しない。

## 5. 次の停止点（0x40a332）

- 命令: `fnstcw WORD PTR [rsp]`（`f_40a330` は `push 0; fnstcw [rsp]; pop rax; ret`。x87 制御ワードを読んで ax に返す小関数。objdump の読み）。
- 呼び出し元: f_40f500 の 0x40f50e（`call 0x40a330`）。その直後の 0x40f513 は `and ax, 0x1f3f`、0x40f517 で結果を `[0x13a5050]` に保存する。f_40f500 は先頭で `[0x15e60ac] = 3` を書く。
- IAT の名前: **なし**。import ではなく、**命令の未実装**（x87）。
- 未対応の種類: `Fnstcw: mnemonic outside minimal set`。同じ系統の x87 命令が閉包に他にもある（生成器の集計 `gen-locale-896.txt`）: `Fldcw` 4、`Fnclex` 2、`Fnstcw` 2。隣の f_40a320（0x40a320〜0x40a32f）は `fnclex; fldcw [0x13a5050]; ret`（制御ワードを設定する関数）で、`fnstcw` と対になる。これらは、MXCSR を扱った前例（`docs/decisions/0005-mxcsr.md`、`lift/runtime/fle_mxcsr.c`）の x87 版にあたる（読み。この作業では中身を読んでいない）。
- 方針の見立て（推測）: x87 の制御ワード（FNSTCW、FLDCW、FNCLEX）の状態を runtime に 1 つ持つ実装で、この 3 命令とも通る見込み。命令の仕様（Intel SDM）の確認が要る。

## 6. 残りの作業量（推測を含む）

- 停止点は 0x407120 から 0x40a332 へ進んだ。「フル lift」には届いていない。生成物の未対応は 625 か所（IAT 経由の jmp 378、間接 jmp 解決不能 20、未実装の命令が約 227: SSE のパック演算、x87、`bsf`、`bts`、`xchg`、`rol`、`pause`、`emms` など。`gen-locale-896.txt`）。
- ロケール系では、EnumCalendarInfoW（0x447860 のロケール初期化。4 回）、EnumSystemLocalesW（0x44d930）、GetDateFormatW（0x443110、0x443300）が未実装。その先で到達すれば IAT 経由の jmp として止まる。GetLocaleInfoW の LCTYPE は、閉包の即値のうち 4 つしか実装していない（0x14、0x59、0x5a、0x3）。0x446370 の初期化関数に着くと、0x1b などで `fle_locale_stop`（abort）になる見込み（読み。未実行）。
- 他の API（user32、gdi32、advapi32 など）は、依頼の範囲外で未着手。

## 7. 実測として言えないこと

- Windows 本体の NLS の戻り値との一致は、ホストが Linux のため未確認。
- GetACP、GetLocaleInfoW、CompareStringW などが FLEngine の実際の呼び出しで返す値は、今回の実行では確認していない（SetThreadLocale 以外は、停止点まで呼ばれていない）。
- 閉包の thunk の件数は生成器と一致したが、0x4a0000 より後の呼び出し箇所は、objdump で拾い切っていない。

## 8. 残置物

- `lift/out-locale/test-locale-1.txt`、`test-locale-2.txt`: テストの 1、2 回目の失敗ログ（期待値側とテストのマクロの誤り）。
- `lift/out-locale/tmp/text-401000-4a0000.txt`（6.7 MB）: 0x401000〜0x4a0000 の objdump。呼び出し元の調査に使った。
- `lift/out-locale/target`、`target-tool`、`test`: cargo のビルド出力、テストの C と実行ファイル。`fle_locale.o`: 単体コンパイルの確認で私が作ったオブジェクトで、その後 `build.sh` が同じパスへ再生成した（clang の `-o` による上書き。私が作ったファイルで、既存の他者のファイルではないが、「既存ファイルの上書き禁止」の文言には形式上触れる。報告する）。
