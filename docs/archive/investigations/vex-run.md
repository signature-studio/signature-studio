# VEX（AVX）18 件と FS/GS 2 件の追加後の実行（896 seed の閉包）

日付: 2026-10-20。決定は `docs/decisions/0032-vex-fsgs.md`。出力はすべて `lift/out-vex/`（`tmp/` は TMPDIR）。手順は `docs/investigations/mmx-run.md` と同じ。

## 1. 未実装命令の一覧（実装前）

入力: `lift/out-mmx/out/after-mmx-896.c`（`unsupported_total` 340、`out-mmx/gen-mmx-896.txt`）の `fle_unsupported` 行と、`lift/objdump-text.txt`。VEX 18 件はすべて関数 0x409c80 の中、すべて VEX.256（ymm、vvvv = 1111B）。

| 命令 | 形 | 件数 | アドレス |
|---|---|---|---|
| Vmovups | `ymm, m256` | 8 | 0x409da0、0x409da4、0x409de0、0x409e00、0x409e79、0x409e7d、0x409eb0、0x409ed0 |
| Vmovups | `m256, ymm` | 4 | 0x409df0、0x409df5、0x409ec0、0x409ec4 |
| Vmovaps | `m256, ymm` | 2 | 0x409de5、0x409eb5 |
| Vmovntps | `m256, ymm` | 2 | 0x409e05、0x409ed5 |
| Vzeroupper | — | 2 | 0x409df9、0x409ec9 |
| Lea | `lea rax, fs:[rax+riz*1+0x0]`（`64 48 8d 44 20 00`） | 1 | 0x409dda |
| Mov | `mov rax, QWORD PTR gs:0x58`（`65 48 8b 04 25 58 00 00 00`） | 1 | 0x41d14b（関数 0x41d110） |
| Xgetbv | `0f 01 d0` | 1 | 0x639a03（実装しない） |

メモリオペランドのベース: `[rcx]`、`[rcx+rax*1(-0x20)]`、`[rdx+rax*1]`、`[rdx]`、`[r10]`（REX.B つきの 3 バイト VEX `c4 c1 ..` が 2 件）。

## 2. 実装

- 新規: `lift/crates/fe-lift/src/vex.rs`、`lift/runtime/fle_vex.h`、`lift/crates/fe-lift/tests/vex.rs`、`docs/decisions/0032-vex-fsgs.md`、この文書。
- 編集: `lib.rs`（+5 行: `mod vex;` と `if let` 4 行。編集直前に読み直した）、`fle_sse.h`（+3 行）、`fle_imports.c`（`fle_tls_values` の `static` を外した、1 行）。`imports_table.rs` は触っていない。
- ymm: `X[n]`（下位）と `YH[n]`（上位 128 ビット、weak 定義）。VEX.128 書き込みは上位ゼロ化、VEX.256 は両半分、ストアはレジスタ不変、VZEROUPPER は YMM0-15 の上位をゼロ化。
- FS/GS: LEA はセグメントを無視（SDM）。`mov r64, gs:[0x58]` と `gs:[0x30]` だけを、ホスト側のバッファ `fle_teb` と `fle_tls_values` に結んだモデルで実装。FS の他の形と GS の他の形は `Err`。

## 3. テスト

- `cargo test --offline -p fe-lift --test vex`: **13 件、すべて通過**（`test-vex-1.txt`、EXIT=0）。ホストは AVX 対応（`/proc/cpuinfo` の flags に `avx`、`avx2`、`fma`。テストも `avx` が無ければ失敗する）。
- ホスト突合のケース数: **9,860、失敗 0**。vmovups 3,200 + vmovups（整列アドレスのみ）1,600 + vmovaps 3,200 + vmovntps 800 + vzeroupper 60 + 命令列 80 + 閉包の実エンコード 440（11 形 × 40） + LEA 480（8 形 × 60）。比較対象は ymm0-15 の 256 ビット全体、rax/rcx/rdx/rbx/rsi/rdi/r10、メモリ 128 バイト。
- ホストの実行: インラインの機械語（intrinsic ではない）。スタブは入口で 16 本の ymm を `vmovdqu` で読み込み、命令後に全 256 ビットを書き戻し、最後に **VZEROUPPER** を実行して戻る。
- 入力: 上位・下位半分ともランダム（上位が全ゼロにならないよう、各 quadword に 1 ビットを立てる。VEX.128 のゼロ化が見える）、±0、±inf、quiet/signalling NaN、非正規数、最大値（POOL32、POOL64）。2 バイト VEX（`c5`）と 3 バイト VEX（`c4`）、レジスタ 0-15、xmm 形と ymm 形、ロード形・ストア形・レジスタコピーの両エンコード、非整列アドレス（VMOVUPS）と整列アドレス（VMOVAPS、VMOVNTPS）。MXCSR 丸めと FMA は、今回の命令が浮動小数点演算をしない（ビットのコピーのみ）ので対象外。
- 空振り防止: ホストが状態を変えたケースは各グループで 94% 以上（vmovups 3,044/3,200、vmovaps 3,046/3,200、vmovups 整列 1,526/1,600、lea 420/480、他は全件）。LEA の 60 件が変えないのは、`lea` の結果が元のレジスタ値と同じになるケース（`lea rcx, gs:[rbx]` で rbx = rcx など）と読める（推測）。
- 感度の確認（変異）: (a) VEX.128 のロードで上位をゼロ化しない（`if (wide) YH[d] = hi;`）、(b) VZEROUPPER をレジスタ 15 まで回さない（`k < 15`）。2 つを同時に入れて 13 件中 6 件が失敗（vzeroupper 60/60、sequences 50/80、closure-encodings 40/440、vmovups 400/3200、vmovups 整列 200/1600、vmovaps 400/3200。`test-vex-mutation.txt`）。元に戻して 13 件通過（`test-vex-1.txt` はその前の通過。戻した後の再実行は、下のワークスペース全体のテストの `vex.rs` 13 件）。vmovntps はストアだけで、この 2 つの変異に影響されない（失敗しないのは想定どおり）。
- 失敗: 初回の実行から全件通過で、テストの失敗はなかった。テストファイルの作成中に、命令列テストの補助関数を一度書き直した（コンパイル前）。
- 別のテスト:
  - `gs_loads_follow_the_model`: 生成 C を clang でコンパイルして実行。`gs:[0x58]` が `fle_tls_values` のアドレス、`gs:[0x30]` が `fle_teb` のアドレスを返し、`fle_teb` の 0x30、0x58 に同じ値が入り、`fle_tls_values[3]` を返されたアドレス経由で読めること。ホストとの突合はない（Linux の GS ベースはゲストの TEB ではない）。
  - `other_fs_gs_forms_stay_unsupported`: 9 形（別オフセット、32 ビット、FS、ベース付き、ストア、ALU、インデックス付き、FS/GS 付き VEX）。
  - `other_vex_and_evex_forms_stay_unsupported`: vaddps、vmovdqu、vzeroall、EVEX の vmovups/vmovaps、vshufps。
  - `misaligned_operands`: VMOVAPS/VMOVNTPS の 6 形が非整列で abort すること、16 バイト境界だが 32 バイト境界でない xmm 形は通ること、VMOVUPS の 3 形が非整列で abort しないこと。
- ワークスペース全体のテスト: 1 回目（`test-ws.txt`、`cargo test --offline --workspace --no-fail-fast`）は 900,000 ms の timeout で打ち切られた（他の担当が同時に同じ `lift/` で cargo を 4 本走らせており、テストの多くが clang を呼ぶため。ロードアベレージは打ち切り後に 1.2。原因の細かい切り分けはしていない）。`test-ws.txt` は fe-lift の `tests/reg.rs` の途中までで EXIT ファイルがない。残りを分けて実行した（フォアグラウンド）:
  - `test-ws-b`（res、sse_int2、sse_more、sse_pack、sse_scalar、sse_shuf、string、sys）: EXIT=0、8 バイナリ、112 件通過。
  - `test-ws-c`（time、uilang、ver、verify、vex、vfs、vmap、vol、x87ctl）: EXIT=0、9 バイナリ、120 件通過。
  - `test-ws-d`（fe-pe 全部）: EXIT=0、7 バイナリ、55 件通過。
  - `test-ws-e`（reg）: EXIT=0、10 件。`test-ws-f`（fe-lift doc）、`test-ws-g`（fe-decode doc）: EXIT=0、0 件。
  - 集計（`test-ws.txt`〜`test-ws-g.txt` の `test result` 行）: **55 バイナリ、502 件 passed、0 failed、0 ignored**。`fe-lift/tests/*.rs` は 38 ファイルで、すべてがいずれかの実行に入っている（`test-ws.txt` が a9_a10〜proc の 18 ファイル、b〜c が残り）。fe-decode の 4 つのテストバイナリ（dataflow、dump_tmp、extra_seed、pdata_seeds）と単体テストは `test-ws.txt` に入っている（`test-ws.txt` の 1 回目の cargo が fe-decode と fe-lift を先に走らせたため）。
  - 前回（`mmx-run.md`）は 48 バイナリ、371 件。差は +7 バイナリ、+131 件。今回の `vex` は +1 バイナリ、+13 件。残りは他の担当のテスト（`a9_a10`、`glob`、`load`、`vmap`、`vol`、`pdata*` など、git status の未追跡ファイルと一致。推測）。

## 4. 生成、clang、リンク

- 生成器: `out-vex/tool`（`out-mmx/tool` の複製）を `out-vex/target-tool` にビルド（`build-tool.txt`、EXIT=0）。seed は `out-vex/seeds-896.txt`（896 件、`out-mmx` の複製）。
- 生成: EXIT=0（`gen-vex-896.txt`、`.err` は空）。functions 8427、closure_instructions 497367、emitted 497527（前回と同じ）。c_lines 548457 → 548610。
- **`unsupported_total`: 340 → 266**（−74）。`unsupported` の種類は 9 → 3。差の内訳（`fle_unsupported` 行の「VA と mnemonic」を `sort` して `diff`。追加された行は 0 件、消えた行は 74 件）:
  - **今回の変更による減少は 20 件**（`gen-vex-896.txt` の handled に出る: `vmovups` 12、`vmovaps` 2、`vmovntps` 2、`vzeroupper` 2、`lea_seg` 1、`mov_gs` 1。合計 20）。
  - 残りの −54 は `Jmp: indirect through IAT slot` が 299 → 245。ワークツリーの `imports_table.rs`（未コミットの +57 行）と、`fle_glob`、`fle_user_nowin`、`fle_vmap`、`fle_vol`、`fle_winmm`、`fle_load`、`fle_imagehlp` などの別担当の変更による（リンクが、これらの runtime 7 つのシンボルを要求したことから）。**今回の変更によるものではない。** 340 − 20 − 54 = 266 で算術は一致。
- 残り 266: `Jmp: indirect through IAT slot` 245、`Jmp: indirect target unresolved` 20、`Xgetbv` 1。
- 生成 C の該当行（`after-vex-896.c`）: `L_409da0: fle_vld(1, (R[1]), 1, 0);`、`L_409dda: R[0] = (R[0]);`、`L_409de5: fle_vst((R[2] + (R[0]*1)), 0, 1, 1);`、`L_409df9: fle_vzeroupper();`、`L_409e05: fle_vst(..., 0, 1, 1);`、`L_41d14b: R[0] = fle_gs_ld8(UINT64_C(0x58));`。
- clang -O0 とリンク: `out-vex/build.sh`（`out-mmx/build.sh` の複製）。`EXTRA_RT` は int-more2-run.md §4 と同じ一覧に `fle_res fle_glob fle_imagehlp fle_load fle_user_nowin fle_vmap fle_vol fle_winmm` を足した。1 回目は `fle_res` の指定を含めても、別担当の追加 runtime の 51 件の `fle_imp_*`（glob 11、imagehlp 1、load 5、user_nowin 12、vmap 5、vol 7、winmm 10。未定義参照の一覧を `grep` で取り、各 runtime の定義に当てた）が未定義でリンク失敗、2 回目に足して EXIT=0（`build-896.txt`、警告 0）。`fle_vex.h` は header only で、`.o` は増えていない。

## 5. 実行

コマンド: `./bin/flengine-after-vex-896 ../../binaries/win64/FLEngine_x64.dll`（`out-vex/` で）。出力 `run-vex-896.txt`、`.err`、`.exit`（EXIT=134、SIGABRT）。

| 項目 | 前回（mmx） | 今回 |
|---|---|---|
| 停止点 | `fle: indirect call to 0x40f960 is not a known function entry` | 同じ |
| 関数実行数（distinct） | 125 | 125 |
| 関数エントリ数（calls） | 658 | 658 |
| 呼び出し深さ | 8 | 8 |

- 停止点も数も同じ。これは、今回の命令が実行で踏まれていないことと整合する（下）。`run-mmx-896.txt` と `run-vex-896.txt` の `diff` は、スタックの値（`rsp=`）の 1 行だけ（`rsp=` を消して比べると差は 0 行）。
- **今回実装した命令は、この実行では 1 つも実行されていない。**
  - AVX 版コピー（0x409da0 以降）に入る条件は `[0x15e60b0] & 1`（0x409d1d、0x409e1a）。この値は 0x409bf0 の 0x409c0f だけが書き、元は `[0x15e69c0] & 0x10000000`。`0x15e69c0` は、関数 0x40f4b0（実行ログの entry 19、実行済み）が `cpuid(1)` の ECX（`0x15e69b8 + 8`）を書く場所（0x40f4b0〜0x40f4f3 の命令列。前任の記録で未確認だった出所を、今回確定）。`fle_cpuid.c` は葉 1 の ECX を 0 で返すので、`0x15e60b0` は 0 になり、AVX 版に入らない。f_409c80 自体は実行ログに 11 回出る（SSE 版を通っている）。
  - `0x41d14b`（`gs:0x58`）は、0x41d114 の `cmp BYTE [0x15ec2d8],0` が 0 のときだけ。0x15ec2d8 は 0x41d912 が 1 を書く。0x41d8f0 は実行ログの entry 2 に出ており、`r9d == 1`（DllMain の reason）なら 0x41d912 に届く。呼び出し元ハーネスは `R[8] = 0`、`R[2] = 1` で、r9 の値はハーネスが設定していない（読んで確認。実行では 0x41d912 を通ったかは確認していない）ため、GS 経由の分岐を通ったかは未確認。停止点が同じことから、少なくとも今回の実行で `fle_gs_ld8` が abort した、ということはない（abort のメッセージは別。実測）。
  - したがって、実装した命令の正しさの根拠は、ホスト突合のテスト（§3）だけ。実行では確かめられていない。
- 0x40f960 の間接呼び出しは前回から変わらない（今回の作業の対象外）。

## 6. XGETBV（実装しない。判断は決定 0032）

- 0x639a03 は関数 0x6399e0 の中。呼び出し元は 0x639beb の 1 件（`call 0x6399e0`）。分岐: `cpuid(0)`、`eax < 1`（`jb`）、`cpuid(1)`、`ecx & 0x18000000 != 0x18000000`（`jne` 0x639a15）、`xor ecx,ecx`、`xgetbv`、`eax & 6 == 6`。ビット 27（OSXSAVE）とビット 28（AVX）がどちらも立たなければ、XGETBV の前の `jne` が取られる。
- `fle_cpuid.c` は葉 1 の ECX を 0 で返すので、XGETBV は実行されない（静的な読み。この関数を実行ログで見ていない）。したがって、`fle_unsupported` のまま残しても、現在の CPUID の前提では実行に影響しない。

## 7. 残りの未対応と、今後に残る点

- `unsupported_total` 266: IAT 経由の間接 jmp 245、解決不能な間接 jmp 20、Xgetbv 1。
- VEX の他の命令（VEX の算術、シャッフル、整数）は閉包に 0 件で、実装していない。
- CPUID が AVX を主張する場合は、XGETBV の値の判断と、`#UD` のモデル（CR4.OSXSAVE、XCR0）が先に要る。

## 8. 取得物・失敗・拒否された操作

- 外部への通信: Vergilius Project の `_TEB`（x64、Windows 11 24H2）のページと Microsoft Learn の PE Format のページを読んだ（取得のみ、ファイルへの保存なし）。SDM は `lift/out-int2/spec/sdm-vol2.txt`（既存）を読んだ。公式 PDF の再取得はしていない。
- 拒否された操作: なし。
- 失敗: §4 のリンク 2 回（1 回目失敗）、§3 のワークスペース全体の 1 回目（timeout）、`--doc` と `--test` の同時指定（cargo のエラー。指定を分けて解決）、分割実行の最初の呼び出しで、サブシェル内の相対パスのリダイレクトが失敗（`out-vex/` が見つからない。カレントがずれたため。絶対パスで再実行して解決）。
- 同時に走っていた他の担当の `cargo test`（`out-a9`、`out-pdata`、`out-apis`、`out-a6`）のプロセスには触れていない。
- `out-vex` 全体で 233 MB（`target-tool` 26 MB、`test` 31 MB、ほかはオブジェクトと生成 C）。2 GB 級のファイルはない。
