# wasm ビルド調査（ADR-0005 A3「生成 C が emcc でリンクできる」の初の実測）

日付: 2026-10-21。HEAD 1f88ad5（作業ツリーには他担当の未コミット変更あり。`lift/runtime/fle_locale.c` の変更を含む状態で実測した。`lift/runtime` のスナップショットを `lift/out-wasm/snap/runtime`（fle_locale.c の md5 abfb27fa…）に取り、`diff -rq runtime out-wasm/rt-wasm` で、`rt-wasm` は `fle_mxcsr_flags.h` 以外 `runtime` と同一であることを確認した）。**調査の途中（リンク、実行、difftest の後）に、他担当が `lift/runtime/fle_x87.c`、`fle_x87.h`、`lift/crates/fe-lift/src/x87.rs` を変更した**（`git status` で確認。x87 超越関数の作業と推測）。本書の数値は、それより前の `runtime`（`snap/runtime` と `rt-wasm/`）と、そのとき生成した C（`out/wasm-pdata.c`）に対するもの。変更後の runtime での再測定はしていない。成果物はすべて `lift/out-wasm/`（`tmp/` = TMPDIR、`emcache/` = EM_CACHE）。`runtime`・`harness`・crates は変更していない。再現は `bash lift/out-wasm/run.sh`（§8）。

「実測」はコマンドの出力、「推測」は未検証の解釈。判断は PM が行う。

## 0. 要約（実測）

| 項目 | 結果 |
|---|---|
| emcc | 6.0.12、`/opt/emsdk`、node v24.19.0。EM_CACHE は `out-wasm/emcache` |
| runtime `fle_*.c` 31 本、**そのまま** `emcc -c`（`-O0 -ffp-contract=off -fno-fast-math`） | **21 成功 / 10 失敗** |
| runtime、`-msse2 -msimd128` のみ追加 | 21 成功 / 10 失敗（失敗の中身が変わるだけ。§2.2） |
| runtime、互換 shim 付き（`syscall`/`SYS_gettid` の読み替え、`fle_mxcsr_flags.h` のソフト版） | **31 成功 / 0 失敗** |
| 生成 C 16 分割（all-pdata、215,539,533 B、58,695 関数）、runtime そのまま | **0 成功 / 16 失敗**（各 56 エラー、すべて `fle_mxcsr_flags.h` の asm 起因） |
| 生成 C 16 分割、shim 付き | **16 成功 / 0 失敗**（`-finstrument-functions` あり、なしの両方） |
| リンク（shim + 最小 main） | **成功**。`flengine.wasm` 115,518,408 B（約 110 MiB）。リンク 1.26 秒、最大 RSS 634 MB |
| node 実行（mmap 系の応答なし） | 停止: `fle: host page size 65536 is not 4096`（関数 86、入口 117） |
| node 実行（`sysconf`/`mmap`/`mprotect`/`madvise` を `--wrap` で応答。probe 用） | **ネイティブと同じ停止点**: `GetLocaleInfoW` LCType 0x37、関数実行数 **198**、入口 **984**、深さ 11。停止レポート全文がネイティブ（`out-loc/run.txt`）と一致（`rsp` の行だけ違う） |

- 注意（実測）: ネイティブの DllMain 経路（198 関数 / 984 入口）で実際に実行された「x86 依存の runtime」は、gdb の呼び出し回数で **`fle_mxcsr_store` 1、`fle_mxcsr_load` 1、`fle_x87_store_cw` 1、`fle_cpuid` 8、`fle_imul1` 47、`fle_div` 46、`fle_imp_VirtualAlloc` 1、`fle_imp_GetCurrentThreadId` 5**。`fle_mx_*`（ホスト SSE 実行）、`fle_x87_*` の演算、`CreateFileMappingW` は **0 回**。つまり「同じ停止点に同じ関数実行数で止まった」ことは、ソフト版 SSE、ソフト x87 の等価性の証拠には**ならない**（その経路が実行されていない）。
- 生成 C の x86 依存の本質は 1 つ: `fle_mxcsr_flags.h`（インライン asm でホスト CPU に SSE 命令を実行させる）。それ以外は、wasm32 でコンパイルは通る（ただし挙動の差は §4）。

## 1. 環境と手順

- emcc 6.0.12（`/opt/emsdk/upstream/emscripten/emcc`）、node v24.19.0（`/home/ubuntu/.local/bin/node`）、16 コア、RAM 約 14 GB。
- 指示にあった `docs/investigations/2026-10-19-environment.md` はこのリポジトリに**存在しない**。旧調査 old-ref の `tools/m8/m8_build.sh`（`lift/out-oracle/old-ref/repo/`）から wasm フラグを確認した: `-O0 -g1 -sALLOW_MEMORY_GROWTH=1 -sSTACK_SIZE=8MB -sGLOBAL_BASE=268435456 -sENVIRONMENT=node -sMODULARIZE=1 -sEXIT_RUNTIME=1 -Wl,--initial-memory=0x36000000 -Wl,--max-memory=0x36000000`。本調査は `-g1` なし、`--initial-memory=0x28000000`（640 MiB）、`--max-memory=0x80000000`（2 GiB）、`-sNODERAWFS=1`（DLL を node のファイルシステムから読むため。STANDALONE_WASM は使っていない）。
- 生成 C: `fe-lift lift FILE OUT.c --pdata`（`out-wasm/target/release/fe-lift`、`CARGO_TARGET_DIR=out-wasm/target`）。3.10 秒、最大 RSS 320 MB、出力 215,539,533 B、`c_lines` 2,880,419、`bad_nodes` 0。**`out-loc/out/loc-pdata.c` と `cmp` で同一**。`split_c.py`（`out-pdata` のもの）で 3,700 関数/ファイル、16 分割。
- コンパイルフラグ:
  - runtime: `-std=gnu11 -O0 -ffp-contract=off -fno-fast-math -Wall -Wextra`（shim 付きは加えて `-msse2 -msimd128 -include shim/wasm_compat.h -Irt-wasm`）。
  - 生成 C: `-std=gnu11 -O0 -w -fno-builtin -ffp-contract=off -fno-fast-math -ferror-limit=0`（shim 付きは加えて `-msse2 -msimd128 -include … -Irt-wasm`、実行用は `-finstrument-functions` も）。emcc の同時実行は 2 本。
- 同じ `-msse2` を単独で渡すと emcc が拒否する（実測）: `passing any of -msse, -msse2, … also requires passing -msimd128`。`-msimd128` だけだと `emmintrin.h` が `#error "SSE2 instruction set not enabled"`。

### 作成したファイル（新規のみ）

`lift/out-wasm/shim/wasm_compat.h`（`syscall(SYS_gettid)` を `gettid()` に読み替える。`-include` で全 TU に入れる）、`shim/fle_mxcsr_flags_soft.h`（`fle_mxcsr_flags.h` のソフト版。同じ名前と署名）、`shim/wasm_stubs.c`（`memfd_create` のスタブ）、`shim/wasm_mmap_probe.c`（`sysconf`/`mmap`/`munmap`/`mprotect`/`madvise` の応答。**probe 専用で設計案ではない**）、`shim/wasm_mmap_probe_log.c`（呼び出しをログに出す複製）、`harness/main.c`（wasm 用の最小 main。`setrlimit`/`mmap` なし）、`harness/image.c`（`lift/harness/image.c` の複製。`mmap` を `memset` に替えただけ）、`harness/trace.c`（`out-pdata/trace.c` の複製。signal 処理を除き `abort` を `--wrap`）、`harness/trace_hash.c`（`trace.c` の複製。ハッシュのみ変更）、`harness/boot.h`（複製）、`run_node.cjs`、`run.sh`、`difftest/*`（native と wasm の突合用）、`rt-wasm/`（`runtime` の複製。`fle_mxcsr_flags.h` だけ shim に替えた）、`snap/`（`runtime` と `harness` の複製）。

## 2. runtime の `emcc -c`（31 本）

### 2.1 そのまま（`rt-summary-asis.txt`）

21 成功、10 失敗。失敗は 2 種類の原因だけ:

| 原因 | ファイル（本数） | エラー | 件数 |
|---|---|---|---|
| `syscall()`/`SYS_gettid` が emscripten の libc に無い | `fle_glob`、`fle_imports`、`fle_ole`、`fle_oleaut`、`fle_proc`、`fle_sys`、`fle_sysinfo`、`fle_vmap`、`fle_vol`（9 本） | `call to undeclared function 'syscall'`、`use of undeclared identifier 'SYS_gettid'`（`fle_imports.c` は 2 か所） | 10 か所（各ファイルの `syscall(SYS_gettid)`: glob:21、imports:13,73、ole:24、oleaut:23、proc:20、sys:38、sysinfo:21、vmap:26、vol:24） |
| `fle_sse.h` → `fle_mxcsr_flags.h`（asm） | `fle_sse_more`（1 本） | `"SSE2 instruction set not enabled"`（`emmintrin.h`）、`"SSE instruction set not enabled"`（`xmmintrin.h`）、`invalid output constraint '+x' in asm`（19 件で打ち切り） | 20（`-ferror-limit` の上限） |

- 警告は 2 件: `fle_proc.c:105` `fle_thread_prio` set but not used、`fle_x87.c:1027` `shift count >= width of type`（§4.5）。
- `fle_sse.h` を include するのは `fle_sse_more.c` と生成 C だけ。他の runtime（`fle_x87.c` など）は `emmintrin.h` に依存せずコンパイルできる。`fle_mxcsr.c`、`fle_cpuid.c`、`fle_x87ctl.c` もそのまま通る。
- 事前に**確認した範囲では**、runtime に `fork`、`signal`、`setjmp`/`longjmp`、`pthread`、`fopen`、`__builtin_ia32_*`、`long double`、`cpuid.h` は無い（grep で 0 件）。

### 2.2 `-msse2 -msimd128` を足したとき（`rt-summary-b.txt`）

21 成功、10 失敗（同じ 10 本）。`emmintrin.h` の `#error` は消え、`fle_sse_more` の残りのエラーは `fle_mxcsr_flags.h` の asm 制約: **`invalid output constraint '+x' in asm`（19 件 + `too many errors` で打ち切り）**。つまり clang の wasm 向け SSE エミュレーション（`-msse2 -msimd128`）は intrinsic を通すが、**x86 のインライン asm（`"+x"`、`"=@ccc"`、`ldmxcsr`、`stmxcsr`）は通さない**。

### 2.3 shim 付き（`rt-summary-c.txt`）

31 成功、0 失敗、警告は 2 件（上と同じ）。shim の中身は §3.2、§5。

## 3. x86 依存の一覧

行番号は `lift/runtime`（HEAD 1f88ad5 + 作業ツリー）。「規模」は**推測**。

### 3.1 一覧

| # | 種類 | ファイル:行 | 件数 | wasm での状態（実測） | 対処の案 | 規模（推測） |
|---|---|---|---|---|---|---|
| D1 | インライン asm（ホスト SSE 命令の実行） | `fle_mxcsr_flags.h`:40、45、49、91、141、161、178、199 | 8 か所の asm、マクロ展開でヘッダ定義は約 62 種の `fle_mx_*`（演算・変換・比較。うち生成 C が使うのは 38 種 + 補助の `fle_mx_m32`/`fle_mx_m64`） | **コンパイル不可**（`invalid output constraint '+x'`、`'=@ccc'`） | 下記 §3.2（案 A〜C） | 案 A: 小（shim 214 行を作成済み）。案 B/C: 中〜大 |
| D2 | MXCSR 状態ビットをホスト実行で取得 | `fle_mxcsr_flags.h`:38–55（`fle_mx_begin`/`fle_mx_end`） | 生成 C から 49,997 呼び出し点（38 種の計算ヘルパ 30,359 + `fle_mx_m32/m64` 19,638） | wasm には MXCSR がない。浮動小数例外フラグ（IE DE ZE OE UE PE）を**取れない** | §3.2 | 案 B（ソフトで例外検出）: 大 |
| D3 | MXCSR 制御ビット（RC、FTZ、DAZ）をホストに設定 | 同上 :38–55 | 同上 | wasm の f32/f64 演算は常に RN、FTZ/DAZ なし。RC を変えられない | 同上 | 同上 |
| D4 | `<emmintrin.h>` の intrinsic | `fle_sse.h`:13、129–135（`_mm_sqrt_ss`、`_mm_sqrt_sd`、`_mm_cvtsi32_si128` 他）、`fle_mxcsr_flags.h`:20（`__m128i`） | 2 関数（`fle_sqrtss`、`fle_sqrtsd`）+ ヘッダ 2 本 | `-msse2 -msimd128` を付ければ**コンパイルできる**（`probe/sse2only.c` で確認）。付けないと `#error` | フラグで足りる。または C の `sqrtf`/`sqrt` に置換（NaN の結果は要検証） | 小 |
| D5 | `syscall(SYS_gettid)` | glob:21、imports:13,73、ole:24、oleaut:23、proc:20、sys:38、sysinfo:21、vmap:26、vol:24 | 10 か所 / 9 ファイル | **コンパイル不可**（libc に無い） | `gettid()` か定数 1 に置換（wasm は 1 スレッド）。`-include` の shim で通した | 小（10 行） |
| D6 | ホストページサイズの検査 | `fle_sys.c`:295–296（`sysconf(_SC_PAGESIZE) != 4096` なら STOP） | 1 | wasm の `sysconf(_SC_PAGESIZE)` は **65536**。実行時に停止（実測: 関数 86、入口 117 で `fle: host page size 65536 is not 4096`） | 検査を外す、または wasm では 64 KiB ページの扱いに設計を変える | 小（検査）〜中（ページ粒度の再設計） |
| D7 | `mmap`/`munmap`/`mprotect`/`madvise` によるアドレス空間の予約とコミット | `fle_sys.c`:346–359、406、432、482、501、558、577–578 | 約 16 呼び出し | コンパイル、リンクは通る。**挙動は別物**: emscripten は匿名 `mmap` を `memalign` + `memset` で実装し、アドレス指定（`want != NULL`）は `EINVAL`、`mprotect` は何もしない（`return 0`）、`madvise` は無視、`MAP_NORESERVE` の遅延コミットなし | 設計変更: ゲストの仮想アドレス空間を wasm 線形メモリ内の固定オフセットに載せる（旧実績: `GLOBAL_BASE` でイメージを置く）。PROT の強制は不可（§4.4）。probe では `--wrap` で `aligned_alloc`/no-op を返した | 中〜大 |
| D8 | `memfd_create`、`ftruncate`、`MAP_SHARED` の `mmap(fd)` | `fle_vmap.c`:373–375、466–475、510 | 3 呼び出し群 | **リンクエラー**（`undefined symbol: memfd_create`。宣言はあるが emscripten の libc に実装がない。**全リンクで唯一の未定義シンボル**）。`ftruncate`、`mmap(fd)` は MEMFS 上なら動く可能性があるが未確認 | `CreateFileMappingW` を線形メモリ上の別実装にする（同じ内容を共有する複数ビューは `MAP_SHARED` 相当を自前で実現する必要）。probe は `abort` するスタブ | 中 |
| D9 | `MAP_FIXED_NOREPLACE` | `fle_sys.c`:12–13（未定義なら 0x100000）、346 | 1 | emscripten の `<sys/mman.h>` に定義あり。意味は実装されない（D7） | D7 に含まれる | — |
| D10 | `__thread` | `fle_locale.c`:51 | 1 | コンパイル通過（スレッドなしなら通常変数と同じ） | 不要 | なし |
| D11 | `__atomic_*` | `fle_imports.c`:27,29,84 | 3 | 通過 | 不要 | なし |
| D12 | `unsigned __int128`、`__int128` | `fle_flags.h`:71,75,90,150,203,205,216、`fle_int_ext.c`:14–45、`fle_int_more.c`:13–23、`fle_x87.c`:14（`u128`）、`fle_int_ext.h`:10、`fle_int_more.h`:11、生成 C 1,524 か所 | runtime 約 25 行 + 生成 C 1,524 | wasm32 でも**コンパイル通過**（`__SIZEOF_INT128__` = 16、clang が `__multi3` などの compiler-rt で実装）。native と wasm の突合: `div`/`imul1`（200,000 ケース）、`idiv`（200,000 ケース）の hash **一致** | 不要 | なし |
| D13 | `long`（wasm32 では 4 バイト） | `fle_x87.c`:43、181、184、201、202、242、283、597、654、672、953、1025–1027 | 約 12 か所 | コンパイル通過。**1 か所に警告**: `fle_x87.c`:1027 `(long)1 << 40`（`shift count >= width of type`）。**挙動が変わる**（§4.5） | `long` を `int64_t` にする | 小 |
| D14 | `long double` | — | 0 | 使用なし（wasm32 は 16 バイト、soft f80 は自前の `fle_f80`） | 不要 | なし |
| D15 | `uintptr_t` ↔ guest アドレスの恒等写像（`fle_ld`/`fle_st`） | `fle_runtime.h`:16,21（`(uintptr_t)a`） | 生成 C に `fle_ld` 817,253、`fle_st` 598,952 | **コンパイル通過。意味が変わる**（§4.3）: wasm32 でポインタは 32 bit。guest アドレスの上位 32 bit は**黙って捨てられる**（`ld(buf + 2^32) == ld(buf)` を実測）。guest アドレス 0 は読める（trap しない） | `fle_ld`/`fle_st` に範囲検査を入れる（性能への影響は大）か、guest アドレス空間を 32 bit に収める（旧実績: `LVA = RVA + GLOBAL_BASE`、lowbase 方式） | 中（設計。DLL の ImageBase は 0x400000 と低いが、`FLE_PROCESS_HEAP` 0x7FF700000000、`FLE_VMAP_HANDLE_BASE` 0x7FFA70000000 など上位アドレスが runtime にある） |
| D16 | x87 のソフト f80（`fle_x87.c`） | 1,034 行 | 生成 C から `fle_x87_*` 2,217 行（`fle_x87_arith_st` 1,418、`fle_x87_fld_m` 774、`fle_x87_fst_m` 512、`fle_x87_arith_m` 462 ほか） | **ホスト実行ではない**（`fle_x87.c` は `fenv` も `long double` も asm も使わない。コメントの "host" は仕様の曖昧な箇所をホスト CPU で測って決めたという意味）。コンパイル通過。native と wasm の hash: add/sub/mul/div/sqrt/rndint/chs/abs は**一致**。**`fscale` だけ不一致**（§4.5、D13 が原因と推測） | `long` → `int64_t`（D13） | 小 |
| D17 | `cpuid` | `fle_cpuid.c`（37 行） | 8 回（DllMain 経路） | **ホスト実行ではない**（固定応答）。通過 | 不要 | なし |
| D18 | `ldmxcsr`/`stmxcsr`（ゲストの命令） | `fle_mxcsr.c`（21 行）、生成 C の `fle_mxcsr_load`/`fle_mxcsr_store` 9 か所 | 生成 C 9 + DllMain 経路 2 | **ホスト実行ではない**（`fle_mxcsr` 変数への読み書き）。通過。ただしゲストが LDMXCSR した値は D1〜D3 の経路でだけ効く（D1 の asm が `fle_mxcsr` から制御ビットを取る） | D1〜D3 と同じ | — |
| D19 | MMX（`fle_mmx.h`）、VEX（`fle_vex.h`）、SSE 整数（`fle_sse_int2.h`、`fle_sse_shuf.h`、`fle_sse_more.c`） | — | — | 自前の C 実装（asm、intrinsic なし）。通過 | 不要 | なし |
| D20 | ハーネス: `setrlimit`、`mmap`（スタック、DLL イメージ）、`MAP_FIXED_NOREPLACE` | `harness/main.c`:30,51–52、`harness/image.c`:91–93 | 3 | wasm では使えない。`out-wasm/harness/` に複製を作り、`memset`/`malloc` に替えた | 設計: イメージを `GLOBAL_BASE` の上に置く（旧実績）。本調査は ImageBase 0x400000 を線形メモリの低位に置いた（イメージ 0x400000–0x1cc0000、`GLOBAL_BASE` 256 MiB の下） | 小 |
| D21 | ハーネス: `sigaction`、`sigaltstack`、`alarm`、`SIGSEGV` 等の捕捉 | `out-pdata/trace.c`:181–234（`on_signal`、`trace_init`） | 1 組 | wasm では `SIGSEGV`/`SIGFPE`/`SIGALRM` が来ない。`abort()` は `__wrap_abort` で捕えた | wasm の trap（`RuntimeError`）を JS 側で捕える。時間制限は JS 側 | 小 |
| D22 | ハーネス: `-finstrument-functions` のハッシュ | `out-pdata/trace.c`:`hset_add`/`hset_has`（`(a >> 4) & (SEEN_SIZE-1)`） | 2 関数 | wasm の関数ポインタは**テーブルのインデックス**（小さい連番）。`a >> 4` では下位ビットが捨てられ、**衝突が集中**して線形探査が長くなる。実行 7.5 秒（`--cpu-prof` で 5.77 秒が wasm 関数 2 つ（wasm-function[16]、[18]）。それがハッシュ探索であることは、ハッシュを替えると 7.5 → 1.5 秒になったことからの推測で、関数名は未確認）。`trace_hash.c`（乗算ハッシュに替えた）で **1.5 秒** | ハーネス側の問題。runtime/生成物は無関係 | 小 |

### 3.2 D1〜D3（MXCSR / ホスト SSE 実行）の案の比較

根本の事実（実測）: `fle_mxcsr_flags.h` は、1 命令ごとに「ホストの MXCSR をゲスト値で設定 → ホストで同じ SSE 命令を実行 → ホストの MXCSR 状態ビットを読む → ゲストの `fle_mxcsr` に OR」を行う（ADR/決定 0038）。wasm には MXCSR も SSE 命令の直接実行もない。ゲスト側の観測可能な挙動は、結果のビット、`fle_mxcsr` の状態ビット（ゲストが `stmxcsr` で読む）、未マスク例外の abort。

| 案 | 内容 | 結果の bit | 状態ビット（IE…PE） | RC/FTZ/DAZ | 規模（推測） |
|---|---|---|---|---|---|
| A. 状態ビットなしのソフト版（作成済みの shim `fle_mxcsr_flags_soft.h`、214 行） | wasm の f32/f64 演算（RN、決定的）と `fle_sse.h` の NaN 規則で計算。状態ビットは**立てない**、RC/FTZ/DAZ は無視（`ROUND*` の imm だけ反映） | RC=0（既定）で 4,960,000 件中、host 実行と **結果が違うのは `round*` で入力に NaN があるときだけ**（RC=0 の 1,240,000 件中 3,291 件、すべて NaN 入力）。RC=1〜3 では大量に違う（RC=1: 297,480、RC=2: 297,492、RC=3: 266,642 件） | **host は 3,385,217 件（全 4,960,000 件の 68%）で状態ビットを立てる。shim は 0 件** | 無視 | 小（実装済み）。ただし**意味が変わる**（下の注） |
| B. ソフトで例外検出まで | f32/f64 のソフト演算（または f64 で計算して丸めを検査）で IE/DE/ZE/OE/UE/PE を判定し、RC/FTZ/DAZ も実装する。結果は 127,104 ケースの host-diff で検証できる（`mxcsr_flags.rs` の既存テストの流用） | 一致が目標 | 実装する | 実装する | 大（60 ニーモニック。`fle_x87.c` と同程度の規模。粗い推測で 800–1500 行） |
| C. ゲストの MXCSR 状態ビット読み出し側の設計変更 | 「ゲストが `stmxcsr` で状態ビットを読まない」ことが静的に確認できる範囲では、状態ビットを持たなくてよい。確認できない箇所だけ B | — | 条件付き | — | 調査が先（§3.3） |
| D. wasm 側でも実 CPU を使う（JS から呼ぶ等） | 不可能（wasm から MXCSR を読む手段はない） | — | — | — | — |

- **注（実測）**: DllMain の経路（198 関数）では `fle_mxcsr` の状態ビットは 0 のまま（`fle_mxcsr` = 0x1f80、`& 0x3f` = 0）、SSE の計算ヘルパ `fle_mx_*` は 0 回。ゲストの LDMXCSR の定数は 0x13a5054 = 0x1f80 と 0x13e4d50 = 0x9f80（FTZ 付き、`fle_mxcsr_load` は 1 回だけ呼ばれた）。**FTZ（0x8000）を設定するコードが DLL にある**（0x63a6d0 の `ldmxcsr [0x13e4d50]`）。案 A は FTZ を無視するので、そこを通る実行では結果が変わり得る（推測: そこへ到達した時点でのみ問題）。
- 状態ビットが 68% の計算で立つ（実測）。多くは PE（inexact）と推測される。ゲストが `stmxcsr` で状態を読んで分岐するコードがあれば、案 A は結果を変える。それを DLL で調べる作業（`stmxcsr` の使われ方。生成 C の Ldmxcsr/Stmxcsr は 9 か所: Stmxcsr 3 か所（0x40a388、0x40a3b2、0x475c61）、Ldmxcsr 6 か所（0x40a36c、0x40a39d、0x40eeab、0x40f530、0x475c8a、0x63a6d0））は、本調査では**していない**。

### 3.3 推奨（1 つ）

案 A を土台にして、`stmxcsr` の使用箇所（生成 C の `fle_mxcsr_store` 3 か所）の静的確認を先にやる。ゲストが状態ビットを観測しない（または観測しても分岐に使わない）ことが示せれば、A のまま wasm の精度の主張を「結果 bit は一致、MXCSR 状態ビットは非モデル」と明記できる。示せなければ B。理由: A は 214 行で実装済み、`RC=0` では結果が host 実行と一致する（`round*` の NaN 入力を除く）。B は規模が大きく、状態ビットの有無がゲストの結果に影響するかが先に判明すれば避けられる可能性がある（推測）。

## 4. 実測の詳細

### 4.1 生成 C の `emcc -c`

| 条件 | 成功 / 失敗 | エラー | 時間（`/usr/bin/time`） |
|---|---|---|---|
| runtime そのまま（`gen-summary-asis.txt`） | 0 / 16 | 各ファイル 56 エラー、16 ファイル合計 896 件（`invalid output constraint '+x'` 672、`invalid input constraint 'x'` 128、`invalid output constraint '=@ccc'` 64、`"SSE2 instruction set not enabled"` 16、`"SSE instruction set not enabled"` 16。`fle_mxcsr_flags.h` の同じ 56 か所が 16 ファイルそれぞれに出る） | 1.2–2.3 秒（エラーで止まる） |
| `-msse2 -msimd128`（`gen-summary-b.txt`） | 0 / 16 | 各 54 エラー、合計 864 件（`+x` 42 + `x` 8 + `=@ccc` 4 = 54 × 16。`#error` は消える） | 1.1–2.4 秒 |
| shim 付き（`gen-summary-c.txt`） | **16 / 0** | 0 | 3.6–8.8 秒、合計 94.5 秒（同時 2 本）、最大 RSS 1,793 MB（part-14）。`.o` 合計 161 MB |
| shim + `-finstrument-functions`（`gen-summary-inst.txt`、リンクに使用） | **16 / 0** | 0 | 3.8–9.4 秒、合計 96.3 秒、最大 RSS 1,812 MB。`.o` 合計 166 MB |

- 生成 C の x86 依存は `fle_sse.h` が include する `fle_mxcsr_flags.h` だけ（生成 C 自体に asm、`__builtin_ia32_*`、`_mm_*` は無い。`__builtin_ctzll` が 5 回、`__int128` が 1,524 回、いずれも wasm32 でコンパイルできる）。
- メモリ: 生成 C 1 TU のコンパイルで最大 1.8 GB。clang（ネイティブ）の `-O0 -g` での同規模の記録 1,204 MB より大きいが、分割を維持すれば 2 本同時で収まる。分割を外した 1 TU は試していない（ネイティブで 6 GB 制限の下で OOM した記録が `pdata-seeds-run.md`）。

### 4.2 リンク

- 構成: `obj-h/{main,image,trace,wasm_stubs}.o` + 生成 C 16 + runtime 31 を `-O0 -sALLOW_MEMORY_GROWTH=1 -sSTACK_SIZE=8MB -sGLOBAL_BASE=268435456 -sENVIRONMENT=node -sMODULARIZE=1 -sEXIT_RUNTIME=1 -sNODERAWFS=1 -Wl,--initial-memory=0x28000000 -Wl,--max-memory=0x80000000 -Wl,--wrap=fprintf -Wl,--wrap=fputs -Wl,--wrap=abort -lm`。
- 1 回目は**失敗**: `wasm-ld: error: obj-c/fle_vmap.o: undefined symbol: memfd_create`（唯一の未定義シンボル）。`shim/wasm_stubs.c` に `abort` するスタブを置いて通った。
- 結果: `flengine.wasm` **115,518,408 B**（約 110 MiB）、`flengine.js` 177,581 B、リンク **1.26 秒**、最大 RSS **634 MB**（`link-log/link.time`）。旧実績の「all-pdata 相当で wasm 約 213 MB」より小さい。差の原因は未調査（推測: 旧実績の `-g1`、旧生成コードの命令あたりのコード量の違い）。
- `-Wl,--initial-memory` は 640 MiB で通った。実行時の node の最大 RSS は約 395 MB。

### 4.3 node での実行

- コマンド: `node run_node.cjs <dll>`（`--stack-size` なしで動く。付けても同じ）。DLL は `-sNODERAWFS=1` で node のファイルを読む。
- **mmap 系の応答なし**（`bin/flengine.wasm`）: 停止 `fle: host page size 65536 is not 4096`（`fle_sys.c`:296）、関数 86、入口 117、深さ 18（`run-node.txt`）。`GetProcessHeap` か `VirtualAlloc` 系の最初の呼び出し（`fle_mem_init`）で起きる。
- **probe の応答つき**（`bin/flengine-mm.wasm`、`--wrap` で `sysconf`→4096、匿名 `mmap` → `aligned_alloc`+`memset`、`munmap`/`mprotect` → no-op、`madvise(DONTNEED)` → `memset`）:
  - 停止: `fle: GetLocaleInfoW: LCType value for en-US is not on the MSDN pages …`（LCType 0x37）。**ネイティブ `out-loc/run.txt` と同じ停止点**。
  - 関数実行数（distinct）**198**、関数入口 **984**、呼び出し深さ **11**、スタック（上位 10 段）と entry log 256 件が一致。`diff <(grep -v '^image:\|^calling' out-loc/run.txt) <(grep -v … out-wasm/run-node-mm.txt)` は差なし。違うのは `rsp=` の値だけ（0x725ea4ffffc0 と 0x11c496d0）。
  - 終了コード 134（`__wrap_abort` が `_exit(134)`）。
  - 時間: `trace.c` そのままで **7.5 秒**。`trace_hash.c`（ハッシュを乗算に替えた）で **1.5 秒**（`bin/flengine-mm2.wasm`、出力は同一）。ネイティブは 0.02 秒。1.5 秒の内訳（`--cpu-prof`）: `jsStackTrace` 0.63 秒 + `defaultPrepareStackTrace` 0.40 秒（`-finstrument-functions` の `__cyg_profile_func_enter` が `_emscripten_return_address` を呼ぶため、984 回で JS のスタックトレースを作る）+ 他（推測: ほとんどが node 起動と wasm のコンパイル。`image_load` だけ（entry 0x1）は 0.31 秒）。
- この経路で host に要求した操作（`wasm_mmap_probe_log.c` のログ、実測）: `sysconf(30)` 1 回、`mmap(addr=0, len=1372160, 匿名)` 1 回（0x14f000 バイト = 1.3 MB の予約）、`mprotect(…, 1310720, PROT_READ|PROT_WRITE)` 1 回、`munmap` 2 回（12,288 と 49,152 バイト）。つまり DllMain の 198 関数では `VirtualAlloc` が 1 回呼ばれただけで、アドレス指定（`want`）なし。

### 4.4 `mprotect`/PROT の強制と `fle_ld` の範囲

- emscripten の `__syscall_mprotect` は `return 0`（"let's not and say we did"）、`__syscall_madvise` は無視、`__syscall_mmap2` はアドレス指定で `-EINVAL`（`/opt/emsdk/upstream/emscripten/system/lib/libc/`）。ゲストの `PAGE_NOACCESS` などの強制はできない。native では guest の不正アクセスは SIGSEGV になる（`trace.c` の `on_signal` が報告）。wasm ではメモリアクセスは範囲内なら成功する（範囲外は `RuntimeError: memory access out of bounds`）。
- 実測（`probe/ptr.c`）: wasm32 で `ld(buf + 2^32) == ld(buf)`（上位ビットの切り捨て）、`ld(0)` は trap せず読める。`sizeof(long)` = 4、`(long)1 << 40` は 256（native では 2^40）。

### 4.5 native と wasm の突合（`difftest/`）

| 対象 | 方法 | 結果 |
|---|---|---|
| host 実行（`fle_mxcsr_flags.h`）と soft shim（`fle_mxcsr_flags_soft.h`）、どちらも native clang | 乱数入力（NaN、Inf、0、非正規化数、1 付近を偏らせる）、56 操作（`op2` 38 種、`cvt*2si` 8 種、`cvtsi2*` 4 種、`*comis*` 4 種、`cmp*` 32 imm、`round*` 64 imm）、RC=0〜3 の各 1,240,000 件、計 **4,960,000 件** | RC=0: result_diff **3,291**（`round*` のみ、他の全操作は 0）。RC=1: 297,480、RC=2: 297,492、RC=3: 266,642。host が状態ビットを立てた件数 **3,385,217**、shim は 0 |
| `round*`（RC=0）の 3,291 件の原因 | `rnd_diag.c`: NaN を含む入力と含まない入力に分類 | **NaN を含まない入力での差は 0**。差は 101,311（標本 1,280,000 件中。別のサンプル）がすべて NaN 入力。shim は `fle_mx_rnd` で NaN を通すが、host の `roundss` は QNaN 化する、ことが原因と推測（個別のビットは見ていない） |
| shim の native と wasm（emcc、`-msse2 -msimd128`） | `hash_d.c`: shim の全出力の FNV-1a、`round*` は NaN 入力を除く | **一致**（`cases=1205118`、hash `adff26b0476ee3e2`）。NaN 入力の `round*` は native と wasm で**一致しない**（同じソースでも `fle_mx_rnd` の NaN 経路が wasm の NaN ビットで違う。個別の原因は未調査） |
| `fle_div`、`fle_imul1`、`fle_idiv`（`__int128`）の native と wasm | `int_x87_d.c`、200,000 + 200,000 ケース | **一致** |
| ソフト x87（`fle_x87.c`）の native と wasm: add/sub/mul/div（ST、pop）、sqrt、rndint、chs、abs | 各 30,000 ケース（m64 で 2 値をロード、結果を m64 に書き出し + sw + ST0 を hash） | **一致**（7 操作） |
| ソフト x87 `fscale` | `fscale_probe2.c`: ST0 の指数を −20〜59、ST1 を小さな値で 60,000 ケース | **不一致 18,837 件**（31%）。ST0 の指数が 31 以上の入力で、32 では 754 件中 377 件が不一致（約半分）。`fle_x87.c`:1027 の `(long)1 << 40` と `(long)(b.m >> (63 - b.e))`（`long` が wasm32 で 32 bit）が原因と**推測**（未検証。`fle_x87.c` を変えて確認する作業は本調査の範囲外） |

- 注: `fle_x87_fscale` は生成 C に 1 か所、DllMain 経路では未実行。`F2xm1`/`Fsincos` は生成 C で未対応（`fle_unsupported`）のまま。

## 5. 作った shim の要点

- `shim/fle_mxcsr_flags_soft.h`: `fle_mx_*` の全種（約 62 種。`add/sub/mul/div` × ss/sd/ps/pd、`min/max`、`sqrt`、`cvt*`、`cmp*`、`round*`、`cvtsi2*`、`cvt*2si`、`*comi*`）を C で実装。NaN は `fle_sse.h` の規則に合わせ、無効演算の結果は x86 の QNaN（0xFFC00000 / 0xFFF8000000000000）。実測（`probe/nan.c`）: `inf-inf`、`0/0`、`inf*0`、`sqrt(-1)` は native（host SSE）と wasm で同じ値。
- **作っていないもの / 近似**: 状態ビットを立てない、RC/FTZ/DAZ を無視（`round*` の imm 以外）、ソフト実装は wasm の f32/f64 に依存（丸めは RN のみ）。
- `shim/wasm_mmap_probe.c`: probe 専用。PROT の強制なし、アドレス指定の `mmap` は失敗、512 MiB 超の予約は失敗、`munmap` は no-op。

## 6. 実測値の一覧

| 項目 | 値 |
|---|---|
| runtime の `emcc -c`（そのまま） | 21 / 31 成功、10 失敗（`syscall` 9 本、`fle_sse_more` 1 本） |
| runtime の `emcc -c`（`-msse2 -msimd128`） | 21 / 31 成功、10 失敗 |
| runtime の `emcc -c`（shim） | 31 / 31 成功 |
| 生成 C 16 分割の `emcc -c`（そのまま、`-msse2 -msimd128` 付き） | 0 / 16 成功（どちらも） |
| 生成 C 16 分割の `emcc -c`（shim） | 16 / 16 成功（`-finstrument-functions` あり、なし） |
| x86 依存（種類別、§3.1 の ID。分類は私の判断） | コンパイルを止めるもの 3 種（D1 asm、D5 `syscall`、D8 `memfd_create`〔リンク〕）、コンパイルは通るが意味が変わるもの 6 種（D2/D3 MXCSR、D6 ページサイズ、D7 mmap/mprotect、D13 `long`、D15 `uintptr_t` 32 bit、D22 ハッシュ）、問題なし 8 種（D4 のうち intrinsic はフラグで可、D9〜D12、D14、D17、D19） |
| リンク | 成功（shim 付き）。`flengine.wasm` 115,518,408 B、1.26 秒、最大 RSS 634 MB |
| node の停止点（probe の応答つき） | GetLocaleInfoW LCType 0x37、関数 198、入口 984、深さ 11（ネイティブと一致） |
| node の実行時間 | 7.5 秒（ネイティブのハーネスの `trace.c` をそのまま）、1.5 秒（ハッシュ修正）。ネイティブ 0.02 秒 |
| ネイティブ 4,960,000 件の host 実行 vs shim | 結果差 864,905（RC=0 で 3,291、すべて NaN 入力の `round*`）、状態ビット: host 3,385,217 件、shim 0 |
| `fscale` の native vs wasm | 60,000 件中 18,837 件が不一致（推測の原因: `long` の幅） |

## 7. 未確認、本調査の限界

- DllMain の 198 関数に **`fle_mx_*` と x87 の演算は 1 回も出てこない**（§0）。この先（停止点より後）で SSE/x87 が大量に実行されたとき、wasm の結果が native と一致するかは**未確認**。突合（§4.5）は単体テストに基づく。
- `-O0` のみ。最適化（`-O1` 以上）での wasm のサイズ、時間、結果は未測定。`-g1` は付けていない。
- `--initial-memory` は 640 MiB で固定。ゲストが大きな `VirtualAlloc` を要求する経路（停止点より後）でのメモリは未測定。`MAXMEM` は 2 GiB。
- 生成 C の 1 TU（分割なし）の emcc は試していない。
- ブラウザ（`-sENVIRONMENT=web`）は試していない。`-sNODERAWFS` は node 限定。
- `CreateFileMappingW`/`MapViewOfFile` の wasm 実装は未検討（D8）。
- `setjmp`/`longjmp`、`fork`、`signal` の依存は runtime、生成 C に無い（grep）。`-sSUPPORT_LONGJMP` は不要。
- 旧調査の「wasm 約 213 MB」との差（今回 110 MiB）の原因は未調査。
- `docs/investigations/2026-10-19-environment.md` は存在しなかったので参照できていない。
- ADR-0005 の本文には「A3」が見当たらなかった（`docs/decisions/0005-mxcsr.md` は MXCSR の決定で、別物）。依頼文の「A3 = 生成 C が emcc でリンクできる」を基準にした。

## 8. 再現

```
bash lift/out-wasm/run.sh                 # 全工程（各工程は出力があればスキップ。上書き・削除しない）
bash lift/out-wasm/run.sh rt-asis         # runtime をそのまま emcc -c（31 本、成否の一覧）
bash lift/out-wasm/run.sh gen split gen-asis gen-shim gen-inst harness link run-mm2 difftest
```

- 工程: `gen`（fe-lift で生成）、`split`（16 分割）、`rt-asis`、`rt-shim`、`gen-asis`、`gen-shim`、`gen-inst`、`harness`、`link`、`run`（mmap 応答なし）、`link-mm`/`run-mm`（応答つき）、`link-mm2`/`run-mm2`（応答つき + ハッシュ修正）、`difftest`。
- 環境変数: `JOBS`（emcc の同時数、既定 2）、`INITMEM`、`MAXMEM`。
- 注意: `run.sh` の `difftest` は `clang`（ネイティブ）を使う。`rt-asis` の出力は `rt-summary-asis.txt`、生成 C は `gen-summary-*.txt`。

## 9. 残置物、触ったファイル

- 新規: `docs/investigations/wasm-build-probe.md`（本書）、`lift/out-wasm/` 以下すべて。主要: `run.sh`、`run_node.cjs`、`shim/`、`harness/`、`difftest/`、`rt-wasm/`、`snap/`、`out/`（生成 C と 16 分割 = 約 430 MB）、`obj-*`、`bin/`（`.wasm` 3 本 = 約 332 MB）、`probe/`、`target/`（fe-lift の release ビルド）、`emcache/`、`tmp/`。合計約 1.2 GB。
- 削除禁止のため残っているもの: `lift/out-wasm/run.sh.new`（`run.sh` を直すために途中で作った一時コピー。内容は古い `run.sh` の一部の修正版。不要）、`difftest/` の途中版（`int_x87.c`、`int_x87_b.c`、`int_x87_c.c`: テストの誤りを直した前の版。`int_x87_d.c` が最終）、`hash_b.c`、`hash_c.c`、`fscale_probe.c`（`fscale_probe2.c` が最終）、`bin/flengine-mm.*`、`bin/flengine-mm3.*`（ログ版）、`out/wasm-pdata.c`（分割の元）、`probe/` の試験用ファイル。
- 変更していない: `lift/runtime`、`lift/harness`、`lift/crates`、他の `lift/out-*`。`git` 操作はしていない（`git status` と `git log` の参照のみ）。
