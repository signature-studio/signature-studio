# 0049: wasm 移植の 3 つの穴（ゲストアドレスの検査漏れ、ホスト SSE 命令、`long` の幅）

状態: 作業者（サブエージェント）が決定。依頼は「決定 0045 とその実行記録、probe が残した 3 つの穴を塞ぐ。ネイティブの挙動と `cargo test --workspace` は変えない」。**番号について**: `docs/decisions/` には、別担当が書いた `0046-m1-acceptance.md` が既にある（本書の作業開始時点で確認した）。本書は依頼どおり 0046 の名前で書くが、番号が重なっているので、PM が採番を直すこと。決定の中身は `docs/investigations/hi32-audit.md`（穴 1）と `docs/investigations/wasm-portability-fixes-run.md`（実行記録、実測）にある。「実測」はコマンドの出力、「推測」は未検証の解釈。

## 決定

### a. `fle_ldx`/`fle_stx` と runtime 内の直接変換を `fle_addr` 経由にする（0045 d の「未適用の経路」）

- `fle_sse.h` の `fle_ldx`、`fle_stx` の `(uintptr_t)a` を `fle_addr(a, 16, 0)`、`fle_addr(a, 16, 1)` にした（2 行）。`fle_ldxa`/`fle_stxa`/`fle_vld`/`fle_vst`（`fle_vex.h`）はこの 2 関数を呼ぶので、生成 C の呼び出し 6,498 か所（`fle_ldx` 264、`fle_ldxa` 3,754、`fle_stx` 206、`fle_stxa` 2,274）と VEX の 16 か所は、生成 C を変えずに通る。**生成器 `fe-lift` と生成 C は変えていない。**
- runtime の `.c` の `(uintptr_t)` 変換（ゲストのアドレスを扱うもの）を洗い出し、`fle_addr`（36 か所）または新規 `fle_addr_opt`（19 か所）に替えた。対象外（ホストのポインタ、自前のバッファ、mmap 系、`malloc` の結果の `free`）の 71 か所は理由とともに `docs/investigations/hi32-audit.md` に書いた。
- `fle_addr_opt(a, n, store)`（`fle_runtime.h`）を足した: NULL を正当な引数として受け取る API 用。`a == 0` ならそのまま 0、他は `fle_addr`。理由: `fle_addr` を NULL にそのまま使うと、`FLE_CHECKED_MEM` が NULL を領域外として止め、API の NULL の規則（`GetLocaleInfoW` の `cch = 0` の問い合わせなど、決定 0041/0042 ほか）を変えてしまう。NULL の判定は変換の後に呼び出し側が従来どおり行う。
- native の既定では `fle_addr` も `fle_addr_opt` も `((uintptr_t)(a))` のマクロに展開される（従来と同じ変換、検査と呼び出しなし）。`clang -E` で確認し、native の生成 C オブジェクト 3 本（part-00、05、14）が修正の前後で命令列同一。
- 制限（0045 d のまま）: 検査は上位 32 ビットと（`FLE_CHECKED_MEM` で）領域だけ。ヒープ内のワイルドポインタ、ページ保護は捕まえない。`fle_ldx` 系を加えたぶん、領域チェックのコスト（0045 の実測）は増える。再測はしていない。

### b. `fle_sse.h` からホストの SSE 命令を外す

- `#include <emmintrin.h>` と、`_mm_sqrt_ss`、`_mm_sqrt_sd` を使っていた `fle_sqrtss`、`fle_sqrtsd` を、`fle_mxcsr_flags.h`（純 C、決定 0044）の整数平方根 `fle_mx_sqrt`（MXCSR は既定値 0x1F80: RN、例外マスク、FTZ/DAZ なし）による実装に替えた。2 関数は `fle_sse.h` の末尾（`fle_mxcsr_flags.h` を include した後）に置いた。名前と署名は変えていない。
- **呼ばれるか**: 生成 C には 0 件（all-pdata 16 分割、実測）。`fle_sqrtss/sd` を出すのは `sse_scalar.rs` だが、`sse_more.rs` が `sse_fp_flags.rs`（`fle_mx_sqrtss/sd`）を先に呼ぶので到達しない（`lib.rs` の `lift_insn` の順序）。テストの `sse_scalar.rs` の `sqrt_matches_host` は `lift_body` の出力（`fle_mx_sqrt*`）を試すので、この 2 関数も呼ばない。runtime の `.c` にも呼び出しはない。したがって、2 関数は**削除せず**（`sse_scalar.rs` のコードが名前を出す。削除すると、将来 `sse_more.rs` の分岐を外した場合にリンクできなくなる）、実装だけを替えた。
- **MXCSR 版との関係**: 呼び出し元がないので、guest の MXCSR を読まない既定値版とした（関数内の MXCSR は固定の 0x1F80。状態ビットは捨て、`fle_mxcsr` に触らない）。実行経路で使われている MXCSR 版（`fle_mx_sqrtss/sd`）は `fle_mxcsr` の RC/FTZ/DAZ と状態ビットを扱う。既定の MXCSR で両者の結果ビットが同じであることは、テスト `mxcsr_flags.rs`（host 突合）が `fle_mx_sqrt*` を、本作業の突合が `fle_sqrtss/sd` を、同じ実装 `fle_mx_sqrt` で確かめている。
- `fle_sse.h` は `-msse2 -msimd128` なしで emcc を通る。`lift/harness/wasm/build.sh` の `SIMD="-msse2 -msimd128"` を `SIMD=""` に替えた（局所変更 1 行。`CC_COMMON` から消える）。`lift/runtime` に `emmintrin.h` を include する行はなくなった（`fle_mxcsr_flags.h` の先頭コメントの否定文を除く）。`tests/ref/fle_mxcsr_flags_host.h` と `tests/mxcsr.rs` はホストの SSE を参照実装として使うテスト側のコードで、runtime ではない（変えていない）。

### c. `long` を固定幅にする（ソフト x87 `fscale`）

- 原因: `round_pack`（`fle_x87.c:181`）の `long` のうち、**`e`（181 行の引数）、`u`（184）、`nb`（202）、`e2`（242）、および `fscale` の `long n`（1025）と `(long)1 << 40`、`(long)(b.m >> …)`（1027）**。二分法の結果は `wasm-portability-fixes-run.md` §3。単独の `long` を `int64_t` にしても不一致は減らない（181 行の `e` と `u`、`nb`、`e2` がそろって初めて効く。`round_pack` は指数を `e - pbits + 1` のように 64 ビットの範囲で計算する）。`uq`（201）と `emin`、`emax`（181 の引数、361 行）は、なくても結果が変わらない（`int64_t` に替えても替えなくても native と wasm が一致する）。
- `fle_x87.c` の `long` を全部 `int64_t` にした（25 行 + `32L` の定数 1 か所）。結果の `native == wasm` が 60,002 行すべてで一致し、native の出力は修正の前後で同一（60,002 行、差 0）。`rr.e`（43 行）、`round_pack` の `emin`/`emax` の引数、`round80`、`bf.e` ほか、効かない `long` も固定幅にしたのは、**ビット幅に依存するコードを残すより、全部を揃える方が安全**（今後の編集で 32 ビットの落とし穴が戻らない）と判断したため。
- runtime 全体の幅依存を grep し、wasm32 で意味が変わるものを一覧にした（`wasm-portability-fixes-run.md` §5）。直したのは `fle_x87.c` の `long` と、ホストの割り当て 4 か所（`LocalAlloc`、`HeapAlloc`、`GlobalAlloc` の 2 か所）の `size_t` 変換。ゲストが 4 GiB + n バイトを要求すると、wasm32 では `(size_t)` で n バイトに切れて成功してしまうので、`fle_host_size_fits`（`fle_host.h`）で、`size_t` に収まらない要求を「ホストが満たせない割り当て」（NULL）にした。native では常に真（`SIZE_MAX == UINT64_MAX`）。
- `long` が 64 ビットで 32 ビットでも結果が同じものは、変えていない（`fle_host_tid()` と各ファイルの owner の `long`（スレッド ID は 1 か Linux の tid、32 ビットに収まる）、`env_find` などの添字）。`realloc(…, nc * sizeof *x)` の `uint64_t` 乗算（`nc` は小さい、wasm で 32 ビットに切れる値にならない）も変えていない。いずれも一覧に理由を書いた。

## 代替案

- `fle_addr` を NULL に使い、`FLE_CHECKED_MEM` の領域で NULL を許す案: 領域チェックは NULL 領域を捕まえることが目的の一つ（0045 d）なので却下。
- `fle_sqrtss/sd` を削除する案: `sse_scalar.rs` が出力する名前で、依頼も削除ではなく整理を求めたため却下。
- 生成 C の `fle_ldx` 等をマクロにして `fle_addr` を呼ぶ案: `fle_sse.h` の 2 関数を直すだけで済むので採らなかった。
- `long` を必要なものだけ `int64_t` にする案: 二分法の最小組（`round_pack` の 4 つと `fscale` の 2 か所）は分かったが、残りも幅依存なので全部替えた。

## 限界

- 領域チェック、上位ビット検査のコスト増（`fle_ldx` 系の追加分）は再測していない。
- ブラウザ、`-O1` 以上は未確認（0045 のまま）。
- `fscale` と x87 の全入口、`fle_sqrtss/sd` の wasm と native の一致は、突合したケースの範囲（実行記録）で確かめた。生成 C の DllMain 経路（停止点より先）で x87/SSE が実行されたときの一致は未確認（0045 の限界のまま）。
