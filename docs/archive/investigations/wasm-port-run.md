# wasm 移植の実行記録（決定 0045）

日付: 2026-10-21。HEAD 4936d44 + ワークツリー（他担当の未コミット変更を含む。測定時点で `runtime/fle_mxcsr_flags.h` は純 C 版（決定 0044、md5 6f7f1eb3…）、`fle_x87.c`、`fle_locale.c` は他担当の変更後）。方式と制限は `docs/decisions/0045-wasm-port.md`。「実測」はコマンドの出力、「推測」は未検証の解釈。成果物は `lift/out-wasm2/`（`tmp/` = TMPDIR、`emcache/` = EM_CACHE。`out-wasm/` の `emcache` は複製。大きな `out-wasm/` のファイルは複製していない）。

## 1. 要約（実測）

| 項目 | 結果 |
|---|---|
| runtime `fle_*.c` の `emcc -c`（`-O0 -Wall -Wextra`、`-msse2 -msimd128`） | **32 / 32 成功**（`fle_host.c` を含む。警告 2 件: `fle_proc.c:105` 未使用変数、`fle_x87.c:1027` `shift count >= width of type`。どちらも既存で、本作業の変更ではない） |
| 生成 C 16 分割（all-pdata、58,695 関数、215,539,555 B）の `emcc -c`（`-finstrument-functions`） | **16 / 16 成功**（エラー 0）。1 TU 3.2〜6.6 秒、最大 RSS 1,813 MB（part-14）、同時 2 本 |
| リンク | **成功**。`flengine.wasm` **115,721,618 B**（約 110 MiB）、`flengine.js` 174,922 B。リンク 1.20 秒、最大 RSS 635 MB |
| node 実行（`bin-base`、`FLEngine_x64.dll`） | 停止: `GetLocaleInfoW: LCType value for en-US is not on the MSDN pages …`。関数実行数（distinct）**199**、入口 **1,386**、深さ **9**。1.7 秒、最大 RSS 396 MB、終了コード 134 |
| 同じ runtime、同じ生成 C のネイティブ（clang）実行 | 停止、関数 199、入口 1,386、深さ 9。**`diff` は `calling entry … rsp=` の 1 行（スタックのアドレス）だけ**。停止レポート（メッセージ、abort kind、スタック 9 段、entry log 256 件）は全文一致 |
| native のワークスペース全テスト | **65 バイナリ、615 passed、0 failed**（`test-ws.txt`）。新規 `tests/wasm_port.rs` の 5 件を含む。本作業の前の同じ測定（`test-ws-1-before-wasm_port.txt`）は 64 バイナリ、610 passed、0 failed |
| 領域チェック（`FLE_CHECKED_MEM`）のコスト | §4 |
| ADR-0005 の A3 | §6（判定: 下の定義と実測の対応のとおり、**リンクと node 実行は成立。ただし A3 の正式な定義は見つからなかった**） |

停止点の数値が probe の記録（198 / 984 / 11）と違うのは、ロケールの追加で `runtime` が変わり、DllMain 経路が少し先まで進んだため（推測。runtime を戻しての比較はしていない）。比較はすべて「同じ runtime、同じ生成 C」の native と wasm で行った。

## 2. 手順

```
bash lift/harness/wasm/build.sh            # gen split rt gen-c harness link（出力 lift/out-wasm2/）
node lift/out-wasm2/bin-base/run_node.cjs binaries/win64/FLEngine_x64.dll
bash lift/harness/wasm/build_native.sh     # 比較用の native（同じ生成 C と runtime）。lift/out-wasm2/native/flengine-native
```

- 変種（`VARIANT`、`EXTRA_CFLAGS`）: `base`（既定）、`chk`（`-DFLE_CHECKED_MEM`）、`nohi`（`-DFLE_MEASURE_NO_HI32`、検査なし。コスト測定用）、`interim`（`MXCSR_OVERRIDE=1`）。各変種は `obj-<変種>/`、`bin-<変種>/`、`log-<変種>/` に出る。上書きも削除もしない（出力があれば段階を飛ばす）。
- emcc は同時 2 本（`JOBS=2`）。生成 C の 1 TU は `-O0` で最大 1.8 GB（§4 の `-O1` の測定では 2.06 GB）。

## 3. 移植の確認

### 3.1 native の挙動が変わっていないこと

- runtime 32 本の native `clang -Wall -Wextra` は警告 0、エラー 0（変更後に再確認）。
- native のワークスペース全テスト 615 件が通る（§1）。`tests/sys.rs`、`tests/vmap.rs` の期待値は変更していない。
- `fle_addr` は native では `((uintptr_t)(a))` のマクロで、`fle_ld`/`fle_st` は従来のコードと同じに展開される（`FLE_CHECK_HI32`、`FLE_CHECKED_MEM` が未定義のとき）。テスト `native_build_has_no_address_check` は、native の既定で上位ビットが非ゼロのアドレスが SIGSEGV（11）になることを確かめる。
- 9 ファイルの tid: native は同じ `syscall(SYS_gettid)`。テスト `native_thread_id_is_the_linux_tid_…` が、`fle_host_tid()` が gettid と同じこと、`GetCurrentThreadId` が `(uint32_t)` の tid を返すことを確かめる。既存の proc.rs（`SYS_gettid` を許す検査）、`a9_a10.rs` などは通る。

### 3.2 wasm 側の確認

- 上位 32 ビット（`hi.c`、node で実行）: `fle_ld(0x6e69727453657261, 8)`（probe のワイルドポインタ）→ `[fle_addr_hi32]` の名前付き abort、`fle_st(buf + 2^32, …)` → 同じ（probe は別アドレスにエイリアスして成功した）。`fle_ld(0, 8)` は上位ビット検査だけでは通る（trap しない。値を読む）。`FLE_CHECKED_MEM` では NULL、0x1000 は `[fle_addr_region]` で止まり、イメージ内（0x400000）と malloc の領域は通る。
- VirtualAlloc 系（`w1.c`、wasm、`-sALLOW_MEMORY_GROWTH=1`）: 割り当て粒度、ゼロ埋め、VirtualQuery（state 0x1000、prot 2、size 0x3000）、VirtualProtect（旧保護 2）、CreateFileMappingW（ゼロ埋め）、FILE_MAP_COPY ビューの私有性、Unmap/Close 後の個数 0、`GetCurrentThreadId` = 1。すべて期待どおり。アリーナ用のメモリに `ALLOW_MEMORY_GROWTH` なしでは「Cannot enlarge memory (OOM)」で止まる（実測）。
- native でアリーナ版のコード（`-DFLE_VMEM_ARENA`）を走らせる `tests/wasm_port.rs` の 5 件: 全部通る。
- 既存テストをアリーナ版で走らせた結果（`out-wasm2/arena-clang/clang`、`-DFLE_VMEM_ARENA` を足したラッパーで `cargo test --test sys --test vmap`）: **sys.rs 18 件中 16 件、vmap.rs 23 件中 18 件が通り、7 件が通らない**（`test-arena-sys-vmap.txt`、`test-arena-vmap.txt`）。通らない 7 件の読み（メッセージ、または失敗した最初の比較から。7 件それぞれの再実行はしていない）:
  - 保護を強制しない（native は SIGSEGV）: `virtual_alloc_protection_faults`（sys）、`protect_changes_committed_pages_and_returns_the_old_protection`（vmap: `write_faults 0`、`noaccess_read_faults 0`）。
  - 同じオブジェクト、同じオフセットの 2 つ目の共有ビュー（名前付きで止まる）: `view_access_types_follow_the_object_protection`、`anonymous_mapping_is_zero_filled_and_views_are_coherent`、`duplicate_handle_of_a_mapping_handle_shares_the_object`（vmap）。
  - アリーナの外の固定アドレス（0x5000…、0x3000… は予約に失敗する）: `virtual_alloc_results`（sys）、`protect_spanning_two_regions_fails`（vmap）。
  これらは「アリーナ版は native と同一ではない」ことの記録で、本作業の native の期待値は変えていない（決定 0045 c）。

## 4. 領域チェックのコスト（実測）

3 変種を比較した: `nohi`（上位ビット検査も領域チェックもなし。測定のためだけの `-DFLE_MEASURE_NO_HI32`）、`base`（上位ビット検査、既定）、`chk`（上位ビット検査 + `-DFLE_CHECKED_MEM`）。

| 項目 | nohi | base（上位ビット） | chk（領域チェック付き） |
|---|---|---|---|
| 生成 C 16 本の `.o` 合計（`-O0`） | 173,605,700 B | 173,641,244 B（+0.02%） | 173,644,054 B（+0.02%） |
| 生成 C 16 本の `emcc -c` 時間の合計（`-O0`、同時 2 本） | 82.1 秒 | 78.2 秒 | 79.7 秒（差は計測のばらつき内。推測） |
| 生成 C の最大 RSS（`-O0`） | 1,813,448 KB | 1,813,196 KB | 1,813,392 KB |
| `flengine.wasm`（`-O0`） | 115,714,231 B | 115,721,618 B（+7,387 B、+0.006%） | 115,725,062 B（+10,831 B、+0.009%） |
| part-00 の `.o`（`-O1`） | 10,225,474 B | 11,678,701 B（+14.2%） | 15,413,258 B（+50.7%） |
| part-00 の `emcc -c` 時間（`-O1`） | 28.4 秒 | 32.2 秒（+13%） | 38.5 秒（+36%） |
| part-00 の最大 RSS（`-O1`） | 1,294 MB | 1,563 MB | 2,062 MB |
| ミクロベンチ、`fle_ld`×2 + `fle_st` を 2 億回（node、`-O0`） | 12.52 ns/回 | 14.46 ns（+15%） | 16.13 ns（+29%） |
| 同（`-O1`） | 0.92 ns | 1.14 ns（+24%） | 1.63 ns（+77%） |
| node の停止まで（3 回の範囲） | 1.56〜1.62 秒 | 1.69〜1.73 秒 | 1.68〜1.78 秒 |

- 読み: `-O0` では、`.o` と wasm のサイズは検査でほとんど増えない（検査が `static inline` で、`-O0` では呼び出しになるため。推測）。`-O1` で inline されると、1 TU の `.o` が上位ビット検査で +14%、領域チェックで +51%、コンパイル時間とメモリも増える。ミクロベンチでは領域チェックは上位ビット検査の約 1.4〜1.5 倍の時間（`-O1`）。
- 実際の停止までの実行（1,386 入口のみ）では、3 変種の差は node の起動と wasm のコンパイル時間（約 1.5 秒）に隠れて分からない。DllMain の先（停止点より後）での実行時間への影響は**未測定**。
- 領域チェックが捕まえるものと捕まえないもの、`fle_ldx` 系が未適用であること: 決定 0045 d。
- `chk` の node 実行は `base` と同じ停止レポート（`diff` なし。この経路では領域外のアクセスは起きていない）。

## 5. ネイティブとの比較（node）

```
diff <(grep -v '^calling' run-native.txt) <(grep -v '^calling' run-node-base.txt)    # 差なし
diff run-native.txt run-node-base.txt                                                  # calling … rsp= の 1 行だけ
```

`interim`（MXCSR の差し替えあり）、`chk`、`nohi` も、`base` と同じ停止レポート（`calling` 行を除き一致）。`interim` の wasm は 115,537,121 B（`base` より 184,497 B 小さい。推測: 差し替え版のコード量の違い）。DllMain の経路では `fle_mx_*` と x87 の演算は 0 回なので、この一致は純 C 版 MXCSR とソフト版の等価性の証拠にはならない（probe §0 の注意と同じ。ここでは再測していない）。

## 6. ADR-0005 の A3

- **定義の確認**: `docs/decisions/0005-mxcsr.md` を全文読んだ。本文は「STMXCSR と LDMXCSR の応答」の決定で、「A3」という語は**ない**。`docs/` 全体の `A3` は、`remaining-unsupported.md` と決定 0024 の「A3 = kernel32 グローバルメモリ・アトム・文字列」（API の項目名）だけで、wasm とは別。リポジトリ内に「A3 = 生成 C が emcc でリンクできる」と書いた文書はなく（`docs/`、`README.md`、`git log` のメッセージを grep）、probe の記載（本文に見つからなかった）と同じ。ネイティブ clang の確認だけで A3 を「達成」と記録した箇所も、ワークツリーと `git log` の範囲では見つからなかった（その記録は依頼文の記述で、原文は未確認）。
- したがって、定義は依頼文と probe の基準（「生成 C が emcc でリンクできる」）を使う。最終ターゲットは「C → emcc → wasm、node 上で実行」。
- 定義と実測の対応（実測）:

| 基準 | 実測 |
|---|---|
| 生成 C（all-pdata、16 分割）が emcc でコンパイルできる | 16 / 16 成功。runtime 32 / 32 成功（§1） |
| emcc でリンクできる | 成功。`flengine.wasm` 115,721,618 B、1.20 秒、最大 RSS 635 MB（§1） |
| node で実行できる | 実行した。DllMain 経路を走らせ、停止まで 1.7 秒 |
| ネイティブと同じ停止点、同じ関数実行数・入口数・深さ・停止レポート | 一致（199 / 1,386 / 9、GetLocaleInfoW LCType 0x37、`rsp` の行だけ違う。§5） |

- 判定: 上の 4 つの基準は、runtime を移植した（shim なし、`-I` やオーバーレイでの差し替えなし。MXCSR はワークツリーの純 C 版をそのまま使った）ビルドで満たされた。**ネイティブ clang の確認だけでは満たされない基準（emcc でのコンパイルとリンク、node 実行）を、今回初めてリポジトリ内のハーネス（`lift/harness/wasm/build.sh`）で実測した**。ただし次は A3 の外で、満たしていない（未達）: 停止点より先の実行（SSE/x87 の演算が wasm で native と同じ結果になること）、ブラウザ、ページ保護の強制、ゲストが触ってよい領域の完全な検査（決定 0045 d、制限）。「A3 を達成した」と言えるのは、定義を上の 4 つの基準とした場合に限る。定義の原文が見つからないため、PM が基準を確認すること。

## 7. `fscale`（probe の native と wasm の不一致）の原因確認

`fle_x87.c` は別担当の編集中のため触っていない。原因の確認は、`out-wasm2/x87chk/` のコピーで行った。

- `long` の使用箇所（現在の `fle_x87.c`。grep、`long long` を除く）: 行 43、181、184、201、202、242、283、361、597、654、672、931、953、1025、1027、1057、1079、1082、1083、1090、1091、1108、1151、1290、1309。キャスト `(long)` は 953 と 1027 の 2 か所。
- 実測: probe の `fscale_probe2.c`（60,002 行の出力）を、元の `fle_x87.c` と、`long` を全部 `int64_t` に替えたコピー（`fle_x87_i64.c`）で、native（clang）と wasm（emcc）の両方で実行した。元のソース: native と wasm で **18,822 行が不一致**。`int64_t` 版: **native == wasm（不一致 0）**。native の出力は、元のソースと `int64_t` 版で同一（`diff` 0 行）。
- 1027 行の 2 か所（`long n`、`(long)1 << 40`、`(long)(b.m >> …)`）だけを `int64_t` に替えたコピーでも、不一致は 18,822 行のまま（変わらない）。したがって fscale の差の原因は 1027 行の 2 か所だけではなく、他の `long`（行 43 の `long e`、`round_pack` の `long e`、`long u`、`long nb` など、`round80`/`round_pack` を通る値）にもあることが分かった（どの `long` が効いているかの個別の特定はしていない）。直す場合は、`long` 全部を `int64_t` に替える（`int64_t` 版で不一致 0 を確認）。
- 同じプローブの他の出力（`div/imul1`、`idiv`、他の x87 の行）は、`orig` でも native と wasm が一致している（不一致の 18,822 行は `fscale` の行）— これは各行のラベルを見ての読みで、行ごとの分類はしていない。

## 8. 触ったファイル

- 新規: `lift/runtime/fle_host.h`、`lift/runtime/fle_host.c`、`lift/harness/wasm/`（`build.sh`、`build_native.sh`、`main.c`、`trace.c`、`run_node.cjs`、`split_c.py`、`interim/fle_mxcsr_flags.h`）、`lift/crates/fe-lift/tests/wasm_port.rs`、`docs/decisions/0045-wasm-port.md`、本書。
- 変更: `lift/runtime/fle_runtime.h`（`fle_addr`、領域チェック）、`fle_sys.c`、`fle_sys.h`（アリーナ）、`fle_vmap.c`（アリーナ）、`fle_glob.c`、`fle_imports.c`、`fle_ole.c`、`fle_oleaut.c`、`fle_proc.c`、`fle_sysinfo.c`、`fle_vol.c`（tid）、`lift/harness/image.c`（`__EMSCRIPTEN__` の分岐のみ）。
- 触っていない: `fle_mxcsr_flags.h`、`fle_sse.h`、`fle_x87.c/.h`、`fle_locale.c`、`tests/` の既存ファイル、`lift/harness/trace.c`、`lift/harness/main.c`、crates の `src/`、`docs/investigations/wasm-build-probe.md`。
- 成果物（`lift/out-wasm2/`）: `bin-{base,chk,nohi,interim}/`、`obj-*`、`log-*`、`out/`（生成 C、約 430 MB）、`native/`、`arena/`、`cost/`、`x87chk/`、`target*`（cargo のビルド）、`test-*.txt`。合計は `du` を取っていない。削除禁止のため、途中のテストファイルが残っている。

## 9. 未確認、限界

- 停止点より先の実行（SSE/x87 の演算、`CreateFileMappingW` の実使用）。ブラウザ。`-O1` 以上のリンクと実行。
- 領域チェックと上位ビット検査は `fle_ldx` 系と runtime 内の直接変換には効かない（決定 0045 d）。
- 同じオブジェクト、同じオフセットの 2 つ目の共有ビューは wasm で止まる。ゲストがそれを使うかは未確認。
- アリーナ 512 MiB を超える合計の予約は失敗する。ゲストが大きな `VirtualAlloc` を要求する経路は未測定。
- `docs/investigations/2026-10-19-environment.md` は存在しない（probe と同じ）。
