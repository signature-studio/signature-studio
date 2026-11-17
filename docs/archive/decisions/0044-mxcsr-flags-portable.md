# 0044: SSE 浮動小数の MXCSR 状態ビット・RC/FTZ/DAZ をホスト非依存の標準 C で実装する（0038 の改訂）

状態: 作業者が決定。実行記録は `docs/investigations/mxcsr-portable-run.md`。0038 §1（方式）を置き換える。0038 のそれ以外（対象命令、SDM の例外節との対応、PREFETCHh、限界のうちホスト CPU に関係しない部分）は有効のまま。0038 は残す。

## 1. 動機

最終ターゲットは C → emcc → wasm。0038 の `lift/runtime/fle_mxcsr_flags.h` は、命令ごとにホスト CPU のインライン asm（`ldmxcsr`/`stmxcsr`、SSE 命令の直接実行、`"=@ccc"`）で実行して MXCSR 状態ビットを取っていた。wasm にはこれらがなく、`emcc -c` は生成 C 16 分割すべてで失敗した（`docs/investigations/wasm-build-probe.md` §4.1）。状態ビットなしのソフト版 shim は RC/FTZ/DAZ を無視し、状態ビットを立てなかった（同 §3.2 案 A）。

## 2. 方式の選択と理由

選択: **整数演算によるソフト浮動小数**（`unsigned __int128` 可、float/double の演算は使わない）。

| 案 | 内容 | 採否と理由 |
|---|---|---|
| A. double の演算 + 誤差なし変換で不正確を検出 | add/sub/mul は double で計算して TwoSum/FMA で誤差を出し PE を判定 | 不採用。RC≠最近接偶数、FTZ、DAZ は C の通常の浮動小数演算では出せない（RC は結果自体が変わる）。結局整数実装が要る。さらに、wasm の f32/f64 は NaN のビット列（ペイロード、符号）の選択が実装依存で、wasm-build-probe §4.5 で NaN 入力の `round*` が native と wasm で一致しなかった実績がある。全部の NaN 規則を整数側で決めるなら浮動小数演算は使わない方が一貫する。 |
| B. 整数のソフト浮動小数（採用） | 各オペランドを（符号、正規化した 64 ビット仮数、指数、種別）に展開し、add/mul/div/sqrt を 128 ビット整数で行い、1 か所の丸め・パック関数（`fle_mx_pack`）で RC、FTZ、OF/UF/PE を決める | 採用。ホストの FP、MXCSR、`long`/`uintptr_t` の幅に依存しない。RC、FTZ、DAZ、tininess after rounding、NaN の選択を明示的に書ける。 |
| C. ホスト実行を残し、wasm 用だけソフト | 2 系統になり、wasm 側の検証が native と別物になる | 不採用。1 系統にして、ホスト実行版をテスト専用のリファレンスにすると、実行系統のずれがテストで検出できる。 |

`div` は `a.m << 64` を 64 ビットの `b.m` で割る（128 ビット商）、`sqrt` は 128 ビット被開平数のビット単位の整数平方根（`fle_mx_isqrt`。関数呼び出しなしの 64 反復。遅いが正確）。`__int128` の除算・乗算は wasm32 では compiler-rt（`__udivti3`、`__multi3`）になり、コンパイルと実行はできる（wasm-build-probe §D12 で native と一致を確認済み）。

実装は `runtime/fle_mxcsr_flags.h`（ヘッダのみ、964 行）。C99 + `<stdint.h>` + `unsigned __int128`。`__builtin_clzll` は GNU 系で使い、それ以外は手書きの代替（`FLE_MX_PORTABLE_CLZ` で強制できる）。asm、`<emmintrin.h>`、`<fenv.h>`、float/double、`long`、`uintptr_t` は使わない（このヘッダの中では。`fle_ld` は `fle_runtime.h` のもの）。

## 3. 仕様（実装したもの）

- 結果ビット: IEEE 754 binary32/binary64。NaN: 第 1 オペランド（dst）が NaN ならそれ、そうでなければ第 2 オペランドの NaN。SNaN は QNaN 化（最上位の仮数ビット）。無効演算（NaN なし）の結果は既定 QNaN（`0xFFC00000`、`0xFFF8000000000000`）。min/max は「どちらかが NaN なら第 2 オペランド」で、IE を立てる（QNaN でも）。DAZ で 0 になった非正規化数は、同符号のゼロとして返す（min/max）。
- RC（bit 14:13）: add sub mul div sqrt、変換（cvt*2si、cvt*2dq、cvtsd2ss、cvtsi2*）、ROUND（imm8 bit 2 が 1 のとき MXCSR の RC、0 のとき imm8 bit 1:0）。TRUNC 系（cvtt*）は常に 0 方向。
- DAZ（bit 6）: 非正規化の入力はゼロ（同符号）で DE を立てない。
- FTZ（bit 15）: 結果が tiny（丸めた後で判定）かつ UM（bit 11）が 1 のとき、同符号のゼロにして UE と PE を立てる。UM が 0（UE が非マスク）なら FTZ は効かず、tiny な結果はすべて UE（不正確なら PE も）で、#XM になる（実機の SDM Vol.1 §4.9.1.5 による。**この UM=0 の経路は実機との突合ができていない**: ホストが #XM になるため。SDM に基づく単体テストだけ）。
- 状態フラグ IE DE ZE OE UE PE（bit 5:0）を `fle_mxcsr` に OR（sticky）。パック形は全レーンのフラグの OR を 1 回で反映。
- 立ったフラグのマスクが外れている（bit 7+n が 0）ときは名前付き abort（メッセージは 0038 と同じ）。abort は、この命令の結果を書く前（`fle_mx_fin` が戻り値の組み立ての前に呼ばれる）。
- COMISS/COMISD/UCOMISS/UCOMISD: ZF/PF/CF を設定、OF/SF/AF=0。COMIS は QNaN でも IE、UCOMIS は SNaN のみ。DE は順序付きのときのみ（ホストと一致を確認）。
- 変換: 範囲外、NaN、無限大は整数不定値（`0x80000000`、`0x8000000000000000`）で IE（PE は立てない）。cvtsd2ss/cvtss2sd の NaN は上位の仮数ビットを保ち QNaN 化。
- 対象 56 ニーモニック（add sub mul div min max sqrt の ps pd ss sd = 28、cmp ×4、comis/ucomis ×4、round ×4、浮動小数どうしの変換 ×10、整数との変換 ×6）。0038 の 60 ニーモニックのうち PREFETCHh 4 は FP 演算ではなく、変更なし。

## 4. API

変えない。`sse_fp_flags.rs` が生成する呼び出し（`fle_mx_*`、`FLE_MX_CMPSS` 等、`fle_mx_m32`/`fle_mx_m64`）の名前と署名はそのまま。生成 C は変わらない（`fle_mx_` を含む行は、変更前に生成した C と全行が同一。差分は他担当の x87 の変更だけ）。`sse_fp_flags.rs` はコメント 1 か所のみ変更（動作なし）。`fle_sse.h` は触っていない。

**wasm でコンパイルするには `-msse2 -msimd128` がまだ要る**: `fle_sse.h`:13 の `#include <emmintrin.h>` と `fle_sqrtss`/`fle_sqrtsd`（`_mm_sqrt_ss`、`_mm_sqrt_sd`、`fle_sse.h`:128-136）が残っているため（これらは旧 `sse_scalar.rs` のハンドラ用で、0038 以降は `sse_more.rs` が先に呼ばれるので生成 C では使われない: 生成 C に `fle_sqrtss`/`fle_sqrtsd` は 0 件）。フラグなしでは `fle_sse_more.c` と生成 C が `"SSE2 instruction set not enabled"` で失敗する。`fle_sse.h` は他担当の領域のため触らず報告する。直すなら、この 2 関数を `fle_mx_sqrtss/sd` のソフト版に置き換えて `#include <emmintrin.h>` を外す（推測: 数行）。

## 5. 検証

- ホスト実行版は `lift/crates/fe-lift/tests/ref/fle_mxcsr_flags_host.h` に残した（名前を `fle_hx_*`/`FLE_HX_*` に替えただけ。`runtime/` には置かない）。
- 0038 の突合（`tests/mxcsr_flags.rs`、127,104 ケース、596 エンコーディング、全 RC × FTZ × DAZ）は、同じテストがそのまま純 C 版に対して差 0 で通る（生成 C → `lift_body` → 純 C 版と、ホストの機械語スタブの比較。XMM、GPR、MXCSR 全ビット、COMIS の EFLAGS）。
- 新規 `random_pure_matches_host_ref`: `tests/ref/mxcsr_diff.c`（純 C 版とホスト版を同じ C プログラムに入れて比較）。166 バリアント（cmp の 8 述語 × 4、round の imm8 20 通り × 4、cvtsi2* と cvt*2si の 32/64 ビット、他）× 各 100,000 ケース = 16,600,000 ケース。オペランドは完全乱数 + 端の値（指数 0/1/最大、NaN ペイロード、タイ、アンダーフロー・オーバーフロー境界の積・商、桁落ち、整数変換の境界）。RC、FTZ、DAZ は毎ケース乱数、4 ケースに 1 回は事前に sticky フラグ。差 0。全 166 バリアントでホストが IE DE ZE OE UE PE（0x3F）を立てた。
- 変異テスト 35 件（`out-mxsoft/mutants.py`）。最初の 34 件のうち 1 件が生存し（「FTZ が UM を見ない」）、調べると UM=0 のときの UE の扱い（正確だが tiny な結果で UE が立つか）が実装とテストで未定義だった。実装を直し（UM=0 では tiny な結果はすべて UE）、abort のテストを足し、変異を 1 件足して **35/35 検出**。
- wasm: 16.6M ケース全部の出力のハッシュ（FNV-1a）が、wasm（emcc、node）の純 C 版、native の純 C 版、native のホスト版で 166 バリアントすべて一致。

## 6. 性能

実測は `mxcsr-portable-run.md` §4。-O0 のネイティブで、ホスト実行版（asm、15 ns 前後）に対する差（最終実行 1 回）: addsd +14、mulsd +9、divsd +9、mulpd +29、addps +73、sqrtsd +101 ns。変換・比較・round・min はほぼ同じ（−3〜+2 ns）。sqrt（ビット単位の整数平方根）とパック形（4 レーンを 1 命令ごとに 128 ビット演算で処理）が重い。生成 C のサイズは変化しない（呼び出し側が同じ）。

## 7. 限界（事実）

- UM=0 の経路（上記）は実機と未突合（ホストで #XM のため）。
- ホスト版のリファレンスは AMD Ryzen 9 9950X でのみ確認（0038 §7 と同じ）。Intel 機で差が出るか未確認。
- sqrt の整数平方根はビット単位の反復で、1 回あたりのコストが大きい（上記）。必要なら 1 回あたりの反復を減らす実装（Newton 法など）に替えられるが、今回は正確さを優先して替えていない。
- RCPPS/RSQRTPS、MMX 形、HADD、DPPS、VEX 形は 0038 と同じく未実装。
