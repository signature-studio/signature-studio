# 符号拡張、IDIV、1 オペランド IMUL の後の実行（int-ext-run）

作業: 作業者（M1）。日付: 2026-10-20。成果物: `lift/out-int-ext/`（生成物と実行結果）。決定: `docs/decisions/0010-int-ext.md`。

## 1. 結果の要約

| 項目 | 前回（cpuid-run、0x407360） | 今回（int-ext） |
|---|---|---|
| 停止点 VA | 0x407360 | 0x407360（変わらず） |
| 停止点の関数 | f_407360 | f_407360 |
| abort の種類 | fle_unsupported | fle_unsupported |
| 関数実行数（distinct） | 28 | 28 |
| 関数の entry 数（calls） | 42 | 42 |
| 呼び出し深さ（停止時） | 9 | 9 |

実行ログの差分は、スタックの位置（`rsp=` の値。実行ごとに変わる）だけ。`diff lift/out-cpuid-run/run-896.txt lift/out-int-ext/run-896.txt` の結果は 1 行（rsp）のみ。

停止点は前回から動かなかった。今回追加した命令は、この停止点より前の経路では使われていない（停止点まで到達する経路に、Movsx 等の未対応は残っていない、という推測。実行ログは停止点の前の命令の一覧を含まないので、確認はしていない）。

## 2. 次の停止点

- VA: 0x407360（DLL の `.text+0x6360`）。
- 命令: `jmp *IAT`（import thunk）。IAT スロットは 0x1571658 で、前回の記録（cpuid-run.md、objdump 直読み）では `VerSetConditionMask`。今回は再確認していない。
- 未対応の種類: `Jmp: indirect through IAT slot (not lifted)`（閉包の未対応 435 件。今回の生成でも同じ 435 件）。
- 生成器の振り分けでは、この import は `imports_table.rs` の表にない。

今回の作業では、この thunk は扱っていない。依頼の対象（符号拡張、IDIV、1 オペランド IMUL）の外。また、`VerSetConditionMask` の仕様（戻り値、副作用）を、この作業では確かめていない。

## 3. 未対応の件数（前後、ニーモニック別）

閉包 896 seed（`lift/out-cpuid-run/seeds-896.txt`）。前は `lift/out-cpuid-run/gen-896.txt`、後は `lift/out-int-ext/gen-896.txt`。

| ニーモニック | 前 | 後 | 差 |
|---|---|---|---|
| Jmp: indirect through IAT slot | 435 | 435 | 0 |
| Movsx | 219 | 0 | -219 |
| Idiv | 157 | 0 | -157 |
| Imul（1 オペランド） | 132 | 0 | -132 |
| Cdq | 128 | 0 | -128 |
| Cqo | 26 | 0 | -26 |
| Xadd | 56 | 56 | 0 |
| Bt | 53 | 53 | 0 |
| Cmpxchg | 30 | 30 | 0 |
| Div | 18 | 18 | 0 |

- 合計: unsupported_total 1617 → 955（-662）。662 = 219 + 157 + 132 + 128 + 26。
- 件数の「種類」（ニーモニックの種類数）: 68 → 63（Movsx、Idiv、Cdq、Cqo、Imul の種類がなくなった）。
- handled（新規）: cdq 128、cqo 26、idiv 157、imul1 132、movsx 219。handled の他の件数は変わらない（imul 1217 も同じ）。

依頼文の数字との食い違い（報告）: 依頼文の Movsx 218、Idiv 156、Cdq 607（の一部）、x87 750 は、生成 C の報告（gen-896.txt）と一致しなかった。実測は Movsx 219、Idiv 157、Cdq 128、Cqo 26、Imul 1 オペランド 132。x87 は、未対応の一覧に x87 の件数の行が無い（Fldcw 4、Fnclex 2、Fnstcw 2、Fninit 1、Fnstsw 1 の合計 10）。750 の出所は確認できなかった。

## 4. 生成

- ツール: `lift/out-int-ext/tool/`（新規。`fe-lift` の公開 API `generate_with_seeds` を呼ぶ）。
- コマンド: `int-ext-tool lift binaries/win64/FLEngine_x64.dll lift/out-int-ext/out/after-int-ext-896.c <seeds>`。出力は `create_new`（既存の上書きはしない）。
- 結果: 成功（EXIT=0）。`c_lines` 548103、`c_bytes` 42475128（前は 42415527）。
- 閉包の件数は前と同じ（closure_instructions 497367、functions 8427）。

## 5. ビルドとリンク

- スクリプト: `lift/out-int-ext/build.sh`（新規。`lift/out-cpuid-run/build.sh` と同じフラグ。`fle_int_ext.c` を追加）。
- 結果: clang -O0 のコンパイルと `-pie` のリンク、成功（EXIT=0）。`lift/out-int-ext/bin/flengine-after-int-ext-896` を作成。

## 6. 実行

- コマンド: `lift/out-int-ext/bin/flengine-after-int-ext-896 binaries/win64/FLEngine_x64.dll`。
- 終了コード: 134（SIGABRT）。
- stderr: `fle: unsupported instruction at 0x407360`。
- 出力: `lift/out-int-ext/run-896.txt`、`run-896.err`、`run-896.exit`。

## 7. 検証

- テスト（ホスト突合）: `cargo test --offline -p fe-lift --test int_ext` で 4 件通過（`lift/out-int-ext/test-int-ext-1.txt`）。
- workspace: `cargo test --offline --workspace` が全件通過（EXIT=0。`lift/out-int-ext/test-workspace-1.txt`）。

## 8. 残りの作業量（推測を含む）

- 停止点の 0x407360 の import thunk（IAT 経由の jmp）が、閉包の未対応 435 件の中で最大の種類。次の作業は、このクラスの import 処理（`imports_table.rs` の表への追加、または汎用の扱い）の判断。仕様の確認が要る（PM の判断）。
- 残りの未対応の上位（生成 C の報告より）: Xadd 56、Bt 53、Cmpxchg 30、Div 18（符号なしの除算）、SSE のパック系など。
- 停止点より後の経路は未確認。

## 9. 注意（作業の失敗）

- `lift/runtime/fle_int_ext.c.note.txt` を誤って作成した（中身は "placeholder"）。rm は禁止のため残置。本番の成果物ではない。
- `lift/lift/out-int-ext/tmp`（誤ったパス。`workdir` の取り違え）を作成した。中身は空。rm は禁止のため残置。
- 初回のテスト実行は、出力先ディレクトリが無いために失敗した（EXIT=1。ログは作られていない）。作成後に再実行して成功。
- 既存ファイルの上書きは行っていない。`git` は使っていない。/tmp は使っていない（TMPDIR は `lift/out-int-ext/tmp`）。
