# SSE 浮動小数シャッフル・半分移動の追加後の実行（896 seed の閉包）

日付: 2026-10-20。決定は `docs/decisions/0027-sse-shuf.md`。出力はすべて `lift/out-shuf/`（`tmp/` は TMPDIR）。前回の記録は `docs/investigations/sse-int2-run.md`（`unsupported_total` 411、停止点 0x407030）。

## 1. 未実装の対象命令の一覧と出現数（実装前）

入力: `lift/out-sse2/out/after-sse2-896.c`（`unsupported_total` 411）。`fle_unsupported` の行のうち Shufps、Movlps、Movhlps、Movlhps（および近縁の Shufpd、Unpck*、Movhps、Movlpd、Movhpd。後者は 0 件）を抜き出し（`out-shuf/addrs.txt`、26 行）、各アドレスを `lift/objdump-text.txt` の逆アセンブルに当てて形を実測した。

| ニーモニック | 件数 | 形（実測の逆アセンブル） |
|---|---|---|
| Shufps | 13 | すべてレジスタ形 `shufps xmm_n,xmm_n,imm8`（同じレジスタ。imm8 は 0x0 が 9 件、0x1 が 4 件） |
| Movlps | 8 | ロード 7（`movlps xmm,QWORD PTR [rcx/rdx/rbx/rbx+0x8]`）、ストア 1（`movlps QWORD PTR [rcx],xmm5`）。レジスタ形はない（`0F 12`/`0F 13` の mod=11 は別命令） |
| Movhlps | 3 | `movhlps xmm,xmm`（xmm2,xmm0 / xmm3,xmm1 / xmm0,xmm7） |
| Movlhps | 2 | `movlhps xmm,xmm`（xmm2,xmm2 / xmm0,xmm1） |
| **計** | **26** | 前任の集計（Shufps 13、Movlps 8、Movhlps 3、Movlhps 2）と一致 |

近縁の閉包での件数: Shufpd、Unpcklps、Unpckhps、Unpcklpd、Unpckhpd、Movhps、Movlpd、Movhpd はいずれも閉包の unsupported に 0 件。（参考: `objdump-text.txt` 全体では shufps 152、shufpd 1、unpcklps 4、unpckhps 5、movlps 61、movhps 10、movhlps 45、movlhps 8 行あるが、それは 896 seed の閉包の外を含む `.text` 全体の数で、今回の統計には入らない。unpcklpd、unpckhpd、movlpd、movhpd は 0 行。）

## 2. 実装

- 新規: `lift/crates/fe-lift/src/sse_shuf.rs`、`lift/runtime/fle_sse_shuf.h`、`lift/crates/fe-lift/tests/sse_shuf.rs`、`docs/decisions/0027-sse-shuf.md`、この文書。
- 編集（追記のみ）: `lift/crates/fe-lift/src/lib.rs`（`mod sse_shuf;` 1 行 + `lift_insn` に `if let` 4 行。編集の直前に読み直した。`git diff --stat` で +5 行）、`lift/runtime/fle_sse.h`（末尾に `#include "fle_sse_shuf.h"` 1 行、+1 行）。`imports_table.rs` は触っていない。
- 実装した mnemonic は 12 種: 閉包に出る 4 種（Shufps、Movlps、Movhlps、Movlhps）と、同じ形で SDM の擬似コードから書ける 8 種（Shufpd、Unpcklps、Unpckhps、Unpcklpd、Unpckhpd、Movhps、Movlpd、Movhpd）。実装前に `src/*.rs` と `runtime/*.h` を grep して、既に実装されたものがないことを確認した（重複なし）。
- メモリ形の境界（SDM で確認、決定 0027 §判断 1）: m128 を取る Shufps、Shufpd、Unpck* は Type 4（#GP(0) 16 バイト境界）なので `fle_ldxa`（境界外は abort）。m64 を取る Movlps、Movhps、Movlpd、Movhpd は Type 5（境界の #GP なし）なので `fle_ld`/`fle_st` で任意のアドレス。Movhlps、Movlhps はレジスタ形のみ。

## 3. テスト

- `cargo test --offline -p fe-lift --test sse_shuf`: **17 件、すべて通過**（`test-sse_shuf-2.txt`、EXIT=0。`test-sse_shuf-1.txt` は同じ内容の 1 回目、17 件通過）。内訳: ホスト突合 13、ケース集合の範囲確認 1、m128 境界外が abort する 1（6 命令）、m64 の境界外が abort しない 1（4 命令のロード/ストア）、VEX 形が未対応のまま 1。
- ホスト突合のケース数の合計: 8,352（`cases N failures 0` の和）。Shufps 1,112、Shufpd 1,112、Unpck 4 命令が各 800、Movhlps 600、Movlhps 600、Movlps/Movhps/Movlpd/Movhpd が各 342、閉包のレジスタ形エンコード 9 種 360。失敗 0。imm8 は Shufps、Shufpd とも 256 通り全部。ホストが xmm かメモリを変えたケースは全命令で 97.6% 以上（Shufpd 1,085/1,112、Unpcklpd 799/800、Movhlps/Movlhps と閉包のレジスタ形 599/600 または 359/360、他は全件）。
- 変異テスト: `fle_shufps` の `(imm >> 4)` を `(imm >> 5)` にすると shufps のテストは 1,112 ケース中 813 件が失敗（EXIT=101、`test-sse_shuf-mutant.txt`）。元へ戻して再実行し 17 件通過。なお変異の 1 回目の実行は通過してしまった（原因は推測: 取り消しの編集とテストの実行を同じ並列ブロックで発行したため。確認していない）。2 回目で失敗を確認した。決定 0027 に記載。
- `cargo test --offline --workspace --no-fail-fast`（`test-ws.txt`、`test-ws.exit` は EXIT=0）: **45 バイナリ、320 件 passed、0 failed、0 ignored**（`test result` 行の集計）。前回（sse-int2-run.md）の 43 バイナリ、294 件に対し +2 バイナリ、+26 件。+17 は今回の `sse_shuf` で、残りの +9 件、+1 バイナリ（`fe-lift/tests/res.rs`）などは他の担当の変更（`fe-pe/tests/resources.rs`、`fe-lift/tests/res.rs`、`runtime/fle_res.*` は作業ツリーにあるが私は触っていない）によるものと**推測**する（内訳は確認していない）。

## 4. 生成、clang、リンク

- 生成器: `out-shuf/tool`（`out-sse2/tool` の複製）、`out-shuf/Cargo.toml`（`out-sse2/Cargo.toml` と同じ内容。cargo は workspace 外のパスを拒否するので、前任と同じ方法）を `out-shuf/target-tool` にビルド（`build-tool.txt`、EXIT=0）。seed は `out-shuf/seeds-896.txt`（896 件、`out-sse2` の複製）。
- 生成: EXIT=0（`gen-shuf-896.txt`、`.err` は空）。`out/after-shuf-896.c`（42,536,308 バイト）。functions 8427、closure_instructions 497367、emitted 497527、c_lines 548442（いずれも前回と同じ）、bad_nodes 0。
- **`unsupported_total`: 411 → 385**（−26）。`unsupported` の種類は 25 → 21。
  - 減ったのは §1 の 26 件（Shufps 13、Movlps 8、Movhlps 3、Movlhps 2）だけ。`u-sse2.txt` と `u-shuf.txt`（`sort` した unsupported の一覧）の `diff` で確認（この 4 種の行が消え、`unsupported` 25→21、`unsupported_total` 411→385 以外の差はない）。
  - `handled` の集計に新しい行: shufps 13、movlps 8、movhlps 3、movlhps 2（実装した命令が閉包から 26 件 lift された）。
- clang -O0 とリンク: `out-shuf/build.sh`（`out-sse2/build.sh` の複製、既定の NAME のみ変更）、`EXTRA_RT="fle_int_more fle_sse_more fle_locale fle_sys fle_x87ctl fle_vfs fle_reg fle_env fle_uilang fle_proc fle_time fle_uipref"`（前回と同じ）、EXIT=0（`build-896.txt`）。runtime に `.o` は増えていない（`fle_sse_shuf.h` は header only）。

## 5. 実行

コマンド: `./bin/flengine-after-shuf-896 ../../binaries/win64/FLEngine_x64.dll`（`out-shuf/` で）。出力 `run-shuf-896.txt`、`.err`、`.exit`（EXIT=134、SIGABRT）。

| 項目 | 前回 | 今回 |
|---|---|---|
| 停止点 | 0x407030（`jmp [rip+..]`、`user32!LoadStringW`） | 0x407030（同じ） |
| 関数実行数（distinct） | 116 | 116 |
| 関数エントリ数（calls） | 340 | 340 |
| 呼び出し深さ | 9 | 9 |

停止点は変わらない（実行は LoadStringW の thunk の手前で止まる経路で、今回実装した命令には届かない）。したがって、実装した命令が実行で踏まれるかは実行では確かめられていない。ホスト突合のテストが、この作業での検証の根拠になる。`run-shuf-896.txt` の末尾 12 行は `run-sse2-896.txt` と同一、`.err` も同一（`diff` で確認）。

## 6. 実装後に残る未対応（`unsupported_total` 385）

`gen-shuf-896.txt` の集計（実測）:

| 種類 | 件数 | 備考 |
|---|---|---|
| `Jmp: indirect through IAT slot` | 304 | 前回と同数 |
| `Jmp: indirect target unresolved` | 20 | 前回と同数 |
| MMX（mm 形）| 37 | Movd 10、Punpcklbw 5、Packuswb 4、Paddw 4、Pxor 4、Pmulhuw 3、Movq 2、Psllw 2、Pmulhw 1、Pshufw 1、Psubsw 1。別作業 |
| Emms | 3 | 別作業（MMX の後始末） |
| AVX/VEX | 18 | Vmovups 12、Vmovaps 2、Vmovntps 2、Vzeroupper 2。別作業 |
| `Lea`/`Mov` の FS/GS | 2 | |
| Xgetbv | 1 | 決定 0023 |
| 計 | 385 | 304 + 20 + 37 + 3 + 18 + 2 + 1 = 385（算術で一致） |

SSE 浮動小数のシャッフル・半分移動で未実装のものは **0 件**。

## 7. 失敗・拒否された操作

- 拒否: なし。
- 失敗: 変異テストの 1 回目が通過した（上記 §3。テストの欠陥ではなく手順の誤りと推測。2 回目で失敗を確認）。ほかに失敗なし。
- 外部への通信: なし（SDM は決定 0023 で取得済みのローカルのテキストを使った）。
- 削除、上書き（`>` による既存ファイルの上書き）、git 操作、/tmp の使用: なし。`out-shuf/tmp` を TMPDIR にした。`test-sse_shuf-mutant.txt` は同じ名前への 2 回目のリダイレクトで内容が置き換わった（自分で作った作業ファイルで、1 回目の出力は残っていない）。
