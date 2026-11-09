# VerifyVersionInfoW の判断と計測実行（0x407370 の後）

作業者による実行記録。推測と実測を分けて書く。判断は `docs/decisions/0012-verifyversioninfow.md`、上位 import の仕様は `docs/investigations/import-top-spec.md`。

## 1. 判断

- VerifyVersionInfoW には応答を作らなかった（0012）。戻り値は実行中の OS に依存し、FLEngine が要求する OS 条件は仕様から決まらない。
- 追加した応答: `kernel32!LocalFree` 1 件。仕様（MSDN: LocalFree）から決まる範囲（NULL を無視して NULL を返す、解放して NULL を返す）。`fle_imports.c` に追記、`fle_imports.h` に宣言、対応表に 1 件追加。既存の応答は変えていない。
- 途中の誤り: 最初の実行では LocalFree の判断を「担当外」と書いたが、誤り（`fle_imports.c` は担当範囲）。LocalFree を実装し直した。

## 2. テスト（実測）

- `cargo test --offline --workspace`（作業場所 `lift/`、TMPDIR は `lift/out-verify/tmp`）: EXIT=0。全件通過。最終ログ `lift/out-verify/test-workspace-2.txt`（1 回目 `test-workspace-1.txt` は LocalFree 追加前）。
- 新規 `lift/crates/fe-lift/tests/verify.rs`: 3 件、通過。
  - `verify_version_info_thunk_stays_unsupported`: VerifyVersionInfoW の thunk が `fle_unsupported` のまま。
  - `local_free_thunk_calls_runtime`: 対応表の thunk が `fle_imp_LocalFree(); return;` を呼ぶ。
  - `local_free_runtime_returns_null_and_frees`: C で、NULL の解放が NULL を返し、LocalAlloc の確保を解放して NULL を返すこと。

## 3. 生成（実測）

- 生成器: `lift/out-verify/tool`（新規。`verify-table`）。ビルド EXIT=0（`lift/out-verify/build-tool.txt`、LocalFree 追加後は `build-tool-2.txt`）。
- seed: `lift/out-verify/seeds-896.txt`（既存の `lift/out-bulk/seeds-table-895.txt` の 895 件と 0x41d050 の計 896 件。既存は読むだけ）。
- 閉包（`lift/out-verify/closure-896.txt`）: functions 8427、instructions 497367、import_jumps 445。前回（out-ver）と一致。
- 生成（最終。`lift/out-verify/gen-verify-896-c.txt`）: EXIT=0。出力 `lift/out-verify/out/after-verify-896-c.c`。c_bytes 42,475,268、unsupported_total 951（前回 953）。`jmp_imp:LocalFree 2` が増え、`jmp_imp:VerSetConditionMask 2` は前回どおり。`jmp_imp:VerifyVersionInfoW` は出ない。
- 誤りの生成物（残す。rm は使えない。使わない）: `gen-verify-896-b.txt` と `out/after-verify-896-b.c`。LocalFree 追加後の生成器を再ビルドする前に作ったため、`jmp_imp:LocalFree` が出ていない。`out/after-verify-896.c` は、ビルド前の段階で作った版（使わない）。

## 4. ビルドとリンク（実測）

- `lift/out-verify/build.sh after-verify-896-c`（新規。`lift/out-ver/build.sh` と同じ構成で、`fle_cpuid.c` と `fle_int_ext.c` を含む）。
- 結果: EXIT=0（`lift/out-verify/build-896-c.txt`）。`lift/out-verify/bin/flengine-after-verify-896-c`。
- 前回の `out-ver` で 1 回目に起きた、リンクの失敗（fle_cpuid、fle_int_ext の抜け）は、今回は起きなかった。

## 5. 実行（実測）

コマンド: `./bin/flengine-after-verify-896-c ../../binaries/win64/FLEngine_x64.dll`（TMPDIR は `lift/out-verify/tmp`）。出力 `lift/out-verify/run-verify-896-c.txt`。EXIT=134（SIGABRT）。LocalFree 追加前の実行（`run-verify-896.txt`）も、同じ停止点。

- 停止点 VA: 0x407370
- 関数: f_407370
- 命令: `jmp *0x156a2aa(%rip)`（objdump の 10376 行、`# 0x1971620`）
- IAT の名前: `kernel32.dll!VerifyVersionInfoW`（`lift/out-imports/imports.txt` の 393 行で確認）
- abort の種類: `fle_unsupported`（runtime message: `fle: unsupported instruction at 0x407370`）
- 関数実行数（distinct）: 29
- 関数の呼び出し（entries）: 45
- 呼び出し深さ: 9
- LocalFree 追加後（`run-verify-896-c.txt`）も、同じ停止点・同じ数字。

呼び出しスタック（実測。前回と同じ）:

```
#0 0x407370 f_407370 guest-return=0x409bb0
#1 0x409b30 f_409b30 guest-return=0x409be6
#2 0x409bd0 f_409bd0 guest-return=0x409bf9
#3 0x409bf0 f_409bf0 guest-return=0x41ce3c
#4 0x41ce10 f_41ce10 guest-return=0x412129
#5 0x4120c0 f_4120c0 guest-return=0x41229a
#6 0x412160 f_412160 guest-return=0x41d989
#7 0x41d8f0 f_41d8f0 guest-return=0x1376875
#8 0x1376820 f_1376820 guest-return=0xdead0000
```

## 6. 前回との比較（実測）

| | 前回（ver、`lift/out-ver/run-ver-896-b.txt`） | 今回（verify、`lift/out-verify/run-verify-896.txt`） |
|---|---|---|
| 停止点 VA | 0x407370 | 0x407370 |
| IAT の名前 | VerifyVersionInfoW | VerifyVersionInfoW |
| abort | fle_unsupported（SIGABRT） | fle_unsupported（SIGABRT） |
| 関数実行数（distinct） | 29 | 29 |
| 関数の呼び出し（entries） | 45 | 45 |
| 呼び出し深さ | 9 | 9 |

前回と同じ結果は、応答を作らないという判断どおり。新しい情報は増えていない。

## 7. 推測（実測ではない）

- 0x409b30 の引数が、dwMajorVersion（cx=6）、dwMinorVersion（dx=1）、wServicePackMajor（r8w=1）に見えること。0x409bd0 の読みで、関数名は未確認。
- 0x409bf0 が VerifyVersionInfoW の結果（BOOL）を 0/1 で使うこと。呼び出し元の判断は未確認。
- 仮に条件が「Windows 7 SP1 以上」だとしても、それは読みにすぎない（未確認）。

## 8. 残りの作業量（推測）

- 次の停止点 0x407370（VerifyVersionInfoW）は、OS 条件の判断が要る。依頼者（PM）の判断待ち。
- 0x409b30 の後続（0x41ce10 の呼び出し先、0x40f470 と 0x40f490 の繰り返し、0x40d1f0 など）は、今回の実行では確認できていない。
- 件数 3 と 2 の残りの import の仕様（`import-top-spec.md` の「未確認」）は、未確認のまま。

## 9. 失敗・拒否された操作

- 権限拒否: なし。
- git 操作: なし（status、log、diff を含む）。
- rm: 使っていない。既存のファイルの上書きもしていない（新規ファイル名のみ）。
- /tmp: 使っていない。TMPDIR は `lift/out-verify/tmp`。sleep と timeout は使っていない。
- 404 の URL: CompareStringW（winnls 側）、SetCurrentDirectoryW（winbase、processenv、ANSI 版）。別の URL で CompareStringW は確認できた。SetCurrentDirectoryW は確認できず。
- 誤りの修正: 最初に書いた `import-top-spec.md` の CompareStringW の戻り値（CSTR_* の数値）を、ページに書かれていない内容だったので削除した。件数の数え方も直した。

## 10. 検証の範囲

- テスト、生成、閉包の件数、ビルド、実行の数字は実測（最終版は `*-c` の系列）。
- LocalFree の実装は、C の単体テストで NULL と LocalAlloc 由来の解放を確かめた。Windows 本体の挙動は未確認（Linux のため）。
- 件数 3 と 2 の仕様は一部のみ確認（`import-top-spec.md`）。
- Windows 本体の VerifyVersionInfoW の挙動は、ホストが Linux のため確認していない。
- 0x407370 より後の挙動は、この実行では得られていない（停止するため）。
