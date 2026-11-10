# MMX（mm 形）命令と EMMS の追加後の実行（896 seed の閉包）

日付: 2026-10-20。決定は `docs/decisions/0028-mmx.md`。出力はすべて `lift/out-mmx/`（`tmp/` は TMPDIR）。手順は `docs/investigations/sse-int2-run.md` と同じ。

## 1. 未実装 MMX 命令の一覧（実装前）

入力: `lift/out-sse2/out/after-sse2-896.c`（`unsupported_total` 411）。`/* 0xVA Mnemonic */ fle_unsupported` の行から MMX と Emms のものを取り出し、VA を `lift/objdump-text.txt` に当てて形を確認した（`out-mmx/work/mmx-insns.txt`）。**40 件、12 mnemonic、すべて mm 形**（xmm 形・VEX 形は 0）。

| mnemonic | 件数 | 形 |
|---|---|---|
| Movd | 10 | `movd mm,r32` 6、`movd r32,mm` 4 |
| Punpcklbw | 5 | `punpcklbw mm,mm` |
| Packuswb | 4 | `packuswb mm,mm` |
| Paddw | 4 | `paddw mm,mm` |
| Pxor | 4 | `pxor mm,mm` |
| Pmulhuw | 3 | `pmulhuw mm,mm` |
| Emms | 3 | `emms` |
| Movq | 2 | `movq mm,m64` 1、`movq mm,r64` 1 |
| Psllw | 2 | `psllw mm,imm8` |
| Pmulhw | 1 | `pmulhw mm,mm` |
| Pshufw | 1 | `pshufw mm,mm,imm8` |
| Psubsw | 1 | `psubsw mm,mm` |
| **計** | **40** | MMX 37 + Emms 3 |

出現する関数（閉包の未対応行のアドレス）: 0x63a660（`fnclex; fldcw; emms; ret`）、0x660c40、0x6a79ae 以降（0x6a79d7〜0x6a7a55）。0x660c40 は `pxor mm4,mm4` から `emms` まで 14 命令。依頼文にある 0x660c80 は `pxor mm4,mm4` で始まる別の関数だが、閉包の未対応行には現れない（閉包の外。`objdump-text.txt` で確認。EMMS は `.text` 全体で 6 件、閉包内は 0x63a668、0x660c6e、0x6a7a55 の 3 件）。

## 2. 実装

- 新規: `lift/crates/fe-lift/src/mmx.rs`、`lift/runtime/fle_mmx.h`、`lift/crates/fe-lift/tests/mmx.rs`、`docs/decisions/0028-mmx.md`、この文書。
- 編集: `lift/crates/fe-lift/src/lib.rs`（`mod mmx;` 1 行 + `if let` 4 行、+5 行）、`lift/runtime/fle_sse.h`（末尾に `#include "fle_mmx.h"` と注釈、+3 行）、`lift/crates/fe-lift/tests/sse_int2.rs`（+2/-2 行。§3 参照）。`imports_table.rs` は触っていない。
- MMX 状態: 独立の `uint64_t MM[8]`。x87 の ST とはエイリアスしない。x87 側は `fle_x87`（決定 0019）の TOP と tag word だけを SDM どおりに更新する（MMX 命令: TOP = 0、tag = 0000H。EMMS: TOP = 0、tag = FFFFH）。理由と未モデル化の範囲は決定 0028 の判断 1、2。

## 3. テスト

- `cargo test --offline -p fe-lift --test mmx`: **14 件、すべて通過**（`test-mmx-3.txt`、EXIT=0。最終 `test-mmx-final.txt`）。内訳: ホスト突合 12（PXOR、PADDW、PSUBSW、PMULHW、PMULHUW、PACKUSWB、PUNPCKLBW、PSLLW、PSHUFW、MOVD、MOVQ、EMMS）、ケース集合の範囲確認 1、他の MMX 形・xmm 形の振り分け確認 1。
- ケース数の合計 10,081、失敗 0。比較対象は mm0-7、GPR 8 本、メモリ 64 バイト、命令後と EMMS 後の x87 ステータスワード（TOP 込み）と空 tag の集合。ホストは命令前に FDECSTP を 0〜7 回行い、半数で EMMS により tag を空にする。ホストのスタブは最後に必ず EMMS を実行する。
- 空振り防止: ホストが状態を変えたケースは全グループで 80% 以上（EMMS 15/16、PXOR 1039/1041、MOVQ 1197/1200、他は全件）。
- 感度の確認: TOP のリセットを外す変異で 11 件失敗（`test-mmx-mutation-top.txt`）、tag のリセットを外す変異で 11 件失敗（`test-mmx-mutation-tag.txt`）。元に戻して 14 件通過。
- 失敗 1 回: 初回（`test-mmx-1.txt`）は `punpcklbw mm,[mem]` のメモリが m32 であるのに実装が m64 を要求して `Err` になり 1 件失敗（SDM の `PUNPCKLBW mm, mm/m32`）。メモリサイズを m32 にして修正（`test-mmx-2.txt` で 14 件通過）。
- 既存テストの変更: `tests/sse_int2.rs` の `mmx_and_vex_forms_stay_unsupported` が `psllw mm,mm`、`pshufw mm,mm,imm8` を「未対応のまま」の例にしていたため、今回対応したので `psrlw mm,mm`、`psraw mm,mm` に差し替えた。最初のワークスペース全体のテストは、この 1 件だけが失敗（370 通過、1 失敗）。差し替え後に全件通過した。
- `cargo test --offline --workspace --no-fail-fast`（`test-ws.txt`、`test-ws.exit` は EXIT=0）: **48 バイナリ、371 件 passed、0 failed、0 ignored**（`test result` 行の集計）。前回（sse-int2-run.md）は 43 バイナリ、294 件。差 +5 バイナリ、+77 件のうち、今回の `mmx` は +1 バイナリ、+14 件。残りは他の担当のテストと思われる（`git status` に `tests/res.rs`、`tests/pdata*.rs` があり、`sse_shuf.rs` は HEAD に入っている。推測。増減の内訳は確認していない）。

## 4. 生成、clang、リンク

- 生成器: `out-mmx/tool`（`out-sse2/tool` の複製）を `out-mmx/target-tool` にビルド（`build-tool.txt`、EXIT=0）。seed は `out-mmx/seeds-896.txt`（896 件、`out-sse2` の複製）。`out-mmx/Cargo.toml` は `out-sse2/Cargo.toml` と同じ内容。
- 生成: EXIT=0（`gen-mmx-896.txt`、`.err` は空）。functions 8427、closure_instructions 497367、emitted 497527（前回と同じ）、c_lines 548442 → 548457。
- **`unsupported_total`: 411 → 340**（−71）。`unsupported` の種類は 25 → 9。差の内訳（`u-sse2.txt` と `u-mmx.txt` の diff、および `out-shuf/gen-shuf-896.txt` との diff）:
  - MMX と Emms の 40 件（12 mnemonic）が handled に移った（`mmx_*` の handled 件数: movd 10、punpcklbw 5、packuswb 4、paddw 4、pxor 4、pmulhuw 3、emms 3、movq 2、psllw 2、pmulhw 1、pshufw 1、psubsw 1 = 40）。**今回の変更による減少は 40 件。**
  - 0027（`sse_shuf`、別担当、すでにコミット済みの HEAD b464d45）の 26 件（Shufps 13、Movlps 8、Movhlps 3、Movlhps 2）。直前の生成 `out-shuf/gen-shuf-896.txt` は 385 で、411 − 26 = 385。
  - 残りの −5: `Jmp: indirect through IAT slot` が 304 → 299。ワークツリーの `imports_table.rs` と `runtime/fle_res.c`（別担当の未コミットの変更）が FindResourceW、LoadResource、LoadStringW、LockResource、SizeofResource の 5 つを解決したため（生成 C の 0x407030 が `fle_imp_LoadStringW` を呼ぶ形になり、`fle_res.o` なしのリンクが上の 5 シンボルの未定義で失敗したことから）。**今回の変更によるものではない。** 385 − 40 − 5 = 340 で算術は一致。
- clang -O0 とリンク: `out-mmx/build.sh`（`out-sse2/build.sh` の複製）。`EXTRA_RT` は int-more2-run.md §4 と同じ一覧に `fle_res` を足した（上記の別担当の変更で必要）。1 回目は `fle_res` なしで 5 シンボルの未定義参照でリンク失敗、2 回目は EXIT=0（`build-896.txt`）。`fle_mmx.h` は header only で、runtime に `.o` は増えていない（`fle_x87ctl.o` は既存の `EXTRA_RT` に入っている）。

## 5. 実行

コマンド: `./bin/flengine-after-mmx-896 ../../binaries/win64/FLEngine_x64.dll`（`out-mmx/` で）。出力 `run-mmx-896.txt`、`.err`、`.exit`（EXIT=134、SIGABRT）。

| 項目 | 前回（sse2） | 今回 |
|---|---|---|
| 停止点 | 0x407030（`user32!LoadStringW` の thunk、`fle_unsupported`） | `fle: indirect call to 0x40f960 is not a known function entry`（`fle_dispatch_miss`） |
| 関数実行数（distinct） | 116 | 125 |
| 関数エントリ数（calls） | 340 | 658 |
| 呼び出し深さ | 9 | 8 |

- 停止点が変わったのは、MMX の追加ではなく、上の別担当の変更で LoadStringW が解決されて 0x407030 を通過できるようになったため、と判断している（理由: 前回の停止点が 0x407030 で、今回の生成 C では同じ VA が `fle_imp_LoadStringW` を呼ぶ。MMX 命令は前回の経路上にない。実行の比較は MMX を入れる前の同じ build では行っていないので、因果は推測）。0x40f960 は閉包の関数エントリではなく（生成 C に `f_40f960` も `L_40f960` もない、`objdump-text.txt` では 0x40f960 は `push rbx` で始まる関数の先頭）、間接呼び出しの解決の問題。MMX とは関係しない（調べていない）。
- **MMX の関数は実行で踏まれていない。** `run-mmx-896.txt` の関数エントリ（256 行、distinct 114 種。ログに出るのは先頭 256 件のみ）に f_63a660、f_660c40、f_6a79xx は 1 件もない（`grep` で 0 件）。「関数実行数 125」はトレーラーの値で、ログの 256 エントリには全部は載っていない。実行全体で踏まれていないとまでは言えず、ログの範囲での確認。したがって、実装した MMX 命令が実行で正しく動くかは、実行では確かめられていない。検証の根拠はホスト突合のテスト（§3）。

## 6. 実装後に残る未対応（`unsupported_total` 340）

`gen-mmx-896.txt` の集計（実測）:

| 種類 | 件数 | 備考 |
|---|---|---|
| `Jmp: indirect through IAT slot` | 299 | 別担当の変更で 304 から減った（§4） |
| `Jmp: indirect target unresolved` | 20 | 前回と同数 |
| AVX/VEX | 18 | Vmovups 12、Vmovaps 2、Vmovntps 2、Vzeroupper 2。別作業 |
| `Lea`/`Mov` の FS/GS | 2 | |
| Xgetbv | 1 | 決定 0023 |
| 計 | 340 | 299 + 20 + 18 + 2 + 1 = 340（算術で一致） |

MMX（mm 形）の未実装、Emms、xmm 形のパック整数、SSE 浮動小数点のシャッフルは、閉包で **0 件**。

## 7. 取得物・失敗・拒否された操作

- 外部への通信: SDM の公式 PDF（`https://cdrdv2.intel.com/v1/dl/getContent/671200` → `cdrdv2-public.intel.com/929349/325462-093-sdm-vol-1-2abcd-3abcd-4.pdf`、26,664,929 バイト）を `lift/out-mmx/spec/probe.pdf` に取得（`probe.hdr` に応答ヘッダ）。`pdftotext -layout` の出力は `sdm-vol1.txt`（Vol. 1 前半）と `sdm-full.txt`（17.8 MB）。2 GB 級のファイルはない。`out-mmx` 全体で 257 MB（`target-tool` と PDF、バイナリを含む）。
- 拒否された操作: なし。
- 失敗: §3 の初回テスト 1 件（m32 の修正）、§4 のリンク 1 回、§3 の `sse_int2` の差し替え前の 1 件。いずれも解決済み。
- ホスト実測: `lift/out-mmx/work/probe_x87.c`、`probe_x87-out.txt`（MMX 命令後と EMMS 後の x87 の SW/TW、m64 オペランドの非整列アクセスが fault しないこと）。最後に EMMS を実行している。
