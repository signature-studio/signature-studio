# x87 制御命令の追加後の実行（896 seed の閉包）

日付: 2026-10-20。決定は `docs/decisions/0019-x87-control.md`。出力はすべて `lift/out-x87ctl/`。

## 1. x87 命令の全件集計（前回の生成 C `lift/out-locale/out/after-locale-896.c`、実測）

`/* VA Mnemonic */` のコメントを集計。`Jmp` 398 件などの他の未対応は別。

制御系（今回の対象）:

| ニーモニック | 件数 |
|---|---|
| Fldcw | 4 |
| Fnclex | 2 |
| Fnstcw | 2 |
| Fninit | 1 |
| Fnstsw | 1 |
| Fwait / Wait | 0 |
| 計 | 10 |

演算系（fld、fstp、fmul など）: 0 件。この閉包に x87 の演算命令は出現しない。
参考: Emms 3 件は MMX（x87 の制御ではない）で、未対応のまま。

## 2. 仕様

`docs/decisions/0019-x87-control.md` に出典と確認状況を書いた。felixcloutier の転載（非公式）と SDM Vol. 1 §8.1.5 の転載ページを読んだ。公式 PDF は未取得。予約ビットの扱いは仕様の文面で確認できず、ホストの実測（65536 通り全数）で決めた。

## 3. 実装

- 新規: `lift/crates/fe-lift/src/x87ctl.rs`、`lift/runtime/fle_x87ctl.h`、`lift/runtime/fle_x87ctl.c`、`lift/crates/fe-lift/tests/x87ctl.rs`。
- 変更: `lift/crates/fe-lift/src/lib.rs`（mod 宣言 1 行と、mxcsr、cpuid と同じ形の `if let` 4 行）。
- runtime は先に `out-x87ctl/work/` でコンパイル（警告なし）し、`work/check.c` で FLDCW 全 65536 値をホストと照合（不一致 0）してから、`lift/runtime/` にヘッダ、.c の順で 1 回置いた。

## 4. テスト

- `cargo test --offline -p fe-lift --test x87ctl`: 7 件、全部 ok（`test-x87ctl-1.txt`）。内容: 30 段の列（fnstcw、fldcw、fnstsw の mem と AX、fnclex、fninit、fwait。生成 C、ホスト、期待値の 3 者比較）、FLDCW 65536 値の全数比較、`fnstcw [rsp]`、`fldcw [rip+...]`、`fnstsw [rbp+0xc]` の生成本文、FWAIT、x87 演算が未対応のままであること。
- `cargo test --offline --workspace --no-fail-fast`（`test-workspace-1.txt`、`test-workspace-1.exit` は EXIT=0）: 合計 165 件 passed、0 failed（`test result` 行の集計）。この実行中に、別担当の `fle_env.*`、`fle_vfs.h` が `lift/runtime/` にあった（実行時点の状態）。

## 5. 生成、clang、リンク

- 生成: `out-x87ctl/tool`（`out-locale/tool` の複製）で 896 seed、`out/after-x87ctl-896.c`（新規）、EXIT=0。`gen-x87ctl-896.txt`。
  - 閉包の命令数 497367、関数 8427、emitted 497527（前回と同じ）。C は 548232 行（前回と同じ）、42514303 バイト（前回 42514031）。
  - 未対応の合計: 625 → 615（x87 の 10 件が handled: fldcw 4、fnclex 2、fninit 1、fnstcw 2、fnstsw 1）。
- clang -O0 とリンク: `out-x87ctl/build.sh`（`out-locale/build.sh` の複製で出力先だけ変更）。`EXTRA_RT="fle_int_more fle_sse_more fle_sys fle_locale fle_x87ctl"`、EXIT=0（`build-896.txt`）。`bin/flengine-after-x87ctl-896`。
  - `fle_env.c` は EXTRA_RT に入れていない（別担当の作業中。生成器の `imports_table.rs` もまだそれを参照していない）。

## 6. 実行

コマンド: `./bin/flengine-after-x87ctl-896 ../../binaries/win64/FLEngine_x64.dll`（TMPDIR は `out-x87ctl/tmp`）。出力 `run-x87ctl-896.txt`、`.err`、`.exit`（EXIT=134、SIGABRT）。

| 項目 | 前回（out-locale） | 今回（out-x87ctl） |
|---|---|---|
| 停止 VA | 0x40a332 | 0x407040 |
| 関数 | f_40a330 | f_407040 |
| abort の種類 | fle_unsupported | fle_unsupported |
| 関数実行数（distinct） | 42 | 44 |
| 関数エントリ数（calls） | 64 | 66 |
| 呼び出し深さ | 7 | 6 |

前回の停止点（f_40f500 内の fnstcw）は通過した。f_40f500 は戻り、続く 0x41ce98 の `call 0x40f530` も呼ばれて戻り、0x41ceb8 の `call 0x407040` で止まった。

## 7. 次の停止点

- VA 0x407040、関数 f_407040（呼び出し元 0x41ceb8、戻り先 0x41cebd。呼び出し元の関数は f_41ce10）。
- 命令: `jmp QWORD PTR [rip+0x156a59a]`（ff 25 9a a5 56 01）、IAT スロット 0x19715e0。
- IAT の名前: `kernel32.dll!GetCommandLineW`（`out-x87ctl/tool/src/iat.rs` で PE のインポートテーブルから引いた。`iat-0x407040.txt`）。
- 未対応の種類: x87 ではなく、`Jmp: indirect through IAT slot (not lifted)`（生成器の `imports_table.rs` に GetCommandLineW の行がない）。生成 C の該当行は `L_407040: { /* 0x407040 Jmp */ fle_unsupported(...) }`。
- 観察: `lift/runtime/fle_env.c`（別担当）に `fle_imp_GetCommandLineW` の実装が既にある（`fle_env.h` に宣言）。生成器側の接続（`imports_table.rs`）は別担当の作業で、私は触っていない。接続されれば、この停止点は進む見込み（推測）。

## 8. 残り

- この閉包の x87 の未対応は 0 件になった。x87 演算の lift は必要が出ていない。
- 次の停止点は GetCommandLineW（別担当の fle_env 系の接続待ち）。その後の停止点は未測定。
- 未対応の合計は 615 件（Jmp の IAT 経由 378、解決不能 20、SSE/MMX の各種など。`gen-x87ctl-896.txt`）。
- 公式 PDF との照合、Intel CPU での FLDCW 読み戻しの確認は未実施。
