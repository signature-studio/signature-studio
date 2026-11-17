# ゲストアドレス → ホストポインタの直接変換の洗い出し（決定 0049 の穴 1。作成時は 0046）

日付: 2026-10-21。対象は `lift/runtime/` の `fle_*.c`、`fle_*.h`（35 本の `.c` と `.h`）。決定 0045 d が「未適用の経路」として挙げた `fle_sse.h` の `fle_ldx` 系と、runtime の各 `.c` の `(uintptr_t)` 変換を調べ、ゲストのアドレスを扱うものを `fle_addr` 経由にした。方式は `docs/decisions/0046-wasm-portability-fixes.md`、実行記録は `docs/investigations/wasm-portability-fixes-run.md`。「実測」はコマンドの出力、「推測」は未検証の解釈。

## 1. 方式

- `fle_addr(a, n, store)`（`fle_runtime.h`）は 0045 のまま。native の既定ではマクロで `((uintptr_t)(a))` に展開される。wasm（`FLE_CHECK_HI32`）では `a >> 32` が非ゼロなら `[fle_addr_hi32]` で止まり、`FLE_CHECKED_MEM` では領域も見る。
- 新規 `fle_addr_opt(a, n, store)`（同じヘッダ）: 「NULL を正当な値として受け取る API の引数」用。`a == 0` ならそのまま 0、それ以外は `fle_addr`。native ではやはり `((uintptr_t)(a))` に展開される。NULL の扱い（NULL なら `E_INVALIDARG`、`fle_stop` など）は呼び出し側が変換の後に判定する従来のまま。`fle_addr` を NULL にそのまま使うと、wasm の `FLE_CHECKED_MEM` が NULL を領域外として止めてしまい、API の NULL の規則（0042 など）を変えるため、NULL を通す形を足した。
- `n`（アクセスの大きさ）は、エラーメッセージとコスト用の情報。API が書く、または読むバイト数が分かるものはその値、分からない（ポインタを渡すだけで、先で別のコードが読み書きする）ものは 1 とした。

## 2. 変更したもの（`fle_addr` 36 か所、`fle_addr_opt` 19 か所。合計 55 か所）

grep（`uintptr_t`、`(void *)`、`(const void *)`、`(uint8_t *)` などの型へのキャスト。`uintptr_t` を使わない形のキャストは、型名つきの正規表現で探して 0 件）で洗い出した箇所のうち、ゲストのアドレスとして使うもの。行番号は変更後。変更前の `(uintptr_t)` は 71（残り）+ 変更した 55 のうち `(uintptr_t)` を使っていた分（`fle_ldx`、`fle_stx` を含む）で、変更前の件数そのものは数え直していない。

| ファイル | 行 | 内容 | 形 |
|---|---|---|---|
| `fle_sse.h` | 184 `fle_ldx` | 16 バイトロード（生成 C に `fle_ldx` 264、`fle_ldxa` 3,754。`fle_ldxa` は `fle_ldx` を呼ぶ。`fle_vex.h` の `fle_vld` 8、`fle_vst` 8 も通る） | `fle_addr(a, 16, 0)` |
| `fle_sse.h` | 194 `fle_stx` | 16 バイトストア（生成 C に `fle_stx` 206、`fle_stxa` 2,274。`fle_stxa` は `fle_stx` を呼ぶ） | `fle_addr(a, 16, 1)` |
| `fle_env.c` | 68, 288 | `fle_w_write` の書き先、`GetStartupInfoW` の構造体 | `fle_addr` ×2 |
| `fle_imports.c` | 174 | `LocalFree` の引数（NULL 可） | `fle_addr_opt` |
| `fle_locale.c` | 181, 383, 510, 512, 621, 623, 625, 626, 709, 711, 781, 783 | `GetCPInfo`、`GetLocaleInfoW`、`MultiByteToWideChar`、`WideCharToMultiByte`、`LCMapStringW`、`CompareStringW` のポインタ引数（NULL を API が定義済み） | `fle_addr_opt` ×12 |
| `fle_ole.c` | 226, 239, 251 | `CLSIDFromString` の出力、`IsEqualGUID` の 2 引数 | `fle_addr` ×4 |
| `fle_oleaut.c` | 111, 352, 366, 418, 430, 437, 492, 497, 500, 612, 811, 863 | `SysAllocStringLen` の入力、`SafeArrayCopy`、`VariantCopy` 系、`VariantCopyInd`、`SafeArrayGetElement`、`SafeArrayPutElement` のゲスト側のアドレス（ホスト側の `calloc` 結果の側は変えていない） | `fle_addr` ×17 |
| `fle_osver.c` | 159, 221 | `VerifyVersionInfoW`、`GetVersionExW` の構造体（NULL 可） | `fle_addr_opt` ×2 |
| `fle_reg.c` | 415, 454 | `RegSetValueExW` の入力、`RegQueryValueExW` の出力 | `fle_addr` ×2 |
| `fle_res.c` | 376 | `LoadStringW` の書き先と読み元 | `fle_addr` ×2 |
| `fle_sys.c` | 251, 768 | `GetSystemInfo` の出力、`HeapFree` の引数（NULL 可） | `fle_addr` 1、`fle_addr_opt` 1 |
| `fle_sysinfo.c` | 92, 130 | `GetLogicalProcessorInformation` の出力、`GlobalMemoryStatus` の出力 | `fle_addr` ×2 |
| `fle_uipref.c` | 96–98 | `GetThreadPreferredUILanguages` の 3 引数（NULL は `fle_stop` で判定） | `fle_addr_opt` ×3 |
| `fle_vfs.c` | 612, 651, 822 | `ReadFile` の書き先、`WriteFile` の読み元、`fill_find_data` の書き先 | `fle_addr` ×3 |
| `fle_vmap.c` | 158 | `VirtualQuery` の出力 | `fle_addr` |

合計: `fle_addr` 36（`fle_sse.h` の 2 を含む）、`fle_addr_opt` 19。`fle_ld`/`fle_st` が経由する 0045 の分とは別。

## 3. 対象外にしたもの（`(uintptr_t)` が 71 か所残る。理由つき）

「ゲスト → ホスト」ではなく、ホストのポインタ（自前のバッファ、`malloc` の結果、`mmap` の結果、アリーナ）を扱うもの。wasm32 でホストのポインタは 32 ビットで、ゲストに渡す値（`uint64_t` へ拡張）は上位 32 ビットがゼロになる。ゲストが上位ビットを書き換えた値を戻してきた場合は、戻す先で `fle_addr` / `fle_ld` が見る。

| 分類 | 場所 | 理由 |
|---|---|---|
| ホスト → ゲスト（`R[0]` や戻り値に自前のポインタを入れる） | `fle_env.c:272`（`cmd`）、`fle_glob.c:177,178,229`、`fle_imports.c:58`、`fle_ole.c:104,105`、`fle_oleaut.c:112`、`fle_sys.c:335,755`、`fle_time.c:101`（`&t`）、`fle_uipref.c:52`（`out`）、`fle_vex.h:102,103`（`fle_teb`、`fle_tls_values`） | ゲストアドレスをホストポインタに変えるのではなく、逆向き。変換する値はホストが持つポインタで、ゲストの値ではない |
| ホストの自前バッファのアドレスをゲストの形にして `fle_st`/`fle_ld` に渡す | `fle_oleaut.c:480,481,486,487,490,493,498,611`（`tmp`）、`fle_ole.c:270,272,275`（`hout`、`w`、`stk`） | 同じ。`tmp` は runtime のスタック上のバッファ。`fle_st` に入ると `fle_addr` を通る（wasm ではここで上位ビットが検査される） |
| ホストが `malloc`/`calloc` したブロックを、runtime の表（`fx_has`、`fle_pm_del` など）で生きていると確かめた後に `free` | `fle_glob.c:207`、`fle_ole.c:121`、`fle_oleaut.c:132,330,331,658,659` | 値は表の照合に通った（自分が返したポインタ）。ゲストが出した未検証のポインタではない |
| ホスト側の確保結果を、ゲストアドレスの変数として持つ | `fle_oleaut.c:349,350,655,656`（`dst`、`ddata`、`psa`、`data` の `calloc`）、`fle_oleaut.c:352,366` の書き先 `dst`、`d`（読み元の `src`、`s` は変更済み） | 書き先はホストが確保した直後のブロック、または `ddata`（ホストが確保したデータ領域）の内側 |
| mmap/munmap/mprotect/madvise の引数 | `fle_sys.c:442,444,453,454,455,467,524,687,708,709`、`fle_vmap.c:525–529,569` | ホストのアドレス空間の管理。`fle_map_reserve` などが返した値、または表（`fle_region`、`vm_view`）が持つホストのアドレス。ゲストが渡した値ではない（VirtualAlloc のゲスト指定アドレス `want` は、`mmap` の `MAP_FIXED_NOREPLACE` の引数として使うため、変換ではなくホストへの依頼。native 専用のコード。アリーナ版（wasm）では `fle_sys_host_reserve` が検査する） |
| アリーナ内の範囲 | `fle_sys.c:521`、`fle_vmap.c:404,405,511`（`abuf`、`a`、`x->buf + off`） | `fle_sys_host_reserve` が返したホストの範囲 |
| 変換の定義そのもの | `fle_runtime.h:50,53,58,62,63` | `fle_addr` の実体 |
| ホストの関数ポインタ | runtime に無い（`(uintptr_t)fn` は `lift/harness/trace.c` と `lift/harness/wasm/trace.c` にあるが、harness はゲストのアドレスを扱わない） | — |
| コメント | `fle_runtime.h:15`、`fle_mxcsr_flags.h:6` | 本文の説明 |

## 4. runtime の外（参考。変更なし）

- `lift/harness/image.c:96–103`、`main.c`、`wasm/main.c`、`oracle/oracle_common.c`: ハーネスと oracle のホストコード。ゲストのアドレスを実行中に受け取る経路ではない（イメージを置く場所、スタックの初期値）。
- `lift/crates/fe-lift/src/insn_int.rs:384`: 生成 C の `R[7] = (uint64_t)(uintptr_t)(M + 16);`（ホストのバッファのアドレスをゲストのレジスタに入れる、ホスト → ゲスト）。生成器は変えていない。
- 生成 C に `fle_ldx`/`fle_ldxa`/`fle_stx`/`fle_stxa` の呼び出し 264 + 3,754 + 206 + 2,274 = 6,498 か所は、`fle_sse.h` の 2 関数の修正だけで通る。**生成 C の再生成は不要**（`out-wasm2/out/` の 16 分割に対して `emcc -c` を通し直した。実行記録 §2）。

## 5. 確認（実測）

- native の既定: `clang -E` の展開で `fle_ldx`、`fle_stx` は `memcpy(&v, (const void *)((uintptr_t)(a)), sizeof v)`、`memcpy((void *)((uintptr_t)(a)), &v, sizeof v)`（従来と同じ変換、呼び出しなし）。
- native の生成 C オブジェクト（`out-wasm2/native/obj/wasm-pdata-part-{00,05,14}.o`）を修正前後で `objdump -d` 比較して、命令列が同一（差 0）。runtime の native オブジェクト 32 本は、29 本が命令列の差 0。差が出た 3 本（`fle_glob.o`、`fle_imports.o`、`fle_sys.o`）は穴 3 の allocation の 4 か所（`fle_host_size_fits`）で、`-O0` ではサイズ比較が 1 回入る（native では常に真）。ここはアドレスの表記を除いた比較ではないため、差の行数（2,588 など）は命令のずれを含む。挙動の同一性は `cargo test --workspace`（616 件、失敗 0）で確かめた。
- wasm: `fle_ldx` と `fle_stx` に上位ビット付きのアドレスを渡す最小の C（emcc、node）が、どちらも `[fle_addr_hi32]` の名前付き abort で止まる（load: `0x100010d90 (16-byte load)`、store: `0x100010d70 (16-byte store)`。修正前の経路は上位ビットを黙って捨てて成功した）。低いアドレスは通る。
- `-DFLE_CHECKED_MEM` で runtime の全 32 本を `emcc -c` が通る（警告 1: 既存の `fle_proc.c:105`）。native の `-DFLE_CHECK_HI32` でも 32 本 `clang -c` が通る。
- **領域チェックのコスト**（0045 の実測: 上位ビット検査の +14%、領域チェック +51% のオブジェクトサイズ）は、`fle_ldx` 系を加えたぶん（生成 C の 6,498 か所）増える。本作業では再測していない。
- 既存の制限（0045 d）はそのまま: 検査は上位 32 ビットと（任意で）領域だけで、ページ保護、ヒープ内のワイルドポインタは捕まえない。
