# SSE2 xmm 整数パック命令の追加後の実行（896 seed の閉包）

日付: 2026-10-20。決定は `docs/decisions/0026-sse-int2.md`。出力はすべて `lift/out-sse2/`（`tmp/` は TMPDIR）。前回の記録は `docs/investigations/int-more2-run.md`（`unsupported_total` 502、停止点 0x407030）。

## 1. 未実装 SSE 整数命令（xmm 形）の一覧と出現数（実装前）

入力: `lift/out-int2/out/after-int2-896.c`（`unsupported_total` 502）。`fle_unsupported` の行 95 件（対象ニーモニック 17 種）のアドレスを `lift/objdump-text.txt` の逆アセンブルに当てて、xmm か mm かを判定した。

| ニーモニック | xmm 形 | mm 形（対象外） | 形（実測の逆アセンブル） |
|---|---|---|---|
| Pshuflw | 16 | 0 | `pshuflw xmm,xmm,imm8` |
| Psllw | 12 | 2 | `psllw xmm,imm8` |
| Packssdw | 10 | 0 | `packssdw xmm,xmm` |
| Psrlw | 8 | 0 | `psrlw xmm,imm8` |
| Paddusb | 6 | 0 | `paddusb xmm,xmm` |
| Pmaxsw | 6 | 0 | `pmaxsw xmm,xmm` |
| Pminsw | 6 | 0 | `pminsw xmm,xmm` |
| Pmulhw | 6 | 1 | `pmulhw xmm,xmm` |
| Pmullw | 6 | 0 | `pmullw xmm,xmm` |
| Psubsw | 4 | 1 | `psubsw xmm,xmm` |
| Psubusb | 3 | 0 | `psubusb xmm,xmm` |
| Pshufhw | 2 | 0 | `pshufhw xmm,xmm,imm8` |
| Psrad | 2 | 0 | `psrad xmm,imm8` |
| Paddq | 1 | 0 | `paddq xmm,xmm` |
| Paddsw | 1 | 0 | `paddsw xmm,xmm` |
| Psraw | 1 | 0 | `psraw xmm,imm8` |
| Psrld | 1 | 0 | `psrld xmm,imm8` |
| **計** | **91** | **4** | |

（xmm 形の列の和は 91。mm 形は Psllw 2、Pmulhw 1、Psubsw 1 の計 4 で、合計 95 行。）メモリオペランドの形は、閉包の 91 件には 0 件（すべてレジスタ形）。メモリ形は、ホスト突合のテストでだけ検証している。

`remaining-unsupported.md` §4 の V1（Pshuflw 16、Psllw 12、Packssdw 10、Psrlw 8 ほか 91 件、17 種）と、xmm 形の件数が一致した。そこに書かれた他の SSE 件数（Shufps、Movlps など 26 件）は浮動小数点のシャッフルで、整数パックではないので対象外（V2）。

## 2. 実装

- 新規: `lift/crates/fe-lift/src/sse_int2.rs`、`lift/runtime/fle_sse_int2.h`、`lift/crates/fe-lift/tests/sse_int2.rs`、`docs/decisions/0026-sse-int2.md`、この文書。
- 編集（追記のみ）: `lift/crates/fe-lift/src/lib.rs`（`mod sse_int2;` 1 行 + `lift_insn` に `if let` 4 行。`git diff --stat` で +5 行）、`lift/runtime/fle_sse.h`（末尾に `#include "fle_sse_int2.h"` と 1 行の注釈、+3 行）。`imports_table.rs` は触っていない。
- 実装した mnemonic は 25 種（`xmm, xmm/m128`、シフトは `xmm, imm8` も、シャッフルは `xmm, xmm/m128, imm8`）: Paddq、Paddsb/w、Paddusb/w、Psubsb/w、Psubusb/w、Pmaxsw、Pminsw、Pmulhw、Pmullw、Packsswb、Packssdw、Psllw/d/q、Psrlw/d/q、Psraw/d、Pshuflw、Pshufhw。閉包に出る 17 種と、同じ形の 8 種（Paddsb、Paddusw、Psubsb、Psubusw、Packsswb、Pslld、Psllq、Psrlq）。
- メモリ形の 16 バイト境界: SDM の Type 4 例外クラスに従い、境界外は abort（決定 0026 §判断 1）。ホストで境界外が SIGSEGV になることを実測した（`out-sse2/work/probe_align-out.txt`）。

## 3. テスト

- `cargo test --offline -p fe-lift --test sse_int2`: **28 件、すべて通過**（`test-sse_int2-3.txt`、EXIT=0）。内訳: ホスト突合 25（命令ごと）、ケース集合の範囲確認 1、境界外メモリが abort する 1（5 命令）、mm/VEX 形が未対応のまま 1。
- ホスト突合のケース数の合計: 19,855（`test-sse_int2-3.txt` の `cases N failures 0`。二項 15 命令が 769〜1,041、シフト 8 命令が各 538、シャッフル 2 命令が各 512）。失敗 0。ホストが xmm を変えたケースは、全命令で 80% 以上（Pmaxsw 972/1041、Pminsw 985/1041、シフト 530〜531/538、その他は全件）。
- 1 回目の実行（`test-sse_int2-1.txt`、28 件通過）の後、「ホストが xmm を変えたか」の検査（空振り防止）を `check` に足して再実行した（`test-sse_int2-2.txt` は足す前の `--nocapture` 実行、`-3` が足した後）。足した検査で失敗はなかった。
- `cargo test --offline --workspace --no-fail-fast`（`test-ws.txt`、`test-ws.exit` は EXIT=0）: **43 バイナリ、294 件 passed、0 failed、0 ignored**（`test result` 行の集計）。前回（int-more2-run.md）の 42 バイナリ、266 件に対し、+1 バイナリ、+28 件。+28 は今回の `sse_int2` の 28 件と一致する（他の担当のテストの増減は確認していない）。

## 4. 生成、clang、リンク

- 生成器: `out-sse2/tool`（`out-int2/tool` の複製、`out-sse2/Cargo.toml` は `out-int2/Cargo.toml` と同じ内容）を `out-sse2/target-tool` にビルド（`build-tool.txt`、EXIT=0）。seed は `out-sse2/seeds-896.txt`（896 件、`out-int2` の複製）。
- 生成: EXIT=0（`gen-sse2-896.txt`、`.err` は空）。`out/after-sse2-896.c`。functions 8427、closure_instructions 497367、emitted 497527、c_lines 548442（いずれも前回と同じ）。
- **`unsupported_total`: 502 → 411**（−91）。`unsupported` の種類は 39 → 25。
  - 減ったのは、§1 の xmm 形 91 件（17 種）だけ。`u-int2.txt` と `u-sse2.txt` の差分で確認（Psllw 14 → 2、Psubsw 5 → 1、Pmulhw 7 → 1 は mm 形が残った分。他の 14 種は 0 になった）。
- clang -O0 とリンク: `out-sse2/build.sh`（`out-int2/build.sh` の複製）、`EXTRA_RT` は int-more2-run.md §4 と同じ、EXIT=0（`build-896.txt`）。runtime に `.o` は増えていない（`fle_sse_int2.h` は header only）。

## 5. 実行

コマンド: `./bin/flengine-after-sse2-896 ../../binaries/win64/FLEngine_x64.dll`（`out-sse2/` で）。出力 `run-sse2-896.txt`、`.err`、`.exit`（EXIT=134、SIGABRT）。

| 項目 | 前回 | 今回 |
|---|---|---|
| 停止点 | 0x407030（`jmp [rip+..]`、`user32!LoadStringW`） | 0x407030（同じ） |
| 関数実行数（distinct） | 116 | 116 |
| 関数エントリ数（calls） | 340 | 340 |
| 呼び出し深さ | 9 | 9 |

停止点は変わらない（実行はまだ LoadStringW の thunk に届く前の経路で止まり、SSE の整数パックには届いていない）。したがって、今回実装した命令が実行で踏まれるかは、実行では確かめられていない。ホスト突合のテストが、この作業での検証の根拠になる。

## 6. 実装後に残る未対応（`unsupported_total` 411）

`gen-sse2-896.txt` の集計（実測）:

| 種類 | 件数 | 備考 |
|---|---|---|
| `Jmp: indirect through IAT slot` | 304 | 前回と同数 |
| `Jmp: indirect target unresolved` | 20 | 前回と同数 |
| MMX（mm 形）| 37 | Movd 10、Punpcklbw 5、Packuswb 4、Paddw 4、Pxor 4、Pmulhuw 3、Movq 2、Psllw 2、Pmulhw 1、Pshufw 1、Psubsw 1。別作業 |
| Emms | 3 | 別作業（MMX の後始末） |
| SSE 浮動小数点のシャッフル・半分移動（V2） | 26 | Shufps 13、Movlps 8、Movhlps 3、Movlhps 2 |
| AVX/VEX（V7） | 18 | Vmovups 12、Vmovaps 2、Vmovntps 2、Vzeroupper 2。別作業 |
| `Lea`/`Mov` の FS/GS | 2 | |
| Xgetbv | 1 | 決定 0023 |
| 計 | 411 | 304 + 20 + 37 + 3 + 26 + 18 + 2 + 1 = 411（算術で一致） |

xmm 形の整数パックで未実装のものは **0 件**。

## 7. 失敗・拒否された操作

- 拒否: なし。
- 失敗: 生成器のビルドで、`out-sse2/tool` を `lift/` のワークスペースの下に置いたとき、`Cargo.toml` がワークスペースの外だと cargo が拒否した（`out-int2` は独立した `[workspace]` の `Cargo.toml` を持っていた）。同じ内容の `out-sse2/Cargo.toml` を作って解決した。
- 外部への通信: なし（SDM は決定 0023 で取得済みのローカルのテキストを使った）。
