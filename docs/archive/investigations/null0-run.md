# 宛先 0 の間接 call（0x4193c4）の特定と GetThreadPreferredUILanguages の応答

作業者による実行記録。推測と実測を分けて書く。判断は `docs/decisions/0022-thread-preferred-ui-languages.md`。前回の記録は `docs/investigations/uilang-run.md`（停止点 0x4193c4、fle_dispatch_miss、関数の呼び出し 150、深さ 15）。

## 1. 呼び出し点（実測 + objdump の読み）

- 停止の表示: `fle: indirect call to 0 is not a known function entry`（`out-uilang/run-uilang-896.txt`）。スタック最上位は `#0 0x4193a0 f_4193a0 guest-return=0x41949c`、`#1 0x419400 f_419400`。
- 呼び出し命令: **0x4193c4**（f_4193a0 内）`call QWORD PTR [rip+0x11d1b16]  # 0x15eaee0`。同じ変数を 0x4193ec（2 回目の呼び出し）も読む。
- 宛先 0 の変数: **`[0x15eaee0]`**（.data の `[rip+X]` 形式のグローバル）。引数（0x4193b5〜0x4193c1）: ecx = 0x38、rdx = rbp+0x2c、r8 = 0、r9 = rbx。
- 0x4193c4 の後 `test al,al; je`、真なら `ecx = [rbx]*2` で 0x409410 を呼び（確保）、同じ関数を 2 回目に呼ぶ。サイズ問い合わせ + 取得の 2 段階の形（objdump の読み）。

## 2. 書き込み元（実測）

- 書き込みは 1 か所: `0x418e63: mov [rip+...], rax`（0x418e10 の関数内）。直前の 0x418e5e は GetProcAddress のスタブ（thunk 0x407070）の呼び出しで、rdx = 0x418ece の ANSI 名。uilang-run.md §1 で、名前が `GetThreadPreferredUILanguages`（DLL の .data を読んだ）と確認済み。
- probe: `out-null0/out/probe-896.c`（`out-uilang/out/after-uilang-896.c` を sed で複製。元は変えていない。4 行に `fprintf(stderr, "PROBE ...")` を挿入）を `build.sh` でビルドして実行（`run-probe-896.err`、EXIT=134）:

```
PROBE write [0x15eaee0] at 0x418e63 = 0
PROBE write [0x15eaee8] at 0x418e85 = 0
PROBE write [0x15eaef0] at 0x418ea7 = 0x7ff900000540
PROBE before call at 0x4193c4: [0x15eaee0] = 0 rcx=0x38 rdx=0x7df2c5bfd4ac r8=0 r9=0x7df2c5bfd508
```

- 解釈: `[0x15eaee0]` は GetProcAddress("GetThreadPreferredUILanguages") の戻り値で、応答表にないので 0。`[0x15eaef0]` は前回の追加（GetThreadUILanguage）で擬似 VA 0x7ff900000540。`[0x15eaee8]`（SetThreadPreferredUILanguages）も 0。引数は ecx = 0x38、r8 = 0（NULL バッファ）で、objdump の読みと実測が一致した。

## 3. 原因（確定）

GetProcAddress の応答表に `kernel32.dll!GetThreadPreferredUILanguages` がなく、`fle_imp_GetProcAddress` が NULL を返した（decision 0015 の動作）。別の原因（未初期化のグローバル、未実行の初期化関数など）ではない。0x418e10 は実行済みで、`[0x15eaee0]` を NULL のまま保存し、0x4193c4 は NULL を確かめずに呼ぶ。

## 4. 仕様の出典と確認状況

- GetThreadPreferredUILanguages: https://learn.microsoft.com/en-us/windows/win32/api/winnls/nf-winnls-getthreadpreferreduilanguages （ms.date 2018-12-05、updated_at 2025-07-01、取得 2026-10-20）。確認できた点: シグネチャ `BOOL f(DWORD dwFlags, PULONG pulNumLanguages, PZZWSTR pwszLanguagesBuffer, PULONG pcchLanguagesBuffer)`。形式フラグ MUI_LANGUAGE_ID / NAME（排他、既定 NAME）。フィルタ MUI_MERGE_USER_FALLBACK（既定）、MUI_MERGE_SYSTEM_FALLBACK（0x20 との組み合わせのみ。組み合わせでフォールバックと中立言語が入る）、MUI_UI_FALLBACK（= 両方）、MUI_THREAD_LANGUAGES。NULL バッファかつサイズ 0 でサイズ問い合わせ（2 つの NUL を含む）。失敗は ERROR_INSUFFICIENT_BUFFER。ID 形式は "0409"（en-US）、"0009"（en）。
- 確認できなかった点: フラグの**数値**はこのページ本文にない（ページの表は名前だけ）。数値は mingw-w64 の `winnls.h`（https://raw.githubusercontent.com/mingw-w64/mingw-w64/master/mingw-w64-headers/include/winnls.h、第三者のヘッダ、取得 2026-10-20）: MUI_LANGUAGE_ID 0x4、MUI_LANGUAGE_NAME 0x8、MUI_MERGE_SYSTEM_FALLBACK 0x10、MUI_MERGE_USER_FALLBACK 0x20、MUI_UI_FALLBACK = 0x30、MUI_THREAD_LANGUAGES 0x40。公式の値ではない。
- 「User Interface Language Management」: https://learn.microsoft.com/en-us/windows/win32/intl/user-interface-language-management （updated_at 2025-03-11、取得 2026-10-20）。リソースローダのフォールバックリストは言語の後にその中立形を置き、重複を除く（例 "fr-FR, fr, es-ES, es, ..."）。
- SetThreadPreferredUILanguages: https://learn.microsoft.com/en-us/windows/win32/api/winnls/nf-winnls-setthreadpreferreduilanguages （同日取得）。読んだが応答は作っていない（decision 0022 §4）。
- 判断（推測を含む）: 中立形を "en"（ID は "0009"）とすること、0x20 のときは中立形を入れないことは、ページ（例と文）からの読みで、ページの規則そのものではない（decision 0022 §2）。

## 5. 実装（実測）

- 新規: `lift/runtime/fle_uipref.h/.c`。`lift/out-null0/work/` で作り、`clang -std=gnu11 -O0 -Wall -Wextra` で警告 0 を確認してから `runtime/` に 1 回で置いた。言語名は `fle_imp_GetLocaleInfoW`（LOCALE_SNAME、LOCALE_SISO639LANGNAME）経由。ページが定めない入力（0x4、0x8、0x10、0x20 以外のビット、両形式、0x10 のみ、NULL の出力ポインタ）は `fle_stop` で止める。
- 追記: `imports_table.rs` に 1 行（`GetThreadUILanguage` の行の直後。編集の直前に読み直した）。生成物の応答表の擬似 VA は 0x7ff900000550。
- 新規: `lift/crates/fe-lift/tests/null0.rs`（6 件）: thunk、応答表の行（擬似 VA が dispatch にもある）、Set は応答表にない、実行（0x38 / 0x28 / 0 / ID 形式 0x34, 0x24 のサイズ問い合わせと取得、バッファ不足、NULL バッファ、ちょうどのサイズ、成功時に last error 不変、綴り・モジュール違いは NULL）、未定義入力 6 種が止まる、ソースに言語の値がない。
- 0x38 の結果: `"en-US\0en\0\0"`、数 2、サイズ 10 文字。

## 6. テスト（実測）

- `cargo test --offline -p fe-lift --test null0`: 6 件通過（`out-null0/test-null0-1.txt`、EXIT=0）。
- `cargo test --offline --workspace --no-fail-fast`: **256 件通過、0 件失敗**、41 バイナリ（`out-null0/test-workspace-1.txt`、EXIT=0）。前回の 208 件との差は 48 件。私の分は 6 件で、残りは他の担当のテスト（proc.rs、time.rs など、ワークツリーにある未コミットのファイル）と読める（内訳の件数は確かめていない）。

## 7. 生成、clang、リンク（実測）

- 生成器: `lift/out-null0/tool`（`out-uilang/tool` の複製）。ビルド EXIT=0（`build-tool.txt`）。seed は `out-null0/seeds-896.txt`（896 件）。
- 生成: EXIT=0（`gen-null0-896.txt`、`.err` は 0 バイト）。`out/after-null0-896.c`（42,530,654 バイト）。functions 8427、closure_instructions 497367（前回と同じ）、c_lines 548442（前回 548361、+81）。`fle_resp_count` は 84 → 111（+27）、`fle_dispatch_count` は 8511 → 8538（+27）。応答表 1 行につき C が 3 行（宣言、dispatch 表、応答表）増えるので 27 × 3 = 81 と一致する。27 行のうち 1 行が私の GetThreadPreferredUILanguages（擬似 VA 0x7ff900000550）で、残り 26 行は他の担当（proc、time）が追記した行（実測: 数と行数の一致。誰の行かは `imports_table.rs` の差分の読み）。`unsupported_total` 541（前回 563）。
- clang -O0 とリンク: `build.sh`、`EXTRA_RT="fle_int_more fle_sse_more fle_locale fle_sys fle_x87ctl fle_vfs fle_reg fle_env fle_uilang fle_proc fle_time fle_uipref"`、EXIT=0（`build-896.txt`）。

## 8. 実行（実測）

`run-null0-896.txt`（EXIT=134、SIGABRT）。

| 項目 | 前回（uilang-run.md） | 今回 |
|---|---|---|
| 停止点 | 0x4193c4（f_4193a0、`call [0x15eaee0]`） | **0x407b85**（f_407a50 内、`bsf eax, esi`） |
| abort の種類 | fle_dispatch_miss | **fle_unsupported**（`fle: unsupported instruction at 0x407b85`） |
| 関数実行数（distinct） | 97 | 112 |
| 関数の呼び出し | 150 | 208 |
| 呼び出し深さ | 15 | 18 |

0x4193c4 は通過した。根拠: 入口ログに `entry 150 callee=0x409410 guest-return=0x4193da`（0x4193d5 の確保 call の後）がある。これは Get の 1 回目（サイズ問い合わせ）が成功（al ≠ 0）して 0x4193d5 に進んだことを示す（0x4193ca の `test al,al; je` の読み）。

## 9. 次の停止点（実測と読み）

- スタック（上位 10 件、実行ログ）: `#0 0x407a50 f_407a50 guest-return=0x409424`、`#1 0x409410 f_409410`、`#2 0x412c20`、`#3 0x413060`、`#4 0x414ec0`、`#5 0x419f60`、`#6 0x41a1a0`、`#7 0x41a3d0`、`#8 0x418b80`、`#9 0x418be0`。
- 命令: `0x407b85: bsf eax, esi`（objdump）。直前: `0x407b75: movsx esi, BYTE PTR [rbx+0x1]`、`0x407b79: and esi, [0x15e6bbc]`、`je 0x407c14`。f_407a50 は 0x409410 から呼ばれる、サイズ別の空きリスト（`[0x15e6bc0]` のビットマップ、`[0x15e6c40]` のリスト頭）を持つ確保関数に見える（読み。推測）。
- 未対応の種類: **x86 命令 `bsf`**（生成器の「mnemonic outside minimal set」）。応答表の問題ではなく、生成器（fe-lift の insn 実装）の未対応。生成統計で `Bsf` は 5 か所、同じ関数に `bts`（5）、`rol`（4）、`pause`（3）、`xchg`（11）、`lock cmpxchg` 周辺もある。`unsupported` の命令種は 49 種（`gen-null0-896.txt`）。
- 次の作業（案）: `bsf`（と、同じ関数で続く `rol`、`bts`、`pause`、`xchg`）を fe-lift に実装する。これらは Intel の命令仕様で意味が決まる（値を作る必要はない）。ただし `bsf` は ZF 以外のフラグと、ソースが 0 のときのデスティネーションが「未定義」とされる点の扱い（Intel SDM の記述を確認して決める）が要る。担当ファイル（`lift/crates/fe-lift/src/insn_*.rs`）は今回の私の担当外なので実装していない。

## 10. 失敗・拒否された操作

- 拒否はなし。
- 失敗: (a) 最初の probe 実行は、引数なしで実行して `image: open failed`（EXIT=2）。引数に DLL のパスを渡して再実行した。(b) 最初の `problem_write` はツール名の取り違え（存在しない名前）で失敗し、ファイルは作られなかった。正しいツール名で書き直した。
- decision 0022 の初版に、「Set の呼び出し点に到達していない」と実測のように読める文があった。入口ログは擬似 VA のスタブ呼び出しを記録しないので根拠にならず、推測と明記する形に直した。
