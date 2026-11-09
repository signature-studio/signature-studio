# Blend2D の `GetModuleHandleA`（決定 0091）の実装と、3 モジュール構成での次の停止点

凡例: **実測**（この作業で実行して得た値）／**読み**（逆アセンブル、ソース、文書を読んだ結果。実行していない）／**未確認**。

この文書の状態: 骨組み（節ごとに追記して保存する）。

## 1. 要約

| # | 項目 | 結果 | 区分 |
|---|---|---|---|
| 1 | `0x4899fed` の `GetModuleHandleA` の引数 | NULL ではなく ANSI 文字列 **`api-ms-win-core-synch-l1-2-0.dll`**（ASCII、小文字、`.dll` つき、パス区切りなし）。`GetModuleHandleW` の規則では決定 0081 の仮想モジュール N = 27（`0x7FF8001B0000`） | 実測（gdb 読み取り 2 回一致、ファイルのバイト） |
| 2 | 戻り値の使い方 | `rbx` に保存、NULL 検査、非 NULL なら `GetProcAddress(rbx, "WaitOnAddress" / "WakeByAddressSingle" / "WakeByAddressAll")` の 3 回。ハンドルは保存しない | 読み（objdump）。3 回の `GetProcAddress` は §7.3 で実測 |
| 3 | Learn のページ | 取得した。NULL、拡張子、末尾の `.`、パス、大文字小文字、参照カウント、失敗は NULL。**失敗時の last error の値、空文字列、パスの照合、非 ASCII、ロード中のモジュールは書かれていない** | 実測（取得） |
| 4 | 実装 | `fle_modules.c` の `GetModuleHandleW` の本体を共通化し、`GetModuleHandleA` を 1 行で足した。A は 0x80 以上のバイトで名前付き停止。応答表に 1 行（末尾）。新しい `.c`/`.h` なし | 実測 |
| 5 | テスト | 97 binaries / **1,178 passed** / 0 failed / 0 ignored（基準 1,172。+6 は新規）。既存テストの期待値の変更は 0 件 | 実測 |
| 6 | **FLEngine の生成 C の SHA-256** | **変わった**: `43985d62…` → `eb9d8e7f…`。変わった行は応答表だけの 5 か所 7 行（`GetModuleHandleA` の行とその個数）。**変わった関数は 0 個**。原因は「応答表に行を足したこと」だけ | 実測（`diff`） |
| 7 | 単独構成の a4 | **基準と一致**（145/942、functions 821、entries 15,685、停止 `LoadLibraryW "dsp_ippv2_x64.dll"`）。2 回同一 | 実測 |
| 8 | 3 モジュール（種 58 個あり） | `GetModuleHandleA` は通った。**次の停止は `fle: unsupported instruction at 0x489cab9`**（`f_489caa0`。`KERNEL32!AcquireSRWLockExclusive`。分類 (i) 未応答の import）。a4: init_count 150/942（前回と同じ）、last unit 149、functions_distinct **1,059**（前回 1,058）、function_entries **27,319**（前回 27,317）、ATTACH 内の import **178 回**（前回 174）。DllMain は戻らず、LoadLibraryW は 0x4700000 を返さない。2 回同一 | 実測 |

**測定だけで、新しい停止は直していない。** 次の停止は `GetModuleHandleA` の直接の続き（同じ初期化子 `f_4899d90` が `0x489a1d2` で呼ぶ関数）で、行なしの SRWLock 系（`AcquireSRWLockExclusive` ほか）。TLS、JIT、ロケールには届いていない。unit 158（quickfontcache_x64.dll）には届いていない。

## 2. 停止点の引数の測定（実装の前。読み取りのみ）

### 2.1 手順（実測）

- 対象: コミット b0dc3ab（決定 0090 の直後。**runtime は未変更**）の 3 モジュール構成の two-native（`lift/out-jtb2d90/three/v-b2d58/native/two-native`。FLEngine + 縮小 dsp_ippv2 基底 0x2000000 + Blend2D 基底 0x4700000、種 58 個あり。SHA-256 先頭 `4fd25502…`、`24e01ec9…` のリベース済み DLL）。
- 読み取り専用 gdb: `lift/out-three/gdb/ro_imports3.py` を `lift/out-gmh/gdb/ro_args_gmha.py` にコピーして使った。変更は 2 点: (1) 停止時のログに**読み取りだけ**の 3 行を足した（`lpModuleName` の文字列、後続の `GetProcAddress` の名前 3 つ、後でゲストが書くセル 4 つの停止時の値。`astr`/`ld` は元のスクリプトの読み取り関数）、(2) ブレークポイントの行を、前回の測定（`out-jtb2d90/gdb/ro_imports.py`）と同じ `fle_load.c:155` に直した（`ro_imports3.py` は `:153` で、runtime の行がずれていた。`LoadRet` のログ文言だけ変わる）。`return`、`set var`、`call`、値の書き込みは無い。環境変数は `GDBI_BIN`、`GDBI_REAL=<dsp>:<Blend2D>`、`GDBI_FLE`、`GDBI_OUT`（存在しないこと）、`GDBI_STDOUT`、`GDBI_STDERR`。
- 2 回実行（`lift/out-gmh/run/args3-imports.txt`、`args4-imports.txt`）。**2 つのログは `cmp` で全バイト一致**。`fle_imp_*` の列（通し番号と名前）は前回の測定（`out-jtb2d90/run/b2d58-imports.txt`、1,699 件）と**同一**（`diff` で差なし）。停止は `fle: unsupported instruction at 0x4899fed`（stderr）。
- 最初に実行した 2 回（`args1`、`args2`）は `fle_load.c:153` のままで、ブレークポイントが `fle_realmod_load` の戻りではなく別の行に当たり `LOADLIBRARY-RETURN` が 22 回出た。import の列は同じだったが、行を直して取り直した（上の (2)）。`args1`、`args2` は作業物に残してある（判断には使っていない）。

### 2.2 `0x4899fed` の引数（実測）

| 項目 | 値 | 区分 |
|---|---|---|
| 呼び出し | `0x4899fed: call qword ptr [rip+0x1d0d5]` → IAT スロット `0x48b70c8` = `KERNEL32.dll!GetModuleHandleA`（pefile で解決） | 実測（バイト列） |
| 直前の命令 | `0x4899fe6: lea rcx,[rip+0x435d3]` → `rcx = 0x48dd5c0` | 実測（objdump） |
| **`lpModuleName`** | **NULL ではない。ANSI 文字列 `api-ms-win-core-synch-l1-2-0.dll`**（停止時の gdb 読み取り: `rcx=0x48dd5c0 ansi='api-ms-win-core-synch-l1-2-0.dll'`。pefile でファイルのバイトから読んだ値も同じ） | 実測 |
| 文字列の性質 | すべて ASCII（0x80 以上のバイトなし）、小文字、拡張子 `.dll` つき、パス区切りなし、長さ 32 文字 | 実測 |
| この名前の既存の扱い | `GetModuleHandleW` の規則では、決定 0081 の仮想モジュール N = 27（`fle_modules[26]`）。ハンドル `0x7FF800000000 + 0x10000 × 27 = 0x7FF8001B0000`（`blend2d-seed58.md` §6 が、Blend2D の ATTACH の中の `GetModuleHandleW("api-ms-win-core-synch-l1-2-0")` が `0x7ff8001b0000` を返したと記録している値と同じ。同文書 §6 の 226 行目付近） | 読み（ソースと算術） |

### 2.3 戻り値の使い方（`f_4899d90`。objdump の読み）

`0x4899ff3` から `0x489a061`:

```
4899ff3: mov rbx,rax            ; 戻り値を rbx に保存
4899ff6: test rax,rax
4899ff9: je 0x489a061           ; NULL なら以下の GetProcAddress 3 回を飛ばす
4899ffb: lea rdx,[0x48dd5b0]    ; "WaitOnAddress"
489a002: mov rcx,rax
489a005: call [0x48b70d0]       ; KERNEL32!GetProcAddress  → [0x48f3280]
489a012: lea rdx,[0x48dd600]    ; "WakeByAddressSingle"
489a019: mov rcx,rbx
489a01c: call [0x48b70d0]       ;                          → [0x48f3288]
489a029: lea rdx,[0x48dd5e8]    ; "WakeByAddressAll"
489a030: mov rcx,rbx
489a033: call [0x48b70d0]       ;                          → [0x48f3290]
489a040: cmp [0x48f3280],rdi    ; rdi = 0。いずれかの結果が 0 なら eax = 0、全部非 0 なら eax = 1
...
489a05b: mov [0x48f31a0],eax    ; フラグ（停止時の値は 0）
```

- 使い方は **(1) NULL 検査（NULL なら `GetProcAddress` を飛ばす）、(2) `GetProcAddress` の hModule に 3 回渡す**。ハンドルそのものはメモリに保存しない（`rbx` は 0x489a061 以降 `0x489a147` の `lea rbx,[0x4700000]` で上書きされる）。読み。
- 停止時のセル（gdb 読み取り。実測）: `[0x48f3280]=[0x48f3288]=[0x48f3290]=0`、`[0x48f31a0]=0`（まだ書かれていない）。
- **既存の応答表で後続が決まる**（読み）: `GetModuleHandleA` が N = 27 のハンドルを返せば、`GetProcAddress(N=27, "WaitOnAddress" ほか)` は応答表に `api-ms-win-core-synch-l1-2-0.dll` の行が無いので **NULL と 127**（決定 0081）。3 つとも NULL なので `[0x48f31a0] = 0`。ゲストは WaitOnAddress の経路を使わない側（SRWLock などの別の経路）に入る。**決定 0081 が dsp_ippv2 について記録した形と同じ**（`blend2d-lift-feasibility.md` §5.2 の読み。`.xc` の初期化子は NULL でも続行する）。
- `f_4899d90` 以外に `0x48b70c8` を参照する `call`/`jmp` は無い（Blend2D の `.text` の objdump 472,892 行を `grep`。`.text` の全範囲。実測）。したがって Blend2D で `GetModuleHandleA` が呼ばれるのはこの 1 か所だけ（`blend2d-lift-feasibility.md` の「`GetModuleHandleA` 1」と一致）。FLEngine は `GetModuleHandleA` を import していない（import は `GetModuleHandleW` のみ。pefile。実測）ので、**FLEngine の生成 C の `jmp thunk` は変わらない**見込み。dsp_ippv2（縮小前の `out-ext` 版と同じ import 表）も `GetModuleHandleA` を import していない（`GetModuleHandleW`、`GetModuleHandleExW`）。

## 3. Learn のページ

- URL: <https://learn.microsoft.com/en-us/windows/win32/api/libloaderapi/nf-libloaderapi-getmodulehandlea>（この作業で取得した。実測。ページの `updated_at` は 2025-07-01、`ms.date` は 2018-12-05）。
- 引用（ページの英語の原文。この文書では訳さず原文を引く）:
  - Parameters, `lpModuleName`: 「The name of the loaded module (either a .dll or .exe file). If the file name extension is omitted, the default library extension .dll is appended. The file name string can include a trailing point character (.) to indicate that the module name has no extension. The string does not have to specify a path. When specifying a path, be sure to use backslashes (\), not forward slashes (/). The name is compared (case independently) to the names of modules currently mapped into the address space of the calling process.」
  - 「If this parameter is NULL, **GetModuleHandle** returns a handle to the file used to create the calling process (.exe file).」
  - 「The **GetModuleHandle** function does not retrieve handles for modules that were loaded using the **LOAD_LIBRARY_AS_DATAFILE** flag.」
  - Return value: 「If the function succeeds, the return value is a handle to the specified module. If the function fails, the return value is NULL. To get extended error information, call GetLastError.」
  - Remarks: 「The **GetModuleHandle** function returns a handle to a mapped module without incrementing its reference count.」
- **ページが書くこと**: NULL は呼び出しプロセスを作ったファイルのハンドル。拡張子が無ければ `.dll` を補う。末尾の `.` は「拡張子なし」を示す。パスは書かなくてよい（書くならバックスラッシュ）。比較は大文字小文字を区別しない。成功は参照カウントを増やさない。失敗は NULL で、`GetLastError` で拡張エラー情報を得る。
- **ページが書かないこと**（決定ファイルでも「書かれていない」と明記する）: (1) 失敗時の `GetLastError` の**値**（`ERROR_MOD_NOT_FOUND` と書いていない。`GetModuleHandleW` と同じ 126 は決定 0015 の PM の決定）、(2) 空文字列の扱い、(3) パスを含む名前をどう照合するか（「ロード済みモジュールの名前と比較する」とだけ書き、パスの比較規則を書かない）、(4) 非 ASCII の文字の大文字小文字の畳み込みと、ANSI のコードページでの解釈、(5) 成功時に `GetLastError` を触るか、(6) 名前の最大長、(7) 「DllMain の実行中のモジュール」が見つかるか（ロード中のモジュールの扱い）、(8) 区切り文字の `/` を受けるか（ページは「forward slashes (/) ではなくバックスラッシュを使え」と書くのみ。受けると書いても拒否すると書いてもいない）。
- `GetModuleHandleA` と `GetModuleHandleW` で規則が違うとは**ページは書いていない**（同じ関数ページで、`lpModuleName` の型だけが `LPCSTR` と `LPCWSTR`）。

## 4. 実装

- `lift/runtime/fle_modules.c`: `fle_imp_GetModuleHandleW` の本体を `static void get_module_handle(const char *api, int wide)` に移し、`fle_imp_GetModuleHandleW` は `get_module_handle("GetModuleHandleW", 1)`、新しい `fle_imp_GetModuleHandleA` は `get_module_handle("GetModuleHandleA", 0)` を呼ぶ 1 行ずつにした（同じ表、同じハンドル、同じ照合、同じ NULL の扱い、同じ 126、同じ実在モジュールの扱い、同じ停止）。違いは 3 点だけ: (1) 名前の読み方（`wide` は 2 バイト単位、A は 1 バイト単位。ASCII のバイトは同じ単位に広がる）、(2) **A は 0x80 以上のバイトで名前付き停止**（`fle: GetModuleHandleA with a byte above 0x7F (0x..) is not supported (the page does not define the ANSI code page folding; decision 0091)`。W は従来どおり「一致しない」として 126）、(3) 停止メッセージと `fle_realmod_module_handle` の `api` 引数が関数名（W は従来と同じ文字列 `GetModuleHandleW`）。W の停止メッセージ（パス、256 文字超）は文字列が `%s` になっただけで、W では**従来と同一の文字列**になる（テスト `undefined_cases_abort` の既存の行が通る）。
- `lift/runtime/fle_modules.h`: `fle_imp_GetModuleHandleA` の宣言とコメント。
- `lift/crates/fe-lift/src/imports_table.rs`: `TABLE` の**末尾**に `("kernel32.dll", "GetModuleHandleA", "fle_imp_GetModuleHandleA", "jmp_imp:GetModuleHandleA")` を 1 行足した（コメント 2 行つき）。末尾なので既存の行の擬似 VA は動かない（決定 0015）。他の行は触っていない。
- 新しい `.c`、`.h` は作っていない。他の runtime ファイル、`fe-decode`、`fe-lift` の他のソースは読み取りのみ。

## 5. テスト

新規 6 件（`#[test]`）。既存テストの期待値の変更は**無い**（`GetModuleHandleA` が行なし／停止だと表明していた既存の行を、`lift/` 全体の `grep GetModuleHandleA`（`*.rs`、`*.c`、`*.h`、`*.py`、`*.sh`、`*.toml`）で探したが、この作業の前は 0 件だった。実測）。既存テストの書き換え:

| ファイル | 変更 | 理由 |
|---|---|---|
| `tests/modules.rs` | `get_module_handle_name_rules` の `rows` を関数 `name_rows()` に切り出した（行の内容は変えていない。`#[test]` の中身は同じ検査） | 新しい A のテストが同じ行を使うため（件数は増減しない） |
| `tests/realmod_s1/shared.h`、`guest_host.c`、`guest_mod2.c` | `W_GetModuleHandleA` のラッパ、`HostGmhNames`、`HostGmhA` の 2 つのシナリオ、DllMain のモード bit 16（既存のモード 1、2、8 は変えていない）を**追加** | 実在モジュールの合成 PE で A と W を並べるため。34 件のステージ 1 の検査（`check_runs.py`）の入力は変わらない（`the_34_stage1_checks_pass_on_this_runtime` は通った） |

新規テスト（`#[test]` 6 件）:

- `modules.rs` `get_module_handle_a_answers_like_w_on_every_ascii_row`: W のテストの ASCII の行（40 件超）と NULL を、A と W の両方で 1 つのプログラムの中で呼び、**A と W の答えが一致**し、期待値（ハンドルまたは `0 126`。成功時は last error が 5 のまま）と一致する。NULL は両方 `0x400000`。
- `modules.rs` `get_module_handle_a_name_rules`: A だけの 14 名（`api-ms-win-core-synch-l1-2-0.dll`（Blend2D の引数）、大文字、拡張子なし、`Kernel32`、`KERNEL32.`（末尾の `.` で拡張子なし。`kernel32` ではない）、`kernel32.dll.`、`ntdll`、空文字列、`.`、表に無い名前、`dwmapi`（absent。126）、`api-ms-win-core-synch-l1-2-1.dll`（近い名前）、`avrt`、`PowrProf.dll`）。
- `modules.rs` `get_module_handle_a_stops_where_the_page_is_silent_and_w_does_not_stop_on_a_high_unit`: **非 ASCII は名前付き停止**（0x80、`caf\xe9.dll`、0xff、`kernel32\xe9`）、パス（`\`、`/`、`:`）、256 文字超が `GetModuleHandleA` の名前つきで停止（SIGABRT、stderr の文言、`returned` が出ない）。同じ文字を UTF-16 の単位にした W は停止せず `0 126`。
- `modules.rs` `get_module_handle_a_has_a_response_row_at_the_end_and_is_found_by_get_proc_address`: 生成器で `jmp [IAT]` の `GetModuleHandleA` が `fle_imp_GetModuleHandleA(); return;` になり、`unsupported_total` が 0。応答表の**最後の行**（VA `0x7ff900000000 + 0x10 × 行数`）で、既存の行（`GetModuleHandleW` の `0x7ff9000000c0`、`GetProcAddress` の `0x7ff9000000d0`）は動かない。実際にリンクして、`GetProcAddress(kernel32, "GetModuleHandleA")` が擬似 VA を返し、`fle_dispatch` で呼ぶと `api-ms-win-core-synch-l1-2-0.dll` が N = 27 のハンドルを返し、NULL が 0x400000、綴りを変えた `getmodulehandlea` は NULL と 127。
- `realmod.rs` `get_module_handle_a_names_that_need_no_load_answer_like_w`: 実在モジュールの合成 PE のホストから、NULL、`Kernel32`、API セット名、**ロード前の実在モジュール**（NULL と 126）、存在しない名前を A と W で呼び、一致。実在モジュールの表あり／なしの両方で同じログ。
- `realmod.rs` `get_module_handle_a_of_a_real_module_follows_w_before_during_and_after_the_load`: 実在モジュールの名前を、ロード前（NULL と 126）、**DllMain の実行中（決定 0082 の保留。A と W の両方が「未ロード」= 0 と 126）**、ロード後（基底。大文字、拡張子なし。`mod2.` は `mod2` で NULL、`mod2.dll.` は `mod2.dll` で基底。成功時は last error 5 のまま）で A と W に問う。順序（ロード前 < ATTACH < DllMain 内 < `LoadLibraryW` の戻り）も表明。

## 6. 検証の統計

### 6.1 `cargo test --workspace`（実測。`lift/out-gmh/test-ws.txt`）

`FLENGINE_PE=…/binaries/win64/FLEngine_x64.dll`、`CARGO_TARGET_DIR=lift/out-gmh/target`、`TMPDIR=lift/out-gmh/tmp`、`--no-fail-fast`、終了コード 0。

| | binaries | passed | failed | ignored |
|---|---|---|---|---|
| 基準（決定 0090） | 97 | 1,172 | 0 | 0 |
| **今回** | **97** | **1,178** | **0** | **0** |

+6 は上の新規 6 件。`SKIPPED` は 0 件（`grep -c SKIPPED`）。対象の 2 バイナリだけを先に実行した結果（`out-gmh/test-targeted.txt`）は `modules` 16 件、`realmod` 14 件、すべて通過。

### 6.2 FLEngine の生成 C（実測）

`fe-lift lift ../binaries/win64/FLEngine_x64.dll out-gmh/fle.c --pdata`（`lift/out-gmh/target/release/fe-lift`。作業コピー = HEAD b0dc3ab + この作業の変更）:

| | 基準（決定 0088、0090） | 今回 |
|---|---|---|
| SHA-256 | `43985d62321a90f562ce1b8144253e00af13df795e4190d26b6b8ce2537137a1` | **`eb9d8e7fa74ae562742fb1999f6da9b3ca82f2884d1845b5b7319153e1b709cf`（変わった）** |
| サイズ | 215,707,512 B | 215,707,705 B（+193 B） |
| functions | 59,297 | 59,297 |
| bad_nodes | 0 | 0 |
| unsupported_total | 525 | 525 |
| register_jmp_sites、jumptabs、msvc_tables、msvc_rejected | 20、4、0、0 | 20、4、0、0 |
| C の関数（`void f_…(void)`）の数 | 118,594 | 118,594 |

**変わった行の全部**（基準の `out-avrt88/fle.c`（SHA が基準と一致）との `diff`。5 か所 7 行。**C の関数の本体の変更は 0 行**）:

1. `fle_imp_GetModuleHandleA` のプロトタイプ宣言 1 行を、`fle_imp_GetModuleHandleExW` の宣言の次に追加。
2. `fle_dispatch_table` の末尾に `{ UINT64_C(0x7ff900001050), fle_imp_GetModuleHandleA },` を 1 行追加。
3. `const uint64_t fle_dispatch_count = 59557;` → `59558;`。
4. `fle_resp_table` の末尾に `{ "kernel32.dll", "GetModuleHandleA", UINT64_C(0x7ff900001050), fle_imp_GetModuleHandleA },` を 1 行追加。
5. `const uint64_t fle_resp_count = 260;` → `261;`。

この 5 か所の行を取り除くと、新旧の C は**バイト単位で同一**（`diff` で差なし）。変わった**関数は 0 個**。原因は「応答表に行を足したこと」だけ（FLEngine の jmp thunk は `GetModuleHandleW` の 2 か所で、`GetModuleHandleA` を import していない。pefile。`fle_imp_GetModuleHandleW(); return;` は新旧とも 2 か所）。決定 0078 の前例（応答表に行を足すと `fle_resp_table` と `fle_dispatch_table` が変わる）と同じ形。既存の擬似 VA は 1 つも動いていない（行を末尾に足したため）。

### 6.3 単独構成の a4（実測。FLEngine だけ。`lift/out-gmh/a4single/`）

`harness/wasm/build.sh gen split`（emcc は使わない。`gen` と `split` の 2 ステージだけ）で生成 C を作り（`out-gmh/fle.c` と `cmp` で同一、17 パート）、`OUT=… JOBS=2 bash harness/wasm/build_native.sh`（新しい OUT。runtime の .o 46 個を最初から作った）、`OUT=… bash harness/a4-survey.sh` を 2 回:

```
A4 survey 20261024T160628Z   harness exit: 134
static : units=942 runnable=895 empty=46 noproc=1 (header 0x13768b0, units 0x13768e0)
stop   : fle: LoadLibraryW: module "dsp_ippv2_x64.dll" is not a virtual module and not a name this environment is defined to lack (external modules are M2; no handle and no failure is invented for them)
         top of stack: 0x423990 f_423990
depth  : init_count=145 / 942   last unit: index=144 run_rank=132/895 proc=0x65c540 in_progress
         started=132 completed=131 in_progress=1 of 895 runnable   all_units_done=no
         entry check: started_but_not_entered=0 runnable_entered_beyond_init_count=0
funcs  : executed (distinct)=821  entries=15685
```

**基準と一致**（145/942、functions 821、entries 15,685、停止 `LoadLibraryW "dsp_ippv2_x64.dll"`）。2 回の出力は a4-survey の見出し（時刻）と `logs` 行以外同一。（1 回目の直後に同じ秒の `units.tsv` と衝突して 2 回目が `pe-init-table failed` で終わったため、6 秒空けて 2 回目を実行した。決定 0090 の調査 §6.2 にも同じ記録がある。a4-survey は変えていない。）

## 7. 3 モジュール構成の測定（測定のみ。gdb の仮の値なし、runtime の応答追加は `GetModuleHandleA` だけ。新しい停止は直していない）

### 7.1 手順（実測）

- 作業コピー（HEAD b0dc3ab + この作業の変更）の release の `fe-lift`、`fe-pe`（`lift/out-gmh/target/release`）で、FLEngine（`--pdata`）、dsp_ippv2 縮小（`--seeds seeds-idx0-plus-initterm2.txt`）、Blend2D（`--pdata --seeds seeds58-4700000.txt`。種 58 個）の生成 C を作り直した（`lift/out-gmh/three/`）。`lift/out-seed58/harness-two/` を `out-gmh/three/harness-two/` にコピー（`diff -rq` で同一。読み取り専用のスクリプトは変えていない）。リベース済み DLL と種は `out-jtb2d90/three/dll/`、`out-seed58/seeds/` からコピーし、SHA-256 が `4fd25502…efc5`（dsp_ippv2）、`24e01ec9…b4cb`（Blend2D）、種 `ca6439b6…`（58 個）、`cf664439…`（dsp 縮小の種）と一致。
- 前回の `MODULES` と同じ引数（`gen_two.sh rebase gen split merge`）。FLEngine の C は §6.2 の `out-gmh/fle.c` をコピー（`gen_two.sh` の `EXPECT_FLE_SHA` は新しい SHA `eb9d8e7f…` を指定。基準の `43985d62…` を指定すると応答表の行のせいで落ちる。これは §6.2 の変化のとおり）。`build_two_native.sh`（`JOBS=8`）で two-native を作った。runtime のオブジェクト（`native/rt`）、FLEngine のオブジェクト（`native/fle`）、ハーネスは**新しい OUT で最初から**作った（古い .o は再利用していない）。two-native は 612,276,680 B。
- **生成 C の変化**（前回の `out-jtb2d90/three/v-b2d58/gen/` との `diff`。実測）:
  - 縮小 dsp_ippv2（`real.c`）: 応答表の 5 行（プロトタイプ 1、`fle_dispatch_table` 1、`fle_dispatch_count` 8290→8291、`fle_resp_table` 1、`fle_resp_count` 260→261）だけ。**関数の本体は変わらない**（dsp_ippv2 は `GetModuleHandleA` を import していない）。
  - Blend2D（`blend2d.c`）: 同じ応答表の 5 行 + **`L_4899fed:` の 1 行**。`fle_unsupported(UINT64_C(0x4899fed));` が `{ void fle_imp_GetModuleHandleA(void); R[4] -= 8; fle_st(R[4], 8, UINT64_C(0x4899ff3)); fle_imp_GetModuleHandleA(); R[4] += 8; }` に変わった（決定 0077 の `call [IAT]` の形）。Blend2D で他の関数は変わっていない。
  - 表の統合: 59,558 + 8,291 + 2,327 → 69,654 行（前回 69,653。複数の表に在る行 261、`f_<VA>` の衝突 0）。

### 7.2 a4 の行（2 回: 16:10:30Z、16:10:40Z。gdb なし。ログ `lift/out-gmh/three-out/a4/`）

```
A4 survey 20261024T161030Z   dll: …/binaries/win64/FLEngine_x64.dll   harness exit: 134
static : units=942 runnable=895 empty=46 noproc=1 (header 0x13768b0, units 0x13768e0)
stop   : fle: unsupported instruction at 0x489cab9
         top of stack: 0x489caa0 f_489caa0
depth  : init_count=150 / 942   last unit: index=149 run_rank=137/895 proc=0x664260 in_progress
         started=137 completed=136 in_progress=1 of 895 runnable   all_units_done=no
         entry check: started_but_not_entered=0 runnable_entered_beyond_init_count=0
funcs  : executed (distinct)=1059  entries=27319
```

- 2 回の a4 の出力（見出しの時刻と `logs` 行を除く）は同一。stderr は `cmp` で一致、stdout は `rsp=` の行を除いて一致。（1 回目と 2 回目の間は 10 秒空けた。同じ秒の `units.tsv` との衝突を避けるため。a4-survey は変えていない。）
- **次の停止メッセージ全文**: **`fle: unsupported instruction at 0x489cab9`**（`abort kind: fle_unsupported`、`call depth at stop: 18`）。stderr も同じ 1 行。
- **C のスタック**（先頭 10 行。`call depth at stop: 18`。a4 の `native.txt`）:

```
#0 0x489caa0 f_489caa0 guest-return=0x489a1d5
#1 0x4899d90 f_4899d90 guest-return=0x4701009
#2 0x4701000 f_4701000 guest-return=0x48a59eb
#3 0x48b5b90 f_48b5b90 guest-return=0x48a59eb
#4 0x48a59a8 f_48a59a8 guest-return=0x489e34e
#5 0x489e2b4 f_489e2b4 guest-return=0x489e4c4
#6 0x489e264 f_489e264 guest-return=0x489e4c4
#7 0x489e450 f_489e450 guest-return=0xcb000000
#8 0x489e584 f_489e584 guest-return=0xcb000000
#9 0x423990 f_423990 guest-return=0x44db70
```

  （`native.txt` の原文のまま。**前回の停止（先頭が `f_4899d90`）に、1 段深い `f_489caa0` が加わっただけ**で、`#1` 以降は前回の `#0` 以降と同一。）
- `0x489cab9` は `call qword ptr [rip+0x1a5d9]`、IAT スロット `0x48b7098` = **`KERNEL32.dll!AcquireSRWLockExclusive`**（pefile でスロットを解決した。実測）。`imports_table` に行が無い import（行なし 15 名の 1 つ。B8）。`f_489caa0` は `f_4899d90` が `0x489a102` で vtable 風の表（`0x49f4750` 以降）に入れた関数の 1 つ（`0x49f47b0`。`f_4899d90` は `0x49f47d0` にオブジェクトを作り、`0x489a1b8` から `[obj+0x20]` を `call r8` で呼ぶ。読み）で、`rcx+0x30` を `AcquireSRWLockExclusive` に渡し、`[rdi]` を検査し、`ReleaseSRWLockExclusive` で閉じる形（objdump の読み）。**スタックの guest-return `0x489a1d5` は `0x489a1d2: call r8` の次の命令**。

### 7.3 読み取りだけの gdb（`lift/out-gmh/gdb/ro_stop3.py`。`ro_imports3.py` のコピー。`return`、`set var`、`call` なし。ログ `lift/out-gmh/run/m3-ro1-imports.txt`、`m3-ro2-imports.txt`）

- 2 回実行し、**ログは `cmp` で全バイト一致**。
- **Blend2D の DllMain(ATTACH) は戻ったか**: **戻っていない**（`DLLMAIN-RETURN reason=1: out of scope (did not return normally)`。スタックに `f_489e584` が残る。停止時の `eax` は `0x49f4790`（途中の値で、DllMain の戻り値ではない）。DllMain の `eax` は存在しない）。
- **LoadLibraryW が 0x4700000 を返したか**: **返していない**（`LOADLIBRARY-RETURN` は 1 回だけで `R[0]=0x2000000`（dsp_ippv2）。Blend2D の `fle_realmod_load` は戻っていない）。
- Blend2D の DllMain に入るまでの `fle_imp_*` は 1,525 回（前回と同じ。`BLEND2D DllMain entered (f_489e584) rcx=0x4700000 rdx=1 r8=0`）。
- **ATTACH 内の import の列**: **178 回**（通し番号 1526〜1703。前回 174 回）。**前回の列（1526〜1699）と通し番号・名前が同一で、その後ろに 4 つが加わっただけ**（`diff` で、前回の列に対して `1700` 以降の 4 行の追加のみ）:

| 通し番号 | import | 引数と戻り値（読み取り） |
|---|---|---|
| 1698、1699（前回と同じ） | `GetSystemInfo` ×2 | |
| **1700** | **`GetModuleHandleA`**（今回実装） | rcx = `0x48dd5c0`（`api-ms-win-core-synch-l1-2-0.dll`）。戻り値は次の 1701 の rcx が `0x7ff8001b0000`（N = 27）であることから確認（`GetModuleHandleW` の規則と同じ値）。戻りアドレス `0x4899ff3` |
| **1701** | `GetProcAddress`（`WaitOnAddress`） | rcx = `0x7ff8001b0000`。**戻り値 NULL**（応答表に `api-ms-win-core-synch-l1-2-0.dll` の行が無い。決定 0081） |
| **1702** | `GetProcAddress`（`WakeByAddressSingle`） | 同上、戻り値 NULL |
| **1703** | `GetProcAddress`（`WakeByAddressAll`） | 同上、戻り値 NULL |

  名前ごとの回数の前回との差は `GetModuleHandleA` +1、`GetProcAddress` 12 → 15 だけ。**§2.3 の読み（NULL が 3 つ返る）は、実測と一致した**（3 つの戻り値が NULL。停止時のセル `[0x48f3280]=[0x48f3288]=[0x48f3290]=0`、フラグ `[0x48f31a0]=0`）。フラグを書く `0x489a05b` は、停止した `0x489a1d2` の `call r8` よりアドレス順で前にあり、その間は直線の書き込みだけ（objdump の読み）なので、`0x489a05b` は停止の前に実行されたと読める。値 0 は「3 つとも NULL なので eax = 0 を書いた」（`0x489a059: mov eax,edi`）と矛盾しない。「WaitOnAddress の経路を使わない側に入る」かどうかは、`[0x48f31a0]` を読む側のコードを追っておらず**未確認**（§7.4 の「読める」も読みであって、その側の経路は実行していない）。
- **名前付き停止の import は無い**。停止の API 名は `fle_unsupported` が持たない（VA を出す）ので、IAT スロットから導いた（`AcquireSRWLockExclusive`）。
- ATTACH の import の最後の 1703 件目の後、`f_4899d90` は `0x489a1d2: call r8`（`r8 = [[0x49f47d0]+0x20] = 0x489caa0`）で `f_489caa0` に入り、`0x489cab9` で停止した。この呼び出しは `GetModuleHandleA` を通った**後**の、同じ初期化子（`f_4899d90`）の中である。

### 7.4 停止点の分類

**(i) 未応答の import**（`imports_table` に行が無い 14 名のうちの `KERNEL32!AcquireSRWLockExclusive`。B8。行なし 15 名のうち `GetModuleHandleA` が今回答えられるようになったので、行なしは 14 名）。(ii) TLS、(iii) JIT、(iv) ロケール、(v) その他、のどれでもない。unit 158（quickfontcache_x64.dll の LoadLibraryW）には届いていない。

- **同じ種類の次の停止**: `AcquireSRWLockExclusive`（20 sites、`ReleaseSRWLockExclusive` 27 sites、`ReleaseSRWLockShared` 6、`AcquireSRWLockShared` 3、`WakeConditionVariable` 4、`WakeAllConditionVariable` 1、`SleepConditionVariableSRW` 4。Blend2D の `.text` 全体の `call/jmp [slot]` の数。実測）。これらは `GetModuleHandleA` の次の停止から直接つながる。**直していない**（この作業は `GetModuleHandleA` だけ。他の行なし import は触らない）。
- 同じ DLL の `GetProcAddress` に 3 つとも NULL が返った（決定 0081 の API セットの仮想モジュールに行が無いため）。停止した `f_489caa0` は `AcquireSRWLockExclusive` を使う関数で、この停止が `WaitOnAddress` の NULL に由来するかは**未確認**（`f_489caa0` を呼ぶ `0x489a1d2: call r8` は `GetProcAddress` の結果を見る分岐の後ろの無条件の呼び出しに見える。読み）。実機の Windows では `WaitOnAddress` などは非 NULL のはずで（Learn の Requirements 欄から。未確認）、この環境は決定 0081 と同じ種類の違い（NULL の経路を通る）を持つ。

## 8. 解釈が分かれた点、実装できなかった点

実装できなかった PM の決定の項目は無い。次は決定の文言の解釈を要した点で、決定の内容は変えていない。

1. **非 ASCII の A と W の違い**: PM の決定は「名前の比較は GetModuleHandleW が今行っている規則どおり」と「0x80 以上のバイトは名前付き停止」の両方を書く。W は 0x7F を超える文字を停止せず「見つからない」（126）にするので、この 2 つは A の非 ASCII について**食い違って見える**。決定の表の項目 2（0x80 以上は名前付き停止）を、「ASCII の範囲の比較規則（畳み込み、`.dll`、末尾の `.`）は W と同じ」という意味に読み、非 ASCII だけは A が停止し W は 126 のままにした。W の挙動は変えていない（既存テストのとおり）。
2. **「共通化」の形**: PM は「既存の関数に渡す、または同等の共通化」と書く。W の本体を `static` 関数に移し、W と A が `wide` の 0／1 で呼ぶ形にした。W の停止メッセージの文字列が `GetModuleHandleW with a path is not supported…` と同一であること（既存の `undefined_cases_abort` が検査）を確認した。ANSI を wide の配列に広げて W を呼ぶ形は、W 側の `fle_ld(p + 2n, 2)` が呼び出し元のメモリを読むので、一時配列が要る。同じ結果になるが、`R[1]` を差し替える必要があり、停止メッセージが `GetModuleHandleW` のものになるので採らなかった。
3. **`api` 引数**: `fle_realmod_module_handle(api, rm)` の `api` は `(void)api` で使われていない（`fle_realmod.c`）ので、W の動作は変わらない。A のとき `"GetModuleHandleA"` を渡す。
4. **ページが書かない入力は名前付き停止にする**: 決定の表の項目 4 は「GetModuleHandleW が名前付き停止にする名前は A でも名前付き停止」。W が停止するのはパス（`\`、`/`、`:`）と 256 文字超だけで、**空文字列、`.` は W も 126 を返す（停止しない）**。ページが書かない入力のうち、W が停止しないものは A も停止しない（W と同じ 126）。「ページが書かない入力は名前付き停止にする」と手順 2 が書くのと食い違って見えるが、**決定の表（W と同じにする）を優先した**。W が既に答えている入力に新しい扱いを作らないため。
5. **`/` を区切りとして受けるか**: ページは「バックスラッシュを使え」と書くだけで `/` の扱いを書かない。W は `/` で名前付き停止する（既存）ので、A も同じ（停止）。
6. **ロード中のモジュールの扱い（決定 0082 の保留）**: 新しい扱いを作らなかった。`fle_realmod_module_handle` が DllMain の実行中（`ST_LOADING`）に 0 を返す既存の動作に A も従う（テストで A と W が同じ値を返すことを確認）。
7. **既存テストの期待値**: 「`GetModuleHandleA` は行なし／停止」を表明していたテストは `lift/` に無かった（`grep GetModuleHandleA` が 0 件）。FLEngine の生成 C の SHA を固定していたテストも無い（`grep 43985d62` が `*.rs` に 0 件）。
8. **3 モジュールの FLEngine の C の SHA の指定**: `gen_two.sh` は既定で基準の SHA（`43985d62…`）を期待して落ちる。新しい SHA（`eb9d8e7f…`）を `EXPECT_FLE_SHA` に渡した。スクリプトは変えていない（環境変数）。
9. **gdb スクリプトの行番号**: `ro_imports3.py` の `LoadRet("fle_load.c:153")` は、runtime の行がずれていて `fle_realmod_load` の戻りに当たらなかった（`LOADLIBRARY-RETURN` が 22 回出た）。コピーの `ro_args_gmha.py`、`ro_stop3.py` で `:155` に直した（前回の測定 `out-jtb2d90/gdb/ro_imports.py` と同じ行）。値を書く機構は無い。

## 9. 限界、未確認

- **実機の Windows での `GetModuleHandleA` の挙動は未確認**（ホストは Linux）。失敗時の last error 126 は決定 0015 の PM の決定を W と共有する（ページは値を書かない）。
- 非 ASCII の A の入力は**名前付き停止**で、実機の ANSI コードページでの解釈はしない。Blend2D の引数は ASCII なので、この停止は Blend2D の経路では起きない（実測の引数）。
- `fle_modules[]` に無い実機の DLL 名（例: `psapi`、`ws2_32`）は A も W も 126 を返す（`GetModuleHandleW` と同じ。決定 0015 のとおり、表に無い名前は「ロードされていない」扱い）。**実機では存在する DLL を「無い」と答える**ので、実機の再現ではない（決定 0015 から続く仮定）。
- 実在モジュールの DllMain 実行中の `GetModuleHandle` を「未ロード」とする扱い（決定 0082 の保留）は変えていない。ロード中のモジュールに実機が何を返すかは、ページが書いておらず未確認。
- Blend2D の `GetModuleHandleA` は 1 か所だけ（`.text` を `grep`）。FLEngine、dsp_ippv2 は import していない。他の同梱 DLL（QuickFontCache、Elastique など）の `GetModuleHandleA` は調べていない。
- 次の停止（`AcquireSRWLockExclusive`）以降は未測定。SRWLock、条件変数、`CreateThread`、`TlsFree`、JIT、ロケール、TLS（B6）がどの順に出るかは分からない。
- 3 モジュールの a4 は、gdb の仮の値なし、`GetModuleHandleA` 以外の runtime の応答追加なし。wasm は使っていない（native のみ）。
- 種 58 個、基底 `0x2000000`、`0x4700000` は測定用の値で、決定ではない（決定 0086）。
- `lift/out-gmh/` の作業物は gitignore 対象（`git status` に出ない）。

## 10. 作ったファイル

- 変更: `lift/runtime/fle_modules.c`、`fle_modules.h`、`lift/crates/fe-lift/src/imports_table.rs`、`lift/crates/fe-lift/tests/modules.rs`、`realmod.rs`、`realmod_s1/shared.h`、`guest_host.c`、`guest_mod2.c`、`docs/README.md`（0091 の行）。
- 新規: `docs/decisions/0091-getmodulehandlea.md`、本文書。
- 作業物（`lift/out-gmh/`）: `test-ws.txt`、`test-targeted.txt`、`fle.c`、`fle-diff.txt`、`a4single/`、`a4single-survey-{1,2}.txt`、`three/`、`three-out/`、`three-survey-{1,2}.txt`、`gdb/{ro_args_gmha,ro_stop3}.py`、`run/`、`target/`、`tmp/`。
