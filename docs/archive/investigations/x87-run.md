# x87 演算の実装と all-pdata 再生成の実行記録

日付: 2026-10-20〜21。決定は `docs/decisions/0033-x87.md`。出力は `lift/out-x87/`（`tmp/` は TMPDIR、`target/` は CARGO_TARGET_DIR、`work/` は作業用 C と測定プログラム）。「実測」はツールの出力、「推測」は未検証の解釈。

## 1. 一覧

`work/x87-insns.txt`、`work/x87-funcs.txt`: F で始まる未対応 2,193 件、73 関数、49 形、32 mnemonic（形の表は決定 0033）。

## 2. 実装

`fe-lift/src/x87.rs`、`runtime/fle_x87.h`、`runtime/fle_x87.c`（`work/fle_x87_final.c` から置いた。ヘッダを先に置き、.c は単体テスト通過後に 1 回）。実装した命令と実装しなかった命令は決定 0033。実装しなかった閉包内の命令: **f2xm1 1 件、fsincos 1 件**（ホストの結果が正しく丸めた値と一致しないため。`work/probe_trans-out.txt`）。

## 3. テスト

- `cargo test --offline -p fe-lift --test x87`（既定）: **14 件**（突合 7 + fail-closed 等 7）通過、突合ケース合計 **31,680**。
- `X87_ROUNDS=8`: 14 件でなく突合 7 件を実行し全通過（`r8-all.txt`、182 秒）。
- `X87_SEED` 0、1、3、5、6: 既定ラウンドで全通過（修正後）。seed 1〜3 の最初の実行では FSCALE の DE と tiny 判定の 3 種の食い違いが出て、runtime を直した。
- 変異: 19 件すべて検出（`mutation.txt`）。
- `tests/x87ctl.rs`、`tests/mmx.rs`: 通過（x87ctl 7、mmx 14）。
- ワークスペース全体（`test-ws.txt`、EXIT=101）: 62 バイナリ、**588 passed、1 failed、0 ignored**。失敗は `tests/null0.rs` の `undefined_cases_stop_the_run`（`run failed: Os { code: 26, kind: ExecutableFileBusy, message: "Text file busy" }`）。`null0` 単独の再実行 3 回は、通過・失敗・通過で**不安定**だった。`modules` は 12 件通過。原因はテストが書き出した実行ファイルを他スレッドの fork と競合して起動するときの ETXTBSY と**推測**するが、未確認。x87 のコードは `null0` のテストに関係しない（推測）が、変更前のツリーで再現するかは試していない。

## 4. all-pdata の再生成（実測）

`out-x87/target/release/fe-lift lift FLEngine_x64.dll out/after-x87.c --pdata`（`gen-x87.txt`、2.46 秒、最大 RSS 320 MB）。

| 項目 | 前（`gen-pdata-b.txt`） | 後 |
|---|---|---|
| 閉包の命令 | 2,523,194 | 2,526,826 |
| 関数 | 58,695 | 58,695 |
| `unsupported_total` | **2,991** | **667** |
| F で始まる未対応 | 2,193 | **2**（F2xm1 1、Fsincos 1） |

差 −2,324 の内訳: x87 −2,191、`Jmp: indirect through IAT slot` 618→557（−61）、`Jmp: indirect target unresolved` 62→1（−61）、Vmovups −12 など VEX（−18）、`Lea/Mov: FS/GS segment` −2、Bswap +2、`Test: high-byte register` +7 など。x87 以外の差は、前回の生成の後に作業ツリーへ入った他の担当の変更（`imports_table.rs`、`vex.rs` など）によるもの（**推測**。個別には確認していない）。閉包の命令が +3,632 増えているのも同じ理由と推測。

## 5. ビルドと実行

- 分割（`split_c.py` 3,700 関数/ファイル、16 ファイル）、`out-x87/build-parts.sh`（`out-pdata/build-parts.sh` の複製で、全 `runtime/fle_*.c` をリンク）: 28.0 秒、最大 RSS 1,202 MB、EXIT=0、未定義・重複定義なし。
- 実行 `./bin/flengine-after-x87 FLEngine_x64.dll`（`run-x87.txt`、EXIT=134）: **停止点は x87 ではない**。`fle: VerifyVersionInfoW: dwOSVersionInfoSize is not sizeof(OSVERSIONINFOEXW) (not defined by the MSDN page; not implemented)`、スタック #0 0x423cc0 ← 0x454030 ← 0x454090 ← 0x4120c0 ← … ← エントリ。実行した関数 147（distinct）、入口 736、深さ 7。
- この実行では**x87 の演算命令に一度も到達していない**（gdb で `fle_x87_*` にブレークポイントを置いた実行でヒット 0。入口ログ 256 件の先頭には x87 関数が無い）。したがって、実行経路で x87 の結果が正しいかは未検証。突合はホストの機械語との単体テストのみ。

## 6. 触ったファイル

- 新規: `lift/crates/fe-lift/src/x87.rs`、`lift/crates/fe-lift/tests/x87.rs`、`lift/runtime/fle_x87.h`、`lift/runtime/fle_x87.c`、`docs/decisions/0033-x87.md`、`docs/investigations/x87-run.md`、`lift/out-x87/` 全体。
- 編集（追記・局所変更）: `lift/crates/fe-lift/src/lib.rs`（`mod x87;` と `if let` 4 行）、`lift/runtime/fle_mmx.h`（`fle_x87_mmx_active` と設定 3 行）、`lift/crates/fe-lift/tests/x87ctl.rs`（`x87_arithmetic_stays_unsupported` の差し替え）。
- 触っていない: `imports_table.rs`、`vex.rs`、`fle_glob/vol/vmap/load/winmm` など他の担当のもの。
- 残置物（削除禁止のため）: `out-x87/work/` の途中版（`fle_x87.c`、`fle_x87.c.new`、`fle_x87_v2.c`、`fle_x87_head.c.txt`、`tail.c.txt`）、`out-x87/mut/`、`out-x87/test/`（突合の生成 C と実行ファイル、約 500 MB）、`out-x87/seed-*.txt`、`r8-*.txt`。
- `rm`、`>` での既存ファイルの上書き、git 操作、`/tmp`、sleep/timeout は使っていない（`cp --update=none` で新規の .c を置き、`cat >>` で追記したのは新規の `tests/x87.rs` のみ）。
