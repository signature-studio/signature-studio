# MXCSR 状態ビット・RC/FTZ/DAZ の標準 C 実装（0044）の実行記録

日付: 2026-10-21。判断は `docs/decisions/0044-mxcsr-flags-portable.md`（0038 の改訂）。「実測」はコマンドの出力、「推測」は未検証の解釈。HEAD 1f88ad5、ワークツリーには他担当の未コミット変更あり（wasm 移植: `fle_sys.c` 他、`fle_runtime.h` の `fle_addr`/`FLE_CHECK_HI32`、`fle_host.c`/`fle_host.h`、x87 超越関数 `fle_x87.c` と `x87.rs`、ロケールなど）。それらは触っていない。成果物は `lift/out-mxsoft/`（`tmp/` = TMPDIR、`emcache/` = EM_CACHE）。

## 1. stmxcsr 3 か所の静的確認（`lift/objdump-text.txt`、FLEngine_x64.dll）

| stmxcsr | 関数 | 内容 | 読んだ値の状態ビット（bit 5:0）で分岐するか |
|---|---|---|---|
| 0x40a388 | 0x40a380 | `ecx & 0x3f`、現在の MXCSR の `& 0xffc0`、or、`ldmxcsr`（状態ビットだけを引数に置き換える書き込み関数。呼び元は 0x40a1a6 の 1 か所） | 分岐しない（読んだ値の状態ビットは `and 0xffc0` で捨てる） |
| 0x40a3b2 | 0x40a3b0（`push 0; stmxcsr [rsp]; pop rax; ret`。MXCSR を返すだけのラッパー） | 呼び元 10 か所（下表） | **1 か所で分岐する**（下） |
| 0x475c61 | 0x475c60 | RC（bit 14:13）だけを `cl & 3` に変更。戻り値は旧 RC（`shr 13; and 3`） | 分岐しない（RC のみ） |

0x40a3b0 の呼び元 10 か所（`call 0x40a3b0` の grep）:

| 呼び元 | 関数 | 使い方 | 状態ビットを分岐に使うか |
|---|---|---|---|
| 0x40a18e | 0x40a180 | `eax & 0x3f`、`edi = eax & esi` を戻り値にし、`(旧 & ~esi) \| (ebx & esi)` を 0x40a380 で書く（指定した状態ビットの「読んで置き換える」） | 戻り値は edi。呼び元（0x40a1c0 内の 6 か所: 0x40a1f7、0x40a214、0x40a230、0x40a24c、0x40a268、0x40a283）は eax を使わない（直後は `test ebx,ebx`、`mov cl,N; call 0x409730` など）。**分岐に使わない** |
| 0x40a2a4 | 0x40a2a0 | `eax & 0x6000`（RC を返す） | RC のみ |
| 0x40a2d1 | 0x40a2c0 | マスクビット（`edx & 0x1f80`）の「読んで置き換える」。旧マスクビットを返す | マスクビット（bit 12:7）。状態ビットではない |
| 0x40a307 | 0x40a300 | `MXCSR & ecx` を返す。呼び元 0x40a1de は `ecx = (ebx << 7) & 0x1f80`（マスクビット）で、直後に `test eax,eax; je` | マスクビットで分岐。状態ビットではない |
| 0x40f124 | 0x40eec0 付近の関数 | `(MXCSR & 0xfffff7ff) \| (edi & 0x800)` を `ldmxcsr`（UM ビットだけを元に戻す） | 分岐しない |
| 0x41189b | 0x411xxx の関数 | `[rcx+0x34] = MXCSR`（保存） | 保存のみ（この関数内で分岐なし） |
| 0x412281 | 0x4121xx の関数 | `[0x13a5054] = MXCSR`（`[0x15e6078] == 0` のとき。ldmxcsr の元データ） | 保存のみ。後で 0x40a360 が `& 0xffc0` で状態ビットを消して読み込む（0x40f530 は 0x13a5054 をそのまま読み込む） |
| **0x4413c7** | **0x4413c0** | `eax = MXCSR & 0x3f; test ebx(引数), eax; sete bl` を返す（「引数のビットがどれも立っていないか」）。続けて 0x40a340 を呼ぶ | **分岐に使う**（下） |
| 0x44175d | 0x441730 付近 | `[rbp+0x68] = MXCSR`（保存）。0x4418be/0x4418eb で 0x40a360（`ecx & 0xffc0`、状態ビットは消して復元） | 保存と復元のみ |
| 0x477704 | 0x477700 | `(MXCSR >> 13) & 3`（RC を返す） | RC のみ |

0x4413c0 の呼び元: 0x441767（`ecx = 0`: `test 0, eax` なので常に 1。分岐への影響なし）、**0x441896（`ecx = 9` = IE（bit 0）\| OE（bit 3））**。0x441896 の直前は `cvtsi2sd xmm1, eax; mulsd xmm0, xmm1; movsd [rbp+0x50], xmm0`（0x441876–0x44187e）で、戻り値 al を `[rbp+0x67]` に保存し、0x44189e の `cmp BYTE PTR [rbp+0x67], 0; jne 0x4418ba` で分岐する（0 のときは 0x4413f0 を呼び、その結果を再び `[rbp+0x67]` に入れる）。つまり**ゲストは、直前の掛け算（と 0x40ef40 の呼び出し）で IE または OE が立ったかを見て分岐する**。

結論（事実）: 3 か所の stmxcsr のうち、状態ビットで分岐するのは 0x40a3b0（0x40a3b2）経由の 0x4413c0 → 0x441896 の 1 経路で、必要な状態ビットは **IE と OE**（mask 9）。0x40a1c0 系（0x40a180、0x40a2c0、0x40a300）が読むのはマスクビット（bit 12:7）で、これらを非マスクにしたゲストは #XM（abort）の経路になる。本調査で未確認: 0x441896 の経路が実際に実行されるか、0x4413c0 の呼び出し直前に 0x40a340 が MXCSR を消す条件（`[0x15e60ac] != 0`）が成り立つか、0x41189b/0x412281 の保存値が後で状態ビットの分岐に使われるか（0x412281 の保存値は 0x40f530 が `ldmxcsr` するだけで、分岐は見ていない）。**実装は全フラグを正しく実装した**ので、この確認の結果に実装は依存しない。

## 2. 実装

`lift/runtime/fle_mxcsr_flags.h`（旧 214 行 → 新 964 行）を、編集ツールで内容を置き換えた（`>` は使っていない）。方式は決定 0044 §2。asm、`<emmintrin.h>`、`<fenv.h>`、float/double の演算、`long`、`uintptr_t` なし（ヘッダ中の `__builtin_clzll` は GNU 系のみ。それ以外は手書き代替）。対象は 0038 の 60 ニーモニックのうち FP 演算の **56**（PREFETCHh 4 は変更なし）。API（`fle_mx_*`、`FLE_MX_CMPSS` 等、`fle_mx_m32`/`fle_mx_m64`）は同じ。生成 C は変更なし（§6）。

ホスト実行版は `lift/crates/fe-lift/tests/ref/fle_mxcsr_flags_host.h`（名前を `fle_hx_*`/`FLE_HX_*` に替えたもの。`fle_mxcsr` は extern）。

## 3. 突合（ホスト版との比較）

### 3.1 既存の突合（`tests/mxcsr_flags.rs`、生成 C → clang -O0 → ホストの機械語スタブ）

純 C 版に対して、**差 0**: arith 42,672（112 エンコーディング）、cmp 15,360（128）、comi 11,872（16）、cvt_fp 16,000（40）、cvt_int 15,600（44）、round 25,600（256）。計 **127,104 ケース、596 エンコーディング**（0038 と同じ数）。全 RC × FTZ × DAZ、事前の sticky ビット、NaN/SNaN/±0/±inf/非正規化数/オーバーフロー境界/不正確。比較は XMM、GPR、MXCSR 全ビット、COMIS の ZF/PF/CF/OF/SF/AF、触ってはいけないレジスタ・メモリ。

### 3.2 新規: ランダムな 32/64 ビットパターン（`random_pure_matches_host_ref`、`tests/ref/mxcsr_diff.c`）

166 バリアント（add/sub/mul/div/min/max/sqrt × ss/sd/ps/pd = 28、cvt FP→FP 10、cmp 述語 0..7 × 4 = 32、round imm8 {0..16, 130, 245, 255} × 4 = 80、cvtsi2ss/sd × 32/64 = 4、cvt[t]ss/sd2si × 32/64 = 8、comis/ucomis 4）× **各 100,000 ケース = 16,600,000 ケース、不一致 0**。純 C 版とホスト版を同じプログラムに入れ、XMM（128 ビット）、GPR 値、MXCSR 全ビット、COMIS の EFLAGS を比較。入力は完全乱数（1/10）と端の値の生成（指数 0/1/最大/最大−1、NaN ペイロード、タイのパターン、第 2 オペランドを第 1 オペランドから作る: 数 ulp 違い、符号反転、アライメント距離、積・商がアンダーフロー/オーバーフロー境界、整数変換の境界 2^24/2^31/2^52/2^53/2^63）。RC、FTZ、DAZ は毎ケース乱数、4 ケースに 1 回は事前に sticky フラグ。全 166 バリアントで、ホストが IE DE ZE OE UE PE（0x3F）をすべて立てた（`raised_union`）。テスト時間 約 6 秒（`-O1`）。

最初の実行（2,000 ケース/バリアント）で差が出た 3 件（実装の誤り、修正済み）: (a) `divsd` の `x/0` で、x が非正規化（DAZ なし）のときに DE を立てていた（ホストは ZE のみ）。(b) min/max で、DAZ が 0 にした非正規化オペランドを元のビット列のまま返していた（ホストは同符号のゼロ）。(c) 同じく NaN の分岐（第 2 オペランドを返す）でも同符号のゼロにしてから返す必要がある（(b) の修正を比較より前に移した）。

### 3.3 変異テスト（`out-mxsoft/mutants.py`、ヘッダの複製に 1 か所ずつ変更を入れて `cargo test`）

最初の 34 件: 33 検出、**1 件生存**（「FTZ が UM を見ない」）。原因: UM=0 のとき tiny で正確な結果に UE が立つか、が実装とテストの両方で未定義だった（ホストでは UM=0 だと #XM になり突合できない）。SDM Vol.1 §4.9.1.5（UM=0 のとき tiny な結果は不正確かどうかによらず UE）に従い実装を直し、`unmasked_raised_flag_is_a_named_abort` に UM=0 のケース（正確な tiny 積で abort、`raised flags 0x10`、FTZ+UM=0 でも FTZ は効かず UE のみ、DM=0 の非正規化入力で abort）を足し、変異を 1 件足して **35 件中 35 件検出**（`mutants-run2.txt`）。変異の内容: タイの偶数丸め、RC の取り違え、DAZ のビット位置、FTZ（UM 無視、無効）、tininess の判定、UE の条件、DE、NaN の選択、既定 NaN の符号、SNaN の QNaN 化、RC 0 方向のオーバーフロー、x−x の符号、0+0 の符号、非マスク abort の削除、状態ビットの sticky、min/max の IE、cmp の述語と IE、round の PE/RC、cvt の範囲/RC、COMIS の CF/IE、div の ZE/剰余 sticky、sqrt の sticky、加算の位置合わせ sticky、cvtsi2 の符号拡張、パックの 1 レーン欠落、min/max の DAZ、cvtpd2ps の上位ゼロ化。

## 4. 性能（ネイティブ -O0、`out-mxsoft/bench.c`、ns/演算、2,000,000 回の平均、最終実行 1 回。実行ごとにばらつく）

ホスト実行版（0038、`bench-host.txt`）と純 C 版（`bench-pure.txt`）:

| 演算 | ホスト版 | 純 C 版 | 差 |
|---|---|---|---|
| addsd 正確（1+1） | 15.6 | 29.2 | +13.6 |
| addsd 不正確（1+1/3） | 15.3 | 32.0 | +16.7 |
| mulsd | 15.5 | 24.6 | +9.1 |
| divsd | 17.6 | 26.9 | +9.3 |
| sqrtsd | 18.9 | 119.9 | +101 |
| addps | 15.3 | 88.5 | +73 |
| mulpd | 15.6 | 44.2 | +29 |
| minsd | 15.1 | 16.5 | +1.4 |
| cvtsi2sd64 | 11.9 | 11.1 | −0.8 |
| cvttsd2si64 | 11.8 | 11.1 | −0.7 |
| cvtsd2ss | 15.2 | 16.5 | +1.3 |
| ucomisd | 15.8 | 12.9 | −2.9 |
| roundsd 9 | 11.4 | 11.8 | +0.4 |
| cmpltsd | 13.5 | 15.5 | +2.0 |
| addsd 不正確、RC=1 | 15.0 | 31.2 | +16.2 |

0038 の記録（`mxcsr-flags-run.md` §2.2）は、状態ビットなしの旧実装に対して addsd 5.1 → 16.4 ns（+11〜13 ns）。今回のホスト版は同じ方式で 15〜16 ns（マシンの負荷で 15〜21 ns の幅があった）。純 C 版は、加減乗除で更にスカラー +9〜17 ns、sqrt と パック形は大きい（sqrtsd 約 120 ns、addps 約 89 ns）。変換・比較・round・min は同じ程度。**スカラー加減乗除は旧実装（状態ビットなし）に対して約 +20〜27 ns**（推測: 旧実装 5.1 ns との差を直接測っていない。今回のホスト版と同じマシン条件の旧実装は測っていない）。生成 C に約 50,000 か所の呼び出し（`fle_mx_` を含む行 49,997）があるが、プログラム全体の実行時間は測っていない（FLEngine はまだ最後まで走らない）。

## 5. wasm

### 5.1 `emcc -c`（emcc 6.0.12、`-O0`、同時 2 本。EM_CACHE は `out-mxsoft/emcache`、`out-wasm/emcache` を複製したもの 29 MB）

| 対象 | 結果 |
|---|---|
| runtime の全 `fle_*.c` 32 本、フラグなし | **31 成功 / 1 失敗**（失敗は `fle_sse_more.c`: `fle_sse.h` の `#include <emmintrin.h>` が `"SSE2 instruction set not enabled"`） |
| runtime の全 `fle_*.c` 32 本、`-msse2 -msimd128` | **32 / 32 成功**。警告 2 件（既存: `fle_proc.c:105`、`fle_x87.c:1027`） |
| 生成 C（全 pdata、`fe-lift lift --pdata`）、**2 分割**（29,500 関数/ファイル）、フラグなし | 0 / 2（`fle_sse.h` の `emmintrin.h`。0044 §4。フラグなしではそれだけが原因） |
| 生成 C 2 分割、`-msse2 -msimd128` | **2 / 2 成功**（0 エラー）。`.o` 74,471,111 + 95,214,554 = **169,685,665 B**。時間 37.1 秒と 48.1 秒（同時 2 本、合計 48.1 秒）。**最大 RSS 8.3 GB と 10.8 GB**（`time -f %M`。同時 2 本で合計約 19 GB、マシンの RAM は約 15 GB で、通ったが余裕はない）。16 分割では 1 本あたり 1.8 GB（wasm-build-probe §4.1）。1〜2 分割にすると 1 本のメモリが約 5 倍になる、が実測 |

フラグ `-msse2 -msimd128` が必要な理由は `fle_sse.h`（触っていない）の `<emmintrin.h>` と `fle_sqrtss`/`fle_sqrtsd`（生成 C では未使用: 生成 C に 0 件）。`fle_mxcsr_flags.h` 自体は両フラグなしでコンパイルできる（`fle_sse.h` を除いた単体では未確認。`fle_sse.h` が先に失敗する）。**報告**: `fle_sse.h` の 2 関数を整数実装に替えて include を外せば、フラグなしで通る（推測）。

### 5.2 node（wasm）上の突合

`tests/ref/mxcsr_diff.c` を emcc（`-msse2 -msimd128 -O0`、`-sENVIRONMENT=node -sNODERAWFS=1`）でビルドし、node で実行。wasm にはホスト SSE がないので、ホスト参照側は純 C 版を `fle_hx_*` に名前替えしたもの（`out-mxsoft/wasm/alias/`）で、比較の実体は**出力の FNV-1a ハッシュ**: 166 バリアント × 100,000 ケース = **16,600,000 ケース**（依頼の「数千ケース」より大きい。同じプログラム、同じ乱数列）。

- wasm 上の純 C 版 の 166 ハッシュ = native の純 C 版の 166 ハッシュ = **native のホスト版（asm）の 166 ハッシュ**、166/166 一致（`diff` で差なし。`difft/n_pure.txt`、`n_ref.txt`、`w_pure.txt`）。wasm 内での 2 版の差 0（`TOTAL cases 16600000 mismatches 0`）。実行時間 13.8 秒（native -O0 は 5.7 秒）。
- `long`/`uintptr_t` の幅: ヘッダは `long`、`uintptr_t` を使わない（`grep long` はコメントの 1 行のみ）。wasm32（`sizeof(long)` = 4）でもネイティブと一致。64 ビット整数は `uint64_t`、128 ビットは `unsigned __int128`（clang が compiler-rt の `__multi3`、`__udivti3` で実装）。
- `clang -std=c99 -fsyntax-only`（ヘッダ単体）通過。`gcc -std=c99` 通過。`clang -m32` は環境に 32 ビットのヘッダがなく未確認。

## 6. 生成 C と .o のサイズ

| | 変更前（0038、ホスト実行版） | 変更後（0044） |
|---|---|---|
| 生成 C `c_bytes`（all-pdata） | 215,539,533（`mxcsr-flags-run.md` §2.1） | **215,539,555**（+22。他担当の x87 `F2xm1`/`Fsincos` の変更の分。`fle_mx_` を含む行は、`out-wasm/out/wasm-pdata.c`（変更前に生成）と全行が同一、`diff` の差は x87 の行のみ） |
| `fle_mx_` を含む行 | 49,997 | 49,997 |
| `c_lines` | 2,880,419 | 2,880,419 |
| `unsupported_total` | 603 | 601（x87 の分と推測） |
| 16 分割 C（3,700 関数/ファイル）合計 | 234,300,124 | 234,300,146 |
| 16 分割 .o（clang -O0 -g、並列 8）合計 | 523,470,768（旧ヘッダ、同じ生成 C を旧 `fle_mxcsr_flags.h` でコンパイルして今回測り直した値。0038 の記録は 531,380,416 だが生成 C が違う） | **523,785,448**（+314,680、+0.06%） |
| 2 分割 wasm `.o`（emcc -O0） | — | 169,685,665 |

`sse_fp_flags.rs` はコメント 1 か所のみ変更（動作変更なし）。生成 C は変わらないので、呼び出し側（`fle_mx_*` の名前と署名）は同じ。

## 7. テスト

- `cargo test --release --offline --workspace --no-fail-fast`（フォアグラウンド、`out-mxsoft/test-ws.txt`、`CARGO_TARGET_DIR=out-mxsoft/target`）: 64 バイナリ、**pass 604、fail 6、ignored 0**。失敗 6 件は `int_ext`（`movsx_matches_host`、`idiv_imul_matches_host`）と `int_more`（`xadd_matches_host`、`cmpxchg_matches_host`、`div_matches_host`、`bt_matches_host`）。0038 の記録（`mxcsr-flags-run.md` §2.3）と同じ 6 件で、同じ名前。メモリオペランドを含む整数命令の突合で SSE とは無関係（失敗の例: `movsx eax, byte [rdi+1]` でレジスタ値が C 左 101765575405648、host 右 125561156387952 とポインタ値らしい 1 つの GPR が違う。原因は調べていない。推測: 他担当の変更中のテストか、スタック/ヒープアドレスをレジスタに比較しているテストの問題）。今回の変更とは無関係である確認（変更を外した複製での再現）は、0038 の `test-ctl-int.txt` の結果に依拠しており、今回は取り直していない。
- 前回（0038）の 593 pass、6 fail に対し +11 pass（`mxcsr_flags` の新規 1 件、他担当のテスト）。
- `tests/mxcsr_flags.rs`: 10 件（0038 の 9 + 新規 1）すべて pass（単独 約 9 秒）。ETXTBSY 対策として `mod etxt_common;` を足し、`compile` のファイル名を `etxt_common::uniq` にした（流儀は `sse_int2.rs` と同じ）。テストの出力先を `out-mxcsr/test` から `out-mxsoft/test` に変更。環境変数 `MXCSR_MUTANT_RT`（runtime 代替ディレクトリ）、`FLE_MXCSR_DIFF_SCALE`（ケース数）を追加。

## 8. 制約の遵守と、守れなかった点

- `rm`: **1 回、誤って実行した**。wasm の突合の途中で、シェルのコマンド列に `rm -rf /dev/null 2>/dev/null;` を書いてしまった（意味のないコマンド。作業のための意図はない）。非 root（uid 1000）なので削除は失敗し、直後に `ls -l /dev/null` で `crw-rw-rw- root root 1,3` のまま存在し書き込めることを 2 回確認した。ファイルの削除は起きていない（コマンドの意図としては規約違反）。それ以外に `rm` は使っていない。
- 上書き `>`: 既存ファイルには使っていない（`fle_mxcsr_flags.h` は編集ツールで置き換え）。リダイレクトで書いたのは新規ファイルのみ。`out-mxsoft/mutants.py`（自分で作ったファイル）には `sed -i` を 1 回、`tests/ref/mxcsr_diff_ops.h`（自分で作ったファイル）には `sed -i` を 1 回使った（既存の他者のファイルには使っていない）。
- git 操作なし、`/tmp` 不使用（TMPDIR は `out-mxsoft/tmp`、EM_CACHE は `out-mxsoft/emcache`）、sleep/timeout をコマンド文字列に含めていない。emcc は同時 2 本まで。権限拒否なし。
- 2 GB 級の中間ファイルなし（最大は生成 C 215 MB、.o 95 MB）。`out-mxsoft/` 全体は約 6.8 GB（cargo の `target` と、生成 C とその分割 2 種（2 分割と 16 分割）、.o の 3 セット（clang 新旧、emcc）、`mut/`・`mut2/` のヘッダ複製が主。`out-pdata`、`out-wasm` の大きなファイルの複製はなし。ただし `out-wasm/emcache` 29 MB は複製）。
- 変異テストの 1 回目（`mut/`）は 2 回目（`mut2/`）で作り直した（スクリプト内のディレクトリ名変更。1 回目の `mut/` は残置、削除していない）。

## 9. 触ったファイル

変更（既存、編集ツール）: `lift/runtime/fle_mxcsr_flags.h`（内容を置換）、`lift/crates/fe-lift/src/sse_fp_flags.rs`（コメント）、`lift/crates/fe-lift/tests/mxcsr_flags.rs`（etxt_common、新規テスト、出力先、MUTANT 変数）。
新規: `lift/crates/fe-lift/tests/ref/fle_mxcsr_flags_host.h`、`ref/mxcsr_diff.c`、`ref/mxcsr_diff_main.c`、`ref/mxcsr_diff_ops.h`、`docs/decisions/0044-mxcsr-flags-portable.md`、本ファイル、`lift/out-mxsoft/` 以下すべて（`mutants.py`、`bench.c`、`wasm/`、`gen/` など）。
触っていない: `fle_sse.h`、`fle_runtime.h` と他担当の変更（wasm 移植、x87、ロケール、ETXTBSY の tests/ 局所変更）、`lib.rs`、`sse_more.rs`。
