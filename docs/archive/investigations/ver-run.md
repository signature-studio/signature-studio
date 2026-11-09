# VerSetConditionMask の計測実行（0x407360 の後）

作業者による実行記録。推測と実測を分けて書く。判断は `docs/decisions/0011-verSetConditionMask.md`、API の出典は `docs/investigations/import-VerSetConditionMask.md`。

## 1. 追加した応答（実装）

- `lift/runtime/fle_imports.c` に `fle_imp_VerSetConditionMask` を追記。`fle_imports.h` に宣言を 1 件追記。既存の関数は変えていない。
- 規則: `ConditionMask | ((Condition & 7) << 3k)`。k は TypeMask の最上位フラグのビット位置。規則の出典は Wine（非公式）。公式ページはビット配置を書いていない（decisions/0011 参照）。
- 対応表 `lift/crates/fe-lift/src/imports_table.rs` に `kernel32.dll / VerSetConditionMask` を 1 件追加。

## 2. テスト（実測）

- `cargo test --offline --workspace`（作業場所 `lift/`、TMPDIR は `lift/out-ver/tmp`）: EXIT=0。全件通過。ログ `lift/out-ver/test-workspace-1.txt`。
- 新規 `lift/crates/fe-lift/tests/ver.rs`: 2 件通過。
  - thunk が応答を呼ぶこと、VerifyVersionInfoW は未対応のまま。
  - 応答の値: 12 件。期待値は手で計算した式の結果（`Condition << 3k`）。Rust から clang で C をビルドして比較。

## 3. 生成（実測）

- 生成器: `lift/out-ver/tool`（新規。`lift/out-bulk/tool/src/main.rs` の写し。fe-lift を path で参照）。ビルド `lift/out-ver/target/release/ver-table`（EXIT=0）。
- 入力の seed: `lift/out-bulk/seeds-table-895.txt` の 895 件と既存 0x41d050 の計 896 件（前回と同じ集合）。
- 閉包の確認: `closure` 実行で functions 8427、instructions 497367、import_jumps 445。前回（closure-896.txt）と同じ。
- 生成: EXIT=0。出力 `lift/out-ver/out/after-ver-896-b.c`（42,475,218 bytes、c_lines 548,103）。`jmp_imp:VerSetConditionMask 2`。0x407360 は `{ void fle_imp_VerSetConditionMask(void); fle_imp_VerSetConditionMask(); return; }` に変わった。
- 注意: `lift/out-ver/out/after-ver-896.c` は、ツールのビルドより前（古い fe-lift）で作った生成物で、0x407360 が未対応のまま。誤りの生成物だが rm 禁止のため残す。使わない。

## 4. ビルドとリンク（実測）

- `lift/out-ver/build.sh after-ver-896-b`: EXIT=0。`lift/out-ver/bin/flengine-after-ver-896-b`（44,831,272 bytes）。
- 1 回目は失敗（EXIT=1）。原因: build.sh に `fle_cpuid.c` と `fle_int_ext.c` が入っていなかった（undefined reference: fle_cpuid、fle_idiv、fle_imul1）。`lift/out-int-ext/build.sh` と同じ構成に直して再ビルドした。
- 1 回目の失敗の直後、ログを読み違えて古いログを見たが、再ビルドのログ（`build-896-b-2.txt`）で確認した。

## 5. 実行（実測）

コマンド: `./bin/flengine-after-ver-896-b binaries/win64/FLEngine_x64.dll`（TMPDIR は `lift/out-ver/tmp`）。出力 `lift/out-ver/run-ver-896-b.txt`。EXIT=134（SIGABRT）。

- 停止点 VA: 0x407370
- 関数: f_407370
- 命令: `jmp *0x156a2aa(%rip)`（objdump、`# 0x1971620`）
- IAT の名前: `kernel32.dll!VerifyVersionInfoW`（imports.txt 393 行）
- abort の種類: `fle_unsupported`（SIGABRT、終了コード 134）
- 関数実行数: 29（distinct）
- 関数の呼び出し（entries）: 45
- 呼び出し深さ: 9

呼び出しスタック（実測）:

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

entry log の末尾（実測）: entry 41、42、43 が `f_407360`（戻り先 0x409b6c、0x409b7c、0x409b8c）、entry 44 が `f_407370`（戻り先 0x409bb0）。0x407360 の 3 回の呼び出しは応答が返り、戻り先（0x409b6c、0x409b7c、0x409b8c）へ戻っている。

## 6. 前回との比較（実測）

| | 前回（int-ext、`lift/out-int-ext/run-896.txt`） | 今回（ver） |
|---|---|---|
| 停止点 VA | 0x407360 | 0x407370 |
| IAT の名前 | VerSetConditionMask | VerifyVersionInfoW |
| 関数実行数（distinct） | 28 | 29 |
| 関数の呼び出し（entries） | 42 | 45 |
| 呼び出し深さ | 9 | 9 |
| abort | fle_unsupported | fle_unsupported |

依頼文の「前回 0x407360 で、関数実行数 22、深さ 8」は、`out-bulk/run-896.txt`（0x40f47b で停止）とは一致しない。比較は int-ext の記録（0x407360、28、9）に合わせた。

## 7. 推測（実測ではない）

- f_409b30 の引数が、dwMajorVersion（cx=6）、dwMinorVersion（dx=1）、wServicePackMajor（r8w=1）に見えること。0x409bd0 の objdump からの読み（関数名は未確認）。
- 0x409bf0 が VerifyVersionInfoW の結果（BOOL）を 0/1 で使うこと。呼び出し元の判断は未確認。
- 応答の規則（Wine の配置）が Windows 本体と一致すること。ホストが Linux のため、Windows の実行はできない。未確認。

## 8. 残りの作業量（推測）

- 次の停止点 0x407370（VerifyVersionInfoW）は依頼により対応しない。ここが次の判断点。
- 0x409b30 の後続、0x41ce10 の呼び出し先（0x40f470、0x40f490 の繰り返し、0x40d1f0 など）の動作は、今回の実行では確認できていない。

## 9. 失敗・拒否された操作

- 権限拒否は無し。
- git 操作は行っていない（status、log、diff も含めて）。rm は使っていない。既存のファイルの上書きは行っていない（新規ファイル名を使った）。
- /tmp は使っていない。TMPDIR は `lift/out-ver/tmp`。sleep や timeout は使っていない。
- 誤り 1: 最初の `mkdir -p lift/out-ver/tmp` を作業場所 `lift/` で実行したため、`lift/lift/out-ver/tmp` が作られた。正しい場所に作り直した。誤って作られた `lift/lift/out-ver/` は、残置物の扱いと rm 禁止のため削除していない（残してある）。
- 誤り 2: cargo を作業場所 `lift/` の外（リポジトリのルート）で実行して失敗した。`lift/` で再実行して解決。
- 誤り 3: 古いツールの生成物（`after-ver-896.c`）を作った。ツールを新しくビルドし直して、`after-ver-896-b.c` を作った。古い生成物は残してある。
- 誤り 4: 1 回目のリンクで `fle_cpuid.c` と `fle_int_ext.c` を抜かした。2 回目で修正。
- 誤り 5: 古いログ（`build-896-b.txt`）を読んで、リンクが失敗したと誤認した。再ビルドのログ `build-896-b-2.txt` で確認した。
- 誤り 6: 調査の文書で IAT のアドレスの説明を誤った。直した。

## 10. 検証の範囲

- テスト、生成、閉包の件数、ビルド、実行の数字は実測。
- Windows 本体の VerSetConditionMask との一致は未確認（Linux のため）。
- 公式ページがビット配置を書いていないこと（decisions/0011、import-VerSetConditionMask.md）。
