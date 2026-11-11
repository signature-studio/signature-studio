# CPUID の応答を入れた後の実行（cpuid-run）

作業: 作業者（M1）。日付: 2026-10-20。成果物: `lift/out-cpuid-run/`（生成物と実行結果）、`lift/out-cpuid/`（計測用の複製と probe）。決定: `docs/decisions/0009-cpuid.md`。

## 1. 結果の要約

| 項目 | 前回（0x40f47b） | 今回 |
|---|---|---|
| 停止点 VA | 0x40f47b（cpuid） | 0x407360（import thunk、`jmp [IAT]`） |
| 停止点の関数 | f_40f470 | f_407360 |
| abort の種類 | fle_unsupported | fle_unsupported |
| 関数実行数（distinct） | 22 | 28 |
| 関数の entry 数（calls） | 22 | 42 |
| 呼び出し深さ（停止時） | 8 | 9 |

前回の停止点は解消した。cpuid の 8 つの葉（0〜7）を取得して先へ進み、次の未対応の import thunk で止まった。

## 2. 次の停止点

- VA: 0x407360（DLL 上の `.text+0x6360`）。
- 関数: f_407360（DLL の `jmp *0x156a2f2(%rip)`、IAT スロット 0x1571658）。
- IAT スロットの API 名: `VerSetConditionMask`（`objdump -p binaries/win64/FLEngine_x64.dll` の表。DLL 名は表示されず、未確認）。
- 命令: `jmp *IAT`（import thunk）。
- 未対応の種類: `fle_unsupported`。生成器の振り分けで、この import は `imports_table.rs` の表にない（`jmp_imp` の対象外）。`Jmp: indirect through IAT slot (not lifted)` の分類（out-bulk の unsupported 一覧の 435 件に含まれる種類）。
- 呼び出し元: 0x409b67、0x409b77、0x409b87 など（f_409b30 と f_409bd0 から）。この箇所の戻り値の使い方は、今回は読んでいない。

注: 前回の停止点の objdump 行は `lift/objdump-text.txt`（0x11fb4af まで）から読んだ。0x407360 の逆アセンブルは `objdump -d --start-address` で DLL を直接読んだ。ELF の複製（out-cpuid/bin）を読んだ出力は位置がずれていたので使っていない。

## 3. 実行の詳細

コマンド:
- 生成: `cpuid-tool lift binaries/win64/FLEngine_x64.dll lift/out-cpuid-run/out/after-cpuid-896.c <seeds>`（seed 896 件、`lift/out-cpuid-run/seeds-896.txt`）。
- ビルド: `lift/out-cpuid-run/build.sh`（clang -O0、`-finstrument-functions`、harness と runtime をリンク）。
- 実行: `lift/out-cpuid-run/bin/flengine-after-cpuid-896 binaries/win64/FLEngine_x64.dll`。終了コード 134（SIGABRT）。

生成の結果（`lift/out-cpuid-run/gen-896.txt`）:
- closure_instructions 497367、functions 8427（out-bulk の 896 件版と一致）。
- handled に cpuid 5 件（前回は 5 件とも unsupported）。
- unsupported_total 1617（前回 1622。cpuid の 5 件分が減った）。

閉包の計測（`lift/out-cpuid-run/closure-896.txt`）は out-bulk の `closure-896.txt` と一致した。

## 4. 検証

- テスト: `cargo test --offline --workspace`（`lift/`、出力先は `lift/out-cpuid/`）。全件通過（EXIT=0）。cpuid のテスト 2 件（`cpuid_body_calls_runtime`、`cpuid_response_matches_table`）を含む。
- 生成 C の応答の値は、テストの表（決定の 10 件）と一致。

## 5. 実行時の葉（probe）

`lift/out-cpuid/run-probe.err`。probe は計測専用の複製（`lift/out-cpuid/out/probe-896.c`、`L_40f47b` 1 行だけ差し替え）。probe の応答は host の値で、応答の実装には使っていない。値は決定 0009 の表にある。

## 6. 注意（作業の失敗）

- 最初の probe 実行で、作業ディレクトリの誤り（`cd lift/out-cpuid` が失敗）により、`echo` の出力が `lift/run-probe.exit` に書かれた。このファイルは作業者が作ったもので、rm は禁止のため残置している。内容は `EXIT=1`。本番の成果物ではない。
- 前回の計測の `lift/out-bulk/` は読み取りのみで、変更していない。

## 7. 残りの作業量（推測を含む）

- 次の停止点（VerSetConditionMask の thunk）の扱いは未決。未実装。
- 停止点より後の経路は未確認。今回の結果から、後続の import thunk が他にもあるかどうかは分からない（閉包の 445 件の import_jumps のうち、対応済みは `imports_table.rs` の 6 件）。
- cpuid の応答が、FLEngine のどの分岐にどう効くかは未確認（前述の決定 0009 の「実測として言えないこと」）。
