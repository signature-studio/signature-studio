# 仮想モジュール表の導入と計測実行（0x407060 の後）

作業者による実行記録。推測と実測を分けて書く。判断は `docs/decisions/0015-virtual-modules.md`。前回は `docs/investigations/osver-run.md`（停止点 0x407060、GetModuleHandleW）。

## 1. 実装（実測）

- 新規: `lift/runtime/fle_modules.h`、`lift/runtime/fle_modules.c`（仮想モジュール表 25 件、`fle_imp_GetModuleHandleW`、`fle_imp_GetProcAddress`）。
- 追記: `lift/crates/fe-lift/src/imports_table.rs`（対応表 11 件 → 13 件。GetModuleHandleW、GetProcAddress。応答表と擬似 VA を出す関数）、`lift/crates/fe-lift/src/lib.rs`（`Stats.resp_entries`、`dispatch::table` の呼び出しの分岐）。`fle_imports.c`、`fle_imports.h` は変更なし（`fle_set_last_error` が既にある）。
- `fle_modules.c` は `-Wall -Wextra` で警告 0（`build-896.txt` と、テストの clang 呼び出しで確認）。

## 2. テスト（実測）

- 新規 `lift/crates/fe-lift/tests/modules.rs`: 12 件、全件通過（`lift/out-modules/test-modules-2.txt`、EXIT=0）。1 回目（`test-modules-1.txt`）は 1 件失敗: 期待文字列に行番号（0x…0000）を誤って書いたため。期待値を直して 2 回目で通過。
  - 生成器: `thunks_call_runtime_and_tables_are_emitted`、`dispatch_table_stays_sorted_and_pseudo_vas_are_unique`、`tables_absent_without_get_proc_address`、`generated_unit_with_tables_compiles_and_links_with_runtime`（生成 C に表が出て、GetModuleHandleW → GetProcAddress → `fle_dispatch` で GetLastError が呼ばれて 77 を返す。応答表にない関数は NULL と 127）。
  - GetModuleHandleW: `get_module_handle_name_rules`（24 行の名前の規則と NULL）、`module_handles_follow_the_formula_and_are_distinct`、`every_pe_import_dll_is_a_module`（実 DLL の 24 DLL。DLL がなければ何もせず通る）。
  - GetProcAddress: `get_proc_address_found_and_not_found`（12 行）、`get_proc_address_result_is_called_through_dispatch`、`get_proc_address_without_a_table_finds_nothing`。
  - 未対応: `undefined_cases_abort`（12 ケース: 序数 2、NULL 名、DLL 自身のハンドル、NULL、不正なハンドル、整列しないハンドル、表の外のハンドル、長い名前 2、パス 2 種）。全て SIGABRT で止まり戻り値を返さない。
  - 実 DLL: `real_dll_emits_response_table`（応答表 13 件。`jmp_imp:GetModuleHandleW` と `jmp_imp:GetProcAddress` が出る）。
- `cargo test --offline --workspace --no-fail-fast`: **全件通過**、EXIT=0（`lift/out-modules/test-workspace-1.txt`）。前回報告の `verify.rs` の失敗は、`verify.rs` が既に更新されており（`verify_version_info_thunk_is_handled_since_decision_0014`）、今回は出ていない。
- テストの実行結果は、cargo のターゲットを `lift/out-modules/target`、TMPDIR を `lift/out-modules/tmp` に向けた。

## 3. 生成、clang、リンク（実測）

- 生成器: `lift/out-modules/tool`（`out-osver/tool` の複製。新規）。ビルド EXIT=0（`build-tool.txt`）。seed は `lift/out-modules/seeds-896.txt`（`out-osver/seeds-896.txt` の複製、896 件）。
- 生成: EXIT=0（`gen-modules-896.txt`）。出力 `lift/out-modules/out/after-modules-896.c`（42,507,136 バイト）。
  - functions 8427、closure_instructions 497367（前回と同じ）。c_lines 548148（前回 548103、+45）、c_bytes 42,507,136（前回 42,504,492）。
  - `jmp_imp:GetModuleHandleW 2`、`jmp_imp:GetProcAddress 3` が出た（`osver-run.md` の閉包の見積りと一致）。
  - 応答表の件数: `fle_resp_count = 13`、`fle_dispatch_count = 8440`（関数 8427 + 擬似 VA 13）。
  - unsupported_total 667（前回 672、−5）。`Jmp: indirect through IAT slot (not lifted)` が 425 → 420（−5 = 2 + 3 か所）。他の分類は変わらない。
- clang（-O0）とリンク: `lift/out-modules/build.sh`（`out-osver/build.sh` の複製に `fle_modules` を追加。新規）。`EXTRA_RT="fle_int_more fle_sse_more"` で EXIT=0（`build-896.txt`）。`lift/out-modules/bin/flengine-after-modules-896`。

## 4. 実行（実測）

コマンド: `./bin/flengine-after-modules-896 ../../binaries/win64/FLEngine_x64.dll`（TMPDIR は `lift/out-modules/tmp`）。出力 `lift/out-modules/run-modules-896.txt`、`run-modules-896.err`。EXIT=134（SIGABRT）。

- 停止点 VA: 0x407120
- 関数: f_407120
- 命令: `jmp *0x156a6da(%rip)  # 0x1971800`（`binaries/win64/FLEngine_x64.dll` の objdump）
- IAT の名前: `kernel32.dll!SetThreadLocale`（`lift/out-imports/imports.txt`、IAT 0x1971800、normal）
- abort の種類: `fle_unsupported`（`fle: unsupported instruction at 0x407120`。生成物 17061 行 `L_407120: { /* 0x407120 Jmp */ fle_unsupported(UINT64_C(0x407120)); }`。分類は「Jmp: indirect through IAT slot (not lifted)」）
- 関数実行数（distinct）: 35
- 関数の呼び出し（entries）: 51
- 呼び出し深さ: 6

```
#0 0x407120 f_407120 guest-return=0x41ce4b
#1 0x41ce10 f_41ce10 guest-return=0x412129
#2 0x4120c0 f_4120c0 guest-return=0x41229a
#3 0x412160 f_412160 guest-return=0x41d989
#4 0x41d8f0 f_41d8f0 guest-return=0x1376875
#5 0x1376820 f_1376820 guest-return=0xdead0000
```

### 前回（0x407060、GetModuleHandleW）との比較

| 項目 | 前回（osver-run.md） | 今回 |
|---|---|---|
| 停止点 | 0x407060 f_407060（kernel32!GetModuleHandleW） | 0x407120 f_407120（kernel32!SetThreadLocale） |
| abort の種類 | fle_unsupported | fle_unsupported |
| 関数実行数（distinct） | 31 | 35 |
| 関数の呼び出し | 47 | 51 |
| 呼び出し深さ | 8 | 6 |

- 実測で言えること（entry の並び）: entry 46（f_407060 = GetModuleHandleW、guest-return 0x4099e9）と entry 47（f_407070 = GetProcAddress、guest-return 0x4099f8）は返った。entry 48（f_4090e0、guest-return 0x41ce41）が現れ、f_4099c0 は返っている（f_409bf0 の `call 0x4099c0` の後）。entry 49（f_409070、guest-return 0x4091c5）に続き entry 50 が停止点。
- 読み（推測を含む）: f_4099c0 は GetProcAddress が NULL（`GetLogicalProcessorInformation` は応答表にないため。実測: 応答表 13 件にない）を受けて 0x4099fb の `je 0x409aae` で戻ったと読める。分岐そのものはトレースしていない。0x407320（遅延 import の GetLogicalProcessorInformation の thunk）が entry に現れていないことが、その読みと矛盾しない。
- 停止点は 0x407060 → 0x407120 に進んだ。この間の 2 つの import（GetModuleHandleW、GetProcAddress）が、本作業で応答するようになった。深さが 8 → 6 になったのは、f_409bf0 と f_4099c0 などが返り、呼び出し元 f_41ce10 に戻って次の呼び出し（0x41ce46）に進んだため（スタックの並びからの読み）。
- 今回の生成器には、他のサブエージェントの変更（`int_more`、`sse_more`、emulated OS）も含まれる（前回と同じ状態。今回は閉包の関数数も命令数も前回と同じ）。停止点の移動は、本作業の 2 件によるもの（前回の停止点 0x407060 を通過した、が実測）。

## 5. 次の停止点（0x407120）

- 命令: `jmp *0x1971800`（`ff 25 da a6 56 01`）= `kernel32.dll!SetThreadLocale`（normal な IAT）。
- 呼び出し元: f_41ce10 の 0x41ce46（`call 0x407120`）。直前の 0x41ce41 は `mov $0x400,%ecx`（objdump）。rcx = 0x400 を渡す。引数の意味は、SetThreadLocale の仕様を読んでいないので確認していない（推測: LCID。0x400 は LOCALE_USER_DEFAULT かもしれない。未確認）。戻りの eax は、続く 0x41ce4b の `call 0x418e10` の前に使われない（0x41ce4b は `call`、その前に eax を読む命令はない。objdump の読み）。
- 未対応の種類: IAT 経由の `jmp`（対応表にない import）。命令の未実装ではない。
- 方針の見立て（推測）: SetThreadLocale は、呼び出しスレッドのロケールを変える関数で、戻り値は BOOL。呼び出しの後の動作が環境のロケールに依存するか、戻り値が使われるかは未確認。仕様ページを読んでから判断する必要がある（この作業では読んでいない）。

## 6. 残りの作業量（推測を含む）

- 停止点は 1 つ進んだだけで、「フル lift」には届いていない。生成物の未対応は 667 か所（うち IAT 経由の jmp 420、間接 jmp 解決不能 20、他は未実装の SSE 命令など）。
- IAT 経由の jmp の 420 か所は、import 数にして kernel32 だけでも 184 個（normal）。応答を作った API は 13 個。今回のように 1 つ作って次の停止点まで進む、を API ごとに繰り返す必要がある（実測の進み方からの見積り。API ごとの仕様確認が要るので、残りの API の数だけ未決がある）。
- GetProcAddress の応答表は、応答を作る API が増えるたびに行が増える。行が増えると NULL だった API が非 NULL に変わり、ゲストの経路が変わり得る（0015 の §6）。

## 7. 実測として言えないこと

- Windows 本体の GetModuleHandleW、GetProcAddress の戻り値との一致は、ホストが Linux のため未確認。
- GetProcAddress が NULL を返した後の f_4099c0 の分岐は、トレースしていない（上の読み）。
- SetThreadLocale の仕様は読んでいない。

## 8. 残置物

- `lift/out-modules/test-modules-1.txt`: テストの 1 回目の失敗ログ（期待値の誤記）。
- `lift/out-modules/target`、`target-tool`: cargo のビルド出力（削除禁止のため残す）。
