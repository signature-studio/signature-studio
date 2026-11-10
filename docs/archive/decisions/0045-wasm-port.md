# 0045: runtime の wasm 移植（emcc、node）とハーネスの製品化

状態: 作業者（サブエージェント）が決定。依頼は「探索用 shim で通っていた wasm ビルドを、runtime 本体の移植とリポジトリ内のハーネスとして製品化する。ネイティブ clang の挙動とテストは変えない」。入力は `docs/investigations/wasm-build-probe.md`（以下 probe。本書からの参照のみで、probe には追記しない）。実測と実行記録は `docs/investigations/wasm-port-run.md`。実装は `lift/runtime/fle_host.h`、`fle_host.c`（新規）、`fle_runtime.h`、`fle_sys.c/.h`、`fle_vmap.c`、9 ファイルの tid 呼び出し、`lift/harness/image.c`（`__EMSCRIPTEN__` 分岐の追加）、`lift/harness/wasm/`（新規）、`tests/wasm_port.rs`（新規）。

## 決定

### a. スレッド ID（probe D5）

- `lift/runtime/fle_host.h` の `fle_host_tid()` に集約した。`syscall(SYS_gettid)` を使っていた 9 ファイル（fle_glob、fle_imports〔2 か所〕、fle_ole、fle_oleaut、fle_proc、fle_sys、fle_sysinfo、fle_vmap、fle_vol）の呼び出し 10 か所をこれに替えた。
- native: 従来どおり `syscall(SYS_gettid)`。`GetCurrentThreadId` は従来どおり `(uint32_t)` のその値（決定 0001 のまま。ゲストから見える意味は変わらない）。
- emscripten（`__EMSCRIPTEN__`）: 単一スレッド前提（決定 0017）の固定値 `FLE_HOST_WASM_TID` = 1。0 にしない理由は、各ファイルの owner 変数が 0 を「未記録」に使うため。wasm では `GetCurrentThreadId` も 1 を返す（ゲストが見るスレッド ID の値は native と違う。native でも値はプロセスごとに変わる値で、決定 0001 は一意性と安定性だけを求めている）。
- 別スレッドの検出（owner と違う tid で止める規則）は wasm では起こり得ない（スレッドが 1 つ）。native の規則は変えていない。

### b. ファイルマッピングの裏付け（probe D8、決定 0024）

- `FLE_VMEM_ARENA`（emscripten で自動定義。native でも手で定義すれば同じコードを実行できる）では、`memfd_create`/`ftruncate`/`pwrite`/`mmap(fd)` を使わない。オブジェクトは下の「アリーナ」の範囲 1 つ（`fle_sys_host_reserve`）。作成時にゼロ埋めし、仮想ファイルの内容をコピーする（決定 0024 のスナップショットのまま）。
- 共有ビュー（FILE_MAP_READ/WRITE）は**そのオブジェクトの範囲そのもの**を指す（アドレス = 範囲の先頭 + オフセット）。同じオブジェクトの全ビューが同じメモリを見るので、ページの「ビューは一貫」は保たれる。copy-on-write ビュー（FILE_MAP_COPY）は範囲のコピーを別に確保する。オブジェクトの範囲は、ハンドルが全部閉じ、ビューが全部外れた時に返す。
- **制限（native との違い）**: 同じオブジェクトの同じオフセットに 2 つ目の共有ビューを同時に作ると、同じアドレスを 2 つのビューに与えることになるので、名前付きで止める（`fle_stop`）。native は別アドレスの 2 ビューを作る。線形メモリには同じバイトに別アドレスを与える手段がない。ゲストがこの形を使うかは未確認。
- native の挙動（memfd + MAP_SHARED）は変えていない。

### c. VirtualAlloc 系とページサイズ（probe D6、D7、決定 0017、0024）

- 調べた事実: native の実装は、予約を `mmap(PROT_NONE, MAP_NORESERVE)`、コミットを `mprotect`、デコミットを `madvise(DONTNEED)` + `PROT_NONE`、解放を `munmap` で行い、領域の状態（ページごとの PAGE_* 値）と `alloc_protect` は runtime の表（`fle_region`）が持つ。emscripten では `sysconf(_SC_PAGESIZE)` が 65536、`mprotect` と `madvise` は何もせず、`mmap` はアドレス指定ができない（probe §4.4）。probe の「応答」は `--wrap` で `sysconf`→4096、匿名 `mmap`→`aligned_alloc`+`memset`（アドレス指定は失敗、512 MiB 超は失敗）、`munmap`/`mprotect`→何もしない、`madvise(DONTNEED)`→`memset`、というものだった（probe 専用で `wasm_mmap_probe.c`）。
- 方式（アリーナ）: 最初の使用時にホストのヒープから 512 MiB（`FLE_VMEM_ARENA_BYTES`、定義で変更可）を `aligned_alloc(65536)` で 1 つ取る。予約はその中の範囲（アドレス順の表で管理。割り当て粒度 65536 に整列。`fle_sys_host_reserve`）。ゲスト指定アドレスの予約は、そのアドレスがアリーナの中で他と重ならない場合にだけ成功し、それ以外は「使用中のアドレス」と同じ扱い（NULL、ERROR_INVALID_ADDRESS 487）になる。ゲストのページは 4096 のままで、「ホストのページサイズが 4096 であること」の検査（`fle_mem_init`）はアリーナでは行わない。ホストの保護機構（mprotect）は使わない。
- コミット: 未コミットだったページをゼロ埋めする（コミットしたページは 0 を読む。MSDN VirtualAlloc。アリーナはヒープから取ったままゼロ埋めされていないため）。すでにコミット済みのページはゼロ埋めしない（native と同じ）。デコミットは状態を「予約」に戻すだけで、次のコミットのゼロ埋めで 0 を読ませる。
- 仮想ページ保護の状態管理（`pages[]` の PAGE_* 値、`alloc_protect`、VirtualProtect、VirtualQuery）は決定 0024 のまま、runtime の表だけが持つ。
- **制限（native との違い）**: ホストが保護を強制しないので、予約のみ、デコミット後、読み取り専用、PAGE_NOACCESS のページへのゲストのアクセスは、native では SIGSEGV で止まるが、wasm では成功する（probe §4.4 と同じ。区別できるのは `FLE_CHECKED_MEM` のような別の検査を足した場合だけで、これはページ保護までは見ない）。固定アドレス（アリーナの外）の予約は失敗する（native は成功する）。アリーナは 512 MiB を超える合計の予約（ファイルマッピング含む）で失敗する（ERROR_NOT_ENOUGH_MEMORY 8）。アリーナの取得には `-sALLOW_MEMORY_GROWTH=1` と、`-Wl,--max-memory` が 512 MiB 以上の余裕を持つことが要る（build.sh は 2 GiB）。
- native のテスト（sys.rs、vmap.rs）の期待値は変えていない。それらは native の既定ビルドで走り、全部通る。アリーナ版を native で走らせて同じ期待値と比べた結果は wasm-port-run.md（上の 3 つの違いのとおり、sys.rs 18 件中 16 件、vmap.rs 23 件中 18 件が通り、通らない 7 件の原因は上の制限（保護の非強制 2 件、同じオフセットの 2 つ目のビュー 3 件、アリーナ外の固定アドレス 2 件）と読んだ。各テストの失敗メッセージ、または失敗した最初の比較から読んだもので、7 件それぞれを制限を外して再実行して確かめてはいない）。アリーナ版の挙動は `tests/wasm_port.rs`（新規）が native で固定している。

### d. ゲストアドレスの上位ビット（probe D15）

- `fle_runtime.h` に `fle_addr(a, n, store)` を置き、`fle_ld`/`fle_st` がこれを通る。**マクロの変更だけで、生成 C の再生成は不要**（生成 C の `fle_ld`/`fle_st` の呼び出しは変わらない。本作業は生成器 `fe-lift` を変えていない。測定に使った生成 C は、その時点のワークツリーの `fe-lift`（他担当の変更を含む）が出した 215,539,555 B で、probe の 215,539,533 B、`out-loc` の出力とは別の内容）。
- `FLE_WASM32`（`__EMSCRIPTEN__` で自動定義）で `FLE_CHECK_HI32` が有効になり、`a >> 32` が非ゼロなら `fle_addr_fault`（`fle_host.c`）が名前付きのメッセージ（`[fle_addr_hi32]`）で `abort()` する。native の SIGSEGV に相当する。ネイティブの既定ビルドでは `fle_addr` は `((uintptr_t)(a))` に展開され、従来のコードと同じ（検査なし）。
- 領域チェック `FLE_CHECKED_MEM`（任意、既定はオフ、wasm のみ）: 上位ビットの検査に加えて、アクセスが「イメージ [ImageBase, ImageBase + SizeOfImage)」または「モジュールのデータ領域の先頭（`__global_base`）以上」にない場合に `[fle_addr_region]` で止める。データ領域以上には、データ、スタック（ハーネスは malloc）、ヒープ、アリーナ、TEB/TLS（`fle_teb`、`fle_tls_values` は runtime の静的変数）、スクラッチが入る。ImageBase と SizeOfImage は `fle_mem_set_image`（ハーネスの main が呼ぶ）で渡す。
- 領域チェックが**捕まえるもの**: NULL ページと低いアドレス（0〜ImageBase の手前）、イメージの終わりから `__global_base` までの隙間（GLOBAL_BASE = 256 MiB の下）。**捕まえないもの**: ヒープ、データ内で別のオブジェクトを指すワイルドポインタ、ページ保護の違反。
- **未適用の経路（制限）**: `fle_sse.h` の `fle_ldx`/`fle_ldxa`/`fle_stx`/`fle_stxa`（生成 C に 264 + 3,754 + 206 + 2,274 = 6,498 か所）と `fle_vex.h` はポインタ変換を自前で行い、`fle_addr` を通っていない。runtime の各 `.c` が `R[n]` や引数のポインタを直接 `(uintptr_t)` で使う箇所（fle_locale、fle_oleaut、fle_ole、fle_vfs など、約 100 か所）も通っていない。したがって上位 32 ビットが落ちる経路は、これらに残っている。`fle_sse.h` は別担当が編集中のため、本作業では触っていない。直す場合は、`fle_ldx`、`fle_stx` の `(uintptr_t)a` を `fle_addr(a, 16, 0 または 1)` に替える 2 行（native には影響しない）。
- コスト実測は wasm-port-run.md（要点: 検査なしを基準に、上位ビット検査で生成 C の 1 TU（part-00）の `-O1` オブジェクトが +14%、領域チェックを足すと +51%。`-O0` の wasm サイズは +0.006% と +0.009%、node の停止まで約 1.6 秒 → 1.7 秒 → 1.7 秒。差は計測のばらつきの範囲に近い）。領域チェックを既定にするかは PM の判断。**既定はオフ**。

### e. ハーネスと build.sh

- `lift/harness/wasm/`: `build.sh`（fe-lift で生成、16 分割、runtime の全 `fle_*.c`、生成 C 16 本、ハーネス 3 本を emcc でコンパイルし、リンクして `flengine.wasm` を作る。emcc は同時 2 本まで。出力は `lift/out-wasm2/`、TMPDIR と EM_CACHE も `out-wasm2/` の中）、`main.c`（DllMain 相当の実行と停止レポート）、`trace.c`（native の `out-pdata/trace.c` から、シグナル処理を除き、ハッシュを乗算ハッシュ（7.5 秒 → 1.5 秒）にし、アドレス停止の分類を足した。`lift/harness/trace.c` は変えていない）、`run_node.cjs`、`split_c.py`（`out-pdata/split_c.py` の複製）、`build_native.sh`（同じ生成 C と現在の runtime を native clang でビルドして比較に使う）。
- ハーネスの `image.c` は `lift/harness/image.c` を共用する。`__EMSCRIPTEN__` の分岐で、`mmap` の代わりにイメージを ImageBase に置く（リンクが `GLOBAL_BASE` = 256 MiB でデータをその上に置くので、イメージ 0x400000〜0x1cc0000 は下に入る）。native の分岐は変えていない。
- リンクのフラグは probe と同じ（`-O0 -sALLOW_MEMORY_GROWTH=1 -sSTACK_SIZE=8MB -sGLOBAL_BASE=268435456 -sENVIRONMENT=node -sMODULARIZE=1 -sEXIT_RUNTIME=1 -sNODERAWFS=1 --initial-memory=640 MiB --max-memory=2 GiB --wrap=fprintf,fputs,abort`）。ブラウザ（`-sENVIRONMENT=web`）は未対応（`NODERAWFS` は node 限定）。

### f. MXCSR の暫定扱い

- `fle_mxcsr_flags.h` は別担当が純 C 版（決定 0044）に置き換え中。build.sh は、`runtime/fle_mxcsr_flags.h` にインライン asm が残っている間だけ、probe のソフト版（`lift/harness/wasm/interim/fle_mxcsr_flags.h`。probe の `fle_mxcsr_flags_soft.h` の複製。MXCSR の状態ビットを立てず、RC/FTZ/DAZ を無視する）に差し替える（`MXCSR_OVERRIDE=auto`）。`-I` を前に置くだけでは差し替わらない（`fle_sse.h` が `"fle_mxcsr_flags.h"` を引用符で include し、コンパイラはまず `fle_sse.h` と同じディレクトリを探す。実測で確認した）ので、clang の仮想ファイルシステムのオーバーレイ（`-ivfsoverlay`、build.sh が生成）で `runtime/fle_mxcsr_flags.h` を差し替える。
- 本作業の測定時点で、ワークツリーの `runtime/fle_mxcsr_flags.h`（md5 6f7f1eb3…）はすでに純 C 版で、差し替えは**自動で OFF**になり、本物の純 C 版でビルド、リンク、実行を確かめた。差し替えの経路（`MXCSR_OVERRIDE=1`）も別の VARIANT でビルド、リンク、実行して確かめた（同じ停止点）。本物が入った後は `interim/` とオーバーレイの分岐を削除してよい（それまでは害がない）。純 C 版の等価性は本作業の対象外（0044 の担当）。

## 代替案

- 線形メモリの先頭付近に低いアドレスのゲスト空間を作り、ゲストのアドレスを 32 ビットに写す案（旧実績の lowbase 方式）: 生成 C の再生成か、`fle_ld`/`fle_st` の変換が要る。ゲストが 64 ビットのポインタを値として持つ（VirtualAlloc の戻りなど）ことと、`FLE_PROCESS_HEAP` 0x7FF700000000 などのトークンが既にあるため、本作業では採らず、上位ビットの検査で止める方式にした。
- 領域チェックを既定にする案: 実測のコスト（上記）と、ヒープ内の誤アクセスを捕まえないため、既定はオフにして PM の判断に委ねた。
- 同じオブジェクトの共有ビューを別アドレスにコピーして同期する案: 書き込みを検出できず一貫性を保てないので却下（止める方を採った）。

## 限界

- ブラウザは未確認。node で実行する DllMain 経路は 199 関数 / 1,386 入口で GetLocaleInfoW の停止まで。この経路で `fle_mx_*` の演算と x87 の演算は 0 回（probe §0 の注意と同じ。MXCSR 純 C 版と x87 の wasm 上の等価性の証拠にはならない）。
- アリーナの制限、ページ保護の非強制、`fle_ldx` 系と runtime 内の直接変換が上位ビット検査の外にあること（上記）。
- ソフト x87 の `fscale` の native と wasm の差は、`fle_x87.c` の `long`（wasm32 で 32 ビット）が原因であることを、コピーでの置換（`long` → `int64_t`）で確かめた（wasm-port-run.md §7）。`fle_x87.c` 本体は別担当の編集中のため未修正。
