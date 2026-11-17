# エミュレート OS の導入と計測実行（0x407370 の後）

作業者による実行記録。推測と実測を分けて書く。判断は `docs/decisions/0014-emulated-os.md`。前回は `docs/investigations/verify-run.md`（停止点 0x407370）。

## 1. 実装（実測）

- 新規: `lift/runtime/fle_osver.h`、`lift/runtime/fle_osver.c`（エミュレート OS `fle_emulated_os` = 10.0.19045、VerifyVersionInfoW、GetVersion、GetVersionExW）。
- 追記: `lift/runtime/fle_imports.c` に `fle_set_last_error`（既存の `fle_last_error` を他ファイルから設定する setter。スレッド検査は既存の `fle_tls_check_thread` と同じ）、`fle_imports.h` に宣言、`imports_table.rs` に 3 件（VerifyVersionInfoW、GetVersion、GetVersionExW。対応表は 8 件から 11 件）。
- `-Wall -Wextra` で `fle_osver.c`、`fle_imports.c` の警告 0 を確認。

## 2. テスト（実測）

新規 `lift/crates/fe-lift/tests/osver.rs`: 6 件、全件通過（`lift/out-osver/test-osver-3.txt`、EXIT=0）。

- `thunks_call_runtime`: 3 つの thunk が `fle_imp_*(); return;` を呼ぶ。
- `verify_version_info_matches_spec_rules_on_emulated_os`: 65 ケース（C 側が `fle_imp_VerifyVersionInfoW` と `fle_osver_check` の両方で確認し、last error が成功で 0、失敗で 1150 であることも確認）。期待値は、仕様の規則を Rust の式（`expect`、`cmp`、`mask`）で別に書いたもの。含むもの: major >= 9 は満たす、major >= 11 は満たさない、major の 5 条件、build（19044、19045、19046 と 5 条件）、platform、product type、suite の AND/OR、4 メンバーの階層比較（>=）、上位が EQUAL で下位が別条件の階層、f_409b30 の条件（TypeMask 2、1、0x20 の GREATER_EQUAL。dwTypeMask 0x23）、複数の要素の組み合わせ（major + build + product）。
- `hierarchy_examples_from_remarks`: Remarks の 3 例を合成 OS で確認（5.1 SP1 以上が、6.0 で成功、5.2 で成功、5.0 SP2 で失敗。加えて 5.1 SP1 で成功、5.1 SP0 で失敗）。
- `get_version_and_get_version_ex`: GetVersion = 0x4A65000A。GetVersionExW が OSVERSIONINFOW（276）と OSVERSIONINFOEXW（284）を size で判別して書く。OSVERSIONINFOW のとき後続バイトに触れない、EXW の wReserved に触れない、szCSDVersion が空、不正な size（100）で 0 を返し構造体に触れず last error 87。
- `undefined_cases_abort`: 9 ケース（dwTypeMask 0、0xFF 外のビット、条件 0、major に条件 6、suite に条件 3、suite の空マスク、dwOSVersionInfoSize 不正、NULL、major GREATER と minor LESS_EQUAL）が SIGABRT で止まり、戻り値を返さない。
- `emulated_os_defined_once`: エミュレート OS の定数が `fle_osver.c` の 1 か所にあり、他のランタイムファイルに 19045 がない。

## 3. `cargo test --offline --workspace`（実測）: 1 件失敗（担当外のファイル）

`lift/out-osver/test-workspace-2.txt`（`--no-fail-fast`）: 117 件中 116 件通過、1 件失敗、EXIT=101。

- 失敗: `lift/crates/fe-lift/tests/verify.rs` の `verify_version_info_thunk_stays_unsupported`。0012 の決定（VerifyVersionInfoW は未対応のまま）を確認するテストで、`assert!(!st.handled.contains_key("jmp_imp:VerifyVersionInfoW"))` が、VerifyVersionInfoW を対応表に入れたため成り立たない。**この決定（0014）によって意図どおりに成り立たなくなったテスト**で、更新が必要。
- このファイルは担当ファイルに含まれていないため、編集していない（「これ以外は触らない」の制約）。PM の指示が要る。想定する更新: 当該テストを「thunk が `fle_imp_VerifyVersionInfoW` を呼ぶ」に変える（または osver.rs の `thunks_call_runtime` に置き換えて削除する）。同じファイル内の LocalFree の 2 件は通過している。
- 他の全件（別のサブエージェントの `int_more`、`sse_more` を含む）は通過。
- 補足（推測ではなく確認した事実）: `tests/ver.rs` の `table_thunk_calls_runtime_and_others_stay_unsupported` は通過している。ただしこれは、その fixture が `VerifyVersionInfoW` の名前を 0x2430 に書いたあと 0x2440 に `Fn` を書いて名前を上書きしており、IAT の名前が `VerifyVersionI` になっているため（Python で再現して確認）。VerifyVersionInfoW の thunk としては効いていない。このテストも担当外で、編集していない。

## 4. 生成、clang、リンク（実測）

- 生成器: `lift/out-osver/tool`（`lift/out-verify/tool` の複製。新規の名前で作成。cargo の TMPDIR は `lift/out-osver/tmp`）。ビルド EXIT=0（`build-tool.txt`）。
- seed: `lift/out-osver/seeds-896.txt`（`out-verify/seeds-896.txt` の複製、896 件）。
- 生成: EXIT=0（`gen-osver-896.txt`）。出力 `lift/out-osver/out/after-osver-896.c`。functions 8427、closure_instructions 497367、c_lines 548103、c_bytes 42,504,492（前回 42,475,268）。`jmp_imp:VerifyVersionInfoW 2`、`jmp_imp:GetVersion 2`、`jmp_imp:GetVersionExW 2` が出た（閉包の見積りと一致）。
- unsupported_total 672（前回 951）。**この差の大部分は、別のサブエージェントの `int_more.rs` と `sse_more.rs`（担当外、読み取りのみ）が生成器に入っているため**。今回の対応表の追加（3 件、6 か所）だけで 279 減ったわけではない。
- clang と全体リンク: `lift/out-osver/build.sh`（新規。`out-verify/build.sh` と同じ旗に、`fle_osver.c` と `EXTRA_RT` を追加）。
  - 1 回目（`build-896.txt`、EXIT=1）: リンクに失敗。未定義参照 `fle_div`、`fle_bt_cf`、`fle_addps`、`fle_mulps`、`fle_maxps`、`fle_minps`、`fle_subps`、`fle_packuswb`、`fle_paddw`。依頼のリンク一覧（fle_cpuid、fle_int_ext、fle_string、fle_mxcsr、fle_imports、fle_osver）には含まれない `lift/runtime/fle_int_more.c` と `fle_sse_more.c`（別のサブエージェントの新規ファイル）に定義がある。
  - 2 回目（`build-896-b.txt`、EXIT=0）: `EXTRA_RT="fle_int_more fle_sse_more"` を足してリンク成功。`lift/out-osver/bin/flengine-after-osver-896`。2 つのファイルは読むだけで、変更していない。

## 5. 実行（実測）

コマンド: `./bin/flengine-after-osver-896 ../../binaries/win64/FLEngine_x64.dll`（TMPDIR は `lift/out-osver/tmp`）。出力 `lift/out-osver/run-osver-896.txt`。EXIT=134（SIGABRT）。

- 停止点 VA: 0x407060
- 関数: f_407060
- 命令: `jmp *0x156a302(%rip)  # 0x1971368`（`binaries/win64/FLEngine_x64.dll` の objdump）
- IAT の名前: `kernel32.dll!GetModuleHandleW`（`lift/out-imports/imports.txt` の 306 行）
- abort の種類: `fle_unsupported`（runtime message: `fle: unsupported instruction at 0x407060`。生成物では `L_407060: { /* 0x407060 Jmp */ fle_unsupported(UINT64_C(0x407060)); }`。生成器の分類は「Jmp: indirect through IAT slot (not lifted)」）
- 関数実行数（distinct）: 31
- 関数の呼び出し（entries）: 47
- 呼び出し深さ: 8

呼び出しスタック:

```
#0 0x407060 f_407060 guest-return=0x4099e9
#1 0x4099c0 f_4099c0 guest-return=0x409c1c
#2 0x409bf0 f_409bf0 guest-return=0x41ce3c
#3 0x41ce10 f_41ce10 guest-return=0x412129
#4 0x4120c0 f_4120c0 guest-return=0x41229a
#5 0x412160 f_412160 guest-return=0x41d989
#6 0x41d8f0 f_41d8f0 guest-return=0x1376875
#7 0x1376820 f_1376820 guest-return=0xdead0000
```

### 前回（0x407370、VerifyVersionInfoW）との比較

| 項目 | 前回（verify-run.md） | 今回 |
|---|---|---|
| 停止点 | 0x407370 f_407370（kernel32!VerifyVersionInfoW） | 0x407060 f_407060（kernel32!GetModuleHandleW） |
| abort の種類 | fle_unsupported | fle_unsupported |
| 関数実行数（distinct） | 29 | 31 |
| 関数の呼び出し | 45 | 47 |
| 呼び出し深さ | 9 | 8 |

- 実測で言えること: VerifyVersionInfoW（entry 44、`f_407370`、guest-return 0x409bb0）は返った。続いて entry 45 で `f_4099c0` が 0x409c1c（`f_409bf0` の `call 0x4099c0` の次）を戻り先として呼ばれた。したがって `f_409bf0` の 0x409bfb の `je 0x409c31`（`test %al,%al`）は分岐せず、`f_409bd0` から `f_409b30` を通った戻り値 al は非 0 になった（実行の経路から確認）。
- 前回とのスタックの違い: 前回 #0 は 0x407370 から f_409b30、f_409bd0、f_409bf0 と並んだ。今回は f_409bf0 の下で f_4099c0 に入り、そこから 0x407060 を呼んでいる。深さが 9 から 8 になったのは、f_409b30 と f_409bd0 のフレームが戻ったため。
- 注意: 今回の生成器には、別のサブエージェントの変更（`int_more`、`sse_more`）も含まれる。停止点の移動は、本作業の 3 件によるものか、他の変更を含むかの切り分けはしていない。ただし、前回の停止点 0x407370 を通過したことと、その呼び出しの経路は、上の実測で確認できる。

## 6. 次の停止点

- 停止点: 0x407060、`jmp *0x156a302(%rip)  # 0x1971368` = `kernel32.dll!GetModuleHandleW`。呼び出し元は f_4099c0（0x4099e4）で、rcx に 0x409ae8 を渡す。その中身は UTF-16 の `kernel32.dll`（バイナリを Python で読んで確認）。続く 0x4099f3 の `call 0x407070` は `jmp *0x1971600` = `kernel32.dll!GetProcAddress`（imports.txt の 389 行）で、名前は `GetLogicalProcessorInformation`（0x409b02、ASCII。バイナリを読んで確認）。つまりこの関数は、kernel32 から GetLogicalProcessorInformation の有無を調べている（推測を含む。0x4099f8 以降の使い方は読んでいない）。
- 未対応の種類: IAT 経由の `jmp`（対応表にない import）。命令の未実装ではない。
- 方針の状況（import-top-spec.md）: GetModuleHandleW と GetProcAddress は「保留」（ハンドルとアドレスをどう定めるかが要る。ランタイムにモジュールのローダーがない）。今回は実装していない。依頼の指示（「次の停止点が出たら報告する。繰り返さない」）に従う。

## 7. 実測として言えないこと

- Windows 本体の VerifyVersionInfoW、GetVersion、GetVersionExW の戻り値との一致は、ホストが Linux のため未確認。
- 条件マスクのビット配置が Windows 本体と一致するかは、0011 のとおり未確認。
- FL Studio の動作要件とこの DLL の要件が一致するかは未確認（0014）。
- 前回との差のうち、他のサブエージェントの変更の影響は切り分けていない。

## 8. 残置物（削除禁止のため残している）

- `lift/runtime/fle_imports.h.new`: 私が作った中間ファイル（`fle_imports.h` に追記する前の試作の出力）。使っていない。内容は `fle_imports.h` の追記後の版と同じ。削除は禁止されているため残した。指示があれば PM 側で消してよい。
- `lift/out-osver/build-896.txt`: リンク失敗の 1 回目のログ（2 回目は `build-896-b.txt`）。
- `lift/out-osver/test-osver-1.txt`、`test-osver-2.txt`: テストの 1 回目（コンパイルエラー）、2 回目（期待値の誤記 2 か所で失敗。GetVersion の 19045 = 0x4A65 の桁と、ケース数の下限 100 に対し実数 65。1 回目の失敗時の panic 出力 `65` による。下限は 60 に直した）。3 回目で通過。
- `lift/out-osver/test-workspace-1.txt`: `--no-fail-fast` なしの実行（verify.rs の失敗で打ち切り）。
