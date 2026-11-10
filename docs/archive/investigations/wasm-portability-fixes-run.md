# wasm 移植の 3 つの穴を塞いだ実行記録（決定 0049。作成時は 0046 だったが 0046-m1-acceptance.md と重なったため 0049 に改番）

日付: 2026-10-21。HEAD 31d0134 + ワークツリー。方式は `docs/decisions/0046-wasm-portability-fixes.md`、ゲストアドレスの洗い出しは `docs/investigations/hi32-audit.md`。「実測」はコマンドの出力、「推測」は未検証の解釈。成果物は `lift/out-port2/`（`tmp/` = TMPDIR、`emcache/` = EM_CACHE。`wasm/` の `out` は `out-wasm2/out`（生成 C 16 分割、約 430 MB）へのシンボリックリンクで、複製していない）。

**ワークツリーの注意（実測）**: 作業開始時の `git status` は `lift/iat-unsup-closure.txt`（未追跡）だけだった。作業の途中で、**別担当**が `lift/crates/fe-lift/src/{cpuid,int_more2,lib,sse_int2,sse_pack}.rs`、`tests/{int_more2,locale}.rs`、`runtime/fle_cpuid.c`、`fle_sse_int2.h`、`fle_locale.c`（0047、0048 の作業）、`fle_rcp.h`、`fle_rcp_table.h`、`tests/rcp/`、`docs/investigations/locale-6-official-sources.md` を変更・追加した。本書の `cargo test --workspace`（§6）は、その変更が入った状態のワークツリーで走っている（私の変更と他担当の変更を分けた再実行はしていない）。本作業が触ったファイルは §8。

## 1. 要約（実測）

| 項目 | 結果 |
|---|---|
| 穴 1: `fle_addr` 経由にした箇所 | **`fle_addr` 36 か所、`fle_addr_opt` 19 か所、合計 55**（`fle_sse.h` の `fle_ldx`、`fle_stx` の 2 を含む）。対象外 **71 か所**（`(uintptr_t)`、理由つきの一覧は hi32-audit.md §3） |
| 穴 1: native の既定 | `fle_addr`、`fle_addr_opt` は `((uintptr_t)(a))` に展開（`clang -E`）。生成 C の native オブジェクト 3 本（part-00、05、14）は修正前後で `objdump -d` の差 0 |
| 穴 1: wasm | `fle_ldx`、`fle_stx` に上位ビット付きのアドレスを渡す C が `[fle_addr_hi32]` で止まる（load、store とも） |
| 穴 2: `sqrtss`/`sqrtsd` の突合（ホスト実機 vs `fle_sqrtss/sd`、native clang） | float **4,294,967,296 件（全 2^32）、不一致 0**。double **50,657,341 件（境界 57,344 + 完全平方数とその隣 599,997 + 乱数 50,000,000）、不一致 0** |
| 穴 2: wasm（node）と native のハッシュ | float 8 分割、double 8 分割とも **全 16 シャードのハッシュ一致**（cases 4,294,967,296 と 50,657,341） |
| 穴 2: `<emmintrin.h>` | `lift/runtime` に include する行は **0**（`fle_mxcsr_flags.h` の先頭コメントの否定文だけが残る） |
| 穴 2: emcc（`-msse2 -msimd128` なし） | runtime `fle_*.c` **32 / 32 成功**（警告 1: 既存の `fle_proc.c:105`）。生成 C **16 / 16 成功**（`emcc -c`、エラー 0、警告 0）。リンク成功、`flengine.wasm` 115,724,157 B。node 実行の停止点が native と一致 |
| 穴 3: `fscale` の原因の `long` | `fle_x87.c` の **181 行 `e`、184 行 `u`、202 行 `nb`、242 行 `e2`、1025 行 `n`、1027 行の 2 つのキャスト**（§3） |
| 穴 3: 修正後 | `fscale` のプローブ 60,002 行（`fscale_probe.c`、`fscale_probe2.c`）の native と wasm の不一致 **0**（修正前 18,822）。x87 の全プローブ（difftest の `int_x87_d.c` 11 グループ）**不一致 0**。新規の x87 全入口の突合 54 グループ、165,000 ケース **不一致 0**（修正前のソースでは `fscale` の 12 グループが不一致） |
| テスト | `cargo test --workspace`: **65 バイナリ、616 passed、0 failed**（`out-port2/test-ws.txt`）。`x87.rs` 14、`x87_trans.rs` 9、`sys.rs` 18、`vmap.rs` 23、`wasm_port.rs` 5、`sse_scalar.rs` 9、`mxcsr_flags.rs` 10、すべて通る |

## 2. 穴 1: `fle_ldx`/`fle_stx` と runtime の直接変換

- 洗い出し: `grep` で `uintptr_t` と `(void *)`、`(const void *)`、型名つきの `(u?int*_t *)`、`(char *)` へのキャストを全 `lift/runtime/*.{c,h}` で探した。型名つきのキャストで `uintptr_t` を使わない形（`*(T *)R[...]` など）は 0 件。変更の一覧は hi32-audit.md。
- native の挙動不変（実測）:
  - `clang -E` の展開: `fle_ldx` → `memcpy(&v, (const void *)((uintptr_t)(a)), sizeof v)`、`fle_stx` → `memcpy((void *)((uintptr_t)(a)), &v, sizeof v)`（従来のコード）。
  - 生成 C 3 本（part-00、05、14）の native オブジェクトを、修正前（`out-wasm2/native/obj/`）と修正後（`out-port2/wasm/native/obj/`）で `objdump -d --no-show-raw-insn` して比較: **差 0**（ファイル名の行だけが違う）。
  - runtime の native オブジェクト 32 本の比較: 29 本は差 0。差が出たのは `fle_glob.o`、`fle_imports.o`、`fle_sys.o` の 3 本で、穴 3 のホスト割り当て 4 か所（`fle_host_size_fits`、native では常に真）に `-O0` でサイズ比較が入ったため（推測。命令の行数の差は、同じ行を挿入した後ろのずれを含む）。
  - `cargo test --workspace`: 616 passed、0 failed。
- wasm（`out-port2/hi32/t.c`、`t2.c`、emcc + node）: `fle_stx` → `fle_ldx` で往復（低いアドレス）は通り、`fle_ldx(buf + 2^32)` は `fle: guest address 0x100010d90 (16-byte load) … [fle_addr_hi32]`、`fle_stx(buf + 2^32)` は `0x100010d70 (16-byte store) … [fle_addr_hi32]` で abort（終了コード 1）。修正前は上位ビットを捨てて `buf` を読み書きした（決定 0045 d の説明、再測はしていない）。
- コンパイル: runtime の全 32 本が native の `-DFLE_CHECK_HI32`（`clang -c`）と wasm の `-DFLE_CHECKED_MEM`（`emcc -c`）で通る（それぞれ 32 / 32。警告 1: 既存の `fle_proc.c:105`）。
- 生成 C の再生成はしていない。`out-wasm2/out/` の 16 分割を、修正した runtime のヘッダで `emcc -c`（§4）と native の `clang -c`（`build_native.sh`）にかけた。

## 3. 穴 3: `fscale` の `long` の二分法

方法: `lift/out-port2/fscale/variant.sh NAME "行番号…" ["追加の sed"]` が、`lift/runtime/fle_x87.c` の指定した行の `long` を `int64_t` にしたコピー（`v/NAME/fle_x87.c`）を作り、プローブ（`probe.c` = `out-wasm/difftest/fscale_probe2.c` の複製、60,002 行）を clang（native）と emcc（wasm32、`-O0`）でビルドして実行し、出力の差（`diff` の `<` の行数）を数える。元の `fle_x87.c` は変えずに試した。`native_ref.txt` は元のソースの native の出力で、全変種の native の出力が `native_ref.txt` と一致（差 0）。

| 変種（`int64_t` にした行） | native と wasm の差の行数 |
|---|---|
| なし（元のソース） | 18,822 |
| 全 25 行 | 0 |
| 43〜953（前半 13 行） | 18,822 |
| 1025〜1309（後半 12 行） | 18,822 |
| 前半 + 1025、1027 | 0 |
| 後半 + 43、181、184、201、202、242、283 | 0 |
| 後半 + 361、597、654、672、931、953 | 18,822 |
| 43 + 1025 + 1027 | 18,822 |
| 181、184 + 1025 + 1027 | 17,540 |
| 201、202、242、283 + 1025 + 1027 | 18,822 |
| 43、181、184、201、202、242、283 + 1025 + 1027 | 0 |
| 181、184、201、202、242 + 1025 + 1027 | 0 |
| 184、201、202、242 + 1025 + 1027（181 を外す） | 18,822 |
| 181、184、201、202、1025 + 1027（242 を外す） | 9,346 |
| 181、184、201、242 + 1025 + 1027（202 を外す） | 8,194 |
| 181、201、202、242 + 1025 + 1027（184 を外す） | 9,346 |
| 181、184、202、242 + 1025 + 1027（201 を外す） | **0** |
| 181 の引数 `e` だけ（`emin`、`emax` は `long` のまま）+ 184、202、242 + 1025 + 1027 | **0** |
| 同じ（181 の `e` を `long` のまま） | 18,822 |
| 同じ（1025 を外す、または 1027 を外す） | 18,822 |
| 1027 のキャストだけ `int64_t` + 181 の `e`、184、202、242、1025 | 0 |

読み（実測から）:

- **効いている `long` は、`round_pack` の指数 `e`（181）、`u`（184）、`nb`（202）、`e2`（242）と、`fscale` の `n`（1025）と 1027 行のキャスト**。どれを外しても差が戻る（上の表。1025 か 1027 を外すと 18,822、184 か 242 を外すと 9,346、202 を外すと 8,194、181 の `e` を外すと 18,822）。
- 効いていない `long`: 201 行の `uq`、181 行の `emin`/`emax`（引数）、361 行の `emin`/`emax`。これらを `long` のまま残しても 0 件になる。
- 理由（推測。コードを読んだ範囲）: `fscale` は `a.e + n` で指数に `±2^40` までの整数を足して `round_pack` に渡す（1027 行）。wasm32 の `long` は 32 ビットなので、`(long)1 << 40` が 256 に、`(long)(b.m >> …)` と、それを足した `e`（181）、`u = e - pbits + 1`（184）、`nb = e - u + 1`（202）、`e2 = u + L - 1`（242）が 32 ビットで折り返す。指数が 32 ビットを超える `fscale` の結果（オーバーフロー、アンダーフローの判定）が変わる、と読んだ（折り返しの具体的な値は見ていない）。
- 修正: `fle_x87.c` の `long` を**全部** `int64_t` にした（25 行、定数 `32L` を `(int64_t)32` に 1 か所）。効かない `long` も、幅依存を残さないために替えた。`grep -w long` と `[0-9]L` が `fle_x87.c` で 0 件（`long long` は元からない）。
- 修正後の確認（実測）:
  - `variant.sh fixed ""`（修正後のソース）: native と wasm の差 **0** / 60,002 行。native は元のソースの出力と一致。
  - `emcc -c -Wall -Wextra`（`-O0`）で `fle_x87.c` の警告 0（修正前は 1027 行の `shift count >= width of type` の警告があった。0045 の記録）。

## 4. 穴 2: `sqrtss`/`sqrtsd` と `emmintrin.h`

- 実装: §2 の決定 b。`fle_sse.h` の `fle_sqrtss`、`fle_sqrtsd` は、`fle_mx_sqrt(f64, bits, 0x1F80, &fl)`（`fle_mxcsr_flags.h`）を呼ぶ。`fle_mxcsr` は読み書きしない。
- 呼び出しの確認（実測）: 生成 C（16 分割）の `fle_sqrtss`/`fle_sqrtsd` は 0 件、`fle_mx_sqrtss`/`fle_mx_sqrtsd` は 1 件（part-00）。runtime の `.c`、`tests/` に `fle_sqrtss/sd` の呼び出しなし（出力するのは `sse_scalar.rs` の 2 行だけ）。
- 突合（`lift/out-port2/sqrt/sqrt_check.c`）:
  - 参照: ホスト実機の `sqrtss` / `sqrtsd`（インライン asm、`-DHOST`、native clang `-O2`。MXCSR は既定）。
  - float: 全 2^32 のビットパターン（8 シャード、各 536,870,912 件）。合計 **4,294,967,296 件、不一致 0**。NaN（静、準、符号付き、ペイロード付き）、±0、±inf、非正規化、負数を含む。
  - double: 境界値（符号 2 × 指数 2,048 × 仮数 14 種 = 57,344 件。NaN、±0、±inf、非正規化、仮数の境界）+ 完全平方数とその ±1 ulp（199,999 × 3 = 599,997 件。厳密な根、丸めの境界）+ 乱数 50,000,000 件（全ビット、指数 1 付近、2 付近、非正規化）。合計 **50,657,341 件、不一致 0**。
  - native の `-DHOST` なしのハッシュ（`fle_sqrtss/sd` だけ）と、`-DHOST` ありのハッシュは 16 シャードすべて一致（モデルの出力が同じ）。
  - wasm（emcc `-O2`、`-msse2 -msimd128` なし、node）: float 8 シャード、double 8 シャードの出力（cases、不一致 0、FNV-1a ハッシュ）が native と**全 16 シャード一致**（`cmp`）。
- `<emmintrin.h>`: `grep -rn "emmintrin\|xmmintrin\|immintrin" lift/runtime` は `fle_mxcsr_flags.h:4` のコメント（"No asm, no <emmintrin.h>…"）だけ。`lift/harness/wasm/build.sh` の `SIMD="-msse2 -msimd128"` を `SIMD=""` に替えた（局所変更）。
- emcc の通し（`-msse2 -msimd128` なし、`-O0`、TMPDIR、EM_CACHE は `out-port2/` の中）:
  - runtime の全 `fle_*.c` **32 / 32 成功**（`build.sh rt`、`VARIANT=base`、作業の終わりに別の `VARIANT=rerun` で再実行して 32 / 32）。警告は既存の `fle_proc.c:105` の 1 件。
  - 生成 C 16 分割（`-finstrument-functions`、同時 2 本）**16 / 16 成功**。1 TU 2.9〜7.2 秒、最大 RSS **1,811,384 KB**（part-14）。最初に part-00、part-14 だけを先に通し（3.73 秒 / 947 MB、6.91 秒 / 1,810 MB）、その後 `build.sh gen-c harness link` で 16 本全部。
  - harness 3 本、リンク成功（5.21 秒、最大 RSS 635 MB）。`flengine.wasm` **115,724,157 B**（0045 の 115,721,618 B より 2,539 B 多い。ほかの担当の変更を含むワークツリーなので、私の変更だけの差ではない）。
  - node 実行（`FLEngine_x64.dll`）: 停止は `GetLocaleInfoW: LCType value for en-US is not on the MSDN pages`、関数 **199**、入口 **1,386**、深さ **9**。同じ runtime と生成 C の native との比較は、`calling … rsp=` の 1 行（スタックのアドレス）を除いて**停止レポート全文が一致**（`run-native.txt` と `run-node.txt`）。

## 5. runtime の幅依存の一覧（wasm32 で意味が変わるもの）

調査方法: `grep` で `long`、`unsigned long`、`size_t`、`uintptr_t`、`intptr_t`、`ssize_t`、`ptrdiff_t`、`off_t`、`time_t`、`%l` 書式、`L` 接尾、`strtol`、`__builtin_clz*`、`malloc`/`calloc`/`realloc` の引数を全 `lift/runtime/*.{c,h}` で探した。加えて、wasm32 の診断（`clang --target=wasm32-unknown-emscripten -Wshorten-64-to-32 -Wint-to-pointer-cast -Wpointer-to-int-cast -Wformat`、emscripten の sysroot）を全 `.c` にかけた。native の同じ診断は 0 件。

| 種類 | 場所 | wasm32 での意味 | 対処 |
|---|---|---|---|
| `long`（x87 の指数） | `fle_x87.c` 25 行 | 32 ビット。**`fscale` の結果が変わる**（§3） | 全部 `int64_t`（修正済み） |
| `(size_t)` へのゲストの 64 ビットの大きさ | `fle_imports.c`（`LocalAlloc`）、`fle_sys.c`（`HeapAlloc`）、`fle_glob.c`（`GlobalAlloc` 2 か所） | `size_t` が 32 ビットで、4 GiB + n が n バイトになって成功する（native は失敗） | `fle_host_size_fits`（`fle_host.h`）で収まらない要求を NULL にした（修正済み。native では常に真） |
| `long` の tid（`fle_host_tid()` と各ファイルの `*_owner_tid`） | `fle_glob.c`、`fle_imports.c`、`fle_ole.c`、`fle_oleaut.c`、`fle_proc.c`、`fle_sys.c`、`fle_sysinfo.c`、`fle_vmap.c`、`fle_vol.c`、`fle_host.h` | 32 ビット。値は wasm で 1、native で Linux の tid（32 ビットに収まる）。意味は変わらない | 変更なし（`%ld`、`(long)` はこの範囲） |
| `long` の添字（`env_find`、`atom_find`、`i`） | `fle_env.c`、`fle_glob.c` | 値は表の添字（小さい）。意味は変わらない | 変更なし |
| `sysconf(_SC_PAGESIZE) != (long)FLE_PAGE_SIZE` | `fle_sys.c:298` | wasm では `FLE_VMEM_ARENA` で検査を行わない（0045） | 変更なし |
| `realloc(p, nc * sizeof *x)` の `uint64_t` の乗算（`nc` は小さな表の容量） | `fle_glob.c`、`fle_ptrmap.h`、`fle_sys.c`、`fle_vfs.c`、`fle_vmap.c`、`fle_vol.c` ほか | `-Wshorten-64-to-32` が 23 件（暗黙の 64→32 の縮小）を出すが、`nc` は 2 倍ずつ増える小さな容量（上限は `FLE_MAX_REGIONS` 65,536 など）で 32 ビットを超えない（推測。各表の上限を個別に確認してはいない）。`fle_vfs.c:164` の `nc` は `MAX_FILE_BYTES`（256 MiB）が上限 | 変更なし（診断は残る。意味は変わらないと読んだ） |
| `size_t`/`uintptr_t` の `(size_t)total` など、上限を先に比べたあとの変換 | `fle_oleaut.c`（`FLE_SA_MAX_BYTES` 1 GiB、`FLE_BSTR_MAX_CHARS` 64 Mi）、`fle_ole.c`（`FLE_COTASK_MAX` 1 GiB）、`fle_vmap.c`（`FLE_VMAP_MAX_BYTES` 1 GiB）、`fle_reg.c`（`cb` は `uint32_t`） | 上限が 4 GiB 未満なので、変換で切れない | 変更なし |
| `uintptr_t` ↔ ゲストのアドレス | §2、hi32-audit.md | 上位 32 ビットを黙って捨てる | `fle_addr`（修正済み） |
| `%ld` の書式 | `fle_imports.c:76`、`fle_sys.c:41,299` | `long` に合っている | 変更なし |
| `__builtin_clz(unsigned)`（32 ビット）、`__builtin_clzll` | `fle_x87.c:57,61,1094` | `int` 32 ビット、`long long` 64 ビットで、どちらも wasm32 でも同じ | 変更なし |
| `off_t` | `fle_vmap.c`（`pwrite`、`mmap` の引数。native の経路のみ） | アリーナ版（wasm）では使わない | 変更なし |
| 生成 C | `(long)`、`unsigned long`、`(size_t)`、`(intptr_t)` が 0 件（`long long` を除く。16 分割） | — | — |

`unsigned long` と `size_t` が runtime の生成物の出力形式に影響する箇所（`%lu` など）は、grep で見つからなかった（`%l[dux]` は上の 3 か所だけ）。

## 6. テスト

```
cd lift && cargo test --workspace --offline > out-port2/test-ws.txt 2>&1     # 終了コード 0、フォアグラウンドで完了
grep '^test result:' out-port2/test-ws.txt     # 65 行
```

集計（`test result:` 行の合計、実測）: **616 passed、0 failed、0 ignored**。0045 の記録は 615（65 バイナリ）で、差の 1 件は別担当が増やしたテスト（`locale.rs` などと推測。内訳の確認はしていない）。

## 7. 他の確認（実測）

- difftest の既存プローブ（`out-wasm/difftest/` のソースを、修正した runtime で clang と emcc の両方に通した。`fle_host.c` を足してリンク。`out-port2/probes/`）: `fscale_probe.c` 60,002 行、`fscale_probe2.c` 60,002 行、`int_x87_d.c` 11 グループ（`div/imul1`、`idiv`、x87 の 9 操作）。**どれも native と wasm の差 0**。新しい native の出力は、`out-wasm/difftest/` の古い native の出力（`fscale_native.txt`、`fscale2_native.txt`、`int_x87_native.out`）と一致（差 0）。
- x87 の全入口（`out-port2/x87/x87_all.c`）: `fle_x87_arith_st`（6 演算 × PC 3 × RC 4 × 600 ケース）、`fsqrt`/`frndint`/`fchs`/`fabs`/`fxam`、`fscale`（ST1 の指数が 0〜79、PC 3 × RC 4 × 3,000）、`f2xm1` と `fsincos`、`fld`/`fild`/`fst`/`fist`、`fcom`/`fcomi`/`ftst`。特殊なオペランド（0、非正規化、擬似非正規化、inf、QNaN、SNaN、unnormal、1 付近、極小、極大）と乱数。**165,000 ケース、54 グループ、native と wasm のハッシュ全一致**。修正前の `fle_x87.c`（`out-port2/fscale/v/ref2/fle_x87.c`）では、**`fscale` の 12 グループだけが不一致**（他は一致）。native は修正前後で全グループ同一。
- wasm は `-O0`（x87 のプローブと `sqrt_check` は一部 `-O2`）。ブラウザ、`-O1` 以上の生成 C は未確認。

## 8. 触ったファイル（本作業）

- 変更（`lift/runtime/`）: `fle_sse.h`（`#include <emmintrin.h>` の削除、`fle_ldx`/`fle_stx`、`fle_sqrtss/sd`）、`fle_runtime.h`（`fle_addr_opt`）、`fle_host.h`（`fle_host_size_fits`）、`fle_x87.c`（`long` → `int64_t`）、`fle_env.c`、`fle_imports.c`、`fle_locale.c`（`fle_addr_opt` の 12 か所だけ。0048 の変更は別担当）、`fle_ole.c`、`fle_oleaut.c`、`fle_osver.c`、`fle_reg.c`、`fle_res.c`、`fle_sys.c`、`fle_sysinfo.c`、`fle_uipref.c`、`fle_vfs.c`、`fle_vmap.c`、`fle_glob.c`（割り当ての `fle_host_size_fits`）。
- 変更: `lift/harness/wasm/build.sh`（`SIMD` を空に）。
- 新規: `docs/decisions/0046-wasm-portability-fixes.md`（`0046-m1-acceptance.md` と番号が重なる。決定の本文に書いた）、`docs/investigations/hi32-audit.md`、本書。`lift/out-port2/`（`sqrt/`、`fscale/`、`x87/`、`probes/`、`hi32/`、`width/`、`chk/`、`wasm/`、`tmp/`、`emcache/`、`test-ws.txt`）。
- 触っていない: `fle_mxcsr_flags.h`、`fle_vex.h`、`fe-lift` の `src/`、`tests/`（既存）、`lift/harness/` の他のファイル、`docs/decisions/0045`。`fle_cpuid.c`、`fle_sse_int2.h`、`fle_rcp*.h`、`crates/`、`locale-6-official-sources.md`、`tests/rcp/` の変更は他担当のもの。

## 9. 未確認、限界

- 領域チェック（`FLE_CHECKED_MEM`）と上位ビット検査のコスト増（`fle_ldx` 系の 6,498 か所を加えたぶん）は再測していない。
- 生成 C の DllMain 経路（停止点より先）で SSE、x87 が実行されたときの native と wasm の一致は未確認（今回の突合は単体のプローブ。0045 の限界のまま）。
- `realloc` の 64→32 の縮小（§5）は、各表の上限を個別に確かめていない。
- `cargo test --workspace` は、別担当の変更が入ったワークツリーでの結果。
- ブラウザ、`-O1` 以上の生成 C は未確認（0045 のまま）。
- 削除禁止のため、`out-port2/fscale/v/` の変種（47 個）、`out-port2/wasm/obj-rerun` などの途中の成果物が残っている。
