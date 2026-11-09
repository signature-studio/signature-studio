# GetThreadUILanguage の応答と計測実行

作業者による実行記録。推測と実測を分けて書く。判断は `docs/decisions/0021-uilang.md`。前回の記録は `docs/investigations/vfs-run.md`（停止点 0x419419、fle_dispatch_miss）。

## 1. 0x419419 の読み（objdump、実測）

- 停止点: f_419400 の `0x419419: call QWORD PTR [rip+0x11d1ad1]  # 0x15eaef0`。引数なしで呼び、戻り値 ax を `cmp ax, di`（di = f_419400 の第 3 引数）と比べる。
- `[0x15eaef0]` の書き込み元は 1 か所だけ: `0x418ea7: mov [rip+0x11d2042], rax`（objdump の grep で全体に 2 件: この書き込みと 0x419419 の読み）。
- 書き込む関数は 0x418e10（呼び出し元 0x41ce4b、ログの entry 51）。内容:
  - 0x418e14〜0x418e39: ロック初期化、`call 0x407220`（`jmp [0x1971450]` = kernel32!GetVersion。imports.txt の読み）の戻り値の下位バイト ≥ 6 を `[0x15eaef8]` に置く（Vista 以降のメジャー版）。
  - その値が非 0 のとき、`lea rcx, 0x418eb4`（UTF-16 の `kernel32.dll`、`GetModuleHandleW` = thunk 0x407060 に渡す）。続いて GetProcAddress（thunk 0x407070、IAT 0x1971600）を 3 回呼ぶ:

| 0x418e5e の rdx | ANSI 名（DLL の .data をバイトで読んだ） | 結果の保存先 |
|---|---|---|
| 0x418ece | `GetThreadPreferredUILanguages` | `[0x15eaee0]`（0x418e63） |
| 0x418eec | `SetThreadPreferredUILanguages` | `[0x15eaee8]`（0x418e85） |
| 0x418f0a | **`GetThreadUILanguage`** | **`[0x15eaef0]`**（0x418ea7） |

- 戻り値 NULL の確認（`test` や `je`）は 0x418ea7 の前後にない。NULL のまま保存される。呼び出し側（0x419419 など）も NULL を確かめない（objdump の読み）。

## 2. 実行時の値（probe、実測）

`out-vfs/out/after-vfs-896.c` を sed で複製した `out-uilang/out/probe-896.c`（4 行に `fprintf(stderr, "PROBE ...")` を挿入。元のファイルは変えていない）を、`build.sh` でビルドして実行（`run-probe-896.err`、EXIT=134）:

```
PROBE write [0x15eaee0] at 0x418e63 = 0
PROBE write [0x15eaee8] at 0x418e85 = 0
PROBE write [0x15eaef0] at 0x418ea7 = 0
PROBE before call at 0x419419: [0x15eaef0] = 0
```

- 3 つとも GetProcAddress が 0 を返した（応答表に 3 つの API がなかった）。0x419419 で `[0x15eaef0]` は 0。途中報告の「GetThreadUILanguage が NULL のまま呼ばれた」は実測で確かめられた。宛先は 0（他の値ではない）。

## 3. 原因（確定）

GetProcAddress の応答表（`imports_table.rs`）に `kernel32.dll!GetThreadUILanguage` がなく、`fle_imp_GetProcAddress` が NULL と ERROR_PROC_NOT_FOUND を返した（decision 0015 の動作）。生成物の `fle_dispatch_miss` は、NULL を `fle_dispatch` に渡した結果。

## 4. 仕様の出典と確認状況

- GetThreadUILanguage: https://learn.microsoft.com/en-us/windows/win32/api/winnls/nf-winnls-getthreaduilanguage （ms.date 2018-12-05、updated_at 2025-07-01、取得 2026-10-20）。確認できた点: Syntax `LANGID GetThreadUILanguage();`（引数なし）。Return value は、SetThreadUILanguage / SetThreadPreferredUILanguages で結び付けた言語、なければユーザー／システムの UI 言語を示しうる。Remarks: MUI_MERGE_SYSTEM_FALLBACK | MUI_MERGE_USER_FALLBACK | MUI_LANGUAGE_ID で GetThreadPreferredUILanguages を呼び、リストの先頭を返すのと同じ。LIP の言語は 1400 / 1000。
- GetThreadPreferredUILanguages: https://learn.microsoft.com/en-us/windows/win32/api/winnls/nf-winnls-getthreadpreferreduilanguages （同日取得）。MUI_LANGUAGE_ID では en-US は `"0409"`（4 文字の 16 進）で、これは GetThreadUILanguage の値 0x0409 と整合する。読んだが、応答は作っていない（decision 0021 §2）。
- 確認できなかった点: ページは「ユーザー」と「システム」のどちらの UI 言語かを厳密に定めない。既定ロケールの値はページにない（0016 の環境設定の値を使う）。

## 5. 実装（実測）

- 新規: `lift/runtime/fle_uilang.h/.c`（`out-uilang/work/` で作り、`clang -Wall -Wextra` で警告 0 を確認してから `runtime/` に置いた）。`R[0] = fle_emulated_locale.lcid & 0xFFFF`。
- 追記: `imports_table.rs` に 1 行（83 → 84 行。生成物の `fle_resp_count = 84`）。編集の直前に読み直した。同時に追記する他のサブエージェントの行は、その時点で見えなかった（`fle_proc*`、`fle_time*` も runtime/ になかった）。
- 新規: `lift/crates/fe-lift/tests/uilang.rs`（4 件）: thunk がランタイムを呼ぶ、応答表の行と擬似 VA、GetModuleHandleW → GetProcAddress → fle_dispatch の経路で 0x409、レジスタの状態に依存せず last error（77）を変えない、大文字小文字・綴り・モジュール（user32）違いは NULL と 127、ソースに 0x409 の定数がない。

## 6. テスト（実測）

- `cargo test --offline -p fe-lift --test uilang`: 4 件通過（`test-uilang-1.txt`、EXIT=0、DONE）。
- `cargo test --offline --workspace --no-fail-fast`: **208 件通過、0 件失敗**、38 バイナリ（`test-workspace-1.txt`、EXIT=0、DONE）。前回 204 件 + 4 件。

## 7. 生成、clang、リンク（実測）

- 生成器: `lift/out-uilang/tool`（`out-vfs/tool` の複製）。ビルド EXIT=0（`build-tool.txt`）。seed は `out-uilang/seeds-896.txt`（896 件、`out-vfs` の複製）。
- 生成: EXIT=0（`gen-uilang-896.txt`、`.err` は空）。`out/after-uilang-896.c`（42,524,298 バイト）。functions 8427、closure_instructions 497367（前回と同じ）、c_lines 548361（+3）。`fle_resp_count = 84`、`GetThreadUILanguage` は擬似 VA 0x7ff900000540。`unsupported_total` 563、`Jmp: indirect through IAT slot` 326（前回と同じ。GetThreadUILanguage は GetProcAddress 経由のみで、IAT の thunk はなかったため）。
- clang -O0 とリンク: `build.sh`、`EXTRA_RT="fle_int_more fle_sse_more fle_locale fle_sys fle_x87ctl fle_vfs fle_reg fle_env fle_uilang"`、EXIT=0（`build-896.txt`）。この時点で `runtime/` の fle_*.c のうち fle_imports、fle_modules、fle_string 以外は上の 9 個と、`build.sh` が常にリンクする fle_mxcsr、fle_cpuid、fle_int_ext、fle_osver で全部。

## 8. 実行（実測）

`run-uilang-896.txt`（EXIT=134、SIGABRT）。

| 項目 | 前回（vfs-run.md） | 今回 |
|---|---|---|
| 停止点 | 0x419419（f_419400 内、`call [0x15eaef0]`） | **0x4193c4**（f_4193a0 内、`call [0x15eaee0]`） |
| abort の種類 | fle_dispatch_miss | fle_dispatch_miss |
| 関数実行数（distinct） | 96 | 97 |
| 関数の呼び出し | 149 | 150 |
| 呼び出し深さ | 14 | 15 |

0x419419 は通過した（GetThreadUILanguage が 0x409 を返し、`cmp ax, di` に進んだ）。

## 9. 次の停止点（実測と読み）

- スタック: `#0 0x4193a0 f_4193a0 guest-return=0x41949c`、`#1 0x419400 f_419400`（0x419613 へ戻る）。以下は前回と同じ。
- 命令: `0x4193c4: call QWORD PTR [rip+0x11d1b16]  # 0x15eaee0`（生成物は `fle_ld(0x15eaee0)` → `fle_dispatch`）。実行時の `[0x15eaee0]` は 0（probe の実測: 0x418e63 の書き込みが 0）。
- 引数（0x4193b5〜0x4193c1、objdump）: ecx = 0x38、rdx = rbp+0x2c（pulNumLanguages）、r8 = 0（バッファ NULL）、r9 = rbx（pcchLanguagesBuffer）。dwFlags 0x38 = MUI_LANGUAGE_NAME(0x8) | MUI_MERGE_SYSTEM_FALLBACK(0x10) | MUI_MERGE_USER_FALLBACK(0x20)、すなわち MUI_UI_FALLBACK | MUI_LANGUAGE_NAME。フラグの数値は MS のページ本文になく、mingw-w64 の `winnls.h`（https://raw.githubusercontent.com/mingw-w64/mingw-w64/master/mingw-w64-headers/include/winnls.h、第三者のヘッダ。取得 2026-10-20）で確認した（公式ではない）。MUI_LANGUAGE_ID は含まれない。r8 = NULL と `*pcch` による、サイズ問い合わせの形。
  - 訂正: 本ファイルの初版は 0x38 を MUI_LANGUAGE_ID を含む組合せと書いたが誤りで、上のとおり直した。これにより、この呼び出し元が受け取る言語リストは `"0409"` ではなく言語名形式（en-US なら `"en-US"`）になる。
- IAT の名前: なし（GetProcAddress の戻り値。名前は `GetThreadPreferredUILanguages`）。
- 未対応の種類: 応答表にない API（`kernel32.dll!GetThreadPreferredUILanguages`）。`SetThreadPreferredUILanguages`（`[0x15eaee8]`、0x41948a ほかで呼ばれる）も同じ状態で、続いて到達する。
- 次の作業（案）: GetThreadPreferredUILanguages と SetThreadPreferredUILanguages を実装する。GetThreadPreferredUILanguages は、この呼び出し元の dwFlags（言語名形式 + MERGE フラグ）では、en-US なら "en-US\0\0"、数 1、必要サイズ 7 文字になると読める（ページの「2 つの NUL を含む」と、"en-US" が 5 文字であることから。MERGE フラグがフォールバック言語（例: "en"）をリストに加える規則（ページの MUI_MERGE_SYSTEM_FALLBACK の説明）を含めると数と長さが変わりうるので、決定前にページで確かめる必要がある。推測）。Set は戻り値と状態を決める必要がある。決定は 0021 の §2。

## 10. 失敗・拒否された操作

なし。
